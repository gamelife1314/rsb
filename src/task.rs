//! task module mainly defines the task model
//!
//! # Example
//!
//! ```ignore
//! use std::io;
//! use std::sync::Arc;
//! use clap::Parser;
//! use rsb::{Arg, Task};
//! use rsb::arg::OutputFormat;
//!
//! let arg = Arg::parse();
//! print_tip(&arg)?;
//! let pb = create_progress_bar(&arg);
//! let output_format = arg.output_format;
//! let task = Arc::new(Task::new(arg, Some(pb))?).run()?;
//! let result = match output_format {
//!    OutputFormat::Text => task.text_output()?,
//!    OutputFormat::Json => {
//!        let output = task.json_output()?;
//!        serde_json::to_string_pretty(&output)?
//!    }
//! };
//! writeln!(&mut io::stdout(), "{result}")?;
//! ```

use std::cmp::min;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use colored::Colorize;
use indicatif::ProgressBar;
use num_cpus;
use reqwest::Client;
use tokio::{
    self, runtime,
    sync::{self as tsync, mpsc},
};

use crate::client::build_client;
use crate::dispatcher::DurationDispatcher;
use crate::dispatcher::{CountDispatcher, Dispatcher};
use crate::limiter::Limiter;
use crate::output::{sync_text_output, Output};
use crate::request::build_request;
use crate::statistics::{Message, Statistics};
use crate::Arg;

/// [Task] indicates a task to be performed
pub struct Task {
    arg: Arg,
    client: Client,
    statistics: Statistics,
    is_canceled: AtomicBool,
    progress_bar: Option<ProgressBar>,
    is_workers_done: AtomicBool,
    dispatcher: Arc<tsync::RwLock<Box<dyn Dispatcher<Limiter = Limiter>>>>,
}

fn create_count_dispatcher(
    total: u64,
    rate: &Option<u16>,
) -> Box<dyn Dispatcher<Limiter = Limiter>> {
    let count_dispatcher = CountDispatcher::new(total, rate);
    Box::new(count_dispatcher)
}

fn create_duration_dispatcher(
    duration: Duration,
    rate: &Option<u16>,
) -> Box<dyn Dispatcher<Limiter = Limiter>> {
    let duration_dispatcher = DurationDispatcher::new(duration, rate);
    Box::new(duration_dispatcher)
}

fn create_dispatcher(
    arg: &Arg,
) -> Arc<tsync::RwLock<Box<dyn Dispatcher<Limiter = Limiter>>>> {
    let dispatcher = if arg.requests.is_some() {
        Arc::new(tsync::RwLock::new(create_count_dispatcher(
            arg.requests.unwrap(),
            &arg.rate,
        )))
    } else {
        Arc::new(tsync::RwLock::new(create_duration_dispatcher(
            arg.duration.unwrap(),
            &arg.rate,
        )))
    };
    dispatcher
}

impl Task {
    /// construct a new task
    ///
    /// Argument:
    ///
    /// [arg][`Arg`] - parameters for the task to run
    ///
    /// [progress_bar][`indicatif::ProgressBar`] - it is an optional value, when
    /// it exists, go back and update the progress
    pub fn new(
        arg: Arg,
        progress_bar: Option<ProgressBar>,
    ) -> anyhow::Result<Self> {
        let client = build_client(&arg)?;
        let dispatcher = create_dispatcher(&arg);

        Ok(Self {
            arg,
            client,
            dispatcher,
            progress_bar,
            statistics: Statistics::new(),
            is_canceled: AtomicBool::new(false),
            is_workers_done: AtomicBool::new(false),
        })
    }

    async fn update_progress_bar(self: Arc<Self>) {
        if self.progress_bar.is_none() {
            return;
        }
        if self.arg.requests.is_some() {
            self.update_count_progress_bar().await;
        } else if self.arg.duration.is_some() {
            self.update_duration_progress_bar().await;
        }
    }

    async fn update_count_progress_bar(self: Arc<Self>) {
        let total = self.arg.requests.unwrap();
        loop {
            self.progress_bar
                .clone()
                .unwrap()
                .set_position(min(self.statistics.get_total(), total));
            if self.is_workers_done.load(Ordering::Acquire) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn update_duration_progress_bar(self: Arc<Self>) {
        let total = self.arg.duration.unwrap().as_secs();
        let mut current = 0;
        loop {
            current += 1;
            self.progress_bar
                .clone()
                .unwrap()
                .set_position(min(current, total));
            if self.is_workers_done.load(Ordering::Acquire) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    fn finish_progress_bar(self: Arc<Self>) {
        if let Some(progress_bar) = &self.progress_bar {
            if !progress_bar.is_finished() {
                if self.is_canceled.load(Ordering::Acquire) {
                    progress_bar.abandon_with_message(
                        "(canceled!!!)".to_uppercase().red().bold().to_string(),
                    );
                } else {
                    progress_bar.finish();
                }
            }
        }
    }

    async fn worker(
        self: Arc<Self>,
        sender: mpsc::Sender<Message>,
    ) -> anyhow::Result<()> {
        loop {
            if !self.dispatcher.read().await.try_apply_job().await {
                break;
            }

            let request = build_request(&self.arg, &self.client).await?;

            let req_at = Instant::now();
            let response = self.client.execute(request).await;
            self.dispatcher.read().await.complete_job();
            let message = Message::new(response, req_at, Instant::now());
            sender.send(message).await?;
        }
        Ok(())
    }

    /// get the text output after task execution
    ///
    /// ```text
    /// Statistics         Avg          Stdev          Max
    ///   Reqs/sec       15197.11       583.93       15817.00
    ///   Latency         3.25ms        2.04ms       56.11ms
    ///   Latency Distribution
    ///      50%      2.10ms
    ///      75%      2.49ms
    ///      90%      2.81ms
    ///      99%      3.16ms
    ///   HTTP codes:
    ///     1XX - 0, 2XX - 151856, 3XX - 0, 4XX - 0, 5XX - 0
    ///     others - 0
    ///   Throughput:   15388.50/s
    /// ```
    pub fn text_output(self: Arc<Self>) -> anyhow::Result<String> {
        sync_text_output(&self.statistics, &self.arg)
    }

    /// returns a structure that can be serialized into json, and users can also
    /// customize it
    pub fn json_output(self: Arc<Self>) -> anyhow::Result<Output> {
        Output::sync_from_statistics(&self.statistics)
    }

    async fn rcv_worker_message(
        self: Arc<Self>,
        mut receiver: mpsc::Receiver<Message>,
    ) {
        loop {
            let result = receiver.try_recv();
            if result.is_ok() {
                self.statistics.handle_message(result.unwrap()).await;
                continue;
            }
            if self.is_workers_done.load(Ordering::Acquire) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    async fn handle_ctrl_c_signal(self: Arc<Self>) -> anyhow::Result<()> {
        loop {
            tokio::signal::ctrl_c().await?;
            self.dispatcher.write().await.cancel();
            self.is_canceled.store(true, Ordering::SeqCst);
        }
    }

    /// run a task to get its result
    ///
    /// # Example
    ///
    /// ```ignore
    /// Arc::new(Task::new(arg, Some(pb))?).run()?.text_output()?
    /// ```
    pub fn run(self: Arc<Self>) -> anyhow::Result<Arc<Self>> {
        let rt = runtime::Builder::new_multi_thread()
            .worker_threads(num_cpus::get())
            .thread_name("rsb-tokio-runtime-worker")
            .unhandled_panic(runtime::UnhandledPanic::ShutdownRuntime)
            .enable_all()
            .build()?;

        rt.block_on(async {
            let (tx, rx) = mpsc::channel::<Message>(500);

            // start workers by connection number
            let mut jobs = Vec::with_capacity(self.arg.connections as usize);

            // reset start time
            let task = self.clone();
            #[allow(clippy::redundant_async_block)]
            tokio::spawn(
                async move { task.statistics.reset_start_time().await },
            )
            .await?;

            // start handle signal
            tokio::spawn(self.clone().handle_ctrl_c_signal());

            // update progress bar job
            let update_pb_job =
                tokio::spawn(self.clone().update_progress_bar());

            // start statistics timer
            let task = self.clone();
            let stat_timer = tokio::spawn(async move {
                task.statistics.timer_per_second().await;
            });

            // start all worker and send request
            for _ in 0..self.arg.connections {
                jobs.push(tokio::spawn(self.clone().worker(tx.clone())));
            }

            // handle statistics
            let statistics_job =
                tokio::spawn(self.clone().rcv_worker_message(rx));

            // wait all jobs end
            for worker in jobs {
                worker.await??;
            }
            self.is_workers_done.store(true, Ordering::SeqCst);

            // notify stop statics timer
            let task = self.clone();
            #[allow(clippy::redundant_async_block)]
            tokio::spawn(async move { task.statistics.stop_timer().await })
                .await?;

            // wait statistics job complete
            statistics_job.await?;

            // wait update progress bar job finish
            update_pb_job.await?;

            // wait statistics timer end
            stat_timer.await?;

            // finish progress bar
            self.clone().finish_progress_bar();

            // wait statistics summary
            let task = self.clone();
            tokio::spawn(async move {
                task.statistics
                    .summary(task.arg.connections, task.arg.percentiles.clone())
                    .await;
            })
            .await?;

            Ok::<(), anyhow::Error>(())
        })?;

        Ok(self)
    }
}

//! mod statistics counts all relevant information about the server response

use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::*};
use std::time::{Duration, Instant};

use num::integer::Roots;
use reqwest::{Response, StatusCode};
use tokio::{sync as tsync, time as ttime};

#[derive(Debug)]
pub(crate) struct Statistics {
    /// status code [100, 200)
    pub(crate) rsp1xx: AtomicU64,

    /// status code [200, 300)
    pub(crate) rsp2xx: AtomicU64,

    /// status code [300, 400)
    pub(crate) rsp3xx: AtomicU64,

    /// status code [400, 500)
    pub(crate) rsp4xx: AtomicU64,

    /// status code [500, 511]
    pub(crate) rsp5xx: AtomicU64,

    /// other response code
    pub(crate) rsp_others: AtomicU64,

    /// errors category
    pub(crate) errors: tsync::Mutex<HashMap<String, u64>>,

    /// start time
    started_at: tsync::Mutex<Instant>,

    /// total_success send and receive response requests
    total_success: AtomicU64,

    /// total send and receive response requests although meets error
    total: AtomicU64,

    /// maximum per second
    pub(crate) max_req_per_second: tsync::Mutex<f64>,

    /// average per second
    pub(crate) avg_req_per_second: tsync::Mutex<f64>,

    /// stdev per second, link: https://en.wikipedia.org/wiki/Standard_deviation
    pub(crate) stdev_per_second: tsync::Mutex<f64>,

    /// log requests by second
    req_per_second: tsync::Mutex<Vec<u64>>,

    /// used for internal statistics, the number of requests accumulated in the
    /// current second will be reset when the next second starts
    current_cumulative: AtomicU64,

    /// average time spent on request
    pub(crate) avg_req_used_time: tsync::Mutex<Duration>,

    /// maximum time spent by the request
    pub(crate) max_req_used_time: tsync::Mutex<Duration>,

    /// stdev per request, link: https://en.wikipedia.org/wiki/Standard_deviation
    pub(crate) stdev_req_used_time: tsync::Mutex<Duration>,

    /// used internally to record the time spent on each request
    used_time: tsync::Mutex<Vec<Duration>>,

    /// indicates whether to stop, used to notify the internal timer to exit
    is_stopped: AtomicBool,

    /// recording stop time
    stopped_at: tsync::Mutex<Option<Instant>>,

    /// throughput, connections / avg_req_used_time, reqs/s
    pub(crate) throughput: tsync::Mutex<f64>,

    /// latencies for different percentiles
    pub(crate) latencies: tsync::Mutex<Vec<(f32, Duration)>>,
}

impl Statistics {
    /// construct empty Statistics
    pub(crate) fn new() -> Statistics {
        Self {
            rsp1xx: AtomicU64::new(0),
            rsp2xx: AtomicU64::new(0),
            rsp3xx: AtomicU64::new(0),
            rsp4xx: AtomicU64::new(0),
            rsp5xx: AtomicU64::new(0),
            rsp_others: AtomicU64::new(0),
            errors: tsync::Mutex::new(HashMap::new()),
            started_at: tsync::Mutex::new(Instant::now()),
            total: AtomicU64::new(0),
            total_success: AtomicU64::new(0),
            req_per_second: tsync::Mutex::new(Vec::new()),
            avg_req_per_second: tsync::Mutex::new(0.0),
            max_req_per_second: tsync::Mutex::new(0.0),
            stdev_per_second: tsync::Mutex::new(0.0),
            is_stopped: AtomicBool::new(false),
            current_cumulative: AtomicU64::new(0),
            stopped_at: tsync::Mutex::new(None),
            latencies: tsync::Mutex::new(Vec::new()),
            throughput: tsync::Mutex::new(0.0),
            used_time: tsync::Mutex::new(Vec::new()),
            avg_req_used_time: tsync::Mutex::new(Duration::from_secs(0)),
            max_req_used_time: tsync::Mutex::new(Duration::from_secs(0)),
            stdev_req_used_time: tsync::Mutex::new(Duration::from_secs(0)),
        }
    }

    /// return current send and rcv requests
    pub(crate) fn get_total(&self) -> u64 {
        self.total.load(Acquire)
    }

    /// if there will be a lot of preparation work before starting the
    /// statistics, it is best to reset the start time at the official start
    pub(crate) async fn reset_start_time(&self) {
        let mut started_at = self.started_at.lock().await;
        *started_at = Instant::now();
    }

    /// used to start the internal timer, and generate a box of snapshots for
    /// some data every second
    pub(crate) async fn timer_per_second(&self) {
        let mut timer = ttime::interval(Duration::from_secs(1));
        timer.tick().await; // skip the first tick
        loop {
            timer.tick().await;
            {
                let mut req_per_second = self.req_per_second.lock().await;
                req_per_second.push(self.current_cumulative.load(Acquire));
                self.current_cumulative.store(0, SeqCst);
            }
            if self.is_stopped.load(Acquire) {
                break;
            }
        }
    }

    fn statistics_rsp_code(&self, status: StatusCode) {
        match status {
            status
                if status >= StatusCode::CONTINUE
                    && status < StatusCode::OK =>
            {
                self.rsp1xx.fetch_add(1, SeqCst);
            },
            status
                if status >= StatusCode::OK
                    && status < StatusCode::MULTIPLE_CHOICES =>
            {
                self.rsp2xx.fetch_add(1, SeqCst);
            },
            status
                if status >= StatusCode::MULTIPLE_CHOICES
                    && status < StatusCode::BAD_REQUEST =>
            {
                self.rsp3xx.fetch_add(1, SeqCst);
            },
            status
                if status >= StatusCode::BAD_REQUEST
                    && status < StatusCode::INTERNAL_SERVER_ERROR =>
            {
                self.rsp4xx.fetch_add(1, SeqCst);
            },
            status
                if status >= StatusCode::INTERNAL_SERVER_ERROR
                    && status
                        <= StatusCode::NETWORK_AUTHENTICATION_REQUIRED =>
            {
                self.rsp5xx.fetch_add(1, SeqCst);
            },
            _ => {
                self.rsp_others.fetch_add(1, SeqCst);
            },
        }
    }

    async fn handle_resp_error(&self, err: reqwest::Error) {
        let err_msg = format!("{}", err.source().as_ref().unwrap());
        {
            let mut errors = self.errors.lock().await;
            errors
                .entry(err_msg)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        if let Some(status) = err.status() {
            self.statistics_rsp_code(status);
        }
    }

    /// receive message and make statistics
    pub(crate) async fn handle_message(&self, message: Message) {
        let Message {
            rsp_at,
            req_at,
            response,
        } = message;

        self.total.fetch_add(1, SeqCst);

        if response.is_err() {
            let err = response.err().unwrap();
            self.handle_resp_error(err).await;
            return;
        }

        let response = response.unwrap();
        self.statistics_rsp_code(response.status());
        self.total_success.fetch_add(1, SeqCst);
        self.current_cumulative.fetch_add(1, SeqCst);
        let mut used_time = self.used_time.lock().await;
        used_time.push(rsp_at - req_at);
    }

    /// notify stop timer
    pub(crate) async fn stop_timer(&self) {
        self.is_stopped.store(true, SeqCst);
        let mut stopped_at = self.stopped_at.lock().await;
        *stopped_at = Some(Instant::now());
    }

    async fn calculate_max_per_second(&self) {
        let req_per_second = self.req_per_second.lock().await;
        if let Some(max) = req_per_second.iter().max() {
            let mut max_per_second = self.max_req_per_second.lock().await;
            *max_per_second = *max as f64;
        }
    }

    async fn calculate_avg_per_second(&self) {
        let req_per_second = self.req_per_second.lock().await;
        if (*req_per_second).is_empty() {
            return;
        }

        let mut origin = &*req_per_second as &[u64];

        // the data at the last second is likely to be incomplete
        if origin.len() > 2 {
            origin = &origin[..origin.len() - 1];
        }

        let mut avg_per_second = self.avg_req_per_second.lock().await;
        *avg_per_second =
            origin.iter().sum::<u64>() as f64 / origin.len() as f64;
    }

    async fn calculate_stdev_per_second(&self) {
        let req_per_second = self.req_per_second.lock().await;
        if (*req_per_second).is_empty() {
            return;
        }

        let mut origin = &*req_per_second as &[u64];

        // the data at the last second is likely to be incomplete
        if origin.len() > 2 {
            origin = &origin[..origin.len() - 1];
        }

        let count = origin.len();
        let sum = origin.iter().sum::<u64>();
        let mean = sum as f64 / count as f64;
        let variance = origin
            .iter()
            .map(|x| {
                let diff = *x as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / count as f64;
        let mut stdev_per_second = self.stdev_per_second.lock().await;
        *stdev_per_second = variance.sqrt();
    }

    async fn calculate_elapsed_time(&self) {
        let mut used_time = self.used_time.lock().await;
        if (*used_time).is_empty() {
            return;
        }
        used_time.sort();

        // avg_req_elapsed_time
        let mut avg_req_used_time = self.avg_req_used_time.lock().await;
        let total: Duration = used_time.iter().sum();
        let count = used_time.len();
        *avg_req_used_time = total / count as u32;

        // max_req_elapsed_time
        let mut max_req_used_time = self.max_req_used_time.lock().await;
        if let Some(max) = used_time.iter().max() {
            *max_req_used_time = *max;
        }

        // stdev_req_elapsed_time
        let sum = (*used_time).iter().sum::<Duration>();
        let mean = (sum as Duration / count as u32).as_nanos();
        let variance: u128 = (*used_time)
            .iter()
            .map(|x| {
                let diff: i128 = (*x).as_nanos() as i128 - mean as i128;
                (diff * diff) as u128
            })
            .sum::<u128>()
            / count as u128;
        let stdev = variance.sqrt();
        let mut stdev_req_used_time = self.stdev_req_used_time.lock().await;
        *stdev_req_used_time = Duration::from_nanos(stdev as u64);
    }

    async fn calculate_throughput(&self, connections: u16) {
        let avg_req_used_time = self.avg_req_used_time.lock().await;
        let mut throughput = self.throughput.lock().await;
        let sec = (*avg_req_used_time).as_secs_f64();
        if sec > 0f64 {
            *throughput = connections as f64 / sec;
        }
    }

    async fn calculate_latencies(&self, percentiles: Vec<f32>) {
        let mut used_time = self.used_time.lock().await;
        if used_time.is_empty() {
            return;
        }
        if !used_time.is_sorted() {
            used_time.sort();
        }

        let mut latencies = self.latencies.lock().await;
        let count = used_time.len();
        for percent in percentiles {
            let percent_len = (count as f32 * percent) as usize;
            if percent_len > count || percent_len == 0 {
                continue;
            }
            let percent_elapsed_time: &[Duration] =
                &(*used_time)[..percent_len];
            let sum = percent_elapsed_time.iter().sum::<Duration>();
            latencies.push((percent, sum / percent_len as u32));
        }
    }

    async fn clear_temporary_data(&self) {
        let mut used_time = self.used_time.lock().await;
        used_time.clear();
        used_time.shrink_to(0);
    }

    /// need to manually call this method for statistical summary
    pub(crate) async fn summary(
        &self,
        connections: u16,
        percentiles: Vec<f32>,
    ) {
        self.calculate_max_per_second().await;
        self.calculate_avg_per_second().await;
        self.calculate_elapsed_time().await;
        self.calculate_stdev_per_second().await;
        self.calculate_throughput(connections).await;
        self.calculate_latencies(percentiles).await;
        self.clear_temporary_data().await;
    }
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics::new()
    }
}

/// Message entity for [Statistics]
#[derive(Debug)]
pub(crate) struct Message {
    rsp_at: Instant,
    req_at: Instant,
    response: Result<Response, reqwest::Error>,
}

impl Message {
    /// construct message
    pub(crate) fn new(
        response: Result<Response, reqwest::Error>,
        req_at: Instant,
        rsp_at: Instant,
    ) -> Message {
        Self {
            rsp_at,
            req_at,
            response,
        }
    }
}

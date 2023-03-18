//! output module defines the output of the task
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
//! let output_format = arg.output_format;
//! let task = Arc::new(Task::new(arg, Some(pb))?).run()?;
//! let result = match output_format {
//!     OutputFormat::Text => task.text_output()?,
//!     OutputFormat::Json => {
//!         let output = task.json_output()?;
//!         serde_json::to_string_pretty(&output)?
//!     }
//! };
//! writeln!(&mut io::stdout(), "{result}")?;
//! ```

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Write};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::runtime;

use crate::statistics::Statistics;
use crate::Arg;

/// the [Output] after executing the task, copied from the statistical results
#[derive(Debug, Deserialize, Serialize)]
pub struct Output {
    /// average per second, data is sampled every second and averaged at the
    /// end
    pub avg_req_per_second: f64,

    /// the data source is the same as `avg_req_per_second`，just
    /// calculates its standard deviation
    pub stdev_per_second: f64,

    /// the data source is the same as `avg_req_per_second`, find its maximum
    /// value
    pub max_req_per_second: f64,

    /// during the running of the program, the time taken for each request from
    /// initiating to receiving the response will be recorded, and the average
    /// value will be calculated at the end
    pub avg_req_used_time: Micros,

    /// the data source is the same as `avg_req_used_time`, just calculates its
    /// standard deviation
    pub stdev_req_used_time: Micros,

    /// the data source is the same as `avg_req_used_time`, find its maximum
    /// value
    pub max_req_used_time: Micros,

    /// sorts the response time of each request, then calculates it based on
    /// the incoming percentage sequence parameter
    pub latencies: Vec<Latency>,

    /// status code [100, 200)
    pub rsp1xx: u64,

    /// status code [200, 300)
    pub rsp2xx: u64,

    /// status code [300, 400)
    pub rsp3xx: u64,

    /// status code [400, 500)
    pub rsp4xx: u64,

    /// status code [500, 511]
    pub rsp5xx: u64,

    /// other response code
    pub rsp_others: u64,

    /// errors encountered during the request and their count
    pub errors: HashMap<String, u64>,

    /// Calculate the throughput of the Server, the calculation formula is:
    /// `connections / avg_req_used_time`
    pub throughput: f64,
}

impl Output {
    pub(crate) async fn from_statistics(s: &Statistics) -> Self {
        Self {
            avg_req_per_second: *(s.avg_req_per_second.lock().await),
            stdev_per_second: *(s.stdev_per_second.lock().await),
            max_req_per_second: *(s.max_req_per_second.lock().await),
            avg_req_used_time: (*(s.avg_req_used_time.lock().await)).into(),
            stdev_req_used_time: (*(s.stdev_req_used_time.lock().await)).into(),
            max_req_used_time: (*(s.max_req_used_time.lock().await)).into(),
            latencies: (*(s.latencies.lock().await).clone())
                .to_owned()
                .iter()
                .map(|x| Latency::new(x.0, x.1.into()))
                .collect(),
            rsp1xx: s.rsp1xx.load(Ordering::Acquire),
            rsp2xx: s.rsp2xx.load(Ordering::Acquire),
            rsp3xx: s.rsp3xx.load(Ordering::Acquire),
            rsp4xx: s.rsp4xx.load(Ordering::Acquire),
            rsp5xx: s.rsp5xx.load(Ordering::Acquire),
            rsp_others: s.rsp_others.load(Ordering::Acquire),
            errors: ((s.errors.lock().await).clone().to_owned()).to_owned(),
            throughput: *(s.throughput.lock().await),
        }
    }

    pub(crate) fn sync_from_statistics(s: &Statistics) -> anyhow::Result<Self> {
        runtime::Builder::new_current_thread()
            .build()?
            .block_on(async { Ok(Self::from_statistics(s).await) })
    }
}

/// Latency indicates how many seconds the first percentage of requests took
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Latency {
    /// values from 0 to 1
    pub percent: f32,
    /// indicates the time taken to execute the request, unit: microseconds
    pub micros: Micros,
}

impl Latency {
    /// construct [Latency]
    ///
    /// Arguments:
    ///
    /// * `percent` - values from 0 to 1
    /// * `micros` - indicates the time taken to execute the request, unit:
    ///   microseconds
    pub fn new(percent: f32, micros: Micros) -> Self {
        Self { percent, micros }
    }
}

/// Micros represents microseconds
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Micros(u64);

impl From<Duration> for Micros {
    fn from(duration: Duration) -> Self {
        Self(duration.as_micros() as u64)
    }
}

impl From<&Duration> for Micros {
    fn from(duration: &Duration) -> Self {
        Self(duration.as_micros() as u64)
    }
}

impl Display for Micros {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let duration = Duration::from_micros(self.0);
        write!(f, "{duration:.2?}")
    }
}

pub(crate) fn sync_text_output(
    s: &Statistics,
    arg: &Arg,
) -> anyhow::Result<String> {
    runtime::Builder::new_current_thread()
        .build()?
        .block_on(text_output(s, arg))
}

pub(crate) async fn text_output(
    s: &Statistics,
    arg: &Arg,
) -> anyhow::Result<String> {
    let mut output = String::new();
    writeln!(
        &mut output,
        "{:<14}{:^14}{:^14}{:^14}
  {:<12}{:^14.2}{:^14.2}{:^14.2}
  {:<12}{:^14}{:^14}{:^14}",
        "Statistics",
        "Avg",
        "Stdev",
        "Max",
        "Reqs/sec",
        *(s.avg_req_per_second.lock().await),
        *(s.stdev_per_second.lock().await),
        *(s.max_req_per_second.lock().await),
        "Latency",
        format!("{:.2?}", *(s.avg_req_used_time.lock().await)),
        format!("{:.2?}", *(s.stdev_req_used_time.lock().await)),
        format!("{:.2?}", *(s.max_req_used_time.lock().await)),
    )?;

    if arg.latencies {
        let latencies = &*(s.latencies.lock().await);
        if !latencies.is_empty() {
            writeln!(&mut output, "  {:<20}", "Latency Distribution")?;
            for (percent, duration) in latencies {
                writeln!(
                    &mut output,
                    "  {:^10}{:^10}",
                    format!("{:.0}%", *percent * 100f32),
                    format!("{:.2?}", *duration),
                )?;
            }
        }
    }

    writeln!(&mut output, "  {:<20}", "HTTP codes:")?;
    writeln!(
        &mut output,
        "    1XX - {}, 2XX - {}, 3XX - {}, 4XX - {}, 5XX - {}",
        s.rsp1xx.load(Ordering::Acquire),
        s.rsp2xx.load(Ordering::Acquire),
        s.rsp3xx.load(Ordering::Acquire),
        s.rsp4xx.load(Ordering::Acquire),
        s.rsp5xx.load(Ordering::Acquire),
    )?;
    writeln!(
        &mut output,
        "    others - {}",
        s.rsp_others.load(Ordering::Acquire)
    )?;

    let errors = s.errors.lock().await;
    if !errors.is_empty() {
        writeln!(&mut output, "  {:<10}", "Errors:")?;
        for (k, v) in &*errors {
            writeln!(&mut output, "    \"{k:>}\":{v:>8}")?;
        }
    }
    write!(
        &mut output,
        "  {:<12}{:>10.2}/s",
        "Throughput:",
        *(s.throughput.lock().await)
    )?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_micros_convert() {
        let duration = Duration::from_micros(1);
        let micros: Micros = duration.into();
        assert_eq!("1.00µs", format!("{micros}"));

        let duration = Duration::from_millis(1);
        let micros: Micros = (&duration).into();
        assert_eq!("1.00ms", format!("{micros}"));

        let duration = Duration::from_millis(1);
        let micros = Micros::from(duration);
        assert_eq!("1.00ms", format!("{micros}"));
    }
}

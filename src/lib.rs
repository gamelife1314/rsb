#![feature(is_sorted)]
#![feature(associated_type_defaults)]
#![deny(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::single_char_pattern))]
//! rsb is a http server benchmark tool.

pub mod arg;
pub(crate) mod client;
pub(crate) mod dispatcher;
pub(crate) mod limiter;
pub mod output;
pub(crate) mod request;
pub(crate) mod statistics;
pub mod task;

pub use self::arg::Arg;
pub use self::output::Output;
pub use self::task::Task;

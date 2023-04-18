use std::io::{self, Write};
use std::sync::Arc;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use rsb::{arg::OutputFormat, Arg, Task};

#[cfg(not(tarpaulin_include))]
fn create_count_progress_bar(arg: &Arg) -> ProgressBar {
    let pb = ProgressBar::new(arg.requests.unwrap());
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {pos}/{len} \
            ({per_sec}, {percent}%, {eta}) {msg}",
        )
            .unwrap()
            .with_key("per_sec", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                write!(w, "{:.0}/s", state.per_sec()).unwrap()
            })
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ])
            .progress_chars("#>-"),
    );
    pb
}

#[cfg(not(tarpaulin_include))]
fn create_duration_progress_bar(arg: &Arg) -> ProgressBar {
    let pb = ProgressBar::new(arg.duration.unwrap().as_secs());
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {pos}s/{len}s ({percent}%) {msg}",
        )
            .unwrap()
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ])
        .progress_chars("#>-"),
    );
    pb
}

#[cfg(not(tarpaulin_include))]
fn create_progress_bar(arg: &Arg) -> ProgressBar {
    if arg.requests.is_some() {
        create_count_progress_bar(arg)
    } else {
        create_duration_progress_bar(arg)
    }
}

#[cfg(not(tarpaulin_include))]
fn print_tip(arg: &Arg) -> anyhow::Result<()> {
    if arg.requests.is_some() {
        writeln!(
            &mut io::stdout(),
            "{:?} {:?} with {} requests using {} connections",
            arg.method,
            arg.url.clone().unwrap(),
            arg.requests.unwrap(),
            arg.connections
        )?;
    } else if arg.duration.is_some() {
        writeln!(
            &mut io::stdout(),
            "{:?} {:?} with for {:?} using {} connections",
            arg.method,
            arg.url.clone().unwrap(),
            arg.duration.unwrap(),
            arg.connections
        )?;
    }
    Ok(())
}

#[cfg(not(tarpaulin_include))]
fn main() -> anyhow::Result<()> {
    env_logger::init();
    let arg = Arg::parse();

    if let Some(shell) = arg.completions {
        let mut cmd = Arg::command();
        let app_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, app_name, &mut io::stdout());
        std::process::exit(0);
    }

    rlimit::increase_nofile_limit(u64::MAX).unwrap();

    print_tip(&arg)?;
    let pb = create_progress_bar(&arg);
    let output_format = arg.output_format;
    let task = Arc::new(Task::new(arg, Some(pb))?).run()?;
    let result = match output_format {
        OutputFormat::Text => task.text_output()?,
        OutputFormat::Json => {
            let output = task.json_output()?;
            serde_json::to_string_pretty(&output)?
        },
    };
    writeln!(&mut io::stdout(), "{result}")?;
    Ok(())
}

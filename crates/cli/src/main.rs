//! Mul-Tor, command-line version.

mod app;
mod args;
mod commands;
mod interactive;
mod logging;
mod output;
mod paths;
mod runner;
mod selection;

use std::process::ExitCode;

use clap::Parser;
use console::style;

use crate::args::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match commands::dispatch(cli).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{} {error:#}", style("error:").red().bold());
            ExitCode::from(2)
        }
    }
}

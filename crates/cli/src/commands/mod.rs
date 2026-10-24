//! One module per command. `dispatch` picks the one the user asked for.

mod check;
mod config;
mod logs;
mod presets;
mod providers;
mod upload;

use std::process::ExitCode;

use anyhow::Result;

use mul_tor_core::logging::LogBuffer;

use crate::app::App;
use crate::args::{Cli, Command};
use crate::{logging, tui};

/// Records kept for the log pane of the full-screen interface.
const LOG_LINES: usize = 2000;

pub async fn dispatch(cli: Cli) -> Result<ExitCode> {
    let app = App::load(cli.home.as_deref(), false)?;
    // The full-screen interface owns the terminal, so its log goes to a buffer
    // it can show instead of to stderr.
    let logs = LogBuffer::new(LOG_LINES);
    let _log = logging::init(
        &app,
        cli.verbose,
        cli.command.is_none().then(|| logs.clone()),
    );

    match cli.command {
        None => tui::run(app, logs).await,
        Some(Command::Upload(args)) => upload::upload(app, args).await,
        Some(Command::Providers(args)) => {
            providers::providers(&app, args.format).map(|()| ExitCode::SUCCESS)
        }
        Some(Command::Presets) => presets::presets(&app).map(|()| ExitCode::SUCCESS),
        Some(Command::Check) => check::check(&app).await.map(|()| ExitCode::SUCCESS),
        Some(Command::Config(args)) => {
            config::config(&app, args.action).map(|()| ExitCode::SUCCESS)
        }
        Some(Command::Logs(args)) => logs::logs(&app, args).map(|()| ExitCode::SUCCESS),
    }
}

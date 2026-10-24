//! One module per command. `dispatch` picks the one the user asked for.

mod check;
mod config;
mod logs;
mod presets;
mod providers;
mod upload;

use std::process::ExitCode;

use anyhow::Result;

use crate::app::App;
use crate::args::{Cli, Command};
use crate::logging;

pub async fn dispatch(cli: Cli) -> Result<ExitCode> {
    let app = App::load(cli.home.as_deref(), false)?;
    let _log = logging::init(&app, cli.verbose, None);

    match cli.command {
        None => crate::interactive::run(app).await,
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

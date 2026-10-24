//! The full-screen terminal interface (`mul-tor` with no arguments).
//!
//! The pieces, from the inside out:
//!
//! - [`model`] holds what is on screen and turns [`msg::Msg`]s into changes and
//!   [`msg::Cmd`]s. It knows nothing about terminals or threads.
//! - [`view`] draws the model.
//! - [`runtime`] carries out the commands (uploads, probes, the clipboard) and
//!   reports back with messages; [`events`] feeds in the keyboard and a clock;
//!   [`terminal`] owns the screen.

mod events;
mod format;
mod input;
mod model;
mod msg;
mod runtime;
mod terminal;
#[cfg(test)]
mod testing;
mod theme;
mod view;

use std::io::{IsTerminal, stdin, stdout};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Result, bail};
use mul_tor_core::logging::LogBuffer;
use tokio::sync::mpsc;

use self::events::Input;
use self::model::Model;
use self::runtime::Runtime;
use self::terminal::Tui;
use self::theme::Theme;
use crate::app::App;

/// Run the interface until the user leaves. The exit code is 0, 1 if an
/// upload failed, or 130 if the user interrupted a running one.
pub async fn run(app: App, logs: LogBuffer) -> Result<ExitCode> {
    if !stdin().is_terminal() || !stdout().is_terminal() {
        bail!("this mode needs a terminal; use `mul-tor upload --help` for scripted use");
    }
    let app = Arc::new(app);
    let theme = Theme::detect();
    let start_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (mut model, startup) = Model::new(&app, logs, start_dir);

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut runtime = Runtime::new(app, tx.clone());
    runtime.exec(startup);

    let mut tui = Tui::enter()?;
    let mut input = Input::spawn(tx.clone());
    let clock = events::spawn_clock(tx);

    let mut result = tui.draw(|frame| view::draw(frame, &mut model, &theme));
    while result.is_ok() && !model.quit {
        let Some(first) = rx.recv().await else { break };
        let mut commands = model.update(first);
        // Everything that piled up while drawing, before drawing again.
        while let Ok(more) = rx.try_recv() {
            commands.extend(model.update(more));
        }
        runtime.exec(commands);
        if model.quit {
            break;
        }
        result = tui.draw(|frame| view::draw(frame, &mut model, &theme));
    }

    clock.abort();
    input.stop();
    runtime.shutdown();
    drop(tui);
    result?;

    print!("{}", model.farewell());
    Ok(ExitCode::from(model.exit_code))
}

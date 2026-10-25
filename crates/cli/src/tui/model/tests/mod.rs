//! The interface as a person uses it: messages in, commands out.

mod flow;
mod interface;

use std::sync::Arc;

use crossterm::event::KeyCode;
use mul_tor_core::{Event, JobOutcome, UploadError, UploadedFile};

use crate::tui::msg::{Cmd, Msg};
use crate::tui::testing::*;

pub fn finished(job: usize, result: Result<UploadedFile, UploadError>) -> Msg {
    Msg::Engine(Event::Finished(Arc::new(JobOutcome {
        job,
        file_name: "backup.7z".into(),
        provider: "Gofile",
        result,
    })))
}

/// Add the file, tick every provider that bulk choices take, and confirm.
pub fn start_upload(f: &mut Fixture) -> Vec<Cmd> {
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    press(&mut f.model, KeyCode::Char('a'));
    press(&mut f.model, KeyCode::Enter);
    press(&mut f.model, KeyCode::Enter)
}

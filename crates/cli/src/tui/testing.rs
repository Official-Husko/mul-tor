//! Helpers for the interface tests: a model on a scratch folder, and key presses.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mul_tor_core::Config;
use mul_tor_core::logging::LogBuffer;

use super::model::Model;
use super::msg::{Cmd, Msg};
use crate::app::test_app;

pub struct Fixture {
    pub _dir: tempfile::TempDir,
    pub file: PathBuf,
    pub model: Model,
    pub start: Vec<Cmd>,
}

pub fn fixture_with(config: Config) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("backup.7z");
    std::fs::write(&file, "payload").unwrap();
    let (model, start) = Model::new(
        &test_app(config),
        LogBuffer::new(50),
        dir.path().to_path_buf(),
    );
    Fixture {
        _dir: dir,
        file,
        model,
        start,
    }
}

pub fn fixture() -> Fixture {
    fixture_with(Config::default())
}

pub fn press(model: &mut Model, code: KeyCode) -> Vec<Cmd> {
    model.update(Msg::Key(KeyEvent::new(code, KeyModifiers::NONE)))
}

pub fn ctrl_c(model: &mut Model) -> Vec<Cmd> {
    model.update(Msg::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    )))
}

pub fn text(model: &mut Model, keys: &str) {
    for c in keys.chars() {
        press(model, KeyCode::Char(c));
    }
}

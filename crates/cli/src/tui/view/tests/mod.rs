//! Drawing checked against a fake terminal.

mod overlays;
mod robustness;
mod screens;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use super::draw;
use crate::tui::model::Model;
use crate::tui::theme::{ColorMode, Theme};

pub fn theme() -> Theme {
    Theme::new(ColorMode::Mono)
}

/// The screen as text, one line per row, without trailing blanks.
pub fn text_of(buffer: &Buffer) -> String {
    let area = buffer.area;
    (0..area.height)
        .map(|y| {
            let row: String = (0..area.width).map(|x| buffer[(x, y)].symbol()).collect();
            row.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Draw `model` on a `width` x `height` terminal and return what is on it.
pub fn render(model: &mut Model, width: u16, height: u16) -> String {
    render_with(model, width, height, &theme())
}

pub fn render_with(model: &mut Model, width: u16, height: u16, theme: &Theme) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, model, theme)).unwrap();
    text_of(terminal.backend().buffer())
}

pub fn assert_shows(screen: &str, wanted: &[&str]) {
    for text in wanted {
        assert!(screen.contains(text), "missing `{text}` on:\n{screen}");
    }
}

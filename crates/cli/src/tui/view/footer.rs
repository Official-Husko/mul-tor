//! The status line and the key hints.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::widgets::fit;
use crate::tui::model::{Model, Pane, Screen};
use crate::tui::msg::Level;
use crate::tui::theme::Theme;

pub fn draw(frame: &mut Frame, status: Rect, keys: Rect, model: &Model, theme: &Theme) {
    frame.render_widget(
        Paragraph::new(status_line(model, status.width as usize, theme)),
        status,
    );
    frame.render_widget(
        Paragraph::new(hint_line(&hints(model), theme)).alignment(Alignment::Left),
        keys,
    );
}

fn status_line<'a>(model: &Model, width: usize, theme: &Theme) -> Line<'a> {
    if let Some(notice) = &model.notice {
        let style = match notice.level {
            Level::Info => theme.text(),
            Level::Warn => theme.warn(),
            Level::Error => theme.error(),
        };
        return Line::styled(fit(&format!(" {}", notice.text), width), style);
    }
    let text = match &model.update {
        Some(release) => format!(" Update available: {} - {}", release.name, release.url),
        None => format!(
            " config {}  ·  logs {}",
            model.settings.config_path.display(),
            model.settings.log_dir.display()
        ),
    };
    let style = if model.update.is_some() {
        theme.link()
    } else {
        theme.dim()
    };
    Line::styled(fit(&text, width), style)
}

fn hint_line<'a>(hints: &[(&'static str, &'static str)], theme: &Theme) -> Line<'a> {
    let mut spans = Vec::new();
    for (key, what) in hints {
        spans.push(Span::styled(format!(" {key} "), theme.key()));
        spans.push(Span::styled(format!(" {what}  "), theme.dim()));
    }
    Line::from(spans)
}

/// The keys that do something right now.
fn hints(model: &Model) -> Vec<(&'static str, &'static str)> {
    if model.overlay.is_some() {
        return vec![("Esc", "close")];
    }
    match model.screen {
        Screen::Files => match model.files.pane {
            Pane::Path => vec![("Enter", "add"), ("Esc", "cancel")],
            Pane::Browser => vec![
                ("↑↓", "move"),
                ("Enter", "open/add"),
                ("Space", "toggle"),
                ("/", "type path"),
                ("Tab", "list"),
                ("n", "next"),
                ("?", "help"),
            ],
            Pane::Basket => vec![
                ("↑↓", "move"),
                ("x", "remove"),
                ("c", "clear"),
                ("Tab", "browse"),
                ("n", "next"),
            ],
        },
        Screen::Providers if model.providers.filtering => {
            vec![("Enter", "keep filter"), ("Esc", "clear")]
        }
        Screen::Providers => vec![
            ("Space", "tick"),
            ("a", "all"),
            ("d", "none"),
            ("/", "filter"),
            ("p", "presets"),
            ("Enter", "upload"),
            ("Esc", "back"),
            ("?", "help"),
        ],
        Screen::Upload => vec![
            ("↑↓", "scroll"),
            ("Esc", "cancel"),
            ("L", "log"),
            ("?", "help"),
        ],
        Screen::Results => vec![
            ("↑↓", "move"),
            ("c", "copy links"),
            ("f", "copy formatted"),
            ("n", "new"),
            ("b", "providers"),
            ("q", "quit"),
        ],
    }
}

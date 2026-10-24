//! The top of the screen: the logo (or a one-line title) and the steps.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::logo;
use crate::tui::model::{Model, Screen};
use crate::tui::theme::Theme;

/// Rows the header takes: the full banner plus the steps, or only the steps.
pub fn height(area: Rect) -> u16 {
    if full_banner(area) {
        logo::HEIGHT + 2
    } else {
        1
    }
}

fn full_banner(area: Rect) -> bool {
    area.height >= 32 && area.width >= logo::WIDTH + 2
}

pub fn draw(frame: &mut Frame, area: Rect, model: &Model, theme: &Theme) {
    let banner = full_banner(frame.area());
    let mut top = area;
    if banner {
        let logo_area = Rect {
            height: logo::HEIGHT,
            ..area
        };
        frame.render_widget(
            Paragraph::new(logo::lines(theme)).alignment(Alignment::Center),
            logo_area,
        );
        let version = Line::from(vec![
            Span::styled(format!("v{}", env!("CARGO_PKG_VERSION")), theme.accent()),
            Span::styled("  ·  upload to many hosts at once", theme.dim()),
        ]);
        let version_area = Rect {
            y: area.y + logo::HEIGHT,
            height: 1,
            ..area
        };
        frame.render_widget(
            Paragraph::new(version).alignment(Alignment::Center),
            version_area,
        );
        top = Rect {
            y: area.y + logo::HEIGHT + 1,
            height: 1,
            ..area
        };
    }
    let mut spans = Vec::new();
    if !banner {
        spans.extend(logo::wordmark(theme));
        spans.push(Span::raw("  "));
    }
    spans.extend(steps(model, theme));
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(if banner {
            Alignment::Center
        } else {
            Alignment::Left
        }),
        top,
    );
}

const STEPS: [(Screen, &str); 4] = [
    (Screen::Files, "Files"),
    (Screen::Providers, "Providers"),
    (Screen::Upload, "Upload"),
    (Screen::Results, "Results"),
];

fn steps<'a>(model: &Model, theme: &Theme) -> Vec<Span<'a>> {
    let current = STEPS
        .iter()
        .position(|(screen, _)| *screen == model.screen)
        .unwrap_or(0);
    let mut spans = Vec::new();
    for (index, (_, name)) in STEPS.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" › ", theme.dim()));
        }
        let label = format!("{} {name}", index + 1);
        spans.push(if index == current {
            Span::styled(label, theme.accent().add_modifier(Modifier::UNDERLINED))
        } else if index < current {
            Span::styled(format!("✓ {name}"), theme.ok())
        } else {
            Span::styled(label, theme.dim())
        });
    }
    spans
}

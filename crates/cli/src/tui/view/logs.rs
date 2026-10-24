//! The log pane: the newest records, coloured by level.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use tracing::Level;

use super::widgets::{fit, panel, title};
use crate::tui::model::Model;
use crate::tui::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, model: &Model, theme: &Theme) {
    let block = panel(title("Log · L hides", theme), false, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let lines: Vec<Line> = model
        .logs
        .tail(inner.height as usize)
        .iter()
        .map(|record| {
            let level_style = match record.level {
                Level::ERROR => theme.error(),
                Level::WARN => theme.warn(),
                Level::INFO => theme.ok(),
                _ => theme.dim(),
            };
            let mut message = record.message.clone();
            if !record.fields.is_empty() {
                message.push(' ');
                message.push_str(&record.fields_text());
            }
            let prefix = format!(
                "{} {:>5} ",
                record.time.format("%H:%M:%S%.3f"),
                record.level
            );
            let room = width.saturating_sub(prefix.chars().count());
            Line::from(vec![
                Span::styled(record.time.format("%H:%M:%S%.3f ").to_string(), theme.dim()),
                Span::styled(format!("{:>5} ", record.level), level_style),
                Span::styled(fit(&message, room), theme.text()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

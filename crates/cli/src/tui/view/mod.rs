//! Drawing the model. Nothing here changes it, apart from the scroll
//! positions that the list widgets keep.

mod files;
mod footer;
mod header;
mod logo;
mod logs;
mod overlay;
mod provider_details;
mod providers;
mod results;
#[cfg(test)]
mod tests;
mod upload;
mod widgets;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use super::model::{Model, Screen};
use super::theme::Theme;

/// Smaller than this and nothing fits; say so instead of drawing garbage.
const MIN_WIDTH: u16 = 56;
const MIN_HEIGHT: u16 = 14;

pub fn draw(frame: &mut Frame, model: &mut Model, theme: &Theme) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let message = Paragraph::new(vec![
            Line::styled("Window too small", theme.warn()),
            Line::styled(
                format!(
                    "needs {MIN_WIDTH}x{MIN_HEIGHT}, has {}x{}",
                    area.width, area.height
                ),
                theme.dim(),
            ),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(message, area);
        return;
    }

    let log_height = if model.show_logs {
        (area.height / 4).clamp(5, 12)
    } else {
        0
    };
    let [head, body, log, status, keys] = Layout::vertical([
        Constraint::Length(header::height(area)),
        Constraint::Min(5),
        Constraint::Length(log_height),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    header::draw(frame, head, model, theme);
    match model.screen {
        Screen::Files => files::draw(frame, body, model, theme),
        Screen::Providers => providers::draw(frame, body, model, theme),
        Screen::Upload => upload::draw(frame, body, model, theme),
        Screen::Results => results::draw(frame, body, model, theme),
    }
    if log_height > 0 {
        logs::draw(frame, log, model, theme);
    }
    footer::draw(frame, status, keys, model, theme);
    overlay::draw(frame, model, theme);
}

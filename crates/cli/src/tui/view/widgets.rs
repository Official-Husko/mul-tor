//! Small building blocks the screens share.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType};

use crate::tui::theme::Theme;

/// A rounded panel with a title; the border brightens when it has focus.
pub fn panel<'a>(title: impl Into<Line<'a>>, focused: bool, theme: &Theme) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border(focused))
        .title(title.into())
}

/// A title with the accent colour, padded with a space on each side.
pub fn title<'a>(text: impl Into<String>, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(format!(" {} ", text.into()), theme.accent()))
}

/// A rectangle of `width` x `height` in the middle of `area` (or all of it,
/// if it is smaller).
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// `████████░░░░`: `fraction` (0 to 1) of `width` cells filled, the fill
/// shading along the theme's gradient.
pub fn bar<'a>(fraction: f64, width: u16, theme: &Theme) -> Vec<Span<'a>> {
    let width = width as usize;
    let filled = ((fraction.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    let mut spans: Vec<Span> = (0..filled)
        .map(|cell| {
            let at = if width > 1 {
                cell as f32 / (width - 1) as f32
            } else {
                0.0
            };
            Span::styled("█", Style::new().fg(theme.gradient_at(at)))
        })
        .collect();
    spans.push(Span::styled("░".repeat(width - filled), theme.dim()));
    spans
}

/// A coloured one-word label.
pub fn badge<'a>(text: impl Into<String>, style: Style) -> Span<'a> {
    Span::styled(text.into(), style)
}

/// Pad or cut `text` to exactly `width` columns.
pub fn fit(text: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push_str(&" ".repeat(width - used));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::theme::ColorMode;

    #[test]
    fn centring_never_leaves_the_area() {
        let area = Rect::new(0, 0, 100, 40);
        assert_eq!(centered(area, 20, 10), Rect::new(40, 15, 20, 10));
        assert_eq!(centered(area, 500, 500), area);
    }

    #[test]
    fn a_bar_is_always_exactly_as_wide_as_asked() {
        let theme = Theme::new(ColorMode::Mono);
        for fraction in [0.0, 0.33, 0.5, 1.0, 7.0, -1.0] {
            let text: String = bar(fraction, 10, &theme)
                .iter()
                .map(|s| s.content.to_string())
                .collect();
            assert_eq!(text.chars().count(), 10, "{fraction}");
        }
        let half: String = bar(0.5, 10, &theme)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(half, "█████░░░░░");
    }

    #[test]
    fn fitting_pads_cuts_and_counts_wide_characters() {
        assert_eq!(fit("ab", 4), "ab  ");
        assert_eq!(fit("abcdef", 4), "abcd");
        assert_eq!(
            fit("日本", 3),
            "日 ",
            "a wide character that does not fit is dropped"
        );
        assert_eq!(fit("x", 0), "");
    }
}

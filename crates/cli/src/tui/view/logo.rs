//! The banner, painted along the theme's gradient.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::tui::theme::Theme;

const LOGO: [&str; 6] = [
    ".88b  d88. db    db db             d888888b  .d88b.  d8888b.",
    "88'YbdP`88 88    88 88             `~~88~~' .8P  Y8. 88  `8D",
    "88  88  88 88    88 88                88    88    88 88oobY'",
    "88  88  88 88    88 88      C8888D    88    88    88 88`8b  ",
    "88  88  88 88b  d88 88booo.           88    `8b  d8' 88 `88.",
    "YP  YP  YP ~Y8888P' Y88888P           YP     `Y88P'  88   YD",
];

/// Width of the widest logo line.
pub const WIDTH: u16 = 62;

/// Rows the full logo needs.
pub const HEIGHT: u16 = LOGO.len() as u16;

/// The logo, one gradient-coloured line per row.
pub fn lines<'a>(theme: &Theme) -> Vec<Line<'a>> {
    LOGO.iter()
        .map(|row| {
            let columns = row.chars().count().max(1);
            let spans: Vec<Span> = row
                .chars()
                .enumerate()
                .map(|(column, c)| {
                    let at = column as f32 / (columns - 1).max(1) as f32;
                    Span::styled(c.to_string(), Style::new().fg(theme.gradient_at(at)))
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

/// `MUL-TOR`, gradient-coloured, for terminals too small for the logo.
pub fn wordmark<'a>(theme: &Theme) -> Vec<Span<'a>> {
    let text = "MUL-TOR";
    text.chars()
        .enumerate()
        .map(|(index, c)| {
            let at = index as f32 / (text.len() - 1) as f32;
            Span::styled(
                c.to_string(),
                Style::new()
                    .fg(theme.gradient_at(at))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::theme::ColorMode;

    #[test]
    fn every_row_fits_the_declared_width() {
        for row in LOGO {
            assert!(row.chars().count() as u16 <= WIDTH, "{row}");
        }
    }

    #[test]
    fn lines_keep_the_text() {
        let theme = Theme::new(ColorMode::TrueColor);
        let first: String = lines(&theme)[0]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(first, LOGO[0]);
        let word: String = wordmark(&theme)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(word, "MUL-TOR");
    }
}

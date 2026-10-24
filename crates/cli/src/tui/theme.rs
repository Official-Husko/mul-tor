//! Colours. Three palettes: true colour where the terminal says it can, the
//! sixteen standard colours (which follow the user's terminal theme) elsewhere,
//! and no colour at all when `NO_COLOR` is set.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi,
    Mono,
}

impl ColorMode {
    /// Decide from the environment.
    pub fn detect() -> Self {
        Self::from_env(|name| std::env::var(name).ok())
    }

    fn from_env(var: impl Fn(&str) -> Option<String>) -> Self {
        if var("NO_COLOR").is_some_and(|value| !value.is_empty()) {
            return ColorMode::Mono;
        }
        let truecolor = var("COLORTERM").is_some_and(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("truecolor") || v.contains("24bit")
        });
        // Windows Terminal does not set COLORTERM but does support it.
        let known_good = var("WT_SESSION").is_some()
            || var("TERM_PROGRAM")
                .is_some_and(|p| matches!(p.as_str(), "iTerm.app" | "vscode" | "WezTerm"));
        if truecolor || known_good {
            ColorMode::TrueColor
        } else {
            ColorMode::Ansi
        }
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ColorMode,
    pub accent: Color,
    pub accent_alt: Color,
    pub ok: Color,
    pub warn: Color,
    pub error: Color,
    pub dim: Color,
    pub text: Color,
    pub selection_bg: Color,
    gradient: (Color, Color),
}

impl Theme {
    pub fn new(mode: ColorMode) -> Self {
        match mode {
            ColorMode::TrueColor => Self {
                mode,
                accent: Color::Rgb(0x7a, 0xa2, 0xf7),
                accent_alt: Color::Rgb(0xbb, 0x9a, 0xf7),
                ok: Color::Rgb(0x9e, 0xce, 0x6a),
                warn: Color::Rgb(0xe0, 0xaf, 0x68),
                error: Color::Rgb(0xf7, 0x76, 0x8e),
                dim: Color::Rgb(0x73, 0x7a, 0xa2),
                text: Color::Rgb(0xc0, 0xca, 0xf5),
                selection_bg: Color::Rgb(0x28, 0x34, 0x57),
                gradient: (Color::Rgb(0xff, 0x5f, 0x87), Color::Rgb(0x5f, 0xd7, 0xff)),
            },
            ColorMode::Ansi => Self {
                mode,
                accent: Color::Cyan,
                accent_alt: Color::Magenta,
                ok: Color::Green,
                warn: Color::Yellow,
                error: Color::Red,
                dim: Color::DarkGray,
                text: Color::Reset,
                selection_bg: Color::DarkGray,
                gradient: (Color::Magenta, Color::Cyan),
            },
            ColorMode::Mono => Self {
                mode,
                accent: Color::Reset,
                accent_alt: Color::Reset,
                ok: Color::Reset,
                warn: Color::Reset,
                error: Color::Reset,
                dim: Color::Reset,
                text: Color::Reset,
                selection_bg: Color::Reset,
                gradient: (Color::Reset, Color::Reset),
            },
        }
    }

    pub fn detect() -> Self {
        Self::new(ColorMode::detect())
    }

    fn mono(&self) -> bool {
        self.mode == ColorMode::Mono
    }

    pub fn text(&self) -> Style {
        Style::new().fg(self.text)
    }

    pub fn dim(&self) -> Style {
        if self.mono() {
            Style::new().add_modifier(Modifier::DIM)
        } else {
            Style::new().fg(self.dim)
        }
    }

    pub fn accent(&self) -> Style {
        Style::new().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    /// A web address: the second accent, underlined.
    pub fn link(&self) -> Style {
        Style::new()
            .fg(self.accent_alt)
            .add_modifier(Modifier::UNDERLINED)
    }

    pub fn ok(&self) -> Style {
        Style::new().fg(self.ok)
    }

    pub fn warn(&self) -> Style {
        Style::new().fg(self.warn)
    }

    pub fn error(&self) -> Style {
        Style::new().fg(self.error)
    }

    /// The row under the cursor. Reverse video when there are no colours.
    pub fn selected(&self) -> Style {
        if self.mono() {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
                .bg(self.selection_bg)
                .add_modifier(Modifier::BOLD)
        }
    }

    /// A key name in the hint bar.
    pub fn key(&self) -> Style {
        if self.mono() {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
                .fg(Color::Black)
                .bg(self.accent)
                .add_modifier(Modifier::BOLD)
        }
    }

    /// A panel border; brighter when the panel has focus.
    pub fn border(&self, focused: bool) -> Style {
        if focused {
            Style::new().fg(self.accent)
        } else {
            self.dim()
        }
    }

    /// Colour `fraction` (0.0 to 1.0) of the way along the logo gradient.
    pub fn gradient_at(&self, fraction: f32) -> Color {
        match (self.gradient.0, self.gradient.1) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let t = fraction.clamp(0.0, 1.0);
                let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
                Color::Rgb(mix(r1, r2), mix(g1, g2), mix(b1, b2))
            }
            (start, end) => {
                if fraction < 0.5 {
                    start
                } else {
                    end
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn no_color_wins_over_everything() {
        let mode = ColorMode::from_env(env(&[("NO_COLOR", "1"), ("COLORTERM", "truecolor")]));
        assert_eq!(mode, ColorMode::Mono);
        assert_eq!(
            ColorMode::from_env(env(&[("NO_COLOR", "")])),
            ColorMode::Ansi
        );
    }

    #[test]
    fn true_colour_is_used_only_when_the_terminal_says_so() {
        assert_eq!(
            ColorMode::from_env(env(&[("COLORTERM", "24bit")])),
            ColorMode::TrueColor
        );
        assert_eq!(
            ColorMode::from_env(env(&[("WT_SESSION", "x")])),
            ColorMode::TrueColor
        );
        assert_eq!(
            ColorMode::from_env(env(&[("TERM", "xterm")])),
            ColorMode::Ansi
        );
    }

    #[test]
    fn the_gradient_runs_from_one_end_to_the_other() {
        let theme = Theme::new(ColorMode::TrueColor);
        assert_eq!(theme.gradient_at(0.0), Color::Rgb(0xff, 0x5f, 0x87));
        assert_eq!(theme.gradient_at(1.0), Color::Rgb(0x5f, 0xd7, 0xff));
        assert_eq!(theme.gradient_at(9.0), theme.gradient_at(1.0));
        let ansi = Theme::new(ColorMode::Ansi);
        assert_ne!(ansi.gradient_at(0.1), ansi.gradient_at(0.9));
    }

    #[test]
    fn mono_uses_only_modifiers() {
        let theme = Theme::new(ColorMode::Mono);
        assert_eq!(theme.ok().fg, Some(Color::Reset));
        assert!(theme.selected().add_modifier.contains(Modifier::REVERSED));
    }
}

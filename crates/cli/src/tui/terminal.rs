//! Taking over the terminal, and giving it back whatever happens.

use std::io::{self, stdout};

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use ratatui::{DefaultTerminal, Frame};

/// Raw mode, the alternate screen and bracketed paste, until dropped.
///
/// ratatui's `try_init` also installs a panic hook that restores the
/// terminal first, so a crash does not leave the shell unusable.
pub struct Tui {
    terminal: DefaultTerminal,
}

impl Tui {
    pub fn enter() -> io::Result<Self> {
        let terminal = ratatui::try_init()?;
        // Bracketed paste is what turns a dropped file into one `Paste` event
        // instead of a burst of key presses. Terminals without it still work.
        let _ = crossterm::execute!(stdout(), EnableBracketedPaste);
        Ok(Self { terminal })
    }

    pub fn draw(&mut self, render: impl FnOnce(&mut Frame)) -> io::Result<()> {
        self.terminal.draw(render).map(|_| ())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = crossterm::execute!(stdout(), DisableBracketedPaste);
        let _ = ratatui::try_restore();
    }
}

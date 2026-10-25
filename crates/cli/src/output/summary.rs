//! The totals at the end of a run.

use std::process::ExitCode;

use console::style;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub ok: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl Summary {
    /// "3 uploaded, 1 failed", coloured.
    pub fn print(&self) {
        let mut parts = vec![format!("{} uploaded", style(self.ok).green().bold())];
        if self.failed > 0 {
            parts.push(format!("{} failed", style(self.failed).red().bold()));
        }
        if self.cancelled > 0 {
            parts.push(format!(
                "{} cancelled",
                style(self.cancelled).yellow().bold()
            ));
        }
        println!("{}", parts.join(", "));
    }

    /// 130 after Ctrl-C (the shell convention), 1 if anything failed, else 0.
    pub fn exit_code(&self, interrupted: bool) -> ExitCode {
        if interrupted {
            ExitCode::from(130)
        } else if self.failed > 0 {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_follow_the_shell_conventions() {
        let clean = Summary {
            ok: 2,
            failed: 0,
            cancelled: 0,
        };
        let failed = Summary {
            ok: 1,
            failed: 1,
            cancelled: 0,
        };
        assert_eq!(clean.exit_code(false), ExitCode::SUCCESS);
        assert_eq!(failed.exit_code(false), ExitCode::from(1));
        assert_eq!(failed.exit_code(true), ExitCode::from(130));
    }
}

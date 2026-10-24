//! Deciding which records are kept.

use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Overrides the level for every output, in `tracing` filter syntax:
/// `MUL_TOR_LOG=debug` or `MUL_TOR_LOG=warn,mul_tor_core::engine=trace`.
pub const ENV_VAR: &str = "MUL_TOR_LOG";

/// Our own crates at `level`, everything else (HTTP stack, runtime) at `warn`.
fn default_directives(level: Level) -> String {
    format!("warn,mul_tor={level},mul_tor_core={level},mul_tor_gui={level}")
}

/// The filter for one output. `MUL_TOR_LOG` wins when it is set and valid.
pub(super) fn build(level: Level) -> EnvFilter {
    let default = default_directives(level);
    match std::env::var(ENV_VAR) {
        Ok(custom) if !custom.trim().is_empty() => {
            EnvFilter::try_new(custom.trim()).unwrap_or_else(|_| EnvFilter::new(default))
        }
        _ => EnvFilter::new(default),
    }
}

/// `error`, `warn`, `info`, `debug` or `trace`, in any case.
pub fn parse_level(text: &str) -> Option<Level> {
    text.trim().parse().ok()
}

/// What `-v`, `-vv`, `-vvv` mean on the terminal.
pub fn console_level(verbosity: u8) -> Option<Level> {
    match verbosity {
        0 => None,
        1 => Some(Level::INFO),
        2 => Some(Level::DEBUG),
        _ => Some(Level::TRACE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_parse_loosely() {
        assert_eq!(parse_level(" DEBUG "), Some(Level::DEBUG));
        assert_eq!(parse_level("warn"), Some(Level::WARN));
        assert_eq!(parse_level("loud"), None);
        assert_eq!(parse_level(""), None);
    }

    #[test]
    fn verbosity_steps_up_to_trace() {
        assert_eq!(console_level(0), None);
        assert_eq!(console_level(1), Some(Level::INFO));
        assert_eq!(console_level(2), Some(Level::DEBUG));
        assert_eq!(console_level(9), Some(Level::TRACE));
    }

    #[test]
    fn our_crates_get_the_level_and_others_stay_quiet() {
        let text = default_directives(Level::DEBUG);
        assert!(text.starts_with("warn,"));
        assert!(text.contains("mul_tor_core=DEBUG"));
        assert!(EnvFilter::try_new(&text).is_ok());
    }
}

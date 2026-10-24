//! `mul-tor logs`: look at what the program has recorded.

use anyhow::{Context, Result};
use console::style;
use mul_tor_core::logging;

use crate::app::App;
use crate::args::LogsArgs;

pub fn logs(app: &App, args: LogsArgs) -> Result<()> {
    let dir = app.dirs.log_dir();
    if args.path {
        println!("{}", dir.display());
        return Ok(());
    }
    let Some(file) = logging::latest_log(&dir) else {
        eprintln!("No log files yet in {}", dir.display());
        return Ok(());
    };
    // The header goes to stderr so stdout stays pure log lines for pipes.
    eprintln!("{}", style(file.display()).dim());
    let lines =
        logging::tail(&file, args.lines).with_context(|| format!("reading {}", file.display()))?;
    for line in lines {
        println!("{}", colour(&line));
    }
    Ok(())
}

/// Colour the level word of a text line; JSON lines are left alone.
fn colour(line: &str) -> String {
    if line.starts_with('{') {
        return line.to_string();
    }
    for (word, paint) in [
        (" ERROR ", style(" ERROR ").red().bold()),
        (" WARN ", style(" WARN ").yellow().bold()),
        (" INFO ", style(" INFO ").green()),
        (" DEBUG ", style(" DEBUG ").blue()),
        (" TRACE ", style(" TRACE ").dim()),
    ] {
        // The level sits right after the timestamp, so only look near the start.
        if let Some(at) = line.get(..48).and_then(|head| head.find(word)) {
            return format!("{}{}{}", &line[..at], paint, &line[at + word.len()..]);
        }
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_lines_and_short_lines_pass_through() {
        assert_eq!(colour("{\"level\":\"INFO\"}"), "{\"level\":\"INFO\"}");
        assert_eq!(colour("short"), "short");
    }

    #[test]
    fn the_message_survives_colouring() {
        let line = "2026-10-24 13:05:41.123 +02:00  WARN mul_tor_core::engine: a: upload failed";
        let painted = colour(line);
        assert!(painted.contains("upload failed"), "{painted}");
        assert!(painted.starts_with("2026-10-24 13:05:41.123 +02:00"));
    }
}

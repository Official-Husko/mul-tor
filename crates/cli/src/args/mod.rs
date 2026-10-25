//! Command-line definition.

mod info;
#[cfg(test)]
mod tests;
mod upload;

use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

pub use info::{ConfigAction, ConfigArgs, LogsArgs, ProvidersArgs, ProvidersFormat};
pub use upload::UploadArgs;

const AFTER_HELP: &str = "\
Run without a command for the full-screen interface.

Examples:
  mul-tor                                        Full-screen interface
  mul-tor upload backup.7z -p pixeldrain -p gofile
  mul-tor upload ./release --preset f95 --parallel 3
  mul-tor upload photos/ -r --all --json         One JSON object per upload
  mul-tor providers                                  What is supported
";

#[derive(Parser, Debug)]
#[command(
    name = "mul-tor",
    version,
    about = "Upload files to many file-hosting providers at once",
    after_help = AFTER_HELP,
    propagate_version = true
)]
pub struct Cli {
    /// Keep config, presets and caches in this folder instead of the
    /// platform's usual locations (portable mode)
    #[arg(
        long,
        global = true,
        env = "MUL_TOR_HOME",
        hide_env_values = true,
        value_name = "DIR"
    )]
    pub home: Option<PathBuf>,

    /// Log more to the terminal (-v: info, -vv: debug)
    #[arg(short, long, global = true, action = ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Upload files or folders to one or more providers
    Upload(UploadArgs),
    /// List the supported providers
    Providers(ProvidersArgs),
    /// List the available presets
    Presets,
    /// Check which providers are reachable right now
    Check,
    /// Show where the config lives, or print it
    Config(ConfigArgs),
    /// Show the latest lines of the log
    Logs(LogsArgs),
}

//! Arguments of the commands that only show things.

use clap::{Args, Subcommand, ValueEnum};

#[derive(Args, Debug)]
pub struct ProvidersArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = ProvidersFormat::Table)]
    pub format: ProvidersFormat,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvidersFormat {
    Table,
    Markdown,
    Json,
}

#[derive(Args, Debug)]
pub struct LogsArgs {
    /// How many of the latest lines to show
    #[arg(short = 'n', long, default_value_t = 50, value_name = "LINES")]
    pub lines: usize,

    /// Only print the folder the log files are in
    #[arg(long)]
    pub path: bool,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Print the path of the config file
    Path,
    /// Print the effective settings (secrets hidden)
    Show,
}

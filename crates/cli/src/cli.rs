//! Command-line definition.

use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

const AFTER_HELP: &str = "\
Run without a command for the interactive mode.

Examples:
  mul-tor                                        Interactive mode
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

#[derive(Args, Debug)]
pub struct UploadArgs {
    /// Files or folders to upload (a folder contributes the files directly in it)
    #[arg(required = true, value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Provider to upload to; repeat or separate with commas. See `mul-tor providers`
    #[arg(short, long = "provider", value_name = "NAME", value_delimiter = ',')]
    pub providers: Vec<String>,

    /// Upload to every provider of a preset (e.g. f95). See `mul-tor presets`
    #[arg(short = 'P', long, value_name = "NAME")]
    pub preset: Option<String>,

    /// Upload to every provider that needs no missing credentials
    #[arg(short, long)]
    pub all: bool,

    /// Include files in sub-folders when a folder is given
    #[arg(short, long)]
    pub recursive: bool,

    /// How many uploads to run at once [default: from the config, 1]
    #[arg(short = 'j', long, value_name = "N")]
    pub parallel: Option<usize>,

    /// Do not check which providers are reachable first
    #[arg(long)]
    pub no_check: bool,

    /// Print one JSON object per finished upload on stdout; no progress bars
    #[arg(long)]
    pub json: bool,

    /// Route everything through this proxy (http://, socks5://, socks5h://)
    #[arg(long, value_name = "URL")]
    pub proxy: Option<String>,

    /// Write file_links.txt here [default: the current folder]
    #[arg(long, value_name = "DIR")]
    pub links_dir: Option<PathBuf>,

    /// Do not write file_links.txt
    #[arg(long)]
    pub no_save_links: bool,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn providers_can_be_repeated_or_comma_separated() {
        let cli = Cli::try_parse_from([
            "mul-tor",
            "upload",
            "a.zip",
            "-p",
            "gofile,oshi",
            "--provider",
            "pixeldrain",
        ])
        .unwrap();
        let Some(Command::Upload(args)) = cli.command else {
            panic!("not an upload command");
        };
        assert_eq!(args.providers, ["gofile", "oshi", "pixeldrain"]);
    }

    #[test]
    fn upload_needs_a_path() {
        assert!(Cli::try_parse_from(["mul-tor", "upload"]).is_err());
    }

    #[test]
    fn no_command_means_interactive() {
        assert!(Cli::try_parse_from(["mul-tor"]).unwrap().command.is_none());
    }
}

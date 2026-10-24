//! Command-line definition.

use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

const AFTER_HELP: &str = "\
Run without a command for the interactive mode.

Examples:
  mul-tor                                        Interactive mode
  mul-tor upload backup.7z -s pixeldrain -s gofile
  mul-tor upload ./release --preset f95 --parallel 3
  mul-tor upload photos/ -r --all --json         One JSON object per upload
  mul-tor sites                                  What is supported
";

#[derive(Parser, Debug)]
#[command(
    name = "mul-tor",
    version,
    about = "Upload files to many file-hosting sites at once",
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
    /// Upload files or folders to one or more sites
    Upload(UploadArgs),
    /// List the supported sites
    Sites(SitesArgs),
    /// List the available presets
    Presets,
    /// Check which sites are reachable right now
    Check,
    /// Show where the config lives, or print it
    Config(ConfigArgs),
}

#[derive(Args, Debug)]
pub struct UploadArgs {
    /// Files or folders to upload (a folder contributes the files directly in it)
    #[arg(required = true, value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Site to upload to; repeat or separate with commas. See `mul-tor sites`
    #[arg(short, long = "site", value_name = "NAME", value_delimiter = ',')]
    pub sites: Vec<String>,

    /// Upload to every site of a preset (e.g. f95). See `mul-tor presets`
    #[arg(short, long, value_name = "NAME")]
    pub preset: Option<String>,

    /// Upload to every site that needs no missing credentials
    #[arg(short, long)]
    pub all: bool,

    /// Include files in sub-folders when a folder is given
    #[arg(short, long)]
    pub recursive: bool,

    /// How many uploads to run at once [default: from the config, 1]
    #[arg(short = 'j', long, value_name = "N")]
    pub parallel: Option<usize>,

    /// Do not check which sites are reachable first
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
pub struct SitesArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = SitesFormat::Table)]
    pub format: SitesFormat,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SitesFormat {
    Table,
    Markdown,
    Json,
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
    fn sites_can_be_repeated_or_comma_separated() {
        let cli = Cli::try_parse_from([
            "mul-tor",
            "upload",
            "a.zip",
            "-s",
            "gofile,oshi",
            "--site",
            "pixeldrain",
        ])
        .unwrap();
        let Some(Command::Upload(args)) = cli.command else {
            panic!("not an upload command");
        };
        assert_eq!(args.sites, ["gofile", "oshi", "pixeldrain"]);
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

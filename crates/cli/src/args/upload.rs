//! Arguments of `mul-tor upload`.

use std::path::PathBuf;

use clap::Args;

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

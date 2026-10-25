//! `mul-tor upload`: send files to providers without any prompts.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use console::style;
use mul_tor_core::files::scan_inputs;

use crate::app::App;
use crate::args::UploadArgs;
use crate::runner::{self, Job};
use crate::selection::{Request, select};

pub async fn upload(app: App, args: UploadArgs) -> Result<ExitCode> {
    let scan = scan_inputs(&args.paths, args.recursive);
    for (path, problem) in &scan.problems {
        eprintln!("{} {}: {problem}", style("[!]").red(), path.display());
    }
    if scan.files.is_empty() {
        bail!("none of the given paths contains a file to upload");
    }

    let request = Request {
        names: &args.providers,
        preset: args.preset.as_deref(),
        all: args.all,
    };
    let Some(selection) = select(&app, &request)? else {
        bail!(
            "no providers chosen: use --provider NAME, --preset NAME or --all (see `mul-tor providers`)"
        );
    };
    for note in &selection.notes {
        eprintln!("{} {note}", style("[i]").cyan());
    }

    let factory = app.http_factory(args.proxy.as_deref(), args.json).await?;
    let providers = if app.config.check_availability && !args.no_check {
        let named = &args.providers;
        runner::filter_reachable(&factory, selection.providers, named, args.json).await
    } else {
        selection.providers
    };

    let job = Job {
        files: scan.files,
        providers,
        link_format: selection.link_format,
        json: args.json,
        parallel: args
            .parallel
            .unwrap_or_else(|| app.config.parallel())
            .clamp(1, 16),
        save_links: app.config.save_links && !args.no_save_links,
        links_dir: args
            .links_dir
            .or_else(|| app.config.links_dir.clone())
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    let code = runner::execute(&app, factory, job).await?;

    // A path that could not be read counts as a failure even if the rest went fine.
    Ok(if code == ExitCode::SUCCESS && !scan.problems.is_empty() {
        ExitCode::from(1)
    } else {
        code
    })
}

//! The guided mode (run `mul-tor` with no arguments): pick files, pick providers,
//! watch the uploads, optionally go again. Mirrors the flow of the Python tool.

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use mul_tor_core::files::scan_inputs;
use mul_tor_core::{HttpFactory, Provider, update};

use crate::app::App;
use crate::paths::parse_paths;
use crate::runner::{self, Job};
use crate::selection::{Request, select};

const LOGO: &str = r"
.88b  d88. db    db db             d888888b  .d88b.  d8888b.
88'YbdP`88 88    88 88             `~~88~~' .8P  Y8. 88  `8D
88  88  88 88    88 88                88    88    88 88oobY'
88  88  88 88    88 88      C8888D    88    88    88 88`8b
88  88  88 88b  d88 88booo.           88    `8b  d8' 88 `88.
YP  YP  YP ~Y8888P' Y88888P           YP     `Y88P'  88   YD";

pub async fn run(app: App) -> Result<ExitCode> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        bail!("interactive mode needs a terminal; use `mul-tor upload --help` for scripted use");
    }

    println!("{}", style(LOGO).red());
    println!(
        "{:>64}\n",
        format!(
            "v{} | by {}",
            style(env!("CARGO_PKG_VERSION")).cyan(),
            style("Official-Husko").yellow()
        )
    );

    let factory = app.http_factory(None, false).await?;

    if app.config.check_for_updates {
        announce_update(&factory).await;
    }

    let mut eligible = app.eligible_providers();
    if app.config.check_availability {
        eligible = runner::filter_reachable(&factory, eligible, &[], false).await;
        println!(
            "{} {} providers available\n",
            style("[i]").cyan(),
            style(eligible.len()).yellow()
        );
    }

    let theme = ColorfulTheme::default();
    let mut exit = ExitCode::SUCCESS;
    loop {
        let Some(files) = ask_files(&theme)? else {
            return Ok(ExitCode::from(130));
        };
        let Some(selection) = ask_providers(&app, &theme, &eligible)? else {
            return Ok(ExitCode::from(130));
        };

        let job = Job {
            files,
            providers: selection.providers,
            link_format: selection.link_format,
            json: false,
            parallel: app.config.parallel(),
            save_links: app.config.save_links,
            links_dir: app
                .config
                .links_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from(".")),
        };
        println!();
        let code = runner::execute(&app, factory.clone(), job).await?;
        if code != ExitCode::SUCCESS {
            exit = code;
        }

        println!();
        let again = Confirm::with_theme(&theme)
            .with_prompt("Upload something else?")
            .default(false)
            .interact_opt()?;
        if again != Some(true) {
            return Ok(exit);
        }
        println!();
    }
}

async fn announce_update(factory: &HttpFactory) {
    let Ok(http) = factory.client() else { return };
    let lookup = tokio::time::timeout(Duration::from_secs(8), update::latest_release(&http)).await;
    if let Ok(Ok(release)) = lookup
        && update::is_newer(env!("CARGO_PKG_VERSION"), &release)
    {
        println!(
            "{} {} {}\n",
            style("UPDATE AVAILABLE:").red().bold(),
            style(&release.name).cyan(),
            style(format!("({})", release.url)).dim()
        );
    }
}

/// `None` means the user pressed Ctrl-C / Esc.
fn ask_files(theme: &ColorfulTheme) -> Result<Option<Vec<PathBuf>>> {
    let kind = Select::with_theme(theme)
        .with_prompt("What do you want to upload?")
        .items(["A single file", "Several files or a whole folder"])
        .default(0)
        .interact_opt()?;
    let Some(kind) = kind else { return Ok(None) };
    let prompt = if kind == 0 {
        "File (drag it here or type the path)"
    } else {
        "Files or a folder (drag them here or type the paths)"
    };

    loop {
        let line: String = match Input::<String>::with_theme(theme)
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()
        {
            Ok(line) => line,
            Err(dialoguer::Error::IO(e)) if e.kind() == io::ErrorKind::Interrupted => {
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        let paths = parse_paths(&line);
        if paths.is_empty() {
            continue;
        }
        let scan = scan_inputs(&paths, false);
        for (path, problem) in &scan.problems {
            println!("{} {}: {problem}", style("[!]").red(), path.display());
        }
        if scan.files.is_empty() {
            println!("{} Nothing to upload there. Try again.", style("[!]").red());
            continue;
        }
        println!(
            "{} {} file{}",
            style("[i]").cyan(),
            scan.files.len(),
            if scan.files.len() == 1 { "" } else { "s" }
        );
        return Ok(Some(scan.files));
    }
}

struct Chosen {
    providers: Vec<Arc<dyn Provider>>,
    link_format: Option<String>,
}

fn ask_providers(
    app: &App,
    theme: &ColorfulTheme,
    eligible: &[Arc<dyn Provider>],
) -> Result<Option<Chosen>> {
    // A preset the config asks for, or one the user picks, pre-ticks its providers.
    let mut preticked: Vec<Arc<dyn Provider>> = Vec::new();
    let mut link_format = None;

    let preset_request =
        if app.config.preset.enable_selection && app.presets.iter().next().is_some() {
            let mut items = vec!["None (choose providers by hand)".to_string()];
            let stems: Vec<&str> = app.presets.iter().map(|(stem, _)| stem).collect();
            items.extend(stems.iter().map(|s| s.to_string()));
            let Some(choice) = Select::with_theme(theme)
                .with_prompt("Use a preset?")
                .items(&items)
                .default(0)
                .interact_opt()?
            else {
                return Ok(None);
            };
            (choice > 0).then(|| stems[choice - 1].to_string())
        } else {
            None
        };

    let request = Request {
        preset: preset_request.as_deref(),
        ..Request::default()
    };
    if let Some(selection) = select(app, &request).ok().flatten() {
        for note in &selection.notes {
            println!("{} {note}", style("[i]").cyan());
        }
        preticked = selection.providers;
        link_format = selection.link_format;
    }

    // Providers needing an account that is not set up cannot work; say so instead
    // of offering them.
    let (usable, unconfigured): (Vec<_>, Vec<_>) = eligible
        .iter()
        .cloned()
        .partition(|provider| !app.lacks_credentials(provider.info()));
    if !unconfigured.is_empty() {
        println!(
            "{} Not offered, no credentials in {}: {}",
            style("[i]").cyan(),
            app.store.path().display(),
            unconfigured
                .iter()
                .map(|s| s.info().name)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if usable.is_empty() {
        bail!("no providers are available; check your config, blacklist and connection");
    }

    let labels: Vec<String> = usable
        .iter()
        .map(|provider| {
            let info = provider.info();
            let limit = info
                .max_size
                .map_or_else(|| "no limit".to_string(), |l| l.to_string());
            format!("{:<14} {}", info.name, style(limit).dim())
        })
        .collect();
    let defaults: Vec<bool> = usable
        .iter()
        .map(|provider| preticked.iter().any(|p| Arc::ptr_eq(p, provider)))
        .collect();

    loop {
        let picked = MultiSelect::with_theme(theme)
            .with_prompt(format!(
                "What providers do you want to upload to? {}",
                style(format!(
                    "({} available, space selects, enter confirms)",
                    usable.len()
                ))
                .dim()
            ))
            .items(&labels)
            .defaults(&defaults)
            .max_length(14)
            .interact_opt()?;
        let Some(picked) = picked else {
            return Ok(None);
        };
        if picked.is_empty() {
            println!("{} Pick at least one provider.", style("[!]").red());
            continue;
        }
        return Ok(Some(Chosen {
            providers: picked.into_iter().map(|i| usable[i].clone()).collect(),
            link_format,
        }));
    }
}

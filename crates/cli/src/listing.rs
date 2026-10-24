//! The read-only commands: `sites`, `presets`, `check`, `config`.

use anyhow::{Context as _, Result};
use console::style;
use mul_tor_core::availability::{self, Health};
use mul_tor_core::{Auth, SiteInfo};
use serde_json::json;

use crate::app::App;
use crate::cli::{ConfigAction, SitesFormat};

pub fn sites(app: &App, format: SitesFormat) -> Result<()> {
    let infos: Vec<&SiteInfo> = app.registry.iter().map(|s| s.info()).collect();
    match format {
        SitesFormat::Table => print!("{}", sites_table(&infos)),
        SitesFormat::Markdown => print!("{}", sites_markdown(&infos)),
        SitesFormat::Json => println!("{}", serde_json::to_string_pretty(&sites_json(&infos))?),
    }
    Ok(())
}

fn size_label(info: &SiteInfo) -> String {
    info.max_size
        .map_or_else(|| "—".to_string(), |limit| limit.to_string())
}

fn account_label(info: &SiteInfo) -> &'static str {
    match info.auth {
        Auth::None => "no",
        Auth::Optional => "optional",
        Auth::Required => "required",
    }
}

pub fn sites_table(infos: &[&SiteInfo]) -> String {
    let name_width = infos
        .iter()
        .map(|i| i.name.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let row = |name: &str, size: &str, account: &str, notes: &str| {
        format!("{name:<name_width$}  {size:>8}  {account:<8}  {notes}")
            .trim_end()
            .to_string()
            + "\n"
    };
    let mut out = row("Site", "Max size", "Account", "Notes");
    for info in infos {
        out.push_str(&row(
            info.name,
            &size_label(info),
            account_label(info),
            info.known_issue.unwrap_or(""),
        ));
    }
    out
}

/// The table in the README's format; regenerate it with `mul-tor sites --format markdown`.
pub fn sites_markdown(infos: &[&SiteInfo]) -> String {
    let mut out = String::from("Site | API | Account | Max file size\n--- | --- | --- | ---\n");
    for info in infos {
        let api = info
            .api_docs
            .map_or_else(|| "No".to_string(), |url| format!("[Yes]({url})"));
        let account = match (info.auth, info.credentials_url) {
            (Auth::None, _) => "No".to_string(),
            (Auth::Optional, _) => "Optional".to_string(),
            (Auth::Required, Some(url)) => format!("[Yes]({url})"),
            (Auth::Required, None) => "Yes".to_string(),
        };
        let size = info
            .max_size
            .map_or_else(|| "∞".to_string(), |l| l.to_string());
        out.push_str(&format!(
            "[{}]({}) | {api} | {account} | {size}\n",
            info.name, info.homepage
        ));
    }
    out
}

pub fn sites_json(infos: &[&SiteInfo]) -> serde_json::Value {
    infos
        .iter()
        .map(|info| {
            json!({
                "name": info.name,
                "aliases": info.aliases,
                "homepage": info.homepage,
                "max_size_bytes": info.max_size.map(|l| l.bytes()),
                "account": account_label(info),
                "credentials": info.credentials.iter().map(|f| f.key).collect::<Vec<_>>(),
                "known_issue": info.known_issue,
            })
        })
        .collect()
}

pub fn presets(app: &App) -> Result<()> {
    for (stem, preset) in app.presets.iter() {
        let resolved = preset.resolve(&app.registry);
        let title = if preset.name.is_empty() {
            stem
        } else {
            &preset.name
        };
        println!(
            "{}  {}",
            style(stem).bold(),
            style(format!("({title}, {} sites)", resolved.sites.len())).dim()
        );
        if !preset.description.is_empty() {
            println!("  {}", preset.description);
        }
        if !resolved.unsupported.is_empty() {
            println!(
                "  {} not supported yet: {}",
                style("note:").yellow(),
                resolved.unsupported.join(", ")
            );
        }
    }
    println!(
        "\nYour own presets go in {}",
        app.dirs.presets_dir().display()
    );
    Ok(())
}

pub async fn check(app: &App) -> Result<()> {
    let sites = app.eligible_sites();
    let factory = app.http_factory(None, true).await?;
    let statuses = availability::check(&factory, &sites).await;

    let name_width = sites
        .iter()
        .map(|s| s.info().name.chars().count())
        .max()
        .unwrap_or(4);
    let mut down = 0;
    for status in &statuses {
        let name = status.site.info().name;
        match &status.health {
            Health::Up => println!("{} {name}", style("up  ").green()),
            Health::Down(reason) => {
                down += 1;
                println!(
                    "{} {name:<name_width$}  {}",
                    style("down").red(),
                    style(reason).dim()
                );
            }
        }
    }
    println!("\n{} reachable, {down} not", statuses.len() - down);
    Ok(())
}

pub fn config(app: &App, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Path => println!("{}", app.store.path().display()),
        ConfigAction::Show => {
            let mut config = app.config.clone();
            for credentials in config.credentials.values_mut() {
                let masked: Vec<(String, String)> = credentials
                    .iter()
                    .map(|(key, value)| {
                        let shown = if value.trim().is_empty() {
                            ""
                        } else {
                            "********"
                        };
                        (key.to_string(), shown.to_string())
                    })
                    .collect();
                *credentials = masked.into_iter().collect();
            }
            print!(
                "{}",
                toml::to_string_pretty(&config).context("could not print the config")?
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mul_tor_core::Registry;

    fn all_infos(registry: &Registry) -> Vec<&SiteInfo> {
        registry.iter().map(|s| s.info()).collect()
    }

    #[test]
    fn the_table_lists_every_site() {
        let registry = Registry::builtin();
        let table = sites_table(&all_infos(&registry));
        assert_eq!(table.lines().count(), 38); // header + 37 sites
        assert!(
            table.contains("Pixeldrain") && table.contains("20 GB") && table.contains("required")
        );
    }

    #[test]
    fn markdown_rows_have_four_columns() {
        let registry = Registry::builtin();
        let md = sites_markdown(&all_infos(&registry));
        for line in md.lines() {
            assert_eq!(line.matches(" | ").count(), 3, "{line}");
        }
        assert!(
            md.contains("[Gofile](https://gofile.io/) | [Yes](https://gofile.io/api) | No | ∞")
        );
        assert!(md.contains("[Pixeldrain](https://pixeldrain.com/) | [Yes](https://pixeldrain.com/api) | [Yes](https://pixeldrain.com/user/api_keys) | 20 GB"));
    }

    #[test]
    fn json_describes_each_site() {
        let registry = Registry::builtin();
        let json = sites_json(&all_infos(&registry));
        let list = json.as_array().unwrap();
        assert_eq!(list.len(), 37);
        let pixeldrain = list.iter().find(|s| s["name"] == "Pixeldrain").unwrap();
        assert_eq!(pixeldrain["account"], "required");
        assert_eq!(pixeldrain["credentials"][0], "apiKey");
        assert_eq!(pixeldrain["max_size_bytes"], 20u64 * 1024 * 1024 * 1024);
    }

    /// The README's site table is generated from the registry; fail loudly when
    /// a site is added without regenerating it.
    #[test]
    fn the_readme_lists_every_supported_site() {
        let readme = include_str!("../../../README.md");
        for site in Registry::builtin().iter() {
            let info = site.info();
            assert!(
                readme.contains(&format!("[{}]({})", info.name, info.homepage)),
                "README.md is missing {}; regenerate with `mul-tor sites --format markdown`",
                info.name
            );
        }
    }
}

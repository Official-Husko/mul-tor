//! `mul-tor providers`: what is supported, as a table, markdown or JSON.

use anyhow::Result;
use mul_tor_core::{Auth, ProviderInfo};
use serde_json::json;

use crate::app::App;
use crate::args::ProvidersFormat;

pub fn providers(app: &App, format: ProvidersFormat) -> Result<()> {
    let infos: Vec<&ProviderInfo> = app.registry.iter().map(|s| s.info()).collect();
    match format {
        ProvidersFormat::Table => print!("{}", providers_table(&infos)),
        ProvidersFormat::Markdown => print!("{}", providers_markdown(&infos)),
        ProvidersFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&providers_json(&infos))?)
        }
    }
    Ok(())
}

fn size_label(info: &ProviderInfo) -> String {
    info.max_size
        .map_or_else(|| "—".to_string(), |limit| limit.to_string())
}

fn account_label(info: &ProviderInfo) -> &'static str {
    match info.auth {
        Auth::None => "no",
        Auth::Optional => "optional",
        Auth::Required => "required",
    }
}

pub fn providers_table(infos: &[&ProviderInfo]) -> String {
    let name_width = infos
        .iter()
        .map(|i| i.name.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let row = |name: &str, size: &str, account: &str, status: &str, notes: &str| {
        format!("{name:<name_width$}  {size:>8}  {account:<8}  {status:<10}  {notes}")
            .trim_end()
            .to_string()
            + "\n"
    };
    let mut out = row("Provider", "Max size", "Account", "Status", "Notes");
    for info in infos {
        out.push_str(&row(
            info.name,
            &size_label(info),
            account_label(info),
            info.reliability.label(),
            info.reliability.note().unwrap_or(""),
        ));
    }
    out
}

/// The table in the README's format; regenerate it with `mul-tor providers --format markdown`.
pub fn providers_markdown(infos: &[&ProviderInfo]) -> String {
    let mut out = String::from("Provider | API | Account | Max file size\n--- | --- | --- | ---\n");
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

pub fn providers_json(infos: &[&ProviderInfo]) -> serde_json::Value {
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
                "status": info.reliability.label(),
                "status_note": info.reliability.note(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mul_tor_core::Registry;

    fn all_infos(registry: &Registry) -> Vec<&ProviderInfo> {
        registry.iter().map(|s| s.info()).collect()
    }

    #[test]
    fn the_table_lists_every_provider() {
        let registry = Registry::builtin();
        let table = providers_table(&all_infos(&registry));
        assert_eq!(table.lines().count(), registry.len() + 1); // header + one row each
        assert!(
            table.contains("Pixeldrain") && table.contains("20 GB") && table.contains("required")
        );
    }

    #[test]
    fn markdown_rows_have_four_columns() {
        let registry = Registry::builtin();
        let md = providers_markdown(&all_infos(&registry));
        for line in md.lines() {
            assert_eq!(line.matches(" | ").count(), 3, "{line}");
        }
        assert!(
            md.contains("[Gofile](https://gofile.io/) | [Yes](https://gofile.io/api) | No | ∞")
        );
        assert!(md.contains("[Pixeldrain](https://pixeldrain.com/) | [Yes](https://pixeldrain.com/api) | [Yes](https://pixeldrain.com/user/api_keys) | 20 GB"));
    }

    #[test]
    fn json_describes_each_provider() {
        let registry = Registry::builtin();
        let json = providers_json(&all_infos(&registry));
        let list = json.as_array().unwrap();
        assert_eq!(list.len(), registry.len());
        let pixeldrain = list.iter().find(|s| s["name"] == "Pixeldrain").unwrap();
        assert_eq!(pixeldrain["account"], "required");
        assert_eq!(pixeldrain["credentials"][0], "apiKey");
        assert_eq!(pixeldrain["max_size_bytes"], 20u64 * 1024 * 1024 * 1024);
    }

    /// The README's provider table is generated from the registry; fail loudly when
    /// a provider is added without regenerating it.
    #[test]
    fn the_readme_lists_every_supported_provider() {
        let readme = include_str!("../../../../README.md");
        for provider in Registry::builtin().iter() {
            let info = provider.info();
            assert!(
                readme.contains(&format!("[{}]({})", info.name, info.homepage)),
                "README.md is missing {}; regenerate with `mul-tor providers --format markdown`",
                info.name
            );
        }
    }
}

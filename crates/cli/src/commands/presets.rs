//! `mul-tor presets`: the lists of providers a community accepts.

use anyhow::Result;
use console::style;

use crate::app::App;

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
            style(format!("({title}, {} providers)", resolved.providers.len())).dim()
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

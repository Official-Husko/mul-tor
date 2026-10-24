//! `mul-tor check`: which providers answer right now.

use anyhow::Result;
use console::style;
use mul_tor_core::availability::{self, Health};

use crate::app::App;

pub async fn check(app: &App) -> Result<()> {
    let providers = app.eligible_providers();
    let factory = app.http_factory(None, true).await?;
    let statuses = availability::check(&factory, &providers).await;

    let name_width = providers
        .iter()
        .map(|s| s.info().name.chars().count())
        .max()
        .unwrap_or(4);
    let mut down = 0;
    for status in &statuses {
        let name = status.provider.info().name;
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

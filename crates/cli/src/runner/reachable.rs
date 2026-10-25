//! Dropping providers that do not answer before an upload starts.

use std::sync::Arc;

use console::style;
use mul_tor_core::availability;
use mul_tor_core::{HttpFactory, Provider};

/// Drop providers that do not answer. `strict` providers (picked by name) are only
/// warned about, since the probe can be wrong about a provider the user asked for.
pub async fn filter_reachable(
    factory: &HttpFactory,
    providers: Vec<Arc<dyn Provider>>,
    named: &[String],
    quiet: bool,
) -> Vec<Arc<dyn Provider>> {
    if !quiet {
        eprint!(
            "{}",
            style(format!("Checking {} providers...", providers.len())).yellow()
        );
    }
    let statuses = availability::check(factory, &providers).await;
    if !quiet {
        eprint!("\r{}\r", " ".repeat(40));
    }

    let mut kept = Vec::new();
    for status in statuses {
        let info = status.provider.info();
        match &status.health {
            availability::Health::Up => kept.push(status.provider),
            availability::Health::Down(reason) => {
                let asked_for_it = named.iter().any(|name| info.matches(name));
                eprintln!(
                    "{} {} did not answer ({reason}){}",
                    style("[!]").yellow(),
                    info.name,
                    if asked_for_it {
                        "; trying anyway"
                    } else {
                        "; skipping it"
                    }
                );
                if asked_for_it {
                    kept.push(status.provider);
                }
            }
        }
    }
    kept
}

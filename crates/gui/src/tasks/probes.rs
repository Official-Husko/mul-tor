//! Reachability and update checks.

use std::sync::Arc;

use mul_tor_core::availability;
use mul_tor_core::update;
use mul_tor_core::{AppDirs, Config, HttpFactory, Provider, setup};
use tokio::runtime::Handle;

use super::{Bus, Msg};

/// Probe `providers` and report which answer.
pub fn check_availability(
    rt: &Handle,
    bus: &Bus,
    config: Config,
    dirs: AppDirs,
    providers: Vec<(String, Arc<dyn Provider>)>,
) {
    let bus = bus.clone();
    rt.spawn(async move {
        let setup = setup::http_factory(&config, &dirs, None).await;
        let just_providers: Vec<_> = providers
            .iter()
            .map(|(_, provider)| provider.clone())
            .collect();
        let statuses = availability::check(&setup.factory, &just_providers).await;
        let results = providers
            .iter()
            .zip(statuses)
            .map(|((id, _), status)| (id.clone(), status.health))
            .collect();
        bus.send(Msg::Availability(results));
    });
}

pub fn check_for_update(rt: &Handle, bus: &Bus) {
    let bus = bus.clone();
    rt.spawn(async move {
        let Ok(http) = HttpFactory::new().client() else {
            return;
        };
        match update::latest_release(&http).await {
            Ok(release) if update::is_newer(env!("CARGO_PKG_VERSION"), &release) => {
                bus.send(Msg::UpdateAvailable(release));
            }
            Ok(_) => {}
            // Not worth bothering the user: an offline start is normal.
            Err(error) => tracing::info!(%error, "update check failed"),
        }
    });
}

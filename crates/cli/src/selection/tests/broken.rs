//! Providers recorded as broken or degraded.

use std::sync::Arc;

use async_trait::async_trait;
use mul_tor_core::{Provider, ProviderInfo, Registry, UploadContext, UploadError, UploadedFile};

use super::*;
use crate::app::test_app_with;

struct Stub(ProviderInfo);

#[async_trait]
impl Provider for Stub {
    fn info(&self) -> &ProviderInfo {
        &self.0
    }

    async fn upload(&self, _: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        unreachable!("selection never uploads")
    }
}

fn app() -> App {
    let stub = |info| -> Arc<dyn Provider> { Arc::new(Stub(info)) };
    test_app_with(
        Config::default(),
        Registry::new(vec![
            stub(ProviderInfo::new("Fine", "https://f", "https://f")),
            stub(ProviderInfo::new("Shaky", "https://s", "https://s").degraded("slow")),
            stub(ProviderInfo::new("Dead", "https://d", "https://d").broken("shut down")),
        ]),
    )
}

#[test]
fn all_skips_broken_providers_and_says_so() {
    let selection = select(
        &app(),
        &Request {
            all: true,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(names(&selection), ["Fine", "Shaky"]);
    assert!(
        selection
            .notes
            .iter()
            .any(|n| n.contains("Dead") && n.contains("shut down")),
        "{:?}",
        selection.notes
    );
}

#[test]
fn naming_a_broken_provider_still_tries_it_with_a_warning() {
    let wanted = strings(&["dead"]);
    let selection = select(
        &app(),
        &Request {
            names: &wanted,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(names(&selection), ["Dead"]);
    assert!(selection.notes[0].contains("trying it anyway"));
}

#[test]
fn a_degraded_provider_is_not_held_back() {
    let wanted = strings(&["shaky"]);
    let selection = select(
        &app(),
        &Request {
            names: &wanted,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(names(&selection), ["Shaky"]);
    assert!(selection.notes.is_empty());
}

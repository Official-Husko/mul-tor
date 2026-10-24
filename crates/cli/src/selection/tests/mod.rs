//! Which providers a request ends up with.

mod broken;
mod plain;
mod presets;

use super::*;
use crate::app::test_app;
use mul_tor_core::Config;

fn names(selection: &Selection) -> Vec<&'static str> {
    selection.providers.iter().map(|s| s.info().name).collect()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

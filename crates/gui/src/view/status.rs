//! The small labelled cells of a provider row.

use egui::{RichText, Ui};
use mul_tor_core::availability::Health;
use mul_tor_core::{Auth, ProviderInfo, Reliability};

use crate::theme;

/// Whether an account is needed, and whether it is set up.
pub fn account_cell(ui: &mut Ui, info: &ProviderInfo, lacks: bool) {
    match (info.auth, lacks) {
        (Auth::None, _) => ui.label(""),
        (_, true) => ui
            .label(RichText::new("needs account").color(theme::WARN))
            .on_hover_text("Add your credentials in Settings, on the Accounts tab"),
        (_, false) => ui.label(RichText::new("account set").color(theme::OK)),
    };
}

/// Whether the last reachability check got an answer.
pub fn health_cell(ui: &mut Ui, health: Option<&Health>) {
    match health {
        Some(Health::Up) => ui.label(RichText::new("online").color(theme::OK)),
        Some(Health::Down(reason)) => ui
            .label(RichText::new("offline").color(theme::ERROR))
            .on_hover_text(reason),
        None => ui.label(""),
    };
}

/// What is recorded about whether the provider works.
pub fn reliability_cell(ui: &mut Ui, reliability: Reliability) {
    match reliability {
        Reliability::Unverified => ui
            .label(RichText::new("unverified").weak())
            .on_hover_text("Nobody has tried this against the live service yet"),
        Reliability::Working { checked } => ui
            .label(RichText::new("working").color(theme::OK))
            .on_hover_text(format!("Seen working on {checked}")),
        Reliability::Degraded(reason) => ui
            .label(RichText::new("degraded").color(theme::WARN))
            .on_hover_text(reason),
        Reliability::Broken(reason) => ui
            .label(RichText::new("broken").color(theme::ERROR))
            .on_hover_text(format!(
                "{reason} (it is left out of Select all and presets)"
            )),
    };
}

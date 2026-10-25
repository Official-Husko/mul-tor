//! The log window.

use egui_kittest::kittest::Queryable;
use tracing_subscriber::prelude::*;

use super::*;
use crate::app::{Action, LogDetail};

fn record_something(app: &MulTorApp) {
    let subscriber = tracing_subscriber::registry().with(app.logs.layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::warn!(provider = "Oshi", "gave up on the reply");
        tracing::info!("routine news");
    });
}

#[test]
fn the_log_button_opens_a_window_with_the_recent_records() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    assert!(harness.query_by_label("Show:").is_none());

    record_something(harness.state());
    harness.get_by_label("Log").click();
    harness.run();

    assert!(harness.state().log_window.open);
    harness.get_by_label("Show:");
    harness.get_by_label_contains("gave up on the reply");
    harness.get_by_label_contains("provider=Oshi");
    harness.get_by_label_contains("routine news");
}

#[test]
fn the_detail_setting_hides_less_severe_records() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    record_something(harness.state());
    harness.state_mut().log_window.open = true;
    harness.state_mut().log_window.detail = LogDetail::Errors;
    harness.run();
    assert!(
        harness
            .query_by_label_contains("gave up on the reply")
            .is_none()
    );

    harness.state_mut().log_window.detail = LogDetail::Warnings;
    harness.run();
    harness.get_by_label_contains("gave up on the reply");
    assert!(harness.query_by_label_contains("routine news").is_none());
}

#[test]
fn closing_the_window_and_toggling_the_button_agree() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    let ctx = harness.ctx.clone();
    harness.state_mut().perform(Action::ToggleLogs, &ctx);
    harness.run();
    assert!(harness.state().log_window.open);
    harness.state_mut().perform(Action::ToggleLogs, &ctx);
    harness.run();
    assert!(!harness.state().log_window.open);
    assert!(harness.query_by_label("Show:").is_none());
}

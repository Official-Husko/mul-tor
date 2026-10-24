//! What the window shows when it opens.

use egui_kittest::kittest::Queryable;

use super::*;
use crate::app::Action;

#[test]
fn the_main_sections_are_there() {
    let home = quiet_home();
    let harness = harness(home.path());

    for label in [
        "Mul-Tor",
        "Files",
        "Providers",
        "Add files…",
        "Add folder…",
        "Settings",
        "Catbox",
        "Gofile",
    ] {
        harness.get_by_label(label);
    }
    harness.get_by_label_contains("Upload 0 files to 0 providers");
    // All 37 providers are listed (none blacklisted by default).
    assert_eq!(harness.state().rows.len(), 37);
}

#[test]
fn an_invalid_action_target_does_not_crash() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    let ctx = harness.ctx.clone();
    harness.state_mut().perform(Action::RemoveFile(42), &ctx);
    harness
        .state_mut()
        .perform(Action::ApplyPreset(Some("no-such-preset".into())), &ctx);
    harness.state_mut().perform(Action::Cancel, &ctx);
    harness.run();
    assert!(harness.state().current_notice().is_some());
}

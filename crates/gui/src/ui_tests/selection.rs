//! Choosing files and providers.

use egui_kittest::kittest::Queryable;

use super::*;

#[test]
fn providers_that_need_an_account_cannot_be_ticked_until_configured() {
    let home = quiet_home();
    let harness = harness(home.path());

    let needing: Vec<_> = harness.query_all_by_label("needs account").collect();
    assert_eq!(
        needing.len(),
        5,
        "BowFile, DooDrive, Mixdrop, Pixeldrain, Rapidgator"
    );
    assert!(is_disabled(harness.get_by_label("Pixeldrain")));
    assert!(!is_disabled(harness.get_by_label("Catbox")));
}

#[test]
fn ticking_a_provider_and_adding_a_file_enables_the_upload_button() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    let file = sample_file(home.path(), "a.txt");

    harness.state_mut().add_paths(&[file]);
    harness.run();
    harness.get_by_label_contains("a.txt");
    assert!(is_disabled(
        harness.get_by_label_contains("Upload 1 file to 0 providers")
    ));

    harness.get_by_label("Catbox").click();
    harness.run();
    assert!(harness.state().model.selected.contains("catbox"));
    assert!(!is_disabled(
        harness.get_by_label_contains("Upload 1 file to 1 provider")
    ));

    harness.get_by_label("Catbox").click(); // untick again
    harness.run();
    assert!(harness.state().model.selected.is_empty());
}

#[test]
fn the_search_box_filters_the_provider_list() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    harness.state_mut().model.provider_filter = "cat".into();
    harness.run();
    assert!(harness.query_by_label("Catbox").is_some());
    assert!(harness.query_by_label("Gofile").is_none());
}

#[test]
fn a_preset_ticks_its_usable_providers_and_explains_what_it_skipped() {
    let home = quiet_home();
    let mut harness = harness(home.path());

    harness.get_by_label("Preset:").click();
    harness.run();
    harness.get_by_label("F95").click();
    harness.run();

    let model = &harness.state().model;
    assert_eq!(model.preset.as_deref(), Some("f95"));
    assert!(model.selected.contains("catbox") && model.selected.contains("gofile"));
    assert!(
        !model.selected.contains("pixeldrain"),
        "no credentials, so not ticked"
    );
    let notice = harness
        .state()
        .current_notice()
        .expect("a note about the skipped providers");
    assert!(notice.text.contains("Pixeldrain"), "{}", notice.text);
}

#[derive(Debug)]
struct Dropped(PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

#[test]
fn files_dropped_on_the_window_are_added() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    let file = sample_file(home.path(), "dropped.txt");

    harness
        .input_mut()
        .dropped_files
        .push(Arc::new(Dropped(file)));
    harness.run();

    assert_eq!(harness.state().model.files.len(), 1);
    harness.get_by_label_contains("dropped.txt");
}

#[test]
fn every_provider_shows_what_is_recorded_about_whether_it_works() {
    let home = quiet_home();
    let harness = harness(home.path());

    // Four are marked degraded for what 1.x reported; nothing has been seen working.
    assert_eq!(harness.query_all_by_label("degraded").count(), 4);
    assert_eq!(harness.query_all_by_label("unverified").count(), 33);
    assert!(harness.query_by_label("working").is_none());
    assert!(harness.query_by_label("broken").is_none());
}

#[test]
fn the_all_button_ticks_only_what_can_work_right_now() {
    let home = quiet_home();
    let mut harness = harness(home.path());

    harness.get_by_label("All").click();
    harness.run();

    let usable = mul_tor_core::Registry::builtin()
        .iter()
        .filter(|s| s.info().auth != mul_tor_core::Auth::Required)
        .count();
    let selected = &harness.state().model.selected;
    assert_eq!(
        selected.len(),
        usable,
        "every provider that does not need an account"
    );
    assert!(selected.contains("catbox") && !selected.contains("pixeldrain"));
    harness.get_by_label_contains(&format!("{usable} selected"));
}

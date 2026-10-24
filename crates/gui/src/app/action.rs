//! What the user can ask for, and what each request does.

use super::MulTorApp;
use crate::tasks::{self, Level};

/// One user intent, produced by the views and applied by [`MulTorApp::perform`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    PickFiles,
    PickFolder,
    ClearFiles,
    RemoveFile(usize),
    ToggleProvider { id: String, on: bool },
    SetFilter(String),
    SelectAllProviders,
    SelectNoProviders,
    ApplyPreset(Option<String>),
    CheckAvailability,
    Start,
    Cancel,
    CopyLinks { formatted: bool },
    CopyText(String),
    OpenUrl(String),
    OpenSettings,
    SaveSettings,
    CloseSettings,
}

impl MulTorApp {
    /// Carry out what the user asked for.
    pub fn perform(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::PickFiles => tasks::pick(self.rt.handle(), &self.bus, false),
            Action::PickFolder => tasks::pick(self.rt.handle(), &self.bus, true),
            Action::ClearFiles => self.model.files.clear(),
            Action::RemoveFile(index) => self.model.remove_file(index),
            Action::ToggleProvider { id, on } => self.model.toggle_provider(&id, on),
            Action::SetFilter(filter) => self.model.provider_filter = filter,
            Action::SelectAllProviders => {
                let ids: Vec<String> = self
                    .rows
                    .iter()
                    .filter(|r| !self.lacks_credentials(&r.provider))
                    .filter(|r| r.provider.info().reliability.included_in_bulk_choices())
                    .filter(|r| {
                        crate::model::matches_filter(&*r.provider, &self.model.provider_filter)
                    })
                    .map(|r| r.id.clone())
                    .collect();
                for id in ids {
                    self.model.toggle_provider(&id, true);
                }
            }
            Action::SelectNoProviders => self.model.selected.clear(),
            Action::ApplyPreset(stem) => self.apply_preset(stem),
            Action::CheckAvailability => self.check_availability(),
            Action::Start => self.start(),
            Action::Cancel => {
                if let Some(cancel) = &self.cancel {
                    cancel.cancel();
                }
                self.model.request_cancel();
            }
            Action::CopyLinks { formatted } => {
                ctx.copy_text(self.model.links_text(formatted));
                self.show_notice(Level::Info, "Links copied to the clipboard");
            }
            Action::CopyText(text) => ctx.copy_text(text),
            Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            Action::OpenSettings => self.open_settings(),
            Action::SaveSettings => self.save_settings(),
            Action::CloseSettings => self.settings.open = false,
        }
    }
}

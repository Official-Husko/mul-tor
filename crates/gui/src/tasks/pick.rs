//! The system file picker.

use std::path::PathBuf;

use tokio::runtime::Handle;

use super::{Bus, Msg};

/// Show the system file (or folder) picker and report what was chosen.
pub fn pick(rt: &Handle, bus: &Bus, folder: bool) {
    let bus = bus.clone();
    rt.spawn(async move {
        let paths: Vec<PathBuf> = if folder {
            rfd::AsyncFileDialog::new()
                .set_title("Add a folder")
                .pick_folder()
                .await
                .into_iter()
                .map(|handle| handle.path().to_path_buf())
                .collect()
        } else {
            rfd::AsyncFileDialog::new()
                .set_title("Add files")
                .pick_files()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|handle| handle.path().to_path_buf())
                .collect()
        };
        if !paths.is_empty() {
            bus.send(Msg::FilesPicked(paths));
        }
    });
}

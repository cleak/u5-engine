//! Native dialogs (folder picker, error box) through `rfd`.
//!
//! Behind the `native-dialogs` feature so terminal-only builds do not link
//! it. On Linux `rfd` uses the XDG desktop portal over D-Bus (falling back to
//! `zenity`), so no GTK development packages are needed to build.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// Show a native "choose folder" dialog and wait for it.
pub fn pick_folder_blocking(title: &str, start: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    if let Some(start) = start.filter(|start| start.is_dir()) {
        dialog = dialog.set_directory(start);
    }
    dialog.pick_folder()
}

/// Start a folder dialog without blocking the caller's event loop.
///
/// The answer arrives on the returned channel (`None` when cancelled). On
/// macOS dialogs must run on the main thread, so there it runs inline and
/// the channel is already filled when this returns; elsewhere it runs on a
/// helper thread.
pub fn spawn_folder_picker(title: &str, start: Option<PathBuf>) -> Receiver<Option<PathBuf>> {
    let (sender, receiver) = channel();
    let title = title.to_string();
    if cfg!(target_os = "macos") {
        let _ = sender.send(pick_folder_blocking(&title, start.as_deref()));
    } else {
        std::thread::spawn(move || {
            let _ = sender.send(pick_folder_blocking(&title, start.as_deref()));
        });
    }
    receiver
}

/// A native error message box (blocks until dismissed).
pub fn show_error_dialog(title: &str, message: &str) {
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;

use crate::Lockdown;

// Runs once at startup, so little work is lost by the restart.
pub fn check(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Ok(updater) = app.updater() else { return };
        let Ok(Some(update)) = updater.check().await else { return };
        if app.state::<Lockdown>().0.load(Ordering::SeqCst) {
            return;
        }
        let handle = app.clone();
        app.dialog()
            .message(format!("Locus {} is available. Install it and restart now?", update.version))
            .title("Update available")
            .buttons(MessageDialogButtons::OkCancelCustom("Install".into(), "Later".into()))
            .show(move |install| {
                if !install {
                    return;
                }
                tauri::async_runtime::spawn(async move {
                    if update.download_and_install(|_, _| {}, || {}).await.is_ok() {
                        handle.restart();
                    }
                });
            });
    });
}

use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{browser, session, Lockdown};

// The update found at startup, and whether a window has offered it yet.
#[derive(Default)]
pub struct Pending(Mutex<Option<(Update, bool)>>);

// Runs once at startup, so little work is lost by the restart.
pub fn check(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Ok(updater) = app.updater() else { return };
        let Ok(Some(update)) = updater.check().await else { return };
        *app.state::<Pending>().0.lock().unwrap() = Some((update, false));
        if let Some(window) = app.get_webview_window(&session::active(&app)) {
            let _ = window.eval("window.locusUpdate && window.locusUpdate()");
        }
    });
}

// The update to offer, once, and not during an exam.
#[tauri::command]
pub fn update_pending(app: AppHandle, webview: Webview) -> Result<Option<Value>, String> {
    browser::trusted(&webview)?;
    if app.state::<Lockdown>().0.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let state = app.state::<Pending>();
    let mut pending = state.0.lock().unwrap();
    let Some((update, offered)) = pending.as_mut().filter(|(_, offered)| !offered) else { return Ok(None) };
    *offered = true;
    Ok(Some(json!({ "version": update.version, "current": update.current_version, "notes": update.body })))
}

#[tauri::command]
pub async fn update_install(app: AppHandle, webview: Webview, progress: Channel<Value>) -> Result<(), String> {
    browser::trusted(&webview)?;
    let update = app.state::<Pending>().0.lock().unwrap().as_ref().map(|(update, _)| update.clone()).ok_or("No update is available.")?;
    let mut downloaded = 0u64;
    update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress.send(json!({ "downloaded": downloaded, "total": total }));
            },
            || {},
        )
        .await
        .map_err(|error| error.to_string())?;
    app.restart();
}

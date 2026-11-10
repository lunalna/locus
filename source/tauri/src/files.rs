use std::path::PathBuf;
use std::sync::Mutex;

use tauri::ipc::{InvokeBody, Request, Response};
#[cfg(mobile)]
use tauri::Manager;
use tauri::{State, Webview};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

use crate::browser;

// Paths the user chose in a file dialog; the page only gets their index.
#[derive(Default)]
pub struct Files(Mutex<Vec<PathBuf>>);

fn pick(
    webview: &Webview,
    files: &Files,
    show: impl FnOnce(FileDialogBuilder<tauri::Wry>) -> Option<tauri_plugin_dialog::FilePath>,
) -> Result<Option<(usize, String)>, String> {
    browser::trusted(webview)?;
    let dialog = webview.dialog().file();
    #[cfg(desktop)]
    let dialog = dialog.set_parent(&webview.window());
    let Some(path) = show(dialog) else {
        return Ok(None);
    };
    Ok(Some(add(
        files,
        path.into_path().map_err(|e| e.to_string())?,
    )))
}

fn add(files: &Files, path: PathBuf) -> (usize, String) {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut files = files.0.lock().unwrap();
    files.push(path);
    (files.len() - 1, name)
}

fn path(files: &Files, id: usize) -> Result<PathBuf, String> {
    files
        .0
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or("Unknown file.".into())
}

#[tauri::command]
pub async fn file_pick_save(
    webview: Webview,
    files: State<'_, Files>,
    name: String,
) -> Result<Option<(usize, String)>, String> {
    // iOS can only export an existing file, so it is written to the app's documents and exported after.
    #[cfg(mobile)]
    return {
        browser::trusted(&webview)?;
        let documents = webview.path().document_dir().map_err(|e| e.to_string())?;
        Ok(Some(add(&files, documents.join(name))))
    };
    #[cfg(desktop)]
    let extension = name
        .rsplit_once('.')
        .map_or("ggb", |(_, extension)| extension);
    #[cfg(desktop)]
    pick(&webview, &files, |dialog| {
        dialog
            .set_file_name(&name)
            .add_filter("Locus", &[extension])
            .blocking_save_file()
    })
}

#[tauri::command]
pub async fn file_pick_open(
    webview: Webview,
    files: State<'_, Files>,
) -> Result<Option<(usize, String)>, String> {
    pick(&webview, &files, |dialog| {
        dialog
            .add_filter("Locus", &["ggb", "ggt", "ggs", "csv", "off", "pdf"])
            .blocking_pick_file()
    })
}

#[tauri::command]
pub fn file_read(webview: Webview, files: State<'_, Files>, id: usize) -> Result<Response, String> {
    browser::trusted(&webview)?;
    let bytes = std::fs::read(path(&files, id)?).map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub fn file_write(
    webview: Webview,
    files: State<'_, Files>,
    request: Request<'_>,
) -> Result<(), String> {
    browser::trusted(&webview)?;
    let id = request
        .headers()
        .get("file")
        .and_then(|id| id.to_str().ok()?.parse().ok())
        .ok_or("Missing file.")?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("Missing file contents.".into());
    };
    let path = path(&files, id)?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    #[cfg(mobile)]
    if let Some(name) = path.file_name() {
        webview
            .dialog()
            .file()
            .set_file_name(name.to_string_lossy())
            .save_file(|_| {});
    }
    Ok(())
}

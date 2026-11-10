// Performance settings. The page reads them from init.js when it loads.

use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview};

use crate::browser;

const FPS: [&str; 3] = ["display", "60", "30"];
const RESOLUTIONS: [&str; 2] = ["full", "standard"];

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("performance.json"))
}

fn settings(app: &AppHandle) -> Value {
    let saved = path(app)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_default();
    json!({
        "low": saved["low"].as_bool().unwrap_or(false),
        "fps": saved["fps"].as_str().filter(|v| FPS.contains(v)).unwrap_or(FPS[0]),
        "resolution": saved["resolution"].as_str().filter(|v| RESOLUTIONS.contains(v)).unwrap_or(RESOLUTIONS[0]),
        "asymptotes": saved["asymptotes"].as_bool().unwrap_or(true),
    })
}

// Low performance mode overrides the individual choices.
fn effective(app: &AppHandle) -> (u32, bool, bool) {
    let settings = settings(app);
    if settings["low"] == true {
        return (30, false, false);
    }
    (
        settings["fps"].as_str().unwrap().parse().unwrap_or(0),
        settings["resolution"] == "full",
        settings["asymptotes"] == true,
    )
}

pub fn script(app: &AppHandle) -> String {
    let (fps, full, asymptotes) = effective(app);
    format!(
        "window.locusPerformance = {};\n",
        json!({ "fps": fps, "maxPixelRatio": if full { 0 } else { 1 }, "asymptotes": if asymptotes { 1 } else { 0 } })
    )
}

#[cfg_attr(not(any(target_os = "macos", target_os = "ios")), allow(unused_variables))]
pub fn apply(webview: &Webview) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    crate::framerate::set_limit(webview, effective(webview.app_handle()).0 != 0);
}

#[tauri::command]
pub fn performance_settings(app: AppHandle, webview: Webview) -> Result<Value, String> {
    browser::trusted(&webview)?;
    Ok(settings(&app))
}

#[tauri::command]
pub fn performance_set(app: AppHandle, webview: Webview, name: String, value: Value) -> Result<(), String> {
    browser::trusted(&webview)?;
    let valid = match name.as_str() {
        "low" | "asymptotes" => value.is_boolean(),
        "fps" => value.as_str().is_some_and(|v| FPS.contains(&v)),
        "resolution" => value.as_str().is_some_and(|v| RESOLUTIONS.contains(&v)),
        _ => false,
    };
    if !valid {
        return Err("Invalid performance setting.".into());
    }
    let mut settings = settings(&app);
    settings[name] = value;
    let path = path(&app)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, settings.to_string()).map_err(|e| e.to_string())?;
    for label in crate::session::labels(&app) {
        if let Some(view) = app.get_webview(&label) {
            apply(&view);
        }
    }
    crate::session::refresh_others(&app, &webview, &Value::Null);
    Ok(())
}

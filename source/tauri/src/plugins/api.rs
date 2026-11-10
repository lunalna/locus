// The commands plugin pages and panels call, and the ones the settings and the app pages use for them.
// Panels pass their plugin id as `plugin`; the app page they are shown in makes the call for them.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Url, Webview};
use tauri_plugin_dialog::DialogExt;

use super::{caller, emit, label, plugin, window, with, Caller, Plugins};
use crate::{browser, exam, tools};

#[tauri::command]
pub fn plugin_window(
    app: AppHandle,
    webview: Webview,
    action: String,
    value: Option<String>,
    plugin: Option<String>,
) -> Result<(), String> {
    let caller = caller(&webview, plugin)?;
    if !caller.plugin.has_window() {
        return Err("This plugin has no window.".into());
    }
    let id = caller.plugin.id.clone();
    let number = || -> Result<f64, String> {
        value.as_deref().ok_or("Missing value.")?.parse::<f64>().map_err(|e| e.to_string())
    };
    match action.as_str() {
        "show" => super::open(&app, &id)?,
        "hide" => super::hide(&app, &id)?,
        // iPadOS moves the panel with a native pan gesture instead, and panels cannot move the window.
        #[cfg(mobile)]
        "drag" | "resize" => {}
        #[cfg(desktop)]
        "drag" if caller.page => webview.window().start_dragging().map_err(|e| e.to_string())?,
        #[cfg(desktop)]
        "resize" if caller.page => {
            use tauri_runtime::ResizeDirection;
            let direction = match value.as_deref() {
                Some("n") => ResizeDirection::North,
                Some("s") => ResizeDirection::South,
                Some("e") => ResizeDirection::East,
                Some("w") => ResizeDirection::West,
                Some("ne") => ResizeDirection::NorthEast,
                Some("nw") => ResizeDirection::NorthWest,
                Some("se") => ResizeDirection::SouthEast,
                _ => ResizeDirection::SouthWest,
            };
            webview.window().start_resize_dragging(direction).map_err(|e| e.to_string())?;
        }
        "opacity" => {
            let opacity = number()?.clamp(0.1, 1.0);
            let views = with(&app, &id, |instance| {
                instance.opacity = opacity;
                instance.views.clone()
            })
            .unwrap_or_default();
            for view in views {
                if let Some(webview) = app.get_webview(&view.id) {
                    window::set_content_opacity(&webview, opacity)?;
                }
            }
        }
        "blur" => window::blur(&app, &label(&id), number()?.clamp(0.0, 1.0))?,
        "filter" => {
            let filter = value.unwrap_or_default();
            let views = with(&app, &id, |instance| {
                instance.filter = filter.clone();
                instance.views.clone()
            })
            .unwrap_or_default();
            for view in views {
                if let Some(webview) = app.get_webview(&view.id) {
                    let _ = webview.eval(window::filter_script(&filter));
                }
            }
        }
        _ => return Err("Unknown window action.".into()),
    }
    Ok(())
}

#[tauri::command]
pub fn plugin_view(
    app: AppHandle,
    webview: Webview,
    action: String,
    id: Option<String>,
    url: Option<String>,
    plugin: Option<String>,
) -> Result<Value, String> {
    let caller = caller(&webview, plugin)?;
    caller.require("websites")?;
    if !caller.plugin.has_window() {
        return Err("Websites need a plugin window.".into());
    }
    let plugin = caller.plugin.id.clone();
    let view = || -> Result<Webview, String> {
        let id = id.as_deref().ok_or("Missing view.")?;
        super::views(&app, &plugin)
            .iter()
            .any(|view| view.id == id)
            .then(|| app.get_webview(id))
            .flatten()
            .ok_or("Unknown view.".into())
    };
    match action.as_str() {
        "list" => return Ok(super::listing(&app, &plugin)),
        "create" => {
            let url = match url {
                Some(url) => browser::address(&url)?,
                None => Url::parse(browser::HOME).unwrap(),
            };
            return Ok(json!(window::create_view(&app, &caller.plugin, url)?));
        }
        "show" => window::show(&app, &plugin, id.as_deref())?,
        "close" => {
            let label = view()?.label().to_string();
            with(&app, &plugin, |instance| {
                instance.views.retain(|view| view.id != label);
                if instance.shown.as_ref() == Some(&label) {
                    instance.shown = None;
                }
            });
            window::close_view(&app, &label)?;
            super::publish(&app, &plugin);
        }
        "back" => view()?.eval("history.back()").map_err(|e| e.to_string())?,
        "forward" => view()?.eval("history.forward()").map_err(|e| e.to_string())?,
        "reload" => view()?.reload().map_err(|e| e.to_string())?,
        "navigate" => {
            let url = browser::address(&url.ok_or("Missing address.")?)?;
            if !browser::navigation(&app, &url) {
                return Err("This site is not allowed in this exam.".into());
            }
            view()?.navigate(url).map_err(|e| e.to_string())?;
        }
        _ => return Err("Unknown view action.".into()),
    }
    Ok(Value::Null)
}

// Reads or changes the construction in an app window: the window a panel is in, or the one used last.
#[tauri::command]
pub async fn plugin_app(
    app: AppHandle,
    webview: Webview,
    action: String,
    args: Option<Value>,
    plugin: Option<String>,
) -> Result<Value, String> {
    let caller = caller(&webview, plugin)?;
    let args = args.unwrap_or(json!({}));
    let read = |body: &str| (body.to_string(), "construction:read");
    let write = |body: &str| (body.to_string(), "construction:write");
    let (body, permission) = match action.as_str() {
        "objects" => read(
            "return JSON.stringify(ggbApplet.getAllObjectNames().map((name) =>
                 ({ name, type: ggbApplet.getObjectType(name), value: ggbApplet.getValueString(name) })));",
        ),
        "get" => read(
            "const n = args.name;
             if (!ggbApplet.exists(n)) throw `There is no object ${n}.`;
             return JSON.stringify({
                 name: n, type: ggbApplet.getObjectType(n), value: ggbApplet.getValue(n),
                 text: ggbApplet.getValueString(n), definition: ggbApplet.getDefinitionString(n),
                 command: ggbApplet.getCommandString(n), x: ggbApplet.getXcoord(n), y: ggbApplet.getYcoord(n),
                 color: ggbApplet.getColor(n), visible: ggbApplet.getVisible(n),
             });",
        ),
        "xml" => read("return JSON.stringify(ggbApplet.getXML());"),
        "png" => read("return JSON.stringify(ggbApplet.getPNGBase64(1, false, 72));"),
        "viewport" => read(
            "const view = JSON.parse(ggbApplet.getViewProperties(1));
             return JSON.stringify({ xmin: view.xMin, xmax: view.xMin + view.width * view.invXscale,
                 ymin: view.yMin, ymax: view.yMin + view.height * view.invYscale });",
        ),
        "cas" => read(&format!("{}return JSON.stringify(ggbApplet.evalCommandCAS(args.input));", tools::CAS_READY)),
        "eval" => write(
            "ggbApplet.setErrorDialogsActive(false);
             try { const labels = await ggbApplet.asyncEvalCommandGetLabels(args.command);
                   return JSON.stringify(labels ? labels.split(',') : []); }
             finally { ggbApplet.setErrorDialogsActive(true); }",
        ),
        "set" => write(
            "const n = args.name;
             if (!ggbApplet.exists(n)) throw `There is no object ${n}.`;
             if ('value' in args) ggbApplet.setValue(n, args.value);
             if ('x' in args && 'y' in args) ggbApplet.setCoords(n, args.x, args.y);
             if (args.color) ggbApplet.setColor(n, ...[1, 3, 5].map((i) => parseInt(args.color.slice(i, i + 2), 16)));
             if ('visible' in args) ggbApplet.setVisible(n, args.visible);
             if ('caption' in args) ggbApplet.setCaption(n, args.caption);
             return 'null';",
        ),
        "delete" => write("ggbApplet.deleteObject(args.name); return 'null';"),
        "setViewport" => write(
            "if (!(args.xmin < args.xmax && args.ymin < args.ymax)) throw 'The minimum must be below the maximum.';
             ggbApplet.setCoordSystem(args.xmin, args.xmax, args.ymin, args.ymax);
             return 'null';",
        ),
        _ => return Err("Unknown construction action.".into()),
    };
    caller.require(permission)?;
    let result = tools::geogebra(&app, &caller.target, &args, &body).await?;
    serde_json::from_str(&result).map_err(|e| e.to_string())
}

// Construction changes reported by an app page, sent on to the plugins that read the construction.
#[tauri::command]
pub fn plugin_app_event(app: AppHandle, webview: Webview, events: Value) -> Result<(), String> {
    browser::trusted(&webview)?;
    let event = json!({"type": "construction", "window": webview.label(), "events": events});
    super::broadcast(&app, "construction:read", &event);
    Ok(())
}

// What the app pages show for the running plugins: panels, commands and windows.
#[tauri::command]
pub fn plugin_app_state(app: AppHandle, webview: Webview) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let running = super::running(&app);
    let plugins: Vec<Value> = running
        .iter()
        .map(|plugin| {
            let panel = &plugin.manifest["panel"];
            json!({
                "id": plugin.id,
                "name": plugin.name(),
                "window": plugin.has_window(),
                "commands": plugin.manifest["commands"].as_array().cloned().unwrap_or_default(),
                "panel": panel.is_object().then(|| json!({
                    "url": plugin.page_url(panel["main"].as_str().unwrap_or("panel.html")).to_string(),
                    "title": panel["title"].as_str().unwrap_or(plugin.name()),
                    "width": panel["width"].as_f64().unwrap_or(320.0),
                })),
            })
        })
        .collect();
    let events = running.iter().any(|plugin| plugin.permits("construction:read"));
    Ok(json!({"plugins": plugins, "events": events}))
}

// Runs a plugin command chosen in an app window, or shows and hides the plugin window without one.
#[tauri::command]
pub fn plugin_command(app: AppHandle, webview: Webview, plugin: String, command: Option<String>) -> Result<(), String> {
    browser::trusted(&webview)?;
    match command {
        Some(command) => super::command_event(&app, &plugin, &command, webview.label()),
        None => super::toggle(app, plugin),
    }
    Ok(())
}

// Shows or hides the plugin's panel in the app window it works on.
#[tauri::command]
pub fn plugin_panel(app: AppHandle, webview: Webview, show: bool, plugin: Option<String>) -> Result<(), String> {
    let caller = caller(&webview, plugin)?;
    if !caller.plugin.manifest["panel"].is_object() {
        return Err("This plugin has no panel.".into());
    }
    super::panel(&app, &caller.plugin.id, &caller.target, Some(show));
    Ok(())
}

fn data_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(app.path().app_config_dir().map_err(|e| e.to_string())?.join("plugin-data").join(format!("{id}.json")))
}

// Each plugin keeps its storage and settings in one file.
fn data(app: &AppHandle, id: &str) -> Value {
    data_path(app, id)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({"storage": {}, "settings": {}}))
}

fn save_data(app: &AppHandle, id: &str, data: &Value) -> Result<(), String> {
    let path = data_path(app, id)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(path, data.to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugin_storage(
    app: AppHandle,
    webview: Webview,
    action: String,
    key: Option<String>,
    value: Option<Value>,
    plugin: Option<String>,
) -> Result<Value, String> {
    let id = caller(&webview, plugin)?.plugin.id;
    let mut data = data(&app, &id);
    let key = || key.clone().ok_or("Missing key.");
    match action.as_str() {
        "get" => return Ok(data["storage"][key()?].clone()),
        "keys" => {
            let keys: Vec<&String> = data["storage"].as_object().map(|map| map.keys().collect()).unwrap_or_default();
            return Ok(json!(keys));
        }
        "set" => data["storage"][key()?] = value.unwrap_or(Value::Null),
        "remove" => {
            if let Some(storage) = data["storage"].as_object_mut() {
                storage.remove(&key()?);
            }
        }
        _ => return Err("Unknown storage action.".into()),
    }
    save_data(&app, &id, &data)?;
    Ok(Value::Null)
}

fn settings(app: &AppHandle, plugin: &super::Plugin) -> Value {
    values(app, &plugin.id, &plugin.manifest)
}

// The plugin's settings from its manifest, with the saved value or the default.
pub fn values(app: &AppHandle, id: &str, manifest: &Value) -> Value {
    let saved = data(app, id)["settings"].clone();
    let values: serde_json::Map<String, Value> = manifest["settings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|setting| {
            let key = setting["key"].as_str()?;
            let value = if saved[key].is_null() { setting["default"].clone() } else { saved[key].clone() };
            Some((key.to_string(), value))
        })
        .collect();
    Value::Object(values)
}

#[tauri::command]
pub fn plugin_settings(app: AppHandle, webview: Webview, plugin: Option<String>) -> Result<Value, String> {
    Ok(settings(&app, &caller(&webview, plugin)?.plugin))
}

// Opens or saves a file the user picks. Options: name, contents, extensions and base64 for binary files.
#[tauri::command]
pub async fn plugin_file(
    app: AppHandle,
    webview: Webview,
    action: String,
    options: Option<Value>,
    plugin: Option<String>,
) -> Result<Value, String> {
    use base64::Engine;
    let caller: Caller = caller(&webview, plugin)?;
    caller.require("files")?;
    if exam::active(&app) {
        return Err("Files are not available during an exam.".into());
    }
    let options = options.unwrap_or(json!({}));
    let encoding = base64::engine::general_purpose::STANDARD;
    let binary = options["base64"] == true;
    let mut dialog = app.dialog().file();
    if let Some(extensions) = options["extensions"].as_array() {
        let extensions: Vec<&str> = extensions.iter().filter_map(Value::as_str).collect();
        dialog = dialog.add_filter(caller.plugin.name(), &extensions);
    }
    match action.as_str() {
        "open" => {
            let Some(picked) = dialog.blocking_pick_file() else {
                return Ok(Value::Null);
            };
            let path = picked.into_path().map_err(|e| e.to_string())?;
            let body = fs::read(&path).map_err(|e| e.to_string())?;
            let contents = if binary { encoding.encode(body) } else { String::from_utf8_lossy(&body).into_owned() };
            let name = path.file_name().map(|name| name.to_string_lossy().into_owned());
            Ok(json!({"name": name, "contents": contents}))
        }
        "save" => {
            let contents = options["contents"].as_str().unwrap_or_default();
            let body = if binary { encoding.decode(contents).map_err(|e| e.to_string())? } else { contents.as_bytes().to_vec() };
            let name = options["name"].as_str().unwrap_or_default();
            let Some(picked) = dialog.set_file_name(name).blocking_save_file() else {
                return Ok(Value::Null);
            };
            let path = picked.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, body).map_err(|e| e.to_string())?;
            Ok(json!(path.file_name().map(|name| name.to_string_lossy().into_owned())))
        }
        _ => Err("Unknown file action.".into()),
    }
}

#[tauri::command]
pub async fn plugin_tool_result(
    app: AppHandle,
    webview: Webview,
    call: u64,
    result: Option<Value>,
    error: Option<String>,
) -> Result<(), String> {
    caller(&webview, None)?;
    let tx = app.state::<Plugins>().0.lock().unwrap().calls.remove(&call);
    if let Some(tx) = tx {
        let _ = tx.send(error.map_or(Ok(result.unwrap_or(Value::Null)), Err)).await;
    }
    Ok(())
}

// The running exam: its state, lines in its log and status, and the accommodations a plugin allows.
#[tauri::command]
pub fn plugin_exam(
    app: AppHandle,
    webview: Webview,
    action: String,
    text: Option<String>,
    allowed: Option<bool>,
    plugin: Option<String>,
) -> Result<Value, String> {
    let caller = caller(&webview, plugin)?;
    if action == "state" {
        return Ok(json!({
            "active": exam::active(&app),
            "assistant": exam::assistant_enabled(&app),
            "leaving": exam::leaving_allowed(&app),
        }));
    }
    let name = caller.plugin.name();
    let allowed = || allowed.ok_or("Missing allowed.");
    match action.as_str() {
        "assistant" => {
            caller.require("assistant")?;
            exam::allow_assistant(&app, allowed()?, name)?;
        }
        "leaving" => {
            caller.require("exam")?;
            exam::allow_leaving(&app, allowed()?, name)?;
        }
        "log" => {
            caller.require("exam")?;
            exam::running(&app)?;
            exam::record(&app, format!("{name}: {}", text.ok_or("Missing text.")?));
        }
        "status" => {
            caller.require("exam")?;
            exam::status(&app, name, text)?;
        }
        _ => return Err("Unknown exam action.".into()),
    }
    Ok(Value::Null)
}

#[tauri::command]
pub fn plugin_list(app: AppHandle, webview: Webview) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let disabled = super::disabled(&app);
    let items: Vec<Value> = super::all(&app)
        .iter()
        .map(|plugin| {
            let manifest = &plugin.manifest;
            json!({
                "id": plugin.id,
                "name": manifest["name"],
                "version": manifest["version"],
                "author": manifest["author"],
                "description": manifest["description"],
                "license": manifest["license"],
                "signed": plugin.signed,
                "exam": plugin.exam(),
                "enabled": !disabled.contains(&plugin.id),
            })
        })
        .collect();
    Ok(json!({"items": items}))
}

#[tauri::command]
pub fn plugin_enable(app: AppHandle, webview: Webview, id: String, enabled: bool) -> Result<(), String> {
    browser::trusted(&webview)?;
    super::set_disabled(&app, &id, !enabled)?;
    super::sync(&app);
    Ok(())
}

#[tauri::command]
pub fn plugin_setting_set(app: AppHandle, webview: Webview, id: String, key: String, value: Value) -> Result<(), String> {
    browser::trusted(&webview)?;
    let mut data = data(&app, &id);
    data["settings"][key] = value;
    save_data(&app, &id, &data)?;
    if let Some(plugin) = plugin(&app, &id) {
        emit(&app, &id, &json!({"type": "settings", "settings": settings(&app, &plugin)}));
    }
    Ok(())
}

// iPadOS has no global shortcuts, so the pages check key presses against these.
#[cfg(target_os = "ios")]
#[tauri::command]
pub fn plugin_shortcuts(app: AppHandle) -> Value {
    json!(super::shortcuts(&app)
        .iter()
        .map(|combo| json!({"mods": combo.mods & !super::keys::RIGHT_ALT, "code": combo.code}))
        .collect::<Vec<_>>())
}

#[cfg(target_os = "ios")]
#[tauri::command]
pub fn plugin_key(app: AppHandle, webview: Webview, mods: u8, code: Option<String>) -> Result<(), String> {
    if browser::trusted(&webview).is_err() {
        caller(&webview, None)?;
    }
    super::pressed(&app, &super::keys::Combo { mods, code });
    Ok(())
}

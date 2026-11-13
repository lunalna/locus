// The commands plugin pages and panels call, and the ones the settings and the app pages use for them.
// Panels pass their plugin id as `plugin`; the app page they are shown in makes the call for them.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Url, Webview};
use tauri_plugin_dialog::DialogExt;

use super::{caller, emit, label, plugin, window, with, Caller, Plugins};
use crate::{browser, exam, session, tools};

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
            if !browser::navigation(&url) {
                return Err("This site is not allowed in this exam.".into());
            }
            view()?.navigate(url).map_err(|e| e.to_string())?;
        }
        _ => return Err("Unknown view action.".into()),
    }
    Ok(Value::Null)
}

const READ: &str = "construction:read";
const WRITE: &str = "construction:write";

// The script of a construction action, the permission it needs and whether it changes the construction,
// which then gets an undo point. Scripts read `args` and return JSON.
fn script(action: &str) -> Option<(String, &'static str, bool)> {
    let (body, permission, changes) = match action {
        "objects" => (
            "return JSON.stringify(ggbApplet.getAllObjectNames(args.type).map((name) =>
                 ({ name, type: ggbApplet.getObjectType(name), value: ggbApplet.getValueString(name) })));",
            READ,
            false,
        ),
        "get" => (
            "const n = args.name;
             if (!ggbApplet.exists(n)) throw `There is no object ${n}.`;
             return JSON.stringify({
                 name: n, type: ggbApplet.getObjectType(n), value: ggbApplet.getValue(n),
                 text: ggbApplet.getValueString(n), definition: ggbApplet.getDefinitionString(n),
                 command: ggbApplet.getCommandString(n), x: ggbApplet.getXcoord(n), y: ggbApplet.getYcoord(n),
                 color: ggbApplet.getColor(n), visible: ggbApplet.getVisible(n), caption: ggbApplet.getCaption(n),
                 labelVisible: ggbApplet.getLabelVisible(n), layer: ggbApplet.getLayer(n), fixed: ggbApplet.isFixed(n),
                 lineThickness: ggbApplet.getLineThickness(n), lineStyle: ggbApplet.getLineStyle(n),
                 pointSize: ggbApplet.getPointSize(n), pointStyle: ggbApplet.getPointStyle(n),
                 filling: ggbApplet.getFilling(n), animating: ggbApplet.isAnimating(n),
             });",
            READ,
            false,
        ),
        "xml" => ("return JSON.stringify(ggbApplet.getXML());", READ, false),
        "png" => ("return JSON.stringify(ggbApplet.getPNGBase64(args.scale ?? 1, !!args.transparent, 72));", READ, false),
        "svg" => ("return JSON.stringify(await new Promise((resolve) => ggbApplet.exportSVG(resolve)));", READ, false),
        "file" => ("return JSON.stringify(await new Promise((resolve) => ggbApplet.getBase64(resolve)));", READ, false),
        "viewport" => ("return JSON.stringify(window.locusPluginView());", READ, false),
        "cas" => {
            return Some((format!("{}return JSON.stringify(ggbApplet.evalCommandCAS(args.input));", tools::CAS_READY), READ, false))
        }
        "selection" => ("return JSON.stringify(window.locusPluginSelection());", READ, false),
        "hits" => ("return JSON.stringify(window.locusPluginHits(args.x, args.y));", READ, false),
        "mode" => ("return JSON.stringify(ggbApplet.getMode());", READ, false),
        "eval" => (
            "ggbApplet.setErrorDialogsActive(false);
             try { const labels = await ggbApplet.asyncEvalCommandGetLabels(args.command);
                   return JSON.stringify(labels ? labels.split(',') : []); }
             finally { ggbApplet.setErrorDialogsActive(true); }",
            WRITE,
            true,
        ),
        "set" => (
            "const n = args.name;
             if (!ggbApplet.exists(n)) throw `There is no object ${n}.`;
             if ('value' in args) ggbApplet.setValue(n, args.value);
             if ('x' in args && 'y' in args) ggbApplet.setCoords(n, args.x, args.y);
             if (args.color) ggbApplet.setColor(n, ...[1, 3, 5].map((i) => parseInt(args.color.slice(i, i + 2), 16)));
             const setters = { visible: 'setVisible', caption: 'setCaption', labelVisible: 'setLabelVisible', layer: 'setLayer',
                 fixed: 'setFixed', lineThickness: 'setLineThickness', lineStyle: 'setLineStyle', pointSize: 'setPointSize',
                 pointStyle: 'setPointStyle', filling: 'setFilling', trace: 'setTrace', animating: 'setAnimating' };
             for (const [key, setter] of Object.entries(setters)) if (key in args) ggbApplet[setter](n, args[key]);
             return 'null';",
            WRITE,
            true,
        ),
        "delete" => ("ggbApplet.deleteObject(args.name); return 'null';", WRITE, true),
        "setViewport" => (
            "if (!(args.xmin < args.xmax && args.ymin < args.ymax)) throw 'The minimum must be below the maximum.';
             ggbApplet.setCoordSystem(args.xmin, args.xmax, args.ymin, args.ymax);
             return 'null';",
            WRITE,
            false,
        ),
        "select" => ("window.locusPluginSelect(args.names ?? []); return 'null';", WRITE, false),
        "setMode" => ("ggbApplet.setMode(args.mode); return 'null';", WRITE, false),
        "perspective" => ("ggbApplet.setPerspective(args.code); return 'null';", WRITE, false),
        "animate" => ("args.running ? ggbApplet.startAnimation() : ggbApplet.stopAnimation(); return 'null';", WRITE, false),
        "undo" => ("ggbApplet.undo(); return 'null';", WRITE, false),
        "redo" => ("ggbApplet.redo(); return 'null';", WRITE, false),
        "open" => ("await new Promise((resolve) => ggbApplet.setBase64(args.file, resolve)); return 'null';", WRITE, false),
        "new" => ("ggbApplet.newConstruction(); return 'null';", WRITE, false),
        _ => return None,
    };
    Some((body.to_string(), permission, changes))
}

// Reads or changes the construction in an app window: the one named in `window`, the window a panel is in,
// or the one used last. A batch runs its calls in order with one undo point and returns their results.
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
    let windows = session::labels(&app);
    if action == "windows" {
        caller.require(READ)?;
        let list: Vec<Value> =
            windows.iter().map(|label| json!({"window": label, "app": session::mode(&app, label).name()})).collect();
        return Ok(json!({"windows": list, "active": session::active(&app)}));
    }
    let target = match args["window"].as_str() {
        Some(window) if windows.iter().any(|label| label == window) => window.to_string(),
        Some(_) => return Err("There is no such app window.".into()),
        None => caller.target.clone(),
    };
    let calls = match action.as_str() {
        "batch" => args["calls"].as_array().ok_or("Missing calls.")?.clone(),
        _ => vec![json!({"action": action, "args": args})],
    };
    let mut undo = false;
    let mut steps = String::new();
    for call in &calls {
        let name = call["action"].as_str().unwrap_or_default();
        let (body, permission, changes) = script(name).ok_or(format!("Unknown construction action {name}."))?;
        caller.require(permission)?;
        undo |= changes;
        let args = if call["args"].is_object() { call["args"].clone() } else { json!({}) };
        steps += &format!("results.push(JSON.parse(await (async (args) => {{ {body} }})({args})));\n");
    }
    let body = format!(
        "const results = [];
         try {{ {steps} }} finally {{ {} }}
         return JSON.stringify(results);",
        if undo { "ggbApplet.setUndoPoint();" } else { "" }
    );
    let results: Value = serde_json::from_str(&tools::geogebra(&app, &target, &json!({}), &body).await?).map_err(|e| e.to_string())?;
    Ok(if action == "batch" { results } else { results[0].clone() })
}

// Events about an app window that its page reports, sent on to the plugins that listen to them.
#[tauri::command]
pub fn plugin_app_event(app: AppHandle, webview: Webview, mut event: Value) -> Result<(), String> {
    browser::trusted(&webview)?;
    let kind = event["type"].as_str().unwrap_or_default();
    if kind == "window" || !super::EVENTS.contains(&kind) {
        return Err("Unknown event.".into());
    }
    event["window"] = json!(webview.label());
    super::broadcast(&app, &event);
    Ok(())
}

// Picking a plugin's tool in an app window and the input on the graphics view while it is picked.
#[tauri::command]
pub fn plugin_input(app: AppHandle, webview: Webview, plugin: String, mut event: Value) -> Result<(), String> {
    browser::trusted(&webview)?;
    if !matches!(event["type"].as_str(), Some("input" | "toolActive")) {
        return Err("Unknown event.".into());
    }
    event["window"] = json!(webview.label());
    emit(&app, &plugin, &event);
    Ok(())
}

// What the app pages show for the running plugins: panels, overlays, styles, scripts, commands and windows.
// Scripts are left out during exams.
#[tauri::command]
pub fn plugin_app_state(app: AppHandle, webview: Webview) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let running = super::running(&app);
    let exam = exam::active(&app);
    let plugins: Vec<Value> = running
        .iter()
        .map(|plugin| {
            let manifest = &plugin.manifest;
            let panel = &manifest["panel"];
            let file = |path: &Value| path.as_str().map(|path| plugin.page_url(path).to_string());
            let updates = with(&app, &plugin.id, |instance| instance.commands.clone()).unwrap_or_default();
            let commands: Vec<Value> = manifest["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|command| {
                    let mut command = command.clone();
                    if let Some(update) = command["id"].as_str().and_then(|id| updates.get(id)) {
                        for key in ["title", "visible"].into_iter().filter(|key| !update[key].is_null()) {
                            command[key] = update[key].clone();
                        }
                    }
                    command["icon"] = json!(file(&command["icon"]));
                    (command["visible"] != false).then_some(command)
                })
                .collect();
            json!({
                "id": plugin.id,
                "name": plugin.name(),
                "window": plugin.has_window(),
                "commands": commands,
                "panel": panel.is_object().then(|| json!({
                    "url": plugin.page_url(panel["main"].as_str().unwrap_or("panel.html")).to_string(),
                    "title": panel["title"].as_str().unwrap_or(plugin.name()),
                    "width": panel["width"].as_f64().unwrap_or(320.0),
                })),
                "overlay": manifest["overlay"].is_object()
                    .then(|| plugin.page_url(manifest["overlay"]["main"].as_str().unwrap_or("overlay.html")).to_string()),
                "styles": manifest["styles"].as_array().into_iter().flatten().filter_map(file).collect::<Vec<_>>(),
                "script": if exam { None } else { file(&manifest["script"]) },
            })
        })
        .collect();
    if plugins.iter().any(|plugin| plugin["script"].is_string()) {
        app.state::<Plugins>().0.lock().unwrap().scripted.insert(webview.label().into());
    }
    let events: Vec<&str> = super::EVENTS.into_iter().filter(|kind| running.iter().any(|plugin| plugin.listens(kind))).collect();
    Ok(json!({"plugins": plugins, "events": events}))
}

// Runs a plugin command chosen in an app window, or shows and hides the plugin window without one.
#[tauri::command]
pub fn plugin_command(
    app: AppHandle,
    webview: Webview,
    plugin: String,
    command: Option<String>,
    objects: Option<Vec<String>>,
) -> Result<(), String> {
    browser::trusted(&webview)?;
    match command {
        Some(command) => super::command_event(&app, &plugin, &command, webview.label(), objects),
        None => super::toggle(app, plugin),
    }
    Ok(())
}

// Changes the title of one of the plugin's commands, or hides and shows it, until the plugin stops.
#[tauri::command]
pub fn plugin_command_update(
    app: AppHandle,
    webview: Webview,
    id: String,
    title: Option<String>,
    visible: Option<bool>,
    plugin: Option<String>,
) -> Result<(), String> {
    let caller = caller(&webview, plugin)?;
    let known = caller.plugin.manifest["commands"].as_array().is_some_and(|list| list.iter().any(|command| command["id"] == id.as_str()));
    if !known {
        return Err(format!("There is no command {id}."));
    }
    with(&app, &caller.plugin.id, |instance| {
        let update = instance.commands.entry(id).or_insert_with(|| json!({}));
        if let Some(title) = title {
            update["title"] = json!(title);
        }
        if let Some(visible) = visible {
            update["visible"] = json!(visible);
        }
    });
    super::changed(&app);
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

// Saves setting values and tells the plugin's pages and panels.
fn save_settings(app: &AppHandle, id: &str, values: &serde_json::Map<String, Value>) -> Result<(), String> {
    let mut data = data(app, id);
    for (key, value) in values {
        data["settings"][key] = value.clone();
    }
    save_data(app, id, &data)?;
    if let Some(plugin) = plugin(app, id) {
        emit(app, id, &json!({"type": "settings", "settings": settings(app, &plugin)}));
    }
    Ok(())
}

// The plugin's settings, after saving the given values of settings from its manifest.
#[tauri::command]
pub fn plugin_settings(app: AppHandle, webview: Webview, values: Option<Value>, plugin: Option<String>) -> Result<Value, String> {
    let plugin = caller(&webview, plugin)?.plugin;
    if let Some(values) = values.as_ref().and_then(Value::as_object) {
        let current = settings(&app, &plugin);
        if let Some(key) = values.keys().find(|key| current.get(key.as_str()).is_none()) {
            return Err(format!("There is no setting {key}."));
        }
        save_settings(&app, &plugin.id, values)?;
    }
    Ok(settings(&app, &plugin))
}

// Tells the user something: a short note in the app window, a message, or a question that returns whether they agreed.
#[tauri::command]
pub async fn plugin_dialog(
    app: AppHandle,
    webview: Webview,
    action: String,
    text: String,
    plugin: Option<String>,
) -> Result<Value, String> {
    use tauri_plugin_dialog::MessageDialogButtons;
    let caller = caller(&webview, plugin)?;
    let name = caller.plugin.name();
    let dialog = || app.dialog().message(&text).title(name);
    match action.as_str() {
        "toast" => {
            let view = app.get_webview(&caller.target).ok_or("Locus window missing.")?;
            view.eval(format!("window.locusPluginToast && window.locusPluginToast({}, {})", json!(name), json!(text)))
                .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "message" => Ok(json!(dialog().blocking_show())),
        "confirm" => Ok(json!(dialog().buttons(MessageDialogButtons::OkCancel).blocking_show())),
        _ => Err("Unknown dialog action.".into()),
    }
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

// The background and foreground of the app theme, null for the default one.
#[tauri::command]
pub fn plugin_theme(app: AppHandle, webview: Webview, plugin: Option<String>) -> Result<Value, String> {
    caller(&webview, plugin)?;
    Ok(crate::resourcepack::base_theme(&app))
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
            exam::allow_assistant(&app, allowed()?)?;
        }
        "leaving" => {
            caller.require("exam")?;
            exam::allow_leaving(&app, allowed()?)?;
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
                "supported": plugin.supported(),
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
    save_settings(&app, &id, &serde_json::Map::from_iter([(key, value)]))
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

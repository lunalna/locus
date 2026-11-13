// A plugin is a folder with a plugin.json and its pages. The main page runs from launch, hidden or in a
// floating window above other apps; a panel page can show in a side panel of the app windows. Pages
// call the plugin_* commands and get events through window.locusPluginEvent; panels and overlays do the
// same through the app page they are shown in. Styles and scripts are added to the app pages.
// During exams only signed plugins with "exam": true run, and no scripts.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::async_runtime::Sender;
use tauri::http::Response;
use tauri::{AppHandle, Manager, Url, Webview, WebviewUrl};

use crate::{browser, exam, resourcepack, session};
use keys::Combo;

pub mod api;
pub mod keys;
pub mod signature;
mod window;

#[cfg(windows)]
const URL: &str = "http://plugin.localhost/";
#[cfg(not(windows))]
const URL: &str = "plugin://localhost/";
#[cfg(windows)]
const SOURCES: &str = "http://plugin.localhost http://ipc.localhost";
#[cfg(not(windows))]
const SOURCES: &str = "plugin: ipc:";

const DIMMED: f64 = 0.4;

// The plugin API version. Plugins that need a newer one, in the manifest's "api", do not run.
pub const API: u64 = 2;

// Events about the app windows that plugins with the construction:read permission list in "events".
pub const EVENTS: [&str; 6] = ["construction", "selection", "view", "mode", "undo", "window"];

#[derive(Clone)]
pub struct Plugin {
    id: String,
    manifest: Value,
    signed: bool,
}

impl Plugin {
    fn name(&self) -> &str {
        self.manifest["name"].as_str().unwrap_or(&self.id)
    }

    fn label(&self) -> String {
        label(&self.id)
    }

    fn page_url(&self, main: &str) -> Url {
        Url::parse(URL).unwrap().join(&format!("{}/{main}", self.id)).unwrap()
    }

    fn main_url(&self) -> Url {
        self.page_url(self.manifest["main"].as_str().unwrap_or("index.html"))
    }

    fn url(&self) -> WebviewUrl {
        let url = self.main_url();
        if cfg!(windows) {
            WebviewUrl::External(url)
        } else {
            WebviewUrl::CustomProtocol(url)
        }
    }

    // The signature covers the manifest, so the exam flag cannot be added afterwards.
    fn exam(&self) -> bool {
        self.signed && self.manifest["exam"] == true
    }

    fn has_window(&self) -> bool {
        self.manifest["window"].is_object()
    }

    fn permits(&self, permission: &str) -> bool {
        self.manifest["permissions"].as_array().is_some_and(|list| list.iter().any(|p| p == permission))
    }

    fn command(&self, id: &str) -> Option<&Value> {
        self.manifest["commands"].as_array()?.iter().find(|command| command["id"] == id)
    }

    fn supported(&self) -> bool {
        self.manifest["api"].as_u64().unwrap_or(1) <= API
    }

    fn listens(&self, event: &str) -> bool {
        self.permits("construction:read") && self.manifest["events"].as_array().is_some_and(|list| list.iter().any(|e| e == event))
    }


    fn size(&self, key: &str, default: f64) -> f64 {
        self.manifest["window"][key].as_f64().unwrap_or(default)
    }

    // Views fill the window inside these insets: top, right, bottom, left.
    fn inset(&self) -> [f64; 4] {
        let inset = &self.manifest["window"]["inset"];
        [0, 1, 2, 3].map(|i| inset[i].as_f64().unwrap_or(0.0))
    }

    fn shortcuts(&self) -> Vec<(String, Combo)> {
        self.manifest["shortcuts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|shortcut| {
                shortcut["platforms"]
                    .as_array()
                    .is_none_or(|platforms| platforms.iter().any(|p| p == std::env::consts::OS))
            })
            .filter_map(|shortcut| {
                Some((shortcut["action"].as_str()?.into(), Combo::parse(shortcut["keys"].as_str()?)?))
            })
            .collect()
    }

    // Assistant tools are named plugin_<id>_<name>, which Claude allows up to 64 characters.
    fn tools(&self) -> Vec<(String, Value)> {
        let prefix = format!("plugin_{}_", self.id.replace(['.', '-'], "_"));
        self.manifest["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|tool| {
                let name = format!("{prefix}{}", tool["name"].as_str()?);
                let valid = name.len() <= 64 && name.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c));
                valid.then(|| (name, tool.clone()))
            })
            .collect()
    }
}

#[derive(Default)]
pub struct Plugins(Mutex<State>);

#[derive(Default)]
struct State {
    running: HashMap<String, Instance>,
    // View labels keep counting up, since a closed view's label can still be taken on iPadOS.
    next: u64,
    // Assistant tool calls waiting for the plugin's answer.
    calls: HashMap<u64, Sender<Result<Value, String>>>,
    // App windows a plugin script was given to since they last loaded.
    scripted: HashSet<String>,
}

struct Instance {
    plugin: Plugin,
    views: Vec<View>,
    shown: Option<String>,
    locked: bool,
    dimmed: bool,
    opacity: f64,
    filter: String,
    // Titles and visibility the plugin gave its commands since it started.
    commands: HashMap<String, Value>,
}

impl Instance {
    fn new(plugin: Plugin) -> Self {
        Instance {
            plugin,
            views: Vec::new(),
            shown: None,
            locked: false,
            dimmed: false,
            opacity: 1.0,
            filter: String::new(),
            commands: HashMap::new(),
        }
    }
}

#[derive(Clone)]
struct View {
    id: String,
    title: String,
}

fn with<R>(app: &AppHandle, id: &str, f: impl FnOnce(&mut Instance) -> R) -> Option<R> {
    let state = app.state::<Plugins>();
    let mut state = state.0.lock().unwrap();
    state.running.get_mut(id).map(f)
}

fn views(app: &AppHandle, id: &str) -> Vec<View> {
    with(app, id, |instance| instance.views.clone()).unwrap_or_default()
}

fn running(app: &AppHandle) -> Vec<Plugin> {
    let state = app.state::<Plugins>();
    let state = state.0.lock().unwrap();
    state.running.values().map(|instance| instance.plugin.clone()).collect()
}

fn plugin(app: &AppHandle, id: &str) -> Option<Plugin> {
    with(app, id, |instance| instance.plugin.clone())
}

// Webview labels cannot hold dots, so they become colons, which plugin ids cannot hold.
fn label(id: &str) -> String {
    format!("plugin:{}", id.replace('.', ":"))
}

// The plugin a window, page or view belongs to.
pub fn owner(label: &str) -> Option<String> {
    let rest = label.strip_prefix("plugin:")?;
    Some(rest.split('/').next()?.replace(':', "."))
}

pub fn plugin_url(url: &Url) -> bool {
    (url.scheme() == "plugin" && url.host_str() == Some("localhost"))
        || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("plugin.localhost"))
}

// A plugin page may only show its own files.
fn own_url(id: &str, url: &Url) -> bool {
    plugin_url(url) && url.path().starts_with(&format!("/{id}/"))
}

fn dir(app: &AppHandle) -> Result<PathBuf, String> {
    resourcepack::kind_dir(app, resourcepack::Kind::Plugin)
}

fn all(app: &AppHandle) -> Vec<Plugin> {
    let mut plugins: Vec<Plugin> = resourcepack::installed(app, resourcepack::Kind::Plugin)
        .into_iter()
        .filter_map(|(id, path)| {
            let manifest = serde_json::from_str(&fs::read_to_string(path.join("plugin.json")).ok()?).ok()?;
            Some(Plugin {
                id,
                manifest,
                signed: signature::verify(&path),
            })
        })
        .collect();
    plugins.sort_by_key(|plugin| plugin.name().to_lowercase());
    plugins
}

fn disabled(app: &AppHandle) -> Vec<String> {
    resourcepack::settings(app)["disabledPlugins"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|id| id.as_str().map(String::from))
        .collect()
}

fn set_disabled(app: &AppHandle, id: &str, disable: bool) -> Result<(), String> {
    let mut list = disabled(app);
    list.retain(|other| other != id);
    if disable {
        list.push(id.into());
    }
    let mut settings = resourcepack::settings(app);
    settings["disabledPlugins"] = json!(list);
    resourcepack::save_settings(app, &settings)
}

// Starts the enabled plugins that may run and stops the others.
pub fn sync(app: &AppHandle) {
    let disabled = disabled(app);
    let exam = exam::active(app);
    let wanted: Vec<Plugin> = all(app)
        .into_iter()
        .filter(|plugin| plugin.supported() && !disabled.contains(&plugin.id) && (plugin.exam() || !exam))
        .collect();
    let started: Vec<String> = app.state::<Plugins>().0.lock().unwrap().running.keys().cloned().collect();
    for id in started.iter().filter(|id| !wanted.iter().any(|plugin| &plugin.id == *id)) {
        let _ = stop(app, id);
    }
    for plugin in wanted.into_iter().filter(|plugin| !started.contains(&plugin.id)) {
        start(app, plugin);
    }
    #[cfg(desktop)]
    crate::shortcuts::register(app, &shortcuts(app));
    #[cfg(target_os = "ios")]
    for view in app.webviews().values() {
        let label = view.label();
        if session::is_main(label) || (owner(label).is_some() && !label.contains('/')) {
            let _ = view.eval("window.locusLoadShortcuts && window.locusLoadShortcuts()");
        }
    }
    changed(app);
}

// Lets the app windows load what the running plugins show again.
fn changed(app: &AppHandle) {
    for label in session::labels(app) {
        if let Some(view) = app.get_webview(&label) {
            let _ = view.eval("window.locusPluginsChanged && window.locusPluginsChanged()");
        }
    }
}

fn start(app: &AppHandle, plugin: Plugin) {
    let id = plugin.id.clone();
    app.state::<Plugins>().0.lock().unwrap().running.insert(id.clone(), Instance::new(plugin.clone()));
    // Windows deadlocks when a window is made inside a command, so it is made on another thread.
    let app = app.clone();
    std::thread::spawn(move || {
        if self::plugin(&app, &id).is_some() {
            let _ = window::start(&app, &plugin);
        }
    });
}

fn stop(app: &AppHandle, id: &str) -> Result<(), String> {
    let views = app
        .state::<Plugins>()
        .0
        .lock()
        .unwrap()
        .running
        .remove(id)
        .map(|instance| instance.views.into_iter().map(|view| view.id).collect::<Vec<_>>())
        .unwrap_or_default();
    window::stop(app, &label(id), &views)
}

// A plugin is added after its permissions were shown, so it starts enabled.
pub fn added(app: &AppHandle, id: &str) {
    let _ = set_disabled(app, id, false);
    sync(app);
}

// What a plugin may do, for the details shown before it is added and in the settings.
pub fn details(id: &str, manifest: &Value, dir: &Path) -> Value {
    let plugin = Plugin {
        id: id.into(),
        manifest: manifest.clone(),
        signed: signature::verify(dir),
    };
    json!({
        "signed": plugin.signed,
        "exam": plugin.exam(),
        "supported": plugin.supported(),
        "permissions": manifest["permissions"],
        "network": manifest["network"],
        "window": plugin.has_window(),
        "panel": manifest["panel"].is_object(),
        "overlay": manifest["overlay"].is_object(),
        "styles": manifest["styles"].as_array().is_some_and(|styles| !styles.is_empty()),
        "script": manifest["script"].is_string(),
        "tools": plugin.tools().iter().map(|(_, tool)| tool["name"].clone()).collect::<Vec<_>>(),
    })
}

pub fn removed(app: &AppHandle, id: &str) {
    let _ = stop(app, id);
    sync(app);
}

// Starts all plugins over, so nothing carries into or out of an exam.
pub fn reset(app: &AppHandle) -> Result<(), String> {
    for plugin in running(app) {
        let views = with(app, &plugin.id, |instance| {
            let views = instance.views.iter().map(|view| view.id.clone()).collect::<Vec<_>>();
            *instance = Instance::new(instance.plugin.clone());
            views
        })
        .unwrap_or_default();
        window::reset(app, &plugin, &views)?;
    }
    sync(app);
    Ok(())
}

fn shortcuts(app: &AppHandle) -> Vec<Combo> {
    running(app)
        .iter()
        .flat_map(|plugin| plugin.shortcuts().into_iter().map(|(_, combo)| combo))
        .collect()
}

// Runs the shortcuts of all plugins that match a pressed key combination.
pub fn pressed(app: &AppHandle, combo: &Combo) {
    let matched: Vec<(String, String)> = running(app)
        .iter()
        .flat_map(|plugin| {
            plugin
                .shortcuts()
                .into_iter()
                .filter(|(_, shortcut)| shortcut.matches(combo))
                .map(|(action, _)| (plugin.id.clone(), action))
        })
        .collect();
    for (id, action) in matched {
        match action.as_str() {
            "toggle" => toggle(app.clone(), id),
            "lock" => lock(app.clone(), id),
            "dim" => dim(app, &id),
            "panel" => panel(app, &id, &session::active(app), None),
            _ => {
                let Some(command) = action.strip_prefix("command:") else { continue };
                let tool = plugin(app, &id).and_then(|plugin| plugin.command(command).map(|c| c["tool"] == true));
                if tool == Some(true) {
                    // Tools are picked in the app window, which then sends their input.
                    if let Some(view) = app.get_webview(&session::active(app)) {
                        let _ = view.eval(format!("window.locusPluginRun && window.locusPluginRun({}, {})", json!(id), json!(command)));
                    }
                } else {
                    command_event(app, &id, command, &session::active(app), None);
                }
            }
        }
    }
}

// Commands from an object's context menu name the selected objects.
fn command_event(app: &AppHandle, id: &str, command: &str, window: &str, objects: Option<Vec<String>>) {
    emit_page(app, id, &json!({"type": "command", "id": command, "window": window, "objects": objects}));
}

// A window that loads again or closes runs no plugin scripts anymore.
pub fn unscripted(app: &AppHandle, window: &str) {
    app.state::<Plugins>().0.lock().unwrap().scripted.remove(window);
}

// Plugin scripts have full access to their app window, so no exam may start while one runs.
pub fn scripted(app: &AppHandle) -> bool {
    !app.state::<Plugins>().0.lock().unwrap().scripted.is_empty()
}

// Opens, closes or toggles a plugin's panel in an app window.
fn panel(app: &AppHandle, id: &str, window: &str, show: Option<bool>) {
    if let Some(view) = app.get_webview(window) {
        let _ = view.eval(format!(
            "window.locusPluginPanel && window.locusPluginPanel({}, {})",
            json!(id),
            json!(show)
        ));
    }
}

fn open(app: &AppHandle, id: &str) -> Result<(), String> {
    let plugin = plugin(app, id).ok_or("This plugin is not running.")?;
    if !plugin.has_window() {
        return Err("This plugin has no window.".into());
    }
    let (locked, dimmed) = with(app, id, |instance| (instance.locked, instance.dimmed)).unwrap_or_default();
    if locked {
        return Ok(());
    }
    window::open(app, &plugin, if dimmed { DIMMED } else { 1.0 })?;
    emit(app, id, &json!({"type": "show"}));
    Ok(())
}

pub fn hide(app: &AppHandle, id: &str) -> Result<(), String> {
    let label = label(id);
    if window::visible(app, &label) {
        emit(app, id, &json!({"type": "hide"}));
        window::hide(app, &label)?;
    }
    Ok(())
}

pub fn hide_all(app: &AppHandle) -> Result<(), String> {
    for plugin in running(app) {
        hide(app, &plugin.id)?;
    }
    Ok(())
}

fn toggle(app: AppHandle, id: String) {
    std::thread::spawn(move || {
        if window::visible(&app, &label(&id)) {
            let _ = hide(&app, &id);
        } else {
            let _ = open(&app, &id);
        }
    });
}

fn lock(app: AppHandle, id: String) {
    let Some(locked) = with(&app, &id, |instance| {
        instance.locked = !instance.locked;
        instance.locked
    }) else {
        return;
    };
    if locked {
        let _ = hide(&app, &id);
    } else {
        std::thread::spawn(move || open(&app, &id));
    }
}

fn dim(app: &AppHandle, id: &str) {
    if let Some(dimmed) = with(app, id, |instance| {
        instance.dimmed = !instance.dimmed;
        instance.dimmed
    }) {
        window::alpha(app, &label(id), if dimmed { DIMMED } else { 1.0 });
    }
}

#[cfg(desktop)]
pub fn layout(app: &AppHandle, id: &str) {
    if let Some(plugin) = plugin(app, id) {
        window::layout(app, &plugin);
    }
}

// Events for the main page only, such as commands and tool calls, which need one answer.
fn emit_page(app: &AppHandle, id: &str, event: &Value) {
    if let Some(page) = app.get_webview(&label(id)) {
        let _ = page.eval(format!("window.locusPluginEvent && window.locusPluginEvent({event})"));
    }
}

// Events for the main page and the plugin's panels in all app windows.
fn emit(app: &AppHandle, id: &str, event: &Value) {
    emit_page(app, id, event);
    for window in session::labels(app) {
        if let Some(view) = app.get_webview(&window) {
            let _ = view.eval(format!(
                "window.locusPluginPanelEvent && window.locusPluginPanelEvent({}, {event})",
                json!(id)
            ));
        }
    }
}

pub fn theme_changed(app: &AppHandle, theme: &Value) {
    for plugin in running(app) {
        emit(app, &plugin.id, &json!({"type": "theme", "theme": theme}));
    }
}

// Sends an event about the app windows to the plugins that listen to its type.
fn broadcast(app: &AppHandle, event: &Value) {
    let kind = event["type"].as_str().unwrap_or_default();
    for plugin in running(app).iter().filter(|plugin| plugin.listens(kind)) {
        emit(app, &plugin.id, event);
    }
}

// Tells plugins that read the construction which app window is in use. iPadOS has one window.
#[cfg_attr(mobile, allow(dead_code))]
pub fn focused(app: &AppHandle, window: &str) {
    let event = json!({"type": "window", "window": window, "app": session::mode(app, window).name()});
    broadcast(app, &event);
}

fn listing(app: &AppHandle, id: &str) -> Value {
    let (views, shown) = with(app, id, |instance| (instance.views.clone(), instance.shown.clone())).unwrap_or_default();
    let views: Vec<Value> = views
        .iter()
        .map(|view| {
            let url = app
                .get_webview(&view.id)
                .and_then(|webview| webview.url().ok())
                .map(|url| url.to_string())
                .unwrap_or_default();
            json!({"id": view.id, "title": view.title, "url": url})
        })
        .collect();
    json!({"views": views, "shown": shown})
}

fn publish(app: &AppHandle, id: &str) {
    let mut event = listing(app, id);
    event["type"] = json!("views");
    emit(app, id, &event);
}

// The plugin of a call, and the app window it works on.
struct Caller {
    plugin: Plugin,
    target: String,
    // Whether the call came from the main page rather than a panel, which may move the window.
    #[cfg_attr(mobile, allow(dead_code))]
    page: bool,
}

impl Caller {
    fn require(&self, permission: &str) -> Result<(), String> {
        if self.plugin.permits(permission) {
            Ok(())
        } else {
            Err(format!("{} does not have the {permission} permission.", self.plugin.name()))
        }
    }
}

// A main page calls directly; a panel calls through the app window it is shown in, which names it.
fn caller(webview: &Webview, panel: Option<String>) -> Result<Caller, String> {
    let app = webview.app_handle();
    let (id, target, page) = match panel {
        Some(id) => {
            browser::trusted(webview)?;
            (id, webview.label().to_string(), false)
        }
        None => {
            let url = webview.url().map_err(|e| e.to_string())?;
            let label = webview.label();
            let id = owner(label)
                .filter(|id| !label.contains('/') && own_url(id, &url))
                .ok_or("Only plugin pages may perform this action.")?;
            (id, session::active(app), true)
        }
    };
    let plugin = plugin(app, &id).ok_or("This plugin is not running.")?;
    Ok(Caller { plugin, target, page })
}

// The app windows and plugin pages with the assistant permission may use the assistant.
pub fn controls(webview: &Webview) -> Result<(), String> {
    if browser::trusted(webview).is_ok() {
        return Ok(());
    }
    caller(webview, None)?.require("assistant")
}

// Tool definitions of the running plugins, for the assistant and the MCP server.
pub fn tools(app: &AppHandle) -> Vec<Value> {
    running(app)
        .iter()
        .flat_map(|plugin| plugin.tools())
        .map(|(name, tool)| {
            json!({
                "name": name,
                "description": tool["description"],
                "input_schema": if tool["input_schema"].is_object() { tool["input_schema"].clone() } else { json!({"type": "object", "properties": {}}) },
            })
        })
        .collect()
}

// Sends a tool call to the plugin that offers the tool and waits for its answer, or None for other tools.
pub async fn call_tool(app: &AppHandle, target: &str, name: &str, input: &Value) -> Option<Result<Value, String>> {
    let (plugin, tool) = running(app)
        .into_iter()
        .find_map(|plugin| plugin.tools().into_iter().find(|(tool, _)| tool == name).map(|(_, tool)| (plugin, tool)))?;
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    let call = {
        let state = app.state::<Plugins>();
        let mut state = state.0.lock().unwrap();
        state.next += 1;
        let call = state.next;
        state.calls.insert(call, tx);
        call
    };
    let event = json!({"type": "tool", "call": call, "name": tool["name"], "input": input, "window": target});
    emit_page(app, &plugin.id, &event);
    Some(rx.recv().await.unwrap_or(Err(format!("{} did not answer.", plugin.name()))))
}

// Serves the files of running plugins on the plugin scheme, limited to the hosts each plugin lists.
pub fn serve(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let path = Path::new(path.trim_start_matches('/'));
    // Hidden files are not covered by the signature.
    let inside = path.components().all(|part| {
        matches!(part, Component::Normal(name) if !name.to_string_lossy().starts_with('.'))
    });
    let plugin = path
        .components()
        .next()
        .and_then(|id| plugin(app, &id.as_os_str().to_string_lossy()))
        .filter(|_| inside);
    let body = plugin.as_ref().and_then(|_| fs::read(dir(app).ok()?.join(path)).ok());
    let kind = match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "html" => "text/html",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    };
    // Network access is blocked during exams.
    let hosts: Vec<&str> = match &plugin {
        Some(plugin) if !exam::active(app) => {
            plugin.manifest["network"].as_array().into_iter().flatten().filter_map(Value::as_str).collect()
        }
        _ => Vec::new(),
    };
    let csp = format!(
        "default-src 'self' {SOURCES} data: blob:; script-src 'self' {SOURCES} 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval'; \
         style-src 'self' {SOURCES} 'unsafe-inline'; img-src * data: blob:; connect-src 'self' {SOURCES} {}",
        hosts.join(" ")
    );
    let (status, body) = match body {
        Some(body) => (200, body),
        None => (404, Vec::new()),
    };
    Response::builder()
        .status(status)
        .header("Content-Type", kind)
        .header("Content-Security-Policy", csp)
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap()
}

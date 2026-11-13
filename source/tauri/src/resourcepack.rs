use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use tauri::http::Response;
use tauri::{AppHandle, Manager, Webview, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::{browser, exam};
use crate::session;

#[cfg(windows)]
const URL: &str = "http://pack.localhost/";
#[cfg(not(windows))]
const URL: &str = "pack://localhost/";
#[cfg(windows)]
const LOCAL_SOURCES: &str = "'self' http://tauri.localhost http://ipc.localhost http://pack.localhost http://plugin.localhost";
#[cfg(not(windows))]
const LOCAL_SOURCES: &str = "'self' tauri: ipc: pack: plugin:";
// The Graspable Math tool in Notes loads from its site, and its styles use Google Fonts.
const GRASPABLE_MATH: &str = "https://graspablemath.com https://fonts.googleapis.com https://fonts.gstatic.com";

// Themes that come with the app and need no install, by usefulness.
// The first is listed next to the default theme, the rest under More themes.
const BUILTIN_THEMES: &[&str] = &[
    include_str!("../themes/dark/theme.json"),
    include_str!("../themes/catppuccin-mocha/theme.json"),
    include_str!("../themes/dracula/theme.json"),
    include_str!("../themes/tokyo-night/theme.json"),
    include_str!("../themes/one-dark/theme.json"),
    include_str!("../themes/nord/theme.json"),
    include_str!("../themes/gruvbox-dark/theme.json"),
    include_str!("../themes/rose-pine/theme.json"),
    include_str!("../themes/catppuccin-macchiato/theme.json"),
    include_str!("../themes/catppuccin-frappe/theme.json"),
    include_str!("../themes/solarized-dark/theme.json"),
    include_str!("../themes/catppuccin-latte/theme.json"),
    include_str!("../themes/solarized-light/theme.json"),
];

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

// Resource packs, themes and plugins are installed the same way; packs and themes are also activated alike.
#[derive(Clone, Copy)]
pub enum Kind {
    Pack,
    Theme,
    Plugin,
}

impl Kind {
    fn parse(name: &str) -> Result<Kind, String> {
        match name {
            "pack" => Ok(Kind::Pack),
            "theme" => Ok(Kind::Theme),
            "plugin" => Ok(Kind::Plugin),
            _ => Err(format!("Unknown kind: {name}")),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Pack => "pack",
            Kind::Theme => "theme",
            Kind::Plugin => "plugin",
        }
    }

    // Each kind is shared as a zip archive with its own extension.
    fn extension(self) -> &'static str {
        match self {
            Kind::Pack => "lcpack",
            Kind::Theme => "lctheme",
            Kind::Plugin => "lcplugin",
        }
    }

    pub fn from_path(path: &Path) -> Option<Kind> {
        let extension = path.extension()?.to_str()?.to_lowercase();
        [Kind::Pack, Kind::Theme, Kind::Plugin].into_iter().find(|kind| kind.extension() == extension)
    }

    fn folder(self) -> &'static str {
        match self {
            Kind::Pack => "resourcepacks",
            Kind::Theme => "themes",
            Kind::Plugin => "plugins",
        }
    }

    fn manifest(self) -> &'static str {
        match self {
            Kind::Pack => "pack.json",
            Kind::Theme => "theme.json",
            Kind::Plugin => "plugin.json",
        }
    }

    fn setting(self) -> &'static str {
        match self {
            Kind::Pack => "resourcePack",
            Kind::Theme => "theme",
            // Plugins are enabled through plugins::plugin_enable instead.
            Kind::Plugin => "plugin",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Pack => "Resource pack",
            Kind::Theme => "Theme",
            Kind::Plugin => "Plugin",
        }
    }
}

pub fn kind_dir(app: &AppHandle, kind: Kind) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join(kind.folder()))
}

pub fn settings(app: &AppHandle) -> Value {
    config_dir(app)
        .and_then(|dir| fs::read_to_string(dir.join("settings.json")).map_err(|e| e.to_string()))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({}))
}

pub fn save_settings(app: &AppHandle, settings: &Value) -> Result<(), String> {
    let dir = config_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join("settings.json"), settings.to_string()).map_err(|e| e.to_string())
}

fn active(app: &AppHandle, kind: Kind) -> Option<String> {
    settings(app)[kind.setting()].as_str().map(String::from)
}

fn manifest(dir: &Path, kind: Kind) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(dir.join(kind.manifest())).ok()?).ok()
}

fn active_dir(app: &AppHandle, kind: Kind) -> Option<PathBuf> {
    let dir = kind_dir(app, kind).ok()?.join(active(app, kind)?);
    dir.join(kind.manifest()).is_file().then_some(dir)
}

fn builtins(kind: Kind) -> Vec<Value> {
    match kind {
        Kind::Pack | Kind::Plugin => Vec::new(),
        Kind::Theme => BUILTIN_THEMES.iter().map(|text| serde_json::from_str(text).unwrap()).collect(),
    }
}

// An installed item overrides a built-in one with the same id.
fn active_manifest(app: &AppHandle, kind: Kind) -> Value {
    active_dir(app, kind)
        .and_then(|dir| manifest(&dir, kind))
        .or_else(|| {
            let id = active(app, kind)?;
            builtins(kind).into_iter().find(|item| item["id"] == id.as_str())
        })
        .unwrap_or(Value::Null)
}

fn dir(app: &AppHandle) -> Option<PathBuf> {
    active_dir(app, Kind::Pack)
}

// The page loads init.js from the pack scheme on every load, so a reload picks up a newly selected pack.
pub fn loader() -> String {
    format!("window.locusPackBase = {};", json!(URL))
}

// The active theme as the page applies it: accent colors, background and foreground, and stylesheet.
fn theme(app: &AppHandle) -> Value {
    let pack = dir(app).as_deref().and_then(|dir| manifest(dir, Kind::Pack)).unwrap_or(Value::Null);
    let theme_dir = active_dir(app, Kind::Theme);
    let theme = active_manifest(app, Kind::Theme);
    // theme accents win over pack accents
    let mut palette = colors(&pack);
    palette.extend(colors(&theme));
    let base = match (hex(&theme["background"]), hex(&theme["foreground"])) {
        (Some(background), Some(foreground)) => json!({ "background": background, "foreground": foreground }),
        _ => Value::Null,
    };
    let css = theme_dir.is_some_and(|dir| dir.join("theme.css").is_file()).then(|| format!("{URL}theme/theme.css"));
    json!({ "colors": palette, "theme": base, "css": css })
}

// The background and foreground of the active theme, null for the default one.
pub fn base_theme(app: &AppHandle) -> Value {
    theme(app)["theme"].clone()
}

// Icons, English strings and URLs are needed synchronously at startup, so they are inlined.
fn script(app: &AppHandle) -> String {
    let dir = dir(app);
    let pack = dir.as_deref().and_then(|dir| manifest(dir, Kind::Pack)).unwrap_or(Value::Null);
    let urls = pack["urls"].clone();
    let theme = theme(app);
    let mut script = format!(
        "window.locusCsp = {};\nwindow.resourcePackColors = {};\n",
        json!(csp(&urls)),
        theme["colors"]
    );
    if !theme["theme"].is_null() {
        script += &format!("window.locusTheme = {};\n", theme["theme"]);
    }
    if !theme["css"].is_null() {
        script += &format!("window.locusThemeCss = {};\n", theme["css"]);
    }
    if icon(app).is_some() {
        script += &format!("window.resourcePackIcon = {};\n", json!(format!("{URL}icon")));
    }
    if let Some(dir) = dir {
        let assets = dir.join("assets");
        let mut icons = Map::new();
        collect_svgs(&assets, &assets, &mut icons);
        script += &format!(
            "window.resourcePackUrl = {};\nwindow.resourcePackUrls = {};\nwindow.resourcePackIcons = {};\nwindow.resourcePackLanguages = {};\nwindow.resourcePackKeys = {{}};\n{}",
            json!(URL),
            urls,
            Value::Object(icons),
            json!(languages(&dir)),
            language(&dir, "en")
        );
    }
    script
}

// The app page may only load its own files and what the active pack links to.
fn csp(urls: &Value) -> String {
    let hosts: Vec<String> = urls
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(_, url)| tauri::Url::parse(url.as_str()?).ok())
        .map(|url| url.origin().ascii_serialization())
        .collect();
    // The dev servers for GWT and Vite run on localhost.
    let dev = if tauri::is_dev() { " http://localhost:* ws://localhost:*" } else { "" };
    let sources = format!("{LOCAL_SOURCES} data: blob:{dev} {GRASPABLE_MATH} {}", hosts.join(" "));
    format!(
        "default-src {sources}; script-src {sources} 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval'; style-src {sources} 'unsafe-inline'"
    )
}

pub fn serve(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let path = path.trim_start_matches('/');
    let file = if path == "init.js" {
        Some(("text/javascript", (crate::performance::script(app) + &crate::session::script(app) + &script(app)).into_bytes()))
    } else if path == "icon" {
        icon(app).and_then(|icon| {
            let kind = if icon.extension().is_some_and(|ext| ext == "svg") { "image/svg+xml" } else { "image/png" };
            fs::read(icon).ok().map(|body| (kind, body))
        })
    } else if path == "theme/theme.css" {
        active_dir(app, Kind::Theme)
            .and_then(|dir| fs::read(dir.join("theme.css")).ok())
            .map(|body| ("text/css", body))
    } else {
        dir(app).and_then(|dir| {
            if let Some(tag) = path.strip_prefix("lang/").and_then(|p| p.strip_suffix(".js")) {
                Some(("text/javascript", language(&dir, tag).into_bytes()))
            } else if path == "theme.css" {
                fs::read(dir.join(path)).ok().map(|body| ("text/css", body))
            } else {
                None
            }
        })
    };
    let (status, kind, body) = match file {
        Some((kind, body)) => (200, kind, body),
        None => (404, "text/plain", Vec::new()),
    };
    Response::builder()
        .status(status)
        .header("Content-Type", kind)
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap()
}

#[tauri::command]
pub fn library_list(app: AppHandle, webview: Webview, kind: String) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let kind = Kind::parse(&kind)?;
    let entry = |id: &str, item: &Value, builtin: bool, more: bool| {
        json!({
            "id": id,
            "name": item["name"],
            "version": item["version"],
            "author": item["author"],
            "description": item["description"],
            "license": item["license"],
            "builtin": builtin,
            "more": more,
        })
    };
    let mut items = Vec::new();
    for (id, path) in installed(&app, kind) {
        if let Some(item) = manifest(&path, kind) {
            items.push(entry(&id, &item, false, false));
        }
    }
    items.sort_by_key(|item| item["name"].as_str().unwrap_or_default().to_lowercase());
    for (index, item) in builtins(kind).iter().enumerate() {
        let id = item["id"].as_str().unwrap_or_default();
        if !items.iter().any(|installed| installed["id"] == id) {
            items.push(entry(id, item, true, index > 0));
        }
    }
    Ok(json!({ "items": items, "active": active(&app, kind) }))
}

// Installed items by id; hidden folders hold an item that is being added.
pub fn installed(app: &AppHandle, kind: Kind) -> Vec<(String, PathBuf)> {
    let Ok(entries) = kind_dir(app, kind).and_then(|dir| fs::read_dir(dir).map_err(|e| e.to_string())) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(kind.manifest()).is_file())
        .filter_map(|path| {
            let id = path.file_name()?.to_string_lossy().into_owned();
            (!id.starts_with('.')).then_some((id, path))
        })
        .collect()
}

// What the details modal shows: the manifest's description and links, and for plugins what they may do.
fn details(app: &AppHandle, kind: Kind, id: &str, item: &Value, dir: Option<&Path>) -> Value {
    let links: Map<String, Value> = item["links"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, url)| url.as_str().is_some_and(|url| url.starts_with("https://") || url.starts_with("http://")))
        .map(|(name, url)| (name.clone(), url.clone()))
        .collect();
    let mut details = json!({
        "kind": kind.name(),
        "id": id,
        "name": item["name"],
        "version": item["version"],
        "author": item["author"],
        "description": item["description"],
        "license": item["license"],
        "links": links,
        "builtin": dir.is_none(),
        "installed": installed(app, kind).iter().any(|(installed, _)| installed == id),
        "active": active(app, kind).as_deref() == Some(id),
    });
    if let (Kind::Plugin, Some(dir)) = (kind, dir) {
        details["plugin"] = crate::plugins::details(id, item, dir);
        details["plugin"]["settings"] = item["settings"].clone();
        details["plugin"]["values"] = crate::plugins::api::values(app, id, item);
    }
    details
}

#[tauri::command]
pub fn library_details(app: AppHandle, webview: Webview, kind: String, id: String) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let kind = Kind::parse(&kind)?;
    let dir = kind_dir(&app, kind)?.join(valid_id(&id)?);
    if let Some(item) = manifest(&dir, kind) {
        return Ok(details(&app, kind, &id, &item, Some(&dir)));
    }
    let item = builtins(kind).into_iter().find(|item| item["id"] == id.as_str()).ok_or("Unknown item.")?;
    Ok(details(&app, kind, &id, &item, None))
}

// Opens a link from the details modal in the browser.
#[tauri::command]
pub fn library_link(app: AppHandle, webview: Webview, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    browser::trusted(&webview)?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only web links can be opened.".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_activate(app: AppHandle, webview: Webview, kind: String, id: Option<String>) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let kind = Kind::parse(&kind)?;
    let mut settings = settings(&app);
    settings[kind.setting()] = json!(id);
    save_settings(&app, &settings)?;
    branding(&app).map_err(|e| e.to_string())?;
    let theme = theme(&app);
    if matches!(kind, Kind::Theme) {
        session::refresh_others(&app, &webview, &theme);
        crate::plugins::theme_changed(&app, &theme["theme"]);
    } else {
        session::refresh_others(&app, &webview, &Value::Null);
    }
    Ok(theme)
}

#[tauri::command]
pub fn library_remove(app: AppHandle, webview: Webview, kind: String, id: String) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let parsed = Kind::parse(&kind)?;
    if active(&app, parsed).as_deref() == Some(id.as_str()) {
        library_activate(app.clone(), webview, kind, None)?;
    }
    fs::remove_dir_all(kind_dir(&app, parsed)?.join(valid_id(&id)?)).map_err(|e| e.to_string())?;
    if matches!(parsed, Kind::Plugin) {
        crate::plugins::removed(&app, &id);
    }
    Ok(theme(&app))
}

// A file to add is extracted into a hidden folder first, so its details can be shown before it is added.
fn pending(app: &AppHandle, kind: Kind) -> Result<PathBuf, String> {
    Ok(kind_dir(app, kind)?.join(".pending"))
}

pub fn stage(app: &AppHandle, file: &Path) -> Result<Value, String> {
    let kind = Kind::from_path(file).ok_or("Locus cannot add this file.")?;
    if exam::active(app) {
        return Err("Nothing can be added during an exam.".into());
    }
    let manifest_name = kind.manifest();
    let items = kind_dir(app, kind)?;
    let extract = items.join(".extract");
    let pending = pending(app, kind)?;
    let _ = fs::remove_dir_all(&extract);
    let _ = fs::remove_dir_all(&pending);
    fs::create_dir_all(&extract).map_err(|e| e.to_string())?;
    let result = (|| {
        let archive = fs::File::open(file).map_err(|e| e.to_string())?;
        zip::ZipArchive::new(archive)
            .and_then(|mut archive| archive.extract(&extract))
            .map_err(|_| format!("This is not a valid {} file.", kind.label().to_lowercase()))?;
        // Archives often wrap the contents in a folder.
        let root = if extract.join(manifest_name).is_file() {
            extract.clone()
        } else {
            fs::read_dir(&extract)
                .map_err(|e| e.to_string())?
                .flatten()
                .map(|entry| entry.path())
                .find(|path| path.join(manifest_name).is_file())
                .ok_or(format!("The file has no {manifest_name}."))?
        };
        let item = manifest(&root, kind).ok_or(format!("{manifest_name} is not valid JSON."))?;
        let id = valid_id(item["id"].as_str().ok_or(format!("{manifest_name} has no id."))?)?.to_string();
        fs::rename(&root, &pending).map_err(|e| e.to_string())?;
        Ok(details(app, kind, &id, &item, Some(&pending)))
    })();
    let _ = fs::remove_dir_all(&extract);
    result
}

#[tauri::command]
pub async fn library_pick(app: AppHandle, webview: Webview, kind: String) -> Result<Option<Value>, String> {
    browser::trusted(&webview)?;
    let kind = Kind::parse(&kind)?;
    let dialog = webview.dialog().file();
    #[cfg(desktop)]
    let dialog = dialog.set_parent(&webview.window());
    let Some(picked) = dialog.add_filter(kind.label(), &[kind.extension()]).blocking_pick_file() else {
        return Ok(None);
    };
    stage(&app, &picked.into_path().map_err(|e| e.to_string())?).map(Some)
}

// Adds the item whose details were shown, replacing an installed one with the same id.
#[tauri::command]
pub fn library_add(app: AppHandle, webview: Webview, kind: String) -> Result<(), String> {
    browser::trusted(&webview)?;
    let kind = Kind::parse(&kind)?;
    let pending = pending(&app, kind)?;
    let item = manifest(&pending, kind).ok_or("Nothing is waiting to be added.")?;
    let id = valid_id(item["id"].as_str().unwrap_or_default())?.to_string();
    let target = kind_dir(&app, kind)?.join(&id);
    if matches!(kind, Kind::Plugin) {
        crate::plugins::removed(&app, &id);
    }
    let _ = fs::remove_dir_all(&target);
    fs::rename(&pending, &target).map_err(|e| e.to_string())?;
    if matches!(kind, Kind::Plugin) {
        crate::plugins::added(&app, &id);
    }
    Ok(())
}

#[tauri::command]
pub fn library_cancel(app: AppHandle, webview: Webview, kind: String) -> Result<(), String> {
    browser::trusted(&webview)?;
    let _ = fs::remove_dir_all(pending(&app, Kind::parse(&kind)?)?);
    Ok(())
}

// The app name with the pack's product name in place of "Locus".
#[cfg(desktop)]
fn product_name(app: &AppHandle, mode: session::Mode) -> String {
    let pack = dir(app).as_deref().and_then(|dir| manifest(dir, Kind::Pack)).unwrap_or(Value::Null);
    match pack["productName"].as_str() {
        Some(brand) => mode.title().replacen("Locus", brand, 1),
        None => mode.title().into(),
    }
}

fn icon(app: &AppHandle) -> Option<PathBuf> {
    let dir = dir(app)?;
    let icon = dir.join(manifest(&dir, Kind::Pack)?["icon"].as_str()?).canonicalize().ok()?;
    // the icon path comes from the pack, so it must not point outside of it
    icon.starts_with(dir.canonicalize().ok()?).then_some(icon)
}

// The title bar color: the theme's background, else the pack's titleBar, else the color of the app's top bar.
#[cfg(desktop)]
fn title_bar(app: &AppHandle, mode: session::Mode) -> (u8, u8, u8) {
    let theme = active_manifest(app, Kind::Theme);
    let pack = dir(app).and_then(|dir| manifest(&dir, Kind::Pack)).unwrap_or(Value::Null);
    let default = if mode == session::Mode::Classic { "#F8F8F8" } else { "#FFFFFF" };
    let color = hex(&theme["background"]).or(hex(&pack["titleBar"])).unwrap_or(default);
    let rgb = u32::from_str_radix(&color[1..], 16).unwrap_or_default();
    ((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

// Window title, title bar color and taskbar icon of one window.
#[cfg_attr(not(desktop), allow(unused_variables))]
pub fn window_branding(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(desktop)]
    {
        let mode = session::mode(app, window.label());
        let (r, g, b) = title_bar(app, mode);
        let dark = r as u32 * 299 + g as u32 * 587 + b as u32 * 114 < 128_000;
        window.set_theme(Some(if dark { tauri::Theme::Dark } else { tauri::Theme::Light }))?;
        // The transparent title bar on macOS shows this color.
        window.set_background_color(Some(tauri::window::Color(r, g, b, 255)))?;
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR};
            // COLORREF is 0x00BBGGRR.
            let color: u32 = r as u32 | (g as u32) << 8 | (b as u32) << 16;
            unsafe {
                DwmSetWindowAttribute(hwnd.0 as _, DWMWA_CAPTION_COLOR as u32, &color as *const u32 as _, 4);
            }
        }
        window.set_title(&product_name(app, mode))?;
    }
    #[cfg(all(desktop, not(target_os = "macos")))]
    match icon(app).and_then(|path| tauri::image::Image::from_path(path).ok()) {
        Some(image) => window.set_icon(image)?,
        None => {
            if let Some(image) = app.default_window_icon() {
                window.set_icon(image.clone())?;
            }
        }
    }
    Ok(())
}

// Branding of every window and the dock icon of the active pack.
pub fn branding(app: &AppHandle) -> tauri::Result<()> {
    for label in session::labels(app) {
        if let Some(window) = app.get_webview_window(&label) {
            window_branding(app, &window)?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        let icon = icon(app);
        app.run_on_main_thread(move || {
            use objc2::{AllocAnyThread, MainThreadMarker};
            use objc2_app_kit::{NSApplication, NSImage};
            use objc2_foundation::NSString;
            let image = icon.and_then(|path| {
                NSImage::initWithContentsOfFile(NSImage::alloc(), &NSString::from_str(&path.to_string_lossy()))
            });
            let mtm = MainThreadMarker::new().expect("main thread");
            unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(image.as_deref()) };
        })?;
    }
    Ok(())
}

fn valid_id(id: &str) -> Result<&str, String> {
    let valid = !id.is_empty()
        && !id.starts_with('.')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    valid.then_some(id).ok_or(format!("Invalid pack id: {id}"))
}

fn collect_svgs(root: &Path, dir: &Path, icons: &mut Map<String, Value>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            collect_svgs(root, &path, icons);
        } else if path.extension().is_some_and(|ext| ext == "svg") {
            if let (Ok(relative), Ok(svg)) = (path.strip_prefix(root), fs::read_to_string(&path)) {
                let key: Vec<_> = relative.iter().map(|part| part.to_string_lossy()).collect();
                icons.insert(key.join("/"), svg.into());
            }
        }
    }
}

fn language(dir: &Path, tag: &str) -> String {
    // Upstream file names use Java's old codes for Hebrew and Indonesian.
    let suffix = match tag {
        "en" => String::new(),
        "he" => "_iw".into(),
        "id" => "_in".into(),
        _ => format!("_{}", tag.replace('-', "_")),
    };
    let mut sections = Map::new();
    for section in ["menu", "command", "symbols"] {
        let file = dir.join("lang").join(format!("{section}{suffix}.properties"));
        if let Ok(text) = fs::read_to_string(file) {
            sections.insert(section.into(), Value::Object(parse(&text)));
        }
    }
    format!("window.resourcePackKeys[{}] = {};", json!(tag), Value::Object(sections))
}

// Only plain hex colors, since they end up in CSS.
fn hex(color: &Value) -> Option<&str> {
    let color = color.as_str()?;
    let valid = color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    valid.then_some(color)
}

fn colors(manifest: &Value) -> Map<String, Value> {
    manifest["colors"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(name, color)| name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && hex(color).is_some())
        .map(|(name, color)| (name.clone(), color.clone()))
        .collect()
}

// Language tags of the pack's translations, the inverse of the file naming in language().
fn languages(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir.join("lang")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let suffix = name.strip_prefix("menu_")?.strip_suffix(".properties")?.to_string();
            Some(match suffix.as_str() {
                "iw" => "he".into(),
                "in" => "id".into(),
                _ => suffix.replace('_', "-"),
            })
        })
        .collect()
}

fn parse(text: &str) -> Map<String, Value> {
    text.lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty() && !line.starts_with(['#', '!']))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), unescape(value.trim_start()).into()))
        .collect()
}

fn unescape(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

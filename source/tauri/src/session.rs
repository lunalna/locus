// The app windows, each running one of the apps. Their modes are saved so the next launch reopens them.

use std::sync::atomic::Ordering;
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::{browser, exam, performance, resourcepack, Lockdown};

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Classic,
    Suite,
    Graphing,
    Geometry,
    Graphing3D,
    Cas,
    Scientific,
    Notes,
}

const MODES: [(Mode, &str, &str); 8] = [
    (Mode::Classic, "classic", "Locus Classic"),
    (Mode::Suite, "suite", "Locus Suite"),
    (Mode::Graphing, "graphing", "Locus Graphing"),
    (Mode::Geometry, "geometry", "Locus Geometry"),
    (Mode::Graphing3D, "3d", "Locus 3D"),
    (Mode::Cas, "cas", "Locus CAS"),
    (Mode::Scientific, "scientific", "Locus Scientific"),
    (Mode::Notes, "notes", "Locus Whiteboard"),
];

impl Mode {
    fn parse(name: &str) -> Option<Mode> {
        MODES.iter().find(|(_, n, _)| *n == name).map(|(mode, _, _)| *mode)
    }

    fn entry(self) -> &'static (Mode, &'static str, &'static str) {
        MODES.iter().find(|(mode, _, _)| *mode == self).unwrap()
    }

    pub fn name(self) -> &'static str {
        self.entry().1
    }

    pub fn title(self) -> &'static str {
        self.entry().2
    }
}

#[derive(Default)]
pub struct Windows(Mutex<State>);

#[derive(Default)]
struct State {
    // Open windows by label, in the order they were opened.
    open: Vec<(String, Mode)>,
    active: String,
    next: u32,
}

pub fn is_main(label: &str) -> bool {
    label == "main" || label.starts_with("main-")
}

pub fn mode(app: &AppHandle, label: &str) -> Mode {
    let state = app.state::<Windows>();
    let state = state.0.lock().unwrap();
    state.open.iter().find(|(open, _)| open == label).map_or(Mode::Classic, |(_, mode)| *mode)
}

pub fn labels(app: &AppHandle) -> Vec<String> {
    app.state::<Windows>().0.lock().unwrap().open.iter().map(|(label, _)| label.clone()).collect()
}

// The window used last, which the MCP server and the F4 assistant act on.
pub fn active(app: &AppHandle) -> String {
    app.state::<Windows>().0.lock().unwrap().active.clone()
}

pub fn focused(app: &AppHandle, label: &str) {
    app.state::<Windows>().0.lock().unwrap().active = label.into();
    crate::plugins::focused(app, label);
}

fn save(app: &AppHandle) {
    let modes: Vec<_> = app.state::<Windows>().0.lock().unwrap().open.iter().map(|(_, mode)| mode.name()).collect();
    let mut settings = resourcepack::settings(app);
    settings["windows"] = json!(modes);
    let _ = resourcepack::save_settings(app, &settings);
}

// Applies a settings change made in one window to the others: a theme in place, anything else by reloading.
pub fn refresh_others(app: &AppHandle, webview: &Webview, theme: &Value) {
    for label in labels(app).iter().filter(|label| *label != webview.label()) {
        if let Some(view) = app.get_webview(label) {
            let _ = view.eval(format!("window.locusRefresh && window.locusRefresh({theme})"));
        }
    }
}

// Opens the windows of the last session, or the app chosen in the settings. Classic when neither is there.
pub fn restore(app: &AppHandle) -> tauri::Result<()> {
    let settings = resourcepack::settings(app);
    let mut modes: Vec<Mode> = match settings["startup"].as_str().and_then(Mode::parse) {
        Some(mode) => vec![mode],
        None => settings["windows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|mode| Mode::parse(mode.as_str()?))
            .collect(),
    };
    // iPadOS keeps a single window.
    if cfg!(mobile) {
        modes.truncate(1);
    }
    if modes.is_empty() {
        modes.push(Mode::Classic);
    }
    for mode in modes {
        open(app, mode)?;
    }
    Ok(())
}

pub fn confirm_close(app: &AppHandle) -> bool {
    resourcepack::settings(app)["confirm_close"].as_bool().unwrap_or(true)
}

// The last window stays in the saved session, so closing it reopens it on the next launch.
pub fn closing(app: &AppHandle, label: &str) {
    let state = app.state::<Windows>();
    let mut state = state.0.lock().unwrap();
    if state.open.len() > 1 {
        state.open.retain(|(open, _)| open != label);
        drop(state);
        save(app);
    }
}

pub fn closed(app: &AppHandle, label: &str) {
    let state = app.state::<Windows>();
    let mut state = state.0.lock().unwrap();
    state.open.retain(|(open, _)| open != label);
    if state.active == label {
        state.active = state.open.last().map(|(open, _)| open.clone()).unwrap_or_default();
    }
}

pub fn open(app: &AppHandle, mode: Mode) -> tauri::Result<WebviewWindow> {
    let label = {
        let state = app.state::<Windows>();
        let mut state = state.0.lock().unwrap();
        state.next += 1;
        let label = if state.next == 1 { "main".to_string() } else { format!("main-{}", state.next) };
        state.open.push((label.clone(), mode));
        state.active = label.clone();
        label
    };
    save(app);
    let navigation_app = app.clone();
    let popup_app = app.clone();
    let page = format!("{}-desktop.html", mode.name());
    let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(page.into()));
    #[cfg(desktop)]
    let builder = builder.inner_size(1280.0, 800.0);
    // set once here, since changing it later resizes the window
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Transparent);
    #[cfg(target_os = "ios")]
    let builder = builder
        .initialization_script(include_str!("ios.js"));
    #[cfg(all(desktop, not(target_os = "macos")))]
    let builder = builder.initialization_script(include_str!("session.js"));
    let window = builder
        .initialization_script(resourcepack::loader())
        .initialization_script(crate::BRIDGE)
        .initialization_script(crate::page_scripts())
        .initialization_script(include_str!("files.js"))
        .on_navigation(move |url| {
            let allowed = !navigation_app.state::<Lockdown>().0.load(Ordering::SeqCst)
                || browser::local_url(&navigation_app, url)
                || crate::plugins::plugin_url(url);
            if !allowed {
                exam::record(&navigation_app, format!("External window blocked: {url}"));
            }
            allowed
        })
        .on_page_load(|webview, event| {
            if matches!(event.event(), tauri::webview::PageLoadEvent::Started) {
                crate::plugins::unscripted(webview.app_handle(), webview.label());
            }
        })
        .on_new_window(move |url, _| {
            if popup_app.state::<Lockdown>().0.load(Ordering::SeqCst) {
                exam::record(&popup_app, format!("External window blocked: {url}"));
                tauri::webview::NewWindowResponse::Deny
            } else {
                tauri::webview::NewWindowResponse::Allow
            }
        })
        .build()?;
    if let Some(webview) = app.get_webview(&label) {
        performance::apply(&webview);
    }
    #[cfg(target_os = "macos")]
    crate::keys::install(&window);
    #[cfg(target_os = "linux")]
    if let Some(native) = app.get_window(&label) {
        crate::shortcuts::configure(&native);
    }
    resourcepack::window_branding(app, &window)?;
    Ok(window)
}

// Opens a window for an app. iPadOS has one window, so it switches that window instead.
pub fn open_app(app: &AppHandle, name: &str) -> Result<(), String> {
    let mode = Mode::parse(name).ok_or("Unknown app.")?;
    if app.state::<Lockdown>().0.load(Ordering::SeqCst) {
        return Err("New windows cannot be opened during an exam.".into());
    }
    #[cfg(mobile)]
    return switch(app, "main", mode);
    #[cfg(desktop)]
    open(app, mode).map(|_| ()).map_err(|e| e.to_string())
}

// Loads another app in a window. The page keeps the construction in sessionStorage for the new app.
fn switch(app: &AppHandle, label: &str, mode: Mode) -> Result<(), String> {
    let window = app.get_webview_window(label).ok_or("Locus window missing.")?;
    {
        let state = app.state::<Windows>();
        let mut state = state.0.lock().unwrap();
        if let Some(entry) = state.open.iter_mut().find(|(open, _)| open == label) {
            entry.1 = mode;
        }
    }
    save(app);
    let mut url = window.url().map_err(|e| e.to_string())?;
    url.set_path(&format!("/{}-desktop.html", mode.name()));
    url.set_fragment(None);
    window.navigate(url).map_err(|e| e.to_string())?;
    resourcepack::window_branding(app, &window).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn window_switch(app: AppHandle, webview: Webview, name: String) -> Result<(), String> {
    browser::trusted(&webview)?;
    let mode = Mode::parse(&name).ok_or("Unknown app.")?;
    if app.state::<Lockdown>().0.load(Ordering::SeqCst) {
        return Err("Apps cannot be switched during an exam.".into());
    }
    switch(&app, webview.label(), mode)
}

#[tauri::command]
pub async fn window_open(app: AppHandle, webview: Webview, name: String) -> Result<(), String> {
    browser::trusted(&webview)?;
    open_app(&app, &name)
}

#[tauri::command]
pub fn general_settings(app: AppHandle, webview: Webview) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let settings = resourcepack::settings(&app);
    Ok(json!({
        "startup": settings["startup"].as_str().filter(|name| Mode::parse(name).is_some()).unwrap_or("last"),
        "confirm_close": confirm_close(&app),
    }))
}

#[tauri::command]
pub fn general_set(app: AppHandle, webview: Webview, name: String, value: Value) -> Result<(), String> {
    browser::trusted(&webview)?;
    let valid = match name.as_str() {
        "startup" => value.as_str().is_some_and(|name| name == "last" || Mode::parse(name).is_some()),
        "confirm_close" => value.is_boolean(),
        _ => false,
    };
    if !valid {
        return Err("Invalid setting.".into());
    }
    let mut settings = resourcepack::settings(&app);
    settings[name] = value;
    resourcepack::save_settings(&app, &settings)
}

// Called by the page once the user has saved or discarded the open file.
#[tauri::command]
pub fn window_close(app: AppHandle, webview: Webview) -> Result<(), String> {
    browser::trusted(&webview)?;
    let window = webview.window();
    closing(&app, window.label());
    window.destroy().map_err(|e| e.to_string())
}

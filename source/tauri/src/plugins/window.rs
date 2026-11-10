// The native side of plugins: each plugin page runs in a window that stays hidden until the plugin
// shows it, floating above other apps, with the websites it opens as views stacked inside it.

use serde_json::json;
use tauri::webview::NewWindowResponse;
#[cfg(desktop)]
use tauri::webview::WebviewBuilder;
#[cfg(desktop)]
use tauri::window::WindowBuilder;
use tauri::{AppHandle, Manager, Url, Webview, WebviewUrl};
#[cfg(desktop)]
use tauri::{LogicalPosition, LogicalSize};

use super::{emit, own_url, publish, with, Plugin};
use crate::{browser, exam};

// Pages may show their own files, and a blank page once stopped on iPadOS.
fn page_navigation(id: &str) -> impl Fn(&Url) -> bool + Send + 'static {
    let id = id.to_string();
    move |url| url.as_str() == "about:blank" || own_url(&id, url)
}

// Loads the plugin page into a hidden window, or loads it again when the window is still there.
pub fn start(app: &AppHandle, plugin: &Plugin) -> Result<(), String> {
    let label = plugin.label();
    if let Some(page) = app.get_webview(&label) {
        return page.navigate(plugin.main_url()).map_err(|e| e.to_string());
    }
    #[cfg(mobile)]
    tauri::WebviewWindowBuilder::new(app, &label, plugin.url())
        .visible(false)
        .transparent(true)
        // wry ignores transparent on iOS and paints the system background unless given a color.
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .initialization_script(include_str!("../ios.js"))
        .on_navigation(page_navigation(&plugin.id))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()
        .map_err(|e| e.to_string())?;
    #[cfg(desktop)]
    {
        let (width, height) = (plugin.size("width", 920.0), plugin.size("height", 640.0));
        let window = WindowBuilder::new(app, &label)
            .title(plugin.manifest["window"]["title"].as_str().unwrap_or(plugin.name()))
            .theme(Some(tauri::Theme::Light))
            .inner_size(width, height)
            .min_inner_size(plugin.size("minWidth", 200.0), plugin.size("minHeight", 120.0))
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(true)
            .visible(false)
            .center()
            .build()
            .map_err(|e| e.to_string())?;
        let page = WebviewBuilder::new(&label, plugin.url())
            .transparent(true)
            .auto_resize()
            .on_navigation(page_navigation(&plugin.id))
            .on_new_window(|_, _| NewWindowResponse::Deny);
        window
            .add_child(page, LogicalPosition::new(0.0, 0.0), LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        crate::shortcuts::configure(&window);
    }
    Ok(())
}

// Closes the views and the window. iPadOS never reports panel windows as destroyed, so their webviews
// stay registered: the page is left blank instead.
pub fn stop(app: &AppHandle, label: &str, views: &[String]) -> Result<(), String> {
    for view in views {
        close_view(app, view)?;
    }
    #[cfg(desktop)]
    if let Some(window) = app.get_window(label) {
        window.destroy().map_err(|e| e.to_string())?;
    }
    #[cfg(mobile)]
    if let Some(page) = app.get_webview(label) {
        crate::ios::set_visible(app, label, false);
        page.navigate(Url::parse("about:blank").unwrap()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn reset(app: &AppHandle, plugin: &Plugin, views: &[String]) -> Result<(), String> {
    for view in views {
        close_view(app, view)?;
    }
    hide(app, &plugin.label())?;
    alpha(app, &plugin.label(), 1.0);
    start(app, plugin)
}

// iPadOS has one window, so the plugin window becomes a panel above the app the first time it is needed.
#[cfg(mobile)]
fn panel(app: &AppHandle, plugin: &Plugin) -> Result<(), String> {
    let label = plugin.label();
    if crate::ios::has_panel(&label) {
        return Ok(());
    }
    let [top, ..] = plugin.inset();
    crate::ios::create_panel(
        app,
        &label,
        plugin.size("width", 920.0),
        plugin.size("height", 640.0),
        plugin.size("minWidth", 200.0),
        plugin.size("minHeight", 120.0),
        top,
    );
    let page = app.get_webview(&label).ok_or("Plugin page missing.")?;
    crate::ios::adopt(&page, &label, 0.0, 0.0, 0.0, 0.0);
    Ok(())
}

pub fn open(app: &AppHandle, plugin: &Plugin, value: f64) -> Result<(), String> {
    let label = plugin.label();
    #[cfg(mobile)]
    {
        panel(app, plugin)?;
        crate::ios::set_alpha(app, &label, value);
        crate::ios::set_visible(app, &label, true);
    }
    #[cfg(desktop)]
    {
        let window = app.get_window(&label).ok_or("Plugin window missing.")?;
        crate::shortcuts::opacity(&window, value);
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn visible(app: &AppHandle, label: &str) -> bool {
    #[cfg(desktop)]
    return app
        .get_window(label)
        .is_some_and(|window| window.is_visible().unwrap_or(false));
    #[cfg(mobile)]
    return app.get_webview(label).is_some() && crate::ios::visible(label);
}

pub fn hide(app: &AppHandle, label: &str) -> Result<(), String> {
    #[cfg(desktop)]
    if let Some(window) = app.get_window(label) {
        window.hide().map_err(|e| e.to_string())?;
    }
    #[cfg(mobile)]
    crate::ios::set_visible(app, label, false);
    Ok(())
}

pub fn alpha(app: &AppHandle, label: &str, value: f64) {
    #[cfg(desktop)]
    if let Some(window) = app.get_window(label) {
        crate::shortcuts::opacity(&window, value);
    }
    #[cfg(mobile)]
    crate::ios::set_alpha(app, label, value);
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
#[cfg(windows)]
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
#[cfg(not(any(windows, target_os = "macos", target_os = "ios")))]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";

fn view_filter(app: &AppHandle, plugin: &str) -> String {
    with(app, plugin, |instance| instance.filter.clone()).unwrap_or_default()
}

// Desktop views are child webviews and iPadOS views are webview windows, with the same settings.
macro_rules! view_settings {
    ($builder:expr, $app:expr, $plugin:expr) => {{
        let (nav_app, popup_app, load_app, title_app, download_app) =
            ($app.clone(), $app.clone(), $app.clone(), $app.clone(), $app.clone());
        let (nav_id, popup_id, load_id, title_id) =
            ($plugin.to_string(), $plugin.to_string(), $plugin.to_string(), $plugin.to_string());
        $builder
            .user_agent(USER_AGENT)
            .transparent(true)
            .devtools(false)
            .on_navigation(move |url| {
                let allowed = browser::navigation(&nav_app, url);
                if !allowed && url.as_str() != "about:blank" {
                    emit(&nav_app, &nav_id, &json!({"type": "blocked", "url": url.as_str()}));
                }
                allowed
            })
            // Popups open as a new view of the same plugin.
            .on_new_window(move |url, _| {
                let (app, plugin) = (popup_app.clone(), popup_id.clone());
                std::thread::spawn(move || {
                    if let Some(info) = super::plugin(&app, &plugin) {
                        let _ = create_view(&app, &info, url).and_then(|id| show(&app, &plugin, Some(&id)));
                    }
                });
                NewWindowResponse::Deny
            })
            .on_document_title_changed(move |view, title| {
                with(&title_app, &title_id, |instance| {
                    if let Some(entry) = instance.views.iter_mut().find(|entry| entry.id == view.label()) {
                        entry.title = title;
                    }
                });
                publish(&title_app, &title_id);
            })
            .on_page_load(move |view, event| {
                // Started fires once the new document commits, so the filter shows before the page paints.
                if matches!(event.event(), tauri::webview::PageLoadEvent::Started) {
                    let _ = view.eval(filter_script(&view_filter(&load_app, &load_id)));
                    return;
                }
                if matches!(event.event(), tauri::webview::PageLoadEvent::Finished) {
                    // Navigation callbacks also include subframes; the native URL belongs to the top-level page.
                    let Ok(url) = view.url() else {
                        return;
                    };
                    if &url != event.url() {
                        return;
                    }
                    exam::record(&load_app, format!("Page loaded: {url}"));
                    let _ = view.eval(filter_script(&view_filter(&load_app, &load_id)));
                    publish(&load_app, &load_id);
                }
            })
            .on_download(move |_, event| {
                if let tauri::webview::DownloadEvent::Requested { url, .. } = event {
                    exam::record(&download_app, format!("Download blocked: {url}"));
                }
                false
            })
    }};
}

pub fn create_view(app: &AppHandle, info: &Plugin, url: Url) -> Result<String, String> {
    if !browser::navigation(app, &url) {
        return Err("This site is not allowed in this exam.".into());
    }
    let plugin = info.id.as_str();
    let id = {
        let state = app.state::<super::Plugins>();
        let mut state = state.0.lock().unwrap();
        state.next += 1;
        format!("{}/view-{}", info.label(), state.next)
    };
    let opacity = with(app, plugin, |instance| {
        instance.views.push(super::View {
            id: id.clone(),
            title: String::new(),
        });
        instance.opacity
    })
    .ok_or("This plugin is not running.")?;
    let url = WebviewUrl::External(url);
    #[cfg(desktop)]
    {
        let window = app.get_window(&info.label()).ok_or("Plugin window missing.")?;
        let [top, _, _, left] = info.inset();
        let view = window
            .add_child(
                view_settings!(WebviewBuilder::new(&id, url), app, plugin),
                LogicalPosition::new(left, top),
                LogicalSize::new(0.0, 0.0),
            )
            .map_err(|e| e.to_string())?;
        set_content_opacity(&view, opacity)?;
        view.set_size(content_size(&window, info)?).map_err(|e| e.to_string())?;
    }
    #[cfg(mobile)]
    {
        panel(app, info)?;
        view_settings!(
            tauri::WebviewWindowBuilder::new(app, &id, url).visible(false),
            app,
            plugin
        )
        .build()
        .map_err(|e| e.to_string())?;
        let view = app.get_webview(&id).ok_or("Plugin view missing.")?;
        let [top, right, bottom, left] = info.inset();
        crate::ios::adopt(&view, &info.label(), left, top, right, bottom);
        set_content_opacity(&view, opacity)?;
    }
    Ok(id)
}

pub fn close_view(app: &AppHandle, id: &str) -> Result<(), String> {
    if let Some(view) = app.get_webview(id) {
        #[cfg(desktop)]
        view.close().map_err(|e| e.to_string())?;
        // On iPadOS a view is a webview taken into the panel from its own hidden window.
        #[cfg(mobile)]
        crate::ios::remove_view(&view);
    }
    Ok(())
}

// Views share one area, so at most one is shown.
pub fn show(app: &AppHandle, plugin: &str, id: Option<&str>) -> Result<(), String> {
    let views = with(app, plugin, |instance| {
        if id.is_some_and(|id| !instance.views.iter().any(|view| view.id == id)) {
            return Err("Unknown view.".to_string());
        }
        instance.shown = id.map(String::from);
        Ok(instance.views.clone())
    })
    .ok_or("This plugin is not running.")??;
    for view in views {
        if let Some(webview) = app.get_webview(&view.id) {
            set_view_visible(&webview, Some(view.id.as_str()) == id)?;
        }
    }
    publish(app, plugin);
    Ok(())
}

fn set_view_visible(view: &Webview, visible: bool) -> Result<(), String> {
    #[cfg(mobile)]
    crate::ios::set_view_visible(view, visible);
    #[cfg(desktop)]
    if visible { view.show() } else { view.hide() }.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(desktop)]
fn content_size(window: &tauri::Window, plugin: &Plugin) -> Result<LogicalSize<f64>, String> {
    let [top, right, bottom, left] = plugin.inset();
    let size = window
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
    Ok(LogicalSize::new(
        (size.width - left - right).max(1.0),
        (size.height - top - bottom).max(1.0),
    ))
}

#[cfg(desktop)]
pub fn layout(app: &AppHandle, plugin: &Plugin) {
    let Some(window) = app.get_window(&plugin.label()) else {
        return;
    };
    let Ok(size) = content_size(&window, plugin) else {
        return;
    };
    for view in super::views(app, &plugin.id) {
        if let Some(webview) = app.get_webview(&view.id) {
            let _ = webview.set_size(size);
        }
    }
}

// Frosted glass behind the window, so busy content behind it does not compete with the text.
// A strength of 0 turns it off; on macOS lower values fade the blur layer.
pub fn blur(app: &AppHandle, label: &str, strength: f64) -> Result<(), String> {
    #[cfg(mobile)]
    crate::ios::set_blur(app, label, strength);
    #[cfg(desktop)]
    {
        let window = app.get_window(label).ok_or("Plugin window missing.")?;
        window
            .clone()
            .run_on_main_thread(move || {
                #[cfg(target_os = "macos")]
                {
                    use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState};
                    // The tag window-vibrancy gives its blur view.
                    const BLUR_VIEW: isize = 91376254;
                    let blur_view = || {
                        let native: &objc2_app_kit::NSWindow =
                            unsafe { &*window.ns_window().ok()?.cast() };
                        native.contentView()?.viewWithTag(BLUR_VIEW)
                    };
                    if strength == 0.0 {
                        let _ = window_vibrancy::clear_vibrancy(&window);
                    } else if blur_view().is_none() {
                        let _ = window_vibrancy::apply_vibrancy(
                            &window,
                            NSVisualEffectMaterial::Popover,
                            Some(NSVisualEffectState::Active),
                            Some(14.0),
                        );
                    }
                    if let Some(view) = blur_view() {
                        view.setAlphaValue(strength);
                    }
                }
                #[cfg(windows)]
                {
                    let _ = if strength > 0.0 {
                        window_vibrancy::apply_acrylic(&window, None)
                    } else {
                        window_vibrancy::clear_acrylic(&window)
                    };
                }
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// A CSS filter on the websites, such as grayscale(1); empty removes it.
pub fn filter_script(filter: &str) -> String {
    let filter = json!(filter);
    format!(
        "(function apply() {{
            const root = document.documentElement;
            if (!root) return setTimeout(apply);
            if ({filter}) root.style.setProperty('filter', {filter}, 'important');
            else root.style.removeProperty('filter');
        }})()"
    )
}

pub fn set_content_opacity(view: &Webview, opacity: f64) -> Result<(), String> {
    #[cfg(mobile)]
    return {
        crate::ios::set_view_alpha(view, opacity);
        Ok(())
    };
    #[cfg(desktop)]
    view.with_webview(move |native| {
        #[cfg(target_os = "macos")]
        {
            let view: &objc2_app_kit::NSView = unsafe { &*native.inner().cast() };
            view.setAlphaValue(opacity);
        }
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::*;
            native.inner().set_opacity(opacity);
        }
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
                LWA_ALPHA, WS_EX_LAYERED,
            };
            let mut handle = Default::default();
            if native.controller().ParentWindow(&mut handle).is_ok() {
                let handle = handle.0 as _;
                let style = GetWindowLongPtrW(handle, GWL_EXSTYLE);
                SetWindowLongPtrW(handle, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
                SetLayeredWindowAttributes(handle, 0, (opacity * 255.0) as u8, LWA_ALPHA);
            }
        }
    })
    .map_err(|e| e.to_string())
}

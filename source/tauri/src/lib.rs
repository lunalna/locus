use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[cfg(target_os = "macos")]
use tauri::menu::Menu;
use tauri::{AppHandle, Manager, RunEvent, Webview, WindowEvent};

mod assistant;
mod browser;
mod exam;
mod files;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod framerate;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "macos")]
mod keys;
mod mcp;
mod performance;
mod plugins;
mod providers;
mod resourcepack;
mod session;
#[cfg(desktop)]
mod shortcuts;
mod tools;
#[cfg(desktop)]
mod updater;

#[cfg(windows)]
mod windows;

const BRIDGE: &str = r#"
window.locusOpenApp = (name) => window.__TAURI__.core.invoke("window_open", { name });
window.locusSwitchApp = (name) => {
    // Notes always starts empty and its pages do not fit the other apps
    if (ggbApplet.getObjectNumber() > 0 && name !== "notes" && appID !== "notes") sessionStorage.setItem("locusFile", ggbApplet.getBase64());
    window.__TAURI__.core.invoke("window_switch", { name }).catch(() => sessionStorage.removeItem("locusFile"));
};
window.ggbExamMode = (on) => window.__TAURI__.core.invoke("exam_mode", { on });
window.ggbExamLockdown = (on, done) => {
    window.ggbExamLockdownDone = done;
    window.__TAURI__.core.invoke("exam_lockdown", { on });
};
window.ggbExamFocusFromHost = true;
addEventListener("DOMContentLoaded", () => {
    const style = document.createElement("style");
    style.textContent = "* { overscroll-behavior: none; }";
    document.head.append(style);
});
"#;

// Loads the Locus settings modal, the AI assistant view and the plugin panels and command list, which the app
// opens through window.locusSettings and window.locusPlugins and fills through window.locusAssistant.
fn page_scripts() -> String {
    ["settings", "assistant", "plugins"]
        .map(|name| {
            let src = if tauri::is_dev() {
                format!("/src/{name}/main.svelte.js")
            } else {
                format!("/browser-assets/{name}.js")
            };
            format!(
                "addEventListener('DOMContentLoaded', () => {{ const script = document.createElement('script'); script.type = 'module'; script.src = {src:?}; document.head.append(script); }});"
            )
        })
        .join("\n")
        // Lets the app create assistant views before the script has loaded; they are filled once it has.
        + "\nwindow.locusAssistant = Object.assign((element) => window.locusAssistant.pending.push(element), { pending: [] });"
}

#[derive(Default)]
struct Lockdown(AtomicBool);

// The window that ran the last exam.
#[derive(Default)]
struct ExamWindow(Mutex<String>);

fn exam_view(app: &AppHandle) -> Option<Webview> {
    app.get_webview(&app.state::<ExamWindow>().0.lock().unwrap())
}

#[cfg(all(desktop, not(target_os = "macos")))]
fn is_exam_window(window: &tauri::Window) -> bool {
    *window.state::<ExamWindow>().0.lock().unwrap() == window.label()
}

// macOS opens new windows from the File menu.
#[cfg(target_os = "macos")]
fn app_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    use tauri::menu::{MenuItem, MenuItemKind, PredefinedMenuItem};
    let menu = Menu::default(app)?;
    for (index, item) in menu.items()?.into_iter().enumerate() {
        if let MenuItemKind::Submenu(submenu) = item {
            // The app menu comes first; its About item is replaced by one that opens the about modal.
            if index == 0 {
                submenu.remove_at(0)?;
                submenu.prepend(&MenuItem::with_id(app, "about", "About Locus", true, None::<&str>)?)?;
            }
            if submenu.text()? == "File" {
                submenu.prepend_items(&[
                    &MenuItem::with_id(app, "classic", "New Classic Window", true, Some("CmdOrCtrl+N"))?,
                    &MenuItem::with_id(app, "suite", "New Suite Window", true, Some("CmdOrCtrl+Shift+N"))?,
                    &PredefinedMenuItem::separator(app)?,
                ])?;
            }
        }
    }
    Ok(menu)
}

// Whether the running exam blocks other apps and Spaces.
#[derive(Default)]
struct Locked(AtomicBool);

#[derive(Default)]
struct FullscreenTransition(AtomicBool);

// Text shortcuts like Cmd+A and Cmd+C only work through menu items, so the exam keeps just these.
#[cfg(target_os = "macos")]
fn edit_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    use tauri::menu::{PredefinedMenuItem as Item, Submenu};
    // macOS always shows the first submenu as the app menu.
    let app_menu = Submenu::new(app, "Locus", true)?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &Item::undo(app, None)?,
            &Item::redo(app, None)?,
            &Item::separator(app)?,
            &Item::cut(app, None)?,
            &Item::copy(app, None)?,
            &Item::paste(app, None)?,
            &Item::select_all(app, None)?,
        ],
    )?;
    Menu::with_items(app, &[&app_menu, &edit])
}

#[tauri::command]
fn exam_mode(app: AppHandle, webview: Webview, on: bool) -> Result<(), String> {
    browser::trusted(&webview)?;
    if on {
        exam::start(&app)?;
    } else {
        exam::end(&app)?;
    }
    app.state::<Lockdown>().0.store(on, Ordering::SeqCst);
    // Other windows are hidden during the exam, so they cannot be used to get around it.
    #[cfg(desktop)]
    for label in session::labels(&app).iter().filter(|label| *label != webview.label()) {
        if let Some(window) = app.get_window(label) {
            let _ = if on { window.hide() } else { window.show() };
        }
    }
    #[cfg(target_os = "macos")]
    {
        let menu = if on { edit_menu(&app) } else { app_menu(&app) };
        app.set_menu(menu.map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    exam_lockdown(app, webview, on)
}

// macOS ignores presentation option changes inside fullscreen, so a change leaves fullscreen first
// and is applied once the exit has finished.
#[tauri::command]
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
fn exam_lockdown(app: AppHandle, webview: Webview, on: bool) -> Result<(), String> {
    browser::trusted(&webview)?;
    let window = webview.window();
    *app.state::<ExamWindow>().0.lock().unwrap() = window.label().into();
    app.state::<Locked>().0.store(on, Ordering::SeqCst);
    app.state::<FullscreenTransition>()
        .0
        .store(true, Ordering::SeqCst);
    #[cfg(target_os = "macos")]
    if window.is_fullscreen().map_err(|e| e.to_string())? {
        return window.set_fullscreen(false).map_err(|e| e.to_string());
    }
    apply(&app).map_err(|e| e.to_string())
}

// Sets the presentation options for the current state and enters fullscreen while the exam runs.
fn apply(app: &AppHandle) -> tauri::Result<()> {
    let running = app.state::<Lockdown>().0.load(Ordering::SeqCst);
    #[cfg(any(target_os = "macos", windows))]
    let locked = app.state::<Locked>().0.load(Ordering::SeqCst);
    let label = app.state::<ExamWindow>().0.lock().unwrap().clone();
    let Some(window) = app.get_window(&label) else {
        return Ok(());
    };
    app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        kiosk::set(locked);
        #[cfg(windows)]
        windows::set(&window, locked);
        #[cfg(desktop)]
        let _ = window.set_fullscreen(running);
        #[cfg(not(target_os = "macos"))]
        if running {
            if let Some(webview) = exam_view(window.app_handle()) {
                lockdown_done(&webview);
            }
        }
    })
}

#[cfg(desktop)]
fn report_fullscreen(app: &AppHandle, fullscreen: bool) {
    if let Some(view) = exam_view(app) {
        let _ = view.eval(format!(
            "window.ggbExamFullscreenChanged && window.ggbExamFullscreenChanged({fullscreen})"
        ));
    }
}

// Tells the page that a lockdown change is complete.
fn lockdown_done(window: &Webview) {
    let _ = window.eval("window.ggbExamLockdownDone && (window.ggbExamLockdownDone(), window.ggbExamLockdownDone = null)");
}

#[cfg(target_os = "macos")]
mod kiosk {
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSApplication, NSApplicationPresentationOptions as Opt,
        NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification,
    };
    use objc2_foundation::{
        NSNotification, NSNotificationCenter, NSNotificationName, NSOperationQueue,
    };
    use tauri::{AppHandle, Manager};

    // Locked blocks Cmd+Tab, Mission Control / Spaces switching (incl. Ctrl+arrows and trackpad swipes), dock and menu bar.
    // Must be called outside fullscreen, entering fullscreen keeps these options.
    pub fn set(locked: bool) {
        let mtm = MainThreadMarker::new().expect("main thread");
        let app = NSApplication::sharedApplication(mtm);
        if locked {
            app.setPresentationOptions(
                Opt::HideDock
                    | Opt::HideMenuBar
                    | Opt::DisableAppleMenu
                    | Opt::DisableProcessSwitching
                    | Opt::DisableForceQuit
                    | Opt::DisableSessionTermination
                    | Opt::DisableHideApplication,
            );
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
        } else {
            app.setPresentationOptions(Opt::Default);
        }
    }

    pub fn watch(handle: AppHandle) {
        let exit_handle = handle.clone();
        observe(
            unsafe { NSWindowDidExitFullScreenNotification },
            move || {
                use std::sync::atomic::Ordering;
                if exit_handle
                    .state::<super::Lockdown>()
                    .0
                    .load(Ordering::SeqCst)
                    && !exit_handle
                        .state::<super::FullscreenTransition>()
                        .0
                        .load(Ordering::SeqCst)
                {
                    super::report_fullscreen(&exit_handle, false);
                }
                exit_handle
                    .state::<super::FullscreenTransition>()
                    .0
                    .store(true, Ordering::SeqCst);
                let _ = super::apply(&exit_handle);
            },
        );
        observe(
            unsafe { NSWindowDidEnterFullScreenNotification },
            move || {
                handle
                    .state::<super::FullscreenTransition>()
                    .0
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                super::report_fullscreen(&handle, true);
                if let Some(webview) = super::exam_view(&handle) {
                    super::lockdown_done(&webview);
                }
            },
        );
    }

    fn observe(name: &NSNotificationName, f: impl Fn() + 'static) {
        let block = RcBlock::new(move |_: NonNull<NSNotification>| f());
        let observer = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(name),
                None,
                Some(&NSOperationQueue::mainQueue()),
                &block,
            )
        };
        std::mem::forget(observer);
    }
}

#[cfg(target_os = "macos")]
mod focus {
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2_app_kit::{NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
    use objc2_foundation::{NSNotification, NSOperationQueue};
    use tauri::AppHandle;

    // Reports focus loss when another app becomes frontmost.
    pub fn watch(app: AppHandle) {
        let block = RcBlock::new(move |_: NonNull<NSNotification>| {
            let focused = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .is_some_and(|front| front.processIdentifier() == std::process::id() as i32);
            if let Some(webview) = super::exam_view(&app) {
                let _ = webview.eval(format!(
                    "window.ggbExamWindowFocus && window.ggbExamWindowFocus({focused})"
                ));
            }
        });
        let observer = unsafe {
            NSWorkspace::sharedWorkspace()
                .notificationCenter()
                .addObserverForName_object_queue_usingBlock(
                    Some(NSWorkspaceDidActivateApplicationNotification),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &block,
                )
        };
        std::mem::forget(observer);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "ios")]
    ios::register_scene_delegate();
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.on_menu_event(|app, event| {
        if event.id().as_ref() == "about" {
            if let Some(window) = app.get_webview_window(&session::active(app)) {
                let _ = window.eval("window.locusAbout && window.locusAbout()");
            }
        } else {
            let _ = session::open_app(app, event.id().as_ref());
        }
    });
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    let app = builder
        .manage(Lockdown::default())
        .manage(ExamWindow::default())
        .manage(session::Windows::default())
        .manage(Locked::default())
        .manage(FullscreenTransition::default())
        .manage(exam::Exam::default())
        .manage(plugins::Plugins::default())
        .manage(assistant::Assistant::default())
        .manage(files::Files::default())
        .manage(mcp::Mcp::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_uri_scheme_protocol("pack", |ctx, request| {
            resourcepack::serve(ctx.app_handle(), request.uri().path())
        })
        .register_uri_scheme_protocol("plugin", |ctx, request| {
            plugins::serve(ctx.app_handle(), request.uri().path())
        })
        .invoke_handler(tauri::generate_handler![
            exam_mode,
            exam_lockdown,
            plugins::api::plugin_window,
            plugins::api::plugin_view,
            plugins::api::plugin_app,
            plugins::api::plugin_app_event,
            plugins::api::plugin_app_state,
            plugins::api::plugin_command,
            plugins::api::plugin_panel,
            plugins::api::plugin_storage,
            plugins::api::plugin_settings,
            plugins::api::plugin_setting_set,
            plugins::api::plugin_file,
            plugins::api::plugin_tool_result,
            plugins::api::plugin_exam,
            plugins::api::plugin_list,
            plugins::api::plugin_enable,
            #[cfg(target_os = "ios")]
            plugins::api::plugin_shortcuts,
            #[cfg(target_os = "ios")]
            plugins::api::plugin_key,
            assistant::assistant_send,
            assistant::assistant_settings,
            assistant::assistant_set,
            assistant::assistant_reset,
            assistant::assistant_history,
            assistant::assistant_models,
            tools::assistant_tool_result,
            files::file_pick_save,
            files::file_pick_open,
            files::file_read,
            files::file_write,
            resourcepack::library_list,
            resourcepack::library_activate,
            resourcepack::library_remove,
            resourcepack::library_pick,
            resourcepack::library_add,
            resourcepack::library_cancel,
            resourcepack::library_details,
            resourcepack::library_link,
            performance::performance_settings,
            performance::performance_set,
            mcp::mcp_settings,
            mcp::mcp_set,
            session::window_open,
            session::window_switch,
            session::window_close,
            session::general_settings,
            session::general_set,
            #[cfg(target_os = "ios")]
            ios::print
        ])
        .setup(|app| {
            session::restore(app.handle())?;
            #[cfg(target_os = "macos")]
            app.set_menu(app_menu(app.handle())?)?;
            resourcepack::branding(app.handle())?;
            // A taken port is reported in the settings, so it does not stop the app.
            let _ = mcp::start(app.handle());
            #[cfg(desktop)]
            shortcuts::install(app.handle())?;
            #[cfg(desktop)]
            updater::check(app.handle());
            plugins::sync(app.handle());
            #[cfg(target_os = "ios")]
            ios::attach_scene(app.handle());
            #[cfg(target_os = "ios")]
            ios::watch_focus(app.handle().clone());
            #[cfg(target_os = "macos")]
            focus::watch(app.handle().clone());
            #[cfg(target_os = "macos")]
            kiosk::watch(app.handle().clone());
            #[cfg(windows)]
            windows::watch(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if matches!(event, WindowEvent::Focused(true)) && session::is_main(window.label()) {
                session::focused(window.app_handle(), window.label());
                // Hiding during the focus change can deadlock with WebView2, so it runs afterwards.
                let app = window.app_handle().clone();
                std::thread::spawn(move || {
                    let _ = plugins::hide_all(&app);
                });
            }
            match event {
                // A plugin, resource pack or theme file dropped on an app window shows its details to add it.
                WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) if session::is_main(window.label()) => {
                    if let Some(file) = paths.iter().find(|path| resourcepack::Kind::from_path(path).is_some()) {
                        let script = match resourcepack::stage(window.app_handle(), file) {
                            Ok(details) => format!("window.locusLibraryOffer && window.locusLibraryOffer({details})"),
                            Err(error) => format!("window.locusLibraryOffer && window.locusLibraryOffer(null, {})", serde_json::json!(error)),
                        };
                        if let Some(view) = window.app_handle().get_webview(window.label()) {
                            let _ = view.eval(script);
                        }
                    }
                }
                WindowEvent::CloseRequested { api, .. } => {
                    if let Some(id) = plugins::owner(window.label()) {
                        api.prevent_close();
                        let _ = plugins::hide(window.app_handle(), &id);
                    } else if window.state::<Lockdown>().0.load(Ordering::SeqCst) {
                        api.prevent_close();
                    } else if session::is_main(window.label()) && session::confirm_close(window.app_handle()) {
                        // The page asks to save unsaved changes, then calls window_close.
                        api.prevent_close();
                        if let Some(view) = window.app_handle().get_webview(window.label()) {
                            let _ = view.eval("window.locusConfirmClose ? window.locusConfirmClose() : window.locusCloseWindow()");
                        }
                    } else if session::is_main(window.label()) {
                        session::closing(window.app_handle(), window.label());
                    }
                }
                WindowEvent::Destroyed if session::is_main(window.label()) => {
                    session::closed(window.app_handle(), window.label())
                }
                #[cfg(all(desktop, not(target_os = "macos")))]
                WindowEvent::Resized(_) if is_exam_window(window) => {
                    if let Ok(fullscreen) = window.is_fullscreen() {
                        let state = window.state::<FullscreenTransition>();
                        if fullscreen || !state.0.load(Ordering::SeqCst) {
                            report_fullscreen(window.app_handle(), fullscreen);
                        }
                        if fullscreen {
                            state.0.store(false, Ordering::SeqCst);
                        }
                    }
                }
                #[cfg(desktop)]
                WindowEvent::Resized(_) if plugins::owner(window.label()).is_some() => {
                    plugins::layout(window.app_handle(), &plugins::owner(window.label()).unwrap())
                }
                #[cfg(target_os = "ios")]
                WindowEvent::Resized(_) if window.label() == "main" => {
                    ios::refit(window.app_handle())
                }
                #[cfg(target_os = "linux")]
                WindowEvent::Focused(_) => {
                    let app = window.app_handle().clone();
                    std::thread::spawn(move || {
                        // Let focus transfers between this app's own windows finish before reporting a loss.
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let focused = app
                            .windows()
                            .values()
                            .any(|window| window.is_focused().unwrap_or(false));
                        if let Some(webview) = exam_view(&app) {
                            let _ = webview.eval(format!(
                                "window.ggbExamWindowFocus && window.ggbExamWindowFocus({focused})"
                            ));
                        }
                    });
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            if app.state::<Lockdown>().0.load(Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
    });
}

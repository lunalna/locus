// With the "unstable" feature every webview is a child of tao's content view, which breaks Cmd+key
// handling in two ways. wry answers performKeyEquivalent with NO for child webviews, so the menu runs
// before the page and GeoGebra's own shortcuts (Cmd+A, Cmd+C, Cmd+Z, ...) never fire. And tao's view
// swallows the `noop:` command WebKit sends up the responder chain to learn whether the page left a key
// unhandled, so WebKit never hands Cmd+V back to the Edit menu. The app windows' webviews get WebKit's
// normal handling back, and tao's view passes `noop:` on.

use std::sync::{Mutex, OnceLock};

use objc2::ffi;
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, Sel};
use objc2::sel;
use objc2_app_kit::{NSEvent, NSView};
use tauri::WebviewWindow;

type PerformKeyEquivalent = unsafe extern "C-unwind" fn(&NSView, Sel, &NSEvent) -> Bool;
type DoCommandBySelector = unsafe extern "C-unwind" fn(&AnyObject, Sel, Sel);

// The app windows' NSWindow pointers.
static MAIN_WINDOWS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static WEBKIT: OnceLock<PerformKeyEquivalent> = OnceLock::new();
static TAO: OnceLock<DoCommandBySelector> = OnceLock::new();

unsafe extern "C-unwind" fn perform_key_equivalent(
    view: &NSView,
    sel: Sel,
    event: &NSEvent,
) -> Bool {
    let main = view
        .window()
        .is_some_and(|window| MAIN_WINDOWS.lock().unwrap().contains(&(&*window as *const _ as usize)));
    match WEBKIT.get() {
        Some(webkit) if main => webkit(view, sel, event),
        _ => Bool::NO,
    }
}

unsafe extern "C-unwind" fn do_command_by_selector(view: &AnyObject, sel: Sel, command: Sel) {
    if command == sel!(noop:) {
        let next: *const AnyObject = objc2::msg_send![view, nextResponder];
        if let Some(next) = next.as_ref() {
            let _: () = objc2::msg_send![next, doCommandBySelector: command];
        }
        return;
    }
    TAO.get().unwrap()(view, sel, command)
}

pub fn install(window: &WebviewWindow) {
    let Ok(ns_window) = window.ns_window() else {
        return;
    };
    MAIN_WINDOWS.lock().unwrap().push(ns_window as usize);
    let _ = window.with_webview(|webview| unsafe {
        // wry registers its WKWebView subclass under a generated name, so the class comes from the view.
        let view: &AnyObject = &*webview.inner().cast();
        let sel = sel!(performKeyEquivalent:);
        let webkit = AnyClass::get(c"WKWebView").and_then(|class| class.instance_method(sel));
        if let (Some(wry_method), Some(webkit_method)) = (view.class().instance_method(sel), webkit) {
            WEBKIT.get_or_init(|| {
                std::mem::transmute::<Imp, PerformKeyEquivalent>(webkit_method.implementation())
            });
            let imp = std::mem::transmute::<PerformKeyEquivalent, Imp>(perform_key_equivalent);
            ffi::method_setImplementation(wry_method as *const _ as *mut _, imp);
        }
        let tao = AnyClass::get(c"TaoView")
            .and_then(|class| class.instance_method(sel!(doCommandBySelector:)));
        if let Some(method) = tao {
            TAO.get_or_init(|| {
                std::mem::transmute::<Imp, DoCommandBySelector>(method.implementation())
            });
            let imp = std::mem::transmute::<DoCommandBySelector, Imp>(do_command_by_selector);
            ffi::method_setImplementation(method as *const _ as *mut _, imp);
        }
    });
}

// iPadOS has one window, so each plugin window is a panel on top of the app that holds the native
// webviews of the plugin page and its views, taken out of the hidden windows Tauri creates for them.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Mutex;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, ClassType, MainThreadMarker, MainThreadOnly, Message};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSOperationQueue,
};
use objc2_ui_kit::{
    UIApplicationDidBecomeActiveNotification, UIApplicationWillResignActiveNotification,
    UIBlurEffect, UIBlurEffectStyle, UIGestureRecognizerDelegate, UIPanGestureRecognizer, UIScene,
    UISceneConnectionOptions, UISceneDelegate, UISceneSession, UIView, UIViewAutoresizing,
    UIVisualEffectView, UIWindow, UIWindowScene,
};
use tauri::{AppHandle, Manager, Webview};

struct Panel {
    root: Retained<UIView>,
    view: Retained<UIView>,
    blur: Retained<UIVisualEffectView>,
    min_width: f64,
    min_height: f64,
    // The height of the plugin's header bar, which drags the panel.
    header: f64,
    // Gesture recognizers hold their delegate weakly.
    _drag: Retained<PanelDrag>,
}

thread_local! {
    static PANELS: RefCell<HashMap<String, Panel>> = RefCell::new(HashMap::new());
    static MAIN_WINDOW: RefCell<Option<Retained<UIWindow>>> = const { RefCell::new(None) };
    static SCENE: RefCell<Option<Retained<UIWindowScene>>> = const { RefCell::new(None) };
    static DIRECTION: Cell<&'static str> = const { Cell::new("") };
}

static VISIBLE: Mutex<Vec<String>> = Mutex::new(Vec::new());
// The panels created so far, readable off the main thread.
static CREATED: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn has_panel(label: &str) -> bool {
    CREATED.lock().unwrap().iter().any(|created| created == label)
}

fn native(webview: &Webview, f: impl FnOnce(&UIView) + Send + 'static) {
    let _ = webview.with_webview(move |platform| f(unsafe { &*platform.inner().cast() }));
}

fn on_main(app: &AppHandle, label: &str, f: impl FnOnce(&Panel) + Send + 'static) {
    let label = label.to_string();
    let _ = app.run_on_main_thread(move || {
        PANELS.with_borrow(|panels| panels.get(&label).map(f));
    });
}

// The panel a gesture recognizer is attached to.
fn panel_of<R>(recognizer: &UIPanGestureRecognizer, f: impl FnOnce(&Panel) -> R) -> Option<R> {
    let view = recognizer.view()?;
    PANELS.with_borrow(|panels| {
        panels
            .values()
            .find(|panel| Retained::as_ptr(&panel.view) == Retained::as_ptr(&view))
            .map(f)
    })
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> CGRect {
    CGRect::new(CGPoint::new(x, y), CGSize::new(width, height))
}

// Keeps the frame inside the screen and at least the minimum size.
fn fit(panel: &Panel, frame: CGRect) -> CGRect {
    let bounds = panel.root.bounds();
    let width = frame.size.width.max(panel.min_width).min(bounds.size.width);
    let height = frame.size.height.max(panel.min_height).min(bounds.size.height);
    let x = frame.origin.x.clamp(0.0, bounds.size.width - width);
    let y = frame.origin.y.clamp(0.0, bounds.size.height - height);
    rect(x, y, width, height)
}

pub fn create_panel(
    app: &AppHandle,
    label: &str,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
    header: f64,
) {
    let Some(main) = app.get_webview("main") else {
        return;
    };
    let label = label.to_string();
    CREATED.lock().unwrap().push(label.clone());
    native(&main, move |webview| {
        let mtm = MainThreadMarker::new().expect("main thread");
        let root = webview.superview().expect("root view");
        let bounds = root.bounds();
        let frame = rect(
            (bounds.size.width - width) / 2.0,
            (bounds.size.height - height) / 2.0,
            width,
            height,
        );
        let view = UIView::initWithFrame(UIView::alloc(mtm), frame);
        view.setClipsToBounds(true);
        view.setHidden(true);
        unsafe {
            let layer: &objc2::runtime::AnyObject = msg_send![&view, layer];
            let _: () = msg_send![layer, setCornerRadius: 14.0f64];
        }
        let effect = UIBlurEffect::effectWithStyle(UIBlurEffectStyle::SystemMaterial, mtm);
        let blur =
            UIVisualEffectView::initWithEffect(UIVisualEffectView::alloc(mtm), Some(&effect));
        blur.setFrame(view.bounds());
        blur.setAutoresizingMask(
            UIViewAutoresizing::FlexibleWidth | UIViewAutoresizing::FlexibleHeight,
        );
        blur.setAlpha(0.0);
        view.addSubview(&blur);
        let drag: Retained<PanelDrag> = unsafe { msg_send![PanelDrag::alloc(mtm), init] };
        let pan = unsafe {
            UIPanGestureRecognizer::initWithTarget_action(
                UIPanGestureRecognizer::alloc(mtm),
                Some(&drag),
                Some(objc2::sel!(pan:)),
            )
        };
        pan.setDelegate(Some(ProtocolObject::from_ref(&*drag)));
        view.addGestureRecognizer(&pan);
        root.addSubview(&view);
        let panel = Panel {
            root,
            view,
            blur,
            min_width,
            min_height,
            header,
            _drag: drag,
        };
        panel.view.setFrame(fit(&panel, frame));
        PANELS.with_borrow_mut(|panels| panels.insert(label, panel));
    });
}

// Moves a webview into the panel, inset from its edges, and hides the window it came from.
pub fn adopt(webview: &Webview, label: &str, left: f64, top: f64, right: f64, bottom: f64) {
    let label = label.to_string();
    native(webview, move |webview| {
        PANELS.with_borrow(|panels| {
            let Some(panel) = panels.get(&label) else {
                return;
            };
            let window = webview.window();
            let size = panel.view.bounds().size;
            panel.view.addSubview(webview);
            webview.setFrame(rect(
                left,
                top,
                size.width - left - right,
                size.height - top - bottom,
            ));
            webview.setAutoresizingMask(
                UIViewAutoresizing::FlexibleWidth | UIViewAutoresizing::FlexibleHeight,
            );
            if let Some(window) = window {
                window.setHidden(true);
            }
            if let Some(main) = panel.root.window() {
                main.makeKeyWindow();
            }
        });
    });
}

pub fn visible(label: &str) -> bool {
    VISIBLE.lock().unwrap().iter().any(|visible| visible == label)
}

pub fn set_visible(app: &AppHandle, label: &str, visible: bool) {
    let mut shown = VISIBLE.lock().unwrap();
    shown.retain(|other| other != label);
    if visible {
        shown.push(label.into());
    }
    drop(shown);
    on_main(app, label, move |panel| {
        panel.view.setHidden(!visible);
        if visible {
            panel.root.bringSubviewToFront(&panel.view);
        }
    });
}

pub fn set_alpha(app: &AppHandle, label: &str, alpha: f64) {
    on_main(app, label, move |panel| panel.view.setAlpha(alpha));
}

pub fn set_blur(app: &AppHandle, label: &str, strength: f64) {
    on_main(app, label, move |panel| panel.blur.setAlpha(strength));
}

pub fn set_view_alpha(webview: &Webview, alpha: f64) {
    native(webview, move |view| view.setAlpha(alpha));
}

pub fn set_view_visible(webview: &Webview, visible: bool) {
    native(webview, move |view| view.setHidden(!visible));
}

// Fingers are imprecise, so the corners reach further in than the edges.
const EDGE: f64 = 20.0;
const CORNER: f64 = 44.0;

// Which part of the panel a touch grabs: an edge or corner resizes, the header bar drags.
fn grab(panel: &Panel, point: CGPoint) -> &'static str {
    let size = panel.view.bounds().size;
    let hits = |top: f64, side: f64| {
        (
            point.y < top,
            point.y > size.height - side,
            point.x < side,
            point.x > size.width - side,
        )
    };
    let corner = hits(CORNER, CORNER);
    // The top edge stays thin so the header bar can still be dragged.
    let (n, s, w, e) = if (corner.0 || corner.1) && (corner.2 || corner.3) {
        corner
    } else {
        hits(8.0, EDGE)
    };
    match (n, s, w, e) {
        (true, _, true, _) => "nw",
        (true, _, _, true) => "ne",
        (_, true, true, _) => "sw",
        (_, true, _, true) => "se",
        (true, ..) => "n",
        (_, true, ..) => "s",
        (_, _, true, _) => "w",
        (_, _, _, true) => "e",
        _ if point.y < panel.header => "drag",
        _ => "",
    }
}

fn move_frame(panel: &Panel, direction: &str, dx: f64, dy: f64) {
    let CGRect { origin, size } = panel.view.frame();
    let (mut x, mut y, mut width, mut height) = (origin.x, origin.y, size.width, size.height);
    if direction == "drag" {
        x += dx;
        y += dy;
    }
    if direction.contains('n') {
        let dy = dy.min(height - panel.min_height);
        y += dy;
        height -= dy;
    }
    if direction.contains('s') {
        height += dy;
    }
    if direction.contains('w') {
        let dx = dx.min(width - panel.min_width);
        x += dx;
        width -= dx;
    }
    if direction.contains('e') {
        width += dx;
    }
    panel.view.setFrame(fit(panel, rect(x, y, width, height)));
}

// iPadOS cannot drag native windows, so a pan on the header or an edge moves or resizes the panel.
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "LocusPanelDrag"]
    #[thread_kind = MainThreadOnly]
    struct PanelDrag;

    unsafe impl NSObjectProtocol for PanelDrag {}

    unsafe impl UIGestureRecognizerDelegate for PanelDrag {
        #[unsafe(method(gestureRecognizerShouldBegin:))]
        fn should_begin(&self, recognizer: &UIPanGestureRecognizer) -> bool {
            panel_of(recognizer, |panel| {
                // The pan begins after the finger moved, so this goes back to where the touch started.
                let now = recognizer.locationInView(Some(&panel.view));
                let moved = recognizer.translationInView(Some(&panel.view));
                let start = CGPoint::new(now.x - moved.x, now.y - moved.y);
                let direction = grab(panel, start);
                DIRECTION.set(direction);
                !direction.is_empty()
            })
            .unwrap_or(false)
        }
    }

    impl PanelDrag {
        #[unsafe(method(pan:))]
        fn pan(&self, recognizer: &UIPanGestureRecognizer) {
            panel_of(recognizer, |panel| {
                let moved = recognizer.translationInView(Some(&panel.root));
                recognizer.setTranslation_inView(CGPoint::ZERO, Some(&panel.root));
                move_frame(panel, DIRECTION.get(), moved.x, moved.y);
            });
        }
    }
);

// Keeps the panels on screen after the iPad rotates.
pub fn refit(app: &AppHandle) {
    let _ = app.run_on_main_thread(|| {
        PANELS.with_borrow(|panels| {
            for panel in panels.values() {
                panel.view.setFrame(fit(panel, panel.view.frame()));
            }
        })
    });
}

pub fn remove_view(webview: &Webview) {
    native(webview, |view| view.removeFromSuperview());
}

// The SDK requires the scene life cycle, but tao creates the main window without a scene,
// so this delegate gives the window the scene once both exist.
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "LocusSceneDelegate"]
    #[thread_kind = MainThreadOnly]
    struct SceneDelegate;

    unsafe impl NSObjectProtocol for SceneDelegate {}

    unsafe impl UISceneDelegate for SceneDelegate {
        #[unsafe(method(scene:willConnectToSession:options:))]
        fn will_connect(&self, scene: &UIScene, _: &UISceneSession, _: &UISceneConnectionOptions) {
            if let Some(scene) = scene.downcast_ref::<UIWindowScene>() {
                SCENE.set(Some(scene.retain()));
                attach();
            }
        }
    }
);

// UIKit looks the delegate up by name, so the class must exist before the app starts.
pub fn register_scene_delegate() {
    SceneDelegate::class();
}

fn attach() {
    MAIN_WINDOW.with_borrow(|window| {
        SCENE.with_borrow(|scene| {
            if let (Some(window), Some(scene)) = (window, scene) {
                window.setWindowScene(Some(scene));
                window.makeKeyAndVisible();
            }
        })
    });
}

pub fn attach_scene(app: &AppHandle) {
    let Some(main) = app.get_webview("main") else {
        return;
    };
    native(&main, |webview| {
        MAIN_WINDOW.set(webview.window());
        attach();
    });
}

// Reports focus loss when the app stops being active, e.g. for the app switcher or Control Center.
pub fn watch_focus(app: AppHandle) {
    for (name, focused) in unsafe {
        [
            (UIApplicationWillResignActiveNotification, false),
            (UIApplicationDidBecomeActiveNotification, true),
        ]
    } {
        let app = app.clone();
        let block = RcBlock::new(move |_: NonNull<NSNotification>| {
            if let Some(webview) = app.get_webview("main") {
                let _ = webview.eval(format!(
                    "window.ggbExamWindowFocus && window.ggbExamWindowFocus({focused})"
                ));
            }
        });
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

// Tauri's window.print does nothing on iOS, so the page is printed with the system print sheet.
#[tauri::command]
pub fn print(app: AppHandle) {
    let Some(main) = app.get_webview("main") else {
        return;
    };
    native(&main, |webview| unsafe {
        let controller: Retained<AnyObject> =
            msg_send![objc2::class!(UIPrintInteractionController), sharedPrintInteractionController];
        let formatter: Retained<AnyObject> = msg_send![webview, viewPrintFormatter];
        let _: () = msg_send![&controller, setPrintFormatter: &*formatter];
        let _: bool = msg_send![
            &controller,
            presentAnimated: true,
            completionHandler: std::ptr::null::<AnyObject>()
        ];
    });
}

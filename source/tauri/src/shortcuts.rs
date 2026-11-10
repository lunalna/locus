use std::str::FromStr;

use tauri::{AppHandle, Window};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::plugins::keys::{self, Combo};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                let names = [
                    (Modifiers::CONTROL, keys::CTRL),
                    (Modifiers::ALT, keys::ALT),
                    (Modifiers::SHIFT, keys::SHIFT),
                    (Modifiers::SUPER, keys::META),
                ];
                let mods = names
                    .iter()
                    .filter(|(modifier, _)| shortcut.mods.contains(*modifier))
                    .fold(0, |mods, (_, flag)| mods | flag);
                crate::plugins::pressed(app, &Combo { mods, code: Some(shortcut.key.to_string()) });
            })
            .build(),
    )?;
    #[cfg(target_os = "macos")]
    mac::watch(app.clone());
    Ok(())
}

// Registers the plugin shortcuts that have a key. Modifiers alone and Fn come from the key monitors,
// and so do the keys global shortcuts cannot name on macOS.
pub fn register(app: &AppHandle, combos: &[Combo]) {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    for combo in combos {
        let global = match combo.code.as_deref() {
            None => false,
            #[cfg(target_os = "macos")]
            Some(code) if mac::KEYS.iter().any(|(_, name)| *name == code) => false,
            Some(_) => combo.mods & (keys::FN | keys::RIGHT_ALT) == 0,
        };
        // A shortcut another app holds, or a key global shortcuts do not know, is left out.
        if let (true, Ok(shortcut)) = (global, Shortcut::from_str(&combo.accelerator())) {
            let _ = shortcuts.register(shortcut);
        }
    }
}

pub fn configure(window: &Window) {
    #[cfg(target_os = "macos")]
    {
        let window = window.clone();
        let dispatcher = window.clone();
        let _ = dispatcher.run_on_main_thread(move || {
            if let Ok(ptr) = window.ns_window() {
                use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior as Behavior};
                let native: &NSWindow = unsafe { &*ptr.cast() };
                native.setLevel(objc2_app_kit::NSScreenSaverWindowLevel);
                native.setHidesOnDeactivate(false);
                // Showing the window brings it to the current space, including a fullscreen one.
                native.setCollectionBehavior(
                    Behavior::MoveToActiveSpace
                        | Behavior::FullScreenAuxiliary
                        | Behavior::Stationary
                        | Behavior::IgnoresCycle,
                );
            }
        });
    }
    #[cfg(target_os = "linux")]
    linux_modifiers(window);
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        use windows_sys::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
        };
        let disabled: i32 = 1;
        unsafe {
            DwmSetWindowAttribute(
                hwnd.0 as _,
                DWMWA_TRANSITIONS_FORCEDISABLED as u32,
                &disabled as *const i32 as _,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
}

pub fn opacity(window: &Window, value: f64) {
    let window = window.clone();
    let dispatcher = window.clone();
    let _ = dispatcher.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        if let Ok(ptr) = window.ns_window() {
            let native: &objc2_app_kit::NSWindow = unsafe { &*ptr.cast() };
            native.setAlphaValue(value);
        }
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
                    LWA_ALPHA, WS_EX_LAYERED,
                };
                let hwnd = hwnd.0 as _;
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
                SetLayeredWindowAttributes(hwnd, 0, (value * 255.0) as u8, LWA_ALPHA);
            }
        }
        #[cfg(target_os = "linux")]
        if let Ok(native) = window.gtk_window() {
            use gtk::prelude::*;
            native.set_opacity(value);
        }
    });
}

#[cfg(target_os = "linux")]
fn linux_modifiers(window: &Window) {
    use gtk::{gdk::keys::constants as key, glib, prelude::*};
    use std::{cell::RefCell, rc::Rc};
    use tauri::Manager;
    let flag = |keyval: gtk::gdk::keys::Key| match keyval {
        key::Control_L | key::Control_R => Some(keys::CTRL),
        key::Alt_L | key::Alt_R => Some(keys::ALT),
        key::Shift_L | key::Shift_R => Some(keys::SHIFT),
        key::Super_L | key::Super_R => Some(keys::META),
        _ => None,
    };
    if let Ok(native) = window.gtk_window() {
        let held = Rc::new(RefCell::new(keys::Modifiers::new()));
        let released = held.clone();
        let app = window.app_handle().clone();
        native.connect_key_press_event(move |_, event| {
            if let Some(combo) = flag(event.keyval()).and_then(|flag| held.borrow_mut().down(flag)) {
                crate::plugins::pressed(&app, &combo);
            }
            glib::Propagation::Proceed
        });
        native.connect_key_release_event(move |_, event| {
            if let Some(flag) = flag(event.keyval()) {
                released.borrow_mut().up(flag);
            }
            glib::Propagation::Proceed
        });
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags as Flags, NSEventType};
    use std::{
        ptr::NonNull,
        sync::{Arc, Mutex},
    };
    use tauri::AppHandle;

    use crate::plugins::keys::{self, Combo, Modifiers};

    // Keys that global shortcuts cannot name, by key code. 0x0A is left of 1 on ISO keyboards, ^ on German layouts.
    pub const KEYS: &[(u16, &str)] = &[(0x0A, "IntlBackslash")];

    fn mods(flags: Flags) -> u8 {
        let names = [
            (Flags::Control, keys::CTRL),
            (Flags::Option, keys::ALT),
            (Flags::Shift, keys::SHIFT),
            (Flags::Command, keys::META),
            (Flags::Function, keys::FN),
        ];
        let mods = names
            .iter()
            .filter(|(flag, _)| flags.contains(*flag))
            .fold(0, |mods, (_, flag)| mods | flag);
        // 0x40 is NX_DEVICERALTKEYMASK, set only while the right Option key is held.
        if flags.bits() & 0x40 != 0 {
            mods | keys::RIGHT_ALT
        } else {
            mods
        }
    }

    fn handle(app: &AppHandle, held: &Mutex<Modifiers>, event: &NSEvent) {
        if event.r#type() != NSEventType::KeyDown {
            if let Some(combo) = held.lock().unwrap().set(mods(event.modifierFlags())) {
                crate::plugins::pressed(app, &combo);
            }
            return;
        }
        let code = KEYS.iter().find(|(code, _)| *code == event.keyCode()).map(|(_, name)| name);
        if let (Some(code), false) = (code, event.isARepeat()) {
            let mods = mods(event.modifierFlags()) & !(keys::FN | keys::RIGHT_ALT);
            crate::plugins::pressed(app, &Combo { mods, code: Some(code.to_string()) });
        }
    }

    pub fn watch(app: AppHandle) {
        let held = Arc::new(Mutex::new(Modifiers::new()));
        let global_held = held.clone();
        let global_app = app.clone();
        let global = RcBlock::new(move |event: NonNull<NSEvent>| {
            handle(&global_app, &global_held, unsafe { event.as_ref() });
        });
        let mask = NSEventMask::FlagsChanged | NSEventMask::KeyDown;
        if let Some(observer) =
            NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global)
        {
            std::mem::forget(observer);
        }
        let local = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            handle(&app, &held, unsafe { event.as_ref() });
            event.as_ptr()
        });
        if let Some(observer) =
            unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local) }
        {
            std::mem::forget(observer);
        }
    }
}

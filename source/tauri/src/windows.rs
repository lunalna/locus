use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};

use tauri::{AppHandle, Window};

use crate::plugins::keys::{self, Modifiers};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetAncestor, GetMessageW, GetWindowThreadProcessId, SetWindowsHookExW,
    UnhookWindowsHookEx, EVENT_SYSTEM_FOREGROUND, GA_ROOTOWNER, HC_ACTION, KBDLLHOOKSTRUCT,
    LLKHF_ALTDOWN, MSG, WH_KEYBOARD_LL, WINEVENT_OUTOFCONTEXT, WM_KEYDOWN, WM_SYSKEYDOWN,
};

static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
static LOCKED: AtomicBool = AtomicBool::new(false);
static HELD: Mutex<Modifiers> = Mutex::new(Modifiers::new());
static APP: OnceLock<AppHandle> = OnceLock::new();

// Locked blocks Alt+Tab, Alt+Esc, Ctrl+Esc and the Windows key, and keeps the window on top.
// Ctrl+Alt+Del cannot be blocked by an app.
pub fn set(window: &Window, locked: bool) {
    let _ = window.set_always_on_top(locked);
    LOCKED.store(locked, Ordering::SeqCst);
}

// Own thread, so a busy UI thread never makes Windows time out the hook and let keys through.
fn spawn_keyboard_hook() -> u32 {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || unsafe {
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(block_keys),
            GetModuleHandleW(null_mut()),
            0,
        );
        let _ = tx.send(GetCurrentThreadId());
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}
        UnhookWindowsHookEx(hook);
    });
    rx.recv().expect("keyboard hook thread")
}

unsafe extern "system" fn block_keys(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let key = &*(lparam as *const KBDLLHOOKSTRUCT);
        let alt = key.flags & LLKHF_ALTDOWN != 0;
        let ctrl = GetAsyncKeyState(VK_CONTROL as i32) < 0;
        let flag = match key.vkCode as u16 {
            VK_MENU | VK_LMENU => Some(keys::ALT),
            VK_RMENU => Some(keys::ALT | keys::RIGHT_ALT),
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Some(keys::CTRL),
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Some(keys::SHIFT),
            VK_LWIN | VK_RWIN => Some(keys::META),
            _ => None,
        };
        if let Some(flag) = flag {
            let down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
            let mut held = HELD.lock().unwrap();
            if !down {
                held.up(flag);
            } else if let (Some(combo), Some(app)) = (held.down(flag), APP.get()) {
                drop(held);
                crate::plugins::pressed(app, &combo);
            }
        }
        let blocked = LOCKED.load(Ordering::SeqCst)
            && match key.vkCode as u16 {
                VK_LWIN | VK_RWIN => true,
                VK_TAB => alt,
                VK_ESCAPE => alt || ctrl,
                _ => false,
            };
        if blocked {
            return 1;
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

// Reports focus loss when another app comes to the foreground.
pub fn watch(app: AppHandle) {
    let _ = APP.set(app);
    HOOK_THREAD.store(spawn_keyboard_hook(), Ordering::SeqCst);
    unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            null_mut(),
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
    }
}

unsafe extern "system" fn on_foreground(
    _: HWINEVENTHOOK,
    _: u32,
    hwnd: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    let mut pid = 0;
    GetWindowThreadProcessId(GetAncestor(hwnd, GA_ROOTOWNER), &mut pid);
    let focused = pid == std::process::id();
    if let Some(webview) = APP.get().and_then(crate::exam_view) {
        let _ = webview.eval(format!(
            "window.ggbExamWindowFocus && window.ggbExamWindowFocus({focused})"
        ));
    }
}

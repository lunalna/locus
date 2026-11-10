// The running exam. Accommodations come from plugins only: they can allow the AI assistant or leaving
// the window, and add lines to the exam log and status the teacher sees.

use std::sync::{Mutex, MutexGuard};

use serde_json::json;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct Exam(Mutex<State>);

#[derive(Default)]
struct State {
    active: bool,
    assistant: bool,
    leaving: bool,
    status: Vec<(String, String)>,
}

fn state(app: &AppHandle) -> MutexGuard<'_, State> {
    app.state::<Exam>().inner().0.lock().unwrap()
}

// Chats and plugin state from before or during the exam must not carry over.
fn reset(app: &AppHandle, active: bool) -> Result<(), String> {
    *state(app) = State { active, ..Default::default() };
    crate::assistant::end(app);
    crate::plugins::reset(app)
}

pub fn start(app: &AppHandle) -> Result<(), String> {
    reset(app, true)
}

pub fn end(app: &AppHandle) -> Result<(), String> {
    reset(app, false)
}

pub fn active(app: &AppHandle) -> bool {
    state(app).active
}

pub fn assistant_enabled(app: &AppHandle) -> bool {
    let state = state(app);
    !state.active || state.assistant
}

pub fn leaving_allowed(app: &AppHandle) -> bool {
    state(app).leaving
}

fn eval(app: &AppHandle, script: String) {
    if let Some(view) = crate::exam_view(app) {
        let _ = view.eval(script);
    }
}

// Adds a line to the exam log, which the page stamps with the exam time.
pub fn record(app: &AppHandle, text: String) {
    if active(app) {
        eval(app, format!("window.ggbExamLog && window.ggbExamLog({})", json!(text)));
    }
}

pub fn running(app: &AppHandle) -> Result<(), String> {
    if active(app) {
        Ok(())
    } else {
        Err("No exam is running.".into())
    }
}

// A plugin's line in the exam status, removed with None.
pub fn status(app: &AppHandle, plugin: &str, text: Option<String>) -> Result<(), String> {
    running(app)?;
    let lines: Vec<String> = {
        let mut state = state(app);
        state.status.retain(|(name, _)| name != plugin);
        if let Some(text) = text {
            state.status.push((plugin.into(), text));
        }
        state.status.iter().map(|(name, text)| format!("{name}: {text}")).collect()
    };
    eval(app, format!("window.ggbExamStatus && window.ggbExamStatus({})", json!(lines)));
    Ok(())
}

pub fn allow_assistant(app: &AppHandle, allowed: bool) -> Result<(), String> {
    running(app)?;
    state(app).assistant = allowed;
    crate::assistant::refresh(app);
    Ok(())
}

// The page lifts or restores the lockdown, and stops counting leaving the window as cheating.
pub fn allow_leaving(app: &AppHandle, allowed: bool) -> Result<(), String> {
    running(app)?;
    state(app).leaving = allowed;
    eval(app, format!("window.ggbExamAllowLeaving && window.ggbExamAllowLeaving({allowed})"));
    Ok(())
}

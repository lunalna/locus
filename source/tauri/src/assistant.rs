use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::async_runtime::Sender;
use tauri::{AppHandle, Manager, Webview};

use crate::exam;
use crate::providers::{self, Protocol, Provider, ANTHROPIC_MODELS, PROVIDERS};
use crate::tools::{call, label, tools};

const MAX_STEPS: usize = 20;
const SYSTEM: &str = "You are an AI assistant that can help the user with using Locus, a math app, and with other excercises. \
Answer concisely. \
Use Markdown, and write math in LaTeX between $...$ or $$...$$.";
const PROMPT: &str = "The user typed this request into the Algebra input instead of a command. \
Carry it out with the tools, without asking questions. \
Then reply with one short plain sentence without Markdown or LaTeX, or with the answer if it was a question.";

// A conversation and the events shown for it, so a reloaded page can show it again.
#[derive(Default)]
struct Chat {
    messages: Vec<Value>,
    events: Vec<Value>,
}

#[derive(Default)]
pub struct Assistant {
    // One chat per webview, keyed by its label.
    chats: Mutex<HashMap<String, Chat>>,
    pub(crate) pending: Mutex<HashMap<u64, Sender<String>>>,
    pub(crate) next: AtomicU64,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_config_dir().map_err(|e| e.to_string())?.join("ai.json"))
}

fn saved(app: &AppHandle) -> Result<Value, String> {
    Ok(std::fs::read_to_string(settings_path(app)?)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({})))
}

fn provider(saved: &Value) -> &'static Provider {
    providers::find(saved["provider"].as_str().unwrap_or_default()).unwrap_or(&PROVIDERS[0])
}

// Providers are only kept once the user added them, with their key and URL.
fn added(saved: &Value, provider: &Provider) -> bool {
    saved["providers"][provider.id].is_object()
}

fn key<'a>(saved: &'a Value, provider: &Provider) -> &'a str {
    saved["providers"][provider.id]["key"].as_str().unwrap_or_default()
}

fn url<'a>(saved: &'a Value, provider: &'a Provider) -> &'a str {
    saved["providers"][provider.id]["url"].as_str().filter(|url| !url.is_empty()).unwrap_or(provider.url)
}

// Whether a provider has what it needs for a request.
fn ready(saved: &Value, provider: &Provider) -> bool {
    added(saved, provider) && !url(saved, provider).is_empty() && (!provider.needs_key || !key(saved, provider).is_empty())
}

fn valid_url(url: &str) -> bool {
    url.is_empty() || url.starts_with("http://") || url.starts_with("https://")
}

fn write(app: &AppHandle, saved: &Value) -> Result<(), String> {
    let path = settings_path(app)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, saved.to_string()).map_err(|e| e.to_string())
}

// The settings of the chat, with defaults for anything missing or no longer valid.
fn settings(app: &AppHandle) -> Result<Value, String> {
    let saved = saved(app)?;
    let provider = provider(&saved);
    let model = saved["model"].as_str().unwrap_or_default();
    let model = match provider.protocol {
        Protocol::Anthropic if !ANTHROPIC_MODELS.contains(&model) => ANTHROPIC_MODELS[0],
        _ => model,
    };
    let effort = saved["effort"].as_str().filter(|e| provider.efforts().contains(e)).unwrap_or(provider.default_effort());
    Ok(json!({
        "enabled": saved["enabled"].as_bool().unwrap_or(true),
        "sidebar": saved["sidebar"].as_bool().unwrap_or(false),
        "prompt": saved["prompt"].as_bool().unwrap_or(true),
        "provider": provider.id,
        "key": key(&saved, provider),
        "url": url(&saved, provider),
        "ready": ready(&saved, provider),
        "model": model,
        "effort": effort,
        "thinking": saved["thinking"].as_bool().unwrap_or(true),
        "instructions": saved["instructions"].as_str().unwrap_or_default(),
    }))
}

// Whether AI features are turned on at all.
pub fn enabled(app: &AppHandle) -> bool {
    saved(app).is_ok_and(|saved| saved["enabled"].as_bool().unwrap_or(true))
}

// Whether chats can be used: AI features are on, a model is set up, and a running exam allows it.
fn available(settings: &Value, app: &AppHandle) -> bool {
    settings["enabled"] == true && settings["ready"] == true && settings["model"] != "" && exam::assistant_enabled(app)
}

// Lets the app windows show or hide the assistant after a change.
pub fn refresh(app: &AppHandle) {
    for label in crate::session::labels(app) {
        if let Some(view) = app.get_webview(&label) {
            let _ = view.eval("window.locusAssistantRefresh && window.locusAssistantRefresh()");
        }
    }
}

// Returns the settings without the keys themselves, so they never reach a webview.
// `key` and `url` are those of the chat's provider.
#[tauri::command]
pub fn assistant_settings(app: AppHandle, webview: Webview) -> Result<Value, String> {
    crate::plugins::controls(&webview)?;
    let saved = saved(&app)?;
    let mut settings = settings(&app)?;
    settings["key"] = json!(settings["key"] != "");
    settings["available"] = json!(available(&settings, &app));
    settings["providers"] = PROVIDERS
        .iter()
        .map(|provider| {
            json!({
                "id": provider.id,
                "name": provider.name,
                "url": url(&saved, provider),
                "defaultUrl": provider.url,
                "added": added(&saved, provider),
                "key": !key(&saved, provider).is_empty(),
                "needsKey": provider.needs_key,
                "ready": ready(&saved, provider),
                "efforts": provider.efforts(),
            })
        })
        .collect();
    Ok(settings)
}

// Changes one setting. `key`, `url` and `remove` belong to the given provider, or to the chat's provider.
#[tauri::command]
pub async fn assistant_set(
    app: AppHandle,
    webview: Webview,
    name: String,
    value: String,
    provider: Option<String>,
) -> Result<(), String> {
    crate::plugins::controls(&webview)?;
    let value = if name == "instructions" { value.trim() } else { value.trim().trim_end_matches('/') };
    let mut saved = saved(&app)?;
    let target = match provider {
        Some(id) => providers::find(&id).ok_or("Unknown provider.")?,
        None => self::provider(&saved),
    };
    match name.as_str() {
        "key" => {
            if !value.is_empty() {
                target.models(url(&saved, target), value).await?;
            }
            saved["providers"][target.id]["key"] = json!(value);
        }
        "url" if valid_url(value) => saved["providers"][target.id]["url"] = json!(value),
        "remove" => {
            if let Some(providers) = saved["providers"].as_object_mut() {
                providers.remove(target.id);
            }
        }
        // Another provider has other models and efforts.
        "provider" if providers::find(value).is_some() => {
            saved["provider"] = json!(value);
            saved["model"] = json!("");
            saved["effort"] = json!("");
        }
        "model" | "effort" | "instructions" => saved[&name] = json!(value),
        "enabled" | "sidebar" | "prompt" | "thinking" if matches!(value, "true" | "false") => saved[&name] = json!(value == "true"),
        _ => return Err("Invalid AI setting.".into()),
    }
    write(&app, &saved)?;
    if name == "enabled" {
        crate::mcp::start(&app)?;
    }
    refresh(&app);
    Ok(())
}

// Adds a provider, or changes its key and URL, once they work. Returns its models.
#[tauri::command]
pub async fn assistant_provider_add(
    app: AppHandle,
    webview: Webview,
    provider: String,
    key: String,
    url: String,
) -> Result<Vec<String>, String> {
    crate::plugins::controls(&webview)?;
    let provider = providers::find(&provider).ok_or("Unknown provider.")?;
    let (key, url) = (key.trim(), url.trim().trim_end_matches('/'));
    if !valid_url(url) {
        return Err("The URL must start with http:// or https://.".into());
    }
    let address = if url.is_empty() { provider.url } else { url };
    if address.is_empty() {
        return Err("Enter the URL of the service.".into());
    }
    if provider.needs_key && key.is_empty() {
        return Err("Enter an API key.".into());
    }
    let models = provider.models(address, key).await?;
    let mut saved = saved(&app)?;
    saved["providers"][provider.id] = json!({"key": key, "url": url});
    write(&app, &saved)?;
    refresh(&app);
    Ok(models)
}

// The models a provider offers, which also checks its key and URL.
#[tauri::command]
pub async fn assistant_models(app: AppHandle, webview: Webview, provider: String) -> Result<Vec<String>, String> {
    crate::plugins::controls(&webview)?;
    let saved = saved(&app)?;
    let provider = providers::find(&provider).ok_or("Unknown provider.")?;
    if !ready(&saved, provider) {
        return Err(format!("Add {} under Model providers first.", provider.name));
    }
    provider.models(url(&saved, provider), key(&saved, provider)).await
}

pub fn end(app: &AppHandle) {
    app.state::<Assistant>().chats.lock().unwrap().clear();
    for label in crate::session::labels(app) {
        if let Some(view) = app.get_webview(&label) {
            let _ = view.eval("window.locusAssistantReset && window.locusAssistantReset()");
        }
    }
}

#[tauri::command]
pub fn assistant_reset(app: AppHandle, webview: Webview) -> Result<(), String> {
    crate::plugins::controls(&webview)?;
    app.state::<Assistant>().chats.lock().unwrap().remove(webview.label());
    Ok(())
}

// The events of this webview's chat, to show it again after the page loaded another app.
#[tauri::command]
pub fn assistant_history(app: AppHandle, webview: Webview) -> Result<Vec<Value>, String> {
    crate::plugins::controls(&webview)?;
    let chats = app.state::<Assistant>();
    let chats = chats.chats.lock().unwrap();
    Ok(chats.get(webview.label()).map(|chat| chat.events.clone()).unwrap_or_default())
}

// Keeps an event for the chat's history and sends it to the chat, a plugin page or the assistant view in an app window.
fn emit(app: &AppHandle, chat: &str, event: Value) {
    {
        let state = app.state::<Assistant>();
        let mut chats = state.chats.lock().unwrap();
        let events = &mut chats.entry(chat.into()).or_default().events;
        // Deltas are joined, so the history does not keep one event per token.
        match events.last_mut() {
            Some(last) if last["type"] == "delta" && event["type"] == "delta" => {
                last["text"] = json!(last["text"].as_str().unwrap_or_default().to_owned() + event["text"].as_str().unwrap_or_default());
            }
            _ => events.push(event.clone()),
        }
    }
    if let Some(view) = app.get_webview(chat) {
        let _ = view.eval(format!("window.assistantEvent && window.assistantEvent({event})"));
    }
}

// Runs the conversation on the target window until the model is done, sending its output to `emit`.
async fn run(
    app: &AppHandle,
    target: &str,
    extra: &str,
    messages: &mut Vec<Value>,
    emit: impl Fn(Value),
) -> Result<(), String> {
    let settings = settings(app)?;
    let provider = providers::find(settings["provider"].as_str().unwrap_or_default()).unwrap_or(&PROVIDERS[0]);
    let instructions = settings["instructions"].as_str().unwrap_or_default();
    let mut system = format!("{SYSTEM}{extra}");
    if !instructions.is_empty() {
        system += &format!("\n\nInstructions from the user:\n{instructions}");
    }
    for _ in 0..MAX_STEPS {
        let response = provider
            .complete(&settings, &system, &tools(app, target), messages, &emit)
            .await?;
        let content = response["content"].as_array().cloned().unwrap_or_default();
        messages.push(json!({"role": "assistant", "content": content}));
        match response["stop_reason"].as_str() {
            Some("tool_use") => {}
            Some("refusal") => return Err("The model declined to answer this.".into()),
            Some("max_tokens") => return Err("The answer was too long and was cut off.".into()),
            _ => return Ok(()),
        }
        let mut results = Vec::new();
        for block in content.iter().filter(|block| block["type"] == "tool_use") {
            let name = block["name"].as_str().unwrap_or_default();
            let result = call(app, target, name, &block["input"]).await;
            emit(json!({"type": "tool", "label": label(name, &block["input"])}));
            let (content, is_error) = match result {
                Ok(content) => (content, false),
                Err(error) => (json!(error), true),
            };
            results.push(json!({
                "type": "tool_result",
                "tool_use_id": block["id"],
                "content": content,
                "is_error": is_error,
            }));
        }
        messages.push(json!({"role": "user", "content": results}));
    }
    Err("The model stopped after too many steps.".into())
}

async fn send(app: &AppHandle, chat: &str, text: String, images: Vec<String>) -> Result<(), String> {
    if !available(&settings(app)?, app) {
        return Err("The AI sidebar is not available.".into());
    }
    // Images are base64 JPEGs.
    let mut content: Vec<Value> = images
        .iter()
        .map(|data| json!({"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": data}}))
        .collect();
    if !text.is_empty() {
        content.push(json!({"type": "text", "text": text}));
    }
    if content.is_empty() {
        return Err("Write a message or add a picture.".into());
    }
    let mut messages = {
        let state = app.state::<Assistant>();
        let chats = state.chats.lock().unwrap();
        chats.get(chat).map(|chat| chat.messages.clone()).unwrap_or_default()
    };
    messages.push(json!({"role": "user", "content": content}));
    // A chat in an app window works on that window, a plugin's chat on the window used last.
    let target = if crate::session::is_main(chat) { chat.to_string() } else { crate::session::active(app) };
    let result = run(app, &target, "", &mut messages, |event| emit(app, chat, event)).await;
    app.state::<Assistant>().chats.lock().unwrap().entry(chat.into()).or_default().messages = messages;
    result
}

// The reply and any error arrive as events, followed by a done event.
#[tauri::command]
pub async fn assistant_send(app: AppHandle, webview: Webview, text: String, images: Vec<String>) -> Result<(), String> {
    crate::plugins::controls(&webview)?;
    let chat = webview.label();
    emit(&app, chat, json!({"type": "user", "text": text, "images": images}));
    let result = send(&app, chat, text, images).await;
    if let Err(error) = &result {
        emit(&app, chat, json!({"type": "error", "text": error}));
    }
    emit(&app, chat, json!({"type": "done"}));
    result
}

// Carries out a request typed into the Algebra input of this window, outside of any chat.
// Returns the model's closing reply.
#[tauri::command]
pub async fn assistant_prompt(app: AppHandle, webview: Webview, text: String) -> Result<String, String> {
    crate::browser::trusted(&webview)?;
    let settings = settings(&app)?;
    if !available(&settings, &app) || settings["prompt"] != true {
        return Err("Natural language input is not available.".into());
    }
    let mut messages = vec![json!({"role": "user", "content": [{"type": "text", "text": text}]})];
    run(&app, webview.label(), &format!("\n\n{PROMPT}"), &mut messages, |_| {}).await?;
    let reply = messages.last().and_then(|message| message["content"].as_array()).into_iter().flatten();
    Ok(reply.filter_map(|block| block["text"].as_str()).collect())
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::async_runtime::Sender;
use tauri::{AppHandle, Manager, Webview};

use crate::exam;
use crate::tools::{call, label, tools};

const MODELS: [&str; 4] = ["claude-sonnet-5-5", "claude-haiku-5-5", "claude-opus-5-5", "claude-fable-5-1"];
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
const DEFAULT_URL: &str = "https://api.anthropic.com";
const VERSION: &str = "2026-10-21";
const MAX_STEPS: usize = 20;
const SYSTEM: &str = "You are an AI assistant that can help the user with using Locus, a math app, and with other excercises. \
Answer concisely. \
Use Markdown, and write math in LaTeX between $...$ or $$...$$.";

#[derive(Default)]
pub struct Assistant {
    // One conversation per chat, keyed by the label of its webview.
    messages: Mutex<HashMap<String, Vec<Value>>>,
    pub(crate) pending: Mutex<HashMap<u64, Sender<String>>>,
    pub(crate) next: AtomicU64,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("claude.json"))
}

fn settings(app: &AppHandle) -> Result<Value, String> {
    let saved = std::fs::read_to_string(settings_path(app)?)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_default();
    Ok(json!({
        "key": saved["key"].as_str().unwrap_or_default(),
        "model": saved["model"].as_str().filter(|m| MODELS.contains(m)).unwrap_or(MODELS[0]),
        "effort": saved["effort"].as_str().filter(|e| EFFORTS.contains(e)).unwrap_or("medium"),
        "url": saved["url"].as_str().filter(|u| !u.is_empty()).unwrap_or(DEFAULT_URL),
        "thinking": saved["thinking"].as_str().unwrap_or("true"),
        "instructions": saved["instructions"].as_str().unwrap_or_default(),
        "enabled": saved["enabled"].as_str().unwrap_or("false"),
    }))
}

// Whether the assistant is turned on and set up, and allowed in a running exam.
fn available(app: &AppHandle) -> Result<bool, String> {
    let settings = settings(app)?;
    Ok(settings["enabled"] == "true" && settings["key"] != "" && exam::assistant_enabled(app))
}

// Lets the app windows show or hide the assistant after a change.
pub fn refresh(app: &AppHandle) {
    for label in crate::session::labels(app) {
        if let Some(view) = app.get_webview(&label) {
            let _ = view.eval("window.locusAssistantRefresh && window.locusAssistantRefresh()");
        }
    }
}

// Returns the settings without the key itself, so it never reaches a webview.
#[tauri::command]
pub fn assistant_settings(app: AppHandle, webview: Webview) -> Result<Value, String> {
    crate::plugins::controls(&webview)?;
    let settings = settings(&app)?;
    Ok(json!({
        "key": settings["key"] != "",
        "model": settings["model"],
        "effort": settings["effort"],
        "url": settings["url"],
        "thinking": settings["thinking"] == "true",
        "instructions": settings["instructions"],
        "enabled": settings["enabled"] == "true",
        "available": available(&app)?,
    }))
}

// Checks the key with a cheap request, so a wrong key is caught on the setup page.
async fn check_key(url: &Value, key: &str) -> Result<(), String> {
    let response = reqwest::Client::new()
        .get(format!("{}/v1/models?limit=1", url.as_str().unwrap_or_default()))
        .header("x-api-key", key)
        .header("anthropic-version", VERSION)
        .send()
        .await
        .map_err(|_| "Could not reach Anthropic. Check the internet connection.")?;
    match response.status().as_u16() {
        200 => Ok(()),
        401 | 403 => Err("This API key was not accepted.".into()),
        _ => Err(error_message(response.json().await.unwrap_or_default())),
    }
}

fn error_message(data: Value) -> String {
    data["error"]["message"]
        .as_str()
        .unwrap_or("The request to Claude failed.")
        .into()
}

#[tauri::command]
pub async fn assistant_set(
    app: AppHandle,
    webview: Webview,
    name: String,
    value: String,
) -> Result<(), String> {
    crate::plugins::controls(&webview)?;
    let value = if name == "instructions" { value.trim() } else { value.trim().trim_end_matches('/') };
    let mut settings = settings(&app)?;
    if name == "key" && !value.is_empty() {
        check_key(&settings["url"], value).await?;
    }
    let valid = match name.as_str() {
        "key" => true,
        "url" => value.is_empty() || value.starts_with("http://") || value.starts_with("https://"),
        "model" => MODELS.contains(&value),
        "effort" => EFFORTS.contains(&value),
        "thinking" | "enabled" => matches!(value, "true" | "false"),
        "instructions" => true,
        _ => false,
    };
    if !valid {
        return Err("Invalid assistant setting.".into());
    }
    settings[name] = json!(value);
    let path = settings_path(&app)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, settings.to_string()).map_err(|e| e.to_string())?;
    refresh(&app);
    Ok(())
}

pub fn end(app: &AppHandle) {
    let state = app.state::<Assistant>();
    state.messages.lock().unwrap().clear();
    for label in crate::session::labels(app) {
        if let Some(view) = app.get_webview(&label) {
            let _ = view.eval("window.locusAssistantReset && window.locusAssistantReset()");
        }
    }
}

#[tauri::command]
pub fn assistant_reset(app: AppHandle, webview: Webview) -> Result<(), String> {
    crate::plugins::controls(&webview)?;
    app.state::<Assistant>()
        .messages
        .lock()
        .unwrap()
        .remove(webview.label());
    Ok(())
}

// Sends a reply event to the chat that asked, a plugin page or the assistant view in an app window.
fn emit(app: &AppHandle, chat: &str, event: Value) {
    if let Some(view) = app.get_webview(chat) {
        let _ = view.eval(format!(
            "window.assistantEvent && window.assistantEvent({event})"
        ));
    }
}

fn append(block: &mut Value, field: &str, text: &Value) {
    let text =
        block[field].as_str().unwrap_or_default().to_owned() + text.as_str().unwrap_or_default();
    block[field] = json!(text);
}

// Rebuilds the message from stream events and forwards text and thinking to the chat.
fn handle(app: &AppHandle, chat: &str, message: &mut Value, event: Value, thinking: bool) -> Result<(), String> {
    match event["type"].as_str().unwrap_or_default() {
        "content_block_start" => {
            let block = event["content_block"].clone();
            if block["type"] == "text" || (thinking && block["type"] == "thinking") {
                emit(app, chat, json!({"type": "block", "kind": block["type"]}));
            }
            message["content"].as_array_mut().unwrap().push(block);
        }
        "content_block_delta" => {
            let block =
                &mut message["content"][event["index"].as_u64().unwrap_or_default() as usize];
            let delta = &event["delta"];
            let field = match delta["type"].as_str().unwrap_or_default() {
                "text_delta" => "text",
                "thinking_delta" => "thinking",
                "signature_delta" => "signature",
                "input_json_delta" => "partial_json",
                _ => return Ok(()),
            };
            append(block, field, &delta[field]);
            if field == "text" || (thinking && field == "thinking") {
                emit(app, chat, json!({"type": "delta", "text": delta[field]}));
            }
        }
        "content_block_stop" => {
            let block =
                &mut message["content"][event["index"].as_u64().unwrap_or_default() as usize];
            if let Some(json) = block.as_object_mut().and_then(|b| b.remove("partial_json")) {
                block["input"] =
                    serde_json::from_str(json.as_str().unwrap_or("{}")).unwrap_or(json!({}));
            }
        }
        "message_delta" => message["stop_reason"] = event["delta"]["stop_reason"].clone(),
        "error" => return Err(error_message(event)),
        _ => {}
    }
    Ok(())
}

async fn complete(app: &AppHandle, chat: &str, target: &str, messages: &[Value]) -> Result<Value, String> {
    let settings = settings(app)?;
    let thinking = settings["thinking"] == "true";
    let instructions = settings["instructions"].as_str().unwrap_or_default();
    let system = if instructions.is_empty() {
        SYSTEM.to_owned()
    } else {
        format!("{SYSTEM}\n\nInstructions from the user:\n{instructions}")
    };
    let mut body = json!({
        "model": settings["model"],
        "max_tokens": 64000,
        "stream": true,
        "system": system,
        "tools": tools(app, target),
        "messages": messages,
        "thinking": {"type": "adaptive", "display": if thinking { "summarized" } else { "omitted" }},
        "output_config": {"effort": settings["effort"]},
    });
    // Haiku has no server-side fallback for refusals.
    if settings["model"] != "claude-haiku-5-5" {
        body["fallbacks"] = json!("default");
    }
    let mut response = reqwest::Client::new()
        .post(format!("{}/v1/messages", settings["url"].as_str().unwrap_or_default()))
        .header("x-api-key", settings["key"].as_str().unwrap_or_default())
        .header("anthropic-version", VERSION)
        .header("anthropic-beta", "server-side-fallback-2026-07-01")
        .json(&body)
        .send()
        .await
        .map_err(|_| "Could not reach Anthropic. Check the internet connection.")?;
    if !response.status().is_success() {
        return Err(error_message(response.json().await.unwrap_or_default()));
    }
    let mut message = json!({"content": []});
    let mut buffer = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "The connection to Anthropic was lost.")?
    {
        buffer.extend_from_slice(&chunk);
        while let Some(end) = buffer.windows(2).position(|w| w == b"\n\n") {
            let event: Vec<u8> = buffer.drain(..end + 2).collect();
            for data in String::from_utf8_lossy(&event)
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
            {
                let event = serde_json::from_str(data).map_err(|e| e.to_string())?;
                handle(app, chat, &mut message, event, thinking)?;
            }
        }
    }
    Ok(message)
}

async fn run(app: &AppHandle, chat: &str, messages: &mut Vec<Value>) -> Result<(), String> {
    // A chat in an app window works on that window, a plugin's chat on the window used last.
    let target = if crate::session::is_main(chat) { chat.to_string() } else { crate::session::active(app) };
    for _ in 0..MAX_STEPS {
        let response = complete(app, chat, &target, messages).await?;
        let content = response["content"].as_array().cloned().unwrap_or_default();
        messages.push(json!({"role": "assistant", "content": content}));
        match response["stop_reason"].as_str() {
            Some("tool_use") => {}
            Some("refusal") => return Err("Claude declined to answer this.".into()),
            Some("max_tokens") => return Err("The answer was too long and was cut off.".into()),
            _ => return Ok(()),
        }
        let mut results = Vec::new();
        for block in content.iter().filter(|block| block["type"] == "tool_use") {
            let name = block["name"].as_str().unwrap_or_default();
            let result = call(app, &target, name, &block["input"]).await;
            emit(app, chat, json!({"type": "tool", "label": label(name, &block["input"])}));
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
    Err("Claude stopped after too many steps.".into())
}

#[tauri::command]
pub async fn assistant_send(
    app: AppHandle,
    webview: Webview,
    text: String,
    images: Vec<String>,
) -> Result<(), String> {
    crate::plugins::controls(&webview)?;
    if !available(&app)? {
        return Err("The AI assistant is not available.".into());
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
    let chat = webview.label();
    let mut messages = app
        .state::<Assistant>()
        .messages
        .lock()
        .unwrap()
        .get(chat)
        .cloned()
        .unwrap_or_default();
    messages.push(json!({"role": "user", "content": content}));
    let result = run(&app, chat, &mut messages).await;
    app.state::<Assistant>()
        .messages
        .lock()
        .unwrap()
        .insert(chat.into(), messages);
    result
}

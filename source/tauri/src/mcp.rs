use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::browser;
use crate::tools;

#[derive(Default)]
pub struct Mcp(Mutex<Option<Arc<Server>>>);

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("mcp.json"))
}

fn settings(app: &AppHandle) -> Result<Value, String> {
    let saved = std::fs::read_to_string(settings_path(app)?)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_default();
    Ok(json!({
        "enabled": saved["enabled"].as_bool().unwrap_or(false),
        "port": saved["port"].as_u64().unwrap_or(3790),
    }))
}

fn stop(app: &AppHandle) {
    if let Some(server) = app.state::<Mcp>().0.lock().unwrap().take() {
        server.unblock();
    }
}

// Starts the server on 127.0.0.1 if it is enabled, replacing a running one.
pub fn start(app: &AppHandle) -> Result<(), String> {
    stop(app);
    let settings = settings(app)?;
    if settings["enabled"] != true {
        return Ok(());
    }
    let port = settings["port"].as_u64().unwrap_or_default() as u16;
    let server = Arc::new(
        Server::http(("127.0.0.1", port))
            .map_err(|e| format!("Could not start the MCP server on port {port}: {e}"))?,
    );
    *app.state::<Mcp>().0.lock().unwrap() = Some(server.clone());
    let app = app.clone();
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let app = app.clone();
            std::thread::spawn(move || respond(&app, request));
        }
    });
    Ok(())
}

fn respond(app: &AppHandle, mut request: Request) {
    // Browsers always send an Origin header, so web pages cannot use the server.
    let from_browser = request.headers().iter().any(|h| h.field.equiv("Origin"));
    let status = if from_browser {
        403
    } else if request.url() != "/mcp" {
        404
    } else if request.method() != &Method::Post {
        405
    } else {
        0
    };
    if status != 0 {
        let _ = request.respond(Response::empty(status));
        return;
    }
    let mut body = String::new();
    let _ = request.as_reader().read_to_string(&mut body);
    let Ok(message) = serde_json::from_str::<Value>(&body) else {
        let _ = request.respond(Response::empty(400));
        return;
    };
    // Notifications get no answer.
    if message["id"].is_null() {
        let _ = request.respond(Response::empty(202));
        return;
    }
    let reply = match handle(app, &message) {
        Some(result) => json!({"jsonrpc": "2.0", "id": message["id"], "result": result}),
        None => json!({
            "jsonrpc": "2.0",
            "id": message["id"],
            "error": {"code": -32601, "message": "Method not found"}
        }),
    };
    let header = Header::from_bytes("Content-Type", "application/json").unwrap();
    let _ = request.respond(Response::from_string(reply.to_string()).with_header(header));
}

fn handle(app: &AppHandle, message: &Value) -> Option<Value> {
    let params = &message["params"];
    match message["method"].as_str()? {
        // Only tools are offered, and they work the same in every protocol version.
        "initialize" => Some(json!({
            "protocolVersion": params["protocolVersion"],
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "locus", "version": app.package_info().version.to_string()}
        })),
        "ping" => Some(json!({})),
        "tools/list" => {
            let list: Vec<Value> = tools::tools(app, &crate::session::active(app))
                .as_array()?
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool["name"],
                        "description": tool["description"],
                        "inputSchema": tool["input_schema"]
                    })
                })
                .collect();
            Some(json!({"tools": list}))
        }
        "tools/call" => Some(call(app, params)),
        _ => None,
    }
}

fn call(app: &AppHandle, params: &Value) -> Value {
    if app.state::<crate::Lockdown>().0.load(Ordering::SeqCst) {
        return error("Locus does not allow MCP tools during an exam.".into());
    }
    let name = params["name"].as_str().unwrap_or_default();
    let input = if params["arguments"].is_object() {
        params["arguments"].clone()
    } else {
        json!({})
    };
    // Tools act on the window used last.
    let target = crate::session::active(app);
    match tauri::async_runtime::block_on(tools::call(app, &target, name, &input)) {
        Ok(Value::Array(blocks)) => json!({"content": blocks.iter().map(block).collect::<Vec<_>>()}),
        Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
        Err(message) => error(message),
    }
}

// Turns an Anthropic API content block into an MCP one.
fn block(block: &Value) -> Value {
    if block["type"] == "image" {
        json!({"type": "image", "data": block["source"]["data"], "mimeType": block["source"]["media_type"]})
    } else {
        block.clone()
    }
}

fn error(message: String) -> Value {
    json!({"content": [{"type": "text", "text": message}], "isError": true})
}

#[tauri::command]
pub fn mcp_settings(app: AppHandle, webview: Webview) -> Result<Value, String> {
    browser::trusted(&webview)?;
    let mut settings = settings(&app)?;
    settings["running"] = json!(app.state::<Mcp>().0.lock().unwrap().is_some());
    Ok(settings)
}

#[tauri::command]
pub fn mcp_set(app: AppHandle, webview: Webview, enabled: bool, port: u16) -> Result<(), String> {
    browser::trusted(&webview)?;
    if port < 1024 {
        return Err("Choose a port from 1024 to 65535.".into());
    }
    let path = settings_path(&app)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, json!({"enabled": enabled, "port": port}).to_string())
        .map_err(|e| e.to_string())?;
    start(&app)
}

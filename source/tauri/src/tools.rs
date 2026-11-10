use std::sync::atomic::Ordering;

use serde_json::{json, Value};
use tauri::async_runtime::channel;
use tauri::{AppHandle, Manager, Webview};

use crate::assistant::Assistant;
use crate::browser;

// Checks the teacher's CAS setting and waits until the CAS engine has loaded.
pub const CAS_READY: &str =
    "if (!window.ggbAssistantCasEnabled()) throw 'The CAS is disabled in this exam.';
    for (let i = 0; i < 60 && ggbApplet.evalCommandCAS('1+1') === '?'; i++)
        await new Promise((resolve) => setTimeout(resolve, 500));
    ";

// Each tool has a definition sent to Claude, a label for the chat and a matching arm in `call`.
// The tools for the window with this label.
pub fn tools(app: &AppHandle, target: &str) -> Value {
    let mut tools = json!([
        {
            "name": "locus_command",
            "description": "Runs a command in the Algebra input, as if the student typed it there, so created objects appear in the Algebra and Graphics views, e.g. \"f(x)=x^2\" or \"A=(1,2)\". Returns the labels of created objects.",
            "input_schema": {
                "type": "object",
                "properties": {"command": {"type": "string"}},
                "required": ["command"]
            }
        },
        {
            "name": "locus_view",
            "description": "Shows or hides one of the app's views: algebra (the Algebra view with its input bar), graphics or cas.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "view": {"type": "string", "enum": ["algebra", "graphics", "cas"]},
                    "visible": {"type": "boolean"}
                },
                "required": ["view", "visible"]
            }
        },
        {
            "name": "cas_evaluate",
            "description": "Computes with the CAS without changing the student's construction, e.g. Solve(x^2=4), Intersect(x^2, 2x), Factor(x^2-1), Derivative(sin(x)) or Integral(x^2, 0, 1). Returns the result.",
            "input_schema": {
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }
        },
        {
            "name": "cas_add_row",
            "description": "Adds a row to the student's CAS view and returns its output, e.g. f(x):=x^2 or Solve(f(x)=4). Objects defined there are not drawn in the Graphics view.",
            "input_schema": {
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }
        },
        {
            "name": "locus_screenshot",
            "description": "Returns a picture of the student's Graphics view and its visible x and y range.",
            "input_schema": {"type": "object", "properties": {}}
        },
        {
            "name": "locus_viewport",
            "description": "Sets the visible area of the Graphics view.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "xmin": {"type": "number"},
                    "xmax": {"type": "number"},
                    "ymin": {"type": "number"},
                    "ymax": {"type": "number"}
                },
                "required": ["xmin", "xmax", "ymin", "ymax"]
            }
        },
        {
            "name": "locus_objects",
            "description": "Lists all objects in the student's construction with their type and value.",
            "input_schema": {"type": "object", "properties": {}}
        }
    ]);
    // Only Classic has a CAS view and Classic perspectives, and Scientific has no Graphics view.
    let mode = crate::session::mode(app, target);
    let mut hidden = vec![];
    if mode != crate::session::Mode::Classic {
        hidden.extend(["locus_view", "cas_add_row"]);
    }
    if mode == crate::session::Mode::Scientific {
        hidden.extend(["locus_screenshot", "locus_viewport"]);
    }
    if let Some(list) = tools.as_array_mut() {
        list.retain(|tool| !hidden.iter().any(|name| tool["name"] == *name));
        list.extend(crate::plugins::tools(app));
    }
    tools
}

pub fn label(name: &str, input: &Value) -> String {
    match name {
        "locus_command" => format!("Ran {}", input["command"].as_str().unwrap_or_default()),
        "locus_objects" => "Read the construction".into(),
        "locus_screenshot" => "Looked at the Graphics view".into(),
        "locus_viewport" => format!(
            "Moved the view to x {} to {}, y {} to {}",
            input["xmin"], input["xmax"], input["ymin"], input["ymax"]
        ),
        "locus_view" => format!(
            "{} the {} view",
            if input["visible"] == true {
                "Showed"
            } else {
                "Hid"
            },
            match input["view"].as_str() {
                Some("cas") => "CAS",
                Some("algebra") => "Algebra",
                _ => "Graphics",
            }
        ),
        "cas_evaluate" => format!("Computed {}", input["input"].as_str().unwrap_or_default()),
        "cas_add_row" => format!(
            "Added CAS row {}",
            input["input"].as_str().unwrap_or_default()
        ),
        _ => name.into(),
    }
}

// Returns tool result content: text, or image and text blocks for the screenshot.
pub async fn call(app: &AppHandle, target: &str, name: &str, input: &Value) -> Result<Value, String> {
    if name == "locus_screenshot" {
        return screenshot(app, target, input).await;
    }
    // Tool results are text, so plugin answers other than text are sent as JSON.
    if let Some(result) = crate::plugins::call_tool(app, target, name, input).await {
        return result.map(|value| if value.is_string() { value } else { Value::from(value.to_string()) });
    }
    text(app, target, name, input).await.map(Value::from)
}

async fn screenshot(app: &AppHandle, target: &str, input: &Value) -> Result<Value, String> {
    let result = geogebra(
        app,
        target,
        input,
        "const view = JSON.parse(ggbApplet.getViewProperties(1));
         const xMax = view.xMin + view.width * view.invXscale;
         const yMax = view.yMin + view.height * view.invYscale;
         return JSON.stringify({
             png: ggbApplet.getPNGBase64(1, false, 72, false, false),
             range: `Visible area: x from ${view.xMin} to ${xMax}, y from ${view.yMin} to ${yMax}.`,
         });",
    )
    .await?;
    let shot: Value = serde_json::from_str(&result).map_err(|e| e.to_string())?;
    Ok(json!([
        {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": shot["png"]}},
        {"type": "text", "text": shot["range"]}
    ]))
}

async fn text(app: &AppHandle, target: &str, name: &str, input: &Value) -> Result<String, String> {
    match name {
        "locus_command" => {
            geogebra(
                app,
                target,
                input,
                "ggbApplet.setErrorDialogsActive(false);
                 try { return await ggbApplet.asyncEvalCommandGetLabels(args.command) || 'No new objects were created. The command may have failed or only changed existing objects.'; }
                 finally { ggbApplet.setErrorDialogsActive(true); }",
            )
            .await
        }
        "locus_objects" => {
            geogebra(
                app,
                target,
                input,
                "return ggbApplet.getAllObjectNames()
                     .map(n => `${n} (${ggbApplet.getObjectType(n)}): ${ggbApplet.getValueString(n)}`)
                     .join('\\n') || 'The construction is empty.';",
            )
            .await
        }
        "locus_view" => {
            geogebra(
                app,
                target,
                input,
                "const code = { algebra: 'A', graphics: 'G', cas: 'C' }[args.view];
                 if (!code) throw 'Unknown view.';
                 if (code === 'C' && !window.ggbAssistantCasEnabled()) throw 'The CAS is disabled in this exam.';
                 ggbApplet.setPerspective((args.visible ? '+' : '-') + code);
                 return 'Done.';",
            )
            .await
        }
        "locus_viewport" => {
            geogebra(
                app,
                target,
                input,
                "if (!(args.xmin < args.xmax && args.ymin < args.ymax)) throw 'The minimum must be below the maximum.';
                 ggbApplet.setCoordSystem(args.xmin, args.xmax, args.ymin, args.ymax);
                 return 'Done.';",
            )
            .await
        }
        "cas_evaluate" => {
            let body = format!("{CAS_READY}return ggbApplet.evalCommandCAS(args.input);");
            geogebra(app, target, input, &body).await
        }
        "cas_add_row" => {
            let body = format!("{CAS_READY}return window.ggbAssistantCasRow(args.input);");
            geogebra(app, target, input, &body).await
        }
        _ => Err(format!("Unknown tool: {name}")),
    }
}

// Runs a script in the GeoGebra page and waits for it to report its result.
pub async fn geogebra(app: &AppHandle, target: &str, args: &Value, body: &str) -> Result<String, String> {
    let state = app.state::<Assistant>();
    let id = state.next.fetch_add(1, Ordering::SeqCst);
    let (tx, mut rx) = channel(1);
    state.pending.lock().unwrap().insert(id, tx);
    let view = app.get_webview(target).ok_or("Locus window missing.")?;
    view.eval(format!(
        "(async () => {{ const args = {args}; let result;
            try {{ result = String(await (async () => {{ {body} }})()); }} catch (e) {{ result = 'Error: ' + e; }}
            window.__TAURI__.core.invoke('assistant_tool_result', {{ id: {id}, result }});
        }})()"
    ))
    .map_err(|e| e.to_string())?;
    let result = rx.recv().await.ok_or("Locus did not respond.")?;
    match result.strip_prefix("Error: ") {
        Some(error) => Err(error.into()),
        None => Ok(result),
    }
}

#[tauri::command]
pub async fn assistant_tool_result(
    app: AppHandle,
    webview: Webview,
    id: u64,
    result: String,
) -> Result<(), String> {
    browser::trusted(&webview)?;
    let tx = app.state::<Assistant>().pending.lock().unwrap().remove(&id);
    if let Some(tx) = tx {
        let _ = tx.send(result).await;
    }
    Ok(())
}

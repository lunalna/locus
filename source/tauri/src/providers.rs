use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::RequestBuilder;
use serde_json::{json, Value};

#[derive(PartialEq)]
pub enum Protocol {
    Anthropic,
    OpenAi,
}

pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub protocol: Protocol,
    pub needs_key: bool,
}

const fn openai(id: &'static str, name: &'static str, url: &'static str, needs_key: bool) -> Provider {
    Provider { id, name, url, protocol: Protocol::OpenAi, needs_key }
}

pub const PROVIDERS: [Provider; 10] = [
    Provider { id: "anthropic", name: "Anthropic", url: "https://api.anthropic.com", protocol: Protocol::Anthropic, needs_key: true },
    openai("openai", "OpenAI", "https://api.openai.com/v1", true),
    openai("openrouter", "OpenRouter", "https://openrouter.ai/api/v1", true),
    openai("google", "Google Gemini", "https://generativelanguage.googleapis.com/v1beta/openai", true),
    openai("mistral", "Mistral", "https://api.mistral.ai/v1", true),
    openai("groq", "Groq", "https://api.groq.com/openai/v1", true),
    openai("deepseek", "DeepSeek", "https://api.deepseek.com/v1", true),
    openai("xai", "xAI", "https://api.x.ai/v1", true),
    openai("ollama", "Ollama", "http://localhost:11434/v1", false),
    openai("custom", "Custom (OpenAI compatible)", "", false),
];

pub const ANTHROPIC_MODELS: [&str; 4] = ["claude-sonnet-5-5", "claude-haiku-5-5", "claude-opus-5-5", "claude-fable-5-1"];
const ANTHROPIC_EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
const OPENAI_EFFORTS: [&str; 4] = ["default", "low", "medium", "high"];
const VERSION: &str = "2026-10-21";

pub fn find(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|provider| provider.id == id)
}

impl Provider {
    pub fn efforts(&self) -> &'static [&'static str] {
        match self.protocol {
            Protocol::Anthropic => &ANTHROPIC_EFFORTS,
            Protocol::OpenAi => &OPENAI_EFFORTS,
        }
    }

    pub fn default_effort(&self) -> &'static str {
        if self.protocol == Protocol::Anthropic { "medium" } else { "default" }
    }

    fn unreachable(&self) -> String {
        format!("Could not reach {}. Check the internet connection and the URL.", self.name)
    }

    fn error(&self, data: &Value) -> String {
        // Gemini wraps its errors in a list.
        let error = if data.is_array() { &data[0]["error"] } else { &data["error"] };
        error["message"]
            .as_str()
            .or(error.as_str())
            .map(String::from)
            .unwrap_or_else(|| format!("The request to {} failed.", self.name))
    }

    fn headers(&self, request: RequestBuilder, key: &str) -> RequestBuilder {
        match self.protocol {
            Protocol::Anthropic => request.header("x-api-key", key).header("anthropic-version", VERSION),
            Protocol::OpenAi if key.is_empty() => request,
            Protocol::OpenAi => request.bearer_auth(key),
        }
    }

    // The models offered with this key. Anthropic's list is fixed to the models the request below supports.
    pub async fn models(&self, url: &str, key: &str) -> Result<Vec<String>, String> {
        let path = if self.protocol == Protocol::Anthropic { "/v1/models?limit=1" } else { "/models" };
        let response = self
            .headers(reqwest::Client::new().get(format!("{url}{path}")), key)
            .send()
            .await
            .map_err(|_| self.unreachable())?;
        match response.status().as_u16() {
            200 => {}
            401 | 403 => return Err("This API key was not accepted.".into()),
            _ => return Err(self.error(&response.json().await.unwrap_or_default())),
        }
        if self.protocol == Protocol::Anthropic {
            return Ok(ANTHROPIC_MODELS.map(String::from).to_vec());
        }
        let data: Value = response.json().await.unwrap_or_default();
        let mut models: Vec<String> = data["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|model| model["id"].as_str().map(String::from))
            .collect();
        models.sort();
        Ok(models)
    }

    // Streams one reply, calling `emit` with block and delta events for the chat.
    // Returns it as an Anthropic message, which is the format the conversation is kept in.
    pub async fn complete(
        &self,
        settings: &Value,
        system: &str,
        tools: &Value,
        messages: &[Value],
        emit: impl FnMut(Value),
    ) -> Result<Value, String> {
        let url = settings["url"].as_str().unwrap_or_default();
        let key = settings["key"].as_str().unwrap_or_default();
        let thinking = settings["thinking"] == true;
        let model = &settings["model"];
        let effort = settings["effort"].as_str().unwrap_or_default();
        let client = reqwest::Client::new();
        let request = match self.protocol {
            Protocol::Anthropic => {
                let mut body = json!({
                    "model": model,
                    "max_tokens": 64000,
                    "stream": true,
                    "system": system,
                    "tools": tools,
                    "messages": messages,
                    "thinking": {"type": "adaptive", "display": if thinking { "summarized" } else { "omitted" }},
                    "output_config": {"effort": effort},
                });
                // Haiku has no server-side fallback for refusals.
                if model != "claude-haiku-5-5" {
                    body["fallbacks"] = json!("default");
                }
                client
                    .post(format!("{url}/v1/messages"))
                    .header("anthropic-beta", "server-side-fallback-2026-07-01")
                    .json(&body)
            }
            Protocol::OpenAi => {
                let tools: Vec<Value> = tools
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|tool| {
                        json!({"type": "function", "function": {
                            "name": tool["name"], "description": tool["description"], "parameters": tool["input_schema"]
                        }})
                    })
                    .collect();
                let mut body = json!({"model": model, "stream": true, "tools": tools, "messages": openai_messages(system, messages)});
                if effort != "default" {
                    if self.id == "openrouter" {
                        body["reasoning"] = json!({"effort": effort});
                    } else {
                        body["reasoning_effort"] = json!(effort);
                    }
                }
                client.post(format!("{url}/chat/completions")).json(&body)
            }
        };
        let response = self.headers(request, key).send().await.map_err(|_| self.unreachable())?;
        if !response.status().is_success() {
            return Err(self.error(&response.json().await.unwrap_or_default()));
        }
        match self.protocol {
            Protocol::Anthropic => self.read_anthropic(response, thinking, emit).await,
            Protocol::OpenAi => self.read_openai(response, thinking, emit).await,
        }
    }

    // Calls `each` with the data of every server-sent event.
    async fn read(&self, mut response: reqwest::Response, mut each: impl FnMut(Value) -> Result<(), String>) -> Result<(), String> {
        let mut buffer = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| format!("The connection to {} was lost.", self.name))?
        {
            buffer.extend_from_slice(&chunk);
            while let Some(end) = buffer.iter().position(|&byte| byte == b'\n') {
                let line: Vec<u8> = buffer.drain(..=end).collect();
                let line = String::from_utf8_lossy(&line);
                let Some(data) = line.trim().strip_prefix("data:").map(str::trim) else { continue };
                if data != "[DONE]" {
                    each(serde_json::from_str(data).map_err(|e| e.to_string())?)?;
                }
            }
        }
        Ok(())
    }

    async fn read_anthropic(&self, response: reqwest::Response, thinking: bool, mut emit: impl FnMut(Value)) -> Result<Value, String> {
        let mut message = json!({"content": []});
        self.read(response, |event| {
            match event["type"].as_str().unwrap_or_default() {
                "content_block_start" => {
                    let block = event["content_block"].clone();
                    if block["type"] == "text" || (thinking && block["type"] == "thinking") {
                        emit(json!({"type": "block", "kind": block["type"]}));
                    }
                    message["content"].as_array_mut().unwrap().push(block);
                }
                "content_block_delta" => {
                    let block = &mut message["content"][event["index"].as_u64().unwrap_or_default() as usize];
                    let delta = &event["delta"];
                    let field = match delta["type"].as_str().unwrap_or_default() {
                        "text_delta" => "text",
                        "thinking_delta" => "thinking",
                        "signature_delta" => "signature",
                        "input_json_delta" => "partial_json",
                        _ => return Ok(()),
                    };
                    append(block, field, delta[field].as_str().unwrap_or_default());
                    if field == "text" || (thinking && field == "thinking") {
                        emit(json!({"type": "delta", "text": delta[field]}));
                    }
                }
                "content_block_stop" => {
                    let block = &mut message["content"][event["index"].as_u64().unwrap_or_default() as usize];
                    if let Some(json) = block.as_object_mut().and_then(|b| b.remove("partial_json")) {
                        block["input"] = serde_json::from_str(json.as_str().unwrap_or("{}")).unwrap_or(json!({}));
                    }
                }
                "message_delta" => message["stop_reason"] = event["delta"]["stop_reason"].clone(),
                "error" => return Err(self.error(&event)),
                _ => {}
            }
            Ok(())
        })
        .await?;
        Ok(message)
    }

    async fn read_openai(&self, response: reqwest::Response, thinking: bool, mut emit: impl FnMut(Value)) -> Result<Value, String> {
        let mut text = String::new();
        let mut calls: Vec<Value> = Vec::new();
        let mut finish = String::new();
        // The kind of the block shown last, so a new one starts when text follows reasoning.
        let mut shown = "";
        self.read(response, |event| {
            if event["error"].is_object() {
                return Err(self.error(&event));
            }
            let choice = &event["choices"][0];
            let delta = &choice["delta"];
            let reasoning = delta["reasoning_content"].as_str().or(delta["reasoning"].as_str()).unwrap_or_default();
            for (kind, part) in [("thinking", reasoning), ("text", delta["content"].as_str().unwrap_or_default())] {
                if part.is_empty() || (kind == "thinking" && !thinking) {
                    continue;
                }
                if shown != kind {
                    emit(json!({"type": "block", "kind": kind}));
                    shown = kind;
                }
                emit(json!({"type": "delta", "text": part}));
            }
            text += delta["content"].as_str().unwrap_or_default();
            for call in delta["tool_calls"].as_array().into_iter().flatten() {
                let index = call["index"].as_u64().map_or(calls.len().saturating_sub(1), |i| i as usize);
                if index >= calls.len() {
                    calls.resize(index + 1, json!({}));
                }
                for (field, part) in [("id", &call["id"]), ("name", &call["function"]["name"]), ("arguments", &call["function"]["arguments"])] {
                    append(&mut calls[index], field, part.as_str().unwrap_or_default());
                }
            }
            if let Some(reason) = choice["finish_reason"].as_str() {
                finish = reason.into();
            }
            Ok(())
        })
        .await?;
        let mut content = Vec::new();
        if !text.is_empty() {
            content.push(json!({"type": "text", "text": text}));
        }
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        for (index, call) in calls.iter().enumerate() {
            // Some providers leave the id out; it must be unique in the conversation.
            let id = call["id"].as_str().filter(|id| !id.is_empty()).map_or(format!("call_{stamp}_{index}"), String::from);
            let input = serde_json::from_str(call["arguments"].as_str().unwrap_or_default()).unwrap_or(json!({}));
            content.push(json!({"type": "tool_use", "id": id, "name": call["name"], "input": input}));
        }
        // Some providers finish with "stop" even after tool calls.
        let stop = if !calls.is_empty() {
            "tool_use"
        } else {
            match finish.as_str() {
                "length" => "max_tokens",
                "content_filter" => "refusal",
                _ => "end_turn",
            }
        };
        Ok(json!({"content": content, "stop_reason": stop}))
    }
}

fn append(block: &mut Value, field: &str, text: &str) {
    let text = block[field].as_str().unwrap_or_default().to_owned() + text;
    block[field] = json!(text);
}

fn image_part(block: &Value) -> Value {
    let source = &block["source"];
    let url = format!("data:{};base64,{}", source["media_type"].as_str().unwrap_or_default(), source["data"].as_str().unwrap_or_default());
    json!({"type": "image_url", "image_url": {"url": url}})
}

// Turns the conversation into Chat Completions messages. Thinking is left out.
fn openai_messages(system: &str, messages: &[Value]) -> Vec<Value> {
    let mut result = vec![json!({"role": "system", "content": system})];
    for message in messages {
        let blocks = message["content"].as_array().cloned().unwrap_or_default();
        if message["role"] == "assistant" {
            let text: String = blocks.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect();
            let calls: Vec<Value> = blocks
                .iter()
                .filter(|b| b["type"] == "tool_use")
                .map(|b| json!({"id": b["id"], "type": "function", "function": {"name": b["name"], "arguments": b["input"].to_string()}}))
                .collect();
            let mut reply = json!({"role": "assistant", "content": text});
            if !calls.is_empty() {
                reply["tool_calls"] = json!(calls);
            }
            result.push(reply);
            continue;
        }
        let mut parts = Vec::new();
        for block in &blocks {
            match block["type"].as_str().unwrap_or_default() {
                "tool_result" => {
                    let items = match &block["content"] {
                        Value::Array(items) => items.clone(),
                        text => vec![json!({"type": "text", "text": text})],
                    };
                    let text: Vec<&str> = items.iter().filter_map(|item| item["text"].as_str()).collect();
                    result.push(json!({"role": "tool", "tool_call_id": block["tool_use_id"], "content": text.join("\n")}));
                    // Tool messages only take text, so pictures follow in a user message.
                    parts.extend(items.iter().filter(|item| item["type"] == "image").map(image_part));
                }
                "image" => parts.push(image_part(block)),
                _ => parts.push(json!({"type": "text", "text": block["text"]})),
            }
        }
        if !parts.is_empty() {
            result.push(json!({"role": "user", "content": parts}));
        }
    }
    result
}

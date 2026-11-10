// The AI sidebar for the web version. The browser talks to the provider's API directly with the user's key.
import { call, definitions, label } from './tools.js';

const KEY = 'locus.ai';
const HISTORY = 'locus.ai.chat';
const ANTHROPIC_MODELS = ['claude-sonnet-5-5', 'claude-haiku-5-5', 'claude-opus-5-5', 'claude-fable-5-1'];
const ANTHROPIC_EFFORTS = ['low', 'medium', 'high', 'xhigh', 'max'];
const OPENAI_EFFORTS = ['default', 'low', 'medium', 'high'];
const VERSION = '2026-10-21';
const MAX_STEPS = 20;
const SYSTEM =
    'You are an AI assistant that can help the user with using Locus, a math app, and with other excercises. ' +
    'Answer concisely. ' +
    'Use Markdown, and write math in LaTeX between $...$ or $$...$$.';
const PROMPT =
    'The user typed this request into the Algebra input instead of a command. ' +
    'Carry it out with the tools, without asking questions. ' +
    'Then reply with one short plain sentence without Markdown or LaTeX, or with the answer if it was a question.';

const openai = (id, name, url, needsKey = true) => ({ id, name, url, anthropic: false, needsKey });
const PROVIDERS = [
    { id: 'anthropic', name: 'Anthropic', url: 'https://api.anthropic.com', anthropic: true, needsKey: true },
    openai('openai', 'OpenAI', 'https://api.openai.com/v1'),
    openai('openrouter', 'OpenRouter', 'https://openrouter.ai/api/v1'),
    openai('google', 'Google Gemini', 'https://generativelanguage.googleapis.com/v1beta/openai'),
    openai('mistral', 'Mistral', 'https://api.mistral.ai/v1'),
    openai('groq', 'Groq', 'https://api.groq.com/openai/v1'),
    openai('deepseek', 'DeepSeek', 'https://api.deepseek.com/v1'),
    openai('xai', 'xAI', 'https://api.x.ai/v1'),
    openai('ollama', 'Ollama', 'http://localhost:11434/v1', false),
    openai('custom', 'Custom (OpenAI compatible)', '', false),
];

const find = (id) => PROVIDERS.find((provider) => provider.id === id);
const efforts = (provider) => (provider.anthropic ? ANTHROPIC_EFFORTS : OPENAI_EFFORTS);
const saved = () => JSON.parse(localStorage.getItem(KEY) || '{}');
// Providers are only kept once the user added them, with their key and URL.
const added = (data, provider) => Boolean(data.providers?.[provider.id]);
const keyOf = (data, provider) => data.providers?.[provider.id]?.key ?? '';
const urlOf = (data, provider) => data.providers?.[provider.id]?.url || provider.url;
const ready = (data, provider) =>
    added(data, provider) && urlOf(data, provider) !== '' && (!provider.needsKey || keyOf(data, provider) !== '');
const validUrl = (url) => !url || url.startsWith('http://') || url.startsWith('https://');
const entry = (data, provider) => ((data.providers ??= {})[provider.id] ??= { key: '', url: '' });

// The chat's settings, with defaults for anything missing or no longer valid.
function settings() {
    const data = saved();
    const provider = find(data.provider) ?? PROVIDERS[0];
    let model = data.model ?? '';
    if (provider.anthropic && !ANTHROPIC_MODELS.includes(model)) model = ANTHROPIC_MODELS[0];
    return {
        provider,
        enabled: data.enabled ?? true,
        sidebar: data.sidebar ?? false,
        prompt: data.prompt ?? true,
        key: keyOf(data, provider),
        url: urlOf(data, provider),
        ready: ready(data, provider),
        model,
        effort: efforts(provider).includes(data.effort) ? data.effort : provider.anthropic ? 'medium' : 'default',
        thinking: data.thinking ?? true,
        instructions: data.instructions ?? '',
    };
}

const isAvailable = (current) => current.enabled && current.ready && current.model !== '';

// Returns the settings without the keys themselves, like the desktop app.
export function publicSettings() {
    const current = settings();
    const data = saved();
    return {
        ...current,
        provider: current.provider.id,
        key: current.key !== '',
        available: isAvailable(current),
        providers: PROVIDERS.map((provider) => ({
            id: provider.id,
            name: provider.name,
            url: urlOf(data, provider),
            defaultUrl: provider.url,
            added: added(data, provider),
            key: keyOf(data, provider) !== '',
            needsKey: provider.needsKey,
            ready: ready(data, provider),
            efforts: efforts(provider),
        })),
    };
}

function headers(provider, key) {
    if (provider.anthropic) return { 'x-api-key': key, 'anthropic-version': VERSION, 'anthropic-dangerous-direct-browser-access': 'true' };
    return key ? { authorization: `Bearer ${key}` } : {};
}

function errorMessage(provider, data) {
    // Gemini wraps its errors in a list.
    const error = Array.isArray(data) ? data[0]?.error : data?.error;
    return error?.message ?? (typeof error === 'string' ? error : `The request to ${provider.name} failed.`);
}

async function request(provider, url, options) {
    const response = await fetch(url, options).catch(() => {
        throw `Could not reach ${provider.name}. Check the internet connection and the URL.`;
    });
    if (response.status === 401 || response.status === 403) throw 'This API key was not accepted.';
    if (!response.ok) throw errorMessage(provider, await response.json().catch(() => null));
    return response;
}

// The models offered with this key. Anthropic's list is fixed to the models the request below supports.
async function listModels(provider, url, key) {
    const path = provider.anthropic ? '/v1/models?limit=1' : '/models';
    const response = await request(provider, url + path, { headers: headers(provider, key) });
    if (provider.anthropic) return ANTHROPIC_MODELS;
    const data = await response.json().catch(() => ({}));
    return (data.data ?? []).map((model) => model.id).sort();
}

export function models({ provider: id }) {
    const data = saved();
    const provider = find(id);
    if (!provider) throw 'Unknown provider.';
    if (!ready(data, provider)) throw `Add ${provider.name} under Model providers first.`;
    return listModels(provider, urlOf(data, provider), keyOf(data, provider));
}

export async function set({ name, value, provider: id }) {
    value = name === 'instructions' ? value.trim() : value.trim().replace(/\/+$/, '');
    const data = saved();
    const target = id ? find(id) : (find(data.provider) ?? PROVIDERS[0]);
    if (!target) throw 'Unknown provider.';
    if (name === 'key') {
        if (value) await listModels(target, urlOf(data, target), value);
        entry(data, target).key = value;
    } else if (name === 'url' && validUrl(value)) {
        entry(data, target).url = value;
    } else if (name === 'remove') {
        delete data.providers?.[target.id];
    } else if (name === 'provider' && find(value)) {
        // Another provider has other models and efforts.
        Object.assign(data, { provider: value, model: '', effort: '' });
    } else if (['model', 'effort', 'instructions'].includes(name)) {
        data[name] = value;
    } else if (['enabled', 'sidebar', 'prompt', 'thinking'].includes(name) && (value === 'true' || value === 'false')) {
        data[name] = value === 'true';
    } else {
        throw 'Invalid AI setting.';
    }
    localStorage.setItem(KEY, JSON.stringify(data));
    window.locusAssistantRefresh?.();
}

// Adds a provider, or changes its key and URL, once they work. Returns its models.
export async function addProvider({ provider: id, key, url }) {
    const provider = find(id);
    if (!provider) throw 'Unknown provider.';
    key = key.trim();
    url = url.trim().replace(/\/+$/, '');
    if (!validUrl(url)) throw 'The URL must start with http:// or https://.';
    if (!(url || provider.url)) throw 'Enter the URL of the service.';
    if (provider.needsKey && !key) throw 'Enter an API key.';
    const list = await listModels(provider, url || provider.url, key);
    const data = saved();
    (data.providers ??= {})[provider.id] = { key, url };
    localStorage.setItem(KEY, JSON.stringify(data));
    window.locusAssistantRefresh?.();
    return list;
}

// The conversation and the events shown for it, kept for the session so another app's page can show it again.
let chat = JSON.parse(sessionStorage.getItem(HISTORY) || 'null') ?? { messages: [], events: [] };
function save() {
    try {
        sessionStorage.setItem(HISTORY, JSON.stringify(chat));
    } catch {
        // Too many pictures for the storage quota; the chat then lasts until the page is left.
    }
}

export function reset() {
    chat = { messages: [], events: [] };
    save();
}

export const history = () => chat.events;

function emit(event) {
    const last = chat.events.at(-1);
    // Deltas are joined, so the history does not keep one event per token.
    if (last?.type === 'delta' && event.type === 'delta') last.text += event.text;
    else chat.events.push({ ...event });
    window.assistantEvent?.(event);
}

// Calls `each` with the data of every server-sent event.
async function read(provider, response, each) {
    const reader = response.body.pipeThrough(new TextDecoderStream()).getReader();
    let buffer = '';
    for (;;) {
        const { done, value } = await reader.read().catch(() => {
            throw `The connection to ${provider.name} was lost.`;
        });
        if (done) return;
        buffer += value;
        let end;
        while ((end = buffer.indexOf('\n')) >= 0) {
            const line = buffer.slice(0, end).trim();
            buffer = buffer.slice(end + 1);
            if (!line.startsWith('data:')) continue;
            const data = line.slice(5).trim();
            if (data !== '[DONE]') each(JSON.parse(data));
        }
    }
}

async function readAnthropic(provider, response, thinking, emit) {
    const message = { content: [] };
    await read(provider, response, (event) => {
        if (event.type === 'content_block_start') {
            const block = event.content_block;
            if (block.type === 'text' || (thinking && block.type === 'thinking')) emit({ type: 'block', kind: block.type });
            message.content.push(block);
        } else if (event.type === 'content_block_delta') {
            const block = message.content[event.index];
            const field = { text_delta: 'text', thinking_delta: 'thinking', signature_delta: 'signature', input_json_delta: 'partial_json' }[event.delta.type];
            if (!field) return;
            block[field] = (block[field] ?? '') + event.delta[field];
            if (field === 'text' || (thinking && field === 'thinking')) emit({ type: 'delta', text: event.delta[field] });
        } else if (event.type === 'content_block_stop') {
            const block = message.content[event.index];
            if ('partial_json' in block) {
                try {
                    block.input = JSON.parse(block.partial_json);
                } catch {
                    block.input = {};
                }
                delete block.partial_json;
            }
        } else if (event.type === 'message_delta') {
            message.stop_reason = event.delta.stop_reason;
        } else if (event.type === 'error') {
            throw errorMessage(provider, event);
        }
    });
    return message;
}

async function readOpenai(provider, response, thinking, emit) {
    let text = '';
    let finish = '';
    // The kind of the block shown last, so a new one starts when text follows reasoning.
    let shown = '';
    const calls = [];
    await read(provider, response, (event) => {
        if (event.error) throw errorMessage(provider, event);
        const choice = event.choices?.[0] ?? {};
        const delta = choice.delta ?? {};
        const parts = [
            ['thinking', thinking ? (delta.reasoning_content ?? delta.reasoning ?? '') : ''],
            ['text', delta.content ?? ''],
        ];
        for (const [kind, part] of parts) {
            if (!part) continue;
            if (shown !== kind) emit({ type: 'block', kind });
            shown = kind;
            emit({ type: 'delta', text: part });
        }
        text += delta.content ?? '';
        for (const call of delta.tool_calls ?? []) {
            const index = call.index ?? Math.max(calls.length - 1, 0);
            calls[index] ??= { id: '', name: '', arguments: '' };
            calls[index].id += call.id ?? '';
            calls[index].name += call.function?.name ?? '';
            calls[index].arguments += call.function?.arguments ?? '';
        }
        if (choice.finish_reason) finish = choice.finish_reason;
    });
    const content = text ? [{ type: 'text', text }] : [];
    calls.forEach((call, index) => {
        let input = {};
        try {
            input = JSON.parse(call.arguments);
        } catch {}
        // Some providers leave the id out; it must be unique in the conversation.
        content.push({ type: 'tool_use', id: call.id || `call_${Date.now()}_${index}`, name: call.name, input });
    });
    // Some providers finish with "stop" even after tool calls.
    const stop = calls.length ? 'tool_use' : ({ length: 'max_tokens', content_filter: 'refusal' }[finish] ?? 'end_turn');
    return { content, stop_reason: stop };
}

const imagePart = (block) => ({ type: 'image_url', image_url: { url: `data:${block.source.media_type};base64,${block.source.data}` } });

// Turns the conversation into Chat Completions messages. Thinking is left out.
function openaiMessages(system, history) {
    const result = [{ role: 'system', content: system }];
    for (const message of history) {
        if (message.role === 'assistant') {
            const text = message.content.filter((b) => b.type === 'text').map((b) => b.text).join('');
            const calls = message.content
                .filter((b) => b.type === 'tool_use')
                .map((b) => ({ id: b.id, type: 'function', function: { name: b.name, arguments: JSON.stringify(b.input) } }));
            result.push({ role: 'assistant', content: text, ...(calls.length && { tool_calls: calls }) });
            continue;
        }
        const parts = [];
        for (const block of message.content) {
            if (block.type === 'tool_result') {
                const items = Array.isArray(block.content) ? block.content : [{ type: 'text', text: block.content }];
                const text = items.filter((item) => item.text !== undefined).map((item) => item.text).join('\n');
                result.push({ role: 'tool', tool_call_id: block.tool_use_id, content: text });
                // Tool messages only take text, so pictures follow in a user message.
                parts.push(...items.filter((item) => item.type === 'image').map(imagePart));
            } else if (block.type === 'image') {
                parts.push(imagePart(block));
            } else {
                parts.push({ type: 'text', text: block.text });
            }
        }
        if (parts.length) result.push({ role: 'user', content: parts });
    }
    return result;
}

async function complete(current, system, history, emit) {
    const { provider, url, key, model, effort, thinking } = current;
    if (provider.anthropic) {
        const body = {
            model,
            max_tokens: 64000,
            stream: true,
            system,
            tools: definitions(),
            messages: history,
            thinking: { type: 'adaptive', display: thinking ? 'summarized' : 'omitted' },
            output_config: { effort },
        };
        // Haiku has no server-side fallback for refusals.
        if (model !== 'claude-haiku-5-5') body.fallbacks = 'default';
        const response = await request(provider, `${url}/v1/messages`, {
            method: 'POST',
            headers: { ...headers(provider, key), 'anthropic-beta': 'server-side-fallback-2026-07-01', 'content-type': 'application/json' },
            body: JSON.stringify(body),
        });
        return readAnthropic(provider, response, thinking, emit);
    }
    const tools = definitions().map((tool) => ({ type: 'function', function: { name: tool.name, description: tool.description, parameters: tool.input_schema } }));
    const body = { model, stream: true, tools, messages: openaiMessages(system, history) };
    if (effort !== 'default') {
        if (provider.id === 'openrouter') body.reasoning = { effort };
        else body.reasoning_effort = effort;
    }
    const response = await request(provider, `${url}/chat/completions`, {
        method: 'POST',
        headers: { ...headers(provider, key), 'content-type': 'application/json' },
        body: JSON.stringify(body),
    });
    return readOpenai(provider, response, thinking, emit);
}

// Runs the conversation until the model is done, sending its output to `emit`.
async function run(history, extra, emit) {
    const current = settings();
    let system = SYSTEM + extra;
    if (current.instructions) system += `\n\nInstructions from the user:\n${current.instructions}`;
    for (let step = 0; step < MAX_STEPS; step++) {
        const response = await complete(current, system, history, emit);
        history.push({ role: 'assistant', content: response.content });
        if (response.stop_reason === 'refusal') throw 'The model declined to answer this.';
        if (response.stop_reason === 'max_tokens') throw 'The answer was too long and was cut off.';
        if (response.stop_reason !== 'tool_use') return;
        const results = [];
        for (const block of response.content.filter((block) => block.type === 'tool_use')) {
            let content;
            let isError = false;
            try {
                content = await call(block.name, block.input);
            } catch (error) {
                content = String(error);
                isError = true;
            }
            emit({ type: 'tool', label: label(block.name, block.input) });
            results.push({ type: 'tool_result', tool_use_id: block.id, content, is_error: isError });
        }
        history.push({ role: 'user', content: results });
    }
    throw 'The model stopped after too many steps.';
}

async function sendMessage(text, images) {
    if (!isAvailable(settings())) throw 'The AI sidebar is not available.';
    // Images are base64 JPEGs.
    const content = images.map((data) => ({ type: 'image', source: { type: 'base64', media_type: 'image/jpeg', data } }));
    if (text) content.push({ type: 'text', text });
    if (!content.length) throw 'Write a message or add a picture.';
    const history = [...chat.messages, { role: 'user', content }];
    try {
        await run(history, '', emit);
    } finally {
        chat.messages = history;
    }
}

// The reply and any error arrive as events, followed by a done event.
export async function send({ text, images }) {
    emit({ type: 'user', text, images });
    try {
        await sendMessage(text, images);
    } catch (error) {
        emit({ type: 'error', text: String(error) });
        throw error;
    } finally {
        emit({ type: 'done' });
        save();
    }
}

// Carries out a request typed into the Algebra input, outside of the chat. Returns the model's closing reply.
export async function prompt({ text }) {
    const current = settings();
    if (!isAvailable(current) || !current.prompt) throw 'Natural language input is not available.';
    const history = [{ role: 'user', content: [{ type: 'text', text }] }];
    await run(history, `\n\n${PROMPT}`, () => {});
    return history.at(-1).content.filter((block) => block.type === 'text').map((block) => block.text).join('');
}

// The AI assistant for the web version. The browser talks to the Anthropic API directly with the user's key.
import { call, definitions, label } from './tools.js';

const KEY = 'locus.assistant';
const MODELS = ['claude-sonnet-5-5', 'claude-haiku-5-5', 'claude-opus-5-5', 'claude-fable-5-1'];
const EFFORTS = ['low', 'medium', 'high', 'xhigh', 'max'];
const DEFAULT_URL = 'https://api.anthropic.com';
const VERSION = '2026-10-21';
const MAX_STEPS = 20;
const SYSTEM =
    'You are an AI assistant that can help the user with using Locus, a math app, and with other excercises. ' +
    'Answer concisely. ' +
    'Use Markdown, and write math in LaTeX between $...$ or $$...$$.';
const UNREACHABLE = 'Could not reach Anthropic. Check the internet connection.';

let messages = [];

function settings() {
    const saved = JSON.parse(localStorage.getItem(KEY) || '{}');
    return {
        key: saved.key ?? '',
        model: MODELS.includes(saved.model) ? saved.model : MODELS[0],
        effort: EFFORTS.includes(saved.effort) ? saved.effort : 'medium',
        url: saved.url || DEFAULT_URL,
        thinking: saved.thinking ?? 'true',
        instructions: saved.instructions ?? '',
        enabled: saved.enabled ?? 'false',
    };
}

const headers = (key) => ({
    'x-api-key': key,
    'anthropic-version': VERSION,
    'anthropic-dangerous-direct-browser-access': 'true',
});

// Returns the settings without the key itself, like the desktop app.
export function publicSettings() {
    const current = settings();
    const enabled = current.enabled === 'true';
    return { ...current, key: current.key !== '', thinking: current.thinking === 'true', enabled, available: enabled && current.key !== '' };
}

const errorMessage = (data) => data?.error?.message ?? 'The request to Claude failed.';

// Checks the key with a cheap request, so a wrong key is caught on the setup page.
async function checkKey(url, key) {
    const response = await fetch(`${url}/v1/models?limit=1`, { headers: headers(key) }).catch(() => {
        throw UNREACHABLE;
    });
    if (response.status === 200) return;
    if (response.status === 401 || response.status === 403) throw 'This API key was not accepted.';
    throw errorMessage(await response.json().catch(() => null));
}

export async function set({ name, value }) {
    value = name === 'instructions' ? value.trim() : value.trim().replace(/\/+$/, '');
    const current = settings();
    if (name === 'key' && value) await checkKey(current.url, value);
    const valid = {
        key: true,
        url: !value || value.startsWith('http://') || value.startsWith('https://'),
        model: MODELS.includes(value),
        effort: EFFORTS.includes(value),
        thinking: value === 'true' || value === 'false',
        enabled: value === 'true' || value === 'false',
        instructions: true,
    }[name];
    if (!valid) throw 'Invalid assistant setting.';
    localStorage.setItem(KEY, JSON.stringify({ ...current, [name]: value }));
    window.locusAssistantRefresh?.();
}

export function reset() {
    messages = [];
}

const emit = (event) => window.assistantEvent?.(event);

// Rebuilds the message from stream events and forwards text and thinking to the chat.
function handle(message, event, thinking) {
    if (event.type === 'content_block_start') {
        const block = event.content_block;
        if (block.type === 'text' || (thinking && block.type === 'thinking')) emit({ type: 'block', kind: block.type });
        message.content.push(block);
    } else if (event.type === 'content_block_delta') {
        const block = message.content[event.index];
        const field = {
            text_delta: 'text',
            thinking_delta: 'thinking',
            signature_delta: 'signature',
            input_json_delta: 'partial_json',
        }[event.delta.type];
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
        throw errorMessage(event);
    }
}

async function complete(history) {
    const current = settings();
    const thinking = current.thinking === 'true';
    const system = current.instructions ? `${SYSTEM}\n\nInstructions from the user:\n${current.instructions}` : SYSTEM;
    const body = {
        model: current.model,
        max_tokens: 64000,
        stream: true,
        system,
        tools: definitions(),
        messages: history,
        thinking: { type: 'adaptive', display: thinking ? 'summarized' : 'omitted' },
        output_config: { effort: current.effort },
    };
    // Haiku has no server-side fallback for refusals.
    if (current.model !== 'claude-haiku-5-5') body.fallbacks = 'default';
    const response = await fetch(`${current.url}/v1/messages`, {
        method: 'POST',
        headers: {
            ...headers(current.key),
            'anthropic-beta': 'server-side-fallback-2026-07-01',
            'content-type': 'application/json',
        },
        body: JSON.stringify(body),
    }).catch(() => {
        throw UNREACHABLE;
    });
    if (!response.ok) throw errorMessage(await response.json().catch(() => null));
    const message = { content: [] };
    const reader = response.body.pipeThrough(new TextDecoderStream()).getReader();
    let buffer = '';
    for (;;) {
        const { done, value } = await reader.read().catch(() => {
            throw 'The connection to Anthropic was lost.';
        });
        if (done) return message;
        buffer += value;
        let end;
        while ((end = buffer.indexOf('\n\n')) >= 0) {
            const lines = buffer.slice(0, end).split('\n');
            buffer = buffer.slice(end + 2);
            for (const line of lines) {
                if (line.startsWith('data: ')) handle(message, JSON.parse(line.slice(6)), thinking);
            }
        }
    }
}

async function run(history) {
    for (let step = 0; step < MAX_STEPS; step++) {
        const response = await complete(history);
        history.push({ role: 'assistant', content: response.content });
        if (response.stop_reason === 'refusal') throw 'Claude declined to answer this.';
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
    throw 'Claude stopped after too many steps.';
}

export async function send({ text, images }) {
    if (!publicSettings().available) throw 'The AI assistant is not available.';
    // Images are base64 JPEGs.
    const content = images.map((data) => ({ type: 'image', source: { type: 'base64', media_type: 'image/jpeg', data } }));
    if (text) content.push({ type: 'text', text });
    if (!content.length) throw 'Write a message or add a picture.';
    const history = [...messages, { role: 'user', content }];
    try {
        await run(history);
    } finally {
        messages = history;
    }
}

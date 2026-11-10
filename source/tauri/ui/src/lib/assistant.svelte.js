import { invoke } from '$backend';

export const assistant = $state({
    settings: { key: false, model: '', effort: '', url: '', enabled: false, available: false },
    // 'conversation' or 'camera'
    view: 'conversation',
    items: [],
    busy: false,
    // Bumped on every change to the conversation, so the list can follow new output.
    version: 0,
    attachments: [],
    focusPrompt: false,
});

export const loadSettings = () =>
    invoke('assistant_settings').then((settings) => (assistant.settings = settings));

export const setSetting = (name, value) =>
    invoke('assistant_set', { name, value }).then(loadSettings);

// Scales a picture down to the size Claude reads at full detail and attaches it as a JPEG.
export function attach(source, width, height) {
    const scale = Math.min(1, 1568 / Math.max(width, height));
    const canvas = document.createElement('canvas');
    canvas.width = Math.round(width * scale);
    canvas.height = Math.round(height * scale);
    canvas.getContext('2d').drawImage(source, 0, 0, canvas.width, canvas.height);
    assistant.attachments.push(canvas.toDataURL('image/jpeg', 0.85).split(',')[1]);
}

export function send(text) {
    const images = assistant.attachments;
    if ((!text && !images.length) || assistant.busy) return false;
    assistant.attachments = [];
    assistant.busy = true;
    push({ kind: 'user', text, images });
    invoke('assistant_send', { text, images })
        .catch((error) => push({ kind: 'error', text: String(error) }))
        .finally(() => {
            assistant.busy = false;
            assistant.version++;
        });
    return true;
}

export function reset() {
    return invoke('assistant_reset').then(() => {
        assistant.items = [];
        assistant.version++;
    });
}

function push(item) {
    assistant.items.push(item);
    assistant.version++;
}

// Called by the app for each streamed event of a reply.
window.assistantEvent = (event) => {
    if (event.type === 'block') {
        push({ kind: event.kind === 'thinking' ? 'thinking' : 'assistant', text: '' });
    } else if (event.type === 'delta') {
        assistant.items.at(-1).text += event.text;
        assistant.version++;
    } else if (event.type === 'tool') {
        push({ kind: 'tool', text: event.label });
    }
};

// Called by the app when the window is hidden, so the camera turns off.
window.closeCamera = () => {
    if (assistant.view === 'camera') assistant.view = 'conversation';
};

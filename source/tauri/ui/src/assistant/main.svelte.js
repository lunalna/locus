import { invoke } from '$backend';
import { mount } from 'svelte';
import katex from 'katex/dist/katex.min.css?inline';
import { assistant, loadHistory, loadSettings } from '../lib/assistant.svelte.js';
import css from './assistant.css?inline';
import Embedded from './Embedded.svelte';

// The AI assistant as a GeoGebra view: the app gives each assistant panel's element to window.locusAssistant.

// Fonts load only from the document, the rest of KaTeX's styles must be in the shadow root.
document.head.append(Object.assign(document.createElement('style'), { textContent: katex }));

// Each panel gets a shadow root so its styles and the app's styles stay apart.
function attach(element) {
    if (element.shadowRoot) return;
    const root = element.attachShadow({ mode: 'open' });
    const container = document.createElement('div');
    root.append(Object.assign(document.createElement('style'), { textContent: css + katex }), container);
    // Keeps the app's keyboard shortcuts from reacting while typing in the assistant.
    element.addEventListener('keydown', (event) => event.stopPropagation());
    // Turns the camera off once the panel is closed or hidden.
    new ResizeObserver(() => element.offsetWidth || window.closeCamera()).observe(element);
    mount(Embedded, { target: container, props: { container } });
}

// The placeholder from the app keeps the panels created before this script loaded.
const pending = window.locusAssistant?.pending ?? [];
window.locusAssistant = attach;
pending.forEach(attach);

// Tells the app whether to show the assistant; called after a settings change, an exam start or end, or a plugin allowing it.
window.locusAssistantRefresh = () =>
    loadSettings().then((settings) => {
        window.locusAssistantEnabled = settings.available && settings.sidebar;
        // "?" at the start of the Algebra input asks the AI.
        window.locusPromptEnabled = settings.available && settings.prompt;
        window.locusAssistantUpdate?.();
    });
window.locusAssistantRefresh();
loadHistory();

// Called by the app with a request typed into the Algebra input after "?".
window.locusPrompt = (text, done) =>
    invoke('assistant_prompt', { text }).then(
        (reply) => done(null, reply),
        (error) => done(String(error), null),
    );

// Called by the app when an exam starts or ends, so chats do not carry across.
window.locusAssistantReset = () => {
    Object.assign(assistant, { view: 'conversation', items: [], attachments: [] });
    window.locusAssistantRefresh();
};

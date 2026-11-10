import { invoke } from '@tauri-apps/api/core';
import { mount } from 'svelte';
import css from '../settings/settings.css?inline';
import Plugins from './Plugins.svelte';

// The running plugins, the open panel and the command list of this app window.
const ui = $state({ plugins: [], events: false, panel: null, list: false });

// Panels and the command list live in a shadow root so their styles and the app's stay apart.
const host = document.createElement('div');
host.style.cssText = 'position: relative; z-index: 99999';
const root = host.attachShadow({ mode: 'open' });
const style = document.createElement('style');
style.textContent = css;
const container = document.createElement('div');
root.append(style, container);
host.addEventListener('keydown', (event) => event.key !== 'Escape' && event.stopPropagation());
document.body.append(host);
mount(Plugins, { target: container, props: { ui, run } });

async function load() {
    Object.assign(ui, await invoke('plugin_app_state'));
    if (ui.panel && !ui.plugins.some((plugin) => plugin.id === ui.panel && plugin.panel)) ui.panel = null;
}
load();

window.locusPlugins = () => (ui.list = true);
window.locusPluginsChanged = load;
// Shows, hides or, without `show`, toggles a plugin's panel.
window.locusPluginPanel = (id, show) => {
    if (show ?? ui.panel !== id) ui.panel = id;
    else if (ui.panel === id) ui.panel = null;
};

// Runs a command from the list or a panel toolbar; without a command it shows or hides the plugin window.
function run(plugin, command) {
    invoke('plugin_command', { plugin, command: command ?? null });
}

// A panel has no access to Tauri, so it sends its calls here, and this page makes them in its name.
const allowed = ['plugin_window', 'plugin_view', 'plugin_app', 'plugin_panel', 'plugin_storage', 'plugin_settings', 'plugin_file', 'plugin_exam'];
const frame = () => root.querySelector('iframe');

window.addEventListener('message', async (event) => {
    const message = event.data;
    if (message?.locus !== 'call' || !frame() || event.source !== frame().contentWindow) return;
    const reply = (answer) => event.source.postMessage({ locus: 'result', id: message.id, ...answer }, '*');
    if (!allowed.includes(message.command)) return reply({ error: `Panels cannot call ${message.command}.` });
    try {
        reply({ result: await invoke(message.command, { ...message.args, plugin: ui.panel }) });
    } catch (error) {
        reply({ error: String(error) });
    }
});

window.locusPluginPanelEvent = (id, event) => {
    if (id === ui.panel) frame()?.contentWindow.postMessage({ locus: 'event', event }, '*');
};

// Construction changes go to the plugins in batches, with each object listed once per update.
let queue = [];
function report(event) {
    if (!ui.events) return;
    if (event.type === 'update' && queue.some((other) => other.type === 'update' && other.name === event.name)) return;
    if (!queue.length) {
        setTimeout(() => {
            invoke('plugin_app_event', { events: queue });
            queue = [];
        }, 50);
    }
    queue.push(event);
}

(function listen() {
    const api = window.ggbApplet;
    if (!api?.registerAddListener) return setTimeout(listen, 500);
    api.registerAddListener((name) => report({ type: 'add', name }));
    api.registerRemoveListener((name) => report({ type: 'remove', name }));
    api.registerUpdateListener((name) => report({ type: 'update', name }));
    api.registerRenameListener((from, to) => report({ type: 'rename', name: to, from }));
    api.registerClearListener(() => report({ type: 'clear' }));
    api.registerClientListener((event) => {
        if (event.type === 'select' || event.type === 'deselect') report({ type: event.type, name: event.target ?? null });
    });
})();

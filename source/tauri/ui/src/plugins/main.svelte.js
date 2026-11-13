import { invoke } from '@tauri-apps/api/core';
import { mount } from 'svelte';
import css from '../settings/settings.css?inline';
import Plugins from './Plugins.svelte';

// The running plugins, the events they listen to, the open panel, the command list, the picked plugin tool,
// the graphics view and the notes plugins show in this app window.
const ui = $state({ plugins: [], events: [], panel: null, list: false, tool: null, view: null, toasts: [] });

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
mount(Plugins, { target: container, props: { ui, run: (plugin, command) => window.locusPluginRun(plugin, command), toolInput: input, wheel } });

// Plugin styles in this page, by URL. Scripts run once and stay until the page loads again.
const styles = new Map();
const scripts = new Set();

async function load() {
    Object.assign(ui, await invoke('plugin_app_state'));
    if (ui.panel && !ui.plugins.some((plugin) => plugin.id === ui.panel && plugin.panel)) ui.panel = null;
    if (ui.tool && !command(ui.tool.plugin, ui.tool.id)?.tool) ui.tool = null;
    const urls = ui.plugins.flatMap((plugin) => plugin.styles);
    for (const [url, link] of styles) {
        if (!urls.includes(url)) {
            link.remove();
            styles.delete(url);
        }
    }
    for (const url of urls.filter((url) => !styles.has(url))) {
        const link = Object.assign(document.createElement('link'), { rel: 'stylesheet', href: url });
        document.head.append(link);
        styles.set(url, link);
    }
    for (const plugin of ui.plugins.filter((plugin) => plugin.script && !scripts.has(plugin.id))) {
        scripts.add(plugin.id);
        const script = Object.assign(document.createElement('script'), { src: plugin.script });
        script.dataset.plugin = plugin.id;
        document.head.append(script);
    }
    window.locusPluginsUpdate?.();
}
load();

const command = (plugin, id) => ui.plugins.find((other) => other.id === plugin)?.commands.find((command) => command.id === id);

// Commands for the app's toolbar, main menu or object context menu: place is toolbar, menu or object.
// In the context menu, commands that list object types show only when all selected objects have one of them.
window.locusPluginEntries = (place, types) =>
    ui.plugins.flatMap((plugin) =>
        plugin.commands
            .filter((command) => command.places?.includes(place))
            .filter((command) => !types || !command.types || types.every((type) => command.types.includes(type)))
            .map((command) => ({ plugin: plugin.id, id: command.id, title: command.title, icon: command.icon })),
    );

// Runs a command, picks or drops a plugin tool, or without a command shows or hides the plugin window.
window.locusPluginRun = (plugin, id, objects) => {
    if (!command(plugin, id)?.tool) return invoke('plugin_command', { plugin, command: id ?? null, objects: objects ?? null });
    pick(ui.tool?.plugin === plugin && ui.tool.id === id ? null : { plugin, id });
};

// The RunPlugin command in a construction's scripts runs commands that allow it.
window.locusPluginScript = (plugin, id, objects) => {
    if (command(plugin, id)?.places?.includes('construction')) window.locusPluginRun(plugin, id, objects);
};

let toasts = 0;
window.locusPluginToast = (name, text) => {
    const id = ++toasts;
    ui.toasts.push({ id, name, text });
    setTimeout(() => (ui.toasts = ui.toasts.filter((toast) => toast.id !== id)), 4000);
};

window.locusPlugins = () => (ui.list = true);
window.locusPluginsChanged = load;
// Shows, hides or, without `show`, toggles a plugin's panel.
window.locusPluginPanel = (id, show) => {
    if (show ?? ui.panel !== id) ui.panel = id;
    else if (ui.panel === id) ui.panel = null;
};

// Panels and overlays have no access to Tauri, so they send their calls here, and this page makes them in their name.
const allowed = [
    'plugin_window',
    'plugin_view',
    'plugin_app',
    'plugin_panel',
    'plugin_storage',
    'plugin_settings',
    'plugin_file',
    'plugin_exam',
    'plugin_theme',
    'plugin_command_update',
    'plugin_dialog',
];
const frames = (plugin) => [...root.querySelectorAll('iframe')].filter((frame) => !plugin || frame.dataset.plugin === plugin);
const post = (plugin, event) => frames(plugin).forEach((frame) => frame.contentWindow.postMessage({ locus: 'event', event }, '*'));

window.addEventListener('message', async (event) => {
    const message = event.data;
    const frame = frames().find((frame) => frame.contentWindow === event.source);
    if (message?.locus !== 'call' || !frame) return;
    const reply = (answer) => event.source.postMessage({ locus: 'result', id: message.id, ...answer }, '*');
    if (!allowed.includes(message.command)) return reply({ error: `Panels cannot call ${message.command}.` });
    try {
        reply({ result: await invoke(message.command, { ...message.args, plugin: frame.dataset.plugin }) });
    } catch (error) {
        reply({ error: String(error) });
    }
});

window.locusPluginPanelEvent = post;

// Calls send once per animation frame with the latest value given.
function framed(send) {
    let latest;
    let pending = false;
    return (value) => {
        latest = value;
        if (pending) return;
        pending = true;
        requestAnimationFrame(() => {
            pending = false;
            send(latest);
        });
    };
}

// Overlays let clicks through, so they follow the pointer from here, with at most one move per frame.
const overlays = (event) => ui.plugins.filter((plugin) => plugin.overlay).forEach((plugin) => post(plugin.id, event));
const moved = framed(overlays);
function track(event) {
    const pointer = { type: 'pointer', kind: event.type.slice(7), x: event.clientX, y: event.clientY, buttons: event.buttons };
    if (pointer.kind === 'move') moved(pointer);
    else overlays(pointer);
}
for (const type of ['pointermove', 'pointerdown', 'pointerup']) window.addEventListener(type, track, { capture: true, passive: true });
document.documentElement.addEventListener('pointerleave', track);

// The graphics view in page pixels and the part of the plane it shows.
window.locusPluginView = () => {
    const view = JSON.parse(ggbApplet.getViewProperties(1));
    return {
        left: view.left,
        top: view.top,
        width: view.width,
        height: view.height,
        xmin: view.xMin,
        xmax: view.xMin + view.width * view.invXscale,
        ymin: view.yMin,
        ymax: view.yMin + view.height * view.invYscale,
    };
};

// A plugin tool takes the input on the graphics view from the app until another tool is picked or Escape is pressed.
function pick(tool) {
    const active = (tool, active) => tool && invoke('plugin_input', { plugin: tool.plugin, event: { type: 'toolActive', tool: tool.id, active } });
    active(ui.tool, false);
    ui.tool = tool;
    ui.view = window.locusPluginView();
    active(tool, true);
}
window.addEventListener('keydown', (event) => event.key === 'Escape' && ui.tool && pick(null));

const toolMoved = framed((event) => invoke('plugin_input', event));
function input(event) {
    const { plugin, id } = ui.tool;
    const view = ui.view;
    const kind = event.type.slice(7);
    if (kind === 'down') event.currentTarget.setPointerCapture(event.pointerId);
    const x = view.xmin + ((event.clientX - view.left) / view.width) * (view.xmax - view.xmin);
    const y = view.ymax - ((event.clientY - view.top) / view.height) * (view.ymax - view.ymin);
    const objects = window.locusPluginHits(event.clientX, event.clientY);
    const call = { plugin, event: { type: 'input', tool: id, kind, x, y, screenX: event.clientX, screenY: event.clientY, buttons: event.buttons, objects } };
    if (kind === 'move') toolMoved(call);
    else invoke('plugin_input', call);
}

// Zooming with the wheel still reaches the app below.
function wheel(event) {
    const layer = event.currentTarget;
    layer.style.pointerEvents = 'none';
    document.elementFromPoint(event.clientX, event.clientY)?.dispatchEvent(new WheelEvent('wheel', event));
    layer.style.pointerEvents = '';
}

const send = (event) => ui.events.includes(event.type) && invoke('plugin_app_event', { event });

// Construction changes go to the plugins in batches, with each object listed once per update.
let queue = [];
function report(event) {
    if (!ui.events.includes('construction')) return;
    if (event.type === 'update' && queue.some((other) => other.type === 'update' && other.name === event.name)) return;
    if (!queue.length) {
        setTimeout(() => {
            for (const event of queue.filter((event) => ['add', 'update'].includes(event.type) && ggbApplet.exists(event.name))) {
                event.objectType = ggbApplet.getObjectType(event.name);
                event.value = ggbApplet.getValueString(event.name);
            }
            send({ type: 'construction', events: queue });
            queue = [];
        }, 50);
    }
    queue.push(event);
}

const viewChanged = framed(() => {
    ui.view = window.locusPluginView();
    send({ type: 'view', view: ui.view });
});
let selecting = false;

(function listen() {
    const api = window.ggbApplet;
    if (!api?.registerAddListener) return setTimeout(listen, 500);
    api.registerAddListener((name) => report({ type: 'add', name }));
    api.registerRemoveListener((name) => report({ type: 'remove', name }));
    api.registerUpdateListener((name) => report({ type: 'update', name }));
    api.registerRenameListener((from, to) => report({ type: 'rename', name: to, from }));
    api.registerClearListener(() => report({ type: 'clear' }));
    api.registerClientListener((event) => {
        if ((event.type === 'select' || event.type === 'deselect') && !selecting) {
            selecting = true;
            setTimeout(() => {
                selecting = false;
                send({ type: 'selection', objects: window.locusPluginSelection() });
            });
        }
        if (event.type === 'viewChanged2D') viewChanged();
        if (event.type === 'setMode') {
            if (ui.tool) pick(null);
            send({ type: 'mode', mode: api.getMode() });
        }
        if (event.type === 'undo' || event.type === 'redo') send({ type: 'undo', kind: event.type });
    });
})();

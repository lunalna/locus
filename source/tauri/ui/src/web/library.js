// Resource packs and themes for the web version. Installed items and the files the page loads from
// pack/ are kept in Cache Storage, and sw.js serves pack/ from there.
import { strFromU8, unzipSync } from 'fflate';
import { script as performanceScript } from './performance.js';

const CACHE = 'locus';
const KEY = 'locus.library';
const root = new URL('.', location.href).href;
const PACK = `${root}pack/`;

const kinds = { pack: 'pack.json', theme: 'theme.json' };
const extensions = { pack: '.lcpack', theme: '.lctheme' };

const themeFiles = import.meta.glob('../../../themes/*/theme.json', { eager: true, import: 'default' });
// Themes that come with the app, by usefulness. The first is listed next to the default theme, the rest under More themes.
const BUILTIN_THEMES = [
    'dark', 'catppuccin-mocha', 'dracula', 'tokyo-night', 'one-dark', 'nord', 'gruvbox-dark',
    'rose-pine', 'catppuccin-macchiato', 'catppuccin-frappe', 'solarized-dark', 'catppuccin-latte', 'solarized-light',
].map((id) => themeFiles[`../../../themes/${id}/theme.json`]);

const builtins = (kind) => (kind === 'theme' ? BUILTIN_THEMES : []);
const folder = (kind, id) => `${root}library/${kind}/${id}/`;
const saved = () => JSON.parse(localStorage.getItem(KEY) || '{}');
const active = (kind) => saved()[kind] ?? null;
const setActive = (kind, id) => localStorage.setItem(KEY, JSON.stringify({ ...saved(), [kind]: id }));

async function entries(prefix) {
    const cache = await caches.open(CACHE);
    return (await cache.keys()).filter((request) => request.url.startsWith(prefix));
}

const read = async (url) => (await caches.open(CACHE)).match(url);
const put = async (url, body, type) =>
    (await caches.open(CACHE)).put(url, new Response(body, { headers: { 'Content-Type': type } }));

async function installed(kind) {
    const items = [];
    for (const request of await entries(`${root}library/${kind}/`)) {
        const id = request.url.slice(`${root}library/${kind}/`.length).split('/')[0];
        if (request.url === folder(kind, id) + kinds[kind]) items.push([id, await (await read(request)).json()]);
    }
    return items;
}

// An installed item overrides a built-in one with the same id.
async function activeManifest(kind) {
    const id = active(kind);
    if (!id) return null;
    const file = await read(folder(kind, id) + kinds[kind]);
    return file ? file.json() : (builtins(kind).find((item) => item.id === id) ?? null);
}

// Only plain hex colors, since they end up in CSS.
const hex = (color) => (/^#[0-9a-fA-F]{6}$/.test(color) ? color : null);

const colors = (manifest) =>
    Object.fromEntries(
        Object.entries(manifest?.colors ?? {}).filter(([name, color]) => /^[A-Za-z0-9-]+$/.test(name) && hex(color)),
    );

// Writes the files the page loads from pack/ for the active pack, theme and performance settings.
// Returns the theme as the page applies it.
export async function publish() {
    const cache = await caches.open(CACHE);
    for (const request of await entries(PACK)) await cache.delete(request);
    const packId = active('pack');
    const pack = packId && (await (await read(folder('pack', packId) + kinds.pack))?.json());
    const theme = await activeManifest('theme');
    const themeCss = active('theme') && (await read(folder('theme', active('theme')) + 'theme.css'));

    // theme accents win over pack accents
    const current = {
        colors: { ...colors(pack), ...colors(theme) },
        theme: hex(theme?.background) && hex(theme?.foreground) ? { background: theme.background, foreground: theme.foreground } : null,
        css: themeCss ? `${PACK}theme/theme.css` : null,
    };
    if (themeCss) await cache.put(`${PACK}theme/theme.css`, themeCss);

    let script = performanceScript() + `window.resourcePackColors = ${JSON.stringify(current.colors)};\n`;
    if (current.theme) script += `window.locusTheme = ${JSON.stringify(current.theme)};\n`;
    if (current.css) script += `window.locusThemeCss = ${JSON.stringify(current.css)};\n`;
    if (pack) {
        const files = folder('pack', packId);
        for (const request of await entries(files)) {
            await cache.put(PACK + request.url.slice(files.length), await read(request));
        }
        if (await read(`${files}icon`)) script += `window.resourcePackIcon = ${JSON.stringify(`${PACK}icon`)};\n`;
        const icons = await (await read(`${files}icons.json`)).text();
        const languages = await (await read(`${files}languages.json`)).text();
        const english = await (await read(`${files}lang/en.js`)).text();
        script +=
            `window.resourcePackUrl = ${JSON.stringify(PACK)};\nwindow.resourcePackUrls = ${JSON.stringify(pack.urls ?? null)};\n` +
            `window.resourcePackIcons = ${icons};\nwindow.resourcePackLanguages = ${languages};\nwindow.resourcePackKeys = {};\n${english}`;
    }
    await put(`${PACK}init.js`, script, 'text/javascript');
    return current;
}

export async function list({ kind }) {
    const entry = (id, item, builtin, more) => ({
        id,
        name: item.name,
        version: item.version,
        author: item.author,
        description: item.description,
        license: item.license,
        builtin,
        more,
    });
    const items = (await installed(kind)).map(([id, item]) => entry(id, item, false, false));
    items.sort((a, b) => String(a.name ?? '').toLowerCase().localeCompare(String(b.name ?? '').toLowerCase()));
    builtins(kind).forEach((item, index) => {
        if (!items.some((other) => other.id === item.id)) items.push(entry(item.id, item, true, index > 0));
    });
    return { items, active: active(kind) };
}

export function activate({ kind, id }) {
    setActive(kind, id);
    return publish();
}

export async function remove({ kind, id }) {
    if (active(kind) === id) setActive(kind, null);
    const cache = await caches.open(CACHE);
    for (const request of await entries(folder(kind, id))) await cache.delete(request);
    return publish();
}

const pick = (kind) =>
    new Promise((resolve) => {
        const input = Object.assign(document.createElement('input'), { type: 'file', accept: extensions[kind] });
        input.onchange = () => resolve(input.files[0]);
        input.oncancel = () => resolve(null);
        input.click();
    });

const validId = (id) => {
    if (typeof id === 'string' && id && !id.startsWith('.') && /^[A-Za-z0-9_.-]+$/.test(id)) return id;
    throw `Invalid pack id: ${id}`;
};

// What the details modal shows, with only web links.
const details = async (kind, id, item, builtin) => ({
    kind,
    id,
    name: item.name,
    version: item.version,
    author: item.author,
    description: item.description,
    license: item.license,
    links: Object.fromEntries(Object.entries(item.links ?? {}).filter(([, url]) => /^https?:\/\//.test(url))),
    builtin,
    installed: (await installed(kind)).some(([other]) => other === id),
    active: active(kind) === id,
});

// The picked file waits here while its details are shown.
let pending = null;

export async function pickFile({ kind }) {
    const file = await pick(kind);
    if (!file) return null;
    const manifestName = kinds[kind];
    let zip;
    try {
        zip = unzipSync(new Uint8Array(await file.arrayBuffer()));
    } catch {
        throw `This is not a valid ${kind === 'pack' ? 'resource pack' : 'theme'} file.`;
    }
    // Archives often wrap the contents in a folder.
    const manifestPath = manifestName in zip ? manifestName : Object.keys(zip).find((path) => path.split('/').length === 2 && path.endsWith(`/${manifestName}`));
    if (!manifestPath) throw `The file has no ${manifestName}.`;
    const prefix = manifestPath.slice(0, -manifestName.length);
    const files = Object.fromEntries(
        Object.entries(zip)
            .filter(([path]) => path.startsWith(prefix) && !path.endsWith('/'))
            .map(([path, data]) => [path.slice(prefix.length), data]),
    );
    let manifest;
    try {
        manifest = JSON.parse(strFromU8(files[manifestName]));
    } catch {
        throw `${manifestName} is not valid JSON.`;
    }
    if (!manifest.id) throw `${manifestName} has no id.`;
    pending = { kind, id: validId(manifest.id), manifest, files };
    return details(kind, pending.id, manifest, false);
}

export function cancel() {
    pending = null;
}

export async function detailsOf({ kind, id }) {
    const file = await read(folder(kind, id) + kinds[kind]);
    if (file) return details(kind, id, await file.json(), false);
    return details(kind, id, builtins(kind).find((item) => item.id === id), true);
}

export function link({ url }) {
    if (/^https?:\/\//.test(url)) window.open(url, '_blank', 'noopener');
}

// Adds the picked item into the pack or theme folder, named by its id.
export async function add({ kind }) {
    if (pending?.kind !== kind) throw 'Nothing is waiting to be added.';
    const { id, manifest, files } = pending;
    pending = null;
    const manifestName = kinds[kind];
    const cache = await caches.open(CACHE);
    for (const request of await entries(folder(kind, id))) await cache.delete(request);
    const target = folder(kind, id);
    await put(target + manifestName, files[manifestName], 'application/json');
    if (files['theme.css']) await put(`${target}theme.css`, files['theme.css'], 'text/css');
    if (kind === 'pack') {
        const icons = {};
        for (const [path, data] of Object.entries(files)) {
            if (path.startsWith('assets/') && path.endsWith('.svg')) icons[path.slice('assets/'.length)] = strFromU8(data);
        }
        await put(`${target}icons.json`, JSON.stringify(icons), 'application/json');
        const tags = languages(files);
        await put(`${target}languages.json`, JSON.stringify(tags), 'application/json');
        for (const tag of ['en', ...tags]) await put(`${target}lang/${tag}.js`, language(files, tag), 'text/javascript');
        const icon = typeof manifest.icon === 'string' && files[manifest.icon.replace(/^\.\//, '')];
        if (icon) await put(`${target}icon`, icon, manifest.icon.endsWith('.svg') ? 'image/svg+xml' : 'image/png');
    }
    if (active(kind) === id) await publish();
}

function language(files, tag) {
    // Upstream file names use Java's old codes for Hebrew and Indonesian.
    const suffix = { en: '', he: '_iw', id: '_in' }[tag] ?? `_${tag.replaceAll('-', '_')}`;
    const sections = {};
    for (const section of ['menu', 'command', 'symbols']) {
        const file = files[`lang/${section}${suffix}.properties`];
        if (file) sections[section] = parse(strFromU8(file));
    }
    return `window.resourcePackKeys[${JSON.stringify(tag)}] = ${JSON.stringify(sections)};`;
}

// Language tags of the pack's translations, the inverse of the file naming in language().
const languages = (files) =>
    Object.keys(files)
        .map((path) => path.match(/^lang\/menu_(.+)\.properties$/)?.[1])
        .filter(Boolean)
        .map((suffix) => ({ iw: 'he', in: 'id' })[suffix] ?? suffix.replaceAll('_', '-'));

function parse(text) {
    const keys = {};
    for (const line of text.split(/\r?\n/).map((line) => line.trimStart())) {
        if (!line || line.startsWith('#') || line.startsWith('!') || !line.includes('=')) continue;
        const index = line.indexOf('=');
        keys[line.slice(0, index).trim()] = unescape(line.slice(index + 1).trimStart());
    }
    return keys;
}

const unescape = (value) =>
    value.replace(/\\(u[0-9a-fA-F]{4}|.?)/g, (_, code) =>
        code[0] === 'u' && code.length === 5 ? String.fromCharCode(parseInt(code.slice(1), 16)) : ({ n: '\n', t: '\t' }[code] ?? code),
    );

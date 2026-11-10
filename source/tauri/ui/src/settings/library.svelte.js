import { invoke } from '$backend';
import { reload } from './reload.js';
import { applyTheme } from './theme.js';

// The details modal: an item about to be added, an installed one, or why a file could not be added.
// `version` goes up when the installed items changed, so the lists load again.
export const library = $state({ details: null, adding: false, error: '', version: 0 });

export function offer(details, error) {
    library.details = details;
    library.adding = Boolean(details);
    library.error = error ?? '';
}

const fail = (error) => offer(null, String(error));

export async function pick(kind) {
    try {
        const details = await invoke('library_pick', { kind });
        if (details) offer(details);
    } catch (error) {
        fail(error);
    }
}

export async function show(kind, id) {
    try {
        library.details = await invoke('library_details', { kind, id });
        library.adding = false;
    } catch (error) {
        fail(error);
    }
}

export function close() {
    if (library.adding) invoke('library_cancel', { kind: library.details.kind });
    offer(null);
}

export async function add() {
    try {
        await invoke('library_add', { kind: library.details.kind });
        offer(null);
        library.version++;
    } catch (error) {
        fail(error);
    }
}

// Removing the active theme applies the default in place; the active resource pack needs a reload.
export async function remove() {
    const { kind, id, active } = library.details;
    try {
        const theme = await invoke('library_remove', { kind, id });
        offer(null);
        library.version++;
        if (active && kind === 'theme') applyTheme(theme);
        if (active && kind === 'pack') reload();
    } catch (error) {
        fail(error);
    }
}

export async function setSetting(key, value) {
    const { id } = library.details;
    await invoke('plugin_setting_set', { id, key, value });
    library.details.plugin.values[key] = value;
}

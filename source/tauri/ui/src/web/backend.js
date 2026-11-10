// The Tauri commands used by the settings and the assistant, for the web version.
import * as assistant from './assistant.js';
import * as library from './library.js';
import * as performance from './performance.js';

const commands = {
    assistant_settings: assistant.publicSettings,
    assistant_set: assistant.set,
    assistant_reset: assistant.reset,
    assistant_send: assistant.send,
    library_list: library.list,
    library_activate: library.activate,
    library_remove: library.remove,
    library_pick: library.pickFile,
    library_add: library.add,
    library_cancel: library.cancel,
    library_details: library.detailsOf,
    library_link: library.link,
    performance_settings: performance.settings,
    performance_set: performance.set,
};

// Resource packs, themes and performance settings are served by a service worker, which needs HTTPS or localhost.
export async function invoke(name, args = {}) {
    if (/^(library|performance)_/.test(name) && !window.isSecureContext) {
        throw 'Resource packs, themes and performance settings need the app to be opened over HTTPS or on localhost.';
    }
    return commands[name](args);
}

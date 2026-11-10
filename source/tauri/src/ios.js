// Tauri's window.print does nothing on iOS, so it opens the system print sheet instead.
window.print = () => window.__TAURI__.core.invoke('print');

// iPadOS has no global shortcuts, so plugin shortcuts from a hardware keyboard come from the page.
// A block keeps the list out of the page's globals.
{
    let shortcuts = [];
    window.locusLoadShortcuts = () =>
        window.__TAURI__.core.invoke('plugin_shortcuts').then((list) => (shortcuts = list));
    addEventListener('DOMContentLoaded', window.locusLoadShortcuts);

    addEventListener('keydown', (event) => {
        if (event.repeat) return;
        const mods = event.ctrlKey | (event.altKey << 1) | (event.shiftKey << 2) | (event.metaKey << 3);
        const code = ['Control', 'Alt', 'Shift', 'Meta'].includes(event.key) ? null : event.code;
        if (shortcuts.some((shortcut) => shortcut.mods === mods && shortcut.code === code)) {
            if (code) event.preventDefault();
            window.__TAURI__.core.invoke('plugin_key', { mods, code });
        }
    }, true);
}

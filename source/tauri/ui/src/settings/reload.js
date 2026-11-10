const KEY = 'locusRestore';

// Reloads the page to apply a resource pack, keeping the open construction.
export function reload() {
    if (window.ggbApplet) sessionStorage.setItem(KEY, window.ggbApplet.getBase64());
    location.reload();
}

// Restores the construction saved by reload() once the app has loaded.
export function restore() {
    const saved = sessionStorage.getItem(KEY);
    if (!saved) return;
    sessionStorage.removeItem(KEY);
    if (window.ggbApplet) {
        window.ggbApplet.setBase64(saved);
        return;
    }
    const onLoad = window.ggbAppletOnLoad;
    // setBase64 calls ggbAppletOnLoad again, so the hook is restored before loading
    window.ggbAppletOnLoad = (...args) => {
        window.ggbAppletOnLoad = onLoad;
        onLoad?.(...args);
        window.ggbApplet.setBase64(saved);
    };
}

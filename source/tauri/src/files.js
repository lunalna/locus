// WKWebView has no file pickers, so GeoGebra would fall back to a download and a plain file input.
(() => {
    const invoke = (...args) => window.__TAURI__.core.invoke(...args);
    // GWT checks casts to these types with instanceof.
    const create = (type, methods) =>
        Object.assign(Object.create((window[type] ??= class {}).prototype), methods);
    const handle = (id, file) => create('FileSystemFileHandle', {
        getFile: async () => file,
        createWritable: async () => {
            const parts = [];
            return create('FileSystemWritableFileStream', {
                write: part => parts.push(part),
                close: async () => {
                    file = new File(parts, file.name);
                    await invoke('file_write', new Uint8Array(await file.arrayBuffer()), {
                        headers: { file: String(id) },
                    }).catch(error => alert(`Could not save ${file.name}: ${error}`));
                },
            });
        },
    });
    const cancelled = () => new DOMException('Cancelled.', 'AbortError');

    if (!window.showSaveFilePicker) {
        window.showSaveFilePicker = async ({ suggestedName }) => {
            const picked = await invoke('file_pick_save', { name: suggestedName });
            if (!picked) {
                throw cancelled();
            }
            const [id, name] = picked;
            return handle(id, new File([], name));
        };
    }
    if (!window.showOpenFilePicker) {
        window.showOpenFilePicker = async () => {
            const picked = await invoke('file_pick_open');
            if (!picked) {
                throw cancelled();
            }
            const [id, name] = picked;
            return [handle(id, new File([await invoke('file_read', { id })], name))];
        };
    }
})();

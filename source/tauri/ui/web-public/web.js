// What the desktop app injects into the app pages, for the web version.
window.locusPackBase = 'pack/';
window.locusOpenApp = (name) => open(`${name}.html`);
window.locusSwitchApp = (name) => {
    // Notes always starts empty and its pages do not fit the other apps
    if (ggbApplet.getObjectNumber() > 0 && name !== 'notes' && window.appID !== 'notes') {
        sessionStorage.setItem('locusFile', ggbApplet.getBase64());
    }
    location.href = `${name}.html`;
};
navigator.serviceWorker?.register('sw.js');

// Loads the settings modal and the AI assistant view, which the app opens through window.locusSettings
// and fills through window.locusAssistant.
addEventListener('DOMContentLoaded', () => {
    for (const name of ['settings', 'assistant']) {
        document.head.append(Object.assign(document.createElement('script'), { type: 'module', src: `browser-assets/${name}.js` }));
    }
});
// Lets the app create assistant views before the script has loaded; they are filled once it has.
window.locusAssistant = Object.assign((element) => window.locusAssistant.pending.push(element), { pending: [] });

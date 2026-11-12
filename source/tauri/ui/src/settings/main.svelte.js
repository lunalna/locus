import { mount } from 'svelte';
import { invoke } from '$backend';
import css from './settings.css?inline';
import Settings from './Settings.svelte';
import { offer } from './library.svelte.js';
import { reload, restore } from './reload.js';
import { applyTheme, paint } from './theme.js';

const modal = $state({ open: false, section: null, about: false, update: null });

// The modal lives in a shadow root so its styles and the app's styles stay apart.
const host = document.createElement('div');
// Stacks the modal and its popups above the app together.
host.style.cssText = 'position: relative; z-index: 100000';
paint();
const root = host.attachShadow({ mode: 'open' });
const style = document.createElement('style');
style.textContent = css;
const container = document.createElement('div');
root.append(style, container);
// Keeps the app's keyboard shortcuts from reacting while the modal is used. The app stops Escape at the body, so it is handed to the document, where the popups and the modal listen.
host.addEventListener('keydown', (event) => {
    event.stopPropagation();
    if (event.key === 'Escape') document.dispatchEvent(new KeyboardEvent('keydown', event));
});
document.body.append(host);
mount(Settings, { target: container, props: { modal, container } });

// Opens the settings, on the section for a settings tab of the app when its raw name is given.
window.locusSettings = (section) => {
    modal.section = section ?? null;
    modal.open = true;
};
window.locusAbout = () => (modal.about = true);
// Called when the startup check finds an update; the check can also finish before this page loads.
window.locusUpdate = async () => (modal.update = (await invoke('update_pending')) ?? modal.update);
// iPad builds have no updater, so the command is missing there.
if (import.meta.env.MODE !== 'web') window.locusUpdate().catch(() => {});
// Called with the details of a dropped plugin, resource pack or theme file, or why it could not be read.
window.locusLibraryOffer = offer;
// Called by the app once unsaved changes are saved or discarded after closing the window.
window.locusCloseWindow = () => invoke('window_close');
// Called when a setting changes in another window.
window.locusRefresh = (theme) => (theme ? applyTheme(theme) : reload());

// Manual reload for when GWT hot reload fails; dev only, since a reload could leave exam mode.
if (import.meta.env.DEV) {
    window.addEventListener(
        'keydown',
        (event) => {
            if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.code === 'KeyR') {
                event.preventDefault();
                event.stopPropagation();
                reload();
            }
        },
        true,
    );
}
restore();

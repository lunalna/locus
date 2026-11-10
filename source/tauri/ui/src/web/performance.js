import { publish } from './library.js';

const KEY = 'locus.performance';
const FPS = ['display', '60', '30'];
const RESOLUTIONS = ['full', 'standard'];

export function settings() {
    const saved = JSON.parse(localStorage.getItem(KEY) || '{}');
    return {
        low: saved.low ?? false,
        fps: FPS.includes(saved.fps) ? saved.fps : FPS[0],
        resolution: RESOLUTIONS.includes(saved.resolution) ? saved.resolution : RESOLUTIONS[0],
        asymptotes: saved.asymptotes ?? true,
    };
}

// Low performance mode overrides the individual choices.
export function script() {
    const current = settings();
    const [fps, full, asymptotes] = current.low
        ? [30, false, false]
        : [Number(current.fps) || 0, current.resolution === 'full', current.asymptotes];
    return `window.locusPerformance = ${JSON.stringify({ fps, maxPixelRatio: full ? 0 : 1, asymptotes: asymptotes ? 1 : 0 })};\n`;
}

export async function set({ name, value }) {
    const valid = {
        low: typeof value === 'boolean',
        asymptotes: typeof value === 'boolean',
        fps: FPS.includes(value),
        resolution: RESOLUTIONS.includes(value),
    }[name];
    if (!valid) throw 'Invalid performance setting.';
    localStorage.setItem(KEY, JSON.stringify({ ...settings(), [name]: value }));
    await publish();
}

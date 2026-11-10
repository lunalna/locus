import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath } from 'node:url';
import about from './about.js';

const path = (file) => fileURLToPath(new URL(file, import.meta.url));
const apps = ['classic', 'suite', 'graphing', 'geometry', '3d', 'cas', 'scientific', 'notes'];

// The GWT code server is reached through /__sdm, so an HTTPS page loads it from its own origin.
const codeServer = {
    '/web3d/web3d.nocache.js': {
        url: 'http://localhost:8888/web3d/web3d.nocache.js',
        edit: (js) => js.replace("'http://' + hostName + ':9876'", "$wnd.location.origin + '/__sdm'"),
    },
    '/__sdm/recompile-requester/web3d': {
        url: 'http://localhost:9876/recompile-requester/web3d',
        // the code server's URLs are otherwise built from the protocol and host alone
        edit: (js) => js.replace("return a.protocol + '//' + a.host + '/';", "return a.href.replace(/recompile-requester\\/.*$/, '');"),
    },
};

// In development the web version has the same URLs as the build: /graphing.html is the GWT dev server's
// graphing-web.html, and the settings and assistant come from the sources.
const webDev = {
    name: 'web-dev',
    configureServer(server) {
        server.middlewares.use(async (req, res, next) => {
            const [pathname, query = ''] = req.url.split('?');
            const page = pathname === '/' ? 'classic' : pathname.match(/^\/(.+)\.html$/)?.[1];
            if (apps.includes(page)) req.url = `/${page}-web.html${query && `?${query}`}`;
            req.url = req.url.replace(/^\/browser-assets\/(settings|assistant)\.js/, '/src/$1/main.svelte.js');
            const patch = codeServer[pathname];
            if (!patch) return next();
            const js = await (await fetch(patch.url)).text();
            res.setHeader('Content-Type', 'text/javascript');
            res.setHeader('Cache-Control', 'no-store');
            res.end(patch.edit(js));
        });
    },
};

// The web mode builds the settings and the assistant for browsers, with a backend in place of the Tauri commands.
export default defineConfig(({ mode }) => ({
    plugins: [svelte(), tailwindcss(), about(mode), ...(mode === 'web' ? [webDev] : [])],
    resolve: {
        alias: {
            $lib: path('src/lib'),
            $backend: mode === 'web' ? path('src/web/backend.js') : '@tauri-apps/api/core',
        },
    },
    publicDir: mode === 'web' ? 'web-public' : 'public',
    build: {
        outDir: mode === 'web' ? 'dist-web' : 'dist',
        assetsDir: 'browser-assets',
        rollupOptions: {
            input: {
                settings: 'src/settings/main.svelte.js',
                assistant: 'src/assistant/main.svelte.js',
                // The web version has no plugins.
                ...(mode !== 'web' && { plugins: 'src/plugins/main.svelte.js' }),
            },
            // The settings modal, the assistant view and the plugin UI are loaded into the app page by a fixed name.
            output: {
                entryFileNames: (chunk) =>
                    ['settings', 'assistant', 'plugins'].includes(chunk.name)
                        ? `browser-assets/${chunk.name}.js`
                        : 'browser-assets/[name]-[hash].js',
            },
        },
    },
    server: {
        port: mode === 'web' ? 5174 : 5173,
        strictPort: true,
        // tailscale serve puts HTTPS in front, which the web version's service worker needs on other devices
        allowedHosts: ['.ts.net'],
        // Everything except the sources, or web.js and sw.js for the web version, comes from the GWT dev server.
        proxy: {
            '/__sdm': { target: 'http://localhost:9876', rewrite: (url) => url.slice('/__sdm'.length) },
            [`^/(?!${mode === 'web' ? 'web\\.js|sw\\.js|__sdm/|' : ''}@|src/|node_modules/).*`]: {
                target: 'http://localhost:8888',
                // the webview would otherwise keep serving stale CSS and icons after rebuilds
                configure: (proxy) =>
                    proxy.on('proxyRes', (res) => {
                        res.headers['cache-control'] = 'no-store';
                    }),
            },
        },
    },
}));

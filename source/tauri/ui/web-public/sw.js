// Serves pack/ from Cache Storage, where the settings write the active resource pack, theme and performance settings.
const pack = new URL('pack/', location.href).href;

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));
self.addEventListener('fetch', (event) => {
    if (!event.request.url.startsWith(pack)) return;
    event.respondWith(
        caches
            .open('locus')
            .then((cache) => cache.match(event.request, { ignoreSearch: true }))
            .then((response) => response ?? new Response('', { status: 404 })),
    );
});

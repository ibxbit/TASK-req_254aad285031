// Field Service Operations Hub — service worker.
// Fully offline-capable: no CDN fetches, no analytics pings.
//
// Strategy:
//   /api GET requests   -> network-first, fall back to cached response if
//                          the LAN is unreachable (HTTP offline).
//   /api non-GET         -> pass through (writes must hit the server).
//   everything else      -> cache-first, populate on first successful fetch.
// Cache name is versioned so a bump retires stale caches on activate.

const CACHE_NAME = 'fsh-cache-v1';

self.addEventListener('install', () => {
    // Claim control as soon as possible so refreshes pick up SW.
    self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((keys) =>
            Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k)))
        )
    );
    self.clients.claim();
});

self.addEventListener('fetch', (event) => {
    const req = event.request;
    const url = new URL(req.url);

    // Only handle same-origin requests. Anything else (shouldn't exist in an
    // offline build) is passed through untouched.
    if (url.origin !== self.location.origin) return;

    // Writes bypass the cache entirely.
    if (req.method !== 'GET') {
        event.respondWith(fetch(req));
        return;
    }

    if (url.pathname.startsWith('/api/')) {
        // Network-first for live API data.
        event.respondWith(
            fetch(req)
                .then((resp) => {
                    if (resp && resp.ok) {
                        const clone = resp.clone();
                        caches.open(CACHE_NAME).then((c) => c.put(req, clone));
                    }
                    return resp;
                })
                .catch(() =>
                    caches.match(req).then(
                        (cached) =>
                            cached ||
                            new Response(
                                JSON.stringify({ error: 'offline', message: 'No cached response for this request.' }),
                                { status: 503, headers: { 'Content-Type': 'application/json' } }
                            )
                    )
                )
        );
        return;
    }

    // Cache-first for static assets (HTML, JS, WASM, CSS, images).
    event.respondWith(
        caches.match(req).then((cached) => {
            if (cached) return cached;
            return fetch(req).then((resp) => {
                if (resp && resp.ok) {
                    const clone = resp.clone();
                    caches.open(CACHE_NAME).then((c) => c.put(req, clone));
                }
                return resp;
            });
        })
    );
});

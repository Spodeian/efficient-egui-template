// Service Worker for Serverless & Desktop Template
// Strategy:
// - Pages: network first, cached copy when offline.
// - Trunk's content-hashed bundles (name-<16 hex>.js/.wasm/.css): cache first. A given URL
//   never changes, so once cached there is no network request at all.
// - Everything else (manifest, icons): network first, cached copy when offline.
// Each deploy stamps a new BUILD_ID, so a new cache replaces the old one on activation.
const BUILD_ID = '__BUILD_ID__';
const CACHE_NAME = `serverless-desktop-template-cache-${BUILD_ID}`;
const PRECACHE_URLS = /*__PRECACHE_URLS__*/ [];

// Static shell assets always precached alongside build-stamped hashed WASM/JS bundles
const BASE_PRECACHE_ASSETS = [
  '/',
  '/index.html',
  '/manifest.json',
  '/favicon.svg',
  '/favicon.ico'
];

function isHostOrSubdomain(hostname, rootDomain) {
  return hostname === rootDomain || hostname.endsWith(`.${rootDomain}`);
}

// 1. Pre-cache on install with error resilience and activate immediately
self.addEventListener('install', (event) => {
  const urls = Array.from(new Set([...BASE_PRECACHE_ASSETS, ...PRECACHE_URLS]));
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return Promise.allSettled(
        urls.map((asset) =>
          fetch(asset, { cache: 'no-cache' }).then((response) => {
            if (response && response.ok) {
              return cache.put(asset, response);
            }
          }).catch(() => {})
        )
      );
    }).then(() => self.skipWaiting())
  );
});

// 2. Purge all legacy caches on activation and claim clients
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))
      )
    ).then(() => self.clients.claim())
  );
});

// 3. Fetch router: Network-First for HTML documents; Cache-First with Background Revalidation for assets
self.addEventListener('fetch', (event) => {
  // Only handle local same-origin GET requests
  if (event.request.method !== 'GET' || !event.request.url.startsWith(self.location.origin)) {
    return;
  }

  const url = new URL(event.request.url);

  // Bypass third-party analytics, beacon, or challenge endpoints using strict domain boundary matching
  if (
    isHostOrSubdomain(url.hostname, 'cloudflareinsights.com')
    || isHostOrSubdomain(url.hostname, 'google-analytics.com')
    || url.pathname.startsWith('/cdn-cgi/')
  ) {
    return;
  }

  // Never cache the service worker script itself so browser can check for updates in background
  if (url.pathname.endsWith('/sw.js')) {
    event.respondWith(fetch(event.request));
    return;
  }

  const isNavigation = event.request.mode === 'navigate' || event.request.destination === 'document' || url.pathname.endsWith('.html') || url.pathname === '/';
  const isHashedBundle = /-[0-9a-f]{16}(_bg)?\.(js|wasm|css)$/.test(url.pathname);

  event.respondWith(
    caches.open(CACHE_NAME).then(async (cache) => {
      if (isHashedBundle) {
        const cached = await cache.match(event.request);
        if (cached) {
          return cached;
        }
      }
      try {
        const networkResponse = await fetch(event.request);
        if (networkResponse && networkResponse.status === 200) {
          event.waitUntil(cache.put(event.request, networkResponse.clone()));
        }
        return networkResponse;
      } catch (_) {
        const cached = await cache.match(event.request)
          || (isNavigation && (await cache.match('/index.html') || await cache.match('/')));
        if (cached) {
          return cached;
        }
        return new Response('Offline: Network unavailable', {
          status: 503,
          statusText: 'Service Unavailable',
          headers: new Headers({ 'Content-Type': 'text/plain; charset=utf-8' })
        });
      }
    })
  );
});

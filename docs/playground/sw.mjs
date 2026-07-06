// SPDX-License-Identifier: MIT OR Apache-2.0
//
// PurRDF console service worker. Caches the app shell + the colocated wasm
// package on install and serves cache-first, so the console runs fully offline
// after the first load. There is no server-side evaluation to fall back to.

const CACHE = "purrdf-console-v1";

const SHELL = [
  "./",
  "./index.html",
  "./app.mjs",
  "./engine.worker.mjs",
  "./style.css",
  "./manifest.webmanifest",
  "./examples/gallery.mjs",
  "./purrdf/index.mjs",
  "./purrdf/pkg/purrdf_wasm.js",
  "./purrdf/pkg/purrdf_wasm_bg.wasm",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE);
      await cache.addAll(SHELL);
      await self.skipWaiting();
    })(),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)));
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") return;
  event.respondWith(
    (async () => {
      const cached = await caches.match(request);
      if (cached) return cached;
      const response = await fetch(request);
      // Populate the cache opportunistically for same-origin GETs.
      try {
        if (response.ok && new URL(request.url).origin === self.location.origin) {
          const cache = await caches.open(CACHE);
          cache.put(request, response.clone());
        }
      } catch {
        // Caching is best-effort; the live response is still returned.
      }
      return response;
    })(),
  );
});

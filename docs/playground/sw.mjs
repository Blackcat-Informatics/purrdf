// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// PurRDF console service worker. Caches the app shell + the colocated wasm
// package on install and serves cache-first, so the console runs fully offline
// after the first load. There is no server-side evaluation to fall back to.

const CACHE = "purrdf-console-v2";

const SHELL = [
  "./",
  "./index.html",
  "./app.mjs",
  "./engine.worker.mjs",
  "./sarif.mjs",
  "./style.css",
  "./manifest.webmanifest",
  "./examples/gallery.mjs",
  "./purrdf/index.mjs",
  "./purrdf/package.json",
  "./purrdf/build-compiler.txt",
  "./purrdf/pkg/purrdf_jspi.mjs",
  "./purrdf/pkg/purrdf_wasm.js",
  "./purrdf/pkg/purrdf_wasm_bg.wasm",
];

// Keep the complete recipient grants with the offline module. Inventory paths
// are confined to this notice tree; an unexpected URL or corrupt notice refuses
// installation rather than advertising an offline copy without its terms.
async function noticeResponses(relative, profile, compiler) {
  const inventoryURL = new URL(relative, self.registration.scope);
  const response = await fetch(inventoryURL);
  if (!response.ok) throw new Error("recipient inventory could not be fetched");
  const inventory = await response.clone().json();
  if (inventory.schema !== 1 || !inventory.files || typeof inventory.files !== "object" ||
      Array.isArray(inventory.files) || Object.keys(inventory.files).length === 0) {
    throw new Error("invalid recipient inventory");
  }
  if (profile && inventory.profile !== profile) throw new Error("wrong recipient profile");
  const controlling = profile
    ? ["LICENSE-MIT", "LICENSE-APACHE", "LICENSE-MULAN", "CC-BY-4.0.txt", "NOTICE.txt"]
    : ["COPYRIGHT-library.html", "licenses/MIT.txt", "licenses/Apache-2.0.txt"];
  if (controlling.some((path) => !Object.hasOwn(inventory.files, path))) {
    throw new Error("recipient inventory omits controlling texts");
  }
  if (profile) {
    if (!Array.isArray(inventory.components) || inventory.components.length === 0 ||
        inventory.components.some((component) => !Array.isArray(component.notices) ||
          component.notices.length === 0 || component.notices.some((path) => !Object.hasOwn(inventory.files, path)))) {
      throw new Error("recipient inventory omits component texts");
    }
  } else {
    if (inventory.compiler !== compiler) {
      throw new Error("offline compiler notices do not match the module build");
    }
  }
  const base = new URL("./", inventoryURL);
  const responses = [];
  for (const [path, expected] of Object.entries(inventory.files)) {
    const parts = path.split("/");
    if (!path || path.includes("\\") || path.includes("%") || path.includes("?") || path.includes("#") ||
        parts.some((part) => !part || part === "." || part === ".." || part.startsWith(".") || part.startsWith("ROADMAP-")) ||
        !/^[a-f0-9]{64}$/.test(expected)) {
      throw new Error("unsafe recipient inventory path or digest");
    }
    const url = new URL(path, base);
    if (!url.href.startsWith(base.href)) throw new Error("recipient path leaves notice tree");
    const notice = await fetch(url);
    if (!notice.ok) throw new Error("recipient notice could not be fetched");
    const bytes = await notice.clone().arrayBuffer();
    const hash = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))]
      .map((byte) => byte.toString(16).padStart(2, "0")).join("");
    if (hash !== expected) throw new Error("recipient notice digest differs");
    responses.push([url, notice]);
  }
  responses.push([inventoryURL, response]);
  return responses;
}

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const compilerURL = new URL("./purrdf/build-compiler.txt", self.registration.scope);
      const compiler = await fetch(compilerURL);
      if (!compiler.ok) throw new Error("module compiler record could not be fetched");
      const notices = [
        ...await noticeResponses("./purrdf/licenses/inventory.json", "npm", null),
        ...await noticeResponses("./purrdf/licenses/runtime/inventory.json", null, await compiler.clone().text()),
      ];
      const cache = await caches.open(CACHE);
      // No module enters even a partially filled cache before both complete
      // grants are verified. A failed shell fetch still refuses activation.
      await cache.put(compilerURL, compiler);
      for (const [url, response] of notices) await cache.put(url, response);
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
      const cache = await caches.open(CACHE);
      const cached = await cache.match(request);
      if (cached) return cached;
      // Cache only the explicitly inventoried notices and app shell. Caller
      // URLs and other same-origin working material remain live requests.
      return fetch(request);
    })(),
  );
});

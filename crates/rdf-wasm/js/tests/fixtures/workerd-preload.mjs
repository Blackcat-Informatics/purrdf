// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Loaded with `node --import` before anything else runs: gives the process the shape
// the yield primitive meets in a Cloudflare Worker (workerd). There is no
// `setImmediate`; `MessageChannel` exists, but its delivery never returns to the event
// loop — modelled here at its limit, a message is delivered as a microtask — so a job
// yielding through it lets no timer, and no other request, run; and `navigator.userAgent`
// is the documented Workers value.
//
// With `PURRDF_TEST_WORKERS_USER_AGENT=0` the user agent is left as it is (the control:
// the same starving `MessageChannel`, without the Workers signal).

delete globalThis.setImmediate;

class MicrotaskPort {
  onmessage = null;
  other = null;
  postMessage(data) {
    const target = this.other;
    queueMicrotask(() => target.onmessage?.({ data }));
  }
  ref() {}
  unref() {}
  close() {}
}

globalThis.MessageChannel = class MessageChannel {
  constructor() {
    this.port1 = new MicrotaskPort();
    this.port2 = new MicrotaskPort();
    this.port1.other = this.port2;
    this.port2.other = this.port1;
  }
};

if (process.env.PURRDF_TEST_WORKERS_USER_AGENT !== "0") {
  Object.defineProperty(globalThis, "navigator", {
    value: { userAgent: "Cloudflare-Workers" },
    configurable: true,
    writable: true,
  });
}

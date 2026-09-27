// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Loaded with `node --import` before anything else runs: installs a `scheduler.yield`
// with a browser's continuation priority, taken to its limit — the continuation always
// runs ahead of every waiting task, here as a microtask, so no timer, message or I/O
// task ever runs while something keeps yielding through it. A yield primitive that
// picked it would starve the event loop exactly as the browser one does.
//
// With `PURRDF_TEST_KEEP_SET_IMMEDIATE=1` it leaves `setImmediate` alone (the Node
// neighbour); otherwise it deletes it (the browser shape: `scheduler.yield` and
// `MessageChannel`, no `setImmediate`).

globalThis.scheduler = {
  yield: () => new Promise((resolve) => queueMicrotask(resolve)),
};
if (process.env.PURRDF_TEST_KEEP_SET_IMMEDIATE !== "1") {
  delete globalThis.setImmediate;
}

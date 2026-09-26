// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Loaded with `node --import` before anything else runs: removes the two preferred yield
// primitives (`scheduler.yield` and `setImmediate`), so the asynchronous scheduler must
// fall back to a `MessageChannel` round trip — the only one a browser Worker without
// `scheduler.yield` has.

delete globalThis.setImmediate;
delete globalThis.scheduler;

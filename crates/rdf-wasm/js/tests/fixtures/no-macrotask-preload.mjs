// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Loaded with `node --import` before anything else runs: removes the preferred yield
// primitive (`setImmediate`), so the asynchronous scheduler must fall back to a
// `MessageChannel` round trip — the one a browser has.

delete globalThis.setImmediate;

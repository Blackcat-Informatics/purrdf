// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Loaded with `node --import` before anything else runs: removes WebAssembly JavaScript
// Promise Integration, so the child process stands in for an engine that does not
// provide it. The deletion is unconditional — there is no path that tolerates an engine
// lacking it already — and the child asserts the result.

delete WebAssembly.Suspending;
delete WebAssembly.promising;

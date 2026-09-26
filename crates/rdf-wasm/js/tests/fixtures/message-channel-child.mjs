// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the yield-primitive fallback test: run under
// `no-macrotask-preload.mjs`. Prints one JSON line; the parent asserts on it.

import { QueryEngine, asyncYieldPrimitive, hasAsyncQueries, ready } from "../../index.mjs";
import { measureYielding } from "./yield-workload.mjs";

await ready();

const report = {
  setImmediate: typeof globalThis.setImmediate,
  scheduler: typeof globalThis.scheduler,
  hasAsyncQueries: hasAsyncQueries(),
  primitive: asyncYieldPrimitive(),
  ...(await measureYielding(new QueryEngine(), 1000)),
};
process.stdout.write(`${JSON.stringify(report)}\n`);

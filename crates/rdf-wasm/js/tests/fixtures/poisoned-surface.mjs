// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Shared by the poisoning children (`trap-child.mjs`, `panic-child.mjs`) and the tests
// that read their reports: settling calls into comparable records, calling the whole
// package surface once the instance is poisoned, and asserting every entry refused.

import assert from "node:assert/strict";

const describe = (error) => ({ name: error?.name, message: error?.message });
const subjects = (result) =>
  result.rows
    .toArray()
    .map((row) => row.s.value)
    .sort();

/** Settle a call — synchronous or asynchronous — into a comparable record. */
export async function settle(call) {
  try {
    const value = await call();
    return { settled: "resolved", ...(value?.rows === undefined ? {} : { subjects: subjects(value) }) };
  } catch (error) {
    return { settled: "rejected", ...describe(error) };
  }
}

/** Settle a synchronous call without awaiting anything. */
export function settleSync(call) {
  try {
    const value = call();
    return { settled: "returned", value: typeof value === "object" ? typeof value : value };
  } catch (error) {
    return { settled: "threw", ...describe(error) };
  }
}

const isClass = (value) => /^class\b/.test(Function.prototype.toString.call(value));
const hasConstructor = (value) => /\n\s*constructor\(/.test(Function.prototype.toString.call(value));
const GLUE_INTERNALS = new Set(["length", "name", "prototype", "__wrap"]);

/**
 * Call the whole package root: every free function called, every class constructed and
 * every static called, then every method and getter of `dataset` and `engine` (objects
 * created before the instance was poisoned). A class the glue gives no constructor builds
 * an empty handle without touching the instance, before the poisoning as after it; one
 * with a constructor must refuse. Called without arguments: every one of them reaches the
 * instance before it could validate any.
 */
export async function enumerateSurface(purrdf, dataset, engine) {
  const surface = {};
  for (const [name, value] of Object.entries(purrdf)) {
    if (typeof value !== "function") {
      surface[name] = { settled: "not a function", type: typeof value };
      continue;
    }
    if (!isClass(value)) {
      surface[name] = await settle(() => value());
      continue;
    }
    if (hasConstructor(value)) surface[`new ${name}`] = await settle(() => new value());
    for (const key of Object.getOwnPropertyNames(value)) {
      if (GLUE_INTERNALS.has(key) || typeof value[key] !== "function") continue;
      surface[`${name}.${key}`] = await settle(() => value[key]());
    }
  }
  for (const [label, held] of [
    ["dataset", dataset],
    ["engine", engine],
  ]) {
    const prototype = Object.getPrototypeOf(held);
    for (const key of Object.getOwnPropertyNames(prototype)) {
      if (key === "constructor" || key === "free" || key === "__destroy_into_raw" || key.startsWith("__purrdf")) continue;
      const descriptor = Object.getOwnPropertyDescriptor(prototype, key);
      if (descriptor.get !== undefined) surface[`${label}.${key}`] = await settle(() => held[key]);
      else if (typeof descriptor.value === "function") surface[`${label}.${key}()`] = await settle(() => held[key]());
    }
  }
  return surface;
}

/**
 * Assert every entry of an enumerated `surface` refused with `message`, and that the
 * enumeration reached every function `packageRoot` exports, so it cannot pass by being
 * empty.
 */
export function assertSurfacePoisoned(surface, packageRoot, message) {
  const rejected = { settled: "rejected", name: "Error", message };
  for (const [name, outcome] of Object.entries(surface)) {
    assert.deepEqual(outcome, rejected, name);
  }
  for (const [name, value] of Object.entries(packageRoot)) {
    if (typeof value !== "function") continue;
    if (!isClass(value)) assert.ok(name in surface, `${name} was not called`);
    else if (hasConstructor(value)) assert.ok(`new ${name}` in surface, `new ${name} was not called`);
  }
  for (const name of ["new QueryEngine", "new Dataset", "Dataset.parse", "version", "ready", "dataset.size", "engine.query()"]) {
    assert.ok(name in surface, `${name} is missing from the enumerated surface`);
  }
}

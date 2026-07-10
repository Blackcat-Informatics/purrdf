// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { parsePackument } from "./npm-pack-output.mjs";

const packument = {
  filename: "blackcatinformatics-purrdf-0.4.2.tgz",
  size: 100,
  unpackedSize: 200,
  entryCount: 8,
};

test("parsePackument accepts the npm 11 array response", () => {
  assert.deepEqual(parsePackument(JSON.stringify([packument])), packument);
});

test("parsePackument accepts the npm 12 package-name map", () => {
  assert.deepEqual(
    parsePackument(JSON.stringify({ "@blackcatinformatics/purrdf": packument })),
    packument,
  );
});

test("parsePackument rejects zero or multiple package records", () => {
  assert.throws(() => parsePackument("[]"), /exactly one package record/);
  assert.throws(
    () => parsePackument(JSON.stringify([packument, packument])),
    /exactly one package record/,
  );
});

test("parsePackument rejects a malformed package record", () => {
  assert.throws(
    () => parsePackument(JSON.stringify({ purrdf: { filename: "purrdf.tgz" } })),
    /invalid package record/,
  );
});

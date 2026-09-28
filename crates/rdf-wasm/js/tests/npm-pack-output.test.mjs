// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { missingPackedFiles, parsePackument } from "./npm-pack-output.mjs";

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

const manifest = {
  files: ["index.mjs", "pkg/purrdf_jspi.mjs", "pkg/purrdf_wasm.js"],
  exports: { ".": { types: "./index.d.ts", import: "./index.mjs" } },
};
const packedPaths = (...paths) => ({ ...packument, files: paths.map((path) => ({ path })) });

test("missingPackedFiles accepts a tarball holding every promised path", () => {
  assert.deepEqual(
    missingPackedFiles(
      manifest,
      packedPaths("index.mjs", "index.d.ts", "package.json", "pkg/purrdf_jspi.mjs", "pkg/purrdf_wasm.js"),
    ),
    [],
  );
});

test("missingPackedFiles names a files entry npm silently omitted", () => {
  assert.deepEqual(
    missingPackedFiles(manifest, packedPaths("index.mjs", "index.d.ts", "pkg/purrdf_wasm.js")),
    ["pkg/purrdf_jspi.mjs"],
  );
});

test("missingPackedFiles names an exports target the tarball lacks", () => {
  const withSubpath = {
    ...manifest,
    exports: { ...manifest.exports, "./extra": { types: "./extra.d.ts", import: "./extra.mjs" } },
  };
  assert.deepEqual(
    missingPackedFiles(
      withSubpath,
      packedPaths("index.mjs", "index.d.ts", "pkg/purrdf_jspi.mjs", "pkg/purrdf_wasm.js"),
    ),
    ["extra.d.ts", "extra.mjs"],
  );
});

test("missingPackedFiles refuses a pack record without a file list", () => {
  assert.throws(() => missingPackedFiles(manifest, packument), /lists no files/);
});

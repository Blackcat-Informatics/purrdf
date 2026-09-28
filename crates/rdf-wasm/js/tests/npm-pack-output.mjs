// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

export function parsePackument(output, context = "npm pack") {
  const parsed = JSON.parse(output);
  const records = Array.isArray(parsed)
    ? parsed
    : parsed !== null && typeof parsed === "object"
      ? Object.values(parsed)
      : [];
  if (
    records.length !== 1 ||
    records[0] === null ||
    typeof records[0] !== "object"
  ) {
    throw new Error(`${context} did not return exactly one package record`);
  }
  const [record] = records;
  if (
    typeof record.filename !== "string" ||
    !Number.isFinite(record.size) ||
    !Number.isFinite(record.unpackedSize) ||
    !Number.isInteger(record.entryCount)
  ) {
    throw new Error(`${context} returned an invalid package record`);
  }
  return record;
}

// Every path the manifest promises — each `files` entry and each `exports` target — that
// the packed tarball does not contain. `npm pack` silently omits a `files` entry that
// does not exist, so a missing module (the glue's `./purrdf_jspi.mjs` import, say) would
// otherwise ship as a package that fails at import time.
export function missingPackedFiles(manifest, packument) {
  if (!Array.isArray(packument.files)) {
    throw new Error("the npm pack record lists no files");
  }
  const packed = new Set(packument.files.map((file) => file.path));
  const promised = new Set(manifest.files ?? []);
  const collect = (target) => {
    if (typeof target === "string") {
      promised.add(target.replace(/^\.\//, ""));
    } else if (target !== null && typeof target === "object") {
      for (const value of Object.values(target)) collect(value);
    }
  };
  collect(manifest.exports);
  return [...promised].filter((path) => !packed.has(path)).sort();
}

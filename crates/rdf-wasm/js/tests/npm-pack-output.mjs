// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

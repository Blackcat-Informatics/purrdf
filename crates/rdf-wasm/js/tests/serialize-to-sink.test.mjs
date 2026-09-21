// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// `serializeToSink` delivers the same document a window at a time.
//
// The property is not "the bytes are right" — `serialize` already covers that, and
// covered it while nothing streamed. The property is that the bytes LEAVE
// incrementally and that a host refusing them is told, rather than quietly receiving
// half an answer.

import assert from "node:assert/strict";
import test from "node:test";

import { ready, Dataset } from "../index.mjs";

await ready();

function populated(rows) {
  let document = "";
  for (let i = 0; i < rows; i += 1) {
    document += `<https://example.org/s${i}> <https://example.org/p> "value ${i} padded out so the document is comfortably large" .\n`;
  }
  return Dataset.parse(document, "ntriples");
}

test("a sink receives the same bytes serialize returns", () => {
  const dataset = populated(2000);
  const whole = dataset.serialize("ntriples");

  const chunks = [];
  dataset.serializeToSink("ntriples", null, {
    write(chunk) {
      chunks.push(Uint8Array.from(chunk));
    },
  });

  assert.ok(chunks.length > 1, "the document must arrive in more than one window");
  const joined = Buffer.concat(chunks.map((c) => Buffer.from(c))).toString("utf8");
  assert.equal(joined, whole, "streamed bytes must equal the eager string");
});

test("a sink that throws aborts instead of truncating", () => {
  const dataset = populated(2000);
  let seen = 0;
  assert.throws(
    () =>
      dataset.serializeToSink("ntriples", null, {
        write() {
          seen += 1;
          throw new Error("the host refused this window");
        },
      }),
    /refused|threw/,
    "a throwing sink must surface, not be swallowed",
  );
  assert.equal(seen, 1, "the serializer must stop at the first refusal");
});

test("an object without write is refused rather than silently producing nothing", () => {
  const dataset = populated(10);
  assert.throws(
    () => dataset.serializeToSink("ntriples", null, {}),
    "a sink with no write method is a caller error",
  );
});

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The event-loop yielding workload the asynchronous tests share: a quadratic self-join
// whose every candidate pair is a governor poll, so a job over it polls — and, at a small
// `yieldEveryPolls`, yields — many times.

import { Dataset } from "../../index.mjs";

const EX = "http://example.org/";

/** A dataset of `size` integer-valued nodes: the operand of a quadratic self-join. */
export function crossDataset(size) {
  const lines = [];
  for (let index = 0; index < size; index += 1) {
    lines.push(`<${EX}n${index}> <${EX}v> "${index}"^^<http://www.w3.org/2001/XMLSchema#integer> .`);
  }
  return Dataset.parse(`${lines.join("\n")}\n`, "nquads");
}

/**
 * A cross product folded to one row: size·(size−1)/2 pairs pass the filter, and every
 * candidate pair is a poll.
 */
export const CROSS_COUNT = `SELECT (COUNT(*) AS ?c) WHERE { ?a <${EX}v> ?x . ?b <${EX}v> ?y . FILTER(?x < ?y) }`;

/** The answer `CROSS_COUNT` must give over `crossDataset(size)`. */
export function expectedCrossCount(size) {
  return String((size * (size - 1)) / 2);
}

// SPDX-License-Identifier: MIT OR Apache-2.0
//
// SARIF contract helpers for the PurRDF console. Side-effect-free and
// Node-importable (no DOM, no Worker) so the smoke suite can assert the SARIF
// version contract directly. The console renders SHACL results as SARIF; this
// module is the single source of truth for the SARIF version the UI accepts —
// a version drift is asserted here and surfaced, NEVER silently echoed.

/** The one SARIF schema version the console understands. */
export const EXPECTED_SARIF_VERSION = "2.1.0";

/**
 * Assert a SARIF payload's version against the console's contract.
 *
 * @param {string|undefined} version the `version` field of a SARIF log
 * @returns {string|null} `null` when it matches {@link EXPECTED_SARIF_VERSION};
 *   otherwise a human-readable warning naming both the seen and expected
 *   versions, for the caller to surface visibly.
 */
export function assertSarifVersion(version) {
  if (version === EXPECTED_SARIF_VERSION) return null;
  return `unexpected SARIF version ${version ?? "(none)"} — the console expects SARIF ${EXPECTED_SARIF_VERSION}`;
}

/**
 * One-line summary of a SARIF payload for the results header.
 * @param {{version?: string, runs?: Array<{results?: unknown[]}>}} sarif
 * @returns {string}
 */
export function describeSarif(sarif) {
  const results = sarif?.runs?.[0]?.results ?? [];
  return `SARIF ${sarif?.version ?? "(none)"} · ${results.length} result(s).`;
}

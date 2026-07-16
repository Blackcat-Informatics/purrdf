<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Conformance & Testing

Every PurRDF engine is gated by its official test suite. The suites are
vendored and **byte-frozen** in-repo — never hand-edited, SHA-256-verified on
every `make check` so a silent content edit fails the build. The full, live
scoreboard is
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md);
this chapter explains how the machine works.

## The single conformance matrix

The native Rust W3C harnesses **and** the Python rdflib drop-in gate are
reported together as one scoreboard:

```sh
make conformance    # aggregates every suite into one table
```

The aggregator runs each suite in a fixed order and prints per-suite
**pass / xfail-or-skip / fail** counts with an overall RED/GREEN verdict. It
exits non-zero on any unexpected failure, and the rendered matrix in
`docs/CONFORMANCE.md` is itself drift-guarded in CI (the gate fails if the
committed block is stale).

## What is gated

| Engine | Suite |
| --- | --- |
| IRI (RFC 3987) | W3C IRI + RFC 3986 §5.4 resolution vectors |
| Syntax codecs | W3C rdf-tests (Turtle/TriG/N-Triples/N-Quads/RDF-XML) |
| CSVW | W3C RDF-conversion and metadata-validation manifests plus a locked independent implementation |
| OBO Graphs | official OBO Graphs 0.3.2 JSON Schema plus corruption probes |
| RDFC-1.0 | W3C rdf-canon fixtures |
| SPARQL 1.1/1.2 | full W3C sparql11 + sparql12 + entailment suites |
| SHACL | W3C data-shapes + DASH SHACL-AF/rules + a first-party frozen corpus |
| ShEx 2.1 | shexTest v2.1.0 (validation, schemas, negative syntax/structure) |
| Entailment | the W3C entailment cases (via the SPARQL harness) |
| GTS | frozen cross-language vectors, byte-exact |
| rdflib drop-in | rdflib 7.6's own vendored tests + first-party parity |

At the time of writing every suite is green — for example 1,105/1,105
attempted shexTest validation cases, 126/126 W3C SHACL, 250/250 codec
round-trips, and 70/70 entailment cases — with the handful of remaining
non-passes strictly ledgered (e.g. five SPARQL fixtures with upstream-errata
non-canonical XSD lexicals). Always read the current numbers from
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)
rather than this snapshot.

## Ledger discipline

A harness never skips silently. Four mechanisms keep the scoreboard honest:

1. **Exact totals** — each harness asserts the number of discovered tests, so
   corpus drift fails loudly.
2. **XFAIL ledgers** — every known gap is listed with a reason string, and the
   harness asserts it *still fails*. Fixing a gap without removing its ledger
   entry breaks the build (XPASS discipline), so the ledgers double as
   roadmaps.
3. **Trait skips (ShEx only)** — whole spec features can be skipped by
   manifest trait tags, counted exactly. The list is currently **empty**.
4. **A monotone budget (ratchet)** — a committed baseline records the exact
   allowed count of ledgered gaps per suite. A larger live count fails RED
   (regression), and a *smaller* one also fails RED until the budget is
   lowered to lock the gain in. The budget may only ever be edited downward,
   which makes "the skip list only shrinks" a mechanical guarantee rather
   than a convention.

## Running the suites locally

```sh
make conformance                                     # the single matrix
cargo test -p purrdf-shex                            # all four ShEx suites
cargo test -p purrdf-shapes --test w3c_conformance   # W3C SHACL scoreboard
cargo test -p purrdf-sparql-conformance              # W3C SPARQL
cargo test -p purrdf-rdf                             # RDFC-1.0 + codec goldens
make projection-oracles                             # W3C CSVW + independent CSVW/OBO checks
cargo test -p purrdf-gts                             # GTS vectors
```

`make check` — the full local gate (fmt, clippy, build, tests, hygiene) —
runs the Rust suites as part of the workspace gate.

## Frozen means frozen

The GTS vectors in `vectors/` are shared byte-exact with the sibling GTS
engines in other languages and are **never** regenerated in this repository;
the wire format is governed in
[`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts). The same
never-hand-edit rule applies to every vendored corpus and everything under
`generated/`.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-sparql-governors` — normative vector corpus

The executable half of the execution-governor profile. A fuel budget is a number
about *one* charge schedule, so a consumer that pins **(profile id, profile
version, profile digest, corpus digest)** needs a body of evidence saying what
those numbers buy. This corpus is that evidence: running it against a linked
build produces a **receipt** rather than a promise.

First-party, not vendored. Run by
`crates/sparql-conformance/tests/governor_corpus.rs`.

## Identity

The corpus is content-addressed. Its digest is the SHA-256 of its freeze
manifest:

```sh
sha256sum scripts/conformance-frozen/vectors-sparql-governors.sha256
```

That value is pinned in the library as
`purrdf_sparql_eval::GOVERNOR_CORPUS_DIGEST`, which **derives** it from the
freeze manifest and compares — `the_corpus_digest_is_a_well_formed_reproducible_sha256`
in `crates/sparql-eval/src/governor/mod.rs` — so the corpus cannot change
without the constant being re-pinned in the same commit.

The freeze manifest covers every payload byte under this directory
(`scripts/check-corpus-frozen.py`), so a silently edited expectation fails the
build rather than passing it. This `README.md` is deliberately **not** covered —
sidecars are excluded from freeze manifests so editing prose never requires
regenerating a digest.

## Layout

| Path | Role |
|---|---|
| `manifest.tsv` | the case list: inputs, source, governors, band, outcome |
| `transport.tsv` | injected-transport wiring and exchange count, for federated cases |
| `cases/<name>.ttl` | input dataset, in Turtle 1.2 |
| `cases/<name>.rq` | input query |
| `cases/<name>.<endpoint>.srj` | a federated endpoint's pinned SPARQL-results JSON response |
| `expected/<case>.answer` | the certified rows and their certificate class |
| `expected/<case>.spend` | what the execution consumed, per dimension |
| `expected/<case>.metered` | what the same case costs under `QueryGovernors::METERED` |

A case's data syntax is taken from its **extension**, not from the manifest, so
a fixture cannot be listed under a syntax it is not written in. Endpoint
responses are found from the query fixture's stem (`cases/federated.rq` finds
`cases/federated.a.srj`), so a response cannot be attached to a case it does not
belong to.

Several cases share one `(data, query)` pair — that is the point of the band
matrix: the *only* thing that differs between `fuel-zero`, `fuel-boundary` and
`fuel-over-bound` is the ceiling.

## Three records, not one

Each case pins three facts, and keeping them apart is deliberate:

1. **the outcome discriminant** (`manifest.tsv`'s last column) — completed, or
   which governor stopped it, spelled in the kernel's own
   `TrippedGovernor::label` vocabulary;
2. **the certified rows and their certificate class** (`.answer`) — what the
   caller receives, and what those rows are licensed to be used as
   (`certain` / `at-most` / `unknown`, plus whether they are a positional
   prefix);
3. **what the execution spent** (`.spend`), recorded **independently of the
   rows**.

The third is the one easy to omit and impossible to reconstruct afterwards. A
corpus that pinned only rows could not detect a charge-schedule change that
happened to cut in the same place: the answer would be unchanged, the receipt
would not, and every budget a consumer sized against the old schedule would be
silently wrong while the corpus stayed green.

## The boundary is measured, never guessed

`expected/<case>.metered` is the consumption vector of the same case run under
`QueryGovernors::METERED`, which engages every counter and bounds nothing. The
band columns are defined against it:

| Band | Ceiling | Must |
|---|---|---|
| `zero` | `0` | trip — a zero ceiling is valid and admits no charged work |
| `boundary` | exactly the metered cost | **complete** — the ceiling is inclusive |
| `over-bound` | the metered cost minus one | **trip** |

The harness re-measures and re-derives that relation on every run rather than
trusting the numbers in the file, so a charge-schedule change cannot leave a
stale boundary behind looking authoritative: the metered cost moves, and the
manifest's ceiling stops being the boundary it claims to be.

## What determinism is claimed for — and what is not

Charging is deterministic by construction on **fuel**, the **answer cap** and
the **intermediate-cell** peak, and the scratch-arena and remote-request
counters are functions of the same fixed inputs. Every case above pins rows and
a cost on that basis, and `the_corpus_is_reproducible_within_a_run` re-runs the
whole corpus to check it.

A **wall deadline** is time-dependent and carries no such claim. The single
deadline case (`deadline-zero-budget`) therefore pins exactly what is
guaranteed — that a trip happened, and that it named the deadline — and has no
`.answer`, `.spend` or `.metered` sidecar at all. Publishing bytes for it would
be publishing a promise this engine does not make.

A **cancellation** is not a clock, so cancellation cases are pinned in full.

## Case inventory

### The band matrix — one zero, one boundary, one over-bound per governor

| Lane | Inputs | Covers |
|---|---|---|
| `fuel-*` | `chain` | abstract execution steps |
| `answer-rows-*` | `chain` | what the query commits to its answer sequence |
| `intermediate-cells-*` | `join` | the largest intermediate bag; the zero and over-bound cases are refused at **admission**, because the planner's estimate already exceeds the ceiling |
| `scratch-bytes-*` | `concat` | arena growth, which is independent of every row and cell count |
| `remote-requests-*` | `federated` | requests issued to a federated endpoint |

### RDF 1.2 — the statement layer is inside the perimeter

| Case | Covers |
|---|---|
| `rdf12-reifier-fuel-boundary` / `-over-bound` | the reifier-layer expansion is charged, so a query over the virtual layer is bounded like any other |
| `rdf12-reifier-answer-boundary` / `-over-bound` | the answer cap over reifier solutions, and the certificate the truncation carries |
| `rdf12-construct-answer-zero` / `-boundary` / `-over-bound` | the answer cap denominating **output statements** over a CONSTRUCTed reification layer: three solutions become six statements, and a cap of five keeps five |

A reification layer is a whole encoding a query can be written around — reifier
and annotation rows live in side tables — so a governor that counted only quads
would report a satisfied ceiling for a query that read, or emitted, an unbounded
number of them.

### The `SERVICE` transport seam

`HttpRequest::stop` can only be honoured by a transport capable of abandoning a
call it is already inside, and nothing forces a host to write one.

| Case | Covers |
|---|---|
| `service-deaf-transport-request-ceiling` | a transport that never reads the signal, bounded anyway by the remote-request ceiling: exactly one exchange is performed |
| `service-deaf-transport-cancel-mid-exchange` | the host cancels while the evaluator is blocked inside a deaf transport |
| `service-honouring-transport-cancel-mid-exchange` | the same, through a transport that does poll the signal |

The last two reach the **same** outcome and the **same** exchange count, which
is the claim: ignoring `stop` degrades to per-request granularity, not to
unboundedness. The one thing that differs is pinned rather than smoothed over —
an abandoned exchange leaves a resumable positional prefix, a
completed-then-discarded one withdraws that licence.

### Deadlines

| Case | Covers |
|---|---|
| `deadline-zero-budget` | a zero-budget wall deadline trips and names the deadline — and nothing further is pinned |

## Regenerating

```sh
PURRDF_UPDATE_GOVERNOR_CORPUS=1 \
  cargo test -p purrdf-sparql-conformance --test governor_corpus
python3 scripts/check-corpus-frozen.py --update
# then re-pin GOVERNOR_CORPUS_DIGEST from the sha256sum above
```

Deliberately three steps, not one. A regeneration that moved a charge is a
`GOVERNOR_PROFILE_VERSION` change, and the friction is what keeps an accidental
refresh from being mistaken for a no-op.

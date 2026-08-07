<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-sparql-governors` — normative vector corpus

The executable half of
[`docs/SPARQL-GOVERNOR-PROFILE.md`](../../docs/SPARQL-GOVERNOR-PROFILE.md). A fuel budget is a number
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
| `manifest.tsv` | the case list: inputs, source, governors (numeric ceilings, stop signals, or injected deadline-poll ceilings), band, outcome |
| `transport.tsv` | injected-transport wiring and exchange count, for federated cases |
| `relations.tsv` | scripted property-function wiring, invocation count and pull count, for relation cases |
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

For a numeric ceiling, `expected/<case>.metered` is the consumption vector of
the same case run under `QueryGovernors::METERED`, which engages every counter
and bounds nothing. For an injected deadline, the harness instead counts the
complete run's stop-signal polls with a never-firing deterministic signal. The
band columns are defined against the applicable measurement:

| Band | Ceiling | Must |
|---|---|---|
| `zero` | `0` | trip — a zero ceiling is valid and admits no charged work |
| `boundary` | exactly the metered cost or complete-run poll count | **complete** — the ceiling is inclusive |
| `over-bound` | the applicable measurement minus one | **trip** |

The harness re-measures and re-derives that relation on every run rather than
trusting the numbers in the file, so a charge- or stop-poll-schedule change
cannot leave a stale boundary behind looking authoritative: the measurement
moves, and the manifest's ceiling stops being the boundary it claims to be.

## What determinism is claimed for — and what is not

Charging is deterministic by construction on **fuel**, the **answer cap** and
the **intermediate-cell** peak, and the scratch-arena and remote-request
counters are functions of the same fixed inputs. Every case above pins rows and
a cost on that basis, and `the_corpus_is_reproducible_within_a_run` re-runs the
whole corpus to check it.

A caller-supplied stop signal can be deterministic even when its cause is
`deadline`. The `deadline-injected-*` cases fire solely after a fixed number of
polls, so their zero, boundary and over-bound outcomes, answers and receipts are
pinned in full. They grade the evaluator's poll schedule, not elapsed time.

A **wall deadline** is time-dependent and carries no such claim. The separate
`deadline-zero-budget` smoke case therefore pins exactly what is guaranteed —
that a trip happened, and that it named the deadline — and has no `.answer`,
`.spend` or `.metered` sidecar at all. Publishing bytes for it would be
publishing a promise this engine does not make.

A **cancellation** is not a clock, so cancellation cases are pinned in full.

## Case inventory

### The band matrix — one zero, one boundary, one over-bound per deterministic lane

| Lane | Inputs | Covers |
|---|---|---|
| `fuel-*` | `chain` | abstract execution steps |
| `property-function-fuel-*` | `property-function` | the two property-function charge points: one per invocation of a host relation, one per row it emits and this engine accepts |
| `answer-rows-*` | `chain` | what the query commits to its answer sequence |
| `intermediate-cells-*` | `join` | the largest intermediate bag; the zero and over-bound cases are refused at **admission**, because the planner's estimate already exceeds the ceiling |
| `scratch-bytes-*` | `concat` | arena growth, which is independent of every row and cell count |
| `remote-requests-*` | `federated` | requests issued to a federated endpoint |
| `deadline-injected-*` | `chain` | deterministic stop-signal polling, independent of wall time |

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
an abandoned exchange preserves the positional-prefix relation needed for
deterministic resumption, while a completed-then-discarded one withdraws it.

### The property-function relation seam

`PfCursor::next` is handed no signal it is obliged to read, and nothing forces a
host to write a cursor that stops early. The evaluator polls before opening a
cursor and between successive pulls, so the seam is bounded per **invocation**
whatever the relation does.

| Case | Covers |
|---|---|
| `property-function-deaf-relation-cancel-mid-invocation` | the host cancels during the relation's second pull, through a cursor that never reads the signal |
| `property-function-cooperating-relation-cancel-mid-invocation` | the same timeline, through a cursor that polls it and abandons the invocation |

The two answers are **byte-identical** — same rows, same certificate — and that
is a stronger statement than the `SERVICE` pair's. A relation's output is a row
stream rather than one atomic exchange, and every row of it crosses the engine's
per-row admission point on the way into the bag; that point is also a bounded
work checkpoint, so a fired signal is observed there whether or not the cursor
ever looked at it. The deaf cursor's extra row is therefore pulled and then
*refused*, never ingested: ignoring the signal buys the relation nothing and
costs the caller nothing.

`relations.tsv` records the invocation and pull counts for the same reason
`transport.tsv` records exchange counts: they are the only observations that
separate "the ceiling prevented the work" from "the work was done and its rows
discarded". Here they are what makes the paragraph above checkable — the deaf
case is pulled exactly once more than it delivers.

### Deadlines

| Case | Covers |
|---|---|
| `deadline-injected-zero` | a deadline signal firing on its first poll stops before output |
| `deadline-injected-boundary` | a ceiling equal to the complete run's measured poll count completes |
| `deadline-injected-over-bound` | one poll fewer trips, exposing a moved or missing terminal poll |
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

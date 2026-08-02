<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-rdfc12` v1 — normative vector corpus

The executable half of [`docs/RDF12-CANON-PROFILE.md`](../../docs/RDF12-CANON-PROFILE.md).
Every clause a consumer pins has a case here, so running this corpus against a
linked build produces a **receipt** rather than a promise.

First-party, not vendored. Run by `crates/rdf/tests/rdf12_canon_profile.rs`.

## Identity

The corpus is content-addressed. Its digest is the SHA-256 of its freeze manifest:

```sh
sha256sum scripts/conformance-frozen/vectors-rdf12-canon.sha256
```

That value is pinned in the library as `purrdf_core::CANON_CORPUS_DIGEST` and
asserted by the harness, so the corpus cannot change without the constant being
re-pinned in the same commit. A consumer pins **(profile id, profile version,
corpus digest)** and checks all three against the artifact it linked.

The freeze manifest covers every payload byte under this directory
(`scripts/check-corpus-frozen.py`), so a silently edited expectation fails the
build rather than passing it. This `README.md` is deliberately **not** covered —
sidecars are excluded from freeze manifests so editing prose never requires
regenerating a digest.

## Layout

| Path | Role |
|---|---|
| `manifest.tsv` | the case list: input file, kind, expectation |
| `pairs.tsv` | relations BETWEEN cases (`same` / `differ`) |
| `cases/<name>.ttl` or `.trig` | input, in Turtle 1.2 / TriG 1.2 |
| `cases/<name>.canonical` | expected canonical bytes (goldens only) |
| `cases/<name>.digest` | SHA-256 of those bytes (goldens only) |

A case's syntax is taken from its **extension**, not from the manifest, so a case
cannot be listed under a syntax it is not written in.

Refusal cases have no `.canonical` — their expectation is the exact typed
discriminant, recorded in `manifest.tsv` as either
`reserved-vocabulary <position> <iri>` (profile §5) or `budget-exceeded` (§6).
The position is part of the expectation because §5.3 requires the *diagnostic* to
be deterministic, not merely the refusal.

The digest sidecar is recorded independently of the bytes rather than derived from
them at read time. Deriving it would make the file decorative; it exists so a
consumer can compare a digest it computed itself against one this corpus
published, and that requires two independent records.

## What the goldens prove — and what they do not

The expected canonical bytes are **generated from this implementation**. They are
therefore evidence of **stability**, not of correctness: they cannot tell you the
algorithm is right, only that it has not moved.

That is what a pinning corpus is for. A consumer minting identity from these bytes
needs to know they will not shift under it, and the goldens make any change that
shifts them impossible to land quietly.

Correctness evidence lives elsewhere and is deliberately not duplicated:

* the **RDF 1.1 subset** is gated against the vendored W3C `rdf-canon` suite
  (`crates/rdf/tests/rdfc_w3c.rs`);
* the **overlay's properties** — isomorphism, reifier-count observability, the
  refusal rule and its determinism — are asserted as relations in `pairs.tsv` and
  as unit tests in `purrdf-core`.

## Case inventory

### Goldens — RDF 1.1 agreement subset

| Case | Covers |
|---|---|
| `plain-rdf11` | ground triples, plain and typed literals |
| `blank-nodes-across-graphs` | blanks shared and distinct across named graphs |
| `isomorphic-a` / `isomorphic-b` | same structure, different blank labels — **must match** |
| `near-isomorphic-a` / `near-isomorphic-b` | one edge relabelled — **must differ** |

### Goldens — the RDF 1.2 overlay

| Case | Covers |
|---|---|
| `reifier-simple` | a single reifier lowered through the sentinel |
| `reifier-nested` | a reifier over a statement that is itself reified |
| `reifier-count-two` | reifier COUNT stays observable (differs from `reifier-simple`) |
| `annotation-simple` | an annotation in the default graph |
| `annotation-named-graph` | an annotation scoped to a named graph (the five-token row, profile §3.1) |
| `triple-term-object` | a quoted triple in object position |
| `triple-term-nested` | a quoted triple inside a quoted triple |
| `triple-term-blank-inside` | blank nodes labelled through a quoted triple |

### Goldens — literal discipline

| Case | Covers |
|---|---|
| `literal-forms` | `"0.70"` ≠ `"0.7"`; lexical forms never normalized |
| `directional-literals` | `@en--ltr` ≠ `@en--rtl` ≠ `@en` |
| `unicode-lexical-forms` | NFC vs NFD are distinct; astral planes survive |

### Refusals — reserved vocabulary (profile §5)

| Case | Covers |
|---|---|
| `poison-forgery` | **the attack**: the lowered reifier row asserted literally |
| `poison-sentinel-subject` / `-predicate` / `-object` / `-graph` | each quad position |
| `poison-sentinel-nested` | reserved IRI inside a quoted triple |
| `poison-sentinel-datatype` | reserved IRI as a literal's datatype |
| `poison-sentinel-unminted` | a name in the namespace the overlay has never minted — the reservation is over the NAMESPACE, not the two sentinels |

`poison-forgery` asserts, as an ordinary quad, exactly the row `reifier-simple`
lowers to. Its test checks **both** halves — that the genuine structure still
produces that row, and that the literal assertion of it is refused — so it cannot
keep passing by quietly ceasing to be a forgery.

### Refusals — complexity poisoning (profile §6)

| Case | Covers |
|---|---|
| `poison-complexity` | a fully symmetric blank graph: bounded refusal, never a hang |

## Regenerating

```sh
PURRDF_UPDATE_CANON_CORPUS=1 cargo test -p purrdf-rdf --test rdf12_canon_profile
python3 scripts/check-corpus-frozen.py --update
# then re-pin CANON_CORPUS_DIGEST from the sha256sum above
```

Deliberately three steps, not one. Per profile §7 a change that moves canonical
bytes also **requires** a `CANON_PROFILE_VERSION` increment — the friction is what
keeps an accidental golden refresh from being mistaken for a no-op.

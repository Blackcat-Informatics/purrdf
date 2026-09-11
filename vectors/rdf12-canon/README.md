<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-rdfc12` v2 — normative vector corpus

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

`manifest.tsv` and `pairs.tsv` are the AUTHORING source and are maintained by
hand; `.canonical` and `.digest` are generated (see **Regenerating** below) and are
never hand-written.

Refusal cases have no `.canonical` — their expectation is the exact typed
discriminant, recorded in `manifest.tsv` as either
`reserved-vocabulary <position> <iri>` (profile §5) or `budget-exceeded` (§6).
The position is part of the expectation because §5.3 requires the *diagnostic* to
be deterministic, not merely the refusal. A golden carries no expectation column at
all, and the harness holds each kind to its own shape: a golden with an expectation
and a refusal without a discriminant are both malformed rows.

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

### Goldens — the fold (profile §3.1)

Two cases live in the reserved namespace and are **admitted**, because each is
written in exactly one of the two shapes the overlay lowers *into*. A quad in that
shape is not a forgery of a statement-layer row; it **is** that row written out, so
it is folded back into the statement layer rather than refused — which is what
makes canonicalization idempotent over its own output.

| Case | Covers |
|---|---|
| `poison-forgery` | the lowered reifier row, written out as an ordinary quad — folds to the reifier binding it spells |
| `poison-sentinel-graph` | the annotation sentinel as a lone graph slot — folds to the annotation row it spells |

Both keep their `poison-` names deliberately. They were refusals under v1, and a
reader tracing why the profile version moved should find them where the attack
inventory is, not renamed out of sight.

`poison-forgery` writes out, as an ordinary quad, exactly the reifier row that
`reifier-simple` lowers to. Its test checks **both** halves — that the genuine
structure still produces that row, and that writing the row out canonicalizes to
it — so the case cannot keep passing by quietly ceasing to spell anything real. It
is paired in `pairs.tsv` against `reifier-simple` as **differ**, because spelling
the reifier row is not the same content as asserting the triple *and* reifying it:
the fold reads the row it is given and never fabricates the assertion alongside it.
`poison-sentinel-graph` is paired against `annotation-simple` for the same reason.

The same test pins the two nearest misses, which must still refuse: the sentinel
predicate over a **non-triple** object, and a folded row carrying a reserved IRI in
a slot the fold does not consume (inside its triple term). The fold is exactly two
shapes wide, and those two cases are what keeps it from widening.

### Refusals — reserved vocabulary (profile §5)

| Case | Covers |
|---|---|
| `poison-sentinel-subject` / `-predicate` / `-object` | each quad position outside a folded shape |
| `poison-sentinel-nested` | reserved IRI inside a quoted triple |
| `poison-sentinel-datatype` | reserved IRI as a literal's datatype |
| `poison-sentinel-unminted` | a name in the namespace the overlay has never minted — the reservation is over the NAMESPACE, not the two sentinels |

Every use of the reserved namespace other than the two folded shapes above refuses
exactly as it did under v1, with the same position-bearing discriminant.

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
bytes — or that changes WHICH inputs are admitted, as reclassifying a case between
`golden` and `refusal` does — also **requires** a `CANON_PROFILE_VERSION`
increment; the friction is what keeps an accidental golden refresh from being
mistaken for a no-op.

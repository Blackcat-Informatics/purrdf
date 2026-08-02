<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-rdfc12` — RDF 1.2 Canonicalization Profile

**Profile identifier:** `purrdf-rdfc12` &nbsp;·&nbsp; **Profile version:** 1 &nbsp;·&nbsp;
**Editor:** Patrick Audley, Blackcat Informatics® Inc.

Both values are readable from the library rather than only from this document —
`purrdf_core::CANON_PROFILE_ID` and `purrdf_core::CANON_PROFILE_VERSION` — so a
consumer can assert that the build it linked is the profile it pinned.

## Abstract

This profile specifies the canonical byte form PurRDF produces for an RDF 1.2
dataset. It takes W3C RDF Dataset Canonicalization (RDFC-1.0) as its base and
extends it to cover the three RDF 1.2 constructs RDFC-1.0 does not address:
reifiers, annotations, and triple terms. It also specifies a **reserved
vocabulary** and the **refusal** of any input that carries it, which is what makes
the canonical bytes safe to mint an identity from.

## 1. Status: this is NOT RDFC-1.0

A digest taken over this profile's output **must never be labelled "RDFC-1.0"**.

The profile differs from RDFC-1.0 in two directions, and both matter:

* **It accepts less.** RDFC-1.0 canonicalizes any well-formed dataset. This profile
  refuses datasets carrying the reserved vocabulary of §4 (§5).
* **It produces different bytes.** A dataset carrying reifiers or annotations
  canonicalizes here to bytes an RDFC-1.0 implementation would not produce, because
  RDFC-1.0 has no notion of those constructs to emit (§3).

On the **RDF 1.1 subset** — no reifiers, no annotations, no triple terms, and no
reserved IRIs — this profile agrees with RDFC-1.0 byte for byte. That agreement is
not a claim; it is gated by the vendored W3C `rdf-canon` suite on every commit.

## 2. Base algorithm

RDFC-1.0 in full, including *Hash First Degree Quads* (§4.6), initial canonical
assignment (§4.4), and *Hash N-Degree Quads* (§4.8) with *Hash Related Blank Node*
(§4.7) and permutation backtracking. Blank nodes receive canonical labels
`c14n0`, `c14n1`, … Output is the set of statement lines, each `\n`-terminated,
**sorted bytewise ascending and deduplicated**.

The hash is SHA-256 by default; SHA-384 is selectable
(`purrdf_core::CanonHash::Sha384`) and corresponds to RDFC-1.0 §3. **The hash
algorithm is part of the identity**: bytes produced under SHA-384 are not
comparable to bytes produced under SHA-256, and a consumer pinning this profile
must pin the algorithm alongside it.

RDFC-1.0 canonicalizes **blank node labels only**. Literal lexical forms,
datatype IRIs, language tags and base directions are emitted verbatim and are
never normalized: `"0.70"` ≠ `"0.7"`, `@en--ltr` ≠ `@en--rtl`, and no Unicode
normalization is applied to any lexical form. Two datasets differing only in a
lexical form are different datasets under this profile.

## 3. The RDF 1.2 overlay

Every statement is normalized into a quad shape before hashing and before
serialization. Genuine quads lower to themselves. The two overlays lower as
follows, where `⟨…⟩` denotes an IRI written in N-Triples form:

| Construct | Canonical row |
|---|---|
| Reifier `(r, t)` in the default graph | `r ⟨urn:purrdf:rdfc:reifies⟩ t .` |
| Reifier `(r, t)` in named graph `g` | `r ⟨urn:purrdf:rdfc:reifies⟩ t g .` |
| Annotation `(r, p, o)` in the default graph | `r p o ⟨urn:purrdf:rdfc:annotation⟩ .` |
| Annotation `(r, p, o)` in named graph `g` | `r p o ⟨urn:purrdf:rdfc:annotation⟩ g .` |

`t` is the triple term itself, written in RDF 1.2 form (`<<( s p o )>>`). Triple
terms nest, and a triple term in any position is written out in full rather than
being replaced by a stand-in.

Because the overlay rows are disjoint from genuine quads, **reifier count and
annotation presence stay observable in the canonical form**. Two datasets
differing only in the number of reifiers, or only in the presence of an
annotation, canonicalize to different bytes. That is the lossless-identity
property the overlay exists to deliver, and it is the property §5's refusal rule
protects.

### 3.1 The output is a canonical BYTE STRING, not a re-parseable document

The named-graph annotation row emits **five** tokens before the terminating
`.` — subject, predicate, object, the annotation sentinel, and the graph term.
That is deliberately not valid N-Quads: a genuine quad never carries two graph
tokens, so the shape cannot collide with one, which is exactly what keeps a
named-graph annotation lossless.

Consumers must therefore treat the output as **an opaque canonical byte string to
be compared or digested**, not as a document to feed back to an N-Quads parser. On
the RDF 1.1 subset the output *is* valid canonical N-Quads, which is why the W3C
suite can gate it.

### 3.2 Blank nodes across the overlay

Blank nodes are labelled over the **whole dataset including the overlay rows**,
not over the genuine quads alone. A blank node appearing only as a reifier, only
inside a triple term, or only as an annotation's graph still participates in
canonical labelling and still contributes its incidence to the n-degree search.

This is what makes the overlay hold under isomorphism: two datasets that differ
only by a renaming of blank nodes canonicalize identically **even when the
renamed blank appears only inside a quoted triple term**, and two datasets whose
blank wiring genuinely differs canonicalize differently.

## 4. Reserved vocabulary

The IRI namespace

```
urn:purrdf:rdfc:
```

is **reserved by this profile**, and is exported as
`purrdf_core::RESERVED_NAMESPACE`. Version 1 lowers into exactly two names within
it — `urn:purrdf:rdfc:reifies` and `urn:purrdf:rdfc:annotation` — but the
reservation covers **the whole namespace**, not those two names.

The reservation is stated over the namespace on purpose. An enumeration would have
to be re-audited every time the overlay grows a row, and a rule whose soundness
depends on an audit nobody schedules is not a rule. A namespace reservation is a
single sentence that can be checked against the entire implementation, and it costs
nothing to hold: PurRDF publishes no vocabulary, mints no ontology terms, and
nothing legitimate lives under this prefix.

## 5. Refusal rule (normative)

> A dataset in which **any term, in any position, is an IRI beginning with
> `urn:purrdf:rdfc:`** is INADMISSIBLE and MUST be refused. It MUST NOT be
> canonicalized, and no digest may be minted from it.

"Any position" means all of: subject, predicate, object and graph; **nested inside
a triple term** at any depth; and **a literal's datatype IRI**. The datatype slot
is swept even though the overlay never lowers a sentinel into one — a rule with a
carve-out for whichever position happens to be harmless today is a rule a consumer
cannot audit.

### 5.1 Why refusal, and why it is necessary

The overlay's losslessness rests on the sentinel rows being disjoint from genuine
quads. Nothing in IRI syntax delivers that disjointness: `urn:purrdf:rdfc:reifies`
is a perfectly legal IRI that a dataset may assert as an ordinary predicate.

Without this rule, two structurally different datasets canonicalize identically:

* **Dataset A** carries a genuine reifier `(r, ⟪s p o⟫)`, which the overlay lowers
  to `r ⟨urn:purrdf:rdfc:reifies⟩ <<( s p o )>> .`
* **Dataset B** carries no reifier at all, and simply asserts that row as an
  ordinary quad.

Identical canonical bytes ⇒ identical digest ⇒ **identity collision**. For an
append-only, content-addressed authority store this is an identity-forgery
primitive: whoever controls input bytes can mint a claim or view whose identity
collides with a structurally different one. A truth-tier substrate must make that
impossible by construction, not by convention.

Refusal is specified rather than injective escaping. Both close the hole, but they
differ in what a consumer has to audit: refusal makes the property a **total rule
over the input**, checkable by reading one predicate, whereas escaping makes it a
**proof about a function** — that the escape is injective, that it composes with
nesting, that it survives the next overlay row. The simpler obligation is the one
that stays true.

### 5.2 Typed outcome

Refusal surfaces as a typed value, never as a message to be parsed:

| Rust | Meaning |
|---|---|
| `CanonError::ReservedVocabulary(ReservedVocabulary)` | §5 violated; carries `iri` and `position` |
| `CanonError::BudgetExceeded(BudgetExceeded)` | §6 bound reached; carries `blank_count` |

The two are separate variants because they oblige a holder differently. A
budget-exceeded dataset is well-formed and merely uncanonicalizable within bounds;
a reserved-vocabulary dataset is one whose acceptance would have been an identity
collision. A consumer auditing a rejection must be able to tell them apart without
reading English.

`purrdf_core::check_admissible` applies the §5 predicate alone, so a store may
screen at **admission** rather than only at the moment identity is minted.

### 5.3 The reported violation is deterministic

When a dataset violates §5 more than once, the violation NAMED is the least
`(position, iri)` pair under the ordering
`Subject < Predicate < Object < Graph`, ties broken by bytewise IRI comparison.

Refusal itself was always total. This makes the **diagnostic** total too: naming
the first violation encountered would mean statement order, statement order is
interning order, and two backends holding the same dataset need not agree on it.
Two conforming implementations must reject the same datasets *and* name the same
violation.

### 5.4 Precedence

The §5 sweep runs **before** any hashing. A dataset that is both inadmissible and
pathologically symmetric is refused under §5, not §6 — refused for the reason that
makes it dangerous, and without spending the §6 budget to discover it.

## 6. Bounds (normative)

The n-degree search is NP-hard in the worst case: a pathologically symmetric blank
graph can force unbounded permutation backtracking. Complexity poisoning must
**refuse, not hang**.

A fixed call/permutation budget of **1 000 000** bounds the search, exported as
`purrdf_core::RDFC_CALL_LIMIT`. It is public because it is part of this contract,
not an implementation detail: a consumer pinning the profile is pinning the bound
at which canonicalization refuses, and a bound stated only in prose is one the
consumer cannot check against the code it linked.

There is **no knob**. The bound is fixed by the build, not configurable at runtime,
so two parties running the same profile version refuse the same datasets.

Exhaustion yields `CanonError::BudgetExceeded`, carrying the input's blank count.

### 6.1 Fallible and trusted entry points

| Entry point | On refusal |
|---|---|
| `try_canonicalize` / `try_canonicalize_with` | returns `Err(CanonError)` |
| `canonicalize` / `canonicalize_with` | panics |

**Any consumer minting identity from canonical bytes must use the fallible entry
points.** The panicking pair is documented for trusted callers only; its contract
is that the caller has already vouched for the dataset's provenance.

## 7. Versioning

`CANON_PROFILE_VERSION` is incremented by any change that could move a consumer's
minted identity:

* a change to the canonical bytes a given dataset produces,
* a change to the reserved vocabulary of §4,
* a change to the refusal rule of §5,
* a change to the bound of §6.

A change that **cannot** move output — a refactor, a faster search, a clearer
diagnostic message — does not increment it. That restraint is what makes the
number worth pinning: a version that changed on every release would tell a
consumer nothing.

## 8. Normative vector corpus

The corpus lives at `vectors/rdf12-canon/` and is **frozen**: every payload byte is
covered by a SHA-256 manifest checked on every build
(`scripts/check-corpus-frozen.py`), so a silently edited expectation fails rather
than passes.

It carries both halves of the contract — goldens that must canonicalize to exact
bytes, and poison cases that must be refused with an exact error discriminant,
including §5.3's named position. See `vectors/rdf12-canon/README.md` for the case
inventory and the file format.

The corpus has its own content-addressed identity, exported as
`purrdf_core::CANON_CORPUS_DIGEST` and asserted by the harness, so a consumer can
pin **(profile id, profile version, corpus digest)** and verify all three against
the artifact it actually linked.

## 9. What a consumer pins

A complete pin is:

| Field | Source |
|---|---|
| profile id | `purrdf_core::CANON_PROFILE_ID` → `purrdf-rdfc12` |
| profile version | `purrdf_core::CANON_PROFILE_VERSION` → `1` |
| hash algorithm | `CanonHash::Sha256` or `CanonHash::Sha384` (§2) |
| corpus digest | `purrdf_core::CANON_CORPUS_DIGEST` (§8) |
| release | the tagged release the above were read from |

Running the corpus of §8 against a linked build turns that pin into a receipt: it
demonstrates that this build produces the bytes the profile specifies and refuses
the inputs the profile forbids.

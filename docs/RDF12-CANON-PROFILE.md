<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-rdfc12` — RDF 1.2 Canonicalization Profile

**Profile identifier:** `purrdf-rdfc12` &nbsp;·&nbsp; **Profile version:** 2 &nbsp;·&nbsp;
**Editor:** Patrick Audley, Blackcat Informatics® Inc.

Both values are readable from the library rather than only from this document —
`purrdf_core::CANON_PROFILE_ID` and `purrdf_core::CANON_PROFILE_VERSION` — so a
consumer can assert that the build it linked is the profile it pinned.

## Abstract

This profile specifies the canonical byte form PurRDF produces for an RDF 1.2
dataset. It takes W3C RDF Dataset Canonicalization (RDFC-1.0) as its base and
extends it to cover the three RDF 1.2 constructs RDFC-1.0 does not address:
reifiers, annotations, and triple terms. It also specifies a **reserved
vocabulary** and the **refusal** of any input that carries it other than in the two
shapes the profile's own output is written in — which is what makes the canonical
bytes both safe to mint an identity from and re-derivable from that identity
(§3.1).

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

### 3.1 Idempotence: canonicalizing the canonical document returns it unchanged

The canonical output is a document, and a document is an input like any other.
Parse this profile's output and canonicalize it again, and the result is
**byte-identical** to what was parsed:

```
canon(parse(canon(D))) == canon(D)
```

— for every `D` whose canonical form is re-parseable at all, which is every `D`
carrying no named-graph annotation (the one emission shape no quad can carry; see
below, and note that the graph-scoped entry point has no such shape and is
idempotent without qualification).

That holds because the statement layer survives the round trip. Re-parsing the
output brings the overlay rows of §3 back as plain quads bearing the sentinels —
which is precisely what the lowering wrote — and a quad **in exactly the shape the
lowering emits** is FOLDED back into the statement layer rather than refused:

| Re-parsed quad | Read back as |
|---|---|
| `r ⟨urn:purrdf:rdfc:reifies⟩ t .` / `… g .`, `r` an IRI or blank node, `t` a triple term | the reifier binding `(r, t)` (in `g`) |
| `r p o ⟨urn:purrdf:rdfc:annotation⟩ .`, `r` an IRI or blank node, `p` an IRI | the default-graph annotation `(r, p, o)` |

Nothing else moves. A sentinel predicate over a non-triple object, a sentinel in
subject, object or datatype position, the annotation sentinel anywhere but a lone
graph slot, and every other IRI in the reserved namespace are refused exactly as
§5 states.

**Why this is not a hole in §5.** The refusal rule exists because two datasets with
different content must never share canonical bytes. A quad in exactly the shape the
lowering emits does not merely *resemble* a reifier or annotation row — it **is**
that row, written down; the two carry identical content, so giving them one digest
is the lossless overlay working as specified rather than a collision. The fold is
shape-exact in both directions and therefore injective on content: each folded quad
denotes exactly one statement-layer row, each row is spelled by exactly one quad
shape, and a dataset carrying a row both ways carries it once. The slots the fold
does not consume are still swept, so no reserved IRI can be smuggled past the
admissibility check inside a triple term or an annotation predicate, and every
other use of the reserved namespace refuses exactly as before.

#### One shape is not re-parseable, and it is the emitter's doing

The **named-graph annotation** row emits **five** tokens before the terminating
`.` — subject, predicate, object, the annotation sentinel, and the graph term.
That is deliberately not valid N-Quads: a genuine quad never carries two graph
tokens, so the shape cannot collide with one, which is exactly what keeps a
named-graph annotation lossless. No quad can carry that line, so none is folded;
the identity above holds for every dataset whose canonical form contains no
named-graph annotation.

The **graph-scoped** entry point (`purrdf_core::canonicalize_graph_view` and its
fallible twin) erases the graph slot, so its output is always quad-shaped and
**fully idempotent** — a consumer that needs the round trip unconditionally should
mint from a graph-scoped canonicalization.

A consumer that only compares or digests is unaffected either way: the output
remains **a canonical byte string**, and comparing or hashing it never requires
parsing it. Idempotence is a property such a consumer may now rely on — the bytes
an identity was minted from re-derive that identity — not an obligation it has to
discharge. On the RDF 1.1 subset the output *is* valid canonical N-Quads, which is
why the W3C suite can gate it.

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
`purrdf_core::RESERVED_NAMESPACE`. This version lowers into exactly two names
within it — `urn:purrdf:rdfc:reifies` and `urn:purrdf:rdfc:annotation` — but the
reservation covers **the whole namespace**, not those two names.

The reservation governs every use of the namespace. It disposes of a use in one of
exactly two ways, and there is no third:

| Use of the reserved namespace | Disposition |
|---|---|
| `r ⟨urn:purrdf:rdfc:reifies⟩ t .` / `… g .`, `r` an IRI or blank node, `t` a triple term | **folded** — admitted as the reifier row it denotes (§3.1) |
| `r p o ⟨urn:purrdf:rdfc:annotation⟩ .`, `r` an IRI or blank node, `p` an IRI | **folded** — admitted as the annotation row it denotes (§3.1) |
| every other occurrence, in every position, at every depth | **refused** (§5) |

The two folded rows are the two shapes §3 lowers *into*, written out exactly. They
are not an exception to the reservation; they are the reservation read in the
direction that makes the profile's own output admissible (§3.1, §5.1).

The reservation is stated over the namespace on purpose. An enumeration would have
to be re-audited every time the overlay grows a row, and a rule whose soundness
depends on an audit nobody schedules is not a rule. A namespace reservation is a
single sentence that can be checked against the entire implementation, and it costs
nothing to hold: PurRDF publishes no vocabulary, mints no ontology terms, and
nothing legitimate lives under this prefix.

## 5. Refusal rule (normative)

> A dataset in which **any term, in any position, is an IRI beginning with
> `urn:purrdf:rdfc:`** is INADMISSIBLE and MUST be refused — **except** where that
> IRI occurs in one of the two quad shapes of §3.1, which are read back as the
> statement-layer rows they denote before the sweep runs. An inadmissible dataset
> MUST NOT be canonicalized, and no digest may be minted from it.

The exception is exact and it is exhaustive: it is the two shapes §3 lowers into,
listed in §3.1's table, and no other. A sentinel predicate over a non-triple
object, a sentinel in subject, object or datatype position, the annotation sentinel
anywhere but a lone graph slot, and every other name in the namespace are refused.
Folding is applied to the quad as a whole, never to a term in isolation, and the
slots the fold does not consume are swept like any others — a reserved IRI nested
inside a folded row's triple term, or standing as a folded annotation's predicate,
is still a refusal.

"Any position" means all of: subject, predicate, object and graph; **nested inside
a triple term** at any depth; and **a literal's datatype IRI**. The datatype slot
is swept even though the overlay never lowers a sentinel into one — a rule with a
carve-out for whichever position happens to be harmless today is a rule a consumer
cannot audit.

### 5.1 Why refusal, and why it is necessary

The overlay's losslessness rests on the sentinel rows being disjoint from genuine
quads. Nothing in IRI syntax delivers that disjointness: `urn:purrdf:rdfc:reifies`
is a perfectly legal IRI that a dataset may assert as an ordinary predicate.

The property being defended is exact: **two datasets with different content must
never share canonical bytes.** Identical canonical bytes ⇒ identical digest, and
for an append-only, content-addressed authority store a digest shared by two
different contents is an identity-forgery primitive — whoever controls input bytes
can mint a claim or view whose identity collides with a structurally different one.
A truth-tier substrate must make that impossible by construction, not by
convention.

That property decides both dispositions of §4's table, and it is why they are not
in tension:

* **Same content ⇒ one digest.** A quad in exactly the shape the lowering emits
  does not resemble a reifier or annotation row; it **is** that row, written down.
  A dataset carrying it and a dataset carrying the genuine structure differ in
  spelling, not in content, so canonicalizing them to one value is the lossless
  overlay working as specified. Because the fold is shape-exact in both directions
  it is injective on content — it can merge a row with its own spelling and with
  nothing else — and that is what §3.1's idempotence rests on.
* **Different content ⇒ refusal.** Every other occurrence of the namespace carries
  content the overlay has no row for. Admitting it would place uninterpreted
  reserved names into the canonical byte space, where a future overlay row could
  collide with data already admitted and digested under an earlier version —
  the collision arriving years after the bytes did. §4 therefore reserves the
  whole namespace and §5 refuses every unfolded use of it, so a name the profile
  has not minted can never be sitting in a consumer's store waiting for it.

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

The §3.1 fold runs **before** the §5 sweep, and the sweep runs **before** any
hashing. Folding first is what makes the two total rules compose: a folded quad is
gone from the quad layer by the time the sweep looks, and everything the fold did
not consume — including every remaining slot of a folded row — is swept. A dataset
that is both inadmissible and pathologically symmetric is refused under §5, not
§6 — refused for the reason that makes it dangerous, and without spending the §6
budget to discover it.

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

### 7.1 History

| Version | Change |
|---|---|
| 1 | initial profile |
| 2 | §5 narrowed: the two quad shapes §3 lowers into are **folded** back into the statement layer rather than refused, making canonicalization idempotent over its own output (§3.1) |

Version 2 moved **no** bytes: every input version 1 admitted canonicalizes to
exactly the bytes version 1 produced. What changed is the refusal rule, and a
refusal is part of the contract a consumer pinned — two inputs version 1 refused
now canonicalize — so §7's third bullet obliges the increment on its own.

## 8. Normative vector corpus

The corpus lives at `vectors/rdf12-canon/` and is **frozen**: every payload byte is
covered by a SHA-256 manifest checked on every build
(`scripts/check-corpus-frozen.py`), so a silently edited expectation fails rather
than passes.

It carries both halves of the contract — goldens that must canonicalize to exact
bytes, and poison cases that must be refused with an exact error discriminant,
including §5.3's named position. Two of its cases sit on the §3.1 boundary from the
admitted side: each writes out one of the two folded shapes as an ordinary quad and
pins the statement-layer row it canonicalizes to, so the fold is measured rather
than asserted, and it cannot silently widen or close. See
`vectors/rdf12-canon/README.md` for the case inventory and the file format.

The corpus has its own content-addressed identity, exported as
`purrdf_core::CANON_CORPUS_DIGEST` and asserted by the harness, so a consumer can
pin **(profile id, profile version, corpus digest)** and verify all three against
the artifact it actually linked.

## 9. What a consumer pins

A complete pin is:

| Field | Source |
|---|---|
| profile id | `purrdf_core::CANON_PROFILE_ID` → `purrdf-rdfc12` |
| profile version | `purrdf_core::CANON_PROFILE_VERSION` → `2` |
| hash algorithm | `CanonHash::Sha256` or `CanonHash::Sha384` (§2) |
| corpus digest | `purrdf_core::CANON_CORPUS_DIGEST` (§8) |
| release | the tagged release the above were read from |

Running the corpus of §8 against a linked build turns that pin into a receipt: it
demonstrates that this build produces the bytes the profile specifies and refuses
the inputs the profile forbids.

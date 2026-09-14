## Prior Art

**Recurrence: first recorded instance of THIS CLASS** (XML literal text-node escaping defects that break round-trips or well-formedness). The `stagectl brief` found no `GhpRsq-Defect-Class` trailers, linked issues, or forge items matching this shape.

**However**, this is NOT the first instance of the broader class: *"XML-shaped writers emit data they cannot read back"*. That class has at least one prior fix in this repository.

### Previous attempt (same class, different surface)

| # | when | commit / PR | what it changed | why it did not cover literals |
|---|------|-------------|-----------------|------------------------------|
| 1 | Fri Sep 11 2026 | `84abf779` — `fix(rdf): the RDF/XML writer emitted predicates it could not read back` | Fixed predicate QName generation: corrected `NameStartChar`/`NameChar` classes; deleted fallacious split-at-last-`#/` fallback; fixed namespace pre-pass for nested quoted triples; added round-trip tests (write → read) | It fixed **predicate element names**, not **literal character data**. The `escape_xml` functions in `rdfxml.rs` and `trix.rs` were untouched by this commit. The commit body says: "emitting XML we cannot re-read is the whole defect" — establishing the verification pattern now asked for in #294, but only exercised on predicates. |

### The shared-escape infrastructure that missed these two

Commit `7a4d9a7b` (same day, earlier) created `crates/rdf-core/src/iri_escape.rs` and collapsed **five** IRIREF escape transcriptions onto one law. The issue body explicitly cites this work: "It was found while auditing the scanner-terminal work, which collapsed five other escape transcriptions onto one law. These two are a sixth and seventh, and the defect is the same shape as that work's headline."

`iri_escape.rs` currently governs **IRIREF** egress only (`is_iriref_escape_required`). Its module docs state: "Only the PREDICATE is shared. The writers emit through three different machines ... and collapsing those would cost the fast paths while sharing no additional law." That reasoning was correct for IRIREF escaping, but it left the two XML **literal** escapers (`rdfxml.rs:1190` and `trix.rs:550`) as byte-identical duplicates with no shared home.

### What changed each time

- `84abf779`: fixed predicate QNames in `rdfxml.rs`, added `rdfxml_predicate_names.rs` tests, updated `scripts/check-terminal-predicates.py`.
- `7a4d9a7b`: extracted shared IRIREF escape predicate to `iri_escape.rs`, did NOT create a shared XML literal escaper.

### Why previous fixes did not cover this

The fixes were **scoped to predicates** (QName grammar) and **IRIREF bodies** (`<...>`), respectively. XML literal character data is a different production with different rules:
- `IRIREF` escapes `[#x00-#x20]` and delimiters, plus C1/DEL for XML transport.
- XML text nodes must escape `& < > "` (the current code does this), **and** `#xD` must be written as `&#xD;` to survive XML 1.0 §2.11 normalization (the current code does NOT do this), **and** C0 controls excluded by XML `Char` must be refused at the serializer (the current code emits them raw, producing non-well-formed XML).

The two `escape_xml` functions (`rdfxml.rs` and `trix.rs`) are byte-identical and both miss `#xD` and the C0 refusal.

### Design constraints already decided

- **Byte determinism** (`AGENTS.md` §2): "Serializers and the GTS writer are byte-deterministic. If your change alters emitted bytes, you must update the affected goldens and say why in the PR." Issue #294 acknowledges this: "Either may legitimately move a frozen vector or a conformance row."
- **Terminal ring-fence** (`AGENTS.md` §2): "a scanner's character classes are exact, in both directions." The issue asks for exact obedience to XML's `Char` production, matching this doctrine.
- **Round-trip verification as the only valid check**: `84abf779` established "write with the production writer and read back with the production reader, assert the graph is identical" as the correct verification pattern for this class of defect. Issue #294 repeats this requirement verbatim.
- **Shared law for duplicated predicates**: `iri_escape.rs` is the established home for shared egress predicates. Issue #294 proposes extending this pattern: "One `escape_xml`, not two. The workspace already has a shared home for egress escaping in `crates/rdf-core/src/iri_escape.rs`."

### Blast radius (graphify)

The two affected writers are:
- `purrdf-rdf` native codec: `RDF/XML` (`rdfxml.rs`)
- `purrdf-rdf` native codec: `TriX` (`trix.rs`)

Both are serializer-only surfaces. The RDF/XML reader is the consumer most directly affected (it already fails on the C0 output). Downstream consumers (`gmeow-ontology`, bindings) ingest through the read paths; the fix makes the write path stricter, so consumers that round-trip through XML will see fewer silent corruptions.

### Retained lessons (hindsight)

No directly relevant retained lesson in `purrdf` or `rust-fleet-doctrine` for XML literal escaping specifically. The `purrdf` bank holds observations about PR #299 (xsd-regex, ShEx PatternCache) and issue #296 (semantic compiler) but nothing about XML literal round-trips.

### Scope recommendation

**POINT FIX with PATTERN EXTENSION** — not root-cause.

This is a straightforward consolidation: two byte-identical `escape_xml` functions should become one shared predicate in `crates/rdf_core/src/iri_escape.rs` (or an adjacent module), with:
1. `#xD` escaped as `&#xD;`
2. C0 controls outside XML `Char` refused before writing
3. Round-trip verification (write → read) as the test

The fix belongs at the same layer as `84abf779` and `7a4d9a7b`: the serializer/escape layer in `purrdf-rdf`/`purrdf-core`. It does not indicate a deeper architectural flaw because:
- The IRIREF escape law was correctly extracted.
- The predicate QName law was correctly fixed.
- This is the remaining surface of the same audit sweep.

The only structural concern: if a **third** XML-shaped writer is added later without routing through the shared escaper, the duplication recurs. The fix should make the shared XML literal escaper the obvious and only path.

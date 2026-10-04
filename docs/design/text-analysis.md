<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Text analysis and auxiliary retrieval

`Analyzer` is an immutable resolved law shared by indexing and every query. Its
three projections have distinct purposes: lexical words determine BM25F and phrase
positions; surface words preserve pre-stem spelling; substring spans preserve
punctuation. The independent `HanCharacterIndex` supplies a separate ranked
character producer. No match equates RDF subjects or changes original literals.

## Configuration and artifacts

`AnalyzerProfile::standard()` selects plain text, Latin/Greek/Cyrillic accent
folding, no stemming, five full baseline dictionaries, a nominal 128-scalar bound,
four-character phonetic codes, distance two, and substring limits of 65,536 logical
posting operations, 4,096 candidate spans and 2 MiB of verification input.

```rust,ignore
let analyzer = purrdf_text::Analyzer::resolve(
    purrdf_text::AnalyzerProfile::standard(),
    &artifact_bytes, // Five borrowed CBOR byte strings supplied by the host.
)?;
let config = purrdf_text::TextIndexConfig::new(predicates, graph, analyzer)?;
```

Missing, duplicate, unexpected or corrupt baseline artifacts fail. The core opens
no files, fetches no data and embeds no dictionaries. `Analyzer::empty_lexicon()`
explicitly selects grapheme fallback for unspaced scripts. Caller dictionaries use
`Segmentation::Dictionary(Arc<Dictionary>)`. Canonical duplicate entries with
conflicting costs fail; pinned baseline normalization collisions use minimum cost.
Effective profile-normalized keys and costs are bound to the resolved identity.

The separate `purrdf-text-lexicons-VERSION.tar.gz` release archive includes the five
content-addressed artifacts, raw source files, normalization collision reports,
manifest, provenance, checksums and original notices. ICU data retains its actual
Unicode, Google, libtabe, IPADIC/ICOT, Lao and Burmese conditions; it is not
relicensed as library code. Runtime algorithms are independently implemented from
prose specifications and principles, with no new external crates.

`Dictionary::from_artifact(expected_blake3, bytes)` validates physical identity,
canonical CBOR, versions, counts and structure. Semantic entry/cost identity is
independent of radix versus minimal-acyclic execution representation. The lattice
minimizes unknown grapheme units, then total integer cost, emitted count, and
longest earliest edge. Unknown edges respect Unicode extended graphemes. It admits
mixed technical entries, supplementary Han and legitimate dictionary punctuation.

## Ordered analysis

Decode explicit HTML references once, protect emoji graphemes, and apply the
named cleanup law. Recompute grapheme boundaries after deletion, then normalize
non-emoji runs with `NFD → full case fold → NFKD → full case fold → NFKD`, remove
eligible accent marks, and finally compose NFC. This permits composition across
removed controls while preserving actual source contributors. Segmentation retains orthographic controls;
lexical terms remove those controls and optionally stem. Every projection receives
its final whole-grapheme prefix bound last.

Accent removal uses General_Category Mn with base/script eligibility. Leading
marks and marks of ineligible scripts survive. The default preserves Thai, Lao,
Khmer, Myanmar and Brahmic signs. Callers independently select any subset of
Latin, Greek, Cyrillic, Arabic and Hebrew; Arabic and Hebrew are opt-in members.
`AccentScripts::from_scripts([unicode::AccentScript::Arabic])` selects Arabic
alone through `AccentFold::Selected`, and `AccentScripts::default()` selects no
scripts. Order and repeated members have no effect. Explicit sets equivalent to
the three presets share their profile and analyzer identities; the preset
fingerprints and default behavior remain unchanged.

Python analyzer dictionaries accept `"accent": ["arabic"]`,
`"accent": ["hebrew"]`, or subsets such as `"accent": ["latin", "cyrillic"]`.
`"accent": []` preserves every accent. The existing `"preserve"`,
`"latin-greek-cyrillic"` and `"latin-greek-cyrillic-arabic-hebrew"` presets remain
accepted. Unknown script names and numeric flags are refused. The same selection
normalizes caller dictionary keys, indexed text and probes.

English Snowball 3.1.0 runs only on words with at least one Latin letter and no
letters outside Latin. Surviving non-ASCII Latin letters are non-vowels. Emoji
bypasses linguistic conflation.

ZWSP separates. ZWNJ, soft hyphen and non-emoji selectors U+180B–180D/U+180F,
U+FE00–FE0F and U+E0100–E01EF are word-internal and survive surface/substring
projections. Lexical terms drop them after segmentation. The named removed set is
U+061C, U+200E–200F, U+202A–202E, U+2066–2069, U+2060, U+FEFF, unprotected tag
controls and non-whitespace C0/C1 controls. Stray ZWJ disappears; joined emoji and
joining-script/Indic orthographic contexts preserve it until lexical projection.
There is no blanket Default_Ignorable removal.

Lexical `.` and `:` split when both significant neighboring bases are letters;
marks and internal controls are transparent to this test. Dictionary edges cannot
bridge the split. Decimals and apostrophes remain whole. Surface words and
substring spans retain dotted paths, prefixed names and timestamps.

Unicode 17 recognition covers every emoji-test spelling/status and standardized
variation pair. Recognition does not exclude unrecognized joined pictographic or
regional-indicator graphemes. Presentation, modifiers, gender and joined forms
remain distinct. Each lexical emoji atom occupies one position.

The bound is nominal: retain the longest complete grapheme prefix within it. If
the first grapheme exceeds the bound, emit it intact as the sole prefix; a later
oversized grapheme stops truncation. The minimum valid bound is generated from the
longest recognized emoji; the maximum is 1024. Independent query work budgets
remain enforceable even for an oversized first cluster.

## References and source evidence

`InputMode::{Plain,HtmlText,HtmlAttribute}` selects interpretation. The shared
`purrdf_lex::html` resolver implements the complete WHATWG named longest-match
and numeric recovery rules with typed diagnostics and original scalar sources.
The analyzer rejects any reference-submachine parse error. Attribute legacy
exceptions follow their context. This does not parse HTML documents: tags,
quotes and line endings remain text. `&amp;lt;` becomes `&lt;`, without recursion;
entity-constructed emoji is protected before normalization. Dictionary entries
are already Unicode and are never reference-decoded.

`Analysis` owns one `AlignedText` plus lexical, surface and span projections.
`Projection::range` denotes normalized origin bytes; `sources` denotes actual
original UTF-8 contributors; `highlight` encloses them and may contain deleted
bytes between contributors. Decomposition duplicates annotations, canonical
ordering moves them, and composition unions contributors. Expansion and multi-
scalar references can share a source range. A stem has whole-word evidence marked
`coarse`, without invented character correspondence.

Auxiliary matches add document/literal identity, projection, analyzer identity and
index generation. Different original RDF terms stay distinct even if their
normalized spelling is equal. Auxiliary-only documents remain inspectable while
zero lexical terms contribute zero lexical BM25 population or length.

## Complete auxiliary operations

`SurfaceIndex` indexes packed scalar grams of widths one, two and three in flat
positional posting arrays. `substring_report` aligns relative positions, validates
protected emoji endpoints, and only then counts distinct candidates. The rarest
anchor wins, with ties by gram key then query offset; remaining lookups follow
key/offset order. One logical operation is an anchor visit or a required-position
membership test. Optimized search paths obey the same counter law.

With N distinct spans, indexed lookup requires candidates strictly fewer than N,
including N=1. Empty indexes and absent grams produce complete empty results.
`TextError::Substring` carries a typed reason, counters, limits and generation.
Charges precede operations, admissions and verification reads. No partial matches
or automatic scan escape the refusal. `substring_exhaustive` is an explicit
complete scan under the configured candidate/verification limits.

Phonetic lookup canonicalizes Latin words through accent/control removal, ASCII
uppercase and `ß→ss, æ→ae, œ→oe, þ→th, ø→o, ł→l, đ/ð→d, ħ→h, ı→i, ŋ→n`.
Internal ASCII/U+2019 apostrophes disappear. Remaining unsupported scalars receive
a typed `PhoneticRefusal`; expanded input is at most 2,048 scalars. Nonempty
primary/alternate code buckets are unioned and refined by exact bounded distance
on canonical widened spellings. Unsupported stored words remain in other lanes.
Distance thresholds are 0–64 and code lengths 1–64. Prepared endpoint-aware banded
DP is the production default; prepared Myers is a separately validated candidate.

`HanCharacterIndex` stores Han scalar unigrams and adjacent bigrams with its own
exact BM25F statistics. Attached marks/selectors remain in evidence but not keys;
punctuation, another script, whitespace, emoji and literal boundaries break
adjacency. Singleton query runs select unigrams; longer runs select bigrams.
`relation()` uses the existing ranked producer seam. Consumers supply producer
IRIs, declarations and fusion weights/law through `purrdf-retrieval`.

## Qualification

Official Unicode normalization, grapheme and emoji data, the complete pinned
English corpus, independent phonetic black-box vectors and full-matrix distance
oracles validate the kernels. Shared runner bodies execute on native and WASM.
Full dictionary source rows and artifacts are checked in both representations.
Original held-out domain judgments and an external Chinese judged pool compare
lexical, character and explicitly weighted fusion results. Performance and quality
reports state their workloads and limits; they do not claim universal optimality.
The [measurement dossier](text-analysis-measurements.md) records the kernel
selections, retrieval quality, allocation costs and production artifact sizes.

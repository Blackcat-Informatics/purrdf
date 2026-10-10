# Native TEXT owner integration

Issue 508 support unit, 2026-10-09. The sole writer reports applying
`native-diagnostic-owner-draft.patch` and `native-text-owner-draft.patch`.
`native-text-coherence-draft.patch` is a supplemental standard diff against that
shipping postimage. This helper wrote only Stage artifacts and ran no compiler,
test, build, Git mutation or forge action. This is not qualification or completed
issue acceptance.

## Production source units

| Artifact | Actual scope |
|---|---|
| `native-diagnostic-owner-draft.patch` | Evaluator error/export seam, shallow immutable diagnostic owners, Internal and FunctionOperational kinds, common exact count/render |
| `native-text-owner-draft.patch` | Sixteen concrete homes: search/occurrence production dispatch and cursors; actual ranking/analyzer owners; shared HTML/Unicode/stem/Han kernels; admitted native literals and decimal formatting |
| `native-text-coherence-draft.patch` | Six shipping-postimage homes: allocation-free term-kind refusals; stable local partition ordinals plus admitted external comparison; orphan docs/private unused wrapper; exhaustive TextError fixture; one decimal renderer |

The original source blocks are preserved in `native-text-owner-source-units.md`.
Do not rerun its historical whole-patch assembler against the integrated
postimage. Reconcile the supplemental patch with concurrent writer edits.

Both native relations override `open_admitted`; resident `open` uses the same
`open_owned` with a resident capability. Both cursor protocols call one
`next_owned`; bounded pulls return `AdmittedPfRow`, and raw resident pulls use the
shipping actual-row-and-term-grant checks of `try_into_resident`. Concrete cursor
boxes are charged by their actual layout before fallible `try_boxed_one`.

Fresh vectors/strings use shared `reserve_vec`/`reserve_string` or `AdmittedVec`.
The old buffer and grant survive replacement allocation. Consuming iteration
retains metadata ownership. Original terms use `clone_term`; bound equality and
subject ordering use admitted core `terms_equal`/`terms_cmp`. Generated cells use
borrowed primitive formatting through `WorkspaceCapability::literal`, followed
by `AdmittedPfRow::from_terms`. No raw extraction sheds a bounded owner.

## Concrete capacity coverage

| Owner/home | Capacity chosen before growth |
|---|---|
| HTML | `resolution_layout` counts actual replacement UTF-8 bytes, scalar source ranges and diagnostics without allocating; caller-preallocated fill calls the same reference reader |
| Normalizer | Decoded scalar count for source buffers; checked local pending append; existing decomposition/casefold kernels count actual pinned expansion; actual output UTF-8 count before emission |
| Canonical order | Pre-admitted ordinal array at actual decomposition count; in-place permutation preserves equal-class order without stable-sort temporary |
| Dictionary lattice | Whole normalized input scalar count bounds every chunk's graphemes; checked endpoints `count+1`, exact filtered UTF-8 bytes, typed fallback/barrier/path buffers before the unchanged dictionary body |
| Lexical/stem | Normalized scalar count for ranges; source word byte length for changed-word output; existing stem's selected Letter capacity is input bytes, covering its scalar/suffix state |
| Han | Owned normalized output, locally admitted run storage, actual one/two scalar UTF-8 bytes for unigram/bigram strings |
| Posting occurrences | Checked sum of actual document frequencies; borrowed predicate-frequency slices |
| Candidates | Exact distinct document runs counted after in-place occurrence sort; every working candidate scored |
| Prepared statistics | Actual field/query counts for metadata; each copied query term retains its string grant |
| Rank heap | Existing BinaryHeap backing: full candidate count, or `min(k,C)+1` where push-before-pop exceeds a kept prefix, otherwise exact kept/full count |
| Bound subject | Actual found document count for holdings; distinct query count for located metadata; actual stable partition ordinals |
| Occurrence cursor | Own term/partition ordinals/bounds/control; borrow immutable postings/positions on each pull, without copying caller index arrays |
| Native diagnostics | Exact stable output length plus actual Shared payload layout before fallible String/control allocation; shallow clones retain the same grant |

The index, dictionaries, vocabulary/profile and existing generation Arc are
prebuilt caller ownership. They are not fresh query allocations. LIMIT never
removes posting/candidate scoring from admission. Bound document/score/matched
postfilters with observable rank require full ranking first. Unobserved rank uses
the existing membership/point-scorer path through the same summation. Only an
actually emitted row decreases the cursor licence.

## Shipping-postimage remedies

`TextError::Capacity` preserves static allocator, layout and WorkspaceStopped
classes through its exact EvalError mapping. Reachable fixed-point failures use
the shared optional-owner arithmetic body; no legacy String is formatted before
pricing. Data/domain messages render directly under their owner. Resident Fixed
and TextError APIs retain their normal resident variants.

The supplemental patch replaces recursive term Debug in invalid needle,
language and nonliteral-rank errors with `TermValue::kind_description` at the
existing core term home. Field/type/domain reasons remain; invalid literal
datatype and malformed/zero/negative rank retain borrowed datatype/lexical
detail. This closes TEXT's formatting walk without pretending the generic
renderer prices arbitrary Display/Debug internals. Other native callers that
still format nested terms must use an admitted formatter or the same kind route.
Core Debug currently owns TWO spilling lists: ValueTok inline 32 and open state
inline 16.

Index-owned key borrows resolve to exact array ordinals in constant time.
Holdings/occurrence restrictions compare ordinals. Genuinely external graph keys
use fallible terms_cmp and borrowed language ordering; any failed comparator's
search result is discarded before use. Subsequent native lookup receives the
actual resolved index-owned key. Addresses only verify identity of a real slot;
they never enter emitted bytes, ranking or content identity.

Source search found only the unused crate-private score_located definition, with
production callers using score_located_owned. The supplemental patch removes
that wrapper, repairs stale doc links/orphan docs, and extends the exhaustive
text/error.rs string-fixture match. The writer already removed the duplicate
capability is_bounded found during inspection. Its new shared AdmittedHeap can
replace the draft's concrete OwnedOrder carrier while retaining the same
push-before-pop capacity and ordering law.

The writer must retain NativeDiagnostic kinds through actual FILTER/BIND/native
property-function/SERVICE SILENT and final governed error boundaries. This helper
did not requalify those evaluator migrations. Static allocator/stop errors must
bypass diagnostic rendering after refusal.

## Required real-entry acceptance

Run the grouped affected compile after the carriers/native units settle, then
the planned qualification lane. Direct rank helpers alone do not qualify the
registered relations from bounded query text with configured options.

- Both relations and lexical/Han indexes; plain/HTML text/HTML attribute,
  default/named/empty/multiple graph-language partitions, untagged versus empty
  language, dictionary and English stem profiles.
- Every supported binding mode and emitted cell; observed-rank bound subject,
  unobserved-rank membership, score/matched postfilters, equal-score tie order,
  repeated driving rows and ceilings.
- Full working candidates with small LIMIT; admission refusal at analyzer,
  postings/candidates, heap, bound copy, cursor, row, lexical and diagnostic
  allocations; earliest typed provider failure wins and no partial ranking leaks.
- Deep nested subjects for admitted comparison/copy and deep invalid arguments
  for kind-only refusal; huge well-formed ranks remain empty, while malformed,
  zero and negative ranks remain errors.
- Cursor dropped before retained rows; cloned retained rows/terms/diagnostics;
  truncation and every failure release live owners correctly.
- HTML count/fill parity, long equal-CCC runs, compatibility/casefold expansion,
  emoji/controls, stem suffixes, Han unigram/bigram and occurrence parity.

None of this acceptance was run by the helper. Shipping integration and the
single resource-capped combined qualification remain the writer/root's work.

# Surface storage integration

## Implemented; focused qualification only

The production surface path below is implemented, including generic corpus
profiles, stable source/document references, persistent shallow term headers,
membership counts, source-local occurrence chunks, keyed grams/codes and borrowed
public views. The final old/new ID maps and rebase/full-merge helpers are removed.
Standalone source generation uses an exact source multiset and new v2 generation
domain; native generation remains bound to the enclosing index identity.

Ten surface unit cases passed in persistent-surface-unit-runtime.log (session
63201, exit 0). After the direct shallow term-header change, nine index delta
controls passed in persistent-surface-index-runtime.log (session 45610, exit 0).
The later persistent-surface-growth.log reports a passing fixed insertion and
fresh-table parity with surface enabled: 109 / 164 / 186 new table nodes and
581 copied leaf values at 128 / 4,096 / 16,384 retained documents. The process
handle for that run was not recovered; the completed test log is the evidence,
not a separately collected command exit. Clone builds no nodes or copied values.
These counters do not prove allocation-byte, live/peak or producer-preparation
bounds. Later all-text-target compilation passes (3306, exit 0), and 164 library
and public cases pass (82783, exit 0), including the same growth control with a
collected shell exit. Original/current evidence is distinguished in validation.md.
Complete portable and total-resource qualification remain pending.
The following design text records the implemented
representation; its descriptions of the former linear path are historical.

The lexical, document, partition, field, side-index and Han paths now use the
same persistent AVL table. Do not rebuild or re-review those components instead
of completing this remaining production path. Current generic table counts are
bounded to u32; surface occurrence populations need usize bounds, with the
existing document/distinct-term bounds enforced at their actual owners.

## Current linear path to remove

`TextIndex::merge_changed` retains the old document root solely to produce
`old_ids` for `SurfaceIndex::rebase_documents`. That method filters every source,
renumbers every source occurrence, compacts word/span IDs and remaps every gram
and phonetic posting. `merge_disjoint` then repeats full merges. `SurfaceIndex`
also derives Clone over complete vectors. These are the remaining index-wide
passes; do not describe the complete index update as proportional to its change.

## One storage path for standalone and enclosing indexes

- Make `SurfaceIndex` generic over the existing corpus profile (default remains
  graph-language). Internally, a surface document key is either the standalone
  caller's stable u32 or a shared immutable `Document<P>`. Construction fixes
  that mode and must refuse incompatible merges. No graph coordinate is added
  to a graph-free document. The enclosing index supplies its current document
  order root once, shared in O(1), for requested canonical ID projection.
- Sources retain immutable document references, literal identities and the
  original aligned normalized buffer. Occurrences retain shared source references,
  never positions in a corpus-wide source array. Source order is document key
  then literal identity; occurrence ordering additionally uses original range.
- Word/span records are persistent, ordered by spelling. Their occurrence and
  distinct-document membership tables are persistent too. A membership counter
  prevents removing a document that still has another occurrence. Retain word
  code/canonical-spelling and span-shape metadata with their actual record.
- Gram and phonetic postings use shared immutable spellings as term addresses,
  not sorted dictionary ordinals. Query lookup resolves the current term record.
  Do not point postings at a prior membership record when that record changes.
  Existing terms retain their gram/code postings; only terms appearing or
  disappearing create/remove those postings. Preserve original positional gram
  matching and endpoint eligibility; there is no second substring engine.
- Reuse the common persistent table for keyed posting ranges. Replace the flat
  keys/offsets/value arrays with ordered `(key, posting)` entries and a borrowed
  range view. Keep deterministic sorted joins; remove obsolete remap/merge paths.
- Public term inspection is a borrowed view holding the current surface owner
  and immutable term record. Its document iterator projects requested IDs.
  Substring/phonetic matches expose that view and current canonical evidence.
  Do not cache a whole projected dictionary during publication or materialize
  retained source data on the first query. Adapt actual callers and preserve
  answer/evidence/work/refusal comparisons, including graph-free behavior.
- Standalone updates also need incremental generation accounting. Use the
  complete deterministic input identities plus analyzer law and exact source
  population, under a new surface domain; do not retain a whole-source hash pass.
  Caller-selected source identities and normalized-analysis conflict refusals
  remain enforced. The enclosing index continues binding its own generation.

Complete the surface constructor, replacement/removal, grammar and phonetic
posting updates and real query callers together. Use focused compilation and
existing native fresh-build/evidence/refusal controls during implementation.
Do not run another broad matrix before this and the remaining source-directory
transition path are implemented. Actual native allocation/peak/growth evidence,
scalar/SIMD WASM, mandatory hooks, independent review and delivery remain open.

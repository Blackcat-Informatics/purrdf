# Issue472 implementation evidence

Status: independent source review PASS in implementation-review.md; complete
local qualification PASS. Normal hook commit45d1fb09f is pushed and PR529 is
published. Hosted checks, integration, merge and closure remain pending.

The one production renderer uses `purrdf_lex::walk::WorkList<Emit,16>` for
property/object/collection emission and the same native work-list home for blank
reachability. Inline/shared/cyclic/quoted classification remains active. Existing
statement side rows enter the original indexed renderer. Content-key traversal
retains40 and frozen triple16; cache reads cannot insert arbitrarily long cached
suffixes past40. Singleton objects need no ordering key. Equal bounded keys use
authored term identity rather than insertion-local IDs. Collection recognition
uses the original strict RDF collection walker over indexed edges, not a
whole-dataset scan at every cell. No dependency, semantic Cargo feature,
namespace, stack enlargement or alternate renderer was introduced.

Indentation retains four spaces per actual level through40, then saturates160.
Continuation-object indentation follows this same policy. The Stage-only Cargo
probe loads the original renderer captured from the assigned base and the actual
shared input fixture. Captures at0/1/2/31/32/33/39/40/41 preserve every term and
punctuation line and every original line whose indentation is at most160.
Differences at blank-chain depths40 and41 are4 and16 spaces respectively because
the terminal predicate is on actual level41 or42. `baseline-cargo.log` terminal0;
earlier unsupported direct-rustc probes remain as historical failed evidence.

Public acceptance is one shared `harness = false` test corpus, also registered in
the scalar portable runtime lane. Real100k/1M frozen datasets exercise packed
output and distinct native Turtle/default and TriG/named routes. All routes retain
reifier/annotation statement rows, reparsing native output and reserializing it
byte identically. Additional cases cover exact neighbors, insertion permutation,
cycles/shared/quoted blanks, tied guarded branches, a100k collection and saturated
continuation indentation. Native final exact selection passes7/7 in16.65s.

Frozen output receipts, independently asserted on both target families:

| Route | Bytes | BLAKE3 |
|---|---:|---|
| packed/100000 | 33094118 | 938071ed811f977dad0bb575dae44701c657a74fc12dea4af80518613201208b |
| turtle/100000 | 4878216 | 5ab357ea2c8db6c837529ec198b8b37359d7d4ad8f1ecfba721877b05dc6f630 |
| trig/100000 | 5078161 | f48e53d4b763e58c34c191f7d717e12abc9c5869468a8897c8e3d2f28f38085e |
| packed/1000000 | 330994118 | 8dcbafdf63c599dcc6bc781de652d70c6f7dae9cb35127dafb28a1b1a203f72a |
| turtle/1000000 | 50778216 | fb165514a50a577964997db5bc344863d25928f8fe3ec232efa55ea3d7eb7384 |
| trig/1000000 | 52778161 | 8ac01ee650c82b0054aa54fc03e75250346bf3728a7180e0198964bc5e373094 |

Portable execution passes7/7 exact cases in25.81s, all six frozen receipts
identical to native, through the existing optimized wasm runner. First-party
benchmark and single full gate now pass as detailed below. The benchmark times actual
packed rendering at8/100k/1M with fixture
construction outside timing, through the existing testkit harness.

Independent review corrected an Eq/Ord consistency defect in ObjKey: equality
now follows ordering rather than local ID. A direct native control passes for
singleton and competing blank keys, including set cardinality. Affected strict
compilation passes; strengthened public native corpus7/7 passes11.04s and
optimized portable corpus7/7 passes18.69s with all six frozen outputs unchanged.
The existing exact-neighbor control also proves actual named-TriG permutation.
The first-party benchmark passes3/3: median depth8 4.645µs,100k236.3ms and
1M2.928s. Its timed path is unchanged by the Eq correction, as the independent
source review confirms; original JSON estimates and pre-correction artifact scope
are retained. The single full gate finished with exit code0 in the original95104
execution session. All static/hygiene/generated checks, native workspace
tests/doctests, the standalone preserve-order consumer and optimized wasm release
build passed. The public Turtle corpus passed7/7 again in11.73s with all six
frozen receipts exact. No whole-gate restart or required-hook bypass occurred.

Intended source files: Makefile; crates/rdf-core/src/turtle_render.rs;
crates/rdf/Cargo.toml; crates/rdf/benches/turtle_chains.rs;
crates/rdf/tests/support/turtle_chains.rs; crates/rdf/tests/turtle_chains.rs;
docs/WASM_TESTING.md. All Stage probes/logs/captures remain ignored evidence.
The actual donor worktree and its zz scratch remain untouched and unshipped.

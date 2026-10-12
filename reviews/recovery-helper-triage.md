# Bounded helper-gate triage

Recorded 2026-10-11 against `recovery-helper-findings.txt` and current bodies.
Read-only source/ledger review: no builds, tests, source edits or process changes.
Root owns the vocabulary, framing, floating control and environment repairs;
this report covers the other 29 findings, including two stale ledger rows.
Recommendations are source judgments, not a claim that the repaired gate passed.

The ledger distinguishes a second implementation from typed adapters to one
implementation. Its normalization deliberately erases types, borrow/move forms,
clones and some conversions. A distinct row is justified only by the actual
different contracts below, with a documented resolving anchor and the exact
reported member set. Do not add blanket path/fingerprint exemptions or change
working semantics merely to change a fingerprint.

## Small actual consolidation repairs

1. **Python response mapping** (`body:acf79934e34bf538`):
   `bindings/python/src/py_geo.rs:53,81` repeat Python detachment plus extraction
   of response text and `GeoCallError` to `PyValueError` mapping. Put the shared
   result-to-text/error mapping in one local helper and call it after each
   original `py.detach(|| self.inner.try_response(request))`. This preserves
   the distinct session/index producers and detachment boundary. No new native
   producer trait is needed. Any remaining tiny wrappers may be documented as
   distinct typed host entries to that shared mapping home.
2. **WASM response mapping** (`body:7e9c978f597017ee`):
   `crates/rdf-wasm/src/geo.rs:63,89` repeat the same response tuple extraction
   and `coded_error(..., OPTIONS_CODE)` conversion. One local helper accepting
   the existing native result is sufficient. Keep the native call, error code
   and response semantics unchanged. Do not introduce another JSON protocol or
   response allocation on failure.
3. **Conversion-only constructors** (`shim:f87af909528e0731`):
   `PreparedCurve::new` (`crates/geo-kernel/src/prepared.rs:1417`) and
   `PyRdfDataset::from_frozen` (`bindings/python/src/py_gts_dataset.rs:37`) are
   exactly single-field `Into` constructors. Use the existing
   `purrdf_lex::constructors!` declaration, preserving signatures, visibility,
   docs and `#[must_use]`. AGENTS explicitly names this home; a new distinct
   exemption would evade that instruction. Its typed-source arm in
   `crates/lex/src/constructors.rs:109` supports the original source type,
   including `impl Into<DatasetHandle>`; no new constructor macro is needed.

These are the narrow actual one-home repairs identified in this review. The
following groups already call their computational homes, or preserve ownership
or grammar distinctions that the census intentionally cannot infer.

## Numeric types and ownership: narrow distinct rows

| Finding | Actual distinction and recommendation |
|---|---|
| `body:705a41be961aaed1`, canonical double/float text | `crates/xsd/src/numeric.rs:808,819` already delegate canonical spelling to `canonical_ieee`. The closures invoke `LowerExp` at different IEEE precisions, and classifications use the original type. Document the f32/f64 distinction; do not cast f32 to f64 or unify their formatting semantics. Anchor `canonical_float_text` already documents original single precision. |
| `body:08a18359fd5eec8c`, `Integer::to_bigint` / `into_bigint` | `crates/xsd/src/exact/integer.rs:254,273`: one borrows and clones the original big magnitude; one consumes and moves it. Normalization drops exactly that difference. A distinct row preserves allocation/owner semantics; implementing the consuming method by the borrowing method would add a clone. Both docs already state their contracts. |
| `body:69a803aee100c946`, `a8f3900fee9ef3e7`, `eedc342b58a45149`, `b55bb0617fc4e918` | `ExactValues` multiply/add/subtract/absolute pairs (`numerical.rs:173,194,303–355,378`) select native admitted versus resident **integer** or **rational** operations. Integer magnitude arithmetic and canonical rational arithmetic are different XSD computations; these bodies implement neither again. Separate exact-pair distinct rows may document the typed dispatch and original scratch/refusal semantics. Add the concrete rationale to a method/type doc used as anchor; do not claim a measured speedup. No new arithmetic trait or policy fallback is justified merely to hide this shape. |
| `body:2c21a553dc8bac26`, `ExactValues::coordinate` / `original` | `numerical.rs:53,63` copies respectively a complete coordinate and one rational through each type's original `copy_in`. `geom.rs:306` retains x/y and optional z/m independently; rational copy has a different payload. Distinct typed adapters are justified; do not replace full coordinate copy with the planar-only helper. |
| `body:90d390142782717c`, `NativeRootValues::deref` / `deref_mut` | `numerical.rs:1113,1121` expose shared versus exclusive borrows over inline/spill storage. Rust's aliasing contracts require these separate trait methods. A narrow distinct row follows existing ledger precedents for shared/exclusive accessors. |

## Constructor routing: distinct validated models

Five findings are thin public routes to different validators:

- `body:1f632e2c894c6b13`: `Applicability::new` versus `Polynomial2d::new`.
- `shim:5737c682cecf2d05` and `shim:68a4f81aa43db529`: their
  `new_in_budget` and `new_in_budget_metered` pairs.
- `shim:0ef3aeac76610846` and `shim:c79b64ed69cce04d`:
  `MetricSourceDomain` versus `PreparedEllipsoid` budgeted and metered entries.

Source homes: `operation.rs:396–455`, `operation/models.rs:130–205`,
`operation/models_inverse.rs:41–95`, `profile.rs:165–220`. The public wrappers
forward original typed fields, budgets and optional observers into each model's
single `new_in_budget_observed`. Rectangle extent validation, polynomial
canonicalization, source-metre domain validation and ellipsoid factor
preparation are not interchangeable jobs. Narrow distinct rows, anchored to
those existing documented public constructors, are warranted. Keep their
native admission and observer paths intact. No common validation algorithm or
universal constructor framework should be invented for these findings.

## Existing shared homes and required typed wrappers

| Finding | Source and disposition |
|---|---|
| `body:21828defae2ec363`, C session/index call | `crates/rdf-capi/src/geo.rs:64,178` already delegate pointer checks, unwind boundary, response ownership and status mapping to **one** `response_call` at line 84. The exported ABI functions take different opaque handle types, call their different native producers and preserve different null diagnostics. Distinct typed ABI adapters are justified; do not duplicate the existing helper or merge the handle types. |
| `shim:fc3f9f54abe0f63c`, C session/index free | Same file lines 117,201 forwards distinct opaque pointer types to the existing `free_handle`. Keep the typed C ABI and exclusive ownership contracts; narrow distinct row. |
| `body:9b93a95f69236c82`, geo/kNN `open` | `sparql-eval/src/geo/relation.rs:414`, `knn/mod.rs:1628` forward to each producer's original `open_admitted` and the one `WorkspaceCursor::into_resident` extraction home. No spatial or nearest-neighbor algorithm is duplicated here. Distinct required trait adapters are justified. Do not make both `PropertyFunction::open` and `open_admitted` default to each other: the existing default `open_admitted` already calls `open`. |
| `body:3efedf4ef41cf3f5`, text occurrence/search `open` | `crates/text/src/relation.rs:1459,1919` forward to each producer's different `open_owned` and the same resident extraction home. Occurrence walking and ranked candidate generation remain their own computations; narrow distinct adapter row. |
| `body:9b8f06e1736a8ae0`, persistent coordinate/rational | `topology/noding.rs:137` and **`topology/relate.rs:263`** both forward to the single generic `context::persistent_numeric` at `context/retained.rs:222`. The census names the latter under `noding::AdmittedWork`, the type's declaration home, despite its implementation's different physical file. This is not a stale or silently repaired finding. Distinct Coord/Rat trait adapters are justified; the admission/detachment implementation already exists once. |
| `shim:6cf2fa3f4a6c73a0`, admitted/legacy truncate | `topology/noding.rs:62,124`: `Vec<T>` and `RetainedBuilder<T>` are different storage owners. Retained truncation (`context/retained.rs:1478`) reaches the uniquely owned retained buffer and drops payloads through that owner. Preserve both typed trait methods; narrow distinct row rather than erasing the retained wrapper. |
| `shim:ba58362c0e076c78`, legacy compare | `topology/locate.rs:361` and `topology/noding.rs:79` adapt native rational `cmp` to two separate work traits/error protocols. The arithmetic already occurs only at Rat's comparison home. Narrow distinct trait-adapter row; do not refactor surrounding traversal or callbacks. |
| `shim:0165e57cd693e4fd`, geometry/point decoding | `validate/src/geo/geometry.rs:140`, `request.rs:31` already call shared `decode::value`, supplying their distinct typed `from_value_native` parsers. Geometry/edge/image and point-coordinate record contracts differ. Narrow distinct public adapter row. |
| `shim:bdb3a1474ca0cfac`, JSON String/bool decoding | `lex/src/json/record.rs:1127,1175` delegate to different `FromJsonNative` implementations at lines 1099,1114. One validates/owns string text; the other validates a boolean without text allocation. The original allocation/error decoder is shared; narrow distinct trait-adapter row, anchored to the documented decode trait. |
| `shim:c63ef6ee91184e4d`, aggregate/cursor resident controls | `sparql-eval/src/agg_fn.rs:635`, `property_fn.rs:1820` wrap different boxed trait objects with the existing zero-byte resident grant. Admission itself remains at `WorkspaceCapability::charge`; cursor and accumulator lifecycle/extraction types differ. A precise distinct row is justified; it must not sanction a second allocator or a bounded extraction bypass. |

## Parser group and stale rows

- `body:c70b2e8f46b6a0df` and the stale three-member parser row are one ledger
  repair. Keep only `purrdf_jsonschema::ecma::Parser::eat` and
  `purrdf_shex::parser::Parser::eat`, updating the existing reason to **two**
  grammars. ECMA consumes characters (`ecma/mod.rs:350`); ShEx consumes its
  grammar's tokens (`shex/src/parser.rs:182`). WKT `Cursor::eat`
  (`geo-kernel/src/wkt.rs:531`) now calls `advance()` instead of directly
  incrementing `pos`, so it no longer belongs to that exact body group. Do not
  change WKT traversal merely to restore the stale exemption.
- Remove the stale two-member `DecodeError::from` **shim** row at
  `helpers-ledger.toml:1830`. Both conversions still exist, but are no longer
  isomorphic: lexical `From<Error>` (`lex/src/json/record.rs:398`) now preserves
  `Storage` and escaped-storage failures explicitly, while RDF
  `From<ProjectionError>` (`rdf/src/projections/json_codec.rs:29`) remains a
  `Self::custom(error)` shim. Do not undo typed storage-error preservation or
  replace the stale row with an unrelated group.

After the coherent root repairs, run the already required helper gate against
current source. New fingerprints/member sets from a real consolidation require
source judgment, not automatic carryover of these receipt-era exemptions.
Broader qualification should follow only where changed behavior or original
mandatory gates require it; this triage adds no test or review ceremony.

## Bounded review of root's shared-home repairs

Root subsequently applied the control/environment, vocabulary and moved-frame
repairs. Source verdict: PASS for these bounded deltas; execution remains
pending. The review read the deleted original files through `git show HEAD`,
the current XSD homes, the actual distance consumers and regression source.

- `rdf-core/src/distance/mod.rs:164` now re-exports the sole
  `purrdf_xsd::ieee::control`; its private `env` name aliases
  `purrdf_xsd::ieee::environment`. The two duplicate implementation files are
  deleted, and the float-control ledger home and entry points identify XSD.
  Public `distance::FloatEnvironmentError`/`Evidence` re-exports at line 182
  remain available. Exact, reassociated, recorded and specified-path resolution
  still call `env::check()` before constructing their resolved arithmetic.
- This consolidation intentionally changes type identity to the shared XSD
  error, makes distance error prose generic numerical prose, and gains
  `TrapsEnabled`. Both original and current error enums are non-exhaustive;
  existing variant field shapes remain available. The hidden control guards
  gain the XSD home's non-Send/non-Sync thread ownership and its runtime-safe
  optional SSE2 availability check. Do not describe the whole public error
  behavior as byte-for-byte unchanged; retain these explicit changes in final
  delivery documentation.
- The shared `check` reads the register before probes. MXCSR checks its six
  exception masks before flush/rounding checks; AArch64/x87 paths have their
  own original shared trapping controls. The existing 13-row probe, scope and
  register handling now have one implementation. The original core probe
  tests remain at `distance/tests.rs`, consuming the XSD home's public hidden
  probe fields/methods rather than a copied table.
- The added `resolving_distances_uses_the_shared_trap_refusal_before_probing`
  regression at `distance/tests.rs:1291` exercises real Exact and Reassociated
  resolution while exception masks are cleared. It checks the same typed
  refusal and original register evidence, then drops the thread-bound guard
  before checking restoration and healthy re-entry. The test performs no
  deliberate floating computation while traps are unmasked; its intended
  assertion is refusal before the behavioral probe. It has not been run by
  this reviewer.
- `slice/src/mapping_support.rs:174–175` now names the existing OGC vocabulary
  namespace constants with identical prefix strings and values. It adds no
  vocabulary fallback or output change.
- The framing variant is not a new exception invented for this gate.
  `origin/main:helpers-ledger.toml` already sanctions
  `purrdf_geo::relation::Digest::field` for its published big-endian
  length-prefixed FNV identity. Original source and current
  `geo-kernel/src/index.rs:92` contain the identical length-prefix/write body
  and frozen-format documentation. Root restored that exact variant at the
  moved type's new home. Replacing it with little-endian `frame_le` would change
  established fingerprints; preserving the sanctioned format is correct.

No new blocker was established by this reread. The coherent repair still needs
the required compiler/helper and affected runtime checks; source review is not
their substitute.

## Follow-up: implemented ledger, constructors and host mapping

Bounded current-source reread requested by root after applying the triage.
Source verdict: PASS for these helper repairs. No new material source defect
or unhandled ownership consequence established. No build, test, census, process
change or shipping-source edit was performed; root owns the running census.

- The 23 added exact `[[distinct]]` rows correspond to the reviewed remaining
  typed numeric/ownership/model/ABI/property/decoder adapters. Their member sets
  and body-versus-shim classifications match the supplied finding receipt.
  They do not exempt the removed duplicate register/environment bodies, host
  response mapping or conversion-only constructors. Anchors resolve by source
  inspection to documented items, including the newly documented `ExactValues`,
  `NodingWork` and `SurfaceArithmetic`, existing `NativeRootValues`, native
  decoding trait and cursor owner, and the existing generic persistent-number
  and C response homes. Final census membership/anchor validation is still the
  gate's job; no receipt-era row automatically sanctions a newly changed group.
- The stale duplicated `DecodeError::from` row is removed and the parser row now
  names only ECMA and ShEx, with the two-grammar rationale. WKT traversal and
  the lexical storage-error special case remain untouched.
- `PreparedCurve::new` and `PyRdfDataset::from_frozen` now use the existing
  typed-source `constructors!` arm. Inspection of that arm confirms it emits
  the same one-field `Into` conversion. Their original parameter types,
  visibility, return types and documentation remain; `PreparedCurve::new`
  retains `#[must_use]`. This adds no clone, parsing, resource admission or
  ownership extraction.
- Python's two call methods pass the result of their original `py.detach`
  producer call into one local `response_text`. Detachment finishes before
  Python exception construction, as before. The helper moves the existing
  String from the same `(String, bool)` response and preserves the same
  `PyValueError` conversion. WASM uses its own single local `response_text`,
  preserving `OPTIONS_CODE`, native producer calls and String moves. Both
  surfaces retain semantic refusal JSON and route operational errors through
  their original host error mapping; neither creates a new error envelope.
- Core control/environment reexports and the actual Exact/Reassociated trap
  refusal regression remain the reviewed source. XSD control/environment files
  are unchanged by this consolidation. Existing public error paths remain, but
  the type identity, generic display wording, added trap refusal and hidden
  guard auto-trait changes remain intentional API consequences documented in
  the preceding section, not an assertion of total API/byte identity.

This follow-up covers the listed helper changes only. Other concurrent numerical
arena/retry changes visible in the same files are not approved by this bounded
helper review. Preserve the normal affected validation and final gate boundary.

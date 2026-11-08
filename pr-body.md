Prepare the coherent PurRDF 4.0.1 release and publish `purrdf-testkit`,
`purrdf-alloc-probe` and `purrdf-conformance-kit` as supported development
libraries. Add versioned dependencies and a topological forty-crate release
set, synchronize Rust/Python/npm versions and the generated C header, and
include reviewed notes while preserving the existing migration history.

Make the packages usable downstream: testkit resolves workspace paths from
its caller, embeds its own JSON Schema meta-schemas with BSD provenance and
recipient notices, and documents the external WASM harness host. Move
repository census checks to checkout-bound runners. Regenerate license
projections with scope-aware notices and update publication inventories.

Fetch pinned CI tools through the official Releases API with exact repository,
tag and asset selection, bounded transfers and a step-scoped read-only token.
Download mdBook fully before extraction; retain the existing pins, checksums,
installed-version and SIMD checks. Browser asset endpoints repeatedly returned
HTTP 504 while the same official artifacts were available through the API.

All three empty `0.0.0` bootstrap records are published and confirmed. The
maintainer confirmed Trusted Publisher setup; strict public preflight verifies
trusted-publishing enforcement on all forty records. Functional versions and
release tags have not been published.

Validation:

- Eight release-profile path tests and strict release testkit library Clippy
  passed; prior focused checks cover moved census and grading behavior.
- Normal archive verification passed for the twenty-one crates in the
  conformance-kit development dependency closure. All three new archives passed
  recipient-license and package-boundary audits, including all seventeen
  meta-schemas and their original notices.
- A real downstream consumer passed against extracted packages with release
  assertions and warnings denied, exercising caller workspace/golden paths,
  temporary cleanup, embedded schemas, allocation windows and conformance grading.
- Version/header bump, forty-four license projections, documented claims and
  release-note checks passed. Independent substantive review's notice correction
  is resolved. Normal signed preparation commit hooks passed.

The qualified EARL suite/harness update landed in #1238 and is integrated here
through managed synchronization. All three community inventories and profile
counts 57/58/15 remain in the relocated census; all three focused cases and
normal sync hooks passed. The release-specific Chinese book catalog preserves
existing translations and passes all six rendered gates with zero template
drift. API downloads of wasm-bindgen, Binaryen and mdBook passed local
checksum/version checks; Binaryen's SIMD check passed. Workflow/shell syntax
and independent transport/source review passed. Hosted installation of all
three tools and both book renderings passed.

Integration corrections are committed through `0ac79466a`:

- The corpus census is corrected from 4,171 to the measured 4,310 files; all
  four focused native release cases pass, with literal/refusal/grading pins
  unchanged.
- Earlier node-expression laws are preserved while November retains its
  explicit semantics. Five Shapes targets, two CLI cases, host restoration and
  strict lint pass. The regenerated product changes only its stage stamp and
  checksums: 96 bytes in the 5,608-byte fixture, with dataset and AST unchanged.
- The actual 4.0.1 release CLI completed all twenty WatDiv queries on frozen
  10M inputs at seed 0, yielding 434,748 rows with all counts unchanged. The
  measured version pins and exact acquisition self-test now pass. Citation:
  Aluc, Hartig, Ozsu and Daudjee, ISWC 2014, pages 197–212.

The fresh managed Python CodeQL job passed after the earlier analyzer download
failed. The final qualified source corrections return ancestor Cargo
manifest read errors through the loader, correct its licensing audit path, and
rebind capacity source identities after two test-only macro migrations. The
original panic helper and capacity proof claims remain supported. Eight path
cases, eleven real loader cases, strict release lint and independent source
reviews pass. The unchanged helper gate passes with all 96 ledger jobs holding.

Fresh hosted checks, final combined release gates and tagged report regeneration
remain pending. Tracks #1235; it stays open through actual release publication.

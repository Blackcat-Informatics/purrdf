# G1 implementation

Status: SUCCESS

Scope: repair HF1 / completion F1 cold composite literal registration. Only
`crates/rdf-core/src/ir/global.rs` changed. No staging, commit, push, forge write,
dependency, feature, public API, parser copy, or other source edit.

Parent HEAD `b2edf7450cf654d20ae97bd856d67102d0916b7b`, tree
`13496228db0e5ec79074acad9020cac5aab7556e` remained unchanged. Final source SHA256
`2934a164143a36cccba6c06c43c626f81438f9d86b59f760d61ca8e2d4696c66`.
Full patch `raw/G1-source.patch`, SHA256
`fef940050d83d6559d215a820864cb2e86f9682ce2cb4365b1acd19681187964`.
Both entries in `raw/G1-source-sha256.txt` independently reverified after all
checks. Working status contains this one modified source and existing `.stage/`.

## Repair and authority

`intern_literal` and the checked validated literal arm now call the one private
`try_intern_literal` registration home. It uses the existing
`cdt_blank::cdt_embedded_blanks` scanner, retaining its occurrence order and
`(label, scope)` identities. Every extracted blank and the final literal use the
unchanged `try_intern_lookup` address/allocation admission and miss-insertion
home. Checked failures propagate `Err(())`; ordinary literal insertion retains
its infallible API and explicit capacity panic. Lexical bytes, language and
direction are passed through unchanged. No second composite parser was added.

The validated-copy documentation now names the actual PageTranslation,
PagedDataset compaction, and PagedStack snapshot authority. Inspected sealed
source dictionary composition, `frozen_head`, and `normalize_row` to establish
that head/removal values pass native freezing before unchecked absolute-IRI
revalidation is skipped. New ingress still must use the validating path.

The cold witness builds three independent dictionaries through ordinary,
checked validated, and infallible validated routes. Final cases cover a
top-level List with nested Map/list/typed composite/triple content, a top-level
Map containing that List, and recursive RDF triple containment. Explicit checks
retain default and scoped equal-label blanks, deep/triple blank identities,
absence of quoted ordinary text identities, exact lexical reconstruction,
insertion-ordered IDs and complete dictionary values, and store-once behavior
across all three routes.

## Executed checks

All Cargo commands use `CARGO_BUILD_JOBS=2`, the repository nightly, locked
resolution, and the unchanged governed profiles. Compiler identity is in
`raw/G1-toolchain.txt`: rustc 1.100.0-nightly,
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1. The earlier effective
profile/configuration receipt remains applicable: no manifest, toolchain,
configuration, profile, or feature changed.

| Command | Exit / observed result | Evidence |
|---|---|---|
| `cargo test --locked -p purrdf-core --lib cold_validated_reinterning_preserves_composite_registration` before production repair | 101; 0 pass / 1 fail / 1155 filtered. Cold checked default `same` blank was `None`, ordinary was `Some(GlobalTermId(2))`. | `raw/G1-regression-before.log`, complete pre-fix patch/hash manifest |
| `cargo test --locked -p purrdf-core --lib ir::global` after initial repair | 0; 24 pass, 1132 filtered | `raw/G1-global-qualified.log` |
| Same global command on final expanded witness/docs | 0; 24 pass, 1132 filtered | `raw/G1-global-final.log` |
| `cargo test --locked -p purrdf-core --test paged_stack --test paged_backend --test paged_fallible --test paged_admission_law --test mutable_declared_graph_drain` | 0; 82 pass: 19 stack, 34 backend, 12 fallible, 11 admission, 6 drain; none ignored/filtered | `raw/G1-core-paging-qualified.log`, `raw/G1-core-paging-tested-source.txt` |
| `cargo test --locked -p purrdf-sparql-eval --test paged_stack_query` on final source | 0; 7 pass, none ignored/filtered | `raw/G1-evaluator-qualified.log` |
| `cargo run --locked -p purrdf-sparql-eval --example paged_stack` on final source | 0; Bob: 2 layers / 2 pages / 202 bytes; Robert: 3 layers / 4 pages / 1088 bytes; one compacted page and actual ordered carrier identity verified | `raw/G1-example-qualified.log` |
| `cargo clippy --locked -p purrdf-core --all-targets -- -D warnings` | 0; warning-free | `raw/G1-clippy-qualified.log` |
| `make helpers-hygiene` | 0; 81 enforced jobs, 23 reasoned variants, 91 exact distinct rows, 1833 files, 80 domains | `raw/G1-helpers-qualified.log` |
| `rustfmt --edition 2024 --check crates/rdf-core/src/ir/global.rs` | 0 | `raw/G1-format-check.log` |
| `git diff --check` | 0 | `raw/G1-whitespace-final.log` |
| Added patch deferral scan | 1; no matches | `raw/G1-deferral-scan.log` |

The original failing witness and production-free patch are retained unchanged;
its source SHA256 was `389f84c40eb726145e69d238d04ac8e48d4d4c96df4b68c05364c96f640746f5`
and patch SHA256 was
`1517f52d0a66670effe3456dda7a4b4e4a56cc18de842ee0e49a3d4e6b8210d3`.
That original List case remains intact in the final regression; the final suite
adds the explicit Map case.

The 82 integration cases executed on source
`f801d280e3bbb62be74f58dd47ccd77e016d2517dbf88397a09e6aa75ec659cd`.
The intervening delta only updated the validated-authority documentation and
added the Map fixture in a global unit test. Non-test production code and all
five integration targets remained identical, so their qualified evidence is
reused on that explicit bounded assessment. The final global suite and actual
evaluator/example were executed again on final source.

## Verification limits and handoff

Existing checked address/allocation admission code and error propagation are preserved;
the global suite executes the maximum-ID refusal. No artificial allocation
exhaustion was induced. No wasm runtime, new wasm build, full workspace check,
hosted CI, commit hooks, or integration was executed by this implementer. This
report establishes the assigned G1 repair and focused qualification, not overall
issue completion. Independent review, normal signed/hooked publication and final
integration remain parent-owned workflow steps.

Source writer and Cargo ownership released. All own execution sessions ended.

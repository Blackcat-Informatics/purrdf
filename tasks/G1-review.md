# G1 independent repair review

VERDICT: PASS

The assigned HF1 / completion F1 repair preserves native composite blank
registration in checked cold literal reinterning. No required G1 finding remains
open. This is independent source judgment and adjudication of attributable
implementation-run receipts, not an independently repeated runtime audit.
G2/PF1 and the remaining Stage 2/3 workflow are not cleared by this report.

## Reviewed source and authority

Issue 457; PR 466; branch `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Parent HEAD: `b2edf7450cf654d20ae97bd856d67102d0916b7b`.
Parent tree: `13496228db0e5ec79074acad9020cac5aab7556e`.
Verified staged candidate tree: `b5a7e8cc9330b199c4f5c33f99c4f7b33bfc3156`.
Only source delta: `crates/rdf-core/src/ir/global.rs`.
Final source SHA-256:
`2934a164143a36cccba6c06c43c626f81438f9d86b59f760d61ca8e2d4696c66`.
Full `raw/G1-source.patch` SHA-256:
`fef940050d83d6559d215a820864cb2e86f9682ce2cb4365b1acd19681187964`.

Independently verified the manifest entries, final working source, candidate blob
digest, parent identity, sole changed path and staged-index equality to the
candidate (git diff --cached --quiet against that tree, exit 0). The actual
staged no-prefix patch digest equals the supplied full patch digest. Source and
Cargo ownership were released before final adjudication; the parent owns the
index. No source/index/history mutation, Cargo/build/runtime run, forge access,
publication or subagent dispatch occurred in this review. Only this report was
written.

Applied the already-read repository laws and backend contract, Stage 2/Stagectl
quality/delegation/no-deferrals/validation instructions, current acceptance index,
G1 remediation plan, gap analysis, historical-fit HF1 and initial separate
completion F1 report. Inspected actual base/current global interning, the existing
CDT extraction home, the regression and affected validated-copy call sites.
Unaffected Task 1/2 source judgment was reused rather than reconstructed.

## Repair judgment

Ordinary `intern_literal` and the literal arm of `try_reintern_validated` now
reach one private `try_intern_literal` registration home. That home calls the
existing `cdt_blank::cdt_embedded_blanks` scanner; no parser, term-conversion or
encoding implementation was copied. It registers each extracted `(label, scope)`
in the scanner's existing occurrence order before inserting the literal. Duplicate
occurrences remain lookup hits. Lexical bytes, datatype, language and direction
pass through unchanged. For non-composite datatypes extraction remains empty,
so quoted blank-looking ordinary text does not mint a blank identity.

Each referenced blank and the final literal uses `try_intern_lookup` and propagates
its error with `?`. The unchanged lookup home checks arena length/address and
term-ID representability, reserves arena/term/index capacity, then uses the single
miss-insertion implementation. Checked callers do not invoke infallible blank
registration as a fallback. The ordinary public literal API retains its explicit
capacity panic; checked reinterning retains `Err(())`, which stack composition
maps to `PagedStackError::Capacity`. Partial private dictionary growth is not
published when composition fails. No artificial allocation exhaustion was tested;
this capacity-propagation judgment comes from source inspection and the existing
maximum-ID refusal test, not a claim to recover from every allocator failure in
the shared scanner/work-list machinery.

The validated-copy authority documentation now names the actual translation,
paged compaction and stack snapshot callers. Inspected source dictionaries,
`frozen_head`, graph declaration and `normalize_row`: head/removal values pass
the native freeze boundary before retained composition skips repeated absolute-IRI
validation. The copy path remains crate-private. Fresh ingress must still use
the validating native path; the repair neither broadens its authority nor
silently changes datatype/IRI semantics.

## Cold regression integrity

The new witness starts three separate empty dictionaries and invokes ordinary,
checked validated and infallible validated ingress independently. It exercises
a top-level List with nested Map/List, embedded composite-typed literal and
triple content; an explicit top-level Map; and recursive RDF triple containment.
It asserts that five concrete embedded identities exist, including default and
scope-7 blanks with the same label, while quoted ordinary `quoted`/`opaque`
identities are absent. Full reconstructed values, numeric IDs and all insertion-
ordered dictionary values agree. Repeating every route retains the original ID
and dictionary length. The test therefore cannot pass merely because ordinary
ingress warmed the dictionary first, unlike the former regression.

Read `raw/G1-regression-before.log`: the production-unmodified cold witness
failed with checked default `same` lookup `None` versus ordinary
`Some(GlobalTermId(2))` (exit 101; one failure). The retained pre-fix source hash
is `389f84c40eb726145e69d238d04ac8e48d4d4c96df4b68c05364c96f640746f5`;
pre-fix patch hash is
`1517f52d0a66670effe3456dda7a4b4e4a56cc18de842ee0e49a3d4e6b8210d3`.
The original List case remains in the final regression; the Map case adds
coverage. Final global qualification executes this witness successfully. No
assertion was weakened, old failure hidden or native invariant redefined.

## Qualification and evidence applicability

Read final `tasks/G1-implementation.md` Status SUCCESS and exact command/exit
handoff, relevant logs and `raw/G1-core-paging-tested-source.txt`. All final gates
below report exit 0. Cargo uses `CARGO_BUILD_JOBS=2`, locked resolution and the
unchanged governed profiles. Compiler capture `raw/G1-toolchain.txt` is rustc
1.100.0-nightly, commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1.
The earlier effective opt-level 3/assertions/overflow profile evidence remains
applicable to configuration; no manifest, toolchain, feature or profile changed.

| Executed command | Attributable result |
| --- | --- |
| `cargo test --locked -p purrdf-core --lib ir::global` | Final 24 pass, 1132 unrelated filtered; `raw/G1-global-final.log` |
| `cargo test --locked -p purrdf-core --test paged_stack --test paged_backend --test paged_fallible --test paged_admission_law --test mutable_declared_graph_drain` | 82 pass: stack 19/backend 34/fallible 12/admission 11/drain 6; none ignored/filtered; `raw/G1-core-paging-qualified.log` |
| `cargo test --locked -p purrdf-sparql-eval --test paged_stack_query` | Final-source seven guarded consumer cases pass; none ignored/filtered; `raw/G1-evaluator-qualified.log` |
| `cargo run --locked -p purrdf-sparql-eval --example paged_stack` | Final-source actual public example passes: retained Bob 2 pages/202 bytes, current Robert 4 pages/1088 bytes; qualified origins and one compacted page's ordered carrier identity asserted; `raw/G1-example-qualified.log` |
| `cargo clippy --locked -p purrdf-core --all-targets -- -D warnings` | Warning-free completion; `raw/G1-clippy-qualified.log` |
| `make helpers-hygiene` | 81 enforced jobs, 23 reasoned variants, no open copies, 91 distinct rows, 1833 files and 80 unique prefix-free domains; `raw/G1-helpers-qualified.log` |
| `rustfmt --edition 2024 --check crates/rdf-core/src/ir/global.rs` and `git diff --check` | Clean; `raw/G1-format-check.log`, `raw/G1-whitespace-final.log` |

The 82 integration cases ran at global source SHA-256
`f801d280e3bbb62be74f58dd47ccd77e016d2517dbf88397a09e6aa75ec659cd`.
The intervening changes only corrected validated-authority documentation and
added the top-level Map fixture in the global unit test. Non-test production
behavior and all five integration targets were unchanged, so those executions
remain applicable on that bounded assessment; they are not represented as a
new run. Final global, evaluator and example executions qualify final source.
The source adds no dependency, feature, platform API or alternate implementation.

No assigned G1 repair or required local check remains unfinished. This report
does not claim a new wasm build/runtime run, full workspace gate, hosted CI,
signed/hooked commit, push/readback, feedback publication, final completion audit
or integration. Those are distinct parent-owned workflow steps. In particular,
G2/PF1 remains outside this repair verdict and must be adjudicated separately.

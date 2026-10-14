# Logical-capacity panic census design

Design review only; no executing source, tool, ledger or build was changed.
Implementation waits for the current full gate to settle. The old tooling-freeze
statements in historical reviews are superseded by the current authorization;
compiler, cache, target-directory and runner machinery remain outside this work.

## Requirement and evidence

Read the live issue through the installed Stage launcher (`stagectl issue 490
--json`, updated 2026-10-13T08:53:10Z), the accepted plan, the retained raw
recovery-capacity-source-audit.txt and the capacity-source-audit-followthrough
and current-capacity-contract-independent reviews. The issue requires no
non-test core/Datalog expect/assert on table capacity, enforced by a census.
The accepted plan additionally names capacity unwraps and forbids disguising
data exhaustion as an invariant. Manual search is useful input, not enforcement.

Logical refusal means representability of a term, text range, row/permutation,
scope, provenance or branded Datalog identity/count. Allocation/layout/admission
failure is a different category. The pre-existing statement histogram's
try_reserve failure and generic SmallVec allocation/growth are not logical table
ID exhaustion. This distinction does not certify those resident APIs against
allocator failure or authorize a generic exemption for anything named reserve.

## Existing integration to reuse

Use one Rust rule home, preferably a focused `capacity.rs` beside layout.rs and
structure.rs, returning the existing rules::RuleHit. Add its rule identifier to
the existing rule registry and invoke it from Workspace::scan_rules on the
already parsed syn::File. That scan parses each file once and already reports
parse failures. Reuse rules::Scoped/scoped_symbol, source::is_test_only,
rules::is_test_file and visit_macro_arguments rather than another file walker,
Rust lexer, cfg parser or script.

Register one enforced ledger job, `logical-capacity-panic`, with the actual
typed home `purrdf_lex::allocation::CapacityError::check`, entry point slot, and
`rule:logical-capacity-panic` as its forbidden fingerprint. Scope the detector
to packages purrdf-core and purrdf-datalog; findings have home_exempt=false.
This is a safety rule, not a duplicate-body exception. Give the job a concrete
specification and its seeded Rust controls; no whole-function forbidden variants
or approved-panic inventory should hide future capacity changes.

Both `census::check_findings` and `census::index` already consume rule_hits.
The existing check-shared-helpers.py reads --index, judges every enforced job,
and fails on copies/findings. Therefore the ordinary existing helpers gate and
make check can enforce this job without a new CLI mode, Python checker, Makefile
recipe, CI job or compiler invocation. Test both Rust --check and the emitted
index contract; passing an isolated visitor alone does not prove gate wiring.

## Detector boundary

The rule must inspect executable AST, including trait defaults, impl functions,
closures, local bindings and const expressions, before any fingerprint minimum
length threshold. Comments, doc examples and diagnostic prose are not code.
Reject panic sinks on logical-capacity expressions, not merely strings that
contain `capacity` or `u32`:

1. `.expect`/`.unwrap` (also qualified Result/Option calls and explicit panic
   error branches) on typed capacity checks or the actual logical mint/range
   helpers. Include a capacity result assigned to a local before the sink.
   Existing producer wrappers returning CapacityError must not escape detection
   solely through an import alias, a harmless map/map_err, or a renamed local.
2. Panicking fallible integer narrowing of table/text lengths or next-ID/scope
   counts, and checked increment/addition feeding their representation. Seed
   the original aggregate CSR u32 narrowing as well as the issue's listed
   constructors; the census must discover such composed-table code outside the
   original file list.
3. assert/debug_assert families or an explicit `if`/`match` panic branch that
   enforces a representability ceiling on a runtime table/count. Recognize both
   comparison directions, cast/parenthesized forms, numeric MAX or equivalent
   literal limits, named/reduced limit locals and the original next-count form.
   An assertion need not carry a message to be a violation.

Keep the analysis local and conservative: track expression provenance through
local bindings and the actual capacity-producing call paths, then invalidate a
local proof when its operand is reassigned. Do not write a general Rust type
checker or silently classify unresolved suspicious expressions as safe. The
implementation review must see the detected inventory and dispositions before
the current-tree zero finding is accepted. Method-name fragments and diagnostic
words can supply additional candidates, never the sole acceptance classifier.

Non-test selection is significant. Test-only cfg items are omitted through the
existing predicate logic; `cfg(any(test, target_arch = "wasm32"))` is production
and must remain checked. A test-only inline module must not suppress its
following production sibling. File collection currently scans all source files,
including an out-of-line test module whose file is not under tests/. Preserve
the existing distinction between file scanning and shipping reachability: use
the source walk's shipping/module information where required, or conservatively
report the ambiguity. Do not silently equate an arbitrary file name with test
status. Add a seeded out-of-line cfg(test) case to settle this integration.

Macros are another explicit boundary. visit_macro_arguments sees expression
arguments but does not expand macro_rules transcribers. A table-width panic
hidden in a local declaration macro cannot simply be ignored. Reuse syn/token
traversal at this same rule home for transcriber groups and recognize the same
panic patterns, or report a suspicious unanalysed macro body as a finding.
No separate textual regex checker or compiler expansion subprocess. The seeded
macro control must prove the chosen handling before claiming complete coverage.

## Invariants and physical refusal

Preserve these distinct current cases and test the distinction structurally:

- Local/global/Datalog NonZero construction follows CapacityError::slot(...)?
  on the same unchanged index. That typed check dominates the representation
  expect; the expect itself is not a data-exhaustion refusal. The exemption must
  disappear if the check is deleted, moved into only one branch, applied to a
  different index, or followed by mutation of the index. The word `checked` in
  an expect message proves nothing.
- An actual resident usize length widened to u64 on supported 32/64-bit targets
  is not a narrowing capacity refusal. Do not treat the reverse conversion of
  arbitrary u64 input as equivalent. Existing resident_index invariants need
  their owner/producer proof, not an exception for all functions with that name.
- The completed four-slot Option array in TermDictionary::with_interned and
  fixed layout size assertions do not impose a data-dependent table ceiling.
  Keep their original runtime/invariant behavior.
- HashTable/Vec physical allocation refusal and generic SmallVec layout limits
  are outside this logical policy only when the actual expression establishes
  that category. A StorageError-valued wrapper can also carry CapacityError, so
  exempting every StorageError or try_reserve-named operation would be unsound.

Prefer narrowly proved AST forms over an allowlist. If an existing invariant
cannot be proved by the bounded detector, report it for a concrete source
disposition; do not waive an entire function or bless a panic-message string.
The final rule's stated coverage must match its implementation: a syntactic
census is not a proof against arbitrarily obfuscated Rust. The current manual
site audit and public boundary tests remain complementary required evidence.

## Seeded controls and acceptance sequence

Use existing source::Memory/Workspace::walk and syn fixture homes. Tests live
in Rust; no ratcheted Python fixture additions or copied parser infrastructure.

Required rejected fixtures:

- Each original issue shape: term mint, arena range, quad/reifier/annotation
  count, lazy permutation, delta terms, merge/import scope, all three provenance
  tables and Datalog branded mint; include the aggregate CSR regression.
- Same failures without diagnostic strings, through renamed locals/imports,
  checked-add then unwrap, a named limit, reversed comparison, namespaced
  assertions, explicit error-to-panic branch and a macro transcriber.
- A public wrapper that converts the original typed capacity result to panic.
- The checked-NonZero invariant with its proof deleted/wrong/conditional/stale.
- A production item below a test module, cfg(any(test, wasm32)), a tiny function
  below the normal clone-fingerprint token threshold, and an impl/trait default.

Required accepted neighbours:

- Exact typed propagation with `?`/Err and unchanged healthy construction.
- The actual dominating slot-check/NonZero form, fixed-size/niche assertions,
  completed fixed-array initialization and supported-width widening.
- Physical try_reserve/layout refusal examples that contain capacity wording
  but no logical mint/range ceiling; unrelated semantic assertions.
- Strings/doc examples, #[test], nested/all(test) and out-of-line cfg(test)
  fixtures, test/bench/example roots, plus unrelated packages.

Integration controls must seed a production violation into the in-memory
workspace, observe its exact package/path/item/line and logical cause through
rule_hits, and require both check_findings failure and an index job match with
copies > 0. Removing the violation must clear the finding; malformed source
must still fail the source walk. This prevents an unregistered or inert rule
from passing merely because the current real tree is clean.

After the running full gate is terminal: implement this one bounded rule/job,
run its seeded controls and affected warning-denied helper-census compilation,
then the unchanged existing helper gate on the complete current tree. Review
every real finding rather than tuning the detector until green. Reuse already
qualified producer/public tests unless a resulting production correction changes
their path. Record current zero logical-capacity findings, actual gate wiring,
and any unresolved classifier coverage honestly before satisfying the issue's
census criterion. No build or implementation acceptance is claimed here.

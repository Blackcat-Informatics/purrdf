<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Host storage carrier review

Current verdict: incomplete production slice; no host-runtime or whole-contract
approval. Reviewed the dirty Python/WASM carrier delta after checkpoint15bc79027.
The generated C header/prose are a separate already-qualified commit boundary.

Shapes sync and async storage failures now route the original native diagnostic
through the existing JobError home. Its storage-only JSON projection uses the
native diagnostic serializer, retaining exact u128 counts as decimal strings.
Both JS routes expose parsed diagnostic separately from the existing presentation
shape. Native reasoning query failures keep their original diagnostic and use a
no-payload rendering tag for the existing contextual message. This avoids a new
retained String in every JobError.

Review findings and disposition:

- The new synchronous JSON parse branch originally silently discarded a required
  storage record. It now returns the actual parse refusal, consistent with the
  asynchronous adapter throwing that refusal. Bounded source finding resolved.
- Configured serialization still discarded native diagnostics through a private
  String helper. Its current helper retains RdfDiagnostic and the actual query
  serializer retains that cause in JobError. Bounded source finding resolved;
  compile and real sync/async package execution remain required. The synthesized
  non-UTF8 invariant error needs its existing host rendering assessed explicitly.
- Python parse/load/serialize private helpers erased original diagnostics before
  ValueError construction. Current caller adaptations reuse the existing typed
  presentation home and retain old contextual prefixes. Actual installed package
  error attributes and all adapted callers remain unqualified.
- Store load truncated an unchecked u64 counter to u32; MutableDataset wrapped
  exhaustion to scope1. The written checked u64 counter retains an exhausted
  sentinel and reports native ImportBlankScopes capacity. It still mints eagerly,
  so valid blank-free documents can refuse for a scope they do not use. The owner
  is completing demand-based admission across nested/direct/CDT blank terms and
  declared graphs. Repeated refusal, no aliasing and blank-free success require
  actual boundary controls before this finding can close.

The standard bounded Python/WASM library check is running on its captured source
slice. No test or gate result is inferred from this source review. No build,
toolchain, cache or target-directory machinery changed.

Subsequent disposition: the first bounded host check ended101 with four WASM
carrier mismatches and four Python unnecessary qualifications. The owner corrected
these source adaptations; no corrected compile has yet run because admission is
below32GiB. This failed receipt remains failed.

Root reviewed the new binding-owned load.rs production home, its actual Store and
MutableDataset call sites and same-package load_scope.rs integration target.
The shared fallible native blank visitor detects direct, nested and CDT identities;
declaration-only blank graphs also mint a scope. The last valid u32 ordinal advances
to a retained u64 exhausted sentinel. Blank-free loads consume no scope. The
exclusive Python borrow persists the updated counter even after a native Result
error. A private LoadError keeps original lexical insertion and declaration error
boundaries separate, so the host reuses its original rendering homes without
changing the ordinary JobError layout. The converter moved rather than duplicated.

Bounded source verdict PASS for this demand-based law. Actual native boundary
tests, compilation and installed Python/WASM error acceptance remain NOT MET.
The first blank-bearing quad is converted twice while selecting its scope; this
is not an allocation-neutral or throughput improvement claim. The original
synthetic invalid-UTF8 serialization invariant now has an explicit native code;
ordinary codec and storage errors retain their original cause and contextual
rendering. This narrow invariant-error change is separately accounted for.

Actual execution supersedes the earlier bounded source confidence: corrected
Python and WASM libraries compile, but the new native load control fails.
Its initial CDT fixture used HTTPS instead of the shipped HTTP datatype; after
that fixture correction the independent stored-identity assertion exposes
embedded CDT blanks retaining scope zero even though the document scope is minted.
The native `from_rdf_term_in_scope` literal branch does not apply that scope.
Direct and triple-term blank branches already do. Whole load-boundary verdict
remains BLOCKED pending the actual production correction and runtime acceptance.

Root inspected the existing unchecked CDT binder and its admitted builder caller.
The binder is explicitly the owned-model bridge, applying identical lexical
binding without adding a new document-validation verdict. Reusing that home in
the nondefault scoped converter is a bounded repair consistent with the existing
infallible owned conversion contract. Preserve the default branch and ordinary
literal bytes; prove independently observed nested CDT identities and default
roundtrip, then rerun the actual load control. This is a source-design disposition,
not evidence that the correction is written, compiled or passing.

Final bounded execution: the correction now uses a private scoped-literal
constructor behind the original public literal constructor. Its existing
admitted Cow binder precedes lexical ownership; an owned bound result transfers
directly, and the default path remains unchanged. Root read the actual source.
Session90188 ends0: the independent native nested-CDT converter regression passes
1/1 and the actual binding-owned load implementation target passes2/2 across all
five blank ingress shapes and exact scope exhaustion/refusal controls. This
closes the observed native CDT-scope defect for those controls. Installed Python
error attributes, actual WASM sync/async record execution, affected strict and
whole-contract gates remain unqualified. The earlier source-only PASS does not
replace these execution limits.

Current acceptance supersedes the pending affected-strict clause: session20064
passes core/WASM/Python all-target strict Clippy and three native storage carrier
controls. Standard installed Python build/install session17593 completes0;
six unchanged installed test files pass57/57, and actual Store and MutableDataset
repeated-CDT loads both preserve distinct document blank scopes. These prove
ordinary installed host wiring and the observed CDT repair, not synthetic
installed capacity-error attributes.

Root additionally inspected the new async diagnostic getter against the existing
poison gate and scheduler. It receives the ordinary trapping export gate, as do
existing error getters. `runJob`/`startRun` reject poisoning directly; the root
driver does not enter `jobFailure` on that rejection, and its finalizer avoids
`finish` while poisoned. No getter/poison defect is established by this source
review. The unchanged poison tests are queued for the fresh optimized package;
actual current-package execution remains unqualified until those receipts exist.

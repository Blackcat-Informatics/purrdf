<!-- SPDX-License-Identifier: CC-BY-4.0 -->
# Source ownership handoff

Root has started the complete two-contract implementation under the independently
accepted `plan.md`. Its full plan was posted to both issue threads. Neither issue
is implemented or qualified yet. The entire isolated worktree now passes to
numeric_reasoning_regroup as sole source writer; root will be read-only on source
and can independently adjudicate the completed contract and evidence.

Seven dirty tracked files are the initial coherent API migration, still unfinished:

- `lex/allocation.rs`: native `CapacitySpace` closed identities and exact u128
  attempted/limit `CapacityError`, retained in `StorageError::Capacity`.
- `rdf-core/ir/term.rs`: one generic `StrRange<W>` and original append/read home,
  compact u32 locally, u64 globally. Synthetic actual-range arithmetic crosses
  four GiB, refuses local space, and reports u64 overflow exactly; tests written.
- `rdf-core/ir/global.rs`: global stored ranges are genuinely u64; ordinary and
  validated misses share complete pre-reservation and typed capacity/allocation
  failure. Public blank/literal/triple ingress now returns `StorageError`, IRI and
  whole value ingress reuses native `IriReadError`. Removed unchecked private
  `reintern_validated` wrapper. CALLER MIGRATION IS STILL REQUIRED, including
  page translation, compact and original tests. Preserve typed causes at those
  public surfaces; do not restore an unchecked parallel wrapper to compile.
- `sparql-eval/error.rs`, `workspace.rs`, `property_fn_eval.rs`: typed capacity
  refusal survives original query boundary; code/display added.
- `validate/regime.rs`: certificate stores space/attempted/limit without dynamic
  leaked identities; decode checks a genuine refusal. Extend original round-trip
  cases and error-code metadata/census as appropriate.

No formatter, compiler, runtime, gate or commit has run on these edits. Their
source is intentionally an unfinished coherent migration, not acceptance. Finish
the complete source batch before targeted strict qualification. Both original
contracts remain required: all listed local capacities and all wide Datalog
brands, actual no-remap host joins, header-only sparse stack, immutable shared
historical tombstone/declaration ownership, chronological effective RDF1.2 set,
whole-operation source budget/fault barriers and actual lexical purge. R1 is
discharged at design level only; see `reviews/plan-review.md`.

Existing captures and assessment already identify current native homes. Do not
repeat broad intake or the unchanged plan review. Read standing source law and
the complete plan, then implement. The root `.baseline` and `.goals` are absent
from worktrees and live in `/home/paudley/Active/purrdf`.

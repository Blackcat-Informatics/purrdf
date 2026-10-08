# Independent interface preparation for Task2

Read-only preparation by rules_intake, independently checked against the existing
RulePlan, RuleRuntime, SlotSolution, SourceRow and evaluate_rule interfaces.
This is design input, not implemented or qualified behavior.

- Private factor descriptors preserve authored positive/body and global slot
  coordinates plus physical operator/cyclic group subsets. Do not compile new
  component subrules or create another public evaluator.
- Connect all four positive-variable positions. Union factors coupled by outer
  variables of a negative atom or guard-free negative conjunction. Runtime maps
  head projections and negative ownership. Ground negative predicates need a
  zero-variable precondition; no-positive rules retain relational identity.
- Reuse indexed and leapfrog extension kernels. Partial source vectors cannot
  use whole-rule swap arrays; restore partial authored body indices, then combine
  complete sources in authored order. One-component rules retain current path.
- Evaluate each factor Full once per frozen round; classify a match New iff any
  SourceRow is in Delta, otherwise Old. Old/New/Full mode indexes borrow matches.
  Factor first-new decomposition preceding Old, anchor New, following Full is
  distinct from current scan_for's LAST-new-atom convention. Test independently.
  Model-reading scheduled rules retain Delta::all. Any empty factor blocks firing.
- Per head, project each factor onto head-used slots. Non-head factors have empty
  projection; each head conjunct has its own projection table over shared matches.
  Lazily visit necessary projected products and emit existing RoundBuffer entries.
- Each projection/mode bucket has height-threshold sum-first AND pure-lex witness
  frontiers. Use saturated global height and sum laws from plan.md. Extract shared
  lexical comparison rather than constructing fake-height candidates. Source fact
  lexical ties and authored-source ties both matter.
- Any opaque body or negative-conjunction guard prevents projection/collapse.
  Keep true multiplicity, deterministic tuple traversal and whole-stage barriers;
  never change to a per-row guard chain or memoize stateful callbacks. Full-once
  factor traversal can change callback input order, so version and document any
  observable correction, never assert old ordering without evidence.
- Assumed confirmations leave RoundBuffer as lexical Derivation without summed
  source heights and schedule currently picks first. Task3 needs shared canonical
  comparison and retained metadata; raw row handles cannot outlive layer retraction
  and index rebuilding. Task2 must not pretend this separate path is already fixed.
- Full/Old/New indexing must not double-charge a matched factor. Admission and
  negative probe exhaustion feed the later shared-credit Task4 design. Callback
  whole-Vec and SHACL eager producer allocations remain later portfolio obligations.

Production path remains CLI shapes_tools -> shapes SRL -> scheduled Datalog ->
evaluate_round -> evaluate_rule. Ordinary entailment inherits shared substrate;
the separate existential chase does not become repaired by these changes.

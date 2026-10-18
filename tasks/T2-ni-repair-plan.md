# Native NI saturation repair under issue 596

The exact failing source and mathematical contradiction are in T2-reserved-current.md. The producer and normal verifier both accept consistency; parsed axioms are correct. Keep the clean failing recursive-origin regression. Current reserved changes remain uncommitted until the required native correctness repair passes.

Primary specification: https://www.cs.ox.ac.uk/boris.motik/pubs/msh09hypertableau.pdf , section 3.2.6, rule precedence and Table 5. Annotated self-equality retains NI information; ordinary equality-tautology elimination is insufficient. This specification comparison guides the repair; the exact final implementation must still be tested and reviewed.

## Investigated seam

clause::BodyAtom::Successors and the matcher enumerate strictly increasing successor subsets. This is sufficient for ordinary at-most equalities, but loses repeated-variable instances whose equality is trivial while its NI annotation is not. Hyper::append_ni_alternatives currently runs only after an ordinary violating at-most match and treats NI choices beside ordinary pairs. The nested fixture can reach a falsely accepted blocked completion before that pressure exists. Do not try to repair this by disabling blocking globally, adding a source-namespace heuristic, changing the verdict expected by the test, introducing another solver or weakening proof checks.

## Concrete repair direction to validate

1. Preserve the ordinary at-most clause/source owner. Add an explicit NI-only branch kind for an eligible nominal root and one actual blockable generative neighbour, rather than requiring a full violating distinct-successor tuple. Its source is the same normalized at-most clause; its root/candidate binding is compact and typed. Share the native eligibility/site/predecessor implementation already added for independent replay. Inspect source guard/index filtering as well as branch generation so ordinary cardinality prefilters cannot hide this case.
2. NI branches must offer the native reserved-set choices exhaustively and obey rule precedence before a supposedly satisfied ordinary equality can discharge the trigger. A selected root/candidate becomes unblocked through the existing generated-root/merge operations. Keep named/generated identity orientation and the existing stable reserved pool. Charge actual branch work before allocation through existing semantic budgets; no new host cap or compiler/profile workaround.
3. Give the branch an explicit wire discriminator in a new native proof layout. Older layouts retain their existing interpretation. Structural replay must regenerate the NI choices from the owning at-most role/filler/bound and binding, reject missing/duplicate/wrong cases and invalid root descriptors, and distinguish this branch from the ordinary equality alternatives. Do not pretend NI-only cases are the ordinary clause's unmodified head.
4. Current-support replay must validate the exact compact binding, actual source class guards, root identity, role/filler and generative predecessor before applying the selected reserved head. Retain all source/alias/role licenses in the existing causal journal. Branch-path checking remains exact. Whole-refutation verification must not turn a legacy unsupported leaf or data record into a checked closure.
5. Revisit model checking/unravelling acceptance on this source. An accepted completion must not rely on unsaturated NI obligations. Production search and independent verification must agree through the same native rule owners; add a forged unsaturated-completion rejection, not a second semantic engine.

## Acceptance

- The exact nested source returns inconsistency and normal verification accepts its genuine complete refutation.
- Its public causal graph contains a reserved birth whose original typed origin is itself reserved, preserving the parent descriptor and original named/scoped source anchor.
- The existing signed inverse, named inverse-partner, scoped-blank, bad nonblockable-target, wrong-prefix, missing-support and merge-closure cases remain valid.
- New NI branch wire round trips and malformed/incomplete branch inventories are tested. Native and actual scalar WASM exercise the original source; finish the optimized packaged acceptance before full-issue PR readiness.
- Preserve existing oracle, termination, work-accounting and large-bound contracts. Do not bless changed answers or widen budgets to make a regression disappear. Record justified algorithmic work changes separately from native verdict correctness.
- Update gate registration and documentation from the actual inventory. Keep original full issue requirements and downstream logic queue active. Obtain independent/CodeRabbit review when available and merge only through ghprsq after complete qualification.

This is an implementation direction, not a completed design or approval. Resolve any contradiction with the primary calculus or repository invariants before editing that seam. The immediate next action is to inspect branch generation/indexing, branch recorder/codec and completion-view handling together, then implement the smallest coherent native repair covering the failing source.

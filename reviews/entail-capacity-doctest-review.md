# Entail capacity doctest review

Verdict: PASS for the bounded source correction; current executable doctest
result is pending root's already-running check. No build or source edit by this
reviewer and no whole-gate success claim.

Read the complete eight-file Entail diff from141b43229. It changes12 existing
examples: rif_resolver; certain_answers; entails; warrant verify;
explain_conclusion; justify; materialize; Reasoner; proof_context;
extract_module; extract_module_with_proofs; profile. These map to the original
12 failing Entail doctest blocks in the retained full-gate receipt.

All newly changed intern_iri and push_quad calls propagate their actual
StorageError before term IDs reach RDF construction. Each block ends with the
hidden typed `Ok::<(), purrdf_lex::allocation::StorageError>(())`. The two
explain.rs blocks already propagated their builder operations; their only new
need was this enclosing return type. purrdf-lex is already a direct dependency.
The existing freeze and entail/reasoning API expect boundaries remain intact,
so the examples do not require erasing those distinct errors into StorageError.

The actual RDF terms and rows, imported document/refusal neighbour, materialized
regimes, transitive and subclass conclusions, warrant replay/witnesses, proof
checking, justification minimality/sufficiency, module extraction/proof parity,
and profile assertions are unchanged. No test is ignored or converted to
compile-only, no assertion is weakened, and no production/helper/dependency or
tooling body changes. Source review found no material defect. Preserve original
failed receipts and append the actual focused doctest terminal once available;
this source review cannot discharge executable acceptance by itself.

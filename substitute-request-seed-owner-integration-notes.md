# Original-owner request, seed and matched-position conversion

This ordinary patch supersedes `substitute-pattern-owner-draft.patch`; do not apply
both. Root authored only Stage postimages. Shipping integration and builds remain
owned by the source writer. Ordinary apply --check and rustfmt parsing passed;
no compiler, test or full-gate success is claimed for this proposal.

Required paired helper APIs in the same substitute module:

- GroundFailure::{Diagnostic(RetainedDiagnostic), Operational(EvalError)} and
  GroundFailure::into_resident_diagnostic for legacy resident boundaries.
- ground_term_from_value_with_memory(value, &mut Memory<LexicalFrame>) and
  clone_ground_with_memory(ground, &mut Memory<LexicalFrame>), both returning
  Result<GroundTerm, GroundFailure> using the original shared postorder body.
- Allocation-free GroundTriple and TriplePattern Child destruction at tree.rs.

The native request builder admits its actual probe array before construction.
Owned and borrowed TermValue inputs ground directly in the original query Memory;
paired Ground inputs deep-clone through the same admitted ground body. Paired
variables shallow-share their intrinsic owner. Native caller names publish through
WorkspaceCapability::authored_text/Variable::from_admitted; the legacy resident
adapter preserves its existing interner. Original lexical/physical refusal remains
GroundFailure and cannot be interpreted as an unbound cell.

One seed_entries_with_memory home now constructs all full and indexed VALUES
seeds. It admits variable, cell and row arrays before construction; cloned ground
boxes join the original query grant while lexical leaves keep intrinsic grants.
It preserves input order, multiplicity, zero-entry row behavior and blank identity.
Resident seed_row and seed_of delegate to that home. Whole-query native caller
migration must pass the original enclosing query Memory instead of these adapters.

The matched-position walk now has one native probe_positions_with_memory body.
Its original eight inline positions use the common WorkList, with actual admitted
spill; probed indices grow fallibly under their original owner. Pattern construction
uses the paired term_pattern_from_ground_with_memory body, preserving subject /
predicate / object traversal, pushdown versus SHACL predicates and SeedOnly rules.
The resident adapter explicitly includes its pre-existing resident probed buffer;
bounded callers must supply its already owning Memory and never use that adapter.
Replacing a variable destroys only its intrinsic lexical owner, not an untracked
original child-box subtree. Existing generated reference and 100000-depth fixtures
reach the new bodies; three added pattern owner/refusal cases remain unrun.

Remaining work is required: retained prepared-probe buffer/cell ownership, all
scope/pushdown/driver/expression rewrite stacks and node construction, original
parser AdmittedQuery grant adoption and publication, and actual production entry
callers. This packet neither closes those paths nor certifies bounded behavior.

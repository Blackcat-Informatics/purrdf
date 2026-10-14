# Bounded LATERAL extraction review

Scoped source/native verdict: PASS; optimized stack qualification remains open.
I read the exact final binop.rs diff and original native receipt. No build or
source modification was performed for this review.

The merged-row arm and its evaluation, first-error, grant, ledger and guard
handling remain byte-for-byte unchanged. The disagreeing-layer tail is moved,
not copied, to one private helper in the same production home. The current row
remains owned by the caller and borrowed across that call; the helper retains
the original nested-site lookup, layer order, placeholder/copy ownership and
evaluation window. The redundant reference removed after the strict failure
now matches the borrowed parameter. No allocation, caller-only compatibility
path or altered result contract is introduced by the extraction.

The actual terminal-zero receipt covers binop22, owned-admission3,
positive-region3, evaluation-stack10 and LATERAL20 controls:58 total.
Strict checks pass at their recorded source scope. This establishes the tested
semantics and ownership; it does not establish a smaller optimized frame or
restore any depth floor. Unchanged helper census still needs to verify that the
extraction introduces no duplicate group, with the older numeric ledger failure
retaining its actual status.

The source uses an out-of-line Rust boundary, but the final optimizer may change
that boundary. The hypothesis must therefore be tested on the exact linked
optimized module with the original bytes-per-level and floor controls. NOT
EXISTS remains unfixed. No commit, package pass, issue closure or broader numeric
acceptance is inferred from this review.

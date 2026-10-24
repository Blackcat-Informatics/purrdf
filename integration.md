# Current integration acceptance

Current main is8652dee3d, head0803c697b. The clean native merge-tree candidate is
dbcbb7ee6653ef5aa47971e13dfcbe7dfffc085b. PR CI40058322933 passes all required
checks (46 jobs, full rollup clear), but its original workspace checkout log
explicitly shows merge416eee854 against the earlier main6c9c66fcf. That input
run alone is not claimed as execution of the current candidate.

The base delta is the original-definition SHACL memo/resolver repair. This branch
retains CDT native diagnostic causes and adds the standard UcharError traits;
neither changes the other's signatures, model schemas, dependencies, build
configuration or output law. Shapes reaches CDT through its existing lexical
consumers, so the combined native callers were checked rather than assuming that
different source files establish independence.

An isolated temporary checkout under canonical .worktrees/697-integration-qualification
was created at the exact PR head, then merged with current main without committing.
git write-tree confirms the candidate above. integration-native.log exits0 with
all41 selected controls passing: seven CDT structured-error cases, twelve SHACL
catalog-reference cases, twelve original change-path allocation/report controls,
three counted sharing/short-circuit controls and seven original catalog controls.
No original ceiling or report golden changed. This temporary checkout contains
neither the experimental698 request types nor any unrelated source.

The complete native CDT221-case result, full strict workspace/caller compilation,
normal hooks and hosted checks remain valid for unchanged branch inputs. The
combined41-case run closes the affected integration gap without restarting CI
solely because the branch is behind. No source synchronization push is needed.

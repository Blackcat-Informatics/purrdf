# Iterative matcher assignment: owner assessment

The general assignment search now runs explicit Enter/NextCandidate frames
through the existing WorkList. A resumed candidate first removes its previous
slot contribution, then tries the next original candidate. A candidate schedules
its resume below the child entry. EXTRA follows all candidates and schedules no
resume because it is the final alternative. A successful leaf returns with the
winning counts and assignment intact. Budget failure retains the same error and
partial local state as the former early-return path; it publishes no assignment.

The original budget is decremented only by Enter, including the terminal count
check. Resuming/undoing a choice consumes no new unit. The exact test has two
failed admission budgets (1/2) and succeeds on budget 3 with slot 1 and the same
final counts. The EXTRA test also consumes exactly three entries and retains the
unassigned arc. Existing interval and repeated-predicate controls pass, six cases
in total. The actual public validator accepts a 10,000-arc repeated-predicate
graph on 256 KiB after a required late backtrack. Neither the recursive search
nor a second alternative search remains.

Strict all-target compilation passes. Corpus, portable compilation, helper and
normal publication results are tracked in validation.md. No performance timing,
new runtime dependency, public type, budget policy, corpus or expected verdict
is introduced. This closes only arc-assignment call-stack growth. Recursive tree
compilation/count/action walks, label/value recursion and AST destruction remain
unqualified. The existing budget error channel is unchanged; full native/host
operational-failure acceptance is not claimed. This is an owner assessment, not
independent review or full issue completion.

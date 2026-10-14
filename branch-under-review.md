# Branch under review — issue #545

Branch `paudley/545-complete-evaluator-acceptance` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `88dfc2b7c` Restore entered evaluator boundaries after callback unwind  _2026-10-14_

## Files changed

```
crates/sparql-eval/src/eval.rs | 161 +++++++++++++++++++++++++++++------------
 1 file changed, 114 insertions(+), 47 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.


# Hosted matched campaign dispatch

Root dispatched the approved pushed branch using the existing CI workflow:

```sh
gh workflow run ci.yaml --repo Blackcat-Informatics/purrdf \
  --ref paudley/308-reduce-native-ci-test-compilation-and-c \
  -f native_profile=true -f simd_projection=false
```

The command completed successfully (actual session24484 exit0), returning
[run37738739777](https://github.com/Blackcat-Informatics/purrdf/actions/runs/37738739777).
Full run metadata is retained in `T4-hosted-run-37738739777.json`.
Source prerequisites and exact complete arm/coverage/comparison constraints are
in `T4-preparation.md`. No earlier branch run existed at admission, so this
dispatch did not supersede another experiment through workflow concurrency.

This is actual experiment launch, not accepted measurements. Require all twelve
cases, their twenty-four cold/warm executions, current-attempt admission/uploads,
mandatory checks and actual matched comparisons before numerical conclusions.
Retain failures/refusals and investigate their actual cause; do not relax matching
or borrow earlier attempts. Avoid a new push/dispatch on this branch while the
experiment runs. Local C attribution and source/config invalidations still await
their separately admitted local build lane; hosted runners do not consume the
running glossary qualifier's local lane.

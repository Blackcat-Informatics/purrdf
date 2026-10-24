# Issue #704: Remove panicking canonicalization APIs and propagate native refusals

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Result

Remove the public trusted-caller canonicalization APIs that convert native RDFC
refusals into panics. Keep one fallible production implementation and propagate
its original typed failures through every in-repository caller. This is a v4 API
cleanup: remove obsolete entry points rather than adding compatibility shims or
deprecation staging.

This is one complete behavior extracted from #489. Caller-selected/removable
canonical-search budgets and exact configuration/work identities, dataset-state
search options, heap-framed UDF evaluation and explicit Datalog logical defaults
remain mandatory in that parent. This child does not close or replace #489.

## Implementation boundary

The existing try_canonicalize family already preserves native reserved-vocabulary,
search-budget, stop and source-read errors. Remove the parallel panicking wrapper
family and CanonState::run, route callers directly through those fallible homes,
and change producer signatures/outer typed errors wherever propagation requires
it. Cover dataset/view/graph/digest entry points and consumers that mint content
or proof identities. Never replace an input-triggered failure with expect,
unwrap, a formatted String, a fabricated digest or partial success in shipping
code. Successful canonical bytes, labels, hash selection and admission priority
remain unchanged.

## Acceptance

- Actual reserved-vocabulary and symmetric search-budget refusals are returned as
  typed values by every applicable remaining public canonicalization path.
- Production consumers retain the original refusal through their native error
  chain; healthy neighbors preserve exact canonical output and identity bytes.
- Existing RDFC/profile, shared/faulting views, reconstruction/certification,
  reasoning/proof and actual host controls qualify affected callers. Ordinary
  semantics run natively; release WASM builds prove portability.
- Remove obsolete public exports/docs and update all in-repository callers.
  No input-driven panic wrapper, silent fallback or compatibility layer remains.
- Strict workspace/real-caller compilation, required normal hooks and gates,
  hosted checks and independent current-head review qualify ghprsq integration;
  verify archive/closure and supported cleanup.

Budget four hours of active engineering/local qualification, hosted waits
excluded. No upstream dependency or external repository work is authorized.


## Comments (0)


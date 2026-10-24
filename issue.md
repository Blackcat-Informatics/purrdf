# Issue #702: Remove transient Columnar metadata allocations and decoded blob copies

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Result

Remove successful-path Columnar metadata allocations and transfer decoded blob
payloads into their final content owner without copying the payload.

This is the independently useful allocation component extracted from #688 under
the approved optimized queue. The complete typed grammar-error migration remains
in #688; the full shipping error contract remains in #558. Neither parent closes
with this delivery.

## Existing implementation

Commit bb07611a9 changes three files: borrow expected metadata/encoding values,
move parsed metadata keys, move checked blob payloads into ContentStore, and build
the fixed five-table output directly in its array. Transfer only this committed
behavior onto current main. Preserve the donor's unfinished grammar and later
writer changes.

## Acceptance

- Metadata validation covers all five tables and both compression modes with
  zero validation allocations and requested bytes.
- Blob reconstruction at 1 KiB and 1 MiB retains the original pointer and complete
  payload, with allocation traffic independent of payload length.
- Actual malformed/checksum and structured-cause controls preserve original
  ownership, first faults and presentation.
- All ten frozen file identities remain unchanged. Native library/integration,
  strict affected compilation, required gates and release WASM portability pass.
  General semantics are tested natively.
- Normal signed hooks, current-head independent hosted review and complete CI
  qualify ghprsq integration. Verify archive/closure and supported cleanup.

No upstream dependency or external repository work is authorized. API
compatibility is not a constraint for v4. Budget four hours of active work,
including local verification; hosted CI/review waits are excluded.


## Comments (0)


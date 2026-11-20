# Issue #1404: Execution receipt binding takes a view that has the content-identity capability

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

# Execution receipt binding takes a view that has the content-identity capability

## Requirement

A receipt requested from a view without `purrdf_core::ContentIdentified` is refused by the
type system at the public boundary, not at a closure inside it (requirement source: 1343).
Every signature that has to change changes before 5.0, with no compatibility alias.

`purrdf_core::ContentIdentified` (`crates/rdf-core/src/dataset_view.rs:1891`) is
`trait ContentIdentified: DatasetView { type IdentityError; fn content_identity(&self) -> Result<Digest32, Self::IdentityError>; }`.
A view declares the capability by implementing it; a view that never can certify its content
has no impl and therefore no method.

## Baseline status

The bind entry points do not exist in the `rust-v4.0.2` tree (`git grep` for
`bind_execution_receipt` and `execution_receipt_bind` at that tag finds nothing), so no v4
signature, symbol or package export is being replaced. Changing them is not a break of the
v4 compatibility baseline in `docs/MIGRATION-4.0.md` ("Compatibility after 4.0"); it corrects
the 5.0 surface before it ships. The C ABI is the one surface with a fixed layout, and its
entry point already takes a dataset handle (see below), so it needs no change.

## 1. Signatures that change

### Rust: `purrdf-validate`

`crates/validate/src/query.rs`

| Item | Line | Current | New |
|---|---|---|---|
| `bind_execution_receipt` | 395 | `fn bind_execution_receipt(receipt: &str, content: impl FnOnce() -> Result<Digest32, StorageError>) -> Result<String, ReceiptBindError>` | `fn bind_execution_receipt<V>(receipt: &str, view: &V) -> Result<String, ReceiptBindError> where V: ContentIdentified + ?Sized, V::IdentityError: std::error::Error + Send + Sync + 'static`. The body calls `view.content_identity()` at the point where it calls `content()` today, so a refused receipt still hashes nothing. |
| `ReceiptBindError::Identity` | 340 (variant), 357, 367, 379 (Display, source and visit arms) | `Identity(StorageError)` | `Identity(Box<dyn std::error::Error + Send + Sync>)`. The type matches `CliError::ContentIdentityFailed` in `crates/cli/src/query.rs:226`. The structured-error arm stays `Cause(error)`. The enum is `#[non_exhaustive]` and new in 5.0. |
| `bind_execution_receipt_hex` | 454 | `fn bind_execution_receipt_hex(receipt: &str, content_identity: &str) -> Result<String, ReceiptBindError>` | Removed. It takes a hex string and no view, which is the same defect in another form. Its two hosts (Python, wasm) bind through a store or dataset (below). |
| `ReceiptBindError::ContentIdentity` | 336 (variant), 353, 368, 377 | 64-hex text refusal | Removed with `bind_execution_receipt_hex`; nothing else produces it. |

Unchanged on purpose: `execution_receipt_json_with_sources` (316), `execution_receipt_value_with_sources`
(295) and `rule_execution_receipt_value_with_sources` (519) take an `ExecutionSources` or `RuleSources`
value built from a `Digest32`. They receive an identity value, not a closure, and the caller
that holds a view obtains the digest from `ContentIdentified` first (`crates/cli/src/query.rs:225`).
`QueryCoordinates::execution_identity(&ExecutionSources)` is the same shape. These are the
low-level value API; the view-taking entry point is the one hosts route through.

### Umbrella re-export

`crates/purrdf/src/lib.rs:352` is `pub use purrdf_validate::*;` inside `purrdf::validate`, so
the new signature and the removal propagate with no edit there. `purrdf::validate::query` is
reached the same way. No other `purrdf` file names these items.

### C ABI: `crates/rdf-capi`

`purrdf_execution_receipt_bind(const PurrdfDataset *dataset, const PurrdfBuffer *receipt, PurrdfBuffer **out_receipt, PurrdfError **out_error)`
(`crates/rdf-capi/src/query.rs:326`, `include/purrdf.h:2974`, `tests/abi_signatures.snapshot:63`)
already takes the dataset handle, and a `PurrdfDataset` wraps a resident `RdfDataset`, which
implements `ContentIdentified`. The C layout, the symbol and the header do not change, and
no new entry point is needed. Only the Rust body changes: lines 346-348 become
`purrdf_validate::query::bind_execution_receipt(text, PurrdfDataset::dataset(dataset))`
(a `&RdfDataset`). The `Identity` error maps to `PURRDF_STATUS_QUERY_ERROR` as today
(line 351-352 of the match). The header comment and the ABI snapshot stay byte-identical.

### Python: `bindings/python`

| Item | Location | Current | New |
|---|---|---|---|
| `purrdf.bind_execution_receipt` (free function) | `src/py_store/query.rs:1200`, registered `src/py_store.rs:137`, stub `python/src/purrdf/__init__.pyi:1166` (comment 1163-1165), reference `docs/book/src/reference/python-api-signatures.md:768` | `bind_execution_receipt(receipt_json: str, content_identity: str) -> str` | Removed. |
| `QuadStore.bind_execution_receipt` | new, beside `content_identity` at `src/py_store/quad_store.rs:950`; stub beside `__init__.pyi:953`; reference beside `python-api-signatures.md:677` | none | `def bind_execution_receipt(self, receipt_json: str) -> str`. It freezes the store once through `freeze_snapshot` (line 1078), then calls `bind_execution_receipt(text, &*frozen)` under `py.detach`. It raises `ValueError` for the same failures as today, with the identity refusal reported as `content identity refused: ...`. |

Binding through the store removes the separate `content_identity()` round trip, so the
identity and the receipt bind come from one frozen snapshot.

### JavaScript and WebAssembly: `crates/rdf-wasm`

| Item | Location | Current | New |
|---|---|---|---|
| `bindExecutionReceipt` (free function) | `src/query.rs:253` (doc 239-252), re-export `src/lib.rs:141`, `js/index.mjs:64` and `:1715`, `js/index.d.ts:391` (doc 384-390) | `bindExecutionReceipt(receiptJson: string, contentIdentity: string): string` | Removed. |
| `Dataset#bindExecutionReceipt` | new, beside `content_identity` at `src/dataset.rs:866`; typing beside `js/index.d.ts:878` | none | `bindExecutionReceipt(receiptJson: string): string`. It freezes once, then binds against the frozen dataset. Error codes stay: `purrdf-wasm-receipt` for `ReceiptBindError::Refused`, `purrdf-wasm-options` for malformed receipts, and the `Identity` refusal keeps the code `diag_to_err` gives `contentIdentity()` today. |

The `ExecutionReceipt` type doc at `js/index.d.ts:366-380` names `{@link bindExecutionReceipt}`
and moves to `{@link Dataset#bindExecutionReceipt}`. The `contentIdentity()` doc at
`js/index.d.ts:876-877` and `src/dataset.rs:857` repoints the same way.

### Docs that quote the signatures

- `docs/book/src/sparql/querying.md:2356-2359` names `purrdf.bind_execution_receipt` and the wasm function.
- `docs/WASM_TESTING.md:151` names `bindExecutionReceipt` crossing the boundary.
- `docs/MIGRATION-5.0.md:222` and `CHANGELOG.md:214-219` (see section 3).
- `crates/sparql-eval/README.md:42-45` describes the capability but not the bind functions; read it for consistency and leave it unless it names them (it does not today).

### Other closure-for-capability entry points

Searched `impl Fn`/`FnOnce`/`FnMut` parameters returning `Digest32`, an identity or a snapshot in
`crates` and `bindings`, and every `ExecutionSources::new`, `RuleSources::new` and
`ContentIdentified` caller. The only public closure that supplies a content identity is
`bind_execution_receipt`. The other hits are not the pattern: `sealed_identity` (`crates/rdf-core/src/dataset_view.rs:1907`)
is a private `pub(crate)` cache seal inside the capability's own implementation, and
`crates/geo-kernel/src/binding.rs:1642` takes a per-reference identity function for geographic
bindings, which no view type carries. Rule receipts have no bind entry point at all
(`rule_execution_receipt_value_with_sources` takes `RuleSources` values). None found beyond the one.

## 2. Callers inside the repository

Rust production
1. `crates/rdf-capi/src/query.rs:346` (closure over the dataset; becomes the view argument).
2. `crates/validate/src/query.rs:460` (`bind_execution_receipt_hex` calls `bind_execution_receipt`; removed with it).
3. `bindings/python/src/py_store/query.rs:1204` (`bind_execution_receipt_hex`; replaced by the store method).
4. `crates/rdf-wasm/src/query.rs:258` (`bind_execution_receipt_hex`; replaced by the dataset method).
5. `crates/rdf-wasm/src/lib.rs:141` (re-export of the free function).
6. `bindings/python/src/py_store.rs:137` (registration of the free function).

Rust tests (all in `crates/validate/src/query.rs`)
7. Line 566 `bind_execution_receipt(&receipt, || Ok(content))` (a fixed digest; no view).
8. Line 570 `bind_execution_receipt_hex(&receipt, &content.to_string())`.
9. Line 578 the `refused` closure calling `bind_execution_receipt(text, unhashed)`.
10. Line 601 `bind_execution_receipt_hex(&receipt, "not hex")`.
11. Line 629 `bind_execution_receipt(&receipt, || Err(StorageError::AllocationFailed))`.
12. Line 637 `bind_execution_receipt(&receipt, || ContentIdentified::content_identity(&dataset))`.

Binding tests
13. `crates/rdf-capi/tests/abi.rs:2142-2199` (`bind_receipt` helper and `a_query_receipt_is_bound_to_its_dataset_only_on_request`); the C call is unchanged, so this file needs no edit. Counted because it exercises the changed Rust body.
14. `bindings/python/tests/test_query_coordinates.py:66` (calls `purrdf.bind_execution_receipt(json, store.content_identity())` twice, including the `not-started` refusal).
15. `crates/rdf-wasm/js/tests/sync-coordinates.test.mjs:37` (imports `bindExecutionReceipt` on line 7; calls it twice).

Docs and stubs that quote it (not callers, edited in the same change): `bindings/python/python/src/purrdf/__init__.pyi:1163-1166`,
`docs/book/src/reference/python-api-signatures.md:768`, `crates/rdf-wasm/js/index.d.ts:366-391,876-878`,
`crates/rdf-wasm/js/index.mjs:64,1715`, `docs/book/src/sparql/querying.md:2356-2359`, `docs/WASM_TESTING.md:151`.

No bench, example or `docs/book` code block calls the function. `crates/cli` does not use it: the
CLI binds through `execution_receipt_json_with_sources` after calling `ContentIdentified` itself
(`crates/cli/src/query.rs:225-231`) and is unchanged.

Count: 15 callers (6 production, 6 Rust tests, 3 binding tests; the ABI test is unchanged).

## 3. Tests required

At the changed Rust boundary, in `crates/validate/src/query.rs` tests:

- `a_receipt_from_a_view_that_cannot_certify_its_content_is_a_typed_refusal` (line ~616) is rewritten, not deleted.
  The closure that returns `StorageError::AllocationFailed` is replaced by a small test view
  whose `ContentIdentified::content_identity` returns a typed `IdentityError` (a real
  `DatasetView` wrapper over a frozen dataset, not a mock that asserts a call sequence). The
  case asserts `ReceiptBindError::Identity` carries that cause (via `source()`), that the bound
  output is absent, and, beside it, that a plain `RdfDataset` binds the same receipt and the
  result contains `contentIdentity` and `executionIdentity`. That pair is the AGENTS.md
  refusal-plus-valid-neighbour rule at the new boundary.
- The type-system claim itself needs a compile-fail check: a `DatasetView` that does not implement
  `ContentIdentified` cannot be passed to `bind_execution_receipt`. Put it as a `compile_fail`
  doctest on the function so it runs with the doctests. Its valid neighbour is the doctest
  example that binds an `RdfDataset`.
- The other tests at 566-601 keep their intent and change their input: the native-equality assertion
  (566, 570) binds a dataset whose `content_identity()` equals the digest the native receipt used,
  built from the same frozen dataset (`ExecutionSources::new(dataset.content_identity())`); the
  hash-nothing refusals (578) use a view whose `content_identity` panics; the "not hex" case
  (601) is deleted with its variant.

Binding tests:
- Python (`bindings/python/tests/test_query_coordinates.py:66`): call `store.bind_execution_receipt(receipt_json)`; keep the
  `not-started` `ValueError` refusal and the bound-equality assertions.
- JavaScript (`crates/rdf-wasm/js/tests/sync-coordinates.test.mjs:7,37`): `data.bindExecutionReceipt(JSON.stringify(receipt))`;
  keep the `purrdf-wasm-receipt` code assertion.
- C: no new test. The refusal cases at `abi.rs:2196-2199` stay.
- Neither Python nor wasm can hold a view without the capability, so the type-level refusal is a Rust-only
  claim; their refusals stay at malformed and not-started receipts.

Docs:
- `docs/MIGRATION-5.0.md:222`: the "Execution receipts" row names `bind_execution_receipt_hex`, Python `bind_execution_receipt`
  and JavaScript `bindExecutionReceipt`. Rewrite it to name `bind_execution_receipt(receipt, &view)`, `QuadStore.bind_execution_receipt`
  and `Dataset#bindExecutionReceipt`, and add the migration line: pass the view the query read.
- `CHANGELOG.md:214-219` (the Unreleased "no query computes a content identity" entry): replace the
  names `bind_execution_receipt_hex`, `purrdf.bind_execution_receipt` and `bindExecutionReceipt` with the new
  ones, and state that a view without `ContentIdentified` cannot be passed.
- `docs/book/src/sparql/querying.md:2356-2359`, `docs/WASM_TESTING.md:151` and the two Python references listed in section 1.

## 4. Size and risk

About 17 files: validate (`query.rs`), capi (`src/query.rs`), Python (`py_store/query.rs`, `py_store.rs`, `quad_store.rs`, `.pyi`,
one test), wasm (`src/query.rs`, `src/lib.rs`, `src/dataset.rs`, `index.mjs`, `index.d.ts`, one test), and docs
(`MIGRATION-5.0.md`, `CHANGELOG.md`, `querying.md`, `python-api-signatures.md`, `WASM_TESTING.md`), with 15 callers.
The riskiest part is the `ReceiptBindError::Identity` payload change to a boxed error and the generic bound on
`IdentityError`, because the structured-error visitor and the three host status/code mappings depend on it; second is the
wasm `Dataset` method, since wasm-bindgen cannot take a trait object and must freeze inside the dataset.

Compile-time-only changes: the `bind_execution_receipt` signature and its callers in `rdf-capi` and the Rust tests,
the removal of `bind_execution_receipt_hex` and `ReceiptBindError::ContentIdentity`, and the C ABI (no change at all).
Behaviour changes: Python and JavaScript no longer accept a caller-supplied hex identity (a string that names no
view can no longer be bound), and they bind from one frozen snapshot of the store or dataset instead of a
separately computed identity; the `Identity` refusal now carries the view's own error type. Values of the bound
identities are unchanged.


## Comments (0)


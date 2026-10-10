# Evaluator Clippy correction 5

Status: complete Stage proposal for the twelve-file assignment. Compilation, Clippy, runtime, and the full gate are NOT RUN by this helper.

`evaluator-clippy-correction-5.patch` is an ordinary unified diff against the current shipping source, with matching postimages and source/postimage SHA-256 identities. It covers all 123 captured diagnostic records in `eval`, `expr`, `binop`, `deferred_exists`, `enf`, `substitute`, `remote`, `remote_http`, `service_endpoints`, `xpath_regex`, `parallel`, and `scratch`. It contains no other source home. Numeric's integrated `xsd_of_term` cache hit uses `cached.into_value()` unchanged, and is absent from this diff.

The meaningful ownership details are:

- Memo/cache mutex guards are released after their actual last use. A guard borrowed by native `Memory` remains alive until the memory-consuming traversal/copy/box operation has completed; it is not dropped at the constructor's apparent last textual guard use.
- Cast whitespace uses one inline `(String, WorkspaceAllocation)` carrier. Native parsing borrows the string, and tuple destruction destroys the payload before its original grant, including on early failure.
- Native unary scalar conversion and response ingestion borrow their existing immutable owner. Both actual unary callers and both response-ingestion callers are migrated; no payload or grant is extracted.
- `InternValueHooks` groups four existing scratch identity/ownership callbacks in a stack-only record at the original store-once home. All three actual calls retain the same callback laws and ordering. Lookup and admission stay distinct; there is no new allocation, backend lookup, or memo.
- The two inline-tree `large_enum_variant` expectations and five original-owner `result_large_err` expectations are attached to the precise emitting enum/function. Reasons name the allocation and lifetime law: replacing these with fresh boxes would change the admitted layout or allocate after physical refusal. No blanket allow is added.
- Resident probe wrappers explicitly destroy consumed probes after the original rewrite. Debug assertions inspect the admitted iterator's exact forwarded size hint without consuming a body. Their existing APIs and production/reference laws remain unchanged.
- Remaining changes are exact borrowed iteration, final-use schema moves, simple redundant-borrow/closure fixes, unit semicolons/patterns, raw pointer identity spelling, and doc relocation to the actual native function/owner. No assertion, ceiling, physical error code, fixture count, or semantic result was weakened.

Evidence: all twelve Stage postimages pass `rustfmt --edition 2024 --config skip_children=true`; all twelve ordinary hunks pass `patch --batch --dry-run -p1`; source identity checks pass after the agreed numeric cache-hit integration. These establish syntax and applicability only. The sole writer must qualify the complete integrated correction group through the failed-step Clippy/build lane and required runtime/full gates.

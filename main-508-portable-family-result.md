# Actual #508/#499 portable combination

The original `main-508-portable-family-1.sh` completed **PASS**, terminal exit0,
original unified session50582. It used the existing private
`/opt/purrdf-schema-499-qualification/run` lane, four compiler/test jobs,
16GiB MemoryMax and zero swap. The complete actual log is
`main-508-portable-family-1.log`.

The unchanged staged combination tree is
`a222610de483e8c0fe0845ca362a4d5dbc5f50f7`, with original parents
`b7195b537f535b688c77c8dbd3ee9b97246e198c` and
`a77b743bcc6fad89d88581ef9cb5a626a326834f`. No working-tree source delta or
unmerged entry remained after qualification; both whitespace checks passed.

- Actual wasm32 XSD exact-arithmetic target: all3 cases PASS. Frozen digest
  `7d6b1f15734f09aa`, corpus365276, and independent oracle/hand answers unchanged.
- Actual wasm32 SPARQL numeric target: both2 cases PASS. Frozen digest
  `da3900a93723c4ac`, corpus2727, and independent hand answers unchanged.
- Fresh original `make wasm-pkg`: PASS. The complete release compilation
  finished in6m46s. Original `wasm-opt -Oz`, the suspend/run/poison post-link
  validation, and actual247496-SIMD-opcode verification all passed.
- Actual public package `node --test tests/entail.test.mjs`: all43 cases PASS,
  no failure/cancellation/skip/todo. The target runs the committed proof and
  tri-host golden artifacts on WASM, verifies byte identity, independently
  checks recorded proofs, rejects unrecorded answers and inconsistent inputs,
  preserves typed budget/refusal and checks every public reasoning service.

The complete affected native family3 is independently terminal PASS in
`main-508-native-family-result.md` (all1739 native/C ABI cases, all70 freshly
rebuilt Python reasoning cases and unchanged strict/helper gates). Historical
family1/2 failures remain failed records. This is actual combination evidence;
no historical full gate is relabeled as a new execution and no blanket full
gate was restarted solely for main advancement.

The parent read the actual terminal native/portable logs and updated the existing
`reviews/main-508-resolution-review.md` to PASS for this source/affected
integration. Normal signed synchronization hooks/push, fresh hosted acceptance
and protected candidate/integration remain pending.
No issue closure is claimed.

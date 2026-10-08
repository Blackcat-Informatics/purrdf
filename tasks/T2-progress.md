Task2 completed and independently reviewed. Commit4db9f7947 passed normal hooks.

Both Make callers, workspace CI and the real staged pre-commit hook now use the
single native glossary implementation. The717-line obsolete Python gate was
removed; rendered-book tooling and its shared PO reader remain. Docs CI includes
helper source changes in its path trigger and installs Rust before execution.

Focused qualification passed two actual caller/parity tests, ten native tests,
strict helper all-target clippy, the full69-row/4,181-unit/1,716-control scan,
parity/layer/toolchain/shard/non-Rust/format gates and actual metadata regeneration.
Real normal-hook controls refused poisoned staged text with clean working text,
then accepted clean staged text with poisoned working text. Exact real-index and
catalogue byte preservation was asserted. The initial unstaged-index metadata
failure is retained; the actual isolated settled-index rerun passed with no
generated differences.

Task3 still owns the settled full suite, actual complete i18n/render acceptance,
completion audit, PR, hosted checks and integration. None is inferred from the
native scan or static caller wiring.

# Current-source Python boundary route — preparation only

Status: READY FOR ROOT ADMISSION; no wheel, install or Python test executed in this preparation. Current source tree d632307fb725304ed864967d5234cdab35bc5c99; owning Rust/interface gates have separately passed actualsession13414. The prepared existing Stage driver `python` phase now uses one fresh production wheel and exact installed artifact, avoiding uv-run editable resynchronization.

The prepared source-bound driver records compiler/PATH/profile environment, tracked source/ref/index hashes and a fresh phase directory before commands. Same active SDK/private build/target/tmp, Cargo8 and scope64GiB/Swap0 should be retained by root's later admission. Each argv, complete output and terminal is captured without overwrite.

1. `uv sync --project bindings/python --locked --group dev --no-install-project` establishes the existing locked test dependencies without installing an editable project. No source/lock update is permitted; final source hash gate detects any mutation.
2. `uv build --no-create-gitignore --wheel --project bindings/python --out-dir "$logs/wheel" --config-setting 'maturin.build-args=--locked --jobs 8'` invokes the existing production PEP517 backend. Exactly one new wheel is required; missing or ambiguous output refuses.
3. `uv pip install --python bindings/python/.venv/bin/python --no-deps --reinstall "$wheel"` installs only that wheel into this worktree's environment, not a global environment. No uv-run invocation follows it.
4. Actual interpreter `bindings/python/.venv/bin/python -I` executes the Stage identity probe using `importlib.import_module("purrdf.purrdf_native")`, avoiding the already demonstrated package-shim alias trap. It requires isolated mode, expected venv prefix and native module inside that venv; exactly one wheel native entry must match its selected path. Every installed purrdf package entry is byte-equal to the wheel, with SHA256/size/path receipts.
5. The same `python -I -m pytest` collects only `test_contextual_mappings.py` and `test_solution_row_protocol.py`: require exactly28 (15 retained +13 new). It then runs those exact files, requiring28 passed and no failure/skip/xfail/xpass. Fixed queries are solely real-row-wrapper ingress, not a new semantic oracle/vendor campaign.
6. Wheel/input/probe hashes and all installed package bytes are revalidated after tests; identity readbacks must byte-match. Final tracked-source/index manifest must also match. Module-selection/stale-wheel/source mutation cannot be a silent success.

The identity probe is Stage-only Python because isolated Python imports/interpreter module selection require the actual Python runtime; it does not add shipping tooling or dependencies. Existing protocol tests carry their concrete Why-not-Rust justification. There is no timing/latency or benchmark acceptance here, and no Python execution until explicit root admission.

The current main2eb exact baseline materialization for the separate untimed native-cost phase is available per root; this Python preparation does not execute or qualify that phase, owning PO generation/glossary/render, or hosted acceptance.

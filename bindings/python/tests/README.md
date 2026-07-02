<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf Python test suite

Run from `bindings/python/`:

```bash
uv run maturin develop      # (re)build the native module after any Rust change
uv run pytest tests         # or: make pytest   (from the repo root)
```

## Layout

| File / dir | Purpose |
|---|---|
| `conftest.py` | Shared fixtures (`compat`, `oracle`) + the xfail-ledger XPASS hook. |
| `xfail_ledger.toml` | `node_id → reason` for tests expected to fail against the shim. Applied as **strict** xfails; the ledger only shrinks (AGENTS.md §2 discipline). |
| `test_compat_parity.py` | Differential suite: real `rdflib` (oracle) vs `purrdf.compat.rdflib` (shim), locking in current behavior. |

## Two rdflib names, one rule: never let the shadow shadow the oracle

- The `dev` dependency group installs the **real** `rdflib` (currently 7.6.x). It
  is the differential *oracle* the parity suite compares against, and the target
  of the #9 "rdflib's own test suite" gate.
- Task 7 ships a top-level **`rdflib` shadow** package that re-exports
  `purrdf.compat.rdflib`. It claims the same import name.
- These MUST NOT co-inhabit one environment — a differential test whose "reference"
  is secretly the implementation under test proves nothing.
  - The oracle env (this `dev` group) never installs the shadow.
  - The shadow is exercised in isolation: a separate venv, or via explicit
    `sys.modules['rdflib'] = purrdf.compat.rdflib` injection inside a dedicated
    test module — not by installing the top-level package here.

## Gates

`make pytest` (and its CI job) is **separate** from `make check`. `make check` is
the Rust gate (fmt/clippy/build/tests/hygiene) and must stay runnable without a
Python toolchain; the Python suite is its own gate.

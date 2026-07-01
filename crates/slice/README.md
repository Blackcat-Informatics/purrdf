<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-slice

`purrdf-slice` is the native slice catalog for the repository. It discovers slice
anatomy from `slices/<group>/<name>/`, reads the manifest vocabulary, inventories
slice-local artifacts, and exposes ownership/dependency facts to the validation,
documentation, mapping, and pipeline lanes.

## Module Map

| Module | Responsibility |
| --- | --- |
| `catalog` / `manifest` | Manifest-based discovery, typed slice metadata, and artifact roles. |
| `artifacts` | Content-addressed artifact inventory and cache-key inputs. |
| `ownership` | Term ownership, dependency edges, extension-dependency rules, and fix suggestions. |
| `mapping_support` | Native support functions for projection/mapping emitters and lints. |
| Python bindings | Exposed from `bindings/python`; this crate stays PyO3-free. |

## Invariants

Slice identity comes from `manifest.ttl`, not the directory name. Core slices may
interlink freely, while extension slices depend only on core. Every declared term
has exactly one owning slice, and generated artifacts are projections of authored
sources rather than editable inputs.

## Local Checks

```bash
cargo test -p purrdf-slice
```

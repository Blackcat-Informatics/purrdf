<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# purrdf-envelope-probe

The capture tool for the small-deployment validation envelope
([`docs/design/micro-hardware-envelope.md`](../../docs/design/micro-hardware-envelope.md)):
one binary that runs a fixed, deterministic workload set — codec round-trips,
governed SPARQL (with a truncation case and its completing neighbor), SHACL,
GTS, and the pack/paged tier — at a named profile's scale, and emits a JSON
report of allocator peaks, retained deltas, OS high-water mark, and governor
evidence.

Never published, never a gate: the memory ceiling is pinned by the harness
around the process (cgroup `MemoryMax`, or a wasm linear-memory maximum), and
wall time is recorded evidence only.

```sh
cargo run -p purrdf-envelope-probe --release -- sbc-64
cargo run -p purrdf-envelope-probe -- smoke          # tiny verification run
```

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

Never published. The PROCESS memory ceiling is still pinned by the harness around
the probe (a cgroup `MemoryMax`, or a wasm linear-memory maximum) and this crate
only measures it; wall time is recorded evidence only and is never a pass
criterion.

One workload is an exception, and it is a deliberate one. `roundtrip` carries a
per-profile `roundtrip_peak_ceiling_bytes`, and the probe exits non-zero when its
allocator peak exceeds it. That figure is an acceptance criterion rather than a
reading — a serialization path that quietly went back to holding whole documents
would otherwise show up as a larger number in a report nobody diffs.

A profile whose envelope has not been measured yet carries no ceiling, and the
report says `"peak_ceiling_bytes": null` rather than passing quietly. That is not
an opt-out: a profile that HAS a number must meet it. Refusing a profile merely
because nobody has measured it would reject a valid run, which is the mirror of
letting a real overrun through.

```sh
cargo run -p purrdf-envelope-probe --release -- sbc-64
cargo run -p purrdf-envelope-probe -- smoke          # tiny verification run
```

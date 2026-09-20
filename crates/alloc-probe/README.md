<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# purrdf-alloc-probe

The workspace's counting allocator, in one place. Tests, benches and examples
across several crates each need the same instrument — a pass-through
`GlobalAlloc` that counts, and a bracket around the region being measured — and
each hand-rolled copy of it was free to drift in what it counted and in which
threads it counted on. This crate is that instrument, once.

Never published, never a runtime dependency: it appears only in
`[dev-dependencies]`, so no release crate and no wasm build ever sees it.

## Using it

`#[global_allocator]` may be declared once per binary, so the registration stays
at the use site; the crate supplies the type and the windows.

```rust,ignore
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

let window = CurrentThreadWindow::open();
do_the_thing();
let measured = window.close();
assert_eq!(measured.allocations, 0);
```

## Two windows, because there are two different measurements

| Window | Counts | Use it for | Failure mode of the other one |
| --- | --- | --- | --- |
| `CurrentThreadWindow` | the opening thread only | `cargo test`, where the harness runs a binary's tests **concurrently** over one allocator | a whole-process window there absorbs a sibling test's traffic, so the same assertion passes or fails by scheduling |
| `WholeProcessWindow` | every thread | code that fans out over `rayon` — SHACL validation above its focus-node threshold, for instance | a per-thread window misses the worker threads entirely and reports a parallel phase as nearly free |

Neither is a default. Picking the wrong one does not error; it returns a
plausible number that means nothing, which is why the names say what they
measure rather than how they are implemented.

## Four quantities, never blended

`Measurement` reports allocation **count**, **requested bytes** (allocator
traffic, including memory freed again inside the window), **retained bytes**
(what the region's result still holds) and **peak working bytes** (the
high-water mark of live bytes). A phase that allocates and frees one large
buffer repeatedly has high traffic and a modest peak; a phase that assembles one
large structure has the reverse. There is deliberately no single combined
figure, because collapsing them erases the only interesting distinction.

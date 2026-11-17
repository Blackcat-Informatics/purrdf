<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-stack

How much stack the running thread has left, for the PurRDF SPARQL stack.

Running out of stack is not an error anywhere: natively the process aborts, and
on `wasm32-unknown-unknown` the shadow stack runs below its floor and traps with
the instance's memory in an unknown state. The SPARQL evaluator
(`purrdf-sparql-eval`) therefore measures the stack actually left — against a
plan's whole height before its recursive passes run, and at every level of a
walk that has no error channel — and refuses, typed, rather than spend the last
`MARGIN_BYTES`. (The
SPARQL parser, `purrdf-sparql-algebra`, does not recurse on a request's nesting
and needs no such check.) This crate is that measurement, in one place:

* `remaining` — the bytes left below the calling frame before the thread's
  stack floor, or `0` while the floor is unknown.
* `try_remaining` — the same measurement, returning the typed `FloorError`
  when the floor cannot be read.
* `is_low` — whether fewer than `MARGIN_BYTES` are left: one thread-local load,
  one subtraction and one comparison on the hot path.
* `replace_floor` — install the floor of a stack a host has switched onto, and
  get back the one it replaces. The PurRDF wasm package installs the linker's
  `__stack_low` when its instance starts and each asynchronous job's own region
  whenever it switches onto it.
* `walk` and `walk_is_low` — a scope for a walk with no error channel: a level
  that finds the stack low stops descending, and the scope discards the
  half-built result and reports the refusal.
* `on_stack` — run a computation on a stack of a stated size: a fresh thread
  natively, and inline under a floor that many bytes down on `wasm32`, where a
  request larger than the stack left is refused with a typed `StackError`.
* `MARGIN_BYTES` — 128 KiB natively, 64 KiB on `wasm32`, with the measurements
  they are derived from in their documentation.

Natively the floor is the thread's stack limit as the operating system reports
it, read through a small platform module in this crate: on Linux, Android and
NetBSD, `pthread_getattr_np`; on FreeBSD and DragonFly BSD,
`pthread_attr_get_np`; on OpenBSD, `pthread_stackseg_np`; on macOS and iOS,
`pthread_get_stackaddr_np` and `pthread_get_stacksize_np`; on Windows,
`GetCurrentThreadStackLimits`. Each is a raw `libc` (or, on Windows,
`windows-sys`) declaration resolved against the target's own C library at link
time — no bundled C source and no build-time C toolchain, so this crate stays a
cross-compilable leaf. A successful read is cached per thread. A failed read is
not cached and is retried by the next check. A thread started by `on_stack`
never reads the operating system: its floor comes from the request.

While the floor is unknown the guards refuse: `remaining` is `0` and `is_low`
is `true`. `try_remaining` returns the typed `FloorError` instead, either
`Unsupported` or `Read`. On a target with no stack-bounds call — a `libc` this
module has no call for, or a target this crate has not been ported to — the
bound can never be read, so on every thread not started by `on_stack` each
guarded call refuses and returns its stack-exhaustion error (the evaluator's
`StackExhausted`, for one), rather than the guard switching off. A thread
started by `on_stack` is still served there, because it takes its floor from the
request.

On `wasm32` the floor is the low end of the shadow stack. That path never links
`libc` or `windows-sys` at all, and builds for `wasm32-unknown-unknown` like
every other release crate in the workspace.

The crate denies `unsafe` code everywhere but the platform module the native
reads above live in, where each block carries its own safety argument.

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)

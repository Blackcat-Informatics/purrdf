<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Native table source provenance

This engine derives from the crates.io `hashbrown` release `0.17.1`, offered
under MIT OR Apache-2.0. The original license texts accompany this source.
`UPSTREAM-SHA256.json` records the published crate checksum and the exact
original source-file hashes. The local source was acquired from the installed
Rust toolchain vendor directory, whose package checksum matches the pinned
registry release.

The upstream no-default-feature allocator/integration configuration is fixed
in source: no random hasher, allocator extension, optional integration or nightly
specialization. The existing workspace `FixedState` supplies the native map/set
default and resident constructors, keeping their documentation executable without
a default-hasher package or feature. Actual callers retain their explicit fixed
hash and framing. Global allocation and the existing target-selected group
kernels retain the native implementation. Module paths and documentation
examples refer to the shared Core home. Upstream tests requiring external test
dependencies are excluded; native consumer and refusal cases own verification.

The inherited source is modified, not a verbatim upstream checkout. Each
inherited Rust file retains the original copyright and dual-license grant and
points to `NOTICE`, which explicitly attributes local modifications under that
same grant. Newly authored wrappers and regression cases retain their own
first-party grant. The complete original licenses and local modification notice
ship in generated Core, Python, WASM and C recipient notices through
`license-inventory.toml`; the inherited source is never relabeled as owned code.

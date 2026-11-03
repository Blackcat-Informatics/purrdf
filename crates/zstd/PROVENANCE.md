<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Codec provenance

This crate imports the reachable library implementation from the published
`structured-zstd` 0.0.49 archive. The archive SHA-256 is
`392eea135985654a313cdfee001e93031bacd96a3693a5dc4b0d13793e40b27a`.
Its recorded upstream commit is `66342a4b1953c7b6c7e8f111a928915cad426677`,
with package path `zstd`. `UPSTREAM-SHA256.json` records the original Rust files.
The original Apache license is preserved verbatim in `LICENSE`.

The fixed configuration includes `std`, `hash`, `dict_builder`, `kernel_scalar`,
`kernel_sse2`, `kernel_bmi2`, `kernel_avx2`, `kernel_neon`, `kernel_sve` and
`kernel_simd128`. Product feature predicates were specialized to this
configuration; target predicates and actual dispatch remain. Disabled paths,
upstream tests and unreachable modules were excluded from the shipped import.
The manifest is a publishable workspace package with no semantic Cargo features.

Local changes expose the original `GetBitsError` and `BlockType`, add borrowed
structured inspection of native errors, preserve frame-extent debug presentation,
and omit the unused legacy COVER trainer, its reservoir sampler and frequency
estimator. FastCOVER and dictionary finalization share none of those helpers;
the retained trainer computes on the supplied bytes without an RNG dependency.
Compiler qualification fixes, inert statement removal and imported API lint
annotations adapt the code to workspace gates. Imported API documentation and
Debug omissions remain scoped to the inherited source files.

Dictionary authoring now returns typed native refusals instead of constructing
string-bearing I/O errors. Original generated Huffman causes and buffers, FSE
counts, header bounds and selected training parameters remain owned by the
refusal. Source/output I/O errors move unchanged through the existing APIs, and
their inspection uses the hash root's borrowed standard-error implementation.
Training choices and previously successful dictionary bytes are unchanged.
Dictionary Huffman descriptions are checked before writing. An unwritable table
is deterministically rebalanced by doubling its bounded statistics with the first
symbol, then checked again; a remaining failure retains those actual statistics.
This repairs the uniform full-alphabet weight description that previously
panicked instead of reaching dictionary finalization or the native FSE refusal.

Dictionary identities call the workspace's FNV implementation. Native chunk
access preserves the original byte widths and offsets. Constructor and error
conversion wrappers use the shared hash and lexical macros. Encoder history,
overlap-copy loops and prefix-probe composition share one implementation per
operation, retaining the named ISA entries, dispatch and pointer contracts.
The lexical dependency supplies these conversion macros; no external package
or semantic feature is added by this consolidation.

The workspace version and package name identify this maintained provider;
downstream consumers do not depend on a root-only registry patch.

The finite frame-sequence reader reuses the streaming block decoder while
advancing through concatenated and skippable frames. It verifies checksums and
reapplies the declared dictionary without retaining the complete decoded output.

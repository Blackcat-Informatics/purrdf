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

The workspace version and package name identify this maintained provider;
downstream consumers do not depend on a root-only registry patch.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# purrdf-zstd

PurRDF's shared pure Rust Zstandard codec, with frame decoding, encoding and
FastCOVER dictionary training. GTS, Columnar and the CLI use this one codec.

This fixed product includes standard-library I/O, content checksums, dictionary
training and scalar, SSE2, BMI2, AVX2, NEON, SVE and wasm SIMD kernels. Target
predicates and native CPU detection select supported kernels. There are no
Cargo features or optional semantic configurations.

Native decoder errors retain their original fields and causes. Borrowed
`purrdf_hash::error::StructuredError` inspection exposes them without rendering
or cloning diagnostic buffers.

See [PROVENANCE.md](PROVENANCE.md) for the imported source identity and local
changes, and the package's `NOTICE` file for upstream attribution.

## License

Apache License, Version 2.0. See the package's `LICENSE` file.

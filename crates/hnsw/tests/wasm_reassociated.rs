// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The reassociated HNSW index, built, recorded, decoded, verified and searched on
//! `wasm32-unknown-unknown` as well as on the host.**
//!
//! The reassociated index records the dispatch path it was built on (image codes 6 for
//! `wasm-simd128` and 7 for `wasm-scalar` on wasm32) and the [`BuildShape`] of the build
//! that compiled it, and refuses to decode an image recorded on a path this build cannot
//! run. Those claims are about wasm32 as much as about the host, and a claim about wasm32
//! is only evidence when it EXECUTES there. This file executes them on each target it
//! runs on:
//!
//! * the image records the code of the path this build was made for -- derived here from
//!   the build's own `cfg`, never read back from the index -- and a shape equal to
//!   [`BuildShape::here`] that names this architecture (and on wasm32, the `simd128` bit
//!   exactly as built);
//! * the image decodes, its rebuild verifies (`true`), and the decoded image is the same
//!   bytes;
//! * every search -- over the built index and the decoded one, from a stored row and from
//!   an off-matrix vector -- returns exactly the brute-force top-`k` of the reassociated
//!   kNN kernel (`Kernel::distance_reassociated`), rows and distance bits, which is an
//!   oracle computed without the graph;
//! * a rebuild verification over an image one distance bit away is refused by name with
//!   [`HnswError::ArithmeticRebuildDiverged`], not answered `true` (the control that shows
//!   the verification observes the payload); and
//! * an image recorded on the OTHER wasm path is refused by name with
//!   [`HnswError::ArithmeticPathUnavailable`], while the unaltered image beside it decodes.
//!
//! `make wasm-test` runs it twice on wasm32: on the baseline build, whose path is
//! `wasm-scalar`, and on a `+simd128` build, whose path is `wasm-simd128`. Natively it is
//! an ordinary `#[test]` along whatever path the host resolves.
//!
//! ```text
//! cargo test -p purrdf-hnsw --target wasm32-unknown-unknown --test wasm_reassociated
//! ```

#![allow(clippy::doc_markdown, reason = "prose names targets, not items")]

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{Arithmetic, BuildShape, Path, Reassociated, Resolved};
use purrdf_hnsw::level::splitmix64;
use purrdf_hnsw::{HnswError, HnswIndex, Kernel, Params, Ranked, VectorMatrix};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// Rows in the fixture.
const ROWS: usize = 40;
/// Dimensions in the fixture: not a multiple of the sixteen-lane block, so every
/// distance runs a block body and a tail.
const DIMS: usize = 70;
/// Neighbours asked for.
const K: usize = 7;

/// Byte offset of the arithmetic's image code in a canonical image header.
const CODE_AT: usize = 60;
/// Byte offset of the build shape, which follows the code in a reassociated header.
const SHAPE_AT: usize = 64;
/// Byte offset of the first recorded distance: row 0, layer 0, first neighbour.
///
/// The header (80 bytes with a shape), then row 0's record (row, level, pad), its layer-0
/// record (layer, pad, count), then the first neighbour's row.
const FIRST_DISTANCE_AT: usize = 80 + 8 + 4 + 4 + 4 + 4 + 8 + 8;

/// A seeded splitmix64 stream of `len` values in `[-1, 1)`.
fn stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = splitmix64(state);
            ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
        })
        .collect()
}

/// The fixture matrix.
fn matrix() -> VectorMatrix {
    VectorMatrix::new(ROWS, DIMS, stream(ROWS * DIMS, 0x5eed_0344_7a55_0c1a))
        .expect("a valid matrix")
}

/// Parameters whose beam covers every row, so a search over the connected graph returns
/// the true top-`k` and the brute-force oracle is exact rather than a recall floor.
fn params() -> Params {
    Params::new(4, 8, 16, ROWS).expect("valid parameters")
}

/// The metrics the fixture is built under, with the kernel each names.
fn metrics() -> [(DistanceMetric, Kernel); 3] {
    [
        (DistanceMetric::SquaredEuclidean, Kernel::SquaredEuclidean),
        (DistanceMetric::NegativeDot, Kernel::NegativeDot),
        (DistanceMetric::Cosine, Kernel::Cosine),
    ]
}

/// The reassociated path this build was made for, from its `cfg` alone.
///
/// On `x86_64` the path is the widest the processor reports, which no `cfg` names, so the
/// expectation there is the set of `x86_64` paths.
fn expected_paths() -> &'static [Path] {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    return &[Path::WasmSimd128];
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    return &[Path::WasmScalar];
    #[cfg(target_arch = "aarch64")]
    return &[Path::Neon];
    #[cfg(target_arch = "x86_64")]
    return &[Path::Sse2, Path::Avx2Fma, Path::Avx512f];
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "wasm64"
    )))]
    return &[Path::Portable];
}

/// The image code of the reassociated `path`, as the format assigns it: written out
/// rather than asked of the arithmetic, so a code that moved is caught here.
fn code_of(path: Path) -> u32 {
    match path {
        Path::Sse2 => 2,
        Path::Avx2Fma => 3,
        Path::Avx512f => 4,
        Path::Neon => 5,
        Path::WasmSimd128 => 6,
        Path::WasmScalar => 7,
        Path::Portable => 8,
        other => panic!("{other} is not a reassociated path"),
    }
}

/// The wasm paths this build does NOT run, with their codes: the other wasm path on
/// wasm32, and both of them anywhere else.
fn foreign_wasm_codes() -> &'static [(u32, &'static str)] {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    return &[(7, "wasm-scalar")];
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    return &[(6, "wasm-simd128")];
    #[cfg(not(target_arch = "wasm32"))]
    return &[(6, "wasm-simd128"), (7, "wasm-scalar")];
}

/// The reassociated arithmetic this thread resolves, held independently of any index.
fn resolved() -> Resolved<Reassociated> {
    Reassociated::resolve().expect("the default float environment is the IEEE one")
}

/// The recorded image code of `image`.
fn recorded_code(image: &[u8]) -> u32 {
    u32::from_le_bytes(image[CODE_AT..CODE_AT + 4].try_into().expect("four bytes"))
}

/// The recorded build shape of a reassociated `image`.
fn recorded_shape(image: &[u8]) -> BuildShape {
    BuildShape::from_bits(u64::from_le_bytes(
        image[SHAPE_AT..SHAPE_AT + 8]
            .try_into()
            .expect("eight bytes"),
    ))
}

/// The brute-force top-`K` of `query` over every row of `matrix`, under the reassociated
/// kNN kernel along `arithmetic`: the oracle, computed without the graph.
fn oracle(
    matrix: &VectorMatrix,
    kernel: Kernel,
    arithmetic: Resolved<Reassociated>,
    query: &[f64],
) -> Vec<Ranked> {
    let query_norm = arithmetic.exact().norm(query);
    let mut scored: Vec<Ranked> = (0..matrix.rows())
        .map(|row| {
            let values = matrix.row(row);
            let row_norm = arithmetic.exact().norm(values);
            Ranked {
                distance: kernel
                    .distance_reassociated(arithmetic, query, query_norm, values, row_norm)
                    .expect("the fixture is finite"),
                row,
            }
        })
        .collect();
    scored.sort_unstable();
    scored.truncate(K);
    scored
}

/// `offered` as (row, distance bits), the form two answers are compared in.
fn bits(offered: &[Ranked]) -> Vec<(usize, u64)> {
    offered
        .iter()
        .map(|ranked| (ranked.row, ranked.distance.to_bits()))
        .collect()
}

/// Every search `index` answers is the oracle's top-`K`, bit for bit.
fn assert_searches_match_oracle(
    index: &HnswIndex<Reassociated>,
    kernel: Kernel,
    arithmetic: Resolved<Reassociated>,
    label: &str,
) {
    let matrix = matrix();
    for query in 0..ROWS {
        let offered = index.search_rows(query, K).expect("the row search runs");
        assert_eq!(
            bits(&offered),
            bits(&oracle(&matrix, kernel, arithmetic, matrix.row(query))),
            "{label} {kernel:?} from row {query} on {}: the index's answer is not the \
             reassociated kernel's brute-force top-{K}",
            arithmetic.path()
        );
    }
    for seed in [0xA1_u64, 0xB2, 0xC3] {
        let query = stream(DIMS, seed);
        let offered = index
            .search_vector(&query, K)
            .expect("the vector search runs");
        assert_eq!(
            bits(&offered),
            bits(&oracle(&matrix, kernel, arithmetic, &query)),
            "{label} {kernel:?} from vector {seed:#x} on {}",
            arithmetic.path()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_image_records_the_path_and_shape_this_build_was_made_for() {
    let arithmetic = resolved();
    assert!(
        expected_paths().contains(&arithmetic.path()),
        "this build resolves {}, not a path it was made for",
        arithmetic.path()
    );
    let expected_code = code_of(arithmetic.path());
    let here = BuildShape::here();
    assert_eq!(
        here.architecture(),
        Some(std::env::consts::ARCH),
        "this build's shape names its architecture: {here}"
    );
    // On wasm32 the shape is written out from the build's `cfg`, not read from `here`:
    // the wasm32 row with exactly the simd128 bit this build enabled, and never
    // relaxed-simd, which PurRDF does not enable.
    #[cfg(target_arch = "wasm32")]
    assert_eq!(
        here,
        BuildShape::encode(
            "wasm32",
            if cfg!(target_feature = "simd128") {
                &["simd128"]
            } else {
                &[]
            }
        ),
        "the shape records simd128 exactly as the build enabled it: {here}"
    );

    for (metric, kernel) in metrics() {
        let index =
            HnswIndex::build_reassociated(matrix(), &metric, params()).expect("the fixture builds");
        let image = index.canonical_image();
        assert_eq!(
            recorded_code(&image),
            expected_code,
            "{kernel:?}: the image records the code of {}",
            arithmetic.path()
        );
        assert_eq!(
            recorded_shape(&image),
            here,
            "{kernel:?}: the image records this build's shape"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_image_decodes_verifies_and_searches_as_the_kernel_ranks() {
    let arithmetic = resolved();
    for (metric, kernel) in metrics() {
        let built =
            HnswIndex::build_reassociated(matrix(), &metric, params()).expect("the fixture builds");
        assert_searches_match_oracle(&built, kernel, arithmetic, "built");

        let image = built.canonical_image();
        let decoded =
            HnswIndex::decode_reassociated(matrix(), &image).expect("the image decodes here");
        assert_eq!(
            decoded.canonical_image(),
            image,
            "{kernel:?}: decoding and re-encoding is the identity"
        );
        assert!(
            decoded
                .verify_rebuild()
                .expect("the rebuild runs on the recorded path"),
            "{kernel:?}: the image is the rebuild of its input on {}",
            arithmetic.path()
        );
        assert_searches_match_oracle(&decoded, kernel, arithmetic, "decoded");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn a_payload_one_distance_bit_away_does_not_verify() {
    let index =
        HnswIndex::build_reassociated(matrix(), &DistanceMetric::SquaredEuclidean, params())
            .expect("the fixture builds");
    let mut image = index.canonical_image();
    image[FIRST_DISTANCE_AT] ^= 1;
    let tampered = HnswIndex::decode_reassociated(matrix(), &image)
        .expect("a finite distance one bit away still decodes");
    assert_ne!(tampered.canonical_image(), index.canonical_image());
    // A reassociated payload that is not its rebuild is refused by name, never answered
    // `false`: no image records everything that decides its bits.
    let verdict = tampered.verify_rebuild();
    assert!(
        matches!(
            verdict,
            Err(HnswError::ArithmeticRebuildDiverged { recorded, shape })
                if recorded == code_of(resolved().path()) && shape == BuildShape::here()
        ),
        "a payload that is not the rebuild is never verified: {verdict:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn an_image_recorded_on_another_wasm_path_is_refused_by_name() {
    let here = code_of(resolved().path());
    let index =
        HnswIndex::build_reassociated(matrix(), &DistanceMetric::SquaredEuclidean, params())
            .expect("the fixture builds");
    let image = index.canonical_image();
    // The valid neighbour: the unaltered image, which differs from each refused one only
    // in its recorded code, decodes.
    assert!(HnswIndex::decode_reassociated(matrix(), &image).is_ok());

    for &(foreign, name) in foreign_wasm_codes() {
        let mut other = image.clone();
        other[CODE_AT..CODE_AT + 4].copy_from_slice(&foreign.to_le_bytes());
        let refusal = HnswIndex::decode_reassociated(matrix(), &other)
            .err()
            .unwrap_or_else(|| panic!("an image recorded on {name} decoded on {here}"));
        assert!(
            matches!(
                refusal,
                HnswError::ArithmeticPathUnavailable { recorded, available }
                    if recorded == foreign && available == here
            ),
            "{name}: {refusal:?}"
        );
        assert!(
            refusal.to_string().contains(name),
            "the refusal names the path it cannot run: {refusal}"
        );
    }
}

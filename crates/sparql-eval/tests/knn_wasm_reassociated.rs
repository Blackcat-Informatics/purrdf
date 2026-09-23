// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The reassociated kNN kernels, executed on the host and on
//! `wasm32-unknown-unknown`, each held to the error bound of the exact answer.**
//!
//! The reassociated arithmetic promises no bits: its sums may be reassociated and
//! contracted differently on every target, so there is no golden to pin and a lexical
//! comparison like `knn_wasm_determinism`'s would be the wrong test. What it does
//! promise holds on every target, and this file executes that promise on each one it
//! runs on: the entry points exist and resolve there, the path they resolve is the one
//! the build was made for, each distance is within `2·n·ε·Σ|tᵢ|` of the exact one, the
//! bounded form agrees bit for bit with the full one, and an overflow is refused.
//!
//! `make wasm-test` runs it twice on wasm32: on the baseline build, whose path is
//! `wasm-scalar`, and on a `+simd128` build, whose path is `wasm-simd128`. Natively it
//! is an ordinary `#[test]`.
//!
//! ```text
//! cargo test -p purrdf-sparql-eval --target wasm32-unknown-unknown --test knn_wasm_reassociated
//! ```

#![allow(clippy::doc_markdown, reason = "prose names targets, not items")]

use purrdf_core::distance::{Arithmetic, Path};
use purrdf_sparql_eval::knn::{Bound, Bounded, Reassociated, Resolved};
use purrdf_sparql_eval::{Kernel, knn::norm};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// A seeded splitmix64 stream of values in `[-1, 1)`, none of them exactly
/// representable as short decimals, so every product and partial sum rounds.
fn stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
        })
        .collect()
}

/// The reassociated arithmetic on this target.
fn resolved() -> Resolved<Reassociated> {
    Reassociated::resolve().expect("the default float environment is the IEEE one")
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_reassociated_path_is_the_one_this_build_was_made_for() {
    let fast = resolved();
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    assert_eq!(fast.path(), Path::WasmSimd128);
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    assert_eq!(fast.path(), Path::WasmScalar);
    #[cfg(target_arch = "aarch64")]
    assert_eq!(fast.path(), Path::Neon);
    #[cfg(target_arch = "x86_64")]
    assert!(matches!(
        fast.path(),
        Path::Sse2 | Path::Avx2Fma | Path::Avx512f
    ));
    assert!(
        fast.evidence()
            .is_some_and(|text| text.contains(fast.path().name())),
        "the handle names its divergence along its own path"
    );
    assert!(Reassociated::IMAGE_CODES.contains(&fast.image_code()));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_reassociated_distance_is_within_the_error_bound_of_the_exact_one() {
    let fast = resolved();
    for (len, seed) in [(6_usize, 1_u64), (64, 2), (70, 3), (200, 4), (1_024, 5)] {
        let a = stream(len, seed);
        let b = stream(len, seed ^ 0xA5A5);
        let a32: Vec<f32> = a.iter().map(|&value| value as f32).collect();
        let (na, nb) = (norm(&a), norm(&b));
        // Components in [-1, 1) make every term at most 4, so `Σ|tᵢ| ≤ 4·n` and the
        // contract's `2·n·ε·Σ|tᵢ|` is at most `8·n²·ε`.
        let bound = 8.0 * (len * len) as f64 * f64::EPSILON;
        for kernel in [
            Kernel::SquaredEuclidean,
            Kernel::NegativeDot,
            Kernel::Cosine,
        ] {
            let exact = kernel.distance(&a, na, &b, nb).expect("finite");
            let reassociated = kernel
                .distance_reassociated(fast, &a, na, &b, nb)
                .expect("finite");
            assert!(
                (exact - reassociated).abs() <= bound,
                "{kernel:?} at len {len} on {}: exact {exact}, reassociated {reassociated}",
                fast.path()
            );
            assert_eq!(
                kernel.distance_bounded_reassociated(
                    fast,
                    &a,
                    na,
                    &b,
                    nb,
                    Bound::Above(f64::INFINITY)
                ),
                Bounded::Below(reassociated),
                "{kernel:?} at len {len}: the bounded form is the same compilation"
            );
            // A narrow operand runs its own compilation; it is held to the same bound
            // against the exact answer for the same (widened) values.
            let na32 = norm(&a32);
            let exact32 = kernel.distance(&a32, na32, &b, nb).expect("finite");
            let fast32 = kernel
                .distance_reassociated(fast, &a32, na32, &b, nb)
                .expect("finite");
            assert!(
                (exact32 - fast32).abs() <= bound,
                "{kernel:?} f32 at len {len}"
            );
        }
        let full = Kernel::SquaredEuclidean
            .distance_reassociated(fast, &a, na, &b, nb)
            .expect("finite");
        assert_eq!(
            Kernel::SquaredEuclidean.distance_bounded_reassociated(
                fast,
                &a,
                na,
                &b,
                nb,
                Bound::AtOrAbove(full.next_up())
            ),
            Bounded::Below(full),
            "len {len}: a bound one ulp above the distance is not met"
        );
        assert_eq!(
            Kernel::SquaredEuclidean.distance_bounded_reassociated(
                fast,
                &a,
                na,
                &b,
                nb,
                Bound::AtOrAbove(full)
            ),
            Bounded::Beyond,
            "len {len}: the distance itself meets `AtOrAbove`"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn the_reassociated_distance_refuses_an_overflow() {
    let fast = resolved();
    let huge = vec![1e300_f64; 70];
    let negative = vec![-1e300_f64; 70];
    assert_eq!(
        Kernel::NegativeDot.distance_reassociated(fast, &huge, 0.0, &huge, 0.0),
        None
    );
    assert_eq!(
        Kernel::SquaredEuclidean.distance_bounded_reassociated(
            fast,
            &huge,
            0.0,
            &negative,
            0.0,
            Bound::AtOrAbove(1.0)
        ),
        Bounded::NonFinite
    );
    let large = vec![1e150_f64; 2];
    assert!(
        Kernel::SquaredEuclidean
            .distance_reassociated(fast, &large, 0.0, &[0.0_f64, 0.0], 0.0)
            .is_some_and(f64::is_finite),
        "the valid neighbour ranks"
    );
}

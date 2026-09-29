// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CSV field scanner's kernels — the portable oracle, the live dispatch,
//! and every explicit vector kernel this build compiles — against the
//! per-byte reference, at every start alignment 0..64 and length 0..256, and
//! over seeded random inputs.
//!
//! `purrdf_core::csv::scan` carries this same differential as a `#[cfg(test)]`
//! unit test, which runs only when `purrdf-core` itself is tested — never
//! from an integration test's own binary, and never on `wasm32-unknown-unknown`
//! at all, since `#[cfg(test)]` code does not exist there. The doc-hidden
//! `purrdf_core::csv::backend` module exists so this file can run the very
//! same comparison from outside the crate, in this crate's own
//! `cargo test` run and — the point of this file — on wasm32.
//!
//! The target is `harness = false` on `purrdf_testkit`'s runner, so the same
//! named cases run natively under `cargo test` and on `wasm32-unknown-unknown`
//! in Node (`make wasm-test`), baseline and `+simd128`. Only the `+simd128`
//! build compiles `csv::arch::simd128::find`; the baseline row proves the
//! portable kernel and the live dispatch (there, the portable kernel again)
//! still agree with the reference on that target.

use purrdf_core::csv::backend::{Backend, kernels, reference};
use purrdf_hash::Backend as _;
use purrdf_testkit::rng::SplitMix64;

/// The byte sets the reader and writer actually stop at, plus edge cases: the
/// empty set, a set wider than the vector kernels (answered portably even on
/// a target with one), a set of repeats (collapses to one needle), and the
/// byte values at both ends of the range.
fn sets() -> Vec<Vec<u8>> {
    vec![
        b"".to_vec(),
        b",".to_vec(),
        b",\"\r\n".to_vec(),
        b"\t\"\\\r\n".to_vec(),
        b";\"\\\r\n\xe2".to_vec(),
        b"abcdefgh".to_vec(),
        // Nine members: wider than the vector kernels, answered portably.
        b"abcdefghi".to_vec(),
        // Repeats collapse to one needle.
        b",,,,".to_vec(),
        // The byte values at both ends of the range.
        b"\x00\xff".to_vec(),
    ]
}

/// Every kernel this build provides against the per-byte reference, at every
/// start alignment 0..64 and length 0..256, with the first member planted at
/// every position (and nowhere), and a member beyond the slice that must
/// never be reported.
fn every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length() {
    let kernels = kernels();
    let mut backing = vec![b'.'; 64 + 256 + 64];
    for members in sets() {
        let member = (0..=u8::MAX).find(|&b| members.contains(&b));
        let filler = (0..=u8::MAX)
            .find(|&b| !members.contains(&b))
            .expect("no set covers every byte");
        for start in 0..64 {
            for len in 0..256 {
                let plants = std::iter::once(None).chain((0..len).map(Some));
                for plant in plants {
                    backing.fill(filler);
                    if let Some(m) = member {
                        backing[start + len] = m;
                    }
                    if let (Some(at), Some(m)) = (plant, member) {
                        backing[start + at] = m;
                    }
                    let haystack = &backing[start..start + len];
                    let expected = reference(&members, haystack);
                    for (name, kernel) in &kernels {
                        assert_eq!(
                            kernel(&members, haystack),
                            expected,
                            "{name} start {start} len {len} plant {plant:?} members {members:?}"
                        );
                    }
                }
            }
        }
    }
}

/// A uniform-enough draw below `n` (`n > 0`) for input generation.
fn below(rng: &mut SplitMix64, n: usize) -> usize {
    (rng.next_u64() % n as u64) as usize
}

/// Seeded random haystacks over a small alphabet, so members are dense in
/// some and absent from others.
fn every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs() {
    let kernels = kernels();
    let mut rng = SplitMix64::new(0x00C5_F1E1_D5CA_2026);
    for _ in 0..4_000 {
        let members: Vec<u8> = (0..below(&mut rng, 10))
            .map(|_| rng.next_u64().to_le_bytes()[0])
            .collect();
        let alphabet: Vec<u8> = (0..=below(&mut rng, 12))
            .map(|_| rng.next_u64().to_le_bytes()[0])
            .collect();
        let len = below(&mut rng, 700);
        let haystack: Vec<u8> = (0..len)
            .map(|_| alphabet[below(&mut rng, alphabet.len())])
            .collect();
        let start = below(&mut rng, len.max(1)).min(len);
        let haystack = &haystack[start..];
        let expected = reference(&members, haystack);
        for (name, kernel) in &kernels {
            assert_eq!(
                kernel(&members, haystack),
                expected,
                "{name} members {members:?}"
            );
        }
    }
}

/// The explicit kernel this target compiles is among those just compared, so
/// a `cfg` slip cannot leave it untested while the two cases above pass —
/// `sse2` on `x86_64`, `neon` on `aarch64`, and — the one this file exists
/// for — `simd128` on `wasm32` built with `simd128` enabled. Also names the
/// selected kernel, so `make wasm-test`'s log shows which one ran.
fn the_target_explicit_kernel_is_among_those_compared() {
    let kernels = kernels();
    let names: Vec<&str> = kernels.iter().map(|(name, _)| *name).collect();
    let selected = Backend::selected();
    purrdf_testkit::harness::print_line(&format!(
        "csv scan: {} checked; {} selected",
        names.join(", "),
        selected.name()
    ));
    assert!(names.contains(&"portable"));
    assert!(names.contains(&"dispatch"));
    assert!(names.contains(&selected.name()), "{names:?}");
    #[cfg(target_arch = "x86_64")]
    assert!(names.contains(&"sse2"), "{names:?}");
    #[cfg(target_arch = "aarch64")]
    {
        assert!(names.contains(&"neon"), "{names:?}");
        assert_eq!(selected, Backend::Neon);
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        assert!(names.contains(&"simd128"), "{names:?}");
        assert_eq!(selected, Backend::Simd128);
    }
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    {
        assert_eq!(names, vec!["dispatch", "portable"]);
        assert_eq!(selected, Backend::Portable);
    }
}

purrdf_testkit::harness_main!(
    every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length,
    every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs,
    the_target_explicit_kernel_is_among_those_compared,
);

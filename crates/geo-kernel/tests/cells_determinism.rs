// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public integer-grid replay on native and wasm32 testkit runners.

use purrdf_geo_kernel::cells::{CubeHilbertQ62V1, NativeGridProfile};
use purrdf_geo_kernel::metric::Metres;
use purrdf_geo_kernel::{LonLat, Rat};
use purrdf_hash::fnv::{BASIS, fold};
use purrdf_testkit::harness::report_digest;

const VECTORS: &str = include_str!("vectors/cube_assignment.txt");
const GOLDEN_DIGEST: u64 = 0xabee_6e51_6224_9afe;

fn public_grid_replay() {
    let count = VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count();
    assert_eq!(count, 10, "freeze complete public-vector coverage");
    let digest = report_digest("public_grid_replay", count * 2 * 31 + 2 * 31 + 4, || {
        let mut digest = BASIS;
        for native_profile in [NativeGridProfile::Wgs84, NativeGridProfile::Cgcs2000] {
            let grid = CubeHilbertQ62V1::new(native_profile);
            digest = fold(digest, grid.profile_id().digest().as_ref());
            for level in 0..=30 {
                let bounds = grid.physical_scale_bounds(level).expect("physical bounds");
                for value in [
                    bounds.nominal_lower(),
                    bounds.nominal_upper(),
                    bounds.footprint_guard(),
                    bounds.lower(),
                    bounds.upper(),
                ] {
                    let mut bytes = Vec::new();
                    for field in [value.exact().numerator(), value.exact().denominator()] {
                        purrdf_hash::frame::frame_le(&mut bytes, field.to_string().as_bytes());
                    }
                    digest = fold(digest, &bytes);
                }
            }
            for (target, expected_level) in [(200, 16), (150, 17)] {
                let level = grid
                    .level_for_max_edge_length(&Metres::new(Rat::from_i64(target)))
                    .expect("physical level");
                assert_eq!(level, expected_level);
                digest = fold(digest, &[level]);
            }
            for row in VECTORS
                .lines()
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
            {
                let fields: Vec<_> = row.split_ascii_whitespace().collect();
                assert_eq!(fields.len(), 3);
                let point = LonLat::new(
                    Rat::parse_decimal(fields[0]).expect("exact longitude"),
                    Rat::parse_decimal(fields[1]).expect("exact latitude"),
                )
                .expect("in-range point");
                let expected: [u8; 8] = purrdf_hash::hex::decode_canonical(fields[2])
                    .expect("canonical key")
                    .try_into()
                    .expect("eight-byte key");
                let leaf = grid.assign(&point, 30).expect("leaf assignment");
                assert_eq!(leaf.to_be_bytes(), expected);
                for level in 0..=30 {
                    let cell = grid.assign(&point, level).expect("public assignment");
                    assert_eq!(cell, leaf.ancestor(level).expect("public ancestor"));
                    let range = cell.descendant_range(30).expect("leaf range");
                    assert!(range.contains(leaf).expect("matching profile"));
                    for bytes in [
                        cell.to_be_bytes(),
                        range.min().to_be_bytes(),
                        range.max().to_be_bytes(),
                        range.stride().to_be_bytes(),
                        range.count().to_be_bytes(),
                    ] {
                        digest = fold(digest, &bytes);
                    }
                }
            }
        }
        digest
    });
    assert_eq!(digest, GOLDEN_DIGEST, "public grid bytes changed");
}

purrdf_testkit::harness_main!(public_grid_replay);

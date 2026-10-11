// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::error::GeoError;
use crate::exact::{Int, Rat};
use crate::geographic::LonLat;
use crate::metric::Metres;
use crate::profile::PreparedEllipsoid;

#[test]
fn borrowed_policy_batches_preserve_keys_and_charge_original_sources() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let sources = [
        point("116.391", "39.9075"),
        point("180", "0"),
        point("45", "90"),
    ];
    let sentinel = grid.assign(&point("0", "0"), 0).unwrap();
    let mut output = [sentinel; 3];
    struct Count(u64);
    impl crate::MetricWorkObserver for Count {
        fn charge_chunk(&mut self, work: u64, _workspace: u64) -> Result<(), GeoError> {
            self.0 += work;
            Ok(())
        }
    }
    let mut count = Count(0);
    grid.assign_batch_in_policy_metered(
        sources.iter(),
        30,
        &mut output,
        crate::ExecutionPolicy::geometry(),
        &mut count,
    )
    .unwrap();
    for (point, result) in sources.iter().zip(output) {
        assert_eq!(result, grid.assign(point, 30).unwrap());
        assert_eq!(
            result,
            grid.assign_in_policy(point, 30, crate::ExecutionPolicy::geometry())
                .unwrap()
        );
    }
    assert!(count.0 > sources.len() as u64 * 160);
    let mut limits = crate::ExecutionLimits::GEOMETRY;
    limits.max_output_elements = 2;
    output.fill(sentinel);
    assert_eq!(
        grid.assign_batch_in_policy(
            sources.iter(),
            30,
            &mut output,
            crate::ExecutionPolicy::new(limits).unwrap()
        )
        .unwrap_err(),
        GeoError::OutputExhausted { limit: 2 }
    );
    assert_eq!(output, [sentinel; 3]);
}

#[test]
fn borrowed_policy_batches_refuse_cancellation_and_dishonest_iterator_lengths() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let sources = [point("1", "2"), point("3", "4")];
    let sentinel = grid.assign(&point("0", "0"), 0).unwrap();
    struct Cancel;
    impl crate::MetricWorkObserver for Cancel {
        fn charge_chunk(&mut self, _work: u64, _workspace: u64) -> Result<(), GeoError> {
            Err(GeoError::Cancelled)
        }
    }
    let mut output = [sentinel; 2];
    assert_eq!(
        grid.assign_batch_in_policy_metered(
            sources.iter(),
            30,
            &mut output,
            crate::ExecutionPolicy::geometry(),
            &mut Cancel
        )
        .unwrap_err(),
        GeoError::Cancelled
    );
    assert_eq!(output, [sentinel; 2]);
    struct Dishonest<'a> {
        values: core::slice::Iter<'a, LonLat>,
        declared: usize,
    }
    impl<'a> Iterator for Dishonest<'a> {
        type Item = &'a LonLat;
        fn next(&mut self) -> Option<Self::Item> {
            self.values.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.declared, Some(self.declared))
        }
    }
    impl ExactSizeIterator for Dishonest<'_> {}
    for (actual, declared) in [(1, 2), (2, 1)] {
        let mut output = vec![sentinel; declared];
        let sources = Dishonest {
            values: sources[..actual].iter(),
            declared,
        };
        assert!(matches!(
            grid.assign_batch_in_policy(
                sources,
                30,
                &mut output,
                crate::ExecutionPolicy::geometry()
            ),
            Err(GeoError::InvalidOutputLength { .. })
        ));
    }
}

#[test]
fn admitted_assignment_preserves_keys_and_refuses_unbounded_original_arithmetic() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    for source in [
        point("116.391", "39.9075"),
        point("180", "0"),
        point("45", "90"),
    ] {
        let mut budget = super::cover::CoverBudget::new(MixedCoverLimits::DEFAULT).unwrap();
        assert_eq!(
            grid.assign_admitted(&source, 30, &mut budget).unwrap(),
            grid.assign(&source, 30).unwrap()
        );
        assert!(budget.work_items() > 160);
    }
    let longitude = Rat::new(Int::one(), Int::one().shl(65_536)).unwrap();
    let source = LonLat::new(longitude, Rat::zero()).unwrap();
    let mut budget = super::cover::CoverBudget::new(MixedCoverLimits::DEFAULT).unwrap();
    assert!(matches!(
        grid.assign_admitted(&source, 30, &mut budget),
        Err(GeoError::WorkExhausted { .. })
    ));
}

use super::constants::{COSINE, PI96, PI192, Q62, SINE};
use super::{
    CellId, CubeHilbertQ62V1, GridProfileId, MixedCoverLimits, NativeGridProfile, hilbert_path,
    owned_bin,
};

// The independent generator remains one implementation inside this crate.
#[path = "../../examples/cube/constants.rs"]
mod generation;

fn point(longitude: &str, latitude: &str) -> LonLat {
    LonLat::new(
        Rat::parse_decimal(longitude).expect("exact longitude"),
        Rat::parse_decimal(latitude).expect("exact latitude"),
    )
    .expect("in-range point")
}

#[test]
fn independently_generated_constants_certify_the_frozen_values() {
    let generated = generation::certified_constants();
    assert_eq!(generated.pi192, PI192);
    assert_eq!(generated.pi96, PI96);
    assert_eq!(generated.sine, SINE);
    assert_eq!(generated.cosine, COSINE);
}

#[test]
fn exact_rational_error_bound_includes_all_component_rounding() {
    let d = Rat::new(Int::one(), Int::one().shl(63)).expect("half Q62 unit");
    let e = Rat::new(Int::one(), Int::one().shl(67)).expect("trig enclosure");
    let sum = d.add(&e);
    let total = d
        .mul(&Rat::from_i64(7))
        .add(&e.mul(&Rat::from_i64(5)))
        .add(&sum.mul(&sum).mul(&Rat::from_i64(2)));
    let required = Rat::new(Int::one(), Int::one().shl(60)).expect("component bound");
    assert!(total < required);

    let four_fifths = Rat::new(Int::from_i64(4), Int::from_i64(5)).expect("fraction");
    let tail_numerator = (0..20).fold(Rat::one(), |v, _| v.mul(&four_fifths));
    let factorial = (1..=20).fold(Int::one(), |v, i| v.mul(&Int::from_i64(i)));
    let tail = tail_numerator
        .div(&Rat::from_int(factorial))
        .expect("positive factorial");
    let roundoff = Rat::new(Int::from_i64(128), Int::one().shl(97)).expect("roundoff bound");
    assert!(tail.add(&roundoff) < e);
}

/// Independent Q160 power-series oracle: full angles, term recurrence and
/// truncation, sharing neither quarter reduction nor Horner arithmetic.
fn reference_sin_cos(degrees: &Rat) -> (Int, Int) {
    let pi192 = PI192.iter().rev().fold(Int::zero(), |value, limb| {
        value.shl(64).add(&Int::from_u64(*limb))
    });
    let angle = degrees
        .numerator()
        .mul(&pi192)
        .div_rem(&degrees.denominator().mul(&Int::from_i64(180)).shl(32))
        .expect("positive angular denominator")
        .0;
    let angle_squared = angle.mul(&angle);
    let mut sine_term = angle;
    let mut cosine_term = Int::one().shl(160);
    let mut sine = sine_term.clone();
    let mut cosine = cosine_term.clone();
    // At |angle|<=pi, the omitted term after degree eighty is <2^-250;
    // forty Q160 recurrence truncations are negligible beside the Q62 bound.
    for term in 1..=40 {
        let denominator = Int::from_i64(2 * term).shl(320);
        sine_term = sine_term
            .mul(&angle_squared)
            .neg()
            .div_rem(&denominator.mul(&Int::from_i64(2 * term + 1)))
            .expect("positive sine denominator")
            .0;
        cosine_term = cosine_term
            .mul(&angle_squared)
            .neg()
            .div_rem(&denominator.mul(&Int::from_i64(2 * term - 1)))
            .expect("positive cosine denominator")
            .0;
        sine = sine.add(&sine_term);
        cosine = cosine.add(&cosine_term);
    }
    (sine, cosine)
}

#[test]
fn original_coordinates_agree_with_independent_higher_precision_components() {
    let scale = Int::one().shl(160);
    let required = Int::one().shl(100); // 2^-60 at scale 2^160.
    let angles = [
        "-180",
        "-179.99999999999999999999999999999999",
        "-135",
        "-90",
        "-45",
        "-0.00000000000000000000000000000001",
        "0",
        "22.5",
        "45",
        "89.99999999999999999999999999999999",
        "90",
        "135",
        "180",
    ];
    for longitude in angles {
        for latitude in [
            "-90",
            "-89.99999999999999999999999999999999",
            "-45",
            "0",
            "45",
            "89.99999999999999999999999999999999",
            "90",
        ] {
            let point = point(longitude, latitude);
            let actual = super::fixed::normal(&point).expect("Q62 normal");
            let (sl, cl) = reference_sin_cos(point.longitude());
            let (sb, cb) = reference_sin_cos(point.latitude());
            let expected = [
                cb.mul(&cl).div_rem(&scale).expect("positive scale").0,
                cb.mul(&sl).div_rem(&scale).expect("positive scale").0,
                sb,
            ];
            let total_error =
                actual
                    .iter()
                    .zip(&expected)
                    .fold(Int::zero(), |sum, (component, expected)| {
                        sum.add(&Int::from_i128(*component).shl(98).sub(expected).abs())
                    });
            assert!(total_error < required, "{longitude},{latitude}");
        }
    }
}

#[test]
fn frozen_assignment_vectors_preserve_poles_seams_and_ties() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    for row in include_str!("../../tests/vectors/cube_assignment.txt")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let fields: Vec<_> = row.split_ascii_whitespace().collect();
        assert_eq!(fields.len(), 3);
        let (longitude, latitude) = (fields[0], fields[1]);
        let key_bytes: [u8; 8] = purrdf_hash::hex::decode_canonical(fields[2])
            .expect("canonical hexadecimal")
            .try_into()
            .expect("eight-byte key");
        let key = u64::from_be_bytes(key_bytes);
        let point = point(longitude, latitude);
        let leaf = grid.assign(&point, 30).expect("leaf assignment");
        assert_eq!(leaf.key(), key, "{longitude},{latitude}");
        for level in 0..=30 {
            assert_eq!(
                grid.assign(&point, level).expect("assignment"),
                leaf.ancestor(level).expect("ancestor")
            );
        }
    }
    assert_eq!(
        super::fixed::normal(&point("0", "90")).expect("normal"),
        [0, 0, Q62]
    );
    assert_eq!(
        grid.assign(&point("-0", "0"), 30).expect("signed zero"),
        grid.assign(&point("0", "-0"), 30).expect("signed zero")
    );
}

#[test]
fn every_small_hilbert_level_is_a_contiguous_bijection() {
    for level in 0..=6 {
        let n = 1_u32 << level;
        let mut ordered = vec![None; (n * n) as usize];
        for x in 0..n {
            for y in 0..n {
                let path = hilbert_path(x, y, level);
                let slot = &mut ordered[path as usize];
                assert!(slot.replace((x, y)).is_none(), "duplicate Hilbert path");
            }
        }
        let ordered: Vec<_> = ordered
            .into_iter()
            .map(|p| p.expect("surjective path"))
            .collect();
        assert_eq!(ordered[0], (0, 0));
        assert_eq!(ordered[ordered.len() - 1], (n - 1, 0));
        for pair in ordered.windows(2) {
            assert_eq!(
                pair[0].0.abs_diff(pair[1].0) + pair[0].1.abs_diff(pair[1].1),
                1
            );
        }
    }
}

#[test]
fn raw_face_endpoints_form_one_continuous_cycle() {
    // Independently evaluate the forward chart equations, then exercise the
    // production inverse charts and odd-root endpoint transposition.
    let chart = |face: usize, u, v| match face {
        0 => [1, u, v],
        1 => [-u, 1, v],
        2 => [-u, -v, 1],
        3 => [-1, -v, -u],
        4 => [v, -1, -u],
        _ => [v, u, -1],
    };
    let endpoints: Vec<_> = (0..6)
        .map(|face| {
            let (u, v) = if face & 1 == 0 { (1, -1) } else { (-1, 1) };
            let start = chart(face, -1, -1);
            let end = chart(face, u, v);
            assert_eq!(super::chart_coordinates(face, start), (-1, -1));
            assert_eq!(super::chart_coordinates(face, end), (u, v));
            (start, end)
        })
        .collect();
    for face in 0..6 {
        assert_eq!(endpoints[face].1, endpoints[(face + 1) % 6].0);
    }
}

#[test]
fn exact_dyadic_warp_boundaries_and_interiors_have_half_open_ownership() {
    for level in 0..=8 {
        let n = 1_i128 << level;
        let denominator = 3 * n * n;
        let coarse_shift = 30 - level;
        for boundary in 0..=n {
            let a = 2 * boundary - n;
            let numerator = a * (2 * n + a.abs());
            let actual = owned_bin(numerator, denominator) >> coarse_shift;
            assert_eq!(i128::from(actual), boundary.min(n - 1));
            if boundary < n {
                let b = a + 2;
                let next_numerator = b * (2 * n + b.abs());
                let midpoint = owned_bin(numerator + next_numerator, 2 * denominator);
                assert_eq!(i128::from(midpoint >> coarse_shift), boundary);
            }
        }
    }
    assert_eq!(owned_bin(-Q62, Q62), 0);
    assert_eq!(owned_bin(Q62, Q62), (1 << 30) - 1);
}

#[test]
fn exhaustive_small_subtrees_match_strided_ranges_and_byte_order() {
    let profile = GridProfileId::wgs84();
    for face in 0..6 {
        for parent_level in 0..=4 {
            for path in 0..1_u64 << (2 * parent_level) {
                let parent = CellId::from_path(profile, face, path, parent_level).expect("cell");
                for stored_level in parent_level..=6 {
                    let range = parent.descendant_range(stored_level).expect("range");
                    let count = 1_u64 << (2 * (stored_level - parent_level));
                    assert_eq!(range.count(), count);
                    let base = path << (2 * (stored_level - parent_level));
                    let mut previous = None;
                    for suffix in 0..count {
                        let child = CellId::from_path(profile, face, base + suffix, stored_level)
                            .expect("descendant");
                        assert_eq!(child.key(), range.min() + suffix * range.stride());
                        assert_eq!(child.ancestor(parent_level).expect("ancestor"), parent);
                        assert!(range.contains(child).expect("same profile"));
                        assert!(child.range_min() >= parent.range_min());
                        assert!(child.range_max() <= parent.range_max());
                        if let Some(previous) = previous {
                            assert!(previous < child.to_be_bytes());
                        }
                        previous = Some(child.to_be_bytes());
                    }
                    assert_eq!(range.max(), range.min() + (count - 1) * range.stride());
                }
                if parent_level > 0 {
                    assert_eq!(parent.ancestors().len(), usize::from(parent_level));
                    assert_eq!(
                        parent.ancestors().last().expect("root"),
                        CellId::root(profile, face).expect("root")
                    );
                }
                for child in parent.children().expect("nonleaf") {
                    assert_eq!(child.parent().expect("parent"), parent);
                }
            }
        }
    }
}

#[test]
fn malformed_keys_and_incompatible_operations_refuse() {
    let wgs = GridProfileId::wgs84();
    let cgcs = GridProfileId::cgcs2000();
    assert_ne!(wgs, cgcs);
    for face in 0_u64..8 {
        for sentinel_bit in 0..61 {
            let key = (face << 61) | (1_u64 << sentinel_bit);
            let decoded = CellId::from_key(wgs, key);
            if face < 6 && sentinel_bit % 2 == 0 {
                let cell = decoded.expect("valid face and sentinel");
                assert_eq!(u64::from(cell.face()), face);
                assert_eq!(u32::from(cell.level()), (60 - sentinel_bit) / 2);
            } else {
                assert!(matches!(decoded, Err(GeoError::InvalidCellId(_))));
            }
        }
    }
    for key in [0, 1 << 1, 1 << 61, 1 << 63, (6 << 61) | 1, (7 << 61) | 1] {
        assert!(matches!(
            CellId::from_key(wgs, key),
            Err(GeoError::InvalidCellId(_))
        ));
    }
    let root = CellId::root(wgs, 0).expect("root");
    assert!(matches!(root.parent(), Err(GeoError::RootHasNoParent)));
    assert!(matches!(
        root.ancestor(1),
        Err(GeoError::InvalidAncestorLevel { .. })
    ));
    assert!(matches!(
        root.descendant_range(31),
        Err(GeoError::InvalidResolution(31))
    ));
    let leaf = CellId::from_key(wgs, 1).expect("leaf");
    assert!(matches!(leaf.children(), Err(GeoError::LeafHasNoChildren)));
    assert!(matches!(
        leaf.descendant_range(29),
        Err(GeoError::InvalidStoredLevel { .. })
    ));
    let other = CellId::from_key(cgcs, 1).expect("other-profile leaf");
    assert!(matches!(
        root.descendant_range(30).expect("range").contains(other),
        Err(GeoError::GridProfileMismatch)
    ));
}

#[test]
fn caller_buffer_batch_matches_scalar_and_rejects_shape_before_writing() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Cgcs2000);
    let points = [point("0", "0"), point("45", "45"), point("180", "-90")];
    let initial = CellId::root(grid.profile_id(), 0).expect("initial valid cell");
    let mut output = [initial; 3];
    grid.assign_batch(&points, 17, &mut output).expect("batch");
    for (point, cell) in points.iter().zip(output) {
        assert_eq!(grid.assign(point, 17).expect("scalar"), cell);
    }
    let before = output;
    assert!(matches!(
        grid.assign_batch(&points, 17, &mut output[..2]),
        Err(GeoError::InvalidOutputLength { .. })
    ));
    assert_eq!(output, before);
    assert!(matches!(
        grid.assign_batch(&points, 31, &mut output),
        Err(GeoError::InvalidResolution(31))
    ));
    assert_eq!(output, before);
    grid.assign_batch(&[], 0, &mut []).expect("empty batch");
}

#[test]
fn physical_scale_bounds_and_selection_use_exact_axes_and_complete_guard() {
    for native in [NativeGridProfile::Wgs84, NativeGridProfile::Cgcs2000] {
        let grid = CubeHilbertQ62V1::new(native);
        assert_eq!(
            grid.level_for_max_edge_length(&Metres::new(Rat::from_i64(200)))
                .expect("200m target"),
            16
        );
        assert_eq!(
            grid.level_for_max_edge_length(&Metres::new(Rat::from_i64(150)))
                .expect("150m target"),
            17
        );
        let bounds16 = grid.physical_scale_bounds(16).expect("bounds");
        assert!(bounds16.nominal_lower().exact() > &Rat::from_i64(90));
        assert!(bounds16.nominal_lower().exact() < &Rat::from_i64(92));
        assert!(bounds16.nominal_upper().exact() > &Rat::from_i64(170));
        assert!(bounds16.nominal_upper().exact() < &Rat::from_i64(172));
        assert_eq!(
            bounds16.upper().exact(),
            &bounds16
                .nominal_upper()
                .exact()
                .add(bounds16.footprint_guard().exact())
        );
        assert_eq!(
            bounds16.lower().exact(),
            &bounds16
                .nominal_lower()
                .exact()
                .sub(bounds16.footprint_guard().exact())
        );
        let mut previous = None;
        for level in 0..=30 {
            let bounds = grid.physical_scale_bounds(level).expect("bounds");
            assert_eq!(
                grid.level_for_max_edge_length(bounds.upper())
                    .expect("inclusive exact bound"),
                level
            );
            if let Some(previous) = previous {
                assert!(bounds.upper() < &previous);
            }
            previous = Some(bounds.upper().clone());
        }
        assert!(matches!(
            grid.level_for_max_edge_length(&Metres::new(Rat::zero())),
            Err(GeoError::NonPositiveEdgeLength(_))
        ));
        assert!(matches!(
            grid.level_for_max_edge_length(&Metres::new(Rat::from_i64(-1))),
            Err(GeoError::NonPositiveEdgeLength(_))
        ));
        let unattainable = grid
            .physical_scale_bounds(30)
            .expect("bounds")
            .upper()
            .exact()
            .div(&Rat::from_i64(2))
            .expect("positive denominator");
        assert!(matches!(
            grid.level_for_max_edge_length(&Metres::new(unattainable)),
            Err(GeoError::UnattainableEdgeLength { .. })
        ));
        assert!(matches!(
            grid.physical_scale_bounds(31),
            Err(GeoError::InvalidResolution(31))
        ));
    }
    let ellipsoid = PreparedEllipsoid::new(Rat::from_i64(10), Rat::from_i64(2)).expect("axes10,5");
    let bounds = super::physical_scale_bounds(&ellipsoid, 0).expect("bounds");
    assert_eq!(bounds.nominal_upper().exact(), &Rat::from_i64(35)); // R=10²/5=20.
    assert_eq!(
        bounds.footprint_guard().exact(),
        &Rat::new(Int::from_i64(20), Int::one().shl(58)).expect("guard")
    );
    let sqrt_floor = Int::from_i64(2)
        .shl(192)
        .sqrt_floor()
        .expect("positive radicand");
    let radicand = Int::from_i64(2).shl(192);
    assert!(sqrt_floor.mul(&sqrt_floor) <= radicand);
    let next = sqrt_floor.add(&Int::one());
    assert!(next.mul(&next) > radicand);
}

#[test]
fn admitted_physical_scales_preserve_exact_values_and_default_level_selection() {
    struct Receipt {
        work: u64,
        peak: u64,
    }
    impl crate::MetricWorkObserver for Receipt {
        fn charge_chunk(&mut self, work: u64, workspace: u64) -> Result<(), GeoError> {
            self.work += work;
            self.peak = self.peak.max(workspace);
            Ok(())
        }
    }
    let policy = crate::ExecutionPolicy::geometry();
    for native in [NativeGridProfile::Wgs84, NativeGridProfile::Cgcs2000] {
        let grid = CubeHilbertQ62V1::new(native);
        for level in 0..=30 {
            let mut receipt = Receipt { work: 0, peak: 0 };
            let result = grid
                .physical_scale_bounds_in_policy_metered(level, policy, &mut receipt)
                .expect("default exact scale admission");
            assert_eq!(result, grid.physical_scale_bounds(level).unwrap());
            assert!(receipt.work > 100);
            assert!(receipt.peak >= result.retained_workspace_bytes());
            assert_eq!(
                grid.level_for_max_edge_length_in_policy(result.upper(), policy)
                    .expect("inclusive exact bound under defaults"),
                level
            );
        }
        for (target, expected) in [(200, 16), (150, 17)] {
            assert_eq!(
                grid.level_for_max_edge_length_in_policy(
                    &Metres::new(Rat::from_i64(target)),
                    policy
                )
                .unwrap(),
                expected
            );
        }
        let mut integer_limits = crate::ExecutionLimits::GEOMETRY;
        integer_limits.max_precision_bits = 1;
        integer_limits.max_iterations = 1;
        integer_limits.max_subdivision_levels = 1;
        integer_limits.max_scratch_destinations = 1;
        let integer_policy = crate::ExecutionPolicy::new(integer_limits).unwrap();
        assert_eq!(
            grid.physical_scale_bounds_in_policy(16, integer_policy)
                .unwrap(),
            grid.physical_scale_bounds(16).unwrap()
        );
        assert_eq!(
            grid.level_for_max_edge_length_in_policy(
                &Metres::new(Rat::from_i64(150)),
                integer_policy
            )
            .unwrap(),
            17
        );
    }
}

#[test]
fn physical_scale_admission_refuses_original_operand_work_memory_and_cancellation() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let mut limits = crate::ExecutionLimits::GEOMETRY;
    limits.max_work_items = 1;
    let work_policy = crate::ExecutionPolicy::new(limits).unwrap();
    assert!(matches!(
        grid.physical_scale_bounds_in_policy(16, work_policy),
        Err(GeoError::WorkExhausted { limit: 1 })
    ));
    assert!(matches!(
        grid.level_for_max_edge_length_in_policy(&Metres::new(Rat::from_i64(150)), work_policy),
        Err(GeoError::WorkExhausted { limit: 1 })
    ));
    limits = crate::ExecutionLimits::GEOMETRY;
    limits.max_workspace_bytes = 1;
    let memory_policy = crate::ExecutionPolicy::new(limits).unwrap();
    assert!(matches!(
        grid.physical_scale_bounds_in_policy(16, memory_policy),
        Err(GeoError::MemoryExhausted { limit: 1 })
    ));
    let huge_denominator = Int::one().shl(65_536);
    let target =
        Metres::new(Rat::new(huge_denominator.add(&Int::one()), huge_denominator).unwrap());
    assert!(matches!(
        grid.level_for_max_edge_length_in_policy(&target, crate::ExecutionPolicy::geometry()),
        Err(GeoError::WorkExhausted { .. })
    ));
    struct Cancel;
    impl crate::MetricWorkObserver for Cancel {
        fn charge_chunk(&mut self, _work: u64, _workspace: u64) -> Result<(), GeoError> {
            Err(GeoError::Cancelled)
        }
    }
    assert_eq!(
        grid.physical_scale_bounds_in_policy_metered(
            16,
            crate::ExecutionPolicy::geometry(),
            &mut Cancel
        )
        .unwrap_err(),
        GeoError::Cancelled
    );
    assert_eq!(
        grid.level_for_max_edge_length_in_policy_metered(
            &Metres::new(Rat::from_i64(150)),
            crate::ExecutionPolicy::geometry(),
            &mut Cancel
        )
        .unwrap_err(),
        GeoError::Cancelled
    );
}

#[test]
fn combined_scale_selection_reuses_exact_factors_under_one_default_admission() {
    for native in [NativeGridProfile::Wgs84, NativeGridProfile::Cgcs2000] {
        let grid = CubeHilbertQ62V1::new(native);
        for (target, expected) in [(150, 17), (200, 16)] {
            let (bounds, selected) = grid
                .physical_scale_bounds_and_level_in_policy(
                    16,
                    Some(&Metres::new(Rat::from_i64(target))),
                    crate::ExecutionPolicy::geometry(),
                )
                .expect("one shared factor construction under defaults");
            assert_eq!(bounds, grid.physical_scale_bounds(16).unwrap());
            assert_eq!(selected, Some(expected));
        }
        let (bounds, selected) = grid
            .physical_scale_bounds_and_level_in_policy(30, None, crate::ExecutionPolicy::geometry())
            .unwrap();
        assert_eq!(bounds, grid.physical_scale_bounds(30).unwrap());
        assert_eq!(selected, None);
    }
}

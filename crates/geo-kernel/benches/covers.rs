// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only cover costs and complete resource/level receipts.
#![allow(missing_docs)]

use purrdf_geo_kernel::{
    LonLat, Metres, Rat,
    cells::{CoverLevels, CubeHilbertQ62V1, MixedCoverLimits, NativeGridProfile},
};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};
use std::time::Duration;

fn covers(bench: &mut Bench) {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let center =
        LonLat::new(Rat::from_i64(116), Rat::from_i64(40)).expect("fixed public synthetic point");
    let level17 = grid
        .level_for_max_edge_length(&Metres::new(Rat::from_i64(150)))
        .expect("attainable scale");
    assert_eq!(level17, 17);
    for level in [16, level17] {
        for radius in [1_000, 30_000, 400_000] {
            let radius = Metres::new(Rat::from_i64(radius));
            let levels = CoverLevels::new(0, level).expect("supported");
            let limits = MixedCoverLimits::DEFAULT;
            match grid.cover_disk_mixed(&center, &radius, levels, limits) {
                Ok(cover) => println!(
                    "receipt radius={radius:?} levels={levels:?} emitted={} {:?}",
                    cover.cells().len(),
                    cover.receipt()
                ),
                Err(error) => println!("refusal radius={radius:?} levels={levels:?} {error}"),
            }
            bench.bench_function(
                &format!(
                    "native-covers/radius-{}-level-{level}",
                    radius.exact().numerator()
                ),
                |sample| {
                    sample.iter(|| {
                        black_box(grid.cover_disk_mixed(
                            black_box(&center),
                            black_box(&radius),
                            levels,
                            limits,
                        ))
                    });
                },
            );
        }
    }
}
bench_group! {name=benches;config=Bench::default().sample_size(10).warm_up_time(Duration::from_millis(10)).measurement_time(Duration::from_millis(100));targets=covers}
bench_main!(benches);

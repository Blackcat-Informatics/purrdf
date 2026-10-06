// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One-shot complete cover receipts; measured distributions use the covers bench.

use purrdf_geo_kernel::{
    LonLat, Metres, Rat,
    cells::{CoverLevels, CubeHilbertQ62V1, MixedCoverLimits, NativeGridProfile},
};
use std::time::Instant;

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let work = arguments.next().map_or(262_144, |value| {
        value.parse::<u64>().expect("positive work count")
    });
    let level = arguments.next().map_or(16, |value| {
        value.parse::<u8>().expect("level zero through thirty")
    });
    let [longitude, latitude] = [116, 40].map(|default| {
        arguments.next().map_or(default, |value| {
            value.parse::<i64>().expect("exact integer coordinate")
        })
    });
    let radii = arguments
        .next()
        .map_or([Some(1_000), Some(30_000), Some(400_000)], |value| {
            [
                Some(value.parse::<i64>().expect("exact metre radius")),
                None,
                None,
            ]
        });
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let center = LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude))
        .expect("public synthetic point");
    println!("center_longitude_degrees={longitude} center_latitude_degrees={latitude}");
    for metres in radii.into_iter().flatten() {
        let radius = Metres::new(Rat::from_i64(metres));
        let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
        let start = Instant::now();
        let output = grid.cover_disk_mixed(
            &center,
            &radius,
            CoverLevels::new(0, level).expect("level interval"),
            MixedCoverLimits {
                max_work_items: work,
                ..MixedCoverLimits::DEFAULT
            },
        );
        let nanos = start.elapsed().as_nanos();
        let allocations = allocations.close();
        match output {
            Ok(cover) => println!(
                "radius_m={metres} max_level={level} elapsed_ns={nanos} emitted={} {:?} allocations={allocations:?}",
                cover.cells().len(),
                cover.receipt()
            ),
            Err(error) => {
                println!(
                    "radius_m={metres} max_level={level} elapsed_ns={nanos} refusal={error} allocations={allocations:?}"
                );
            }
        }
    }
}

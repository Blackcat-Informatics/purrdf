// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical Parquet reader refusals through the portable public API.

use purrdf_columnar::{ColumnarError, ParquetFiles, read};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

fn original_parquet_bounds_survive_file_disposal_without_narrowing() {
    let files = ParquetFiles::from_array(std::array::from_fn(|_| Vec::new()));
    let error = read(&files).unwrap_err();
    drop(files);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy);
    assert_eq!(fields.code, "columnar-truncated");
    assert!(matches!(
        fields.field("context"),
        ErrorValue::Code("Parquet file")
    ));
    assert!(matches!(fields.field("needed"), ErrorValue::ByteCount(12)));
    assert!(matches!(
        fields.field("available"),
        ErrorValue::ByteCount(0)
    ));
    let error = ColumnarError::LimitExceeded {
        context: "original wire count",
        value: u64::MAX,
        maximum: 7,
    };
    let fields = Capture::read(&error);
    assert!(
        matches!(fields.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(fields.field("maximum"), ErrorValue::Unsigned(7)));
}

purrdf_testkit::harness_main!(original_parquet_bounds_survive_file_disposal_without_narrowing);

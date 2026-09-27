// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The official JSON-Schema-Test-Suite, draft 2020-12, one libtest case per
//! suite test (see `suite_runner` for what runs and how).

mod suite_runner;

use std::process::ExitCode;

use purrdf_jsonschema::Dialect;
use suite_runner::{Draft, Inventory};

fn main() -> ExitCode {
    suite_runner::run(Draft {
        directory: "draft2020-12",
        dialect: Dialect::Draft2020_12,
        output_tests: true,
        expected: Inventory {
            files: 80,
            groups: 462,
            cases: 1463,
            format_cases: 874,
            output_cases: 4,
            remotes: 53,
        },
    })
}

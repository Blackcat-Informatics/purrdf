// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The official JSON-Schema-Test-Suite, draft draft-07, one libtest case per
//! suite test (see `suite_runner` for what runs and how).

mod suite_runner;

use std::process::ExitCode;

use purrdf_jsonschema::Dialect;
use suite_runner::{Draft, Inventory};

fn main() -> ExitCode {
    suite_runner::run(Draft {
        directory: "draft7",
        dialect: Dialect::Draft07,
        output_tests: false,
        expected: Inventory {
            files: 64,
            groups: 322,
            cases: 1047,
            format_cases: 793,
            output_cases: 0,
            remotes: 53,
        },
    })
}

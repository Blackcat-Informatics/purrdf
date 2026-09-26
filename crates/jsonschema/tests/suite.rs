// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The official JSON-Schema-Test-Suite, draft 2020-12, one libtest case per
//! suite test, `optional/` and the output-format tests included. See
//! `suite_runner` for what is run and how.

mod suite_runner;

use std::process::ExitCode;

use suite_runner::Draft;

static DRAFT: Draft = Draft {
    directory: "draft2020-12",
    metaschema: purrdf_jsonschema::DRAFT_2020_12,
    output_tests: true,
    expected_files: 59,
    expected_groups: 434,
    expected_cases: 1_463,
    expected_output_cases: 4,
    expected_remotes: 53,
};

fn main() -> ExitCode {
    suite_runner::run(&DRAFT)
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The official JSON-Schema-Test-Suite, draft-07, one libtest case per suite
//! test, `optional/` included (draft-07 has no output-format tests). See
//! `suite_runner` for what is run and how.

mod suite_runner;

use std::process::ExitCode;

use suite_runner::Draft;

static DRAFT: Draft = Draft {
    directory: "draft7",
    metaschema: purrdf_jsonschema::DRAFT_07,
    output_tests: false,
    expected_files: 45,
    expected_groups: 296,
    expected_cases: 1_047,
    expected_output_cases: 0,
    expected_remotes: 53,
};

fn main() -> ExitCode {
    suite_runner::run(&DRAFT)
}

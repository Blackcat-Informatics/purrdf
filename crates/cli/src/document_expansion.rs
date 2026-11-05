// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Raw source acquisition around the shared document expansion request.

use crate::{
    canonical,
    error::{CliError, CliOutcome},
    sink, source,
};
use purrdf_validate::{document_expansion, rdf_input::MAX_OPTIONS_BYTES};

fn acquire(path: &str, maximum: usize) -> Result<Vec<u8>, CliError> {
    source::read_raw_bytes_bounded(path, maximum).map_err(|cause| CliError::DocumentInput {
        path: path.to_owned(),
        cause,
    })
}

pub(crate) fn run(
    request: &str,
    input: &str,
    output: &str,
    canonical: &canonical::Request,
) -> Result<CliOutcome, CliError> {
    crate::argv_documents::refuse_shared_stdin(&[
        ("--request".to_owned(), request),
        ("IN".to_owned(), input),
    ])?;
    let request = acquire(request, MAX_OPTIONS_BYTES)?;
    let request = document_expansion::request(&request)?;
    // A malformed request, context or registry refuses before input acquisition.
    let input = acquire(input, request.max_input_bytes())?;
    let expanded = document_expansion::expand_decoded(&input, &request)?;
    sink::write_out_with_request(output, expanded.as_bytes(), canonical)?;
    Ok(CliOutcome::Complete)
}

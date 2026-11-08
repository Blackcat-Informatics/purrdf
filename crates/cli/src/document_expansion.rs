// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Raw source acquisition around the concrete structural document operations.

use crate::{
    canonical,
    error::{CliError, CliOutcome},
    sink, source,
};
use purrdf_validate::{
    document::{self, Operation},
    rdf_input::MAX_OPTIONS_BYTES,
};

fn acquire(path: &str, maximum: usize) -> Result<Vec<u8>, CliError> {
    source::read_raw_bytes_bounded(path, maximum).map_err(|cause| CliError::DocumentInput {
        path: path.to_owned(),
        cause,
    })
}

pub(crate) fn run(
    operation: Operation,
    request: &str,
    input: &str,
    frame: Option<&str>,
    output: &str,
    canonical: &canonical::Request,
) -> Result<CliOutcome, CliError> {
    let mut inputs = vec![("--request".to_owned(), request), ("IN".to_owned(), input)];
    if let Some(frame) = frame {
        inputs.push(("FRAME".to_owned(), frame));
    }
    crate::argv_documents::refuse_shared_stdin(&inputs)?;
    let request = acquire(request, MAX_OPTIONS_BYTES)?;
    let request = document::request(&request, operation)?;
    // A malformed request, context or registry refuses before input acquisition.
    let input = acquire(input, request.max_input_bytes())?;
    let frame = frame
        .map(|path| acquire(path, request.max_input_bytes()))
        .transpose()?;
    let text = match frame {
        Some(frame) => document::frame_decoded(&input, &frame, &request)?,
        None => document::execute_decoded(&input, &request)?,
    };
    sink::write_out_with_request(output, text.as_bytes(), canonical)?;
    Ok(CliOutcome::Complete)
}

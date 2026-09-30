// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reading a JSON document an emitter wrote into an oracle fixture.

use purrdf_lex::json::{Error, Value};

/// `text` read as JSON with every object's members in name order: the order
/// the emitters write, and so the order a fixture embeds the value in.
pub(crate) fn read_sorted(text: &str) -> Result<Value, Error> {
    let mut value = purrdf_lex::json::read(text)?;
    value.sort_keys();
    Ok(value)
}

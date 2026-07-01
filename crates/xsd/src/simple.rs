// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The simple (non-numeric, non-temporal) XSD value spaces: `boolean` and `string`.
//!
//! `xsd:string`'s value space is its lexical space, so it has no dedicated parser
//! (the [`crate::parse`] entry maps it straight to [`crate::XsdValue::String`]).

use crate::datatype::XsdDatatype;
use crate::value::XsdError;

/// `xsd:boolean`: lexical space is `true | false | 1 | 0`; canonical is `true|false`.
pub fn parse_boolean(s: &str) -> Result<bool, XsdError> {
    match s {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(XsdError::InvalidLexical {
            datatype: XsdDatatype::Boolean,
            lexical: s.to_string(),
            reason: "expected one of: true, false, 1, 0",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_lexicals() {
        assert_eq!(parse_boolean("true"), Ok(true));
        assert_eq!(parse_boolean("1"), Ok(true));
        assert_eq!(parse_boolean("false"), Ok(false));
        assert_eq!(parse_boolean("0"), Ok(false));
        assert!(parse_boolean("TRUE").is_err());
        assert!(parse_boolean("yes").is_err());
        assert!(parse_boolean("").is_err());
    }
}

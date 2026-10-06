// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The exact value of a JSON Schema number.
//!
//! A [`Number`] keeps the lexeme the document wrote, and
//! [`purrdf_xsd::json_number::JsonNumber`] is the workspace's one exact
//! reading of it: `const`, `enum`, `uniqueItems`, `type: integer`, the range
//! keywords and `multipleOf` all compare and divide on that value, never on a
//! rounded machine number.

use purrdf_lex::json::Number;
pub(crate) use purrdf_xsd::json_number::JsonNumber as Decimal;

/// The exact value `number` denotes.
pub(crate) fn exact(number: &Number) -> Decimal {
    Decimal::parse(number.lexeme()).expect("a JSON number's lexeme matches the RFC 8259 grammar")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    fn dec(text: &str) -> Decimal {
        exact(&Number::from_lexeme(text).expect("number"))
    }

    #[test]
    fn frozen_python_fraction_oracle() {
        for line in include_str!("../tests/numeric_oracle_vectors.txt").lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let mut fields = line.split('|');
            let op = fields.next().expect("operation");
            let left = fields.next().expect("left");
            let right = fields.next().expect("right");
            let expected = fields.next().expect("answer");
            assert_eq!(fields.next(), None, "{line}");
            let answer = match op {
                "cmp" => match dec(left).cmp(&dec(right)) {
                    Ordering::Less => "<",
                    Ordering::Equal => "=",
                    Ordering::Greater => ">",
                },
                "mul" => {
                    if dec(left).is_multiple_of(&dec(right)) {
                        "true"
                    } else {
                        "false"
                    }
                }
                _ => panic!("unknown oracle operation: {op}"),
            };
            assert_eq!(answer, expected, "{line}");
        }
    }
}

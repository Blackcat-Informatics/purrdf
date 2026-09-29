// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What an untagged plain scalar denotes: the one resolver the reader reads
//! with and the writer quotes by, so a string the writer leaves plain is a
//! string the reader reads back.
//!
//! The rules are the YAML 1.2 core schema (§10.3.2) as `serde_yaml` applies
//! them: `null`, `~` and the empty scalar are null; `true`/`false` in three
//! casings are booleans; an integer is decimal, `0x` hexadecimal, `0o` octal or
//! `0b` binary, optionally signed; a float is the core schema's decimal float
//! or `.inf`/`.nan`. A decimal with a redundant leading zero (`007`) is a
//! string, as `serde_yaml` reads it, and a radix integer beyond 128 bits is a
//! string too.

/// What a plain scalar denotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Plain {
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A finite number, as a JSON number lexeme.
    Number(String),
    /// `.inf`, `-.inf` or `.nan`, which JSON cannot hold.
    NonFinite,
    /// Text.
    String,
}

/// `0123`, `-007`: digits with a redundant leading zero, which `serde_yaml`
/// reads (and quotes) as a string although the core schema reads an integer.
pub(crate) fn digits_but_not_number(scalar: &str) -> bool {
    let digits = scalar.strip_prefix(['-', '+']).unwrap_or(scalar);
    digits.len() > 1 && digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())
}

/// The value of `digits` in `radix`, when every byte is a digit of it, there is
/// at least one, and the value fits `u128`.
fn radix_value(digits: &str, radix: u32) -> Option<u128> {
    if digits.is_empty() {
        return None;
    }
    let mut value = 0_u128;
    for byte in digits.bytes() {
        let digit = u128::from(match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return None,
        });
        if digit >= u128::from(radix) {
            return None;
        }
        value = value.checked_mul(u128::from(radix))?.checked_add(digit)?;
    }
    Some(value)
}

/// An integer spelling's JSON lexeme.
fn integer(scalar: &str) -> Option<String> {
    let (negative, unsigned) = match scalar.as_bytes().first() {
        Some(b'-') => (true, &scalar[1..]),
        Some(b'+') => (false, &scalar[1..]),
        _ => (false, scalar),
    };
    let radix = [("0x", 16), ("0o", 8), ("0b", 2)]
        .into_iter()
        .find_map(|(prefix, radix)| unsigned.strip_prefix(prefix).map(|rest| (rest, radix)));
    let magnitude = match radix {
        Some((rest, radix)) => radix_value(rest, radix)?,
        None => {
            if unsigned.is_empty()
                || digits_but_not_number(scalar)
                || !unsigned.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            if negative {
                // A JSON-grammatical negative decimal keeps its spelling.
                return Some(scalar.to_owned());
            }
            if !scalar.starts_with('+') {
                return Some(scalar.to_owned());
            }
            radix_value(unsigned, 10)?
        }
    };
    if negative {
        (magnitude <= 1_u128 << 127).then(|| format!("-{magnitude}"))
    } else {
        Some(magnitude.to_string())
    }
}

/// A core-schema decimal float spelling's JSON lexeme:
/// `[-+]? ( \. [0-9]+ | [0-9]+ ( \. [0-9]* )? ) ( [eE] [-+]? [0-9]+ )?`.
fn float(scalar: &str) -> Option<String> {
    let bytes = scalar.as_bytes();
    let mut at = 0;
    let negative = match bytes.first() {
        Some(b'-') => {
            at = 1;
            true
        }
        Some(b'+') => {
            at = 1;
            false
        }
        _ => false,
    };
    let digits = |from: usize| {
        let mut to = from;
        while bytes.get(to).is_some_and(u8::is_ascii_digit) {
            to += 1;
        }
        to
    };
    let int_end = digits(at);
    let integer_part = &scalar[at..int_end];
    let mut fraction = "";
    let mut end = int_end;
    if bytes.get(end) == Some(&b'.') {
        let frac_end = digits(end + 1);
        fraction = &scalar[end + 1..frac_end];
        end = frac_end;
    }
    if integer_part.is_empty() && fraction.is_empty() {
        return None;
    }
    let mut exponent = "";
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut exp_digits = end + 1;
        if matches!(bytes.get(exp_digits), Some(b'+' | b'-')) {
            exp_digits += 1;
        }
        let exp_end = digits(exp_digits);
        if exp_end == exp_digits {
            return None;
        }
        exponent = &scalar[end..exp_end];
        end = exp_end;
    }
    if end != bytes.len() {
        return None;
    }
    let integer_part = integer_part.trim_start_matches('0');
    let mut lexeme = String::with_capacity(scalar.len() + 2);
    if negative {
        lexeme.push('-');
    }
    lexeme.push_str(if integer_part.is_empty() {
        "0"
    } else {
        integer_part
    });
    if bytes[int_end..].first() == Some(&b'.') {
        lexeme.push('.');
        lexeme.push_str(if fraction.is_empty() { "0" } else { fraction });
    }
    lexeme.push_str(exponent);
    Some(lexeme)
}

/// What the untagged plain scalar `scalar` denotes.
pub(crate) fn resolve(scalar: &str) -> Plain {
    match scalar {
        "" | "~" | "null" | "Null" | "NULL" => return Plain::Null,
        "true" | "True" | "TRUE" => return Plain::Bool(true),
        "false" | "False" | "FALSE" => return Plain::Bool(false),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" | "-.inf" | "-.Inf" | "-.INF"
        | ".nan" | ".NaN" | ".NAN" => return Plain::NonFinite,
        _ => {}
    }
    if let Some(lexeme) = integer(scalar) {
        return Plain::Number(lexeme);
    }
    if !digits_but_not_number(scalar)
        && let Some(lexeme) = float(scalar)
    {
        return Plain::Number(lexeme);
    }
    Plain::String
}

/// Whether a string must be quoted to read back as a string.
pub(crate) fn needs_quotes(text: &str) -> bool {
    resolve(text) != Plain::String || digits_but_not_number(text)
}

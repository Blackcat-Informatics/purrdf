// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What an untagged plain scalar denotes: the one resolver the reader reads
//! with and the writer quotes by, so a string the writer leaves plain is a
//! string the reader reads back.
//!
//! YAML 1.2.2 Core Schema (§10.3.2): signed decimal integers, unsigned octal
//! and hexadecimal integers, decimal floats, booleans and null. Classification
//! borrows source text and never narrows a number to a machine integer.

/// What a plain scalar denotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Plain<'a> {
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A finite number, with a borrowed JSON-normalization plan.
    Number(NumberText<'a>),
    /// `.inf`, `-.inf` or `.nan`, which JSON cannot hold.
    NonFinite,
    /// Text.
    String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Radix {
    Octal,
    Hexadecimal,
}
impl Radix {
    pub(super) const fn bits(self) -> usize {
        match self {
            Self::Octal => 3,
            Self::Hexadecimal => 4,
        }
    }
    pub(super) fn digit(self, byte: u8) -> Option<u8> {
        purrdf_hash::hex::nibble(byte).filter(|digit| usize::from(*digit) < (1 << self.bits()))
    }
}

/// A recognized number's borrowed spelling plan. Classification allocates
/// nothing; native materialization uses the caller's admitted account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumberText<'a> {
    Decimal {
        negative: bool,
        digits: &'a str,
    },
    Radix {
        radix: Radix,
        digits: &'a str,
    },
    Float {
        negative: bool,
        integer: &'a str,
        fraction: Option<&'a str>,
        exponent: &'a str,
    },
}
impl NumberText<'_> {
    pub(crate) fn is_integer(&self) -> bool {
        match self {
            Self::Decimal { .. } | Self::Radix { .. } => true,
            Self::Float {
                fraction, exponent, ..
            } => fraction.is_none() && exponent.is_empty(),
        }
    }
}
/// An integer spelling's JSON lexeme.
fn integer(scalar: &str) -> Option<NumberText<'_>> {
    for (prefix, radix) in [("0x", Radix::Hexadecimal), ("0o", Radix::Octal)] {
        if let Some(digits) = scalar.strip_prefix(prefix) {
            return (!digits.is_empty() && digits.bytes().all(|byte| radix.digit(byte).is_some()))
                .then_some(NumberText::Radix { radix, digits });
        }
    }
    let (negative, unsigned) = match scalar.as_bytes().first() {
        Some(b'-') => (true, &scalar[1..]),
        Some(b'+') => (false, &scalar[1..]),
        _ => (false, scalar),
    };
    if unsigned.is_empty() || !unsigned.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let digits = unsigned.trim_start_matches('0');
    Some(NumberText::Decimal {
        negative,
        digits: if digits.is_empty() { "0" } else { digits },
    })
}

/// A core-schema decimal float spelling's JSON lexeme:
/// `[-+]? ( \. [0-9]+ | [0-9]+ ( \. [0-9]* )? ) ( [eE] [-+]? [0-9]+ )?`.
fn float(scalar: &str) -> Option<NumberText<'_>> {
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
    Some(NumberText::Float {
        negative,
        integer: integer_part.trim_start_matches('0'),
        fraction: (bytes[int_end..].first() == Some(&b'.')).then_some(fraction),
        exponent,
    })
}

/// What the untagged plain scalar `scalar` denotes.
pub(crate) fn resolve(scalar: &str) -> Plain<'_> {
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
    if let Some(lexeme) = float(scalar) {
        return Plain::Number(lexeme);
    }
    Plain::String
}

/// Whether a string must be quoted to read back as a string.
pub(crate) fn needs_quotes(text: &str) -> bool {
    !matches!(resolve(text), Plain::String)
}

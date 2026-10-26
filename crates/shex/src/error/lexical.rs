// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original ShExC scanner reasons and native lexical causes.

use core::fmt;
use purrdf_hash::error::ErrorValue;
use purrdf_iri::{langtag::LanguageTagError, terminals::UcharError};

/// The original lexical construct containing a refused escape or terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexicalContext {
    /// An angle-delimited IRI reference.
    Iri,
    /// A short or long quoted string.
    String,
    /// A semantic action's code body.
    Code,
    /// A slash-delimited regular expression.
    Regex,
}

impl purrdf_hash::error::StructuredError for LexicalContext {
    purrdf_hash::error_visit! {
        Iri => "shex-lex-context-iri" {}, String => "shex-lex-context-string" {},
        Code => "shex-lex-context-code" {}, Regex => "shex-lex-context-regex" {},
    }
}

/// The exact checked operation that refused a repeat-range bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatOperation {
    /// Multiply the accumulated prefix by ten.
    Multiply,
    /// Add the next decimal digit to the scaled prefix.
    Add,
}

impl purrdf_hash::error::StructuredError for RepeatOperation {
    purrdf_hash::error_visit! {
        Multiply => "shex-repeat-multiply" {}, Add => "shex-repeat-add" {},
    }
}

/// A native lexical refusal. Text is retained only when it is original input.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LexicalReason {
    /// ShExC has no triple-term or reifier value token.
    TripleTerm,
    /// No token starts with this original scalar.
    UnexpectedCharacter {
        /// The original input scalar.
        scalar: char,
    },
    /// An IRI reference forbids this original scalar.
    ForbiddenIriCharacter {
        /// The original input scalar.
        scalar: char,
    },
    /// The named construct reached the end of input before its terminator.
    Unterminated {
        /// The original lexical construct.
        context: LexicalContext,
    },
    /// The shared Unicode-escape decoder's original refusal.
    UnicodeEscape {
        /// The original lexical construct.
        context: LexicalContext,
        /// The shared decoder's original cause.
        cause: UcharError,
    },
    /// The original scalar after a backslash, or its absence at end of input.
    BadEscape {
        /// The original lexical construct.
        context: LexicalContext,
        /// The original next scalar, or end of input.
        following: Option<char>,
    },
    /// A short quoted string contains an original raw CR or LF.
    StringNewline {
        /// The original input scalar.
        scalar: char,
    },
    /// The original language body and the shared parser's original refusal.
    LanguageTag {
        /// The owned original language-tag body, without its sigil.
        language: String,
        /// The shared language parser's original cause.
        cause: LanguageTagError,
    },
    /// A blank-node label has no valid first scalar after its sigil.
    BlankNodeLabel {
        /// The original next scalar, or end of input.
        following: Option<char>,
    },
    /// A repeat range has an unexpected next scalar or end of input.
    MalformedRepeat {
        /// The original next scalar, or end of input.
        following: Option<char>,
    },
    /// A repeat range did not close after its upper bound.
    UnterminatedRepeat {
        /// The original next scalar, or end of input.
        following: Option<char>,
    },
    /// The original operands of an overflowing signed repeat-bound operation.
    RepeatOverflow {
        /// The original checked operation.
        operation: RepeatOperation,
        /// The original left operand.
        left: i64,
        /// The original right operand.
        right: i64,
    },
    /// No integer begins at the repeat-bound cursor.
    ExpectedRepeatInteger {
        /// The original next scalar, or end of input.
        following: Option<char>,
    },
    /// A semantic action has no IRI name at its cursor.
    ExpectedActionIri {
        /// The original next scalar, or end of input.
        following: Option<char>,
    },
    /// A consumed word is not a prefixed IRI semantic-action name.
    ActionName {
        /// The original owned word, moved from its token.
        word: String,
    },
    /// A semantic action name is not followed by its code or closing sigil.
    ExpectedActionBody {
        /// The original next scalar, or end of input.
        following: Option<char>,
    },
    /// A code-body percent neither escapes nor closes the body.
    UnescapedActionPercent {
        /// The original scalar after the percent, or end of input.
        following: Option<char>,
    },
    /// A regular expression contains an original raw CR or LF.
    RegexNewline {
        /// The original input scalar.
        scalar: char,
    },
    /// The regular expression has no pattern scalars.
    EmptyRegex,
}

impl purrdf_hash::error::StructuredError for LexicalReason {
    purrdf_hash::error_visit! {
        TripleTerm => "shex-lex-triple-term" {},
        UnexpectedCharacter { scalar } => "shex-lex-unexpected-character" { "scalar" => Character(*scalar) },
        ForbiddenIriCharacter { scalar } => "shex-lex-forbidden-iri-character" { "scalar" => Character(*scalar) },
        Unterminated { context } => "shex-lex-unterminated" { "context" => Record(context) },
        UnicodeEscape { context, cause } => "shex-lex-unicode-escape" { "context" => Record(context), "cause" => Cause(cause) },
        BadEscape { context, following } => "shex-lex-bad-escape" { "context" => Record(context), "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        StringNewline { scalar } => "shex-lex-string-newline" { "scalar" => Character(*scalar) },
        LanguageTag { language, cause } => "shex-lex-language-tag" { "language" => Text(language), "cause" => Cause(cause) },
        BlankNodeLabel { following } => "shex-lex-blank-node-label" { "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        MalformedRepeat { following } => "shex-lex-malformed-repeat" { "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        UnterminatedRepeat { following } => "shex-lex-unterminated-repeat" { "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        RepeatOverflow { operation, left, right } => "shex-lex-repeat-overflow" { "operation" => Record(operation), "left" => Signed(i128::from(*left)), "right" => Signed(i128::from(*right)) },
        ExpectedRepeatInteger { following } => "shex-lex-expected-repeat-integer" { "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        ExpectedActionIri { following } => "shex-lex-expected-action-iri" { "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        ExpectedActionBody { following } => "shex-lex-expected-action-body" { "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        ActionName { word } => "shex-lex-action-name" { "word" => Text(word) },
        UnescapedActionPercent { following } => "shex-lex-unescaped-action-percent" { "following" => Value(following.map_or(ErrorValue::Absent, ErrorValue::Character)) },
        RegexNewline { scalar } => "shex-lex-regex-newline" { "scalar" => Character(*scalar) },
        EmptyRegex => "shex-lex-empty-regex" {},
    }
}

impl fmt::Display for LexicalReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TripleTerm => f.write_str("a ShExC schema has no triple-term value: `<<( s p o )>>` / `<< s p o >>` cannot appear in a value set or node constraint"),
            Self::UnexpectedCharacter { scalar } => write!(f, "unexpected character {scalar:?}"),
            Self::ForbiddenIriCharacter { scalar } => write!(f, "character {scalar:?} is not allowed in an IRI reference"),
            Self::Unterminated { context } => write!(f, "unterminated {}", match context {
                LexicalContext::Iri => "IRI reference", LexicalContext::String => "string literal",
                LexicalContext::Code => "semantic action code", LexicalContext::Regex => "regex",
            }),
            Self::UnicodeEscape { cause, .. } => f.write_str(match cause {
                UcharError::NotAnEscape => "backslash must start a \\u/\\U escape here",
                UcharError::BadHex => "bad \\u/\\U escape (expected hex digits)",
                UcharError::NotAScalar => "\\u/\\U escape is not a Unicode scalar",
            }),
            Self::BadEscape { context: LexicalContext::Code, .. } => f.write_str("bad escape in semantic action code"),
            Self::BadEscape { context, following } => {
                write!(f, "bad {} escape \\", match context {
                    LexicalContext::String => "string", LexicalContext::Regex => "regex",
                    LexicalContext::Iri => "IRI", LexicalContext::Code => "code",
                })?;
                if let Some(scalar) = following { write!(f, "{scalar}")?; }
                Ok(())
            }
            Self::StringNewline { .. } => f.write_str("raw newline in short string literal"),
            Self::LanguageTag { language, cause } => write!(f, "malformed language tag @{language}: {cause} [{}]", cause.diagnostic_code()),
            Self::BlankNodeLabel { .. } => f.write_str("malformed blank node label"),
            Self::MalformedRepeat { .. } => f.write_str("malformed repeat range"),
            Self::UnterminatedRepeat { .. } => f.write_str("unterminated repeat range"),
            Self::RepeatOverflow { .. } => f.write_str("repeat range bound overflows"),
            Self::ExpectedRepeatInteger { .. } => f.write_str("expected integer in repeat range"),
            Self::ActionName { .. } | Self::ExpectedActionIri { .. } => f.write_str("expected IRI after '%'"),
            Self::ExpectedActionBody { .. } => f.write_str("expected '{' or '%' after semantic action name"),
            Self::UnescapedActionPercent { .. } => f.write_str("'%' in semantic action code must be escaped as \\% or close the block"),
            Self::RegexNewline { .. } => f.write_str("raw newline in regex"),
            Self::EmptyRegex => f.write_str("empty regex"),
        }
    }
}

purrdf_lex::error_source!(LexicalReason => {
    Self::UnicodeEscape { cause, .. } => cause,
    Self::LanguageTag { cause, .. } => cause,
});

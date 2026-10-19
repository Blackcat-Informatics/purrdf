// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native grammar and resource refusals of the ECMA parser and matcher.

use std::fmt;

/// The grammar production that rejected a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxRefusal {
    /// A closing parenthesis has no group.
    UnmatchedParenthesis,
    /// A quantifier follows an assertion.
    QuantifiedAssertion,
    /// A modifier appears more than once.
    RepeatedModifier {
        /// Original flag.
        flag: char,
    },
    /// An opening group has no known group form.
    UnknownGroup,
    /// A modifier group names no flags.
    EmptyModifiers,
    /// A group has no closing parenthesis.
    UnterminatedGroup,
    /// An atom was required at the end of the pattern.
    End,
    /// A quantifier has no preceding atom.
    NothingToRepeat,
    /// A reserved grammar character occurs as an unescaped atom.
    LoneCharacter {
        /// Original character.
        character: char,
    },
    /// An opening brace does not begin a quantifier.
    QuantifierStart,
    /// A quantifier has no upper bound after its comma.
    QuantifierBounds,
    /// A quantifier has no closing brace.
    UnterminatedQuantifier,
    /// The upper repetition bound is smaller than the lower bound.
    QuantifierOrder {
        /// Lower bound.
        minimum: u32,
        /// Upper bound.
        maximum: u32,
    },
    /// A backslash has no following character.
    TrailingEscape,
    /// A numeric reference names no group.
    MissingGroup {
        /// Requested group.
        group: u32,
        /// Declared group count.
        groups: u32,
    },
    /// A named reference has no name opener.
    NamedReferenceStart,
    /// A named reference names no group.
    MissingNamedGroup {
        /// Decoded original name.
        name: String,
    },
    /// A control escape has no ASCII letter.
    ControlEscape,
    /// An octal escape occurs in Unicode mode.
    OctalEscape,
    /// A hexadecimal escape has fewer than two hex digits.
    HexEscape,
    /// An escape is not part of the Unicode-mode grammar.
    UnknownEscape {
        /// Original escaped character.
        character: char,
    },
    /// A braced Unicode escape has no valid scalar-code-point spelling.
    UnicodeCodepoint {
        /// Accumulated value.
        value: u32,
        /// Number of digits consumed.
        digits: usize,
    },
    /// A Unicode escape has neither four digits nor a braced value.
    UnicodeEscape,
    /// A property escape has no opening brace.
    PropertyStart,
    /// A property name has a forbidden character.
    PropertyCharacter {
        /// Original character.
        character: char,
    },
    /// A property escape has no closing brace.
    UnterminatedProperty,
    /// A property is absent from ECMA's closed property table.
    UnknownProperty {
        /// Original property body.
        body: String,
    },
    /// A group name has no closing delimiter.
    UnterminatedGroupName,
    /// A group name contains an escape other than Unicode.
    GroupNameEscape,
    /// A group name contains a lone surrogate.
    GroupNameSurrogate {
        /// Original code unit.
        code: u32,
    },
    /// A group name contains a forbidden identifier character.
    GroupNameCharacter {
        /// Original character.
        character: char,
        /// Whether this was the initial character.
        first: bool,
    },
    /// A group name is empty.
    EmptyGroupName,
    /// A character class has no closing bracket.
    UnterminatedClass,
    /// A class escape is used as a range endpoint.
    ClassEscapeRange {
        /// Original source spelling of the range.
        range: String,
    },
    /// Range endpoints are reversed.
    ClassRangeOrder {
        /// Original lower endpoint.
        low: u32,
        /// Original upper endpoint.
        high: u32,
    },
    /// A character class contains a backreference.
    ClassBackreference,
    /// The non-boundary assertion occurs in a class.
    ClassBoundary,
    /// Two simultaneously reachable captures have the same name.
    DuplicateGroupName {
        /// Original decoded name.
        name: String,
    },
}

impl purrdf_hash::error::StructuredError for SyntaxRefusal {
    purrdf_hash::error_visit! {
        UnmatchedParenthesis => "ecma-unmatched-parenthesis" {},
        QuantifiedAssertion => "ecma-quantified-assertion" {},
        RepeatedModifier { flag } => "ecma-repeated-modifier" { "flag" => Character(*flag) },
        UnknownGroup => "ecma-unknown-group" {},
        EmptyModifiers => "ecma-empty-modifiers" {},
        UnterminatedGroup => "ecma-unterminated-group" {},
        End => "ecma-pattern-end" {},
        NothingToRepeat => "ecma-nothing-to-repeat" {},
        LoneCharacter { character } => "ecma-lone-character" { "character" => Character(*character) },
        QuantifierStart => "ecma-quantifier-start" {},
        QuantifierBounds => "ecma-quantifier-bounds" {},
        UnterminatedQuantifier => "ecma-unterminated-quantifier" {},
        QuantifierOrder { minimum, maximum } => "ecma-quantifier-order" { "minimum" => Unsigned(u128::from(*minimum)), "maximum" => Unsigned(u128::from(*maximum)) },
        TrailingEscape => "ecma-trailing-escape" {},
        MissingGroup { group, groups } => "ecma-missing-group" { "group" => Unsigned(u128::from(*group)), "groups" => Unsigned(u128::from(*groups)) },
        NamedReferenceStart => "ecma-named-reference-start" {},
        MissingNamedGroup { name } => "ecma-missing-named-group" { "name" => Text(name) },
        ControlEscape => "ecma-control-escape" {},
        OctalEscape => "ecma-octal-escape" {},
        HexEscape => "ecma-hex-escape" {},
        UnknownEscape { character } => "ecma-unknown-escape" { "character" => Character(*character) },
        UnicodeCodepoint { value, digits } => "ecma-unicode-codepoint" { "value" => Unsigned(u128::from(*value)), "digits" => Unsigned(*digits as u128) },
        UnicodeEscape => "ecma-unicode-escape" {},
        PropertyStart => "ecma-property-start" {},
        PropertyCharacter { character } => "ecma-property-character" { "character" => Character(*character) },
        UnterminatedProperty => "ecma-unterminated-property" {},
        UnknownProperty { body } => "ecma-unknown-property" { "body" => Text(body) },
        UnterminatedGroupName => "ecma-unterminated-group-name" {},
        GroupNameEscape => "ecma-group-name-escape" {},
        GroupNameSurrogate { code } => "ecma-group-name-surrogate" { "code" => Unsigned(u128::from(*code)) },
        GroupNameCharacter { character, first } => "ecma-group-name-character" { "character" => Character(*character), "first" => Boolean(*first) },
        EmptyGroupName => "ecma-empty-group-name" {},
        UnterminatedClass => "ecma-unterminated-class" {},
        ClassEscapeRange { range } => "ecma-class-escape-range" { "range" => Text(range) },
        ClassRangeOrder { low, high } => "ecma-class-range-order" { "low" => Unsigned(u128::from(*low)), "high" => Unsigned(u128::from(*high)) },
        ClassBackreference => "ecma-class-backreference" {},
        ClassBoundary => "ecma-class-boundary" {},
        DuplicateGroupName { name } => "ecma-duplicate-group-name" { "name" => Text(name) },
    }
}

impl fmt::Display for SyntaxRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedParenthesis => f.write_str("unmatched `)`"),
            Self::QuantifiedAssertion => {
                f.write_str("nothing to repeat: an assertion cannot be quantified")
            }
            Self::RepeatedModifier { .. } => f.write_str("a modifier flag is repeated"),
            Self::UnknownGroup => f.write_str("invalid group: `(?` must begin a known group form"),
            Self::EmptyModifiers => f.write_str("a modifier group must name at least one flag"),
            Self::UnterminatedGroup => f.write_str("unterminated group"),
            Self::End => f.write_str("unexpected end of pattern"),
            Self::NothingToRepeat => f.write_str("nothing to repeat"),
            Self::LoneCharacter { character } => write!(
                f,
                "a lone `{character}` is not a pattern character under the `u` flag"
            ),
            Self::QuantifierStart => {
                f.write_str("a `{` must begin a `{n}`, `{n,}` or `{n,m}` quantifier")
            }
            Self::QuantifierBounds => f.write_str("malformed `{n,m}` quantifier"),
            Self::UnterminatedQuantifier => f.write_str("unterminated `{…}` quantifier"),
            Self::QuantifierOrder { .. } => {
                f.write_str("numbers out of order in `{n,m}` quantifier")
            }
            Self::TrailingEscape => f.write_str("`\\` at end of pattern"),
            Self::MissingGroup { .. } => {
                f.write_str("backreference to a group the pattern does not have")
            }
            Self::NamedReferenceStart => f.write_str("`\\k` must name a group under the `u` flag"),
            Self::MissingNamedGroup { .. } => {
                f.write_str("`\\k<…>` names a group the pattern does not have")
            }
            Self::ControlEscape => f.write_str("`\\c` must be followed by an ASCII letter"),
            Self::OctalEscape => f.write_str("octal escapes are not allowed under the `u` flag"),
            Self::HexEscape => f.write_str("`\\x` must be followed by two hex digits"),
            Self::UnknownEscape { character } => {
                write!(f, "`\\{character}` is not an escape under the `u` flag")
            }
            Self::UnicodeCodepoint { .. } => {
                f.write_str("`\\u{…}` must hold a code point no greater than 10FFFF")
            }
            Self::UnicodeEscape => {
                f.write_str("`\\u` must be followed by four hex digits or `{…}`")
            }
            Self::PropertyStart => f.write_str("`\\p` must be followed by `{`"),
            Self::PropertyCharacter { .. } => {
                f.write_str("a property name may hold only ASCII letters, digits and `_`")
            }
            Self::UnterminatedProperty => f.write_str("unterminated `\\p{…}`"),
            Self::UnknownProperty { body } => {
                write!(f, "`\\p{{{body}}}` is not a property ECMA-262 lists")
            }
            Self::UnterminatedGroupName => f.write_str("unterminated group name"),
            Self::GroupNameEscape => f.write_str("only `\\u` escapes may appear in a group name"),
            Self::GroupNameSurrogate { .. } => {
                f.write_str("a group name may not hold a lone surrogate")
            }
            Self::GroupNameCharacter { .. } => {
                f.write_str("a group name must be an ECMAScript IdentifierName")
            }
            Self::EmptyGroupName => f.write_str("a group name may not be empty"),
            Self::UnterminatedClass => f.write_str("unterminated character class"),
            Self::ClassEscapeRange { .. } => {
                f.write_str("a class escape cannot bound a range under the `u` flag")
            }
            Self::ClassRangeOrder { .. } => f.write_str("range out of order in character class"),
            Self::ClassBackreference => {
                f.write_str("a backreference cannot appear in a character class")
            }
            Self::ClassBoundary => f.write_str("`\\B` is not a class escape"),
            Self::DuplicateGroupName { name } => write!(f, "duplicate capture group name `{name}`"),
        }
    }
}

impl std::error::Error for SyntaxRefusal {}

/// The original resource boundary and quantities that stopped a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternLimit {
    /// Parser or emitter nesting.
    GroupNesting {
        /// Depth at refusal.
        depth: usize,
        /// Maximum group depth.
        maximum: usize,
    },
    /// Decimal repetition/backreference accumulation overflow.
    Decimal {
        /// Value before the refused digit.
        value: u32,
        /// Original digit.
        digit: u32,
    },
    /// Live VM instruction/capture storage.
    Slots {
        /// Already live slots.
        used: usize,
        /// Additional slots needed.
        extra: usize,
        /// Maximum live slots.
        maximum: usize,
    },
    /// The caller's instruction/state limits.
    Matcher {
        /// Instructions remaining.
        steps: u64,
        /// Active alternative states.
        active_states: usize,
        /// Maximum alternative states.
        maximum_states: usize,
    },
}

impl purrdf_hash::error::StructuredError for PatternLimit {
    purrdf_hash::error_visit! {
        GroupNesting { depth, maximum } => "ecma-group-limit" { "depth" => Unsigned(*depth as u128), "maximum" => Unsigned(*maximum as u128) },
        Decimal { value, digit } => "ecma-decimal-limit" { "value" => Unsigned(u128::from(*value)), "digit" => Unsigned(u128::from(*digit)), "maximum" => Unsigned(u128::from(u32::MAX)) },
        Slots { used, extra, maximum } => "ecma-slot-limit" { "used" => Unsigned(*used as u128), "extra" => Unsigned(*extra as u128), "maximum" => Unsigned(*maximum as u128) },
        Matcher { steps, active_states, maximum_states } => "ecma-matcher-limit" { "steps" => Unsigned(u128::from(*steps)), "active_states" => Unsigned(*active_states as u128), "maximum_states" => Unsigned(*maximum_states as u128) },
    }
}

impl fmt::Display for PatternLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GroupNesting { maximum, .. } => write!(f, "group nesting exceeds {maximum}"),
            Self::Decimal { .. } => f.write_str("decimal repetition or backreference exceeds u32"),
            Self::Slots { .. } | Self::Matcher { .. } => {
                f.write_str("ECMA matcher budget exhausted")
            }
        }
    }
}

impl std::error::Error for PatternLimit {}

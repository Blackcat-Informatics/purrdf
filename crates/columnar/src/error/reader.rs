// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original RDF reconstruction operands retained without diagnostic strings.

use core::{fmt, num::TryFromIntError, str::Utf8Error};

/// Why decoded Columnar rows violate the RDF reconstruction contract.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReaderReason {
    /// A triple-term closure contains a cycle.
    CyclicTriple,
    /// A term's named-graph flag is outside the closed Boolean representation.
    NamedGraphFlag {
        /// Original table row.
        row: usize,
        /// Original signed discriminator.
        flag: i64,
    },
    /// A literal or triple term was marked as a graph name.
    GraphName {
        /// Original table row.
        row: usize,
    },
    /// Dictionary ids are not dense and zero-based.
    DenseId {
        /// Original table row and required id.
        row: usize,
        /// Original stored id.
        id: i64,
    },
    /// A blank-node scope exceeds its unsigned 32-bit domain.
    Scope {
        /// Original table row.
        row: usize,
        /// Original scope value before narrowing.
        scope: i64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
    /// Dictionary resolution left an unresolved record.
    UnresolvedTerm,
    /// A literal's datatype record is not an IRI.
    LiteralDatatype {
        /// Original literal row.
        row: usize,
    },
    /// A language tag is empty or is not identity-folded.
    LanguageTag {
        /// Original literal row.
        row: usize,
        /// Original stored language tag.
        language: String,
    },
    /// A language literal names a different datatype from the required one.
    LanguageDatatype {
        /// Original literal row.
        row: usize,
        /// Original datatype IRI.
        actual: String,
        /// Required RDF datatype IRI.
        expected: &'static str,
    },
    /// A base direction is present without a language tag.
    DirectionWithoutLanguage {
        /// Original literal row.
        row: usize,
    },
    /// A blob digest is not UTF-8.
    DigestUtf8 {
        /// Original blob row.
        row: usize,
        /// Original UTF-8 coordinates.
        cause: Utf8Error,
        /// Original rejected decoded cell, transferred without copying.
        bytes: Vec<u8>,
    },
    /// A blob digest does not have the required hexadecimal shape.
    DigestShape {
        /// Original blob row.
        row: usize,
        /// Original rejected decoded cell, transferred without copying.
        bytes: Vec<u8>,
    },
    /// A valid hexadecimal digest uses uppercase letters.
    DigestCase {
        /// Original blob row.
        row: usize,
        /// Original rejected decoded cell, transferred without copying.
        bytes: Vec<u8>,
    },
    /// Blob rows are not in strict digest order.
    DigestOrder,
    /// A reifier tuple has no corresponding triple-term dictionary entry.
    MissingTriple,
    /// A required logical column has a different physical representation.
    ColumnType {
        /// Original table.
        table: crate::Table,
        /// Original column index.
        column: usize,
        /// Required physical representation.
        expected: crate::PhysicalType,
    },
    /// A required row value is absent.
    Null {
        /// Original table row.
        row: usize,
    },
    /// A text cell is not UTF-8.
    RowUtf8 {
        /// Original table row.
        row: usize,
        /// Original UTF-8 coordinates.
        cause: Utf8Error,
    },
    /// A stored term reference cannot be represented as an index.
    TermId {
        /// Original table row.
        row: usize,
        /// Original signed id.
        value: i64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
    /// A term reference is outside the decoded dictionary.
    TermReference {
        /// Original table row.
        row: usize,
        /// Original decoded id.
        id: usize,
        /// Actual dictionary row count.
        term_count: usize,
    },
    /// A stored graph reference cannot be represented as an index.
    GraphId {
        /// Original table row.
        row: usize,
        /// Original signed id.
        value: i64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
    /// A graph reference is outside the decoded dictionary.
    MissingGraph {
        /// Original table row.
        row: usize,
        /// Original decoded id.
        id: usize,
        /// Actual dictionary row count.
        term_count: usize,
    },
    /// A graph reference names a term without its graph declaration.
    UndeclaredGraph {
        /// Original table row.
        row: usize,
        /// Original decoded id.
        id: usize,
    },
    /// A cell required to be absent for this term kind is present.
    ExpectedNull {
        /// Original table row.
        row: usize,
    },
    /// Canonical logical rows are not strictly increasing.
    RowOrder,
}

impl purrdf_hash::error::StructuredError for ReaderReason {
    purrdf_hash::error_visit! {
        CyclicTriple => "columnar-cyclic-triple" {},
        NamedGraphFlag { row, flag } => "columnar-named-graph-flag" {
            "row" => Unsigned(*row as u128), "flag" => Signed(i128::from(*flag)),
        },
        GraphName { row } => "columnar-graph-name" { "row" => Unsigned(*row as u128) },
        DenseId { row, id } => "columnar-dense-id" {
            "row" => Unsigned(*row as u128), "id" => Signed(i128::from(*id)),
        },
        Scope { row, scope, cause } => "columnar-scope" {
            "row" => Unsigned(*row as u128), "scope" => Signed(i128::from(*scope)),
            "cause" => Cause(cause),
        },
        UnresolvedTerm => "columnar-unresolved-term" {},
        LiteralDatatype { row } => "columnar-literal-datatype" { "row" => Unsigned(*row as u128) },
        LanguageTag { row, language } => "columnar-language-tag" {
            "row" => Unsigned(*row as u128), "language" => Text(language),
        },
        LanguageDatatype { row, actual, expected } => "columnar-language-datatype" {
            "row" => Unsigned(*row as u128), "actual" => Iri(actual), "expected" => Iri(expected),
        },
        DirectionWithoutLanguage { row } => "columnar-direction-without-language" {
            "row" => Unsigned(*row as u128),
        },
        DigestUtf8 { row, cause, bytes } => "columnar-digest-utf8" {
            "row" => Unsigned(*row as u128), "cause" => Cause(cause), "bytes" => Bytes(bytes),
        },
        DigestShape { row, bytes } => "columnar-digest-shape" { "row" => Unsigned(*row as u128), "bytes" => Bytes(bytes) },
        DigestCase { row, bytes } => "columnar-digest-case" { "row" => Unsigned(*row as u128), "bytes" => Bytes(bytes) },
        DigestOrder => "columnar-digest-order" {},
        MissingTriple => "columnar-missing-triple" {},
        ColumnType { table, column, expected } => "columnar-column-type" {
            "table" => Code(table.name()), "column" => Unsigned(*column as u128),
            "expected" => Record(expected),
        },
        Null { row } => "columnar-null" { "row" => Unsigned(*row as u128) },
        RowUtf8 { row, cause } => "columnar-row-utf8" {
            "row" => Unsigned(*row as u128), "cause" => Cause(cause),
        },
        TermId { row, value, cause } => "columnar-term-id" {
            "row" => Unsigned(*row as u128), "value" => Signed(i128::from(*value)),
            "cause" => Cause(cause),
        },
        TermReference { row, id, term_count } => "columnar-term-reference" {
            "row" => Unsigned(*row as u128), "id" => Unsigned(*id as u128),
            "term_count" => Unsigned(*term_count as u128),
        },
        GraphId { row, value, cause } => "columnar-graph-id" {
            "row" => Unsigned(*row as u128), "value" => Signed(i128::from(*value)),
            "cause" => Cause(cause),
        },
        MissingGraph { row, id, term_count } => "columnar-missing-graph" {
            "row" => Unsigned(*row as u128), "id" => Unsigned(*id as u128),
            "term_count" => Unsigned(*term_count as u128),
        },
        UndeclaredGraph { row, id } => "columnar-undeclared-graph" {
            "row" => Unsigned(*row as u128), "id" => Unsigned(*id as u128),
        },
        ExpectedNull { row } => "columnar-expected-null" { "row" => Unsigned(*row as u128) },
        RowOrder => "columnar-row-order" {},
    }
}

impl fmt::Display for ReaderReason {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CyclicTriple => output.write_str("cyclic triple-term reference"),
            Self::NamedGraphFlag { row, flag } => {
                write!(output, "row {row} has flag {flag}, expected 0 or 1")
            }
            Self::GraphName { row } => write!(
                output,
                "row {row} marks a literal or triple term as a graph name"
            ),
            Self::DenseId { row, id } => write!(
                output,
                "row {row} has id {id}; ids must be dense and zero-based"
            ),
            Self::Scope { row, scope, .. } => {
                write!(output, "row {row} has out-of-range scope {scope}")
            }
            Self::UnresolvedTerm => output.write_str("unresolved term record"),
            Self::LiteralDatatype { row } => {
                write!(output, "literal row {row} references a non-IRI datatype")
            }
            Self::LanguageTag { row, .. } => {
                write!(output, "literal row {row} has a non-canonical language tag")
            }
            Self::LanguageDatatype { row, expected, .. } => {
                write!(output, "language literal row {row} must use {expected}")
            }
            Self::DirectionWithoutLanguage { row } => write!(
                output,
                "literal row {row} has a direction without a language tag"
            ),
            Self::DigestUtf8 { row, .. } => write!(output, "row {row} is not UTF-8"),
            Self::DigestShape { row, .. } => {
                write!(output, "row {row} is not a 64-digit SHA-256 hex value")
            }
            Self::DigestCase { row, .. } => {
                write!(output, "row {row} is not canonical lowercase hexadecimal")
            }
            Self::DigestOrder => output.write_str("digests are not strictly increasing"),
            Self::MissingTriple => {
                output.write_str("component tuple has no corresponding triple term")
            }
            Self::ColumnType {
                table,
                column,
                expected,
            } => {
                let name = match expected {
                    crate::PhysicalType::Int64 => "INT64",
                    crate::PhysicalType::ByteArray => "BYTE_ARRAY",
                };
                write!(output, "{}.{column} is not {name}", table.name())
            }
            Self::Null { row } => write!(output, "required value is null at row {row}"),
            Self::RowUtf8 { row, .. } => write!(output, "row {row} is not valid UTF-8"),
            Self::TermId { row, value, .. } => {
                write!(output, "row {row} has negative term id {value}")
            }
            Self::TermReference {
                row,
                id,
                term_count,
            } => write!(
                output,
                "row {row} references term {id}, but dictionary has {term_count} rows"
            ),
            Self::GraphId { row, value, .. } => write!(output, "row {row} has negative id {value}"),
            Self::MissingGraph { row, id, .. } => {
                write!(output, "row {row} references missing term {id}")
            }
            Self::UndeclaredGraph { row, id } => write!(
                output,
                "row {row} references term {id} not marked as a named graph"
            ),
            Self::ExpectedNull { row } => {
                write!(output, "value must be null for term kind at row {row}")
            }
            Self::RowOrder => output.write_str("rows are not strictly increasing"),
        }
    }
}

purrdf_lex::error_source!(ReaderReason => {
    Self::Scope { cause, .. } => cause,
    Self::DigestUtf8 { cause, .. } => cause,
    Self::RowUtf8 { cause, .. } => cause,
    Self::TermId { cause, .. } => cause,
    Self::GraphId { cause, .. } => cause,
});

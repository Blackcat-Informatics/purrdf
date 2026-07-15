// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The closed five-table columnar schema.

/// A Parquet physical type used by the columnar contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalType {
    /// A signed 64-bit integer.
    Int64,
    /// An arbitrary byte string.
    ByteArray,
}

/// Whether a flat column is required or nullable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repetition {
    /// Every row carries exactly one value.
    Required,
    /// A row carries zero or one value, encoded through definition levels.
    Optional,
}

/// One primitive column in a [`TableSchema`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnSchema {
    /// Stable field name in the Parquet schema.
    pub name: &'static str,
    /// Parquet physical storage type.
    pub physical_type: PhysicalType,
    /// Required or optional cardinality.
    pub repetition: Repetition,
    /// Whether a BYTE_ARRAY column carries the standard UTF8 annotation.
    pub utf8: bool,
}

impl ColumnSchema {
    const fn int64(name: &'static str, repetition: Repetition) -> Self {
        Self {
            name,
            physical_type: PhysicalType::Int64,
            repetition,
            utf8: false,
        }
    }

    const fn bytes(name: &'static str, repetition: Repetition, utf8: bool) -> Self {
        Self {
            name,
            physical_type: PhysicalType::ByteArray,
            repetition,
            utf8,
        }
    }
}

/// The schema of one logical table/file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableSchema {
    /// Logical table name, also used in file metadata.
    pub name: &'static str,
    /// Deterministic output filename.
    pub file_name: &'static str,
    /// Primitive columns in schema order.
    pub columns: &'static [ColumnSchema],
}

/// One member of the closed five-table dataset projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Table {
    /// Unified RDF term dictionary.
    Terms,
    /// Base RDF quad set.
    Quads,
    /// RDF 1.2 reifier bindings.
    Reifiers,
    /// RDF 1.2 statement annotations.
    Annotations,
    /// Content-addressed blob payloads.
    Blobs,
}

impl Table {
    /// Every table in canonical dependency order.
    pub const ALL: [Self; 5] = [
        Self::Terms,
        Self::Quads,
        Self::Reifiers,
        Self::Annotations,
        Self::Blobs,
    ];

    /// The normative schema for this table.
    pub const fn schema(self) -> &'static TableSchema {
        match self {
            Self::Terms => &TERMS,
            Self::Quads => &QUADS,
            Self::Reifiers => &REIFIERS,
            Self::Annotations => &ANNOTATIONS,
            Self::Blobs => &BLOBS,
        }
    }

    /// The stable logical table name.
    pub const fn name(self) -> &'static str {
        self.schema().name
    }

    /// The stable Parquet filename.
    pub const fn file_name(self) -> &'static str {
        self.schema().file_name
    }
}

const TERMS_COLUMNS: &[ColumnSchema] = &[
    ColumnSchema::int64("id", Repetition::Required),
    ColumnSchema::int64("kind", Repetition::Required),
    ColumnSchema::bytes("lex", Repetition::Optional, true),
    ColumnSchema::int64("datatype", Repetition::Optional),
    ColumnSchema::bytes("lang", Repetition::Optional, true),
    ColumnSchema::int64("direction", Repetition::Optional),
    ColumnSchema::int64("scope", Repetition::Optional),
    ColumnSchema::int64("triple_s", Repetition::Optional),
    ColumnSchema::int64("triple_p", Repetition::Optional),
    ColumnSchema::int64("triple_o", Repetition::Optional),
    ColumnSchema::int64("named_graph", Repetition::Required),
];

const QUADS_COLUMNS: &[ColumnSchema] = &[
    ColumnSchema::int64("s", Repetition::Required),
    ColumnSchema::int64("p", Repetition::Required),
    ColumnSchema::int64("o", Repetition::Required),
    ColumnSchema::int64("g", Repetition::Optional),
];

const REIFIERS_COLUMNS: &[ColumnSchema] = &[
    ColumnSchema::int64("reifier", Repetition::Required),
    ColumnSchema::int64("s", Repetition::Required),
    ColumnSchema::int64("p", Repetition::Required),
    ColumnSchema::int64("o", Repetition::Required),
    ColumnSchema::int64("g", Repetition::Optional),
];

const ANNOTATIONS_COLUMNS: &[ColumnSchema] = &[
    ColumnSchema::int64("reifier", Repetition::Required),
    ColumnSchema::int64("predicate", Repetition::Required),
    ColumnSchema::int64("value", Repetition::Required),
    ColumnSchema::int64("g", Repetition::Optional),
];

const BLOBS_COLUMNS: &[ColumnSchema] = &[
    ColumnSchema::bytes("digest", Repetition::Required, true),
    ColumnSchema::bytes("bytes", Repetition::Required, false),
];

const TERMS: TableSchema = TableSchema {
    name: "terms",
    file_name: "terms.parquet",
    columns: TERMS_COLUMNS,
};

const QUADS: TableSchema = TableSchema {
    name: "quads",
    file_name: "quads.parquet",
    columns: QUADS_COLUMNS,
};

const REIFIERS: TableSchema = TableSchema {
    name: "reifiers",
    file_name: "reifiers.parquet",
    columns: REIFIERS_COLUMNS,
};

const ANNOTATIONS: TableSchema = TableSchema {
    name: "annotations",
    file_name: "annotations.parquet",
    columns: ANNOTATIONS_COLUMNS,
};

const BLOBS: TableSchema = TableSchema {
    name: "blobs",
    file_name: "blobs.parquet",
    columns: BLOBS_COLUMNS,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn five_table_names_and_files_are_unique() {
        assert_eq!(Table::ALL.len(), 5);
        assert_eq!(
            Table::ALL
                .iter()
                .map(|table| table.name())
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
        assert_eq!(
            Table::ALL
                .iter()
                .map(|table| table.file_name())
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
    }

    #[test]
    fn schema_uses_only_the_closed_physical_type_set() {
        for table in Table::ALL {
            assert!(!table.schema().columns.is_empty());
            for column in table.schema().columns {
                assert!(matches!(
                    column.physical_type,
                    PhysicalType::Int64 | PhysicalType::ByteArray
                ));
                assert!(!column.utf8 || column.physical_type == PhysicalType::ByteArray);
            }
        }
    }
}

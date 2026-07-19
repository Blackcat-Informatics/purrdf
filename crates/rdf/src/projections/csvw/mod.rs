// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! CSVW annotated tables, normative RDF conversion, and exact RDF 1.2 packages.

mod config;
mod engine;
mod exact;
mod input;
mod metadata;
mod model;
mod rdf;
mod table;
mod terms;
mod writer;

pub use config::{CsvwConfig, CsvwContext, CsvwMode, CsvwVocabulary};
pub use engine::{CsvwReadOutcome, read_csvw};
pub use exact::{CsvwExactProjection, CsvwExactReadOutcome, project_csvw_exact, read_csvw_exact};
pub use input::{CsvwAction, CsvwInput, CsvwWarning, CsvwWarningKind};
pub use model::{
    CsvwAnnotations, CsvwCell, CsvwColumn, CsvwDatatype, CsvwDatatypeFormat, CsvwDialect,
    CsvwForeignKey, CsvwInheritedProperties, CsvwNaturalLanguage, CsvwNumericFormat, CsvwReference,
    CsvwRow, CsvwSchema, CsvwTable, CsvwTableDirection, CsvwTableGroup, CsvwTextDirection,
    CsvwTransformation, CsvwTrim, CsvwValue,
};
pub use terms::{
    CSVW_TERMS_PROFILE, CsvwTermsCardinality, CsvwTermsColumn, CsvwTermsConfig,
    CsvwTermsGraphSelection, CsvwTermsIdentityColumn, CsvwTermsLimits, CsvwTermsProjection,
    CsvwTermsReport, CsvwTermsSelector, CsvwTermsTable, CsvwTermsValueMode, project_csvw_terms,
};
pub use writer::{
    CsvwMappedTableGroup, CsvwRdfTableMapping, CsvwWriteOutcome, CsvwWritePlan, project_csvw,
    write_csvw,
};

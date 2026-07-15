// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PurRDF's first-party, byte-deterministic Parquet bridge.
//!
//! The codec projects an RDF 1.2 [`purrdf_core::DatasetView`] plus a
//! [`purrdf_core::ContentStore`] into the fixed five-table schema exposed by
//! [`schema`]. Its deliberately narrow Parquet subset keeps the implementation
//! wasm-clean and auditable: INT64/BYTE_ARRAY columns, flat OPTIONAL fields,
//! PLAIN values, RLE definition levels, Data Page V2, and UNCOMPRESSED or ZSTD
//! page bodies.
//!
//! No vocabulary is built into this crate. Every IRI is carried from the caller's
//! dataset verbatim.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

mod compact;
pub mod error;
mod files;
mod parquet;
mod reader;
pub mod schema;
mod writer;

pub use error::ColumnarError;
pub use files::ParquetFiles;
pub use parquet::Compression;
pub use reader::{ColumnarRead, read};
pub use schema::{ColumnSchema, PhysicalType, Repetition, Table, TableSchema};
pub use writer::{ColumnarWrite, write};

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lossless MIME-to-RDF over unchanged caller-owned octets and the kernel's
//! single byte-cover law. Select a [`Profile`] explicitly, [`analyze`] original
//! bytes, [`project`] ordered occurrences, and [`decode_document`] only after
//! complete cover and metadata validation. Invalid message syntax is retained
//! as typed [`Problem`] occurrences; repairs and charset guesses are never
//! substituted for source bytes. Flat parts and iterative parsing impose no
//! compiled depth/count/size ceiling. Every resource maximum is caller policy.

mod decode;
mod error;
mod model;
mod parse;
mod profile;
mod project;
mod transfer;

pub use decode::decode_document;
pub use error::MimeError;
pub use model::{
    DecodedAttachment, Document, Header, MediaType, Part, Problem, ProblemKind, SourceDocument,
    Structure, StructureKind, TransferEncoding,
};
pub use parse::analyze;
pub use profile::{Limits, Profile, STANDARD_NAMESPACE, Vocabulary};
pub use project::project;

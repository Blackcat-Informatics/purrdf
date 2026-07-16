// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, caller-configured RDF 1.2 projection foundations.
//!
//! Projection codecs share one bounded in-memory package, one durable RDF term
//! representation, one typed error surface, and one set of escaping/identity
//! primitives. Filesystem and network access stay outside this module, so the same
//! engine runs unchanged in native, WebAssembly, Python, and C hosts.

mod error;
mod package;
mod term;
mod util;

pub use error::{ProjectionError, ProjectionErrorKind};
pub use package::{ProjectionLimits, ProjectionPackage};
pub use term::{ProjectionDirection, ProjectionTerm};
pub use util::{
    escape_cypher_identifier, escape_cypher_string, escape_xml_attribute, escape_xml_text,
    stable_identifier, validate_absolute_iri,
};

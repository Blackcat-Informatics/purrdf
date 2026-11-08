#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Re-exports of std traits or local reimplementations if std is not available

pub use std::io::{Error, ErrorKind, Read, Write};

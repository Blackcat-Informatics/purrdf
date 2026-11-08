#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Encoding agnostic ways to read and write binary data

mod bit_reader;
mod bit_reader_reverse;
mod bit_writer;

pub(crate) use bit_reader::BitReader;
pub use bit_reader::GetBitsError;
// `pub` only when `bench_internals` is active so `crate::testing` can
// re-export it for criterion benchmarks; `pub(crate)` otherwise to
// keep the type out of the public API in normal builds.

pub(crate) use bit_reader_reverse::BitReaderReversed;
pub(crate) use bit_writer::*;

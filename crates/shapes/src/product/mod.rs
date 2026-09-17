// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The prepared-product codec for SHACL: the admission boundary where PurRDF
//! decides whether a sequence of bytes is a prepared shapes product it will
//! execute.
//!
//! # Why an admission boundary exists at all
//!
//! A prepared product is compiled once and then handed around — cached on disk,
//! shipped between processes, restored after a restart. By the time the bytes
//! come back they are **untrusted**: nothing in them is evidence of its own
//! provenance, and the validator that loads them cannot assume it produced them,
//! produced them at this version, or produced them against the same
//! caller-supplied configuration. Decoding is therefore not parsing — it is
//! admission. Every field is a claim the loader has to check before any of it
//! reaches the validator.
//!
//! # Why a refusal names a dimension
//!
//! A refusal here is itself a claim, so it carries the *exact* thing that failed:
//! [`error::ProductDimension`] is the closed set of ways a candidate product can
//! fail admission, and [`error::ShapesProductError`] pairs one of those with a
//! message that names the fix rather than only the fault. A caller can branch on
//! the dimension — a [`Malformed`] product is a corrupt cache to discard and
//! rebuild, a [`FormatVersion`] mismatch is a stale artifact to recompile, a
//! [`FunctionRegistry`] mismatch is a *configuration* error in the caller's own
//! code that rebuilding will not fix — and each of those is a different action.
//!
//! The rejected alternative was a single opaque error carrying only a string.
//! It types perfectly well and it is the thing this module exists to avoid: it
//! collapses "these bytes are garbage" and "you supplied a different function
//! registry than the one this product was prepared against" into one outcome, so
//! the only available recovery is the pessimistic one. Callers then do the
//! predictable thing and match on substrings of the message, which converts
//! every wording improvement into a silent behaviour change downstream.
//!
//! The failure this design prevents is the dangerous half of that collapse:
//! admitting a product whose *identity* no longer matches the environment
//! executing it. A product prepared under one vocabulary, prefix map, or
//! constraint-component set encodes decisions that are only correct under those
//! inputs. Loading it under different ones does not crash — it validates, and
//! quietly returns a report for a shapes graph nobody asked about. Refusing on a
//! named dimension is what makes that mismatch loud at the boundary instead of
//! invisible in the results.
//!
//! Nothing in this module touches the filesystem, a clock, a thread, or a source
//! of randomness: it names outcomes over caller-supplied bytes and stays
//! `wasm32-unknown-unknown` compatible.
//!
//! [`Malformed`]: error::ProductDimension::Malformed
//! [`FormatVersion`]: error::ProductDimension::FormatVersion
//! [`FunctionRegistry`]: error::ProductDimension::FunctionRegistry

// The AST codec is `pub(crate)` and its caller — the container writer/reader that
// frames this section — is a later stage, so in a NON-TEST build nothing in the
// crate calls it yet and every item is transitively unused. The allow is
// deliberately `not(test)` only: the test build exercises the whole surface, so
// genuinely unreachable code still fails there, and this exemption disappears the
// moment the container lands.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the AST codec's in-crate caller is the container stage; the test build exercises \
                  every item, so unreachable code is still caught"
    )
)]
pub(crate) mod ast;
// Same exemption, same reason, same expiry as `ast` above: the dataset section's
// in-crate caller is the container stage, so in a NON-TEST build nothing calls
// `encode_dataset`/`open_dataset`/`certify_dataset` yet. The test build exercises
// all three, so genuinely unreachable code still fails there.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the dataset section's in-crate caller is the container stage; the test build \
                  exercises every item, so unreachable code is still caught"
    )
)]
pub(crate) mod dataset;
pub mod error;
// Same exemption, same reason, same expiry as `ast` and `dataset` above: the
// identity's in-crate caller is the container stage, so in a NON-TEST build nothing
// calls `build_identity`/`check_identity`/`class_catalog_digest` yet. The test build
// exercises all three, so genuinely unreachable code is still caught.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the identity's in-crate caller is the container stage; the test build exercises \
                  every item, so unreachable code is still caught"
    )
)]
pub(crate) mod identity;

pub use error::{ProductDimension, ShapesProductError};

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original verification findings retained by the RDF diagnostic.

use purrdf_gts::{policy::ProfileFinding, verify::VerificationError};

#[derive(Debug)]
pub(super) struct SignatureVerificationFailure {
    pub(super) errors: Vec<VerificationError>,
    pub(super) profile_findings: Vec<ProfileFinding>,
}

impl core::fmt::Display for SignatureVerificationFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("GTS COSE signature verification failed")
    }
}

impl std::error::Error for SignatureVerificationFailure {}

impl purrdf_hash::error::StructuredError for SignatureVerificationFailure {
    purrdf_hash::error_visit! {
        @record self { errors, profile_findings } => "rdf-gts-signature-verification" {
            "errors" => List(self.errors.iter().map(|error| purrdf_hash::error::ErrorValue::Cause(error))),
            "profile_findings" => List(self.profile_findings.iter().map(|finding| purrdf_hash::error::ErrorValue::Record(finding)))
        }
    }
}

#[derive(Debug)]
pub(super) struct MissingDigest {
    pub(super) term: crate::TermId,
    pub(super) digest: purrdf_core::Blake3ContentId,
    pub(super) cause: purrdf_gts::mmr::ProofError,
}

impl core::fmt::Display for MissingDigest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "term#{} {}",
            self.term.index(),
            purrdf_gts::wire::digest_label(self.digest.as_bytes())
        )
    }
}

impl purrdf_hash::error::StructuredError for MissingDigest {
    purrdf_hash::error_visit! {
        @record self { term, digest, cause } => "rdf-gts-missing-digest" {
            "term_id" => Unsigned(self.term.index() as u128), "digest" => Bytes(self.digest.as_bytes()), "cause" => Cause(&self.cause)
        }
    }
}

#[derive(Debug)]
pub(super) struct DigestInclusionFailure {
    pub(super) missing: Vec<MissingDigest>,
}

impl core::fmt::Display for DigestInclusionFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} content-addressed term(s) are not included in the verified chain",
            self.missing.len()
        )
    }
}

impl std::error::Error for DigestInclusionFailure {}

impl purrdf_hash::error::StructuredError for DigestInclusionFailure {
    purrdf_hash::error_visit! {
        @record self { missing } => "rdf-gts-digest-inclusion" {
            "missing" => List(self.missing.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value)))
        }
    }
}

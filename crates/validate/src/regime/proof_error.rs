// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned proof-document grammar refusals, before native proof checking.

/// A proof envelope whose original text does not describe an admitted proof.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProofDocumentError {
    /// An answer line does not open with its required keyword.
    AnswerPrefix {
        /// Expected keyword.
        keyword: String,
        /// Original answer line.
        offered: String,
    },
    /// An answer line lacks its required pair of terms.
    AnswerTerms {
        /// Expected keyword.
        keyword: String,
        /// Original answer line.
        offered: String,
    },
    /// An answer's first line does not state a three-valued verdict.
    AnswerVerdict {
        /// Expected keyword.
        keyword: String,
        /// Original first line.
        offered: String,
    },
    /// An answer describes a different subject or grammar than the checked claim.
    AnswerMismatch {
        /// Original answer.
        offered: String,
        /// Expected answer for the original checked subject.
        expected: String,
    },
    /// A module extraction supplied a different kind of certificate.
    ModuleCertificate {
        /// Original supplied certificate.
        offered: String,
    },
    /// The first line is absent or names another document grammar.
    Banner {
        /// Original first line, absent for an empty document.
        offered: Option<String>,
    },
    /// The producer explicitly recorded no proof.
    NotRecorded,
    /// No body bytes were supplied.
    MissingBody,
    /// A body line is not canonical lowercase hex.
    Hex {
        /// Original one-based document line.
        line: usize,
        /// Original body spelling after the prefix.
        offered: String,
        /// Original native decoder refusal.
        cause: purrdf_hash::hex::HexError,
    },
    /// The original envelope differs from the canonical rendering of its body.
    HeaderMismatch {
        /// Original document.
        offered: String,
        /// Canonical document describing the decoded proof.
        expected: String,
    },
}

impl core::fmt::Display for ProofDocumentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AnswerPrefix { keyword, offered } => write!(f,
                "a `{keyword}` line was expected, and this is {offered:?}"),
            Self::AnswerTerms { keyword, offered } => write!(f,
                "a `{keyword}` line carries two terms, and this is {offered:?}"),
            Self::AnswerVerdict { keyword, offered } => write!(f,
                "a `{keyword}` answer opens `{keyword} true|false|unknown`, and this opens {offered:?}"),
            Self::AnswerMismatch { .. } => f.write_str(
                "the proof does not cover the answer beside it: the answer describes a different subject or grammar"),
            Self::ModuleCertificate { .. } => write!(f,
                "`extract-module` issues a {:?} block and no DL certificate; this is some other document", super::MODULE_CERTIFICATE_BANNER),
            Self::Banner { .. } => write!(f,
                "a proof document opens with {:?}, and this one does not", super::DL_PROOF_BANNER),
            Self::NotRecorded => f.write_str(
                "this answer carries no proof: nothing was recorded, because nobody asked. That \
                 is not the same as a proof with nothing in it, and it is not something a check \
                 can report as verified — ask for the proof (open the session with proofs, or \
                 call the `prove` entry point) and check that"),
            Self::MissingBody => f.write_str("a recorded proof document carries no `body` line"),
            Self::Hex { cause: purrdf_hash::hex::HexError::InvalidDigit { byte, .. }, .. } => write!(f,
                "a proof body carries {:?}, which is not a lowercase hex digit", char::from(*byte)),
            Self::Hex { .. } => f.write_str("a proof body line has an odd number of hex digits"),
            Self::HeaderMismatch { .. } => f.write_str(
                "the proof document's header does not describe the proof its own body carries, \
                 so at least one of the two was rewritten after the proof was issued"),
        }
    }
}

impl core::error::Error for ProofDocumentError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Hex { cause, .. } => Some(cause),
            Self::Banner { .. }
            | Self::NotRecorded
            | Self::MissingBody
            | Self::HeaderMismatch { .. }
            | Self::AnswerPrefix { .. }
            | Self::AnswerTerms { .. }
            | Self::AnswerVerdict { .. }
            | Self::AnswerMismatch { .. }
            | Self::ModuleCertificate { .. } => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ProofDocumentError {
    purrdf_hash::error_visit! {
        AnswerPrefix { keyword, offered } => "reasoning-answer-prefix" { "keyword" => Code(keyword), "offered" => Text(offered) },
        AnswerTerms { keyword, offered } => "reasoning-answer-terms" { "keyword" => Code(keyword), "offered" => Text(offered) },
        AnswerVerdict { keyword, offered } => "reasoning-answer-verdict" { "keyword" => Code(keyword), "offered" => Text(offered) },
        AnswerMismatch { offered, expected } => "reasoning-answer-mismatch" { "offered" => Text(offered), "expected" => Text(expected) },
        ModuleCertificate { offered } => "reasoning-module-certificate" {
            "expected" => Code(super::MODULE_CERTIFICATE_BANNER), "offered" => Text(offered),
        },
        Banner { offered } => "reasoning-proof-banner" {
            "expected" => Code(super::DL_PROOF_BANNER),
            "offered" => Value(offered.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Text)),
        },
        NotRecorded => "reasoning-proof-not-recorded" {},
        MissingBody => "reasoning-proof-missing-body" {},
        Hex { line, offered, cause } => "reasoning-proof-hex" {
            "line" => Unsigned(*line as u128), "offered" => Text(offered), "cause" => Cause(cause),
        },
        HeaderMismatch { offered, expected } => "reasoning-proof-header-mismatch" {
            "offered" => Text(offered), "expected" => Text(expected),
        },
    }
}

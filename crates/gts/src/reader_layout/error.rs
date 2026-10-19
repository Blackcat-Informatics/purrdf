// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original layout and index integrity findings.

use core::fmt;
use purrdf_hash::error::ErrorValue;

#[derive(Debug)]
pub(super) enum LayoutError {
    MissingFooter,
    Footer {
        count: u64,
        preceding: u64,
        head: Vec<u8>,
        actual: Option<Vec<u8>>,
    },
    BlobOrder {
        digest: String,
        frame: u64,
        covered: u64,
    },
    MmrWidth {
        root: Vec<u8>,
    },
    MmrPreceding {
        count: u64,
        preceding: u64,
    },
    MmrCount {
        count: u64,
        frames: usize,
    },
    MmrHead {
        head: Vec<u8>,
        actual: Option<Vec<u8>>,
    },
    MmrRoot {
        root: Vec<u8>,
        computed: Vec<u8>,
    },
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFooter => f.write_str(
                "segment claims layout 'streamable' but carries no intact index footer (§3.3)",
            ),
            Self::Footer { count, .. } => write!(
                f,
                "index footer contradicts the frames it covers: count {count} must name the frame immediately before the footer and head must be that frame's id (§3.3)"
            ),
            Self::BlobOrder { digest, .. } => write!(
                f,
                "covered blob {digest} delivered before its stream:digest description (catalog-before-payload, §3.3)"
            ),
            Self::MmrWidth { .. } => f.write_str("index mmr root is not a 32-byte digest"),
            Self::MmrPreceding { count, preceding } => write!(
                f,
                "index mmr covers {count} frame(s), but only {preceding} precede the index"
            ),
            Self::MmrCount { count, frames } => write!(
                f,
                "index mmr covers {count} frame(s), but the segment has {frames} frame id(s)"
            ),
            Self::MmrHead { .. } => {
                f.write_str("index mmr head does not match the last covered frame")
            }
            Self::MmrRoot { .. } => {
                f.write_str("index mmr root does not match the covered frame ids")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

impl purrdf_hash::error::StructuredError for LayoutError {
    purrdf_hash::error_visit! {
        MissingFooter => "gts-layout-missing-footer" {},
        Footer { count, preceding, head, actual } => "gts-layout-footer" {
            "count" => Unsigned(u128::from(*count)), "preceding" => Unsigned(u128::from(*preceding)),
            "head" => Bytes(head), "actual" => Value(actual.as_deref().map_or(ErrorValue::Absent, ErrorValue::Bytes))
        },
        BlobOrder { digest, frame, covered } => "gts-layout-blob-order" {
            "digest" => Text(digest), "frame" => Unsigned(u128::from(*frame)), "covered" => Unsigned(u128::from(*covered))
        },
        MmrWidth { root } => "gts-index-mmr-width" { "root" => Bytes(root) },
        MmrPreceding { count, preceding } => "gts-index-mmr-preceding" { "count" => Unsigned(u128::from(*count)), "preceding" => Unsigned(u128::from(*preceding)) },
        MmrCount { count, frames } => "gts-index-mmr-count" { "count" => Unsigned(u128::from(*count)), "frames" => Unsigned(*frames as u128) },
        MmrHead { head, actual } => "gts-index-mmr-head" { "head" => Bytes(head), "actual" => Value(actual.as_deref().map_or(ErrorValue::Absent, ErrorValue::Bytes)) },
        MmrRoot { root, computed } => "gts-index-mmr-root" { "root" => Bytes(root), "computed" => Bytes(computed) },
    }
}

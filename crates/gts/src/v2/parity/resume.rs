// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{Digest, Fields, GEOMETRY_ONE, Geometry, ParityClaim, ParityHistory, ParityTable};
use crate::v2::link::frame_hasher;
use crate::v2::payload::{CandidateRefusal, payload_digest};
use crate::v2::{FRAME_FIELDS, FRAME_KIND, FrameType, PayloadBudget, PayloadRefusal};

pub(in crate::v2) struct ParityResumption<'a> {
    pub(in crate::v2) length: usize,
    pub(in crate::v2) payload_end: usize,
    pub(in crate::v2) anchor: Digest,
    pub(in crate::v2) table: ParityTable<'a>,
}

struct Candidate<'a> {
    length: usize,
    payload_end: usize,
    identity: &'a [u8],
    identifier: &'a Digest,
    payload: &'a [u8],
    digest: &'a Digest,
    claim: ParityClaim<'a>,
    geometry: Geometry,
}

impl<'a> Candidate<'a> {
    fn read(bytes: &'a [u8], history: &ParityHistory) -> Option<Self> {
        let mut fields = Fields::new(bytes);
        fields.record(FRAME_FIELDS as u64).ok()?;
        if fields.uint().ok()? != FRAME_KIND {
            return None;
        }
        let identity_start = fields.offset();
        if fields.uint().ok()? != FrameType::Parity as u64
            || fields.boolean().ok()?
            || !fields.boolean().ok()?
        {
            return None;
        }
        let claim = ParityClaim::read(&mut fields).ok()?;
        if claim.geometry != GEOMETRY_ONE || history.blocks(claim.first) {
            return None;
        }
        let geometry = Geometry::for_span(claim.first, claim.end)?;
        if fields.uint().ok()? != geometry.payload_length() as u64 {
            return None;
        }
        let digest = fields.fixed_bytes().ok()?;
        let identity = &bytes[identity_start..fields.offset()];
        fields.record(1).ok()?;
        if fields.uint().ok()? != 1 {
            return None;
        }
        let payload = fields.bytes().ok()?;
        if payload.len() != geometry.payload_length() {
            return None;
        }
        let payload_end = fields.offset();
        let identifier = fields.fixed_bytes().ok()?;
        fields.record(0).ok()?;
        Some(Self {
            length: fields.offset(),
            payload_end,
            identity,
            identifier,
            payload,
            digest,
            claim,
            geometry,
        })
    }
}

impl ParityHistory {
    pub(in crate::v2) fn scan_at<'a>(
        &self,
        bytes: &'a [u8],
        offset: usize,
        budget: &mut PayloadBudget,
    ) -> Result<Option<ParityResumption<'a>>, CandidateRefusal> {
        let Some(candidate) = Candidate::read(bytes, self) else {
            return Ok(None);
        };
        let mut hasher = frame_hasher(candidate.claim.anchor);
        hasher.update(candidate.identity);
        if hasher.finalize().as_bytes() != candidate.identifier {
            return Ok(None);
        }
        match budget.charge_parity_candidate(candidate.payload.len() as u64) {
            Ok(()) => {}
            Err(PayloadRefusal::TotalDecoded(requested_total)) => {
                return Err(CandidateRefusal {
                    offset,
                    length: candidate.length,
                    requested_total,
                });
            }
            Err(_) => return Ok(None),
        }
        if &payload_digest(candidate.payload) != candidate.digest {
            return Ok(None);
        }
        Ok(candidate
            .claim
            .read_table(candidate.payload, candidate.geometry, &self.opener)
            .map(|table| ParityResumption {
                length: candidate.length,
                payload_end: candidate.payload_end,
                anchor: *candidate.claim.anchor,
                table,
            }))
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic positional query planning, admission budgets and exact checking.

use super::{
    MatchProjection, SubstringMatch, SurfaceIndex,
    postings::{self, GramPosting},
};
use crate::TextError;
use std::fmt;

/// Immutable logical work limits, included in the analyzer profile identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubstringLimits {
    /// Anchor posting visits plus relative-position membership tests.
    pub posting_operations: u64,
    /// Distinct candidate spans admitted before text verification.
    pub candidate_spans: u64,
    /// Full UTF-8 length of each candidate span, charged once per span.
    pub verification_bytes: u64,
}
impl SubstringLimits {
    /// Standard limits: 65,536 operations, 4,096 spans and 2 MiB of text.
    pub const STANDARD: Self = Self {
        posting_operations: 65_536,
        candidate_spans: 4_096,
        verification_bytes: 2 * 1_024 * 1_024,
    };
}
impl Default for SubstringLimits {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Charged logical work; independent of CPU, search strategy and architecture.
/// Charges precede work, so a refusal includes the first charge exceeding its
/// budget. Exact byte verification never starts before selective admission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SubstringWork {
    /// Anchor visits and required-position membership lookups.
    pub posting_operations: u64,
    /// Distinct admitted candidates, after positional and emoji filtering.
    pub candidate_spans: u64,
    /// Candidate text bytes charged for exact verification.
    pub verification_bytes: u64,
}

/// Why an indexed or explicitly exhaustive query did not produce an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SubstringRefusalReason {
    /// The final indexed candidate set equals all stored distinct spans.
    NonSelective,
    /// The next posting operation exceeded the immutable logical budget.
    PostingOperations,
    /// The next distinct candidate exceeded the immutable candidate budget.
    CandidateSpans,
    /// The next candidate's text exceeded the immutable verification budget.
    VerificationBytes,
}

/// A complete typed refusal; it never carries partial matches.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubstringRefusal {
    /// Admission or resource condition that refused the query.
    pub reason: SubstringRefusalReason,
    /// Deterministic counters at refusal, including the refused charge.
    pub counters: SubstringWork,
    /// The immutable limits under which this request was evaluated.
    pub limits: SubstringLimits,
    /// Total distinct stored spans, the denominator of selectivity.
    pub distinct_spans: u64,
    /// Exact immutable index generation.
    pub generation: [u8; 32],
}
impl fmt::Display for SubstringRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "substring refused ({:?}): {} operations, {} of {} candidate spans, {} verification bytes; limits {}/{}/{}; generation {}",
            self.reason,
            self.counters.posting_operations,
            self.counters.candidate_spans,
            self.distinct_spans,
            self.counters.verification_bytes,
            self.limits.posting_operations,
            self.limits.candidate_spans,
            self.limits.verification_bytes,
            purrdf_hash::hex::Lower(&self.generation)
        )
    }
}
impl std::error::Error for SubstringRefusal {}

/// Complete matches and the logical work used to establish them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubstringReport<'a> {
    /// Exact matches, in lexical span order; never a partial prefix.
    pub matches: Vec<SubstringMatch<'a>>,
    /// Deterministic work charged by this operation.
    pub counters: SubstringWork,
    /// Exact immutable index generation.
    pub generation: [u8; 32],
}

struct Budget {
    limits: SubstringLimits,
    work: SubstringWork,
    generation: [u8; 32],
    total: u64,
}
impl Budget {
    fn refusal(&self, reason: SubstringRefusalReason) -> SubstringRefusal {
        SubstringRefusal {
            reason,
            counters: self.work,
            limits: self.limits,
            distinct_spans: self.total,
            generation: self.generation,
        }
    }
    fn charge(
        &mut self,
        reason: SubstringRefusalReason,
        amount: u64,
    ) -> Result<(), SubstringRefusal> {
        let (counter, limit) = match reason {
            SubstringRefusalReason::PostingOperations => (
                &mut self.work.posting_operations,
                self.limits.posting_operations,
            ),
            SubstringRefusalReason::CandidateSpans => {
                (&mut self.work.candidate_spans, self.limits.candidate_spans)
            }
            SubstringRefusalReason::VerificationBytes => (
                &mut self.work.verification_bytes,
                self.limits.verification_bytes,
            ),
            SubstringRefusalReason::NonSelective => return Err(self.refusal(reason)),
        };
        let next = counter.checked_add(amount);
        *counter = next.unwrap_or(u64::MAX);
        if next.is_none() || *counter > limit {
            return Err(self.refusal(reason));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct QueryGram<'a> {
    key: u64,
    offset: u64,
    postings: &'a [GramPosting],
}
struct Candidate {
    span: usize,
    starts: Vec<usize>,
}

impl SurfaceIndex {
    /// Find exact substrings only after the positional index proves selectivity.
    /// Even a singleton matching index is nonselective and requires explicit
    /// exhaustive opt-in. Missing grams and empty indexes return empty results.
    ///
    /// # Errors
    /// Returns a typed refusal on universal candidates or a logical work limit;
    /// malformed or erroneous input is rejected through the analyzer.
    pub fn substring(&self, fragment: &str) -> Result<Vec<SubstringMatch<'_>>, TextError> {
        Ok(self.substring_report(fragment)?.matches)
    }

    /// Indexed substring lookup with inspectable deterministic work counters.
    ///
    /// # Errors
    /// Same admission and analysis refusals as [`Self::substring`].
    pub fn substring_report(&self, fragment: &str) -> Result<SubstringReport<'_>, TextError> {
        self.run_substring(fragment, false)
    }

    /// Explicitly inspect every span, without requiring indexed selectivity.
    /// Candidate and verification-byte limits still apply. There is no automatic
    /// fallback to this operation after an indexed refusal.
    ///
    /// # Errors
    /// Returns the analyzer's errors or typed candidate/verification refusals.
    pub fn substring_exhaustive(
        &self,
        fragment: &str,
    ) -> Result<Vec<SubstringMatch<'_>>, TextError> {
        Ok(self.substring_exhaustive_report(fragment)?.matches)
    }

    /// Explicit exhaustive lookup with inspectable resource counters.
    ///
    /// # Errors
    /// Same refusals as [`Self::substring_exhaustive`].
    pub fn substring_exhaustive_report(
        &self,
        fragment: &str,
    ) -> Result<SubstringReport<'_>, TextError> {
        self.run_substring(fragment, true)
    }

    fn run_substring(
        &self,
        fragment: &str,
        exhaustive: bool,
    ) -> Result<SubstringReport<'_>, TextError> {
        let terms = self.analyzer.substring_terms(fragment)?;
        let [needle] = terms.as_slice() else {
            return Err(TextError::data(
                "substring query must normalize to one nonempty span",
            ));
        };
        let mut budget = Budget {
            limits: self.analyzer.profile().substring_limits(),
            work: SubstringWork::default(),
            generation: self.generation,
            total: self.spans.len() as u64,
        };
        let scalar_count = needle.chars().count();
        let candidates = if exhaustive {
            let capacity = usize::try_from(budget.limits.candidate_spans).unwrap_or(usize::MAX);
            let mut candidates = Vec::with_capacity(self.spans.len().min(capacity));
            for (span, shape) in self.shapes.iter().enumerate() {
                budget.charge(SubstringRefusalReason::CandidateSpans, 1)?;
                let starts = (0..shape.offsets.len())
                    .filter(|&start| eligible(shape, start, scalar_count))
                    .collect();
                candidates.push(Candidate { span, starts });
            }
            candidates
        } else {
            let candidates = self.indexed_candidates(needle, &mut budget)?;
            if !self.spans.is_empty() && candidates.len() == self.spans.len() {
                return Err(budget.refusal(SubstringRefusalReason::NonSelective).into());
            }
            candidates
        };
        // Charge every candidate before inspecting any text. A byte-budget
        // refusal cannot depend on a previously discovered matching prefix.
        for candidate in &candidates {
            budget.charge(
                SubstringRefusalReason::VerificationBytes,
                self.spans[candidate.span].text.len() as u64,
            )?;
        }
        let mut matches = Vec::new();
        for candidate in candidates {
            let term = &self.spans[candidate.span];
            let shape = &self.shapes[candidate.span];
            let ranges: Vec<_> = candidate
                .starts
                .into_iter()
                .filter_map(|start| {
                    let range = shape.offsets[start]..shape.offsets[start + scalar_count];
                    (term.text.get(range.clone()) == Some(needle.as_str())).then_some(range)
                })
                .collect();
            if !ranges.is_empty() {
                let evidence = self.evidence(term, &ranges, MatchProjection::SubstringSpan);
                matches.push(SubstringMatch {
                    term,
                    ranges,
                    evidence,
                });
            }
        }
        Ok(SubstringReport {
            matches,
            counters: budget.work,
            generation: self.generation,
        })
    }

    fn indexed_candidates(
        &self,
        needle: &str,
        budget: &mut Budget,
    ) -> Result<Vec<Candidate>, SubstringRefusal> {
        let scalars: Vec<_> = needle.chars().collect();
        let width = scalars.len().min(3);
        let index = &self.grams[width - 1];
        let mut grams = Vec::with_capacity(scalars.len() - width + 1);
        for (offset, scalars) in scalars.windows(width).enumerate() {
            let key = postings::key(scalars);
            let Some(postings) = index.get(&key) else {
                return Ok(Vec::new());
            };
            grams.push(QueryGram {
                key,
                offset: offset as u64,
                postings,
            });
        }
        // Canonical law: rarest posting list, then gram key, then query offset.
        // Remaining membership tests are ordered by (gram key, query offset).
        grams.sort_by_key(|gram| (gram.key, gram.offset));
        let anchor = *grams
            .iter()
            .min_by_key(|gram| (gram.postings.len(), gram.key, gram.offset))
            .expect("nonempty needle");
        let mut candidates: Vec<Candidate> = Vec::new();
        'anchor: for posting in anchor.postings {
            budget.charge(SubstringRefusalReason::PostingOperations, 1)?;
            let Some(start) = posting.position.checked_sub(anchor.offset) else {
                continue;
            };
            for gram in &grams {
                if gram.key == anchor.key && gram.offset == anchor.offset {
                    continue;
                }
                budget.charge(SubstringRefusalReason::PostingOperations, 1)?;
                let Some(position) = start.checked_add(gram.offset) else {
                    continue 'anchor;
                };
                if gram
                    .postings
                    .binary_search(&GramPosting {
                        span: posting.span,
                        position,
                    })
                    .is_err()
                {
                    continue 'anchor;
                }
            }
            let span = posting.span as usize;
            let Ok(start) = usize::try_from(start) else {
                continue;
            };
            if !eligible(&self.shapes[span], start, scalars.len()) {
                continue;
            }
            if candidates
                .last()
                .is_none_or(|candidate| candidate.span != span)
            {
                budget.charge(SubstringRefusalReason::CandidateSpans, 1)?;
                candidates.push(Candidate {
                    span,
                    starts: Vec::new(),
                });
            }
            candidates
                .last_mut()
                .expect("candidate inserted")
                .starts
                .push(start);
        }
        Ok(candidates)
    }
}

fn eligible(shape: &postings::SpanShape, start: usize, length: usize) -> bool {
    start.checked_add(length).is_some_and(|end| {
        shape.eligible.get(start) == Some(&true) && shape.eligible.get(end) == Some(&true)
    })
}

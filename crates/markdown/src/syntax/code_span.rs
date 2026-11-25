// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admitted backtick matching, independent of document or scalar policy.
use super::{Policy, SyntaxError, SyntaxLimits, budget::Budget};
use std::borrow::Cow;

/// Inclusive admission ceilings for an already-decoded UTF-8 scalar.
/// All three controls are explicit; this type supplies no unbounded preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeSpanLimits {
    /// Original decoded scalar bytes.
    pub source_bytes: usize,
    /// Live native scratch and output capacity, excluding the caller's borrowed input.
    pub auxiliary_bytes: usize,
    /// Deterministic source, probe, comparison and copy work units.
    pub work: u64,
}

fn run_end(
    text: &str,
    mut at: usize,
    position: &impl Fn(usize) -> usize,
    budget: &mut Budget<'_>,
) -> Result<usize, SyntaxError> {
    while at < text.len() {
        budget.tick(position(at), 1)?;
        if text.as_bytes()[at] != b'`' {
            break;
        }
        at += 1;
    }
    Ok(at)
}

fn next_run(
    text: &str,
    at: &mut usize,
    position: &impl Fn(usize) -> usize,
    budget: &mut Budget<'_>,
) -> Result<Option<(usize, usize)>, SyntaxError> {
    while *at < text.len() {
        budget.tick(position(*at), 1)?;
        if text.as_bytes()[*at] != b'`' {
            *at += 1;
            continue;
        }
        let start = *at;
        *at = run_end(text, start, position, budget)?;
        return Ok(Some((start, *at)));
    }
    Ok(None)
}

#[derive(Default)]
struct Suffix {
    runs: usize,
    widest: usize,
}

impl Suffix {
    fn fits(&self, budget: &Budget<'_>) -> bool {
        self.runs != 0
            && self
                .runs
                .checked_mul(size_of::<Run>())
                .zip(
                    self.widest
                        .checked_add(1)
                        .and_then(|n| n.checked_mul(size_of::<usize>())),
                )
                .and_then(|(links, heads)| links.checked_add(heads))
                .is_some_and(|bytes| bytes <= budget.limits.auxiliary_bytes - budget.live)
    }
}

const NO_RUN: usize = usize::MAX;

struct Run {
    start: usize,
    end: usize,
    next: usize,
}

pub(super) struct RunIndex {
    runs: Vec<Run>,
    heads: Vec<usize>,
}

impl RunIndex {
    fn build(
        text: &str,
        mut at: usize,
        bound: &Suffix,
        position: &impl Fn(usize) -> usize,
        budget: &mut Budget<'_>,
    ) -> Result<Self, SyntaxError> {
        let mut runs = Vec::new();
        budget.memory(|memory| memory.reserve(&mut runs, bound.runs))?;
        let mut heads = budget
            .memory(|memory| memory.collect(std::iter::repeat_n(NO_RUN, bound.widest + 1)))?;
        while let Some((start, end)) = next_run(text, &mut at, position, budget)? {
            budget.tick(position(start), 0)?;
            budget.push(
                &mut runs,
                Run {
                    start,
                    end,
                    next: NO_RUN,
                },
            )?;
        }
        // A later opening end leaves only a subset of the last failed
        // search's full runs. Its width directory and link bound still hold.
        for (index, run) in runs.iter_mut().enumerate().rev() {
            budget.tick(position(run.start), 1)?;
            let head = &mut heads[run.end - run.start];
            run.next = *head;
            *head = index;
        }
        Ok(Self { runs, heads })
    }

    pub(super) fn close(
        &mut self,
        at: usize,
        width: usize,
        position: &impl Fn(usize) -> usize,
        budget: &mut Budget<'_>,
    ) -> Result<Option<usize>, SyntaxError> {
        budget.tick(position(at), 1)?;
        let Some(head) = self.heads.get_mut(width) else {
            return Ok(None);
        };
        while *head != NO_RUN {
            budget.tick(position(at), 1)?;
            let run = &self.runs[*head];
            if run.start >= at {
                return Ok(Some(run.end));
            }
            *head = run.next;
        }
        Ok(None)
    }
}

/// One matcher's openers are visited in source order in one logical input.
pub(super) struct CodeSpans<'a> {
    text: &'a str,
    failed_work: u64,
    suffix: Suffix,
    index_started: bool,
}

impl<'a> CodeSpans<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        Self {
            text,
            failed_work: 0,
            suffix: Suffix::default(),
            index_started: false,
        }
    }

    fn close(
        &mut self,
        mut at: usize,
        width: usize,
        position: &impl Fn(usize) -> usize,
        budget: &mut Budget<'_>,
    ) -> Result<Option<usize>, SyntaxError> {
        // Successful spans never justify an extra suffix pass. Only repeated
        // misses can spend this much closing work before requesting an index.
        if !self.index_started
            && at < self.text.len()
            && self.failed_work >= (self.text.len() as u64).saturating_mul(2)
            && self.suffix.fits(budget)
        {
            let before = budget.live;
            let index = RunIndex::build(self.text, at, &self.suffix, position, budget)?;
            budget.retain_code_span_index(index, before);
            self.index_started = true;
        }
        if self.index_started
            && let Some(result) = budget.indexed_code_span_close(at, width, position)
        {
            return result;
        }
        // After eviction, direct searches resume without rebuilding the index.
        let before = budget.work;
        let mut suffix = Suffix::default();
        while let Some((start, end)) = next_run(self.text, &mut at, position, budget)? {
            budget.tick(position(start), 1)?;
            let candidate = end - start;
            if candidate == width {
                return Ok(Some(end));
            }
            suffix.runs += 1;
            suffix.widest = suffix.widest.max(candidate);
        }
        self.failed_work = self.failed_work.saturating_add(budget.work - before);
        self.suffix = suffix;
        Ok(None)
    }

    // Callers select a backtick byte at a UTF-8 boundary. Returning the maximal
    // opening end lets unmatched and escaped runs advance without a second scan.
    pub(super) fn read(
        &mut self,
        at: usize,
        position: impl Fn(usize) -> usize,
        budget: &mut Budget<'_>,
    ) -> Result<(usize, Option<usize>), SyntaxError> {
        let opening_end = run_end(self.text, at, &position, budget)?;
        let mut before = at;
        let mut escaped = false;
        while before > 0 {
            budget.tick(position(before - 1), 1)?;
            if self.text.as_bytes()[before - 1] != b'\\' {
                break;
            }
            escaped = !escaped;
            before -= 1;
        }
        if escaped {
            return Ok((opening_end, None));
        }
        let closing_end = self.close(opening_end, opening_end - at, &position, budget)?;
        Ok((opening_end, closing_end))
    }

    pub(super) fn release(self, budget: &mut Budget<'_>) {
        if self.index_started {
            budget.release_code_span_index();
        }
    }
}

/// Replace every complete backtick span in a decoded scalar with one ASCII space.
/// Matching crosses LF and CR and uses maximal equal-length runs. Only an odd
/// number of backslashes immediately before an opening run escapes it; closing
/// runs are never escaped. All other original UTF-8 bytes remain unchanged,
/// including comments, HTML, entities, fence-looking text and encoding marks.
///
/// Unchanged input is borrowed without output allocation. Changed output is
/// admitted before growth and copying. A refusal returns the original typed
/// cause with positions in this decoded scalar and publishes no partial text.
/// Repeated failed searches can admit a width index when scratch storage fits.
pub fn replace_code_spans(
    source: &str,
    limits: CodeSpanLimits,
) -> Result<Cow<'_, str>, SyntaxError> {
    let mut budget = Budget::new(
        source,
        SyntaxLimits {
            source_bytes: limits.source_bytes,
            auxiliary_bytes: limits.auxiliary_bytes,
            work: limits.work,
            containers: 0,
            nesting: 0,
            link_part_bytes: 0,
        },
        Policy::Standard,
    );
    budget.limit(0, "source bytes", source.len(), limits.source_bytes)?;
    budget.tick(0, source.len())?;
    let mut output = None;
    let mut spans = CodeSpans::new(source);
    let mut at = 0;
    let mut from = 0;
    while at < source.len() {
        budget.tick(at, 1)?;
        if source.as_bytes()[at] != b'`' {
            at += 1;
            continue;
        }
        let (opening_end, closing_end) = spans.read(at, |at| at, &mut budget)?;
        if let Some(end) = closing_end {
            let text = output.get_or_insert_with(String::new);
            budget.tick(at, 0)?;
            budget.memory(|m| {
                m.push_str(text, &source[from..at])?;
                m.push_char(text, ' ')
            })?;
            at = end;
            from = end;
        } else {
            at = opening_end;
        }
    }
    spans.release(&mut budget);
    if let Some(mut output) = output {
        budget.tick(from, 0)?;
        budget.memory(|m| m.push_str(&mut output, &source[from..]))?;
        Ok(Cow::Owned(output))
    } else {
        Ok(Cow::Borrowed(source))
    }
}

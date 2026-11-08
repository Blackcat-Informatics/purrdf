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
    /// Live native output capacity, excluding the caller's borrowed input.
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

fn close(
    text: &str,
    mut at: usize,
    count: usize,
    position: &impl Fn(usize) -> usize,
    budget: &mut Budget<'_>,
) -> Result<Option<usize>, SyntaxError> {
    while at < text.len() {
        budget.tick(position(at), 1)?;
        if text.as_bytes()[at] != b'`' {
            at += 1;
            continue;
        }
        let end = run_end(text, at, position, budget)?;
        budget.tick(position(at), 1)?;
        if end - at == count {
            return Ok(Some(end));
        }
        at = end;
    }
    Ok(None)
}

// Callers select a backtick byte at a UTF-8 boundary. Returning the maximal
// opening end lets unmatched and escaped runs advance without a second scan.
pub(super) fn read(
    text: &str,
    at: usize,
    position: impl Fn(usize) -> usize,
    budget: &mut Budget<'_>,
) -> Result<(usize, Option<usize>), SyntaxError> {
    let opening_end = run_end(text, at, &position, budget)?;
    let mut before = at;
    let mut escaped = false;
    while before > 0 {
        budget.tick(position(before - 1), 1)?;
        if text.as_bytes()[before - 1] != b'\\' {
            break;
        }
        escaped = !escaped;
        before -= 1;
    }
    if escaped {
        return Ok((opening_end, None));
    }
    let closing_end = close(text, opening_end, opening_end - at, &position, budget)?;
    Ok((opening_end, closing_end))
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
/// Repeated closing searches consume work; linear execution is not promised.
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
    let mut at = 0;
    let mut from = 0;
    while at < source.len() {
        budget.tick(at, 1)?;
        if source.as_bytes()[at] != b'`' {
            at += 1;
            continue;
        }
        let (opening_end, closing_end) = read(source, at, |at| at, &mut budget)?;
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
    if let Some(mut output) = output {
        budget.tick(from, 0)?;
        budget.memory(|m| m.push_str(&mut output, &source[from..]))?;
        Ok(Cow::Owned(output))
    } else {
        Ok(Cow::Borrowed(source))
    }
}

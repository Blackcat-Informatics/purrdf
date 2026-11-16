// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one search for the end of a run in sorted data.
//!
//! A sorted table or column groups equal keys into runs, and a probe wants the
//! end of the run that starts at a known position. The run is one row in the
//! common case and the table can be large, so the search gallops: it doubles a
//! step from the start until the predicate fails, then bisects the last step.
//! A run of `n` elements costs `O(log n)` comparisons however large the table is.

/// The first index at or after `start` whose element fails `holds`, given that
/// `holds` is true for a prefix of `items[start..]` and false after it — the same
/// answer as `items.partition_point(holds)` when everything before `start` holds.
///
/// A lower bound is the case `holds = |item| *item < key`.
///
/// # Panics
/// When `start` is past the end of `items`.
pub fn run_end<T>(items: &[T], start: usize, holds: impl Fn(&T) -> bool) -> usize {
    let rest = &items[start..];
    // `rest[..known]` holds; `rest[known..bound]` is where the first failure lies.
    let mut known = 0;
    let mut step = 1;
    while step <= rest.len() && holds(&rest[step - 1]) {
        known = step;
        step = step.saturating_mul(2);
    }
    let bound = step.min(rest.len());
    start + known + rest[known..bound].partition_point(holds)
}

#[cfg(test)]
mod tests {
    use super::run_end;

    /// A run's end found by galloping is the end the binary search finds, from
    /// every start at or before the run and for every run length, including a run
    /// that ends the table, one that is empty and a start at the end of the table.
    #[test]
    fn a_run_end_found_by_galloping_is_the_one_binary_search_finds() {
        for len in 0..40usize {
            for run in 0..=len {
                // `len` rows; the first `run` rows hold, the rest do not.
                let items: Vec<usize> = (0..len).collect();
                for start in 0..=run {
                    assert_eq!(
                        run_end(&items, start, |&item| item < run),
                        items.partition_point(|&item| item < run),
                        "table of {len}, run ending at {run}, starting at {start}"
                    );
                }
            }
        }
    }
}

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
/// `holds` is true for a prefix of `items[start..]` and false after it: the same
/// answer as `items.partition_point(holds)` when everything before `start` holds.
///
/// A lower bound of `key` in an ascending slice is the case `holds = |item| *item < key`.
///
/// The probes sit at offsets 0, 1, 3, 7, ... from `start`. When one fails, the
/// bisection covers the offsets after the last success and before the failed probe;
/// when the step runs past the end it covers the rest of the slice. An element the
/// search has already seen fail is never compared again.
///
/// # Panics
/// When `start` is past the end of `items`.
pub fn run_end<T>(items: &[T], start: usize, holds: impl Fn(&T) -> bool) -> usize {
    let rest = &items[start..];
    let len = rest.len();
    // `rest[..known]` holds; the first failure lies in `rest[known..bound]`.
    let mut known = 0;
    let mut step = 1;
    let bound = loop {
        if step > len {
            break len;
        }
        if !holds(&rest[step - 1]) {
            break step - 1;
        }
        known = step;
        // The next doubled step would pass the end; stop before it can overflow.
        if step > len / 2 {
            break len;
        }
        step *= 2;
    };
    let (mut left, mut right) = (known, bound);
    while left < right {
        let middle = left + (right - left) / 2;
        if holds(&rest[middle]) {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    start + left
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::run_end;

    /// The comparisons the Datalog subject-run lookup made before it shared this
    /// search, for a table of `len` rows whose first `run` rows hold, probed from the
    /// start: the first row, then offsets 1, 3, 7, ..., then a bisection of the
    /// bracket between the last success and the failed probe.
    fn comparisons_before_sharing(len: usize, run: usize) -> usize {
        let count = Cell::new(0usize);
        let holds = |index: usize| {
            count.set(count.get() + 1);
            index < run
        };
        if len == 0 || !holds(0) {
            return count.get();
        }
        let mut low = 0usize;
        let mut step = 1usize;
        let high = loop {
            let probe = low + step;
            if probe >= len {
                break len;
            }
            if !holds(probe) {
                break probe;
            }
            low = probe;
            step *= 2;
        };
        let (mut left, mut right) = (low + 1, high);
        while left < right {
            let middle = left + (right - left) / 2;
            if holds(middle) {
                left = middle + 1;
            } else {
                right = middle;
            }
        }
        count.get()
    }

    /// A run's end found by galloping is the end the binary search finds, from
    /// every start at or before the run and for every run length, including a run
    /// that ends the table, one that is empty and a start at the end of the table;
    /// and a lookup from the start never compares more than the Datalog lookup did
    /// before it shared this search.
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
                let count = Cell::new(0usize);
                let end = run_end(&items, 0, |&item| {
                    count.set(count.get() + 1);
                    item < run
                });
                assert_eq!(end, run);
                assert!(
                    count.get() <= comparisons_before_sharing(len, run),
                    "table of {len}, run of {run}: {} comparisons",
                    count.get()
                );
            }
        }
    }

    /// A slice of zero-sized elements as long as an index can be, all holding, ends
    /// the search at its length instead of doubling the step forever.
    #[test]
    fn a_zero_sized_slice_of_maximal_length_terminates() {
        let items = [(); usize::MAX];
        assert_eq!(run_end(&items, 0, |()| true), usize::MAX);
        assert_eq!(run_end(&items, 0, |()| false), 0);
    }
}

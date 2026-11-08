// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation-free fallible ordering for consumers that meter every comparison.

use core::cmp::Ordering;

/// Sort a slice in place, calling the fallible comparator for every comparison.
///
/// Three-way partitions remove complete equal-key runs; small tails use
/// insertion and a bounded-depth binary-heap reduction protects the worst case.
/// The explicit smaller-first frontier uses no allocation or recursion. Equal
/// elements may change relative order. A consumer can charge work and poll
/// cancellation in the comparator, so a refusal stops before another comparison.
///
/// # Errors
/// Returns the comparator's error immediately. On refusal the slice retains all
/// its elements in an unspecified order; only success promises sorted output.
pub fn try_sort_unstable_by<T, E>(
    values: &mut [T],
    mut compare: impl FnMut(&T, &T) -> Result<Ordering, E>,
) -> Result<(), E> {
    if values.len() < 2 {
        return Ok(());
    }
    let mut pending = [SortSpan::default(); usize::BITS as usize];
    let mut count = 0;
    let mut span = SortSpan {
        start: 0,
        end: values.len(),
        depth: 2 * values.len().bit_width(),
    };
    loop {
        while span.end - span.start > 16 {
            let current = &mut values[span.start..span.end];
            if span.depth == 0 {
                heap_sort(current, &mut compare)?;
                span.end = span.start;
                break;
            }
            let (lower, upper) = partition(current, &mut compare)?;
            let left = SortSpan {
                start: span.start,
                end: span.start + lower,
                depth: span.depth - 1,
            };
            let right = SortSpan {
                start: span.start + upper,
                end: span.end,
                depth: span.depth - 1,
            };
            let (small, large) = if lower <= current.len() - upper {
                (left, right)
            } else {
                (right, left)
            };
            if large.end - large.start > 1 {
                // Each additional pending sibling follows a child no larger
                // than half its parent, so positive usize lengths need at most
                // usize::BITS-1 simultaneous pending spans.
                pending[count] = large;
                count += 1;
            }
            span = small;
        }
        if span.end - span.start > 1 {
            for next in span.start + 1..span.end {
                let mut index = next;
                while index > span.start && compare(&values[index], &values[index - 1])?.is_lt() {
                    values.swap(index, index - 1);
                    index -= 1;
                }
            }
        }
        if count == 0 {
            return Ok(());
        }
        count -= 1;
        span = pending[count];
    }
}

#[derive(Clone, Copy, Default)]
struct SortSpan {
    start: usize,
    end: usize,
    depth: u32,
}

/// Keep the pivot in the last slot while growing disjoint less/equal/greater
/// runs. Every iteration consumes one unclassified element, including ties.
fn partition<T, E>(
    values: &mut [T],
    compare: &mut impl FnMut(&T, &T) -> Result<Ordering, E>,
) -> Result<(usize, usize), E> {
    let (middle, last) = (values.len() / 2, values.len() - 1);
    if compare(&values[middle], &values[0])?.is_lt() {
        values.swap(0, middle);
    }
    if compare(&values[last], &values[middle])?.is_lt() {
        values.swap(middle, last);
    }
    if compare(&values[middle], &values[0])?.is_lt() {
        values.swap(0, middle);
    }
    values.swap(middle, last);
    let (mut lower, mut scan, mut upper) = (0, 0, last);
    while scan < upper {
        match compare(&values[scan], &values[last])? {
            Ordering::Less => {
                values.swap(lower, scan);
                lower += 1;
                scan += 1;
            }
            Ordering::Equal => scan += 1,
            Ordering::Greater => {
                upper -= 1;
                values.swap(scan, upper);
            }
        }
    }
    values.swap(upper, last);
    Ok((lower, upper + 1))
}

/// The original allocation-free worst-case reduction, shared by every fallback.
fn heap_sort<T, E>(
    values: &mut [T],
    compare: &mut impl FnMut(&T, &T) -> Result<Ordering, E>,
) -> Result<(), E> {
    HeapSortFrame::new(values.len())
        .resume(values, compare, || true)
        .map(|complete| {
            debug_assert!(
                complete,
                "an unlimited comparison grant completes the original sort"
            );
        })
}

/// Retained progress of the native in-place heap reduction.
///
/// It owns no elements or allocation. Its caller keeps the original slice
/// unchanged between resumes. Every comparison first asks the caller for work;
/// refusal retains the exact sift position without repeating a comparison.
#[derive(Debug)]
pub struct HeapSortFrame {
    len: usize,
    build: usize,
    end: usize,
    active: Option<SiftFrame>,
}

impl HeapSortFrame {
    /// Start sorting an original slice of the stated length.
    pub const fn new(len: usize) -> Self {
        Self {
            len,
            build: len / 2,
            end: len,
            active: None,
        }
    }

    /// Resume through the shared native sift, charging before each comparison.
    /// Returns false when work is refused and true only after the whole sort.
    ///
    /// # Errors
    /// Returns the original comparator failure with every element still owned.
    ///
    /// # Panics
    /// Panics if a caller replaces the retained slice with a different length.
    pub fn resume<T, E>(
        &mut self,
        values: &mut [T],
        mut compare: impl FnMut(&T, &T) -> Result<Ordering, E>,
        mut allow: impl FnMut() -> bool,
    ) -> Result<bool, E> {
        assert_eq!(
            values.len(),
            self.len,
            "sort progress retains its original slice"
        );
        loop {
            if let Some(active) = &mut self.active {
                if !active.resume(&mut values[..self.end], &mut compare, &mut allow)? {
                    return Ok(false);
                }
                self.active = None;
            }
            if self.build != 0 {
                self.build -= 1;
                self.active = Some(SiftFrame::new(self.build));
            } else if self.end > 1 {
                self.end -= 1;
                values.swap(0, self.end);
                self.active = Some(SiftFrame::new(0));
            } else {
                return Ok(true);
            }
        }
    }
}

#[derive(Debug)]
struct SiftFrame {
    root: usize,
    child: Option<usize>,
}

impl SiftFrame {
    const fn new(root: usize) -> Self {
        Self { root, child: None }
    }

    fn resume<T, E>(
        &mut self,
        values: &mut [T],
        compare: &mut impl FnMut(&T, &T) -> Result<Ordering, E>,
        allow: &mut impl FnMut() -> bool,
    ) -> Result<bool, E> {
        // Every internal node is below len/2, bounding 2*root+1 before arithmetic.
        while self.root < values.len() / 2 {
            if self.child.is_none() {
                let mut child = 2 * self.root + 1;
                if child + 1 < values.len() {
                    if !allow() {
                        return Ok(false);
                    }
                    if compare(&values[child], &values[child + 1])? == Ordering::Less {
                        child += 1;
                    }
                }
                self.child = Some(child);
            }
            let child = self
                .child
                .expect("the original child selection remains owned");
            if !allow() {
                return Ok(false);
            }
            if compare(&values[self.root], &values[child])? != Ordering::Less {
                return Ok(true);
            }
            values.swap(self.root, child);
            self.root = child;
            self.child = None;
        }
        Ok(true)
    }
}

/// Restore a maximum heap after its caller appends one admitted element.
///
/// The preceding prefix must already be a heap under the same comparator. No
/// allocation or element copy occurs; the caller retains its original buffer.
///
/// # Errors
/// Returns the first comparator refusal with every element still present. On
/// refusal only the element inventory, rather than heap order, is promised.
pub fn try_heap_push_by<T, E>(
    values: &mut [T],
    mut compare: impl FnMut(&T, &T) -> Result<Ordering, E>,
) -> Result<(), E> {
    let Some(mut child) = values.len().checked_sub(1) else {
        return Ok(());
    };
    while child > 0 {
        let parent = (child - 1) / 2;
        if !compare(&values[parent], &values[child])?.is_lt() {
            break;
        }
        values.swap(parent, child);
        child = parent;
    }
    Ok(())
}

/// Move a maximum heap's root into its last slot and restore the remaining
/// prefix through the same sift used by the in-place sorting reduction.
///
/// The caller removes the last slot only after success. Empty heaps are valid.
/// This function neither allocates nor destroys an element.
///
/// # Errors
/// Returns the first comparator refusal with every original element present,
/// in an unspecified order. Only success promises a heap in the shorter prefix.
pub fn try_heap_pop_by<T, E>(
    values: &mut [T],
    mut compare: impl FnMut(&T, &T) -> Result<Ordering, E>,
) -> Result<(), E> {
    let Some(last) = values.len().checked_sub(1) else {
        return Ok(());
    };
    values.swap(0, last);
    sift(&mut values[..last], 0, &mut compare)
}

/// Find all equal keys in an already sorted slice without allocating.
/// The comparator orders each element relative to the searched key; it must
/// produce Less, then Equal, then Greater. Empty and absent runs are valid.
/// At most twice the slice's binary-search depth is compared.
///
/// # Errors
/// Returns the comparator's refusal immediately, without changing the slice.
pub fn try_equal_range_by<T, E>(
    values: &[T],
    mut compare: impl FnMut(&T) -> Result<Ordering, E>,
) -> Result<core::ops::Range<usize>, E> {
    let mut ends = [0; 2];
    for (side, end) in ends.iter_mut().enumerate() {
        let (mut lower, mut upper) = (0, values.len());
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let order = compare(&values[middle])?;
            if order.is_lt() || (side == 1 && order.is_eq()) {
                lower = middle + 1;
            } else {
                upper = middle;
            }
        }
        *end = lower;
    }
    Ok(ends[0]..ends[1])
}

/// Remove adjacent equal elements using a fallible comparison.
///
/// Keeps the first member of each adjacent run. A sorted vector therefore
/// becomes unique without allocating. The callback can charge work and cancel.
///
/// # Errors
/// Returns the callback error immediately. On refusal no element is removed;
/// their order may have changed. Only success promises deduplicated output.
pub fn try_dedup_by<T, E>(
    values: &mut Vec<T>,
    same: impl FnMut(&T, &T) -> Result<bool, E>,
) -> Result<(), E> {
    let kept = try_partition_unique_by(values, same)?;
    values.truncate(kept);
    Ok(())
}

/// Partition adjacent equal runs into a unique prefix without allocating or
/// destroying any element. The caller truncates its original owned container
/// only after success, keeping the first element of each run.
///
/// # Errors
/// Returns the comparator failure with every original element still present.
pub fn try_partition_unique_by<T, E>(
    values: &mut [T],
    mut same: impl FnMut(&T, &T) -> Result<bool, E>,
) -> Result<usize, E> {
    let mut kept = 0;
    for read in 0..values.len() {
        if kept > 0 && same(&values[kept - 1], &values[read])? {
            continue;
        }
        values.swap(kept, read);
        kept += 1;
    }
    Ok(kept)
}

fn sift<T, E>(
    values: &mut [T],
    root: usize,
    compare: &mut impl FnMut(&T, &T) -> Result<Ordering, E>,
) -> Result<(), E> {
    SiftFrame::new(root)
        .resume(values, compare, &mut || true)
        .map(|complete| {
            debug_assert!(
                complete,
                "an unlimited comparison grant completes the original sift"
            );
        })
}

#[cfg(test)]
mod tests {
    use super::{
        try_dedup_by, try_equal_range_by, try_heap_pop_by, try_heap_push_by, try_sort_unstable_by,
    };
    use core::cmp::Ordering;

    #[test]
    fn retained_heap_sort_resumes_every_comparison_cut_without_replay() {
        for len in [0, 1, 2, 3, 16, 63, 96] {
            let original: Vec<_> = (0..len).map(|index| (index * 53) % 17).collect();
            let mut expected = original.clone();
            let mut total = 0;
            let mut reference = super::HeapSortFrame::new(len);
            assert!(
                reference
                    .resume(
                        &mut expected,
                        |a, b| {
                            total += 1;
                            Ok::<_, ()>(a.cmp(b))
                        },
                        || true
                    )
                    .unwrap()
            );
            for cut in 0..=total {
                let mut values = original.clone();
                let mut frame = super::HeapSortFrame::new(len);
                let calls = core::cell::Cell::new(0);
                let mut left = cut;
                let compare = |a: &usize, b: &usize| {
                    calls.set(calls.get() + 1);
                    Ok::<_, ()>(a.cmp(b))
                };
                let completed = frame
                    .resume(&mut values, compare, || {
                        if left == 0 {
                            false
                        } else {
                            left -= 1;
                            true
                        }
                    })
                    .unwrap();
                assert_eq!(completed, cut == total);
                if !completed {
                    assert!(frame.resume(&mut values, compare, || true).unwrap());
                }
                assert_eq!(values, expected);
                assert_eq!(calls.get(), total, "length {len}, cut {cut}");
            }
            assert!(expected.windows(2).all(|pair| pair[0] <= pair[1]));
        }
    }

    #[test]
    fn fallible_heap_uses_original_inventory_and_matches_independent_heap() {
        use std::collections::BinaryHeap;
        let mut original = BinaryHeap::new();
        let mut values = Vec::new();
        for index in 0..1024 {
            let value = (53 * index) % 127;
            original.push(value);
            values.push(value);
            try_heap_push_by(&mut values, |a, b| Ok::<_, ()>(a.cmp(b))).unwrap();
            assert_eq!(values.first(), original.peek());
        }
        while let Some(expected) = original.pop() {
            try_heap_pop_by(&mut values, |a, b| Ok::<_, ()>(a.cmp(b))).unwrap();
            assert_eq!(values.pop(), Some(expected));
            assert_eq!(values.first(), original.peek());
        }
        try_heap_pop_by(&mut values, |a, b| Ok::<_, ()>(a.cmp(b))).unwrap();
    }

    #[test]
    fn heap_comparator_refusal_preserves_every_element_and_stops_immediately() {
        let mut heap = Vec::new();
        for value in 0..63 {
            heap.push(value);
            try_heap_push_by(&mut heap, |a, b| Ok::<_, ()>(a.cmp(b))).unwrap();
        }
        for push in [false, true] {
            let mut original = heap.clone();
            if push {
                original.push(100);
            }
            let mut measured = original.clone();
            let mut calls = 0;
            let mut count = |a: &i32, b: &i32| {
                calls += 1;
                Ok::<_, usize>(a.cmp(b))
            };
            if push {
                try_heap_push_by(&mut measured, &mut count).unwrap();
            } else {
                try_heap_pop_by(&mut measured, &mut count).unwrap();
            }
            let mut expected = original.clone();
            expected.sort_unstable();
            for refused_at in 1..=calls {
                let mut values = original.clone();
                let mut seen = 0;
                let mut compare = |a: &i32, b: &i32| {
                    seen += 1;
                    if seen == refused_at {
                        Err(refused_at)
                    } else {
                        Ok(a.cmp(b))
                    }
                };
                let result = if push {
                    try_heap_push_by(&mut values, &mut compare)
                } else {
                    try_heap_pop_by(&mut values, &mut compare)
                };
                assert_eq!(result, Err(refused_at));
                assert_eq!(seen, refused_at);
                values.sort_unstable();
                assert_eq!(values, expected);
            }
        }
    }

    #[test]
    fn large_adversarial_orders_match_independent_sort_and_equal_runs_use_linear_comparisons() {
        for length in [17_usize, 31, 32, 63, 127, 1024, 4096] {
            for original in [
                (0..length).collect::<Vec<_>>(),
                (0..length).rev().collect(),
                (0..length).map(|index| index % 7).collect(),
                (0..length).map(|index| index.min(length - index)).collect(),
                (0..length).map(|index| (53 * index) % length).collect(),
                vec![3; length],
            ] {
                let mut expected = original.clone();
                expected.sort_unstable();
                let mut values = original;
                let mut comparisons = 0;
                try_sort_unstable_by(&mut values, |left, right| {
                    comparisons += 1;
                    Ok::<_, ()>(left.cmp(right))
                })
                .unwrap();
                assert_eq!(values, expected);
                assert!(comparisons <= 128 * length * length.bit_width() as usize);
                if expected.first() == expected.last() {
                    assert_eq!(comparisons, length + 2);
                }
            }
        }
    }

    #[test]
    fn maximal_zero_sized_inventory_refuses_without_index_overflow() {
        let mut values = vec![(); usize::MAX];
        let mut comparisons = 0;
        assert_eq!(
            try_sort_unstable_by(&mut values, |(), ()| {
                comparisons += 1;
                Err::<Ordering, _>("first comparison")
            }),
            Err("first comparison"),
        );
        assert_eq!(comparisons, 1);
        assert_eq!(values.len(), usize::MAX);
    }

    #[test]
    fn equal_ranges_match_linear_partition_and_refuse_at_each_comparison() {
        for length in 0..=100 {
            let values: Vec<_> = (0..length).map(|index| index / 3).collect();
            for key in -1..=length / 3 + 1 {
                let first = values
                    .iter()
                    .position(|value| *value >= key)
                    .unwrap_or(values.len());
                let last = values
                    .iter()
                    .position(|value| *value > key)
                    .unwrap_or(values.len());
                let mut calls = 0;
                let range = try_equal_range_by(&values, |value| {
                    calls += 1;
                    Ok::<_, usize>(value.cmp(&key))
                })
                .unwrap();
                assert_eq!(range, first..last);
                for refused_at in 1..=calls {
                    let mut seen = 0;
                    assert_eq!(
                        try_equal_range_by(&values, |value| {
                            seen += 1;
                            if seen == refused_at {
                                Err(refused_at)
                            } else {
                                Ok(value.cmp(&key))
                            }
                        }),
                        Err(refused_at)
                    );
                    assert_eq!(seen, refused_at);
                }
            }
        }
    }

    #[test]
    fn every_small_repeated_key_sequence_matches_independent_ordering() {
        for length in 0..=8 {
            for mut code in 0..3_u32.pow(length) {
                let mut values = Vec::new();
                for _ in 0..length {
                    values.push(code % 3);
                    code /= 3;
                }
                let mut expected = values.clone();
                expected.sort_unstable();
                try_sort_unstable_by(&mut values, |a, b| Ok::<_, ()>(a.cmp(b)))
                    .expect("infallible");
                assert_eq!(values, expected);
            }
        }
    }

    #[test]
    fn every_comparison_refusal_stops_immediately_without_losing_elements() {
        let original: Vec<_> = (0..100).rev().collect();
        let mut measured = original.clone();
        let mut comparisons = 0;
        try_sort_unstable_by(&mut measured, |a, b| {
            comparisons += 1;
            Ok::<_, ()>(a.cmp(b))
        })
        .expect("measured");
        for refused_at in 1..=comparisons {
            let mut values = original.clone();
            let mut count = 0;
            assert_eq!(
                try_sort_unstable_by(&mut values, |a, b| {
                    count += 1;
                    if count == refused_at {
                        Err(refused_at)
                    } else {
                        Ok(a.cmp(b))
                    }
                }),
                Err(refused_at)
            );
            assert_eq!(count, refused_at);
            values.sort_unstable();
            assert_eq!(values, measured);
        }
    }
    #[test]
    fn fallible_dedup_matches_adjacent_runs_and_preserves_elements_on_refusal() {
        let original = vec![1, 1, 2, 2, 1, 1, 3, 3];
        let mut expected = original.clone();
        expected.dedup();
        let mut values = original.clone();
        try_dedup_by(&mut values, |a, b| Ok::<_, ()>(a == b)).expect("infallible");
        assert_eq!(values, expected);
        for refused_at in 1..original.len() {
            let mut values = original.clone();
            let mut calls = 0;
            assert_eq!(
                try_dedup_by(&mut values, |a, b| {
                    calls += 1;
                    if calls == refused_at {
                        Err(refused_at)
                    } else {
                        Ok(a == b)
                    }
                }),
                Err(refused_at)
            );
            assert_eq!(calls, refused_at);
            values.sort_unstable();
            let mut sorted = original.clone();
            sorted.sort_unstable();
            assert_eq!(values, sorted);
        }
    }
}

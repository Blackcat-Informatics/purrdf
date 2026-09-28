// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The work list every whole-tree walk keeps: a stack whose first `N` entries live
//! inline, in the walk's own frame, and whose rest spill to the heap.
//!
//! A walk over a shallow tree — the common case — never grows past `N` pending
//! entries, so it allocates nothing of its own: an iterative `Clone`, `==`, `Hash` or
//! drop costs the allocations of the tree it builds and no others. A deep or wide tree
//! spills, and is walked in heap memory rather than on the machine stack.

/// A stack holding its first `N` entries inline and the rest on the heap.
pub struct WorkList<T, const N: usize> {
    inline: [Option<T>; N],
    /// How many of `inline`'s slots are occupied, from the bottom.
    held: usize,
    /// Entries past the first `N`; non-empty only when `inline` is full.
    spill: Vec<T>,
}

impl<T, const N: usize> core::fmt::Debug for WorkList<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WorkList")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl<T, const N: usize> WorkList<T, N> {
    /// An empty stack; it allocates nothing until it holds more than `N` entries.
    pub(crate) fn new() -> Self {
        Self {
            inline: core::array::from_fn(|_| None),
            held: 0,
            spill: Vec::new(),
        }
    }

    /// A stack holding `first`.
    pub(crate) fn with(first: T) -> Self {
        let mut stack = Self::new();
        stack.push(first);
        stack
    }

    pub(crate) fn push(&mut self, value: T) {
        if self.held < N {
            self.inline[self.held] = Some(value);
            self.held += 1;
        } else {
            self.spill.push(value);
        }
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if let Some(value) = self.spill.pop() {
            return Some(value);
        }
        if self.held == 0 {
            return None;
        }
        self.held -= 1;
        self.inline[self.held].take()
    }

    pub(crate) fn len(&self) -> usize {
        self.held + self.spill.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.held == 0
    }

    /// The top entry.
    pub(crate) fn top_mut(&mut self) -> Option<&mut T> {
        if let Some(top) = self.spill.last_mut() {
            return Some(top);
        }
        self.held
            .checked_sub(1)
            .and_then(|top| self.inline[top].as_mut())
    }

    /// Push every entry of `values`, in order.
    pub(crate) fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        for value in values {
            self.push(value);
        }
    }

    /// The entry at `index` from the bottom, which the caller knows exists.
    fn slot(&mut self, index: usize) -> &mut T {
        if index < N {
            self.inline[index]
                .as_mut()
                .expect("an occupied slot below the stack's length")
        } else {
            &mut self.spill[index - N]
        }
    }

    /// Reverse the order of the top `count` entries: what was pushed in order pops in
    /// the same order.
    pub(crate) fn reverse_top(&mut self, count: usize) {
        let len = self.len();
        let (mut low, mut high) = (len - count, len);
        while low + 1 < high {
            high -= 1;
            if high < N {
                self.inline.swap(low, high);
            } else if low >= N {
                self.spill.swap(low - N, high - N);
            } else {
                let lower = self.inline[low]
                    .take()
                    .expect("an occupied slot below the stack's length");
                let upper = core::mem::replace(&mut self.spill[high - N], lower);
                self.inline[low] = Some(upper);
            }
            low += 1;
        }
    }

    /// Replace the entry at `index` from the bottom.
    pub(crate) fn set(&mut self, index: usize, value: T) {
        *self.slot(index) = value;
    }
}

#[cfg(test)]
mod tests {
    use super::WorkList;

    #[test]
    fn it_pops_in_reverse_push_order_across_the_spill() {
        let mut stack: WorkList<usize, 3> = WorkList::new();
        for i in 0..10 {
            stack.push(i);
        }
        assert_eq!(stack.len(), 10);
        let popped: Vec<_> = core::iter::from_fn(|| stack.pop()).collect();
        assert_eq!(popped, (0..10).rev().collect::<Vec<_>>());
    }

    #[test]
    fn reversing_the_top_crosses_the_inline_boundary() {
        for pushed in 0..9 {
            for count in 0..=pushed {
                let mut stack: WorkList<usize, 4> = WorkList::new();
                for i in 0..pushed {
                    stack.push(i);
                }
                stack.reverse_top(count);
                let popped: Vec<_> = core::iter::from_fn(|| stack.pop()).collect();
                let mut expected: Vec<_> = (0..pushed).collect();
                expected[pushed - count..].reverse();
                expected.reverse();
                assert_eq!(popped, expected, "pushed {pushed}, reversed {count}");
            }
        }
    }

    #[test]
    fn set_replaces_an_entry_in_either_storage() {
        let mut stack: WorkList<usize, 2> = WorkList::new();
        for i in 0..4 {
            stack.push(i);
        }
        stack.set(1, 10);
        stack.set(3, 30);
        let popped: Vec<_> = core::iter::from_fn(|| stack.pop()).collect();
        assert_eq!(popped, [30, 2, 10, 0]);
    }
}

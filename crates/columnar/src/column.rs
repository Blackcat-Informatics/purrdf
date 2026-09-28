// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The in-memory shape of a flat nullable column, laid out the way Parquet
//! stores it: a definition level per row, and the present values packed densely.
//!
//! A row is not an `Option<i64>`. That array-of-structs layout puts a
//! discriminant word beside every payload, so the PLAIN value copy (which
//! writes present values only, back to back) has to read each payload under its
//! discriminant, and a vectorizer can only do that with a masked load. Split
//! into a presence bitmap and a dense `Vec<i64>`, the PLAIN body *is* the value
//! vector in little-endian bytes: encode and decode are one contiguous copy, and
//! a presence count is a popcount over the bitmap.

/// One presence bit per row: bit `row % 64` of word `row / 64` is set when the
/// row holds a value (Parquet definition level 1) and clear when it is null
/// (level 0).
///
/// Every bit at or past `len` is clear, so two bitmaps over the same rows are
/// equal exactly when their words are, and a popcount of the words counts the
/// present rows and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Presence {
    words: Vec<u64>,
    len: usize,
}

const WORD_BITS: usize = u64::BITS as usize;

impl Presence {
    /// An empty bitmap with room for `rows` rows before it reallocates.
    pub(crate) fn with_capacity(rows: usize) -> Self {
        Self {
            words: Vec::with_capacity(rows.div_ceil(WORD_BITS)),
            len: 0,
        }
    }

    /// `rows` rows, every one present: a REQUIRED column's definition levels.
    pub(crate) fn full(rows: usize) -> Self {
        let mut presence = Self::with_capacity(rows);
        presence.push_run(true, rows);
        presence
    }

    /// The number of rows.
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    /// Append one row.
    pub(crate) fn push(&mut self, present: bool) {
        let bit = self.len % WORD_BITS;
        if bit == 0 {
            self.words.push(0);
        }
        if present {
            let last = self.words.len() - 1;
            self.words[last] |= 1 << bit;
        }
        self.len += 1;
    }

    /// Append `run` rows that are all present or all null: the partial word
    /// first, then whole words, then the partial word that ends the run.
    pub(crate) fn push_run(&mut self, present: bool, run: usize) {
        let mut remaining = run;
        let bit = self.len % WORD_BITS;
        if bit != 0 && remaining > 0 {
            let take = remaining.min(WORD_BITS - bit);
            if present {
                let last = self.words.len() - 1;
                self.words[last] |= low_bits(take) << bit;
            }
            self.len += take;
            remaining -= take;
        }
        let whole = remaining / WORD_BITS;
        self.words.extend(std::iter::repeat_n(
            if present { u64::MAX } else { 0 },
            whole,
        ));
        self.len += whole * WORD_BITS;
        remaining -= whole * WORD_BITS;
        if remaining > 0 {
            self.words
                .push(if present { low_bits(remaining) } else { 0 });
            self.len += remaining;
        }
    }

    /// Whether `row` holds a value.
    pub(crate) fn get(&self, row: usize) -> bool {
        debug_assert!(row < self.len, "row {row} of a {}-row bitmap", self.len);
        self.words[row / WORD_BITS] >> (row % WORD_BITS) & 1 == 1
    }

    /// The number of present rows: a popcount of the words, which holds no bit
    /// past the last row.
    pub(crate) fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Each row's presence, in row order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        (0..self.len).map(|row| self.get(row))
    }

    /// The maximal runs of equal presence, in row order: `(present, length)`,
    /// each length at least one, and no two adjacent runs alike. These are the
    /// RLE runs of the definition levels.
    pub(crate) fn runs(&self) -> Runs<'_> {
        Runs {
            presence: self,
            row: 0,
        }
    }
}

impl FromIterator<bool> for Presence {
    fn from_iter<I: IntoIterator<Item = bool>>(rows: I) -> Self {
        let rows = rows.into_iter();
        let mut presence = Self::with_capacity(rows.size_hint().0);
        for present in rows {
            presence.push(present);
        }
        presence
    }
}

/// A word with its low `bits` bits set, `bits` in `1..=64`.
const fn low_bits(bits: usize) -> u64 {
    u64::MAX >> (WORD_BITS - bits)
}

/// The maximal equal-presence runs of a [`Presence`] bitmap.
///
/// Each run's end is found a word at a time: the word is shifted so the run's
/// first row is bit 0, inverted when the run is present, and its trailing zeros
/// are the rows the run covers in that word.
#[derive(Debug, Clone)]
pub(crate) struct Runs<'a> {
    presence: &'a Presence,
    row: usize,
}

impl Iterator for Runs<'_> {
    type Item = (bool, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.presence.len;
        if self.row >= len {
            return None;
        }
        let start = self.row;
        let present = self.presence.get(start);
        let mut row = start;
        while row < len {
            let shift = row % WORD_BITS;
            let word = self.presence.words[row / WORD_BITS] >> shift;
            let differing = if present { !word } else { word };
            // Past the shifted-out rows the high bits read as "differs" when the
            // run is present and "same" when it is null; both are bounded below
            // by the word's own width.
            let same = (differing.trailing_zeros() as usize).min(WORD_BITS - shift);
            row += same;
            if same < WORD_BITS - shift {
                break;
            }
        }
        let end = row.min(len);
        self.row = end;
        Some((present, end - start))
    }
}

/// A nullable `INT64` column: the row presence and the present values, dense
/// and in row order. `values.len()` is always `presence.count()`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Int64Column {
    presence: Presence,
    values: Vec<i64>,
}

impl Int64Column {
    /// An empty column with room for `rows` rows.
    pub(crate) fn with_capacity(rows: usize) -> Self {
        Self {
            presence: Presence::with_capacity(rows),
            values: Vec::with_capacity(rows),
        }
    }

    /// A column over `presence` whose present rows hold `values`, in order.
    pub(crate) fn from_parts(presence: Presence, values: Vec<i64>) -> Self {
        debug_assert_eq!(presence.count(), values.len(), "one value per present row");
        Self { presence, values }
    }

    /// Append one row.
    pub(crate) fn push(&mut self, value: Option<i64>) {
        self.presence.push(value.is_some());
        if let Some(value) = value {
            self.values.push(value);
        }
    }

    /// The number of rows.
    pub(crate) const fn len(&self) -> usize {
        self.presence.len()
    }

    /// The number of null rows.
    pub(crate) fn null_count(&self) -> usize {
        self.presence.len() - self.values.len()
    }

    /// The row presence.
    pub(crate) const fn presence(&self) -> &Presence {
        &self.presence
    }

    /// The present values, in row order.
    pub(crate) fn values(&self) -> &[i64] {
        &self.values
    }

    /// The present values, in row order, for a test that rewrites one.
    #[cfg(test)]
    pub(crate) fn values_mut(&mut self) -> &mut [i64] {
        &mut self.values
    }

    /// Each row in order: its value, or `None` when it is null.
    pub(crate) fn rows(&self) -> Int64Rows<'_> {
        Int64Rows {
            presence: &self.presence,
            values: self.values.iter(),
            row: 0,
        }
    }
}

impl FromIterator<Option<i64>> for Int64Column {
    fn from_iter<I: IntoIterator<Item = Option<i64>>>(rows: I) -> Self {
        let rows = rows.into_iter();
        let mut column = Self::with_capacity(rows.size_hint().0);
        for value in rows {
            column.push(value);
        }
        column
    }
}

/// The rows of an [`Int64Column`], each `Some(value)` or `None`.
#[derive(Debug, Clone)]
pub(crate) struct Int64Rows<'a> {
    presence: &'a Presence,
    values: std::slice::Iter<'a, i64>,
    row: usize,
}

impl Int64Rows<'_> {
    /// The next row, which the caller knows exists: every column of a
    /// validated table holds the table's row count.
    pub(crate) fn next_row(&mut self) -> Option<i64> {
        self.next()
            .expect("a validated table's columns hold every row")
    }
}

impl Iterator for Int64Rows<'_> {
    type Item = Option<i64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.row >= self.presence.len() {
            return None;
        }
        let present = self.presence.get(self.row);
        self.row += 1;
        Some(if present {
            Some(*self.values.next().expect("one value per present row"))
        } else {
            None
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.presence.len() - self.row;
        (left, Some(left))
    }
}

impl ExactSizeIterator for Int64Rows<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_testkit::rng::splitmix64_next as mix;

    /// Presence patterns that cross every word boundary shape: empty, all
    /// present, all null, alternating, seeded runs (short and word-spanning)
    /// and a lone flip at every position of a short pattern.
    fn patterns() -> Vec<Vec<bool>> {
        let mut state = 0x5EED_u64;
        let mut out = Vec::new();
        for len in (0..=130).chain([191, 192, 193, 1_000, 4_099]) {
            out.push(vec![true; len]);
            out.push(vec![false; len]);
            out.push((0..len).map(|i| i % 2 == 0).collect());
            out.push((0..len).map(|i| i % 3 != 0).collect());
            for max_run in [3_u64, 70, 200] {
                let mut rows = Vec::with_capacity(len);
                let mut present = mix(&mut state).is_multiple_of(2);
                while rows.len() < len {
                    let run = 1 + (mix(&mut state) % max_run) as usize;
                    rows.extend(std::iter::repeat_n(present, run.min(len - rows.len())));
                    present = !present;
                }
                out.push(rows);
            }
            if len <= 70 {
                for at in 0..len {
                    out.push((0..len).map(|i| i != at).collect());
                    out.push((0..len).map(|i| i == at).collect());
                }
            }
        }
        out
    }

    fn naive_runs(rows: &[bool]) -> Vec<(bool, usize)> {
        let mut runs: Vec<(bool, usize)> = Vec::new();
        for &row in rows {
            match runs.last_mut() {
                Some((present, len)) if *present == row => *len += 1,
                _ => runs.push((row, 1)),
            }
        }
        runs
    }

    #[test]
    fn presence_answers_as_the_bool_vector_it_was_built_from() {
        for rows in patterns() {
            let presence: Presence = rows.iter().copied().collect();
            assert_eq!(presence.len(), rows.len());
            assert_eq!(presence.iter().collect::<Vec<_>>(), rows);
            assert_eq!(presence.count(), rows.iter().filter(|p| **p).count());
            assert_eq!(presence.runs().collect::<Vec<_>>(), naive_runs(&rows));

            // Rebuilt from its runs, a run at a time, it is the same bitmap:
            // `push_run` leaves no bit set past the last row.
            let mut rebuilt = Presence::with_capacity(0);
            for (present, run) in naive_runs(&rows) {
                rebuilt.push_run(present, run);
            }
            assert_eq!(rebuilt, presence, "{rows:?}");
        }
    }

    #[test]
    fn full_presence_is_the_all_true_bitmap() {
        for len in [0, 1, 63, 64, 65, 128, 1_000] {
            let full = Presence::full(len);
            assert_eq!(full, vec![true; len].into_iter().collect::<Presence>());
            assert_eq!(full.count(), len);
        }
    }

    #[test]
    fn int64_column_rows_round_trip_through_the_split_layout() {
        let mut state = 0xC011_u64;
        for rows in patterns() {
            let options: Vec<Option<i64>> = rows
                .iter()
                .map(|&present| present.then(|| mix(&mut state).cast_signed()))
                .collect();
            let column: Int64Column = options.iter().copied().collect();
            assert_eq!(column.len(), options.len());
            assert_eq!(
                column.null_count(),
                options.iter().filter(|v| v.is_none()).count()
            );
            assert_eq!(
                column.values(),
                options.iter().flatten().copied().collect::<Vec<_>>()
            );
            assert_eq!(column.rows().collect::<Vec<_>>(), options);
            let rebuilt =
                Int64Column::from_parts(column.presence().clone(), column.values().to_vec());
            assert_eq!(rebuilt, column);
        }
    }
}

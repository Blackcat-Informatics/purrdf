// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed integer-word operations shared by lexical conversion and BigInt.

use core::num::NonZeroU64;

/// Divide little-endian unsigned words in place by one nonzero word.
///
/// Returns the significant quotient length and remainder. Leading zero words
/// are accepted; all words outside the returned length are zero. The caller
/// owns storage and work admission. This operation allocates nothing.
pub fn div_rem_word(words: &mut [u64], divisor: NonZeroU64) -> (usize, u64) {
    let divisor = u128::from(divisor.get());
    let mut remainder = 0_u128;
    for word in words.iter_mut().rev() {
        let current = (remainder << 64) | u128::from(*word);
        *word = (current / divisor) as u64;
        remainder = current % divisor;
    }
    let length = words
        .iter()
        .rposition(|&word| word != 0)
        .map_or(0, |index| index + 1);
    (length, remainder as u64)
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The seeded mixed-script input stream the frozen word vectors were recorded
//! over. Changing anything here invalidates `word_differential_vectors.txt`.

use purrdf_testkit::rng::Xoshiro256;

/// The seed the frozen vectors were recorded under.
pub(crate) const SEED: u64 = 0x5eed_0f0a_2915;
/// How many inputs the frozen vectors hold.
pub(crate) const COUNT: usize = 100_000;

/// Inclusive scalar ranges the generator draws from, one script or class each.
const POOLS: &[(u32, u32)] = &[
    (0x20, 0x7E),       // ASCII
    (0x61, 0x7A),       // ASCII lowercase, weighted
    (0x30, 0x39),       // digits
    (0xA0, 0xFF),       // Latin-1
    (0x100, 0x24F),     // Latin Extended-A/B
    (0x300, 0x36F),     // combining diacritics
    (0x370, 0x3FF),     // Greek
    (0x400, 0x4FF),     // Cyrillic
    (0x590, 0x5FF),     // Hebrew
    (0x600, 0x6FF),     // Arabic
    (0x900, 0x97F),     // Devanagari
    (0xE00, 0xE7F),     // Thai
    (0x1100, 0x11FF),   // Hangul Jamo
    (0x1E00, 0x1FFF),   // Latin Extended Additional, Greek Extended
    (0x2000, 0x206F),   // General Punctuation (ZWJ, ZWNJ, spaces)
    (0x3000, 0x30FF),   // CJK punctuation, Hiragana, Katakana
    (0x4E00, 0x4E7F),   // Han
    (0xAC00, 0xAD00),   // Hangul syllables
    (0xFB00, 0xFB4F),   // ligatures, presentation forms
    (0xFF00, 0xFFEF),   // halfwidth and fullwidth forms
    (0x1F1E6, 0x1F1FF), // regional indicators
    (0x1F300, 0x1F6FF), // emoji
    (0x1F3FB, 0x1F3FF), // emoji modifiers
    (0x0, 0x10_FFFF),   // any scalar
];

/// Separators drawn between runs so boundaries fall in every context.
const JOINERS: &[char] = &[' ', '.', ',', '\'', ':', '_', '-', '\u{200D}', '\n', '\r'];

/// The next input from `rng`.
pub(crate) fn next_input(rng: &mut Xoshiro256) -> String {
    let len = rng.up_to(24);
    let mut out = String::new();
    for _ in 0..len {
        if rng.up_to(5) == 0 {
            let i = usize::try_from(rng.up_to(JOINERS.len() as u64 - 1)).unwrap_or(0);
            out.push(JOINERS[i]);
            continue;
        }
        let pool = POOLS[usize::try_from(rng.up_to(POOLS.len() as u64 - 1)).unwrap_or(0)];
        loop {
            let cp = pool.0 + u32::try_from(rng.up_to(u64::from(pool.1 - pool.0))).unwrap_or(0);
            if let Some(c) = char::from_u32(cp) {
                out.push(c);
                break;
            }
        }
    }
    out
}

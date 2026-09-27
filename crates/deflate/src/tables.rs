// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixed alphabets of RFC 1951 §3.2.5–§3.2.7, derived rather than typed.
//!
//! §3.2.5 tabulates the length and distance codes; both tables follow one rule
//! — the extra-bit count grows by one every four (lengths) or two (distances)
//! codes, and each base is the previous base plus the previous code's range —
//! with the single exception that length code 285 is 258 with no extra bits.
//! The tables are computed from that rule at compile time and their end points
//! asserted against the published ones.

/// Size of the sliding window, and the largest distance (RFC 1951 §2).
pub(crate) const WINDOW: usize = 32_768;
/// Shortest and longest match lengths (§3.2.5).
pub(crate) const MIN_MATCH: usize = 3;
pub(crate) const MAX_MATCH: usize = 258;
/// End-of-block symbol.
pub(crate) const END_OF_BLOCK: u16 = 256;
/// Literal/length symbols that may occur in data (0–285).
pub(crate) const LITLEN_SYMBOLS: usize = 286;
/// Distance symbols that may occur in data (0–29).
pub(crate) const DIST_SYMBOLS: usize = 30;
/// The largest code length of the literal/length and distance codes.
pub(crate) const MAX_CODE_LEN: u8 = 15;
/// The largest code length of the code-length code (3-bit fields).
pub(crate) const MAX_CL_CODE_LEN: u8 = 7;

/// The order code-length code lengths are sent in (§3.2.7).
pub(crate) const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

const fn length_extra(index: usize) -> u8 {
    if index < 8 || index == 28 {
        0
    } else {
        ((index - 4) / 4) as u8
    }
}

const fn distance_extra(index: usize) -> u8 {
    if index < 4 {
        0
    } else {
        ((index - 2) / 2) as u8
    }
}

/// Base length and extra-bit count of length symbols 257..=285, by `symbol − 257`.
pub(crate) const LENGTH_BASE: [u16; 29] = {
    let mut table = [0u16; 29];
    let mut base = 3u16;
    let mut index = 0;
    while index < 28 {
        table[index] = base;
        base += 1 << length_extra(index);
        index += 1;
    }
    table[28] = 258;
    table
};
pub(crate) const LENGTH_EXTRA: [u8; 29] = {
    let mut table = [0u8; 29];
    let mut index = 0;
    while index < 29 {
        table[index] = length_extra(index);
        index += 1;
    }
    table
};

/// Base distance and extra-bit count of distance symbols 0..=29.
pub(crate) const DIST_BASE: [u16; 30] = {
    let mut table = [0u16; 30];
    let mut base = 1u32;
    let mut index = 0;
    while index < 30 {
        table[index] = base as u16;
        base += 1 << distance_extra(index);
        index += 1;
    }
    table
};
pub(crate) const DIST_EXTRA: [u8; 30] = {
    let mut table = [0u8; 30];
    let mut index = 0;
    while index < 30 {
        table[index] = distance_extra(index);
        index += 1;
    }
    table
};

// The published end points of §3.2.5.
const _: () = assert!(LENGTH_BASE[0] == 3 && LENGTH_BASE[8] == 11 && LENGTH_EXTRA[8] == 1);
const _: () = assert!(LENGTH_BASE[27] == 227 && LENGTH_EXTRA[27] == 5);
const _: () = assert!(LENGTH_BASE[28] == 258 && LENGTH_EXTRA[28] == 0);
const _: () = assert!(DIST_BASE[4] == 5 && DIST_EXTRA[4] == 1);
const _: () = assert!(DIST_BASE[29] == 24_577 && DIST_EXTRA[29] == 13);
const _: () = assert!(DIST_BASE[29] as u32 + (1 << DIST_EXTRA[29]) - 1 == 32_768);

/// `match length − 3` (0..=255) → length-code index (`symbol − 257`).
pub(crate) const LENGTH_CODE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut code = 0;
    while code < 29 {
        let start = LENGTH_BASE[code] as usize - 3;
        let span = if code == 28 {
            1
        } else {
            1 << LENGTH_EXTRA[code]
        };
        let mut offset = 0;
        while offset < span && start + offset < 256 {
            table[start + offset] = code as u8;
            offset += 1;
        }
        code += 1;
    }
    // Length 258 is code 285 alone (not code 284 + 31).
    table[255] = 28;
    table
};

/// `distance − 1` below 256 → distance code.
pub(crate) const DIST_CODE_LOW: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut code = 0;
    while code < 16 {
        let start = DIST_BASE[code] as usize - 1;
        let span = 1usize << DIST_EXTRA[code];
        let mut offset = 0;
        while offset < span {
            table[start + offset] = code as u8;
            offset += 1;
        }
        code += 1;
    }
    table
};

/// `(distance − 1) >> 7` for distances above 256 → distance code (codes 16
/// and up start on multiples of 128, so the shift loses nothing).
pub(crate) const DIST_CODE_HIGH: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut code = 16;
    while code < 30 {
        let start = (DIST_BASE[code] as usize - 1) >> 7;
        let span = (1usize << DIST_EXTRA[code]) >> 7;
        let mut offset = 0;
        while offset < span {
            table[start + offset] = code as u8;
            offset += 1;
        }
        code += 1;
    }
    table
};

/// The distance code of `distance` (1..=32768).
#[inline]
pub(crate) fn distance_code(distance: u32) -> usize {
    let d = distance as usize - 1;
    if d < 256 {
        usize::from(DIST_CODE_LOW[d])
    } else {
        usize::from(DIST_CODE_HIGH[d >> 7])
    }
}

/// The fixed literal/length code lengths of §3.2.6 (all 288 symbols).
pub(crate) const FIXED_LITLEN_LENGTHS: [u8; 288] = {
    let mut table = [0u8; 288];
    let mut symbol = 0;
    while symbol < 288 {
        table[symbol] = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
        symbol += 1;
    }
    table
};

/// The fixed distance code lengths: 5 bits for all 32 symbols (§3.2.6).
pub(crate) const FIXED_DIST_LENGTHS: [u8; 32] = [5; 32];

/// Wire-order canonical codes for the fixed literal/length tree (§3.2.6).
/// The RFC assigns the four consecutive canonical ranges below; reversing
/// each code at compile time avoids rebuilding an invariant tree per stream.
pub(crate) const FIXED_LITLEN_CODES: [u16; 288] = {
    let mut codes = [0u16; 288];
    let mut symbol = 0;
    while symbol < codes.len() {
        let (code, bits) = match symbol {
            0..=143 => (0x30 + symbol, 8),
            144..=255 => (0x190 + symbol - 144, 9),
            256..=279 => (symbol - 256, 7),
            _ => (0xC0 + symbol - 280, 8),
        };
        codes[symbol] = crate::huffman::reverse_bits(code as u32, bits) as u16;
        symbol += 1;
    }
    codes
};

/// Wire-order canonical codes for the fixed distance tree (§3.2.6).
pub(crate) const FIXED_DIST_CODES: [u16; 32] = {
    let mut codes = [0u16; 32];
    let mut symbol = 0;
    while symbol < codes.len() {
        codes[symbol] = crate::huffman::reverse_bits(symbol as u32, 5) as u16;
        symbol += 1;
    }
    codes
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_codes_match_canonical_construction() {
        let mut literal_codes = [0u16; 288];
        let mut distance_codes = [0u16; 32];
        crate::huffman::canonical_codes(&FIXED_LITLEN_LENGTHS, &mut literal_codes);
        crate::huffman::canonical_codes(&FIXED_DIST_LENGTHS, &mut distance_codes);
        assert_eq!(FIXED_LITLEN_CODES, literal_codes);
        assert_eq!(FIXED_DIST_CODES, distance_codes);
    }

    #[test]
    fn every_length_maps_to_the_code_whose_range_holds_it() {
        for length in 3..=258usize {
            let code = usize::from(LENGTH_CODE[length - 3]);
            let base = usize::from(LENGTH_BASE[code]);
            let span = 1usize << LENGTH_EXTRA[code];
            assert!(base <= length && length < base + span, "length {length}");
        }
    }

    #[test]
    fn every_distance_maps_to_the_code_whose_range_holds_it() {
        for distance in 1..=32_768u32 {
            let code = distance_code(distance);
            let base = u32::from(DIST_BASE[code]);
            let span = 1u32 << DIST_EXTRA[code];
            assert!(
                base <= distance && distance < base + span,
                "distance {distance}"
            );
        }
    }
}

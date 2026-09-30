// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Text a test generates or reads: parser input nested to any depth, and a child
//! process's output.

use std::process::Output;

use crate::rng::SplitMix64;

/// `open` written `levels` times around `core`, closed by `close` written
/// `levels` times: a construct nested to any depth, built without recursion.
#[must_use]
pub fn nested(open: &str, core: &str, close: &str, levels: usize) -> String {
    format!("{}{core}{}", open.repeat(levels), close.repeat(levels))
}

/// `len` bytes of space-separated lowercase words drawn from `rng`, over letters
/// without `b`, `d`, `f`, `q` or digits: the haystack a regex bench plants its
/// needles in, so `needle`, `foo`, `bar`, dates and addresses occur only where the
/// bench puts them.
#[must_use]
pub fn lowercase_filler(rng: &mut SplitMix64, len: usize) -> String {
    const LETTERS: &[u8] = b"aceghijklmnoprstuvwxyz";
    let mut text = String::with_capacity(len + 8);
    while text.len() < len {
        for _ in 0..2 + rng.below_usize(8) {
            text.push(char::from(LETTERS[rng.below_usize(LETTERS.len())]));
        }
        text.push(' ');
    }
    text.truncate(len);
    text
}

/// stdout of a child process's [`Output`], which must be UTF-8.
///
/// # Panics
///
/// If it is not UTF-8.
#[must_use]
pub fn stdout_utf8(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

/// stderr of a child process's [`Output`], which must be UTF-8.
///
/// # Panics
///
/// If it is not UTF-8.
#[must_use]
pub fn stderr_utf8(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 stderr")
}

#[cfg(test)]
mod tests {
    use super::nested;

    #[test]
    fn the_core_sits_inside_every_level() {
        assert_eq!(nested("(", "x", ")", 3), "(((x)))");
        assert_eq!(nested("{", "x", "}", 0), "x");
    }
}

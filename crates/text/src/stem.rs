// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! English Snowball 3.1.0 over scalar positions and the pinned Latin script.
//!
//! The caller supplies the folded lexical word. A word is eligible exactly
//! when it contains a Latin letter and none of its letters belong to another
//! script. Surviving non-ASCII letters are consonants under the English rule.
//! Regions remain fixed scalar offsets while suffix rewrites shorten the word.
//! The implementation derives from the published prose specification; the
//! reference vocabulary supplies expected results, not implementation code.

use crate::unicode::{is_latin_letter, is_letter};

/// Versioned stemming algorithm and scalar/Latin eligibility law.
pub const PROFILE_ID: &str = "snowball-english-3.1.0-latin-scalars-v2";

/// Stem one folded lexical word, preserving words outside the Latin domain.
#[must_use]
pub fn english(input: &str) -> String {
    let mut word = input.to_owned();
    english_in_place(&mut word);
    word
}

/// Stem a folded lexical word without replacing its string allocation.
///
/// The suffix law never increases the original byte length. Working storage
/// holds scalar values and their vowel classifications, with no recursion.
pub fn english_in_place(word: &mut String) {
    let mut latin = false;
    for letter in word.chars() {
        if is_latin_letter(letter) {
            latin = true;
        } else if is_letter(letter) {
            return;
        }
    }
    if !latin || word.chars().take(3).count() < 3 {
        return;
    }
    if let Some(exception) = exception(word) {
        word.replace_range(.., exception);
        return;
    }
    let input = word.strip_prefix('\'').unwrap_or(word);
    let mut state = Stem::new(input);
    state.possessive();
    state.plural();
    state.participle();
    state.terminal_y();
    state.rewrite(&DERIVATIONAL);
    state.rewrite(&SECONDARY);
    state.rewrite(&REMOVABLE);
    state.final_letter();
    word.clear();
    word.extend(state.letters.iter().map(|letter| letter.value));
}

fn exception(word: &str) -> Option<&'static str> {
    Some(match word {
        "skis" => "ski",
        "skies" | "sky" => "sky",
        "idly" => "idl",
        "gently" => "gentl",
        "ugly" => "ugli",
        "early" => "earli",
        "only" => "onli",
        "singly" => "singl",
        "news" => "news",
        "howe" => "howe",
        "atlas" => "atlas",
        "cosmos" => "cosmos",
        "bias" => "bias",
        "andes" => "andes",
        _ => return None,
    })
}

#[derive(Clone, Copy)]
struct Letter {
    value: char,
    vowel: bool,
}

const fn simple_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

struct Stem {
    letters: Vec<Letter>,
    regions: [usize; 2],
}

impl Stem {
    fn new(input: &str) -> Self {
        let mut letters: Vec<Letter> = Vec::with_capacity(input.len());
        for value in input.chars() {
            let vowel = simple_vowel(value)
                && (value != 'y' || letters.last().is_some_and(|previous| !previous.vowel));
            letters.push(Letter { value, vowel });
        }
        let first = [
            "gener", "commun", "arsen", "past", "univers", "later", "emerg", "organ", "inter",
        ]
        .into_iter()
        .find(|prefix| input.starts_with(prefix))
        .map_or_else(|| region(&letters, 0), str::len);
        let second = region(&letters, first);
        Self {
            letters,
            regions: [first, second],
        }
    }

    fn len(&self) -> usize {
        self.letters.len()
    }

    fn ends(&self, suffix: &str) -> Option<usize> {
        let start = self.len().checked_sub(suffix.len())?;
        self.letters[start..]
            .iter()
            .map(|letter| letter.value)
            .eq(suffix.chars())
            .then_some(start)
    }

    fn longest<'a>(&self, suffixes: &'a [&str]) -> Option<(&'a str, usize)> {
        suffixes
            .iter()
            .filter_map(|&suffix| self.ends(suffix).map(|start| (suffix, start)))
            .max_by_key(|(suffix, _)| suffix.len())
    }

    fn replace(&mut self, start: usize, replacement: &str) {
        self.letters.truncate(start);
        self.letters.extend(replacement.chars().map(|value| Letter {
            value,
            vowel: simple_vowel(value),
        }));
    }

    fn possessive(&mut self) {
        if let Some((_, start)) = self.longest(&["'", "'s", "'s'"]) {
            self.replace(start, "");
        }
    }

    fn plural(&mut self) {
        let Some((suffix, start)) = self.longest(&["sses", "ied", "ies", "s", "us", "ss"]) else {
            return;
        };
        match suffix {
            "sses" => self.replace(start, "ss"),
            "ied" | "ies" => self.replace(start, if start > 1 { "i" } else { "ie" }),
            "s" if start > 0 && self.letters[..start - 1].iter().any(|letter| letter.vowel) => {
                self.replace(start, "");
            }
            _ => {}
        }
    }

    fn participle(&mut self) {
        let Some((suffix, start)) = self.longest(&["eed", "eedly", "ed", "edly", "ing", "ingly"])
        else {
            return;
        };
        if matches!(suffix, "eed" | "eedly") {
            let exceptional = ["proc", "exc", "succ"].iter().any(|prefix| {
                prefix.len() == start
                    && self.letters[..start]
                        .iter()
                        .map(|letter| letter.value)
                        .eq(prefix.chars())
            });
            if start >= self.regions[0] && !exceptional {
                self.replace(start, "ee");
            }
            return;
        }
        if suffix == "ing" {
            if start == 2 && !self.letters[0].vowel && self.letters[1].value == 'y' {
                self.replace(1, "ie");
                return;
            }
            if ["inn", "out", "cann", "herr", "earr", "even"]
                .iter()
                .any(|prefix| {
                    prefix.len() == start
                        && self.letters[..start]
                            .iter()
                            .map(|letter| letter.value)
                            .eq(prefix.chars())
                })
            {
                return;
            }
        }
        if !self.letters[..start].iter().any(|letter| letter.vowel) {
            return;
        }
        self.replace(start, "");
        if self.longest(&["at", "bl", "iz"]).is_some() {
            self.replace(self.len(), "e");
        } else if let Some((_, double_start)) =
            self.longest(&["bb", "dd", "ff", "gg", "mm", "nn", "pp", "rr", "tt"])
        {
            if double_start != 1 || !matches!(self.letters[0].value, 'a' | 'e' | 'o') {
                self.letters.pop();
            }
        } else if self.regions[0] >= self.len() && short_syllable(&self.letters) {
            self.replace(self.len(), "e");
        }
    }

    fn terminal_y(&mut self) {
        let len = self.len();
        if len >= 3 && self.letters[len - 1].value == 'y' && !self.letters[len - 2].vowel {
            self.replace(len - 1, "i");
        }
    }

    fn rewrite(&mut self, rules: &[Rule]) {
        let Some((rule, start)) = rules
            .iter()
            .filter_map(|rule| self.ends(rule.suffix).map(|start| (rule, start)))
            .max_by_key(|(rule, _)| rule.suffix.len())
        else {
            return;
        };
        if start < self.regions[rule.region] {
            return;
        }
        if !rule.predecessors.is_empty()
            && (start == 0 || !rule.predecessors.contains(self.letters[start - 1].value))
        {
            return;
        }
        self.replace(start, rule.replacement);
    }

    fn final_letter(&mut self) {
        if let Some(start) = self.ends("e") {
            if start >= self.regions[1]
                || (start >= self.regions[0] && !short_syllable(&self.letters[..start]))
            {
                self.replace(start, "");
            }
        } else if let Some(start) = self.ends("l")
            && start >= self.regions[1]
            && start > 0
            && self.letters[start - 1].value == 'l'
        {
            self.replace(start, "");
        }
    }
}

fn region(letters: &[Letter], start: usize) -> usize {
    letters[start..]
        .windows(2)
        .position(|pair| pair[0].vowel && !pair[1].vowel)
        .map_or(letters.len(), |position| start + position + 2)
}

fn short_syllable(letters: &[Letter]) -> bool {
    if letters.len() == 2 {
        return letters[0].vowel && !letters[1].vowel;
    }
    if letters
        .iter()
        .rev()
        .take(4)
        .map(|letter| letter.value)
        .eq("tsap".chars())
    {
        return true;
    }
    let Some(end) = letters.last_chunk::<3>() else {
        return false;
    };
    !end[0].vowel && end[1].vowel && !end[2].vowel && !matches!(end[2].value, 'w' | 'x' | 'y')
}

struct Rule {
    suffix: &'static str,
    replacement: &'static str,
    region: usize,
    predecessors: &'static str,
}

const fn rule(
    suffix: &'static str,
    replacement: &'static str,
    region: usize,
    predecessors: &'static str,
) -> Rule {
    Rule {
        suffix,
        replacement,
        region,
        predecessors,
    }
}

const DERIVATIONAL: [Rule; 25] = [
    rule("tional", "tion", 0, ""),
    rule("enci", "ence", 0, ""),
    rule("anci", "ance", 0, ""),
    rule("abli", "able", 0, ""),
    rule("entli", "ent", 0, ""),
    rule("izer", "ize", 0, ""),
    rule("ization", "ize", 0, ""),
    rule("ational", "ate", 0, ""),
    rule("ation", "ate", 0, ""),
    rule("ator", "ate", 0, ""),
    rule("alism", "al", 0, ""),
    rule("aliti", "al", 0, ""),
    rule("alli", "al", 0, ""),
    rule("fulness", "ful", 0, ""),
    rule("ousli", "ous", 0, ""),
    rule("ousness", "ous", 0, ""),
    rule("iveness", "ive", 0, ""),
    rule("iviti", "ive", 0, ""),
    rule("biliti", "ble", 0, ""),
    rule("bli", "ble", 0, ""),
    rule("ogist", "og", 0, ""),
    rule("ogi", "og", 0, "l"),
    rule("fulli", "ful", 0, ""),
    rule("lessli", "less", 0, ""),
    rule("li", "", 0, "cdeghkmnrt"),
];
const SECONDARY: [Rule; 9] = [
    rule("tional", "tion", 0, ""),
    rule("ational", "ate", 0, ""),
    rule("alize", "al", 0, ""),
    rule("icate", "ic", 0, ""),
    rule("iciti", "ic", 0, ""),
    rule("ical", "ic", 0, ""),
    rule("ful", "", 0, ""),
    rule("ness", "", 0, ""),
    rule("ative", "", 1, ""),
];
const REMOVABLE: [Rule; 18] = [
    rule("al", "", 1, ""),
    rule("ance", "", 1, ""),
    rule("ence", "", 1, ""),
    rule("er", "", 1, ""),
    rule("ic", "", 1, ""),
    rule("able", "", 1, ""),
    rule("ible", "", 1, ""),
    rule("ant", "", 1, ""),
    rule("ement", "", 1, ""),
    rule("ment", "", 1, ""),
    rule("ent", "", 1, ""),
    rule("ism", "", 1, ""),
    rule("ate", "", 1, ""),
    rule("iti", "", 1, ""),
    rule("ous", "", 1, ""),
    rule("ive", "", 1, ""),
    rule("ize", "", 1, ""),
    rule("ion", "", 1, "st"),
];

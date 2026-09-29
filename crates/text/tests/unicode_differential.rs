// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf_text::unicode` replayed against the frozen differential vectors.
//!
//! The two vector files hold answers recorded from the implementations this
//! layer replaced (answers only; see the headers of the files). Each
//! disagreement a replay admits is enumerated here, with the reason it is the
//! frozen answer, not this layer, that departs from the Unicode Standard:
//!
//! * case fold: the frozen table is `CaseFolding.txt` 16.0.0 and this layer's
//!   is 17.0.0, so they differ exactly on the code points whose `C`/`F`
//!   entries differ between the two vendored files — a set computed here from
//!   the files, not restated;
//! * word segmentation: the records listed in [`WORD_DISAGREEMENTS`], each
//!   decided by the `UAX #29` rule it names;
//! * the analysis form recorded beside each word list: no disagreement.

mod support {
    pub(crate) mod unicode_inputs;
}

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use purrdf_testkit::rng::Xoshiro256;
use purrdf_testkit::vectors::{VectorFile, decode_str, encode_str};
use purrdf_text::unicode;
use support::unicode_inputs;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

fn hex(text: &str) -> u32 {
    u32::from_str_radix(text.trim(), 16).unwrap_or_else(|err| panic!("bad hex {text:?}: {err}"))
}

/// The `C` and `F` mappings of one `CaseFolding.txt`.
fn full_folding(relative: &str) -> BTreeMap<u32, String> {
    read(relative)
        .lines()
        .filter_map(|line| {
            let data = line.split('#').next().unwrap_or("").trim();
            let fields: Vec<&str> = data.split(';').map(str::trim).collect();
            (fields.len() >= 3 && matches!(fields[1], "C" | "F")).then(|| {
                let mapping = fields[2]
                    .split_whitespace()
                    .map(|p| char::from_u32(hex(p)).expect("a scalar value"))
                    .collect();
                (hex(fields[0]), mapping)
            })
        })
        .collect()
}

/// The code points whose full fold differs between the vendored 16.0.0 and
/// 17.0.0 `CaseFolding.txt`.
fn fold_levelled_points() -> BTreeSet<u32> {
    let old = full_folding("../iri/unicode/16.0.0/CaseFolding.txt");
    let new = full_folding("../iri/unicode/17.0.0/CaseFolding.txt");
    old.keys()
        .chain(new.keys())
        .copied()
        .filter(|point| old.get(point) != new.get(point))
        .collect()
}

#[test]
fn the_case_fold_replays_except_where_17_0_0_folds_differently() {
    let text = read("tests/case_fold_differential_vectors.txt");
    let file = VectorFile::parse(&text).expect("a valid frozen vector file");
    let frozen: BTreeMap<u32, String> = file
        .records()
        .iter()
        .map(|record| {
            (
                hex(record.fields[0]),
                decode_str(record.fields[1]).expect("a valid field"),
            )
        })
        .collect();
    let levelled = fold_levelled_points();
    let new = full_folding("../iri/unicode/17.0.0/CaseFolding.txt");

    let mut disagreements: BTreeSet<u32> = BTreeSet::new();
    for point in 0..=0x10_FFFF_u32 {
        let Some(c) = char::from_u32(point) else {
            continue;
        };
        let itself = c.to_string();
        let ours = unicode::case_fold(&itself);
        let theirs = frozen.get(&point).unwrap_or(&itself);
        if &ours != theirs {
            disagreements.insert(point);
            assert_eq!(
                Some(&ours),
                new.get(&point),
                "U+{point:04X}: a disagreement must be the 17.0.0 mapping"
            );
        }
    }
    assert_eq!(
        disagreements, levelled,
        "the fold must disagree with the frozen 16.0.0 answers exactly where the two \
         CaseFolding.txt releases differ"
    );
    // The 17.0.0 additions: Latin Extended-D and Beria Erfe capitals.
    assert_eq!(levelled.len(), 28, "{levelled:X?}");
}

/// Word-vector records whose frozen word list contradicts `UAX #29`, as
/// `(ordinal, this layer's word list, rule)`.
///
/// Every one is the shape `AHLetter (MidLetter | MidNumLetQ) (Extend |
/// Format)* ZWJ \p{Extended_Pictographic}`, and the test checks that shape
/// against the vendored property files for each record. Rule WB4 attaches the
/// ZWJ to the punctuation, so the next character after the punctuation that
/// the rules see is the pictograph, which is not `AHLetter`: WB6
/// (`AHLetter × (MidLetter | MidNumLetQ) AHLetter`) does not hold, no later
/// rule joins, and WB999 breaks before the punctuation. The frozen answers
/// keep the punctuation, ZWJ and pictograph in the letter's segment. WB12
/// (`Numeric × (MidNum | MidNumLetQ) Numeric`) decides the same shape after a
/// digit and is checked the same way; this input stream holds no instance.
const WORD_DISAGREEMENTS: &[(usize, &str, &str)] = &[
    (1167, "0:1,4:5,13:5,26:3,29:4", "WB6"),
    (5031, "3:12,15:3,18:8,45:9", "WB6"),
    (11963, "0:6,20:2,30:2,32:3,35:3", "WB6"),
    (13121, "5:1,6:3,18:4,23:2,25:3,28:3,39:2,43:7", "WB6"),
    (
        31607,
        "1:2,7:2,9:3,12:3,18:3,21:3,24:5,29:3,40:15,63:2",
        "WB6",
    ),
    (46483, "0:1,3:9,16:4,32:9,41:6,47:2", "WB6"),
    (46523, "2:17,27:6,33:3,37:2", "WB6"),
    (46750, "0:1,2:3,5:3", "WB6"),
    (48245, "4:2,15:2,18:3,22:13,35:4,39:3,42:2", "WB6"),
    (56442, "0:11,15:2,28:2", "WB6"),
    (69402, "4:5,9:3,12:3,16:1,18:3,29:5,38:1", "WB6"),
    (
        69485,
        "0:3,3:5,20:3,23:3,27:2,29:3,33:8,41:3,45:3,48:3",
        "WB6",
    ),
    (73326, "3:3,11:1,21:5,26:3,29:3,32:3,40:3,43:3", "WB6"),
    (77272, "0:7,12:3,15:3,19:10,38:2", "WB6"),
];

/// `Word_Break` values and `Extended_Pictographic`, read from the vendored
/// files rather than from the tables under test.
struct WordProperties {
    word_break: BTreeMap<u32, String>,
    pictographic: BTreeSet<u32>,
}

impl WordProperties {
    fn load() -> Self {
        let ranges = |relative: &str| -> Vec<(u32, u32, String)> {
            read(relative)
                .lines()
                .filter_map(|line| {
                    let data = line.split('#').next().unwrap_or("").trim();
                    let (points, value) = data.split_once(';')?;
                    let (low, high) = points
                        .trim()
                        .split_once("..")
                        .map_or_else(|| (hex(points), hex(points)), |(l, h)| (hex(l), hex(h)));
                    Some((low, high, value.trim().to_owned()))
                })
                .collect()
        };
        let mut word_break = BTreeMap::new();
        for (low, high, value) in ranges("../iri/unicode/17.0.0/WordBreakProperty.txt") {
            for point in low..=high {
                word_break.insert(point, value.clone());
            }
        }
        let pictographic = ranges("../iri/unicode/17.0.0/emoji-data.txt")
            .into_iter()
            .filter(|(_, _, value)| value == "Extended_Pictographic")
            .flat_map(|(low, high, _)| low..=high)
            .collect();
        Self {
            word_break,
            pictographic,
        }
    }

    fn of(&self, c: char) -> &str {
        self.word_break
            .get(&(c as u32))
            .map_or("Other", String::as_str)
    }

    /// The rule that puts a boundary at byte `at` of `input` where the frozen
    /// answer has none: `WB6` or `WB12` when `input[at..]` is punctuation,
    /// then `(Extend | Format)*`, then ZWJ and an `Extended_Pictographic`
    /// character, and the character before `at` (past WB4's ignorables) is
    /// `AHLetter` or `Numeric` respectively. `None` for any other shape.
    fn wb6_wb12_shape(&self, input: &str, at: usize) -> Option<&'static str> {
        let ignorable = |c: char| matches!(self.of(c), "Extend" | "Format" | "ZWJ");
        let before = input[..at].chars().rev().find(|&c| !ignorable(c))?;
        let mut after = input[at..].chars();
        let punctuation = self.of(after.next()?);
        let mut rest = after.skip_while(|&c| matches!(self.of(c), "Extend" | "Format"));
        if rest.next().map(|c| self.of(c)) != Some("ZWJ") {
            return None;
        }
        if !rest
            .next()
            .is_some_and(|c| self.pictographic.contains(&(c as u32)))
        {
            return None;
        }
        match (self.of(before), punctuation) {
            ("ALetter" | "Hebrew_Letter", "MidLetter" | "MidNumLet" | "Single_Quote") => {
                Some("WB6")
            }
            ("Numeric", "MidNum" | "MidNumLet" | "Single_Quote") => Some("WB12"),
            _ => None,
        }
    }
}

/// Where this layer's word list first ends a word the frozen list continues:
/// the byte offset of that boundary.
fn first_shortened_word(ours: &str, frozen: &str) -> Option<usize> {
    let parse = |list: &str| -> Vec<(usize, usize)> {
        if list == "\\0" {
            return Vec::new();
        }
        list.split(',')
            .map(|entry| {
                let (offset, len) = entry.split_once(':').expect("offset:len");
                (offset.parse().expect("offset"), len.parse().expect("len"))
            })
            .collect()
    };
    let (ours, frozen) = (parse(ours), parse(frozen));
    let (&(offset, len), &(frozen_offset, frozen_len)) =
        ours.iter().zip(&frozen).find(|(a, b)| a != b)?;
    (offset == frozen_offset && len < frozen_len).then_some(offset + len)
}

/// Encode word indices the way the vector file does: `offset:len`, comma
/// separated, `\0` when there are none.
fn encode_words(text: &str) -> String {
    let words: Vec<String> = unicode::word_indices(text)
        .map(|(offset, word)| format!("{offset}:{}", word.len()))
        .collect();
    encode_str(&words.join(","))
}

#[test]
fn the_word_segmentation_and_analysis_form_replay() {
    let text = read("tests/word_differential_vectors.txt");
    let file = VectorFile::parse(&text).expect("a valid frozen vector file");
    let levelled = fold_levelled_points();
    let mut rng = Xoshiro256::from_seed(unicode_inputs::SEED);
    let listed: BTreeMap<usize, (&str, &str)> = WORD_DISAGREEMENTS
        .iter()
        .map(|&(ordinal, words, rule)| (ordinal, (words, rule)))
        .collect();
    assert_eq!(file.records().len(), unicode_inputs::COUNT);

    let properties = WordProperties::load();
    let mut word_disagreements: Vec<String> = Vec::new();
    let mut form_disagreements: Vec<String> = Vec::new();
    for (ordinal, record) in file.records().iter().enumerate() {
        let input = unicode_inputs::next_input(&mut rng);
        assert_eq!(
            record.fields[0],
            ordinal.to_string(),
            "records are in input order"
        );
        let mut form = String::new();
        unicode::analysis_form(&input, &mut form);

        let words = encode_words(&input);
        match listed.get(&ordinal) {
            Some(&(expected, rule)) => {
                assert_eq!(words, expected, "record {ordinal} ({rule})");
                let at = first_shortened_word(&words, record.fields[1])
                    .unwrap_or_else(|| panic!("record {ordinal} no longer shortens a word"));
                assert_eq!(
                    properties.wb6_wb12_shape(&input, at),
                    Some(rule),
                    "record {ordinal}: the boundary at byte {at} is not the {rule} shape"
                );
            }
            None => {
                if words != record.fields[1] {
                    let rule = first_shortened_word(&words, record.fields[1])
                        .and_then(|at| properties.wb6_wb12_shape(&input, at));
                    word_disagreements.push(format!(
                        "({ordinal}, {words:?}, {rule:?}), input {:?} frozen {}",
                        encode_str(&input),
                        record.fields[1]
                    ));
                }
            }
        }

        // The analysis form replays exactly. The fold levelling could only
        // move a form whose input holds one of the levelled capitals, and no
        // input of this stream does; that is asserted rather than assumed.
        assert!(
            !input.chars().any(|c| levelled.contains(&(c as u32))),
            "record {ordinal} holds a levelled code point; admit its form explicitly"
        );
        let encoded_form = encode_str(&form);
        if encoded_form != record.fields[2] {
            form_disagreements.push(format!(
                "record {ordinal}: input {:?}: frozen {} ours {encoded_form}",
                encode_str(&input),
                record.fields[2]
            ));
        }
    }
    assert!(
        word_disagreements.is_empty(),
        "{} unlisted word disagreements: {:#?}",
        word_disagreements.len(),
        &word_disagreements[..word_disagreements.len().min(30)]
    );
    assert!(
        form_disagreements.is_empty(),
        "{} analysis-form disagreements: {:#?}",
        form_disagreements.len(),
        &form_disagreements[..form_disagreements.len().min(30)]
    );
}

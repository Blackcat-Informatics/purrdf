// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Regenerates the committed analyzer tables (`crates/text/src/unicode_tables.rs`)
//! from the Unicode Character Database vendored under `crates/iri/unicode/`.
//!
//! Read: `UnicodeData.txt` (General_Category, Canonical_Combining_Class and
//! the decomposition mappings), `CompositionExclusions.txt`,
//! `DerivedCoreProperties.txt` (`Alphabetic`), `CaseFolding.txt` (statuses `C`
//! and `F`), `WordBreakProperty.txt` and `emoji-data.txt`
//! (`Extended_Pictographic`). File formats per UAX 44.
//!
//! Every per-code-point property is emitted as a two-stage table: one `u16`
//! block number per 128-code-point block, and the deduplicated blocks.
//! Decompositions are stored fully applied (recursively, with the Hangul
//! syllable arithmetic of the core specification §3.12), so the runtime maps
//! one character in one lookup. Primary composites are the two-character
//! canonical decompositions whose source is not `Full_Composition_Exclusion`,
//! derived as UAX 15 §5 describes.
//!
//! Output goes to stdout; `scripts/check-generated.sh` pipes it through
//! `rustfmt` and compares it with the committed file.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// Code points per second-stage block.
const BLOCK_SHIFT: u32 = 7;
/// One past the last code point.
const CODE_SPACE: u32 = 0x11_0000;

/// §3.12 Hangul constants.
const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT;
const S_COUNT: u32 = L_COUNT * N_COUNT;

/// `Word_Break` values, in the numbering the runtime uses.
const WORD_BREAK_VALUES: &[&str] = &[
    "Other",
    "CR",
    "LF",
    "Newline",
    "Extend",
    "ZWJ",
    "Regional_Indicator",
    "Format",
    "Katakana",
    "Hebrew_Letter",
    "ALetter",
    "Single_Quote",
    "Double_Quote",
    "MidNumLet",
    "MidLetter",
    "MidNum",
    "Numeric",
    "ExtendNumLet",
    "WSegSpace",
];
/// The segmentation byte's `Extended_Pictographic` bit.
const EXTENDED_PICTOGRAPHIC: u8 = 0x20;
/// The segmentation byte's alphanumeric bit.
const ALPHANUMERIC: u8 = 0x40;

/// One `UnicodeData.txt` decomposition mapping.
struct Mapping {
    /// Whether the mapping carries a `<tag>`, which makes it a compatibility
    /// mapping rather than a canonical one.
    compatibility: bool,
    chars: Vec<u32>,
}

/// What the generator needs from `UnicodeData.txt`.
#[derive(Default)]
struct UnicodeData {
    general_category: HashMap<u32, String>,
    combining_class: HashMap<u32, u8>,
    mappings: BTreeMap<u32, Mapping>,
}

fn read(dir: &Path, name: &str) -> String {
    let path = dir.join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

fn hex(text: &str) -> u32 {
    u32::from_str_radix(text.trim(), 16).unwrap_or_else(|err| panic!("bad hex {text:?}: {err}"))
}

/// The data lines of a UCD file: comments stripped, blanks skipped, fields
/// split on `;` and trimmed.
fn data_lines(text: &str) -> impl Iterator<Item = Vec<&str>> {
    text.lines().filter_map(|line| {
        let data = line.split('#').next().unwrap_or("").trim();
        (!data.is_empty()).then(|| data.split(';').map(str::trim).collect())
    })
}

/// A `XXXX` or `XXXX..YYYY` code point field as an inclusive range.
fn range(field: &str) -> (u32, u32) {
    match field.split_once("..") {
        Some((low, high)) => (hex(low), hex(high)),
        None => {
            let point = hex(field);
            (point, point)
        }
    }
}

/// The version a UCD file names in its first line, `# Name-X.Y.Z.txt`.
fn header_version(text: &str, name: &str) -> (u8, u8, u8) {
    let first = text.lines().next().unwrap_or("");
    let stem = name.trim_end_matches(".txt");
    let version = first
        .strip_prefix("# ")
        .and_then(|rest| rest.strip_prefix(stem))
        .and_then(|rest| rest.strip_prefix('-'))
        .and_then(|rest| rest.strip_suffix(".txt"))
        .unwrap_or_else(|| panic!("{name} does not open with its versioned file name: {first:?}"));
    let parts: Vec<u8> = version
        .split('.')
        .map(|part| {
            part.parse()
                .unwrap_or_else(|err| panic!("{name} version {version:?}: {err}"))
        })
        .collect();
    let [major, minor, patch] = parts[..] else {
        panic!("{name} version {version:?} is not X.Y.Z");
    };
    (major, minor, patch)
}

fn parse_unicode_data(text: &str) -> UnicodeData {
    let mut data = UnicodeData::default();
    let mut range_start: Option<u32> = None;
    for fields in text.lines().map(|line| line.split(';').collect::<Vec<_>>()) {
        if fields.len() < 6 {
            continue;
        }
        let point = hex(fields[0]);
        let name = fields[1];
        let category = fields[2].to_owned();
        let class: u8 = fields[3]
            .parse()
            .unwrap_or_else(|err| panic!("bad combining class for {point:04X}: {err}"));
        let points = if name.ends_with(", First>") {
            range_start = Some(point);
            continue;
        } else if name.ends_with(", Last>") {
            let start = range_start
                .take()
                .unwrap_or_else(|| panic!("range end {point:04X} without a start"));
            start..=point
        } else {
            point..=point
        };
        for p in points {
            data.general_category.insert(p, category.clone());
            if class != 0 {
                data.combining_class.insert(p, class);
            }
        }
        let decomposition = fields[5].trim();
        if !decomposition.is_empty() {
            let (compatibility, rest) = match decomposition.strip_prefix('<') {
                Some(tagged) => (
                    true,
                    tagged.split_once('>').map_or("", |(_, rest)| rest).trim(),
                ),
                None => (false, decomposition),
            };
            data.mappings.insert(
                point,
                Mapping {
                    compatibility,
                    chars: rest.split_whitespace().map(hex).collect(),
                },
            );
        }
    }
    data
}

/// The full decomposition of `point`: mappings applied recursively, only the
/// canonical ones unless `compatibility`, and Hangul syllables by the §3.12
/// arithmetic.
fn decompose(data: &UnicodeData, point: u32, compatibility: bool, out: &mut Vec<u32>) {
    if (S_BASE..S_BASE + S_COUNT).contains(&point) {
        let index = point - S_BASE;
        out.push(L_BASE + index / N_COUNT);
        out.push(V_BASE + (index % N_COUNT) / T_COUNT);
        if !index.is_multiple_of(T_COUNT) {
            out.push(T_BASE + index % T_COUNT);
        }
        return;
    }
    match data.mappings.get(&point) {
        Some(mapping) if compatibility || !mapping.compatibility => {
            for &part in &mapping.chars {
                decompose(data, part, compatibility, out);
            }
        }
        _ => out.push(point),
    }
}

/// A two-stage table over the whole code space.
struct TwoStage<T> {
    index: Vec<u16>,
    blocks: Vec<T>,
}

fn two_stage<T: Copy + Eq + std::hash::Hash>(values: &[T]) -> TwoStage<T> {
    let block_len = 1usize << BLOCK_SHIFT;
    let mut seen: HashMap<&[T], u16> = HashMap::new();
    let mut index = Vec::new();
    let mut blocks = Vec::new();
    for block in values.chunks(block_len) {
        let next = u16::try_from(seen.len()).expect("fewer than 65536 distinct blocks");
        let number = *seen.entry(block).or_insert_with(|| {
            blocks.extend_from_slice(block);
            next
        });
        index.push(number);
    }
    TwoStage { index, blocks }
}

/// Append `run` to `chars`, returning its `(start, len)`.
fn push_run(chars: &mut Vec<u32>, run: &[u32]) -> (u16, u8) {
    let start = u16::try_from(chars.len()).expect("mapping data under 64 Ki chars");
    chars.extend_from_slice(run);
    (
        start,
        u8::try_from(run.len()).expect("a mapping under 256 chars"),
    )
}

fn char_literal(point: u32) -> String {
    assert!(
        char::from_u32(point).is_some(),
        "{point:04X} is not a scalar value"
    );
    format!("'\\u{{{point:04X}}}'")
}

fn emit_list<I: IntoIterator<Item = String>>(out: &mut String, items: I) {
    for (n, item) in items.into_iter().enumerate() {
        if n % 16 == 0 {
            out.push_str("\n    ");
        } else {
            out.push(' ');
        }
        out.push_str(&item);
        out.push(',');
    }
    out.push('\n');
}

fn emit_two_stage<T: Copy + std::fmt::Display>(
    out: &mut String,
    name: &str,
    doc: &str,
    ty: &str,
    table: &TwoStage<T>,
) {
    let _ = writeln!(out, "/// {doc}: block numbers, one per 128 code points.");
    let _ = write!(
        out,
        "pub(crate) static {name}_INDEX: [u16; {}] = [",
        table.index.len()
    );
    emit_list(out, table.index.iter().map(u16::to_string));
    out.push_str("];\n\n");
    let _ = writeln!(out, "/// {doc}: the deduplicated 128-entry blocks.");
    let _ = write!(
        out,
        "pub(crate) static {name}_BLOCKS: [{ty}; {}] = [",
        table.blocks.len()
    );
    emit_list(out, table.blocks.iter().map(ToString::to_string));
    out.push_str("];\n\n");
}

fn main() {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../iri/unicode/17.0.0");

    let unicode_data = read(&dir, "UnicodeData.txt");
    let exclusions_text = read(&dir, "CompositionExclusions.txt");
    let core_properties = read(&dir, "DerivedCoreProperties.txt");
    let case_folding = read(&dir, "CaseFolding.txt");
    let word_break = read(&dir, "WordBreakProperty.txt");
    let emoji = read(&dir, "emoji-data.txt");

    // `UnicodeData.txt` carries no header; the files beside it name their
    // release, and must all name the same one.
    let version = header_version(&case_folding, "CaseFolding.txt");
    for (text, name) in [
        (&exclusions_text, "CompositionExclusions.txt"),
        (&core_properties, "DerivedCoreProperties.txt"),
        (&word_break, "WordBreakProperty.txt"),
    ] {
        assert_eq!(
            header_version(text, name),
            version,
            "{name} is another release"
        );
    }
    let emoji_version = format!("# Version: {}.{}", version.0, version.1);
    assert!(
        emoji.lines().any(|line| line.trim() == emoji_version),
        "emoji-data.txt does not declare {emoji_version:?}"
    );

    let data = parse_unicode_data(&unicode_data);
    let class = |point: u32| data.combining_class.get(&point).copied().unwrap_or(0);

    // Combining classes.
    let classes: Vec<u8> = (0..CODE_SPACE).map(class).collect();
    let class_table = two_stage(&classes);

    // Decompositions, fully applied. Record 0 is "none".
    let mut decomposition_chars: Vec<u32> = Vec::new();
    let mut decomposition_records: Vec<(u16, u8, u16, u8)> = vec![(0, 0, 0, 0)];
    let mut decomposition_of = vec![0u16; CODE_SPACE as usize];
    for &point in data.mappings.keys() {
        let mut canonical = Vec::new();
        decompose(&data, point, false, &mut canonical);
        let mut compatibility = Vec::new();
        decompose(&data, point, true, &mut compatibility);
        // A length of zero means "the character itself".
        let (canonical_start, canonical_len) = if canonical == [point] {
            (0, 0)
        } else {
            push_run(&mut decomposition_chars, &canonical)
        };
        let (compat_start, compat_len) = if compatibility == canonical {
            (canonical_start, canonical_len)
        } else {
            push_run(&mut decomposition_chars, &compatibility)
        };
        decomposition_of[point as usize] =
            u16::try_from(decomposition_records.len()).expect("under 64 Ki records");
        decomposition_records.push((canonical_start, canonical_len, compat_start, compat_len));
    }
    let decomposition_table = two_stage(&decomposition_of);

    // Primary composites (UAX 15 §5): every canonical pair mapping whose
    // source is not Full_Composition_Exclusion — not listed in
    // CompositionExclusions.txt, not a non-starter, and not decomposing to a
    // non-starter first. Singletons have one-character mappings and never
    // pair.
    let excluded: BTreeSet<u32> = data_lines(&exclusions_text)
        .flat_map(|fields| {
            let (low, high) = range(fields[0]);
            low..=high
        })
        .collect();
    let mut compositions: Vec<(u32, u32, u32)> = data
        .mappings
        .iter()
        .filter(|(point, mapping)| {
            !mapping.compatibility
                && mapping.chars.len() == 2
                && !excluded.contains(point)
                && class(**point) == 0
                && class(mapping.chars[0]) == 0
        })
        .map(|(&point, mapping)| (mapping.chars[0], mapping.chars[1], point))
        .collect();
    compositions.sort_unstable();
    for &(first, second, composite) in &compositions {
        // The ASCII fast path flushes the pipeline before an ASCII character,
        // which is exact only if no ASCII character composes with what
        // precedes it.
        assert!(
            second >= 0x80,
            "{composite:04X} composes {first:04X} with ASCII {second:04X}"
        );
    }
    // Starters that are the second character of a primary composite: the
    // composer consults the pair table for a starter only if it is one.
    let composing_starters: BTreeSet<u32> = compositions
        .iter()
        .map(|&(_, second, _)| second)
        .filter(|&second| class(second) == 0)
        .collect();
    // The composer skips the search below U+0300 without looking.
    assert!(
        composing_starters.iter().all(|&second| second >= 0x300),
        "a composing starter below U+0300"
    );

    // Full case folding: statuses C and F.
    let mut fold_chars: Vec<u32> = Vec::new();
    let mut fold_records: Vec<(u16, u8)> = vec![(0, 0)];
    let mut fold_of = vec![0u16; CODE_SPACE as usize];
    for fields in data_lines(&case_folding) {
        if fields[1] != "C" && fields[1] != "F" {
            continue;
        }
        let point = hex(fields[0]);
        assert_eq!(fold_of[point as usize], 0, "{point:04X} folds twice");
        let mapping: Vec<u32> = fields[2].split_whitespace().map(hex).collect();
        let run = push_run(&mut fold_chars, &mapping);
        fold_of[point as usize] = u16::try_from(fold_records.len()).expect("under 64 Ki records");
        fold_records.push(run);
    }
    let fold_table = two_stage(&fold_of);

    // Segmentation byte: Word_Break, Extended_Pictographic, alphanumeric.
    let mut segmentation = vec![0u8; CODE_SPACE as usize];
    for fields in data_lines(&word_break) {
        let value = WORD_BREAK_VALUES
            .iter()
            .position(|name| *name == fields[1])
            .unwrap_or_else(|| panic!("unknown Word_Break value {:?}", fields[1]));
        let (low, high) = range(fields[0]);
        for point in low..=high {
            assert_eq!(
                segmentation[point as usize], 0,
                "{point:04X} has two Word_Break values"
            );
            segmentation[point as usize] = u8::try_from(value).expect("few Word_Break values");
        }
    }
    for fields in data_lines(&emoji) {
        if fields[1] == "Extended_Pictographic" {
            let (low, high) = range(fields[0]);
            for point in low..=high {
                segmentation[point as usize] |= EXTENDED_PICTOGRAPHIC;
            }
        }
    }
    for fields in data_lines(&core_properties) {
        if fields[1] == "Alphabetic" {
            let (low, high) = range(fields[0]);
            for point in low..=high {
                segmentation[point as usize] |= ALPHANUMERIC;
            }
        }
    }
    for (&point, category) in &data.general_category {
        if matches!(category.as_str(), "Nd" | "Nl" | "No") {
            segmentation[point as usize] |= ALPHANUMERIC;
        }
    }
    let segmentation_table = two_stage(&segmentation);

    // Emit.
    let (major, minor, patch) = version;
    let mut out = String::new();
    out.push_str(
        "// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>\n\
         // SPDX-FileCopyrightText: Unicode, Inc. <https://www.unicode.org>\n\
         // SPDX-License-Identifier: (MIT OR Apache-2.0 OR MulanPSL-2.0) AND Unicode-3.0\n\n",
    );
    let _ = write!(
        out,
        "//! @generated by `cargo run -p purrdf-text --example gen_unicode_text_tables --locked`.\n\
         //! Do not hand-edit; `scripts/check-generated.sh` diffs this file against the\n\
         //! generator's stdout.\n\
         //!\n\
         //! Source: the Unicode Character Database {major}.{minor}.{patch} vendored under\n\
         //! `crates/iri/unicode/{major}.{minor}.{patch}/` — `UnicodeData.txt`,\n\
         //! `CompositionExclusions.txt`, `DerivedCoreProperties.txt`, `CaseFolding.txt`,\n\
         //! `WordBreakProperty.txt` and `emoji-data.txt`. The data is Unicode's, under\n\
         //! the Unicode-3.0 licence.\n\n\
         /// The Unicode version every table here is generated from.\n\
         pub(crate) const UNICODE_VERSION: (u8, u8, u8) = ({major}, {minor}, {patch});\n\n\
         /// The version of the `CaseFolding.txt` the fold tables come from.\n\
         pub(crate) const FOLD_UNICODE_VERSION: (u8, u8, u8) = ({major}, {minor}, {patch});\n\n\
         /// Code points per second-stage block, as a shift.\n\
         pub(crate) const BLOCK_SHIFT: u32 = {BLOCK_SHIFT};\n\n"
    );
    // Value 0 is `Other`, which no rule names.
    for (value, name) in WORD_BREAK_VALUES.iter().enumerate().skip(1) {
        let constant = name.to_uppercase();
        let _ = writeln!(
            out,
            "/// `Word_Break={name}`.\npub(crate) const WB_{constant}: u8 = {value};"
        );
    }
    let _ = write!(
        out,
        "\n/// Mask of the `Word_Break` value in a segmentation byte (0: `Other`).\n\
         pub(crate) const WB_MASK: u8 = 0x1F;\n\
         /// Segmentation-byte bit: `Extended_Pictographic`.\n\
         pub(crate) const EXTENDED_PICTOGRAPHIC: u8 = {EXTENDED_PICTOGRAPHIC:#04X};\n\
         /// Segmentation-byte bit: `Alphabetic`, or General_Category `Nd`, `Nl` or `No`.\n\
         pub(crate) const ALPHANUMERIC: u8 = {ALPHANUMERIC:#04X};\n\n"
    );

    emit_two_stage(
        &mut out,
        "COMBINING_CLASS",
        "Canonical_Combining_Class",
        "u8",
        &class_table,
    );
    emit_two_stage(
        &mut out,
        "DECOMPOSITION",
        "Decomposition record numbers (0: none)",
        "u16",
        &decomposition_table,
    );
    out.push_str(
        "/// Decomposition records: canonical `(start, len)` then compatibility\n\
         /// `(start, len)` into [`DECOMPOSITION_CHARS`]; a length of 0 means the\n\
         /// character is its own decomposition of that kind.\n",
    );
    let _ = write!(
        out,
        "pub(crate) static DECOMPOSITION_RECORDS: [(u16, u8, u16, u8); {}] = [",
        decomposition_records.len()
    );
    emit_list(
        &mut out,
        decomposition_records
            .iter()
            .map(|(a, b, c, d)| format!("({a}, {b}, {c}, {d})")),
    );
    out.push_str("];\n\n/// The fully applied decompositions, concatenated.\n");
    let _ = write!(
        out,
        "pub(crate) static DECOMPOSITION_CHARS: [char; {}] = [",
        decomposition_chars.len()
    );
    emit_list(
        &mut out,
        decomposition_chars.iter().map(|&p| char_literal(p)),
    );
    out.push_str("];\n\n");

    out.push_str(
        "/// The starters (combining class 0) that are the second character of a\n\
         /// primary composite, sorted. Hangul jamo are composed by arithmetic and\n\
         /// are not listed.\n",
    );
    let _ = write!(
        out,
        "pub(crate) static COMPOSING_STARTERS: [char; {}] = [",
        composing_starters.len()
    );
    emit_list(
        &mut out,
        composing_starters.iter().map(|&p| char_literal(p)),
    );
    out.push_str("];\n\n");

    out.push_str(
        "/// Primary composites, sorted by `(first, second)`: the canonical pair\n\
         /// mappings whose source is not `Full_Composition_Exclusion`.\n",
    );
    let _ = write!(
        out,
        "pub(crate) static COMPOSITIONS: [(char, char, char); {}] = [",
        compositions.len()
    );
    emit_list(
        &mut out,
        compositions.iter().map(|&(a, b, c)| {
            format!(
                "({}, {}, {})",
                char_literal(a),
                char_literal(b),
                char_literal(c)
            )
        }),
    );
    out.push_str("];\n\n");

    emit_two_stage(
        &mut out,
        "FOLD",
        "Case fold record numbers (0: folds to itself)",
        "u16",
        &fold_table,
    );
    out.push_str("/// Case fold records: `(start, len)` into [`FOLD_CHARS`].\n");
    let _ = write!(
        out,
        "pub(crate) static FOLD_RECORDS: [(u16, u8); {}] = [",
        fold_records.len()
    );
    emit_list(
        &mut out,
        fold_records.iter().map(|(a, b)| format!("({a}, {b})")),
    );
    out.push_str("];\n\n/// The `C` and `F` fold mappings, concatenated.\n");
    let _ = write!(
        out,
        "pub(crate) static FOLD_CHARS: [char; {}] = [",
        fold_chars.len()
    );
    emit_list(&mut out, fold_chars.iter().map(|&p| char_literal(p)));
    out.push_str("];\n\n");

    emit_two_stage(
        &mut out,
        "SEGMENTATION",
        "Segmentation bytes (`Word_Break` | flags)",
        "u8",
        &segmentation_table,
    );

    print!("{out}");
}

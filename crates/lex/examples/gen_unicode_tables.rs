// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Regenerates every Unicode table in the workspace from the one Unicode
//! Character Database vendored under `crates/iri/unicode/`, at one version,
//! [`UNICODE_VERSION`].
//!
//! ```text
//! cargo run -p purrdf-lex --example gen_unicode_tables --locked -- <table set>
//! ```
//!
//! | Table set | Committed file | What it holds |
//! |---|---|---|
//! | `normalization` | `crates/lex/src/unicode_tables.rs` | `Canonical_Combining_Class`, the full decompositions, the primary composites (UAX 15) |
//! | `text` | `crates/text/src/unicode_tables.rs` | full case folding (`C` + `F`), the UAX 29 segmentation byte |
//! | `idna` | `crates/iri/src/idna_tables.rs` | the RFC 5892 derived property, Joining_Type, Bidi_Class, the Appendix A scripts, the combining marks, NFKC_Casefold |
//! | `ecma-properties` | `crates/jsonschema/src/ecma/property_tables.rs` | the property names and values an ECMA-262 `\p{…}` escape may spell |
//! | `ecma-ranges` | `crates/jsonschema/src/ecma/unicode_ranges.rs` | the code point ranges of each of those properties, and simple case folding |
//!
//! Every UCD file read names its release in its header, and the generator
//! refuses to run unless each names [`UNICODE_VERSION`]. The `normalization`
//! set carries that version as `purrdf_lex::unicode::UNICODE_VERSION`; every
//! other set asserts at compile time that it is the version it was generated
//! from, so no table in the workspace can trail another. (The XSD block-escape
//! table of `purrdf-core` is the one deliberate exception: it is pinned to the
//! Unicode version of the locked `regex-syntax`, and
//! `scripts/check-generated.sh` holds that pin.)
//!
//! File formats per UAX 44. Output goes to stdout;
//! `scripts/check-generated.sh` pipes it through `rustfmt` and compares it
//! with the committed file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The Unicode version every table is generated from.
const UNICODE_VERSION: (u8, u8, u8) = (17, 0, 0);

/// One past the last code point.
const CODE_SPACE: u32 = 0x11_0000;
/// The highest code point.
const MAX_CODE_POINT: u32 = 0x10_FFFF;

/// Code points per second-stage block of a two-stage table, as a shift.
const BLOCK_SHIFT: u32 = 7;

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

/// The licence header every generated file opens with.
const SPDX: &str = "// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>\n\
                    // SPDX-FileCopyrightText: Unicode, Inc. <https://www.unicode.org>\n\
                    // SPDX-License-Identifier: (MIT OR Apache-2.0 OR MulanPSL-2.0) AND Unicode-3.0\n";

// ---------------------------------------------------------------------------
// The database.

/// The vendored database directory.
fn ucd_dir() -> PathBuf {
    let (major, minor, patch) = UNICODE_VERSION;
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../iri/unicode/{major}.{minor}.{patch}"))
}

fn read(name: &str) -> String {
    let path = ucd_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

fn hex(text: &str) -> u32 {
    u32::from_str_radix(text.trim(), 16).unwrap_or_else(|err| panic!("bad hex {text:?}: {err}"))
}

fn hex_list(field: &str) -> Vec<u32> {
    field.split_whitespace().map(hex).collect()
}

/// A `XXXX` or `XXXX..YYYY` code point field as an inclusive range.
fn range(field: &str) -> (u32, u32) {
    match field.trim().split_once("..") {
        Some((low, high)) => (hex(low), hex(high)),
        None => {
            let point = hex(field);
            (point, point)
        }
    }
}

/// The data fields of a UCD file line, comment stripped and fields trimmed;
/// `None` for a blank or comment line.
fn fields(line: &str) -> Option<Vec<&str>> {
    let data = line.split('#').next().unwrap_or("").trim();
    (!data.is_empty()).then(|| data.split(';').map(str::trim).collect())
}

/// The data lines of a UCD file.
fn data_lines(text: &str) -> impl Iterator<Item = Vec<&str>> {
    text.lines().filter_map(fields)
}

/// The version a UCD file names in its first line, `# Name-X.Y.Z.txt`.
fn header_version(name: &str) -> (u8, u8, u8) {
    let text = read(name);
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

/// Refuse to generate from files of another release than [`UNICODE_VERSION`].
/// `UnicodeData.txt` carries no header; the files beside it name their
/// release, and `emoji-data.txt` names its major and minor version.
fn assert_one_version() {
    for name in [
        "Blocks.txt",
        "CaseFolding.txt",
        "CompositionExclusions.txt",
        "DerivedBidiClass.txt",
        "DerivedCoreProperties.txt",
        "DerivedJoiningType.txt",
        "DerivedNormalizationProps.txt",
        "HangulSyllableType.txt",
        "PropList.txt",
        "PropertyAliases.txt",
        "PropertyValueAliases.txt",
        "ScriptExtensions.txt",
        "Scripts.txt",
        "WordBreakProperty.txt",
    ] {
        assert_eq!(
            header_version(name),
            UNICODE_VERSION,
            "{name} is another release"
        );
    }
    let emoji_version = format!("# Version: {}.{}", UNICODE_VERSION.0, UNICODE_VERSION.1);
    assert!(
        read("emoji-data.txt")
            .lines()
            .any(|line| line.trim() == emoji_version),
        "emoji-data.txt does not declare {emoji_version:?}"
    );
}

/// Everything read from `UnicodeData.txt` for one code point.
#[derive(Clone, Default)]
struct Entry {
    general_category: String,
    ccc: u8,
    /// The decomposition mapping (one level, as written), and whether it is a
    /// compatibility (`<tag>`) mapping.
    decomposition: Option<(bool, Vec<u32>)>,
}

/// Every assigned code point's entry, with the `First`/`Last` ranges
/// expanded.
fn unicode_data() -> BTreeMap<u32, Entry> {
    let text = read("UnicodeData.txt");
    let mut entries = BTreeMap::new();
    let mut range_start: Option<(u32, Entry)> = None;
    for line in text.lines() {
        let f: Vec<&str> = line.split(';').collect();
        assert_eq!(
            f.len(),
            15,
            "UnicodeData.txt line has {} fields: {line:?}",
            f.len()
        );
        let point = hex(f[0]);
        let decomposition = if f[5].is_empty() {
            None
        } else if let Some(rest) = f[5].strip_prefix('<') {
            let (_, list) = rest.split_once('>').expect("closed decomposition tag");
            Some((true, hex_list(list)))
        } else {
            Some((false, hex_list(f[5])))
        };
        let entry = Entry {
            general_category: f[2].to_owned(),
            ccc: f[3].parse().expect("numeric combining class"),
            decomposition,
        };
        if f[1].ends_with(", First>") {
            range_start = Some((point, entry));
        } else if f[1].ends_with(", Last>") {
            let (start, first) = range_start.take().expect("range Last without First");
            for member in start..=point {
                entries.insert(member, first.clone());
            }
        } else {
            entries.insert(point, entry);
        }
    }
    assert!(range_start.is_none(), "range First without Last");
    entries
}

fn general_category(entries: &BTreeMap<u32, Entry>, point: u32) -> &str {
    entries
        .get(&point)
        .map_or("Cn", |entry| entry.general_category.as_str())
}

fn combining_class(entries: &BTreeMap<u32, Entry>, point: u32) -> u8 {
    entries.get(&point).map_or(0, |entry| entry.ccc)
}

/// The full decomposition of `point`: mappings applied recursively, only the
/// canonical ones unless `compatibility`, and Hangul syllables by the §3.12
/// arithmetic.
fn decompose(entries: &BTreeMap<u32, Entry>, point: u32, compatibility: bool, out: &mut Vec<u32>) {
    if (S_BASE..S_BASE + S_COUNT).contains(&point) {
        let index = point - S_BASE;
        out.push(L_BASE + index / N_COUNT);
        out.push(V_BASE + (index % N_COUNT) / T_COUNT);
        if !index.is_multiple_of(T_COUNT) {
            out.push(T_BASE + index % T_COUNT);
        }
        return;
    }
    match entries
        .get(&point)
        .and_then(|entry| entry.decomposition.as_ref())
    {
        Some((is_compatibility, list)) if compatibility || !is_compatibility => {
            for &part in list {
                decompose(entries, part, compatibility, out);
            }
        }
        _ => out.push(point),
    }
}

/// Every code point with `property` in a binary-property file.
fn binary_property(text: &str, property: &str) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for f in data_lines(text) {
        if f.len() == 2 && f[1] == property {
            let (low, high) = range(f[0]);
            set.extend(low..=high);
        }
    }
    set
}

// ---------------------------------------------------------------------------
// Emission helpers.

/// A two-stage table over the whole code space.
struct TwoStage<T> {
    index: Vec<u16>,
    blocks: Vec<T>,
}

fn two_stage<T: Copy + Ord>(values: &[T]) -> TwoStage<T> {
    let block_len = 1usize << BLOCK_SHIFT;
    let mut seen: BTreeMap<&[T], u16> = BTreeMap::new();
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

fn char_slice(list: &[u32]) -> String {
    let parts: Vec<String> = list.iter().map(|&point| char_literal(point)).collect();
    format!("&[{}]", parts.join(", "))
}

/// A code point as a separated eight-digit hex literal, `0x0001_F600`.
fn hex_literal(point: u32) -> String {
    format!("0x{:04X}_{:04X}", point >> 16, point & 0xFFFF)
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

/// The file header: licence, `@generated` line naming `set`, and the module
/// documentation lines `doc`.
fn emit_header(out: &mut String, set: &str, doc: &[&str]) {
    out.push_str(SPDX);
    out.push('\n');
    let _ = writeln!(
        out,
        "//! @generated by `cargo run -p purrdf-lex --example gen_unicode_tables --locked -- {set}`.\n\
         //! Do not hand-edit; `scripts/check-generated.sh` diffs this file against the\n\
         //! generator's stdout.\n\
         //!"
    );
    for line in doc {
        if line.is_empty() {
            out.push_str("//!\n");
        } else {
            let _ = writeln!(out, "//! {line}");
        }
    }
    out.push('\n');
}

/// The compile-time assertion a consumer's table file makes: it was generated
/// from the Unicode version of the workspace's normalization tables.
fn emit_version_assertion(out: &mut String) {
    let (major, minor, patch) = UNICODE_VERSION;
    let _ = write!(
        out,
        "// Generated from Unicode {major}.{minor}.{patch}, the version of every Unicode table in the\n\
         // workspace: `purrdf_lex::unicode::UNICODE_VERSION`.\n\
         const _: () = assert!(\n    \
             matches!(purrdf_lex::unicode::UNICODE_VERSION, ({major}, {minor}, {patch})),\n    \
             \"generated from another Unicode version than purrdf_lex::unicode::UNICODE_VERSION\"\n\
         );\n\n"
    );
}

/// The source line of a table file's module documentation.
fn source_line(files: &str) -> String {
    let (major, minor, patch) = UNICODE_VERSION;
    format!(
        "Source: the Unicode Character Database {major}.{minor}.{patch} vendored under \
         `crates/iri/unicode/{major}.{minor}.{patch}/` — {files}. The data is Unicode's, under the \
         Unicode-3.0 licence."
    )
}

/// `text` wrapped to lines of at most `width` characters, for a doc comment.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

// ---------------------------------------------------------------------------
// `normalization`: purrdf_lex::unicode.

fn normalization() -> String {
    let entries = unicode_data();
    let class = |point: u32| combining_class(&entries, point);

    // Combining classes.
    let classes: Vec<u8> = (0..CODE_SPACE).map(class).collect();
    let class_table = two_stage(&classes);

    // Decompositions, fully applied. Record 0 is "none".
    let mut decomposition_chars: Vec<u32> = Vec::new();
    let mut decomposition_records: Vec<(u16, u8, u16, u8)> = vec![(0, 0, 0, 0)];
    let mut decomposition_of = vec![0u16; CODE_SPACE as usize];
    for (&point, _) in entries
        .iter()
        .filter(|(_, entry)| entry.decomposition.is_some())
    {
        let mut canonical = Vec::new();
        decompose(&entries, point, false, &mut canonical);
        let mut compatibility = Vec::new();
        decompose(&entries, point, true, &mut compatibility);
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
    let excluded: BTreeSet<u32> = data_lines(&read("CompositionExclusions.txt"))
        .flat_map(|fields| {
            let (low, high) = range(fields[0]);
            low..=high
        })
        .collect();
    let mut compositions: Vec<(u32, u32, u32)> = entries
        .iter()
        .filter_map(|(&point, entry)| match &entry.decomposition {
            Some((false, chars))
                if chars.len() == 2
                    && !excluded.contains(&point)
                    && class(point) == 0
                    && class(chars[0]) == 0 =>
            {
                Some((chars[0], chars[1], point))
            }
            _ => None,
        })
        .collect();
    compositions.sort_unstable();
    // The same set, derived the other way: `DerivedNormalizationProps.txt`'s
    // Full_Composition_Exclusion.
    let full_exclusion = binary_property(
        &read("DerivedNormalizationProps.txt"),
        "Full_Composition_Exclusion",
    );
    let derived: Vec<(u32, u32, u32)> = entries
        .iter()
        .filter_map(|(&point, entry)| match &entry.decomposition {
            Some((false, chars)) if chars.len() == 2 && !full_exclusion.contains(&point) => {
                Some((chars[0], chars[1], point))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(
        compositions, derived,
        "CompositionExclusions.txt and Full_Composition_Exclusion disagree"
    );
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
    // `ccc` answers 0 below U+0300 without looking.
    assert!(
        (0..0x300).all(|point| class(point) == 0),
        "a non-starter below U+0300"
    );

    let (major, minor, patch) = UNICODE_VERSION;
    let mut out = String::new();
    let source = source_line(
        "`UnicodeData.txt`, `CompositionExclusions.txt` and `DerivedNormalizationProps.txt`",
    );
    let mut doc: Vec<String> = wrap(&source, 76);
    doc.insert(0, String::new());
    doc.insert(
        0,
        "The normalization data of `purrdf_lex::unicode` (UAX 15, core specification §3.11)."
            .to_owned(),
    );
    emit_header(
        &mut out,
        "normalization",
        &doc.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let _ = write!(
        out,
        "/// The Unicode version every Unicode table in the workspace is generated from.\n\
         pub(crate) const UNICODE_VERSION: (u8, u8, u8) = ({major}, {minor}, {patch});\n\n\
         /// Code points per second-stage block, as a shift.\n\
         pub(crate) const BLOCK_SHIFT: u32 = {BLOCK_SHIFT};\n\n"
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
    out.push_str("];\n");
    out
}

// ---------------------------------------------------------------------------
// `text`: purrdf_text::unicode.

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

fn text() -> String {
    let entries = unicode_data();

    // Full case folding: statuses C and F.
    let mut fold_chars: Vec<u32> = Vec::new();
    let mut fold_records: Vec<(u16, u8)> = vec![(0, 0)];
    let mut fold_of = vec![0u16; CODE_SPACE as usize];
    for fields in data_lines(&read("CaseFolding.txt")) {
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
    for fields in data_lines(&read("WordBreakProperty.txt")) {
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
    for fields in data_lines(&read("emoji-data.txt")) {
        if fields[1] == "Extended_Pictographic" {
            let (low, high) = range(fields[0]);
            for point in low..=high {
                segmentation[point as usize] |= EXTENDED_PICTOGRAPHIC;
            }
        }
    }
    for fields in data_lines(&read("DerivedCoreProperties.txt")) {
        if fields[1] == "Alphabetic" {
            let (low, high) = range(fields[0]);
            for point in low..=high {
                segmentation[point as usize] |= ALPHANUMERIC;
            }
        }
    }
    for (&point, entry) in &entries {
        if matches!(entry.general_category.as_str(), "Nd" | "Nl" | "No") {
            segmentation[point as usize] |= ALPHANUMERIC;
        }
    }
    let segmentation_table = two_stage(&segmentation);

    let mut out = String::new();
    let source = source_line(
        "`UnicodeData.txt`, `DerivedCoreProperties.txt`, `CaseFolding.txt`, \
         `WordBreakProperty.txt` and `emoji-data.txt`",
    );
    let mut doc = vec![
        "The full case folding and word-boundary data of `purrdf_text::unicode`,".to_owned(),
        "read through `purrdf_lex::unicode::lookup_two_stage`.".to_owned(),
        String::new(),
    ];
    doc.extend(wrap(&source, 76));
    emit_header(
        &mut out,
        "text",
        &doc.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    emit_version_assertion(&mut out);
    let _ = write!(
        out,
        "// The two-stage tables below are read with `purrdf_lex::unicode::lookup_two_stage`.\n\
         const _: () = assert!(purrdf_lex::unicode::BLOCK_SHIFT == {BLOCK_SHIFT});\n\n"
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
    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// `idna`: purrdf_iri::idna.

/// The parsed character database the IDNA tables are derived over.
struct Idna {
    entries: BTreeMap<u32, Entry>,
    default_ignorable: BTreeSet<u32>,
    white_space: BTreeSet<u32>,
    noncharacter: BTreeSet<u32>,
    join_control: BTreeSet<u32>,
    /// Hangul_Syllable_Type L, V or T.
    old_hangul_jamo: BTreeSet<u32>,
    case_fold: BTreeMap<u32, Vec<u32>>,
    changes_when_nfkc_casefolded: BTreeSet<u32>,
    nfkc_casefold: BTreeMap<u32, Vec<u32>>,
    joining: Vec<&'static str>,
    bidi: Vec<&'static str>,
    script: Vec<&'static str>,
    ignorable_blocks: Vec<(u32, u32)>,
    /// Two-code-point canonical decompositions whose composite is not
    /// Full_Composition_Exclusion: `(first, second) -> composite`.
    pairs: BTreeMap<(u32, u32), u32>,
}

fn parse_case_folding(text: &str) -> BTreeMap<u32, Vec<u32>> {
    let mut map = BTreeMap::new();
    for f in data_lines(text) {
        if f[1] == "C" || f[1] == "F" {
            map.insert(hex(f[0]), hex_list(f[2]));
        }
    }
    map
}

fn parse_nfkc_casefold(text: &str) -> BTreeMap<u32, Vec<u32>> {
    let mut map = BTreeMap::new();
    for f in data_lines(text) {
        if f.len() >= 3 && f[1] == "NFKC_CF" {
            let (low, high) = range(f[0]);
            let image = hex_list(f[2]);
            for point in low..=high {
                map.insert(point, image.clone());
            }
        }
    }
    map
}

/// A total enumerated-property table: `@missing` defaults in file order, then
/// the explicit lines. `canon` maps both long and short value names onto the
/// generator's own short spelling.
fn enumerated_property(text: &str, canon: fn(&str) -> &'static str) -> Vec<&'static str> {
    let mut table = vec![""; CODE_SPACE as usize];
    for line in text.lines() {
        if let Some(missing) = line.strip_prefix("# @missing:") {
            let f: Vec<&str> = missing.split(';').map(str::trim).collect();
            let (low, high) = range(f[0]);
            let value = canon(f[1]);
            for point in low..=high {
                table[point as usize] = value;
            }
        }
    }
    for f in data_lines(text) {
        let (low, high) = range(f[0]);
        let value = canon(f[1]);
        for point in low..=high {
            table[point as usize] = value;
        }
    }
    assert!(
        table.iter().all(|v| !v.is_empty()),
        "an enumerated property left a code point without a value"
    );
    table
}

fn canon_joining(value: &str) -> &'static str {
    match value {
        "U" | "Non_Joining" => "NonJoining",
        "T" | "Transparent" => "Transparent",
        "L" | "Left_Joining" => "Left",
        "R" | "Right_Joining" => "Right",
        "D" | "Dual_Joining" => "Dual",
        "C" | "Join_Causing" => "JoinCausing",
        other => panic!("unknown Joining_Type {other:?}"),
    }
}

/// Bidi_Class, collapsed to the classes RFC 5893 names; every other class
/// (B, S, WS and the explicit embedding/override/isolate controls) is one
/// `Other`, which neither label direction admits.
fn canon_bidi(value: &str) -> &'static str {
    match value {
        "L" | "Left_To_Right" => "Left",
        "R" | "Right_To_Left" => "Right",
        "AL" | "Arabic_Letter" => "ArabicLetter",
        "EN" | "European_Number" => "EuropeanNumber",
        "ES" | "European_Separator" => "EuropeanSeparator",
        "ET" | "European_Terminator" => "EuropeanTerminator",
        "AN" | "Arabic_Number" => "ArabicNumber",
        "CS" | "Common_Separator" => "CommonSeparator",
        "NSM" | "Nonspacing_Mark" => "NonspacingMark",
        "BN" | "Boundary_Neutral" => "BoundaryNeutral",
        "ON" | "Other_Neutral" => "OtherNeutral",
        "B"
        | "Paragraph_Separator"
        | "S"
        | "Segment_Separator"
        | "WS"
        | "White_Space"
        | "LRE"
        | "Left_To_Right_Embedding"
        | "LRO"
        | "Left_To_Right_Override"
        | "RLE"
        | "Right_To_Left_Embedding"
        | "RLO"
        | "Right_To_Left_Override"
        | "PDF"
        | "Pop_Directional_Format"
        | "LRI"
        | "Left_To_Right_Isolate"
        | "RLI"
        | "Right_To_Left_Isolate"
        | "FSI"
        | "First_Strong_Isolate"
        | "PDI"
        | "Pop_Directional_Isolate" => "Other",
        other => panic!("unknown Bidi_Class {other:?}"),
    }
}

/// Script, reduced to the five RFC 5892 Appendix A consults.
fn canon_script(value: &str) -> &'static str {
    match value {
        "Greek" => "Greek",
        "Hebrew" => "Hebrew",
        "Hiragana" => "Hiragana",
        "Katakana" => "Katakana",
        "Han" => "Han",
        _ => "Other",
    }
}

/// The named blocks' ranges, read from a `Blocks.txt`.
fn named_blocks(text: &str, names: &[&str]) -> Vec<(u32, u32)> {
    let mut found = Vec::new();
    for name in names {
        let hit = data_lines(text).find_map(|f| (f[1] == *name).then(|| range(f[0])));
        found.push(hit.unwrap_or_else(|| panic!("Blocks.txt has no block named {name:?}")));
    }
    found
}

impl Idna {
    fn load() -> Self {
        let prop_list = read("PropList.txt");
        let hangul = read("HangulSyllableType.txt");
        let dnp = read("DerivedNormalizationProps.txt");
        let entries = unicode_data();
        let full_composition_exclusion = binary_property(&dnp, "Full_Composition_Exclusion");
        let mut pairs = BTreeMap::new();
        for (&point, entry) in &entries {
            if let Some((false, list)) = &entry.decomposition
                && list.len() == 2
                && !full_composition_exclusion.contains(&point)
            {
                assert!(
                    pairs.insert((list[0], list[1]), point).is_none(),
                    "two primary composites share a decomposition"
                );
            }
        }
        Self {
            entries,
            default_ignorable: binary_property(
                &read("DerivedCoreProperties.txt"),
                "Default_Ignorable_Code_Point",
            ),
            white_space: binary_property(&prop_list, "White_Space"),
            noncharacter: binary_property(&prop_list, "Noncharacter_Code_Point"),
            join_control: binary_property(&prop_list, "Join_Control"),
            old_hangul_jamo: ["L", "V", "T"]
                .iter()
                .flat_map(|value| binary_property(&hangul, value))
                .collect(),
            case_fold: parse_case_folding(&read("CaseFolding.txt")),
            changes_when_nfkc_casefolded: binary_property(&dnp, "Changes_When_NFKC_Casefolded"),
            nfkc_casefold: parse_nfkc_casefold(&dnp),
            joining: enumerated_property(&read("DerivedJoiningType.txt"), canon_joining),
            bidi: enumerated_property(&read("DerivedBidiClass.txt"), canon_bidi),
            script: enumerated_property(&read("Scripts.txt"), |value| {
                if value == "Unknown" {
                    "Other"
                } else {
                    canon_script(value)
                }
            }),
            ignorable_blocks: named_blocks(
                &read("Blocks.txt"),
                &[
                    "Combining Diacritical Marks for Symbols",
                    "Musical Symbols",
                    "Ancient Greek Musical Notation",
                ],
            ),
            pairs,
        }
    }

    fn general_category(&self, point: u32) -> &str {
        general_category(&self.entries, point)
    }

    fn ccc(&self, point: u32) -> u8 {
        combining_class(&self.entries, point)
    }

    // ---- Normalization (Unicode Standard §3.11) ------------------------------

    fn reorder(&self, text: &mut [u32]) {
        // Canonical ordering: a stable sort of each run of non-starters by
        // combining class.
        let mut start = 0;
        while start < text.len() {
            if self.ccc(text[start]) == 0 {
                start += 1;
                continue;
            }
            let mut end = start;
            while end < text.len() && self.ccc(text[end]) != 0 {
                end += 1;
            }
            text[start..end].sort_by_key(|&point| self.ccc(point));
            start = end;
        }
    }

    fn primary_composite(&self, first: u32, second: u32) -> Option<u32> {
        if (L_BASE..L_BASE + L_COUNT).contains(&first)
            && (V_BASE..V_BASE + V_COUNT).contains(&second)
        {
            return Some(S_BASE + ((first - L_BASE) * V_COUNT + (second - V_BASE)) * T_COUNT);
        }
        if (S_BASE..S_BASE + S_COUNT).contains(&first)
            && (first - S_BASE).is_multiple_of(T_COUNT)
            && (T_BASE + 1..T_BASE + T_COUNT).contains(&second)
        {
            return Some(first + (second - T_BASE));
        }
        self.pairs.get(&(first, second)).copied()
    }

    fn compose(&self, text: &[u32]) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::with_capacity(text.len());
        let mut starter: Option<usize> = None;
        let mut last_class: Option<u8> = None;
        for &point in text {
            let class = self.ccc(point);
            if let Some(at) = starter {
                let blocked = last_class.is_some_and(|last| last == 0 || last >= class);
                if !blocked && let Some(composite) = self.primary_composite(out[at], point) {
                    out[at] = composite;
                    continue;
                }
            }
            if class == 0 {
                starter = Some(out.len());
                last_class = None;
            } else {
                last_class = Some(class);
            }
            out.push(point);
        }
        out
    }

    fn normalize(&self, text: &[u32], compat: bool) -> Vec<u32> {
        let mut decomposed = Vec::new();
        for &point in text {
            decompose(&self.entries, point, compat, &mut decomposed);
        }
        self.reorder(&mut decomposed);
        self.compose(&decomposed)
    }

    fn case_fold(&self, text: &[u32]) -> Vec<u32> {
        let mut out = Vec::new();
        for &point in text {
            match self.case_fold.get(&point) {
                Some(image) => out.extend_from_slice(image),
                None => out.push(point),
            }
        }
        out
    }

    // ---- RFC 5892 §2 categories ----------------------------------------------

    /// §2.2 Unstable: `toNFKC(toCaseFold(toNFKC(cp))) != cp`.
    fn unstable(&self, point: u32) -> bool {
        let once = self.normalize(&[point], true);
        let folded = self.case_fold(&once);
        self.normalize(&folded, true) != [point]
    }

    /// §2.3 IgnorableProperties.
    fn ignorable_properties(&self, point: u32) -> bool {
        self.default_ignorable.contains(&point)
            || self.white_space.contains(&point)
            || self.noncharacter.contains(&point)
    }

    /// §2.6 Exceptions.
    fn exception(point: u32) -> Option<&'static str> {
        match point {
            0x00DF | 0x03C2 | 0x06FD | 0x06FE | 0x0F0B | 0x3007 => Some("Pvalid"),
            0x00B7 | 0x0375 | 0x05F3 | 0x05F4 | 0x30FB | 0x0660..=0x0669 | 0x06F0..=0x06F9 => {
                Some("ContextO")
            }
            0x0640 | 0x07FA | 0x302E | 0x302F | 0x3031..=0x3035 | 0x303B => Some("Disallowed"),
            _ => None,
        }
    }

    /// RFC 5892 §3: the derived property, rules applied in order.
    fn derived_property(&self, point: u32) -> &'static str {
        if let Some(value) = Self::exception(point) {
            return value;
        }
        // BackwardCompatible (G) is the empty set.
        if self.general_category(point) == "Cn" && !self.noncharacter.contains(&point) {
            return "Unassigned";
        }
        if point == 0x2D || (0x30..=0x39).contains(&point) || (0x61..=0x7A).contains(&point) {
            return "Pvalid";
        }
        if self.join_control.contains(&point) {
            return "ContextJ";
        }
        let surrogate = (0xD800..=0xDFFF).contains(&point);
        if !surrogate && self.unstable(point) {
            return "Disallowed";
        }
        if self.ignorable_properties(point) {
            return "Disallowed";
        }
        if self
            .ignorable_blocks
            .iter()
            .any(|&(low, high)| (low..=high).contains(&point))
        {
            return "Disallowed";
        }
        if self.old_hangul_jamo.contains(&point) {
            return "Disallowed";
        }
        if matches!(
            self.general_category(point),
            "Ll" | "Lu" | "Lo" | "Nd" | "Lm" | "Mn" | "Mc"
        ) {
            return "Pvalid";
        }
        "Disallowed"
    }
}

/// Runs of equal values over the whole code space, as `(start, value)`.
fn runs<T: PartialEq + Copy>(value: impl Fn(u32) -> T) -> Vec<(u32, T)> {
    let mut out: Vec<(u32, T)> = Vec::new();
    for point in 0..CODE_SPACE {
        let v = value(point);
        if out.last().is_none_or(|&(_, last)| last != v) {
            out.push((point, v));
        }
    }
    out
}

fn emit_runs(out: &mut String, doc: &str, name: &str, ty: &str, rows: &[(u32, &str)]) {
    let _ = writeln!(out, "{doc}");
    let _ = writeln!(out, "pub(crate) static {name}: &[(u32, {ty})] = &[");
    for (start, value) in rows {
        let _ = writeln!(out, "    ({}, {ty}::{value}),", hex_literal(*start));
    }
    let _ = writeln!(out, "];");
    let _ = writeln!(out);
}

fn emit_enum(out: &mut String, doc: &str, name: &str, variants: &[(&str, &str)]) {
    let _ = writeln!(out, "{doc}");
    let _ = writeln!(out, "#[derive(Clone, Copy, PartialEq, Eq, Debug)]");
    let _ = writeln!(out, "pub(crate) enum {name} {{");
    for (variant, variant_doc) in variants {
        let _ = writeln!(out, "    /// {variant_doc}");
        let _ = writeln!(out, "    {variant},");
    }
    let _ = writeln!(out, "}}");
    let _ = writeln!(out);
}

fn idna() -> String {
    let ucd = Idna::load();

    // ---- Derived property, with the self-checks the design note promises ----
    let derived: Vec<&'static str> = (0..CODE_SPACE)
        .map(|point| ucd.derived_property(point))
        .collect();
    for point in 0..CODE_SPACE {
        if (0xD800..=0xDFFF).contains(&point) || ucd.general_category(point) == "Cn" {
            continue;
        }
        let unstable = ucd.unstable(point);
        let cwkcf = ucd.changes_when_nfkc_casefolded.contains(&point);
        assert!(
            !unstable || cwkcf,
            "U+{point:04X} is Unstable but not Changes_When_NFKC_Casefolded"
        );
        assert!(
            unstable || !cwkcf || ucd.default_ignorable.contains(&point),
            "U+{point:04X} changes under NFKC_Casefold, is stable under RFC 5892 §2.2, \
             and is not Default_Ignorable"
        );
    }
    assert_eq!(derived[0x200C], "ContextJ");
    assert_eq!(derived[0x0061], "Pvalid");
    assert_eq!(derived[0x0041], "Disallowed");

    let derived_runs = runs(|point| derived[point as usize]);
    let joining_runs = runs(|point| ucd.joining[point as usize]);
    let bidi_runs = runs(|point| ucd.bidi[point as usize]);
    let script_runs = runs(|point| ucd.script[point as usize]);
    let mark_runs = runs(|point| matches!(ucd.general_category(point), "Mn" | "Mc" | "Me"));

    // ---- Emit ----------------------------------------------------------------
    let (major, minor, patch) = UNICODE_VERSION;
    let mut out = String::new();
    let version_line = format!(
        "Derived from the Unicode Character Database {major}.{minor}.{patch} vendored under"
    );
    let dir_line = format!(
        "`crates/iri/unicode/{major}.{minor}.{patch}/`: the RFC 5892 derived property (computed by"
    );
    emit_header(
        &mut out,
        "idna",
        &[
            &version_line,
            &dir_line,
            "the ordered algorithm of RFC 5892 §3), Joining_Type, Bidi_Class, the",
            "scripts RFC 5892 Appendix A consults, the combining-mark general",
            "categories, and the NFKC_Casefold mapping. Normalization is",
            "`purrdf_lex::unicode`'s.",
            "",
            "Every `(start, value)` table is sorted by `start`, starts at `0x0000`,",
            "and gives the value for every code point up to the next row's start.",
        ],
    );
    emit_version_assertion(&mut out);

    emit_enum(
        &mut out,
        "/// The RFC 5892 §3 derived property value.",
        "Derived",
        &[
            ("Pvalid", "PVALID: permitted in a label."),
            (
                "ContextJ",
                "CONTEXTJ: permitted where an Appendix A join rule holds.",
            ),
            (
                "ContextO",
                "CONTEXTO: permitted where an Appendix A rule holds.",
            ),
            ("Disallowed", "DISALLOWED: never permitted."),
            (
                "Unassigned",
                "UNASSIGNED: not assigned in this Unicode version.",
            ),
        ],
    );
    emit_enum(
        &mut out,
        "/// Joining_Type (RFC 5892 Appendix A.1).",
        "JoiningType",
        &[
            ("NonJoining", "U."),
            ("Transparent", "T."),
            ("Left", "L."),
            ("Right", "R."),
            ("Dual", "D."),
            ("JoinCausing", "C."),
        ],
    );
    emit_enum(
        &mut out,
        "/// Bidi_Class, collapsed to the classes RFC 5893 §2 names; every other\n\
         /// class is `Other`, which no label admits.",
        "Bidi",
        &[
            ("Left", "L."),
            ("Right", "R."),
            ("ArabicLetter", "AL."),
            ("EuropeanNumber", "EN."),
            ("EuropeanSeparator", "ES."),
            ("EuropeanTerminator", "ET."),
            ("ArabicNumber", "AN."),
            ("CommonSeparator", "CS."),
            ("NonspacingMark", "NSM."),
            ("BoundaryNeutral", "BN."),
            ("OtherNeutral", "ON."),
            ("Other", "B, S, WS and the explicit directional controls."),
        ],
    );
    emit_enum(
        &mut out,
        "/// Script, reduced to the scripts RFC 5892 Appendix A consults.",
        "Script",
        &[
            ("Other", "Any other script."),
            ("Greek", "Greek."),
            ("Hebrew", "Hebrew."),
            ("Hiragana", "Hiragana."),
            ("Katakana", "Katakana."),
            ("Han", "Han."),
        ],
    );

    emit_runs(
        &mut out,
        "/// RFC 5892 derived property runs.",
        "DERIVED",
        "Derived",
        &derived_runs,
    );
    emit_runs(
        &mut out,
        "/// Joining_Type runs.",
        "JOINING",
        "JoiningType",
        &joining_runs,
    );
    emit_runs(&mut out, "/// Bidi_Class runs.", "BIDI", "Bidi", &bidi_runs);
    emit_runs(
        &mut out,
        "/// Script runs.",
        "SCRIPT",
        "Script",
        &script_runs,
    );

    let _ = writeln!(
        out,
        "/// General_Category Mn, Mc or Me runs (`true` = a combining mark).\n\
         pub(crate) static MARK: &[(u32, bool)] = &["
    );
    for (start, value) in &mark_runs {
        let _ = writeln!(out, "    ({}, {value}),", hex_literal(*start));
    }
    let _ = writeln!(out, "];\n");

    let _ = writeln!(
        out,
        "/// NFKC_Casefold, from `DerivedNormalizationProps.txt`: every code point\n\
         /// whose image is not itself. Sorted.\n\
         pub(crate) static NFKC_CASEFOLD: &[(char, &[char])] = &["
    );
    for (point, image) in &ucd.nfkc_casefold {
        if (0xD800..=0xDFFF).contains(point) || image.as_slice() == [*point] {
            continue;
        }
        let _ = writeln!(
            out,
            "    ({}, {}),",
            char_literal(*point),
            char_slice(image)
        );
    }
    let _ = writeln!(out, "];");
    out
}

// ---------------------------------------------------------------------------
// `ecma-properties`: the names and values an ECMA-262 `\p{…}` escape spells.
//
// What ECMA-262 accepts (§22.2.1.1 Static Semantics: Early Errors, §22.2.2.9.7
// `UnicodeMatchProperty`, §22.2.2.9.8 `UnicodeMatchPropertyValue`):
//
// * `General_Category` and `Script`/`Script_Extensions` values: "the Unicode
//   property values and property value aliases listed in
//   PropertyValueAliases.txt for the properties listed in Table 65", and no
//   others. Every `gc` and `sc` row of `PropertyValueAliases.txt` is therefore
//   taken whole. `Script_Extensions` takes its values from `sc`: the file's
//   `scx` section is empty.
// * Binary properties: exactly the rows of Table 66, "Binary Unicode property
//   aliases and their canonical property names". That table is closed
//   ("implementations must not support any other property names or aliases"),
//   so it is the filter, held below as `TABLE_66`; it does not list every
//   `PropertyAliases.txt` alias (`WSpace` is absent). Every name it lists is
//   checked against `PropertyAliases.txt` (its Note 3: the spellings match that
//   file), except `Any`, `ASCII` and `Assigned`, which are UTS 18
//   pseudo-properties with no row there.
//
// Canonical forms are the ones the translated pattern hands to `regex`: the
// short `gc` value (`Lu`), the long `sc` value (`Latin`), and the long binary
// property name.

/// ECMA-262 Table 66, "Binary Unicode property aliases and their canonical
/// property names": `(canonical property name, [every alias the table lists,
/// canonical name included])`, in the table's row order.
const TABLE_66: &[(&str, &[&str])] = &[
    ("ASCII", &["ASCII"]),
    ("ASCII_Hex_Digit", &["ASCII_Hex_Digit", "AHex"]),
    ("Alphabetic", &["Alphabetic", "Alpha"]),
    ("Any", &["Any"]),
    ("Assigned", &["Assigned"]),
    ("Bidi_Control", &["Bidi_Control", "Bidi_C"]),
    ("Bidi_Mirrored", &["Bidi_Mirrored", "Bidi_M"]),
    ("Case_Ignorable", &["Case_Ignorable", "CI"]),
    ("Cased", &["Cased"]),
    (
        "Changes_When_Casefolded",
        &["Changes_When_Casefolded", "CWCF"],
    ),
    (
        "Changes_When_Casemapped",
        &["Changes_When_Casemapped", "CWCM"],
    ),
    (
        "Changes_When_Lowercased",
        &["Changes_When_Lowercased", "CWL"],
    ),
    (
        "Changes_When_NFKC_Casefolded",
        &["Changes_When_NFKC_Casefolded", "CWKCF"],
    ),
    (
        "Changes_When_Titlecased",
        &["Changes_When_Titlecased", "CWT"],
    ),
    (
        "Changes_When_Uppercased",
        &["Changes_When_Uppercased", "CWU"],
    ),
    ("Dash", &["Dash"]),
    (
        "Default_Ignorable_Code_Point",
        &["Default_Ignorable_Code_Point", "DI"],
    ),
    ("Deprecated", &["Deprecated", "Dep"]),
    ("Diacritic", &["Diacritic", "Dia"]),
    ("Emoji", &["Emoji"]),
    ("Emoji_Component", &["Emoji_Component", "EComp"]),
    ("Emoji_Modifier", &["Emoji_Modifier", "EMod"]),
    ("Emoji_Modifier_Base", &["Emoji_Modifier_Base", "EBase"]),
    ("Emoji_Presentation", &["Emoji_Presentation", "EPres"]),
    (
        "Extended_Pictographic",
        &["Extended_Pictographic", "ExtPict"],
    ),
    ("Extender", &["Extender", "Ext"]),
    ("Grapheme_Base", &["Grapheme_Base", "Gr_Base"]),
    ("Grapheme_Extend", &["Grapheme_Extend", "Gr_Ext"]),
    ("Hex_Digit", &["Hex_Digit", "Hex"]),
    ("IDS_Binary_Operator", &["IDS_Binary_Operator", "IDSB"]),
    ("IDS_Trinary_Operator", &["IDS_Trinary_Operator", "IDST"]),
    ("ID_Continue", &["ID_Continue", "IDC"]),
    ("ID_Start", &["ID_Start", "IDS"]),
    ("Ideographic", &["Ideographic", "Ideo"]),
    ("Join_Control", &["Join_Control", "Join_C"]),
    (
        "Logical_Order_Exception",
        &["Logical_Order_Exception", "LOE"],
    ),
    ("Lowercase", &["Lowercase", "Lower"]),
    ("Math", &["Math"]),
    (
        "Noncharacter_Code_Point",
        &["Noncharacter_Code_Point", "NChar"],
    ),
    ("Pattern_Syntax", &["Pattern_Syntax", "Pat_Syn"]),
    ("Pattern_White_Space", &["Pattern_White_Space", "Pat_WS"]),
    ("Quotation_Mark", &["Quotation_Mark", "QMark"]),
    ("Radical", &["Radical"]),
    ("Regional_Indicator", &["Regional_Indicator", "RI"]),
    ("Sentence_Terminal", &["Sentence_Terminal", "STerm"]),
    ("Soft_Dotted", &["Soft_Dotted", "SD"]),
    ("Terminal_Punctuation", &["Terminal_Punctuation", "Term"]),
    ("Unified_Ideograph", &["Unified_Ideograph", "UIdeo"]),
    ("Uppercase", &["Uppercase", "Upper"]),
    ("Variation_Selector", &["Variation_Selector", "VS"]),
    ("White_Space", &["White_Space", "space"]),
    ("XID_Continue", &["XID_Continue", "XIDC"]),
    ("XID_Start", &["XID_Start", "XIDS"]),
];

/// The Table 66 properties UTS 18 §1.2.1 defines rather than the UCD, so
/// `PropertyAliases.txt` has no row to check them against.
const UTS18_PSEUDO_PROPERTIES: &[&str] = &["ASCII", "Any", "Assigned"];

/// Adds `(alias, canonical)`, refusing one alias with two meanings.
fn insert_alias(table: &mut BTreeMap<String, String>, alias: &str, canonical: &str, what: &str) {
    if let Some(previous) = table.insert(alias.to_owned(), canonical.to_owned()) {
        assert!(
            previous == canonical,
            "{what}: {alias} names both {previous} and {canonical}"
        );
    }
}

/// Every alias of `property`'s values in `PropertyValueAliases.txt`, mapped to
/// the field `canonical` (1: short name, 2: long name).
fn value_aliases(rows: &[Vec<&str>], property: &str, canonical: usize) -> BTreeMap<String, String> {
    let mut table = BTreeMap::new();
    for row in rows.iter().filter(|row| row[0] == property) {
        assert!(row.len() >= 3, "{property}: short row {row:?}");
        for alias in &row[1..] {
            insert_alias(&mut table, alias, row[canonical], property);
        }
    }
    assert!(
        !table.is_empty(),
        "no `{property}` rows in PropertyValueAliases.txt"
    );
    table
}

/// Table 66, each spelling confirmed by `PropertyAliases.txt`.
fn binary_aliases(property_aliases: &[Vec<&str>]) -> BTreeMap<String, String> {
    let mut ucd: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for row in property_aliases {
        // Field 1 is the long name; every field is a spelling of it.
        let spellings = ucd.entry(row[1]).or_default();
        spellings.extend(row.iter().copied());
    }
    let mut table = BTreeMap::new();
    for &(canonical, aliases) in TABLE_66 {
        if UTS18_PSEUDO_PROPERTIES.contains(&canonical) {
            assert!(
                !ucd.contains_key(canonical),
                "{canonical} is now a UCD property; review its Table 66 row"
            );
            assert!(aliases == [canonical], "{canonical}: unexpected aliases");
        } else {
            let spellings = ucd.get(canonical).unwrap_or_else(|| {
                panic!("Table 66 names {canonical}, which PropertyAliases.txt lacks")
            });
            for alias in aliases {
                assert!(
                    spellings.contains(alias),
                    "Table 66 spells {canonical} as {alias}, which PropertyAliases.txt does not"
                );
            }
        }
        for alias in aliases {
            insert_alias(&mut table, alias, canonical, "binary");
        }
    }
    table
}

/// The three ECMA-262 alias tables: `General_Category`, `Script` and binary.
struct EcmaAliases {
    general_category: BTreeMap<String, String>,
    script: BTreeMap<String, String>,
    binary: BTreeMap<String, String>,
}

fn ecma_aliases() -> EcmaAliases {
    let value_text = read("PropertyValueAliases.txt");
    let value_rows: Vec<Vec<&str>> = data_lines(&value_text).collect();
    assert!(
        !value_rows.iter().any(|row| row[0] == "scx"),
        "PropertyValueAliases.txt now lists `scx` values; Script_Extensions no longer \
         shares `sc`'s"
    );
    let property_text = read("PropertyAliases.txt");
    let property_rows: Vec<Vec<&str>> = data_lines(&property_text).collect();
    EcmaAliases {
        general_category: value_aliases(&value_rows, "gc", 1),
        script: value_aliases(&value_rows, "sc", 2),
        binary: binary_aliases(&property_rows),
    }
}

/// One `pub(super) const NAME: &[(&str, &str)]` with its doc comment.
fn emit_alias_table(out: &mut String, doc: &str, name: &str, table: &BTreeMap<String, String>) {
    for line in doc.lines() {
        let _ = writeln!(out, "/// {line}");
    }
    let _ = writeln!(out, "pub(super) const {name}: &[(&str, &str)] = &[");
    for (alias, canonical) in table {
        let _ = writeln!(out, "    ({alias:?}, {canonical:?}),");
    }
    out.push_str("];\n");
}

fn ecma_properties() -> String {
    let aliases = ecma_aliases();
    let (major, minor, patch) = UNICODE_VERSION;
    let mut out = String::new();
    let version_line =
        format!("Source: the Unicode Character Database {major}.{minor}.{patch} vendored under");
    let dir_line =
        format!("`crates/iri/unicode/{major}.{minor}.{patch}/` — `PropertyValueAliases.txt` and");
    emit_header(
        &mut out,
        "ecma-properties",
        &[
            &version_line,
            &dir_line,
            "`PropertyAliases.txt` — filtered to what ECMA-262 §22.2.2.9.7-8 and its",
            "Tables 65 and 66 accept. The data is Unicode's, under the Unicode-3.0",
            "licence. Each table is sorted by alias.",
        ],
    );
    emit_version_assertion(&mut out);
    emit_alias_table(
        &mut out,
        "`(alias, short General_Category value)`: every `gc` spelling in\n\
         `PropertyValueAliases.txt`.",
        "GENERAL_CATEGORY",
        &aliases.general_category,
    );
    out.push('\n');
    emit_alias_table(
        &mut out,
        "`(alias, long Script value)`: every `sc` spelling in\n\
         `PropertyValueAliases.txt`, for both `Script`/`sc` and\n\
         `Script_Extensions`/`scx`.",
        "SCRIPT",
        &aliases.script,
    );
    out.push('\n');
    emit_alias_table(
        &mut out,
        "`(alias, canonical binary property name)`: the rows of ECMA-262 Table 66.",
        "BINARY",
        &aliases.binary,
    );
    out
}

// ---------------------------------------------------------------------------
// `ecma-ranges`: the code points of every ECMA-262 `\p{…}` property, and
// simple case folding.

type Spans = Vec<(u32, u32)>;

/// Every data line of a UAX 44 range file: `(low, high, remaining fields)`.
fn records(name: &str) -> Vec<(u32, u32, Vec<String>)> {
    data_lines(&read(name))
        .map(|f| {
            let (low, high) = range(f[0]);
            (low, high, f[1..].iter().map(|s| (*s).to_owned()).collect())
        })
        .collect()
}

/// `spans` sorted, with overlapping and adjacent spans joined.
fn merge(spans: impl IntoIterator<Item = (u32, u32)>) -> Spans {
    let mut sorted: Spans = spans.into_iter().collect();
    sorted.sort_unstable();
    let mut result: Spans = Vec::new();
    for (low, high) in sorted {
        match result.last_mut() {
            Some(last) if low <= last.1 + 1 => last.1 = last.1.max(high),
            _ => result.push((low, high)),
        }
    }
    result
}

/// `spans` less the sorted, merged `cuts`.
fn subtract(spans: &[(u32, u32)], cuts: &[(u32, u32)]) -> Spans {
    let mut result = Vec::new();
    for &(low, high) in spans {
        let mut current = low;
        let mut done = false;
        for &(left, right) in cuts {
            if right < current {
                continue;
            }
            if left > high {
                break;
            }
            if left > current {
                result.push((current, left - 1));
            }
            current = current.max(right + 1);
            if current > high {
                done = true;
                break;
            }
        }
        if !done && current <= high {
            result.push((current, high));
        }
    }
    result
}

fn complement(spans: &[(u32, u32)]) -> Spans {
    subtract(&[(0, MAX_CODE_POINT)], &merge(spans.iter().copied()))
}

/// The canonical values of an alias table.
fn canonical_values(table: &BTreeMap<String, String>) -> BTreeSet<String> {
    table.values().cloned().collect()
}

/// A hex literal with `_` every four digits from the right: `0x10_FFFF`.
fn grouped_hex(number: u32) -> String {
    let digits = format!("{number:X}");
    let mut groups: Vec<&str> = Vec::new();
    let mut rest = digits.as_str();
    while rest.len() > 4 {
        let (head, tail) = rest.split_at(rest.len() - 4);
        groups.push(tail);
        rest = head;
    }
    groups.push(rest);
    groups.reverse();
    format!("0x{}", groups.join("_"))
}

/// Every General_Category value's spans, `Cn` the complement of the assigned.
fn category_spans() -> BTreeMap<String, Spans> {
    let mut categories: BTreeMap<String, Spans> = BTreeMap::new();
    let mut first: Option<(u32, String)> = None;
    for line in read("UnicodeData.txt").lines() {
        let f: Vec<&str> = line.split(';').collect();
        let point = hex(f[0]);
        let (name, category) = (f[1], f[2]);
        if name.ends_with(", First>") {
            first = Some((point, category.to_owned()));
        } else if name.ends_with(", Last>") {
            let (start, first_category) = first.take().expect("range Last without First");
            assert_eq!(first_category, category, "a range changes category");
            categories
                .entry(category.to_owned())
                .or_default()
                .push((start, point));
        } else {
            categories
                .entry(category.to_owned())
                .or_default()
                .push((point, point));
        }
    }
    assert!(first.is_none(), "range First without Last");
    let assigned = merge(categories.values().flatten().copied());
    categories.insert("Cn".to_owned(), complement(&assigned));
    categories
        .into_iter()
        .map(|(name, spans)| (name, merge(spans)))
        .collect()
}

fn ecma_ranges() -> String {
    let aliases = ecma_aliases();
    let category_ranges = category_spans();
    let mut properties: BTreeMap<String, Spans> = BTreeMap::new();
    for name in canonical_values(&aliases.general_category) {
        let spans = if let Some(spans) = category_ranges.get(&name) {
            spans.clone()
        } else if name == "LC" {
            merge(
                ["Lu", "Ll", "Lt"]
                    .iter()
                    .flat_map(|kind| category_ranges[*kind].iter().copied()),
            )
        } else {
            merge(
                category_ranges
                    .iter()
                    .filter(|(kind, _)| kind.starts_with(name.as_str()))
                    .flat_map(|(_, spans)| spans.iter().copied()),
            )
        };
        properties.insert(format!("gc={name}"), spans);
    }

    let mut scripts: BTreeMap<String, Spans> = BTreeMap::new();
    for (low, high, fields) in records("Scripts.txt") {
        scripts
            .entry(fields[0].clone())
            .or_default()
            .push((low, high));
    }
    let assigned_script = merge(scripts.values().flatten().copied());
    scripts.insert("Unknown".to_owned(), complement(&assigned_script));
    let scripts: BTreeMap<String, Spans> = scripts
        .into_iter()
        .map(|(name, spans)| (name, merge(spans)))
        .collect();
    let script_names = canonical_values(&aliases.script);
    for name in &script_names {
        properties.insert(
            format!("Script={name}"),
            scripts.get(name).cloned().unwrap_or_default(),
        );
    }
    // Every listed extension overrides Script for its range. Unlisted points
    // retain Script, per the UCD @missing declaration.
    let value_text = read("PropertyValueAliases.txt");
    let short_to_long: BTreeMap<&str, &str> = value_text
        .lines()
        .filter_map(|raw| {
            let f: Vec<&str> = raw
                .split('#')
                .next()
                .unwrap_or("")
                .split(';')
                .map(str::trim)
                .collect();
            (f.len() >= 3 && f[0] == "sc").then(|| (f[1], f[2]))
        })
        .collect();
    let extension_rows = records("ScriptExtensions.txt");
    let overrides = merge(extension_rows.iter().map(|&(low, high, _)| (low, high)));
    for name in &script_names {
        let base = subtract(scripts.get(name).map_or(&[][..], Vec::as_slice), &overrides);
        let added = extension_rows.iter().filter_map(|(low, high, fields)| {
            fields[0]
                .split_whitespace()
                .any(|alias| {
                    short_to_long
                        .get(alias)
                        .unwrap_or_else(|| panic!("unknown script alias {alias}"))
                        == name
                })
                .then_some((*low, *high))
        });
        properties.insert(
            format!("Script_Extensions={name}"),
            merge(base.into_iter().chain(added)),
        );
    }

    let mut binaries: BTreeMap<String, Spans> = BTreeMap::new();
    for file in [
        "PropList.txt",
        "DerivedCoreProperties.txt",
        "DerivedNormalizationProps.txt",
        "emoji-data.txt",
    ] {
        for (low, high, fields) in records(file) {
            binaries
                .entry(fields[0].clone())
                .or_default()
                .push((low, high));
        }
    }
    // Bidi_Mirrored is stored in UnicodeData's tenth field rather than a
    // stand-alone property file.
    for line in read("UnicodeData.txt").lines() {
        let f: Vec<&str> = line.split(';').collect();
        if f[9] == "Y" {
            let point = hex(f[0]);
            binaries
                .entry("Bidi_Mirrored".to_owned())
                .or_default()
                .push((point, point));
        }
    }
    binaries.insert("Any".to_owned(), vec![(0, MAX_CODE_POINT)]);
    binaries.insert("ASCII".to_owned(), vec![(0, 0x7F)]);
    binaries.insert("Assigned".to_owned(), complement(&category_ranges["Cn"]));
    let binary_names = canonical_values(&aliases.binary);
    let missing: Vec<&String> = binary_names
        .iter()
        .filter(|name| !binaries.contains_key(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "missing binary UCD properties: {missing:?}"
    );
    for name in &binary_names {
        properties.insert(name.clone(), merge(binaries[name].iter().copied()));
    }
    for special in ["ID_Start", "ID_Continue"] {
        assert!(!properties[special].is_empty(), "{special} is empty");
    }

    let (major, minor, patch) = UNICODE_VERSION;
    let mut out = String::new();
    let summary = format!(
        "Exact Unicode {major}.{minor}.{patch} executable property ranges for ECMA-262 /u."
    );
    let source = source_line(
        "`UnicodeData.txt`, `Scripts.txt`, `ScriptExtensions.txt`, \
         `PropertyValueAliases.txt`, `PropList.txt`, `DerivedCoreProperties.txt`, \
         `DerivedNormalizationProps.txt`, `emoji-data.txt` and `CaseFolding.txt`",
    );
    let mut doc = vec![summary, String::new()];
    doc.extend(wrap(&source, 76));
    emit_header(
        &mut out,
        "ecma-ranges",
        &doc.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    emit_version_assertion(&mut out);
    out.push_str("pub(super) const RANGES: &[(&str, &[(u32, u32)])] = &[\n");
    for (name, spans) in &properties {
        let _ = writeln!(out, "    (\"{name}\", &[");
        for &(low, high) in spans {
            let _ = writeln!(
                out,
                "        ({}, {}),",
                grouped_hex(low),
                grouped_hex(high)
            );
        }
        out.push_str("    ]),\n");
    }
    out.push_str("];\n");

    let mut folding: BTreeMap<u32, u32> = BTreeMap::new();
    for f in data_lines(&read("CaseFolding.txt")) {
        if f.len() >= 3 && (f[1] == "C" || f[1] == "S") {
            let mapping = hex_list(f[2]);
            if let [target] = mapping[..] {
                folding.insert(hex(f[0]), target);
            }
        }
    }
    out.push_str("pub(super) const CASE_FOLD: &[(u32, u32)] = &[\n");
    for (&source, &target) in &folding {
        let _ = writeln!(
            out,
            "    ({}, {}),",
            grouped_hex(source),
            grouped_hex(target)
        );
    }
    out.push_str("];\n");
    let reverse: BTreeSet<(u32, u32)> = folding
        .iter()
        .map(|(&source, &target)| (target, source))
        .collect();
    out.push_str("pub(super) const CASE_FOLD_REVERSE: &[(u32, u32)] = &[\n");
    for &(target, source) in &reverse {
        let _ = writeln!(
            out,
            "    ({}, {}),",
            grouped_hex(target),
            grouped_hex(source)
        );
    }
    out.push_str("];\n");
    out
}

fn main() {
    let set = std::env::args().nth(1).unwrap_or_default();
    assert_one_version();
    let out = match set.as_str() {
        "normalization" => normalization(),
        "text" => text(),
        "idna" => idna(),
        "ecma-properties" => ecma_properties(),
        "ecma-ranges" => ecma_ranges(),
        other => panic!(
            "unknown table set {other:?}: expected normalization, text, idna, ecma-properties or \
             ecma-ranges"
        ),
    };
    print!("{out}");
}

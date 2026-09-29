// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Regenerates `crates/iri/src/idna_tables.rs` from the vendored Unicode
//! 17.0.0 character database under `crates/iri/unicode/17.0.0/`.
//!
//! What it computes:
//!
//! * the RFC 5892 derived property of every code point, by the ordered
//!   algorithm of RFC 5892 §3 over the categories of §2;
//! * Joining_Type, Bidi_Class, the five Appendix A scripts, and the
//!   combining-mark general categories the label rules consult;
//! * the NFC data (canonical combining classes, full canonical
//!   decompositions, primary composite pairs);
//! * the NFKC_Casefold mapping the local mapping step applies.
//!
//! Every property RFC 5892 §2 names is read from its UCD file:
//! Join_Control, Noncharacter_Code_Point and White_Space from `PropList.txt`,
//! Default_Ignorable_Code_Point from `DerivedCoreProperties.txt`,
//! Hangul_Syllable_Type from `HangulSyllableType.txt`, and the three
//! IgnorableBlocks from `Blocks.txt`.
//!
//! Output goes to stdout; `scripts/check-generated.sh` pipes it through
//! `rustfmt` and byte-compares it with the committed file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The highest Unicode scalar value plus one.
const CODE_SPACE: u32 = 0x11_0000;

// ---- Hangul syllable arithmetic (Unicode Standard §3.12) --------------------

const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT;
const S_COUNT: u32 = L_COUNT * N_COUNT;

/// Everything read from `UnicodeData.txt` for one code point.
#[derive(Clone, Default)]
struct Entry {
    general_category: String,
    ccc: u8,
    /// The decomposition mapping (one level, as written), and whether it is a
    /// compatibility (`<tag>`) mapping.
    decomposition: Option<(bool, Vec<u32>)>,
}

/// The parsed character database the generator works over.
struct Ucd {
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

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn hex(field: &str) -> u32 {
    purrdf_hash::hex::parse_u32(field.trim().as_bytes())
        .unwrap_or_else(|| panic!("bad hex {field:?}"))
}

fn hex_list(field: &str) -> Vec<u32> {
    field.split_whitespace().map(hex).collect()
}

/// `lo..hi` or a single code point, per UAX 44 §4.2.
fn range(field: &str) -> (u32, u32) {
    match field.trim().split_once("..") {
        Some((lo, hi)) => (hex(lo), hex(hi)),
        None => {
            let cp = hex(field);
            (cp, cp)
        }
    }
}

/// The data fields of a UCD property file line, comment stripped; `None` for
/// blank and comment lines.
fn fields(line: &str) -> Option<Vec<&str>> {
    let data = line.split('#').next().unwrap_or("").trim();
    if data.is_empty() {
        None
    } else {
        Some(data.split(';').map(str::trim).collect())
    }
}

fn parse_unicode_data(text: &str) -> BTreeMap<u32, Entry> {
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
        let cp = hex(f[0]);
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
            range_start = Some((cp, entry));
        } else if f[1].ends_with(", Last>") {
            let (start, first) = range_start.take().expect("range Last without First");
            for member in start..=cp {
                entries.insert(member, first.clone());
            }
        } else {
            entries.insert(cp, entry);
        }
    }
    entries
}

/// Every code point with `property` in a binary-property file.
fn binary_property(text: &str, property: &str) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for line in text.lines() {
        if let Some(f) = fields(line)
            && f.len() == 2
            && f[1] == property
        {
            let (lo, hi) = range(f[0]);
            set.extend(lo..=hi);
        }
    }
    set
}

fn parse_case_folding(text: &str) -> BTreeMap<u32, Vec<u32>> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        if let Some(f) = fields(line)
            && (f[1] == "C" || f[1] == "F")
        {
            map.insert(hex(f[0]), hex_list(f[2]));
        }
    }
    map
}

fn parse_nfkc_casefold(text: &str) -> BTreeMap<u32, Vec<u32>> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        if let Some(f) = fields(line)
            && f.len() >= 3
            && f[1] == "NFKC_CF"
        {
            let (lo, hi) = range(f[0]);
            let image = hex_list(f[2]);
            for cp in lo..=hi {
                map.insert(cp, image.clone());
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
            let (lo, hi) = range(f[0]);
            let value = canon(f[1]);
            for cp in lo..=hi {
                table[cp as usize] = value;
            }
        }
    }
    for line in text.lines() {
        if let Some(f) = fields(line) {
            let (lo, hi) = range(f[0]);
            let value = canon(f[1]);
            for cp in lo..=hi {
                table[cp as usize] = value;
            }
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
        let hit = text.lines().find_map(|line| {
            let f = fields(line)?;
            (f[1] == *name).then(|| range(f[0]))
        });
        found.push(hit.unwrap_or_else(|| panic!("Blocks.txt has no block named {name:?}")));
    }
    found
}

impl Ucd {
    fn load(unicode_dir: &Path) -> Self {
        let prop_list = read(&unicode_dir.join("PropList.txt"));
        let hangul = read(&unicode_dir.join("HangulSyllableType.txt"));
        let dnp = read(&unicode_dir.join("DerivedNormalizationProps.txt"));
        let entries = parse_unicode_data(&read(&unicode_dir.join("UnicodeData.txt")));
        let full_composition_exclusion = binary_property(&dnp, "Full_Composition_Exclusion");
        let mut pairs = BTreeMap::new();
        for (&cp, entry) in &entries {
            if let Some((false, list)) = &entry.decomposition
                && list.len() == 2
                && !full_composition_exclusion.contains(&cp)
            {
                assert!(
                    pairs.insert((list[0], list[1]), cp).is_none(),
                    "two primary composites share a decomposition"
                );
            }
        }
        Self {
            entries,
            default_ignorable: binary_property(
                &read(&unicode_dir.join("DerivedCoreProperties.txt")),
                "Default_Ignorable_Code_Point",
            ),
            white_space: binary_property(&prop_list, "White_Space"),
            noncharacter: binary_property(&prop_list, "Noncharacter_Code_Point"),
            join_control: binary_property(&prop_list, "Join_Control"),
            old_hangul_jamo: ["L", "V", "T"]
                .iter()
                .flat_map(|value| binary_property(&hangul, value))
                .collect(),
            case_fold: parse_case_folding(&read(&unicode_dir.join("CaseFolding.txt"))),
            changes_when_nfkc_casefolded: binary_property(&dnp, "Changes_When_NFKC_Casefolded"),
            nfkc_casefold: parse_nfkc_casefold(&dnp),
            joining: enumerated_property(
                &read(&unicode_dir.join("DerivedJoiningType.txt")),
                canon_joining,
            ),
            bidi: enumerated_property(&read(&unicode_dir.join("DerivedBidiClass.txt")), canon_bidi),
            script: enumerated_property(&read(&unicode_dir.join("Scripts.txt")), |value| {
                if value == "Unknown" {
                    "Other"
                } else {
                    canon_script(value)
                }
            }),
            ignorable_blocks: named_blocks(
                &read(&unicode_dir.join("Blocks.txt")),
                &[
                    "Combining Diacritical Marks for Symbols",
                    "Musical Symbols",
                    "Ancient Greek Musical Notation",
                ],
            ),
            pairs,
        }
    }

    fn general_category(&self, cp: u32) -> &str {
        self.entries
            .get(&cp)
            .map_or("Cn", |e| e.general_category.as_str())
    }

    fn ccc(&self, cp: u32) -> u8 {
        self.entries.get(&cp).map_or(0, |e| e.ccc)
    }

    // ---- Normalization (Unicode Standard §3.11) ------------------------------

    /// Full decomposition of one code point, recursively; compatibility
    /// mappings too when `compat`.
    fn decompose_into(&self, cp: u32, compat: bool, out: &mut Vec<u32>) {
        if (S_BASE..S_BASE + S_COUNT).contains(&cp) {
            let s = cp - S_BASE;
            out.push(L_BASE + s / N_COUNT);
            out.push(V_BASE + (s % N_COUNT) / T_COUNT);
            if !s.is_multiple_of(T_COUNT) {
                out.push(T_BASE + s % T_COUNT);
            }
            return;
        }
        match self.entries.get(&cp).and_then(|e| e.decomposition.as_ref()) {
            Some((is_compat, list)) if compat || !is_compat => {
                for &part in list {
                    self.decompose_into(part, compat, out);
                }
            }
            _ => out.push(cp),
        }
    }

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
            text[start..end].sort_by_key(|&cp| self.ccc(cp));
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
        for &cp in text {
            let class = self.ccc(cp);
            if let Some(at) = starter {
                let blocked = last_class.is_some_and(|last| last == 0 || last >= class);
                if !blocked && let Some(composite) = self.primary_composite(out[at], cp) {
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
            out.push(cp);
        }
        out
    }

    fn normalize(&self, text: &[u32], compat: bool) -> Vec<u32> {
        let mut decomposed = Vec::new();
        for &cp in text {
            self.decompose_into(cp, compat, &mut decomposed);
        }
        self.reorder(&mut decomposed);
        self.compose(&decomposed)
    }

    fn case_fold(&self, text: &[u32]) -> Vec<u32> {
        let mut out = Vec::new();
        for &cp in text {
            match self.case_fold.get(&cp) {
                Some(image) => out.extend_from_slice(image),
                None => out.push(cp),
            }
        }
        out
    }

    // ---- RFC 5892 §2 categories ----------------------------------------------

    /// §2.2 Unstable: `toNFKC(toCaseFold(toNFKC(cp))) != cp`.
    fn unstable(&self, cp: u32) -> bool {
        let once = self.normalize(&[cp], true);
        let folded = self.case_fold(&once);
        self.normalize(&folded, true) != [cp]
    }

    /// §2.3 IgnorableProperties.
    fn ignorable_properties(&self, cp: u32) -> bool {
        self.default_ignorable.contains(&cp)
            || self.white_space.contains(&cp)
            || self.noncharacter.contains(&cp)
    }

    /// §2.6 Exceptions.
    fn exception(cp: u32) -> Option<&'static str> {
        match cp {
            0x00DF | 0x03C2 | 0x06FD | 0x06FE | 0x0F0B | 0x3007 => Some("Pvalid"),
            0x00B7 | 0x0375 | 0x05F3 | 0x05F4 | 0x30FB | 0x0660..=0x0669 | 0x06F0..=0x06F9 => {
                Some("ContextO")
            }
            0x0640 | 0x07FA | 0x302E | 0x302F | 0x3031..=0x3035 | 0x303B => Some("Disallowed"),
            _ => None,
        }
    }

    /// RFC 5892 §3: the derived property, rules applied in order.
    fn derived_property(&self, cp: u32) -> &'static str {
        if let Some(value) = Self::exception(cp) {
            return value;
        }
        // BackwardCompatible (G) is the empty set.
        if self.general_category(cp) == "Cn" && !self.noncharacter.contains(&cp) {
            return "Unassigned";
        }
        if cp == 0x2D || (0x30..=0x39).contains(&cp) || (0x61..=0x7A).contains(&cp) {
            return "Pvalid";
        }
        if self.join_control.contains(&cp) {
            return "ContextJ";
        }
        let surrogate = (0xD800..=0xDFFF).contains(&cp);
        if !surrogate && self.unstable(cp) {
            return "Disallowed";
        }
        if self.ignorable_properties(cp) {
            return "Disallowed";
        }
        if self
            .ignorable_blocks
            .iter()
            .any(|&(lo, hi)| (lo..=hi).contains(&cp))
        {
            return "Disallowed";
        }
        if self.old_hangul_jamo.contains(&cp) {
            return "Disallowed";
        }
        if matches!(
            self.general_category(cp),
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
    for cp in 0..CODE_SPACE {
        let v = value(cp);
        if out.last().is_none_or(|&(_, last)| last != v) {
            out.push((cp, v));
        }
    }
    out
}

/// A code point as a separated eight-digit hex literal, `0x0001_F600`.
fn hex_literal(cp: u32) -> String {
    format!("0x{:04X}_{:04X}", cp >> 16, cp & 0xFFFF)
}

fn ch(cp: u32) -> String {
    assert!(
        char::from_u32(cp).is_some(),
        "{cp:04X} is not a scalar value"
    );
    format!("'\\u{{{cp:04X}}}'")
}

fn char_slice(list: &[u32]) -> String {
    let parts: Vec<String> = list.iter().map(|&cp| ch(cp)).collect();
    format!("&[{}]", parts.join(", "))
}

fn emit_runs(out: &mut String, doc: &str, name: &str, ty: &str, rows: &[(u32, &str)]) {
    writeln!(out, "{doc}").unwrap();
    writeln!(out, "pub(crate) static {name}: &[(u32, {ty})] = &[").unwrap();
    for (start, value) in rows {
        writeln!(out, "    ({}, {ty}::{value}),", hex_literal(*start)).unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();
}

fn emit_enum(out: &mut String, doc: &str, name: &str, variants: &[(&str, &str)]) {
    writeln!(out, "{doc}").unwrap();
    writeln!(out, "#[derive(Clone, Copy, PartialEq, Eq, Debug)]").unwrap();
    writeln!(out, "pub(crate) enum {name} {{").unwrap();
    for (variant, variant_doc) in variants {
        writeln!(out, "    /// {variant_doc}").unwrap();
        writeln!(out, "    {variant},").unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let unicode_dir = manifest_dir.join("unicode/17.0.0");
    let ucd = Ucd::load(&unicode_dir);

    // ---- Derived property, with the self-checks the design note promises ----
    let derived: Vec<&'static str> = (0..CODE_SPACE).map(|cp| ucd.derived_property(cp)).collect();
    for cp in 0..CODE_SPACE {
        if (0xD800..=0xDFFF).contains(&cp) || ucd.general_category(cp) == "Cn" {
            continue;
        }
        let unstable = ucd.unstable(cp);
        let cwkcf = ucd.changes_when_nfkc_casefolded.contains(&cp);
        assert!(
            !unstable || cwkcf,
            "U+{cp:04X} is Unstable but not Changes_When_NFKC_Casefolded"
        );
        assert!(
            unstable || !cwkcf || ucd.default_ignorable.contains(&cp),
            "U+{cp:04X} changes under NFKC_Casefold, is stable under RFC 5892 §2.2, \
             and is not Default_Ignorable"
        );
    }
    assert_eq!(derived[0x200C], "ContextJ");
    assert_eq!(derived[0x0061], "Pvalid");
    assert_eq!(derived[0x0041], "Disallowed");

    let derived_runs = runs(|cp| derived[cp as usize]);
    let joining_runs = runs(|cp| ucd.joining[cp as usize]);
    let bidi_runs = runs(|cp| ucd.bidi[cp as usize]);
    let script_runs = runs(|cp| ucd.script[cp as usize]);
    let mark_runs = runs(|cp| matches!(ucd.general_category(cp), "Mn" | "Mc" | "Me"));
    let ccc_runs = runs(|cp| ucd.ccc(cp));

    let mut decompositions: Vec<(u32, Vec<u32>)> = Vec::new();
    for (&cp, entry) in &ucd.entries {
        if matches!(entry.decomposition, Some((false, _))) {
            let mut full = Vec::new();
            ucd.decompose_into(cp, false, &mut full);
            decompositions.push((cp, full));
        }
    }
    let pairs = &ucd.pairs;

    // ---- Emit ----------------------------------------------------------------
    let mut out = String::new();
    for line in [
        "// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>",
        "// SPDX-FileCopyrightText: Unicode, Inc. <https://www.unicode.org>",
        "// SPDX-License-Identifier: (MIT OR Apache-2.0 OR MulanPSL-2.0) AND Unicode-3.0",
        "",
        "//! @generated by `cargo run -p purrdf-iri --example gen_idna_tables --locked`.",
        "//! Do not hand-edit; `scripts/check-generated.sh` diffs this file against the",
        "//! generator's stdout.",
        "//!",
        "//! Derived from the Unicode Character Database 17.0.0 vendored under",
        "//! `crates/iri/unicode/17.0.0/`: the RFC 5892 derived property (computed by",
        "//! the ordered algorithm of RFC 5892 §3), Joining_Type, Bidi_Class, the",
        "//! scripts RFC 5892 Appendix A consults, the combining-mark general",
        "//! categories, the NFC data, and the NFKC_Casefold mapping.",
        "//!",
        "//! Every `(start, value)` table is sorted by `start`, starts at `0x0000`,",
        "//! and gives the value for every code point up to the next row's start.",
        "",
    ] {
        writeln!(out, "{line}").unwrap();
    }
    writeln!(
        out,
        "/// The Unicode version every table in this file was derived from.\n\
         pub(crate) const UNICODE_VERSION: (u8, u8, u8) = (17, 0, 0);\n"
    )
    .unwrap();

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

    writeln!(
        out,
        "/// General_Category Mn, Mc or Me runs (`true` = a combining mark).\n\
         pub(crate) static MARK: &[(u32, bool)] = &["
    )
    .unwrap();
    for (start, value) in &mark_runs {
        writeln!(out, "    ({}, {value}),", hex_literal(*start)).unwrap();
    }
    writeln!(out, "];\n").unwrap();

    writeln!(
        out,
        "/// Canonical_Combining_Class runs.\n\
         pub(crate) static CCC: &[(u32, u8)] = &["
    )
    .unwrap();
    for (start, value) in &ccc_runs {
        writeln!(out, "    ({}, {value}),", hex_literal(*start)).unwrap();
    }
    writeln!(out, "];\n").unwrap();

    writeln!(
        out,
        "/// The full canonical decomposition of every code point that has one,\n\
         /// Hangul syllables excepted (they decompose algorithmically). Sorted.\n\
         pub(crate) static CANONICAL_DECOMPOSITION: &[(char, &[char])] = &["
    )
    .unwrap();
    for (cp, full) in &decompositions {
        writeln!(out, "    ({}, {}),", ch(*cp), char_slice(full)).unwrap();
    }
    writeln!(out, "];\n").unwrap();

    writeln!(
        out,
        "/// Primary composites: `(first, second, composite)` for every\n\
         /// two-code-point canonical decomposition whose composite is not\n\
         /// Full_Composition_Exclusion, Hangul excepted. Sorted by `(first, second)`.\n\
         pub(crate) static COMPOSITION: &[(char, char, char)] = &["
    )
    .unwrap();
    for ((first, second), composite) in pairs {
        writeln!(
            out,
            "    ({}, {}, {}),",
            ch(*first),
            ch(*second),
            ch(*composite)
        )
        .unwrap();
    }
    writeln!(out, "];\n").unwrap();

    writeln!(
        out,
        "/// NFKC_Casefold, from `DerivedNormalizationProps.txt`: every code point\n\
         /// whose image is not itself. Sorted.\n\
         pub(crate) static NFKC_CASEFOLD: &[(char, &[char])] = &["
    )
    .unwrap();
    for (cp, image) in &ucd.nfkc_casefold {
        if (0xD800..=0xDFFF).contains(cp) || image.as_slice() == [*cp] {
            continue;
        }
        writeln!(out, "    ({}, {}),", ch(*cp), char_slice(image)).unwrap();
    }
    writeln!(out, "];").unwrap();

    print!("{out}");
}

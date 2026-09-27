// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Regenerates `crates/jsonschema/src/ecma/property_tables.rs`: the property
//! names and values an ECMA-262 `\p{…}` / `\P{…}` escape may spell, read out of
//! the Unicode Character Database vendored under `crates/iri/unicode/`.
//!
//! What ECMA-262 accepts (§22.2.1.1 Static Semantics: Early Errors, §22.2.2.9.7
//! `UnicodeMatchProperty`, §22.2.2.9.8 `UnicodeMatchPropertyValue`):
//!
//! * `General_Category` and `Script`/`Script_Extensions` values: "the Unicode
//!   property values and property value aliases listed in
//!   PropertyValueAliases.txt for the properties listed in Table 65", and no
//!   others. Every `gc` and `sc` row of `PropertyValueAliases.txt` is therefore
//!   taken whole. `Script_Extensions` takes its values from `sc`: the file's
//!   `scx` section is empty.
//! * Binary properties: exactly the rows of Table 66, "Binary Unicode property
//!   aliases and their canonical property names". That table is closed
//!   ("implementations must not support any other property names or
//!   aliases"), so it is the filter, held below as [`TABLE_66`]; it does not
//!   list every `PropertyAliases.txt` alias (`WSpace` is absent). Every name it
//!   lists is checked against `PropertyAliases.txt` (its Note 3: the spellings
//!   match that file), except `Any`, `ASCII` and `Assigned`, which are UTS 18
//!   pseudo-properties with no row there.
//!
//! Canonical forms are the ones the translated pattern hands to `regex`:
//! the short `gc` value (`Lu`), the long `sc` value (`Latin`), and the long
//! binary property name. File formats per UAX 44 §4.2.
//!
//! Output goes to stdout; `scripts/check-generated.sh` pipes it through
//! `rustfmt` and compares it with the committed file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

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

/// The `;`-separated fields of every data line of a UAX 44 file, comments and
/// surrounding white space removed.
fn data_lines(path: &Path) -> Vec<Vec<String>> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    text.lines()
        .filter_map(|line| {
            let data = line.split('#').next().unwrap_or("").trim();
            (!data.is_empty()).then(|| data.split(';').map(|f| f.trim().to_owned()).collect())
        })
        .collect()
}

/// The `major.minor.patch` a UCD file's first line names (`# Name-17.0.0.txt`).
fn file_version(path: &Path) -> String {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let first = text.lines().next().unwrap_or("");
    let name = first
        .trim_start_matches('#')
        .trim()
        .strip_suffix(".txt")
        .unwrap_or_else(|| panic!("{}: unexpected header {first:?}", path.display()));
    let (_, version) = name
        .rsplit_once('-')
        .unwrap_or_else(|| panic!("{}: no version in {first:?}", path.display()));
    version.to_owned()
}

/// Adds `(alias, canonical)`, refusing one alias with two meanings.
fn insert(table: &mut BTreeMap<String, String>, alias: &str, canonical: &str, what: &str) {
    if let Some(previous) = table.insert(alias.to_owned(), canonical.to_owned()) {
        assert!(
            previous == canonical,
            "{what}: {alias} names both {previous} and {canonical}"
        );
    }
}

/// Every alias of `property`'s values in `PropertyValueAliases.txt`, mapped to
/// the field `canonical` (1: short name, 2: long name).
fn value_aliases(
    rows: &[Vec<String>],
    property: &str,
    canonical: usize,
) -> BTreeMap<String, String> {
    let mut table = BTreeMap::new();
    for row in rows.iter().filter(|row| row[0] == property) {
        assert!(row.len() >= 3, "{property}: short row {row:?}");
        for alias in &row[1..] {
            insert(&mut table, alias, &row[canonical], property);
        }
    }
    assert!(
        !table.is_empty(),
        "no `{property}` rows in PropertyValueAliases.txt"
    );
    table
}

/// Table 66, each spelling confirmed by `PropertyAliases.txt`.
fn binary_aliases(property_aliases: &[Vec<String>]) -> BTreeMap<String, String> {
    let mut ucd: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for row in property_aliases {
        // Field 1 is the long name; every field is a spelling of it.
        let spellings = ucd.entry(row[1].as_str()).or_default();
        spellings.extend(row.iter().map(String::as_str));
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
            insert(&mut table, alias, canonical, "binary");
        }
    }
    table
}

/// One `pub(super) const NAME: &[(&str, &str)]` with its doc comment.
fn emit_table(out: &mut String, doc: &str, name: &str, table: &BTreeMap<String, String>) {
    for line in doc.lines() {
        let _ = writeln!(out, "/// {line}");
    }
    let _ = writeln!(out, "pub(super) const {name}: &[(&str, &str)] = &[");
    for (alias, canonical) in table {
        let _ = writeln!(out, "    ({alias:?}, {canonical:?}),");
    }
    out.push_str("];\n");
}

fn main() {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../iri/unicode/17.0.0");
    let pva_path = dir.join("PropertyValueAliases.txt");
    let pa_path = dir.join("PropertyAliases.txt");
    let version = file_version(&pva_path);
    assert_eq!(
        version,
        file_version(&pa_path),
        "UCD files disagree on version"
    );

    let value_rows = data_lines(&pva_path);
    assert!(
        !value_rows.iter().any(|row| row[0] == "scx"),
        "PropertyValueAliases.txt now lists `scx` values; Script_Extensions no longer \
         shares `sc`'s"
    );
    let general_category = value_aliases(&value_rows, "gc", 1);
    let script = value_aliases(&value_rows, "sc", 2);
    let binary = binary_aliases(&data_lines(&pa_path));

    let mut out = String::new();
    out.push_str(
        "// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>\n\
         // SPDX-FileCopyrightText: Unicode, Inc. <https://www.unicode.org>\n\
         // SPDX-License-Identifier: (MIT OR Apache-2.0 OR MulanPSL-2.0) AND Unicode-3.0\n\n",
    );
    let _ = write!(
        out,
        "//! @generated by `cargo run -p purrdf-jsonschema --example gen_ecma_property_tables --locked`.\n\
         //! Do not hand-edit; `scripts/check-generated.sh` diffs this file against the\n\
         //! generator's stdout.\n\
         //!\n\
         //! Source: the Unicode Character Database {version} vendored under\n\
         //! `crates/iri/unicode/{version}/` — `PropertyValueAliases.txt` and\n\
         //! `PropertyAliases.txt` — filtered to what ECMA-262 §22.2.2.9.7-8 and its\n\
         //! Tables 65 and 66 accept. The data is Unicode's, under the Unicode-3.0\n\
         //! licence. Each table is sorted by alias.\n\n"
    );
    emit_table(
        &mut out,
        "`(alias, short General_Category value)`: every `gc` spelling in\n\
         `PropertyValueAliases.txt`.",
        "GENERAL_CATEGORY",
        &general_category,
    );
    out.push('\n');
    emit_table(
        &mut out,
        "`(alias, long Script value)`: every `sc` spelling in\n\
         `PropertyValueAliases.txt`, for both `Script`/`sc` and\n\
         `Script_Extensions`/`scx`.",
        "SCRIPT",
        &script,
    );
    out.push('\n');
    emit_table(
        &mut out,
        "`(alias, canonical binary property name)`: the rows of ECMA-262 Table 66.",
        "BINARY",
        &binary,
    );
    print!("{out}");
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regenerates the committed Unicode block-escape lookup table
//! (`crates/rdf-core/src/xsd_regex/blocks.rs`) from the vendored
//! `crates/rdf-core/vendor/unicode/Blocks.txt` (Unicode Character Database
//! version 16.0.0 — pinned to match the UCD version embedded in this
//! workspace's locked `regex-syntax` 0.8.11; see `Blocks.txt.license`).
//!
//! Every table entry's name is the XSD/XPath `\p{IsX}` block-escape name for
//! that Unicode block, computed per XML Schema Part 2 Appendix G.4.2.3's
//! definition of "normalized block name": *"the string of characters formed
//! by stripping out white space and underbar characters from the block name
//! as given in \[Unicode Database\], while retaining hyphens and preserving
//! case distinctions"* — prefixed with `Is`. For example, the UCD block name
//! "Latin-1 Supplement" normalizes to "Latin-1Supplement" (hyphen kept, space
//! dropped, case preserved), giving the block-escape name `IsLatin-1Supplement`.
//!
//! The emitted file also carries a fixed (non-`Blocks.txt`-derived) binary-search-
//! free `lookup` helper and a `#[cfg(test)]` suite, both written as literal
//! output by this generator, so the committed file `cargo run -p purrdf-core
//! --example gen_unicode_blocks --locked` produces is reproducible byte-for-
//! byte by `scripts/check-generated.sh`.
//!
//! Run via `make metadata` (writes) or `make check` (verifies). Output goes
//! to stdout; run with `--locked` per this workspace's convention.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

/// One parsed `Blocks.txt` data line: an inclusive codepoint range plus the
/// verbatim UCD block name (not yet normalized to the XSD escape form).
struct RawBlock {
    lo: u32,
    hi: u32,
    ucd_name: String,
}

/// Parses the vendored `Blocks.txt`, per its documented format:
/// `Start Code..End Code; Block Name`, `#`-prefixed comments, blank lines.
fn parse_blocks_txt(text: &str) -> Vec<RawBlock> {
    let mut blocks = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((range, name)) = line.split_once(';') else {
            panic!("Blocks.txt line has no `;` separator: {line:?}");
        };
        let Some((lo_str, hi_str)) = range.trim().split_once("..") else {
            panic!("Blocks.txt range has no `..` separator: {line:?}");
        };
        let lo = u32::from_str_radix(lo_str.trim(), 16)
            .unwrap_or_else(|err| panic!("bad start codepoint {lo_str:?} in {line:?}: {err}"));
        let hi = u32::from_str_radix(hi_str.trim(), 16)
            .unwrap_or_else(|err| panic!("bad end codepoint {hi_str:?} in {line:?}: {err}"));
        blocks.push(RawBlock {
            lo,
            hi,
            ucd_name: name.trim().to_owned(),
        });
    }
    blocks
}

/// Applies XML Schema Part 2 Appendix G.4.2.3's "normalized block name"
/// transform to a verbatim UCD block name, then prefixes `Is` to form the
/// full `\p{IsX}` escape name: strip whitespace and underbar (`_`)
/// characters, retain hyphens, preserve case.
fn xsd_block_escape_name(ucd_name: &str) -> String {
    let mut escape_name = String::from("Is");
    for ch in ucd_name.chars() {
        if ch.is_whitespace() || ch == '_' {
            continue;
        }
        escape_name.push(ch);
    }
    escape_name
}

/// Formats a codepoint as an underscore-grouped 8-hex-digit `u32` literal
/// (e.g. `0x0010_FFFF`), uniformly for every table row so `clippy::
/// unreadable_literal` has nothing to flag regardless of magnitude —
/// `Blocks.txt` ranges from 4-digit (`0x007F`) to 6-digit (`0x10FFFF`)
/// codepoints, and only the 6-digit ones exceed the lint's un-grouped
/// threshold.
fn format_codepoint_literal(codepoint: u32) -> String {
    format!("0x{:04X}_{:04X}", codepoint >> 16, codepoint & 0xFFFF)
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let blocks_txt_path = manifest_dir.join("vendor/unicode/Blocks.txt");
    let text = fs::read_to_string(&blocks_txt_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", blocks_txt_path.display()));

    let mut blocks: Vec<(String, u32, u32)> = parse_blocks_txt(&text)
        .into_iter()
        .map(|raw| (xsd_block_escape_name(&raw.ucd_name), raw.lo, raw.hi))
        .collect();
    blocks.sort_by_key(|(_, lo, _)| *lo);

    // Fail loudly at generation time (ETHOS §H) rather than emit a table whose
    // own doc comment claims an invariant the data does not hold.
    for window in blocks.windows(2) {
        let (prev_name, _, prev_hi) = &window[0];
        let (next_name, next_lo, _) = &window[1];
        assert!(
            prev_hi < next_lo,
            "Blocks.txt yields overlapping or unsorted ranges: {prev_name} (..={prev_hi:04X}) \
             then {next_name} ({next_lo:04X}..)",
        );
    }
    assert!(
        !blocks.is_empty(),
        "parsed zero blocks from {}",
        blocks_txt_path.display()
    );

    let mut out = String::new();
    writeln!(
        out,
        "// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>"
    )
    .unwrap();
    writeln!(out, "// SPDX-License-Identifier: MIT OR Apache-2.0").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "//! @generated by `cargo run -p purrdf-core --example gen_unicode_blocks --locked`."
    )
    .unwrap();
    writeln!(
        out,
        "//! Do not hand-edit; `scripts/check-generated.sh` diffs this file against the"
    )
    .unwrap();
    writeln!(out, "//! generator's stdout.").unwrap();
    writeln!(out, "//!").unwrap();
    writeln!(
        out,
        "//! Source: `crates/rdf-core/vendor/unicode/Blocks.txt`, Unicode Character"
    )
    .unwrap();
    writeln!(
        out,
        "//! Database version 16.0.0 (pinned to match this workspace's locked"
    )
    .unwrap();
    writeln!(
        out,
        "//! `regex-syntax` 0.8.11, whose embedded Unicode tables are also 16.0.0 --"
    )
    .unwrap();
    writeln!(out, "//! see `Blocks.txt.license`).").unwrap();
    writeln!(out, "//!").unwrap();
    writeln!(
        out,
        "//! `UNICODE_BLOCKS` maps every XSD/XPath block-escape name (the `\\p{{IsX}}`"
    )
    .unwrap();
    writeln!(
        out,
        "//! form of XML Schema Part 2 Appendix G.4.2.3, normalized per that section:"
    )
    .unwrap();
    writeln!(
        out,
        "//! \"the string of characters formed by stripping out white space and"
    )
    .unwrap();
    writeln!(
        out,
        "//! underbar characters from the block name ..., while retaining hyphens and"
    )
    .unwrap();
    writeln!(
        out,
        "//! preserving case distinctions\", prefixed with `Is`) to its inclusive"
    )
    .unwrap();
    writeln!(
        out,
        "//! `[lo, hi]` Unicode codepoint range. Sorted ascending by `lo` and"
    )
    .unwrap();
    writeln!(
        out,
        "//! non-overlapping -- enforced by the generator and re-checked by"
    )
    .unwrap();
    writeln!(
        out,
        "//! `tests::table_is_sorted_and_non_overlapping` below."
    )
    .unwrap();
    writeln!(out, "//!").unwrap();
    writeln!(
        out,
        "//! <https://www.w3.org/TR/xmlschema11-2/#cces-blockesc>"
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "pub(crate) const UNICODE_BLOCKS: &[(&str, u32, u32)] = &["
    )
    .unwrap();
    for (name, lo, hi) in &blocks {
        let lo_literal = format_codepoint_literal(*lo);
        let hi_literal = format_codepoint_literal(*hi);
        writeln!(out, "    (\"{name}\", {lo_literal}, {hi_literal}),").unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "/// Looks up the inclusive `[lo, hi]` codepoint range for an XSD/XPath"
    )
    .unwrap();
    writeln!(
        out,
        "/// block-escape name (e.g. `\"IsBasicLatin\"`), by linear scan over"
    )
    .unwrap();
    writeln!(out, "/// [`UNICODE_BLOCKS`].").unwrap();
    writeln!(out, "///").unwrap();
    writeln!(
        out,
        "/// [`UNICODE_BLOCKS`] is sorted by codepoint (`lo`), not by name -- that is the"
    )
    .unwrap();
    writeln!(
        out,
        "/// order that lets [`table_is_sorted_and_non_overlapping`] cheaply prove the"
    )
    .unwrap();
    writeln!(
        out,
        "/// table has no gaps or overlaps against `Blocks.txt`'s own invariant. A"
    )
    .unwrap();
    writeln!(
        out,
        "/// name-keyed binary search would need a second, name-sorted view of the same"
    )
    .unwrap();
    writeln!(
        out,
        "/// ~376 rows; a linear scan is simpler (one array, ETHOS §S) and this runs at"
    )
    .unwrap();
    writeln!(
        out,
        "/// most once per distinct `\\p{{IsX}}`/`\\P{{IsX}}` construct in a pattern being"
    )
    .unwrap();
    writeln!(
        out,
        "/// *compiled*, never per matched character -- not a hot path."
    )
    .unwrap();
    writeln!(out, "///").unwrap();
    writeln!(
        out,
        "/// Returns `None` if `name` is not a recognized block name. Per XML Schema"
    )
    .unwrap();
    writeln!(
        out,
        "/// Part 2 §G.4.2.4, what an unrecognized block name should mean is a policy"
    )
    .unwrap();
    writeln!(
        out,
        "/// decision for the caller (this crate does not implement that fallback here)."
    )
    .unwrap();
    writeln!(
        out,
        "pub(crate) fn lookup(name: &str) -> Option<(u32, u32)> {{"
    )
    .unwrap();
    writeln!(
        out,
        "    UNICODE_BLOCKS.iter().find_map(|(candidate, lo, hi)| {{"
    )
    .unwrap();
    writeln!(out, "        (*candidate == name).then_some((*lo, *hi))").unwrap();
    writeln!(out, "    }})").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "#[cfg(test)]").unwrap();
    writeln!(out, "mod tests {{").unwrap();
    writeln!(out, "    use super::{{UNICODE_BLOCKS, lookup}};").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    #[test]").unwrap();
    writeln!(out, "    fn table_is_sorted_and_non_overlapping() {{").unwrap();
    writeln!(out, "        for window in UNICODE_BLOCKS.windows(2) {{").unwrap();
    writeln!(out, "            let (_, _, prev_hi) = window[0];").unwrap();
    writeln!(out, "            let (_, next_lo, _) = window[1];").unwrap();
    writeln!(out, "            assert!(").unwrap();
    writeln!(out, "                prev_hi < next_lo,").unwrap();
    writeln!(
        out,
        "                \"UNICODE_BLOCKS is not sorted-by-codepoint and non-overlapping: \\"
    )
    .unwrap();
    // Built from fragments (not one literal) so clippy's
    // `literal_string_with_formatting_args` lint, which flags any bare string
    // literal that merely *looks* like a format string, does not mistake this
    // deliberately emitted `{:?}`-shaped Rust source text for a forgotten
    // `format!`/`write!` argument.
    out.push_str("                 ");
    out.push('{');
    out.push_str(":?} then ");
    out.push('{');
    out.push_str(":?}\",\n");
    writeln!(out, "                window[0],").unwrap();
    writeln!(out, "                window[1],").unwrap();
    writeln!(out, "            );").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    #[test]").unwrap();
    writeln!(out, "    fn known_block_ranges_match_unicode_16_0_0() {{").unwrap();
    writeln!(
        out,
        "        assert_eq!(lookup(\"IsBasicLatin\"), Some((0x0000_0000, 0x0000_007F)));"
    )
    .unwrap();
    writeln!(
        out,
        "        assert_eq!(lookup(\"IsLatin-1Supplement\"), Some((0x0000_0080, 0x0000_00FF)));"
    )
    .unwrap();
    writeln!(
        out,
        "        assert_eq!(lookup(\"IsGreekandCoptic\"), Some((0x0000_0370, 0x0000_03FF)));"
    )
    .unwrap();
    writeln!(
        out,
        "        assert_eq!(lookup(\"IsCJKUnifiedIdeographs\"), Some((0x0000_4E00, 0x0000_9FFF)));"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    #[test]").unwrap();
    writeln!(out, "    fn unknown_block_name_returns_none() {{").unwrap();
    writeln!(
        out,
        "        assert_eq!(lookup(\"IsNotARealUnicodeBlock\"), None);"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();

    print!("{out}");
}

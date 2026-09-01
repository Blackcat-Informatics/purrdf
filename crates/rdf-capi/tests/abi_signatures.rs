// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The exported-prototype freeze: every `extern "C"` signature libpurrdf ships,
//! snapshotted against the ABI version triple that describes it.
//!
//! # Why this test exists
//!
//! `make capi-check` proves the committed `include/purrdf.h` **agrees with** the
//! Rust source. It cannot see that the contract *moved*: regenerate the header
//! after retyping a parameter and the two agree perfectly while every host
//! compiled against the previous header is now calling a different function.
//! `purrdf_abi_version` keeps reporting the old triple, so no runtime guard can
//! detect it either — the caller pushes its arguments into the wrong slots and
//! the boundary dereferences them.
//!
//! That is exactly what happened when `shapes_base_iri` was inserted into the
//! middle of `purrdf_shacl_validate_to_sarif` and `purrdf_shacl_entail_to_ntriples`
//! with the version left at `0.6.0`: a `0.6.0` host passed its `data_nt` string
//! into the `shapes_base_iri` slot and its `PurrdfBuffer **` out-pointer into
//! `data_nt`, which `cstr_to_str` read as a NUL-terminated C string.
//!
//! So the snapshot binds the two facts that must move together — the prototype
//! list and the version triple — into one file. Editing a signature makes this
//! test fail; the only way to make it pass is to write the new prototype list
//! **and** the version that describes it, which is the deliberate decision the
//! ABI contract requires.
//!
//! # What is snapshotted
//!
//! The prototypes are read out of the committed `include/purrdf.h` — the
//! generated artifact that *is* the ABI contract, and the exact text a C
//! consumer compiles against. Each entry is the canonicalized return type,
//! function name, and full parameter list; entries are sorted by name so a
//! harmless source reordering is not a diff, while any name, type, order, or
//! arity change is.

use std::collections::BTreeSet;

use purrdf::version::{PURRDF_ABI_MAJOR, PURRDF_ABI_MINOR, PURRDF_ABI_PATCH};

/// The committed ABI contract, compiled into the test so the snapshot can never
/// be compared against a header the build did not ship.
const HEADER: &str = include_str!("../include/purrdf.h");

/// The frozen prototype list. Blank lines and `#`-prefixed lines are commentary;
/// the first meaningful line is `abi-version <major>.<minor>.<patch>` and every
/// line after it is one canonicalized exported prototype.
const SNAPSHOT: &str = include_str!("abi_signatures.snapshot");

/// The remediation text every failure in this file ends with. It names the
/// decision, not just the mismatch.
const WHAT_TO_DO: &str = "\
WHAT TO DO
  1. Decide whether this is an INCOMPATIBLE change. It is, if a host compiled
     against the previous header would mis-execute: a removed or renamed
     function, a retyped/reordered parameter, a parameter inserted anywhere but
     the end, or a changed return type. Adding a whole new function is NOT
     incompatible.
  2. If it is incompatible, bump `PURRDF_ABI_MINOR` in `crates/rdf-capi/src/version.rs`
     and reset `PURRDF_ABI_PATCH` to 0. Pre-1.0, breaking changes ride the MINOR
     component (`docs/book/src/project/releases.md`, \"Pre-1.0 semver policy\");
     MAJOR stays 0. Record the break in that constant's doc comment and in the
     crate README so a C consumer learns why their build broke.
     Do NOT add a `_v2` entry point: two exported functions for one job is the
     duplication this library exists to delete.
  3. Regenerate the header: `make capi-header` (never hand-edit `include/purrdf.h`).
  4. Rewrite `crates/rdf-capi/tests/abi_signatures.snapshot` with the ACTUAL
     block printed above, verbatim.
  5. Re-run: `cargo test -p purrdf-capi --test abi_signatures && make capi-check`.";

/// Remove C block and line comments, so a prototype-shaped sentence inside a
/// doc comment can never be mistaken for a declaration.
fn strip_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut index = 0;
    let mut in_block = false;
    while index < bytes.len() {
        if in_block {
            if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_block = false;
                index += 2;
            } else {
                // Keep newlines so line-oriented filtering below stays aligned.
                if bytes[index] == b'\n' {
                    out.push('\n');
                }
                index += 1;
            }
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_block = true;
            index += 2;
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
        } else {
            // The header is ASCII outside comments, so byte indexing is safe here.
            out.push(char::from(bytes[index]));
            index += 1;
        }
    }
    assert!(!in_block, "unterminated block comment in include/purrdf.h");
    out
}

/// The trailing identifier of `text`, or `""` when it does not end in one.
fn trailing_identifier(text: &str) -> &str {
    let start = text
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or(0, |index| index + 1);
    &text[start..]
}

/// Every exported `purrdf_*` prototype in the committed header, canonicalized to
/// `<return type> <name>(<parameters>)` on one line and sorted by that text.
fn exported_prototypes() -> BTreeSet<String> {
    let stripped = strip_comments(HEADER);
    // Drop preprocessor lines and the `cpp_compat` `extern "C" { ... }` wrapper,
    // so what remains splits cleanly on `;` into declarations.
    let declarations: String = stripped
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty() && !line.starts_with('#') && *line != "extern \"C\" {" && *line != "}"
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prototypes: BTreeSet<String> = declarations
        .split(';')
        .filter(|chunk| !chunk.contains('{') && !chunk.contains('}'))
        .map(|chunk| chunk.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|chunk| {
            chunk.find('(').is_some_and(|open| {
                trailing_identifier(chunk[..open].trim_end()).starts_with("purrdf_")
            })
        })
        .collect();

    assert!(
        !prototypes.is_empty(),
        "no exported prototypes were recovered from include/purrdf.h — the \
         extractor in {} no longer matches the header cbindgen emits, so this \
         freeze is not protecting anything. Fix the extractor before touching \
         the snapshot.",
        file!()
    );
    prototypes
}

/// The version triple the Rust source reports, rendered the way the snapshot
/// records it.
fn source_version() -> String {
    format!("{PURRDF_ABI_MAJOR}.{PURRDF_ABI_MINOR}.{PURRDF_ABI_PATCH}")
}

/// Render the snapshot body the source currently implies, ready to be pasted
/// into `abi_signatures.snapshot` verbatim.
fn rendered_snapshot() -> String {
    let mut body = format!("abi-version {}\n", source_version());
    for prototype in exported_prototypes() {
        body.push_str(prototype.as_str());
        body.push('\n');
    }
    body
}

/// The snapshot file's meaningful lines: commentary and blank lines removed.
fn snapshot_lines() -> Vec<&'static str> {
    SNAPSHOT
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// The header's own `#define PURRDF_ABI_<part>` value.
fn header_define(part: &str) -> u32 {
    let needle = format!("#define PURRDF_ABI_{part} ");
    let line = HEADER
        .lines()
        .find(|line| line.starts_with(&needle))
        .unwrap_or_else(|| panic!("include/purrdf.h declares no {needle}… macro"));
    line[needle.len()..]
        .trim()
        .parse()
        .unwrap_or_else(|error| panic!("malformed `{needle}` macro: {error}"))
}

/// The committed header must report the same triple the Rust source does. A
/// header regenerated from a bumped source but left uncommitted (or a source
/// bumped without regenerating) would otherwise ship a library whose
/// `purrdf_abi_version` disagrees with the contract its consumers compiled
/// against. `make capi-check` catches this too, but it is not part of
/// `make check` — this assertion is.
#[test]
fn committed_header_reports_the_source_abi_version() {
    assert_eq!(
        (
            header_define("MAJOR"),
            header_define("MINOR"),
            header_define("PATCH")
        ),
        (PURRDF_ABI_MAJOR, PURRDF_ABI_MINOR, PURRDF_ABI_PATCH),
        "include/purrdf.h reports a different ABI version than \
         crates/rdf-capi/src/version.rs.\n\n{WHAT_TO_DO}"
    );
}

/// The snapshot records the version triple the exported surface below it
/// belongs to. Bumping the version without re-freezing the prototypes (or
/// re-freezing without bumping) fails here.
#[test]
fn snapshot_records_the_current_abi_version() {
    let recorded = snapshot_lines()
        .first()
        .copied()
        .and_then(|line| line.strip_prefix("abi-version "))
        .map(str::to_owned)
        .expect(
            "abi_signatures.snapshot must begin (after commentary) with a line \
             `abi-version <major>.<minor>.<patch>`",
        );
    assert_eq!(
        recorded,
        source_version(),
        "the exported-prototype snapshot is frozen at ABI {recorded}, but \
         crates/rdf-capi/src/version.rs reports {}.\n\n{WHAT_TO_DO}",
        source_version()
    );
}

/// **The freeze.** Every exported `extern "C"` signature in the committed header
/// must match the snapshot exactly — name, parameter types, parameter order, and
/// return type. Changing any of them fails here until the author either restores
/// the signature or deliberately re-declares the ABI.
#[test]
fn exported_signatures_match_the_frozen_snapshot() {
    let actual = exported_prototypes();
    let expected: BTreeSet<String> = snapshot_lines()
        .into_iter()
        .skip(1)
        .map(str::to_owned)
        .collect();

    if actual == expected {
        return;
    }

    let removed: Vec<&String> = expected.difference(&actual).collect();
    let added: Vec<&String> = actual.difference(&expected).collect();
    let mut report = String::from(
        "the exported C-ABI prototype list changed.\n\n\
         GONE from the frozen ABI (a host built against the committed header \
         calls these and they no longer exist as declared):\n",
    );
    if removed.is_empty() {
        report.push_str("  (none)\n");
    } else {
        for prototype in removed {
            report.push_str("  - ");
            report.push_str(prototype);
            report.push('\n');
        }
    }
    report.push_str("\nNEW in the built library:\n");
    if added.is_empty() {
        report.push_str("  (none)\n");
    } else {
        for prototype in added {
            report.push_str("  + ");
            report.push_str(prototype);
            report.push('\n');
        }
    }
    report.push_str(
        "\nACTUAL snapshot body (paste this below the commentary in \
         crates/rdf-capi/tests/abi_signatures.snapshot):\n\n",
    );
    report.push_str(&rendered_snapshot());
    report.push('\n');
    report.push_str(WHAT_TO_DO);
    panic!("{report}");
}

/// The exported prototype named `name`, or a panic naming the missing symbol.
fn prototype_of(name: &str) -> String {
    exported_prototypes()
        .into_iter()
        .find(|candidate| {
            candidate
                .find('(')
                .is_some_and(|open| trailing_identifier(candidate[..open].trim_end()) == name)
        })
        .unwrap_or_else(|| panic!("{name} is no longer exported"))
}

/// Every signature whose change forced `0.6.0` → `0.7.0`, pinned by name and full
/// parameter list so the breaks that motivated this freeze cannot silently revert.
///
/// The snapshot above already fails on any of these, but it fails on *every* other
/// prototype too, and a wholesale re-freeze would carry a revert through unnoticed.
/// These four are spelled out so a reverted break is named at the point of failure.
#[test]
fn the_signatures_the_minor_bump_paid_for_are_the_ones_that_shipped() {
    let expected: [(&str, String); 4] = [
        (
            "purrdf_shacl_validate_to_sarif",
            "int32_t purrdf_shacl_validate_to_sarif(const char *shapes_ttl, \
             const char *shapes_base_iri, const char *data_nt, PurrdfBuffer **out_buffer, \
             PurrdfError **out_error)"
                .to_owned(),
        ),
        (
            "purrdf_shacl_entail_to_ntriples",
            "int32_t purrdf_shacl_entail_to_ntriples(const char *shapes_ttl, \
             const char *shapes_base_iri, const char *data_nt, PurrdfBuffer **out_buffer, \
             PurrdfError **out_error)"
                .to_owned(),
        ),
        (
            "purrdf_serialize",
            "int32_t purrdf_serialize(const PurrdfDataset *dataset, const char *media_type, \
             const char *base_iri, PurrdfBuffer **out_buffer, size_t *out_statement_rows_dropped, \
             size_t *out_directional_literals_dropped, size_t *out_named_graph_rows_dropped, \
             PurrdfError **out_error)"
                .to_owned(),
        ),
        (
            "purrdf_serialize_jsonld_configured",
            "int32_t purrdf_serialize_jsonld_configured(const PurrdfDataset *dataset, \
             const char *media_type, const char *base_iri, const uint8_t *options_json, \
             size_t options_len, const PurrdfJsonLdContext *context, \
             const char *yaml_schema_url, PurrdfBuffer **out_buffer, PurrdfError **out_error)"
                .to_owned(),
        ),
    ];
    for (name, wanted) in expected {
        assert_eq!(
            prototype_of(name),
            wanted,
            "{name} no longer has the parameter list ABI {} declares.\n\n{WHAT_TO_DO}",
            source_version()
        );
    }
}

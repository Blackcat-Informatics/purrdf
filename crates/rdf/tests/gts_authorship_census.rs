// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GTS-AUTHORSHIP CENSUS — the closed list of places that can mint a GTS file.
//!
//! Every authoring decision this branch introduced (which in-band dictionaries a
//! pack pins, which one primes which frame, the declared `zstd` level) is
//! carried in [`WriterOptions`] and chosen at ONE moment: when a
//! header-minting `Writer` constructor runs. A new call site added later that
//! forgets to state a plan does not fail — it silently emits a pack at the
//! writer's ~level-1 default with no dictionary, which is exactly the kind of
//! quiet capability degradation the plan exists to prevent.
//!
//! So this gate is a STRUCTURAL source scan over `crates/*/src` and
//! `bindings/*/src`, in two rules:
//!
//!   RULE 1 — `Writer`'s public constructor set is exactly
//!            `{new, deterministic, with_layout, with_options}` (header-MINTING)
//!            plus `{appending}` (segment-CONTINUING, mints no header and so
//!            makes no authoring choices — it adopts the ones already on the
//!            wire). A fifth minting constructor must be added here on purpose.
//!
//!   RULE 2 — the set of `(file, enclosing fn)` production call sites of those
//!            four minting constructors is exactly [`AUTHORSHIP_SITES`]. This is
//!            the census proper: the complete list of functions in this
//!            repository that can bring a GTS segment header into existence.
//!
//! The `tests` module below proves the detector is not vacuous: a synthetic
//! source carrying a call site MUST be flagged, and commented-out or
//! `#[cfg(test)]`-scoped calls MUST NOT be.

use std::path::{Path, PathBuf};

/// The `Writer` constructors that MINT a new segment header, and therefore
/// decide the pack's `WriterOptions` (dictionaries, declared level, layout).
const MINTING_CONSTRUCTORS: [&str; 4] = ["new", "deterministic", "with_layout", "with_options"];

/// The `Writer` constructors that do NOT mint a header. `appending` continues an
/// existing segment and adopts the catalog/dictionaries already on the wire, so
/// it makes no authoring choice and is deliberately outside the census.
const CONTINUING_CONSTRUCTORS: [&str; 1] = ["appending"];

/// THE CENSUS: every production `(file, enclosing fn)` that mints a GTS header.
///
/// Adding a row here is a deliberate act — the reviewer's question for any new
/// row is "does this function state its dictionaries and its zstd level, or does
/// it silently take the writer defaults?".
///
/// The three general-purpose PUBLIC authoring facades a consumer is expected to
/// reach for are `purrdf_rdf::gts_write::{to_writer, to_gts}` (RDF dataset →
/// GTS; `to_gts` delegates to `to_writer`, so only the latter is a direct site)
/// and `purrdf_gts::compact::compact_streamable` (§10.1 repack). The remaining
/// rows are narrower producers — a bundle emitter, an archive packer, an example
/// store, and the frozen-vector fixtures — each of which is still a way to mint
/// a header and so still belongs in the census.
const AUTHORSHIP_SITES: [(&str, &str); 10] = [
    // §10.1 streamable compaction: authors a pack under a `DictPlan`.
    ("crates/gts/src/compact.rs", "compact_streamable"),
    // The append-only agent-memory example store (mints the header once, then
    // continues that segment through `Writer::appending`).
    ("crates/gts/src/examples/agent_memory.rs", "writer"),
    // The `files` profile archive packers.
    ("crates/gts/src/files.rs", "pack_to_writer"),
    ("crates/gts/src/files.rs", "build_entries_v2_prefix"),
    // The single-shot snapshot-bundle authoring helper.
    ("crates/gts/src/writer.rs", "snapshot_from_graph"),
    // The `dist` snapshot-bundle emitter, driven by a `MediumPlan`.
    ("crates/rdf/src/gts_compose.rs", "emit_gts"),
    // The frozen dict-vector fixtures (`vectors/30`–`33`).
    ("crates/rdf/src/gts_dict_vectors.rs", "fixed_source"),
    (
        "crates/rdf/src/gts_dict_vectors.rs",
        "size_comparison_source",
    ),
    ("crates/rdf/src/gts_dict_vectors.rs", "multi_dict_pack"),
    // The public RDF-dataset → GTS surface (`to_gts` delegates to this).
    ("crates/rdf/src/gts_write.rs", "to_writer"),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// Every `.rs` file under `crates/*/src` and `bindings/*/src`.
fn production_sources(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for group in ["crates", "bindings"] {
        let group_dir = root.join(group);
        let entries = std::fs::read_dir(&group_dir)
            .unwrap_or_else(|err| panic!("read {}: {err}", group_dir.display()));
        for entry in entries {
            let crate_dir = entry.expect("directory entry").path();
            let src = crate_dir.join("src");
            if src.is_dir() {
                collect_rs(&src, &mut out);
            }
        }
    }
    out.sort();
    assert!(
        out.len() > 50,
        "the scan found only {} sources — the walker is broken, not the repo",
        out.len()
    );
    out
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Blank out `//`-to-end-of-line comments, preserving line structure.
///
/// This is what keeps the doc examples in `lib.rs`/`reader.rs` — which do call
/// `Writer::new` — out of the census: they are comments, not call sites. A `//`
/// inside a string literal (`"https://…"`) is NOT a comment, so the scan is
/// quote-aware; treating one as a comment would silently truncate real code.
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line[..comment_start(line)].to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The byte offset where `line`'s comment begins, or `line.len()`.
fn comment_start(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        let ch = bytes[index];
        if escaped {
            escaped = false;
        } else if ch == b'\\' && in_string {
            escaped = true;
        } else if ch == b'"' {
            in_string = !in_string;
        } else if !in_string && ch == b'/' && bytes.get(index + 1) == Some(&b'/') {
            return index;
        }
        index += 1;
    }
    line.len()
}

/// Blank out every `#[cfg(test)] mod … { … }` body, preserving line structure.
/// Test modules author GTS constantly and are not authorship surface.
///
/// The source is rustfmt-normalised, so a module opened by an attribute at
/// indentation `n` is closed by the first later line that is exactly `}` at
/// indentation `n`. That is exact and, unlike brace counting over
/// comment-stripped text, cannot be thrown off by a brace inside a string
/// literal. `#[cfg(test)]` on a non-`mod` item is deliberately NOT stripped: a
/// test-only helper that mints a header is still a call site a reviewer should
/// see.
///
/// Panics when a test module is never closed — this gate must fail LOUDLY rather
/// than silently blind itself to the rest of the file.
fn strip_test_module(path: &Path, stripped: &str) -> String {
    let mut lines: Vec<String> = stripped.lines().map(str::to_string).collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].clone();
        if line.trim_start() != "#[cfg(test)]" {
            index += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let opens_module = lines[index + 1..]
            .iter()
            .find(|next| !next.trim().is_empty())
            .is_some_and(|next| {
                let body = next.trim_start();
                body.starts_with("mod ") || body.starts_with("pub mod ")
            });
        if !opens_module {
            index += 1;
            continue;
        }
        let closer = format!("{}}}", " ".repeat(indent));
        let end = match lines[index..]
            .iter()
            .position(|candidate| candidate.trim_end() == closer)
        {
            Some(offset) => index + offset,
            None => panic!(
                "{}: the #[cfg(test)] module opened at line {} is never closed by a \
                 `}}` at its own indentation — the census scanner cannot safely exclude it",
                path.display(),
                index + 1
            ),
        };
        for line in &mut lines[index..=end] {
            line.clear();
        }
        index = end + 1;
    }
    lines.join("\n")
}

/// The name declared by a `fn NAME(` item on `line`, if any.
fn declared_fn(line: &str) -> Option<String> {
    let at = line.find("fn ")?;
    // `fn` must start a token: preceded by nothing, whitespace, or `(` (for a
    // `dyn Fn`-free world this is enough; `Fn(` is capitalised and so excluded).
    if at > 0 && !matches!(line.as_bytes()[at - 1], b' ' | b'\t') {
        return None;
    }
    let rest = &line[at + 3..];
    let name: String = rest
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Whether `line` calls one of the header-minting `Writer` constructors.
fn mints_a_header(line: &str) -> bool {
    MINTING_CONSTRUCTORS
        .iter()
        .any(|ctor| calls_writer_ctor(line, ctor))
}

/// Whether `haystack` names `Writer::<ctor>(` as a WHOLE type name.
///
/// `contains` alone is wrong: `OkfWriter::new(` ends in `Writer::new(` and is a
/// different type entirely. A leading `::` or module path is fine
/// (`purrdf_gts::writer::Writer::new(`), a leading identifier character is not.
fn calls_writer_ctor(haystack: &str, ctor: &str) -> bool {
    let needle = format!("Writer::{ctor}(");
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(&needle) {
        let at = from + offset;
        let preceded_by_ident = at > 0
            && haystack[..at]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
        if !preceded_by_ident {
            return true;
        }
        from = at + 1;
    }
    false
}

/// Extract the `(enclosing fn, line)` census sites from one already-stripped
/// source body.
fn sites_in(body: &str) -> Vec<String> {
    let mut enclosing: Option<String> = None;
    let mut out = Vec::new();
    for line in body.lines() {
        if let Some(name) = declared_fn(line) {
            enclosing = Some(name);
        }
        if mints_a_header(line) {
            out.push(
                enclosing
                    .clone()
                    .unwrap_or_else(|| "<no enclosing fn>".to_string()),
            );
        }
    }
    out
}

/// RULE 1 — the `Writer` constructor set is closed and classified.
#[test]
fn the_writer_constructor_set_is_exactly_the_pinned_minting_and_continuing_sets() {
    let root = repo_root();
    let source = std::fs::read_to_string(root.join("crates/gts/src/writer.rs")).expect("writer.rs");
    let body = strip_line_comments(&source);
    let body = strip_test_module(Path::new("crates/gts/src/writer.rs"), &body);

    // A constructor is a `pub fn` whose return type names `Self`.
    let mut found: Vec<String> = body
        .lines()
        .filter(|line| line.contains("pub fn ") && line.contains("Self"))
        .filter_map(declared_fn)
        .collect();
    found.sort();
    found.dedup();

    let mut expected: Vec<String> = MINTING_CONSTRUCTORS
        .iter()
        .chain(CONTINUING_CONSTRUCTORS.iter())
        .map(|name| (*name).to_string())
        .collect();
    expected.sort();

    assert_eq!(
        found, expected,
        "purrdf_gts::writer::Writer's constructor set changed. A new HEADER-MINTING \
         constructor must also be classified in MINTING_CONSTRUCTORS and every one of \
         its call sites added to AUTHORSHIP_SITES; a new CONTINUING constructor \
         (mints no header) belongs in CONTINUING_CONSTRUCTORS."
    );
}

/// RULE 2 — the census of production header-minting call sites is closed.
#[test]
fn every_production_gts_authorship_site_is_in_the_census() {
    let root = repo_root();
    let mut observed: Vec<(String, String)> = Vec::new();
    for path in production_sources(&root) {
        let relative = path
            .strip_prefix(&root)
            .expect("scanned paths live under the root")
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        if !MINTING_CONSTRUCTORS
            .iter()
            .any(|ctor| source.contains(&format!("Writer::{ctor}(")))
        {
            continue;
        }
        let body = strip_line_comments(&source);
        let body = strip_test_module(&path, &body);
        for enclosing in sites_in(&body) {
            observed.push((relative.clone(), enclosing));
        }
    }
    observed.sort();
    observed.dedup();

    let mut expected: Vec<(String, String)> = AUTHORSHIP_SITES
        .iter()
        .map(|(file, function)| ((*file).to_string(), (*function).to_string()))
        .collect();
    expected.sort();
    let mut deduped = expected.clone();
    deduped.dedup();
    assert_eq!(
        deduped.len(),
        AUTHORSHIP_SITES.len(),
        "AUTHORSHIP_SITES carries a duplicate row"
    );

    assert_eq!(
        observed, expected,
        "the GTS-authorship census changed. Every function that mints a GTS segment \
         header must state its in-band dictionaries and its declared zstd level, or it \
         silently emits an undicted pack at the writer's default level. Add the new row \
         to AUTHORSHIP_SITES only after checking that."
    );
}

/// The three general-purpose PUBLIC authoring facades are present under their
/// pinned names — a rename or removal must be a conscious edit here.
#[test]
fn the_public_authoring_facades_are_present_and_named() {
    let root = repo_root();
    let census: Vec<(&str, &str)> = AUTHORSHIP_SITES.to_vec();
    assert!(
        census.contains(&("crates/rdf/src/gts_write.rs", "to_writer")),
        "gts_write::to_writer must be in the census"
    );
    assert!(
        census.contains(&("crates/gts/src/compact.rs", "compact_streamable")),
        "compact::compact_streamable must be in the census"
    );
    // `to_gts` is a public facade that authors a whole GTS file but mints its
    // header THROUGH `to_writer`, so it is not a direct census site. Pin that
    // delegation, or the census would silently lose a public entry point if
    // `to_gts` ever grew its own `Writer`.
    let gts_write =
        std::fs::read_to_string(root.join("crates/rdf/src/gts_write.rs")).expect("gts_write.rs");
    let body = strip_line_comments(&gts_write);
    let body = strip_test_module(Path::new("crates/rdf/src/gts_write.rs"), &body);
    assert!(
        body.contains("pub fn to_gts("),
        "gts_write::to_gts must remain a public authoring facade"
    );
    let to_gts = body.split("pub fn to_gts(").nth(1).expect("to_gts body");
    assert!(
        to_gts.contains("to_writer("),
        "to_gts must author THROUGH to_writer; if it mints its own Writer it becomes a \
         census site and must be added to AUTHORSHIP_SITES"
    );
}

mod detector_self_tests {
    use super::*;

    const SYNTHETIC: &str = r#"
/// A doc example: let w = Writer::new("doc");
// let w = Writer::with_options("commented", opts);
pub fn authors_a_pack() -> Vec<u8> {
    let mut w = Writer::new("synthetic");
    w.into_bytes()
}

fn helper() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_may_author_freely() {
        let _ = Writer::new("test-only");
        let _ = Writer::with_options("test-only", Default::default());
    }
}
"#;

    /// The detector FINDS a real production call site, attributed to its fn.
    #[test]
    fn a_production_call_site_is_flagged() {
        let body = strip_line_comments(SYNTHETIC);
        let body = strip_test_module(Path::new("synthetic.rs"), &body);
        assert_eq!(sites_in(&body), vec!["authors_a_pack".to_string()]);
    }

    /// …and a commented-out one is NOT — otherwise the doc examples in
    /// `lib.rs`/`reader.rs` would pollute the census.
    #[test]
    fn commented_out_call_sites_are_not_flagged() {
        let body = strip_line_comments("// let w = Writer::new(\"x\");\n/// Writer::new(\"y\")\n");
        assert!(sites_in(&body).is_empty());
    }

    /// …and neither is one inside the trailing `#[cfg(test)]` module.
    #[test]
    fn test_module_call_sites_are_not_flagged() {
        let body = strip_line_comments(SYNTHETIC);
        let full = sites_in(&body);
        assert_eq!(
            full.len(),
            3,
            "before excluding the test module the scanner sees all three sites"
        );
        let trimmed = strip_test_module(Path::new("synthetic.rs"), &body);
        assert_eq!(sites_in(&trimmed).len(), 1);
    }

    /// The scanner refuses to run blind: an unclosed test module is a loud
    /// failure, never a silently-skipped rest-of-file.
    #[test]
    #[should_panic(expected = "is never closed")]
    fn an_unclosed_test_module_is_a_loud_failure() {
        let body = strip_line_comments("#[cfg(test)]\nmod tests {\n    fn t() {}\n");
        let _ = strip_test_module(Path::new("synthetic.rs"), &body);
    }

    /// A test module is excluded, and production code BELOW it is still scanned.
    #[test]
    fn code_after_a_test_module_is_still_scanned() {
        let body = strip_line_comments(
            "#[cfg(test)]\nmod tests {\n    fn t() { Writer::new(\"t\"); }\n}\n\n\
             pub fn later() { Writer::new(\"real\"); }\n",
        );
        let trimmed = strip_test_module(Path::new("synthetic.rs"), &body);
        assert_eq!(sites_in(&trimmed), vec!["later".to_string()]);
    }

    #[test]
    fn a_trailing_comment_on_the_module_closer_is_accepted() {
        let body = strip_line_comments(
            "#[cfg(test)]\nmod tests {\n    fn t() { Writer::new(\"t\"); }\n} // tests\n\n\
             pub fn later() { Writer::new(\"real\"); }\n",
        );
        let trimmed = strip_test_module(Path::new("synthetic.rs"), &body);
        assert_eq!(sites_in(&trimmed), vec!["later".to_string()]);
    }

    /// `#[cfg(test)]` on a NON-module item is not a module and is not stripped —
    /// a test-only helper that mints a header is still a call site.
    #[test]
    fn a_cfg_test_attribute_on_a_plain_item_is_not_treated_as_a_module() {
        let body =
            strip_line_comments("#[cfg(test)]\npub fn helper() { Writer::new(\"helper\"); }\n");
        let trimmed = strip_test_module(Path::new("synthetic.rs"), &body);
        assert_eq!(sites_in(&trimmed), vec!["helper".to_string()]);
    }

    /// A `//` inside a string literal is not a comment — truncating there would
    /// silently swallow whatever followed on the line.
    #[test]
    fn a_double_slash_inside_a_string_is_not_a_comment() {
        let line = r#"    let iri = "https://example.org/cat"; // trailing"#;
        assert_eq!(
            strip_line_comments(line).trim_end(),
            r#"    let iri = "https://example.org/cat";"#
        );
        assert_eq!(
            sites_in(&strip_line_comments(
                r#"fn f() { let s = "a // b"; Writer::new("real"); }"#
            )),
            vec!["f".to_string()],
            "a call site after a string containing // must still be seen"
        );
    }

    /// A DIFFERENT type whose name ends in `Writer` is not this `Writer` — the
    /// OKF codec's `OkfWriter::new` must never enter the GTS census.
    #[test]
    fn a_type_whose_name_merely_ends_in_writer_is_not_flagged() {
        assert!(!mints_a_header(
            "    let mut writer = OkfWriter::new(config);"
        ));
        assert!(!mints_a_header(
            "    let w = EmbeddingStreamWriter::new(cursor);"
        ));
        assert!(mints_a_header("    let mut w = Writer::new(\"files\");"));
        assert!(mints_a_header(
            "    purrdf_gts::writer::Writer::with_options(profile, opts)"
        ));
    }

    /// `declared_fn` reads the item name, not an arbitrary `fn` substring.
    #[test]
    fn declared_fn_reads_the_item_name() {
        assert_eq!(
            declared_fn("pub fn to_writer(").as_deref(),
            Some("to_writer")
        );
        assert_eq!(
            declared_fn("    fn writer(&self) -> X {").as_deref(),
            Some("writer")
        );
        assert_eq!(declared_fn("let confn = 3;"), None);
        assert_eq!(declared_fn("    let x = 1;"), None);
    }
}

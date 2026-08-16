// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A gate for the one class of documentation defect no other gate in this
//! repository catches: a SPARQL query/update string, or a copy-pasteable
//! `AGG(<iri>...)` call fragment, embedded in shipped human-facing text that
//! does not actually parse.
//!
//! Five shipped surfaces carried the SAME wrong `AGG(<iri>NAME, ...)` spelling
//! (the IRI closed before the aggregate's local name instead of after it) —
//! the CLI's `--help` text, the npm `.d.ts` typings, the Python `.pyi` stub,
//! and two spots in the mdBook user guide — and none of the eight
//! compiling/executing gates this workspace already runs (`make check`,
//! `make conformance`, `make capi-check`, the wasm and Python test suites)
//! could have caught any of them: `--help` text, `.d.ts` typings, `.pyi`
//! stubs, and mdBook prose are never compiled or executed by anything else in
//! this repository. This is that gate, extended to the same surfaces —
//! **the book, `--help` text, `.pyi`, `.d.ts`, and rustdoc across every
//! crate** — and it parses every candidate with THIS crate's own
//! [`SparqlParser`], not a hand-rolled approximation of the grammar: a
//! candidate is wrong exactly when the real parser says so.
//!
//! # What counts as a candidate
//!
//! Three independent extraction passes, applied per file:
//!
//! 1. **Fenced ` ```sparql ` blocks** (Markdown only) — the whole block is one
//!    candidate, parsed as a query (falling back to an update parse, since
//!    SPARQL UPDATE has no fenced-block convention of its own in this book).
//! 2. **Single-quoted shell arguments inside fenced ` ```sh `/` ```bash `
//!    blocks** (Markdown only) — extracted with POSIX single-quote semantics
//!    (the span between `'` characters, taken byte-for-byte with NO escape
//!    processing, exactly like a real shell) so a stray backslash meant as a
//!    line continuation — which single quotes do NOT support — shows up in the
//!    extracted text exactly as it would reach the real `purrdf` binary, and a
//!    query broken that way fails here the same way it fails on the command
//!    line.
//! 3. **Quoted string literals inside documentation text** — rustdoc
//!    (`///`/`//!` lines) in every crate's `.rs` sources, `#`-comment lines in
//!    the Python `.pyi` stub, and JSDoc/line-comment text in the npm `.d.ts`
//!    typings, PLUS every line of a Markdown book page (which is entirely
//!    "documentation text" already). Consecutive lines that are each, on
//!    their own, nothing but one quoted string literal are joined in order —
//!    the Rust/Python "adjacent string literal" idiom the `.pyi` stub's own
//!    worked example uses (`"SELECT ... " "WHERE ... "`).
//! 4. **`AGG(<iri>...)` call fragments** — a narrower pass over the same
//!    documentation text, independent of whether the fragment sits inside a
//!    full query: it matches `AGG(<...>...)` wherever the bracketed part
//!    contains a real `://` scheme (excluding schematic placeholders like
//!    `AGG(<NAMESPACE><NAME>, args…)`, which is prose ABOUT the shape, not a
//!    literal invocation), and wraps the fragment in a synthetic
//!    `SELECT (<fragment> AS ?x) WHERE { ?s ?p ?o }` so a bare, non-full-query
//!    copy-pasteable snippet — exactly the shape all five originally broken
//!    spots used — is still parsed for real.
//!
//! A candidate from passes 3/4 is kept only if it (pass 3) begins with a real
//! SPARQL query/update leading keyword (`SELECT`, `ASK`, `CONSTRUCT`,
//! `DESCRIBE`, `PREFIX`, `BASE`, `VERSION`, `INSERT`, `DELETE`, `WITH`,
//! `LOAD`, `CLEAR`, `DROP`, `CREATE`, `COPY`, `MOVE`, `ADD`) — the cheap,
//! conservative filter that keeps this gate from trying to parse an unrelated
//! quoted string that happens to contain the substring "SELECT" — or (pass 4)
//! names a real IRI scheme.
//!
//! # What this gate does NOT cover, and why
//!
//! * **Rust code blocks that teach a public API shape** (`QueryOptions { .. }`
//!   struct-update syntax, trait impls) are a Rust *compile* error class, not
//!   a SPARQL *parse* error class — this gate does not attempt to compile
//!   Rust. The one seam-teaching Rust example in the book (custom aggregate
//!   registration) instead has a real, compiled (non-`ignore`) doctest on
//!   [`purrdf_sparql_eval::agg_fn::AggregateRegistry::register`] carrying
//!   the identical shape, which `cargo test --workspace` (and therefore
//!   `make check`) runs on every gate. The book's own copy of that example
//!   remains prose mdBook does not compile; making it byte-identical to the
//!   doctest would need mdBook's `{{#include}}` anchor mechanism (unused
//!   anywhere else in this book today) or an `mdbook test -L <deps>` harness
//!   wired to this workspace's `CARGO_TARGET_DIR`, neither of which exists
//!   yet — named here rather than left to be discovered missing.
//! * **Python (`.py`) implementation source** and the `rdflib`-compat shim are
//!   out of scope: they are library CODE, not the documentation surfaces the
//!   finding named (`--help` text, `.pyi`, `.d.ts`, rustdoc, the book).
//! * **Inline expression fragments other than `AGG(<iri>...)`** (e.g. the
//!   `ADJUST(?x)` arity examples in the book's own prose) are not swept by
//!   pass 4, which is deliberately narrow to the one call shape the finding
//!   is about; pass 3 still catches any of those that appear as a complete,
//!   quoted query string.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use purrdf_sparql_algebra::SparqlParser;
use regex::Regex;

/// The workspace root, resolved from this crate's own manifest directory so the
/// test works regardless of the caller's current directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root resolves")
}

/// One shipped example this gate found and must parse successfully.
#[derive(Debug)]
struct Candidate {
    file: PathBuf,
    line: usize,
    kind: &'static str,
    text: String,
}

/// Leading keywords a real SPARQL query or update prologue may open with.
/// Deliberately conservative: a quoted string that does not start with one of
/// these is not treated as a shipped SPARQL example at all, rather than being
/// parsed and reported as a false-positive failure.
const LEADING_KEYWORDS: &[&str] = &[
    "SELECT",
    "ASK",
    "CONSTRUCT",
    "DESCRIBE",
    "PREFIX",
    "BASE",
    "VERSION",
    "INSERT",
    "DELETE",
    "WITH",
    "LOAD",
    "CLEAR",
    "DROP",
    "CREATE",
    "COPY",
    "MOVE",
    "ADD",
];

/// Update-form keywords that, per SPARQL 1.1 Update's own grammar, MUST be
/// followed immediately by one of a small fixed set of tokens — an IRIREF, or
/// one of `SILENT`/`DATA`/`DEFAULT`/`GRAPH`/`WHERE`/`NAMED`/`ALL`. Requiring
/// that next token is what tells an actual `ADD`/`DROP`/`CLEAR`/... statement
/// apart from an ordinary English sentence that happens to open with the same
/// word (a SEP proposal title: "Add Support Durations, Dates, and Times").
const UPDATE_FORM_KEYWORDS: &[&str] = &[
    "INSERT", "DELETE", "LOAD", "CLEAR", "DROP", "CREATE", "COPY", "MOVE", "ADD",
];

/// A form keyword that establishes what KIND of query/update this is — as
/// opposed to `PREFIX`/`BASE`/`VERSION`/`WITH`, which only open a prologue
/// that some later form keyword must complete.
const FORM_KEYWORDS: &[&str] = &[
    "SELECT",
    "ASK",
    "CONSTRUCT",
    "DESCRIBE",
    "INSERT",
    "DELETE",
    "LOAD",
    "CLEAR",
    "DROP",
    "CREATE",
    "COPY",
    "MOVE",
    "ADD",
];

/// Whether `haystack` (already upper-cased) contains `word` at a word boundary.
fn contains_word(haystack: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(word) {
        let abs = start + pos;
        let before_ok = haystack[..abs]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let after = abs + word.len();
        let after_ok = haystack[after..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// The first whitespace-delimited token of `text`, upper-cased and stripped of
/// trailing punctuation that is not itself part of the token shapes this checks
/// for (`<...`, `?...`, `*`).
fn next_token(text: &str) -> String {
    text.trim_start()
        .split(char::is_whitespace)
        .next()
        .unwrap_or("")
        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '<' && c != '?' && c != '*')
        .to_ascii_uppercase()
}

/// Whether `text` is shaped like a real, complete SPARQL query or update —
/// conservative by design, since a false positive here is a spurious gate
/// failure on prose that was never meant to be parsed at all.
///
/// A candidate must (1) be at least 20 characters — every real example in
/// this corpus is; a bare keyword like `"describe"` or `"select"` used as an
/// ordinary English word is not — (2) open with a real SPARQL leading
/// keyword, and (3) satisfy a keyword-specific structural check: `SELECT`/
/// `ASK`/`CONSTRUCT` must contain a `{` (every form of all three requires
/// one), `DESCRIBE` must be followed by an IRI/variable/`*`, `PREFIX`/`BASE`/
/// `VERSION` must be followed somewhere by a real form keyword (rather than,
/// say, a ShEx shape expression that also opens on `PREFIX`), `WITH` must be
/// followed somewhere by `DELETE`/`INSERT`, and the remaining update-form
/// keywords must be followed by the fixed token SPARQL Update's own grammar
/// requires there.
fn looks_like_query_or_update(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.chars().count() < 20 {
        return false;
    }
    let upper = trimmed.to_ascii_uppercase();
    let Some(opener) = LEADING_KEYWORDS.iter().find(|kw| {
        upper.starts_with(*kw)
            && upper[kw.len()..]
                .chars()
                .next()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
    }) else {
        return false;
    };
    let rest = &trimmed[opener.len()..];
    match *opener {
        "SELECT" | "ASK" | "CONSTRUCT" => upper.contains('{'),
        "DESCRIBE" => {
            let next = next_token(rest);
            next.starts_with('<') || next.starts_with('?') || next == "*"
        }
        "PREFIX" | "BASE" | "VERSION" => FORM_KEYWORDS.iter().any(|kw| contains_word(&upper, kw)),
        "WITH" => contains_word(&upper, "DELETE") || contains_word(&upper, "INSERT"),
        _ => {
            debug_assert!(UPDATE_FORM_KEYWORDS.contains(opener));
            let next = next_token(rest);
            next.starts_with('<')
                || matches!(
                    next.as_str(),
                    "SILENT" | "DATA" | "DEFAULT" | "GRAPH" | "WHERE" | "NAMED" | "ALL"
                )
        }
    }
}

/// A line that is, once a documentation-comment marker is stripped, nothing
/// but one quoted string literal (optionally trailed by `,`/`;`) — the shape
/// the adjacent-string-literal idiom joins across consecutive lines.
static PURE_QUOTED_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"((?:[^"\\]|\\.)*)"[,;]?$"#).expect("valid regex"));

/// Any quoted string literal, anywhere in a line.
static QUOTED_STRING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""((?:[^"\\]|\\.)*)""#).expect("valid regex"));

/// An `AGG(<iri>...)` call fragment whose bracketed part names a real scheme —
/// excludes schematic placeholders like `AGG(<NAMESPACE><NAME>, args…)`, which
/// contain no `://`.
static AGG_FRAGMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"AGG\(<([a-zA-Z][a-zA-Z0-9+.-]*://[^>]*)>([^)]*)\)").expect("valid regex")
});

/// Minimal unescape for a Rust/Python/JS double-quoted string body: the only
/// escapes this repository's own examples use.
fn unescape(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Extract candidates from documentation TEXT that is already known to be
/// "documentation" (every line of a Markdown page, or the already-stripped
/// doc-comment lines of a source file). `lines` is `(1-based line number,
/// stripped content)`.
fn scan_doc_lines(file: &Path, lines: &[(usize, String)], out: &mut Vec<Candidate>) {
    // Pass: consecutive pure-quoted-string lines, joined. A run of two or more
    // lines is recorded as CONSUMED so the individual-literal pass below does
    // not also emit one HALF of the same literal as its own (necessarily
    // incomplete, necessarily failing) candidate.
    let mut consumed = std::collections::HashSet::new();
    let mut i = 0;
    while i < lines.len() {
        let (start_line, first) = &lines[i];
        if let Some(cap) = PURE_QUOTED_LINE.captures(first.trim()) {
            let mut joined = unescape(&cap[1]);
            let mut j = i + 1;
            while j < lines.len() {
                let (line_no, content) = &lines[j];
                if *line_no != lines[j - 1].0 + 1 {
                    break;
                }
                let Some(cap) = PURE_QUOTED_LINE.captures(content.trim()) else {
                    break;
                };
                joined.push_str(&unescape(&cap[1]));
                j += 1;
            }
            if j - i >= 2 {
                for (line_no, _) in &lines[i..j] {
                    consumed.insert(*line_no);
                }
            }
            if looks_like_query_or_update(&joined) {
                out.push(Candidate {
                    file: file.to_path_buf(),
                    line: *start_line,
                    kind: "joined quoted-string literal",
                    text: joined,
                });
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }

    // Pass: any single quoted string literal on a doc line NOT already fully
    // accounted for by a multi-line join above.
    for (line_no, content) in lines {
        if consumed.contains(line_no) {
            continue;
        }
        for cap in QUOTED_STRING.captures_iter(content) {
            let text = unescape(&cap[1]);
            if looks_like_query_or_update(&text) {
                out.push(Candidate {
                    file: file.to_path_buf(),
                    line: *line_no,
                    kind: "quoted-string literal",
                    text,
                });
            }
        }
    }

    // Pass: AGG(<iri>...) fragments, wrapped into a synthetic query. Run over
    // each CONTIGUOUS run of doc lines joined with a single space rather than
    // line by line, because rustdoc/Markdown prose wraps at the column limit —
    // exactly what happened to one of the five originally-broken copy-pasteable
    // spots, whose `AGG(<https://ex.example/agg#>MEDIAN,` opened on one line
    // and closed on the next. A "contiguous run" is doc lines whose 1-based
    // line numbers increase by exactly one, so two unrelated doc comments
    // separated by ordinary code are never joined into one search space.
    for run in contiguous_runs(lines) {
        let mut joined = String::new();
        let mut line_at: Vec<usize> = Vec::new();
        for (line_no, content) in run {
            for _ in 0..=content.len() {
                line_at.push(*line_no);
            }
            joined.push_str(content);
            joined.push(' ');
        }
        for cap in AGG_FRAGMENT.captures_iter(&joined) {
            let whole = cap.get(0).expect("match 0");
            let fragment = whole.as_str();
            let line = line_at.get(whole.start()).copied().unwrap_or(run[0].0);
            out.push(Candidate {
                file: file.to_path_buf(),
                line,
                kind: "AGG(<iri>...) fragment",
                text: format!("SELECT ({fragment} AS ?agg_check_x) WHERE {{ ?s ?p ?o }}"),
            });
        }
    }
}

/// Split `lines` into maximal runs whose 1-based line numbers increase by
/// exactly one from entry to entry — the paragraph-adjacency a doc comment or
/// a Markdown page actually has, without assuming the WHOLE file is one
/// contiguous block (two doc comments separated by code are not adjacent).
fn contiguous_runs(lines: &[(usize, String)]) -> Vec<&[(usize, String)]> {
    let mut runs = Vec::new();
    let mut start = 0;
    for i in 1..=lines.len() {
        if i == lines.len() || lines[i].0 != lines[i - 1].0 + 1 {
            if start < i {
                runs.push(&lines[start..i]);
            }
            start = i;
        }
    }
    runs
}

/// Every line of a Markdown file counts as documentation text, PLUS fenced
/// ` ```sparql ` blocks (whole-block candidates) and single-quoted shell
/// arguments inside fenced ` ```sh `/` ```bash ` blocks (POSIX single-quote
/// semantics: no escape processing).
fn markdown_candidates(file: &Path, text: &str) -> Vec<Candidate> {
    let mut out = Vec::new();
    let lines: Vec<(usize, String)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect();
    scan_doc_lines(file, &lines, &mut out);

    let mut in_fence = false;
    let mut fence_lang = String::new();
    let mut fence_start = 0usize;
    let mut fence_body = String::new();
    for (line_no, content) in &lines {
        let trimmed = content.trim_start();
        if let Some(lang) = trimmed.strip_prefix("```") {
            if in_fence {
                if fence_lang == "sparql" {
                    out.push(Candidate {
                        file: file.to_path_buf(),
                        line: fence_start,
                        kind: "fenced sparql block",
                        text: fence_body.clone(),
                    });
                } else if fence_lang == "sh" || fence_lang == "bash" {
                    extract_shell_single_quoted(file, fence_start, &fence_body, &mut out);
                }
                in_fence = false;
            } else {
                in_fence = true;
                fence_lang = lang.trim().to_string();
                fence_start = *line_no;
            }
            fence_body.clear();
            continue;
        }
        if in_fence {
            fence_body.push_str(content);
            fence_body.push('\n');
        }
    }
    out
}

/// POSIX single-quote extraction: the span between `'` characters, taken
/// literally with NO escape processing — a `\` inside is ordinary text, since
/// that is exactly what a real shell does with it. `(?s)` lets the span cross
/// the line breaks a wrapped multi-line invocation uses.
fn extract_shell_single_quoted(
    file: &Path,
    block_start: usize,
    body: &str,
    out: &mut Vec<Candidate>,
) {
    static SINGLE_QUOTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)'([^']*)'").expect("valid regex"));
    for cap in SINGLE_QUOTED.captures_iter(body) {
        let text = cap[1].to_string();
        if looks_like_query_or_update(&text) {
            let line = block_start
                + body[..cap.get(0).expect("match 0").start()]
                    .matches('\n')
                    .count();
            out.push(Candidate {
                file: file.to_path_buf(),
                line,
                kind: "shell single-quoted argument",
                text,
            });
        }
    }
}

/// Rust doc-comment lines (`///`/`//!`), stripped of their marker.
///
/// Lines inside a fenced ` ``` ` block WITHIN the doc comment are dropped
/// entirely rather than returned: every such block is a real Rust doctest,
/// compiled and run by `cargo test --doc` (part of `cargo test --workspace`
/// and therefore `make check`) unless explicitly `ignore`d — and this
/// workspace's own crate sources use no `rust,ignore`/`rust,no_run` doctest
/// (only the book does), so every fenced block this sweep would see is
/// already gated elsewhere. Scanning it here would both duplicate that gate
/// and misfire on a deliberately-invalid negative example a doctest asserts
/// `.is_err()` against (e.g. [`SparqlParser::parse_query`]'s own relative-IRI
/// doctest) or a bare substring assertion that was never a whole query at all
/// (`assert!(rendered.starts_with("SELECT * WHERE {"))`).
fn rust_doc_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let stripped = trimmed
            .strip_prefix("//!")
            .or_else(|| trimmed.strip_prefix("///"));
        let Some(rest) = stripped else { continue };
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        if rest.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            out.push((i + 1, rest.to_string()));
        }
    }
    out
}

/// Python `.pyi` `#`-comment lines, stripped of their marker.
fn python_comment_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            out.push((i + 1, rest.strip_prefix(' ').unwrap_or(rest).to_string()));
        }
    }
    out
}

/// TypeScript `.d.ts` JSDoc block-comment and line-comment text.
fn typescript_doc_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_block = false;
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if in_block {
            let stripped = trimmed.strip_prefix('*').unwrap_or(trimmed);
            out.push((
                i + 1,
                stripped.strip_prefix(' ').unwrap_or(stripped).to_string(),
            ));
            if trimmed.contains("*/") {
                in_block = false;
            }
            continue;
        }
        if trimmed.starts_with("/**") {
            in_block = !trimmed.contains("*/");
            let stripped = trimmed.trim_start_matches("/**");
            out.push((i + 1, stripped.to_string()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("//") {
            out.push((i + 1, rest.strip_prefix(' ').unwrap_or(rest).to_string()));
        }
    }
    out
}

/// Every candidate this gate found across the shipped surfaces named in its
/// module docs.
fn collect_candidates() -> Vec<Candidate> {
    let root = workspace_root();
    let mut out = Vec::new();

    // The book.
    let book_src = root.join("docs/book/src");
    for entry in walk(&book_src) {
        if entry.extension().and_then(|e| e.to_str()) == Some("md") {
            let text = fs::read_to_string(&entry).unwrap_or_else(|e| panic!("read {entry:?}: {e}"));
            out.extend(markdown_candidates(&entry, &text));
        }
    }

    // Every crate's rustdoc — INCLUDING `crates/cli/src/cli.rs`, whose clap
    // `///` doc comments become the `--help` text a caller reads: clap
    // derives its help strings straight from the field-level `///` comments
    // this same rustdoc sweep is already reading, so `--help` needs no
    // separate extraction pass, only naming here as one of the surfaces this
    // sweep exists to cover.
    for crate_dir in walk(&root.join("crates")) {
        if crate_dir.extension().and_then(|e| e.to_str()) == Some("rs")
            && crate_dir.components().any(|c| c.as_os_str() == "src")
        {
            let text = fs::read_to_string(&crate_dir)
                .unwrap_or_else(|e| panic!("read {crate_dir:?}: {e}"));
            scan_doc_lines(&crate_dir, &rust_doc_lines(&text), &mut out);
        }
    }

    // The npm `.d.ts` typings.
    let dts = root.join("crates/rdf-wasm/js/index.d.ts");
    let text = fs::read_to_string(&dts).unwrap_or_else(|e| panic!("read {dts:?}: {e}"));
    scan_doc_lines(&dts, &typescript_doc_lines(&text), &mut out);

    // The Python `.pyi` stub.
    let pyi = root.join("bindings/python/python/src/purrdf/__init__.pyi");
    let text = fs::read_to_string(&pyi).unwrap_or_else(|e| panic!("read {pyi:?}: {e}"));
    scan_doc_lines(&pyi, &python_comment_lines(&text), &mut out);

    out
}

/// Every file under `root`, recursively, in a stable order.
fn walk(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The gate itself: every candidate this sweep found must parse — as a query,
/// or (for a fenced block / joined literal, which may legitimately be an
/// UPDATE request) as an update — with THIS crate's own [`SparqlParser`].
#[test]
fn every_shipped_sparql_example_parses() {
    let parser = SparqlParser::new();
    let candidates = collect_candidates();
    assert!(
        candidates.len() >= 5,
        "expected to find at least the five known AGG(<iri>...) call sites this gate exists to \
         cover, found {}; the extraction swept nothing, which means it is silently covering \
         zero shipped surfaces rather than proving them correct",
        candidates.len()
    );

    let mut failures = Vec::new();
    for candidate in &candidates {
        let query_err = parser.parse_query(&candidate.text).err();
        if query_err.is_none() {
            continue;
        }
        let update_err = parser.parse_update(&candidate.text).err();
        if update_err.is_none() {
            continue;
        }
        failures.push(format!(
            "{}:{}: [{}] does not parse as either a query or an update:\n    text: {:?}\n    \
             query error: {}\n    update error: {}",
            candidate.file.display(),
            candidate.line,
            candidate.kind,
            candidate.text,
            query_err.expect("checked above"),
            update_err.expect("checked above"),
        ));
    }

    assert!(
        failures.is_empty(),
        "{} shipped SPARQL example(s) do not parse:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

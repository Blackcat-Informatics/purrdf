// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf_lex::xml` graded against the W3C XML Conformance Test Suite
//! (`vectors/xmlconf`, vendored by `scripts/vendor-xmlconf.py`, see its
//! `PROVENANCE.md`).
//!
//! Every `TEST` in the suite's manifests lands in exactly one class, derived
//! from the test's own attributes and the reader's own error kinds; there is no
//! list of skipped cases.
//!
//! * **`error`**: `TYPE="error"`. The suite says parsers are not required to
//!   report them; no obligation.
//! * **other edition**: `EDITION` lists editions and not the fifth, which is
//!   the edition the reader implements.
//! * **XML 1.1**: `VERSION` lists `1.1` and not `1.0`. The reader is an XML 1.0
//!   processor and must refuse the document; a document that is read is a
//!   failure (a mis-parse), and a `valid` one must be refused specifically
//!   with [`XmlErrorKind::UnsupportedVersion`].
//! * **pass, valid / invalid**: the suite says a non-validating processor must
//!   accept the document (`invalid` is about validity, not well-formedness),
//!   and the reader accepts it. When the suite gives an `OUTPUT` (Second
//!   Canonical Form) the reader's information set must match it byte for byte.
//! * **pass, not-wf**: the suite says the document is not well-formed and the
//!   reader refuses it.
//! * **unsupported, refused**: a `valid` or `invalid` document the reader
//!   hard-fails on for a documented, principled reason, checked against the
//!   suite's attributes: the document's bytes are in an encoding the reader
//!   does not decode ([`XmlErrorKind::UnsupportedEncoding`], and the test's
//!   declared encoding is not one the reader supports); the test's
//!   `ENTITIES` is not `none`, so it needs an external entity the reader never
//!   fetches ([`XmlErrorKind::ExternalEntity`]); or the test is marked
//!   `NAMESPACE="no"` (it uses colons in ways Namespaces in XML forbids, and
//!   the reader always processes namespaces), refused with a namespace error.
//! * **FAIL**: anything else: a valid document refused for another reason
//!   (over-refusal), a not-well-formed document accepted, a wrong information
//!   set, or a panic.
//!
//! The totals are pinned, so any drift in either direction (a fix that turns a
//! refusal into a pass, or a regression) fails here and is re-pinned
//! deliberately.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use purrdf_lex::xml::{Document, Dtd, Node, NodeType, Options, XmlErrorKind, decode};
use purrdf_testkit::paths::workspace_root;

/// One `TEST` of the manifest.
struct Case {
    id: String,
    kind: String,
    versions: Option<Vec<String>>,
    editions: Option<Vec<String>>,
    external_entities: bool,
    namespaces_off: bool,
    path: PathBuf,
    output: Option<PathBuf>,
}

/// The manifest bytes as text.
fn read_text(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    decode(&bytes)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .into_owned()
}

/// `xmlconf.xml` with its external `SYSTEM` entities (one manifest per
/// sub-suite) spliced in place: the reader never fetches, so the runner does.
fn master(root: &Path) -> String {
    let text = read_text(&root.join("xmlconf.xml"));
    let body_at = text
        .find("<TESTSUITE")
        .expect("the master manifest has a root");
    let (head, body) = text.split_at(body_at);
    let mut body = body.to_owned();
    for declaration in head.split("<!ENTITY").skip(1) {
        let name = declaration
            .split_whitespace()
            .next()
            .expect("an entity name");
        let path = declaration
            .split('"')
            .nth(1)
            .expect("a quoted SYSTEM identifier");
        let mut part = read_text(&root.join(path));
        if part.starts_with("<?xml")
            && let Some(end) = part.find("?>")
        {
            part.replace_range(..end + 2, "");
        }
        // A test's `URI` is relative to the manifest file that holds it (the
        // master's own `xml:base` for the Edinburgh misc tests names a
        // directory that does not exist), so each spliced manifest is wrapped
        // in its directory.
        let dir = Path::new(path)
            .parent()
            .and_then(Path::to_str)
            .unwrap_or("");
        body = body.replace(
            &format!("&{name};"),
            &format!("<MANIFEST dir=\"{dir}\">{part}</MANIFEST>"),
        );
    }
    body
}

fn list(node: &Node<'_, '_>, name: &str) -> Option<Vec<String>> {
    node.attribute(name)
        .map(|v| v.split_whitespace().map(str::to_owned).collect())
}

fn cases(root: &Path) -> Vec<Case> {
    let text = master(root);
    let document = Document::parse(&text).expect("the master manifest is well-formed");
    let mut found = Vec::new();
    for test in document.descendants().filter(|n| n.has_tag_name("TEST")) {
        let base = test
            .ancestors()
            .find_map(|a| a.attribute("dir").filter(|_| a.has_tag_name("MANIFEST")))
            .expect("every TEST is inside a spliced manifest");
        let uri = test.attribute("URI").expect("every TEST has a URI");
        let path = root.join(base).join(uri);
        found.push(Case {
            id: test
                .attribute("ID")
                .expect("every TEST has an ID")
                .to_owned(),
            kind: test
                .attribute("TYPE")
                .expect("every TEST has a TYPE")
                .to_owned(),
            versions: list(&test, "VERSION"),
            editions: list(&test, "EDITION"),
            external_entities: test.attribute("ENTITIES").unwrap_or("none") != "none",
            namespaces_off: test.attribute("NAMESPACE") == Some("no"),
            output: test.attribute("OUTPUT").map(|o| root.join(base).join(o)),
            path,
        });
    }
    found
}

/// One pseudo-attribute (`version`, `encoding`) of the XML declaration that
/// opens `text`, when there is one.
fn declared(text: &str, name: &str) -> Option<String> {
    let head = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let decl = head.strip_prefix("<?xml")?;
    let decl = &decl[..decl.find("?>")?];
    let at = decl.find(name)?;
    let rest = decl[at + name.len()..]
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let quote = rest.chars().next()?;
    let rest = &rest[1..];
    Some(rest[..rest.find(quote)?].to_owned())
}

/// The `EncName` of a document whose bytes are ASCII-compatible where the
/// declaration is.
fn declared_encoding(bytes: &[u8]) -> Option<String> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]).into_owned();
    declared(&head, "encoding")
}

const SUPPORTED_ENCODINGS: [&str; 4] = ["UTF-8", "UTF-16", "US-ASCII", "ISO-8859-1"];

fn namespace_error(kind: &XmlErrorKind) -> bool {
    matches!(
        kind,
        XmlErrorKind::UndeclaredPrefix(_)
            | XmlErrorKind::ReservedNamespace(_)
            | XmlErrorKind::EmptyNamespacePrefix(_)
            | XmlErrorKind::InvalidName
            | XmlErrorKind::ReservedPiTarget
            | XmlErrorKind::DuplicateAttribute(_)
    )
}

/// The suite's Second Canonical Form of the document's content: what a
/// conforming processor reports, comparable byte for byte with an `OUTPUT`
/// file. Comments are dropped, attributes sorted, CDATA merged into text.
fn canonical(document: &Document<'_>) -> String {
    let mut out = String::new();
    for node in document.root().children() {
        write_node(&node, &mut out);
    }
    out
}

/// The canonical form's `Datachar`: `&`, `<`, `>`, `"`, TAB, LF and CR are
/// character or entity references; everything else stands for itself.
fn escape_text(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

fn write_node(node: &Node<'_, '_>, out: &mut String) {
    match node.node_type() {
        NodeType::Element => {
            let name = node.qname().expect("an element has a name");
            let _ = write!(out, "<{name}");
            let mut attributes: Vec<(String, &str)> = node
                .attributes()
                .iter()
                .map(|a| (a.qname().to_owned(), a.value()))
                .chain(node.namespace_declarations().iter().map(|n| {
                    (
                        n.name()
                            .map_or_else(|| "xmlns".to_owned(), |p| format!("xmlns:{p}")),
                        n.uri(),
                    )
                }))
                .collect();
            attributes.sort();
            for (attribute, value) in attributes {
                let _ = write!(out, " {attribute}=\"");
                escape_text(value, out);
                out.push('"');
            }
            out.push('>');
            for child in node.children() {
                write_node(&child, out);
            }
            let _ = write!(out, "</{name}>");
        }
        NodeType::Text => escape_text(node.text().unwrap_or_default(), out),
        NodeType::Pi => {
            let (target, data) = node.pi().expect("a PI has a target");
            let _ = write!(out, "<?{target} {}?>", data.unwrap_or_default());
        }
        _ => {}
    }
}

/// The verdict on one case: the class it lands in, or why it fails.
enum Verdict {
    Class(&'static str),
    Fail(String),
}

/// What the reader made of a document.
struct Read {
    canonical: String,
    unread: bool,
}

/// `<!DOCTYPE ...]>` and its line end removed from a canonical form: Second
/// Canonical Form lists the document's `NOTATION` declarations there, which
/// the reader does not report (the tree it builds is the content, and no
/// caller of the reader has a use for a notation).
fn without_notations(canonical: &str) -> String {
    let Some(start) = canonical.find("<!DOCTYPE") else {
        return canonical.to_owned();
    };
    let end = canonical[start..]
        .find("]>\n")
        .expect("a canonical DOCTYPE closes with `]>` and a line end");
    format!("{}{}", &canonical[..start], &canonical[start + end + 3..])
}

fn grade(case: &Case) -> Verdict {
    use Verdict::{Class, Fail};
    if case.kind == "error" {
        // "Parsers are not required to report errors", except that a
        // processor that does not support a document's encoding "must report
        // a fatal error".
        let bytes = fs::read(&case.path).unwrap_or_else(|e| panic!("{}: {e}", case.path.display()));
        return match decode(&bytes) {
            Err(e) if matches!(e.kind(), XmlErrorKind::UnsupportedEncoding(_)) => {
                Class("error: unsupported encoding refused")
            }
            _ => Class("error: no obligation"),
        };
    }
    if case
        .editions
        .as_ref()
        .is_some_and(|e| !e.iter().any(|e| e == "5"))
    {
        return Class("other edition");
    }
    let bytes = fs::read(&case.path).unwrap_or_else(|e| panic!("{}: {e}", case.path.display()));
    let options = Options {
        dtd: Dtd::internal_subset(),
        ..Options::default()
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let text = decode(&bytes)?;
        Document::parse_with_options(&text, options).map(|d| Read {
            canonical: canonical(&d),
            unread: d.declarations_unread(),
        })
    }));
    let Ok(outcome) = outcome else {
        return Fail("the reader panicked".to_owned());
    };
    // `VERSION` lists only versions the reader does not implement (XML 1.1):
    // the test is for XML 1.1 processors. A document that declares 1.1 must
    // be refused as such, never read as 1.0; a document that declares 1.0
    // under such a test is not run (its expected result is XML 1.1's).
    if case
        .versions
        .as_ref()
        .is_some_and(|v| !v.iter().any(|v| v == "1.0"))
    {
        let text = decode(&bytes).ok();
        let declares_11 = text
            .as_deref()
            .is_some_and(|t| declared(t, "version").as_deref() == Some("1.1"));
        return if !declares_11 {
            Class("XML 1.1 processors only: not run")
        } else {
            match outcome {
                Err(e) if matches!(e.kind(), XmlErrorKind::UnsupportedVersion(v) if v == "1.1") => {
                    Class("XML 1.1 document: refused")
                }
                Err(e) => Fail(format!(
                    "an XML 1.1 document refused as {e}, not as an unsupported version"
                )),
                Ok(_) => Fail("an XML 1.1 document was read as XML 1.0".to_owned()),
            }
        };
    }
    if case.kind == "not-wf" {
        return match outcome {
            Err(_) => Class("not-wf: refused"),
            // "No parser should accept a not-wf testcase unless it's a
            // nonvalidating parser and the test contains external entities
            // that the parser doesn't read."
            Ok(read) if case.external_entities && read.unread => {
                Class("not-wf: accepted, external declarations unread (permitted)")
            }
            Ok(_) => Fail("a not-well-formed document was accepted".to_owned()),
        };
    }
    // valid / invalid: a non-validating processor must accept.
    match outcome {
        Ok(read) => {
            if read.unread {
                // The suite's output assumes the external declarations were
                // read.
                return Class("pass: valid/invalid, external declarations unread");
            }
            if let Some(output) = &case.output {
                let want = without_notations(&read_text(output));
                if read.canonical != want {
                    return Fail(format!(
                        "wrong information set:\n    got  {:?}\n    want {want:?}",
                        read.canonical
                    ));
                }
                return Class("pass: valid/invalid, canonical form matches");
            }
            Class("pass: valid/invalid")
        }
        Err(e) => match e.kind() {
            XmlErrorKind::UnsupportedEncoding(_) => {
                let declared = declared_encoding(&bytes);
                let supported = declared.as_deref().is_some_and(|d| {
                    SUPPORTED_ENCODINGS
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(d))
                });
                if supported {
                    Fail(format!(
                        "refused as an unsupported encoding but declares {declared:?}"
                    ))
                } else {
                    Class("unsupported, refused: encoding")
                }
            }
            XmlErrorKind::ExternalEntity if case.external_entities => {
                Class("unsupported, refused: external entity")
            }
            XmlErrorKind::UnexpandedEntity(_)
                if case.external_entities || bytes.contains(&b'%') =>
            {
                Class("unsupported, refused: skipped entity")
            }
            kind if case.namespaces_off && namespace_error(kind) => {
                Class("unsupported, refused: NAMESPACE=no")
            }
            _ => Fail(format!("a valid document refused: {e}")),
        },
    }
}

#[test]
fn the_reader_is_graded_against_the_w3c_xml_conformance_suite() {
    let root = workspace_root().join("vectors/xmlconf");
    let cases = cases(&root);
    let mut tally: BTreeMap<String, BTreeMap<&'static str, usize>> = BTreeMap::new();
    let mut failures = Vec::new();
    for case in &cases {
        match grade(case) {
            Verdict::Class(class) => {
                if std::env::var("XMLCONF_LIST").is_ok_and(|want| class.contains(&want)) {
                    println!("{class}: {} {}", case.id, case.path.display());
                }
                *tally
                    .entry(case.kind.clone())
                    .or_default()
                    .entry(class)
                    .or_default() += 1;
            }
            Verdict::Fail(why) => {
                *tally
                    .entry(case.kind.clone())
                    .or_default()
                    .entry("FAIL")
                    .or_default() += 1;
                failures.push(format!("{} ({}): {why}", case.id, case.path.display()));
            }
        }
    }
    let mut report = format!("xmlconf: {} cases\n", cases.len());
    for (kind, classes) in &tally {
        for (class, count) in classes {
            let _ = writeln!(report, "  {kind:8} {class}: {count}");
        }
    }
    println!("{report}");
    assert!(
        failures.is_empty(),
        "{} failures:\n{}\n{report}",
        failures.len(),
        failures.join("\n")
    );
    let mut got = Vec::new();
    for (kind, classes) in &tally {
        for (class, count) in classes {
            got.push((kind.as_str(), *class, *count));
        }
    }
    assert_eq!(
        got, EXPECTED,
        "the totals moved: re-pin them deliberately\n{report}"
    );
    assert_eq!(cases.len(), 2585);
}

/// The pinned totals: `(TYPE, class, cases)`. Every case is in exactly one
/// row; none fails.
const EXPECTED: [(&str, &str, usize); 23] = [
    ("error", "error: no obligation", 27),
    ("error", "error: unsupported encoding refused", 6),
    ("invalid", "XML 1.1 document: refused", 7),
    ("invalid", "XML 1.1 processors only: not run", 6),
    ("invalid", "pass: valid/invalid", 138),
    ("invalid", "pass: valid/invalid, canonical form matches", 34),
    (
        "invalid",
        "pass: valid/invalid, external declarations unread",
        48,
    ),
    ("invalid", "unsupported, refused: NAMESPACE=no", 2),
    ("invalid", "unsupported, refused: external entity", 6),
    ("invalid", "unsupported, refused: skipped entity", 1),
    ("not-wf", "XML 1.1 document: refused", 147),
    ("not-wf", "XML 1.1 processors only: not run", 22),
    (
        "not-wf",
        "not-wf: accepted, external declarations unread (permitted)",
        50,
    ),
    ("not-wf", "not-wf: refused", 967),
    ("not-wf", "other edition", 312),
    ("valid", "XML 1.1 document: refused", 74),
    ("valid", "XML 1.1 processors only: not run", 10),
    ("valid", "pass: valid/invalid", 370),
    ("valid", "pass: valid/invalid, canonical form matches", 230),
    (
        "valid",
        "pass: valid/invalid, external declarations unread",
        98,
    ),
    ("valid", "unsupported, refused: NAMESPACE=no", 6),
    ("valid", "unsupported, refused: external entity", 16),
    ("valid", "unsupported, refused: skipped entity", 8),
];

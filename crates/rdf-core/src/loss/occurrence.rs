// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed occurrence of a runtime loss: where it happened, what it concerns
//! and the counts and identifiers that qualify it.
//!
//! A producer holds these as values (a term with its datatype and language, a
//! graph name, a segment index, a count) at the point it records a loss. They
//! are stored as values and written to the JSON ledger as typed members; prose
//! is produced from them by [`LossEntry::presentation`](super::LossEntry::presentation)
//! and is never the stored form. No member's meaning depends on a prefix, a
//! suffix, a marker or a reserved value: absence is an explicit variant.

use std::borrow::Cow;
use std::fmt::Write as _;

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

/// Where a loss occurred.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[non_exhaustive]
pub enum LossSite {
    /// The producer names no position.
    #[default]
    Unspecified,
    /// A position in a source document.
    Document {
        /// The source file path, when the document has one.
        path: Option<String>,
        /// The 1-based line within the document.
        line: Option<u64>,
        /// The 1-based column within the line.
        column: Option<u32>,
    },
    /// A position in a GTS container.
    Gts {
        /// The segment index.
        segment: Option<u64>,
        /// The frame index.
        frame: Option<u64>,
        /// The term id.
        term: Option<u64>,
        /// The quad index.
        quad: Option<u64>,
        /// The reifier id.
        reifier: Option<u64>,
    },
    /// A named stage of a conversion or projection.
    Stage {
        /// The stage name.
        name: String,
    },
}

/// The name of a graph in a dataset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum LossGraphName {
    /// The default graph.
    Default,
    /// A graph named by an IRI.
    Iri(String),
    /// A graph named by a blank node, by its label.
    Blank(String),
}

/// An RDF term, kept with every part that identifies it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum LossTerm {
    /// An IRI.
    Iri(String),
    /// A blank node: its label and, when the producer scopes labels, its scope.
    Blank {
        /// The blank-node label.
        label: String,
        /// The label scope, when labels are scoped.
        scope: Option<String>,
    },
    /// A literal with its datatype IRI and, for a language-tagged literal, its tag.
    Literal {
        /// The lexical form.
        lexical: String,
        /// The datatype IRI.
        datatype: String,
        /// The language tag.
        language: Option<String>,
    },
    /// A triple term.
    Triple {
        /// The subject.
        subject: Box<Self>,
        /// The predicate.
        predicate: Box<Self>,
        /// The object.
        object: Box<Self>,
    },
    /// A term known only by its index in the producer's term table.
    Indexed(u64),
}

/// What a loss concerns.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[non_exhaustive]
pub enum LossSubject {
    /// The loss concerns no particular subject.
    #[default]
    Absent,
    /// An RDF term.
    Term(LossTerm),
    /// A row of a table, by its kind and its stable identifier (a content
    /// digest-derived identifier, or the identifier the source assigned).
    Row {
        /// The row kind.
        kind: String,
        /// The row's stable identifier.
        identifier: String,
    },
    /// A graph.
    Graph(LossGraphName),
    /// A JSON Pointer, as its unescaped reference tokens in order.
    Pointer(Vec<String>),
}

/// The value of one [`LossDetail`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum LossValue {
    /// A count.
    Count(u64),
    /// An identifier, such as a segment head id.
    Identifier(String),
    /// A term.
    Term(LossTerm),
    /// A graph name.
    Graph(LossGraphName),
}

/// One named, typed fact about a loss occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LossDetail {
    /// The stable name of the fact, kebab-case.
    pub name: Cow<'static, str>,
    /// The fact.
    pub value: LossValue,
}

impl LossDetail {
    /// A count named `name`.
    #[must_use]
    pub const fn count(name: &'static str, count: u64) -> Self {
        Self {
            name: Cow::Borrowed(name),
            value: LossValue::Count(count),
        }
    }

    /// An identifier named `name`.
    #[must_use]
    pub fn identifier(name: &'static str, identifier: impl Into<String>) -> Self {
        Self {
            name: Cow::Borrowed(name),
            value: LossValue::Identifier(identifier.into()),
        }
    }

    /// A term named `name`.
    #[must_use]
    pub const fn term(name: &'static str, term: LossTerm) -> Self {
        Self {
            name: Cow::Borrowed(name),
            value: LossValue::Term(term),
        }
    }

    /// A graph name named `name`.
    #[must_use]
    pub const fn graph(name: &'static str, graph: LossGraphName) -> Self {
        Self {
            name: Cow::Borrowed(name),
            value: LossValue::Graph(graph),
        }
    }
}

impl std::fmt::Display for LossGraphName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        self.present(&mut out);
        f.write_str(&out)
    }
}

impl std::fmt::Display for LossTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        self.present(&mut out);
        f.write_str(&out)
    }
}

impl LossSubject {
    /// The subject spelled for a person: a pointer as `#` and its escaped
    /// tokens, a term or graph as its display text, a row as its identifier.
    /// `None` when the loss has no subject. Presentation only.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        match self {
            Self::Absent => None,
            Self::Term(term) => Some(term.to_string()),
            Self::Row { identifier, .. } => Some(identifier.clone()),
            Self::Graph(graph) => Some(graph.to_string()),
            Self::Pointer(tokens) => {
                let mut out = String::from("#");
                for token in tokens {
                    out.push('/');
                    out.push_str(&token.replace('~', "~0").replace('/', "~1"));
                }
                Some(out)
            }
        }
    }

    /// The subject a JSON Pointer (RFC 6901) names: its reference tokens, with
    /// `~1` and `~0` decoded. The empty string names the document root (no
    /// tokens). A path that does not begin with `/` is one token, because it is
    /// not a pointer and is not split.
    #[must_use]
    pub fn pointer(path: &str) -> Self {
        let Some(rest) = path.strip_prefix('/') else {
            return Self::Pointer(if path.is_empty() {
                Vec::new()
            } else {
                vec![path.to_owned()]
            });
        };
        Self::Pointer(
            rest.split('/')
                .map(|token| token.replace("~1", "/").replace("~0", "~"))
                .collect(),
        )
    }
}

// ---------------------------------------------------------------------------
// Structured inspection
// ---------------------------------------------------------------------------

fn optional_unsigned<T: Copy + Into<u128>>(value: Option<T>) -> ErrorValue<'static> {
    value.map_or(ErrorValue::Absent, |value| {
        ErrorValue::Unsigned(value.into())
    })
}

impl StructuredError for LossSite {
    purrdf_hash::error_visit! {
        Unspecified => "loss-site-unspecified" {},
        Document { path, line, column } => "loss-site-document" {
            "path" => Value(path.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "line" => Value(optional_unsigned(*line)),
            "column" => Value(optional_unsigned(*column)),
        },
        Gts { segment, frame, term, quad, reifier } => "loss-site-gts" {
            "segment" => Value(optional_unsigned(*segment)),
            "frame" => Value(optional_unsigned(*frame)),
            "term" => Value(optional_unsigned(*term)),
            "quad" => Value(optional_unsigned(*quad)),
            "reifier" => Value(optional_unsigned(*reifier)),
        },
        Stage { name } => "loss-site-stage" {
            "name" => Code(name),
        },
    }
}

impl StructuredError for LossGraphName {
    purrdf_hash::error_visit! {
        Default => "loss-graph-default" {},
        Iri(iri) => "loss-graph-iri" { "iri" => Iri(iri) },
        Blank(label) => "loss-graph-blank" { "label" => Text(label) },
    }
}

impl StructuredError for LossTerm {
    purrdf_hash::error_visit! {
        Iri(iri) => "loss-term-iri" { "iri" => Iri(iri) },
        Blank { label, scope } => "loss-term-blank" {
            "label" => Text(label),
            "scope" => Value(scope.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
        },
        Literal { lexical, datatype, language } => "loss-term-literal" {
            "lexical" => Text(lexical),
            "datatype" => Iri(datatype),
            "language" => Value(language.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
        },
        Triple { subject, predicate, object } => "loss-term-triple" {
            "subject" => Record(&**subject),
            "predicate" => Record(&**predicate),
            "object" => Record(&**object),
        },
        Indexed(index) => "loss-term-indexed" { "index" => Unsigned(u128::from(*index)) },
    }
}

impl StructuredError for LossSubject {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        match self {
            Self::Absent => visitor.variant("loss-subject-absent"),
            Self::Term(term) => {
                visitor.variant("loss-subject-term");
                visitor.value(Some("term"), ErrorValue::Record(term));
            }
            Self::Row { kind, identifier } => {
                visitor.variant("loss-subject-row");
                visitor.value(Some("kind"), ErrorValue::Code(kind));
                visitor.value(Some("identifier"), ErrorValue::Text(identifier));
            }
            Self::Graph(graph) => {
                visitor.variant("loss-subject-graph");
                visitor.value(Some("graph"), ErrorValue::Record(graph));
            }
            Self::Pointer(tokens) => {
                visitor.variant("loss-subject-pointer");
                visitor.begin_list(Some("tokens"), tokens.len());
                for token in tokens {
                    visitor.value(None, ErrorValue::Text(token));
                }
                visitor.end_list();
            }
        }
    }
}

impl StructuredError for LossValue {
    purrdf_hash::error_visit! {
        Count(count) => "loss-value-count" { "count" => Unsigned(u128::from(*count)) },
        Identifier(identifier) => "loss-value-identifier" { "identifier" => Text(identifier) },
        Term(term) => "loss-value-term" { "term" => Record(term) },
        Graph(graph) => "loss-value-graph" { "graph" => Record(graph) },
    }
}

impl StructuredError for LossDetail {
    purrdf_hash::error_visit! {
        @record self { name, value } => "loss-detail" {
            "name" => Code(self.name.as_ref()),
            "value" => Record(&self.value),
        }
    }
}

// ---------------------------------------------------------------------------
// Presentation
// ---------------------------------------------------------------------------

impl LossGraphName {
    fn present(&self, out: &mut String) {
        match self {
            Self::Default => out.push_str("the default graph"),
            Self::Iri(iri) => {
                let _ = write!(out, "<{iri}>");
            }
            Self::Blank(label) => {
                let _ = write!(out, "_:{label}");
            }
        }
    }
}

impl LossTerm {
    fn present(&self, out: &mut String) {
        let _ = match self {
            Self::Iri(iri) => write!(out, "<{iri}>"),
            Self::Blank {
                label,
                scope: Some(scope),
            } => write!(out, "_:{label} (scope {scope})"),
            Self::Blank { label, scope: None } => write!(out, "_:{label}"),
            Self::Literal {
                lexical,
                datatype,
                language: Some(language),
            } => write!(out, "\"{lexical}\"@{language} ({datatype})"),
            Self::Literal {
                lexical,
                datatype,
                language: None,
            } => write!(out, "\"{lexical}\"^^<{datatype}>"),
            Self::Triple {
                subject,
                predicate,
                object,
            } => {
                out.push_str("<< ");
                subject.present(out);
                out.push(' ');
                predicate.present(out);
                out.push(' ');
                object.present(out);
                out.push_str(" >>");
                Ok(())
            }
            Self::Indexed(index) => write!(out, "term {index}"),
        };
    }
}

impl LossSite {
    pub(super) fn present(&self, out: &mut String) {
        let _ = match self {
            Self::Unspecified => return,
            Self::Document { path, line, column } => {
                out.push_str("at ");
                out.push_str(path.as_deref().unwrap_or("the document"));
                if let Some(line) = line {
                    let _ = write!(out, ", line {line}");
                }
                if let Some(column) = column {
                    let _ = write!(out, ", column {column}");
                }
                return;
            }
            Self::Gts {
                segment,
                frame,
                term,
                quad,
                reifier,
            } => {
                out.push_str("at GTS");
                for (label, value) in [
                    ("segment", segment),
                    ("frame", frame),
                    ("term", term),
                    ("quad", quad),
                    ("reifier", reifier),
                ] {
                    if let Some(value) = value {
                        let _ = write!(out, " {label} {value}");
                    }
                }
                return;
            }
            Self::Stage { name } => write!(out, "in stage {name}"),
        };
    }
}

impl LossSubject {
    pub(super) fn present(&self, out: &mut String) {
        match self {
            Self::Absent => {}
            Self::Term(term) => {
                out.push_str("concerning ");
                term.present(out);
            }
            Self::Row { kind, identifier } => {
                let _ = write!(out, "concerning {kind} row {identifier}");
            }
            Self::Graph(graph) => {
                out.push_str("concerning graph ");
                graph.present(out);
            }
            Self::Pointer(tokens) => {
                out.push_str("concerning ");
                if tokens.is_empty() {
                    out.push_str("the document root");
                }
                for token in tokens {
                    out.push('/');
                    out.push_str(&token.replace('~', "~0").replace('/', "~1"));
                }
            }
        }
    }
}

impl LossDetail {
    pub(super) fn present(&self, out: &mut String) {
        let _ = write!(out, "{}: ", self.name);
        match &self.value {
            LossValue::Count(count) => {
                let _ = write!(out, "{count}");
            }
            LossValue::Identifier(identifier) => out.push_str(identifier),
            LossValue::Term(term) => term.present(out),
            LossValue::Graph(graph) => graph.present(out),
        }
    }
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

/// A minimal writer for the ledger's typed members. Every object carries a
/// `kind` member naming its variant, so a reader dispatches on one string.
pub(super) struct JsonWriter<'out> {
    out: &'out mut String,
}

impl<'out> JsonWriter<'out> {
    pub(super) const fn new(out: &'out mut String) -> Self {
        Self { out }
    }

    /// One `    "name": <value>,` line of a ledger entry, the value written by `body`.
    pub(super) fn member_line(&mut self, name: &str, body: impl FnOnce(&mut Self), last: bool) {
        self.out.push_str("    \"");
        self.out.push_str(name);
        self.out.push_str("\": ");
        body(self);
        if !last {
            self.out.push(',');
        }
        self.out.push('\n');
    }

    fn string(&mut self, value: &str) {
        self.out.push('"');
        super::escape_json_into(self.out, value);
        self.out.push('"');
    }

    fn optional_string(&mut self, value: Option<&str>) {
        match value {
            Some(value) => self.string(value),
            None => self.out.push_str("null"),
        }
    }

    fn optional_number<T: std::fmt::Display>(&mut self, value: Option<T>) {
        match value {
            Some(value) => {
                let _ = write!(self.out, "{value}");
            }
            None => self.out.push_str("null"),
        }
    }

    /// Open an object with its `kind`; later members go through [`Self::member`].
    fn open(&mut self, kind: &str) {
        self.out.push_str("{\"kind\": ");
        self.string(kind);
    }

    fn member(&mut self, name: &str) {
        self.out.push_str(", ");
        self.string(name);
        self.out.push_str(": ");
    }

    fn close(&mut self) {
        self.out.push('}');
    }

    pub(super) fn site(&mut self, site: &LossSite) {
        match site {
            LossSite::Unspecified => self.open("unspecified"),
            LossSite::Document { path, line, column } => {
                self.open("document");
                self.member("path");
                self.optional_string(path.as_deref());
                self.member("line");
                self.optional_number(*line);
                self.member("column");
                self.optional_number(*column);
            }
            LossSite::Gts {
                segment,
                frame,
                term,
                quad,
                reifier,
            } => {
                self.open("gts");
                self.member("segment");
                self.optional_number(*segment);
                self.member("frame");
                self.optional_number(*frame);
                self.member("term");
                self.optional_number(*term);
                self.member("quad");
                self.optional_number(*quad);
                self.member("reifier");
                self.optional_number(*reifier);
            }
            LossSite::Stage { name } => {
                self.open("stage");
                self.member("name");
                self.string(name);
            }
        }
        self.close();
    }

    fn graph(&mut self, graph: &LossGraphName) {
        match graph {
            LossGraphName::Default => self.open("default"),
            LossGraphName::Iri(iri) => {
                self.open("iri");
                self.member("value");
                self.string(iri);
            }
            LossGraphName::Blank(label) => {
                self.open("blank");
                self.member("label");
                self.string(label);
            }
        }
        self.close();
    }

    fn term(&mut self, term: &LossTerm) {
        match term {
            LossTerm::Iri(iri) => {
                self.open("iri");
                self.member("value");
                self.string(iri);
            }
            LossTerm::Blank { label, scope } => {
                self.open("blank");
                self.member("label");
                self.string(label);
                self.member("scope");
                self.optional_string(scope.as_deref());
            }
            LossTerm::Literal {
                lexical,
                datatype,
                language,
            } => {
                self.open("literal");
                self.member("lexical");
                self.string(lexical);
                self.member("datatype");
                self.string(datatype);
                self.member("language");
                self.optional_string(language.as_deref());
            }
            LossTerm::Triple {
                subject,
                predicate,
                object,
            } => {
                self.open("triple");
                self.member("subject");
                self.term(subject);
                self.member("predicate");
                self.term(predicate);
                self.member("object");
                self.term(object);
            }
            LossTerm::Indexed(index) => {
                self.open("indexed");
                self.member("index");
                let _ = write!(self.out, "{index}");
            }
        }
        self.close();
    }

    pub(super) fn subject(&mut self, subject: &LossSubject) {
        match subject {
            LossSubject::Absent => self.open("absent"),
            LossSubject::Term(term) => {
                self.open("term");
                self.member("term");
                self.term(term);
            }
            LossSubject::Row { kind, identifier } => {
                self.open("row");
                self.member("row_kind");
                self.string(kind);
                self.member("identifier");
                self.string(identifier);
            }
            LossSubject::Graph(graph) => {
                self.open("graph");
                self.member("graph");
                self.graph(graph);
            }
            LossSubject::Pointer(tokens) => {
                self.open("pointer");
                self.member("tokens");
                self.out.push('[');
                for (position, token) in tokens.iter().enumerate() {
                    if position > 0 {
                        self.out.push_str(", ");
                    }
                    self.string(token);
                }
                self.out.push(']');
            }
        }
        self.close();
    }

    pub(super) fn details(&mut self, details: &[LossDetail]) {
        self.out.push('[');
        for (position, detail) in details.iter().enumerate() {
            if position > 0 {
                self.out.push_str(", ");
            }
            self.open("detail");
            self.member("name");
            self.string(&detail.name);
            self.member("value");
            match &detail.value {
                LossValue::Count(count) => {
                    self.open("count");
                    self.member("count");
                    let _ = write!(self.out, "{count}");
                }
                LossValue::Identifier(identifier) => {
                    self.open("identifier");
                    self.member("identifier");
                    self.string(identifier);
                }
                LossValue::Term(term) => {
                    self.open("term");
                    self.member("term");
                    self.term(term);
                }
                LossValue::Graph(graph) => {
                    self.open("graph");
                    self.member("graph");
                    self.graph(graph);
                }
            }
            self.close();
            self.close();
        }
        self.out.push(']');
    }
}

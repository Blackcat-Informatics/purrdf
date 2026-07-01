// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Rust-owned read-side view over a folded GTS graph.
//!
//! This module owns the query idioms that used to live in the Python fold-view
//! and relational projection shims: scoped quad lookup, term accessors, language
//! tag projection, RDF list walking, statement-layer access, and the compact
//! dictionary-encoded database rows.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_gts::model::{BlobEntry, Graph, Quad, Term, TermKind, Triple3};

pub const DEFAULT_SCOPE: &str = "";
pub const ALL_SCOPE: &str = "__all__";

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
#[cfg(test)]
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const NAMESPACE: &str = "https://blackcatinformatics.ca/purrdf/";
const LANGUAGE_CLASS: &str = "https://blackcatinformatics.ca/purrdf/Language";
const LANGUAGE_TAG: &str = "https://blackcatinformatics.ca/purrdf/languageTag";
const BCP47_TAG: &str = "https://blackcatinformatics.ca/purrdf/bcp47Tag";

#[derive(Clone, Debug, PartialEq)]
pub enum PublicValue {
    Iri(String),
    Blank(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    LanguageString { value: String, lang: String },
}

pub type TermRow = (
    usize,
    u8,
    Option<String>,
    Option<usize>,
    Option<String>,
    Option<usize>,
);
pub type QuadRow = (usize, usize, usize, Option<usize>);
pub type ReifierRow = (usize, usize, usize, usize);
pub type AnnotationRow = (usize, usize, usize);
pub type BlobRow = (String, Vec<u8>);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelationalRows {
    pub terms: Vec<TermRow>,
    pub quads: Vec<QuadRow>,
    pub reifiers: Vec<ReifierRow>,
    pub annotations: Vec<AnnotationRow>,
    pub blobs: Vec<BlobRow>,
}

#[derive(Debug)]
pub struct GtsFoldView {
    graph: Graph,
    iri_index: BTreeMap<String, usize>,
    spo: BTreeMap<ScopeKey, BTreeMap<usize, Vec<(usize, usize)>>>,
    po: BTreeMap<ScopeKey, BTreeMap<(usize, usize), Vec<usize>>>,
    tag_map: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ScopeKey {
    Default,
    All,
    Named(usize),
}

impl GtsFoldView {
    pub fn new(graph: Graph) -> Self {
        let mut view = Self {
            graph,
            iri_index: BTreeMap::new(),
            spo: BTreeMap::new(),
            po: BTreeMap::new(),
            tag_map: BTreeMap::new(),
        };
        view.build_iri_index();
        view.build_quad_indexes();
        view.tag_map = view.build_tag_map();
        view
    }

    pub fn into_graph(self) -> Graph {
        self.graph
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn term(&self, tid: usize) -> &Term {
        &self.graph.terms[tid]
    }

    pub fn is_iri(&self, tid: usize) -> bool {
        self.term(tid).kind == TermKind::Iri
    }

    pub fn is_bnode(&self, tid: usize) -> bool {
        self.term(tid).kind == TermKind::Bnode
    }

    pub fn is_literal(&self, tid: usize) -> bool {
        self.term(tid).kind == TermKind::Literal
    }

    pub fn iri(&self, tid: usize) -> Option<&str> {
        let term = self.term(tid);
        (term.kind == TermKind::Iri)
            .then_some(term.value.as_deref())
            .flatten()
    }

    pub fn lex(&self, tid: usize) -> &str {
        self.term(tid).value.as_deref().unwrap_or("")
    }

    pub fn lang(&self, tid: usize) -> Option<&str> {
        self.term(tid).lang.as_deref()
    }

    pub fn datatype(&self, tid: usize) -> String {
        self.graph.datatype_iri(self.term(tid))
    }

    pub fn nq_token(&self, tid: usize) -> String {
        render_term(&self.graph, tid)
    }

    pub fn public_value(&self, tid: usize) -> PublicValue {
        let term = self.term(tid);
        match term.kind {
            TermKind::Literal => {
                let lex = self.lex(tid).to_string();
                let datatype = self.datatype(tid);
                let xsd_type = datatype.strip_prefix(XSD);
                if xsd_type == Some("integer") {
                    return lex
                        .parse::<i64>()
                        .map(PublicValue::Integer)
                        .unwrap_or(PublicValue::String(lex));
                }
                if matches!(xsd_type, Some("decimal" | "double" | "float")) {
                    return lex
                        .parse::<f64>()
                        .map(PublicValue::Float)
                        .unwrap_or(PublicValue::String(lex));
                }
                if xsd_type == Some("boolean") {
                    return match lex.to_ascii_lowercase().as_str() {
                        "true" | "1" => PublicValue::Boolean(true),
                        "false" | "0" => PublicValue::Boolean(false),
                        _ => PublicValue::String(lex),
                    };
                }
                if let Some(lang) = self.lang(tid) {
                    return PublicValue::LanguageString {
                        value: lex,
                        lang: lang.to_string(),
                    };
                }
                PublicValue::String(lex)
            }
            TermKind::Iri => PublicValue::Iri(curie(self.lex(tid))),
            TermKind::Bnode => PublicValue::Blank(format!("_:{}", self.lex(tid))),
            TermKind::Triple => PublicValue::String(self.nq_token(tid)),
        }
    }

    pub fn tid_of_iri(&self, iri: &str) -> Option<usize> {
        self.iri_index.get(iri).copied()
    }

    pub fn curie(&self, iri: &str) -> String {
        curie(iri)
    }

    pub fn quads(&self, scope: Option<&str>) -> Vec<Quad> {
        let Some(key) = self.scope_key(scope) else {
            return Vec::new();
        };
        if key == ScopeKey::All {
            return self.graph.quads.clone();
        }
        let graph_scope = graph_term_for_scope(key);
        self.spo
            .get(&key)
            .map(|subjects| {
                let mut rows = Vec::new();
                for (&s, pairs) in subjects {
                    for &(p, o) in pairs {
                        rows.push((s, p, o, graph_scope));
                    }
                }
                rows
            })
            .unwrap_or_default()
    }

    pub fn subjects_by_type(&self, class_iri: &str, scope: Option<&str>) -> Vec<usize> {
        let (Some(type_tid), Some(class_tid)) =
            (self.tid_of_iri(RDF_TYPE), self.tid_of_iri(class_iri))
        else {
            return Vec::new();
        };
        let Some(key) = self.scope_key(scope) else {
            return Vec::new();
        };
        let mut out = BTreeSet::new();
        if let Some(subjects) = self
            .po
            .get(&key)
            .and_then(|idx| idx.get(&(type_tid, class_tid)))
        {
            out.extend(subjects.iter().copied());
        }
        out.into_iter().collect()
    }

    pub fn objects(&self, s_tid: usize, p_iri: &str, scope: Option<&str>) -> Vec<usize> {
        let Some(p_tid) = self.tid_of_iri(p_iri) else {
            return Vec::new();
        };
        let Some(key) = self.scope_key(scope) else {
            return Vec::new();
        };
        let mut out = BTreeSet::new();
        if let Some(rows) = self.spo.get(&key).and_then(|idx| idx.get(&s_tid)) {
            for &(p, o) in rows {
                if p == p_tid {
                    out.insert(o);
                }
            }
        }
        out.into_iter().collect()
    }

    pub fn value(&self, s_tid: usize, p_iri: &str, scope: Option<&str>) -> Option<usize> {
        self.objects(s_tid, p_iri, scope)
            .into_iter()
            .map(|tid| (self.nq_token(tid), tid))
            .min_by(|(left, _), (right, _)| left.cmp(right))
            .map(|(_, tid)| tid)
    }

    pub fn predicate_objects(&self, s_tid: usize, scope: Option<&str>) -> Vec<(usize, usize)> {
        let Some(key) = self.scope_key(scope) else {
            return Vec::new();
        };
        let mut out = BTreeSet::new();
        if let Some(rows) = self.spo.get(&key).and_then(|idx| idx.get(&s_tid)) {
            out.extend(rows.iter().copied());
        }
        out.into_iter().collect()
    }

    pub fn has(&self, s_tid: usize, p_iri: &str, o_tid: usize, scope: Option<&str>) -> bool {
        let Some(p_tid) = self.tid_of_iri(p_iri) else {
            return false;
        };
        let Some(key) = self.scope_key(scope) else {
            return false;
        };
        self.spo
            .get(&key)
            .and_then(|idx| idx.get(&s_tid))
            .is_some_and(|rows| rows.contains(&(p_tid, o_tid)))
    }

    pub fn rdf_list(&self, head_tid: usize, scope: Option<&str>) -> Vec<usize> {
        let nil = self.tid_of_iri(RDF_NIL);
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = Some(head_tid);
        while let Some(tid) = current {
            if Some(tid) == nil || !seen.insert(tid) {
                break;
            }
            if let Some(first) = self.value(tid, RDF_FIRST, scope) {
                out.push(first);
            }
            current = self.value(tid, RDF_REST, scope);
        }
        out
    }

    /// The purrdf-gts 0.9.11 reifier rows `(reifier_id, (s,p,o), graph?)`. purrdf's
    /// statement layer is standpoint-scoped, so the graph slot is always `None`.
    pub fn reifiers(&self) -> &[(usize, Triple3, Option<usize>)] {
        &self.graph.reifiers
    }

    /// The 0.9.11 annotation rows `(reifier, predicate, value, graph?)` (graph `None`).
    pub fn annotations(&self) -> &[(usize, usize, usize, Option<usize>)] {
        &self.graph.annotations
    }

    pub fn tag_map(&self) -> &BTreeMap<String, String> {
        &self.tag_map
    }

    pub fn available_languages(&self) -> BTreeSet<String> {
        let mut tags = BTreeSet::from(["en".to_string()]);
        for term in &self.graph.terms {
            if term.kind != TermKind::Literal {
                continue;
            }
            let Some(lang) = &term.lang else {
                continue;
            };
            let public = if is_internal_tag(lang) {
                self.tag_map
                    .get(lang)
                    .cloned()
                    .unwrap_or_else(|| lang.to_string())
            } else {
                lang.to_string()
            };
            if !public.eq_ignore_ascii_case("en") {
                tags.insert(public.to_ascii_lowercase());
            }
        }
        tags
    }

    pub fn public_text(&self, s_tid: usize, p_iri: &str, scope: Option<&str>) -> String {
        self.public_literal(s_tid, p_iri, scope).0
    }

    pub fn public_literal(
        &self,
        s_tid: usize,
        p_iri: &str,
        scope: Option<&str>,
    ) -> (String, Option<String>) {
        let mut candidates: Vec<usize> = self
            .objects(s_tid, p_iri, scope)
            .into_iter()
            .filter(|&tid| self.is_literal(tid))
            .collect();
        if candidates.is_empty() {
            return (String::new(), None);
        }
        candidates.sort_by(|&a, &b| {
            (self.lang(a).unwrap_or(""), self.lex(a))
                .cmp(&(self.lang(b).unwrap_or(""), self.lex(b)))
        });
        let mut ranked = candidates.clone();
        ranked.sort_by_key(|&tid| rank_language(self.lang(tid).unwrap_or("")));
        for tid in ranked {
            if let Some(lang) = self.lang(tid) {
                if is_internal_tag(lang) {
                    if let Some(public) = self.tag_map.get(lang) {
                        return (self.lex(tid).to_string(), Some(public.clone()));
                    }
                }
            }
        }
        let first = candidates[0];
        let public = self.public_bcp47_for(first);
        (self.lex(first).to_string(), public)
    }

    pub fn public_literal_with_fallback(
        &self,
        s_tid: usize,
        p_iri: &str,
        requested: &[String],
        scope: Option<&str>,
    ) -> (String, Option<String>, bool) {
        let candidates: Vec<usize> = self
            .objects(s_tid, p_iri, scope)
            .into_iter()
            .filter(|&tid| self.is_literal(tid))
            .collect();
        self.select_literal(&candidates, requested)
            .unwrap_or((String::new(), None, false))
    }

    pub fn public_texts(
        &self,
        s_tid: usize,
        p_iri: &str,
        requested: &[String],
        scope: Option<&str>,
    ) -> Vec<(String, Option<String>, bool)> {
        let candidates: Vec<usize> = self
            .objects(s_tid, p_iri, scope)
            .into_iter()
            .filter(|&tid| self.is_literal(tid))
            .collect();
        self.filter_literals(&candidates, requested)
    }

    pub fn relational_rows(&self) -> Result<RelationalRows, String> {
        relational_rows(&self.graph)
    }

    fn build_iri_index(&mut self) {
        for (tid, term) in self.graph.terms.iter().enumerate() {
            if term.kind == TermKind::Iri {
                if let Some(value) = &term.value {
                    self.iri_index.entry(value.clone()).or_insert(tid);
                }
            }
        }
    }

    fn build_quad_indexes(&mut self) {
        for &(s, p, o, g) in &self.graph.quads {
            let scope = g.map(ScopeKey::Named).unwrap_or(ScopeKey::Default);
            for key in [scope, ScopeKey::All] {
                self.spo
                    .entry(key)
                    .or_default()
                    .entry(s)
                    .or_default()
                    .push((p, o));
                self.po
                    .entry(key)
                    .or_default()
                    .entry((p, o))
                    .or_default()
                    .push(s);
            }
        }
    }

    fn build_tag_map(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for lang_tid in self.subjects_by_type(LANGUAGE_CLASS, Some(ALL_SCOPE)) {
            let internal = self.value(lang_tid, LANGUAGE_TAG, Some(ALL_SCOPE));
            let bcp = self.value(lang_tid, BCP47_TAG, Some(ALL_SCOPE));
            if let (Some(internal), Some(bcp)) = (internal, bcp) {
                out.insert(self.lex(internal).to_string(), self.lex(bcp).to_string());
            }
        }
        out
    }

    fn public_bcp47_for(&self, tid: usize) -> Option<String> {
        let lang = self.lang(tid)?;
        if is_internal_tag(lang) {
            Some(
                self.tag_map
                    .get(lang)
                    .cloned()
                    .unwrap_or_else(|| lang.to_string()),
            )
        } else {
            Some(lang.to_string())
        }
    }

    fn bucket_by_bcp(&self, candidates: &[usize]) -> BTreeMap<String, Vec<LitRow>> {
        let mut by_bcp: BTreeMap<String, Vec<LitRow>> = BTreeMap::new();
        for &tid in candidates {
            let bcp = self.public_bcp47_for(tid);
            let key = bcp.as_deref().unwrap_or("").to_ascii_lowercase();
            let original = self.lang(tid).unwrap_or("").to_string();
            by_bcp
                .entry(key)
                .or_default()
                .push((self.lex(tid).to_string(), bcp, original));
        }
        for rows in by_bcp.values_mut() {
            rows.sort_by(|a, b| (rank_language(&a.2), &a.0).cmp(&(rank_language(&b.2), &b.0)));
        }
        by_bcp
    }

    fn requested_tags(requested: &[String]) -> Vec<String> {
        if requested.is_empty() {
            return vec!["en".to_string()];
        }
        requested
            .iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect()
    }

    fn select_literal(
        &self,
        candidates: &[usize],
        requested: &[String],
    ) -> Option<(String, Option<String>, bool)> {
        if candidates.is_empty() {
            return None;
        }
        let by_bcp = self.bucket_by_bcp(candidates);
        for req in Self::requested_tags(requested) {
            if let Some(rows) = by_bcp.get(&req) {
                let (text, bcp, _) = &rows[0];
                return Some((text.clone(), bcp.clone(), false));
            }
        }
        self.fallback_literal(&by_bcp)
    }

    fn filter_literals(
        &self,
        candidates: &[usize],
        requested: &[String],
    ) -> Vec<(String, Option<String>, bool)> {
        if candidates.is_empty() {
            return Vec::new();
        }
        let by_bcp = self.bucket_by_bcp(candidates);
        let mut out = Vec::new();
        for req in Self::requested_tags(requested) {
            if let Some(rows) = by_bcp.get(&req) {
                out.extend(
                    rows.iter()
                        .map(|(text, bcp, _)| (text.clone(), bcp.clone(), false)),
                );
            }
        }
        if !out.is_empty() {
            return out;
        }
        self.fallback_literal(&by_bcp).into_iter().collect()
    }

    fn fallback_literal(
        &self,
        by_bcp: &BTreeMap<String, Vec<LitRow>>,
    ) -> Option<(String, Option<String>, bool)> {
        if let Some(rows) = by_bcp.get("en") {
            let (text, bcp, _) = &rows[0];
            return Some((text.clone(), bcp.clone(), true));
        }
        if let Some((_tag, (text, bcp, _))) = best_tagged(by_bcp) {
            return Some((text.clone(), bcp.clone(), true));
        }
        if let Some(rows) = by_bcp.get("") {
            let (text, bcp, _) = &rows[0];
            return Some((text.clone(), bcp.clone(), true));
        }
        None
    }

    fn scope_key(&self, scope: Option<&str>) -> Option<ScopeKey> {
        match scope {
            Some(ALL_SCOPE) => Some(ScopeKey::All),
            Some(scope) => self.tid_of_iri(scope).map(ScopeKey::Named),
            None => Some(ScopeKey::Default),
        }
    }
}

type LitRow = (String, Option<String>, String);

pub fn relational_rows(graph: &Graph) -> Result<RelationalRows, String> {
    let blobs = graph
        .blobs
        .iter()
        .map(|(digest, entry)| decoded_blob(entry).map(|bytes| (digest.clone(), bytes)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(RelationalRows {
        terms: graph
            .terms
            .iter()
            .enumerate()
            .map(|(id, term)| {
                (
                    id,
                    term_kind_int(term.kind),
                    term.value.clone(),
                    term.datatype,
                    term.lang.clone(),
                    term.reifier,
                )
            })
            .collect(),
        quads: graph.quads.clone(),
        // Flatten the 0.9.11 row-array to the narrow relational view, dropping the
        // always-`None` graph slot (the relational view carries no graph axis).
        reifiers: graph
            .reifiers
            .iter()
            .map(|&(r, (s, p, o), _graph)| (r, s, p, o))
            .collect(),
        annotations: graph
            .annotations
            .iter()
            .map(|&(r, p, o, _graph)| (r, p, o))
            .collect(),
        blobs,
    })
}

fn decoded_blob(entry: &BlobEntry) -> Result<Vec<u8>, String> {
    entry
        .decoded_vec()
        .map_err(|err| format!("cannot decode blob: {err:?}"))
}

fn graph_term_for_scope(scope: ScopeKey) -> Option<usize> {
    match scope {
        ScopeKey::Named(tid) => Some(tid),
        ScopeKey::Default | ScopeKey::All => None,
    }
}

fn term_kind_int(kind: TermKind) -> u8 {
    match kind {
        TermKind::Iri => 0,
        TermKind::Literal => 1,
        TermKind::Bnode => 2,
        TermKind::Triple => 3,
    }
}

fn is_internal_tag(lang: &str) -> bool {
    let lower = lang.to_ascii_lowercase();
    let Some(suffix) = lower.strip_prefix("x-purrdf-") else {
        return false;
    };
    !suffix.is_empty()
        && suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn rank_language(lang: &str) -> (u8, String) {
    let lower = lang.to_ascii_lowercase();
    let rank = if lower == "x-purrdf-english" { 0 } else { 1 };
    (rank, lower)
}

fn best_tagged(by_bcp: &BTreeMap<String, Vec<LitRow>>) -> Option<(&str, &LitRow)> {
    by_bcp
        .iter()
        .filter(|(tag, rows)| !tag.is_empty() && !rows.is_empty())
        .map(|(tag, rows)| (tag.as_str(), &rows[0]))
        .min_by(|a, b| rank_language(&a.1 .2).cmp(&rank_language(&b.1 .2)))
}

fn render_term(graph: &Graph, tid: usize) -> String {
    let term = &graph.terms[tid];
    match term.kind {
        TermKind::Iri => format!("<{}>", term.value.as_deref().unwrap_or("")),
        TermKind::Bnode => term
            .value
            .as_ref()
            .map(|value| format!("_:{value}"))
            .unwrap_or_else(|| format!("_:b{tid}")),
        TermKind::Literal => render_literal(graph, term),
        TermKind::Triple => graph
            .reifier(term.reifier.unwrap_or(tid))
            .map(|(s, p, o)| {
                format!(
                    "<<( {} {} {} )>>",
                    render_term(graph, s),
                    render_term(graph, p),
                    render_term(graph, o)
                )
            })
            .unwrap_or_else(|| format!("_:unbound_triple_{tid}")),
    }
}

fn render_literal(graph: &Graph, term: &Term) -> String {
    let lit = format!("\"{}\"", nt_escape(term.value.as_deref().unwrap_or("")));
    if let Some(lang) = &term.lang {
        if let Some(direction) = term.direction.as_deref() {
            return format!("{lit}@{lang}--{direction}");
        }
        return format!("{lit}@{lang}");
    }
    if let Some(datatype) = term.datatype {
        return format!("{lit}^^{}", render_term(graph, datatype));
    }
    lit
}

fn nt_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn curie(iri: &str) -> String {
    for (prefix, namespace) in PREFIXES {
        if let Some(local) = iri.strip_prefix(namespace) {
            return format!("{prefix}:{local}");
        }
    }
    iri.to_string()
}

const PREFIXES: &[(&str, &str)] = &[
    ("purrdf", NAMESPACE),
    ("logic", "https://blackcatinformatics.ca/logic/"),
    ("schema", "https://schema.org/"),
    ("rdf", RDF),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("xsd", XSD),
    ("skos", "http://www.w3.org/2004/02/skos/core#"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_gts::model::Term;
    use purrdf_gts::writer::Writer;

    const EX: &str = "https://example.org/";
    const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

    fn iri(value: &str) -> Term {
        Term {
            kind: TermKind::Iri,
            value: Some(value.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        }
    }

    fn bnode(value: &str) -> Term {
        Term {
            kind: TermKind::Bnode,
            value: Some(value.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        }
    }

    fn lit(value: &str, lang: Option<&str>, datatype: Option<usize>) -> Term {
        Term {
            kind: TermKind::Literal,
            value: Some(value.to_string()),
            datatype,
            lang: lang.map(str::to_string),
            direction: None,
            reifier: None,
        }
    }

    fn test_view() -> GtsFoldView {
        let mut writer = Writer::new("dist");
        writer.add_terms(&[
            iri(&(EX.to_string() + "cat")),
            iri(RDF_TYPE),
            iri(&(EX.to_string() + "Animal")),
            iri(RDFS_LABEL),
            lit("Cat", Some("en"), None),
            iri(&(EX.to_string() + "age")),
            iri(&(XSD.to_string() + "integer")),
            lit("7", None, Some(6)),
            iri(&(EX.to_string() + "graph")),
            iri(&(EX.to_string() + "dog")),
            bnode("l1"),
            bnode("l2"),
            iri(RDF_FIRST),
            iri(RDF_REST),
            iri(RDF_NIL),
            iri(&(EX.to_string() + "members")),
            iri(&(EX.to_string() + "r1")),
            iri(&(EX.to_string() + "confidence")),
            lit("0.9", None, None),
        ]);
        writer.add_quads(&[
            (0, 1, 2, None),
            (9, 1, 2, Some(8)),
            (0, 3, 4, None),
            (0, 5, 7, None),
            (0, 15, 10, None),
            (10, 12, 0, None),
            (10, 13, 11, None),
            (11, 12, 9, None),
            (11, 13, 14, None),
        ]);
        writer.add_reifies(&[(16, (0, 1, 2), None)]);
        writer.add_annot(&[(16, 17, 18, None)]);
        GtsFoldView::new(purrdf_gts::reader::read(&writer.to_bytes(), true, None))
    }

    #[test]
    fn term_accessors_and_python_values_are_native() {
        let view = test_view();
        let cat = view.tid_of_iri(&(EX.to_string() + "cat")).expect("cat");
        assert!(view.is_iri(cat));
        assert_eq!(view.iri(cat), Some(&(EX.to_string() + "cat") as &str));
        assert_eq!(view.nq_token(cat), format!("<{EX}cat>"));
        let label = view.objects(cat, RDFS_LABEL, None)[0];
        assert_eq!(view.lex(label), "Cat");
        assert_eq!(view.lang(label), Some("en"));
        assert_eq!(view.datatype(label), RDF_LANG_STRING);
        assert_eq!(
            view.public_value(label),
            PublicValue::LanguageString {
                value: "Cat".to_string(),
                lang: "en".to_string()
            }
        );
        assert_eq!(
            view.public_literal(cat, RDFS_LABEL, None),
            ("Cat".to_string(), Some("en".to_string()))
        );
        let age = view.objects(cat, &(EX.to_string() + "age"), None)[0];
        assert_eq!(view.public_value(age), PublicValue::Integer(7));
    }

    #[test]
    fn invalid_boolean_literals_stay_strings() {
        let mut writer = Writer::new("dist");
        writer.add_terms(&[
            iri(&(XSD.to_string() + "boolean")),
            lit("false", None, Some(0)),
            lit("not-a-boolean", None, Some(0)),
        ]);
        let view = GtsFoldView::new(purrdf_gts::reader::read(&writer.to_bytes(), true, None));
        assert_eq!(view.public_value(1), PublicValue::Boolean(false));
        assert_eq!(
            view.public_value(2),
            PublicValue::String("not-a-boolean".to_string())
        );
    }

    #[test]
    fn scoped_lookup_rdf_lists_and_statement_rows_are_native() {
        let view = test_view();
        let cat = view.tid_of_iri(&(EX.to_string() + "cat")).expect("cat");
        let dog = view.tid_of_iri(&(EX.to_string() + "dog")).expect("dog");
        assert_eq!(
            view.subjects_by_type(&(EX.to_string() + "Animal"), None),
            vec![cat]
        );
        assert_eq!(
            view.subjects_by_type(
                &(EX.to_string() + "Animal"),
                Some(&(EX.to_string() + "graph"))
            ),
            vec![dog]
        );
        assert_eq!(
            view.subjects_by_type(&(EX.to_string() + "Animal"), Some(ALL_SCOPE)),
            vec![cat, dog]
        );
        let head = view.objects(cat, &(EX.to_string() + "members"), None)[0];
        assert_eq!(view.rdf_list(head, None), vec![cat, dog]);
        assert_eq!(view.reifiers(), &[(16, (0, 1, 2), None)]);
        assert_eq!(view.annotations(), &[(16, 17, 18, None)]);
    }

    #[test]
    fn language_boundary_reads_tag_map_across_all_scopes() {
        let mut writer = Writer::new("dist");
        writer.add_terms(&[
            iri(RDF_TYPE),
            iri(LANGUAGE_CLASS),
            iri(LANGUAGE_TAG),
            iri(BCP47_TAG),
            iri(RDFS_LABEL),
            iri(&(NAMESPACE.to_string() + "English")),
            lit("x-purrdf-english", Some("en"), None),
            lit("en", Some("en"), None),
            iri(&(NAMESPACE.to_string() + "French")),
            lit("x-purrdf-french", Some("en"), None),
            lit("fr", Some("en"), None),
            iri(&(NAMESPACE.to_string() + "Thing")),
            lit("Hello", Some("x-purrdf-english"), None),
            lit("Bonjour", Some("x-purrdf-french"), None),
            iri(&(NAMESPACE.to_string() + "graph/languages")),
        ]);
        writer.add_quads(&[
            (5, 0, 1, Some(14)),
            (5, 2, 6, Some(14)),
            (5, 3, 7, Some(14)),
            (8, 0, 1, Some(14)),
            (8, 2, 9, Some(14)),
            (8, 3, 10, Some(14)),
            (11, 4, 12, None),
            (11, 4, 13, None),
        ]);
        let view = GtsFoldView::new(purrdf_gts::reader::read(&writer.to_bytes(), true, None));
        assert_eq!(
            view.tag_map().get("x-purrdf-english"),
            Some(&"en".to_string())
        );
        assert_eq!(
            view.tag_map().get("x-purrdf-french"),
            Some(&"fr".to_string())
        );
        assert_eq!(
            view.available_languages(),
            BTreeSet::from(["en".to_string(), "fr".to_string()])
        );
        assert_eq!(
            view.public_literal_with_fallback(11, RDFS_LABEL, &["fr".to_string()], None),
            ("Bonjour".to_string(), Some("fr".to_string()), false)
        );
    }

    #[test]
    fn relational_rows_keep_dictionary_ids() {
        let view = test_view();
        let rows = view.relational_rows().expect("relational rows");
        assert_eq!(rows.terms.len(), view.graph().terms.len());
        assert_eq!(rows.quads.len(), view.graph().quads.len());
        assert_eq!(rows.reifiers, vec![(16, 0, 1, 2)]);
        assert_eq!(rows.annotations, vec![(16, 17, 18)]);
        assert!(rows.blobs.is_empty());
    }
}

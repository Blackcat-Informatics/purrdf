// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Rust-owned read-side view over a folded GTS graph.
//!
//! This module owns the query idioms that used to live in the Python fold-view
//! and relational projection shims: scoped quad lookup, term accessors, language
//! tag projection, RDF list walking, statement-layer access, and the compact
//! dictionary-encoded database rows.
//!
//! IRI compaction ([`GtsFoldView::curie`]) is [`purrdf_iri::contract`] over the
//! caller's [`GtsFoldViewConfig::curie_prefixes`] and a built-in table of W3C
//! Recommendation namespaces only (`rdf`, `rdfs`, `owl`, `xsd`, `skos`): the longest
//! matching namespace wins. Any other vocabulary, schema.org included, compacts only
//! when the caller supplies its prefix.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use purrdf_core::collections::{SoleObject, walk_rdf_list};
use purrdf_gts::model::{BlobEntry, Graph, Quad, Term, TermKind, Triple3, language_tag_refusal};
use purrdf_iri::PrefixMap;
use purrdf_iri::langtag::identity_fold;
#[cfg(test)]
use purrdf_iri::vocab::rdf::LANG_STRING as RDF_LANG_STRING;
use purrdf_iri::vocab::rdf::{
    FIRST as RDF_FIRST, NIL as RDF_NIL, REST as RDF_REST, TYPE as RDF_TYPE,
};
use purrdf_iri::vocab::{owl, rdf, rdfs, skos};
use purrdf_lex::term_syntax;
use purrdf_xsd::datatype::{XSD_NS as XSD, XSD_STRING};

use crate::RdfDiagnostic;
use crate::gts_resolve::ensure_terms_terminate;

/// Scope name for the default graph (quads with no graph term).
pub const DEFAULT_SCOPE: &str = "";
/// Sentinel scope name selecting every quad regardless of graph.
pub const ALL_SCOPE: &str = "__all__";

/// The consumer-ontology language vocabulary the fold view scans to build its
/// internal→BCP-47 retag map (mirrors the `StatementMetadataVocab` pattern in
/// the JSON-LD codec).
///
/// The retag map is built from subjects typed `a <language_class>` carrying a
/// `<language_tag>` (the internal `x-purrdf-*` tag) and a `<bcp47_tag>` (the
/// public tag). These are the CALLER's vocabulary terms — PurRDF mints no such
/// ontology terms of its own — so there is no default: a view constructed
/// without a vocab performs no namespace scanning and its retag map is empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageVocab {
    /// The language-individual class IRI (subjects typed with this are scanned).
    pub language_class: String,
    /// The predicate carrying the internal (`x-purrdf-*`) tag literal.
    pub language_tag: String,
    /// The predicate carrying the public BCP-47 tag literal.
    pub bcp47_tag: String,
}

impl LanguageVocab {
    /// Derive the three term IRIs by concatenation from a namespace whose
    /// local names are `Language` / `languageTag` / `bcp47Tag`.
    #[must_use]
    pub fn for_namespace(ns: &str) -> Self {
        Self {
            language_class: format!("{ns}Language"),
            language_tag: format!("{ns}languageTag"),
            bcp47_tag: format!("{ns}bcp47Tag"),
        }
    }
}

/// Consumer configuration for [`GtsFoldView`]: the optional language vocabulary
/// (drives the retag map) and any extra CURIE prefix entries consulted, beside the
/// built-in W3C Recommendation table, when compacting IRIs for
/// [`PublicValue::Iri`] / [`GtsFoldView::curie`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GtsFoldViewConfig {
    /// The language vocabulary; `None` leaves the retag map empty.
    pub language_vocab: Option<LanguageVocab>,
    /// Extra `(prefix, namespace)` CURIE entries. An IRI compacts under the
    /// LONGEST namespace any entry or the built-in W3C table names; an entry
    /// rebinds a built-in prefix of the same name, and of two entries naming one
    /// prefix the first wins. Any namespace outside the built-in table
    /// (schema.org, an application ontology, …) compacts only when it is listed
    /// here.
    pub curie_prefixes: Vec<(String, String)>,
}

/// A term projected to a consumer-facing native value: XSD numeric/boolean
/// literals are parsed, IRIs are CURIE-compacted, and everything else stays a
/// string (see [`GtsFoldView::public_value`]).
#[derive(Clone, Debug, PartialEq)]
pub enum PublicValue {
    /// An IRI, compacted to a CURIE when a known prefix matches.
    Iri(String),
    /// A blank node rendered with its `_:` prefix.
    Blank(String),
    /// A plain string literal, an unparseable typed literal, or the N-Quads
    /// token of a triple term.
    String(String),
    /// An `xsd:integer` literal with a parseable lexical form.
    Integer(i64),
    /// An `xsd:decimal`/`xsd:double`/`xsd:float` literal with a parseable
    /// lexical form.
    Float(f64),
    /// An `xsd:boolean` literal (`true`/`1` or `false`/`0`).
    Boolean(bool),
    /// A language-tagged string literal.
    LanguageString {
        /// The lexical form.
        value: String,
        /// The language tag as stored (internal `x-purrdf-*` tags are NOT
        /// retagged here).
        lang: String,
    },
}

/// One dictionary term row: `(term_id, kind, value, datatype_id, lang,
/// reifier_id, triple)` where `kind` is `0` IRI / `1` literal / `2` blank node /
/// `3` triple term.
///
/// `triple` carries a quoted-triple term's OWN `(s, p, o)` component ids when
/// the term is self-describing. It is not derivable from `reifier_id`: one
/// reifier id may bind several different triples, so a projection that offered
/// only the reifier column could not say which triple THIS term is.
///
/// # RDF 1.2 base direction is NOT in this row
///
/// It is a parallel column instead — [`term_directions`] — because this alias is
/// published API and callers unpack it POSITIONALLY. Widening the tuple is a
/// breaking change to every such caller, and it fails at runtime rather than at
/// the type boundary for the Python and JS surfaces that see the same row. A
/// parallel column is a worse shape to read than an eighth field and a better one
/// to add, which is the whole trade: this is an additive release.
pub type TermRow = (
    usize,
    u8,
    Option<String>,
    Option<usize>,
    Option<String>,
    Option<usize>,
    Option<(usize, usize, usize)>,
);
/// One quad row of dictionary term ids: `(subject, predicate, object, graph?)`.
pub type QuadRow = (usize, usize, usize, Option<usize>);
/// One statement-layer reifier row: `(reifier_id, subject, predicate, object,
/// graph?)`. The graph slot is the term id of the named graph the declaration was
/// asserted in, `None` for the default graph — the same axis [`QuadRow`] carries,
/// because the RDF 1.2 statement layer is keyed per graph.
pub type ReifierRow = (usize, usize, usize, usize, Option<usize>);
/// One statement-layer annotation row: `(reifier_id, predicate, value, graph?)`;
/// the graph slot is the one described on [`ReifierRow`].
pub type AnnotationRow = (usize, usize, usize, Option<usize>);
/// One decoded blob row: `(digest, payload bytes)`.
pub type BlobRow = (String, Vec<u8>);

/// The RDF 1.2 base direction of every term, positionally parallel to
/// [`RelationalRows::terms`].
///
/// `Some("ltr")` / `Some("rtl")` for a directional language-tagged literal, `None`
/// for everything else. Index `i` describes the term whose id is `i`, which is the
/// same index [`relational_rows`] assigns, so the two zip.
///
/// # Why this is not an eighth column on `TermRow`
///
/// Direction genuinely belongs beside `lang`, and putting it there is a breaking
/// change: [`TermRow`] is a published tuple alias that callers unpack positionally,
/// and the Python and JS surfaces expose the same row shape where the failure is at
/// runtime rather than at a type boundary. So it is added ALONGSIDE rather than
/// inside — a worse shape to read, a safe one to ship.
///
/// The term model has always carried direction; only the projection omitted it, and
/// `lang` is stored bare (`en`) with `@en--ltr` recombined for display, so a caller
/// reading `lang` alone silently loses it. Two literals differing only in direction
/// are DIFFERENT terms under the canonicalization profile, so that loss conflates.
#[must_use]
pub fn term_directions(graph: &Graph) -> Vec<Option<String>> {
    graph
        .terms
        .iter()
        .map(|term| term.direction.clone())
        .collect()
}

/// The compact dictionary-encoded relational projection of a folded GTS graph:
/// term dictionary, quads, statement-layer rows, and decoded blobs, all keyed by
/// dictionary term ids (see [`relational_rows`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationalRows {
    /// The term dictionary, one [`TermRow`] per term id in order.
    pub terms: Vec<TermRow>,
    /// All quads as dictionary-id tuples.
    pub quads: Vec<QuadRow>,
    /// The statement-layer reifier rows, each carrying the graph it was declared in.
    pub reifiers: Vec<ReifierRow>,
    /// The statement-layer annotation rows, each carrying the graph it was asserted in.
    pub annotations: Vec<AnnotationRow>,
    /// The decoded blob payloads, keyed by content digest.
    pub blobs: Vec<BlobRow>,
}

/// Rust-owned read-side view over a folded GTS [`Graph`]: scoped quad lookup,
/// term accessors, language-tag projection, RDF list walking, statement-layer
/// access, and the relational projection.
///
/// Construction builds an IRI→term-id index and per-scope subject/predicate
/// indexes once; all lookups are then index-backed and deterministic.
#[derive(Debug)]
pub struct GtsFoldView {
    graph: Graph,
    iri_index: BTreeMap<String, usize>,
    spo: BTreeMap<ScopeKey, BTreeMap<usize, Vec<(usize, usize)>>>,
    po: BTreeMap<ScopeKey, BTreeMap<(usize, usize), Vec<usize>>>,
    tag_map: BTreeMap<String, String>,
    /// The built-in table overlaid with the caller's prefixes.
    curie_prefixes: PrefixMap,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ScopeKey {
    Default,
    All,
    Named(usize),
}

/// Refuse a caller-assembled graph whose language tags the RDF concrete-syntax
/// grammar cannot read.
///
/// The reader gates `"l"` at its single decode point, so a graph off the wire
/// cannot reach here carrying an unreadable tag. A graph a caller assembled can:
/// `Graph.terms` and `Term::lang` are both `pub`, so building one field by field
/// bypasses the reader entirely. That is the same door the Python `from_parts`
/// constructor opens, and the reason it is gated there — this is its Rust twin,
/// and leaving one open while closing the other would mean the language a fold
/// view can render depends on which binding assembled it.
///
/// It refuses at construction for exactly the reason the self-reaching-term check
/// above does: every renderer built on [`GtsFoldView::nq_token`] would otherwise
/// emit `"x"@en us`, a token no parser reads, and catching it once here beats a
/// guard inside each renderer. `with_config` already returns `Result`, so this
/// costs no caller a signature change.
fn ensure_language_tags_readable(graph: &Graph) -> Result<(), RdfDiagnostic> {
    for (tid, term) in graph.terms.iter().enumerate() {
        let Some(tag) = term.lang.as_deref() else {
            continue;
        };
        if let Some(code) = language_tag_refusal(tag) {
            return Err(RdfDiagnostic::error(
                "gts-unreadable-language-tag",
                format!(
                    "GTS term {tid} carries a language tag the RDF concrete-syntax grammar \
                     refuses ({code}): {tag:?} — every rendering of this term would emit a \
                     token no parser reads, so the view refuses to exist"
                ),
            ));
        }
    }
    Ok(())
}

impl GtsFoldView {
    /// A view with no consumer vocabulary: the retag map stays empty (no
    /// namespace scanning is fabricated) and CURIE compaction uses only the
    /// built-in W3C Recommendation prefixes. Pass a [`GtsFoldViewConfig`] via
    /// [`GtsFoldView::with_config`] to supply the consumer's language vocab
    /// and CURIE prefixes.
    ///
    /// # Errors
    ///
    /// Returns `gts-self-reaching-term` when the graph's term table lets a term
    /// resolve through itself, and `gts-unreadable-language-tag` when a term
    /// carries a language tag the RDF concrete-syntax grammar refuses — see
    /// [`GtsFoldView::with_config`].
    pub fn new(graph: Graph) -> Result<Self, RdfDiagnostic> {
        Self::with_config(graph, GtsFoldViewConfig::default())
    }

    /// A view configured with the consumer's [`GtsFoldViewConfig`].
    ///
    /// # Errors
    ///
    /// Returns `gts-self-reaching-term` when the graph's term table lets a term
    /// resolve through itself. The view's accessors — [`GtsFoldView::nq_token`],
    /// [`GtsFoldView::public_value`] and everything built on them — walk a quoted
    /// triple's resolved components to the leaves, so a self-reaching term would
    /// recurse without bound and abort the process. A graph off the wire cannot
    /// contain one (the reader refuses the row that would close the loop); a graph
    /// a caller assembled can, so the view REFUSES to exist rather than handing
    /// back an object whose every renderer is a process kill. This is the fold-time
    /// refusal GTS-SPEC §7.3 permits, applied once at construction rather than as a
    /// guard inside every walk.
    ///
    /// Returns `gts-unreadable-language-tag` when a term's `lang` is a tag the RDF
    /// concrete-syntax grammar refuses. The reasoning is the same shape: the reader
    /// gates the wire format's `"l"` field, so a graph off the wire cannot carry
    /// one, but `Graph.terms` and `Term::lang` are `pub` and a caller can assemble
    /// one field by field. Left standing, every renderer built on
    /// [`GtsFoldView::nq_token`] would emit a token no parser reads.
    pub fn with_config(graph: Graph, config: GtsFoldViewConfig) -> Result<Self, RdfDiagnostic> {
        ensure_terms_terminate(&graph)?;
        ensure_language_tags_readable(&graph)?;
        let mut view = Self {
            graph,
            iri_index: BTreeMap::new(),
            spo: BTreeMap::new(),
            po: BTreeMap::new(),
            tag_map: BTreeMap::new(),
            curie_prefixes: curie_prefix_map(config.curie_prefixes),
        };
        view.build_iri_index();
        view.build_quad_indexes();
        if let Some(vocab) = &config.language_vocab {
            view.tag_map = view.build_tag_map(vocab);
        }
        Ok(view)
    }

    /// Consume the view and return the underlying folded [`Graph`].
    pub fn into_graph(self) -> Graph {
        self.graph
    }

    /// The underlying folded [`Graph`].
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// The dictionary [`Term`] for the given term id.
    ///
    /// # Panics
    ///
    /// Panics if `tid` is out of range for the graph's term dictionary.
    pub fn term(&self, tid: usize) -> &Term {
        &self.graph.terms[tid]
    }

    /// Whether the term is an IRI.
    pub fn is_iri(&self, tid: usize) -> bool {
        self.term(tid).kind == TermKind::Iri
    }

    /// Whether the term is a blank node.
    pub fn is_bnode(&self, tid: usize) -> bool {
        self.term(tid).kind == TermKind::Bnode
    }

    /// Whether the term is a literal.
    pub fn is_literal(&self, tid: usize) -> bool {
        self.term(tid).kind == TermKind::Literal
    }

    /// The IRI value of the term, or `None` if the term is not an IRI.
    pub fn iri(&self, tid: usize) -> Option<&str> {
        let term = self.term(tid);
        (term.kind == TermKind::Iri)
            .then_some(term.value.as_deref())
            .flatten()
    }

    /// The lexical value of the term (empty string when the term has none).
    pub fn lex(&self, tid: usize) -> &str {
        self.term(tid).value.as_deref().unwrap_or("")
    }

    /// The language tag of the term, if any.
    pub fn lang(&self, tid: usize) -> Option<&str> {
        self.term(tid).lang.as_deref()
    }

    /// The datatype IRI of the term (as resolved by the graph's dictionary).
    pub fn datatype(&self, tid: usize) -> String {
        self.graph.datatype_iri(self.term(tid))
    }

    /// The N-Quads token for the term (`<iri>`, `_:label`, quoted literal, or
    /// `<<( s p o )>>` for a triple term).
    pub fn nq_token(&self, tid: usize) -> String {
        render_term(&self.graph, tid)
    }

    /// Project the term to a consumer-facing [`PublicValue`]: XSD
    /// integer/float/boolean literals are parsed (falling back to
    /// [`PublicValue::String`] on an unparseable lexical form), IRIs are
    /// CURIE-compacted, and language-tagged literals keep their stored tag.
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
            TermKind::Iri => PublicValue::Iri(self.curie(self.lex(tid))),
            TermKind::Bnode => PublicValue::Blank(format!("_:{}", self.lex(tid))),
            TermKind::Triple => PublicValue::String(self.nq_token(tid)),
        }
    }

    /// The dictionary term id of an IRI, or `None` if the graph has no such
    /// IRI term. When the dictionary contains duplicates the lowest id wins.
    pub fn tid_of_iri(&self, iri: &str) -> Option<usize> {
        self.iri_index.get(iri).copied()
    }

    /// Compact an IRI to a CURIE under the longest matching namespace of the
    /// consumer-supplied prefixes and the built-in W3C Recommendation table
    /// ([`purrdf_iri::contract`]); unmatched IRIs are returned unchanged.
    pub fn curie(&self, iri: &str) -> String {
        purrdf_iri::contract(iri, &self.curie_prefixes).unwrap_or_else(|| iri.to_owned())
    }

    /// All quads visible in a scope. `scope` is `None` for the default graph,
    /// [`ALL_SCOPE`] for every graph, or a named-graph IRI; an unknown graph
    /// IRI yields no quads.
    pub fn quads(&self, scope: Option<&str>) -> Vec<Quad> {
        let Some(key) = self.scope_key(scope) else {
            return Vec::new();
        };
        if key == ScopeKey::All {
            return self.graph.quads.clone();
        }
        let graph_scope = graph_term_for_scope(key);
        self.spo.get(&key).map_or_default(|subjects| {
            let mut rows = Vec::new();
            for (&s, pairs) in subjects {
                for &(p, o) in pairs {
                    rows.push((s, p, o, graph_scope));
                }
            }
            rows
        })
    }

    /// The (sorted, deduplicated) subject term ids typed `rdf:type
    /// <class_iri>` within the scope.
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

    /// The (sorted, deduplicated) object term ids of `(s_tid, <p_iri>, ?o)`
    /// within the scope.
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

    /// A single deterministic object of `(s_tid, <p_iri>, ?o)`: the candidate
    /// whose N-Quads token sorts first, or `None` when there is no match.
    pub fn value(&self, s_tid: usize, p_iri: &str, scope: Option<&str>) -> Option<usize> {
        self.objects(s_tid, p_iri, scope)
            .into_iter()
            .map(|tid| (self.nq_token(tid), tid))
            .min_by(|(left, _), (right, _)| left.cmp(right))
            .map(|(_, tid)| tid)
    }

    /// All (sorted, deduplicated) `(predicate, object)` term-id pairs asserted
    /// about a subject within the scope.
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

    /// Whether the quad `(s_tid, <p_iri>, o_tid)` is asserted within the scope.
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

    /// The members, in order, of the RDF Collection headed by `head_tid` within
    /// the scope, read by the strict walker
    /// ([`walk_rdf_list`](purrdf_core::collections::walk_rdf_list)) every
    /// reader of a collection shares.
    ///
    /// A well-formed list yields all its members. A malformed one yields the
    /// members read before the walk stopped ([`ListError::members`](purrdf_core::ListError)):
    /// a cycle ends at the revisited cell, and a cell with no or several
    /// `rdf:first`, or no or several `rdf:rest`, ends the list there rather
    /// than having one of its values picked. A head that is `rdf:nil`, or no
    /// list at all, yields nothing.
    pub fn rdf_list(&self, head_tid: usize, scope: Option<&str>) -> Vec<usize> {
        let Some(key) = self.scope_key(scope) else {
            return Vec::new();
        };
        let (first, rest) = (self.tid_of_iri(RDF_FIRST), self.tid_of_iri(RDF_REST));
        walk_rdf_list(
            head_tid,
            self.tid_of_iri(RDF_NIL),
            |cell| self.sole_object(cell, first, key),
            |cell| self.sole_object(cell, rest, key),
        )
        .unwrap_or_else(|error| error.members)
    }

    /// How many distinct objects `(s_tid, p_tid, ?o)` has within the scope,
    /// counted up to two without allocating.
    fn sole_object(&self, s_tid: usize, p_tid: Option<usize>, key: ScopeKey) -> SoleObject<usize> {
        let Some(p_tid) = p_tid else {
            return SoleObject::None;
        };
        SoleObject::of(
            self.spo
                .get(&key)
                .and_then(|idx| idx.get(&s_tid))
                .into_iter()
                .flatten()
                .filter(|&&(p, _)| p == p_tid)
                .map(|&(_, o)| o),
        )
    }

    /// The purrdf-gts 0.9.11 reifier rows `(reifier_id, (s,p,o), graph?)`. purrdf's
    /// statement layer is keyed per graph, so the graph slot is the term id of the
    /// named graph the reifier was declared in, and `None` names the default graph.
    /// A caller that reads a row must read its graph slot too: one reifier id may be
    /// declared in several graphs, and those are distinct rows.
    pub fn reifiers(&self) -> &[(usize, Triple3, Option<usize>)] {
        &self.graph.reifiers
    }

    /// The 0.9.11 annotation rows `(reifier, predicate, value, graph?)`; the graph slot
    /// carries the same per-graph key as [`Self::reifiers`] (`None` = default graph).
    pub fn annotations(&self) -> &[(usize, usize, usize, Option<usize>)] {
        &self.graph.annotations
    }

    /// The internal→BCP-47 retag map built from the consumer's
    /// [`LanguageVocab`]; empty when the view was constructed without one.
    pub fn tag_map(&self) -> &BTreeMap<String, String> {
        &self.tag_map
    }

    /// The set of public (lowercased BCP-47) language tags present on literals
    /// in the graph, with internal `x-purrdf-*` tags retagged through the tag
    /// map. Always contains `"en"`.
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
                    .unwrap_or_else(|| lang.clone())
            } else {
                lang.clone()
            };
            if !public.eq_ignore_ascii_case("en") {
                tags.insert(identity_fold(&public));
            }
        }
        tags
    }

    /// The text of [`GtsFoldView::public_literal`], dropping the language tag.
    pub fn public_text(&self, s_tid: usize, p_iri: &str, scope: Option<&str>) -> String {
        self.public_literal(s_tid, p_iri, scope).0
    }

    /// The preferred literal of `(s_tid, <p_iri>, ?o)` as `(text, public
    /// BCP-47 tag)`. Internally tagged (`x-purrdf-*`) candidates with a retag
    /// mapping win (English-first); otherwise the deterministically first
    /// candidate is returned with its public tag. Returns `(String::new(),
    /// None)` when no literal matches.
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
            if let Some(lang) = self.lang(tid)
                && is_internal_tag(lang)
                && let Some(public) = self.tag_map.get(lang)
            {
                return (self.lex(tid).to_string(), Some(public.clone()));
            }
        }
        let first = candidates[0];
        let public = self.public_bcp47_for(first);
        (self.lex(first).to_string(), public)
    }

    /// The literal of `(s_tid, <p_iri>, ?o)` best matching the requested
    /// language tags (defaulting to `["en"]` when empty), as `(text, public
    /// BCP-47 tag, is_fallback)`. When no requested tag matches, falls back to
    /// English, then the best other tagged candidate, then an untagged one —
    /// with the flag set to `true`.
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

    /// All literals of `(s_tid, <p_iri>, ?o)` matching the requested language
    /// tags (defaulting to `["en"]` when empty), each as `(text, public BCP-47
    /// tag, is_fallback)`. When no requested tag matches, at most one fallback
    /// row is returned using the [`GtsFoldView::public_literal_with_fallback`]
    /// ordering.
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

    /// Project the underlying graph to [`RelationalRows`] (see
    /// [`relational_rows`]).
    ///
    /// # Errors
    ///
    /// Returns an error message when a blob payload cannot be decoded.
    pub fn relational_rows(&self) -> Result<RelationalRows, String> {
        relational_rows(&self.graph)
    }

    fn build_iri_index(&mut self) {
        for (tid, term) in self.graph.terms.iter().enumerate() {
            if term.kind == TermKind::Iri
                && let Some(value) = &term.value
            {
                self.iri_index.entry(value.clone()).or_insert(tid);
            }
        }
    }

    fn build_quad_indexes(&mut self) {
        for &(s, p, o, g) in &self.graph.quads {
            let scope = g.map_or(ScopeKey::Default, ScopeKey::Named);
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

    fn build_tag_map(&self, vocab: &LanguageVocab) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for lang_tid in self.subjects_by_type(&vocab.language_class, Some(ALL_SCOPE)) {
            let internal = self.value(lang_tid, &vocab.language_tag, Some(ALL_SCOPE));
            let bcp = self.value(lang_tid, &vocab.bcp47_tag, Some(ALL_SCOPE));
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
            let key = identity_fold(bcp.as_deref().unwrap_or(""));
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
        requested.iter().map(|tag| identity_fold(tag)).collect()
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

/// Project a folded GTS [`Graph`] to the compact dictionary-encoded
/// [`RelationalRows`] view: the full term dictionary, all quads, the
/// statement-layer reifier/annotation rows (each with the graph it was asserted in,
/// as [`ReifierRow`] describes), and every blob decoded to bytes.
///
/// # Errors
///
/// Returns an error message when a blob payload cannot be decoded.
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
                    term.triple,
                )
            })
            .collect(),
        quads: graph.quads.clone(),
        // Widen the 0.9.11 reifier row-array's nested triple into flat columns. The
        // graph slot is CARRIED, exactly as it is for quads: the statement layer is
        // keyed per graph, so one reifier id may be declared and annotated
        // independently in several graphs, and a relational view that dropped the
        // column would collapse those rows into one indistinguishable heap.
        reifiers: graph
            .reifiers
            .iter()
            .map(|&(r, (s, p, o), g)| (r, s, p, o, g))
            .collect(),
        annotations: graph.annotations.clone(),
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
    let lower = identity_fold(lang);
    let Some(suffix) = lower.strip_prefix("x-purrdf-") else {
        return false;
    };
    !suffix.is_empty()
        && suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn rank_language(lang: &str) -> (u8, String) {
    let lower = identity_fold(lang);
    let rank = u8::from(lower != "x-purrdf-english");
    (rank, lower)
}

fn best_tagged(by_bcp: &BTreeMap<String, Vec<LitRow>>) -> Option<(&str, &LitRow)> {
    by_bcp
        .iter()
        .filter(|(tag, rows)| !tag.is_empty() && !rows.is_empty())
        .map(|(tag, rows)| (tag.as_str(), &rows[0]))
        .min_by(|a, b| rank_language(&a.1.2).cmp(&rank_language(&b.1.2)))
}

/// Render one term as its canonical N-Quads token ([`term_syntax`]), resolving a quoted
/// triple's components to the leaves.
///
/// A quoted triple is written over a work list: its opening `<<( ` at once, then its
/// subject next, with the separators, the predicate, the object and the closing ` )>>`
/// held back in that order until the subject's whole nesting is written.
///
/// # Termination
///
/// The walk follows a quoted triple's `(s, p, o)` with no depth bound and no visited set,
/// because the graph it walks has already been proven to terminate. Every route to a
/// `Graph` inside this module runs through [`GtsFoldView::with_config`], which
/// refuses a self-reaching term table outright. Keep it that way: a new constructor
/// that skips that check hands this function an input it would never finish writing.
fn render_term(graph: &Graph, tid: usize) -> String {
    enum Piece {
        Term(usize),
        Text(&'static str),
    }
    let mut out = String::new();
    let mut held: Vec<Piece> = Vec::new();
    let mut next = Some(Piece::Term(tid));
    while let Some(piece) = next.take().or_else(|| held.pop()) {
        let tid = match piece {
            Piece::Text(text) => {
                out.push_str(text);
                continue;
            }
            Piece::Term(tid) => tid,
        };
        let term = &graph.terms[tid];
        match term.kind {
            TermKind::Iri => term_syntax::write_iri(term.value.as_deref().unwrap_or(""), &mut out),
            TermKind::Bnode => match term.value.as_ref() {
                Some(value) => term_syntax::write_blank(value, &mut out),
                None => {
                    let _ = write!(out, "_:b{tid}");
                }
            },
            // A tagged literal's datatype is implied by its tag and never written, so
            // only an untagged one resolves its datatype term (GTS-SPEC §7.1 defaults
            // an absent one to `xsd:string`, which the canonical form leaves bare).
            TermKind::Literal => term_syntax::write_literal(
                term.value.as_deref().unwrap_or(""),
                term.datatype
                    .and_then(|datatype| graph.terms.get(datatype))
                    .and_then(|datatype| datatype.value.as_deref())
                    .unwrap_or(XSD_STRING),
                term.lang.as_deref(),
                term.lang.as_ref().and(term.direction.as_deref()),
                &mut out,
            ),
            TermKind::Triple => match graph.triple_of(tid) {
                Some((s, p, o)) => {
                    out.push_str(term_syntax::TRIPLE_TERM_OPEN);
                    out.push(' ');
                    held.extend([
                        Piece::Text(term_syntax::TRIPLE_TERM_CLOSE),
                        Piece::Text(" "),
                        Piece::Term(o),
                        Piece::Text(" "),
                        Piece::Term(p),
                        Piece::Text(" "),
                    ]);
                    next = Some(Piece::Term(s));
                }
                None => {
                    let _ = write!(out, "_:unbound_triple_{tid}");
                }
            },
        }
    }
    out
}

/// Built-in CURIE prefixes: W3C Recommendation namespaces only. Every other
/// namespace (schema.org, an application ontology, …) is caller-supplied via
/// [`GtsFoldViewConfig::curie_prefixes`], which may also rebind these prefixes.
const PREFIXES: &[(&str, &str)] = &[
    ("rdf", rdf::NS),
    ("rdfs", rdfs::NS),
    ("owl", owl::NS),
    ("xsd", XSD),
    ("skos", skos::NS),
];

/// [`PREFIXES`] overlaid with the caller's entries: a caller prefix rebinds a
/// built-in of the same name, and of two caller entries naming one prefix the
/// first is kept.
fn curie_prefix_map(caller: Vec<(String, String)>) -> PrefixMap {
    let mut map: PrefixMap = PREFIXES.iter().copied().collect();
    for (prefix, namespace) in caller.into_iter().rev() {
        map.insert(prefix, namespace);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_gts::model::Term;
    use purrdf_gts::writer::Writer;

    const EX: &str = "https://example.org/";
    use purrdf_iri::vocab::rdfs::LABEL as RDFS_LABEL;

    fn iri(value: &str) -> Term {
        Term {
            kind: TermKind::Iri,
            value: Some(value.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
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
            triple: None,
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
            triple: None,
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
            .expect("the fixture graph's terms terminate")
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
        let view = GtsFoldView::new(purrdf_gts::reader::read(&writer.to_bytes(), true, None))
            .expect("the fixture graph's terms terminate");
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

    /// A view over `quads` among the terms `cat`, `dog`, `_:l1`, `_:l2`,
    /// `rdf:first`, `rdf:rest`, `rdf:nil` (ids 0 to 6).
    fn list_view(quads: &[(usize, usize, usize, Option<usize>)]) -> GtsFoldView {
        let mut writer = Writer::new("dist");
        writer.add_terms(&[
            iri(&(EX.to_string() + "cat")),
            iri(&(EX.to_string() + "dog")),
            bnode("l1"),
            bnode("l2"),
            iri(RDF_FIRST),
            iri(RDF_REST),
            iri(RDF_NIL),
        ]);
        writer.add_quads(quads);
        GtsFoldView::new(purrdf_gts::reader::read(&writer.to_bytes(), true, None))
            .expect("the fixture graph's terms terminate")
    }

    /// The strict walk ends a malformed list where it breaks: at a cycle, at a
    /// cell with two `rdf:first` values (neither is picked), and at a cell with
    /// none; a well-formed neighbour reads whole.
    #[test]
    fn rdf_list_stops_where_the_collection_is_malformed() {
        let whole = list_view(&[
            (2, 4, 0, None),
            (2, 5, 3, None),
            (3, 4, 1, None),
            (3, 5, 6, None),
        ]);
        assert_eq!(whole.rdf_list(2, None), vec![0, 1]);
        let cycle = list_view(&[
            (2, 4, 0, None),
            (2, 5, 3, None),
            (3, 4, 1, None),
            (3, 5, 2, None),
        ]);
        assert_eq!(cycle.rdf_list(2, None), vec![0, 1]);
        let ambiguous = list_view(&[
            (2, 4, 0, None),
            (2, 5, 3, None),
            (3, 4, 0, None),
            (3, 4, 1, None),
            (3, 5, 6, None),
        ]);
        assert_eq!(ambiguous.rdf_list(2, None), vec![0]);
        let missing = list_view(&[(2, 4, 0, None), (2, 5, 3, None), (3, 5, 6, None)]);
        assert_eq!(missing.rdf_list(2, None), vec![0]);
        assert_eq!(whole.rdf_list(6, None), Vec::<usize>::new());
        assert_eq!(whole.rdf_list(0, None), Vec::<usize>::new());
    }

    #[test]
    fn language_boundary_reads_tag_map_across_all_scopes() {
        // The language vocabulary is CALLER-supplied: this fixture mints its
        // terms in the example.org namespace and hands them to the view.
        let vocab = LanguageVocab::for_namespace(EX);
        let mut writer = Writer::new("dist");
        writer.add_terms(&[
            iri(RDF_TYPE),
            iri(&vocab.language_class),
            iri(&vocab.language_tag),
            iri(&vocab.bcp47_tag),
            iri(RDFS_LABEL),
            iri(&(EX.to_string() + "English")),
            lit("x-purrdf-english", Some("en"), None),
            lit("en", Some("en"), None),
            iri(&(EX.to_string() + "French")),
            lit("x-purrdf-french", Some("en"), None),
            lit("fr", Some("en"), None),
            iri(&(EX.to_string() + "Thing")),
            lit("Hello", Some("x-purrdf-english"), None),
            lit("Bonjour", Some("x-purrdf-french"), None),
            iri(&(EX.to_string() + "graph/languages")),
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
        let bytes = writer.to_bytes();
        let view = GtsFoldView::with_config(
            purrdf_gts::reader::read(&bytes, true, None),
            GtsFoldViewConfig {
                language_vocab: Some(vocab),
                curie_prefixes: vec![("ex".to_string(), EX.to_string())],
            },
        )
        .expect("the fixture graph's terms terminate");
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
        // The caller-supplied CURIE prefix is honoured ahead of the built-ins.
        assert_eq!(view.curie(&(EX.to_string() + "Thing")), "ex:Thing");

        // With NO vocab configured the retag map is empty — no fabricated
        // namespace scanning happens.
        let bare = GtsFoldView::new(purrdf_gts::reader::read(&bytes, true, None))
            .expect("the fixture graph's terms terminate");
        assert!(bare.tag_map().is_empty());
    }

    /// A one-term graph holding the schema.org `Person` class IRI.
    fn schema_org_graph() -> Graph {
        let mut writer = Writer::new("dist");
        writer.add_terms(&[iri("https://schema.org/Person")]);
        purrdf_gts::reader::read(&writer.to_bytes(), true, None)
    }

    #[test]
    fn a_schema_org_iri_stays_a_full_iri_without_a_caller_prefix() {
        let view = GtsFoldView::new(schema_org_graph()).expect("terms terminate");
        assert_eq!(
            view.curie("https://schema.org/Person"),
            "https://schema.org/Person"
        );
        assert_eq!(
            view.public_value(0),
            PublicValue::Iri("https://schema.org/Person".to_string())
        );
    }

    #[test]
    fn a_schema_org_iri_compacts_under_a_caller_supplied_prefix() {
        let view = GtsFoldView::with_config(
            schema_org_graph(),
            GtsFoldViewConfig {
                language_vocab: None,
                curie_prefixes: vec![("schema".to_string(), "https://schema.org/".to_string())],
            },
        )
        .expect("terms terminate");
        assert_eq!(view.curie("https://schema.org/Person"), "schema:Person");
        assert_eq!(
            view.public_value(0),
            PublicValue::Iri("schema:Person".to_string())
        );
    }

    #[test]
    fn w3c_recommendation_iris_compact_without_a_caller_prefix() {
        let view = GtsFoldView::new(schema_org_graph()).expect("terms terminate");
        assert_eq!(view.curie(RDFS_LABEL), "rdfs:label");
        assert_eq!(view.curie(RDF_TYPE), "rdf:type");
        assert_eq!(view.curie(&(XSD.to_string() + "integer")), "xsd:integer");
        assert_eq!(
            view.curie("http://www.w3.org/2002/07/owl#Class"),
            "owl:Class"
        );
        assert_eq!(
            view.curie("http://www.w3.org/2004/02/skos/core#Concept"),
            "skos:Concept"
        );
    }

    #[test]
    fn relational_rows_keep_dictionary_ids() {
        let view = test_view();
        let rows = view.relational_rows().expect("relational rows");
        assert_eq!(rows.terms.len(), view.graph().terms.len());
        assert_eq!(rows.quads.len(), view.graph().quads.len());
        assert_eq!(rows.reifiers, vec![(16, 0, 1, 2, None)]);
        assert_eq!(rows.annotations, vec![(16, 17, 18, None)]);
        assert_eq!(rows.blobs, [] as [_; 0]);
    }

    /// The relational projection keeps the statement layer's graph column. One
    /// reifier id declared and annotated in two graphs must yield rows that a
    /// consumer can still tell apart; flattening the column would collapse them into
    /// an indistinguishable heap.
    #[test]
    fn relational_rows_keep_the_statement_layers_graph_column() {
        let mut writer = Writer::new("dist");
        writer.add_terms(&[
            iri(&(EX.to_string() + "a")),
            iri(&(EX.to_string() + "related")),
            iri(&(EX.to_string() + "b")),
            iri(&(EX.to_string() + "c")),
            iri(&(EX.to_string() + "r1")),
            iri(&(EX.to_string() + "source")),
            iri(&(EX.to_string() + "ledger")),
            iri(&(EX.to_string() + "elsewhere")),
            iri(&(EX.to_string() + "g1")),
            iri(&(EX.to_string() + "g2")),
        ]);
        writer.add_quads(&[(0, 1, 2, Some(8)), (0, 1, 2, Some(9))]);
        // ONE reifier id, declared about the same triple in two graphs — two distinct
        // statement-layer rows that only the graph column tells apart.
        writer.add_reifies(&[(4, (0, 1, 2), Some(8)), (4, (0, 1, 2), Some(9))]);
        writer.add_annot(&[(4, 5, 6, Some(8)), (4, 5, 7, Some(9))]);
        let view = GtsFoldView::new(purrdf_gts::reader::read(&writer.to_bytes(), true, None))
            .expect("the fixture graph's terms terminate");

        let rows = view.relational_rows().expect("relational rows");
        assert_eq!(
            rows.reifiers,
            vec![(4, 0, 1, 2, Some(8)), (4, 0, 1, 2, Some(9))]
        );
        assert_eq!(
            rows.annotations,
            vec![(4, 5, 6, Some(8)), (4, 5, 7, Some(9))]
        );
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The N-Quads token renderer against its recursive reference, and at a hundred
    //! thousand levels on a 128 KiB thread.

    use core::convert::Infallible;

    use purrdf_core::{Nested, TermValue, try_fold_nested};
    use purrdf_gts::model::{Graph, Term, TermKind};

    use purrdf_lex::term_syntax::{write_blank, write_iri, write_literal};
    use purrdf_xsd::datatype::XSD_STRING;

    use super::render_term;

    /// Lower `value` into `graph`'s term table, a triple term naming its own components,
    /// and return its term id.
    fn lower(graph: &mut Graph, value: &TermValue) -> usize {
        fn push(graph: &mut Graph, kind: TermKind, value: Option<String>) -> usize {
            graph.terms.push(Term {
                kind,
                value,
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            });
            graph.terms.len() - 1
        }
        let lowered = try_fold_nested(
            value,
            graph,
            |graph, value| {
                Ok::<_, Infallible>(Nested::Leaf(match value {
                    TermValue::Iri(iri) => push(graph, TermKind::Iri, Some(iri.clone())),
                    TermValue::Blank { label, .. } => {
                        push(graph, TermKind::Bnode, Some(label.clone()))
                    }
                    TermValue::Literal {
                        lexical_form,
                        datatype,
                        language,
                        direction,
                    } => {
                        let datatype = language
                            .is_none()
                            .then(|| push(graph, TermKind::Iri, Some(datatype.clone())));
                        let id = push(graph, TermKind::Literal, Some(lexical_form.clone()));
                        let term = &mut graph.terms[id];
                        term.datatype = datatype;
                        term.lang.clone_from(language);
                        term.direction = direction.map(|d| d.as_str().to_owned());
                        id
                    }
                    TermValue::Triple { s, p, o } => return Ok(Nested::Triple(&**s, &**p, &**o)),
                }))
            },
            |graph, _, s, p, o| {
                let id = push(graph, TermKind::Triple, None);
                graph.terms[id].triple = Some((s, p, o));
                Ok(id)
            },
        );
        match lowered {
            Ok(id) => id,
        }
    }

    /// The recursive reference of [`render_term`].
    fn reference(graph: &Graph, tid: usize) -> String {
        let term = &graph.terms[tid];
        let mut out = String::new();
        match term.kind {
            TermKind::Iri => write_iri(term.value.as_deref().unwrap_or(""), &mut out),
            TermKind::Bnode => match term.value.as_deref() {
                Some(value) => write_blank(value, &mut out),
                None => out = format!("_:b{tid}"),
            },
            TermKind::Literal => write_literal(
                term.value.as_deref().unwrap_or(""),
                term.datatype
                    .and_then(|datatype| graph.terms[datatype].value.as_deref())
                    .unwrap_or(XSD_STRING),
                term.lang.as_deref(),
                term.lang.as_ref().and(term.direction.as_deref()),
                &mut out,
            ),
            TermKind::Triple => {
                out = graph.triple_of(tid).map_or_else(
                    || format!("_:unbound_triple_{tid}"),
                    |(s, p, o)| {
                        format!(
                            "<<( {} {} {} )>>",
                            reference(graph, s),
                            reference(graph, p),
                            reference(graph, o)
                        )
                    },
                );
            }
        }
        out
    }

    /// An N-Quads token is a canonical term: an IRI's forbidden scalars ride as
    /// `UCHAR`, and a literal's body is escaped with every canonical `ECHAR`.
    #[test]
    fn the_token_escapes_iris_and_literal_bodies_canonically() {
        let mut graph = Graph::default();
        let iri = lower(
            &mut graph,
            &TermValue::Iri("http://example.org/a b<c>".to_owned()),
        );
        assert_eq!(
            render_term(&graph, iri),
            "<http://example.org/a\\u0020b\\u003Cc\\u003E>"
        );
        let plain = lower(
            &mut graph,
            &TermValue::Iri("http://example.org/a".to_owned()),
        );
        assert_eq!(render_term(&graph, plain), "<http://example.org/a>");
        let literal = lower(
            &mut graph,
            &TermValue::Literal {
                lexical_form: "q\"\u{8}\u{c}\u{85}".to_owned(),
                datatype: XSD_STRING.to_owned(),
                language: None,
                direction: None,
            },
        );
        assert_eq!(render_term(&graph, literal), "\"q\\\"\\b\\f\u{85}\"");
    }

    /// The work-list renderer spells every generated term exactly as the recursive
    /// reference does, nested triple terms included.
    #[test]
    fn the_renderer_agrees_with_its_recursive_reference_on_generated_terms() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::Any,
            );
            nested += usize::from(budget < 7);
            let mut graph = Graph::default();
            let id = lower(&mut graph, &value);
            assert_eq!(
                render_term(&graph, id),
                reference(&graph, id),
                "seed {seed}"
            );
        }
        assert!(nested > 0, "some generated term nests a triple term in one");
    }

    /// A triple term a hundred thousand levels deep is rendered on a thread whose whole
    /// stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_rendered_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let value = purrdf_core::term_fixture::triple_chain(LEVELS);
            let mut graph = Graph::default();
            let id = lower(&mut graph, &value);
            drop(value);
            let rendered = render_term(&graph, id);
            let level = "<<( <http://example.org/s> <http://example.org/p> ".len() + " )>>".len();
            assert_eq!(
                rendered.len(),
                LEVELS * level + "<http://example.org/o>".len()
            );
        })
        .expect("the thread starts");
    }
}

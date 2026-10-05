// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete typed dataset state, canonically labeled by one global bijection.
//!
//! The versioned byte grammar and canonical terminal-family proof are specified
//! in [`docs/DATASET-STATE-DIGEST.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/DATASET-STATE-DIGEST.md).

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use purrdf_hash::{Domain, blake3, frame::frame_le, hex::Digest32};
use purrdf_lex::walk::WorkList;

use super::RDFC_CALL_LIMIT;
use crate::dataset_view::{
    DrainFailure, FallibleDatasetView, TermGuard, WorkspaceReservation, checkpointed_drain,
};
use crate::ir::term_walk::{Nested, try_fold_nested};
use crate::{BlankScope, QuadIds, RdfTextDirection, TermRef, TermValue};

const STATE_DOMAIN: Domain = Domain::new(b"purrdf-core/dataset-state/v1");

/// BLAKE3 identity of the complete RDF dataset state under a global blank bijection.
///
/// Includes the default graph, empty named declarations and distinct ordinary,
/// reifier and annotation roles. Storage layout, unused terms and diagnostics do
/// not participate. This is separate from every RDFC, row and PACK identity.
/// Canonical bytes are equal exactly for isomorphic complete states; digest
/// equality additionally relies on BLAKE3's usual collision-resistance assumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DatasetStateDigest(Digest32);

impl DatasetStateDigest {
    /// Construct an identity from a complete, checkpointed view drain.
    ///
    /// Literal bytes remain exact except for bound CDT blank references, which
    /// share the global mapping with graph names and nested triple components.
    /// Exact individualization/refinement searches the invariant terminal family;
    /// it never substitutes a refinement-only approximation for canonical state.
    ///
    /// # Errors
    /// Returns typed source/admission failures, incoherent embedded identities,
    /// unrepresentable workspace or fixed canonical-search exhaustion. No partial
    /// state is certified. The fixed search bound is [`RDFC_CALL_LIMIT`]; this
    /// construction does not change the existing RDFC algorithm or its identities.
    pub fn from_view<D: FallibleDatasetView>(view: &D) -> StateResult<D, Self> {
        checkpointed_drain(view, |view| {
            let mut reservation = view.reserve_workspace(0).map_err(DatasetStateError::Read)?;
            let mut state = Captured::new(view, &mut reservation)?;
            state.collect()?;
            let bytes = state.canonical_bytes(RDFC_CALL_LIMIT)?;
            let mut hasher = blake3::Hasher::new();
            hasher.update(STATE_DOMAIN.as_bytes());
            hasher.update(&bytes);
            Ok(Self(Digest32::new(*hasher.finalize().as_bytes())))
        })
        .map_err(DatasetStateError::Operation)?
    }

    /// The 32 identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// The lowercase hexadecimal identity.
    #[must_use]
    pub fn to_hex(self) -> String {
        self.0.to_hex()
    }
}

impl fmt::Display for DatasetStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A complete-state identity refusal, distinct from legacy canonicalization errors.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DatasetStateError<Error, Evidence> {
    /// An atomic completeness checkpoint refused the entire drain.
    Operation(DrainFailure<Error, Evidence>),
    /// A point read or retained-workspace admission failed.
    Read(Error),
    /// A bound CDT reference has no coherent blank identity in the view.
    IncoherentEmbeddedBlank,
    /// A literal's datatype did not resolve to an IRI.
    InvalidTerm,
    /// The required byte or workspace size cannot be represented.
    Capacity,
    /// Exact canonical search reached its fixed work limit.
    SearchBudgetExceeded,
}

impl<E: fmt::Display, V> fmt::Display for DatasetStateError<E, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operation(failure) => write!(
                f,
                "dataset-state {:?} checkpoint failed: {}",
                failure.checkpoint, failure.error
            ),
            Self::Read(error) => write!(f, "dataset-state source or admission failed: {error}"),
            Self::IncoherentEmbeddedBlank => {
                f.write_str("dataset-state CDT reference has no coherent blank identity")
            }
            Self::InvalidTerm => f.write_str("dataset-state literal datatype is not an IRI"),
            Self::Capacity => f.write_str("dataset-state workspace size is unrepresentable"),
            Self::SearchBudgetExceeded => {
                f.write_str("dataset-state exact canonical search exhausted its fixed work limit")
            }
        }
    }
}

impl<E: std::error::Error + 'static, V: fmt::Debug> std::error::Error for DatasetStateError<E, V> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Operation(failure) => Some(&failure.error),
            Self::Read(error) => Some(error),
            _ => None,
        }
    }
}

type StateResult<D, T> = Result<
    T,
    DatasetStateError<<D as FallibleDatasetView>::Error, <D as FallibleDatasetView>::Evidence>,
>;

struct Literal {
    lexical: String,
    datatype: String,
    language: Option<String>,
    direction: Option<RdfTextDirection>,
    blanks: BTreeMap<BlankScope, BTreeMap<String, usize>>,
}

enum Node {
    Iri(String),
    Blank(usize),
    Literal(Literal),
    Triple([usize; 3]),
}

struct Term {
    node: Node,
    /// Upper bound for canonical and focus/color renderings, including CDT growth.
    bytes: u64,
}

#[derive(Clone, Copy)]
enum Record {
    Default,
    Named(usize),
    Quad {
        role: u8,
        terms: [usize; 3],
        graph: Option<usize>,
    },
}

impl Record {
    fn terms(self) -> impl Iterator<Item = usize> {
        let (terms, graph) = match self {
            Self::Default => ([None; 3], None),
            Self::Named(term) => ([Some(term), None, None], None),
            Self::Quad { terms, graph, .. } => (terms.map(Some), graph),
        };
        terms.into_iter().flatten().chain(graph)
    }
}

#[derive(Clone, Copy)]
enum Labels<'a> {
    Ordinals(&'a [usize]),
    Colors { colors: &'a [usize], focus: usize },
}

impl Labels<'_> {
    fn blank(self, blank: usize, out: &mut Vec<u8>) {
        match self {
            Self::Ordinals(labels) => out.extend_from_slice(&(labels[blank] as u64).to_le_bytes()),
            Self::Colors { colors, focus } => {
                out.push(u8::from(blank != focus));
                if blank != focus {
                    out.extend_from_slice(&(colors[blank] as u64).to_le_bytes());
                }
            }
        }
    }

    fn lexical(self, blank: usize) -> String {
        match self {
            Self::Ordinals(labels) => format!("_:s{}", labels[blank]),
            Self::Colors { focus, .. } if blank == focus => "_:f".to_owned(),
            Self::Colors { colors, .. } => format!("_:c{}", colors[blank]),
        }
    }
}

/// The arena is flat: folds, rendering, search and destruction do not recurse.
struct Captured<'a, D: FallibleDatasetView, R> {
    view: &'a D,
    reservation: &'a mut R,
    retained: u64,
    nodes: Vec<Term>,
    terms: BTreeMap<D::Id, usize>,
    blanks: BTreeMap<BlankScope, BTreeMap<String, usize>>,
    blank_count: usize,
    records: Vec<Record>,
}

impl<'a, D: FallibleDatasetView, R: WorkspaceReservation<Error = D::Error>> Captured<'a, D, R> {
    fn new(view: &'a D, reservation: &'a mut R) -> StateResult<D, Self> {
        // A memoized term fold enters each id once. This also admits the shared
        // walk's pending frames/answers before a deep traversal allocates them.
        let retained = view
            .term_count()
            .checked_mul(128)
            .and_then(|bytes| bytes.checked_add(1024))
            .ok_or(DatasetStateError::Capacity)?;
        reservation
            .resize(retained)
            .map_err(DatasetStateError::Read)?;
        Ok(Self {
            view,
            reservation,
            retained,
            nodes: Vec::new(),
            terms: BTreeMap::new(),
            blanks: BTreeMap::new(),
            blank_count: 0,
            records: Vec::new(),
        })
    }

    fn admit(&mut self, bytes: u64) -> StateResult<D, ()> {
        self.retained = self
            .retained
            .checked_add(bytes)
            .ok_or(DatasetStateError::Capacity)?;
        self.reservation
            .resize(self.retained)
            .map_err(DatasetStateError::Read)
    }

    fn blank(&mut self, label: &str, scope: BlankScope) -> StateResult<D, usize> {
        // Lookup with borrowed spelling avoids allocating an identity per occurrence.
        if let Some(&index) = self.blanks.get(&scope).and_then(|names| names.get(label)) {
            return Ok(index);
        }
        self.admit(
            (label.len() as u64)
                .checked_add(256)
                .ok_or(DatasetStateError::Capacity)?,
        )?;
        let index = self.blank_count;
        self.blanks
            .entry(scope)
            .or_default()
            .insert(label.to_owned(), index);
        self.blank_count += 1;
        Ok(index)
    }

    fn literal(
        &mut self,
        lexical: &str,
        datatype: D::Id,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> StateResult<D, Term> {
        let guard = self
            .view
            .resolve(datatype)
            .map_err(DatasetStateError::Read)?;
        let TermRef::Iri(datatype) = guard.term() else {
            return Err(DatasetStateError::InvalidTerm);
        };
        // The CDT scanner's output and its strings fit within this conservative
        // lexical-byte bound; admit them before calling the shared scanner.
        let text_bytes = [lexical.len(), datatype.len(), language.map_or(0, str::len)]
            .into_iter()
            .try_fold(0_u64, |bytes, n| bytes.checked_add(n as u64))
            .ok_or(DatasetStateError::Capacity)?;
        let scanner_factor = if crate::cdt_blank::is_cdt_datatype(datatype) {
            256
        } else {
            0
        };
        let payload = (lexical.len() as u64)
            .checked_mul(scanner_factor)
            .and_then(|n| n.checked_add(text_bytes))
            .and_then(|n| n.checked_add(256))
            .ok_or(DatasetStateError::Capacity)?;
        self.admit(payload)?;
        let embedded = crate::cdt_blank::cdt_embedded_blanks(lexical, datatype);
        let occurrences = embedded.len() as u64;
        let mut blanks = BTreeMap::<BlankScope, BTreeMap<String, usize>>::new();
        for (label, scope) in embedded {
            let value = TermValue::Blank {
                label: label.clone(),
                scope,
            };
            let id = self
                .view
                .term_id_by_value(&value)
                .map_err(DatasetStateError::Read)?
                .ok_or(DatasetStateError::IncoherentEmbeddedBlank)?;
            let guard = self.view.resolve(id).map_err(DatasetStateError::Read)?;
            if !matches!(guard.term(), TermRef::Blank { label: found, scope: s } if found == label && s == scope)
            {
                return Err(DatasetStateError::IncoherentEmbeddedBlank);
            }
            let index = self.blank(&label, scope)?;
            blanks.entry(scope).or_default().insert(label, index);
        }
        // Framing/option tags need at most64 bytes. Every replacement is ASCII
        // and at most23 bytes even for a full u64 ordinal;32 bytes per occurrence
        // bounds growth in nested typed strings without conflating byte layout
        // with the larger scanner/map workspace admission above.
        let bytes = occurrences
            .checked_mul(32)
            .and_then(|n| n.checked_add(text_bytes))
            .and_then(|n| n.checked_add(64))
            .ok_or(DatasetStateError::Capacity)?;
        Ok(Term {
            node: Node::Literal(Literal {
                lexical: lexical.to_owned(),
                datatype: datatype.to_owned(),
                language: language.map(str::to_owned),
                direction,
                blanks,
            }),
            bytes,
        })
    }

    fn term(&mut self, id: D::Id) -> StateResult<D, usize> {
        let view = self.view;
        try_fold_nested(
            id,
            self,
            |this, id| {
                if let Some(&index) = this.terms.get(&id) {
                    return Ok(Nested::Leaf(index));
                }
                this.admit(512)?;
                let guard = view.resolve(id).map_err(DatasetStateError::Read)?;
                let term = match guard.term() {
                    TermRef::Triple { s, p, o } => return Ok(Nested::Triple(s, p, o)),
                    TermRef::Iri(iri) => {
                        this.admit(iri.len() as u64)?;
                        Term {
                            node: Node::Iri(iri.to_owned()),
                            bytes: (iri.len() as u64)
                                .checked_add(9)
                                .ok_or(DatasetStateError::Capacity)?,
                        }
                    }
                    TermRef::Blank { label, scope } => Term {
                        node: Node::Blank(this.blank(label, scope)?),
                        bytes: 10,
                    },
                    TermRef::Literal {
                        lexical,
                        datatype,
                        language,
                        direction,
                    } => this.literal(lexical, datatype, language, direction)?,
                };
                Ok(Nested::Leaf(this.store(id, term)))
            },
            |this, id, s, p, o| {
                let bytes = [s, p, o]
                    .into_iter()
                    .try_fold(1_u64, |n, child| n.checked_add(this.nodes[child].bytes))
                    .ok_or(DatasetStateError::Capacity)?;
                Ok(this.store(
                    id,
                    Term {
                        node: Node::Triple([s, p, o]),
                        bytes,
                    },
                ))
            },
        )
    }

    fn store(&mut self, id: D::Id, term: Term) -> usize {
        let index = self.nodes.len();
        self.nodes.push(term);
        self.terms.insert(id, index);
        index
    }

    fn named(&mut self, id: D::Id) -> StateResult<D, ()> {
        let term = self.term(id)?;
        self.admit(128)?;
        self.records.push(Record::Named(term));
        Ok(())
    }

    fn quad(&mut self, role: u8, q: QuadIds<D::Id>) -> StateResult<D, ()> {
        let terms = [self.term(q.s)?, self.term(q.p)?, self.term(q.o)?];
        let graph = q.g.map(|id| self.term(id)).transpose()?;
        self.admit(256)?;
        if let Some(graph) = graph {
            self.records.push(Record::Named(graph));
        }
        self.records.push(Record::Quad { role, terms, graph });
        Ok(())
    }

    fn collect(&mut self) -> StateResult<D, ()> {
        self.records.push(Record::Default);
        let view = self.view;
        for graph in view.named_graphs() {
            self.named(graph)?;
        }
        for q in view.quads() {
            self.quad(2, q)?;
        }
        for q in view.reifier_quads() {
            self.quad(3, q)?;
        }
        for q in view.annotation_quads() {
            self.quad(4, q)?;
        }
        // Set semantics must precede refinement. Ground-term aliases and repeated
        // declarations/rows cannot change multiplicities in an incidence signature.
        let identity: Vec<_> = (0..self.blank_count).collect();
        let bound = self.record_bytes_bound()?;
        self.admit(bound.checked_mul(4).ok_or(DatasetStateError::Capacity)?)?;
        let mut unique = BTreeMap::new();
        for &record in &self.records {
            unique
                .entry(self.render(record, Labels::Ordinals(&identity)))
                .or_insert(record);
        }
        self.records = unique.into_values().collect();
        Ok(())
    }

    fn record_bound(&self, record: Record) -> StateResult<D, u64> {
        record
            .terms()
            .try_fold(2_u64, |bytes, term| {
                bytes.checked_add(self.nodes[term].bytes)
            })
            .ok_or(DatasetStateError::Capacity)
    }

    fn record_bytes_bound(&self) -> StateResult<D, u64> {
        self.records.iter().try_fold(8_u64, |bytes, &record| {
            bytes
                .checked_add(
                    self.record_bound(record)?
                        .checked_add(40)
                        .ok_or(DatasetStateError::Capacity)?,
                )
                .ok_or(DatasetStateError::Capacity)
        })
    }

    fn render_term(&self, root: usize, labels: Labels<'_>, out: &mut Vec<u8>) {
        let mut pending: WorkList<usize, 32> = WorkList::with(root);
        while let Some(index) = pending.pop() {
            match &self.nodes[index].node {
                Node::Iri(iri) => {
                    out.push(0);
                    frame_le(out, iri.as_bytes());
                }
                Node::Blank(blank) => {
                    out.push(1);
                    labels.blank(*blank, out);
                }
                Node::Triple([s, p, o]) => {
                    out.push(3);
                    pending.extend([*o, *p, *s]);
                }
                Node::Literal(literal) => {
                    out.push(2);
                    let lexical = crate::cdt_blank::rewrite_cdt_blank_terms(
                        &literal.lexical,
                        &literal.datatype,
                        &mut |raw| {
                            let (label, scope) = crate::blank_label::decode_blank_label(
                                raw,
                                crate::blank_label::LabelAlphabet::BlankNodeLabel,
                            );
                            let blank = literal
                                .blanks
                                .get(&scope)
                                .and_then(|names| names.get(label.as_ref()))
                                .expect("capture checked every embedded identity");
                            Some(labels.lexical(*blank))
                        },
                    );
                    frame_le(out, lexical.as_bytes());
                    frame_le(out, literal.datatype.as_bytes());
                    out.push(u8::from(literal.language.is_some()));
                    if let Some(language) = &literal.language {
                        frame_le(out, language.as_bytes());
                    }
                    out.push(u8::from(literal.direction.is_some()));
                    if let Some(direction) = literal.direction {
                        frame_le(out, direction.as_str().as_bytes());
                    }
                }
            }
        }
    }

    fn render(&self, record: Record, labels: Labels<'_>) -> Vec<u8> {
        let mut out = Vec::new();
        match record {
            Record::Default => out.push(0),
            Record::Named(term) => {
                out.push(1);
                self.render_term(term, labels, &mut out);
            }
            Record::Quad { role, terms, graph } => {
                out.push(role);
                for term in terms {
                    self.render_term(term, labels, &mut out);
                }
                out.push(u8::from(graph.is_some()));
                if let Some(graph) = graph {
                    self.render_term(graph, labels, &mut out);
                }
            }
        }
        out
    }

    fn rendered_records(&self, labels: Labels<'_>) -> Vec<Vec<u8>> {
        let mut records: Vec<_> = self
            .records
            .iter()
            .map(|&record| self.render(record, labels))
            .collect();
        records.sort_unstable();
        records
    }

    fn incidence(&self) -> Vec<Vec<usize>> {
        let mut incidence = vec![Vec::new(); self.blank_count];
        for (index, &record) in self.records.iter().enumerate() {
            let mut found = BTreeSet::new();
            let mut pending: WorkList<usize, 32> = WorkList::default();
            pending.extend(record.terms());
            while let Some(term) = pending.pop() {
                match &self.nodes[term].node {
                    Node::Blank(blank) => {
                        found.insert(*blank);
                    }
                    Node::Literal(literal) => found.extend(
                        literal
                            .blanks
                            .values()
                            .flat_map(|names| names.values().copied()),
                    ),
                    Node::Triple(parts) => pending.extend(parts.iter().copied()),
                    Node::Iri(_) => {}
                }
            }
            for blank in found {
                incidence[blank].push(index);
            }
        }
        incidence
    }

    fn refine(
        &self,
        mut colors: Vec<usize>,
        incidence: &[Vec<usize>],
        work: &mut u64,
        limit: u64,
    ) -> StateResult<D, Vec<usize>> {
        loop {
            Self::tick(work, limit)?;
            let cells = colors.iter().max().map_or(0, |n| n + 1);
            let mut next = vec![0; colors.len()];
            let mut count = 0;
            // Keep parent-cell order: a refinement only splits, never merges or
            // reorders old cells according to endian-dependent integer spellings.
            for cell in 0..cells {
                let mut groups = BTreeMap::<Vec<u8>, Vec<usize>>::new();
                for (blank, &color) in colors
                    .iter()
                    .enumerate()
                    .filter(|(_, color)| **color == cell)
                {
                    Self::tick(work, limit)?;
                    let mut records: Vec<_> = incidence[blank]
                        .iter()
                        .map(|&i| {
                            self.render(
                                self.records[i],
                                Labels::Colors {
                                    colors: &colors,
                                    focus: blank,
                                },
                            )
                        })
                        .collect();
                    records.sort_unstable();
                    let mut key = Vec::new();
                    key.extend_from_slice(&(color as u64).to_le_bytes());
                    for record in records {
                        frame_le(&mut key, &record);
                    }
                    groups.entry(key).or_default().push(blank);
                }
                for group in groups.into_values() {
                    for blank in group {
                        next[blank] = count;
                    }
                    count += 1;
                }
            }
            if count == cells {
                return Ok(next);
            }
            colors = next;
        }
    }

    fn tick(work: &mut u64, limit: u64) -> StateResult<D, ()> {
        if *work == limit {
            return Err(DatasetStateError::SearchBudgetExceeded);
        }
        *work += 1;
        Ok(())
    }

    fn automorphism(
        &self,
        a: usize,
        b: usize,
        original: &[Vec<u8>],
        identity: &mut [usize],
    ) -> bool {
        identity.swap(a, b);
        let same = self.rendered_records(Labels::Ordinals(identity)) == original;
        identity.swap(a, b);
        same
    }

    fn canonical_bytes(&mut self, limit: u64) -> StateResult<D, Vec<u8>> {
        let count = self.blank_count;
        let bound = self.record_bytes_bound()?;
        // Whole renderings, incidence keys/maps, and term-render work lists are
        // covered before allocating search scratch. Capacity overflow is refusal.
        let scratch = (count as u64)
            .checked_add(16)
            .and_then(|factor| bound.checked_mul(factor))
            .and_then(|n| n.checked_add((count as u64).checked_mul(1024)?))
            .ok_or(DatasetStateError::Capacity)?;
        self.admit(scratch)?;
        usize::try_from(scratch).map_err(|_| DatasetStateError::Capacity)?;
        let incidence = self.incidence();
        let mut identity: Vec<_> = (0..count).collect();
        let original = self.rendered_records(Labels::Ordinals(&identity));
        let mut pending: WorkList<Vec<usize>, 8> = WorkList::with(vec![0; count]);
        let mut best: Option<Vec<u8>> = None;
        let mut work = 0;
        let mut high_water = 1;
        while let Some(colors) = pending.pop() {
            Self::tick(&mut work, limit)?;
            let colors = self.refine(colors, &incidence, &mut work, limit)?;
            let mut members = Vec::new();
            for cell in 0..count {
                members = colors
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &color)| (color == cell).then_some(i))
                    .collect();
                if members.len() > 1 {
                    break;
                }
            }
            if members.len() <= 1 {
                let records = self.rendered_records(Labels::Ordinals(&colors));
                let mut bytes = Vec::new();
                bytes.extend_from_slice(&(records.len() as u64).to_le_bytes());
                for record in records {
                    frame_le(&mut bytes, &record);
                }
                if best.as_ref().is_none_or(|old| bytes < *old) {
                    best = Some(bytes);
                }
                continue;
            }
            let cell = colors[members[0]];
            let mut representatives = Vec::new();
            for &candidate in &members {
                Self::tick(&mut work, limit)?;
                let mut equivalent = false;
                for &representative in &representatives {
                    Self::tick(&mut work, limit)?;
                    if self.automorphism(candidate, representative, &original, &mut identity) {
                        equivalent = true;
                        break;
                    }
                }
                if !equivalent {
                    representatives.push(candidate);
                }
            }
            // Every checked transposition preserves the entire record set and
            // partition. If the whole cell is interchangeable, its permutations
            // yield the same terminal family; split it at once without recursion.
            let branches = if representatives.len() == 1 {
                1
            } else {
                representatives.len()
            };
            high_water = high_water.max(
                pending
                    .len()
                    .checked_add(branches)
                    .ok_or(DatasetStateError::Capacity)?,
            );
            let frame_bytes = (count as u64)
                .checked_mul(16)
                .and_then(|n| n.checked_add(128))
                .ok_or(DatasetStateError::Capacity)?;
            let search_bytes = (high_water as u64)
                .checked_add(2)
                .and_then(|n| n.checked_mul(frame_bytes))
                .and_then(|n| self.retained.checked_add(n))
                .ok_or(DatasetStateError::Capacity)?;
            self.reservation
                .resize(search_bytes)
                .map_err(DatasetStateError::Read)?;
            if representatives.len() == 1 {
                let mut next = colors;
                for color in &mut next {
                    if *color > cell {
                        *color += members.len() - 1;
                    }
                }
                for (offset, blank) in members.into_iter().enumerate() {
                    next[blank] = cell + offset;
                }
                pending.push(next);
            } else {
                for candidate in representatives {
                    let mut next = colors.clone();
                    for color in &mut next {
                        if *color >= cell {
                            *color += 1;
                        }
                    }
                    next[candidate] = cell;
                    pending.push(next);
                }
            }
        }
        Ok(best.expect("exact search has at least one terminal labeling"))
    }
}

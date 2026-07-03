// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fallible `RdfDataset` builder: value-interning plus the quad / reifier /
//! annotation / source-location tables, and the validate-then-freeze path (#819 C1).
//!
//! This module owns term storage and value-interning. The C0 literal-identity
//! policy (datatype expansion, language lowercasing, direction-in-key, verbatim
//! lexical spelling — see `docs/design/819-rdf-ir-dataflow.md` *Appendix C0.1*) is
//! applied here, at intern time, so that the frozen dataset carries fully resolved
//! identity.
//!
//! Pushing structure (`push_quad` / `push_reifier` / `push_annotation` /
//! `attach_location`) accumulates raw rows; [`RdfDatasetBuilder::freeze`] then runs
//! structural validation ([`super::validate`]) and, on success, materializes an
//! immutable, deterministically-ordered, deduplicated [`RdfDataset`]. Per the
//! no-optionality doctrine, malformed structure is a HARD failure (`Err`), never a
//! silent default.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use hashbrown::HashTable;

use crate::{
    Blake3ContentId, ContentIdScheme, RdfAnnotation, RdfLiteral, RdfQuad, RdfReifier, RdfTerm,
    RdfTextDirection,
};

use super::dataset::{FastHasher, QuadHandle, QuadIds, QuadRow, RdfDataset, TermRef};
use super::term::{
    arena_str, BlankScope, InternedLiteral, InternedTerm, StrRange, TermId, RDF_LANG_STRING,
    XSD_STRING,
};
use crate::RdfLocation;

/// A fixed-key hash of a value, so the store-once tables are deterministic across
/// runs. The frozen output is sorted by id, not hash-iteration order, so any hash
/// would do; fixed-key `AHasher` just avoids SipHash on the hot interning path.
fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = ahash::AHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

/// **Store-once** insert-or-find (#880 P3c): `vec` is the sole owner of the values;
/// `table` holds only their `u32` indices, with hash/eq that look INTO `vec`. Returns
/// the index of the (existing or newly pushed) value. No value is ever stored twice.
///
/// `vec` and `table` are distinct `&mut` params — callers pass disjoint struct fields,
/// so the find-then-push-then-insert sequence has no overlapping borrow.
fn store_once<T: Hash + Eq>(vec: &mut Vec<T>, table: &mut HashTable<u32>, value: T) -> u32 {
    let hash = hash_of(&value);
    if let Some(&i) = table.find(hash, |&i| vec[i as usize] == value) {
        return i;
    }
    let i = u32::try_from(vec.len()).expect("interner table exceeds u32::MAX entries");
    vec.push(value);
    table.insert_unique(hash, i, |&i| hash_of(&vec[i as usize]));
    i
}

/// A borrowed term lookup key (#879 P3b): carries the string components by reference
/// so the interner can dedup BY VALUE *before* anything is pushed to the arena.
/// Mirrors [`InternedTerm`] but with `&str` where the stored form holds a `StrRange`.
#[derive(Clone, Copy)]
enum TermLookup<'a> {
    Iri(&'a str),
    Blank {
        label: &'a str,
        scope: BlankScope,
    },
    Literal {
        lexical: &'a str,
        datatype: TermId,
        language: Option<&'a str>,
        direction: Option<RdfTextDirection>,
    },
    Triple {
        s: TermId,
        p: TermId,
        o: TermId,
    },
}

/// Hash a borrowed lookup. MUST hash byte-identically to [`hash_stored`] for equal
/// values — explicit discriminant tags + `str::hash` (so the find/insert hashes agree).
fn hash_lookup<H: Hasher>(lookup: &TermLookup<'_>, state: &mut H) {
    match lookup {
        TermLookup::Iri(iri) => {
            0u8.hash(state);
            iri.hash(state);
        }
        TermLookup::Blank { label, scope } => {
            1u8.hash(state);
            label.hash(state);
            scope.hash(state);
        }
        TermLookup::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            2u8.hash(state);
            lexical.hash(state);
            datatype.hash(state);
            language.hash(state);
            direction.hash(state);
        }
        TermLookup::Triple { s, p, o } => {
            3u8.hash(state);
            s.hash(state);
            p.hash(state);
            o.hash(state);
        }
    }
}

/// Hash a stored term, resolving its `StrRange`s through `arena`. MUST match
/// [`hash_lookup`] for equal values.
fn hash_stored<H: Hasher>(arena: &[u8], term: &InternedTerm, state: &mut H) {
    match term {
        InternedTerm::Iri(r) => {
            0u8.hash(state);
            arena_str(arena, *r).hash(state);
        }
        InternedTerm::Blank { label, scope } => {
            1u8.hash(state);
            arena_str(arena, *label).hash(state);
            scope.hash(state);
        }
        InternedTerm::Literal(lit) => {
            2u8.hash(state);
            arena_str(arena, lit.lexical_form).hash(state);
            lit.datatype.hash(state);
            lit.language.map(|r| arena_str(arena, r)).hash(state);
            lit.direction.hash(state);
        }
        InternedTerm::Triple { s, p, o } => {
            3u8.hash(state);
            s.hash(state);
            p.hash(state);
            o.hash(state);
        }
    }
}

/// Whether a stored term equals a lookup, resolving the stored ranges through `arena`.
fn term_eq(arena: &[u8], term: &InternedTerm, lookup: &TermLookup<'_>) -> bool {
    match (term, lookup) {
        (InternedTerm::Iri(r), TermLookup::Iri(s)) => arena_str(arena, *r) == *s,
        (
            InternedTerm::Blank { label, scope },
            TermLookup::Blank {
                label: ls,
                scope: ss,
            },
        ) => arena_str(arena, *label) == *ls && scope == ss,
        (
            InternedTerm::Literal(lit),
            TermLookup::Literal {
                lexical,
                datatype,
                language,
                direction,
            },
        ) => {
            arena_str(arena, lit.lexical_form) == *lexical
                && lit.datatype == *datatype
                && lit.direction == *direction
                && lit.language.map(|r| arena_str(arena, r)) == *language
        }
        (
            InternedTerm::Triple { s, p, o },
            TermLookup::Triple {
                s: ls,
                p: lp,
                o: lo,
            },
        ) => s == ls && p == lp && o == lo,
        _ => false,
    }
}

/// Term storage + value-interning dedup + the C0 identity policy, in one cohesive
/// unit (SRP). Private: the builder is the only public surface.
#[derive(Debug)]
struct Interner {
    /// The byte arena owning every interned string ONCE (#879 P3b); terms hold ranges.
    arena: Vec<u8>,
    /// Dense table of interned terms (range-backed); the sole structural owner.
    terms: Vec<InternedTerm>,
    /// Store-once value→id index: `u32` indices into `terms`, with hash/eq resolving
    /// ranges through the arena and comparing BY VALUE (so equal strings dedup to
    /// one id even though the table itself stores neither strings nor ranges).
    index: HashTable<u32>,
    /// The caller-supplied content-id recognition spelling (e.g. `"blake3:"`).
    /// `None` (the default) means recognition is INACTIVE: no fabricated default,
    /// per the no-vocabulary-minting policy. Fixed at construction time (see
    /// [`RdfDatasetBuilder::with_content_addressing`]) — there is no setter.
    content_scheme: Option<ContentIdScheme>,
    /// Side table from a recognized content-id term to its decoded
    /// [`Blake3ContentId`]. Empty while recognition is inactive; populated at
    /// intern time in the miss branch of [`Interner::intern`] (IRI arm only).
    content_ids: HashMap<TermId, Blake3ContentId, FastHasher>,
}

impl Interner {
    fn new() -> Self {
        Self {
            arena: Vec::new(),
            terms: Vec::new(),
            index: HashTable::new(),
            content_scheme: None,
            content_ids: HashMap::default(),
        }
    }

    /// Append a string to the arena, returning its range.
    fn push_str(&mut self, s: &str) -> StrRange {
        // Validate the range fits u32 BEFORE mutating the arena: a checked overflow
        // here fails fast and leaves the builder consistent, rather than extending the
        // arena past u32::MAX and corrupting every subsequent push_str.
        let offset = u32::try_from(self.arena.len()).expect("term arena exceeds u32::MAX bytes");
        let len = u32::try_from(s.len()).expect("term string exceeds u32::MAX bytes");
        offset
            .checked_add(len)
            .expect("term arena exceeds u32::MAX bytes");
        self.arena.extend_from_slice(s.as_bytes());
        StrRange { offset, len }
    }

    /// Intern a term BY VALUE: dedups against existing terms (resolving their ranges
    /// through the arena) and pushes the strings to the arena only on a MISS, so a
    /// duplicate value costs no arena bytes. Idempotent: equal values map to one id.
    fn intern(&mut self, lookup: TermLookup<'_>) -> TermId {
        let hash = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            hash_lookup(&lookup, &mut h);
            h.finish()
        };
        {
            let (arena, terms) = (&self.arena, &self.terms);
            if let Some(&i) = self
                .index
                .find(hash, |&i| term_eq(arena, &terms[i as usize], &lookup))
            {
                return TermId::from_index(i);
            }
        }
        let i = u32::try_from(self.terms.len()).expect("term table exceeds u32::MAX entries");
        // Content-id recognition (miss branch, IRI arm ONLY): gated first on
        // `content_scheme` so the check is a single `Option` branch — skipped
        // entirely, with zero further work, when recognition is inactive. Computed
        // here (borrowing `&lookup`) BEFORE the moving `match lookup` below consumes
        // `iri`. A prefix hit with a bad hex suffix is an ORDINARY IRI, not an error:
        // `from_hex` returning `None` is correct, not a swallowed failure.
        let recognized: Option<Blake3ContentId> = match &lookup {
            TermLookup::Iri(iri) => self
                .content_scheme
                .as_ref()
                .and_then(|scheme| iri.strip_prefix(scheme.prefix()))
                .and_then(Blake3ContentId::from_hex),
            _ => None,
        };
        // Miss: now (and only now) push the strings to the arena and build the term.
        let term = match lookup {
            TermLookup::Iri(iri) => InternedTerm::Iri(self.push_str(iri)),
            TermLookup::Blank { label, scope } => InternedTerm::Blank {
                label: self.push_str(label),
                scope,
            },
            TermLookup::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let lexical_form = self.push_str(lexical);
                let language = language.map(|l| self.push_str(l));
                InternedTerm::Literal(InternedLiteral {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                })
            }
            TermLookup::Triple { s, p, o } => InternedTerm::Triple { s, p, o },
        };
        self.terms.push(term);
        if let Some(id) = recognized {
            self.content_ids.insert(TermId::from_index(i), id);
        }
        let (arena, terms) = (&self.arena, &self.terms);
        self.index.insert_unique(hash, i, |&i| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            hash_stored(arena, &terms[i as usize], &mut h);
            h.finish()
        });
        TermId::from_index(i)
    }

    /// The decoded content id for a recognized term, if any (builder-level
    /// accessor for tests; the frozen-dataset public accessor is separate).
    #[cfg(test)]
    fn content_id(&self, id: TermId) -> Option<Blake3ContentId> {
        self.content_ids.get(&id).copied()
    }

    /// Look up the [`TermId`] of an already-interned IRI, WITHOUT interning a new
    /// term on a miss. Mirrors the hit path of [`Interner::intern`]'s IRI arm
    /// exactly (same hash, same `term_eq` comparison), so it agrees with `intern`
    /// on every IRI that has actually been interned.
    ///
    /// Used at [`RdfDatasetBuilder::materialize`] to resolve the configured
    /// derivation-predicate IRI to a frozen `TermId` without minting a new term
    /// after the arena/term table are otherwise frozen-bound: terms are frozen at
    /// that point, so a miss here must return `None`, never insert.
    fn lookup_iri(&self, iri: &str) -> Option<TermId> {
        let lookup = TermLookup::Iri(iri);
        let hash = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            hash_lookup(&lookup, &mut h);
            h.finish()
        };
        let (arena, terms) = (&self.arena, &self.terms);
        self.index
            .find(hash, |&i| term_eq(arena, &terms[i as usize], &lookup))
            .copied()
            .map(TermId::from_index)
    }

    fn term(&self, id: TermId) -> &InternedTerm {
        &self.terms[id.index()]
    }

    /// The byte arena backing this interner's term ranges (for the few build-time
    /// readers that resolve a term's string before freeze).
    fn arena(&self) -> &[u8] {
        &self.arena
    }

    fn term_count(&self) -> usize {
        self.terms.len()
    }
}

/// The fallible builder that interns terms, accumulates structure, and freezes
/// into an immutable `Arc<RdfDataset>`.
///
/// Pushed structure is accumulated in deterministic insertion order; quads and
/// annotations are deduplicated *during* push (the dataset is a set, C0.5), while
/// [`freeze`](RdfDatasetBuilder::freeze) re-sorts everything into a stable,
/// reproducible order.
#[derive(Debug)]
pub struct RdfDatasetBuilder {
    /// Owns terms + the value-intern index + the C0 identity policy.
    interner: Interner,
    /// Deduplicated quad rows in first-seen order; `g == None` is the default graph.
    /// The **sole** owner of each row; `quad_index` holds only indices (#880 P3c).
    quads: Vec<QuadRow>,
    /// Store-once dedup index into `quads` (replaces the duplicate `HashSet<QuadRow>`).
    quad_index: HashTable<u32>,
    /// `(reifier, triple-term, graph)` bindings. Several reifiers MAY bind one triple
    /// term and the same binding MAY be pushed more than once; duplicates collapse
    /// (C0.4). The `graph` slot (`None` = default graph) records the named graph the
    /// reifier declaration was asserted in, so a reifier inside a TriG `GRAPH g { … }`
    /// block is matchable under `GRAPH ?g`.
    reifiers: Vec<(TermId, TermId, Option<TermId>)>,
    reifier_index: HashTable<u32>,
    /// `(reifier, predicate, object, graph)` annotations; duplicates collapse (C0.5).
    /// The `graph` slot mirrors [`Self::reifiers`].
    annotations: Vec<(TermId, TermId, TermId, Option<TermId>)>,
    annotation_index: HashTable<u32>,
    /// Sparse source locations keyed by the pushed-quad ordinal. Only quads with a
    /// recorded location appear here.
    locations: Vec<(QuadHandle, RdfLocation)>,
    /// Named graphs EXPLICITLY declared to exist even with zero quads (see
    /// [`declare_named_graph`](Self::declare_named_graph)). Deduplicated at freeze
    /// alongside every graph term that DOES own a quad.
    declared_graphs: Vec<TermId>,
    /// Counter for the next blank-node scope to use when merging a foreign dataset
    /// via [`push_dataset`](RdfDatasetBuilder::push_dataset). Each call to
    /// `push_dataset` claims one fresh scope (starting at 1; 0 = DEFAULT) so that
    /// blank nodes from different source datasets can never collide even when their
    /// labels are identical (standardize-apart, C0.2).
    next_merge_scope: u32,
    /// The caller-supplied predicate IRI that marks a derivation edge between a
    /// content-addressed term and the term(s) it was derived from. `None` (the
    /// default) means no derivation predicate is configured — no fabricated
    /// default, per the no-vocabulary-minting policy. Fixed at construction time
    /// (see [`RdfDatasetBuilder::with_content_addressing`]) — there is no setter.
    derivation_predicate: Option<String>,
}

/// A dataset builder whose structural validation has already passed.
///
/// This type-state splits validation from materialization for callers that need to
/// make that phase boundary explicit, while [`RdfDatasetBuilder::freeze`] keeps the
/// traditional one-shot validate-then-freeze API.
#[derive(Debug)]
pub struct ValidatedRdfDatasetBuilder {
    inner: RdfDatasetBuilder,
}

impl ValidatedRdfDatasetBuilder {
    /// Materialize the already-validated builder into an immutable dataset.
    #[must_use]
    pub fn freeze(self) -> Arc<RdfDataset> {
        Arc::new(self.inner.materialize())
    }
}

impl Default for RdfDatasetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Bulk-push already-interned quads. The ids MUST belong to THIS builder's interner
/// (C0.8 — `TermId`s are dataset-local), so this is the ergonomic bulk form of
/// [`RdfDatasetBuilder::push_quad`], NOT a cross-dataset merge. (There is no
/// `FromIterator<QuadIds>`: a fresh builder's interner is empty, so foreign ids
/// would be out-of-range; a `FromIterator<RdfQuad>` that re-interns owned terms is
/// the right cross-dataset form and is left as follow-up work.)
impl Extend<QuadIds> for RdfDatasetBuilder {
    fn extend<T: IntoIterator<Item = QuadIds>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        // Reserve up front so a bulk push reallocates the quad table/dedup set at
        // most once. The lower bound is exact for non-deduping sources; with
        // duplicates the reserve is a harmless over-estimate.
        let reserve = iter.size_hint().0;
        if reserve > 0 {
            // Fail BEFORE mutating: quad ids are u32 (C0.8), so a merge that would
            // push the table past u32::MAX must abort up front rather than reserve
            // and then panic mid-loop in `next_id`/`intern` with a half-grown table.
            assert!(
                u32::try_from(self.quads.len() + reserve).is_ok(),
                "bulk quad push exceeds maximum quad capacity of u32::MAX"
            );
            self.quads.reserve(reserve);
            self.quad_index
                .reserve(reserve, |&i| hash_of(&self.quads[i as usize]));
        }
        for q in iter {
            self.push_quad(q.s, q.p, q.o, q.g);
        }
    }
}

impl RdfDatasetBuilder {
    /// A fresh, empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            quads: Vec::new(),
            quad_index: HashTable::new(),
            reifiers: Vec::new(),
            reifier_index: HashTable::new(),
            annotations: Vec::new(),
            annotation_index: HashTable::new(),
            locations: Vec::new(),
            declared_graphs: Vec::new(),
            // Merge scopes start at 1; scope 0 is BlankScope::DEFAULT (local pushes).
            next_merge_scope: 1,
            derivation_predicate: None,
        }
    }

    /// A fresh builder configured for content addressing: `scheme` marks which
    /// IRIs are content-id term references and `derivation_predicate`, if given,
    /// is the predicate IRI used to record derivation edges.
    ///
    /// This is the **single** construction path for content addressing (§19
    /// one-path): the config is fixed here, before any term is interned, and
    /// deliberately has no setter — a builder's content-addressing config never
    /// changes mid-build.
    #[must_use]
    pub fn with_content_addressing(
        scheme: ContentIdScheme,
        derivation_predicate: Option<String>,
    ) -> Self {
        let mut builder = Self::new();
        builder.interner.content_scheme = Some(scheme);
        builder.derivation_predicate = derivation_predicate;
        builder
    }

    /// Intern an IRI term. Idempotent: the same IRI string yields the same id.
    pub fn intern_iri(&mut self, iri: &str) -> TermId {
        self.interner.intern(TermLookup::Iri(iri))
    }

    /// Explicitly declare that a named graph exists, even if it turns out to own
    /// zero quads. The frozen dataset's `GRAPH ?g` enumeration
    /// ([`RdfDataset::named_graphs`](super::dataset::RdfDataset::named_graphs))
    /// is the union of this declaration list and every graph term that DOES own a
    /// quad — everything else in the engine (quad matching, `CREATE`/`CLEAR`/`DROP
    /// GRAPH`, capability flags) keeps the ordinary "a graph exists iff it holds a
    /// quad" doctrine untouched. Idempotent (deduplicated at freeze); `g` must be a
    /// [`TermId`] already interned in THIS builder (typically via
    /// [`intern_iri`](Self::intern_iri)).
    pub fn declare_named_graph(&mut self, g: TermId) {
        self.declared_graphs.push(g);
    }

    /// Intern a blank node. Identity is `(label, scope)` (C0.2): same label + same
    /// scope → same id; same label + different scope → different id.
    pub fn intern_blank(&mut self, label: &str, scope: BlankScope) -> TermId {
        self.interner.intern(TermLookup::Blank { label, scope })
    }

    /// Intern a literal, applying the C0.1 identity policy:
    ///
    /// - A language tag → datatype `rdf:langString`; the language is lowercased
    ///   for the key.
    /// - Otherwise an explicit datatype → that datatype.
    /// - Otherwise → `xsd:string`.
    ///
    /// The datatype is always stored as an interned IRI [`TermId`]. The lexical
    /// form is preserved byte-for-byte; base direction participates in identity.
    pub fn intern_literal(&mut self, lit: RdfLiteral) -> TermId {
        let RdfLiteral {
            lexical_form,
            datatype,
            language,
            direction,
        } = lit;

        // C0.1: a language tag forces rdf:langString and a lowercased language key,
        // regardless of any (illegal) explicit datatype on the input literal.
        let (datatype_iri, language_key) = match language {
            Some(lang) => (RDF_LANG_STRING.to_string(), Some(lang.to_lowercase())),
            None => match datatype {
                Some(dt) => (dt, None),
                None => (XSD_STRING.to_string(), None),
            },
        };

        let datatype_id = self.intern_iri(&datatype_iri);

        self.interner.intern(TermLookup::Literal {
            lexical: &lexical_form,
            datatype: datatype_id,
            language: language_key.as_deref(),
            direction,
        })
    }

    /// Intern a triple term (RDF 1.2 quoted triple). Identified structurally by the
    /// resolved `(s, p, o)` ids (C0.3); dedup is by that triple. Acyclicity is a
    /// freeze-time concern ([`super::validate`]), not enforced here.
    pub fn intern_triple(&mut self, s: TermId, p: TermId, o: TermId) -> TermId {
        self.interner.intern(TermLookup::Triple { s, p, o })
    }

    /// Intern one owned model term into this builder, recursively for triple terms.
    ///
    /// This is the inverse of [`RdfDataset::to_owned_quad`](super::dataset::RdfDataset::to_owned_quad)
    /// at the owned boundary: tests and adapter edges that already hold
    /// [`RdfTerm`] values can freeze them into the concrete IR without detouring.
    ///
    /// Blank nodes are interned under [`BlankScope::DEFAULT`]. For cross-dataset
    /// merges use [`push_dataset`](Self::push_dataset), which assigns a fresh scope
    /// per merged dataset (standardize-apart, C0.2).
    pub fn intern_owned_term(&mut self, term: &RdfTerm) -> TermId {
        self.intern_owned_term_scoped(term, BlankScope::DEFAULT)
    }

    /// Like [`intern_owned_term`](Self::intern_owned_term) but overrides the scope
    /// used for every blank node (including blanks nested inside quoted triples).
    /// Private: callers outside this module use the public `push_dataset` path.
    fn intern_owned_term_scoped(&mut self, term: &RdfTerm, scope: BlankScope) -> TermId {
        match term {
            RdfTerm::Iri(iri) => self.intern_iri(iri),
            RdfTerm::BlankNode(label) => self.intern_blank(label, scope),
            RdfTerm::Literal(literal) => self.intern_literal(literal.clone()),
            RdfTerm::Triple(triple) => {
                let s = self.intern_owned_term_scoped(&triple.subject, scope);
                let p = self.intern_iri(&triple.predicate);
                let o = self.intern_owned_term_scoped(&triple.object, scope);
                self.intern_triple(s, p, o)
            }
        }
    }

    /// Push one owned model quad into this builder.
    ///
    /// The predicate is interned as an IRI term, and any source location on the
    /// owned quad is preserved on the corresponding pushed quad handle. Structural
    /// validity remains the normal [`freeze`](Self::freeze) contract.
    pub fn push_owned_quad(&mut self, quad: &RdfQuad) {
        self.push_owned_quad_scoped(quad, BlankScope::DEFAULT);
    }

    /// Like [`push_owned_quad`](Self::push_owned_quad) but routes blank interning
    /// through `scope` (standardize-apart support for `push_dataset`).
    fn push_owned_quad_scoped(&mut self, quad: &RdfQuad, scope: BlankScope) {
        let handle = self.next_quad_handle();
        let s = self.intern_owned_term_scoped(&quad.subject, scope);
        let p = self.intern_iri(&quad.predicate);
        let o = self.intern_owned_term_scoped(&quad.object, scope);
        let g = quad
            .graph_name
            .as_ref()
            .map(|graph_name| self.intern_owned_term_scoped(graph_name, scope));
        self.push_quad(s, p, o, g);
        if let Some(location) = &quad.location {
            self.attach_location(handle, location.clone());
        }
    }

    /// Push one owned RDF 1.2 reifier binding into this builder, re-interning the
    /// reifier resource and the bound triple's `(s, p, o)` terms. The companion of
    /// [`push_owned_quad`](Self::push_owned_quad) for the reifier side-table.
    pub fn push_owned_reifier(&mut self, reifier: &RdfReifier) {
        self.push_owned_reifier_scoped(reifier, BlankScope::DEFAULT);
    }

    /// Like [`push_owned_reifier`](Self::push_owned_reifier) but routes blank
    /// interning through `scope`.
    fn push_owned_reifier_scoped(&mut self, reifier: &RdfReifier, scope: BlankScope) {
        let s = self.intern_owned_term_scoped(&reifier.statement.subject, scope);
        let p = self.intern_iri(&reifier.statement.predicate);
        let o = self.intern_owned_term_scoped(&reifier.statement.object, scope);
        let triple = self.intern_triple(s, p, o);
        let reifier_id = self.intern_owned_term_scoped(&reifier.reifier, scope);
        let g = reifier
            .graph
            .as_ref()
            .map(|g| self.intern_owned_term_scoped(g, scope));
        self.push_reifier_in_graph(reifier_id, triple, g);
    }

    /// Push one owned RDF 1.2 statement annotation into this builder, re-interning
    /// the reifier resource and the `(predicate, object)` terms.
    pub fn push_owned_annotation(&mut self, annotation: &RdfAnnotation) {
        self.push_owned_annotation_scoped(annotation, BlankScope::DEFAULT);
    }

    /// Like [`push_owned_annotation`](Self::push_owned_annotation) but routes blank
    /// interning through `scope`.
    fn push_owned_annotation_scoped(&mut self, annotation: &RdfAnnotation, scope: BlankScope) {
        let reifier_id = self.intern_owned_term_scoped(&annotation.reifier, scope);
        let p = self.intern_iri(&annotation.predicate);
        let o = self.intern_owned_term_scoped(&annotation.object, scope);
        let g = annotation
            .graph
            .as_ref()
            .map(|g| self.intern_owned_term_scoped(g, scope));
        self.push_annotation_in_graph(reifier_id, p, o, g);
    }

    /// Merge every quad, reifier, and annotation of `other` into this builder,
    /// re-interning each owned term into THIS builder's interner (`TermId`s are
    /// dataset-local, C0.8). This is the cross-dataset form the
    /// [`Extend<QuadIds>`] doc reserves as follow-up: unlike the id-based bulk push,
    /// it carries the FULL RDF 1.2 statement layer — reifier bindings and
    /// annotations, not just base quads — so merging a graph that carries `<<>>` /
    /// `rdf:reifies` does not silently drop its side-tables. Duplicates collapse on
    /// freeze per the normal C0.4/C0.5 contract.
    ///
    /// # Standardize-apart (blank-node safety)
    ///
    /// Each call to `push_dataset` allocates a FRESH [`BlankScope`] for `other`
    /// (scopes 1, 2, 3, … on successive calls; scope 0 is [`BlankScope::DEFAULT`]
    /// reserved for direct `push_owned_*` pushes). Blank nodes from different source
    /// datasets can therefore never collide even when they share the same label
    /// (e.g. both datasets have `_:b0`). The qualified label rendered for legacy
    /// consumers is `"{label}.s{n}"` per [`BlankScope::qualify_label`] (C0.2).
    pub fn push_dataset(&mut self, other: &RdfDataset) {
        // Allocate one fresh scope for this entire `other` dataset.
        let scope = BlankScope(self.next_merge_scope);
        self.next_merge_scope = self
            .next_merge_scope
            .checked_add(1)
            .expect("merge scope counter exceeded u32::MAX");

        // Reserve the dominant quad table up front (mirrors the Extend<QuadIds>
        // reserve); the reifier/annotation tables grow on demand.
        let reserve = other.quad_count();
        if reserve > 0 {
            // Fail BEFORE mutating: the merged quad table is u32-indexed, so abort
            // up front if `other` would overflow it rather than corrupt builder
            // state midway through the merge loop.
            assert!(
                u32::try_from(self.quads.len() + reserve).is_ok(),
                "dataset merge exceeds maximum quad capacity of u32::MAX"
            );
            self.quads.reserve(reserve);
            self.quad_index
                .reserve(reserve, |&i| hash_of(&self.quads[i as usize]));
        }
        for quad in other.owned_quads() {
            self.push_owned_quad_scoped(&quad, scope);
        }
        for reifier in other.owned_reifiers() {
            self.push_owned_reifier_scoped(&reifier, scope);
        }
        for annotation in other.owned_annotations() {
            self.push_owned_annotation_scoped(&annotation, scope);
        }
        for graph in other.owned_named_graphs() {
            let g = self.intern_owned_term_scoped(&graph, scope);
            self.declare_named_graph(g);
        }
    }

    /// Crate-internal read access to an interned term. [`freeze`](Self::freeze) and
    /// [`super::validate`] consume this to materialize and check the dataset.
    pub(crate) fn term(&self, id: TermId) -> &InternedTerm {
        self.interner.term(id)
    }

    /// Resolve a term's [`StrRange`] to a `&str` borrowed from the builder's arena
    /// (for build-time readers that have an interned term's range, #879).
    pub(crate) fn interned_str(&self, range: StrRange) -> &str {
        arena_str(self.interner.arena(), range)
    }

    /// Resolve a builder-local term id to a borrowed term view.
    ///
    /// This mirrors [`RdfDataset::resolve`] for adapters that need to validate a
    /// just-interned term before freezing the dataset.
    pub fn resolve(&self, id: TermId) -> TermRef<'_> {
        match self.term(id) {
            InternedTerm::Iri(iri) => TermRef::Iri(self.interned_str(*iri)),
            InternedTerm::Blank { label, scope } => TermRef::Blank {
                label: self.interned_str(*label),
                scope: *scope,
            },
            InternedTerm::Literal(literal) => TermRef::Literal {
                lexical: self.interned_str(literal.lexical_form),
                datatype: literal.datatype,
                language: literal.language.map(|lang| self.interned_str(lang)),
                direction: literal.direction,
            },
            InternedTerm::Triple { s, p, o } => TermRef::Triple {
                s: *s,
                p: *p,
                o: *o,
            },
        }
    }

    /// The number of distinct interned terms. Used by validation (the ID-reference
    /// bound) and as the frozen dataset's term count.
    pub(crate) fn term_count(&self) -> usize {
        self.interner.term_count()
    }

    /// Push a quad. Duplicate quads collapse to a single row (C0.5); `g == None`
    /// names the default graph. Returns nothing — the quad's ordinal is reflected
    /// by [`attach_location`](Self::attach_location), which keys off the pushed
    /// (deduped) order via [`QuadHandle`].
    pub fn push_quad(&mut self, s: TermId, p: TermId, o: TermId, g: Option<TermId>) {
        let row = QuadRow { s, p, o, g };
        store_once(&mut self.quads, &mut self.quad_index, row);
    }

    /// Bind a reifier resource to a triple term (C0.4). Several reifiers MAY bind
    /// one triple term; an identical `(reifier, triple)` binding pushed twice
    /// collapses to one.
    ///
    /// Interns `rdf:reifies` into the term table so the side-table is always
    /// projectable as virtual `reifier rdf:reifies <<…>>` quads
    /// ([`RdfDataset::reifier_quads`]) — without it, a dataset rebuilt purely from
    /// `push_owned_reifier` (e.g. a named-graph projection) would carry reifiers the
    /// renderer / pattern-matcher cannot see, because the predicate id is absent.
    pub fn push_reifier(&mut self, reifier: TermId, triple: TermId) {
        self.push_reifier_in_graph(reifier, triple, None);
    }

    /// Like [`push_reifier`](Self::push_reifier) but records the named graph the
    /// reifier declaration was asserted in (`None` = default graph). A `(reifier,
    /// triple, graph)` triplet pushed twice collapses to one; the SAME `(reifier,
    /// triple)` in two distinct graphs is two bindings.
    pub fn push_reifier_in_graph(&mut self, reifier: TermId, triple: TermId, g: Option<TermId>) {
        let _ = self.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies");
        let binding = (reifier, triple, g);
        store_once(&mut self.reifiers, &mut self.reifier_index, binding);
    }

    /// Push a statement annotation `(reifier, predicate, object)`. Duplicate
    /// annotations collapse to one (C0.5).
    pub fn push_annotation(&mut self, reifier: TermId, p: TermId, o: TermId) {
        self.push_annotation_in_graph(reifier, p, o, None);
    }

    /// Like [`push_annotation`](Self::push_annotation) but records the named graph the
    /// annotation was asserted in (`None` = default graph); see
    /// [`push_reifier_in_graph`](Self::push_reifier_in_graph).
    pub fn push_annotation_in_graph(
        &mut self,
        reifier: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    ) {
        let annotation = (reifier, p, o, g);
        store_once(
            &mut self.annotations,
            &mut self.annotation_index,
            annotation,
        );
    }

    /// Attach a source location to a previously pushed quad, identified by its
    /// [`QuadHandle`] (the dense ordinal of the deduplicated quad). Sparse: only
    /// quads with a recorded location are stored. An empty location is ignored.
    pub fn attach_location(&mut self, handle: QuadHandle, loc: RdfLocation) {
        if !loc.is_empty() {
            self.locations.push((handle, loc));
        }
    }

    /// The [`QuadHandle`] that the next [`push_quad`](Self::push_quad) call will
    /// assign to a *newly seen* quad — i.e. the current deduplicated-quad count.
    /// Callers that need to attach a location pair this with the immediately
    /// following push.
    pub fn next_quad_handle(&self) -> QuadHandle {
        QuadHandle::from_index(self.quads.len() as u32)
    }

    /// Validate structure (positional constraints, ID-reference validity,
    /// triple-term acyclicity) and FREEZE into an immutable, deterministically
    /// ordered, deduplicated `Arc<RdfDataset>`.
    ///
    /// Per the no-optionality doctrine this HARD-fails (`Err`) on malformed
    /// structure — there is no degraded fallback and no silent default.
    pub fn freeze(self) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        self.validate().map(ValidatedRdfDatasetBuilder::freeze)
    }

    /// Validate structure and return the type-state that is allowed to materialize
    /// the frozen dataset.
    pub fn validate(self) -> Result<ValidatedRdfDatasetBuilder, RdfDiagnostic> {
        super::validate::validate(&self)?;
        Ok(ValidatedRdfDatasetBuilder { inner: self })
    }

    /// Borrow the accumulated quad rows (validation reads these).
    pub(crate) fn quad_rows(&self) -> &[QuadRow] {
        &self.quads
    }

    /// Borrow the accumulated reifier bindings (validation reads these).
    pub(crate) fn reifier_rows(&self) -> &[(TermId, TermId, Option<TermId>)] {
        &self.reifiers
    }

    /// Borrow the accumulated annotation rows (validation reads these).
    pub(crate) fn annotation_rows(&self) -> &[(TermId, TermId, TermId, Option<TermId>)] {
        &self.annotations
    }

    /// Consume the builder and materialize the frozen dataset. Called only by
    /// [`freeze`](Self::freeze) AFTER validation has passed.
    fn materialize(self) -> RdfDataset {
        let Self {
            interner,
            quads,
            mut reifiers,
            mut annotations,
            locations,
            declared_graphs,
            derivation_predicate,
            ..
        } = self;

        // Resolve the configured derivation-predicate IRI to its frozen `TermId`
        // via a lookup-only probe (never interns): terms are frozen from this
        // point on, so an IRI that was configured but never actually interned
        // resolves to `None` — "no derivations present", not an error.
        let derivation_predicate = derivation_predicate.and_then(|iri| interner.lookup_iri(&iri));

        // Deterministic, reproducible frozen order: sort by id tuples. Terms keep
        // their interning (allocation) order, which is itself deterministic for a
        // fixed push sequence.
        //
        // A `QuadHandle` addresses a quad by its FROZEN ordinal, but locations are
        // pushed keyed by the *push-order* ordinal. Sorting the quads moves each one
        // to a new position, so every location handle must be remapped to its quad's
        // post-sort index — otherwise `location_of` returns a *different* quad's
        // location, an LSP break for any consumer reading through the compat bridge.
        let mut indexed: Vec<(QuadRow, u32)> = quads
            .into_iter()
            .enumerate()
            .map(|(push, row)| (row, push as u32))
            .collect();
        indexed.sort_unstable_by_key(|(row, _)| *row);
        let mut push_to_frozen: HashMap<u32, u32> = HashMap::with_capacity(indexed.len());
        let quads: Vec<QuadRow> = indexed
            .into_iter()
            .enumerate()
            .map(|(frozen, (row, push))| {
                push_to_frozen.insert(push, frozen as u32);
                row
            })
            .collect();
        // Remap each location's handle from push ordinal → frozen ordinal. A handle
        // whose push ordinal never materialized (e.g. attached then a duplicate
        // quad was pushed) is dropped: its quad collapsed into a surviving row that
        // carries its own handle.
        let mut locations: Vec<(QuadHandle, RdfLocation)> = locations
            .into_iter()
            .filter_map(|(handle, loc)| {
                push_to_frozen
                    .get(&(handle.index() as u32))
                    .map(|&frozen| (QuadHandle::from_index(frozen), loc))
            })
            .collect();
        reifiers.sort_unstable();
        annotations.sort_unstable();
        locations.sort_unstable_by_key(|(handle, _)| *handle);

        // The frozen `GRAPH ?g` enumeration set: every graph term that owns a quad,
        // UNION every graph the caller explicitly declared (possibly empty).
        let mut named_graphs: Vec<TermId> = declared_graphs;
        named_graphs.extend(quads.iter().filter_map(|q| q.g));
        // A reifier / annotation declared inside a `GRAPH g { … }` block owns no base
        // quad in g (the `<< … >>` folds entirely into the side-tables), so g would be
        // invisible to `GRAPH ?g` enumeration unless its overlay rows are counted too.
        named_graphs.extend(reifiers.iter().filter_map(|(_, _, g)| *g));
        named_graphs.extend(annotations.iter().filter_map(|(_, _, _, g)| *g));
        named_graphs.sort_unstable();
        named_graphs.dedup();

        let caps =
            compute_capabilities(&interner.terms, &quads, &reifiers, &annotations, &locations);

        RdfDataset::from_parts(
            interner.arena.into_boxed_slice(),
            interner.terms.into_boxed_slice(),
            quads.into_boxed_slice(),
            reifiers.into_boxed_slice(),
            annotations.into_boxed_slice(),
            locations.into_boxed_slice(),
            caps,
            named_graphs.into_boxed_slice(),
            interner.content_ids,
            derivation_predicate,
        )
    }
}

use crate::{RdfDiagnostic, RdfStoreCapabilities};

/// Compute the dataset's capability flags ONCE at freeze, from the frozen tables.
fn compute_capabilities(
    terms: &[InternedTerm],
    quads: &[QuadRow],
    reifiers: &[(TermId, TermId, Option<TermId>)],
    annotations: &[(TermId, TermId, TermId, Option<TermId>)],
    locations: &[(QuadHandle, RdfLocation)],
) -> RdfStoreCapabilities {
    RdfStoreCapabilities {
        named_graphs: quads.iter().any(|q| q.g.is_some())
            || reifiers.iter().any(|(_, _, g)| g.is_some())
            || annotations.iter().any(|(_, _, _, g)| g.is_some()),
        quoted_triples: terms
            .iter()
            .any(|t| matches!(t, InternedTerm::Triple { .. })),
        reifiers: !reifiers.is_empty(),
        annotations: !annotations.is_empty(),
        source_locations: !locations.is_empty(),
        // The frozen dataset is the hot graph only; envelope concerns (loss
        // records, lookaside) live elsewhere (C0.6).
        loss_records: false,
        lookaside: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RdfTextDirection;
    use proptest::prelude::*;

    fn lit_simple(s: &str) -> RdfLiteral {
        RdfLiteral::simple(s)
    }

    #[test]
    fn intern_iri_is_idempotent() {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/x");
        let c = b.intern_iri("http://example.org/x");
        let d = b.intern_iri("http://example.org/y");
        assert_eq!(a, c);
        assert_ne!(a, d);
    }

    /// No fabricated default: a plain `new`/`default` builder has content-id
    /// recognition INACTIVE.
    #[test]
    fn new_builder_has_no_content_scheme() {
        let b = RdfDatasetBuilder::new();
        assert!(b.interner.content_scheme.is_none());
        assert!(b.derivation_predicate.is_none());
    }

    /// `with_content_addressing` is the single construction path: it fixes the
    /// scheme (and, optionally, the derivation predicate) before any intern.
    #[test]
    fn with_content_addressing_sets_config() {
        let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
        let b = RdfDatasetBuilder::with_content_addressing(
            scheme.clone(),
            Some("http://example.org/derivedFrom".to_string()),
        );
        assert_eq!(b.interner.content_scheme, Some(scheme));
        assert_eq!(
            b.derivation_predicate.as_deref(),
            Some("http://example.org/derivedFrom")
        );
    }

    /// A recognized `blake3:<64hex>` IRI gets a side-table entry with the exact
    /// decoded bytes.
    #[test]
    fn intern_iri_recognizes_content_id() {
        let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let hex = "ab".repeat(32);
        let id = b.intern_iri(&format!("blake3:{hex}"));
        let expected = Blake3ContentId::from_hex(&hex).expect("valid hex");
        assert_eq!(b.interner.content_id(id), Some(expected));
    }

    /// Interning the same content-id IRI twice yields the same `TermId` and
    /// exactly one side-table entry (idempotent, no double-insert drift).
    #[test]
    fn intern_iri_content_id_is_idempotent() {
        let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let iri = format!("blake3:{}", "cd".repeat(32));
        let a = b.intern_iri(&iri);
        let c = b.intern_iri(&iri);
        assert_eq!(a, c);
        assert_eq!(b.interner.content_ids.len(), 1);
    }

    /// A prefix hit with a malformed hex suffix is an ORDINARY IRI, not an
    /// error: no side-table entry, but interning still succeeds.
    #[test]
    fn intern_iri_rejects_malformed_suffix_as_ordinary_iri() {
        let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);

        let too_short = format!("blake3:{}", "a".repeat(63));
        let too_long = format!("blake3:{}", "a".repeat(65));
        let non_hex = format!("blake3:{}z", "a".repeat(63));
        let uppercase = format!("blake3:{}", "AB".repeat(32));

        for iri in [&too_short, &too_long, &non_hex, &uppercase] {
            let id = b.intern_iri(iri);
            assert_eq!(
                b.interner.content_id(id),
                None,
                "unexpected match for {iri}"
            );
        }
        assert!(b.interner.content_ids.is_empty());
    }

    /// An ordinary IRI with no content-id prefix at all gets no entry.
    #[test]
    fn intern_iri_ordinary_iri_has_no_content_id() {
        let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let id = b.intern_iri("http://example.org/x");
        assert_eq!(b.interner.content_id(id), None);
    }

    /// Recognition is IRI-arm only: a blank node whose label happens to look
    /// like a content-id IRI is never recognized (rejected by construction,
    /// not by a runtime check).
    #[test]
    fn intern_blank_never_recognized_as_content_id() {
        let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let label = format!("blake3:{}", "ef".repeat(32));
        let id = b.intern_blank(&label, BlankScope::DEFAULT);
        assert_eq!(b.interner.content_id(id), None);
        assert!(b.interner.content_ids.is_empty());
    }

    /// With recognition inactive (plain `new()`), a `blake3:<64hex>` IRI is
    /// interned as an ordinary IRI: no side-table entry.
    #[test]
    fn intern_iri_no_recognition_when_scheme_inactive() {
        let mut b = RdfDatasetBuilder::new();
        let id = b.intern_iri(&format!("blake3:{}", "12".repeat(32)));
        assert_eq!(b.interner.content_id(id), None);
        assert!(b.interner.content_ids.is_empty());
    }

    #[test]
    fn intern_blank_is_idempotent() {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_blank("b0", BlankScope::DEFAULT);
        let c = b.intern_blank("b0", BlankScope::DEFAULT);
        assert_eq!(a, c);
    }

    #[test]
    fn intern_literal_is_idempotent() {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_literal(lit_simple("x"));
        let c = b.intern_literal(lit_simple("x"));
        assert_eq!(a, c);
    }

    /// C0.1: a plain literal expands to `xsd:string`, so `"x"` and an explicit
    /// `"x"^^xsd:string` intern to the same id.
    #[test]
    fn datatype_expansion_equality() {
        let mut b = RdfDatasetBuilder::new();
        let plain = b.intern_literal(RdfLiteral::simple("x"));
        let explicit = b.intern_literal(RdfLiteral::typed("x", XSD_STRING));
        assert_eq!(plain, explicit);
    }

    /// C0.1: base direction participates in identity — same lexical + language but
    /// different direction are distinct.
    #[test]
    fn directional_literal_distinctness() {
        let mut b = RdfDatasetBuilder::new();
        let base = RdfLiteral {
            lexical_form: "x".to_string(),
            datatype: None,
            language: Some("en".to_string()),
            direction: Some(RdfTextDirection::Ltr),
        };
        let mut other = base.clone();
        other.direction = Some(RdfTextDirection::Rtl);
        let none = RdfLiteral {
            direction: None,
            ..base.clone()
        };
        let ltr = b.intern_literal(base);
        let rtl = b.intern_literal(other);
        let no_dir = b.intern_literal(none);
        assert_ne!(ltr, rtl);
        assert_ne!(ltr, no_dir);
        assert_ne!(rtl, no_dir);
    }

    /// C0.1: language tags are lowercased for the key, so `@EN` and `@en` are equal.
    #[test]
    fn language_lowercasing() {
        let mut b = RdfDatasetBuilder::new();
        let upper = b.intern_literal(RdfLiteral::language_tagged("x", "EN"));
        let lower = b.intern_literal(RdfLiteral::language_tagged("x", "en"));
        assert_eq!(upper, lower);
    }

    /// A language-tagged literal expands to `rdf:langString`, distinct from a plain
    /// `xsd:string` literal of the same lexical form.
    #[test]
    fn lang_tagged_distinct_from_plain() {
        let mut b = RdfDatasetBuilder::new();
        let plain = b.intern_literal(RdfLiteral::simple("x"));
        let tagged = b.intern_literal(RdfLiteral::language_tagged("x", "en"));
        assert_ne!(plain, tagged);
    }

    /// C0.2: blank-node scope participates in the key.
    #[test]
    fn blank_scope_distinctness() {
        let mut b = RdfDatasetBuilder::new();
        let s1 = b.intern_blank("b", BlankScope(1));
        let s2 = b.intern_blank("b", BlankScope(2));
        let s1_again = b.intern_blank("b", BlankScope(1));
        assert_ne!(s1, s2);
        assert_eq!(s1, s1_again);
    }

    /// C0.3: triple terms are identified structurally by resolved `(s, p, o)`, and
    /// a triple term nests as the object of another triple and stays reusable.
    #[test]
    fn nested_triple_term_structural_identity() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");

        let t1 = b.intern_triple(s, p, o);
        let t2 = b.intern_triple(s, p, o);
        assert_eq!(t1, t2, "same (s,p,o) → same triple-term id");

        // Nest the triple term as the object of an outer triple, twice.
        let outer_p = b.intern_iri("http://example.org/asserts");
        let outer1 = b.intern_triple(s, outer_p, t1);
        let outer2 = b.intern_triple(s, outer_p, t2);
        assert_eq!(outer1, outer2, "nested triple term is reusable by id");

        // The inner triple term remains a distinct, single interned term.
        let t3 = b.intern_triple(s, p, o);
        assert_eq!(t1, t3);
    }

    /// A small helper interning a fresh IRI by suffix.
    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    #[test]
    fn freeze_dedupes_quads() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(ds.quads().count(), 1, "duplicate quads collapse to one row");
    }

    #[test]
    fn validated_builder_type_state_freezes_after_validation() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, p, o, None);

        let validated = b.validate().expect("well-formed builder validates");
        let ds = validated.freeze();
        assert_eq!(ds.quads().count(), 1);
    }

    #[test]
    fn freeze_preserves_named_graphs() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let g = iri(&mut b, "g");
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, Some(g));
        let ds = b.freeze().expect("valid");
        assert_eq!(
            ds.quads().count(),
            2,
            "default and named graph are distinct"
        );
        assert!(ds.capabilities().named_graphs);
        let graphs: Vec<_> = ds.quads().map(|q| q.g).collect();
        assert!(graphs.contains(&None));
        assert!(graphs.contains(&Some(g)));
    }

    #[test]
    fn freeze_keeps_multiple_reifiers_for_one_triple() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let triple = b.intern_triple(s, p, o);
        let r1 = iri(&mut b, "r1");
        let r2 = iri(&mut b, "r2");
        b.push_reifier(r1, triple);
        b.push_reifier(r2, triple);
        b.push_reifier(r1, triple); // duplicate binding collapses
        let ds = b.freeze().expect("valid");
        let reifiers: Vec<_> = ds.reifiers().collect();
        assert_eq!(
            reifiers.len(),
            2,
            "two distinct reifiers survive, dup collapses"
        );
        assert!(reifiers.contains(&(r1, triple)));
        assert!(reifiers.contains(&(r2, triple)));
    }

    #[test]
    fn freeze_dedupes_annotations() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let triple = b.intern_triple(s, p, o);
        let r = iri(&mut b, "r");
        let ap = iri(&mut b, "ap");
        let ao = iri(&mut b, "ao");
        b.push_reifier(r, triple);
        b.push_annotation(r, ap, ao);
        b.push_annotation(r, ap, ao);
        let ds = b.freeze().expect("valid");
        assert_eq!(
            ds.annotations().count(),
            1,
            "duplicate annotation collapses"
        );
    }

    /// `push_dataset` re-interns a foreign dataset's base quads into a fresh
    /// builder's interner; quads shared with an already-merged dataset collapse.
    #[test]
    fn push_dataset_merges_and_dedupes_quads() {
        let dataset_a = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o, o2) = (
                iri(&mut b, "s"),
                iri(&mut b, "p"),
                iri(&mut b, "o"),
                iri(&mut b, "o2"),
            );
            b.push_quad(s, p, o, None);
            b.push_quad(s, p, o2, None);
            b.freeze().expect("valid A")
        };
        let dataset_b = {
            let mut b = RdfDatasetBuilder::new();
            let (s, s2, p, o, o2) = (
                iri(&mut b, "s"),
                iri(&mut b, "s2"),
                iri(&mut b, "p"),
                iri(&mut b, "o"),
                iri(&mut b, "o2"),
            );
            b.push_quad(s2, p, o, None); // B-only quad
            b.push_quad(s, p, o2, None); // shared with A → collapses on merge
            b.freeze().expect("valid B")
        };

        let mut merged = RdfDatasetBuilder::new();
        merged.push_dataset(&dataset_a);
        merged.push_dataset(&dataset_b);
        let merged = merged.freeze().expect("valid merged");

        assert_eq!(
            merged.quad_count(),
            3,
            "A(2) + B(2) with one shared quad → 3 distinct rows"
        );
        // The B-only quad resolves through the merged interner.
        let has_b_only = merged.owned_quads().any(|q| {
            q.subject == RdfTerm::Iri("http://example.org/s2".to_string())
                && q.predicate == "http://example.org/p"
                && q.object == RdfTerm::Iri("http://example.org/o".to_string())
        });
        assert!(has_b_only, "B-only quad survives the cross-dataset merge");
    }

    /// `push_dataset` carries the FULL RDF 1.2 statement layer: a merged dataset's
    /// reifier bindings AND annotations survive, not just its base quads. (A
    /// quad-only merge would silently drop these — the regression this guards.)
    #[test]
    fn push_dataset_carries_reifiers_and_annotations() {
        let dataset = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o);
            let r = iri(&mut b, "r");
            let ap = iri(&mut b, "ap");
            let ao = iri(&mut b, "ao");
            b.push_reifier(r, triple);
            b.push_annotation(r, ap, ao);
            b.freeze().expect("valid source")
        };

        let mut merged = RdfDatasetBuilder::new();
        merged.push_dataset(&dataset);
        let merged = merged.freeze().expect("valid merged");

        assert_eq!(
            merged.reifiers().count(),
            1,
            "reifier binding survives merge"
        );
        assert_eq!(merged.annotations().count(), 1, "annotation survives merge");

        let reifier = merged.owned_reifiers().next().expect("one reifier");
        assert_eq!(
            reifier.reifier,
            RdfTerm::Iri("http://example.org/r".to_string())
        );
        assert_eq!(
            reifier.statement.subject,
            RdfTerm::Iri("http://example.org/s".to_string())
        );
        let annotation = merged.owned_annotations().next().expect("one annotation");
        assert_eq!(
            annotation.reifier,
            RdfTerm::Iri("http://example.org/r".to_string())
        );
        assert_eq!(annotation.predicate, "http://example.org/ap");
        assert_eq!(
            annotation.object,
            RdfTerm::Iri("http://example.org/ao".to_string())
        );
    }

    /// `push_dataset` must NOT collapse blank nodes that have the same label but
    /// originate from DIFFERENT source datasets (standardize-apart, C0.2).
    ///
    /// WITHOUT standardize-apart these would collapse to one node.
    #[test]
    fn push_dataset_standardizes_apart_blank_nodes() {
        // Dataset A: single quad  _:b0 <ex:p> <ex:o1>
        let dataset_a = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_blank("b0", BlankScope::DEFAULT);
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o1");
            b.push_quad(s, p, o, None);
            b.freeze().expect("valid A")
        };

        // Dataset B: single quad  _:b0 <ex:p> <ex:o2>  (same blank label, different object)
        let dataset_b = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_blank("b0", BlankScope::DEFAULT);
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o2");
            b.push_quad(s, p, o, None);
            b.freeze().expect("valid B")
        };

        let mut merged = RdfDatasetBuilder::new();
        merged.push_dataset(&dataset_a);
        merged.push_dataset(&dataset_b);
        let merged = merged.freeze().expect("valid merged");

        // Both quads must survive: without standardize-apart the two _:b0 subjects
        // would collapse into one node and one of the two objects would be lost.
        assert_eq!(merged.quad_count(), 2, "both quads survive the merge");

        // The two blank subject nodes must be DISTINCT (different TermIds / qualified
        // labels), because they came from different source datasets.
        let subjects: Vec<_> = merged.owned_quads().map(|q| q.subject).collect();
        assert_eq!(subjects.len(), 2);
        // Collect the unique subject labels; standardize-apart gives them distinct
        // qualified labels via BlankScope::qualify_label.
        let subject_labels: std::collections::HashSet<String> = subjects
            .iter()
            .filter_map(|t| {
                if let RdfTerm::BlankNode(label) = t {
                    Some(label.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            subject_labels.len(),
            2,
            "two distinct blank-node subjects after standardize-apart; got: {subject_labels:?}"
        );
    }

    /// A `Strategy` producing one of a small fixed pool of distinct intern calls,
    /// so we can count the distinct *values* requested and compare to `term_count`.
    #[derive(Clone, Debug)]
    enum Op {
        Iri(u8),
        Blank(u8, u32),
        Literal(u8),
    }

    fn op_strategy() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0u8..4).prop_map(Op::Iri),
            (0u8..4, 0u32..3).prop_map(|(l, s)| Op::Blank(l, s)),
            (0u8..4).prop_map(Op::Literal),
        ]
    }

    proptest! {
        /// Idempotence holds across arbitrary call sequences, and `term_count`
        /// never exceeds the number of distinct values interned.
        ///
        /// The distinct-value count is computed independently of the interner: a
        /// literal also interns its datatype IRI, so each literal value contributes
        /// itself plus its (shared) datatype term to the upper bound.
        #[test]
        fn proptest_idempotence_and_bounded_count(ops in prop::collection::vec(op_strategy(), 0..64)) {
            use std::collections::HashSet;

            let mut b = RdfDatasetBuilder::new();
            // Map a value-key → the id it first produced, to assert idempotence.
            let mut seen: HashMap<String, TermId> = HashMap::new();
            // The set of distinct *terms* (value keys, incl. datatype IRIs) that
            // SHOULD exist after the run — the exact upper bound for term_count.
            let mut distinct_terms: HashSet<String> = HashSet::new();

            for op in ops {
                let (call_key, id) = match op {
                    Op::Iri(n) => {
                        let iri = format!("http://example.org/i{n}");
                        distinct_terms.insert(format!("iri:{iri}"));
                        (format!("iri:{iri}"), b.intern_iri(&iri))
                    }
                    Op::Blank(l, s) => {
                        let key = format!("blank:{l}:{s}");
                        distinct_terms.insert(key.clone());
                        (key, b.intern_blank(&format!("b{l}"), BlankScope(s)))
                    }
                    Op::Literal(n) => {
                        let lex = format!("v{n}");
                        // Plain literal → xsd:string; both the literal and its
                        // datatype IRI become distinct interned terms.
                        distinct_terms.insert(format!("lit:{lex}"));
                        distinct_terms.insert(format!("iri:{XSD_STRING}"));
                        (format!("lit:{lex}"), b.intern_literal(RdfLiteral::simple(lex)))
                    }
                };

                match seen.get(&call_key) {
                    Some(&prev) => prop_assert_eq!(prev, id, "intern not idempotent"),
                    None => {
                        seen.insert(call_key, id);
                    }
                }
            }

            prop_assert_eq!(b.term_count(), distinct_terms.len());
        }
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! First-party in-memory serialization model + RDF text serializers.
//!
//! [`SerGraph`] is the first-party term/quad/reifier/annotation shape the frozen
//! [`RdfDataset`](crate::RdfDataset) IR is lowered into before egress. The Turtle /
//! TriG / N-Triples / N-Quads serializers walk exactly this shape, emitting literal
//! lexical forms VERBATIM — they never canonicalize a literal's value-space nor narrow
//! its datatype (the whole point of the native codec: byte-for-byte lexical fidelity).

use std::borrow::Cow;

use purrdf_core::sink::TextOut;
use purrdf_iri::BaseIri;
use purrdf_lex::literal_escape::{self, Carrier};
use purrdf_lex::term_syntax::{self, TRIPLE_TERM_CLOSE, TRIPLE_TERM_OPEN};

use crate::{FastHasher, FastMap, RdfDiagnostic, RdfTextDirection};

/// The kind of a serialization term.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SerTermKind {
    Iri,
    Bnode,
    Literal,
    Triple,
}

/// A single RDF term in the serialization model, carried by integer id.
///
/// `Hash` is derived so the interner can memoize on the EMITTED shape rather than on
/// a second owned copy of every term's text — see `SerGraphInterner::memo`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SerTerm {
    /// Term kind.
    pub kind: SerTermKind,
    /// IRI string, literal lexical form, or blank-node label (scope-local).
    pub value: Option<String>,
    /// Term-id of the literal's datatype IRI, when explicit.
    pub datatype: Option<usize>,
    /// Literal language tag (BCP 47).
    pub lang: Option<String>,
    /// RDF 1.2 literal base direction (`"ltr"` or `"rtl"`).
    pub direction: Option<String>,
    /// Term-id of the reifier of a quoted triple (`kind == Triple`).
    pub reifier: Option<usize>,
}

/// A quad of term-ids; the graph slot is `None` for the default graph.
pub(crate) type SerQuad = (usize, usize, usize, Option<usize>);
/// A subject/predicate/object triple of term-ids.
pub(crate) type SerTriple3 = (usize, usize, usize);
/// A reifier row: `(reifier, (s, p, o), graph?)`.
pub(crate) type SerReifierRow = (usize, SerTriple3, Option<usize>);
/// An annotation row: `(reifier, predicate, value, graph?)`.
pub(crate) type SerAnnotationRow = (usize, usize, usize, Option<usize>);

/// The serialization graph: terms plus the base quads and the RDF 1.2 statement layer
/// (reifier bindings + annotations). Each row carries an `Option<usize>` graph slot
/// (`None` = default graph).
///
/// It is the native codec's own model rather than `purrdf_gts::model::Graph`, whose
/// reifier rows have the same shape: the native codec depends on no GTS codec
/// (`tests/gts_codec_hygiene.rs` holds that seam), so each model keeps its own
/// first-row reifier lookup.
#[derive(Debug, Default)]
pub(crate) struct SerGraph {
    pub terms: Vec<SerTerm>,
    pub quads: Vec<SerQuad>,
    pub reifiers: Vec<SerReifierRow>,
    pub annotations: Vec<SerAnnotationRow>,
    /// The document base this graph is EMITTED under, or `None` for absolute output.
    ///
    /// `Some` is set by exactly one place —
    /// [`build_ser_graph`](super::serialize::build_ser_graph) — and only for a format
    /// whose [`emits_base`](super::media_type::NativeRdfFormat::emits_base) column is
    /// `true`. That is the egress mirror of the ingress rule: a syntax that can express
    /// a base emits it and relativizes against it; a syntax that cannot never applies
    /// one, so N-Triples, N-Quads, TriX and HexTuples reach their writers with `None`
    /// here and emit absolute IRIs whatever the caller passed.
    ///
    /// The terms themselves stay ABSOLUTE in the table; relativization happens at write
    /// time ([`write_term`]), so [`Self::sort_canonical`] can key on absolute text and
    /// the emitted statement order never moves with the base.
    pub base: Option<BaseIri>,
}

impl SerGraph {
    /// Whether reifier `rid` is a self-reifier sentinel: a triple term that
    /// names itself as its reifier, i.e. an inline quoted triple its parent
    /// quad already carries rather than a separate reifier binding.
    pub(crate) fn is_self_reifier(&self, rid: usize) -> bool {
        self.terms
            .get(rid)
            .is_some_and(|term| term.kind == SerTermKind::Triple && term.reifier == Some(rid))
    }

    /// The document base this graph is emitted under, or `None` for absolute output.
    pub(crate) fn base(&self) -> Option<&BaseIri> {
        self.base.as_ref()
    }

    /// Look up a reifier binding: the `(s, p, o)` of the FIRST `reifiers` row whose id
    /// equals `rid`.
    ///
    /// A linear scan of the table. A writer that resolves many quoted-triple terms
    /// takes a [`reifier_index`](Self::reifier_index) once instead — this is O(rows)
    /// per call, and rendering every triple term through it is quadratic in the
    /// statement layer.
    pub(crate) fn reifier(&self, rid: usize) -> Option<SerTriple3> {
        self.reifiers
            .iter()
            .find(|(r, _, _)| *r == rid)
            .map(|(_, spo, _)| *spo)
    }

    /// An O(1) snapshot of [`reifier`](Self::reifier) over the CURRENT `reifiers`
    /// table, with the same first-row-wins semantics. Build it once per document walk;
    /// it does not track later mutation of `reifiers`.
    pub(crate) fn reifier_index(&self) -> ReifierIndex {
        ReifierIndex::build(&self.reifiers)
    }

    /// Reorder the base quads and the RDF 1.2 statement layer into a **canonical,
    /// backend-independent** order, keyed on each row's rendered term text.
    ///
    /// The term-table indices a [`SerGraph`] carries are assigned in the interning
    /// order of whichever [`DatasetView`](crate::DatasetView) fed the builder, so two
    /// backends holding the SAME dataset (e.g. the production `RdfDataset` and a
    /// `PackView` over its pack bytes) index their terms differently and thus iterate
    /// quads in different orders. Sorting on the *rendered value* — a pure function of
    /// the term, identical across backends — makes the emitted document byte-identical
    /// regardless of backend and removes the interning-order dependence from the
    /// serializer — serializers are byte-deterministic.
    ///
    /// The lookup in [`Self::reifier`] is by id, so permuting `reifiers` never changes
    /// which binding a quoted-triple term resolves to; the self-reifier sentinel rows
    /// (skipped on output) are permuted harmlessly among the real reifier rows.
    ///
    /// # The keys are the ABSOLUTE term text, deliberately
    ///
    /// The comparator renders through [`write_term_absolute`], never [`write_term`], so a
    /// [`base`](Self::base) in force cannot reach the sort key. Sorting on the emitted
    /// (possibly relativized) spelling would make the document's statement ORDER a
    /// function of the base — `<b>` and `<../b>` sort in different places than the
    /// absolute IRIs they abbreviate — which is exactly the nondeterminism this method
    /// exists to remove. Two exports of one dataset that differ only in `--base`
    /// therefore carry the same rows in the same order, spelled differently.
    pub(crate) fn sort_canonical(&mut self) {
        // The rows are sorted THROUGH a comparator that renders on demand, rather than
        // by precomputing a key per row. The keys were `Vec<String>` — four or five
        // allocations for every quad, reifier and annotation in the document, every one
        // of them alive for the whole sort — and they were compared element by element
        // and then thrown away. Two reusable buffers give the identical ordering: the
        // comparison is still term-by-term on rendered text, and a `None` graph still
        // renders as the empty string, which is what places it before any named graph.
        //
        // The trade is deliberate. This renders O(n log n) times instead of n, and holds
        // O(1) scratch instead of O(n) keys. It is the right way round for a serializer,
        // whose peak is what decides whether a large export completes at all — and the
        // rendering it repeats is a walk over a term table already in memory.
        //
        // Each vector is taken out before sorting so the comparator can borrow `self`
        // immutably while the rows it orders are a local.
        let mut left = String::new();
        let mut right = String::new();

        // Each sort renders through an O(1) snapshot of the reifier table AS IT STANDS
        // for that sort, rather than scanning `reifiers` on every quoted-triple term
        // of every comparison (O(rows) per term, O(n log n) comparisons). The snapshot
        // is taken per sort because the table is not the same for all three: see the
        // reifier sort below.
        let index = self.reifier_index();
        let mut quads = std::mem::take(&mut self.quads);
        quads.sort_by(|&(s1, p1, o1, g1), &(s2, p2, o2, g2)| {
            cmp_terms(
                self,
                &index,
                &[s1, p1, o1],
                &[s2, p2, o2],
                &mut left,
                &mut right,
            )
            .then_with(|| cmp_graph(self, &index, g1, g2, &mut left, &mut right))
        });
        self.quads = quads;

        // The reifier rows are sorted with the table TAKEN OUT of `self` (it is the
        // thing being sorted), so a quoted-triple term met in a reifier row resolves
        // to no binding and renders as its `_:unbound_triple_{tid}` fallback. That is
        // the ordering this comparator has always produced, and the emitted order of
        // an RDF 1.2 document depends on it; an EMPTY index reproduces it exactly.
        let mut reifiers = std::mem::take(&mut self.reifiers);
        let emptied = ReifierIndex::default();
        reifiers.sort_by(|&(r1, (s1, p1, o1), g1), &(r2, (s2, p2, o2), g2)| {
            cmp_terms(
                self,
                &emptied,
                &[r1, s1, p1, o1],
                &[r2, s2, p2, o2],
                &mut left,
                &mut right,
            )
            .then_with(|| cmp_graph(self, &emptied, g1, g2, &mut left, &mut right))
        });
        self.reifiers = reifiers;

        // Rebuilt from the RESTORED (now sorted) table, so the first-row-wins answer is
        // the one the scan gave over that table.
        let index = self.reifier_index();
        let mut annotations = std::mem::take(&mut self.annotations);
        annotations.sort_by(|&(r1, p1, o1, g1), &(r2, p2, o2, g2)| {
            cmp_terms(
                self,
                &index,
                &[r1, p1, o1],
                &[r2, p2, o2],
                &mut left,
                &mut right,
            )
            .then_with(|| cmp_graph(self, &index, g1, g2, &mut left, &mut right))
        });
        self.annotations = annotations;
    }
}

/// An id-keyed snapshot of a [`SerGraph`]'s `reifiers` table: `rid → (s, p, o)` of the
/// FIRST row carrying `rid`, exactly [`SerGraph::reifier`]'s answer, in O(1).
///
/// Built once per document walk (each writer takes one at entry) so resolving a
/// quoted-triple term costs a hash probe rather than a scan of every reifier row — the
/// scan made a star-heavy document quadratic in its statement layer. The map is
/// fixed-key hashed and only ever probed, never iterated, so it cannot reach the output
/// order.
#[derive(Debug, Default)]
pub(crate) struct ReifierIndex {
    bindings: FastMap<usize, SerTriple3>,
}

impl ReifierIndex {
    /// Index `rows` in order; a repeated id keeps its first binding.
    pub(crate) fn build(rows: &[SerReifierRow]) -> Self {
        let mut bindings = FastMap::with_capacity_and_hasher(rows.len(), FastHasher::default());
        for &(rid, spo, _) in rows {
            bindings.entry(rid).or_insert(spo);
        }
        Self { bindings }
    }

    /// The `(s, p, o)` bound to `rid`, if any row carries it.
    pub(crate) fn get(&self, rid: usize) -> Option<SerTriple3> {
        self.bindings.get(&rid).copied()
    }
}

/// A deterministic blank-node label with the given `prefix`, byte-identical to the
/// prior purrdf-gts `deterministic_label(prefix, counter)`: `prefix` plus the 26-digit
/// Crockford Base32 rendering of a zero-timestamp ULID built from `counter`.
///
/// With a zero timestamp the rendered ULID value equals `counter` for any
/// `counter < 2^80`, so this is [`purrdf_lex::crockford::write_u128`] of
/// `counter as u128`.
pub(crate) fn deterministic_blank_label_with_prefix(prefix: &str, counter: usize) -> String {
    let mut label = String::with_capacity(prefix.len() + purrdf_lex::crockford::U128_DIGITS);
    label.push_str(prefix);
    // Writing into a `String` cannot fail.
    let _ = purrdf_lex::crockford::write_u128(counter as u128, &mut label);
    label
}

/// A deterministic blank-node label, byte-identical to the prior purrdf-gts
/// `deterministic_label("gts_", counter)`. See
/// [`deterministic_blank_label_with_prefix`].
pub(crate) fn deterministic_blank_label(counter: usize) -> String {
    deterministic_blank_label_with_prefix("gts_", counter)
}

use purrdf_iri::vocab::rdf::NS as RDF_NS;
use purrdf_iri::vocab::rdf::REIFIES as RDF_REIFIES;
use purrdf_xsd::datatype::XSD_NS;

/// Compare two term-id sequences by their rendered text, position by position.
///
/// The sequences are the same length at every call site. `left` and `right` are scratch
/// reused across the whole sort, which is the point: this is the comparison a
/// `Vec<String>` key used to make after allocating one string per position per row.
fn cmp_terms(
    g: &SerGraph,
    ix: &ReifierIndex,
    a: &[usize],
    b: &[usize],
    left: &mut String,
    right: &mut String,
) -> std::cmp::Ordering {
    for (&x, &y) in a.iter().zip(b.iter()) {
        left.clear();
        right.clear();
        write_term_absolute(g, ix, x, left);
        write_term_absolute(g, ix, y, right);
        let ordering = left.as_str().cmp(right.as_str());
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    std::cmp::Ordering::Equal
}

/// Compare two optional graph slots by rendered text, `None` rendering as empty.
///
/// Empty is what `unwrap_or_default` produced for the absent graph in the previous key,
/// and it is load-bearing: the empty string sorts before every rendered term, so the
/// default graph's rows lead. Rendering `None` as anything else would reorder the
/// document.
fn cmp_graph(
    g: &SerGraph,
    ix: &ReifierIndex,
    a: Option<usize>,
    b: Option<usize>,
    left: &mut String,
    right: &mut String,
) -> std::cmp::Ordering {
    left.clear();
    right.clear();
    if let Some(x) = a {
        write_term_absolute(g, ix, x, left);
    }
    if let Some(y) = b {
        write_term_absolute(g, ix, y, right);
    }
    left.as_str().cmp(right.as_str())
}

/// Spell `iri` the way this document should carry it: relative to `base` when a relative
/// reference round-trips to it, absolute otherwise.
///
/// [`BaseIri::relativize`] is the one relativization algorithm in the workspace and
/// verifies its own answer by re-resolving it, so a `Some` here always resolves back to
/// `iri` byte for byte and a `None` is the semantic "no relative spelling exists" — the
/// correct response to which is the absolute IRI, not an error.
///
/// With `base = None` — every N-Triples / N-Quads / TriX / HexTuples export, and every
/// export with no `--base` — this is an `Option` test and a borrow: no parse, no
/// allocation, and the emitted bytes are the ones this serializer always produced.
pub(super) fn spell_iri<'a>(iri: &'a str, base: Option<&BaseIri>) -> Cow<'a, str> {
    let Some(base) = base else {
        return Cow::Borrowed(iri);
    };
    // A term table holds absolute IRIs, so the parse succeeds; an unparseable value has
    // no relative spelling either way and rides out verbatim rather than failing egress.
    purrdf_iri::parse(iri)
        .ok()
        .and_then(|target| base.relativize(&target))
        .map_or(Cow::Borrowed(iri), Cow::Owned)
}

/// Append an `IRIREF` token — [`term_syntax::write_iri`] — spelling the IRI against
/// `base` per [`spell_iri`].
fn write_iri_ref<W: TextOut + ?Sized>(out: &mut W, iri: &str, base: Option<&BaseIri>) {
    term_syntax::write_iri(&spell_iri(iri, base), out);
}

/// Append one term's N-Triples surface to `out`.
///
/// Appends rather than returns. Building each term as its own `String` cost an
/// allocation per term — three or four per quad, every one of them alive until the
/// whole document had been assembled — and every byte was going to be copied into the
/// output anyway. A literal with a datatype paid twice over, since the datatype's IRI
/// was rendered into a `String` only to be `format!`ed into the literal's.
///
/// The escapers write straight into `out`, so an unescaped value — which is nearly all of
/// them — is copied from the term table without an intermediate of any kind.
///
/// IRIs are spelled against the graph's [`base`](SerGraph::base), which is `None` for
/// every format whose registry row says it cannot express one. Use
/// [`write_term_absolute`] where the absolute spelling is what is wanted regardless — the
/// canonical sort keys.
///
/// # Termination
///
/// This follows a literal's datatype and a quoted triple's `(s, p, o)` over a work
/// list with no depth bound and no visited set — a serializer's inner loop should carry
/// neither — so it finishes only because every producer of a [`SerGraph`] hands it a
/// term table that terminates. There are exactly three, and each owes that guarantee: the text
/// parsers and the [`DatasetView`](crate::DatasetView) lowering in `serialize.rs` both
/// intern a triple term's components BEFORE the term itself, so a component id is
/// always strictly smaller than the term naming it; and `crate::gts::gts_to_ser`,
/// whose input is a caller-supplied GTS graph and which therefore proves it, refusing
/// a self-reaching table with `gts-self-reaching-term`. A fourth producer must do the
/// same — a self-reaching table would never finish writing.
///
/// `ix` is the caller's one-per-document [`ReifierIndex`]: quoted-triple terms resolve
/// through it rather than through a scan of the reifier table per term.
fn write_term<W: TextOut + ?Sized>(g: &SerGraph, ix: &ReifierIndex, tid: usize, out: &mut W) {
    write_term_in(g, ix, tid, out, g.base(), false);
}

/// Append one term's N-Triples surface to `out` with every IRI spelled ABSOLUTELY,
/// whatever base the graph carries. This is the canonical-ordering key (see
/// [`SerGraph::sort_canonical`]), never an output spelling.
fn write_term_absolute<W: TextOut + ?Sized>(
    g: &SerGraph,
    ix: &ReifierIndex,
    tid: usize,
    out: &mut W,
) {
    write_term_in(g, ix, tid, out, None, false);
}

/// [`write_term`] against `base`, and [`write_trig_term`] when `trig` is set.
///
/// The term syntax is [`term_syntax`]'s: an IRI through [`term_syntax::write_iri`], a
/// blank node through [`term_syntax::write_blank`], and a triple term between
/// [`TRIPLE_TERM_OPEN`] and [`TRIPLE_TERM_CLOSE`]. A literal's body rides the
/// [`Carrier::Xml`] escaper rather than the canonical one, because this serializer's
/// output is also embedded verbatim in an XML text node by the CL-dialect carrier, and
/// an XML reader rewrites or refuses the C1 controls, U+FFFE and U+FFFF; its datatype
/// is a term of the table, spelled against `base` like every other IRI. TriG differs
/// in exactly one place: `rdf:reifies` is written through its declared prefix.
///
/// A quoted triple is written over a work list: its opening `<<( ` at once, then its
/// subject next, with the separators, the predicate, the object and the closing ` )>>`
/// held back in that order until the subject's whole nesting is written. A typed
/// literal's datatype is written next after its `^^`.
fn write_term_in<W: TextOut + ?Sized>(
    g: &SerGraph,
    ix: &ReifierIndex,
    tid: usize,
    out: &mut W,
    base: Option<&BaseIri>,
    trig: bool,
) {
    let mut held: Vec<TermPiece> = Vec::new();
    let mut next = Some(TermPiece::Term(tid));
    while let Some(piece) = next.take().or_else(|| held.pop()) {
        let tid = match piece {
            TermPiece::Text(text) => {
                out.push_str(text);
                continue;
            }
            TermPiece::Term(tid) => tid,
        };
        let t = &g.terms[tid];
        match t.kind {
            SerTermKind::Iri if trig && t.value.as_deref() == Some(RDF_REIFIES) => {
                out.push_str("rdf:reifies");
            }
            SerTermKind::Iri => write_iri_ref(out, t.value.as_deref().unwrap_or(""), base),
            SerTermKind::Bnode => match &t.value {
                Some(v) => term_syntax::write_blank(v, out),
                None => {
                    let _ = write!(out, "_:b{tid}");
                }
            },
            SerTermKind::Literal => {
                out.push('"');
                literal_escape::write(t.value.as_deref().unwrap_or(""), Carrier::Xml, out);
                out.push('"');
                if let Some(lang) = &t.lang {
                    out.push('@');
                    out.push_str(lang);
                    if let Some(direction) = t
                        .direction
                        .as_deref()
                        .filter(|d| RdfTextDirection::from_str_token(d).is_some())
                    {
                        out.push_str("--");
                        out.push_str(direction);
                    }
                } else if let Some(dt) = t.datatype {
                    out.push_str("^^");
                    next = Some(TermPiece::Term(dt));
                }
                // else: plain literal == xsd:string, written bare
            }
            // quoted triple (RDF 1.2 triple term), resolved through its reifier
            SerTermKind::Triple => match t.reifier.and_then(|rf| ix.get(rf)) {
                Some((s, p, o)) => {
                    out.push_str(TRIPLE_TERM_OPEN);
                    out.push(' ');
                    held.extend(TermPiece::triple_tail(p, o));
                    next = Some(TermPiece::Term(s));
                }
                // degraded but syntactically valid: an unbound reifier becomes a blank node
                None => {
                    let _ = write!(out, "_:unbound_triple_{tid}");
                }
            },
        }
    }
}

/// One pending piece of a term being written over a work list: a term, or fixed text.
enum TermPiece {
    Term(usize),
    Text(&'static str),
}

impl TermPiece {
    /// What follows a quoted triple's subject, in the order it is popped: a space, the
    /// predicate, a space, the object, and the closing ` )>>`.
    const fn triple_tail(p: usize, o: usize) -> [Self; 6] {
        [
            Self::Text(TRIPLE_TERM_CLOSE),
            Self::Text(" "),
            Self::Term(o),
            Self::Text(" "),
            Self::Term(p),
            Self::Text(" "),
        ]
    }
}

/// Append a [`SerGraph`]'s N-Quads text to `out`.
///
/// One line is built at a time into `out` itself. The previous shape collected a
/// `String` per line into a `Vec`, `join`ed it — copying the whole document — and then
/// `format!`ed the result to add a trailing newline, copying the whole document a
/// second time. Peak was therefore about twice the output on top of one live `String`
/// per quad; Turtle paid a third copy by wrapping this function's result.
pub(crate) fn write_nquads<W: TextOut + ?Sized>(g: &SerGraph, out: &mut W) {
    // One reifier index per document: every quoted-triple term below resolves through
    // it in O(1) instead of scanning the reifier table.
    let ix = g.reifier_index();

    for &(s, p, o, gname) in &g.quads {
        write_term(g, &ix, s, out);
        out.push(' ');
        write_term(g, &ix, p, out);
        out.push(' ');
        write_term(g, &ix, o, out);
        write_graph_terminator(g, &ix, gname, out);
    }

    for &(rid, (s, p, o), gname) in &g.reifiers {
        if g.terms
            .get(rid)
            .is_some_and(|term| term.kind == SerTermKind::Triple && term.reifier == Some(rid))
        {
            continue;
        }
        write_term(g, &ix, rid, out);
        out.push(' ');
        // Through the same speller as every other IRI, so one term cannot be written two
        // ways in one document when a base happens to cover the RDF namespace.
        write_iri_ref(out, RDF_REIFIES, g.base());
        out.push(' ');
        out.push_str(TRIPLE_TERM_OPEN);
        out.push(' ');
        write_term(g, &ix, s, out);
        out.push(' ');
        write_term(g, &ix, p, out);
        out.push(' ');
        write_term(g, &ix, o, out);
        out.push(' ');
        out.push_str(TRIPLE_TERM_CLOSE);
        write_graph_terminator(g, &ix, gname, out);
    }

    for &(r, p, v, gname) in &g.annotations {
        write_term(g, &ix, r, out);
        out.push(' ');
        write_term(g, &ix, p, out);
        out.push(' ');
        write_term(g, &ix, v, out);
        write_graph_terminator(g, &ix, gname, out);
    }
}

/// Close one N-Quads statement: the optional graph name, the `.`, and the line break.
fn write_graph_terminator<W: TextOut + ?Sized>(
    g: &SerGraph,
    ix: &ReifierIndex,
    gname: Option<usize>,
    out: &mut W,
) {
    if let Some(gv) = gname {
        out.push(' ');
        write_term(g, ix, gv, out);
    }
    out.push_str(" .\n");
}

/// Append the Turtle/TriG `@base <…> .` directive when `g` carries a base, and nothing at
/// all when it does not.
///
/// The directive's own IRI is written ABSOLUTELY — it is what every later reference
/// resolves against, so relativizing it against itself would produce `<>`, which resolves
/// to the base only if a base is already in force.
fn write_base_directive<W: TextOut + ?Sized>(g: &SerGraph, out: &mut W) {
    let Some(base) = g.base() else {
        return;
    };
    out.push_str("@base ");
    write_iri_ref(out, base.as_str(), None);
    out.push_str(" .\n");
}

/// Assert that no row of `g` carries a named-graph slot — the single-graph syntaxes
/// (N-Triples, Turtle) cannot serialize named-graph quads. Mirrors the upstream
/// `ensure_default_graph_projection` rejection.
fn ensure_default_graph_projection(g: &SerGraph, format: &str) -> Result<(), RdfDiagnostic> {
    let named = g.quads.iter().any(|(_, _, _, gname)| gname.is_some())
        || g.reifiers.iter().any(|(_, _, gname)| gname.is_some())
        || g.annotations.iter().any(|(_, _, _, gname)| gname.is_some());
    if named {
        return Err(RdfDiagnostic::error(
            "native-codec-serialize",
            format!("{format} cannot serialize a named graph"),
        ));
    }
    Ok(())
}

/// Serialise a [`SerGraph`] to N-Triples text (default graph only).
pub(crate) fn write_ntriples<W: TextOut + ?Sized>(
    g: &SerGraph,
    out: &mut W,
) -> Result<(), RdfDiagnostic> {
    ensure_default_graph_projection(g, "N-Triples")?;
    write_nquads(g, out);
    Ok(())
}

/// Serialise a [`SerGraph`] to Turtle text (default graph only); the N-Quads body is
/// prefixed with the `rdf:`/`xsd:` `@prefix` header, and by an `@base` directive when the
/// graph carries one. IRIs in the body stay full `<...>` — they are NOT abbreviated
/// against the declared prefixes, but they ARE relativized against the emitted `@base`,
/// which the directive at the top of the document makes exact on re-read.
///
/// The body is [`write_nquads`], which spells its IRIs through the graph's base as well.
/// N-Triples and N-Quads reach that same function with `base: None` — their registry rows
/// say they cannot express a base — so the directive-less syntaxes stay absolute for free
/// rather than by a check repeated per writer.
pub(crate) fn write_turtle<W: TextOut + ?Sized>(
    g: &SerGraph,
    out: &mut W,
) -> Result<(), RdfDiagnostic> {
    ensure_default_graph_projection(g, "Turtle")?;

    // Decided BEFORE the header is written, not after. The previous shape emitted the
    // header, emitted the body, and retracted the header with `truncate` when the body
    // turned out empty — a rewind an incremental sink cannot perform once bytes have
    // gone downstream. The predicate is the one `write_nquads` already computed and
    // threw away, so an empty graph still emits nothing at all, header included.
    if !emits_any_statement(g) {
        return Ok(());
    }

    write_base_directive(g, out);
    out.push_str("@prefix rdf: <");
    out.push_str(RDF_NS);
    out.push_str("> .\n@prefix xsd: <");
    out.push_str(XSD_NS);
    out.push_str("> .\n\n");

    write_nquads(g, out);
    Ok(())
}

/// Whether this graph emits at least one statement.
///
/// Hoisted out of [`write_nquads`], which computed exactly this and discarded it. It
/// is what lets a header-bearing syntax decide before emitting rather than emitting
/// and retracting.
///
/// A self-reifier sentinel does NOT count. Such a row is an inline quoted-triple term
/// already carried by its parent row, not a statement of its own, which is why
/// [`write_nquads`] skips it — so a graph holding nothing else emits nothing, and
/// Turtle and TriG agree on that.
pub(crate) fn emits_any_statement(g: &SerGraph) -> bool {
    !g.quads.is_empty()
        || !g.annotations.is_empty()
        || g.reifiers.iter().any(|&(rid, _, _)| {
            !g.terms
                .get(rid)
                .is_some_and(|term| term.kind == SerTermKind::Triple && term.reifier == Some(rid))
        })
}

// ── TriG ──────────────────────────────────────────────────────────────────────────

/// Append one term's TriG surface to `out`: [`write_term`] with `rdf:reifies` written
/// through the declared prefix rather than as a full IRI.
fn write_trig_term<W: TextOut + ?Sized>(g: &SerGraph, ix: &ReifierIndex, tid: usize, out: &mut W) {
    write_term_in(g, ix, tid, out, g.base(), true);
}

/// Close the open `GRAPH { … }` block, if one is open.
fn close_graph<W: TextOut + ?Sized>(out: &mut W, open_graph: &mut Option<String>) {
    if open_graph.take().is_some() {
        out.push_str("}\n");
    }
}

/// Put `out` in the right block for `graph_name` and write the statement's indent.
///
/// The caller then appends the statement's own terms directly, rather than handing over
/// a finished `String`. `open_graph` still holds the RENDERED graph name because that
/// name is what decides whether the next statement continues this block or starts
/// another — but it is now rebuilt only when the graph CHANGES, not once per statement.
///
/// `scratch` is the caller's reusable buffer: the graph name is rendered into it and
/// compared as TEXT against the open block (the comparison the writer has always made),
/// and copied out only when a new block opens — one allocation per graph change rather
/// than one per statement.
fn begin_statement<W: TextOut + ?Sized>(
    out: &mut W,
    open_graph: &mut Option<String>,
    scratch: &mut String,
    graph: &SerGraph,
    ix: &ReifierIndex,
    graph_name: Option<usize>,
) {
    let Some(gid) = graph_name else {
        close_graph(out, open_graph);
        return;
    };
    scratch.clear();
    write_trig_term(graph, ix, gid, scratch);
    if open_graph.as_deref() != Some(scratch.as_str()) {
        close_graph(out, open_graph);
        out.push_str(scratch);
        out.push_str(" {\n");
        *open_graph = Some(scratch.clone());
    }
    out.push_str("  ");
}

/// Append a [`SerGraph`]'s TriG text to `out`, led by an `@base` directive when the graph
/// carries one (and by nothing when it does not).
///
/// Statements are written in place. The previous shape collected every line into a
/// `Vec<String>`, `join`ed it — copying the whole document — and then `format!`ed the
/// result to add a trailing newline, copying it again. Writing each line followed by
/// its own newline produces exactly those bytes: a join with `"\n"` plus one trailing
/// `"\n"` is the same sequence as one `"\n"` after each line.
pub(crate) fn write_trig<W: TextOut + ?Sized>(g: &SerGraph, out: &mut W) {
    // The same predicate Turtle uses. The previous guard tested `reifiers.is_empty()`,
    // which counts self-reifier sentinel rows that this writer then skips — so a graph
    // holding only sentinels emitted a bare `@base`/`@prefix` header here while Turtle
    // emitted nothing. Sharing the predicate makes the two agree, in the direction of
    // emitting nothing: a document with no statements has no reason to declare
    // prefixes.
    if !emits_any_statement(g) {
        return;
    }

    write_base_directive(g, out);
    out.push_str("@prefix rdf: <");
    out.push_str(RDF_NS);
    out.push_str("> .\n\n");
    let mut open_graph: Option<String> = None;
    // One graph-name scratch buffer and one reifier index for the whole document (see
    // `begin_statement` and `ReifierIndex`).
    let mut scratch = String::new();
    let ix = g.reifier_index();

    for &(s, p, o, gname) in &g.quads {
        begin_statement(out, &mut open_graph, &mut scratch, g, &ix, gname);
        write_trig_term(g, &ix, s, out);
        out.push(' ');
        write_trig_term(g, &ix, p, out);
        out.push(' ');
        write_trig_term(g, &ix, o, out);
        out.push_str(" .\n");
    }

    for &(rid, (s, p, o), gname) in &g.reifiers {
        // A triple TERM keys its own components under its own id (a self-reference, not
        // a reifier relationship); rendering it as `<<( … )>> rdf:reifies <<( … )>>`
        // would assert a triple term in subject position. Its components are already
        // carried inline wherever the term appears, so skip the entry.
        if g.terms
            .get(rid)
            .is_some_and(|t| t.kind == SerTermKind::Triple && t.reifier == Some(rid))
        {
            continue;
        }
        begin_statement(out, &mut open_graph, &mut scratch, g, &ix, gname);
        write_trig_term(g, &ix, rid, out);
        out.push_str(" rdf:reifies ");
        out.push_str(TRIPLE_TERM_OPEN);
        out.push(' ');
        write_trig_term(g, &ix, s, out);
        out.push(' ');
        write_trig_term(g, &ix, p, out);
        out.push(' ');
        write_trig_term(g, &ix, o, out);
        out.push(' ');
        out.push_str(TRIPLE_TERM_CLOSE);
        out.push_str(" .\n");
    }

    for &(r, p, v, gname) in &g.annotations {
        begin_statement(out, &mut open_graph, &mut scratch, g, &ix, gname);
        write_trig_term(g, &ix, r, out);
        out.push(' ');
        write_trig_term(g, &ix, p, out);
        out.push(' ');
        write_trig_term(g, &ix, v, out);
        out.push_str(" .\n");
    }

    close_graph(out, &mut open_graph);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Collect-into-a-`String` shims. Production has no such function any more: every
    // caller reaches the writers through `RdfCodec::serialize_into` and supplies its own
    // buffer, so a `to_*` in the crate proper would be dead code kept alive by tests.
    // These assert on a whole document, which is the one place materialising it is the
    // point rather than a cost.

    /// The lazy comparator orders rows exactly as the materialised keys did.
    ///
    /// `sort_canonical` used to build a `Vec<String>` key per row and sort on that. The
    /// keys are gone; the ORDER they produced is a published property, because it is
    /// what makes an exported document byte-identical across backends. So the old key
    /// construction is reproduced here and the two orderings are compared directly —
    /// asserting the replacement is equivalent, not merely that it is some valid order.
    ///
    /// The graph is built to exercise the part a round-trip vector is least likely to:
    /// rows identical in subject, predicate and object that differ ONLY in the graph
    /// slot, including the absent one. The old key rendered `None` through
    /// `unwrap_or_default` as the empty string, which sorts before every rendered term
    /// and therefore puts the default graph's rows first — behaviour the comparator has
    /// to reproduce deliberately rather than inherit.
    #[test]
    fn the_lazy_comparator_orders_rows_exactly_as_the_materialised_keys_did() {
        fn term(g: &mut SerGraph, iri: &str) -> usize {
            g.terms.push(SerTerm {
                kind: SerTermKind::Iri,
                value: Some(iri.to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            });
            g.terms.len() - 1
        }

        let mut g = SerGraph::default();
        let s = term(&mut g, "https://example.org/s");
        let p = term(&mut g, "https://example.org/p");
        let o = term(&mut g, "https://example.org/o");
        let o2 = term(&mut g, "https://example.org/a");
        let ga = term(&mut g, "https://example.org/gz");
        let gb = term(&mut g, "https://example.org/ga");

        // Deliberately inserted out of order, with the default-graph rows in the middle
        // so neither the input order nor a stable sort can produce the answer by luck.
        g.quads = vec![
            (s, p, o, Some(ga)),
            (s, p, o2, None),
            (s, p, o, Some(gb)),
            (s, p, o, None),
            (s, p, o2, Some(ga)),
        ];

        // The key the previous implementation built, verbatim in shape.
        let key = |g: &SerGraph, &(a, b, c, d): &SerQuad| -> Vec<String> {
            vec![
                render_term(g, a),
                render_term(g, b),
                render_term(g, c),
                d.map_or_default(|x| render_term(g, x)),
            ]
        };
        let mut expected = g.quads.clone();
        expected.sort_by_key(|x| key(&g, x));

        g.sort_canonical();
        assert_eq!(
            g.quads, expected,
            "the comparator must reproduce the key-based order exactly; the emitted \
             byte order is what that order decides"
        );

        // And the property that empty-renders-`None` actually buys. The graph slot is
        // the LAST key component, so it separates rows only once subject, predicate and
        // object have tied — the default graph leads within each such group, not the
        // document. Stated as the invariant rather than as fixed positions, because
        // fixed positions would also pass for a comparator that ignored the graph.
        for pair in g.quads.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if (a.0, a.1, a.2) == (b.0, b.1, b.2) {
                assert!(
                    !(a.3.is_some() && b.3.is_none()),
                    "within one subject/predicate/object group an absent graph renders \
                     as the empty string and must precede every named one: {:?}",
                    g.quads
                );
            }
        }
    }

    /// A one-IRI-term-per-entry graph over `iris`, with one quad per `(s, p, o)` row.
    fn graph_of(iris: &[&str], rows: &[(usize, usize, usize)]) -> SerGraph {
        let mut g = SerGraph::default();
        for iri in iris {
            g.terms.push(SerTerm {
                kind: SerTermKind::Iri,
                value: Some((*iri).to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            });
        }
        g.quads = rows.iter().map(|&(s, p, o)| (s, p, o, None)).collect();
        g
    }

    /// The writers emit the same bytes into a `String` and into a draining
    /// `TextSink`, because they are ONE writer generic over its append target.
    ///
    /// This is the property the whole incremental path rests on, and it is
    /// structural rather than coincidental: there is no second emitter to drift.
    /// The drain collects what actually LEFT the sink rather than what it still
    /// had staged, and the fixture is large enough to cross the window many times,
    /// so a writer that cached, re-read, or retracted its output would diverge
    /// here even though it looks correct against a `String`.
    #[test]
    fn every_writer_emits_the_same_bytes_into_a_string_and_into_a_drain() {
        /// Collects every drained window.
        struct Collect(Vec<u8>);
        impl purrdf_core::sink::ByteDrain for Collect {
            fn drain(&mut self, chunk: &[u8]) -> Result<(), purrdf_core::sink::DrainError> {
                self.0.extend_from_slice(chunk);
                Ok(())
            }
        }

        /// Run one writer both ways and compare. `write` is called twice with
        /// different append targets and nothing else differs.
        fn both_ways(name: &str, eager: &str, drained: &Collect) {
            assert!(!eager.is_empty(), "{name} fixture must emit something");
            assert_eq!(
                eager.as_bytes(),
                drained.0.as_slice(),
                "{name} diverged between a String and a drain"
            );
        }

        // Long IRIs and repeated rows, so the document crosses the drain window.
        let iris: Vec<String> = (0..6)
            .map(|i| format!("https://example.org/{i}/{}", "segment-".repeat(400)))
            .collect();
        let refs: Vec<&str> = iris.iter().map(String::as_str).collect();
        let mut g = graph_of(&refs, &[]);
        for i in 0..200_u32 {
            let slot = if i % 3 == 0 {
                None
            } else {
                Some(3 + (i as usize % 2))
            };
            g.quads.push((0, 1, 2, slot));
        }
        g.sort_canonical();

        // N-Quads and TriG carry named graphs; TriG additionally opens, closes and
        // reopens blocks across the window.
        let mut eager = String::new();
        write_nquads(&g, &mut eager);
        let mut collect = Collect(Vec::new());
        {
            let mut sink = purrdf_core::sink::TextSink::to_drain(&mut collect);
            write_nquads(&g, &mut sink);
            sink.finish().expect("collecting drain never fails");
        }
        both_ways("nquads", &eager, &collect);

        let mut eager = String::new();
        write_trig(&g, &mut eager);
        let mut collect = Collect(Vec::new());
        {
            let mut sink = purrdf_core::sink::TextSink::to_drain(&mut collect);
            write_trig(&g, &mut sink);
            sink.finish().expect("collecting drain never fails");
        }
        both_ways("trig", &eager, &collect);

        // N-Triples and Turtle are default-graph-only.
        let mut flat = graph_of(&refs, &[]);
        for _ in 0..200 {
            flat.quads.push((0, 1, 2, None));
        }
        flat.sort_canonical();

        let mut eager = String::new();
        write_ntriples(&flat, &mut eager).expect("default-graph projection");
        let mut collect = Collect(Vec::new());
        {
            let mut sink = purrdf_core::sink::TextSink::to_drain(&mut collect);
            write_ntriples(&flat, &mut sink).expect("default-graph projection");
            sink.finish().expect("collecting drain never fails");
        }
        both_ways("ntriples", &eager, &collect);

        let mut eager = String::new();
        write_turtle(&flat, &mut eager).expect("default-graph projection");
        let mut collect = Collect(Vec::new());
        {
            let mut sink = purrdf_core::sink::TextSink::to_drain(&mut collect);
            write_turtle(&flat, &mut sink).expect("default-graph projection");
            sink.finish().expect("collecting drain never fails");
        }
        both_ways("turtle", &eager, &collect);
    }

    /// A graph carrying ONLY self-reifier sentinel rows — an inline quoted-triple term
    /// that is its own reifier, with no statement of its own.
    ///
    /// Built DIRECTLY rather than through `build_ser_graph`, deliberately. `SerGraph`
    /// has four producers in this crate, so "the builder cannot construct this" is not
    /// a proof that no writer ever sees it; constructing it here is.
    fn self_reifier_sentinel_only_graph() -> SerGraph {
        let mut g = graph_of(&["https://example.org/s", "https://example.org/p"], &[]);
        // A `Triple` term whose reifier is itself: the sentinel shape both writers skip.
        let rid = g.terms.len();
        g.terms.push(SerTerm {
            kind: SerTermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(rid),
        });
        g.reifiers.push((rid, (0, 1, 0), None));
        g
    }

    /// Turtle and TriG agree that a graph with no statements emits NOTHING.
    ///
    /// Turtle used to write its header and retract it with `truncate`; TriG guarded on
    /// `reifiers.is_empty()`, which counts sentinel rows it then skips — so this input
    /// made TriG emit a bare header while Turtle emitted nothing. Both now consult one
    /// predicate, and this is the executable proof of it rather than an argument about
    /// which inputs are reachable.
    #[test]
    fn a_graph_with_no_statements_emits_nothing_in_turtle_and_trig() {
        for mut g in [
            SerGraph::default(),
            self_reifier_sentinel_only_graph(),
            graph_of(&["https://example.org/s"], &[]),
        ] {
            g.sort_canonical();
            assert!(
                !emits_any_statement(&g),
                "fixture should carry no emittable statement"
            );

            let mut turtle = String::new();
            write_turtle(&g, &mut turtle).expect("default-graph projection");
            assert_eq!(turtle, "", "Turtle emitted a header for an empty document");

            let mut trig = String::new();
            write_trig(&g, &mut trig);
            assert_eq!(trig, "", "TriG emitted a header for an empty document");

            let mut nquads = String::new();
            write_nquads(&g, &mut nquads);
            assert_eq!(
                nquads, "",
                "N-Quads emitted a statement for an empty document"
            );
        }
    }

    /// The hoisted predicate agrees with what the writers actually emit, for graphs that
    /// DO carry statements — the valid neighbour of the refusal above, so "emits
    /// nothing" cannot be satisfied by emitting nothing for everything.
    #[test]
    fn a_graph_with_statements_still_emits_a_header_and_a_body() {
        let mut g = graph_of(
            &[
                "https://example.org/s",
                "https://example.org/p",
                "https://example.org/o",
            ],
            &[(0, 1, 2)],
        );
        g.sort_canonical();
        assert!(emits_any_statement(&g));

        let mut turtle = String::new();
        write_turtle(&g, &mut turtle).expect("default-graph projection");
        assert!(turtle.contains("@prefix rdf:"), "header is still emitted");
        assert!(turtle.contains("https://example.org/s"), "body is emitted");

        let mut trig = String::new();
        write_trig(&g, &mut trig);
        assert!(trig.contains("@prefix rdf:"));
        assert!(trig.contains("https://example.org/s"));
    }

    /// THE EMITTED ROW ORDER DOES NOT MOVE WITH THE BASE.
    ///
    /// `sort_canonical` orders rows by rendered term text. If it rendered through the
    /// base in force, the document's statement order would become a function of
    /// `--base`: two exports of one dataset differing only in their base would carry the
    /// same statements in a different sequence, which is precisely the byte
    /// nondeterminism this serializer forbids.
    ///
    /// The fixture is chosen so the two orders genuinely DISAGREE — asserted below
    /// rather than assumed, so this cannot pass by the relative and absolute spellings
    /// happening to sort alike. `http://example.org/dir/a` abbreviates to `a` and
    /// `http://example.org/dir2/b` to `../dir2/b`; absolutely the `dir/` row leads
    /// (`/` < `2`), relatively the `../` row does (`.` < `a`).
    #[test]
    fn canonical_row_order_is_identical_with_and_without_a_base() {
        const BASE: &str = "http://example.org/dir/";
        const IN_DIR: &str = "http://example.org/dir/a";
        const IN_SIBLING: &str = "http://example.org/dir2/b";
        const PREDICATE: &str = "http://example.org/p";

        let base = BaseIri::parse(BASE).expect("fixture base is absolute");

        // The premise: the two spellings really do sort the other way round.
        let relative_in_dir = spell_iri(IN_DIR, Some(&base));
        let relative_sibling = spell_iri(IN_SIBLING, Some(&base));
        assert_eq!(relative_in_dir, "a");
        assert_eq!(relative_sibling, "../dir2/b");
        assert!(IN_DIR < IN_SIBLING, "absolutely, the dir/ row leads");
        assert!(
            relative_sibling < relative_in_dir,
            "relatively, the ../ row leads — the orders disagree, which is what makes \
             this fixture able to catch a base-sensitive sort"
        );

        // Rows inserted so neither input order nor a stable sort can produce the answer
        // by luck: the sibling (which relativization would promote) comes first.
        let rows = [(2, 0, 1), (1, 0, 2)];
        let iris = [PREDICATE, IN_DIR, IN_SIBLING];

        let mut absolute = graph_of(&iris, &rows);
        absolute.sort_canonical();

        let mut based = graph_of(&iris, &rows);
        based.base = Some(base);
        based.sort_canonical();

        assert_eq!(
            based.quads, absolute.quads,
            "the canonical sort keys must be the ABSOLUTE term text, so the emitted row \
             order is the same whatever base the document is written under"
        );
        // And the leading row is the absolutely-first one, not the relatively-first one.
        assert_eq!(based.quads[0].0, 1, "the `dir/` subject leads either way");
    }

    /// The Turtle emission of that same graph: the `@base` directive is present, the
    /// bodies are relativized, and the STATEMENT ORDER is the absolute one.
    #[test]
    fn turtle_under_a_base_emits_the_directive_and_keeps_the_absolute_row_order() {
        let mut g = graph_of(
            &[
                "http://example.org/p",
                "http://example.org/dir/a",
                "http://example.org/dir2/b",
            ],
            &[(2, 0, 1), (1, 0, 2)],
        );
        g.base = Some(BaseIri::parse("http://example.org/dir/").expect("absolute base"));
        g.sort_canonical();
        let ttl = to_turtle(&g).expect("turtle");

        assert!(
            ttl.starts_with("@base <http://example.org/dir/> .\n@prefix rdf:"),
            "the base directive leads the document, spelled absolutely: {ttl}"
        );
        let body = ttl
            .split_once("\n\n")
            .expect("header then body")
            .1
            .to_owned();
        assert_eq!(
            body, "<a> <../p> <../dir2/b> .\n<../dir2/b> <../p> <a> .\n",
            "IRIs are relativized, and the row order is the absolute-text order"
        );
    }

    /// One term's N-Triples surface as an owned `String`.
    ///
    /// Production has none: `sort_canonical` compares through reusable buffers and the
    /// serializers append, so nothing outside this module ever wants a term on its own.
    /// It survives here to reconstruct the key the sort used to build, which is what
    /// lets the ordering test compare against the old behaviour rather than against
    /// itself.
    fn render_term(g: &SerGraph, tid: usize) -> String {
        let mut out = String::new();
        write_term(g, &g.reifier_index(), tid, &mut out);
        out
    }

    fn to_ntriples(g: &SerGraph) -> Result<String, RdfDiagnostic> {
        let mut out = String::new();
        write_ntriples(g, &mut out)?;
        Ok(out)
    }

    fn to_turtle(g: &SerGraph) -> Result<String, RdfDiagnostic> {
        let mut out = String::new();
        write_turtle(g, &mut out)?;
        Ok(out)
    }

    #[test]
    fn deterministic_blank_label_matches_zero_timestamp_ulid() {
        // The raw blank-label shape is byte-identity critical: the W3C canonical
        // comparison relabels blanks and will NOT catch a label-shape regression, so
        // these exact strings are checked directly. Each is the 26-digit Crockford
        // Base32 rendering of the zero-timestamp ULID built from the counter.
        assert_eq!(
            deterministic_blank_label(0),
            "gts_00000000000000000000000000"
        );
        assert_eq!(
            deterministic_blank_label(1),
            "gts_00000000000000000000000001"
        );
        assert_eq!(
            deterministic_blank_label(31),
            "gts_0000000000000000000000000Z"
        );
        assert_eq!(
            deterministic_blank_label(32),
            "gts_00000000000000000000000010"
        );
        assert_eq!(
            deterministic_blank_label(1000),
            "gts_000000000000000000000000Z8"
        );
    }

    /// A single-quad graph `<s> <p> "<lit>"` over default-graph terms.
    fn lit_graph(lexical: &str, datatype_iri: &str) -> SerGraph {
        let mut g = SerGraph::default();
        // 0: s, 1: p, 2: datatype IRI, 3: literal
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some(datatype_iri.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Literal,
            value: Some(lexical.to_owned()),
            datatype: Some(2),
            lang: None,
            direction: None,
            reifier: None,
        });
        g.quads.push((0, 1, 3, None));
        g
    }

    #[test]
    fn decimal_lexical_preserved_verbatim_in_ntriples() {
        // The trailing zero of "0.90"^^xsd:decimal MUST survive verbatim: no
        // value-space canonicalization, no datatype narrowing.
        let g = lit_graph("0.90", "http://www.w3.org/2001/XMLSchema#decimal");
        let nt = to_ntriples(&g).expect("ntriples");
        assert!(
            nt.contains("\"0.90\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "raw N-Triples output must carry the verbatim lexical form, got: {nt}"
        );
    }

    #[test]
    fn turtle_begins_with_prefix_header() {
        let g = lit_graph("0.90", "http://www.w3.org/2001/XMLSchema#decimal");
        let ttl = to_turtle(&g).expect("turtle");
        let expected = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n";
        assert!(
            ttl.starts_with(expected),
            "Turtle must begin with the two @prefix lines, got: {ttl}"
        );
        // The IRI body stays full <...>, NOT abbreviated against the declared prefixes.
        assert!(
            ttl.contains("\"0.90\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "Turtle body keeps the verbatim lexical form + full datatype IRI"
        );
    }

    #[test]
    fn empty_turtle_is_empty_string() {
        let g = SerGraph::default();
        assert_eq!(to_turtle(&g).expect("turtle"), "");
    }

    #[test]
    fn ntriples_rejects_named_graph() {
        let mut g = lit_graph("x", "http://www.w3.org/2001/XMLSchema#string");
        // Re-point the literal as a plain literal and add a named-graph quad.
        g.quads.push((0, 1, 0, Some(0)));
        assert!(
            to_ntriples(&g).is_err(),
            "N-Triples must reject a named-graph quad"
        );
    }

    #[test]
    fn language_tag_with_direction_renders() {
        let mut g = SerGraph::default();
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Literal,
            value: Some("hi".to_owned()),
            datatype: None,
            lang: Some("en".to_owned()),
            direction: Some("ltr".to_owned()),
            reifier: None,
        });
        g.quads.push((0, 1, 2, None));
        let nt = to_ntriples(&g).expect("ntriples");
        assert!(nt.contains("\"hi\"@en--ltr"), "got: {nt}");
    }
}

#[cfg(test)]
pub(crate) mod term_walk_tests {
    //! The N-Quads and TriG term writers against their recursive references, and at a
    //! hundred thousand levels on a 128 KiB thread.

    use core::convert::Infallible;
    use core::fmt::Write as _;

    use purrdf_core::{Nested, TermValue, try_fold_nested};

    use super::{
        RDF_REIFIES, ReifierIndex, SerGraph, SerTerm, SerTermKind, write_term, write_trig_term,
    };

    /// Lower `value` into `graph`'s term table, a triple term through a self-reifier
    /// binding as the production lowering does, and return its term id. The IRI
    /// `http://example.org/p1` is lowered as `rdf:reifies`, so the TriG writer's one
    /// special spelling is reached.
    pub(crate) fn lower(graph: &mut SerGraph, value: &TermValue) -> usize {
        fn push(graph: &mut SerGraph, term: SerTerm) -> usize {
            graph.terms.push(term);
            graph.terms.len() - 1
        }
        fn shaped(kind: SerTermKind, value: Option<String>) -> SerTerm {
            SerTerm {
                kind,
                value,
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            }
        }
        let lowered = try_fold_nested(
            value,
            graph,
            |graph, value| {
                Ok::<_, Infallible>(Nested::Leaf(match value {
                    TermValue::Iri(iri) if iri == "http://example.org/p1" => push(
                        graph,
                        shaped(SerTermKind::Iri, Some(RDF_REIFIES.to_owned())),
                    ),
                    TermValue::Iri(iri) => push(graph, shaped(SerTermKind::Iri, Some(iri.clone()))),
                    TermValue::Blank { label, .. } => {
                        push(graph, shaped(SerTermKind::Bnode, Some(label.clone())))
                    }
                    TermValue::Literal {
                        lexical_form,
                        datatype,
                        language,
                        direction,
                    } => {
                        let datatype = (language.is_none()
                            && datatype != "http://www.w3.org/2001/XMLSchema#string")
                            .then(|| push(graph, shaped(SerTermKind::Iri, Some(datatype.clone()))));
                        push(
                            graph,
                            SerTerm {
                                kind: SerTermKind::Literal,
                                value: Some(lexical_form.clone()),
                                datatype,
                                lang: language.clone(),
                                direction: direction.map(|d| d.as_str().to_owned()),
                                reifier: None,
                            },
                        )
                    }
                    TermValue::Triple { s, p, o } => return Ok(Nested::Triple(&**s, &**p, &**o)),
                }))
            },
            |graph, _, s, p, o| {
                let id = graph.terms.len();
                graph.reifiers.push((id, (s, p, o), None));
                let mut term = shaped(SerTermKind::Triple, None);
                term.reifier = Some(id);
                Ok(push(graph, term))
            },
        );
        match lowered {
            Ok(id) => id,
        }
    }

    /// The recursive reference of `write_term_in` (and so [`write_term`]).
    fn reference_term(g: &SerGraph, ix: &ReifierIndex, tid: usize, out: &mut String) {
        let t = &g.terms[tid];
        match t.kind {
            SerTermKind::Iri => {
                let _ = write!(
                    out,
                    "<{}>",
                    purrdf_lex::iri_escape::escape(t.value.as_deref().unwrap_or(""))
                );
            }
            SerTermKind::Bnode => {
                let _ = write!(out, "_:{}", t.value.as_deref().unwrap_or(""));
            }
            SerTermKind::Literal => reference_literal(g, ix, t, out, reference_term),
            SerTermKind::Triple => {
                let (s, p, o) = ix.get(t.reifier.expect("bound")).expect("bound");
                out.push_str("<<( ");
                reference_term(g, ix, s, out);
                out.push(' ');
                reference_term(g, ix, p, out);
                out.push(' ');
                reference_term(g, ix, o, out);
                out.push_str(" )>>");
            }
        }
    }

    /// The recursive reference of [`write_trig_term`].
    fn reference_trig(g: &SerGraph, ix: &ReifierIndex, tid: usize, out: &mut String) {
        let t = &g.terms[tid];
        match t.kind {
            SerTermKind::Iri if t.value.as_deref() == Some(RDF_REIFIES) => {
                out.push_str("rdf:reifies");
            }
            SerTermKind::Literal => reference_literal(g, ix, t, out, reference_trig),
            SerTermKind::Triple => {
                let (s, p, o) = ix.get(t.reifier.expect("bound")).expect("bound");
                out.push_str("<<( ");
                reference_trig(g, ix, s, out);
                out.push(' ');
                reference_trig(g, ix, p, out);
                out.push(' ');
                reference_trig(g, ix, o, out);
                out.push_str(" )>>");
            }
            SerTermKind::Iri | SerTermKind::Bnode => reference_term(g, ix, tid, out),
        }
    }

    /// A literal as both references spell it, its datatype written by `datatype`.
    fn reference_literal(
        g: &SerGraph,
        ix: &ReifierIndex,
        t: &SerTerm,
        out: &mut String,
        datatype: fn(&SerGraph, &ReifierIndex, usize, &mut String),
    ) {
        let _ = write!(
            out,
            "\"{}\"",
            purrdf_lex::literal_escape::escape(
                t.value.as_deref().unwrap_or(""),
                purrdf_lex::literal_escape::Carrier::Xml
            )
        );
        if let Some(lang) = &t.lang {
            let _ = write!(out, "@{lang}");
            if let Some(direction) = t.direction.as_deref() {
                let _ = write!(out, "--{direction}");
            }
        } else if let Some(dt) = t.datatype {
            out.push_str("^^");
            datatype(g, ix, dt, out);
        }
    }

    /// Both work-list writers spell every generated term exactly as their recursive
    /// references do, nested triple terms included.
    #[test]
    fn the_term_writers_agree_with_their_recursive_references_on_generated_terms() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::IriPredicates,
            );
            nested += usize::from(budget < 7);
            let mut graph = SerGraph::default();
            let id = lower(&mut graph, &value);
            let ix = graph.reifier_index();
            let (mut written, mut expected) = (String::new(), String::new());
            write_term(&graph, &ix, id, &mut written);
            reference_term(&graph, &ix, id, &mut expected);
            assert_eq!(written, expected, "seed {seed}: {value:?}");
            let (mut written, mut expected) = (String::new(), String::new());
            write_trig_term(&graph, &ix, id, &mut written);
            reference_trig(&graph, &ix, id, &mut expected);
            assert_eq!(written, expected, "seed {seed}: {value:?}");
        }
        assert!(nested > 0, "some generated term nests a triple term in one");
    }

    /// A triple term a hundred thousand levels deep is written both ways on a thread
    /// whose whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_written_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let value = purrdf_core::term_fixture::triple_chain(LEVELS);
            let mut graph = SerGraph::default();
            let id = lower(&mut graph, &value);
            drop(value);
            let ix = graph.reifier_index();
            let level = "<<( <http://example.org/s> <http://example.org/p> ".len() + " )>>".len();
            let innermost = "<http://example.org/o>".len();
            let mut written = String::new();
            write_term(&graph, &ix, id, &mut written);
            assert_eq!(written.len(), LEVELS * level + innermost);
            let mut written = String::new();
            write_trig_term(&graph, &ix, id, &mut written);
            assert_eq!(written.len(), LEVELS * level + innermost);
        })
        .expect("the thread starts");
    }
}

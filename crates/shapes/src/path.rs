// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL property path evaluation.
//!
//! Evaluates a [`Path`] against a frozen [`::purrdf::RdfDataset`], returning the set of
//! value nodes reachable from a given focus node. All six SHACL §2.3.1 path forms
//! are supported: predicate, inverse, sequence, alternative, and the three closure
//! paths (`zeroOrMore`, `oneOrMore`, `zeroOrOne`). Pattern lookups are ID-native
//! ([`quads_for_pattern_ids`]) — only the matched value nodes are resolved to the
//! native [`Term`] model, no per-quad materialization. Closure evaluation walks a
//! worklist with a visited set, so cyclic data graphs terminate; result order is
//! deterministic (first-seen order over the deterministic pattern lookups).
//!
//! The focus stays a native [`Term`]: a SHACL-AF node expression may drive path
//! evaluation from a term that is not interned in the data graph (a `sh:this`
//! Constant), in which case non-reflexive steps yield nothing while a reflexive
//! closure step still yields the focus itself.
//!
//! # ONE evaluator
//!
//! There is a single recursive walk here, and it is driven by a LOWERED path — the
//! crate-internal `LoweredPath`, deliberately not linked because it is not part of
//! this crate's public surface — whose predicate steps are binding-row slots rather
//! than IRIs to hash. Every caller reaches it: the validation hot path and the change expansion
//! hand it a stage-0 lowering they already hold, and the public [`Path`]-driven
//! entry points below lower their argument first — exactly as
//! [`crate::expression`]'s public node-expression entry points do for an expression
//! nobody lowered.
//!
//! It used to be two walks. The second one took a `Path` directly, re-resolved each
//! predicate IRI at every step of every focus node, and rebuilt the entire subtree of
//! an `sh:inversePath` over a composite on each visit — the cost the lowering exists
//! to remove, kept alive in the one evaluator nothing on the hot path called, and
//! reachable the moment a new surface picked the wrong entry point. The change
//! expansion did exactly that.

use crate::data_view::ShaclRead;

use ::purrdf::{IdSet, IdVec, TermId, smallvec};

use crate::data::{GraphFilter, quads_for_pattern_ids, resolve_id};
use crate::plan::{DatasetBinding, LoweredPath, lower_standalone_path};
use crate::shapes::Path;
use crate::term::{NamedNode, Term, term_id_to_native};

/// Evaluate a SHACL property path from `focus`, returning all reachable value
/// nodes in the default graph.
///
/// The result set is deduplicated (preserving first occurrence order) as SHACL
/// specifies value nodes as a set.  If `focus` is a `Literal` or cannot serve
/// as a subject, non-reflexive steps return no matches (a reflexive closure
/// step may still yield the focus itself).
///
/// # Traversal strategy
///
/// `path` carries no lowering — a caller holding a parsed [`Path`] has no
/// preparation to have lowered it against — so one is made for this call and the
/// walk runs on it. Past that the traversal is the ordinary one: when `focus` is
/// interned in `ds` (the common case — a data-graph node) the whole of it runs in id
/// space, every step maps matched `QuadIds` to `.o`/`.s` [`TermId`]s, frontiers and
/// closures dedup on `Copy` ids, and only the deduped result set is resolved to the
/// native [`Term`] model at the end.
///
/// When `focus` is NOT interned (a SHACL-AF node expression may drive evaluation
/// from a `sh:this` Constant that never appears in the data), it has no id and
/// therefore no outgoing/incoming quads: every STEP is empty, and the only value
/// a path can yield is the focus itself via reflexive inclusion
/// (`sh:zeroOrMore` / `sh:zeroOrOne`). That case returns the focus term verbatim.
pub fn eval(ds: &impl ShaclRead, focus: &Term, path: &Path) -> Vec<Term> {
    let lowering = lower_standalone_path(path);
    let binding = lowering.bind(ds);
    // The walk reports exactly one error — a slot the lowering never handed out —
    // and the two lines above are why it cannot arrive here: the lowering that names
    // the slots and the row that answers them come from the same `ShapeWalk`, made in
    // this call, so the row has one entry per slot the lowered path can name. The
    // empty answer is what a caller would receive from a path that matched nothing,
    // and this module's own tests drive every path form through this function and
    // require a non-empty result, so a lowering that ever did come apart would fail
    // there by name rather than answer short here.
    eval_planned(ds, focus, lowering.path(), &binding).unwrap_or_else(|_| Vec::new())
}

/// Id-native value-node producer: the deduped set of value nodes reachable from
/// `focus` along `path`, in interned [`TermId`] space (first-seen order).
///
/// Returns `Some(ids)` for an **interned** focus — the common case, where every
/// value node originates from a real quad and therefore has a `TermId`. The
/// caller resolves an id to an owned [`Term`] only when it actually needs the
/// term's content or records a violation, so a value node that participates only
/// in identity/set operations is never materialized.
///
/// Returns `None` for a **non-interned** focus (a SHACL-AF node expression may
/// drive evaluation from a `sh:this` Constant that never appears in the data). Its
/// value nodes have no id and must be produced in the owned-[`Term`] model by
/// [`eval`]; that owned-term fallback is a genuine necessity, not optionality.
pub fn eval_ids(ds: &impl ShaclRead, focus: &Term, path: &Path) -> Option<IdVec> {
    let focus_id = resolve_id(ds, focus)?;
    let lowering = lower_standalone_path(path);
    let binding = lowering.bind(ds);
    // Unreachable for the reason [`eval`] states, and answered the same way.
    Some(
        eval_planned_ids_from_id(ds, focus_id, lowering.path(), &binding)
            .unwrap_or_else(|_| IdVec::new()),
    )
}

// ── The evaluator ──────────────────────────────────────────────────────────────
//
// The six path forms, driven by a path whose predicate steps were resolved to
// dataset identities once, at bind, instead of once per focus node.

/// First-seen-order membership over an id accumulator, allocation-free while the
/// accumulator is small.
///
/// A path frontier needs a SET, and [`IdSet`] is the right shape for one — but a
/// `HashSet` allocates its table on its first insert, and a path is evaluated
/// once per focus node, so that first insert is one allocation per focus node,
/// every focus node, for a set that in the overwhelming majority of real shapes
/// holds one or two ids. It was the whole of `path_sequence`'s and both closure
/// paths' measured cost above the floor.
///
/// So membership is answered by a linear scan over the accumulator the caller is
/// already building, until that accumulator reaches [`Self::LINEAR_MAX`]; past
/// that the set is built once from what is already there and every later probe
/// hashes. Both regimes answer the same question, in the same order, so the value
/// nodes a path produces and the order it produces them in are unchanged — and
/// the asymptotics are unchanged too, because the quadratic regime is bounded by
/// a constant.
#[derive(Default)]
struct FrontierDedup {
    /// The hashed set, once the accumulator has outgrown the linear scan. `None`
    /// until then, and a `None` here has never allocated.
    hashed: Option<IdSet>,
}

impl FrontierDedup {
    /// The accumulator length past which probing switches from a linear scan to a
    /// hash lookup. A scan of this many `Copy` ids is a couple of cache lines and
    /// beats hashing one; past it the scan would start to cost more than the
    /// allocation it avoids.
    const LINEAR_MAX: usize = 16;

    /// Forget every id admitted so far, keeping any table already paid for.
    ///
    /// Used between the steps of a sequence, where each step dedups its own
    /// frontier. Once spilled it stays spilled: the table is already allocated,
    /// so there is nothing left to save by going back to the scan.
    fn clear(&mut self) {
        if let Some(set) = &mut self.hashed {
            set.clear();
        }
    }

    /// Whether `id` is new, given `accumulated` — every id admitted since the last
    /// [`Self::clear`], in order. The caller appends `id` to `accumulated` exactly
    /// when this returns `true`, which is what keeps the two in step.
    fn insert(&mut self, accumulated: &[TermId], id: TermId) -> bool {
        if let Some(set) = &mut self.hashed {
            return set.insert(id);
        }
        if accumulated.len() < Self::LINEAR_MAX {
            return !linear_contains(accumulated, id);
        }
        let mut set: IdSet =
            IdSet::with_capacity_and_hasher(accumulated.len() * 2, ::purrdf::FastHasher::default());
        set.extend(accumulated.iter().copied());
        let fresh = set.insert(id);
        self.hashed = Some(set);
        fresh
    }
}

/// Ids per chunk of [`linear_contains`]: four `u32` lanes, one 128-bit vector.
const DEDUP_CHUNK: usize = 4;

/// Whether `id` is one of `ids`, testing every id with no early exit.
///
/// Four ids at a time: each chunk's answers are `0x0000_0000`/`0xFFFF_FFFF`
/// lanes, OR-ed lane-wise into an accumulator that is folded once after the
/// last chunk, and the at most three ids after the last whole chunk are tested
/// one at a time. A loop that can stop at the first hit (the slice's own
/// `contains`) is not vectorized; with no exit, each chunk is one packed
/// compare on every target with vector compares. The chunk is four lanes rather than eight because the linear stage
/// holds fewer than [`FrontierDedup::LINEAR_MAX`] ids, so a wider chunk would
/// leave most frontiers entirely in the one-at-a-time tail.
#[allow(
    clippy::inline_always,
    reason = "the chunk loop must inline into the dedup probe so the probed id stays a \
              broadcast register and each chunk's compares pack"
)]
#[inline(always)]
fn linear_contains(ids: &[TermId], id: TermId) -> bool {
    let (chunks, tail) = ids.as_chunks::<DEDUP_CHUNK>();
    let mut acc = [0_u32; DEDUP_CHUNK];
    for chunk in chunks {
        for (lane, seen) in acc.iter_mut().zip(chunk) {
            *lane |= u32::from(*seen == id).wrapping_neg();
        }
    }
    let mut hit = acc.iter().fold(0, |any, &lane| any | lane) != 0;
    for seen in tail {
        hit |= *seen == id;
    }
    hit
}

/// Id-native value-node producer for a focus node whose interned identity is
/// already known, driven by a lowered path.
///
/// This is the validation hot path. Every predicate step reads an ARRAY SLOT the
/// lowering assigned rather than hashing an IRI into the dataset's dictionary, so a
/// property shape evaluated across a million focus nodes resolves its predicate
/// exactly once.
///
/// # Errors
///
/// Returns an error when the lowering names a slot the walk never handed out — a
/// defect in this crate, never in a caller's data.
pub(crate) fn eval_planned_ids_from_id(
    ds: &impl ShaclRead,
    focus_id: TermId,
    path: &LoweredPath,
    binding: &DatasetBinding,
) -> Result<IdVec, String> {
    let ids = eval_planned_inner_ids(ds, focus_id, path, binding)?;
    let mut seen = FrontierDedup::default();
    let mut out: IdVec = IdVec::with_capacity(ids.len());
    for id in ids {
        if seen.insert(&out, id) {
            out.push(id);
        }
    }
    Ok(out)
}

/// [`eval`] driven by a lowered path, for a focus node that may not be interned.
///
/// # Errors
///
/// As [`eval_planned_ids_from_id`].
pub(crate) fn eval_planned(
    ds: &impl ShaclRead,
    focus: &Term,
    path: &LoweredPath,
    binding: &DatasetBinding,
) -> Result<Vec<Term>, String> {
    let Some(focus_id) = resolve_id(ds, focus) else {
        // Non-interned focus: it has no id and therefore no incoming/outgoing
        // quads, so every predicate/inverse STEP is empty. The only value a path
        // can yield is the focus term itself, via reflexive (zero-length)
        // inclusion.
        if admits_empty_planned_path(path) {
            return Ok(vec![focus.clone()]);
        }
        return Ok(Vec::new());
    };
    let ids = eval_planned_ids_from_id(ds, focus_id, path, binding)?;
    let mut nodes: Vec<Term> = Vec::with_capacity(ids.len());
    for id in ids {
        nodes.push(term_id_to_native(ds, id));
    }
    Ok(nodes)
}

/// [`admits_empty_path`] for a lowered path.
///
/// Stated over the lowering rather than delegated to the AST so a lowered path is
/// self-sufficient: the evaluator holds no `Path` to consult for the one question
/// that decides a non-interned focus's whole result.
fn admits_empty_planned_path(path: &LoweredPath) -> bool {
    match path {
        // A predicate step is never zero-length.
        LoweredPath::Predicate(_) | LoweredPath::InversePredicate(_) => false,
        // Inversion does not change reflexivity: `^p` admits empty iff `p` does,
        // so the already-inverted lowering answers for the declared `^(…)`.
        LoweredPath::InvertedComposite(inverted) => admits_empty_planned_path(inverted),
        LoweredPath::OneOrMore(inner) => admits_empty_planned_path(inner),
        // A sequence admits empty only if EVERY step can be taken in zero steps.
        LoweredPath::Sequence(parts) => parts.iter().all(admits_empty_planned_path),
        // An alternative admits empty if ANY branch does.
        LoweredPath::Alternative(parts) => parts.iter().any(admits_empty_planned_path),
        // The reflexive closures always admit the zero-length path.
        LoweredPath::ZeroOrMore(_) | LoweredPath::ZeroOrOne(_) => true,
    }
}

/// The recursive id-native evaluator for a lowered path and an interned `focus`.
fn eval_planned_inner_ids(
    ds: &impl ShaclRead,
    focus: TermId,
    path: &LoweredPath,
    binding: &DatasetBinding,
) -> Result<IdVec, String> {
    Ok(match path {
        LoweredPath::Predicate(slot) => match binding.term(*slot)? {
            Some(p_id) => {
                quads_for_pattern_ids(ds, Some(focus), Some(p_id), None, GraphFilter::DefaultGraph)
                    .map(|q| q.o)
                    .collect()
            }
            None => IdVec::new(),
        },
        LoweredPath::InversePredicate(slot) => match binding.term(*slot)? {
            Some(p_id) => {
                quads_for_pattern_ids(ds, None, Some(p_id), Some(focus), GraphFilter::DefaultGraph)
                    .map(|q| q.s)
                    .collect()
            }
            None => IdVec::new(),
        },
        // Inverse of a composite path. The inversion was pushed inward at stage 0
        // and the result lowered like any other path, so there is nothing left to
        // rewrite here and nothing to re-resolve: this is an ordinary recursive
        // step over slots.
        LoweredPath::InvertedComposite(inverted) => {
            eval_planned_inner_ids(ds, focus, inverted, binding)?
        }
        LoweredPath::Sequence(parts) => {
            // Fold the frontier through each step, deduplicating per step
            // (first-seen order) so diamond-shaped graphs stay linear. The
            // scratch `next`/`seen` are hoisted out of the loop and cleared each
            // iteration so their capacity is reused across steps.
            let mut frontier: IdVec = smallvec![focus];
            let mut next: IdVec = IdVec::new();
            let mut seen = FrontierDedup::default();
            for part in parts {
                next.clear();
                seen.clear();
                for &node in &frontier {
                    for value in eval_planned_inner_ids(ds, node, part, binding)? {
                        if seen.insert(&next, value) {
                            next.push(value);
                        }
                    }
                }
                std::mem::swap(&mut frontier, &mut next);
            }
            frontier
        }
        LoweredPath::Alternative(parts) => {
            let mut out: IdVec = IdVec::new();
            for part in parts {
                out.extend(eval_planned_inner_ids(ds, focus, part, binding)?);
            }
            out
        }
        LoweredPath::ZeroOrMore(inner) => planned_closure_ids(ds, focus, inner, binding, true)?,
        LoweredPath::OneOrMore(inner) => planned_closure_ids(ds, focus, inner, binding, false)?,
        LoweredPath::ZeroOrOne(inner) => {
            let mut nodes: IdVec = smallvec![focus];
            nodes.extend(eval_planned_inner_ids(ds, focus, inner, binding)?);
            nodes
        }
    })
}

/// [`closure_ids`] over a lowered inner path.
fn planned_closure_ids(
    ds: &impl ShaclRead,
    focus: TermId,
    inner: &LoweredPath,
    binding: &DatasetBinding,
    reflexive: bool,
) -> Result<IdVec, String> {
    // `order` IS the visited set: every id the dedup admits is pushed to it, and
    // it is never popped, so the closure's membership question is answered against
    // the output it is already building rather than against a second copy of it.
    // The worklist is a cursor over that same sequence.
    let mut seen = FrontierDedup::default();
    let mut order: IdVec = IdVec::new();
    let mut cursor = 0;

    if reflexive {
        // The first id needs no membership probe — nothing has been admitted yet
        // — and the dedup reads `order` itself, so pushing IS recording it.
        order.push(focus);
    } else {
        for value in eval_planned_inner_ids(ds, focus, inner, binding)? {
            if seen.insert(&order, value) {
                order.push(value);
            }
        }
    }

    while cursor < order.len() {
        let node = order[cursor];
        cursor += 1;
        for value in eval_planned_inner_ids(ds, node, inner, binding)? {
            if seen.insert(&order, value) {
                order.push(value);
            }
        }
    }
    Ok(order)
}

/// Convert a [`Path`] to its term representation for use in `result_path`.
///
/// - `Predicate(p)` → `Term::NamedNode(p)` (a simple path IS its IRI);
/// - every other form → a deterministic blank node (`path_label`) standing
///   for the SHACL path structure, matching the spec's `sh:resultPath`
///   rendering (`[ sh:inversePath … ]`, sequence lists, …). The structure
///   itself travels in `ValidationResult::path_structure` and is emitted into
///   the report graph by `ValidationReport::to_ntriples`.
pub fn path_to_term(path: &Path) -> Term {
    match path {
        Path::Predicate(p) => Term::NamedNode(p.clone()),
        complex => Term::BlankNode(path_label(complex)),
    }
}

/// Render a [`Path`] in SPARQL 1.1 property-path surface syntax
/// (`^<p>`, `<a>/<b>`, `<a>|<b>`, `(<p>)*`, …).
///
/// Used for SHACL-SPARQL `$PATH` substitution (a property-shape `sh:sparql`
/// validator's `$PATH` placeholder is replaced with the shape's path in SPARQL
/// syntax) and as the seed for `path_label`. Composite sub-paths are always
/// parenthesised, so the rendering is unambiguous in any embedding position.
pub fn path_to_sparql(path: &Path) -> String {
    fn grouped(path: &Path) -> String {
        match path {
            Path::Predicate(_) => path_to_sparql(path),
            composite => format!("({})", path_to_sparql(composite)),
        }
    }
    match path {
        Path::Predicate(p) => format!("<{}>", p.as_str()),
        Path::Inverse(inner) => format!("^{}", grouped(inner)),
        Path::Sequence(parts) => parts.iter().map(grouped).collect::<Vec<_>>().join("/"),
        Path::Alternative(parts) => parts.iter().map(grouped).collect::<Vec<_>>().join("|"),
        Path::ZeroOrMore(inner) => format!("{}*", grouped(inner)),
        Path::OneOrMore(inner) => format!("{}+", grouped(inner)),
        Path::ZeroOrOne(inner) => format!("{}?", grouped(inner)),
    }
}

/// A deterministic blank-node label for a complex path: the SPARQL rendering
/// sanitised to blank-node-label-safe characters (`[A-Za-z0-9-]`, no trailing
/// `-`). Distinct paths get distinct-enough labels; equal paths always get the
/// SAME label, keeping report output byte-stable across runs.
fn path_label(path: &Path) -> String {
    let rendered = path_to_sparql(path);
    let mut label = String::with_capacity(rendered.len() + 8);
    label.push_str("path-");
    for c in rendered.chars() {
        // Path OPERATORS keep distinct spellings (they would all sanitise to
        // `-` otherwise, colliding e.g. `a/b` with `a|b`).
        match c {
            '^' => label.push_str("inv-"),
            '/' => label.push_str("-seq-"),
            '|' => label.push_str("-alt-"),
            '*' => label.push_str("-star"),
            '+' => label.push_str("-plus"),
            '?' => label.push_str("-opt"),
            c if c.is_ascii_alphanumeric() => label.push(c),
            _ => label.push('-'),
        }
    }
    while label.ends_with('-') {
        label.pop();
    }
    label
}

/// The first (leftmost) predicate IRI mentioned in a path, if any.
///
/// Used for predicate-keyed metadata lookups (e.g. graph-box roles) where a
/// single representative predicate suffices.
pub fn primary_predicate(path: &Path) -> Option<&NamedNode> {
    match path {
        Path::Predicate(p) => Some(p),
        Path::Inverse(inner)
        | Path::ZeroOrMore(inner)
        | Path::OneOrMore(inner)
        | Path::ZeroOrOne(inner) => primary_predicate(inner),
        Path::Sequence(parts) | Path::Alternative(parts) => {
            parts.first().and_then(primary_predicate)
        }
    }
}

// ── Inverse rewrite ─────────────────────────────────────────────────────────────

/// Rewrite the inverse of a composite path by pushing the inversion inward:
///
/// - `^(^p)      = p`
/// - `^(a/b/…/z) = ^z/…/^b/^a`
/// - `^(a|b)     = ^a|^b`
/// - `^(p*)      = (^p)*` (and likewise `+`, `?`)
///
/// STAGE 0 ONLY. The rewrite builds a new `Path` subtree, so it belongs where the
/// shapes graph is read and nowhere else. Its three callers are all once-per-shapes-
/// graph: [`crate::plan`]'s `lower_path` (an `sh:inversePath` over a composite,
/// which lowers to `LoweredPath::InvertedComposite`), the same module's reversal of
/// every [`crate::footprint`] trigger chain, and the footprint walk's own record of
/// what an inverse composite READS. Nothing per focus node or per changed row
/// inverts anything: by the time either of those walks runs, the inversion is
/// already inside the lowering they are handed.
pub(crate) fn invert(path: &Path) -> Path {
    match path {
        Path::Predicate(_) => Path::Inverse(Box::new(path.clone())),
        Path::Inverse(inner) => inner.as_ref().clone(),
        Path::Sequence(parts) => Path::Sequence(parts.iter().rev().map(invert).collect()),
        Path::Alternative(parts) => Path::Alternative(parts.iter().map(invert).collect()),
        Path::ZeroOrMore(inner) => Path::ZeroOrMore(Box::new(invert(inner))),
        Path::OneOrMore(inner) => Path::OneOrMore(Box::new(invert(inner))),
        Path::ZeroOrOne(inner) => Path::ZeroOrOne(Box::new(invert(inner))),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ::purrdf::RdfDataset;

    use super::*;
    use crate::term::Literal;

    // `NamedNode` is referenced through `super::*` (re-exported into scope by the
    // module's own `use`), so the test helpers below construct terms directly.

    fn load_data(ttl: &str) -> Arc<RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("turtle parse")
    }

    const DATA: &str = r"
        @prefix ex: <http://example.org/ns#> .
        ex:a ex:p ex:b .
        ex:a ex:p ex:c .
        ex:d ex:q ex:a .
    ";

    fn nn(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn pred(local: &str) -> Path {
        Path::Predicate(NamedNode::new_unchecked(format!(
            "http://example.org/ns#{local}"
        )))
    }

    #[test]
    fn predicate_path_returns_objects() {
        let data = load_data(DATA);
        let focus = nn("http://example.org/ns#a");
        let path = Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&nn("http://example.org/ns#b")));
        assert!(result.contains(&nn("http://example.org/ns#c")));
    }

    #[test]
    fn inverse_path_returns_subjects() {
        let data = load_data(DATA);
        let focus = nn("http://example.org/ns#a");
        let path = Path::Inverse(Box::new(Path::Predicate(NamedNode::new_unchecked(
            "http://example.org/ns#q",
        ))));
        let result = eval(&data, &focus, &path);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], nn("http://example.org/ns#d"));
    }

    #[test]
    fn literal_focus_returns_empty() {
        let data = load_data(DATA);
        let focus = Term::Literal(Literal::new_simple_literal("hello"));
        let path = Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
        assert_eq!(eval(&data, &focus, &path), [] as [_; 0]);
    }

    #[test]
    fn predicate_path_deduplicates() {
        let data = load_data(DATA);
        let focus = nn("http://example.org/ns#a");
        let path = Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
        let result = eval(&data, &focus, &path);
        // Should be exactly 2 distinct values
        assert_eq!(result.len(), 2);
    }

    // ── Sequence paths ─────────────────────────────────────────────────────────

    #[test]
    fn sequence_path_chains_predicates() {
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:p ex:b . ex:b ex:q ex:c . ex:b ex:q ex:d .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::Sequence(vec![pred("p"), pred("q")]);
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        assert_eq!(
            result,
            vec![nn("http://example.org/ns#c"), nn("http://example.org/ns#d")]
        );
    }

    #[test]
    fn sequence_path_diamond_deduplicates() {
        // a → {b, c} → d : d is reachable twice but reported once.
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:p ex:b , ex:c . ex:b ex:q ex:d . ex:c ex:q ex:d .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::Sequence(vec![pred("p"), pred("q")]);
        let result = eval(&data, &focus, &path);
        assert_eq!(result, vec![nn("http://example.org/ns#d")]);
    }

    // ── Alternative paths ──────────────────────────────────────────────────────

    #[test]
    fn alternative_path_unions_branches() {
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:p ex:b . ex:a ex:q ex:c . ex:a ex:p ex:c .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::Alternative(vec![pred("p"), pred("q")]);
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        // ex:c is reachable via both branches but reported once (set semantics).
        assert_eq!(
            result,
            vec![nn("http://example.org/ns#b"), nn("http://example.org/ns#c")]
        );
    }

    // ── Closure paths ──────────────────────────────────────────────────────────

    #[test]
    fn zero_or_more_includes_focus_and_terminates_on_cycle() {
        // a → b → c → a : a cyclic graph must terminate and include the focus.
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:next ex:b . ex:b ex:next ex:c . ex:c ex:next ex:a .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::ZeroOrMore(Box::new(pred("next")));
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        assert_eq!(
            result,
            vec![
                nn("http://example.org/ns#a"),
                nn("http://example.org/ns#b"),
                nn("http://example.org/ns#c")
            ]
        );
    }

    #[test]
    fn one_or_more_excludes_unreachable_focus() {
        // a → b → c (no cycle): oneOrMore from a yields {b, c}, not a.
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:next ex:b . ex:b ex:next ex:c .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::OneOrMore(Box::new(pred("next")));
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        assert_eq!(
            result,
            vec![nn("http://example.org/ns#b"), nn("http://example.org/ns#c")]
        );
    }

    #[test]
    fn one_or_more_includes_focus_when_cyclically_reached() {
        // a → b → a : oneOrMore from a reaches a in ≥1 step, so a IS a value.
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:next ex:b . ex:b ex:next ex:a .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::OneOrMore(Box::new(pred("next")));
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        assert_eq!(
            result,
            vec![nn("http://example.org/ns#a"), nn("http://example.org/ns#b")]
        );
    }

    #[test]
    fn zero_or_one_is_focus_plus_one_step() {
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:next ex:b . ex:b ex:next ex:c .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::ZeroOrOne(Box::new(pred("next")));
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        // Focus itself (zero steps) plus one step; c (two steps) excluded.
        assert_eq!(
            result,
            vec![nn("http://example.org/ns#a"), nn("http://example.org/ns#b")]
        );
    }

    // ── Nested combinations ────────────────────────────────────────────────────

    #[test]
    fn nested_sequence_of_alternative_and_closure() {
        // (p|q) / r* : from a, step p|q then any number of r.
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:p ex:b . ex:a ex:q ex:c .
            ex:b ex:r ex:d . ex:d ex:r ex:e .
        ",
        );
        let focus = nn("http://example.org/ns#a");
        let path = Path::Sequence(vec![
            Path::Alternative(vec![pred("p"), pred("q")]),
            Path::ZeroOrMore(Box::new(pred("r"))),
        ]);
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        assert_eq!(
            result,
            vec![
                nn("http://example.org/ns#b"),
                nn("http://example.org/ns#c"),
                nn("http://example.org/ns#d"),
                nn("http://example.org/ns#e")
            ]
        );
    }

    #[test]
    fn inverse_of_sequence_reverses_and_inverts() {
        // ^(p/q) from d must find a, since a p/q d.
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:p ex:b . ex:b ex:q ex:d .
        ",
        );
        let focus = nn("http://example.org/ns#d");
        let path = Path::Inverse(Box::new(Path::Sequence(vec![pred("p"), pred("q")])));
        let result = eval(&data, &focus, &path);
        assert_eq!(result, vec![nn("http://example.org/ns#a")]);
    }

    #[test]
    fn inverse_of_zero_or_more_includes_focus() {
        // ^(next*) from b: everything that reaches b via next*, plus b itself
        // (zero steps).
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:next ex:b .
        ",
        );
        let focus = nn("http://example.org/ns#b");
        let path = Path::Inverse(Box::new(Path::ZeroOrMore(Box::new(pred("next")))));
        let mut result = eval(&data, &focus, &path);
        crate::term::sort_terms_canonical(&mut result);
        assert_eq!(
            result,
            vec![nn("http://example.org/ns#a"), nn("http://example.org/ns#b")]
        );
    }

    // ── Non-interned focus: reflexive inclusion only ───────────────────────────

    #[test]
    fn non_interned_focus_yields_focus_only_for_reflexive_paths() {
        let data = load_data(DATA);
        // A focus term that never appears in the data graph, so it has no
        // interned id and no incoming/outgoing quads: every step is empty.
        let focus = nn("http://example.org/ns#not-in-graph");

        // Reflexive paths (admit the zero-length path) return exactly {focus}.
        let reflexive: Vec<Path> = vec![
            Path::ZeroOrMore(Box::new(pred("p"))),
            Path::ZeroOrOne(Box::new(pred("p"))),
            // Sequence where every part is reflexive.
            Path::Sequence(vec![
                Path::ZeroOrMore(Box::new(pred("p"))),
                Path::ZeroOrOne(Box::new(pred("q"))),
            ]),
            // Alternative with a reflexive branch (the other branch is not).
            Path::Alternative(vec![pred("p"), Path::ZeroOrMore(Box::new(pred("q")))]),
        ];
        for path in &reflexive {
            assert_eq!(
                eval(&data, &focus, path),
                vec![focus.clone()],
                "reflexive path must yield the focus itself: {path:?}"
            );
        }

        // Non-reflexive paths (no zero-length match) return {}.
        let non_reflexive: Vec<Path> = vec![
            pred("p"),
            Path::OneOrMore(Box::new(pred("p"))),
            // Sequence with a non-reflexive part cannot be taken in zero steps.
            Path::Sequence(vec![Path::ZeroOrMore(Box::new(pred("p"))), pred("q")]),
        ];
        for path in &non_reflexive {
            assert!(
                eval(&data, &focus, path).is_empty(),
                "non-reflexive path must yield nothing: {path:?}"
            );
        }
    }

    // ── path_to_term / primary_predicate approximations ────────────────────────

    #[test]
    fn path_to_term_simple_is_iri_complex_is_deterministic_bnode() {
        // A plain predicate path is its IRI.
        assert_eq!(path_to_term(&pred("p")), nn("http://example.org/ns#p"));
        // A complex path is a blank node whose label is deterministic: the same
        // path yields the same label, distinct paths yield distinct labels.
        let seq = Path::Sequence(vec![pred("p"), pred("q")]);
        let alt = Path::Alternative(vec![pred("p"), pred("q")]);
        let (t_seq, t_alt) = (path_to_term(&seq), path_to_term(&alt));
        assert!(matches!(t_seq, Term::BlankNode(_)), "sequence → bnode");
        assert!(matches!(t_alt, Term::BlankNode(_)), "alternative → bnode");
        assert_ne!(t_seq, t_alt, "sequence and alternative labels must differ");
        assert_eq!(path_to_term(&seq), t_seq, "labels are deterministic");
    }

    #[test]
    fn path_to_sparql_renders_surface_syntax() {
        let p = "<http://example.org/ns#p>";
        let q = "<http://example.org/ns#q>";
        assert_eq!(path_to_sparql(&pred("p")), p);
        assert_eq!(
            path_to_sparql(&Path::Inverse(Box::new(pred("p")))),
            format!("^{p}")
        );
        assert_eq!(
            path_to_sparql(&Path::Sequence(vec![pred("p"), pred("q")])),
            format!("{p}/{q}")
        );
        assert_eq!(
            path_to_sparql(&Path::Alternative(vec![pred("p"), pred("q")])),
            format!("{p}|{q}")
        );
        assert_eq!(
            path_to_sparql(&Path::ZeroOrMore(Box::new(pred("p")))),
            format!("{p}*")
        );
        // Composite sub-paths are parenthesised.
        assert_eq!(
            path_to_sparql(&Path::OneOrMore(Box::new(Path::Sequence(vec![
                pred("p"),
                pred("q")
            ])))),
            format!("({p}/{q})+")
        );
    }

    #[test]
    fn primary_predicate_descends_composites() {
        let path = Path::Inverse(Box::new(Path::Sequence(vec![
            Path::ZeroOrOne(Box::new(pred("p"))),
            pred("q"),
        ])));
        assert_eq!(
            primary_predicate(&path).map(NamedNode::as_str),
            Some("http://example.org/ns#p")
        );
    }

    /// The linear stage as it was written: the slice's own short-circuiting
    /// `contains`. Kept as the oracle of the chunked fold.
    fn reference_linear_contains(ids: &[TermId], id: TermId) -> bool {
        ids.contains(&id)
    }

    /// [`FrontierDedup`] as it was written, with the short-circuiting linear
    /// stage. Kept as the oracle of the rewritten dedup across the promotion
    /// into the hashed set.
    #[derive(Default)]
    struct ReferenceDedup {
        hashed: Option<IdSet>,
    }

    impl ReferenceDedup {
        fn clear(&mut self) {
            if let Some(set) = &mut self.hashed {
                set.clear();
            }
        }

        fn insert(&mut self, accumulated: &[TermId], id: TermId) -> bool {
            if let Some(set) = &mut self.hashed {
                return set.insert(id);
            }
            if accumulated.len() < FrontierDedup::LINEAR_MAX {
                return !reference_linear_contains(accumulated, id);
            }
            let mut set: IdSet = IdSet::with_capacity_and_hasher(
                accumulated.len() * 2,
                ::purrdf::FastHasher::default(),
            );
            set.extend(accumulated.iter().copied());
            let fresh = set.insert(id);
            self.hashed = Some(set);
            fresh
        }
    }

    // A deterministic SplitMix64 step: the tests' only source of variety.
    use purrdf_testkit::rng::splitmix64_next as mix;

    /// Every frontier size of the linear stage, 0 through 16, and a few past it:
    /// each id is found wherever it sits (a hit at every position, including in
    /// the one-at-a-time rows after the last whole chunk), and ids that are not
    /// there are not found.
    #[test]
    fn linear_contains_matches_the_slice_scan_at_every_size() {
        for len in 0..=FrontierDedup::LINEAR_MAX + 4 {
            let ids: Vec<TermId> = (0..len)
                .map(|i| TermId::from_index(u32::try_from(3 * i + 1).unwrap()))
                .collect();
            for (position, &id) in ids.iter().enumerate() {
                assert!(linear_contains(&ids, id), "len {len}: position {position}");
                assert!(reference_linear_contains(&ids, id));
            }
            for miss in [0, 2, 3, 3 * u32::try_from(len).unwrap() + 1, u32::MAX - 1] {
                let id = TermId::from_index(miss);
                assert_eq!(
                    linear_contains(&ids, id),
                    reference_linear_contains(&ids, id),
                    "len {len}: probe {miss}"
                );
            }
            assert!(!linear_contains(&ids, TermId::from_index(0)), "len {len}");
        }
    }

    /// Random frontiers with repeats, at every size of the linear stage: the
    /// fold and the slice scan agree on every probe.
    #[test]
    fn linear_contains_matches_the_slice_scan_on_random_frontiers() {
        let mut state = 0xF207_u64;
        for len in 0..=FrontierDedup::LINEAR_MAX {
            for _ in 0..64 {
                let ids: Vec<TermId> = (0..len)
                    .map(|_| TermId::from_index((mix(&mut state) % 20) as u32))
                    .collect();
                for probe in 0..22 {
                    let id = TermId::from_index(probe);
                    assert_eq!(
                        linear_contains(&ids, id),
                        reference_linear_contains(&ids, id),
                        "len {len}: {ids:?} probe {probe}"
                    );
                }
            }
        }
    }

    /// The dedup admits the same ids in the same order as the one it replaced,
    /// through the linear stage, across the promotion into the hashed set at
    /// [`FrontierDedup::LINEAR_MAX`] admitted ids, and after a `clear`, for
    /// streams whose repeats land on either side of the boundary.
    #[test]
    fn frontier_dedup_admits_what_the_reference_admits() {
        let mut state = 0xDED0_u64;
        for space in [4_u64, 15, 16, 17, 24, 64] {
            for _ in 0..32 {
                let mut dedup = FrontierDedup::default();
                let mut reference = ReferenceDedup::default();
                for _step in 0..2 {
                    let mut out: Vec<TermId> = Vec::new();
                    let mut expected: Vec<TermId> = Vec::new();
                    for _ in 0..48 {
                        let id = TermId::from_index((mix(&mut state) % space) as u32);
                        let fresh = dedup.insert(&out, id);
                        assert_eq!(
                            fresh,
                            reference.insert(&expected, id),
                            "space {space}: {id:?} after {out:?}"
                        );
                        if fresh {
                            out.push(id);
                            expected.push(id);
                        }
                    }
                    assert_eq!(out, expected);
                    dedup.clear();
                    reference.clear();
                }
            }
        }
    }

    /// The promotion boundary exactly: with `LINEAR_MAX - 1` ids admitted the
    /// probe is still linear, with `LINEAR_MAX` it builds the set, and a repeat
    /// of any admitted id is refused on both sides while a fresh id is admitted.
    #[test]
    fn frontier_dedup_promotion_boundary() {
        let max = FrontierDedup::LINEAR_MAX;
        for admitted in [max - 1, max, max + 1] {
            let ids: Vec<TermId> = (0..admitted)
                .map(|i| TermId::from_index(u32::try_from(i).unwrap()))
                .collect();
            for (position, &repeat) in ids.iter().enumerate() {
                let mut dedup = FrontierDedup::default();
                assert!(
                    !dedup.insert(&ids, repeat),
                    "{admitted} admitted: position {position} is a repeat"
                );
                assert_eq!(dedup.hashed.is_some(), admitted >= max);
            }
            let fresh = TermId::from_index(u32::try_from(admitted).unwrap());
            let mut dedup = FrontierDedup::default();
            assert!(dedup.insert(&ids, fresh), "{admitted} admitted: fresh id");
            assert_eq!(dedup.hashed.is_some(), admitted >= max);
        }
    }
}

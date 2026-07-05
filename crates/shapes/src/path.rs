// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL property path evaluation.
//!
//! Evaluates a [`Path`] against a frozen [`RdfDataset`], returning the set of
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

use ::purrdf::{smallvec, IdSet, IdVec, RdfDataset, TermId};

use crate::data::{quads_for_pattern_ids, resolve_id, GraphFilter};
use crate::shapes::Path;
use crate::term::{term_id_to_native, NamedNode, Term};

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
/// When `focus` is interned in `ds` (the common case — a data-graph node), the
/// entire traversal runs in id space: every step maps matched `QuadIds` to
/// `.o`/`.s` [`TermId`]s, frontiers/closures dedup on a `Copy` [`IdSet`], and only
/// the deduped result set is resolved to the native [`Term`] model at the end. No
/// owned term is allocated per intermediate step, and diamond/cycle dedup hashes
/// interned ids rather than rendered strings.
///
/// When `focus` is NOT interned (a SHACL-AF node expression may drive evaluation
/// from a `sh:this` Constant that never appears in the data), it has no id and
/// therefore no outgoing/incoming quads: every STEP is empty, and the only value
/// a path can yield is the focus itself via reflexive inclusion
/// (`sh:zeroOrMore` / `sh:zeroOrOne`). That case keeps the native [`Term`]
/// traversal so the reflexive focus term is returned verbatim.
pub fn eval(ds: &RdfDataset, focus: &Term, path: &Path) -> Vec<Term> {
    let Some(focus_id) = resolve_id(ds, focus) else {
        // Non-interned focus: it has no id and therefore no incoming/outgoing
        // quads, so every predicate/inverse STEP is empty. The only value a path
        // can yield is the focus term itself, via reflexive (zero-length)
        // inclusion. So the whole traversal collapses to a single predicate:
        // return `{focus}` iff the path admits the empty path, else `{}`. A single
        // focus term needs no dedup.
        if admits_empty_path(path) {
            return vec![focus.clone()];
        }
        return Vec::new();
    };
    // Interned focus: id-native traversal, resolve only the deduped result set.
    let ids = eval_inner_ids(ds, focus_id, path);
    let mut seen: IdSet = IdSet::default();
    let mut nodes: Vec<Term> = Vec::with_capacity(ids.len());
    for id in ids {
        if seen.insert(id) {
            nodes.push(term_id_to_native(ds, id));
        }
    }
    nodes
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
pub fn eval_ids(ds: &RdfDataset, focus: &Term, path: &Path) -> Option<IdVec> {
    let focus_id = resolve_id(ds, focus)?;
    let ids = eval_inner_ids(ds, focus_id, path);
    let mut seen: IdSet = IdSet::default();
    let mut out: IdVec = IdVec::with_capacity(ids.len());
    for id in ids {
        if seen.insert(id) {
            out.push(id);
        }
    }
    Some(out)
}

/// Whether a SHACL property path matches the zero-length (reflexive) path — i.e.
/// whether it can relate a node to itself in zero steps. This is the sole thing
/// that governs a non-interned focus's result: with no incoming/outgoing quads,
/// every predicate/inverse step is empty, so the path can yield the focus itself
/// only when it admits the empty path.
fn admits_empty_path(path: &Path) -> bool {
    match path {
        // A predicate step is never zero-length.
        Path::Predicate(_) => false,
        // Inversion does not change reflexivity: `^p` admits empty iff `p` does.
        Path::Inverse(inner) | Path::OneOrMore(inner) => admits_empty_path(inner),
        // A sequence admits empty only if EVERY step can be taken in zero steps.
        Path::Sequence(parts) => parts.iter().all(admits_empty_path),
        // An alternative admits empty if ANY branch does.
        Path::Alternative(parts) => parts.iter().any(admits_empty_path),
        // The reflexive closures always admit the zero-length path.
        Path::ZeroOrMore(_) | Path::ZeroOrOne(_) => true,
    }
}

/// Resolve a predicate IRI to its interned id, if present in `ds`.
#[inline]
fn resolve_pred(ds: &RdfDataset, predicate: &NamedNode) -> Option<TermId> {
    resolve_id(ds, &Term::NamedNode(predicate.clone()))
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
fn invert(path: &Path) -> Path {
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

// ── Id-native traversal (interned focus) ───────────────────────────────────────
//
// Every frontier, worklist, and visited set is a `Copy` [`TermId`] rather than an
// owned [`Term`]. Every value a step produces is the object/subject of a real
// quad, so it is always interned and always has a `TermId`. First-seen order and
// per-step dedup are deterministic; the caller ([`eval`]) resolves the result to
// terms.

/// The recursive id-native path evaluator, for an interned `focus`.
fn eval_inner_ids(ds: &RdfDataset, focus: TermId, path: &Path) -> IdVec {
    match path {
        Path::Predicate(p) => match resolve_pred(ds, p) {
            Some(p_id) => {
                quads_for_pattern_ids(ds, Some(focus), Some(p_id), None, GraphFilter::DefaultGraph)
                    .map(|q| q.o)
                    .collect()
            }
            None => IdVec::new(),
        },
        Path::Inverse(inner) => match inner.as_ref() {
            // Inverse of a predicate: collect subjects of (?, p, focus).
            Path::Predicate(p) => match resolve_pred(ds, p) {
                Some(p_id) => quads_for_pattern_ids(
                    ds,
                    None,
                    Some(p_id),
                    Some(focus),
                    GraphFilter::DefaultGraph,
                )
                .map(|q| q.s)
                .collect(),
                None => IdVec::new(),
            },
            // Inverse of a composite path: push the inversion inward and evaluate.
            composite => eval_inner_ids(ds, focus, &invert(composite)),
        },
        Path::Sequence(parts) => {
            // Fold the frontier through each step, deduplicating per step
            // (first-seen order) so diamond-shaped graphs stay linear. The
            // scratch `next`/`seen` are hoisted out of the loop and cleared each
            // iteration so their capacity is reused across steps.
            let mut frontier: IdVec = smallvec![focus];
            let mut next: IdVec = IdVec::new();
            let mut seen: IdSet = IdSet::default();
            for part in parts {
                next.clear();
                seen.clear();
                for &node in &frontier {
                    for value in eval_inner_ids(ds, node, part) {
                        if seen.insert(value) {
                            next.push(value);
                        }
                    }
                }
                std::mem::swap(&mut frontier, &mut next);
            }
            frontier
        }
        Path::Alternative(parts) => parts
            .iter()
            .flat_map(|part| eval_inner_ids(ds, focus, part))
            .collect(),
        Path::ZeroOrMore(inner) => closure_ids(ds, focus, inner, true),
        Path::OneOrMore(inner) => closure_ids(ds, focus, inner, false),
        Path::ZeroOrOne(inner) => {
            let mut nodes: IdVec = smallvec![focus];
            nodes.extend(eval_inner_ids(ds, focus, inner));
            nodes
        }
    }
}

/// The (reflexive-)transitive closure of `inner` from an interned `focus`: a
/// breadth-first worklist walk with a visited set, so cyclic graphs terminate.
/// `reflexive` includes the focus node itself (`zeroOrMore`); otherwise the walk
/// starts from the focus's direct step values (`oneOrMore`). The visited set is an
/// [`IdSet`] over `Copy` [`TermId`]s; first-seen order is preserved.
fn closure_ids(ds: &RdfDataset, focus: TermId, inner: &Path, reflexive: bool) -> IdVec {
    let mut seen: IdSet = IdSet::default();
    let mut order: IdVec = IdVec::new();
    let mut worklist: IdVec = IdVec::new();

    if reflexive {
        seen.insert(focus);
        order.push(focus);
        worklist.push(focus);
    } else {
        for value in eval_inner_ids(ds, focus, inner) {
            if seen.insert(value) {
                order.push(value);
                worklist.push(value);
            }
        }
    }

    let mut cursor = 0;
    while cursor < worklist.len() {
        let node = worklist[cursor];
        cursor += 1;
        for value in eval_inner_ids(ds, node, inner) {
            if seen.insert(value) {
                order.push(value);
                worklist.push(value);
            }
        }
    }
    order
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
        crate::text_ingest::parse_turtle_to_dataset(ttl).expect("turtle parse")
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
        result.sort_by_key(Term::to_string);
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
        assert!(eval(&data, &focus, &path).is_empty());
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
        result.sort_by_key(Term::to_string);
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
        result.sort_by_key(Term::to_string);
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
        result.sort_by_key(Term::to_string);
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
        result.sort_by_key(Term::to_string);
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
        result.sort_by_key(Term::to_string);
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
        result.sort_by_key(Term::to_string);
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
        result.sort_by_key(Term::to_string);
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
        result.sort_by_key(Term::to_string);
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
}

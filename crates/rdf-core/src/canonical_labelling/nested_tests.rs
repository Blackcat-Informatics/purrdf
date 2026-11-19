// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The lowering of nested triple terms to term nodes over a dataset view.
//!
//! Each case builds a dataset, labels it through the view, and compares the
//! result with the lowered set the format text's section "Nested terms,
//! lowering and term nodes" gives for it, written here by hand as typed lines
//! whose term-node sources are computed from the text's construction and
//! labelled by the definition alone ([`label_with_walk`]). Agreement covers
//! the line list, every source identifier, every position, the canonical line
//! sequence and its digest, the structural record (`guaranteed` included) and
//! the totals of the walk.

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_iri::vocab::rdf::DIR_LANG_STRING;
use purrdf_lex::cbor::head::{ARRAY, UNSIGNED, push_head, write_bytes, write_text};
use purrdf_testkit::error_capture::CallerRefusal;
use purrdf_xsd::datatype::XSD_NON_NEGATIVE_INTEGER;

use super::view::{Run, lowered};
use super::{
    Direction, Labelled, Labelling, LabellingCapacity, Limit, Line, LineSet, LoweringRefusal,
    Record, Scope, Term, ViewError, label, label_view, label_view_with_capacity, label_with_walk,
    optimised, write_line_with,
};
use crate::{
    BlankScope, DatasetView, GraphMatch, QuadIds, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfStoreCapabilities, RdfTextDirection, TermId, TermRef, TermValue,
};

/// The spelling of the hash context of term nodes.
const CONTEXT: &str = "gts2 nested term";

fn iri(name: &str) -> String {
    format!("http://e/{name}")
}

/// `E(bytes)` as a CBOR byte string.
fn byte_string(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write_bytes(&mut out, bytes).expect("writes to a Vec");
    out
}

fn push_text(out: &mut Vec<u8>, text: &str) {
    write_text(out, text).expect("writes to a Vec");
}

/// The payload of a nested-term record: `E([5, [[1], [1], [1], [1]]])`.
fn nested_payload() -> Vec<u8> {
    let mut out = Vec::new();
    push_head(&mut out, ARRAY, 2);
    push_head(&mut out, UNSIGNED, 5);
    push_head(&mut out, ARRAY, 4);
    for _ in 0..4 {
        push_head(&mut out, ARRAY, 1);
        push_head(&mut out, UNSIGNED, 1);
    }
    out
}

/// `K` of an IRI: the byte string of `E([1, text])`.
fn k_iri(name: &str) -> Vec<u8> {
    let mut v = Vec::new();
    push_head(&mut v, ARRAY, 2);
    push_head(&mut v, UNSIGNED, 1);
    push_text(&mut v, &iri(name));
    byte_string(&v)
}

/// `K` of a literal: the byte string of `E([2, lexical, form, x])`.
fn k_literal(lexical: &str, form: u64, x: &str) -> Vec<u8> {
    let mut v = Vec::new();
    push_head(&mut v, ARRAY, 4);
    push_head(&mut v, UNSIGNED, 2);
    push_text(&mut v, lexical);
    push_head(&mut v, UNSIGNED, form);
    push_text(&mut v, x);
    byte_string(&v)
}

/// The source identifier of a blank node: `E([ordinal, label])`.
fn blank_source(ordinal: u64, label: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    push_head(&mut out, ARRAY, 2);
    push_head(&mut out, UNSIGNED, ordinal);
    out.extend_from_slice(&byte_string(label));
    out
}

/// `K` of a blank node: `E([0, h(b)])`.
fn k_blank(ordinal: u64, label: &[u8]) -> Vec<u8> {
    let h = purrdf_hash::blake3::derive_key(CONTEXT, &blank_source(ordinal, label));
    let mut out = Vec::new();
    push_head(&mut out, ARRAY, 2);
    push_head(&mut out, UNSIGNED, 0);
    out.extend_from_slice(&byte_string(&h));
    out
}

/// `E([d])`: the source identifier of a term node, and `K` of its nested term.
fn node(d: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::new();
    push_head(&mut out, ARRAY, 1);
    out.extend_from_slice(&byte_string(d));
    out
}

/// `d` of the nested term of the sources `k`, and the length of its preimage.
fn d(k: [&[u8]; 3]) -> ([u8; 32], usize) {
    let mut message = Vec::new();
    push_head(&mut message, ARRAY, 3);
    for source in k {
        message.extend_from_slice(&byte_string(source));
    }
    (
        purrdf_hash::blake3::derive_key(CONTEXT, &message),
        message.len(),
    )
}

fn t(name: &str) -> Term {
    Term::iri(iri(name).as_bytes())
}

fn quad(s: Term, p: Term, o: Term) -> Line {
    Line::Quad {
        subject: s,
        predicate: p,
        object: o,
        graph: None,
    }
}

fn record(terms: Vec<Term>) -> Line {
    Line::Record {
        payload: nested_payload().into(),
        terms,
    }
}

/// A dataset written term by term.
struct Data(RdfDatasetBuilder);

impl Data {
    fn new() -> Self {
        Self(RdfDatasetBuilder::new())
    }

    fn iri(&mut self, name: &str) -> TermId {
        self.0.intern_iri(&iri(name)).expect("interns")
    }

    fn iris<const N: usize>(&mut self, names: [&str; N]) -> [TermId; N] {
        names.map(|name| self.iri(name))
    }

    fn blank(&mut self, label: &str, scope: u32) -> TermId {
        self.0
            .intern_blank(label, BlankScope(scope))
            .expect("interns")
    }

    fn triple(&mut self, s: TermId, p: TermId, o: TermId) -> TermId {
        self.0.intern_triple(s, p, o).expect("interns")
    }

    fn quad(&mut self, s: TermId, p: TermId, o: TermId) {
        self.0.push_quad(s, p, o, None).expect("pushes");
    }

    fn freeze(self) -> Arc<RdfDataset> {
        self.0.freeze().expect("freezes")
    }
}

fn labelled<D: DatasetView>(view: &D) -> Labelling<D::Id> {
    label_view(view, &[], Scope::Dataset, Limit::ENGINE, None).expect("labelled")
}

/// Each node's position by its source identifier.
fn positions(set: &LineSet, labelled: &Labelled) -> BTreeMap<Vec<u8>, u64> {
    (0..set.blank_nodes())
        .map(|node| {
            (
                set.source(node).to_vec(),
                labelled.placements[node].position,
            )
        })
        .collect()
}

/// The labelling of the view agrees with the definition's labelling of
/// `lines`, the lowered set of the text. Returns that labelling.
fn agree<Id: Copy>(labelling: &Labelling<Id>, lines: &[Line]) -> Labelled {
    let (set, expected, walk) =
        label_with_walk(lines, Limit::ENGINE).expect("the lowered set of the text is labelled");
    let (got, got_walk) =
        optimised::label_walked(labelling.lines(), Limit::ENGINE.get(), false, None)
            .expect("the lowered view is labelled");
    assert_eq!(labelling.lines().len(), set.len(), "one line per member");
    assert_eq!(got.lines, expected.lines, "the canonical line sequence");
    assert_eq!(got.digest, expected.digest);
    assert_eq!(labelling.labelled().digest, expected.digest);
    assert_eq!(got.record, expected.record, "the structural record");
    assert_eq!(got_walk.totals(), walk, "the totals of the walk");
    assert_eq!(
        positions(labelling.lines(), &got),
        positions(&set, &expected),
        "every source identifier and its position"
    );
    // The canonical encodings write every blank leaf `fixed`, so a set with no
    // two equal members weighs the bytes of its canonical line sequence.
    let bytes: u64 = expected.lines.iter().map(|line| line.len() as u64).sum();
    assert_eq!(labelling.weight(), bytes, "the weight");
    expected
}

/// The canonical document of a labelling.
fn document<Id: Copy>(labelling: &Labelling<Id>) -> String {
    let composites = labelling.composites().expect("spellings");
    let mut text = String::new();
    for line in &labelling.labelled().lines {
        write_line_with(line, &composites, &mut text).expect("spelled");
    }
    text
}

/// The worked example of the text: the nested term `x = <<b q c>>` of
/// `t = <<a p x>>` in the line `quad(a, p, t)`. The lowered set is the line
/// with `x` as its term node and one record for `x`; the preimage of `d(x)` is
/// 46 bytes; the size counts the row and the one nested candidate, and the
/// weight is `88 + 94`. The canonical document spells the term node as the
/// nested term it stands for and writes no record.
#[test]
fn a_nested_term_lowers_to_one_term_node_and_one_record() {
    let mut data = Data::new();
    let [a, p, b, q, c] = data.iris(["a", "p", "b", "q", "c"]);
    let x = data.triple(b, q, c);
    let tt = data.triple(a, p, x);
    data.quad(a, p, tt);
    let labelling = labelled(data.freeze().as_ref());

    let (dx, preimage) = d([&k_iri("b"), &k_iri("q"), &k_iri("c")]);
    assert_eq!(preimage, 46);
    let n = Term::blank(&node(&dx));
    let lines = [
        quad(t("a"), t("p"), Term::triple(t("a"), t("p"), n.clone())),
        record(vec![n, t("b"), t("q"), t("c")]),
    ];
    let expected = agree(&labelling, &lines);
    assert!(expected.record.guaranteed);
    assert_eq!((labelling.size(), labelling.weight()), (2, 182));
    assert_eq!(
        labelling.view_positions().count(),
        0,
        "a term node is no blank node of the view"
    );
    assert_eq!(
        document(&labelling),
        "<http://e/a> <http://e/p> <<( <http://e/a> <http://e/p> \
         <<( <http://e/b> <http://e/q> <http://e/c> )>> )>> .\n"
    );
}

/// One nested term held by two lines has one term node and one record, and a
/// value that stands at depth one in one line and as a component in another
/// keeps its depth-one place: `quad(a, p, x)` is not rewritten.
#[test]
fn a_nested_term_has_one_node_however_often_it_occurs_and_depth_one_stays() {
    let (dx, _) = d([&k_iri("b"), &k_iri("q"), &k_iri("c")]);
    let n = Term::blank(&node(&dx));

    let mut data = Data::new();
    let [a, p, b, q, c] = data.iris(["a", "p", "b", "q", "c"]);
    let x = data.triple(b, q, c);
    let t1 = data.triple(a, p, x);
    let t2 = data.triple(c, p, x);
    data.quad(a, p, t1);
    data.quad(c, p, t2);
    let shared = labelled(data.freeze().as_ref());
    agree(
        &shared,
        &[
            quad(t("a"), t("p"), Term::triple(t("a"), t("p"), n.clone())),
            quad(t("c"), t("p"), Term::triple(t("c"), t("p"), n.clone())),
            record(vec![n.clone(), t("b"), t("q"), t("c")]),
        ],
    );
    assert_eq!(shared.size(), 4);

    let mut data = Data::new();
    let [a, p, b, q, c] = data.iris(["a", "p", "b", "q", "c"]);
    let x = data.triple(b, q, c);
    let tt = data.triple(a, p, x);
    data.quad(a, p, x);
    data.quad(a, p, tt);
    let two_depths = labelled(data.freeze().as_ref());
    agree(
        &two_depths,
        &[
            quad(t("a"), t("p"), Term::triple(t("b"), t("q"), t("c"))),
            quad(t("a"), t("p"), Term::triple(t("a"), t("p"), n.clone())),
            record(vec![n, t("b"), t("q"), t("c")]),
        ],
    );
    assert_eq!(two_depths.size(), 3);
}

/// The three cases. With no nested term the lowering is the identity: the
/// lines are those the view always lowered to, every blank node keeps its
/// source identifier (scope ordinal and label), and nothing is added. The
/// inputs are the text's `lowering-none` (a triple term in a quad and under a
/// reifier with an annotation) and the claim shape of the counts bench: blank
/// reifiers of triple terms over named nodes with six to eight annotations.
#[test]
fn triple_terms_that_are_not_nested_lower_to_themselves() {
    let legacy = |label: &str| {
        let mut source = 0u32.to_be_bytes().to_vec();
        source.extend_from_slice(label.as_bytes());
        Term::Blank(source.into())
    };

    let mut data = Data::new();
    let [a, p, b, q, c] = data.iris(["a", "p", "b", "q", "c"]);
    let r = data.blank("r", 0);
    let tt = data.triple(b, q, c);
    data.quad(a, p, tt);
    data.0.push_reifier(r, tt).expect("reifier row");
    data.0.push_annotation(r, q, c).expect("annotation row");
    let none = labelled(data.freeze().as_ref());
    agree(
        &none,
        &[
            quad(t("a"), t("p"), Term::triple(t("b"), t("q"), t("c"))),
            Line::ReifierBinding {
                reifier: legacy("r"),
                triple: Term::triple(t("b"), t("q"), t("c")),
                graph: None,
            },
            Line::Annotation {
                count: 1,
                reifier: legacy("r"),
                predicate: t("q"),
                object: t("c"),
                graph: None,
            },
        ],
    );
    assert_eq!(none.size(), 3, "no nested candidate");

    let mut data = Data::new();
    let mut lines = Vec::new();
    let claim = data.iri("claim");
    let pp = data.iri("p");
    for m in 0..4 {
        let message = data.blank(&format!("m{m}"), 0);
        for c in 0..20usize {
            let name = format!("m{m}c{c:02}");
            let reifier = data.blank(&name, 0);
            let key = if c < 3 { 0 } else { c };
            let (s, o) = (format!("s{m}_{key}"), format!("o{m}_{key}"));
            let triple = {
                let (s, o) = (data.iri(&s), data.iri(&o));
                data.triple(s, pp, o)
            };
            data.quad(message, claim, reifier);
            data.0.push_reifier(reifier, triple).expect("reifier row");
            lines.push(quad(legacy(&format!("m{m}")), t("claim"), legacy(&name)));
            lines.push(Line::ReifierBinding {
                reifier: legacy(&name),
                triple: Term::triple(t(&s), t("p"), t(&o)),
                graph: None,
            });
            for k in 0..6 + key % 3 {
                let (q, v) = (format!("q{k}"), format!("v{m}_{key}_{k}"));
                let (qq, vv) = (data.iri(&q), data.iri(&v));
                data.0
                    .push_annotation(reifier, qq, vv)
                    .expect("annotation row");
                lines.push(Line::Annotation {
                    count: 1,
                    reifier: legacy(&name),
                    predicate: t(&q),
                    object: t(&v),
                    graph: None,
                });
            }
        }
    }
    let claims = labelled(data.freeze().as_ref());
    let expected = agree(&claims, &lines);
    assert!(expected.record.guaranteed);
    assert_eq!(claims.size(), lines.len() as u64, "no nested candidate");
    let (_, plain) = label(&lines).expect("labelled");
    assert_eq!(plain.digest, expected.digest);
}

/// The chain `t(1) = <<a a a>>`, `t(k + 1) = <<a a t(k)>>` of `n` terms with a
/// row `quad(a, a, t(k))` for each, and its lowered set: `n - 1` term nodes,
/// each record naming the node of the term before.
fn chain(n: usize) -> (Arc<RdfDataset>, Vec<Line>) {
    let mut data = Data::new();
    let a = data.iri("a");
    let mut lines = Vec::with_capacity(2 * n);
    let mut term = data.triple(a, a, a);
    let mut previous: Option<Term> = None;
    let mut digest: Option<[u8; 32]> = None;
    for k in 1..=n {
        if k > 1 {
            term = data.triple(a, a, term);
        }
        data.quad(a, a, term);
        let object = previous.clone().unwrap_or_else(|| t("a"));
        lines.push(quad(
            t("a"),
            t("a"),
            Term::triple(t("a"), t("a"), object.clone()),
        ));
        if k < n {
            let k_object = digest.map_or_else(|| k_iri("a"), |d| node(&d));
            let (dk, _) = d([&k_iri("a"), &k_iri("a"), &k_object]);
            let n_k = Term::blank(&node(&dk));
            lines.push(record(vec![n_k.clone(), t("a"), t("a"), object]));
            previous = Some(n_k);
            digest = Some(dk);
        }
    }
    (data.freeze(), lines)
}

/// The text's `lowering-deep`: a chain of 1,000 terms lowers to 999 term
/// nodes and 999 records, its size is the 1,000 rows and the 999 nested
/// candidates, and its weight is `172 n - 64`, linear in `n`.
#[test]
fn a_chain_of_nested_terms_lowers_to_a_path_of_records() {
    let (view, lines) = chain(1000);
    let labelling = labelled(view.as_ref());
    let expected = agree(&labelling, &lines);
    assert!(expected.record.guaranteed);
    assert_eq!(labelling.lines().len(), 1999);
    assert_eq!((labelling.size(), labelling.weight()), (1999, 171_936));
}

/// A refusal is a claim: the size is refused one line below the size of the
/// chain and admitted at it; the weight of a read is refused one byte below the
/// weight and admitted at it, and the total of a read holds every admitted
/// weight, so the second of two equal inputs is refused at the weight of one.
/// After a refusal by weight every later input of the read whose size is
/// admitted is refused, whatever its weight. A size refusal adds nothing.
#[test]
fn the_size_and_the_weight_of_a_read_are_refused_past_their_capacities() {
    let (view, _) = chain(1000);
    let view = view.as_ref();
    let small = {
        let mut data = Data::new();
        let a = data.iri("a");
        data.quad(a, a, a);
        data.freeze()
    };
    let run = |capacity: &mut LabellingCapacity, view: &RdfDataset| {
        label_view_with_capacity(view, &[], Scope::Dataset, Limit::ENGINE, None, capacity)
            .map(|labelling| labelling.weight())
    };

    let mut read = LabellingCapacity::new(1999, u64::MAX);
    assert_eq!(run(&mut read, view), Ok(171_936));
    let mut read = LabellingCapacity::new(1998, u64::MAX);
    assert_eq!(
        run(&mut read, view),
        Err(ViewError::Lowering(LoweringRefusal::Size {
            size: 1999,
            capacity: 1998
        }))
    );
    assert_eq!(read.total(), 0);

    let mut read = LabellingCapacity::new(u64::MAX, 171_936);
    assert_eq!(run(&mut read, view), Ok(171_936));
    assert_eq!(
        run(&mut read, view),
        Err(ViewError::Lowering(LoweringRefusal::Weight {
            weight: 171_936,
            total: 171_936,
            capacity: 171_936
        }))
    );
    assert_eq!(read.total(), 171_936);

    let mut read = LabellingCapacity::new(u64::MAX, 171_935);
    assert!(run(&mut read, small.as_ref()).is_ok());
    let total = read.total();
    assert!(matches!(
        run(&mut read, view),
        Err(ViewError::Lowering(LoweringRefusal::Weight { .. }))
    ));
    assert!(matches!(
        run(&mut read, small.as_ref()),
        Err(ViewError::Lowering(LoweringRefusal::Weight { .. }))
    ));
    assert_eq!(read.total(), total);
}

/// Every digest is the same, so any two different nested terms collide.
const fn collide(_: &[u8]) -> [u8; 32] {
    [7; 32]
}

/// A refusal is a claim: two different nested terms with one digest are
/// refused with `term_node`, and a neighbouring input with one nested term is
/// labelled under the same digest function. The refusal comes after the size
/// and the weight, and the weight it admitted stays in the total of the read.
/// No input can be built whose two nested terms share a BLAKE3 digest, so the
/// digest function is the lowering's argument and the test hands it one that
/// is constant.
#[test]
fn two_nested_terms_with_one_digest_are_refused_after_the_size_and_the_weight() {
    let mut data = Data::new();
    let [a, p, b, q, c] = data.iris(["a", "p", "b", "q", "c"]);
    for inner in [data.triple(b, q, c), data.triple(c, q, b)] {
        let outer = data.triple(a, p, inner);
        data.quad(a, p, outer);
    }
    let two = data.freeze();
    let mut data = Data::new();
    let [a, p, b, q, c] = data.iris(["a", "p", "b", "q", "c"]);
    let inner = data.triple(b, q, c);
    let outer = data.triple(a, p, inner);
    data.quad(a, p, outer);
    let one = data.freeze();

    let lower = |view: &RdfDataset, capacity: Option<&mut LabellingCapacity>| {
        let run = Run {
            capacity,
            hash: collide,
            ..Run::plain(None)
        };
        lowered(view, Scope::Dataset, &[], run).map(|lowered| (lowered.size, lowered.weight))
    };
    let Ok((size, weight)) = lowered(two.as_ref(), Scope::Dataset, &[], Run::plain(None))
        .map(|lowered| (lowered.size, lowered.weight))
    else {
        panic!("the two nested terms have two BLAKE3 digests");
    };
    assert_eq!(
        lower(two.as_ref(), None).err(),
        Some(ViewError::Lowering(LoweringRefusal::TermNode {
            digest: [7; 32]
        }))
    );
    assert!(lower(one.as_ref(), None).is_ok());

    let mut read = LabellingCapacity::new(u64::MAX, u64::MAX);
    assert!(matches!(
        lower(two.as_ref(), Some(&mut read)),
        Err(ViewError::Lowering(LoweringRefusal::TermNode { .. }))
    ));
    assert_eq!(read.total(), weight, "an admitted weight stays added");
    let mut read = LabellingCapacity::new(size - 1, u64::MAX);
    assert!(matches!(
        lower(two.as_ref(), Some(&mut read)),
        Err(ViewError::Lowering(LoweringRefusal::Size { .. }))
    ));
    let mut read = LabellingCapacity::new(u64::MAX, weight - 1);
    assert!(matches!(
        lower(two.as_ref(), Some(&mut read)),
        Err(ViewError::Lowering(LoweringRefusal::Weight { .. }))
    ));
}

/// A blank node inside a nested term: its source `K` is `E([0, h(b)])`, the
/// digest of its source identifier `E([ordinal, label])`, and in one scope the
/// preimage of `d(x)` is 69 bytes. Two scopes holding the same shape give two
/// nested terms and two term nodes. Every term node precedes every blank node
/// of the view in source order.
#[test]
fn a_blank_node_inside_a_nested_term_is_sourced_by_its_digest() {
    let shape = |data: &mut Data, scope: u32| {
        let [a, p, q, c] = data.iris(["a", "p", "q", "c"]);
        let u = data.blank("u", scope);
        let x = data.triple(u, q, c);
        let tt = data.triple(a, p, x);
        data.quad(a, p, tt);
    };
    let lines_of = |scope: u64| {
        let (dx, preimage) = d([&k_blank(scope, b"u"), &k_iri("q"), &k_iri("c")]);
        assert_eq!(preimage, 69);
        let n = Term::blank(&node(&dx));
        [
            quad(t("a"), t("p"), Term::triple(t("a"), t("p"), n.clone())),
            record(vec![
                n,
                Term::blank(&blank_source(scope, b"u")),
                t("q"),
                t("c"),
            ]),
        ]
    };
    assert_eq!(blank_source(0, b"u").len(), 4);
    assert_eq!(k_blank(0, b"u").len(), 36);

    let mut data = Data::new();
    shape(&mut data, 0);
    let one = labelled(data.freeze().as_ref());
    agree(&one, &lines_of(0));
    assert_eq!(one.size(), 2);

    let mut data = Data::new();
    shape(&mut data, 0);
    shape(&mut data, 1);
    let two = labelled(data.freeze().as_ref());
    let lines: Vec<Line> = lines_of(0).into_iter().chain(lines_of(1)).collect();
    agree(&two, &lines);
    assert_eq!(two.size(), 4);
    let sources: Vec<&[u8]> = (0..two.lines().blank_nodes())
        .map(|node| two.lines().source(node))
        .collect();
    let first_blank = sources.iter().filter(|s| s[0] == 0x82).min();
    let last_node = sources.iter().filter(|s| s[0] == 0x81).max();
    assert!(last_node < first_blank);
    assert_eq!(two.view_positions().count(), 2);
}

/// The twins of the text, `u` and `w`, each inside its own nested term. After
/// the lowering each stands in the record of its own term node, so the set has
/// two top components, `{u, n_u}` and `{w, n_w}`, and is `guaranteed`; their
/// order is decided by the source identifiers of the term nodes, which precede
/// those of the blank nodes. With the lines `quad(u, r, w)` and `quad(w, r, u)`
/// the set has one top component, whose automorphism moves the term nodes
/// with `u` and `w`: `u` and `w` alone are not twins there, and the set is not
/// `guaranteed`.
#[test]
fn twins_inside_nested_terms_are_ordered_by_their_term_nodes() {
    let build = |linked: bool| {
        let mut data = Data::new();
        let [a, p, q, c, r] = data.iris(["a", "p", "q", "c", "r"]);
        let (u, w) = (data.blank("u", 0), data.blank("w", 0));
        for blank in [u, w] {
            let x = data.triple(blank, q, c);
            let tt = data.triple(a, p, x);
            data.quad(a, p, tt);
        }
        if linked {
            data.quad(u, r, w);
            data.quad(w, r, u);
        }
        data.freeze()
    };
    let (du, _) = d([&k_blank(0, b"u"), &k_iri("q"), &k_iri("c")]);
    let (dw, _) = d([&k_blank(0, b"w"), &k_iri("q"), &k_iri("c")]);
    let lowered_lines = |linked: bool| {
        let mut lines = Vec::new();
        for (label, digest) in [(b"u", du), (b"w", dw)] {
            let n = Term::blank(&node(&digest));
            let blank = Term::blank(&blank_source(0, label));
            lines.push(quad(
                t("a"),
                t("p"),
                Term::triple(t("a"), t("p"), n.clone()),
            ));
            lines.push(record(vec![n, blank, t("q"), t("c")]));
        }
        if linked {
            let (u, w) = (
                Term::blank(&blank_source(0, b"u")),
                Term::blank(&blank_source(0, b"w")),
            );
            lines.push(quad(u.clone(), t("r"), w.clone()));
            lines.push(quad(w, t("r"), u));
        }
        lines
    };

    let twins = labelled(build(false).as_ref());
    let expected = agree(&twins, &lowered_lines(false));
    assert!(expected.record.guaranteed);
    assert_eq!(expected.record.top_components, 2);
    let at = positions(twins.lines(), twins.labelled());
    let placed = |at: &BTreeMap<Vec<u8>, u64>| {
        [
            at[&blank_source(0, b"u")],
            at[&blank_source(0, b"w")],
            at[&node(&du)],
            at[&node(&dw)],
        ]
    };
    // `lowering-twins`: u 2, w 0, n_u 3, n_w 1.
    assert_eq!(placed(&at), [2, 0, 3, 1]);

    let linked = labelled(build(true).as_ref());
    let expected = agree(&linked, &lowered_lines(true));
    assert_eq!(expected.record.top_components, 1);
    assert!(!expected.record.guaranteed);
    assert_eq!(linked.size(), 6);
    // `lowering-twins-one-component`: u 0, w 1, n_u 2, n_w 3, and the walk
    // charges 282 units against an allowance of 552.
    assert_eq!(
        placed(&positions(linked.lines(), linked.labelled())),
        [0, 1, 2, 3]
    );
    let (_, _, walk) = label_with_walk(&lowered_lines(true), Limit::ENGINE).expect("labelled");
    assert_eq!((walk.total(), expected.record.allowance), (282, 552));
}

/// The case the two forms of a view blank node's source exist for: twins
/// outside any nested term whose labels differ in length. The four-byte form
/// orders `b10` before `b9` bytewise; the text form `E([ordinal, label])`
/// orders the shorter label first. With no nested term in the input the twins
/// keep the order they always had; with one, they take the text's order.
#[test]
fn view_blank_twins_take_the_order_of_the_source_form_of_their_input() {
    let build = |nested: bool| {
        let mut data = Data::new();
        let [a, p, q, c] = data.iris(["a", "p", "q", "c"]);
        for label in ["b9", "b10"] {
            let blank = data.blank(label, 0);
            data.quad(blank, p, c);
        }
        if nested {
            let x = data.triple(a, q, c);
            let tt = data.triple(a, p, x);
            data.quad(a, p, tt);
        }
        data.freeze()
    };
    let legacy = |label: &[u8]| {
        let mut source = 0u32.to_be_bytes().to_vec();
        source.extend_from_slice(label);
        source
    };

    let plain = labelled(build(false).as_ref());
    let lines: Vec<Line> = [b"b9".as_slice(), b"b10"]
        .map(|label| quad(Term::blank(&legacy(label)), t("p"), t("c")))
        .into();
    agree(&plain, &lines);
    let at = positions(plain.lines(), plain.labelled());
    assert_eq!((at[&legacy(b"b10")], at[&legacy(b"b9")]), (0, 1));

    let with_nested = labelled(build(true).as_ref());
    let (dx, _) = d([&k_iri("a"), &k_iri("q"), &k_iri("c")]);
    let n = Term::blank(&node(&dx));
    let mut lines: Vec<Line> = [b"b9".as_slice(), b"b10"]
        .map(|label| quad(Term::blank(&blank_source(0, label)), t("p"), t("c")))
        .into();
    lines.push(quad(
        t("a"),
        t("p"),
        Term::triple(t("a"), t("p"), n.clone()),
    ));
    lines.push(record(vec![n, t("a"), t("q"), t("c")]));
    agree(&with_nested, &lines);
    let at = positions(with_nested.lines(), with_nested.labelled());
    assert!(at[&blank_source(0, b"b9")] < at[&blank_source(0, b"b10")]);
}

/// The source form is decided by the input the lowering reads. A nested term
/// only in a graph the scope leaves out leaves the form of the selected
/// statements as it was: four big-endian bytes of the scope ordinal and the
/// label, and no term node.
#[test]
fn a_nested_term_in_a_graph_the_scope_leaves_out_keeps_the_four_byte_form() {
    let mut data = Data::new();
    let [a, p, q, c, g1, g2] = data.iris(["a", "p", "q", "c", "g1", "g2"]);
    let x = data.blank("x", 0);
    data.0.push_quad(x, p, c, Some(g1)).expect("pushes");
    let inner = data.triple(a, q, c);
    let outer = data.triple(a, p, inner);
    data.0.push_quad(a, p, outer, Some(g2)).expect("pushes");
    let view = data.freeze();

    let kept = label_view(
        view.as_ref(),
        &[],
        Scope::Graph(&iri("g1")),
        Limit::ENGINE,
        None,
    )
    .expect("labelled");
    assert_eq!(kept.lines().blank_nodes(), 1);
    let mut legacy = 0u32.to_be_bytes().to_vec();
    legacy.push(b'x');
    assert_eq!(kept.lines().source(0), legacy.as_slice());
    assert_eq!(kept.size(), 1);

    // The neighbouring input that does hold the nested term: the other graph.
    let taken = label_view(
        view.as_ref(),
        &[],
        Scope::Graph(&iri("g2")),
        Limit::ENGINE,
        None,
    )
    .expect("labelled");
    assert_eq!(taken.lines().blank_nodes(), 1);
    assert!(is_term_node(taken.lines().source(0)));
}

/// A nested term only in a caller's record switches every blank node of the
/// input to the text form, including a blank node only the record names, even
/// though the view holds no nested term.
#[test]
fn a_nested_term_only_in_a_record_switches_every_node_to_the_text_form() {
    let mut data = Data::new();
    let [p, c] = data.iris(["p", "c"]);
    let x = data.blank("x", 0);
    data.quad(x, p, c);
    let view = data.freeze();

    let blank = |label: &str| TermValue::Blank {
        label: label.to_owned(),
        scope: BlankScope(0),
    };
    let leaf = |name: &str| TermValue::iri(iri(name));
    let nested = TermValue::Triple {
        s: crate::TermBox::new(leaf("a")),
        p: crate::TermBox::new(leaf("p")),
        o: crate::TermBox::new(TermValue::Triple {
            s: crate::TermBox::new(blank("y")),
            p: crate::TermBox::new(leaf("q")),
            o: crate::TermBox::new(leaf("c")),
        }),
    };
    let terms = [nested];
    let records = [Record::new(&[0x80], &terms)];
    let with =
        label_view(view.as_ref(), &records, Scope::Dataset, Limit::ENGINE, None).expect("labelled");
    let sources: Vec<&[u8]> = (0..with.lines().blank_nodes())
        .map(|node| with.lines().source(node))
        .collect();
    assert!(sources.contains(&blank_source(0, b"x").as_slice()));
    assert!(sources.contains(&blank_source(0, b"y").as_slice()));
    assert_eq!(sources.iter().filter(|s| is_term_node(s)).count(), 1);
    assert_eq!(sources.len(), 3);

    // The neighbouring input without the record keeps the four-byte form.
    let without = labelled(view.as_ref());
    let mut legacy = 0u32.to_be_bytes().to_vec();
    legacy.push(b'x');
    assert_eq!(without.lines().source(0), legacy.as_slice());
}

/// A term node's source identifier: an array of one, first byte `0x81`.
fn is_term_node(source: &[u8]) -> bool {
    source.first() == Some(&0x81)
}

/// A view whose one literal carries its language as written, for the text's
/// literal sources: the dataset folds the case of a language tag, the format
/// does not.
struct Cased {
    inner: Arc<RdfDataset>,
    literal: TermId,
    language: String,
}

impl DatasetView for Cased {
    type Id = TermId;
    type ReadError = CallerRefusal;
    type TermGuard<'a> = TermRef<'a>;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads()
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.reifier_quads()
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.annotation_quads()
    }
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        DatasetView::named_graphs(self.inner.as_ref())
    }
    fn resolve(&self, id: TermId) -> Result<TermRef<'_>, CallerRefusal> {
        Ok(match self.inner.as_ref().resolve(id) {
            TermRef::Literal {
                lexical,
                datatype,
                direction,
                ..
            } if id == self.literal => TermRef::Literal {
                lexical,
                datatype,
                language: Some(&self.language),
                direction,
            },
            term => term,
        })
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, CallerRefusal> {
        Ok(self.inner.as_ref().term_id_by_value(value))
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.inner.capabilities()
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch) {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads_for_pattern(s, p, o, g)
    }
    fn term_count(&self) -> u64 {
        self.inner.term_count()
    }
}

/// The literal sources of the text: a language literal with its language as
/// written and the direction `ltr` (form 3), and a typed literal (form 1),
/// each the object of a nested term; with the record of the typed value the
/// format adds beside them, a caller's record here. The preimages are 44 and
/// 93 bytes.
#[test]
fn a_literal_inside_a_nested_term_is_sourced_by_its_texts() {
    let mut data = Data::new();
    let [a, p, b, q] = data.iris(["a", "p", "b", "q"]);
    let language = data
        .0
        .intern_literal(RdfLiteral {
            lexical_form: "x".to_owned(),
            datatype: None,
            language: Some("en-us".to_owned()),
            direction: Some(RdfTextDirection::Ltr),
        })
        .expect("interns");
    let typed = data
        .0
        .intern_literal(RdfLiteral {
            lexical_form: "7".to_owned(),
            datatype: Some(XSD_NON_NEGATIVE_INTEGER.to_owned()),
            language: None,
            direction: None,
        })
        .expect("interns");
    for literal in [language, typed] {
        let x = data.triple(b, q, literal);
        let tt = data.triple(a, p, x);
        data.quad(a, p, tt);
    }
    let view = Cased {
        inner: data.freeze(),
        literal: language,
        language: "EN-us".to_owned(),
    };
    let typed_value = TermValue::Literal {
        lexical_form: "7".to_owned(),
        datatype: XSD_NON_NEGATIVE_INTEGER.to_owned(),
        language: None,
        direction: None,
    };
    // The payload of the record of a pair of VALUES of kind `uint`:
    // `E([4, [2, [1]]])`.
    let mut values = Vec::new();
    push_head(&mut values, ARRAY, 2);
    push_head(&mut values, UNSIGNED, 4);
    push_head(&mut values, ARRAY, 2);
    push_head(&mut values, UNSIGNED, 2);
    push_head(&mut values, ARRAY, 1);
    push_head(&mut values, UNSIGNED, 1);
    let terms = [typed_value];
    let records = [Record::new(&values, &terms)];
    let labelling =
        label_view(&view, &records, Scope::Dataset, Limit::ENGINE, None).expect("labelled");

    let (d_language, language_preimage) =
        d([&k_iri("b"), &k_iri("q"), &k_literal("x", 3, "EN-us")]);
    let (d_typed, typed_preimage) = d([
        &k_iri("b"),
        &k_iri("q"),
        &k_literal("7", 1, XSD_NON_NEGATIVE_INTEGER),
    ]);
    assert_eq!((language_preimage, typed_preimage), (44, 93));
    let language_term = Term::Literal {
        lexical: (*b"x").into(),
        datatype: DIR_LANG_STRING.as_bytes().into(),
        language: Some((*b"EN-us").into()),
        direction: Some(Direction::Ltr),
    };
    let typed_term = Term::typed(b"7", XSD_NON_NEGATIVE_INTEGER.as_bytes());
    let mut lines = Vec::new();
    for (digest, literal) in [(d_language, language_term), (d_typed, typed_term.clone())] {
        let n = Term::blank(&node(&digest));
        lines.push(quad(
            t("a"),
            t("p"),
            Term::triple(t("a"), t("p"), n.clone()),
        ));
        lines.push(record(vec![n, t("b"), t("q"), literal]));
    }
    lines.push(Line::Record {
        payload: values.clone().into(),
        terms: vec![typed_term],
    });
    let expected = agree(&labelling, &lines);
    assert!(expected.record.guaranteed);
    assert_eq!(labelling.size(), 5);
}

/// The canonical file of the two-scope fold of a blank node inside a nested
/// term stores each blank node of the fold under the label `E(position)`. A
/// second run over that file may give positions that differ from the stored
/// labels; the map `tau`, from each node to the node whose stored label is
/// `E(its position in the second run)`, is then an automorphism of the fold,
/// the exchange of the two blank nodes, and the content digest is the same.
#[test]
fn stored_labels_of_twins_in_nested_terms_name_a_labelling_up_to_automorphism() {
    let fold = |labels: [&str; 2]| {
        let mut data = Data::new();
        let [a, p, q, c] = data.iris(["a", "p", "q", "c"]);
        for (scope, label) in [
            (0, labels[0]),
            (u32::from(labels[0] == labels[1]), labels[1]),
        ] {
            let u = data.blank(label, scope);
            let x = data.triple(u, q, c);
            let tt = data.triple(a, p, x);
            data.quad(a, p, tt);
        }
        data.freeze()
    };
    let position_of = |labelling: &Labelling<TermId>, source: &[u8]| {
        positions(labelling.lines(), labelling.labelled())[source]
    };
    // The first run: `u` of scope 0 and of scope 1.
    let first = labelled(fold(["u", "u"]).as_ref());
    let stored = [
        position_of(&first, &blank_source(0, b"u")),
        position_of(&first, &blank_source(1, b"u")),
    ];
    // The canonical file: one scope, each node named by `E(position)`.
    let label = |position: u64| {
        let mut out = Vec::new();
        push_head(&mut out, UNSIGNED, position);
        String::from_utf8(out).expect("a small position is one ASCII byte")
    };
    let names = stored.map(label);
    let second = labelled(fold([&names[0], &names[1]]).as_ref());
    assert_eq!(second.labelled().digest, first.labelled().digest);
    let again = names
        .clone()
        .map(|name| position_of(&second, &blank_source(0, name.as_bytes())));
    // `tau` sends the node stored as `names[i]` to the node stored as
    // `E(again[i])`.
    let tau: Vec<usize> = again
        .iter()
        .map(|&position| {
            names
                .iter()
                .position(|name| *name == label(position))
                .expect("tau is defined")
        })
        .collect();
    assert_eq!(tau, [1, 0], "tau is the exchange, not the identity");
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::{HashMap, HashSet};

use ciborium::value::Value;

use crate::model::{Graph, Quad, Suppression, Term, TermKind, Triple3};
use crate::reader::{as_idx, as_text, text_or};
use crate::reader_index::DigestIndex;
use crate::wire::map_get;

// --------------------------------------------------------------------------- //
// Multi-segment union (§3.1, §7.5): term-ids are segment-scoped compression
// artifacts; the union re-interns BY TERM VALUE. Blank nodes carry a segment
// discriminator (labels are segment-local and never merge); quoted-triple
// terms intern through their bound SPO identity. Because the union is
// value-interned, "apply suppression value-wise" (§11) reduces to applying it
// by result-id.
//
// TERMINATION (§7.3, normative) — a quoted triple's VALUE is its `(s, p, o)`
// binding, so interning one walks its components, and `map_term`'s walk has NO
// bound of its own. It terminates
// because a term can never reach itself:
//
//   * `tt` and `dt` components must name an ALREADY-INTRODUCED term, and `rf`
//     must too (or the term itself, handled without recursing) — see the
//     sanitizing in `reader::Folder::h_terms`. Ids therefore strictly descend.
//   * the `reifies` statement layer is the one field that may name any term,
//     so `reader::reifier_binding_is_recursive` REFUSES, as a `DamagedFrame`,
//     any binding that would let a triple term reach itself.
//
// That is the fold-time-refusal strategy §7.3 permits; the alternative it
// permits — a per-term sentinel that states no triple — belongs to engines
// whose readers accept the row. Because the refusal is what this engine does,
// this walk needs no guard of its own, and adding one would put two hash
// lookups per term on the fold's hot path to handle input the reader cannot
// produce. The coupling is not local, so it is pinned:
// `tests/union_self_reaching_term.rs` builds each attempt at the shape through
// the real writer and asserts the fold refuses it. Relax the refusal and those
// tests fail — which is the signal that this walk then needs the sentinel.
// --------------------------------------------------------------------------- //

#[derive(Clone, PartialEq, Eq, Hash)]
enum InternKey {
    Iri(Option<String>),
    Lit(Option<String>, String, Option<String>, Option<String>),
    Bnode(usize, Option<String>, Option<usize>),
    Qt(Option<Triple3>),
}

#[derive(Default)]
struct Unioner {
    out: Graph,
    blob_index: DigestIndex,
    blob_meta_index: DigestIndex,
    intern: HashMap<InternKey, usize>,
}

/// Where a term being mapped by [`Unioner::map_term`] stands: each stage waits for the
/// mappings it asked for, in the order the stages are listed.
enum MapStage {
    /// Nothing is known yet.
    Start,
    /// The quoted triple's key components are being mapped.
    KeyComponents,
    /// The key is known; the union is consulted for it.
    Lookup,
    /// The datatype, if any, is being mapped.
    Datatype,
    /// The reifier, if any and not the term itself, is being mapped.
    Reifier,
    /// The stated `tt` components, if any, are being mapped.
    Components,
}

/// A term being mapped by [`Unioner::map_term`].
struct MapFrame {
    tid: usize,
    stage: MapStage,
    /// Terms still to map before the stage advances, the next one last.
    calls: Vec<usize>,
    /// The union ids the calls of the current stage produced, in call order.
    results: Vec<usize>,
    key: Option<InternKey>,
    datatype: Option<usize>,
    reifier: Option<usize>,
}

impl MapFrame {
    const fn new(tid: usize) -> Self {
        Self {
            tid,
            stage: MapStage::Start,
            calls: Vec::new(),
            results: Vec::new(),
            key: None,
            datatype: None,
            reifier: None,
        }
    }

    /// The three results of a stage that mapped a triple's `(s, p, o)`.
    fn triple_results(&mut self) -> Triple3 {
        <[usize; 3]>::try_from(std::mem::take(&mut self.results))
            .expect("a triple's three components are mapped")
            .into()
    }
}

impl Unioner {
    /// The intern key of a segment term that is not a quoted triple with bound
    /// components — those key on their components' union ids instead.
    fn leaf_key(seg: &Graph, seg_idx: usize, tid: usize) -> InternKey {
        let t = &seg.terms[tid];
        match t.kind {
            TermKind::Iri => InternKey::Iri(t.value.clone()),
            TermKind::Literal => InternKey::Lit(
                t.value.clone(),
                seg.datatype_iri(t),
                t.lang.clone(),
                t.direction.clone(),
            ),
            // Non-empty labels are segment-local; absent/empty labels are fresh
            // anonymous nodes keyed by their source term entry (§7.1).
            TermKind::Bnode => {
                let label = t.value.as_ref().filter(|v| !v.is_empty()).cloned();
                let anon_tid = label.is_none().then_some(tid);
                InternKey::Bnode(seg_idx, label, anon_tid)
            }
            TermKind::Triple => InternKey::Qt(None),
        }
    }

    /// Map segment term `tid` into the union, returning its union id.
    ///
    /// A term's key is found first. A quoted triple keys on its interned SPO
    /// binding, taken from the term's own `tt` when it has one and otherwise through
    /// the legacy reifier indirection, so its subject, predicate and object are
    /// mapped first, in that order. A self-bound triple term uses `rf == tid` and its
    /// reifier is not mapped. A term whose key the union already holds is that union
    /// term; any other has its datatype, then its reifier, then its `tt` components
    /// mapped, and is pushed once all of them are, so its union id is the length of
    /// the union's term table at that moment. Every nested mapping runs to completion
    /// before the next begins, over a work list of [`MapFrame`]s.
    ///
    /// # Termination
    ///
    /// The walk follows a term's triple components, its datatype and its reifier
    /// with no depth bound and no visited set — see the module-level rationale above
    /// for why: `union_segments` is `pub(crate)` in a private module, so
    /// `reader::read_with_options` is its only caller, and every `Graph` reaching it
    /// has already passed `reader::read`'s per-segment refusal of a self-reaching
    /// triple term (`reifier_binding_is_recursive`, `DamagedFrame`, covering the
    /// implicit self-bound spelling of §7.1 too). A guard was considered and rejected
    /// here: it would put two hash lookups per term on the fold's hot path to defend
    /// against a shape the reader cannot produce — a closed decision. Any path into
    /// this union that does not run through `reader::read` must establish its own
    /// acyclicity before calling in; this walk will not.
    fn map_term(&mut self, seg: &Graph, seg_idx: usize, tid: usize) -> usize {
        let mut frames = vec![MapFrame::new(tid)];
        loop {
            let frame = frames.last_mut().expect("a term is being mapped");
            if let Some(call) = frame.calls.pop() {
                frames.push(MapFrame::new(call));
                continue;
            }
            // `seg` is a parameter, not a field of `self`, so the term can be
            // borrowed across the `&mut self` mappings; only the two `Option<String>`
            // fields that move into the pushed `Term` are cloned below.
            let t = &seg.terms[frame.tid];
            let self_bound = t.kind == TermKind::Triple && t.reifier == Some(frame.tid);
            let mapped = match frame.stage {
                MapStage::Start => {
                    match (t.kind, seg.term_triple(t)) {
                        (TermKind::Triple, Some((s, p, o))) => {
                            frame.calls = vec![o, p, s];
                            frame.stage = MapStage::KeyComponents;
                        }
                        _ => {
                            frame.key = Some(Self::leaf_key(seg, seg_idx, frame.tid));
                            frame.stage = MapStage::Lookup;
                        }
                    }
                    continue;
                }
                MapStage::KeyComponents => {
                    frame.key = Some(InternKey::Qt(Some(frame.triple_results())));
                    frame.stage = MapStage::Lookup;
                    continue;
                }
                MapStage::Lookup => {
                    let key = frame.key.as_ref().expect("the key is found first");
                    if let Some(&got) = self.intern.get(key) {
                        got
                    } else {
                        frame.calls.extend(t.datatype);
                        frame.stage = MapStage::Datatype;
                        continue;
                    }
                }
                MapStage::Datatype => {
                    frame.datatype = frame.results.pop();
                    if !self_bound {
                        frame.calls.extend(t.reifier);
                    }
                    frame.stage = MapStage::Reifier;
                    continue;
                }
                MapStage::Reifier => {
                    frame.reifier = frame.results.pop();
                    if let Some((s, p, o)) = t.triple {
                        frame.calls = vec![o, p, s];
                    }
                    frame.stage = MapStage::Components;
                    continue;
                }
                MapStage::Components => {
                    let triple = t.triple.map(|_| frame.triple_results());
                    // Mapping the datatype, reifier and components can push terms, so
                    // this term's output id is taken only now that they are done.
                    let new_id = self.out.terms.len();
                    let reifier = if self_bound {
                        Some(new_id)
                    } else {
                        frame.reifier
                    };
                    // Blank nodes are relabelled with a segment prefix (§7.1 permits
                    // isomorphism-preserving relabeling): within a segment,
                    // byte-identical entries already intern to one union term (§7.8);
                    // ACROSS segments the same label names DIFFERENT nodes, and
                    // emitting the raw label from the union would merge them.
                    // Label-less nodes (absent or empty "v") are distinct TERMS under
                    // the intern key, so their serialized labels must stay distinct
                    // too — the union id disambiguates them. Computed after dt/rf
                    // mapping so out.terms.len() IS this term's id.
                    let value = if t.kind == TermKind::Bnode {
                        Some(match t.value.as_deref() {
                            Some(label) if !label.is_empty() => format!("s{seg_idx}.{label}"),
                            _ => format!("s{seg_idx}._anon{new_id}"),
                        })
                    } else {
                        t.value.clone()
                    };
                    self.out.terms.push(Term {
                        kind: t.kind,
                        value,
                        datatype: frame.datatype,
                        lang: t.lang.clone(),
                        direction: t.direction.clone(),
                        reifier,
                        triple,
                    });
                    let key = frame.key.take().expect("the key is found first");
                    self.intern.insert(key, new_id);
                    new_id
                }
            };
            frames.pop();
            match frames.last_mut() {
                Some(parent) => parent.results.push(mapped),
                None => return mapped,
            }
        }
    }

    /// Re-intern a suppression's id-addressed targets (§11).
    ///
    /// Digest-addressed targets (`frame`, `blob`) pass through unchanged
    /// (content-ids are file-global). Id-addressed targets resolve in their
    /// OWN segment and re-intern into the union — exactly the value-wise
    /// application the spec requires, because the union is value-interned.
    fn remap_suppression(&mut self, sup: &Suppression, seg: &Graph, seg_idx: usize) -> Suppression {
        let n = seg.terms.len();
        let mut new_targets = Vec::with_capacity(sup.targets.len());
        for target in &sup.targets {
            let Value::Map(entries) = target else {
                new_targets.push(target.clone());
                continue;
            };
            let kind = text_or(map_get(entries, "kind"), "");
            if kind == "frame" || kind == "blob" {
                new_targets.push(target.clone());
                continue;
            }
            let mapped: Vec<(Value, Value)> = entries
                .iter()
                .map(|(k, v)| {
                    let key = as_text(k);
                    if (kind == "term" || kind == "reifier") && key == Some("id") {
                        if let Some(tid) = as_idx(v)
                            && tid < n
                        {
                            let new = self.map_term(seg, seg_idx, tid);
                            return (k.clone(), Value::from(new as u64));
                        }
                    } else if kind == "quad"
                        && key == Some("q")
                        && let Value::Array(ids) = v
                    {
                        let remapped: Vec<Value> = ids
                            .iter()
                            .map(|x| match as_idx(x) {
                                Some(tid) if tid < n => {
                                    Value::from(self.map_term(seg, seg_idx, tid) as u64)
                                }
                                _ => x.clone(),
                            })
                            .collect();
                        return (k.clone(), Value::Array(remapped));
                    }
                    (k.clone(), v.clone())
                })
                .collect();
            new_targets.push(Value::Map(mapped));
        }
        Suppression {
            targets: new_targets,
            reason: sup.reason.clone(),
            // "by" is a segment-scoped term-id (the suppressing agent) —
            // remap it into the union's id space like every other id ref.
            by: sup
                .by
                .and_then(|b| (b < n).then(|| self.map_term(seg, seg_idx, b))),
        }
    }
}

/// Union per-segment folds into one value-interned [`Graph`].
pub(crate) fn union_segments(segments: &[Graph]) -> Graph {
    let mut u = Unioner::default();
    let mut seen: HashSet<Quad> = HashSet::new();
    for (seg_idx, seg) in segments.iter().enumerate() {
        for &(s, p, o, gq) in &seg.quads {
            let q: Quad = (
                u.map_term(seg, seg_idx, s),
                u.map_term(seg, seg_idx, p),
                u.map_term(seg, seg_idx, o),
                gq.map(|x| u.map_term(seg, seg_idx, x)),
            );
            if seen.insert(q) {
                // the folded graph is a set (§7.8)
                u.out.quads.push(q);
            }
        }
        for &(rf, (s, p, o), gr) in &seg.reifiers {
            let new_rf = u.map_term(seg, seg_idx, rf);
            let spo = (
                u.map_term(seg, seg_idx, s),
                u.map_term(seg, seg_idx, p),
                u.map_term(seg, seg_idx, o),
            );
            let graph_name = gr.map(|x| u.map_term(seg, seg_idx, x));
            u.out.set_reifier(new_rf, spo, graph_name);
        }
        for &(r, p, v, gr) in &seg.annotations {
            let row = (
                u.map_term(seg, seg_idx, r),
                u.map_term(seg, seg_idx, p),
                u.map_term(seg, seg_idx, v),
                gr.map(|x| u.map_term(seg, seg_idx, x)),
            );
            u.out.annotations.push(row);
        }
        for (digest, entry) in &seg.blobs {
            u.blob_index
                .set(&mut u.out.blobs, digest.clone(), entry.clone());
        }
        for (digest, meta) in &seg.blob_meta {
            u.blob_meta_index
                .set(&mut u.out.blob_meta, digest.clone(), meta.clone());
        }
        for (k, v) in &seg.meta {
            // file-level shallow merge; later segments win
            u.out.set_meta(k.clone(), v.clone());
        }
        u.out.segment_meta.extend(seg.segment_meta.iter().cloned());
        for sup in &seg.suppressions {
            let remapped = u.remap_suppression(sup, seg, seg_idx);
            u.out.suppressions.push(remapped);
        }
        u.out.opaque.extend(seg.opaque.iter().cloned());
        u.out.signatures.extend(seg.signatures.iter().cloned());
        u.out.diagnostics.extend(seg.diagnostics.iter().cloned());
        u.out
            .segment_heads
            .extend(seg.segment_heads.iter().cloned());
        u.out
            .segment_profiles
            .extend(seg.segment_profiles.iter().cloned());
        u.out
            .segment_streamable
            .extend(seg.segment_streamable.iter().cloned());
    }
    u.out
}

#[cfg(test)]
mod term_walk_tests {
    //! The union's term mapping against its recursive reference, on generated segment
    //! term tables, and at a hundred thousand levels on a 128 KiB thread.

    use super::{InternKey, Unioner};
    use crate::model::{Graph, Term, TermKind};

    /// The recursive reference of [`Unioner::map_term`], with its key.
    fn reference_map(union: &mut Unioner, seg: &Graph, seg_idx: usize, tid: usize) -> usize {
        let t = &seg.terms[tid];
        let key = match (t.kind, seg.term_triple(t)) {
            (TermKind::Triple, Some((s, p, o))) => InternKey::Qt(Some((
                reference_map(union, seg, seg_idx, s),
                reference_map(union, seg, seg_idx, p),
                reference_map(union, seg, seg_idx, o),
            ))),
            _ => Unioner::leaf_key(seg, seg_idx, tid),
        };
        if let Some(&got) = union.intern.get(&key) {
            return got;
        }
        let datatype = t.datatype.map(|d| reference_map(union, seg, seg_idx, d));
        let self_bound = t.kind == TermKind::Triple && t.reifier == Some(tid);
        let mapped_reifier = if self_bound {
            None
        } else {
            t.reifier.map(|r| reference_map(union, seg, seg_idx, r))
        };
        let triple = t.triple.map(|(s, p, o)| {
            (
                reference_map(union, seg, seg_idx, s),
                reference_map(union, seg, seg_idx, p),
                reference_map(union, seg, seg_idx, o),
            )
        });
        let new_id = union.out.terms.len();
        let reifier = if self_bound {
            Some(new_id)
        } else {
            mapped_reifier
        };
        let value = if t.kind == TermKind::Bnode {
            Some(match t.value.as_deref() {
                Some(label) if !label.is_empty() => format!("s{seg_idx}.{label}"),
                _ => format!("s{seg_idx}._anon{new_id}"),
            })
        } else {
            t.value.clone()
        };
        union.out.terms.push(Term {
            kind: t.kind,
            value,
            datatype,
            lang: t.lang.clone(),
            direction: t.direction.clone(),
            reifier,
            triple,
        });
        union.intern.insert(key, new_id);
        new_id
    }

    /// A SplitMix64 draw from the counter at `state`.
    const fn splitmix64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn term(kind: TermKind, value: Option<String>) -> Term {
        Term {
            kind,
            value,
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        }
    }

    /// A generated segment term table as the reader leaves one: every component,
    /// datatype and reifier names an earlier term (or, for a self-bound triple term, the
    /// term itself), triple terms state their components or reach them through a
    /// reifier row, and labels, values and shapes repeat so the union deduplicates.
    fn generated(seed: u64) -> Graph {
        /// A draw below `n` from the counter at `state`.
        fn draw(state: &mut u64, n: usize) -> usize {
            let n = u64::try_from(n).expect("a small bound fits");
            usize::try_from(splitmix64(state) % n).expect("a draw below a small bound fits")
        }
        let mut state = seed;
        let mut graph = Graph::default();
        let len = 2 + draw(&mut state, 12);
        for index in 0..len {
            let kind = if index == 0 { 0 } else { draw(&mut state, 6) };
            let next = match kind {
                0 => term(
                    TermKind::Iri,
                    Some(format!("http://example.org/i{}", index % 2)),
                ),
                1 => term(
                    TermKind::Bnode,
                    (index % 3 != 0).then(|| format!("b{}", index % 2)),
                ),
                2 => {
                    let mut literal = term(TermKind::Literal, Some("7".to_owned()));
                    literal.datatype = Some(draw(&mut state, index));
                    literal
                }
                3 => {
                    let mut triple = term(TermKind::Triple, None);
                    triple.triple = Some((
                        draw(&mut state, index),
                        draw(&mut state, index),
                        draw(&mut state, index),
                    ));
                    triple.reifier = (index % 2 == 0).then(|| draw(&mut state, index));
                    triple
                }
                4 => {
                    // Self-bound: the reifier row is keyed by the term's own id.
                    let mut triple = term(TermKind::Triple, None);
                    triple.reifier = Some(index);
                    let components = (
                        draw(&mut state, index),
                        draw(&mut state, index),
                        draw(&mut state, index),
                    );
                    graph.reifiers.push((index, components, None));
                    triple
                }
                _ => {
                    let mut triple = term(TermKind::Triple, None);
                    let components = (
                        draw(&mut state, index),
                        draw(&mut state, index),
                        draw(&mut state, index),
                    );
                    let rid = draw(&mut state, index);
                    graph.reifiers.push((rid, components, None));
                    triple.reifier = Some(rid);
                    triple
                }
            };
            graph.terms.push(next);
        }
        graph
    }

    /// Mapping every term of two generated segments, in order, answers the union ids and
    /// builds the union term table exactly as the recursive reference does.
    #[test]
    fn term_mapping_agrees_with_its_recursive_reference_on_generated_segments() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let (first, second) = (generated(seed), generated(seed + 10_000));
            let (mut union, mut expected) = (Unioner::default(), Unioner::default());
            for (seg_idx, seg) in [&first, &second].into_iter().enumerate() {
                for tid in 0..seg.terms.len() {
                    nested += usize::from(seg.terms[tid].kind == TermKind::Triple);
                    assert_eq!(
                        union.map_term(seg, seg_idx, tid),
                        reference_map(&mut expected, seg, seg_idx, tid),
                        "seed {seed}, segment {seg_idx}, term {tid}"
                    );
                }
            }
            assert_eq!(union.out.terms, expected.out.terms, "seed {seed}");
        }
        assert!(nested > 0, "some generated segment holds a triple term");
    }

    /// A chain of triple terms a hundred thousand levels deep, each naming the next in
    /// its object slot, is mapped into a union on a thread whose whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_chain_maps_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut seg = Graph::default();
                for iri in ["s", "p", "o"] {
                    seg.terms.push(term(
                        TermKind::Iri,
                        Some(format!("http://example.org/{iri}")),
                    ));
                }
                let mut below = 2;
                for _ in 0..LEVELS {
                    let mut triple = term(TermKind::Triple, None);
                    triple.triple = Some((0, 1, below));
                    seg.terms.push(triple);
                    below = seg.terms.len() - 1;
                }
                let mut union = Unioner::default();
                let mapped = union.map_term(&seg, 0, below);
                assert_eq!(mapped, LEVELS + 2, "every component precedes its triple");
                assert_eq!(union.out.terms.len(), LEVELS + 3);
            })
            .expect("the thread starts")
            .join()
            .expect("the mapping did not overflow the thread's stack");
    }
}

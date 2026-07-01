// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{HashMap, HashSet};

use ciborium::value::Value;

use crate::model::{Graph, Quad, Suppression, Term, TermKind, Triple3};
use crate::reader::{as_idx, as_text, text_or};
use crate::wire::map_get;

// --------------------------------------------------------------------------- //
// Multi-segment union (§3.1, §7.5): term-ids are segment-scoped compression
// artifacts; the union re-interns BY TERM VALUE. Blank nodes carry a segment
// discriminator (labels are segment-local and never merge); quoted-triple
// terms intern through their bound SPO identity. Because the union is
// value-interned, "apply suppression value-wise" (§11) reduces to applying it
// by result-id.
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
    intern: HashMap<InternKey, usize>,
}

impl Unioner {
    fn key_for(&mut self, seg: &Graph, seg_idx: usize, tid: usize) -> InternKey {
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
            // Quoted triple: identity is the interned SPO binding. Self-bound
            // triple terms use `rf == tid`; do not recursively map the reifier.
            TermKind::Triple => InternKey::Qt(t.reifier.and_then(|rf| {
                seg.reifier(rf).map(|(s, p, o)| {
                    (
                        self.map_term(seg, seg_idx, s),
                        self.map_term(seg, seg_idx, p),
                        self.map_term(seg, seg_idx, o),
                    )
                })
            })),
        }
    }

    fn map_term(&mut self, seg: &Graph, seg_idx: usize, tid: usize) -> usize {
        let key = self.key_for(seg, seg_idx, tid);
        if let Some(&got) = self.intern.get(&key) {
            return got;
        }
        let t = seg.terms[tid].clone();
        let datatype = t.datatype.map(|d| self.map_term(seg, seg_idx, d));
        let self_bound = t.kind == TermKind::Triple && t.reifier == Some(tid);
        let mapped_reifier = if self_bound {
            None
        } else {
            t.reifier.map(|r| self.map_term(seg, seg_idx, r))
        };
        // Recursive datatype/reifier mapping can push terms, so capture this
        // term's output id only after those mappings have completed.
        let new_id = self.out.terms.len();
        let reifier = if self_bound {
            Some(new_id)
        } else {
            mapped_reifier
        };
        // Blank nodes are relabelled with a segment prefix (§7.1 permits
        // isomorphism-preserving relabeling): within a segment, byte-identical
        // entries already intern to one union term (§7.8); ACROSS segments the
        // same label names DIFFERENT nodes, and emitting the raw label from
        // the union would merge them. Label-less nodes (absent or empty "v")
        // are distinct TERMS under the intern key, so their serialized labels
        // must stay distinct too — the union id disambiguates them. Computed
        // after dt/rf mapping so out.terms.len() IS this term's id.
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
            datatype,
            lang: t.lang,
            direction: t.direction,
            reifier,
        });
        self.intern.insert(key, new_id);
        new_id
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
                        if let Some(tid) = as_idx(v) {
                            if tid < n {
                                let new = self.map_term(seg, seg_idx, tid);
                                return (k.clone(), Value::from(new as u64));
                            }
                        }
                    } else if kind == "quad" && key == Some("q") {
                        if let Value::Array(ids) = v {
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
            u.out.set_blob_entry(digest.clone(), entry.clone());
        }
        for (digest, meta) in &seg.blob_meta {
            u.out.set_blob_meta(digest.clone(), meta.clone());
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

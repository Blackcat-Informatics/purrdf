// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Falsifiable acceptance tests for the graph-partitioned succinct bitmap-triples
//! codec (`purrdf_core::ir::pack::triples`): every one of the 8 `(s, p, o)`
//! bound/unbound pattern shapes, crossed
//! with every [`GraphMatch`] variant, must return exactly the same SET of quads a
//! brute-force scan over the source `RdfDataset` would — before AND after a
//! `to_bytes`/`from_bytes` round trip.

use std::collections::{BTreeSet, HashSet};

use purrdf_core::ir::pack::bits::{IntVector, IntVectorRef, RankSelectRef};
use purrdf_core::ir::pack::dict::{PackDict, PackTermId};
use purrdf_core::ir::pack::triples::{PackTriplesError, Triples, TriplesRef};
use purrdf_core::{BlankScope, GraphMatch, RdfDataset, RdfDatasetBuilder, TermId, TermValue};

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// One resolved quad, in dataset-independent `TermValue` form (the comparable
/// unit both the brute-force oracle and the triples-codec query reduce to).
type ValueQuad = (TermValue, TermValue, TermValue, Option<TermValue>);

/// A candidate probe for one pattern axis: its dataset-independent value, its
/// dataset-local [`TermId`] (for the brute-force oracle), and the unified
/// [`PackTermId`] the triples codec should be queried with for THIS axis (which,
/// for the deliberately-mismatched "absent" probes below, may be a real id drawn
/// from the WRONG role space, proving the codec still yields no rows).
struct Probe {
    value: TermValue,
    term_id: TermId,
    pack_id: PackTermId,
}

/// The full fixture: a dataset rich enough to exercise every corner named in the
/// task brief (two named graphs, IRIs/literals/blanks/an RDF 1.2 triple term used
/// as BOTH a subject and an object, a predicate that is also an object, and
/// repeated subjects/predicates so adjacency groups have more than one member),
/// plus the pre-resolved dict/triples codecs and axis probe lists the parity
/// tests iterate over.
struct Fixture {
    dataset: std::sync::Arc<RdfDataset>,
    dict: PackDict,
    triples_bytes: Vec<u8>,
    g1: (TermId, PackTermId),
    g2: (TermId, PackTermId),
    subject_probes: Vec<Probe>,
    predicate_probes: Vec<Probe>,
    object_probes: Vec<Probe>,
}

fn build_fixture() -> Fixture {
    let mut b = RdfDatasetBuilder::new();

    let s1 = b.intern_iri("http://example.org/s1");
    let s2 = b.intern_iri("http://example.org/s2");
    let s3 = b.intern_iri("http://example.org/s3");
    let p1 = b.intern_iri("http://example.org/p1");
    let p2 = b.intern_iri("http://example.org/p2");
    let o1 = b.intern_iri("http://example.org/o1");
    let lit1 = b.intern_literal(purrdf_core::RdfLiteral {
        lexical_form: "hello".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    let lit2 = b.intern_literal(purrdf_core::RdfLiteral {
        lexical_form: "bonjour".to_string(),
        datatype: None,
        language: Some("fr".to_string()),
        direction: None,
    });
    let blank1 = b.intern_blank("b0", BlankScope::DEFAULT);
    // "only_subject" plays a subject role and NOTHING else anywhere in the
    // fixture — the dedicated "never an object" absent-probe term.
    let only_subject = b.intern_iri("http://example.org/only_subject");
    // The RDF 1.2 triple term `<< s1 p1 o1 >>`. RDF 1.2 forbids a quoted triple
    // as the SUBJECT of an asserted quad (`rdf-ir-triple-subject`), so "used as a
    // subject" here means the STRUCTURAL subject slot of an OUTER triple term
    // (`outer_triple`, below) — the legitimate way a triple term appears in
    // subject position. `inner_triple` is ALSO used directly as an object
    // (G1b), giving it the "both a subject and an object" role the fixture
    // brief calls for.
    let inner_triple = b.intern_triple(s1, p1, o1);
    let outer_triple = b.intern_triple(inner_triple, p2, o1);
    let g1 = b.intern_iri("http://example.org/g1");
    let g2 = b.intern_iri("http://example.org/g2");

    // -- Default graph ------------------------------------------------------
    b.push_quad(s1, p1, o1, None);
    b.push_quad(s1, p1, lit1, None); // repeated (s1, p1): >1-member So group
    b.push_quad(s1, p2, o1, None); // repeated subject s1, different predicate
    b.push_quad(s2, p1, o1, None); // repeated predicate p1, different subject
    b.push_quad(s3, p2, p1, None); // predicate p1 ALSO used as an object here
    b.push_quad(blank1, p1, o1, None); // blank-node subject
    b.push_quad(only_subject, p1, o1, None);

    // -- Named graph g1 -------------------------------------------------------
    b.push_quad(s3, p2, outer_triple, Some(g1)); // outer triple as OBJECT
    // (`inner_triple` sits in `outer_triple`'s structural SUBJECT slot).
    b.push_quad(s2, p1, inner_triple, Some(g1)); // inner triple as OBJECT too

    // -- Named graph g2 ---------------------------------------------------
    b.push_quad(s1, p1, o1, Some(g2)); // same (s,p,o) as the first default-graph
    // quad, but in a different graph — partition separation.
    b.push_quad(s3, p1, lit2, Some(g2));

    let dataset = b.freeze().expect("valid dataset");

    let dict_bytes = PackDict::encode(&dataset).to_bytes();
    let dict = PackDict::open(&dict_bytes).expect("dict opens");
    let triples_bytes = Triples::encode(&dict, &dataset).to_bytes();

    let non_predicate_id = |v: &TermValue| dict.id_by_value(v).expect("present in dict");
    let predicate_id = |v: &TermValue| dict.predicate_id_by_value(v).expect("present in dict");
    let dataset_id = |v: &TermValue| {
        dataset
            .term_id_by_value(v)
            .expect("interned in this dataset")
    };

    let s1v = iri("s1");
    let s2v = iri("s2");
    let s3v = iri("s3");
    let p1v = iri("p1");
    let p2v = iri("p2");
    let o1v = iri("o1");
    let lit1v = TermValue::simple_literal("hello");
    let lit2v = TermValue::lang_literal("bonjour", "fr");
    let blank1v = TermValue::Blank {
        label: "b0".to_string(),
        scope: BlankScope::DEFAULT,
    };
    let only_subject_v = iri("only_subject");
    let inner_triple_v = TermValue::Triple {
        s: Box::new(s1v.clone()),
        p: Box::new(p1v.clone()),
        o: Box::new(o1v.clone()),
    };
    let outer_triple_v = TermValue::Triple {
        s: Box::new(inner_triple_v.clone()),
        p: Box::new(p2v.clone()),
        o: Box::new(o1v.clone()),
    };

    let mk = |v: TermValue, pack_id: PackTermId| Probe {
        term_id: dataset_id(&v),
        pack_id,
        value: v,
    };

    // Subject/object axes both resolve via the non-predicate unified id space
    // (matching how `Triples::encode` resolves s/o) — including the dedicated
    // "never a subject"/"never an object" absent probes. Neither triple term is
    // a subject probe: RDF 1.2 forbids a quoted triple as an asserted quad's
    // subject, so no query could ever legitimately bind "s" to one.
    let subject_probes = vec![
        mk(s1v.clone(), non_predicate_id(&s1v)),
        mk(s2v.clone(), non_predicate_id(&s2v)),
        mk(s3v.clone(), non_predicate_id(&s3v)),
        mk(blank1v.clone(), non_predicate_id(&blank1v)),
        // Absent: a literal can never be a subject in valid RDF, and never IS
        // one anywhere in this fixture — guaranteed zero matches.
        mk(lit1v.clone(), non_predicate_id(&lit1v)),
    ];
    let predicate_probes = vec![
        mk(p1v.clone(), predicate_id(&p1v)),
        mk(p2v.clone(), predicate_id(&p2v)),
        // Absent: `lit1`'s NON-predicate unified id is a real dict entry, but it
        // is never a member of ANY partition's predicate-role local map, so
        // binding "p" to it must yield zero rows everywhere.
        mk(lit1v.clone(), non_predicate_id(&lit1v)),
    ];
    let object_probes = vec![
        mk(o1v.clone(), non_predicate_id(&o1v)),
        mk(lit1v.clone(), non_predicate_id(&lit1v)),
        mk(lit2v.clone(), non_predicate_id(&lit2v)),
        mk(p1v.clone(), non_predicate_id(&p1v)), // predicate-that-is-also-object
        mk(inner_triple_v.clone(), non_predicate_id(&inner_triple_v)),
        mk(outer_triple_v.clone(), non_predicate_id(&outer_triple_v)),
        // Absent: `only_subject` never appears as an object anywhere.
        mk(only_subject_v.clone(), non_predicate_id(&only_subject_v)),
    ];

    let g1_ids = (dataset_id(&iri("g1")), non_predicate_id(&iri("g1")));
    let g2_ids = (dataset_id(&iri("g2")), non_predicate_id(&iri("g2")));

    Fixture {
        dataset,
        dict,
        triples_bytes,
        g1: g1_ids,
        g2: g2_ids,
        subject_probes,
        predicate_probes,
        object_probes,
    }
}

/// Brute-force reference: every quad in `dataset` matching the id-space pattern,
/// resolved to dataset-independent `TermValue`s — an explicit linear scan and
/// filter, independent of any indexed/succinct machinery under test.
fn brute_force(
    dataset: &RdfDataset,
    s: Option<TermId>,
    p: Option<TermId>,
    o: Option<TermId>,
    g: GraphMatch<TermId>,
) -> HashSet<ValueQuad> {
    dataset
        .quads()
        .filter(|q| {
            s.is_none_or(|id| q.s == id)
                && p.is_none_or(|id| q.p == id)
                && o.is_none_or(|id| q.o == id)
                && g.matches(q.g)
        })
        .map(|q| {
            (
                dataset.term_value(q.s),
                dataset.term_value(q.p),
                dataset.term_value(q.o),
                q.g.map(|gid| dataset.term_value(gid)),
            )
        })
        .collect()
}

/// The triples codec's answer for the same pattern, resolved to
/// dataset-independent `TermValue`s via the dictionary.
fn codec_pattern(
    dict: &PackDict,
    triples: &TriplesRef<'_>,
    s: Option<PackTermId>,
    p: Option<PackTermId>,
    o: Option<PackTermId>,
    g: GraphMatch<PackTermId>,
) -> HashSet<ValueQuad> {
    triples
        .pattern(s, p, o, g)
        .map(|(s, p, o, g)| {
            (
                dict.term_value(s),
                dict.term_value(p),
                dict.term_value(o),
                g.map(|gid| dict.term_value(gid)),
            )
        })
        .collect()
}

/// The four [`GraphMatch`] selections the parity sweep crosses every pattern
/// shape with, paired for the dataset (`TermId`) and codec (`PackTermId`) sides.
fn graph_selections(fx: &Fixture) -> Vec<(GraphMatch<TermId>, GraphMatch<PackTermId>)> {
    vec![
        (GraphMatch::Default, GraphMatch::Default),
        (GraphMatch::Named(fx.g1.0), GraphMatch::Named(fx.g1.1)),
        (GraphMatch::Named(fx.g2.0), GraphMatch::Named(fx.g2.1)),
        (GraphMatch::Any, GraphMatch::Any),
    ]
}

/// Assert `codec_pattern` and `brute_force` agree for every combination of
/// bound/unbound axis probes, for the pattern shape selected by
/// `(bound_s, bound_p, bound_o)`, against `triples` (either the freshly-encoded
/// reader or a round-tripped one — the caller supplies which).
fn assert_shape_parity(
    fx: &Fixture,
    triples: &TriplesRef<'_>,
    bound_s: bool,
    bound_p: bool,
    bound_o: bool,
) {
    let s_choices: Vec<Option<&Probe>> = if bound_s {
        fx.subject_probes.iter().map(Some).collect()
    } else {
        vec![None]
    };
    let p_choices: Vec<Option<&Probe>> = if bound_p {
        fx.predicate_probes.iter().map(Some).collect()
    } else {
        vec![None]
    };
    let o_choices: Vec<Option<&Probe>> = if bound_o {
        fx.object_probes.iter().map(Some).collect()
    } else {
        vec![None]
    };

    for (g_tid, g_pack) in graph_selections(fx) {
        for &sp in &s_choices {
            for &pp in &p_choices {
                for &op in &o_choices {
                    let expected = brute_force(
                        &fx.dataset,
                        sp.map(|pr| pr.term_id),
                        pp.map(|pr| pr.term_id),
                        op.map(|pr| pr.term_id),
                        g_tid,
                    );
                    let actual = codec_pattern(
                        &fx.dict,
                        triples,
                        sp.map(|pr| pr.pack_id),
                        pp.map(|pr| pr.pack_id),
                        op.map(|pr| pr.pack_id),
                        g_pack,
                    );
                    assert_eq!(
                        actual,
                        expected,
                        "shape (s_bound={bound_s}, p_bound={bound_p}, o_bound={bound_o}) \
                         mismatch for s={:?} p={:?} o={:?} g={g_tid:?}",
                        sp.map(|pr| &pr.value),
                        pp.map(|pr| &pr.value),
                        op.map(|pr| &pr.value),
                    );
                }
            }
        }
    }
}

#[test]
fn all_pattern_shapes_match_brute_force_before_round_trip() {
    let fx = build_fixture();
    let triples = TriplesRef::from_bytes(&fx.triples_bytes).expect("opens");
    for bound_s in [false, true] {
        for bound_p in [false, true] {
            for bound_o in [false, true] {
                assert_shape_parity(&fx, &triples, bound_s, bound_p, bound_o);
            }
        }
    }
}

#[test]
fn full_scan_matches_dataset_quads() {
    let fx = build_fixture();
    let triples = TriplesRef::from_bytes(&fx.triples_bytes).expect("opens");

    let expected: HashSet<ValueQuad> = fx
        .dataset
        .quads()
        .map(|q| {
            (
                fx.dataset.term_value(q.s),
                fx.dataset.term_value(q.p),
                fx.dataset.term_value(q.o),
                q.g.map(|g| fx.dataset.term_value(g)),
            )
        })
        .collect();
    let actual: HashSet<ValueQuad> = triples
        .all_quads()
        .map(|(s, p, o, g)| {
            (
                fx.dict.term_value(s),
                fx.dict.term_value(p),
                fx.dict.term_value(o),
                g.map(|gid| fx.dict.term_value(gid)),
            )
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 11, "the fixture's total quad count");
}

#[test]
fn serialization_round_trip_preserves_every_pattern_result() {
    let fx = build_fixture();
    let before = TriplesRef::from_bytes(&fx.triples_bytes).expect("opens");

    // Re-derive the byte buffer from scratch a second time and open THAT — the
    // round trip under test is "encode -> to_bytes -> from_bytes", proven
    // lossless by requiring the reopened reader to answer identically to the
    // original for every shape (which is itself checked against the brute-force
    // oracle), not merely byte-identical to itself.
    let dict_bytes = PackDict::encode(&fx.dataset).to_bytes();
    let dict2 = PackDict::open(&dict_bytes).expect("dict opens");
    let bytes2 = Triples::encode(&dict2, &fx.dataset).to_bytes();
    assert_eq!(fx.triples_bytes, bytes2, "encode is deterministic");
    let after = TriplesRef::from_bytes(&bytes2).expect("opens");

    for bound_s in [false, true] {
        for bound_p in [false, true] {
            for bound_o in [false, true] {
                assert_shape_parity(&fx, &before, bound_s, bound_p, bound_o);
                assert_shape_parity(&fx, &after, bound_s, bound_p, bound_o);
            }
        }
    }
}

#[test]
fn named_graph_ids_match_dataset_named_graphs() {
    let fx = build_fixture();
    let triples = TriplesRef::from_bytes(&fx.triples_bytes).expect("opens");

    let expected: BTreeSet<TermValue> = fx
        .dataset
        .quads()
        .filter_map(|q| q.g)
        .map(|g| fx.dataset.term_value(g))
        .collect();
    let actual: BTreeSet<TermValue> = triples
        .named_graph_ids()
        .map(|id| fx.dict.term_value(id))
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(actual, BTreeSet::from([iri("g1"), iri("g2")]));
}

/// The FoQ index cross-check gap: `PartitionRef::from_bytes` sums every
/// `pred_counts`/`pred_totals`/`obj_counts` entry to cross-check it against
/// `sp.len()`/`n_triples`/`so.len()`. Those entries come straight off an
/// untrusted pack, so before this fix the summing helper used
/// `Iterator::sum`, which PANICS on `u64` overflow in a debug build and
/// SILENTLY WRAPS in a release build — a wrapped total could coincidentally
/// equal the expected length, letting a corrupted FoQ index pass its
/// cross-check (a silent wrong-answer risk), and the debug-build panic is
/// itself a DoS on untrusted input.
///
/// This builds a REAL two-predicate default-graph partition through the
/// production `Triples::encode` path, then splices the on-disk bytes of the
/// `pred_counts` int-vector: its two genuine entries (each `1`, the real
/// per-predicate group count) are replaced with two `u64::MAX` entries at bit
/// width 64. Width 64 is not an out-of-format value — `IntVector::with_width`
/// documents `0..=64` as its whole legal range, and `IntVectorRef::from_bytes`
/// accepts the substituted bytes as a perfectly well-formed int-vector (same
/// element count, self-consistent width/words). No genuine dataset would ever
/// need width 64 for a `pred_counts` entry (it is bounded by the partition's
/// own — tiny, in any realistic pack — triple count), so this is exactly the
/// kind of adversarial-but-well-formed byte pattern the fail-closed cross-check
/// exists to catch, not a shape `IntVectorRef`'s own parser would reject on its
/// own. `TriplesRef::from_bytes` must reject the tampered pack with
/// `PackTriplesError::Malformed(_)` — never panic, never silently accept.
#[test]
fn from_bytes_rejects_a_triples_index_whose_counts_overflow() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/overflow_s");
    let p1 = b.intern_iri("http://example.org/overflow_p1");
    let p2 = b.intern_iri("http://example.org/overflow_p2");
    let o = b.intern_iri("http://example.org/overflow_o");
    b.push_quad(s, p1, o, None);
    b.push_quad(s, p2, o, None);
    let dataset = b.freeze().expect("valid dataset");

    let dict_bytes = PackDict::encode(&dataset).to_bytes();
    let dict = PackDict::open(&dict_bytes).expect("dict opens");
    let triples_bytes = Triples::encode(&dict, &dataset).to_bytes();

    // Sanity: the freshly encoded pack opens cleanly before any tamper.
    TriplesRef::from_bytes(&triples_bytes).expect("a freshly encoded pack opens");

    // -- Walk the exact field sequence `PartitionRef::from_bytes` parses (the
    // fixed on-disk layout documented in triples.rs's module docs: graph_id,
    // n_triples, local_s/local_p/local_o, sp, bp, so, bo, pred_offsets,
    // pred_counts, ...) purely through PUBLIC readers, to locate
    // `pred_counts`'s exact byte span. --------------------------------------
    let version = triples_bytes[0];
    assert_eq!(version, 1, "triples format version");
    let mut pos = 1usize;
    let partition_count = u64::from_le_bytes(triples_bytes[pos..pos + 8].try_into().unwrap());
    assert_eq!(
        partition_count, 1,
        "the fixture has only the default-graph partition"
    );
    pos += 8;
    let plen = u64::from_le_bytes(triples_bytes[pos..pos + 8].try_into().unwrap()) as usize;
    pos += 8;
    let partition_start = pos;
    let partition = &triples_bytes[partition_start..partition_start + plen];

    let mut ppos = 16usize; // graph_id: u64 (8B) + n_triples: u64 (8B)
    let local_s = IntVectorRef::from_bytes(&partition[ppos..]).expect("local_s parses");
    ppos += local_s.serialized_len();
    let local_p = IntVectorRef::from_bytes(&partition[ppos..]).expect("local_p parses");
    ppos += local_p.serialized_len();
    assert_eq!(local_p.len(), 2, "the fixture has exactly two predicates");
    let local_o = IntVectorRef::from_bytes(&partition[ppos..]).expect("local_o parses");
    ppos += local_o.serialized_len();
    let sp = IntVectorRef::from_bytes(&partition[ppos..]).expect("sp parses");
    ppos += sp.serialized_len();
    let bp = RankSelectRef::from_bytes(&partition[ppos..]).expect("bp parses");
    ppos += bp.serialized_len();
    let so = IntVectorRef::from_bytes(&partition[ppos..]).expect("so parses");
    ppos += so.serialized_len();
    let bo = RankSelectRef::from_bytes(&partition[ppos..]).expect("bo parses");
    ppos += bo.serialized_len();
    let pred_offsets = IntVectorRef::from_bytes(&partition[ppos..]).expect("pred_offsets parses");
    ppos += pred_offsets.serialized_len();
    let pred_counts = IntVectorRef::from_bytes(&partition[ppos..]).expect("pred_counts parses");
    let pred_counts_start = ppos;
    let pred_counts_old_len = pred_counts.serialized_len();

    assert_eq!(pred_counts.len(), 2, "one count per predicate");
    assert_eq!(pred_counts.get(0), 1, "p1 has exactly one (s,p) group");
    assert_eq!(pred_counts.get(1), 1, "p2 has exactly one (s,p) group");
    assert_ne!(
        pred_counts.width(),
        64,
        "the real encoder picks a tiny width for these small counts, proving the \
         width-64 substitute below is a genuine widening, not a no-op"
    );

    // -- Splice in a substitute `pred_counts`: same 2 entries, but each
    // `u64::MAX` at width 64 — legal per the on-disk format, unsummable
    // without overflow. --------------------------------------------------
    let mut tampered = IntVector::with_width(64);
    tampered.push(u64::MAX);
    tampered.push(u64::MAX);
    let tampered_bytes = tampered.to_bytes();

    let mut new_partition =
        Vec::with_capacity(partition.len() - pred_counts_old_len + tampered_bytes.len());
    new_partition.extend_from_slice(&partition[..pred_counts_start]);
    new_partition.extend_from_slice(&tampered_bytes);
    new_partition.extend_from_slice(&partition[pred_counts_start + pred_counts_old_len..]);

    let mut new_triples_bytes = Vec::with_capacity(1 + 8 + 8 + new_partition.len());
    new_triples_bytes.push(version);
    new_triples_bytes.extend_from_slice(&1u64.to_le_bytes()); // partition_count
    new_triples_bytes.extend_from_slice(&(new_partition.len() as u64).to_le_bytes());
    new_triples_bytes.extend_from_slice(&new_partition);

    let err = TriplesRef::from_bytes(&new_triples_bytes).expect_err(
        "predicate-index counts that overflow when summed must be rejected, not panic or \
         silently accepted via a wrapped sum",
    );
    assert_eq!(
        err,
        PackTriplesError::Malformed("triples: predicate index counts do not sum to sp.len()"),
        "expected the pred_counts cross-check to be the one that catches this, got {err:?}"
    );
}

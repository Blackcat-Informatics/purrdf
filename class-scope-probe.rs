// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Temporary inspection of checked native clash scope on tiny authored worlds.

use purrdf_core::RdfDatasetBuilder;
use purrdf_entail::{Question, Reasoner};
use purrdf_iri::vocab::{owl, rdf, rdfs};

fn main() {
    let base = vec![
        ("urn:x", rdf::TYPE, "urn:C"),
        ("urn:x", rdf::TYPE, "urn:notC"),
        ("urn:notC", owl::COMPLEMENT_OF, "urn:C"),
        ("urn:z", rdf::TYPE, "urn:D"),
    ];
    let mut equality = base.clone();
    equality[1].0 = "urn:y";
    equality.push(("urn:x", owl::SAME_AS, "urn:y"));
    let nominal = vec![
        ("urn:U", owl::ONE_OF, "urn:H"),
        ("urn:H", rdf::FIRST, "urn:a"),
        ("urn:H", rdf::REST, rdf::NIL),
        ("urn:x", rdf::TYPE, "urn:U"),
        ("urn:y", rdf::TYPE, "urn:U"),
        ("urn:x", owl::DIFFERENT_FROM, "urn:y"),
        ("urn:z", rdf::TYPE, "urn:D"),
    ];
    let choice = vec![
        ("urn:x", rdf::TYPE, "urn:U"),
        ("urn:U", owl::UNION_OF, "urn:H"),
        ("urn:H", rdf::FIRST, "urn:A"),
        ("urn:H", rdf::REST, "urn:T"),
        ("urn:T", rdf::FIRST, "urn:B"),
        ("urn:T", rdf::REST, rdf::NIL),
        ("urn:A", rdfs::SUB_CLASS_OF, "urn:C"),
        ("urn:A", rdfs::SUB_CLASS_OF, "urn:notC"),
        ("urn:notC", owl::COMPLEMENT_OF, "urn:C"),
        ("urn:B", rdfs::SUB_CLASS_OF, "urn:D"),
        ("urn:B", rdfs::SUB_CLASS_OF, "urn:notD"),
        ("urn:notD", owl::COMPLEMENT_OF, "urn:D"),
        ("urn:z", rdf::TYPE, "urn:unrelated"),
    ];
    for (name, rows) in [("local", base), ("equality", equality), ("nominal", nominal), ("choice", choice)] {
        let mut builder = RdfDatasetBuilder::new();
        for (s, p, o) in rows {
            let s = builder.intern_iri(s).unwrap();
            let p = builder.intern_iri(p).unwrap();
            let o = builder.intern_iri(o).unwrap();
            builder.push_quad(s, p, o, None).unwrap();
        }
        let dataset = builder.freeze().unwrap();
        let owner = Reasoner::with_proofs(&dataset).unwrap();
        let answer = owner.consistency();
        println!("CASE {name}: {:?}", answer.answer());
        let proof = answer.proof().unwrap();
        let context = owner.borrowed_proof_context().unwrap();
        let checked = proof.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &context).unwrap();
        for (run, refutation) in checked.refutations() {
            println!("RUN {run} root {:?}", refutation.proof().root());
            for branch in refutation.branches() { println!("BRANCH {branch:?}"); }
            for clash in refutation.clashes() {
                println!("CLASH {} merge={} reads {:?}", clash.index(), clash.is_merge(), clash.root().reads().collect::<Vec<_>>());
                for step in clash.steps() { println!("STEP {} {:?} READS {:?}", step.ordinal(), step.observation(), step.reads().collect::<Vec<_>>()); }
            }
        }
    }
}

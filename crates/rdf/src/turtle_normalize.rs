// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A canonical, review-friendly Turtle serializer over the **purrdf IR**.
//!
//! Replaces rdflib's `longturtle` as the on-disk normalizer (`purrdf normalize`).
//! The IR ([`RdfDataset`]) — not oxigraph — is the representation that is read,
//! ordered, and rendered; the native [`parse_dataset`] codec
//! appears only as the text *parser* at the ingest edge. Every triple is interned
//! into the IR verbatim
//! (RDF-star triple terms stay triple-term objects, NOT split into reifier
//! tables — the native parser folds the statement layer, so the ingest flattens it
//! back to the un-folded flat quad stream before re-interning), so the rendered
//! graph is identical to the input.
//!
//! The renderer itself ([`render`]) is the oxigraph-free half and lives in the
//! wasm-clean kernel ([`purrdf_core::turtle_render`]); it is re-exported here so
//! existing `purrdf::turtle_normalize::render` callers resolve unchanged. The text
//! *parser* edge ([`canonical_turtle`] / `ingest`) is the native codec — fully
//! oxigraph-free (EPIC #906).

use std::sync::Arc;

use crate::ir::{RdfDataset, RdfDatasetBuilder, TermId};
use crate::native_quads::flat_rdf_quads_from_dataset;
use crate::{parse_dataset, BlankScope, NativeRdfFormat, RdfTerm};

/// The canonical, review-friendly Turtle renderer — the oxigraph-free half, now in
/// the wasm-clean kernel. Re-exported so `purrdf::turtle_normalize::render`
/// resolves unchanged for in-tree callers.
pub use crate::turtle_render::render;

/// Parse a Turtle document and re-serialize it as canonical, review-friendly
/// Turtle. `extra_prefixes` supplies prefix bindings (the project's standard set);
/// only those actually used appear in the header.
pub fn canonical_turtle(
    input: &[u8],
    extra_prefixes: &[(String, String)],
) -> Result<String, String> {
    let dataset = ingest(input)?;
    Ok(render(&dataset, extra_prefixes))
}

/// Ingest: the native codec parses the Turtle text at the edge into the IR, which
/// is flattened back to the un-folded flat quad stream (RDF-star triple terms
/// preserved as triple-term objects, nothing reclassified) and re-interned verbatim
/// into a fresh builder so the rendered graph is identical to the input.
fn ingest(input: &[u8]) -> Result<Arc<RdfDataset>, String> {
    let parsed = parse_dataset(input, NativeRdfFormat::Turtle.media_type(), None)
        .map_err(|e| format!("Turtle parse error: {e}"))?;
    let mut builder = RdfDatasetBuilder::new();
    for quad in flat_rdf_quads_from_dataset(&parsed) {
        let s = intern_term(&mut builder, &quad.subject)?;
        let p = builder.intern_iri(quad.predicate);
        let o = intern_term(&mut builder, &quad.object)?;
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().map_err(|e| e.to_string())
}

fn intern_term(builder: &mut RdfDatasetBuilder, term: &RdfTerm) -> Result<TermId, String> {
    Ok(match term {
        RdfTerm::Iri(n) => builder.intern_iri(n.clone()),
        RdfTerm::BlankNode(b) => builder.intern_blank(b.clone(), BlankScope::DEFAULT),
        RdfTerm::Literal(l) => builder.intern_literal(l.clone()),
        RdfTerm::Triple(t) => {
            let s = intern_term(builder, &t.subject)?;
            let p = builder.intern_iri(t.predicate.clone());
            let o = intern_term(builder, &t.object)?;
            builder.intern_triple(s, p, o)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn prefixes() -> Vec<(String, String)> {
        vec![
            ("rdf".into(), RDF.into()),
            (
                "rdfs".into(),
                "http://www.w3.org/2000/01/rdf-schema#".into(),
            ),
            ("owl".into(), "http://www.w3.org/2002/07/owl#".into()),
            ("xsd".into(), XSD.into()),
            ("ex".into(), "http://example.org/".into()),
        ]
    }

    fn norm(ttl: &str) -> String {
        canonical_turtle(ttl.as_bytes(), &prefixes()).expect("normalize")
    }

    fn iso(a: &str, b: &str) -> bool {
        let da = ingest(a.as_bytes()).unwrap();
        let db = ingest(b.as_bytes()).unwrap();
        crate::ir::datasets_isomorphic(&da, &db)
    }

    #[test]
    fn isomorphism_preserved_and_idempotent() {
        let src = r#"
            @prefix ex: <http://example.org/> .
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            ex:A a owl:Class ;
                rdfs:label "A" ;
                rdfs:subClassOf [ a owl:Restriction ;
                    owl:onProperty ex:p ; owl:someValuesFrom ex:B ] .
        "#;
        let once = norm(src);
        assert!(iso(src, &once), "isomorphic to input:\n{once}");
        let twice = norm(&once);
        assert_eq!(once, twice, "idempotent");
        assert!(
            once.contains("rdfs:subClassOf [\n"),
            "inline bnode:\n{once}"
        );
        assert!(
            once.contains("        a owl:Restriction ;"),
            "a-first nested:\n{once}"
        );
    }

    #[test]
    fn reifier_annotation_render_is_flat_and_idempotent() {
        // #1155 bug 2 + Task 5 guard. The canonical renderer must emit the RDF 1.2
        // statement layer (reifier bindings + annotations) from the SIDE-TABLES — which
        // `canonical_turtle`/`ingest` flattens away, but the #1142 byte-exact fold renders
        // directly — flat (never nested) and byte-idempotent under parse→render→parse.
        // Use `parse_dataset` + `render` directly (NOT `canonical_turtle`, which flattens
        // the side-tables before rendering and so never exercises this path).
        let nt = concat!(
            "<http://example.org/s> <http://example.org/label> \"Hello\" .\n",
            "<http://example.org/r1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <http://example.org/s> <http://example.org/label> \"Hello\" )>> .\n",
            "<http://example.org/r1> <http://example.org/confidence> \"0.9\" .\n",
        );
        let ds1 = parse_dataset(nt.as_bytes(), NativeRdfFormat::NTriples.media_type(), None)
            .expect("parse1");
        assert_eq!(ds1.reifiers().count(), 1, "one reifier folded");
        assert_eq!(ds1.annotations().count(), 1, "one annotation folded");

        let text1 = render(&ds1, &prefixes());
        // The reifier statement is EMITTED (not dropped) and FLAT (not a nested
        // `rdf:reifies [ rdf:reifies … ]`).
        assert!(
            text1.contains("rdf:reifies <<"),
            "reifier emitted flat:\n{text1}"
        );
        assert!(
            !text1.contains("rdf:reifies [\n"),
            "reifier NOT nested:\n{text1}"
        );
        assert!(
            text1.contains("ex:confidence \"0.9\""),
            "annotation emitted:\n{text1}"
        );

        let ds2 = parse_dataset(text1.as_bytes(), NativeRdfFormat::Turtle.media_type(), None)
            .expect("parse2");
        // No growth across the round trip — the classic non-idempotence failure adds
        // `#reifiers` quads per cycle.
        assert_eq!(
            ds2.reifiers().count(),
            ds1.reifiers().count(),
            "reifier count stable"
        );
        assert_eq!(
            ds2.annotations().count(),
            ds1.annotations().count(),
            "annotation count stable"
        );
        assert_eq!(
            ds2.quad_count(),
            ds1.quad_count(),
            "base quad count stable (no growth)"
        );

        // Byte-idempotent: a second render is identical to the first.
        let text2 = render(&ds2, &prefixes());
        assert_eq!(
            text1, text2,
            "render is byte-idempotent:\n{text1}\n---\n{text2}"
        );
    }

    #[test]
    fn rdf_collection_renders_as_parens() {
        let src = r#"
            @prefix ex: <http://example.org/> .
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            ex:U owl:unionOf ( ex:A ex:B ex:C ) .
        "#;
        let out = norm(src);
        assert!(out.contains("owl:unionOf ( ex:A ex:B ex:C )"), "{out}");
        assert!(iso(src, &out));
    }

    #[test]
    fn literals_use_native_syntax() {
        let src = r#"
            @prefix ex: <http://example.org/> .
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
            ex:s ex:i 42 ; ex:d "1.5"^^xsd:decimal ; ex:b true ;
                 ex:plain "hi" ; ex:typed "hi"^^xsd:string ; ex:lang "bonjour"@fr .
        "#;
        let out = norm(src);
        assert!(out.contains("ex:i 42 "), "{out}");
        assert!(out.contains("ex:d 1.5 "), "{out}");
        assert!(out.contains("ex:b true "), "{out}");
        assert!(out.contains("ex:plain \"hi\" "), "{out}");
        assert!(out.contains("ex:typed \"hi\" "), "{out}");
        assert!(out.contains("ex:lang \"bonjour\"@fr "), "{out}");
        assert!(iso(src, &out));
    }

    #[test]
    fn directional_literal_round_trips() {
        // RDF 1.2 base direction must survive normalize: render `@lang--dir` and stay
        // isomorphic to the input (oxigraph's Turtle parser round-trips the `--dir`
        // form at ingest, so the isomorphism gate holds).
        let src = r#"
            @prefix ex: <http://example.org/> .
            ex:s ex:rtl "مرحبا"@ar--rtl ;
                 ex:ltr "hello"@en--ltr .
        "#;
        let out = norm(src);
        assert!(
            out.contains("\"مرحبا\"@ar--rtl"),
            "rtl direction rendered:\n{out}"
        );
        assert!(
            out.contains("\"hello\"@en--ltr"),
            "ltr direction rendered:\n{out}"
        );
        assert!(iso(src, &out), "directional literal preserved:\n{out}");
        // Idempotent: re-normalizing the output is byte-identical.
        assert_eq!(out, norm(&out), "idempotent");
    }

    #[test]
    fn only_used_prefixes_in_header() {
        let src = "@prefix ex: <http://example.org/> .\nex:a ex:p ex:o .\n";
        let out = norm(src);
        assert!(out.starts_with("@prefix ex:"), "{out}");
        assert!(!out.contains("owl:"), "unused prefixes omitted:\n{out}");
    }

    #[test]
    fn shared_blank_gets_stable_label() {
        // A blank referenced by two subjects cannot inline; it gets a _:bN label,
        // and re-normalizing is idempotent.
        let src = r#"
            @prefix ex: <http://example.org/> .
            ex:A ex:p _:x .
            ex:B ex:q _:x .
            _:x ex:v "shared" .
        "#;
        let out = norm(src);
        assert!(out.contains("_:b0"), "shared blank labeled:\n{out}");
        assert_eq!(out, norm(&out), "idempotent with shared blank");
        assert!(iso(src, &out));
    }

    #[test]
    fn nested_sibling_blanks_order_by_deep_content() {
        // Two inline blank siblings under the SAME predicate that are identical except
        // for a value buried two levels down. Their order must be decided by that deep
        // content — NOT by blank-node interning order — so presenting the siblings in
        // either source order normalizes to byte-identical output.
        let a = r#"
            @prefix ex: <http://example.org/> .
            ex:S ex:p
                [ ex:tag "same" ; ex:child [ ex:leaf "alpha" ] ] ,
                [ ex:tag "same" ; ex:child [ ex:leaf "beta" ] ] .
        "#;
        // Same graph, siblings written in the opposite source order.
        let b = r#"
            @prefix ex: <http://example.org/> .
            ex:S ex:p
                [ ex:tag "same" ; ex:child [ ex:leaf "beta" ] ] ,
                [ ex:tag "same" ; ex:child [ ex:leaf "alpha" ] ] .
        "#;
        let na = norm(a);
        let nb = norm(b);
        assert_eq!(
            na, nb,
            "source order must not affect output:\n{na}\n---\n{nb}"
        );
        assert_eq!(na, norm(&na), "idempotent");
        assert!(iso(a, &na), "isomorphic to input:\n{na}");
        // The deep value that breaks the tie must appear before its sibling's.
        let pos_alpha = na.find("alpha").expect("alpha present");
        let pos_beta = na.find("beta").expect("beta present");
        assert!(
            pos_alpha < pos_beta,
            "deep content orders the siblings:\n{na}"
        );
    }
}

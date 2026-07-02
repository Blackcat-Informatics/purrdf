// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-class RDF list functions.
//!
//! Six primitive `rdf:List` operations declared as FnO functions and emitted to
//! `generated/projections/list-functions.fno.ttl` (folded into `purrdf.gts`):
//! `listLength`, `listGet`, `listIndexOf`, `listSlice`, `listConcat`,
//! `listContains`. They give external `rdf:List` data and SPARQL authors a named,
//! typed surface for the operations the logic layer resolves recursively
//! (`crates/logic/src/reason`) and the native SPARQL engine executes directly
//! (`crates/sparql-eval`, the `purrdf:list*` custom functions).
//!
//! Unlike `functions.fno.ttl` (PurRDF→external projection transforms derived from
//! the mapping DSL) these are *primitives* — they bind no PurRDF data predicate, so
//! their parameters/outputs carry `fno:type` (the RDF type guard) but no
//! `fno:predicate`. This is a hand-shaped catalog like `dsl/mappings/transforms.fno.ttl`,
//! but emitted into `generated/` so it ships in the bundle. The output is fixed
//! (six functions), hence deterministic by construction.
//!
//! All six are executably backed. The scalar readers — `listLength`, `listGet`,
//! `listIndexOf`, `listContains` — resolve via a recursive `rdf:first`/`rdf:rest`
//! walk with arithmetic builtins (conformance case `goal-rdf-list-functions`). The
//! list-constructing `listSlice`/`listConcat` invent a fresh `rdf:List`: the native
//! SPARQL engine mints the new cells and surfaces them (CONSTRUCT output / SELECT
//! auxiliary graph), and the logic layer derives the result content. The functions
//! carry no `rdfs:seeAlso` — there is no related on-graph resource to point at, and
//! process/tracking links do not belong in the ontology.

/// One list-function declaration.
struct ListFn {
    /// Local name (the issue-named function: `listLength`, …).
    name: &'static str,
    label: &'static str,
    definition: &'static str,
    /// Ordered (param-IRI-local, …) the function expects.
    expects: &'static [&'static str],
    /// The output IRI-local (`o<Name>`).
    output: &'static str,
    /// The output `fno:type` (an absolute IRI).
    output_type: &'static str,
}

/// One parameter/output individual (deduped param or per-function output).
struct ListTerm {
    /// IRI-local (`pList`, `oListLength`, …).
    local: &'static str,
    label: &'static str,
    definition: &'static str,
    /// `fno:type` (an absolute IRI).
    ty: &'static str,
}

const RDF_LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";
const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// The six functions, in stable order.
const FUNCTIONS: &[ListFn] = &[
    ListFn {
        name: "listLength",
        label: "list length",
        definition: "The number of members in an rdf:List (the length of the rdf:first/rdf:rest chain to rdf:nil). Executably backed by the reasoning layer: a recursive rdf:rest walk with arithmetic builtins under ProceduralPrologProfile (conformance case goal-rdf-list-functions).",
        expects: &["pList"],
        output: "oListLength",
        output_type: XSD_INTEGER,
    },
    ListFn {
        name: "listGet",
        label: "list get",
        definition: "The member of an rdf:List at a zero-based index (the nth rdf:first along the chain). Executably backed by the reasoning layer: a recursive walk with index arithmetic under ProceduralPrologProfile (conformance case goal-rdf-list-functions).",
        expects: &["pList", "pIndex"],
        output: "oListGet",
        output_type: RDFS_RESOURCE,
    },
    ListFn {
        name: "listIndexOf",
        label: "list index of",
        definition: "The zero-based index of the first occurrence of a value in an rdf:List, or absent when the value is not a member. Membership is term-exact: a value matches a member by RDF term identity (lexical form and datatype), not by SPARQL value-space, so 1 (xsd:integer) does not match 1 (xsd:decimal). Executably backed by the reasoning layer: a recursive walk with count arithmetic under ProceduralPrologProfile (conformance case goal-rdf-list-functions).",
        expects: &["pList", "pValue"],
        output: "oListIndexOf",
        output_type: XSD_INTEGER,
    },
    ListFn {
        name: "listSlice",
        label: "list slice",
        definition: "A new rdf:List of the members in the half-open index range [start, end) of an rdf:List (indices clamped to the list bounds; an out-of-range or inverted range yields rdf:nil). Constructs a new list (value invention): the native SPARQL engine mints the fresh rdf:first/rdf:rest cells and surfaces them in CONSTRUCT output or the SELECT auxiliary graph.",
        expects: &["pList", "pSliceStart", "pSliceEnd"],
        output: "oListSlice",
        output_type: RDF_LIST,
    },
    ListFn {
        name: "listConcat",
        label: "list concat",
        definition: "A new rdf:List that is the concatenation of two rdf:Lists (the members of the first followed by the members of the second). Constructs a new list (value invention): the native SPARQL engine mints the fresh rdf:first/rdf:rest cells and surfaces them in CONSTRUCT output or the SELECT auxiliary graph.",
        expects: &["pListA", "pListB"],
        output: "oListConcat",
        output_type: RDF_LIST,
    },
    ListFn {
        name: "listContains",
        label: "list contains",
        definition: "True when a value is a member of an rdf:List, false otherwise. Membership is term-exact: a value matches a member by RDF term identity (lexical form and datatype), not by SPARQL value-space, so 1 (xsd:integer) does not match 1 (xsd:decimal). Executably backed by the reasoning layer: a recursive rdf:first/rdf:rest membership walk under ProceduralPrologProfile (conformance case goal-rdf-list-functions).",
        expects: &["pList", "pValue"],
        output: "oListContains",
        output_type: XSD_BOOLEAN,
    },
];

/// The deduped parameter individuals (first-use order across `FUNCTIONS`).
const PARAMS: &[ListTerm] = &[
    ListTerm {
        local: "pList",
        label: "list",
        definition: "The input rdf:List the operation reads.",
        ty: RDF_LIST,
    },
    ListTerm {
        local: "pIndex",
        label: "index",
        definition: "A zero-based position into an rdf:List.",
        ty: XSD_INTEGER,
    },
    ListTerm {
        local: "pValue",
        label: "value",
        definition: "A value to locate as a member of an rdf:List.",
        ty: RDFS_RESOURCE,
    },
    ListTerm {
        local: "pSliceStart",
        label: "slice start",
        definition: "The inclusive zero-based start index of a slice.",
        ty: XSD_INTEGER,
    },
    ListTerm {
        local: "pSliceEnd",
        label: "slice end",
        definition: "The exclusive zero-based end index of a slice.",
        ty: XSD_INTEGER,
    },
    ListTerm {
        local: "pListA",
        label: "first list",
        definition: "The first (left) rdf:List of a concatenation.",
        ty: RDF_LIST,
    },
    ListTerm {
        local: "pListB",
        label: "second list",
        definition: "The second (right) rdf:List of a concatenation.",
        ty: RDF_LIST,
    },
];

/// The document node banner (`rdfs:comment` after `to_quads`, like
/// `functions.fno.ttl`). Note the predicate shift from the legacy hand-Turtle's
/// `skos:definition` to `rdfs:comment` is intentional — the FnO doc-node idiom uses
/// `rdfs:comment` for its generated banner.
const BANNER: &str =
    "GENERATED by `purrdf regenerate` (mappings) — DO NOT EDIT. Six primitive rdf:List operations \
     (listLength, listGet, listIndexOf, listSlice, listConcat, listContains) declared as FnO, all \
     executable as native SPARQL custom functions (purrdf:list*). The scalar readers — listContains, \
     listLength, listGet, listIndexOf — resolve via a recursive rdf:first/rdf:rest walk with \
     arithmetic builtins (conformance case goal-rdf-list-functions). The list-constructing \
     operations (listSlice, listConcat) invent a fresh rdf:List: the engine mints the new cells \
     and surfaces them in CONSTRUCT output or the SELECT auxiliary graph.";

/// Build the FnO catalog of the six primitive list functions from the
/// [`FUNCTIONS`]/[`PARAMS`] consts (the single source of truth), minting every
/// function/param/output IRI under the caller's [`SliceVocab`](crate::vocab::SliceVocab)
/// namespace.
///
/// These are PRIMITIVES: their params/outputs bind NO `fno:predicate` and the
/// functions are typed `fno:Function` ONLY (`kind_types` is empty — they are NOT
/// the consumer's `ProjectionFunction`). The maximal-information-flow `rdfs:label` /
/// `skos:definition` on every function, param, and output is carried via the
/// optional model fields and survives the shared [`purrdf::fno::to_quads`] path.
pub fn list_functions_catalog(vocab: &crate::vocab::SliceVocab) -> purrdf::fno::FnoCatalog {
    use purrdf::fno::{FnFunction, FnOutput, FnParam, FnoCatalog};

    let functions: Vec<FnFunction> = FUNCTIONS
        .iter()
        .map(|f| FnFunction {
            iri: vocab.term(f.name),
            label: f.label.to_owned(),
            description: Some(f.definition.to_owned()),
            // Primitive — `fno:Function` only.
            kind_types: vec![],
            // No `rdfs:seeAlso`: there is no related on-graph resource to point at,
            // and per-function backing status lives in `skos:definition` and the
            // document banner.
            see_also: None,
            expects: f.expects.iter().map(|p| vocab.term(p)).collect(),
            output: FnOutput {
                iri: vocab.term(f.output),
                predicate: None,
                r#type: f.output_type.to_owned(),
                label: Some(format!("{} result", f.label)),
                description: Some(format!("The result of {}:{}.", vocab.prefix_name(), f.name)),
            },
        })
        .collect();

    let params: Vec<FnParam> = PARAMS
        .iter()
        .map(|p| FnParam {
            iri: vocab.term(p.local),
            predicate: None,
            r#type: p.ty.to_owned(),
            required: true,
            label: Some(p.label.to_owned()),
            description: Some(p.definition.to_owned()),
        })
        .collect();

    FnoCatalog {
        ontology_iri: vocab.ontology_iri().to_owned(),
        // Keep the legacy doc-node IRI shape (`<vocab>list-functions`).
        document_iri: vocab.term("list-functions"),
        doc_label: "PURRDF first-class RDF list functions (FnO)".to_owned(),
        banner: BANNER.to_owned(),
        functions,
        params,
        implementations: vec![],
        mappings: vec![],
    }
}

/// Emit the FnO catalog of the six list functions as deterministic N-Triples.
///
/// Routes through the SAME validated [`purrdf::fno::to_quads`] serializer as
/// `functions.fno.ttl` (§19 one-path), then retags the internal `@x-purrdf-english`
/// language tag to the public `@en` and renders each quad as one N-Triples line.
/// The content is fixed, so re-running is byte-identical.
pub fn emit_list_functions(vocab: &crate::vocab::SliceVocab) -> String {
    let cat = list_functions_catalog(vocab);
    let tag_map: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::from([("x-purrdf-english".to_owned(), "en".to_owned())]);
    let quads: Vec<purrdf::RdfQuad> = purrdf::fno::to_quads(&cat)
        .into_iter()
        .map(|q| crate::mapping_support::retag_quad(q, &tag_map))
        .collect();
    quads.iter().map(purrdf::turtle::emit_quad).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdf_query::{Dataset, Object};
    use crate::vocab::SliceVocab;

    /// Pure fixtures use a caller-supplied example.org vocabulary.
    fn vocab() -> SliceVocab {
        SliceVocab::for_namespace("https://example.org/vocab/")
    }

    /// The fixture namespace the vocab mints terms under.
    const NS: &str = "https://example.org/vocab/";

    const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
    const FNO_FUNCTION: &str = "https://w3id.org/function/ontology#Function";
    const FNO_OUTPUT: &str = "https://w3id.org/function/ontology#Output";
    const FNO_PARAMETER: &str = "https://w3id.org/function/ontology#Parameter";
    const FNO_TYPE: &str = "https://w3id.org/function/ontology#type";
    const FNO_PREDICATE: &str = "https://w3id.org/function/ontology#predicate";

    /// Parse the emitted N-Triples into a dataset (the new committed-artifact form).
    fn emitted_store() -> Dataset {
        let text = emit_list_functions(&vocab());
        Dataset::parse(
            text.as_bytes(),
            "application/n-triples",
            "emitted list-functions",
        )
        .unwrap()
    }

    /// Every named-node subject of `?s a <type_iri>`.
    fn subjects_of_type(store: &Dataset, type_iri: &str) -> std::collections::BTreeSet<String> {
        store
            .subjects_of_type(type_iri)
            .unwrap()
            .into_iter()
            .collect()
    }

    #[test]
    fn six_functions_typed_fno_function_and_not_projection() {
        let store = emitted_store();
        let functions = subjects_of_type(&store, FNO_FUNCTION);
        assert_eq!(functions.len(), 6, "expected six fno:Function declarations");
        for name in [
            "listLength",
            "listGet",
            "listIndexOf",
            "listSlice",
            "listConcat",
            "listContains",
        ] {
            assert!(
                functions.contains(&format!("{NS}{name}")),
                "missing function {name}"
            );
        }
        // Primitives are NOT the consumer's ProjectionFunction.
        assert!(
            subjects_of_type(&store, &vocab().projection_function()).is_empty(),
            "list functions must not be typed as the consumer's ProjectionFunction"
        );
    }

    #[test]
    fn six_outputs_and_correct_param_count() {
        let store = emitted_store();
        assert_eq!(subjects_of_type(&store, FNO_OUTPUT).len(), 6);
        assert_eq!(
            subjects_of_type(&store, FNO_PARAMETER).len(),
            PARAMS.len(),
            "one fno:Parameter per deduped PARAMS entry"
        );
    }

    #[test]
    fn each_output_carries_its_specified_fno_type() {
        let store = emitted_store();
        for f in FUNCTIONS {
            let out = format!("{NS}{}", f.output);
            let found = store
                .objects(&out, FNO_TYPE)
                .unwrap()
                .iter()
                .any(|o| matches!(o, Object::Named(iri) if iri == f.output_type));
            assert!(found, "{}: output fno:type != {}", f.name, f.output_type);
        }
    }

    #[test]
    fn every_param_and_output_carries_an_rdfs_label() {
        let store = emitted_store();
        let mut targets: Vec<String> = PARAMS.iter().map(|p| format!("{NS}{}", p.local)).collect();
        targets.extend(FUNCTIONS.iter().map(|f| format!("{NS}{}", f.output)));
        for iri in targets {
            let has_label = !store.objects(&iri, RDFS_LABEL).unwrap().is_empty();
            assert!(has_label, "{iri} missing rdfs:label");
        }
    }

    #[test]
    fn no_fno_predicate_triples_exist_primitive_check() {
        let store = emitted_store();
        let mut count = 0usize;
        store.for_each_quad(|_, p, _, _| {
            if p == FNO_PREDICATE {
                count += 1;
            }
        });
        assert_eq!(count, 0, "primitives must bind no fno:predicate");
    }

    #[test]
    fn every_expected_param_is_defined() {
        // No function may reference a parameter that is not defined (dangling ref).
        let defined: std::collections::BTreeSet<&str> = PARAMS.iter().map(|p| p.local).collect();
        for f in FUNCTIONS {
            for p in f.expects {
                assert!(
                    defined.contains(p),
                    "function {} expects undefined {p}",
                    f.name
                );
            }
        }
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(emit_list_functions(&vocab()), emit_list_functions(&vocab()));
    }
}

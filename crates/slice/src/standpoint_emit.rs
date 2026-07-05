// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native standpoint-projection emission — PurRDF's hand-authored
//! `generated/queries/standpoint-*.rq` SPARQL CONSTRUCT projections.
//!
//! Unlike the per-profile SPARQL projections (the correspondence lowerings), these
//! queries are NOT compiled from the `mapping-dsl/` tree: they are fixed,
//! template-coded SPARQL strings. Six re-express the standpoint axis as each
//! peer model (Standpoint-OWL 2, CRMinf, PROV-O, Web Annotation, schema.org
//! Claim, BBC News Ontology) — the `standpointLabel` encoding and its siblings,
//! ported from the historical Python emitters verbatim. The seventh,
//! `standpoint-modality.rq`, is the table-driven projection of the six canonical
//! claim-modality axes down to the legacy five-value `purrdf:claimModality`.
//!
//! Each emitter assembles a fixed `header` + `body` and threads the body through
//! the shared `crate::mapping_support::prefix_block` (registry-ordered `PREFIX`
//! emission) so the declared prefixes track the body exactly as Python's
//! `_prefix_block` did. The output is **byte-identical** to the committed
//! `standpoint-*.rq` (the parity gate).

use std::collections::BTreeMap;

use crate::error::SliceError;
use crate::mapping_support::{GENERATED_BANNER, prefix_block, rename_template_prefix};
use crate::vocab::SliceVocab;

/// The committed file names for the seven standpoint projections (six peer-model
/// re-expressions plus the legacy-modality projection), in emission order.
const STANDPOINT_FILES: &[&str] = &[
    "standpoint-owl2.rq",
    "standpoint-crminf.rq",
    "standpoint-prov.rq",
    "standpoint-oa.rq",
    "standpoint-schema.rq",
    "standpoint-bbc.rq",
    "standpoint-modality.rq",
];

/// Emit every standpoint SPARQL projection, returning `{ "standpoint-<x>.rq" →
/// rq_text }` for all seven fixed projections. Every vocabulary term the
/// queries reference (`accordingTo`, `StandpointClaim`, `claimModality`, …) is
/// minted under the caller's [`SliceVocab`]; for a fixed vocab the output is
/// byte-deterministic.
///
/// These take no DSL input — they are constant template-coded queries — so the
/// `root` argument is accepted only for call-site symmetry with the other mapping
/// emitters and is unused. For the original consumer vocabulary the text is
/// byte-identical to the historical Python `emit_standpoint_*_sparql` emitters.
///
/// # Errors
///
/// Returns [`SliceError`] only for forward compatibility (currently infallible);
/// the constant bodies cannot fail to render.
pub fn emit_standpoint_sets(
    _root: &std::path::Path,
    vocab: &SliceVocab,
) -> Result<BTreeMap<String, String>, SliceError> {
    let texts = [
        emit_owl2(vocab),
        emit_crminf(vocab),
        emit_prov(vocab),
        emit_oa(vocab),
        emit_schema(vocab),
        emit_bbc(vocab),
        emit_modality(vocab),
    ];
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for (name, text) in STANDPOINT_FILES.iter().zip(texts) {
        out.insert((*name).to_owned(), text);
    }
    Ok(out)
}

/// Assemble a standpoint query from its fixed header + body: rename the
/// template's `purrdf:` CURIE tokens to the caller's prefix name, then thread
/// the renamed body through the shared registry-ordered prefix block (mirrors
/// the Python `f"{header}{_prefix_block(body)}\n\n{body}"` tail every emitter
/// shares) so the prefix label is bound to the caller's namespace.
fn assemble(header: &str, body: &str, vocab: &SliceVocab) -> String {
    let header = rename_template_prefix(header, vocab.prefix_name());
    let body = rename_template_prefix(body, vocab.prefix_name());
    format!("{header}{}\n\n{body}", prefix_block(&body, vocab))
}

// ── Standpoint-OWL 2 (standpointLabel) ──────────────────────────────────────────

fn emit_owl2(vocab: &SliceVocab) -> String {
    let label_iri = format!("{}#standpointLabel", vocab.ontology_iri());
    let label_concat = "    BIND(CONCAT(\"<standpointAxiom><\", ?op, \"><Standpoint name=\\\"\", ?spName, \"\\\"/></\", ?op, \"></standpointAxiom>\") AS ?label)\n";
    let sp_name = "    BIND(IF(!BOUND(?sp) || ?sp = purrdf:universalStandpoint, \"*\", STR(?sp)) AS ?spName)\n";
    let modality = "    BIND(IF(BOUND(?mod) && (?mod = purrdf:conceivable || ?mod = purrdf:probable), \"Diamond\", \"Box\") AS ?op)\n";
    let refuted_filter = "    FILTER(!BOUND(?mod) || ?mod != purrdf:refuted)\n";
    let body = format!(
        "CONSTRUCT {{\n\
         \x20   <{label_iri}> a owl:AnnotationProperty .\n\
         \x20   ?ax a owl:Axiom ;\n\
         \x20       owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o ;\n\
         \x20       <{label_iri}> ?label .\n\
         \x20   ?s ?p ?o .\n\
         }}\n\
         WHERE {{\n\
         \x20   {{ ?ax a owl:Axiom ;\n\
         \x20       owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:accordingTo ?sp }}\n\
         \x20     OPTIONAL {{ ?ax purrdf:standpointModality ?mod }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?claim a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?ax .\n\
         \x20     ?ax a owl:Axiom ;\n\
         \x20         owl:annotatedSource ?s ;\n\
         \x20         owl:annotatedProperty ?p ;\n\
         \x20         owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?claim purrdf:claimModality ?mod }} }}\n\
         {refuted_filter}{sp_name}{modality}{label_concat}\
         }}\n"
    );
    let header = format!(
        "# Projection: PURRDF → Standpoint-OWL 2 (standpointLabel). {GENERATED_BANNER}\n\
         # Lossless multi-perspective downcast: re-expresses purrdf:accordingTo +\n\
         # purrdf:standpointModality as the cl-tud/standpoint-owl2 standpointLabel\n\
         # encoding (Box=□ settled, Diamond=◊ possible/probable, name=* universal).\n\
         # Refuted (denied) claims are excluded — carried by standpoint-crminf.rq.\n\
         # Branch B (#127): StandpointClaim with reified-statement observedFeature.\n\
         # Branch C (generic-entity observedFeature) is excluded by design — the\n\
         # translator matches only owl:Axiom individuals.\n"
    );
    assemble(&header, &body, vocab)
}

// ── CRMinf (CIDOC-CRM Argumentation) ────────────────────────────────────────────

fn emit_crminf(vocab: &SliceVocab) -> String {
    let holder = "    BIND(COALESCE(?sp, purrdf:universalStandpoint) AS ?holder)\n";
    let value = "    BIND(IF(!BOUND(?mod), \"true\", IF(?mod = purrdf:refuted, \"false\", IF(?mod = purrdf:conceivable, \"possible\", IF(?mod = purrdf:probable, \"probable\", \"true\")))) AS ?value)\n";
    let subject_bind = "    BIND(COALESCE(?s, ?feature) AS ?subject)\n";
    let prop_text = "    BIND(IF(BOUND(?s), CONCAT(STR(?s), \" \", STR(?p), \" \", STR(?o)), STR(?feature)) AS ?propText)\n";
    let mint = "    BIND(IRI(CONCAT(STR(?ax), \"/argumentation\")) AS ?arg)\n\
                \x20   BIND(IRI(CONCAT(STR(?ax), \"/belief\")) AS ?belief)\n\
                \x20   BIND(IRI(CONCAT(STR(?ax), \"/proposition\")) AS ?prop)\n";
    let body = format!(
        "CONSTRUCT {{\n\
         \x20   ?arg a crminf:I1_Argumentation ;\n\
         \x20       crm:P14_carried_out_by ?holder ;\n\
         \x20       crminf:J2_concluded_that ?belief .\n\
         \x20   ?belief a crminf:I2_Belief ;\n\
         \x20       crminf:J4_that ?prop .\n\
         \x20   ?prop a crminf:I4_Proposition_Set ;\n\
         \x20       crm:P67_refers_to ?subject ;\n\
         \x20       rdf:value ?propText ;\n\
         \x20       crminf:J5_holds_to_be ?value .\n\
         }}\n\
         WHERE {{\n\
         \x20   {{ ?ax a owl:Axiom ;\n\
         \x20       owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:accordingTo ?sp }}\n\
         \x20     OPTIONAL {{ ?ax purrdf:standpointModality ?mod }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?stmt .\n\
         \x20     ?stmt owl:annotatedSource ?s ;\n\
         \x20           owl:annotatedProperty ?p ;\n\
         \x20           owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?feature .\n\
         \x20     FILTER NOT EXISTS {{ ?feature owl:annotatedSource ?s }}\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         {holder}{value}{subject_bind}{prop_text}{mint}\
         }}\n"
    );
    let header = format!(
        "# Projection: PURRDF → CRMinf (CIDOC-CRM Argumentation). {GENERATED_BANNER}\n\
         # Lossless belief projection: each standpoint-indexed statement becomes an\n\
         # I1 Argumentation (carried out by the standpoint) concluding an I2 Belief\n\
         # J4 that an I4 Proposition Set J5 holds to be true/probable/possible/false.\n\
         # The explicit belief value carries DENIAL faithfully (≥ CRMinf); the\n\
         # proposition is referred to, never asserted as fact.\n\
         # Branches B/C (#127): StandpointClaim with reified-statement or\n\
         # generic-entity observedFeature. Generic entities use ?feature as the\n\
         # referred-to subject.\n"
    );
    assemble(&header, &body, vocab)
}

// ── PROV-O (qualified attribution) ──────────────────────────────────────────────

fn emit_prov(vocab: &SliceVocab) -> String {
    let holder = "    BIND(COALESCE(?sp, purrdf:universalStandpoint) AS ?holder)\n";
    let mint = "    BIND(IRI(CONCAT(STR(?ax), \"/attribution\")) AS ?attr)\n";
    let body = format!(
        "CONSTRUCT {{\n\
         \x20   ?ax a prov:Entity ;\n\
         \x20       owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o ;\n\
         \x20       prov:wasAttributedTo ?holder ;\n\
         \x20       prov:qualifiedAttribution ?attr .\n\
         \x20   ?attr a prov:Attribution ;\n\
         \x20       prov:agent ?holder .\n\
         \x20   ?holder a prov:Agent .\n\
         }}\n\
         WHERE {{\n\
         \x20   {{ ?ax a owl:Axiom ;\n\
         \x20       owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:accordingTo ?sp }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?stmt .\n\
         \x20     ?stmt owl:annotatedSource ?s ;\n\
         \x20           owl:annotatedProperty ?p ;\n\
         \x20           owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?feature .\n\
         \x20     FILTER NOT EXISTS {{ ?feature owl:annotatedSource ?s }}\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         {holder}{mint}\
         }}\n"
    );
    let header = format!(
        "# Projection: PURRDF → PROV-O (qualified attribution). {GENERATED_BANNER}\n\
         # Perspective-preserving provenance: each reified claim is a prov:Entity\n\
         # attributed to its standpoint agent (prov:qualifiedAttribution). Every\n\
         # standpoint retained, none privileged; lossy-drop: belief value (modality)\n\
         # and confidence dropped — carried by standpoint-crminf.rq / -owl2.rq.\n\
         # Branches B/C (#127): StandpointClaim with reified-statement or\n\
         # generic-entity observedFeature. Generic-entity branch omits\n\
         # owl:annotated* (unbound, skipped).\n"
    );
    assemble(&header, &body, vocab)
}

// ── W3C Web Annotation (oa) ──────────────────────────────────────────────────────

fn emit_oa(vocab: &SliceVocab) -> String {
    let holder = "    BIND(COALESCE(?sp, purrdf:universalStandpoint) AS ?holder)\n";
    let target = "    BIND(COALESCE(?s, ?feature) AS ?target)\n";
    let mint = "    BIND(IRI(CONCAT(STR(?ax), \"/annotation\")) AS ?ann)\n";
    let body = format!(
        "CONSTRUCT {{\n\
         \x20   ?ann a oa:Annotation ;\n\
         \x20       oa:hasTarget ?target ;\n\
         \x20       oa:hasBody ?ax ;\n\
         \x20       oa:motivatedBy oa:describing ;\n\
         \x20       dcterms:creator ?holder .\n\
         \x20   ?ax owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o .\n\
         }}\n\
         WHERE {{\n\
         \x20   {{ ?ax a owl:Axiom ;\n\
         \x20       owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:accordingTo ?sp }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?stmt .\n\
         \x20     ?stmt owl:annotatedSource ?s ;\n\
         \x20           owl:annotatedProperty ?p ;\n\
         \x20           owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?feature .\n\
         \x20     FILTER NOT EXISTS {{ ?feature owl:annotatedSource ?s }}\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         {holder}{target}{mint}\
         }}\n"
    );
    let header = format!(
        "# Projection: PURRDF → W3C Web Annotation (oa). {GENERATED_BANNER}\n\
         # Perspective-preserving: each reified claim is an oa:Annotation (body = the\n\
         # quoted statement, target = its subject, creator = the standpoint). Every\n\
         # standpoint retained, none privileged; lossy-drop: belief value (modality)\n\
         # and confidence dropped — carried by standpoint-crminf.rq / -owl2.rq.\n\
         # Branches B/C (#127): StandpointClaim with reified-statement or\n\
         # generic-entity observedFeature. Generic-entity branch uses ?feature as\n\
         # oa:hasTarget.\n"
    );
    assemble(&header, &body, vocab)
}

// ── schema.org Claim ─────────────────────────────────────────────────────────────

fn emit_schema(vocab: &SliceVocab) -> String {
    let refuted_filter = "    FILTER(!BOUND(?mod) || ?mod != purrdf:refuted)\n";
    let holder = "    BIND(COALESCE(?sp, purrdf:universalStandpoint) AS ?holder)\n";
    let prop_text = "    BIND(IF(BOUND(?s), CONCAT(STR(?s), \" \", STR(?p), \" \", STR(?o)), STR(?feature)) AS ?propText)\n";
    let mint = "    BIND(IRI(CONCAT(STR(?ax), \"/claim\")) AS ?claim)\n";
    let body = format!(
        "CONSTRUCT {{\n\
         \x20   ?claim a schema:Claim ;\n\
         \x20       schema:author ?holder ;\n\
         \x20       schema:text ?propText .\n\
         }}\n\
         WHERE {{\n\
         \x20   {{ ?ax a owl:Axiom ;\n\
         \x20       owl:annotatedSource ?s ;\n\
         \x20       owl:annotatedProperty ?p ;\n\
         \x20       owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:accordingTo ?sp }}\n\
         \x20     OPTIONAL {{ ?ax purrdf:standpointModality ?mod }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?stmt .\n\
         \x20     ?stmt owl:annotatedSource ?s ;\n\
         \x20           owl:annotatedProperty ?p ;\n\
         \x20           owl:annotatedTarget ?o .\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         \x20   UNION\n\
         \x20   {{ ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?feature .\n\
         \x20     FILTER NOT EXISTS {{ ?feature owl:annotatedSource ?s }}\n\
         \x20     OPTIONAL {{ ?ax purrdf:claimModality ?mod }} }}\n\
         {refuted_filter}{holder}{prop_text}{mint}\
         }}\n"
    );
    let header = format!(
        "# Projection: PURRDF → schema.org Claim. {GENERATED_BANNER}\n\
         # Web / fact-check ecosystem: each (non-denied) claim is a schema:Claim\n\
         # authored by its standpoint; per-standpoint claims coexist (no single\n\
         # ClaimReview verdict). Refuted/denied claims excluded (carried by\n\
         # standpoint-crminf.rq); lossy-drop: belief value (modality) + confidence.\n\
         # Branches B/C (#127): StandpointClaim with reified-statement or\n\
         # generic-entity observedFeature. Generic-entity branch renders ?feature\n\
         # IRI as schema:text.\n"
    );
    assemble(&header, &body, vocab)
}

// ── BBC News Ontology ────────────────────────────────────────────────────────────

fn emit_bbc(vocab: &SliceVocab) -> String {
    let holder = "    BIND(COALESCE(?sp, purrdf:universalStandpoint) AS ?holder)\n";
    let mint = "    BIND(IRI(CONCAT(STR(?event), \"/news-event\")) AS ?newsEvent)\n";
    let body = format!(
        "CONSTRUCT {{\n\
         \x20   ?newsEvent a bbc:NewsEvent ;\n\
         \x20       bbc:about ?event ;\n\
         \x20       bbc:standpoint ?holder ;\n\
         \x20       bbc:modality ?mod .\n\
         }}\n\
         WHERE {{\n\
         \x20   ?ax a purrdf:StandpointClaim ;\n\
         \x20       purrdf:vantage ?sp ;\n\
         \x20       purrdf:observedFeature ?event .\n\
         \x20   ?event a purrdf:Event .\n\
         \x20   OPTIONAL {{ ?ax purrdf:claimModality ?mod }}\n\
         {holder}{mint}\
         }}\n"
    );
    let header = format!(
        "# Projection: PURRDF → BBC News Ontology. {GENERATED_BANNER}\n\
         # Media-standpoint projection: StandpointClaim about an Event becomes a\n\
         # bbc:NewsEvent with standpoint and modality metadata.\n"
    );
    assemble(&header, &body, vocab)
}

// ── Factored claim-modality legacy projection ────────────────────────────────

fn emit_modality(vocab: &SliceVocab) -> String {
    // Build one UNION arm per row of the DECOMPOSITIONS table — the single source
    // of truth, read at runtime so the emitter and the logic module share the same
    // five rows without hard-coding them a second time.
    let rows = crate::standpoint_modality::decompositions();

    // Build the WHERE arms (one UNION block per row).
    let mut arms: Vec<String> = Vec::new();
    for d in rows {
        let arm = format!(
            "{{ ?claim a purrdf:StandpointClaim ;\n\
             \x20       purrdf:claimPolarity purrdf:{polarity} ;\n\
             \x20       purrdf:claimModalForce purrdf:{modal_force} ;\n\
             \x20       purrdf:claimCredenceLevel purrdf:{credence} ;\n\
             \x20       purrdf:claimAssertoricForce purrdf:{assertoric_force} ;\n\
             \x20       purrdf:claimTruthDirectedness purrdf:{truth_directedness} ;\n\
             \x20       purrdf:claimSupportStatus purrdf:{support_status} .\n\
             \x20   BIND(purrdf:{legacy} AS ?legacy) }}",
            legacy = d.legacy,
            polarity = d.polarity,
            modal_force = d.modal_force,
            credence = d.credence,
            assertoric_force = d.assertoric_force,
            truth_directedness = d.truth_directedness,
            support_status = d.support_status,
        );
        arms.push(arm);
    }

    let where_union = arms.join("\n    UNION\n    ");

    let body = format!(
        "CONSTRUCT {{\n\
         \x20   ?claim purrdf:claimModality ?legacy .\n\
         }}\n\
         WHERE {{\n\
         \x20   {where_union}\n\
         }}\n"
    );

    let header = format!(
        "# Projection: PURRDF factored claim-modality axes \u{2192} legacy purrdf:claimModality. {GENERATED_BANNER}\n\
         # Read the six canonical axis properties (claimPolarity, claimModalForce,\n\
         # claimCredenceLevel, claimAssertoricForce, claimTruthDirectedness,\n\
         # claimSupportStatus) and reconstruct the legacy purrdf:claimModality value\n\
         # for any StandpointClaim whose axis tuple matches one of the five seeded\n\
         # decompositions. Tuples with no legacy equivalent produce no triple\n\
         # (hard-unsupported, never approximated \u{2014} Principle 9).\n"
    );
    assemble(&header, &body, vocab)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn first_diff(got: &str, want: &str) -> String {
        for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
            if g != w {
                return format!("line {}:\n  got:  {g:?}\n  want: {w:?}", i + 1);
            }
        }
        format!(
            "length differs (got {} lines, want {} lines)",
            got.lines().count(),
            want.lines().count()
        )
    }

    #[test]
    fn every_standpoint_file_matches_committed() {
        let root = repo_root();
        // Committed-artifact parity: the committed queries were generated with
        // the blackcatinformatics purrdf namespace (prefix `purrdf`), so this
        // cross-check must use it (pure fixtures elsewhere use example.org).
        let vocab = SliceVocab::for_namespace("https://blackcatinformatics.ca/gmeow/");
        let sets = emit_standpoint_sets(&root, &vocab).expect("emit standpoint");
        let dir = root.join("generated").join("queries");
        if !dir.exists() {
            eprintln!(
                "skipping committed standpoint query comparison; {} is absent",
                dir.display()
            );
            return;
        }
        let mut mismatches: Vec<String> = Vec::new();
        for (filename, text) in &sets {
            let committed_path = dir.join(filename);
            let committed = std::fs::read_to_string(&committed_path)
                .unwrap_or_else(|e| panic!("read {}: {e}", committed_path.display()));
            if *text != committed {
                mismatches.push(format!("{filename}: {}", first_diff(text, &committed)));
            }
        }
        assert!(
            mismatches.is_empty(),
            "{} standpoint file(s) differ:\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );
        assert_eq!(sets.len(), 7, "expected 7 standpoint files");
    }
}

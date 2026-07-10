// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RIF-in-XML parsing with caller-owned import resolution.

use std::sync::Arc;

use purrdf_core::{RdfDataset, TermValue};
use roxmltree::{Document, Node};

use crate::{Atom, EntailError, Fact, Regime, RifTerm, Rule, RuleSet, materialize};

const RIF_NS: &str = "http://www.w3.org/2007/rif#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// One RIF `Import` directive. Resolving its location is deliberately caller-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RifImport {
    /// The import location exactly as authored.
    pub location: String,
    /// Optional W3C entailment-profile IRI.
    pub profile: Option<String>,
}

/// A parsed RIF document before external imports are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRifDocument {
    /// Facts and rules carried directly by the document.
    pub ruleset: RuleSet,
    /// Imports in document order.
    pub imports: Vec<RifImport>,
}

/// Parse normative RIF XML into PurRDF's monotonic definite-Horn model.
///
/// # Errors
///
/// Returns [`EntailError::Parse`] for malformed XML and every unsupported or
/// unexpected construct; the parser never skips unknown semantics.
pub fn parse_rif_xml(text: &str) -> Result<ParsedRifDocument, EntailError> {
    parse_document(text).map_err(EntailError::Parse)
}

/// Resolve imports through a caller callback and merge their materialized
/// default-graph facts into the rule set.
///
/// The callback owns all I/O. `OWL-Direct` imports use OWL-RL's sound atomic-fact
/// subset; RDF, RDFS, and OWL-RL imports use their corresponding closure.
///
/// # Errors
///
/// Propagates resolver and entailment failures.
pub fn resolve_rif_imports<F>(
    parsed: ParsedRifDocument,
    mut resolver: F,
) -> Result<RuleSet, EntailError>
where
    F: FnMut(&RifImport) -> Result<Arc<RdfDataset>, EntailError>,
{
    let mut ruleset = parsed.ruleset;
    let mut imported_facts = Vec::new();
    for import in &parsed.imports {
        let source = resolver(import)?;
        let closed = materialize(&source, import_regime(import.profile.as_deref()))?;
        imported_facts.clear();
        fill_dataset_facts(&closed, &mut imported_facts);
        ruleset.facts.append(&mut imported_facts);
    }
    Ok(ruleset)
}

fn parse_document(text: &str) -> Result<ParsedRifDocument, String> {
    // RIF imports are resolved by the caller; XML DTD/entity expansion is never needed.
    let document = Document::parse(text).map_err(|error| error.to_string())?;
    let root = document.root_element();
    require(&root, "Document")?;
    let mut ruleset = RuleSet::new();
    let mut imports = Vec::new();
    for child in elements(&root) {
        match local_name(&child)? {
            "directive" => collect_import(&child, &mut imports)?,
            "payload" => parse_payload(&child, &mut ruleset)?,
            "meta" | "id" => {}
            other => return Err(format!("unexpected Document child <{other}>")),
        }
    }
    Ok(ParsedRifDocument { ruleset, imports })
}

fn collect_import(directive: &Node<'_, '_>, imports: &mut Vec<RifImport>) -> Result<(), String> {
    let import = only_element(directive, "Import")?;
    let mut location = None;
    let mut profile = None;
    for child in elements(&import) {
        match local_name(&child)? {
            "location" => location = Some(text_of(&child)),
            "profile" => profile = Some(text_of(&child)),
            "meta" | "id" => {}
            other => return Err(format!("unexpected Import child <{other}>")),
        }
    }
    imports.push(RifImport {
        location: location.ok_or("Import without a <location>")?,
        profile,
    });
    Ok(())
}

fn parse_payload(payload: &Node<'_, '_>, ruleset: &mut RuleSet) -> Result<(), String> {
    let group = only_element(payload, "Group")?;
    for child in elements(&group) {
        match local_name(&child)? {
            "sentence" => parse_sentence(&child, ruleset)?,
            "meta" | "id" | "behavior" => {}
            other => return Err(format!("unexpected Group child <{other}>")),
        }
    }
    Ok(())
}

fn parse_sentence(sentence: &Node<'_, '_>, ruleset: &mut RuleSet) -> Result<(), String> {
    let inner = single_element(sentence, "sentence")?;
    match local_name(&inner)? {
        "Frame" => {
            for atom in parse_frame(&inner)? {
                ruleset.push_fact(ground_fact(atom)?);
            }
            Ok(())
        }
        "Forall" => {
            ruleset.push_rule(parse_forall(&inner)?);
            Ok(())
        }
        other => Err(format!("unexpected sentence body <{other}>")),
    }
}

fn parse_forall(forall: &Node<'_, '_>) -> Result<Rule, String> {
    let mut formula = None;
    for child in elements(forall) {
        match local_name(&child)? {
            "declare" | "meta" | "id" => {}
            "formula" => formula = Some(child),
            other => return Err(format!("unexpected Forall child <{other}>")),
        }
    }
    let formula = formula.ok_or("Forall without a <formula>")?;
    let implies = single_element(&formula, "formula")?;
    require(&implies, "Implies")?;
    let mut body = None;
    let mut head = None;
    for child in elements(&implies) {
        match local_name(&child)? {
            "if" => body = Some(parse_conjunction(&single_element(&child, "if")?)?),
            "then" => head = Some(parse_conjunction(&single_element(&child, "then")?)?),
            "meta" | "id" => {}
            other => return Err(format!("unexpected Implies child <{other}>")),
        }
    }
    Ok(Rule {
        body: body.ok_or("Implies without an <if>")?,
        head: head.ok_or("Implies without a <then>")?,
    })
}

fn parse_conjunction(node: &Node<'_, '_>) -> Result<Vec<Atom>, String> {
    match local_name(node)? {
        "Frame" => parse_frame(node),
        "And" => {
            let mut atoms = Vec::new();
            for child in elements(node) {
                match local_name(&child)? {
                    "formula" => atoms.extend(parse_frame(&single_element(&child, "formula")?)?),
                    "meta" | "id" => {}
                    other => return Err(format!("unexpected And child <{other}>")),
                }
            }
            Ok(atoms)
        }
        other => Err(format!("unexpected conjunction node <{other}>")),
    }
}

fn parse_frame(frame: &Node<'_, '_>) -> Result<Vec<Atom>, String> {
    require(frame, "Frame")?;
    let mut object = None;
    let mut slots = Vec::new();
    for child in elements(frame) {
        match local_name(&child)? {
            "object" => object = Some(parse_term(&single_element(&child, "object")?)?),
            "slot" => slots.push(parse_slot(&child)?),
            "meta" | "id" => {}
            other => return Err(format!("unexpected Frame child <{other}>")),
        }
    }
    let subject = object.ok_or("Frame without an <object>")?;
    if slots.is_empty() {
        return Err("Frame without any <slot>".to_owned());
    }
    Ok(slots
        .into_iter()
        .map(|(predicate, object)| Atom {
            s: subject.clone(),
            p: predicate,
            o: object,
        })
        .collect())
}

fn parse_slot(slot: &Node<'_, '_>) -> Result<(RifTerm, RifTerm), String> {
    let mut children = elements(slot);
    let predicate = parse_term(&children.next().ok_or("slot without a predicate")?)?;
    let value = parse_term(&children.next().ok_or("slot without a value")?)?;
    if children.next().is_some() {
        return Err("slot with more than two children".to_owned());
    }
    Ok((predicate, value))
}

fn parse_term(node: &Node<'_, '_>) -> Result<RifTerm, String> {
    match local_name(node)? {
        "Var" => Ok(RifTerm::Var(text_of(node))),
        "Const" => Ok(RifTerm::Const(parse_const(node)?)),
        other => Err(format!("unexpected term node <{other}>")),
    }
}

fn parse_const(node: &Node<'_, '_>) -> Result<TermValue, String> {
    let kind = node
        .attribute("type")
        .ok_or("Const without a type attribute")?;
    let value = text_of(node);
    if kind.strip_prefix(RIF_NS) == Some("iri") {
        Ok(TermValue::iri(value))
    } else if kind.starts_with(XSD_NS) {
        Ok(TermValue::typed_literal(value, kind))
    } else if kind.strip_prefix(RIF_NS) == Some("local") {
        Err("rif:local const outside <meta> is unsupported".to_owned())
    } else {
        Err(format!("unsupported Const type {kind}"))
    }
}

fn ground_fact(atom: Atom) -> Result<Fact, String> {
    Ok((const_of(atom.s)?, const_of(atom.p)?, const_of(atom.o)?))
}

fn const_of(term: RifTerm) -> Result<TermValue, String> {
    match term {
        RifTerm::Const(value) => Ok(value),
        RifTerm::Var(name) => Err(format!("variable ?{name} in a ground fact")),
    }
}

fn import_regime(profile: Option<&str>) -> Regime {
    match profile.and_then(Regime::from_iri) {
        Some(Regime::OwlDirect | Regime::OwlRl) => Regime::OwlRl,
        Some(Regime::Rdfs) => Regime::Rdfs,
        Some(Regime::Rdf) => Regime::Rdf,
        _ => Regime::Simple,
    }
}

fn fill_dataset_facts(dataset: &RdfDataset, facts: &mut Vec<Fact>) {
    facts.extend(dataset.quads().filter(|quad| quad.g.is_none()).map(|quad| {
        (
            dataset.term_value(quad.s),
            dataset.term_value(quad.p),
            dataset.term_value(quad.o),
        )
    }));
}

fn elements<'a, 'input>(
    node: &Node<'a, 'input>,
) -> impl Iterator<Item = Node<'a, 'input>> + use<'a, 'input> {
    node.children().filter(Node::is_element)
}

fn local_name<'a>(node: &Node<'a, '_>) -> Result<&'a str, String> {
    let tag = node.tag_name();
    match tag.namespace() {
        Some(RIF_NS) => Ok(tag.name()),
        other => Err(format!(
            "element <{}> is not in the RIF namespace (found {other:?})",
            tag.name()
        )),
    }
}

fn require(node: &Node<'_, '_>, expected: &str) -> Result<(), String> {
    let actual = local_name(node)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected <{expected}>, found <{actual}>"))
    }
}

fn single_element<'a, 'input>(
    node: &Node<'a, 'input>,
    parent: &str,
) -> Result<Node<'a, 'input>, String> {
    let mut children = elements(node);
    let first = children
        .next()
        .ok_or_else(|| format!("<{parent}> is empty"))?;
    if children.next().is_some() {
        return Err(format!("<{parent}> has more than one child element"));
    }
    Ok(first)
}

fn only_element<'a, 'input>(
    node: &Node<'a, 'input>,
    name: &str,
) -> Result<Node<'a, 'input>, String> {
    let child = single_element(node, name)?;
    require(&child, name)?;
    Ok(child)
}

fn text_of(node: &Node<'_, '_>) -> String {
    let mut text = String::new();
    for child in node.children() {
        if let Some(value) = child.text() {
            text.push_str(value);
        }
    }
    text.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use purrdf_core::RdfDatasetBuilder;

    use super::*;

    const RIF: &str = r#"<Document xmlns="http://www.w3.org/2007/rif#">
  <directive><Import><location>facts.ttl</location><profile>http://www.w3.org/ns/entailment/RDFS</profile></Import></directive>
  <payload><Group><sentence><Frame>
    <object><Const type="http://www.w3.org/2007/rif#iri">https://example.org/s</Const></object>
    <slot ordered="yes"><Const type="http://www.w3.org/2007/rif#iri">https://example.org/p</Const><Const type="http://www.w3.org/2001/XMLSchema#string">value</Const></slot>
  </Frame></sentence></Group></payload>
</Document>"#;

    #[test]
    fn parses_fact_and_leaves_import_to_caller() {
        let parsed = parse_rif_xml(RIF).unwrap();
        assert_eq!(parsed.ruleset.facts.len(), 1);
        assert_eq!(parsed.imports[0].location, "facts.ttl");
    }

    #[test]
    fn rejects_unknown_construct() {
        let text = RIF
            .replace("<Frame>", "<Atom>")
            .replace("</Frame>", "</Atom>");
        assert!(matches!(parse_rif_xml(&text), Err(EntailError::Parse(_))));
    }

    #[test]
    fn accepts_rif_metadata_without_interpreting_it() {
        let text = RIF
            .replace(
                "<Document xmlns=\"http://www.w3.org/2007/rif#\">",
                "<Document xmlns=\"http://www.w3.org/2007/rif#\"><id/>",
            )
            .replace("<Group>", "<Group><meta/><behavior/>")
            .replace("<Frame>", "<Frame><id/><meta/>");
        let parsed = parse_rif_xml(&text).unwrap();
        assert_eq!(parsed.ruleset.facts.len(), 1);
    }

    #[test]
    fn rejects_dtds() {
        let text = RIF.replacen(
            "<Document",
            "<!DOCTYPE Document [<!ENTITY x \"expanded\">]><Document",
            1,
        );
        assert!(matches!(parse_rif_xml(&text), Err(EntailError::Parse(_))));
    }

    #[test]
    fn resolves_imports_and_merges_default_graph_facts() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/imported");
        let predicate = builder.intern_iri("https://example.org/p");
        let object = builder.intern_iri("https://example.org/o");
        builder.push_quad(subject, predicate, object, None);
        let imported = builder.freeze().unwrap();

        let parsed = parse_rif_xml(RIF).unwrap();
        let ruleset = resolve_rif_imports(parsed, |_| Ok(Arc::clone(&imported))).unwrap();
        assert!(ruleset.facts.iter().any(|fact| {
            fact == &(
                TermValue::iri("https://example.org/imported"),
                TermValue::iri("https://example.org/p"),
                TermValue::iri("https://example.org/o"),
            )
        }));
    }

    #[test]
    fn maps_import_profiles_to_supported_materializers() {
        let profile = |name: &str| Some(format!("http://www.w3.org/ns/entailment/{name}"));
        for (name, expected) in [
            ("OWL-Direct", Regime::OwlRl),
            ("OWL-RL", Regime::OwlRl),
            ("RDFS", Regime::Rdfs),
            ("RDF", Regime::Rdf),
            ("Simple", Regime::Simple),
            ("RIF", Regime::Simple),
        ] {
            let value = profile(name);
            assert_eq!(import_regime(value.as_deref()), expected);
        }
        assert_eq!(import_regime(None), Regime::Simple);
    }
}

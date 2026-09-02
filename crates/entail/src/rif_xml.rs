// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RIF-in-XML parsing with caller-owned import resolution.
//!
//! # `xml:base`
//!
//! RIF-in-XML is an XML dialect, so XML Base governs it exactly as it governs RDF/XML:
//! `xml:base` on an element establishes the base IRI for that element's subtree, an
//! `xml:base` may itself be relative (and then resolves against the base already in
//! force, RFC-3986 §5.1.1), and every IRI-valued attribute and IRI-valued element
//! content resolves against the base in force where it appears.
//!
//! That matters more here than in a data document: a RIF `Const` of type `rif:iri`
//! becomes a rule-head or rule-body PREDICATE, and an `Import` location names a graph to
//! fetch. A relative reference left unresolved does not fail loudly, it silently denotes
//! a different predicate than the author wrote — so this parser has no "no base in scope,
//! keep the raw text" fallthrough. With no base in scope a relative reference is
//! [`purrdf_iri::IriError::NoBase`], reported as [`EntailError::Parse`].
//!
//! The structure mirrors `purrdf_rdf`'s RDF/XML codec rather than inventing a second one:
//! one [`BaseScope`] rebound per element on the way down, and no RFC-3986 arithmetic of
//! its own — resolution is [`BaseScope::resolve`] throughout.

use std::borrow::Cow;
use std::sync::Arc;

use purrdf_core::{RdfDataset, TermValue};
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
use roxmltree::{Document, Node};

use crate::{
    Atom, EntailError, Fact, Materialization, Regime, RifTerm, Rule, RuleSet, materialize,
};

const RIF_NS: &str = "http://www.w3.org/2007/rif#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
/// The reserved XML namespace `xml:base` lives in (XML Base, §3).
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// One RIF `Import` directive. Fetching its location is deliberately caller-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RifImport {
    /// The import location, **resolved** against the base in force where the
    /// `<location>` appeared. Fetching it is the caller's; spelling it is not — an
    /// unresolved relative location would name a different graph for every reader.
    pub location: String,
    /// Optional W3C entailment-profile IRI, likewise resolved against the base in force.
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
/// `base` is the caller-supplied base IRI (RFC-3986 §5.1.2) — the document's own
/// retrieval IRI when the caller read it from somewhere, and `None` for a rule document
/// handed over as bytes with no origin. An in-document `xml:base` (§5.1.1) takes
/// precedence over it, exactly as in RDF/XML. With neither, a relative IRI reference in
/// the document is a hard error rather than a predicate silently renamed.
///
/// # Errors
///
/// Returns [`EntailError::Parse`] for malformed XML, a non-absolute `base`, an
/// unresolvable relative IRI reference, and every unsupported or unexpected construct;
/// the parser never skips unknown semantics.
pub fn parse_rif_xml(text: &str, base: Option<&str>) -> Result<ParsedRifDocument, EntailError> {
    let scope = base_scope(base).map_err(EntailError::Parse)?;
    parse_document(text, &scope).map_err(EntailError::Parse)
}

/// The initial base scope: rooted at the caller's `base`, or empty when there is none.
///
/// A caller-supplied base must be absolute (RFC-3986 §5.1's precondition), which
/// [`BaseIri::parse`] checks once here so no resolution site downstream has to.
fn base_scope(base: Option<&str>) -> Result<BaseScope, String> {
    match base {
        None => Ok(BaseScope::empty()),
        Some(text) => BaseIri::parse(text)
            .map(|iri| BaseScope::rooted(iri, BaseOrigin::Caller))
            .map_err(|error| format!("invalid base IRI {text:?}: {error}")),
    }
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
        // The import lane consumes the closure's FACTS; its report belongs to the RDFS /
        // OWL-RL run that produced them, not to the RIF rule set being assembled here, so
        // it is bound and dropped rather than folded into a claim about the rule set.
        let (closed, _report) = materialize(&source, import_plan(import.profile.as_deref()))?;
        imported_facts.clear();
        fill_dataset_facts(&closed, &mut imported_facts);
        ruleset.facts.append(&mut imported_facts);
    }
    Ok(ruleset)
}

fn parse_document(text: &str, base: &BaseScope) -> Result<ParsedRifDocument, String> {
    // RIF imports are resolved by the caller; XML DTD/entity expansion is never needed.
    let document = Document::parse(text).map_err(|error| error.to_string())?;
    let root = document.root_element();
    require(&root, "Document")?;
    // `xml:base` on the document element scopes the whole document.
    let base = enter(&root, base)?;
    let mut ruleset = RuleSet::new();
    let mut imports = Vec::new();
    for child in elements(&root) {
        let base = enter(&child, &base)?;
        match local_name(&child)? {
            "directive" => collect_import(&child, &mut imports, &base)?,
            "payload" => parse_payload(&child, &mut ruleset, &base)?,
            "meta" | "id" => {}
            other => return Err(format!("unexpected Document child <{other}>")),
        }
    }
    Ok(ParsedRifDocument { ruleset, imports })
}

fn collect_import(
    directive: &Node<'_, '_>,
    imports: &mut Vec<RifImport>,
    base: &BaseScope,
) -> Result<(), String> {
    let import = only_element(directive, "Import")?;
    let base = enter(&import, base)?;
    let mut location = None;
    let mut profile = None;
    for child in elements(&import) {
        let base = enter(&child, &base)?;
        match local_name(&child)? {
            // Both are IRI-valued: a `<location>` names the graph to fetch and a
            // `<profile>` names a W3C entailment regime, so both resolve against the base
            // in force here rather than travelling on as authored text.
            "location" => location = Some(iri_ref(&text_of(&child), &base)?),
            "profile" => profile = Some(iri_ref(&text_of(&child), &base)?),
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

fn parse_payload(
    payload: &Node<'_, '_>,
    ruleset: &mut RuleSet,
    base: &BaseScope,
) -> Result<(), String> {
    let group = only_element(payload, "Group")?;
    let base = enter(&group, base)?;
    for child in elements(&group) {
        let base = enter(&child, &base)?;
        match local_name(&child)? {
            "sentence" => parse_sentence(&child, ruleset, &base)?,
            "meta" | "id" | "behavior" => {}
            other => return Err(format!("unexpected Group child <{other}>")),
        }
    }
    Ok(())
}

fn parse_sentence(
    sentence: &Node<'_, '_>,
    ruleset: &mut RuleSet,
    base: &BaseScope,
) -> Result<(), String> {
    let inner = single_element(sentence, "sentence")?;
    let base = enter(&inner, base)?;
    match local_name(&inner)? {
        "Frame" => {
            for atom in parse_frame(&inner, &base)? {
                ruleset.push_fact(ground_fact(atom)?);
            }
            Ok(())
        }
        "Forall" => {
            ruleset.push_rule(parse_forall(&inner, &base)?);
            Ok(())
        }
        other => Err(format!("unexpected sentence body <{other}>")),
    }
}

fn parse_forall(forall: &Node<'_, '_>, base: &BaseScope) -> Result<Rule, String> {
    let mut formula = None;
    for child in elements(forall) {
        match local_name(&child)? {
            "declare" | "meta" | "id" => {}
            "formula" => formula = Some(child),
            other => return Err(format!("unexpected Forall child <{other}>")),
        }
    }
    let formula = formula.ok_or("Forall without a <formula>")?;
    let base = enter(&formula, base)?;
    let implies = single_element(&formula, "formula")?;
    require(&implies, "Implies")?;
    let base = enter(&implies, &base)?;
    let mut body = None;
    let mut head = None;
    for child in elements(&implies) {
        let base = enter(&child, &base)?;
        match local_name(&child)? {
            "if" => {
                let inner = single_element(&child, "if")?;
                let inner_base = enter(&inner, &base)?;
                body = Some(parse_conjunction(&inner, &inner_base)?);
            }
            "then" => {
                let inner = single_element(&child, "then")?;
                let inner_base = enter(&inner, &base)?;
                head = Some(parse_conjunction(&inner, &inner_base)?);
            }
            "meta" | "id" => {}
            other => return Err(format!("unexpected Implies child <{other}>")),
        }
    }
    Ok(Rule {
        body: body.ok_or("Implies without an <if>")?,
        head: head.ok_or("Implies without a <then>")?,
    })
}

fn parse_conjunction(node: &Node<'_, '_>, base: &BaseScope) -> Result<Vec<Atom>, String> {
    match local_name(node)? {
        "Frame" => parse_frame(node, base),
        "And" => {
            let mut atoms = Vec::new();
            for child in elements(node) {
                let base = enter(&child, base)?;
                match local_name(&child)? {
                    "formula" => {
                        let inner = single_element(&child, "formula")?;
                        let inner_base = enter(&inner, &base)?;
                        atoms.extend(parse_frame(&inner, &inner_base)?);
                    }
                    "meta" | "id" => {}
                    other => return Err(format!("unexpected And child <{other}>")),
                }
            }
            Ok(atoms)
        }
        other => Err(format!("unexpected conjunction node <{other}>")),
    }
}

fn parse_frame(frame: &Node<'_, '_>, base: &BaseScope) -> Result<Vec<Atom>, String> {
    require(frame, "Frame")?;
    let mut object = None;
    let mut slots = Vec::new();
    for child in elements(frame) {
        let base = enter(&child, base)?;
        match local_name(&child)? {
            "object" => {
                let inner = single_element(&child, "object")?;
                let inner_base = enter(&inner, &base)?;
                object = Some(parse_term(&inner, &inner_base)?);
            }
            "slot" => slots.push(parse_slot(&child, &base)?),
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

fn parse_slot(slot: &Node<'_, '_>, base: &BaseScope) -> Result<(RifTerm, RifTerm), String> {
    let mut children = elements(slot);
    let predicate_node = children.next().ok_or("slot without a predicate")?;
    let predicate_base = enter(&predicate_node, base)?;
    let predicate = parse_term(&predicate_node, &predicate_base)?;
    let value_node = children.next().ok_or("slot without a value")?;
    let value_base = enter(&value_node, base)?;
    let value = parse_term(&value_node, &value_base)?;
    if children.next().is_some() {
        return Err("slot with more than two children".to_owned());
    }
    Ok((predicate, value))
}

fn parse_term(node: &Node<'_, '_>, base: &BaseScope) -> Result<RifTerm, String> {
    match local_name(node)? {
        // A `Var` names a rule variable, not a resource: it is never an IRI reference.
        "Var" => Ok(RifTerm::Var(text_of(node))),
        "Const" => Ok(RifTerm::Const(parse_const(node, base)?)),
        other => Err(format!("unexpected term node <{other}>")),
    }
}

fn parse_const(node: &Node<'_, '_>, base: &BaseScope) -> Result<TermValue, String> {
    // `type` is itself IRI-valued, so it resolves too. An absolute one resolves to
    // itself, which is why the RIF/XSD comparisons below are unaffected.
    let kind = iri_ref(
        node.attribute("type")
            .ok_or("Const without a type attribute")?,
        base,
    )?;
    let value = text_of(node);
    if kind.strip_prefix(RIF_NS) == Some("iri") {
        // The whole point of the base stack: this IRI becomes a rule-head or rule-body
        // predicate, so a relative one left as authored would silently denote something
        // other than what the document says.
        Ok(TermValue::iri(iri_ref(&value, base)?))
    } else if kind.starts_with(XSD_NS) {
        Ok(TermValue::typed_literal(value, kind))
    } else if kind.strip_prefix(RIF_NS) == Some("local") {
        Err("rif:local const outside <meta> is unsupported".to_owned())
    } else {
        Err(format!("unsupported Const type {kind}"))
    }
}

/// The base in force INSIDE `element`: the enclosing base, rebound when the element
/// carries an `xml:base`.
///
/// The attribute may itself be relative, in which case it resolves against the base
/// already in force (XML Base §3, RFC-3986 §5.1.1) — so a chain of `xml:base` composes
/// down the tree. Borrowing the enclosing scope when there is no `xml:base` keeps the
/// common element free of an allocation; the rebound copy is what scopes the subtree, and
/// dropping it on the way back up is the "pop".
fn enter<'b>(
    element: &Node<'_, '_>,
    enclosing: &'b BaseScope,
) -> Result<Cow<'b, BaseScope>, String> {
    let Some(directive) = element.attribute((XML_NS, "base")) else {
        return Ok(Cow::Borrowed(enclosing));
    };
    let mut scoped = enclosing.clone();
    scoped
        .rebind(directive, BaseOrigin::Enclosing)
        .map_err(|error| format!("invalid xml:base {directive:?}: {error}"))?;
    Ok(Cow::Owned(scoped))
}

/// Resolve an IRI reference against the base in force.
///
/// RIF-XML admits relative references, so this is [`BaseScope::resolve`]. There is
/// deliberately no "no base, keep the raw text" fallthrough — that is exactly how an
/// unresolved relative IRI would become a rule predicate.
fn iri_ref(value: &str, base: &BaseScope) -> Result<String, String> {
    base.resolve(value)
        .map(|iri| iri.as_str().to_owned())
        .map_err(|error| format!("{error} [{}]", error.diagnostic_code()))
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

/// The plan an `Import` directive's graph is closed under.
///
/// Every arm is a rule-table lane, and that is the point: a RIF import is a
/// materialize-and-combine step, so an `OWL-Direct` profile folds to the OWL-RL closure
/// (a sound subset for the atomic combination facts) rather than starting a tableau with
/// no query to direct it, and a `RIF` profile — a rule set this directive does not carry —
/// folds to the identity, exactly as an absent profile does.
fn import_plan(profile: Option<&str>) -> Materialization<'static> {
    match profile.and_then(Regime::from_iri) {
        Some(Regime::OwlDirect | Regime::OwlRl) => Materialization::OwlRl,
        Some(Regime::Rdfs) => Materialization::Rdfs,
        Some(Regime::Rdf) => Materialization::Rdf,
        _ => Materialization::Simple,
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

    /// The base every relative reference in [`RIF`] is authored against.
    const DOC_BASE: &str = "https://example.org/rules/doc.rif";

    const RIF: &str = r#"<Document xmlns="http://www.w3.org/2007/rif#" xml:base="https://example.org/rules/doc.rif">
  <directive><Import><location>facts.ttl</location><profile>http://www.w3.org/ns/entailment/RDFS</profile></Import></directive>
  <payload><Group><sentence><Frame>
    <object><Const type="http://www.w3.org/2007/rif#iri">https://example.org/s</Const></object>
    <slot ordered="yes"><Const type="http://www.w3.org/2007/rif#iri">https://example.org/p</Const><Const type="http://www.w3.org/2001/XMLSchema#string">value</Const></slot>
  </Frame></sentence></Group></payload>
</Document>"#;

    #[test]
    fn parses_fact_and_leaves_import_to_caller() {
        let parsed = parse_rif_xml(RIF, None).unwrap();
        assert_eq!(parsed.ruleset.facts.len(), 1);
        // The relative `<location>` resolved against the document's `xml:base`; fetching
        // it is still the caller's job, spelling it is not.
        assert_eq!(
            parsed.imports[0].location,
            "https://example.org/rules/facts.ttl"
        );
    }

    #[test]
    fn xml_base_governs_iri_valued_content() {
        // `xml:base` on the document element, a nested `xml:base` on the payload that is
        // ITSELF relative (so it composes), and a relative `rif:iri` predicate under it.
        let text = r#"<Document xmlns="http://www.w3.org/2007/rif#" xml:base="https://example.org/rules/doc.rif">
  <payload xml:base="vocab/"><Group><sentence><Frame>
    <object><Const type="http://www.w3.org/2007/rif#iri">subject</Const></object>
    <slot ordered="yes"><Const type="http://www.w3.org/2007/rif#iri">predicate</Const><Const type="http://www.w3.org/2007/rif#iri">object</Const></slot>
  </Frame></sentence></Group></payload>
</Document>"#;
        let parsed = parse_rif_xml(text, None).unwrap();
        assert_eq!(
            parsed.ruleset.facts,
            vec![(
                TermValue::iri("https://example.org/rules/vocab/subject"),
                TermValue::iri("https://example.org/rules/vocab/predicate"),
                TermValue::iri("https://example.org/rules/vocab/object"),
            )]
        );
    }

    #[test]
    fn an_inner_xml_base_scopes_only_its_own_subtree() {
        // The `xml:base` on the FIRST sentence must not leak into the second.
        let text = r#"<Document xmlns="http://www.w3.org/2007/rif#" xml:base="https://example.org/a/">
  <payload><Group>
    <sentence xml:base="https://example.org/b/"><Frame>
      <object><Const type="http://www.w3.org/2007/rif#iri">s</Const></object>
      <slot><Const type="http://www.w3.org/2007/rif#iri">p</Const><Const type="http://www.w3.org/2007/rif#iri">o</Const></slot>
    </Frame></sentence>
    <sentence><Frame>
      <object><Const type="http://www.w3.org/2007/rif#iri">s</Const></object>
      <slot><Const type="http://www.w3.org/2007/rif#iri">p</Const><Const type="http://www.w3.org/2007/rif#iri">o</Const></slot>
    </Frame></sentence>
  </Group></payload>
</Document>"#;
        let parsed = parse_rif_xml(text, None).unwrap();
        assert_eq!(parsed.ruleset.facts.len(), 2);
        assert_eq!(
            parsed.ruleset.facts[0].0,
            TermValue::iri("https://example.org/b/s")
        );
        assert_eq!(
            parsed.ruleset.facts[1].0,
            TermValue::iri("https://example.org/a/s")
        );
    }

    #[test]
    fn the_caller_base_resolves_a_document_with_no_xml_base() {
        let text = RIF.replace(&format!(" xml:base=\"{DOC_BASE}\""), "");
        let parsed = parse_rif_xml(&text, Some(DOC_BASE)).unwrap();
        assert_eq!(
            parsed.imports[0].location,
            "https://example.org/rules/facts.ttl"
        );
    }

    #[test]
    fn an_in_document_xml_base_outranks_the_caller_base() {
        // RFC-3986 §5.1.1 beats §5.1.2: the document's own directive wins.
        let parsed = parse_rif_xml(RIF, Some("https://other.example/elsewhere/")).unwrap();
        assert_eq!(
            parsed.imports[0].location,
            "https://example.org/rules/facts.ttl"
        );
    }

    #[test]
    fn a_relative_iri_with_no_base_in_scope_is_a_hard_error() {
        let text = RIF.replace(&format!(" xml:base=\"{DOC_BASE}\""), "");
        let error = parse_rif_xml(&text, None).expect_err("no base can resolve `facts.ttl`");
        let EntailError::Parse(message) = error else {
            panic!("expected a parse error");
        };
        assert!(
            message.contains("iri-relative-no-base"),
            "the diagnostic must name the missing base, got: {message}"
        );
    }

    #[test]
    fn a_non_absolute_caller_base_is_refused_at_the_boundary() {
        let error = parse_rif_xml(RIF, Some("/not/absolute")).expect_err("a base must be absolute");
        assert!(matches!(error, EntailError::Parse(_)));
    }

    #[test]
    fn rejects_unknown_construct() {
        let text = RIF
            .replace("<Frame>", "<Atom>")
            .replace("</Frame>", "</Atom>");
        assert!(matches!(
            parse_rif_xml(&text, None),
            Err(EntailError::Parse(_))
        ));
    }

    #[test]
    fn accepts_rif_metadata_without_interpreting_it() {
        let text = RIF
            .replace("<Group>", "<Group><meta/><behavior/>")
            .replace("<Frame>", "<Frame><id/><meta/>")
            .replacen("<directive>", "<id/><directive>", 1);
        let parsed = parse_rif_xml(&text, None).unwrap();
        assert_eq!(parsed.ruleset.facts.len(), 1);
    }

    #[test]
    fn rejects_dtds() {
        let text = RIF.replacen(
            "<Document",
            "<!DOCTYPE Document [<!ENTITY x \"expanded\">]><Document",
            1,
        );
        assert!(matches!(
            parse_rif_xml(&text, None),
            Err(EntailError::Parse(_))
        ));
    }

    #[test]
    fn resolves_imports_and_merges_default_graph_facts() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/imported");
        let predicate = builder.intern_iri("https://example.org/p");
        let object = builder.intern_iri("https://example.org/o");
        builder.push_quad(subject, predicate, object, None);
        let imported = builder.freeze().unwrap();

        let parsed = parse_rif_xml(RIF, None).unwrap();
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
    fn maps_import_profiles_to_rule_table_lanes() {
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
            assert_eq!(import_plan(value.as_deref()).regime(), expected);
        }
        assert_eq!(import_plan(None).regime(), Regime::Simple);
    }
}

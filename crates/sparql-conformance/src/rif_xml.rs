// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reader for the normative RIF-in-XML (RIF XML) syntax used by the W3C SPARQL
//! RIF-entailment cases (`rif01`/`rif03`/`rif04`/`rif06`).
//!
//! A RIF document is `Document > payload > Group > sentence*`. Each sentence is
//! either a ground `Frame` (a fact) or a `Forall > declare* + formula > Implies`
//! (a Horn rule). A `Frame` `o[p->v ; …]` is one triple per slot, all sharing the
//! object `o`. `Const type="&rif;iri"` is an IRI; `Const type="&xs;*"` is a typed
//! literal; `Var` is a rule variable. `directive > Import > location` pulls in an
//! external RDF graph, resolved to a **local** vendored fixture by basename (never
//! over the network). `<meta>` (a `&rif;local`-typed annotation) is ignored.
//!
//! Only the monotonic definite-Horn fragment these cases use is accepted; any
//! unrecognized element is a hard error (no silent skip), keeping the reader
//! honest about what it actually understands.

use std::path::{Path, PathBuf};

use purrdf_core::{RdfDataset, TermValue};
use purrdf_entail::{Atom, Regime, RifTerm, Rule, RuleSet};
use roxmltree::{Document, Node, ParsingOptions};

/// An `Import` directive: the external RDF graph's location and its entailment
/// profile (which governs how the imported graph is combined — the combination is
/// forward-materialized before its triples seed the RIF rule set).
struct Import {
    location: String,
    profile: Option<String>,
}

/// The RIF namespace every RIF-XML element and the `iri`/`local` const types live
/// in.
const RIF_NS: &str = "http://www.w3.org/2007/rif#";
/// The XML Schema datatype namespace RIF typed-literal consts reference.
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// Load a `.rif` document at `rif_path`, plus every RDF graph it `Import`s
/// (resolved to local fixtures beside it), into one combined [`RuleSet`].
///
/// # Errors
///
/// Returns a message on any read, XML-parse, unrecognized-construct, or
/// import-resolution failure (never silent).
pub fn load_ruleset(rif_path: &Path) -> Result<RuleSet, String> {
    let text = std::fs::read_to_string(rif_path)
        .map_err(|e| format!("read rif {}: {e}", rif_path.display()))?;
    let (mut ruleset, imports) =
        parse_document(&text).map_err(|e| format!("parse rif {}: {e}", rif_path.display()))?;

    let dir = rif_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    for import in &imports {
        let facts = load_import(&dir, import)?;
        ruleset.facts.extend(facts);
    }
    Ok(ruleset)
}

/// Parse a RIF-XML document string into a [`RuleSet`] and the list of `Import`
/// directives (still to be resolved to local files by the caller).
fn parse_document(text: &str) -> Result<(RuleSet, Vec<Import>), String> {
    // The RIF fixtures declare an internal DTD (entity table for the `rif`/`xs`/…
    // prefixes), so DTD parsing must be allowed; roxmltree then expands the entity
    // references in both element text and attribute values.
    let opts = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc = Document::parse_with_options(text, opts).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    require(&root, "Document")?;

    let mut ruleset = RuleSet::new();
    let mut imports = Vec::new();

    for child in elements(&root) {
        match local_name(&child)? {
            "directive" => collect_import(&child, &mut imports)?,
            "payload" => parse_payload(&child, &mut ruleset)?,
            other => return Err(format!("unexpected Document child <{other}>")),
        }
    }
    Ok((ruleset, imports))
}

/// Read a `directive > Import > location + [profile]` into `imports`.
fn collect_import(directive: &Node<'_, '_>, imports: &mut Vec<Import>) -> Result<(), String> {
    let import = only_element(directive, "Import")?;
    let mut location: Option<String> = None;
    let mut profile: Option<String> = None;
    for child in elements(&import) {
        match local_name(&child)? {
            "location" => location = Some(text_of(&child)),
            "profile" => profile = Some(text_of(&child)),
            other => return Err(format!("unexpected Import child <{other}>")),
        }
    }
    imports.push(Import {
        location: location.ok_or("Import without a <location>")?,
        profile,
    });
    Ok(())
}

/// Parse `payload > Group`, folding each sentence into `ruleset`.
fn parse_payload(payload: &Node<'_, '_>, ruleset: &mut RuleSet) -> Result<(), String> {
    let group = only_element(payload, "Group")?;
    for child in elements(&group) {
        match local_name(&child)? {
            "sentence" => parse_sentence(&child, ruleset)?,
            // A nested Group is legal RIF; these cases never use one, so reject it
            // loudly rather than silently descend.
            other => return Err(format!("unexpected Group child <{other}>")),
        }
    }
    Ok(())
}

/// Parse one `sentence`: either a ground `Frame` fact or a `Forall` rule.
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

/// Parse a `Forall > declare* + [meta] + formula > Implies` into a Horn [`Rule`].
fn parse_forall(forall: &Node<'_, '_>) -> Result<Rule, String> {
    let mut formula: Option<Node<'_, '_>> = None;
    for child in elements(forall) {
        match local_name(&child)? {
            "declare" | "meta" => {} // variable declarations and annotations: ignored
            "formula" => formula = Some(child),
            other => return Err(format!("unexpected Forall child <{other}>")),
        }
    }
    let formula = formula.ok_or("Forall without a <formula>")?;
    let implies = single_element(&formula, "formula")?;
    require(&implies, "Implies")?;

    let mut body: Option<Vec<Atom>> = None;
    let mut head: Option<Vec<Atom>> = None;
    for child in elements(&implies) {
        match local_name(&child)? {
            "if" => body = Some(parse_conjunction(&single_element(&child, "if")?)?),
            "then" => head = Some(parse_conjunction(&single_element(&child, "then")?)?),
            other => return Err(format!("unexpected Implies child <{other}>")),
        }
    }
    Ok(Rule {
        body: body.ok_or("Implies without an <if>")?,
        head: head.ok_or("Implies without a <then>")?,
    })
}

/// Parse an `if`/`then` body node — either a single `Frame` or an `And > formula*
/// > Frame` — into a flat list of triple-pattern atoms.
fn parse_conjunction(node: &Node<'_, '_>) -> Result<Vec<Atom>, String> {
    match local_name(node)? {
        "Frame" => parse_frame(node),
        "And" => {
            let mut atoms = Vec::new();
            for child in elements(node) {
                match local_name(&child)? {
                    "formula" => {
                        let frame = single_element(&child, "formula")?;
                        atoms.extend(parse_frame(&frame)?);
                    }
                    other => return Err(format!("unexpected And child <{other}>")),
                }
            }
            Ok(atoms)
        }
        other => Err(format!("unexpected conjunction node <{other}>")),
    }
}

/// Parse a `Frame` `object + slot*` into one atom per slot (all sharing the
/// object as subject).
fn parse_frame(frame: &Node<'_, '_>) -> Result<Vec<Atom>, String> {
    require(frame, "Frame")?;
    let mut object: Option<RifTerm> = None;
    let mut slots: Vec<(RifTerm, RifTerm)> = Vec::new();
    for child in elements(frame) {
        match local_name(&child)? {
            "object" => object = Some(parse_term(&single_element(&child, "object")?)?),
            "slot" => slots.push(parse_slot(&child)?),
            other => return Err(format!("unexpected Frame child <{other}>")),
        }
    }
    let subject = object.ok_or("Frame without an <object>")?;
    if slots.is_empty() {
        return Err("Frame without any <slot>".to_owned());
    }
    Ok(slots
        .into_iter()
        .map(|(p, o)| Atom {
            s: subject.clone(),
            p,
            o,
        })
        .collect())
}

/// Parse a `slot ordered="yes"` — a predicate const followed by a value term.
fn parse_slot(slot: &Node<'_, '_>) -> Result<(RifTerm, RifTerm), String> {
    let mut kids = elements(slot);
    let predicate = parse_term(&kids.next().ok_or("slot without a predicate")?)?;
    let value = parse_term(&kids.next().ok_or("slot without a value")?)?;
    if kids.next().is_some() {
        return Err("slot with more than two children".to_owned());
    }
    Ok((predicate, value))
}

/// Parse a `Var` or `Const` into a [`RifTerm`].
fn parse_term(node: &Node<'_, '_>) -> Result<RifTerm, String> {
    match local_name(node)? {
        "Var" => Ok(RifTerm::Var(text_of(node))),
        "Const" => Ok(RifTerm::Const(parse_const(node)?)),
        other => Err(format!("unexpected term node <{other}>")),
    }
}

/// Parse a `Const type="…"` into its [`TermValue`]. An `&rif;iri` const is an IRI;
/// an `&xs;*` const is a typed literal; `&rif;local` only appears inside ignored
/// `<meta>`, so seeing it here is an error.
fn parse_const(node: &Node<'_, '_>) -> Result<TermValue, String> {
    let ty = node
        .attribute("type")
        .ok_or("Const without a type attribute")?;
    let value = text_of(node);
    if ty == format!("{RIF_NS}iri") {
        Ok(TermValue::iri(value))
    } else if ty.starts_with(XSD_NS) {
        Ok(TermValue::typed_literal(value, ty))
    } else if ty == format!("{RIF_NS}local") {
        Err("rif:local const outside <meta> is unsupported".to_owned())
    } else {
        Err(format!("unsupported Const type {ty}"))
    }
}

/// Convert a fully-ground atom into a [`Fact`](purrdf_entail::Fact) triple; a
/// variable in a fact position is an error.
fn ground_fact(atom: Atom) -> Result<purrdf_entail::Fact, String> {
    Ok((const_of(atom.s)?, const_of(atom.p)?, const_of(atom.o)?))
}

/// The ground term behind a [`RifTerm`], or an error if it is a variable.
fn const_of(term: RifTerm) -> Result<TermValue, String> {
    match term {
        RifTerm::Const(v) => Ok(v),
        RifTerm::Var(name) => Err(format!("variable ?{name} in a ground fact")),
    }
}

/// Resolve an `Import` to a local fixture (by basename, in `dir`), parse it as
/// RDF, forward-materialize it under the directive's profile, and return the
/// resulting default-graph triples as facts.
///
/// The URL basename maps to a vendored file beside the `.rif`. A `.rdf` basename
/// is RDF/XML; anything else (extensionless / N-Triples) is parsed as N-Triples.
/// Network fetches are forbidden — a missing local fixture is a hard error.
///
/// The `Import` profile governs the imported graph's own entailment: a graph
/// imported `OWL-Direct`/`OWL-RL` is closed under the forward-materializable
/// OWL-RL rule set (a sound subset of OWL Direct-Semantics for the atomic facts
/// the RIF rules join against — e.g. the `rdfs:subClassOf`/`rdfs:domain` typing
/// the brain-anatomy rule requires); `RDFS`/`RDF` close under their regimes; every
/// other profile is the identity closure (no fabricated facts).
fn load_import(dir: &Path, import: &Import) -> Result<Vec<purrdf_entail::Fact>, String> {
    let location = &import.location;
    let basename = location
        .rsplit(['/', '#'])
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("import location {location} has no basename"))?;
    let path = dir.join(basename);
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("read import {} (from {location}): {e}", path.display()))?;
    let is_rdfxml = path.extension().and_then(|e| e.to_str()) == Some("rdf");
    let media_type = if is_rdfxml {
        "application/rdf+xml"
    } else {
        "application/n-triples"
    };
    // The vendored RDF/XML imports carry an internal DTD (an entity table for the
    // `owl`/`xsd`/… prefixes); the native RDF/XML codec rejects any DTD, so expand
    // the internal general entities and strip the DOCTYPE here first. N-Triples has
    // no DTD and passes through untouched.
    let bytes: Vec<u8> = if is_rdfxml {
        let text = std::str::from_utf8(&bytes)
            .map_err(|e| format!("utf-8 in import {}: {e}", path.display()))?;
        expand_internal_entities(text).into_bytes()
    } else {
        bytes
    };
    let ds = purrdf::parse_dataset(&bytes, media_type, Some(location))
        .map_err(|e| format!("parse import {}: {e}", path.display()))?;
    let closed = purrdf_entail::materialize(&ds, import_regime(import.profile.as_deref()))
        .map_err(|e| format!("close import {}: {e}", path.display()))?;
    Ok(dataset_facts(&closed))
}

/// Map an `Import` profile IRI to the forward-materializable regime under which
/// its graph is closed. `OWL-Direct` folds to the OWL-RL closure (query-directed
/// DL is not a materialize-and-combine affair, and OWL-RL is a sound subset for
/// the atomic combination facts); an unrecognized/absent profile is the identity.
fn import_regime(profile: Option<&str>) -> Regime {
    match profile.and_then(Regime::from_iri) {
        Some(Regime::OwlDirect | Regime::OwlRl) => Regime::OwlRl,
        Some(Regime::Rdfs) => Regime::Rdfs,
        Some(Regime::Rdf) => Regime::Rdf,
        _ => Regime::Simple,
    }
}

/// Every default-graph triple of `ds` as a [`Fact`](purrdf_entail::Fact).
fn dataset_facts(ds: &RdfDataset) -> Vec<purrdf_entail::Fact> {
    ds.quads()
        .filter(|q| q.g.is_none())
        .map(|q| (ds.term_value(q.s), ds.term_value(q.p), ds.term_value(q.o)))
        .collect()
}

/// Expand an XML document's internal-DTD general entities and remove its
/// `<!DOCTYPE …>` declaration, yielding DTD-free XML the native RDF/XML codec
/// accepts.
///
/// Only entities declared in the internal subset (`<!ENTITY name "value">`) are
/// expanded; the five predefined XML entities (`&amp;` etc.) are left for the XML
/// parser. This is a targeted preprocessing step for the vendored RIF import
/// fixtures, not a general DTD processor (no external subsets, parameter entities,
/// or recursive expansion).
fn expand_internal_entities(text: &str) -> String {
    let Some(dt_start) = text.find("<!DOCTYPE") else {
        return text.to_owned();
    };
    // The DOCTYPE ends at the first '>' after its internal subset `[ … ]` (if any).
    let bracket = text[dt_start..].find('[').map(|i| dt_start + i);
    let (entities, doctype_end) = if let Some(lb) = bracket {
        let rb = text[lb..].find(']').map_or(text.len(), |i| lb + i);
        let subset = &text[lb + 1..rb.min(text.len())];
        let end = text[rb.min(text.len())..]
            .find('>')
            .map_or(text.len(), |i| rb + i + 1);
        (scan_entities(subset), end)
    } else {
        let end = text[dt_start..]
            .find('>')
            .map_or(text.len(), |i| dt_start + i + 1);
        (Vec::new(), end)
    };

    let mut body = String::with_capacity(text.len());
    body.push_str(&text[..dt_start]);
    body.push_str(&text[doctype_end..]);
    for (name, value) in entities {
        body = body.replace(&format!("&{name};"), &value);
    }
    body
}

/// Parse `<!ENTITY name "value">` declarations out of a DTD internal subset,
/// returning `(name, value)` pairs in declaration order.
fn scan_entities(subset: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = subset;
    while let Some(pos) = rest.find("<!ENTITY") {
        rest = &rest[pos + "<!ENTITY".len()..];
        let after_name = rest.trim_start();
        let name_end = after_name
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after_name.len());
        let name = after_name[..name_end].trim().to_owned();
        let after = &after_name[name_end..];
        // The value is delimited by the next quote character (either " or ').
        let Some(q_off) = after.find(['"', '\'']) else {
            break;
        };
        let quote = after.as_bytes()[q_off] as char;
        let value_start = &after[q_off + 1..];
        let Some(v_end) = value_start.find(quote) else {
            break;
        };
        let value = value_start[..v_end].to_owned();
        if !name.is_empty() {
            out.push((name, value));
        }
        rest = &value_start[v_end + 1..];
    }
    out
}

// --- roxmltree helpers -----------------------------------------------------

/// The element children of `node` (skipping text, comments, PIs).
fn elements<'a, 'input>(
    node: &Node<'a, 'input>,
) -> impl Iterator<Item = Node<'a, 'input>> + use<'a, 'input> {
    node.children().filter(Node::is_element)
}

/// The local name of an element, requiring it to be in the RIF namespace.
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

/// Assert `node` is a RIF element with local `name`.
fn require(node: &Node<'_, '_>, name: &str) -> Result<(), String> {
    let got = local_name(node)?;
    if got == name {
        Ok(())
    } else {
        Err(format!("expected <{name}>, found <{got}>"))
    }
}

/// The single element child of `node`, erroring if there is not exactly one.
/// `parent` names the caller for diagnostics.
fn single_element<'a, 'input>(
    node: &Node<'a, 'input>,
    parent: &str,
) -> Result<Node<'a, 'input>, String> {
    let mut it = elements(node);
    let first = it.next().ok_or_else(|| format!("<{parent}> is empty"))?;
    if it.next().is_some() {
        return Err(format!("<{parent}> has more than one child element"));
    }
    Ok(first)
}

/// The single `name`-typed element child of `node`.
fn only_element<'a, 'input>(
    node: &Node<'a, 'input>,
    name: &str,
) -> Result<Node<'a, 'input>, String> {
    let child = single_element(node, name)?;
    require(&child, name)?;
    Ok(child)
}

/// The concatenated, trimmed text content of an element (entity references are
/// already expanded by roxmltree).
fn text_of(node: &Node<'_, '_>) -> String {
    let mut s = String::new();
    for child in node.children() {
        if let Some(t) = child.text() {
            s.push_str(t);
        }
    }
    s.trim().to_owned()
}

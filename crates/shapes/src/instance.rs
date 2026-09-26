// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON-LD `@graph` projector for PurRDF instance data.
//!
//! Walks the default graph of a dataset and emits a JSON-LD document
//! `{ "@context": {<prefix map>}, "@graph": [ <node objects> ] }`. The projection
//! is the matched pair of the JSON Schema produced by [`crate::json_schema`]: a
//! projected node always validates against the schema the emitter derives from
//! the same shapes when the node conforms to them.
//!
//! The projection drops nothing: every triple of the default graph is carried,
//! and every carried term keeps what a constraint can judge — an IRI's full
//! string, a literal's lexical form, datatype, language tag and base direction.
//!
//! # Value conventions (lock-step with `json_schema.rs`)
//!
//! * **IRI** — `{"@id": "<full IRI>"}`. The `@id` is never compacted, so a
//!   lexical-form constraint (`sh:pattern`, `sh:minLength`, `sh:maxLength`)
//!   judges `str()` of the IRI directly, and no `@id` is ambiguous between a
//!   compact IRI and an absolute IRI whose scheme happens to be a declared
//!   prefix. Node `@id`s follow the same rule; `@type` values and property keys
//!   stay compacted.
//! * **Blank node** — `{"@id": "_:<label>"}`.
//! * **`rdf:type`** — folded into the node's `@type` (a CURIE string, or an array
//!   of CURIE strings when there are several).
//! * **RDF list** — a well-formed list is the JSON-LD list object
//!   `{"@list": [ <members> ]}` (JSON-LD 1.1 §4.3.2), members projected
//!   recursively and in list order, and its cells are not `@graph` nodes;
//!   `rdf:nil` is `{"@list": []}`. A list that is a member of a list also
//!   carries its head cell's label as `@index` (see `project_list_or_value`). Which lists convert is decided exactly by
//!   the list conversion of JSON-LD 1.1 Processing Algorithms and API §8.4.2
//!   (Serialize RDF as JSON-LD, step 6.4) — see [`ListIndex`] — so a branching,
//!   cyclic, shared, IRI-named or annotated list keeps the node-graph form.
//! * **Typed literal** — `{"@value": "<lexical>", "@type": "<compacted-datatype>"}`,
//!   except for the two literal kinds a JSON scalar denotes without loss
//!   (JSON-LD 1.1 §8.6, Object to RDF Conversion, maps them back to exactly the
//!   same literal): an `xsd:boolean` whose lexical form is `true` or `false` is
//!   a bare JSON boolean, and an `xsd:integer` whose lexical form is canonical
//!   (`0`, or an optional `-` and digits without a leading zero) and within 64
//!   bits is a bare JSON integer. Every other numeric literal — `xsd:int`,
//!   `xsd:decimal`, `xsd:double`, a non-canonical `xsd:integer` such as `+5` or
//!   `007` — keeps its typed-literal object, so its datatype and lexical form
//!   stay judgeable. An `xsd:string` literal is a bare string.
//! * **Language-tagged literal** — `{"@value": "<lexical>", "@language": "<tag>"}`,
//!   with `"@direction": "ltr"` or `"rtl"` (JSON-LD 1.1 §4.2.4.1) for an
//!   `rdf:dirLangString` literal.
//! * **Triple term** (RDF 1.2) — the JSON-LD-star embedded node
//!   `{"@id": {"@id": <subject>, "<predicate>": <object>}}`. A triple term is
//!   not part of the graph's list structure, so `rdf:nil` inside one stays a
//!   node reference.
//!
//! Multi-valued predicates project as a JSON array; single-valued as the scalar.
//! Subjects, predicate keys, and array members are sorted for determinism; list
//! members keep their list order.

use std::collections::BTreeMap;

use ::purrdf::{FastMap, FastSet, RdfDataset, TermId, TermRef};
use serde_json::{Map, Value, json};

use crate::data::{GraphFilter, native_quads, quads_for_pattern_ids, resolve_id};
use crate::json_schema::Namespaces;
use crate::model::rdf;
use crate::term::{Term, term_id_to_native};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// Project the default graph of `dataset` into a JSON-LD `@graph` document.
///
/// `ns` is the SAME caller-supplied [`Namespaces`] the matching schema was
/// compiled with ([`crate::json_schema::compile`]) — it drives every CURIE
/// compaction of a key or `@type` and the emitted `@context`:
///
/// ```text
/// let ns = Namespaces::new("gmeow", &doc_prefixes)?;
/// let doc = project_graph(&dataset, &ns);
/// ```
pub fn project_graph(dataset: &std::sync::Arc<RdfDataset>, ns: &Namespaces) -> Value {
    project_graph_data(dataset.as_ref(), ns)
}

/// Project the default graph of a frozen [`RdfDataset`] into a JSON-LD `@graph`
/// document.
fn project_graph_data(data: &RdfDataset, ns: &Namespaces) -> Value {
    let lists = ListIndex::build(data);
    // Collect distinct named-node / blank-node subjects of the default graph.
    let mut subjects: Vec<Term> = Vec::new();
    let mut seen: FastSet<String> = FastSet::default();
    // Scope to the DEFAULT graph only: an AnyGraph filter would match named graphs
    // too, leaking named-graph subjects into the projected `@graph`.
    for (subject, _pred, _object) in native_quads(data, None, None, None, GraphFilter::DefaultGraph)
    {
        // Only IRI / blank-node subjects become @graph nodes (always true here),
        // and a list cell carried by its list's `@list` is not a node of its own.
        if subject.is_subject() && !lists.is_cell(data, &subject) {
            let key = subject.to_string();
            if seen.insert(key) {
                subjects.push(subject);
            }
        }
    }
    crate::term::sort_terms_canonical(&mut subjects);

    let mut nodes: Vec<Value> = Vec::with_capacity(subjects.len());
    for subj in &subjects {
        nodes.push(project_subject_data(data, ns, &lists, subj));
    }

    json!({
        "@context": Value::Object(ns.context_object()),
        "@graph": Value::Array(nodes),
    })
}

/// Project a single subject term into a JSON-LD node object, compacting keys
/// and `@type` values through the caller-supplied [`Namespaces`].
pub fn project_subject(
    dataset: &std::sync::Arc<RdfDataset>,
    ns: &Namespaces,
    subject: &Term,
) -> Value {
    let data = dataset.as_ref();
    project_subject_data(data, ns, &ListIndex::build(data), subject)
}

fn project_subject_data(
    data: &RdfDataset,
    ns: &Namespaces,
    lists: &ListIndex,
    subject: &Term,
) -> Value {
    if !subject.is_subject() {
        // Literals (and quoted triples) are never node subjects.
        return Value::Object(Map::new());
    }

    // Gather predicate → [objects], grouping by compacted predicate key.
    let mut by_pred: BTreeMap<String, Vec<Term>> = BTreeMap::new();
    let mut types: Vec<String> = Vec::new();

    for (_subject, pred, object) in
        native_quads(data, Some(subject), None, None, GraphFilter::DefaultGraph)
    {
        if pred.as_str() == rdf::TYPE
            && let Term::NamedNode(n) = &object
        {
            types.push(ns.compact_iri(n.as_str()));
            continue;
        }
        by_pred
            .entry(ns.compact_iri(pred.as_str()))
            .or_default()
            .push(object);
    }

    let mut obj: Map<String, Value> = Map::new();

    // @id
    let id = match subject {
        Term::NamedNode(n) => n.as_str().to_owned(),
        Term::BlankNode(b) => format!("_:{b}"),
        _ => String::new(),
    };
    obj.insert("@id".to_owned(), Value::String(id));

    // @type (string or array; sorted/deduped)
    if !types.is_empty() {
        types.sort();
        types.dedup();
        if types.len() == 1 {
            obj.insert("@type".to_owned(), Value::String(types.remove(0)));
        } else {
            obj.insert(
                "@type".to_owned(),
                Value::Array(types.into_iter().map(Value::String).collect()),
            );
        }
    }

    for (key, mut objects) in by_pred {
        crate::term::sort_terms_canonical(&mut objects);
        let mut values: Vec<Value> = objects
            .iter()
            .map(|t| project_object(data, ns, lists, t))
            .collect();
        let v = if values.len() == 1 {
            values.pop().expect("one value")
        } else {
            Value::Array(values)
        };
        obj.insert(key, v);
    }

    Value::Object(obj)
}

/// Project an object term of the default graph: the `@list` object when it is
/// the head of a list [`ListIndex`] converts, else [`project_value`].
fn project_object(data: &RdfDataset, ns: &Namespaces, lists: &ListIndex, term: &Term) -> Value {
    project_list_or_value(data, ns, lists, term, false)
}

/// [`project_object`]; `member` marks a member of a list.
///
/// A converted list that is itself a member of a list carries its head cell's
/// blank-node label as `@index`. JSON-LD 1.1 §4.3.2 admits `@index` on a list
/// object, and it is not RDF-significant (Object to RDF Conversion ignores
/// it), so the graph the document states is unchanged; but two distinct member
/// lists with equal members stay distinct JSON values, as they are distinct
/// nodes — which is what `sh:uniqueMembers`, projected as `uniqueItems`,
/// compares.
fn project_list_or_value(
    data: &RdfDataset,
    ns: &Namespaces,
    lists: &ListIndex,
    term: &Term,
    member: bool,
) -> Value {
    if let Some(members) = lists.members(data, term) {
        let members: Vec<Value> = members
            .into_iter()
            .map(|item| {
                project_list_or_value(data, ns, lists, &term_id_to_native(data, item), true)
            })
            .collect();
        let mut list = Map::new();
        list.insert("@list".to_owned(), Value::Array(members));
        if member && let Term::BlankNode(label) = term {
            list.insert("@index".to_owned(), Value::String(format!("_:{label}")));
        }
        return Value::Object(list);
    }
    project_value(term, ns)
}

/// Project a single object term into its JSON-LD value form.
///
/// MUST match the value-schema convention in [`crate::json_schema`]. It is the
/// SINGLE source of the value encoding: the schema emitter's `sh:in` enum members
/// delegate here (`crate::json_schema::term_enum_value`) so a projected value and
/// its enum member can never drift. `rdf:nil` is the empty list `{"@list": []}`;
/// a non-empty list is projected by the dataset-aware caller, since its members
/// live in the data graph.
pub(crate) fn project_value(term: &Term, ns: &Namespaces) -> Value {
    match term {
        Term::NamedNode(n) if n.as_str() == RDF_NIL => json!({ "@list": [] }),
        _ => project_term(term, ns),
    }
}

/// A term's value form outside the graph's list structure (inside a triple
/// term, `rdf:nil` is a node like any other).
fn project_term(term: &Term, ns: &Namespaces) -> Value {
    match term {
        Term::NamedNode(n) => json!({ "@id": n.as_str() }),
        Term::BlankNode(b) => json!({ "@id": format!("_:{}", b.as_str()) }),
        Term::Literal(lit) => {
            if let Some(lang) = lit.language() {
                let mut object = Map::new();
                object.insert("@value".to_owned(), Value::String(lit.value().to_owned()));
                object.insert("@language".to_owned(), Value::String(lang.to_owned()));
                if let Some(direction) = lit.direction() {
                    object.insert(
                        "@direction".to_owned(),
                        Value::String(direction.as_str().to_owned()),
                    );
                }
                return Value::Object(object);
            }
            let dt_iri = lit.datatype_str();
            // Plain string / langString without a tag → bare string.
            if dt_iri == XSD_STRING || dt_iri == RDF_LANG_STRING {
                return Value::String(lit.value().to_owned());
            }
            // A literal a JSON scalar denotes exactly → the bare scalar.
            if let Some(scalar) = native_scalar(dt_iri, lit.value()) {
                return scalar;
            }
            // Every other typed literal → the {"@value","@type"} object form.
            json!({ "@value": lit.value(), "@type": ns.compact_iri(dt_iri) })
        }
        // An RDF 1.2 triple term is the JSON-LD-star embedded node: an `@id`
        // whose value is the node object stating the one triple. It is neither a
        // string nor a node reference, so no schema confuses it with a literal
        // or a named node. (Statement-layer reifiers are projected via
        // `@annotation`, not as plain object values.)
        Term::Triple(triple) => {
            let subject = match project_term(&triple.subject, ns) {
                Value::Object(mut reference) => reference.remove("@id").unwrap_or(Value::Null),
                other => other,
            };
            let mut embedded = Map::new();
            embedded.insert("@id".to_owned(), subject);
            embedded.insert(
                ns.compact_iri(triple.predicate.as_str()),
                project_term(&triple.object, ns),
            );
            json!({ "@id": Value::Object(embedded) })
        }
    }
}

/// The bare JSON scalar of a literal a JSON scalar denotes without loss, or
/// `None` (so the caller keeps the `{"@value","@type"}` object form).
///
/// JSON-LD 1.1 §8.6 (Object to RDF Conversion) maps a JSON boolean to the
/// `xsd:boolean` literal `true` or `false`, and a JSON number without a
/// fractional part to the `xsd:integer` literal in canonical form. Exactly those
/// literals — and no others — are projected as the scalar, so the scalar
/// determines the datatype and the lexical form. A JSON number with a fraction
/// maps to an `xsd:double` in a canonical form (`1.5E0`) data seldom uses, and
/// would read back as an integer when its value is whole, so no double is a
/// bare number.
fn native_scalar(dt_iri: &str, lexical: &str) -> Option<Value> {
    match dt_iri {
        XSD_BOOLEAN => match lexical {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        XSD_INTEGER if is_canonical_integer(lexical) => lexical
            .parse::<i64>()
            .ok()
            .map(Value::from)
            .or_else(|| lexical.parse::<u64>().ok().map(Value::from)),
        _ => None,
    }
}

/// Whether `lexical` is the canonical `xsd:integer` form: `0`, or an optional
/// `-` followed by a non-zero digit and further digits.
pub(crate) fn is_canonical_integer(lexical: &str) -> bool {
    let digits = lexical.strip_prefix('-').unwrap_or(lexical);
    match digits.as_bytes() {
        [b'0'] => lexical.len() == 1,
        [b'1'..=b'9', rest @ ..] => rest.iter().all(u8::is_ascii_digit),
        _ => false,
    }
}

/// The RDF lists of the default graph the projection carries as `@list`
/// objects, and so the list cells it does not project as `@graph` nodes.
///
/// JSON-LD 1.1 Processing Algorithms and API §8.4.2, step 6.4, walks each
/// `rdf:rest rdf:nil` usage back towards the list's head while the cell `node`
/// "represents a well-formed list node": its `@id` is a blank node identifier,
/// it is *referenced once* (step 6.2 records a blank node's single usage as an
/// object, and marks one used twice), it "has rdf:first and rdf:rest entries,
/// both of which have as value an array consisting of a single element, and
/// node has no other entries" — and the walk stops at the first cell that is
/// not, or whose usage is not an `rdf:rest`; the cells it passed become the
/// `@list` of the value that referenced the last of them.
///
/// Two readings are fixed here, both keeping a triple the algorithm would
/// otherwise lose. The algorithm tolerates "an optional @type entry whose value
/// is an array with a single item equal to rdf:List" and drops that triple; a
/// cell stating `rdf:type rdf:List` is treated as having another entry, so it
/// stays a node. And a blank node is referenced once only when its one
/// reference is a default-graph object: a reference from another graph, as a
/// graph name, or inside a triple term counts as a second one, since the cell's
/// label would otherwise dangle there.
///
/// Built only when some `rdf:rest` of the default graph is `rdf:nil`; for data
/// without lists it allocates nothing.
pub(crate) struct ListIndex {
    /// The cells carried inside some `@list`.
    cells: FastSet<TermId>,
    first: Option<TermId>,
    rest: Option<TermId>,
    nil: Option<TermId>,
}

/// How often, and from where, a blank node is referenced.
#[derive(Clone, Copy)]
enum Usage {
    /// Exactly one reference: a default-graph object, as `(subject, predicate)`.
    Once(TermId, TermId),
    /// Any other number or kind of reference.
    Other,
}

impl ListIndex {
    /// No list converts (an empty hash set does not allocate).
    fn empty() -> Self {
        Self {
            cells: FastSet::default(),
            first: None,
            rest: None,
            nil: None,
        }
    }

    pub(crate) fn build(data: &RdfDataset) -> Self {
        let (Some(first), Some(rest), Some(nil)) = (
            data.term_id_by_iri(rdf::FIRST),
            data.term_id_by_iri(rdf::REST),
            data.term_id_by_iri(RDF_NIL),
        ) else {
            return Self::empty();
        };
        let tails: Vec<TermId> =
            quads_for_pattern_ids(data, None, Some(rest), Some(nil), GraphFilter::DefaultGraph)
                .map(|quad| quad.s)
                .collect();
        if tails.is_empty() {
            return Self::empty();
        }
        let is_blank = |id: TermId| matches!(data.resolve(id), TermRef::Blank { .. });
        let mut usages: FastMap<TermId, Usage> = FastMap::default();
        let note = |id: TermId, usage: Usage, usages: &mut FastMap<TermId, Usage>| {
            usages
                .entry(id)
                .and_modify(|seen| *seen = Usage::Other)
                .or_insert(usage);
        };
        for quad in quads_for_pattern_ids(data, None, None, None, GraphFilter::AnyGraph) {
            if is_blank(quad.o) {
                let usage = if quad.g.is_none() {
                    Usage::Once(quad.s, quad.p)
                } else {
                    Usage::Other
                };
                note(quad.o, usage, &mut usages);
            } else if matches!(data.resolve(quad.o), TermRef::Triple { .. }) {
                let mut pending = vec![quad.o];
                while let Some(term) = pending.pop() {
                    if let TermRef::Triple { s, o, .. } = data.resolve(term) {
                        pending.extend([s, o]);
                    } else if is_blank(term) {
                        note(term, Usage::Other, &mut usages);
                        note(term, Usage::Other, &mut usages);
                    }
                }
            }
            if let Some(graph) = quad.g
                && is_blank(graph)
            {
                note(graph, Usage::Other, &mut usages);
                note(graph, Usage::Other, &mut usages);
            }
        }
        // A well-formed list node: blank, referenced once from the default
        // graph, and the subject of exactly one `rdf:first` and one `rdf:rest`
        // there and of nothing else.
        let well_formed = |cell: TermId| -> Option<(TermId, TermId)> {
            if !is_blank(cell) {
                return None;
            }
            let Some(&Usage::Once(subject, predicate)) = usages.get(&cell) else {
                return None;
            };
            let mut firsts = 0_u8;
            let mut rests = 0_u8;
            for quad in
                quads_for_pattern_ids(data, Some(cell), None, None, GraphFilter::DefaultGraph)
            {
                if quad.p == first {
                    firsts += 1;
                } else if quad.p == rest {
                    rests += 1;
                } else {
                    return None;
                }
            }
            (firsts == 1 && rests == 1).then_some((subject, predicate))
        };
        let mut cells: FastSet<TermId> = FastSet::default();
        for tail in tails {
            let mut node = tail;
            while !cells.contains(&node)
                && let Some((subject, predicate)) = well_formed(node)
            {
                cells.insert(node);
                if predicate != rest {
                    break;
                }
                node = subject;
            }
        }
        Self {
            cells,
            first: Some(first),
            rest: Some(rest),
            nil: Some(nil),
        }
    }

    /// Whether `term` is a list cell carried inside a `@list`.
    fn is_cell(&self, data: &RdfDataset, term: &Term) -> bool {
        !self.cells.is_empty()
            && matches!(term, Term::BlankNode(_))
            && resolve_id(data, term).is_some_and(|id| self.cells.contains(&id))
    }

    /// The members, in order, of the list whose head is `term`, when `term` is a
    /// converted list's head; `None` otherwise (including for `rdf:nil`, which
    /// [`project_value`] projects on its own).
    fn members(&self, data: &RdfDataset, term: &Term) -> Option<Vec<TermId>> {
        if !self.is_cell(data, term) {
            return None;
        }
        let (first, rest, nil) = (self.first?, self.rest?, self.nil?);
        let mut cell = resolve_id(data, term)?;
        let mut members = Vec::new();
        while cell != nil && members.len() <= self.cells.len() {
            let single = |predicate: TermId| {
                quads_for_pattern_ids(
                    data,
                    Some(cell),
                    Some(predicate),
                    None,
                    GraphFilter::DefaultGraph,
                )
                .next()
                .map(|quad| quad.o)
            };
            members.push(single(first)?);
            cell = single(rest)?;
        }
        Some(members)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn load(ttl: &str) -> std::sync::Arc<RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("Turtle parse")
    }

    const PREFIXES_TTL: &str = r"
        @prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix meta: <https://example.org/meta/> .
    ";

    /// The fixture namespace table — the same `meta` declaration the Turtle
    /// fixtures use, supplied by the caller (nothing hardcoded in library code).
    fn fixture_ns() -> Namespaces {
        Namespaces::new(
            "meta",
            &[("meta".to_owned(), "https://example.org/meta/".to_owned())],
        )
        .expect("fixture namespaces are valid")
    }

    #[test]
    fn test_project_graph_envelope_and_context() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:alice a meta:Person ;
                meta:name "Alice" .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        // @context carries the prefix map (meta:).
        assert_eq!(doc["@context"]["meta"], json!("https://example.org/meta/"));
        let graph = doc["@graph"].as_array().expect("@graph array");
        assert_eq!(graph.len(), 1);
        let node = &graph[0];
        assert_eq!(node["@id"], json!("https://example.org/meta/alice"));
        assert_eq!(node["@type"], json!("meta:Person"));
        assert_eq!(node["meta:name"], json!("Alice"));
    }

    #[test]
    fn test_object_property_is_node_ref() {
        let store = load(&format!(
            r"{PREFIXES_TTL}
            meta:org meta:member meta:alice .
        "
        ));
        let doc = project_graph(&store, &fixture_ns());
        let graph = doc["@graph"].as_array().unwrap();
        let org = graph
            .iter()
            .find(|n| n["@id"] == json!("https://example.org/meta/org"))
            .expect("org node");
        assert_eq!(
            org["meta:member"],
            json!({ "@id": "https://example.org/meta/alice" })
        );
    }

    #[test]
    fn test_typed_and_numeric_and_lang_literals() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:e meta:count 3 ;
                meta:flag true ;
                meta:at "2026-06-23T00:00:00Z"^^xsd:dateTime ;
                meta:label "bonjour"@fr .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let node = &doc["@graph"].as_array().unwrap()[0];
        // integer → bare scalar
        assert_eq!(node["meta:count"], json!(3));
        // boolean → bare scalar
        assert_eq!(node["meta:flag"], json!(true));
        // dateTime → typed-literal object
        assert_eq!(
            node["meta:at"],
            json!({ "@value": "2026-06-23T00:00:00Z", "@type": "xsd:dateTime" })
        );
        // lang literal → {@value,@language}
        assert_eq!(
            node["meta:label"],
            json!({ "@value": "bonjour", "@language": "fr" })
        );
    }

    #[test]
    fn test_multi_valued_predicate_is_array_and_sorted() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:x meta:tag "b", "a", "c" .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let node = &doc["@graph"].as_array().unwrap()[0];
        let tags = node["meta:tag"].as_array().expect("array");
        assert_eq!(tags.len(), 3);
        // sorted by term string → "a","b","c"
        assert_eq!(tags, &[json!("a"), json!("b"), json!("c")]);
    }

    #[test]
    fn test_named_graph_data_is_excluded() {
        // alice lives in the default graph; bob lives ONLY in a named graph.
        // A TriG document expresses both; the native codec preserves the named
        // graph, and the projector must scope to the default graph only.
        let trig = format!(
            r#"{PREFIXES_TTL}
            meta:alice a meta:Person ; meta:name "Alice" .
            meta:graph_other {{
                meta:bob meta:name "Bob" .
            }}
        "#
        );
        let store = std::sync::Arc::new(
            ::purrdf::parse_dataset(trig.as_bytes(), "application/trig", None).expect("TriG parse"),
        );
        let doc = project_graph(&store, &fixture_ns());
        let graph = doc["@graph"].as_array().expect("@graph array");
        // Only the default-graph subject is projected — no named-graph leak.
        assert_eq!(graph.len(), 1, "named-graph subject must not appear");
        assert_eq!(graph[0]["@id"], json!("https://example.org/meta/alice"));
        assert!(
            graph
                .iter()
                .all(|n| n["@id"] != json!("https://example.org/meta/bob")),
            "named-graph subject leaked into @graph"
        );
    }

    #[test]
    fn scalars_are_bare_only_where_they_denote_the_literal_exactly() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:e meta:canonical 42 ;
                meta:negative -7 ;
                meta:signed "+5"^^xsd:integer ;
                meta:padded "007"^^xsd:integer ;
                meta:int "5"^^xsd:int ;
                meta:decimal 1.5 ;
                meta:double 1.5e0 ;
                meta:yes true ;
                meta:one "1"^^xsd:boolean ;
                meta:huge 123456789012345678901234567890 .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let node = &doc["@graph"].as_array().unwrap()[0];
        assert_eq!(node["meta:canonical"], json!(42));
        assert_eq!(node["meta:negative"], json!(-7));
        assert_eq!(node["meta:yes"], json!(true));
        for (key, lexical, datatype) in [
            ("meta:signed", "+5", "xsd:integer"),
            ("meta:padded", "007", "xsd:integer"),
            ("meta:int", "5", "xsd:int"),
            ("meta:decimal", "1.5", "xsd:decimal"),
            ("meta:double", "1.5e0", "xsd:double"),
            ("meta:one", "1", "xsd:boolean"),
            ("meta:huge", "123456789012345678901234567890", "xsd:integer"),
        ] {
            assert_eq!(
                node[key],
                json!({ "@value": lexical, "@type": datatype }),
                "{key}"
            );
        }
    }

    #[test]
    fn directional_literals_carry_their_base_direction() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:e meta:plain "hello"@en ;
                meta:rtl "שלום"@he--rtl ;
                meta:ltr "hello"@en--ltr .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let node = &doc["@graph"].as_array().unwrap()[0];
        assert_eq!(
            node["meta:plain"],
            json!({ "@value": "hello", "@language": "en" })
        );
        assert_eq!(
            node["meta:rtl"],
            json!({ "@value": "שלום", "@language": "he", "@direction": "rtl" })
        );
        assert_eq!(
            node["meta:ltr"],
            json!({ "@value": "hello", "@language": "en", "@direction": "ltr" })
        );
    }

    fn node<'a>(doc: &'a Value, local: &str) -> Option<&'a Value> {
        let id = format!("https://example.org/meta/{local}");
        doc["@graph"]
            .as_array()
            .expect("@graph")
            .iter()
            .find(|node| node["@id"] == json!(id))
    }

    #[test]
    fn well_formed_lists_are_list_objects_and_their_cells_are_not_nodes() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:e meta:items ( 3 "b" meta:x ( 1 2 ) ) ;
                meta:none () .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let graph = doc["@graph"].as_array().expect("@graph");
        assert_eq!(graph.len(), 1, "no list cell is a node: {graph:?}");
        let e = node(&doc, "e").expect("e");
        let items = e["meta:items"]["@list"].as_array().expect("@list");
        assert_eq!(items.len(), 4);
        assert_eq!(
            items[..3],
            [
                json!(3),
                json!("b"),
                json!({ "@id": "https://example.org/meta/x" })
            ]
        );
        // The member list carries its head cell's label, and no other key.
        let nested = items[3].as_object().expect("a nested list");
        assert_eq!(nested["@list"], json!([1, 2]));
        assert!(
            nested["@index"]
                .as_str()
                .is_some_and(|label| label.starts_with("_:"))
        );
        assert_eq!(nested.len(), 2);
        assert_eq!(e["meta:none"], json!({ "@list": [] }));
    }

    /// JSON-LD 1.1 §8.4.2 step 6.4 converts only well-formed list nodes: a cell
    /// with another property, a cell referenced twice, a cell that is an IRI
    /// and a cell typed `rdf:List` all keep the node-graph form. The walk from
    /// `rdf:nil` converts the well-formed suffix behind the first such cell, and
    /// an `rdf:rest rdf:nil` is the empty list wherever it stands.
    #[test]
    fn ill_formed_lists_keep_their_cells_as_nodes() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:e meta:extra _:a .
            _:a rdf:first 1 ; rdf:rest _:b ; meta:note "annotated" .
            _:b rdf:first 2 ; rdf:rest rdf:nil .

            meta:e meta:shared _:c .
            meta:f meta:shared _:c .
            _:c rdf:first 1 ; rdf:rest rdf:nil .

            meta:e meta:named meta:cell .
            meta:cell rdf:first 1 ; rdf:rest rdf:nil .

            meta:e meta:typed _:d .
            _:d a rdf:List ; rdf:first 1 ; rdf:rest rdf:nil .

            meta:e meta:branching _:g .
            _:g rdf:first 1 , 2 ; rdf:rest rdf:nil .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let graph = doc["@graph"].as_array().expect("@graph");
        let e = node(&doc, "e").expect("e");
        // The annotated head stays a node; its well-formed tail is a list.
        let head = e["meta:extra"]["@id"].as_str().expect("node reference");
        let head = graph
            .iter()
            .find(|node| node["@id"] == json!(head))
            .expect("the annotated head is a node");
        assert_eq!(head["rdf:rest"], json!({ "@list": [2] }));
        assert_eq!(head["meta:note"], json!("annotated"));
        // A shared cell, an IRI cell, a typed cell and a branching cell stay
        // nodes, each ending in the empty list.
        for key in ["meta:shared", "meta:named", "meta:typed", "meta:branching"] {
            let id = e[key]["@id"].as_str().unwrap_or_else(|| panic!("{key}"));
            let cell = graph
                .iter()
                .find(|node| node["@id"] == json!(id))
                .unwrap_or_else(|| panic!("{key} cell is a node"));
            assert_eq!(cell["rdf:rest"], json!({ "@list": [] }), "{key}");
        }
        assert_eq!(node(&doc, "cell").expect("IRI cell")["rdf:first"], json!(1));
    }

    #[test]
    fn a_list_cell_inside_a_triple_term_stays_a_node() {
        let store = load(&format!(
            r"{PREFIXES_TTL}
            meta:e meta:items _:a .
            _:a rdf:first 1 ; rdf:rest rdf:nil .
            meta:e meta:says <<( _:a rdf:rest rdf:nil )>> .
        "
        ));
        let doc = project_graph(&store, &fixture_ns());
        let e = node(&doc, "e").expect("e");
        assert!(e["meta:items"]["@id"].is_string(), "{e}");
        assert_eq!(
            e["meta:says"]["@id"]["rdf:rest"],
            json!({ "@id": "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil" })
        );
    }

    #[test]
    fn a_cyclic_list_terminates() {
        let store = load(&format!(
            r"{PREFIXES_TTL}
            meta:e meta:items _:a .
            _:a rdf:first 1 ; rdf:rest _:b .
            _:b rdf:first 2 ; rdf:rest _:a .
        "
        ));
        let doc = project_graph(&store, &fixture_ns());
        let e = node(&doc, "e").expect("e");
        assert!(e["meta:items"]["@id"].is_string(), "{e}");
    }

    #[test]
    fn test_determinism_byte_stable() {
        let ttl = format!(
            r#"{PREFIXES_TTL}
            meta:alice a meta:Person ; meta:name "Alice" ; meta:age 30 .
            meta:bob a meta:Person ; meta:name "Bob" .
        "#
        );
        let a = serde_json::to_string_pretty(&project_graph(&load(&ttl), &fixture_ns())).unwrap();
        let b = serde_json::to_string_pretty(&project_graph(&load(&ttl), &fixture_ns())).unwrap();
        assert_eq!(a, b, "projection must be byte-stable");
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What each schema emitter makes of the SHACL 1.2 constructs, pinned exactly.
//!
//! The TypeScript, Pydantic, GraphQL and LinkML emitters read the compiled JSON
//! Schema ([`CompiledSchema`]), never the shapes graph, so a construct reaches
//! them only as the JSON Schema compiler projects it. For every construct and
//! every emitter, a test named `<emitter>_projects_<construct>` asserts the
//! exact emitted fragment and the exact loss-ledger entries: the SHACL → JSON
//! Schema entries on [`CompiledSchema::losses`] (which every downstream emitter
//! inherits, since it consumes that schema) and the emitter's own entries under
//! the affected definition. A construct is projected or its loss is recorded;
//! none vanishes.
//!
//! A SHACL list value node is an RDF list, which the instance projection keeps
//! as a node reference whose members live on separate `@graph` nodes, so the
//! list components are recorded losses rather than `minItems`/`maxItems`/
//! `uniqueItems`/`items` (which would judge a multi-valued property's values
//! instead); the Pydantic emitter additionally keeps each `$comment` naming a
//! dropped constraint in its models' `model_json_schema()`.

use std::collections::BTreeMap;

use purrdf_shapes::json_schema::{CompiledSchema, Namespaces, compile};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_shapes::{
    GRAPHQL_SCHEMA_PATH, GraphqlConfig, LinkmlConfig, PydanticConfig, TYPESCRIPT_DECLARATION_PATH,
    TypeScriptConfig, emit_graphql, emit_linkml, emit_pydantic, emit_typescript,
};
use serde_json::Value;

const PREFIXES: &str = r"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex:  <https://example.org/> .
";

/// The subject every fixture's losses are recorded against.
const HOLDER: &str = "<https://example.org/HolderShape>";

/// The JSON pointer prefix of the definition each fixture's constructs land in.
const HOLDER_DEF: &str = "#/$defs/Holder";

/// `sh:minListLength` on a property shape and on a node shape.
const MIN_LIST_LENGTH: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:minListLength 1 ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:minListLength 2 ] .
";

/// `sh:maxListLength` on a property shape and on a node shape.
const MAX_LIST_LENGTH: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:maxListLength 4 ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:maxListLength 3 ] .
";

/// `sh:uniqueMembers true` on a property shape and on a node shape, beside a
/// `sh:uniqueMembers false` neighbour (`ex:other`) that checks nothing.
const UNIQUE_MEMBERS: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:uniqueMembers true ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:uniqueMembers true ] ;
        sh:property [ sh:path ex:other ; sh:maxCount 1 ; sh:uniqueMembers false ] .
";

/// `sh:memberShape` on a property shape and on a node shape.
const MEMBER_SHAPE: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:memberShape [ sh:nodeKind sh:IRI ] ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:memberShape [ sh:nodeKind sh:IRI ] ] .
";

/// A list value of `sh:class` on a property shape (one member with a node
/// shape, one without) and on a node shape.
const LIST_VALUED_CLASS: &str = r"
    ex:CatShape a sh:NodeShape ; sh:targetClass ex:Cat .
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:class ( ex:Cat ex:Dog ) ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:class ( ex:Cat ex:Dog ) ] .
";

/// A list value of `sh:datatype` on a property shape and on a node shape.
const LIST_VALUED_DATATYPE: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:datatype ( xsd:string xsd:integer ) ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:datatype ( xsd:string xsd:integer ) ] .
";

/// A list value of `sh:nodeKind` on a property shape (with a `sh:TripleTerm`
/// member) and on a node shape.
const LIST_VALUED_NODE_KIND: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:nodeKind ( sh:IRI sh:BlankNode ) ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:nodeKind ( sh:IRI sh:Literal sh:TripleTerm ) ] .
";

/// `sh:subsetOf` on a property shape (a sequence path) and on a node shape (an
/// IRI).
const SUBSET_OF: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:subsetOf ex:allowed ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:subsetOf ( ex:allowed ex:member ) ] .
";

/// Path-valued `sh:equals`, `sh:lessThan` and `sh:lessThanOrEquals` on a
/// property shape and `sh:disjoint` on a node shape.
const PATH_VALUED_PAIRS: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:disjoint ( ex:a ex:b ) ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:equals ( ex:a ex:b ) ; sh:lessThan [ sh:inversePath ex:c ] ; sh:lessThanOrEquals [ sh:zeroOrMorePath ex:d ] ] .
";

/// An IRI-valued `sh:equals`, the neighbour of the path-valued pairs: it
/// converts exactly as they do.
const IRI_PAIR: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:equals ex:twin ] .
";

/// `sh:singleLine true` on a property shape (projected) and on a node shape
/// (vacuous: a subject is never a literal).
const SINGLE_LINE: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:singleLine true ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:datatype xsd:string ; sh:singleLine true ] .
";

/// `sh:rootClass` on a property shape (a list of roots) and on a node shape.
const ROOT_CLASS: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:rootClass ex:Root ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:rootClass ( ex:Root ex:Other ) ] .
";

/// `sh:someValue` on a property shape (dropped) and on a node shape (projected
/// as `sh:node` is).
const SOME_VALUE: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:someValue [ sh:property [ sh:path ex:name ; sh:minCount 1 ] ] ;
        sh:property [ sh:path ex:subject ; sh:someValue [ sh:nodeKind sh:IRI ] ] .
";

/// `sh:uniqueValuesFor` on a node shape and on a property shape.
const UNIQUE_VALUES_FOR: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:uniqueValuesFor ( ex:a ex:b ) ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:uniqueValuesFor ex:serial ] .
";

/// `sh:closed sh:ByTypes` on a node shape: the object stays open.
const CLOSED_BY_TYPES: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:closed sh:ByTypes ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:datatype xsd:string ] .
";

/// Property shapes whose value nodes `sh:values` and `sh:defaultValue` compute,
/// beside a plain property shape (`ex:kept`).
const COMPUTED_VALUES: &str = r#"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:values [ sh:path ex:source ] ] ;
        sh:property [ sh:path ex:fallback ; sh:maxCount 1 ; sh:defaultValue "none" ] ;
        sh:property [ sh:path ex:kept ; sh:maxCount 1 ; sh:datatype xsd:string ] .
"#;

/// A shape targeted by `sh:targetWhere`, beside a class-targeted shape.
const TARGET_WHERE: &str = r"
    ex:WhereShape a sh:NodeShape ; sh:targetWhere [ sh:class ex:Holder ] ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:datatype xsd:string ] .
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ] .
";

/// A shape targeted by a node-expression `sh:targetNode`, beside a class-
/// targeted shape.
const NODE_EXPRESSION_TARGET: &str = r"
    ex:PinnedShape a sh:NodeShape ; sh:targetNode [ sh:path ex:pins ] ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:datatype xsd:string ] .
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ] .
";

/// A property shape of severity `sh:Debug`, beside a `sh:Violation` one
/// (`ex:kept`).
const DEBUG_SEVERITY: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:datatype xsd:string ; sh:severity sh:Debug ] ;
        sh:property [ sh:path ex:kept ; sh:maxCount 1 ; sh:datatype xsd:string ] .
";

/// A constraint given severity `sh:Trace` by a reifier annotation.
const REIFIER_SEVERITY: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:subject ; sh:maxCount 1 ; sh:datatype xsd:string {| sh:severity sh:Trace |} ] .
";

/// One fixture compiled and emitted by all five emitters.
struct Emitted {
    compiled: CompiledSchema,
    schema: Value,
    typescript: String,
    typescript_losses: String,
    pydantic: String,
    pydantic_losses: String,
    graphql: String,
    graphql_losses: String,
    linkml: Value,
    linkml_losses: String,
}

fn namespaces() -> Namespaces {
    Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )
    .expect("namespace table")
}

fn emit(body: &str) -> Emitted {
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES}{body}"), None).expect("Turtle");
    let shapes = from_dataset(&dataset).expect("shapes graph");
    let compiled = compile(&shapes, &namespaces()).expect("schema compilation");
    let schema = serde_json::from_str(&compiled.schema_json).expect("schema JSON");
    let typescript = emit_typescript(
        &compiled,
        &TypeScriptConfig::new("example-types", "Example package.", "Example declarations.")
            .expect("TypeScript config"),
    )
    .expect("TypeScript emission");
    let pydantic = emit_pydantic(
        &compiled,
        &PydanticConfig::new("example_models", "Example package.", "Example models.")
            .expect("Pydantic config"),
    )
    .expect("Pydantic emission");
    let graphql = emit_graphql(
        &compiled,
        &GraphqlConfig::new("Example", "Example package.", "Example module.", "RdfValue")
            .expect("GraphQL config"),
    )
    .expect("GraphQL emission");
    let linkml = emit_linkml(
        &compiled,
        &LinkmlConfig::new(
            "https://example.org/generated",
            "ExampleSchema",
            "Example schema.",
            "ex",
            BTreeMap::from([
                ("ex".to_owned(), "https://example.org/".to_owned()),
                ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
            ]),
        )
        .expect("LinkML config"),
    )
    .expect("LinkML emission");
    Emitted {
        schema,
        typescript: String::from_utf8(typescript.artifacts[TYPESCRIPT_DECLARATION_PATH].clone())
            .expect("UTF-8 declarations"),
        typescript_losses: typescript.losses.render_json(),
        pydantic: String::from_utf8(pydantic.artifacts["example_models/models.py"].clone())
            .expect("UTF-8 models"),
        pydantic_losses: pydantic.losses.render_json(),
        graphql: String::from_utf8(graphql.artifacts[GRAPHQL_SCHEMA_PATH].clone())
            .expect("UTF-8 SDL"),
        graphql_losses: graphql.losses.render_json(),
        linkml: linkml.document.as_value().clone(),
        linkml_losses: linkml.losses.render_json(),
        compiled,
    }
}

fn json(text: &str) -> Value {
    serde_json::from_str(text).expect("expected JSON")
}

fn ledger_rows(ledger: &str) -> Vec<(String, String, String)> {
    let ledger: Value = serde_json::from_str(ledger).expect("ledger JSON");
    ledger["losses"]
        .as_array()
        .expect("losses array")
        .iter()
        .map(|entry| {
            let location = entry["location"].as_str().unwrap_or_default();
            let subject = location.split_once("subject=").map_or(location, |(_, s)| s);
            (
                entry["code"].as_str().expect("code").to_owned(),
                subject.to_owned(),
                entry["note"].as_str().expect("note").to_owned(),
            )
        })
        .collect()
}

/// `(code, subject, note)` of every SHACL → JSON Schema entry.
fn source_losses(emitted: &Emitted) -> Vec<(String, String, String)> {
    ledger_rows(&emitted.compiled.losses.render_json())
}

/// `(code, subject)` of every SHACL → JSON Schema entry — what each downstream
/// emitter inherits with the schema it consumes.
fn source_codes(emitted: &Emitted) -> Vec<(String, String)> {
    source_losses(emitted)
        .into_iter()
        .map(|(code, subject, _)| (code, subject))
        .collect()
}

/// `(code, pointer)` of every emitter-stage entry under `Holder`.
fn holder_losses(ledger: &str) -> Vec<(String, String)> {
    ledger_rows(ledger)
        .into_iter()
        .filter(|(_, pointer, _)| {
            pointer == HOLDER_DEF || pointer.starts_with(&format!("{HOLDER_DEF}/"))
        })
        .map(|(code, pointer, _)| (code, pointer))
        .collect()
}

fn owned2(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|&(a, b)| (a.to_owned(), b.to_owned()))
        .collect()
}

fn owned3(rows: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
    rows.iter()
        .map(|&(a, b, c)| (a.to_owned(), b.to_owned(), c.to_owned()))
        .collect()
}

/// The declaration of the exported TypeScript type `name`, through its `;`.
fn ts_type(ts: &str, name: &str) -> Option<String> {
    let start = ts.find(&format!("export type {name} = "))?;
    let block = &ts[start..];
    Some(block[..block.find(";\n\n").expect("declaration end") + 2].to_owned())
}

/// The Pydantic field whose alias is `alias`, when the model has one.
fn py_field(py: &str, alias: &str) -> Option<String> {
    py.lines()
        .find(|line| line.ends_with(&format!("alias=\"{alias}\")")))
        .map(ToOwned::to_owned)
}

/// The GraphQL object type `name`, when the SDL declares one.
fn gql_type(gql: &str, name: &str) -> Option<String> {
    let start = gql.find(&format!("type {name} {{\n"))?;
    let block = &gql[start..];
    Some(block[..block.find("\n}\n").expect("type end") + 2].to_owned())
}

/// Every `$comment` the compiled `Holder` definition carries (its own and each
/// property's), which the Pydantic models keep in `model_json_schema()`.
fn holder_comments(emitted: &Emitted) -> Vec<String> {
    let holder = &emitted.schema["$defs"]["Holder"];
    let mut comments: Vec<String> = holder["$comment"]
        .as_str()
        .map(ToOwned::to_owned)
        .into_iter()
        .collect();
    if let Some(properties) = holder["properties"].as_object() {
        for property in properties.values() {
            let alternatives = property["anyOf"].as_array().cloned().unwrap_or_default();
            for schema in std::iter::once(property).chain(alternatives.iter()) {
                if let Some(comment) = schema["$comment"].as_str() {
                    comments.push(comment.to_owned());
                }
            }
        }
    }
    comments
}

fn assert_pydantic_keeps_comments(emitted: &Emitted) {
    for comment in holder_comments(emitted) {
        let encoded = serde_json::to_string(&comment).expect("JSON string");
        assert!(
            emitted.pydantic.contains(&encoded),
            "model_json_schema() keeps {encoded}"
        );
    }
}

#[test]
fn json_schema_projects_min_list_length() {
    let emitted = emit(MIN_LIST_LENGTH);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:minListLength",
                HOLDER,
                "a SHACL list constraint has no projection in this emitter"
            ),
            (
                "sh:minListLength",
                HOLDER,
                "a SHACL list constraint on the focus node has no projection in this emitter"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:minListLength constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:minListLength constraint was dropped (no projection in this emitter)""#
        )
    );
}

#[test]
fn typescript_projects_min_list_length() {
    let emitted = emit(MIN_LIST_LENGTH);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:minListLength", HOLDER), ("sh:minListLength", HOLDER),])
    );
}

#[test]
fn pydantic_projects_min_list_length() {
    let emitted = emit(MIN_LIST_LENGTH);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:minListLength", HOLDER), ("sh:minListLength", HOLDER),])
    );
}

#[test]
fn graphql_projects_min_list_length() {
    let emitted = emit(MIN_LIST_LENGTH);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:minListLength", HOLDER), ("sh:minListLength", HOLDER),])
    );
}

#[test]
fn linkml_projects_min_list_length() {
    let emitted = emit(MIN_LIST_LENGTH);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:minListLength", HOLDER), ("sh:minListLength", HOLDER),])
    );
}

#[test]
fn json_schema_projects_max_list_length() {
    let emitted = emit(MAX_LIST_LENGTH);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:maxListLength",
                HOLDER,
                "a SHACL list constraint has no projection in this emitter"
            ),
            (
                "sh:maxListLength",
                HOLDER,
                "a SHACL list constraint on the focus node has no projection in this emitter"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:maxListLength constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:maxListLength constraint was dropped (no projection in this emitter)""#
        )
    );
}

#[test]
fn typescript_projects_max_list_length() {
    let emitted = emit(MAX_LIST_LENGTH);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:maxListLength", HOLDER), ("sh:maxListLength", HOLDER),])
    );
}

#[test]
fn pydantic_projects_max_list_length() {
    let emitted = emit(MAX_LIST_LENGTH);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:maxListLength", HOLDER), ("sh:maxListLength", HOLDER),])
    );
}

#[test]
fn graphql_projects_max_list_length() {
    let emitted = emit(MAX_LIST_LENGTH);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:maxListLength", HOLDER), ("sh:maxListLength", HOLDER),])
    );
}

#[test]
fn linkml_projects_max_list_length() {
    let emitted = emit(MAX_LIST_LENGTH);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:maxListLength", HOLDER), ("sh:maxListLength", HOLDER),])
    );
}

#[test]
fn json_schema_projects_unique_members() {
    let emitted = emit(UNIQUE_MEMBERS);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:uniqueMembers",
                HOLDER,
                "a SHACL list constraint has no projection in this emitter"
            ),
            (
                "sh:uniqueMembers",
                HOLDER,
                "a SHACL list constraint on the focus node has no projection in this emitter"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:uniqueMembers constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:uniqueMembers constraint was dropped (no projection in this emitter)""#
        )
    );
    assert_eq!(holder["properties"]["ex:other"], json(r"{}"));
}

#[test]
fn typescript_projects_unique_members() {
    let emitted = emit(UNIQUE_MEMBERS);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:other\"?: JsonValue;\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:uniqueMembers", HOLDER), ("sh:uniqueMembers", HOLDER),])
    );
}

#[test]
fn pydantic_projects_unique_members() {
    let emitted = emit(UNIQUE_MEMBERS);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:uniqueMembers", HOLDER), ("sh:uniqueMembers", HOLDER),])
    );
}

#[test]
fn graphql_projects_unique_members() {
    let emitted = emit(UNIQUE_MEMBERS);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exOther: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:other"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:uniqueMembers", HOLDER), ("sh:uniqueMembers", HOLDER),])
    );
}

#[test]
fn linkml_projects_unique_members() {
    let emitted = emit(UNIQUE_MEMBERS);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/properties/ex:other"
            ),
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/properties/ex:subject"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:uniqueMembers", HOLDER), ("sh:uniqueMembers", HOLDER),])
    );
}

#[test]
fn json_schema_projects_member_shape() {
    let emitted = emit(MEMBER_SHAPE);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:memberShape",
                HOLDER,
                "a SHACL list constraint has no projection in this emitter"
            ),
            (
                "sh:memberShape",
                HOLDER,
                "a SHACL list constraint on the focus node has no projection in this emitter"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:memberShape constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:memberShape constraint was dropped (no projection in this emitter)""#
        )
    );
}

#[test]
fn typescript_projects_member_shape() {
    let emitted = emit(MEMBER_SHAPE);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:memberShape", HOLDER), ("sh:memberShape", HOLDER),])
    );
}

#[test]
fn pydantic_projects_member_shape() {
    let emitted = emit(MEMBER_SHAPE);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:memberShape", HOLDER), ("sh:memberShape", HOLDER),])
    );
}

#[test]
fn graphql_projects_member_shape() {
    let emitted = emit(MEMBER_SHAPE);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:memberShape", HOLDER), ("sh:memberShape", HOLDER),])
    );
}

#[test]
fn linkml_projects_member_shape() {
    let emitted = emit(MEMBER_SHAPE);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:memberShape", HOLDER), ("sh:memberShape", HOLDER),])
    );
}

#[test]
fn json_schema_projects_list_valued_class() {
    let emitted = emit(LIST_VALUED_CLASS);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:class",
            HOLDER,
            "a constraint on the focus node itself has no projection in an object schema describing the node's properties"
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r##"{"anyOf":[{"$comment":"ex:Dog has no NodeShape; node reference only","properties":{"@id":{"type":"string"}},"required":["@id"],"type":"object"},{"$ref":"#/$defs/Cat"},{"properties":{"@id":{"type":"string"}},"required":["@id"],"type":"object"}]}"##
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:class constraint was dropped (no projection in an object schema)""#
        )
    );
    let defs: Vec<&String> = emitted.schema["$defs"]
        .as_object()
        .expect("$defs")
        .keys()
        .collect();
    assert_eq!(defs, ["Annotation", "Cat", "Holder", "Node"]);
}

#[test]
fn typescript_projects_list_valued_class() {
    let emitted = emit(LIST_VALUED_CLASS);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: ({\n        readonly \"@id\": string;\n        readonly [key: string]: JsonValue;\n      } | Cat);\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(source_codes(&emitted), owned2(&[("sh:class", HOLDER),]));
}

#[test]
fn pydantic_projects_list_valued_class() {
    let emitted = emit(LIST_VALUED_CLASS);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some(
            "    subject: _InlineDefsHolderPropertiesExSubjectAnyOf0Object | Cat | _InlineDefsHolderPropertiesExSubjectAnyOf2Object = Field(default=None, alias=\"ex:subject\")"
        )
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(source_codes(&emitted), owned2(&[("sh:class", HOLDER),]));
}

#[test]
fn graphql_projects_list_valued_class() {
    let emitted = emit(LIST_VALUED_CLASS);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/ex:subject/anyOf"
            ),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:class", HOLDER),]));
}

#[test]
fn linkml_projects_list_valued_class() {
    let emitted = emit(LIST_VALUED_CLASS);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(
            r#"{"alias":"ex:subject","any_of":[{"inlined":true,"range":"InlineDefsHolderPropertiesExSubjectAnyOf0Object"},{"inlined":true,"range":"Cat"},{"inlined":true,"range":"InlineDefsHolderPropertiesExSubjectAnyOf2Object"}],"required":false,"slot_uri":"ex:subject"}"#
        )
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(holder_losses(&emitted.linkml_losses), owned2(&[]));
    assert_eq!(source_codes(&emitted), owned2(&[("sh:class", HOLDER),]));
}

#[test]
fn json_schema_projects_list_valued_datatype() {
    let emitted = emit(LIST_VALUED_DATATYPE);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:datatype",
                HOLDER,
                "numeric literals project as bare JSON numbers without their datatype or lexical form, so a numeric literal of another numeric datatype, or an ill-typed one, is not told apart"
            ),
            (
                "sh:datatype",
                HOLDER,
                "a constraint on the focus node itself has no projection in an object schema describing the node's properties"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:datatype constraint on property ex:subject is widened (numeric datatypes are not told apart)","anyOf":[{"anyOf":[{"type":"integer"},{"properties":{"@type":{"const":"xsd:integer"},"@value":{"type":"string"}},"required":["@value","@type"],"type":"object"}]},{"type":"string"}]}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:datatype constraint was dropped (no projection in an object schema)""#
        )
    );
}

#[test]
fn typescript_projects_list_valued_datatype() {
    let emitted = emit(LIST_VALUED_DATATYPE);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: ((number | {\n          readonly \"@type\": \"xsd:integer\";\n          readonly \"@value\": string;\n          readonly [key: string]: JsonValue;\n        }) | string);\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(
        holder_losses(&emitted.typescript_losses),
        owned2(&[(
            "integer-validation-widened",
            "#/$defs/Holder/properties/ex:subject/anyOf/0/anyOf/0/type"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:datatype", HOLDER), ("sh:datatype", HOLDER),])
    );
}

#[test]
fn pydantic_projects_list_valued_datatype() {
    let emitted = emit(LIST_VALUED_DATATYPE);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some(
            "    subject: StrictInt | _InlineDefsHolderPropertiesExSubjectAnyOf0AnyOf1Object | StrictStr = Field(default=None, alias=\"ex:subject\")"
        )
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:datatype", HOLDER), ("sh:datatype", HOLDER),])
    );
}

#[test]
fn graphql_projects_list_valued_datatype() {
    let emitted = emit(LIST_VALUED_DATATYPE);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/ex:subject/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:datatype", HOLDER), ("sh:datatype", HOLDER),])
    );
}

#[test]
fn linkml_projects_list_valued_datatype() {
    let emitted = emit(LIST_VALUED_DATATYPE);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(
            r#"{"alias":"ex:subject","any_of":[{"any_of":[{"range":"integer"},{"inlined":true,"range":"InlineDefsHolderPropertiesExSubjectAnyOf0AnyOf1Object"}]},{"range":"string"}],"required":false,"slot_uri":"ex:subject"}"#
        )
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(holder_losses(&emitted.linkml_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:datatype", HOLDER), ("sh:datatype", HOLDER),])
    );
}

#[test]
fn json_schema_projects_list_valued_node_kind() {
    let emitted = emit(LIST_VALUED_NODE_KIND);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:nodeKind",
            HOLDER,
            "a constraint on the focus node itself has no projection in an object schema describing the node's properties"
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"anyOf":[{"properties":{"@id":{"pattern":"^(?:[^_]|_(?:[^:]|$))","type":"string"}},"required":["@id"],"type":"object"},{"properties":{"@id":{"type":"object"}},"required":["@id"],"type":"object"},{"properties":{"@type":{"type":"string"},"@value":{}},"required":["@value"],"type":"object"},{"type":"boolean"},{"type":"number"},{"type":"string"}]}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:nodeKind constraint was dropped (no projection in an object schema)""#
        )
    );
}

#[test]
fn typescript_projects_list_valued_node_kind() {
    let emitted = emit(LIST_VALUED_NODE_KIND);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: ({\n        readonly \"@id\": string;\n        readonly [key: string]: JsonValue;\n      } | {\n        readonly \"@id\": {\n            readonly [key: string]: JsonValue;\n          };\n        readonly [key: string]: JsonValue;\n      } | {\n        readonly \"@type\"?: string;\n        readonly \"@value\": JsonValue;\n        readonly [key: string]: JsonValue;\n      } | boolean | number | string);\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(
        holder_losses(&emitted.typescript_losses),
        owned2(&[(
            "string-validation-dropped",
            "#/$defs/Holder/properties/ex:subject/anyOf/0/properties/@id/pattern"
        ),])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:nodeKind", HOLDER),]));
}

#[test]
fn pydantic_projects_list_valued_node_kind() {
    let emitted = emit(LIST_VALUED_NODE_KIND);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some(
            "    subject: _InlineDefsHolderPropertiesExSubjectAnyOf0Object | _InlineDefsHolderPropertiesExSubjectAnyOf1Object | _InlineDefsHolderPropertiesExSubjectAnyOf2Object | StrictBool | Annotated[StrictFloat, Field(allow_inf_nan=False)] | StrictInt | StrictStr = Field(default=None, alias=\"ex:subject\")"
        )
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(source_codes(&emitted), owned2(&[("sh:nodeKind", HOLDER),]));
}

#[test]
fn graphql_projects_list_valued_node_kind() {
    let emitted = emit(LIST_VALUED_NODE_KIND);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/ex:subject/anyOf"
            ),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:nodeKind", HOLDER),]));
}

#[test]
fn linkml_projects_list_valued_node_kind() {
    let emitted = emit(LIST_VALUED_NODE_KIND);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(
            r#"{"alias":"ex:subject","any_of":[{"inlined":true,"range":"InlineDefsHolderPropertiesExSubjectAnyOf0Object"},{"inlined":true,"range":"InlineDefsHolderPropertiesExSubjectAnyOf1Object"},{"inlined":true,"range":"InlineDefsHolderPropertiesExSubjectAnyOf2Object"},{"range":"boolean"},{"range":"double"},{"range":"string"}],"required":false,"slot_uri":"ex:subject"}"#
        )
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject/anyOf/2/properties/@value"
        ),])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:nodeKind", HOLDER),]));
}

#[test]
fn json_schema_projects_subset_of() {
    let emitted = emit(SUBSET_OF);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:subsetOf",
                HOLDER,
                "a comparison of the values against the nodes the path <https://example.org/allowed>/<https://example.org/member> reaches from the focus node has no projection in this emitter"
            ),
            (
                "sh:subsetOf",
                HOLDER,
                "a comparison of the values against the nodes the path <https://example.org/allowed> reaches from the focus node has no projection in this emitter"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:subsetOf <https://example.org/allowed>/<https://example.org/member> constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:subsetOf <https://example.org/allowed> constraint was dropped (no projection in this emitter)""#
        )
    );
}

#[test]
fn typescript_projects_subset_of() {
    let emitted = emit(SUBSET_OF);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:subsetOf", HOLDER), ("sh:subsetOf", HOLDER),])
    );
}

#[test]
fn pydantic_projects_subset_of() {
    let emitted = emit(SUBSET_OF);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:subsetOf", HOLDER), ("sh:subsetOf", HOLDER),])
    );
}

#[test]
fn graphql_projects_subset_of() {
    let emitted = emit(SUBSET_OF);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:subsetOf", HOLDER), ("sh:subsetOf", HOLDER),])
    );
}

#[test]
fn linkml_projects_subset_of() {
    let emitted = emit(SUBSET_OF);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:subsetOf", HOLDER), ("sh:subsetOf", HOLDER),])
    );
}

#[test]
fn json_schema_projects_path_valued_pairs() {
    let emitted = emit(PATH_VALUED_PAIRS);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:disjoint",
                HOLDER,
                "a comparison of the values against the nodes the path <https://example.org/a>/<https://example.org/b> reaches from the focus node has no projection in this emitter"
            ),
            (
                "sh:equals",
                HOLDER,
                "a comparison of the values against the nodes the path <https://example.org/a>/<https://example.org/b> reaches from the focus node has no projection in this emitter"
            ),
            (
                "sh:lessThan",
                HOLDER,
                "a comparison of the values against the nodes the path ^<https://example.org/c> reaches from the focus node has no projection in this emitter"
            ),
            (
                "sh:lessThanOrEquals",
                HOLDER,
                "a comparison of the values against the nodes the path <https://example.org/d>* reaches from the focus node has no projection in this emitter"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:equals <https://example.org/a>/<https://example.org/b> constraint on property ex:subject was dropped (no projection in this emitter); a sh:lessThan ^<https://example.org/c> constraint on property ex:subject was dropped (no projection in this emitter); a sh:lessThanOrEquals <https://example.org/d>* constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:disjoint <https://example.org/a>/<https://example.org/b> constraint was dropped (no projection in this emitter)""#
        )
    );
}

#[test]
fn typescript_projects_path_valued_pairs() {
    let emitted = emit(PATH_VALUED_PAIRS);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:disjoint", HOLDER),
            ("sh:equals", HOLDER),
            ("sh:lessThan", HOLDER),
            ("sh:lessThanOrEquals", HOLDER),
        ])
    );
}

#[test]
fn pydantic_projects_path_valued_pairs() {
    let emitted = emit(PATH_VALUED_PAIRS);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:disjoint", HOLDER),
            ("sh:equals", HOLDER),
            ("sh:lessThan", HOLDER),
            ("sh:lessThanOrEquals", HOLDER),
        ])
    );
}

#[test]
fn graphql_projects_path_valued_pairs() {
    let emitted = emit(PATH_VALUED_PAIRS);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:disjoint", HOLDER),
            ("sh:equals", HOLDER),
            ("sh:lessThan", HOLDER),
            ("sh:lessThanOrEquals", HOLDER),
        ])
    );
}

#[test]
fn linkml_projects_path_valued_pairs() {
    let emitted = emit(PATH_VALUED_PAIRS);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:disjoint", HOLDER),
            ("sh:equals", HOLDER),
            ("sh:lessThan", HOLDER),
            ("sh:lessThanOrEquals", HOLDER),
        ])
    );
}

#[test]
fn json_schema_projects_iri_pair() {
    let emitted = emit(IRI_PAIR);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:equals",
            HOLDER,
            "a comparison of the values against the nodes the path <https://example.org/twin> reaches from the focus node has no projection in this emitter"
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:equals <https://example.org/twin> constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(holder["$comment"], Value::Null);
}

#[test]
fn typescript_projects_iri_pair() {
    let emitted = emit(IRI_PAIR);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(source_codes(&emitted), owned2(&[("sh:equals", HOLDER),]));
}

#[test]
fn pydantic_projects_iri_pair() {
    let emitted = emit(IRI_PAIR);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(source_codes(&emitted), owned2(&[("sh:equals", HOLDER),]));
}

#[test]
fn graphql_projects_iri_pair() {
    let emitted = emit(IRI_PAIR);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:equals", HOLDER),]));
}

#[test]
fn linkml_projects_iri_pair() {
    let emitted = emit(IRI_PAIR);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:equals", HOLDER),]));
}

#[test]
fn json_schema_projects_single_line() {
    let emitted = emit(SINGLE_LINE);
    assert_eq!(source_losses(&emitted), owned3(&[]));
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(r#"{"not":{"pattern":"[\\n\\r\\u000B\\u000C]","type":"string"},"type":"string"}"#)
    );
    assert_eq!(holder["$comment"], Value::Null);
}

#[test]
fn typescript_projects_single_line() {
    let emitted = emit(SINGLE_LINE);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: string;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(
        holder_losses(&emitted.typescript_losses),
        owned2(&[
            (
                "negation-validation-dropped",
                "#/$defs/Holder/properties/ex:subject/not"
            ),
            (
                "string-validation-dropped",
                "#/$defs/Holder/properties/ex:subject/not/pattern"
            ),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[]));
}

#[test]
fn pydantic_projects_single_line() {
    let emitted = emit(SINGLE_LINE);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: StrictStr = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(
        holder_losses(&emitted.pydantic_losses),
        owned2(&[(
            "negation-validation-dropped",
            "#/$defs/Holder/properties/ex:subject/not"
        ),])
    );
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(source_codes(&emitted), owned2(&[]));
}

#[test]
fn graphql_projects_single_line() {
    let emitted = emit(SINGLE_LINE);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "negation-validation-delegated",
                "#/$defs/Holder/properties/ex:subject/not"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[]));
}

#[test]
fn linkml_projects_single_line() {
    let emitted = emit(SINGLE_LINE);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(
            r#"{"alias":"ex:subject","none_of":[{"pattern":"[\\n\\r\\u000B\\u000C]","range":"string"}],"range":"string","required":false,"slot_uri":"ex:subject"}"#
        )
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(holder_losses(&emitted.linkml_losses), owned2(&[]));
    assert_eq!(source_codes(&emitted), owned2(&[]));
}

#[test]
fn json_schema_projects_root_class() {
    let emitted = emit(ROOT_CLASS);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:rootClass",
                HOLDER,
                "an rdfs:subClassOf* bound on class-valued values has no JSON Schema equivalent"
            ),
            (
                "sh:rootClass",
                HOLDER,
                "an rdfs:subClassOf* bound on the focus node has no JSON Schema equivalent"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:rootClass constraint on property ex:subject was dropped (no JSON Schema equivalent)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(r#""a node-level sh:rootClass constraint was dropped (no JSON Schema equivalent)""#)
    );
}

#[test]
fn typescript_projects_root_class() {
    let emitted = emit(ROOT_CLASS);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:rootClass", HOLDER), ("sh:rootClass", HOLDER),])
    );
}

#[test]
fn pydantic_projects_root_class() {
    let emitted = emit(ROOT_CLASS);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:rootClass", HOLDER), ("sh:rootClass", HOLDER),])
    );
}

#[test]
fn graphql_projects_root_class() {
    let emitted = emit(ROOT_CLASS);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:rootClass", HOLDER), ("sh:rootClass", HOLDER),])
    );
}

#[test]
fn linkml_projects_root_class() {
    let emitted = emit(ROOT_CLASS);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:rootClass", HOLDER), ("sh:rootClass", HOLDER),])
    );
}

#[test]
fn json_schema_projects_some_value() {
    let emitted = emit(SOME_VALUE);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:someValue",
            HOLDER,
            "an at-least-one-value-conforms condition over a property's values has no projection in this emitter"
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"anyOf":[{"$comment":"a sh:someValue constraint on property ex:subject was dropped (no projection in this emitter)"},{"items":{"$comment":"a sh:someValue constraint on property ex:subject was dropped (no projection in this emitter)"},"type":"array"}]}"#
        )
    );
    assert_eq!(holder["$comment"], Value::Null);
}

#[test]
fn typescript_projects_some_value() {
    let emitted = emit(SOME_VALUE);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = ({\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n} & {\n    readonly \"@annotation\"?: Annotation;\n    readonly \"@id\"?: string;\n    readonly \"@type\"?: (string | readonly (string)[]);\n    readonly \"ex:name\": JsonValue;\n    readonly [key: string]: JsonValue;\n  });\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(source_codes(&emitted), owned2(&[("sh:someValue", HOLDER),]));
}

#[test]
fn pydantic_projects_some_value() {
    let emitted = emit(SOME_VALUE);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any | list[Any] = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(
        holder_losses(&emitted.pydantic_losses),
        owned2(&[("intersection-validation-widened", "#/$defs/Holder/allOf"),])
    );
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(source_codes(&emitted), owned2(&[("sh:someValue", HOLDER),]));
}

#[test]
fn graphql_projects_some_value() {
    let emitted = emit(SOME_VALUE);
    assert_eq!(gql_type(&emitted.graphql, "Holder").as_deref(), None);
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            ("custom-scalar-validation-delegated", "#/$defs/Holder"),
            ("intersection-validation-delegated", "#/$defs/Holder/allOf"),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:someValue", HOLDER),]));
}

#[test]
fn linkml_projects_some_value() {
    let emitted = emit(SOME_VALUE);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(
            r#"{"alias":"ex:subject","any_of":[{"range":"string"},{"list_elements_ordered":true,"multivalued":true,"range":"string"}],"required":false,"slot_uri":"ex:subject"}"#
        )
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/allOf/0/properties/ex:name/anyOf/0"
            ),
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/allOf/0/properties/ex:name/anyOf/1/items"
            ),
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/properties/ex:subject/anyOf/0"
            ),
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/properties/ex:subject/anyOf/1/items"
            ),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:someValue", HOLDER),]));
}

#[test]
fn json_schema_projects_unique_values_for() {
    let emitted = emit(UNIQUE_VALUES_FOR);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:uniqueValuesFor",
                HOLDER,
                "a uniqueness condition on the values of (<https://example.org/serial>) across every target node of the shape has no projection in a schema that judges one instance alone"
            ),
            (
                "sh:uniqueValuesFor",
                HOLDER,
                "a uniqueness condition on the values of (<https://example.org/a> <https://example.org/b>) across every target node of the shape has no projection in a schema that judges one instance alone"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(
            r#"{"$comment":"a sh:uniqueValuesFor (<https://example.org/serial>) constraint on property ex:subject was dropped (no projection in this emitter)"}"#
        )
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:uniqueValuesFor (<https://example.org/a> <https://example.org/b>) constraint was dropped (no projection in this emitter)""#
        )
    );
}

#[test]
fn typescript_projects_unique_values_for() {
    let emitted = emit(UNIQUE_VALUES_FOR);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:uniqueValuesFor", HOLDER),
            ("sh:uniqueValuesFor", HOLDER),
        ])
    );
}

#[test]
fn pydantic_projects_unique_values_for() {
    let emitted = emit(UNIQUE_VALUES_FOR);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:uniqueValuesFor", HOLDER),
            ("sh:uniqueValuesFor", HOLDER),
        ])
    );
}

#[test]
fn graphql_projects_unique_values_for() {
    let emitted = emit(UNIQUE_VALUES_FOR);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:uniqueValuesFor", HOLDER),
            ("sh:uniqueValuesFor", HOLDER),
        ])
    );
}

#[test]
fn linkml_projects_unique_values_for() {
    let emitted = emit(UNIQUE_VALUES_FOR);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[
            ("sh:uniqueValuesFor", HOLDER),
            ("sh:uniqueValuesFor", HOLDER),
        ])
    );
}

#[test]
fn json_schema_projects_closed_by_types() {
    let emitted = emit(CLOSED_BY_TYPES);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:closed sh:ByTypes",
            HOLDER,
            "a set of permitted properties chosen by each instance's own rdf:type values has no projection in an object schema's fixed key set"
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(r#"{"type":"string"}"#)
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a node-level sh:closed sh:ByTypes constraint was dropped (the permitted properties depend on each instance's types)""#
        )
    );
}

#[test]
fn typescript_projects_closed_by_types() {
    let emitted = emit(CLOSED_BY_TYPES);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: string;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:closed sh:ByTypes", HOLDER),])
    );
}

#[test]
fn pydantic_projects_closed_by_types() {
    let emitted = emit(CLOSED_BY_TYPES);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: StrictStr = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:closed sh:ByTypes", HOLDER),])
    );
}

#[test]
fn graphql_projects_closed_by_types() {
    let emitted = emit(CLOSED_BY_TYPES);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: String\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:closed sh:ByTypes", HOLDER),])
    );
}

#[test]
fn linkml_projects_closed_by_types() {
    let emitted = emit(CLOSED_BY_TYPES);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(holder_losses(&emitted.linkml_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:closed sh:ByTypes", HOLDER),])
    );
}

#[test]
fn json_schema_projects_computed_values() {
    let emitted = emit(COMPUTED_VALUES);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:defaultValue",
                HOLDER,
                "a property shape's computed value nodes have no JSON Schema equivalent"
            ),
            (
                "sh:values",
                HOLDER,
                "a property shape's computed value nodes have no JSON Schema equivalent"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(holder["properties"]["ex:subject"], Value::Null);
    assert_eq!(
        holder["$comment"],
        json(
            r#""the property shape on ex:fallback was dropped (its value nodes are computed by sh:values / sh:defaultValue, which a JSON document does not carry); the property shape on ex:subject was dropped (its value nodes are computed by sh:values / sh:defaultValue, which a JSON document does not carry)""#
        )
    );
    assert_eq!(
        holder["properties"]["ex:kept"],
        json(r#"{"type":"string"}"#)
    );
}

#[test]
fn typescript_projects_computed_values() {
    let emitted = emit(COMPUTED_VALUES);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:kept\"?: string;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:defaultValue", HOLDER), ("sh:values", HOLDER),])
    );
}

#[test]
fn pydantic_projects_computed_values() {
    let emitted = emit(COMPUTED_VALUES);
    assert_eq!(py_field(&emitted.pydantic, "ex:subject").as_deref(), None);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:kept").as_deref(),
        Some("    kept: StrictStr = Field(default=None, alias=\"ex:kept\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:defaultValue", HOLDER), ("sh:values", HOLDER),])
    );
}

#[test]
fn graphql_projects_computed_values() {
    let emitted = emit(COMPUTED_VALUES);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exKept: String\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/ex:kept"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:defaultValue", HOLDER), ("sh:values", HOLDER),])
    );
}

#[test]
fn linkml_projects_computed_values() {
    let emitted = emit(COMPUTED_VALUES);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(holder["attributes"]["ex:subject"], Value::Null);
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder["attributes"]["ex:kept"],
        json(r#"{"alias":"ex:kept","range":"string","required":false,"slot_uri":"ex:kept"}"#)
    );
    assert_eq!(holder_losses(&emitted.linkml_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:defaultValue", HOLDER), ("sh:values", HOLDER),])
    );
}

#[test]
fn json_schema_projects_target_where() {
    let emitted = emit(TARGET_WHERE);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:targetWhere",
            "<https://example.org/WhereShape>",
            "A shape targeted via sh:targetWhere selects the nodes that conform to another shape, not a class extension; it has no closed-world JSON Schema $def and its constraints are not enforced by the emitted schema."
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(holder["properties"]["ex:subject"], json(r"{}"));
    assert_eq!(holder["$comment"], Value::Null);
    let defs: Vec<&String> = emitted.schema["$defs"]
        .as_object()
        .expect("$defs")
        .keys()
        .collect();
    assert_eq!(defs, ["Annotation", "Holder", "Node"]);
}

#[test]
fn typescript_projects_target_where() {
    let emitted = emit(TARGET_WHERE);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert!(!emitted.typescript.contains("Where") && !emitted.typescript.contains("Pinned"));
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetWhere", "<https://example.org/WhereShape>"),])
    );
}

#[test]
fn pydantic_projects_target_where() {
    let emitted = emit(TARGET_WHERE);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetWhere", "<https://example.org/WhereShape>"),])
    );
}

#[test]
fn graphql_projects_target_where() {
    let emitted = emit(TARGET_WHERE);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetWhere", "<https://example.org/WhereShape>"),])
    );
}

#[test]
fn linkml_projects_target_where() {
    let emitted = emit(TARGET_WHERE);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetWhere", "<https://example.org/WhereShape>"),])
    );
}

#[test]
fn json_schema_projects_node_expression_target() {
    let emitted = emit(NODE_EXPRESSION_TARGET);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:targetNode",
            "<https://example.org/PinnedShape>",
            "A shape targeted via a node-expression sh:targetNode selects the output nodes of that expression, not a class extension; it has no closed-world JSON Schema $def and its constraints are not enforced by the emitted schema."
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(holder["properties"]["ex:subject"], json(r"{}"));
    assert_eq!(holder["$comment"], Value::Null);
    let defs: Vec<&String> = emitted.schema["$defs"]
        .as_object()
        .expect("$defs")
        .keys()
        .collect();
    assert_eq!(defs, ["Annotation", "Holder", "Node"]);
}

#[test]
fn typescript_projects_node_expression_target() {
    let emitted = emit(NODE_EXPRESSION_TARGET);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert!(!emitted.typescript.contains("Where") && !emitted.typescript.contains("Pinned"));
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetNode", "<https://example.org/PinnedShape>"),])
    );
}

#[test]
fn pydantic_projects_node_expression_target() {
    let emitted = emit(NODE_EXPRESSION_TARGET);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetNode", "<https://example.org/PinnedShape>"),])
    );
}

#[test]
fn graphql_projects_node_expression_target() {
    let emitted = emit(NODE_EXPRESSION_TARGET);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetNode", "<https://example.org/PinnedShape>"),])
    );
}

#[test]
fn linkml_projects_node_expression_target() {
    let emitted = emit(NODE_EXPRESSION_TARGET);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:targetNode", "<https://example.org/PinnedShape>"),])
    );
}

#[test]
fn json_schema_projects_debug_severity() {
    let emitted = emit(DEBUG_SEVERITY);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[
            (
                "sh:severity",
                HOLDER,
                "a sh:datatype constraint whose results carry the severity <http://www.w3.org/ns/shacl#Debug>, outside the default conformance-disallow set, decides no conformance the schema states"
            ),
            (
                "sh:severity",
                HOLDER,
                "a sh:maxCount constraint whose results carry the severity <http://www.w3.org/ns/shacl#Debug>, outside the default conformance-disallow set, decides no conformance the schema states"
            ),
        ])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"],
        json(r#"{"anyOf":[{},{"items":{},"type":"array"}]}"#)
    );
    assert_eq!(
        holder["$comment"],
        json(
            r#""a sh:datatype constraint on property ex:subject was dropped (its severity <http://www.w3.org/ns/shacl#Debug> does not decide conformance); a sh:maxCount constraint on property ex:subject was dropped (its severity <http://www.w3.org/ns/shacl#Debug> does not decide conformance)""#
        )
    );
    assert_eq!(
        holder["properties"]["ex:kept"],
        json(r#"{"type":"string"}"#)
    );
}

#[test]
fn typescript_projects_debug_severity() {
    let emitted = emit(DEBUG_SEVERITY);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:kept\"?: string;\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:severity", HOLDER), ("sh:severity", HOLDER),])
    );
}

#[test]
fn pydantic_projects_debug_severity() {
    let emitted = emit(DEBUG_SEVERITY);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any | list[Any] = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(
        py_field(&emitted.pydantic, "ex:kept").as_deref(),
        Some("    kept: StrictStr = Field(default=None, alias=\"ex:kept\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:severity", HOLDER), ("sh:severity", HOLDER),])
    );
}

#[test]
fn graphql_projects_debug_severity() {
    let emitted = emit(DEBUG_SEVERITY);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exKept: String\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/ex:kept"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/ex:subject/anyOf"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:severity", HOLDER), ("sh:severity", HOLDER),])
    );
}

#[test]
fn linkml_projects_debug_severity() {
    let emitted = emit(DEBUG_SEVERITY);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(
            r#"{"alias":"ex:subject","any_of":[{"range":"string"},{"list_elements_ordered":true,"multivalued":true,"range":"string"}],"required":false,"slot_uri":"ex:subject"}"#
        )
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder["attributes"]["ex:kept"],
        json(r#"{"alias":"ex:kept","range":"string","required":false,"slot_uri":"ex:kept"}"#)
    );
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/properties/ex:subject/anyOf/0"
            ),
            (
                "keyword-validation-dropped",
                "#/$defs/Holder/properties/ex:subject/anyOf/1/items"
            ),
        ])
    );
    assert_eq!(
        source_codes(&emitted),
        owned2(&[("sh:severity", HOLDER), ("sh:severity", HOLDER),])
    );
}

#[test]
fn json_schema_projects_reifier_severity() {
    let emitted = emit(REIFIER_SEVERITY);
    assert_eq!(
        source_losses(&emitted),
        owned3(&[(
            "sh:severity",
            HOLDER,
            "a sh:datatype constraint whose results carry the severity <http://www.w3.org/ns/shacl#Trace>, outside the default conformance-disallow set, decides no conformance the schema states"
        ),])
    );
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(holder["properties"]["ex:subject"], json(r"{}"));
    assert_eq!(
        holder["$comment"],
        json(
            r#""a sh:datatype constraint on property ex:subject was dropped (its severity <http://www.w3.org/ns/shacl#Trace> does not decide conformance)""#
        )
    );
}

#[test]
fn typescript_projects_reifier_severity() {
    let emitted = emit(REIFIER_SEVERITY);
    assert_eq!(
        ts_type(&emitted.typescript, "Holder").as_deref(),
        Some(
            "export type Holder = {\n  readonly \"@annotation\"?: Annotation;\n  readonly \"@id\"?: string;\n  readonly \"@type\"?: (string | readonly (string)[]);\n  readonly \"ex:subject\"?: JsonValue;\n  readonly [key: string]: JsonValue;\n};\n"
        )
    );
    assert_eq!(holder_losses(&emitted.typescript_losses), owned2(&[]));
    assert_eq!(source_codes(&emitted), owned2(&[("sh:severity", HOLDER),]));
}

#[test]
fn pydantic_projects_reifier_severity() {
    let emitted = emit(REIFIER_SEVERITY);
    assert_eq!(
        py_field(&emitted.pydantic, "ex:subject").as_deref(),
        Some("    subject: Any = Field(default=None, alias=\"ex:subject\")")
    );
    assert_eq!(holder_losses(&emitted.pydantic_losses), owned2(&[]));
    assert_pydantic_keeps_comments(&emitted);
    assert_eq!(source_codes(&emitted), owned2(&[("sh:severity", HOLDER),]));
}

#[test]
fn graphql_projects_reifier_severity() {
    let emitted = emit(REIFIER_SEVERITY);
    assert_eq!(
        gql_type(&emitted.graphql, "Holder").as_deref(),
        Some(
            "type Holder {\n  annotation: RdfValue\n  exSubject: RdfValue\n  id: String\n  type: RdfValue\n}"
        )
    );
    assert_eq!(
        holder_losses(&emitted.graphql_losses),
        owned2(&[
            (
                "additional-properties-validation-narrowed",
                "#/$defs/Holder"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/@type"
            ),
            (
                "custom-scalar-validation-delegated",
                "#/$defs/Holder/properties/ex:subject"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@annotation"
            ),
            (
                "nullable-presence-validation-widened",
                "#/$defs/Holder/properties/@id"
            ),
            (
                "union-validation-delegated",
                "#/$defs/Holder/properties/@type/anyOf"
            ),
        ])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:severity", HOLDER),]));
}

#[test]
fn linkml_projects_reifier_severity() {
    let emitted = emit(REIFIER_SEVERITY);
    let holder = &emitted.linkml["classes"]["Holder"];
    assert_eq!(
        holder["attributes"]["ex:subject"],
        json(r#"{"alias":"ex:subject","range":"string","required":false,"slot_uri":"ex:subject"}"#)
    );
    assert_eq!(holder["extra_slots"], json(r#"{"allowed":true}"#));
    assert_eq!(
        holder_losses(&emitted.linkml_losses),
        owned2(&[(
            "keyword-validation-dropped",
            "#/$defs/Holder/properties/ex:subject"
        ),])
    );
    assert_eq!(source_codes(&emitted), owned2(&[("sh:severity", HOLDER),]));
}

// ── Observed behaviour ────────────────────────────────────────────────────────
//
// The fragments above pin what is emitted; these observe what it means. Each
// compares, over the same data graph, SHACL validation with a trusted external
// JSON Schema (draft 2020-12) validator judging the projected instance node
// against the compiled `Holder` definition, so a projection is shown to accept
// and reject exactly what validation does.

#[cfg(not(target_arch = "wasm32"))]
mod observed {
    use super::*;
    use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};

    /// Whether the compiled schema's `Holder` definition accepts the projected
    /// node `ex:h` of `data`, and whether `ex:h` conforms under SHACL.
    fn judge(shapes: &str, data: &str) -> (bool, bool) {
        let shapes_ttl = format!("{PREFIXES}{shapes}");
        let parsed = parse_shapes(&shapes_ttl, None).expect("shapes graph");
        let data = parse_turtle_to_dataset(&format!("{PREFIXES}{data}"), None).expect("data");
        let report =
            validate_dataset_with_shapes_graph(&data, &parsed, None).expect("validation runs");
        let compiled = compile(&parsed, &namespaces()).expect("schema compilation");
        let projected = purrdf_shapes::instance::project_graph(&data, &namespaces());
        let node = projected["@graph"]
            .as_array()
            .expect("@graph")
            .iter()
            .find(|node| node["@id"] == "ex:h")
            .expect("ex:h is projected")
            .clone();
        let schema: Value = serde_json::from_str(&compiled.schema_json).expect("schema JSON");
        let location = "mem:///holder.schema.json";
        let mut schemas = boon::Schemas::new();
        let mut compiler = boon::Compiler::new();
        compiler
            .add_resource(location, schema)
            .expect("schema registers");
        let holder = compiler
            .compile(&format!("{location}#/$defs/Holder"), &mut schemas)
            .expect("schema compiles under draft 2020-12");
        (schemas.validate(&node, holder).is_ok(), report.conforms)
    }

    /// `sh:singleLine true` rejects a literal whose lexical form holds a line
    /// feed, carriage return, vertical tab or form feed — as a bare string, a
    /// typed literal's `@value` or a language-tagged literal's — and accepts
    /// every other value: a tab, a number, an IRI.
    #[test]
    fn json_schema_single_line_agrees_with_validation() {
        let shapes = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:singleLine true ] .";
        for (value, conforms) in [
            (r#""one line""#, true),
            (r#""tab\there""#, true),
            ("42", true),
            ("ex:elsewhere", true),
            (r#""two\nlines""#, false),
            (r#""carriage\rreturn""#, false),
            (r#""vertical\u000Btab""#, false),
            (r#""form\u000Cfeed""#, false),
            (r#""typed\nvalue"^^ex:code"#, false),
            (r#""tagged\nvalue"@en"#, false),
        ] {
            let data = format!("ex:h a ex:Holder ; ex:subject {value} .");
            assert_eq!(judge(shapes, &data), (conforms, conforms), "{value}");
        }
    }

    /// The verdicts over each data value (one `ex:subject` value of `ex:h`).
    #[track_caller]
    fn agree(shapes: &str, cases: &[(&str, bool)]) {
        for &(value, conforms) in cases {
            let data = format!("ex:h a ex:Holder ; ex:subject {value} .");
            assert_eq!(judge(shapes, &data), (conforms, conforms), "{value}");
        }
    }

    /// `sh:pattern` judges the lexical form wherever the projection carries it —
    /// a bare string, a typed literal's `@value`, a language-tagged literal's
    /// `@value` — and rejects a blank node and a triple term, which have none;
    /// `sh:flags` is honoured through the translation (`m`, `s`, `x`, `q`).
    #[test]
    fn json_schema_pattern_agrees_with_validation() {
        let shapes = r#"ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:nodeKind sh:BlankNodeOrLiteral ;
                          sh:pattern "^A.C$" ; sh:flags "s" ] ."#;
        agree(
            shapes,
            &[
                (r#""ABC""#, true),
                (r#""A\nC""#, true),
                (r#""xyz""#, false),
                (r#""ABC"^^ex:code"#, true),
                (r#""xyz"^^ex:code"#, false),
                (r#""ABC"@en"#, true),
                (r#""xyz"@en"#, false),
                ("[]", false),
                ("<<( ex:a ex:b ex:c )>>", false),
            ],
        );
        // A numeric literal's lexical form is not in its projection (a bare JSON
        // number), so the pattern cannot judge it: recorded, and observed.
        assert_eq!(
            judge(shapes, "ex:h a ex:Holder ; ex:subject 42 ."),
            (true, false)
        );
        assert_eq!(
            source_codes(&emit(shapes)),
            owned2(&[("sh:pattern", HOLDER)])
        );
    }

    /// Where the other constraints admit an IRI, `sh:pattern` judges `str()` of
    /// it — the full IRI — which the projection's compacted `@id` does not carry:
    /// the part is recorded, and an IRI the pattern rejects is accepted by the
    /// schema. The literal and blank-node verdicts still agree.
    #[test]
    fn json_schema_pattern_on_iris_is_a_recorded_loss() {
        let shapes = r#"ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:nodeKind sh:IRIOrLiteral ;
                          sh:pattern "^https://example.org/" ] ."#;
        agree(
            shapes,
            &[
                ("ex:inside", true),
                (r#""https://example.org/x""#, true),
                (r#""elsewhere""#, false),
            ],
        );
        let data = "ex:h a ex:Holder ; ex:subject <urn:outside> .";
        assert_eq!(judge(shapes, data), (true, false));
        assert_eq!(
            source_losses(&emit(shapes)),
            owned3(&[(
                "sh:pattern",
                HOLDER,
                "the lexical form is not checked on IRI values (projected as a compacted @id, \
                 not the IRI string) or numeric and boolean literals (projected as JSON scalars \
                 without their lexical form)"
            )])
        );
        // With the values held to IRIs and strings, only the IRI part remains.
        let listed = r#"ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:pattern "^https://example.org/" ;
                          sh:in ( ex:inside <urn:outside> "https://example.org/x" ) ] ."#;
        agree(
            listed,
            &[("ex:inside", true), (r#""https://example.org/x""#, true)],
        );
        assert_eq!(
            judge(listed, "ex:h a ex:Holder ; ex:subject <urn:outside> ."),
            (true, false)
        );
        assert_eq!(
            source_losses(&emit(listed)),
            owned3(&[(
                "sh:pattern",
                HOLDER,
                "the lexical form is not checked on IRI values (projected as a compacted @id, \
                 not the IRI string)"
            )])
        );
    }

    /// A pattern whose `sh:flags` has no ECMA-262 translation (`i`, whose XPath
    /// case variants are not simple case folding) is not emitted: the loss is
    /// recorded rather than a pattern with another language.
    #[test]
    fn json_schema_untranslatable_flags_record_the_loss() {
        let shapes = r#"ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:datatype xsd:string ;
                          sh:pattern "^abc$" ; sh:flags "i" ] ."#;
        let emitted = emit(shapes);
        assert_eq!(
            emitted.schema["$defs"]["Holder"]["properties"]["ex:subject"]["anyOf"][0]["pattern"],
            Value::Null
        );
        let codes: Vec<(String, String)> = source_codes(&emitted);
        assert_eq!(codes, owned2(&[("sh:pattern", HOLDER)]));
        // The neighbour without the flag translates, and records nothing.
        let plain = shapes.replace(r#"; sh:flags "i" "#, "");
        assert!(emit(&plain).compiled.losses.is_empty());
        agree(&plain, &[(r#""abc""#, true), (r#""ABC""#, false)]);
    }

    /// `sh:minLength` and `sh:maxLength` count the lexical form's characters on a
    /// bare string, a typed literal and a language-tagged literal alike, and
    /// reject a blank node.
    #[test]
    fn json_schema_lengths_agree_with_validation() {
        let shapes = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:nodeKind sh:BlankNodeOrLiteral ;
                          sh:minLength 2 ; sh:maxLength 3 ] .";
        agree(
            shapes,
            &[
                (r#""ab""#, true),
                (r#""abcd""#, false),
                (r#""a""#, false),
                (r#""abc"^^ex:code"#, true),
                (r#""abcd"^^ex:code"#, false),
                (r#""a"^^ex:code"#, false),
                (r#""ab"@en"#, true),
                (r#""abcd"@en"#, false),
                ("[]", false),
            ],
        );
        assert_eq!(
            judge(shapes, "ex:h a ex:Holder ; ex:subject 12345 ."),
            (true, false)
        );
        assert_eq!(
            source_codes(&emit(shapes)),
            owned2(&[("sh:maxLength", HOLDER), ("sh:minLength", HOLDER)])
        );
    }

    /// `sh:languageIn` compares tags case-insensitively at subtag boundaries and
    /// admits only language-tagged literals, alone or beside another value-type
    /// constraint — a conjunction, not a choice between them.
    #[test]
    fn json_schema_language_in_agrees_with_validation() {
        let shapes = r#"ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:languageIn ( "en" "fr-CA" ) ;
                          sh:nodeKind sh:Literal ] ."#;
        agree(
            shapes,
            &[
                (r#""hi"@en"#, true),
                (r#""hi"@EN-gb"#, true),
                (r#""salut"@fr-ca"#, true),
                (r#""salut"@fr"#, false),
                (r#""hi"@eng"#, false),
                (r#""hi""#, false),
                ("ex:iri", false),
            ],
        );
    }

    /// A numeric bound compares numbers only: a string, a boolean, an IRI or a
    /// language-tagged literal is not comparable and fails, as in validation.
    #[test]
    fn json_schema_numeric_bounds_agree_with_validation() {
        let shapes = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:minInclusive 1 ; sh:maxExclusive 10 ] .";
        agree(
            shapes,
            &[
                ("5", true),
                ("1", true),
                ("10", false),
                ("0", false),
                (r#""5""#, false),
                ("true", false),
                ("ex:iri", false),
                (r#""5"@en"#, false),
            ],
        );
    }

    /// `sh:hasValue` is existential: a property with several values conforms when
    /// one of them is the term — the schema must not require every value to be.
    #[test]
    fn json_schema_has_value_agrees_with_validation() {
        let shapes = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:hasValue ex:a ] .";
        agree(
            shapes,
            &[
                ("ex:a", true),
                ("ex:a , ex:b", true),
                ("ex:b", false),
                ("ex:b , ex:c", false),
            ],
        );
        let absent = "ex:h a ex:Holder .";
        assert_eq!(judge(shapes, absent), (false, false));
    }

    /// `sh:nodeKind sh:IRI` rejects a blank node and `sh:BlankNode` an IRI; a
    /// numeric literal is a literal; a triple term is its own kind.
    #[test]
    fn json_schema_node_kinds_agree_with_validation() {
        for (kind, cases) in [
            (
                "sh:IRI",
                [
                    ("ex:a", true),
                    ("[]", false),
                    ("42", false),
                    ("<<( ex:a ex:b ex:c )>>", false),
                ],
            ),
            (
                "sh:BlankNode",
                [
                    ("ex:a", false),
                    ("[]", true),
                    ("42", false),
                    ("<<( ex:a ex:b ex:c )>>", false),
                ],
            ),
            (
                "sh:Literal",
                [
                    ("ex:a", false),
                    ("[]", false),
                    ("42", true),
                    ("<<( ex:a ex:b ex:c )>>", false),
                ],
            ),
            (
                "sh:TripleTerm",
                [
                    ("ex:a", false),
                    ("[]", false),
                    ("42", false),
                    ("<<( ex:a ex:b ex:c )>>", true),
                ],
            ),
        ] {
            let shapes = format!(
                "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
                   sh:property [ sh:path ex:subject ; sh:nodeKind {kind} ] ."
            );
            agree(&shapes, &cases);
            assert!(emit(&shapes).compiled.losses.is_empty(), "{kind}");
        }
    }

    /// A value is typed by exactly its datatype: a bare string is an `xsd:string`
    /// literal, so an `xsd:date` shape rejects it, and a typed literal of another
    /// datatype fails too.
    #[test]
    fn json_schema_datatypes_agree_with_validation() {
        let shapes = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:datatype xsd:date ] .";
        agree(
            shapes,
            &[
                (r#""2026-01-01"^^xsd:date"#, true),
                (r#""2026-01-01""#, false),
                (r#""2026-01-01"^^ex:code"#, false),
                ("ex:iri", false),
            ],
        );
        let booleans = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:datatype xsd:boolean ] .";
        agree(
            booleans,
            &[
                ("true", true),
                (r#"" true "^^xsd:boolean"#, true),
                (r#""yes"^^xsd:boolean"#, false),
                (r#""true""#, false),
            ],
        );
        assert!(emit(booleans).compiled.losses.is_empty());
    }

    /// Two `sh:in` lists on one path (from two property shapes) admit only their
    /// common members.
    #[test]
    fn json_schema_in_lists_intersect() {
        let shapes = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:in ( ex:a ex:b ) ] ;
            sh:property [ sh:path ex:subject ; sh:in ( ex:b ex:c ) ] .";
        agree(shapes, &[("ex:b", true), ("ex:a", false), ("ex:c", false)]);
    }

    /// A deactivated property shape is ignored by validation, and by the schema;
    /// the active control rejects the same value in both.
    #[test]
    fn json_schema_deactivated_property_shape_agrees_with_validation() {
        let data = "ex:h a ex:Holder ; ex:subject 5 .";
        let active = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:closed true ; sh:ignoredProperties ( rdf:type ) ;
            sh:property [ sh:path ex:subject ; sh:datatype xsd:string ] .";
        let deactivated = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ; sh:closed true ; sh:ignoredProperties ( rdf:type ) ;
            sh:property [ sh:path ex:subject ; sh:datatype xsd:string ; sh:deactivated true ] .";
        assert_eq!(judge(active, data), (false, false));
        assert_eq!(judge(deactivated, data), (true, true));
        assert!(
            emit(deactivated).compiled.losses.is_empty(),
            "an ignored shape drops nothing"
        );
    }

    /// A constraint of severity `sh:Debug` (on its shape) or `sh:Trace` (on a
    /// reifier of its triple) does not decide conformance under the default
    /// conformance-disallow set, and the schema does not enforce it; the
    /// `sh:Violation` control is enforced by both.
    #[test]
    fn json_schema_severity_agrees_with_validation() {
        let data = "ex:h a ex:Holder ; ex:subject 5 .";
        for (constraint, conforms) in [
            ("sh:datatype xsd:string", false),
            ("sh:datatype xsd:string ; sh:severity sh:Warning", false),
            ("sh:datatype xsd:string ; sh:severity sh:Info", false),
            ("sh:datatype xsd:string ; sh:severity sh:Debug", true),
            ("sh:datatype xsd:string ; sh:severity sh:Trace", true),
            ("sh:datatype xsd:string {| sh:severity sh:Trace |}", true),
            (
                "sh:datatype xsd:string {| sh:severity sh:Violation |} ; sh:severity sh:Debug",
                false,
            ),
        ] {
            let shapes = format!(
                "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
                   sh:property [ sh:path ex:subject ; {constraint} ] ."
            );
            assert_eq!(judge(&shapes, data), (conforms, conforms), "{constraint}");
        }
    }

    /// `sh:closed true` projects as a closed object and `sh:closed sh:ByTypes`
    /// leaves it open: an undeclared property is rejected by the first in both
    /// the schema and validation, and accepted by the schema under the second —
    /// which records the widening, since validation (with no type declaring the
    /// property) rejects it.
    #[test]
    fn json_schema_closed_by_types_is_recorded_where_declared_closure_projects() {
        let data = "ex:h a ex:Holder ; ex:subject \"s\" ; ex:stray \"x\" .";
        let declared = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:closed true ; sh:ignoredProperties ( rdf:type ) ;
            sh:property [ sh:path ex:subject ] .";
        let by_types = "ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:closed sh:ByTypes ; sh:property [ sh:path ex:subject ] .";
        assert_eq!(judge(declared, data), (false, false));
        assert!(emit(declared).compiled.losses.is_empty());
        assert_eq!(judge(by_types, data), (true, false));
        assert_eq!(
            source_codes(&emit(by_types)),
            owned2(&[("sh:closed sh:ByTypes", HOLDER)])
        );
    }
}

/// The constraints a property's value schema cannot judge — each value is a
/// node reference or a literal — record their own codes and a `$comment`, as do
/// a nested property shape, a reifier shape, `sh:reificationRequired` and a
/// property shape on a path that is not one predicate. `sh:uniqueLang false`
/// checks nothing and records nothing.
#[test]
fn json_schema_records_shape_based_property_constraints() {
    let emitted = emit(
        r"
        ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:property [ sh:path ex:subject ; sh:node ex:Inner ] ;
            sh:property [ sh:path ex:either ; sh:or ( [ sh:datatype xsd:string ] [ sh:datatype xsd:integer ] ) ] ;
            sh:property [ sh:path ex:both ; sh:and ( [ sh:minLength 1 ] ) ] ;
            sh:property [ sh:path ex:one ; sh:xone ( [ sh:datatype xsd:string ] ) ] ;
            sh:property [ sh:path ex:some ; sh:qualifiedValueShape ex:Inner ; sh:qualifiedMinCount 1 ] ;
            sh:property [ sh:path ex:label ; sh:uniqueLang true ] ;
            sh:property [ sh:path ex:quiet ; sh:uniqueLang false ] ;
            sh:property [ sh:path ex:shut ; sh:closed true ] ;
            sh:property [ sh:path ex:outer ; sh:property [ sh:path ex:inner ; sh:minCount 1 ] ] ;
            sh:property [ sh:path ex:claim ; sh:reifierShape ex:Inner ; sh:reificationRequired true ] ;
            sh:property [ sh:path [ sh:inversePath ex:parent ] ; sh:minCount 1 ] .
        ex:Inner a sh:NodeShape ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        ",
    );
    let mut codes: Vec<String> = source_codes(&emitted)
        .into_iter()
        .map(|(code, _)| code)
        .collect();
    codes.sort();
    assert_eq!(
        codes,
        [
            "sh:and",
            "sh:closed",
            "sh:node",
            "sh:or",
            "sh:path",
            "sh:property",
            "sh:qualifiedValueShape",
            "sh:reificationRequired",
            "sh:reifierShape",
            "sh:uniqueLang",
            "sh:xone",
        ]
    );
    purrdf::loss::assert_ledger_sound(&emitted.compiled.losses, "shacl", "json-schema");
    let holder = &emitted.schema["$defs"]["Holder"];
    assert_eq!(
        holder["properties"]["ex:subject"]["anyOf"][0]["$comment"],
        "a sh:node constraint on property ex:subject was dropped (no projection in its value schema)"
    );
    assert!(
        !holder["properties"]["ex:quiet"]
            .to_string()
            .contains("$comment"),
        "sh:uniqueLang false drops nothing: {holder}"
    );
}

/// On a node shape, a constraint judging the focus node itself — here its
/// value, lexical form and range — records its own code; a node-level
/// `sh:singleLine` holds of every subject and records nothing.
#[test]
fn json_schema_records_focus_node_constraints() {
    let emitted = emit(
        r#"
        ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
            sh:in ( ex:a ex:b ) ; sh:pattern "^https:" ; sh:minInclusive 1 ; sh:singleLine true .
        "#,
    );
    let mut codes: Vec<String> = source_codes(&emitted)
        .into_iter()
        .map(|(code, _)| code)
        .collect();
    codes.sort();
    assert_eq!(codes, ["sh:in", "sh:minInclusive", "sh:pattern"]);
    purrdf::loss::assert_ledger_sound(&emitted.compiled.losses, "shacl", "json-schema");
}

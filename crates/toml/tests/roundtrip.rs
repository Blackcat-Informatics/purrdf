// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native grammar, source-cover, identity and real-consumer acceptance.

use purrdf_toml::{
    Bounds, Kind, Profile, Scalar, SourceDocument, TomlError, Vocabulary, analyze, decode_document,
    project,
};

const SOURCE: &str = "https://example.org/ledger.toml";
const LEDGER: &str = "# authored ledger\r\nname = 'Census'\r\nquoted.\"a.b\" = \"\\x41\\u0042\\U00000043\\e\"\r\n[[job]]\r\nid = 'one'\r\nproviders = [{ name = 'native', enabled = true, },]\r\n[job.variant]\r\ncount = 0x7fff\r\n[[job]]\r\nid = 'two'\r\nempty = []\r\n";

fn all_forms_round_trip_through_native_rdf() {
    let profile = Profile::standard().unwrap();
    for text in [
        LEDGER,
        "",
        "# comment\r\n",
        r#"
str = "plain"
literal = 'C:\path'
multi = """
one\
    two"""
raw = '''
it's literal'''
quotes = """"quoted""""
min = -9223372036854775808
max = 9223372036854775807
octal = 0o755
binary = 0b1_010
float = +1_2.3_4e-0_2
negative_zero = -0.0
infinity = -inf
not_number = +nan
negative_nan = -nan
date = 2024-02-29
time = 07:32
local = 1979-05-27 07:32
offset = 1979-05-27t07:32-07:00
precision = 07:32:00.123456789012345
leap_offset = 2000-01-01t23:59:60z
leap_local = 2000-01-01 23:59:60
leap_time = 23:59:60
mixed = [[1, 2], { quoted."a/b" = true, }, 'end']
inline = {
  x = 1, # TOML 1.1
}
[super.child]
key = false
[super]
empty = {}
"#,
    ] {
        let model = analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes(),
            },
            &profile,
        )
        .unwrap();
        let dataset = project(&model, &profile).unwrap();
        assert_eq!(
            decode_document(&dataset, model.id(), &profile).unwrap(),
            text
        );
        let encoded = purrdf_rdf::serialize_dataset_to_format(
            &dataset,
            purrdf_rdf::NativeRdfFormat::Turtle,
            None,
        )
        .unwrap();
        let parsed = purrdf_rdf::parse_dataset(&encoded.bytes, "text/turtle", None).unwrap();
        assert_eq!(
            decode_document(&parsed, model.id(), &profile).unwrap(),
            text
        );
    }
}

fn declarations_and_typed_semantics_keep_original_ownership() {
    let profile = Profile::standard().unwrap();
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: LEDGER.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    let jobs: Vec<_> = model
        .nodes()
        .iter()
        .filter(|node| {
            node.path.starts_with("/job/")
                && node.key.as_ref().is_some_and(|key| key.decoded == "id")
        })
        .collect();
    assert_eq!(jobs.len(), 2);
    assert_ne!(jobs[0].parent, jobs[1].parent);
    assert_eq!(jobs[0].kind, Kind::Scalar(Scalar::String("one".into())));
    assert_eq!(jobs[1].kind, Kind::Scalar(Scalar::String("two".into())));
    assert!(model.nodes().iter().any(|node| node.path == "/quoted/a.b"
        && node.kind == Kind::Scalar(Scalar::String("ABC\u{1b}".into()))));
    assert!(
        model
            .keys()
            .windows(2)
            .all(|keys| keys[0].span.end <= keys[1].span.start)
    );
    // TOML leap seconds cannot be represented by the XSD temporal lexical spaces.
    for (text, lexical, local) in [
        (
            "x=2000-01-01t23:59:60z",
            "2000-01-01T23:59:60Z",
            "OffsetDateTime",
        ),
        (
            "x=2000-01-01 23:59:60",
            "2000-01-01T23:59:60",
            "LocalDateTime",
        ),
        ("x=23:59:60", "23:59:60", "LocalTime"),
        ("x=2000-01-01", "2000-01-01", "LocalDate"),
    ] {
        let model = analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes(),
            },
            &profile,
        )
        .unwrap();
        let dataset = project(&model, &profile).unwrap();
        assert!(dataset.quads().enumerate().any(|(index, row)| {
            let statement = dataset.to_owned_quad(index, row);
            statement.predicate == profile.vocabulary().term("value").unwrap()
                && matches!(statement.object, purrdf_core::RdfTerm::Literal(literal)
                    if literal.lexical_form == lexical
                        && literal.datatype.as_deref() == Some(profile.vocabulary().term(local).unwrap().as_str()))
        }));
        assert_eq!(
            decode_document(&dataset, model.id(), &profile).unwrap(),
            text
        );
    }
}

fn invalid_forms_and_valid_neighbours_obey_toml_boundaries() {
    let profile = Profile::standard().unwrap();
    for text in [
        "a = 1\na = 2",
        "[a]\n[a]",
        "a.b = 1\n[a]",
        "a = {}\na.b = 1",
        "a=[]\n[[a]]",
        "[a.b]\n[[a]]",
        "a = 01",
        "a = 0_1",
        "a = 1__2",
        "a = 0x_1",
        "a = +0x1",
        "a = 1.",
        "a = .1",
        "a = 1e",
        "a = 2023-02-29",
        "a = 07:60",
        "a = 07:32Z",
        "a = \"\\'\"",
        "a = '\u{7f}'",
        "a=1\rb=2",
        "a = [1 2]",
        "a=\"open\n",
        "a=1\nb =",
        "a = 9223372036854775808",
    ] {
        assert!(
            analyze(
                SourceDocument {
                    id: SOURCE,
                    bytes: text.as_bytes()
                },
                &profile
            )
            .is_err(),
            "accepted invalid {text:?}"
        );
    }
    for text in [
        "a.b=1\na.c=2",
        "[a.b]\n[a]",
        "a={b=1,c={d=2}}",
        "a=0\nb=1_000",
        "a=2024-02-29",
        "a=07:59",
        "a='\t'",
        "a=[1,2,]",
        "a=1\r\nb=2",
        "[[a]]\n[a.b]\nx=1\n[[a]]\n[a.b]\nx=2",
    ] {
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: text.as_bytes(),
            },
            &profile,
        )
        .unwrap_or_else(|error| panic!("valid {text:?}: {error}"));
    }
}

fn depth_nodes_and_model_bounds_refuse_without_native_recursion() {
    let bounds = Bounds {
        max_depth: 512,
        ..Bounds::standard()
    };
    let profile = Profile::new("deep", 1, Vocabulary::standard().unwrap(), bounds).unwrap();
    let text = format!("value={}0{}", "[".repeat(511), "]".repeat(511));
    let owned = text;
    let copied = profile.clone();
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(move || {
            let model = analyze(
                SourceDocument {
                    id: SOURCE,
                    bytes: owned.as_bytes(),
                },
                &copied,
            )
            .unwrap();
            drop(model);
        })
        .unwrap()
        .join()
        .unwrap();
    let refused = format!("value={}0{}", "[".repeat(512), "]".repeat(512));
    assert!(matches!(
        analyze(
            SourceDocument {
                id: SOURCE,
                bytes: refused.as_bytes()
            },
            &profile
        ),
        Err(TomlError::Limit {
            resource: "depth",
            ..
        })
    ));
    for bounds in [
        Bounds {
            max_source_bytes: 2,
            ..bounds
        },
        Bounds {
            max_nodes: 1,
            ..bounds
        },
        Bounds {
            max_model_bytes: 1,
            ..bounds
        },
    ] {
        let profile = Profile::new("bounded", 1, Vocabulary::standard().unwrap(), bounds).unwrap();
        assert!(matches!(
            analyze(
                SourceDocument {
                    id: SOURCE,
                    bytes: b"a=1"
                },
                &profile
            ),
            Err(TomlError::Limit { .. })
        ));
    }
}

purrdf_testkit::harness_main!(
    all_forms_round_trip_through_native_rdf,
    declarations_and_typed_semantics_keep_original_ownership,
    invalid_forms_and_valid_neighbours_obey_toml_boundaries,
    depth_nodes_and_model_bounds_refuse_without_native_recursion,
    frozen_toml_conformance_checks_grammar_and_typed_values,
    a_ledger_job_and_its_provider_are_queried_through_the_real_engine,
    selected_documents_refuse_forged_structure_and_incomplete_covers,
);

fn a_ledger_job_and_its_provider_are_queried_through_the_real_engine() {
    use purrdf_core::{SparqlResult, TermValue};
    use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
    let profile = Profile::standard().unwrap();
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: LEDGER.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    let dataset = project(&model, &profile).unwrap();
    let engine = NativeSparqlEngine::new();
    let query = engine
        .prepare_query(
            r#"
        PREFIX t: <https://w3id.org/purrdf/toml#>
        SELECT ?id ?provider WHERE {
          ?job t:kind "table-element" .
          ?field t:parent ?job; t:key "id"; t:value ?id .
          OPTIONAL {
            ?providers t:parent ?job; t:key "providers" .
            ?entry t:parent ?providers .
            ?name t:parent ?entry; t:key "name"; t:value ?provider .
          }
        } ORDER BY ?id"#,
            None,
        )
        .unwrap();
    let SparqlResult::Solutions { rows, .. } = engine
        .query_prepared_view(&*dataset, &query, &[], QueryOptions::EMPTY)
        .unwrap()
    else {
        panic!("SELECT produces solutions");
    };
    assert_eq!(
        rows,
        vec![
            vec![
                Some(TermValue::simple_literal("one")),
                Some(TermValue::simple_literal("native"))
            ],
            vec![Some(TermValue::simple_literal("two")), None]
        ]
    );
}

fn selected_documents_refuse_forged_structure_and_incomplete_covers() {
    use purrdf_core::{RdfLiteral, RdfTerm, ir::RdfDatasetBuilder};
    let profile = Profile::standard().unwrap();
    let model = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: LEDGER.as_bytes(),
        },
        &profile,
    )
    .unwrap();
    let dataset = project(&model, &profile).unwrap();
    let base = profile.vocabulary().base();
    // Remove each required fact, then change each semantic/identity field while
    // leaving cover bytes intact. Both the byte and semantic laws must hold.
    for local in [
        "source",
        "sourceDigest",
        "profile",
        "document",
        "byteStart",
        "byteEnd",
        "verbatim",
        "kind",
        "path",
        "parent",
        "key",
        "value",
        "occurrence",
        "ordinal",
        "size",
        "node",
        "header",
    ] {
        for change in [false, true] {
            let mut builder = RdfDatasetBuilder::new();
            let predicate = format!("{base}{local}");
            let mut edited = false;
            for (index, row) in dataset.quads().enumerate() {
                let mut statement = dataset.to_owned_quad(index, row);
                if !edited && statement.predicate == predicate {
                    edited = true;
                    if !change {
                        continue;
                    }
                    statement.object = RdfTerm::Literal(RdfLiteral::simple("forged"));
                }
                builder.push_owned_quad(&statement).unwrap();
            }
            assert!(edited, "fixture must exercise {local}");
            let tampered = builder.freeze().unwrap();
            assert!(
                decode_document(&tampered, model.id(), &profile).is_err(),
                "accepted forged {local}, change={change}"
            );
        }
    }
    let second = analyze(
        SourceDocument {
            id: SOURCE,
            bytes: b"name = 'revision two'\n",
        },
        &profile,
    )
    .unwrap();
    let second_graph = project(&second, &profile).unwrap();
    let mut combined = RdfDatasetBuilder::new();
    for graph in [&dataset, &second_graph] {
        for (index, row) in graph.quads().enumerate() {
            combined
                .push_owned_quad(&graph.to_owned_quad(index, row))
                .unwrap();
        }
    }
    let combined = combined.freeze().unwrap();
    assert_eq!(
        decode_document(&combined, model.id(), &profile).unwrap(),
        LEDGER
    );
    assert_eq!(
        decode_document(&combined, second.id(), &profile).unwrap(),
        second.text()
    );
    assert!(decode_document(&combined, "https://example.org/absent", &profile).is_err());
    let other = Profile::new(
        "other",
        1,
        Vocabulary::standard().unwrap(),
        Bounds::standard(),
    )
    .unwrap();
    assert!(decode_document(&combined, model.id(), &other).is_err());
    assert!(project(&model, &other).is_err());

    // RDF 1.2 role projections share fact identity, while a conflicting fact or
    // invented owner in a native side table still belongs to the closed graph.
    for forged in [false, true] {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_dataset(&dataset).unwrap();
        builder
            .push_owned_annotation(&purrdf_core::RdfAnnotation::new(
                RdfTerm::Iri(format!("{}#v0", model.id())),
                format!("{base}path"),
                RdfTerm::Literal(RdfLiteral::simple(if forged { "/forged" } else { "" })),
            ))
            .unwrap();
        let side_table = builder.freeze().unwrap();
        assert_eq!(
            decode_document(&side_table, model.id(), &profile).is_err(),
            forged
        );
    }
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset).unwrap();
    builder
        .push_owned_annotation(&purrdf_core::RdfAnnotation::new(
            RdfTerm::Iri("https://example.org/unknown-owner".to_owned()),
            format!("{base}document"),
            RdfTerm::Iri(model.id().to_owned()),
        ))
        .unwrap();
    assert!(decode_document(&builder.freeze().unwrap(), model.id(), &profile).is_err());
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&dataset).unwrap();
    let graph = builder.intern_iri(model.id()).unwrap();
    builder.declare_named_graph(graph).unwrap();
    assert!(decode_document(&builder.freeze().unwrap(), model.id(), &profile).is_err());
}

fn frozen_toml_conformance_checks_grammar_and_typed_values() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/suite");
    let manifest = std::fs::read_to_string(root.join("files-toml-1.1.0")).unwrap();
    let profile = Profile::standard().unwrap();
    let mut failures = Vec::new();
    let (mut valid, mut invalid) = (0, 0);
    for name in manifest.lines().filter(|name| {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"))
    }) {
        let path = root.join(name);
        let bytes = std::fs::read(&path).unwrap();
        let result = analyze(
            SourceDocument {
                id: SOURCE,
                bytes: &bytes,
            },
            &profile,
        );
        if name.starts_with("invalid/") {
            invalid += 1;
            if result.is_ok() {
                failures.push(format!("accepted invalid {name}"));
            }
        } else {
            valid += 1;
            match result {
                Err(error) => failures.push(format!("refused valid {name}: {error}")),
                Ok(document) => {
                    let expected = purrdf_lex::json::read(
                        &std::fs::read_to_string(path.with_extension("json")).unwrap(),
                    )
                    .unwrap();
                    if let Err(reason) = check_tagged(&document, &expected) {
                        failures.push(format!("wrong value {name}: {reason}"));
                    }
                }
            }
        }
    }
    assert_eq!((valid, invalid), (218, 494));
    assert!(
        failures.is_empty(),
        "{} conformance failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn check_tagged(
    document: &purrdf_toml::Document<'_>,
    expected: &purrdf_lex::json::Value,
) -> Result<(), String> {
    use purrdf_lex::json::Value;
    use purrdf_toml::Float;
    use purrdf_xsd::rfc3339::{self, LeapSecond, Separator};
    let stamp =
        |text: &str| rfc3339::parse(text, Separator::GtsSpaceAllowed, LeapSecond::AnyDay).unwrap();
    let mut pending = vec![(0, expected)];
    while let Some((index, expected)) = pending.pop() {
        let node = &document.nodes()[index];
        let good = match expected {
            Value::Array(items) if matches!(node.kind, Kind::Array | Kind::ArrayOfTables) => {
                if items.len() != node.children.len() {
                    return Err(format!("{} array size", node.path));
                }
                pending.extend(node.children.iter().copied().zip(items));
                true
            }
            Value::Object(entries) if expected["type"].as_str().is_none() => {
                if !matches!(node.kind, Kind::Table(_)) || entries.len() != node.children.len() {
                    return Err(format!("{} table size/kind", node.path));
                }
                for (name, value) in entries.iter() {
                    let child = node
                        .children
                        .iter()
                        .find(|&&id| {
                            document.nodes()[id]
                                .key
                                .as_ref()
                                .is_some_and(|key| key.decoded == *name)
                        })
                        .ok_or_else(|| format!("missing key {name}"))?;
                    pending.push((*child, value));
                }
                true
            }
            Value::Object(_) => {
                let text = expected["value"].as_str().ok_or("tagged scalar text")?;
                match (&node.kind, expected["type"].as_str().ok_or("tagged type")?) {
                    (Kind::Scalar(Scalar::String(value)), "string") => value == text,
                    (Kind::Scalar(Scalar::Integer(value)), "integer") => {
                        *value == text.parse::<i64>().unwrap()
                    }
                    (Kind::Scalar(Scalar::Boolean(value)), "bool") => {
                        *value == text.parse::<bool>().unwrap()
                    }
                    (Kind::Scalar(Scalar::Float(Float::Nan { .. })), "float") => {
                        text.ends_with("nan")
                    }
                    (Kind::Scalar(Scalar::Float(Float::Infinity { negative })), "float") => {
                        text.ends_with("inf") && *negative == text.starts_with('-')
                    }
                    (Kind::Scalar(Scalar::Float(Float::Finite(value))), "float") => {
                        value.to_bits() == text.parse::<f64>().unwrap().to_bits()
                    }
                    (Kind::Scalar(Scalar::OffsetDateTime(value)), "datetime") => {
                        stamp(value) == stamp(text)
                    }
                    (Kind::Scalar(Scalar::LocalDateTime(value)), "datetime-local") => {
                        stamp(&format!("{value}Z")) == stamp(&format!("{text}Z"))
                    }
                    (Kind::Scalar(Scalar::LocalDate(value)), "date-local") => {
                        rfc3339::parse_date(value).unwrap() == rfc3339::parse_date(text).unwrap()
                    }
                    (Kind::Scalar(Scalar::LocalTime(value)), "time-local") => {
                        rfc3339::parse_time(&format!("{value}Z")).unwrap()
                            == rfc3339::parse_time(&format!("{text}Z")).unwrap()
                    }
                    _ => false,
                }
            }
            _ => false,
        };
        if !good {
            return Err(format!("{}: {:?} != {expected:?}", node.path, node.kind));
        }
    }
    Ok(())
}

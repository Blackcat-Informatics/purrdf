// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The real binary consumes the public compiled matcher through the facade.
mod support;
use support::{run, stderr, stdout_utf8};

#[test]
fn explicit_surfaces_decide_and_refusals_publish_no_answer() {
    let matched = run(&[
        "glob",
        "**/a*b/c",
        "axxx/y/ab/c",
        "--mode",
        "path",
        "--surface",
        "literal",
        "--max-work",
        "1000",
    ]);
    assert!(matched.status.success(), "{}", stderr(&matched));
    assert!(stdout_utf8(&matched).starts_with("matched true\nwork "));
    let negative = run(&[
        "glob",
        "a/*",
        "https://example.org/a/b/c",
        "--mode",
        "path",
        "--surface",
        "iri-path",
        "--max-work",
        "1000",
    ]);
    assert!(negative.status.success(), "{}", stderr(&negative));
    assert!(stdout_utf8(&negative).starts_with("matched false\nwork "));
    let iri = run(&[
        "glob",
        "/a/%2F",
        "https://example.org/a/%2F?x#y",
        "--mode",
        "path",
        "--surface",
        "iri-path",
        "--max-work",
        "1000",
    ]);
    assert!(iri.status.success(), "{}", stderr(&iri));
    assert!(stdout_utf8(&iri).starts_with("matched true\nwork "));
    for (value, expected) in [
        ("https://example.org/", "false"),
        ("https://example.org/?", "true"),
    ] {
        let query = run(&[
            "glob",
            "*",
            value,
            "--mode",
            "text",
            "--surface",
            "iri-query",
            "--max-work",
            "100",
        ]);
        assert!(query.status.success(), "{}", stderr(&query));
        assert!(stdout_utf8(&query).starts_with(&format!("matched {expected}\nwork ")));
    }
    for (pattern, value, surface, work, message) in [
        ("*", "anything", "literal", "0", "glob work refused"),
        ("[z-a]", "a", "literal", "100", "glob syntax at byte 1"),
        ("*", "http://[", "iri", "100", "malformed authority"),
    ] {
        let refused = run(&[
            "glob",
            pattern,
            value,
            "--mode",
            "text",
            "--surface",
            surface,
            "--max-work",
            work,
        ]);
        assert_eq!(refused.status.code(), Some(1), "{}", stderr(&refused));
        assert_eq!(refused.stdout, [] as [u8; 0]);
        assert!(stderr(&refused).contains(message), "{}", stderr(&refused));
    }
    let usage = run(&["glob", "*", "a"]);
    assert_eq!(usage.status.code(), Some(2));
    let help = run(&["glob", "--help"]);
    assert!(help.status.success());
    for flag in ["--mode", "--surface", "--max-work"] {
        assert!(stdout_utf8(&help).contains(flag));
    }
}

#[test]
fn ordered_pattern_sets_publish_all_indices_only_after_complete_success() {
    let command = [
        "glob-set",
        "a",
        "--pattern",
        "**",
        "--pattern",
        "z",
        "--pattern",
        "**",
        "--mode",
        "path",
        "--surface",
        "literal",
        "--max-work",
    ];
    let mut args = command.to_vec();
    args.push("3");
    let matched = run(&args);
    assert!(matched.status.success(), "{}", stderr(&matched));
    assert_eq!(
        stdout_utf8(&matched),
        "matches 2\nindex 0\nindex 2\nwork 3\n"
    );
    *args.last_mut().unwrap() = "2";
    let refused = run(&args);
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(refused.stdout, [] as [u8; 0]);
    assert!(stderr(&refused).contains("glob pattern 2: glob work refused"));
    let empty = run(&[
        "glob-set",
        "a",
        "--mode",
        "path",
        "--surface",
        "literal",
        "--max-work",
        "0",
    ]);
    assert!(empty.status.success(), "{}", stderr(&empty));
    assert_eq!(stdout_utf8(&empty), "matches 0\nwork 0\n");
    let iri = run(&[
        "glob-set",
        "https://example.org/a/%2F?q",
        "--pattern",
        "/a/%2F",
        "--pattern",
        "/a//",
        "--mode",
        "path",
        "--surface",
        "iri-path",
        "--max-work",
        "100",
    ]);
    assert!(iri.status.success(), "{}", stderr(&iri));
    assert!(stdout_utf8(&iri).starts_with("matches 1\nindex 0\nwork "));
    for (value, pattern, surface, message) in [
        ("a", "[z-a]", "literal", "glob pattern 1: glob syntax"),
        ("http://[", "*", "iri", "malformed authority"),
    ] {
        let refusal = run(&[
            "glob-set",
            value,
            "--pattern",
            "*",
            "--pattern",
            pattern,
            "--mode",
            "text",
            "--surface",
            surface,
            "--max-work",
            "100",
        ]);
        assert_eq!(refusal.status.code(), Some(1));
        assert_eq!(refusal.stdout, [] as [u8; 0]);
        assert!(stderr(&refusal).contains(message), "{}", stderr(&refusal));
    }
}

#[test]
fn rdf_plans_publish_duplicate_memberships_in_both_codecs_and_nothing_on_refusal() {
    use purrdf_core::{DatasetView, GraphMatch, TermRef};
    let directory = purrdf_testkit::temp_dir!().unwrap();
    let plan_path = directory.path().join("plan.rdf");
    let input_path = directory.path().join("input.rdf");
    let plan = format!(
        "@prefix v: <https://example.org/v/> .
         @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
         <https://example.org/plan> a v:description_type ;
         v:version \"{}\" ; v:language \"{}\" ; v:mode \"text\" ;
         v:surface \"literal\" ; v:patterns (<https://example.org/pattern> <https://example.org/pattern>) .
         <https://example.org/pattern> a v:pattern_type ; v:source \"猫*\" .",
        purrdf::glob::rdf::DESCRIPTION_VERSION, purrdf::glob::LANGUAGE_VERSION,
    );
    let input = "<https://example.org/subject> <https://example.org/property> \"猫語\"@ja--rtl .";
    let parsed_plan = purrdf::parse_dataset(plan.as_bytes(), "text/turtle", None).unwrap();
    let parsed_input = purrdf::parse_dataset(input.as_bytes(), "text/turtle", None).unwrap();
    for (from, to, format, media) in [
        (
            "turtle",
            "ntriples",
            purrdf::NativeRdfFormat::Turtle,
            "application/n-triples",
        ),
        (
            "ntriples",
            "turtle",
            purrdf::NativeRdfFormat::NTriples,
            "text/turtle",
        ),
    ] {
        for (path, dataset) in [(&plan_path, &parsed_plan), (&input_path, &parsed_input)] {
            std::fs::write(
                path,
                purrdf::serialize_dataset_to_format(&**dataset, format, None)
                    .unwrap()
                    .bytes,
            )
            .unwrap();
        }
        let mut args: Vec<String> = [
            "glob-rdf",
            plan_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
            "--root",
            "https://example.org/plan",
            "--evaluation",
            "https://example.org/evaluation",
            "--input-subject",
            "https://example.org/subject",
            "--input-predicate",
            "https://example.org/property",
            "--from",
            from,
            "--to",
            to,
            "--max-work",
            "1000",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        for role in purrdf::glob::rdf::ROLES {
            args.extend([
                "--vocabulary".to_owned(),
                format!("{role}=https://example.org/v/{role}"),
            ]);
        }
        let run_args =
            |values: &[String]| run(&values.iter().map(String::as_str).collect::<Vec<_>>());
        let completed = run_args(&args);
        assert!(completed.status.success(), "{}", stderr(&completed));
        let output = purrdf::parse_dataset(&completed.stdout, media, None).unwrap();
        let evaluation = output
            .term_id_by_iri("https://example.org/evaluation")
            .unwrap();
        let matched = output
            .term_id_by_iri("https://example.org/v/matched")
            .unwrap();
        let memberships: Vec<_> = output
            .quads_for_pattern(Some(evaluation), Some(matched), None, GraphMatch::Default)
            .map(|quad| quad.o)
            .collect();
        assert_eq!(memberships.len(), 2);
        assert_ne!(memberships[0], memberships[1]);
        for membership in memberships {
            assert!(matches!(
                output.as_ref().resolve(membership),
                TermRef::Triple { .. }
            ));
        }
        let value = output
            .term_id_by_iri("https://example.org/v/input")
            .unwrap();
        let original = output
            .quads_for_pattern(Some(evaluation), Some(value), None, GraphMatch::Default)
            .next()
            .unwrap()
            .o;
        assert!(matches!(
            output.as_ref().resolve(original),
            TermRef::Literal {
                lexical: "猫語",
                language: Some("ja"),
                direction: Some(_),
                ..
            }
        ));
        let work_index = args.iter().position(|value| value == "--max-work").unwrap() + 1;
        args[work_index] = "0".to_owned();
        let refused = run_args(&args);
        assert_eq!(refused.status.code(), Some(1));
        assert_eq!(refused.stdout, [] as [u8; 0]);
        assert!(stderr(&refused).contains("glob work refused"));
        args[work_index] = "1000".to_owned();
        std::fs::write(&input_path, b"malformed RDF").unwrap();
        let malformed = run_args(&args);
        assert_eq!(malformed.status.code(), Some(1));
        assert_eq!(malformed.stdout, [] as [u8; 0]);
    }
}

mod native_rdf_contract {
    use purrdf::error::ErrorValue;
    use purrdf::glob::rdf::{DESCRIPTION_VERSION, Execution, PreparedPlan, ROLES, Vocabulary};
    use purrdf::glob::{Admission, Resident, StorageError};
    use purrdf::{NativeRdfFormat, parse_dataset, serialize_dataset_to_format};
    use purrdf_core::{RdfDataset, TermId, TermRef};
    use purrdf_testkit::error_capture::Capture;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[global_allocator]
    static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

    const EX: &str = "https://example.org/id/";
    const TERMS: [&str; 13] = [
        "https://example.org/custom/Description",
        "https://example.org/custom/Pattern",
        "https://example.org/custom/revision",
        "https://example.org/custom/language",
        "https://example.org/custom/mode",
        "https://example.org/custom/surface",
        "https://example.org/custom/programs",
        "https://example.org/custom/text",
        "https://example.org/custom/Evaluation",
        "https://example.org/custom/plan",
        "https://example.org/custom/input",
        "https://example.org/custom/matched",
        "https://example.org/custom/work",
    ];

    fn vocabulary() -> Vocabulary<'static> {
        let entries: [(&str, &str); 13] = std::array::from_fn(|index| (ROLES[index], TERMS[index]));
        Vocabulary::new(&entries).unwrap()
    }

    fn document(surface: &str, positive: &str, negative: &str, input: &str) -> String {
        format!(
            r#"@prefix v: <https://example.org/custom/> .
@prefix ex: <https://example.org/id/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:plan a v:Description ; v:revision "{version}" ; v:language "{language}" ;
  v:mode "text" ; v:surface "{surface}" ; v:programs ex:c0 .
ex:other a v:Description ; v:revision "{version}" ; v:language "{language}" ;
  v:mode "text" ; v:surface "{surface}" ; v:programs ex:c0 .
ex:c0 rdf:first ex:yes ; rdf:rest ex:c1 .
ex:c1 rdf:first ex:no ; rdf:rest ex:c2 .
ex:c2 rdf:first ex:yes ; rdf:rest rdf:nil .
ex:yes a v:Pattern ; v:text "{positive}" .
ex:no a v:Pattern ; v:text "{negative}" .
ex:unlisted a v:Pattern ; v:text "*" .
ex:previous a v:Evaluation .
ex:datum ex:value {input} .
"#,
            version = DESCRIPTION_VERSION,
            language = purrdf::glob::LANGUAGE_VERSION
        )
    }

    fn data(text: &str) -> Arc<RdfDataset> {
        parse_dataset(text.as_bytes(), "text/turtle", None).unwrap()
    }
    fn id(data: &RdfDataset, local: &str) -> TermId {
        data.term_id_by_iri(&format!("{EX}{local}")).unwrap()
    }
    fn object(data: &RdfDataset, subject: TermId, predicate: TermId) -> TermId {
        use purrdf_core::collections::SoleObject;
        match SoleObject::of(
            data.quads()
                .filter(|quad| quad.g.is_none() && quad.s == subject && quad.p == predicate)
                .map(|quad| quad.o),
        ) {
            SoleObject::One(value) => value,
            other => panic!("fixture needs exactly one object: {other:?}"),
        }
    }
    fn input(data: &RdfDataset) -> TermId {
        object(data, id(data, "datum"), id(data, "value"))
    }
    fn prepared(source: Arc<RdfDataset>) -> PreparedPlan<'static> {
        let root = id(&source, "plan");
        PreparedPlan::compile(source.into(), root, &vocabulary(), Resident).unwrap()
    }
    fn executed(source: Arc<RdfDataset>, max_work: u64) -> Execution<'static> {
        let value = input(&source);
        prepared(source.clone())
            .execute(source.into(), value, max_work, Resident)
            .unwrap()
    }

    #[derive(Debug, Clone)]
    struct Budget {
        limit: usize,
        live: Arc<AtomicUsize>,
        peak: Arc<AtomicUsize>,
    }
    impl Budget {
        fn new(limit: usize) -> Self {
            Self {
                limit,
                live: Arc::new(AtomicUsize::new(0)),
                peak: Arc::new(AtomicUsize::new(0)),
            }
        }
    }
    impl Admission for Budget {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live.store(bytes, Ordering::Relaxed);
            self.peak.fetch_max(bytes, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn facade_rdf_plans_preserve_occurrences_metadata_and_lexical_iri_surfaces() {
        for (surface, positive, negative, value) in [
            ("literal", "猫*", "z", r#""猫語"@ja--rtl"#),
            ("literal", "12*", "z", r#""12.50"^^xsd:decimal"#),
            ("iri-path", "/a/%2F", "/a//", "<https://example.org/a/%2F>"),
        ] {
            let original = data(&document(surface, positive, negative, value));
            let original_value = original.term_value(input(&original));
            for format in [NativeRdfFormat::Turtle, NativeRdfFormat::NTriples] {
                let encoded = serialize_dataset_to_format(&*original, format, None).unwrap();
                let source = parse_dataset(&encoded.bytes, format.media_type(), None).unwrap();
                let result = executed(source, u64::MAX);
                assert_eq!(result.indices(), [0, 2]);
                // This identity already exists as an unrelated input subject.
                let output = result.export(&format!("{EX}datum"), Resident).unwrap();
                assert!(output.retains_admission());
                let evaluation = id(&output, "datum");
                let input_predicate = output.term_id_by_iri(TERMS[10]).unwrap();
                assert_eq!(
                    output.term_value(object(&output, evaluation, input_predicate)),
                    original_value
                );
                let matched = output.term_id_by_iri(TERMS[11]).unwrap();
                let mut cells = Vec::new();
                for quad in output
                    .quads()
                    .filter(|quad| quad.g.is_none() && quad.s == evaluation && quad.p == matched)
                {
                    let TermRef::Triple { s, p, o } = output.resolve(quad.o) else {
                        panic!("each match must quote its actual membership");
                    };
                    assert_eq!(
                        output.resolve(p),
                        TermRef::Iri(purrdf::iri::vocab::rdf::FIRST)
                    );
                    assert_eq!(o, id(&output, "yes"));
                    let TermRef::Iri(cell) = output.resolve(s) else {
                        panic!("fixture uses named membership cells");
                    };
                    cells.push(cell.to_owned());
                }
                cells.sort();
                assert_eq!(cells, [format!("{EX}c0"), format!("{EX}c2")]);
                let encoded = serialize_dataset_to_format(&*output, format, None).unwrap();
                assert_eq!(encoded.statement_rows_dropped, 0);
                assert_eq!(encoded.directional_literals_dropped, 0);
                assert_eq!(encoded.named_graph_rows_dropped, 0);
                let restored = parse_dataset(&encoded.bytes, format.media_type(), None).unwrap();
                assert_eq!(executed(restored, u64::MAX).indices(), [0, 2]);
            }
            for collision in ["plan", "other", "yes", "unlisted", "previous"] {
                let error = executed(original.clone(), u64::MAX)
                    .export(&format!("{EX}{collision}"), Resident)
                    .unwrap_err();
                let fields = Capture::read(&error);
                assert!(matches!(
                    fields.field("reason"),
                    ErrorValue::Code("evaluation-identity-collision")
                ));
                assert!(
                    matches!(fields.child("node").field("iri"), ErrorValue::Iri(iri) if iri == format!("{EX}{collision}"))
                );
            }
        }
        for (datatype, lexical) in [("List", "[_:onlyInput]"), ("Map", "{ 'key': _:onlyInput }")] {
            let plan = data(&document("literal", "*", "not matched", "\"unused\""));
            let source = data(&format!(
                "<{EX}datum> <{EX}value> \"{lexical}\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/{datatype}> .
                 _:onlyInput <{EX}marker> <{EX}object> ."
            ));
            let (label, scope) = source
                .quads()
                .find_map(|quad| match source.as_ref().resolve(quad.s) {
                    TermRef::Blank { label, scope } => Some((label.to_owned(), scope)),
                    _ => None,
                })
                .unwrap();
            assert!(plan.term_id_by_blank(&label, scope).is_none());
            let value = input(&source);
            let original = source.term_value(value);
            let result = prepared(plan)
                .execute(source.into(), value, u64::MAX, Resident)
                .unwrap();
            assert_eq!(result.indices(), [0, 2]);
            let output = result.export(&format!("{EX}run"), Resident).unwrap();
            assert!(output.term_id_by_blank(&label, scope).is_some());
            let selected = object(
                &output,
                id(&output, "run"),
                output.term_id_by_iri(TERMS[10]).unwrap(),
            );
            assert_eq!(output.term_value(selected), original);
            for format in [NativeRdfFormat::Turtle, NativeRdfFormat::NTriples] {
                let encoded = serialize_dataset_to_format(&*output, format, None).unwrap();
                let restored = parse_dataset(&encoded.bytes, format.media_type(), None).unwrap();
                let selected = object(
                    &restored,
                    id(&restored, "run"),
                    restored.term_id_by_iri(TERMS[10]).unwrap(),
                );
                assert_eq!(restored.term_value(selected), original);
                assert!(restored.term_id_by_blank(&label, scope).is_some());
                assert_eq!(
                    prepared(restored.clone())
                        .execute(restored.into(), selected, u64::MAX, Resident)
                        .unwrap()
                        .indices(),
                    [0, 2]
                );
            }
        }
    }

    #[test]
    fn rdf_refusals_keep_native_causes_original_context_and_safe_id_boundaries() {
        let text = document("literal", "猫*", "z", r#""猫語"@ja--rtl"#);
        for (forged, reason) in [
            (
                text.replace(DESCRIPTION_VERSION, "purrdf-glob-rdf-v0"),
                "incompatible-version",
            ),
            (
                text.replace(
                    &format!("v:revision \"{DESCRIPTION_VERSION}\""),
                    &format!("v:revision \"{DESCRIPTION_VERSION}\", \"purrdf-glob-rdf-v0\""),
                ),
                "required-single-value",
            ),
        ] {
            let source = data(&forged);
            let root = id(&source, "plan");
            let error =
                PreparedPlan::compile(source.into(), root, &vocabulary(), Resident).unwrap_err();
            assert_eq!(error.plan_id(), root);
            assert!(
                matches!(Capture::read(&error).field("reason"), ErrorValue::Code(code) if code == reason)
            );
        }
        let unknown = data(&format!("{text} <{EX}yes> <{EX}unexpected> \"bad\" ."));
        let root = id(&unknown, "plan");
        let error =
            PreparedPlan::compile(unknown.into(), root, &vocabulary(), Resident).unwrap_err();
        let fields = Capture::read(&error);
        assert!(matches!(
            fields.field("role"),
            ErrorValue::Code("pattern_type")
        ));
        assert!(
            matches!(fields.child("node").field("iri"), ErrorValue::Iri(iri) if iri == format!("{EX}yes"))
        );
        let cyclic = data(&text.replace("rdf:rest rdf:nil", "rdf:rest ex:c0"));
        let root = id(&cyclic, "plan");
        let error =
            PreparedPlan::compile(cyclic.into(), root, &vocabulary(), Resident).unwrap_err();
        assert!(matches!(
            Capture::read(&error).field("members"),
            ErrorValue::Unsigned(3)
        ));
        let malformed = data(&text.replace("v:text \"z\"", "v:text \"[z-a]\""));
        let root = id(&malformed, "plan");
        let error =
            PreparedPlan::compile(malformed.into(), root, &vocabulary(), Resident).unwrap_err();
        let cause = Capture::read(&error).child("cause");
        assert!(matches!(cause.field("index"), ErrorValue::Unsigned(1)));
        assert_eq!(cause.child("cause").code, "glob-syntax");
        let mut entries: [(&str, &str); 13] =
            std::array::from_fn(|index| (ROLES[index], TERMS[index]));
        entries[0].0 = "unknown";
        assert_eq!(
            Vocabulary::new(&entries).unwrap_err().reason,
            "unknown-role"
        );
        let source = data(&text);
        let foreign = TermId::from_index(u32::try_from(source.term_count()).unwrap()).unwrap();
        let error = PreparedPlan::compile(source.clone().into(), foreign, &vocabulary(), Resident)
            .unwrap_err();
        assert!(matches!(
            Capture::read(&error).field("reason"),
            ErrorValue::Code("term-id-out-of-range")
        ));
        drop(error);
        let error = prepared(source.clone())
            .execute(source.clone().into(), foreign, u64::MAX, Resident)
            .unwrap_err();
        assert!(matches!(
            Capture::read(&error).field("reason"),
            ErrorValue::Code("term-id-out-of-range")
        ));
        drop(error);
        let work = executed(source.clone(), u64::MAX).work();
        assert_eq!(executed(source.clone(), work).indices(), [0, 2]);
        let root = id(&source, "plan");
        let input_source = data(&format!("<{EX}datum> <{EX}value> \"猫語\"@ja--rtl ."));
        let value = input(&input_source);
        let weak_plan = Arc::downgrade(&source);
        let weak_input = Arc::downgrade(&input_source);
        let error = prepared(source)
            .execute(input_source.into(), value, work - 1, Resident)
            .unwrap_err();
        assert!(weak_plan.upgrade().is_some());
        assert!(weak_input.upgrade().is_some());
        assert_eq!(error.plan_id(), root);
        assert_eq!(error.input_id(), Some(value));
        let buffer = Vec::with_capacity(64);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let fields = Capture::read_with_buffer(&error, buffer);
        let measured = window.close();
        assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
        assert!(matches!(
            fields.field("surface"),
            ErrorValue::Code("literal")
        ));
        assert!(matches!(
            fields.child("input").field("language"),
            ErrorValue::Text("ja")
        ));
        assert_eq!(
            fields.child("cause").child("cause").code,
            "glob-work-exceeded"
        );
        drop(fields);
        drop(error);
        assert!(weak_plan.upgrade().is_none());
        assert!(weak_input.upgrade().is_none());
    }

    #[test]
    fn rdf_plan_admission_boundaries_refund_program_result_and_export_owners() {
        let source = data(&document("literal", "猫*", "z", r#""猫語"@ja--rtl"#));
        let root = id(&source, "plan");
        let budget = Budget::new(usize::MAX);
        let plan =
            PreparedPlan::compile(source.clone().into(), root, &vocabulary(), budget.clone())
                .unwrap();
        let compile_peak = budget.peak.load(Ordering::Relaxed);
        assert!(budget.live.load(Ordering::Relaxed) > 0);
        drop(plan);
        assert_eq!(budget.live.load(Ordering::Relaxed), 0);
        for limit in [compile_peak, compile_peak - 1] {
            let budget = Budget::new(limit);
            let result =
                PreparedPlan::compile(source.clone().into(), root, &vocabulary(), budget.clone());
            assert_eq!(result.is_ok(), limit == compile_peak);
            drop(result);
            assert_eq!(budget.live.load(Ordering::Relaxed), 0);
        }
        let slots = 3 * size_of::<usize>();
        for limit in [slots, slots - 1] {
            let budget = Budget::new(limit);
            let result = prepared(source.clone()).execute(
                source.clone().into(),
                input(&source),
                u64::MAX,
                budget.clone(),
            );
            assert_eq!(result.is_ok(), limit == slots);
            drop(result);
            assert_eq!(budget.live.load(Ordering::Relaxed), 0);
        }
        let budget = Budget::new(usize::MAX);
        let output = executed(source.clone(), u64::MAX)
            .export(&format!("{EX}datum"), budget.clone())
            .unwrap();
        let export_peak = budget.peak.load(Ordering::Relaxed);
        assert!(budget.live.load(Ordering::Relaxed) > 0);
        drop(output);
        assert_eq!(budget.live.load(Ordering::Relaxed), 0);
        for limit in [export_peak, export_peak - 1] {
            let budget = Budget::new(limit);
            let result =
                executed(source.clone(), u64::MAX).export(&format!("{EX}datum"), budget.clone());
            assert_eq!(result.is_ok(), limit == export_peak);
            drop(result);
            assert_eq!(budget.live.load(Ordering::Relaxed), 0);
        }
    }
}

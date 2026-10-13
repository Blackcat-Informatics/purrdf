// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual checker entrypoints and independent graph-answer tamper controls.
use purrdf_bench::lubm::{Spec, check, generate_directory};
use purrdf_rdf::NativeRdfFormat;
use purrdf_testkit::TempDir;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
const BIN: &str = env!("CARGO_BIN_EXE_lubm-check");
fn command(args: &[&str], input: &[u8]) -> Output {
    let mut process = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.take().unwrap().write_all(input).unwrap();
    process.wait_with_output().unwrap()
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
struct Fixture {
    _owned: TempDir,
    generated: PathBuf,
    converted: PathBuf,
    aggregate: PathBuf,
    receipt: PathBuf,
    spec: Spec,
}
impl Fixture {
    fn new(index: u64, ontology: &str) -> Self {
        let owned = purrdf_testkit::temp_dir!("native-university-verification").unwrap();
        let generated = owned.path().join("generated");
        let converted = owned.path().join("converted");
        fs::create_dir(&converted).unwrap();
        let spec = Spec::new(
            7,
            index,
            1,
            ontology.into(),
            "http://example.org/native-docs/".into(),
        )
        .unwrap();
        let receipt = generate_directory(&spec, &generated).unwrap();
        let mut joined = Vec::new();
        for entry in receipt.files {
            let dataset = purrdf_rdf::parse_dataset(
                &fs::read(generated.join(&entry.name)).unwrap(),
                "application/n-triples",
                None,
            )
            .unwrap();
            let serialized = purrdf_rdf::serialize_dataset_to_format(
                dataset.as_ref(),
                NativeRdfFormat::NQuads,
                None,
            )
            .unwrap()
            .bytes;
            fs::write(
                converted.join(entry.name.replace(".nt", ".nq")),
                &serialized,
            )
            .unwrap();
            joined.extend(serialized);
        }
        let aggregate = owned.path().join("aggregate.nq");
        fs::write(&aggregate, joined).unwrap();
        let receipt_path = owned.path().join("acceptance.json");
        Self {
            _owned: owned,
            generated,
            converted,
            aggregate,
            receipt: receipt_path,
            spec,
        }
    }
    fn check(&self, mode: &str) -> Output {
        command(
            &[
                mode,
                self.generated.to_str().unwrap(),
                self.converted.to_str().unwrap(),
                self.aggregate.to_str().unwrap(),
                &self.spec.seed.to_string(),
                &self.spec.index.to_string(),
                &self.spec.universities.to_string(),
                &self.spec.ontology,
                &self.spec.document_base,
                self.receipt.to_str().unwrap(),
            ],
            b"",
        )
    }
    fn acceptance(&self) -> check::Acceptance {
        purrdf_lex::json::record::from_slice(&fs::read(&self.receipt).unwrap()).unwrap()
    }
}
fn result_bytes(iris: &[String]) -> Vec<u8> {
    use purrdf_lex::json::{Object, Value};
    let row = |iri: &String| {
        Value::Object(Object::from_iter([(
            "X",
            Value::Object(Object::from_iter([
                ("type", Value::String("uri".into())),
                ("value", Value::String(iri.clone())),
            ])),
        )]))
    };
    let value = Value::Object(Object::from_iter([
        (
            "head",
            Value::Object(Object::from_iter([(
                "vars",
                Value::Array(vec![Value::String("X".into())]),
            )])),
        ),
        (
            "results",
            Value::Object(Object::from_iter([(
                "bindings",
                Value::Array(iris.iter().map(row).collect()),
            )])),
        ),
    ]));
    purrdf_lex::json::write_compact(&value).into_bytes()
}
#[test]
fn complete_graph_receipts_and_exact_native_oracles_use_real_entrypoints() {
    for (index, ontology) in [
        (0, check::EXTERNAL_ONTOLOGY),
        (3, "https://example.org/custom-schema"),
    ] {
        let fixture = Fixture::new(index, ontology);
        success(&fixture.check("verify"));
        success(&fixture.check("recheck"));
        assert!(
            !fixture.check("verify").status.success(),
            "existing acceptance must not be overwritten"
        );
        let acceptance = fixture.acceptance();
        assert_ne!(acceptance.q14, [] as [String; 0]);
        if index != 0 {
            assert!(
                acceptance.q1.is_empty(),
                "Q1 fixed course must not be retargeted"
            );
        }
        for (id, iris) in [("Q1", acceptance.q1), ("Q14", acceptance.q14)] {
            let output = command(
                &["results", fixture.receipt.to_str().unwrap(), id],
                &result_bytes(&iris),
            );
            success(&output);
            assert_eq!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .trim()
                    .parse::<usize>()
                    .unwrap(),
                iris.len()
            );
            let wrong = vec!["http://example.org/incorrect-answer".into()];
            assert!(
                !command(
                    &["results", fixture.receipt.to_str().unwrap(), id],
                    &result_bytes(&wrong)
                )
                .status
                .success()
            );
            if !iris.is_empty() {
                let duplicate = vec![iris[0].clone(), iris[0].clone()];
                assert!(
                    !command(
                        &["results", fixture.receipt.to_str().unwrap(), id],
                        &result_bytes(&duplicate)
                    )
                    .status
                    .success()
                );
            }
        }
    }
}
#[test]
fn retained_receipts_refuse_inventory_payload_conversion_and_acceptance_tampering() {
    let mut fixture = Fixture::new(2, check::EXTERNAL_ONTOLOGY);
    success(&fixture.check("verify"));
    let original = fs::read(&fixture.aggregate).unwrap();
    let mut changed = original.clone();
    changed.extend_from_slice(original.split_inclusive(|byte| *byte == 10).next().unwrap());
    fs::write(&fixture.aggregate, changed).unwrap();
    assert!(
        !fixture.check("recheck").status.success(),
        "duplicate aggregate bytes must fail despite an unchanged set graph"
    );
    fs::write(&fixture.aggregate, original).unwrap();
    let extra = fixture.generated.join("unaccounted.nt");
    fs::write(&extra, b"extra").unwrap();
    assert!(!fixture.check("recheck").status.success());
    fs::remove_file(extra).unwrap();
    let file = fs::read_dir(&fixture.converted)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let original = fs::read(&file).unwrap();
    fs::write(
        &file,
        b"<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    )
    .unwrap();
    assert!(!fixture.check("recheck").status.success());
    fs::write(file, original).unwrap();
    let file = fixture.generated.join("receipt.json");
    let original = fs::read(&file).unwrap();
    fs::write(&file, b"{}").unwrap();
    assert!(!fixture.check("recheck").status.success());
    fs::write(&file, original).unwrap();
    let file = fs::read_dir(&fixture.generated)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            (path.extension() == Some(std::ffi::OsStr::new("nt"))).then_some(path)
        })
        .unwrap();
    let original = fs::read(&file).unwrap();
    fs::remove_file(&file).unwrap();
    assert!(!fixture.check("recheck").status.success());
    fs::write(&file, original).unwrap();
    fixture.spec.seed += 1;
    assert!(!fixture.check("recheck").status.success());
    fixture.spec.seed -= 1;
    success(&fixture.check("recheck"));
    fs::write(&fixture.receipt, b"{}").unwrap();
    assert!(!fixture.check("recheck").status.success());
}
#[test]
fn schema_projection_changes_all_rdf_positions_and_preserves_w3c_and_lexical_content() {
    let external = check::EXTERNAL_ONTOLOGY;
    let sub_class = purrdf_iri::vocab::rdfs::SUB_CLASS_OF;
    let rdf_type = purrdf_iri::vocab::rdf::TYPE;
    let owl_class = purrdf_iri::vocab::owl::CLASS;
    let bytes = format!(
        "<{external}#Subject> <{external}#predicate> <{external}#Object> .\n<{external}> <{external}#typed> \"{external}#lexical\"^^<{external}#Datatype> .\n_:schema <{sub_class}> <{external}#Class> .\n<http://example.org/foreign> <{rdf_type}> <{owl_class}> .\n<{external}#Quoted> <{external}#quotes> <<( <{external}#Subject> <{external}#predicate> \"x\"^^<{external}#Datatype> )>> <{external}#Graph> .\n"
    );
    let output = command(&["project", "https://example.org/custom"], bytes.as_bytes());
    success(&output);
    let text = String::from_utf8(output.stdout).unwrap();
    let dataset = purrdf_rdf::parse_dataset(text.as_bytes(), "application/n-quads", None).unwrap();
    assert_eq!(dataset.quads().count(), 5);
    assert!(text.contains("<https://example.org/custom#Subject> <https://example.org/custom#predicate> <https://example.org/custom#Object>"));
    assert!(text.contains(&format!(
        "<{0}> <{0}#typed> \"{external}#lexical\"^^<{0}#Datatype>",
        "https://example.org/custom"
    )));
    assert!(text.contains(sub_class));
    assert!(text.contains(owl_class));
    assert!(text.contains("<http://example.org/foreign>"));
    assert!(text.contains("<https://example.org/custom#Quoted>"));
    assert!(text.contains("<https://example.org/custom#Graph>"));
    assert!(text.contains("\"x\"^^<https://example.org/custom#Datatype>"));
    assert!(
        !command(&["project", "relative"], bytes.as_bytes())
            .status
            .success()
    );
    assert!(
        !command(&["project", "https://example.org/custom"], b"not RDF")
            .status
            .success()
    );
}
#[test]
fn native_config_results_and_cargo_readers_fail_actionably() {
    let admitted = command(
        &[
            "config",
            "007",
            "2",
            "1",
            check::EXTERNAL_ONTOLOGY,
            "http://example.org/data/",
        ],
        b"",
    );
    success(&admitted);
    assert_eq!(admitted.stdout, b"7\t2\n");
    let maximum = command(
        &[
            "config",
            "18446744073709551615",
            "18446744073709551614",
            "1",
            check::EXTERNAL_ONTOLOGY,
            "http://example.org/data/",
        ],
        b"",
    );
    success(&maximum);
    assert_eq!(
        maximum.stdout,
        b"18446744073709551615\t18446744073709551614\n"
    );
    for args in [
        [
            "0",
            "0",
            "0",
            check::EXTERNAL_ONTOLOGY,
            "http://example.org/data/",
        ],
        [
            "0",
            "18446744073709551615",
            "1",
            check::EXTERNAL_ONTOLOGY,
            "http://example.org/data/",
        ],
        ["0", "0", "1", "relative", "http://example.org/data/"],
        [
            "0",
            "0",
            "1",
            check::EXTERNAL_ONTOLOGY,
            "http://example.org/data",
        ],
    ] {
        let mut invocation = vec!["config"];
        invocation.extend(args);
        assert!(!command(&invocation, b"").status.success());
    }
    for bytes in [b"{}".as_slice(), b"null", b"{\"results\":{\"bindings\":[]}}", b"{\"head\":{\"vars\":[\"X\"]},\"results\":{\"bindings\":[{\"X\":{\"type\":\"invalid\",\"value\":\"x\"}}]}}"] {
        assert!(!command(&["results"], bytes).status.success());
    }
    success(&command(&["results"], &result_bytes(&[])));
    let artifact = concat!("{\"reason\":\"compiler-artifact\",\"target\":{\"name\":\"purrdf\"},\"executable\":\"/opt/example/purrdf\"}\n", "{\"reason\":\"build-finished\",\"success\":true}\n").as_bytes();
    let output = command(&["artifact", "purrdf"], artifact);
    success(&output);
    assert_eq!(output.stdout, b"/opt/example/purrdf\n");
    assert!(!command(&["artifact", "missing"], artifact).status.success());
    assert!(
        !command(&["artifact", "purrdf"], b"not JSON")
            .status
            .success()
    );
}

#[test]
fn external_output_admission_uses_source_rules_and_rejects_personal_exclusions() {
    let owned = purrdf_testkit::temp_dir!("native-output-admission").unwrap();
    success(&command(&["output", owned.path().to_str().unwrap()], b""));
    let root = purrdf_testkit::paths::workspace_root();
    assert!(
        !command(&["output", root.to_str().unwrap()], b"")
            .status
            .success()
    );
    let visible = root.join("crates/bench");
    assert!(
        !command(&["output", visible.to_str().unwrap()], b"")
            .status
            .success()
    );
    let personal = owned.path().join("personal-excludes");
    fs::write(&personal, b"/crates/bench/\n").unwrap();
    let output = Command::new(BIN)
        .args(["output", visible.to_str().unwrap()])
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.excludesFile")
        .env("GIT_CONFIG_VALUE_0", &personal)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "a personal exclusion must not admit visible source output"
    );
    let broken = owned.path().join("broken-repository");
    fs::create_dir_all(broken.join(".git")).unwrap();
    fs::create_dir(broken.join("arena")).unwrap();
    assert!(
        !command(&["output", broken.join("arena").to_str().unwrap()], b"")
            .status
            .success()
    );
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Process-boundary tests for the SPARQL UPDATE pipeline and its atomic governor trip.

mod support;
use support::run;

const INSERT: &str =
    "INSERT DATA { <http://example.org/new> <http://example.org/p> <http://example.org/value> }";

/// Exact inputs reach every CLI execution/source route and one random stream
/// advances across the request's operations instead of restarting each WHERE.
#[test]
fn supplied_update_coordinates_replay_across_operations_and_routes() {
    use support::{code, pipe, stderr, stdout, write_file};
    let dir = purrdf_testkit::temp_dir!().unwrap();
    let text = "<http://example.org/old> <http://example.org/p> <http://example.org/value> .";
    let input = write_file(dir.path(), "coordinates.ttl", text);
    let pack = write_file(dir.path(), "coordinates.purrpck", "");
    let built = run(&["convert", "--from", "turtle", "--to", "pack", &input, &pack]);
    assert_eq!(code(&built), 0, "{}", stderr(&built));
    let update = ["a", "b"]
        .map(|subject| {
            format!(
                "INSERT {{ <http://example.org/{subject}> <http://example.org/time> ?t ; \
         <http://example.org/random> ?r ; <http://example.org/uuid> ?u ; \
         <http://example.org/str> ?v }} WHERE {{ BIND(NOW() AS ?t) \
         BIND(RAND() AS ?r) BIND(UUID() AS ?u) BIND(STRUUID() AS ?v) \
         FILTER(REGEX(\"ok\", \"^ok$\")) }}"
            )
        })
        .join("; ");
    let time = "2020-01-02T03:04:05.12345678901234567890123456789+05:30";
    let other = "-0001-02-03T04:05:06.98765432109876543210987654321-04:00";
    let answer = |source: &str, now: Option<&str>, seed: Option<&str>, lane: &[&str]| {
        let mut args = vec!["update", "--data", source, "--to", "ntriples"];
        if source == "-" {
            args.extend(["--from", "turtle"]);
        }
        if let Some(now) = now {
            args.extend(["--now", now]);
        }
        if let Some(seed) = seed {
            args.extend(["--entropy-seed", seed]);
        }
        args.extend_from_slice(lane);
        args.push(&update);
        let out = if source == "-" {
            pipe(&args, text)
        } else {
            run(&args)
        };
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        let body = stdout(&out);
        let mut cells = Vec::new();
        for subject in ["a", "b"] {
            for predicate in ["time", "random", "uuid", "str"] {
                let prefix =
                    format!("<http://example.org/{subject}> <http://example.org/{predicate}> ");
                cells.push(
                    body.lines()
                        .find_map(|line| line.strip_prefix(&prefix))
                        .expect("inserted coordinate value")
                        .to_owned(),
                );
            }
        }
        cells
    };
    let first = answer(&input, Some(time), Some("0"), &[]);
    assert!(first[0].contains(time));
    assert_eq!(first[0], first[4]);
    for index in 1..4 {
        assert_ne!(first[index], first[index + 4]);
    }
    for source in [&input[..], &pack[..], "-"] {
        for lane in [
            &[][..],
            &["--fuel", "1000000"][..],
            &["--no-ceiling"][..],
            &["--xpath-regex", "xpath-3.1-2017-03-21"][..],
        ] {
            assert_eq!(answer(source, Some(time), Some("0"), lane), first);
        }
    }
    let later = answer(&input, Some(other), Some("0"), &[]);
    let seeded = answer(&input, Some(time), Some("1"), &[]);
    let time_only = answer(&input, Some(time), None, &[]);
    let seed_only = answer(&input, None, Some("0"), &[]);
    assert_eq!(answer(&input, None, None, &[]).len(), 8);
    for index in 0..8 {
        if index % 4 == 0 {
            assert!(later[index].contains(other));
            assert_eq!(seeded[index], first[index]);
            assert_eq!(time_only[index], first[index]);
        } else {
            assert_eq!(later[index], first[index]);
            assert_ne!(seeded[index], first[index]);
            assert_eq!(seed_only[index], first[index]);
        }
    }
}

/// Invalid coordinates precede source reads; later failure or a governor trip
/// keeps both the source and an existing destination untouched.
#[test]
fn supplied_update_coordinates_preserve_atomic_errors_and_governor_evidence() {
    use support::{code, stderr, write_file};
    let dir = purrdf_testkit::temp_dir!().unwrap();
    let input = fixture(dir.path());
    let before = std::fs::read(&input).unwrap();
    let output = write_file(dir.path(), "destination.nt", "unchanged destination");
    let missing = dir.path().join("missing.ttl");
    for (flag, value) in [
        ("--now", "invalid"),
        ("--entropy-seed", "-1"),
        ("--entropy-seed", "18446744073709551616"),
    ] {
        let out = run(&[
            "update",
            "--data",
            missing.to_str().unwrap(),
            "--output",
            &output,
            "--to",
            "ntriples",
            flag,
            value,
            INSERT,
        ]);
        assert_eq!(code(&out), 2, "{}", stderr(&out));
        assert!(stderr(&out).contains(flag));
        assert!(!stderr(&out).contains("missing.ttl"));
        assert_eq!(out.stdout, [] as [u8; 0]);
    }
    let absorbed = run(&[
        "update",
        "--data",
        &input,
        "--to",
        "ntriples",
        "--fuel",
        "1000000",
        "--now",
        "2020-01-01T00:00:00Z",
        "--entropy-seed",
        "0",
        "INSERT { <http://example.org/a> <http://example.org/time> ?t } WHERE {
         BIND(NOW() AS ?t) BIND(1 / 0 AS ?bad) }",
    ]);
    assert_eq!(code(&absorbed), 0, "{}", stderr(&absorbed));
    assert!(stderr(&absorbed).contains("purrdf-expression-errors 1\nabsorbed err:FOAR0001 1\n"));
    let update = "INSERT { <http://example.org/a> <http://example.org/time> ?t } \
        WHERE { BIND(NOW() AS ?t) }; CREATE GRAPH <http://example.org/g>; \
        CREATE GRAPH <http://example.org/g>";
    for lane in [&[][..], &["--fuel", "1000000"][..], &["--fuel", "0"][..]] {
        let mut args = vec![
            "update",
            "--data",
            &input,
            "--output",
            &output,
            "--to",
            "ntriples",
            "--now",
            "2020-01-01T00:00:00Z",
            "--entropy-seed",
            "0",
        ];
        args.extend_from_slice(lane);
        args.push(update);
        let out = run(&args);
        assert_eq!(
            code(&out),
            if lane.last() == Some(&"0") { 3 } else { 1 },
            "{}",
            stderr(&out)
        );
        assert_eq!(out.stdout, [] as [u8; 0]);
        if code(&out) == 3 {
            assert!(stderr(&out).contains("tripped fuel-exhausted"));
            assert!(stderr(&out).contains("mutation none"));
        }
        assert_eq!(std::fs::read(&input).unwrap(), before);
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            "unchanged destination"
        );
    }
}

fn fixture(dir: &std::path::Path) -> String {
    let path = dir.join("input.ttl");
    std::fs::write(
        &path,
        "<http://example.org/old> <http://example.org/p> <http://example.org/value> .\n",
    )
    .expect("write fixture");
    path.to_str().expect("UTF-8 path").to_owned()
}

#[test]
fn update_applies_then_serializes_the_committed_dataset() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let input = fixture(dir.path());
    let output = run(&["update", "--data", &input, "--to", "ntriples", INSERT]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(support::execution_diagnostics(&output, "not-started"), "");
    let body = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(body.contains("<http://example.org/old>"), "{body}");
    assert!(body.contains("<http://example.org/new>"), "{body}");
}

#[test]
fn governed_update_trip_writes_no_dataset_and_exits_three() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let input = fixture(dir.path());
    let target = dir.path().join("must-not-exist.nt");
    let target = target.to_str().expect("UTF-8 path");
    let output = run(&[
        "update", "--data", &input, "--output", target, "--to", "ntriples", "--fuel", "0", INSERT,
    ]);

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stdout.is_empty(),
        "a tripped mutation emits no dataset"
    );
    assert!(!std::path::Path::new(target).exists());
    let report = support::execution_diagnostics(&output, "not-started");
    assert!(report.starts_with("purrdf-governor-report 1\n"), "{report}");
    assert!(report.contains("\noperation update\n"), "{report}");
    assert!(report.contains("\ntripped fuel-exhausted\n"), "{report}");
    assert!(report.contains("\nmutation none\n"), "{report}");
}

/// Run `update` over a TriG document and return the TriG it writes.
fn update_trig(document: &str, update: &str) -> String {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let path = dir.path().join("input.trig");
    std::fs::write(&path, document).expect("write fixture");
    let input = path.to_str().expect("UTF-8 path");
    let output = run(&[
        "update",
        "--graph-existence",
        "implicit",
        "--data",
        input,
        "--to",
        "trig",
        update,
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

const DECLARED: &str = "@prefix ex: <http://example.org/> .\n\
    ex:a ex:p ex:c .\n\
    GRAPH ex:g {}\n\
    GRAPH ex:h { ex:a ex:b ex:c }\n\
    GRAPH _:bg {}\n";

#[test]
fn drop_all_writes_no_graph_the_input_declared() {
    assert_eq!(update_trig(DECLARED, "DROP ALL").trim(), "");
}

#[test]
fn drop_graph_writes_no_declaration_of_the_dropped_graph() {
    let body = update_trig(DECLARED, "DROP GRAPH <http://example.org/g>");
    assert!(!body.contains("<http://example.org/g>"), "{body}");
    assert!(body.contains("<http://example.org/h> {"), "{body}");
    assert!(body.contains("_:bg {}"), "{body}");
}

#[test]
fn a_no_op_update_keeps_the_input_declarations() {
    let body = update_trig(
        DECLARED,
        "DELETE DATA { <http://example.org/none> <http://example.org/p> <http://example.org/none> }",
    );
    assert!(body.contains("<http://example.org/g> {}"), "{body}");
    assert!(body.contains("_:bg {}"), "{body}");
    assert!(body.contains("<http://example.org/h> {"), "{body}");
}

#[test]
fn a_graph_emptied_by_delete_is_neither_enumerated_nor_written() {
    // The graph `g1` loses its only quad; `GRAPH ?g` later in the same request must
    // not bind it, and the result must not declare it. The declared empty `e` that no
    // operation touched still enumerates and is still written.
    let update = "DELETE DATA { GRAPH <http://example.org/g1> { \
        <http://example.org/a> <http://example.org/b> <http://example.org/c> } } ; \
        INSERT { ?g <http://example.org/p> \"var-enumerated\" } \
        WHERE { GRAPH ?g { BIND(1 AS ?x) } }";
    let body = update_trig(
        "<http://example.org/g1> { <http://example.org/a> <http://example.org/b> <http://example.org/c> }\n",
        update,
    );
    assert_eq!(body.trim(), "", "{body}");
    let body = update_trig(
        "<http://example.org/g1> { <http://example.org/a> <http://example.org/b> <http://example.org/c> }\n\
         GRAPH <http://example.org/e> {}\n",
        update,
    );
    assert!(!body.contains("g1"), "{body}");
    assert!(
        body.contains("<http://example.org/e> <http://example.org/p> \"var-enumerated\" ."),
        "{body}"
    );
    assert!(body.contains("<http://example.org/e> {}"), "{body}");
}

#[test]
fn a_graph_emptied_by_delete_is_not_addressed_by_constant() {
    // The constant form agrees with the variable form: once `g1` loses its only quad,
    // `GRAPH <g1> { BIND(1 AS ?x) }` later in the same request matches nothing and
    // `GRAPH ?g` does not bind `g1`. The neighbours still match: a declared empty
    // graph no operation touched, a populated graph, and a graph repopulated after
    // DROP.
    let document = "@prefix ex: <http://example.org/> .\n\
        GRAPH ex:g1 { ex:a ex:b ex:c }\n\
        GRAPH ex:g2 { ex:a ex:b ex:c }\n\
        GRAPH ex:e {}\n";
    let probe = |graph: &str, label: &str| {
        format!(
            "INSERT {{ <http://example.org/r> <http://example.org/p> \"{label}\" }} \
             WHERE {{ GRAPH <http://example.org/{graph}> {{ BIND(1 AS ?x) }} }}"
        )
    };
    let delete = "DELETE DATA { GRAPH <http://example.org/g1> { \
        <http://example.org/a> <http://example.org/b> <http://example.org/c> } }";
    let body = update_trig(
        document,
        &format!(
            "{delete} ; {} ; {} ; {} ; \
             INSERT {{ <http://example.org/r> <http://example.org/saw> ?g }} \
             WHERE {{ GRAPH ?g {{ BIND(1 AS ?x) }} }}",
            probe("g1", "const-hit"),
            probe("e", "empty-hit"),
            probe("g2", "g2-hit"),
        ),
    );
    assert!(!body.contains("const-hit"), "{body}");
    assert!(
        !body.contains("<http://example.org/saw> <http://example.org/g1>"),
        "{body}"
    );
    for kept in ["empty-hit", "g2-hit"] {
        assert!(body.contains(kept), "{kept}: {body}");
    }
    let body = update_trig(
        document,
        &format!(
            "DROP GRAPH <http://example.org/g1> ; INSERT DATA {{ GRAPH <http://example.org/g1> {{ \
             <http://example.org/x> <http://example.org/y> <http://example.org/z> }} }} ; {}",
            probe("g1", "repopulated-hit")
        ),
    );
    assert!(body.contains("repopulated-hit"), "{body}");
}

#[test]
fn graph_lifetime_default_and_explicit_modes_reach_the_update_engine() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let input = fixture(dir.path());
    let request = "CREATE GRAPH <http://example.org/g>; COPY GRAPH <http://example.org/g> TO GRAPH <http://example.org/h>; CLEAR GRAPH <http://example.org/g>";
    for (mode, governed) in [None, Some("remember-empty"), Some("implicit")]
        .into_iter()
        .flat_map(|mode| [false, true].map(|governed| (mode, governed)))
    {
        let mut args = vec!["update", "--data", &input, "--to", "trig"];
        if let Some(mode) = mode {
            args.extend(["--graph-existence", mode]);
        }
        if governed {
            args.extend(["--fuel", "0"]);
        }
        args.push(request);
        let output = run(&args);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let body = String::from_utf8(output.stdout).expect("UTF-8 output");
        for graph in ["g", "h"] {
            assert_eq!(
                body.contains(&format!("<http://example.org/{graph}> {{}}")),
                mode != Some("implicit"),
                "{mode:?}: {body}"
            );
        }
        let last = args.len() - 1;
        args[last] = "CREATE GRAPH <http://example.org/g>; DROP GRAPH <http://example.org/g>";
        let dropped = run(&args);
        assert_eq!(dropped.status.code(), Some(0), "{dropped:?}");
        assert!(
            !String::from_utf8(dropped.stdout)
                .expect("UTF-8")
                .contains("<http://example.org/g>")
        );
    }
}

#[test]
fn invalid_graph_lifetime_and_governor_refusal_preserve_input_and_output() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let input = fixture(dir.path());
    let before = std::fs::read(&input).expect("input bytes");
    let target = dir.path().join("output.trig");
    std::fs::write(&target, b"unchanged output").expect("output preimage");
    let target = target.to_str().expect("UTF-8 target");
    let invalid = run(&[
        "update",
        "--data",
        &input,
        "--output",
        target,
        "--to",
        "trig",
        "--graph-existence",
        "invalid",
        INSERT,
    ]);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(
        String::from_utf8(invalid.stderr)
            .expect("error text")
            .contains("remember-empty or implicit")
    );
    for mode in ["remember-empty", "implicit"] {
        let refused = run(&[
            "update",
            "--data",
            &input,
            "--output",
            target,
            "--to",
            "trig",
            "--graph-existence",
            mode,
            "--fuel",
            "0",
            "CREATE GRAPH <http://example.org/g>; INSERT DATA { <http://example.org/new> <http://example.org/p> <http://example.org/value> }",
        ]);
        assert_eq!(refused.status.code(), Some(3), "{refused:?}");
        assert_eq!(refused.stdout, [] as [u8; 0]);
        assert_eq!(
            std::fs::read(target).expect("output unchanged"),
            b"unchanged output"
        );
        assert_eq!(std::fs::read(&input).expect("input unchanged"), before);
    }
}

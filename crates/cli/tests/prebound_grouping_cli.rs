// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL node expressions and the grouping constraint (SPARQL 1.1 §11.4), through the
//! BUILT `purrdf` binary.
//!
//! A node expression's SPARQL query runs with `$this`, the shape context and its
//! context's names bound — `value` inside an `sh:expression` constraint, a custom
//! function's arguments inside its body, a free evaluation's `--scope`. Those are the
//! ONLY variables an aggregate query may read outside an aggregate without grouping by
//! them, and at load (`validate`, `shacl pack`) the check holds for exactly them:
//!
//! * **Evaluation carries them past the group.** A pre-bound `$this` read above a
//!   `GROUP BY` is the bound node, not an unbound cell: `node-expr` outputs it per
//!   group, and an `sh:expression` reading `BOUND($this)` above an aggregate is true,
//!   so the constraint reports no violation.
//! * **Load refuses every other variable** — in an expression no focus node reaches,
//!   so nothing but the load-time check could see it — beside neighbours that read
//!   `$this`, `$value`, a function argument and a `--scope` name in
//!   aggregates and still load.

mod support;
use support::{code, run, stderr, stdout, write_file};

const DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:a a ex:T ; ex:p ex:o1, ex:o2 .\n",
    "ex:b ex:p ex:o3 .\n",
);

/// A shapes document: the prefixes, then `body`.
fn shapes(body: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
         @prefix ex: <http://example.org/> .\n{body}"
    )
}

#[test]
fn node_expr_outputs_the_pre_bound_focus_for_every_group() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let shapes_file = write_file(
        dir.path(),
        "shapes.ttl",
        &shapes(
            "ex:E sh:select \"SELECT $this WHERE { $this <http://example.org/p> ?o } GROUP BY ?o\" .\n",
        ),
    );
    let out = run(&[
        "node-expr",
        "--shapes",
        &shapes_file,
        "--focus",
        "<http://example.org/a>",
        "--expr",
        "<http://example.org/E>",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("node-expr outputs 2"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        stdout(&out),
        "<http://example.org/a>\n<http://example.org/a>\n",
        "one output per group (ex:o1, ex:o2), each the pre-bound focus node"
    );
}

#[test]
fn an_expression_reading_the_pre_bound_focus_above_an_aggregate_conforms() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let shapes_file = write_file(
        dir.path(),
        "shapes.ttl",
        &shapes(
            "ex:S a sh:NodeShape ; sh:targetClass ex:T ;\n\
             sh:expression [ sh:select \"SELECT ((BOUND($this) && COUNT(*) = 2) AS ?r) WHERE { $this <http://example.org/p> ?o }\" ] .\n",
        ),
    );
    let out = run(&["validate", "--shapes", &shapes_file, &data]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "{err}");
    assert!(
        err.contains("shacl conforms true\n") && err.contains("shacl results 0\n"),
        "BOUND($this) above the implicit group is true, so ex:a conforms: {err}"
    );
    // The neighbour that must still report: the same expression over a count that
    // does not hold.
    let failing = write_file(
        dir.path(),
        "failing.ttl",
        &shapes(
            "ex:S a sh:NodeShape ; sh:targetClass ex:T ;\n\
             sh:expression [ sh:select \"SELECT ((BOUND($this) && COUNT(*) = 3) AS ?r) WHERE { $this <http://example.org/p> ?o }\" ] .\n",
        ),
    );
    let out = run(&["validate", "--shapes", &failing, &data]);
    assert!(
        stderr(&out).contains("shacl conforms false\n"),
        "{}",
        stderr(&out)
    );
}

/// An `sh:expression` no focus node reaches (its shape targets nothing in the data).
fn unreached_expression(select: &str) -> String {
    shapes(&format!(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Nothing ;\n  sh:expression [ sh:select \"{select}\" ] .\n"
    ))
}

#[test]
fn an_unreached_expression_reading_a_non_key_variable_is_refused_at_load_and_pack() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let bad = write_file(
        dir.path(),
        "bad.ttl",
        &unreached_expression(
            "SELECT ?o WHERE { $this <http://example.org/p> ?o . ?n ?pp ?o } GROUP BY ?n",
        ),
    );
    let out = run(&["validate", "--shapes", &bad, &data]);
    assert_ne!(code(&out), 0, "validate must refuse the shapes graph");
    assert!(
        stderr(&out).contains("neither a GROUP BY key"),
        "{}",
        stderr(&out)
    );
    let product = dir.path().join("bad.product");
    let out = run(&[
        "shacl",
        "pack",
        "--shapes",
        &bad,
        "--out",
        product.to_str().expect("utf-8 path"),
    ]);
    assert_ne!(code(&out), 0, "pack must refuse the shapes graph");
    assert!(
        stderr(&out).contains("neither a GROUP BY key"),
        "{}",
        stderr(&out)
    );
    assert!(!product.exists(), "a refused pack writes no product");

    // The valid neighbour: `$this` and `$value` are bound by the expression's context,
    // so reading them in an aggregate projection loads and packs.
    let good = write_file(
        dir.path(),
        "good.ttl",
        &unreached_expression(
            "SELECT ((COUNT(?o) > 0 && BOUND($this) && BOUND($value)) AS ?r) \
             WHERE { $this <http://example.org/p> ?o }",
        ),
    );
    let out = run(&["validate", "--shapes", &good, &data]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let product = dir.path().join("good.product");
    let out = run(&[
        "shacl",
        "pack",
        "--shapes",
        &good,
        "--out",
        product.to_str().expect("utf-8 path"),
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
}

/// A node expression runs with no shape context, so `$currentShape` and `$shapesGraph`
/// are not among the names it binds: a REACHED `sh:expression` reading one in an
/// aggregate projection is refused when the shapes graph is loaded and when it is
/// packed — never admitted and then aborted at evaluation. The neighbour reading
/// `$this` and `$value` loads, packs, and answers the same from the restored product.
#[test]
fn a_reached_expression_reading_the_shape_context_is_refused_at_load_and_pack() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let reached = |select: &str| {
        shapes(&format!(
            "ex:S a sh:NodeShape ; sh:targetClass ex:T ;\n  sh:expression [ sh:select \"{select}\" ] .\n"
        ))
    };
    for (name, variable) in [("shape", "$currentShape"), ("graph", "$shapesGraph")] {
        let refused = write_file(
            dir.path(),
            &format!("{name}.ttl"),
            &reached(&format!(
                "SELECT ((BOUND({variable}) && COUNT(*) >= 0) AS ?r) WHERE {{ }}"
            )),
        );
        let out = run(&["validate", "--shapes", &refused, &data]);
        assert_ne!(
            code(&out),
            0,
            "{variable}: validate must refuse the shapes graph"
        );
        assert!(
            stderr(&out).contains("neither a GROUP BY key"),
            "{variable}: {}",
            stderr(&out)
        );
        let product = dir.path().join(format!("{name}.product"));
        let out = run(&[
            "shacl",
            "pack",
            "--shapes",
            &refused,
            "--out",
            product.to_str().expect("utf-8 path"),
        ]);
        assert_ne!(
            code(&out),
            0,
            "{variable}: pack must refuse the shapes graph"
        );
        assert!(
            !product.exists(),
            "{variable}: a refused pack writes no product"
        );
    }
    let good = write_file(
        dir.path(),
        "good.ttl",
        &reached(
            "SELECT ((BOUND($this) && BOUND($value) && COUNT(*) = 2) AS ?r) \
             WHERE { $this <http://example.org/p> ?o }",
        ),
    );
    let out = run(&["validate", "--shapes", &good, &data]);
    let verdict = stderr(&out);
    assert_eq!(code(&out), 0, "{verdict}");
    assert!(verdict.contains("shacl conforms true\n"), "{verdict}");
    let product = dir.path().join("good.product");
    let product = product.to_str().expect("utf-8 path");
    let out = run(&["shacl", "pack", "--shapes", &good, "--out", product]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let out = run(&["validate", "--shapes-product", product, &data]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("shacl conforms true\n"),
        "the restored product answers as the parsed shapes graph did: {}",
        stderr(&out)
    );
}

#[test]
fn a_function_body_reads_its_own_arguments_and_no_others() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let function = |arg: &str| {
        shapes(&format!(
            "ex:count a sh:ListParameterExpressionFunction ;\n\
             sh:parameter [ sh:path shnex:arg0 ] ;\n\
             sh:bodyExpression [ sh:select \"SELECT ((COUNT(?o) > 0 && BOUND(${arg})) AS ?r) \
             WHERE {{ $this <http://example.org/p> ?o }}\" ] .\n"
        ))
    };
    let declared = write_file(dir.path(), "declared.ttl", &function("arg0"));
    let out = run(&["validate", "--shapes", &declared, &data]);
    assert_eq!(
        code(&out),
        0,
        "the declared argument is bound: {}",
        stderr(&out)
    );
    let undeclared = write_file(dir.path(), "undeclared.ttl", &function("arg1"));
    let out = run(&["validate", "--shapes", &undeclared, &data]);
    assert_ne!(code(&out), 0, "an undeclared argument is no key");
    assert!(
        stderr(&out).contains("neither a GROUP BY key"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_free_evaluation_s_scope_names_are_bound() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let shapes_file = write_file(
        dir.path(),
        "shapes.ttl",
        &shapes(
            "ex:E sh:select \"SELECT ((COUNT(?o) > 0 && BOUND(?k)) AS ?r) WHERE { $this <http://example.org/p> ?o }\" .\n",
        ),
    );
    let node_expr = |scope: Option<&str>| {
        let mut args = vec![
            "node-expr",
            "--shapes",
            &shapes_file,
            "--focus",
            "<http://example.org/a>",
            "--expr",
            "<http://example.org/E>",
        ];
        if let Some(scope) = scope {
            args.extend(["--scope", scope]);
        }
        args.push(&data);
        run(&args)
    };
    let out = node_expr(Some("k=<http://example.org/z>"));
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>\n"
    );
    let out = node_expr(None);
    assert_ne!(code(&out), 0, "with no --scope, ?k is no key");
    assert!(
        stderr(&out).contains("neither a GROUP BY key"),
        "{}",
        stderr(&out)
    );
}

/// **A custom function's argument binds what its evaluation binds: `$this` alone.**
/// The argument is evaluated where the body reads it, in the empty scope (SHACL 1.2
/// Node Expressions §6.3), so a query inside it sees neither the call site's `$value`
/// nor an enclosing body's `$arg0`. Load refuses a read of either in an aggregate, and
/// lets the argument assign `?value`, which nothing binds there; the `$this`
/// neighbour conforms through `validate`, `shacl pack` and `--shapes-product`.
#[test]
fn a_custom_call_argument_binds_only_what_its_empty_scope_binds() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let id = "ex:id a sh:ListParameterExpressionFunction ;\n\
         sh:parameter [ sh:path shnex:arg0 ] ;\n\
         sh:bodyExpression [ sh:select \"SELECT ($arg0 AS ?r) WHERE { }\" ] .\n";
    // A function whose body passes its own `$arg0` into a nested call's argument.
    let outer = "ex:outer a sh:ListParameterExpressionFunction ;\n\
         sh:parameter [ sh:path shnex:arg0 ] ;\n\
         sh:bodyExpression [ ex:id ( [ sh:select \"SELECT ((COUNT(*) > 0 && BOUND($arg0)) AS ?r) \
         WHERE { $this <http://example.org/p> ?o }\" ] ) ] .\n";
    let document = |functions: &str, expression: &str| {
        shapes(&format!(
            "{id}{functions}ex:S a sh:NodeShape ; sh:targetClass ex:T ;\n  sh:expression {expression} .\n"
        ))
    };
    let argument = |select: &str| format!("[ ex:id ( [ sh:select \"{select}\" ] ) ]");
    let pack = |name: &str, shapes: &str| {
        let product = dir.path().join(format!("{name}.product"));
        let out = run(&[
            "shacl",
            "pack",
            "--shapes",
            shapes,
            "--out",
            product.to_str().expect("utf-8 path"),
        ]);
        (out, product)
    };
    for (name, functions, expression) in [
        (
            "value",
            "",
            argument(
                "SELECT ((COUNT(*) > 0 && BOUND($value)) AS ?r) \
                 WHERE { $this <http://example.org/p> ?o }",
            ),
        ),
        ("arg0", outer, "[ ex:outer ( true ) ]".to_owned()),
    ] {
        let refused = write_file(
            dir.path(),
            &format!("{name}.ttl"),
            &document(functions, &expression),
        );
        let out = run(&["validate", "--shapes", &refused, &data]);
        assert_ne!(code(&out), 0, "{name}: validate must refuse at load");
        assert!(
            stderr(&out).contains("neither a GROUP BY key"),
            "{name}: refused at load, not aborted at evaluation: {}",
            stderr(&out)
        );
        let (out, product) = pack(name, &refused);
        assert_ne!(code(&out), 0, "{name}: pack must refuse");
        assert!(
            !product.exists(),
            "{name}: a refused pack writes no product"
        );
    }
    for (name, select) in [
        (
            "this",
            "SELECT ((COUNT(*) > 0 && BOUND($this)) AS ?r) \
             WHERE { $this <http://example.org/p> ?o }",
        ),
        ("assigns", "SELECT ?value WHERE { BIND(true AS ?value) }"),
    ] {
        let good = write_file(
            dir.path(),
            &format!("{name}.ttl"),
            &document("", &argument(select)),
        );
        let out = run(&["validate", "--shapes", &good, &data]);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("shacl conforms true\n"),
            "{name}: {}",
            stderr(&out)
        );
        let (out, product) = pack(name, &good);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        let product = product.to_str().expect("utf-8 path");
        let out = run(&["validate", "--shapes-product", product, &data]);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("shacl conforms true\n"),
            "{name}: the restored product answers as the parsed shapes graph did: {}",
            stderr(&out)
        );
    }
}

/// **A named-parameter function's body scope follows each parameter's own
/// `sh:optional`, not its `sh:path` order.** `ex:a` sorts before `ex:b`, but here
/// `ex:a` is the optional one: the body may read the required `$b` in an aggregate
/// projection (a call always supplies it), and may not read the optional `$a` there
/// (a call may omit it). Each verdict is checked through `validate` and
/// `shacl pack`, and the admitted body answers through `--shapes-product` too.
#[test]
fn a_named_function_body_reads_its_required_parameters_whatever_their_iri_order() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let document = |read: &str, call: &str| {
        shapes(&format!(
            "ex:fn a sh:NamedParameterExpressionFunction ;\n\
             sh:parameter [ sh:path ex:a ; sh:optional true ] ;\n\
             sh:parameter [ sh:path ex:b ; sh:keyParameter true ] ;\n\
             sh:bodyExpression [ sh:select \"SELECT ((COUNT(?o) = 2 && BOUND(${read})) AS ?r) \
             WHERE {{ $this <http://example.org/p> ?o }}\" ] .\n\
             ex:S a sh:NodeShape ; sh:targetClass ex:T ;\n  sh:expression {call} .\n"
        ))
    };
    let pack = |name: &str, shapes: &str| {
        let product = dir.path().join(format!("{name}.product"));
        let out = run(&[
            "shacl",
            "pack",
            "--shapes",
            shapes,
            "--out",
            product.to_str().expect("utf-8 path"),
        ]);
        (out, product)
    };
    // The required `$b`, with and without the optional `ex:a` supplied.
    for (name, call) in [
        ("required-only", "[ ex:b true ]"),
        ("both", "[ ex:a true ; ex:b true ]"),
    ] {
        let good = write_file(dir.path(), &format!("{name}.ttl"), &document("b", call));
        let out = run(&["validate", "--shapes", &good, &data]);
        let verdict = stderr(&out);
        assert_eq!(code(&out), 0, "{name}: {verdict}");
        assert!(
            verdict.contains("shacl conforms true\n"),
            "{name}: {verdict}"
        );
        let (out, product) = pack(name, &good);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        let product = product.to_str().expect("utf-8 path");
        let out = run(&["validate", "--shapes-product", product, &data]);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("shacl conforms true\n"),
            "{name}: the restored product answers as the parsed shapes graph did: {}",
            stderr(&out)
        );
    }
    // The optional `$a`: no call is certain to bind it, so it is no key.
    let refused = write_file(
        dir.path(),
        "optional.ttl",
        &document("a", "[ ex:a true ; ex:b true ]"),
    );
    let out = run(&["validate", "--shapes", &refused, &data]);
    assert_ne!(code(&out), 0, "the optional $a is no key");
    assert!(
        stderr(&out).contains("neither a GROUP BY key"),
        "refused at load: {}",
        stderr(&out)
    );
    let (out, product) = pack("optional", &refused);
    assert_ne!(code(&out), 0, "pack must refuse");
    assert!(!product.exists(), "a refused pack writes no product");
}

/// A `sh:sparql` constraint whose query names `$shapesGraph` and `$currentShape`.
fn sparql_constraint(select: &str) -> String {
    shapes(&format!(
        "ex:S a sh:NodeShape ; sh:targetClass ex:T ;\n  sh:sparql [ sh:select \"{select}\" ] .\n"
    ))
}

/// **The shape context is pre-bound whether or not this run has a value for it.**
/// A shapes graph loads with `$shapesGraph` and `$currentShape` declared to the
/// grouping check, because whether the shapes graph is exposed under an IRI is a
/// choice of each validation, not of the shapes graph. So a `sh:sparql` constraint
/// reading either above its `GROUP BY` validates the same with `--shapes-graph` and
/// without it — an unnamed shapes graph leaves `$shapesGraph` unbound, it does not
/// turn the query into one its load would have refused.
#[test]
fn the_shape_context_is_a_group_constant_with_and_without_a_shapes_graph_iri() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    for (name, variable, expected) in [
        // An unnamed shapes graph leaves `$shapesGraph` unbound, so the row — a
        // violation — is selected only when no IRI is set. `$currentShape` is the
        // shape either way, so no row is.
        ("graph", "$shapesGraph", ["false", "true"]),
        ("shape", "$currentShape", ["true", "true"]),
    ] {
        let file = write_file(
            dir.path(),
            &format!("{name}.ttl"),
            &sparql_constraint(&format!(
                "SELECT $this WHERE {{ $this <http://example.org/p> ?o }} GROUP BY $this \
                 HAVING (COUNT(?o) = 2 && !BOUND({variable}))"
            )),
        );
        let projected = write_file(
            dir.path(),
            &format!("{name}-projected.ttl"),
            &sparql_constraint(&format!(
                "SELECT $this ((COUNT(?o) = 2 && !BOUND({variable})) AS ?unbound) \
                 WHERE {{ $this <http://example.org/p> ?o }} GROUP BY $this \
                 HAVING (COUNT(?o) = 2 && !BOUND({variable}))"
            )),
        );
        for (shapes_graph, conforms) in [
            (None, expected[0]),
            (Some("http://example.org/sg"), expected[1]),
        ] {
            for file in [&file, &projected] {
                let mut args = vec!["validate", "--shapes", file.as_str()];
                if let Some(iri) = shapes_graph {
                    args.extend(["--shapes-graph", iri]);
                }
                args.push(&data);
                let out = run(&args);
                let verdict = stderr(&out);
                assert!(
                    verdict.contains(&format!("shacl conforms {conforms}\n")),
                    "{variable} with shapes graph {shapes_graph:?}, {file}: {verdict}"
                );
            }
        }
    }
}

/// **Assigning the shape context is refused where the shapes graph loads.** The
/// evaluation declares `$shapesGraph` and `$currentShape` pre-bound on every run, and
/// a pre-bound variable may not be reassigned — so the load refuses the query with
/// or without a shapes-graph IRI, rather than admitting it and aborting the
/// validation. The neighbour assigning a fresh variable loads and validates.
#[test]
fn assigning_the_shape_context_is_refused_at_load_and_a_fresh_variable_is_not() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let assigning = |variable: &str| {
        sparql_constraint(&format!(
            "SELECT $this WHERE {{ $this <http://example.org/p> ?o . BIND(?o AS ?{variable}) \
             FILTER(false) }}"
        ))
    };
    for shapes_graph in [None, Some("http://example.org/sg")] {
        let validate = |file: &str| {
            let mut args = vec!["validate", "--shapes", file];
            if let Some(iri) = shapes_graph {
                args.extend(["--shapes-graph", iri]);
            }
            args.push(&data);
            run(&args)
        };
        for variable in ["shapesGraph", "currentShape"] {
            let refused = write_file(dir.path(), &format!("{variable}.ttl"), &assigning(variable));
            let out = validate(&refused);
            assert_ne!(code(&out), 0, "?{variable}, {shapes_graph:?}: refused");
            assert!(
                stderr(&out).contains(&format!(
                    "the query assigns ?{variable}, which SHACL-SPARQL pre-binds"
                )) && stderr(&out).contains("--shapes"),
                "?{variable}, {shapes_graph:?}: refused as the shapes graph loads: {}",
                stderr(&out)
            );
        }
        let fresh = write_file(dir.path(), "fresh.ttl", &assigning("fresh"));
        let out = validate(&fresh);
        assert_eq!(code(&out), 0, "{shapes_graph:?}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("shacl conforms true\n"),
            "{shapes_graph:?}: {}",
            stderr(&out)
        );
    }
}

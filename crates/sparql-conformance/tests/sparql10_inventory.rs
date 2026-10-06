// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native completeness and byte-freeze tripwires for the W3C data-r2 import.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// Observe the production guard, without adding assertions to its Python bridge.
fn loaded_receipt(script: &Path, root: &str) -> Output {
    Command::new("python3")
        .arg("-c")
        .arg("import runpy,sys; print(runpy.run_path(sys.argv[1])['GUARDED_ROOTS'][sys.argv[2]])")
        .arg(script)
        .arg(root)
        .output()
        .expect("read the production freeze registry")
}

#[test]
fn the_sparql10_payload_is_registered_with_its_exact_freeze_receipt() {
    const ROOT: &str = "crates/sparql-conformance/suite/w3c-sparql10";
    const RECEIPT: &str = "scripts/conformance-frozen/sparql-conformance-suite-w3c-sparql10.sha256";
    let workspace = purrdf_testkit::paths::workspace_root();
    // Observe the production guard's loaded registry. All expectations belong
    // to this native Rust test; this bridge only returns its configured value.
    let loaded = loaded_receipt(&workspace.join("scripts/check-corpus-frozen.py"), ROOT);
    assert!(
        loaded.status.success(),
        "{}",
        String::from_utf8_lossy(&loaded.stderr)
    );
    assert_eq!(
        String::from_utf8(loaded.stdout)
            .expect("receipt path is UTF-8")
            .trim(),
        RECEIPT
    );
    let receipt = fs::read_to_string(workspace.join(RECEIPT)).expect("frozen receipt exists");
    let mut files = BTreeSet::new();
    for line in receipt.lines() {
        let (digest, path) = line.split_once("  ").expect("SHA-256 receipt record");
        assert_eq!(digest.len(), 64);
        assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(files.insert(path), "duplicate frozen path {path}");
        assert!(workspace.join(ROOT).join(path).is_file(), "{path}");
    }
    assert_eq!(files.len(), 875, "every pinned upstream payload is frozen");
    assert!(files.contains("manifest.ttl"));
    assert!(files.contains("sort/extended-manifest.ttl"));
}

#[test]
fn a_missing_or_malformed_freeze_registry_is_refused() {
    let workspace = purrdf_testkit::paths::workspace_root();
    let fixture = purrdf_testkit::temp_dir!("sparql10-freeze-registry-").expect("fixture");
    let script = fixture.path().join("check-corpus-frozen.py");
    fs::copy(workspace.join("scripts/check-corpus-frozen.py"), &script).expect("copy guard");
    assert!(!loaded_receipt(&script, "fixture").status.success());
    let registry_dir = fixture.path().join("conformance-frozen");
    fs::create_dir(&registry_dir).expect("registry directory");
    let registry = registry_dir.join("roots.toml");
    fs::write(&registry, "[roots]\n").expect("empty registry");
    let empty = Command::new("python3")
        .arg(&script)
        .output()
        .expect("run the production freeze guard");
    assert!(
        !empty.status.success(),
        "an empty registry must not pass vacuously: {}",
        String::from_utf8_lossy(&empty.stdout)
    );
    // The valid neighbour observes an exact receipt from the same guard.
    fs::write(&registry, "[roots]\nfixture = \"receipt.sha256\"\n").expect("valid registry");
    let valid = loaded_receipt(&script, "fixture");
    assert!(valid.status.success());
    assert_eq!(valid.stdout, b"receipt.sha256\n");
    for malformed in [
        "[roots\n",
        "unrelated = \"receipt.sha256\"\n",
        "[roots]\nfixture = 1\n",
        "[roots]\nfixture = \"\"\n",
        "[roots]\n\"\" = \"receipt.sha256\"\n",
    ] {
        fs::write(&registry, malformed).expect("malformed registry");
        assert!(!loaded_receipt(&script, "fixture").status.success());
    }
}

#[test]
fn every_data_r2_leaf_and_the_root_closure_have_the_exact_pinned_inventory() {
    use purrdf_sparql_conformance::manifest;
    const GROUPS: &[(&str, usize)] = &[
        ("algebra", 14),
        ("ask", 4),
        ("basic", 27),
        ("bnode-coreference", 1),
        ("boolean-effective-value", 7),
        ("bound", 1),
        ("cast", 7),
        ("construct", 5),
        ("dataset", 12),
        ("distinct", 11),
        ("expr-builtin", 25),
        ("expr-equals", 15),
        ("expr-ops", 18),
        ("graph", 17),
        ("i18n", 5),
        ("open-world", 18),
        ("optional", 7),
        ("optional-filter", 5),
        ("reduced", 2),
        ("regex", 21),
        ("solution-seq", 13),
        ("sort", 14),
        ("syntax-sparql1", 81),
        ("syntax-sparql2", 53),
        ("syntax-sparql3", 51),
        ("syntax-sparql4", 12),
        ("syntax-sparql5", 2),
        ("triple-match", 4),
        ("type-promotion", 30),
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite/w3c-sparql10");
    let discovery = purrdf_sparql_conformance::discover(&root).expect("complete discovery");
    // The upstream root keeps its name and is discovered as the one index.
    assert_eq!(discovery.indexes.len(), 1);
    let (index, members) = &discovery.indexes[0];
    assert_eq!(index.relative, "manifest.ttl");
    assert_eq!(members.len(), GROUPS.len());
    let discovered = discovery.groups;
    let expected_leaves: BTreeSet<_> = GROUPS
        .iter()
        .map(|(group, _)| format!("{group}/manifest.ttl"))
        .collect();
    assert_eq!(
        discovered
            .iter()
            .map(|leaf| leaf.relative.clone())
            .collect::<BTreeSet<_>>(),
        expected_leaves
    );
    // The extended sort leaf is not discovered: no upstream suite root but the
    // extended one lists it, and its case is graded in its own row (see
    // `the_extended_sort_case_is_graded_against_the_sparql12_order`).
    assert_eq!(discovered.len(), 29);
    let mut declared = BTreeSet::new();
    for &(group, count) in GROUPS {
        let leaf = root.join(group).join("manifest.ttl");
        let cases = manifest::load(&leaf).unwrap_or_else(|error| panic!("{group}: {error}"));
        assert_eq!(cases.len(), count, "pinned {group} case count");
        for case in cases {
            assert_ne!(case.kind, manifest::TestKind::Unknown, "{}", case.iri);
            assert!(
                declared.insert(case.iri),
                "a case is executed by two leaves"
            );
        }
    }
    assert_eq!(declared.len(), 482);
    let closure = manifest::load(&root.join("manifest.ttl")).expect("pinned root closure");
    assert_eq!(closure.len(), 482);
    assert_eq!(
        closure
            .into_iter()
            .map(|case| case.iri)
            .collect::<BTreeSet<_>>(),
        declared
    );
    let extended = manifest::load(&root.join("extended-manifest-evaluation.ttl"))
        .expect("the upstream extended root");
    assert_eq!(extended.len(), 1);
    assert!(extended[0].iri.ends_with("#dawg-sort-11"));
    assert!(declared.insert(extended[0].iri.clone()));
    assert_eq!(declared.len(), 483);
}

/// Every vendored data-r2 file is accounted for: a manifest an upstream root
/// reaches, a file one of their cases reads, an upstream document, or one of
/// the exact files no manifest lists. Each unlisted file is pinned with the
/// reason it cannot run, so a re-vendor that adds an unlisted file, or a fix
/// that starts listing one of these, fails here rather than passing unseen.
///
/// None of the orphan or unreferenced files can be executed without authoring a
/// test: a DAWG test is what a manifest entry declares (its type, query, data and
/// result together), and they belong to no loadable entry. The one described
/// entry the upstream group leaves out is graded against the reading it lost to.
#[test]
fn every_vendored_data_r2_file_is_listed_or_an_exact_unlisted_remainder() {
    use purrdf_sparql_conformance::manifest::{self, ExpectedResult, TestKind};
    use std::path::PathBuf;
    /// The upstream manifest `open-world/sameTerm-manifest.ttl`, which no
    /// upstream aggregator includes, and the files only it names. Its six
    /// entries are blank nodes with no `rdf:type`, so they have neither a case
    /// IRI nor a test kind, and their three `qt:query` files are absent at the
    /// pinned commit.
    const ORPHAN: &[&str] = &[
        "open-world/sameTerm-manifest.ttl",
        "open-world/sameTerm.ttl",
        "open-world/sameTerm.srx",
        "open-world/sameTerm-StringSimpleLiteralCmp.srx",
        "open-world/sameTerm-eq.srx",
        "open-world/sameTerm-eq-StringSimpleLiteralCmp.srx",
        "open-world/sameTerm-not-eq.srx",
        "open-world/sameTerm-not-eq-StringSimpleLiteralCmp.srx",
    ];
    /// Payloads no manifest names at all: earlier spellings of listed cases
    /// (`query-eq2-2.rq` beside the listed `query-eq-2-2.rq`; `dataset-09.rq`
    /// beside the listed `dataset-09b.rq`, which reads `data-g3-dup.ttl` where it
    /// read `data-g3.ttl`) and results of cases the groups no longer declare. With no entry there is no declared
    /// pairing of query, data and result to run.
    const UNREFERENCED: &[&str] = &[
        "dataset/dataset-09.rq",
        "dataset/dataset-10.rq",
        "dataset/dataset-12.rq",
        "distinct/distinct-1-results.srx",
        "expr-builtin/result-plus-2.srx",
        "expr-equals/query-eq2-2.rq",
        "expr-equals/query-eq2-graph-1.rq",
        "expr-equals/result-eq2-2.ttl",
        "expr-equals/result-eq2-graph-1.ttl",
        "triple-match/data-03.ttl",
        "triple-match/dawg-tp-05.rq",
    ];
    /// The result of `dawg-optional-filter-005-simplified`, which the
    /// optional-filter manifest describes and leaves out of `mf:entries`
    /// ("Ambiguity in SPARQL 1.0"), listing the "preferred reading and SPARQL 1.1"
    /// `-not-simplified` case over the same query and data instead. It is graded
    /// below against that reading.
    const UNLISTED_ENTRY: &[&str] = &["optional-filter/expr-5-result-simplified.ttl"];
    /// Upstream documentation, which declares no test.
    const DOCUMENTS: &[&str] = &[
        "LICENSE",
        "README",
        "algebra-expressions.txt",
        "template.haml",
    ];
    /// The upstream aggregators: the whole suite, its evaluation and syntax
    /// halves, and the extended evaluation tests.
    const ROOTS: &[&str] = &[
        "manifest.ttl",
        "manifest-evaluation.ttl",
        "manifest-syntax.ttl",
        "extended-manifest-evaluation.ttl",
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite/w3c-sparql10");
    let canonical = |path: &Path| {
        path.canonicalize()
            .unwrap_or_else(|e| panic!("{e}: {path:?}"))
    };
    let mut listed: BTreeSet<PathBuf> = BTreeSet::new();
    let discovery = purrdf_sparql_conformance::discover(&root).expect("complete discovery");
    for manifest in discovery
        .groups
        .iter()
        .chain(discovery.indexes.iter().map(|(index, _)| index))
    {
        listed.insert(canonical(&manifest.path));
    }
    // The one member of the extended root, which discovery does not find.
    listed.insert(canonical(&root.join("sort/extended-manifest.ttl")));
    for name in ROOTS {
        let aggregator = root.join(name);
        listed.insert(canonical(&aggregator));
        for case in manifest::load(&aggregator).unwrap_or_else(|e| panic!("{name}: {e}")) {
            let mut read: Vec<PathBuf> = vec![case.query.clone()];
            read.extend(case.data.iter().cloned());
            read.extend(case.graph_data.iter().map(|(_, path)| path.clone()));
            read.extend(case.service_data.iter().map(|(_, path)| path.clone()));
            match &case.expected {
                ExpectedResult::Srx(path)
                | ExpectedResult::Srj(path)
                | ExpectedResult::Graph(path)
                | ExpectedResult::ResultSetRdf(path) => read.push(path.clone()),
                ExpectedResult::None => {}
                other => panic!("{}: an unexpected result carrier {other:?}", case.iri),
            }
            if case.kind == TestKind::QueryEval {
                let text = fs::read_to_string(&case.query).expect("query text");
                for (_, path) in purrdf_sparql_conformance::run::query_dataset_sources(&case, &text)
                    .unwrap_or_else(|e| panic!("{e}"))
                {
                    read.push(path);
                }
            }
            for path in read {
                listed.insert(canonical(&path));
            }
        }
    }
    let mut unlisted = BTreeSet::new();
    let mut walk = vec![root.clone()];
    while let Some(dir) = walk.pop() {
        for entry in fs::read_dir(&dir).expect("vendored directory") {
            let path = entry.expect("directory entry").path();
            let relative = path
                .strip_prefix(&root)
                .expect("below the root")
                .to_str()
                .expect("UTF-8 path")
                .to_owned();
            if path.is_dir() {
                if relative != "LICENSES" {
                    walk.push(path);
                }
            } else if !relative.ends_with(".license")
                && relative != "PROVENANCE.md"
                && !DOCUMENTS.contains(&relative.as_str())
                && !listed.contains(&canonical(&path))
            {
                unlisted.insert(relative);
            }
        }
    }
    let pinned: BTreeSet<String> = ORPHAN
        .iter()
        .chain(UNREFERENCED)
        .chain(UNLISTED_ENTRY)
        .map(|path| (*path).to_owned())
        .collect();
    assert_eq!(unlisted, pinned, "the exact unlisted remainder");
    // The orphan's entries are refused by the loader, beside the listed
    // open-world group in the same directory, which loads all eighteen: each is
    // a blank node, so it has no IRI to identify the case by, and none names a
    // test type.
    let orphan = manifest::load(&root.join(ORPHAN[0])).expect_err("anonymous entries do not load");
    assert!(
        orphan.contains("mf:entries member is not an IRI"),
        "got: {orphan}"
    );
    let orphan_text = fs::read_to_string(root.join(ORPHAN[0])).expect("orphan manifest");
    assert_eq!(orphan_text.matches("mf:name").count(), 6);
    assert!(!orphan_text.contains("Test ;") && !orphan_text.contains("rdf:type mf:Q"));
    assert_eq!(
        manifest::load(&root.join("open-world/manifest.ttl"))
            .expect("the listed group loads")
            .len(),
        18
    );
    for query in ["sameTerm.rq", "sameTerm-eq.rq", "sameTerm-not-eq.rq"] {
        assert!(!root.join("open-world").join(query).exists(), "{query}");
    }
    // The unlisted entry is graded: its query over its data, against its own
    // result, disagrees exactly where the listed reading agrees. SPARQL 1.1 and
    // 1.2 scope a FILTER to the group it is written in, `{{ }}` included
    // (§18.2.2), which is the not-simplified reading.
    let listed_reading = manifest::load(&root.join("optional-filter/manifest.ttl"))
        .expect("the listed group loads")
        .into_iter()
        .find(|case| {
            case.iri
                .ends_with("#dawg-optional-filter-005-not-simplified")
        })
        .expect("the listed reading");
    let outcome = purrdf_sparql_conformance::run::run(&listed_reading, None).expect("evaluation");
    purrdf_sparql_conformance::compare::compare(&listed_reading, &outcome)
        .expect("the SPARQL 1.1 reading passes");
    let mut simplified = listed_reading;
    simplified.iri = simplified.iri.replace("-not-simplified", "-simplified");
    simplified.expected = ExpectedResult::ResultSetRdf(root.join(UNLISTED_ENTRY[0]));
    assert!(
        purrdf_sparql_conformance::compare::compare(&simplified, &outcome).is_err(),
        "the reading SPARQL 1.1 settled against does not hold"
    );
    eprintln!(
        "W3C10 UNLISTED: files {}, orphan-manifest-files {}, unreferenced-payloads {}, \
         unlisted-entry-results {}",
        pinned.len(),
        ORPHAN.len(),
        UNREFERENCED.len(),
        UNLISTED_ENTRY.len()
    );
}

/// The one case of the upstream extended evaluation root, `dawg-sort-11`, is
/// graded in its own row, against the order SPARQL 1.2 defines, because the
/// order it froze is unreachable under RDF 1.2.
///
/// It tests the SPARQL 1.0 rule that a simple literal sorts before the
/// `xsd:string` of the same lexical form. RDF 1.2 makes the two spellings one
/// term (<https://www.w3.org/TR/rdf12-concepts/#section-Graph-Literal>: a simple
/// literal is syntactic sugar for an `xsd:string`), and SPARQL 1.2 §15.1
/// (<https://www.w3.org/TR/sparql12-query/#modOrderBy>) no longer has the rule,
/// so its frozen sequence `Alice, Bob, Eve, Fred, Alice, Bob, Eve, Fred` puts a
/// value before a smaller one. The grade executes every part of that claim:
/// the engine answers the SPARQL 1.2 order, its answer is the frozen result's
/// exact multiset of terms, and only the frozen order disagrees.
#[test]
fn the_extended_sort_case_is_graded_against_the_sparql12_order() {
    use purrdf_core::{SparqlResult, TermValue};
    use purrdf_sparql_conformance::{compare, manifest, rs_resultset, run};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite/w3c-sparql10");
    let cases = manifest::load(&root.join("extended-manifest-evaluation.ttl"))
        .expect("the upstream extended root");
    assert_eq!(cases.len(), 1, "the extended root lists exactly one case");
    let case = &cases[0];
    assert!(case.iri.ends_with("/sort/#dawg-sort-11"), "{}", case.iri);
    let outcome = run::run(case, None).expect("native evaluation");
    let run::RunOutcome::Eval {
        result: SparqlResult::Solutions {
            variables, rows, ..
        },
        ordered,
    } = &outcome
    else {
        panic!("a SELECT result")
    };
    assert!(*ordered, "the query orders its solutions");
    assert_eq!(variables, &["name"]);
    // SPARQL 1.2 §15.1: one value space, ascending, equal values adjacent.
    let sparql12: Vec<_> = ["Alice", "Alice", "Bob", "Bob", "Eve", "Eve", "Fred", "Fred"]
        .map(|name| vec![Some(TermValue::simple_literal(name))])
        .into();
    assert_eq!(rows, &sparql12);
    // The frozen result, read under RDF 1.2, holds exactly the same terms in an
    // explicit order that is not ascending: only the superseded order differs.
    let manifest::ExpectedResult::ResultSetRdf(frozen) = &case.expected else {
        panic!("an rs:ResultSet result")
    };
    let rs_resultset::RdfResult::Solutions { solutions, ordered } = rs_resultset::parse_result(
        &case.base,
        "text/turtle",
        &fs::read(frozen).expect("frozen result"),
    )
    .expect("a well-formed result set") else {
        panic!("SELECT rows")
    };
    assert!(ordered, "the frozen rows carry rs:index");
    assert_eq!(solutions.variables, ["name"]);
    let sorted = |rows: &[Vec<Option<TermValue>>]| {
        let mut keys: Vec<String> = rows.iter().map(|row| format!("{row:?}")).collect();
        keys.sort();
        keys
    };
    assert_eq!(
        sorted(&solutions.rows),
        sorted(rows),
        "the same multiset of terms"
    );
    let lexical = |row: &Vec<Option<TermValue>>| match &row[0] {
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        other => panic!("a literal, not {other:?}"),
    };
    assert!(
        solutions
            .rows
            .windows(2)
            .any(|pair| lexical(&pair[0]) > lexical(&pair[1])),
        "the frozen order puts a value before a smaller one"
    );
    assert!(
        compare::compare(case, &outcome).is_err(),
        "the frozen SPARQL 1.0 order cannot hold under RDF 1.2"
    );
    eprintln!(
        "W3C10 SUPERSEDED: graded {}, sparql12-order {}",
        cases.len(),
        cases.len()
    );
}

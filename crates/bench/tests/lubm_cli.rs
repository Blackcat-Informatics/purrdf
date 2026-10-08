// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Production process and independent parsed-graph acceptance, not generator counters.
use purrdf_bench::lubm::{PROFILE, Receipt};
use purrdf_iri::vocab::{owl, rdf};
use purrdf_lex::json::record::FromJson;
use purrdf_rdf::TermRef;
use purrdf_testkit::TempDir;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_lubm-corpus");
const ONTOLOGY: &str = "http://swat.cse.lehigh.edu/onto/univ-bench.owl";
const BASE: &str = "http://example.org/lubm/";

fn generate(
    root: &Path,
    name: &str,
    population: (u64, u64, u64),
    ontology: &str,
    base: &str,
    locale: &str,
) -> BTreeMap<String, Vec<u8>> {
    let (seed, index, count) = population;
    let output = Command::new(BIN)
        .args([
            "--universities",
            &count.to_string(),
            "--index",
            &index.to_string(),
            "--seed",
            &seed.to_string(),
            "--ontology",
            ontology,
            "--document-base",
            base,
            "--out",
            name,
        ])
        .current_dir(root)
        .env("LC_ALL", locale)
        .output()
        .expect("native generator process");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    fs::read_dir(root.join(name))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

struct Graph {
    triples: BTreeSet<(String, String, String)>,
    literals: BTreeMap<(String, String), String>,
    outgoing: BTreeMap<(String, String), BTreeSet<String>>,
    incoming: BTreeMap<(String, String), BTreeSet<String>>,
}
impl Graph {
    fn parse(bytes: &[u8]) -> Self {
        let dataset = purrdf_rdf::parse_dataset(bytes, "application/n-triples", None)
            .expect("strict native document");
        let mut triples = BTreeSet::<(String, String, String)>::new();
        let mut literals = BTreeMap::new();
        for row in dataset.quad_refs() {
            assert!(row.g.is_none());
            let (TermRef::Iri(subject), TermRef::Iri(predicate)) = (row.s, row.p) else {
                panic!("only named subjects and predicates");
            };
            for iri in [subject, predicate] {
                assert!(purrdf_iri::parse(iri).unwrap().has_scheme());
            }
            match row.o {
                TermRef::Iri(object) => {
                    assert!(purrdf_iri::parse(object).unwrap().has_scheme());
                    triples.insert((subject.into(), predicate.into(), object.into()));
                }
                TermRef::Literal {
                    lexical,
                    datatype,
                    language,
                    ..
                } => {
                    assert_eq!(
                        dataset.resolve(datatype),
                        TermRef::Iri(purrdf_xsd::datatype::XSD_STRING)
                    );
                    assert!(language.is_none());
                    assert!(
                        literals
                            .insert((subject.into(), predicate.into()), lexical.into())
                            .is_none()
                    );
                }
                _ => panic!("unexpected native term kind"),
            }
        }
        // A duplicate source statement must not disappear behind dataset set semantics.
        assert_eq!(
            std::str::from_utf8(bytes).unwrap().lines().count(),
            triples.len() + literals.len()
        );
        let mut outgoing = BTreeMap::<_, BTreeSet<_>>::new();
        let mut incoming = BTreeMap::<_, BTreeSet<_>>::new();
        for (subject, predicate, object) in &triples {
            outgoing
                .entry((subject.clone(), predicate.clone()))
                .or_default()
                .insert(object.clone());
            incoming
                .entry((predicate.clone(), object.clone()))
                .or_default()
                .insert(subject.clone());
        }
        Self {
            triples,
            literals,
            outgoing,
            incoming,
        }
    }
    fn objects(&self, subject: &str, predicate: &str) -> BTreeSet<&str> {
        self.outgoing
            .get(&(subject.to_string(), predicate.to_string()))
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }
    fn subjects(&self, predicate: &str, object: &str) -> BTreeSet<&str> {
        self.incoming
            .get(&(predicate.to_string(), object.to_string()))
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }
    fn one(&self, subject: &str, predicate: &str, expected: &str) {
        assert_eq!(self.objects(subject, predicate), BTreeSet::from([expected]));
    }
    fn named(&self, schema: &str, department: &str, class: &str) -> BTreeSet<&str> {
        let actual = self.subjects(rdf::TYPE, &format!("{schema}{class}"));
        let expected: BTreeSet<String> = (0..actual.len())
            .map(|index| format!("{department}/{class}{index}"))
            .collect();
        assert_eq!(actual, expected.iter().map(String::as_str).collect());
        actual
    }
}

fn verify_document(bytes: &[u8], filename: &str, ontology: &str, base: &str) {
    let graph = Graph::parse(bytes);
    let schema = format!("{ontology}#");
    let term = |local: &str| format!("{schema}{local}");
    let (university_index, department_index) = filename
        .strip_prefix("University")
        .unwrap()
        .strip_suffix(".nt")
        .unwrap()
        .split_once('_')
        .unwrap();
    let university = format!("http://www.University{university_index}.edu");
    let department =
        format!("http://www.Department{department_index}.University{university_index}.edu");
    let document = format!("{base}{filename}");
    graph.one(&document, rdf::TYPE, owl::ONTOLOGY);
    graph.one(&document, owl::IMPORTS, ontology);
    graph.one(&department, rdf::TYPE, &term("Department"));
    assert_eq!(
        graph.subjects(rdf::TYPE, owl::ONTOLOGY),
        BTreeSet::from([document.as_str()])
    );
    assert_eq!(
        graph.subjects(owl::IMPORTS, ontology),
        BTreeSet::from([document.as_str()])
    );
    assert_eq!(
        graph.subjects(rdf::TYPE, &term("Department")),
        BTreeSet::from([department.as_str()])
    );
    assert_eq!(
        graph
            .subjects(rdf::TYPE, &term("University"))
            .into_iter()
            .filter(|subject| !subject.starts_with("http://www.ExternalUniversity"))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([university.as_str()])
    );
    graph.one(&department, &term("subOrganizationOf"), &university);
    let all_types = |class: &str| graph.named(&schema, &department, class);
    for forbidden in ["Student", "Professor", "Chair", "Faculty"] {
        assert!(all_types(forbidden).is_empty());
    }
    let mut faculty = BTreeSet::new();
    let mut professors = BTreeSet::new();
    let mut faculty_publications = BTreeSet::new();
    let mut courses = BTreeSet::new();
    let mut graduate_courses = BTreeSet::new();
    for (class, minimum, maximum, pub_min, pub_max) in [
        ("FullProfessor", 7, 10, 15, 20),
        ("AssociateProfessor", 10, 14, 10, 18),
        ("AssistantProfessor", 8, 11, 5, 10),
        ("Lecturer", 5, 7, 0, 5),
    ] {
        let people = all_types(class);
        assert!((minimum..=maximum).contains(&people.len()));
        for person in &people {
            assert!(person.starts_with(&format!("{department}/{class}")));
            assert!(faculty.insert(*person));
            if class != "Lecturer" {
                professors.insert(*person);
            }
            graph.one(person, &term("worksFor"), &department);
            for property in ["name", "emailAddress", "telephone", "researchInterest"] {
                assert_ne!(
                    graph
                        .literals
                        .get(&(person.to_string(), term(property)))
                        .unwrap(),
                    ""
                );
            }
            for property in [
                "undergraduateDegreeFrom",
                "mastersDegreeFrom",
                "doctoralDegreeFrom",
            ] {
                let origins = graph.objects(person, &term(property));
                assert_eq!(origins.len(), 1);
                for origin in origins {
                    assert!(origin.starts_with("http://www.ExternalUniversity"));
                    graph.one(origin, rdf::TYPE, &term("University"));
                }
            }
            let taught = graph.objects(person, &term("teacherOf"));
            let mut undergraduate_count = 0;
            let mut graduate_count = 0;
            for course in taught {
                let classes = graph.objects(course, rdf::TYPE);
                if classes == BTreeSet::from([term("Course").as_str()]) {
                    undergraduate_count += 1;
                    assert!(courses.insert(course));
                } else {
                    assert_eq!(classes, BTreeSet::from([term("GraduateCourse").as_str()]));
                    graduate_count += 1;
                    assert!(graduate_courses.insert(course));
                }
                assert_ne!(
                    graph
                        .literals
                        .get(&(course.to_string(), term("name")))
                        .unwrap(),
                    ""
                );
            }
            assert!((1..=2).contains(&undergraduate_count) && (1..=2).contains(&graduate_count));
            let authored = graph.subjects(&term("publicationAuthor"), person);
            assert!((pub_min..=pub_max).contains(&authored.len()));
            for publication in authored {
                assert!(faculty_publications.insert(publication));
                graph.one(publication, rdf::TYPE, &term("Publication"));
            }
        }
    }
    let heads = graph.subjects(&term("headOf"), &department);
    assert_eq!(heads.len(), 1);
    assert!(heads.is_subset(&all_types("FullProfessor")));
    assert_eq!(courses, all_types("Course"));
    assert_eq!(graduate_courses, all_types("GraduateCourse"));
    assert_eq!(faculty_publications, all_types("Publication"));
    let groups = all_types("ResearchGroup");
    assert!((10..=20).contains(&groups.len()));
    for group in groups {
        graph.one(group, &term("subOrganizationOf"), &department);
    }
    let undergraduates = all_types("UndergraduateStudent");
    let graduates = all_types("GraduateStudent");
    for publication in &faculty_publications {
        let authors = graph.objects(publication, &term("publicationAuthor"));
        assert_eq!(authors.intersection(&faculty).count(), 1);
        assert!(authors.is_subset(&faculty.union(&graduates).copied().collect()));
        assert_ne!(
            graph
                .literals
                .get(&(publication.to_string(), term("name")))
                .unwrap(),
            ""
        );
    }
    assert!((8 * faculty.len()..=14 * faculty.len()).contains(&undergraduates.len()));
    assert!((3 * faculty.len()..=4 * faculty.len()).contains(&graduates.len()));
    let mut undergraduate_advisors = 0;
    for (people, graduate, load_min, load_max) in
        [(&undergraduates, false, 2, 4), (&graduates, true, 1, 3)]
    {
        for person in people {
            graph.one(person, &term("memberOf"), &department);
            for property in ["name", "emailAddress", "telephone"] {
                assert_ne!(
                    graph
                        .literals
                        .get(&(person.to_string(), term(property)))
                        .unwrap(),
                    ""
                );
            }
            let loads = graph.objects(person, &term("takesCourse"));
            assert!((load_min..=load_max).contains(&loads.len()));
            assert!(loads.is_subset(if graduate {
                &graduate_courses
            } else {
                &courses
            }));
            let advisors = graph.objects(person, &term("advisor"));
            assert!(advisors.is_subset(&professors));
            assert!(advisors.len() <= 1);
            if graduate {
                assert_eq!(advisors.len(), 1);
                let degree = graph.objects(person, &term("undergraduateDegreeFrom"));
                assert_eq!(degree.len(), 1);
                for origin in degree {
                    assert!(origin.starts_with("http://www.ExternalUniversity"));
                    graph.one(origin, rdf::TYPE, &term("University"));
                }
                assert!(graph.objects(person, &term("mastersDegreeFrom")).is_empty());
                assert!(
                    graph
                        .objects(person, &term("doctoralDegreeFrom"))
                        .is_empty()
                );
                let authored = graph.subjects(&term("publicationAuthor"), person);
                assert!(authored.len() <= 5);
                for publication in authored {
                    assert!(
                        graph
                            .objects(publication, &term("publicationAuthor"))
                            .intersection(&professors)
                            .next()
                            .is_some()
                    );
                }
            } else {
                undergraduate_advisors += advisors.len();
            }
        }
    }
    assert_eq!(undergraduate_advisors, undergraduates.len() / 5);
    let teaching = graph.subjects(rdf::TYPE, &term("TeachingAssistant"));
    let research = graph.subjects(rdf::TYPE, &term("ResearchAssistant"));
    assert!(teaching.is_subset(&graduates) && research.is_subset(&graduates));
    assert!(teaching.len() * 5 >= graduates.len() && teaching.len() * 4 <= graduates.len());
    assert!(research.len() * 4 >= graduates.len() && research.len() * 3 <= graduates.len());
    let mut assistant_courses = BTreeSet::new();
    for person in teaching {
        let assigned = graph.objects(person, &term("teachingAssistantOf"));
        assert_eq!(assigned.len(), 1);
        assert!(assigned.is_subset(&courses));
        for course in assigned {
            assert!(assistant_courses.insert(course));
        }
    }
    for person in research {
        graph.one(person, &term("worksFor"), &department);
    }
    // Every owned entity and reference has a declared class; schema/import objects are external vocabulary.
    for (subject, predicate, object) in &graph.triples {
        assert!(!graph.objects(subject, rdf::TYPE).is_empty());
        if predicate == rdf::TYPE {
            assert!(
                object == owl::ONTOLOGY
                    || [
                        "University",
                        "Department",
                        "FullProfessor",
                        "AssociateProfessor",
                        "AssistantProfessor",
                        "Lecturer",
                        "Course",
                        "GraduateCourse",
                        "Publication",
                        "ResearchGroup",
                        "UndergraduateStudent",
                        "GraduateStudent",
                        "TeachingAssistant",
                        "ResearchAssistant"
                    ]
                    .iter()
                    .any(|class| object == &term(class))
            );
        }
        if predicate != rdf::TYPE && predicate != owl::IMPORTS {
            assert!(!graph.objects(object, rdf::TYPE).is_empty());
        }
    }
}

fn verify_files(
    files: &BTreeMap<String, Vec<u8>>,
    seed: u64,
    index: u64,
    count: u64,
    ontology: &str,
    base: &str,
) {
    let receipt = Receipt::from_json(
        &purrdf_lex::json::read(std::str::from_utf8(&files["receipt.json"]).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt.profile, PROFILE);
    assert_eq!(
        (receipt.seed, receipt.index, receipt.universities),
        (seed, index, count)
    );
    assert_eq!(
        (&receipt.ontology, &receipt.document_base),
        (&ontology.to_string(), &base.to_string())
    );
    assert_eq!(files.len(), receipt.files.len() + 1);
    assert!(
        receipt
            .files
            .windows(2)
            .all(|pair| pair[0].name < pair[1].name)
    );
    let mut departments = BTreeMap::<u64, usize>::new();
    for identity in receipt.files {
        let bytes = &files[&identity.name];
        assert_eq!(identity.bytes, u64::try_from(bytes.len()).unwrap());
        assert_ne!(bytes.as_slice(), []);
        assert_eq!(
            identity.blake3,
            purrdf_hash::hex::Lower(purrdf_hash::blake3::hash(bytes).as_bytes()).to_string()
        );
        let university: u64 = identity
            .name
            .strip_prefix("University")
            .unwrap()
            .split('_')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        *departments.entry(university).or_default() += 1;
        verify_document(bytes, &identity.name, ontology, base);
    }
    assert_eq!(
        departments.keys().copied().collect::<Vec<_>>(),
        (index..index + count).collect::<Vec<_>>()
    );
    assert!(
        departments
            .values()
            .all(|number| (15..=25).contains(number))
    );
    for (university, number) in departments {
        let actual: BTreeSet<_> = files
            .keys()
            .filter(|name| name.starts_with(&format!("University{university}_")))
            .cloned()
            .collect();
        let expected: BTreeSet<_> = (0..number)
            .map(|department| format!("University{university}_{department}.nt"))
            .collect();
        assert_eq!(actual, expected);
    }
}

#[test]
fn named_seed_index_matrix_satisfies_every_published_graph_constraint() {
    let root = TempDir::for_unit_test().unwrap();
    for (name, seed, index, count) in [
        ("zero", 0, 0, 1),
        ("nonzero", 1, 7, 1),
        ("boundary", u64::MAX, u64::MAX - 1, 1),
        ("range", 42, 2, 2),
    ] {
        let files = generate(root.path(), name, (seed, index, count), ONTOLOGY, BASE, "C");
        verify_files(&files, seed, index, count, ONTOLOGY, BASE);
        if name == "zero" {
            // Native profile's independently captured sorted whole-corpus identity.
            assert_eq!(
                purrdf_hash::hex::Lower(
                    purrdf_hash::blake3::hash(&files["receipt.json"]).as_bytes()
                )
                .to_string(),
                "09438d399fc64246fa5f72fa127ca972212c491b97937dd820ec3d55cae22c6e"
            );
        }
    }
}

#[test]
fn independent_graph_controls_refuse_removed_relationships_and_invented_membership() {
    let root = TempDir::for_unit_test().unwrap();
    let files = generate(root.path(), "poison-source", (0, 0, 1), ONTOLOGY, BASE, "C");
    let (name, bytes) = files
        .iter()
        .find(|(name, _)| {
            Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "nt")
        })
        .unwrap();
    let text = std::str::from_utf8(bytes).unwrap();
    for property in [
        "memberOf",
        "teacherOf",
        "headOf",
        "advisor",
        "publicationAuthor",
        "undergraduateDegreeFrom",
        "mastersDegreeFrom",
        "doctoralDegreeFrom",
        "teachingAssistantOf",
        "subOrganizationOf",
    ] {
        let needle = format!("<{ONTOLOGY}#{property}>");
        let mut removed = false;
        let changed = text
            .lines()
            .filter(|line| {
                if !removed && line.contains(&needle) {
                    removed = true;
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert!(removed, "required graph relation {property} must exist");
        assert!(
            std::panic::catch_unwind(|| verify_document(changed.as_bytes(), name, ONTOLOGY, BASE))
                .is_err(),
            "removed {property} escaped independent oracle"
        );
    }
    let changed = format!(
        "{text}<http://www.Department0.University0.edu/GraduateStudent0> <{}> <{ONTOLOGY}#Student> .\n",
        rdf::TYPE
    );
    assert!(
        std::panic::catch_unwind(|| verify_document(changed.as_bytes(), name, ONTOLOGY, BASE))
            .is_err()
    );
}

#[test]
fn processes_paths_locale_ranges_and_configuration_have_declared_identity() {
    let root = TempDir::for_unit_test().unwrap();
    let first = generate(root.path(), "first", (42, 2, 1), ONTOLOGY, BASE, "C");
    let second_path = root.path().join("second");
    let second = generate(
        root.path(),
        second_path.to_str().unwrap(),
        (42, 2, 1),
        ONTOLOGY,
        BASE,
        "C.UTF-8",
    );
    assert_eq!(first, second);
    let range = generate(root.path(), "range", (42, 1, 3), ONTOLOGY, BASE, "C");
    for (name, bytes) in &first {
        if name != "receipt.json" {
            assert_eq!(bytes, &range[name]);
        }
    }
    let other_seed = generate(root.path(), "seed", (43, 2, 1), ONTOLOGY, BASE, "C");
    assert_ne!(first, other_seed);
    let other_schema = generate(
        root.path(),
        "schema",
        (42, 2, 1),
        "https://example.org/custom.owl",
        BASE,
        "C",
    );
    verify_files(
        &other_schema,
        42,
        2,
        1,
        "https://example.org/custom.owl",
        BASE,
    );
    assert_ne!(first, other_schema);
    let other_base = generate(
        root.path(),
        "base",
        (42, 2, 1),
        ONTOLOGY,
        "https://example.org/data/",
        "C",
    );
    verify_files(&other_base, 42, 2, 1, ONTOLOGY, "https://example.org/data/");
    assert_ne!(first, other_base);
}

#[test]
fn invalid_arguments_and_existing_outputs_never_succeed_or_overwrite() {
    let root = TempDir::for_unit_test().unwrap();
    let standard = [
        "--universities",
        "1",
        "--index",
        "0",
        "--seed",
        "0",
        "--ontology",
        ONTOLOGY,
        "--document-base",
        BASE,
        "--out",
        "out",
    ];
    for (flag, bad) in [
        ("--universities", "0"),
        ("--universities", "-1"),
        ("--seed", "1x"),
        ("--index", "18446744073709551615"),
        ("--ontology", "relative"),
        ("--ontology", "http://example.org/a#fragment"),
        ("--document-base", "http://example.org/no-slash"),
    ] {
        let mut args = standard.to_vec();
        let at = args.iter().position(|arg| *arg == flag).unwrap();
        args[at + 1] = bad;
        let result = Command::new(BIN)
            .args(args)
            .current_dir(root.path())
            .output()
            .unwrap();
        assert!(!result.status.success() && result.stdout.is_empty());
        assert!(!root.path().join("out").exists());
    }
    for args in [
        vec!["--universities"],
        vec!["--unknown"],
        vec!["--help", "--seed", "1"],
        standard.iter().copied().chain(["--seed", "2"]).collect(),
    ] {
        let result = Command::new(BIN)
            .args(args)
            .current_dir(root.path())
            .output()
            .unwrap();
        assert!(!result.status.success() && result.stdout.is_empty());
        assert!(!root.path().join("out").exists());
    }
    fs::create_dir(root.path().join("out")).unwrap();
    fs::write(root.path().join("out/sibling"), b"preserve").unwrap();
    let result = Command::new(BIN)
        .args(standard)
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(
        fs::read(root.path().join("out/sibling")).unwrap(),
        b"preserve"
    );
    assert!(!root.path().join("out/receipt.json").exists());
}

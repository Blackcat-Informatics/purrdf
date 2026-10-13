// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Rust-owned expectations at the installed Python extension boundary.
//! Run the ignored case from `uv run` in `bindings/python`; ordinary workspace
//! tests do not promise an installed extension. The bridge only observes Python
//! calls. Missing imports, missing APIs and wrong selection fail the invoked case.

use std::process::{Command, ExitCode};

use purrdf_lex::json::{self, Object, Value};
use purrdf_testkit::harness::{self, Trial};

const EX: &str = "http://example.org/";

// Python is required here to exercise PyO3 constructor/exception dispatch, class
// inheritance, returned term objects, and the uv-installed native module itself.
// No semantic expectation or assertion is implemented by this observation bridge.
const OBSERVE: &str = r#"
import importlib, json, sys
import purrdf

config = json.loads(sys.argv[1])
native = importlib.import_module("purrdf.purrdf_native")

def names(store):
    return sorted([[type(g).__name__, g.value] for g in store.named_graphs()])

def image(store):
    return [names(store), sorted(str(q) for q in store)]

def queried_names(store):
    return sorted([[type(row["g"]).__name__, row["g"].value] for row in store.query("SELECT ?g WHERE { GRAPH ?g {} }")])

def refusal(call):
    try:
        call()
        return None
    except Exception as error:
        return [type(error).__name__, str(error)]

records = []
for class_name in ["Store", "MutableDataset"]:
    cls = getattr(purrdf, class_name)
    for mode in [None, False, True]:
        make = (lambda: cls()) if mode is None else (lambda: cls(remember_empty_graphs=mode))
        store = make()
        g = purrdf.NamedNode(config["ex"] + "g")
        blank = purrdf.BlankNode("empty")
        record = {"class": class_name, "mode": mode, "initial": names(store)}
        record["positional"] = refusal(lambda: cls(True))
        record["create_return"] = store.add_graph(g)
        record["created"] = names(store)
        record["duplicate"] = refusal(lambda: store.add_graph(g))
        record["silent_duplicate"] = refusal(lambda: store.add_graph(g, silent=True))
        store.add_graph(blank)
        record["blank_created"] = names(store)
        record["queried"] = queried_names(store)
        record["invalid"] = [
            refusal(lambda: store.add_graph(purrdf.NamedNode("relative"), silent=True)),
            refusal(lambda: store.add_graph(purrdf.Literal("not a graph"), silent=True)),
            refusal(lambda: store.add_graph(purrdf.DefaultGraph(), silent=True)),
        ]
        quad = purrdf.Quad(purrdf.NamedNode(config["ex"] + "s"), purrdf.NamedNode(config["ex"] + "p"), purrdf.Literal("value"), g)
        store.add(quad)
        record["populated"] = names(store)
        store.remove(quad)
        record["last_removed"] = names(store)
        store.update("CREATE GRAPH <" + config["ex"] + "ordinary>")
        record["ordinary_create"] = names(store)
        store.update("CLEAR SILENT GRAPH <" + config["ex"] + "ordinary>")
        record["ordinary_clear"] = names(store)
        store.update("DROP SILENT GRAPH <" + config["ex"] + "ordinary>")
        record["ordinary_drop"] = names(store)
        outcome = store.update_governed("CREATE GRAPH <" + config["ex"] + "governed>", fuel=0)
        record["governed_create"] = [outcome.is_applied, outcome.evidence.consumed_in("fuel"), names(store)]
        cleared = store.update_governed("CLEAR SILENT GRAPH <" + config["ex"] + "governed>", fuel=0)
        record["governed_clear"] = [cleared.is_applied, cleared.evidence.consumed_in("fuel"), names(store)]
        dropped = store.update_governed("DROP SILENT GRAPH <" + config["ex"] + "governed>", fuel=0)
        record["governed_drop"] = [dropped.is_applied, dropped.evidence.consumed_in("fuel"), names(store)]
        store.update_governed("CREATE GRAPH <" + config["ex"] + "governed>", fuel=0)
        (store.checkpoint if class_name == "Store" else store.compact)()
        store.add_graph(purrdf.NamedNode(config["ex"] + "compacted"))
        record["after_compaction"] = names(store)
        record["queried_compacted"] = queried_names(store)
        token = purrdf.CancellationToken()
        token.cancel()
        before = image(store)
        stopped = store.update_governed("CREATE GRAPH <" + config["ex"] + "stopped>; CLEAR ALL", cancel=token, no_ceiling=True)
        record["cancelled"] = [stopped.is_applied, stopped.tripped.label, before, image(store)]
        store.add_graph(purrdf.NamedNode(config["ex"] + "after_stop"))
        record["after_stop"] = names(store)
        store.update("INSERT DATA { GRAPH <" + config["ex"] + "destination> { <" + config["ex"] + "s> <" + config["ex"] + "p> <" + config["ex"] + "o> } }")
        before = image(store)
        record["missing_copy"] = [refusal(lambda: store.update("COPY GRAPH <" + config["ex"] + "missing> TO GRAPH <" + config["ex"] + "destination>")), before, image(store)]
        loaded = []
        for fixture in config["fixtures"]:
            source = make()
            fmt = getattr(purrdf.RdfFormat, fixture["format"])
            source.load(fixture["input"], format=fmt)
            dumped = source.dump(format=fmt)
            back = make()
            back.load(dumped, format=fmt)
            loaded.append({"format": fixture["format"], "loaded": image(source), "back": image(back), "dump": dumped.hex(), "redump": back.dump(format=fmt).hex(), "loss": source.dump_with_loss(fmt).empty_named_graphs_dropped})
        record["loaded"] = loaded
        records.append(record)
print(json.dumps({"native": native.__file__, "records": records}))
"#;

fn config() -> Value {
    let trig = "<http://example.org/empty> {} <http://example.org/populated> { <http://example.org/s> <http://example.org/p> <http://example.org/o> . }";
    let jsonld = r#"[{"@id":"http://example.org/empty","@graph":[]},{"@id":"http://example.org/populated","@graph":[{"@id":"http://example.org/s","http://example.org/p":[{"@id":"http://example.org/o"}]}]}]"#;
    let trix = r#"<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/"><graph><uri>http://example.org/empty</uri></graph><graph><uri>http://example.org/populated</uri><triple><uri>http://example.org/s</uri><uri>http://example.org/p</uri><uri>http://example.org/o</uri></triple></graph></TriX>"#;
    Value::from(Object::new().with("ex", EX).with(
        "fixtures",
        vec![
        Object::new().with("format", "TRIG").with("input", trig),
        Object::new().with("format", "JSON_LD").with("input", jsonld),
        Object::new().with("format", "TRIX").with("input", trix),
    ],
    ))
}

fn names(iri_suffixes: &[&str], blank: bool) -> Value {
    let mut names: Vec<Vec<String>> = iri_suffixes
        .iter()
        .map(|suffix| vec!["NamedNode".to_owned(), format!("{EX}{suffix}")])
        .collect();
    if blank {
        names.push(vec!["BlankNode".to_owned(), "empty".to_owned()]);
    }
    names.sort();
    Value::from(names)
}

fn assert_error(value: &Value, class: &str, code: Option<&str>) {
    let parts = value.as_array().expect("an observed exception");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].as_str(), Some(class));
    if let Some(code) = code {
        assert!(
            parts[1].as_str().expect("exception text").contains(code),
            "{parts:?}"
        );
    }
}

fn check_record(record: &Value, class: &str, mode: Option<bool>) {
    let remembered = mode == Some(true);
    assert_eq!(record["class"].as_str(), Some(class));
    assert_eq!(record["mode"], mode.map_or(Value::Null, Value::Bool));
    assert_eq!(record["initial"], names(&[], false));
    assert_error(&record["positional"], "TypeError", None);
    assert_eq!(record["create_return"], Value::Null);
    let slots = if remembered { &["g"][..] } else { &[] };
    assert_eq!(record["created"], names(slots, false));
    if remembered {
        assert_error(
            &record["duplicate"],
            "ValueError",
            Some("rdf-ir-graph-already-exists"),
        );
    } else {
        assert_eq!(record["duplicate"], Value::Null);
    }
    assert_eq!(record["silent_duplicate"], Value::Null);
    assert_eq!(record["blank_created"], names(slots, remembered));
    assert_eq!(record["queried"], names(slots, remembered));
    let invalid = record["invalid"]
        .as_array()
        .expect("invalid name observations");
    assert_eq!(invalid.len(), 3);
    assert_error(&invalid[0], "ValueError", Some("iri-relative-no-base"));
    assert_error(&invalid[1], "TypeError", None);
    assert_error(&invalid[2], "TypeError", None);
    assert_eq!(record["populated"], names(&["g"], remembered));
    assert_eq!(record["last_removed"], names(slots, remembered));
    let ordinary = if remembered {
        &["g", "ordinary"][..]
    } else {
        &[]
    };
    assert_eq!(record["ordinary_create"], names(ordinary, remembered));
    assert_eq!(record["ordinary_clear"], names(ordinary, remembered));
    assert_eq!(record["ordinary_drop"], names(slots, remembered));
    check_adoption(record, remembered);
    check_load_dump(record, class, mode);
}

fn check_adoption(record: &Value, remembered: bool) {
    let after_governed = if remembered {
        &["g", "governed"][..]
    } else {
        &[]
    };
    check_zero_fuel_update(
        &record["governed_create"],
        &names(after_governed, remembered),
    );
    check_zero_fuel_update(
        &record["governed_clear"],
        &names(after_governed, remembered),
    );
    check_zero_fuel_update(
        &record["governed_drop"],
        &names(if remembered { &["g"] } else { &[] }, remembered),
    );
    let compacted = if remembered {
        &["g", "governed", "compacted"][..]
    } else {
        &[]
    };
    assert_eq!(record["after_compaction"], names(compacted, remembered));
    assert_eq!(record["queried_compacted"], names(compacted, remembered));
    let cancelled = record["cancelled"].as_array().expect("stop observations");
    assert_eq!(cancelled.len(), 4);
    assert_eq!(cancelled[0], Value::Bool(false));
    assert_eq!(cancelled[1].as_str(), Some("cancelled"));
    assert_eq!(
        cancelled[2], cancelled[3],
        "complete state is retained after refusal"
    );
    assert_eq!(cancelled[2][0], names(compacted, remembered));
    assert_eq!(cancelled[2][1], Value::from(Vec::<String>::new()));
    let after_stop = if remembered {
        &["g", "governed", "compacted", "after_stop"][..]
    } else {
        &[]
    };
    assert_eq!(record["after_stop"], names(after_stop, remembered));
    let copy = record["missing_copy"]
        .as_array()
        .expect("copy observations");
    assert_eq!(copy.len(), 3);
    if remembered {
        assert_error(
            &copy[0],
            "ValueError",
            Some("native-sparql-update-graph-missing"),
        );
        assert_eq!(
            copy[1], copy[2],
            "missing source cannot destroy destination"
        );
    } else {
        assert_eq!(copy[0], Value::Null);
        assert_eq!(copy[2][1], Value::from(Vec::<String>::new()));
    }
}

fn check_zero_fuel_update(observed: &Value, graph_names: &Value) {
    let parts = observed.as_array().expect("governed observations");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], Value::Bool(true));
    assert_eq!(parts[1].as_u64(), Some(0));
    assert_eq!(&parts[2], graph_names);
}

fn check_load_dump(record: &Value, class: &str, mode: Option<bool>) {
    let loaded = record["loaded"].as_array().expect("codec observations");
    assert_eq!(loaded.len(), 3);
    for (observed, format) in loaded.iter().zip(["TRIG", "JSON_LD", "TRIX"]) {
        assert_eq!(observed["format"].as_str(), Some(format));
        assert_eq!(observed["loaded"][0], names(&["empty", "populated"], false));
        assert_eq!(
            observed["loaded"][1],
            Value::from(vec![
                "<http://example.org/s> <http://example.org/p> <http://example.org/o> <http://example.org/populated>"
            ])
        );
        assert_eq!(
            observed["loaded"], observed["back"],
            "{class}/{mode:?}/{format}"
        );
        assert_ne!(observed["dump"].as_str().expect("dump bytes"), "");
        assert_eq!(
            observed["dump"], observed["redump"],
            "deterministic carrier bytes"
        );
        assert_eq!(observed["loss"].as_u64(), Some(0));
    }
}

fn installed_empty_graph_modes() -> Result<(), harness::Failed> {
    let output = Command::new("python3")
        .args(["-c", OBSERVE, &json::write_compact(&config())])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "installed Python bridge {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let observation = json::read(&String::from_utf8(output.stdout)?)?;
    let native = observation["native"]
        .as_str()
        .expect("installed native module identity");
    assert!(
        std::path::Path::new(native)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("so")
                || extension.eq_ignore_ascii_case("pyd")),
        "{native}"
    );
    eprintln!("Installed native module: {native}");
    let records = observation["records"]
        .as_array()
        .expect("six language-boundary observations");
    assert_eq!(records.len(), 6);
    for (index, class) in ["Store", "MutableDataset"].into_iter().enumerate() {
        for (mode_index, mode) in [None, Some(false), Some(true)].into_iter().enumerate() {
            check_record(&records[index * 3 + mode_index], class, mode);
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    harness::main(vec![
        Trial::test("installed_empty_graph_modes", installed_empty_graph_modes)
            .with_ignored_flag(true),
    ])
}

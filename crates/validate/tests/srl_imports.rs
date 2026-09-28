// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A SPARQL 1.2 RL rule set's `IMPORTS`, through the shared rules boundary every host
//! calls: resolved from the request's import table, refused by name when the table does
//! not supply one, and refused as unused when the table supplies one nothing imports.

use purrdf_validate::{RulesRequest, apply_rules_to_ntriples};

/// `ex:a ex:n 1`.
const DATA: &str = "<http://example.org/ns#a> <http://example.org/ns#n> \
\"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n";

/// A rule set importing `<http://example.org/more>`.
const IMPORTING: &str = "PREFIX ex: <http://example.org/ns#>\n\
IMPORTS <http://example.org/more>\n\
RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n";

/// The rule set `<http://example.org/more>` names: it reads the importer's inference.
const IMPORTED: &str = "PREFIX ex: <http://example.org/ns#>\n\
RULE { ?x ex:counted true } WHERE { ?x ex:q ?y }\n";

/// The importer's rule set without its `IMPORTS`.
const LONE: &str = "PREFIX ex: <http://example.org/ns#>\n\
RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n";

/// The refusal every host reports for the unsupplied import.
const UNRESOLVED: &str = "SPARQL 1.2 RL import <http://example.org/more> failed: no \
import-table entry supplies the rule set it names, and PurRDF fetches nothing it was not \
handed; supply that rule set's text under this IRI";

/// The refusal every host reports for the unused table entry.
const UNREACHED: &str = "the SPARQL 1.2 RL rule set's import closure never reaches \
<http://example.org/more>, so the import table's rule set would be read and never used; \
remove it";

/// The imported rule's inference, which only the resolved closure derives.
const COUNTED: &str = "<http://example.org/ns#a> <http://example.org/ns#counted> \
\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n";

fn run(srl: &str, imports: &[(&str, &str)]) -> Result<String, String> {
    apply_rules_to_ntriples(&RulesRequest {
        data_nt: DATA,
        srl: Some(srl),
        imports,
        ..RulesRequest::default()
    })
    .map(|outcome| outcome.inferred_ntriples)
    .map_err(|error| error.to_string())
}

#[test]
fn an_srl_import_resolves_from_the_import_table() {
    let inferred = run(IMPORTING, &[("http://example.org/more", IMPORTED)]).expect("resolves");
    assert!(
        inferred.contains(COUNTED),
        "the imported rule ran:\n{inferred}"
    );
    assert!(
        inferred.contains("<http://example.org/ns#a> <http://example.org/ns#q> "),
        "{inferred}"
    );
}

#[test]
fn an_srl_import_the_table_does_not_supply_is_refused_by_name() {
    assert_eq!(run(IMPORTING, &[]), Err(UNRESOLVED.to_owned()));
    // An entry under another IRI supplies nothing this import names.
    let other = run(IMPORTING, &[("http://example.org/other", IMPORTED)]);
    assert_eq!(other, Err(UNRESOLVED.to_owned()));
}

#[test]
fn an_import_table_entry_nothing_imports_is_refused_as_unused() {
    assert_eq!(
        run(LONE, &[("http://example.org/more", IMPORTED)]),
        Err(UNREACHED.to_owned())
    );
    // The same rule set with no table runs, and derives nothing the import would have.
    let lone = run(LONE, &[]).expect("runs");
    assert!(!lone.contains(COUNTED), "{lone}");
}

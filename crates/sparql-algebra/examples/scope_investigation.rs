// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic machine-readable candidate outcomes, layouts and allocation costs.

#[path = "../tests/support/scope_candidates.rs"]
mod candidates;

use std::collections::BTreeMap;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_lex::json::{self, Object, Value};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn outcome(result: Result<(), candidates::Diagnostic>, legal: bool) -> Value {
    let (status, explanation) = match result {
        Ok(()) => (
            if legal {
                "accepted_control"
            } else {
                "not_detected"
            },
            String::new(),
        ),
        Err(error) => ("detected", format!("{error:?}")),
    };
    Object::new()
        .with("status", status)
        .with("diagnostic", explanation)
        .into()
}

fn measurement(measured: Measurement) -> Value {
    Object::new()
        .with("allocations", measured.allocations)
        .with("requested_bytes", measured.requested_bytes)
        .with("retained_bytes", measured.retained_bytes)
        .with("peak_working_bytes", measured.peak_working_bytes)
        .into()
}

fn unavailable(result: Result<(), candidates::Diagnostic>) -> Value {
    let error = result.expect_err("this evidence has an explicit proof boundary");
    Object::new()
        .with("status", "not_established_from_input")
        .with("diagnostic", format!("{error:?}"))
        .into()
}

fn boundary_admission(result: Result<(), candidates::Diagnostic>) -> Value {
    match result {
        Ok(()) => outcome(Ok(()), true),
        Err(error) => unavailable(Err(error)),
    }
}

fn main() {
    let rows: Vec<Value> = candidates::cases()
        .into_iter()
        .map(|case| {
            let contract = candidates::Contract::prepare(&case.before).expect("valid frozen input");
            Object::new()
                .with("case", case.name)
                .with("legal", case.legal)
                .with("claim", case.claim)
                .with(
                    "existing",
                    outcome(candidates::existing(&case.after), case.legal),
                )
                .with(
                    "explicit",
                    outcome(candidates::explicit(&case.after), case.legal),
                )
                .with(
                    "contract",
                    outcome(
                        contract.check(&case.after, &case.correspondence),
                        case.legal,
                    ),
                )
                .into()
        })
        .collect();
    let unsupported: Vec<Value> = candidates::proof_boundaries()
        .into_iter()
        .map(|(name, evidence)| {
            Object::new()
                .with("case", name)
                .with("existing", outcome(candidates::existing(&evidence), true))
                .with("explicit", unavailable(candidates::explicit(&evidence)))
                .with(
                    "contract",
                    unavailable(candidates::Contract::prepare(&evidence).map(|_| ())),
                )
                .into()
        })
        .collect();
    let transitions: Vec<Value> = [
        (
            "remove_inner_ordinary_projection",
            candidates::projection_boundary(),
        ),
        ("rename_variable_graph", candidates::graph_name_boundary()),
        (
            "merge_ambiguous_copied_raw_owners",
            candidates::raw_owner_boundary(),
        ),
        (
            "rename_template_variable_graph",
            candidates::template_graph_boundary(),
        ),
        (
            "rename_template_variable_predicate",
            candidates::template_predicate_boundary(),
        ),
        (
            "change_describe_observer_target",
            candidates::describe_boundary(),
        ),
    ]
    .into_iter()
    .map(|(name, (before, after))| {
        Object::new()
            .with("transformation", name)
            .with(
                "existing_before",
                outcome(candidates::existing(&before), true),
            )
            .with(
                "existing_after",
                outcome(candidates::existing(&after), true),
            )
            .with(
                "explicit_after",
                boundary_admission(candidates::explicit(&after)),
            )
            .with(
                "contract_transition",
                unavailable(
                    candidates::Contract::prepare(&before)
                        .and_then(|contract| contract.check(&after, &BTreeMap::new())),
                ),
            )
            .into()
    })
    .collect();
    let layouts: Vec<Value> = candidates::layouts()
        .into_iter()
        .map(|(name, bytes)| {
            Object::new()
                .with("type", name)
                .with("bytes", u64::try_from(bytes).expect("layout fits"))
                .into()
        })
        .collect();
    let costs: Vec<Value> = candidates::measured_inputs()
        .into_iter()
        .map(|(name, evidence)| {
            let current = CurrentThreadWindow::open();
            candidates::existing(&evidence).expect("existing admission");
            let current = current.close();
            let typed = CurrentThreadWindow::open();
            candidates::explicit(&evidence).expect("typed admission");
            let typed = typed.close();
            let declarations_window = CurrentThreadWindow::open();
            let declarations = candidates::declare_variables(&evidence.query);
            let declarations_measurement = declarations_window.close();
            let prepare = CurrentThreadWindow::open();
            let contract = candidates::Contract::prepare(&evidence).expect("valid evidence");
            let prepare = prepare.close();
            let reuse = CurrentThreadWindow::open();
            contract
                .check(&evidence, &BTreeMap::new())
                .expect("reusable contract");
            let reuse = reuse.close();
            let row = Object::new()
                .with("input", name)
                .with(
                    "query_retained_size_bytes",
                    evidence.query.retained_size_bytes(),
                )
                .with(
                    "contract_retained_payload_bytes_excluding_map_nodes",
                    contract.retained_payload_bytes(),
                )
                .with("existing_check", measurement(current))
                .with("explicit_check", measurement(typed))
                .with(
                    "encoded_variable_declaration_prepare",
                    measurement(declarations_measurement),
                )
                .with("contract_prepare", measurement(prepare))
                .with("contract_reuse_check", measurement(reuse));
            drop(declarations);
            row.into()
        })
        .collect();
    let output = Object::new().with("format", "purrdf-scope-investigation-v1")
        .with("proof_boundary", "SELECT positive flat-term BGP/Join/Union with named source predicates, constant GRAPH, outer projection and inner projection without ordinary-binding removal; small VALUES records and CONSTRUCT template category records with flat subject/object, named source predicate and absent/constant graph. Trusted stable source-site provenance requires coherent repeated fields and injective raw source-owner keys. Recursive triple terms, variable query/template GRAPH names, variable predicates, inner projections removing ordinary bindings, ambiguous raw copies across owner boundaries and ASK/DESCRIBE observer forms are explicitly unavailable. UPDATE is not represented by this Query-based prototype. This is not a query-equivalence proof.")
        .with("runtime_freshness", Object::new().with("status", "not_established_from_ast")
            .with("explanation", "The same valid CONSTRUCT/INSERT AST can be executed by a fresh or identity-reusing allocator. Runtime rows and repeated executions require semantic evidence."))
        .with("layouts", layouts).with("outcomes", rows).with("proof_boundaries", unsupported)
        .with("unavailable_transformations", transitions).with("allocation_costs", costs);
    println!("{}", json::write_pretty(&output.into()));
}

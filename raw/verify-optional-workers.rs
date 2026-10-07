// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::BTreeSet;

fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1).expect("perf script path");
    let input = std::fs::read_to_string(path)?;
    let mut predicate_workers = BTreeSet::new();
    let mut vm_workers = BTreeSet::new();
    let mut samples = 0;
    for sample in input.split("\n\n") {
        let Some(header) = sample.lines().next() else { continue };
        let Some(ids) = header.split_whitespace().nth(1) else { continue };
        let Some((pid, tid)) = ids.split_once('/') else { continue };
        if pid == tid { continue }
        let predicate = sample.lines().any(|line| {
            line.contains("purrdf_sparql_eval::binop::left_outer_join_filtered::<purrdf_core::ir::dataset::RdfDataset>::{closure#0}")
        });
        if !predicate { continue }
        samples += 1;
        predicate_workers.insert(tid.to_owned());
        if sample.lines().any(|line| {
            line.contains("purrdf_sparql_eval::vm::Linked<") && line.contains("::term::<")
        }) {
            vm_workers.insert(tid.to_owned());
        }
    }
    assert!(predicate_workers.len() >= 2, "actual OPTIONAL join-row closure on multiple non-caller TIDs");
    assert!(vm_workers.len() >= 2, "actual predicate VM term evaluation and join-row closure in the same samples on multiple non-caller TIDs");
    println!("PASS: {samples} actual OPTIONAL join-row samples, {} distinct non-caller predicate TIDs: {predicate_workers:?}", predicate_workers.len());
    println!("PASS: predicate VM term plus OPTIONAL join-row in the same sample on {} distinct non-caller TIDs: {vm_workers:?}", vm_workers.len());
    Ok(())
}

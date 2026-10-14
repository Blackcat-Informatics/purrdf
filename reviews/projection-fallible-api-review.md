<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Projection benchmark fallible-builder adaptation

VERDICT: SCOPED SOURCE PASS; helper census and strict bench compilation are pending. Read-only review of the sole dirty source path `crates/rdf/benches/projections.rs` at base `eb43e0b5452f`. No Cargo, gate, source, tooling or forge mutation by this reviewer.

The private benchmark `push_iri` still interns subject, predicate and object in the same order, then pushes the same default-graph quad. It now returns `Result<(), purrdf_lex::allocation::StorageError>` and propagates each original builder failure with `?`; `purrdf-lex` is already a normal crate dependency. The nine existing graph/OBO/SKOS fixture call sites immediately `expect` success with contextual messages. Thus successful fixture bytes, row counts, inputs, loops and benchmark measurements are unchanged. On a failed builder call the benchmark still stops rather than silently publishing a partial dataset; the failure is now named at the fixture caller rather than inside the helper. No builder or projection production behavior changed.

This is a legitimate adaptation to the fallible builder API and matches the ledger row's reason: the benchmark remains separate from crate-private `projections::util::push_iri_triple`, using the public builder. The four-operation source shape now matches the production helper's fallible sequence for a semantic reason, not by changing flags or output to evade the census. Standard rustfmt check and diff check passed per the owner; that is source formatting evidence only. The existing `[[distinct]]` row is still stale **until** the unchanged helper census actually recognizes the pair, and strict bench compilation must run on this dirty source. The other six full-gate helper groups, the separately required capacity-panic census and the remainder of the full gate remain open.

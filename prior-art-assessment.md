# Existing owner and repair

Read regime::DL_PROOF_GOLDEN_VECTORS/dl_proof_golden_vectors/check_dl_proof_golden_vectors and every host consumer. The native shared checker already compares actual proof and replay report bytes for all services. The packaged WASM entry and Python assertion call that same source. No replacement checker is needed.

The only existing writer was the ignored one-time schema_vectors::tests::seal_schema_vectors. It rewrote only schema cases while preserving historical service bytes, so it cannot own whole-artifact proof-layout evolution. Replaced it with the explicit Rust example refresh_dl_proof_vectors over every original case. Normal tests remain read-only. Existing schema controls/check() are retained; only the obsolete test sealer is removed.

Self-assessment only; independent dispatch is unavailable. Intake/recurrence evidence is in brief.json/prior-art.md. Do not infer zero related defects from the new issue number or empty prior commit search.

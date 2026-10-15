# Captured stale owner proof authorities

Original index: 40 stale proof errors; 40 exact sinks.
Constructor closure: 156 files; source:ba39e8037cd816b8.

| Authority | Previous literal identity | Current literal identity |
| --- | --- | --- |
| crates/lex/src/allocation.rs | source:5100728db53dae7f | source:bd1cfeddaeae116a |
| crates/rdf-core/src/ir/builder.rs | source:aa47200c5548da15 | source:285ec037c8fdd275 |
| crates/rdf-core/src/ir/dataset.rs | source:328419615aacbd47 | source:a85fabdda717b4ff |
| crates/rdf-core/src/ir/validate.rs | source:cc105b93beed4742 | source:bb7eaea66da9a620 |

The original ledger is unchanged. Both moved find_term_hashed sinks retain
source:bcb4a119fb003223 at frozen_terms.rs lines59/62. Excluded-path owner
coverage remains under correction; this capture is not a passing gate.

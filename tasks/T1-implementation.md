# Task1 typed mutable storage

VERDICT: PASS for Task1 implementation and focused qualification on the source captured in T1-source.sha256. Task2 production transfer/LOAD wiring and Task3 whole gate/host qualification are not claimed.

## Implementation

RecordKind and owned RecordValues are additive public core APIs. RdfDatasetBuilder::push_record and MutableDataset::insert_record borrow an exact physical record; remove_record removes only that role. The borrowed insertion signature avoids needless ownership and copying while retaining the accepted typed ingress contract. Existing DatasetMut remains whole-value: ordinary removal affects every visible physical role, and restoration restores matching base roles before ordinary classification.

Membership, suppression and insertion ordinals use role-qualified RecordKey. Same-role duplicates collapse; equal values in different roles coexist. Graph lifetime updates use the net physical cardinality change after an entire mutation, avoiding transient last-row withdrawals during conversions. Public added_len/suppressed_len remain unique caller-value metrics, maintained through caller-origin role reference counts. Internal classification masks and derived targets do not inflate them.

The mutation owner alone normalizes ordinary classification. Indexed subject/graph origin groups and added reifier membership avoid a growing-delta scan for unrelated insertion. Reversible provenance distinguishes targets the conversion created or restored from independently authored targets. Removing a declaration restores only its owned origins and re-suppresses owned restored-base targets. Restoration of an ordinary-suppressed base row applies current declaration state; typed restoration retains the requested role.

Snapshot readers apply exact role masks without inferring roles. Sparse owner-produced base-conversion provenance preserves the original indexed base segment ahead of caller delta rows; only conversion-owned added targets enter that segment. Authored term admission order is retained, with a retention check after every primed term and no unused base-only priming. The unchanged complete probe-order golden passes. Admission and frozen retention price the same role-qualified masks and conversion set, including copied-index work. Failed publication adds no successful work.

Typed ingress shares the owning builder validator: subject/predicate/graph kinds, absolute IRIs, nested triple depth, literal components and canonical reifier positions. The iterative term fold and existing CDT blank-binding parser reject malformed composite literals before mutation; valid bound CDT bytes and scoped identity remain unchanged. No temporary dataset is built per record, duplicated classifier, new runtime dependency or feature is introduced.

Freeze uses the existing typed DatasetImporter and builder, retaining roles, surviving graph declarations, configured vocabulary/derivation metadata, source locations and base-owned non-RDF sidecars. Snapshot independence, exact same-/cross-role state transitions, graph modes and retained-reader lifetime remain covered by owning native tests.

## Actual qualification

All commands used the active managed SDK, private /opt/purrdf-401-qualification target/build/tmp, Cargo/libtest8, and an outer systemd user scope with MemoryMax64GiB and MemorySwapMax0. The live final scope properties were read while active; this does not claim a globally exclusive host or unobserved compiler containment.

Final session1373 actually exited0, scope purrdf-401-settled-task1-20261008, invocation a8a8fb8706354a92a75a6cee75c5fb2f:

- cargo clippy -p purrdf-core --all-targets --locked --jobs8 -- -D warnings: exit0.
- cargo test -p purrdf-core --lib --test import_view --test graph_existence_modes --test mutable_declared_graph_drain --test shared_views --test blank_publication --test cdt_blank_identity --locked --jobs8: exit0; library1230, blank-publication4, CDT12, graph modes11, import2, graph drain6, shared views59; all0failed/0ignored.
- cargo fmt --all --check: exit0. Source diff whitespace check: exit0.

Current controls include exact three-role randomized state models under both graph modes; typed and whole-value restoration; independent snapshots; wrong-kind/IRI/CDT refusals with no term publication; exact and one-below snapshot limits; classification collision cardinality; restored-base target ownership; indexed restoration after declaration changes; deterministic ordered conversion/undo histories; public metric promotion/demotion/undo; and independently authored target order/collision/undo.

## Preserved failures and evidence

The failing-before public count regression actually exited101 (base ordinary/annotation collision reported3 versus actual2). Original compile/import/lint failures, the complete old order failure and the intermediate term-priming failure remain retained. Two accidental integration filters selected zero tests; those outputs prove no integration acceptance. Final separate complete shared_views execution establishes all59 current controls and the unchanged golden. No failed command was relabeled PASS and no golden was regenerated.

T1-logs contains39 copied raw text command outputs, exit receipts and exact failed golden outputs. T1-evidence-copy.txt records byte/mode equality against preserved originals; T1-log-receipt.sha256 and T1-source.sha256 bind the evidence. No target/build, full checkout, source archive or private key payload was copied. Baseline production witnesses retained by root remain linked separately in graph-update-baseline-witnesses.md.

## Final inactive-arm correction and evidence reuse

The report-time performance review found that a combined-set guard could open the wrong role's base cursor. The constructor now scans the sparse set once to compute two role-presence booleans. Each cursor's false arm constructs no base iterator. Membership, role classification, indexed replay, count, admission and term order are unchanged. There is no existing native table-visit counter; independent source review verifies cursor construction instead of introducing instrumentation or a mirrored test.

Actual final session97654 exited0, scope purrdf-401-replay-guard-20261008, invocation bbbbf2d7ad34474491da6d6f44ec2d4b. Separate mutable55 and complete shared_views59 controls passed with0failed/ignored, followed by strict core all-target clippy0 and fmt0. These qualify the touched cursor paths; the unchanged full1230 and six integration targets from session1373 remain applicable. All final command exits are retained under T1-logs. Source is frozen with no remaining Task1 finding known. Normal hooks/commit/push are root-owned and have not run. Whole makecheck/wasm, semver and production transfer/LOAD remain Task2/Task3 responsibilities.

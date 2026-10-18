Share inline immutable owned-or-borrowed storage

Closes #597.

Prepared native proof contexts and retrieval profiles now use one OwnedOrBorrowed implementation in their existing core dependency. Both duplicate private enums and accessors are removed. Owning storage remains inline; immutable borrowing retains the original value and lifetime without Clone, ToOwned, allocation or a mutable accessor.

The helper ledger enforces the shared home. Manual constructor-source review found exactly one added module and unchanged existing constructors and capacity authorities. Only the constructor-set references in the 40 existing proof rows were renewed; their sinks, classes, authorities, reasons and the audit policies are unchanged.

Validation: full helpers-hygiene, strict affected all-target Clippy, two new carrier controls within the six-test ownership selection, four independently prepared proof-context tests, all 114 retrieval library tests, all 21 public boundary tests, and the actual borrowed-context lifetime compile-fail test passed. The public boundary suite covers the borrowed fuse entry point and exported evaluator bridge. Normal commit and push hooks passed.

CodeRabbit completed its substantive review with no actionable code comments and explicitly passed the linked-issue and scope checks. Targeted function documentation addresses its coverage warning. Its incomplete helper execution is resolved by the actual complete local gate; no missing dependency is concealed. The final source delta is documentation plus the reviewed source-reference renewal, so the runtime evidence and substantive review remain applicable. Local analysis is self-review; no independent local audit is invented.

The predicted integration is clean and the base has not advanced, so the affected qualification covers the actual candidate. Hosted CI is pending on the documentation update; no full-workspace or packaged WASM pass is claimed. The user authorized integration after passing applicable checks while later CI findings receive visible issue ownership.

Authoritative plan: .stage/share-owned-or-borrowed-storage-to/plan.md. Selected evidence directory: .stage/share-owned-or-borrowed-storage-to. The unfinished causal dependency graph remains owned by #596.

Defect-Class: duplicate-implementation

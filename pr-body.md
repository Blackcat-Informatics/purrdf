Recorded DL reasoning previously required choosing between proof recording and a caller stop signal; canonical input identity ran before stoppable ontology preparation. One signal now covers canonical collection/search/serialization, identity hashing, ontology preparation and decision search. Cancellation refuses typed preparation or returns an honest unknown search receipt; completed identities and replayed proofs are unchanged.

The existing stop interface now lives in core and Datalog re-exports it. Existing constructors and uncontrolled canonicalization share the same implementation. No dependency, dataset copy, clock, poison-budget change or alternative canonical algorithm was added. The RDF facade exposes the controlled canonical entry point.

Validation: 61 canonicalization tests; 93 reasoner tests (one existing ignored); seven Datalog stop/chase tests; seven new public recorded-stop cases, also executed successfully in wasm32/Node and registered in `make wasm-test`; strict all-target Clippy for core/Datalog/entail/RDF; formatting; normal commit hooks. Whole-workspace hosted CI is separate from these focused checks. This does not add shared-store/delta or rooted source-proof resolution.

Closes #582.

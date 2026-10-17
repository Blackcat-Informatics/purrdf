# Borrow the prepared proof checking context

The existing native context owns its prepared knowledge base and consumes Reasoner. Add a borrowing constructor to the same context and checker, preserving the owning constructor for callers that need independent lifetime ownership. The actual validation proof checker will borrow its independently prepared consumer reasoner after preparing the requested question.

## Task 1: Share context storage without copying source

Represent ownership explicitly inside DlProofContext with a lifetime tied to the borrowed knowledge base. All checking methods continue using the same immutable Kb, native clauses, source identity and calculus contract. Owning contexts remain static and preserve existing consumer preparation. Borrowed contexts may not outlive or coexist with mutation of their reasoner. The explicit storage choice adds no second proof algorithm, source filtering, semantic feature or dependency.

## Task 2: Exercise the real checker and its lifetime

Test borrowed versus owned replay over independently prepared consumers, both complete consistency outcomes and selected questions that intern additional concepts. Verify actual storage borrowing, unchanged preparation measurements, exact input/contract identity, changed-source and question rejection, absent recording refusal, original stop behavior, and later reuse of the reasoner after dropping the context. Add compile-fail documentation for mutation/lifetime exclusion. Run the actual validation proof regressions and native/packaged WASM registrations through existing targets.

## Task 3: Qualify and integrate

Run affected native tests, strict affected all-target Clippy and normal hooks. Inspect full review surfaces and current hosted checks, address actionable feedback and merge only through ghprsq. Keep downstream local-premise support, monotone ABox update and class-engine replacement explicitly open under the existing full plan. No local GMEOW corpus producer, full GMEOW gate, machine probe, resource cap or model/service action is authorized. Reviews are self-reviews unless external feedback actually arrives.

## Prepared verification dataflow

Source inspection after the first runtime checks found that validation canonicalizes the complete source in Reasoner::with_proofs and then immediately repeats it in ServiceProof::verify. Add verify_prepared against the sealed immutable consumer context. Factor source/question/trust-base binding into one helper and retain the existing supplied-source check as an explicit validation boundary using the same replay implementation. The production validation caller will use the prepared path. Owning/borrowed and source/prepared paths must return identical reports, with wrong source context, wrong question, tampered proof and stopping receipt still rejected. This eliminates one duplicate complete-source identity computation per actual validation check; no elapsed-time speedup is claimed without measurement. This strengthens the same prepared-store scope and leaves the larger ownership plan open.

The prepared native reasoner currently has to be consumed to obtain its proof checking context. A caller retaining one prepared source for repeated decisions cannot borrow that same immutable context for checking; it must give up the reasoner or prepare another one. This obstructs the shared native store boundary required by downstream logic execution.

Add a borrowed checking context over the caller-owned prepared native knowledge base. Reuse the existing source identity, prepared schema and single proof checker. Preserve the independently owned checking context for callers that need ownership to outlive the reasoner. Both are explicit storage/lifetime choices of the same context, not alternate proof semantics.

Acceptance:

- A recorded reasoner can lend its prepared knowledge base to the existing checker without cloning, reverse mapping or source canonicalization. The Rust lifetime prevents mutation or destruction while the borrow is live.
- Owning and borrowed contexts produce identical source/calculus identities and replay reports for independently prepared consumers. Keep exact service-question preparation and reject changed input, question and proof.
- Contexts retain the caller stop signal and preparation refusals. Unrecorded reasoners still refuse checking-context construction. Dropping a borrowed context leaves the reasoner usable for later decisions.
- Route the actual native validation proof-checking path through the borrowed form after preparing the consumer question. Do not use producer search state as checking authority or add a duplicate checker.
- Run affected native checks, strict Clippy, normal hooks and the packaged WASM proof registration lane where available. No new runtime dependency, source filtering, machine probe, resource cap or semantic feature.

This advances the shared prepared-store boundary needed by Blackcat-Informatics/gmeow-ontology#1803. It does not claim source-premise dependency extraction, incremental ABox updates or downstream class-engine replacement.

Prepared verification must reuse the immutable consumer context identity instead of immediately canonicalizing the whole source a second time. Retain the explicit supplied-source verification boundary, with both forms sharing source/question/trust-base binding and the same replay implementation. The actual validation caller must use prepared verification. Tests must compare complete replay reports across both storage and input choices and reject a different prepared source, wrong question, unprepared query context and stopped caller.

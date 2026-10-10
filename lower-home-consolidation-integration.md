# Lower-home consolidation

The complete nine-file proposal is `lower-home-consolidation.patch`; its durable preimages and formatted postimages are under `lower-home-consolidation-{preimages,postimages}/`, using `.rs.txt` suffixes. `lower-home-consolidation-identities.json` records source and postimage SHA-256 identities. No shipping source or ledger was edited by this helper.

This replaces the overlapping protocols reported in `implementation-native-owner-full-check-5.log` lines 349–394 and `implementation-native-owner-helper-correction-1.log`:

| Finding | Shared source body |
| --- | --- |
| CdtTerm / CdtValue / CdtLiteral / CdtKey `clone_admitted` | Private `tree::clone_entry!` generates the original public entry and resident Clone. Each type keeps its existing native iterative or lexical clone body. |
| CdtLiteral / CdtKey `release_with_memory` | Private `memory::release_entry!` computes each payload's checked original capacity, destroys the payload, and only then refunds its original Memory. |
| Resident `boxed_value` / `boxed_triple` | Typed defaults in the existing object-safe `Storage` trait instantiate one `native_box_factories!` body calling the lower fallible box allocator. Resident uses those defaults. |
| NativeGrant `boxed_value` / `boxed_triple` | Both typed trait adapters call one generic `NativeGrant::boxed`; the original refusal injection and actual lower allocation remain in that single body. |
| CdtStorage `boxed_value` / `boxed_triple` | Both typed trait adapters call one generic `CdtStorage::boxed`; the original first failure is latched once with the original exact construct string. |
| Integer / Decimal `try_from_lexical` and `FromStr` | Private `exact::error::lexical_entry!` generates both public protocols. The separate integer/decimal lexical readers and destination policies remain unchanged. |
| OwnedText String / borrowed str `From` | One local caller conversion macro consumes String/Arc or copies a borrowed slice through the standard Arc conversion. The intrinsically retained SharedText conversion remains its separate original native owner path. |

Ownership and failure review: native clone work stays inside `Memory::scope`, returns only surviving original capacity, and releases scratch by the same native bodies. Leaf refund still follows actual payload destruction. Box layout admission still occurs at `CdtMemory` before the typed factory, and physical allocation refusal remains distinct from malformed lexical input. Evaluator box refusal preserves `LexicalFrame::latch_failure` and its original `native composite value box` / `native composite triple box` constructs. XSD resident `FromStr` still selects Unbounded and owns the original lexical diagnostic; the fallible entry still selects Fallible and preserves ExactParseError. Arc text input moves its existing control owner; `.into()` is the identity conversion for an already supplied Arc.

The current writer renames are preserved: Memory and LexicalFrame use `admitted_bytes`, and numeric certificate methods use `required_bytes`. This patch does not touch the shared constructor macro home, numeric work/cost methods, stat_agg, or helper ledger.

Only these two narrowly typed adapter pairs may remain visible as isomorphic shims after macro consolidation:

- `purrdf_sparql_eval::cdt_fn::CdtStorage::boxed_value` and `purrdf_sparql_eval::cdt_fn::CdtStorage::boxed_triple`: the object-safe CDT Storage protocol needs two closed payload/return types, while one generic private body performs the allocator operation and first typed evaluator refusal latch. The original construct identifies which actual native box failed. A generic trait method would lose object safety; removing these overrides would lose the original evaluator failure latch.
- `purrdf_cdt::test_crate::value_relations::NativeGrant::boxed_value` and `purrdf_cdt::test_crate::value_relations::NativeGrant::boxed_triple`: the same closed trait types feed one generic fault-injection/allocator body. Keeping both adapters preserves the existing test's actual typed box-refusal boundary. Default trait factories cannot perform this fixture's injected refusal.

These are exact candidate members and rationales for the parent's ledger adjudication, not an exemption request for broader families. No ledger row was added here.

All existing acceptance assertions are retained, including native parsing, clone survival/refusal, compound relation cleanup, and typed box-failure fixtures. No new test duplicates the refactor. Rustfmt syntax/format and ordinary patch dry-run are the only helper checks; compiler, census, runtime tests, and full gate remain unrun by this helper and must bind to the writer-integrated source.

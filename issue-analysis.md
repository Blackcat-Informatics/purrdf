# Five-contract shared logic delivery

All five issue bodies and complete comments were captured once with stagectl; all are open and have zero comments. No shortened acceptance is accepted.

| Issue | Complete behavior and public proof |
|---|---|
| 451 | Recognized dateTimeStamp parsing with required timezone; exact dateTime subset, complement, finite enumerations, value identity and cardinality. Public Reasoner consistency has no data-range boundary; invalid/no-timezone values clash where required. |
| 453 | Recognize both RDF 1.2 tagged-string datatypes; disjoint from each other and xsd:string; exact infinite ranges, finite enumeration/complement and cardinality; every stored tagged literal gets its original singleton range. Public Reasoner and range-containment consumers use the same algebra. |
| 468 | Every emitted coverage cell that can admit unchecked XMLLiteral reports representation_approximation. Preserve deliberate checked-literal admission and distinguish both well-formed and ill-formed XML fixtures; do not pretend a regex decides balanced XML. |
| 474 | Input-derived class/property, relation propagation, coverage and emitter-definition bounds; exact checked arithmetic and existing shared depth law. Actual public compilation/emission above each former cap and depth-neighbor refusal, with scaling bench lanes. |
| 476 | Bottom roles empty in assertions, restrictions and role hierarchy; nonempty abstract interpretation even without ABox; cyclic RDF class expressions become finite definitional equations in the existing calculus. All four verbatim published cases agree, 262 total and zero ledger; existing RL negative corpus remains qualified. |

Source owners: xsd datatype/value/temporal/range; entail owl_dl parser/data/graph/construct inventory and query consumer; entail range-containment adapter; shapes schema_surface/json_schema/schema_catalog and the three emitters; sparql-conformance owl2 ledger and existing corpus runner. No separate numerical engine, reasoner or XML validator. No new dependency, feature, namespace default, third-party implementation, or selected arbitrary limit.

No named ADR for these owners was found in the repository. Governing authority is current AGENTS.md, .baseline/.goals, owner README contracts and the cited W3C datatype/OWL/RDF specifications already attached to those homes. Conformance corpus bytes stay frozen.

Current source observations: dateTimeStamp constant exists but enum/from_iri does not model it; dateTime whole/listed algebra needs timezone strata. Language literal IR identity is already normalized, but register_literals skips its singleton constraint. The reserved construct inventory omits both datatype IRIs. Unconstrained schema properties admit the shared unchecked XML branch while coverage says Exact. Fixed compiler/propagation/coverage and emitter-definition limits cause the reported refusals. Graph::init_state can have no abstract node; cyclic CeExtractor currently returns a parse error.

Scope completion remains NOT MET until the complete proposal is integrated and native/WASM/full required qualification proves every row. No build or test has been run by this helper.


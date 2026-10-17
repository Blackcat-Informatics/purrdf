# Native anyURI contract audit

This is an owner audit of the actual issue body, not a duplicate-title closure
or an independent review claim. No closure is claimed yet. Native implementation
and downstream work were delivered by other work; this delivery repairs the
remaining discovered SPARQL consumer and stale capability descriptions.

| Requirement | Current authoritative evidence |
|---|---|
| Native datatype/value primitive | crates/xsd/src/datatype.rs and value.rs register AnyUri, its native value, retained storage and formatting |
| Ordinary/governed parsing | Both parse doors call simple::read_any_uri before allocation; exact_governance tests cover valid collapse, invalid XML and admitted storage accounting |
| Fixed lexical mapping | any_uri::lexical_mapping_preserves_everything_except_xml_whitespace exercises empty/relative/fragment references, Unicode, XML 1.1 controls, invalid characters and absence of URL normalization |
| Disjoint identity/order | any_uri::primitive_identity_is_disjoint_and_unordered checks collapsed equality, URI/string separation, absence of ordering/EBV and rejected numeric operations through the native ops home |
| Range and cardinality soundness | range.rs TextSpace::AnyUri, collapsed-character lengths and native finite-space machinery; the portable ranges test checks intersections, complements, enumeration deduplication, character lengths, exact empty-length population and a large lower bound. The range module separately tests finite character-space exhaustion |
| Entailment consumer | owl_dl parser constructs DataRange::Datatype for modelled URI; data::native_any_uri_is_exact_while_an_unmodelled_datatype_is_reported preserves the real opaque boundary. Stale unsupported-URI examples in range/parser/report are corrected locally |
| Native/WASM execution | The current base 0994332d9 is the published XML 1.1 correction. Its complete XSD qualification and actual portable three-case text contract are recorded in the merged native work; current PR 579's XSD, conformance and WASM jobs pass. Reuse unchanged leaf behavior, not a new claim of local execution |
| Downstream replacement, no fallback | GMEOW PRs 1838 and 1839 are merged. They remove the local URI normalization/membership/equality/recognition/capacity branches and pre-trimming exceptions. PR 1839 records 82 passing native cases and regenerated verified attestations at 0994332d9. The final manifest readback now pins all five substrate dependencies to d839dd72a; this later main retains the same URI implementation. The URI-specific evidence remains applicable; broader class/corpus acceptance is not inferred from it |
| Remaining consumer correction | The old unmodelled-row classifier was bypassed when URI gained a native enum variant. Local changes restore URI's primitive cast row, parse its value for string conversion and use the native URI parser as the single collapse owner |

Primary raw evidence: issue571-current.txt, closed-572-refresh.txt,
closed-575-refresh.txt, anyuri-downstream-cutover.json,
gmeow-recent-deliveries.json and gmeow-current-Cargo.toml under tasks/.
Normative cast rule: https://www.w3.org/TR/xpath-functions-31/#casting-from-primitive-to-primitive.
Primitive contract: https://www.w3.org/TR/xmlschema11-2/#anyURI.

The corrected operator table passes all 23 cases; the public governed target
passes all three cases on native and actual WASM, including both primitive
directions, normalized conversions and malformed URI refusal. Strict all-target
compilation for sparql-eval, xsd and entail passes with warnings denied. Normal
commit hooks pass at f329f8120. The original full native workspace run separately
executes all three any_uri leaf controls successfully. Their implementation is
unchanged by this consumer repair, so those native results and the published leaf
portable evidence remain applicable. Logs are indexed in validation.md.

The missing local recipes now pass, including the complete release WASM build.
Independent source review covers all eight repair files with no actionable
finding. The final inventory-only head preserves runtime source unchanged.
Final hosted checks and actual integration remain required. The original failed integration
run remains a failure. Full unrelated GMEOW class/chase ownership and broad
corpus/browser acceptance remain outside this URI-specific contract and are not
claimed complete. No new compatibility path or second datatype implementation
has been created.

Final downstream manifest: tasks/gmeow-final-Cargo.toml. The current GMEOW Stage
index separately retains unfinished authenticated corpus/class-engine work;
neither that broader goal nor issue 566 is counted complete by this URI audit.

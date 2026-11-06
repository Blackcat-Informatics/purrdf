Finalize five pre-v4 API boundaries so later work can extend their settled owners and refusal families:

- JSON Schema accepts an explicit regex/reference policy and reports actual attempted/maximum reference counts, preserving cycles, descent resets and defaults.
- ShEx returns typed matcher exhaustion across ordinary, exact, shape-map and retained validation; CLI/Python callers preserve the refusal and caller-selected limits. Validation options are extensible.
- Datalog term inspection is non-exhaustive with explicit unsupported-form handling.
- Structured Horn programs use validated constructors and immutable inspection. Original construction operands survive typed failures, and clause position supplies the rule index.
- Structured truth probes distinguish established WFS values from incomplete, unevaluated and nonground probes. Negative dependencies are demanded before establishing absence, fixing false successful negations.

Ground execution, existing term/clause/proof identities and successful serialized output retain their meaning. Migration documentation and actual consumers are updated. This does not qualify symbolic execution or independently authenticate the existing Boolean proof-checker callback.

Validation passed: normal optimized native JSON Schema API 34 cases plus one external consumer; ShEx 157 cases; structured resolver 26 cases plus three construction and two negative-demand cases; prior term/unifier selections 31 cases. Scoped strict release Clippy and actual production-consumer checks passed, including the combined Datalog/entailment/JSON Schema/ShEx/CLI/Python libraries. Actual Python ShEx smoke preserved matcher counts, valid neighbors and native XPath identity. Independent source reviews found no blocking findings; normal signed commit hooks passed. Hosted checks are pending. External GMEOW qualification remains unrun and outside this repository.

Closes #1177.
Closes #1175.
Closes #1145.
Closes #1141.
Closes #1142.

# Issue #699: Preserve validation requests across registered expression-function callbacks

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Preserve the actual validation request when SHACL expression functions are invoked through their registered SPARQL call forms. This is a concrete native caller defect and a prerequisite of #698's shared recursive run state, under the unchanged #569/#690 contracts.

Confirmed public reproduction on the existing constraint engine: a request disallowing only sh:Violation judges a warning-only inner shape conforming through direct shnex:conformsToShape and a direct custom node-expression function. The same question through registered SPARQL built-in and custom functions incorrectly returns false. The observed results for a foreign focus are [(native,true),(native custom,true),(SPARQL built-in,false),(SPARQL custom,false)]. Source shows both callbacks constructing fresh bindings with default request options.

Acceptance:

- Wire the real registered built-in and custom-expression callback paths through the caller's complete current validation request and source context. Preserve conformance-disallow masks, custom severity values, subclass-in-shapes behavior, selected profile/regex law and explicit shapes-graph context where each applies. Keep the callee's original function/model owner; do not substitute the caller's model by IRI.
- Transport invocation state through the existing function-call/registry boundary with one explicit production path. Avoid another validator, hidden global cache, ad-hoc message parsing, dependency changes or semantic features. Standalone raw SPARQL calls retain their documented default request when no validation request exists.
- The focus graph remains the graph of the actual calling query. Repeated validation, rules rounds, nested calls and worker chunks must not reuse a captured stale dataset or another request's state. Native serial/parallel and prepared/restored paths must agree. No hypothetical future fixed-point behavior is claimed by this child.
- Verify the actual public reproducer and its inverse custom-severity case, default neighbours, interned/foreign focuses, original input disposal and independent requests. Include applicable subclass/profile/graph controls and actual registered-function queries, native direct functions, targets/rules and complete-report callers. Preserve existing messages, hard failures, original source scopes and non-recursive report observations.
- Keep context ownership shared/borrowed rather than cloning whole models or diagnostic payloads per invocation. Preserve original default-path allocation/admission bounds and check the new scoped-request cost meaningfully. Compile real workspace/binding callers, run focused native semantic/resource checks and strict callers, required hooks, full hosted checks and current-head independent review. General semantics stay native; release WASM portability remains required.
- Land the complete boundary repair through ghprsq; verify actual integration/tree/archive/closure and clean owned products. Keep #698, #569 and #690 open with their complete remaining fixed-point/corpus/host contracts. Preserve the isolated #698 request-layout work and all original donors.

The reproducer and original failing runtime output are currently under the #698 Stage. Transfer only that useful test into the new clean child; its unrelated uncommitted request types are not part of this repair. No upstream dependency or external repository work is authorized. Target no more than four hours for this individual issue.


## Comments (0)


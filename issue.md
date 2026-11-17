# Issue #1356: Typed cause for the completion refusal of the SPARQL query boundary

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## What this adds

The completion refusal of the fallible SPARQL query boundary gets a typed cause. Its refusals are carried as a structured value that can be inspected, rather than as a rendered message.

The change adds the `CompletionRefusal` type to the SPARQL evaluator's governor, and the types it carries (`Certificate`, `ChildEdge`, `ChildRole`, `PrefixFidelity`), with their structured error, display and error implementations. Existing constructors and string-carrying variants stay in place for current callers.

## Provenance

Ported from the earlier structured-error branch for this work. The port is additive: the old behaviour stays, and the typed family is added alongside it.

## Acceptance

- The evaluator's tests pass, and clippy is clean under the workspace lints.
- Existing callers in dependent crates still compile.
- A focused test protects the completion refusal's structured fields.


## Comments (0)


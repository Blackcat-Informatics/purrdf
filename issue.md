# Issue #566: Unify native graph projection across default and named RDF worlds

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The resident dataset exposes only `project_named_graph(&str)`. A consumer cannot
select the actual default world using the same native graph-faithful,
graph-erasing operation across base assertions, reifier declarations and
annotations. Reconstructing those tables downstream duplicates a kernel job.

This surfaced while benchmarking GMEOW's recorded production worlds: its full
object-world assembler lists 179,177 default-world assertions, while a shortcut
that reads the authored default before the assembler's meta-level filtering
produces 180,062. That shortcut is not the production world and must be removed.

Replace the named-only projection with one native `project_graph` operation
whose explicit selector addresses the default graph or a named RDF graph.
Keep one implementation for all three native tables. Remove the superseded
named-only API and migrate its repository callers and documentation. Preserve
existing named-graph isolation and support graph names without converting
them into fabricated IRI strings. Invalid graph names must fail explicitly.

Acceptance:

- Select the true default graph and erase only that selected graph slot.
- Preserve every selected base assertion, reifier declaration and annotation;
  never infer statement-layer membership from a reifier's use as a subject.
- Isolate default, IRI-named and blank-named graphs even when the same reifier
  identity occurs in more than one graph.
- Missing and declared-empty graphs yield an empty selected dataset without
  importing content from another world.
- Keep named-world canonicalization and bundle identity controls passing.
- Use first-party Rust, no dependency, legacy alias or fallback.

Downstream integration is part of GMEOW's engine responsibility split. Its
benchmark must select the default world after complete production assembly,
using this native operation, and remove the authored-default shortcut.


## Comments (0)


# Issue #673: Preserve original source failures through columnar projection and writing

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Parent: #558. This is a complete source-failure preservation delivery within that existing contract; it does not replace or close the parent.

## Verified defect

On main 68d554993, `purrdf-columnar::project` formats the original `DatasetView::checked_read` failure into `ColumnarError::SourceRead { detail: String }`. The writer's iterative term resolver does the same to `DatasetView::resolve` failures. The visitor therefore emits only text, losing the original variant/fields and potentially invoking a caller error's Display before it can be returned. A third textual match is the recursive test reference, not a production boundary.

## Required delivery

- Preserve the original caller read error by value through public `project` and `write`, including initial/completed-read failure and iterative term resolution. Use a typed source/codec carrier at the existing columnar error home; do not add a second projection/resolver, string fallback, silent recovery or duplicate source-error implementation.
- Expose the original nested source fields through the single borrowed structured-error protocol and std error chain. Keep source identity, full-width values, ownership/destruction, refusal priority, deterministic byte output and human presentation. Do not clone or render the source error merely to transport/inspect it.
- Wire all actual callers, exports and the recursive reference through that production implementation. Breaking API correction is authorized for v4; remove the obsolete lossy columnar source variant rather than maintaining an alternative string path. Native codec reader errors and their existing inspection remain intact.
- Keep named/empty graphs, quoted terms, content-store validation and both Parquet compression modes on their current owners.

## Acceptance

- Exercise both public projection and writing with real faulting DatasetView producers at completed-read and term-resolution boundaries. Original custom source fields, including full-width counts, must survive without Display; verify original owner/clone/destruction behavior and zero-allocation borrowed inspection.
- Exercise the same original failures and exact typed fields through the actual packaged WASM runner, not compilation alone.
- Preserve codec goldens, successful deterministic round trips and existing malformed/limit inspection controls; compile affected production callers early and qualify native/portable surfaces. Run normal required hooks without bypass.

## Retained parent scope

Columnar Malformed detail strings still need the full producer audit and typed native/data reasons under #558, as do other shipping families and complete host/inventory acceptance. This child owns source-failure transport only. The four existing parent donor worktrees remain preserved; no ambiguous setup is forced.


## Comments (0)


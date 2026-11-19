# Issue #1319: Python GTS fold view ignores read diagnostics; the seeking reader accepts an unclassified tail

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

The Python GTS fold view ignores the diagnostics of the read it is built from. A file that the reader reports as damaged produces a successful, partly or wholly empty view and no exception.

Read on main, not run:

- `bindings/python/src/py_gts_view.rs` builds the fold view and the relational rows from `reader::read(data, true, None)` (near lines 52 and 315) and does not inspect `graph.diagnostics`.
- `reader::read` reports damage as diagnostics on a returned value, not as an error. For example a file whose first item is not a header returns an empty graph with `DamagedFrame`, "first item is not a header"; a non-map item among a segment's frames returns the earlier content with `gts-frame-not-map`.
- The other consumers of the same reader do fail: the import into an RDF dataset, `read_graph` and the command line all turn any fold diagnostic into an error.

So the same bytes raise on the command line and return an empty or truncated result in Python.

A related quiet path: the seeking reader accepts a non-map item in a segment's tail after its index frame and reports it only if that frame is fetched, so a catalog over a file with unknown trailing material is returned as complete.

## Expected

- The Python view raises on a fold diagnostic, as the dataset import does, or returns the diagnostics where a caller must see them. Either way a damaged file is not a silent success.
- The seeking reader reports trailing items it cannot classify when the catalog is built, or exposes that the segment has an unverified tail, so that a caller can refuse it.

## Acceptance

- A test per surface: a file whose first item is not a header; a valid file followed by a non-map item. Each raises or reports in Python, and each is reported by the seeking reader.
- A neighbouring valid file still reads as before on both surfaces.
- No change to what the low-level reader returns.


## Comments (0)


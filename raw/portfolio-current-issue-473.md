# Issue #473: Python: iterating a SPARQL solution row with 'for v in row' raises KeyError

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

`for v in row` over a solution row raises KeyError instead of yielding values. rdflib 7.6's ResultRow is a tuple, so iteration yields values in projection order, with None for unbound. Fix the native row and the `purrdf.compat.rdflib` row so they match that, consistently with the .pyi stubs. Test bound, unbound, empty-projection and SELECT * rows. As neighbours, indexing by name and by position, `len` and `asdict` must keep working.

## Comments (2)

### paudley — 2026-10-07T03:00:25Z

### Preserved work-in-progress: solution-row iteration (branch paudley/python-row-iter)

This work had not landed when its host environment was retired. It is kept here as a patch so the code survives. It is unreviewed, in-progress work, not a finished change.

- Base commit (on main): `863c75e2c1e13001bcbe025820d257bd0ee49c4f`
- Files: 2; patch SHA-256 `54a80b26a3fb168ddef8634544cdc8b08558370af94231d8503502bc601598bd`
- Parts: 1 comments following this one, each a base64 block of the xz-compressed patch.

Restore by concatenating the parts in order into `parts.b64`, removing the fence lines, then:
```sh
git switch -c restore 863c75e2c1e1
base64 -d parts.b64 | xz -d > wip.patch && git apply --index wip.patch
```

<details><summary>Files changed</summary>

```
 bindings/python/python/src/purrdf/__init__.pyi |  9 ++++++-
 bindings/python/src/py_store/query.rs          | 96 +++++++++++++++++++++++++++++++++++++++++++++++-------------------
 2 files changed, 77 insertions(+), 28 deletions(-)
```
</details>


### paudley — 2026-10-07T03:00:26Z

Part 1 of 1 (i473)
```
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
```



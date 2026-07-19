<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# `purrdf-capi` — libpurrdf

The **purrdf semantic C-ABI** (purrdf parcel P8): a stable,
SemVer-disciplined `extern "C"` surface over the native `purrdf` RDF-1.2
stack. It is the rich companion to the permissive `libgts` C-ABI — where
`libgts` is transport/format only, **libpurrdf** exposes parse, serialize,
pattern iteration, copy-on-write mutation, SPARQL, SHACL validation/entailment,
and GTS container round-trip.

**One shared library, not two.** libpurrdf statically reuses the permissive
`purrdf-gts` Rust crate, so a language shim links **`libpurrdf` alone** and still
reads/writes `.gts` containers — no second `.so` to coordinate.

## Building

The C library, header, and pkg-config file are produced by
[`cargo-c`](https://github.com/lu-zero/cargo-c):

```sh
make capi-build                 # cargo capi build: libpurrdf.{so,a} + purrdf.h + purrdf.pc
make capi-install PREFIX=/usr   # cargo capi install into a prefix
make capi-check                 # verify the committed header is current + run the C smoke
make capi-header                # regenerate the committed include/purrdf.h after an ABI change
```

The committed header `include/purrdf.h` **is the ABI contract**; CI fails if it
drifts from the crate (`make capi-check`).

## Configured JSON-LD and YAML-LD

`purrdf_jsonld_context_compile` decodes the shared versioned options document
and returns an immutable, thread-safe `PurrdfJsonLdContext`. Reuse that handle
with `purrdf_serialize_jsonld_configured`, then free it with
`purrdf_jsonld_context_free`. The serializer accepts exactly one options byte
slice or compiled handle and preserves an optional caller YAML schema URL.

The three modes are explicit: byte-frozen `expanded`, caller-owned `context`,
and deterministic dataset-IRI `derived`. Context IRI and `@import` resolution is
restricted to the immutable registry in the options document; libpurrdf never
performs network context loading.

## Graph, tabular, and research-object projection carriers

`purrdf_project` and `purrdf_lift` expose the same canonical archive engine as
Rust, Python, WebAssembly, and the CLI. `purrdf_project` accepts all fourteen
profiles; `purrdf_lift` accepts the ten structurally bidirectional ones:

| Profile | Project | Lift |
| --- | :---: | :---: |
| `lpg-csv` | yes | yes |
| `neo4j-csv` | yes | yes |
| `open-cypher` | yes | yes |
| `graphml` | yes | yes |
| `csvw-exact` | yes | yes |
| `csvw-terms` | yes | no |
| `okf-terms` | yes | no |
| `obo-graphs` | yes | no |
| `skos` | yes | no |
| `croissant-1.1` | yes | yes |
| `ro-crate-1.3` | yes | yes |
| `datacite-4.6` | yes | yes |
| `dcat-3` | yes | yes |
| `frictionless-data-package-1` | yes | yes |

Configuration is mandatory profile-tagged JSON with caller-owned vocabulary,
identity, limits, and policy. Projection returns two independent caller-owned
`PurrdfBuffer` handles: canonical deterministic USTAR bytes and the versioned
loss-ledger JSON. Lift returns a caller-owned `PurrdfDataset` plus an independent
ledger buffer. Free every buffer with `purrdf_buffer_free` and the dataset with
`purrdf_dataset_free`.

`purrdf_project_with_assets` is the attached RO-Crate entry point. It accepts a
canonical payload-only USTAR under the configuration limits and routes it through
the same Rust ownership, byte-size, preview, and determinism checks. The profile
must be `ro-crate-1.3` and its configuration must declare `packaging: "attached"`.

The complete compiled
[`projection_roundtrip.c`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/examples/projection_roundtrip.c)
example parses Turtle, projects LPG CSV, writes the archive, lifts it, verifies
the quad count, and releases every handle. `make capi-check` compiles and
executes that example against the generated shared library and committed header.

## ABI contract (every entry point)

- **No unwinding across the boundary.** Every function runs inside
  `catch_unwind`; a caught panic becomes `PURRDF_STATUS_PANIC` (never a process
  abort across FFI).
- **`int32_t` status + out-params.** Fallible functions return a
  `PurrdfStatus` value (as `int32_t`) and write results through out-pointers. On
- **SemVer-frozen ABI.** The status enum is append-only; new fields/functions are
  additive. The current ABI is **0.1.0 (beta)** — the freeze *discipline* is in
  place, but the version stays pre-1.0 until a real C consumer and the rdflib
  shim exercise it. `purrdf_abi_version` reports it.

### Status codes

| Code | Value | Meaning |
|------|-------|---------|
| `PURRDF_STATUS_OK` | 0 | success |
| `PURRDF_STATUS_NULL_POINTER` | 1 | a required pointer was null |
| `PURRDF_STATUS_INVALID_UTF8` | 2 | a C string was not valid UTF-8 |
| `PURRDF_STATUS_INVALID_ARGUMENT` | 3 | a structurally invalid argument |
| `PURRDF_STATUS_UNSUPPORTED_FORMAT` | 4 | unknown media type / format id |
| `PURRDF_STATUS_PARSE_ERROR` | 5 | parse failed |
| `PURRDF_STATUS_SERIALIZE_ERROR` | 6 | serialize failed |
| `PURRDF_STATUS_QUERY_ERROR` | 7 | SPARQL evaluation failed |
| `PURRDF_STATUS_FREEZE_ERROR` | 8 | freezing a mutable graph failed |
| `PURRDF_STATUS_CURSOR_EXHAUSTED` | 9 | no more rows (a non-error terminal signal, `> 0`) |
| `PURRDF_STATUS_GTS_ERROR` | 10 | GTS container read/write failed |
| `PURRDF_STATUS_PANIC` | 100 | a panic was caught at the boundary |

## Ownership

- Every handle / buffer / error / cursor the library hands out has **exactly one
  matching `*_free`** (`purrdf_dataset_free`, `purrdf_graph_free`,
  `purrdf_cursor_free`, `purrdf_rowcursor_free`, `purrdf_buffer_free`,
  `purrdf_error_free`). Free each exactly once; freeing `NULL` is a no-op.
- **The C side never `free()`s a `PurrdfStr.ptr`.** A `PurrdfStr` borrows
  library-owned memory; copy the bytes out if you need them to outlive the
  borrow.

## Lifetimes (borrowed slices)

- A term view from `purrdf_cursor_next` borrows into the dataset arena; its
  `PurrdfStr` pointers are valid until the next `purrdf_cursor_next` on that
  cursor or `purrdf_cursor_free`. The cursor pins the dataset's `Arc`, so it
  stays valid even after every `PurrdfDataset` handle is freed. Pattern rows
  are pulled lazily from the selected core index rather than collected when
  the cursor opens.
- A term view from `purrdf_rowcursor_term` borrows into the current row's owned
  value; valid until the next `purrdf_rowcursor_next` or `purrdf_rowcursor_free`.
- A buffer's bytes (`purrdf_buffer_data`) are valid until `purrdf_buffer_free`.
- An error message (`purrdf_error_message`) is valid until `purrdf_error_free`.

## Thread-safety (per handle)

| Handle | Safety |
|--------|--------|
| `PurrdfDataset` | `Send + Sync` — frozen; may be read concurrently from many threads |
| `PurrdfJsonLdContext` | `Send + Sync` — immutable compiled context; may be reused concurrently |
| `PurrdfGraph` | single-threaded mutable (COW delta); external locking required to share |
| `PurrdfCursor` / `PurrdfRowCursor` | single-threaded |
| `PurrdfBuffer` / `PurrdfError` | immutable once returned; read from any thread, free once |

## Term crossing

Three representations are offered (per-row N-Triples reparse is **not** the only
path): structured borrowed term views (`PurrdfTermView`), a cursor-scoped opaque
`term_id` for re-addressing a term (notably a quoted triple, whose components do
not fit a flat view), and the `purrdf_term_to_ntriples` convenience function.

## Known limitation

The GTS **star layer** round-trip (`purrdf_to_gts` → `purrdf_from_gts` of a
dataset containing quoted triples / reifier bindings) currently fails with
`PURRDF_STATUS_GTS_ERROR` (`gts-missing-reifier-binding`). This is a pre-existing
gap in the kernel `to_gts` → `read_graph` → `import_gts_graph` path (reifier
binding rows are dropped on read-back), not in the C-ABI, which calls the
canonical kernel path. Star-free GTS round-trips are lossless. A characterization
test pins the current behavior so a kernel fix will flip it.

## License

MIT OR Apache-2.0 (the semantic layer). The permissive `purrdf-gts` I/O core it
statically reuses remains independently usable under Apache/MIT.

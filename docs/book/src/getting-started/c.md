<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Getting Started: C

`libpurrdf` is a stable, SemVer-disciplined `extern "C"` surface over the
native PurRDF stack: parse, serialize, pattern iteration, copy-on-write
mutation, SPARQL, SHACL validation/entailment, and GTS container round-trips.
The committed, reproducible header
[`include/purrdf.h`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/include/purrdf.h)
**is the ABI contract** — CI fails if it drifts from the crate.

It is one shared library: `libpurrdf` statically reuses the `purrdf-gts` Rust
crate, so a language shim links `libpurrdf` alone and still reads/writes
`.gts` containers.

## Building

The library, header, and pkg-config file are produced by
[`cargo-c`](https://github.com/lu-zero/cargo-c):

```sh
make capi-build                 # cargo capi build: libpurrdf.{so,a} + purrdf.h + purrdf.pc
make capi-install PREFIX=/usr   # cargo capi install into a prefix
make capi-check                 # verify the committed header is current + run the C smoke
```

## A first program

Adapted from the repository's C smoke test
([`crates/rdf-capi/tests/smoke.c`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/tests/smoke.c)):

```c
#include "purrdf.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *doc = "<http://a> <http://b> <http://c> .";
    PurrdfDataset *dataset = NULL;
    PurrdfError *error = NULL;

    int rc = purrdf_parse((const uint8_t *)doc, strlen(doc), "text/turtle",
                          NULL, NULL, &dataset, &error);
    if (rc != PURRDF_STATUS_OK) return 1;

    size_t quad_count = 0;
    purrdf_dataset_quad_count(dataset, &quad_count);
    printf("%zu quad(s)\n", quad_count);

    /* Iterate every quad through a pattern cursor. */
    PurrdfGraphMatch any;
    memset(&any, 0, sizeof(any));
    any.kind = PURRDF_GRAPH_MATCH_KIND_ANY;
    PurrdfCursor *cursor = NULL;
    purrdf_quads_for_pattern(dataset, NULL, NULL, NULL, &any, &cursor, &error);

    PurrdfTermView s, p, o, g;
    uint8_t has_graph = 0;
    while (purrdf_cursor_next(cursor, &s, &p, &o, &g, &has_graph) == PURRDF_STATUS_OK) {
        printf("subject=%.*s\n", (int)s.lexical.len, (const char *)s.lexical.ptr);
    }
    purrdf_cursor_free(cursor);
    purrdf_dataset_free(dataset);
    return 0;
}
```

## The ABI contract

- **No unwinding across the boundary.** Every function runs inside
  `catch_unwind`; a caught panic becomes `PURRDF_STATUS_PANIC`, never a
  process abort across FFI.
- **`int32_t` status + out-params.** Fallible functions return a
  `PurrdfStatus` value and write results through out-pointers.
  `PURRDF_STATUS_CURSOR_EXHAUSTED` is the (non-error) end-of-rows signal.
- **SemVer-frozen ABI.** The status enum is append-only; new fields and
  functions are additive. `purrdf_abi_version` reports the current ABI
  version (0.1.x, beta).

## Ownership and lifetimes

- Every handle/buffer/error/cursor has **exactly one matching `*_free`**
  (`purrdf_dataset_free`, `purrdf_graph_free`, `purrdf_cursor_free`,
  `purrdf_rowcursor_free`, `purrdf_buffer_free`, `purrdf_error_free`).
  Freeing `NULL` is a no-op.
- **The C side never `free()`s a `PurrdfStr.ptr`** — it borrows library-owned
  memory; copy the bytes out if they must outlive the borrow. Term views from
  `purrdf_cursor_next` are valid until the next `purrdf_cursor_next` on that
  cursor or `purrdf_cursor_free`.
- `PurrdfDataset` is frozen and `Send + Sync` — readable concurrently from
  many threads. `PurrdfGraph` (the copy-on-write mutable delta) and cursors
  are single-threaded.

## Known limitation

The GTS **star layer** round-trip (`purrdf_to_gts` → `purrdf_from_gts` of a
dataset containing quoted triples / reifier bindings) currently fails with
`PURRDF_STATUS_GTS_ERROR`. This is a pre-existing gap in the kernel path, not
in the C ABI; star-free GTS round-trips are lossless, and a characterization
test pins the current behavior so a kernel fix will flip it.

Full contract details — status codes, term crossing representations,
thread-safety per handle — are in the
[`purrdf-capi` README](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/README.md).

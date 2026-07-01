<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-events

`purrdf-events` is the RDF 1.2 event protocol shared by the RDF IR engine and
the GTS container. It is intentionally dependency free and owns its own event
term IDs rather than leaking either side's concrete identity model.

## Contract

- `RdfEventSink` is the fallible ingestion direction.
- `RdfEventSource` drives an event stream into a sink.
- Forward references are allowed and resolved at `finish`.
- Blank labels are scoped by `ScopeId`; `EventTermId` is drive-global.
- Cancellation must not freeze partial state.
- Nested RDF 1.2 triple terms are depth-bounded.

## Checks

```bash
make rust-docs
```

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-events` — Zero-Dependency RDF 1.2 Event Protocol

[![crates.io](https://img.shields.io/crates/v/purrdf-events.svg)](https://crates.io/crates/purrdf-events)
[![docs.rs](https://docs.rs/purrdf-events/badge.svg)](https://docs.rs/purrdf-events)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-events` is the streaming RDF 1.2 ingestion protocol of the PurRDF
toolkit: the neutral event seam an RDF *source* (a parser, a GTS reader, a
frozen-dataset replayer) uses to push a dataset into a *sink* (an IR builder,
a serializer) without either side knowing the other's concrete types.

The crate has **zero dependencies** on purpose — it is the contract that both
the IR engine (`purrdf-core`) and the GTS container (`purrdf-gts`) depend *on*.
Its value types (`EventTerm`, `EventQuad`, `EventTriple`) are self-contained:
the protocol carries its own `EventTermId` term ids, never a store's
dataset-local identity.

## The contract

- **`RdfEventSink`** is the fallible ingestion direction; **`RdfEventSource`**
  drives an event stream into a sink.
- **Forward references are allowed** — a quad may reference an `EventTermId`
  declared later; references resolve at `finish`, never eagerly.
- **Ids are drive-global; scopes namespace blank labels only.** An
  `EventTermId` may be declared at most once per drive; a `ScopeId` scopes
  blank-node *label* identity, mirroring per-segment blank scope in GTS.
- **Unresolved at `finish` is a hard error** — never a silent drop.
- **Cancellation must not freeze partial state** — a sink can break the drive
  from any event, and its partial state is discarded, not frozen.
- **Nested RDF 1.2 triple terms are depth-bounded** (`MAX_TERM_NESTING_DEPTH`),
  so a hostile input cannot recurse a sink without bound.
- **Ill-typed literals are carried through**, flagged downstream if at all —
  the protocol never rejects them at ingestion.

## Usage

```sh
cargo add purrdf-events
```

```rust
use core::ops::ControlFlow;
use purrdf_events::{
    EventError, EventQuad, EventTerm, EventTermId, EventTriple, RdfEventSink, ScopeId,
};

/// A sink that just counts the quads a source pushes into it.
struct QuadCounter(usize);

impl RdfEventSink for QuadCounter {
    fn term(&mut self, _id: EventTermId, _term: EventTerm<'_>)
        -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn quad(&mut self, _q: EventQuad) -> Result<ControlFlow<()>, EventError> {
        self.0 += 1;
        Ok(ControlFlow::Continue(()))
    }
    fn reifier(&mut self, _reifier: EventTermId, _triple: EventTriple)
        -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn annotation(&mut self, _reifier: EventTermId, _p: EventTermId, _o: EventTermId)
        -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn open_scope(&mut self) -> Result<ScopeId, EventError> {
        Ok(ScopeId::DEFAULT)
    }
    fn close_scope(&mut self, _scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
    fn finish(&mut self) -> Result<(), EventError> {
        Ok(())
    }
}
```

Any parser or store that speaks these two traits can feed any other — that is
the whole point of the seam.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::events`; depend on `purrdf-events` directly only when implementing
a source or sink.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.

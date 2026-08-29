// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The input source: reading a path (or stdin) into a queryable/serializable view.
//!
//! ## Dispatching over a non-object-safe `DatasetView`
//!
//! [`DatasetView`] uses return-position `impl Trait` in
//! its methods, so it is **not** object-safe: there is no `&dyn DatasetView`.
//! The pipeline therefore cannot erase the concrete view type behind a trait
//! object. Instead it dispatches by input KIND and runs a **generic operation**
//! monomorphized per arm:
//!
//! * a text/graph source is parsed into a concrete `RdfDataset` and the operation
//!   runs over `&RdfDataset`;
//! * a pack source is opened as a `PackView` and the operation runs over
//!   `&PackView` — zero materialization;
//! * a GTS source is folded into a `GtsBundle` by the authoritative importer and the
//!   operation runs over its `&RdfDataset`.
//!
//! Because a Rust closure cannot itself be generic, the operation is expressed as
//! a [`ViewOp`] trait whose single `run` method is generic over the view type;
//! [`run_over_input`] calls `op.run(&view)` in each arm so the compiler emits one
//! monomorphization per concrete view.
//!
//! ## Pack sources and immutable acquisition
//!
//! A pack source is acquired through [`ImmutableInput`](crate::immutable::ImmutableInput),
//! which yields bytes guaranteed **stable and un-truncatable** for the lifetime of
//! the owner: a disk pack is memory-mapped only when the mapping cannot be faulted
//! by a hostile concurrent pathname writer (a verified kernel seal, or our own
//! sealed `memfd` snapshot), and otherwise — and always for a **stdin** pack —
//! read into an owned buffer. The verified bytes are then handed to
//! [`PackView::from_bytes`] zero-copy, and the owner is held alive for the whole
//! operation, so the `PackView` reads it with no materialization.
//!
//! Integrity is verified **once**, unconditionally, via [`verify_pack`] over the
//! already-immutable bytes — *after* acquisition, closing the
//! time-of-check/time-of-use gap the previous "map then verify a mutable file" seam
//! left open. There is no longer any raw, memory-unsafe `mmap` of an unverified,
//! mutable file: the safety is enforced by [`ImmutableInput`](crate::immutable), not
//! promised by a contract on the caller.
//!
//! ## GTS sources and the envelope
//!
//! A GTS file is folded by [`import_gts_events`] — the AUTHORITATIVE importer, which
//! drives the reader in per-segment mode so **per-segment blank-node scope survives**
//! (C2.a) and which hard-fails on any reader diagnostic or dangling term reference.
//! It is deliberately not `import_gts_graph`, whose own documentation records that it
//! flattens those scopes. Because scope is preserved, the `bnode-scope-flatten` contract
//! entry of `purrdf_core::gts_to_rdf_loss_ledger` describes a loss that **did not
//! happen** on this path and is never attached here.
//!
//! What a GTS read genuinely does drop is the ENVELOPE: the segment ledger, sidecar
//! resources, scoped metadata, blob references, suppression directives, opaque nodes and
//! signature records that travel beside the hot graph and have no home in an RDF dataset.
//! [`gts_envelope_ledger`] records exactly those, with counts read off the bundle the
//! importer actually returned — never a count this module assumed.
//!
//! ## Transport encoding
//!
//! Every byte-oriented read goes through [`read_bytes`], which sniffs gzip / zstd
//! through `purrdf_rdf::detect_transport` (the workspace's one transport authority, in
//! `purrdf-gts`) and decodes with `read_to_end`, so a truncated or corrupt stream is an
//! ERROR and not a short read. [`crate::format::resolve`] strips the matching filename
//! suffix before inferring a format; the two halves share the one suffix table rather
//! than each keeping their own.
//!
//! A pack is the one input that does NOT take this path: it is acquired as immutable
//! bytes and verified in place, which a decode step would defeat. A transport-wrapped
//! pack is therefore refused by name in [`acquire_pack_input`] rather than handed to the
//! verifier as garbage.

use std::borrow::Cow;
use std::io::Read;
use std::sync::Arc;

use purrdf_core::{
    DatasetView, LossEntry, LossLedger, PackView, RdfDataset, RdfLookaside, dataset_from_view,
    verify_pack,
};
use purrdf_rdf::{
    NativeRdfFormat, SerializeOutcome, SourceFormat, TransportEncoding, decode_transport,
    detect_transport, import_gts_events, parse_dataset, serialize_dataset_to_format,
};

use crate::error::CliError;
use crate::immutable::ImmutableInput;

/// What to do about a possible gzip / zstd transport wrapper around an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TransportPolicy {
    /// Sniff the leading bytes, then the filename suffix, and decode what is found.
    /// The default, and what every subcommand without an explicit override uses.
    #[default]
    Detect,
    /// Read the bytes verbatim — do not decode even a stream that sniffs as wrapped.
    Verbatim,
    /// Decode under exactly this encoding; a stream that is not it hard-fails.
    Forced(TransportEncoding),
}

/// A generic operation to run over whichever concrete [`DatasetView`] the input
/// resolves to.
///
/// The one method is generic over the view type (`D`), which is exactly why this
/// is a trait rather than a closure: it lets [`run_over_input`] hand the operation
/// either a `&RdfDataset` or a `&PackView` and have the compiler monomorphize
/// `run` for each, sidestepping `DatasetView`'s lack of object safety.
pub(crate) trait ViewOp {
    /// What the operation produces on success.
    type Output;

    /// Run the operation over a borrowed concrete view.
    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<Self::Output, CliError>;
}

/// Read every byte of a path, or of stdin when `path` is `-`, WITHOUT decoding.
fn read_raw_bytes(path: &str) -> Result<Vec<u8>, CliError> {
    if path == "-" {
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer)?;
        Ok(buffer)
    } else {
        Ok(std::fs::read(path)?)
    }
}

/// The filename a transport sniff may consult, or `None` for stdin (which has none —
/// stdin is decided by its leading bytes alone).
fn transport_name(path: &str) -> Option<&str> {
    (path != "-").then_some(path)
}

/// Read every byte of a path (or stdin when `path` is `-`), decoding a detected gzip /
/// zstd transport wrapper.
///
/// Decoding is all-or-nothing: the decoder is drained with `read_to_end`, so a
/// truncated or corrupt stream returns an error rather than the prefix it inflated.
pub(crate) fn read_bytes(path: &str) -> Result<Vec<u8>, CliError> {
    read_bytes_with_transport(path, TransportPolicy::Detect)
}

/// [`read_bytes`] under an explicit [`TransportPolicy`].
pub(crate) fn read_bytes_with_transport(
    path: &str,
    policy: TransportPolicy,
) -> Result<Vec<u8>, CliError> {
    let raw = read_raw_bytes(path)?;
    let encoding = match policy {
        TransportPolicy::Verbatim => None,
        TransportPolicy::Detect => detect_transport(&raw, transport_name(path)),
        TransportPolicy::Forced(encoding) => Some(encoding),
    };
    match encoding {
        None => Ok(raw),
        Some(encoding) => decode_transport(&raw, encoding)
            .map_err(|error| CliError::Runtime(format!("{}: {error}", display_path(path)))),
    }
}

/// How a source names itself in a diagnostic: `-` reads as `<stdin>`.
fn display_path(path: &str) -> &str {
    if path == "-" { "<stdin>" } else { path }
}

/// Acquire a pack `path` (or stdin when `path` is `-`) as an immutable byte owner,
/// **without** verifying it.
///
/// The bytes are obtained through [`ImmutableInput`] — memory-safe against a hostile
/// concurrent pathname writer. Almost every consumer wants [`verified_pack_input`],
/// which additionally runs [`verify_pack`]; the standalone `pack verify` verb uses
/// this raw acquisition so it can surface the [`PackDigest`](purrdf_core::PackDigest)
/// that `verify_pack` returns rather than discard it.
///
/// A transport-wrapped pack is refused here by name. The pack lane's whole point is
/// that the bytes are acquired immutably and verified IN PLACE; decoding them into a
/// fresh buffer would discard exactly the property the lane exists for, so a gzip/zstd
/// pack is a request this pipeline declines rather than one it silently re-routes.
pub(crate) fn acquire_pack_input(path: &str) -> Result<ImmutableInput, CliError> {
    let input = if path == "-" {
        ImmutableInput::from_stdin()?
    } else {
        ImmutableInput::from_disk_path(path)?
    };
    if let Some(encoding) = detect_transport(input.as_bytes(), transport_name(path)) {
        return Err(CliError::Usage(format!(
            "{}: the pack container cannot be read through a {encoding} transport wrapper. A \
             pack is acquired as immutable bytes and verified in place; decoding it into a \
             fresh buffer would discard that guarantee. Decompress it first",
            display_path(path)
        )));
    }
    Ok(input)
}

/// Acquire a pack `path` (or stdin) as an immutable owner and verify it **once**.
///
/// [`verify_pack`] (fail-closed, canonical integrity) runs over the already-immutable
/// bytes, so nothing enters the pipeline unverified. The returned owner must be held
/// alive for as long as its bytes are used: a [`PackView`] borrows them zero-copy, so
/// callers keep the [`ImmutableInput`] in scope for the whole operation. This is the
/// single acquisition seam the pipeline arms below and `convert`'s pack→pack byte
/// passthrough route through.
pub(crate) fn verified_pack_input(path: &str) -> Result<ImmutableInput, CliError> {
    let input = acquire_pack_input(path)?;
    // Unconditional canonical integrity, once, over the stable bytes.
    verify_pack(input.as_bytes())?;
    Ok(input)
}

/// Fold a GTS source into its dataset AND the ledger of what its envelope dropped.
///
/// This is the one GTS ingestion seam in the CLI. It calls [`import_gts_events`], the
/// authoritative importer that preserves per-segment blank-node scope and hard-fails on
/// a reader diagnostic or a dangling term reference — never `import_gts_graph`, which
/// flattens those scopes.
pub(crate) fn load_gts(
    path: &str,
    policy: TransportPolicy,
) -> Result<(Arc<RdfDataset>, LossLedger), CliError> {
    let bytes = read_bytes_with_transport(path, policy)?;
    let bundle = import_gts_events(&bytes)
        .map_err(|diagnostic| CliError::Runtime(format!("{}: {diagnostic}", display_path(path))))?;
    let ledger = gts_envelope_ledger(&bundle.envelope.lookaside, display_path(path));
    Ok((bundle.dataset, ledger))
}

/// The GTS loss-ledger code family: what a GTS envelope carried and an RDF dataset
/// cannot. Each is recorded ONLY when the imported bundle actually carried that
/// material, with the count read off the bundle.
const GTS_LOSS_CODES: [(&str, &str); 7] = [
    (
        "gts-segment-ledger-dropped",
        "segment record(s) (head id, declared profile, streamable-layout claim) are not \
         representable in an RDF dataset. The segments' BLANK-NODE SCOPES ARE PRESERVED — the \
         authoritative event importer reads the file per segment — so this is the loss of the \
         segment LEDGER, not of segment identity in the graph",
    ),
    (
        "gts-sidecar-resources-dropped",
        "typed sidecar resource declaration(s) (SHACL/ShEx/docs/logic/schema/query companions) \
         travel beside the hot graph in the GTS envelope and have no place in an RDF dataset",
    ),
    (
        "gts-metadata-dropped",
        "scoped key/value metadata entr(y|ies) from the GTS envelope are not triples and are \
         not carried into the RDF dataset",
    ),
    (
        "gts-blob-references-dropped",
        "content-addressed blob reference(s) (digest plus origin segments, never the payload \
         bytes) are envelope material with no RDF dataset representation",
    ),
    (
        "gts-suppressions-dropped",
        "`suppress` directive(s) (GTS section 11) are an overlay on the transport, not \
         statements, and do not survive into an RDF dataset",
    ),
    (
        "gts-opaque-nodes-dropped",
        "frame(s) the reader preserved as opaque nodes rather than decoded content are \
         envelope material and are not carried into the RDF dataset",
    ),
    (
        "gts-signature-records-dropped",
        "COSE frame signature record(s) certify the TRANSPORT rather than any triple, so they \
         do not survive into an RDF dataset",
    ),
];

/// The ledger of what a GTS envelope carried and the RDF dataset could not.
///
/// Every entry's count comes from the [`RdfLookaside`] the importer returned, and an
/// empty table records nothing — an envelope that carried no blobs must not claim a blob
/// loss. The segment entry additionally carries the segment head ids verbatim, in
/// segment order, so an operator can trace a converted document back to the exact
/// segments it came from: the provenance the envelope held, surfaced at the moment it
/// stops travelling with the graph.
fn gts_envelope_ledger(lookaside: &RdfLookaside, source: &str) -> LossLedger {
    let counts = [
        lookaside.segments.len(),
        lookaside.resources.len(),
        lookaside.metadata.len(),
        lookaside.blobs.len(),
        lookaside.suppressions.len(),
        lookaside.opaque_nodes.len(),
        lookaside.signatures.len(),
    ];

    let mut ledger = LossLedger::new();
    for ((code, note), count) in GTS_LOSS_CODES.into_iter().zip(counts) {
        if count == 0 {
            continue;
        }
        let mut text = format!("{source}: {count} {note}.");
        if code == GTS_LOSS_CODES[0].0 {
            let heads: Vec<&str> = lookaside
                .segments
                .iter()
                .filter_map(|segment| segment.head.as_deref())
                .collect();
            if !heads.is_empty() {
                text.push_str(" Segment head id(s), in segment order: ");
                text.push_str(&heads.join(", "));
                text.push('.');
            }
        }
        ledger.record(LossEntry {
            code: Cow::Borrowed(code),
            from: Cow::Borrowed("gts"),
            to: Cow::Borrowed("rdf-1.2-dataset"),
            note: Cow::Owned(text),
            location: None,
        });
    }
    ledger
}

/// Open `path` as the concrete view its `format` implies and run `op` over it.
///
/// The text arm parses into an `RdfDataset`; the pack arm acquires a verified,
/// immutable byte owner ([`verified_pack_input`]) and opens a zero-copy `PackView`
/// over it. The owner is held alive for the whole `op.run` call. The GTS arm folds the
/// file through [`load_gts`] and runs over the resulting dataset.
///
/// The GTS envelope ledger is DISCARDED here, because this entry point answers a
/// question (a query, a description, an entailment) rather than converting a document,
/// and none of those verbs surfaces a conversion ledger for their input. `convert`,
/// which does, routes a GTS source through [`load_dataset_reporting`] instead.
pub(crate) fn run_over_input<Op: ViewOp>(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
    op: Op,
) -> Result<Op::Output, CliError> {
    run_over_input_with_transport(path, format, base, TransportPolicy::Detect, op)
}

/// [`run_over_input`] under an explicit [`TransportPolicy`].
///
/// `convert` is the one subcommand that offers `--transport`, and it reaches this
/// variant so an explicitly-named encoding is HONOURED on the zero-copy lane rather than
/// quietly overridden by the sniff — a flag accepted and ignored being the shape this
/// pipeline refuses everywhere else.
pub(crate) fn run_over_input_with_transport<Op: ViewOp>(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
    policy: TransportPolicy,
    op: Op,
) -> Result<Op::Output, CliError> {
    match format {
        SourceFormat::Native(rdf_format) => {
            let bytes = read_bytes_with_transport(path, policy)?;
            let dataset = parse_dataset(&bytes, rdf_format.media_type(), base)?;
            op.run(&*dataset)
        }
        SourceFormat::Pack => {
            let input = verified_pack_input(path)?;
            let view = PackView::from_bytes(input.as_bytes())?;
            op.run(&view)
        }
        SourceFormat::Gts => {
            let (dataset, _envelope_ledger) = load_gts(path, policy)?;
            op.run(&*dataset)
        }
    }
}

/// Open `path` and reconstruct a concrete `Arc<RdfDataset>`, whatever its kind.
///
/// The text arm parses directly; the pack arm opens a verified zero-copy `PackView`
/// (over an immutable byte owner) and reconstructs a concrete dataset via
/// [`dataset_from_view`]; the GTS arm folds the file through the authoritative event
/// importer. This is the entry point for steps that genuinely need an owned MUTABLE
/// dataset (e.g. SPARQL UPDATE), which cannot run over an immutable view; read-only
/// reasoning enters the reasoner over the `PackView` directly through
/// [`run_over_input`] instead.
///
/// A GTS envelope ledger is discarded here for the reason [`run_over_input`] discards
/// one; [`load_dataset_reporting`] is the variant that carries it.
pub(crate) fn load_dataset(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
) -> Result<Arc<RdfDataset>, CliError> {
    load_dataset_reporting(path, format, base, TransportPolicy::Detect).map(|(dataset, _)| dataset)
}

/// [`load_dataset`] plus the ledger of what reading this source dropped.
///
/// The ledger is empty for every source but GTS, whose envelope is the only input-side
/// material this pipeline loses on the way in (see [`gts_envelope_ledger`]).
pub(crate) fn load_dataset_reporting(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
    policy: TransportPolicy,
) -> Result<(Arc<RdfDataset>, LossLedger), CliError> {
    match format {
        SourceFormat::Native(rdf_format) => {
            let bytes = read_bytes_with_transport(path, policy)?;
            Ok((
                parse_dataset(&bytes, rdf_format.media_type(), base)?,
                LossLedger::new(),
            ))
        }
        SourceFormat::Pack => {
            let input = verified_pack_input(path)?;
            let view = PackView::from_bytes(input.as_bytes())?;
            Ok((dataset_from_view(&view)?, LossLedger::new()))
        }
        SourceFormat::Gts => load_gts(path, policy),
    }
}

/// Serialize the input at `path` (text, pack, or GTS) to N-Quads over its view.
///
/// A pack is **not** rebuilt into an owned `RdfDataset` to be serialized: the
/// `PackView` is serialized directly. This is the read side of the `entails` and
/// `consistency` string boundaries, which decide over N-Quads text.
pub(crate) fn serialize_input_to_nquads(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
) -> Result<SerializeOutcome, CliError> {
    struct NQuadsOp;

    impl ViewOp for NQuadsOp {
        type Output = SerializeOutcome;

        fn run<D: DatasetView + Sync>(self, view: &D) -> Result<Self::Output, CliError> {
            Ok(serialize_dataset_to_format(
                view,
                NativeRdfFormat::NQuads,
                None,
            )?)
        }
    }

    run_over_input(path, format, base, NQuadsOp)
}

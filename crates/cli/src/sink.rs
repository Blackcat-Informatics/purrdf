// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The unified output sink: serialize a view to a target format and write it.
//!
//! [`write_rdf`] is the one place the pipeline emits a dataset. It handles both
//! target kinds:
//!
//! * an RDF syntax → [`serialize_dataset_to_format`] over the borrowed view (a
//!   `PackView` serializes with zero materialization), then write the bytes;
//! * the pack container → reconstruct a concrete dataset via
//!   [`dataset_from_view`] and build the pack bytes with [`PackBuilder`].
//!
//! It returns the [`LossLedger`] for the conversion so `main` can surface it under
//! `--loss-ledger`. The ledger combines the **contract** losses for the
//! `(source-codec → target-codec)` pair ([`pair_loss_ledger`], when both codec
//! names are known) with the **realized** counts of what the serializer actually
//! dropped ([`RealizedDrops`] — statement-layer rows, base-direction literals, and
//! named-graph rows), each recorded as a runtime entry only when non-zero.
//!
//! The two halves are not interchangeable, and the realized half is the one that
//! always reports: the contract half needs BOTH codec names, so a lane that
//! serializes a freshly-built dataset (a CONSTRUCT result, a reasoned closure) has
//! no source codec and gets no contract entries at all. Every loss that lane can
//! realize must therefore be counted, or it is silent.

use std::borrow::Cow;
use std::io::Write;

use purrdf_core::{
    DatasetView, LossEntry, LossLedger, PackBuilder, dataset_from_view, pair_loss_ledger,
};
use purrdf_rdf::{
    JsonLdSerializeOptions, NativeRdfFormat, SerializeGraph, SerializeOptions, SourceFormat,
    StatementLayer, serialize_dataset_to_writer_with,
};

use crate::error::CliError;
use crate::format;

/// The runtime loss code recording how many RDF-1.2 statement-layer rows the
/// serializer dropped because the target format does not carry the star layer.
const STATEMENT_ROWS_DROPPED_CODE: &str = "statement-rows-dropped";

/// The runtime loss code recording how many base-direction object literals the
/// serializer dropped because the target format (TriX / HexTuples) has no
/// direction surface — it keeps the language tag but cannot carry `--ltr` / `--rtl`.
const DIRECTION_DROPPED_CODE: &str = "rdf12-direction-dropped";

/// The runtime loss code recording how many rows the serializer dropped because
/// the target format is a single-graph syntax with no named-graph construct.
///
/// The REALIZED twin of `purrdf_core::loss`'s static `named-graph-dropped` contract
/// entry, and the reason it exists: the contract half is only reachable when both
/// codec names are known (`src_codec` is `Some`), so before this counter a lane with
/// no source codec — a freshly-CONSTRUCTed graph, a reasoned closure — could discard
/// every named graph it held and hand back an EMPTY ledger. A realized count needs no
/// source codec, so the drop is now reported on every lane that can produce it.
const NAMED_GRAPH_ROWS_DROPPED_CODE: &str = "named-graph-rows-dropped";

/// Write `bytes` to `out`, or to stdout when `out` is `-`.
///
/// A downstream consumer that closes its end of the pipe early (the ubiquitous
/// `purrdf … | head` / `| grep -q` idiom) makes the stdout write fail with
/// [`std::io::ErrorKind::BrokenPipe`]. Standard Unix filters exit 0 silently on a
/// downstream EPIPE, so that one error kind is treated as a clean success here; every
/// other error (including on a file target) still propagates.
pub(crate) fn write_out(out: &str, bytes: &[u8]) -> Result<(), CliError> {
    let mut target = OutTarget::open(out)?;
    match target.write_all(bytes) {
        Ok(()) => target.finish(),
        Err(error) => match target.abandon(error.into()) {
            CliError::DownstreamClosed => Ok(()),
            other => Err(other),
        },
    }
}

/// Where a command's bytes go, as a writer a serializer can stream into.
///
/// The whole-buffer spelling above is this type with one `write_all`; the streaming
/// one is the same type fed incrementally. Both therefore share one policy for the
/// two things that differ between a pipe and a file — a downstream reader closing
/// early, and what a failure leaves behind.
pub(crate) enum OutTarget {
    /// Standard output. `-` on the command line.
    ///
    /// `saw_hangup` records a downstream reader closing the pipe. The write still
    /// FAILS when that happens, so the serializer stops within a row instead of
    /// formatting the rest of a document nobody is reading — but the flag lets the
    /// command exit 0, which is the filter contract. Accepting-and-discarding later
    /// chunks would also exit 0 and would run `purrdf convert huge.nq | head -1` to
    /// completion, which is the behaviour this avoids.
    Stdout {
        stdout: std::io::Stdout,
        saw_hangup: bool,
    },
    /// A file, created-or-truncated exactly as `fs::write` did.
    File {
        file: std::fs::File,
        path: std::path::PathBuf,
        /// Whether a failure should unlink the target. Set only when the path was
        /// verified a regular file that this open truncated — see `abandon`.
        unlink_on_abandon: bool,
    },
}

impl OutTarget {
    /// Open `out` for writing (`-` is stdout).
    pub(crate) fn open(out: &str) -> Result<Self, CliError> {
        if out == "-" {
            return Ok(Self::Stdout {
                stdout: std::io::stdout(),
                saw_hangup: false,
            });
        }
        let path = std::path::PathBuf::from(out);
        // `File::create` is what `fs::write` does: create or truncate, follow
        // symlinks, keep existing permissions. The starting semantics are unchanged.
        let file = std::fs::File::create(&path)?;
        // Only a REGULAR file may be unlinked on failure. A FIFO, a device node or
        // `/dev/stdout` would otherwise be removed by an error path, and a symlink
        // would lose the LINK while its target kept the half-written bytes.
        //
        // This asks the PATH, not the descriptor, and the difference is the whole
        // guard. `File::create` above followed the link, so `file.metadata()` is an
        // `fstat` describing whatever the link RESOLVED TO — for a symlink pointing
        // at an ordinary file that reports a regular file, the guard opens, and
        // `remove_file` on the path then unlinks the LINK and leaves the
        // half-written target in place. Precisely the loss the comment above
        // describes, performed by the code meant to prevent it.
        //
        // `symlink_metadata` is an `lstat`: it describes the path itself. A symlink
        // reports `is_symlink()`, so `is_file()` is false and the guard stays shut —
        // as it already did for a FIFO or a device node, which is why only the
        // symlink leg was wrong.
        let unlink_on_abandon =
            std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_file());
        Ok(Self::File {
            file,
            path,
            unlink_on_abandon,
        })
    }

    /// The writer a serializer streams into.
    pub(crate) fn writer(&mut self) -> &mut dyn Write {
        self
    }

    /// Whether a downstream reader closed the pipe during this write.
    const fn hung_up(&self) -> bool {
        matches!(
            self,
            Self::Stdout {
                saw_hangup: true,
                ..
            }
        )
    }

    /// Flush and close.
    ///
    /// A downstream consumer that closes its end early — the ubiquitous
    /// `purrdf … | head` idiom — makes the stdout write fail with `BrokenPipe`.
    /// Standard Unix filters exit 0 silently on a downstream EPIPE, so that ONE
    /// error kind on stdout is a clean success; every other kind, and every kind on
    /// a file target, still propagates.
    pub(crate) fn finish(mut self) -> Result<(), CliError> {
        if self.hung_up() {
            return Ok(());
        }
        match self.flush() {
            Ok(()) => Ok(()),
            Err(error)
                if matches!(self, Self::Stdout { .. })
                    && error.kind() == std::io::ErrorKind::BrokenPipe =>
            {
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Abandon after a failure, returning the error to report.
    ///
    /// A file target is truncated when the write begins, so a mid-document failure
    /// leaves a SHORT file — and a short N-Triples document still parses, silently
    /// missing rows, which is the worst shape a failure can take here. The partial
    /// file is therefore removed, and the message says so, because the caller's
    /// previous contents are already gone either way.
    pub(crate) fn abandon(self, error: CliError) -> CliError {
        // A downstream reader that closed early did not fail: the serializer's write
        // error is the MECHANISM that stopped it, not a fault to report.
        if self.hung_up() {
            return CliError::DownstreamClosed;
        }
        if let Self::File {
            path,
            unlink_on_abandon: true,
            ..
        } = self
            && std::fs::remove_file(&path).is_ok()
        {
            return CliError::Runtime(format!(
                "{error}; the partial output file `{}` was removed (its previous \
                 contents were replaced when the write began)",
                path.display()
            ));
        }
        error
    }
}

impl Write for OutTarget {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Stdout { stdout, saw_hangup } => match stdout.write(buf) {
                Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                    *saw_hangup = true;
                    Err(error)
                }
                other => other,
            },
            Self::File { file, .. } => file.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Stdout { stdout, saw_hangup } => match stdout.flush() {
                Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                    *saw_hangup = true;
                    Err(error)
                }
                other => other,
            },
            Self::File { file, .. } => file.flush(),
        }
    }
}

/// Serialize `view` to `target` and write it to `out`, returning the loss ledger.
///
/// `base` is the EGRESS base: `serialize_dataset_to_format` writes it as the output
/// document's base directive and relativizes against it on a syntax whose `emits_base`
/// registry column is set (Turtle, TriG, RDF/XML, JSON-LD, YAML-LD), and discards it on
/// one whose column is clear — the filter lives there, in the serializer, so this sink
/// does not re-decide it. The pack arm reads no base at all: the container stores
/// fully-resolved terms. `crate::format::refuse_unconsumable_base` is what keeps a base
/// that no leg can spend from reaching here in the first place.
///
/// `src_codec` is the source format's loss-ledger codec name when known (`None`
/// for a pack source or a codec-less syntax); it seeds the contract-loss half of
/// the returned ledger.
/// Stream a SPARQL result to `out`, under the same policy [`write_rdf`] applies to a
/// dataset.
///
/// This exists so the results lane and the RDF lane share ONE answer to the two things
/// that differ between a pipe and a file — a downstream reader closing early, and what a
/// failure leaves behind. Before it, `query` built the whole results document and handed
/// the bytes to `write_out`, which meant the sink the results serializers were given had
/// no caller anywhere in the shipped binary: the capability existed and nothing reached
/// it. It also meant `purrdf query … | head -1` formatted every row of an answer nobody
/// was reading.
///
/// The `StreamOutcome` is discarded here for the same reason the eager lane discarded
/// its `SerializeOutcome`: this lane reports an empty ledger, because a tabular or
/// boolean result performs no lossy transcode. `provenance_dropped` is not a transcode
/// loss and is refused up front rather than reported after the fact.
pub(crate) fn write_results(
    out: &str,
    result: &purrdf_sparql_results::SparqlResult,
    format: purrdf_sparql_results::SparqlResultsFormat,
    provenance: &purrdf_sparql_results::ResultProvenance,
    namespace: Option<&purrdf_sparql_results::ProvenanceNamespace>,
) -> Result<(), CliError> {
    let mut target = OutTarget::open(out)?;
    match purrdf_sparql_results::serialize_into(
        result,
        format,
        provenance,
        namespace,
        target.writer(),
    ) {
        Ok(_) => target.finish(),
        Err(error) => match target.abandon(error.into()) {
            // The reader went away; what it asked for is complete enough for it.
            CliError::DownstreamClosed => Ok(()),
            other => Err(other),
        },
    }
}

pub(crate) fn write_rdf<D: DatasetView>(
    view: &D,
    out: &str,
    target: SourceFormat,
    base: Option<&str>,
    src_codec: Option<&str>,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<LossLedger, CliError> {
    validate_jsonld_options(target, jsonld_options)?;
    match target {
        SourceFormat::Native(format) => {
            // Streamed: the document is written as it is produced rather than built
            // whole and handed over. The loss ledger is unaffected — the drop counts
            // come back in the report instead of the outcome, and mean the same.
            let mut target = OutTarget::open(out)?;
            let options = SerializeOptions {
                selection: SerializeGraph::Dataset,
                statement_layer: StatementLayer::PerFormatCapability,
                jsonld_options,
            };
            let report = match serialize_dataset_to_writer_with(
                view,
                format,
                base,
                &options,
                target.writer(),
            ) {
                Ok(report) => report,
                Err(diagnostic) => match target.abandon(diagnostic.into()) {
                    // The reader went away; the document it asked for is complete
                    // enough for it, and the command succeeded.
                    CliError::DownstreamClosed => return Ok(LossLedger::new()),
                    other => return Err(other),
                },
            };
            target.finish()?;
            Ok(build_ledger(
                src_codec,
                format.loss_codec_name(),
                &RealizedDrops {
                    statement_rows: report.statement_rows_dropped,
                    directional_literals: report.directional_literals_dropped,
                    named_graph_rows: report.named_graph_rows_dropped,
                },
            ))
        }
        SourceFormat::Pack => {
            let dataset = dataset_from_view(view)?;
            let bytes = PackBuilder::build_bytes(&dataset)?;
            write_out(out, &bytes)?;
            // A pack is a lossless RDF-1.2 container: no ledger entries.
            Ok(LossLedger::new())
        }
        // GTS is a READ-ONLY target for this pipeline: `format::refuse_gts_target`
        // declines `--to gts` at resolution time, with the reason. This arm keeps the
        // match total and fails closed rather than writing something that is not GTS,
        // for the caller who reaches the sink by some other route.
        SourceFormat::Gts => Err(format::refuse_gts_target(target, "the output target")
            .expect_err("refuse_gts_target rejects SourceFormat::Gts")),
    }
}

/// Reject a JSON-LD options document unless the selected sink is JSON-LD/YAML-LD.
pub(crate) fn validate_jsonld_options(
    target: SourceFormat,
    options: Option<&JsonLdSerializeOptions>,
) -> Result<(), CliError> {
    if options.is_some()
        && !matches!(
            target,
            SourceFormat::Native(NativeRdfFormat::JsonLd | NativeRdfFormat::YamlLd)
        )
    {
        return Err(CliError::Usage(
            "--jsonld-options requires a JSON-LD or YAML-LD RDF output".to_owned(),
        ));
    }
    Ok(())
}

/// What the serializer ACTUALLY discarded on this one emission, by cause.
///
/// The three counts partition the dropped rows — star layer, base direction, graph
/// scoping — so no row is reported twice and none is reported under a cause that did
/// not produce it.
struct RealizedDrops {
    /// RDF-1.2 statement-layer rows dropped because the target has no star layer.
    statement_rows: usize,
    /// Object literals whose base direction the target has no surface for.
    directional_literals: usize,
    /// Rows dropped because the target is a single-graph syntax.
    named_graph_rows: usize,
}

/// Combine the contract losses for `(src_codec → dst_codec)` with the realized
/// dropped-row counts (RDF-1.2 statement-layer rows, base-direction literals, and
/// named-graph rows).
fn build_ledger(
    src_codec: Option<&str>,
    dst_codec: Option<&str>,
    realized: &RealizedDrops,
) -> LossLedger {
    let &RealizedDrops {
        statement_rows: statement_rows_dropped,
        directional_literals: directional_literals_dropped,
        named_graph_rows: named_graph_rows_dropped,
    } = realized;
    let mut ledger = match (src_codec, dst_codec) {
        (Some(from), Some(to)) => pair_loss_ledger(from, to),
        _ => LossLedger::new(),
    };
    if statement_rows_dropped > 0 {
        ledger.record(LossEntry {
            code: Cow::Borrowed(STATEMENT_ROWS_DROPPED_CODE),
            from: Cow::Owned(src_codec.unwrap_or("unknown").to_string()),
            to: Cow::Owned(dst_codec.unwrap_or("unknown").to_string()),
            note: Cow::Owned(format!(
                "{statement_rows_dropped} RDF-1.2 statement-layer row(s) (reifier bindings + \
                 annotation triples) were dropped because the target format does not carry the \
                 star layer"
            )),
            location: None,
        });
    }
    if directional_literals_dropped > 0 {
        ledger.record(LossEntry {
            code: Cow::Borrowed(DIRECTION_DROPPED_CODE),
            from: Cow::Owned(src_codec.unwrap_or("unknown").to_string()),
            to: Cow::Owned(dst_codec.unwrap_or("unknown").to_string()),
            note: Cow::Owned(format!(
                "{directional_literals_dropped} literal base direction(s) were dropped because \
                 the target format (TriX / HexTuples) has no direction surface — the language \
                 tag is retained but `--ltr` / `--rtl` is lost"
            )),
            location: None,
        });
    }
    if named_graph_rows_dropped > 0 {
        ledger.record(LossEntry {
            code: Cow::Borrowed(NAMED_GRAPH_ROWS_DROPPED_CODE),
            from: Cow::Owned(src_codec.unwrap_or("unknown").to_string()),
            to: Cow::Owned(dst_codec.unwrap_or("unknown").to_string()),
            note: Cow::Owned(format!(
                "{named_graph_rows_dropped} row(s) asserted in a named graph (base quads plus \
                 the statement-layer rows scoped to them) were DROPPED because the target \
                 format is a single-graph syntax with no named-graph construct — they are not \
                 folded into the default graph"
            )),
            location: None,
        });
    }
    ledger
}

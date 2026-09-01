// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
    JsonLdSerializeOptions, NativeRdfFormat, SourceFormat, serialize_dataset_to_format,
    serialize_dataset_to_format_with_jsonld_options,
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
    if out == "-" {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        if let Err(error) = handle.write_all(bytes).and_then(|()| handle.flush())
            && error.kind() != std::io::ErrorKind::BrokenPipe
        {
            return Err(error.into());
        }
    } else {
        std::fs::write(out, bytes)?;
    }
    Ok(())
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
            let outcome = if let Some(options) = jsonld_options {
                serialize_dataset_to_format_with_jsonld_options(view, format, base, options)?
            } else {
                serialize_dataset_to_format(view, format, base)?
            };
            write_out(out, &outcome.bytes)?;
            Ok(build_ledger(
                src_codec,
                format.loss_codec_name(),
                &RealizedDrops {
                    statement_rows: outcome.statement_rows_dropped,
                    directional_literals: outcome.directional_literals_dropped,
                    named_graph_rows: outcome.named_graph_rows_dropped,
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

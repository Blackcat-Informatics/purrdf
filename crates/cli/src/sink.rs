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
//! names are known) with the **realized** count of RDF-1.2 statement-layer rows the
//! serializer actually dropped (recorded as a runtime entry only when non-zero).

use std::borrow::Cow;
use std::io::Write;

use purrdf_core::{
    DatasetView, LossEntry, LossLedger, PackBuilder, dataset_from_view, pair_loss_ledger,
};
use purrdf_rdf::serialize_dataset_to_format;

use crate::error::CliError;
use crate::format::CliFormat;

/// The runtime loss code recording how many RDF-1.2 statement-layer rows the
/// serializer dropped because the target format does not carry the star layer.
const STATEMENT_ROWS_DROPPED_CODE: &str = "statement-rows-dropped";

/// Write `bytes` to `out`, or to stdout when `out` is `-`.
pub(crate) fn write_out(out: &str, bytes: &[u8]) -> Result<(), CliError> {
    if out == "-" {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(bytes)?;
        handle.flush()?;
    } else {
        std::fs::write(out, bytes)?;
    }
    Ok(())
}

/// Serialize `view` to `target` and write it to `out`, returning the loss ledger.
///
/// `src_codec` is the source format's loss-ledger codec name when known (`None`
/// for a pack source or a codec-less syntax); it seeds the contract-loss half of
/// the returned ledger.
pub(crate) fn write_rdf<D: DatasetView>(
    view: &D,
    out: &str,
    target: CliFormat,
    base: Option<&str>,
    src_codec: Option<&str>,
) -> Result<LossLedger, CliError> {
    match target {
        CliFormat::Rdf(format) => {
            let outcome = serialize_dataset_to_format(view, format, base)?;
            write_out(out, &outcome.bytes)?;
            Ok(build_ledger(
                src_codec,
                format.loss_codec_name(),
                outcome.statement_rows_dropped,
            ))
        }
        CliFormat::Pack => {
            let dataset = dataset_from_view(view)?;
            let bytes = PackBuilder::build_bytes(&dataset)?;
            write_out(out, &bytes)?;
            // A pack is a lossless RDF-1.2 container: no ledger entries.
            Ok(LossLedger::new())
        }
    }
}

/// Combine the contract losses for `(src_codec → dst_codec)` with the realized
/// dropped-statement-row count.
fn build_ledger(
    src_codec: Option<&str>,
    dst_codec: Option<&str>,
    statement_rows_dropped: usize,
) -> LossLedger {
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
    ledger
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Browser qualification of a fully certified million-row persistent fixture.
//! The preparation script caps this module's entire linear memory at 256 MiB.
//! Source bytes belong to that cap even though the session's separate 64 MiB
//! ledger measures only its cache, metadata, pins, scratch, evidence and sink.

use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::{
    DatasetView, SegmentedBytes, SegmentedError, SegmentedReadLimits, SegmentedReceipt,
    SegmentedReceiptAuthority, SegmentedSession, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use wasm_bindgen::prelude::*;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const STORAGE_CEILING: u64 = 64 << 20;
const QUERY: &str = "SELECT ?object WHERE { <http://example.org/s0042> <http://example.org/link> ?object . <http://example.org/s0042> <http://example.org/other> ?object }";

struct ReceiptPin<'a>(&'a str);

impl SegmentedReceiptAuthority for ReceiptPin<'_> {
    fn authenticate(&self, receipt: &[u8]) -> Result<(), SegmentedError> {
        let found = purrdf_hash::blake3::hash(receipt);
        let trusted = purrdf_hash::hex::decode_32_canonical(self.0)
            .ok_or(SegmentedError::SnapshotMismatch)?;
        if found.as_bytes() != &trusted {
            return Err(SegmentedError::SnapshotMismatch);
        }
        Ok(())
    }
}

fn qualify(source: Vec<u8>, encoded_receipt: &[u8], trusted_pin: &str) -> Result<String, String> {
    let receipt =
        SegmentedReceipt::from_authenticated_bytes(encoded_receipt, &ReceiptPin(trusted_pin))
            .map_err(|error| error.to_string())?;
    let source_bytes = source.len();
    let transfer_window = WholeProcessWindow::open();
    let provider = Arc::new(SegmentedBytes::new(
        Arc::<[u8]>::from(source),
        receipt.snapshot(),
    ));
    let transfer = transfer_window.close();
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(QUERY, None)
        .map_err(|error| error.to_string())?;
    let reader_window = WholeProcessWindow::open();
    let session = SegmentedSession::open(
        provider,
        &receipt,
        SegmentedReadLimits::new(STORAGE_CEILING, 512, 2_000_000, 2_000_000_000, 8),
    )
    .map_err(|error| error.to_string())?;
    let source_rows = session
        .checked_read(|view| {
            view.quads().try_fold(0_u64, |count, _| {
                count.checked_add(1).ok_or("source row count overflow")
            })
        })
        .map_err(|error| error.to_string())??;
    if source_rows != 1_000_000 {
        return Err(format!("expected 1000000 source rows, found {source_rows}"));
    }
    let answer = engine
        .query_prepared_fallible_view(&session, &prepared, &[], QueryOptions::EMPTY)
        .map_err(|error| error.to_string())?;
    let Some((_, rows)) = answer.result.solutions() else {
        return Err("SELECT did not return solutions".to_owned());
    };
    if rows.len() != 1
        || rows[0].len() != 1
        || !matches!(&rows[0][0], Some(TermValue::Iri(iri)) if iri == "http://example.org/o0059")
    {
        return Err("selective join did not return the exact certified fixture answer".to_owned());
    }
    let mut drain = purrdf_core::sink::Measure::default();
    let evidence = session
        .export_trig_lines(&mut drain)
        .map_err(|error| error.to_string())?;
    let measured = reader_window.close();
    if drain.bytes() != 82_999_530 {
        return Err(format!(
            "unexpected complete streamed export size: {}",
            drain.bytes()
        ));
    }
    let counted_peak = u64::try_from(measured.peak_working_bytes)
        .map_err(|_| "negative allocator peak".to_owned())?;
    if counted_peak > evidence.peak_bytes() || evidence.peak_bytes() > STORAGE_CEILING {
        return Err("reader peak escaped the shared storage ledger".to_owned());
    }
    Ok(format!(
        "{{\"schema\":\"purrdf-wasm-storage-qualification-v1\",\"source_bytes\":{source_bytes},\"source_rows\":{source_rows},\"selective_rows\":1,\"export_bytes\":{},\"storage_ceiling_bytes\":{STORAGE_CEILING},\"charged_peak_bytes\":{},\"counted_reader_peak_bytes\":{counted_peak},\"source_transfer_peak_bytes\":{},\"request_count\":{},\"io_bytes\":{},\"evictions\":{}}}",
        drain.bytes(),
        evidence.peak_bytes(),
        transfer.peak_working_bytes,
        evidence.request_count(),
        evidence.io_bytes(),
        evidence.evictions(),
    ))
}

/// Run the pinned persistent workload and return its completed measurement record.
/// The pin must be retained independently from successful full certification.
///
/// # Errors
/// Refuses an unauthenticated receipt, incomplete read/export, wrong answer,
/// resource exhaustion or allocations outside the declared reader ledger.
#[wasm_bindgen]
pub fn qualify_persistent_storage(
    source: Vec<u8>,
    encoded_receipt: &[u8],
    trusted_pin: &str,
) -> Result<String, JsError> {
    qualify(source, encoded_receipt, trusted_pin).map_err(|error| JsError::new(&error))
}

/// Run the existing resident/browser envelope workload roster verbatim.
/// `false` preserves its frozen profile; `true` uses the same workloads with
/// 16,016 groups, verifying at least 100,000 actual RDF rows after interning.
///
/// # Errors
/// Refuses a failed workload or an allocation peak above its profile ceiling.
#[wasm_bindgen]
pub fn qualify_resident_envelope(actual_hundred_thousand: bool) -> Result<String, JsError> {
    use purrdf_envelope_probe::{WORKLOADS, profile, run};
    use std::fmt::Write as _;

    let mut selected = *profile("wasm-browser")
        .ok_or_else(|| JsError::new("wasm-browser qualification profile is absent"))?;
    if actual_hundred_thousand {
        selected.groups = 16_016;
    }
    let profile = &selected;
    let mut record = format!(
        "{{\"profile\":\"wasm-browser\",\"groups\":{},\"workloads\":[",
        profile.groups
    );
    for (index, workload) in WORKLOADS.iter().enumerate() {
        let window = WholeProcessWindow::open();
        let result = run(workload, profile, &window);
        let measured = window.close();
        let metrics = result.map_err(|error| JsError::new(&error))?;
        if actual_hundred_thousand
            && *workload == "roundtrip"
            && !metrics
                .iter()
                .any(|(name, value)| *name == "rdf_rows" && *value >= 100_000)
        {
            return Err(JsError::new(
                "supplementary resident fixture contains fewer than 100000 RDF rows",
            ));
        }
        if *workload == "roundtrip"
            && profile.roundtrip_peak_ceiling_bytes.is_some_and(|ceiling| {
                u64::try_from(measured.peak_working_bytes).map_or(true, |peak| peak > ceiling)
            })
        {
            return Err(JsError::new(
                "resident roundtrip allocation peak exceeded its frozen profile ceiling",
            ));
        }
        if index != 0 {
            record.push(',');
        }
        write!(record,
            "{{\"name\":\"{workload}\",\"peak_working_bytes\":{},\"retained_bytes\":{},\"metrics\":{{",
            measured.peak_working_bytes, measured.retained_bytes,
        ).expect("writing a String is infallible");
        for (metric_index, (name, value)) in metrics.iter().enumerate() {
            if metric_index != 0 {
                record.push(',');
            }
            write!(record, "\"{name}\":{value}").expect("writing a String is infallible");
        }
        record.push_str("}}");
    }
    record.push_str("]}");
    Ok(record)
}

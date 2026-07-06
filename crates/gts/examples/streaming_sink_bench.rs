// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Report-only streaming-sink throughput probe for the GTS writer; not a
//! criterion gate — see `docs/BENCHMARKS.md` for the benchmarking policy.

use std::env;
use std::fs;
use std::fs::File;
use std::process::ExitCode;

use purrdf_gts::model::{
    AnnotationRow, Diagnostic, OpaqueNode, Quad, ReifierRow, Signature, StreamableInfo,
    Suppression, Term,
};
use purrdf_gts::reader::{ReadOptions, StreamingSink, read_to_sink_from_reader};

#[derive(Default)]
struct CountingSink {
    terms: usize,
    quads: usize,
    reifiers: usize,
    annotations: usize,
    suppressions: usize,
    blobs: usize,
    opaque: usize,
    signatures: usize,
    diagnostics: usize,
    segment_heads: usize,
    streamable_layouts: usize,
}

impl StreamingSink for CountingSink {
    fn term(&mut self, _segment_index: usize, _term_id: usize, _term: &Term) {
        self.terms += 1;
    }

    fn quad(&mut self, _segment_index: usize, _quad: Quad) {
        self.quads += 1;
    }

    fn reifier(&mut self, _segment_index: usize, _reifier: ReifierRow) {
        self.reifiers += 1;
    }

    fn annotation(&mut self, _segment_index: usize, _annotation: AnnotationRow) {
        self.annotations += 1;
    }

    fn suppression(&mut self, _segment_index: usize, _suppression: &Suppression) {
        self.suppressions += 1;
    }

    fn blob(
        &mut self,
        _segment_index: usize,
        _digest: &str,
        _meta: Option<&ciborium::value::Value>,
    ) {
        self.blobs += 1;
    }

    fn opaque(&mut self, _segment_index: usize, _opaque: &OpaqueNode) {
        self.opaque += 1;
    }

    fn signature(&mut self, _segment_index: usize, _signature: &Signature) {
        self.signatures += 1;
    }

    fn diagnostic(&mut self, _diagnostic: &Diagnostic) {
        self.diagnostics += 1;
    }

    fn segment_head(&mut self, _segment_index: usize, _head: &[u8]) {
        self.segment_heads += 1;
    }

    fn streamable_layout(&mut self, _segment_index: usize, _info: &StreamableInfo) {
        self.streamable_layouts += 1;
    }
}

fn linux_peak_kib() -> Option<usize> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        let Some(rest) = line.strip_prefix("VmHWM:") else {
            continue;
        };
        return rest.split_whitespace().next()?.parse().ok();
    }
    None
}

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: streaming_sink_bench <file.gts>");
        return ExitCode::from(2);
    };
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("streaming_sink_bench: {err}");
            return ExitCode::from(2);
        }
    };
    let mut sink = CountingSink::default();
    let result = read_to_sink_from_reader(file, ReadOptions::new(true, None), &mut sink);
    let peak = linux_peak_kib().map_or_else(|| "null".to_string(), |value| value.to_string());
    println!(
        concat!(
            "{{",
            "\"items\":null,",
            "\"frames\":null,",
            "\"terms\":{},",
            "\"quads\":{},",
            "\"blobs\":{},",
            "\"reifiers\":{},",
            "\"annotations\":{},",
            "\"suppressions\":{},",
            "\"opaque\":{},",
            "\"signatures\":{},",
            "\"diagnostics\":{},",
            "\"segment_heads\":{},",
            "\"streamable_layouts\":{},",
            "\"peak_kib\":{}",
            "}}"
        ),
        sink.terms,
        sink.quads,
        sink.blobs,
        sink.reifiers,
        sink.annotations,
        sink.suppressions,
        sink.opaque,
        sink.signatures,
        result.diagnostics.len(),
        result.segment_heads.len(),
        result.segment_streamable.len(),
        peak
    );
    ExitCode::SUCCESS
}

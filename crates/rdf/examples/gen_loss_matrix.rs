// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regenerates the committed loss-ledger JSON artifacts: the RDF↔GTS-only matrix
//! (`generated/rdf-loss-matrix.json`, `rdf` mode) and the full enumerable
//! registry — every registered `(from, to)` pair, including the RDF↔GTS
//! directions — (`generated/transcode-loss-matrix.json`, `transcode` mode).
//! Run via `make metadata`; pass `rdf` or `transcode` to select the output.

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "rdf".to_owned());
    match mode.as_str() {
        "rdf" => print!("{}", purrdf_rdf::loss::rdf_gts_loss_matrix_json()),
        "transcode" => print!("{}", purrdf_rdf::loss::loss_matrix_json()),
        other => {
            eprintln!("unknown loss matrix mode `{other}`; expected `rdf` or `transcode`");
            std::process::exit(2);
        }
    }
}

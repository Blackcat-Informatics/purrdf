// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "rdf".to_owned());
    match mode.as_str() {
        "rdf" => print!("{}", purrdf_rdf::loss::loss_matrix_json()),
        "transcode" => print!("{}", purrdf_rdf::loss::transcode_loss_matrix_json()),
        other => {
            eprintln!("unknown loss matrix mode `{other}`; expected `rdf` or `transcode`");
            std::process::exit(2);
        }
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::fmt::Write as _;

pub(crate) const SIZES: [usize; 2] = [100, 1_000];
pub(crate) const SOURCE: &str = "https://example.org/bench.json";

pub(crate) fn document(rows: usize) -> String {
    let mut output = String::from("{\n  \"rows\": [\n");
    for index in 0..rows {
        if index > 0 {
            output.push_str(",\n");
        }
        write!(output, "    {{\"id\":{index},\"label\":\"café 🐈 {index}\",\"duplicate\":1,\"duplicate\":2,\"nested\":[null,true,{{\"a/b\":\"line\\nnext\"}}],\"empty\":[],\"number\":-0.00e+07}}").unwrap();
    }
    output.push_str("\n  ]\n}\n");
    output
}

pub(crate) fn profile() -> purrdf_json::Profile {
    purrdf_json::Profile::new(
        "bench-json",
        1,
        purrdf_json::Vocabulary::under("https://example.org/json#").unwrap(),
        purrdf_json::Bounds::standard(),
    )
    .unwrap()
}

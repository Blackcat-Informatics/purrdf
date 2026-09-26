// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Replays `pattern_differential_vectors.txt` — JavaScript `RegExp`'s own
//! verdicts under the `u` flag, recorded by `pattern_oracle.mjs` — against
//! the ECMA-262 translation.
//!
//! Every record must agree: the same match verdict for every (pattern,
//! string) pair, the same syntax verdict for every pattern Node refuses, and
//! for the fixed syntax list the same answer to "is this ECMA-262 at all",
//! including for the constructs the translation refuses to run.

use purrdf_jsonschema::ecma::{self, PatternError};
use purrdf_testkit::vectors::{VectorFile, decode_str};

const VECTORS: &str = include_str!("pattern_differential_vectors.txt");

fn test_verdict(pattern: &str, input: &str) -> String {
    match ecma::compile(pattern) {
        Ok(regex) => if regex.is_match(input) {
            "match"
        } else {
            "no-match"
        }
        .to_owned(),
        Err(PatternError::Syntax { .. }) => "syntax-error".to_owned(),
        Err(other) => format!("refused: {other}"),
    }
}

fn syntax_verdict(pattern: &str) -> &'static str {
    if ecma::is_valid_syntax(pattern) {
        "valid"
    } else {
        "syntax-error"
    }
}

#[test]
fn every_node_verdict_is_reproduced() {
    let vectors = VectorFile::parse(VECTORS).expect("the vector file is intact");
    let mut disagreements = Vec::new();
    let mut by_source = [0_usize; 3];
    for record in vectors.records() {
        let fields: Vec<String> = record
            .fields
            .iter()
            .map(|field| decode_str(field).expect("text field"))
            .collect();
        let [source, pattern, input, expected] = fields.as_slice() else {
            panic!("line {}: a record has four fields", record.line);
        };
        let got = match source.as_str() {
            "suite" | "corpus" => {
                by_source[usize::from(source == "corpus")] += 1;
                test_verdict(pattern, input)
            }
            "syntax" => {
                by_source[2] += 1;
                syntax_verdict(pattern).to_owned()
            }
            other => panic!("line {}: unknown source {other:?}", record.line),
        };
        if &got != expected {
            disagreements.push(format!(
                "line {} [{source}] /{pattern}/u on {input:?}: Node says {expected}, this crate {got}",
                record.line
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} disagreements:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
    let declared = |key: &str| -> usize {
        vectors
            .header(key)
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("header {key}"))
    };
    assert_eq!(
        by_source,
        [
            declared("suite-pairs"),
            declared("corpus-pairs"),
            declared("syntax-patterns")
        ]
    );
    assert_eq!(declared("corpus-pairs"), 5000);
}

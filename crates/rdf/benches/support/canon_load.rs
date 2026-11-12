// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![allow(missing_docs)]

//! Load-shaped fixtures for the flat canonical document and digest benches.
//!
//! Both fixtures are built in memory and shaped so every blank node differs at
//! first degree: no n-degree search runs, and the measured cost is the labeling
//! and output construction a large, mostly-unique load pays. `scale` 1 is the
//! normal line length (about 316 bytes per canonical line); `scale` 2 doubles
//! every padded term and leaves the leaf count and line count unchanged.

use std::sync::Arc;

use purrdf_rdf::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral};

/// A frozen fixture and the number of distinct non-fixed terms it interned.
pub(crate) struct Loaded {
    pub(crate) dataset: Arc<RdfDataset>,
    pub(crate) leaves: usize,
}

/// `prefix` followed by `x` characters up to exactly `len` bytes.
fn pad(prefix: &str, len: usize) -> String {
    assert!(
        prefix.len() <= len,
        "fixture bug: prefix {prefix:?} is longer than {len} bytes"
    );
    let mut out = String::with_capacity(len);
    out.push_str(prefix);
    out.extend(std::iter::repeat_n('x', len - prefix.len()));
    out
}

/// Blank-free fixture: `lines` quads, half in four named graphs, two distinct
/// padded IRIs per line.
pub(crate) fn blank_free(lines: usize, scale: usize) -> Loaded {
    let mut b = RdfDatasetBuilder::new();
    let preds: Vec<_> = (0..8)
        .map(|k| {
            b.intern_iri(&pad(&format!("https://example.org/p/{k}/"), 48 * scale))
                .expect("predicate interns")
        })
        .collect();
    let graphs: Vec<_> = (0..4)
        .map(|k| {
            b.intern_iri(&pad(&format!("https://example.org/g/{k}/"), 44 * scale))
                .expect("graph interns")
        })
        .collect();
    let mut leaves = 0;
    for i in 0..lines {
        let s = b
            .intern_iri(&pad(
                &format!("https://example.org/e/{i:09}/s/"),
                116 * scale,
            ))
            .expect("subject interns");
        let o = b
            .intern_iri(&pad(
                &format!("https://example.org/e/{i:09}/o/"),
                116 * scale,
            ))
            .expect("object interns");
        leaves += 2;
        let g = (i % 2 == 1).then(|| graphs[(i / 2) % 4]);
        b.push_quad(s, preds[i % 8], o, g).expect("quad pushes");
    }
    Loaded {
        dataset: b.freeze().expect("blank-free fixture freezes"),
        leaves,
    }
}

/// Reifier fixture: `lines / 4` statements, each a reifier binding plus three
/// annotations, every reifier blank distinct at first degree through its values.
pub(crate) fn reifier(lines: usize, scale: usize) -> Loaded {
    let statements = lines / 4;
    let mut b = RdfDatasetBuilder::new();
    let preds: Vec<_> = (0..8)
        .map(|k| {
            b.intern_iri(&pad(&format!("https://example.org/q/{k}/"), 40 * scale))
                .expect("predicate interns")
        })
        .collect();
    let ann: Vec<_> = (0..3)
        .map(|k| {
            b.intern_iri(&pad(&format!("https://example.org/a/{k}/"), 40 * scale))
                .expect("annotation predicate interns")
        })
        .collect();
    let mut leaves = 0;
    for j in 0..statements {
        let r = b
            .intern_blank(&format!("st{j}"), BlankScope::DEFAULT)
            .expect("reifier blank interns");
        let s = b
            .intern_iri(&pad(
                &format!("https://example.org/r/{j:09}/s/"),
                80 * scale,
            ))
            .expect("subject interns");
        let o = b
            .intern_iri(&pad(
                &format!("https://example.org/r/{j:09}/o/"),
                80 * scale,
            ))
            .expect("object interns");
        let t = b
            .intern_triple(s, preds[j % 8], o)
            .expect("triple term interns");
        b.push_reifier_in_graph(r, t, None)
            .expect("reifier binding pushes");
        let v0 = b
            .intern_iri(&pad(&format!("https://example.org/v/{j:09}/"), 267 * scale))
            .expect("value interns");
        let v1 = b
            .intern_literal(RdfLiteral::simple(pad(
                &format!("value {j} with \"quote\" and \\ slash "),
                260 * scale,
            )))
            .expect("literal interns");
        let v2 = b
            .intern_literal(RdfLiteral::language_tagged(
                pad(&format!("label {j} "), 260 * scale),
                "en",
            ))
            .expect("tagged literal interns");
        leaves += 6;
        for (k, v) in [v0, v1, v2].into_iter().enumerate() {
            b.push_annotation_in_graph(r, ann[k], v, None)
                .expect("annotation pushes");
        }
    }
    Loaded {
        dataset: b.freeze().expect("reifier fixture freezes"),
        leaves,
    }
}

/// Bytes divided by the count of `\n`.
pub(crate) fn mean_line_len(doc: &str) -> f64 {
    let lines = doc.bytes().filter(|&byte| byte == b'\n').count();
    doc.len() as f64 / lines as f64
}

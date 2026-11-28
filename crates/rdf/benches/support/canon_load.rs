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
//!
//! [`Order`] selects how the canonical lines arrive at the output stage. The
//! dataset walks its rows in creation order. `Sorted` spells each line's
//! identifiers with its index, so the walk yields lines already ascending, the
//! best case for a sort that detects order. `Unsorted` keeps every quad's shape
//! and every padded length but spells the identifiers with [`spelled`], a fixed
//! bijection of the index zero padded to the same width, so the walk order and
//! the text order are unrelated.

use std::sync::Arc;

use purrdf_rdf::{BlankScope, DatasetView, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};

/// How the lines arrive at the canonical document's output stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Order {
    /// Identifiers spelled by index: lines arrive ascending.
    Sorted,
    /// Identifiers spelled by [`spelled`]: lines arrive in an order unrelated
    /// to their text order.
    Unsorted,
}

impl Order {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Sorted => "sorted",
            Self::Unsorted => "unsorted",
        }
    }
}

/// Identifiers are drawn from `0..2^29`, which fits the nine-digit spelling.
const SPELLED_BITS: u32 = 29;

/// The identifier line `index` is spelled with. `Unsorted` applies a fixed
/// invertible mix on 29 bits (xor-shifts and odd multiplications modulo
/// 2^29), so distinct indices keep distinct identifiers of the same width.
fn spelled(index: usize, order: Order) -> usize {
    const MASK: u64 = (1 << SPELLED_BITS) - 1;
    let x = u64::try_from(index).expect("the index fits 64 bits");
    assert!(
        x <= MASK,
        "fixture bug: {index} lines exceed 2^29 identifiers"
    );
    match order {
        Order::Sorted => index,
        Order::Unsorted => {
            let mut x = x;
            x ^= x >> 14;
            x = x.wrapping_mul(0x16A0_9E67) & MASK;
            x ^= x >> 13;
            x = x.wrapping_mul(0x0F35_68B3) & MASK;
            x ^= x >> 15;
            usize::try_from(x).expect("a 29-bit identifier fits usize")
        }
    }
}

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
pub(crate) fn blank_free(lines: usize, scale: usize, order: Order) -> Loaded {
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
        let id = spelled(i, order);
        let s = b
            .intern_iri(&pad(
                &format!("https://example.org/e/{id:09}/s/"),
                116 * scale,
            ))
            .expect("subject interns");
        let o = b
            .intern_iri(&pad(
                &format!("https://example.org/e/{id:09}/o/"),
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
pub(crate) fn reifier(lines: usize, scale: usize, order: Order) -> Loaded {
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
        let id = spelled(j, order);
        let r = b
            .intern_blank(&format!("st{j}"), BlankScope::DEFAULT)
            .expect("reifier blank interns");
        let s = b
            .intern_iri(&pad(
                &format!("https://example.org/r/{id:09}/s/"),
                80 * scale,
            ))
            .expect("subject interns");
        let o = b
            .intern_iri(&pad(
                &format!("https://example.org/r/{id:09}/o/"),
                80 * scale,
            ))
            .expect("object interns");
        let t = b
            .intern_triple(s, preds[j % 8], o)
            .expect("triple term interns");
        b.push_reifier_in_graph(r, t, None)
            .expect("reifier binding pushes");
        let v0 = b
            .intern_iri(&pad(
                &format!("https://example.org/v/{id:09}/"),
                267 * scale,
            ))
            .expect("value interns");
        let v1 = b
            .intern_literal(RdfLiteral::simple(pad(
                &format!("value {id} with \"quote\" and \\ slash "),
                260 * scale,
            )))
            .expect("literal interns");
        let v2 = b
            .intern_literal(RdfLiteral::language_tagged(
                pad(&format!("label {id} "), 260 * scale),
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

/// How many of the first `limit` adjacent pairs, in the order `ds` walks its
/// rows, are out of text order: `(descents, pairs)`.
///
/// Blank-free lines start with their subject IRI, so the walked subjects are
/// the sort keys. Reifier lines start with the canonical label of their
/// reifier, which `document` states; each walked reifier binding is matched to
/// its label through the identifier its triple term's subject carries.
pub(crate) fn arrival_descents(
    shape: &str,
    ds: &RdfDataset,
    document: &str,
    limit: usize,
) -> (usize, usize) {
    // `/r/` followed by the nine-digit identifier.
    let id_of = |text: &str| -> Option<String> {
        let at = text.find("/r/")? + 3;
        text.get(at..at + 9).map(str::to_owned)
    };
    let keys: Vec<String> = if shape == "blank_free" {
        ds.quads()
            .take(limit)
            .map(|q| match DatasetView::term_value(ds, q.s) {
                Ok(TermValue::Iri(iri)) => iri,
                other => panic!("a blank-free subject is an IRI, got {other:?}"),
            })
            .collect()
    } else {
        let label_of: std::collections::BTreeMap<String, String> = document
            .lines()
            .filter_map(|line| {
                let (label, _) = line.split_once(' ')?;
                Some((id_of(line)?, format!("{label} ")))
            })
            .collect();
        ds.reifier_quads()
            .take(limit)
            .map(|q| match DatasetView::term_value(ds, q.o) {
                Ok(TermValue::Triple { s, .. }) => match &*s {
                    TermValue::Iri(iri) => {
                        let id = id_of(iri).expect("a reifier subject names its identifier");
                        label_of[&id].clone()
                    }
                    other => panic!("a reifier subject is an IRI, got {other:?}"),
                },
                other => panic!("a reifier target is a triple term, got {other:?}"),
            })
            .collect()
    };
    let descents = keys.windows(2).filter(|pair| pair[0] > pair[1]).count();
    (descents, keys.len().saturating_sub(1))
}

/// Bytes divided by the count of `\n`.
pub(crate) fn mean_line_len(doc: &str) -> f64 {
    let lines = doc.bytes().filter(|&byte| byte == b'\n').count();
    doc.len() as f64 / lines as f64
}

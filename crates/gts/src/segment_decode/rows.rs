// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compact replay of accepted numeric row events, using the shared CBOR heads.
//!
//! Only `push` writes these bytes. The original reader has already decided
//! which rows exist; replay never decodes a frame or rechecks source acceptance.

use purrdf_lex::cbor::head::{self, ARRAY, UNSIGNED};

#[derive(Debug, Default)]
pub(super) struct Rows<const N: usize> {
    bytes: Vec<u8>,
    count: usize,
}

impl<const N: usize> Rows<N> {
    pub(super) fn len(&self) -> usize {
        self.count
    }

    pub(super) fn push(&mut self, segment: u64, ids: [usize; N], graph: Option<usize>) {
        head::push_head(
            &mut self.bytes,
            ARRAY,
            (N + 1 + usize::from(graph.is_some())) as u64,
        );
        head::push_head(&mut self.bytes, UNSIGNED, segment);
        for id in ids.into_iter().chain(graph) {
            head::push_head(&mut self.bytes, UNSIGNED, id as u64);
        }
        self.count += 1;
    }
}

impl<const N: usize> IntoIterator for Rows<N> {
    type Item = (u64, [usize; N], Option<usize>);
    type IntoIter = Iter<N>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            bytes: self.bytes,
            offset: 0,
            remaining: self.count,
        }
    }
}

#[derive(Debug)]
pub(super) struct Iter<const N: usize> {
    bytes: Vec<u8>,
    offset: usize,
    remaining: usize,
}

impl<const N: usize> Iterator for Iter<N> {
    type Item = (u64, [usize; N], Option<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut input = &self.bytes[self.offset..];
        if self.remaining == 0 {
            assert!(
                input.is_empty(),
                "an accepted row spool has no trailing bytes"
            );
            return None;
        }
        assert!(
            !input.is_empty(),
            "an accepted row spool contains every recorded row"
        );
        let width = head::read_argument(&mut input, ARRAY)
            .expect("an accepted row spool contains its own array head");
        assert!(
            width == (N + 1) as u64 || width == (N + 2) as u64,
            "an accepted row spool has its own fixed row width"
        );
        let segment = head::read_argument(&mut input, UNSIGNED)
            .expect("an accepted row spool contains its segment index");
        let mut id = || {
            let value = head::read_argument(&mut input, UNSIGNED)
                .expect("an accepted row spool contains its numeric identifiers");
            usize::try_from(value)
                .expect("an accepted row identifier fits the platform that wrote it")
        };
        let ids = std::array::from_fn(|_| id());
        let graph = (width == (N + 2) as u64).then(&mut id);
        self.offset = self.bytes.len() - input.len();
        self.remaining -= 1;
        Some((segment, ids, graph))
    }
}

impl<const N: usize> std::iter::FusedIterator for Iter<N> {}

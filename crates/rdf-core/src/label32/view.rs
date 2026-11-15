// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical lines of a resident dataset view, blank positions masked.
//!
//! Constants are spelled by the workspace's term writer, the spelling the
//! canonical document already uses. A blank node, at any depth inside a triple
//! term and in a graph position, is written `_:b<number>`. The statement layer is read through the
//! view's own side-table accessors in either presentation.

use std::convert::Infallible;

use crate::{DatasetView, FastMap, TermValue, write_term_value};

/// The two spellings of the statement layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    /// Reifier and annotation rows through the profile's reserved IRIs.
    Overlay,
    /// The same rows as ordinary quads; equal rows are one row.
    Flat,
}

const REIFIES_SENTINEL: &str = "<urn:purrdf:rdfc:reifies>";
const ANNOTATION_SENTINEL: &str = "<urn:purrdf:rdfc:annotation>";
const REIFIES: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>";

struct Reader<'a, D: DatasetView<ReadError = Infallible>> {
    view: &'a D,
    blanks: FastMap<(String, u32), u32>,
}

impl<D: DatasetView<ReadError = Infallible>> Reader<'_, D> {
    fn spell_value(&mut self, v: &TermValue, text: &mut String, tuple: &mut Vec<u32>) {
        match v {
            TermValue::Blank { label, scope } => {
                let next = self.blanks.len() as u32;
                let id = *self
                    .blanks
                    .entry((label.clone(), scope.ordinal()))
                    .or_insert(next);
                text.push_str("_:b");
                text.push_str(&id.to_string());
                tuple.push(id);
            }
            TermValue::Triple { s, p, o } => {
                text.push_str("<<( ");
                self.spell_value(s, text, tuple);
                text.push(' ');
                self.spell_value(p, text, tuple);
                text.push(' ');
                self.spell_value(o, text, tuple);
                text.push_str(" )>>");
            }
            other => write_term_value(other, text),
        }
    }

    fn spell(&mut self, id: D::Id, text: &mut String, tuple: &mut Vec<u32>) {
        let value = self.view.term_value(id).expect("resident read");
        self.spell_value(&value, text, tuple);
    }

    fn graph(&mut self, g: Option<D::Id>, text: &mut String, tuple: &mut Vec<u32>) {
        if let Some(g) = g {
            text.push(' ');
            self.spell(g, text, tuple);
        }
    }
}

/// The distinct lines of `view` in `presentation`.
pub fn lines_from_view<D: DatasetView<ReadError = Infallible>>(
    view: &D,
    presentation: Presentation,
) -> String {
    let mut r = Reader {
        view,
        blanks: FastMap::default(),
    };
    let mut out: Vec<String> = Vec::new();
    let mut push = |text: String, _tuple: Vec<u32>| out.push(text);
    for q in view.quads() {
        let (mut t, mut u) = (String::new(), Vec::new());
        r.spell(q.s, &mut t, &mut u);
        t.push(' ');
        r.spell(q.p, &mut t, &mut u);
        t.push(' ');
        r.spell(q.o, &mut t, &mut u);
        r.graph(q.g, &mut t, &mut u);
        t.push_str(" .");
        push(t, u);
    }
    for q in view.reifier_quads() {
        let (mut t, mut u) = (String::new(), Vec::new());
        r.spell(q.s, &mut t, &mut u);
        t.push(' ');
        t.push_str(match presentation {
            Presentation::Overlay => REIFIES_SENTINEL,
            Presentation::Flat => REIFIES,
        });
        t.push(' ');
        r.spell(q.o, &mut t, &mut u);
        r.graph(q.g, &mut t, &mut u);
        t.push_str(" .");
        push(t, u);
    }
    for q in view.annotation_quads() {
        let (mut t, mut u) = (String::new(), Vec::new());
        r.spell(q.s, &mut t, &mut u);
        t.push(' ');
        r.spell(q.p, &mut t, &mut u);
        t.push(' ');
        r.spell(q.o, &mut t, &mut u);
        if presentation == Presentation::Overlay {
            t.push(' ');
            t.push_str(ANNOTATION_SENTINEL);
        }
        r.graph(q.g, &mut t, &mut u);
        t.push_str(" .");
        push(t, u);
    }
    // The document is a set of lines: a row spelled twice is one row.
    out.sort_unstable();
    out.dedup();
    let mut doc = out.join("\n");
    doc.push('\n');
    doc
}

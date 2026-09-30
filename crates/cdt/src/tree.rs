// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The owning value tree's `Drop`, `Clone` and `Debug`, each over an explicit heap
//! work list rather than the compiler's recursive glue.
//!
//! A [`CdtTerm`] owns [`CdtValue`]s and [`CdtTripleTerm`]s, and those own
//! [`CdtTerm`]s again, so a value is a tree whose depth only [`crate::MAX_ELEMENTS`]
//! and [`crate::MAX_LEXICAL_BYTES`] bound — a level is one element and a few bytes.
//! The glue `#[derive]` writes for such a type recurses once per level, and in Rust
//! a stack overflow is an `abort` no caller can catch, so every walk that touches the
//! whole tree is a loop over a heap stack:
//!
//! * **`Drop`** lives on the two owners whose contents are sealed or structural,
//!   [`CdtValue`] and [`CdtTripleTerm`]. Each moves the nodes it owns onto a work
//!   list and dismantles that list in a loop, taking every popped node's children
//!   before the node itself goes, so no drop it runs recurses. It does not live on
//!   [`CdtTerm`]: a type that implements `Drop` cannot be destructured by value, and
//!   matching an element by value to move its payload out is how the scanner, the
//!   function library and every consumer take a term apart.
//! * **`Clone`** on [`CdtTerm`] builds the copy bottom-up: a two-phase walk enters
//!   each node and, on the way back out, assembles its copy from its children's
//!   finished copies. The derived `Clone` on [`CdtValue`], [`CdtParts`],
//!   [`CdtEntry`] and [`CdtTripleTerm`] stays, because each reaches its children's
//!   copies through this impl within one frame.
//! * **`Debug`** on [`CdtTerm`] prints the *script* `#[derive(Debug)]` prints —
//!   variant names, field names, list lengths, leaf values — token by token, in both
//!   the plain (`{:?}`) and the pretty (`{:#?}`) form, indenting the pretty form the
//!   way the standard library's builders do, so the bytes are the derive's exactly.
//!   The `Debug` on the other four types delegates here within one frame.
//!
//! Equality and ordering are already iterative in [`crate::ops`], and the renderer
//! in [`crate::render`].

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use core::mem;

use purrdf_lex::walk::{WorkList, write_debug};

use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtTripleTerm};
use crate::value::{CdtContents, CdtParts, CdtValue};

// ── Drop ─────────────────────────────────────────────────────────────────────────

/// Whether `term` owns other nodes, so that dropping it as it stands would recurse.
const fn owns_nodes(term: &CdtTerm) -> bool {
    matches!(term, CdtTerm::Composite(_) | CdtTerm::TripleTerm(_))
}

/// Move the nodes `parts` owns onto `work`; the leaves drop here, recursing nowhere.
fn release_parts(parts: CdtParts, work: &mut Vec<CdtTerm>) {
    match parts {
        CdtParts::List(items) => work.extend(items.into_iter().filter(owns_nodes)),
        CdtParts::Map(entries) => work.extend(
            entries
                .into_iter()
                .map(|entry| entry.value)
                .filter(owns_nodes),
        ),
    }
}

/// Move the components `triple` owns onto `work`, leaving it holding three nulls.
fn release_components(triple: &mut CdtTripleTerm, work: &mut Vec<CdtTerm>) {
    for slot in [
        &mut triple.subject,
        &mut triple.predicate,
        &mut triple.object,
    ] {
        let component = mem::replace(slot, CdtTerm::Null);
        if owns_nodes(&component) {
            work.push(component);
        }
    }
}

/// Dismantle `work`: every node popped gives its children up onto the list before it
/// is dropped, so the drop that then runs on it owns nothing and recurses nowhere.
fn reclaim(mut work: Vec<CdtTerm>) {
    while let Some(node) = work.pop() {
        match node {
            CdtTerm::Composite(mut value) => release_parts(value.take_parts(), &mut work),
            CdtTerm::TripleTerm(mut triple) => release_components(&mut triple, &mut work),
            CdtTerm::Iri(_) | CdtTerm::Blank(_) | CdtTerm::Literal(_) | CdtTerm::Null => {}
        }
    }
}

impl Drop for CdtValue {
    fn drop(&mut self) {
        let nested = match self.contents() {
            CdtContents::List(items) => items.iter().any(owns_nodes),
            CdtContents::Map(entries) => entries.iter().any(|entry| owns_nodes(&entry.value)),
        };
        // A value of leaves drops as the compiler drops it; only a value that owns
        // another node is taken apart on the work list.
        if nested {
            let mut work = Vec::new();
            release_parts(self.take_parts(), &mut work);
            reclaim(work);
        }
    }
}

impl Drop for CdtTripleTerm {
    fn drop(&mut self) {
        if owns_nodes(&self.subject) || owns_nodes(&self.predicate) || owns_nodes(&self.object) {
            let mut work = Vec::new();
            release_components(self, &mut work);
            reclaim(work);
        }
    }
}

// ── Clone ────────────────────────────────────────────────────────────────────────

/// The copy of a term that owns no node.
fn shallow(leaf: &CdtTerm) -> CdtTerm {
    match leaf {
        CdtTerm::Iri(iri) => CdtTerm::Iri(iri.clone()),
        CdtTerm::Blank(label) => CdtTerm::Blank(label.clone()),
        CdtTerm::Literal(literal) => CdtTerm::Literal(literal.clone()),
        CdtTerm::Null => CdtTerm::Null,
        CdtTerm::Composite(_) | CdtTerm::TripleTerm(_) => {
            unreachable!("only a leaf is copied shallowly")
        }
    }
}

/// One step of the bottom-up copy.
enum Step<'a> {
    /// Visit a node: copy a leaf outright, or schedule a node's children before it.
    Enter(&'a CdtTerm),
    /// Every child of the node has been copied; assemble the node's own copy.
    Exit(&'a CdtTerm),
}

/// The finished copy on top of `copies`.
fn finished(copies: &mut Vec<CdtTerm>) -> CdtTerm {
    copies
        .pop()
        .expect("a copy is assembled from its children's copies, which precede it")
}

/// A copy of the tree under `root`, built bottom-up over a work list: each node's copy
/// is assembled from its shallow fields and its children's finished copies.
fn clone_tree(root: &CdtTerm) -> CdtTerm {
    let mut steps: Vec<Step<'_>> = alloc::vec![Step::Enter(root)];
    let mut copies: Vec<CdtTerm> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(node) => match node {
                CdtTerm::Composite(value) => {
                    // Children are pushed last-first so they pop, and so their copies
                    // land, in the value's own order.
                    steps.push(Step::Exit(node));
                    match value.contents() {
                        CdtContents::List(items) => {
                            steps.extend(items.iter().rev().map(Step::Enter));
                        }
                        CdtContents::Map(entries) => {
                            steps.extend(
                                entries.iter().rev().map(|entry| Step::Enter(&entry.value)),
                            );
                        }
                    }
                }
                CdtTerm::TripleTerm(triple) => {
                    steps.push(Step::Exit(node));
                    steps.push(Step::Enter(&triple.object));
                    steps.push(Step::Enter(&triple.predicate));
                    steps.push(Step::Enter(&triple.subject));
                }
                leaf => copies.push(shallow(leaf)),
            },
            Step::Exit(node) => {
                let copy = match node {
                    CdtTerm::Composite(value) => {
                        let first = copies.len() - value.len();
                        let kids = copies.drain(first..);
                        let parts = match value.contents() {
                            CdtContents::List(_) => CdtParts::List(kids.collect()),
                            CdtContents::Map(entries) => CdtParts::Map(
                                entries
                                    .iter()
                                    .zip(kids)
                                    .map(|(entry, value)| CdtEntry {
                                        key: entry.key.clone(),
                                        value,
                                    })
                                    .collect(),
                            ),
                        };
                        // The copy holds the same contents, so it carries the same
                        // measure.
                        let extent = value.extent();
                        CdtTerm::Composite(Box::new(match parts {
                            CdtParts::List(items) => CdtValue::from_checked_items(items, extent),
                            CdtParts::Map(entries) => {
                                CdtValue::from_checked_entries(entries, extent)
                            }
                        }))
                    }
                    CdtTerm::TripleTerm(_) => {
                        let object = finished(&mut copies);
                        let predicate = finished(&mut copies);
                        let subject = finished(&mut copies);
                        CdtTerm::TripleTerm(Box::new(CdtTripleTerm {
                            subject,
                            predicate,
                            object,
                        }))
                    }
                    CdtTerm::Iri(_) | CdtTerm::Blank(_) | CdtTerm::Literal(_) | CdtTerm::Null => {
                        unreachable!("only a node that owns nodes is exited")
                    }
                };
                copies.push(copy);
            }
        }
    }
    finished(&mut copies)
}

impl Clone for CdtTerm {
    fn clone(&self) -> Self {
        if owns_nodes(self) {
            clone_tree(self)
        } else {
            shallow(self)
        }
    }
}

// ── Debug ────────────────────────────────────────────────────────────────────────

/// A value a term holds that is not itself a node.
#[derive(Clone, Copy)]
enum Leaf<'a> {
    Str(&'a str),
    Literal(&'a CdtLiteral),
    Key(&'a CdtKey),
}

impl fmt::Debug for Leaf<'_> {
    /// The leaf's own `Debug`, in the form `f` asks for.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Str(x) => fmt::Debug::fmt(x, f),
            Self::Literal(x) => fmt::Debug::fmt(x, f),
            Self::Key(x) => fmt::Debug::fmt(x, f),
        }
    }
}

/// One token of a term's script.
type Tok<'a> = purrdf_lex::walk::Tok<&'a CdtTerm, Leaf<'a>>;

/// Append the script `#[derive(Debug)]` prints for `term` to `out`, each child as a
/// [`Tok::Node`]. A boxed child prints as the child itself, as `Box`'s `Debug` does.
fn script<'a>(term: &'a CdtTerm, out: &mut WorkList<Tok<'a>, 32>) {
    match term {
        CdtTerm::Iri(iri) => {
            out.extend([Tok::Tuple("Iri"), Tok::Leaf(Leaf::Str(iri)), Tok::EndTuple]);
        }
        CdtTerm::Blank(label) => {
            out.extend([
                Tok::Tuple("Blank"),
                Tok::Leaf(Leaf::Str(label)),
                Tok::EndTuple,
            ]);
        }
        CdtTerm::Literal(literal) => {
            out.extend([
                Tok::Tuple("Literal"),
                Tok::Leaf(Leaf::Literal(literal)),
                Tok::EndTuple,
            ]);
        }
        CdtTerm::Null => out.push(Tok::Unit("Null")),
        CdtTerm::TripleTerm(triple) => {
            out.extend([
                Tok::Tuple("TripleTerm"),
                Tok::Struct("CdtTripleTerm"),
                Tok::Field("subject"),
                Tok::Node(&triple.subject),
                Tok::Field("predicate"),
                Tok::Node(&triple.predicate),
                Tok::Field("object"),
                Tok::Node(&triple.object),
                Tok::EndStruct,
                Tok::EndTuple,
            ]);
        }
        CdtTerm::Composite(value) => {
            out.extend([
                Tok::Tuple("Composite"),
                Tok::Struct("CdtValue"),
                Tok::Field("parts"),
            ]);
            match value.contents() {
                CdtContents::List(items) => {
                    out.extend([Tok::Tuple("List"), Tok::List(items.len())]);
                    out.extend(items.iter().map(Tok::Node));
                }
                CdtContents::Map(entries) => {
                    out.extend([Tok::Tuple("Map"), Tok::List(entries.len())]);
                    for entry in entries {
                        out.extend([
                            Tok::Struct("CdtEntry"),
                            Tok::Field("key"),
                            Tok::Leaf(Leaf::Key(&entry.key)),
                            Tok::Field("value"),
                            Tok::Node(&entry.value),
                            Tok::EndStruct,
                        ]);
                    }
                }
            }
            out.extend([Tok::EndList, Tok::EndTuple, Tok::EndStruct, Tok::EndTuple]);
        }
    }
}

impl fmt::Debug for CdtTerm {
    /// `{:?}` / `{:#?}` exactly as `#[derive(Debug)]` writes them, over a work list.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_debug(f, self, script)
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;

    use crate::{CdtTerm, CdtValue, parse_list, parse_map};

    /// A type-for-type twin of the value tree with the compiler's own `Debug`, so the
    /// derive itself is the oracle for what the iterative `Debug` must write.
    #[allow(
        dead_code,
        reason = "the twin's fields exist to be printed by its derived `Debug`, which dead-code \
                  analysis does not count as a read"
    )]
    mod derived {
        use alloc::boxed::Box;
        use alloc::string::String;
        use alloc::vec::Vec;

        #[derive(Debug)]
        pub(super) struct CdtValue {
            pub parts: CdtParts,
        }

        #[derive(Debug)]
        pub(super) enum CdtParts {
            List(Vec<CdtTerm>),
            Map(Vec<CdtEntry>),
        }

        #[derive(Debug)]
        pub(super) enum CdtTerm {
            Iri(String),
            Blank(String),
            Literal(crate::CdtLiteral),
            TripleTerm(Box<CdtTripleTerm>),
            Composite(Box<CdtValue>),
            Null,
        }

        #[derive(Debug)]
        pub(super) struct CdtTripleTerm {
            pub subject: CdtTerm,
            pub predicate: CdtTerm,
            pub object: CdtTerm,
        }

        #[derive(Debug)]
        pub(super) struct CdtEntry {
            pub key: crate::CdtKey,
            pub value: CdtTerm,
        }
    }

    /// The twin of `term`; recursive, on shallow fixtures only.
    fn twin(term: &CdtTerm) -> derived::CdtTerm {
        match term {
            CdtTerm::Iri(iri) => derived::CdtTerm::Iri(iri.clone()),
            CdtTerm::Blank(label) => derived::CdtTerm::Blank(label.clone()),
            CdtTerm::Literal(literal) => derived::CdtTerm::Literal(literal.clone()),
            CdtTerm::Null => derived::CdtTerm::Null,
            CdtTerm::TripleTerm(triple) => {
                derived::CdtTerm::TripleTerm(Box::new(derived::CdtTripleTerm {
                    subject: twin(&triple.subject),
                    predicate: twin(&triple.predicate),
                    object: twin(&triple.object),
                }))
            }
            CdtTerm::Composite(value) => derived::CdtTerm::Composite(Box::new(twin_value(value))),
        }
    }

    fn twin_value(value: &CdtValue) -> derived::CdtValue {
        derived::CdtValue {
            parts: match value.contents() {
                crate::CdtContents::List(items) => {
                    derived::CdtParts::List(items.iter().map(twin).collect())
                }
                crate::CdtContents::Map(entries) => derived::CdtParts::Map(
                    entries
                        .iter()
                        .map(|entry| derived::CdtEntry {
                            key: entry.key.clone(),
                            value: twin(&entry.value),
                        })
                        .collect(),
                ),
            },
        }
    }

    /// Every fixture shape: each leaf kind, an empty and a filled list and map, a
    /// triple term with a composite inside, and a composite-typed literal.
    fn fixtures() -> Vec<CdtValue> {
        [
            "[]",
            "[null]",
            "[1, 'a'@en--rtl, \"q\\\"uote\", <http://example.org/i>, _:b, true]",
            "[[], {}, [[1]], {'k': [null, {'j': <<(1 2 [3])>>}]}]",
            "[<<(<http://example.org/s> <http://example.org/p> <<(_:x _:y _:z)>>)>>]",
            "['[1]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>]",
        ]
        .into_iter()
        .map(|lexical| parse_list(lexical).expect("the fixture parses"))
        .chain([
            parse_map("{}").expect("the fixture parses"),
            parse_map("{'b': [1, {'a': null}], <http://example.org/k>: <<(1 2 3)>>, 1: 2}")
                .expect("the fixture parses"),
        ])
        .collect()
    }

    /// The iterative `Debug` writes byte for byte what the derive writes, in both
    /// forms, for every term and value shape.
    #[test]
    fn the_iterative_debug_is_byte_identical_to_the_derive() {
        for value in fixtures() {
            let expected = twin_value(&value);
            assert_eq!(format!("{value:?}"), format!("{expected:?}"));
            assert_eq!(format!("{value:#?}"), format!("{expected:#?}"));
            for term in value.as_list().into_iter().flatten() {
                let expected = twin(term);
                assert_eq!(format!("{term:?}"), format!("{expected:?}"));
                assert_eq!(format!("{term:#?}"), format!("{expected:#?}"));
            }
        }
    }

    /// A clone is equal to its original and prints identically, for every shape.
    #[test]
    fn a_clone_is_the_same_value() {
        for value in fixtures() {
            let copy = value.clone();
            assert_eq!(copy, value);
            assert_eq!(format!("{copy:?}"), format!("{value:?}"));
            assert_eq!(copy.canonical_lexical(), value.canonical_lexical());
            assert_eq!(copy.element_count(), value.element_count());
        }
    }

    /// A chain of triple terms with no composite between them is a tree the value's
    /// own `Drop` never sees, so the triple term's `Drop` must dismantle it on its
    /// own. Built raw, one level at a time, cloned, and dropped, on a small stack.
    #[test]
    fn a_deep_triple_chain_clones_prints_and_drops_iteratively() {
        let depth = 100_000usize;
        purrdf_stack::on_stack(256 * 1024, move || {
            let mut term = CdtTerm::Null;
            for _ in 0..depth {
                term = CdtTerm::TripleTerm(Box::new(crate::CdtTripleTerm {
                    subject: term,
                    predicate: CdtTerm::Null,
                    object: CdtTerm::Null,
                }));
            }
            let copy = term.clone();
            assert_eq!(copy, term);
            let text: String = format!("{term:?}");
            // Each level writes `TripleTerm(CdtTripleTerm { subject: ` (36 bytes)
            // before its subject and `, predicate: Null, object: Null })` (34
            // bytes) after it, around the innermost `Null` (4 bytes).
            assert_eq!(text.len(), 4 + depth * (36 + 34));
            drop(copy);
            drop(term);
        })
        .expect("the thread starts");
    }
}

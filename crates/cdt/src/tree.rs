// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The owning value tree's iterative `Drop`, `Clone` and `Debug`.
//! Destruction threads ancestors through existing slots without allocating;
//! copying and debugging use explicit work lists instead of recursive glue.
//!
//! A [`CdtTerm`] owns [`CdtValue`]s and [`CdtTripleTerm`]s, and those own
//! [`CdtTerm`]s again, so a value is a tree whose depth only [`crate::MAX_ELEMENTS`]
//! and [`crate::MAX_LEXICAL_BYTES`] bound — a level is one element and a few bytes.
//! The glue `#[derive]` writes for such a type recurses once per level, and in Rust
//! a stack overflow is an `abort` no caller can catch, so every walk that touches the
//! whole tree is iterative; destruction reuses its existing slots and the other
//! walks use a heap work list:
//!
//! * **`Drop`** lives on the two owners whose contents are sealed or structural,
//!   [`CdtValue`] and [`CdtTripleTerm`]. Each threads ancestors through already evacuated term
//!   slots using the one allocation-free lexical dismantling loop, so no drop it runs recurses. It does not live on
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

impl purrdf_lex::walk::DismantleOwned for CdtTerm {
    fn take_child(&mut self) -> Option<Self> {
        match self {
            Self::Composite(value) => match value.dismantling_parts() {
                CdtParts::List(items) => loop {
                    let slot = items.last_mut()?;
                    let child = mem::replace(slot, Self::Null);
                    if owns_nodes(&child) {
                        return Some(child);
                    }
                    drop(child);
                    items.pop();
                },
                CdtParts::Map(entries) => loop {
                    let slot = &mut entries.last_mut()?.value;
                    let child = mem::replace(slot, Self::Null);
                    if owns_nodes(&child) {
                        return Some(child);
                    }
                    drop(child);
                    entries.pop();
                },
            },
            Self::TripleTerm(triple) => loop {
                let child = mem::replace(&mut triple.object, Self::Null);
                if owns_nodes(&child) {
                    return Some(child);
                }
                drop(child);
                triple.object = mem::replace(&mut triple.predicate, Self::Null);
                triple.predicate = mem::replace(&mut triple.subject, Self::Null);
                if matches!(
                    (&triple.subject, &triple.predicate, &triple.object),
                    (Self::Null, Self::Null, Self::Null)
                ) {
                    return None;
                }
            },
            Self::Iri(_) | Self::Blank(_) | Self::Literal(_) | Self::Null => None,
        }
    }

    fn store_parent(&mut self, parent: Option<Self>) {
        let parent = parent.unwrap_or(Self::Null);
        match self {
            Self::Composite(value) => match value.dismantling_parts() {
                CdtParts::List(items) => *items.last_mut().expect("removed a child") = parent,
                CdtParts::Map(entries) => {
                    entries.last_mut().expect("removed a child").value = parent;
                }
            },
            Self::TripleTerm(triple) => triple.object = parent,
            _ => unreachable!("only an owning node removes a child"),
        }
    }

    fn take_parent(&mut self) -> Option<Self> {
        let parent = match self {
            Self::Composite(value) => match value.dismantling_parts() {
                CdtParts::List(items) => items.pop().expect("a parent occupies the last slot"),
                CdtParts::Map(entries) => {
                    entries
                        .pop()
                        .expect("a parent occupies the last slot")
                        .value
                }
            },
            Self::TripleTerm(triple) => {
                let parent = mem::replace(&mut triple.object, Self::Null);
                triple.object = mem::replace(&mut triple.predicate, Self::Null);
                triple.predicate = mem::replace(&mut triple.subject, Self::Null);
                parent
            }
            _ => unreachable!("only an owning node resumes"),
        };
        owns_nodes(&parent).then_some(parent)
    }
}

impl Drop for CdtValue {
    fn drop(&mut self) {
        // Each existing top-level term is the root of the same shared loop. The
        // loop empties every existing box before its ordinary Drop can recurse.
        match self.take_parts() {
            CdtParts::List(items) => {
                for item in items {
                    purrdf_lex::walk::dismantle_owned(item);
                }
            }
            CdtParts::Map(entries) => {
                for entry in entries {
                    purrdf_lex::walk::dismantle_owned(entry.value);
                }
            }
        }
    }
}

impl Drop for CdtTripleTerm {
    fn drop(&mut self) {
        for slot in [&mut self.subject, &mut self.predicate, &mut self.object] {
            purrdf_lex::walk::dismantle_owned(mem::replace(slot, CdtTerm::Null));
        }
    }
}

// ── Clone ────────────────────────────────────────────────────────────────────────

/// The copy of a term that owns no node.
fn shallow(
    leaf: &CdtTerm,
    memory: &mut crate::memory::Memory<'_>,
) -> Result<CdtTerm, crate::memory::StorageError> {
    Ok(match leaf {
        CdtTerm::Iri(iri) => CdtTerm::Iri(memory.string(iri)?),
        CdtTerm::Blank(label) => CdtTerm::Blank(memory.string(label)?),
        CdtTerm::Literal(literal) => CdtTerm::Literal(literal.clone_with_memory(memory)?),
        CdtTerm::Null => CdtTerm::Null,
        CdtTerm::Composite(_) | CdtTerm::TripleTerm(_) => {
            unreachable!("only a leaf is copied shallowly")
        }
    })
}

/// One step of the bottom-up copy.
enum Step<'a> {
    Enter(&'a CdtTerm),
    Exit(&'a CdtTerm),
}

fn finished(copies: &mut Vec<CdtTerm>) -> CdtTerm {
    copies
        .pop()
        .expect("copies are assembled after their children")
}

/// The original bottom-up copy, admitting each native destination before birth.
fn clone_tree(
    root: &CdtTerm,
    memory: &mut crate::memory::Memory<'_>,
) -> Result<CdtTerm, crate::memory::StorageError> {
    use crate::memory::CdtMemory as _;
    let mut steps: WorkList<Step<'_>, 8> = WorkList::new();
    steps.try_push_admitted(Step::Enter(root), memory)?;
    let mut copies: Vec<CdtTerm> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(node) => match node {
                CdtTerm::Composite(value) => {
                    steps.try_push_admitted(Step::Exit(node), memory)?;
                    match value.contents() {
                        CdtContents::List(items) => {
                            for child in items.iter().rev() {
                                steps.try_push_admitted(Step::Enter(child), memory)?;
                            }
                        }
                        CdtContents::Map(entries) => {
                            for entry in entries.iter().rev() {
                                steps.try_push_admitted(Step::Enter(&entry.value), memory)?;
                            }
                        }
                    }
                }
                CdtTerm::TripleTerm(triple) => {
                    steps.try_push_admitted(Step::Exit(node), memory)?;
                    steps.try_push_admitted(Step::Enter(&triple.object), memory)?;
                    steps.try_push_admitted(Step::Enter(&triple.predicate), memory)?;
                    steps.try_push_admitted(Step::Enter(&triple.subject), memory)?;
                }
                leaf => {
                    let copy = shallow(leaf, memory)?;
                    memory.push(&mut copies, copy)?;
                }
            },
            Step::Exit(node) => {
                let copy = match node {
                    CdtTerm::Composite(value) => {
                        let first = copies.len() - value.len();
                        let parts = match value.contents() {
                            CdtContents::List(_) => {
                                let mut items = Vec::new();
                                memory.reserve(&mut items, value.len())?;
                                items.extend(copies.drain(first..));
                                CdtParts::List(items)
                            }
                            CdtContents::Map(entries) => {
                                let mut output = Vec::new();
                                memory.reserve(&mut output, entries.len())?;
                                for (entry, value) in entries.iter().zip(copies.drain(first..)) {
                                    let key = entry.key.clone_with_memory(memory)?;
                                    output.push(CdtEntry { key, value });
                                }
                                CdtParts::Map(output)
                            }
                        };
                        let extent = value.extent();
                        let value = match parts {
                            CdtParts::List(items) => CdtValue::from_checked_items(items, extent),
                            CdtParts::Map(entries) => {
                                CdtValue::from_checked_entries(entries, extent)
                            }
                        };
                        CdtTerm::Composite(memory.boxed_value(value)?)
                    }
                    CdtTerm::TripleTerm(_) => {
                        let object = finished(&mut copies);
                        let predicate = finished(&mut copies);
                        let subject = finished(&mut copies);
                        CdtTerm::TripleTerm(memory.boxed_triple(CdtTripleTerm {
                            subject,
                            predicate,
                            object,
                        })?)
                    }
                    _ => unreachable!("only a node that owns children is exited"),
                };
                memory.push(&mut copies, copy)?;
            }
        }
    }
    let result = finished(&mut copies);
    memory.release_vec(copies)?;
    steps.release_admitted(memory)?;
    Ok(result)
}

// The public clone entry and resident Clone use one protocol. Each payload
// retains its own native clone body, including the original iterative tree law.
macro_rules! clone_entry {
    ($(#[$meta:meta])* $type:ty, $resident:literal) => {
        impl $type {
            $(#[$meta])*
            pub fn clone_admitted(
                &self,
                storage: &mut dyn $crate::memory::Storage,
            ) -> Result<(Self, usize), $crate::memory::StorageError> {
                let mut memory = $crate::memory::Memory::new(storage);
                let value = memory.scope(|memory| self.clone_with_memory(memory))?;
                Ok((value, memory.admitted_bytes()))
            }
        }
        impl Clone for $type {
            fn clone(&self) -> Self {
                self.clone_admitted(&mut $crate::memory::Resident)
                    .expect($resident).0
            }
        }
    };
}
pub(crate) use clone_entry;

impl CdtTerm {
    pub(crate) fn clone_with_memory(
        &self,
        memory: &mut crate::memory::Memory<'_>,
    ) -> Result<Self, crate::memory::StorageError> {
        if owns_nodes(self) {
            clone_tree(self, memory)
        } else {
            shallow(self, memory)
        }
    }
    /// Release only storage already included in this original Memory account.
    pub(crate) fn release_with_memory(
        self,
        memory: &mut crate::memory::Memory<'_>,
    ) -> Result<(), crate::memory::StorageError> {
        match self {
            Self::Iri(value) | Self::Blank(value) => memory.release_string(value),
            Self::Literal(value) => value.release_with_memory(memory),
            Self::Null => Ok(()),
            node => {
                let bytes = owned_bytes(OwnedRoot::Term(&node), memory)?;
                drop(node);
                memory.release_bytes(bytes)
            }
        }
    }
}
crate::tree::clone_entry! {
    /// Copy under original native storage; surviving bytes exclude dead scratch.
    ///
    /// # Errors
    /// Returns checked layout, physical allocator or original admission refusal.
    CdtTerm, "resident CDT clone capacity"
}

impl CdtValue {
    pub(crate) fn clone_with_memory(
        &self,
        memory: &mut crate::memory::Memory<'_>,
    ) -> Result<Self, crate::memory::StorageError> {
        let extent = self.extent();
        match self.contents() {
            CdtContents::List(items) => {
                let mut output = Vec::new();
                memory.reserve(&mut output, items.len())?;
                for item in items {
                    output.push(item.clone_with_memory(memory)?);
                }
                Ok(Self::from_checked_items(output, extent))
            }
            CdtContents::Map(entries) => {
                let mut output = Vec::new();
                memory.reserve(&mut output, entries.len())?;
                for entry in entries {
                    output.push(entry.clone_with_memory(memory)?);
                }
                Ok(Self::from_checked_entries(output, extent))
            }
        }
    }
    pub(crate) fn release_with_memory(
        self,
        memory: &mut crate::memory::Memory<'_>,
    ) -> Result<(), crate::memory::StorageError> {
        let bytes = owned_bytes(OwnedRoot::Value(&self), memory)?;
        drop(self);
        memory.release_bytes(bytes)
    }
}
crate::tree::clone_entry! {
    /// Copy the immutable composite under its original native caller.
    ///
    /// # Errors
    /// Returns checked layout, allocator or original admission refusal.
    CdtValue, "resident CDT clone capacity"
}

/// Both destruction-only roots share one checked physical-layout observation.
/// This cannot admit or certify an unpriced producer.
enum OwnedRoot<'a> {
    Term(&'a CdtTerm),
    Value(&'a CdtValue),
}
fn owned_bytes(
    root: OwnedRoot<'_>,
    memory: &mut crate::memory::Memory<'_>,
) -> Result<usize, crate::memory::StorageError> {
    use crate::memory::StorageError;
    use core::alloc::Layout;
    let mut bytes = 0usize;
    let mut pending: WorkList<OwnedRoot<'_>, 8> = WorkList::new();
    pending.try_push_admitted(root, memory)?;
    while let Some(node) = pending.pop() {
        let local = match node {
            OwnedRoot::Term(term) => match term {
                CdtTerm::Iri(value) | CdtTerm::Blank(value) => value.capacity(),
                CdtTerm::Literal(value) => value.owned_bytes()?,
                CdtTerm::Null => 0,
                CdtTerm::Composite(value) => {
                    pending.try_push_admitted(OwnedRoot::Value(value), memory)?;
                    Layout::new::<CdtValue>().size()
                }
                CdtTerm::TripleTerm(value) => {
                    pending.try_push_admitted(OwnedRoot::Term(&value.object), memory)?;
                    pending.try_push_admitted(OwnedRoot::Term(&value.predicate), memory)?;
                    pending.try_push_admitted(OwnedRoot::Term(&value.subject), memory)?;
                    Layout::new::<CdtTripleTerm>().size()
                }
            },
            OwnedRoot::Value(value) => match value.parts_for_release() {
                CdtParts::List(items) => {
                    for term in items {
                        pending.try_push_admitted(OwnedRoot::Term(term), memory)?;
                    }
                    Layout::array::<CdtTerm>(items.capacity())
                        .map_err(|_| StorageError::SizeOverflow)?
                        .size()
                }
                CdtParts::Map(entries) => {
                    for entry in entries {
                        bytes = bytes
                            .checked_add(entry.key.owned_bytes()?)
                            .ok_or(StorageError::SizeOverflow)?;
                        pending.try_push_admitted(OwnedRoot::Term(&entry.value), memory)?;
                    }
                    Layout::array::<CdtEntry>(entries.capacity())
                        .map_err(|_| StorageError::SizeOverflow)?
                        .size()
                }
            },
        };
        bytes = bytes.checked_add(local).ok_or(StorageError::SizeOverflow)?;
    }
    pending.release_admitted(memory)?;
    Ok(bytes)
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

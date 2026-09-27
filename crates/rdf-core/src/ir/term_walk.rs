// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`TermValue`]'s whole-term operations over work lists, and [`TermBox`], the edge a
//! triple term owns its components through.
//!
//! A triple term nests triple terms to any depth, and every whole-term operation the
//! compiler would derive for it — `Clone`, `==`, `Debug`, the drop — and the ones
//! written by hand — `Ord`, `Hash`, the canonical bytes — would recurse once per
//! level. Each is written here as a loop over an explicit work list instead, so none
//! needs more machine stack for a deeper term.
//!
//! The drop lives on [`TermBox`], not on [`TermValue`]: a type that implements `Drop`
//! cannot be destructured by value, and `TermValue::Triple { s, p, o } => …` is how
//! every consumer takes a triple term apart. What that moves out is three `TermBox`es,
//! whose drop — or [`TermBox::into_inner`] — takes over from there.

use core::cmp::Ordering;
use core::fmt::{self, Write as _};
use core::hash::{Hash, Hasher};
use core::ops::{Deref, DerefMut};

use super::term::TermValue;

/// A stack holding its first `N` entries inline and the rest on the heap: a walk over
/// a shallowly nested term allocates nothing of its own.
struct Pending<T, const N: usize> {
    inline: [Option<T>; N],
    held: usize,
    spill: Vec<T>,
}

impl<T, const N: usize> Pending<T, N> {
    fn with(first: T) -> Self {
        let mut stack = Self {
            inline: core::array::from_fn(|_| None),
            held: 0,
            spill: Vec::new(),
        };
        stack.push(first);
        stack
    }

    fn push(&mut self, value: T) {
        if self.held < N {
            self.inline[self.held] = Some(value);
            self.held += 1;
        } else {
            self.spill.push(value);
        }
    }

    fn pop(&mut self) -> Option<T> {
        if let Some(value) = self.spill.pop() {
            return Some(value);
        }
        self.held = self.held.checked_sub(1)?;
        self.inline[self.held].take()
    }

    fn extend<const K: usize>(&mut self, values: [T; K]) {
        for value in values {
            self.push(value);
        }
    }
}

/// One boxed component of a triple term.
///
/// Reads like a `Box<TermValue>` — it dereferences to the term — and is built with
/// [`TermBox::new`] or `TermValue::into()`, and taken apart with
/// [`TermBox::into_inner`]. Its drop takes a nested triple term apart over a work list,
/// so dropping a term of any depth needs no more machine stack.
pub struct TermBox(Option<Box<TermValue>>);

impl TermBox {
    /// Box `value` as a triple-term component.
    #[must_use]
    pub fn new(value: TermValue) -> Self {
        Self(Some(Box::new(value)))
    }

    /// The component, unboxed.
    #[must_use]
    pub fn into_inner(mut self) -> TermValue {
        *self.take_box()
    }

    /// The component, still boxed.
    #[must_use]
    pub fn into_box(mut self) -> Box<TermValue> {
        self.take_box()
    }

    fn take_box(&mut self) -> Box<TermValue> {
        self.0
            .take()
            .expect("a term box is emptied only by its own drop")
    }
}

impl Drop for TermBox {
    fn drop(&mut self) {
        let Some(node) = self.0.take() else {
            return;
        };
        if !matches!(*node, TermValue::Triple { .. }) {
            return;
        }
        let mut work: Pending<Box<TermValue>, 8> = Pending::with(node);
        while let Some(mut node) = work.pop() {
            if let TermValue::Triple { s, p, o } = &mut *node {
                for component in [s, p, o] {
                    if let Some(inner) = component.0.take()
                        && matches!(*inner, TermValue::Triple { .. })
                    {
                        work.push(inner);
                    }
                }
            }
        }
    }
}

impl Deref for TermBox {
    type Target = TermValue;

    fn deref(&self) -> &TermValue {
        self.0
            .as_deref()
            .expect("a term box is emptied only by its own drop")
    }
}

impl DerefMut for TermBox {
    fn deref_mut(&mut self) -> &mut TermValue {
        self.0
            .as_deref_mut()
            .expect("a term box is emptied only by its own drop")
    }
}

impl AsRef<TermValue> for TermBox {
    fn as_ref(&self) -> &TermValue {
        self
    }
}

impl core::borrow::Borrow<TermValue> for TermBox {
    fn borrow(&self) -> &TermValue {
        self
    }
}

impl From<TermValue> for TermBox {
    fn from(value: TermValue) -> Self {
        Self::new(value)
    }
}

impl From<Box<TermValue>> for TermBox {
    fn from(value: Box<TermValue>) -> Self {
        Self(Some(value))
    }
}

impl Clone for TermBox {
    fn clone(&self) -> Self {
        Self::new(TermValue::clone(self))
    }
}

impl PartialEq for TermBox {
    fn eq(&self, other: &Self) -> bool {
        TermValue::eq(self, other)
    }
}

impl Eq for TermBox {}

impl PartialOrd for TermBox {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TermBox {
    fn cmp(&self, other: &Self) -> Ordering {
        TermValue::cmp(self, other)
    }
}

impl Hash for TermBox {
    fn hash<H: Hasher>(&self, state: &mut H) {
        TermValue::hash(self, state);
    }
}

impl fmt::Debug for TermBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        TermValue::fmt(self, f)
    }
}

impl Clone for TermValue {
    /// A copy, built bottom-up over a work list.
    fn clone(&self) -> Self {
        enum Step<'a> {
            Enter(&'a TermValue),
            Assemble,
        }
        let Self::Triple { .. } = self else {
            return shallow_clone(self);
        };
        let mut stack: Pending<Step<'_>, 32> = Pending::with(Step::Enter(self));
        let mut copies: Vec<Self> = Vec::with_capacity(3);
        while let Some(step) = stack.pop() {
            match step {
                Step::Enter(Self::Triple { s, p, o }) => {
                    stack.extend([
                        Step::Assemble,
                        Step::Enter(o),
                        Step::Enter(p),
                        Step::Enter(s),
                    ]);
                }
                Step::Enter(leaf) => copies.push(shallow_clone(leaf)),
                Step::Assemble => {
                    let o = copies.pop().expect("a triple's object is copied");
                    let p = copies.pop().expect("a triple's predicate is copied");
                    let s = copies.pop().expect("a triple's subject is copied");
                    copies.push(Self::Triple {
                        s: TermBox::new(s),
                        p: TermBox::new(p),
                        o: TermBox::new(o),
                    });
                }
            }
        }
        copies
            .pop()
            .expect("the root's copy is the last one assembled")
    }
}

/// A copy of a term that is not a triple term.
fn shallow_clone(term: &TermValue) -> TermValue {
    match term {
        TermValue::Iri(iri) => TermValue::Iri(iri.clone()),
        TermValue::Blank { label, scope } => TermValue::Blank {
            label: label.clone(),
            scope: *scope,
        },
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => TermValue::Literal {
            lexical_form: lexical_form.clone(),
            datatype: datatype.clone(),
            language: language.clone(),
            direction: *direction,
        },
        TermValue::Triple { .. } => unreachable!("a triple term is copied over the work list"),
    }
}

/// Compare `a` and `b` in pre-order, component by component: the first pair of nodes
/// `shallow` does not call equal decides; a triple's components are compared after
/// the triple itself, subject first.
fn compare_pairs(
    a: &TermValue,
    b: &TermValue,
    shallow: impl Fn(&TermValue, &TermValue) -> Ordering,
) -> Ordering {
    if !matches!(a, TermValue::Triple { .. }) || !matches!(b, TermValue::Triple { .. }) {
        return shallow(a, b);
    }
    let mut pending: Pending<(&TermValue, &TermValue), 16> = Pending::with((a, b));
    while let Some((a, b)) = pending.pop() {
        match shallow(a, b) {
            Ordering::Equal => {}
            decided => return decided,
        }
        if let (
            TermValue::Triple {
                s: sa,
                p: pa,
                o: oa,
            },
            TermValue::Triple {
                s: sb,
                p: pb,
                o: ob,
            },
        ) = (a, b)
        {
            pending.extend([(&**oa, &**ob), (&**pa, &**pb), (&**sa, &**sb)]);
        }
    }
    Ordering::Equal
}

/// The variant and the leaf fields of two terms, not their components: equal when
/// they are the same variant with the same leaves.
fn shallow_eq(a: &TermValue, b: &TermValue) -> bool {
    match (a, b) {
        (TermValue::Iri(a), TermValue::Iri(b)) => a == b,
        (
            TermValue::Blank {
                label: la,
                scope: sa,
            },
            TermValue::Blank {
                label: lb,
                scope: sb,
            },
        ) => la == lb && sa == sb,
        (
            TermValue::Literal {
                lexical_form: la,
                datatype: da,
                language: ga,
                direction: ra,
            },
            TermValue::Literal {
                lexical_form: lb,
                datatype: db,
                language: gb,
                direction: rb,
            },
        ) => la == lb && da == db && ga == gb && ra == rb,
        (TermValue::Triple { .. }, TermValue::Triple { .. }) => true,
        _ => false,
    }
}

impl PartialEq for TermValue {
    /// The same variant, then the same fields — components included — as
    /// `#[derive(PartialEq)]` compares, over a work list.
    fn eq(&self, other: &Self) -> bool {
        compare_pairs(self, other, |a, b| {
            if shallow_eq(a, b) {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        }) == Ordering::Equal
    }
}

impl Eq for TermValue {}

// A TOTAL, dataset-independent order over `TermValue` — the canonical order in which
// `PagedDataset::compact` re-interns the live terms, so the renumbered
// `GlobalTermId` assignment is a pure function of the live term-VALUE set (never of
// ingest order, page order, or the old numbering). Cross-kind order follows
// [`canonical_tag`](TermValue::canonical_tag) (the serializer's IRI < Literal < Blank
// < Triple); within a kind the components compare in the same (datatype, language,
// lexical) precedence the renderer's `ObjKey` uses, with `direction` as a final
// tiebreak so two literals differing ONLY in base direction (distinct values) still
// order deterministically. A triple term compares subject, then predicate, then
// object, each by this same order — walked over a work list.
impl Ord for TermValue {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_pairs(self, other, |a, b| {
            a.canonical_tag()
                .cmp(&b.canonical_tag())
                .then_with(|| match (a, b) {
                    (Self::Iri(a), Self::Iri(b)) => a.cmp(b),
                    (
                        Self::Literal {
                            lexical_form: la,
                            datatype: da,
                            language: ga,
                            direction: dira,
                        },
                        Self::Literal {
                            lexical_form: lb,
                            datatype: db,
                            language: gb,
                            direction: dirb,
                        },
                    ) => da
                        .cmp(db)
                        .then_with(|| ga.cmp(gb))
                        .then_with(|| la.cmp(lb))
                        .then_with(|| dira.cmp(dirb)),
                    (
                        Self::Blank {
                            label: la,
                            scope: sa,
                        },
                        Self::Blank {
                            label: lb,
                            scope: sb,
                        },
                    ) => la.cmp(lb).then_with(|| sa.cmp(sb)),
                    // Equal tags guarantee the same variant; two triple terms are
                    // ordered by their components, which the walk compares next.
                    _ => Ordering::Equal,
                })
        })
    }
}

impl PartialOrd for TermValue {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// `Hash` is hand-written (not derived) with **explicit** discriminant tags so it is
// robust against compiler-dependent enum-discriminant hashing AND matches the
// allocation-free `RdfDataset::hash_term` (which hashes the interned representation
// directly) byte-for-byte. The two MUST stay in sync — the
// `term_id_by_value` round-trip tests fail if they diverge. `String`/`Box<str>`/
// `&str` all hash via `str`, so the by-value datatype here matches the resolved IRI
// string there.
//
// `Hash` is the second of THREE hand-written encodings of this type — [`Ord`] above,
// this `Hash`, and [`TermValue::canonical_bytes`]. All three enumerate the variants
// and their fields by hand, and all three MUST stay in sync with the enum definition:
// adding a variant or a field means visiting every one of them. `Hash` and
// `canonical_bytes` additionally share ONE discriminant numbering (`Iri` = 0, `Blank` =
// 1, `Literal` = 2, `Triple` = 3) so there is never a second, conflicting tag space to
// reconcile. (Note this numbering is deliberately NOT
// [`canonical_tag`](TermValue::canonical_tag), which encodes the serializer's
// cross-kind SORT order and is a different question.)
//
// A triple term feeds its tag and then its subject, predicate and object in turn —
// the pre-order sequence, fed from a work list.
impl Hash for TermValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut pending: Pending<&Self, 16> = Pending::with(self);
        while let Some(term) = pending.pop() {
            match term {
                Self::Iri(iri) => {
                    0u8.hash(state);
                    iri.hash(state);
                }
                Self::Blank { label, scope } => {
                    1u8.hash(state);
                    label.hash(state);
                    scope.hash(state);
                }
                Self::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => {
                    2u8.hash(state);
                    lexical_form.hash(state);
                    datatype.hash(state);
                    language.hash(state);
                    direction.hash(state);
                }
                Self::Triple { s, p, o } => {
                    3u8.hash(state);
                    pending.extend([&**o, &**p, &**s]);
                }
            }
        }
    }
}

/// Writes into a formatter, indenting every line after the first by four spaces per
/// open pretty-printed struct — what the standard library's builders produce by
/// nesting one `PadAdapter` per level.
struct Pad<'f, 'g> {
    f: &'f mut fmt::Formatter<'g>,
    depth: usize,
    line_start: bool,
}

impl fmt::Write for Pad<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for piece in s.split_inclusive('\n') {
            if self.line_start {
                for _ in 0..self.depth {
                    self.f.write_str("    ")?;
                }
            }
            self.line_start = piece.ends_with('\n');
            self.f.write_str(piece)?;
        }
        Ok(())
    }
}

impl fmt::Debug for TermValue {
    /// `{:?}` / `{:#?}` exactly as `#[derive(Debug)]` writes them, over a work list.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        enum Piece<'a> {
            Term(&'a TermValue),
            /// A struct field's name, preceded by what separates it from the one
            /// before; `first` opens the struct.
            Field(&'static str, bool),
            /// The value of a field ends.
            EndField,
            /// A struct with fields closes.
            Close,
        }
        let pretty = f.alternate();
        let mut out = Pad {
            f,
            depth: 0,
            line_start: false,
        };
        let leaf = |out: &mut Pad<'_, '_>, value: &dyn fmt::Debug| {
            if pretty {
                write!(out, "{value:#?}")
            } else {
                write!(out, "{value:?}")
            }
        };
        let mut stack = vec![Piece::Term(self)];
        while let Some(piece) = stack.pop() {
            match piece {
                Piece::Field(name, first) => write_field(&mut out, pretty, name, first)?,
                Piece::EndField => {
                    if pretty {
                        out.write_str(",\n")?;
                    }
                }
                Piece::Close => close(&mut out, pretty)?,
                Piece::Term(Self::Iri(iri)) => {
                    out.write_str("Iri")?;
                    if pretty {
                        out.write_str("(\n")?;
                        out.depth += 1;
                        leaf(&mut out, iri)?;
                        out.write_str(",\n")?;
                        out.depth -= 1;
                    } else {
                        out.write_str("(")?;
                        leaf(&mut out, iri)?;
                    }
                    out.write_str(")")?;
                }
                Piece::Term(Self::Blank { label, scope }) => {
                    out.write_str("Blank")?;
                    for (k, (name, value)) in [
                        ("label", label as &dyn fmt::Debug),
                        ("scope", scope as &dyn fmt::Debug),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        write_field(&mut out, pretty, name, k == 0)?;
                        leaf(&mut out, value)?;
                        if pretty {
                            out.write_str(",\n")?;
                        }
                    }
                    close(&mut out, pretty)?;
                }
                Piece::Term(Self::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                }) => {
                    out.write_str("Literal")?;
                    for (k, (name, value)) in [
                        ("lexical_form", lexical_form as &dyn fmt::Debug),
                        ("datatype", datatype as &dyn fmt::Debug),
                        ("language", language as &dyn fmt::Debug),
                        ("direction", direction as &dyn fmt::Debug),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        write_field(&mut out, pretty, name, k == 0)?;
                        leaf(&mut out, value)?;
                        if pretty {
                            out.write_str(",\n")?;
                        }
                    }
                    close(&mut out, pretty)?;
                }
                Piece::Term(Self::Triple { s, p, o }) => {
                    out.write_str("Triple")?;
                    stack.extend([
                        Piece::Close,
                        Piece::EndField,
                        Piece::Term(o),
                        Piece::Field("o", false),
                        Piece::EndField,
                        Piece::Term(p),
                        Piece::Field("p", false),
                        Piece::EndField,
                        Piece::Term(s),
                        Piece::Field("s", true),
                    ]);
                }
            }
        }
        Ok(())
    }
}

/// Open a struct field: what separates it from the one before, then its name.
fn write_field(out: &mut Pad<'_, '_>, pretty: bool, name: &str, first: bool) -> fmt::Result {
    if pretty {
        if first {
            out.write_str(" {\n")?;
            out.depth += 1;
        }
    } else {
        out.write_str(if first { " { " } else { ", " })?;
    }
    out.write_str(name)?;
    out.write_str(": ")
}

/// Close a struct that has fields.
fn close(out: &mut Pad<'_, '_>, pretty: bool) -> fmt::Result {
    if pretty {
        out.depth -= 1;
        out.write_str("}")
    } else {
        out.write_str(" }")
    }
}

#[cfg(test)]
mod tests {
    use core::cmp::Ordering;
    use core::hash::{Hash, Hasher};

    use proptest::prelude::*;

    use super::{TermBox, TermValue};
    use crate::RdfTextDirection;
    use crate::ir::term::BlankScope;

    /// The recursive reference: the same variants with `Box` components and every
    /// trait derived, plus the hand-written order, hash and encoding as recursive
    /// functions over it.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Reference {
        Iri(String),
        Blank {
            label: String,
            scope: BlankScope,
        },
        Literal {
            lexical_form: String,
            datatype: String,
            language: Option<String>,
            direction: Option<RdfTextDirection>,
        },
        Triple {
            s: Box<Self>,
            p: Box<Self>,
            o: Box<Self>,
        },
    }

    impl Reference {
        fn of(term: &TermValue) -> Self {
            match term {
                TermValue::Iri(iri) => Self::Iri(iri.clone()),
                TermValue::Blank { label, scope } => Self::Blank {
                    label: label.clone(),
                    scope: *scope,
                },
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => Self::Literal {
                    lexical_form: lexical_form.clone(),
                    datatype: datatype.clone(),
                    language: language.clone(),
                    direction: *direction,
                },
                TermValue::Triple { s, p, o } => Self::Triple {
                    s: Box::new(Self::of(s)),
                    p: Box::new(Self::of(p)),
                    o: Box::new(Self::of(o)),
                },
            }
        }

        const fn tag(&self) -> u8 {
            match self {
                Self::Iri(_) => 0,
                Self::Literal { .. } => 1,
                Self::Blank { .. } => 2,
                Self::Triple { .. } => 3,
            }
        }

        fn cmp(&self, other: &Self) -> Ordering {
            self.tag()
                .cmp(&other.tag())
                .then_with(|| match (self, other) {
                    (Self::Iri(a), Self::Iri(b)) => a.cmp(b),
                    (
                        Self::Literal {
                            lexical_form: la,
                            datatype: da,
                            language: ga,
                            direction: ra,
                        },
                        Self::Literal {
                            lexical_form: lb,
                            datatype: db,
                            language: gb,
                            direction: rb,
                        },
                    ) => da
                        .cmp(db)
                        .then_with(|| ga.cmp(gb))
                        .then_with(|| la.cmp(lb))
                        .then_with(|| ra.cmp(rb)),
                    (
                        Self::Blank {
                            label: la,
                            scope: sa,
                        },
                        Self::Blank {
                            label: lb,
                            scope: sb,
                        },
                    ) => la.cmp(lb).then_with(|| sa.cmp(sb)),
                    (
                        Self::Triple {
                            s: sa,
                            p: pa,
                            o: oa,
                        },
                        Self::Triple {
                            s: sb,
                            p: pb,
                            o: ob,
                        },
                    ) => sa.cmp(sb).then_with(|| pa.cmp(pb)).then_with(|| oa.cmp(ob)),
                    _ => Ordering::Equal,
                })
        }

        fn feed<H: Hasher>(&self, state: &mut H) {
            match self {
                Self::Iri(iri) => {
                    0u8.hash(state);
                    iri.hash(state);
                }
                Self::Blank { label, scope } => {
                    1u8.hash(state);
                    label.hash(state);
                    scope.hash(state);
                }
                Self::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => {
                    2u8.hash(state);
                    lexical_form.hash(state);
                    datatype.hash(state);
                    language.hash(state);
                    direction.hash(state);
                }
                Self::Triple { s, p, o } => {
                    3u8.hash(state);
                    s.feed(state);
                    p.feed(state);
                    o.feed(state);
                }
            }
        }
    }

    /// A hasher that keeps every byte it is fed, so two feeds compare exactly.
    #[derive(Default)]
    struct Recorded(Vec<u8>);

    impl Hasher for Recorded {
        fn finish(&self) -> u64 {
            0
        }

        fn write(&mut self, bytes: &[u8]) {
            self.0.extend_from_slice(bytes);
        }
    }

    fn fed(term: &TermValue) -> Vec<u8> {
        let mut state = Recorded::default();
        term.hash(&mut state);
        state.0
    }

    fn reference_fed(term: &Reference) -> Vec<u8> {
        let mut state = Recorded::default();
        term.feed(&mut state);
        state.0
    }

    fn small() -> impl Strategy<Value = String> {
        proptest::sample::select(&["", "a", "ab", "b\n", "\u{e9}"][..]).prop_map(str::to_owned)
    }

    fn term() -> impl Strategy<Value = TermValue> {
        let leaf = prop_oneof![
            small().prop_map(TermValue::Iri),
            (small(), 0u32..3).prop_map(|(label, scope)| TermValue::Blank {
                label,
                scope: BlankScope(scope),
            }),
            (
                small(),
                small(),
                proptest::option::of(small()),
                prop_oneof![
                    Just(None),
                    Just(Some(RdfTextDirection::Ltr)),
                    Just(Some(RdfTextDirection::Rtl)),
                ],
            )
                .prop_map(|(lexical_form, datatype, language, direction)| {
                    TermValue::Literal {
                        lexical_form,
                        datatype,
                        language,
                        direction,
                    }
                }),
        ];
        leaf.prop_recursive(4, 32, 3, |inner| {
            (inner.clone(), inner.clone(), inner).prop_map(|(s, p, o)| TermValue::Triple {
                s: TermBox::new(s),
                p: TermBox::new(p),
                o: TermBox::new(o),
            })
        })
    }

    proptest! {
        /// `Debug`, a copy, `==`, the order, the hash feed and the canonical bytes of
        /// every generated pair agree with the recursive reference.
        #[test]
        fn every_walk_agrees_with_the_recursive_reference(a in term(), b in term()) {
            let (ra, rb) = (Reference::of(&a), Reference::of(&b));
            prop_assert_eq!(format!("{a:?}"), format!("{ra:?}"));
            prop_assert_eq!(format!("{a:#?}"), format!("{ra:#?}"));
            let copy = a.clone();
            prop_assert_eq!(&Reference::of(&copy), &ra);
            prop_assert_eq!(a == b, ra == rb);
            prop_assert_eq!(a.cmp(&b), ra.cmp(&rb));
            prop_assert_eq!(a.cmp(&a), Ordering::Equal);
            prop_assert_eq!(fed(&a), reference_fed(&ra));
            let (ba, bb) = (a.to_canonical_bytes(), b.to_canonical_bytes());
            prop_assert_eq!(ba == bb, a == b);
        }
    }

    /// A triple term a million levels deep is copied, compared, ordered, hashed,
    /// formatted, encoded and dropped on a thread whose whole stack is 128 KiB: a
    /// recursive walk would overflow it many times over.
    #[test]
    fn a_million_level_triple_term_is_walked_without_recursion() {
        const LEVELS: usize = 1_000_000;
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut term = TermValue::iri("o");
                for _ in 0..LEVELS {
                    term = TermValue::Triple {
                        s: TermBox::new(TermValue::iri("s")),
                        p: TermBox::new(TermValue::iri("p")),
                        o: TermBox::new(term),
                    };
                }
                let copy = term.clone();
                assert!(term == copy, "a copy equals its original");
                assert_eq!(term.cmp(&copy), Ordering::Equal);
                assert_eq!(fed(&term), fed(&copy));
                assert!(format!("{term:?}").len() > LEVELS);
                assert_eq!(term.to_canonical_bytes(), copy.to_canonical_bytes());
                drop(copy);
                drop(term);
            })
            .expect("the thread starts")
            .join()
            .expect("no walk overflowed the thread's stack");
    }
}

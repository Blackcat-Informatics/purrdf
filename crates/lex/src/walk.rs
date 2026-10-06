// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-tree walks that spend heap, never machine stack: the work list every
//! such walk keeps ([`WorkList`]), and the `Debug` writer ([`write_debug`]) that
//! prints a recursive type exactly as `#[derive(Debug)]` would, one token at a
//! time.
//!
//! A tree nested as deep as memory allows cannot be walked by recursion: the
//! glue `#[derive]` writes recurses once per level, and a stack overflow is an
//! abort no caller can catch. Every recursive type of the workspace — terms,
//! composite values, geometries, query algebra — therefore walks itself over a
//! work list, and prints itself through one writer. Both live here, below every
//! crate that owns such a type.
//!
//! # The `Debug` script
//!
//! A type describes one node of its value as a flat run of [`Tok`]s — struct and
//! variant names, field names, leaves — and names each nested value as a
//! [`Tok::Node`], which [`write_debug`] expands in place when it pops it. The bytes
//! are the derive's exactly: the separators, the brackets, a bare one-element
//! tuple's trailing comma, and the pretty form's four spaces of indentation per
//! open container are the rules of the standard library's `DebugStruct`,
//! `DebugTuple` and `DebugList` builders and of the `PadAdapter` they nest once
//! per level.
//! Scripts containing only standard primitive leaves can use
//! [`write_debug_scalars`] to preserve every formatter option as well.
//!
//! ```rust
//! use core::fmt;
//!
//! use purrdf_lex::walk::{Tok, WorkList, write_debug};
//!
//! enum Tree {
//!     Leaf(u8),
//!     Pair(Box<Tree>, Box<Tree>),
//! }
//!
//! impl fmt::Debug for Tree {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         write_debug(f, self, |node, out: &mut WorkList<Tok<&Tree, &u8>, 8>| match node {
//!             Tree::Leaf(value) => out.extend([Tok::Tuple("Leaf"), Tok::Leaf(value), Tok::EndTuple]),
//!             Tree::Pair(left, right) => out.extend([
//!                 Tok::Tuple("Pair"),
//!                 Tok::Node(&**left),
//!                 Tok::Node(&**right),
//!                 Tok::EndTuple,
//!             ]),
//!         })
//!     }
//! }
//!
//! let tree = Tree::Pair(Box::new(Tree::Leaf(1)), Box::new(Tree::Leaf(2)));
//! assert_eq!(format!("{tree:?}"), "Pair(Leaf(1), Leaf(2))");
//! assert_eq!(
//!     format!("{tree:#?}"),
//!     "Pair(\n    Leaf(\n        1,\n    ),\n    Leaf(\n        2,\n    ),\n)"
//! );
//! ```

use core::fmt::{self, Write as _};
mod sort;
pub use sort::{try_dedup_by, try_equal_range_by, try_sort_unstable_by};

mod scalar;

pub use scalar::{DebugScalar, write_debug_scalars};

/// A stack holding its first `N` entries inline and the rest on the heap.
///
/// A walk over a shallow tree — the common case — never grows past `N` pending
/// entries, so it allocates nothing of its own: an iterative `Clone`, `==`,
/// `Hash`, `Debug` or drop costs the allocations of the tree it builds and no
/// others. A deep or wide tree spills, and is walked in heap memory rather than
/// on the machine stack.
pub struct WorkList<T, const N: usize> {
    inline: [Option<T>; N],
    /// How many of `inline`'s slots are occupied, from the bottom.
    held: usize,
    /// Entries past the first `N`; non-empty only when `inline` is full.
    spill: Vec<T>,
}

impl<T, const N: usize> fmt::Debug for WorkList<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkList")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

purrdf_hash::default_from_new!([T, const N: usize] WorkList<T, N>);

impl<T, const N: usize> WorkList<T, N> {
    /// An empty stack; it allocates nothing until it holds more than `N` entries.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inline: core::array::from_fn(|_| None),
            held: 0,
            spill: Vec::new(),
        }
    }

    /// A stack holding `first`.
    #[must_use]
    pub fn with(first: T) -> Self {
        let mut stack = Self::new();
        stack.push(first);
        stack
    }

    /// Push `value` on top.
    pub fn push(&mut self, value: T) {
        if self.held < N {
            self.inline[self.held] = Some(value);
            self.held += 1;
        } else {
            self.spill.push(value);
        }
    }

    /// Take the top entry, or `None` when the stack is empty.
    pub fn pop(&mut self) -> Option<T> {
        if let Some(value) = self.spill.pop() {
            return Some(value);
        }
        self.held = self.held.checked_sub(1)?;
        self.inline[self.held].take()
    }

    /// How many entries the stack holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held + self.spill.len()
    }

    /// Whether the stack holds nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.held == 0
    }

    /// The top entry.
    pub fn top_mut(&mut self) -> Option<&mut T> {
        if let Some(top) = self.spill.last_mut() {
            return Some(top);
        }
        self.held
            .checked_sub(1)
            .and_then(|top| self.inline[top].as_mut())
    }

    /// The entry at `index` from the bottom, which the caller knows exists.
    fn slot(&mut self, index: usize) -> &mut T {
        if index < N {
            self.inline[index]
                .as_mut()
                .expect("an occupied slot below the stack's length")
        } else {
            &mut self.spill[index - N]
        }
    }

    /// Reverse the order of the top `count` entries: what was pushed in order pops
    /// in the same order.
    ///
    /// # Panics
    ///
    /// When `count` exceeds [`Self::len`].
    pub fn reverse_top(&mut self, count: usize) {
        let len = self.len();
        let (mut low, mut high) = (len - count, len);
        while low + 1 < high {
            high -= 1;
            if high < N {
                self.inline.swap(low, high);
            } else if low >= N {
                self.spill.swap(low - N, high - N);
            } else {
                let lower = self.inline[low]
                    .take()
                    .expect("an occupied slot below the stack's length");
                let upper = core::mem::replace(&mut self.spill[high - N], lower);
                self.inline[low] = Some(upper);
            }
            low += 1;
        }
    }

    /// Replace the entry at `index` from the bottom.
    ///
    /// # Panics
    ///
    /// When `index` is not below [`Self::len`].
    pub fn set(&mut self, index: usize, value: T) {
        *self.slot(index) = value;
    }
}

impl<T, const N: usize> Extend<T> for WorkList<T, N> {
    /// Push every entry, in order.
    fn extend<I: IntoIterator<Item = T>>(&mut self, values: I) {
        for value in values {
            self.push(value);
        }
    }
}

/// A value that a [`Nested`] box drops without recursing.
///
/// A recursive type owns its children through [`Nested`] boxes, and the
/// compiler's drop glue for it would recurse once per level. Instead the box
/// hands the value it owns to [`Dismantle::dismantle`], which takes the nested
/// values apart over a work list, so no drop recurses however deep the value.
pub trait Dismantle: Sized {
    /// Drop `node` and everything it nests, over a work list: take each nested
    /// box's value out ([`Nested::take`]) before its owner goes.
    fn dismantle(node: Box<Self>);
}

/// One boxed nested value of a recursive type — a triple term's component, an
/// operator's operand — whose drop takes the nesting apart over a work list
/// ([`Dismantle`]) instead of recursing.
///
/// Reads like a `Box<T>`: it dereferences to the value, is built with
/// [`Nested::new`] or `T::into()`, and is taken apart with [`Nested::into_inner`].
/// `Clone`, `==`, ordering, `Hash` and `Debug` are the value's own. The drop
/// lives on this box rather than on the recursive type because a type that
/// implements `Drop` cannot be destructured by value, and matching a node by
/// value to move its fields out is how every consumer takes one apart: what that
/// moves out is a box, whose drop — or [`Nested::into_inner`] — takes over.
pub struct Nested<T: Dismantle>(Option<Box<T>>);

impl<T: Dismantle> Nested<T> {
    /// Box `value` as a nested value.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self::from(Box::new(value))
    }

    /// The value, unboxed.
    #[must_use]
    pub fn into_inner(mut self) -> T {
        *self.take_box()
    }

    /// The value, still boxed.
    #[must_use]
    pub fn into_box(mut self) -> Box<T> {
        self.take_box()
    }

    /// The boxed value, taken out by a [`Dismantle`] walk that is dropping this
    /// box's owner; `None` once taken. A taken box is never read again: it only
    /// drops, as nothing.
    pub fn take(&mut self) -> Option<Box<T>> {
        self.0.take()
    }

    fn take_box(&mut self) -> Box<T> {
        self.0
            .take()
            .expect("a nested box is emptied only while it is dropped")
    }

    fn get(&self) -> &T {
        self.0
            .as_deref()
            .expect("a nested box is emptied only while it is dropped")
    }
}

impl<T: Dismantle> Drop for Nested<T> {
    fn drop(&mut self) {
        if let Some(node) = self.0.take() {
            T::dismantle(node);
        }
    }
}

impl<T: Dismantle> core::ops::Deref for Nested<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.get()
    }
}

impl<T: Dismantle> core::ops::DerefMut for Nested<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0
            .as_deref_mut()
            .expect("a nested box is emptied only while it is dropped")
    }
}

impl<T: Dismantle> AsRef<T> for Nested<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: Dismantle> AsMut<T> for Nested<T> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: Dismantle> core::borrow::Borrow<T> for Nested<T> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: Dismantle> From<T> for Nested<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Dismantle> From<Box<T>> for Nested<T> {
    fn from(value: Box<T>) -> Self {
        Self(Some(value))
    }
}

impl<T: Dismantle + Clone> Clone for Nested<T> {
    fn clone(&self) -> Self {
        Self::new(self.get().clone())
    }
}

impl<T: Dismantle + PartialEq> PartialEq for Nested<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Dismantle + Eq> Eq for Nested<T> {}

impl<T: Dismantle + PartialOrd> PartialOrd for Nested<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<T: Dismantle + Ord> Ord for Nested<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.get().cmp(other.get())
    }
}

impl<T: Dismantle + core::hash::Hash> core::hash::Hash for Nested<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: Dismantle + fmt::Debug> fmt::Debug for Nested<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

/// One token of a value's `Debug` script: what `#[derive(Debug)]` prints for one
/// node, with each nested value left as a [`Tok::Node`] of its own.
#[derive(Clone, Copy, Debug)]
pub enum Tok<N, L> {
    /// A struct or struct-like variant opens: its name.
    Struct(&'static str),
    /// A field of the open struct: its name. Its value follows.
    Field(&'static str),
    /// The open struct closes.
    EndStruct,
    /// A tuple or tuple-like variant opens: its name, empty for a bare tuple.
    Tuple(&'static str),
    /// The open tuple closes.
    EndTuple,
    /// A fieldless variant.
    Unit(&'static str),
    /// A list of this many entries opens. `Debug` does not print the count; a
    /// walk that hashes or compares a script reads it.
    List(usize),
    /// The open list closes.
    EndList,
    /// A leaf, written with its own `Debug` in the form being printed.
    Leaf(L),
    /// A nested value, read as its own script when it is reached.
    Node(N),
}

/// Write `root`'s script into `f`, in the form `f` asks for (`{:?}` or `{:#?}`).
///
/// `script` pushes the tokens of one node onto the pending stack in reading
/// order; each [`Tok::Node`] is expanded the moment it is popped, so the walk
/// costs heap and never stack. This is the one writer of the derive's `Debug`
/// form, so every hand-written iterative `Debug` in the workspace prints the
/// bytes the derive would.
///
/// # Errors
///
/// What `f` returns.
///
/// # Panics
///
/// When the script is unbalanced: a field outside a struct, or a close with
/// nothing open.
pub fn write_debug<N, L: fmt::Debug, const K: usize>(
    f: &mut fmt::Formatter<'_>,
    root: N,
    script: impl FnMut(N, &mut WorkList<Tok<N, L>, K>),
) -> fmt::Result {
    let pretty = f.alternate();
    write_debug_with(f, root, script, |leaf, out| {
        if pretty {
            write!(out, "{leaf:#?}")
        } else {
            write!(out, "{leaf:?}")
        }
    })
}

/// The sole traversal and container writer; each entry supplies only its leaves,
/// written at the current position of the indenting writer.
fn write_debug_with<N, L, const K: usize>(
    f: &mut fmt::Formatter<'_>,
    root: N,
    mut script: impl FnMut(N, &mut WorkList<Tok<N, L>, K>),
    mut render_leaf: impl FnMut(L, &mut Pad<'_, '_>) -> fmt::Result,
) -> fmt::Result {
    let pretty = f.alternate();
    let mut out = Pad {
        f,
        pretty,
        depth: 0,
        line_start: false,
    };
    let mut stack: WorkList<Tok<N, L>, K> = WorkList::with(Tok::Node(root));
    let mut open: WorkList<Open, 16> = WorkList::new();
    while let Some(tok) = stack.pop() {
        match tok {
            Tok::Node(node) => {
                // The node's script replaces its token, reversed so that its first
                // token is popped next.
                let before = stack.len();
                script(node, &mut stack);
                stack.reverse_top(stack.len() - before);
            }
            Tok::Struct(name) => out.open(&mut open, name, OpenKind::Struct)?,
            Tok::Tuple(name) => out.open(
                &mut open,
                name,
                OpenKind::Tuple {
                    bare: name.is_empty(),
                },
            )?,
            Tok::List(_) => out.open(&mut open, "[", OpenKind::List)?,
            Tok::Field(name) => {
                let container = open
                    .top_mut()
                    .expect("a field is written inside its struct");
                if pretty {
                    if container.entries == 0 {
                        out.write_str(" {\n")?;
                        out.depth += 1;
                    }
                } else {
                    out.write_str(if container.entries == 0 { " { " } else { ", " })?;
                }
                container.entries += 1;
                out.write_str(name)?;
                out.write_str(": ")?;
            }
            Tok::EndStruct | Tok::EndTuple | Tok::EndList => {
                let container = open.pop().expect("a container closes after it opens");
                out.close(container)?;
                out.end(&open)?;
            }
            Tok::Unit(name) => {
                out.begin(&mut open)?;
                out.write_str(name)?;
                out.end(&open)?;
            }
            Tok::Leaf(leaf) => {
                out.begin(&mut open)?;
                render_leaf(leaf, &mut out)?;
                out.end(&open)?;
            }
        }
    }
    Ok(())
}

/// Writes into a formatter, indenting every line after the first by four spaces per
/// open pretty-printed container — what nesting the standard library's `PadAdapter`
/// once per level produces.
struct Pad<'f, 'g> {
    f: &'f mut fmt::Formatter<'g>,
    pretty: bool,
    depth: usize,
    line_start: bool,
}

impl fmt::Write for Pad<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for piece in s.split_inclusive('\n') {
            self.indent()?;
            self.line_start = piece.ends_with('\n');
            self.f.write_str(piece)?;
        }
        Ok(())
    }
}

impl<'g> Pad<'_, 'g> {
    /// Write the indentation a pending line start owes, before the next byte.
    fn indent(&mut self) -> fmt::Result {
        if self.line_start {
            self.line_start = false;
            for _ in 0..self.depth {
                self.f.write_str("    ")?;
            }
        }
        Ok(())
    }

    /// The caller's own formatter at the current position, for a value whose
    /// output contains no line break: every option it carries applies unchanged.
    fn formatter(&mut self) -> Result<&mut fmt::Formatter<'g>, fmt::Error> {
        self.indent()?;
        Ok(&mut *self.f)
    }
}

/// A container open in [`write_debug`]: what it is and how many entries it holds
/// so far.
#[derive(Clone, Copy)]
struct Open {
    kind: OpenKind,
    entries: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenKind {
    Struct,
    Tuple { bare: bool },
    List,
}

impl Pad<'_, '_> {
    /// A container starts as a value of the innermost open one: write what
    /// separates it, then its opening (`name` for a struct or tuple, `[` for a list).
    fn open(
        &mut self,
        open: &mut WorkList<Open, 16>,
        opening: &str,
        kind: OpenKind,
    ) -> fmt::Result {
        self.begin(open)?;
        self.write_str(opening)?;
        open.push(Open { kind, entries: 0 });
        Ok(())
    }

    /// A value starts inside the innermost open container: write what separates it
    /// from the entry before it.
    fn begin(&mut self, open: &mut WorkList<Open, 16>) -> fmt::Result {
        let Some(container) = open.top_mut() else {
            return Ok(());
        };
        match container.kind {
            // The field token already wrote the separator and the name.
            OpenKind::Struct => return Ok(()),
            OpenKind::Tuple { .. } => {
                if self.pretty {
                    if container.entries == 0 {
                        self.write_str("(\n")?;
                        self.depth += 1;
                    }
                } else {
                    self.write_str(if container.entries == 0 { "(" } else { ", " })?;
                }
            }
            OpenKind::List => {
                if self.pretty {
                    if container.entries == 0 {
                        self.write_str("\n")?;
                        self.depth += 1;
                    }
                } else if container.entries > 0 {
                    self.write_str(", ")?;
                }
            }
        }
        container.entries += 1;
        Ok(())
    }

    /// Close `container`, which has just been popped.
    fn close(&mut self, container: Open) -> fmt::Result {
        match container.kind {
            OpenKind::Struct if container.entries > 0 => {
                if self.pretty {
                    self.depth -= 1;
                    self.write_str("}")
                } else {
                    self.write_str(" }")
                }
            }
            OpenKind::Tuple { bare } if container.entries > 0 => {
                if self.pretty {
                    self.depth -= 1;
                } else if container.entries == 1 && bare {
                    self.write_str(",")?;
                }
                self.write_str(")")
            }
            OpenKind::Struct | OpenKind::Tuple { .. } => Ok(()),
            OpenKind::List => {
                if self.pretty && container.entries > 0 {
                    self.depth -= 1;
                }
                self.write_str("]")
            }
        }
    }

    /// A value inside an open container has ended.
    fn end(&mut self, open: &WorkList<Open, 16>) -> fmt::Result {
        if self.pretty && !open.is_empty() {
            self.write_str(",\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use super::{Dismantle, Nested, Tok, WorkList, write_debug};

    #[test]
    fn it_pops_in_reverse_push_order_across_the_spill() {
        let mut stack: WorkList<usize, 3> = WorkList::new();
        for i in 0..10 {
            stack.push(i);
        }
        assert_eq!(stack.len(), 10);
        let popped: Vec<_> = core::iter::from_fn(|| stack.pop()).collect();
        assert_eq!(popped, (0..10).rev().collect::<Vec<_>>());
        assert!(stack.is_empty());
        assert!(stack.pop().is_none());
    }

    #[test]
    fn reversing_the_top_crosses_the_inline_boundary() {
        for pushed in 0..9 {
            for count in 0..=pushed {
                let mut stack: WorkList<usize, 4> = WorkList::new();
                stack.extend(0..pushed);
                stack.reverse_top(count);
                let popped: Vec<_> = core::iter::from_fn(|| stack.pop()).collect();
                let mut expected: Vec<_> = (0..pushed).collect();
                expected[pushed - count..].reverse();
                expected.reverse();
                assert_eq!(popped, expected, "pushed {pushed}, reversed {count}");
            }
        }
    }

    #[test]
    fn set_replaces_an_entry_in_either_storage() {
        let mut stack: WorkList<usize, 2> = WorkList::new();
        stack.extend(0..4);
        stack.set(1, 10);
        stack.set(3, 30);
        let popped: Vec<_> = core::iter::from_fn(|| stack.pop()).collect();
        assert_eq!(popped, [30, 2, 10, 0]);
    }

    #[test]
    fn top_mut_reaches_the_top_in_either_storage() {
        let mut stack: WorkList<usize, 2> = WorkList::with(1);
        *stack.top_mut().expect("one entry") += 10;
        stack.extend([2, 3]);
        *stack.top_mut().expect("three entries") += 30;
        let popped: Vec<_> = core::iter::from_fn(|| stack.pop()).collect();
        assert_eq!(popped, [33, 2, 11]);
        assert!(stack.top_mut().is_none());
    }

    /// A linked chain whose links are [`Nested`] boxes, dismantled over a work list.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    enum Chain {
        End,
        Link(Nested<Self>),
    }

    impl Dismantle for Chain {
        fn dismantle(node: Box<Self>) {
            let mut work = vec![node];
            while let Some(mut node) = work.pop() {
                if let Self::Link(next) = &mut *node
                    && let Some(next) = next.take()
                {
                    work.push(next);
                }
            }
        }
    }

    #[test]
    fn a_nested_box_reads_like_the_value_it_owns() {
        let chain = Chain::Link(Nested::new(Chain::End));
        let Chain::Link(link) = &chain else {
            unreachable!("built as a link")
        };
        assert_eq!(**link, Chain::End);
        assert_eq!(link.as_ref(), &Chain::End);
        assert_eq!(chain.clone(), chain);
        assert_eq!(format!("{chain:?}"), "Link(End)");
        assert!(Chain::End < chain);
        let owned = Nested::from(Box::new(Chain::End));
        assert_eq!(owned.clone().into_inner(), Chain::End);
        assert_eq!(*owned.into_box(), Chain::End);
    }

    /// A chain a hundred thousand links long drops on a thread whose whole stack is
    /// 256 KiB.
    #[test]
    fn a_hundred_thousand_link_chain_drops_on_a_256_kib_thread() {
        purrdf_stack::on_stack(256 * 1024, || {
            let mut chain = Chain::End;
            for _ in 0..100_000 {
                chain = Chain::Link(Nested::new(chain));
            }
            drop(chain);
        })
        .expect("a 256 KiB stack thread runs the drop");
    }

    /// Every shape the writer prints, with the compiler's own `Debug` as the oracle.
    #[derive(Debug)]
    #[allow(
        dead_code,
        reason = "the fields exist to be printed by the derived `Debug`, which dead-code \
                  analysis does not count as a read"
    )]
    enum Derived {
        Unit,
        Leaf(u8),
        Named { left: Box<Self>, right: Box<Self> },
        Bare((Box<Self>,)),
        Many(Vec<Self>),
        Empty {},
        Text(&'static str),
    }

    /// The same tree, printed through [`write_debug`].
    struct Scripted<'a>(&'a Derived);

    impl fmt::Debug for Scripted<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_debug(
                f,
                self.0,
                |node, out: &mut WorkList<Tok<&Derived, &dyn fmt::Debug>, 4>| match node {
                    Derived::Unit => out.push(Tok::Unit("Unit")),
                    Derived::Leaf(value) => {
                        out.extend([
                            Tok::Tuple("Leaf"),
                            Tok::Leaf(value as &dyn fmt::Debug),
                            Tok::EndTuple,
                        ]);
                    }
                    Derived::Named { left, right } => out.extend([
                        Tok::Struct("Named"),
                        Tok::Field("left"),
                        Tok::Node(&**left),
                        Tok::Field("right"),
                        Tok::Node(&**right),
                        Tok::EndStruct,
                    ]),
                    Derived::Bare((inner,)) => out.extend([
                        Tok::Tuple("Bare"),
                        Tok::Tuple(""),
                        Tok::Node(&**inner),
                        Tok::EndTuple,
                        Tok::EndTuple,
                    ]),
                    Derived::Many(items) => {
                        out.extend([Tok::Tuple("Many"), Tok::List(items.len())]);
                        out.extend(items.iter().map(Tok::Node));
                        out.extend([Tok::EndList, Tok::EndTuple]);
                    }
                    Derived::Empty {} => out.extend([Tok::Struct("Empty"), Tok::EndStruct]),
                    Derived::Text(text) => {
                        out.extend([
                            Tok::Tuple("Text"),
                            Tok::Leaf(text as &dyn fmt::Debug),
                            Tok::EndTuple,
                        ]);
                    }
                },
            )
        }
    }

    fn fixtures() -> Vec<Derived> {
        vec![
            Derived::Unit,
            Derived::Leaf(7),
            Derived::Empty {},
            Derived::Text("a \"quoted\"\nline"),
            Derived::Many(Vec::new()),
            Derived::Many(vec![Derived::Unit, Derived::Leaf(1)]),
            Derived::Bare((Box::new(Derived::Leaf(2)),)),
            Derived::Named {
                left: Box::new(Derived::Many(vec![Derived::Empty {}])),
                right: Box::new(Derived::Bare((Box::new(Derived::Unit),))),
            },
        ]
    }

    #[test]
    fn the_writer_prints_what_the_derive_prints_in_both_forms() {
        for fixture in fixtures() {
            assert_eq!(format!("{:?}", Scripted(&fixture)), format!("{fixture:?}"));
            assert_eq!(
                format!("{:#?}", Scripted(&fixture)),
                format!("{fixture:#?}")
            );
        }
    }

    /// A tree a hundred thousand levels deep prints on a thread whose whole stack is
    /// 256 KiB.
    #[test]
    fn a_hundred_thousand_level_tree_prints_on_a_256_kib_thread() {
        const DEPTH: usize = 100_000;
        purrdf_stack::on_stack(256 * 1024, || {
            let mut tree = Derived::Leaf(0);
            for _ in 0..DEPTH {
                tree = Derived::Many(vec![tree]);
            }
            let plain = format!("{:?}", Scripted(&tree));
            assert_eq!(
                plain.len(),
                "Many([".len() * DEPTH + "Leaf(0)".len() + "])".len() * DEPTH
            );
            let mut work = vec![tree];
            while let Some(node) = work.pop() {
                if let Derived::Many(items) = node {
                    work.extend(items);
                }
            }
        })
        .expect("a 256 KiB stack thread runs the walk");
    }
}

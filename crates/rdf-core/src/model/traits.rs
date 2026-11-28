// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The recursive triple boundary of the owned model's value traits.

use core::convert::Infallible;
use core::fmt;
use core::hash::{Hash, Hasher};

use purrdf_lex::walk::{DebugScalar, Tok, WorkList, write_debug_scalars};

use super::{RdfLiteral, RdfTerm, RdfTextDirection, RdfTriple};

/// How many quoted-triple levels, the root included, each value trait walks by
/// direct calls before handing deeper children to its heap walk. Every bounded
/// shape the model's callers commonly hold — a flat triple and up to three
/// levels of nesting below it — therefore costs exactly the derive's calls.
const DIRECT_LEVELS: u8 = 4;

impl Clone for RdfTerm {
    /// The derive's leaf arms, verbatim. A triple term is copied by one outlined
    /// call that builds the copy in its new allocation and walks the bounded
    /// levels and then the heap fold, so this dispatch never recurses.
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Self::Iri(value) => Self::Iri(value.clone()),
            Self::BlankNode(value) => Self::BlankNode(value.clone()),
            Self::Literal(value) => Self::Literal(value.clone()),
            Self::Triple(triple) => Self::Triple(clone_boxed(triple)),
        }
    }
}

/// A boxed triple's copy, written straight into its allocation. Kept out of
/// line and marked cold so the leaf arms of [`RdfTerm`]'s clone are laid out as
/// the derive's are: measured in retired instructions, an inline or hot triple
/// arm cost the literal arm about 2%, and this costs a triple nothing.
#[cold]
#[inline(never)]
fn clone_boxed(triple: &RdfTriple) -> Box<RdfTriple> {
    Box::write(Box::new_uninit(), clone_at_depth::<DIRECT_LEVELS>(triple))
}

impl Clone for RdfTriple {
    /// Reconstruct bounded children directly and fold deeper children on the heap.
    #[inline(never)]
    fn clone(&self) -> Self {
        clone_at_depth::<DIRECT_LEVELS>(self)
    }
}

/// Direct child calls are literally 4 -> 3 -> 2 -> 1 -> 0, then the existing heap fold.
#[allow(
    clippy::inline_always,
    reason = "the fixed compile-time depth removes the prewalk and recursive derive dispatch"
)]
#[inline(always)]
fn clone_at_depth<const DEPTH: u8>(triple: &RdfTriple) -> RdfTriple {
    let clone_child: fn(&RdfTerm) -> RdfTerm = match DEPTH {
        4 => clone_term_at_depth::<3>,
        3 => clone_term_at_depth::<2>,
        2 => clone_term_at_depth::<1>,
        _ => clone_term_at_depth::<0>,
    };
    // Field by field in declaration order, as the derive builds it, so each
    // field is written straight into the destination.
    RdfTriple {
        subject: clone_child(&triple.subject),
        predicate: triple.predicate.clone(),
        object: clone_child(&triple.object),
        location: triple.location.clone(),
    }
}

#[allow(
    clippy::inline_always,
    reason = "only the literal bounded instantiations call each other before the heap fold"
)]
#[inline(always)]
fn clone_term_at_depth<const DEPTH: u8>(term: &RdfTerm) -> RdfTerm {
    // One dispatch: the leaf arms are the derive's, and a leaf cannot reenter
    // Triple.
    match term {
        RdfTerm::Iri(value) => RdfTerm::Iri(value.clone()),
        RdfTerm::BlankNode(value) => RdfTerm::BlankNode(value.clone()),
        RdfTerm::Literal(value) => RdfTerm::Literal(value.clone()),
        RdfTerm::Triple(_) if DEPTH == 0 => clone_nested(term),
        // Allocate first, as the derived `Box` clone does, so the copy is built
        // in its heap slot rather than on the stack and then moved there.
        RdfTerm::Triple(triple) => RdfTerm::Triple(Box::write(
            Box::new_uninit(),
            clone_at_depth::<DEPTH>(triple),
        )),
    }
}

/// The heap fold's owned reconstruction of one triple from its folded children.
#[inline]
fn rebuild_clone(
    original: &RdfTriple,
    subject: RdfTerm,
    predicate: String,
    object: RdfTerm,
) -> RdfTriple {
    RdfTriple {
        subject,
        predicate,
        object,
        location: original.location.clone(),
    }
}

/// Fold both children and the predicate, retaining each original node's location.
#[inline(never)]
fn clone_nested(term: &RdfTerm) -> RdfTerm {
    #[derive(Clone, Copy)]
    enum Node<'a> {
        Term(&'a RdfTerm),
        Predicate(&'a str),
    }

    let result: Result<RdfTerm, Infallible> = crate::try_fold_nested(
        Node::Term(term),
        &mut (),
        |(), node| {
            Ok(match node {
                Node::Term(RdfTerm::Triple(triple)) => crate::Nested::Triple(
                    Node::Term(&triple.subject),
                    Node::Predicate(&triple.predicate),
                    Node::Term(&triple.object),
                ),
                // A proven leaf cannot restart quoted-triple cloning.
                Node::Term(leaf) => crate::Nested::Leaf(leaf.clone()),
                // Node distinguishes a predicate from an authored IRI term.
                Node::Predicate(iri) => crate::Nested::Leaf(RdfTerm::Iri(iri.to_owned())),
            })
        },
        |(), original, subject, predicate, object| {
            let Node::Term(RdfTerm::Triple(original)) = original else {
                unreachable!("only a triple node is assembled")
            };
            let RdfTerm::Iri(predicate) = predicate else {
                unreachable!("the predicate answer is its copied IRI")
            };
            Ok(RdfTerm::Triple(Box::write(
                Box::new_uninit(),
                rebuild_clone(original, subject, predicate, object),
            )))
        },
    );
    result.unwrap_or_else(|never| match never {})
}

/// The one local quoted-triple field comparison for bounded and heap equality.
#[inline]
fn triple_fields_eq(a: &RdfTriple, b: &RdfTriple) -> bool {
    a.predicate == b.predicate && a.location == b.location
}

impl PartialEq for RdfTriple {
    /// Compare local fields and paired children without input-dependent recursion.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        eq_root(self, other)
    }
}

impl Eq for RdfTriple {}

/// Keep the unrolled tree prefix outside the derived term's small dispatch.
#[inline(never)]
fn eq_root(a: &RdfTriple, b: &RdfTriple) -> bool {
    eq_at_depth::<DIRECT_LEVELS>(a, b)
}

#[allow(
    clippy::inline_always,
    reason = "the fixed compile-time depth removes the prewalk and recursive derive dispatch"
)]
#[inline(always)]
fn eq_at_depth<const DEPTH: u8>(a: &RdfTriple, b: &RdfTriple) -> bool {
    let eq_child: fn(&RdfTerm, &RdfTerm) -> bool = match DEPTH {
        4 => eq_term_at_depth::<3>,
        3 => eq_term_at_depth::<2>,
        2 => eq_term_at_depth::<1>,
        _ => eq_term_at_depth::<0>,
    };
    triple_fields_eq(a, b) && eq_child(&a.subject, &b.subject) && eq_child(&a.object, &b.object)
}

#[allow(
    clippy::inline_always,
    reason = "only the literal bounded instantiations call each other before the heap walk"
)]
#[inline(always)]
fn eq_term_at_depth<const DEPTH: u8>(a: &RdfTerm, b: &RdfTerm) -> bool {
    let (RdfTerm::Triple(left), RdfTerm::Triple(right)) = (a, b) else {
        // A leaf or discriminant mismatch cannot reenter paired Triple descent.
        return a == b;
    };
    if DEPTH == 0 {
        eq_nested(left, right)
    } else {
        eq_at_depth::<DEPTH>(left, right)
    }
}

/// Compare only paired triples on the work list; leaves retain their payload equality.
#[inline(never)]
fn eq_nested(a: &RdfTriple, b: &RdfTriple) -> bool {
    let mut pending: WorkList<(&RdfTriple, &RdfTriple), 16> = WorkList::with((a, b));
    while let Some((a, b)) = pending.pop() {
        if !triple_fields_eq(a, b) {
            return false;
        }
        for (a, b) in [(&a.subject, &b.subject), (&a.object, &b.object)] {
            if let (RdfTerm::Triple(a), RdfTerm::Triple(b)) = (a, b) {
                pending.push((a, b));
            } else if a != b {
                // Parent dispatch sees a leaf pair or refuses different variants.
                return false;
            }
        }
    }
    true
}

#[derive(Clone, Copy)]
enum HashField<'a> {
    Term(&'a RdfTerm),
    Predicate(&'a String),
    Location(&'a Option<crate::RdfLocation>),
}

/// The original struct's one ordered field script for bounded and heap Hash.
#[inline]
fn hash_fields<'a>(triple: &'a RdfTriple, mut visit: impl FnMut(HashField<'a>)) {
    visit(HashField::Term(&triple.subject));
    visit(HashField::Predicate(&triple.predicate));
    visit(HashField::Term(&triple.object));
    visit(HashField::Location(&triple.location));
}

/// Feed metadata directly; leave a term to the caller's stated traversal boundary.
#[inline]
fn hash_field<'a, H: Hasher>(field: HashField<'a>, state: &mut H) -> Option<&'a RdfTerm> {
    match field {
        HashField::Term(term) => return Some(term),
        HashField::Predicate(predicate) => predicate.hash(state),
        HashField::Location(location) => location.hash(state),
    }
    None
}

/// Emit one term tag and its leaf payload; leave quoted fields to the caller.
#[inline]
fn hash_term_payload<'a, H: Hasher>(term: &'a RdfTerm, state: &mut H) -> Option<&'a RdfTriple> {
    core::mem::discriminant(term).hash(state);
    match term {
        RdfTerm::Iri(value) | RdfTerm::BlankNode(value) => value.hash(state),
        RdfTerm::Literal(value) => value.hash(state),
        RdfTerm::Triple(triple) => return Some(triple),
    }
    None
}

impl Hash for RdfTriple {
    /// Feed the original struct fields; the outer term already fed its tag.
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_root(self, state);
    }
}

/// Keep the unrolled tree prefix outside the term's small dispatch.
#[inline(never)]
fn hash_root<H: Hasher>(triple: &RdfTriple, state: &mut H) {
    hash_at_depth::<DIRECT_LEVELS, H>(triple, state);
}

#[allow(
    clippy::inline_always,
    reason = "the fixed compile-time depth removes the prewalk and recursive derive dispatch"
)]
#[inline(always)]
fn hash_at_depth<const DEPTH: u8, H: Hasher>(triple: &RdfTriple, state: &mut H) {
    let hash_child: fn(&RdfTerm, &mut H) = match DEPTH {
        4 => hash_term_at_depth::<3, H>,
        3 => hash_term_at_depth::<2, H>,
        2 => hash_term_at_depth::<1, H>,
        _ => hash_term_at_depth::<0, H>,
    };
    hash_fields(triple, |field| {
        if let Some(term) = hash_field(field, state) {
            hash_child(term, state);
        }
    });
}

#[allow(
    clippy::inline_always,
    reason = "only the literal bounded instantiations call each other before the heap walk"
)]
#[inline(always)]
fn hash_term_at_depth<const DEPTH: u8, H: Hasher>(term: &RdfTerm, state: &mut H) {
    if let Some(triple) = hash_term_payload(term, state) {
        if DEPTH == 0 {
            hash_nested(triple, state);
        } else {
            hash_at_depth::<DEPTH, H>(triple, state);
        }
    }
}

#[inline(never)]
fn hash_nested<H: Hasher>(root: &RdfTriple, state: &mut H) {
    enum Node<'a> {
        Triple(&'a RdfTriple),
        Field(HashField<'a>),
    }

    let mut pending: WorkList<Node<'_>, 16> = WorkList::with(Node::Triple(root));
    while let Some(node) = pending.pop() {
        match node {
            Node::Triple(triple) => {
                let mut fields: WorkList<HashField<'_>, 4> = WorkList::new();
                hash_fields(triple, |field| fields.push(field));
                while let Some(field) = fields.pop() {
                    pending.push(Node::Field(field));
                }
            }
            Node::Field(field) => {
                if let Some(term) = hash_field(field, state)
                    && let Some(triple) = hash_term_payload(term, state)
                {
                    pending.push(Node::Triple(triple));
                }
            }
        }
    }
}

/// A triple printed by the standard builders, exactly as the derive prints it,
/// with `levels` quoted-triple levels left before its subtree is handed to the
/// heap writer. The recursion is bounded by [`DIRECT_LEVELS`], never by input.
struct Bounded<'a> {
    triple: &'a RdfTriple,
    levels: u8,
}

/// A triple's subject or object under a [`Bounded`] parent.
struct BoundedTerm<'a> {
    term: &'a RdfTerm,
    levels: u8,
}

impl fmt::Debug for Bounded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let child = |term| BoundedTerm {
            term,
            levels: self.levels,
        };
        f.debug_struct("RdfTriple")
            .field("subject", &child(&self.triple.subject))
            .field("predicate", &self.triple.predicate)
            .field("object", &child(&self.triple.object))
            .field("location", &self.triple.location)
            .finish()
    }
}

impl fmt::Debug for BoundedTerm<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let RdfTerm::Triple(triple) = self.term else {
            // A leaf runs its own derived formatter and cannot reenter Triple.
            return fmt::Debug::fmt(self.term, f);
        };
        if self.levels > 1 {
            f.debug_tuple("Triple")
                .field(&Bounded {
                    triple,
                    levels: self.levels - 1,
                })
                .finish()
        } else {
            f.debug_tuple("Triple").field(&Nested(triple)).finish()
        }
    }
}

/// A subtree past the directly printed levels, written by the heap writer at
/// the formatter position the builders reached.
struct Nested<'a>(&'a RdfTriple);

impl fmt::Debug for Nested<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_nested(self.0, f)
    }
}

#[derive(Clone, Copy)]
enum DebugNode<'a> {
    Term(&'a RdfTerm),
    Triple(&'a RdfTriple),
    Literal(&'a RdfLiteral),
    Location(&'a crate::RdfLocation),
    OptionalLocation(Option<&'a crate::RdfLocation>),
    OptionalScalar(Option<DebugScalar<'a>>),
    OptionalDirection(Option<RdfTextDirection>),
}

type DebugTok<'a> = Tok<DebugNode<'a>, DebugScalar<'a>>;

/// The one struct-field script; the derived enum supplies the outer tuple itself.
fn triple_debug_script<'a>(triple: &'a RdfTriple, out: &mut WorkList<DebugTok<'a>, 16>) {
    out.extend([
        Tok::Struct("RdfTriple"),
        Tok::Field("subject"),
        Tok::Node(DebugNode::Term(&triple.subject)),
        Tok::Field("predicate"),
        Tok::Leaf(DebugScalar::Str(&triple.predicate)),
        Tok::Field("object"),
        Tok::Node(DebugNode::Term(&triple.object)),
        Tok::Field("location"),
        Tok::Node(DebugNode::OptionalLocation(triple.location.as_ref())),
        Tok::EndStruct,
    ]);
}

/// All literal fields, in the original derive's declaration order.
fn literal_debug_script<'a>(literal: &'a RdfLiteral, out: &mut WorkList<DebugTok<'a>, 16>) {
    out.extend([
        Tok::Struct("RdfLiteral"),
        Tok::Field("lexical_form"),
        Tok::Leaf(DebugScalar::Str(&literal.lexical_form)),
        Tok::Field("datatype"),
        Tok::Node(DebugNode::OptionalScalar(
            literal.datatype.as_deref().map(DebugScalar::Str),
        )),
        Tok::Field("language"),
        Tok::Node(DebugNode::OptionalScalar(
            literal.language.as_deref().map(DebugScalar::Str),
        )),
        Tok::Field("direction"),
        Tok::Node(DebugNode::OptionalDirection(literal.direction)),
        Tok::EndStruct,
    ]);
}

/// All ten authored location fields, retaining both integer widths and Options.
fn location_debug_script<'a>(
    location: &'a crate::RdfLocation,
    out: &mut WorkList<DebugTok<'a>, 16>,
) {
    out.extend([
        Tok::Struct("RdfLocation"),
        Tok::Field("path"),
        Tok::Node(DebugNode::OptionalScalar(
            location.path.as_deref().map(DebugScalar::Str),
        )),
        Tok::Field("line"),
        Tok::Node(DebugNode::OptionalScalar(
            location.line.map(DebugScalar::U64),
        )),
        Tok::Field("column"),
        Tok::Node(DebugNode::OptionalScalar(
            location.column.map(DebugScalar::U32),
        )),
        Tok::Field("logical"),
        Tok::Node(DebugNode::OptionalScalar(
            location.logical.as_deref().map(DebugScalar::Str),
        )),
        Tok::Field("subject"),
        Tok::Node(DebugNode::OptionalScalar(
            location.subject.as_deref().map(DebugScalar::Str),
        )),
        Tok::Field("gts_term_id"),
        Tok::Node(DebugNode::OptionalScalar(
            location.gts_term_id.map(DebugScalar::U64),
        )),
        Tok::Field("gts_quad_index"),
        Tok::Node(DebugNode::OptionalScalar(
            location.gts_quad_index.map(DebugScalar::U64),
        )),
        Tok::Field("gts_reifier_id"),
        Tok::Node(DebugNode::OptionalScalar(
            location.gts_reifier_id.map(DebugScalar::U64),
        )),
        Tok::Field("gts_frame_index"),
        Tok::Node(DebugNode::OptionalScalar(
            location.gts_frame_index.map(DebugScalar::U64),
        )),
        Tok::Field("gts_segment_index"),
        Tok::Node(DebugNode::OptionalScalar(
            location.gts_segment_index.map(DebugScalar::U64),
        )),
        Tok::EndStruct,
    ]);
}

/// One Some/None container law for primitive, location and direction fields.
fn optional_debug_script<'a>(value: Option<DebugTok<'a>>, out: &mut WorkList<DebugTok<'a>, 16>) {
    if let Some(value) = value {
        out.extend([Tok::Tuple("Some"), value, Tok::EndTuple]);
    } else {
        out.push(Tok::Unit("None"));
    }
}

/// Expand every composite without calling its public derive inside the heap walk.
fn debug_script<'a>(node: DebugNode<'a>, out: &mut WorkList<DebugTok<'a>, 16>) {
    match node {
        DebugNode::Triple(triple) => triple_debug_script(triple, out),
        DebugNode::Literal(literal) => literal_debug_script(literal, out),
        DebugNode::Location(location) => location_debug_script(location, out),
        DebugNode::OptionalLocation(location) => optional_debug_script(
            location.map(|location| Tok::Node(DebugNode::Location(location))),
            out,
        ),
        DebugNode::OptionalScalar(value) => optional_debug_script(value.map(Tok::Leaf), out),
        DebugNode::OptionalDirection(direction) => optional_debug_script(
            direction.map(|direction| {
                Tok::Unit(match direction {
                    RdfTextDirection::Ltr => "Ltr",
                    RdfTextDirection::Rtl => "Rtl",
                })
            }),
            out,
        ),
        DebugNode::Term(term) => match term {
            RdfTerm::Iri(value) => out.extend([
                Tok::Tuple("Iri"),
                Tok::Leaf(DebugScalar::Str(value)),
                Tok::EndTuple,
            ]),
            RdfTerm::BlankNode(value) => out.extend([
                Tok::Tuple("BlankNode"),
                Tok::Leaf(DebugScalar::Str(value)),
                Tok::EndTuple,
            ]),
            RdfTerm::Literal(value) => out.extend([
                Tok::Tuple("Literal"),
                Tok::Node(DebugNode::Literal(value)),
                Tok::EndTuple,
            ]),
            RdfTerm::Triple(triple) => out.extend([
                Tok::Tuple("Triple"),
                Tok::Node(DebugNode::Triple(triple)),
                Tok::EndTuple,
            ]),
        },
    }
}

impl fmt::Debug for RdfTriple {
    /// Preserve compact/alternate bytes and formatter errors at the struct boundary.
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_root(self, f)
    }
}

/// Dispatch bounded nodes to the derive and deeper nodes to its outlined writer.
/// Each derived level re-enters here for its quoted children, so the derive's
/// recursion is bounded by [`DIRECT_LEVELS`] as well.
#[inline]
fn debug_root(triple: &RdfTriple, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(
        &Bounded {
            triple,
            levels: DIRECT_LEVELS,
        },
        f,
    )
}

/// Keep the iterative writer outside the inline shallow dispatch.
#[inline(never)]
fn debug_nested(triple: &RdfTriple, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write_debug_scalars(f, DebugNode::Triple(triple), debug_script)
}

/// The leaf a field is left holding once its term has been moved out: an empty
/// IRI, whose drop allocates and frees nothing.
pub(super) const PLACEHOLDER: RdfTerm = RdfTerm::Iri(String::new());

impl Drop for RdfTriple {
    /// A triple whose subject and object are both leaves falls through to the
    /// plain field drops after two discriminant reads. A quoted child is moved
    /// out and dropped by direct calls for the levels the other traits walk
    /// directly, then by the heap walk; no glue ever meets a nested triple.
    #[inline]
    fn drop(&mut self) {
        if matches!(self.subject, RdfTerm::Triple(_)) || matches!(self.object, RdfTerm::Triple(_)) {
            drop_children::<{ DIRECT_LEVELS - 1 }>(self);
        }
    }
}

/// Move each quoted child of `triple` out and drop it with `LEVELS` direct
/// levels left, so the glue that later drops `triple` meets only leaves.
#[inline(never)]
fn drop_children<const LEVELS: u8>(triple: &mut RdfTriple) {
    let drop_child: fn(Box<RdfTriple>) = match LEVELS {
        3 => drop_at::<2>,
        2 => drop_at::<1>,
        1 => drop_at::<0>,
        _ => drop_nested,
    };
    for term in [&mut triple.subject, &mut triple.object] {
        if matches!(term, RdfTerm::Triple(_))
            && let RdfTerm::Triple(child) = core::mem::replace(term, PLACEHOLDER)
        {
            drop_child(child);
        }
    }
}

#[allow(
    clippy::inline_always,
    reason = "only the literal bounded instantiations call each other before the heap walk"
)]
#[inline(always)]
fn drop_at<const LEVELS: u8>(mut triple: Box<RdfTriple>) {
    if matches!(triple.subject, RdfTerm::Triple(_)) || matches!(triple.object, RdfTerm::Triple(_)) {
        drop_children::<LEVELS>(&mut triple);
    }
    // Its children are leaves now, so its own drop returns at once.
    drop(triple);
}

/// Move every quoted triple below `root` onto a work list and drop each once
/// its own quoted children are gone, so no drop glue ever finds a nested one.
#[inline(never)]
fn drop_nested(mut root: Box<RdfTriple>) {
    fn detach(term: &mut RdfTerm, pending: &mut WorkList<Box<RdfTriple>, 16>) {
        if matches!(term, RdfTerm::Triple(_))
            && let RdfTerm::Triple(triple) = core::mem::replace(term, PLACEHOLDER)
        {
            pending.push(triple);
        }
    }
    let mut pending = WorkList::new();
    detach(&mut root.subject, &mut pending);
    detach(&mut root.object, &mut pending);
    drop(root);
    while let Some(mut triple) = pending.pop() {
        detach(&mut triple.subject, &mut pending);
        detach(&mut triple.object, &mut pending);
        // Its children are leaves now, so its own drop returns at once.
        drop(triple);
    }
}

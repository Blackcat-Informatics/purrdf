// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The owned model's value traits, with quoted-triple depth kept off the stack.

use core::convert::Infallible;
use core::fmt;
use core::hash::{Hash, Hasher};

use purrdf_lex::walk::{Tok, WorkList, write_debug};

use super::{RdfTerm, RdfTriple};

/// Whether a triple's children are leaves: derived struct traits are then bounded.
#[inline]
fn is_flat(triple: &RdfTriple) -> bool {
    !matches!(triple.subject, RdfTerm::Triple(_)) && !matches!(triple.object, RdfTerm::Triple(_))
}

/// Bound direct derive calls to two quoted-triple levels, the shallow bench's
/// branching shape. A deeper node always switches to the heap walk.
#[inline]
fn is_shallow(triple: &RdfTriple) -> bool {
    let child_is_flat = |term: &RdfTerm| match term {
        RdfTerm::Triple(child) => is_flat(child),
        _ => true,
    };
    child_is_flat(&triple.subject) && child_is_flat(&triple.object)
}

/// Copy one leaf, without reentering the recursive trait implementation.
#[inline]
fn clone_leaf(term: &RdfTerm) -> RdfTerm {
    match term {
        RdfTerm::Iri(value) => RdfTerm::Iri(value.clone()),
        RdfTerm::BlankNode(value) => RdfTerm::BlankNode(value.clone()),
        RdfTerm::Literal(value) => RdfTerm::Literal(value.clone()),
        RdfTerm::Triple(_) => unreachable!("quoted triples are reconstructed by the fold"),
    }
}

impl Clone for RdfTerm {
    /// Copy bounded shallow trees directly and reconstruct deeper trees by folding.
    #[inline]
    fn clone(&self) -> Self {
        let Self::Triple(triple) = self else {
            return clone_leaf(self);
        };
        if is_shallow(triple) {
            return Self::Triple(Box::new((**triple).clone()));
        }
        clone_nested(self)
    }
}

/// Fold both children and the predicate, retaining each original node's location.
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
                Node::Term(leaf) => crate::Nested::Leaf(clone_leaf(leaf)),
                // The fold's predicate answer uses the same owned string carrier;
                // Node keeps it distinct from an authored IRI term.
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
            Ok(RdfTerm::Triple(Box::new(RdfTriple {
                subject,
                predicate,
                object,
                location: original.location.clone(),
            })))
        },
    );
    result.unwrap_or_else(|never| match never {})
}

/// Equal variants and local fields; nested subjects and objects are visited separately.
#[inline]
fn shallow_eq(a: &RdfTerm, b: &RdfTerm) -> bool {
    match (a, b) {
        (RdfTerm::Iri(a), RdfTerm::Iri(b)) | (RdfTerm::BlankNode(a), RdfTerm::BlankNode(b)) => {
            a == b
        }
        (RdfTerm::Literal(a), RdfTerm::Literal(b)) => a == b,
        (RdfTerm::Triple(a), RdfTerm::Triple(b)) => {
            a.predicate == b.predicate && a.location == b.location
        }
        _ => false,
    }
}

impl PartialEq for RdfTerm {
    /// Compare authored fields exactly, with deeper child pairs on a heap work list.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Iri(a), Self::Iri(b)) | (Self::BlankNode(a), Self::BlankNode(b)) => a == b,
            (Self::Literal(a), Self::Literal(b)) => a == b,
            (Self::Triple(a), Self::Triple(b)) if is_shallow(a) && is_shallow(b) => a == b,
            (Self::Triple(_), Self::Triple(_)) => eq_nested(self, other),
            _ => false,
        }
    }
}

/// Compare the local fields and paired children of two deeper trees.
fn eq_nested(a: &RdfTerm, b: &RdfTerm) -> bool {
    let mut pending: WorkList<(&RdfTerm, &RdfTerm), 16> = WorkList::with((a, b));
    while let Some((a, b)) = pending.pop() {
        if !shallow_eq(a, b) {
            return false;
        }
        if let (RdfTerm::Triple(a), RdfTerm::Triple(b)) = (a, b) {
            pending.extend([(&a.object, &b.object), (&a.subject, &b.subject)]);
        }
    }
    true
}

impl Eq for RdfTerm {}

/// Feed exactly the derive's discriminant and leaf events; return unvisited triple fields.
#[inline]
fn hash_header<'a, H: Hasher>(term: &'a RdfTerm, state: &mut H) -> Option<&'a RdfTriple> {
    // The published model used the compiler's discriminant Hash, not the IR's
    // explicit u8 tags. Preserve that actual event kind and value.
    match term {
        RdfTerm::Iri(value) => {
            core::mem::discriminant(term).hash(state);
            value.hash(state);
        }
        RdfTerm::BlankNode(value) => {
            core::mem::discriminant(term).hash(state);
            value.hash(state);
        }
        RdfTerm::Literal(value) => {
            core::mem::discriminant(term).hash(state);
            value.hash(state);
        }
        RdfTerm::Triple(triple) => {
            core::mem::discriminant(term).hash(state);
            return Some(triple);
        }
    }
    None
}

impl Hash for RdfTerm {
    /// Preserve the derive's ordered observations on both bounded and heap paths.
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Some(triple) = hash_header(self, state) else {
            return;
        };
        if is_shallow(triple) {
            triple.hash(state);
            return;
        }
        hash_nested(triple, state);
    }
}

/// Feed the already tagged root's subject, predicate, object and location in order.
fn hash_nested<H: Hasher>(root: &RdfTriple, state: &mut H) {
    enum Node<'a> {
        Term(&'a RdfTerm),
        Triple(&'a RdfTriple),
        Predicate(&'a String),
        Location(&'a Option<crate::RdfLocation>),
    }
    let mut pending: WorkList<Node<'_>, 16> = WorkList::with(Node::Triple(root));
    while let Some(node) = pending.pop() {
        match node {
            Node::Term(term) => {
                if let Some(triple) = hash_header(term, state) {
                    if is_shallow(triple) {
                        triple.hash(state);
                    } else {
                        pending.push(Node::Triple(triple));
                    }
                }
            }
            Node::Triple(triple) => pending.extend([
                Node::Location(&triple.location),
                Node::Term(&triple.object),
                Node::Predicate(&triple.predicate),
                Node::Term(&triple.subject),
            ]),
            Node::Predicate(predicate) => predicate.hash(state),
            Node::Location(location) => location.hash(state),
        }
    }
}

type DebugTok<'a> = Tok<&'a RdfTerm, &'a dyn fmt::Debug>;

/// The compiler-derived tuple/struct spelling, expanded by the shared Debug writer.
fn debug_script<'a>(term: &'a RdfTerm, out: &mut WorkList<DebugTok<'a>, 16>) {
    match term {
        RdfTerm::Iri(value) => out.extend([
            Tok::Tuple("Iri"),
            Tok::Leaf(value as &dyn fmt::Debug),
            Tok::EndTuple,
        ]),
        RdfTerm::BlankNode(value) => {
            out.extend([
                Tok::Tuple("BlankNode"),
                Tok::Leaf(value as &dyn fmt::Debug),
                Tok::EndTuple,
            ]);
        }
        RdfTerm::Literal(value) => out.extend([
            Tok::Tuple("Literal"),
            Tok::Leaf(value as &dyn fmt::Debug),
            Tok::EndTuple,
        ]),
        RdfTerm::Triple(triple) => out.extend([
            Tok::Tuple("Triple"),
            Tok::Struct("RdfTriple"),
            Tok::Field("subject"),
            Tok::Node(&triple.subject),
            Tok::Field("predicate"),
            Tok::Leaf(&triple.predicate as &dyn fmt::Debug),
            Tok::Field("object"),
            Tok::Node(&triple.object),
            Tok::Field("location"),
            Tok::Leaf(&triple.location as &dyn fmt::Debug),
            Tok::EndStruct,
            Tok::EndTuple,
        ]),
    }
}

impl fmt::Debug for RdfTerm {
    /// Write the derive's compact or alternate bytes, expanding deeper nodes iteratively.
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Iri(value) => f.debug_tuple("Iri").field(value).finish(),
            Self::BlankNode(value) => f.debug_tuple("BlankNode").field(value).finish(),
            Self::Literal(value) => f.debug_tuple("Literal").field(value).finish(),
            Self::Triple(triple) if is_shallow(triple) => {
                f.debug_tuple("Triple").field(triple).finish()
            }
            Self::Triple(_) => write_debug(f, self, debug_script),
        }
    }
}

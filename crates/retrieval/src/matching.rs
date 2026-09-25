// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One matching rule and one placement rule, shared by the planner and the
//! compiler.
//!
//! [`plan`](crate::plan) decides which producers a request reaches;
//! [`compile`](crate::compile) re-derives that same decision over a plan it must
//! treat as untrusted input. Admission exists precisely so the second derivation
//! can disagree with a hand-edited plan — which only works if both derivations
//! run the *same* rule. Two copies of "does this pattern accept this term" would
//! be a latent divergence that no test on either side could see, so the rule
//! lives here once and both stages call it.
//!
//! # Matching is a lookup, placement is a rendering
//!
//! [`pattern_matches`] answers *whether* a producer takes a request term, by
//! field equality over the declaration. [`place`] answers *where each facet of
//! that term goes* and *what constant it becomes*, by reading the same
//! declaration's [`TermPlacement`]s. The two halves are paired in the registry's
//! [`AcceptedTerm`](purrdf_sparql_eval::AcceptedTerm) so a reader cannot mis-align
//! them, and they are paired here for the same reason.
//!
//! # Matching is not receiving
//!
//! An alternative can match a term and declare no placement for it. That is a
//! producer saying "call me for this request, but write none of it into my
//! arguments" — a legitimate declaration, and one whose invocation carries no
//! trace of the term. [`carries_content`] is the third question, asked between
//! the other two: does the alternative that matched place anything? A term it
//! does not is not bound to that producer, because binding it would record a
//! term as served while transporting nothing.
//!
//! # The `Value`/`Language` interaction, stated once
//!
//! A lexical request term is a needle *and*, sometimes, a language tag. Where
//! both ride matters:
//!
//! * If the accepted alternative declares **no** [`RequestFacet::Language`]
//!   placement, a tagged needle is rendered as the language-tagged literal
//!   `"needle"@en`, because the tag is part of what the caller asked for and
//!   dropping it would silently widen the question.
//! * If the alternative **does** declare a `Language` placement, that position
//!   carries the tag, and the `Value` position renders the plain string
//!   `"needle"`.
//!
//! These are different queries — `"fox"@en` matches only English-tagged objects,
//! `"fox"` matches only the plain string — so the choice is made by the
//! producer's own declaration and never by a default. The rule is pinned by
//! `lexical_value_carries_the_tag_only_when_no_language_placement_does`.

use purrdf_core::TermValue;
use purrdf_core::binding_pattern::BindingPattern;
use purrdf_sparql_eval::{
    PfDescriptor, RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement,
};

use crate::embedding::encode_embedding;
use crate::render::{self, RDF_LANG_STRING, XSD_STRING};
use crate::request::RequestTerm;

/// What one flattened argument position of an invocation holds.
///
/// Three states rather than two, and the third is the point: a producer that declared
/// a [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement) has that position
/// **occupied** — which is what decides the invocation's access mode — while the
/// number in it belongs to the unit and is rendered on every read of the unit's text.
/// Represented as an occupied slot with no value, the two facts stop being one: the
/// mode is derivable without a depth, and the depth is rendered without a second
/// spelling of which position it goes in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    /// Nothing was placed here: the call renders the free variable `?cN`.
    Free,
    /// A request facet's constant, proved renderable by [`place`].
    Placed(TermValue),
    /// The per-stratum depth's own position, carrying the literal datatype its
    /// producer declared for it and no value.
    Depth {
        /// The literal datatype IRI the producer declared for its depth argument.
        datatype: String,
    },
}

impl Slot {
    /// Whether this position is occupied, which is what the access mode is read
    /// from.
    const fn is_occupied(&self) -> bool {
        !matches!(*self, Self::Free)
    }
}

/// One producer invocation, rendered from the request: what each flattened
/// argument position holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    /// Flattened position -> what that position holds.
    pub(crate) slots: Vec<Slot>,
    /// The access pattern the invocation actually has: bit `p` set iff
    /// `slots[p]` is occupied.
    ///
    /// This is the mode the evaluator will compute for the emitted call, and
    /// therefore the mode the producer's own `rows_per_invocation` is read at —
    /// see [`crate::admission::declared_row_bound`].
    pub(crate) mode: BindingPattern,
}

/// Why a producer cannot be invoked for a request.
///
/// Every variant is a refusal to emit a call that would answer a different
/// question than the one the caller asked. A producer that trips one is not
/// bound by the planner and is not emitted by the compiler; it is never bound
/// with the offending facet silently dropped.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PlacementError {
    /// No alternative the producer accepts matches the request term.
    ///
    /// A term index the request does not carry also lands here: a term that is
    /// not in the request has, trivially, no accepted alternative.
    #[error("request term {term_index} matches no accepted alternative")]
    NoAcceptedAlternative {
        /// The index into the plan's request terms.
        term_index: u32,
    },

    /// The matched alternative renders a facet the request term does not carry.
    #[error("request term {term_index} carries no {} facet", facet.as_str())]
    MissingFacet {
        /// The index into the plan's request terms.
        term_index: u32,
        /// The facet the declaration asked for.
        facet: RequestFacet,
    },

    /// The facet exists but has no SPARQL constant form.
    #[error("request term {term_index}'s {} facet cannot be rendered: {reason}", facet.as_str())]
    Unrenderable {
        /// The index into the plan's request terms.
        term_index: u32,
        /// The facet that could not be written.
        facet: RequestFacet,
        /// Why it could not be written.
        reason: String,
    },

    /// Two placements target one position with different values, or a declared
    /// position lies outside the relation's arity, so nothing can occupy it.
    #[error("argument position {position} cannot hold every value placed in it")]
    PositionConflict {
        /// The contested flattened argument position.
        position: usize,
    },

    /// No declared access pattern is general enough to serve the invocation.
    #[error("invocation mode {invocation} is served by none of the declared modes {declared:?}")]
    NoSatisfiableMode {
        /// The invocation's own per-position code.
        invocation: String,
        /// Every mode the relation declares.
        declared: Vec<String>,
    },
}

impl PlacementError {
    /// The stable name of the rule that refused.
    ///
    /// The spelling is part of the diagnostic contract, exactly as
    /// [`AdmissionError::dimension`](crate::AdmissionError::dimension) is: it is
    /// what a caller switches on and a report quotes.
    pub(crate) const fn rule(&self) -> &'static str {
        match self {
            Self::NoAcceptedAlternative { .. } => "no_accepted_alternative",
            Self::MissingFacet { .. } => "missing_facet",
            Self::Unrenderable { .. } => "unrenderable",
            Self::PositionConflict { .. } => "position_conflict",
            Self::NoSatisfiableMode { .. } => "no_satisfiable_mode",
        }
    }

    /// The invocation's access-pattern code, when the refusal derived one.
    pub(crate) fn invocation(&self) -> Option<String> {
        match self {
            Self::NoSatisfiableMode { invocation, .. } => Some(invocation.clone()),
            _ => None,
        }
    }

    /// Every access pattern the relation declared, when the refusal read them.
    pub(crate) fn declared(&self) -> Vec<String> {
        match self {
            Self::NoSatisfiableMode { declared, .. } => declared.clone(),
            _ => Vec::new(),
        }
    }
}

/// Whether the producer's declared pattern accepts the request term.
pub(crate) fn pattern_matches(pattern: &TermPattern, term: &RequestTerm) -> bool {
    if !kind_matches(pattern.kind, term) {
        return false;
    }
    // No request term carries a datatype, so a datatype constraint matches nothing.
    if pattern.datatype.is_some() {
        return false;
    }
    if let Some(language) = &pattern.language {
        match term {
            RequestTerm::Lexical {
                language: Some(term_language),
                ..
            } if term_language == language => {}
            _ => return false,
        }
    }
    if let Some(predicate) = &pattern.predicate {
        let matches = match term {
            RequestTerm::Lexical {
                predicate: Some(term_predicate),
                ..
            } => term_predicate.as_str() == predicate,
            RequestTerm::Spatial {
                predicate: term_predicate,
                ..
            }
            | RequestTerm::Temporal {
                predicate: term_predicate,
                ..
            }
            | RequestTerm::NumericRange {
                predicate: term_predicate,
                ..
            } => term_predicate.as_str() == predicate,
            _ => false,
        };
        if !matches {
            return false;
        }
    }
    true
}

/// Whether a declared term kind accepts a request term.
pub(crate) fn kind_matches(kind: TermKind, term: &RequestTerm) -> bool {
    if kind == TermKind::Any {
        return true;
    }
    match term {
        // Every one of these renders as a literal, which is the whole of what a
        // declared kind classifies: an interval's endpoints, a needle, a
        // geometry and a query embedding are all written as literals, so a
        // literal-accepting producer is reachable by any of them. What each one
        // then *needs* of the placement differs — an embedding and a geometry
        // need the producer's declared datatype, an interval needs two endpoint
        // positions rather than one value position — and that is decided at
        // placement, by the producer's own declaration, not here.
        RequestTerm::Lexical { .. }
        | RequestTerm::Spatial { .. }
        | RequestTerm::Temporal { .. }
        | RequestTerm::NumericRange { .. }
        | RequestTerm::Vector { .. } => kind == TermKind::Literal,
        RequestTerm::EntitySeed { entity } => seed_kind(entity.as_str()) == Some(kind),
    }
}

/// Whether the alternative `term` matches places any facet of it at all.
///
/// This is the difference between a producer that *matches* a request term and
/// one that *receives* it. An [`AcceptedTerm`](purrdf_sparql_eval::AcceptedTerm)
/// whose `placements` list is empty accepts the shape and renders none of it, so
/// the emitted call carries no trace of the term: the producer is invoked, but
/// with the whole request absent from its arguments. Binding such a term would
/// record it as served while transporting nothing, which is exactly the claim
/// [`Plan::unserved_terms`](crate::Plan::unserved_terms) exists to keep honest —
/// so the planner routes it to
/// [`UnservedReason::AcceptedWithoutPlacement`](crate::UnservedReason::AcceptedWithoutPlacement)
/// instead.
///
/// The alternative consulted is the **first** whose pattern matches, because
/// that is the one [`place`] will apply. Reading any other would let the two
/// disagree about whether a term is carried.
pub(crate) fn carries_content(decl: &RankedDeclaration, term: &RequestTerm) -> bool {
    decl.accepted_terms
        .iter()
        .find(|accepted| pattern_matches(&accepted.pattern, term))
        .is_some_and(|accepted| !accepted.placements.is_empty())
}

/// The RDF term kind a canonical seed lexical names, when it names one.
///
/// The quoted-triple test precedes the IRI test, and the order is the whole
/// point: an RDF 1.2 triple term opens `<<`, so a test for `<` alone claims it
/// first and reports every quoted triple as an IRI. That is not a cosmetic
/// misnomer — it makes [`TermKind::Triple`] a kind no seed can ever have, so a
/// producer accepting only quoted triples is never selected, while one
/// accepting IRIs is handed a term it cannot read.
///
/// [`crate::render::decode_term`] discriminates in this same order, and the two
/// must agree: the decoder decides what a seed *is* and this decides what a
/// pattern may *match* it, so a disagreement selects a producer on one reading
/// and hands it a term built on the other.
pub(crate) fn seed_kind(text: &str) -> Option<TermKind> {
    if text.starts_with("<<") {
        Some(TermKind::Triple)
    } else if text.starts_with('<') {
        Some(TermKind::Iri)
    } else if text.starts_with("_:") {
        Some(TermKind::Blank)
    } else if text.starts_with('"') {
        Some(TermKind::Literal)
    } else {
        None
    }
}

/// Render `bound_indices` of `request_terms` into `producer`'s argument
/// positions, per its declaration.
///
/// The indices are consumed in **ascending** order, never in the order a plan
/// happens to list them: a hand-edited plan that reorders its binding indices
/// must not change the emitted text. Within one term, the alternative chosen is
/// the **first** accepted term whose pattern matches, and its placements are
/// applied in declaration order — caller order is identity-bearing and is never
/// sorted.
///
/// # No depth is needed, and that is what makes the bound knowable
///
/// This takes no depth, because the invocation's access mode does not depend on one.
/// A declared [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement) occupies its
/// position whatever number goes in it, the registry refuses a declaration that puts a
/// term placement in that same position
/// (`PropertyFunctionRegistry::register_ranked`), and the position's *rendered* text is
/// the unit's to write on every read of its query. So the mode is a function of the
/// declaration and the bound terms alone.
///
/// It used to take one, and the argument's only effect was to occupy that slot — the
/// text it produced was discarded at emission. Its cost was an ordering that looked
/// circular: the depth was derived from the registry's declared row bound, the bound
/// was wanted at the mode, and the mode came from here. Both the planner and the waist
/// resolved that by reading the bound at *every* declared mode and taking the maximum,
/// which is a number no invocation is ever made under. With no depth here the order is
/// the honest one — place, then read the declaration at the mode placement derived,
/// then choose the depth — and the planner's whole provisional depth pass, which
/// existed only to supply this argument, is gone with it.
///
/// # Errors
///
/// A [`PlacementError`] naming the exact rule that refused; see that type.
pub(crate) fn place(
    producer: &str,
    descriptor: &PfDescriptor,
    decl: &RankedDeclaration,
    request_terms: &[RequestTerm],
    bound_indices: &[u32],
) -> Result<Invocation, PlacementError> {
    let total = descriptor.subject_arity + descriptor.object_arity;
    if total > BindingPattern::MAX_ARITY {
        // The evaluator expresses an access pattern as a 64-bit set, so a
        // relation wider than that has no mode any invocation could satisfy.
        return Err(PlacementError::NoSatisfiableMode {
            invocation: format!(
                "<{total} positions, wider than the {} a binding pattern carries>",
                BindingPattern::MAX_ARITY
            ),
            declared: declared_modes(descriptor),
        });
    }
    let mut slots: Vec<Slot> = vec![Slot::Free; total];

    let mut indices: Vec<u32> = bound_indices.to_vec();
    indices.sort_unstable();
    indices.dedup();

    for term_index in indices {
        let term = request_terms
            .get(term_index as usize)
            .ok_or(PlacementError::NoAcceptedAlternative { term_index })?;
        let alternative = decl
            .accepted_terms
            .iter()
            .find(|accepted| pattern_matches(&accepted.pattern, term))
            .ok_or(PlacementError::NoAcceptedAlternative { term_index })?;
        // Whether a separate position already carries the tag decides whether the
        // needle rides tagged; see this module's header.
        let language_placed = alternative
            .placements
            .iter()
            .any(|placement| placement.facet == RequestFacet::Language);
        for placement in &alternative.placements {
            let value = facet_value(producer, term_index, term, placement, language_placed)?;
            // Proved renderable here, not at emission: the planner must refuse a
            // producer whose argument would have to be written as something the
            // grammar cannot spell (a blank node above all), and it refuses by
            // running the same check the compiler will.
            renderable(term_index, placement.facet, &value)?;
            occupy(&mut slots, placement.position, value)?;
        }
    }

    if let Some(depth_placement) = decl.depth_placement.as_ref() {
        // Occupied with the declared datatype and no value. The registry refuses a
        // declaration that also places a term facet here, so there is no rewrite for
        // `occupy` to reconcile — but the position can still be outside the relation's
        // arity under a registry that moved, which is the one refusal left.
        let slot =
            slots
                .get_mut(depth_placement.position)
                .ok_or(PlacementError::PositionConflict {
                    position: depth_placement.position,
                })?;
        *slot = Slot::Depth {
            datatype: depth_placement.datatype.clone(),
        };
    }

    let mode = BindingPattern::from_bools(slots.iter().map(Slot::is_occupied));
    if !descriptor
        .modes
        .iter()
        .any(|declared_mode| BindingPattern::from_code(&declared_mode.code).subsumes(mode))
    {
        return Err(PlacementError::NoSatisfiableMode {
            invocation: mode.code(),
            declared: declared_modes(descriptor),
        });
    }

    Ok(Invocation { slots, mode })
}

/// Every access-pattern code the relation declares, copied out for a refusal.
///
/// Read only where a [`PlacementError::NoSatisfiableMode`] is actually being
/// built: the codes are owned `String`s, and materializing them on the path
/// where the placement succeeds would allocate one per declared mode for a
/// diagnostic nobody reads.
fn declared_modes(descriptor: &PfDescriptor) -> Vec<String> {
    descriptor
        .modes
        .iter()
        .map(|mode| mode.code.clone())
        .collect()
}

/// Write `value` into `slots[position]`, accepting an identical re-write.
///
/// A second placement that writes the same value is not a conflict — two
/// alternatives can legitimately render the same constant into one position —
/// but a *different* value is, because only one of the two questions could then
/// be asked.
fn occupy(slots: &mut [Slot], position: usize, value: TermValue) -> Result<(), PlacementError> {
    let slot = slots
        .get_mut(position)
        .ok_or(PlacementError::PositionConflict { position })?;
    match slot {
        Slot::Placed(existing) if *existing == value => Ok(()),
        // A position the depth already holds cannot also hold a request value — one
        // argument renders one value, which is exactly what the registry asserts at
        // registration, so reaching this means the registry moved under the plan.
        Slot::Placed(_) | Slot::Depth { .. } => Err(PlacementError::PositionConflict { position }),
        Slot::Free => {
            *slot = Slot::Placed(value);
            Ok(())
        }
    }
}

/// The constant one placement renders from one request term.
fn facet_value(
    producer: &str,
    term_index: u32,
    term: &RequestTerm,
    placement: &TermPlacement,
    language_placed: bool,
) -> Result<TermValue, PlacementError> {
    let facet = placement.facet;
    match facet {
        RequestFacet::Value => value_facet(producer, term_index, term, placement, language_placed),
        RequestFacet::Language => match term {
            RequestTerm::Lexical {
                language: Some(tag),
                ..
            } => Ok(plain_or_typed(tag, placement)),
            _ => Err(PlacementError::MissingFacet { term_index, facet }),
        },
        RequestFacet::Predicate => match term {
            RequestTerm::Lexical {
                predicate: Some(predicate),
                ..
            } => Ok(TermValue::iri(predicate.as_str())),
            RequestTerm::Spatial { predicate, .. }
            | RequestTerm::Temporal { predicate, .. }
            | RequestTerm::NumericRange { predicate, .. } => Ok(TermValue::iri(predicate.as_str())),
            _ => Err(PlacementError::MissingFacet { term_index, facet }),
        },
        RequestFacet::LowerBound => {
            endpoint_facet(producer, term_index, term, placement, Endpoint::Lower)
        }
        RequestFacet::UpperBound => {
            endpoint_facet(producer, term_index, term, placement, Endpoint::Upper)
        }
        RequestFacet::MaxDistance => match term {
            RequestTerm::Spatial {
                max_distance: Some(distance),
                ..
            } => {
                let datatype = required_datatype(
                    producer,
                    term_index,
                    facet,
                    placement,
                    "a distance has no meaning without the unit its datatype names",
                )?;
                Ok(TermValue::Literal {
                    // Exact base-10, never a float: `to_decimal_lexical` reproduces
                    // the raw fixed-point integer with nothing lost.
                    lexical_form: distance.to_decimal_lexical(),
                    datatype,
                    language: None,
                    direction: None,
                })
            }
            _ => Err(PlacementError::MissingFacet { term_index, facet }),
        },
        // `RequestFacet` is `#[non_exhaustive]`: a facet this build does not know
        // how to render is refused by name rather than rendered as something else.
        other => Err(PlacementError::Unrenderable {
            term_index,
            facet: other,
            reason: format!(
                "producer {producer} declares facet {other:?}, which this build cannot render"
            ),
        }),
    }
}

/// The `Value` facet: the needle, the geometry, or the seed itself.
fn value_facet(
    producer: &str,
    term_index: u32,
    term: &RequestTerm,
    placement: &TermPlacement,
    language_placed: bool,
) -> Result<TermValue, PlacementError> {
    let facet = RequestFacet::Value;
    match term {
        RequestTerm::Lexical { text, language, .. } => {
            let tag = language.as_ref().filter(|_| !language_placed);
            let Some(tag) = tag else {
                return Ok(plain_or_typed(text, placement));
            };
            if placement.datatype.is_some() {
                return Err(PlacementError::Unrenderable {
                    term_index,
                    facet,
                    reason: format!(
                        "producer {producer} declares a datatype for a needle that carries \
                         the language tag {tag:?}; a literal is typed or language-tagged, \
                         never both — declare a Language placement to carry the tag in its \
                         own position"
                    ),
                });
            }
            Ok(TermValue::Literal {
                lexical_form: text.clone(),
                datatype: RDF_LANG_STRING.to_owned(),
                language: Some(tag.clone()),
                direction: None,
            })
        }
        RequestTerm::Spatial { geometry, .. } => {
            let datatype = required_datatype(
                producer,
                term_index,
                facet,
                placement,
                "PurRDF mints no geometry datatype, so the producer must declare the one \
                 its own geometry encoding uses",
            )?;
            Ok(TermValue::Literal {
                lexical_form: geometry.clone(),
                datatype,
                language: None,
                direction: None,
            })
        }
        RequestTerm::EntitySeed { entity } => {
            render::decode_term(entity.as_str()).map_err(|reason| PlacementError::Unrenderable {
                term_index,
                facet,
                reason,
            })
        }
        RequestTerm::Vector { embedding, .. } => {
            let datatype = required_datatype(
                producer,
                term_index,
                facet,
                placement,
                "PurRDF mints no datatype for a query embedding, so the producer must \
                 declare the one its own space reads an embedding under — and it owns the \
                 parse, through `decode_embedding`",
            )?;
            Ok(TermValue::Literal {
                // Exact: the components' bit patterns, so the constant the
                // producer reads is the vector the caller handed in. See
                // `crate::embedding` for why a decimal form would not be.
                lexical_form: encode_embedding(embedding),
                datatype,
                language: None,
                direction: None,
            })
        }
        RequestTerm::Temporal { .. } | RequestTerm::NumericRange { .. } => {
            Err(PlacementError::Unrenderable {
                term_index,
                facet,
                reason: format!(
                    "producer {producer} declares a value placement for an interval, which is \
                     two constants and not one; declare LowerBound and UpperBound placements \
                     so each endpoint reaches its own argument position"
                ),
            })
        }
    }
}

/// Which endpoint of an interval a placement renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Endpoint {
    /// The inclusive lower endpoint.
    Lower,
    /// The inclusive upper endpoint.
    Upper,
}

/// The `LowerBound` / `UpperBound` facet: one endpoint of an interval term.
///
/// An absent endpoint is a genuinely half-open interval, so it is a
/// [`PlacementError::MissingFacet`] rather than a rendering failure: the term
/// carries no such endpoint, and the producer asked for one. Rendering a
/// substitute — an infinity, a zero, the other endpoint — would answer a
/// different question, so none is invented.
///
/// Neither endpoint is rendered without a producer-declared datatype. An
/// untyped endpoint is a plain string, and a plain string compares
/// lexicographically: `"9"` would sort above `"10"`, and a temporal endpoint
/// would not be a point in time at all. The producer names the datatype its own
/// relation compares under, or the facet is not renderable.
fn endpoint_facet(
    producer: &str,
    term_index: u32,
    term: &RequestTerm,
    placement: &TermPlacement,
    endpoint: Endpoint,
) -> Result<TermValue, PlacementError> {
    let facet = placement.facet;
    let lexical = match term {
        RequestTerm::Temporal { lower, upper, .. } => match endpoint {
            Endpoint::Lower => lower.clone(),
            Endpoint::Upper => upper.clone(),
        },
        RequestTerm::NumericRange { lower, upper, .. } => match endpoint {
            Endpoint::Lower => *lower,
            Endpoint::Upper => *upper,
        }
        // Exact base-10, never a float: `to_decimal_lexical` reproduces the raw
        // fixed-point integer with nothing lost.
        .map(purrdf_text::Fixed::to_decimal_lexical),
        _ => return Err(PlacementError::MissingFacet { term_index, facet }),
    };
    let Some(lexical_form) = lexical else {
        return Err(PlacementError::MissingFacet { term_index, facet });
    };
    let datatype = required_datatype(
        producer,
        term_index,
        facet,
        placement,
        "an untyped endpoint is a plain string and a plain string compares \
         lexicographically, so the producer must declare the datatype its own \
         relation orders under",
    )?;
    Ok(TermValue::Literal {
        lexical_form,
        datatype,
        language: None,
        direction: None,
    })
}

/// A literal carrying the placement's datatype, or a plain string when it
/// declares none.
fn plain_or_typed(lexical_form: &str, placement: &TermPlacement) -> TermValue {
    TermValue::Literal {
        lexical_form: lexical_form.to_owned(),
        datatype: placement
            .datatype
            .clone()
            .unwrap_or_else(|| XSD_STRING.to_owned()),
        language: None,
        direction: None,
    }
}

/// The placement's datatype, refusing the placement when it declares none.
///
/// Used where an absent datatype would force this layer to invent one, which it
/// never does: the producer names the datatype its own encoding uses, or the
/// facet is not renderable.
fn required_datatype(
    producer: &str,
    term_index: u32,
    facet: RequestFacet,
    placement: &TermPlacement,
    why: &str,
) -> Result<String, PlacementError> {
    placement
        .datatype
        .clone()
        .ok_or_else(|| PlacementError::Unrenderable {
            term_index,
            facet,
            reason: format!("producer {producer} declares no datatype for this placement: {why}"),
        })
}

/// Refuse a value that has no SPARQL constant form, naming the facet it came
/// from.
fn renderable(
    term_index: u32,
    facet: RequestFacet,
    value: &TermValue,
) -> Result<(), PlacementError> {
    render::sparql_term(value)
        .map(|_| ())
        .map_err(|error| PlacementError::Unrenderable {
            term_index,
            facet,
            reason: error.to_string(),
        })
}

/// One argument position of a rendered call.
///
/// Lives here rather than beside the query it is written into, because it is the
/// per-position output of [`render_slots`] and therefore the shape of a [`Slot`] once
/// rendered: the two enums are read together, and a reader who finds them in one place
/// cannot pair the wrong arms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UnitArgument {
    /// A position rendered once, at emission: a constant [`place`] put a request facet
    /// into. It does not depend on the depth.
    Placed(String),
    /// A position nothing was placed into. Each text renders it for the question it
    /// asks: the streaming read as the variable `?c{position}`, because it projects
    /// the candidate and the block out of such positions, and the exclusion lookup as
    /// a blank node, because it reads nothing out of them but the candidate — see
    /// `RenderedQuery::exclusion_text`.
    Free,
    /// The depth argument of a producer that declared a
    /// [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement), held as the datatype
    /// that producer declared for it and rendered from the unit's own depth every
    /// time the text is asked for.
    ///
    /// The number is `compile::depth_argument`'s, which is not the emitted `LIMIT`'s —
    /// see the [`compile`](crate::compile) module header for why a request and a
    /// ceiling are not one number.
    Depth {
        /// The literal datatype IRI the producer declared for its depth argument.
        datatype: String,
    },
}

impl UnitArgument {
    /// Whether this position is the depth argument.
    pub(crate) const fn is_depth(&self) -> bool {
        matches!(*self, Self::Depth { .. })
    }
}

/// Render every occupied slot, leaving a free position [`UnitArgument::Free`] and the
/// depth's own position as the number's placeholder.
///
/// Shared so the branch text is written in exactly one place. The depth position is
/// carried out rather than rendered, because the number in it is the unit's and is
/// written on every read of the unit's text; which position that is comes from the
/// [`Slot`] placement itself rather than from a second reading of the declaration.
///
/// # Errors
///
/// A [`RenderError`](render::RenderError) if a placed value has no constant
/// form. [`place`] proves every slot it fills renders, so only a hand-built
/// [`Invocation`] can reach this.
pub(crate) fn render_slots(
    invocation: &Invocation,
) -> Result<Vec<UnitArgument>, render::RenderError> {
    invocation
        .slots
        .iter()
        .map(|slot| match slot {
            Slot::Free => Ok(UnitArgument::Free),
            Slot::Placed(value) => render::sparql_term(value).map(UnitArgument::Placed),
            Slot::Depth { datatype } => Ok(UnitArgument::Depth {
                datatype: datatype.clone(),
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Invocation, PlacementError, UnitArgument, place, render_slots};
    use crate::iri::{Iri, Term};
    use crate::request::{Metric, RequestTerm};
    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_sparql_eval::{
        AcceptedTerm, CandidateDomains, DepthPlacement, DuplicatePolicy, ExclusionBasis,
        PfDescriptor, PfMode, RankArithmetic, RankFidelity, RankedDeclaration, RequestFacet,
        TermKind, TermPattern, TermPlacement, Volatility,
    };

    fn ex(suffix: &str) -> String {
        format!("http://example.org/{suffix}")
    }

    fn descriptor(subject: usize, object: usize, modes: &[&str]) -> PfDescriptor {
        PfDescriptor {
            iri: ex("pf/mock"),
            subject_arity: subject,
            object_arity: object,
            volatility: Volatility::Stable,
            modes: modes
                .iter()
                .map(|code| PfMode {
                    code: (*code).to_owned(),
                    rows_per_invocation: 10,
                })
                .collect(),
            ranked: None,
        }
    }

    fn declaration(
        accepted: Vec<AcceptedTerm>,
        depth: Option<DepthPlacement>,
    ) -> RankedDeclaration {
        RankedDeclaration {
            stratum: purrdf_core::parse_iri(&ex("stratum/mock")).expect("fixture IRI"),
            accepted_terms: accepted,
            depth_placement: depth,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            // Placement is about rendering arguments, not about fusion, so
            // these fixtures make the widest promise there is on both terms.
            fidelity: RankFidelity::EXACT,
            arithmetic: RankArithmetic::FloatFree,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            exclusion: ExclusionBasis::Unavailable,
            mandatory: false,
        }
    }

    fn value_at(position: usize) -> Vec<TermPlacement> {
        vec![TermPlacement {
            facet: RequestFacet::Value,
            position,
            datatype: None,
        }]
    }

    fn any_accepting(placements: Vec<TermPlacement>) -> Vec<AcceptedTerm> {
        vec![AcceptedTerm {
            pattern: TermPattern::of_kind(TermKind::Any),
            placements,
        }]
    }

    fn lexical(language: Option<&str>) -> RequestTerm {
        RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: language.map(ToOwned::to_owned),
            predicate: Some(Iri::parse(&ex("body")).expect("fixture IRI")),
        }
    }

    /// The constant text at each argument position, with the depth's own position
    /// named rather than rendered: the number there is the unit's, written when the
    /// unit's text is asked for, so placement has no value to show.
    fn placed(invocation: &Invocation) -> Vec<String> {
        render_slots(invocation)
            .expect("every placed slot renders")
            .into_iter()
            .enumerate()
            .map(|(position, argument)| match argument {
                UnitArgument::Placed(text) => text,
                UnitArgument::Free => format!("?c{position}"),
                UnitArgument::Depth { datatype } => format!("<the unit's depth>^^<{datatype}>"),
            })
            .collect()
    }

    #[test]
    fn lexical_value_carries_the_tag_only_when_no_language_placement_does() {
        let descriptor = descriptor(1, 2, &["fff"]);
        // No Language placement: the tag rides on the needle.
        let tagged = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(any_accepting(value_at(1)), None),
            &[lexical(Some("en"))],
            &[0],
        )
        .expect("a tagged needle places");
        assert_eq!(placed(&tagged)[1], "\"quick brown fox\"@en");

        // A Language placement: the tag has its own position, the needle is plain.
        let split = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(
                any_accepting(vec![
                    TermPlacement {
                        facet: RequestFacet::Value,
                        position: 1,
                        datatype: None,
                    },
                    TermPlacement {
                        facet: RequestFacet::Language,
                        position: 2,
                        datatype: None,
                    },
                ]),
                None,
            ),
            &[lexical(Some("en"))],
            &[0],
        )
        .expect("a split needle places");
        assert_eq!(placed(&split)[1], "\"quick brown fox\"");
        assert_eq!(placed(&split)[2], "\"en\"");
    }

    #[test]
    fn an_untagged_needle_with_a_language_placement_is_refused_but_a_tagged_one_places() {
        let descriptor = descriptor(1, 2, &["fff"]);
        let decl = declaration(
            any_accepting(vec![
                TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                },
                TermPlacement {
                    facet: RequestFacet::Language,
                    position: 2,
                    datatype: None,
                },
            ]),
            None,
        );
        let error = place(&ex("pf/mock"), &descriptor, &decl, &[lexical(None)], &[0])
            .expect_err("an untagged needle cannot fill a language position");
        assert!(
            matches!(
                error,
                PlacementError::MissingFacet {
                    facet: RequestFacet::Language,
                    ..
                }
            ),
            "got {error:?}"
        );
        assert!(
            place(
                &ex("pf/mock"),
                &descriptor,
                &decl,
                &[lexical(Some("en"))],
                &[0],
            )
            .is_ok(),
            "the neighbouring tagged needle still places"
        );
    }

    /// `example.org/embedding`, named by the *fixture producer*: PurRDF mints no
    /// datatype for a query embedding, and a producer that declares none has its
    /// value placement refused rather than rendered under an invented one.
    fn embedding_datatype() -> String {
        ex("embedding")
    }

    #[test]
    fn an_untyped_embedding_is_refused_but_a_typed_one_places_exactly() {
        let descriptor = descriptor(1, 1, &["ff"]);
        let vector = RequestTerm::Vector {
            embedding: vec![0.25, -1.5],
            metric: Metric::Cosine,
            index_hint: None,
        };
        let error = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(any_accepting(value_at(1)), None),
            std::slice::from_ref(&vector),
            &[0],
        )
        .expect_err("an embedding under no declared datatype is not renderable");
        match error {
            PlacementError::Unrenderable { reason, .. } => {
                assert!(reason.contains("embedding"), "{reason}");
            }
            other => panic!("expected Unrenderable, got {other:?}"),
        }

        // The neighbouring valid case: the producer names the datatype its own
        // space reads an embedding under, and the components ride exactly.
        let typed = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(
                any_accepting(vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: Some(embedding_datatype()),
                }]),
                None,
            ),
            std::slice::from_ref(&vector),
            &[0],
        )
        .expect("a declared datatype makes the embedding renderable");
        assert_eq!(
            placed(&typed)[1],
            format!("\"3E800000 BFC00000\"^^<{}>", embedding_datatype())
        );
        assert_eq!(
            crate::embedding::decode_embedding("3E800000 BFC00000"),
            Ok(vec![0.25, -1.5]),
            "and the lexical the producer receives recovers the exact vector"
        );
    }

    #[test]
    fn a_seed_places() {
        let descriptor = descriptor(1, 1, &["ff"]);
        let decl = declaration(any_accepting(value_at(1)), None);
        let seeded = place(
            &ex("pf/mock"),
            &descriptor,
            &decl,
            &[RequestTerm::EntitySeed {
                entity: Term::new(format!("<{}>", ex("s"))),
            }],
            &[0],
        )
        .expect("an IRI seed places");
        assert_eq!(placed(&seeded)[1], format!("<{}>", ex("s")));
    }

    #[test]
    fn a_blank_seed_is_refused_but_an_iri_seed_places() {
        let descriptor = descriptor(1, 1, &["ff"]);
        let decl = declaration(any_accepting(value_at(1)), None);
        let error = place(
            &ex("pf/mock"),
            &descriptor,
            &decl,
            &[RequestTerm::EntitySeed {
                entity: Term::new("_:b0"),
            }],
            &[0],
        )
        .expect_err("a blank node is a non-distinguished variable");
        match error {
            PlacementError::Unrenderable { reason, .. } => {
                assert!(reason.contains("non-distinguished variable"), "{reason}");
            }
            other => panic!("expected Unrenderable, got {other:?}"),
        }
        assert!(
            place(
                &ex("pf/mock"),
                &descriptor,
                &decl,
                &[RequestTerm::EntitySeed {
                    entity: Term::new(format!("<{}>", ex("s"))),
                }],
                &[0],
            )
            .is_ok(),
            "the neighbouring IRI seed still places"
        );
    }

    #[test]
    fn a_different_value_at_one_position_conflicts_but_the_same_value_does_not() {
        let descriptor = descriptor(1, 1, &["ff"]);
        let decl = declaration(any_accepting(value_at(1)), None);
        let conflicting = place(
            &ex("pf/mock"),
            &descriptor,
            &decl,
            &[
                RequestTerm::EntitySeed {
                    entity: Term::new(format!("<{}>", ex("a"))),
                },
                RequestTerm::EntitySeed {
                    entity: Term::new(format!("<{}>", ex("b"))),
                },
            ],
            &[0, 1],
        )
        .expect_err("two different seeds cannot share one position");
        assert_eq!(
            conflicting,
            PlacementError::PositionConflict { position: 1 }
        );
        let identical = place(
            &ex("pf/mock"),
            &descriptor,
            &decl,
            &[
                RequestTerm::EntitySeed {
                    entity: Term::new(format!("<{}>", ex("a"))),
                },
                RequestTerm::EntitySeed {
                    entity: Term::new(format!("<{}>", ex("a"))),
                },
            ],
            &[0, 1],
        )
        .expect("an identical re-write is not a conflict");
        assert_eq!(placed(&identical)[1], format!("<{}>", ex("a")));
    }

    #[test]
    fn a_depth_argument_is_refused_when_undeclared_and_placed_when_declared() {
        // A kNN-shaped relation: subject candidate, object needle and depth, and
        // it can only run with the depth bound.
        let descriptor = descriptor(1, 2, &["fbb"]);
        let without = place(
            &ex("pf/knn"),
            &descriptor,
            &declaration(any_accepting(value_at(1)), None),
            &[lexical(None)],
            &[0],
        )
        .expect_err("`fbb` cannot serve an invocation that leaves the depth free");
        match without {
            PlacementError::NoSatisfiableMode {
                invocation,
                declared,
            } => {
                assert_eq!(invocation, "fbf");
                assert_eq!(declared, vec!["fbb".to_owned()]);
            }
            other => panic!("expected NoSatisfiableMode, got {other:?}"),
        }
        let with = place(
            &ex("pf/knn"),
            &descriptor,
            &declaration(
                any_accepting(value_at(1)),
                Some(DepthPlacement {
                    position: 2,
                    datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                }),
            ),
            &[lexical(None)],
            &[0],
        )
        .expect("the declared depth placement makes `fbb` satisfiable");
        assert_eq!(with.mode, BindingPattern::from_code("fbb"));
        // The depth's position is OCCUPIED — which is the whole of what makes `fbb`
        // satisfiable — and holds no number: the datatype its producer declared comes
        // back, and the number is rendered from the unit's own depth when the unit's
        // text is asked for. That is why placement needs no depth to run.
        assert_eq!(
            placed(&with)[2],
            "<the unit's depth>^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn bound_indices_are_consumed_in_ascending_order() {
        let descriptor = descriptor(1, 2, &["fff"]);
        let decl = declaration(
            vec![
                AcceptedTerm {
                    pattern: TermPattern::of_kind(TermKind::Iri),
                    placements: value_at(1),
                },
                AcceptedTerm {
                    pattern: TermPattern::of_kind(TermKind::Literal),
                    placements: value_at(2),
                },
            ],
            None,
        );
        let terms = vec![
            RequestTerm::EntitySeed {
                entity: Term::new(format!("<{}>", ex("a"))),
            },
            RequestTerm::EntitySeed {
                entity: Term::new("\"needle\""),
            },
        ];
        let forward = place(&ex("pf/mock"), &descriptor, &decl, &terms, &[0, 1])
            .expect("ascending indices place");
        let reversed = place(&ex("pf/mock"), &descriptor, &decl, &terms, &[1, 0])
            .expect("reordered indices place identically");
        assert_eq!(forward, reversed);
        assert_eq!(placed(&forward), placed(&reversed));
    }

    #[test]
    fn a_refusal_names_the_lowest_refusing_term_in_either_order() {
        // Both terms refuse. Whichever is consumed first names the error, so a
        // hand-edited plan that reorders its indices would otherwise change the
        // refusal it gets back.
        let descriptor = descriptor(1, 1, &["ff"]);
        let decl = declaration(any_accepting(value_at(1)), None);
        let terms = vec![
            RequestTerm::Vector {
                embedding: vec![0.5],
                metric: Metric::Euclidean,
                index_hint: None,
            },
            RequestTerm::EntitySeed {
                entity: Term::new("_:b0"),
            },
        ];
        for order in [vec![0_u32, 1], vec![1, 0]] {
            let error = place(&ex("pf/mock"), &descriptor, &decl, &terms, &order)
                .expect_err("both terms refuse");
            match error {
                PlacementError::Unrenderable { term_index, .. } => assert_eq!(term_index, 0),
                other => panic!("expected Unrenderable, got {other:?}"),
            }
        }
    }

    // -----------------------------------------------------------------------
    // The interval modalities: carried ahead of any producer that takes them,
    // and renderable now rather than dead.
    // -----------------------------------------------------------------------

    /// `xsd:dateTime`, named by the *fixture producer*, never by this layer:
    /// PurRDF mints no calendar datatype, and a producer that declares none has
    /// its endpoint placement refused rather than rendered as a plain string.
    const XSD_DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

    /// `xsd:decimal`, likewise the fixture producer's declaration.
    const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

    fn bounds_at(lower: usize, upper: usize, datatype: &str) -> Vec<TermPlacement> {
        vec![
            TermPlacement {
                facet: RequestFacet::LowerBound,
                position: lower,
                datatype: Some(datatype.to_owned()),
            },
            TermPlacement {
                facet: RequestFacet::UpperBound,
                position: upper,
                datatype: Some(datatype.to_owned()),
            },
        ]
    }

    #[test]
    fn a_temporal_interval_renders_both_endpoints_under_the_declared_datatype() {
        let descriptor = descriptor(1, 2, &["fff"]);
        let placed_interval = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(any_accepting(bounds_at(1, 2, XSD_DATE_TIME)), None),
            &[RequestTerm::Temporal {
                predicate: Iri::parse(&ex("observed")).expect("fixture IRI"),
                lower: Some("2026-01-01T00:00:00Z".to_owned()),
                upper: Some("2026-02-01T00:00:00Z".to_owned()),
            }],
            &[0],
        )
        .expect("both endpoints have a constant form");
        assert_eq!(
            placed(&placed_interval)[1],
            format!("\"2026-01-01T00:00:00Z\"^^<{XSD_DATE_TIME}>")
        );
        assert_eq!(
            placed(&placed_interval)[2],
            format!("\"2026-02-01T00:00:00Z\"^^<{XSD_DATE_TIME}>")
        );
    }

    #[test]
    fn a_numeric_range_renders_its_endpoints_in_exact_base_ten() {
        let descriptor = descriptor(1, 2, &["fff"]);
        let placed_range = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(any_accepting(bounds_at(1, 2, XSD_DECIMAL)), None),
            &[RequestTerm::NumericRange {
                predicate: Iri::parse(&ex("price")).expect("fixture IRI"),
                lower: Some(purrdf_text::Fixed::from_raw(1_500_000_000_000)),
                upper: Some(purrdf_text::Fixed::from_raw(2_250_000_000_000)),
            }],
            &[0],
        )
        .expect("both endpoints have a constant form");
        // Exact: the raw fixed-point integer, reproduced digit for digit.
        assert_eq!(
            placed(&placed_range)[1],
            format!("\"1.500000000000\"^^<{XSD_DECIMAL}>")
        );
        assert_eq!(
            placed(&placed_range)[2],
            format!("\"2.250000000000\"^^<{XSD_DECIMAL}>")
        );
    }

    #[test]
    fn a_half_open_interval_refuses_the_endpoint_it_does_not_carry_but_places_the_one_it_does() {
        let descriptor = descriptor(1, 2, &["fff"]);
        let open_above = RequestTerm::NumericRange {
            predicate: Iri::parse(&ex("price")).expect("fixture IRI"),
            lower: Some(purrdf_text::Fixed::ONE),
            upper: None,
        };
        let error = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(any_accepting(bounds_at(1, 2, XSD_DECIMAL)), None),
            std::slice::from_ref(&open_above),
            &[0],
        )
        .expect_err("an absent endpoint is not a value to invent");
        assert!(
            matches!(
                error,
                PlacementError::MissingFacet {
                    facet: RequestFacet::UpperBound,
                    ..
                }
            ),
            "got {error:?}"
        );
        // The neighbouring valid case: a producer that asks only for the
        // endpoint the term carries places it.
        let lower_only = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(
                any_accepting(vec![TermPlacement {
                    facet: RequestFacet::LowerBound,
                    position: 1,
                    datatype: Some(XSD_DECIMAL.to_owned()),
                }]),
                None,
            ),
            &[open_above],
            &[0],
        )
        .expect("the endpoint the term does carry places");
        assert_eq!(
            placed(&lower_only)[1],
            format!("\"1.000000000000\"^^<{XSD_DECIMAL}>")
        );
    }

    #[test]
    fn an_untyped_endpoint_is_refused_but_a_typed_one_places() {
        // An untyped endpoint would be a plain string, and a plain string
        // compares lexicographically: `"9"` above `"10"`. The producer names the
        // datatype its own relation orders under, or the facet is not rendered.
        let descriptor = descriptor(1, 1, &["ff"]);
        let term = RequestTerm::Temporal {
            predicate: Iri::parse(&ex("observed")).expect("fixture IRI"),
            lower: Some("2026-01-01T00:00:00Z".to_owned()),
            upper: None,
        };
        let untyped = vec![TermPlacement {
            facet: RequestFacet::LowerBound,
            position: 1,
            datatype: None,
        }];
        let error = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(any_accepting(untyped), None),
            std::slice::from_ref(&term),
            &[0],
        )
        .expect_err("an endpoint with no declared datatype is not renderable");
        match error {
            PlacementError::Unrenderable { reason, .. } => {
                assert!(reason.contains("lexicographically"), "{reason}");
            }
            other => panic!("expected Unrenderable, got {other:?}"),
        }
        assert!(
            place(
                &ex("pf/mock"),
                &descriptor,
                &declaration(
                    any_accepting(vec![TermPlacement {
                        facet: RequestFacet::LowerBound,
                        position: 1,
                        datatype: Some(XSD_DATE_TIME.to_owned()),
                    }]),
                    None,
                ),
                &[term],
                &[0],
            )
            .is_ok(),
            "the neighbouring typed endpoint still places"
        );
    }

    #[test]
    fn an_interval_refuses_a_value_placement_and_says_where_its_endpoints_go() {
        // An interval is two constants. A producer that declares one `Value`
        // position for it would receive one of them, or neither, and answer a
        // wider question than the caller asked.
        let descriptor = descriptor(1, 1, &["ff"]);
        let error = place(
            &ex("pf/mock"),
            &descriptor,
            &declaration(any_accepting(value_at(1)), None),
            &[RequestTerm::NumericRange {
                predicate: Iri::parse(&ex("price")).expect("fixture IRI"),
                lower: Some(purrdf_text::Fixed::ONE),
                upper: Some(purrdf_text::Fixed::ONE),
            }],
            &[0],
        )
        .expect_err("an interval has no single constant form");
        match error {
            PlacementError::Unrenderable { reason, .. } => {
                assert!(reason.contains("LowerBound"), "{reason}");
                assert!(reason.contains("UpperBound"), "{reason}");
            }
            other => panic!("expected Unrenderable, got {other:?}"),
        }
    }

    #[test]
    fn an_interval_is_reached_by_a_literal_pattern_and_by_its_own_predicate() {
        let term = RequestTerm::Temporal {
            predicate: Iri::parse(&ex("observed")).expect("fixture IRI"),
            lower: Some("2026-01-01T00:00:00Z".to_owned()),
            upper: None,
        };
        assert!(
            super::kind_matches(TermKind::Literal, &term),
            "an interval's endpoints are literals, so a literal producer is reachable"
        );
        assert!(super::kind_matches(TermKind::Any, &term));
        assert!(!super::kind_matches(TermKind::Iri, &term));
        assert!(
            super::pattern_matches(
                &TermPattern {
                    kind: TermKind::Literal,
                    datatype: None,
                    language: None,
                    predicate: Some(ex("observed")),
                },
                &term
            ),
            "a predicate constraint reads the interval's own predicate"
        );
        assert!(
            !super::pattern_matches(
                &TermPattern {
                    kind: TermKind::Literal,
                    datatype: None,
                    language: None,
                    predicate: Some(ex("elsewhere")),
                },
                &term
            ),
            "and refuses one it does not carry"
        );
    }

    /// A quoted-triple seed opens `<<`, which also opens `<`, so a classifier
    /// that tests for `<` first reports it as an IRI. Both halves are asserted
    /// here because each is a distinct defect: the triple kind becoming
    /// unmatchable, and the IRI kind capturing a term it cannot read.
    #[test]
    fn a_quoted_triple_seed_is_a_triple_and_not_an_iri() {
        let seed = RequestTerm::EntitySeed {
            entity: Term::new("<<( <http://example.org/s> <http://example.org/p> \"o\" )>>"),
        };

        assert_eq!(
            super::seed_kind("<<( <http://example.org/s> <http://example.org/p> \"o\" )>>"),
            Some(TermKind::Triple),
            "a term opening `<<` is a quoted triple, not an IRI"
        );
        assert!(
            super::kind_matches(TermKind::Triple, &seed),
            "a producer accepting quoted triples must be reachable by one"
        );
        assert!(
            !super::kind_matches(TermKind::Iri, &seed),
            "a producer accepting IRIs must not be handed a quoted triple"
        );
        assert!(
            super::kind_matches(TermKind::Any, &seed),
            "an unconstrained pattern still accepts it"
        );
    }

    /// The neighbouring valid case: narrowing the quoted-triple classification
    /// must not have narrowed the IRI one, which is the term shape every
    /// existing seed fixture uses.
    #[test]
    fn an_iri_seed_is_still_an_iri_and_not_a_triple() {
        let seed = RequestTerm::EntitySeed {
            entity: Term::new("<http://example.org/seed>"),
        };

        assert_eq!(
            super::seed_kind("<http://example.org/seed>"),
            Some(TermKind::Iri),
            "a term opening a single `<` is still an IRI"
        );
        assert!(
            super::kind_matches(TermKind::Iri, &seed),
            "an IRI seed still reaches a producer accepting IRIs"
        );
        assert!(
            !super::kind_matches(TermKind::Triple, &seed),
            "an IRI is not a quoted triple"
        );
    }

    /// `seed_kind` decides what a pattern may match and `decode_term` decides
    /// what the seed is; if they disagree, a producer is selected on one
    /// reading and handed a term built on the other.
    #[test]
    fn seed_classification_agrees_with_the_decoder() {
        for (text, kind) in [
            (
                "<<( <http://example.org/s> <http://example.org/p> \"o\" )>>",
                TermKind::Triple,
            ),
            ("<http://example.org/seed>", TermKind::Iri),
            ("\"needle\"", TermKind::Literal),
            ("\"needle\"@en", TermKind::Literal),
        ] {
            assert_eq!(
                super::seed_kind(text),
                Some(kind),
                "classification of {text}"
            );
            let decoded = crate::render::decode_term(text)
                .unwrap_or_else(|error| panic!("the decoder reads {text}: {error}"));
            let decoded_kind = match decoded {
                purrdf_core::TermValue::Triple { .. } => TermKind::Triple,
                purrdf_core::TermValue::Iri(_) => TermKind::Iri,
                purrdf_core::TermValue::Literal { .. } => TermKind::Literal,
                purrdf_core::TermValue::Blank { .. } => TermKind::Blank,
            };
            assert_eq!(
                decoded_kind, kind,
                "the decoder and the classifier disagree about {text}"
            );
        }
    }
}

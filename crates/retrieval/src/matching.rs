// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! [`AcceptedTerm`] so a reader cannot mis-align them, and they are paired here
//! for the same reason.
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

use crate::render::{self, RDF_LANG_STRING, XSD_STRING};
use crate::request::RequestTerm;

/// One producer invocation, rendered from the request: what each flattened
/// argument position holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    /// Flattened position -> the constant to render, or `None` for a free `?cN`.
    pub(crate) slots: Vec<Option<TermValue>>,
    /// The access pattern the invocation actually has: bit `p` set iff
    /// `slots[p]` is occupied.
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
        RequestTerm::Lexical { .. } | RequestTerm::Spatial { .. } => kind == TermKind::Literal,
        RequestTerm::EntitySeed { entity } => seed_kind(entity.as_str()) == Some(kind),
        // A vector term targets no RDF term kind; only `Any` accepts it.
        RequestTerm::Vector { .. } => false,
    }
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
/// # Errors
///
/// A [`PlacementError`] naming the exact rule that refused; see that type.
pub(crate) fn place(
    producer: &str,
    descriptor: &PfDescriptor,
    decl: &RankedDeclaration,
    request_terms: &[RequestTerm],
    bound_indices: &[u32],
    depth: u32,
) -> Result<Invocation, PlacementError> {
    let total = descriptor.subject_arity + descriptor.object_arity;
    let declared: Vec<String> = descriptor
        .modes
        .iter()
        .map(|mode| mode.code.clone())
        .collect();
    if total > BindingPattern::MAX_ARITY {
        // The evaluator expresses an access pattern as a 64-bit set, so a
        // relation wider than that has no mode any invocation could satisfy.
        return Err(PlacementError::NoSatisfiableMode {
            invocation: format!(
                "<{total} positions, wider than the {} a binding pattern carries>",
                BindingPattern::MAX_ARITY
            ),
            declared,
        });
    }
    let mut slots: Vec<Option<TermValue>> = vec![None; total];

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
        // A decimal integer with a declared datatype and no tag: the one term
        // shape [`render::sparql_term`] can never refuse, so it is not re-checked.
        let value = TermValue::Literal {
            lexical_form: depth.to_string(),
            datatype: depth_placement.datatype.clone(),
            language: None,
            direction: None,
        };
        occupy(&mut slots, depth_placement.position, value)?;
    }

    let mode = BindingPattern::from_bools(slots.iter().map(Option::is_some));
    if !descriptor
        .modes
        .iter()
        .any(|declared_mode| BindingPattern::from_code(&declared_mode.code).subsumes(mode))
    {
        return Err(PlacementError::NoSatisfiableMode {
            invocation: mode.code(),
            declared,
        });
    }

    Ok(Invocation { slots, mode })
}

/// Write `value` into `slots[position]`, accepting an identical re-write.
///
/// A second placement that writes the same value is not a conflict — two
/// alternatives can legitimately render the same constant into one position —
/// but a *different* value is, because only one of the two questions could then
/// be asked.
fn occupy(
    slots: &mut [Option<TermValue>],
    position: usize,
    value: TermValue,
) -> Result<(), PlacementError> {
    let slot = slots
        .get_mut(position)
        .ok_or(PlacementError::PositionConflict { position })?;
    match slot {
        Some(existing) if *existing == value => Ok(()),
        Some(_) => Err(PlacementError::PositionConflict { position }),
        None => {
            *slot = Some(value);
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
            RequestTerm::Spatial { predicate, .. } => Ok(TermValue::iri(predicate.as_str())),
            _ => Err(PlacementError::MissingFacet { term_index, facet }),
        },
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
        RequestTerm::Vector { embedding, .. } => Err(PlacementError::Unrenderable {
            term_index,
            facet,
            reason: format!(
                "producer {producer} declares a value placement for a {}-dimensional query \
                 embedding, which has no SPARQL constant form",
                embedding.len()
            ),
        }),
    }
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

/// Render every occupied slot, leaving a free position as `?c{position}`.
///
/// Shared so the branch text is written in exactly one place.
///
/// # Errors
///
/// A [`RenderError`](render::RenderError) if a placed value has no constant
/// form. [`place`] proves every slot it fills renders, so only a hand-built
/// [`Invocation`] can reach this.
pub(crate) fn render_slots(invocation: &Invocation) -> Result<Vec<String>, render::RenderError> {
    invocation
        .slots
        .iter()
        .enumerate()
        .map(|(position, slot)| match slot {
            None => Ok(format!("?c{position}")),
            Some(value) => render::sparql_term(value),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Invocation, PlacementError, place, render_slots};
    use crate::iri::{Iri, Term};
    use crate::request::{Metric, RequestTerm};
    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_sparql_eval::{
        AcceptedTerm, DepthPlacement, DuplicatePolicy, PfDescriptor, PfMode, RankOrdering,
        RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
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
            ordering: RankOrdering::StrictlyDescending,
            duplicates: DuplicatePolicy::Unique,
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

    fn placed(invocation: &Invocation) -> Vec<String> {
        render_slots(invocation).expect("every placed slot renders")
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
            5,
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
            5,
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
        let error = place(
            &ex("pf/mock"),
            &descriptor,
            &decl,
            &[lexical(None)],
            &[0],
            5,
        )
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
                5,
            )
            .is_ok(),
            "the neighbouring tagged needle still places"
        );
    }

    #[test]
    fn a_vector_is_unrenderable_but_a_seed_places() {
        let descriptor = descriptor(1, 1, &["ff"]);
        let decl = declaration(any_accepting(value_at(1)), None);
        let error = place(
            &ex("pf/mock"),
            &descriptor,
            &decl,
            &[RequestTerm::Vector {
                embedding: vec![0.25, -1.5],
                metric: Metric::Cosine,
                index_hint: None,
            }],
            &[0],
            5,
        )
        .expect_err("an embedding has no constant form");
        assert!(
            matches!(error, PlacementError::Unrenderable { .. }),
            "got {error:?}"
        );
        let seeded = place(
            &ex("pf/mock"),
            &descriptor,
            &decl,
            &[RequestTerm::EntitySeed {
                entity: Term::new(format!("<{}>", ex("s"))),
            }],
            &[0],
            5,
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
            5,
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
                5,
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
            5,
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
            5,
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
            7,
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
            7,
        )
        .expect("the declared depth placement makes `fbb` satisfiable");
        assert_eq!(with.mode, BindingPattern::from_code("fbb"));
        assert_eq!(
            placed(&with)[2],
            "\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>"
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
        let forward = place(&ex("pf/mock"), &descriptor, &decl, &terms, &[0, 1], 5)
            .expect("ascending indices place");
        let reversed = place(&ex("pf/mock"), &descriptor, &decl, &terms, &[1, 0], 5)
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
            let error = place(&ex("pf/mock"), &descriptor, &decl, &terms, &order, 5)
                .expect_err("both terms refuse");
            match error {
                PlacementError::Unrenderable { term_index, .. } => assert_eq!(term_index, 0),
                other => panic!("expected Unrenderable, got {other:?}"),
            }
        }
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

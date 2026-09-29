// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A governed call's ceilings, as a host received them, turned into the
//! evaluator's [`QueryGovernors`].
//!
//! The command line, the C ABI, the wasm package and the Python binding each
//! take the same five ceilings — fuel, answers, intermediate cells, scratch
//! bytes, remote requests — in their own spelling, plus a stop signal they build
//! from their own clock and cancellation handle. [`from_parts`] is the one place
//! those become a configuration, so a governed call means the same thing on
//! every host.
//!
//! # The base is `METERED`
//!
//! A ceiling the caller did not name is not enforced, but it is **metered**:
//! every call starts from [`QueryGovernors::METERED`], which charges every
//! caller-settable dimension against a ceiling no query can reach. Two things
//! follow, and both are what a governed call promises. Every outcome — a
//! complete one included — carries evidence a caller can size the next budget
//! from, where [`QueryGovernors::UNBOUNDED`] reports nothing because it charges
//! nothing. And the evaluator polls the stop signal every
//! [`STOP_POLL_FUEL`](purrdf_sparql_eval::STOP_POLL_FUEL) units of fuel as well
//! as at each algebra node it enters, so a deadline or a cancellation is noticed
//! promptly inside a long-running operator; with fuel disengaged only the second
//! poll runs.
//!
//! # "No ceiling" is said, not defaulted
//!
//! [`GovernorParts::no_ceiling`] asks for [`QueryGovernors::UNBOUNDED`]: no
//! ceiling and no accounting, the cost of a query that was never governed. It
//! has to be written down, because declining governance is a decision, and it
//! cannot be combined with a ceiling — a request that names a ceiling and asks
//! for none contradicts itself, and is refused rather than resolved by guessing
//! which half was meant. A stop signal is not a ceiling and combines with it.

use std::fmt;
use std::sync::Arc;

use purrdf_sparql_eval::{QueryGovernors, ResourceDimension, StopSignal};

/// The ceilings of one governed call, before they are engaged.
///
/// `None` in a slot means the caller named no ceiling on that dimension — never
/// zero, which is a valid ceiling that trips on the first charged unit of work.
/// Every ceiling is inclusive: consumption equal to it is admitted.
///
/// The default names nothing, and [`from_parts`] turns it into
/// [`QueryGovernors::METERED`]: a governed call that bounds nothing and reports
/// everything. Only [`Self::no_ceiling`] reaches the ungoverned state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GovernorParts {
    /// Abstract execution steps ([`ResourceDimension::Fuel`]).
    pub fuel: Option<u64>,
    /// Units committed to the answer sequence ([`ResourceDimension::AnswerRows`]):
    /// solution rows for `SELECT`, output statements for `CONSTRUCT` and
    /// `DESCRIBE`, nothing for `ASK`.
    pub max_answers: Option<u64>,
    /// The largest intermediate bag, in `rows * columns`
    /// ([`ResourceDimension::IntermediateCells`]).
    pub max_intermediate_cells: Option<u64>,
    /// Bytes minted into the per-query scratch arena
    /// ([`ResourceDimension::ScratchBytes`]).
    pub max_scratch_bytes: Option<u64>,
    /// Requests issued to remote or federated endpoints
    /// ([`ResourceDimension::RemoteRequests`]).
    pub max_remote_requests: Option<u64>,
    /// Decline every ceiling and all accounting:
    /// [`QueryGovernors::UNBOUNDED`] instead of [`QueryGovernors::METERED`].
    /// Refused beside any named ceiling.
    pub no_ceiling: bool,
}

impl GovernorParts {
    /// The dimensions this names a ceiling on, with the ceiling, in the order the
    /// fields are declared.
    #[must_use]
    pub fn named(&self) -> Vec<(ResourceDimension, u64)> {
        [
            (ResourceDimension::Fuel, self.fuel),
            (ResourceDimension::AnswerRows, self.max_answers),
            (
                ResourceDimension::IntermediateCells,
                self.max_intermediate_cells,
            ),
            (ResourceDimension::ScratchBytes, self.max_scratch_bytes),
            (ResourceDimension::RemoteRequests, self.max_remote_requests),
        ]
        .into_iter()
        .filter_map(|(dimension, ceiling)| ceiling.map(|ceiling| (dimension, ceiling)))
        .collect()
    }
}

/// Why a governed call's parts do not describe a configuration.
///
/// Each variant names the dimension or the conflict rather than carrying a
/// sentence, so a host words the refusal in its own spelling of the flag
/// (`--max-answers`, `maxAnswers`, `max_answers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GovernorPartsError {
    /// [`GovernorParts::no_ceiling`] was asked for beside a ceiling on
    /// `dimension` (the first one named).
    CeilingWithNoCeiling {
        /// The dimension a ceiling was named on.
        dimension: ResourceDimension,
    },
    /// An `UPDATE` was given an answer cap. An update has no answer sequence to
    /// bound, so the cap would be a governor the caller believes is set and that
    /// nothing can ever trip.
    AnswerCapOnUpdate,
}

impl fmt::Display for GovernorPartsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CeilingWithNoCeiling { dimension } => write!(
                f,
                "a {dimension:?} ceiling was named together with no ceiling at all; name the \
                 ceiling, or ask for none"
            ),
            Self::AnswerCapOnUpdate => f.write_str(
                "an answer cap is not accepted by an UPDATE: an UPDATE has no answer sequence \
                 to bound. Bound the work that computes it with fuel, intermediate cells or \
                 scratch bytes instead",
            ),
        }
    }
}

impl std::error::Error for GovernorPartsError {}

/// The [`QueryGovernors`] a governed query runs under: [`QueryGovernors::METERED`]
/// with each ceiling `parts` names engaged, or [`QueryGovernors::UNBOUNDED`] when
/// `parts` asks for no ceiling; then `stop`, if any, attached.
///
/// `stop` is built by the host — a wall deadline, a cancellation token, an
/// interpreter's interrupt flag, or those composed — because only the host owns a
/// clock and a cancellation handle. It is attached under either base.
///
/// # Errors
///
/// [`GovernorPartsError::CeilingWithNoCeiling`] when `parts` asks for no ceiling
/// and names one.
pub fn from_parts(
    parts: &GovernorParts,
    stop: Option<Arc<dyn StopSignal>>,
) -> Result<QueryGovernors, GovernorPartsError> {
    let mut governors = if parts.no_ceiling {
        if let Some(&(dimension, _)) = parts.named().first() {
            return Err(GovernorPartsError::CeilingWithNoCeiling { dimension });
        }
        QueryGovernors::UNBOUNDED
    } else {
        QueryGovernors::METERED
    };
    if let Some(fuel) = parts.fuel {
        governors = governors.with_fuel(fuel);
    }
    if let Some(answers) = parts.max_answers {
        governors = governors.with_max_answers(answers);
    }
    if let Some(cells) = parts.max_intermediate_cells {
        governors = governors.with_max_intermediate_cells(cells);
    }
    if let Some(bytes) = parts.max_scratch_bytes {
        governors = governors.with_max_scratch_bytes(bytes);
    }
    if let Some(requests) = parts.max_remote_requests {
        governors = governors.with_max_remote_requests(requests);
    }
    if let Some(stop) = stop {
        governors = governors.with_stop_signal(stop);
    }
    Ok(governors)
}

/// [`from_parts`] for a governed `UPDATE`, which refuses an answer cap first.
///
/// # Errors
///
/// [`GovernorPartsError::AnswerCapOnUpdate`] when `parts` names an answer cap,
/// and otherwise whatever [`from_parts`] refuses.
pub fn from_update_parts(
    parts: &GovernorParts,
    stop: Option<Arc<dyn StopSignal>>,
) -> Result<QueryGovernors, GovernorPartsError> {
    if parts.max_answers.is_some() {
        return Err(GovernorPartsError::AnswerCapOnUpdate);
    }
    from_parts(parts, stop)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDatasetBuilder, SparqlRequest};
    use purrdf_sparql_eval::{
        GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions, ResourceDimension,
        StopCause, StopSignal, TrippedGovernor,
    };

    use super::{GovernorParts, GovernorPartsError, from_parts, from_update_parts};

    /// A stop signal that never fires.
    #[derive(Debug)]
    struct Never;

    impl StopSignal for Never {
        fn poll(&self) -> Option<StopCause> {
            None
        }
    }

    fn never() -> Option<Arc<dyn StopSignal>> {
        Some(Arc::new(Never))
    }

    /// Five solutions, one per `VALUES` row, over an empty dataset.
    const FIVE_ANSWERS: &str = "SELECT ?x WHERE { VALUES ?x { 1 2 3 4 5 } }";

    fn run(governors: &QueryGovernors) -> GovernedOutcome {
        NativeSparqlEngine::new()
            .query_governed(
                &RdfDatasetBuilder::new().freeze().expect("an empty dataset"),
                SparqlRequest {
                    query: FIVE_ANSWERS,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                governors,
            )
            .expect("the query parses and evaluates")
    }

    #[test]
    fn naming_nothing_is_metered() {
        let governors = from_parts(&GovernorParts::default(), None).expect("no conflict");
        assert_eq!(governors.limits(), QueryGovernors::METERED.limits());
        assert!(governors.stop_signal().is_none());
        assert!(governors.is_engaged(), "metering engages every dimension");
    }

    #[test]
    fn each_ceiling_lands_on_its_own_dimension_over_the_metered_base() {
        let parts = GovernorParts {
            fuel: Some(11),
            max_answers: Some(12),
            max_intermediate_cells: Some(13),
            max_scratch_bytes: Some(14),
            max_remote_requests: Some(15),
            no_ceiling: false,
        };
        let limits = from_parts(&parts, None).expect("no conflict").limits();
        let expected = QueryGovernors::METERED
            .with_fuel(11)
            .with_max_answers(12)
            .with_max_intermediate_cells(13)
            .with_max_scratch_bytes(14)
            .with_max_remote_requests(15)
            .limits();
        assert_eq!(limits, expected);

        let only_cells = GovernorParts {
            max_intermediate_cells: Some(0),
            ..GovernorParts::default()
        };
        let limits = from_parts(&only_cells, None)
            .expect("zero is a ceiling")
            .limits();
        assert_eq!(limits.get(ResourceDimension::IntermediateCells), 0);
        assert_eq!(
            limits.get(ResourceDimension::Fuel),
            QueryGovernors::METERED
                .limits()
                .get(ResourceDimension::Fuel),
            "an unnamed dimension stays metered"
        );
    }

    #[test]
    fn no_ceiling_is_unbounded() {
        let parts = GovernorParts {
            no_ceiling: true,
            ..GovernorParts::default()
        };
        let governors = from_parts(&parts, None).expect("no ceiling alone is accepted");
        assert_eq!(governors.limits(), QueryGovernors::UNBOUNDED.limits());
        assert!(!governors.is_engaged(), "nothing is charged");
    }

    #[test]
    fn no_ceiling_keeps_a_stop_signal() {
        let parts = GovernorParts {
            no_ceiling: true,
            ..GovernorParts::default()
        };
        let governors = from_parts(&parts, never()).expect("a stop signal is not a ceiling");
        assert_eq!(governors.limits(), QueryGovernors::UNBOUNDED.limits());
        assert!(governors.stop_signal().is_some());
        let metered = from_parts(&GovernorParts::default(), never()).expect("accepted");
        assert!(metered.stop_signal().is_some());
    }

    #[test]
    fn a_ceiling_beside_no_ceiling_is_refused_naming_its_dimension() {
        for (parts, dimension) in [
            (
                GovernorParts {
                    fuel: Some(1),
                    ..GovernorParts::default()
                },
                ResourceDimension::Fuel,
            ),
            (
                GovernorParts {
                    max_answers: Some(0),
                    ..GovernorParts::default()
                },
                ResourceDimension::AnswerRows,
            ),
            (
                GovernorParts {
                    max_intermediate_cells: Some(u64::MAX),
                    ..GovernorParts::default()
                },
                ResourceDimension::IntermediateCells,
            ),
            (
                GovernorParts {
                    max_scratch_bytes: Some(7),
                    max_remote_requests: Some(8),
                    ..GovernorParts::default()
                },
                ResourceDimension::ScratchBytes,
            ),
            (
                GovernorParts {
                    max_remote_requests: Some(8),
                    ..GovernorParts::default()
                },
                ResourceDimension::RemoteRequests,
            ),
        ] {
            // The same ceiling without `no_ceiling` is accepted.
            assert!(from_parts(&parts, None).is_ok(), "{parts:?}");
            let contradictory = GovernorParts {
                no_ceiling: true,
                ..parts
            };
            assert_eq!(
                from_parts(&contradictory, None).map(|governors| governors.limits()),
                Err(GovernorPartsError::CeilingWithNoCeiling { dimension }),
                "{contradictory:?}"
            );
        }
    }

    #[test]
    fn an_update_refuses_an_answer_cap_and_accepts_every_other_ceiling() {
        let capped = GovernorParts {
            max_answers: Some(10),
            ..GovernorParts::default()
        };
        assert_eq!(
            from_update_parts(&capped, None).map(|governors| governors.limits()),
            Err(GovernorPartsError::AnswerCapOnUpdate)
        );
        assert!(
            from_parts(&capped, None).is_ok(),
            "a query takes the same cap"
        );

        let bounded = GovernorParts {
            fuel: Some(10),
            max_intermediate_cells: Some(10),
            max_scratch_bytes: Some(10),
            max_remote_requests: Some(10),
            ..GovernorParts::default()
        };
        assert_eq!(
            from_update_parts(&bounded, None)
                .expect("an update takes every other ceiling")
                .limits(),
            from_parts(&bounded, None).expect("accepted").limits()
        );
        assert_eq!(
            from_update_parts(&GovernorParts::default(), None)
                .expect("an update naming nothing is metered")
                .limits(),
            QueryGovernors::METERED.limits()
        );
    }

    #[test]
    fn the_refusals_read_as_sentences() {
        assert_eq!(
            GovernorPartsError::CeilingWithNoCeiling {
                dimension: ResourceDimension::Fuel
            }
            .to_string(),
            "a Fuel ceiling was named together with no ceiling at all; name the ceiling, or ask \
             for none"
        );
        assert!(
            GovernorPartsError::AnswerCapOnUpdate
                .to_string()
                .starts_with("an answer cap is not accepted by an UPDATE")
        );
    }

    /// The default base refuses an answer sequence one unit over the cap, and
    /// completes the same query at the cap: ceilings are inclusive.
    #[test]
    fn the_default_refuses_over_cap_and_completes_at_the_cap() {
        let over = from_parts(
            &GovernorParts {
                max_answers: Some(4),
                ..GovernorParts::default()
            },
            None,
        )
        .expect("accepted");
        match run(&over).tripped() {
            Some(TrippedGovernor::Budget {
                dimension, limit, ..
            }) => {
                assert_eq!(dimension, ResourceDimension::AnswerRows);
                assert_eq!(limit, 4);
            }
            other => panic!("five answers over a cap of four trip it, got {other:?}"),
        }

        let at = from_parts(
            &GovernorParts {
                max_answers: Some(5),
                ..GovernorParts::default()
            },
            None,
        )
        .expect("accepted");
        let outcome = run(&at);
        assert!(outcome.is_complete(), "five answers fit a cap of five");
        assert_eq!(
            outcome
                .evidence()
                .consumed()
                .get(ResourceDimension::AnswerRows),
            5
        );
    }

    /// With nothing named the call is metered: it completes and reports what it
    /// spent. With no ceiling it completes and reports nothing, because nothing
    /// was charged.
    #[test]
    fn no_ceiling_accepts_and_the_metered_default_reports_its_cost() {
        let metered = run(&from_parts(&GovernorParts::default(), None).expect("accepted"));
        assert!(metered.is_complete());
        assert_eq!(
            metered
                .evidence()
                .consumed()
                .get(ResourceDimension::AnswerRows),
            5
        );
        assert!(metered.evidence().consumed().get(ResourceDimension::Fuel) > 0);

        let unbounded = run(&from_parts(
            &GovernorParts {
                no_ceiling: true,
                ..GovernorParts::default()
            },
            None,
        )
        .expect("accepted"));
        assert!(unbounded.is_complete());
        assert_eq!(
            unbounded.evidence().consumed().get(ResourceDimension::Fuel),
            0
        );
        assert_eq!(
            unbounded
                .evidence()
                .consumed()
                .get(ResourceDimension::AnswerRows),
            0
        );
    }
}

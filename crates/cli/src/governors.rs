// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `query` and `update` governor flags: what they mean, what they build, and what a
//! trip prints.
//!
//! `purrdf query` reaches the engine's governed lane through six flags — `--fuel`,
//! `--deadline`, `--max-answers`, `--max-intermediate-cells`, `--max-scratch-bytes`, and
//! `--max-remote-requests` — plus `--no-ceiling`. [`GovernorFlags`] is what clap parsed;
//! [`GovernorFlags::to_governors`] is the one place they become a [`QueryGovernors`].
//! `purrdf update` uses the same contract except that `--max-answers` does not apply to a
//! mutation.
//!
//! # The base is `METERED`; `UNBOUNDED` is asked for with `--no-ceiling`
//!
//! [`GovernorFlags::to_governors`] hands the flags to
//! [`purrdf_validate::governors::from_parts`], the decoder every host shares, so a
//! governed run means on the command line what it means through the C ABI, the wasm
//! package and the Python binding. It starts from [`QueryGovernors::METERED`]: a
//! dimension no flag names carries no ceiling anyone asked for, but it is still charged,
//! so the stop signal a `--deadline` installs is polled inside a long-running operator as
//! well as between operators. `--no-ceiling` asks for [`QueryGovernors::UNBOUNDED`]
//! instead — no ceiling and no accounting — and is refused beside any numeric ceiling,
//! because a request that names a ceiling and declines them all contradicts itself.
//! [`GovernorFlags`] has no `Default`: every construction site names every flag.
//!
//! # A deadline is the only flag that is not a number
//!
//! Every other governor is a count the engine charges against. A deadline is a host-owned
//! stop signal: the CLI builds a [`WallDeadline`] — the library's single clock reader —
//! and hands it over as the execution's [`StopSignal`](purrdf_sparql_eval::StopSignal).
//! The engine reads no clock of its own. The deadline starts when
//! [`GovernorFlags::to_governors`] is called, and the `query` lane calls it with the data
//! source already open, immediately before evaluation — so reading and parsing that source
//! are **outside** the budget. Building the configuration earlier would charge a large
//! file's parse time to an evaluation budget, which is a budget no operator could reason
//! about. [`parse_deadline`] documents the accepted spelling; `--deadline`'s own help
//! states the boundary.
//!
//! # The trip report is text, deterministic, and never on stdout
//!
//! [`render_trip`] is the CLI's rendering of a [`BudgetExhausted`]: a banner, the governor
//! that stopped the run, what the rows in hand bound, and the whole per-dimension
//! consumption-and-ceiling vector. It is line-oriented with a fixed field order and reads
//! no clock, so two identical trips render byte-identical bytes.
//!
//! It goes to **stderr**, always, and that is a load-bearing choice rather than a
//! convention borrowed from `--report`. A trip still writes the answers it certified to
//! stdout in the requested serialization, and a caller piping SPARQL-Results JSON or XML
//! into a parser must receive a WELL-FORMED document — so nothing the CLI has to say about
//! the trip may be interleaved into that stream. The three channels carry three different
//! things and none of them can carry another's: stdout holds a valid document, stderr
//! holds the governor report, and the exit code (3, see [`CliOutcome`](crate::error::CliOutcome))
//! is what a shell tests to learn that the document on stdout is a partial answer.

use std::sync::Arc;
use std::time::Duration;

use purrdf_core::ResourceVector;
use purrdf_sparql_eval::{
    BudgetExhausted, GovernorEvidence, PartialAnswers, QueryGovernors, ResourceDimension,
    StopSignal, TrippedGovernor, WallDeadline,
};
use purrdf_validate::governors::{GovernorParts, GovernorPartsError, from_parts};

use crate::error::CliError;

/// The banner line every governor report starts with.
///
/// Versioned so a later change to the field set is visible to a consumer that pinned it,
/// rather than silently reshaping a document they parse.
pub(crate) const GOVERNOR_REPORT_BANNER: &str = "purrdf-governor-report 1";

/// A `query` or `update` subcommand's governor flags, exactly as clap parsed them.
///
/// Every field is `Option`: `None` is "the operator named no ceiling on this dimension",
/// which is the only thing that may become an unbounded dimension. There is deliberately
/// no `Default` — see the module documentation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GovernorFlags {
    /// `--fuel`: abstract execution steps, priced by the engine's charge schedule.
    pub(crate) fuel: Option<u64>,
    /// `--deadline`: the wall-clock evaluation budget, as a [`WallDeadline`].
    pub(crate) deadline: Option<Duration>,
    /// `--max-answers`: units committed to the query form's own answer sequence.
    pub(crate) max_answers: Option<u64>,
    /// `--max-intermediate-cells`: the largest intermediate bag, in `rows * columns`.
    pub(crate) max_intermediate_cells: Option<u64>,
    /// `--max-scratch-bytes`: bytes minted into the per-query scratch arena.
    pub(crate) max_scratch_bytes: Option<u64>,
    /// `--max-remote-requests`: requests issued to a remote or federated endpoint.
    pub(crate) max_remote_requests: Option<u64>,
    /// `--no-ceiling`: decline every ceiling and all accounting
    /// ([`QueryGovernors::UNBOUNDED`] instead of [`QueryGovernors::METERED`]).
    pub(crate) no_ceiling: bool,
}

impl GovernorFlags {
    /// The flags the operator actually named, in declaration order.
    ///
    /// Used to NAME them in a refusal rather than describe them collectively: an operator
    /// told "a governor flag is not accepted here" has to guess which of six, and a
    /// refusal that cannot be acted on is barely better than the silent no-op it replaced.
    pub(crate) fn named(&self) -> Vec<&'static str> {
        let mut named = Vec::new();
        if self.fuel.is_some() {
            named.push("--fuel");
        }
        if self.deadline.is_some() {
            named.push("--deadline");
        }
        if self.max_answers.is_some() {
            named.push("--max-answers");
        }
        if self.max_intermediate_cells.is_some() {
            named.push("--max-intermediate-cells");
        }
        if self.max_scratch_bytes.is_some() {
            named.push("--max-scratch-bytes");
        }
        if self.max_remote_requests.is_some() {
            named.push("--max-remote-requests");
        }
        named
    }

    /// Whether the operator engaged any governor at all.
    ///
    /// `false` selects the ungoverned lane verbatim — the same call, over the same
    /// zero-copy view, that this binary made before governors existed. An ungoverned
    /// query pays nothing for the governed lane's existence, which is only true because
    /// this predicate decides between them rather than a ceiling of `u64::MAX` being
    /// carried through it.
    pub(crate) const fn is_engaged(&self) -> bool {
        self.fuel.is_some()
            || self.deadline.is_some()
            || self.max_answers.is_some()
            || self.max_intermediate_cells.is_some()
            || self.max_scratch_bytes.is_some()
            || self.max_remote_requests.is_some()
    }

    /// Build the engine configuration these flags describe, through
    /// [`purrdf_validate::governors::from_parts`]: [`QueryGovernors::METERED`] with each
    /// named ceiling engaged, or [`QueryGovernors::UNBOUNDED`] under `--no-ceiling`. A
    /// `--deadline` becomes a [`WallDeadline`] constructed **here**, so the budget starts
    /// when the caller of this function is about to evaluate rather than when the process
    /// started.
    ///
    /// # Errors
    ///
    /// A usage error naming both flags when `--no-ceiling` is given beside a numeric
    /// ceiling.
    pub(crate) fn to_governors(self) -> Result<QueryGovernors, CliError> {
        let parts = GovernorParts {
            fuel: self.fuel,
            max_answers: self.max_answers,
            max_intermediate_cells: self.max_intermediate_cells,
            max_scratch_bytes: self.max_scratch_bytes,
            max_remote_requests: self.max_remote_requests,
            no_ceiling: self.no_ceiling,
        };
        let stop = self
            .deadline
            .map(|budget| Arc::new(WallDeadline::after(budget)) as Arc<dyn StopSignal>);
        from_parts(&parts, stop).map_err(|error| CliError::Usage(refusal(error)))
    }
}

/// The command line's wording of a [`GovernorPartsError`], in its own flag spelling.
fn refusal(error: GovernorPartsError) -> String {
    match error {
        GovernorPartsError::CeilingWithNoCeiling { dimension } => format!(
            "--no-ceiling declines every ceiling, and {} names one: drop --no-ceiling to run \
             under the ceiling, or drop the ceiling to run with none",
            ceiling_flag(dimension)
        ),
        other => other.to_string(),
    }
}

/// The flag that names a ceiling on `dimension`.
fn ceiling_flag(dimension: ResourceDimension) -> String {
    match dimension {
        ResourceDimension::Fuel => "--fuel".to_owned(),
        ResourceDimension::AnswerRows => "--max-answers".to_owned(),
        ResourceDimension::IntermediateCells => "--max-intermediate-cells".to_owned(),
        ResourceDimension::ScratchBytes => "--max-scratch-bytes".to_owned(),
        ResourceDimension::RemoteRequests => "--max-remote-requests".to_owned(),
        other => format!("a {} ceiling", other.label()),
    }
}

/// The ceiling on `dimension` a trip report prints: the one a flag named, or `None` for a
/// dimension that carries none — unbounded, or bounded only by the ceiling
/// [`QueryGovernors::METERED`] installs so its counter runs, which no flag asked for.
fn named_limit(limits: ResourceVector, dimension: ResourceDimension) -> Option<u64> {
    let metering = QueryGovernors::METERED.limits();
    let limit = limits.get(dimension);
    (limits.is_bounded(dimension)
        && !(metering.is_bounded(dimension) && limit == metering.get(dimension)))
    .then_some(limit)
}

/// Parse `--deadline`'s human duration: a non-empty run of `<count><unit>` components.
///
/// The accepted units are `ms`, `s`, `m` and `h`, and components sum, so `90s`, `1m30s`
/// and `90000ms` all name the same budget. Whitespace is not accepted anywhere, so one
/// shell word is always one duration. A bare number is refused rather than assumed to be
/// seconds: a deadline is the one flag whose unit a caller cannot infer from the value,
/// and guessing wrong is the difference between a millisecond and a quarter of an hour.
///
/// Total and allocation-light: one pass over the bytes, no regular expression, no locale.
///
/// # Errors
///
/// A string that is empty, carries a component without a count or without a unit, names a
/// unit outside the four, or describes a budget larger than [`u64::MAX`] milliseconds.
pub(crate) fn parse_deadline(text: &str) -> Result<Duration, String> {
    if text.is_empty() {
        return Err(deadline_syntax("it is empty"));
    }
    let mut millis: u64 = 0;
    let mut rest = text;
    while !rest.is_empty() {
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits == 0 {
            return Err(deadline_syntax(&format!(
                "`{rest}` starts a component with no count"
            )));
        }
        let (count, tail) = rest.split_at(digits);
        let unit_len = tail.len()
            - tail
                .trim_start_matches(|c: char| c.is_ascii_alphabetic())
                .len();
        if unit_len == 0 {
            return Err(deadline_syntax(&format!("`{count}` carries no unit")));
        }
        let (unit, tail) = tail.split_at(unit_len);
        let scale = match unit {
            "ms" => 1_u64,
            "s" => 1_000,
            "m" => 60_000,
            "h" => 3_600_000,
            other => return Err(deadline_syntax(&format!("`{other}` is not a unit"))),
        };
        let count: u64 = count
            .parse()
            .map_err(|_| deadline_syntax(&format!("`{count}` does not fit")))?;
        millis = count
            .checked_mul(scale)
            .and_then(|component| millis.checked_add(component))
            .ok_or_else(|| deadline_syntax("it is longer than this clock can express"))?;
        rest = tail;
    }
    Ok(Duration::from_millis(millis))
}

/// The one wording of a `--deadline` syntax refusal, so every rejection teaches the
/// accepted spelling rather than only naming what was wrong with this one.
fn deadline_syntax(problem: &str) -> String {
    format!(
        "{problem}: a deadline is a run of count+unit components over `ms`, `s`, `m`, `h` \
         — e.g. `750ms`, `30s`, `1m30s`, `2h`"
    )
}

/// Render a budget trip as the deterministic, line-oriented governor report.
///
/// The field order is fixed and every value is a counter, a pinned label, or an algebra
/// variant name, so the same trip over the same data renders the same bytes. Read by a
/// human on stderr and parseable by a shell: one `key value` pair per line, the banner
/// first.
///
/// The `answers` line is the certificate, restated: `certain` licenses the rows on stdout
/// as answers, `at-most` licenses only the negative reading (a row absent from them is
/// definitively not an answer), and `withheld` means no bound survived — in which case
/// there are structurally no rows, stdout carries nothing, and the `barrier` line names
/// the operator that withheld them.
pub(crate) fn render_trip(exhausted: &BudgetExhausted) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out, "{GOVERNOR_REPORT_BANNER}");
    let _ = writeln!(out, "outcome budget-exhausted");
    let _ = writeln!(out, "tripped {}", exhausted.tripped.label());
    let _ = writeln!(out, "detail {}", exhausted.tripped);
    match &exhausted.partial {
        PartialAnswers::Certain(partial) => {
            let _ = writeln!(out, "answers certain");
            let _ = writeln!(out, "positional-prefix {}", partial.is_positional_prefix());
        }
        PartialAnswers::AtMost(partial) => {
            let _ = writeln!(out, "answers at-most");
            let _ = writeln!(out, "positional-prefix {}", partial.is_positional_prefix());
        }
        PartialAnswers::Unknown(barrier) => {
            let _ = writeln!(out, "answers withheld");
            let _ = writeln!(out, "barrier {}", barrier.operator());
        }
    }
    for dimension in ResourceDimension::ALL {
        let _ = writeln!(
            out,
            "consumed {} {}",
            dimension.label(),
            exhausted.evidence.consumed_in(dimension)
        );
    }
    for dimension in ResourceDimension::ALL {
        let _ = match named_limit(exhausted.evidence.limits(), dimension) {
            Some(limit) => writeln!(out, "limit {} {limit}", dimension.label()),
            None => writeln!(out, "limit {} unbounded", dimension.label()),
        };
    }
    out
}

/// Render a governed UPDATE trip. The request is atomic, so there is deliberately no
/// partial-result vocabulary: `mutation none` is the complete mutation receipt.
pub(crate) fn render_update_trip(tripped: TrippedGovernor, evidence: &GovernorEvidence) -> String {
    render_all_or_nothing_trip("update", "mutation none", tripped, evidence)
}

/// Render a governed VALIDATION trip.
///
/// Shares [`render_update_trip`]'s shape because it shares its situation: SHACL validation is
/// all-or-nothing for exactly the reason an UPDATE is. Every SHACL constraint is a NEGATIVE
/// claim — "no solution of this query violates the shape" — so a truncated solution bag and a
/// complete one that found nothing yield the identical sentence, and the engine's own
/// `GovernedValidation` refuses to hand back a partial report because of it. `report none` is
/// therefore the complete receipt, exactly as `mutation none` is: there is no partial-answer
/// vocabulary to print, because there is structurally no partial answer.
pub(crate) fn render_validation_trip(
    tripped: TrippedGovernor,
    evidence: &GovernorEvidence,
) -> String {
    render_all_or_nothing_trip("validate", "report none", tripped, evidence)
}

/// The shared rendering for an operation whose governed outcome is all-or-nothing.
///
/// `operation` names the verb and `effect` is its one-line receipt for having produced
/// nothing. The banner, the `key value` grammar and the full per-dimension
/// consumption-and-ceiling vector are identical to [`render_trip`]'s, so a shell parses one
/// governor report regardless of which subcommand emitted it.
fn render_all_or_nothing_trip(
    operation: &str,
    effect: &str,
    tripped: TrippedGovernor,
    evidence: &GovernorEvidence,
) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out, "{GOVERNOR_REPORT_BANNER}");
    let _ = writeln!(out, "outcome budget-exhausted");
    let _ = writeln!(out, "operation {operation}");
    let _ = writeln!(out, "tripped {}", tripped.label());
    let _ = writeln!(out, "detail {tripped}");
    let _ = writeln!(out, "{effect}");
    for dimension in ResourceDimension::ALL {
        let _ = writeln!(
            out,
            "consumed {} {}",
            dimension.label(),
            evidence.consumed_in(dimension)
        );
    }
    for dimension in ResourceDimension::ALL {
        let _ = match named_limit(evidence.limits(), dimension) {
            Some(limit) => writeln!(out, "limit {} {limit}", dimension.label()),
            None => writeln!(out, "limit {} unbounded", dimension.label()),
        };
    }
    out
}

/// Render a CLOSURE stop as the same deterministic, line-oriented governor report.
///
/// The `--entailment` lane can be stopped in a place [`render_trip`] has no vocabulary for:
/// while the regime's closure is still being MATERIALIZED, before any query was evaluated.
/// There are then no answers, no bound on any, and no consumption vector — the SPARQL
/// evaluator never ran, so it charged nothing and reporting its zeroes would describe an
/// execution that did not happen.
///
/// So the report says exactly the four things that are true, under the same banner and the
/// same `key value` grammar a shell already parses: the outcome, which signal fired, its
/// prose, and the fact that no query was evaluated. `answers withheld` is the honest reading
/// of "there are no rows on stdout" — the same word [`render_trip`] uses when no bound
/// survived — and the `barrier` line names the phase rather than an algebra operator,
/// because the phase is what withheld them.
pub(crate) fn render_closure_stop(tripped: TrippedGovernor) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out, "{GOVERNOR_REPORT_BANNER}");
    let _ = writeln!(out, "outcome budget-exhausted");
    let _ = writeln!(out, "tripped {}", tripped.label());
    let _ = writeln!(out, "detail {tripped}");
    let _ = writeln!(out, "answers withheld");
    let _ = writeln!(out, "barrier entailment-closure");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every accepted spelling of a duration, and the millisecond budget it names.
    #[test]
    fn a_deadline_is_a_sum_of_count_unit_components() {
        for (text, millis) in [
            ("0ms", 0),
            ("750ms", 750),
            ("30s", 30_000),
            ("1m30s", 90_000),
            ("2h", 7_200_000),
            ("1h1m1s1ms", 3_661_001),
        ] {
            assert_eq!(
                parse_deadline(text).expect("the spelling is accepted"),
                Duration::from_millis(millis),
                "`{text}`"
            );
        }
    }

    /// A bare number, an unknown unit, a missing count, and an empty string are all
    /// refused — and every refusal teaches the accepted spelling.
    #[test]
    fn a_deadline_without_a_unit_is_refused_rather_than_guessed() {
        for text in ["", "30", "30x", "ms", "1m30", "-5s"] {
            let error = parse_deadline(text).expect_err("the spelling is refused");
            assert!(
                error.contains("`750ms`") && error.contains("`1m30s`"),
                "the refusal of `{text}` must teach the spelling: {error}"
            );
        }
    }

    /// A budget past the clock's range is refused rather than wrapped into a short one.
    #[test]
    fn an_unrepresentable_deadline_is_refused_rather_than_wrapped() {
        let error = parse_deadline("18446744073709551615h").expect_err("it cannot be expressed");
        assert!(error.contains("longer than this clock"), "{error}");
    }

    /// The flags with nothing named.
    const NONE: GovernorFlags = GovernorFlags {
        fuel: None,
        deadline: None,
        max_answers: None,
        max_intermediate_cells: None,
        max_scratch_bytes: None,
        max_remote_requests: None,
        no_ceiling: false,
    };

    /// `METERED` is what no flag means, and each flag lands on its own dimension.
    #[test]
    fn flags_engage_exactly_the_dimensions_they_name() {
        assert!(!NONE.is_engaged());
        assert_eq!(NONE.named(), [] as [&str; 0]);
        let metered = NONE.to_governors().expect("nothing to refuse");
        assert_eq!(metered.limits(), QueryGovernors::METERED.limits());
        assert!(metered.stop_signal().is_none());

        let all = GovernorFlags {
            fuel: Some(11),
            deadline: Some(Duration::from_millis(12)),
            max_answers: Some(13),
            max_intermediate_cells: Some(14),
            max_scratch_bytes: Some(15),
            max_remote_requests: Some(16),
            no_ceiling: false,
        };
        assert!(all.is_engaged());
        assert_eq!(
            all.named(),
            vec![
                "--fuel",
                "--deadline",
                "--max-answers",
                "--max-intermediate-cells",
                "--max-scratch-bytes",
                "--max-remote-requests",
            ]
        );
        let governors = all.to_governors().expect("nothing to refuse");
        let limits = governors.limits();
        assert_eq!(limits.get(ResourceDimension::Fuel), 11);
        assert_eq!(limits.get(ResourceDimension::AnswerRows), 13);
        assert_eq!(limits.get(ResourceDimension::IntermediateCells), 14);
        assert_eq!(limits.get(ResourceDimension::ScratchBytes), 15);
        assert_eq!(limits.get(ResourceDimension::RemoteRequests), 16);
        assert!(
            governors.stop_signal().is_some(),
            "a --deadline is a stop signal, not a ceiling"
        );

        // A dimension no flag named carries no named ceiling even when its neighbours are
        // capped: it is metered, and the report prints it as unbounded.
        let only_fuel = GovernorFlags {
            fuel: Some(7),
            ..NONE
        };
        let limits = only_fuel
            .to_governors()
            .expect("nothing to refuse")
            .limits();
        assert_eq!(named_limit(limits, ResourceDimension::Fuel), Some(7));
        assert_eq!(named_limit(limits, ResourceDimension::AnswerRows), None);
        assert_eq!(
            named_limit(limits, ResourceDimension::IntermediateCells),
            None
        );
    }

    /// `--no-ceiling` alone is `UNBOUNDED`, and keeps a `--deadline` beside it.
    #[test]
    fn no_ceiling_declines_every_ceiling_and_keeps_the_deadline() {
        let unbounded = GovernorFlags {
            no_ceiling: true,
            ..NONE
        }
        .to_governors()
        .expect("nothing named beside it");
        assert_eq!(unbounded.limits(), QueryGovernors::UNBOUNDED.limits());
        assert!(!unbounded.is_engaged());

        let with_deadline = GovernorFlags {
            no_ceiling: true,
            deadline: Some(Duration::from_secs(60)),
            ..NONE
        }
        .to_governors()
        .expect("a deadline is not a ceiling");
        assert_eq!(with_deadline.limits(), QueryGovernors::UNBOUNDED.limits());
        assert!(with_deadline.stop_signal().is_some());
    }

    /// `--no-ceiling` beside a numeric ceiling is refused, naming both flags.
    #[test]
    fn no_ceiling_beside_a_ceiling_is_refused_by_flag_name() {
        for (flags, flag) in [
            (
                GovernorFlags {
                    fuel: Some(1),
                    ..NONE
                },
                "--fuel",
            ),
            (
                GovernorFlags {
                    max_answers: Some(1),
                    ..NONE
                },
                "--max-answers",
            ),
            (
                GovernorFlags {
                    max_remote_requests: Some(1),
                    ..NONE
                },
                "--max-remote-requests",
            ),
        ] {
            let refused = GovernorFlags {
                no_ceiling: true,
                ..flags
            }
            .to_governors()
            .expect_err("a ceiling and no ceiling contradict");
            let CliError::Usage(message) = refused else {
                panic!("a usage error, not {refused:?}");
            };
            assert!(
                message.contains("--no-ceiling") && message.contains(flag),
                "{message}"
            );
            // The neighbour without `--no-ceiling` runs under the ceiling.
            assert!(flags.to_governors().is_ok(), "{flag} alone is accepted");
        }
    }
}

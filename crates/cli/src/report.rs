// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Surfacing the reasoning report under the `--report` flag.
//!
//! [`surface`] renders a [`ReasoningReport`] exactly where the flag's decoded
//! [`ReportTarget`] directs: nowhere, stderr, or a file. The rendering is a line-oriented
//! text block whose field order is fixed, so two identical runs produce byte-identical
//! bytes and a diff over two runs is a diff over what the reasoner did.
//!
//! # What the operator gets, and why it is not optional
//!
//! `purrdf reason --regime owl-rl in.ttl out.ttl` used to hand back a document with no
//! provenance whatsoever: nothing in it distinguished a closure under every rule the regime
//! defines from a closure under a subset, and nothing said that the named graphs went
//! unread or that a conclusion had been withheld. The report is where each of those is a
//! LINE rather than an assumption — the rules that fired and their counts, the constructs
//! the run could not fully handle WITH the technical reason, the evaluation cost, the
//! contract hash of the calculus, the withheld-surrogate count, and the overclaim verdict
//! the report may never fail.
//!
//! # Why this rendering rather than a shared one
//!
//! `purrdf_validate::regime::render_reasoning_report` renders the same fields for the
//! string-in/string-out boundary the Python, C-ABI and WASM hosts share, and the line
//! grammar below is deliberately the same one (`regime`, `completeness`, `missing`,
//! `fired`, `boundary`, `budget`, `contract-hash`, `inconsistency`, `overclaims`), so the
//! two are read the same way. Calling it directly would mean a new `purrdf-validate`
//! dependency edge for this crate — a Cargo.lock change under a `--locked` gate, and a
//! dependency this binary otherwise does not need — so the grammar is shared and the code
//! is not.

use std::fmt::Write as _;

use purrdf_entail::{Completeness, ReasoningReport};

use crate::cli::ReportTarget;
use crate::error::CliError;

/// Surface `report` per the decoded `--report` target.
///
/// * [`ReportTarget::Silent`] — emit nothing.
/// * [`ReportTarget::Stderr`] — write the rendering to stderr, leaving stdout for the data
///   (so `purrdf reason … - --report` still pipes cleanly).
/// * [`ReportTarget::File`] — write the rendering to the given path.
pub(crate) fn surface(target: &ReportTarget, report: &ReasoningReport) -> Result<(), CliError> {
    match target {
        ReportTarget::Silent => Ok(()),
        ReportTarget::Stderr => {
            eprint!("{}", render(report));
            Ok(())
        }
        ReportTarget::File(path) => {
            std::fs::write(path, render(report))?;
            Ok(())
        }
    }
}

/// The refusal for `--report` without a regime to report on.
///
/// `convert` and `query` reason only under `--entailment`; asked for the certificate of a
/// run that will not happen, they say so rather than writing an empty file or ignoring the
/// flag.
pub(crate) fn requires_entailment(subcommand: &str) -> CliError {
    CliError::Usage(format!(
        "--report asks for the reasoning report of a run `{subcommand}` was not asked to \
         make: add --entailment REGIME, or drop --report"
    ))
}

/// Render `report` as a deterministic, line-oriented text block.
///
/// One line per fact, in a fixed field order, each line ending in `\n`; the block therefore
/// ends with a newline. Every sequence the report hands out is already in a documented
/// order (rules in specification table order, boundaries in construct declaration order),
/// so nothing here sorts and nothing here iterates a map.
fn render(report: &ReasoningReport) -> String {
    let mut out = String::new();
    // `write!` to a `String` cannot fail, so the results are discarded rather than
    // propagated: there is no I/O here to have an error about.
    let _ = writeln!(out, "regime {}", regime_token(report.regime()));
    match report.completeness() {
        Completeness::Exact => {
            let _ = writeln!(out, "completeness exact");
        }
        Completeness::ExactWithinBoundaries => {
            let _ = writeln!(out, "completeness exact-within-boundaries");
        }
        Completeness::SoundIncomplete { missing } => {
            let _ = writeln!(out, "completeness sound-incomplete {}", missing.len());
        }
    }
    for rule in report.completeness().missing() {
        let _ = writeln!(out, "missing {}", rule.as_str());
    }
    for &(rule, count) in report.rules_fired() {
        let _ = writeln!(out, "fired {} {count}", rule.as_str());
    }
    for boundary in report.boundaries() {
        let _ = writeln!(
            out,
            "boundary {} {}",
            boundary.construct().as_str(),
            boundary.reason()
        );
    }
    let budget = report.budget();
    let _ = writeln!(out, "budget join-steps {}", budget.join_steps());
    let _ = writeln!(out, "budget stored-facts {}", budget.stored_facts());
    let _ = writeln!(out, "budget term-arena-bytes {}", budget.term_arena_bytes());
    let _ = writeln!(out, "contract-hash {}", report.contract_hash().to_hex());
    let _ = writeln!(out, "withheld-surrogates {}", report.withheld_surrogates());
    match report.inconsistency() {
        None => {
            let _ = writeln!(out, "inconsistency none");
        }
        Some(witness) => {
            let _ = writeln!(
                out,
                "inconsistency {} premises {}",
                witness.rule().as_str(),
                witness.premises().len()
            );
        }
    }
    let _ = writeln!(out, "overclaims {}", report.overclaims());
    out
}

/// The CLI spelling of a regime — the same token `--regime` / `--entailment` accept, so a
/// report names the regime the way the operator asked for it.
fn regime_token(regime: purrdf_entail::Regime) -> &'static str {
    use purrdf_entail::Regime;
    match regime {
        Regime::Simple => "simple",
        Regime::Rdf => "rdf",
        Regime::Rdfs => "rdfs",
        Regime::OwlRl => "owl-rl",
        Regime::OwlDirect => "owl-direct",
        Regime::Rif => "rif",
        Regime::D => "d",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;
    use purrdf_entail::{Regime, materialize};

    /// A dataset with a schema, an instance, and a quad outside the default graph.
    fn fixture() -> std::sync::Arc<purrdf_core::RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let cat = b.intern_iri("http://example.org/Cat");
        let animal = b.intern_iri("http://example.org/Animal");
        let tom = b.intern_iri("http://example.org/tom");
        let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
        let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let g = b.intern_iri("http://example.org/g");
        b.push_quad(cat, sub, animal, None);
        b.push_quad(tom, ty, cat, None);
        b.push_quad(tom, ty, animal, Some(g));
        b.freeze().expect("freeze")
    }

    /// THE RENDERING CARRIES THE EVIDENCE, and it is deterministic.
    #[test]
    fn the_rendering_names_every_field_and_repeats_byte_for_byte() {
        let (_, report) = materialize(&fixture(), Regime::Rdfs).expect("rdfs");
        let rendered = render(&report);
        assert!(rendered.starts_with("regime rdfs\n"), "{rendered}");
        assert!(rendered.contains("\nfired rdfs9 "), "{rendered}");
        // The boundary line carries the construct AND its technical reason.
        assert!(rendered.contains("\nboundary named-graph "), "{rendered}");
        assert!(rendered.contains("\ncontract-hash "), "{rendered}");
        assert!(rendered.contains("\nwithheld-surrogates "), "{rendered}");
        assert!(rendered.ends_with("overclaims false\n"), "{rendered}");
        assert_eq!(rendered, render(&report));
    }

    /// Every regime renders under the token the command line uses for it.
    #[test]
    fn the_regime_token_is_the_command_line_spelling() {
        for (regime, token) in [
            (Regime::Simple, "simple"),
            (Regime::Rdf, "rdf"),
            (Regime::Rdfs, "rdfs"),
            (Regime::OwlRl, "owl-rl"),
            (Regime::OwlDirect, "owl-direct"),
            (Regime::Rif, "rif"),
            (Regime::D, "d"),
        ] {
            assert_eq!(regime_token(regime), token);
        }
    }
}

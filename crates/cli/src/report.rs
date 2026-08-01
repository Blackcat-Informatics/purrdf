// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Surfacing the reasoning report under the `--report` flag.
//!
//! [`surface`] writes a [`ReasoningReport`] exactly where the flag's decoded
//! [`ReportTarget`] directs: nowhere, stderr, or a file. The rendering is
//! [`purrdf_validate::regime::render_reasoning_report`] — a line-oriented text block whose
//! field order is fixed, so two identical runs produce byte-identical bytes and a diff
//! over two runs is a diff over what the reasoner did.
//!
//! # What the operator gets, and why it is not optional
//!
//! `purrdf reason --regime owl-rl in.ttl out.ttl` used to hand back a document with no
//! provenance whatsoever: nothing in it distinguished a closure under every rule the regime
//! defines from a closure under a subset, and nothing said that the named graphs went
//! unread or that a conclusion had been withheld. The report is where each of those is a
//! LINE rather than an assumption — the rules that fired and their counts, the constructs
//! the run could not fully handle WITH the technical reason, the evaluation cost, the
//! contract hash of the calculus, and the withheld-surrogate count.
//!
//! # There is ONE renderer, and this is not it
//!
//! This module used to carry a second, private renderer, whose own header claimed it
//! "renders the same fields" as `purrdf_validate::regime::render_reasoning_report`. That
//! was false in both directions: it omitted the format banner and it emitted a
//! `withheld-surrogates` line the shared renderer did not, so the CLI and the three
//! bindings disagreed about what a report even contains, and nothing compared them. The
//! duplicate is gone. The shared renderer emits the withheld count now, and this module
//! calls it.
//!
//! The reason the duplicate was tolerated was a dependency edge — reaching the shared
//! renderer means `purrdf-cli` depending on `purrdf-validate`. It is a path dependency on
//! a workspace member this repository already builds, adds no third-party crate, and is a
//! smaller cost than two renderers with no gate between them.
//!
//! # A refusal is reported too
//!
//! [`materialize_reported`] is the only way this binary materializes. An INCONSISTENT
//! knowledge base has no closure — it entails every triple — but it did have a RUN, and
//! `purrdf_entail::EntailError::Inconsistent` carries that run's report. The `--report`
//! target is therefore written on the refusal path as well, so `--report FILE` produces a
//! file whichever way the command exits, and the operator learns which rule refused, on
//! which triples, after how much work. Writing nothing was the previous behaviour and it
//! left the one operator who most needed the certificate with only an exit code.

use purrdf_core::RdfDataset;
use purrdf_entail::{EntailError, Materialization, ReasoningReport, materialize};
use purrdf_validate::regime::render_reasoning_report;
use std::sync::Arc;

use crate::cli::ReportTarget;
use crate::error::CliError;

/// Materialize `plan` over `dataset`, surfacing the run's report to `target` either way.
///
/// The success path writes the report beside the closure. The INCONSISTENT path writes the
/// report too and then returns the refusal: the run happened, cost a budget, fired rules
/// and named a calculus, and every one of those is something the operator needs in order
/// to act on the refusal.
///
/// Every other [`EntailError`] is the absence of a run — an exhausted ceiling, a malformed
/// rule document, an unsatisfiable tableau — with no report to write, so nothing is
/// surfaced and nothing is implied about a closure that was never assembled.
pub(crate) fn materialize_reported(
    dataset: &RdfDataset,
    plan: Materialization<'_>,
    target: &ReportTarget,
) -> Result<Arc<RdfDataset>, CliError> {
    match materialize(dataset, plan) {
        Ok((closure, report)) => {
            surface(target, &report)?;
            Ok(closure)
        }
        Err(EntailError::Inconsistent(run)) => {
            surface(target, run.report())?;
            Err(CliError::Runtime(
                EntailError::Inconsistent(run).to_string(),
            ))
        }
        Err(other) => Err(other.into()),
    }
}

/// Surface `report` per the decoded `--report` target.
///
/// * [`ReportTarget::Silent`] — emit nothing.
/// * [`ReportTarget::Stderr`] — write the rendering to stderr, leaving stdout for the data
///   (so `purrdf reason … - --report` still pipes cleanly).
/// * [`ReportTarget::File`] — write the rendering to the given path.
///
/// Reachable from the `query` lane as well as from [`materialize_reported`]: that lane
/// materializes THROUGH `purrdf::query_with_entailment` (the only entry point that runs the
/// query-directed combined approach), so it holds the report rather than obtaining it here,
/// and it surfaces it through this one function so the two lanes cannot render differently.
pub(crate) fn surface(target: &ReportTarget, report: &ReasoningReport) -> Result<(), CliError> {
    surface_rendered(target, &render_reasoning_report(report))
}

/// Surface an ALREADY-RENDERED certificate per the decoded `--report` target.
///
/// The same three states [`surface`] decodes, one level lower. It exists because
/// `purrdf-validate`'s conclusion-directed services hand back the certificate as the string
/// they already rendered — the `entails` lane never holds a [`ReasoningReport`] value at all
/// — and the alternative was a second `--report` convention for one subcommand. There is one
/// convention, and both lanes end here.
pub(crate) fn surface_rendered(target: &ReportTarget, rendered: &str) -> Result<(), CliError> {
    match target {
        ReportTarget::Silent => Ok(()),
        ReportTarget::Stderr => {
            eprint!("{rendered}");
            Ok(())
        }
        ReportTarget::File(path) => {
            std::fs::write(path, rendered)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;
    use purrdf_entail::Regime;
    use purrdf_validate::regime::{REPORT_FORMAT_BANNER, regime_name};

    /// A dataset with a schema, an instance, and a quad outside the default graph.
    fn fixture() -> Arc<RdfDataset> {
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

    /// A dataset OWL 2 RL's `cax-dw` refuses: two disjoint classes, one shared instance.
    fn inconsistent() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/A");
        let c = b.intern_iri("http://example.org/B");
        let x = b.intern_iri("http://example.org/x");
        let disjoint = b.intern_iri("http://www.w3.org/2002/07/owl#disjointWith");
        let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        b.push_quad(a, disjoint, c, None);
        b.push_quad(x, ty, a, None);
        b.push_quad(x, ty, c, None);
        b.freeze().expect("freeze")
    }

    /// THE RENDERING CARRIES THE EVIDENCE, and it is the SHARED one.
    ///
    /// The banner is the load-bearing assertion: the private renderer this replaced did
    /// not emit it, so its presence is what proves the CLI and the bindings now read the
    /// same bytes rather than two grammars described as the same one.
    #[test]
    fn the_rendering_is_the_shared_one_and_names_every_field() {
        let (_, report) = materialize(&fixture(), Materialization::Rdfs).expect("rdfs");
        let rendered = render_reasoning_report(&report);
        assert!(
            rendered.starts_with(&format!("{REPORT_FORMAT_BANNER}\nregime rdfs\n")),
            "{rendered}"
        );
        assert!(rendered.contains("\nfired rdfs9 "), "{rendered}");
        // The boundary line carries the construct AND its technical reason.
        assert!(rendered.contains("\nboundary named-graph "), "{rendered}");
        assert!(rendered.contains("\ncontract-hash "), "{rendered}");
        assert!(rendered.contains("\nwithheld-surrogates "), "{rendered}");
        assert!(rendered.ends_with("inconsistency none\n"), "{rendered}");
        assert_eq!(rendered, render_reasoning_report(&report));
    }

    /// The `--report` file is written on the INCONSISTENT path, and carries the witness.
    #[test]
    fn an_inconsistent_run_still_writes_its_report() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("report.txt");
        let error = materialize_reported(
            &inconsistent(),
            Materialization::OwlRl,
            &ReportTarget::File(path.clone()),
        )
        .expect_err("cax-dw refuses");
        assert!(error.to_string().contains("cax-dw"), "{error}");
        let written = std::fs::read_to_string(&path).expect("the report was written");
        assert!(written.starts_with(REPORT_FORMAT_BANNER), "{written}");
        assert!(
            written.contains("\ninconsistency cax-dw premises 3\n"),
            "{written}"
        );
        assert!(
            written.contains("\ninconsistency-graph default\n"),
            "{written}"
        );
        assert_eq!(
            written.matches("\ninconsistency-premise ").count(),
            3,
            "the three asserted triples that satisfied the rule: {written}"
        );
    }

    /// Every regime renders under the token the command line uses for it.
    ///
    /// The CLI's `--regime` / `--entailment` spellings and the shared renderer's `regime`
    /// line are one vocabulary, which is why this binary no longer keeps a private map.
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
            assert_eq!(regime_name(regime), token);
        }
    }
}

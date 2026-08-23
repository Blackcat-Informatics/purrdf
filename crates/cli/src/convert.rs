// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `convert` subcommand: `Source → [entail] → [canonicalize] → Sink`.
//!
//! Resolve the source and target formats, open the source as a view, and write it
//! through the [`sink`]. A pack→pack conversion is a verified byte
//! passthrough (re-encoding a pack would be pointless churn); every other
//! combination flows the source view into [`sink::write_rdf`], which serializes to
//! the target syntax or rebuilds the pack container. The resulting loss ledger is
//! surfaced under `--loss-ledger`.
//!
//! ## Transforms: `--entailment` and `--canonical`
//!
//! Two optional transforms compose in a fixed order (entail first, then
//! canonicalize), and both need a concrete owned dataset — so when either is
//! present the pipeline reconstructs an `Arc<RdfDataset>` up front (a text source is
//! parsed, a pack source is rebuilt via [`source::load_dataset`]) instead of taking
//! the zero-copy view path:
//!
//! * `--entailment REGIME` materializes the regime's closure in memory (through the
//!   same `EntailmentPlan` `reason` resolves, so `--rules` means the same thing
//!   here), and its reasoning report is surfaced under `--report`.
//! * `--canonical` emits the RDFC-1.0 canonical N-Quads document
//!   ([`canonical_flat_nquads`]) rather than the `--to` format. Canonical output is
//!   always N-Quads, so `--canonical` lets you OMIT `--to` — and refuses it by name
//!   (see [`run`]) rather than silently ignoring a `--to` naming a different format,
//!   exactly as it refuses `--jsonld-options`.

use std::sync::Arc;

use purrdf_core::{DatasetView, LossLedger, RdfDataset};
use purrdf_rdf::JsonLdSerializeOptions;
use purrdf_rdf::SourceFormat;
use purrdf_rdf::canonical_flat_nquads;

use crate::cli::{CliRdfFormat, CliRegime, LedgerTarget, ReportTarget};
use crate::error::CliError;
use crate::format;
use crate::ledger;
use crate::reason;
use crate::report;
use crate::sink;
use crate::source::{self, ViewOp};

/// The generic sink operation for a convert: serialize whichever concrete view the
/// source resolved to into the target format.
struct ConvertOp<'a> {
    out: &'a str,
    target: SourceFormat,
    base: Option<&'a str>,
    src_codec: Option<&'a str>,
    jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

impl ViewOp for ConvertOp<'_> {
    type Output = LossLedger;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<LossLedger, CliError> {
        sink::write_rdf(
            view,
            self.out,
            self.target,
            self.base,
            self.src_codec,
            self.jsonld_options,
        )
    }
}

/// The resolved `convert` flags: the format overrides plus the parse base and the
/// two optional transforms. Grouping them keeps [`run`]'s signature small and lets
/// the transform lane borrow them by reference.
pub(crate) struct ConvertOptions<'a> {
    /// The `--from` input-format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// The `--to` output-format override.
    pub(crate) to: Option<CliRdfFormat>,
    /// The `--base` parse/serialize base IRI.
    pub(crate) base: Option<&'a str>,
    /// The `--entailment` regime to materialize before serializing.
    pub(crate) entailment: Option<CliRegime>,
    /// `--rules`: the RIF-in-XML rule document `--entailment rif` runs.
    pub(crate) rules: Option<&'a std::path::Path>,
    /// Whether `--canonical` was set (emit RDFC-1.0 canonical N-Quads).
    pub(crate) canonical: bool,
    /// Explicit JSON-LD/YAML-LD serialization configuration.
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// Run the `convert` subcommand.
pub(crate) fn run(
    options: &ConvertOptions<'_>,
    input: &str,
    output: &str,
    ledger_target: &LedgerTarget,
    report_target: &ReportTarget,
) -> Result<(), CliError> {
    let source_format = format::resolve(options.from, input)?;
    format::refuse_base_with_pack(source_format, options.base, "a pack --from source")?;
    if options.canonical && options.jsonld_options.is_some() {
        return Err(CliError::Usage(
            "--jsonld-options cannot be combined with --canonical".to_owned(),
        ));
    }
    // `--canonical` output is always N-Quads, so a `--to` naming a different target
    // format is never read on this lane (see `run_with_transforms`, which resolves
    // `--to` only in its non-canonical arm) — the same silent-no-op shape
    // `--jsonld-options` is refused for just above, rather than an override a caller
    // could mistake for a format that was honored.
    if options.canonical && options.to.is_some() {
        return Err(CliError::Usage(
            "--canonical emits RDFC-1.0 canonical N-Quads unconditionally, so a --to \
             naming a different target format would be accepted and never read: drop \
             one of the two"
                .to_owned(),
        ));
    }
    // `--report` names the certificate of a reasoning run; without `--entailment` there is
    // no run to certify, and answering that with silence is the shape this pipeline
    // refuses everywhere else.
    if report_target.is_requested() && options.entailment.is_none() {
        return Err(report::requires_entailment("convert"));
    }
    // `--rules` names the RIF-in-XML rule document `--entailment rif` runs; without
    // `--entailment` at all, `options.rules` is never even read (only the
    // `--entailment`/`--canonical` transform lane below consults it), so a bare
    // `--rules FILE` would otherwise be accepted by clap and silently do nothing —
    // the same no-op shape this pipeline refuses everywhere else.
    if options.rules.is_some() && options.entailment.is_none() {
        return Err(CliError::Usage(
            "--rules names the rule document an entailment regime runs under; it has no \
             effect without --entailment"
                .to_owned(),
        ));
    }

    // The transform lane: either `--entailment` or `--canonical` needs a concrete
    // owned dataset, so reconstruct one and apply the transforms in order.
    if options.canonical || options.entailment.is_some() {
        return run_with_transforms(
            source_format,
            options,
            input,
            output,
            ledger_target,
            report_target,
        );
    }

    // Pack → pack: a verified byte passthrough (no decode/re-encode churn). The pack
    // is acquired through the one immutable-input seam — a DISK pack is borrowed from
    // a memory-safe mapping (no `Vec<u8>` copy of the contents where the platform can
    // guarantee immutability), stdin is owned — and verified once before its bytes
    // are written straight through.
    let target_format = format::resolve(options.to, output)?;
    format::refuse_base_with_pack(target_format, options.base, "a pack --to target")?;
    sink::validate_jsonld_options(target_format, options.jsonld_options)?;
    if source_format.is_pack() && target_format.is_pack() {
        let owner = source::verified_pack_input(input)?;
        sink::write_out(output, owner.as_bytes())?;
        return ledger::surface(ledger_target, &LossLedger::new());
    }

    let src_codec = source_format.loss_codec_name();
    let ledger = source::run_over_input(
        input,
        source_format,
        options.base,
        ConvertOp {
            out: output,
            target: target_format,
            base: options.base,
            src_codec,
            jsonld_options: options.jsonld_options,
        },
    )?;
    ledger::surface(ledger_target, &ledger)
}

/// The `--entailment` / `--canonical` lane: reconstruct an owned dataset, optionally
/// materialize its entailment closure, then either emit canonical N-Quads or
/// serialize to the `--to` target.
fn run_with_transforms(
    source_format: SourceFormat,
    options: &ConvertOptions<'_>,
    input: &str,
    output: &str,
    ledger_target: &LedgerTarget,
    report_target: &ReportTarget,
) -> Result<(), CliError> {
    let target_format = if options.canonical {
        None
    } else {
        let target = format::resolve(options.to, output)?;
        format::refuse_base_with_pack(target, options.base, "a pack --to target")?;
        sink::validate_jsonld_options(target, options.jsonld_options)?;
        Some(target)
    };
    let dataset = source::load_dataset(input, source_format, options.base)?;

    // Entail first: materialize the regime's closure, through the same
    // `EntailmentPlan` `reason` resolves — so `--rules` means the same thing here.
    let dataset: Arc<RdfDataset> = match options.entailment {
        Some(regime) => {
            let plan = reason::EntailmentPlan::resolve(regime, options.rules)?;
            // The closure is what gets serialized; the report is what `--report` carries,
            // so a converted document can be traced back to the run that derived it.
            report::materialize_reported(&dataset, plan.materialization(), report_target)?
        }
        None => dataset,
    };

    // Then canonicalize: RDFC-1.0 canonical N-Quads always override `--to`.
    if options.canonical {
        let nquads = canonical_flat_nquads(&dataset).map_err(CliError::Runtime)?;
        sink::write_out(output, nquads.as_bytes())?;
        // The RDFC-1.0 canonical N-Quads document flattens the RDF 1.2 statement
        // overlay into plain triples; it is a lossless re-rendering, so no ledger.
        return ledger::surface(ledger_target, &LossLedger::new());
    }

    // No `--canonical`: serialize the (possibly entailed) closure to `--to`.
    let target_format = target_format.expect("non-canonical branch resolved a target format");
    let src_codec = source_format.loss_codec_name();
    let ledger = sink::write_rdf(
        &*dataset,
        output,
        target_format,
        options.base,
        src_codec,
        options.jsonld_options,
    )?;
    ledger::surface(ledger_target, &ledger)
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `convert` subcommand: `Source(s) → [entail] → [canonicalize] → Sink`.
//!
//! Resolve the source and target formats, open the source as a view, and write it
//! through the [`sink`]. A pack→pack conversion is a verified byte
//! passthrough (re-encoding a pack would be pointless churn); every other
//! combination flows the source view into [`sink::write_rdf`], which serializes to
//! the target syntax or rebuilds the pack container. The resulting loss ledger is
//! surfaced under `--loss-ledger`.
//!
//! ## Three input admissions, and the one lane each takes
//!
//! * **One native/pack source** takes the ZERO-COPY lane unchanged: the source is opened
//!   as its concrete view and serialized straight through (a pack is never materialized),
//!   and pack→pack is still a verified byte passthrough. Nothing about the following two
//!   admissions may perturb it, which is why they are separate branches rather than a
//!   generalization of it — `RdfDataset::union` re-scopes blank nodes even for a single
//!   input, so routing one source through the merge lane would relabel blank nodes in
//!   every ordinary conversion.
//! * **A GTS source** is folded by `purrdf_rdf::import_gts_events`, the authoritative
//!   importer that preserves per-segment blank-node scope, and the ENVELOPE it could not
//!   carry into an RDF dataset (segment ledger, sidecars, metadata, blob references,
//!   suppressions, opaque nodes, signatures) is reported as loss-ledger entries with the
//!   counts and segment head ids the importer actually returned. See
//!   [`source`](crate::source) for why the `bnode-scope-flatten` contract entry is NOT
//!   attached on this path.
//! * **Two or more sources** — the positional `IN` plus each repeatable `--input`, in
//!   command-line order — are merged with `RdfDataset::union`: order-independent,
//!   duplicate-collapsing at freeze, and standardize-apart per source. See
//!   [`ingest`](crate::ingest) for the full contract and for the zero-copy trade a pack
//!   in a multi-source list pays.
//!
//! Every lane threads the same `--base`, `--transport`, diagnostics and ledger; a flag
//! that cannot apply to the lane the operator selected is refused by name rather than
//! accepted and ignored.
//!
//! ## Transforms: `--entailment` and `--canonical`
//!
//! Two optional transforms compose in a fixed order (entail first, then
//! canonicalize). The entailment lane enters the reasoner over the source's zero-copy
//! view — a pack is NOT rebuilt into an owned dataset to materialize; the closure a
//! run produces is then serialized. Without `--entailment`, canonicalization and
//! serialization operate on an owned `Arc<RdfDataset>` reconstructed from the source
//! (a text source is parsed, a pack source read via [`source::load_dataset`]):
//!
//! * `--entailment REGIME` materializes the regime's closure over the view (through
//!   the same `EntailmentPlan` `reason` resolves, so `--rules` means the same thing
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
use crate::ingest::{self, Sources};
use crate::ledger;
use crate::reason;
use crate::report;
use crate::sink;
use crate::source::{self, TransportPolicy, ViewOp};

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
    /// The `--from` input-format override. Applies to EVERY source in the list.
    pub(crate) from: Option<CliRdfFormat>,
    /// The `--to` output-format override.
    pub(crate) to: Option<CliRdfFormat>,
    /// The repeatable `--input` sources, appended after the positional `IN`.
    pub(crate) inputs: &'a [String],
    /// `--transport`: how a gzip/zstd wrapper around each source is handled.
    pub(crate) transport: TransportPolicy,
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
    let sources = Sources::new(
        input,
        options.inputs,
        options.from,
        options.base,
        options.transport,
    );
    // Resolve EVERY source's format up front, so a list whose third entry is unreadable
    // fails before the first two are read. `--base` is decided later, once the target is
    // resolved too: whether it can be spent depends on both legs.
    let formats = sources.resolve_formats()?;

    refuse_inapplicable_combinations(options, report_target)?;

    // The transform lane: either `--entailment` or `--canonical` needs a concrete
    // owned dataset, so reconstruct one and apply the transforms in order.
    if options.canonical || options.entailment.is_some() {
        return run_with_transforms(
            &sources,
            &formats,
            options,
            output,
            ledger_target,
            report_target,
        );
    }

    let target_format = format::resolve_target(options.to, output, "the --to target")?;
    refuse_unconsumable_base(
        &sources,
        &formats,
        target_format,
        "the --to target",
        options,
    )?;
    sink::validate_jsonld_options(target_format, options.jsonld_options)?;

    // One native/pack source: the untouched zero-copy lane. A GTS source is excluded
    // because its envelope loss has to be surfaced, and a multi-source list because a
    // union takes concrete datasets.
    if let (Some(only), [source_format]) = (sources.single(), formats.as_slice())
        && !source_format.is_gts()
    {
        return convert_one_view(
            only,
            *source_format,
            options,
            output,
            target_format,
            ledger_target,
        );
    }

    // Otherwise: materialize (folding GTS through the authoritative importer, merging a
    // list through the deterministic union) and serialize the result.
    let (dataset, mut merged) = ingest::ingest(&sources, &formats)?;
    let write = sink::write_rdf(
        &*dataset,
        output,
        target_format,
        options.base,
        source_codec(&formats),
        options.jsonld_options,
    )?;
    for entry in write.entries() {
        merged.record(entry.clone());
    }
    ledger::surface(ledger_target, &merged)
}

/// The single-source native/pack lane, byte-for-byte what it has always been.
fn convert_one_view(
    input: &str,
    source_format: SourceFormat,
    options: &ConvertOptions<'_>,
    output: &str,
    target_format: SourceFormat,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    // Pack → pack: a verified byte passthrough (no decode/re-encode churn). The pack
    // is acquired through the one immutable-input seam — a DISK pack is borrowed from
    // a memory-safe mapping (no `Vec<u8>` copy of the contents where the platform can
    // guarantee immutability), stdin is owned — and verified once before its bytes
    // are written straight through.
    if source_format.is_pack() && target_format.is_pack() {
        let owner = source::verified_pack_input(input)?;
        sink::write_out(output, owner.as_bytes())?;
        return ledger::surface(ledger_target, &LossLedger::new());
    }

    let src_codec = source_format.loss_codec_name();
    let ledger = source::run_over_input_with_transport(
        input,
        source_format,
        options.base,
        options.transport,
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

/// The loss-ledger codec name for a source LIST.
///
/// One source contributes its own codec identity. A merged list contributes NONE: the
/// contract half of the ledger describes a `(from → to)` codec pair, and a union of
/// Turtle and a pack has no single `from` to name. Claiming one of them would attribute
/// the whole conversion's contract losses to a codec that carried only part of it.
fn source_codec(formats: &[SourceFormat]) -> Option<&'static str> {
    match formats {
        [only] => only.loss_codec_name(),
        _ => None,
    }
}

/// Refuse `--base` when NEITHER leg of this conversion can spend it.
///
/// The legs are every source's PARSE and the target's SERIALIZE, so `--base X --from turtle
/// --to ntriples` still works (the parse leg spends it) while `--from ntriples --to
/// ntriples` is refused by name instead of exiting 0 having ignored the flag. The predicate
/// is `format::refuse_unconsumable_base`'s, driven off the format registry's own columns.
fn refuse_unconsumable_base(
    sources: &Sources<'_>,
    formats: &[SourceFormat],
    target: SourceFormat,
    target_role: &str,
    options: &ConvertOptions<'_>,
) -> Result<(), CliError> {
    let roles: Vec<String> = sources
        .paths
        .iter()
        .map(|path| format!("the source `{path}`"))
        .collect();
    let mut legs: Vec<format::BaseUse<'_>> = formats
        .iter()
        .zip(&roles)
        .map(|(format, role)| format::BaseUse::parse(*format, role))
        .collect();
    legs.push(format::BaseUse::serialize(target, target_role));
    format::refuse_unconsumable_base(options.base, &legs)
}

/// Refuse the flag combinations `convert` cannot honour, each by name.
fn refuse_inapplicable_combinations(
    options: &ConvertOptions<'_>,
    report_target: &ReportTarget,
) -> Result<(), CliError> {
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
    Ok(())
}

/// The `--entailment` / `--canonical` lane: reconstruct an owned dataset, optionally
/// materialize its entailment closure, then either emit canonical N-Quads or
/// serialize to the `--to` target.
fn run_with_transforms(
    sources: &Sources<'_>,
    formats: &[SourceFormat],
    options: &ConvertOptions<'_>,
    output: &str,
    ledger_target: &LedgerTarget,
    report_target: &ReportTarget,
) -> Result<(), CliError> {
    let target_format = if options.canonical {
        // `--canonical` writes RDFC-1.0 canonical N-Quads unconditionally, so the egress
        // leg is N-Quads whatever else was named — and N-Quads can express no base.
        refuse_unconsumable_base(
            sources,
            formats,
            SourceFormat::Native(purrdf_rdf::NativeRdfFormat::NQuads),
            "the --canonical N-Quads output",
            options,
        )?;
        None
    } else {
        let target = format::resolve_target(options.to, output, "the --to target")?;
        refuse_unconsumable_base(sources, formats, target, "the --to target", options)?;
        sink::validate_jsonld_options(target, options.jsonld_options)?;
        Some(target)
    };
    // Entail first: materialize the regime's closure over the source, through the same
    // `EntailmentPlan` `reason` resolves — so `--rules` means the same thing here. A pack
    // source enters the reasoner as a zero-copy `PackView` (no `dataset_from_view`
    // rebuild) in the single-source case. Without `--entailment` the source list is
    // ingested as an owned dataset, which the canonicalization/serialization below need.
    let mut read_ledger = LossLedger::new();
    let dataset: Arc<RdfDataset> = match options.entailment {
        Some(regime) => {
            let plan = reason::EntailmentPlan::resolve(regime, options.rules)?;
            // The closure is what gets serialized; the report is what `--report` carries,
            // so a converted document can be traced back to the run that derived it.
            match (sources.single(), formats) {
                (Some(only), [source_format]) if !source_format.is_gts() => {
                    report::materialize_reported_over_input(
                        only,
                        *source_format,
                        options.base,
                        options.transport,
                        plan.materialization(),
                        report_target,
                    )?
                }
                _ => {
                    // A merged list (or a GTS source, whose envelope ledger must be
                    // carried) has no single view to reason over: materialize first,
                    // then reason over the merged dataset.
                    let (merged, ledger) = ingest::ingest(sources, formats)?;
                    read_ledger = ledger;
                    report::materialize_reported(&*merged, plan.materialization(), report_target)?
                }
            }
        }
        None => {
            let (merged, ledger) = ingest::ingest(sources, formats)?;
            read_ledger = ledger;
            merged
        }
    };

    // Then canonicalize: RDFC-1.0 canonical N-Quads always override `--to`.
    if options.canonical {
        let nquads = canonical_flat_nquads(&dataset).map_err(CliError::Runtime)?;
        sink::write_out(output, nquads.as_bytes())?;
        // The RDFC-1.0 canonical N-Quads document flattens the RDF 1.2 statement
        // overlay into plain triples; it is a lossless re-rendering, so the only
        // entries are whatever reading the sources already dropped.
        return ledger::surface(ledger_target, &read_ledger);
    }

    // No `--canonical`: serialize the (possibly entailed) closure to `--to`.
    let target_format = target_format.expect("non-canonical branch resolved a target format");
    let write = sink::write_rdf(
        &*dataset,
        output,
        target_format,
        options.base,
        source_codec(formats),
        options.jsonld_options,
    )?;
    for entry in write.entries() {
        read_ledger.record(entry.clone());
    }
    ledger::surface(ledger_target, &read_ledger)
}

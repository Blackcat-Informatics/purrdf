// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Admitting an ORDERED LIST of input sources as one dataset.
//!
//! `convert IN OUT` reads one document. [`ingest`] is the lane that reads several — the
//! positional `IN` followed by every `--input` in command-line order — and merges them
//! into a single frozen dataset through `RdfDataset::union`.
//!
//! # The command-line contract, and why it is shaped this way
//!
//! `IN` and `OUT` are positional, and clap fills positionals in order. A variadic
//! `IN... OUT` is therefore not expressible: with `convert a.ttl b.ttl`, nothing in the
//! grammar says whether `b.ttl` is a second input or the output. Rather than change what
//! `convert a.ttl out.nq` has always meant, the extra sources arrive through a REPEATABLE
//! `--input PATH` flag and the effective source list is:
//!
//! > `IN`, then each `--input` value, in the order they were written.
//!
//! `IN` keeps its `-` (stdin) default, so `purrdf convert --input b.ttl out.nq` reads
//! stdin AND `b.ttl` — that is not a surprise smuggled in by the new flag, it is the
//! documented meaning `IN` has carried on every subcommand since the CLI existed, and it
//! fails immediately with the usual "stdin requires --from" usage error rather than
//! blocking on a terminal.
//!
//! `--from`, when given, applies to EVERY source in the list: it is one flag and a
//! per-source override would need a per-source flag. When it is absent each source is
//! classified from its own extension, so a mixed `a.ttl` + `b.nq` + `c.gts` list needs no
//! flag at all. `--base` and `--transport` apply list-wide for the same reason, and
//! `--base` is refused against any container source exactly as it is for one source.
//!
//! # What the union guarantees, and the one thing it costs
//!
//! `RdfDataset::union` re-interns every input BY VALUE and re-freezes, so the merged
//! result is canonical: identical regardless of the order the inputs were supplied, with
//! duplicate ground quads collapsing to one at freeze. Each input is merged under its own
//! fresh `BlankScope` (standardize-apart, C0.2), so `_:b0` in the first source and `_:b0`
//! in the second stay DISTINCT nodes; blank nodes therefore never deduplicate across
//! sources, and a quad that mentions one never collapses with its counterpart.
//!
//! The cost is that a pack in a multi-source list must be MATERIALIZED through
//! `dataset_from_view` rather than read as a zero-copy `PackView`: a union takes concrete
//! datasets, and there is no view type that is the union of two others. That is a real,
//! deliberate trade — stated here rather than hidden — and it is why a SINGLE-source
//! convert never enters this lane at all.
//!
//! # Why one source never enters this lane
//!
//! `union` re-scopes blank nodes even for a single input: its result is graph-ISOMORPHIC
//! to the input, not label-identical (see `RdfDataset::owned_snapshot`). Routing the
//! one-source case through here would silently relabel blank nodes in every ordinary
//! conversion and defeat the pack→pack byte passthrough. [`ingest`] therefore returns the
//! single dataset UNCHANGED when the list holds one entry, and calls `union` only from
//! two upward.

use std::sync::Arc;

use purrdf_core::{LossLedger, RdfDataset};
use purrdf_rdf::SourceFormat;

use crate::cli::CliRdfFormat;
use crate::error::CliError;
use crate::format;
use crate::source::{self, TransportPolicy};

/// The resolved input side of a command: the ordered source list plus the flags that
/// apply to every entry in it.
pub(crate) struct Sources<'a> {
    /// The ordered source paths — the positional `IN` first, then each `--input`.
    /// Never empty.
    pub(crate) paths: Vec<&'a str>,
    /// `--from`: applies to EVERY source when given; otherwise each source is
    /// classified from its own extension.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`: the parse base every source resolves relative IRIs against.
    pub(crate) base: Option<&'a str>,
    /// `--transport`: how a gzip/zstd wrapper around each source is handled.
    pub(crate) transport: TransportPolicy,
}

impl<'a> Sources<'a> {
    /// The ordered source list for a positional `IN` plus the repeatable `--input`
    /// values, in command-line order.
    pub(crate) fn new(
        input: &'a str,
        extra: &'a [String],
        from: Option<CliRdfFormat>,
        base: Option<&'a str>,
        transport: TransportPolicy,
    ) -> Self {
        let mut paths = Vec::with_capacity(1 + extra.len());
        paths.push(input);
        paths.extend(extra.iter().map(String::as_str));
        Self {
            paths,
            from,
            base,
            transport,
        }
    }

    /// The one source, when exactly one was named.
    pub(crate) fn single(&self) -> Option<&'a str> {
        match self.paths.as_slice() {
            [only] => Some(only),
            _ => None,
        }
    }

    /// Resolve every source's format, refusing an explicit `--transport` against a pack and
    /// refusing a second `-` (stdin can be consumed once).
    ///
    /// `--base` is NOT decided here. Whether it can be spent depends on the SERIALIZE leg
    /// as well as these parse legs — `--base X --from turtle --to ntriples` is honoured on
    /// the way in, and `--from ntriples --to ntriples` can spend it on neither side — so the
    /// one refusal lives where both halves are known, in [`crate::convert`].
    pub(crate) fn resolve_formats(&self) -> Result<Vec<SourceFormat>, CliError> {
        let mut stdin_seen = false;
        let mut formats = Vec::with_capacity(self.paths.len());
        for path in &self.paths {
            if *path == "-" {
                if stdin_seen {
                    return Err(CliError::Usage(
                        "`-` names standard input, which can be read exactly once: a source \
                         list may not contain it twice"
                            .to_owned(),
                    ));
                }
                stdin_seen = true;
            }
            let format = format::resolve(self.from, path)?;
            // A pack is acquired as immutable bytes and verified in place; the transport
            // decoder is never reached for one, so a `--transport` naming an encoding for
            // a pack source would be accepted and never read.
            if format.is_pack() && self.transport != TransportPolicy::Detect {
                return Err(CliError::Usage(format!(
                    "--transport has no effect on the pack source `{path}`: a pack is acquired \
                     as immutable bytes and verified in place, so it is never handed to the \
                     transport decoder. A transport-wrapped pack is refused outright rather \
                     than decoded"
                )));
            }
            formats.push(format);
        }
        Ok(formats)
    }
}

/// Read every source in order and merge them into one frozen dataset.
///
/// `formats` is the caller's already-resolved [`Sources::resolve_formats`] result, one
/// entry per source in the same order: resolution happens ONCE, before anything is read,
/// so a list whose last entry cannot be classified fails before the first is opened.
///
/// Returns the dataset together with the accumulated read-side loss ledger (the GTS
/// envelope entries every GTS source contributes; empty otherwise). A single-source list
/// is returned verbatim — `union` is called only from two sources upward, because it
/// re-scopes blank nodes even for one input.
pub(crate) fn ingest(
    sources: &Sources<'_>,
    formats: &[SourceFormat],
) -> Result<(Arc<RdfDataset>, LossLedger), CliError> {
    debug_assert_eq!(sources.paths.len(), formats.len());

    let mut datasets: Vec<Arc<RdfDataset>> = Vec::with_capacity(sources.paths.len());
    let mut ledger = LossLedger::new();
    for (path, format) in sources.paths.iter().zip(formats.iter().copied()) {
        let (dataset, source_ledger) =
            source::load_dataset_reporting(path, format, sources.base, sources.transport)?;
        for entry in source_ledger.entries() {
            ledger.record(entry.clone());
        }
        datasets.push(dataset);
    }

    let [only] = datasets.as_slice() else {
        let borrowed: Vec<&RdfDataset> = datasets.iter().map(Arc::as_ref).collect();
        return Ok((Arc::new(RdfDataset::union(&borrowed)), ledger));
    };
    Ok((Arc::clone(only), ledger))
}

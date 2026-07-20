// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `project` and `lift` graph, tabular, and research-object carrier pipelines.

use purrdf_core::{DatasetView, LossLedger};
use purrdf_rdf::JsonLdSerializeOptions;
use purrdf_rdf::{
    ProjectionArchive, ProjectionConfig, RoCrateAssets, lift_archive, project_archive,
    project_archive_with_assets,
};

use crate::cli::{
    CliLiftProfile, CliNativeRdfFormat, CliProjectionProfile, CliRdfFormat, LedgerTarget,
};
use crate::error::CliError;
use crate::format::{self, CliFormat};
use crate::ledger;
use crate::sink;
use crate::source::{self, ViewOp};

struct ProjectOp<'a> {
    profile: CliProjectionProfile,
    config: &'a ProjectionConfig,
    assets: Option<&'a RoCrateAssets>,
}

impl ViewOp for ProjectOp<'_> {
    type Output = ProjectionArchive;

    fn run<D: DatasetView + Sync>(self, view: &D) -> Result<Self::Output, CliError> {
        Ok(if let Some(assets) = self.assets {
            project_archive_with_assets(view, self.profile.to_profile(), self.config, assets)?
        } else {
            project_archive(view, self.profile.to_profile(), self.config)?
        })
    }
}

/// Run `purrdf project` over a native RDF/pack source.
#[allow(
    clippy::too_many_arguments,
    reason = "the CLI dispatcher passes the explicit command fields without hidden defaults"
)]
pub(crate) fn run_project(
    profile: CliProjectionProfile,
    config_path: &str,
    assets_path: Option<&str>,
    from: Option<CliRdfFormat>,
    base: Option<&str>,
    input: &str,
    output: &str,
    jsonld_options: Option<&JsonLdSerializeOptions>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    if jsonld_options.is_some() {
        return Err(CliError::Usage(
            "--jsonld-options cannot be used with projection carrier output".to_owned(),
        ));
    }
    let config = read_config(config_path, input)?;
    if assets_path == Some("-") && (config_path == "-" || input == "-") {
        return Err(CliError::Usage(
            "projection assets cannot share stdin with configuration or RDF input".to_owned(),
        ));
    }
    let assets = assets_path
        .map(source::read_bytes)
        .transpose()?
        .map(|archive| RoCrateAssets::from_ustar(&archive, config.limits()))
        .transpose()?;
    let source_format = format::resolve(from, input)?;
    let outcome = source::run_over_input(
        input,
        source_format,
        base,
        ProjectOp {
            profile,
            config: &config,
            assets: assets.as_ref(),
        },
    )?;
    sink::write_out(output, &outcome.archive)?;
    ledger::surface(ledger_target, &outcome.loss_ledger)
}

/// Run `purrdf lift` over a canonical projection USTAR archive.
#[allow(
    clippy::too_many_arguments,
    reason = "the CLI dispatcher passes the explicit command fields without hidden defaults"
)]
pub(crate) fn run_lift(
    profile: CliLiftProfile,
    config_path: &str,
    to: CliNativeRdfFormat,
    base: Option<&str>,
    input: &str,
    output: &str,
    jsonld_options: Option<&JsonLdSerializeOptions>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    sink::validate_jsonld_options(CliFormat::Rdf(to.to_native()), jsonld_options)?;
    let config = read_config(config_path, input)?;
    let archive = source::read_bytes(input)?;
    let mut outcome = lift_archive(&archive, profile.to_profile(), &config)?;
    let serialization_ledger = sink::write_rdf(
        &*outcome.dataset,
        output,
        CliFormat::Rdf(to.to_native()),
        base,
        None,
        jsonld_options,
    )?;
    merge_ledger(&mut outcome.loss_ledger, &serialization_ledger);
    ledger::surface(ledger_target, &outcome.loss_ledger)
}

fn read_config(path: &str, input: &str) -> Result<ProjectionConfig, CliError> {
    if path == "-" && input == "-" {
        return Err(CliError::Usage(
            "projection configuration and data/archive input cannot both read from stdin"
                .to_owned(),
        ));
    }
    Ok(ProjectionConfig::from_json(&source::read_bytes(path)?)?)
}

fn merge_ledger(target: &mut LossLedger, source: &LossLedger) {
    for entry in source.entries() {
        target.record(entry.clone());
    }
}

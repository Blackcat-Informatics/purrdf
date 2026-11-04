// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Restore the existing snapshot law and retain one native plan per operation.

use super::{Command, Failure, Format, Lines, Refusals, digest, io, word_id, write_digest};
use clap::{Args, ValueEnum};
use purrdf_core::{
    content_alias::{self, AliasScope, MappingRevision, Presentation, ScopeId},
    content_prefix::{BuildError, Length, PrefixPlan, ScopedReference},
};
use purrdf_hash::{hex::Digest32, word_id::Reference};
use purrdf_lex::allocation::{Memory, OwnedAdmission, Resident};
use std::io::{Read as _, Write as _};

#[derive(Debug, Args)]
pub(crate) struct Snapshot {
    /// Exact original `PURRAL01` export; this must be a file.
    #[arg(long)]
    pub(super) manifest: String,
    /// Caller-owned scope, as canonical full hex.
    #[arg(long)]
    scope: String,
    /// Revision retained when the references were presented, as canonical hex.
    #[arg(long)]
    revision: String,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum LengthChoice {
    Uniform,
    PerEntry,
}
impl From<LengthChoice> for Length {
    fn from(value: LengthChoice) -> Self {
        match value {
            LengthChoice::Uniform => Self::Uniform,
            LengthChoice::PerEntry => Self::PerEntry,
        }
    }
}

pub(super) struct State {
    plan: PrefixPlan<Resident>,
    aliases: Option<AliasScope<Resident>>,
    reference: Option<Reference>,
    length: Length,
}

fn plan(aliases: &mut AliasScope<Resident>) -> Result<PrefixPlan<Resident>, Failure> {
    PrefixPlan::from_aliases(aliases, Resident).map_err(|error| match error {
        BuildError::Source { ordinal, cause } => Failure::Record {
            record: ordinal,
            offset: 0,
            cause: Box::new(cause.into()),
        },
        BuildError::Plan(cause) => cause.into(),
    })
}

fn committed(commit: content_alias::Commit) -> Result<(), Failure> {
    if let Some(cause) = commit.cleanup_refusal {
        return Err(cause.into());
    }
    Ok(())
}

pub(super) fn prepare(
    command: &Command,
    reader: &mut dyn io::BufRead,
) -> Result<(Option<Digest32>, State), Failure> {
    if let Command::Plan { scope, .. } = command {
        let scope = ScopeId::new(digest(scope.as_bytes(), Format::Hex)?);
        let mut owner = OwnedAdmission::new(Resident);
        let mut values = Vec::new();
        let mut memory = Memory::new(&mut owner);
        let mut input = Vec::new();
        input.try_reserve_exact(word_id::MAX_ENCODED_LEN + 1)?;
        let (mut ordinal, mut offset) = (0, 0);
        loop {
            let start = offset;
            let value = match super::read_record(reader, &mut input, &mut ordinal, &mut offset) {
                Ok(false) => break,
                Ok(true) => digest(&input, Format::Hex),
                Err(cause) => Err(cause),
            }
            .map_err(|cause| Failure::Record {
                record: ordinal,
                offset: start,
                cause: Box::new(cause),
            })?;
            memory.push(&mut values, value)?;
        }
        let mut aliases = AliasScope::new(scope, Resident)?;
        // One atomic assignment operation over the once-read admitted source.
        committed(aliases.extend(scope, &values)?)?;
        let plan = plan(&mut aliases)?;
        return Ok((
            None,
            State {
                plan,
                aliases: Some(aliases),
                reference: None,
                length: Length::Uniform,
            },
        ));
    }
    let snapshot = command
        .snapshot()
        .ok_or(Failure::Invariant("missing snapshot arguments"))?;
    let expected = Presentation {
        scope: ScopeId::new(digest(snapshot.scope.as_bytes(), Format::Hex)?),
        revision: MappingRevision::new(digest(snapshot.revision.as_bytes(), Format::Hex)?),
    };
    let mut owner = OwnedAdmission::new(Resident);
    let mut bytes = Vec::new();
    let mut memory = Memory::new(&mut owner);
    let mut file = std::fs::File::open(&snapshot.manifest)?;
    let mut chunk = [0; 8192];
    loop {
        let count = file.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        memory.extend(&mut bytes, chunk[..count].iter().copied())?;
    }
    let mut aliases = AliasScope::new(expected.scope, Resident)?;
    committed(aliases.restore(expected.scope, &bytes)?)?;
    let actual = aliases.presentation(expected.scope)?;
    if actual.revision != expected.revision {
        return Err(content_alias::Error::Revision {
            expected: expected.revision,
            actual: actual.revision,
        }
        .into());
    }
    let mut state = State {
        plan: plan(&mut aliases)?,
        aliases: None,
        reference: None,
        length: match command {
            Command::Shorten { length, .. } | Command::BatchScoped { length, .. } => {
                (*length).into()
            }
            _ => Length::Uniform,
        },
    };
    let single = if matches!(command, Command::BatchScoped { .. }) {
        None
    } else {
        let format = match command {
            Command::Shorten { from, .. } => *from,
            _ => Format::Words,
        };
        let input = super::read_single(reader, format)?;
        let value = state.read(&input, format)?;
        Some(value)
    };
    Ok((single, state))
}

impl State {
    pub(super) fn read(&mut self, input: &[u8], format: Format) -> Result<Digest32, Failure> {
        let reference = match format {
            Format::Words => word_id::decode(input)?,
            _ => Reference::Full(digest(input, format)?),
        };
        self.reference = Some(reference);
        Ok(self.plan.resolve(ScopedReference {
            presentation: self.plan.presentation(),
            reference,
        })?)
    }
    pub(super) fn write(
        &mut self,
        output: &mut dyn io::Write,
        digest: Digest32,
        format: Lines,
        buffer: &mut [u8; word_id::MAX_ENCODED_LEN],
    ) -> Result<(), Failure> {
        match format {
            Lines::HexLines => write_digest(output, digest, Format::Hex, buffer),
            Lines::WordLines => {
                let reference = self
                    .plan
                    .shorten(self.plan.presentation(), digest, self.length)?
                    .reference;
                super::write_reference(output, reference, buffer)?;
                Ok(())
            }
        }
    }
    pub(super) fn convert(
        &mut self,
        command: &Command,
        reader: &mut dyn io::BufRead,
        single: Option<Digest32>,
        output: &mut dyn io::Write,
        buffer: &mut [u8; word_id::MAX_ENCODED_LEN],
        refusals: &mut Refusals,
    ) -> Result<(), Failure> {
        if let Command::BatchScoped { from, to, .. } = command {
            return super::batch(reader, output, *from, *to, buffer, refusals, Some(self));
        }
        let binding = self.plan.presentation();
        match command {
            Command::Plan { .. } => {
                let aliases = self
                    .aliases
                    .as_mut()
                    .ok_or(Failure::Invariant("missing prepared snapshot"))?;
                let exported = aliases.export(binding.scope)?;
                output.write_all(&exported)?;
                exported.release()?;
                let summary = self.plan.summary();
                let checked = if summary.uniform_payload_words == 0 {
                    0
                } else {
                    summary.uniform_payload_words + 1
                };
                let mut stderr = io::stderr().lock();
                writeln!(
                    stderr,
                    "scope {}\nrevision {}\nidentities {}\nuniform_payload_words {}\nuniform_checked_words {checked}",
                    binding.scope.digest(),
                    binding.revision.digest(),
                    summary.identities,
                    summary.uniform_payload_words
                )?;
                for (index, count) in summary.per_entry_lengths.iter().enumerate() {
                    writeln!(
                        stderr,
                        "per_entry_payload_words {} count {count}",
                        index + 1
                    )?;
                }
            }
            Command::Shorten { .. } => self.write(
                output,
                single.ok_or(Failure::Invariant("missing validated digest"))?,
                Lines::WordLines,
                buffer,
            )?,
            Command::Expand { to, .. } => write_digest(
                output,
                single.ok_or(Failure::Invariant("missing validated digest"))?,
                (*to).into(),
                buffer,
            )?,
            Command::InspectScoped { .. } => {
                let digest = single.ok_or(Failure::Invariant("missing resolved digest"))?;
                let reference = self
                    .reference
                    .ok_or(Failure::Invariant("missing validated selector"))?;
                writeln!(
                    output,
                    "scope {}\nrevision {}\ndigest {digest}",
                    binding.scope.digest(),
                    binding.revision.digest(),
                )?;
                super::inspect_reference(output, reference, buffer)?;
            }
            _ => return Err(Failure::Invariant("unexpected scoped operation")),
        }
        Ok(())
    }
}

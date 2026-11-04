// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact checked word prefixes bound to an immutable content snapshot.
//!
//! A caller supplies the real [`Presentation`] of its scoped assignment history
//! and that revision's complete content membership. This module does not derive
//! a new scope, revision or content identity. Equal digests are deduplicated;
//! their separate source-row identities and integer aliases remain unchanged.
//! Growth constructs a new plan with the new presentation. An old plan and its
//! references retain their original binding and never silently change meaning.
//!
//! Construction sorts native digest bytes and compares neighbours, without
//! formatting words. Queries use binary search followed by at most two prefix
//! matches. Every source read, comparison and query retains the original caller
//! admission; the index's fallible allocation dies before that account.

use core::{convert::Infallible, fmt};
use purrdf_hash::{
    hex::Digest32,
    word_id::{self, Reference},
};
use purrdf_lex::{
    allocation::{Admission, Memory, OwnedAdmission, StorageError},
    walk::{try_dedup_by, try_equal_range_by, try_sort_unstable_by},
};

use crate::content_alias::{self, Presentation};

/// The exact trusted snapshot envelope retained alongside displayed words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedReference {
    /// Original scope and assignment revision supplied by the dataset owner.
    pub presentation: Presentation,
    /// A validated full identity or checked lexical prefix.
    pub reference: Reference,
}

/// Explicit choice between one dataset-wide length and each entry's minimum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Length {
    /// Use the exact uniform payload minimum for this snapshot.
    Uniform,
    /// Use the shortest uniquely resolving payload for this entry.
    PerEntry,
}

/// Complete exact prefix statistics, excluding duplicate content identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Number of distinct complete digests.
    pub identities: usize,
    /// Zero for an empty index, otherwise the exact minimum in 1 through 22.
    pub uniform_payload_words: usize,
    /// Count of entries needing each payload length; slot zero means one word.
    pub per_entry_lengths: [usize; word_id::FULL_PAYLOAD_WORDS],
}

/// A snapshot, lookup, input-order or original admission refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The supplied scope or revision differs from this frozen owner.
    Binding(content_alias::Error),
    /// No content identity in this snapshot matches the submitted reference.
    Unknown {
        /// Exact checked selector, never a guessed candidate.
        reference: Reference,
    },
    /// The prefix matches at least these two distinct identities.
    Ambiguous {
        /// Exact checked selector.
        reference: Reference,
        /// First matching identity in digest order.
        first: Digest32,
        /// Second matching identity; no further candidates need be read.
        second: Digest32,
    },
    /// A claimed sorted source decreases at this one-based source ordinal.
    Order {
        /// Original input position, including duplicate entries.
        ordinal: usize,
        /// Immediately preceding source value.
        previous: Digest32,
        /// Current source value.
        actual: Digest32,
    },
    /// Original lexical word-codec refusal.
    Codec(word_id::Error),
    /// Original work, storage, layout or physical allocation refusal.
    Storage(StorageError),
}
purrdf_lex::variant_from!(Error { Binding(content_alias::Error), Codec(word_id::Error), Storage(StorageError) });
purrdf_hash::error_sources!(Error, Binding, Codec, Storage);
impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        Binding(cause) => "content-prefix-binding" { "cause" => Cause(cause) },
        Unknown { reference } => "content-prefix-unknown" { "reference" => Record(reference) },
        Ambiguous { reference, first, second } => "content-prefix-ambiguous" { "reference" => Record(reference), "first" => Record(first), "second" => Record(second) },
        Order { ordinal, previous, actual } => "content-prefix-source-order" { "ordinal" => Unsigned(*ordinal as u128), "previous" => Record(previous), "actual" => Record(actual) },
        Codec(cause) => "content-prefix-codec" { "cause" => Cause(cause) },
        Storage(cause) => "content-prefix-storage" { "cause" => Cause(cause) },
    }
}
impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binding(cause) => cause.fmt(out),
            Self::Unknown { .. } => {
                out.write_str("word reference is unknown in the bound snapshot")
            }
            Self::Ambiguous { first, second, .. } => {
                write!(out, "word prefix is ambiguous between {first} and {second}")
            }
            Self::Order { ordinal, .. } => write!(
                out,
                "digest source decreases at record {ordinal}; sorted input is required"
            ),
            Self::Codec(cause) => cause.fmt(out),
            Self::Storage(cause) => cause.fmt(out),
        }
    }
}

/// A failed construction preserves the source's original typed cause.
#[derive(Debug)]
pub enum BuildError<E> {
    /// A source read failed before this one-based input position was admitted.
    Source {
        /// Original input position, including duplicate entries.
        ordinal: usize,
        /// Unmodified source failure.
        cause: E,
    },
    /// Native plan construction or admission refusal.
    Plan(Error),
}
purrdf_lex::variant_from!(impl<E> BuildError<E> { Plan(Error) });
purrdf_lex::variant_from!(impl<E> BuildError<E> { Plan(StorageError) via Error::Storage });
impl<E: fmt::Display> fmt::Display for BuildError<E> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source { ordinal, cause } => {
                write!(out, "digest source record {ordinal}: {cause}")
            }
            Self::Plan(cause) => cause.fmt(out),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for BuildError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Source { cause, .. } => cause,
            Self::Plan(cause) => cause,
        })
    }
}
impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError for BuildError<E> {
    purrdf_hash::error_visit! {
        Source { ordinal, cause } => "content-prefix-source" { "ordinal" => Unsigned(*ordinal as u128), "cause" => Cause(cause) },
        Plan(cause) => "content-prefix-build" { "cause" => Cause(cause) },
    }
}

#[derive(Clone, Copy)]
struct Entry {
    digest: Digest32,
    words: usize,
}

/// An immutable, admitted digest index with exact uniform and per-entry plans.
/// Query methods borrow mutably only to poll the original caller's admission.
pub struct PrefixPlan<S: Admission> {
    entries: Vec<Entry>,
    summary: Summary,
    presentation: Presentation,
    admission: OwnedAdmission<S>,
}
purrdf_hash::debug_non_exhaustive!([S: Admission] PrefixPlan<S> {});
impl<S: Admission> PrefixPlan<S> {
    /// Capture the exact current membership and presentation of an actual
    /// alias owner. Each original ordinal read retains that owner's admission;
    /// the new index retains its own supplied account. Later alias growth cannot
    /// retarget the returned immutable plan.
    /// # Errors
    /// Original alias read/admission failure or native plan allocation refusal.
    pub fn from_aliases<A: Admission>(
        owner: &mut content_alias::AliasScope<A>,
        admission: S,
    ) -> Result<Self, BuildError<content_alias::Error>> {
        let scope = owner.scope();
        let presentation = owner.presentation(scope).map_err(Error::from)?;
        let count = owner.identity_count();
        let source = (0..count).map(|index| {
            let ordinal = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(StorageError::SizeOverflow)?;
            owner.resolve(content_alias::ScopedAlias {
                scope,
                alias: content_alias::Alias::new(ordinal)?,
            })
        });
        Self::from_source(presentation, source, admission)
    }

    /// Build from the complete membership of the supplied real presentation.
    ///
    /// The caller must pair this source with its exact scoped assignment
    /// revision. Scope/revision values are trusted host metadata, never values
    /// inferred from a word string. Equal source digests are deduplicated.
    /// # Errors
    /// Preserves the indexed original source failure or native construction
    /// refusal. No partial owner escapes; source reads stop on admission refusal.
    pub fn from_source<E>(
        presentation: Presentation,
        source: impl IntoIterator<Item = Result<Digest32, E>>,
        admission: S,
    ) -> Result<Self, BuildError<E>> {
        Self::build(presentation, source, admission, false)
    }

    /// Reuse nondecreasing complete digest input, validating its actual order.
    /// The same trusted presentation/source pairing as [`Self::from_source`]
    /// is required. Equal consecutive digests are admitted and deduplicated.
    /// # Errors
    /// Original source/admission failure or the first decreasing input pair.
    pub fn from_sorted_source<E>(
        presentation: Presentation,
        source: impl IntoIterator<Item = Result<Digest32, E>>,
        admission: S,
    ) -> Result<Self, BuildError<E>> {
        Self::build(presentation, source, admission, true)
    }

    /// Build from an already available complete membership slice.
    /// The exact presentation/source pairing remains the caller's obligation.
    /// # Errors
    /// Returns native work/storage/allocation refusal without a partial owner.
    pub fn from_digests(
        presentation: Presentation,
        digests: &[Digest32],
        admission: S,
    ) -> Result<Self, Error> {
        match Self::from_source(
            presentation,
            digests.iter().copied().map(Ok::<_, Infallible>),
            admission,
        ) {
            Ok(plan) => Ok(plan),
            Err(BuildError::Plan(cause)) => Err(cause),
            Err(BuildError::Source { cause, .. }) => match cause {},
        }
    }

    fn build<E>(
        presentation: Presentation,
        source: impl IntoIterator<Item = Result<Digest32, E>>,
        admission: S,
        sorted: bool,
    ) -> Result<Self, BuildError<E>> {
        let mut admission = OwnedAdmission::new(admission);
        let (entries, summary) = Memory::new(&mut admission).try_scope(|memory| {
            let mut entries: Vec<Entry> = Vec::new();
            let mut source = source.into_iter();
            let mut ordinal = 0_usize;
            loop {
                memory.admission_mut().checkpoint(1)?;
                let Some(next) = source.next() else { break };
                ordinal = ordinal.checked_add(1).ok_or(StorageError::SizeOverflow)?;
                let digest = next.map_err(|cause| BuildError::Source { ordinal, cause })?;
                if sorted
                    && let Some(previous) = entries.last()
                    && previous.digest > digest
                {
                    return Err(Error::Order {
                        ordinal,
                        previous: previous.digest,
                        actual: digest,
                    }
                    .into());
                }
                memory.push(&mut entries, Entry { digest, words: 1 })?;
            }
            if !sorted {
                try_sort_unstable_by(&mut entries, |left, right| {
                    memory.admission_mut().checkpoint(1)?;
                    Ok::<_, StorageError>(left.digest.cmp(&right.digest))
                })?;
            }
            try_dedup_by(&mut entries, |left, right| {
                memory.admission_mut().checkpoint(1)?;
                Ok::<_, StorageError>(left.digest == right.digest)
            })?;
            for index in 1..entries.len() {
                memory.admission_mut().checkpoint(1)?;
                let left = entries[index - 1].digest;
                let right = entries[index].digest;
                let same = purrdf_deflate::common_prefix_len(left.as_bytes(), right.as_bytes());
                // Deduplication ensures a differing byte exists. Its native
                // leading-zero count completes the exact shared-bit prefix.
                let bits = same * 8
                    + (left.as_bytes()[same] ^ right.as_bytes()[same]).leading_zeros() as usize;
                let words = bits / 12 + 1;
                entries[index - 1].words = entries[index - 1].words.max(words);
                entries[index].words = entries[index].words.max(words);
            }
            let mut summary = Summary {
                identities: entries.len(),
                uniform_payload_words: 0,
                per_entry_lengths: [0; word_id::FULL_PAYLOAD_WORDS],
            };
            for entry in &entries {
                memory.admission_mut().checkpoint(1)?;
                summary.uniform_payload_words = summary.uniform_payload_words.max(entry.words);
                summary.per_entry_lengths[entry.words - 1] += 1;
            }
            Ok::<_, BuildError<E>>((entries, summary))
        })?;
        Ok(Self {
            entries,
            summary,
            presentation,
            admission,
        })
    }

    /// The frozen trusted envelope; this does not perform a source lookup.
    pub const fn presentation(&self) -> Presentation {
        self.presentation
    }

    /// Complete construction statistics; no source read or output allocation.
    pub const fn summary(&self) -> Summary {
        self.summary
    }

    fn binding(&mut self, actual: Presentation) -> Result<(), Error> {
        self.admission.checkpoint(1)?;
        let expected = self.presentation;
        if expected.scope != actual.scope {
            return Err(content_alias::Error::Scope {
                expected: expected.scope,
                actual: actual.scope,
            }
            .into());
        }
        if expected.revision != actual.revision {
            return Err(content_alias::Error::Revision {
                expected: expected.revision,
                actual: actual.revision,
            }
            .into());
        }
        Ok(())
    }

    fn lower_bound(&mut self, digest: Digest32) -> Result<usize, Error> {
        Ok(try_equal_range_by(&self.entries, |entry| {
            self.admission.checkpoint(1)?;
            Ok::<_, StorageError>(entry.digest.cmp(&digest))
        })?
        .start)
    }

    /// Shorten a member using this snapshot's exact selected length law.
    /// # Errors
    /// Wrong snapshot, unknown complete identity or original work refusal.
    pub fn shorten(
        &mut self,
        presentation: Presentation,
        digest: Digest32,
        length: Length,
    ) -> Result<ScopedReference, Error> {
        self.binding(presentation)?;
        let index = self.lower_bound(digest)?;
        self.admission.checkpoint(1)?;
        let entry = self
            .entries
            .get(index)
            .filter(|entry| entry.digest == digest)
            .ok_or(Error::Unknown {
                reference: Reference::Full(digest),
            })?;
        let words = match length {
            Length::Uniform => self.summary.uniform_payload_words,
            Length::PerEntry => entry.words,
        };
        Ok(ScopedReference {
            presentation,
            reference: Reference::from_digest(digest, words)?,
        })
    }

    /// Resolve only within the exact retained snapshot, stopping after the
    /// second matching candidate establishes ambiguity. Full forms also require
    /// membership when resolved through this scoped owner.
    /// # Errors
    /// Wrong scope/revision, unknown/ambiguous selector or original work refusal.
    pub fn resolve(&mut self, reference: ScopedReference) -> Result<Digest32, Error> {
        self.binding(reference.presentation)?;
        let lower = match reference.reference {
            Reference::Full(digest) => digest,
            Reference::Prefix(prefix) => prefix.lower_bound(),
        };
        let index = self.lower_bound(lower)?;
        let matches = |digest| match reference.reference {
            Reference::Full(full) => full == digest,
            Reference::Prefix(prefix) => prefix.matches(digest),
        };
        self.admission.checkpoint(1)?;
        let first = self
            .entries
            .get(index)
            .filter(|entry| matches(entry.digest))
            .ok_or(Error::Unknown {
                reference: reference.reference,
            })?
            .digest;
        if matches!(reference.reference, Reference::Prefix(_)) {
            self.admission.checkpoint(1)?;
            if let Some(second) = self
                .entries
                .get(index + 1)
                .filter(|entry| matches(entry.digest))
            {
                return Err(Error::Ambiguous {
                    reference: reference.reference,
                    first,
                    second: second.digest,
                });
            }
        }
        Ok(first)
    }

    /// Validate the displayed lexical check before consulting a dataset index.
    /// # Errors
    /// Original codec refusal, then the same scoped errors as [`Self::resolve`].
    pub fn resolve_text(
        &mut self,
        presentation: Presentation,
        text: &[u8],
    ) -> Result<Digest32, Error> {
        let reference = word_id::decode(text)?;
        self.resolve(ScopedReference {
            presentation,
            reference,
        })
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original admission ownership for a reserved-vocabulary refusal.

use purrdf_lex::allocation::{Admission, Memory, SharedOwned, StorageError};

use super::{CanonError, TermPosition};
use crate::{DatasetView, OwnedWorkspaceReservation};

#[derive(Debug, PartialEq, Eq)]
struct Fields {
    iri: String,
    position: TermPosition,
}

/// The input carries an IRI in the profile's [`super::RESERVED_NAMESPACE`]
/// outside one of the overlay's own lowered shapes. Accepting it could let
/// datasets with different content canonicalize to the same bytes, an identity
/// forgery. The exact shapes emitted by the overlay are folded back into their
/// statement-layer rows instead of refused; see the [canonicalization module](super)
/// for both parts of that rule.
///
/// The original IRI and position remain immutable under their original admission.
/// Cloning shares the same payload and grant without copying the IRI.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ReservedVocabulary {
    payload: SharedOwned<Fields>,
}

impl ReservedVocabulary {
    /// The original offending IRI, in full.
    #[must_use]
    pub fn iri(&self) -> &str {
        &self.payload.iri
    }

    /// The original statement position containing the IRI.
    #[must_use]
    pub fn position(&self) -> TermPosition {
        self.payload.position
    }
}

impl PartialEq for ReservedVocabulary {
    fn eq(&self, other: &Self) -> bool {
        *self.payload == *other.payload
    }
}
impl Eq for ReservedVocabulary {}

impl purrdf_hash::error::StructuredError for ReservedVocabulary {
    purrdf_hash::error_visit! {
        @record self { payload } => "canonical-reserved-vocabulary" {
            "iri" => Iri(self.iri()),
            "position" => Record(&self.payload.position),
        }
    }
}

impl std::fmt::Display for ReservedVocabulary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the dataset carries the reserved IRI <{}> in the {} position; \
             <{}…> is reserved by canonicalization profile \
             {} v{} for the RDF 1.2 overlay and \
             cannot appear in an input dataset",
            self.iri(),
            self.position(),
            super::RESERVED_NAMESPACE,
            super::CANON_PROFILE_ID,
            super::CANON_PROFILE_VERSION,
        )
    }
}
impl std::error::Error for ReservedVocabulary {}

struct SourceGrant<E> {
    reservation: OwnedWorkspaceReservation<E>,
    live: usize,
    failure: Option<E>,
}

impl<E> Admission for SourceGrant<E> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        let bytes = u64::try_from(bytes).map_err(|_| StorageError::SizeOverflow)?;
        if self.failure.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        self.reservation.resize(bytes).map_err(|cause| {
            self.failure = Some(cause);
            StorageError::AdmissionFailed
        })
    }
}

impl<E> SourceGrant<E> {
    fn memory<T>(
        &mut self,
        operation: impl FnOnce(&mut Memory<'_, Self>) -> Result<T, StorageError>,
    ) -> Result<T, CanonError<E>> {
        let mut memory = Memory::resume(self, self.live);
        let result = operation(&mut memory);
        let live = memory.admitted_bytes();
        self.live = live;
        result.map_err(|error| match (error, self.failure.take()) {
            (StorageError::AdmissionFailed, Some(cause)) => CanonError::OwnedAdmission(cause),
            (error, _) => CanonError::Storage(error),
        })
    }
}

/// The payload precedes its grant so every unsuccessful construction drops it first.
pub(super) struct RefusalBuilder<E> {
    fields: Fields,
    grant: SourceGrant<E>,
}

impl<E: Send + Sync + 'static> RefusalBuilder<E> {
    pub(super) fn new<D: DatasetView<ReadError = E>>(
        view: &D,
        iri: &str,
        position: TermPosition,
    ) -> Result<Self, CanonError<E>> {
        let reservation = view
            .reserve_owned_workspace(0)
            .ok_or(CanonError::OwnedWorkspaceUnavailable)?
            .map_err(CanonError::OwnedAdmission)?;
        let mut builder = Self {
            fields: Fields {
                iri: String::new(),
                position,
            },
            grant: SourceGrant {
                reservation,
                live: 0,
                failure: None,
            },
        };
        builder.fields.iri = builder.grant.memory(|memory| memory.string(iri))?;
        Ok(builder)
    }

    pub(super) fn consider(
        &mut self,
        iri: &str,
        position: TermPosition,
    ) -> Result<(), CanonError<E>> {
        if (position, iri) < (self.fields.position, self.fields.iri.as_str()) {
            let next = self.grant.memory(|memory| memory.string(iri))?;
            let previous = std::mem::replace(&mut self.fields.iri, next);
            self.fields.position = position;
            self.grant
                .memory(|memory| memory.release_string(previous))?;
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<ReservedVocabulary, CanonError<E>> {
        let bytes = SharedOwned::<Fields>::allocation_layout::<SourceGrant<E>>().size();
        self.grant.memory(|memory| memory.add_bytes(bytes))?;
        SharedOwned::try_from_admitted(self.fields, self.grant)
            .map(|payload| ReservedVocabulary { payload })
            .map_err(|_| CanonError::Storage(StorageError::AllocationFailed))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::{
        CanonHash, GraphMatch, NoopReservation, QuadIds, RdfDataset, RdfDatasetBuilder,
        RdfStoreCapabilities, TermId, TermRef, TermValue, WorkspaceReservation,
    };
    use purrdf_testkit::error_capture::{CallerRefusal, Capture};
    use std::sync::Arc;

    const ORIGINAL: CallerRefusal = CallerRefusal { limit: 17 };

    #[derive(Clone, Copy)]
    enum Capability {
        Granted,
        Missing,
        FactoryRefused,
        GrowthRefused,
        PublicationRefused,
        SourceRefused,
    }

    struct View {
        inner: Arc<RdfDataset>,
        capability: Capability,
    }

    struct RefusedGrowth;
    impl WorkspaceReservation for RefusedGrowth {
        type Error = CallerRefusal;
        fn resize(&mut self, _: u64) -> Result<(), CallerRefusal> {
            Err(ORIGINAL)
        }
    }

    struct RefusedPublication;
    impl WorkspaceReservation for RefusedPublication {
        type Error = CallerRefusal;
        fn resize(&mut self, bytes: u64) -> Result<(), CallerRefusal> {
            if bytes > u64::try_from("urn:purrdf:rdfc:original".len()).unwrap() {
                Err(ORIGINAL)
            } else {
                Ok(())
            }
        }
    }

    impl DatasetView for View {
        type Id = TermId;
        type ReadError = CallerRefusal;
        type TermGuard<'a> = TermRef<'a>;
        type ProbePlan = ();
        fn read_error(&self) -> Option<CallerRefusal> {
            matches!(self.capability, Capability::SourceRefused).then_some(ORIGINAL)
        }
        fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.quads()
        }
        fn resolve(&self, id: TermId) -> Result<TermRef<'_>, CallerRefusal> {
            Ok(self.inner.as_ref().resolve(id))
        }
        fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, CallerRefusal> {
            Ok(self.inner.as_ref().term_id_by_value(value))
        }
        fn capabilities(&self) -> RdfStoreCapabilities {
            self.inner.capabilities()
        }
        fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch) {}
        fn quads_for_pattern_with_plan(
            &self,
            (): &(),
            s: Option<TermId>,
            p: Option<TermId>,
            o: Option<TermId>,
            g: GraphMatch,
        ) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.quads_for_pattern(s, p, o, g)
        }
        fn reserve_owned_workspace(
            &self,
            _: u64,
        ) -> Option<Result<OwnedWorkspaceReservation<CallerRefusal>, CallerRefusal>> {
            match self.capability {
                Capability::Missing => None,
                Capability::FactoryRefused => Some(Err(ORIGINAL)),
                Capability::GrowthRefused => Some(Ok(Box::new(RefusedGrowth))),
                Capability::PublicationRefused => Some(Ok(Box::new(RefusedPublication))),
                Capability::Granted | Capability::SourceRefused => {
                    Some(Ok(NoopReservation::owned()))
                }
            }
        }
        fn term_count(&self) -> u64 {
            self.inner.term_count()
        }
    }

    fn view(capability: Capability, reserved: bool) -> View {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder
            .intern_iri(if reserved {
                "urn:purrdf:rdfc:original"
            } else {
                "urn:healthy"
            })
            .unwrap();
        let p = builder.intern_iri("urn:predicate").unwrap();
        builder.push_quad(s, p, p, None).unwrap();
        View {
            inner: builder.freeze().unwrap(),
            capability,
        }
    }

    #[test]
    fn owned_refusal_requires_explicit_capability_without_a_ceiling_heuristic() {
        for capability in [
            Capability::Granted,
            Capability::Missing,
            Capability::FactoryRefused,
            Capability::GrowthRefused,
            Capability::PublicationRefused,
            Capability::SourceRefused,
        ] {
            let source = view(capability, true);
            assert!(source.storage_live_budget().is_none());
            let error =
                super::super::try_canonicalize_view(&source, CanonHash::Sha256).unwrap_err();
            drop(source);
            match capability {
                Capability::Granted => {
                    let CanonError::ReservedVocabulary(refusal) = error else {
                        panic!("expected owned refusal");
                    };
                    assert_eq!(refusal.iri(), "urn:purrdf:rdfc:original");
                }
                Capability::Missing => {
                    assert!(matches!(error, CanonError::OwnedWorkspaceUnavailable));
                }
                Capability::FactoryRefused
                | Capability::GrowthRefused
                | Capability::PublicationRefused
                | Capability::SourceRefused => {
                    let (cause, code) = match (&error, capability) {
                        (
                            CanonError::OwnedAdmission(cause),
                            Capability::FactoryRefused
                            | Capability::GrowthRefused
                            | Capability::PublicationRefused,
                        ) => (cause, "canonical-owned-admission"),
                        (CanonError::Read(cause), Capability::SourceRefused) => {
                            (cause, "canonical-read")
                        }
                        _ => panic!("expected the distinct admission or source category"),
                    };
                    let borrowed = std::error::Error::source(&error)
                        .unwrap()
                        .downcast_ref::<CallerRefusal>()
                        .unwrap();
                    assert!(std::ptr::eq(cause, borrowed));
                    assert_eq!(cause, &ORIGINAL);
                    let captured = Capture::read(&error);
                    assert_eq!(captured.code, code);
                    assert_eq!(captured.child("cause").code, "caller-refused");
                    assert!(matches!(
                        captured.child("cause").field("limit"),
                        purrdf_hash::error::ErrorValue::Unsigned(17)
                    ));
                }
            }
            if !matches!(capability, Capability::SourceRefused) {
                let healthy = view(capability, false);
                assert!(super::super::try_canonicalize_view(&healthy, CanonHash::Sha256).is_ok());
            }
        }
    }
}

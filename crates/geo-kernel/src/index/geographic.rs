// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepared physical relations over the original dataset projection.

use std::ops::ControlFlow;
use std::sync::Arc;

use purrdf_core::TermValue;
use purrdf_hash::{Domain, blake3::Hasher, frame::frame_le_into, hex::Digest32};

use super::{GeoIndex, PairWorker, RelationPair, SpatialRelation};
use crate::context::{PreparedSourceReceipt, WorkProgress};
use crate::{
    Crs, GeoBindingId, GeoError, GeoProfile, GeographicReference, MetricContext,
    MetricWorkObserver, PreparedGeometry, SemanticLawId,
};

const CONTENT: Domain = Domain::new(b"purrdf-geo/geographic-index-source/v1");
const LAW: Domain = Domain::new(b"purrdf-geo/geographic-index-law/v1");
const INDEX: Domain = Domain::new(b"purrdf-geo/geographic-index/v1");

/// Immutable geographic preparation of an exact dataset geometry projection.
/// Every original serialization is interpreted on one actual target surface;
/// projected sources retain their complete registered operation images.
/// The planar projection and its historical fingerprint remain unchanged.
#[derive(Clone, Debug)]
pub struct GeographicGeoIndex {
    source: Arc<GeoIndex>,
    prepared: crate::PreparedValues<crate::PreparedValues<PreparedGeometry>>,
    reference: GeographicReference,
    target: Crs,
    binding: GeoBindingId,
    content: Digest32,
    id: Digest32,
    retained: u64,
    preparation_work: u64,
    maximum_term_bytes: u64,
}

/// Actual complete geographic relation traversal costs, separate from its law.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeographicRelationReceipt {
    /// Entry pairs admitted by the conservative broad phase.
    pub candidate_pairs: u64,
    /// Prepared geometry pairings examined by the shared atlas narrow phase.
    pub geometry_pairs: u64,
    /// Complete invocation work, including original index source admission.
    pub work_items: u64,
    /// Complete invocation peak storage allowance, in bytes.
    pub workspace_peak: u64,
}

impl GeographicGeoIndex {
    /// Prepare every original source under explicit carrier bindings/chains.
    /// # Errors
    /// Refuses incompatible references, missing operations, uncertified source
    /// topology and incomplete work or storage admission, without a partial index.
    pub fn prepare(
        source: Arc<GeoIndex>,
        profile: &GeoProfile,
        target: Crs,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        Self::prepare_inner(source, profile, target, None, context, None)
    }

    /// Prepare the same immutable index with bounded external work charging.
    /// # Errors
    /// Adds the observer's original refusal to [`Self::prepare`].
    pub fn prepare_metered(
        source: Arc<GeoIndex>,
        profile: &GeoProfile,
        target: Crs,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_inner(source, profile, target, None, context, Some(observer))
    }

    /// Prepare original sources at an explicitly supplied exact decimal-year epoch.
    /// # Errors
    /// Uses the complete reference, operation and admission contract of [`Self::prepare`].
    pub fn prepare_with_epoch(
        source: Arc<GeoIndex>,
        profile: &GeoProfile,
        target: Crs,
        epoch: &crate::Rat,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        Self::prepare_inner(source, profile, target, Some(epoch), context, None)
    }

    /// Prepare the same declared epoch with bounded external work charging.
    /// # Errors
    /// Adds observer refusal to [`Self::prepare_with_epoch`].
    pub fn prepare_with_epoch_metered(
        source: Arc<GeoIndex>,
        profile: &GeoProfile,
        target: Crs,
        epoch: &crate::Rat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_inner(
            source,
            profile,
            target,
            Some(epoch),
            context,
            Some(observer),
        )
    }

    fn prepare_inner(
        source: Arc<GeoIndex>,
        profile: &GeoProfile,
        target: Crs,
        epoch: Option<&crate::Rat>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin(source.len())?;
        let work_base = context.work_items();
        let storage_base = context.retained_workspace_bytes();
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(context)?;
        let declared = profile.reference_with_progress(&target, context, &mut progress)?;
        let matches = crate::numerical::reference_matches(declared, context, &mut progress)?;
        let reference_binding = crate::numerical::reference_identity(context, &mut progress, None)?;
        if !matches || profile.policy() != context.policy() {
            return Err(progress.retained_message(
                "geographic index requires its actual target reference and original profile policy",
                GeoError::Config,
            ));
        }
        let mut hash = Hasher::new();
        hash.update(CONTENT.as_bytes());
        frame_le_into(&mut hash, target.as_str().as_bytes());
        frame_le_into(&mut hash, reference_binding.digest().as_bytes());
        for term in source.data.config.serializations() {
            frame_term(&mut hash, term, context, &mut progress)?;
        }
        match source.data.config.graph() {
            super::GraphSelector::Any => frame_le_into(&mut hash, b"any"),
            super::GraphSelector::Default => frame_le_into(&mut hash, b"default"),
            super::GraphSelector::Named(term) => {
                frame_le_into(&mut hash, b"named");
                frame_term(&mut hash, term, context, &mut progress)?;
            }
        }
        // The source is already immutable caller-owned storage. Only the
        // newly built preparation tables receive fresh native allocation owners.
        let mut prepared = crate::context::RetainedBuilder::default();
        let mut table_bytes = 0_u64;
        prepared.reserve_admitted(source.len(), context, &mut progress, &mut table_bytes)?;
        context.retain_current_preparation()?;
        let mut maximum_term_bytes = 0_u64;
        for entry in &source.data.entries {
            maximum_term_bytes = maximum_term_bytes.max(frame_term(
                &mut hash,
                entry.subject(),
                context,
                &mut progress,
            )?);
            frame_le_into(&mut hash, &(entry.geometries.len() as u64).to_be_bytes());
            let mut geometries = crate::context::RetainedBuilder::default();
            geometries.reserve_admitted(
                entry.geometries.len(),
                context,
                &mut progress,
                &mut table_bytes,
            )?;
            context.retain_current_preparation()?;
            for literal in &entry.geometries {
                let inventory = {
                    let limits = *context.policy().limits();
                    let mut limits = limits;
                    limits.max_work_items =
                        limits.max_work_items.saturating_sub(context.work_items());
                    limits.max_workspace_bytes = context.remaining_workspace();
                    let mut nested = progress.nested(
                        context.work_items(),
                        context.retained_workspace_bytes(),
                        context.workspace_peak(),
                    );
                    crate::prepared::source_inventory(
                        literal.geometry(),
                        &limits,
                        Some(&mut nested),
                    )?
                };
                context.charge_work(inventory.work_items)?;
                frame_le_into(&mut hash, literal.crs().as_str().as_bytes());
                frame_le_into(&mut hash, inventory.content_id.as_bytes());
                let geometry = {
                    let mut nested = progress.nested(
                        context.work_items(),
                        context.retained_workspace_bytes(),
                        context.workspace_peak(),
                    );
                    profile.prepare_transformed_literal_metered(
                        literal,
                        &target,
                        epoch,
                        context,
                        &mut nested,
                    )?
                };
                frame_le_into(&mut hash, geometry.id().as_bytes());
                geometries.push_reserved(geometry);
                progress.context_poll(context)?;
            }
            prepared.push_reserved(crate::PreparedValues::from_admitted(geometries));
        }
        for (position, pairs) in source.data.asserted.iter().enumerate() {
            frame_le_into(&mut hash, &(position as u64).to_be_bytes());
            frame_le_into(&mut hash, &(pairs.len() as u64).to_be_bytes());
            for (left, right) in pairs {
                maximum_term_bytes =
                    maximum_term_bytes.max(frame_term(&mut hash, left, context, &mut progress)?);
                maximum_term_bytes =
                    maximum_term_bytes.max(frame_term(&mut hash, right, context, &mut progress)?);
            }
        }
        context.retain_current_preparation()?;
        let reference = crate::numerical::reference_clone(context, &mut progress)?;
        context.retain_current_preparation()?;
        let content = Digest32::new(*hash.finalize().as_bytes());
        let binding = profile.binding_id_in_context(context, &mut progress)?;
        let mut identity = Hasher::new();
        identity.update(INDEX.as_bytes());
        frame_le_into(&mut identity, content.as_bytes());
        frame_le_into(&mut identity, binding.digest().as_bytes());
        frame_le_into(&mut identity, Self::law_id().digest().as_bytes());
        progress.context_poll(context)?;
        context.retain_current_preparation()?;
        Ok(Self {
            source,
            prepared: crate::PreparedValues::from_admitted(prepared),
            reference,
            target,
            binding,
            content,
            id: Digest32::new(*identity.finalize().as_bytes()),
            retained: context
                .retained_workspace_bytes()
                .saturating_sub(storage_base),
            preparation_work: context.work_items().saturating_sub(work_base),
            maximum_term_bytes,
        })
    }

    /// Physical relation/index output law, independent of admission limits.
    #[must_use]
    pub fn law_id() -> SemanticLawId {
        SemanticLawId::from_digest(crate::profile::hash_fields(LAW, [crate::atlas::topology_law_id().digest().as_bytes().as_slice(), b"original-serializations;explicit-target;existential-pairings;asserted-union;sorted-unique-rows;six-root-conservative-candidates".as_slice()]))
    }
    /// Complete original dataset/configuration content identity.
    #[must_use]
    pub const fn content_id(&self) -> Digest32 {
        self.content
    }
    /// Content, carrier binding and geographic relation law identity.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.id
    }
    /// Immutable carrier/operation registration identity; policy is excluded.
    #[must_use]
    pub const fn binding_id(&self) -> GeoBindingId {
        self.binding
    }
    /// The exact historical projection, including asserted relations.
    #[must_use]
    pub fn source(&self) -> &GeoIndex {
        &self.source
    }
    /// Actual declared target carrier.
    #[must_use]
    pub const fn target(&self) -> &Crs {
        &self.target
    }
    /// Actual target reference used by every prepared geometry.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        &self.reference
    }
    /// Complete conservative retained source/preparation storage allowance.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained
    }
    /// Actual construction work, carried separately from content identities.
    #[must_use]
    pub const fn preparation_work_items(&self) -> u64 {
        self.preparation_work
    }

    /// Original immutable index preparation, for a worker that already retains
    /// the complete construction admission.
    #[must_use]
    pub const fn source_receipt(&self) -> PreparedSourceReceipt {
        PreparedSourceReceipt::new(self.id, self.preparation_work, self.retained)
    }

    /// Complete sorted relation rows; every geometric pairing is a candidate.
    /// Six root faces conservatively cover every admitted source, so a missing
    /// tighter whole-curve broad phase never excludes a qualifying pair.
    /// # Errors
    /// Refuses any incomplete narrow phase or resource admission; no partial bag.
    pub fn relation_pairs(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        context: &mut MetricContext,
    ) -> Result<Vec<(TermValue, TermValue)>, GeoError> {
        self.relation_pairs_inner(relation, subject, object, None, context, None)
            .map(|(rows, _)| rows)
    }
    /// Same complete relation rows with bounded governor polling.
    /// # Errors
    /// Adds observer refusal to [`Self::relation_pairs`].
    pub fn relation_pairs_metered(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Vec<(TermValue, TermValue)>, GeoError> {
        self.relation_pairs_inner(relation, subject, object, None, context, Some(observer))
            .map(|(rows, _)| rows)
    }

    /// Complete relation rows with actual candidate and resource evidence.
    /// # Errors
    /// Returns precisely the failures of [`Self::relation_pairs`].
    pub fn relation_pairs_with_receipt(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        context: &mut MetricContext,
    ) -> Result<(Vec<(TermValue, TermValue)>, GeographicRelationReceipt), GeoError> {
        self.relation_pairs_inner(relation, subject, object, None, context, None)
    }

    /// Complete rows and traversal evidence with bounded governor polling.
    /// # Errors
    /// Adds observer refusal to [`Self::relation_pairs_with_receipt`].
    pub fn relation_pairs_with_receipt_metered(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(Vec<(TermValue, TermValue)>, GeographicRelationReceipt), GeoError> {
        self.relation_pairs_inner(relation, subject, object, None, context, Some(observer))
    }

    /// Query an index whose exact source preparation is already retained in
    /// this worker, preserving the construction baseline for following queries.
    /// # Errors
    /// Refuses an unrelated or missing source receipt and every traversal error.
    pub fn relation_pairs_prepared_with_receipt(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        receipt: &PreparedSourceReceipt,
        context: &mut MetricContext,
    ) -> Result<(Vec<(TermValue, TermValue)>, GeographicRelationReceipt), GeoError> {
        self.relation_pairs_inner(relation, subject, object, Some(receipt), context, None)
    }

    /// Query the same retained index with bounded work/cancellation callbacks.
    /// # Errors
    /// Adds observer refusal to [`Self::relation_pairs_prepared_with_receipt`].
    pub fn relation_pairs_prepared_with_receipt_metered(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        receipt: &PreparedSourceReceipt,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(Vec<(TermValue, TermValue)>, GeographicRelationReceipt), GeoError> {
        self.relation_pairs_inner(
            relation,
            subject,
            object,
            Some(receipt),
            context,
            Some(observer),
        )
    }

    /// Return the complete sorted relation as retained source ordinals, with
    /// original work and physical-owner evidence. No source term is copied until
    /// the consumer emits a row through its admitted term factory.
    ///
    /// # Errors
    /// Preserves reference, relation, physical and observer refusal.
    pub fn relation_pair_references_with_receipt_metered(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<
        (
            crate::PreparedValues<RelationPair>,
            GeographicRelationReceipt,
        ),
        GeoError,
    > {
        self.relation_pair_references_inner(
            relation,
            subject,
            object,
            None,
            context,
            Some(observer),
        )
    }

    fn relation_pairs_inner(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        receipt: Option<&PreparedSourceReceipt>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<(Vec<(TermValue, TermValue)>, GeographicRelationReceipt), GeoError> {
        self.relation_pair_references_inner(relation, subject, object, receipt, context, observer)
            .and_then(|(pairs, receipt)| {
                self.source
                    .materialize_pairs(relation, &pairs)
                    .map(|rows| (rows, receipt))
            })
    }

    fn relation_pair_references_inner(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        receipt: Option<&PreparedSourceReceipt>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<
        (
            crate::PreparedValues<RelationPair>,
            GeographicRelationReceipt,
        ),
        GeoError,
    > {
        if context.reference() != &self.reference {
            return Err(GeoError::config(
                "geographic index query requires its actual prepared target reference",
            ));
        }
        context.begin(1)?;
        if let Some(receipt) = receipt {
            if *receipt != self.source_receipt() {
                return Err(GeoError::config(
                    "geographic index source receipt does not match its immutable preparation",
                ));
            }
            receipt.validate_context(context)?;
        }
        let mut progress = WorkProgress::new(observer);
        let mut admitted = false;
        let result = (|| {
            if receipt.is_none() {
                context.admit_workspace(self.retained)?;
                context.charge_work(self.preparation_work)?;
                context.retain_current_preparation()?;
                admitted = true;
            }
            progress.context_poll(context)?;
            let mut worker = GeographicPairs {
                index: self,
                relation,
                context,
                progress,
                candidate_pairs: 0,
                geometry_pairs: 0,
                maximum_term_bytes: self.maximum_term_bytes,
            };
            let rows = self
                .source
                .relation_pairs_with(relation, subject, object, &mut worker)?;
            Ok((
                rows,
                GeographicRelationReceipt {
                    candidate_pairs: worker.candidate_pairs,
                    geometry_pairs: worker.geometry_pairs,
                    work_items: worker.context.work_items(),
                    workspace_peak: worker.context.workspace_peak(),
                },
            ))
        })();
        context.release_transient_workspace()?;
        if admitted {
            context.release_preparation_admission(self.preparation_work, self.retained)?;
        }
        result
    }
}

struct GeographicPairs<'index, 'context, 'observer> {
    index: &'index GeographicGeoIndex,
    relation: SpatialRelation,
    context: &'context mut MetricContext,
    progress: WorkProgress<'observer>,
    candidate_pairs: u64,
    geometry_pairs: u64,
    maximum_term_bytes: u64,
}
impl PairWorker for GeographicPairs<'_, '_, '_> {
    fn bindings(
        &mut self,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
    ) -> Result<(), GeoError> {
        for bound in [subject, object].into_iter().flatten() {
            self.maximum_term_bytes = self.maximum_term_bytes.max(term_inventory(
                bound,
                self.context,
                &mut self.progress,
            )?);
        }
        let comparisons = u64::from(self.index.source.len().bit_width())
            .saturating_add(1)
            .saturating_mul(2);
        self.charge_comparisons(comparisons)
    }
    fn reserve(
        &mut self,
        pairs: &mut crate::context::RetainedBuilder<RelationPair>,
        required: usize,
    ) -> Result<(), GeoError> {
        let Some(bytes) = pairs.replacement_bytes(required, &self.progress)? else {
            return Ok(());
        };
        let bytes = u64::try_from(bytes)
            .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?;
        let previous = u64::try_from(pairs.admitted_bytes())
            .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?;
        self.context.admit_workspace(bytes)?;
        let result = self
            .progress
            .context_poll(self.context)
            .and_then(|()| pairs.reserve(required, &mut self.progress));
        match result {
            Ok(()) => self.context.release_workspace(previous),
            Err(error) => {
                let _ = self.context.release_workspace(bytes);
                Err(error)
            }
        }
    }
    fn holds(&mut self, left: usize, right: usize) -> Result<bool, GeoError> {
        self.context.charge_work(1)?;
        self.candidate_pairs = self
            .candidate_pairs
            .checked_add(1)
            .ok_or(GeoError::ArithmeticOverflow("geographic candidate pairs"))?;
        self.progress.context_poll(self.context)?;
        for a in &self.index.prepared[left] {
            for b in &self.index.prepared[right] {
                self.geometry_pairs = self
                    .geometry_pairs
                    .checked_add(1)
                    .ok_or(GeoError::ArithmeticOverflow("geographic geometry pairings"))?;
                let (source_work, source_storage) = crate::atlas::relation_source_admission(a, b)?;
                let mut child = self
                    .context
                    .remaining_child_with_baseline(source_work, source_storage)?;
                let result = {
                    let mut nested = self.progress.nested(
                        self.context.work_items().saturating_sub(source_work),
                        self.context
                            .retained_workspace_bytes()
                            .saturating_sub(source_storage),
                        self.context.workspace_peak(),
                    );
                    crate::atlas::relate_prepared_metered(
                        a,
                        b,
                        [&a.source_receipt(), &b.source_receipt()],
                        &mut child,
                        &mut nested,
                    )
                };
                let matrix = self.progress.absorb_child_result_with_baseline(
                    self.context,
                    &child,
                    source_work,
                    source_storage,
                    result,
                )?;
                self.progress.context_poll(self.context)?;
                let [dimension_a, dimension_b] = matrix
                    .input_dimensions()
                    .map(|dimension| dimension.dimension().map_or(-1, i32::from));
                if self.relation.holds(&matrix, dimension_a, dimension_b) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    fn row(&mut self, subject: &TermValue, object: &TermValue, len: usize) -> Result<(), GeoError> {
        let limit = self.context.policy().limits().max_output_elements;
        if u64::try_from(len)
            .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?
            >= limit
        {
            return Err(GeoError::OutputExhausted { limit });
        }
        self.maximum_term_bytes =
            self.maximum_term_bytes
                .max(term_inventory(subject, self.context, &mut self.progress)?);
        self.maximum_term_bytes =
            self.maximum_term_bytes
                .max(term_inventory(object, self.context, &mut self.progress)?);
        self.progress.context_poll(self.context)
    }
    fn finish(&mut self, count: usize) -> Result<(), GeoError> {
        let comparisons = u64::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(u64::from(count.bit_width()) + 1))
            .ok_or(GeoError::ArithmeticOverflow("geographic row ordering"))?;
        self.context.charge_work(comparisons)?;
        self.progress.context_poll(self.context)
    }
    fn compare_terms(
        &mut self,
        left: &TermValue,
        right: &TermValue,
    ) -> Result<core::cmp::Ordering, GeoError> {
        self.charge_comparisons(1)?;
        self.progress.with_storage(0, |memory| {
            left.try_cmp_with_memory(right, memory)
                .map_err(GeoError::Storage)
        })
    }
}

impl GeographicPairs<'_, '_, '_> {
    fn charge_comparisons(&mut self, comparisons: u64) -> Result<(), GeoError> {
        let work = self
            .maximum_term_bytes
            .div_ceil(64)
            .checked_mul(16)
            .and_then(|work| work.checked_mul(comparisons))
            .ok_or(GeoError::ArithmeticOverflow(
                "geographic term comparison work",
            ))?;
        self.context.charge_work(work)?;
        self.progress.context_poll(self.context)
    }
}

pub(super) fn term_inventory(
    term: &TermValue,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<u64, GeoError> {
    let mut encoded = 0_u64;
    let result =
        progress.with_storage(0, |memory| {
            term.visit_terms_with_memory(
                |value, _| {
                    let text = match value {
                        TermValue::Iri(text) => text.capacity(),
                        TermValue::Blank { label, .. } => label.capacity(),
                        TermValue::Literal {
                            lexical_form,
                            datatype,
                            language,
                            ..
                        } => match lexical_form
                            .capacity()
                            .checked_add(datatype.capacity())
                            .and_then(|bytes| {
                                bytes.checked_add(language.as_ref().map_or(0, String::capacity))
                            }) {
                            Some(bytes) => bytes,
                            None => {
                                return Ok(ControlFlow::Break(GeoError::ArithmeticOverflow(
                                    "geographic term text",
                                )));
                            }
                        },
                        TermValue::Triple { .. } => 0,
                    };
                    let Some(bytes) = u64::try_from(text)
                        .ok()
                        .and_then(|text| text.checked_add(64))
                    else {
                        return Ok(ControlFlow::Break(GeoError::ArithmeticOverflow(
                            "geographic term framing",
                        )));
                    };
                    let result =
                        (|| {
                            context.charge_work(bytes.div_ceil(64).checked_mul(8).ok_or(
                                GeoError::ArithmeticOverflow("geographic term copy work"),
                            )?)?;
                            encoded = encoded
                                .checked_add(bytes)
                                .ok_or(GeoError::ArithmeticOverflow("geographic term encoding"))?;
                            Ok(())
                        })();
                    Ok(match result {
                        Ok(()) => ControlFlow::Continue(()),
                        Err(error) => ControlFlow::Break(error),
                    })
                },
                memory,
            )
            .map_err(GeoError::Storage)
        })?;
    match result {
        ControlFlow::Continue(()) => {
            progress.context_poll(context)?;
            Ok(encoded)
        }
        ControlFlow::Break(error) => Err(error),
    }
}

fn frame_term(
    hash: &mut Hasher,
    term: &TermValue,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<u64, GeoError> {
    let maximum = term_inventory(term, context, progress)?;
    let mut resident = 0;
    super::projection::native_memory(context, progress, &mut resident, |memory| {
        memory.try_scope(|memory| {
            let mut bytes = Vec::new();
            term.canonical_bytes_with_memory(&mut bytes, memory)?;
            frame_le_into(hash, &bytes);
            memory.release_vec(bytes)?;
            Ok(())
        })
    })?;
    context.retain_current_preparation()?;
    progress.context_poll(context)?;
    Ok(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
    use purrdf_iri::vocab::ogc;

    fn projection() -> Arc<GeoIndex> {
        let mut builder = RdfDatasetBuilder::new();
        let property = builder.intern_iri(ogc::geo::AS_WKT).unwrap();
        let equals = builder.intern_iri(ogc::geo::SF_EQUALS).unwrap();
        let left = builder.intern_iri("http://example.org/a").unwrap();
        let right = builder.intern_iri("http://example.org/b").unwrap();
        let asserted = builder.intern_iri("http://example.org/c").unwrap();
        for (subject, text) in [(left, "POINT(90 90)"), (right, "POINT(-90 90)")] {
            let value = builder
                .intern_literal(RdfLiteral::typed(text, ogc::geo::WKT_LITERAL))
                .unwrap();
            builder.push_quad(subject, property, value, None).unwrap();
        }
        builder.push_quad(left, equals, asserted, None).unwrap();
        let dataset = builder.freeze().unwrap();
        let config = super::super::GeoIndexConfig::new(
            vec![TermValue::iri(ogc::geo::AS_WKT)],
            super::super::GraphSelector::Any,
        )
        .unwrap();
        Arc::new(GeoIndex::from_dataset(&*dataset, crate::standard_vocabulary(), &config).unwrap())
    }

    fn prepare(source: Arc<GeoIndex>, profile: &GeoProfile) -> GeographicGeoIndex {
        let target = Crs::new(ogc::CRS84).unwrap();
        let mut context = MetricContext::new(
            profile.reference(&target).unwrap().clone(),
            profile.policy(),
        )
        .unwrap();
        GeographicGeoIndex::prepare(source, profile, target, &mut context).unwrap()
    }

    #[test]
    fn geographic_index_preserves_planar_projection_and_asserted_union() {
        let source = projection();
        let original_id = source.source_fingerprint();
        let planar = source
            .relation_pairs(SpatialRelation::SfEquals, None, None)
            .unwrap();
        assert_eq!(planar.len(), 3);
        let index = prepare(Arc::clone(&source), &GeoProfile::standard());
        assert_eq!(index.source().source_fingerprint(), original_id);
        assert_eq!(
            index
                .source()
                .relation_pairs(SpatialRelation::SfEquals, None, None)
                .unwrap(),
            planar
        );
        let mut context = MetricContext::wgs84().unwrap();
        let (rows, receipt) = index
            .relation_pairs_with_receipt(SpatialRelation::SfEquals, None, None, &mut context)
            .unwrap();
        let iri = |name: &str| TermValue::iri(format!("http://example.org/{name}"));
        assert_eq!(
            rows,
            vec![
                (iri("a"), iri("a")),
                (iri("a"), iri("b")),
                (iri("a"), iri("c")),
                (iri("b"), iri("a")),
                (iri("b"), iri("b"))
            ]
        );
        assert_eq!(receipt.candidate_pairs, 4);
        assert_eq!(receipt.geometry_pairs, 4);
        assert_eq!(receipt.work_items, context.work_items());
        assert_eq!(receipt.workspace_peak, context.workspace_peak());
        assert_eq!(context.preparation_work_items(), 0);
        assert_eq!(context.retained_workspace_bytes(), 0);
        assert_eq!(
            index
                .relation_pairs_with_receipt(SpatialRelation::SfEquals, None, None, &mut context)
                .unwrap(),
            (rows, receipt)
        );
    }

    #[test]
    fn geographic_index_admits_cold_reference_and_lookup_before_allocating() {
        let source = projection();
        let target = Crs::new(ogc::CRS84).unwrap();
        for limits in [
            crate::ExecutionLimits {
                max_work_items: 1,
                ..crate::ExecutionLimits::GEOMETRY
            },
            crate::ExecutionLimits {
                max_workspace_bytes: 1,
                ..crate::ExecutionLimits::GEOMETRY
            },
        ] {
            let profile = GeoProfile::standard().with_limits(limits).unwrap();
            let mut context =
                MetricContext::new(GeographicReference::wgs84(), profile.policy()).unwrap();
            let source = Arc::clone(&source);
            let target = target.clone();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = GeographicGeoIndex::prepare(source, &profile, target, &mut context);
            let allocations = window.close();
            assert!(matches!(
                result,
                Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
            ));
            assert_eq!(allocations.allocations, 0);
            assert_eq!(allocations.requested_bytes, 0);
            assert_eq!(context.retained_workspace_bytes(), 0);
        }
        let profile = GeoProfile::standard();
        // An independently constructed equivalent reference has a cold ID
        // cache even when the immutable profile declaration is already warm.
        let mut context = MetricContext::wgs84().unwrap();
        let index = GeographicGeoIndex::prepare(source, &profile, target, &mut context).unwrap();
        assert_eq!(index.reference, *profile.reference(index.target()).unwrap());
        assert!(context.preparation_work_items() > 0);
    }

    #[test]
    fn geographic_index_uses_actual_dimensions_of_constant_source_curves() {
        let mut builder = RdfDatasetBuilder::new();
        let property = builder.intern_iri(ogc::geo::AS_WKT).unwrap();
        for (name, text) in [
            ("a", "MULTILINESTRING((0 0,0 0),(1 0,1 0))"),
            ("b", "MULTILINESTRING((0 0,0 0),(2 0,2 0))"),
        ] {
            let subject = builder
                .intern_iri(&format!("http://example.org/{name}"))
                .unwrap();
            let value = builder
                .intern_literal(RdfLiteral::typed(text, ogc::geo::WKT_LITERAL))
                .unwrap();
            builder.push_quad(subject, property, value, None).unwrap();
        }
        let dataset = builder.freeze().unwrap();
        let config = super::super::GeoIndexConfig::new(
            vec![TermValue::iri(ogc::geo::AS_WKT)],
            super::super::GraphSelector::Any,
        )
        .unwrap();
        let source = Arc::new(
            GeoIndex::from_dataset(&*dataset, crate::standard_vocabulary(), &config).unwrap(),
        );
        let index = prepare(source, &GeoProfile::standard());
        let a = TermValue::iri("http://example.org/a");
        let b = TermValue::iri("http://example.org/b");
        let (rows, _) = index
            .relation_pairs_with_receipt(
                SpatialRelation::SfOverlaps,
                Some(&a),
                Some(&b),
                &mut MetricContext::wgs84().unwrap(),
            )
            .unwrap();
        assert_eq!(rows, vec![(a, b)]);
    }

    #[test]
    fn geographic_index_identity_excludes_admission_and_refusals_return_no_rows() {
        let source = projection();
        let profile = GeoProfile::standard();
        let first = prepare(Arc::clone(&source), &profile);
        let mut limits = *profile.policy().limits();
        limits.max_work_items *= 2;
        limits.max_workspace_bytes *= 2;
        let second = prepare(source, &profile.with_limits(limits).unwrap());
        assert_eq!(first.id(), second.id());
        assert_eq!(first.content_id(), second.content_id());
        let mut context = MetricContext::wgs84().unwrap();
        struct Stop;
        impl MetricWorkObserver for Stop {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        assert_eq!(
            first.relation_pairs_metered(
                SpatialRelation::SfEquals,
                None,
                None,
                &mut context,
                &mut Stop
            ),
            Err(GeoError::Cancelled)
        );
        assert_eq!(context.preparation_work_items(), 0);
        assert_eq!(context.retained_workspace_bytes(), 0);
        let mut limits = *context.policy().limits();
        limits.max_work_items = first.preparation_work_items();
        let mut context = MetricContext::new(
            first.reference().clone(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            first.relation_pairs(SpatialRelation::SfEquals, None, None, &mut context),
            Err(GeoError::WorkExhausted { .. })
        ));
        assert_eq!(context.retained_workspace_bytes(), 0);
    }

    #[test]
    fn prepared_index_queries_reuse_only_the_matching_construction_receipt() {
        let profile = GeoProfile::standard();
        let target = Crs::new(ogc::CRS84).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let index =
            GeographicGeoIndex::prepare(projection(), &profile, target, &mut context).unwrap();
        let source = index.source_receipt();
        let work = context.preparation_work_items();
        let retained = context.retained_workspace_bytes();
        let first = index
            .relation_pairs_prepared_with_receipt(
                SpatialRelation::SfEquals,
                None,
                None,
                &source,
                &mut context,
            )
            .unwrap();
        assert_eq!(first.0.len(), 5);
        assert_eq!(context.preparation_work_items(), work);
        assert_eq!(context.retained_workspace_bytes(), retained);
        assert_eq!(
            index
                .relation_pairs_prepared_with_receipt(
                    SpatialRelation::SfEquals,
                    None,
                    None,
                    &source,
                    &mut context,
                )
                .unwrap(),
            first
        );
        assert!(matches!(
            index.relation_pairs_prepared_with_receipt(
                SpatialRelation::SfEquals,
                None,
                None,
                &index.prepared[0][0].source_receipt(),
                &mut context,
            ),
            Err(GeoError::Config(_))
        ));
        assert!(matches!(
            index.relation_pairs_prepared_with_receipt(
                SpatialRelation::SfEquals,
                None,
                None,
                &source,
                &mut MetricContext::wgs84().unwrap(),
            ),
            Err(GeoError::Config(_))
        ));
    }
}

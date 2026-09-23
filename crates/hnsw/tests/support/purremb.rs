// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A real PURREMB fixture for the HNSW integration tests.
//!
//! This is the same construction `crates/rdf-core/tests/purremb_indexes.rs` uses: a
//! certified `.purrpck` source, one embedding family, a target set of one dataset target
//! plus caller extension targets, and one `f32` matrix. On top of it this module builds an
//! HNSW index through the typed guard adapter, commits it into a second artifact, and
//! exposes the bytes plus the row-ordered matrix and terms. Tests therefore exercise the
//! real serialized sections, not a mock format.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_core::IndexGuardView;
use purrdf_core::distance::Arithmetic;
use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, DerivedIndex, DimensionalityPolicy, DistanceMetric,
    EffectivePrefix, EmbeddingBuilder, EmbeddingFamily, EmbeddingFamilyContract, EmbeddingTarget,
    EmbeddingView, ExtensionTarget, IndexBuildDeterminism, IndexCoordinates, IndexGuardContract,
    IndexLossContract, IndexPayloadStorage, MatrixInput, MatrixRow, PURREMB_HEADER_LENGTH,
    PrefixPostprocessing, RdfDatasetBuilder, SECTION_INDEX_PAYLOAD, StageImplementation, TargetId,
    TargetSet, TargetSetId, TermValue, VectorDtype, VectorSpaceId, derive_artifact_root,
    verify_embedding,
};
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix, guard, level::splitmix64};

/// Directory entry length in the PURREMB v1 framing.
const DIRECTORY_ENTRY_LENGTH: usize = 64;

/// One fixture context: everything needed to rebuild the artifact with a different index.
struct Context {
    source: CertifiedPurrpckSource,
    contract: EmbeddingFamilyContract,
    family: EmbeddingFamily,
    targets: Vec<EmbeddingTarget>,
    target_set: TargetSet,
    matrix: MatrixInput<f32>,
}

/// A built fixture: the artifact with the HNSW index, the row-ordered data, and the
/// identity of the index it carries.
pub struct Fixture {
    context: Context,
    /// The artifact carrying exactly one HNSW derived index.
    pub bytes: Vec<u8>,
    /// The same artifact with no derived index at all.
    pub without_index: Vec<u8>,
    /// The row-ordered vectors, in ascending `TargetId` order.
    pub matrix: VectorMatrix,
    /// The parameters the committed index was built under.
    pub params: Params,
    /// The exact coordinates the committed index is bound to.
    pub coordinates: IndexCoordinates,
    /// The committed derived index, so a test can rebuild with it plus a second one.
    pub derived: DerivedIndex,
    /// The canonical payload image of the committed index.
    pub image: Vec<u8>,
    /// The target set the index covers.
    pub target_set: TargetSetId,
    /// The vector space the index accelerates.
    pub vector_space: VectorSpaceId,
    /// The RDF term of each row, in row order.
    pub terms: Vec<TermValue>,
    /// The target id of each row, in row order.
    pub row_targets: Vec<TargetId>,
    /// The PURREMB role this artifact's projection puts the index in.
    pub use_role: purrdf_core::IndexUseRole,
    /// The guard contract the committed index was folded under.
    pub guard_contract: IndexGuardContract,
}

impl Fixture {
    /// Build the complete two-pass fixture: artifact without an index, read the effective
    /// matrix, build HNSW over it, recommit the artifact with the guard.
    #[must_use]
    pub fn new(rows: usize, dims: usize, params: Params) -> Self {
        Self::with_prefix(rows, dims, dims, params)
    }

    /// The same fixture whose declared effective prefix is `prefix` of `dims` stored
    /// coordinates.
    ///
    /// When `prefix < dims` the artifact puts any index built over it in PURREMB's
    /// coarse-prefix-retrieval role: the graph decided its answer on a truncation of the
    /// vectors the artifact stores. Equal values are the ordinary full-width case.
    #[must_use]
    pub fn with_prefix(rows: usize, dims: usize, prefix: usize, params: Params) -> Self {
        Self::assemble(rows, dims, prefix, params, HnswIndex::build, guard::load)
    }

    /// The fixture whose committed index is built under the reassociated arithmetic, so its
    /// guard names the reassociated implementation and the dispatch path of this process.
    #[must_use]
    pub fn new_reassociated(rows: usize, dims: usize, params: Params) -> Self {
        Self::assemble(
            rows,
            dims,
            dims,
            params,
            HnswIndex::build_reassociated,
            guard::load_reassociated,
        )
    }

    /// The two-pass construction, with the index built by `build` and read back by `load`.
    fn assemble<A: Arithmetic>(
        rows: usize,
        dims: usize,
        prefix: usize,
        params: Params,
        build_index: fn(VectorMatrix, &DistanceMetric, Params) -> purrdf_hnsw::Result<HnswIndex<A>>,
        load: fn(&IndexGuardView<'_>, VectorMatrix) -> purrdf_hnsw::Result<HnswIndex<A>>,
    ) -> Self {
        assert!(rows > 0 && dims > 0, "the fixture shape is nonempty");
        assert!(
            prefix > 0 && prefix <= dims,
            "a prefix fits inside the width"
        );
        let context = context(rows, dims, prefix);
        let without_index = build(&context, Vec::new());

        let mut view = EmbeddingView::from_bytes(&without_index).expect("the base artifact opens");
        verify_embedding(&mut view).expect("the base artifact verifies");
        let target_set = context.target_set.id;
        let vector_space = context.family.spaces[0].id;
        let effective = view
            .effective_matrix(target_set, vector_space)
            .expect("the effective matrix is readable")
            .expect("the effective matrix exists");
        let matrix = guard::read_effective_matrix(&effective).expect("the matrix reads");
        let coordinates =
            guard::coordinates(&view, target_set, vector_space).expect("the coordinates derive");
        let index = build_index(matrix.clone(), &DistanceMetric::SquaredEuclidean, params)
            .expect("the HNSW fixture builds");
        let image = index.canonical_image();
        let role = guard::use_role(&effective);
        let guard_contract = guard::guard_contract_for(&index, role);
        let derived =
            guard::derived_index(coordinates, &index, role).expect("the derived index folds");
        let bytes = build(&context, vec![derived.clone()]);

        let mut committed = EmbeddingView::from_bytes(&bytes).expect("the indexed artifact opens");
        verify_embedding(&mut committed).expect("the indexed artifact verifies");
        let selected = guard::select(&committed).expect("exactly one HNSW guard");
        let _ = load(&selected, matrix.clone()).expect("the payload decodes");

        let row_targets = context.target_set.targets.clone();
        let terms = (0..rows)
            .map(|row| TermValue::iri(format!("https://example.org/index/target/{row}")))
            .collect();

        Self {
            context,
            bytes,
            without_index,
            matrix,
            params,
            coordinates,
            derived,
            image,
            target_set,
            vector_space,
            terms,
            row_targets,
            use_role: role,
            guard_contract,
        }
    }

    /// Rebuild the artifact with the committed index plus a second HNSW index under
    /// `params`, for the ambiguity test.
    #[must_use]
    pub fn with_second_index(&self, params: Params) -> Vec<u8> {
        let index = HnswIndex::build(
            self.matrix.clone(),
            &DistanceMetric::SquaredEuclidean,
            params,
        )
        .expect("the second HNSW fixture builds");
        let second = guard::derived_index(self.coordinates, &index, self.use_role)
            .expect("the second index folds");
        build(&self.context, vec![self.derived.clone(), second])
    }

    /// The artifact with one payload byte flipped, resealed so the framing still verifies
    /// structurally — a substituted payload the adapter's commitment check must refuse.
    #[must_use]
    pub fn tampered_payload(&self) -> Vec<u8> {
        let mut bytes = self.bytes.clone();
        let (offset, _length) = section_span(&bytes, SECTION_INDEX_PAYLOAD, 1);
        bytes[offset] ^= 0x01;
        reseal(&mut bytes, &[(SECTION_INDEX_PAYLOAD, 1)]);
        bytes
    }

    /// The artifact whose HNSW guard publishes some OTHER implementation evidence revision.
    ///
    /// Identifier, media type, profile digest, parameter encoding, parameters, loss contract,
    /// use role and payload are all the profile's own, so the revision bytes are the single
    /// thing a test over this artifact is exercising.
    #[must_use]
    pub fn foreign_evidence_revision(&self) -> Vec<u8> {
        self.with_guard_contract(|contract| {
            contract.implementation.revision =
                Some(b"approximate: recall is exact at every scale and absence is proven".to_vec());
        })
    }

    /// The artifact whose HNSW guard carries `identity` as its implementation, every other
    /// field -- parameters, loss contract, role, payload -- the fixture's own.
    #[must_use]
    pub fn with_implementation(&self, identity: ArtifactIdentity) -> Vec<u8> {
        self.with_guard_contract(|contract| contract.implementation = identity)
    }

    /// The artifact whose HNSW guard declares a loss contract that transforms vectors.
    ///
    /// PURREMB requires an encoding and parameters alongside that claim, so both are supplied
    /// -- the artifact is well-formed at the container boundary and false only about this
    /// profile, which stores no vectors and therefore cannot transform any.
    #[must_use]
    pub fn transforming_loss_contract(&self) -> Vec<u8> {
        self.with_guard_contract(|contract| {
            contract.loss = IndexLossContract {
                transforms_vectors: true,
                loss_encoding: Some("https://example.org/index/loss/int8".to_owned()),
                loss_parameters: Some(vec![8]),
            };
        })
    }

    /// The artifact rebuilt with this profile's guard contract mutated by `mutate`.
    ///
    /// The builder is the real one, so the result is a properly sealed artifact that opens and
    /// verifies; nothing is patched behind rdf-core's back. The unmutated contract is rebuilt
    /// first and asserted byte-identical to `self.bytes`, which is what makes the mutation the
    /// ONLY difference between the two artifacts: a refusal observed over the result cannot be
    /// firing for something else the rebuild happened to change.
    fn with_guard_contract(&self, mutate: impl FnOnce(&mut IndexGuardContract)) -> Vec<u8> {
        let mut contract = self.guard_contract.clone();
        assert_eq!(
            self.rebuild(&contract),
            self.bytes,
            "the unmutated contract must reproduce the fixture byte for byte"
        );
        mutate(&mut contract);
        self.rebuild(&contract)
    }

    /// One artifact carrying exactly this fixture's payload under `contract`.
    fn rebuild(&self, contract: &IndexGuardContract) -> Vec<u8> {
        let derived = DerivedIndex::new(
            self.coordinates,
            IndexPayloadStorage::Inline(self.image.clone()),
            IndexBuildDeterminism::Deterministic,
            contract,
        )
        .expect("the substituted guard folds into a derived index");
        build(&self.context, vec![derived])
    }

    /// Bind every row target to its row term, in row order.
    #[must_use]
    pub fn bindings(&self) -> Vec<(TargetId, TermValue)> {
        self.row_targets
            .iter()
            .copied()
            .zip(self.terms.iter().cloned())
            .collect()
    }
}

/// Build a fixture context with `rows` targets and `dims` dimensions.
fn context(rows: usize, dims: usize, prefix: usize) -> Context {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty dataset");
    let (source, _source_bytes) =
        CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let dataset_target = source.dataset_target(true).expect("dataset target");

    let mut targets = vec![dataset_target];
    for index in 1..rows {
        let extension = ExtensionTarget {
            kind_identifier: format!("https://example.org/index/target-kind/{index}"),
            payload_encoding: "application/octet-stream".into(),
            payload: vec![u8::try_from(index % 256).expect("small index")],
        }
        .into_target(true)
        .expect("extension target");
        targets.push(extension);
    }

    let target_set = TargetSet::new(targets.iter().map(|target| target.id).collect())
        .expect("target set is nonempty and distinct");

    let contract = EmbeddingFamilyContract {
        model: artifact("model"),
        engine: artifact("engine"),
        tokenizer: artifact("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::SquaredEuclidean,
        // A shorter effective prefix is only well-formed alongside the stored-dimension
        // projection it truncates -- that pair IS the Matryoshka policy, and PURREMB refuses
        // a lone short prefix outright. Equal values are the ordinary fixed case.
        dimensionality: if prefix < dims {
            DimensionalityPolicy::Matryoshka(vec![
                EffectivePrefix {
                    dimension: u32::try_from(prefix).expect("the prefix fits u32"),
                    postprocessing: PrefixPostprocessing::None,
                },
                EffectivePrefix {
                    dimension: u32::try_from(dims).expect("dims fit u32"),
                    postprocessing: PrefixPostprocessing::None,
                },
            ])
        } else {
            DimensionalityPolicy::Fixed(EffectivePrefix {
                dimension: u32::try_from(dims).expect("dims fit u32"),
                postprocessing: PrefixPostprocessing::None,
            })
        },
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family derives");

    let mut state = 0x0dd0_c0de_0dd0_c0de_u64;
    let rows_data = targets
        .iter()
        .map(|target| {
            let values = (0..dims)
                .map(|_| {
                    state = splitmix64(state);
                    let unit = (state >> 11) as f32 / (1_u64 << 53) as f32;
                    let value = unit.mul_add(2.0, -1.0);
                    if value == 0.0 { 0.25 } else { value }
                })
                .collect();
            MatrixRow::new(target.id, values)
        })
        .collect();
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: target_set.id,
        stored_dimension: u32::try_from(dims).expect("dims fit u32"),
        rows: rows_data,
        projections: family
            .spaces
            .iter()
            .map(|space| {
                purrdf_core::ProjectionSpec::derive(
                    space.family_id,
                    space.dimension,
                    space.postprocessing,
                )
            })
            .collect(),
    };

    Context {
        source,
        contract,
        family,
        targets,
        target_set,
        matrix,
    }
}

/// Build one artifact from the context and the given derived indexes.
fn build(context: &Context, indexes: Vec<DerivedIndex>) -> Vec<u8> {
    let metadata = CanonicalMetadataInput {
        source: context.source,
        family_contracts: vec![context.contract.clone()],
        targets: context.targets.clone(),
        target_sets: vec![context.target_set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes,
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(context.matrix.clone());
    builder.build().expect("the indexed artifact builds").bytes
}

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/index/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        None,
        ArtifactIdentityKind::Single,
    )
    .expect("artifact")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/index/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/cbor",
            vec![1, 2],
        )
        .expect("stage"),
    )
}

// ---------------------------------------------------------------------------
// Framing helpers, shared with `crates/rdf-core/tests/purremb_indexes.rs`
// ---------------------------------------------------------------------------

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32 field"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64 field"))
}

fn directory_entry(bytes: &[u8], kind: u32, instance: u32) -> usize {
    let count = usize::try_from(read_u32(bytes, 20)).expect("section count");
    (0..count)
        .map(|index| PURREMB_HEADER_LENGTH as usize + index * DIRECTORY_ENTRY_LENGTH)
        .find(|offset| read_u32(bytes, *offset) == kind && read_u32(bytes, *offset + 8) == instance)
        .expect("section directory entry")
}

fn section_span(bytes: &[u8], kind: u32, instance: u32) -> (usize, usize) {
    let entry = directory_entry(bytes, kind, instance);
    (
        usize::try_from(read_u64(bytes, entry + 16)).expect("section offset"),
        usize::try_from(read_u64(bytes, entry + 24)).expect("section length"),
    )
}

fn reseal(bytes: &mut [u8], sections: &[(u32, u32)]) {
    for &(kind, instance) in sections {
        let entry = directory_entry(bytes, kind, instance);
        let (offset, length) = section_span(bytes, kind, instance);
        let digest = ContentDigest::of(&bytes[offset..offset + length]);
        bytes[entry + 32..entry + 64].copy_from_slice(digest.as_bytes());
    }
    let count = usize::try_from(read_u32(bytes, 20)).expect("section count");
    let directory_end = PURREMB_HEADER_LENGTH as usize + count * DIRECTORY_ENTRY_LENGTH;
    let mut header = [0u8; PURREMB_HEADER_LENGTH as usize];
    header.copy_from_slice(&bytes[..PURREMB_HEADER_LENGTH as usize]);
    header[64..96].fill(0);
    let root = derive_artifact_root(
        &header,
        &bytes[PURREMB_HEADER_LENGTH as usize..directory_end],
    );
    bytes[64..96].copy_from_slice(root.as_bytes());
    let trailer = usize::try_from(read_u64(bytes, 48)).expect("trailer offset");
    bytes[trailer + 24..trailer + 56].copy_from_slice(root.as_bytes());
}

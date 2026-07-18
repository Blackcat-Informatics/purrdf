// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared deterministic fixtures for PURREMB companion-format benchmarks.
//!
//! The primary fixture is a 16,384 x 384 binary32 Matryoshka matrix. A smaller
//! binary64 fixture exercises the other v1 scalar representation. The catalog
//! fixture defaults to exactly one million digest-only chunk subjects spread
//! across 4,096 retained document shards. Set `PURREMB_CATALOG_SUBJECTS` only
//! for local smoke runs; published observations use the one-million default.

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::io::Cursor;

use purrdf_core::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, CanonicalMetadataInput,
    CertifiedPurrpckSource, ContentDigest, CorpusTarget, DimensionalityPolicy, DistanceMetric,
    DocumentTarget, EffectiveMatrixView, EffectivePrefix, EmbeddingBuilder,
    EmbeddingFamilyContract, EmbeddingStreamWriter, EmbeddingTarget, EmbeddingView,
    ExtensionTarget, MatrixCommitment, MatrixInput, MatrixRow, PrefixPostprocessing,
    ProjectionCommitment, ProjectionSpec, RdfDatasetBuilder, RelationKind, StageImplementation,
    TargetId, TargetRelation, TargetSet, TextChunkTarget, VectorDtype, VectorSpaceId,
    verify_embedding,
};

pub(crate) const F32_ROWS: usize = 16_384;
pub(crate) const F32_DIMENSION: usize = 384;
pub(crate) const COARSE_DIMENSION: u32 = 64;
const RAW_DIMENSION: u32 = 32;
pub(crate) const FULL_DIMENSION: u32 = 384;
pub(crate) const F64_ROWS: usize = 4_096;
pub(crate) const F64_DIMENSION: usize = 128;
const DEFAULT_CATALOG_CHUNKS: usize = 1_000_000;
const CATALOG_DOCUMENTS: usize = 4_096;
pub(crate) const RERANK_CANDIDATES: usize = 128;
pub(crate) const RECALL_K: usize = 10;

pub(crate) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn artifact(name: &str) -> ArtifactIdentity {
    ArtifactIdentity::new(
        format!("https://example.org/bench/purremb/{name}"),
        "application/octet-stream",
        ContentDigest::of(name.as_bytes()),
        Some(b"deterministic-benchmark-v1".to_vec()),
        ArtifactIdentityKind::Single,
    )
    .expect("benchmark artifact identity")
}

fn stage(name: &str) -> AppliedStage {
    AppliedStage::Applied(
        StageImplementation::new(
            format!("https://example.org/bench/purremb/{name}"),
            ContentDigest::of(name.as_bytes()),
            "application/cbor",
            vec![0xa1, 0x61, b'v', 0x01],
        )
        .expect("benchmark stage"),
    )
}

fn source() -> (CertifiedPurrpckSource, EmbeddingTarget) {
    let dataset = RdfDatasetBuilder::new().freeze().expect("empty RDF source");
    let (source, _) = CertifiedPurrpckSource::from_dataset(&dataset).expect("source pack");
    let target = source.dataset_target(true).expect("source dataset target");
    (source, target)
}

fn f32_contract() -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: artifact("f32-model"),
        engine: artifact("engine"),
        tokenizer: artifact("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("extension-subject-projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::Cosine,
        dimensionality: DimensionalityPolicy::matryoshka(vec![
            EffectivePrefix {
                dimension: RAW_DIMENSION,
                postprocessing: PrefixPostprocessing::None,
            },
            EffectivePrefix {
                dimension: COARSE_DIMENSION,
                postprocessing: PrefixPostprocessing::DeterministicL2,
            },
            EffectivePrefix {
                dimension: FULL_DIMENSION,
                postprocessing: PrefixPostprocessing::DeterministicL2,
            },
        ])
        .expect("Matryoshka dimensions"),
        extensions: Vec::new(),
    }
}

fn f64_contract() -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: artifact("f64-model"),
        engine: artifact("engine"),
        tokenizer: artifact("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("extension-subject-projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F64,
        metric: DistanceMetric::SquaredEuclidean,
        dimensionality: DimensionalityPolicy::fixed(
            u32::try_from(F64_DIMENSION).expect("f64 dimension"),
            PrefixPostprocessing::None,
        )
        .expect("fixed f64 dimension"),
        extensions: Vec::new(),
    }
}

fn extension_targets(label: &str, count: usize) -> Vec<EmbeddingTarget> {
    (0..count)
        .map(|index| {
            let mut payload = label.as_bytes().to_vec();
            payload.extend_from_slice(&usize_to_u64(index).to_le_bytes());
            ExtensionTarget {
                kind_identifier: "https://example.org/bench/subject".into(),
                payload_encoding: "application/octet-stream".into(),
                payload,
            }
            .into_target(false)
            .expect("extension target")
        })
        .collect()
}

fn coordinate_word(target: TargetId, column: usize) -> i16 {
    let bytes = target.as_bytes();
    let first = (column * 2) % bytes.len();
    let second = (first + 1) % bytes.len();
    let word = u16::from_le_bytes([bytes[first], bytes[second]]);
    let column = u16::try_from(column).expect("benchmark dimension fits u16");
    let mixed = word
        .wrapping_add(column.wrapping_mul(7_919))
        .rotate_left(u32::from(column % 15));
    i16::try_from(mixed % 2_001).expect("coordinate fits i16") - 1_000
}

fn f32_coordinate(target: TargetId, column: usize) -> f32 {
    let direct = i32::from(coordinate_word(target, column));
    let coarse = usize::try_from(COARSE_DIMENSION).expect("coarse dimension");
    let coordinate = if column < coarse {
        direct
    } else {
        let nested = i32::from(coordinate_word(target, column % coarse));
        (nested * 9 + direct) / 10
    };
    let value =
        f32::from(i16::try_from(coordinate).expect("blended coordinate fits i16")) / 1_000.0;
    if column == 0 { value + 0.03125 } else { value }
}

fn f64_coordinate(target: TargetId, column: usize) -> f64 {
    let value = f64::from(coordinate_word(target, column)) / 1_000.0;
    if column == 0 { value + 0.03125 } else { value }
}

fn matrix_commitment(view: &EmbeddingView<'_>) -> MatrixCommitment {
    let matrix = view.matrices().next().expect("one benchmark matrix");
    MatrixCommitment {
        matrix_id: matrix.id(),
        content_digest: matrix.content_digest(),
        target_set_id: matrix.target_set_id(),
        family_id: matrix.family_id(),
        dtype: matrix.dtype().expect("matrix dtype"),
        row_count: matrix.row_count(),
        stored_dimension: matrix.stored_dimension(),
        projections: view
            .projections()
            .filter(|projection| projection.matrix_id() == matrix.id())
            .map(|projection| ProjectionCommitment {
                projection_id: projection.id(),
                content_digest: projection.content_digest(),
                vector_space_id: projection.vector_space_id(),
                effective_dimension: projection.effective_dimension(),
                postprocessing: projection.postprocessing().expect("postprocessing"),
            })
            .collect(),
    }
}

pub(crate) struct F32Fixture {
    pub(crate) bytes: Vec<u8>,
    source: CertifiedPurrpckSource,
    contract: EmbeddingFamilyContract,
    targets: Vec<EmbeddingTarget>,
    pub(crate) target_set: TargetSet,
    pub(crate) commitment: MatrixCommitment,
    pub(crate) row_values: Vec<f32>,
    pub(crate) raw_space: VectorSpaceId,
    pub(crate) coarse_space: VectorSpaceId,
    pub(crate) full_space: VectorSpaceId,
}

impl F32Fixture {
    pub(crate) fn metadata(&self) -> CanonicalMetadataInput {
        CanonicalMetadataInput {
            source: self.source,
            family_contracts: vec![self.contract.clone()],
            targets: self.targets.clone(),
            target_sets: vec![self.target_set.clone()],
            relations: Vec::new(),
            token_spans: Vec::new(),
            external_bindings: Vec::new(),
            indexes: Vec::new(),
            extensions: Vec::new(),
        }
    }

    pub(crate) fn stream_once(&self) -> Vec<u8> {
        let output = Cursor::new(Vec::with_capacity(self.bytes.len()));
        let mut writer = EmbeddingStreamWriter::from_typed_metadata(
            output,
            self.metadata(),
            vec![self.commitment.clone()],
        )
        .expect("stream writer");
        writer
            .write_f32_matrix(
                self.target_set
                    .targets
                    .iter()
                    .copied()
                    .zip(self.row_values.chunks_exact(F32_DIMENSION)),
            )
            .expect("stream matrix");
        let (output, _) = writer.finish().expect("finish stream");
        output.into_inner()
    }
}

pub(crate) fn build_f32_fixture() -> F32Fixture {
    let (source, dataset_target) = source();
    let contract = f32_contract();
    let family = contract.derive().expect("f32 family");
    let row_targets = extension_targets("f32", F32_ROWS);
    let target_set = TargetSet::new(row_targets.iter().map(|target| target.id).collect())
        .expect("f32 target set");
    let mut targets = Vec::with_capacity(row_targets.len() + 1);
    targets.push(dataset_target);
    targets.extend(row_targets);
    let mut row_values = Vec::with_capacity(F32_ROWS * F32_DIMENSION);
    for target in &target_set.targets {
        row_values.extend((0..F32_DIMENSION).map(|column| f32_coordinate(*target, column)));
    }
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: target_set.id,
        stored_dimension: u32::try_from(F32_DIMENSION).expect("f32 dimension"),
        rows: target_set
            .targets
            .iter()
            .copied()
            .zip(row_values.chunks_exact(F32_DIMENSION))
            .map(|(target, values)| MatrixRow::new(target, values.to_vec()))
            .collect(),
        projections: family
            .spaces
            .iter()
            .map(|space| {
                ProjectionSpec::derive(space.family_id, space.dimension, space.postprocessing)
            })
            .collect(),
    };
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract.clone()],
        targets: targets.clone(),
        target_sets: vec![target_set.clone()],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(matrix);
    let bytes = builder.build().expect("f32 benchmark artifact").bytes;
    let mut view = EmbeddingView::from_bytes(&bytes).expect("f32 benchmark view");
    verify_embedding(&mut view).expect("verified f32 benchmark artifact");
    let commitment = matrix_commitment(&view);
    let raw_space = family.spaces[0].id;
    let coarse_space = family.spaces[1].id;
    let full_space = family.spaces[2].id;
    F32Fixture {
        bytes,
        source,
        contract,
        targets,
        target_set,
        commitment,
        row_values,
        raw_space,
        coarse_space,
        full_space,
    }
}

pub(crate) struct F64Fixture {
    pub(crate) bytes: Vec<u8>,
}

pub(crate) fn build_f64_fixture() -> F64Fixture {
    let (source, dataset_target) = source();
    let contract = f64_contract();
    let family = contract.derive().expect("f64 family");
    let row_targets = extension_targets("f64", F64_ROWS);
    let target_set = TargetSet::new(row_targets.iter().map(|target| target.id).collect())
        .expect("f64 target set");
    let mut targets = Vec::with_capacity(row_targets.len() + 1);
    targets.push(dataset_target);
    targets.extend(row_targets);
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: target_set.id,
        stored_dimension: u32::try_from(F64_DIMENSION).expect("f64 dimension"),
        rows: target_set
            .targets
            .iter()
            .copied()
            .map(|target| {
                MatrixRow::new(
                    target,
                    (0..F64_DIMENSION)
                        .map(|column| f64_coordinate(target, column))
                        .collect(),
                )
            })
            .collect(),
        projections: vec![ProjectionSpec::derive(
            family.id,
            u32::try_from(F64_DIMENSION).expect("f64 dimension"),
            PrefixPostprocessing::None,
        )],
    };
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets,
        target_sets: vec![target_set],
        relations: Vec::new(),
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f64_matrix(matrix);
    let bytes = builder.build().expect("f64 benchmark artifact").bytes;
    let mut view = EmbeddingView::from_bytes(&bytes).expect("f64 benchmark view");
    verify_embedding(&mut view).expect("verified f64 benchmark artifact");
    F64Fixture { bytes }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RankedRow {
    score: f32,
    pub(crate) row: u64,
}

impl PartialEq for RankedRow {
    fn eq(&self, other: &Self) -> bool {
        self.score.to_bits() == other.score.to_bits() && self.row == other.row
    }
}

impl Eq for RankedRow {}

impl PartialOrd for RankedRow {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RankedRow {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| other.row.cmp(&self.row))
    }
}

pub(crate) fn effective_row(matrix: EffectiveMatrixView<'_>, row: u64) -> Vec<f32> {
    matrix
        .f32_row(row)
        .expect("effective f32 row")
        .map(|value| value.expect("finite f32 coordinate"))
        .collect()
}

fn dot(matrix: EffectiveMatrixView<'_>, row: u64, query: &[f32]) -> f32 {
    matrix
        .f32_row(row)
        .expect("effective f32 row")
        .zip(query)
        .map(|(value, query)| value.expect("finite f32 coordinate") * query)
        .sum()
}

pub(crate) fn top_k(
    matrix: EffectiveMatrixView<'_>,
    query: &[f32],
    count: usize,
    excluded_row: u64,
) -> Vec<RankedRow> {
    let mut best = BinaryHeap::<Reverse<RankedRow>>::with_capacity(count + 1);
    for row in 0..matrix.matrix().row_count() {
        if row == excluded_row {
            continue;
        }
        let candidate = RankedRow {
            score: dot(matrix, row, query),
            row,
        };
        if best.len() < count {
            best.push(Reverse(candidate));
        } else if best.peek().is_some_and(|worst| candidate > worst.0) {
            best.pop();
            best.push(Reverse(candidate));
        }
    }
    let mut result = best.into_iter().map(|entry| entry.0).collect::<Vec<_>>();
    result.sort_unstable_by(|left, right| right.cmp(left));
    result
}

pub(crate) fn rerank(
    matrix: EffectiveMatrixView<'_>,
    query: &[f32],
    candidates: &[RankedRow],
    count: usize,
) -> Vec<RankedRow> {
    let mut reranked = candidates
        .iter()
        .map(|candidate| RankedRow {
            score: dot(matrix, candidate.row, query),
            row: candidate.row,
        })
        .collect::<Vec<_>>();
    reranked.sort_unstable_by(|left, right| right.cmp(left));
    reranked.truncate(count);
    reranked
}

fn catalog_subject_count() -> usize {
    std::env::var("PURREMB_CATALOG_SUBJECTS")
        .ok()
        .map_or(DEFAULT_CATALOG_CHUNKS, |value| {
            value
                .parse::<usize>()
                .expect("PURREMB_CATALOG_SUBJECTS must be a positive integer")
        })
        .max(1)
}

fn catalog_contract() -> EmbeddingFamilyContract {
    EmbeddingFamilyContract {
        model: artifact("catalog-model"),
        engine: artifact("engine"),
        tokenizer: artifact("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("rdf-dataset-projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: stage("catalog-chunking"),
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::SquaredEuclidean,
        dimensionality: DimensionalityPolicy::fixed(1, PrefixPostprocessing::None)
            .expect("catalog dimension"),
        extensions: Vec::new(),
    }
}

pub(crate) struct CatalogFixture {
    pub(crate) bytes: Vec<u8>,
    pub(crate) sample_target: TargetId,
    pub(crate) sample_document: TargetId,
    pub(crate) chunk_count: usize,
}

pub(crate) fn build_catalog_fixture() -> CatalogFixture {
    let chunk_count = catalog_subject_count();
    let document_count = CATALOG_DOCUMENTS.min(chunk_count);
    let (source, dataset_target) = source();
    let dataset_target_id = dataset_target.id;
    let contract = catalog_contract();
    let family = contract.derive().expect("catalog family");
    let corpus = CorpusTarget {
        manifest_digest: ContentDigest::of(b"million-chunk-catalog"),
        manifest_media_type: "application/vnd.example.corpus-manifest+cbor".into(),
        logical_id_digest: ContentDigest::of(b"catalog:benchmark"),
    }
    .into_target(true)
    .expect("catalog corpus");
    let mut targets = Vec::with_capacity(chunk_count + document_count + 2);
    let mut relations = Vec::with_capacity(chunk_count + document_count);
    targets.push(dataset_target);
    targets.push(corpus.clone());
    let mut documents = Vec::with_capacity(document_count);
    for document in 0..document_count {
        let chunks = (chunk_count + document_count - 1 - document) / document_count;
        let byte_length = usize_to_u64(chunks.saturating_sub(1))
            .saturating_mul(2)
            .saturating_add(1);
        let document_target = DocumentTarget {
            corpus_id: corpus.id,
            content_digest: ContentDigest::of(&usize_to_u64(document).to_le_bytes()),
            logical_id_digest: ContentDigest::of(
                &usize_to_u64(document).wrapping_add(1).to_le_bytes(),
            ),
            media_type: "text/plain;charset=utf-8".into(),
            byte_length,
            scalar_count: byte_length,
        }
        .into_target(true)
        .expect("catalog document");
        relations.push(TargetRelation::builtin(
            corpus.id,
            RelationKind::CorpusDocument,
            document_target.id,
        ));
        documents.push(document_target.id);
        targets.push(document_target);
    }
    let sample_document = documents[document_count / 2];
    let mut sample_target = None;
    for chunk in 0..chunk_count {
        let document = chunk % document_count;
        let local = chunk / document_count;
        let byte_start = usize_to_u64(local).saturating_mul(2);
        let chunk_target = TextChunkTarget {
            document_id: documents[document],
            chunking_id: family.chunking_id,
            content_digest: ContentDigest::of(&usize_to_u64(chunk).to_le_bytes()),
            byte_start,
            byte_end: byte_start + 1,
            scalar_start: byte_start,
            scalar_end: byte_start + 1,
        }
        .into_target(false)
        .expect("catalog chunk");
        if chunk == chunk_count / 2 {
            sample_target = Some(chunk_target.id);
        }
        relations.push(TargetRelation::builtin(
            documents[document],
            RelationKind::DocumentChunk,
            chunk_target.id,
        ));
        targets.push(chunk_target);
    }
    let target_set = TargetSet::new(vec![dataset_target_id]).expect("catalog target set");
    let metadata = CanonicalMetadataInput {
        source,
        family_contracts: vec![contract],
        targets,
        target_sets: vec![target_set.clone()],
        relations,
        token_spans: Vec::new(),
        external_bindings: Vec::new(),
        indexes: Vec::new(),
        extensions: Vec::new(),
    };
    let matrix = MatrixInput {
        family_id: family.id,
        target_set_id: target_set.id,
        stored_dimension: 1,
        rows: vec![MatrixRow::new(dataset_target_id, vec![1.0])],
        projections: vec![ProjectionSpec::derive(
            family.id,
            1,
            PrefixPostprocessing::None,
        )],
    };
    let mut builder = EmbeddingBuilder::from_typed_metadata(metadata);
    builder.add_f32_matrix(matrix);
    CatalogFixture {
        bytes: builder.build().expect("catalog artifact").bytes,
        sample_target: sample_target.expect("sample chunk"),
        sample_document,
        chunk_count,
    }
}

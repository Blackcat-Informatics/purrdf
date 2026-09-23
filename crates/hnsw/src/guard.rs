// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed adapter between `purrdf-hnsw` and PURREMB's derived-index lifecycle.
//!
//! PURREMB already carries every primitive this integration needs: a canonical
//! [`IndexGuardContract`] naming an implementation and its parameters, an
//! [`IndexPayloadStorage`] that is inline or detached, a [`DerivedIndex`] that folds
//! coordinates and payload bytes into a stable id, and an [`IndexGuardView`] that lends the
//! guard and payload back out of a verified artifact. **No parallel wire format is
//! invented here.** What this module adds is the part rdf-core deliberately leaves to the
//! producer: interpreting *this crate's* profile inside those opaque fields.
//!
//! It is a one-way adapter. `purrdf-hnsw` depends on `purrdf-core`; `purrdf-core` does not
//! name this crate, its profile, or its types, and this task changes no rdf-core source.
//!
//! # What a binding proves before a search can run
//!
//! [`load`] is the strict entry point. In order:
//!
//! 1. the guard declares one of this crate's implementations, with the evidence revision
//!    that implementation publishes, and this profile's parameter encoding and media type
//!    ([`validate_guard`]);
//! 2. the inline payload's length and SHA-256 match what the guard committed
//!    ([`verify_payload_commitment`]) — a substituted payload is rejected even when the
//!    container's own verification was never asked to run;
//! 3. the canonical image decodes over the matrix under the arithmetic the guard names
//!    ([`load`] for the exact index, [`load_reassociated`] for the reassociated one), its
//!    header records a code that implementation publishes, and its embedded parameters
//!    agree with the guard's parameter block.
//!
//! # Rebuildability, made checkable
//!
//! [`verify_rebuild`] answers the question an opaque payload otherwise cannot: *is this
//! payload the canonical image of building the given matrix under the given parameters?*
//! It recomputes the graph and compares. A `true` means the payload is a pure function of
//! the source data and the declared identity; a `false` means it is not — a stale or
//! tampered payload, or one built under different parameters.

use purrdf_core::{
    ContentDigest, DerivedIndex, EffectiveMatrixView, EmbeddingView, IndexBuildDeterminism,
    IndexCoordinates, IndexGuardContract, IndexGuardView, IndexPayloadStorage, IndexStorage,
    IndexUseRole, TargetSetId, TlvEntryRef, TlvWireType, VectorDtype, VectorSpaceId, canonical_tlv,
};

use purrdf_core::distance::{Arithmetic, Exact, Reassociated};

use crate::error::{HnswError, Result};
use crate::graph::VectorMatrix;
use crate::profile::Published;
use crate::{
    HnswIndex, IMPLEMENTATION_ID, IMPLEMENTATION_ID_REASSOCIATED, INDEX_MEDIA_TYPE, Params, profile,
};

// ---------------------------------------------------------------------------
// The guard contract
// ---------------------------------------------------------------------------

/// The PURREMB use role an index over `effective` occupies.
///
/// The role is **read off the artifact**, not chosen. PURREMB defines
/// [`IndexUseRole::CoarsePrefixRetrieval`] as "coarse retrieval over a shorter effective
/// prefix", and an index built over a projection whose effective dimension is shorter than
/// the matrix it projects is exactly that, whatever the profile would prefer to call itself.
/// Only an index over the whole stored width is [`IndexUseRole::Generic`].
///
/// Getting this from the data matters because the role is a claim a consumer acts on: a
/// planner that reads `Generic` may treat the offer as final, while `CoarsePrefixRetrieval`
/// says the answer was decided on a truncation of the vectors and invites a full-width
/// rerank. Declaring `Generic` over a prefix understates the loss and is a false statement
/// about the artifact, not a conservative one.
///
/// [`IndexUseRole::FullPrefixReranking`] is never emitted here: this profile retrieves, it
/// does not rerank, and claiming the role would be a claim about a query plan it does not
/// execute.
#[must_use]
pub fn use_role(effective: &EffectiveMatrixView<'_>) -> IndexUseRole {
    if effective.projection().effective_dimension() < effective.matrix().stored_dimension() {
        IndexUseRole::CoarsePrefixRetrieval
    } else {
        IndexUseRole::Generic
    }
}

/// The guard contract this crate emits for an exact index under `params` and `role`.
///
/// The certified-metadata binding is absent: this profile names no RDF metadata document,
/// and a default is exactly what the workspace forbids.
#[must_use]
pub fn guard_contract(params: Params, role: IndexUseRole) -> IndexGuardContract {
    contract(profile::implementation(), params, role)
}

/// The guard contract this crate emits for `index` under `role`: the implementation of its
/// arithmetic on the dispatch path its image records, and the same loss contract,
/// parameter encoding and payload media type under either arithmetic.
#[must_use]
pub fn guard_contract_for<A: Arithmetic>(
    index: &HnswIndex<A>,
    role: IndexUseRole,
) -> IndexGuardContract {
    contract(
        profile::implementation_for::<A>(index.arithmetic().path()),
        index.params(),
        role,
    )
}

/// The guard contract around `implementation`.
fn contract(
    implementation: purrdf_core::ArtifactIdentity,
    params: Params,
    role: IndexUseRole,
) -> IndexGuardContract {
    IndexGuardContract {
        implementation,
        parameter_encoding: profile::PARAMETER_ENCODING.to_owned(),
        parameters: profile::parameters(params),
        loss: profile::loss_contract(),
        use_role: role,
        payload_media_type: INDEX_MEDIA_TYPE.to_owned(),
        certified_metadata_binding: None,
    }
}

/// Fold a built index into a deterministic PURREMB [`DerivedIndex`].
///
/// `coordinates` must name the exact matrix this index was built over; rdf-core rejects a
/// derived index whose coordinates do not describe a matrix the artifact holds, so that
/// obligation is checked at the container boundary as well as here by construction. The
/// guard is [`guard_contract_for`] the index, so a reassociated index publishes its own
/// implementation and the evidence of the path it was built on.
///
/// # Errors
///
/// [`HnswError`] wrapping any [`purrdf_core::EmbeddingError`] — an empty payload, a
/// non-deterministic inline payload, or a guard whose contract is malformed.
pub fn derived_index<A: Arithmetic>(
    coordinates: IndexCoordinates,
    index: &HnswIndex<A>,
    role: IndexUseRole,
) -> Result<DerivedIndex> {
    let guard = guard_contract_for(index, role);
    Ok(DerivedIndex::new(
        coordinates,
        IndexPayloadStorage::Inline(index.canonical_image()),
        IndexBuildDeterminism::Deterministic,
        &guard,
    )?)
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

/// Whether `guard` names this crate's profile, without trusting anything inside it.
///
/// The test is deliberately narrow: parse the guard's canonical block, read the
/// implementation identity's stable identifier, and compare it with the identifiers this
/// profile publishes ([`IMPLEMENTATION_ID`] and [`IMPLEMENTATION_ID_REASSOCIATED`]). A guard that cannot be parsed
/// at all does not name this profile. Everything else — parameters, media type, evidence —
/// is checked strictly by [`validate_guard`] once a single candidate is selected, so a
/// malformed guard that *does* claim the identifier is a loud error rather than a silently
/// unselected one.
#[must_use]
pub fn names_hnsw(guard: &IndexGuardView<'_>) -> bool {
    guard_entries(guard)
        .ok()
        .and_then(|entries| find(&entries, 1))
        .and_then(|identity| identity_block_identifier(identity.value).ok())
        .is_some_and(|identifier| is_published_identifier(&identifier))
}

/// Whether `identifier` is one of the implementations this profile publishes.
fn is_published_identifier(identifier: &str) -> bool {
    profile::published()
        .iter()
        .any(|row| row.implementation == identifier)
}

/// Select the single HNSW guard from a verified artifact.
///
/// # Errors
///
/// [`HnswError::MissingIndexGuard`] when no guard names the profile, and
/// [`HnswError::AmbiguousIndexGuard`] when more than one does. Two HNSW guards cannot be
/// distinguished by a caller that names the profile alone, and picking one silently would
/// answer from an index the caller did not name.
pub fn select<'a>(view: &EmbeddingView<'a>) -> Result<IndexGuardView<'a>> {
    let mut selected = None;
    let mut count = 0_usize;
    for guard in view.index_guards() {
        if names_hnsw(&guard) {
            count += 1;
            if selected.is_none() {
                selected = Some(guard);
            }
        }
    }
    match count {
        0 => Err(HnswError::MissingIndexGuard {
            description: format!(
                "the artifact holds no derived index whose implementation is \
                 `{IMPLEMENTATION_ID}` or `{IMPLEMENTATION_ID_REASSOCIATED}`; register one \
                 with `guard::derived_index`"
            ),
        }),
        1 => Ok(selected.expect("count one implies a selected guard")),
        count => Err(HnswError::AmbiguousIndexGuard { count }),
    }
}

// ---------------------------------------------------------------------------
// Profile validation
// ---------------------------------------------------------------------------

/// Strictly validate a selected guard against the HNSW profile.
///
/// # The legal pairings
///
/// The implementation identifier, the evidence revision and the image codes a payload may
/// record are one row of [`profile`]'s published table, derived from the [`Exact`] and
/// [`Reassociated`] arithmetics' own constants. An identifier must carry a revision its
/// own row publishes: the exact implementation with a reassociated revision, or the
/// reassociated implementation with the exact revision, is a cross pairing and a profile
/// failure, never a guard that validates as whichever half a reader looked at.
///
/// # Errors
///
/// [`HnswError::GuardProfile`] if the implementation identifier is not one this profile
/// publishes, its media type differs, its evidence revision is not one that identifier
/// publishes, the parameter encoding names something else, the loss contract is not the
/// non-transforming approximate one, or the parameter block is not exactly four canonical
/// `u64` fields readable by [`Params::new`].
pub fn validate_guard(guard: &IndexGuardView<'_>) -> Result<Params> {
    validate_profile(guard).map(|(params, _)| params)
}

/// [`validate_guard`], also returning the published row the guard's identity is.
pub(crate) fn validate_profile(guard: &IndexGuardView<'_>) -> Result<(Params, Published)> {
    let entries = guard_entries(guard)?;
    let identity = required(&entries, 1, "the implementation identity")?;
    if identity.wire_type != TlvWireType::Block {
        return Err(profile_failure(
            "the implementation identity is not a block",
        ));
    }
    let (identifier, media_type, revision) = identity_block_fields(identity.value)?;
    let published = profile::published();
    if !published.iter().any(|row| row.implementation == identifier) {
        return Err(profile_failure(format!(
            "the implementation identifier is `{identifier}`, not `{IMPLEMENTATION_ID}` or \
             `{IMPLEMENTATION_ID_REASSOCIATED}`"
        )));
    }
    if media_type != profile::IMPLEMENTATION_MEDIA_TYPE {
        return Err(profile_failure(format!(
            "the implementation media type is `{media_type}`, not \
             `{}`",
            profile::IMPLEMENTATION_MEDIA_TYPE
        )));
    }
    let matching = published.into_iter().find(|row| {
        row.implementation == identifier && revision.as_deref() == Some(row.revision.as_bytes())
    });
    let Some(row) = matching else {
        return Err(foreign_revision(&identifier, revision.as_deref()));
    };

    let encoding = required_utf8(&entries, 2, "the parameter encoding")?;
    if encoding != profile::PARAMETER_ENCODING {
        return Err(profile_failure(format!(
            "the parameter encoding is `{encoding}`, not `{}`",
            profile::PARAMETER_ENCODING
        )));
    }

    let parameters = required(&entries, 3, "the parameter block")?;
    if parameters.wire_type != TlvWireType::Bytes {
        return Err(profile_failure("the parameter block is not a byte string"));
    }
    let params = profile::parse_parameters(parameters.value)?;

    validate_loss(required(&entries, 5, "the loss contract")?)?;

    let media_type = required_utf8(&entries, 7, "the payload media type")?;
    if media_type != INDEX_MEDIA_TYPE {
        return Err(profile_failure(format!(
            "the payload media type is `{media_type}`, not `{INDEX_MEDIA_TYPE}`"
        )));
    }

    let role = required(&entries, 6, "the index use role")?;
    if role.wire_type != TlvWireType::U32 {
        return Err(profile_failure("the index use role is not a u32"));
    }
    if role.value.len() != 4 {
        return Err(profile_failure("the index use role is not four bytes"));
    }
    let role = u32::from_le_bytes(role.value.try_into().expect("checked length"));
    if !matches!(role, 1..=3) {
        return Err(profile_failure(format!("unknown index use role {role}")));
    }

    Ok((params, row))
}

/// The refusal of an evidence revision `identifier` does not publish.
///
/// A revision that another published implementation carries is named as the cross pairing
/// it is; anything else is the profile's own approximation statement missing.
fn foreign_revision(identifier: &str, revision: Option<&[u8]>) -> HnswError {
    let other = profile::published()
        .into_iter()
        .find(|row| row.implementation != identifier && revision == Some(row.revision.as_bytes()));
    match other {
        Some(row) => profile_failure(format!(
            "the implementation `{identifier}` carries the evidence revision `{}` publishes \
             for the {} arithmetic; an implementation must publish its own arithmetic's \
             evidence, so the pairing is refused",
            row.implementation, row.arithmetic
        )),
        None => profile_failure(
            "the implementation evidence revision is not the profile's approximation \
             statement; an HNSW guard must publish what it does not promise",
        ),
    }
}

/// Validate the loss block: approximate, and no vector transform.
fn validate_loss(loss: TlvEntryRef<'_>) -> Result<()> {
    if loss.wire_type != TlvWireType::Block {
        return Err(profile_failure("the loss contract is not a block"));
    }
    let entries = canonical_tlv(loss.value)?;
    let entries: Vec<TlvEntryRef<'_>> = entries.collect();
    let approximate = required(&entries, 1, "the approximation flag")?;
    if approximate.value != [1] {
        return Err(profile_failure("the loss contract is not approximate"));
    }
    let transforms = required(&entries, 2, "the vector-transform flag")?;
    if transforms.value != [0] {
        return Err(profile_failure(
            "the loss contract claims an HNSW payload transforms vectors; this profile \
             stores no vectors, so the claim is false",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Coordinates
// ---------------------------------------------------------------------------

/// The exact coordinates that bind an index to one effective matrix.
///
/// This is the canonical `(source, family, space, matrix, projection, target set,
/// prefix)` tuple PURREMB commits. Reading it from the view rather than re-deriving it
/// means the coordinates cannot disagree with the artifact they will be stored in.
///
/// # Errors
///
/// [`HnswError::Embedding`] if the artifact declares no such space, target set, or
/// effective matrix.
pub fn coordinates(
    view: &EmbeddingView<'_>,
    target_set: TargetSetId,
    vector_space: VectorSpaceId,
) -> Result<IndexCoordinates> {
    let space = view
        .vector_space(vector_space)
        .ok_or_else(|| HnswError::Embedding {
            description: format!("the artifact declares no vector space {vector_space}"),
        })?;
    let set = view
        .target_set(target_set)
        .ok_or_else(|| HnswError::Embedding {
            description: format!("the artifact declares no target set {target_set}"),
        })?;
    let effective = view
        .effective_matrix(target_set, vector_space)?
        .ok_or_else(|| HnswError::Embedding {
            description: format!(
                "the artifact holds no matrix joining target set {target_set} to vector \
                 space {vector_space}"
            ),
        })?;
    Ok(IndexCoordinates {
        source_exact_digest: view.source().source_exact_digest(),
        family_id: space.family_id(),
        vector_space_id: space.id(),
        matrix_id: effective.matrix().id(),
        projection_id: effective.projection().id(),
        target_set_id: set.id(),
        prefix_dimension: effective.projection().effective_dimension(),
    })
}

/// Prove a guard's coordinates describe `effective`, `target_set` and `vector_space`.
///
/// rdf-core already refuses a structurally stale relationship when the artifact is parsed,
/// so through this workspace's encoder the failure below is unreachable. It is kept
/// because PURREMB is a wire format with third-party producers: a reader that assumed its
/// own writer had produced the bytes would be assuming exactly what a fail-closed borrowed
/// view exists to stop assuming.
///
/// # Errors
///
/// [`HnswError::GuardProfile`] naming the first coordinate that disagrees.
pub fn check_coordinates(
    guard: &IndexGuardView<'_>,
    target_set: TargetSetId,
    vector_space: VectorSpaceId,
    effective: &EffectiveMatrixView<'_>,
) -> Result<()> {
    let matrix = effective.matrix();
    let projection = effective.projection();
    let mismatch = |field: &str| {
        profile_failure(format!(
            "guard {} is stale or substituted: its {field} does not name the searched \
             effective matrix",
            guard.id()
        ))
    };
    if guard.target_set_id() != target_set {
        return Err(mismatch("target set"));
    }
    if guard.vector_space_id() != vector_space {
        return Err(mismatch("vector space"));
    }
    if guard.matrix_id() != matrix.id() {
        return Err(mismatch("matrix"));
    }
    if guard.projection_id() != projection.id() {
        return Err(mismatch("projection"));
    }
    if guard.family_id() != matrix.family_id() {
        return Err(mismatch("family"));
    }
    if guard.prefix_dimension() != projection.effective_dimension() {
        return Err(mismatch("prefix dimension"));
    }
    // The role is checked, not merely carried. A guard declaring a general-purpose index
    // over a truncated projection understates its own loss, and a consumer that reads the
    // role to decide whether a rerank is owed would act on the wrong answer.
    if declared_use_role(guard)? != use_role(effective) {
        return Err(mismatch("use role"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Payload and decode
// ---------------------------------------------------------------------------

/// The inline payload bytes a guard commits, or a typed refusal for detached storage.
///
/// # Errors
///
/// [`HnswError::PayloadUnavailable`] for a detached payload (the bytes live in an external
/// binding this function is not given) or an absent inline section.
pub fn payload_bytes<'a>(guard: &IndexGuardView<'a>) -> Result<&'a [u8]> {
    match guard.storage()? {
        IndexStorage::Inline => {
            guard
                .payload_bytes()
                .ok_or_else(|| HnswError::PayloadUnavailable {
                    description: format!(
                        "guard {} declares an inline payload but the artifact lends no bytes",
                        guard.id()
                    ),
                })
        }
        IndexStorage::Detached => Err(HnswError::PayloadUnavailable {
            description: format!(
                "guard {} stores its payload in an external binding; resolve that binding \
                 and verify it before searching",
                guard.id()
            ),
        }),
    }
}

/// Prove the inline payload matches the guard's committed length and SHA-256.
///
/// This is the tamper check a search cannot run without: the container's own verification
/// is a separate call a host may or may not have made, and a substituted payload is a
/// wrong index, not a slower one.
///
/// # Errors
///
/// [`HnswError::PayloadCommitment`] if either the length or the digest disagrees.
pub fn verify_payload_commitment(guard: &IndexGuardView<'_>, bytes: &[u8]) -> Result<()> {
    let length = u64::try_from(bytes.len()).map_err(|_| HnswError::PayloadCommitment {
        description: format!("guard {} payload length does not fit u64", guard.id()),
    })?;
    if length != guard.payload_length() {
        return Err(HnswError::PayloadCommitment {
            description: format!(
                "guard {} commits {} payload byte(s) but {} are present",
                guard.id(),
                guard.payload_length(),
                length
            ),
        });
    }
    let digest = ContentDigest::of(bytes);
    if digest != guard.payload_sha256() {
        return Err(HnswError::PayloadCommitment {
            description: format!(
                "guard {} commits payload SHA-256 {} but the bytes hash to {}",
                guard.id(),
                guard.payload_sha256(),
                digest
            ),
        });
    }
    Ok(())
}

/// Verify the guard profile, the payload commitment, and decode the exact index over
/// `matrix`.
///
/// The decoded graph must have one node per matrix row, and its embedded identity
/// (kernel and parameters) must agree with the guard's parameter block. Everything a
/// search could otherwise fail on mid-query is therefore proven here, once.
///
/// # Errors
///
/// [`HnswError::GuardProfile`] (a guard naming the reassociated implementation among
/// them: load it with [`load_reassociated`]), [`HnswError::PayloadUnavailable`],
/// [`HnswError::PayloadCommitment`], or any decoder error from [`HnswIndex::decode`].
pub fn load(guard: &IndexGuardView<'_>, matrix: VectorMatrix) -> Result<HnswIndex> {
    load_as(guard, matrix, HnswIndex::decode)
}

/// [`load`] for a guard naming the reassociated implementation, decoding the index with
/// [`HnswIndex::decode_reassociated`].
///
/// # Errors
///
/// As [`load`], with [`HnswError::GuardProfile`] for a guard naming the exact
/// implementation or one whose revision names another dispatch path than the payload
/// records, and [`HnswError::ArithmeticPathUnavailable`] for a payload recorded on a path
/// this process does not run.
pub fn load_reassociated(
    guard: &IndexGuardView<'_>,
    matrix: VectorMatrix,
) -> Result<HnswIndex<Reassociated>> {
    load_as(guard, matrix, HnswIndex::decode_reassociated)
}

/// [`load`] under arithmetic `A`, decoding with `decode`.
fn load_as<A: Arithmetic>(
    guard: &IndexGuardView<'_>,
    matrix: VectorMatrix,
    decode: fn(VectorMatrix, &[u8]) -> Result<HnswIndex<A>>,
) -> Result<HnswIndex<A>> {
    let (params, row) = validate_profile(guard)?;
    if row.arithmetic != A::ID {
        return Err(profile_failure(format!(
            "the guard names the `{}` implementation, whose distances are computed under the \
             {} arithmetic, and it is being loaded as an index computed under {}",
            row.implementation,
            row.arithmetic,
            A::ID
        )));
    }
    let bytes = payload_bytes(guard)?;
    verify_payload_commitment(guard, bytes)?;
    let index = decode(matrix, bytes)?;
    let recorded = index.arithmetic().image_code();
    if !row.codes.contains(&recorded) {
        return Err(profile_failure(format!(
            "the guard's evidence revision names the dispatch path of arithmetic code(s) \
             {:?}, and the payload records code {recorded} ({}); the guard and the payload \
             describe two different compilations",
            row.codes,
            profile::path_label(recorded)
        )));
    }
    if index.params() != params {
        return Err(profile_failure(
            "the guard's parameter block and the payload's embedded parameters disagree",
        ));
    }
    Ok(index)
}

/// Read an effective matrix out of an artifact as an owned [`VectorMatrix`].
///
/// The rows are in the artifact's canonical row order — the target set's ascending
/// `TargetId` order — which is precisely the stable row numbering the HNSW level formula
/// and the relation's row-to-term mapping rely on. Casting every stored scalar to `f64` is
/// exact for an `f32` and identity for an `f64`, so there is one arithmetic path.
///
/// # Errors
///
/// [`HnswError::Embedding`] for an unreadable scalar type or row, and
/// [`HnswError::ParameterValidation`] if the shape is unusable.
pub fn read_effective_matrix(effective: &EffectiveMatrixView<'_>) -> Result<VectorMatrix> {
    let dtype = effective.matrix().dtype()?;
    let row_count =
        usize::try_from(effective.matrix().row_count()).map_err(|_| HnswError::Embedding {
            description: "the matrix row count does not fit this platform's index range".to_owned(),
        })?;
    let dimension =
        usize::try_from(effective.projection().effective_dimension()).map_err(|_| {
            HnswError::Embedding {
                description: "the effective dimension does not fit this platform's index range"
                    .to_owned(),
            }
        })?;
    let expected = row_count.saturating_mul(dimension);

    // The matrix is kept at the width the artifact stores it. Widening a binary32 corpus to
    // binary64 here would be exact and would change no arithmetic -- and would cost twice the
    // resident memory for that privilege. At a million rows of 4,096 components that is
    // sixteen gigabytes spent on nothing, and on wasm32 it is the difference between a corpus
    // loading and being refused. Every distance is still computed in binary64, in the same
    // order, with the same separate roundings; the widening happens per component inside the
    // fold, where it is free.
    match dtype {
        VectorDtype::F32 => {
            let mut data: Vec<f32> = Vec::with_capacity(expected);
            for row in 0..row_count {
                let before = data.len();
                for value in effective.f32_row(row as u64)? {
                    data.push(value?);
                }
                check_row_width(row, data.len() - before, dimension)?;
            }
            VectorMatrix::from_f32(row_count, dimension, data)
        }
        VectorDtype::F64 => {
            let mut data: Vec<f64> = Vec::with_capacity(expected);
            for row in 0..row_count {
                let before = data.len();
                for value in effective.f64_row(row as u64)? {
                    data.push(value?);
                }
                check_row_width(row, data.len() - before, dimension)?;
            }
            VectorMatrix::new(row_count, dimension, data)
        }
    }
}

/// Refuse a decoded row whose component count disagrees with the declared dimension.
fn check_row_width(row: usize, decoded: usize, dimension: usize) -> Result<()> {
    if decoded == dimension {
        return Ok(());
    }
    Err(HnswError::ParameterValidation {
        description: format!(
            "row {row} decoded {decoded} component(s); the effective dimension is {dimension}"
        ),
    })
}

/// Answer whether `guard`'s payload is the canonical image of building `source_matrix`
/// under `params`.
///
/// This is the claim that makes an opaque payload rebuildable rather than opaque: `true`
/// means the bytes are a pure function of the source matrix and the declared identity.
/// `false` means the payload and the declaration disagree — a stale payload, a tampered
/// one, or one built under other parameters — and is returned rather than raised, because
/// the question is a yes/no about a commitment.
///
/// # Errors
///
/// [`HnswError`] from the rebuild itself (for example a kernel result that leaves the
/// finite range on the supplied matrix, or [`HnswError::FloatEnvironment`] for a thread
/// whose float environment the arithmetic refuses), and
/// [`HnswError::VersionMismatch`] / [`HnswError::ArithmeticMismatch`] for a payload of
/// another image version or arithmetic: a version-1 image is an index folded under a
/// different law, not a tampered one, and answering `false` would say otherwise.
/// [`HnswError::ArithmeticPathUnavailable`] for a reassociated payload recorded on a
/// dispatch path this process does not run, for the same reason. Any other payload that
/// cannot be read or decoded is `Ok(false)`, since it cannot be the rebuild either.
///
/// The arithmetic the rebuild runs is the one the guard's implementation identifier names:
/// the reassociated implementation is rebuilt under [`Reassociated`], and anything else
/// under [`Exact`], whose decoder refuses a header that names another law.
pub fn verify_rebuild(
    guard: &IndexGuardView<'_>,
    source_matrix: &VectorMatrix,
    params: &Params,
) -> Result<bool> {
    let Ok(bytes) = payload_bytes(guard) else {
        return Ok(false);
    };
    let committed = u64::try_from(bytes.len()).ok() == Some(guard.payload_length())
        && ContentDigest::of(bytes) == guard.payload_sha256();
    if !committed {
        return Ok(false);
    }
    // Decoded and rebuilt against a BORROW of the source vectors. Taking ownership meant
    // cloning the matrix to decode and cloning it again to rebuild, so this path used to
    // cost twice the matrix in transient memory to answer a yes/no question.
    let reassociated = guard_entries(guard)
        .ok()
        .and_then(|entries| find(&entries, 1))
        .and_then(|identity| identity_block_identifier(identity.value).ok())
        .is_some_and(|identifier| identifier == profile::implementation_id_for::<Reassociated>());
    let verdict = if reassociated {
        HnswIndex::verify_bytes_against_reassociated(source_matrix, bytes)?
    } else {
        HnswIndex::<Exact>::verify_bytes_against(source_matrix, bytes)?
    };
    match verdict {
        Some(declared) => Ok(declared == *params),
        None => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// TLV helpers
// ---------------------------------------------------------------------------

/// Parse a guard's canonical block into its entries.
fn guard_entries<'a>(guard: &IndexGuardView<'a>) -> Result<Vec<TlvEntryRef<'a>>> {
    Ok(canonical_tlv(guard.guard_bytes())?.collect())
}

/// One required entry by tag, with a profile-shaped error on absence.
fn required<'a>(entries: &[TlvEntryRef<'a>], tag: u16, what: &str) -> Result<TlvEntryRef<'a>> {
    find(entries, tag).ok_or_else(|| profile_failure(format!("{what} is missing")))
}

/// One required UTF-8 entry by tag.
fn required_utf8<'a>(entries: &[TlvEntryRef<'a>], tag: u16, what: &str) -> Result<&'a str> {
    let entry = required(entries, tag, what)?;
    if entry.wire_type != TlvWireType::Utf8 {
        return Err(profile_failure(format!("{what} is not UTF-8")));
    }
    core::str::from_utf8(entry.value)
        .map_err(|_| profile_failure(format!("{what} is not valid UTF-8")))
}

/// The use role a guard declares, read from its canonical block.
///
/// Tag 6, a little-endian `u32` over PURREMB's role codes. An unrecognised code is refused
/// rather than mapped to a default: a role this profile does not understand is not a role it
/// may assume is harmless.
fn declared_use_role(guard: &IndexGuardView<'_>) -> Result<IndexUseRole> {
    let entries = guard_entries(guard)?;
    let entry = required(&entries, 6, "the use role")?;
    if entry.wire_type != TlvWireType::U32 {
        return Err(profile_failure("the use role is not a u32"));
    }
    let bytes: [u8; 4] = entry
        .value
        .try_into()
        .map_err(|_| profile_failure("the use role is not four bytes"))?;
    match u32::from_le_bytes(bytes) {
        1 => Ok(IndexUseRole::Generic),
        2 => Ok(IndexUseRole::CoarsePrefixRetrieval),
        3 => Ok(IndexUseRole::FullPrefixReranking),
        other => Err(profile_failure(format!(
            "the use role is {other}, which PURREMB does not define"
        ))),
    }
}

/// One entry by tag, if present.
fn find<'a>(entries: &[TlvEntryRef<'a>], tag: u16) -> Option<TlvEntryRef<'a>> {
    entries.iter().find(|entry| entry.tag == tag).copied()
}

/// The stable identifier of an implementation identity block.
fn identity_block_identifier(block: &[u8]) -> Result<String> {
    let entries: Vec<TlvEntryRef<'_>> = canonical_tlv(block)?.collect();
    let identifier = required(&entries, 1, "the implementation identifier")?;
    if identifier.wire_type != TlvWireType::Utf8 {
        return Err(profile_failure(
            "the implementation identifier is not UTF-8",
        ));
    }
    core::str::from_utf8(identifier.value)
        .map(str::to_owned)
        .map_err(|_| profile_failure("the implementation identifier is not valid UTF-8"))
}

/// The identifier, media type, and optional revision of an implementation identity block.
fn identity_block_fields(block: &[u8]) -> Result<(String, String, Option<Vec<u8>>)> {
    let entries: Vec<TlvEntryRef<'_>> = canonical_tlv(block)?.collect();
    let identifier = required_utf8(&entries, 1, "the implementation identifier")?.to_owned();
    let media_type = required_utf8(&entries, 2, "the implementation media type")?.to_owned();
    let revision = match find(&entries, 4) {
        Some(entry) if entry.wire_type == TlvWireType::Bytes => Some(entry.value.to_vec()),
        Some(_) => {
            return Err(profile_failure(
                "the implementation revision is not a byte string",
            ));
        }
        None => None,
    };
    Ok((identifier, media_type, revision))
}

/// A guard-profile failure.
fn profile_failure(description: impl Into<String>) -> HnswError {
    HnswError::GuardProfile {
        description: description.into(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use purrdf_core::ArtifactIdentity;

    use super::*;

    #[test]
    fn the_profile_identifier_is_recognised_by_its_name() {
        // A guard-shaped TLV block is not needed here; the identity helper is exercised
        // through the profile declaration directly.
        let identity = profile::implementation();
        assert_eq!(identity.identifier, IMPLEMENTATION_ID);
        assert_eq!(
            identity_block_identifier(&identity_block_bytes(&identity)).expect("parses"),
            IMPLEMENTATION_ID
        );
    }

    /// Re-encode an artifact identity's fields the way rdf-core does, for the helper test.
    fn identity_block_bytes(identity: &ArtifactIdentity) -> Vec<u8> {
        let mut out = Vec::new();
        put(
            &mut out,
            1,
            TlvWireType::Utf8,
            identity.identifier.as_bytes(),
        );
        put(
            &mut out,
            2,
            TlvWireType::Utf8,
            identity.media_type.as_bytes(),
        );
        put(
            &mut out,
            3,
            TlvWireType::Digest32,
            identity.digest.as_bytes(),
        );
        if let Some(revision) = &identity.revision {
            put(&mut out, 4, TlvWireType::Bytes, revision);
        }
        put(&mut out, 5, TlvWireType::U32, &[1, 0, 0, 0]);
        out
    }

    fn put(out: &mut Vec<u8>, tag: u16, wire: TlvWireType, value: &[u8]) {
        out.extend_from_slice(&tag.to_le_bytes());
        out.push(wire as u8);
        out.push(1);
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value);
        let aligned = (out.len() + 7) & !7;
        out.resize(aligned, 0);
    }
}

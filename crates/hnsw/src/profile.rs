// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The HNSW **derived-index profile**: the one place the crate's PURREMB identity is
//! spelled out.
//!
//! PURREMB stores opaque index payloads and refuses to interpret them (see
//! `purrdf_core`'s `IndexGuardView`). Everything a reader needs to decide *which* opaque
//! payload is this crate's — and to reject one that claims to be but is not — is therefore
//! a contract this crate must state for itself. That contract is this module:
//!
//! * **Identity.** The [`IndexGuardContract::implementation`](purrdf_core::IndexGuardContract)
//!   artifact identity, whose stable identifier is [`IMPLEMENTATION_ID`],
//!   whose revision bytes carry the approximation evidence string, and whose digest binds
//!   the profile declaration.
//! * **Parameter encoding.** [`PARAMETER_ENCODING`], naming the canonical TLV block that
//!   [`parameters`] produces and [`parse_parameters`] reads.
//! * **Payload media type.** [`INDEX_MEDIA_TYPE`](crate::INDEX_MEDIA_TYPE).
//! * **Loss contract.** [`loss_contract`] — approximate, and no vector transform. The
//!   approximation is inherent: PURREMB's index loss contract always carries the
//!   `approximate = true` field (tag 1), and this profile sets `transforms_vectors` false
//!   because an HNSW graph stores no vectors at all.
//! * **Canonical payload layout.** The byte image [`crate::HnswIndex::canonical_image`]
//!   emits, documented below.
//!
//! # The approximation contract, stated honestly
//!
//! [`LOSS_EVIDENCE`] is the one sentence this profile publishes about its own quality:
//! the sentence quoted on `LOSS_EVIDENCE` below.
//!
//! Both of its halves are load-bearing and the second is the one that is easy to lose. An
//! earlier draft said only that recall was MEASURED, which is true and is the favourable
//! half. A declared approximation is required to name "the
//! exact strength of the evidence", and a sentence reporting that evidence exists without
//! reporting its strength is an overclaim by omission -- the worse kind here, because this
//! sentence travels IN THE ARTIFACT as the identity's revision bytes and is covered by the
//! guard digest. A third party binding this guard would bind the favourable half and have no
//! way to see what the repository says elsewhere.
//!
//! What changed and what did not: the original "recall unmeasured on realistic corpora" was
//! written against uniform-random fixtures, the one input class whose distances concentrate
//! so hard that no index can score well on it, so that figure described the generator rather
//! than the index. Fixing the corpora earned the first clause. It did not earn silence about
//! scale: the measurement reaches 50,000 rows, and the regime this index exists for is one
//! order of magnitude beyond it. It is not a
//! disclaimer appended to documentation — it is carried **in the artifact**, as the
//! implementation identity's revision bytes, so the guard digest covers it and
//! [`crate::guard::validate_guard`] refuses a guard whose revision says anything else. A
//! host cannot bind an HNSW index without also binding the statement of what it does not
//! promise.
//!
//! # The parameter TLV schema
//!
//! Four `u64` fields in ascending tag order, little-endian, each padded to an 8-byte
//! boundary — the canonical form PURREMB's own TLV codec uses:
//!
//! | tag | field | wire type |
//! |----:|-------|-----------|
//! | 1 | `M` | `u64` |
//! | 2 | `M0` | `u64` |
//! | 3 | `ef_construction` | `u64` |
//! | 4 | `ef_search` | `u64` |
//!
//! The block is the guard's `parameters` field, and rdf-core commits its SHA-256 as the
//! guard's self-digest, so a changed parameter is a guard that fails verification rather
//! than an index that silently searches under a different identity.
//!
//! # The canonical payload layout
//!
//! The image [`parameters`] does not touch is the graph itself, emitted by
//! [`crate::HnswIndex::canonical_image`] and decoded by
//! [`crate::HnswIndex::decode`]. It is a contiguous, little-endian, 8-byte-aligned image
//! with no pointers and no host-endian fields, so it is the same bytes on every target:
//!
//! ```text
//! header:
//!   magic      [u8; 8]   IMAGE_MAGIC
//!   version    u32       IMAGE_VERSION
//!   kernel     u32       0 cosine, 1 negative-dot, 2 squared-euclidean
//!   M          u64
//!   M0         u64
//!   ef_c       u64
//!   ef_search  u64
//!   node_count u64
//!   max_level  u32
//!   arithmetic u32       the distance arithmetic's image code: 1 = Exact
//!                        (binary64-lane16-tree-v1); 2..=8 = Reassociated
//!                        (binary64-reassociated-v1) on the dispatch path the
//!                        build ran; 0 is refused
//!   shape      u64       Reassociated codes only: the BuildShape of the build
//!                        that computed the distances (purrdf_core::distance's
//!                        versioned layout); absent from an Exact image
//!   entry      u64       row, or u64::MAX for an empty graph
//! node records, in ascending row order:
//!   row        u64       must equal the record's position
//!   level      u32
//!   reserved   u32       0
//!   layer records, ascending layer 0..=level:
//!     layer      u32     must equal the record's position
//!     reserved   u32     0
//!     count      u64
//!     neighbours, strictly ascending by neighbour row:
//!       row      u64
//!       distance u64     f64 bits
//! ```
//!
//! The in-memory graph orders a node's neighbours by `(distance, row)`; the image orders
//! them by neighbour row, because the row set is the identity and the distances are
//! derived. Decoding re-sorts by rank, so the two are views of one graph.
//!
//! Version 2 of the image is the first whose distances are folded by the sixteen-lane
//! exact arithmetic, and its `arithmetic` field (the `u32` version 1 reserved as zero)
//! records that. A version-1 image is refused with
//! [`HnswError::VersionMismatch`], and an image whose
//! field names another arithmetic with
//! [`HnswError::ArithmeticMismatch`].
//!
//! # A reassociated image records the build as well as the path
//!
//! What a reassociated dispatch path compiles to depends on the consumer build's target
//! features as well as on the path: the baseline `x86_64` path contracts to fused
//! multiply-add in a build compiled with `fma`, and the NEON path becomes SVE under a
//! Neoverse target. So two builds recording the same path can compute different bits, and
//! the path alone does not pin the code that ran. A reassociated image therefore records,
//! after its code, the [`BuildShape`] of the build that computed it -- its target
//! architecture and the target features that decide the reassociated body's code -- and a
//! build of another shape refuses it with [`HnswError::ArithmeticBuildMismatch`]. An exact
//! image records no shape: its bits are the same in every build, and its bytes are the
//! ones version 2 always had.
//!
//! Equal path and shape are necessary and not sufficient. CPU tuning (`-C target-cpu`)
//! and the compiler version are not visible to the source, so a reassociated image is
//! reproducible only by the compiled artifact that built it; a rebuild that diverges under
//! a matching path and shape is refused with [`HnswError::ArithmeticRebuildDiverged`], not
//! answered `false`.
//!
//! # The arithmetic is part of the profile
//!
//! [`profile_declaration`] folds the arithmetic's identifier, so the implementation
//! identity's digest binds the law every recorded distance was computed under. The
//! [`loss_contract`] does not change with it: an arithmetic decides how distances are
//! rounded, not what vectors are stored, and PURREMB requires `loss_encoding: None` for
//! a non-transforming index. The choice is recorded where it does change something --
//! the image header's arithmetic field, the implementation identifier and the evidence
//! revision -- rather than in a loss contract that would then claim a transform no
//! vector underwent.
//!
//! # Two published implementations, one per arithmetic
//!
//! The profile publishes one implementation per [`Arithmetic`], and each is derived from
//! that arithmetic's constants rather than written out beside it:
//!
//! | arithmetic | identifier | evidence revision | image codes |
//! |---|---|---|---|
//! | [`Exact`] | [`IMPLEMENTATION_ID`] | [`LOSS_EVIDENCE`] | `Exact::IMAGE_CODES` (`1`) |
//! | [`Reassociated`] | [`IMPLEMENTATION_ID_REASSOCIATED`] | [`loss_evidence_reassociated`] of the path | that path's one code (`2`..=`8`) |
//!
//! The exact arithmetic returns the same bits on every path, so it has one revision and
//! one code. The reassociated arithmetic's bits depend on the dispatch path, so its
//! evidence names the path and each path is its own revision, bound to the one code an
//! image built on that path records. [`crate::guard::validate_guard`] accepts exactly
//! these rows; an identifier carrying another row's revision, or a payload recording a
//! code its guard's revision does not name, is a profile failure.

use purrdf_core::distance::{Arithmetic, BuildShape, Exact, Path, Reassociated};
use purrdf_core::{ArtifactIdentity, ArtifactIdentityKind, ContentDigest, IndexLossContract};

use crate::error::{HnswError, Result};
use crate::params::Params;

/// The stable identifier of the HNSW derived-index implementation.
///
/// The same string as [`crate::IMPLEMENTATION_ID`]; re-exported through this module so a
/// consumer that reads the profile does not have to hop between modules.
pub use crate::IMPLEMENTATION_ID;

/// The stable identifier of the HNSW implementation whose distances are computed under
/// the [`Reassociated`] arithmetic.
///
/// The same string as [`crate::IMPLEMENTATION_ID_REASSOCIATED`], re-exported beside
/// [`IMPLEMENTATION_ID`].
pub use crate::IMPLEMENTATION_ID_REASSOCIATED;

/// The stable identifier of the canonical parameter encoding.
pub const PARAMETER_ENCODING: &str =
    "application/vnd.blackcatinformatics.purrdf.hnsw.parameters+tlv-v1";

/// The media type of the implementation identity itself.
pub const IMPLEMENTATION_MEDIA_TYPE: &str =
    "application/vnd.blackcatinformatics.purrdf.hnsw.profile-v1";

/// The approximation evidence string, carried as the implementation identity's revision.
pub const LOSS_EVIDENCE: &str = "approximate: recall measured against the exact oracle on \
     synthetic corpora up to 50,000 rows, and UNMEASURED at the 10^6 scale this index exists \
     for; an offer of candidates is never a proof of absence";

/// The sentence a reassociated evidence revision closes with.
///
/// Its own sentence, after the arithmetic's evidence, because it states a different
/// fact: not how the numbers may differ, but who can reproduce the image they built.
const REASSOCIATED_REPRODUCIBILITY: &str = "Its canonical image is reproducible only by the \
     compiled build that made it, running the same dispatch path: the image records that \
     build's target architecture and features, and CPU tuning and the compiler version, which \
     it cannot record, may change its bits too.";

/// The approximation evidence of an index whose distances are computed under the
/// [`Reassociated`] arithmetic along `path`: [`LOSS_EVIDENCE`], then the arithmetic's
/// own evidence for the path, then the sentence saying that only the compiled build that
/// made the image reproduces it.
///
/// It is carried as that implementation's revision, so a guard over a reassociated index
/// publishes, in the artifact, both what the graph does not promise and what its last
/// bits do not.
#[must_use]
pub fn loss_evidence_reassociated(path: Path) -> String {
    loss_evidence_for::<Reassociated>(path)
}

/// The approximation evidence of an index computed under arithmetic `A` along `path`.
///
/// [`LOSS_EVIDENCE`] for an arithmetic whose bits are the same on every path (it has no
/// evidence of its own); otherwise [`LOSS_EVIDENCE`], `"; "`, the arithmetic's evidence
/// along `path`, and the sentence saying that the canonical image is reproducible only by
/// the compiled build that made it, on that path.
#[must_use]
pub fn loss_evidence_for<A: Arithmetic>(path: Path) -> String {
    A::evidence(path).map_or_else(
        || LOSS_EVIDENCE.to_owned(),
        |evidence| format!("{LOSS_EVIDENCE}; {evidence}. {REASSOCIATED_REPRODUCIBILITY}"),
    )
}

/// The implementation identifier of the index whose distances arithmetic `A` computes.
///
/// # Panics
///
/// Never: `Arithmetic` is sealed, and every implementation of it is a row of the table
/// this reads.
#[must_use]
pub fn implementation_id_for<A: Arithmetic>() -> &'static str {
    IMPLEMENTATIONS
        .iter()
        .find(|(law, _)| *law == A::ID)
        .map_or_else(
            || unreachable!("{} is an arithmetic this profile publishes", A::ID),
            |(_, identifier)| *identifier,
        )
}

/// The implementation identifier each arithmetic's index publishes, by law.
const IMPLEMENTATIONS: [(&str, &str); 2] = [
    (Exact::ID, IMPLEMENTATION_ID),
    (Reassociated::ID, IMPLEMENTATION_ID_REASSOCIATED),
];

/// Every dispatch path an arithmetic resolves to, so the published rows can be derived
/// from each arithmetic's own `image_code`.
///
/// A path added to `purrdf_core::distance::Path` and missing here would leave a code
/// with no published row; `every_image_code_has_a_published_row` compares the codes
/// gathered here with each arithmetic's `IMAGE_CODES`.
pub(crate) const PATHS: [Path; 8] = [
    Path::Portable,
    Path::Avx2,
    Path::Sse2,
    Path::Avx2Fma,
    Path::Avx512f,
    Path::Neon,
    Path::WasmSimd128,
    Path::WasmScalar,
];

/// One implementation this profile publishes: its identifier, the law it computes
/// under, the evidence revision it binds, and the image codes a payload under it may
/// record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Published {
    /// The implementation identifier.
    pub implementation: &'static str,
    /// The arithmetic's stable identifier.
    pub arithmetic: &'static str,
    /// The evidence revision the identity carries.
    pub revision: String,
    /// The image codes a payload under this row records, ascending.
    pub codes: Vec<u32>,
}

/// Every implementation this profile publishes, derived from [`Exact`] and
/// [`Reassociated`]: one row per distinct evidence revision of each.
pub(crate) fn published() -> Vec<Published> {
    let mut rows = Vec::new();
    publish::<Exact>(&mut rows);
    publish::<Reassociated>(&mut rows);
    rows
}

/// Append arithmetic `A`'s rows: its paths grouped by the revision each publishes.
fn publish<A: Arithmetic>(rows: &mut Vec<Published>) {
    for path in PATHS {
        let Some(code) = A::image_code(path) else {
            continue;
        };
        let revision = loss_evidence_for::<A>(path);
        match rows
            .iter_mut()
            .find(|row| row.arithmetic == A::ID && row.revision == revision)
        {
            Some(row) => {
                if !row.codes.contains(&code) {
                    row.codes.push(code);
                    row.codes.sort_unstable();
                }
            }
            None => rows.push(Published {
                implementation: implementation_id_for::<A>(),
                arithmetic: A::ID,
                revision,
                codes: vec![code],
            }),
        }
    }
}

/// A human label for an image arithmetic code: the law, and for a code that names a
/// dispatch path, the path.
pub(crate) fn path_label(code: u32) -> String {
    if Exact::IMAGE_CODES.contains(&code) {
        return Exact::ID.to_owned();
    }
    PATHS
        .iter()
        .find(|path| Reassociated::image_code(**path) == Some(code))
        .map_or_else(
            || "a code no arithmetic of this build defines".to_owned(),
            |path| format!("{} along {path}", Reassociated::ID),
        )
}

/// The parameter block's tag for `M`.
pub const PARAM_M: u16 = 1;
/// The parameter block's tag for `M0`.
pub const PARAM_M0: u16 = 2;
/// The parameter block's tag for `ef_construction`.
pub const PARAM_EF_CONSTRUCTION: u16 = 3;
/// The parameter block's tag for `ef_search`.
pub const PARAM_EF_SEARCH: u16 = 4;

/// The canonical image's magic marker, exposed for profile documentation and tooling.
pub const PAYLOAD_MAGIC: [u8; 8] = crate::graph::IMAGE_MAGIC;

/// The canonical image's format version.
pub const PAYLOAD_VERSION: u32 = crate::graph::IMAGE_VERSION;

/// The TLV value wire type for a little-endian `u64`.
const WIRE_U64: u8 = 4;
/// The critical-field flag bit.
const FLAG_CRITICAL: u8 = 1;

/// The implementation identity this profile binds into every guard over an exact index.
///
/// The digest is a domain-separated SHA-256 over the profile declaration, and the revision
/// bytes are [`LOSS_EVIDENCE`] — so the guard digest commits the approximation statement
/// and not merely the algorithm name. [`implementation_for`] is the same identity for
/// either arithmetic.
///
/// # Panics
///
/// Panics only if the static profile declaration is malformed, which the compile-time
/// constants rule out; callers cannot reach a panic.
#[must_use]
pub fn implementation() -> ArtifactIdentity {
    implementation_for::<Exact>(Path::Portable)
}

/// The implementation identity of an index computed under arithmetic `A` along `path`:
/// [`implementation_id_for`], the digest of [`profile_declaration_for`], and
/// [`loss_evidence_for`] as the revision.
///
/// `path` decides nothing for an arithmetic whose bits are the same on every path.
///
/// # Panics
///
/// Panics only if the profile declaration is malformed, which the compile-time constants
/// rule out; callers cannot reach a panic.
#[must_use]
pub fn implementation_for<A: Arithmetic>(path: Path) -> ArtifactIdentity {
    ArtifactIdentity::new(
        implementation_id_for::<A>(),
        IMPLEMENTATION_MEDIA_TYPE,
        ContentDigest::of(profile_declaration_for::<A>(path).as_bytes()),
        Some(loss_evidence_for::<A>(path).into_bytes()),
        ArtifactIdentityKind::Single,
    )
    .expect("the HNSW profile declaration is static and valid")
}

/// The exact bytes whose SHA-256 is the exact index's implementation identity digest.
///
/// A stable, human-readable declaration rather than a serialized struct: it has no host
/// layout and no trailing version-dependent representation, so the digest is a function of
/// the profile's *meaning*. It names the distance arithmetic, so the digest binds the law
/// the recorded distances were folded under as well as the algorithm.
#[must_use]
pub fn profile_declaration() -> String {
    profile_declaration_for::<Exact>(Path::Portable)
}

/// The profile declaration of an index computed under arithmetic `A` along `path`.
///
/// Folds `A::ID`, the implementation identifier and the evidence revision, so two
/// arithmetics -- and, for a reassociated index, two dispatch paths -- declare two
/// different profiles with two different digests. For an arithmetic whose bits depend on
/// the build it also folds this build's [`BuildShape`], as a final
/// `build-shape=<bits>` line, so the identity digest binds the build that computed the
/// distances as the image header does; an exact declaration has no such line.
#[must_use]
pub fn profile_declaration_for<A: Arithmetic>(path: Path) -> String {
    let declaration = format!(
        "{}\n{PARAMETER_ENCODING}\n{}\n{}\narithmetic={}\n{}",
        implementation_id_for::<A>(),
        crate::INDEX_MEDIA_TYPE,
        "approximate=true;transforms_vectors=false",
        A::ID,
        loss_evidence_for::<A>(path)
    );
    match A::build_shape() {
        Some(shape) => format!("{declaration}\n{}", build_shape_line(shape)),
        None => declaration,
    }
}

/// The declaration line that binds a build shape: `build-shape=` and its bits as sixteen
/// lowercase hexadecimal digits.
fn build_shape_line(shape: BuildShape) -> String {
    format!("build-shape={:016x}", shape.bits())
}

/// The loss contract every HNSW guard carries, under either arithmetic.
///
/// Approximate (PURREMB's loss contract writes the `approximate = true` field itself) and
/// non-transforming: an HNSW graph stores no vectors, so no quantization contract applies
/// and `loss_encoding`/`loss_parameters` are absent, exactly as rdf-core requires for
/// `transforms_vectors == false`. The arithmetic is not a transform: it decides how a
/// distance is rounded and changes no stored vector, so a reassociated index carries the
/// same contract and records its arithmetic in the image field and the implementation
/// identity instead.
#[must_use]
pub const fn loss_contract() -> IndexLossContract {
    IndexLossContract {
        transforms_vectors: false,
        loss_encoding: None,
        loss_parameters: None,
    }
}

/// Encode `params` as the canonical parameter TLV block.
#[must_use]
pub fn parameters(params: Params) -> Vec<u8> {
    let mut out = Vec::new();
    put_u64(&mut out, PARAM_M, params.m() as u64);
    put_u64(&mut out, PARAM_M0, params.m0() as u64);
    put_u64(
        &mut out,
        PARAM_EF_CONSTRUCTION,
        params.ef_construction() as u64,
    );
    put_u64(&mut out, PARAM_EF_SEARCH, params.ef_search() as u64);
    out
}

/// Decode and validate a canonical parameter TLV block.
///
/// # Errors
///
/// [`HnswError::GuardProfile`] for a missing, out-of-order, over-long or wrong-wire-type
/// field, a non-zero padding byte, or a value that [`Params::new`] refuses (including a
/// negative parameter smuggled in as a two's-complement `u64`).
pub fn parse_parameters(bytes: &[u8]) -> Result<Params> {
    let mut reader = Reader::new(bytes);
    let mut m = None;
    let mut m0 = None;
    let mut ef_construction = None;
    let mut ef_search = None;
    let mut previous_tag = 0_u16;
    while !reader.is_empty() {
        let (tag, wire, value) = reader.entry()?;
        if tag <= previous_tag {
            return Err(profile_error(format!(
                "parameter tags must be strictly ascending, got {tag} after {previous_tag}"
            )));
        }
        previous_tag = tag;
        if wire != WIRE_U64 {
            return Err(profile_error(format!(
                "parameter {tag} has wire type {wire}, not a u64"
            )));
        }
        let value = u64::from_le_bytes(
            value
                .try_into()
                .map_err(|_| profile_error(format!("parameter {tag} is not eight bytes")))?,
        );
        match tag {
            PARAM_M => m = Some(value),
            PARAM_M0 => m0 = Some(value),
            PARAM_EF_CONSTRUCTION => ef_construction = Some(value),
            PARAM_EF_SEARCH => ef_search = Some(value),
            other => {
                return Err(profile_error(format!(
                    "unknown critical parameter tag {other}"
                )));
            }
        }
    }
    let missing = |field: &str| profile_error(format!("the parameter block is missing {field}"));
    let m = m.ok_or_else(|| missing("M"))?;
    let m0 = m0.ok_or_else(|| missing("M0"))?;
    let ef_construction = ef_construction.ok_or_else(|| missing("ef_construction"))?;
    let ef_search = ef_search.ok_or_else(|| missing("ef_search"))?;
    let widen = |name: &str, value: u64| {
        usize::try_from(value)
            .map_err(|_| profile_error(format!("parameter {name} = {value} does not fit usize")))
    };
    Params::new(
        widen("M", m)?,
        widen("M0", m0)?,
        widen("ef_construction", ef_construction)?,
        widen("ef_search", ef_search)?,
    )
}

/// A guard-profile failure, so every rejection reads as this profile's own.
fn profile_error(description: impl Into<String>) -> HnswError {
    HnswError::GuardProfile {
        description: description.into(),
    }
}

/// Append one canonical TLV entry (critical, padded to an 8-byte boundary).
fn put_u64(out: &mut Vec<u8>, tag: u16, value: u64) {
    put_entry(out, tag, WIRE_U64, &value.to_le_bytes());
}

/// Append one canonical TLV entry with the given wire type and value.
fn put_entry(out: &mut Vec<u8>, tag: u16, wire: u8, value: &[u8]) {
    out.extend_from_slice(&tag.to_le_bytes());
    out.push(wire);
    out.push(FLAG_CRITICAL);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
    let aligned = (out.len() + 7) & !7;
    out.resize(aligned, 0);
}

/// A bounds-checked reader over a canonical TLV block.
///
/// Deliberately rejects the same shapes rdf-core's codec does — non-ascending tags, a
/// non-critical-flag bit, wrong wire types, non-zero padding, and trailing bytes — so a
/// parameter block this crate accepts is one a canonical reader would too.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    const fn is_empty(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// One `(tag, wire, value)` triple, advancing past its padding.
    fn entry(&mut self) -> Result<(u16, u8, &'a [u8])> {
        let header_end = self.at + 8;
        let header = self
            .bytes
            .get(self.at..header_end)
            .ok_or_else(|| profile_error("a parameter entry is truncated"))?;
        let tag = u16::from_le_bytes([header[0], header[1]]);
        let wire = header[2];
        if header[3] & !FLAG_CRITICAL != 0 {
            return Err(profile_error("a parameter entry sets a reserved flag"));
        }
        if header[3] & FLAG_CRITICAL == 0 {
            return Err(profile_error("a parameter entry is not critical"));
        }
        let length = u32::from_le_bytes(header[4..8].try_into().expect("fixed slice"));
        let length = usize::try_from(length)
            .map_err(|_| profile_error("a parameter entry length does not fit usize"))?;
        let value_end = header_end
            .checked_add(length)
            .ok_or_else(|| profile_error("a parameter entry length overflows"))?;
        let value = self
            .bytes
            .get(header_end..value_end)
            .ok_or_else(|| profile_error("a parameter entry value is truncated"))?;
        let padded = (value_end + 7) & !7;
        let padding = self
            .bytes
            .get(value_end..padded)
            .ok_or_else(|| profile_error("a parameter entry padding is truncated"))?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(profile_error("a parameter entry has non-zero padding"));
        }
        self.at = padded;
        Ok((tag, wire, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_round_trip_through_the_canonical_block() {
        let params = Params::new(16, 32, 200, 64).expect("valid");
        let bytes = parameters(params);
        assert_eq!(bytes.len() % 8, 0, "the block is 8-byte aligned");
        assert_eq!(parse_parameters(&bytes).expect("round trips"), params);
    }

    #[test]
    fn a_non_ascending_tag_is_refused() {
        let mut bytes = Vec::new();
        // M0 before M.
        put_u64(&mut bytes, PARAM_M0, 32);
        put_u64(&mut bytes, PARAM_M, 16);
        assert!(matches!(
            parse_parameters(&bytes),
            Err(HnswError::GuardProfile { .. })
        ));
    }

    #[test]
    fn a_missing_parameter_is_refused() {
        let mut bytes = Vec::new();
        put_u64(&mut bytes, PARAM_M, 16);
        assert!(parse_parameters(&bytes).is_err());
    }

    #[test]
    fn non_zero_padding_is_refused() {
        // A four-`u64` block is already 8-byte aligned, so the padding case is exercised
        // on the reader directly: one aligned entry, then a one-byte value whose seven
        // padding bytes are non-zero.
        let mut block = Vec::new();
        put_u64(&mut block, PARAM_M, 16);
        block.extend_from_slice(&PARAM_M0.to_le_bytes());
        block.push(2); // the UTF-8 wire type, whose value here is one byte
        block.push(FLAG_CRITICAL);
        block.extend_from_slice(&1u32.to_le_bytes());
        block.push(b'x');
        block.extend_from_slice(&[9; 7]);
        let mut reader = Reader::new(&block);
        reader.entry().expect("the aligned entry parses");
        assert!(reader.entry().is_err(), "non-zero padding must be refused");
    }

    #[test]
    fn the_loss_contract_is_approximate_and_non_transforming() {
        let loss = loss_contract();
        assert!(!loss.transforms_vectors);
        assert!(loss.loss_encoding.is_none());
        assert!(loss.loss_parameters.is_none());
    }

    #[test]
    fn the_profile_declaration_binds_the_arithmetic() {
        let declaration = profile_declaration();
        assert_eq!(IMPLEMENTATION_ID, "hnsw-v2");
        assert!(
            declaration.contains("arithmetic=binary64-lane16-tree-v1"),
            "the digest must bind the law: {declaration}"
        );
        // The neighbour: every other line of the declaration is the one it was before
        // the arithmetic joined it, so the arithmetic line is the only one added.
        let lines: Vec<&str> = declaration.lines().collect();
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0], IMPLEMENTATION_ID);
        assert_eq!(lines[1], PARAMETER_ENCODING);
        assert_eq!(lines[2], crate::INDEX_MEDIA_TYPE);
        assert_eq!(lines[3], "approximate=true;transforms_vectors=false");
        assert_eq!(lines[5], LOSS_EVIDENCE);
        assert_eq!(PAYLOAD_VERSION, 2);
    }

    #[test]
    fn the_reassociated_evidence_is_pinned() {
        // Literal, so a change to any of its three parts is a visible edit of an
        // artifact-bound sentence and not a silent consequence of one.
        assert_eq!(
            loss_evidence_reassociated(Path::Avx2Fma),
            "approximate: recall measured against the exact oracle on synthetic corpora up \
             to 50,000 rows, and UNMEASURED at the 10^6 scale this index exists for; an offer \
             of candidates is never a proof of absence; reassociated binary64 arithmetic: \
             distance sums are reassociated and may be contracted to fused multiply-add along \
             the avx2+fma dispatch path of this build, so results may differ in the last bits \
             from the exact arithmetic and between dispatch paths or builds, the sign of a \
             zero result is unspecified, and near-ties may order differently. Its canonical \
             image is reproducible only by the compiled build that made it, running the same \
             dispatch path: the image records that build's target architecture and features, \
             and CPU tuning and the compiler version, which it cannot record, may change its \
             bits too."
        );
        for path in PATHS {
            let Some(evidence) = Reassociated::evidence(path) else {
                panic!("the reassociated arithmetic names its evidence along {path}");
            };
            assert_eq!(
                loss_evidence_reassociated(path),
                format!("{LOSS_EVIDENCE}; {evidence}. {REASSOCIATED_REPRODUCIBILITY}")
            );
        }
        // The neighbour: the exact arithmetic has no evidence, so its revision is the
        // profile's sentence alone.
        assert_eq!(loss_evidence_for::<Exact>(Path::Portable), LOSS_EVIDENCE);
    }

    #[test]
    fn every_image_code_has_a_published_row() {
        let rows = published();
        for (law, codes) in [
            (Exact::ID, Exact::IMAGE_CODES),
            (Reassociated::ID, Reassociated::IMAGE_CODES),
        ] {
            let mut gathered: Vec<u32> = rows
                .iter()
                .filter(|row| row.arithmetic == law)
                .flat_map(|row| row.codes.iter().copied())
                .collect();
            gathered.sort_unstable();
            assert_eq!(gathered, codes, "{law}: a code with no published row");
        }
        let exact: Vec<&Published> = rows
            .iter()
            .filter(|row| row.arithmetic == Exact::ID)
            .collect();
        assert_eq!(
            exact.len(),
            1,
            "the exact arithmetic publishes one revision"
        );
        assert_eq!(exact[0].implementation, IMPLEMENTATION_ID);
        assert_eq!(exact[0].revision, LOSS_EVIDENCE);
        for row in rows.iter().filter(|row| row.arithmetic == Reassociated::ID) {
            assert_eq!(row.implementation, IMPLEMENTATION_ID_REASSOCIATED);
            assert_eq!(row.codes.len(), 1, "a reassociated revision names one path");
        }
        let revisions: std::collections::BTreeSet<&str> =
            rows.iter().map(|row| row.revision.as_str()).collect();
        assert_eq!(revisions.len(), rows.len(), "no two rows share a revision");
    }

    #[test]
    fn the_reassociated_declaration_binds_its_arithmetic_and_path() {
        let declaration = profile_declaration_for::<Reassociated>(Path::Sse2);
        let lines: Vec<&str> = declaration.lines().collect();
        assert_eq!(lines.len(), 7);
        assert_eq!(lines[0], IMPLEMENTATION_ID_REASSOCIATED);
        assert_eq!(lines[4], "arithmetic=binary64-reassociated-v1");
        assert_eq!(lines[5], loss_evidence_reassociated(Path::Sse2));
        // The build is bound too, as its shape's bits.
        assert_eq!(
            lines[6],
            format!("build-shape={:016x}", BuildShape::here().bits())
        );
        assert_ne!(
            build_shape_line(BuildShape::here()),
            build_shape_line(BuildShape::from_bits(BuildShape::here().bits() ^ 1 << 40)),
            "two shapes are two declarations"
        );
        // Two paths are two profiles, and neither is the exact one.
        assert_ne!(
            declaration,
            profile_declaration_for::<Reassociated>(Path::Avx2Fma)
        );
        assert_ne!(
            implementation_for::<Reassociated>(Path::Sse2).digest,
            implementation().digest
        );
        assert_eq!(implementation_id_for::<Exact>(), IMPLEMENTATION_ID);
        assert_eq!(
            implementation_id_for::<Reassociated>(),
            IMPLEMENTATION_ID_REASSOCIATED
        );
    }

    #[test]
    fn the_implementation_identity_carries_the_evidence() {
        let identity = implementation();
        assert_eq!(identity.identifier, IMPLEMENTATION_ID);
        assert_eq!(
            identity.revision.as_deref(),
            Some(LOSS_EVIDENCE.as_bytes()),
            "the evidence sentence is bound into the guard identity"
        );
    }
}

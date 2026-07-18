// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PURREMB v1 whole-file framing and canonical section-directory assembly.

use sha2::{Digest as _, Sha256};

use crate::ContentDigest;

use super::error::EmbeddingError;
use super::identity::{ArtifactRoot, derive_artifact_root};

/// PURREMB v1 file magic.
pub const PURREMB_MAGIC: [u8; 8] = *b"PURREMB1";
/// PURREMB v1 trailer magic.
pub const PURREMB_TRAILER_MAGIC: [u8; 8] = *b"PURREND1";
/// PURREMB v1 format version.
pub const PURREMB_VERSION: u32 = 1;
/// Fixed PURREMB v1 header size.
pub const PURREMB_HEADER_LENGTH: u32 = 128;
/// Fixed PURREMB v1 directory-entry size.
pub const PURREMB_DIRECTORY_ENTRY_LENGTH: u64 = 64;
/// Fixed PURREMB v1 trailer size.
pub const PURREMB_TRAILER_LENGTH: u32 = 64;
/// Required file-relative section alignment.
pub const PURREMB_FILE_ALIGNMENT: u64 = 64;
/// Maximum number of directory entries in v1.
pub const PURREMB_MAX_SECTION_COUNT: u32 = 65_535;

/// Required-section flag.
pub const SECTION_CRITICAL: u32 = 1;
/// Derived-accelerator flag.
pub const SECTION_DERIVED: u32 = 1 << 1;

/// Exact source-pack binding section.
pub const SECTION_SOURCE: u32 = 0x0000_0001;
/// Vector-family contracts section.
pub const SECTION_CONTRACTS: u32 = 0x0000_0002;
/// Canonical target table section.
pub const SECTION_TARGETS: u32 = 0x0000_0003;
/// Canonical target-set table section.
pub const SECTION_TARGET_SETS: u32 = 0x0000_0004;
/// Structural target relations section.
pub const SECTION_RELATIONS: u32 = 0x0000_0005;
/// Family-scoped token-span section.
pub const SECTION_TOKEN_SPANS: u32 = 0x0000_0006;
/// Stored-matrix and effective-projection records section.
pub const SECTION_MATRICES: u32 = 0x0000_0007;
/// Exact external-artifact bindings section.
pub const SECTION_EXTERNAL_BINDINGS: u32 = 0x0000_0008;
/// Opaque derived-index guard section.
pub const SECTION_INDEX_GUARDS: u32 = 0x0000_0009;
/// Raw authoritative matrix-data section kind.
pub const SECTION_MATRIX_DATA: u32 = 0x0000_1000;
/// Raw inline index-payload section kind.
pub const SECTION_INDEX_PAYLOAD: u32 = 0x0000_1001;
/// First caller-extension section kind.
pub const SECTION_EXTENSION_MIN: u32 = 0x8000_0000;

const ROOT_OFFSET: usize = 64;
const ROOT_END: usize = 96;
const SOURCE_DIGEST_OFFSET: usize = 96;
const SOURCE_DIGEST_END: usize = 128;

/// The canonical sort key for one section-directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionKey {
    /// Numeric section kind.
    pub kind: u32,
    /// Instance number within the kind.
    pub instance: u32,
}

impl SectionKey {
    /// Constructs a section key.
    #[must_use]
    pub const fn new(kind: u32, instance: u32) -> Self {
        Self { kind, instance }
    }
}

/// One fully encoded section supplied to the canonical file assembler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SectionPayload {
    /// Section key written into the directory.
    pub(super) key: SectionKey,
    /// Exact v1 directory flags.
    pub(super) flags: u32,
    /// Exact section body, excluding file-level alignment padding.
    pub(super) bytes: Vec<u8>,
}

impl SectionPayload {
    /// Constructs an encoded section payload.
    #[must_use]
    pub(super) fn new(kind: u32, instance: u32, flags: u32, bytes: Vec<u8>) -> Self {
        Self {
            key: SectionKey::new(kind, instance),
            flags,
            bytes,
        }
    }
}

/// A section whose length is known before its bytes are streamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SectionDescriptor {
    /// Section key written into the directory.
    pub(super) key: SectionKey,
    /// Exact v1 directory flags.
    pub(super) flags: u32,
    /// Exact section length, excluding file-level padding.
    pub(super) length: u64,
}

impl SectionDescriptor {
    /// Constructs a section descriptor.
    #[must_use]
    pub(super) const fn new(kind: u32, instance: u32, flags: u32, length: u64) -> Self {
        Self {
            key: SectionKey::new(kind, instance),
            flags,
            length,
        }
    }
}

/// One planned directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DirectoryEntry {
    key: SectionKey,
    flags: u32,
    offset: u64,
    length: u64,
    sha256: Option<[u8; 32]>,
}

impl DirectoryEntry {
    /// Section key.
    #[must_use]
    pub(super) const fn key(self) -> SectionKey {
        self.key
    }

    /// Absolute section offset.
    #[must_use]
    pub(super) const fn offset(self) -> u64 {
        self.offset
    }

    /// Exact section length.
    #[must_use]
    pub(super) const fn length(self) -> u64 {
        self.length
    }
}

/// Canonical offsets and directory state for one PURREMB file.
#[derive(Debug, Clone)]
pub(super) struct FileLayout {
    source_exact_digest: ContentDigest,
    entries: Vec<DirectoryEntry>,
    directory_length: u64,
    first_section_offset: u64,
    trailer_offset: u64,
    file_length: u64,
}

impl FileLayout {
    /// Plans canonical offsets for already-sized sections.
    ///
    /// Descriptors may arrive in any order; this function sorts them by
    /// `(kind, instance)` and rejects every noncanonical cardinality or flag.
    pub(super) fn plan(
        source_exact_digest: ContentDigest,
        mut descriptors: Vec<SectionDescriptor>,
    ) -> Result<Self, EmbeddingError> {
        descriptors.sort_unstable_by_key(|descriptor| descriptor.key);
        validate_descriptors(&descriptors)?;

        let section_count = u64::try_from(descriptors.len())
            .map_err(|_| EmbeddingError::ArithmeticOverflow("section count"))?;
        let directory_length = section_count
            .checked_mul(PURREMB_DIRECTORY_ENTRY_LENGTH)
            .ok_or(EmbeddingError::ArithmeticOverflow("directory length"))?;
        let directory_end = u64::from(PURREMB_HEADER_LENGTH)
            .checked_add(directory_length)
            .ok_or(EmbeddingError::ArithmeticOverflow("directory end"))?;
        let first_section_offset = checked_align_up(directory_end, PURREMB_FILE_ALIGNMENT)?;

        let mut cursor = first_section_offset;
        let mut entries = Vec::with_capacity(descriptors.len());
        for descriptor in descriptors {
            entries.push(DirectoryEntry {
                key: descriptor.key,
                flags: descriptor.flags,
                offset: cursor,
                length: descriptor.length,
                sha256: None,
            });
            let end = cursor
                .checked_add(descriptor.length)
                .ok_or(EmbeddingError::ArithmeticOverflow("section end"))?;
            cursor = checked_align_up(end, PURREMB_FILE_ALIGNMENT)?;
        }

        let trailer_offset = cursor;
        let file_length = trailer_offset
            .checked_add(u64::from(PURREMB_TRAILER_LENGTH))
            .ok_or(EmbeddingError::ArithmeticOverflow("file length"))?;

        Ok(Self {
            source_exact_digest,
            entries,
            directory_length,
            first_section_offset,
            trailer_offset,
            file_length,
        })
    }

    /// Planned entries in canonical order.
    #[must_use]
    pub(super) fn entries(&self) -> &[DirectoryEntry] {
        &self.entries
    }

    /// Offset of the first section.
    #[must_use]
    pub(super) const fn first_section_offset(&self) -> u64 {
        self.first_section_offset
    }

    /// Offset of the fixed trailer.
    #[must_use]
    pub(super) const fn trailer_offset(&self) -> u64 {
        self.trailer_offset
    }

    /// Exact final file length.
    #[must_use]
    pub(super) const fn file_length(&self) -> u64 {
        self.file_length
    }

    /// Finds a planned directory entry.
    #[must_use]
    pub(super) fn entry(&self, key: SectionKey) -> Option<DirectoryEntry> {
        self.entries
            .binary_search_by_key(&key, |entry| entry.key)
            .ok()
            .map(|index| self.entries[index])
    }

    /// Supplies the plain SHA-256 for one planned section.
    pub(super) fn set_section_digest(
        &mut self,
        key: SectionKey,
        digest: [u8; 32],
    ) -> Result<(), EmbeddingError> {
        let index = self
            .entries
            .binary_search_by_key(&key, |entry| entry.key)
            .map_err(|_| EmbeddingError::MissingReference("planned section"))?;
        self.entries[index].sha256 = Some(digest);
        Ok(())
    }

    /// Encodes the populated canonical directory.
    pub(super) fn directory_bytes(&self) -> Result<Vec<u8>, EmbeddingError> {
        let capacity = usize::try_from(self.directory_length)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("directory allocation"))?;
        let mut output = Vec::with_capacity(capacity);
        for entry in &self.entries {
            let digest = entry
                .sha256
                .ok_or(EmbeddingError::Missing("section digest"))?;
            push_u32(&mut output, entry.key.kind);
            push_u32(&mut output, entry.flags);
            push_u32(&mut output, entry.key.instance);
            push_u32(&mut output, 0);
            push_u64(&mut output, entry.offset);
            push_u64(&mut output, entry.length);
            output.extend_from_slice(&digest);
        }
        if output.len() != capacity {
            return Err(EmbeddingError::Malformed(
                "directory encoding length disagrees with its plan",
            ));
        }
        Ok(output)
    }

    /// Encodes the canonical header with a zero artifact-root field.
    #[must_use]
    pub(super) fn header_zero_root(&self) -> [u8; PURREMB_HEADER_LENGTH as usize] {
        self.header(ArtifactRoot::from_raw([0; 32]))
    }

    /// Computes the artifact root after every section digest has been supplied.
    pub(super) fn artifact_root(&self) -> Result<ArtifactRoot, EmbeddingError> {
        let directory = self.directory_bytes()?;
        Ok(derive_artifact_root(&self.header_zero_root(), &directory))
    }

    /// Encodes the final header.
    #[must_use]
    pub(super) fn header(&self, root: ArtifactRoot) -> [u8; PURREMB_HEADER_LENGTH as usize] {
        let mut output = [0u8; PURREMB_HEADER_LENGTH as usize];
        output[0..8].copy_from_slice(&PURREMB_MAGIC);
        put_u32(&mut output, 8, PURREMB_VERSION);
        put_u32(&mut output, 12, PURREMB_HEADER_LENGTH);
        put_u32(&mut output, 16, 0);
        put_u32(
            &mut output,
            20,
            u32::try_from(self.entries.len()).expect("v1 section count was validated"),
        );
        put_u64(&mut output, 24, u64::from(PURREMB_HEADER_LENGTH));
        put_u64(&mut output, 32, self.directory_length);
        put_u64(&mut output, 40, self.first_section_offset);
        put_u64(&mut output, 48, self.trailer_offset);
        put_u64(&mut output, 56, self.file_length);
        output[ROOT_OFFSET..ROOT_END].copy_from_slice(root.as_bytes());
        output[SOURCE_DIGEST_OFFSET..SOURCE_DIGEST_END]
            .copy_from_slice(self.source_exact_digest.as_bytes());
        output
    }

    /// Encodes the final fixed trailer.
    #[must_use]
    pub(super) fn trailer(&self, root: ArtifactRoot) -> [u8; PURREMB_TRAILER_LENGTH as usize] {
        let mut output = [0u8; PURREMB_TRAILER_LENGTH as usize];
        output[0..8].copy_from_slice(&PURREMB_TRAILER_MAGIC);
        put_u32(&mut output, 8, PURREMB_VERSION);
        put_u32(&mut output, 12, PURREMB_TRAILER_LENGTH);
        put_u64(&mut output, 16, self.file_length);
        output[24..56].copy_from_slice(root.as_bytes());
        output
    }
}

/// Result of canonical in-memory file assembly.
#[derive(Debug, Clone)]
pub struct EncodedArtifact {
    /// Complete canonical file bytes.
    pub bytes: Vec<u8>,
    /// Whole-artifact integrity root.
    pub root: ArtifactRoot,
}

/// Assembles a canonical PURREMB file from fully encoded section bodies.
pub(super) fn encode_artifact(
    source_exact_digest: ContentDigest,
    mut sections: Vec<SectionPayload>,
) -> Result<EncodedArtifact, EmbeddingError> {
    sections.sort_unstable_by_key(|section| section.key);
    validate_source_section(source_exact_digest, &sections)?;

    let descriptors = sections
        .iter()
        .map(|section| {
            let length = u64::try_from(section.bytes.len())
                .map_err(|_| EmbeddingError::ArithmeticOverflow("section length"))?;
            Ok(SectionDescriptor {
                key: section.key,
                flags: section.flags,
                length,
            })
        })
        .collect::<Result<Vec<_>, EmbeddingError>>()?;
    let mut layout = FileLayout::plan(source_exact_digest, descriptors)?;
    for section in &sections {
        layout.set_section_digest(section.key, Sha256::digest(&section.bytes).into())?;
    }

    let root = layout.artifact_root()?;
    let file_len = usize::try_from(layout.file_length())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("file allocation"))?;
    let mut output = vec![0u8; file_len];
    output[..PURREMB_HEADER_LENGTH as usize].copy_from_slice(&layout.header(root));
    let directory = layout.directory_bytes()?;
    let directory_end = PURREMB_HEADER_LENGTH as usize + directory.len();
    output[PURREMB_HEADER_LENGTH as usize..directory_end].copy_from_slice(&directory);

    for section in &sections {
        let entry = layout
            .entry(section.key)
            .ok_or(EmbeddingError::MissingReference("section layout"))?;
        let start = usize::try_from(entry.offset())
            .map_err(|_| EmbeddingError::ArithmeticOverflow("section offset"))?;
        let end = start
            .checked_add(section.bytes.len())
            .ok_or(EmbeddingError::ArithmeticOverflow("section copy"))?;
        output
            .get_mut(start..end)
            .ok_or_else(|| EmbeddingError::InvalidSpan {
                context: "section output",
                offset: entry.offset(),
                length: entry.length(),
            })?
            .copy_from_slice(&section.bytes);
    }

    let trailer_offset = usize::try_from(layout.trailer_offset())
        .map_err(|_| EmbeddingError::ArithmeticOverflow("trailer offset"))?;
    output[trailer_offset..].copy_from_slice(&layout.trailer(root));
    Ok(EncodedArtifact {
        bytes: output,
        root,
    })
}

/// Checked `align_up` for the normative power-of-two alignments.
pub(super) fn checked_align_up(value: u64, alignment: u64) -> Result<u64, EmbeddingError> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(EmbeddingError::Malformed("alignment is not a power of two"));
    }
    let biased = value
        .checked_add(alignment - 1)
        .ok_or(EmbeddingError::ArithmeticOverflow("alignment"))?;
    Ok(biased & !(alignment - 1))
}

fn validate_source_section(
    source_exact_digest: ContentDigest,
    sections: &[SectionPayload],
) -> Result<(), EmbeddingError> {
    let index = sections
        .binary_search_by_key(&SectionKey::new(SECTION_SOURCE, 0), |section| section.key)
        .map_err(|_| EmbeddingError::Missing("SOURCE section"))?;
    let source = &sections[index].bytes;
    if source.len() != 128 {
        return Err(EmbeddingError::InvalidSpan {
            context: "SOURCE section",
            offset: 0,
            length: u64::try_from(source.len()).unwrap_or(u64::MAX),
        });
    }
    if source[24..56] != source_exact_digest.as_bytes()[..] {
        let mut expected = [0u8; 32];
        expected.copy_from_slice(&source[24..56]);
        return Err(EmbeddingError::DigestMismatch {
            kind: super::error::DigestKind::SourceExact,
            expected,
            actual: *source_exact_digest.as_bytes(),
        });
    }
    Ok(())
}

fn validate_descriptors(descriptors: &[SectionDescriptor]) -> Result<(), EmbeddingError> {
    let count = descriptors.len();
    if count < 10 {
        return Err(EmbeddingError::CountLimit {
            field: "section",
            value: u64::try_from(count).unwrap_or(u64::MAX),
        });
    }
    if count > PURREMB_MAX_SECTION_COUNT as usize {
        return Err(EmbeddingError::CountLimit {
            field: "section",
            value: u64::try_from(count).unwrap_or(u64::MAX),
        });
    }

    let mut previous = None;
    let mut singleton_seen = [false; 9];
    let mut expected_matrix_instance = 1u32;
    let mut expected_index_instance = 1u32;
    for descriptor in descriptors {
        if descriptor.length == 0 {
            return Err(EmbeddingError::InvalidSpan {
                context: "empty section",
                offset: 0,
                length: 0,
            });
        }
        if previous.is_some_and(|key| descriptor.key <= key) {
            return Err(EmbeddingError::NonCanonicalOrder("section directory"));
        }
        previous = Some(descriptor.key);

        match descriptor.key.kind {
            SECTION_SOURCE..=SECTION_INDEX_GUARDS => {
                if descriptor.key.instance != 0 {
                    return Err(EmbeddingError::Malformed(
                        "singleton section has a nonzero instance",
                    ));
                }
                let index = usize::try_from(descriptor.key.kind - 1)
                    .expect("singleton section index fits usize");
                if singleton_seen[index] {
                    return Err(EmbeddingError::Duplicate("singleton section"));
                }
                singleton_seen[index] = true;
                let expected_flags = if descriptor.key.kind == SECTION_INDEX_GUARDS {
                    SECTION_CRITICAL | SECTION_DERIVED
                } else {
                    SECTION_CRITICAL
                };
                if descriptor.flags != expected_flags {
                    return Err(EmbeddingError::ReservedNonzero("section flags"));
                }
                if descriptor.key.kind == SECTION_SOURCE && descriptor.length != 128 {
                    return Err(EmbeddingError::Malformed("SOURCE section is not 128 bytes"));
                }
            }
            SECTION_MATRIX_DATA => {
                if descriptor.flags != SECTION_CRITICAL {
                    return Err(EmbeddingError::ReservedNonzero("MATRIX_DATA flags"));
                }
                if descriptor.key.instance != expected_matrix_instance {
                    return Err(EmbeddingError::NonCanonicalOrder("MATRIX_DATA instances"));
                }
                expected_matrix_instance = expected_matrix_instance
                    .checked_add(1)
                    .ok_or(EmbeddingError::ArithmeticOverflow("matrix instances"))?;
            }
            SECTION_INDEX_PAYLOAD => {
                if descriptor.flags != SECTION_CRITICAL | SECTION_DERIVED {
                    return Err(EmbeddingError::ReservedNonzero("INDEX_PAYLOAD flags"));
                }
                if descriptor.key.instance != expected_index_instance {
                    return Err(EmbeddingError::NonCanonicalOrder("INDEX_PAYLOAD instances"));
                }
                expected_index_instance = expected_index_instance
                    .checked_add(1)
                    .ok_or(EmbeddingError::ArithmeticOverflow("index instances"))?;
            }
            kind if kind >= SECTION_EXTENSION_MIN => {
                if descriptor.flags & !(SECTION_CRITICAL | SECTION_DERIVED) != 0 {
                    return Err(EmbeddingError::ReservedNonzero("extension section flags"));
                }
            }
            kind => {
                return Err(EmbeddingError::UnsupportedCode {
                    field: "section kind",
                    value: kind,
                });
            }
        }
    }

    if singleton_seen.iter().any(|seen| !seen) {
        return Err(EmbeddingError::Missing("singleton metadata section"));
    }
    if expected_matrix_instance == 1 {
        return Err(EmbeddingError::Missing("MATRIX_DATA section"));
    }
    Ok(())
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_sections(source_digest: ContentDigest) -> Vec<SectionPayload> {
        let mut source = vec![0u8; 128];
        source[24..56].copy_from_slice(source_digest.as_bytes());
        let mut sections = vec![
            SectionPayload::new(SECTION_SOURCE, 0, SECTION_CRITICAL, source),
            SectionPayload::new(SECTION_CONTRACTS, 0, SECTION_CRITICAL, vec![2]),
            SectionPayload::new(SECTION_TARGETS, 0, SECTION_CRITICAL, vec![3]),
            SectionPayload::new(SECTION_TARGET_SETS, 0, SECTION_CRITICAL, vec![4]),
            SectionPayload::new(SECTION_RELATIONS, 0, SECTION_CRITICAL, vec![5]),
            SectionPayload::new(SECTION_TOKEN_SPANS, 0, SECTION_CRITICAL, vec![6]),
            SectionPayload::new(SECTION_MATRICES, 0, SECTION_CRITICAL, vec![7]),
            SectionPayload::new(SECTION_EXTERNAL_BINDINGS, 0, SECTION_CRITICAL, vec![8]),
            SectionPayload::new(
                SECTION_INDEX_GUARDS,
                0,
                SECTION_CRITICAL | SECTION_DERIVED,
                vec![9],
            ),
            SectionPayload::new(SECTION_MATRIX_DATA, 1, SECTION_CRITICAL, vec![10]),
        ];
        sections.reverse();
        sections
    }

    #[test]
    fn canonical_assembly_sorts_and_aligns_sections() {
        let source_digest = ContentDigest::of(b"source");
        let artifact = encode_artifact(source_digest, minimal_sections(source_digest)).unwrap();
        assert_eq!(&artifact.bytes[0..8], b"PURREMB1");
        assert_eq!(
            &artifact.bytes[artifact.bytes.len() - 64..][0..8],
            b"PURREND1"
        );
        assert_eq!(artifact.bytes.len() % 64, 0);

        let section_count = u32::from_le_bytes(artifact.bytes[20..24].try_into().unwrap());
        assert_eq!(section_count, 10);
        let first = u64::from_le_bytes(artifact.bytes[40..48].try_into().unwrap());
        assert_eq!(first % 64, 0);
        let first_kind = u32::from_le_bytes(artifact.bytes[128..132].try_into().unwrap());
        assert_eq!(first_kind, SECTION_SOURCE);
    }

    #[test]
    fn artifact_root_changes_with_a_section() {
        let source_digest = ContentDigest::of(b"source");
        let a = encode_artifact(source_digest, minimal_sections(source_digest)).unwrap();
        let mut sections = minimal_sections(source_digest);
        sections
            .iter_mut()
            .find(|section| section.key.kind == SECTION_MATRIX_DATA)
            .unwrap()
            .bytes[0] ^= 1;
        let b = encode_artifact(source_digest, sections).unwrap();
        assert_ne!(a.root, b.root);
        assert_ne!(a.bytes, b.bytes);
    }

    #[test]
    fn rejects_noncontiguous_matrix_instances() {
        let source_digest = ContentDigest::of(b"source");
        let mut sections = minimal_sections(source_digest);
        sections
            .iter_mut()
            .find(|section| section.key.kind == SECTION_MATRIX_DATA)
            .unwrap()
            .key
            .instance = 2;
        assert!(matches!(
            encode_artifact(source_digest, sections),
            Err(EmbeddingError::NonCanonicalOrder(_))
        ));
    }

    #[test]
    fn alignment_is_checked() {
        assert_eq!(checked_align_up(0, 64).unwrap(), 0);
        assert_eq!(checked_align_up(1, 64).unwrap(), 64);
        assert_eq!(checked_align_up(64, 64).unwrap(), 64);
        assert!(checked_align_up(u64::MAX, 64).is_err());
        assert!(checked_align_up(1, 3).is_err());
    }
}

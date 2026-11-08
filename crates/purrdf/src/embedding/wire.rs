// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! PURREMB v1 whole-file framing and canonical section-directory assembly.

use purrdf_core::ContentDigest;
use purrdf_core::embedding::{
    ArtifactRoot, DigestKind, EmbeddingError, PURREMB_DIRECTORY_ENTRY_LENGTH,
    PURREMB_FILE_ALIGNMENT, PURREMB_HEADER_LENGTH, PURREMB_MAGIC, PURREMB_MAX_SECTION_COUNT,
    PURREMB_TRAILER_LENGTH, PURREMB_TRAILER_MAGIC, PURREMB_VERSION, SECTION_CRITICAL,
    SECTION_DERIVED, SECTION_EXTENSION_MIN, SECTION_INDEX_GUARDS, SECTION_INDEX_PAYLOAD,
    SECTION_MATRIX_DATA, SECTION_SOURCE, SectionKey, derive_artifact_root, put_u32, put_u64,
};
use sha2::{Digest as _, Sha256};

const ROOT_OFFSET: usize = 64;
const ROOT_END: usize = 96;
const SOURCE_DIGEST_OFFSET: usize = 96;
const SOURCE_DIGEST_END: usize = 128;

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
        let first_section_offset = directory_end
            .checked_next_multiple_of(PURREMB_FILE_ALIGNMENT)
            .ok_or(EmbeddingError::ArithmeticOverflow("alignment"))?;

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
            cursor = end
                .checked_next_multiple_of(PURREMB_FILE_ALIGNMENT)
                .ok_or(EmbeddingError::ArithmeticOverflow("alignment"))?;
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
            output.extend_from_slice(&entry.key.kind.to_le_bytes());
            output.extend_from_slice(&entry.flags.to_le_bytes());
            output.extend_from_slice(&entry.key.instance.to_le_bytes());
            output.extend_from_slice(&0u32.to_le_bytes());
            output.extend_from_slice(&entry.offset.to_le_bytes());
            output.extend_from_slice(&entry.length.to_le_bytes());
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
            kind: DigestKind::SourceExact,
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

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::embedding::{
        SECTION_CONTRACTS, SECTION_EXTERNAL_BINDINGS, SECTION_MATRICES, SECTION_RELATIONS,
        SECTION_TARGET_SETS, SECTION_TARGETS, SECTION_TOKEN_SPANS,
    };

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
}

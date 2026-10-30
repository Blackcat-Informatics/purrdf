// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! PURREMB v1 whole-file framing and canonical section-directory assembly.

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

/// Write a fixed `u32` field of an image this module's writers sized
/// themselves, through [`crate::bytes::put_u32_le`]: an offset outside it is a
/// writer defect, not an input to refuse.
pub fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    crate::bytes::put_u32_le(bytes, offset, value)
        .expect("a field inside the image its writer sized");
}

/// Write a fixed `u64` field of a writer-sized image (see [`put_u32`]).
pub fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    crate::bytes::put_u64_le(bytes, offset, value)
        .expect("a field inside the image its writer sized");
}

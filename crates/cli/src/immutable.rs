// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The immutable-input authority: a byte buffer guaranteed **stable and
//! un-truncatable** for its whole lifetime.
//!
//! A pack on disk is read by memory-mapping it and handing the mapped bytes to
//! [`PackView`](purrdf_core::PackView) zero-copy. A bare `mmap` of a file this
//! process does not own is memory-**unsafe**: a hostile concurrent writer can
//! `ftruncate` the backing inode, and a later access to a now-missing page faults
//! the process with **SIGBUS** — an uncatchable crash. [`ImmutableInput`] closes
//! that hole by only ever handing out bytes whose backing object cannot shrink or
//! be mutated for the lifetime of the value.
//!
//! ## One open, one identity
//!
//! A path is opened **exactly once**; every subsequent decision — the `fstat` for
//! length, the Tier-1 snapshot copy, and the Tier-2 fallback read — is made against
//! that single file descriptor, never by re-resolving the path. Re-opening the path
//! would let a hostile pathname writer swap the file (or substitute a special file)
//! *between* the opens, so the object we inspected, the object we copied, and the
//! object we returned could diverge. Binding everything to one descriptor makes the
//! acquired bytes tied to the file that existed at open time.
//!
//! ## Tiered acquisition
//!
//! From that one descriptor, the cheapest sound option is taken, degrading — never
//! silently, always to something equally safe:
//!
//! * **Tier 0 — direct `mmap`, truly zero-copy (no byte copy).** Taken only when
//!   the descriptor already carries a *testable, kernel-enforced* invariant that
//!   prevents shrink/grow/mutation for the mapping's lifetime: it reports
//!   `F_SEAL_SHRINK | F_SEAL_GROW | F_SEAL_WRITE` via `fcntl(F_GET_SEALS)`. A regular
//!   disk file never carries seals, so this fires only for a pre-sealed `memfd`; a
//!   read-only mount or a bare `O_TMPFILE` is deliberately **not** accepted (a mount
//!   can be remounted read-write; an unsealed tmpfile can still be truncated).
//! * **Tier 1 — sealed `memfd` snapshot (one copy).** The common case for a regular
//!   file: snapshot the bytes (via offset-independent `pread`, so the descriptor's
//!   offset is never disturbed) into an anonymous `memfd` **we own**, seal it
//!   (`SHRINK | GROW | WRITE | SEAL`), *verify the complete seal set took*, and map
//!   the sealed fd. No external pathname writer can touch our memfd, so access can
//!   never SIGBUS.
//! * **Tier 2 — owned buffer (fail closed, portable).** stdin, a non-Linux target,
//!   a kernel without `memfd`/sealing, a non-regular file, or an **empty** input
//!   (a zero-length mapping is invalid): read the descriptor (rewound to offset 0)
//!   whole into a `Vec<u8>` we own — no external process can truncate our heap.
//!
//! Integrity (`verify_pack`) is checked by the caller **after** acquisition, over
//! the already-immutable [`as_bytes`](ImmutableInput::as_bytes), closing the
//! time-of-check/time-of-use gap. Acquisition never trusts the input's *contents*; a
//! zero-length or otherwise invalid pack is rejected by that later `verify_pack`.

use std::fs::File;
use std::io::{Read as _, Seek as _};

use memmap2::Mmap;

use crate::error::CliError;

/// Which acquisition tier produced an [`ImmutableInput`]'s bytes.
///
/// Reported for the CLI pack benchmark (so acquisition cost can be attributed to a
/// tier) and for diagnostics; it carries no correctness obligation — every tier is
/// memory-safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTier {
    /// Tier 0: a direct mapping of a descriptor whose verified
    /// `SHRINK | GROW | WRITE` seals make it immutable for the mapping's lifetime.
    SealedDirectMmap,
    /// Tier 1: a mapping of an anonymous, sealed `memfd` snapshot we own (one copy).
    SealedSnapshotMmap,
    /// Tier 2: an owned in-memory buffer.
    Owned,
}

/// An owner of immutable bytes, stable and un-truncatable for its lifetime.
///
/// Obtain one with [`from_disk_path`](Self::from_disk_path),
/// [`from_stdin`](Self::from_stdin), or [`from_owned`](Self::from_owned), then read
/// the bytes with [`as_bytes`](Self::as_bytes). The value must be held alive for as
/// long as the bytes are used (a [`PackView`](purrdf_core::PackView) borrows them).
#[derive(Debug)]
pub enum ImmutableInput {
    /// A memory mapping whose backing object cannot shrink, grow, or be written for
    /// the mapping's lifetime — a descriptor with verified seals (Tier 0) or our own
    /// sealed `memfd` snapshot (Tier 1).
    Mapped {
        /// The read-only mapping. Held alive for the value's lifetime.
        mmap: Mmap,
        /// Which tier produced this mapping.
        tier: InputTier,
    },
    /// An owned buffer (Tier 2).
    Owned(Vec<u8>),
}

impl ImmutableInput {
    /// The immutable bytes. Stable for the lifetime of `self`.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Mapped { mmap, .. } => &mmap[..],
            Self::Owned(buffer) => &buffer[..],
        }
    }

    /// The acquisition tier that produced these bytes.
    pub fn tier(&self) -> InputTier {
        match self {
            Self::Mapped { tier, .. } => *tier,
            Self::Owned(_) => InputTier::Owned,
        }
    }

    /// Acquire immutable bytes from a disk `path`, tiered (Tier 0 → 1 → 2).
    ///
    /// The path is opened **exactly once**; the acquisition is then performed by
    /// [`from_opened_file`](Self::from_opened_file) against that single descriptor,
    /// which has no path to re-resolve — so a hostile pathname swap cannot divert it.
    ///
    /// # Errors
    ///
    /// Returns a [`CliError`] if the path cannot be opened or read. A tier that is
    /// *unavailable* (no seals, no `memfd`, an empty or non-regular file) is not an
    /// error — it degrades to the next tier; only an actual I/O failure surfaces.
    pub fn from_disk_path(path: &str) -> Result<Self, CliError> {
        Self::from_opened_file(File::open(path)?, false)
    }

    /// The post-open acquisition seam: tiered acquisition off an ALREADY-OPEN
    /// descriptor.
    ///
    /// This function has no `path` and therefore CANNOT re-resolve one — the acquired
    /// bytes are structurally bound to `file`, the descriptor the caller opened. That
    /// is what makes a hostile pathname swap unable to divert acquisition (a re-open
    /// per tier is not even expressible here), and it is what the
    /// `acquired_bytes_are_tied_to_the_opened_descriptor` regression exercises: it
    /// swaps the path *before* calling this, so a re-opening implementation would
    /// return the replacement bytes and fail the test.
    ///
    /// `force_tier1_failure` (only ever `true` in this module's tests) makes Tier 1
    /// perform its snapshot `pread` and then bail as if sealing failed, so a test can
    /// deterministically exercise the Tier-2 fallback *after* Tier 1 ran and prove
    /// the full input still reaches the owned buffer.
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
    fn from_opened_file(mut file: File, force_tier1_failure: bool) -> Result<Self, CliError> {
        let meta = file.metadata()?;

        if meta.is_file() {
            // A zero-length mapping is invalid; own it (the later `verify_pack`
            // rejects an empty pack — a content decision after a safe acquisition).
            if meta.len() == 0 {
                return Ok(Self::Owned(Vec::new()));
            }

            #[cfg(target_os = "linux")]
            {
                if let Some(mapped) = tier0_sealed_direct(&file)? {
                    return Ok(mapped);
                }
                if let Some(mapped) = tier1_sealed_snapshot(&file, meta.len(), force_tier1_failure)?
                {
                    return Ok(mapped);
                }
            }

            // Tier 2 for a regular file: rewind the SAME descriptor and read it
            // whole. `pread`/`mmap` above never moved the offset, but rewind is
            // explicit so the read is unambiguously from byte zero of this fd.
            file.rewind()?;
            let mut buffer = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
            file.read_to_end(&mut buffer)?;
            return Ok(Self::Owned(buffer));
        }

        // Non-regular (a FIFO/socket the path resolved to): not seekable and not
        // mappable — stream the SAME descriptor to own its bytes.
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(Self::Owned(buffer))
    }

    /// Acquire immutable bytes from standard input (Tier 2 only — there is no file
    /// descriptor to seal or map).
    ///
    /// # Errors
    ///
    /// Returns a [`CliError`] if stdin cannot be read.
    pub fn from_stdin() -> Result<Self, CliError> {
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer)?;
        Ok(Self::Owned(buffer))
    }

    /// Wrap bytes already owned in memory (Tier 2).
    pub fn from_owned(bytes: Vec<u8>) -> Self {
        Self::Owned(bytes)
    }
}

/// Tier 0: if `file`'s descriptor already carries the verified complete invariant
/// `SHRINK | GROW | WRITE`, a direct read-only mapping is sound with **no copy** —
/// the kernel guarantees the backing object cannot shrink, grow, or be written for
/// the mapping's lifetime.
///
/// Returns `Ok(None)` (degrade) when the fd carries no such seals — the ordinary
/// case for a regular disk file, whose `fcntl(F_GET_SEALS)` fails or lacks the
/// required bits.
#[cfg(target_os = "linux")]
fn tier0_sealed_direct(file: &File) -> Result<Option<ImmutableInput>, CliError> {
    use rustix::fs::{SealFlags, fcntl_get_seals};

    // A descriptor that does not support sealing (a regular disk file) errors here;
    // that is not a failure, it just means Tier 0 does not apply.
    let Ok(seals) = fcntl_get_seals(file) else {
        return Ok(None);
    };
    // The COMPLETE invariant: shrink AND grow AND write are all sealed. Anything
    // less does not prove the backing object is immutable for the mapping lifetime.
    if !seals.contains(SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE) {
        return Ok(None);
    }
    // SAFETY: the descriptor carries the verified complete seal set
    // (`SHRINK | GROW | WRITE`), so its backing object's size and contents are
    // immutable for the whole lifetime of the mapping. No external truncation,
    // growth, or write can fault the mapping — exactly the invariant `Mmap::map`'s
    // safety contract requires. (The caller has already excluded zero length.)
    let mmap = unsafe { Mmap::map(file)? };
    Ok(Some(ImmutableInput::Mapped {
        mmap,
        tier: InputTier::SealedDirectMmap,
    }))
}

/// Tier 1: snapshot `file`'s bytes into an anonymous `memfd` we own, seal it against
/// shrink/grow/write, verify the complete seal set took, and map the sealed fd.
///
/// The copy uses offset-independent `pread` on the SAME descriptor the caller
/// opened — it neither re-opens the path nor disturbs the descriptor's offset, so
/// the Tier-2 fallback can still read the identical fd from byte zero. Returns
/// `Ok(None)` (degrade) when `memfd`/sealing is unavailable, when the snapshot turns
/// out empty (the file was truncated under us), or — for the test seam — when
/// `force_failure` is set.
#[cfg(target_os = "linux")]
fn tier1_sealed_snapshot(
    file: &File,
    len: u64,
    force_failure: bool,
) -> Result<Option<ImmutableInput>, CliError> {
    use std::io::Write as _;
    use std::os::unix::fs::FileExt as _;

    use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, fcntl_get_seals, memfd_create};

    // An anonymous, sealable, close-on-exec memory file. If the kernel has no
    // `memfd_create`, degrade to the owned tier.
    let Ok(memfd) = memfd_create(
        "purrdf-pack",
        MemfdFlags::ALLOW_SEALING | MemfdFlags::CLOEXEC,
    ) else {
        return Ok(None);
    };
    let mut sink = File::from(memfd);

    // One O(n) snapshot copy, via `pread` on the caller's descriptor (no offset
    // change, no re-open). If the file shrank under us, we copy only what is there;
    // the sealed snapshot is internally consistent and the later `verify_pack`
    // rejects a short/invalid pack.
    let mut offset: u64 = 0;
    // Heap buffer (not a stack array): reused across the copy loop, one allocation.
    let mut chunk = vec![0u8; 64 * 1024];
    while offset < len {
        let read = file.read_at(&mut chunk, offset)?;
        if read == 0 {
            break;
        }
        sink.write_all(&chunk[..read])?;
        offset += read as u64;
    }

    // A zero-length memfd cannot be mapped; degrade so the owned tier handles it.
    if offset == 0 {
        return Ok(None);
    }
    // Test seam: model a post-copy seal failure so the caller falls through to the
    // owned tier with the snapshot `pread` already having run.
    if force_failure {
        return Ok(None);
    }

    // Seal the memfd shut: no shrink, no grow, no write, and no further sealing.
    let seals = SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE | SealFlags::SEAL;
    if fcntl_add_seals(&sink, seals).is_err() {
        // The seal could not be applied (should not happen for a fresh
        // ALLOW_SEALING memfd, but never map an unsealed object): degrade.
        return Ok(None);
    }
    // Verify the COMPLETE invariant actually holds before we rely on it for memory
    // safety.
    let Ok(applied) = fcntl_get_seals(&sink) else {
        return Ok(None);
    };
    if !applied.contains(SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE) {
        return Ok(None);
    }

    // SAFETY: `sink` is our own anonymous `memfd`, now carrying the verified
    // complete seal set `SHRINK | GROW | WRITE` (confirmed just above) and non-empty
    // (`offset > 0`). No other process holds it, and its size and contents can no
    // longer change, so the read-only mapping cannot be faulted by truncation,
    // growth, or mutation for its whole lifetime.
    let mmap = unsafe { Mmap::map(&sink)? };
    Ok(Some(ImmutableInput::Mapped {
        mmap,
        tier: InputTier::SealedSnapshotMmap,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn temp_with(payload: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        file.write_all(payload).expect("write payload");
        file.flush().expect("flush");
        file
    }

    fn path_of(file: &tempfile::NamedTempFile) -> &str {
        file.path().to_str().expect("utf-8 path")
    }

    #[test]
    fn owned_from_bytes_round_trips() {
        let input = ImmutableInput::from_owned(vec![1, 2, 3, 4]);
        assert_eq!(input.as_bytes(), &[1, 2, 3, 4]);
        assert_eq!(input.tier(), InputTier::Owned);
    }

    #[test]
    fn disk_path_bytes_match_file_contents() {
        let payload: Vec<u8> = (0u8..=255).cycle().take(8192).collect();
        let file = temp_with(&payload);

        let input = ImmutableInput::from_disk_path(path_of(&file)).expect("acquire");
        assert_eq!(input.as_bytes(), payload.as_slice());
        // On Linux a regular disk file is not sealable, so Tier 0 is skipped and the
        // sealed-memfd snapshot (Tier 1) is taken; elsewhere the owned tier is used.
        #[cfg(target_os = "linux")]
        assert_eq!(input.tier(), InputTier::SealedSnapshotMmap);
        #[cfg(not(target_os = "linux"))]
        assert_eq!(input.tier(), InputTier::Owned);
    }

    #[test]
    fn empty_disk_file_is_acquired_as_owned_empty() {
        let file = temp_with(&[]);
        let input = ImmutableInput::from_disk_path(path_of(&file)).expect("acquire");
        assert!(input.as_bytes().is_empty());
        assert_eq!(input.tier(), InputTier::Owned);
    }

    #[test]
    fn tier1_failure_falls_back_to_full_owned_bytes() {
        // Larger than a page so a truncated/short read would be obvious.
        let payload: Vec<u8> = (0u8..=255).cycle().take(64 * 1024).collect();
        let file = temp_with(&payload);

        // Force Tier 1 to run its snapshot `pread` and then fail; the Tier-2
        // fallback must still deliver every byte from the SAME descriptor.
        let handle = File::open(path_of(&file)).expect("open");
        let input = ImmutableInput::from_opened_file(handle, true).expect("acquire");
        assert_eq!(input.as_bytes(), payload.as_slice());
        assert_eq!(input.tier(), InputTier::Owned);
    }

    #[test]
    fn acquired_bytes_are_tied_to_the_opened_descriptor() {
        // Falsifies any reopen-per-tier implementation: open the ORIGINAL file, then
        // atomically replace its pathname with DIFFERENT bytes, and only then acquire
        // through the already-open descriptor. Acquisition has no path to re-resolve,
        // so it must return the original bytes; an implementation that reopened the
        // path would return the replacement bytes and fail here.
        let original: Vec<u8> = (0u8..=255).cycle().take(32 * 1024).collect();
        let original_file = temp_with(&original);
        let handle = File::open(path_of(&original_file)).expect("open original");

        // Atomically replace the pathname with a different file (a hostile writer)
        // BEFORE acquisition runs.
        let replacement: Vec<u8> = vec![0xAB; 48 * 1024];
        let replacement_file = temp_with(&replacement);
        std::fs::rename(replacement_file.path(), original_file.path()).expect("replace path");

        // Acquire through the already-open descriptor: the bytes are the original's,
        // never the replacement now living at the path.
        let input = ImmutableInput::from_opened_file(handle, false).expect("acquire");
        assert_eq!(input.as_bytes(), original.as_slice());
        assert_ne!(input.as_bytes(), replacement.as_slice());
    }
}

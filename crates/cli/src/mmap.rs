// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The kernel calls behind the immutable-input authority on Linux: a read-only,
//! whole-file memory mapping ([`Mmap`]), anonymous memory files
//! ([`memfd_create`]), and file seals ([`add_seals`], [`get_seals`]).
//!
//! Every failure is the operating system's own error, read back through
//! `std::io::Error::last_os_error`. This is the only module in the crate that
//! calls into the C library; outside it, `unsafe` appears only where a caller
//! discharges [`Mmap::map`]'s contract.
//!
//! None of this is a hot path: a pack is mapped once per acquisition, at the
//! cost of one `mmap` and one `munmap`.

use std::ffi::CStr;
use std::fmt;
use std::fs::File;
use std::io;
use std::ops::Deref;
use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
use std::ptr::NonNull;

/// A read-only mapping of a whole file, unmapped when dropped.
///
/// Dereferences to the file's bytes. A zero-length file yields an empty slice
/// without any mapping being made (the kernel refuses zero-length mappings).
pub struct Mmap {
    /// Start of the mapping; dangling (and never dereferenced) when `len == 0`.
    ptr: NonNull<u8>,
    /// Length of the mapping in bytes; `0` means nothing was mapped.
    len: usize,
}

// SAFETY: the mapping is read-only and, by `Mmap::map`'s contract, its backing
// object cannot change while it exists, so the bytes behave like an immutable
// owned buffer: handing the owner to another thread, or `&[u8]` views of it to
// several threads, cannot race. `munmap` may be called from any thread.
unsafe impl Send for Mmap {}

// SAFETY: see the `Send` impl — shared access only ever reads immutable bytes.
unsafe impl Sync for Mmap {}

impl Mmap {
    /// Maps the whole of `file` read-only.
    ///
    /// Makes one `fstat` for the length and one `mmap(PROT_READ, MAP_SHARED)`
    /// call; a zero-length file makes no mapping and answers an empty slice.
    ///
    /// # Safety
    ///
    /// For the whole lifetime of the returned value the object behind `file`
    /// must not shrink, grow, or be written by anyone (for example: it carries
    /// the seals `F_SEAL_SHRINK | F_SEAL_GROW | F_SEAL_WRITE`). A shrink would
    /// make reads of the mapping fault the process, and a write would change
    /// bytes behind an immutable borrow.
    ///
    /// # Errors
    ///
    /// Returns the operating system's error when the length cannot be read or
    /// the mapping cannot be made, and an [`io::ErrorKind::InvalidInput`] error
    /// when the length does not fit this host's address space.
    pub unsafe fn map(file: &File) -> io::Result<Self> {
        let len = usize::try_from(file.metadata()?.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "file length exceeds the address space",
            )
        })?;
        if len == 0 {
            return Ok(Self {
                ptr: NonNull::dangling(),
                len: 0,
            });
        }
        // SAFETY: a fresh read-only shared mapping at a kernel-chosen address
        // (`addr` is null), so no existing memory is replaced. `len` is non-zero
        // and `fd` is a live descriptor borrowed for the call. The result is
        // checked against `MAP_FAILED` before use.
        let addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if addr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let ptr = NonNull::new(addr.cast::<u8>()).ok_or_else(|| {
            io::Error::other("mmap answered a null address for a kernel-chosen mapping")
        })?;
        Ok(Self { ptr, len })
    }
}

impl Deref for Mmap {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // SAFETY: either `len == 0` and `ptr` is a well-aligned dangling pointer
        // (valid for an empty slice), or `ptr..ptr + len` is a live read-only
        // mapping owned by `self`, whose bytes cannot change while it exists
        // (`Mmap::map`'s contract). The borrow ends before `self` can be dropped.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        if self.len == 0 {
            return;
        }
        // SAFETY: `ptr`/`len` are exactly what `mmap` returned and was asked
        // for, the mapping is owned by `self` alone, and no borrow of it can
        // outlive `self`. It is unmapped once, here. A failure could only mean
        // an address `mmap` did not produce, and a destructor has nowhere to
        // report it, so the result is not inspected.
        unsafe {
            libc::munmap(self.ptr.as_ptr().cast::<libc::c_void>(), self.len);
        }
    }
}

impl fmt::Debug for Mmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mmap")
            .field("ptr", &self.ptr)
            .field("len", &self.len)
            .finish()
    }
}

/// Creates an anonymous memory file named `name` (a debugging label only) with
/// the `MFD_*` `flags`, answered as an owned read-write [`File`].
///
/// # Errors
///
/// Returns the operating system's error, for example when the kernel lacks
/// `memfd_create` or `flags` holds an unknown bit.
pub fn memfd_create(name: &CStr, flags: libc::c_uint) -> io::Result<File> {
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call;
    // the kernel only reads it.
    let fd = unsafe { libc::memfd_create(name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is a descriptor the kernel just created for this call and
    // nothing else owns, so the `OwnedFd` becomes its sole owner.
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(fd) }))
}

/// Adds the `F_SEAL_*` bits in `seals` to the seals of the inode behind
/// `file` (`fcntl(F_ADD_SEALS)`). Seals can never be removed.
///
/// # Errors
///
/// Returns the operating system's error: `EPERM` when the inode already carries
/// `F_SEAL_SEAL` or `file` is not open for writing, `EINVAL` when the file does
/// not support sealing, `EBUSY` when `F_SEAL_WRITE` meets a writable shared
/// mapping.
pub fn add_seals(file: &File, seals: libc::c_int) -> io::Result<()> {
    // SAFETY: `F_ADD_SEALS` takes one `int` argument and touches no memory of
    // ours; the descriptor is live for the call.
    let result = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, seals) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Answers the `F_SEAL_*` bits currently set on the inode behind `file`
/// (`fcntl(F_GET_SEALS)`).
///
/// # Errors
///
/// Returns the operating system's error; `EINVAL` means the file does not
/// support sealing (every ordinary disk file).
pub fn get_seals(file: &File) -> io::Result<libc::c_int> {
    // SAFETY: `F_GET_SEALS` takes no argument and touches no memory of ours;
    // the descriptor is live for the call.
    let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
    if seals < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(seals)
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    fn sealable(label: &CStr) -> File {
        memfd_create(label, libc::MFD_ALLOW_SEALING | libc::MFD_CLOEXEC).expect("memfd_create")
    }

    #[test]
    fn seals_read_back_equal_the_seals_added_step_by_step() {
        let file = sealable(c"purrdf-seal-steps");
        let initial = get_seals(&file).expect("a sealable memfd reports its seals");
        let steps = [
            libc::F_SEAL_SHRINK,
            libc::F_SEAL_GROW,
            libc::F_SEAL_WRITE,
            libc::F_SEAL_SEAL,
        ];
        for step in steps {
            assert_eq!(
                initial & step,
                0,
                "seal {step:#x} is not set before it is added"
            );
        }
        let mut expected = initial;
        for step in steps {
            add_seals(&file, step).expect("add a seal to an unlocked memfd");
            expected |= step;
            assert_eq!(get_seals(&file).expect("read seals back"), expected);
        }
    }

    #[test]
    fn adding_a_seal_after_the_seal_seal_is_refused_with_eperm() {
        let file = sealable(c"purrdf-seal-lock");
        // The neighbouring valid case: before the lock, an add succeeds.
        add_seals(&file, libc::F_SEAL_SHRINK).expect("an add before F_SEAL_SEAL succeeds");
        add_seals(&file, libc::F_SEAL_SEAL).expect("the lock itself can be added");
        let refused = add_seals(&file, libc::F_SEAL_GROW).expect_err("the seal set is locked");
        assert_eq!(refused.raw_os_error(), Some(libc::EPERM));
        let seals = get_seals(&file).expect("read seals back");
        assert_eq!(
            seals & libc::F_SEAL_GROW,
            0,
            "the refused seal was not applied"
        );
        assert_ne!(
            seals & libc::F_SEAL_SHRINK,
            0,
            "the earlier seal stays applied"
        );
    }

    #[test]
    fn a_regular_disk_file_reports_sealing_unsupported() {
        let file = purrdf_testkit::NamedTempFile::for_unit_test().expect("temp file");
        let handle = File::open(file.path()).expect("open temp file");
        let error = get_seals(&handle).expect_err("disk files cannot be sealed");
        assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    }

    #[test]
    fn mapped_bytes_equal_the_file_bytes() {
        let payload: Vec<u8> = (0..20_000_u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut file = sealable(c"purrdf-map-bytes");
        file.write_all(&payload).expect("fill memfd");
        add_seals(
            &file,
            libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE | libc::F_SEAL_SEAL,
        )
        .expect("seal memfd");
        // SAFETY: the memfd is ours and sealed against shrink, grow and write.
        let mapped = unsafe { Mmap::map(&file) }.expect("map sealed memfd");
        assert_eq!(&mapped[..], payload.as_slice());
        assert_eq!(mapped.len(), payload.len());
    }

    #[test]
    fn a_zero_length_file_maps_to_an_empty_slice() {
        let file = sealable(c"purrdf-map-empty");
        add_seals(
            &file,
            libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE | libc::F_SEAL_SEAL,
        )
        .expect("seal memfd");
        // SAFETY: the memfd is ours and sealed against shrink, grow and write.
        let mapped = unsafe { Mmap::map(&file) }.expect("an empty file maps");
        assert!(mapped.is_empty());
        assert_eq!(&mapped[..], &[] as &[u8]);
    }
}

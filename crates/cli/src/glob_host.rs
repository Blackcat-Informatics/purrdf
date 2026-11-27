// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Linux filesystem authority for the portable glob expansion.

use purrdf::glob::filesystem::{Directory, Entry, EntryKind, Source};
use std::ffi::CStr;
use std::io;
use std::os::fd::{AsRawFd as _, FromRawFd as _, IntoRawFd as _, OwnedFd};
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;
use std::ptr::NonNull;

pub(crate) struct Root(OwnedFd);

impl Root {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        open_directory(
            libc::AT_FDCWD,
            path.as_os_str().as_bytes(),
            libc::O_PATH,
            libc::RESOLVE_NO_SYMLINKS,
        )
        .map(Self)
    }
}

impl Source for Root {
    type Error = io::Error;
    type Cursor<'a> = Cursor;

    fn open_directory(&mut self, relative: &str) -> io::Result<Cursor> {
        let path = if relative.is_empty() { "." } else { relative };
        let fd = open_directory(
            self.0.as_raw_fd(),
            path.as_bytes(),
            libc::O_RDONLY,
            libc::RESOLVE_BENEATH | libc::RESOLVE_NO_SYMLINKS | libc::RESOLVE_NO_XDEV,
        )?;
        Cursor::open(fd)
    }
}

fn open_directory(
    anchor: libc::c_int,
    path: &[u8],
    flags: libc::c_int,
    resolve: u64,
) -> io::Result<OwnedFd> {
    let mut terminated = [0; libc::PATH_MAX as usize];
    let Some((nul, bytes)) = terminated
        .get_mut(..=path.len())
        .and_then(<[u8]>::split_last_mut)
    else {
        return Err(io::Error::from_raw_os_error(libc::ENAMETOOLONG));
    };
    *nul = 0;
    bytes.copy_from_slice(path);
    let path = CStr::from_bytes_with_nul(&terminated[..=path.len()])
        .map_err(|cause| io::Error::new(io::ErrorKind::InvalidInput, cause))?;
    // SAFETY: The kernel ABI structure contains only unsigned integer fields.
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags = (flags | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .cast_unsigned()
        .into();
    how.resolve = resolve;
    // SAFETY: The anchor is AT_FDCWD or a live descriptor. Both pointers
    // address initialized data and remain live for the call.
    let raw = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            anchor,
            path.as_ptr(),
            &raw const how,
            size_of::<libc::open_how>(),
        )
    };
    if raw == -1 {
        return Err(io::Error::last_os_error());
    }
    // A successful open syscall returns a newly owned c_int descriptor.
    let raw = libc::c_int::try_from(raw).expect("openat2 returns a c_int descriptor");
    // SAFETY: The successful syscall just returned this new descriptor.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}

pub(crate) struct Cursor(NonNull<libc::DIR>);

impl Cursor {
    fn open(fd: OwnedFd) -> io::Result<Self> {
        // SAFETY: fd is an open directory. On failure fdopendir does not own it.
        let dir = unsafe { libc::fdopendir(fd.as_raw_fd()) };
        let dir = NonNull::new(dir).ok_or_else(io::Error::last_os_error)?;
        let _ = fd.into_raw_fd();
        Ok(Self(dir))
    }

    fn classify(&self, name: &CStr, kind: u8) -> io::Result<EntryKind> {
        Ok(match kind {
            libc::DT_REG => EntryKind::File,
            libc::DT_DIR => EntryKind::Directory,
            libc::DT_LNK => EntryKind::Link,
            libc::DT_UNKNOWN => {
                let mut state = std::mem::MaybeUninit::<libc::stat>::uninit();
                // SAFETY: The directory is live, the name is the current
                // terminated entry, and fstatat writes into a valid stat slot.
                let result = unsafe {
                    libc::fstatat(
                        libc::dirfd(self.0.as_ptr()),
                        name.as_ptr(),
                        state.as_mut_ptr(),
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                };
                if result == -1 {
                    return Err(io::Error::last_os_error());
                }
                // SAFETY: A successful fstatat initializes the stat slot.
                match unsafe { state.assume_init() }.st_mode & libc::S_IFMT {
                    libc::S_IFREG => EntryKind::File,
                    libc::S_IFDIR => EntryKind::Directory,
                    libc::S_IFLNK => EntryKind::Link,
                    _ => EntryKind::Other,
                }
            }
            _ => EntryKind::Other,
        })
    }
}

impl Directory for Cursor {
    type Error = io::Error;

    fn next_entry(&mut self) -> io::Result<Option<Entry<'_>>> {
        loop {
            // SAFETY: This cursor uniquely owns a live stream. Its previous
            // entry borrow ended before this mutable call. errno belongs to
            // the current thread and is cleared to distinguish EOF from error.
            let (entry, errno) = unsafe {
                let errno = libc::__errno_location();
                errno.write(0);
                let entry = libc::readdir(self.0.as_ptr());
                (entry, errno.read())
            };
            let Some(entry) = NonNull::new(entry) else {
                return if errno == 0 {
                    Ok(None)
                } else {
                    Err(io::Error::from_raw_os_error(errno))
                };
            };
            // SAFETY: readdir returns a live fixed prefix and a terminated
            // name until the next read or close of this stream. Records are
            // variable sized, so never borrow the complete Rust dirent.
            let (name, kind) = unsafe {
                let entry = entry.as_ptr().cast::<u8>();
                let kind = entry.add(std::mem::offset_of!(libc::dirent, d_type)).read();
                let name = entry.add(std::mem::offset_of!(libc::dirent, d_name));
                (CStr::from_ptr(name.cast()), kind)
            };
            if matches!(name.to_bytes(), b"." | b"..") {
                continue;
            }
            let kind = self.classify(name, kind)?;
            let name = name
                .to_str()
                .map_err(|cause| io::Error::new(io::ErrorKind::InvalidData, cause))?;
            return Ok(Some(Entry { name, kind }));
        }
    }
}

impl Drop for Cursor {
    fn drop(&mut self) {
        // SAFETY: This cursor owns the stream once and holds no live borrows.
        unsafe { libc::closedir(self.0.as_ptr()) };
    }
}

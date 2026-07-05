// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tar stream import into files-profile-v2 GTS archives.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::files::{FileBlobBytes, FileEntry, FileEntryKind, pack_entries_v2_with_blob_bytes};
use crate::files::{FileBlobRange, digest_blob_range, pack_entries_v2_with_blob_ranges};
use crate::tar::{
    RawTarEntry, SeekTarEntry, TarError, index_uncompressed_tar, read_uncompressed_tar,
};
use crate::writer::digest_string;

/// Options for [`from_tar`].
#[derive(Clone, Debug, Default)]
pub struct FromTarOptions {
    pub allow_symlinks: bool,
    pub allow_special: bool,
    pub owner: bool,
    /// Optional source label used for compression detection by extension.
    pub source_name: Option<String>,
}

/// Read a tar stream, optionally decompress it, and author a files-profile-v2 GTS archive.
pub fn from_tar<R: Read>(reader: R, options: &FromTarOptions) -> Result<Vec<u8>, TarError> {
    let mut out = Vec::new();
    from_tar_to_writer(reader, &mut out, options)?;
    Ok(out)
}

/// Read a seekable tar stream and author a files-profile-v2 GTS archive.
///
/// Uncompressed tar is indexed and replayed by byte range, so regular-file
/// payloads are not retained in memory. Compressed tar streams fall back to the
/// buffered path because the decompressed member bytes are not seekable.
pub fn from_seekable_tar<R: Read + Seek>(
    reader: &mut R,
    options: &FromTarOptions,
) -> Result<Vec<u8>, TarError> {
    let mut out = Vec::new();
    from_seekable_tar_to_writer(reader, &mut out, options)?;
    Ok(out)
}

/// Seekable counterpart to [`from_tar_to_writer`].
pub fn from_seekable_tar_to_writer<R: Read + Seek, W: Write>(
    reader: &mut R,
    writer: W,
    options: &FromTarOptions,
) -> Result<(), TarError> {
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|err| TarError::new(format!("seek tar input: {err}")))?;
    let mut prefix = [0_u8; 4];
    let n = reader
        .read(&mut prefix)
        .map_err(|err| TarError::new(format!("read tar prefix: {err}")))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|err| TarError::new(format!("seek tar input: {err}")))?;
    if detect_compression(&prefix[..n], options.source_name.as_deref()).is_some() {
        return from_tar_to_writer(reader, writer, options);
    }

    let raw_entries = index_uncompressed_tar(reader)?;
    let mut entries = Vec::with_capacity(raw_entries.len());
    let mut blobs: BTreeMap<String, FileBlobRange<'_>> = BTreeMap::new();
    for entry in &raw_entries {
        let source = FileBlobRange {
            offset: entry.data_offset,
            size: entry.data_size,
            media_type: None,
            representation: None,
        };
        let digest = (entry.kind == FileEntryKind::File)
            .then(|| digest_blob_range(reader, source))
            .transpose()
            .map_err(TarError::new)?;
        let file_entry = file_entry_from_seek_tar(entry, digest.clone(), options)?;
        if let Some(digest) = digest {
            blobs.entry(digest).or_insert(source);
        }
        entries.push(file_entry);
    }
    pack_entries_v2_with_blob_ranges(&entries, reader, &blobs, writer).map_err(TarError::new)
}

/// Read a tar stream and author files-profile-v2 GTS bytes to `writer`.
pub fn from_tar_to_writer<R: Read, W: Write>(
    mut reader: R,
    writer: W,
    options: &FromTarOptions,
) -> Result<(), TarError> {
    let mut input = Vec::new();
    reader
        .read_to_end(&mut input)
        .map_err(|err| TarError::new(format!("read tar input: {err}")))?;
    write_tar_data_as_gts(&input, writer, options)
}

fn write_tar_data_as_gts<W: Write>(
    data: &[u8],
    writer: W,
    options: &FromTarOptions,
) -> Result<(), TarError> {
    let tar = decompress_input(data, options.source_name.as_deref())?;
    let raw_entries = read_uncompressed_tar(tar.as_ref())?;
    let mut entries = Vec::with_capacity(raw_entries.len());
    let mut blobs: BTreeMap<String, FileBlobBytes<'_>> = BTreeMap::new();
    for entry in &raw_entries {
        let file_entry = file_entry_from_tar(entry, options)?;
        if entry.kind == FileEntryKind::File {
            let digest = file_entry
                .digest
                .clone()
                .ok_or_else(|| TarError::new(format!("file entry {} has no digest", entry.path)))?;
            blobs.entry(digest).or_insert(FileBlobBytes {
                data: entry.data,
                media_type: None,
                representation: None,
            });
        }
        entries.push(file_entry);
    }
    pack_entries_v2_with_blob_bytes(&entries, &blobs, writer).map_err(TarError::new)
}

/// Author a files-profile-v2 GTS archive from bytes containing tar, tar.gz, or tar.zst.
pub fn from_tar_bytes(data: &[u8], options: &FromTarOptions) -> Result<Vec<u8>, TarError> {
    let mut out = Vec::new();
    write_tar_data_as_gts(data, &mut out, options)?;
    Ok(out)
}

fn decompress_input<'a>(
    data: &'a [u8],
    source_name: Option<&str>,
) -> Result<Cow<'a, [u8]>, TarError> {
    match detect_compression(data, source_name) {
        None => Ok(Cow::Borrowed(data)),
        Some("gzip") => {
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(data)
                .read_to_end(&mut out)
                .map_err(|err| TarError::new(format!("gzip decode tar input: {err}")))?;
            Ok(Cow::Owned(out))
        }
        Some("zstd") => {
            let mut out = Vec::new();
            structured_zstd::decoding::StreamingDecoder::new(data)
                .map_err(|err| TarError::new(format!("zstd decode tar input: {err}")))?
                .read_to_end(&mut out)
                .map_err(|err| TarError::new(format!("zstd decode tar input: {err}")))?;
            Ok(Cow::Owned(out))
        }
        Some(other) => Err(TarError::new(format!(
            "unsupported tar compression: {other}"
        ))),
    }
}

// Matching stays byte-exact on the (already lowercased) name; multi-part suffixes
// like ".tar.gz" cannot be expressed via Path::extension.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn detect_compression(data: &[u8], source_name: Option<&str>) -> Option<&'static str> {
    if data.starts_with(&[0x1f, 0x8b]) {
        return Some("gzip");
    }
    if data.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return Some("zstd");
    }
    let name = source_name?.to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".gz") {
        Some("gzip")
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") || name.ends_with(".zst") {
        Some("zstd")
    } else {
        None
    }
}

fn file_entry_from_tar(
    entry: &RawTarEntry<'_>,
    options: &FromTarOptions,
) -> Result<FileEntry, TarError> {
    if entry.path.is_empty() {
        return Err(TarError::new("empty tar path"));
    }
    match entry.kind {
        FileEntryKind::Symlink => {
            if !options.allow_symlinks {
                return Err(TarError::new(format!(
                    "refusing symlink entry {}: use --allow-symlinks",
                    entry.path
                )));
            }
            require_link_target(entry)?;
        }
        FileEntryKind::Hardlink => {
            if !options.allow_symlinks {
                return Err(TarError::new(format!(
                    "refusing hardlink entry {}: use --allow-symlinks",
                    entry.path
                )));
            }
            require_link_target(entry)?;
        }
        FileEntryKind::Fifo | FileEntryKind::CharDev | FileEntryKind::BlockDev => {
            if !options.allow_special {
                return Err(TarError::new(format!(
                    "refusing {} entry {}: use --allow-special",
                    entry.kind.as_str(),
                    entry.path
                )));
            }
        }
        FileEntryKind::Socket => {
            return Err(TarError::new(format!(
                "unsupported tar socket entry {}",
                entry.path
            )));
        }
        FileEntryKind::File | FileEntryKind::Directory => {}
    }
    Ok(FileEntry {
        path: entry.path.clone(),
        kind: entry.kind,
        digest: (entry.kind == FileEntryKind::File).then(|| digest_string(entry.data)),
        size: (entry.kind == FileEntryKind::File).then_some(entry.data.len() as u64),
        mode: entry.mode,
        modified: entry.modified_seconds.map(format_mtime).transpose()?,
        media_type: None,
        link_target: entry.link_target.clone(),
        uid: options.owner.then_some(entry.uid).flatten(),
        gid: options.owner.then_some(entry.gid).flatten(),
        user_name: options.owner.then_some(entry.user_name.clone()).flatten(),
        group_name: options.owner.then_some(entry.group_name.clone()).flatten(),
        dev_major: entry.dev_major,
        dev_minor: entry.dev_minor,
        xattrs: Vec::new(),
        pax_records: entry.pax_records.clone(),
        data: None,
    })
}

fn file_entry_from_seek_tar(
    entry: &SeekTarEntry,
    digest: Option<String>,
    options: &FromTarOptions,
) -> Result<FileEntry, TarError> {
    if entry.path.is_empty() {
        return Err(TarError::new("empty tar path"));
    }
    match entry.kind {
        FileEntryKind::Symlink => {
            if !options.allow_symlinks {
                return Err(TarError::new(format!(
                    "refusing symlink entry {}: use --allow-symlinks",
                    entry.path
                )));
            }
            require_seek_link_target(entry)?;
        }
        FileEntryKind::Hardlink => {
            if !options.allow_symlinks {
                return Err(TarError::new(format!(
                    "refusing hardlink entry {}: use --allow-symlinks",
                    entry.path
                )));
            }
            require_seek_link_target(entry)?;
        }
        FileEntryKind::Fifo | FileEntryKind::CharDev | FileEntryKind::BlockDev => {
            if !options.allow_special {
                return Err(TarError::new(format!(
                    "refusing {} entry {}: use --allow-special",
                    entry.kind.as_str(),
                    entry.path
                )));
            }
        }
        FileEntryKind::Socket => {
            return Err(TarError::new(format!(
                "unsupported tar socket entry {}",
                entry.path
            )));
        }
        FileEntryKind::File | FileEntryKind::Directory => {}
    }
    Ok(FileEntry {
        path: entry.path.clone(),
        kind: entry.kind,
        digest,
        size: (entry.kind == FileEntryKind::File).then_some(entry.data_size),
        mode: entry.mode,
        modified: entry.modified_seconds.map(format_mtime).transpose()?,
        media_type: None,
        link_target: entry.link_target.clone(),
        uid: options.owner.then_some(entry.uid).flatten(),
        gid: options.owner.then_some(entry.gid).flatten(),
        user_name: options.owner.then_some(entry.user_name.clone()).flatten(),
        group_name: options.owner.then_some(entry.group_name.clone()).flatten(),
        dev_major: entry.dev_major,
        dev_minor: entry.dev_minor,
        xattrs: Vec::new(),
        pax_records: entry.pax_records.clone(),
        data: None,
    })
}

fn require_link_target(entry: &RawTarEntry<'_>) -> Result<(), TarError> {
    if entry.link_target.as_deref().is_none_or(str::is_empty) {
        return Err(TarError::new(format!(
            "link entry {} has no link target",
            entry.path
        )));
    }
    Ok(())
}

fn require_seek_link_target(entry: &SeekTarEntry) -> Result<(), TarError> {
    if entry.link_target.as_deref().is_none_or(str::is_empty) {
        return Err(TarError::new(format!(
            "link entry {} has no link target",
            entry.path
        )));
    }
    Ok(())
}

fn format_mtime(seconds: u64) -> Result<String, TarError> {
    let dt = time::OffsetDateTime::from_unix_timestamp(seconds as i64)
        .map_err(|err| TarError::new(format!("invalid tar mtime {seconds}: {err}")))?;
    dt.format(&time::format_description::well_known::Rfc3339)
        .map(|text| text.replace("+00:00", "Z"))
        .map_err(|err| TarError::new(format!("format tar mtime {seconds}: {err}")))
}

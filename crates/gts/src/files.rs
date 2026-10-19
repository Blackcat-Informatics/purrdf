// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Files-profile pack/unpack/diff logic for GTS archives (§13.2, §14.2).

mod error;
mod fs_error;
pub use error::FileError;
pub use fs_error::FsRefusal;

use std::collections::BTreeMap;

use crate::{FastMap, FastSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use purrdf_lex::cbor::Value;
use purrdf_lex::cbor::head::{BYTES, MAP, write_bytes, write_head, write_text};

use crate::model::{Graph, Quad, Term, TermKind};
use crate::wire::digest_label;
use crate::writer::{Writer, WriterOptions, digest_string};

const FILES_NS: &str = "https://w3id.org/gts/files#";
const FILE_ENTRY: &str = "https://w3id.org/gts/files#FileEntry";
const FILES_PATH: &str = "https://w3id.org/gts/files#path";
const FILES_DIGEST: &str = "https://w3id.org/gts/files#digest";
const FILES_SIZE: &str = "https://w3id.org/gts/files#size";
const FILES_MODE: &str = "https://w3id.org/gts/files#mode";
const FILES_MODIFIED: &str = "https://w3id.org/gts/files#modified";
const FILES_MEDIA_TYPE: &str = "https://w3id.org/gts/files#mediaType";
const FILES_TYPE: &str = "https://w3id.org/gts/files#type";
const FILES_LINK_TARGET: &str = "https://w3id.org/gts/files#linkTarget";
const FILES_UID: &str = "https://w3id.org/gts/files#uid";
const FILES_GID: &str = "https://w3id.org/gts/files#gid";
const FILES_USER_NAME: &str = "https://w3id.org/gts/files#userName";
const FILES_GROUP_NAME: &str = "https://w3id.org/gts/files#groupName";
const FILES_DEV_MAJOR: &str = "https://w3id.org/gts/files#devMajor";
const FILES_DEV_MINOR: &str = "https://w3id.org/gts/files#devMinor";
const FILES_XATTR: &str = "https://w3id.org/gts/files#xattr";
const FILES_XATTR_NAME: &str = "https://w3id.org/gts/files#xattrName";
const FILES_XATTR_VALUE: &str = "https://w3id.org/gts/files#xattrValue";
const FILES_PAX_RECORD: &str = "https://w3id.org/gts/files#paxRecord";
const FILES_PAX_KEY: &str = "https://w3id.org/gts/files#paxKey";
const FILES_PAX_VALUE: &str = "https://w3id.org/gts/files#paxValue";
use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;
use purrdf_xsd::datatype::{XSD_DATE_TIME as XSD_DATETIME, XSD_INTEGER};
const STREAM_CHUNK_SIZE: usize = 128 * 1024;

type InlineBlobMap<'a> = BTreeMap<String, (&'a [u8], Option<&'a str>)>;

/// files-profile v2 entry kind. Absence of `files:type` is read as [`Self::File`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileEntryKind {
    /// A regular file (the default when `files:type` is absent).
    #[default]
    File,
    /// A directory.
    Directory,
    /// A symbolic link; the target is carried in `files:linkTarget`.
    Symlink,
    /// A hard link; the target is carried in `files:linkTarget`.
    Hardlink,
    /// A FIFO (named pipe).
    Fifo,
    /// A character device; major/minor numbers ride in `files:devMajor`/`files:devMinor`.
    CharDev,
    /// A block device; major/minor numbers ride in `files:devMajor`/`files:devMinor`.
    BlockDev,
    /// A Unix domain socket.
    Socket,
}

impl FileEntryKind {
    /// The canonical `files:type` lexical value for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Symlink => "symlink",
            Self::Hardlink => "hardlink",
            Self::Fifo => "fifo",
            Self::CharDev => "chardev",
            Self::BlockDev => "blockdev",
            Self::Socket => "socket",
        }
    }

    /// Parse a `files:type` lexical value.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is not a known files-profile entry kind.
    pub fn parse(value: &str) -> Result<Self, FileError> {
        match value {
            "file" => Ok(Self::File),
            "directory" => Ok(Self::Directory),
            "symlink" => Ok(Self::Symlink),
            "hardlink" => Ok(Self::Hardlink),
            "fifo" => Ok(Self::Fifo),
            "chardev" => Ok(Self::CharDev),
            "blockdev" => Ok(Self::BlockDev),
            "socket" => Ok(Self::Socket),
            other => Err(FileError::Kind {
                actual: other.to_owned(),
            }),
        }
    }
}

/// One files-profile v2 extended attribute row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileXattr {
    /// Extended-attribute name (e.g. `user.mime_type`).
    pub name: String,
    /// Base64 lexical value of the attribute bytes.
    pub value: String,
}

/// One verbatim PAX escape-hatch key/value row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePaxRecord {
    /// PAX record key, carried verbatim.
    pub key: String,
    /// PAX record value, carried verbatim.
    pub value: String,
}

/// Typed files-profile entry used by the Rust v2 writer/reader.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileEntry {
    /// Archive-relative entry path (`files:path`), unique per archive.
    pub path: String,
    /// Entry kind (`files:type`); absent on the wire means [`FileEntryKind::File`].
    pub kind: FileEntryKind,
    /// BLAKE3 content digest of a regular file's blob (`files:digest`).
    pub digest: Option<String>,
    /// Decoded payload size in bytes (`files:size`).
    pub size: Option<u64>,
    /// Unix permission bits (`files:mode`).
    pub mode: Option<u32>,
    /// Modification timestamp as an `xsd:dateTime` lexical value (`files:modified`).
    pub modified: Option<String>,
    /// Media type of a regular file's payload (`files:mediaType`).
    pub media_type: Option<String>,
    /// Symlink or hardlink target path (`files:linkTarget`).
    pub link_target: Option<String>,
    /// Numeric owner user id (`files:uid`).
    pub uid: Option<u64>,
    /// Numeric owner group id (`files:gid`).
    pub gid: Option<u64>,
    /// Owner user name (`files:userName`).
    pub user_name: Option<String>,
    /// Owner group name (`files:groupName`).
    pub group_name: Option<String>,
    /// Device major number for character/block devices (`files:devMajor`).
    pub dev_major: Option<u64>,
    /// Device minor number for character/block devices (`files:devMinor`).
    pub dev_minor: Option<u64>,
    /// Extended-attribute rows (`files:xattr`).
    pub xattrs: Vec<FileXattr>,
    /// Verbatim PAX escape-hatch rows (`files:paxRecord`).
    pub pax_records: Vec<FilePaxRecord>,
    /// Optional inline payload bytes for regular-file authoring.
    pub data: Option<Vec<u8>>,
}

/// A regular-file blob source for bounded files-profile authoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileBlobSource {
    /// Filesystem path the blob bytes are streamed from.
    pub path: PathBuf,
    /// Decoded payload size in bytes.
    pub size: u64,
    /// Media type recorded with the blob frame.
    pub media_type: Option<String>,
    /// Transform-chain representation (e.g. `"gzip"`) to apply when writing.
    pub representation: Option<String>,
}

/// Borrowed regular-file blob bytes for bounded files-profile authoring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileBlobBytes<'a> {
    /// Decoded blob payload bytes.
    pub data: &'a [u8],
    /// Media type recorded with the blob frame.
    pub media_type: Option<&'a str>,
    /// Transform-chain representation (e.g. `"gzip"`) to apply when writing.
    pub representation: Option<&'a str>,
}

/// Borrowed regular-file blob range inside a seekable source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileBlobRange<'a> {
    /// Byte offset of the payload within the seekable source.
    pub offset: u64,
    /// Payload size in bytes at `offset`.
    pub size: u64,
    /// Media type recorded with the blob frame.
    pub media_type: Option<&'a str>,
    /// Transform-chain representation (e.g. `"gzip"`) to apply when writing.
    pub representation: Option<&'a str>,
}

/// Safety policy for materializing files-profile archives.
// Deliberate flag set: each bool is an independent opt-in safety toggle, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnpackOptions {
    /// Extract blobs hidden by suppression frames.
    pub include_suppressed: bool,
    /// Create symlink and hardlink entries after validating that they stay
    /// within the destination tree.
    pub allow_symlinks: bool,
    /// Recreate fifo/device/socket nodes where the host platform supports it.
    pub allow_special: bool,
    /// Restore numeric uid/gid values. Requires platform support and usually
    /// elevated privileges.
    pub same_owner: bool,
    /// Preserve setuid, setgid, and sticky bits. These are stripped by default.
    pub preserve_setid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TermKey {
    kind: TermKind,
    value: String,
    datatype: Option<String>,
}

#[derive(Default)]
struct TermBuilder {
    ids: FastMap<TermKey, usize>,
    terms: Vec<Term>,
    quads: Vec<Quad>,
}

impl TermBuilder {
    fn atom(&mut self, kind: TermKind, value: String, datatype: Option<&str>) -> usize {
        let key = TermKey {
            kind,
            value: value.clone(),
            datatype: datatype.map(str::to_string),
        };
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let datatype_id = if kind == TermKind::Literal {
            datatype.map(|iri| self.atom(TermKind::Iri, iri.to_string(), None))
        } else {
            None
        };
        let id = self.terms.len();
        self.terms.push(Term {
            kind,
            value: Some(value),
            datatype: datatype_id,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        });
        self.ids.insert(key, id);
        id
    }

    fn iri(&mut self, value: &str) -> usize {
        self.atom(TermKind::Iri, value.to_string(), None)
    }

    fn bnode(&mut self, label: &str) -> usize {
        self.atom(TermKind::Bnode, label.to_string(), None)
    }

    fn literal(&mut self, value: &str, datatype: Option<&str>) -> usize {
        self.atom(TermKind::Literal, value.to_string(), datatype)
    }

    fn quad_lit(&mut self, subject: usize, predicate: &str, value: &str, datatype: Option<&str>) {
        let p = self.iri(predicate);
        let o = self.literal(value, datatype);
        self.quads.push((subject, p, o, None));
    }
}

fn safe_archive_path(name: &str) -> Result<(), FileError> {
    if name.is_empty() {
        return Err(FileError::EmptyPath);
    }
    let normalized = name.replace('\\', "/");
    let drive_relative =
        name.len() >= 2 && name.as_bytes()[1] == b':' && name.as_bytes()[0].is_ascii_alphabetic();
    if drive_relative || normalized.starts_with('/') {
        return Err(FileError::AbsolutePath {
            path: name.to_owned(),
        });
    }
    let parts: Vec<&str> = normalized.split('/').collect();
    for part in &parts {
        if *part == ".." {
            return Err(FileError::TraversalPath {
                path: name.to_owned(),
            });
        }
    }
    if name.contains('\\') {
        return Err(FileError::BackslashPath {
            path: name.to_owned(),
        });
    }
    if parts.iter().any(|part| part.is_empty() || *part == ".") {
        return Err(FileError::ComponentPath {
            path: name.to_owned(),
        });
    }
    Ok(())
}

fn to_posix_path(path: &Path) -> Result<String, FileError> {
    let mut parts = Vec::new();
    for c in path.components() {
        let s = c
            .as_os_str()
            .to_str()
            .ok_or_else(|| FsRefusal::NonUtf8Path {
                path: path.to_path_buf(),
            })?;
        parts.push(s.to_string());
    }
    Ok(parts.join("/"))
}

fn walk_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>, FileError> {
    fn recurse(out: &mut Vec<PathBuf>, dir: &Path) -> Result<(), std::io::Error> {
        let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let ftype = entry.file_type()?;
            if ftype.is_symlink() {
                return Err(purrdf_hash::error::io_error(
                    std::io::ErrorKind::InvalidInput,
                    FsRefusal::Symlink { path },
                ));
            }
            if ftype.is_dir() {
                recurse(out, &path)?;
            } else if ftype.is_file() {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    recurse(&mut out, dir).map_err(|cause| FileError::Io {
        operation: "walk",
        path: Some(dir.to_path_buf()),
        cause: cause.into(),
    })?;
    Ok(out)
}

fn resolve_sources(sources: &[&Path]) -> Result<Vec<(PathBuf, String)>, FileError> {
    let mut entries: Vec<(PathBuf, String)> = Vec::new();
    let mut seen: FastSet<String> = FastSet::default();
    for src in sources {
        let meta = fs::symlink_metadata(src).map_err(|cause| FileError::Io {
            operation: "",
            path: Some(src.to_path_buf()),
            cause: cause.into(),
        })?;
        if meta.file_type().is_symlink() {
            return Err(FsRefusal::Symlink {
                path: src.to_path_buf(),
            }
            .into());
        }
        if meta.is_file() {
            let name = src
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| FsRefusal::InvalidSourceName {
                    path: src.to_path_buf(),
                })?
                .to_string();
            safe_archive_path(&name)?;
            if !seen.insert(name.clone()) {
                return Err(FileError::DuplicateArchivePath { path: name });
            }
            entries.push((src.to_path_buf(), name));
        } else if meta.is_dir() {
            let files = walk_dir_sorted(src)?;
            for fspath in files {
                let relpath = to_posix_path(fspath.strip_prefix(src).map_err(|_| {
                    FsRefusal::OutsideSource {
                        path: fspath.clone(),
                        base: src.to_path_buf(),
                    }
                })?)?;
                safe_archive_path(&relpath)?;
                if !seen.insert(relpath.clone()) {
                    return Err(FileError::DuplicateArchivePath { path: relpath });
                }
                entries.push((fspath, relpath));
            }
        } else {
            return Err(FsRefusal::UnsupportedSource {
                path: src.to_path_buf(),
            }
            .into());
        }
    }
    entries.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(entries)
}

/// Media type for a file extension (without its dot), matched case-insensitively.
///
/// The workspace's one extension-to-media-type table: GTS file and tar ingest, the
/// slice catalog, the slice-artifact blob rows of the Python binding and the
/// slice RDF query loader all call it. The RDF entries name the media types the
/// native codecs register (`text/turtle`, `application/n-triples`,
/// `application/n-quads`, `application/trig`, `application/rdf+xml`); an extension it does not know
/// maps to `application/octet-stream`.
#[must_use]
pub fn media_type_for_extension(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "xml" => "application/xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "ttl" => "text/turtle",
        "nt" => "application/n-triples",
        "nq" => "application/n-quads",
        "trig" => "application/trig",
        "rdf" => "application/rdf+xml",
        "sparql" | "rq" => "application/sparql-query",
        "md" => "text/markdown",
        "yaml" | "yml" | "cff" => "application/yaml",
        _ => "application/octet-stream",
    }
}

/// Media type for a path, from its final extension; see [`media_type_for_extension`].
/// A path with no extension maps to `application/octet-stream`.
#[must_use]
pub fn media_type_for_path(path: &Path) -> &'static str {
    path.extension()
        .and_then(|e| e.to_str())
        .map_or("application/octet-stream", media_type_for_extension)
}

/// [`media_type_for_path`] restricted to an accepted list: the guessed media type where
/// `accepted` names it, `fallback` for every other path.
///
/// A loader that feeds one family of codecs (the native RDF syntaxes) keeps the shared
/// table's answer where it names a syntax the loader can read and routes the rest to the
/// fallback, so the loader holds its list and never a second extension table.
#[must_use]
pub fn media_type_for_path_among(
    path: &Path,
    accepted: &[&'static str],
    fallback: &'static str,
) -> &'static str {
    let guessed = media_type_for_path(path);
    accepted
        .iter()
        .copied()
        .find(|media_type| *media_type == guessed)
        .unwrap_or(fallback)
}

fn write_blob_pub_map<W: Write>(
    writer: &mut W,
    digest: &str,
    media_type: Option<&str>,
    representation: Option<&str>,
) -> std::io::Result<()> {
    let len = 1 + u64::from(media_type.is_some()) + u64::from(representation.is_some());
    write_head(writer, MAP, len)?;
    if let Some(media_type) = media_type {
        write_text(writer, "mt")?;
        write_text(writer, media_type)?;
    }
    if let Some(representation) = representation {
        write_text(writer, "rep")?;
        write_text(writer, representation)?;
    }
    write_text(writer, "digest")?;
    write_text(writer, digest)
}

fn copy_counted_and_hash<R: Read, W: Write>(
    mut reader: R,
    writer: &mut W,
    expected_size: u64,
) -> std::io::Result<(String, u64)> {
    let mut digest = purrdf_hash::blake3::Hasher::new();
    let mut written = 0_u64;
    let mut buf = vec![0_u8; STREAM_CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        digest.update(&buf[..n]);
        written += n as u64;
    }
    if written != expected_size {
        return Err(purrdf_hash::error::io_error(
            std::io::ErrorKind::UnexpectedEof,
            FileError::BlobSize {
                expected: expected_size,
                actual: written,
            },
        ));
    }
    Ok((digest_label(digest.finalize().as_bytes()), written))
}

fn write_blob_preimage<R: Read, W: Write>(
    writer: &mut W,
    reader: R,
    size: u64,
    media_type: Option<&str>,
    representation: Option<&str>,
    prev: &[u8],
) -> std::io::Result<String> {
    write_head(writer, MAP, 4)?;
    write_text(writer, "d")?;
    write_head(writer, BYTES, size)?;
    let (digest, _) = copy_counted_and_hash(reader, writer, size)?;
    write_text(writer, "t")?;
    write_text(writer, "blob")?;
    write_text(writer, "pub")?;
    write_blob_pub_map(writer, &digest, media_type, representation)?;
    write_text(writer, "prev")?;
    write_bytes(writer, prev)?;
    Ok(digest)
}

struct BlobFrameMeta<'a> {
    size: u64,
    id: &'a [u8],
    digest: &'a str,
    media_type: Option<&'a str>,
    representation: Option<&'a str>,
    prev: &'a [u8],
}

fn write_blob_frame<R: Read, W: Write>(
    writer: &mut W,
    reader: R,
    meta: &BlobFrameMeta<'_>,
) -> std::io::Result<()> {
    write_head(writer, MAP, 5)?;
    write_text(writer, "d")?;
    write_head(writer, BYTES, meta.size)?;
    let (observed_digest, _) = copy_counted_and_hash(reader, writer, meta.size)?;
    if observed_digest != meta.digest {
        return Err(purrdf_hash::error::io_error(
            std::io::ErrorKind::InvalidData,
            FileError::BlobChangedDigest {
                expected: meta.digest.to_owned(),
                actual: observed_digest,
            },
        ));
    }
    write_text(writer, "t")?;
    write_text(writer, "blob")?;
    write_text(writer, "id")?;
    write_bytes(writer, meta.id)?;
    write_text(writer, "pub")?;
    write_blob_pub_map(writer, meta.digest, meta.media_type, meta.representation)?;
    write_text(writer, "prev")?;
    write_bytes(writer, meta.prev)
}

fn append_blob_path<W: Write>(
    writer: &mut W,
    prev: &mut Vec<u8>,
    source: &FileBlobSource,
    expected_digest: &str,
) -> Result<(), FileError> {
    let media_type = source.media_type.as_deref();
    let representation = source.representation.as_deref();
    let mut file = fs::File::open(&source.path).map_err(|cause| FileError::Io {
        operation: "read",
        path: Some(source.path.clone()),
        cause: cause.into(),
    })?;
    let mut hasher = purrdf_hash::blake3::Hasher::new();
    let digest = {
        write_blob_preimage(
            &mut hasher,
            &mut file,
            source.size,
            media_type,
            representation,
            prev,
        )
        .map_err(|cause| FileError::Io {
            operation: "hash blob",
            path: Some(source.path.clone()),
            cause: cause.into(),
        })?
    };
    if digest != expected_digest {
        return Err(FileError::BlobDigest {
            kind: "file",
            path: Some(source.path.clone()),
            expected: expected_digest.to_owned(),
            actual: digest,
        });
    }
    let id = hasher.finalize().as_bytes().to_vec();
    file.rewind().map_err(|cause| FileError::Io {
        operation: "seek",
        path: Some(source.path.clone()),
        cause: cause.into(),
    })?;
    let meta = BlobFrameMeta {
        size: source.size,
        id: &id,
        digest: &digest,
        media_type,
        representation,
        prev,
    };
    write_blob_frame(writer, &mut file, &meta).map_err(|cause| FileError::Io {
        operation: "write blob",
        path: Some(source.path.clone()),
        cause: cause.into(),
    })?;
    *prev = id;
    Ok(())
}

fn append_blob_bytes<W: Write>(
    writer: &mut W,
    prev: &mut Vec<u8>,
    source: FileBlobBytes<'_>,
    expected_digest: &str,
) -> Result<(), FileError> {
    let mut hasher = purrdf_hash::blake3::Hasher::new();
    let digest = {
        write_blob_preimage(
            &mut hasher,
            source.data,
            source.data.len() as u64,
            source.media_type,
            source.representation,
            prev,
        )
        .map_err(|cause| FileError::Io {
            operation: "hash inline blob",
            path: None,
            cause: cause.into(),
        })?
    };
    if digest != expected_digest {
        return Err(FileError::BlobDigest {
            kind: "inline blob",
            path: None,
            expected: expected_digest.to_owned(),
            actual: digest,
        });
    }
    let id = hasher.finalize().as_bytes().to_vec();
    let meta = BlobFrameMeta {
        size: source.data.len() as u64,
        id: &id,
        digest: &digest,
        media_type: source.media_type,
        representation: source.representation,
        prev,
    };
    write_blob_frame(writer, source.data, &meta).map_err(|cause| FileError::Io {
        operation: "write inline blob",
        path: None,
        cause: cause.into(),
    })?;
    *prev = id;
    Ok(())
}

fn append_blob_range<R: Read + Seek, W: Write>(
    writer: &mut W,
    prev: &mut Vec<u8>,
    reader: &mut R,
    source: FileBlobRange<'_>,
    expected_digest: &str,
) -> Result<(), FileError> {
    let mut hasher = purrdf_hash::blake3::Hasher::new();
    let digest = {
        reader
            .seek(SeekFrom::Start(source.offset))
            .map_err(|cause| FileError::Io {
                operation: "seek inline blob",
                path: None,
                cause: cause.into(),
            })?;
        write_blob_preimage(
            &mut hasher,
            reader.take(source.size),
            source.size,
            source.media_type,
            source.representation,
            prev,
        )
        .map_err(|cause| FileError::Io {
            operation: "hash inline blob range",
            path: None,
            cause: cause.into(),
        })?
    };
    if digest != expected_digest {
        return Err(FileError::BlobDigest {
            kind: "inline blob range",
            path: None,
            expected: expected_digest.to_owned(),
            actual: digest,
        });
    }
    let id = hasher.finalize().as_bytes().to_vec();
    reader
        .seek(SeekFrom::Start(source.offset))
        .map_err(|cause| FileError::Io {
            operation: "seek inline blob",
            path: None,
            cause: cause.into(),
        })?;
    let meta = BlobFrameMeta {
        size: source.size,
        id: &id,
        digest: &digest,
        media_type: source.media_type,
        representation: source.representation,
        prev,
    };
    write_blob_frame(writer, reader.take(source.size), &meta).map_err(|cause| FileError::Io {
        operation: "write inline blob range",
        path: None,
        cause: cause.into(),
    })?;
    *prev = id;
    Ok(())
}

pub(crate) fn digest_blob_range<R: Read + Seek>(
    reader: &mut R,
    source: FileBlobRange<'_>,
) -> Result<String, FileError> {
    reader
        .seek(SeekFrom::Start(source.offset))
        .map_err(|cause| FileError::Io {
            operation: "seek inline blob",
            path: None,
            cause: cause.into(),
        })?;
    let mut sink = std::io::sink();
    let (digest, _) = copy_counted_and_hash(reader.take(source.size), &mut sink, source.size)
        .map_err(|cause| FileError::Io {
            operation: "hash inline blob range",
            path: None,
            cause: cause.into(),
        })?;
    Ok(digest)
}

fn digest_file(path: &Path, expected_size: u64) -> Result<String, FileError> {
    let mut file = fs::File::open(path).map_err(|cause| FileError::Io {
        operation: "read",
        path: Some(path.to_path_buf()),
        cause: cause.into(),
    })?;
    let mut sink = std::io::sink();
    let (digest, _) =
        copy_counted_and_hash(&mut file, &mut sink, expected_size).map_err(|cause| {
            FileError::Io {
                operation: "hash",
                path: Some(path.to_path_buf()),
                cause: cause.into(),
            }
        })?;
    Ok(digest)
}

#[derive(Clone, Debug)]
struct ResolvedFile {
    fspath: PathBuf,
    relpath: String,
    digest: String,
    size: u64,
    mode: u32,
    modified: String,
    media_type: String,
}

fn resolved_files_with_metadata(sources: &[&Path]) -> Result<Vec<ResolvedFile>, FileError> {
    let entries = resolve_sources(sources)?;
    let mut out = Vec::with_capacity(entries.len());
    for (fspath, relpath) in entries {
        let meta = fs::metadata(&fspath).map_err(|cause| FileError::Io {
            operation: "stat",
            path: Some(fspath.clone()),
            cause: cause.into(),
        })?;
        let size = meta.len();
        let mode = 0o644_u32;
        let mtime = meta.modified().map_err(|cause| FileError::Io {
            operation: "mtime",
            path: Some(fspath.clone()),
            cause: cause.into(),
        })?;
        let modified = format_datetime(&mtime, &fspath)?;
        let media_type = media_type_for_path(&fspath).to_string();
        let digest = digest_file(&fspath, size)?;
        out.push(ResolvedFile {
            fspath,
            relpath,
            digest,
            size,
            mode,
            modified,
            media_type,
        });
    }
    Ok(out)
}

/// Pack files/directories into a deterministic GTS files-profile archive.
pub fn pack(sources: &[&Path]) -> Result<Vec<u8>, FileError> {
    let mut out = Vec::new();
    pack_to_writer(sources, &mut out)?;
    Ok(out)
}

/// Pack files/directories into a deterministic GTS files-profile archive without
/// retaining regular-file payloads in memory.
pub fn pack_to_writer<W: Write>(sources: &[&Path], mut output: W) -> Result<(), FileError> {
    let mut w = Writer::new("files");

    let shared = vec![
        Term::iri(FILES_NS.to_string() + "FileEntry"),
        Term::iri(FILES_NS.to_string() + "path"),
        Term::iri(FILES_NS.to_string() + "digest"),
        Term::iri(FILES_NS.to_string() + "size"),
        Term::iri(FILES_NS.to_string() + "mode"),
        Term::iri(FILES_NS.to_string() + "modified"),
        Term::iri(FILES_NS.to_string() + "mediaType"),
        Term::iri(RDF_TYPE),
        Term::iri(XSD_INTEGER),
        Term::iri(XSD_DATETIME),
    ];
    w.add_terms(&shared);
    let file_entry_id: usize = 0;
    let path_id: usize = 1;
    let digest_id: usize = 2;
    let size_id: usize = 3;
    let mode_id: usize = 4;
    let modified_id: usize = 5;
    let media_type_id: usize = 6;
    let type_id: usize = 7;
    let xsd_integer_id: usize = 8;
    let xsd_datetime_id: usize = 9;

    let entries = resolved_files_with_metadata(sources)?;

    let mut file_terms: Vec<Term> = Vec::new();
    let mut quads: Vec<Quad> = Vec::new();

    for (idx, entry) in entries.iter().enumerate() {
        let entry_label = format!("f{idx}");
        let entry_term = Term::blank(&entry_label);
        let path_term = Term::literal(&entry.relpath, None);
        let digest_term = Term::literal(&entry.digest, None);
        let size_term = Term::literal(entry.size.to_string(), Some(xsd_integer_id));
        let mode_term = Term::literal(entry.mode.to_string(), Some(xsd_integer_id));
        let modified_term = Term::literal(&entry.modified, Some(xsd_datetime_id));
        let media_term = Term::literal(&entry.media_type, None);

        let base = shared.len() + file_terms.len();
        file_terms.extend(vec![
            entry_term,
            path_term,
            digest_term,
            size_term,
            mode_term,
            modified_term,
            media_term,
        ]);
        let entry_id = base;
        quads.push((entry_id, type_id, file_entry_id, None));
        quads.push((entry_id, path_id, base + 1, None));
        quads.push((entry_id, digest_id, base + 2, None));
        quads.push((entry_id, size_id, base + 3, None));
        quads.push((entry_id, mode_id, base + 4, None));
        quads.push((entry_id, modified_id, base + 5, None));
        quads.push((entry_id, media_type_id, base + 6, None));
    }

    if !file_terms.is_empty() {
        w.add_terms(&file_terms);
    }
    if !quads.is_empty() {
        w.add_quads(&quads);
    }

    let mut prev = w.head().to_vec();
    output
        .write_all(&w.into_bytes())
        .map_err(|cause| FileError::Io {
            operation: "write files-profile metadata",
            path: None,
            cause: cause.into(),
        })?;
    let mut seen: FastSet<String> = FastSet::default();
    for entry in &entries {
        if !seen.insert(entry.digest.clone()) {
            continue;
        }
        let source = FileBlobSource {
            path: entry.fspath.clone(),
            size: entry.size,
            media_type: Some(entry.media_type.clone()),
            representation: None,
        };
        append_blob_path(&mut output, &mut prev, &source, &entry.digest)?;
    }
    Ok(())
}

/// Author an opt-in files-profile v2 archive from typed entries.
///
/// This helper is the Rust foundation for the tar bridge. The default
/// [`pack`] command continues to emit v1 archives.
pub fn pack_entries_v2(entries: &[FileEntry]) -> Result<Vec<u8>, FileError> {
    let mut out = Vec::new();
    pack_entries_v2_to_writer(entries, &mut out)?;
    Ok(out)
}

/// Author an opt-in files-profile v2 archive directly to a writer.
pub fn pack_entries_v2_to_writer<W: Write>(
    entries: &[FileEntry],
    mut output: W,
) -> Result<(), FileError> {
    let (writer, blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|cause| FileError::Io {
            operation: "write files-profile-v2 metadata",
            path: None,
            cause: cause.into(),
        })?;
    for (digest, (data, media_type)) in blobs {
        append_blob_bytes(
            &mut output,
            &mut prev,
            FileBlobBytes {
                data,
                media_type,
                representation: None,
            },
            &digest,
        )?;
    }
    Ok(())
}

/// Author a files-profile v2 archive from typed entries and borrowed blob bytes.
pub fn pack_entries_v2_with_blob_bytes<W: Write>(
    entries: &[FileEntry],
    blob_sources: &BTreeMap<String, FileBlobBytes<'_>>,
    mut output: W,
) -> Result<(), FileError> {
    let (writer, _inline_blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|cause| FileError::Io {
            operation: "write files-profile-v2 metadata",
            path: None,
            cause: cause.into(),
        })?;
    for (digest, source) in blob_sources {
        append_blob_bytes(&mut output, &mut prev, *source, digest)?;
    }
    Ok(())
}

/// Author a files-profile v2 archive from typed entries and seekable blob ranges.
pub fn pack_entries_v2_with_blob_ranges<R: Read + Seek, W: Write>(
    entries: &[FileEntry],
    reader: &mut R,
    blob_sources: &BTreeMap<String, FileBlobRange<'_>>,
    mut output: W,
) -> Result<(), FileError> {
    let (writer, _inline_blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|cause| FileError::Io {
            operation: "write files-profile-v2 metadata",
            path: None,
            cause: cause.into(),
        })?;
    for (digest, source) in blob_sources {
        append_blob_range(&mut output, &mut prev, reader, *source, digest)?;
    }
    Ok(())
}

/// Author a files-profile v2 archive from typed entries and regular-file blob paths.
///
/// This is the bounded-memory counterpart to [`pack_entries_v2`]: metadata is
/// still collected and sorted in memory, while regular-file payload frames are
/// hashed and written from the supplied paths in bounded chunks.
pub fn pack_entries_v2_with_blob_paths<W: Write>(
    entries: &[FileEntry],
    blob_sources: &BTreeMap<String, FileBlobSource>,
    mut output: W,
) -> Result<(), FileError> {
    let (writer, _inline_blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|cause| FileError::Io {
            operation: "write files-profile-v2 metadata",
            path: None,
            cause: cause.into(),
        })?;
    for (digest, source) in blob_sources {
        append_blob_path(&mut output, &mut prev, source, digest)?;
    }
    Ok(())
}

fn build_entries_v2_prefix(
    entries: &[FileEntry],
) -> Result<(Writer, InlineBlobMap<'_>), FileError> {
    if entries.is_empty() {
        return Err(FileError::EmptyEntries);
    }

    let mut entries: Vec<&FileEntry> = entries.iter().collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let mut seen_paths = FastSet::default();
    let mut builder = TermBuilder::default();
    let rdf_type = builder.iri(RDF_TYPE);
    let file_entry = builder.iri(FILE_ENTRY);
    let mut blobs: InlineBlobMap<'_> = BTreeMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        safe_archive_path(&entry.path)?;
        if !seen_paths.insert(entry.path.clone()) {
            return Err(FileError::DuplicateArchivePath {
                path: entry.path.clone(),
            });
        }
        validate_v2_entry(entry)?;

        let subject = builder.bnode(&format!("f{idx}"));
        builder.quads.push((subject, rdf_type, file_entry, None));
        builder.quad_lit(subject, FILES_PATH, &entry.path, None);
        builder.quad_lit(subject, FILES_TYPE, entry.kind.as_str(), None);

        if entry.kind == FileEntryKind::File {
            let (digest, size) = file_digest_and_size(entry)?;
            builder.quad_lit(subject, FILES_DIGEST, &digest, None);
            builder.quad_lit(subject, FILES_SIZE, &size.to_string(), Some(XSD_INTEGER));
            if let Some(data) = entry.data.as_deref() {
                blobs
                    .entry(digest)
                    .or_insert_with(|| (data, entry.media_type.as_deref()));
            }
        }

        if let Some(mode) = entry.mode {
            builder.quad_lit(subject, FILES_MODE, &mode.to_string(), Some(XSD_INTEGER));
        }
        if let Some(modified) = &entry.modified {
            builder.quad_lit(subject, FILES_MODIFIED, modified, Some(XSD_DATETIME));
        }
        if let Some(media_type) = &entry.media_type {
            builder.quad_lit(subject, FILES_MEDIA_TYPE, media_type, None);
        }
        if let Some(link_target) = &entry.link_target {
            builder.quad_lit(subject, FILES_LINK_TARGET, link_target, None);
        }
        if let Some(uid) = entry.uid {
            builder.quad_lit(subject, FILES_UID, &uid.to_string(), Some(XSD_INTEGER));
        }
        if let Some(gid) = entry.gid {
            builder.quad_lit(subject, FILES_GID, &gid.to_string(), Some(XSD_INTEGER));
        }
        if let Some(user_name) = &entry.user_name {
            builder.quad_lit(subject, FILES_USER_NAME, user_name, None);
        }
        if let Some(group_name) = &entry.group_name {
            builder.quad_lit(subject, FILES_GROUP_NAME, group_name, None);
        }
        if let Some(dev_major) = entry.dev_major {
            builder.quad_lit(
                subject,
                FILES_DEV_MAJOR,
                &dev_major.to_string(),
                Some(XSD_INTEGER),
            );
        }
        if let Some(dev_minor) = entry.dev_minor {
            builder.quad_lit(
                subject,
                FILES_DEV_MINOR,
                &dev_minor.to_string(),
                Some(XSD_INTEGER),
            );
        }

        let mut xattrs = entry.xattrs.clone();
        xattrs.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
        for (xidx, xattr) in xattrs.iter().enumerate() {
            let node = builder.bnode(&format!("x{idx}_{xidx}"));
            let predicate = builder.iri(FILES_XATTR);
            builder.quads.push((subject, predicate, node, None));
            builder.quad_lit(node, FILES_XATTR_NAME, &xattr.name, None);
            builder.quad_lit(node, FILES_XATTR_VALUE, &xattr.value, None);
        }

        let mut pax_records = entry.pax_records.clone();
        pax_records.sort_by(|a, b| a.key.cmp(&b.key).then(a.value.cmp(&b.value)));
        for (pidx, record) in pax_records.iter().enumerate() {
            let node = builder.bnode(&format!("p{idx}_{pidx}"));
            let predicate = builder.iri(FILES_PAX_RECORD);
            builder.quads.push((subject, predicate, node, None));
            builder.quad_lit(node, FILES_PAX_KEY, &record.key, None);
            builder.quad_lit(node, FILES_PAX_VALUE, &record.value, None);
        }
    }

    builder.quads.sort();
    let profile_meta = files_v2_profile_meta();
    let mut writer = Writer::with_options(
        "files",
        WriterOptions {
            meta: Some(profile_meta.clone()),
            ..WriterOptions::default()
        },
    )
    .map_err(FileError::Writer)?;
    writer.add_meta(profile_meta);
    if !builder.terms.is_empty() {
        writer.add_terms(&builder.terms);
    }
    if !builder.quads.is_empty() {
        writer.add_quads(&builder.quads);
    }
    Ok((writer, blobs))
}

fn files_v2_profile_meta() -> Value {
    Value::Map(vec![("profileVersion".into(), Value::from(2_u64))])
}

fn validate_v2_entry(entry: &FileEntry) -> Result<(), FileError> {
    match entry.kind {
        FileEntryKind::File => {
            let _ = file_digest_and_size(entry)?;
        }
        FileEntryKind::Directory => {
            if entry.data.is_some() || entry.digest.is_some() || entry.size.is_some() {
                return Err(FileError::EntryPayload {
                    path: entry.path.clone(),
                    kind: entry.kind,
                    data_size: entry.data.as_ref().map(Vec::len),
                    digest: entry.digest.clone(),
                    size: entry.size,
                });
            }
        }
        FileEntryKind::Symlink => {
            if entry.link_target.as_deref().unwrap_or("").is_empty() {
                return Err(FileError::LinkTarget {
                    path: entry.path.clone(),
                    kind: entry.kind,
                });
            }
            reject_payload_fields(entry)?;
        }
        FileEntryKind::Hardlink => {
            let target = entry.link_target.as_deref().unwrap_or("");
            if target.is_empty() {
                return Err(FileError::LinkTarget {
                    path: entry.path.clone(),
                    kind: entry.kind,
                });
            }
            safe_archive_path(target)?;
            reject_payload_fields(entry)?;
        }
        FileEntryKind::Fifo | FileEntryKind::Socket => {
            reject_payload_fields(entry)?;
        }
        FileEntryKind::CharDev | FileEntryKind::BlockDev => {
            reject_payload_fields(entry)?;
            if entry.dev_major.is_none() || entry.dev_minor.is_none() {
                return Err(FileError::Device {
                    path: entry.path.clone(),
                    kind: entry.kind,
                    major: entry.dev_major,
                    minor: entry.dev_minor,
                });
            }
        }
    }
    Ok(())
}

fn reject_payload_fields(entry: &FileEntry) -> Result<(), FileError> {
    if entry.data.is_some() || entry.digest.is_some() || entry.size.is_some() {
        return Err(FileError::EntryPayload {
            path: entry.path.clone(),
            kind: entry.kind,
            data_size: entry.data.as_ref().map(Vec::len),
            digest: entry.digest.clone(),
            size: entry.size,
        });
    }
    Ok(())
}

fn file_digest_and_size(entry: &FileEntry) -> Result<(String, u64), FileError> {
    match (&entry.data, &entry.digest, entry.size) {
        (Some(data), digest, size) => {
            let computed_digest = digest_string(data);
            if let Some(expected) = digest.as_ref().filter(|value| **value != computed_digest) {
                return Err(FileError::EntryDigest {
                    path: entry.path.clone(),
                    expected: expected.clone(),
                    actual: computed_digest,
                });
            }
            let computed_size = data.len() as u64;
            if let Some(expected) = size.filter(|value| *value != computed_size) {
                return Err(FileError::EntrySize {
                    path: entry.path.clone(),
                    expected,
                    actual: computed_size,
                });
            }
            Ok((computed_digest, computed_size))
        }
        (None, Some(digest), Some(size)) => Ok((digest.clone(), size)),
        (None, _, _) => Err(FileError::MissingPayload {
            path: entry.path.clone(),
            digest: entry.digest.clone(),
            size: entry.size,
        }),
    }
}

fn format_datetime(time: &std::time::SystemTime, path: &Path) -> Result<String, FsRefusal> {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|cause| FsRefusal::DateTimeBeforeEpoch {
            path: path.to_path_buf(),
            cause,
        })?;
    let seconds = duration.as_secs();
    let signed = i64::try_from(seconds).map_err(|cause| FsRefusal::DateTimeRange {
        path: path.to_path_buf(),
        seconds,
        cause,
    })?;
    crate::rfc3339::format(signed, 0).map_err(|cause| FsRefusal::DateTimeFormat {
        path: path.to_path_buf(),
        seconds,
        cause,
    })
}

/// Read typed files-profile entries from a folded graph, keyed by `files:path`.
///
/// # Errors
///
/// Returns an error when the graph is not a files-profile archive, an entry
/// carries an invalid field value, or two entries share the same path.
pub fn read_entries(graph: &Graph) -> Result<BTreeMap<String, FileEntry>, FileError> {
    let mut type_ids: FastSet<usize> = FastSet::default();
    let mut file_entry_ids: FastSet<usize> = FastSet::default();
    let mut field_name_by_id: FastMap<usize, String> = FastMap::default();
    for (idx, term) in graph.terms.iter().enumerate() {
        if term.kind != TermKind::Iri {
            continue;
        }
        let Some(value) = &term.value else {
            continue;
        };
        if value == RDF_TYPE {
            type_ids.insert(idx);
        } else if value == FILE_ENTRY {
            file_entry_ids.insert(idx);
        } else if let Some(rest) = value.strip_prefix(FILES_NS) {
            field_name_by_id.insert(idx, rest.to_string());
        }
    }
    if type_ids.is_empty() {
        return Err(FileError::MissingVocabulary { term: "rdf:type" });
    }
    if file_entry_ids.is_empty() {
        return Err(FileError::MissingVocabulary { term: "FileEntry" });
    }

    let mut direct: BTreeMap<usize, BTreeMap<String, String>> = BTreeMap::new();
    let mut xattr_links: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut pax_links: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut file_entry_subjects: FastSet<usize> = FastSet::default();
    for &(s, p, o, _g) in &graph.quads {
        if type_ids.contains(&p) && file_entry_ids.contains(&o) {
            file_entry_subjects.insert(s);
            direct.entry(s).or_default();
        } else if let Some(field_name) = field_name_by_id.get(&p) {
            if field_name == "xattr" {
                xattr_links.entry(s).or_default().push(o);
            } else if field_name == "paxRecord" {
                pax_links.entry(s).or_default().push(o);
            } else {
                let term = &graph.terms[o];
                let value = term.value.clone().unwrap_or_default();
                direct
                    .entry(s)
                    .or_default()
                    .insert(field_name.clone(), value);
            }
        }
    }

    let mut by_path: BTreeMap<String, FileEntry> = BTreeMap::new();
    for (s, fields) in &direct {
        if !file_entry_subjects.contains(s) {
            continue;
        }
        let Some(path) = fields.get("path") else {
            continue;
        };
        let kind = fields
            .get("type")
            .map(|value| FileEntryKind::parse(value))
            .transpose()?
            .unwrap_or_default();
        let entry = FileEntry {
            path: path.clone(),
            kind,
            digest: fields.get("digest").cloned(),
            size: parse_optional_integer(fields, "size")?,
            mode: parse_optional_integer(fields, "mode")?,
            modified: fields.get("modified").cloned(),
            media_type: fields.get("mediaType").cloned(),
            link_target: fields.get("linkTarget").cloned(),
            uid: parse_optional_integer(fields, "uid")?,
            gid: parse_optional_integer(fields, "gid")?,
            user_name: fields.get("userName").cloned(),
            group_name: fields.get("groupName").cloned(),
            dev_major: parse_optional_integer(fields, "devMajor")?,
            dev_minor: parse_optional_integer(fields, "devMinor")?,
            xattrs: read_xattrs(*s, &direct, &xattr_links)?,
            pax_records: read_pax_records(*s, &direct, &pax_links)?,
            data: None,
        };
        if by_path.contains_key(path) {
            return Err(FileError::DuplicatePath { path: path.clone() });
        }
        by_path.insert(path.clone(), entry);
    }
    Ok(by_path)
}

/// Read files-profile entries as string field maps, keyed by `files:path`.
///
/// # Errors
///
/// Propagates the same failures as [`read_entries`].
pub fn read_file_entries(
    graph: &Graph,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, FileError> {
    read_entries(graph).map(|entries| {
        entries
            .into_iter()
            .map(|(path, entry)| (path, entry_to_field_map(&entry)))
            .collect()
    })
}

fn read_xattrs(
    subject: usize,
    direct: &BTreeMap<usize, BTreeMap<String, String>>,
    links: &BTreeMap<usize, Vec<usize>>,
) -> Result<Vec<FileXattr>, FileError> {
    let mut out = Vec::new();
    for node in links.get(&subject).into_iter().flatten() {
        let fields = direct.get(node).ok_or(FileError::MissingFields {
            role: "xattr",
            node: *node,
        })?;
        let name = fields.get("xattrName").ok_or(FileError::MissingField {
            role: "xattr",
            node: *node,
            field: "xattrName",
        })?;
        let value = fields.get("xattrValue").ok_or(FileError::MissingField {
            role: "xattr",
            node: *node,
            field: "xattrValue",
        })?;
        out.push(FileXattr {
            name: name.clone(),
            value: value.clone(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
    Ok(out)
}

fn read_pax_records(
    subject: usize,
    direct: &BTreeMap<usize, BTreeMap<String, String>>,
    links: &BTreeMap<usize, Vec<usize>>,
) -> Result<Vec<FilePaxRecord>, FileError> {
    let mut out = Vec::new();
    for node in links.get(&subject).into_iter().flatten() {
        let fields = direct.get(node).ok_or(FileError::MissingFields {
            role: "paxRecord",
            node: *node,
        })?;
        let key = fields.get("paxKey").ok_or(FileError::MissingField {
            role: "paxRecord",
            node: *node,
            field: "paxKey",
        })?;
        let value = fields.get("paxValue").ok_or(FileError::MissingField {
            role: "paxRecord",
            node: *node,
            field: "paxValue",
        })?;
        out.push(FilePaxRecord {
            key: key.clone(),
            value: value.clone(),
        });
    }
    out.sort_by(|a, b| a.key.cmp(&b.key).then(a.value.cmp(&b.value)));
    Ok(out)
}

/// The integer at `key`, when present: `size`, `uid` and the device numbers
/// are `u64`, `mode` is `u32`.
fn parse_optional_integer<T>(
    fields: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<T>, FileError>
where
    T: std::str::FromStr<Err = core::num::ParseIntError>,
{
    fields
        .get(key)
        .map(|value| {
            value.parse::<T>().map_err(|cause| FileError::Integer {
                field: key.to_owned(),
                actual: value.clone(),
                cause,
            })
        })
        .transpose()
}

fn entry_to_field_map(entry: &FileEntry) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    fields.insert("path".to_string(), entry.path.clone());
    fields.insert("type".to_string(), entry.kind.as_str().to_string());
    if let Some(value) = &entry.digest {
        fields.insert("digest".to_string(), value.clone());
    }
    if let Some(value) = entry.size {
        fields.insert("size".to_string(), value.to_string());
    }
    if let Some(value) = entry.mode {
        fields.insert("mode".to_string(), value.to_string());
    }
    if let Some(value) = &entry.modified {
        fields.insert("modified".to_string(), value.clone());
    }
    if let Some(value) = &entry.media_type {
        fields.insert("mediaType".to_string(), value.clone());
    }
    if let Some(value) = &entry.link_target {
        fields.insert("linkTarget".to_string(), value.clone());
    }
    if let Some(value) = entry.uid {
        fields.insert("uid".to_string(), value.to_string());
    }
    if let Some(value) = entry.gid {
        fields.insert("gid".to_string(), value.to_string());
    }
    if let Some(value) = &entry.user_name {
        fields.insert("userName".to_string(), value.clone());
    }
    if let Some(value) = &entry.group_name {
        fields.insert("groupName".to_string(), value.clone());
    }
    if let Some(value) = entry.dev_major {
        fields.insert("devMajor".to_string(), value.to_string());
    }
    if let Some(value) = entry.dev_minor {
        fields.insert("devMinor".to_string(), value.to_string());
    }
    fields
}

fn dest_path(dest: &Path, archive_path: &str) -> Result<PathBuf, FileError> {
    safe_archive_path(archive_path)?;
    // Resolve the destination itself (e.g. `/tmp` -> `/private/tmp` on macOS),
    // then resolve the closest existing target ancestor so an existing
    // symlinked directory below the destination cannot redirect writes.
    let dest_canon = dest.canonicalize().map_err(|cause| FileError::Io {
        operation: "resolve destination",
        path: Some(dest.to_path_buf()),
        cause: cause.into(),
    })?;
    let target = dest_canon.join(archive_path);

    let mut ancestor = target
        .parent()
        .unwrap_or(dest_canon.as_path())
        .to_path_buf();
    while !ancestor.exists() {
        let Some(parent) = ancestor.parent() else {
            break;
        };
        ancestor = parent.to_path_buf();
    }
    let ancestor_canon = ancestor.canonicalize().map_err(|cause| FileError::Io {
        operation: "resolve target ancestor",
        path: Some(ancestor.clone()),
        cause: cause.into(),
    })?;
    if !ancestor_canon.starts_with(&dest_canon) {
        return Err(FsRefusal::DestinationEscape {
            path: archive_path.to_owned(),
            root: dest_canon,
            ancestor: ancestor_canon,
        }
        .into());
    }
    Ok(target)
}

fn suppressed_blob_digests(graph: &Graph) -> FastSet<String> {
    let mut out: FastSet<String> = FastSet::default();
    for sup in &graph.suppressions {
        for target in &sup.targets {
            let Value::Map(entries) = target else {
                continue;
            };
            let mut kind = "";
            let mut digest: Option<String> = None;
            for (k, v) in entries {
                if let Value::Text(key) = k {
                    if key == "kind" {
                        if let Value::Text(val) = v {
                            kind = val.as_str();
                        }
                    } else if key == "digest" {
                        digest = Some(match v {
                            Value::Text(t) => t.clone(),
                            Value::Bytes(b) => digest_label(b),
                            _ => continue,
                        });
                    }
                }
            }
            if kind == "blob"
                && let Some(d) = digest
            {
                out.insert(d);
            }
        }
    }
    out
}

/// Extract FileEntry quads from a folded graph into dest.
pub fn unpack(graph: &Graph, dest: &Path, include_suppressed: bool) -> Result<(), FileError> {
    unpack_with_options(
        graph,
        dest,
        &UnpackOptions {
            include_suppressed,
            ..UnpackOptions::default()
        },
    )
}

/// Extract FileEntry quads from a folded graph into dest using an explicit safety policy.
pub fn unpack_with_options(
    graph: &Graph,
    dest: &Path,
    options: &UnpackOptions,
) -> Result<(), FileError> {
    let entries = read_entries(graph)?;
    let suppressed = if options.include_suppressed {
        FastSet::default()
    } else {
        suppressed_blob_digests(graph)
    };
    fs::create_dir_all(dest).map_err(|cause| FileError::Io {
        operation: "create",
        path: Some(dest.to_path_buf()),
        cause: cause.into(),
    })?;
    let blob_entries: BTreeMap<&str, &crate::model::BlobEntry> =
        graph.blobs.iter().map(|(d, e)| (d.as_str(), e)).collect();
    let mut decoded_cache: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut deferred_dirs = Vec::new();
    let mut deferred_links = Vec::new();

    for (path, entry) in entries {
        let target = dest_path(dest, &path)?;

        match entry.kind {
            FileEntryKind::Directory => {
                fs::create_dir_all(&target).map_err(|cause| FileError::Io {
                    operation: "create dir",
                    path: Some(target.clone()),
                    cause: cause.into(),
                })?;
                deferred_dirs.push((target, entry));
            }
            FileEntryKind::File => {
                let digest = entry
                    .digest
                    .as_ref()
                    .ok_or_else(|| FsRefusal::MissingDigest { path: path.clone() })?;
                if suppressed.contains(digest) {
                    continue;
                }
                let data = if let Some(cached) = decoded_cache.get(digest) {
                    cached.clone()
                } else {
                    let decoded = blob_entries
                        .get(digest.as_str())
                        .map(|entry| {
                            entry.decoded_vec().map_err(|cause| FsRefusal::BlobDecode {
                                path: path.clone(),
                                cause,
                            })
                        })
                        .transpose()?
                        .ok_or_else(|| FsRefusal::MissingBlob {
                            path: path.clone(),
                            digest: digest.clone(),
                        })?;
                    decoded_cache.insert(digest.clone(), decoded.clone());
                    decoded
                };
                let actual = digest_string(&data);
                if actual != *digest {
                    return Err(FsRefusal::Integrity {
                        path: path.clone(),
                        digest: digest.clone(),
                        actual,
                    }
                    .into());
                }

                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|cause| FileError::Io {
                        operation: "create dir",
                        path: Some(parent.to_path_buf()),
                        cause: cause.into(),
                    })?;
                }
                write_file_without_following_symlink(&target, &data, &path)?;

                restore_path_metadata(&target, &entry, options)?;
            }
            FileEntryKind::Symlink => {
                if !options.allow_symlinks {
                    return Err(FsRefusal::Refused {
                        path,
                        kind: entry.kind,
                        option: "--allow-symlinks",
                    }
                    .into());
                }
                let link_target =
                    entry
                        .link_target
                        .as_deref()
                        .ok_or_else(|| FileError::LinkTarget {
                            path: path.clone(),
                            kind: entry.kind,
                        })?;
                validate_symlink_target(&path, link_target)?;
                deferred_links.push((target, entry));
            }
            FileEntryKind::Hardlink => {
                if !options.allow_symlinks {
                    return Err(FsRefusal::Refused {
                        path,
                        kind: entry.kind,
                        option: "--allow-symlinks",
                    }
                    .into());
                }
                let link_target =
                    entry
                        .link_target
                        .as_deref()
                        .ok_or_else(|| FileError::LinkTarget {
                            path: path.clone(),
                            kind: entry.kind,
                        })?;
                safe_archive_path(link_target)?;
                let _ = dest_path(dest, link_target)?;
                deferred_links.push((target, entry));
            }
            FileEntryKind::Fifo
            | FileEntryKind::CharDev
            | FileEntryKind::BlockDev
            | FileEntryKind::Socket => {
                if !options.allow_special {
                    return Err(FsRefusal::Refused {
                        path,
                        kind: entry.kind,
                        option: "--allow-special",
                    }
                    .into());
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|cause| FileError::Io {
                        operation: "create dir",
                        path: Some(parent.to_path_buf()),
                        cause: cause.into(),
                    })?;
                }
                create_special_node(&target, &entry)?;
                restore_path_metadata(&target, &entry, options)?;
            }
        }
    }
    for (target, entry) in deferred_links {
        materialize_link(dest, &target, &entry)?;
        restore_path_metadata(&target, &entry, options)?;
    }
    for (target, entry) in deferred_dirs.into_iter().rev() {
        restore_path_metadata(&target, &entry, options)?;
    }
    Ok(())
}

fn write_file_without_following_symlink(
    target: &Path,
    data: &[u8],
    archive_path: &str,
) -> Result<(), FileError> {
    let parent = target.parent().ok_or_else(|| FsRefusal::MissingParent {
        path: target.to_path_buf(),
    })?;
    let name = target
        .file_name()
        .ok_or_else(|| FsRefusal::MissingFileName {
            path: target.to_path_buf(),
        })?
        .to_string_lossy();
    for attempt in 0..100 {
        let temp = parent.join(format!(".{name}.gts-tmp-{}-{attempt}", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => {
                let result = (|| {
                    // Scoped so the handle is closed before the rename replaces the
                    // target.
                    {
                        let mut file = file;
                        file.write_all(data).map_err(|cause| FileError::Io {
                            operation: "write",
                            path: Some(temp.clone()),
                            cause: cause.into(),
                        })?;
                    }
                    prepare_replace_target(target, archive_path)?;
                    fs::rename(&temp, target).map_err(|cause| FileError::Io {
                        operation: "replace",
                        path: Some(target.to_path_buf()),
                        cause: cause.into(),
                    })?;
                    Ok(())
                })();
                if result.is_err() {
                    let _ = fs::remove_file(&temp);
                }
                return result;
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(cause) => {
                return Err(FileError::Io {
                    operation: "create temp file for",
                    path: Some(target.to_path_buf()),
                    cause: cause.into(),
                });
            }
        }
    }
    Err(FsRefusal::TempAttempts {
        path: target.to_path_buf(),
        attempts: 100,
    }
    .into())
}

fn prepare_replace_target(target: &Path, archive_path: &str) -> Result<(), FileError> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(FsRefusal::WriteThroughSymlink {
            archive_path: archive_path.to_owned(),
            path: target.to_path_buf(),
        }
        .into()),
        Ok(metadata) if metadata.is_file() => {
            fs::remove_file(target).map_err(|cause| FileError::Io {
                operation: "remove existing",
                path: Some(target.to_path_buf()),
                cause: cause.into(),
            })
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(cause) => Err(FileError::Io {
            operation: "inspect",
            path: Some(target.to_path_buf()),
            cause: cause.into(),
        }),
    }
}

fn prepare_create_node_target(target: &Path, archive_path: &str) -> Result<(), FileError> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_dir() => Err(FsRefusal::ReplaceDirectory {
            archive_path: archive_path.to_owned(),
            path: target.to_path_buf(),
        }
        .into()),
        Ok(_) => fs::remove_file(target).map_err(|cause| FileError::Io {
            operation: "remove existing",
            path: Some(target.to_path_buf()),
            cause: cause.into(),
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(cause) => Err(FileError::Io {
            operation: "inspect",
            path: Some(target.to_path_buf()),
            cause: cause.into(),
        }),
    }
}

/// Refuse a symlink whose target, read relative to the link's own directory,
/// leaves the extraction root.
///
/// Targets are archive member paths, not IRI references, so this is not RFC 3986
/// reference resolution: dot-segment removal there clamps `..` at the root, which
/// would silently rewrite an escaping target into one inside the tree. Here a `..`
/// that would climb above the root is refused, together with absolute,
/// drive-relative and backslash-separated targets, so a hostile archive cannot
/// plant a link that points outside the destination.
fn validate_symlink_target(archive_path: &str, link_target: &str) -> Result<(), FileError> {
    if link_target.is_empty() {
        return Err(FileError::LinkTarget {
            path: archive_path.to_owned(),
            kind: FileEntryKind::Symlink,
        });
    }
    let normalized = link_target.replace('\\', "/");
    let drive_relative = link_target.len() >= 2
        && link_target.as_bytes()[1] == b':'
        && link_target.as_bytes()[0].is_ascii_alphabetic();
    if drive_relative || normalized.starts_with('/') {
        return Err(FsRefusal::SymlinkEscape {
            path: archive_path.to_owned(),
            target: link_target.to_owned(),
        }
        .into());
    }
    if link_target.contains('\\') {
        return Err(FsRefusal::SymlinkBackslash {
            path: archive_path.to_owned(),
            target: link_target.to_owned(),
        }
        .into());
    }

    let mut parts: Vec<&str> = archive_path
        .rsplit_once('/')
        .map_or(Vec::new(), |(parent, _)| {
            parent.split('/').filter(|part| !part.is_empty()).collect()
        });
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(FsRefusal::SymlinkEscape {
                        path: archive_path.to_owned(),
                        target: link_target.to_owned(),
                    }
                    .into());
                }
            }
            value => parts.push(value),
        }
    }
    if parts.is_empty() {
        return Err(FsRefusal::SymlinkRoot {
            path: archive_path.to_owned(),
            target: link_target.to_owned(),
        }
        .into());
    }
    safe_archive_path(&parts.join("/"))
}

fn materialize_link(dest: &Path, target: &Path, entry: &FileEntry) -> Result<(), FileError> {
    match entry.kind {
        FileEntryKind::Symlink => refuse_host_extraction(entry),
        FileEntryKind::Hardlink => {
            let link_target =
                entry
                    .link_target
                    .as_deref()
                    .ok_or_else(|| FileError::LinkTarget {
                        path: entry.path.clone(),
                        kind: entry.kind,
                    })?;
            let source = dest_path(dest, link_target)?;
            prepare_create_node_target(target, &entry.path)?;
            fs::hard_link(&source, target).map_err(|cause| {
                FsRefusal::HardLinkIo {
                    target: target.to_path_buf(),
                    source,
                    cause: cause.into(),
                }
                .into()
            })
        }
        _ => Err(FsRefusal::NotLink {
            path: entry.path.clone(),
            kind: entry.kind,
        }
        .into()),
    }
}

fn refuse_host_extraction(entry: &FileEntry) -> Result<(), FileError> {
    Err(FsRefusal::UnsupportedExtraction {
        path: entry.path.clone(),
        kind: entry.kind,
    }
    .into())
}

fn create_special_node(target: &Path, entry: &FileEntry) -> Result<(), FileError> {
    prepare_create_node_target(target, &entry.path)?;
    refuse_host_extraction(entry)
}

fn restore_path_metadata(
    target: &Path,
    entry: &FileEntry,
    options: &UnpackOptions,
) -> Result<(), FileError> {
    restore_owner(target, entry, options)?;

    // Symlink entries never reach this point: `refuse_host_extraction` refuses every
    // one before metadata is restored, so the only targets here are regular files,
    // hardlinks (which name a regular file) and directories.
    if let Some(modified) = &entry.modified {
        let (seconds, nanos) = parse_modified(modified)?;
        let instant =
            system_time_from_unix(seconds, nanos).ok_or_else(|| FsRefusal::MtimeHost {
                actual: modified.clone(),
                seconds,
                nanos,
            })?;
        set_modified_time(target, instant).map_err(|cause| FileError::Io {
            operation: "set mtime for",
            path: Some(target.to_path_buf()),
            cause: cause.into(),
        })?;
    }
    Ok(())
}

/// The instant `seconds` whole seconds (negative: before the epoch) plus
/// `nanos` nanoseconds after the Unix epoch, or `None` when `nanos` is not a
/// sub-second part or the host's `SystemTime` cannot hold the instant.
fn system_time_from_unix(seconds: i64, nanos: u32) -> Option<std::time::SystemTime> {
    use std::time::{Duration, UNIX_EPOCH};

    if nanos >= 1_000_000_000 {
        return None;
    }
    let whole = Duration::from_secs(seconds.unsigned_abs());
    let second = if seconds < 0 {
        UNIX_EPOCH.checked_sub(whole)?
    } else {
        UNIX_EPOCH.checked_add(whole)?
    };
    second.checked_add(Duration::from_nanos(u64::from(nanos)))
}

/// Sets `target`'s modification time to `modified`, leaving its access time
/// as it is. Works for regular files and directories, read-only ones included.
fn set_modified_time(target: &Path, modified: std::time::SystemTime) -> std::io::Result<()> {
    open_for_set_times(target)?.set_times(fs::FileTimes::new().set_modified(modified))
}

/// Opens `target` with exactly the access that setting its times needs.
///
/// Setting explicit times requires owning the file, not being able to write
/// it, so a read-only open serves read-only files and directories alike.
#[cfg(not(windows))]
fn open_for_set_times(target: &Path) -> std::io::Result<fs::File> {
    fs::File::open(target)
}

/// Opens `target` with exactly the access that setting its times needs.
///
/// `SetFileTime` needs a handle with `FILE_WRITE_ATTRIBUTES`, and asking for
/// that right alone keeps read-only files openable; a directory handle can
/// only be opened with `FILE_FLAG_BACKUP_SEMANTICS`.
#[cfg(windows)]
fn open_for_set_times(target: &Path) -> std::io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt as _;

    /// `FILE_WRITE_ATTRIBUTES` (WinNT.h, "File Access Rights Constants").
    const FILE_WRITE_ATTRIBUTES: u32 = 0x0000_0100;
    /// `FILE_FLAG_BACKUP_SEMANTICS` (fileapi.h, `CreateFileW` flags).
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    fs::OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(target)
}

fn restore_owner(
    _target: &Path,
    entry: &FileEntry,
    options: &UnpackOptions,
) -> Result<(), FileError> {
    if options.same_owner && (entry.uid.is_some() || entry.gid.is_some()) {
        return Err(FsRefusal::UnsupportedOwner {
            path: entry.path.clone(),
            uid: entry.uid,
            gid: entry.gid,
        }
        .into());
    }
    Ok(())
}

fn parse_modified(text: &str) -> Result<(i64, u32), FileError> {
    crate::rfc3339::parse(text).map_err(|cause| {
        FsRefusal::ParseModified {
            actual: text.to_owned(),
            cause,
        }
        .into()
    })
}

/// Compare an archive to a directory by content digest.
pub fn diff(graph: &Graph, directory: &Path) -> Result<Vec<String>, FileError> {
    let entries = read_entries(graph)?;
    let archive_digests: BTreeMap<String, String> = entries
        .iter()
        .filter(|(_, entry)| entry.kind == FileEntryKind::File)
        .map(|(p, e)| (p.clone(), e.digest.clone().unwrap_or_default()))
        .collect();

    if !directory.exists() {
        return Err(FsRefusal::MissingDirectory {
            path: directory.to_path_buf(),
        }
        .into());
    }

    let mut disk_digests: BTreeMap<String, String> = BTreeMap::new();
    let files = walk_dir_sorted(directory)?;
    for fspath in files {
        let relpath = to_posix_path(fspath.strip_prefix(directory).map_err(|_| {
            FsRefusal::OutsideDirectory {
                path: fspath.clone(),
                base: directory.to_path_buf(),
            }
        })?)?;
        let data = fs::read(&fspath).map_err(|cause| FileError::Io {
            operation: "read",
            path: Some(fspath.clone()),
            cause: cause.into(),
        })?;
        disk_digests.insert(relpath, digest_string(&data));
    }

    let archive_paths: FastSet<&String> = archive_digests.keys().collect();
    let disk_paths: FastSet<&String> = disk_digests.keys().collect();

    let mut lines: Vec<String> = Vec::new();
    for path in archive_paths.difference(&disk_paths) {
        lines.push(format!("removed: {path}"));
    }
    for path in disk_paths.difference(&archive_paths) {
        lines.push(format!("added: {path}"));
    }
    for path in archive_paths.intersection(&disk_paths) {
        if archive_digests.get(*path) != disk_digests.get(*path) {
            lines.push(format!("modified: {path}"));
        }
    }
    lines.sort();
    Ok(lines)
}

#[cfg(test)]
mod media_type_tests {
    use super::{media_type_for_extension, media_type_for_path};
    use std::path::Path;

    #[test]
    fn every_extension_maps_to_its_media_type() {
        let table = [
            ("txt", "text/plain"),
            ("html", "text/html"),
            ("htm", "text/html"),
            ("json", "application/json"),
            ("xml", "application/xml"),
            ("png", "image/png"),
            ("jpg", "image/jpeg"),
            ("jpeg", "image/jpeg"),
            ("gif", "image/gif"),
            ("webp", "image/webp"),
            ("pdf", "application/pdf"),
            ("zip", "application/zip"),
            ("gz", "application/gzip"),
            ("tar", "application/x-tar"),
            ("ttl", "text/turtle"),
            ("nt", "application/n-triples"),
            ("nq", "application/n-quads"),
            ("trig", "application/trig"),
            ("rdf", "application/rdf+xml"),
            ("sparql", "application/sparql-query"),
            ("rq", "application/sparql-query"),
            ("md", "text/markdown"),
            ("yaml", "application/yaml"),
            ("yml", "application/yaml"),
            ("cff", "application/yaml"),
        ];
        for (extension, media_type) in table {
            assert_eq!(
                media_type_for_extension(extension),
                media_type,
                "{extension}"
            );
            assert_eq!(
                media_type_for_extension(&extension.to_ascii_uppercase()),
                media_type,
                "{extension} upper-cased"
            );
        }
    }

    #[test]
    fn an_unknown_or_missing_extension_is_octet_stream() {
        assert_eq!(media_type_for_extension("bin"), "application/octet-stream");
        assert_eq!(media_type_for_extension(""), "application/octet-stream");
        assert_eq!(
            media_type_for_path(Path::new("LICENSE")),
            "application/octet-stream"
        );
        // A bare file name that merely equals an extension has no extension.
        assert_eq!(
            media_type_for_path(Path::new("ttl")),
            "application/octet-stream"
        );
    }

    #[test]
    fn a_path_uses_its_final_extension() {
        assert_eq!(
            media_type_for_path(Path::new("a/b.tar.gz")),
            "application/gzip"
        );
        assert_eq!(
            media_type_for_path(Path::new("dir.ttl/readme.md")),
            "text/markdown"
        );
    }
}

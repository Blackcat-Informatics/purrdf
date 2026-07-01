// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Files-profile pack/unpack/diff logic for GTS archives (§13.2, §14.2).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use ciborium::value::Value;

use crate::model::{Graph, Quad, Term, TermKind};
use crate::writer::{digest_string, Writer, WriterOptions};

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
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
const STREAM_CHUNK_SIZE: usize = 128 * 1024;

type InlineBlobMap<'a> = BTreeMap<String, (&'a [u8], Option<&'a str>)>;

/// files-profile v2 entry kind. Absence of `files:type` is read as [`Self::File`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileEntryKind {
    #[default]
    File,
    Directory,
    Symlink,
    Hardlink,
    Fifo,
    CharDev,
    BlockDev,
    Socket,
}

impl FileEntryKind {
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

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "file" => Ok(Self::File),
            "directory" => Ok(Self::Directory),
            "symlink" => Ok(Self::Symlink),
            "hardlink" => Ok(Self::Hardlink),
            "fifo" => Ok(Self::Fifo),
            "chardev" => Ok(Self::CharDev),
            "blockdev" => Ok(Self::BlockDev),
            "socket" => Ok(Self::Socket),
            other => Err(format!("unknown files:type value: {other}")),
        }
    }
}

/// One files-profile v2 extended attribute row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileXattr {
    pub name: String,
    /// Base64 lexical value of the attribute bytes.
    pub value: String,
}

/// One verbatim PAX escape-hatch key/value row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePaxRecord {
    pub key: String,
    pub value: String,
}

/// Typed files-profile entry used by the Rust v2 writer/reader.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub kind: FileEntryKind,
    pub digest: Option<String>,
    pub size: Option<u64>,
    pub mode: Option<u32>,
    pub modified: Option<String>,
    pub media_type: Option<String>,
    pub link_target: Option<String>,
    pub uid: Option<u64>,
    pub gid: Option<u64>,
    pub user_name: Option<String>,
    pub group_name: Option<String>,
    pub dev_major: Option<u64>,
    pub dev_minor: Option<u64>,
    pub xattrs: Vec<FileXattr>,
    pub pax_records: Vec<FilePaxRecord>,
    /// Optional inline payload bytes for regular-file authoring.
    pub data: Option<Vec<u8>>,
}

/// A regular-file blob source for bounded files-profile authoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileBlobSource {
    pub path: PathBuf,
    pub size: u64,
    pub media_type: Option<String>,
    pub representation: Option<String>,
}

/// Borrowed regular-file blob bytes for bounded files-profile authoring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileBlobBytes<'a> {
    pub data: &'a [u8],
    pub media_type: Option<&'a str>,
    pub representation: Option<&'a str>,
}

/// Borrowed regular-file blob range inside a seekable source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileBlobRange<'a> {
    pub offset: u64,
    pub size: u64,
    pub media_type: Option<&'a str>,
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
    ids: HashMap<TermKey, usize>,
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

fn iri_term(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn literal_term(value: &str, datatype: Option<usize>) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_string()),
        datatype,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn bnode_term(label: &str) -> Term {
    Term {
        kind: TermKind::Bnode,
        value: Some(label.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn safe_archive_path(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("empty archive path".to_string());
    }
    let normalized = name.replace('\\', "/");
    let drive_relative =
        name.len() >= 2 && name.as_bytes()[1] == b':' && name.as_bytes()[0].is_ascii_alphabetic();
    if drive_relative || normalized.starts_with('/') {
        return Err(format!(
            "absolute or drive-relative path not allowed in archive: {name}"
        ));
    }
    let parts: Vec<&str> = normalized.split('/').collect();
    for part in &parts {
        if *part == ".." {
            return Err(format!("path traversal not allowed in archive: {name}"));
        }
    }
    if name.contains('\\') {
        return Err(format!(
            "backslash path separator not allowed in archive: {name}"
        ));
    }
    if parts.iter().any(|part| part.is_empty() || *part == ".") {
        return Err(format!(
            "empty or current-directory path component not allowed in archive: {name}"
        ));
    }
    Ok(())
}

fn to_posix_path(path: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for c in path.components() {
        let s = c
            .as_os_str()
            .to_str()
            .ok_or_else(|| format!("non-UTF-8 path component in {path:?}"))?;
        parts.push(s.to_string());
    }
    Ok(parts.join("/"))
}

fn walk_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>, String> {
    fn recurse(out: &mut Vec<PathBuf>, dir: &Path) -> Result<(), std::io::Error> {
        let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let ftype = entry.file_type()?;
            if ftype.is_symlink() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("symlink not supported: {}", path.display()),
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
    recurse(&mut out, dir).map_err(|e| format!("walk {dir:?}: {e}"))?;
    Ok(out)
}

fn resolve_sources(sources: &[&Path]) -> Result<Vec<(PathBuf, String)>, String> {
    let mut entries: Vec<(PathBuf, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for src in sources {
        let meta = fs::symlink_metadata(src).map_err(|e| format!("{src:?}: {e}"))?;
        if meta.file_type().is_symlink() {
            return Err(format!("symlink not supported: {}", src.display()));
        }
        if meta.is_file() {
            let name = src
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("invalid source name: {src:?}"))?
                .to_string();
            safe_archive_path(&name)?;
            if !seen.insert(name.clone()) {
                return Err(format!("duplicate archive path: {name}"));
            }
            entries.push((src.to_path_buf(), name));
        } else if meta.is_dir() {
            let files = walk_dir_sorted(src)?;
            for fspath in files {
                let relpath = to_posix_path(
                    fspath
                        .strip_prefix(src)
                        .map_err(|_| format!("path outside source: {fspath:?}"))?,
                )?;
                safe_archive_path(&relpath)?;
                if !seen.insert(relpath.clone()) {
                    return Err(format!("duplicate archive path: {relpath}"));
                }
                entries.push((fspath, relpath));
            }
        } else {
            return Err(format!("unsupported source type: {src:?}"));
        }
    }
    entries.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(entries)
}

fn guess_media_type(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("txt") => "text/plain".to_string(),
        Some("html" | "htm") => "text/html".to_string(),
        Some("json") => "application/json".to_string(),
        Some("xml") => "application/xml".to_string(),
        Some("png") => "image/png".to_string(),
        Some("jpg" | "jpeg") => "image/jpeg".to_string(),
        Some("gif") => "image/gif".to_string(),
        Some("webp") => "image/webp".to_string(),
        Some("pdf") => "application/pdf".to_string(),
        Some("zip") => "application/zip".to_string(),
        Some("gz") => "application/gzip".to_string(),
        Some("tar") => "application/x-tar".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

struct HashingWriter<'a> {
    hasher: &'a mut blake3::Hasher,
}

impl Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn write_cbor_type_len<W: Write>(writer: &mut W, major: u8, len: u64) -> std::io::Result<()> {
    let prefix = major << 5;
    if len < 24 {
        writer.write_all(&[prefix | len as u8])
    } else if u8::try_from(len).is_ok() {
        writer.write_all(&[prefix | 0x18, len as u8])
    } else if u16::try_from(len).is_ok() {
        writer.write_all(&[prefix | 0x19])?;
        writer.write_all(&(len as u16).to_be_bytes())
    } else if u32::try_from(len).is_ok() {
        writer.write_all(&[prefix | 0x1a])?;
        writer.write_all(&(len as u32).to_be_bytes())
    } else {
        writer.write_all(&[prefix | 0x1b])?;
        writer.write_all(&len.to_be_bytes())
    }
}

fn write_cbor_map_len<W: Write>(writer: &mut W, len: u64) -> std::io::Result<()> {
    write_cbor_type_len(writer, 5, len)
}

fn write_cbor_text<W: Write>(writer: &mut W, text: &str) -> std::io::Result<()> {
    write_cbor_type_len(writer, 3, text.len() as u64)?;
    writer.write_all(text.as_bytes())
}

fn write_cbor_bytes_header<W: Write>(writer: &mut W, len: u64) -> std::io::Result<()> {
    write_cbor_type_len(writer, 2, len)
}

fn write_cbor_bytes<W: Write>(writer: &mut W, bytes: &[u8]) -> std::io::Result<()> {
    write_cbor_bytes_header(writer, bytes.len() as u64)?;
    writer.write_all(bytes)
}

fn write_blob_pub_map<W: Write>(
    writer: &mut W,
    digest: &str,
    media_type: Option<&str>,
    representation: Option<&str>,
) -> std::io::Result<()> {
    let len = 1 + u64::from(media_type.is_some()) + u64::from(representation.is_some());
    write_cbor_map_len(writer, len)?;
    if let Some(media_type) = media_type {
        write_cbor_text(writer, "mt")?;
        write_cbor_text(writer, media_type)?;
    }
    if let Some(representation) = representation {
        write_cbor_text(writer, "rep")?;
        write_cbor_text(writer, representation)?;
    }
    write_cbor_text(writer, "digest")?;
    write_cbor_text(writer, digest)
}

fn copy_counted_and_hash<R: Read, W: Write>(
    mut reader: R,
    writer: &mut W,
    expected_size: u64,
) -> std::io::Result<(String, u64)> {
    let mut digest = blake3::Hasher::new();
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
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!("blob source changed size: expected {expected_size}, read {written}"),
        ));
    }
    Ok((
        format!("blake3:{}", hex(digest.finalize().as_bytes())),
        written,
    ))
}

fn write_blob_preimage<R: Read, W: Write>(
    writer: &mut W,
    reader: R,
    size: u64,
    media_type: Option<&str>,
    representation: Option<&str>,
    prev: &[u8],
) -> std::io::Result<String> {
    write_cbor_map_len(writer, 4)?;
    write_cbor_text(writer, "d")?;
    write_cbor_bytes_header(writer, size)?;
    let (digest, _) = copy_counted_and_hash(reader, writer, size)?;
    write_cbor_text(writer, "t")?;
    write_cbor_text(writer, "blob")?;
    write_cbor_text(writer, "pub")?;
    write_blob_pub_map(writer, &digest, media_type, representation)?;
    write_cbor_text(writer, "prev")?;
    write_cbor_bytes(writer, prev)?;
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
    write_cbor_map_len(writer, 5)?;
    write_cbor_text(writer, "d")?;
    write_cbor_bytes_header(writer, meta.size)?;
    let (observed_digest, _) = copy_counted_and_hash(reader, writer, meta.size)?;
    if observed_digest != meta.digest {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "blob source changed digest: expected {}, read {observed_digest}",
                meta.digest
            ),
        ));
    }
    write_cbor_text(writer, "t")?;
    write_cbor_text(writer, "blob")?;
    write_cbor_text(writer, "id")?;
    write_cbor_bytes(writer, meta.id)?;
    write_cbor_text(writer, "pub")?;
    write_blob_pub_map(writer, meta.digest, meta.media_type, meta.representation)?;
    write_cbor_text(writer, "prev")?;
    write_cbor_bytes(writer, meta.prev)
}

fn append_blob_path<W: Write>(
    writer: &mut W,
    prev: &mut Vec<u8>,
    source: &FileBlobSource,
    expected_digest: &str,
) -> Result<(), String> {
    let media_type = source.media_type.as_deref();
    let representation = source.representation.as_deref();
    let mut file =
        fs::File::open(&source.path).map_err(|e| format!("read {:?}: {e}", source.path))?;
    let mut hasher = blake3::Hasher::new();
    let digest = {
        let mut sink = HashingWriter {
            hasher: &mut hasher,
        };
        write_blob_preimage(
            &mut sink,
            &mut file,
            source.size,
            media_type,
            representation,
            prev,
        )
        .map_err(|e| format!("hash blob {:?}: {e}", source.path))?
    };
    if digest != expected_digest {
        return Err(format!(
            "digest mismatch for {:?}: expected {expected_digest}, read {digest}",
            source.path
        ));
    }
    let id = hasher.finalize().as_bytes().to_vec();
    file.rewind()
        .map_err(|e| format!("seek {:?}: {e}", source.path))?;
    let meta = BlobFrameMeta {
        size: source.size,
        id: &id,
        digest: &digest,
        media_type,
        representation,
        prev,
    };
    write_blob_frame(writer, &mut file, &meta)
        .map_err(|e| format!("write blob {:?}: {e}", source.path))?;
    *prev = id;
    Ok(())
}

fn append_blob_bytes<W: Write>(
    writer: &mut W,
    prev: &mut Vec<u8>,
    source: FileBlobBytes<'_>,
    expected_digest: &str,
) -> Result<(), String> {
    let mut hasher = blake3::Hasher::new();
    let digest = {
        let mut sink = HashingWriter {
            hasher: &mut hasher,
        };
        write_blob_preimage(
            &mut sink,
            source.data,
            source.data.len() as u64,
            source.media_type,
            source.representation,
            prev,
        )
        .map_err(|e| format!("hash inline blob: {e}"))?
    };
    if digest != expected_digest {
        return Err(format!(
            "digest mismatch for inline blob: expected {expected_digest}, read {digest}"
        ));
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
    write_blob_frame(writer, source.data, &meta).map_err(|e| format!("write inline blob: {e}"))?;
    *prev = id;
    Ok(())
}

fn append_blob_range<R: Read + Seek, W: Write>(
    writer: &mut W,
    prev: &mut Vec<u8>,
    reader: &mut R,
    source: FileBlobRange<'_>,
    expected_digest: &str,
) -> Result<(), String> {
    let mut hasher = blake3::Hasher::new();
    let digest = {
        reader
            .seek(SeekFrom::Start(source.offset))
            .map_err(|e| format!("seek inline blob: {e}"))?;
        let mut sink = HashingWriter {
            hasher: &mut hasher,
        };
        write_blob_preimage(
            &mut sink,
            reader.take(source.size),
            source.size,
            source.media_type,
            source.representation,
            prev,
        )
        .map_err(|e| format!("hash inline blob range: {e}"))?
    };
    if digest != expected_digest {
        return Err(format!(
            "digest mismatch for inline blob range: expected {expected_digest}, read {digest}"
        ));
    }
    let id = hasher.finalize().as_bytes().to_vec();
    reader
        .seek(SeekFrom::Start(source.offset))
        .map_err(|e| format!("seek inline blob: {e}"))?;
    let meta = BlobFrameMeta {
        size: source.size,
        id: &id,
        digest: &digest,
        media_type: source.media_type,
        representation: source.representation,
        prev,
    };
    write_blob_frame(writer, reader.take(source.size), &meta)
        .map_err(|e| format!("write inline blob range: {e}"))?;
    *prev = id;
    Ok(())
}

pub(crate) fn digest_blob_range<R: Read + Seek>(
    reader: &mut R,
    source: FileBlobRange<'_>,
) -> Result<String, String> {
    reader
        .seek(SeekFrom::Start(source.offset))
        .map_err(|e| format!("seek inline blob: {e}"))?;
    let mut sink = std::io::sink();
    let (digest, _) = copy_counted_and_hash(reader.take(source.size), &mut sink, source.size)
        .map_err(|e| format!("hash inline blob range: {e}"))?;
    Ok(digest)
}

fn digest_file(path: &Path, expected_size: u64) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let mut sink = std::io::sink();
    let (digest, _) = copy_counted_and_hash(&mut file, &mut sink, expected_size)
        .map_err(|e| format!("hash {path:?}: {e}"))?;
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

fn resolved_files_with_metadata(sources: &[&Path]) -> Result<Vec<ResolvedFile>, String> {
    let entries = resolve_sources(sources)?;
    let mut out = Vec::with_capacity(entries.len());
    for (fspath, relpath) in entries {
        let meta = fs::metadata(&fspath).map_err(|e| format!("stat {fspath:?}: {e}"))?;
        let size = meta.len();
        let mode = 0o644_u32;
        let mtime = meta
            .modified()
            .map_err(|e| format!("mtime {fspath:?}: {e}"))?;
        let modified = format_datetime(&mtime).map_err(|e| format!("datetime {fspath:?}: {e}"))?;
        let media_type = guess_media_type(&fspath);
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
pub fn pack(sources: &[&Path]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    pack_to_writer(sources, &mut out)?;
    Ok(out)
}

/// Pack files/directories into a deterministic GTS files-profile archive without
/// retaining regular-file payloads in memory.
pub fn pack_to_writer<W: Write>(sources: &[&Path], mut output: W) -> Result<(), String> {
    let mut w = Writer::new("files");

    let shared = vec![
        iri_term(&(FILES_NS.to_string() + "FileEntry")),
        iri_term(&(FILES_NS.to_string() + "path")),
        iri_term(&(FILES_NS.to_string() + "digest")),
        iri_term(&(FILES_NS.to_string() + "size")),
        iri_term(&(FILES_NS.to_string() + "mode")),
        iri_term(&(FILES_NS.to_string() + "modified")),
        iri_term(&(FILES_NS.to_string() + "mediaType")),
        iri_term(RDF_TYPE),
        iri_term(XSD_INTEGER),
        iri_term(XSD_DATETIME),
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
        let entry_term = bnode_term(&entry_label);
        let path_term = literal_term(&entry.relpath, None);
        let digest_term = literal_term(&entry.digest, None);
        let size_term = literal_term(&entry.size.to_string(), Some(xsd_integer_id));
        let mode_term = literal_term(&entry.mode.to_string(), Some(xsd_integer_id));
        let modified_term = literal_term(&entry.modified, Some(xsd_datetime_id));
        let media_term = literal_term(&entry.media_type, None);

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
        .map_err(|e| format!("write files-profile metadata: {e}"))?;
    let mut seen: HashSet<String> = HashSet::new();
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
pub fn pack_entries_v2(entries: &[FileEntry]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    pack_entries_v2_to_writer(entries, &mut out)?;
    Ok(out)
}

/// Author an opt-in files-profile v2 archive directly to a writer.
pub fn pack_entries_v2_to_writer<W: Write>(
    entries: &[FileEntry],
    mut output: W,
) -> Result<(), String> {
    let (writer, blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|e| format!("write files-profile-v2 metadata: {e}"))?;
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
) -> Result<(), String> {
    let (writer, _inline_blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|e| format!("write files-profile-v2 metadata: {e}"))?;
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
) -> Result<(), String> {
    let (writer, _inline_blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|e| format!("write files-profile-v2 metadata: {e}"))?;
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
) -> Result<(), String> {
    let (writer, _inline_blobs) = build_entries_v2_prefix(entries)?;
    let mut prev = writer.head().to_vec();
    output
        .write_all(&writer.into_bytes())
        .map_err(|e| format!("write files-profile-v2 metadata: {e}"))?;
    for (digest, source) in blob_sources {
        append_blob_path(&mut output, &mut prev, source, digest)?;
    }
    Ok(())
}

fn build_entries_v2_prefix(entries: &[FileEntry]) -> Result<(Writer, InlineBlobMap<'_>), String> {
    if entries.is_empty() {
        return Err("files-profile v2 archive needs at least one entry".to_string());
    }

    let mut entries: Vec<&FileEntry> = entries.iter().collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let mut seen_paths = HashSet::new();
    let mut builder = TermBuilder::default();
    let rdf_type = builder.iri(RDF_TYPE);
    let file_entry = builder.iri(FILE_ENTRY);
    let mut blobs: InlineBlobMap<'_> = BTreeMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        safe_archive_path(&entry.path)?;
        if !seen_paths.insert(entry.path.clone()) {
            return Err(format!("duplicate archive path: {}", entry.path));
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
    .map_err(|err| format!("cannot author files v2 header: {err}"))?;
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

fn validate_v2_entry(entry: &FileEntry) -> Result<(), String> {
    match entry.kind {
        FileEntryKind::File => {
            let _ = file_digest_and_size(entry)?;
        }
        FileEntryKind::Directory => {
            if entry.data.is_some() || entry.digest.is_some() || entry.size.is_some() {
                return Err(format!(
                    "directory entry {} must not carry file bytes, digest, or size",
                    entry.path
                ));
            }
        }
        FileEntryKind::Symlink => {
            if entry.link_target.as_deref().unwrap_or("").is_empty() {
                return Err(format!("symlink entry {} needs linkTarget", entry.path));
            }
            reject_payload_fields(entry)?;
        }
        FileEntryKind::Hardlink => {
            let target = entry.link_target.as_deref().unwrap_or("");
            if target.is_empty() {
                return Err(format!("hardlink entry {} needs linkTarget", entry.path));
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
                return Err(format!(
                    "{} entry {} needs devMajor and devMinor",
                    entry.kind.as_str(),
                    entry.path
                ));
            }
        }
    }
    Ok(())
}

fn reject_payload_fields(entry: &FileEntry) -> Result<(), String> {
    if entry.data.is_some() || entry.digest.is_some() || entry.size.is_some() {
        return Err(format!(
            "{} entry {} must not carry file bytes, digest, or size",
            entry.kind.as_str(),
            entry.path
        ));
    }
    Ok(())
}

fn file_digest_and_size(entry: &FileEntry) -> Result<(String, u64), String> {
    match (&entry.data, &entry.digest, entry.size) {
        (Some(data), digest, size) => {
            let computed_digest = digest_string(data);
            if digest
                .as_deref()
                .is_some_and(|value| value != computed_digest)
            {
                return Err(format!("digest mismatch for {}", entry.path));
            }
            let computed_size = data.len() as u64;
            if size.is_some_and(|value| value != computed_size) {
                return Err(format!("size mismatch for {}", entry.path));
            }
            Ok((computed_digest, computed_size))
        }
        (None, Some(digest), Some(size)) => Ok((digest.clone(), size)),
        (None, _, _) => Err(format!(
            "regular file entry {} needs data or digest+size",
            entry.path
        )),
    }
}

fn format_datetime(time: &std::time::SystemTime) -> Result<String, String> {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("mtime before unix epoch: {e}"))?;
    let secs = duration.as_secs();
    let dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
        .map_err(|e| format!("invalid mtime timestamp {secs}: {e}"))?;
    let text = dt
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| format!("format datetime: {e}"))?;
    Ok(text.replace("+00:00", "Z"))
}

pub fn read_entries(graph: &Graph) -> Result<BTreeMap<String, FileEntry>, String> {
    let mut type_ids: HashSet<usize> = HashSet::new();
    let mut file_entry_ids: HashSet<usize> = HashSet::new();
    let mut field_name_by_id: HashMap<usize, String> = HashMap::new();
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
        return Err("not a files-profile archive: missing rdf:type".to_string());
    }
    if file_entry_ids.is_empty() {
        return Err("not a files-profile archive: missing FileEntry".to_string());
    }

    let mut direct: BTreeMap<usize, BTreeMap<String, String>> = BTreeMap::new();
    let mut xattr_links: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut pax_links: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut file_entry_subjects: HashSet<usize> = HashSet::new();
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
            size: parse_optional_u64(fields, "size")?,
            mode: parse_optional_u32(fields, "mode")?,
            modified: fields.get("modified").cloned(),
            media_type: fields.get("mediaType").cloned(),
            link_target: fields.get("linkTarget").cloned(),
            uid: parse_optional_u64(fields, "uid")?,
            gid: parse_optional_u64(fields, "gid")?,
            user_name: fields.get("userName").cloned(),
            group_name: fields.get("groupName").cloned(),
            dev_major: parse_optional_u64(fields, "devMajor")?,
            dev_minor: parse_optional_u64(fields, "devMinor")?,
            xattrs: read_xattrs(*s, &direct, &xattr_links)?,
            pax_records: read_pax_records(*s, &direct, &pax_links)?,
            data: None,
        };
        if by_path.contains_key(path) {
            return Err(format!("duplicate files:path in archive: {path}"));
        }
        by_path.insert(path.clone(), entry);
    }
    Ok(by_path)
}

pub fn read_file_entries(
    graph: &Graph,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
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
) -> Result<Vec<FileXattr>, String> {
    let mut out = Vec::new();
    for node in links.get(&subject).into_iter().flatten() {
        let fields = direct
            .get(node)
            .ok_or_else(|| format!("files:xattr node {node} has no fields"))?;
        let name = fields
            .get("xattrName")
            .ok_or_else(|| format!("files:xattr node {node} missing xattrName"))?;
        let value = fields
            .get("xattrValue")
            .ok_or_else(|| format!("files:xattr node {node} missing xattrValue"))?;
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
) -> Result<Vec<FilePaxRecord>, String> {
    let mut out = Vec::new();
    for node in links.get(&subject).into_iter().flatten() {
        let fields = direct
            .get(node)
            .ok_or_else(|| format!("files:paxRecord node {node} has no fields"))?;
        let key = fields
            .get("paxKey")
            .ok_or_else(|| format!("files:paxRecord node {node} missing paxKey"))?;
        let value = fields
            .get("paxValue")
            .ok_or_else(|| format!("files:paxRecord node {node} missing paxValue"))?;
        out.push(FilePaxRecord {
            key: key.clone(),
            value: value.clone(),
        });
    }
    out.sort_by(|a, b| a.key.cmp(&b.key).then(a.value.cmp(&b.value)));
    Ok(out)
}

fn parse_optional_u64(fields: &BTreeMap<String, String>, key: &str) -> Result<Option<u64>, String> {
    fields
        .get(key)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|err| format!("invalid files:{key} integer {value:?}: {err}"))
        })
        .transpose()
}

fn parse_optional_u32(fields: &BTreeMap<String, String>, key: &str) -> Result<Option<u32>, String> {
    fields
        .get(key)
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|err| format!("invalid files:{key} integer {value:?}: {err}"))
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

fn dest_path(dest: &Path, archive_path: &str) -> Result<PathBuf, String> {
    safe_archive_path(archive_path)?;
    // Resolve the destination itself (e.g. `/tmp` -> `/private/tmp` on macOS),
    // then resolve the closest existing target ancestor so an existing
    // symlinked directory below the destination cannot redirect writes.
    let dest_canon = dest
        .canonicalize()
        .map_err(|e| format!("resolve destination {dest:?}: {e}"))?;
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
    let ancestor_canon = ancestor
        .canonicalize()
        .map_err(|e| format!("resolve target ancestor {ancestor:?}: {e}"))?;
    if !ancestor_canon.starts_with(&dest_canon) {
        return Err(format!("path escapes destination: {archive_path}"));
    }
    Ok(target)
}

fn suppressed_blob_digests(graph: &Graph) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
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
                            Value::Bytes(b) => format!("blake3:{}", hex(b)),
                            _ => continue,
                        });
                    }
                }
            }
            if kind == "blob" {
                if let Some(d) = digest {
                    out.insert(d);
                }
            }
        }
    }
    out
}

fn hex(data: &[u8]) -> String {
    use std::fmt::Write as _;
    data.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Extract FileEntry quads from a folded graph into dest.
pub fn unpack(graph: &Graph, dest: &Path, include_suppressed: bool) -> Result<(), String> {
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
) -> Result<(), String> {
    let entries = read_entries(graph)?;
    let suppressed = if options.include_suppressed {
        HashSet::new()
    } else {
        suppressed_blob_digests(graph)
    };
    fs::create_dir_all(dest).map_err(|e| format!("create {dest:?}: {e}"))?;
    let blob_entries: BTreeMap<&str, &crate::model::BlobEntry> =
        graph.blobs.iter().map(|(d, e)| (d.as_str(), e)).collect();
    let mut decoded_cache: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut deferred_dirs = Vec::new();
    let mut deferred_links = Vec::new();

    for (path, entry) in entries {
        let target = dest_path(dest, &path)?;

        match entry.kind {
            FileEntryKind::Directory => {
                fs::create_dir_all(&target).map_err(|e| format!("create dir {target:?}: {e}"))?;
                deferred_dirs.push((target, entry));
            }
            FileEntryKind::File => {
                let digest = entry
                    .digest
                    .as_ref()
                    .ok_or_else(|| format!("missing digest for {path}"))?;
                if suppressed.contains(digest) {
                    continue;
                }
                let data = if let Some(cached) = decoded_cache.get(digest) {
                    cached.clone()
                } else {
                    let decoded = blob_entries
                        .get(digest.as_str())
                        .map(|entry| {
                            entry
                                .decoded_vec()
                                .map_err(|err| format!("decode inline blob for {path}: {err:?}"))
                        })
                        .transpose()?
                        .ok_or_else(|| format!("missing inline blob for {path}: {digest}"))?;
                    decoded_cache.insert(digest.clone(), decoded.clone());
                    decoded
                };
                if digest_string(&data) != *digest {
                    return Err(format!("integrity failure for {path}: {digest}"));
                }

                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("create dir {parent:?}: {e}"))?;
                }
                write_file_without_following_symlink(&target, &data, &path)?;

                restore_path_metadata(&target, &entry, options)?;
            }
            FileEntryKind::Symlink => {
                if !options.allow_symlinks {
                    return Err(format!(
                        "refusing symlink entry {path}: use --allow-symlinks"
                    ));
                }
                let link_target = entry
                    .link_target
                    .as_deref()
                    .ok_or_else(|| format!("symlink entry {path} needs linkTarget"))?;
                validate_symlink_target(&path, link_target)?;
                deferred_links.push((target, entry));
            }
            FileEntryKind::Hardlink => {
                if !options.allow_symlinks {
                    return Err(format!(
                        "refusing hardlink entry {path}: use --allow-symlinks"
                    ));
                }
                let link_target = entry
                    .link_target
                    .as_deref()
                    .ok_or_else(|| format!("hardlink entry {path} needs linkTarget"))?;
                safe_archive_path(link_target)?;
                let _ = dest_path(dest, link_target)?;
                deferred_links.push((target, entry));
            }
            FileEntryKind::Fifo
            | FileEntryKind::CharDev
            | FileEntryKind::BlockDev
            | FileEntryKind::Socket => {
                if !options.allow_special {
                    return Err(format!(
                        "refusing {} entry {path}: use --allow-special",
                        entry.kind.as_str()
                    ));
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("create dir {parent:?}: {e}"))?;
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
) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("missing parent for {target:?}"))?;
    let name = target
        .file_name()
        .ok_or_else(|| format!("missing file name for {target:?}"))?
        .to_string_lossy();
    for attempt in 0..100 {
        let temp = parent.join(format!(".{name}.gts-tmp-{}-{attempt}", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(data)
                        .map_err(|e| format!("write {temp:?}: {e}"))?;
                    drop(file);
                    prepare_replace_target(target, archive_path)?;
                    fs::rename(&temp, target).map_err(|e| format!("replace {target:?}: {e}"))?;
                    Ok(())
                })();
                if result.is_err() {
                    let _ = fs::remove_file(&temp);
                }
                return result;
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(format!("create temp file for {target:?}: {err}")),
        }
    }
    Err(format!(
        "create temp file for {target:?}: too many attempts"
    ))
}

fn prepare_replace_target(target: &Path, archive_path: &str) -> Result<(), String> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "refusing to write through symlink for {archive_path}: {target:?}"
        )),
        Ok(metadata) if metadata.is_file() => {
            fs::remove_file(target).map_err(|e| format!("remove existing {target:?}: {e}"))
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("inspect {target:?}: {err}")),
    }
}

fn prepare_create_node_target(target: &Path, archive_path: &str) -> Result<(), String> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_dir() => Err(format!(
            "refusing to replace directory for {archive_path}: {target:?}"
        )),
        Ok(_) => fs::remove_file(target).map_err(|e| format!("remove existing {target:?}: {e}")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("inspect {target:?}: {err}")),
    }
}

fn validate_symlink_target(archive_path: &str, link_target: &str) -> Result<(), String> {
    if link_target.is_empty() {
        return Err(format!("symlink entry {archive_path} needs linkTarget"));
    }
    let normalized = link_target.replace('\\', "/");
    let drive_relative = link_target.len() >= 2
        && link_target.as_bytes()[1] == b':'
        && link_target.as_bytes()[0].is_ascii_alphabetic();
    if drive_relative || normalized.starts_with('/') {
        return Err(format!(
            "symlink target escapes destination for {archive_path}: {link_target}"
        ));
    }
    if link_target.contains('\\') {
        return Err(format!(
            "backslash symlink target not allowed for {archive_path}: {link_target}"
        ));
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
                    return Err(format!(
                        "symlink target escapes destination for {archive_path}: {link_target}"
                    ));
                }
            }
            value => parts.push(value),
        }
    }
    if parts.is_empty() {
        return Err(format!(
            "symlink target resolves to destination root for {archive_path}: {link_target}"
        ));
    }
    safe_archive_path(&parts.join("/"))
}

fn materialize_link(dest: &Path, target: &Path, entry: &FileEntry) -> Result<(), String> {
    match entry.kind {
        FileEntryKind::Symlink => materialize_symlink(target, entry),
        FileEntryKind::Hardlink => {
            let link_target = entry
                .link_target
                .as_deref()
                .ok_or_else(|| format!("hardlink entry {} needs linkTarget", entry.path))?;
            let source = dest_path(dest, link_target)?;
            prepare_create_node_target(target, &entry.path)?;
            fs::hard_link(&source, target)
                .map_err(|e| format!("create hardlink {target:?} -> {source:?}: {e}"))
        }
        _ => Err(format!("{} is not a link entry", entry.path)),
    }
}

fn materialize_symlink(_target: &Path, entry: &FileEntry) -> Result<(), String> {
    Err(format!(
        "symlink extraction is host-specific and is not performed by the portable core: {}",
        entry.path
    ))
}

fn create_special_node(target: &Path, entry: &FileEntry) -> Result<(), String> {
    prepare_create_node_target(target, &entry.path)?;
    create_special_node_platform(target, entry)
}

fn create_special_node_platform(_target: &Path, entry: &FileEntry) -> Result<(), String> {
    Err(format!(
        "{} extraction is host-specific and is not performed by the portable core: {}",
        entry.kind.as_str(),
        entry.path
    ))
}

fn restore_path_metadata(
    target: &Path,
    entry: &FileEntry,
    options: &UnpackOptions,
) -> Result<(), String> {
    restore_owner(target, entry, options)?;

    if let Some(modified) = &entry.modified {
        let (seconds, nanos) = parse_datetime(modified)?;
        let timestamp = filetime::FileTime::from_unix_time(seconds, nanos);
        if entry.kind == FileEntryKind::Symlink {
            filetime::set_symlink_file_times(target, timestamp, timestamp)
                .map_err(|e| format!("set symlink mtime for {target:?}: {e}"))?;
        } else {
            filetime::set_file_mtime(target, timestamp)
                .map_err(|e| format!("set mtime for {target:?}: {e}"))?;
        }
    }
    Ok(())
}

fn restore_owner(_target: &Path, entry: &FileEntry, options: &UnpackOptions) -> Result<(), String> {
    if options.same_owner && (entry.uid.is_some() || entry.gid.is_some()) {
        return Err(format!(
            "ownership restoration is host-specific and is not performed by the portable core: {}",
            entry.path
        ));
    }
    Ok(())
}

fn parse_datetime(text: &str) -> Result<(i64, u32), String> {
    let text = text.strip_suffix('Z').unwrap_or(text);
    let dt = time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
        .or_else(|_| {
            time::OffsetDateTime::parse(
                &(text.to_string() + "+00:00"),
                &time::format_description::well_known::Rfc3339,
            )
        })
        .map_err(|e| format!("parse datetime {text}: {e}"))?;
    Ok((dt.unix_timestamp(), dt.nanosecond()))
}

/// Compare an archive to a directory by content digest.
pub fn diff(graph: &Graph, directory: &Path) -> Result<Vec<String>, String> {
    let entries = read_entries(graph)?;
    let archive_digests: BTreeMap<String, String> = entries
        .iter()
        .filter(|(_, entry)| entry.kind == FileEntryKind::File)
        .map(|(p, e)| (p.clone(), e.digest.clone().unwrap_or_default()))
        .collect();

    if !directory.exists() {
        return Err(format!("diff destination does not exist: {directory:?}"));
    }

    let mut disk_digests: BTreeMap<String, String> = BTreeMap::new();
    let files = walk_dir_sorted(directory)?;
    for fspath in files {
        let relpath = to_posix_path(
            fspath
                .strip_prefix(directory)
                .map_err(|_| format!("path outside directory: {fspath:?}"))?,
        )?;
        let data = fs::read(&fspath).map_err(|e| format!("read {fspath:?}: {e}"))?;
        disk_digests.insert(relpath, digest_string(&data));
    }

    let archive_paths: HashSet<&String> = archive_digests.keys().collect();
    let disk_paths: HashSet<&String> = disk_digests.keys().collect();

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

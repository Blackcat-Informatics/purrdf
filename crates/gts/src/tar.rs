// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tar stream import/export for files-profile-v2 GTS archives.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::codec::encode_chain;
use crate::files::{FileEntry, FileEntryKind, FilePaxRecord, read_entries};
use crate::model::{BlobEntry, Graph};

const BLOCK: usize = 512;
const ZERO_BLOCK: [u8; BLOCK] = [0; BLOCK];
const CORE_PAX_KEYS: &[&str] = &[
    "path", "linkpath", "size", "uid", "gid", "uname", "gname", "mtime", "devmajor", "devminor",
];

/// Error raised by tar import/export helpers.
#[derive(Debug)]
pub struct TarError {
    detail: String,
}

impl TarError {
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for TarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for TarError {}

/// Compression to apply while writing a tar stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TarCompression {
    #[default]
    None,
    Gzip,
    Zstd,
}

/// Options for [`to_tar`].
#[derive(Clone, Debug, Default)]
pub struct ToTarOptions {
    pub compression: TarCompression,
    pub numeric_owner: bool,
}

/// A decoded tar entry used by the tar-to-files-profile bridge.
#[derive(Clone, Debug)]
pub(crate) struct RawTarEntry<'a> {
    pub path: String,
    pub kind: FileEntryKind,
    pub mode: Option<u32>,
    pub modified_seconds: Option<u64>,
    pub uid: Option<u64>,
    pub gid: Option<u64>,
    pub user_name: Option<String>,
    pub group_name: Option<String>,
    pub link_target: Option<String>,
    pub dev_major: Option<u64>,
    pub dev_minor: Option<u64>,
    pub pax_records: Vec<FilePaxRecord>,
    pub data: &'a [u8],
}

/// A decoded tar entry whose regular-file body can be replayed from a seekable
/// uncompressed tar source.
#[derive(Clone, Debug)]
pub(crate) struct SeekTarEntry {
    pub path: String,
    pub kind: FileEntryKind,
    pub mode: Option<u32>,
    pub modified_seconds: Option<u64>,
    pub uid: Option<u64>,
    pub gid: Option<u64>,
    pub user_name: Option<String>,
    pub group_name: Option<String>,
    pub link_target: Option<String>,
    pub dev_major: Option<u64>,
    pub dev_minor: Option<u64>,
    pub pax_records: Vec<FilePaxRecord>,
    pub data_offset: u64,
    pub data_size: u64,
}

/// Write a deterministic tar stream from a folded files-profile graph.
pub fn to_tar<W: Write>(
    graph: &Graph,
    mut writer: W,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    match options.compression {
        TarCompression::None => write_tar_stream(graph, writer, options),
        TarCompression::Gzip => {
            let mut encoder = flate2::GzBuilder::new()
                .mtime(0)
                .write(writer, flate2::Compression::default());
            write_tar_stream(graph, &mut encoder, options)?;
            encoder
                .finish()
                .map_err(|err| TarError::new(format!("gzip encode tar stream: {err}")))?;
            Ok(())
        }
        TarCompression::Zstd => {
            let raw = to_tar_vec(graph, options)?;
            let encoded = encode_chain(&["zstd".to_string()], &raw)
                .map_err(|err| TarError::new(format!("zstd encode tar stream: {err}")))?;
            writer
                .write_all(&encoded)
                .map_err(|err| TarError::new(format!("write tar stream: {err}")))
        }
    }
}

/// Build an uncompressed tar stream from a folded files-profile graph.
pub fn to_tar_vec(graph: &Graph, options: &ToTarOptions) -> Result<Vec<u8>, TarError> {
    let mut out = Vec::new();
    write_tar_stream(graph, &mut out, options)?;
    Ok(out)
}

fn write_tar_stream<W: Write>(
    graph: &Graph,
    mut writer: W,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    let entries = read_entries(graph).map_err(TarError::new)?;
    let blobs: BTreeMap<&str, &BlobEntry> = graph
        .blobs
        .iter()
        .map(|(digest, entry)| (digest.as_str(), entry))
        .collect();
    for entry in entries.values() {
        append_entry(&mut writer, entry, &blobs, options)?;
    }
    writer
        .write_all(&ZERO_BLOCK)
        .and_then(|()| writer.write_all(&ZERO_BLOCK))
        .map_err(|err| TarError::new(format!("finish tar stream: {err}")))
}

fn append_entry<W: Write>(
    writer: &mut W,
    entry: &FileEntry,
    blobs: &BTreeMap<&str, &BlobEntry>,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    if entry.kind == FileEntryKind::Socket {
        return Err(TarError::new(format!(
            "tar cannot encode socket entry {}",
            entry.path
        )));
    }
    let data = if entry.kind == FileEntryKind::File {
        let digest = entry
            .digest
            .as_deref()
            .ok_or_else(|| TarError::new(format!("file entry {} has no digest", entry.path)))?;
        blobs
            .get(digest)
            .ok_or_else(|| {
                TarError::new(format!("missing inline blob for {}: {digest}", entry.path))
            })?
            .decoded_vec()
            .map_err(|err| TarError::new(format!("decode blob for {}: {err}", entry.path)))?
    } else {
        Vec::new()
    };
    let metadata = HeaderMetadata::from_entry(entry, data.len() as u64, options)?;
    append_pax_header(writer, entry, &metadata, options)?;
    write_header(writer, &metadata, &data)?;
    Ok(())
}

#[derive(Debug)]
struct HeaderMetadata {
    path: String,
    link_target: Option<String>,
    typeflag: u8,
    mode: u32,
    mtime: u64,
    uid: u64,
    gid: u64,
    size: u64,
    user_name: Option<String>,
    group_name: Option<String>,
    dev_major: Option<u64>,
    dev_minor: Option<u64>,
}

impl HeaderMetadata {
    fn from_entry(entry: &FileEntry, size: u64, options: &ToTarOptions) -> Result<Self, TarError> {
        let typeflag = match entry.kind {
            FileEntryKind::File => b'0',
            FileEntryKind::Directory => b'5',
            FileEntryKind::Symlink => b'2',
            FileEntryKind::Hardlink => b'1',
            FileEntryKind::Fifo => b'6',
            FileEntryKind::CharDev => b'3',
            FileEntryKind::BlockDev => b'4',
            FileEntryKind::Socket => unreachable!("socket rejected before header construction"),
        };
        Ok(Self {
            path: entry.path.clone(),
            link_target: match entry.kind {
                FileEntryKind::Symlink | FileEntryKind::Hardlink => {
                    Some(required_link_target(entry, entry.kind.as_str())?)
                }
                _ => None,
            },
            typeflag,
            mode: entry.mode.unwrap_or_else(|| default_mode(entry.kind)),
            mtime: parse_mtime(entry.modified.as_deref())?,
            uid: entry.uid.unwrap_or(0),
            gid: entry.gid.unwrap_or(0),
            size,
            user_name: (!options.numeric_owner)
                .then(|| entry.user_name.clone())
                .flatten(),
            group_name: (!options.numeric_owner)
                .then(|| entry.group_name.clone())
                .flatten(),
            dev_major: entry.dev_major,
            dev_minor: entry.dev_minor,
        })
    }
}

fn append_pax_header<W: Write>(
    writer: &mut W,
    entry: &FileEntry,
    metadata: &HeaderMetadata,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    let mut records: BTreeMap<String, String> = BTreeMap::new();
    if !fits_tar_path(&metadata.path) {
        records.insert("path".to_string(), metadata.path.clone());
    }
    if let Some(link_target) = &metadata.link_target
        && !fits_tar_field(link_target, 100)
    {
        records.insert("linkpath".to_string(), link_target.clone());
    }
    if !options.numeric_owner {
        if let Some(user_name) = &metadata.user_name
            && !fits_tar_field(user_name, 32)
        {
            records.insert("uname".to_string(), user_name.clone());
        }
        if let Some(group_name) = &metadata.group_name
            && !fits_tar_field(group_name, 32)
        {
            records.insert("gname".to_string(), group_name.clone());
        }
    }
    for record in &entry.pax_records {
        if !record.key.is_empty() && !CORE_PAX_KEYS.contains(&record.key.as_str()) {
            records.insert(record.key.clone(), record.value.clone());
        }
    }
    if records.is_empty() {
        return Ok(());
    }
    let body = pax_body(&records);
    let pax_name = pax_header_name(&metadata.path);
    let pax_meta = HeaderMetadata {
        path: pax_name,
        link_target: None,
        typeflag: b'x',
        mode: 0o644,
        mtime: 0,
        uid: 0,
        gid: 0,
        size: body.len() as u64,
        user_name: None,
        group_name: None,
        dev_major: None,
        dev_minor: None,
    };
    write_header(writer, &pax_meta, &body)
}

fn write_header<W: Write>(
    writer: &mut W,
    metadata: &HeaderMetadata,
    data: &[u8],
) -> Result<(), TarError> {
    let mut header = [0u8; BLOCK];
    write_path_fields(&mut header, &metadata.path)?;
    write_octal(&mut header[100..108], u64::from(metadata.mode))?;
    write_octal(&mut header[108..116], metadata.uid)?;
    write_octal(&mut header[116..124], metadata.gid)?;
    write_octal(&mut header[124..136], metadata.size)?;
    write_octal(&mut header[136..148], metadata.mtime)?;
    for byte in &mut header[148..156] {
        *byte = b' ';
    }
    header[156] = metadata.typeflag;
    if let Some(link_target) = &metadata.link_target {
        write_bytes_field(&mut header[157..257], link_target.as_bytes());
    }
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    if let Some(user_name) = &metadata.user_name {
        write_bytes_field(&mut header[265..297], user_name.as_bytes());
    }
    if let Some(group_name) = &metadata.group_name {
        write_bytes_field(&mut header[297..329], group_name.as_bytes());
    }
    if let Some(dev_major) = metadata.dev_major {
        write_octal(&mut header[329..337], dev_major)?;
    }
    if let Some(dev_minor) = metadata.dev_minor {
        write_octal(&mut header[337..345], dev_minor)?;
    }
    let sum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
    let checksum = format!("{sum:06o}\0 ");
    header[148..156].copy_from_slice(checksum.as_bytes());
    writer
        .write_all(&header)
        .map_err(|err| TarError::new(format!("write tar header for {}: {err}", metadata.path)))?;
    writer
        .write_all(data)
        .map_err(|err| TarError::new(format!("write tar data for {}: {err}", metadata.path)))?;
    let padding = (BLOCK - data.len() % BLOCK) % BLOCK;
    if padding != 0 {
        writer.write_all(&ZERO_BLOCK[..padding]).map_err(|err| {
            TarError::new(format!("write tar padding for {}: {err}", metadata.path))
        })?;
    }
    Ok(())
}

fn required_link_target(entry: &FileEntry, kind: &str) -> Result<String, TarError> {
    entry
        .link_target
        .as_deref()
        .filter(|target| !target.is_empty())
        .map(str::to_string)
        .ok_or_else(|| TarError::new(format!("{kind} entry {} has no link target", entry.path)))
}

fn default_mode(kind: FileEntryKind) -> u32 {
    match kind {
        FileEntryKind::Directory => 0o755,
        _ => 0o644,
    }
}

fn parse_mtime(value: Option<&str>) -> Result<u64, TarError> {
    let Some(value) = value else {
        return Ok(0);
    };
    let dt = time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .or_else(|_| {
            let text = value.strip_suffix('Z').unwrap_or(value);
            time::OffsetDateTime::parse(
                &(text.to_string() + "+00:00"),
                &time::format_description::well_known::Rfc3339,
            )
        })
        .map_err(|err| TarError::new(format!("parse mtime {value}: {err}")))?;
    let timestamp = dt.unix_timestamp();
    if timestamp < 0 {
        return Err(TarError::new(format!("negative tar mtime: {value}")));
    }
    Ok(timestamp as u64)
}

fn write_path_fields(header: &mut [u8; BLOCK], path: &str) -> Result<(), TarError> {
    if path.len() <= 100 {
        write_bytes_field(&mut header[0..100], path.as_bytes());
        return Ok(());
    }
    if let Some((prefix, name)) = split_ustar_path(path) {
        write_bytes_field(&mut header[0..100], name.as_bytes());
        write_bytes_field(&mut header[345..500], prefix.as_bytes());
        return Ok(());
    }
    write_bytes_field(&mut header[0..100], truncated_bytes(path.as_bytes(), 100));
    Ok(())
}

fn split_ustar_path(path: &str) -> Option<(&str, &str)> {
    if path.len() > 255 {
        return None;
    }
    for (idx, _) in path.match_indices('/').rev() {
        let prefix = &path[..idx];
        let name = &path[idx + 1..];
        if !name.is_empty() && prefix.len() <= 155 && name.len() <= 100 {
            return Some((prefix, name));
        }
    }
    None
}

fn fits_tar_path(path: &str) -> bool {
    path.len() <= 100 || split_ustar_path(path).is_some()
}

fn fits_tar_field(value: &str, width: usize) -> bool {
    value.len() <= width
}

fn write_bytes_field(field: &mut [u8], value: &[u8]) {
    let n = value.len().min(field.len());
    field[..n].copy_from_slice(&value[..n]);
}

fn truncated_bytes(value: &[u8], limit: usize) -> &[u8] {
    &value[..value.len().min(limit)]
}

fn write_octal(field: &mut [u8], value: u64) -> Result<(), TarError> {
    let width = field.len() - 1;
    let rendered = format!("{value:0width$o}");
    if rendered.len() > width {
        return Err(TarError::new(format!(
            "tar numeric value {value} does not fit {}-byte field",
            field.len()
        )));
    }
    let pad = width - rendered.len();
    for byte in &mut field[..pad] {
        *byte = b'0';
    }
    field[pad..width].copy_from_slice(rendered.as_bytes());
    field[width] = 0;
    Ok(())
}

fn pax_header_name(path: &str) -> String {
    let leaf = path
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("entry");
    let name = format!("PaxHeaders.0/{leaf}");
    if name.len() <= 100 {
        name
    } else {
        "PaxHeaders.0/entry".to_string()
    }
}

fn pax_body(records: &BTreeMap<String, String>) -> Vec<u8> {
    let estimated = records
        .iter()
        .map(|(key, value)| key.len() + value.len() + 32)
        .sum();
    let mut out = Vec::with_capacity(estimated);
    for (key, value) in records {
        let payload = format!("{key}={value}\n");
        let mut len = payload.len() + 2;
        loop {
            let digits = decimal_digits(len);
            let actual = digits + 1 + payload.len();
            if actual == len {
                break;
            }
            len = actual;
        }
        out.extend_from_slice(format!("{len} {payload}").as_bytes());
    }
    out
}

fn decimal_digits(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

pub(crate) fn read_uncompressed_tar(data: &[u8]) -> Result<Vec<RawTarEntry<'_>>, TarError> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut pax: BTreeMap<String, String> = BTreeMap::new();
    let mut long_path: Option<String> = None;
    let mut long_link: Option<String> = None;
    while offset + BLOCK <= data.len() {
        let header = &data[offset..offset + BLOCK];
        if header == ZERO_BLOCK {
            break;
        }
        verify_checksum(header, offset)?;
        let typeflag = header[156];
        let size = parse_octal(&header[124..136]).ok_or_else(|| {
            TarError::new(format!(
                "tar entry at block {} has invalid size",
                offset / BLOCK
            ))
        })?;
        offset += BLOCK;
        let body_end = offset
            .checked_add(size as usize)
            .filter(|end| *end <= data.len())
            .ok_or_else(|| TarError::new("tar member body overruns archive"))?;
        let body = &data[offset..body_end];
        offset = body_end + (BLOCK - (size as usize % BLOCK)) % BLOCK;

        match typeflag {
            b'x' => {
                pax = parse_pax_body(body)?;
            }
            b'L' => {
                long_path = Some(tar_string(body));
            }
            b'K' => {
                long_link = Some(tar_string(body));
            }
            b'0' | 0 | b'5' | b'2' | b'1' | b'6' | b'3' | b'4' => {
                let entry = raw_entry_from_header(
                    header,
                    typeflag,
                    body,
                    &pax,
                    &mut long_path,
                    &mut long_link,
                )?;
                out.push(entry);
                pax.clear();
            }
            other => {
                return Err(TarError::new(format!(
                    "unsupported tar entry type {:?} at block {}",
                    other as char,
                    offset / BLOCK
                )));
            }
        }
    }
    if out.is_empty() {
        return Err(TarError::new("tar archive contains no entries"));
    }
    Ok(out)
}

pub(crate) fn index_uncompressed_tar<R: Read + Seek>(
    reader: &mut R,
) -> Result<Vec<SeekTarEntry>, TarError> {
    let mut out = Vec::new();
    let mut offset = reader
        .stream_position()
        .map_err(|err| TarError::new(format!("read tar position: {err}")))?;
    let mut pax: BTreeMap<String, String> = BTreeMap::new();
    let mut long_path: Option<String> = None;
    let mut long_link: Option<String> = None;

    loop {
        let mut header = [0_u8; BLOCK];
        match reader.read_exact(&mut header) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(TarError::new(format!("read tar header: {err}"))),
        }
        let header_offset = offset;
        offset = offset
            .checked_add(BLOCK as u64)
            .ok_or_else(|| TarError::new("tar offset overflow"))?;
        if header == ZERO_BLOCK {
            break;
        }
        verify_checksum(&header, header_offset as usize)?;
        let typeflag = header[156];
        let size = parse_octal(&header[124..136]).ok_or_else(|| {
            TarError::new(format!(
                "tar entry at block {} has invalid size",
                header_offset as usize / BLOCK
            ))
        })?;
        let body_offset = offset;
        let next_offset = next_tar_offset(body_offset, size)?;

        match typeflag {
            b'x' => {
                let body = read_current_body(reader, size)?;
                pax = parse_pax_body(&body)?;
            }
            b'L' => {
                let body = read_current_body(reader, size)?;
                long_path = Some(tar_string(&body));
            }
            b'K' => {
                let body = read_current_body(reader, size)?;
                long_link = Some(tar_string(&body));
            }
            b'0' | 0 | b'5' | b'2' | b'1' | b'6' | b'3' | b'4' => {
                let entry = seek_entry_from_header(
                    &header,
                    typeflag,
                    &pax,
                    &mut long_path,
                    &mut long_link,
                    body_offset,
                    size,
                )?;
                out.push(entry);
                pax.clear();
            }
            other => {
                return Err(TarError::new(format!(
                    "unsupported tar entry type {:?} at block {}",
                    other as char,
                    header_offset as usize / BLOCK
                )));
            }
        }

        reader
            .seek(SeekFrom::Start(next_offset))
            .map_err(|err| TarError::new(format!("seek tar member boundary: {err}")))?;
        offset = next_offset;
    }

    if out.is_empty() {
        return Err(TarError::new("tar archive contains no entries"));
    }
    Ok(out)
}

fn read_current_body<R: Read>(reader: &mut R, size: u64) -> Result<Vec<u8>, TarError> {
    let len = usize::try_from(size)
        .map_err(|_| TarError::new(format!("tar member body too large: {size}")))?;
    let mut body = vec![0_u8; len];
    reader
        .read_exact(&mut body)
        .map_err(|err| TarError::new(format!("read tar member body: {err}")))?;
    Ok(body)
}

fn next_tar_offset(body_offset: u64, size: u64) -> Result<u64, TarError> {
    let padding = (BLOCK as u64 - size % BLOCK as u64) % BLOCK as u64;
    body_offset
        .checked_add(size)
        .and_then(|offset| offset.checked_add(padding))
        .ok_or_else(|| TarError::new("tar offset overflow"))
}

fn raw_entry_from_header<'a>(
    header: &[u8],
    typeflag: u8,
    body: &'a [u8],
    pax: &BTreeMap<String, String>,
    long_path: &mut Option<String>,
    long_link: &mut Option<String>,
) -> Result<RawTarEntry<'a>, TarError> {
    let kind = match typeflag {
        b'0' | 0 => FileEntryKind::File,
        b'5' => FileEntryKind::Directory,
        b'2' => FileEntryKind::Symlink,
        b'1' => FileEntryKind::Hardlink,
        b'6' => FileEntryKind::Fifo,
        b'3' => FileEntryKind::CharDev,
        b'4' => FileEntryKind::BlockDev,
        _ => unreachable!("filtered by caller"),
    };
    let path = pax
        .get("path")
        .cloned()
        .or_else(|| long_path.take())
        .unwrap_or_else(|| header_path(header));
    if path.is_empty() {
        return Err(TarError::new("empty tar path"));
    }
    let link_target = pax
        .get("linkpath")
        .cloned()
        .or_else(|| long_link.take())
        .or_else(|| {
            let link = field_string(&header[157..257]);
            (!link.is_empty()).then_some(link)
        });
    let pax_records = pax
        .iter()
        .filter(|(key, _)| !CORE_PAX_KEYS.contains(&key.as_str()))
        .map(|(key, value)| FilePaxRecord {
            key: key.clone(),
            value: value.clone(),
        })
        .collect();
    Ok(RawTarEntry {
        path: path
            .trim_start_matches("./")
            .trim_end_matches('/')
            .to_string(),
        kind,
        mode: parse_octal(&header[100..108]).map(|value| value as u32),
        modified_seconds: pax
            .get("mtime")
            .and_then(|value| parse_pax_seconds(value))
            .or_else(|| parse_octal(&header[136..148])),
        uid: pax
            .get("uid")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[108..116])),
        gid: pax
            .get("gid")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[116..124])),
        user_name: pax
            .get("uname")
            .cloned()
            .or_else(|| non_empty(field_string(&header[265..297]))),
        group_name: pax
            .get("gname")
            .cloned()
            .or_else(|| non_empty(field_string(&header[297..329]))),
        link_target,
        dev_major: pax
            .get("devmajor")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[329..337])),
        dev_minor: pax
            .get("devminor")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[337..345])),
        pax_records,
        data: if kind == FileEntryKind::File {
            body
        } else {
            &[]
        },
    })
}

fn seek_entry_from_header(
    header: &[u8],
    typeflag: u8,
    pax: &BTreeMap<String, String>,
    long_path: &mut Option<String>,
    long_link: &mut Option<String>,
    body_offset: u64,
    size: u64,
) -> Result<SeekTarEntry, TarError> {
    let kind = match typeflag {
        b'0' | 0 => FileEntryKind::File,
        b'5' => FileEntryKind::Directory,
        b'2' => FileEntryKind::Symlink,
        b'1' => FileEntryKind::Hardlink,
        b'6' => FileEntryKind::Fifo,
        b'3' => FileEntryKind::CharDev,
        b'4' => FileEntryKind::BlockDev,
        _ => unreachable!("filtered by caller"),
    };
    let path = pax
        .get("path")
        .cloned()
        .or_else(|| long_path.take())
        .unwrap_or_else(|| header_path(header));
    if path.is_empty() {
        return Err(TarError::new("empty tar path"));
    }
    let link_target = pax
        .get("linkpath")
        .cloned()
        .or_else(|| long_link.take())
        .or_else(|| {
            let link = field_string(&header[157..257]);
            (!link.is_empty()).then_some(link)
        });
    let pax_records = pax
        .iter()
        .filter(|(key, _)| !CORE_PAX_KEYS.contains(&key.as_str()))
        .map(|(key, value)| FilePaxRecord {
            key: key.clone(),
            value: value.clone(),
        })
        .collect();
    Ok(SeekTarEntry {
        path: path
            .trim_start_matches("./")
            .trim_end_matches('/')
            .to_string(),
        kind,
        mode: parse_octal(&header[100..108]).map(|value| value as u32),
        modified_seconds: pax
            .get("mtime")
            .and_then(|value| parse_pax_seconds(value))
            .or_else(|| parse_octal(&header[136..148])),
        uid: pax
            .get("uid")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[108..116])),
        gid: pax
            .get("gid")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[116..124])),
        user_name: pax
            .get("uname")
            .cloned()
            .or_else(|| non_empty(field_string(&header[265..297]))),
        group_name: pax
            .get("gname")
            .cloned()
            .or_else(|| non_empty(field_string(&header[297..329]))),
        link_target,
        dev_major: pax
            .get("devmajor")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[329..337])),
        dev_minor: pax
            .get("devminor")
            .and_then(|value| value.parse().ok())
            .or_else(|| parse_octal(&header[337..345])),
        pax_records,
        data_offset: body_offset,
        data_size: if kind == FileEntryKind::File { size } else { 0 },
    })
}

fn verify_checksum(header: &[u8], offset: usize) -> Result<(), TarError> {
    let Some(expected) = parse_octal(&header[148..156]) else {
        return Err(TarError::new(format!(
            "tar entry at block {} has invalid checksum",
            offset / BLOCK
        )));
    };
    let mut sum = 0u64;
    for (idx, byte) in header.iter().enumerate() {
        if (148..156).contains(&idx) {
            sum += u64::from(b' ');
        } else {
            sum += u64::from(*byte);
        }
    }
    if sum != expected {
        return Err(TarError::new(format!(
            "tar entry at block {} has checksum {sum:o}, expected {expected:o}",
            offset / BLOCK
        )));
    }
    Ok(())
}

fn parse_octal(field: &[u8]) -> Option<u64> {
    let end = field
        .iter()
        .position(|byte| *byte == 0 || *byte == b' ')
        .unwrap_or(field.len());
    let text = std::str::from_utf8(&field[..end]).ok()?.trim();
    if text.is_empty() {
        Some(0)
    } else {
        u64::from_str_radix(text, 8).ok()
    }
}

fn parse_pax_body(body: &[u8]) -> Result<BTreeMap<String, String>, TarError> {
    let mut out = BTreeMap::new();
    let mut offset = 0usize;
    while offset < body.len() {
        let len_end = body[offset..]
            .iter()
            .position(|byte| *byte == b' ')
            .map(|idx| offset + idx)
            .ok_or_else(|| TarError::new("malformed pax record length"))?;
        let len_text = std::str::from_utf8(&body[offset..len_end])
            .map_err(|_| TarError::new("pax record length is not UTF-8"))?;
        let len: usize = len_text
            .parse()
            .map_err(|_| TarError::new(format!("invalid pax record length: {len_text}")))?;
        let end = offset
            .checked_add(len)
            .filter(|end| *end <= body.len())
            .ok_or_else(|| TarError::new("pax record overruns body"))?;
        let record = std::str::from_utf8(&body[len_end + 1..end])
            .map_err(|_| TarError::new("pax record is not UTF-8"))?;
        let record = record
            .strip_suffix('\n')
            .ok_or_else(|| TarError::new("pax record missing newline"))?;
        let (key, value) = record
            .split_once('=')
            .ok_or_else(|| TarError::new("pax record missing '='"))?;
        out.insert(key.to_string(), value.to_string());
        offset = end;
    }
    Ok(out)
}

fn parse_pax_seconds(value: &str) -> Option<u64> {
    let whole = value.split('.').next().unwrap_or(value);
    whole.parse().ok()
}

fn header_path(header: &[u8]) -> String {
    let name = field_string(&header[0..100]);
    let prefix = field_string(&header[345..500]);
    if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    }
}

fn field_string(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).to_string()
}

fn tar_string(body: &[u8]) -> String {
    String::from_utf8_lossy(body)
        .trim_end_matches('\0')
        .to_string()
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn pax_body_lengths_include_decimal_prefix() {
        let mut records = BTreeMap::new();
        records.insert("path".to_string(), "a/b.ttl".to_string());
        let body = pax_body(&records);
        assert_eq!(std::str::from_utf8(&body).unwrap(), "16 path=a/b.ttl\n");
    }

    #[test]
    fn write_and_read_regular_file() {
        let meta = HeaderMetadata {
            path: "a/b.txt".to_string(),
            link_target: None,
            typeflag: b'0',
            mode: 0o644,
            mtime: 0,
            uid: 0,
            gid: 0,
            size: 5,
            user_name: None,
            group_name: None,
            dev_major: None,
            dev_minor: None,
        };
        let mut tar = Vec::new();
        write_header(&mut tar, &meta, b"hello").unwrap();
        tar.extend(std::iter::repeat_n(0, BLOCK * 2));
        let entries = read_uncompressed_tar(&tar).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "a/b.txt");
        assert_eq!(entries[0].data, b"hello");
    }

    #[test]
    fn write_and_read_pax_long_path() {
        let path = format!("root/{}", "a".repeat(140));
        let entry = FileEntry {
            path: path.clone(),
            kind: FileEntryKind::File,
            data: Some(b"x".to_vec()),
            ..FileEntry::default()
        };
        let mut tar = Vec::new();
        let metadata = HeaderMetadata::from_entry(&entry, 1, &ToTarOptions::default()).unwrap();
        append_pax_header(&mut tar, &entry, &metadata, &ToTarOptions::default()).unwrap();
        write_header(&mut tar, &metadata, b"x").unwrap();
        tar.extend(std::iter::repeat_n(0, BLOCK * 2));
        let entries = read_uncompressed_tar(&tar).unwrap();
        assert_eq!(entries[0].path, path);
    }

    #[test]
    fn core_pax_keys_are_not_round_tripped_as_extension_rows() {
        let mut records = BTreeMap::new();
        records.insert("path".to_string(), "a/b.txt".to_string());
        records.insert("comment".to_string(), "kept".to_string());
        let body = pax_body(&records);
        let pax = parse_pax_body(&body).unwrap();
        let mut long_path = None;
        let mut long_link = None;
        let mut header = [0u8; BLOCK];
        write_path_fields(&mut header, "ignored").unwrap();
        write_octal(&mut header[100..108], 0o644).unwrap();
        write_octal(&mut header[108..116], 0).unwrap();
        write_octal(&mut header[116..124], 0).unwrap();
        write_octal(&mut header[124..136], 0).unwrap();
        write_octal(&mut header[136..148], 0).unwrap();
        header[156] = b'0';
        let entry = raw_entry_from_header(&header, b'0', &[], &pax, &mut long_path, &mut long_link)
            .unwrap();
        assert_eq!(entry.path, "a/b.txt");
        assert_eq!(entry.pax_records.len(), 1);
        assert_eq!(entry.pax_records[0].key, "comment");
    }

    #[test]
    fn all_core_pax_keys_are_unique() {
        let keys: std::collections::BTreeSet<&str> = CORE_PAX_KEYS.iter().copied().collect();
        assert_eq!(keys.len(), CORE_PAX_KEYS.len());
    }

    #[test]
    fn seekable_tar_import_replays_file_blob() {
        let meta = HeaderMetadata {
            path: "a/b.txt".to_string(),
            link_target: None,
            typeflag: b'0',
            mode: 0o644,
            mtime: 0,
            uid: 0,
            gid: 0,
            size: 5,
            user_name: None,
            group_name: None,
            dev_major: None,
            dev_minor: None,
        };
        let mut tar = Vec::new();
        write_header(&mut tar, &meta, b"hello").unwrap();
        tar.extend(std::iter::repeat_n(0, BLOCK * 2));

        let mut cursor = Cursor::new(tar);
        let bytes = crate::from_tar::from_seekable_tar(
            &mut cursor,
            &crate::from_tar::FromTarOptions::default(),
        )
        .unwrap();
        let mut graph = crate::reader::read(&bytes, true, None);
        let entries = read_entries(&graph).unwrap();
        let entry = entries.get("a/b.txt").unwrap();
        assert_eq!(entry.size, Some(5));
        let digest = entry.digest.as_deref().unwrap();
        let blob = graph.blob_bytes(digest).unwrap().unwrap();
        assert_eq!(blob, b"hello");
    }
}

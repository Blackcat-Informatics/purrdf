// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared USTAR (tar) codec — byte-deterministic writer + reader.
//!
//! Consolidates the three hand-rolled copies that previously lived in
//! `crates/pipeline/src/stages/snapshot.rs` (writer + test reader) and
//! `crates/validate/src/data_validate.rs` (reader), keeping a single source of
//! truth for the wire format the snapshot stage writes and the validate path reads.
//!
//! # Wire format
//!
//! The writer produces a byte-deterministic USTAR archive: 512-byte header per
//! member + 512-padded body, terminated by two trailing zero blocks.
//! mtime/uid/gid = 0, mode = 0644.  Member names longer than 100 bytes are
//! preceded by a GNU `'L'` (LongLink) record carrying the full NUL-terminated path.
//!
//! The reader handles both regular (`'0'` / `\0`) and LongLink (`'L'`) records.

/// The GNU long-name sentinel: a `'L'`-typeflag record carrying a member path
/// that overflows the 100-byte USTAR `name` field.
const LONGLINK_NAME: &str = "././@LongLink";

/// Write a byte-deterministic USTAR archive from `members` (name, bytes).
///
/// A member whose name overflows the 100-byte `name` field is preceded by a GNU
/// `'L'` (LongLink) record carrying the full path (NUL-terminated, 512-padded);
/// the real header then truncates the name to 100 bytes (GNU convention — readers
/// take the path from the LongLink). Names ≤ 100 bytes emit no LongLink and are
/// byte-identical to the pre writer, so existing archive blobs are
/// fold-stable.
///
/// # Errors
///
/// Returns `Err(String)` if a USTAR header cannot be constructed (unexpected for
/// well-formed inputs).
pub fn write_archive(members: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    write_archive_borrowed(
        members
            .iter()
            .map(|(name, data)| (name.as_str(), data.as_slice())),
    )
}

/// Write a byte-deterministic USTAR archive from borrowed `(name, bytes)` members.
///
/// This is the allocation-conscious form of [`write_archive`]: package codecs can
/// archive an existing ordered map without cloning every member first. Member order
/// remains caller-defined and therefore observable; callers that require canonical
/// order must supply a sorted iterator.
///
/// # Errors
///
/// Returns `Err(String)` if a USTAR header cannot be constructed.
pub fn write_archive_borrowed<'a>(
    members: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::new();
    for (name, data) in members {
        if name.len() > 100 {
            let mut payload = name.as_bytes().to_vec();
            payload.push(0); // GNU LongLink bodies are NUL-terminated.
            out.extend_from_slice(&ustar_header(LONGLINK_NAME, payload.len(), b'L')?);
            out.extend_from_slice(&payload);
            let pad = (512 - payload.len() % 512) % 512;
            out.extend(std::iter::repeat_n(0u8, pad));
        }
        out.extend_from_slice(&ustar_header(name, data.len(), b'0')?);
        out.extend_from_slice(data);
        let pad = (512 - data.len() % 512) % 512;
        out.extend(std::iter::repeat_n(0u8, pad));
    }
    out.extend(std::iter::repeat_n(0u8, 1024)); // two trailing zero blocks
    Ok(out)
}

/// One borrowed regular-file member from a USTAR archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveMember<'a> {
    /// Member path.
    pub name: &'a str,
    /// Unpadded member body.
    pub data: &'a [u8],
}

/// Allocation-free iterator over regular-file members in a USTAR archive.
#[derive(Debug)]
pub struct ArchiveMembers<'a> {
    tar: &'a [u8],
    offset: usize,
    long_name: Option<&'a str>,
    done: bool,
}

/// Iterate over borrowed regular-file members without cloning their bodies.
///
/// This is the resource-bounded reader seam used by projection packages: callers can
/// validate each path and body length before allocating owned storage.
pub fn archive_members(tar: &[u8]) -> ArchiveMembers<'_> {
    ArchiveMembers {
        tar,
        offset: 0,
        long_name: None,
        done: false,
    }
}

impl<'a> ArchiveMembers<'a> {
    fn fail(&mut self, message: impl Into<String>) -> Option<Result<ArchiveMember<'a>, String>> {
        self.done = true;
        Some(Err(message.into()))
    }
}

impl<'a> Iterator for ArchiveMembers<'a> {
    type Item = Result<ArchiveMember<'a>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done {
                return None;
            }
            let Some(header_end) = self.offset.checked_add(512) else {
                return self.fail("USTAR archive: header offset overflow");
            };
            if header_end > self.tar.len() {
                if self.offset == self.tar.len() {
                    self.done = true;
                    return None;
                }
                return self.fail("USTAR archive: truncated header");
            }

            let tar = self.tar;
            let header = &tar[self.offset..header_end];
            if header.iter().all(|&byte| byte == 0) {
                self.done = true;
                return None;
            }
            let typeflag = header[156];
            let Some(size) = parse_octal(&header[124..136]) else {
                return self.fail("USTAR archive: unreadable size field");
            };
            let Some(body_end) = header_end.checked_add(size) else {
                return self.fail("USTAR archive: member body offset overflow");
            };
            if body_end > tar.len() {
                return self.fail("USTAR archive: member body overruns archive");
            }
            let body = &tar[header_end..body_end];
            let pad = (512 - size % 512) % 512;
            let Some(next_offset) = body_end.checked_add(pad) else {
                return self.fail("USTAR archive: padded member offset overflow");
            };
            if next_offset > tar.len() {
                return self.fail("USTAR archive: member padding overruns archive");
            }
            self.offset = next_offset;

            match typeflag {
                b'L' => {
                    let Some(name_bytes) = body.strip_suffix(&[0]) else {
                        return self.fail("USTAR archive: LongLink name is not NUL-terminated");
                    };
                    let Ok(name) = std::str::from_utf8(name_bytes) else {
                        return self.fail("USTAR archive: LongLink name is not UTF-8");
                    };
                    self.long_name = Some(name);
                }
                b'0' | 0 => {
                    let name = if let Some(name) = self.long_name.take() {
                        name
                    } else {
                        let name_bytes = &header[..100];
                        let end = name_bytes
                            .iter()
                            .position(|&byte| byte == 0)
                            .unwrap_or(name_bytes.len());
                        let Ok(name) = std::str::from_utf8(&name_bytes[..end]) else {
                            return self.fail("USTAR archive: member name is not UTF-8");
                        };
                        name
                    };
                    return Some(Ok(ArchiveMember { name, data: body }));
                }
                _ => {
                    self.long_name = None;
                }
            }
        }
    }
}

/// Read a byte-deterministic USTAR archive, returning owned `(name, bytes)` members.
///
/// Handles regular-file records (`'0'` / `\0`) and GNU LongLink (`'L'`) records.
/// Overflow and bounds guards are enforced. Projection readers should prefer
/// [`archive_members`] so they can apply caller limits before cloning member bodies.
///
/// # Errors
///
/// Returns `Err(String)` if the archive is structurally malformed (unreadable size
/// field or a body that overruns the archive).
pub fn read_archive(tar: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    archive_members(tar)
        .map(|member| {
            let member = member?;
            Ok((member.name.to_owned(), member.data.to_vec()))
        })
        .collect()
}

/// A single USTAR 512-byte header with the given `typeflag` (`b'0'` regular file,
/// `b'L'` GNU LongLink). A name longer than 100 bytes is truncated into the field
/// — the caller MUST have emitted a preceding LongLink record carrying the full
/// path. For a name ≤ 100 bytes the bytes are identical to the pre
/// single-typeflag header.
fn ustar_header(name: &str, size: usize, typeflag: u8) -> Result<[u8; 512], String> {
    let nb = name.as_bytes();
    let n = nb.len().min(100);
    let mut h = [0u8; 512];
    h[..n].copy_from_slice(&nb[..n]);
    write_octal(&mut h[100..108], 0o644); // mode
    write_octal(&mut h[108..116], 0); // uid
    write_octal(&mut h[116..124], 0); // gid
    write_octal(&mut h[124..136], size as u64); // size
    write_octal(&mut h[136..148], 0); // mtime
    for b in &mut h[148..156] {
        *b = b' '; // checksum field is spaces while the sum is computed
    }
    h[156] = typeflag;
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    let sum: u32 = h.iter().map(|&b| u32::from(b)).sum();
    // 6 octal digits, then NUL + space (the canonical checksum encoding).
    let chk = format!("{sum:06o}\0 ");
    h[148..156].copy_from_slice(chk.as_bytes());
    Ok(h)
}

/// Write `value` as right-justified, zero-padded octal into `field`, NUL-terminated.
fn write_octal(field: &mut [u8], value: u64) {
    let width = field.len() - 1;
    let s = format!("{value:0width$o}");
    field[..width].copy_from_slice(&s.as_bytes()[..width]);
    field[width] = 0;
}

/// Parse a NUL/space-padded octal USTAR numeric field.
/// Allocation-free: parses directly from the byte slice.
fn parse_octal(field: &[u8]) -> Option<usize> {
    let end = field
        .iter()
        .position(|&b| b == b'\0' || b == b' ')
        .unwrap_or(field.len());
    let s = std::str::from_utf8(&field[..end]).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    usize::from_str_radix(trimmed, 8).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_octal_reads_zero_padded_nul_terminated_field() {
        // The snapshot writer emits right-justified, zero-padded octal + NUL.
        let mut field = [0u8; 12];
        let octal = b"00000000142"; // 0o142 == 98
        field[..octal.len()].copy_from_slice(octal);
        assert_eq!(parse_octal(&field), Some(0o142));
    }

    #[test]
    fn parse_octal_empty_field_is_zero() {
        assert_eq!(parse_octal(&[0u8; 12]), Some(0));
    }

    #[test]
    fn read_archive_round_trips_a_minimal_archive() {
        // A 1-record USTAR archive: header (name + octal size + '0' typeflag) +
        // 512-padded body + two trailing zero blocks. Mirrors the snapshot writer.
        let name = b"shapes/x.ttl";
        let body = b"@prefix ex: <https://example.org/> .\n";
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name);
        let size_field = format!("{:011o}\0", body.len());
        header[124..136].copy_from_slice(size_field.as_bytes());
        header[156] = b'0';

        let mut tar = Vec::new();
        tar.extend_from_slice(&header);
        tar.extend_from_slice(body);
        tar.extend(std::iter::repeat_n(0u8, (512 - body.len() % 512) % 512));
        tar.extend(std::iter::repeat_n(0u8, 1024));

        let members = read_archive(&tar).expect("read_archive");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].0, "shapes/x.ttl");
        assert_eq!(members[0].1, body);
    }

    #[test]
    fn write_then_read_round_trips_short_names() {
        let members = vec![
            ("shapes/a.ttl".to_string(), b"hello".to_vec()),
            ("shapes/b.ttl".to_string(), b"world".to_vec()),
        ];
        let raw = write_archive(&members).expect("write_archive");
        let got = read_archive(&raw).expect("read_archive");
        assert_eq!(got, members);
    }

    #[test]
    fn write_then_read_round_trips_long_name_via_longlink() {
        // Name > 100 bytes exercises the LongLink path.
        let long = format!(
            "x-purrdf-english/terms/classes/purrdf-{}.html",
            "A".repeat(90)
        );
        assert!(long.len() > 100, "fixture must exceed the 100-byte field");
        let members = vec![
            (long, b"<html>long</html>".to_vec()),
            ("x-purrdf-english/index.html".to_string(), b"idx".to_vec()),
        ];
        let raw = write_archive(&members).expect("write_archive");
        // First record on the wire must be the LongLink sentinel.
        assert_eq!(raw[156], b'L', "first record is a LongLink");
        assert_eq!(&raw[0..LONGLINK_NAME.len()], LONGLINK_NAME.as_bytes());
        let got = read_archive(&raw).expect("read_archive");
        assert_eq!(got, members, "long name must round-trip exactly");
    }

    #[test]
    fn borrowed_reader_rejects_truncated_padding() {
        let raw = write_archive(&[("a".to_owned(), b"body".to_vec())]).expect("archive");
        let truncated = &raw[..700];
        let error = archive_members(truncated)
            .next()
            .expect("one result")
            .expect_err("padding must be complete");
        assert!(error.contains("padding overruns"));
    }
}

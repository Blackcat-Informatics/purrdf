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
/// byte-identical to the pre-#897 writer, so existing archive blobs are
/// fold-stable.
///
/// # Errors
///
/// Returns `Err(String)` if a USTAR header cannot be constructed (unexpected for
/// well-formed inputs).
pub fn write_archive(members: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
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

/// Read a byte-deterministic USTAR archive, returning `(name, bytes)` members.
///
/// Handles regular-file records (`'0'` / `\0`) and GNU LongLink (`'L'`) records.
/// Overflow and bounds guards (`checked_add`, `<= tar.len()`) are enforced.
///
/// # Errors
///
/// Returns `Err(String)` if the archive is structurally malformed (unreadable size
/// field or a body that overruns the archive).
pub fn read_archive(tar: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut i = 0usize;
    let mut long_name: Option<String> = None;

    while i + 512 <= tar.len() {
        let header = &tar[i..i + 512];
        if header.iter().all(|&b| b == 0) {
            break; // trailing zero block(s) — end of archive
        }
        let typeflag = header[156];
        let size = parse_octal(&header[124..136])
            .ok_or_else(|| "USTAR archive: unreadable size field".to_string())?;
        i += 512;
        let body_end = i
            .checked_add(size)
            .filter(|end| *end <= tar.len())
            .ok_or_else(|| "USTAR archive: member body overruns archive".to_string())?;
        let body = &tar[i..body_end];
        // Advance past the 512-padded body.
        i = body_end + (512 - size % 512) % 512;

        match typeflag {
            b'L' => {
                // GNU LongLink: the body is the full path, NUL-terminated.
                let name = String::from_utf8_lossy(body)
                    .trim_end_matches('\0')
                    .to_string();
                long_name = Some(name);
            }
            b'0' | 0 => {
                let name = long_name.take().unwrap_or_else(|| {
                    let nb = &header[0..100];
                    let end = nb.iter().position(|&b| b == 0).unwrap_or(nb.len());
                    String::from_utf8_lossy(&nb[..end]).to_string()
                });
                out.push((name, body.to_vec()));
            }
            _ => {
                // Non-file records (other than LongLink) are not emitted by the
                // writer; skip defensively and clear any pending long name.
                long_name = None;
            }
        }
    }
    Ok(out)
}

/// A single USTAR 512-byte header with the given `typeflag` (`b'0'` regular file,
/// `b'L'` GNU LongLink). A name longer than 100 bytes is truncated into the field
/// — the caller MUST have emitted a preceding LongLink record carrying the full
/// path. For a name ≤ 100 bytes the bytes are identical to the pre-#897
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
}

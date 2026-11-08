// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

mod error;
pub use error::UstarError;

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
/// Shares the typed [`UstarError`] result interface with the readers. Current
/// resident header construction produces no refusal.
pub fn write_archive(members: &[(String, Vec<u8>)]) -> Result<Vec<u8>, UstarError> {
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
/// Shares the typed [`UstarError`] result interface with the readers. Current
/// resident header construction produces no refusal.
pub fn write_archive_borrowed<'a>(
    members: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Result<Vec<u8>, UstarError> {
    let mut out: Vec<u8> = Vec::new();
    for (name, data) in members {
        if name.len() > 100 {
            let mut payload = name.as_bytes().to_vec();
            payload.push(0); // GNU LongLink bodies are NUL-terminated.
            out.extend_from_slice(&ustar_header(LONGLINK_NAME, payload.len(), b'L'));
            out.extend_from_slice(&payload);
            let pad = (512 - payload.len() % 512) % 512;
            out.extend(std::iter::repeat_n(0u8, pad));
        }
        out.extend_from_slice(&ustar_header(name, data.len(), b'0'));
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
    fn fail(&mut self, error: UstarError) -> Option<Result<ArchiveMember<'a>, UstarError>> {
        self.done = true;
        Some(Err(error))
    }
}

impl<'a> Iterator for ArchiveMembers<'a> {
    type Item = Result<ArchiveMember<'a>, UstarError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done {
                return None;
            }
            let header_offset = self.offset;
            let Some(header_end) = header_offset.checked_add(512) else {
                return self.fail(UstarError::HeaderOffsetOverflow {
                    offset: header_offset,
                });
            };
            if header_end > self.tar.len() {
                if self.offset == self.tar.len() {
                    self.done = true;
                    return None;
                }
                return self.fail(UstarError::TruncatedHeader {
                    offset: header_offset,
                    required: 512,
                    available: self.tar.len() - header_offset,
                });
            }

            let tar = self.tar;
            let header = &tar[header_offset..header_end];
            if header.iter().all(|&byte| byte == 0) {
                self.done = true;
                return None;
            }
            let typeflag = header[156];
            let Some(size) = parse_octal(&header[124..136]) else {
                let mut field = [0; 12];
                field.copy_from_slice(&header[124..136]);
                return self.fail(UstarError::InvalidSize {
                    offset: header_offset + 124,
                    field,
                });
            };
            let Some(body_end) = header_end.checked_add(size) else {
                return self.fail(UstarError::BodyOffsetOverflow {
                    offset: header_end,
                    required: size,
                });
            };
            if body_end > tar.len() {
                return self.fail(UstarError::TruncatedBody {
                    offset: header_end,
                    required: size,
                    available: tar.len() - header_end,
                });
            }
            let body = &tar[header_end..body_end];
            let pad = (512 - size % 512) % 512;
            let Some(next_offset) = body_end.checked_add(pad) else {
                return self.fail(UstarError::PaddingOffsetOverflow {
                    offset: body_end,
                    required: pad,
                });
            };
            if next_offset > tar.len() {
                return self.fail(UstarError::TruncatedPadding {
                    offset: body_end,
                    required: pad,
                    available: tar.len() - body_end,
                });
            }
            self.offset = next_offset;

            match typeflag {
                b'L' => {
                    let Some(name_bytes) = body.strip_suffix(&[0]) else {
                        return self.fail(UstarError::UnterminatedLongName {
                            offset: header_end,
                            length: size,
                        });
                    };
                    let name = match std::str::from_utf8(name_bytes) {
                        Ok(name) => name,
                        Err(cause) => {
                            return self.fail(UstarError::LongNameUtf8 {
                                offset: header_end,
                                cause,
                            });
                        }
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
                        match std::str::from_utf8(&name_bytes[..end]) {
                            Ok(name) => name,
                            Err(cause) => {
                                return self.fail(UstarError::NameUtf8 {
                                    offset: header_offset,
                                    cause,
                                });
                            }
                        }
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
/// Returns the original typed refusal if the archive is structurally malformed.
pub fn read_archive(tar: &[u8]) -> Result<Vec<(String, Vec<u8>)>, UstarError> {
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
fn ustar_header(name: &str, size: usize, typeflag: u8) -> [u8; 512] {
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
    h
}

/// Write `value` as right-justified, zero-padded octal into `field`, NUL-terminated.
fn write_octal(field: &mut [u8], value: u64) {
    let width = field.len() - 1;
    let s = format!("{value:0width$o}");
    field[..width].copy_from_slice(&s.as_bytes()[..width]);
    field[width] = 0;
}

/// Parse a NUL/space-padded octal USTAR numeric field.
///
/// The field grammar is POSIX ustar's: *"Each numeric field is terminated by one
/// or more `<space>` or NUL characters."* This reader is the one that sets
/// `body_end` from SIZE, so a field it reads differently from the writer is a
/// framing disagreement — two readers walk the same archive to different member
/// lists, with no diagnostic on either side.
///
/// The law is [`purrdf_gts::tar::parse_octal`], not a second transcription of
/// it. The local spelling this replaced was wrong three ways at once, and the
/// third was the dangerous one: it sliced at the FIRST pad byte, so a
/// left-padded `b"     644"` sliced to empty and read as **zero** — a SIZE of
/// zero truncates the member's body silently. It also discarded every byte
/// after the terminator (`b"644\0x"` read as `0o644`), and reached for
/// `str::trim`, whose Unicode class admits a TAB that ustar does not name as
/// padding (`b"\t644"` read as `0o644`).
///
/// The width conversion is checked rather than cast: `usize` is 32 bits on the
/// wasm32 target this workspace must keep building for, and a silently truncated
/// size is the same framing bug in a new place.
fn parse_octal(field: &[u8]) -> Option<usize> {
    usize::try_from(purrdf_gts::tar::parse_octal(field)?).ok()
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

    /// Every padding shape POSIX ustar admits still reads, including the
    /// LEFT-padded one that the previous spelling silently read as zero. A
    /// zero here is not a wrong number, it is a truncated member body.
    #[test]
    fn parse_octal_reads_every_lawful_padding_shape() {
        for field in [
            &b"0000644\0"[..],
            &b"0000644 "[..],
            &b"     644"[..],
            &b"644\0\0\0"[..],
            &b"644   "[..],
            &b" 644\0"[..],
        ] {
            assert_eq!(parse_octal(field), Some(0o644), "{field:?}");
        }
    }

    /// A non-pad byte AFTER the terminator is a framing disagreement: the
    /// header states a byte this reader would discard, and SIZE decides where
    /// the next header begins. TAB is not ustar padding, whatever the Unicode
    /// whitespace property says.
    #[test]
    fn parse_octal_refuses_data_after_the_terminator_and_unpadded_tabs() {
        for field in [
            &b"644\0x"[..],
            &b"644 x"[..],
            &b"64\08"[..],
            &b"\t644\0"[..],
        ] {
            assert_eq!(parse_octal(field), None, "{field:?}");
        }
    }

    /// The size field feeds `body_end`, so a left-padded size must frame the
    /// member correctly rather than reading as an empty body.
    #[test]
    fn a_left_padded_size_field_frames_the_member_body() {
        let body = b"@prefix ex: <https://example.org/> .\n";
        let mut header = [0u8; 512];
        header[..12].copy_from_slice(b"shapes/x.ttl");
        // Left-padded with spaces rather than zero-filled: lawful ustar, and
        // what several foreign writers emit.
        let size = format!("{:>11o}\0", body.len());
        header[124..136].copy_from_slice(size.as_bytes());
        header[156] = b'0';
        assert_eq!(parse_octal(&header[124..136]), Some(body.len()));
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
        let mut members = archive_members(truncated);
        let error = members
            .next()
            .expect("one result")
            .expect_err("padding must be complete");
        assert_eq!(
            error,
            UstarError::TruncatedPadding {
                offset: 516,
                required: 508,
                available: 184,
            }
        );
        assert!(error.to_string().contains("padding overruns"));
        assert!(members.next().is_none(), "the first refusal ends iteration");
    }

    #[test]
    fn malformed_records_keep_original_fields_and_refusal_priority() {
        let raw = write_archive(&[("a".to_owned(), b"body".to_vec())]).expect("archive");
        let field = *b"00000000008\0";
        let mut invalid_size = raw.clone();
        invalid_size[124..136].copy_from_slice(&field);
        let long_name = "a".repeat(101);
        let mut invalid_long = write_archive(&[(long_name, b"body".to_vec())]).expect("archive");
        invalid_long[512] = 0xff;
        invalid_long[613] = b'x';
        for (bytes, expected) in [
            (
                raw[..100].to_vec(),
                UstarError::TruncatedHeader {
                    offset: 0,
                    required: 512,
                    available: 100,
                },
            ),
            (invalid_size, UstarError::InvalidSize { offset: 124, field }),
            (
                raw[..514].to_vec(),
                UstarError::TruncatedBody {
                    offset: 512,
                    required: 4,
                    available: 2,
                },
            ),
            (
                invalid_long,
                UstarError::UnterminatedLongName {
                    offset: 512,
                    length: 102,
                },
            ),
        ] {
            let mut members = archive_members(&bytes);
            assert_eq!(members.next().expect("refused record"), Err(expected));
            assert!(members.next().is_none(), "a refused archive cannot resume");
        }
        assert_eq!(read_archive(&raw).expect("valid neighbor")[0].1, b"body");
    }

    #[test]
    fn name_refusals_keep_native_causes_after_source_drops_without_inspection_allocation() {
        use purrdf_alloc_probe::CurrentThreadWindow;
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        for long in [false, true] {
            let name = "a".repeat(if long { 101 } else { 1 });
            let offset = if long { 512 } else { 0 };
            let error = {
                let mut raw = write_archive(&[(name, b"body".to_vec())]).expect("archive");
                raw[offset] = 0xff;
                archive_members(&raw)
                    .next()
                    .expect("one record")
                    .expect_err("bad UTF-8")
            };
            let outer_events = Vec::with_capacity(8);
            let native_events = Vec::with_capacity(8);
            let window = CurrentThreadWindow::open();
            let capture = Capture::read_with_buffer(&error, outer_events);
            let ErrorValue::Cause(cause) = capture.field("cause") else {
                panic!("the original UTF-8 refusal must remain a native cause");
            };
            let native = Capture::read_with_buffer(cause, native_events);
            let measured = window.close();
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            assert_eq!(
                capture.code,
                if long {
                    "ustar-long-name-utf8"
                } else {
                    "ustar-name-utf8"
                }
            );
            assert!(
                matches!(capture.field("offset"), ErrorValue::ByteCount(value) if value == offset as u128)
            );
            assert_eq!(native.code, "std-utf8");
            assert!(matches!(
                native.field("valid_up_to"),
                ErrorValue::ByteCount(0)
            ));
            assert!(matches!(
                native.field("error_len"),
                ErrorValue::ByteCount(1)
            ));
            let original = std::error::Error::source(&error)
                .expect("native source")
                .downcast_ref::<core::str::Utf8Error>()
                .expect("original UTF-8 error");
            assert_eq!(original.valid_up_to(), 0);
            assert_eq!(original.error_len(), Some(1));
        }
    }
}

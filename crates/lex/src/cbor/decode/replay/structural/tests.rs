// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;

/// A caller-owned sparse transport whose omitted bodies cannot be read.
struct Sparse {
    bytes: Vec<(u64, u8)>,
    position: u64,
    length: u64,
    reads: Vec<u64>,
    wrong_seek: bool,
    report_low_position: bool,
}

impl Read for Sparse {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let mut count = 0;
        while count < output.len() && self.position < self.length {
            let Some((_, byte)) = self.bytes.iter().find(|(at, _)| *at == self.position) else {
                if count != 0 {
                    break;
                }
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "unselected string body",
                ));
            };
            output[count] = *byte;
            self.reads.push(self.position);
            self.position += 1;
            count += 1;
        }
        Ok(count)
    }
}
impl Seek for Sparse {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.report_low_position
            && !self.reads.is_empty()
            && matches!(position, SeekFrom::Current(0))
        {
            return Ok(0);
        }
        let target = match position {
            SeekFrom::Start(target) => Some(target),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => self.length.checked_add_signed(delta),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek overflow"))?;
        self.position = target;
        Ok(if self.wrong_seek { target + 1 } else { target })
    }
}

fn sparse_array() -> (Sparse, u64) {
    let start = u64::from(u32::MAX) + 17;
    let body = u64::MAX / 2 + 7;
    let mut head = vec![0x82, 0x7b];
    head.extend_from_slice(&body.to_be_bytes());
    let mut bytes: Vec<_> = head
        .into_iter()
        .enumerate()
        .map(|(index, byte)| (start + index as u64, byte))
        .collect();
    let last = start + bytes.len() as u64 + body;
    bytes.push((last, 7));
    (
        Sparse {
            bytes,
            position: 0,
            length: last + 1,
            reads: Vec::new(),
            wrong_seek: false,
            report_low_position: false,
        },
        start,
    )
}

#[test]
fn structural_ranges_cross_wide_unread_text_bodies_without_admitting_them() {
    let (mut source, start) = sparse_array();
    let length = source.length;
    let range = StructuralRange::scan(&mut source, start, length, Limits::default())
        .expect("wide structure")
        .expect("one array");
    assert_eq!(range.start(), start);
    assert_eq!(range.end(), length);
    assert_eq!(
        source.reads,
        source.bytes.iter().map(|(at, _)| *at).collect::<Vec<_>>()
    );
    let mut items = range
        .array(&mut source)
        .expect("array head")
        .expect("array");
    let text = items.next(&mut source).expect("text extent").expect("text");
    assert_eq!(text.major(&mut source).expect("text head"), 3);
    assert!(
        text.decode(&mut source).is_err(),
        "structural navigation cannot admit unread bytes"
    );
    let integer = items
        .next(&mut source)
        .expect("integer extent")
        .expect("integer");
    assert_eq!(
        integer.decode(&mut source).expect("selected metadata"),
        Value::from(7_u8)
    );
    assert_eq!(
        integer
            .encoded(&mut source)
            .expect("admitted encoded selected metadata")
            .view()
            .encoded(),
        &[7]
    );
    assert!(items.next(&mut source).expect("array end").is_none());
}

#[test]
fn structural_text_is_validated_only_when_selected_for_decode() {
    let bytes = [0xd9, 0xd9, 0xf7, 0x81, 0x61, 0xff];
    let mut source = io::Cursor::new(bytes);
    let tagged = StructuralRange::scan(&mut source, 0, 6, Limits::default())
        .expect("structural tagged array")
        .expect("tag");
    let (tag, range) = tagged
        .tag(&mut source)
        .expect("structural tag")
        .expect("tagged child");
    assert_eq!(tag, 55799);
    assert_eq!(range.start(), 3);
    assert_eq!(range.end(), 6);
    let text = range
        .array(&mut source)
        .expect("array head")
        .expect("array")
        .next(&mut source)
        .expect("child extent")
        .expect("text");
    let error = text
        .decode(&mut source)
        .expect_err("selected text must validate UTF-8");
    assert_eq!(error.kind(), DecodeErrorKind::InvalidUtf8);
    assert_eq!(error.offset(), 4);
    let encoded = text
        .encoded(&mut source)
        .expect_err("encoded ownership must also validate selected UTF-8");
    assert_eq!(encoded.kind(), DecodeErrorKind::InvalidUtf8);
    assert_eq!(encoded.offset(), 4);
    assert_eq!(
        super::super::super::super::decode(&bytes, Limits::default())
            .expect_err("ordinary admission still validates")
            .offset(),
        4
    );
}

#[test]
fn structural_navigation_refuses_truncation_wrong_chunk_types_and_seek_positions() {
    let (mut source, start) = sparse_array();
    let end = source.length - 2;
    let error = StructuralRange::scan(&mut source, start, end, Limits::default())
        .expect_err("a skipped body must remain within the pinned extent");
    assert_eq!(error.kind(), DecodeErrorKind::Truncated);
    assert_eq!(error.offset(), end);

    let mut chunks = io::Cursor::new([0x7f, 0x41, 0x00, 0xff]);
    let error = StructuralRange::scan(&mut chunks, 0, 4, Limits::default())
        .expect_err("text chunks cannot be byte strings");
    assert!(matches!(error.kind(), DecodeErrorKind::Malformed(_)));
    assert_eq!(error.offset(), 1);

    let (mut source, start) = sparse_array();
    source.wrong_seek = true;
    let length = source.length;
    let error = StructuralRange::scan(&mut source, start, length, Limits::default())
        .expect_err("seek return positions are checked");
    assert!(
        matches!(error.kind(), DecodeErrorKind::Io(cause) if cause.kind() == io::ErrorKind::InvalidData)
    );
    assert_eq!(error.offset(), start);
    assert!(source.reads.is_empty());
}

#[test]
fn buffered_navigation_checks_actual_relative_seek_and_preserves_cached_rewinds() {
    struct Shifted(io::Cursor<[u8; 8]>);
    impl Read for Shifted {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            self.0.read(output)
        }
    }
    impl Seek for Shifted {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            let position = match position {
                SeekFrom::Current(delta) if delta != 0 => SeekFrom::Current(delta + 1),
                position => position,
            };
            self.0.seek(position)
        }
    }
    let mut shifted = super::super::ReplayBuffer::new(Shifted(io::Cursor::new([0; 8])))
        .expect("initial position");
    let error = StructuralRange::scan(&mut shifted, 2, 8, Limits::default())
        .expect_err("a buffered relative seek must not claim the requested position");
    assert!(
        matches!(error.kind(), DecodeErrorKind::Io(cause) if cause.kind() == io::ErrorKind::InvalidData)
    );
    assert_eq!(error.offset(), 2);

    let source = Sparse {
        bytes: vec![(0, 0x82), (1, 1), (2, 2)],
        position: 0,
        length: 3,
        reads: Vec::new(),
        wrong_seek: false,
        report_low_position: false,
    };
    let mut buffered = super::super::ReplayBuffer::new(source).expect("source position");
    let range = StructuralRange::scan(&mut buffered, 0, 3, Limits::default())
        .expect("prefetched structure")
        .expect("array");
    let mut children = range
        .array(&mut buffered)
        .expect("cached rewind")
        .expect("array");
    for expected in [1_u8, 2] {
        let child = children
            .next(&mut buffered)
            .expect("cached child")
            .expect("item");
        assert_eq!(
            child.decode(&mut buffered).expect("cached metadata"),
            Value::from(expected)
        );
    }
    assert!(children.next(&mut buffered).expect("array end").is_none());
    let mut source = buffered.into_inner();
    assert_eq!(source.reads, [0, 1, 2], "prefetched bytes are counted once");
    source.position = 0;
    source.reads.clear();
    source.report_low_position = true;
    let mut buffered = super::super::ReplayBuffer::new(source).expect("initial raw position");
    let range = StructuralRange::scan(&mut buffered, 0, 3, Limits::default())
        .expect("prefetched structure")
        .expect("array");
    let error = range
        .array(&mut buffered)
        .expect_err("a physical position below unread bytes must fail without panicking");
    assert!(
        matches!(error.kind(), DecodeErrorKind::Io(cause) if cause.kind() == io::ErrorKind::InvalidData)
    );
}

#![allow(
    clippy::cast_lossless,
    clippy::map_unwrap_or,
    clippy::unreadable_literal,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use crate::common::{MAGIC_NUM, MAX_WINDOW_SIZE, MIN_WINDOW_SIZE};
use crate::decoding::errors::{FrameDescriptorError, FrameHeaderError, ReadFrameHeaderError};
use crate::io::Read;

/// Read a single serialized frame header. When `magicless` is
/// `true`, the 4-byte magic prefix is NOT consumed and skippable-
/// frame detection is bypassed — the caller MUST know out-of-band
/// that the stream is magicless. Upstream zstd parity:
/// `ZSTD_f_zstd1_magicless` via `ZSTD_d_format`.
#[inline]
pub(crate) fn read_frame_header_with_format(
    mut r: impl Read,
    magicless: bool,
) -> Result<(FrameHeader, u8), ReadFrameHeaderError> {
    use ReadFrameHeaderError as err;
    let mut buf = [0u8; 4];

    let mut bytes_read = 0;
    if !magicless {
        r.read_exact(&mut buf).map_err(err::MagicNumberReadError)?;
        bytes_read = 4;
        let magic_num = u32::from_le_bytes(buf);

        // Skippable frames have a magic number in this interval
        if (0x184D2A50..=0x184D2A5F).contains(&magic_num) {
            r.read_exact(&mut buf)
                .map_err(err::FrameDescriptorReadError)?;
            let skip_size = u32::from_le_bytes(buf);
            return Err(ReadFrameHeaderError::SkipFrame {
                magic_number: magic_num,
                length: skip_size,
            });
        }

        if magic_num != MAGIC_NUM {
            return Err(ReadFrameHeaderError::BadMagicNumber(magic_num));
        }
    }

    r.read_exact(&mut buf[0..1])
        .map_err(err::FrameDescriptorReadError)?;
    let desc = FrameDescriptor(buf[0]);

    bytes_read += 1;

    let mut frame_header = FrameHeader {
        descriptor: FrameDescriptor(desc.0),
        dict_id: None,
        frame_content_size: 0,
        window_descriptor: 0,
    };

    // Each variable header field is read with its own field-specific error so
    // a truncated frame reports which field is missing. The slice `read_exact`
    // override (`io_nostd`) makes each of these a single bounds-checked copy,
    // so the small per-field reads are not the cost they once were.
    if !desc.single_segment_flag() {
        r.read_exact(&mut buf[0..1])
            .map_err(err::WindowDescriptorReadError)?;
        frame_header.window_descriptor = buf[0];
        bytes_read += 1;
    }

    let dict_id_len = desc.dictionary_id_bytes()? as usize;
    if dict_id_len != 0 {
        let buf = &mut buf[..dict_id_len];
        r.read_exact(buf).map_err(err::DictionaryIdReadError)?;
        bytes_read += dict_id_len;
        let mut dict_id = 0u32;
        for (i, &b) in buf.iter().enumerate() {
            dict_id += (b as u32) << (8 * i);
        }
        if dict_id != 0 {
            frame_header.dict_id = Some(dict_id);
        }
    }

    let fcs_len = desc.frame_content_size_bytes()? as usize;
    if fcs_len != 0 {
        let mut fcs_buf = [0u8; 8];
        let fcs_buf = &mut fcs_buf[..fcs_len];
        r.read_exact(fcs_buf)
            .map_err(err::FrameContentSizeReadError)?;
        bytes_read += fcs_len;
        let mut fcs = 0u64;
        for (i, &b) in fcs_buf.iter().enumerate() {
            fcs += (b as u64) << (8 * i);
        }
        if fcs_len == 2 {
            fcs += 256;
        }
        frame_header.frame_content_size = fcs;
    }

    Ok((frame_header, bytes_read as u8))
}

/// Slice-direct equivalent of [`read_frame_header_with_format`]: parses the
/// header straight out of `input` with byte indexing + `from_le_bytes`, instead
/// of per-field `read_exact` calls through the `Read` trait, and advances
/// `*input` past the consumed bytes EXACTLY as the `Read` version does — so the
/// skippable-frame and truncation contracts are byte-identical. The decode path
/// already holds a `&[u8]`, so this avoids the `io::impls` `Read`-trait
/// dispatch the generic version pays per field, mirroring upstream zstd, which
/// parses the header from a raw pointer (`MEM_readLE32` / byte reads).
pub(crate) fn read_frame_header_from_slice(
    input: &mut &[u8],
    magicless: bool,
) -> Result<(FrameHeader, u8), ReadFrameHeaderError> {
    use ReadFrameHeaderError as err;
    fn eof() -> crate::io::Error {
        crate::io::Error::from(crate::io::ErrorKind::UnexpectedEof)
    }
    fn take<'a>(input: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
        if input.len() < n {
            return None;
        }
        let (head, tail) = input.split_at(n);
        *input = tail;
        Some(head)
    }

    let mut bytes_read: u8 = 0;
    if !magicless {
        let m = take(input, 4).ok_or_else(|| err::MagicNumberReadError(eof()))?;
        let magic_num = u32::from_le_bytes([m[0], m[1], m[2], m[3]]);
        bytes_read = 4;
        if (0x184D2A50..=0x184D2A5F).contains(&magic_num) {
            let s = take(input, 4).ok_or_else(|| err::FrameDescriptorReadError(eof()))?;
            let skip_size = u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
            return Err(ReadFrameHeaderError::SkipFrame {
                magic_number: magic_num,
                length: skip_size,
            });
        }
        if magic_num != MAGIC_NUM {
            return Err(ReadFrameHeaderError::BadMagicNumber(magic_num));
        }
    }

    let d = take(input, 1).ok_or_else(|| err::FrameDescriptorReadError(eof()))?;
    let desc = FrameDescriptor(d[0]);
    bytes_read += 1;

    let mut frame_header = FrameHeader {
        descriptor: FrameDescriptor(desc.0),
        dict_id: None,
        frame_content_size: 0,
        window_descriptor: 0,
    };

    if !desc.single_segment_flag() {
        let w = take(input, 1).ok_or_else(|| err::WindowDescriptorReadError(eof()))?;
        frame_header.window_descriptor = w[0];
        bytes_read += 1;
    }

    let dict_id_len = desc.dictionary_id_bytes()? as usize;
    if dict_id_len != 0 {
        let b = take(input, dict_id_len).ok_or_else(|| err::DictionaryIdReadError(eof()))?;
        bytes_read += dict_id_len as u8;
        let mut buf4 = [0u8; 4];
        buf4[..dict_id_len].copy_from_slice(b);
        let dict_id = u32::from_le_bytes(buf4);
        if dict_id != 0 {
            frame_header.dict_id = Some(dict_id);
        }
    }

    let fcs_len = desc.frame_content_size_bytes()? as usize;
    if fcs_len != 0 {
        let b = take(input, fcs_len).ok_or_else(|| err::FrameContentSizeReadError(eof()))?;
        bytes_read += fcs_len as u8;
        let mut buf8 = [0u8; 8];
        buf8[..fcs_len].copy_from_slice(b);
        let mut fcs = u64::from_le_bytes(buf8);
        if fcs_len == 2 {
            fcs += 256;
        }
        frame_header.frame_content_size = fcs;
    }

    Ok((frame_header, bytes_read))
}

/// A frame header has a variable size, with a minimum of 2 bytes, and a maximum of 14 bytes.
pub(crate) struct FrameHeader {
    pub descriptor: FrameDescriptor,
    /// The `Window_Descriptor` field contains the minimum size of a memory buffer needed to
    /// decompress the entire frame.
    ///
    /// This byte is not included in the frame header when the `Single_Segment_flag` is set.
    ///
    /// Bits 7-3 refer to the `Exponent`, where bits 2-0 refer to the `Mantissa`.
    ///
    /// To determine the size of a window, the following formula can be used:
    /// ```text
    /// windowLog = 10 + Exponent;
    /// windowBase = 1 << windowLog;
    /// windowAdd = (windowBase / 8) * Mantissa;
    /// Window_Size = windowBase + windowAdd;
    /// ```
    /// <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#window_descriptor>
    window_descriptor: u8,
    /// The `Dictionary_ID` field contains the ID of the dictionary to be used to decode the frame.
    /// When this value is not present, it's up to the decoder to know which dictionary to use.
    dict_id: Option<u32>,
    /// The size of the original/uncompressed content.
    frame_content_size: u64,
}

impl FrameHeader {
    /// Read the size of the window from the header or the total frame content size,
    /// whichever is defined, returning the size in bytes.
    pub(crate) fn window_size(&self) -> Result<u64, FrameHeaderError> {
        if self.descriptor.single_segment_flag() {
            Ok(self.frame_content_size())
        } else {
            let exp = self.window_descriptor >> 3;
            let mantissa = self.window_descriptor & 0x7;

            let window_log = 10 + u64::from(exp);
            let window_base = 1 << window_log;
            let window_add = (window_base / 8) * u64::from(mantissa);

            let window_size = window_base + window_add;

            if window_size >= MIN_WINDOW_SIZE {
                if window_size < MAX_WINDOW_SIZE {
                    Ok(window_size)
                } else {
                    Err(FrameHeaderError::WindowTooBig { got: window_size })
                }
            } else {
                Err(FrameHeaderError::WindowTooSmall { got: window_size })
            }
        }
    }

    /// The ID (if provided) of the dictionary required to decode this frame.
    pub(crate) fn dictionary_id(&self) -> Option<u32> {
        self.dict_id
    }

    /// Obtain the uncompressed size (in bytes) of the frame contents.
    pub(crate) fn frame_content_size(&self) -> u64 {
        self.frame_content_size
    }

    /// Whether the frame header carried an explicit `Frame_Content_Size`
    /// field on the wire. Distinguishes "FCS absent" (FCS_flag=0 +
    /// `Single_Segment_flag=0`) from "FCS=0 explicitly declared"
    /// (FCS_flag>=1 with a zero value, or FCS_flag=0 +
    /// `Single_Segment_flag=1` with a 1-byte FCS=0). Both leave
    /// [`Self::frame_content_size`] at `0`, so [`Self::frame_content_size`]
    /// alone cannot distinguish the two; callers that need to know
    /// whether the value is actually a wire-format declaration (e.g.
    /// for post-decode size validation) should consult this method.
    #[allow(dead_code)]
    pub(crate) fn fcs_declared(&self) -> bool {
        // `frame_content_size_bytes()` returns 0 only when FCS_flag=0
        // AND single_segment_flag=0 — exactly the "no FCS on the wire"
        // case. Any other combination (FCS_flag in 1..=3, or
        // single_segment_flag set) writes a non-zero number of bytes.
        // The descriptor was already validated when the header was
        // parsed; if `frame_content_size_bytes()` is `Err` here the
        // header object should not exist, so unwrap_or(0) collapses
        // the (unreachable) error into the safe "treat as absent"
        // answer.
        self.descriptor
            .frame_content_size_bytes()
            .map(|n| n != 0)
            .unwrap_or(false)
    }

    /// Raw `Window_Descriptor` byte from the frame header
    /// (RFC 8878 §3.1.1.1.2 layout: `(exp << 3) | mantissa`),
    /// or `None` when the `Single_Segment_flag` is set — in
    /// single-segment frames the byte is absent from the wire
    /// (the `Window_Size` is derived from `Frame_Content_Size`
    /// instead). The parser leaves the struct's
    /// `window_descriptor` field at its default `0` in that case;
    /// this accessor reports the absence via `None` so callers
    /// don't conflate "missing byte on the wire" with "byte
    /// present and equal to 0".
    ///
    /// `frame` module is `pub(crate)`, so this method is reachable
    /// only from in-crate validation paths (e.g. the `lsm` feature's
    /// `expect_window_descriptor` setter); `allow(dead_code)` keeps
    /// default builds warning-free when no in-crate caller invokes
    /// it.
    #[allow(dead_code)]
    pub(crate) fn window_descriptor(&self) -> Option<u8> {
        if self.descriptor.single_segment_flag() {
            None
        } else {
            Some(self.window_descriptor)
        }
    }
}

/// The first byte is called the `Frame Header Descriptor`, and it describes what other fields
/// are present.
pub(crate) struct FrameDescriptor(pub u8);

impl FrameDescriptor {
    /// Read the `Frame_Content_Size_flag` from the frame header descriptor.
    ///
    /// This is a 2 bit flag, specifying if the `Frame_Content_Size` field is present
    /// within the header. It notates the number of bytes used by `Frame_Content_size`
    ///
    /// When this value is is 0, `FCS_Field_Size` depends on Single_Segment_flag.
    /// If the `Single_Segment_flag` field is set in the frame header descriptor,
    /// the size of the `Frame_Content_Size` field of the header is 1 byte.
    /// Otherwise, `FCS_Field_Size` is 0, and the `Frame_Content_Size` is not provided.
    ///
    /// | Flag Value (decimal) | Size of the `Frame_Content_Size` field in bytes |
    /// | -- | -- |
    /// | 0 | 0 or 1 (see above) |
    /// | 1 | 2 |
    /// | 2 | 4 |
    /// | 3 | 8 |
    pub(crate) fn frame_content_size_flag(&self) -> u8 {
        self.0 >> 6
    }

    /// This bit is reserved for some future feature, a compliant decoder **must ensure**
    /// that this value is set to zero.
    #[expect(dead_code)]
    pub(crate) fn reserved_flag(&self) -> bool {
        ((self.0 >> 3) & 0x1) == 1
    }

    /// If this flag is set, data must be regenerated within a single continuous memory segment.
    ///
    /// In this case, the `Window_Descriptor` byte is skipped, but `Frame_Content_Size` is present.
    /// The decoder must allocate a memory segment equal to or larger than `Frame_Content_Size`.
    pub(crate) fn single_segment_flag(&self) -> bool {
        ((self.0 >> 5) & 0x1) == 1
    }

    /// If this flag is set, a 32 bit `Content_Checksum` will be present at the end of the frame.
    pub(crate) fn content_checksum_flag(&self) -> bool {
        ((self.0 >> 2) & 0x1) == 1
    }

    /// This is a two bit flag telling if a dictionary ID is provided within the header. It also
    /// specifies the size of this field
    ///
    /// | Value (Decimal) | `DID_Field_Size` (bytes) |
    /// | -- | -- |
    /// | 0 | 0 |
    /// | 1 | 1 |
    /// | 2 | 2 |
    /// | 3 | 4 |
    pub(crate) fn dict_id_flag(&self) -> u8 {
        self.0 & 0x3
    }

    /// Read the size of the `Frame_Content_size` field from the frame header descriptor, returning
    /// the size in bytes.
    /// If this value is zero, then the `Frame_Content_Size` field is not present within the header.
    pub(crate) fn frame_content_size_bytes(&self) -> Result<u8, FrameDescriptorError> {
        match self.frame_content_size_flag() {
            0 => {
                if self.single_segment_flag() {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            1 => Ok(2),
            2 => Ok(4),
            3 => Ok(8),
            other => Err(FrameDescriptorError::InvalidFrameContentSizeFlag { got: other }),
        }
    }

    /// Read the size of the `Dictionary_ID` field from the frame header descriptor, returning the size in bytes.
    /// If this value is zero, then the dictionary id is not present within the header,
    /// and "It's up to the decoder to know which dictionary to use."
    pub(crate) fn dictionary_id_bytes(&self) -> Result<u8, FrameDescriptorError> {
        match self.dict_id_flag() {
            0 => Ok(0),
            1 => Ok(1),
            2 => Ok(2),
            3 => Ok(4),
            other => Err(FrameDescriptorError::InvalidFrameContentSizeFlag { got: other }),
        }
    }
}

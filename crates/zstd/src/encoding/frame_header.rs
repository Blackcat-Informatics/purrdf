#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Utilities and representations for a frame header.
use crate::common::MAGIC_NUM;
use crate::encoding::util::{find_fcs_field_size, find_min_size, write_minified_val};
use alloc::vec::Vec;

/// A header for a single Zstandard frame.
///
/// <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#frame_header>
///
/// The containing module is `pub(crate)`, so this struct is not
/// externally constructible. The `Default` derive is kept as a
/// convenience for tests and for internal callers that only need
/// to override specific fields.
#[derive(Debug, Default)]
pub(crate) struct FrameHeader {
    /// Optionally, the original (uncompressed) size of the data within the frame in bytes.
    /// If not present, `window_size` must be set.
    pub frame_content_size: Option<u64>,
    /// If set to true, data must be regenerated within a single
    /// continuous memory segment.
    pub single_segment: bool,
    /// If set to true, a 32 bit content checksum will be present
    /// at the end of the frame.
    pub content_checksum: bool,
    /// If a dictionary ID is provided, the ID of that dictionary.
    pub dictionary_id: Option<u64>,
    /// The minimum memory buffer required to compress a frame. If not present,
    /// `single_segment` will be set to true. If present, this value must be greater than 1KB
    /// and less than 3.75TB. Encoders should not generate a frame that requires a window size larger than
    /// 8mb.
    pub window_size: Option<u64>,
    /// If true, the 4-byte magic number prefix is omitted from the
    /// serialized output. The caller MUST know out-of-band that the
    /// stream is magicless and use a magicless-aware decoder.
    /// Upstream zstd parity: `ZSTD_f_zstd1_magicless` (see `ZSTD_d_format`).
    pub magicless: bool,
}

impl FrameHeader {
    /// Writes the serialized frame header into the provided buffer.
    ///
    /// The returned header *does include* a frame header descriptor.
    pub(crate) fn serialize(self, output: &mut Vec<u8>) {
        vprintln!("Serializing frame with header: {self:?}");
        // https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#frame_header
        // Magic Number (omitted in magicless mode — `ZSTD_f_zstd1_magicless`):
        if !self.magicless {
            output.extend_from_slice(&MAGIC_NUM.to_le_bytes());
        }

        // `Frame_Header_Descriptor`:
        output.push(self.descriptor());

        // `Window_Descriptor
        // TODO: https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#window_descriptor
        if !self.single_segment
            && let Some(window_size) = self.window_size
        {
            let log = window_size.next_power_of_two().ilog2();
            let exponent = if log > 10 { log - 10 } else { 1 } as u8;
            output.push(exponent << 3);
        }

        if let Some(id) = self.dictionary_id {
            write_minified_val(id, output);
        }

        if let Some(frame_content_size) = self.frame_content_size {
            let field_size = find_fcs_field_size(frame_content_size, self.single_segment);
            write_fcs(frame_content_size, field_size, output);
        }
    }

    /// Generate a serialized frame header descriptor for the frame header.
    ///
    /// https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#frame_header_descriptor
    fn descriptor(&self) -> u8 {
        // A frame header starts with a frame header descriptor.
        // It describes what other fields are present
        // https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#frame_header_descriptor
        // Built with plain bit arithmetic (LSB-first field order below); a
        // `BitWriter` here allocated a fresh `Vec` for the single byte on
        // every frame header.

        // Bits 0-1, `Dictionary_ID_flag`: size class of the
        // `Dictionary_ID` field (0 = absent).
        let dict_id_flag: u8 = match self.dictionary_id.map(find_min_size) {
            None => 0,
            Some(1) => 1,
            Some(2) => 2,
            Some(4) => 3,
            _ => panic!(),
        };

        // Bit 2, `Content_Checksum_flag`.
        let checksum_flag = u8::from(self.content_checksum);

        // Bits 3-4: `Reserved_bit` + `Unused_bit`, both zero.

        // Bit 5, `Single_Segment_flag`:
        // If this flag is set, data must be regenerated within a single continuous memory segment,
        // and the `Frame_Content_Size` field must be present in the header.
        // If this flag is not set, the `Window_Descriptor` field must be present in the frame header.
        if self.single_segment {
            assert!(
                self.frame_content_size.is_some(),
                "if the `single_segment` flag is set to true, then a frame content size must be provided"
            );
        } else {
            assert!(
                self.window_size.is_some(),
                "if the `single_segment` flag is set to false, then a window size must be provided"
            );
        }
        let single_segment_flag = u8::from(self.single_segment);

        // Bits 6-7, `Frame_Content_Size_flag`: size class of the FCS field.
        // If the `Single_Segment_flag` is set and this value is zero,
        // the size of the FCS field is 1 byte; otherwise the FCS field is
        // omitted when the flag is zero.
        // | Value | Size of field (Bytes)
        // | 0     | 0 or 1
        // | 1     | 2
        // | 2     | 4
        // | 3     | 8
        let fcs_flag: u8 = match self.frame_content_size {
            Some(frame_content_size) => {
                match find_fcs_field_size(frame_content_size, self.single_segment) {
                    1 => 0,
                    2 => 1,
                    4 => 2,
                    8 => 3,
                    _ => unreachable!(),
                }
            }
            None => 0,
        };

        dict_id_flag | (checksum_flag << 2) | (single_segment_flag << 5) | (fcs_flag << 6)
    }
}

/// Serialize a `Frame_Content_Size` value into `field_size` bytes.
///
/// For 1, 4, and 8-byte fields the value is stored directly.
/// For 2-byte fields an offset of 256 is subtracted before encoding.
///
/// <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#frame_content_size>
fn write_fcs(val: u64, field_size: usize, output: &mut Vec<u8>) {
    debug_assert!(matches!(field_size, 1 | 2 | 4 | 8));
    debug_assert!(field_size != 2 || val >= 256);

    let adjusted = match field_size {
        2 => val - 256,
        1 | 4 | 8 => val,
        _ => unreachable!("invalid Frame_Content_Size field size: {field_size}"),
    };
    output.extend_from_slice(&adjusted.to_le_bytes()[..field_size]);
}

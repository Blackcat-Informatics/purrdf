#![allow(
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::convert::TryInto;

use crate::decoding::errors::DictionaryDecodeError;
use crate::decoding::scratch::FSEScratch;
use crate::decoding::scratch::HuffmanScratch;

/// Zstandard includes support for "raw content" dictionaries, that store bytes optionally used
/// during sequence execution.
///
/// <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#dictionary-format>
#[derive(Clone)]
pub struct Dictionary {
    /// A 4 byte value used by decoders to check if they can use
    /// the correct dictionary. This value must not be zero.
    pub id: u32,
    /// A dictionary can contain an entropy table, either FSE or
    /// Huffman.
    pub fse: FSEScratch,
    /// A dictionary can contain an entropy table, either FSE or
    /// Huffman.
    pub huf: HuffmanScratch,
    /// The content of a dictionary acts as a "past" in front of data
    /// to compress or decompress,
    /// so it can be referenced in sequence commands.
    /// As long as the amount of data decoded from this frame is less than or
    /// equal to Window_Size, sequence commands may specify offsets longer than
    /// the total length of decoded output so far to reference back to the
    /// dictionary, even parts of the dictionary with offsets larger than Window_Size.
    /// After the total output has surpassed Window_Size however,
    /// this is no longer allowed and the dictionary is no longer accessible
    pub dict_content: Vec<u8>,
    /// The 3 most recent offsets are stored so that they can be used
    /// during sequence execution, see
    /// <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#repeat-offsets>
    /// for more.
    pub offset_hist: [u32; 3],
}

#[cfg(target_has_atomic = "ptr")]
type SharedDictionary = Arc<Dictionary>;
#[cfg(not(target_has_atomic = "ptr"))]
type SharedDictionary = Rc<Dictionary>;

/// Shared pre-parsed dictionary handle for repeated decoding.
///
/// Uses `Arc` on targets with atomics and falls back to `Rc` otherwise.
#[derive(Clone)]
pub struct DictionaryHandle {
    inner: SharedDictionary,
}

/// This 4 byte (little endian) magic number refers to the start of a dictionary
pub(crate) const MAGIC_NUM: [u8; 4] = [0x37, 0xA4, 0x30, 0xEC];

impl Dictionary {
    /// Heap bytes owned by this dictionary: the content plus the parsed
    /// entropy tables' heap (the fixed-size FSE decode arrays are inline,
    /// counted by `size_of::<Dictionary>()`).
    pub fn heap_bytes(&self) -> usize {
        self.dict_content.capacity() + self.fse.heap_bytes() + self.huf.heap_bytes()
    }

    /// Build a dictionary from raw content bytes (without entropy table sections).
    ///
    /// This is primarily intended for dictionaries produced by the `dict_builder`
    /// module, which currently emits raw-content dictionaries.
    pub fn from_raw_content(
        id: u32,
        dict_content: Vec<u8>,
    ) -> Result<Dictionary, DictionaryDecodeError> {
        if id == 0 {
            return Err(DictionaryDecodeError::ZeroDictionaryId);
        }
        if dict_content.is_empty() {
            return Err(DictionaryDecodeError::DictionaryTooSmall { got: 0, need: 1 });
        }

        Ok(Dictionary {
            id,
            fse: FSEScratch::new(),
            huf: HuffmanScratch::new(),
            dict_content,
            offset_hist: [1, 4, 8],
        })
    }

    /// Parses the dictionary from `raw`, initializes its tables,
    /// and returns a fully constructed [`Dictionary`] whose `id` can be
    /// checked against the frame's `dict_id`.
    pub fn decode_dict(raw: &[u8]) -> Result<Dictionary, DictionaryDecodeError> {
        Self::decode_dict_inner(raw, true)
    }

    /// Parse a dictionary for ENCODER use: builds the entropy
    /// probabilities/weights needed by `to_encoder_table` but skips the
    /// decode-only work the encoder never reads — the FSE *decoding*
    /// tables + their `enrich_*` post-passes, and the HUF decode lookup
    /// table (`packed_decode`). Produces a [`Dictionary`] whose FSE
    /// `symbol_probabilities` / `accuracy_log` and HUF `bits` /
    /// `max_num_bits` match `decode_dict` exactly, so the encoder entropy
    /// tables — and thus the emitted frame — are byte-identical; only the
    /// wasted decode-table builds are dropped. Offset history + content
    /// are parsed the same way.
    /// Crate-internal: the returned [`Dictionary`] deliberately has no
    /// decode lookup tables (`packed_decode` / FSE `decode`), so it is
    /// NOT safe to feed into a [`FrameDecoder`](crate::decoding::FrameDecoder)
    /// — Huffman decode would index an empty `packed_decode`. The only caller
    /// is `EncoderDictionary::from_bytes`, which wraps the result in the
    /// encoder-only `EncoderDictionary` type (no decode path), so this
    /// incomplete dictionary can never escape to the decode side. Keeping
    /// this `pub(crate)` keeps it off the public `Dictionary` API entirely.
    pub(crate) fn decode_dict_for_encoding(
        raw: &[u8],
    ) -> Result<Dictionary, DictionaryDecodeError> {
        Self::decode_dict_inner(raw, false)
    }

    /// Shared dictionary parser. `build_decode_tables` selects whether the
    /// FSE/HUF tables get their full decoding tables (FSE decode table +
    /// `enrich_*`, HUF `packed_decode`; decoder path) or only the
    /// probability/weight parse (encoder path — see
    /// [`Self::decode_dict_for_encoding`]).
    fn decode_dict_inner(
        raw: &[u8],
        build_decode_tables: bool,
    ) -> Result<Dictionary, DictionaryDecodeError> {
        const MIN_MAGIC_AND_ID_LEN: usize = 8;
        const OFFSET_HISTORY_LEN: usize = 12;

        if raw.len() < MIN_MAGIC_AND_ID_LEN {
            return Err(DictionaryDecodeError::DictionaryTooSmall {
                got: raw.len(),
                need: MIN_MAGIC_AND_ID_LEN,
            });
        }

        let mut new_dict = Dictionary {
            id: 0,
            fse: FSEScratch::new(),
            huf: HuffmanScratch::new(),
            dict_content: Vec::new(),
            offset_hist: [1, 4, 8],
        };

        let magic_num: [u8; 4] = raw[..4].try_into().expect("optimized away");
        if magic_num != MAGIC_NUM {
            return Err(DictionaryDecodeError::BadMagicNum { got: magic_num });
        }

        let dict_id = raw[4..8].try_into().expect("optimized away");
        let dict_id = u32::from_le_bytes(dict_id);
        if dict_id == 0 {
            return Err(DictionaryDecodeError::ZeroDictionaryId);
        }
        new_dict.id = dict_id;

        let raw_tables = &raw[8..];

        let huf_size = if build_decode_tables {
            new_dict.huf.table.build_decoder(raw_tables)?
        } else {
            new_dict.huf.table.build_weights_only(raw_tables)?
        };
        let raw_tables = &raw_tables[huf_size as usize..];

        let of_size = if build_decode_tables {
            let n = new_dict.fse.offsets.build_decoder(
                raw_tables,
                crate::decoding::sequence_section_decoder::OF_MAX_LOG,
            )?;
            new_dict.fse.offsets.enrich_for_offsets();
            // Compute the pipeline-gate long-offset share ONCE here, while the
            // dictionary handle is built, so the per-decode `init_from_dict`
            // path can COPY it instead of re-walking the offsets table on every
            // `decode_*_with_dict_handle` call (the dict is immutable, so the
            // share never changes after this).
            new_dict.fse.offsets_long_share =
                crate::decoding::sequence_section_decoder::compute_offsets_long_share(
                    &new_dict.fse.offsets,
                );
            n
        } else {
            new_dict.fse.offsets.read_table_probabilities(
                raw_tables,
                crate::decoding::sequence_section_decoder::OF_MAX_LOG,
            )?
        };
        let raw_tables = &raw_tables[of_size..];

        let ml_size = if build_decode_tables {
            let n = new_dict.fse.match_lengths.build_decoder(
                raw_tables,
                crate::decoding::sequence_section_decoder::ML_MAX_LOG,
            )?;
            new_dict
                .fse
                .match_lengths
                .enrich_with_packed_seq_meta(&crate::decoding::sequence_section_decoder::ML_META);
            n
        } else {
            new_dict.fse.match_lengths.read_table_probabilities(
                raw_tables,
                crate::decoding::sequence_section_decoder::ML_MAX_LOG,
            )?
        };
        let raw_tables = &raw_tables[ml_size..];

        let ll_size = if build_decode_tables {
            let n = new_dict.fse.literal_lengths.build_decoder(
                raw_tables,
                crate::decoding::sequence_section_decoder::LL_MAX_LOG,
            )?;
            new_dict
                .fse
                .literal_lengths
                .enrich_with_packed_seq_meta(&crate::decoding::sequence_section_decoder::LL_META);
            n
        } else {
            new_dict.fse.literal_lengths.read_table_probabilities(
                raw_tables,
                crate::decoding::sequence_section_decoder::LL_MAX_LOG,
            )?
        };
        let raw_tables = &raw_tables[ll_size..];

        if raw_tables.len() < OFFSET_HISTORY_LEN {
            return Err(DictionaryDecodeError::DictionaryTooSmall {
                got: raw_tables.len(),
                need: OFFSET_HISTORY_LEN,
            });
        }

        let offset1 = raw_tables[0..4].try_into().expect("optimized away");
        let offset1 = u32::from_le_bytes(offset1);

        let offset2 = raw_tables[4..8].try_into().expect("optimized away");
        let offset2 = u32::from_le_bytes(offset2);

        let offset3 = raw_tables[8..12].try_into().expect("optimized away");
        let offset3 = u32::from_le_bytes(offset3);

        if offset1 == 0 {
            return Err(DictionaryDecodeError::ZeroRepeatOffsetInDictionary { index: 0 });
        }
        if offset2 == 0 {
            return Err(DictionaryDecodeError::ZeroRepeatOffsetInDictionary { index: 1 });
        }
        if offset3 == 0 {
            return Err(DictionaryDecodeError::ZeroRepeatOffsetInDictionary { index: 2 });
        }

        new_dict.offset_hist[0] = offset1;
        new_dict.offset_hist[1] = offset2;
        new_dict.offset_hist[2] = offset3;

        let raw_content = &raw_tables[12..];
        new_dict.dict_content.extend(raw_content);

        Ok(new_dict)
    }

    /// Convert this parsed dictionary into a reusable shared handle.
    pub fn into_handle(self) -> DictionaryHandle {
        DictionaryHandle::from_dictionary(self)
    }
}

impl DictionaryHandle {
    /// Wrap an already-parsed dictionary in a shared handle.
    pub fn from_dictionary(dict: Dictionary) -> Self {
        Self {
            inner: SharedDictionary::new(dict),
        }
    }

    /// Parse a serialized dictionary and return a reusable shared handle.
    pub fn decode_dict(raw: &[u8]) -> Result<Self, DictionaryDecodeError> {
        Dictionary::decode_dict(raw).map(Self::from_dictionary)
    }

    pub fn id(&self) -> u32 {
        self.inner.id
    }

    pub fn as_dict(&self) -> &Dictionary {
        &self.inner
    }
}

impl AsRef<Dictionary> for DictionaryHandle {
    fn as_ref(&self) -> &Dictionary {
        self.as_dict()
    }
}

impl From<Dictionary> for DictionaryHandle {
    fn from(dict: Dictionary) -> Self {
        DictionaryHandle::from_dictionary(dict)
    }
}

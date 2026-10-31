#![allow(
    clippy::cast_lossless,
    clippy::map_unwrap_or,
    clippy::redundant_closure_for_method_calls,
    clippy::struct_excessive_bools,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::mem;

use crate::common::MAX_BLOCK_SIZE;

use core::hash::Hasher;

use twox_hash::XxHash64;

use crate::encoding::levels::compress_block_encoded;
use crate::encoding::{
    CompressionLevel, EncoderDictionary, MatchGeneratorDriver, Matcher, block_header::BlockHeader,
    frame_compressor::CachedDictionaryEntropy, frame_compressor::CompressState,
    frame_compressor::FseTables, frame_compressor::PreviousFseTable, frame_header::FrameHeader,
};
use crate::io::{Error, ErrorKind, Write};

/// Incremental frame encoder that implements [`Write`].
///
/// Data can be provided with multiple `write()` calls. Full blocks are compressed
/// automatically, `flush()` emits the currently buffered partial block as non-last,
/// and `finish()` closes the frame and returns the wrapped writer.
pub struct StreamingEncoder<W: Write, M: Matcher = MatchGeneratorDriver> {
    drain: Option<W>,
    compression_level: CompressionLevel,
    state: CompressState<M>,
    pending: Vec<u8>,
    encoded_scratch: Vec<u8>,
    errored: bool,
    last_error_kind: Option<ErrorKind>,
    last_error_message: Option<String>,
    frame_started: bool,
    /// Upper bound on emitted block sizes (upstream `ZSTD_c_targetCBlockSize`
    /// semantics; see `FrameCompressor::set_target_block_size`). `None` =
    /// the format's 128 KiB ceiling.
    target_block_size: Option<u32>,
    pledged_content_size: Option<u64>,
    /// Advisory source-size hint from [`set_source_size_hint`](Self::set_source_size_hint).
    /// Unlike `pledged_content_size` it carries no end-of-frame enforcement, but
    /// it still feeds the small-input gates (matcher sizing AND the Fast HUF
    /// fast-path gate) so `set_source_size_hint(small)` reduces work the same way
    /// a pledge does. The HUF gate reads `pledged_content_size.or(source_size_hint)`.
    source_size_hint: Option<u64>,
    /// Whether a pledged size is written into the header's
    /// `Frame_Content_Size` field (upstream `ZSTD_c_contentSizeFlag`).
    /// Pledge *enforcement* is independent of this flag — upstream
    /// validates consumed bytes against the pledge at frame end even
    /// when the header omits the field. Default `true`.
    content_size_flag: bool,
    bytes_consumed: u64,
    /// Effective strategy tag when a public-parameter
    /// [`Strategy`](crate::encoding::Strategy) override is active, mirroring
    /// [`FrameCompressor`](crate::encoding::FrameCompressor)'s field. `Some`
    /// survives frame start so the literal-compression gates run the same
    /// strategy the matcher does; `None` keeps the level-derived tag.
    strategy_override: Option<crate::encoding::strategy::StrategyTag>,
    /// `ZSTD_f_zstd1_magicless` — omit the 4-byte magic number prefix.
    /// Default false. See [`Self::set_magicless`].
    magicless: bool,
    /// Whether to emit a trailing XXH64 content checksum and set the frame
    /// header's `Content_Checksum_flag` (upstream `ZSTD_c_checksumFlag`).
    /// Default `false`, matching the upstream library default; combined with
    /// the `hash` feature, so without `hash` no checksum is emitted
    /// regardless. See [`Self::set_content_checksum`].
    content_checksum: bool,
    /// Dictionary applied to the frame (upstream zstd `ZSTD_CCtx_loadDictionary` on a
    /// streaming context). `None` = no dictionary. Set before the first write.
    dictionary: Option<EncoderDictionary>,
    /// Whether the frame header records the attached dictionary's ID
    /// (upstream `ZSTD_c_dictIDFlag`). Default `true`. Raw-content
    /// dictionaries (upstream `ZSTD_CCtx_refPrefix`) carry a synthetic
    /// non-zero ID that must not reach the wire, so their attach path
    /// turns this off. See [`Self::set_dictionary_id_flag`].
    dictionary_id_flag: bool,
    /// Encoder entropy tables (literals Huffman + LL/ML/OF FSE "previous"
    /// tables) the dictionary seeds into the first block, derived once when the
    /// dictionary is attached so each frame start is a cheap clone.
    dictionary_entropy_cache: Option<CachedDictionaryEntropy>,

    hasher: XxHash64,
}

impl<W: Write> StreamingEncoder<W, MatchGeneratorDriver> {
    /// Creates a streaming encoder backed by the default match generator.
    ///
    /// The encoder writes compressed bytes into `drain` and applies `compression_level`
    /// to all subsequently written blocks.
    pub fn new(drain: W, compression_level: CompressionLevel) -> Self {
        Self::new_with_matcher(
            MatchGeneratorDriver::new(MAX_BLOCK_SIZE as usize, 1),
            drain,
            compression_level,
        )
    }

    /// Configure fine-grained compression parameters: resets the level to
    /// the parameters' level and installs the per-knob overrides (window / hash
    /// / chain / search logs, strategy, long-distance matching) applied at the
    /// next frame. Mirrors
    /// [`FrameCompressor::set_parameters`](crate::encoding::FrameCompressor::set_parameters). Must be called
    /// before the first [`write`](Write::write). Only the built-in
    /// `MatchGeneratorDriver` exposes the override knobs, so this lives on the
    /// default-matcher impl.
    pub fn set_parameters(
        &mut self,
        params: &crate::encoding::CompressionParameters,
    ) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "compression parameters must be set before the first write",
            ));
        }
        self.compression_level = params.level();
        let overrides = params.overrides();
        // Persist the strategy override so `ensure_frame_started`'s level-based
        // resync does not discard it (matching `FrameCompressor::set_parameters`).
        self.strategy_override = overrides.strategy.map(|s| s.tag());
        self.state.strategy_tag = self.strategy_override.unwrap_or_else(|| {
            crate::encoding::strategy::StrategyTag::for_compression_level(self.compression_level)
        });
        self.state.huf_optimal_search = crate::encoding::frame_compressor::huf_search_enabled(
            self.state.strategy_tag,
            self.pledged_content_size.or(self.source_size_hint),
        );
        self.state.literal_compression_disabled = self.state.strategy_tag
            == crate::encoding::strategy::StrategyTag::Fast
            && overrides.target_length.map_or_else(
                || matches!(self.compression_level, CompressionLevel::Level(n) if n < 0),
                |length| length > 0,
            );
        self.state.matcher.set_param_overrides(Some(overrides));
        Ok(())
    }
}

impl<W: Write, M: Matcher> StreamingEncoder<W, M> {
    /// Creates a streaming encoder with an explicitly provided matcher implementation.
    ///
    /// This constructor is primarily intended for tests and advanced callers that need
    /// custom match-window behavior.
    pub fn new_with_matcher(matcher: M, drain: W, compression_level: CompressionLevel) -> Self {
        Self {
            drain: Some(drain),
            compression_level,
            state: CompressState {
                matcher,
                last_huff_table: None,
                huff_table_spare: None,
                fse_tables: FseTables::new(),
                block_scratch: crate::encoding::blocks::CompressedBlockScratch::new(),
                offset_hist: [1, 4, 8],
                strategy_tag: crate::encoding::strategy::StrategyTag::for_compression_level(
                    compression_level,
                ),
                huf_optimal_search: true,
                literal_compression_disabled: matches!(
                    compression_level,
                    CompressionLevel::Level(n) if n < 0
                ),
            },
            pending: Vec::new(),
            encoded_scratch: Vec::new(),
            errored: false,
            last_error_kind: None,
            last_error_message: None,
            frame_started: false,
            target_block_size: None,
            pledged_content_size: None,
            source_size_hint: None,
            content_size_flag: true,
            bytes_consumed: 0,
            strategy_override: None,
            magicless: false,
            content_checksum: false,
            dictionary: None,
            dictionary_id_flag: true,
            dictionary_entropy_cache: None,

            hasher: XxHash64::with_seed(0),
        }
    }

    /// Set an upper bound on each physical block's payload (semantics of
    /// upstream `ZSTD_c_targetCBlockSize`): every block carries at most
    /// `target` payload bytes, +3-byte block header on the wire — the
    /// upstream knob is likewise a convergence target for block sizing,
    /// not a cap on header-inclusive wire bytes. Clamped to
    /// `[MIN_TARGET_BLOCK_SIZE, MAX_BLOCK_SIZE]`; mirrors
    /// `FrameCompressor::set_target_block_size`. Must be set before the
    /// first write.
    pub fn set_target_block_size(&mut self, target: Option<u32>) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "the block-size target must be set before the first write",
            ));
        }
        self.target_block_size =
            target.map(|t| t.clamp(crate::common::MIN_TARGET_BLOCK_SIZE, MAX_BLOCK_SIZE));
        Ok(())
    }

    /// Enable or disable the trailing XXH64 content checksum
    /// (upstream `ZSTD_c_checksumFlag`). Default `false`, matching the
    /// upstream library default (`ZSTD_c_checksumFlag = 0`). Must be called
    /// before the first [`write`](Write::write); once the frame header is
    /// emitted the flag is fixed, so a late change returns an error rather
    /// than producing a header/trailer mismatch. Without the `hash` feature
    /// no checksum is emitted regardless.
    pub fn set_content_checksum(&mut self, emit: bool) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "content checksum must be set before the first write",
            ));
        }
        self.content_checksum = emit;
        Ok(())
    }

    /// Enable or disable magicless frame format (`ZSTD_f_zstd1_magicless`).
    ///
    /// When set to `true`, the frame header serialized by this encoder
    /// omits the 4-byte magic number prefix. Must be called BEFORE the
    /// first [`write`](Write::write) call; calling it after the frame
    /// header has already been emitted returns an error so the caller
    /// can't be misled into thinking they produced a magicless stream.
    pub fn set_magicless(&mut self, magicless: bool) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "magicless format must be set before the first write",
            ));
        }
        self.magicless = magicless;
        Ok(())
    }

    /// Pledge the total uncompressed content size for this frame.
    ///
    /// When set, the frame header will include a `Frame_Content_Size` field.
    /// This enables decoders to pre-allocate output buffers.
    /// The pledged size is also forwarded as a source-size hint to the
    /// matcher so small inputs can use smaller matching tables.
    ///
    /// Must be called **before** the first [`write`](Write::write) call;
    /// calling it after the frame header has already been emitted returns an
    /// error.
    pub fn set_pledged_content_size(&mut self, size: u64) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "pledged content size must be set before the first write",
            ));
        }
        self.pledged_content_size = Some(size);
        // Also use pledged size as source-size hint so the matcher
        // can select smaller tables for small inputs.
        self.state.matcher.set_source_size_hint(size);
        Ok(())
    }

    /// Control whether the pledged size is written into the header's
    /// `Frame_Content_Size` field (upstream `ZSTD_c_contentSizeFlag`,
    /// default on). With the flag off the header omits the field, but a
    /// pledge set via [`set_pledged_content_size`](Self::set_pledged_content_size)
    /// is still enforced against the bytes actually written. Must be
    /// called before the first [`write`](Write::write).
    pub fn set_content_size_flag(&mut self, emit: bool) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "content size flag must be set before the first write",
            ));
        }
        self.content_size_flag = emit;
        Ok(())
    }

    /// Provide a hint about the total uncompressed size for the next frame.
    ///
    /// Unlike [`set_pledged_content_size`](Self::set_pledged_content_size),
    /// this does **not** enforce that exactly `size` bytes are written; it
    /// may reduce matcher tables, advertised frame window, and block sizing
    /// for small inputs. Must be called before the first
    /// [`write`](Write::write).
    pub fn set_source_size_hint(&mut self, size: u64) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "source size hint must be set before the first write",
            ));
        }
        self.state.matcher.set_source_size_hint(size);
        // Feed the same hint to the Fast HUF fast-path gate (resolved in
        // `set_parameters` / `ensure_frame_started` via
        // `pledged_content_size.or(source_size_hint)`), so a small advisory size
        // also lifts Fast streams off the expensive optimal-HUF search.
        self.source_size_hint = Some(size);
        Ok(())
    }

    /// Attach a serialized dictionary blob to the frame (upstream zstd
    /// `ZSTD_CCtx_loadDictionary` on a streaming context). The dictionary primes
    /// the match-finder and seeds the first block's entropy tables + repeat
    /// offsets, and its ID is written into the frame header. Must be called
    /// before the first [`write`](Write::write); the parsed dictionary must have
    /// a non-zero ID and non-zero repeat offsets.
    pub fn set_dictionary_from_bytes(&mut self, raw_dictionary: &[u8]) -> Result<(), Error> {
        let dict = EncoderDictionary::from_bytes(raw_dictionary)
            .map_err(|err| invalid_input_error(&alloc::format!("invalid dictionary: {err:?}")))?;
        self.set_encoder_dictionary(dict)
    }

    /// Whether the frame header records the dictionary ID when a dictionary
    /// is attached (upstream `ZSTD_c_dictIDFlag` semantics; default `true`).
    /// Mirrors
    /// [`FrameCompressor::set_dictionary_id_flag`](crate::encoding::FrameCompressor::set_dictionary_id_flag). Decoders can still
    /// decode such frames by supplying the dictionary explicitly.
    pub fn set_dictionary_id_flag(&mut self, emit: bool) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "dictionary ID flag must be set before the first write",
            ));
        }
        self.dictionary_id_flag = emit;
        Ok(())
    }

    /// Attach an already-parsed [`EncoderDictionary`] to the frame. See
    /// [`set_dictionary_from_bytes`](Self::set_dictionary_from_bytes); must be
    /// called before the first write.
    pub fn set_encoder_dictionary(&mut self, dict: EncoderDictionary) -> Result<(), Error> {
        self.ensure_open()?;
        if self.frame_started {
            return Err(invalid_input_error(
                "dictionary must be attached before the first write",
            ));
        }
        let inner = &dict.inner;
        if inner.id == 0 {
            return Err(invalid_input_error("dictionary has a zero ID"));
        }
        if inner.offset_hist.contains(&0) {
            return Err(invalid_input_error(
                "dictionary carries a zero repeat offset",
            ));
        }
        self.dictionary_entropy_cache = Some(CachedDictionaryEntropy::from_dictionary(inner));
        self.dictionary = Some(dict);
        Ok(())
    }

    /// Returns an immutable reference to the wrapped output drain.
    ///
    /// The drain remains available for the encoder lifetime; [`finish`](Self::finish)
    /// consumes the encoder and returns ownership of the drain.
    pub fn get_ref(&self) -> &W {
        self.drain
            .as_ref()
            .expect("streaming encoder drain is present until finish consumes self")
    }

    /// Total heap bytes this encoder's allocations hold, excluding the
    /// inline struct and the drain `W` (whose footprint the owner can
    /// measure through [`get_ref`](Self::get_ref)): match-finder tables /
    /// history / recycled buffers, retained Huffman tables, the staging
    /// `pending` / `encoded_scratch` buffers, the retained dictionary
    /// content, and the cached dictionary entropy tables. Mirrors
    /// `FrameCompressor::heap_size` so a context can report its true
    /// footprint through `ZSTD_sizeof_CCtx`.
    pub fn heap_size(&self) -> usize {
        let mut total = self.state.matcher.heap_size();
        total += self
            .state
            .last_huff_table
            .as_ref()
            .map_or(0, |table| table.heap_size());
        total += self
            .state
            .huff_table_spare
            .as_ref()
            .map_or(0, |table| table.heap_size());
        total += self.pending.capacity();
        total += self.encoded_scratch.capacity();
        total += self
            .dictionary
            .as_ref()
            .map_or(0, |d| d.inner.dict_content.capacity());
        total += self
            .dictionary_entropy_cache
            .as_ref()
            .map_or(0, CachedDictionaryEntropy::heap_size);
        total
    }

    /// Returns a mutable reference to the wrapped output drain.
    ///
    /// It is inadvisable to directly write to the underlying writer, as doing
    /// so would corrupt the zstd frame being assembled by the encoder.
    ///
    /// The drain remains available for the encoder lifetime; [`finish`](Self::finish)
    /// consumes the encoder and returns ownership of the drain.
    pub fn get_mut(&mut self) -> &mut W {
        self.drain
            .as_mut()
            .expect("streaming encoder drain is present until finish consumes self")
    }

    /// Finalizes the current zstd frame and returns the wrapped output drain.
    ///
    /// If no payload was written yet, this still emits a valid empty frame.
    /// Calling this method consumes the encoder.
    pub fn finish(mut self) -> Result<W, Error> {
        self.ensure_open()?;

        // Validate the pledge before finalizing the frame. If finish() is
        // called before any writes, this also avoids emitting a header with
        // an incorrect FCS into the drain on mismatch.
        if let Some(pledged) = self.pledged_content_size
            && self.bytes_consumed != pledged
        {
            return Err(invalid_input_error(
                "pledged content size does not match bytes consumed",
            ));
        }

        self.ensure_frame_started()?;

        if self.pending.is_empty() {
            self.write_empty_last_block()
                .map_err(|err| self.fail(err))?;
        } else {
            self.emit_pending_block(true)?;
        }

        let mut drain = self
            .drain
            .take()
            .expect("streaming encoder drain must be present when finishing");

        if self.content_checksum {
            let checksum = self.hasher.finish() as u32;
            drain
                .write_all(&checksum.to_le_bytes())
                .map_err(|err| self.fail(err))?;
        }

        drain.flush().map_err(|err| self.fail(err))?;
        Ok(drain)
    }

    fn ensure_open(&self) -> Result<(), Error> {
        if self.errored {
            return Err(self.sticky_error());
        }
        Ok(())
    }

    // Cold path (only reached after poisoning). The format!() calls still allocate
    // in no_std even though error_with_kind_message/other_error_owned drop the
    // message; this is acceptable on an error recovery path to keep match arms simple.
    fn sticky_error(&self) -> Error {
        match (self.last_error_kind, self.last_error_message.as_deref()) {
            (Some(kind), Some(message)) => error_with_kind_message(
                kind,
                format!(
                    "streaming encoder is in an errored state due to previous {kind:?} failure: {message}"
                ),
            ),
            (Some(kind), None) => error_from_kind(kind),
            (None, Some(message)) => other_error_owned(format!(
                "streaming encoder is in an errored state: {message}"
            )),
            (None, None) => other_error("streaming encoder is in an errored state"),
        }
    }

    fn drain_mut(&mut self) -> Result<&mut W, Error> {
        self.drain
            .as_mut()
            .ok_or_else(|| other_error("streaming encoder has no active drain"))
    }

    fn ensure_frame_started(&mut self) -> Result<(), Error> {
        if self.frame_started {
            return Ok(());
        }

        self.ensure_level_supported()?;
        // A dictionary is only active when it can actually be primed: the level
        // compresses (not `Uncompressed`) AND the matcher supports priming AND a
        // dictionary is attached. Mirrors `FrameCompressor`'s `use_dictionary_state`
        // so a streaming frame never advertises a `Dictionary_ID`, disables
        // single-segment, or seeds dict entropy/offsets unless the dictionary is
        // genuinely in play (otherwise it would emit frames that needlessly
        // require a dictionary at decode time).
        let use_dictionary_state =
            !matches!(self.compression_level, CompressionLevel::Uncompressed)
                && self.state.matcher.supports_dictionary_priming()
                && self.dictionary.is_some();
        // The dictionary content size drives dict-tier match-finder sizing
        // (consumed inside `reset`), so hand it over BEFORE reset.
        if use_dictionary_state && let Some(dict) = self.dictionary.as_ref() {
            self.state
                .matcher
                .set_dictionary_size_hint(dict.inner.dict_content.len());
        }
        self.state.matcher.reset(self.compression_level);
        // Seed the repeat-offset history from the dictionary (upstream zstd
        // `ZSTD_compress_insertDictionary`), or the default rep codes otherwise.
        self.state.offset_hist = if use_dictionary_state {
            self.dictionary
                .as_ref()
                .map(|dict| dict.inner.offset_hist)
                .unwrap_or([1, 4, 8])
        } else {
            [1, 4, 8]
        };
        // Prime the match-finder with the dictionary content + offsets.
        // `dict` borrows `self.dictionary`; `self.state.matcher` is a disjoint
        // field, so the immutable dict borrow and the mutable matcher borrow
        // coexist (field-level borrow splitting) with no conflict.
        if use_dictionary_state && let Some(dict) = self.dictionary.as_ref() {
            let offset_hist = dict.inner.offset_hist;
            self.state
                .matcher
                .prime_with_dictionary(dict.inner.dict_content.as_slice(), offset_hist);
        }
        // Seed the first block's entropy from the dictionary's cached encoder
        // tables (upstream zstd `cdict->cBlockState`), or clear to defaults.
        if use_dictionary_state && let Some(cache) = self.dictionary_entropy_cache.as_ref() {
            self.state.last_huff_table.clone_from(&cache.huff);
            self.state
                .fse_tables
                .ll_previous
                .clone_from(&cache.ll_previous);
            self.state
                .fse_tables
                .ml_previous
                .clone_from(&cache.ml_previous);
            self.state
                .fse_tables
                .of_previous
                .clone_from(&cache.of_previous);
            let ll_entropy = match cache.ll_previous.as_ref() {
                Some(PreviousFseTable::Custom(table)) => Some(table.as_ref()),
                _ => None,
            };
            let ml_entropy = match cache.ml_previous.as_ref() {
                Some(PreviousFseTable::Custom(table)) => Some(table.as_ref()),
                _ => None,
            };
            let of_entropy = match cache.of_previous.as_ref() {
                Some(PreviousFseTable::Custom(table)) => Some(table.as_ref()),
                _ => None,
            };
            self.state.matcher.seed_dictionary_entropy(
                self.state.last_huff_table.as_ref(),
                ll_entropy,
                ml_entropy,
                of_entropy,
            );
        } else {
            self.state.last_huff_table = None;
            self.state.fse_tables.ll_previous = None;
            self.state.fse_tables.ml_previous = None;
            self.state.fse_tables.of_previous = None;
        }
        // Sync `state.strategy_tag` from the active compression level so the
        // literal-compression gates (`min_literals_to_compress`, `min_gain`
        // in `encoding::blocks::compressed`) see the correct strategy for
        // every frame. Mirrors `FrameCompressor::compress` and keeps both
        // entry points byte-equivalent at the gate level. A public-parameter
        // strategy override wins over the level's derived tag so the
        // gates see the strategy the matcher actually runs.
        self.state.strategy_tag = self.strategy_override.unwrap_or_else(|| {
            crate::encoding::strategy::StrategyTag::for_compression_level(self.compression_level)
        });
        self.state.huf_optimal_search = crate::encoding::frame_compressor::huf_search_enabled(
            self.state.strategy_tag,
            self.pledged_content_size.or(self.source_size_hint),
        );

        {
            self.hasher = XxHash64::with_seed(0);
        }

        let window_size = self.state.matcher.window_size();
        if window_size == 0 {
            return Err(invalid_input_error(
                "matcher reported window_size == 0, which is invalid",
            ));
        }

        // Single-segment is incompatible with a dictionary (the dictionary
        // pushes referenceable history before the content, so the frame needs
        // an explicit window descriptor); gate it off when a dict is attached,
        // mirroring `FrameCompressor`'s `!use_dictionary_state` guard.
        // Single-segment also requires the FCS field to be present
        // (`content_size_flag`): the layout drops the window descriptor,
        // so the header must carry the content size for decoders to size
        // their window.
        let single_segment = self.content_size_flag
            && !use_dictionary_state
            && self
                .pledged_content_size
                .map(|size| (512..=(1 << 14)).contains(&size) && size <= window_size)
                .unwrap_or(false);

        let header = FrameHeader {
            frame_content_size: if self.content_size_flag {
                self.pledged_content_size
            } else {
                None
            },
            single_segment,
            content_checksum: self.content_checksum,
            dictionary_id: if use_dictionary_state && self.dictionary_id_flag {
                self.dictionary.as_ref().map(|dict| dict.inner.id as u64)
            } else {
                None
            },
            window_size: if single_segment {
                None
            } else {
                Some(window_size)
            },
            magicless: self.magicless,
        };
        let mut encoded_header = Vec::new();
        header.serialize(&mut encoded_header);
        self.drain_mut()
            .and_then(|drain| drain.write_all(&encoded_header))
            .map_err(|err| self.fail(err))?;

        self.frame_started = true;
        Ok(())
    }

    fn block_capacity(&self) -> usize {
        let matcher_window = self.state.matcher.window_size() as usize;
        let ceiling = self
            .target_block_size
            .map_or(MAX_BLOCK_SIZE as usize, |t| t as usize);
        core::cmp::max(1, core::cmp::min(matcher_window, ceiling))
    }

    fn allocate_pending_space(&mut self, block_capacity: usize) -> Vec<u8> {
        let mut space = match self.compression_level {
            CompressionLevel::Fastest
            | CompressionLevel::Default
            | CompressionLevel::Better
            | CompressionLevel::Best
            | CompressionLevel::Level(_) => self.state.matcher.get_next_space(),
            CompressionLevel::Uncompressed => Vec::new(),
        };
        space.clear();
        if space.capacity() > block_capacity {
            space.shrink_to(block_capacity);
        }
        if space.capacity() < block_capacity {
            space.reserve(block_capacity - space.capacity());
        }
        space
    }

    fn emit_full_pending_block(
        &mut self,
        block_capacity: usize,
        consumed: usize,
    ) -> Option<Result<usize, Error>> {
        if self.pending.len() != block_capacity {
            return None;
        }

        let new_pending = self.allocate_pending_space(block_capacity);
        let full_block = mem::replace(&mut self.pending, new_pending);
        if let Err((err, restored_block)) = self.encode_block(full_block, false) {
            self.pending = restored_block;
            let err = self.fail(err);
            if consumed > 0 {
                return Some(Ok(consumed));
            }
            return Some(Err(err));
        }
        None
    }

    fn emit_pending_block(&mut self, last_block: bool) -> Result<(), Error> {
        let block = mem::take(&mut self.pending);
        if let Err((err, restored_block)) = self.encode_block(block, last_block) {
            self.pending = restored_block;
            return Err(self.fail(err));
        }
        if !last_block {
            let block_capacity = self.block_capacity();
            self.pending = self.allocate_pending_space(block_capacity);
        }
        Ok(())
    }

    // Exhaustive match kept intentionally: adding a new CompressionLevel
    // variant will produce a compile error here, forcing the developer to
    // decide whether the streaming encoder supports it before shipping.
    fn ensure_level_supported(&self) -> Result<(), Error> {
        match self.compression_level {
            CompressionLevel::Uncompressed
            | CompressionLevel::Fastest
            | CompressionLevel::Default
            | CompressionLevel::Better
            | CompressionLevel::Best
            | CompressionLevel::Level(_) => Ok(()),
        }
    }

    fn encode_block(
        &mut self,
        uncompressed_data: Vec<u8>,
        last_block: bool,
    ) -> Result<(), (Error, Vec<u8>)> {
        let mut raw_block = Some(uncompressed_data);
        let mut encoded = Vec::new();
        mem::swap(&mut encoded, &mut self.encoded_scratch);
        encoded.clear();
        let needed_capacity = self.block_capacity() + 3;
        if encoded.capacity() < needed_capacity {
            encoded.reserve(needed_capacity.saturating_sub(encoded.len()));
        }
        let mut moved_into_matcher = false;
        if raw_block.as_ref().is_some_and(|block| block.is_empty()) {
            let header = BlockHeader {
                last_block,
                block_type: crate::blocks::block::BlockType::Raw,
                block_size: 0,
            };
            header.serialize(&mut encoded);
        } else {
            match self.compression_level {
                CompressionLevel::Uncompressed => {
                    let block = raw_block.as_ref().expect("raw block missing");
                    let header = BlockHeader {
                        last_block,
                        block_type: crate::blocks::block::BlockType::Raw,
                        block_size: block.len() as u32,
                    };
                    header.serialize(&mut encoded);
                    encoded.extend_from_slice(block);
                }
                CompressionLevel::Fastest
                | CompressionLevel::Default
                | CompressionLevel::Better
                | CompressionLevel::Best
                | CompressionLevel::Level(_) => {
                    let block = raw_block.take().expect("raw block missing");
                    debug_assert!(!block.is_empty(), "empty blocks handled above");
                    // A primed dictionary makes "incompressible-looking" blocks
                    // matchable, so the raw-fast-path must NOT fire. But a dict is
                    // only PRIMED when the matcher supports priming — a non-priming
                    // matcher ignores the attached dictionary, so the raw-fast-path
                    // must stay enabled for it. (This arm is already non-Uncompressed.)
                    // Mirrors `FrameCompressor`'s `dict_active`.
                    let dict_active = self.dictionary.is_some()
                        && self.state.matcher.supports_dictionary_priming();
                    compress_block_encoded(
                        &mut self.state,
                        self.compression_level,
                        last_block,
                        block,
                        &mut encoded,
                        dict_active,
                        // No FrameEmitInfo on the streaming encoder path — it
                        // does not surface per-block layout, so no sidecar.
                    );
                    moved_into_matcher = true;
                }
            }
        }

        if let Err(err) = self.drain_mut().and_then(|drain| drain.write_all(&encoded)) {
            encoded.clear();
            mem::swap(&mut encoded, &mut self.encoded_scratch);
            let restored = if moved_into_matcher {
                self.state.matcher.get_last_space().to_vec()
            } else {
                raw_block.unwrap_or_default()
            };
            return Err((err, restored));
        }

        if moved_into_matcher {
            if self.content_checksum {
                self.hasher.write(self.state.matcher.get_last_space());
            }
        } else {
            self.hash_block(raw_block.as_deref().unwrap_or(&[]));
        }
        encoded.clear();
        mem::swap(&mut encoded, &mut self.encoded_scratch);
        Ok(())
    }

    fn write_empty_last_block(&mut self) -> Result<(), Error> {
        self.encode_block(Vec::new(), true).map_err(|(err, _)| err)
    }

    fn fail(&mut self, err: Error) -> Error {
        self.errored = true;
        if self.last_error_kind.is_none() {
            self.last_error_kind = Some(err.kind());
        }
        if self.last_error_message.is_none() {
            self.last_error_message = Some(err.to_string());
        }
        err
    }

    fn hash_block(&mut self, uncompressed_data: &[u8]) {
        if self.content_checksum {
            self.hasher.write(uncompressed_data);
        }
    }
}

impl<W: Write, M: Matcher> Write for StreamingEncoder<W, M> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.ensure_open()?;
        if buf.is_empty() {
            return Ok(0);
        }

        // Check pledge before emitting the frame header so that a misuse
        // like set_pledged_content_size(0) + write(non_empty) doesn't leave
        // a partially-written header in the drain.
        if let Some(pledged) = self.pledged_content_size
            && self.bytes_consumed >= pledged
        {
            return Err(invalid_input_error(
                "write would exceed pledged content size",
            ));
        }

        self.ensure_frame_started()?;

        // Enforce pledged upper bound: truncate the accepted slice to the
        // remaining allowance so that partial-write semantics are honored
        // (return Ok(n) with n < buf.len()) instead of failing the full call.
        let buf = if let Some(pledged) = self.pledged_content_size {
            let remaining_allowed = pledged
                .checked_sub(self.bytes_consumed)
                .ok_or_else(|| invalid_input_error("bytes consumed exceed pledged content size"))?;
            if remaining_allowed == 0 {
                return Err(invalid_input_error(
                    "write would exceed pledged content size",
                ));
            }
            let accepted = core::cmp::min(
                buf.len(),
                usize::try_from(remaining_allowed).unwrap_or(usize::MAX),
            );
            &buf[..accepted]
        } else {
            buf
        };

        let block_capacity = self.block_capacity();
        if self.pending.capacity() == 0 {
            self.pending = self.allocate_pending_space(block_capacity);
        }
        let mut remaining = buf;
        let mut consumed = 0usize;

        while !remaining.is_empty() {
            if let Some(result) = self.emit_full_pending_block(block_capacity, consumed) {
                return result;
            }

            let available = block_capacity - self.pending.len();
            let to_take = core::cmp::min(remaining.len(), available);
            if to_take == 0 {
                break;
            }
            self.pending.extend_from_slice(&remaining[..to_take]);
            remaining = &remaining[to_take..];
            consumed += to_take;

            if let Some(result) = self.emit_full_pending_block(block_capacity, consumed) {
                if let Ok(n) = &result {
                    self.bytes_consumed += *n as u64;
                }
                return result;
            }
        }
        self.bytes_consumed += consumed as u64;
        Ok(consumed)
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.ensure_open()?;
        if self.pending.is_empty() {
            return self
                .drain_mut()
                .and_then(|drain| drain.flush())
                .map_err(|err| self.fail(err));
        }
        self.ensure_frame_started()?;
        self.emit_pending_block(false)?;
        self.drain_mut()
            .and_then(|drain| drain.flush())
            .map_err(|err| self.fail(err))
    }
}

fn error_from_kind(kind: ErrorKind) -> Error {
    Error::from(kind)
}

fn error_with_kind_message(kind: ErrorKind, message: String) -> Error {
    Error::new(kind, message)
}

fn invalid_input_error(message: &str) -> Error {
    Error::new(ErrorKind::InvalidInput, message)
}

fn other_error_owned(message: String) -> Error {
    Error::other(message)
}

fn other_error(message: &str) -> Error {
    Error::other(message)
}

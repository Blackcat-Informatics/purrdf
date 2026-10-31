#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! The [StreamingDecoder] wraps a [FrameDecoder] and provides a Read impl that decodes data when necessary

use core::borrow::BorrowMut;

use crate::common::MAX_BLOCK_SIZE;
use crate::decoding::errors::FrameDecoderError;
use crate::decoding::{BlockDecodingStrategy, DictionaryHandle, FrameDecoder};

use crate::io::{Error, Read};

/// High level Zstandard frame decoder that can be used to decompress a given Zstandard frame.
///
/// This decoder implements `io::Read`, so you can interact with it by calling
/// `io::Read::read_to_end` / `io::Read::read_exact` or passing this to another library / module as a source for the decoded content
///
/// If you need more control over how decompression takes place, you can use
/// the lower level [FrameDecoder], which allows for greater control over how
/// decompression takes place but the implementor must call
/// [FrameDecoder::decode_blocks] repeatedly to decode the entire frame.
///
/// ## Caveat
/// Plain `read` / `read_exact` operate on the single frame this decoder was
/// initialised with: they do not advance into following frames. `read_to_end`,
/// by contrast, is specialised to consume a finite source to EOF, decoding
/// concatenated frames and skipping skippable frames along the way.
///
/// To recover the bytes that follow one frame WITHOUT consuming the rest of the
/// source, recreate the decoder manually and handle
/// [crate::decoding::errors::ReadFrameHeaderError::SkipFrame]
/// errors by skipping forward the `length` amount of bytes, see <https://github.com/KillingSpark/zstd-rs/issues/57>
///
/// ```no_run
/// // `File` is std-only; `read_to_end` itself is available under no_std too.
/// #[cfg(all())]
/// {
///     use std::fs::File;
///     use std::io::Read;
///     use purrdf_zstd::decoding::StreamingDecoder;
///
///     // Read a Zstandard archive from the filesystem then decompress it into a vec.
///     let mut f: File = todo!("Read a .zstd archive from somewhere");
///     let mut decoder = StreamingDecoder::new(f).unwrap();
///     let mut result = Vec::new();
///     Read::read_to_end(&mut decoder, &mut result).unwrap();
/// }
/// ```
pub struct StreamingDecoder<READ: Read, DEC: BorrowMut<FrameDecoder>> {
    pub decoder: DEC,
    source: READ,
    /// Dictionary the decoder was constructed with, if any. Retained so the
    /// `read_to_end` paths can re-initialise FOLLOWING concatenated frames with
    /// the same forced dictionary (a plain re-init resolves dictionaries by
    /// frame id only and would lose a forced dict for frames omitting the id).
    /// Cheap to hold: `DictionaryHandle` is an `Arc`/`Rc` handle.
    dict: Option<DictionaryHandle>,
}

impl<READ: Read, DEC: BorrowMut<FrameDecoder>> StreamingDecoder<READ, DEC> {
    pub fn new_with_decoder(
        mut source: READ,
        mut decoder: DEC,
    ) -> Result<StreamingDecoder<READ, DEC>, FrameDecoderError> {
        decoder.borrow_mut().init(&mut source)?;
        Ok(StreamingDecoder {
            decoder,
            source,
            dict: None,
        })
    }
}

impl<READ: Read> StreamingDecoder<READ, FrameDecoder> {
    pub fn new(
        mut source: READ,
    ) -> Result<StreamingDecoder<READ, FrameDecoder>, FrameDecoderError> {
        let mut decoder = FrameDecoder::new();
        decoder.init(&mut source)?;
        Ok(StreamingDecoder {
            decoder,
            source,
            dict: None,
        })
    }

    /// Create a streaming decoder using a pre-parsed dictionary handle.
    ///
    /// # Warning
    ///
    /// This constructor initializes the underlying [`FrameDecoder`] with
    /// `dict`, even if a frame header omits the optional dictionary ID.
    /// Callers must only use it when they already know the stream was encoded
    /// with this dictionary; otherwise decoded output can be silently
    /// corrupted.
    pub fn new_with_dictionary_handle(
        mut source: READ,
        dict: &DictionaryHandle,
    ) -> Result<StreamingDecoder<READ, FrameDecoder>, FrameDecoderError> {
        let mut decoder = FrameDecoder::new();
        decoder.init_with_dict_handle(&mut source, dict)?;
        Ok(StreamingDecoder {
            decoder,
            source,
            dict: Some(dict.clone()),
        })
    }

    /// Create a streaming decoder using a serialized dictionary blob.
    ///
    /// # Warning
    ///
    /// This API forwards to [`StreamingDecoder::new_with_dictionary_handle`]
    /// and therefore applies the decoded dictionary to frames whose headers may
    /// omit the optional dictionary ID. Only use it when the stream is known to
    /// be encoded with that dictionary.
    pub fn new_with_dictionary_bytes(
        source: READ,
        raw_dictionary: &[u8],
    ) -> Result<StreamingDecoder<READ, FrameDecoder>, FrameDecoderError> {
        let dict = DictionaryHandle::decode_dict(raw_dictionary)?;
        Self::new_with_dictionary_handle(source, &dict)
    }
}

impl<READ: Read, DEC: BorrowMut<FrameDecoder>> StreamingDecoder<READ, DEC> {
    /// Gets a reference to the underlying reader.
    pub fn get_ref(&self) -> &READ {
        &self.source
    }

    /// Gets a mutable reference to the underlying reader.
    ///
    /// It is inadvisable to directly read from the underlying reader.
    pub fn get_mut(&mut self) -> &mut READ {
        &mut self.source
    }

    /// Destructures this object into the inner reader.
    pub fn into_inner(self) -> READ
    where
        READ: Sized,
    {
        self.source
    }

    /// Destructures this object into both the inner reader and [FrameDecoder].
    pub fn into_parts(self) -> (READ, DEC)
    where
        READ: Sized,
    {
        (self.source, self.decoder)
    }

    /// Destructures this object into the inner [FrameDecoder].
    pub fn into_frame_decoder(self) -> DEC {
        self.decoder
    }
}

impl<READ: Read, DEC: BorrowMut<FrameDecoder>> Read for StreamingDecoder<READ, DEC> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let decoder = self.decoder.borrow_mut();
        if decoder.is_finished() && decoder.can_collect() == 0 {
            // Frame fully decoded and fully drained: the running XXH64 digest
            // is final, so a `Verify`-mode decoder validates the content
            // checksum at this finish point. No-op in other modes.

            if let Err(e) = decoder.verify_content_checksum() {
                return Err(Error::other(e));
            }
            //No more bytes can ever be decoded
            return Ok(0);
        }

        // Interleave bounded decode with draining so the decode window
        // (`RingBuffer`) stays near `window_size` instead of accumulating the
        // whole request before a single end-of-call drain. `read_to_end` hands
        // ever-larger buffers; decoding `buf.len()` worth into the ring up
        // front grew it far past the window (repeated `reserve_amortized`
        // alloc+copy). Decode at most one block worth per step, then drain
        // what is now collectable into `buf`, mirroring upstream zstd's
        // window-bounded flush loop.
        let mut written = 0;
        while written < buf.len() {
            // Drain whatever is collectable now (retaining `window_size` until
            // the frame finishes). Reclaims the ring promptly so the next
            // decode step reuses the same capacity.
            written += decoder.read(&mut buf[written..])?;
            if written == buf.len() || decoder.is_finished() {
                break;
            }
            // Decode one bounded chunk. `UptoBytes` may overshoot a little but
            // is capped to one block, so the ring's live region stays within
            // `window_size + MAX_BLOCK_SIZE`.
            let step = (buf.len() - written).min(MAX_BLOCK_SIZE as usize);
            if let Err(e) =
                decoder.decode_blocks(&mut self.source, BlockDecodingStrategy::UptoBytes(step))
            {
                {
                    return Err(Error::other(e));
                }
            }
        }

        // The loop can finish AND fully drain a frame within this same call
        // (decode last block, then drain it into `buf`). Validate here too when
        // the frame is finished and nothing is left to collect, but ONLY when
        // this call wrote no bytes: the `Read` contract forbids returning `Err`
        // after bytes were delivered, so when `written > 0` the verify is
        // deferred to the next call, where the top early-return runs it and
        // returns `Err` on the zero-byte path. Idempotent with that top check.

        if written == 0
            && decoder.is_finished()
            && decoder.can_collect() == 0
            && let Err(e) = decoder.verify_content_checksum()
        {
            return Err(Error::other(e));
        }

        Ok(written)
    }

    /// Decode-in-place fast path for whole-frame consumption. Instead of the
    /// generic `read` loop (decode block -> `RingBuffer` -> copy into the
    /// caller buffer), buffer the (compressed, hence small) source and decode
    /// STRAIGHT into `output`'s spare capacity via the single-copy direct path,
    /// pre-sized from the frame's declared content size. Only taken when the
    /// decoder is at a frame boundary (nothing partially decoded / undrained);
    /// otherwise it falls back to the generic grow-and-`read` loop so a caller
    /// that mixed `read` with `read_to_end` still gets correct output.
    ///
    /// Per the `Read::read_to_end` contract this consumes the source to EOF: if
    /// the stream holds several concatenated frames they are ALL decoded (and
    /// skippable frames skipped). To recover bytes that follow a single frame,
    /// use `read` plus the
    /// [`SkipFrame`](crate::decoding::errors::ReadFrameHeaderError::SkipFrame)
    /// recreate-the-decoder pattern instead.

    fn read_to_end(&mut self, output: &mut alloc::vec::Vec<u8>) -> Result<usize, Error> {
        let start_total = output.len();
        // `new()` already read the frame header, so the fast path applies when
        // the decoder sits at the start of that frame with nothing decoded yet.
        let at_start = {
            let d = self.decoder.borrow_mut();
            d.is_at_frame_start() && d.can_collect() == 0
        };
        // Clone the (cheap Arc/Rc) dict handle out so the `decoder` borrow below
        // does not conflict with borrowing `self.dict`.
        let dict = self.dict.clone();
        if at_start {
            let mut compressed = alloc::vec::Vec::new();
            self.source.read_to_end(&mut compressed)?;
            self.decoder
                .borrow_mut()
                .decode_current_frame_to_vec(&compressed, output, dict.as_ref())
                .map_err(Error::other)?;
            return Ok(output.len() - start_total);
        }
        // Mid-frame fallback: drain the partially-read CURRENT frame through the
        // generic path, then decode any FOLLOWING concatenated frames so
        // read_to_end still consumes the source to true EOF.
        loop {
            let start = output.len();
            output.resize(start + MAX_BLOCK_SIZE as usize, 0);
            // On error, drop the just-grown (zeroed) tail before propagating so
            // the caller never observes bytes that were never decoded.
            let n = match self.read(&mut output[start..]) {
                Ok(n) => n,
                Err(e) => {
                    output.truncate(start);
                    return Err(e);
                }
            };
            output.truncate(start + n);
            if n == 0 {
                break;
            }
        }
        // Current frame fully drained; `source` is positioned at the next frame.
        let mut rest = alloc::vec::Vec::new();
        self.source.read_to_end(&mut rest)?;
        if !rest.is_empty() {
            let mut input = rest.as_slice();
            self.decoder
                .borrow_mut()
                .decode_concatenated_frames_to_vec(&mut input, output, dict.as_ref())
                .map_err(Error::other)?;
        }
        Ok(output.len() - start_total)
    }
}

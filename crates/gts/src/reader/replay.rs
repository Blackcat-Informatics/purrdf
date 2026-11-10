// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Snapshot interpretation over native admitted CBOR replay ranges.

use super::{
    BLOB_BUDGET_DIAGNOSTIC, BlobContent, BlobPayload, BlobRefusalKind, FoldError, Folder,
    PayloadCause, PayloadError, Read, TermIdBase, Value, ValueRef, cbor, digest_label,
};

/// Mark host replay failures at their boundary, leaving codec/source failures
/// on the production reader's existing damaged-payload diagnostic path.
struct DictionaryReplay(Box<dyn cbor::ReplayStorage>);

fn replay_io(error: std::io::Error) -> std::io::Error {
    purrdf_hash::error::io_error(
        std::io::ErrorKind::Other,
        crate::dictionary::DictionaryError::from(error),
    )
}
fn replay_transfer(error: std::io::Error) -> std::io::Error {
    if error.kind() == std::io::ErrorKind::Interrupted && error.get_ref().is_none() {
        error
    } else {
        replay_io(error)
    }
}
impl Read for DictionaryReplay {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(bytes).map_err(replay_transfer)
    }
}
impl std::io::Write for DictionaryReplay {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.write(bytes).map_err(replay_transfer)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush().map_err(replay_io)
    }
}
impl std::io::Seek for DictionaryReplay {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(position).map_err(replay_io)
    }
}
impl cbor::ReplayStorage for DictionaryReplay {
    fn finish(&mut self) -> std::io::Result<()> {
        self.0.finish().map_err(replay_io)
    }
}

impl Folder<'_, '_, '_> {
    pub(super) fn spool_snapshot(
        &mut self,
        _frame: &[(Value, Value)],
        index: u64,
        data: Option<ValueRef<'_>>,
        ids: &[Value],
        scratch: std::io::Result<Box<dyn cbor::ReplayStorage>>,
    ) -> Result<(), PayloadError> {
        let stored = self.stored_terms.is_some();
        let mut scratch: Box<dyn cbor::ReplayStorage> = if stored {
            Box::new(DictionaryReplay(
                scratch.map_err(crate::dictionary::DictionaryError::from)?,
            ))
        } else {
            scratch?
        };
        let bytes = data
            .and_then(ValueRef::byte_chunks)
            .ok_or(PayloadError::Damaged(PayloadCause::TransformedNotBytes))?;
        let codecs = super::payload::resolve_codecs(&self.catalog, ids)?;
        let limit = self.decode_limit(false).unwrap_or(usize::MAX);
        let mut source = crate::codec::decode_chain_reader(&codecs, bytes, limit)?;
        let spooled = cbor::spool_item(&mut source, &mut *scratch, cbor::Limits::DEFAULT)?;
        let mut replay = cbor::ReplayBuffer::new(scratch)?;
        self.h_snapshot_range(spooled.item(), &mut replay, index)
            .map_err(|error| match error {
                PayloadError::Damaged(PayloadCause::Cbor(cause)) if stored => {
                    PayloadError::Dictionary(crate::dictionary::DictionaryError::Replay(cause))
                }
                error => error,
            })?;
        spooled.verify(&mut replay).map_err(|cause| {
            if stored {
                PayloadError::Dictionary(crate::dictionary::DictionaryError::Replay(cause))
            } else {
                cause.into()
            }
        })?;
        replay.into_inner().finish()?;
        Ok(())
    }

    fn h_snapshot_range<R: Read + std::io::Seek + ?Sized>(
        &mut self,
        payload: cbor::ItemRange,
        replay: &mut R,
        index: u64,
    ) -> Result<(), PayloadError> {
        if payload.major(replay)? != 5 {
            return Ok(());
        }
        let base = TermIdBase(self.terms().len());
        let [terms, quads, reifies, annot, blobs, meta] = payload.first_fields(
            replay,
            ["terms", "quads", "reifies", "annot", "blobs", "meta"],
        )?;
        for (kind, field) in [(0, terms), (1, quads), (2, reifies), (3, annot)] {
            let Some(field) = field else {
                continue;
            };
            if kind == 2 && field.major(replay)? == 5 {
                self.diag(FoldError::ReifiesArray { snapshot: true }, index);
                continue;
            }
            let Some(mut rows) = field.array(replay)? else {
                continue;
            };
            while let Some(row) = rows.next(replay)? {
                let value = row.decode(replay)?;
                match kind {
                    0 => self.h_term_row(&value, index, base)?,
                    1 => self.h_quad_row(ValueRef::Owned(&value), index, base)?,
                    2 => self.h_reifier_row(ValueRef::Owned(&value), index, base)?,
                    3 => self.h_annotation_row(ValueRef::Owned(&value), index, base)?,
                    _ => unreachable!("the snapshot row phases are a fixed native list"),
                }
            }
        }
        if let Some(blobs) = blobs
            && let Some(mut entries) = blobs.map(replay)?
        {
            let ceiling = self.decode_limit(true);
            while let Some((_, value)) = entries.next(replay)? {
                let Some(mut bytes) = value.byte_reader(replay)? else {
                    continue;
                };
                let mut digest = purrdf_hash::blake3::Hasher::new();
                let mut length = 0usize;
                let mut scratch = [0; 8192];
                loop {
                    let count = match bytes.read(&mut scratch) {
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        result => result?,
                    };
                    if count == 0 {
                        break;
                    }
                    digest.update(&scratch[..count]);
                    // Payload chunks are disjoint inside one admitted item's
                    // addressable extent, so their sum fits usize.
                    length += count;
                }
                let digest = digest_label(digest.finalize().as_bytes());
                if let Some(limit) = ceiling
                    && length > limit
                {
                    let error = PayloadError::Budget(PayloadCause::SnapshotBlobLimit {
                        actual: length,
                        limit,
                    });
                    let detail = error.diagnostic_detail();
                    self.emit_blob_refusal(
                        BlobRefusalKind::ByteBudget,
                        Some(&digest),
                        true,
                        None,
                        length,
                        &detail,
                    );
                    self.diag_native(
                        error.diagnostic_code(BLOB_BUDGET_DIAGNOSTIC),
                        error,
                        detail,
                        index,
                    );
                    continue;
                }
                let mut bytes = value
                    .byte_reader(replay)?
                    .ok_or(PayloadError::Damaged(PayloadCause::BlobNotBytes))?;
                if let Some(sink) = self.sink.as_deref_mut() {
                    let metadata = self.blob_meta_index.get(&self.g.blob_meta, &digest);
                    sink.blob(self.segment_index, &digest, metadata);
                    sink.blob_payload(BlobPayload {
                        segment_index: self.segment_index,
                        digest: &digest,
                        digest_computed: true,
                        metadata,
                        metadata_declared: false,
                        bytes: BlobContent::Reader {
                            source: &mut bytes,
                            len: length,
                        },
                        payload_present: true,
                        encoded_len: length,
                        codecs: &[],
                    });
                }
            }
        }
        if let Some(meta) = meta
            && let Some(mut entries) = meta.map(replay)?
        {
            while let Some((key, value)) = entries.next(replay)? {
                let key = key.decode(replay)?;
                let key = key
                    .as_text()
                    .map_or_else(|| format!("{key:?}"), str::to_owned);
                self.g.set_meta(key, value.decode(replay)?);
            }
        }
        Ok(())
    }
}

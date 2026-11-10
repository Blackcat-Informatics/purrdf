// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One payload/capability decoder for sequential and indexed frame admission.

use super::{
    ContentKeyResolver, FastMap, Payload, PayloadCause, PayloadError, Value, ValueRef, as_i128,
    cbor, decrypt_codec, map_get,
};
use crate::codec::{Codec, decode_chain, decode_chain_bounded, decode_chain_with_decrypt_bounded};
use purrdf_lex::cbor::EncodedItem;
use std::borrow::Cow;

pub(super) fn resolve_codecs(
    catalog: &FastMap<i128, Codec>,
    ids: &[Value],
) -> Result<Vec<Codec>, PayloadError> {
    let mut chain = Vec::with_capacity(ids.len());
    for cid in ids {
        let codec = as_i128(cid).and_then(|c| catalog.get(&c));
        match codec {
            Some(c) => chain.push(c.clone()),
            None => {
                return Err(PayloadError::Unavailable {
                    reason: "unknown-codec",
                    detail: PayloadCause::MissingCatalog(cid.clone()),
                });
            }
        }
    }
    Ok(chain)
}

pub(super) struct PayloadContext<'a, 'k> {
    pub(super) catalog: &'a FastMap<i128, Codec>,
    pub(super) content_key: Option<&'k ContentKeyResolver<'k>>,
}
impl PayloadContext<'_, '_> {
    pub(super) fn decode<'frame>(
        &self,
        frame: &'frame [(Value, Value)],
        blob: bool,
        data: Option<ValueRef<'frame>>,
        decoded_limit: impl FnOnce() -> Option<usize>,
        encoded_blob_limit: impl FnOnce() -> Option<usize>,
    ) -> Result<Payload<'frame>, PayloadError> {
        let d = data;
        let bytes = d
            .map(ValueRef::bytes)
            .transpose()
            .map(Option::flatten)
            .map_err(|cause| PayloadError::Damaged(PayloadCause::Cbor(cause)))?;
        if blob
            && let Some(bytes) = bytes.as_deref()
            && let Some(limit) = encoded_blob_limit()
            && bytes.len() > limit
        {
            return Err(PayloadError::Budget(PayloadCause::EncodedBlobLimit {
                actual: bytes.len(),
                limit,
            }));
        }
        if let Some(Value::Array(ids)) = map_get(frame, "x")
            && !ids.is_empty()
        {
            let Some(db) = bytes.as_deref() else {
                return Err(PayloadError::Damaged(PayloadCause::TransformedNotBytes));
            };
            let chain = resolve_codecs(self.catalog, ids)?;
            // A blob frame is bounded by the blob ceiling; every other frame by
            // the frame ceiling, which is what keeps an embedded snapshot blob
            // map from decoding without limit.
            let limit = decoded_limit();
            let decoded = if let Some(content_key) = self.content_key {
                let limit = limit.unwrap_or(usize::MAX);
                let decrypt =
                    |codec: &Codec, data: &[u8]| decrypt_codec(codec, data, content_key, limit);
                decode_chain_with_decrypt_bounded(&chain, db, Some(&decrypt), limit)?
            } else if let Some(limit) = limit {
                decode_chain_bounded(&chain, db, limit)?
            } else {
                decode_chain(&chain, db)?
            };
            if blob {
                return Ok(Payload::Blob(Cow::Owned(decoded)));
            }
            return EncodedItem::from_bytes(decoded, cbor::Limits::DEFAULT)
                .map(Payload::Encoded)
                .map_err(|e| PayloadError::Damaged(PayloadCause::Cbor(e)));
        }
        if blob
            && let Some(bytes) = bytes.as_deref()
            && let Some(limit) = decoded_limit()
            && bytes.len() > limit
        {
            return Err(PayloadError::Budget(PayloadCause::DecodedBlobLimit {
                actual: bytes.len(),
                limit,
            }));
        }
        Ok(Payload::Value(d))
    }
}

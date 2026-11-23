// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One payload/capability decoder for sequential and indexed frame admission.

use super::{
    ContentKeyResolver, FastMap, Payload, PayloadCause, PayloadError, Value, ValueRef, as_i128,
    cbor, decrypt_codec, map_get,
};
use crate::codec::{Codec, decode_chain_with_decrypt_bounded_observed};
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

pub(super) trait TransformAdmission {
    fn limit(&self) -> Option<usize>;
    fn produced(&mut self, bytes: usize) -> bool;
}

pub(super) struct FixedTransformAdmission {
    limit: usize,
}

impl FixedTransformAdmission {
    pub(super) const fn new(limit: usize) -> Self {
        Self { limit }
    }
}

impl TransformAdmission for FixedTransformAdmission {
    fn limit(&self) -> Option<usize> {
        Some(self.limit)
    }

    fn produced(&mut self, _bytes: usize) -> bool {
        true
    }
}

impl PayloadContext<'_, '_> {
    pub(super) fn decode<'frame>(
        &self,
        frame: &'frame [(Value, Value)],
        blob: bool,
        data: Option<ValueRef<'frame>>,
        admission: &mut dyn TransformAdmission,
        encoded_blob_limit: impl FnOnce() -> Option<usize>,
    ) -> Result<Payload<'frame>, PayloadError> {
        let d = data;
        let bytes = d
            .map(ValueRef::bytes)
            .transpose()
            .map(Option::flatten)
            .map_err(|cause| PayloadError::Damaged(PayloadCause::Cbor(cause)))?;
        let limit = admission.limit();
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
            let decoded = if let Some(content_key) = self.content_key {
                let decrypt = |codec: &Codec, data: &[u8], limit: usize| {
                    decrypt_codec(codec, data, content_key, limit)
                };
                decode_chain_with_decrypt_bounded_observed(
                    &chain,
                    db,
                    Some(&decrypt),
                    |produced| {
                        if let Some(bytes) = produced
                            && !admission.produced(bytes)
                        {
                            return Err(PayloadError::Cancelled);
                        }
                        Ok(admission.limit().unwrap_or(usize::MAX))
                    },
                    PayloadError::from,
                )?
            } else {
                decode_chain_with_decrypt_bounded_observed(
                    &chain,
                    db,
                    None,
                    |produced| {
                        if let Some(bytes) = produced
                            && !admission.produced(bytes)
                        {
                            return Err(PayloadError::Cancelled);
                        }
                        Ok(admission.limit().unwrap_or(usize::MAX))
                    },
                    PayloadError::from,
                )?
            };
            if blob {
                return Ok(Payload::Blob(Cow::Owned(decoded)));
            }
            let actual = decoded.len();
            return EncodedItem::from_bytes(decoded, cbor::Limits::DEFAULT)
                .map(Payload::Encoded)
                .map_err(|cause| {
                    PayloadError::Damaged(PayloadCause::DecodedCbor { actual, cause })
                });
        }
        if blob
            && let Some(bytes) = bytes.as_deref()
            && let Some(limit) = limit
            && bytes.len() > limit
        {
            return Err(PayloadError::Budget(PayloadCause::DecodedBlobLimit {
                actual: bytes.len(),
                limit,
            }));
        }
        if !blob
            && let Some(value) = d
            && let Some(limit) = limit
        {
            let actual = match value {
                ValueRef::Owned(value) => cbor::encoded_len(value),
                ValueRef::Encoded(value) => value.encoded().len(),
            };
            if actual > limit {
                return Err(PayloadError::Budget(PayloadCause::DecodedFrameLimit {
                    actual,
                    limit,
                }));
            }
        }
        Ok(Payload::Value(d))
    }
}

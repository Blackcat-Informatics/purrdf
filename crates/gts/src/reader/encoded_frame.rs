// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Encoded frame ownership and source-backed RDF payload selection.

use purrdf_lex::cbor::{self, BorrowedItem, EncodedItem, Value, ValueRef};

pub(super) enum StreamItem {
    Header(Value),
    Frame(EncodedItem),
}

pub(super) fn classify(item: EncodedItem) -> Result<StreamItem, cbor::DecodeError> {
    let raw = ValueRef::Encoded(item.view());
    let inner = raw.tag().map_or(raw, |(_, item)| item);
    if inner.is_map() && inner.get("gts")?.is_some() && inner.get("t")?.is_none() {
        return item.decode().map(StreamItem::Header);
    }
    Ok(StreamItem::Frame(item))
}

pub(super) struct Frame<'a> {
    pub(super) metadata: Value,
    pub(super) borrowed_data: bool,
    pub(super) data: Option<ValueRef<'a>>,
    pub(super) computed: Option<Vec<u8>>,
}

impl<'a> Frame<'a> {
    pub(super) fn new(item: BorrowedItem<'a>) -> Result<Self, cbor::DecodeError> {
        let raw = ValueRef::Encoded(item);
        let Some(entries) = raw.map()? else {
            // The actual refusal carries no rendered source value. Retaining a
            // whole non-map item would spend memory to produce the same frame
            // provenance and FoldError::FrameMap diagnostic.
            return Ok(Self {
                metadata: Value::Null,
                borrowed_data: false,
                data: None,
                computed: None,
            });
        };
        // Retain the outer map's source spans once. Looking up each
        // metadata field by walking the encoded map would repeatedly scan a
        // large earlier payload, even though no owned tree was retained.
        let entries: Vec<_> = entries.into_iter().collect::<Result<_, _>>()?;
        let lookup = |name: &str| -> Result<Option<ValueRef<'a>>, cbor::DecodeError> {
            for &(key, value) in &entries {
                if key.text()?.is_some_and(|key| key == name) {
                    return Ok(Some(value));
                }
            }
            Ok(None)
        };
        let frame_type = lookup("t")?.map(ValueRef::text).transpose()?.flatten();
        // Blob callbacks retain the existing eager/lazy byte contract. Every
        // other frame can inspect its logical payload through the same view,
        // including unknown frames that still resolve capabilities before refusal.
        let borrowed_data = frame_type.as_deref() != Some("blob");
        let data = if borrowed_data { lookup("d")? } else { None };
        let mut metadata = Vec::new();
        for entry in entries {
            let (key, value) = entry;
            if borrowed_data && key.text()?.is_some_and(|key| key == "d") {
                continue;
            }
            metadata.push((key.to_value()?.into_owned(), value.to_value()?.into_owned()));
        }
        let computed = match crate::wire::content_id_encoded(item) {
            Ok(id) => id,
            Err(cbor::WriteError::Decode(error)) => return Err(error),
            Err(cbor::WriteError::Io(error)) => {
                unreachable!("the BLAKE3 writer cannot fail: {error}")
            }
            Err(cbor::WriteError::ExpectedMap) => {
                unreachable!("the encoded frame was checked as a map")
            }
        };
        Ok(Self {
            metadata: Value::Map(metadata),
            borrowed_data,
            data,
            computed: Some(computed),
        })
    }
}

pub(super) enum Payload<'a> {
    Value(Option<ValueRef<'a>>),
    Encoded(EncodedItem),
    Blob(std::borrow::Cow<'a, [u8]>),
}

static NULL: Value = Value::Null;

impl Payload<'_> {
    pub(super) fn view(&self) -> ValueRef<'_> {
        match self {
            Self::Value(value) => value.unwrap_or(ValueRef::Owned(&NULL)),
            Self::Encoded(value) => ValueRef::Encoded(value.view()),
            Self::Blob(_) => unreachable!("only an RDF payload is inspected as CBOR"),
        }
    }
}

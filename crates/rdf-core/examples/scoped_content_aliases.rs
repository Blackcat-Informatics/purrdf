// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A native host presents original content and provenance with scoped ordinals,
//! resolves structured replies, then matches full identities across datasets.

use purrdf_core::content_alias::{AliasScope, ScopeId};
use purrdf_core::visual_fingerprint::{self, Request};
use purrdf_hash::{blake3, hex::Digest32};
use purrdf_lex::{
    allocation::{Admission, Memory, OwnedAdmission, StorageError},
    json::{self, Object, Value},
};
use std::io::Write as _;

struct Authority {
    max_bytes: usize,
    remaining_work: u64,
}
impl Authority {
    fn bounded() -> Self {
        Self {
            max_bytes: 1 << 20,
            remaining_work: 1 << 20,
        }
    }
}
impl Admission for Authority {
    fn progress_enabled(&self) -> bool {
        true
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        self.remaining_work = self
            .remaining_work
            .checked_sub(units)
            .ok_or(StorageError::AdmissionFailed)?;
        Ok(())
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes <= self.max_bytes {
            Ok(())
        } else {
            Err(StorageError::AdmissionFailed)
        }
    }
}

struct Document<'a> {
    digest: Digest32,
    content: &'a str,
    provenance: &'a str,
}
fn document<'a>(content: &'a str, provenance: &'a str) -> Document<'a> {
    Document {
        digest: Digest32::new(*blake3::hash(content.as_bytes()).as_bytes()),
        content,
        provenance,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let left = [document("original alpha", "source A, row 7")];
    let right = [
        document("original beta", "source B, row 2"),
        document("original alpha", "source B, row 9"),
    ];
    // The host owns these namespaces for the lifetime of each assignment history.
    let left_scope = ScopeId::new(Digest32::new([1; 32]));
    let right_scope = ScopeId::new(Digest32::new([2; 32]));
    let mut left_map = AliasScope::new(left_scope, Authority::bounded())?;
    let mut right_map = AliasScope::new(right_scope, Authority::bounded())?;
    if let Some(cause) = left_map
        .extend(left_scope, &[left[0].digest])?
        .cleanup_refusal
    {
        return Err(cause.into());
    }
    if let Some(cause) = right_map
        .extend(right_scope, &[right[0].digest, right[1].digest])?
        .cleanup_refusal
    {
        return Err(cause.into());
    }
    let left_envelope = left_map.presentation(left_scope)?;
    let right_envelope = right_map.presentation(right_scope)?;

    // This caller-owned account also covers the request and parsed replies.
    // It is declared first and therefore destroyed after all its local payloads.
    let mut wire_owner = OwnedAdmission::new(Authority::bounded());
    let mut memory = Memory::new(&mut wire_owner);
    let references = left_map.references(left_scope)?;
    let mut rows = Vec::new();
    for (reference, original) in references.iter().zip(&left) {
        let row = Object::new()
            .with_memory("alias", reference.alias.ordinal(), &mut memory)?
            .with_memory("content", original.content, &mut memory)?
            .with_memory("provenance", original.provenance, &mut memory)?;
        memory.push(&mut rows, Value::Object(row))?;
    }
    references.release()?;
    let request = Value::Array(rows);
    let rendered = json::write_with_memory(&request, json::Format::COMPACT, &mut memory)?;
    assert!(rendered.contains("\"alias\":1"));
    assert!(rendered.contains("source A, row 7"));

    // A provider returns only short ordinals. The trusted host retains the
    // original envelopes and checks replies before any native content operation.
    let response_text = r#"{"aliases":[1,2]}"#;
    memory
        .admission_mut()
        .checkpoint(response_text.len() as u64)?;
    let mut reader = json::Reader::new(
        response_text,
        json::Limits {
            max_depth: 2,
            max_values: 4,
            max_string_bytes: 7,
            unique_members: true,
        },
    );
    let right_response = reader.read_value_with_memory(&mut memory)?;
    reader.finish_with_memory(&mut memory)?;
    reader.release_with_memory(&mut memory)?;
    let mut reader = json::Reader::new(
        r#"{"aliases":[1]}"#,
        json::Limits {
            max_depth: 2,
            max_values: 3,
            max_string_bytes: 7,
            unique_members: true,
        },
    );
    let left_response = reader.read_value_with_memory(&mut memory)?;
    reader.finish_with_memory(&mut memory)?;
    reader.release_with_memory(&mut memory)?;
    let selected_left = left_map.resolve_response(left_envelope, &left_response)?;
    let selected_right = right_map.resolve_response(right_envelope, &right_response)?;
    assert_ne!(selected_left[0], selected_right[0]); // Both were bare ordinal one.
    assert_eq!(selected_left[0], selected_right[1]); // Full identities match.
    let original_left = left
        .iter()
        .find(|item| item.digest == selected_left[0])
        .ok_or("missing original left content")?;
    let original_right = right
        .iter()
        .find(|item| item.digest == selected_right[1])
        .ok_or("missing original right content")?;
    assert_eq!(original_left.content, original_right.content);
    assert_eq!(original_left.provenance, "source A, row 7");
    assert_eq!(original_right.provenance, "source B, row 9");
    selected_left.release()?;
    selected_right.release()?;

    // Scope changes the short reference, never the presentation of the resolved
    // authoritative digest. No alias spelling enters the rendering algorithm.
    let left_reference = left_map.reference(left_scope, left[0].digest)?;
    let right_reference = right_map.reference(right_scope, left[0].digest)?;
    let visual_request = Request::new(11, "BLAKE3 256")?;
    let left_visual = visual_fingerprint::present(
        left_map.resolve(left_reference)?,
        visual_request,
        Authority::bounded(),
    )?;
    let right_visual = visual_fingerprint::present(
        right_map.resolve(right_reference)?,
        visual_request,
        Authority::bounded(),
    )?;
    let left_rendering = left_visual.rendering();
    let right_rendering = right_visual.rendering();
    assert_eq!(
        (
            left_rendering.emoji(),
            left_rendering.spoken(),
            left_rendering.randomart()
        ),
        (
            right_rendering.emoji(),
            right_rendering.spoken(),
            right_rendering.randomart()
        ),
    );
    for (reference, visual) in [
        (left_reference, &left_visual),
        (right_reference, &right_visual),
    ] {
        let rendering = visual.rendering();
        writeln!(
            std::io::stdout().lock(),
            "digest {} scope {} alias {} scheme {} symbols {} emoji {} spoken {}",
            visual.digest(),
            reference.scope.digest(),
            reference.alias.ordinal(),
            rendering.scheme(),
            rendering.symbols(),
            rendering.emoji(),
            rendering.spoken()
        )?;
    }
    left_visual.release()?;
    right_visual.release()?;

    // Persist these bytes with the host's original scope identity, then restore
    // them in a later process without allocating different aliases.
    let exported = left_map.export(left_scope)?;
    let mut restored = AliasScope::new(left_scope, Authority::bounded())?;
    if let Some(cause) = restored.restore(left_scope, &exported)?.cleanup_refusal {
        return Err(cause.into());
    }
    assert_eq!(
        restored
            .reference(left_scope, left[0].digest)?
            .alias
            .ordinal(),
        1
    );
    exported.release()?;
    Ok(())
}

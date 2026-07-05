// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Merkle-Mountain-Range commitments for `index.mmr` and detached proof JSON.
//!
//! This module deliberately stays dependency-light: detached proof JSON is parsed
//! with a tiny schema-local parser instead of a general JSON library.

use ciborium::value::Value;

use crate::wire::{
    MAGIC, VERSION, blake3_256, canonical, content_id, header_id, hex, iter_items, map_get,
    unwrap_header,
};

/// Stable detached proof schema tag emitted by [`Proof::to_json`].
pub const PROOF_SCHEMA: &str = "gts-mmr-proof-v1";
const HASH_ALGORITHM: &str = "blake3-256";
const PREIMAGE_VERSION: &str = "gts-mmr-v1";
const LEAF_DOMAIN: &str = "gts-mmr-leaf-v1";
const PARENT_DOMAIN: &str = "gts-mmr-parent-v1";
const ROOT_DOMAIN: &str = "gts-mmr-root-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MmrPeak {
    /// Peak tree height. Height zero is a leaf.
    pub height: usize,
    /// Peak hash using the `gts-mmr-*` preimage domains.
    pub hash: Vec<u8>,
}

/// Position of a proof sibling relative to the carried node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofSide {
    /// Sibling hash is the left child.
    Left,
    /// Sibling hash is the right child.
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofStep {
    /// Height of the parent node created by this step.
    pub parent_height: usize,
    /// Which side the sibling hash occupies relative to the carried node.
    pub side: ProofSide,
    /// Sibling hash at this step.
    pub hash: Vec<u8>,
}

/// Detached inclusion proof for one frame id in an indexed segment.
///
/// The proof binds a frame id to the `index.mmr` root without requiring the
/// original GTS bytes. `count`, `peaks`, and `path` are enough to reconstruct
/// the selected peak and then the segment root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// Number of frame ids committed by the index root.
    pub count: usize,
    /// Zero-based leaf/frame index proven by this proof.
    pub leaf_index: usize,
    /// 32-byte frame content id at `leaf_index`.
    pub frame_id: Vec<u8>,
    /// 32-byte MMR root from the `index.mmr` footer.
    pub root: Vec<u8>,
    /// Index into [`Self::peaks`] containing `leaf_index`.
    pub peak_index: usize,
    /// Complete peak list for the committed frame count.
    pub peaks: Vec<MmrPeak>,
    /// Sibling path from the leaf to the selected peak.
    pub path: Vec<ProofStep>,
}

#[derive(Clone, Debug)]
struct Node {
    height: usize,
    start: usize,
    end: usize,
    hash: Vec<u8>,
    left: Option<Box<Self>>,
    right: Option<Box<Self>>,
}

fn uint(n: usize) -> Value {
    Value::from(n as u64)
}

fn leaf_hash(index: usize, frame_id: &[u8]) -> Vec<u8> {
    blake3_256(&canonical(&Value::Array(vec![
        LEAF_DOMAIN.into(),
        uint(index),
        Value::Bytes(frame_id.to_vec()),
    ])))
}

fn parent_hash(parent_height: usize, left: &[u8], right: &[u8]) -> Vec<u8> {
    blake3_256(&canonical(&Value::Array(vec![
        PARENT_DOMAIN.into(),
        uint(parent_height),
        Value::Bytes(left.to_vec()),
        Value::Bytes(right.to_vec()),
    ])))
}

fn root_hash(count: usize, peaks: &[MmrPeak]) -> Vec<u8> {
    let peak_values: Vec<Value> = peaks
        .iter()
        .map(|peak| Value::Array(vec![uint(peak.height), Value::Bytes(peak.hash.clone())]))
        .collect();
    blake3_256(&canonical(&Value::Array(vec![
        ROOT_DOMAIN.into(),
        uint(count),
        Value::Array(peak_values),
    ])))
}

fn build_nodes(frame_ids: &[Vec<u8>]) -> Vec<Node> {
    let mut peaks: Vec<Node> = Vec::new();
    for (index, frame_id) in frame_ids.iter().enumerate() {
        peaks.push(Node {
            height: 0,
            start: index,
            end: index + 1,
            hash: leaf_hash(index, frame_id),
            left: None,
            right: None,
        });
        while peaks.len() >= 2 {
            let right_i = peaks.len() - 1;
            let left_i = peaks.len() - 2;
            if peaks[left_i].height != peaks[right_i].height {
                break;
            }
            // MMR append invariant: only the newest adjacent equal-height
            // peaks can merge, preserving append-order coverage ranges.
            let right = peaks.pop().expect("right peak exists");
            let left = peaks.pop().expect("left peak exists");
            let height = left.height + 1;
            let hash = parent_hash(height, &left.hash, &right.hash);
            peaks.push(Node {
                height,
                start: left.start,
                end: right.end,
                hash,
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            });
        }
    }
    peaks
}

fn peak_list(nodes: &[Node]) -> Vec<MmrPeak> {
    nodes
        .iter()
        .map(|node| MmrPeak {
            height: node.height,
            hash: node.hash.clone(),
        })
        .collect()
}

/// Compute the stable `index.mmr` root over ordered frame ids.
///
/// The root commits to both the frame count and the ordered peak list, so
/// adding a frame changes the root even when an earlier proof path is reused.
pub fn root(frame_ids: &[Vec<u8>]) -> Vec<u8> {
    let nodes = build_nodes(frame_ids);
    root_hash(frame_ids.len(), &peak_list(&nodes))
}

fn append_path(node: &Node, target: usize, path: &mut Vec<ProofStep>) -> bool {
    if node.height == 0 {
        return node.start == target;
    }
    let (Some(left), Some(right)) = (&node.left, &node.right) else {
        return false;
    };
    if target < left.end {
        if append_path(left, target, path) {
            path.push(ProofStep {
                parent_height: node.height,
                side: ProofSide::Right,
                hash: right.hash.clone(),
            });
            return true;
        }
    } else if append_path(right, target, path) {
        path.push(ProofStep {
            parent_height: node.height,
            side: ProofSide::Left,
            hash: left.hash.clone(),
        });
        return true;
    }
    false
}

/// Create a detached inclusion proof for `target_index`.
///
/// Returns `None` when the target is outside the covered frame id list.
pub fn prove(frame_ids: &[Vec<u8>], target_index: usize) -> Option<Proof> {
    if target_index >= frame_ids.len() {
        return None;
    }
    let nodes = build_nodes(frame_ids);
    let peaks = peak_list(&nodes);
    let peak_index = nodes
        .iter()
        .position(|node| target_index >= node.start && target_index < node.end)?;
    let mut path = Vec::new();
    if !append_path(&nodes[peak_index], target_index, &mut path) {
        return None;
    }
    Some(Proof {
        count: frame_ids.len(),
        leaf_index: target_index,
        frame_id: frame_ids[target_index].clone(),
        root: root_hash(frame_ids.len(), &peaks),
        peak_index,
        peaks,
        path,
    })
}

fn expected_peak_heights(count: usize) -> Vec<usize> {
    let mut remaining = count;
    let mut heights = Vec::new();
    while remaining > 0 {
        let height = (usize::BITS - 1 - remaining.leading_zeros()) as usize;
        heights.push(height);
        remaining -= 1usize << height;
    }
    heights
}

fn peak_width(height: usize) -> Result<usize, String> {
    let shift = u32::try_from(height).map_err(|_| format!("peak height {height} is too large"))?;
    1usize
        .checked_shl(shift)
        .ok_or_else(|| format!("peak height {height} is too large"))
}

fn peak_index_for_leaf(
    count: usize,
    heights: &[usize],
    leaf_index: usize,
) -> Result<usize, String> {
    if leaf_index >= count {
        return Err(format!(
            "leaf_index {leaf_index} is outside covered count {count}"
        ));
    }
    let mut start = 0usize;
    for (index, height) in heights.iter().enumerate() {
        let width = peak_width(*height)?;
        let end = start
            .checked_add(width)
            .ok_or_else(|| "peak ranges overflow usize".to_string())?;
        if leaf_index >= start && leaf_index < end {
            return Ok(index);
        }
        start = end;
    }
    Err(format!(
        "peak ranges do not cover leaf_index {leaf_index} for count {count}"
    ))
}

/// Verify a detached proof without access to the original GTS file.
///
/// Verification checks shape first, then recomputes the leaf-to-peak path and
/// final root using the same domain-separated preimages as [`root`].
pub fn verify_proof(proof: &Proof) -> Result<(), String> {
    if proof.frame_id.len() != 32 {
        return Err("frame_id must be 32 bytes".to_string());
    }
    if proof.root.len() != 32 {
        return Err("root must be 32 bytes".to_string());
    }
    if proof.leaf_index >= proof.count {
        return Err(format!(
            "leaf_index {} is outside covered count {}",
            proof.leaf_index, proof.count
        ));
    }
    if proof.peak_index >= proof.peaks.len() {
        return Err(format!("peak_index {} is out of range", proof.peak_index));
    }
    let expected_heights = expected_peak_heights(proof.count);
    let actual_heights: Vec<usize> = proof.peaks.iter().map(|peak| peak.height).collect();
    if actual_heights != expected_heights {
        return Err(format!(
            "peak heights {:?} do not match count {}",
            actual_heights, proof.count
        ));
    }
    let computed_peak_index = peak_index_for_leaf(proof.count, &actual_heights, proof.leaf_index)?;
    if computed_peak_index != proof.peak_index {
        return Err(format!(
            "leaf_index {} belongs to peak {}, not {}",
            proof.leaf_index, computed_peak_index, proof.peak_index
        ));
    }
    for peak in &proof.peaks {
        if peak.hash.len() != 32 {
            return Err("peak hash must be 32 bytes".to_string());
        }
    }
    let mut carried = leaf_hash(proof.leaf_index, &proof.frame_id);
    let mut height = 0usize;
    for step in &proof.path {
        if step.hash.len() != 32 {
            return Err("path hash must be 32 bytes".to_string());
        }
        if step.parent_height != height + 1 {
            return Err(format!(
                "path parent height {} does not follow height {}",
                step.parent_height, height
            ));
        }
        carried = match step.side {
            ProofSide::Left => parent_hash(step.parent_height, &step.hash, &carried),
            ProofSide::Right => parent_hash(step.parent_height, &carried, &step.hash),
        };
        height = step.parent_height;
    }
    let peak = &proof.peaks[proof.peak_index];
    if height != peak.height {
        return Err(format!(
            "path height {height} does not reach peak height {}",
            peak.height
        ));
    }
    if carried != peak.hash {
        return Err("proof path does not reconstruct the selected peak".to_string());
    }
    let computed_root = root_hash(proof.count, &proof.peaks);
    if computed_root != proof.root {
        return Err("proof peaks do not reconstruct the declared root".to_string());
    }
    Ok(())
}

fn json_escape(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

impl Proof {
    /// Render the stable detached proof JSON form.
    pub fn to_json(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        out.push_str("{\n");
        let _ = writeln!(out, "  \"schema\": \"{}\",", json_escape(PROOF_SCHEMA));
        let _ = writeln!(out, "  \"hash\": \"{}\",", json_escape(HASH_ALGORITHM));
        let _ = writeln!(
            out,
            "  \"preimage\": \"{}\",",
            json_escape(PREIMAGE_VERSION)
        );
        let _ = writeln!(out, "  \"count\": {},", self.count);
        let _ = writeln!(out, "  \"leaf_index\": {},", self.leaf_index);
        let _ = writeln!(out, "  \"frame_id\": \"{}\",", hex(&self.frame_id));
        let _ = writeln!(out, "  \"root\": \"{}\",", hex(&self.root));
        let _ = writeln!(out, "  \"peak_index\": {},", self.peak_index);
        out.push_str("  \"peaks\": [\n");
        for (index, peak) in self.peaks.iter().enumerate() {
            let _ = writeln!(
                out,
                "    {{\"height\": {}, \"hash\": \"{}\"}}{}",
                peak.height,
                hex(&peak.hash),
                if index + 1 == self.peaks.len() {
                    ""
                } else {
                    ","
                }
            );
        }
        out.push_str("  ],\n");
        out.push_str("  \"path\": [\n");
        for (index, step) in self.path.iter().enumerate() {
            let side = match step.side {
                ProofSide::Left => "left",
                ProofSide::Right => "right",
            };
            let _ = writeln!(
                out,
                "    {{\"side\": \"{}\", \"parent_height\": {}, \"hash\": \"{}\"}}{}",
                side,
                step.parent_height,
                hex(&step.hash),
                if index + 1 == self.path.len() {
                    ""
                } else {
                    ","
                }
            );
        }
        out.push_str("  ]\n");
        out.push_str("}\n");
        out
    }

    /// Parse the stable detached proof JSON form.
    pub fn from_json(text: &str) -> Result<Self, String> {
        proof_from_json(text)
    }
}

#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool,
    Number(u64),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<Json, String> {
        let value = self.value()?;
        self.ws();
        if self.pos != self.bytes.len() {
            return Err(format!("trailing JSON at byte {}", self.pos));
        }
        Ok(value)
    }

    fn ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.bytes.get(self.pos).copied() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::String),
            Some(b'0'..=b'9') => self.number().map(Json::Number),
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Json::Bool)
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Json::Bool)
            }
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Json::Null)
            }
            Some(other) => Err(format!(
                "unexpected JSON byte {other:?} at byte {}",
                self.pos
            )),
            None => Err("unexpected end of JSON".to_string()),
        }
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.pos..self.pos + literal.len()) == Some(literal) {
            self.pos += literal.len();
            Ok(())
        } else {
            Err(format!("expected JSON literal at byte {}", self.pos))
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut entries = Vec::new();
        self.ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            if self.bytes.get(self.pos) != Some(&b':') {
                return Err(format!("expected ':' at byte {}", self.pos));
            }
            self.pos += 1;
            let value = self.value()?;
            entries.push((key, value));
            self.ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.pos)),
            }
        }
        Ok(Json::Object(entries))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut items = Vec::new();
        self.ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.pos)),
            }
        }
        Ok(Json::Array(items))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(format!("expected string at byte {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        while let Some(byte) = self.bytes.get(self.pos).copied() {
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = self
                        .bytes
                        .get(self.pos)
                        .copied()
                        .ok_or_else(|| "unterminated JSON escape".to_string())?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let cp = self.hex4()?;
                            let ch = char::from_u32(cp)
                                .ok_or_else(|| format!("invalid unicode escape {cp:04x}"))?;
                            out.push(ch);
                        }
                        _ => return Err(format!("invalid JSON escape '\\{}'", esc as char)),
                    }
                }
                0x00..=0x1f => return Err("control byte in JSON string".to_string()),
                _ => out.push(byte as char),
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let start = self.pos;
        let mut value = 0u32;
        for _ in 0..4 {
            let byte = self
                .bytes
                .get(self.pos)
                .copied()
                .ok_or_else(|| "short unicode escape".to_string())?;
            self.pos += 1;
            value = (value << 4)
                | match byte {
                    b'0'..=b'9' => u32::from(byte - b'0'),
                    b'a'..=b'f' => u32::from(byte - b'a' + 10),
                    b'A'..=b'F' => u32::from(byte - b'A' + 10),
                    _ => return Err(format!("invalid unicode escape at byte {start}")),
                };
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<u64, String> {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'0') {
            self.pos += 1;
        } else {
            while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|e| format!("invalid number bytes: {e}"))?;
        text.parse::<u64>()
            .map_err(|e| format!("invalid JSON integer {text:?}: {e}"))
    }
}

fn object_entries<'a>(json: &'a Json, context: &str) -> Result<&'a [(String, Json)], String> {
    match json {
        Json::Object(entries) => Ok(entries),
        _ => Err(format!("{context} must be a JSON object")),
    }
}

fn array_items<'a>(json: &'a Json, context: &str) -> Result<&'a [Json], String> {
    match json {
        Json::Array(items) => Ok(items),
        _ => Err(format!("{context} must be a JSON array")),
    }
}

fn get<'a>(entries: &'a [(String, Json)], key: &str) -> Result<&'a Json, String> {
    entries
        .iter()
        .find(|(stored, _)| stored == key)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("proof JSON missing {key:?}"))
}

fn string_field(entries: &[(String, Json)], key: &str) -> Result<String, String> {
    match get(entries, key)? {
        Json::String(value) => Ok(value.clone()),
        _ => Err(format!("{key:?} must be a string")),
    }
}

fn usize_field(entries: &[(String, Json)], key: &str) -> Result<usize, String> {
    match get(entries, key)? {
        Json::Number(value) => {
            usize::try_from(*value).map_err(|_| format!("{key:?} is too large for this platform"))
        }
        _ => Err(format!("{key:?} must be an unsigned integer")),
    }
}

fn proof_from_json(text: &str) -> Result<Proof, String> {
    let json = JsonParser::new(text).parse()?;
    let entries = object_entries(&json, "proof")?;
    let schema = string_field(entries, "schema")?;
    if schema != PROOF_SCHEMA {
        return Err(format!("unsupported proof schema {schema:?}"));
    }
    let hash = string_field(entries, "hash")?;
    if hash != HASH_ALGORITHM {
        return Err(format!("unsupported hash algorithm {hash:?}"));
    }
    let preimage = string_field(entries, "preimage")?;
    if preimage != PREIMAGE_VERSION {
        return Err(format!("unsupported preimage version {preimage:?}"));
    }
    let peaks = array_items(get(entries, "peaks")?, "peaks")?
        .iter()
        .map(|item| {
            let item = object_entries(item, "peak")?;
            Ok(MmrPeak {
                height: usize_field(item, "height")?,
                hash: parse_hex_32(&string_field(item, "hash")?)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let path = array_items(get(entries, "path")?, "path")?
        .iter()
        .map(|item| {
            let item = object_entries(item, "path step")?;
            let side = match string_field(item, "side")?.as_str() {
                "left" => ProofSide::Left,
                "right" => ProofSide::Right,
                other => return Err(format!("unsupported proof side {other:?}")),
            };
            Ok(ProofStep {
                parent_height: usize_field(item, "parent_height")?,
                side,
                hash: parse_hex_32(&string_field(item, "hash")?)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Proof {
        count: usize_field(entries, "count")?,
        leaf_index: usize_field(entries, "leaf_index")?,
        frame_id: parse_hex_32(&string_field(entries, "frame_id")?)?,
        root: parse_hex_32(&string_field(entries, "root")?)?,
        peak_index: usize_field(entries, "peak_index")?,
        peaks,
        path,
    })
}

/// Parse a raw 32-byte hex id, accepting an optional `blake3:` prefix.
pub fn parse_hex_32(input: &str) -> Result<Vec<u8>, String> {
    let trimmed = input.trim();
    let raw = trimmed.strip_prefix("blake3:").unwrap_or(trimmed);
    if raw.len() != 64 {
        return Err("expected a 32-byte hex value".to_string());
    }
    let mut out = Vec::with_capacity(32);
    for chunk in raw.as_bytes().as_chunks::<2>().0 {
        let hi = (chunk[0] as char)
            .to_digit(16)
            .ok_or_else(|| "hex value contains a non-hex character".to_string())?;
        let lo = (chunk[1] as char)
            .to_digit(16)
            .ok_or_else(|| "hex value contains a non-hex character".to_string())?;
        out.push(((hi << 4) | lo) as u8);
    }
    Ok(out)
}

fn as_i128(v: &Value) -> Option<i128> {
    if let Value::Integer(i) = v {
        Some(i128::from(*i))
    } else {
        None
    }
}

fn as_usize(v: &Value) -> Option<usize> {
    as_i128(v).and_then(|n| usize::try_from(n).ok())
}

fn as_text(v: &Value) -> Option<&str> {
    if let Value::Text(t) = v {
        Some(t)
    } else {
        None
    }
}

fn is_header_item(item: &Value) -> bool {
    unwrap_header(item).is_ok_and(|header| {
        map_get(header, "gts").and_then(as_text) == Some(MAGIC)
            && map_get(header, "v").and_then(as_i128) == Some(i128::from(VERSION))
    })
}

/// Create a detached proof for `target_frame_id` from a file that carries an
/// intact `index.mmr` covering that frame.
pub fn prove_file(data: &[u8], target_frame_id: &[u8]) -> Result<Proof, String> {
    if target_frame_id.len() != 32 {
        return Err("target frame id must be 32 bytes".to_string());
    }
    let (items, torn) = iter_items(data);
    if let Some(offset) = torn {
        return Err(format!(
            "input has a torn trailing CBOR item at byte {offset}"
        ));
    }
    if items.is_empty() {
        return Err("input is empty".to_string());
    }
    let mut item_index = 0usize;
    let mut candidate: Option<Proof> = None;
    while item_index < items.len() {
        let header = unwrap_header(&items[item_index].1)
            .map_err(|e| format!("item {item_index} is not a segment header: {e}"))?;
        if map_get(header, "gts").and_then(as_text) != Some(MAGIC)
            || map_get(header, "v").and_then(as_i128) != Some(i128::from(VERSION))
        {
            return Err(format!("item {item_index} is not a GTS v1 header"));
        }
        let computed_header = header_id(header);
        let stored_header = match map_get(header, "id") {
            Some(Value::Bytes(id)) if id.as_slice() == computed_header.as_slice() => id.clone(),
            Some(Value::Bytes(_)) => return Err(format!("header {item_index} id mismatch")),
            _ => return Err(format!("header {item_index} is missing id")),
        };
        let mut expected_prev = stored_header.clone();
        let mut frame_ids: Vec<Vec<u8>> = Vec::new();
        item_index += 1;
        while item_index < items.len() && !is_header_item(&items[item_index].1) {
            let abs_item = item_index;
            let Value::Map(frame) = &items[item_index].1 else {
                return Err(format!("item {abs_item} frame is not a map"));
            };
            let computed = content_id(frame);
            match map_get(frame, "id") {
                Some(Value::Bytes(stored)) if stored.as_slice() == computed.as_slice() => {}
                Some(Value::Bytes(_)) => return Err(format!("item {abs_item} id mismatch")),
                _ => return Err(format!("item {abs_item} is missing id")),
            }
            match map_get(frame, "prev") {
                Some(Value::Bytes(prev)) if prev.as_slice() == expected_prev.as_slice() => {}
                _ => return Err(format!("item {abs_item} prev mismatch")),
            }
            expected_prev.clone_from(&computed);
            frame_ids.push(computed);
            if map_get(frame, "t").and_then(as_text) == Some("index") {
                let Some(Value::Map(index_payload)) = map_get(frame, "d") else {
                    item_index += 1;
                    continue;
                };
                let Some(count) = map_get(index_payload, "count").and_then(as_usize) else {
                    item_index += 1;
                    continue;
                };
                let Some(Value::Bytes(head)) = map_get(index_payload, "head") else {
                    item_index += 1;
                    continue;
                };
                let Some(Value::Bytes(mmr_root)) = map_get(index_payload, "mmr") else {
                    item_index += 1;
                    continue;
                };
                let covered_limit = frame_ids.len().saturating_sub(1);
                if count > covered_limit {
                    return Err(format!(
                        "item {abs_item} index covers {count} frame(s), but only \
                         {covered_limit} precede the index"
                    ));
                }
                if count == 0 {
                    if head.as_slice() != stored_header.as_slice() {
                        return Err(format!("item {abs_item} empty index head mismatch"));
                    }
                } else if frame_ids[count - 1].as_slice() != head.as_slice() {
                    return Err(format!("item {abs_item} index head mismatch"));
                }
                let covered = &frame_ids[..count];
                let computed_root = root(covered);
                if computed_root.as_slice() != mmr_root.as_slice() {
                    return Err(format!("item {abs_item} index mmr mismatch"));
                }
                if let Some(target_index) = covered
                    .iter()
                    .position(|frame_id| frame_id.as_slice() == target_frame_id)
                {
                    candidate = prove(covered, target_index);
                }
            }
            item_index += 1;
        }
    }
    candidate.ok_or_else(|| format!("no valid index mmr covers frame {}", hex(target_frame_id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Vec<u8> {
        vec![n; 32]
    }

    #[test]
    fn proof_round_trip_and_tamper() {
        let frame_ids = vec![id(1), id(2), id(3), id(4), id(5)];
        let proof = prove(&frame_ids, 3).expect("proof exists");
        verify_proof(&proof).expect("proof verifies");
        let parsed = Proof::from_json(&proof.to_json()).expect("proof json parses");
        assert_eq!(parsed, proof);

        let mut bad = proof.clone();
        bad.root[0] ^= 1;
        assert!(verify_proof(&bad).is_err());

        let mut bad = proof;
        bad.frame_id[0] ^= 1;
        assert!(verify_proof(&bad).is_err());
    }

    #[test]
    fn roots_change_with_order() {
        let left = vec![id(1), id(2), id(3)];
        let right = vec![id(1), id(3), id(2)];
        assert_ne!(root(&left), root(&right));
    }
}

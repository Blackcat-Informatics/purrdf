// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Merkle-Mountain-Range commitments for `index.mmr` and detached proof JSON.
//!
//! Detached proof JSON is read by the workspace's one JSON reader,
//! [`purrdf_lex::json`], under its default depth cap.

use purrdf_hash::Domain;
use purrdf_hash::hex::Lower;
use purrdf_iri::json_escape::{JsonEscapes, push_body};
use purrdf_lex::cbor::Value;
use purrdf_lex::json::{self, Object, Value as Json};

use crate::reader::{as_i128, as_idx};
use crate::wire::{
    MAGIC, VERSION, blake3_256, canonical, content_id, header_id, iter_items, map_get,
    unwrap_header,
};

/// Stable detached proof schema tag emitted by [`Proof::to_json`].
pub const PROOF_SCHEMA: &str = "gts-mmr-proof-v1";
const HASH_ALGORITHM: &str = "blake3-256";
const PREIMAGE_VERSION: &str = "gts-mmr-v1";
/// The hash domain of a leaf preimage: the first element of its CBOR array.
const LEAF_DOMAIN: Domain = Domain::new(b"gts-mmr-leaf-v1");
/// The hash domain of an interior node preimage.
const PARENT_DOMAIN: Domain = Domain::new(b"gts-mmr-parent-v1");
/// The hash domain of the root preimage over the peaks.
const ROOT_DOMAIN: Domain = Domain::new(b"gts-mmr-root-v1");

/// One peak of the Merkle Mountain Range committed by `index.mmr`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MmrPeak {
    /// Peak tree height. Height zero is a leaf.
    pub height: usize,
    /// Peak hash using the `gts-mmr-*` preimage domains.
    pub hash: Vec<u8>,
}

/// Why an incremental MMR state could not be restored or extended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MmrStateError {
    /// The number of peaks differs from the count's binary decomposition.
    PeakCount {
        /// Number of committed frame ids.
        count: u64,
        /// Required number of peaks.
        expected: usize,
        /// Supplied number of peaks.
        actual: usize,
    },
    /// A peak has the wrong height or occurs in the wrong order.
    PeakHeight {
        /// Zero-based position in the ordered peak list.
        peak: usize,
        /// Required height at this position.
        expected: usize,
        /// Supplied height.
        actual: usize,
    },
    /// A peak hash is not a BLAKE3-256 digest.
    PeakHashLength {
        /// Zero-based position in the ordered peak list.
        peak: usize,
        /// Supplied hash length in bytes.
        actual: usize,
    },
    /// Appending would exceed the maximum `u64` frame count.
    CountOverflow,
}

impl std::fmt::Display for MmrStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PeakCount {
                count,
                expected,
                actual,
            } => {
                write!(f, "count {count} requires {expected} peaks, got {actual}")
            }
            Self::PeakHeight {
                peak,
                expected,
                actual,
            } => {
                write!(f, "peak {peak} requires height {expected}, got {actual}")
            }
            Self::PeakHashLength { peak, actual } => {
                write!(f, "peak {peak} hash must be 32 bytes, got {actual}")
            }
            Self::CountOverflow => f.write_str("MMR frame count would exceed u64::MAX"),
        }
    }
}

impl std::error::Error for MmrStateError {}

/// The portable, incremental commitment frontier of an ordered MMR.
///
/// Only the count and its descending-height peaks are retained. Appending hashes
/// one leaf and merges the rightmost equal-height peaks in `O(log n)` worst-case
/// hash operations, using the same preimages as [`root`]. Hashing the frame id
/// also reads its bytes. The `u64` count permits restoration
/// of a compact frontier even when its historical leaf list would exceed the
/// target's address width. [`Default`] constructs the empty frontier.
///
/// ```
/// use purrdf_gts::mmr::{MmrPeaks, MmrStateError, root};
///
/// let ids = vec![vec![1; 32], vec![2; 32]];
/// let mut frontier = MmrPeaks::default();
/// frontier.push(&ids[0])?;
/// let trusted_root = frontier.root();
/// let mut restored = MmrPeaks::from_parts(frontier.count(), frontier.peaks().to_vec())?;
/// assert_eq!(restored.root(), trusted_root);
/// restored.push(&ids[1])?;
/// assert_eq!(restored.count(), 2);
/// assert_eq!(restored.root(), root(&ids));
/// # Ok::<(), MmrStateError>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MmrPeaks {
    count: u64,
    peaks: Vec<MmrPeak>,
}

impl MmrPeaks {
    /// Number of frame ids committed by this frontier.
    pub const fn count(&self) -> u64 {
        self.count
    }

    /// Ordered peaks, from the largest/oldest subtree to the smallest/newest.
    pub fn peaks(&self) -> &[MmrPeak] {
        &self.peaks
    }

    /// Restore the count and ordered peaks of a persisted frontier.
    ///
    /// Heights must match the descending set bits of `count`, and each hash
    /// must contain 32 bytes. This checks the shape, not the historical hashes:
    /// compare [`Self::root`] with a trusted commitment to authenticate the state.
    pub fn from_parts(count: u64, peaks: Vec<MmrPeak>) -> Result<Self, MmrStateError> {
        let expected = expected_peak_heights(count);
        if peaks.len() != expected.len() {
            return Err(MmrStateError::PeakCount {
                count,
                expected: expected.len(),
                actual: peaks.len(),
            });
        }
        for (index, (peak, height)) in peaks.iter().zip(expected).enumerate() {
            if peak.height != height {
                return Err(MmrStateError::PeakHeight {
                    peak: index,
                    expected: height,
                    actual: peak.height,
                });
            }
            if peak.hash.len() != 32 {
                return Err(MmrStateError::PeakHashLength {
                    peak: index,
                    actual: peak.hash.len(),
                });
            }
        }
        Ok(Self { count, peaks })
    }

    /// Append a frame id under the existing MMR leaf and parent preimages.
    ///
    /// Like [`root`], this accepts any frame-id byte slice. Count overflow is
    /// refused before hashing or changing the frontier.
    pub fn push(&mut self, frame_id: &[u8]) -> Result<(), MmrStateError> {
        let count = self
            .count
            .checked_add(1)
            .ok_or(MmrStateError::CountOverflow)?;
        let mut carried = MmrPeak {
            height: 0,
            hash: leaf_hash(self.count, frame_id),
        };
        while self
            .peaks
            .last()
            .is_some_and(|peak| peak.height == carried.height)
        {
            let left = self.peaks.pop().expect("matching left peak exists");
            let height = carried.height + 1;
            carried = MmrPeak {
                height,
                hash: parent_hash(height, &left.hash, &carried.hash),
            };
        }
        self.peaks.push(carried);
        self.count = count;
        Ok(())
    }

    /// Commit to the count and ordered peaks using the existing root preimage.
    pub fn root(&self) -> Vec<u8> {
        root_hash(self.count, &self.peaks)
    }
}

/// Position of a proof sibling relative to the carried node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofSide {
    /// Sibling hash is the left child.
    Left,
    /// Sibling hash is the right child.
    Right,
}

/// One sibling-hash step on the path from a proven leaf to its peak.
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

fn uint(n: u64) -> Value {
    Value::from(n)
}

fn leaf_hash(index: u64, frame_id: &[u8]) -> Vec<u8> {
    blake3_256(&canonical(&Value::Array(vec![
        LEAF_DOMAIN.as_str().into(),
        uint(index),
        Value::Bytes(frame_id.to_vec()),
    ])))
    .to_vec()
}

fn parent_hash(parent_height: usize, left: &[u8], right: &[u8]) -> Vec<u8> {
    blake3_256(&canonical(&Value::Array(vec![
        PARENT_DOMAIN.as_str().into(),
        uint(parent_height as u64),
        Value::Bytes(left.to_vec()),
        Value::Bytes(right.to_vec()),
    ])))
    .to_vec()
}

fn root_hash(count: u64, peaks: &[MmrPeak]) -> Vec<u8> {
    let peak_values: Vec<Value> = peaks
        .iter()
        .map(|peak| {
            Value::Array(vec![
                uint(peak.height as u64),
                Value::Bytes(peak.hash.clone()),
            ])
        })
        .collect();
    blake3_256(&canonical(&Value::Array(vec![
        ROOT_DOMAIN.as_str().into(),
        uint(count),
        Value::Array(peak_values),
    ])))
    .to_vec()
}

fn build_nodes(frame_ids: &[Vec<u8>]) -> Vec<Node> {
    let mut peaks: Vec<Node> = Vec::new();
    for (index, frame_id) in frame_ids.iter().enumerate() {
        peaks.push(Node {
            height: 0,
            start: index,
            end: index + 1,
            hash: leaf_hash(index as u64, frame_id),
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

/// Peaks-only MMR fold: the same append/merge rule as [`build_nodes`] and the
/// same `parent_hash` inputs, but it keeps just `(height, hash)` per peak.
///
/// `root` never descends into the tree, so boxing every merged child (two
/// heap nodes per merge, `O(n)` boxes total) was pure waste on that path;
/// `prove` still needs the children and keeps using [`build_nodes`].
fn peak_fold(frame_ids: &[Vec<u8>]) -> Vec<MmrPeak> {
    let mut frontier = MmrPeaks::default();
    for frame_id in frame_ids {
        frontier
            .push(frame_id)
            .expect("a slice's frame count fits u64");
    }
    frontier.peaks
}

/// Compute the stable `index.mmr` root over ordered frame ids.
///
/// The root commits to both the frame count and the ordered peak list, so
/// adding a frame changes the root even when an earlier proof path is reused.
pub fn root(frame_ids: &[Vec<u8>]) -> Vec<u8> {
    root_hash(frame_ids.len() as u64, &peak_fold(frame_ids))
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
        root: root_hash(frame_ids.len() as u64, &peaks),
        peak_index,
        peaks,
        path,
    })
}

fn expected_peak_heights(count: u64) -> Vec<usize> {
    let mut remaining = count;
    let mut heights = Vec::new();
    while remaining > 0 {
        let height = (u64::BITS - 1 - remaining.leading_zeros()) as usize;
        heights.push(height);
        remaining -= 1u64 << height;
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
    let expected_heights = expected_peak_heights(proof.count as u64);
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
    let mut carried = leaf_hash(proof.leaf_index as u64, &proof.frame_id);
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
    let computed_root = root_hash(proof.count as u64, &proof.peaks);
    if computed_root != proof.root {
        return Err("proof peaks do not reconstruct the declared root".to_string());
    }
    Ok(())
}

/// A proof string's JSON body: the workspace's one JSON escape law,
/// [`purrdf_iri::json_escape`], in its [`JsonEscapes::Controls`] spelling (DEL
/// and the C1 controls escaped too, as this writer always has).
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    push_body(&mut out, text, JsonEscapes::Controls);
    out
}

/// The original per-`char` escaper, kept as the oracle for [`json_escape`].
#[cfg(test)]
fn json_escape_reference(text: &str) -> String {
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
        let _ = writeln!(out, "  \"frame_id\": \"{}\",", Lower(&self.frame_id));
        let _ = writeln!(out, "  \"root\": \"{}\",", Lower(&self.root));
        let _ = writeln!(out, "  \"peak_index\": {},", self.peak_index);
        out.push_str("  \"peaks\": [\n");
        for (index, peak) in self.peaks.iter().enumerate() {
            let _ = writeln!(
                out,
                "    {{\"height\": {}, \"hash\": \"{}\"}}{}",
                peak.height,
                Lower(&peak.hash),
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
                Lower(&step.hash),
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

fn object<'a>(json: &'a Json, context: &str) -> Result<&'a Object, String> {
    json.as_object()
        .ok_or_else(|| format!("{context} must be a JSON object"))
}

fn array_items<'a>(json: &'a Json, context: &str) -> Result<&'a [Json], String> {
    json.as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{context} must be a JSON array"))
}

fn get<'a>(entries: &'a Object, key: &str) -> Result<&'a Json, String> {
    entries
        .get(key)
        .ok_or_else(|| format!("proof JSON missing {key:?}"))
}

fn string_field<'a>(entries: &'a Object, key: &str) -> Result<&'a str, String> {
    get(entries, key)?
        .as_str()
        .ok_or_else(|| format!("{key:?} must be a string"))
}

fn usize_field(entries: &Object, key: &str) -> Result<usize, String> {
    let value = get(entries, key)?
        .as_u64()
        .ok_or_else(|| format!("{key:?} must be an unsigned integer"))?;
    usize::try_from(value).map_err(|_| format!("{key:?} is too large for this platform"))
}

fn proof_from_json(text: &str) -> Result<Proof, String> {
    let json = json::read(text).map_err(|error| format!("invalid proof JSON: {error}"))?;
    let entries = object(&json, "proof")?;
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
            let item = object(item, "peak")?;
            Ok(MmrPeak {
                height: usize_field(item, "height")?,
                hash: parse_hex_32(string_field(item, "hash")?)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let path = array_items(get(entries, "path")?, "path")?
        .iter()
        .map(|item| {
            let item = object(item, "path step")?;
            let side = match string_field(item, "side")? {
                "left" => ProofSide::Left,
                "right" => ProofSide::Right,
                other => return Err(format!("unsupported proof side {other:?}")),
            };
            Ok(ProofStep {
                parent_height: usize_field(item, "parent_height")?,
                side,
                hash: parse_hex_32(string_field(item, "hash")?)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Proof {
        count: usize_field(entries, "count")?,
        leaf_index: usize_field(entries, "leaf_index")?,
        frame_id: parse_hex_32(string_field(entries, "frame_id")?)?,
        root: parse_hex_32(string_field(entries, "root")?)?,
        peak_index: usize_field(entries, "peak_index")?,
        peaks,
        path,
    })
}

/// Parse a raw 32-byte hex id, accepting an optional `blake3:` prefix and
/// digits of either case ([`purrdf_hash::hex::decode_32`]).
pub fn parse_hex_32(input: &str) -> Result<Vec<u8>, String> {
    let trimmed = input.trim();
    let raw = trimmed.strip_prefix("blake3:").unwrap_or(trimmed);
    if raw.len() != 64 {
        return Err("expected a 32-byte hex value".to_string());
    }
    purrdf_hash::hex::decode_32(raw)
        .map(Vec::from)
        .ok_or_else(|| "hex value contains a non-hex character".to_string())
}

fn is_header_item(item: &Value) -> bool {
    unwrap_header(item).is_ok_and(|header| {
        map_get(header, "gts").and_then(Value::as_text) == Some(MAGIC)
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
        if map_get(header, "gts").and_then(Value::as_text) != Some(MAGIC)
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
            if map_get(frame, "t").and_then(Value::as_text) == Some("index") {
                let Some(Value::Map(index_payload)) = map_get(frame, "d") else {
                    item_index += 1;
                    continue;
                };
                let Some(count) = map_get(index_payload, "count").and_then(as_idx) else {
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
    candidate.ok_or_else(|| format!("no valid index mmr covers frame {}", Lower(target_frame_id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Vec<u8> {
        vec![n; 32]
    }

    /// A valid proof and its JSON text.
    fn proof_and_json() -> (Proof, String) {
        let proof = prove(&[id(1), id(2), id(3), id(4), id(5)], 3).expect("proof exists");
        let text = proof.to_json();
        (proof, text)
    }

    /// The proof JSON with its schema string spelled as `spelling`.
    fn with_schema(spelling: &str) -> String {
        let (_, text) = proof_and_json();
        let pinned = format!("\"{PROOF_SCHEMA}\"");
        assert!(text.contains(&pinned));
        text.replacen(&pinned, spelling, 1)
    }

    /// The proof JSON with an ignored member nested `depth` arrays deep.
    fn with_nested_member(depth: usize) -> String {
        let (_, text) = proof_and_json();
        let nested = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        text.replacen('{', &format!("{{\"extension\": {nested},"), 1)
    }

    #[test]
    fn a_proof_string_keeps_raw_non_ascii_scalars_whole() {
        let error = Proof::from_json(&with_schema("\"caf\u{e9} \u{1f431}\"")).unwrap_err();
        assert!(error.contains("caf\u{e9} \u{1f431}"), "{error}");
        let (proof, text) = proof_and_json();
        assert_eq!(Proof::from_json(&text), Ok(proof));
    }

    #[test]
    fn a_proof_string_decodes_escapes_and_surrogate_pairs() {
        let error = Proof::from_json(&with_schema(r#""\ud83d\ude00""#)).unwrap_err();
        assert!(error.contains('\u{1f600}'), "{error}");
        let (proof, _) = proof_and_json();
        assert_eq!(
            Proof::from_json(&with_schema(r#""\u0067ts-mmr-proof-v1""#)),
            Ok(proof)
        );
    }

    #[test]
    fn an_unpaired_surrogate_in_a_proof_is_refused() {
        for spelling in [
            r#""\ud83d""#,
            r#""\ud83dx""#,
            r#""\ude00""#,
            r#""\ud83d\u0041""#,
        ] {
            let error = Proof::from_json(&with_schema(spelling)).unwrap_err();
            assert!(
                error.starts_with("invalid proof JSON"),
                "{spelling}: {error}"
            );
        }
    }

    #[test]
    fn deep_proof_json_is_refused_and_shallow_nesting_parses() {
        let (proof, _) = proof_and_json();
        assert_eq!(Proof::from_json(&with_nested_member(32)), Ok(proof));
        let error = Proof::from_json(&with_nested_member(10_000)).unwrap_err();
        assert!(error.starts_with("invalid proof JSON"), "{error}");
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

    #[test]
    fn an_incremental_frontier_restores_and_continues() {
        let frame_ids = vec![id(1), id(2), id(3), id(4), id(5)];
        let mut frontier = MmrPeaks::default();
        assert_eq!(frontier.count(), 0);
        assert_eq!(frontier.peaks(), []);
        assert_eq!(frontier.root(), root(&[]));
        for frame_id in &frame_ids[..3] {
            frontier.push(frame_id).expect("append fits");
        }
        let mut restored = MmrPeaks::from_parts(frontier.count(), frontier.peaks().to_vec())
            .expect("produced peaks restore");
        assert_eq!(restored, frontier);
        for frame_id in &frame_ids[3..] {
            restored.push(frame_id).expect("continued append fits");
        }
        assert_eq!(restored.count(), 5);
        assert_eq!(restored.root(), root(&frame_ids));
        assert_eq!(restored.peaks(), peak_list(&build_nodes(&frame_ids)));
    }

    #[test]
    fn malformed_frontiers_have_typed_refusals() {
        assert_eq!(
            MmrPeaks::from_parts(1, Vec::new()),
            Err(MmrStateError::PeakCount {
                count: 1,
                expected: 1,
                actual: 0
            })
        );
        assert_eq!(
            MmrPeaks::from_parts(
                2,
                vec![MmrPeak {
                    height: 0,
                    hash: id(1)
                }]
            ),
            Err(MmrStateError::PeakHeight {
                peak: 0,
                expected: 1,
                actual: 0
            })
        );
        assert_eq!(
            MmrPeaks::from_parts(
                1,
                vec![MmrPeak {
                    height: 0,
                    hash: vec![1; 31]
                }]
            ),
            Err(MmrStateError::PeakHashLength {
                peak: 0,
                actual: 31
            })
        );
    }

    /// The tree-building computation `root` used before the peaks-only fold.
    fn root_via_tree(frame_ids: &[Vec<u8>]) -> Vec<u8> {
        let nodes = build_nodes(frame_ids);
        root_hash(frame_ids.len() as u64, &peak_list(&nodes))
    }

    #[test]
    fn peaks_only_root_matches_tree_root_for_every_small_count() {
        for count in 0..=40usize {
            let frame_ids: Vec<Vec<u8>> = (0..count)
                .map(|i| {
                    let mut fid = id(u8::try_from(i).expect("count fits in a byte"));
                    fid[31] = u8::try_from((i * 7) % 256).expect("reduced mod 256");
                    fid
                })
                .collect();
            assert_eq!(
                root(&frame_ids),
                root_via_tree(&frame_ids),
                "root mismatch at count {count}"
            );
            assert_eq!(
                peak_fold(&frame_ids),
                peak_list(&build_nodes(&frame_ids)),
                "peak list mismatch at count {count}"
            );
            if count > 0 {
                let proof = prove(&frame_ids, count - 1).expect("proof exists");
                assert_eq!(proof.root, root(&frame_ids));
            }
        }
    }

    fn varied_ids(count: usize) -> Vec<Vec<u8>> {
        (0..count)
            .map(|index| {
                let mut frame_id = id(index as u8);
                frame_id[..8].copy_from_slice(&(index as u64).to_le_bytes());
                frame_id
            })
            .collect()
    }

    #[test]
    fn every_incremental_prefix_and_restored_continuation_matches_the_tree() {
        let frame_ids = varied_ids(1101);
        let mut frontier = MmrPeaks::default();
        let mut restored = MmrPeaks::default();
        for count in 0..=1100 {
            assert_eq!(restored, frontier, "restored continuation at {count}");
            assert_eq!(frontier.count(), count as u64);
            assert_eq!(
                frontier.root(),
                root_via_tree(&frame_ids[..count]),
                "independent tree at {count}"
            );
            assert_eq!(
                frontier.root(),
                root(&frame_ids[..count]),
                "batch at {count}"
            );
            restored = MmrPeaks::from_parts(frontier.count(), frontier.peaks().to_vec())
                .expect("every produced frontier restores");
            assert_eq!(restored, frontier, "parts round trip at {count}");
            frontier.push(&frame_ids[count]).expect("append fits");
            restored
                .push(&frame_ids[count])
                .expect("restored append fits");
        }
        assert_eq!(restored, frontier);
        assert_eq!(restored.root(), root_via_tree(&frame_ids));
        assert_eq!(restored.root(), root(&frame_ids));
    }

    #[test]
    fn incremental_roots_preserve_the_batch_contract_for_arbitrary_id_bytes() {
        let frame_ids = [Vec::new(), vec![1], vec![2; 31], vec![3; 33], vec![4; 256]];
        let mut frontier = MmrPeaks::default();
        for (index, frame_id) in frame_ids.iter().enumerate() {
            frontier
                .push(frame_id)
                .expect("arbitrary id bytes are accepted");
            assert_eq!(frontier.root(), root_via_tree(&frame_ids[..=index]));
            assert_eq!(frontier.root(), root(&frame_ids[..=index]));
        }
    }

    #[test]
    fn restoration_refuses_count_order_height_and_every_bad_hash_length() {
        let frame_ids = varied_ids(13);
        let peaks = peak_list(&build_nodes(&frame_ids));
        let valid = MmrPeaks::from_parts(13, peaks.clone()).expect("valid neighbour");
        assert_eq!(valid.root(), root_via_tree(&frame_ids));
        for count in [0, 12, 14, 16] {
            assert!(
                MmrPeaks::from_parts(count, peaks.clone()).is_err(),
                "count {count}"
            );
        }
        let mut reordered = peaks.clone();
        reordered.swap(0, 2);
        assert_eq!(
            MmrPeaks::from_parts(13, reordered),
            Err(MmrStateError::PeakHeight {
                peak: 0,
                expected: 3,
                actual: 0
            })
        );
        let mut missing = peaks.clone();
        missing.pop();
        assert_eq!(
            MmrPeaks::from_parts(13, missing),
            Err(MmrStateError::PeakCount {
                count: 13,
                expected: 3,
                actual: 2
            })
        );
        let mut extra = peaks.clone();
        extra.push(peaks[2].clone());
        assert_eq!(
            MmrPeaks::from_parts(13, extra),
            Err(MmrStateError::PeakCount {
                count: 13,
                expected: 3,
                actual: 4
            })
        );
        for index in 0..peaks.len() {
            for height in [peaks[index].height + 1, 64, usize::MAX] {
                let mut malformed = peaks.clone();
                malformed[index].height = height;
                assert_eq!(
                    MmrPeaks::from_parts(13, malformed),
                    Err(MmrStateError::PeakHeight {
                        peak: index,
                        expected: peaks[index].height,
                        actual: height
                    })
                );
            }
            for length in [0, 1, 31, 33, 64] {
                let mut malformed = peaks.clone();
                malformed[index].hash.resize(length, 0);
                assert_eq!(
                    MmrPeaks::from_parts(13, malformed),
                    Err(MmrStateError::PeakHashLength {
                        peak: index,
                        actual: length
                    })
                );
            }
        }
        let mut duplicate = peaks.clone();
        duplicate[1].height = duplicate[0].height;
        assert!(MmrPeaks::from_parts(13, duplicate).is_err());
        let mut changed_hash = peaks;
        changed_hash[0].hash[0] ^= 1;
        let unauthenticated = MmrPeaks::from_parts(13, changed_hash)
            .expect("shape validation does not authenticate historical hashes");
        assert_ne!(unauthenticated.root(), valid.root());
    }

    /// Shape-valid synthetic hashes suffice to exercise counts without storing leaves.
    fn synthetic_peaks(count: u64) -> Vec<MmrPeak> {
        (0..64)
            .rev()
            .filter(|height| count & (1u64 << height) != 0)
            .map(|height| MmrPeak {
                height,
                hash: id(height as u8),
            })
            .collect()
    }

    #[test]
    fn restored_frontiers_carry_across_address_width_and_u64_boundaries() {
        for count in [
            0,
            1,
            2,
            (1u64 << 32) - 1,
            1u64 << 32,
            (1u64 << 32) + 1,
            (1u64 << 63) - 1,
            1u64 << 63,
            (1u64 << 63) + 1,
            u64::MAX - 1,
        ] {
            let peaks = synthetic_peaks(count);
            let mut frontier =
                MmrPeaks::from_parts(count, peaks.clone()).expect("wide state restores");
            assert_eq!(frontier.count(), count);
            assert_eq!(frontier.peaks(), peaks);
            assert_eq!(frontier.root().len(), 32);
            frontier.push(&id(42)).expect("wide continuation fits");
            assert_eq!(frontier.count(), count + 1);
            assert_eq!(
                frontier
                    .peaks()
                    .iter()
                    .map(|peak| peak.height)
                    .collect::<Vec<_>>(),
                synthetic_peaks(count + 1)
                    .iter()
                    .map(|peak| peak.height)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                MmrPeaks::from_parts(frontier.count(), frontier.peaks().to_vec()),
                Ok(frontier)
            );
        }
    }

    #[test]
    fn maximum_count_refuses_append_without_changing_any_state() {
        let mut frontier = MmrPeaks::from_parts(u64::MAX, synthetic_peaks(u64::MAX))
            .expect("all 64 peaks restore without a leaf list");
        let before = frontier.clone();
        let frame_id = id(42);
        for bytes in [&[][..], frame_id.as_slice()] {
            assert_eq!(frontier.push(bytes), Err(MmrStateError::CountOverflow));
            assert_eq!(frontier, before);
            assert_eq!(frontier.root(), before.root());
        }
    }

    #[test]
    fn wide_counts_commit_to_literal_canonical_cbor_preimages() {
        assert_eq!(
            MmrPeaks::default().root(),
            blake3_256(b"\x83\x6fgts-mmr-root-v1\x00\x80").to_vec()
        );
        let cases: &[(u64, usize, &[u8])] = &[
            (1u64 << 32, 32,
             b"\x83\x6fgts-mmr-root-v1\x1b\x00\x00\x00\x01\x00\x00\x00\x00\x81\x82\x18\x20\x58\x20"),
            (1u64 << 63, 63,
             b"\x83\x6fgts-mmr-root-v1\x1b\x80\x00\x00\x00\x00\x00\x00\x00\x81\x82\x18\x3f\x58\x20"),
        ];
        for &(count, height, prefix) in cases {
            let hash = id(7);
            let frontier = MmrPeaks::from_parts(
                count,
                vec![MmrPeak {
                    height,
                    hash: hash.clone(),
                }],
            )
            .expect("single wide peak restores");
            let mut preimage = prefix.to_vec();
            preimage.extend_from_slice(&hash);
            assert_eq!(
                frontier.root(),
                blake3_256(&preimage).to_vec(),
                "count {count}"
            );
        }
    }

    #[test]
    fn wide_leaf_indices_and_parent_heights_preserve_literal_preimages() {
        let cases: &[(u64, &[u8])] = &[
            (
                1u64 << 32,
                b"\x83\x6fgts-mmr-leaf-v1\x1b\x00\x00\x00\x01\x00\x00\x00\x00\x58\x20",
            ),
            (
                u64::MAX - 1,
                b"\x83\x6fgts-mmr-leaf-v1\x1b\xff\xff\xff\xff\xff\xff\xff\xfe\x58\x20",
            ),
        ];
        for &(count, prefix) in cases {
            let frame_id = id(42);
            let mut frontier =
                MmrPeaks::from_parts(count, synthetic_peaks(count)).expect("wide state");
            frontier.push(&frame_id).expect("one more frame fits");
            let mut preimage = prefix.to_vec();
            preimage.extend_from_slice(&frame_id);
            let appended = frontier.peaks().last().expect("new height-zero peak");
            assert_eq!(appended.height, 0);
            assert_eq!(
                appended.hash,
                blake3_256(&preimage).to_vec(),
                "index {count}"
            );
        }
        let left = id(3);
        let right = id(4);
        let mut preimage = b"\x84\x71gts-mmr-parent-v1\x18\x3f\x58\x20".to_vec();
        preimage.extend_from_slice(&left);
        preimage.extend_from_slice(b"\x58\x20");
        preimage.extend_from_slice(&right);
        assert_eq!(
            parent_hash(63, &left, &right),
            blake3_256(&preimage).to_vec()
        );
    }

    #[test]
    fn existing_proofs_verify_against_incremental_roots_at_merge_boundaries() {
        let frame_ids = varied_ids(1100);
        for count in [
            1, 2, 3, 7, 8, 9, 31, 32, 33, 255, 256, 257, 1023, 1024, 1100,
        ] {
            let mut frontier = MmrPeaks::default();
            for frame_id in &frame_ids[..count] {
                frontier.push(frame_id).expect("append fits");
            }
            for index in [0, count / 2, count - 1] {
                let mut proof = prove(&frame_ids[..count], index).expect("covered leaf");
                assert_eq!(proof.root, frontier.root());
                proof.root = frontier.root();
                verify_proof(&proof).expect("existing proof verifies against incremental root");
            }
            assert_eq!(prove(&frame_ids[..count], count), None);
        }
    }

    #[test]
    fn json_escape_matches_reference() {
        let cases: [&str; 14] = [
            "",
            "plain ascii text 0123456789 ~!@#$%^&*()_+-=[]{};':,./<>?",
            "\"",
            "\\",
            "\n",
            "\r",
            "\t",
            "\u{0}\u{1}\u{1f}\u{7f}",
            "\u{80}\u{85}\u{9f}",
            "caf\u{e9} \u{4e2d}\u{6587} \u{1f431}",
            "mixed \"quoted\" \\ back\\slash\n\ttab \u{e9}\u{1}end",
            "\u{7f}\u{2028}\u{feff}",
            "trailing quote\"",
            "\"leading quote",
        ];
        for case in cases {
            assert_eq!(json_escape(case), json_escape_reference(case), "{case:?}");
        }
    }
}

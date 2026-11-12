// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native file-backed qualification store: no resident dictionary or key index.

use purrdf_core::TermId;
use purrdf_gts::dictionary::ResidentTermStorage;
use purrdf_gts::dictionary::{DictionaryError, DictionaryLimits, TermKey, TermLease, TermStorage};
use purrdf_gts::model::{Term, TermKind, Triple3};
use purrdf_gts::segment_decode::{
    PendingResolution, ResolutionCounts, ResolutionStorage, ResolutionTerm,
};
use purrdf_lex::allocation::{Ceiling, Memory, OwnedAdmission, SharedOwned};
use purrdf_lex::cbor::{self, ReplayStorage, Value};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const WINDOW: usize = 8192;

#[derive(Default)]
pub(super) struct Resident {
    pub(super) reject_limits: bool,
    terms: [Option<(TermKey, TermLease)>; 4],
    bindings: [Option<(TermKey, Triple3)>; 4],
    live: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

struct ResidentOwner {
    account: OwnedAdmission<Ceiling>,
    live: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    bytes: usize,
}
impl Drop for ResidentOwner {
    fn drop(&mut self) {
        self.live
            .fetch_sub(self.bytes, std::sync::atomic::Ordering::SeqCst);
    }
}

impl TermStorage for Resident {
    fn limits(&mut self) -> Result<DictionaryLimits, DictionaryError> {
        if self.reject_limits {
            return Err(DictionaryError::Corrupt("unused resident policy"));
        }
        Ok(DictionaryLimits {
            encoded_frame_bytes: WINDOW,
            decoded_frame_bytes: WINDOW,
            metadata_entries: 100_000,
        })
    }
    fn put(&mut self, key: TermKey, term: &Term) -> Result<(), DictionaryError> {
        let mut owner = ResidentOwner {
            account: OwnedAdmission::new(Ceiling(WINDOW)),
            live: self.live.clone(),
            bytes: 0,
        };
        let mut memory = Memory::new(&mut owner.account);
        let mut text =
            |value: &Option<String>| value.as_deref().map(|s| memory.string(s)).transpose();
        let copied = Term {
            kind: term.kind,
            value: text(&term.value)?,
            datatype: term.datatype,
            lang: text(&term.lang)?,
            direction: text(&term.direction)?,
            reifier: term.reifier,
            triple: term.triple,
        };
        memory.add_bytes(SharedOwned::<Term>::allocation_layout::<ResidentOwner>().size())?;
        owner.bytes = TermLease::allocation_bytes::<ResidentOwner>(&copied)?;
        owner
            .live
            .fetch_add(owner.bytes, std::sync::atomic::Ordering::SeqCst);
        let lease = TermLease::try_from_admitted(copied, owner)?;
        let slot = self
            .terms
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(DictionaryError::Corrupt("qualification resident capacity"))?;
        *slot = Some((key, lease));
        Ok(())
    }
    fn get(&mut self, key: TermKey) -> Result<Option<TermLease>, DictionaryError> {
        Ok(self
            .terms
            .iter()
            .flatten()
            .find(|(stored, _)| *stored == key)
            .map(|(_, term)| term.clone()))
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, DictionaryError> {
        Ok(self
            .bindings
            .iter()
            .flatten()
            .find(|(stored, _)| *stored == key)
            .map(|(_, triple)| *triple))
    }
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), DictionaryError> {
        if self.first_binding(key)?.is_none() {
            let slot = self
                .bindings
                .iter_mut()
                .find(|entry| entry.is_none())
                .ok_or(DictionaryError::Corrupt("qualification binding capacity"))?;
            *slot = Some((key, triple));
        }
        Ok(())
    }
    fn checkpoint(&mut self, _: u64) -> Result<(), DictionaryError> {
        Ok(())
    }
    fn commit(&mut self) -> Result<(), DictionaryError> {
        Ok(())
    }
}

impl ResidentTermStorage for Resident {
    fn required_term_bytes(&self, _: TermKey, term: &Term) -> Result<usize, DictionaryError> {
        let strings = [
            term.value.as_ref(),
            term.lang.as_ref(),
            term.direction.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(String::len)
        .sum::<usize>();
        Ok(self.retained_bytes()
            + SharedOwned::<Term>::allocation_layout::<ResidentOwner>().size()
            + strings)
    }
    fn required_binding_bytes(&self, _: TermKey, _: Triple3) -> Result<usize, DictionaryError> {
        Ok(self.retained_bytes())
    }
    fn retained_bytes(&self) -> usize {
        size_of::<Self>() + self.live.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn next_key(&self, after: Option<TermKey>) -> Option<TermKey> {
        self.terms
            .iter()
            .flatten()
            .map(|(key, _)| *key)
            .filter(|key| after.is_none_or(|after| *key > after))
            .min()
    }
    fn next_binding(&self, after: Option<TermKey>) -> Option<(TermKey, Triple3)> {
        self.bindings
            .iter()
            .flatten()
            .copied()
            .filter(|(key, _)| after.is_none_or(|after| *key > after))
            .min_by_key(|(key, _)| *key)
    }
    fn release_entries(&mut self) -> Result<(), DictionaryError> {
        self.terms = std::array::from_fn(|_| None);
        self.bindings = [None; 4];
        Ok(())
    }
}

pub(super) struct Files {
    pub(super) metadata: purrdf_gts::dictionary::ReaderMetadata,
    pub(super) peak_metadata_entries: usize,
    root: PathBuf,
    pub(super) reads: usize,
    pub(super) writes: usize,
    pub(super) commits: usize,
    pub(super) discard_writes: Option<&'static str>,
    pub(super) cancel_after: Option<usize>,
    pub(super) limits: DictionaryLimits,
}

impl Files {
    pub(super) fn new(root: &Path) -> Self {
        std::fs::create_dir_all(root).expect("create caller-owned native store");
        Self {
            metadata: purrdf_gts::dictionary::ReaderMetadata::default(),
            peak_metadata_entries: 0,
            root: root.to_owned(),
            reads: 0,
            writes: 0,
            commits: 0,
            discard_writes: None,
            cancel_after: None,
            limits: DictionaryLimits {
                encoded_frame_bytes: WINDOW,
                decoded_frame_bytes: WINDOW,
                metadata_entries: 100_000,
            },
        }
    }
    fn path(&self, key: TermKey, suffix: &str) -> PathBuf {
        self.root
            .join(format!("{}-{}.{suffix}", key.segment, key.term))
    }
    fn read(&mut self, key: TermKey, suffix: &str) -> Result<Option<Value>, DictionaryError> {
        self.checkpoint(1)?;
        let file = match File::open(self.path(key, suffix)) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take((WINDOW + 33) as u64).read_to_end(&mut bytes)?;
        self.reads += 1;
        if bytes.len() < 32 || bytes.len() > WINDOW + 32 {
            return Err(DictionaryError::Corrupt("native record length"));
        }
        let (checksum, payload) = bytes.split_at(32);
        if checksum != purrdf_hash::blake3::hash(payload).as_bytes() {
            return Err(DictionaryError::Corrupt("native record checksum"));
        }
        cbor::decode(payload, cbor::Limits::DEFAULT)
            .map(Some)
            .map_err(|_| DictionaryError::Corrupt("native record encoding"))
    }
    fn write(&mut self, key: TermKey, suffix: &str, value: &Value) -> Result<(), DictionaryError> {
        self.checkpoint(1)?;
        if self.discard_writes == Some(suffix) {
            return Ok(());
        }
        let bytes = cbor::encode(value);
        if bytes.len() > WINDOW {
            return Err(DictionaryError::Corrupt("native record exceeds window"));
        }
        let mut file = File::create(self.path(key, suffix))?;
        file.write_all(purrdf_hash::blake3::hash(&bytes).as_bytes())?;
        // Exercise native interrupted/short-write recovery on every persisted payload.
        let mut interrupted = InterruptedWrite {
            file: &mut file,
            first: true,
        };
        interrupted.write_all(&bytes)?;
        file.sync_data()?;
        self.writes += 1;
        Ok(())
    }
    pub(super) fn replay(&self) -> [Box<dyn ReplayStorage>; 3] {
        std::array::from_fn(|index| {
            Box::new(Scratch(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(self.root.join(format!("replay-{index}")))
                    .expect("open native row replay"),
            )) as Box<dyn ReplayStorage>
        })
    }
    pub(super) fn corrupt(&self, key: TermKey) {
        std::fs::write(self.path(key, "term"), b"incomplete").expect("inject interrupted record");
    }
}

struct InterruptedWrite<'a> {
    file: &'a mut File,
    first: bool,
}
impl Write for InterruptedWrite<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.first {
            self.first = false;
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        self.file.write(&bytes[..bytes.len().min(17)])
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

struct Scratch(File);
impl Read for Scratch {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(bytes)
    }
}
impl Write for Scratch {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
impl std::io::Seek for Scratch {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.0, position)
    }
}
impl ReplayStorage for Scratch {
    fn finish(&mut self) -> std::io::Result<()> {
        self.0.sync_data()
    }
}

fn integer(value: &Value) -> Result<usize, DictionaryError> {
    value
        .as_integer()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(DictionaryError::Corrupt("native integer"))
}
fn optional(value: &Value) -> Result<Option<usize>, DictionaryError> {
    if matches!(value, Value::Null) {
        Ok(None)
    } else {
        integer(value).map(Some)
    }
}
fn triple(value: &Value) -> Result<Triple3, DictionaryError> {
    let Some([s, p, o]) = value.as_array().map(Vec::as_slice) else {
        return Err(DictionaryError::Corrupt("native triple"));
    };
    Ok((integer(s)?, integer(p)?, integer(o)?))
}
fn triple_value(value: Triple3) -> Value {
    Value::Array(vec![value.0.into(), value.1.into(), value.2.into()])
}

impl TermStorage for Files {
    fn reader_metadata(
        &mut self,
        usage: purrdf_gts::dictionary::ReaderMetadata,
    ) -> Result<(), DictionaryError> {
        self.metadata = usage;
        self.peak_metadata_entries = self.peak_metadata_entries.max(usage.entries);
        Ok(())
    }
    fn limits(&mut self) -> Result<DictionaryLimits, DictionaryError> {
        Ok(self.limits)
    }
    fn put(&mut self, key: TermKey, term: &Term) -> Result<(), DictionaryError> {
        let text = |value: &Option<String>| {
            value
                .as_ref()
                .map_or(Value::Null, |s| Value::Text(s.clone()))
        };
        self.write(
            key,
            "term",
            &Value::Array(vec![
                Value::from(term.kind as usize),
                text(&term.value),
                term.datatype.map_or(Value::Null, Value::from),
                text(&term.lang),
                text(&term.direction),
                term.reifier.map_or(Value::Null, Value::from),
                term.triple.map_or(Value::Null, triple_value),
            ]),
        )
    }
    fn get(&mut self, key: TermKey) -> Result<Option<TermLease>, DictionaryError> {
        let Some(value) = self.read(key, "term")? else {
            return Ok(None);
        };
        let Some([kind, value, datatype, lang, direction, reifier, quoted]) =
            value.as_array().map(Vec::as_slice)
        else {
            return Err(DictionaryError::Corrupt("native term record"));
        };
        let mut owner = OwnedAdmission::new(Ceiling(WINDOW));
        let mut memory = Memory::new(&mut owner);
        let mut text = |value: &Value| {
            if matches!(value, Value::Null) {
                return Ok(None);
            }
            let text = value
                .as_text()
                .ok_or(DictionaryError::Corrupt("native text"))?;
            memory.string(text).map(Some).map_err(DictionaryError::from)
        };
        let term = Term {
            kind: match integer(kind)? {
                0 => TermKind::Iri,
                1 => TermKind::Literal,
                2 => TermKind::Bnode,
                3 => TermKind::Triple,
                _ => return Err(DictionaryError::Corrupt("native term kind")),
            },
            value: text(value)?,
            datatype: optional(datatype)?,
            lang: text(lang)?,
            direction: text(direction)?,
            reifier: optional(reifier)?,
            triple: if matches!(quoted, Value::Null) {
                None
            } else {
                Some(triple(quoted)?)
            },
        };
        memory.add_bytes(
            SharedOwned::<Term>::allocation_layout::<OwnedAdmission<Ceiling>>().size(),
        )?;
        Ok(Some(TermLease::try_from_admitted(term, owner)?))
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, DictionaryError> {
        self.read(key, "binding")?.as_ref().map(triple).transpose()
    }
    fn bind_first(&mut self, key: TermKey, value: Triple3) -> Result<(), DictionaryError> {
        if self.first_binding(key)?.is_none() {
            self.write(key, "binding", &triple_value(value))?;
        }
        Ok(())
    }
    fn checkpoint(&mut self, _: u64) -> Result<(), DictionaryError> {
        if self
            .cancel_after
            .is_some_and(|limit| self.reads + self.writes >= limit)
        {
            return Err(purrdf_hash::error::io_error(
                std::io::ErrorKind::Interrupted,
                purrdf_testkit::error_capture::CallerRefusal { limit: u128::MAX },
            )
            .into());
        }
        Ok(())
    }
    fn commit(&mut self) -> Result<(), DictionaryError> {
        self.checkpoint(0)?;
        self.commits += 1;
        Ok(())
    }
}

pub(super) trait IdCodec: Copy {
    fn encode(self) -> Value;
    fn decode(value: &Value) -> Result<Self, DictionaryError>;
}
impl IdCodec for TermKey {
    fn encode(self) -> Value {
        Value::Array(vec![Value::from(self.segment), Value::from(self.term)])
    }
    fn decode(value: &Value) -> Result<Self, DictionaryError> {
        let Some([segment, term]) = value.as_array().map(Vec::as_slice) else {
            return Err(DictionaryError::Corrupt("native source key"));
        };
        Ok(Self {
            segment: integer(segment)? as u64,
            term: integer(term)?,
        })
    }
}
impl IdCodec for TermId {
    fn encode(self) -> Value {
        Value::from(self.index())
    }
    fn decode(value: &Value) -> Result<Self, DictionaryError> {
        let index = u32::try_from(integer(value)?)
            .map_err(|_| DictionaryError::Corrupt("native target id"))?;
        Self::from_index(index).map_err(|_| DictionaryError::Corrupt("native target id"))
    }
}

pub(super) struct Resolution<Id> {
    files: Files,
    counts: ResolutionCounts,
    marker: core::marker::PhantomData<Id>,
}
impl<Id> Resolution<Id> {
    pub(super) fn new(root: &Path) -> Self {
        let files = Files::new(root);
        let mut counts = ResolutionCounts::default();
        // Reopening scans directory metadata once without loading any term or
        // collecting keys. This host work is separate from lookup/read counts.
        for entry in std::fs::read_dir(root).expect("reopen resolution directory") {
            let path = entry.expect("native resolution directory entry").path();
            let extension = path.extension().and_then(|s| s.to_str());
            if !matches!(extension, Some("pending" | "resolved" | "binding")) {
                continue;
            }
            let segment = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.split_once('-'))
                .and_then(|(segment, _)| segment.parse::<u64>().ok())
                .expect("native resolution segment key");
            if files
                .path(
                    TermKey {
                        segment,
                        term: usize::MAX,
                    },
                    "complete",
                )
                .try_exists()
                .expect("read native completion metadata")
            {
                continue;
            }
            match extension {
                Some("pending") => counts.pending += 1,
                Some("resolved") => counts.resolved += 1,
                Some("binding") => counts.reifiers += 1,
                _ => unreachable!("record extensions selected above"),
            }
        }
        Self {
            files,
            counts,
            marker: core::marker::PhantomData,
        }
    }
}
impl<Id: IdCodec> ResolutionStorage<Id> for Resolution<Id> {
    type Error = DictionaryError;
    fn pending(&mut self, key: TermKey) -> Result<Option<PendingResolution>, DictionaryError> {
        let Some(value) = self.files.read(key, "pending")? else {
            return Ok(None);
        };
        let Some([present, parent, next, active]) = value.as_array().map(Vec::as_slice) else {
            return Err(DictionaryError::Corrupt("native continuation"));
        };
        Ok(Some(PendingResolution {
            term: if integer(present)? == 1 {
                Some(ResolutionTerm::Stored(
                    self.files.get(key)?.ok_or(DictionaryError::Missing(key))?,
                ))
            } else {
                None
            },
            parent: optional(parent)?,
            next: u8::try_from(integer(next)?)
                .map_err(|_| DictionaryError::Corrupt("native dependency slot"))?,
            active: integer(active)? == 1,
        }))
    }
    fn set_pending(
        &mut self,
        key: TermKey,
        value: PendingResolution,
    ) -> Result<(), DictionaryError> {
        if let Some(term) = &value.term {
            self.files.put(key, term)?;
        }
        if !self.files.path(key, "pending").try_exists()? {
            self.counts.pending += 1;
        }
        let count_key = TermKey {
            segment: key.segment,
            term: usize::MAX,
        };
        let count = self
            .files
            .read(count_key, "count")?
            .as_ref()
            .map(integer)
            .transpose()?
            .unwrap_or(0);
        if key.term > count {
            return Err(DictionaryError::Corrupt(
                "noncontiguous native introduction",
            ));
        }
        if key.term == count {
            self.files
                .write(count_key, "count", &Value::from(count + 1))?;
        } else if !self.files.path(key, "introduced").try_exists()? {
            return Err(DictionaryError::Corrupt("missing native introduced entry"));
        }
        self.files.write(key, "introduced", &Value::Null)?;
        self.files.write(
            key,
            "pending",
            &Value::Array(vec![
                usize::from(value.term.is_some()).into(),
                value.parent.map_or(Value::Null, Value::from),
                usize::from(value.next).into(),
                usize::from(value.active).into(),
            ]),
        )
    }
    fn remove_pending(&mut self, key: TermKey) -> Result<(), DictionaryError> {
        std::fs::remove_file(self.files.path(key, "pending"))?;
        self.counts.pending -= 1;
        Ok(())
    }
    fn next_term(
        &mut self,
        segment: u64,
        after: Option<usize>,
    ) -> Result<Option<usize>, DictionaryError> {
        let next = after.map_or(0, |id| id + 1);
        let count_key = TermKey {
            segment,
            term: usize::MAX,
        };
        let count = self
            .files
            .read(count_key, "count")?
            .as_ref()
            .map(integer)
            .transpose()?
            .unwrap_or(0);
        if next >= count {
            return Ok(None);
        }
        if !self
            .files
            .path(
                TermKey {
                    segment,
                    term: next,
                },
                "introduced",
            )
            .try_exists()?
        {
            return Err(DictionaryError::Corrupt("missing native introduced entry"));
        }
        Ok(Some(next))
    }
    fn resolved(&mut self, key: TermKey) -> Result<Option<Id>, DictionaryError> {
        self.files
            .read(key, "resolved")?
            .as_ref()
            .map(Id::decode)
            .transpose()
    }
    fn set_resolved(&mut self, key: TermKey, id: Id) -> Result<(), DictionaryError> {
        let introduced = !self.files.path(key, "resolved").try_exists()?;
        self.files.write(key, "resolved", &id.encode())?;
        self.counts.resolved += usize::from(introduced);
        Ok(())
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, DictionaryError> {
        self.files.first_binding(key)
    }
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), DictionaryError> {
        let introduced = self.files.first_binding(key)?.is_none();
        self.files.bind_first(key, triple)?;
        self.counts.reifiers += usize::from(introduced);
        Ok(())
    }
    fn complete_segment(&mut self, segment: u64) -> Result<(), DictionaryError> {
        self.files.write(
            TermKey {
                segment,
                term: usize::MAX,
            },
            "complete",
            &Value::Null,
        )?;
        self.counts = ResolutionCounts::default();
        self.files.commit()
    }
    fn counts(&self) -> ResolutionCounts {
        self.counts
    }
}

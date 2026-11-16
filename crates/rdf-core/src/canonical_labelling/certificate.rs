// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certificates `(s, E, C)`, their order, their encoding and REBUILD.

use std::cmp::Ordering;

use super::lines::{Interner, LineSet, TAG_FIXED, Tok};

/// One element of a fully fixed masked line, in walk order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntryToken {
    /// A structural tag byte.
    Tag(u8),
    /// The annotation count.
    Count(u64),
    /// The number of terms of a record.
    Length(u64),
    /// A constant leaf or a record's payload, by its encoding.
    Constant(Box<[u8]>),
    /// A blank leaf fixed at a path position.
    Fixed(u64),
}

/// An entry `(m, line)`: a fully fixed masked line and the largest path
/// position among its blank occurrences.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The largest path position among the line's blank occurrences.
    pub m: u64,
    /// The encoding of the fully fixed masked line.
    pub encoding: Box<[u8]>,
    /// The same line as typed tokens.
    pub tokens: Box<[EntryToken]>,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Entry {
    /// By `m`, then by the encoding, bytewise.
    fn cmp(&self, other: &Self) -> Ordering {
        (self.m, &self.encoding).cmp(&(other.m, &other.encoding))
    }
}

/// The certificate of a call. The derived order is the definition's: `s`,
/// then `E` as a list, then `C` as a list, a proper prefix first.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Certificate {
    /// The number of nodes the call (its chosen child) fixes.
    pub s: u64,
    /// The sorted entries of the lines it completes.
    pub entries: Vec<Entry>,
    /// The certificates of its nested calls, in their order.
    pub nested: Vec<Self>,
}

impl Certificate {
    /// The size `n(c)`: the number of nodes of the component.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.s + self.nested.iter().map(Self::size).sum::<u64>()
    }

    /// Append the certificate encoding to `out`.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.s.to_be_bytes());
        out.extend_from_slice(&(self.entries.len() as u64).to_be_bytes());
        for entry in &self.entries {
            out.extend_from_slice(&entry.m.to_be_bytes());
            out.extend_from_slice(&entry.encoding);
        }
        out.extend_from_slice(&(self.nested.len() as u64).to_be_bytes());
        for nested in &self.nested {
            nested.encode(out);
        }
    }

    /// The certificate encoding.
    #[must_use]
    pub fn encoding(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode(&mut out);
        out
    }
}

/// Why REBUILD stopped: a fixed leaf at or beyond `p + s` of its call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RebuildError {
    /// The path position found.
    pub position: u64,
    /// The first path position the call does not define.
    pub bound: u64,
}

/// REBUILD of a top component's certificate: the set of typed lines it
/// determines, the node of local position `i` being the blank node whose
/// source identifier is the eight bytes of `i`, big-endian.
///
/// # Errors
/// When an entry holds a fixed leaf its call does not define.
pub fn rebuild(certificate: &Certificate) -> Result<LineSet, RebuildError> {
    let mut interner = Interner::default();
    let mut kappa: Vec<usize> = Vec::new();
    rebuild_into(certificate, 0, 0, &mut kappa, &mut interner)?;
    Ok(interner.set)
}

fn rebuild_into(
    certificate: &Certificate,
    p: u64,
    o: u64,
    kappa: &mut Vec<usize>,
    interner: &mut Interner,
) -> Result<(), RebuildError> {
    let base = kappa.len();
    for i in 0..certificate.s {
        kappa.push(interner.node(&(o + i).to_be_bytes()));
    }
    let bound = p + certificate.s;
    for entry in &certificate.entries {
        let mut tokens = Vec::with_capacity(entry.tokens.len());
        for token in &*entry.tokens {
            tokens.push(match token {
                EntryToken::Tag(tag) => Tok::Tag(*tag),
                EntryToken::Count(count) => Tok::Count(*count),
                EntryToken::Length(n) => Tok::Len(*n),
                EntryToken::Constant(encoding) => Tok::Const(interner.constant(encoding)),
                EntryToken::Fixed(q) => {
                    if *q >= bound {
                        return Err(RebuildError {
                            position: *q,
                            bound,
                        });
                    }
                    let index = usize::try_from(*q).map_err(|_| RebuildError {
                        position: *q,
                        bound,
                    })?;
                    Tok::Blank(kappa[index])
                }
            });
        }
        interner.set.lines.push(tokens.into_boxed_slice());
    }
    let mut t = 0;
    for nested in &certificate.nested {
        rebuild_into(nested, bound, o + certificate.s + t, kappa, interner)?;
        t += nested.size();
    }
    kappa.truncate(base);
    Ok(())
}

/// The entry of line `line` of `set` with its blank occurrences, in walk
/// order, fixed at `positions`.
pub(crate) fn entry_of(set: &LineSet, line: usize, positions: &[u64]) -> Entry {
    let mut encoding = Vec::new();
    let mut tokens = Vec::with_capacity(set.lines[line].len());
    let mut m = 0;
    debug_assert_eq!(positions.len(), set.occurrences(line).count());
    let mut position = positions.iter().copied();
    for token in &*set.lines[line] {
        match *token {
            Tok::Tag(tag) => {
                encoding.push(tag);
                tokens.push(EntryToken::Tag(tag));
            }
            Tok::Count(count) => {
                encoding.extend_from_slice(&count.to_be_bytes());
                tokens.push(EntryToken::Count(count));
            }
            Tok::Len(n) => {
                encoding.extend_from_slice(&n.to_be_bytes());
                tokens.push(EntryToken::Length(n));
            }
            Tok::Const(c) => {
                let bytes = &set.consts[c];
                encoding.extend_from_slice(bytes);
                tokens.push(EntryToken::Constant(bytes.clone()));
            }
            Tok::Blank(_) => {
                // INVARIANT: `positions` holds one value per blank occurrence;
                // the caller builds it from the same walk.
                let q = position.next().unwrap_or(u64::MAX);
                m = m.max(q);
                encoding.push(TAG_FIXED);
                encoding.extend_from_slice(&q.to_be_bytes());
                tokens.push(EntryToken::Fixed(q));
            }
        }
    }
    Entry {
        m,
        encoding: encoding.into_boxed_slice(),
        tokens: tokens.into_boxed_slice(),
    }
}

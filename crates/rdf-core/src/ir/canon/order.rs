// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The order of canonical lines, decided over ranks of symbols instead of text.
//!
//! A canonical document is the set of its lines, sorted bytewise and
//! deduplicated. Holding one `String` per line is what this module replaces: a
//! line is kept as the sequence of its *symbols*, and the symbols are ranked once.
//!
//! A symbol is a leaf's text followed by a space, or one of the fixed
//! `<<( `, `)>> ` and `.\n`. A symbol is *aligned* when it is exactly one token
//! of the tokenizer below, which reads a quoted literal up to its closing quote
//! and then, like every other symbol, up to the first space or newline. Tokens of
//! that tokenizer are prefix-free by construction, because each stop depends only
//! on the bytes already read. Hence the bytewise order of two lines made only of
//! aligned symbols is the lexicographic order of their symbol sequences: at the
//! first differing symbol neither is a prefix of the other, so the first
//! differing byte lies inside both and decides, and if one sequence is a prefix of
//! the other the shorter line is a byte prefix of the longer one. Equal lines
//! have equal sequences. Alignment is decided per distinct leaf from its rendered
//! bytes, never from its kind, so equal text is never split across the two
//! classes.
//!
//! The only leaves that are not aligned hold a space or a newline in a language
//! tag, which no frozen dataset admits and an out-of-tree view may present.
//! Lines containing such a symbol are *irregular*: they are ordered by their
//! bytes and merged into the regular order by byte comparison, so every input
//! yields the bytes a plain sort of the rendered lines would.
//!
//! Ranking sorts by a head key first: the first [`HEAD`] bytes of a symbol's
//! spelling, padded with zero bytes, read when ranking begins. The key is monotone in the bytewise order
//! of the full spelling: if `a` is below `b` then `key(a) <= key(b)`, because a
//! first difference within the head is kept by both keys, and otherwise the keys
//! agree on the bytes of `a` while each zero of its padding is at most the byte
//! of `b` in the same place. So sorting by key and then ordering each run of
//! equal keys by the full spelling gives the full order, and two different keys
//! are never two equal spellings.

use std::cmp::Ordering;

use purrdf_lex::allocation::{Admission, Memory, StorageError, bounded_growth_capacity};
use purrdf_lex::walk::{try_dedup_by, try_sort_unstable_by};

use crate::hash::table::HashTable;

/// `<<( `, opening a triple term.
pub(super) const OPEN: u32 = 0;
/// `)>> `, closing a triple term.
pub(super) const CLOSE: u32 = 1;
/// `.\n`, ending every line.
pub(super) const TERM: u32 = 2;
/// The overlay's reifier sentinel IRI, with its trailing space.
pub(super) const SENTINEL_REIFIES: u32 = 3;
/// `rdf:reifies`, spelled as text by the flat presentation.
pub(super) const REIFIES: u32 = 4;
/// The overlay's annotation-graph sentinel IRI.
pub(super) const SENTINEL_ANNOTATION: u32 = 5;
/// The first symbol number that names a leaf.
pub(super) const FIRST_LEAF: u32 = 6;

/// How many leading bytes of a spelling its head key holds.
pub(super) const HEAD: usize = 32;

/// A symbol's head key and the symbol.
pub(super) type Head = ([u8; HEAD], u32);

/// The head key of a whole spelling: its first [`HEAD`] bytes, padded with
/// zero bytes.
pub(super) fn head_key(spelling: &[u8]) -> [u8; HEAD] {
    let mut key = [0; HEAD];
    let len = spelling.len().min(HEAD);
    key[..len].copy_from_slice(&spelling[..len]);
    key
}

/// The head key of a clean IRI's spelling, `<`, the IRI, `>` and a space,
/// without writing it.
pub(super) fn bracketed_head_key(iri: &[u8]) -> [u8; HEAD] {
    let mut key = [0; HEAD];
    key[0] = b'<';
    let len = iri.len().min(HEAD - 1);
    key[1..=len].copy_from_slice(&iri[..len]);
    for (at, byte) in [(iri.len() + 1, b'>'), (iri.len() + 2, b' ')] {
        if let Some(slot) = key.get_mut(at) {
            *slot = byte;
        }
    }
    key
}

/// Every symbol's head key, read by `key_of` into an array admitted at its
/// exact length, so no growth copy is ever held beside it.
///
/// # Errors
/// The first failure of `key_of` or of admitting the array.
pub(super) fn head_keys<
    Id: Copy + Eq + std::hash::Hash,
    E: From<StorageError>,
    S: Admission + ?Sized,
>(
    symbols: &Symbols<Id>,
    memory: &mut Memory<'_, S>,
    mut key_of: impl FnMut(&Symbols<Id>, &mut Memory<'_, S>, u32) -> Result<[u8; HEAD], E>,
) -> Result<Vec<Head>, E> {
    let mut heads = Vec::new();
    memory.reserve(&mut heads, symbols.count())?;
    for symbol in 0..symbols.count() {
        let symbol = u32::try_from(symbol).map_err(|_| StorageError::SizeOverflow)?;
        heads.push((key_of(symbols, memory, symbol)?, symbol));
    }
    Ok(heads)
}

/// How a symbol is spelled again.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Iri,
    Blank,
    Literal,
    /// Punctuation and the sentinel IRIs, which have no view term.
    Fixed,
}

const LEN_BITS: u32 = 27;
/// A stored length of this many bytes or more is kept in the side list.
pub(super) const LONG: u32 = (1 << LEN_BITS) - 1;

/// One symbol's length, kind and flags in 32 bits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Meta(u32);

impl Meta {
    pub(super) fn new(len: usize, kind: Kind, clean: bool, arena: bool, aligned: bool) -> Self {
        let field = u32::try_from(len).map_or(LONG, |len| len.min(LONG));
        let kind = match kind {
            Kind::Iri => 0,
            Kind::Blank => 1,
            Kind::Literal => 2,
            Kind::Fixed => 3,
        };
        Self(
            field
                | (kind << LEN_BITS)
                | (u32::from(clean) << 29)
                | (u32::from(arena) << 30)
                | (u32::from(aligned) << 31),
        )
    }

    pub(super) const fn bits(self) -> u32 {
        self.0
    }

    pub(super) const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// The stored length, or [`LONG`] when the exact length is in the side list.
    pub(super) const fn len_field(self) -> u32 {
        self.0 & LONG
    }

    pub(super) const fn kind(self) -> Kind {
        match (self.0 >> LEN_BITS) & 3 {
            0 => Kind::Iri,
            1 => Kind::Blank,
            2 => Kind::Literal,
            _ => Kind::Fixed,
        }
    }

    /// An IRI whose text is exactly `<` + the IRI + `>`: nothing was escaped.
    pub(super) const fn clean(self) -> bool {
        self.0 & (1 << 29) != 0
    }

    /// The text lives in the arena, because a composite literal was rewritten.
    pub(super) const fn arena(self) -> bool {
        self.0 & (1 << 30) != 0
    }

    pub(super) const fn aligned(self) -> bool {
        self.0 & (1 << 31) != 0
    }
}

/// The symbol number of the `leaves`th distinct leaf.
///
/// # Errors
/// More than 2^32 - 1 distinct symbols cannot be numbered.
pub(super) fn symbol_number(leaves: usize) -> Result<u32, StorageError> {
    usize::try_from(FIRST_LEAF)
        .ok()
        .and_then(|first| first.checked_add(leaves))
        .and_then(|number| u32::try_from(number).ok())
        .ok_or(StorageError::SizeOverflow)
}

/// Where the tokenizer stands after reading some bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Scan {
    /// It stopped, and the token is this many bytes long.
    Stopped(usize),
    /// It has not stopped. `quoted` is whether it is inside a quoted literal,
    /// `escaped` whether the last byte was a backslash that takes the next one.
    Open { quoted: bool, escaped: bool },
}

/// Run the tokenizer over `bytes` in one pass.
fn scan(bytes: &[u8]) -> Scan {
    let Some((&first, rest)) = bytes.split_first() else {
        return Scan::Open {
            quoted: false,
            escaped: false,
        };
    };
    let mut quoted = first == b'"';
    if first == b'\n' || (!quoted && first == b' ') {
        return Scan::Stopped(1);
    }
    let mut escaped = false;
    for (at, &byte) in rest.iter().enumerate() {
        if escaped {
            escaped = false;
        } else if quoted {
            match byte {
                b'\\' => escaped = true,
                b'"' => quoted = false,
                b'\n' => return Scan::Stopped(at + 2),
                _ => {}
            }
        } else if byte == b' ' || byte == b'\n' {
            return Scan::Stopped(at + 2);
        }
    }
    Scan::Open { quoted, escaped }
}

/// Whether `bytes` are exactly one token.
pub(super) fn is_one_token(bytes: &[u8]) -> bool {
    scan(bytes) == Scan::Stopped(bytes.len())
}

/// Whether a leaf's `text` followed by its space is exactly one token, without
/// writing the space: the tokenizer must not stop inside `text` and must stand
/// outside a quoted literal at its end, where the space then stops it. An empty
/// `text` leaves a lone space, which is one token.
fn leaf_is_one_token(text: &[u8]) -> bool {
    scan(text)
        == Scan::Open {
            quoted: false,
            escaped: false,
        }
}

/// Whether a leaf symbol is aligned. A clean IRI is aligned by construction:
/// its text is `<`, the IRI and `>` with nothing escaped, and the IRI escape set
/// holds every byte up to and including the space, so the tokenizer, which
/// starts outside a quote at `<`, first stops at the trailing space.
fn leaf_aligned(kind: Kind, clean: bool, text: &str) -> bool {
    (kind == Kind::Iri && clean) || leaf_is_one_token(text.as_bytes())
}

/// The tokenizer as it was first written, over an iterator that is counted
/// and then read again; kept as the oracle for the one-pass [`scan`].
#[cfg(test)]
fn reference_token_len(mut bytes: impl Iterator<Item = u8>) -> Option<usize> {
    let first = bytes.next()?;
    let mut quoted = first == b'"';
    if first == b'\n' || (!quoted && first == b' ') {
        return Some(1);
    }
    let mut read = 1;
    while let Some(byte) = bytes.next() {
        read += 1;
        if quoted {
            match byte {
                b'\\' => {
                    bytes.next()?;
                    read += 1;
                }
                b'"' => quoted = false,
                b'\n' => return Some(read),
                _ => {}
            }
        } else if byte == b' ' || byte == b'\n' {
            return Some(read);
        }
    }
    None
}

/// Whether `bytes` are exactly one token, by [`reference_token_len`].
#[cfg(test)]
pub(super) fn reference_is_one_token(bytes: &[u8]) -> bool {
    reference_token_len(bytes.iter().copied()) == Some(bytes.len())
}

/// The bytewise order of `x` followed by `>` and `y` followed by `>`, without
/// writing either bracket: the common length decides when it differs, and
/// otherwise the shorter side's `>` meets the other side's next byte. Equal
/// bytes there leave the shorter side's text a prefix of the other's, which
/// orders it first.
pub(super) fn cmp_bracketed(x: &[u8], y: &[u8]) -> Ordering {
    let common = x.len().min(y.len());
    x[..common]
        .cmp(&y[..common])
        .then_with(|| match x.len().cmp(&y.len()) {
            Ordering::Equal => Ordering::Equal,
            Ordering::Less => b'>'.cmp(&y[common]).then(Ordering::Less),
            Ordering::Greater => x[common].cmp(&b'>').then(Ordering::Greater),
        })
}

/// The distinct symbols of a run and the data needed to spell each again.
pub(super) struct Symbols<Id> {
    /// The view term of each leaf symbol, from [`FIRST_LEAF`].
    reps: Vec<Id>,
    /// Packed [`Meta`] per symbol; replaced by the symbol's rank while ranking.
    meta: Vec<u32>,
    /// Exact lengths of the symbols whose length field is [`LONG`].
    long_lens: Vec<(u32, u64)>,
    /// Rendered text of the symbols whose literal was rewritten.
    arena: String,
    /// `(symbol, end offset in the arena)`, in symbol order.
    arena_index: Vec<(u32, u64)>,
    table: HashTable<u32>,
    max_len: usize,
}

/// A borrow of what is needed to spell a symbol again.
pub(super) struct Spelling<'a, Id> {
    reps: &'a [Id],
    long_lens: &'a [(u32, u64)],
    arena: &'a str,
    arena_index: &'a [(u32, u64)],
}

impl<Id: Copy> Spelling<'_, Id> {
    /// The view term of a leaf symbol.
    pub(super) fn rep(&self, symbol: u32) -> Id {
        self.reps[(symbol - FIRST_LEAF) as usize]
    }

    /// The length of the symbol's text, without the trailing space of a leaf.
    pub(super) fn len(&self, symbol: u32, meta: Meta) -> usize {
        if meta.len_field() < LONG {
            return meta.len_field() as usize;
        }
        let at = self
            .long_lens
            .binary_search_by_key(&symbol, |entry| entry.0)
            .expect("a symbol with the long length marker has a recorded length");
        usize::try_from(self.long_lens[at].1).expect("a recorded length was a usize")
    }

    /// The text of an arena symbol.
    pub(super) fn arena_text(&self, symbol: u32) -> &str {
        let at = self
            .arena_index
            .binary_search_by_key(&symbol, |entry| entry.0)
            .expect("an arena symbol has recorded text");
        let start = if at == 0 {
            0
        } else {
            usize::try_from(self.arena_index[at - 1].1).expect("an arena offset was a usize")
        };
        let end = usize::try_from(self.arena_index[at].1).expect("an arena offset was a usize");
        &self.arena[start..end]
    }
}

/// The numbered symbols of a run and the lines written in their numbers.
pub(super) type Discovered<Id> = (Symbols<Id>, Lines);

/// The class of a symbol's first byte, in byte order: `"` (a literal), `)`
/// (`CLOSE`), `.` (`TERM`), `<` (an IRI, `OPEN` or a sentinel) and `_` (a
/// blank node). Two symbols of different classes differ at their first byte,
/// so their order is the order of their classes and neither is read.
///
/// `meta` is the symbol's packed meta, which is only valid before it is ranked.
pub(super) const fn first_byte_class(symbol: u32, meta: Meta) -> u8 {
    match meta.kind() {
        Kind::Literal => 0,
        Kind::Fixed if symbol == CLOSE => 1,
        Kind::Fixed if symbol == TERM => 2,
        Kind::Fixed | Kind::Iri => 3,
        Kind::Blank => 4,
    }
}

/// Grow a vector of a dataset's size by a quarter, admitting the layout first.
pub(super) fn push_bounded<T, S: Admission + ?Sized>(
    memory: &mut Memory<'_, S>,
    values: &mut Vec<T>,
    value: T,
) -> Result<(), StorageError> {
    if values.len() == values.capacity() {
        let required = values
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        let capacity = bounded_growth_capacity::<T>(values.capacity(), required)?;
        memory.reserve(values, capacity)?;
    }
    values.push(value);
    Ok(())
}

impl<Id: Copy + Eq + std::hash::Hash> Symbols<Id> {
    pub(super) fn new() -> Self {
        Self {
            reps: Vec::new(),
            meta: Vec::new(),
            long_lens: Vec::new(),
            arena: String::new(),
            arena_index: Vec::new(),
            table: HashTable::new(),
            max_len: 0,
        }
    }

    /// The largest symbol text seen, including a leaf's trailing space.
    pub(super) const fn max_len(&self) -> usize {
        self.max_len
    }

    /// How many symbols exist, fixed ones included.
    pub(super) fn count(&self) -> usize {
        self.meta.len()
    }

    pub(super) fn meta_of(&self, symbol: u32) -> Meta {
        Meta(self.meta[symbol as usize])
    }

    /// The rank of `symbol`; valid only after [`rank`].
    pub(super) fn rank_of(&self, symbol: u32) -> u32 {
        self.meta[symbol as usize]
    }

    pub(super) fn spelling(&self) -> Spelling<'_, Id> {
        Spelling {
            reps: &self.reps,
            long_lens: &self.long_lens,
            arena: &self.arena,
            arena_index: &self.arena_index,
        }
    }

    /// The symbol already standing for view term `id`.
    pub(super) fn find(&self, id: Id) -> Option<u32> {
        self.table
            .find(crate::hash::hash_of(&id), |&symbol| {
                self.reps[(symbol - FIRST_LEAF) as usize] == id
            })
            .copied()
    }

    /// Number the next fixed symbol; fixed symbols come first, in order.
    pub(super) fn push_fixed<S: Admission + ?Sized>(
        &mut self,
        len: usize,
        clean: bool,
        aligned: bool,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.max_len = self.max_len.max(len);
        let meta = Meta::new(len, Kind::Fixed, clean, false, aligned);
        push_bounded(memory, &mut self.meta, meta.bits())
    }

    /// Number a newly seen leaf and record how to spell it again. `text` is the
    /// leaf's bytes without the trailing space; `rewritten` text is kept.
    pub(super) fn push_leaf<S: Admission + ?Sized>(
        &mut self,
        id: Id,
        kind: Kind,
        clean: bool,
        rewritten: bool,
        text: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<(u32, Meta), StorageError> {
        let symbol = symbol_number(self.reps.len())?;
        let aligned = leaf_aligned(kind, clean, text);
        let meta = Meta::new(text.len(), kind, clean, rewritten, aligned);
        self.max_len = self.max_len.max(text.len() + 1);
        if meta.len_field() == LONG {
            let len = u64::try_from(text.len()).map_err(|_| StorageError::SizeOverflow)?;
            push_bounded(memory, &mut self.long_lens, (symbol, len))?;
        }
        if rewritten {
            memory.push_str(&mut self.arena, text)?;
            let end = u64::try_from(self.arena.len()).map_err(|_| StorageError::SizeOverflow)?;
            push_bounded(memory, &mut self.arena_index, (symbol, end))?;
        }
        push_bounded(memory, &mut self.reps, id)?;
        push_bounded(memory, &mut self.meta, meta.bits())?;

        let required = self
            .table
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        let reps = &self.reps;
        let hash = |&symbol: &u32| crate::hash::hash_of(&reps[(symbol - FIRST_LEAF) as usize]);
        crate::hash::reserve_table_with_memory(&mut self.table, required, hash, memory)?;
        self.table
            .insert_unique(crate::hash::hash_of(&id), symbol, hash);
        Ok((symbol, meta))
    }

    /// Free the lookup table once discovery is over.
    pub(super) fn release_table<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let table = std::mem::replace(&mut self.table, HashTable::new());
        let bytes =
            crate::hash::hash_table_retained_bound(&table).ok_or(StorageError::SizeOverflow)?;
        drop(table);
        memory.release_bytes(bytes)
    }

    /// Free the per-symbol ranks after every line sequence is remapped.
    pub(super) fn release_meta<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(std::mem::take(&mut self.meta))
    }

    /// Free everything that remains.
    pub(super) fn release<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.release_table(memory)?;
        self.release_meta(memory)?;
        memory.release_vec(self.reps)?;
        memory.release_vec(self.long_lens)?;
        memory.release_vec(self.arena_index)?;

        memory.release_string(self.arena)
    }
}

/// The distinct ranks, each with the symbol that stands for it and that symbol's
/// packed [`Meta`], read before ranks replaced it.
pub(super) struct Ranked {
    pub(super) symbols: Vec<u32>,
    pub(super) meta: Vec<u32>,
}

impl Ranked {
    pub(super) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.symbols)?;
        memory.release_vec(self.meta)
    }
}

impl Ranked {
    /// The rank table a line is spelled from, with the rank of the terminator.
    pub(super) fn table(&self, term: u32) -> RankTable<'_> {
        RankTable {
            symbols: &self.symbols,
            meta: &self.meta,
            term,
        }
    }
}

/// The symbol and packed meta standing for each rank, and the rank of `.\n`,
/// which ends every line's rank sequence.
#[derive(Clone, Copy)]
pub(super) struct RankTable<'a> {
    pub(super) symbols: &'a [u32],
    pub(super) meta: &'a [u32],
    pub(super) term: u32,
}

/// What spelling the ordered document again needs, kept after a run: the symbol
/// table, the rank table and every line as its rank sequence, back to back in
/// document order with irregular lines already placed. Nothing here is text of
/// the document except the rewritten literals the symbol table already kept.
pub(super) struct Retained<Id> {
    reps: Vec<Id>,
    long_lens: Vec<(u32, u64)>,
    arena: String,
    arena_index: Vec<(u32, u64)>,
    rank_symbols: Vec<u32>,
    rank_meta: Vec<u32>,
    ranks: Vec<u32>,
    term: u32,
    total: usize,
    longest: usize,
    lines: usize,
}

impl<Id: Copy> Retained<Id> {
    /// No lines and no symbols: the canonical form of an empty selection.
    pub(super) const fn empty() -> Self {
        Self {
            reps: Vec::new(),
            long_lens: Vec::new(),
            arena: String::new(),
            arena_index: Vec::new(),
            rank_symbols: Vec::new(),
            rank_meta: Vec::new(),
            ranks: Vec::new(),
            term: 0,
            total: 0,
            longest: 0,
            lines: 0,
        }
    }

    /// Copy what the ordered document needs into `memory`'s account. `lines` is
    /// ordered and measured; `total` and `longest` are its measured byte lengths.
    ///
    /// # Errors
    /// The first admission or allocation refusal; what was copied is dropped.
    pub(super) fn capture<S: Admission + ?Sized>(
        symbols: &Symbols<Id>,
        ranked: &Ranked,
        ordered: &Lines,
        table: RankTable<'_>,
        measured: (usize, usize),
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let term = table.term;
        let mut words = 0usize;
        let mut lines = 0usize;
        for start in ordered.merged() {
            let mut at = start;
            while ordered.seq[at] != term {
                at += 1;
            }
            words = words
                .checked_add(at - start + 1)
                .ok_or(StorageError::SizeOverflow)?;
            lines += 1;
        }
        let mut retained = Self::empty();
        retained.term = term;
        retained.total = measured.0;
        retained.longest = measured.1;
        retained.lines = lines;
        memory.reserve(&mut retained.ranks, words)?;
        for start in ordered.merged() {
            let mut at = start;
            while ordered.seq[at] != term {
                at += 1;
            }
            memory.extend(&mut retained.ranks, ordered.seq[start..=at].iter().copied())?;
        }
        memory.reserve(&mut retained.reps, symbols.reps.len())?;
        memory.extend(&mut retained.reps, symbols.reps.iter().copied())?;
        memory.reserve(&mut retained.long_lens, symbols.long_lens.len())?;
        memory.extend(&mut retained.long_lens, symbols.long_lens.iter().copied())?;
        memory.reserve(&mut retained.arena_index, symbols.arena_index.len())?;
        memory.extend(
            &mut retained.arena_index,
            symbols.arena_index.iter().copied(),
        )?;
        memory.reserve_string(&mut retained.arena, symbols.arena.len())?;
        memory.push_str(&mut retained.arena, &symbols.arena)?;
        memory.reserve(&mut retained.rank_symbols, ranked.symbols.len())?;
        memory.extend(&mut retained.rank_symbols, ranked.symbols.iter().copied())?;
        memory.reserve(&mut retained.rank_meta, ranked.meta.len())?;
        memory.extend(&mut retained.rank_meta, ranked.meta.iter().copied())?;
        Ok(retained)
    }

    /// The symbol data a line is spelled from.
    pub(super) fn spelling(&self) -> Spelling<'_, Id> {
        Spelling {
            reps: &self.reps,
            long_lens: &self.long_lens,
            arena: &self.arena,
            arena_index: &self.arena_index,
        }
    }

    pub(super) fn table(&self) -> RankTable<'_> {
        RankTable {
            symbols: &self.rank_symbols,
            meta: &self.rank_meta,
            term: self.term,
        }
    }

    /// Every line's rank sequence, each ending in the terminator rank.
    pub(super) fn ranks(&self) -> &[u32] {
        &self.ranks
    }

    /// The canonical document's length in bytes.
    pub(super) const fn total(&self) -> usize {
        self.total
    }

    /// The longest canonical line in bytes.
    pub(super) const fn longest(&self) -> usize {
        self.longest
    }

    /// The number of canonical lines.
    pub(super) const fn lines(&self) -> usize {
        self.lines
    }
}

/// Order all symbols by their text and give equal text one dense rank.
///
/// `heads` holds every symbol's head key (see [`head_keys`]). They are sorted
/// by key with the standard sort, which sees only that total, infallible order. Each run of equal keys is then ordered by
/// `compare`, which orders two symbols by their full text and may fail, through
/// the fallible sort, so a failure is returned as it was met. Only two symbols
/// of equal keys are compared again when equal text is grouped. The head keys
/// are released before this returns.
///
/// On return each symbol's
/// entry in `symbols` is its rank, and the result lists, per rank, the symbol that
/// stands for it. The order of writes matters: a symbol's packed metadata is read
/// by `compare` until the comparison of that symbol with its successor has
/// returned, so a rank is written only after that comparison. A symbol that
/// `compare` reads is therefore never one that already holds a rank.
///
/// # Errors
/// The first failure of `compare` or of admitting the working arrays.
pub(super) fn rank<
    Id: Copy + Eq + std::hash::Hash,
    E: From<StorageError>,
    S: Admission + ?Sized,
>(
    symbols: &mut Symbols<Id>,
    memory: &mut Memory<'_, S>,
    mut heads: Vec<Head>,
    mut compare: impl FnMut(&Symbols<Id>, &mut Memory<'_, S>, u32, u32) -> Result<Ordering, E>,
) -> Result<Ranked, E> {
    let count = symbols.count();
    debug_assert_eq!(heads.len(), count, "every symbol has a head key");
    heads.sort_unstable_by_key(|head| head.0);
    let mut start = 0;
    while start < heads.len() {
        let key = heads[start].0;
        let end = start
            + heads[start..]
                .iter()
                .take_while(|head| head.0 == key)
                .count();
        if end - start > 1 {
            try_sort_unstable_by(&mut heads[start..end], |a, b| {
                if a.1 == b.1 {
                    return Ok(Ordering::Equal);
                }
                compare(symbols, memory, a.1, b.1)
            })?;
        }
        start = end;
    }
    let mut order: Vec<u32> = Vec::new();
    memory.reserve(&mut order, count)?;
    let mut meta: Vec<u32> = Vec::new();
    memory.reserve(&mut meta, count)?;
    let mut rank = 0usize;
    let mut starts_group = true;
    for at in 0..heads.len() {
        let (key, symbol) = heads[at];
        if starts_group {
            meta.push(symbols.meta[symbol as usize]);
            order.push(symbol);
        }
        // Different keys are different spellings; only equal keys are read.
        let same_as_next = match heads.get(at + 1) {
            Some(&(next_key, next)) if next_key == key => {
                compare(symbols, memory, symbol, next)? == Ordering::Equal
            }
            _ => false,
        };
        symbols.meta[symbol as usize] =
            u32::try_from(rank).map_err(|_| StorageError::SizeOverflow)?;
        starts_group = !same_as_next;
        if !same_as_next {
            rank += 1;
        }
    }
    memory.release_vec(heads)?;
    Ok(Ranked {
        symbols: order,
        meta,
    })
}

/// Every line of the run as a sequence of symbols, each line ending in [`TERM`].
pub(super) struct Lines {
    /// All symbol sequences, back to back; ranks after [`Self::remap`].
    pub(super) seq: Vec<u32>,
    /// Where each regular line starts in `seq`.
    pub(super) starts: Vec<usize>,
    /// Where each irregular line starts in `seq`, and the index of the regular
    /// line it precedes once placed.
    pub(super) irregular: Vec<(usize, usize)>,
}

impl Lines {
    pub(super) const fn new() -> Self {
        Self {
            seq: Vec::new(),
            starts: Vec::new(),
            irregular: Vec::new(),
        }
    }

    /// Replace each symbol by its rank.
    pub(super) fn remap<Id>(&mut self, symbols: &Symbols<Id>) {
        for entry in &mut self.seq {
            *entry = symbols.meta[*entry as usize];
        }
    }

    /// Sort the regular lines by rank sequence and drop duplicates. Ranks tie
    /// exactly for equal text, so this is the byte order of the lines.
    pub(super) fn order_regular(&mut self, term: u32) {
        let seq = &self.seq;
        self.starts
            .sort_unstable_by(|&a, &b| compare_ranks(seq, a, b, term));
        self.starts
            .dedup_by(|b, a| compare_ranks(seq, *a, *b, term) == Ordering::Equal);
    }

    /// The bytes of every line, summed over the regular lines and the placed
    /// irregular ones, with the longest line.
    pub(super) fn measure(
        &self,
        term: u32,
        mut len_of: impl FnMut(u32) -> usize,
    ) -> Result<(usize, usize), StorageError> {
        let mut total = 0usize;
        let mut longest = 0usize;
        let starts = self.starts.iter().copied();
        let irregular = self.irregular.iter().map(|&(start, _)| start);
        for start in starts.chain(irregular) {
            let mut bytes = 0usize;
            let mut at = start;
            loop {
                let rank = self.seq[at];
                bytes = bytes
                    .checked_add(len_of(rank))
                    .ok_or(StorageError::SizeOverflow)?;
                if rank == term {
                    break;
                }
                at += 1;
            }
            total = total.checked_add(bytes).ok_or(StorageError::SizeOverflow)?;
            longest = longest.max(bytes);
        }
        Ok((total, longest))
    }

    /// Order the irregular lines by their bytes, drop duplicates, and place each
    /// among the regular lines: a line byte-equal to a regular one is dropped,
    /// every other records the index of the regular line it precedes.
    ///
    /// `render` writes one line's bytes at the end of the buffer it is given.
    ///
    /// # Errors
    /// The first failure of `render`.
    pub(super) fn place_irregular<E>(
        &mut self,
        first: &mut String,
        second: &mut String,
        mut render: impl FnMut(&[u32], usize, &mut String) -> Result<(), E>,
    ) -> Result<(), E> {
        let seq = &self.seq;
        let mut same = |a: &(usize, usize), b: &(usize, usize)| -> Result<Ordering, E> {
            first.clear();
            second.clear();
            render(seq, a.0, first)?;
            render(seq, b.0, second)?;
            Ok(first.as_bytes().cmp(second.as_bytes()))
        };
        try_sort_unstable_by(&mut self.irregular, &mut same)?;
        try_dedup_by(&mut self.irregular, |a, b| {
            same(a, b).map(|order| order == Ordering::Equal)
        })?;
        let mut kept = 0;
        for read in 0..self.irregular.len() {
            let line = self.irregular[read];
            let (mut low, mut high) = (0, self.starts.len());
            let mut equal = false;
            while low < high {
                let mid = low + (high - low) / 2;
                same(&(self.starts[mid], 0), &line).map(|order| match order {
                    Ordering::Less => low = mid + 1,
                    Ordering::Greater => high = mid,
                    Ordering::Equal => {
                        equal = true;
                        high = low;
                    }
                })?;
            }
            if !equal {
                self.irregular[kept] = (line.0, low);
                kept += 1;
            }
        }
        self.irregular.truncate(kept);
        Ok(())
    }

    /// The line starts in document order: each irregular line before the regular
    /// line it was placed ahead of.
    pub(super) fn merged(&self) -> impl Iterator<Item = usize> + '_ {
        let mut irregular = self.irregular.iter().peekable();
        let mut index = 0;
        std::iter::from_fn(move || {
            if let Some(&&(start, before)) = irregular.peek()
                && before <= index
            {
                irregular.next();
                return Some(start);
            }
            let start = self.starts.get(index).copied()?;
            index += 1;
            Some(start)
        })
    }

    pub(super) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.seq)?;
        memory.release_vec(self.starts)?;
        memory.release_vec(self.irregular)
    }
}

/// Compare two rank sequences that each end at the `term` rank.
fn compare_ranks(seq: &[u32], a: usize, b: usize, term: u32) -> Ordering {
    let (mut left, mut right) = (a, b);
    loop {
        let (x, y) = (seq[left], seq[right]);
        if x != y {
            return x.cmp(&y);
        }
        if x == term {
            return Ordering::Equal;
        }
        left += 1;
        right += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_keeps_every_field_and_marks_a_long_length_at_the_boundary() {
        for (len, long) in [(0, false), (5, false), (LONG as usize - 1, false)] {
            let meta = Meta::new(len, Kind::Literal, true, false, true);
            assert_eq!(meta.len_field() as usize, len);
            assert_eq!(meta.len_field() == LONG, long);
            assert_eq!(meta.kind(), Kind::Literal);
            assert!(meta.clean() && !meta.arena() && meta.aligned());
        }
        for len in [LONG as usize, LONG as usize + 1, usize::MAX] {
            let meta = Meta::new(len, Kind::Iri, false, true, false);
            assert_eq!(meta.len_field(), LONG, "length {len} is kept aside");
            assert_eq!(meta.kind(), Kind::Iri);
            assert!(!meta.clean() && meta.arena() && !meta.aligned());
        }
    }

    #[test]
    fn symbols_beyond_thirty_two_bits_are_refused_and_the_last_one_is_numbered() {
        let last = usize::try_from(u32::MAX - FIRST_LEAF).unwrap();
        assert_eq!(symbol_number(0), Ok(FIRST_LEAF));
        assert_eq!(symbol_number(last), Ok(u32::MAX));
        assert_eq!(symbol_number(last + 1), Err(StorageError::SizeOverflow));
    }

    fn one_token(text: &str) -> bool {
        is_one_token(text.as_bytes())
    }

    /// The one-pass alignment checks equal the reference tokenizer on every byte
    /// string up to six bytes over the bytes that steer it (quote, backslash,
    /// space, newline) and two ordinary ones, both for a whole symbol and for a
    /// leaf whose space is implied. It protects the alignment bit, which decides
    /// whether a line is ordered by ranks or by its bytes.
    #[test]
    fn the_one_pass_alignment_check_equals_the_reference_tokenizer() {
        const ALPHABET: [u8; 6] = *b"\"\\ \na@";
        let mut checked = 0;
        for len in 0..=6_u32 {
            for mut code in 0..ALPHABET.len().pow(len) {
                let mut bytes = Vec::new();
                for _ in 0..len {
                    bytes.push(ALPHABET[code % ALPHABET.len()]);
                    code /= ALPHABET.len();
                }
                assert_eq!(
                    is_one_token(&bytes),
                    reference_is_one_token(&bytes),
                    "{bytes:?}"
                );
                let mut spaced = bytes.clone();
                spaced.push(b' ');
                assert_eq!(
                    leaf_is_one_token(&bytes),
                    reference_is_one_token(&spaced),
                    "{bytes:?} and its space"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, (0..=6).map(|len| 6_usize.pow(len)).sum::<usize>());
    }

    #[test]
    fn a_symbol_is_one_token_unless_a_tag_holds_a_space_or_newline() {
        assert!(one_token("<http://e/a> "));
        assert!(one_token("\"x\"@en "));
        assert!(one_token("\"x\"@ "));
        assert!(one_token("\"a b\"^^<http://e/d> "));
        assert!(one_token("\"x\\\" y\" "));
        assert!(one_token(".\n"));
        assert!(one_token("<<( "));
        assert!(!one_token("\"x\"@en US "));
        assert!(!one_token("\"x\"@en <http://e/g> "));
        assert!(!one_token("\"x\"@en.\n "));
        // A stray quote in a tag cannot make the tokenizer span the next symbol.
        assert!(one_token("\"x\"@a\"b "));
    }
    /// The bracketed comparison equals comparing `x + ">"` with `y + ">"` byte
    /// by byte, on every pair of strings up to four bytes over bytes on both
    /// sides of `>` and `>` itself, so pairs share prefixes and one is often a
    /// prefix of the other. It protects the order of clean IRIs, which are
    /// compared without writing their closing brackets.
    #[test]
    fn the_bracketed_comparison_equals_comparing_with_the_bracket_written() {
        const ALPHABET: [u8; 5] = *b"/=>?a";
        let mut strings = Vec::new();
        for len in 0..=4_u32 {
            for mut code in 0..ALPHABET.len().pow(len) {
                let mut bytes = Vec::new();
                for _ in 0..len {
                    bytes.push(ALPHABET[code % ALPHABET.len()]);
                    code /= ALPHABET.len();
                }
                strings.push(bytes);
            }
        }
        let (mut prefixes, mut pairs) = (0, 0);
        for x in &strings {
            for y in &strings {
                let written = |bytes: &[u8]| [bytes, b">"].concat();
                assert_eq!(
                    cmp_bracketed(x, y),
                    written(x).cmp(&written(y)),
                    "{x:?} {y:?}"
                );
                prefixes += usize::from(x.len() < y.len() && y.starts_with(x));
                pairs += 1;
            }
        }
        assert_eq!(pairs, 781 * 781);
        assert!(prefixes > 1000, "proper prefixes are compared ({prefixes})");
    }

    /// A clean IRI's key, built without writing the brackets and the space, is
    /// the key of its written spelling for every length around the head, so
    /// the closing bracket and the space land where the spelling puts them.
    #[test]
    fn a_clean_iri_key_equals_the_key_of_its_written_spelling() {
        for len in 0..=HEAD + 2 {
            let iri = "i".repeat(len);
            let spelled = format!("<{iri}> ");
            assert_eq!(
                bracketed_head_key(iri.as_bytes()),
                head_key(spelled.as_bytes()),
                "an IRI of {len} bytes"
            );
        }
    }

    /// Rank `leaves` (each spelled with its trailing space) after the six fixed
    /// symbols, comparing full spellings; the ranks in symbol order, and how
    /// many comparisons ranking made.
    fn rank_spellings(leaves: &[&[u8]]) -> (Vec<u32>, usize) {
        let fixed: [&[u8]; 6] = [b"<<( ", b")>> ", b".\n", b"<r> ", b"<s> ", b"<t> "];
        let mut spellings: Vec<Vec<u8>> = fixed.iter().map(|text| text.to_vec()).collect();
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = Memory::new(&mut resident);
        let mut symbols = Symbols::<u32>::new();
        for text in fixed {
            symbols
                .push_fixed(text.len(), false, true, &mut memory)
                .unwrap();
        }
        for (at, text) in leaves.iter().enumerate() {
            let body = std::str::from_utf8(text).unwrap();
            let id = u32::try_from(at).unwrap();
            symbols
                .push_leaf(id, Kind::Literal, false, false, body, &mut memory)
                .unwrap();
            spellings.push([*text, b" "].concat());
        }
        let heads = head_keys::<u32, StorageError, _>(&symbols, &mut memory, |_, _, symbol| {
            Ok(head_key(&spellings[symbol as usize]))
        })
        .unwrap();
        let mut compares = 0;
        rank::<u32, StorageError, _>(&mut symbols, &mut memory, heads, |_, _, a, b| {
            compares += 1;
            Ok(spellings[a as usize].cmp(&spellings[b as usize]))
        })
        .unwrap();
        let ranks = (0..u32::try_from(spellings.len()).unwrap())
            .map(|symbol| symbols.rank_of(symbol))
            .collect();
        let mut distinct = spellings.clone();
        distinct.sort_unstable();
        distinct.dedup();
        for (symbol, spelling) in spellings.iter().enumerate() {
            let rank = symbols.rank_of(u32::try_from(symbol).unwrap());
            assert_eq!(
                &distinct[rank as usize], spelling,
                "symbol {symbol} ranks where its spelling sorts"
            );
        }
        (ranks, compares)
    }

    /// When every leaf shares its first 40 bytes, every head key ties and the
    /// refinement by full spelling decides the whole order; the result is still
    /// the bytewise order of the spellings. It protects ranking on data whose
    /// symbols differ only past the head.
    #[test]
    fn ranking_equals_the_text_order_when_every_head_key_ties() {
        let shared = "\"".to_owned() + &"p".repeat(40);
        let mut leaves: Vec<Vec<u8>> = Vec::new();
        for tail in [
            "\"", "a\"", "\" ", "\"@en", "b\"", "a\"", "\"^^<x>", "\\\"\"",
        ] {
            leaves.push(format!("{shared}{tail}").into_bytes());
        }
        let borrowed: Vec<&[u8]> = leaves.iter().map(Vec::as_slice).collect();
        let (_, compares) = rank_spellings(&borrowed);
        assert!(
            compares >= leaves.len() - 1,
            "the refinement compares the tied leaves ({compares})"
        );
    }

    /// Spellings shorter than the head, one a prefix of another and one holding
    /// a zero byte beside the padding, rank in bytewise order with no comparison:
    /// different keys already decide them. It protects the zero padding.
    #[test]
    fn short_spellings_rank_by_their_padded_keys() {
        let leaves: [&[u8]; 8] = [b"<a", b"<ab", b"x", b"x ", b"x\0", b"", b"<a>", b"\"x\""];
        let (ranks, compares) = rank_spellings(&leaves);
        assert_eq!(compares, 0, "no two of these keys tie");
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ranks.len(), "every spelling is distinct");
    }

    /// Ranking is dense and never lets the comparison read a rank as if it were
    /// packed meta: equal texts of different view terms share one rank, ranks
    /// follow the text order with no gaps, and each rank keeps the meta of the
    /// symbols it stands for. It protects the shared meta array that holds a
    /// symbol's packed bits before its rank and its rank afterwards.
    #[test]
    fn ranking_is_dense_and_the_comparison_only_reads_unranked_meta() {
        let texts = [
            "<<( ",
            ")>> ",
            ".\n",
            "<r> ",
            "<r> ",
            "<a> ",
            "<a> ",
            "<b c> ",
            "\"x\" ",
            "\"x\"@en ",
            "<a> ",
            "_:c14n1 ",
            "<b> ",
        ];
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = Memory::new(&mut resident);
        let mut symbols = Symbols::<u32>::new();
        let mut original = Vec::new();
        for (at, text) in texts.iter().enumerate() {
            let clean = (text.starts_with('<') && !text.contains(' ')) || at < 6;
            if at < usize::try_from(FIRST_LEAF).unwrap() {
                symbols
                    .push_fixed(text.len(), clean, true, &mut memory)
                    .unwrap();
                original.push(symbols.meta_of(u32::try_from(at).unwrap()).bits());
            } else {
                let id = u32::try_from(at).unwrap();
                let body = &text[..text.len() - 1];
                let kind = if body.starts_with('<') {
                    Kind::Iri
                } else {
                    Kind::Literal
                };
                let (symbol, meta) = symbols
                    .push_leaf(id, kind, clean, false, body, &mut memory)
                    .unwrap();
                assert_eq!(symbol as usize, at);
                original.push(meta.bits());
            }
        }
        let mut ranked_reads = 0_usize;
        let heads = head_keys::<u32, StorageError, _>(&symbols, &mut memory, |_, _, symbol| {
            Ok(head_key(texts[symbol as usize].as_bytes()))
        })
        .unwrap();
        let ranked =
            rank::<u32, StorageError, _>(&mut symbols, &mut memory, heads, |s, _, a, b| {
                for symbol in [a, b] {
                    assert_eq!(
                        s.meta_of(symbol).bits(),
                        original[symbol as usize],
                        "symbol {symbol} was compared after its rank was written"
                    );
                    ranked_reads += 1;
                }
                Ok(texts[a as usize].cmp(texts[b as usize]))
            })
            .unwrap();
        assert!(ranked_reads > 0);
        let mut distinct: Vec<&str> = texts.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(ranked.symbols.len(), distinct.len(), "ranks are dense");
        assert_eq!(ranked.meta.len(), distinct.len());
        for (at, text) in texts.iter().enumerate() {
            let rank = symbols.rank_of(u32::try_from(at).unwrap()) as usize;
            assert_eq!(distinct[rank], *text, "rank order is text order");
            assert_eq!(
                ranked.meta[rank], original[ranked.symbols[rank] as usize],
                "a rank keeps the meta of the symbols it stands for"
            );
        }
    }
}

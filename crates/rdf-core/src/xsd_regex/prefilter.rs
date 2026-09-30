// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The required-literal prefilter in front of a compiled pattern's matcher.
//!
//! The `regex` crate is built without its own literal prefilter, so every
//! unanchored search walks the haystack through its lazy DFA, a table step per
//! byte. Most real patterns name bytes every match must contain: `needle`,
//! `foo.*bar`, `\d{4}-\d{2}`, `[a-z]+@example\.org`. This module finds such a
//! **window** in the pattern — consecutive positions of which each matches
//! exactly one byte from a known set — and searches the haystack for it with
//! [`purrdf_lex::scan`]'s packed compares before the engine runs, so a haystack
//! that holds no window is refused without the engine, and one that does is
//! handed to the engine only around where a match through the window can lie.
//!
//! # What the window is, and why it is exact
//!
//! The window is read from the same `regex-syntax` HIR the engine compiles
//! (same source, same `i`/`s`/`m` modes), so case folding is already in it:
//! under `i`, `needle` is the classes `[Nn][Ee][Ee][Dd][Ll][Ee]`, each a
//! two-byte position, and a letter whose fold leaves ASCII (`k` also matches
//! U+212A KELVIN SIGN) is not a single-byte position and ends the window
//! rather than being approximated. A window is taken only from a sub-pattern
//! every match passes through: a concatenation's members, the body of a
//! repetition with at least one iteration, a capture group. An alternation of
//! more than single characters, an optional part and a repetition that may run
//! zero times contribute nothing.
//!
//! The search tests two of the window's positions at once, the two cheapest
//! by a commonness heuristic and as far apart as the tie allows: positions of
//! one or two bytes by equality ([`find_byte_pair`]), narrow classes such as
//! `[0-9]` by range ([`find_range_pair`]); a window with one such position runs
//! the one-position searches. Every other position is verified afterwards.
//!
//! # Where the engine runs
//!
//! Each window carries what precedes it in a match, member by member: a member
//! of bounded length contributes its maximum length, and a repetition of an
//! ASCII class with no upper bound (`[a-z]+`) contributes the run of that
//! class's bytes that ends where the next member begins. Read backward from a
//! window found at `q`, these give the lowest offset a match through it can
//! begin at, and the engine is started there with
//! [`regex::Regex::is_match_at`], which keeps the text before the start as
//! context for `^` under `m`. Any other unbounded member (`.*`) leaves the
//! start at zero.
//!
//! What follows the window bounds where such a match ends, when every member
//! after it is bounded and the pattern asserts nothing about the text after a
//! position (no `$`). The engine then runs on that slice alone and the search
//! moves on to the next window when it finds nothing there; otherwise the
//! engine runs once from the start bound to the end of the haystack.
//!
//! A pattern that is its window and nothing else (`needle`, a `q` literal, a
//! case-folded word) is answered by the search alone. A pattern anchored at the
//! start of the haystack (`^…` without `m`) gets no prefilter: the engine only
//! tries offset zero and stops at its first dead state, which no search can
//! beat. Nor does one anchored at its end (`…$` without `m`): the engine
//! matches it backward from the last byte.
//!
//! # Anchored patterns
//!
//! What such a pattern gets instead is an [`Affix`]: the single-byte positions
//! directly after its `^` must be the haystack's first bytes, and those
//! directly before its `$` its last, so a haystack whose ends do not hold them
//! is refused by a slice compare, and one that does is decided without the
//! engine when those positions are the whole pattern (`^http://example\.org/`,
//! `[0-9]{3}$`, `^[a-z]`): the engine's per-call start-up (its cache, its
//! search configuration) and its walk are not run at all.
//!
//! # When the plan is read
//!
//! A [`Plan`] (window, literal, affix, or the engine alone) is read from the
//! parsed syntax when a match first needs it, not when the pattern is
//! compiled, so compiling costs one parse, the engine's, and a pattern never
//! matched is never parsed for its plan. The translation pass decides the
//! common plans without any parse: a pattern that is literal text is its own
//! syntax tree, and one with no token that can be a single-byte position, or
//! anchored with only repeated or non-ASCII atoms next to its anchors
//! (`^[a-z0-9]+$`), has no plan beyond the engine.

use purrdf_lex::scan::{ByteRun, find_byte, find_byte_pair, find_byte2, find_range_pair};
use regex_syntax::hir::{Class, Hir, HirKind, Look};

/// The most positions a window keeps; a longer run of single-byte positions is
/// cut to its first `MAX_WINDOW`, which every match still contains.
const MAX_WINDOW: usize = 64;

/// The widest slice, in bytes past the window, the engine is confined to.
/// Past it, a window's slice costs as much as running on from the window.
const MAX_TAIL: usize = 4096;

/// How many windows the engine is tried on before it is run once over the rest
/// of the haystack: a haystack full of windows that are not matches is walked
/// at the engine's pace rather than restarted per window.
const MAX_WINDOW_TRIES: usize = 16;

/// Failed window verifications after which, if they are denser than one per
/// [`MISS_DENSITY`] bytes, the search gives way to the engine.
const MAX_DENSE_MISSES: usize = 16;

/// See [`MAX_DENSE_MISSES`].
const MISS_DENSITY: usize = 32;

/// The most bytes a position searched by range may hold: a wider class
/// (`[a-z]`, `[^a]`) matches most text, so every byte would be a candidate.
const MAX_PROBE_BYTES: u32 = 16;

/// How the search tests one window position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Probe {
    /// One or two bytes, a single byte written twice.
    Bytes([u8; 2]),
    /// One or two inclusive runs, a single run written twice.
    Runs([ByteRun; 2]),
}

impl Probe {
    /// The probe as runs, for the range search.
    const fn runs(self) -> [ByteRun; 2] {
        match self {
            Self::Bytes([a, b]) => [(a, a), (b, b)],
            Self::Runs(runs) => runs,
        }
    }
}

/// A set of bytes, one bit per byte value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteSet([u64; 4]);

impl ByteSet {
    const EMPTY: Self = Self([0; 4]);

    fn single(b: u8) -> Self {
        let mut set = Self::EMPTY;
        set.insert(b);
        set
    }

    const fn insert(&mut self, b: u8) {
        self.0[(b >> 6) as usize] |= 1 << (b & 63);
    }

    const fn contains(self, b: u8) -> bool {
        self.0[(b >> 6) as usize] & (1 << (b & 63)) != 0
    }

    const fn len(self) -> u32 {
        self.0[0].count_ones()
            + self.0[1].count_ones()
            + self.0[2].count_ones()
            + self.0[3].count_ones()
    }

    /// The set's one byte, when it holds exactly one.
    fn only(self) -> Option<u8> {
        if self.len() == 1 {
            self.members().next()
        } else {
            None
        }
    }

    /// The set's bytes in ascending order, read off its words' set bits.
    fn members(self) -> impl Iterator<Item = u8> {
        self.0.into_iter().enumerate().flat_map(|(word, mut bits)| {
            std::iter::from_fn(move || {
                (bits != 0).then(|| {
                    let low = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    // `word < 4` and `low < 64`: the byte value.
                    u8::try_from(word * 64 + low).unwrap_or(u8::MAX)
                })
            })
        })
    }

    /// Whether the set shares a byte with `other`.
    const fn meets(self, other: Self) -> bool {
        (self.0[0] & other.0[0])
            | (self.0[1] & other.0[1])
            | (self.0[2] & other.0[2])
            | (self.0[3] & other.0[3])
            != 0
    }

    /// How the search tests the set, when it can: one or two bytes by
    /// equality, one or two runs of at most [`MAX_PROBE_BYTES`] bytes by range.
    fn probe(self) -> Option<Probe> {
        if self.len() <= 2 {
            let mut bytes = self.members();
            let first = bytes.next()?;
            return Some(Probe::Bytes([first, bytes.next().unwrap_or(first)]));
        }
        if self.len() > MAX_PROBE_BYTES {
            return None;
        }
        let mut runs: [ByteRun; 2] = [(0, 0); 2];
        let mut count = 0;
        for b in self.members() {
            match count {
                1 | 2 if runs[count - 1].1.checked_add(1) == Some(b) => runs[count - 1].1 = b,
                0 | 1 => {
                    runs[count] = (b, b);
                    count += 1;
                }
                _ => return None,
            }
        }
        match count {
            1 => Some(Probe::Runs([runs[0], runs[0]])),
            _ => Some(Probe::Runs(runs)),
        }
    }

    /// How costly the set is to search for, lower is better: how common its
    /// bytes are, and for a range how many it holds.
    fn cost(self) -> u8 {
        let breadth = match self.len() {
            0..=2 => 0,
            3..=10 => 1,
            _ => 3,
        };
        self.commonness() + breadth
    }

    /// How common the set's bytes are in ordinary text, lower is rarer: the
    /// search tests the rarest positions, so a common byte stops it less often.
    /// A heuristic only; any choice gives the same answers. The commonest
    /// class any member falls in, each class tested as one mask.
    fn commonness(self) -> u8 {
        if self.meets(COMMONEST) {
            4
        } else if self.meets(COMMON) {
            3
        } else if self.meets(HIGH) {
            2
        } else {
            u8::from(self.meets(UPPER))
        }
    }
}

/// The set of the bytes in `lo..=hi`, for the commonness masks.
const fn byte_range(mut set: ByteSet, lo: u8, hi: u8) -> ByteSet {
    let mut b = lo;
    loop {
        set.insert(b);
        if b == hi {
            return set;
        }
        b += 1;
    }
}

/// The set of `bytes`, for the commonness masks.
const fn byte_list(mut set: ByteSet, bytes: &[u8]) -> ByteSet {
    let mut i = 0;
    while i < bytes.len() {
        set.insert(bytes[i]);
        i += 1;
    }
    set
}

/// The commonest bytes of ordinary text: commonness 4.
const COMMONEST: ByteSet = byte_list(ByteSet::EMPTY, b" etaoinsrhl");
/// Lowercase letters, digits and IRI punctuation: commonness 3.
const COMMON: ByteSet = byte_list(
    byte_range(byte_range(ByteSet::EMPTY, b'a', b'z'), b'0', b'9'),
    b"/.:-_",
);
/// Bytes of multi-byte UTF-8 sequences: commonness 2.
const HIGH: ByteSet = byte_range(ByteSet::EMPTY, 0x80, 0xFF);
/// Uppercase letters: commonness 1.
const UPPER: ByteSet = byte_range(ByteSet::EMPTY, b'A', b'Z');

/// One member of a match before its window, as the backward bound reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Back {
    /// At most this many bytes.
    Bytes(usize),
    /// Any number of bytes from the set: the run of them ending where the next
    /// member begins.
    Run(ByteSet),
}

/// What precedes a window in a match, first member first, or `None` when a
/// member is neither bounded nor an ASCII-class run.
type Reach = Option<Vec<Back>>;

/// `reach` followed by `member`.
fn extend(reach: &Reach, member: &Hir) -> Reach {
    let mut steps = reach.clone()?;
    if let Some(bytes) = member.properties().maximum_len() {
        match steps.last_mut() {
            Some(Back::Bytes(total)) => *total = total.saturating_add(bytes),
            _ => steps.push(Back::Bytes(bytes)),
        }
        return Some(steps);
    }
    match member.kind() {
        HirKind::Repetition(repetition) => match repetition.sub.kind() {
            HirKind::Class(class) => {
                steps.push(Back::Run(ascii_set(class)?));
                Some(steps)
            }
            _ => None,
        },
        _ => None,
    }
}

/// A window found in the pattern: its per-position byte sets, what precedes
/// it, and the most bytes a match spans after it.
struct Run {
    sets: Vec<ByteSet>,
    reach: Reach,
    after: Option<usize>,
}

impl Run {
    /// Which of two windows to search for. A window with two searchable
    /// positions filters far better than one with a single position; a known
    /// reach lets the engine start near the window rather than at zero; and a
    /// longer window is verified more strictly.
    fn quality(&self) -> (usize, bool, bool, usize) {
        let searchable = self.sets.iter().filter(|set| set.probe().is_some()).count();
        (
            searchable.min(2),
            self.reach.is_some(),
            self.after.is_some(),
            self.sets.len(),
        )
    }
}

/// The prefilter of one compiled pattern: a window every match contains, how
/// to search for it, and how far around it a match can lie.
#[derive(Debug, Clone)]
pub(super) struct Prefilter {
    window: Box<[ByteSet]>,
    /// The window as bytes when each position is one byte, so a candidate is
    /// verified by one slice compare.
    literal: Option<Box<[u8]>>,
    /// The window position the search tests, and how it tests it.
    lead: (usize, Probe),
    /// A second, later position tested together with `lead`.
    trail: Option<(usize, Probe)>,
    /// What precedes the window in a match; `None` when the start of a match
    /// cannot be bounded from its window.
    reach: Option<Box<[Back]>>,
    /// The most bytes a match spans from the window's first byte, when the
    /// engine may be run on a haystack cut there: every member after the
    /// window is bounded and the pattern asserts nothing about the text after
    /// a position.
    tail: Option<usize>,
    /// The pattern is the window and nothing else, so a window is a match.
    exact: bool,
}

/// Where the search for the next window stopped.
enum Candidate {
    /// A verified window begins here.
    Window(usize),
    /// The search gave way to the engine here: no window begins before it.
    GiveWay(usize),
    /// No window remains, so no match does.
    None,
}

impl Prefilter {
    /// The prefilter for `hir`, the syntax the engine compiled, or `None` when
    /// the pattern names no single-byte window every match contains, or is
    /// anchored at the start or the end of the haystack.
    pub(super) fn build(hir: &Hir) -> Option<Self> {
        let properties = hir.properties();
        if properties.look_set_prefix().contains(Look::Start)
            || properties.look_set_suffix().contains(Look::End)
        {
            return None;
        }
        let mut best = None;
        collect(hir, &Some(Vec::new()), Some(0), &mut best);
        let run = best?;
        let window: Box<[ByteSet]> = run.sets.into_boxed_slice();
        let mut searchable: Vec<(usize, Probe, u8)> = window
            .iter()
            .enumerate()
            .filter_map(|(at, set)| set.probe().map(|probe| (at, probe, set.cost())))
            .collect();
        // The cheapest position, the earliest among equals; then the cheapest
        // of the rest, the farthest from the first among equals, since bytes
        // near each other in a word tend to occur together.
        searchable.sort_by_key(|&(at, _, cost)| (cost, at));
        let &(first_at, first_pair, _) = searchable.first()?;
        let second = searchable[1..]
            .iter()
            .min_by_key(|&&(at, _, cost)| (cost, std::cmp::Reverse(at.abs_diff(first_at))))
            .map(|&(at, probe, _)| (at, probe));
        let (lead, trail) = match second {
            Some((at, pair)) if at < first_at => ((at, pair), Some((first_at, first_pair))),
            Some(other) => ((first_at, first_pair), Some(other)),
            None => ((first_at, first_pair), None),
        };
        let looks = properties.look_set();
        let forward_blind = looks
            .iter()
            .all(|look| matches!(look, Look::Start | Look::StartLF));
        let tail = run
            .after
            .map(|after| window.len().saturating_add(after))
            .filter(|&tail| forward_blind && tail <= MAX_TAIL);
        let exact = looks.is_empty() && positions(hir).is_some_and(|sets| *sets == *window);
        let literal = window
            .iter()
            .map(|set| set.only())
            .collect::<Option<Box<[u8]>>>();
        Some(Self {
            window,
            literal,
            lead,
            trail,
            reach: run.reach.map(Vec::into_boxed_slice),
            tail,
            exact,
        })
    }

    /// Whether `regex`, the engine this prefilter was built for, matches
    /// anywhere in `haystack`: exactly `regex.is_match(haystack)`.
    ///
    /// Out of line: inlined into [`Plan::is_match`], its window loop would give
    /// every plan's call the large stack frame it needs, and the literal and
    /// anchored plans need none of it.
    #[inline(never)]
    pub(super) fn is_match(&self, regex: &regex::Regex, haystack: &str) -> bool {
        let bytes = haystack.as_bytes();
        let mut from = 0;
        let mut misses = 0;
        let mut tries = 0;
        loop {
            let (at, give_way) = match self.next_window(bytes, from, &mut misses) {
                Candidate::Window(_) if self.exact => return true,
                Candidate::Window(at) => (at, false),
                Candidate::GiveWay(at) => (at, true),
                Candidate::None => return false,
            };
            // Every match whose window begins at or after `at` begins at or
            // after this bound; windows before `at` were ruled out.
            let start = haystack.floor_char_boundary(self.start_bound(bytes, at));
            match self.tail {
                Some(tail) if !give_way && tries < MAX_WINDOW_TRIES => {
                    // A match through this window ends by `at + tail`.
                    let end = haystack.ceil_char_boundary(at.saturating_add(tail));
                    if regex.is_match_at(&haystack[..end], start) {
                        return true;
                    }
                    tries += 1;
                    from = at + 1;
                }
                _ => return regex.is_match_at(haystack, start),
            }
        }
    }

    /// Whether `haystack` holds a window, so that the pattern may match in it;
    /// `false` proves it cannot.
    pub(super) fn may_match(&self, haystack: &str) -> bool {
        let mut misses = 0;
        !matches!(
            self.next_window(haystack.as_bytes(), 0, &mut misses),
            Candidate::None
        )
    }

    /// The lowest offset a match whose window begins at `at` can begin at:
    /// the members before the window read backward from it. A bounded member
    /// moves the bound back by its maximum length; a class run moves it back
    /// over the run of that class's bytes ending at the bound, which is the
    /// longest the member can be wherever the next member begins, since a run
    /// ending later cannot begin earlier.
    fn start_bound(&self, bytes: &[u8], at: usize) -> usize {
        let Some(reach) = &self.reach else {
            return 0;
        };
        let mut bound = at;
        for step in reach.iter().rev() {
            bound -= match *step {
                Back::Bytes(most) => most.min(bound),
                Back::Run(set) => bytes[..bound]
                    .iter()
                    .rev()
                    .take_while(|&&b| set.contains(b))
                    .count(),
            };
        }
        bound
    }

    /// The first window beginning at or after `from`.
    #[inline]
    fn next_window(&self, bytes: &[u8], mut from: usize, misses: &mut usize) -> Candidate {
        let (lead_at, lead) = self.lead;
        loop {
            let Some(rest) = bytes.get(from + lead_at..) else {
                return Candidate::None;
            };
            let found = match (lead, self.trail) {
                (Probe::Bytes(lead), Some((trail_at, Probe::Bytes(trail)))) => {
                    find_byte_pair(rest, lead, trail, trail_at - lead_at)
                }
                (lead, Some((trail_at, trail))) => {
                    find_range_pair(rest, lead.runs(), trail.runs(), trail_at - lead_at)
                }
                (Probe::Bytes([a, b]), None) if a == b => find_byte(rest, a),
                (Probe::Bytes([a, b]), None) => find_byte2(rest, a, b),
                (Probe::Runs(runs), None) => find_range_pair(rest, runs, runs, 0),
            };
            let Some(offset) = found else {
                return Candidate::None;
            };
            let at = from + offset;
            let verified =
                bytes
                    .get(at..at + self.window.len())
                    .is_some_and(|slice| match &self.literal {
                        Some(literal) => same_bytes(slice, literal),
                        None => slice
                            .iter()
                            .zip(&self.window)
                            .all(|(&b, set)| set.contains(b)),
                    });
            if verified {
                return Candidate::Window(at);
            }
            *misses += 1;
            if *misses > MAX_DENSE_MISSES && *misses * MISS_DENSITY > at {
                return Candidate::GiveWay(at);
            }
            from = at + 1;
        }
    }
}

/// What a pattern anchored at the start or the end of the haystack must match
/// there: the single-byte positions directly after its `^` and directly before
/// its `$` (both only without `m`, where they are [`Look::Start`] and
/// [`Look::End`]).
///
/// Every match of `^…` begins at offset zero, so its leading positions are the
/// haystack's first bytes; every match of `…$` ends at the last byte, so its
/// trailing positions are the haystack's last bytes. A haystack whose ends do
/// not hold them has no match, and is refused before the engine runs. When the
/// positions are the whole pattern (`^http://example\.org/`, `[0-9]{3}$`,
/// `^[a-z]`) the check is the answer, and the engine never runs at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Affix {
    /// The positions the haystack must begin with (empty unless `^`-anchored).
    prefix: Positions,
    /// The positions the haystack must end with (empty unless `$`-anchored).
    suffix: Positions,
    /// How the checks decide a haystack that passes them.
    rest: Rest,
}

/// The positions of one end of an anchored pattern: bytes when each position
/// is one byte (none included), compared as one slice, and sets otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Positions {
    Bytes(Box<[u8]>),
    Sets(Box<[ByteSet]>),
}

impl Positions {
    fn of(sets: Vec<ByteSet>) -> Self {
        match sets
            .iter()
            .map(|set| set.only())
            .collect::<Option<Vec<u8>>>()
        {
            Some(bytes) => Self::Bytes(bytes.into_boxed_slice()),
            None => Self::Sets(sets.into_boxed_slice()),
        }
    }

    const fn len(&self) -> usize {
        match self {
            Self::Bytes(bytes) => bytes.len(),
            Self::Sets(sets) => sets.len(),
        }
    }

    /// Whether `slice`, exactly [`len`](Self::len) bytes, holds the positions.
    fn hold(&self, slice: &[u8]) -> bool {
        match self {
            Self::Bytes(bytes) => same_bytes(slice, bytes),
            Self::Sets(sets) => slice.iter().zip(sets).all(|(&b, set)| set.contains(b)),
        }
    }
}

/// What decides a haystack whose ends hold an [`Affix`]'s positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rest {
    /// The pattern is the positions and its anchors: every such haystack
    /// matches.
    Matches,
    /// The pattern is `^`, its positions and `$`: a haystack matches when it
    /// is the positions, byte for byte.
    Whole,
    /// The engine decides.
    Engine,
}

impl Affix {
    /// The affix of `hir`, the syntax the engine compiled, or `None` when the
    /// pattern is anchored at neither end or has no position next to an
    /// anchor.
    pub(super) fn build(hir: &Hir) -> Option<Self> {
        let members = match hir.kind() {
            HirKind::Concat(members) => members.as_slice(),
            _ => std::slice::from_ref(hir),
        };
        let is = |member: Option<&Hir>, want: Look| {
            member.is_some_and(|m| matches!(m.kind(), HirKind::Look(look) if *look == want))
        };
        let start = is(members.first(), Look::Start);
        let body = &members[usize::from(start)..];
        let end = is(body.last(), Look::End);
        let body = &body[..body.len() - usize::from(end)];
        let mut taken = 0;
        let mut prefix = Vec::new();
        if start {
            for member in body {
                let Some(sets) = positions(member) else { break };
                prefix.extend(sets);
                taken += 1;
            }
        }
        let mut suffix = Vec::new();
        if end {
            for member in body[taken..].iter().rev() {
                let Some(mut sets) = positions(member) else {
                    break;
                };
                sets.extend(suffix);
                suffix = sets;
                taken += 1;
            }
        }
        if prefix.is_empty() && suffix.is_empty() {
            return None;
        }
        let rest = match (taken == body.len(), start, end) {
            (true, true, true) => Rest::Whole,
            (true, _, _) => Rest::Matches,
            (false, _, _) => Rest::Engine,
        };
        Some(Self {
            prefix: Positions::of(prefix),
            suffix: Positions::of(suffix),
            rest,
        })
    }

    /// Whether the haystack's ends hold the positions, so that the pattern may
    /// match it; `false` proves it cannot.
    #[inline]
    pub(super) fn may_match(&self, haystack: &str) -> bool {
        let bytes = haystack.as_bytes();
        let (head, tail) = (self.prefix.len(), self.suffix.len());
        bytes.len() >= head.max(tail)
            && self.prefix.hold(&bytes[..head])
            && self.suffix.hold(&bytes[bytes.len() - tail..])
    }

    /// Whether `regex`, the engine this affix was built for, matches
    /// `haystack`: exactly `regex.is_match(haystack)`.
    #[inline]
    pub(super) fn is_match(&self, regex: &regex::Regex, haystack: &str) -> bool {
        if !self.may_match(haystack) {
            return false;
        }
        match self.rest {
            Rest::Matches => true,
            Rest::Whole => haystack.len() == self.prefix.len(),
            Rest::Engine => regex.is_match(haystack),
        }
    }
}

/// A pattern that is literal bytes and nothing else (`example\.org`, a `q`
/// literal): a match is an occurrence of the bytes, so the search answers
/// without the engine.
///
/// Two of the bytes its [`Prefilter`] window tests are searched for together
/// with [`find_byte_pair`] (one with [`find_byte`] when the window has one
/// searchable byte), and a candidate is verified against the whole literal by
/// a word compare, however long it is: the engine's automaton for a literal is
/// as long as the literal, so it is never handed one to confirm. Unlike the
/// general window loop, it keeps no start or end bounds and never runs the
/// engine on a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Needle {
    bytes: Box<[u8]>,
    /// The needle offset and byte the search tests first.
    lead: (usize, u8),
    /// A second, later offset and byte tested together with `lead`.
    trail: Option<(usize, u8)>,
}

impl Needle {
    /// The needle of `hir` when it is literal bytes and nothing else, searched
    /// for by the positions `prefilter`, its window's, tests. A literal longer
    /// than the window ([`MAX_WINDOW`]) is searched by the window's positions
    /// and verified whole.
    fn of(prefilter: &Prefilter, hir: &Hir) -> Option<Self> {
        let HirKind::Literal(literal) = hir.kind() else {
            return None;
        };
        let byte = |(at, probe): (usize, Probe)| match probe {
            Probe::Bytes([a, b]) if a == b => Some((at, a)),
            _ => None,
        };
        let bytes = literal.0.clone();
        let lead = byte(prefilter.lead)?;
        let trail = match prefilter.trail {
            Some(trail) => Some(byte(trail)?),
            None => None,
        };
        Some(Self { bytes, lead, trail })
    }

    /// The first offset at or after `from` where `haystack` holds the needle.
    #[allow(
        clippy::inline_always,
        reason = "the search of a short haystack is a few word compares; inlined, it has no \
                  call frame around them"
    )]
    #[inline(always)]
    fn find(&self, haystack: &[u8], mut from: usize, misses: &mut usize) -> Found {
        let n = self.bytes.len();
        let Some(last) = haystack.len().checked_sub(n) else {
            return Found::None;
        };
        let (lead_at, lead) = self.lead;
        while from <= last {
            // Candidates `p` run over `from..=last`, so every byte the search
            // and the verification read is in bounds.
            let found = match self.trail {
                Some((trail_at, trail)) => find_byte_pair(
                    &haystack[from + lead_at..=last + trail_at],
                    [lead; 2],
                    [trail; 2],
                    trail_at - lead_at,
                ),
                None => find_byte(&haystack[from + lead_at..=last + lead_at], lead),
            };
            let Some(offset) = found else {
                return Found::None;
            };
            let at = from + offset;
            if same_bytes(&haystack[at..at + n], &self.bytes) {
                return Found::Needle;
            }
            *misses += 1;
            // Only a needle as short as a window gives way: the engine's
            // automaton for a longer literal is as long as the literal, so a
            // longer literal stays on the search, restarted per candidate.
            if n <= MAX_WINDOW && *misses > MAX_DENSE_MISSES && *misses * MISS_DENSITY > at {
                return Found::GiveWay(at);
            }
            from = at + 1;
        }
        Found::None
    }

    /// Whether `regex`, the engine this needle was built for, matches anywhere
    /// in `haystack`: exactly `regex.is_match(haystack)`. A haystack dense with
    /// candidates that are not the needle gives way to the engine where the
    /// search stopped, as the general window does.
    #[allow(
        clippy::inline_always,
        reason = "the search of a short haystack is a few word compares; inlined, it has no \
                  call frame around them"
    )]
    #[inline(always)]
    pub(super) fn is_match<'r>(
        &self,
        engine: impl FnOnce() -> &'r regex::Regex,
        haystack: &str,
    ) -> bool {
        match self.find(haystack.as_bytes(), 0, &mut 0) {
            Found::Needle => true,
            Found::None => false,
            Found::GiveWay(at) => engine().is_match_at(haystack, haystack.floor_char_boundary(at)),
        }
    }

    /// Whether `haystack` may hold the needle; `false` proves it cannot.
    fn may_match(&self, haystack: &str) -> bool {
        !matches!(self.find(haystack.as_bytes(), 0, &mut 0), Found::None)
    }
}

/// Where [`Needle::find`] stopped.
enum Found {
    /// The haystack holds the needle.
    Needle,
    /// The search gave way to the engine here: the needle does not begin
    /// before it.
    GiveWay(usize),
    /// The haystack does not hold the needle.
    None,
}

/// How [`CompiledPattern::is_match`](super::CompiledPattern::is_match) answers
/// for one pattern, read from the syntax the engine compiled.
#[derive(Debug, Clone)]
pub(super) enum Plan {
    /// The engine alone: no window every match contains, and no position next
    /// to an anchor.
    Engine,
    /// A window searched for before the engine runs.
    Window(Prefilter),
    /// Literal bytes, searched for without the engine.
    Literal(Needle),
    /// Positions checked at the anchored ends before the engine runs.
    Anchored(Affix),
}

impl Plan {
    /// The plan for `hir`, the syntax the engine compiled.
    pub(super) fn build(hir: &Hir) -> Self {
        if let Some(prefilter) = Prefilter::build(hir) {
            Needle::of(&prefilter, hir).map_or(Self::Window(prefilter), Self::Literal)
        } else if let Some(affix) = Affix::build(hir) {
            Self::Anchored(affix)
        } else {
            Self::Engine
        }
    }

    /// Whether the engine `engine` returns matches anywhere in `haystack`:
    /// exactly `engine().is_match(haystack)`. A literal asks for the engine
    /// only when its search gives way to it.
    pub(super) fn is_match<'r>(
        &self,
        engine: impl FnOnce() -> &'r regex::Regex,
        haystack: &str,
    ) -> bool {
        match self {
            Self::Engine => engine().is_match(haystack),
            Self::Window(prefilter) => prefilter.is_match(engine(), haystack),
            Self::Literal(needle) => needle.is_match(engine, haystack),
            Self::Anchored(affix) => affix.is_match(engine(), haystack),
        }
    }

    /// Whether the pattern may match in `haystack`; `false` proves it cannot.
    pub(super) fn may_match(&self, haystack: &str) -> bool {
        match self {
            Self::Engine => true,
            Self::Window(prefilter) => prefilter.may_match(haystack),
            Self::Literal(needle) => needle.may_match(haystack),
            Self::Anchored(affix) => affix.may_match(haystack),
        }
    }
}

/// Whether `a` and `b`, of equal length, hold the same bytes: in eight-byte
/// words, the last one overlapping the one before it, inline rather than a
/// call into the C library's `memcmp` for the few bytes a window or an
/// anchored end holds.
#[allow(
    clippy::inline_always,
    reason = "a verification of a few bytes: inlined, it has no call frame around the compare"
)]
#[inline(always)]
fn same_bytes(a: &[u8], b: &[u8]) -> bool {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len().min(b.len());
    if n < 8 {
        return a.iter().zip(b).all(|(x, y)| x == y);
    }
    let word = |s: &[u8], at: usize| {
        u64::from_ne_bytes(*s[at..].first_chunk::<8>().expect("eight bytes from here"))
    };
    (0..n / 8).all(|k| word(a, k * 8) == word(b, k * 8)) && word(a, n - 8) == word(b, n - 8)
}

/// The ASCII bytes `class` matches, when it matches ASCII only: one byte per
/// match, so one window position.
fn ascii_set(class: &Class) -> Option<ByteSet> {
    let mut set = ByteSet::EMPTY;
    match class {
        Class::Unicode(class) => {
            for range in class.ranges() {
                let (lo, hi) = (u32::from(range.start()), u32::from(range.end()));
                if hi > 0x7F {
                    return None;
                }
                for b in lo..=hi {
                    set.insert(u8::try_from(b).ok()?);
                }
            }
        }
        Class::Bytes(class) => {
            for range in class.ranges() {
                if range.end() > 0x7F {
                    return None;
                }
                for b in range.start()..=range.end() {
                    set.insert(b);
                }
            }
        }
    }
    (set != ByteSet::EMPTY).then_some(set)
}

/// The positions `hir` matches when it always matches the same number of
/// bytes, one from a known set at each: a literal, an ASCII class, an exact
/// repetition or a group of either.
fn positions(hir: &Hir) -> Option<Vec<ByteSet>> {
    match hir.kind() {
        HirKind::Literal(literal) => Some(literal.0.iter().copied().map(ByteSet::single).collect()),
        HirKind::Class(class) => ascii_set(class).map(|set| vec![set]),
        HirKind::Capture(capture) => positions(&capture.sub),
        HirKind::Repetition(repetition) if repetition.max == Some(repetition.min) => {
            let one = positions(&repetition.sub)?;
            let times = usize::try_from(repetition.min).ok()?;
            (one.len().saturating_mul(times) <= MAX_WINDOW).then(|| one.repeat(times))
        }
        HirKind::Concat(members) => {
            let mut all = Vec::new();
            for member in members {
                all.extend(positions(member)?);
                if all.len() > MAX_WINDOW {
                    return None;
                }
            }
            Some(all)
        }
        _ => None,
    }
}

/// Offer the windows of `hir`, which every match passes through, to `best`.
/// `reach` is what precedes `hir` in a match, `after` the most bytes a match
/// spans after it (`None` when unbounded).
fn collect(hir: &Hir, reach: &Reach, after: Option<usize>, best: &mut Option<Run>) {
    match hir.kind() {
        HirKind::Literal(_) | HirKind::Class(_) => {
            if let Some(sets) = positions(hir) {
                offer(sets, reach.clone(), after, best);
            }
        }
        HirKind::Capture(capture) => collect(&capture.sub, reach, after, best),
        HirKind::Repetition(repetition) if repetition.min >= 1 => {
            // The window is sought in the first iteration; the others follow it.
            let others = repetition.max.and_then(|max| {
                let one = repetition.sub.properties().maximum_len()?;
                usize::try_from(max - 1).ok()?.checked_mul(one)
            });
            let after = others.zip(after).map(|(a, b)| a.saturating_add(b));
            collect(&repetition.sub, reach, after, best);
        }
        HirKind::Concat(members) => {
            // `afters[i]`: the most bytes a match spans after member `i`.
            let mut afters = vec![after; members.len()];
            for i in (0..members.len().saturating_sub(1)).rev() {
                afters[i] = afters[i + 1]
                    .zip(members[i + 1].properties().maximum_len())
                    .map(|(a, b)| a.saturating_add(b));
            }
            let mut run: Vec<ByteSet> = Vec::new();
            let mut run_reach = reach.clone();
            let mut at = reach.clone();
            for (member, &member_after) in members.iter().zip(&afters) {
                if let Some(sets) = positions(member) {
                    if run.is_empty() {
                        run_reach.clone_from(&at);
                    }
                    run.extend(sets);
                } else {
                    // A run ending here is followed by this member and all
                    // that follows it.
                    let run_after = member
                        .properties()
                        .maximum_len()
                        .zip(member_after)
                        .map(|(a, b)| a.saturating_add(b));
                    offer(std::mem::take(&mut run), run_reach.clone(), run_after, best);
                    collect(member, &at, member_after, best);
                }
                at = extend(&at, member);
            }
            offer(run, run_reach, after, best);
        }
        _ => {}
    }
}

fn offer(mut sets: Vec<ByteSet>, reach: Reach, after: Option<usize>, best: &mut Option<Run>) {
    // Positions cut from the end of a long run still follow the window.
    let cut = sets.len().saturating_sub(MAX_WINDOW);
    sets.truncate(MAX_WINDOW);
    if !sets.iter().any(|set| set.probe().is_some()) {
        return;
    }
    let after = after.map(|after| after.saturating_add(cut));
    let run = Run { sets, reach, after };
    if best
        .as_ref()
        .is_none_or(|current| run.quality() > current.quality())
    {
        *best = Some(run);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CompiledPattern, compile};
    use super::{Affix, Plan, Prefilter};
    use purrdf_testkit::rng::SplitMix64;

    /// The window a compiled pattern searches for, if its plan has one.
    fn prefilter_of(pattern: &CompiledPattern) -> Option<Prefilter> {
        match pattern.plan() {
            Plan::Window(prefilter) => Some(prefilter.clone()),
            Plan::Literal(_) => {
                // The needle is the exact window of the same syntax.
                let hir = regex_syntax::ParserBuilder::new()
                    .build()
                    .parse(pattern.as_regex().as_str())
                    .expect("the engine's source parses");
                Prefilter::build(&hir)
            }
            Plan::Engine | Plan::Anchored(_) => None,
        }
    }

    /// The anchored-end checks of a compiled pattern, if its plan has them.
    fn affix_of(pattern: &CompiledPattern) -> Option<&Affix> {
        match pattern.plan() {
            Plan::Anchored(affix) => Some(affix),
            Plan::Engine | Plan::Window(_) | Plan::Literal(_) => None,
        }
    }

    /// The plan read from a parse of the engine's own source, as `compile`
    /// reads it for a pattern whose translation did not decide it.
    fn parsed_plan(pattern: &CompiledPattern, flags: &str) -> String {
        let quoted = flags.contains('q');
        let hir = regex_syntax::ParserBuilder::new()
            .case_insensitive(flags.contains('i'))
            .dot_matches_new_line(!quoted && flags.contains('s'))
            .multi_line(!quoted && flags.contains('m'))
            .build()
            .parse(pattern.as_regex().as_str())
            .expect("the engine's source parses");
        format!("{:?}", Plan::build(&hir))
    }

    /// The answers the prefilter must not change: `is_match` and
    /// `replace_all` against the engine alone.
    fn assert_agrees(pattern: &CompiledPattern, source: &str, flags: &str, haystack: &str) -> bool {
        let expected = pattern.as_regex().is_match(haystack);
        assert_eq!(
            pattern.is_match(haystack),
            expected,
            "is_match /{source}/{flags} on {haystack:?}"
        );
        let replaced = pattern
            .replace_all(haystack, "#")
            .expect("a plain replacement");
        let engine = pattern
            .as_regex()
            .replace_all(haystack, regex::NoExpand("#"));
        assert_eq!(
            replaced, engine,
            "replace_all /{source}/{flags} on {haystack:?}"
        );
        expected
    }

    fn pick<'a>(rng: &mut SplitMix64, items: &[&'a str]) -> &'a str {
        items[rng.below_usize(items.len())]
    }

    /// One random XSD pattern: literals (ASCII, two- and three-byte UTF-8, and
    /// letters whose case fold leaves ASCII), classes, `.`, `\d`, groups,
    /// alternations, every quantifier shape, and `^`/`$` anchors.
    fn random_pattern(rng: &mut SplitMix64, depth: usize) -> String {
        const ATOMS: &[&str] = &[
            "a",
            "b",
            "n",
            "e",
            "E",
            "-",
            "@",
            "k",
            "s",
            "\u{e9}",
            "\u{4e2d}",
            "\\.",
            "\\-",
            "[a-c]",
            "[nN]",
            "[e\u{e9}]",
            "[^a]",
            ".",
            "\\d",
            "\\s",
            "[0-9]",
            "ne",
            "need",
            "le",
            "ab",
            "@b\\.",
            "\u{212a}",
        ];
        const QUANTIFIERS: &[&str] = &["", "", "", "*", "+", "?", "{2}", "{1,3}", "{0,2}", "{3}"];
        let mut out = String::new();
        if rng.below_usize(6) == 0 {
            out.push('^');
        }
        for _ in 0..=rng.below_usize(5) {
            let atom = if depth < 2 && rng.below_usize(6) == 0 {
                let mut group = String::from("(");
                group.push_str(&random_pattern(rng, depth + 1));
                if rng.below_usize(3) == 0 {
                    group.push('|');
                    group.push_str(&random_pattern(rng, depth + 1));
                }
                group.push(')');
                group
            } else {
                pick(rng, ATOMS).to_owned()
            };
            out.push_str(&atom);
            out.push_str(pick(rng, QUANTIFIERS));
        }
        if rng.below_usize(6) == 0 {
            out.push('$');
        }
        out
    }

    /// One random haystack over the pattern alphabet plus newlines, the
    /// case-fold partners outside ASCII (U+212A KELVIN SIGN, U+017F LONG S) and
    /// runs of filler, at lengths that reach the scan's wide blocks.
    fn random_haystack(rng: &mut SplitMix64) -> String {
        const PIECES: &[&str] = &[
            "a",
            "b",
            "n",
            "N",
            "e",
            "E",
            "-",
            "@",
            ".",
            "k",
            "K",
            "s",
            "S",
            "\u{212a}",
            "\u{17f}",
            "\u{e9}",
            "\u{c9}",
            "\u{4e2d}",
            "0",
            "7",
            "\n",
            "\r",
            " ",
            "need",
            "needle",
            "NEEDLE",
            "le",
            "ab@b.",
            "2026-09",
            "xxxxxxxxxxxxxxxxxxxxxxxx",
        ];
        let pieces = match rng.below_usize(8) {
            0 => 0,
            1 => 200 + rng.below_usize(800),
            _ => rng.below_usize(24),
        };
        let mut out = String::new();
        for _ in 0..pieces {
            // Mostly filler, so windows are scattered rather than everywhere.
            if rng.below_usize(3) == 0 {
                out.push_str(pick(rng, PIECES));
            } else {
                out.push('x');
            }
        }
        out
    }

    /// The prefilter against the engine alone on randomized patterns, flags
    /// and haystacks. Every answer must agree, and the run must actually
    /// exercise the prefilter: patterns with a window, windows the engine is
    /// confined to, and both answers.
    #[test]
    fn prefiltered_answers_equal_the_engine_on_random_patterns() {
        const FLAGS: &[&str] = &["", "", "i", "s", "m", "q", "im", "is", "iq", "x"];
        let mut rng = SplitMix64::new(0x0047_2600_5EED_0001);
        let (mut patterns, mut filtered, mut tailed, mut exact, mut class_runs, mut cases) =
            (0, 0, 0, 0, 0, 0);
        let mut anchored = 0;
        let mut answers = [0_usize; 2];
        while patterns < 4000 {
            let source = random_pattern(&mut rng, 0);
            let flags = pick(&mut rng, FLAGS);
            let Ok(pattern) = compile(&source, flags) else {
                continue;
            };
            patterns += 1;
            // The plan the translation pass decided without a parse is the
            // plan the parsed syntax gives.
            assert_eq!(
                format!("{:?}", pattern.plan()),
                parsed_plan(&pattern, flags),
                "plan of /{source}/{flags}"
            );
            anchored += usize::from(affix_of(&pattern).is_some());
            if let Some(prefilter) = prefilter_of(&pattern) {
                filtered += 1;
                tailed += usize::from(prefilter.tail.is_some());
                exact += usize::from(prefilter.exact);
                class_runs += usize::from(prefilter.reach.as_ref().is_some_and(|reach| {
                    reach.iter().any(|step| matches!(step, super::Back::Run(_)))
                }));
            }
            for _ in 0..16 {
                let haystack = random_haystack(&mut rng);
                answers[usize::from(assert_agrees(&pattern, &source, flags, &haystack))] += 1;
                cases += 1;
            }
        }
        assert!(filtered > patterns / 4, "{filtered} of {patterns} filtered");
        assert!(
            anchored > patterns / 50,
            "{anchored} of {patterns} anchored"
        );
        assert!(tailed > filtered / 4, "{tailed} of {filtered} with a tail");
        assert!(exact > filtered / 20, "{exact} of {filtered} exact");
        assert!(
            class_runs > filtered / 100,
            "{class_runs} of {filtered} read a class run"
        );
        assert!(
            answers.iter().all(|&n| n > cases / 20),
            "{answers:?} of {cases}"
        );
    }

    /// The hand-picked neighbours of every rule the prefilter relies on.
    #[test]
    fn prefiltered_answers_equal_the_engine_on_the_edge_cases() {
        let cases: &[(&str, &str, &[&str])] = &[
            // Case folding: the window is the folded classes, and a fold that
            // leaves ASCII ends the window instead of being approximated.
            ("needle", "i", &["a NeEdLe", "needl", "NEEDLE"]),
            ("k", "i", &["\u{212a}", "K", "x"]),
            ("ks", "i", &["\u{212a}\u{17f}", "KS", "k"]),
            ("\u{e9}t\u{e9}", "i", &["\u{c9}T\u{c9}", "ete"]),
            // Empty matches: no window is required, so no prefilter.
            ("a*", "", &["", "b"]),
            ("(ab)?", "", &["", "x"]),
            // Anchors: `^` without `m` is left to the engine; under `m` the
            // text before the start offset is context.
            ("^ab", "", &["ab", "xab"]),
            ("^ab", "m", &["x\nab", "xab", "x\rab"]),
            ("ab$", "", &["ab", "ab\n", "abx"]),
            ("ab$", "m", &["ab\nx", "abx"]),
            // Reach: the engine starts where a match through the window can.
            (
                r"\d{4}-\d{2}",
                "",
                &["2026-09", "\u{0663}\u{0663}\u{0663}\u{0663}-12", "2026-9"],
            ),
            (
                r"[a-z]+@example\.org",
                "",
                &["me@example.org", "@example.org", "ME@example.org"],
            ),
            ("foo.*bar", "", &["foo bar", "foo\nbar", "barfoo"]),
            ("foo.*bar", "s", &["foo\nbar"]),
            // The `q` flag: the whole pattern is the window.
            ("a.c", "q", &["xa.cx", "abc"]),
        ];
        for &(source, flags, haystacks) in cases {
            let pattern = compile(source, flags).expect("an edge-case pattern compiles");
            for haystack in haystacks {
                assert_agrees(&pattern, source, flags, haystack);
                // The same window far into a long haystack, past the scan's
                // wide blocks, and preceded by windows that fail verification.
                let long = format!("{}{haystack}", "nx\u{e9}-@".repeat(300));
                assert_agrees(&pattern, source, flags, &long);
            }
        }
    }

    /// A haystack dense with windows that are not matches gives way to the
    /// engine rather than restarting it per window, and still answers exactly.
    #[test]
    fn dense_false_windows_give_way_to_the_engine() {
        let pattern = compile("ab[0-9]", "").expect("compiles");
        let dense = "ab".repeat(5000);
        assert!(!pattern.is_match(&dense));
        assert!(pattern.is_match(&format!("{dense}ab7")));
        let unbounded = compile("ab.*z", "").expect("compiles");
        assert!(!unbounded.is_match(&dense));
        assert!(unbounded.is_match(&format!("{dense}z")));
        // A literal needle among dense candidates that fail verification.
        let literal = compile("abcab", "").expect("compiles");
        assert!(matches!(literal.plan(), Plan::Literal(_)));
        let near = "abzab".repeat(2000);
        for (haystack, expected) in [
            (near.clone(), false),
            (format!("{near}abcab"), true),
            (format!("{near}\u{e9}abcab"), true),
            (format!("abcab{near}"), true),
            ("abca".to_owned(), false),
            (String::new(), false),
        ] {
            assert_agrees(&literal, "abcab", "", &haystack);
            assert_eq!(literal.is_match(&haystack), expected);
        }
    }

    /// Which patterns get a prefilter, where it searches, and what it knows
    /// about the rest of a match.
    #[test]
    fn windows_are_the_required_single_byte_runs() {
        use super::{Back, ByteSet};
        let window = |source: &str, flags: &str| {
            let pattern = compile(source, flags).expect("compiles");
            prefilter_of(&pattern).map(|p| {
                (
                    p.window.len(),
                    p.reach.as_ref().map(|reach| reach.to_vec()),
                    p.tail,
                    p.exact,
                )
            })
        };
        let none: Option<Vec<Back>> = Some(Vec::new());
        assert_eq!(window("needle", ""), Some((6, none.clone(), Some(6), true)));
        assert_eq!(
            window("needle", "i"),
            Some((6, none.clone(), Some(6), true))
        );
        assert_eq!(window("a.c", "q"), Some((3, none.clone(), Some(3), true)));
        // `k` under `i` also matches U+212A KELVIN SIGN, so it is no
        // single-byte position; `s` likewise matches U+017F LONG S.
        assert!(
            compile("k", "i")
                .expect("compiles")
                .as_regex()
                .is_match("\u{212a}")
        );
        assert_eq!(window("k", "i"), None);
        assert_eq!(window("s", "i"), None);
        assert_eq!(window("foo.*bar", ""), Some((3, none.clone(), None, false)));
        // Unicode `\d` is up to four bytes per digit: a match begins at most
        // 16 bytes before its `-` and ends at most 9 bytes from it.
        assert_eq!(
            window(r"\d{4}-\d{2}", ""),
            Some((1, Some(vec![Back::Bytes(16)]), Some(9), false))
        );
        // An ASCII class repeated exactly joins the window.
        assert_eq!(
            window("[0-9]{4}-[0-9]{2}", ""),
            Some((7, none.clone(), Some(7), true))
        );
        // An unbounded ASCII-class run before the window is read backward.
        let lower = {
            let mut set = ByteSet::EMPTY;
            for b in b'a'..=b'z' {
                set.insert(b);
            }
            set
        };
        assert_eq!(
            window(r"[a-z]+@example\.org", ""),
            Some((12, Some(vec![Back::Run(lower)]), Some(12), false))
        );
        // A class position is searched by range when it is narrow: the two
        // digit positions, not the letter between them.
        assert_eq!(
            window("[0-9][a-z][0-9]", ""),
            Some((3, none.clone(), Some(3), true))
        );
        let control = compile("[0-9][a-z][0-9]", "").expect("compiles");
        let probes = prefilter_of(&control).map(|p| (p.lead, p.trail));
        let digits = super::Probe::Runs([(b'0', b'9'); 2]);
        assert_eq!(probes, Some(((0, digits), Some((2, digits)))));
        // A position as wide as `[a-z]` is verified, never searched for.
        assert_eq!(window("[a-z]", ""), None);
        assert_eq!(window("^http://example", ""), None);
        // A one-byte alternation is a class; a longer one names no window.
        assert_eq!(window("a|b", "").map(|w| w.0), Some(1));
        assert_eq!(window("ab|cd", ""), None);
        assert_eq!(window("a*", ""), None);
        // Anchored at the end, the engine runs backward from the last byte;
        // under `m` the `$` is a line end, and the window stays.
        assert_eq!(window("x$", ""), None);
        assert_eq!(window("x$", "m"), Some((1, none, None, false)));
    }

    /// The anchored-end checks against the engine alone: patterns anchored at
    /// one end, the other or both, over haystacks short enough, and drawn from
    /// an alphabet small enough, that they often match.
    #[test]
    fn anchored_answers_equal_the_engine_on_random_patterns() {
        const FLAGS: &[&str] = &["", "", "", "i", "s", "m", "x", "is"];
        const PIECES: &[&str] = &[
            "a", "b", "n", "e", "E", "-", "@", ".", "k", "K", "\u{212a}", "s", "\u{17f}", "\u{e9}",
            "\u{c9}", "\u{4e2d}", "0", "7", "\n", "ne", "ab",
        ];
        let mut rng = SplitMix64::new(0x0047_2600_5EED_0002);
        let (mut patterns, mut anchored, mut decided, mut cases) = (0, 0, 0, 0);
        // Patterns whose plan the translation pass decided without a parse.
        let mut unparsed = 0;
        let mut answers = [0_usize; 2];
        while patterns < 3000 {
            let body = random_pattern(&mut rng, 0);
            let source = match rng.below_usize(3) {
                0 => format!("^{body}"),
                1 => format!("{body}$"),
                _ => format!("^{body}$"),
            };
            let flags = pick(&mut rng, FLAGS);
            let Ok(pattern) = compile(&source, flags) else {
                continue;
            };
            patterns += 1;
            unparsed += usize::from(pattern.plan.get().is_some());
            assert_eq!(
                format!("{:?}", pattern.plan()),
                parsed_plan(&pattern, flags),
                "plan of /{source}/{flags}"
            );
            if let Some(affix) = affix_of(&pattern) {
                anchored += 1;
                decided += usize::from(affix.rest != super::Rest::Engine);
            }
            for _ in 0..24 {
                let haystack: String = (0..rng.below_usize(7))
                    .map(|_| pick(&mut rng, PIECES))
                    .collect();
                answers[usize::from(assert_agrees(&pattern, &source, flags, &haystack))] += 1;
                cases += 1;
            }
        }
        assert!(anchored > patterns / 4, "{anchored} of {patterns} anchored");
        assert!(decided > anchored / 8, "{decided} of {anchored} decided");
        assert!(
            unparsed > patterns / 10,
            "{unparsed} of {patterns} planned without a parse"
        );
        assert!(
            answers.iter().all(|&n| n > cases / 50),
            "{answers:?} of {cases}"
        );
    }

    /// Which anchored patterns get end checks, and the neighbours of every
    /// rule they rely on.
    #[test]
    fn anchored_ends_are_checked_before_the_engine() {
        use super::Rest;
        let rest = |source: &str, flags: &str| {
            let pattern = compile(source, flags).expect("compiles");
            affix_of(&pattern).map(|affix| (affix.prefix.len(), affix.suffix.len(), affix.rest))
        };
        assert_eq!(
            rest(r"^http://example\.org/", ""),
            Some((19, 0, Rest::Matches))
        );
        assert_eq!(rest("[0-9]{3}$", ""), Some((0, 3, Rest::Matches)));
        assert_eq!(rest("^[a-z]", ""), Some((1, 0, Rest::Matches)));
        assert_eq!(rest("^ab$", ""), Some((2, 0, Rest::Whole)));
        assert_eq!(rest("^ab.*cd$", ""), Some((2, 2, Rest::Engine)));
        assert_eq!(rest("^[a-z]+$", ""), None);
        // Under `m` the anchors are line ends: no end checks.
        assert_eq!(rest("^ab", "m"), None);
        assert_eq!(rest("ab$", "m").map(|r| r.2), None);
        // `k` under `i` also matches U+212A KELVIN SIGN: the check stops
        // before it rather than approximating it.
        assert_eq!(rest("^ak", "i"), Some((1, 0, Rest::Engine)));
        // An alternation is anchored per branch, not as a whole.
        assert_eq!(rest("^ab|cd", ""), None);
        let cases: &[(&str, &str, &[&str])] = &[
            (
                r"^http://example\.org/",
                "",
                &[
                    "http://example.org/x",
                    "http://example.org",
                    "https://example.org/",
                    "",
                ],
            ),
            (
                "[0-9]{3}$",
                "",
                &["a123", "12", "123\n", "\u{0663}23", "1234"],
            ),
            ("^[a-z]", "", &["a", "", "A", "\u{e9}", "-a"]),
            ("^ab$", "", &["ab", "ab\n", "abc", "a", ""]),
            ("^ab.*cd$", "", &["abcd", "abxcd", "ab\ncd", "abc", "abd"]),
            ("^ab.*cd$", "s", &["ab\ncd"]),
            ("^ak", "i", &["A\u{212a}", "AK", "ax", "\u{212a}"]),
            (
                "^\u{e9}t\u{e9}$",
                "",
                &["\u{e9}t\u{e9}", "ete", "\u{e9}t\u{e9}x"],
            ),
            ("^\u{e9}t\u{e9}$", "i", &["\u{c9}T\u{c9}", "\u{e9}t\u{e9}"]),
            ("x\u{4e2d}$", "", &["ax\u{4e2d}", "x\u{4e2d}a", "\u{4e2d}"]),
            ("^ab", "m", &["x\nab", "ab", "xab"]),
            ("ab$", "m", &["ab\nx", "abx"]),
        ];
        for &(source, flags, haystacks) in cases {
            let pattern = compile(source, flags).expect("an edge-case pattern compiles");
            for haystack in haystacks {
                assert_agrees(&pattern, source, flags, haystack);
            }
        }
    }

    /// The backward bound over an unbounded class run starts the engine at the
    /// run, and still answers exactly when the run is broken or absent.
    #[test]
    fn class_runs_before_the_window_bound_the_start() {
        let pattern = compile(r"[a-z]+@example\.org", "").expect("compiles");
        let filler = "x".repeat(5000);
        for haystack in [
            format!("{filler} user@example.org"),
            format!("{filler} USER@example.org"),
            format!("{filler}@example.org"),
            format!("@example.org {filler}"),
            format!("{filler} a@example.org"),
        ] {
            assert_agrees(&pattern, r"[a-z]+@example\.org", "", &haystack);
        }
    }
}

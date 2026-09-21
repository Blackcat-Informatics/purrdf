// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The deterministic scale-corpus generator.
//!
//! Capacity claims need a corpus a single trick cannot flatter: a generator
//! that mints only `…/e/{n}` hands the dictionary a front-coder's dream and
//! proves nothing. Every IRI here is minted **purely from its index** under a
//! fixed seed, drawn from five deliberately adversarial classes — front-codable
//! plain, long zero-padded numerics beyond machine integer widths, raw-Han
//! Chinese, host-scattered irregular, and very-long — so the corpus exercises
//! the same surfaces real mixed data does. Index-pure minting is also what
//! makes generation **shardable**: shard `k` of `n` emits exactly its slice of
//! the quad sequence, and the concatenation of all shards is byte-identical to
//! a single whole run.
//!
//! Everything is arithmetic on a [`splitmix64`] stream — no RNG syscalls, no
//! platform floats, no iteration-order dependence — so output is
//! byte-deterministic across targets, pinned by a golden digest test. Each
//! decision a row makes (row kind, subject, object, predicate, literal
//! payload, named-graph membership, graph id, reifier id, blank-node label)
//! draws from its **own** stream tag, so no two decisions are correlated: a
//! consumer cannot find a regularity — "every literal is in the default
//! graph", say — that a real dataset would not have.
//!
//! **Term-kind coverage is a property of the profile, not an accident of it.**
//! [`ROW_MIX_PER_MILLE`] pins a share for every row kind the corpus emits, and
//! those kinds span the full RDF 1.2 term space this generator targets: IRI–IRI
//! edges, plain literals, language-tagged literals, datatyped literals, long
//! text literals, blank nodes in both subject and object position, and RDF 1.2
//! reifier rows binding a triple term. The statement layer is therefore **in**
//! the scale corpus, not out of it — a capacity claim measured here has been
//! measured against reifiers and triple terms too.
//!
//! [`ROW_MIX_PER_MILLE`] and [`CLASS_MIX_PER_MILLE`] are two **different axes**
//! and are never mixed: the row mix partitions the emitted *rows* by shape,
//! while the class mix partitions the *entity index space* by IRI shape. A row
//! of any kind may name an entity of any class, and because slots draw entity
//! indexes with a deliberate skew, the class shares measured over rows are not
//! the class shares pinned over the entity space. [`manifest`] names the two
//! keys accordingly (`entity_class_mix_per_mille`, `row_mix_per_mille`) so a
//! capture cannot read one as the other.
//!
//! This is corpus *generation* only. Output digests, ingest timings, and
//! capacity evidence are captured by the harnesses that consume it.
//!
//! ## The other deterministic generator
//!
//! PurRDF has two deterministic corpus generators and they own different
//! regimes. The keystone fixture generator (`purrdf_rdf::gts_fixtures`, driven
//! by `purrdf-envelope-probe`) is **small, fixed and statement-layer-focused**:
//! a handful of hand-shaped row groups whose job is to make the micro-hardware
//! envelope's ceilings meaningful on a constrained deployment. This profile is
//! the opposite regime — **shardable, anti-compressible and parameterized for
//! capacity at scale**, where the question is whether a dictionary trick could
//! have flattered a number rather than whether a small box still fits. Neither
//! substitutes for the other: a capacity claim measured over the keystone
//! fixture would be measured over a corpus designed to be small, and an
//! envelope measured over this profile would not be an envelope.

use core::fmt::Write as _;

/// A corpus specification: everything the output bytes depend on.
///
/// Two specs that compare equal generate byte-identical corpora on every
/// target, and [`manifest`] **records** exactly these fields. It computes no
/// digest of anything: digesting the emitted bytes is the capture's job, and a
/// capture is the manifest *plus* that digest.
///
/// The fields are private and only reachable through [`CorpusSpec::new`],
/// which is the sole home of the spec's invariants: `quads`, `iris`, and
/// `shards` are positive, and `shard < shards`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpusSpec {
    /// Seed folded into every derivation.
    seed: u64,
    /// Total quads across all shards.
    quads: u64,
    /// Entity **index space**: entity IRIs are minted from indexes `0..iris`.
    /// This is a target, never an achieved count — see [`CorpusSpec::iris`].
    iris: u64,
    /// This shard's zero-based index.
    shard: u64,
    /// Total shard count (`1` = whole corpus in one run).
    shards: u64,
}

/// Why a [`CorpusSpec::new`] call was rejected.
///
/// This is the one place the spec's invariants are named; every rejection
/// reason a caller can hit is a variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecError {
    /// `quads` was zero: a corpus must emit at least one row.
    ZeroQuads,
    /// `iris` was zero: every slot would draw entity index `0`, collapsing
    /// the corpus to a single subject instead of an entity space.
    ZeroIris,
    /// `shards` was zero: shard math divides by the shard count.
    ZeroShards,
    /// `shard` was not less than `shards`: this shard owns no slots.
    ShardOutOfRange {
        /// The requested shard index.
        shard: u64,
        /// The total shard count it must be less than.
        shards: u64,
    },
}

impl core::fmt::Display for SpecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroQuads => f.write_str("quads must be positive"),
            Self::ZeroIris => f.write_str("iris must be positive"),
            Self::ZeroShards => f.write_str("shards must be positive"),
            Self::ShardOutOfRange { shard, shards } => {
                write!(f, "shard {shard} must be less than shards {shards}")
            }
        }
    }
}

impl CorpusSpec {
    /// Builds a validated corpus specification.
    ///
    /// This is the only constructor and the one place the spec's invariants
    /// are enforced: every other method may assume a `CorpusSpec` in hand is
    /// valid.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] if `quads`, `iris`, or `shards` is zero, or if
    /// `shard` is not strictly less than `shards`.
    pub const fn new(
        seed: u64,
        quads: u64,
        iris: u64,
        shard: u64,
        shards: u64,
    ) -> Result<Self, SpecError> {
        if quads == 0 {
            return Err(SpecError::ZeroQuads);
        }
        if iris == 0 {
            return Err(SpecError::ZeroIris);
        }
        if shards == 0 {
            return Err(SpecError::ZeroShards);
        }
        if shard >= shards {
            return Err(SpecError::ShardOutOfRange { shard, shards });
        }
        Ok(Self {
            seed,
            quads,
            iris,
            shard,
            shards,
        })
    }

    /// Seed folded into every derivation.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Total quads across all shards.
    #[must_use]
    pub const fn quads(&self) -> u64 {
        self.quads
    }

    /// The entity **index space**: entity IRIs are minted from indexes
    /// `0..iris`.
    ///
    /// This is the size of the space rows draw from — a **target**, not an
    /// achieved distinct-entity count. Slots draw entities with a deliberate
    /// `sqrt`-CDF skew and a shard emits only `end - start` rows, so the
    /// distinct entities a run actually names is at most
    /// `min(iris, emitted rows)` and in practice well below both. A capacity
    /// claim must say which of the two numbers it is about: this one (the
    /// space that was available) or a distinct count (which only enumerating
    /// the output can establish, and which this crate deliberately never
    /// computes — enumerating a corpus defeats streaming it at full scale).
    #[must_use]
    pub const fn iris(&self) -> u64 {
        self.iris
    }

    /// This shard's zero-based index.
    #[must_use]
    pub const fn shard(&self) -> u64 {
        self.shard
    }

    /// Total shard count (`1` = whole corpus in one run).
    #[must_use]
    pub const fn shards(&self) -> u64 {
        self.shards
    }

    /// The half-open quad-index range this shard emits.
    ///
    /// Shards partition `0..quads` contiguously; the last shard absorbs the
    /// remainder, and every boundary is a pure function of the spec.
    /// Infallible by construction: [`CorpusSpec::new`] guarantees
    /// `shards > 0` and `shard < shards`, so this can never divide by zero
    /// or observe `shard >= shards`.
    #[must_use]
    pub const fn shard_range(&self) -> (u64, u64) {
        let base = self.quads / self.shards;
        let start = self.shard * base;
        let end = if self.shard + 1 == self.shards {
            self.quads
        } else {
            start + base
        };
        (start, end)
    }
}

/// The stable identity of the default mixes. Any change to a mix, the class
/// shapes, or the row derivation **after publication** is a new profile id —
/// the manifest carries it so a capture names exactly what generated its
/// bytes.
///
/// `…-v1` is *defined* here and has never been published: its definition was
/// corrected (the skew map's 128-bit fixed point, the reserved-octet escapes,
/// the explicit row mix, and the independent stream tags) before any capture
/// could name it, and the golden digest was re-pinned once to the corrected
/// definition. Fixing a definition that no capture has ever referenced is not
/// a silent regeneration; from the first published capture onward, the rule
/// above binds absolutely.
pub const CORPUS_PROFILE_ID: &str = "purrdf-scale-mixed-v1";

/// The five IRI classes and their per-mille shares of the **entity space**.
/// The shares are the anti-compressibility contract: no single dictionary
/// trick can carry the whole corpus.
///
/// This is a different axis from [`ROW_MIX_PER_MILLE`]: this table decides
/// what an entity IRI *looks like*, that one decides what a row *is*. A row of
/// any kind may name an entity of any class.
///
/// **The shares are of the entity INDEX space, not of the emitted rows.**
/// `class_of` is a function of the entity index alone, and slots draw entity
/// indexes with a deliberate `sqrt`-CDF skew, so the class distribution
/// *measured over rows* is a different number from the one pinned here. The
/// manifest therefore reports this table under the key
/// `entity_class_mix_per_mille`, so a capture that records it cannot be read
/// as a row-level measurement; the row-level axis is
/// [`ROW_MIX_PER_MILLE`] / `row_mix_per_mille`.
pub const CLASS_MIX_PER_MILLE: [(&str, u16); 5] = [
    ("plain", 400),
    ("numeric-long", 200),
    ("chinese", 200),
    ("irregular", 150),
    ("very-long", 50),
];

/// Row kinds and their per-mille shares of the emitted **rows**. This table is
/// the contract: every row kind has a pinned share, so no share can drift
/// silently, and every kind is non-empty by construction rather than by luck.
///
/// This is a different axis from [`CLASS_MIX_PER_MILLE`]: that table decides
/// what an entity IRI *looks like*, this one decides what a row *is*.
///
/// One slot emits exactly one line for every kind — including the reified and
/// blank-node kinds — so shard stitching stays byte-exact and the line-count
/// invariant (`lines == quads`) holds unchanged.
pub const ROW_MIX_PER_MILLE: [(&str, u16); 7] = [
    ("entity-edge", 500),
    ("plain-literal", 150),
    ("zh-literal", 100),
    ("typed-literal", 100),
    ("long-text-literal", 50),
    ("reified", 60),
    ("blank-node", 40),
];

/// Sums a per-mille table at compile time. The accumulator is `u16`, so a
/// table whose shares overflow 65 535 is a const-evaluation error rather than
/// a wrapped total that could still compare equal to 1000.
const fn per_mille_total(table: &[(&str, u16)]) -> u16 {
    let mut total = 0u16;
    let mut index = 0;
    while index < table.len() {
        total += table[index].1;
        index += 1;
    }
    total
}

// Both mixes must sum to exactly 1000, checked at COMPILE time. A mix that
// sums to 900 used to dump the missing 10% into the table's last entry with
// every test still green; that is now a build failure.
const _: () = assert!(
    per_mille_total(&CLASS_MIX_PER_MILLE) == 1_000,
    "CLASS_MIX_PER_MILLE must sum to exactly 1000 per mille"
);
const _: () = assert!(
    per_mille_total(&ROW_MIX_PER_MILLE) == 1_000,
    "ROW_MIX_PER_MILLE must sum to exactly 1000 per mille"
);

/// The shape of one emitted row, in bijection with [`ROW_MIX_PER_MILLE`]'s
/// positions. Dispatch is over this enum, so the compiler — not a trailing
/// `_` arm — proves every kind is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RowKind {
    /// `<s> <p> <o>` — an IRI-to-IRI edge.
    EntityEdge,
    /// `<s> <p> "value N"` — a plain (untyped, untagged) literal.
    PlainLiteral,
    /// `<s> <p> "漢字…"@zh` — a language-tagged literal.
    ZhLiteral,
    /// `<s> <p> "lex"^^<xsd:…>` — a datatyped literal with a lexical form
    /// valid for its datatype.
    TypedLiteral,
    /// `<s> <p> "text …"` — a long plain literal (arena and framing stress).
    LongTextLiteral,
    /// `<r> <rdf:reifies> <<( <s> <p> <o> )>>` — an RDF 1.2 reifier row
    /// binding a triple term.
    Reified,
    /// A row with a blank node in subject or object position.
    BlankNode,
}

impl RowKind {
    /// The kind at table position `position`.
    ///
    /// Exhaustive over [`ROW_MIX_PER_MILLE`]'s positions; anything past the
    /// end is a bug in the table or in the roll, never a silently absorbed
    /// share. The identity `from_position(k as usize) == k` is asserted at
    /// compile time below, so the enum order can never drift from the table.
    const fn from_position(position: usize) -> Self {
        match position {
            0 => Self::EntityEdge,
            1 => Self::PlainLiteral,
            2 => Self::ZhLiteral,
            3 => Self::TypedLiteral,
            4 => Self::LongTextLiteral,
            5 => Self::Reified,
            6 => Self::BlankNode,
            // Reached only if ROW_MIX_PER_MILLE grew a row this enum has no
            // variant for. The const bijection check below turns that into a
            // BUILD failure, so it can never be a runtime surprise.
            _ => panic!(
                "row-kind position is past ROW_MIX_PER_MILLE: add the matching RowKind variant"
            ),
        }
    }

    /// This kind's name, read from [`ROW_MIX_PER_MILLE`] so the table stays
    /// the single source of truth for both names and shares.
    #[must_use]
    pub const fn name(self) -> &'static str {
        ROW_MIX_PER_MILLE[self as usize].0
    }

    /// This kind's pinned per-mille share, read from [`ROW_MIX_PER_MILLE`].
    #[must_use]
    pub const fn share_per_mille(self) -> u16 {
        ROW_MIX_PER_MILLE[self as usize].1
    }
}

// The enum and the table must stay in bijection. If a row kind is added to the
// table without a matching variant (or the order drifts), this fails the
// build: `from_position` either panics past the end or returns a variant whose
// discriminant is not its own position.
const _: () = {
    let mut position = 0;
    while position < ROW_MIX_PER_MILLE.len() {
        assert!(
            RowKind::from_position(position) as usize == position,
            "RowKind must be in bijection with ROW_MIX_PER_MILLE's positions"
        );
        position += 1;
    }
};

/// Independent `splitmix64` stream tags, one per decision a row makes.
///
/// Sharing one draw across decisions manufactures a regularity that a real
/// dataset would not have: deriving graph membership from `o % 6` and object
/// kind from `o % 3` made `o ≡ 0 (mod 6)` imply `o ≡ 0 (mod 3)`, so named
/// graphs held literals at a third the corpus-wide rate. Every decision below
/// gets its own tag, so every pair is independent.
mod tags {
    /// Which [`super::CLASS_MIX_PER_MILLE`] class an entity index belongs to.
    pub(super) const CLASS: u64 = 0xC1A5_5000;
    /// The per-entity hash that fills in an entity IRI's shape.
    pub(super) const ENTITY_SHAPE: u64 = 0x1121_1121;
    /// The row's subject entity index.
    pub(super) const SUBJECT: u64 = 0x5AB1_0001;
    /// The row's object entity index.
    pub(super) const OBJECT: u64 = 0x0B1F_0002;
    /// Which predicate the row uses.
    pub(super) const PREDICATE: u64 = 0x9AED_0003;
    /// Which [`super::ROW_MIX_PER_MILLE`] kind the row is.
    pub(super) const ROW_KIND: u64 = 0x0B1E_0004;
    /// The literal payload (lexical text, datatype selection).
    pub(super) const LITERAL: u64 = 0x117E_0005;
    /// Whether the row sits in a named graph.
    pub(super) const GRAPH_MEMBER: u64 = 0x6EA9_0006;
    /// Which named graph, when it does.
    pub(super) const GRAPH_ID: u64 = 0x6EA9_0007;
    /// The reifier IRI's index, for a reified row.
    pub(super) const REIFIER: u64 = 0x2E1F_0008;
    /// The blank-node label and its position family.
    pub(super) const BLANK: u64 = 0xB1A4_0009;
}

/// `splitmix64` — the classic public-domain mixing step: deterministic,
/// allocation-free, and identical on every target.
#[must_use]
pub const fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Mixes the seed with a stream tag and an index into one draw.
const fn draw(seed: u64, tag: u64, index: u64) -> u64 {
    splitmix64(seed ^ splitmix64(tag ^ splitmix64(index)))
}

/// Resolves a per-mille `roll` (`0..1000`) to a position in `table`.
///
/// Falling past the end is impossible for a table that sums to 1000 — both
/// tables are asserted to, at compile time — so it is a bug, not a share to
/// absorb into the last entry. The old code returned `table.len() - 1` there,
/// which is exactly how a mix summing to 900 could quietly hand its missing
/// 10% to `very-long` with every test green.
fn pick(table: &[(&str, u16)], roll: u64) -> usize {
    let mut acc = 0u64;
    for (position, (_, share)) in table.iter().enumerate() {
        acc += u64::from(*share);
        if roll < acc {
            return position;
        }
    }
    unreachable!("per-mille roll {roll} fell past a table summing to only {acc}")
}

/// Which class an entity index belongs to (pure function of the index).
fn class_of(seed: u64, entity: u64) -> usize {
    pick(
        &CLASS_MIX_PER_MILLE,
        draw(seed, tags::CLASS, entity) % 1_000,
    )
}

/// Which kind of row slot `slot` emits (pure function of the slot).
fn row_kind_of(seed: u64, slot: u64) -> RowKind {
    RowKind::from_position(pick(
        &ROW_MIX_PER_MILLE,
        draw(seed, tags::ROW_KIND, slot) % 1_000,
    ))
}

/// RFC 3986 §2.2 **reserved** octets, the only bytes whose percent-escape a
/// conformant normalizer must preserve.
///
/// The irregular class used to escape `A`-`Z`, which are RFC 3986 §2.3
/// **unreserved**. Percent-encoding an unreserved character is
/// normalization-equivalent to writing it literally (RFC 3986 §6.2.2.2), so a
/// conformant normalizer decodes `%41` straight back to `A` and the class's
/// stated purpose — an escaped tail that defeats suffix tricks — evaporated
/// before the first comparison. A reserved octet's escape has a different
/// meaning from the raw octet, so it must survive verbatim and the stress is
/// real.
/// Spelled as a byte string (`/ ? # [ ] @ ! $ & + ; =`) because
/// `clippy::byte_char_slices` rejects the twelve-`b'…'` array form.
pub const RESERVED_OCTETS: [u8; 12] = *b"/?#[]@!$&+;=";

/// Selects a reserved octet from a draw, indexed by the table's own length so
/// the modulus can never drift from the table.
fn reserved_octet(value: u64) -> u8 {
    let index = value % RESERVED_OCTETS.len() as u64;
    RESERVED_OCTETS[usize::try_from(index).expect(
        "index is a remainder modulo RESERVED_OCTETS.len() (12), so it is below 12 and fits \
         usize on every supported target",
    )]
}

/// The first code point of the CJK Unified Ideographs span the raw-Han shapes
/// draw from.
const HAN_BASE: u32 = 0x4E00;

/// The width of that span: `0x4E00..=0x9FA4`. Every code point in it is a
/// Unicode scalar value — the block ends far below the `0xD800..=0xDFFF`
/// surrogate range — so no draw can land on a non-character.
const HAN_SPAN: u64 = 0x51A5;

/// Maps a draw onto one Han character from `0x4E00..=0x9FA4`.
///
/// Both narrowings below are infallible by construction, and they are spelled
/// as `expect` rather than as a fallback on purpose: a fallback would answer a
/// future derivation change by **emitting a different corpus byte**, in a
/// generator whose entire value is that its bytes are pinned.
fn han_char(value: u64) -> char {
    let offset = u32::try_from(value % HAN_SPAN)
        .expect("a remainder modulo HAN_SPAN (0x51A5) is below 0x51A5 and fits u32");
    char::from_u32(HAN_BASE + offset).expect(
        "HAN_BASE + offset lies in 0x4E00..=0x9FA4, every value of which is a Unicode scalar \
         value (the span ends well below the 0xD800 surrogate range)",
    )
}

/// Appends entity IRI `entity`'s text (without angle brackets) to `out`.
///
/// Minting is index-pure: the same `(seed, entity)` always yields the same
/// IRI on every shard and every target.
pub fn write_entity_iri(out: &mut String, seed: u64, entity: u64) {
    let h = draw(seed, tags::ENTITY_SHAPE, entity);
    match class_of(seed, entity) {
        // Front-codable: the dictionary-friendly floor of the mix.
        0 => {
            let _ = write!(out, "https://example.org/e/{entity}");
        }
        // Long zero-padded numerics: 36 digits, beyond u64 and 2^53, with
        // leading zeros that MUST survive verbatim (identity, not number).
        1 => {
            let _ = write!(
                out,
                "https://example.org/n/{:018}{:018}",
                h % 1_000_000_000_000_000_000,
                entity % 1_000_000_000_000_000_000
            );
        }
        // Raw-Han path segments: IRIs carry non-ASCII directly (RFC 3987
        // `ucschar`), which N-Quads IRIREF admits raw — no percent-encoding.
        2 => {
            out.push_str("https://example.org/中文/");
            let mut v = h | 1;
            for _ in 0..4 {
                out.push(han_char(v));
                v = splitmix64(v);
            }
            let _ = write!(out, "/{entity}");
        }
        // Host-scattered irregular: a hashed subdomain defeats host-prefix
        // sharing, and the genuinely mixed-case percent-escaped tail defeats
        // suffix tricks. Two escapes, one uppercase-hex and one lowercase-hex,
        // over RFC 3986 RESERVED octets — see RESERVED_OCTETS for why an
        // unreserved octet would have been no stress at all.
        3 => {
            let _ = write!(
                out,
                "https://s{:04x}.example.org/x/%{:02X}%{:02x}/K{}~{:x}",
                h & 0xFFFF,
                reserved_octet(h >> 16),
                reserved_octet(h >> 24),
                entity,
                h
            );
        }
        // Very long: ~628 bytes of index-derived segments (measured, not
        // designed to a round number); length itself is the stress (arena
        // growth, bucket boundaries, wire framing).
        _ => {
            out.push_str("https://example.org/long");
            let mut v = h | 1;
            for _ in 0..30 {
                let _ = write!(out, "/seg{v:016x}");
                v = splitmix64(v);
            }
            let _ = write!(out, "/{entity}");
        }
    }
}

/// Skew mapping: quad slots draw entities with a hot head and a long tail
/// (integer approximation of a power-law; exact, no floats).
///
/// The draw is squared in **128-bit** fixed point, over its full 64 bits. The
/// previous 64-bit form did two things wrong at once. It computed
/// `(biased * iris) >> 32`, which overflows `u64` for `iris > 2^32 + 2` — and
/// `[profile.dev]` sets `overflow-checks = true`, so every debug and test
/// build panicked there while release wrapped silently. And it funnelled the
/// draw through `r >> 32`, so `biased` had at most `2^32` values and the map
/// had at most `2^32` **distinct outputs** no matter how large `iris` was: ten
/// billion distinct IRIs were unmintable. (The output *range* was not capped —
/// with the overflow removed the old formula's maximum still reached `iris` —
/// which is why a max-value check cannot detect the funnel and a
/// distinct-count check can.)
///
/// The skew SHAPE is unchanged: the empirical CDF still tracks
/// `sqrt(k / iris)` (measured 0.1009 / 0.2006 / 0.5011 / 0.7077 against
/// 0.1 / 0.2 / 0.5 / 0.7071).
fn skewed_entity(seed: u64, tag: u64, slot: u64, iris: u64) -> u64 {
    let r = draw(seed, tag, slot);
    // Square the unit draw in 128-bit fixed point: u^2 biases toward 0.
    let wide = u128::from(r);
    let squared = (wide * wide) >> 64;
    let scaled = (squared * u128::from(iris)) >> 64;
    // The narrowing cannot fail, in exact arithmetic: the largest possible
    // `r` is `2^64 - 1`, whose square shifted down 64 bits is `2^64 - 2`, so
    // `scaled <= ((2^64 - 2) * iris) >> 64 < iris <= u64::MAX` for every
    // positive `iris` — and `CorpusSpec::new` rejects `iris == 0`.
    scaled as u64
}

/// The predicate vocabulary (small and fixed, as real datasets have).
const PREDICATES: [&str; 8] = [
    "https://example.org/p/rel",
    "https://example.org/p/name",
    "https://example.org/p/type",
    "https://example.org/p/part",
    "https://example.org/p/near",
    "https://example.org/p/note",
    "https://example.org/p/标签",
    "https://example.org/p/seen",
];

/// The row's predicate, indexed by [`PREDICATES`]'s own length so the modulus
/// can never drift from the table.
fn predicate_of(seed: u64, slot: u64) -> &'static str {
    let index = draw(seed, tags::PREDICATE, slot) % PREDICATES.len() as u64;
    PREDICATES[usize::try_from(index).expect(
        "index is a remainder modulo PREDICATES.len() (8), so it is below 8 and fits usize on \
         every supported target",
    )]
}

/// The RDF 1.2 reification predicate. W3C standard vocabulary: using it is
/// what the spec requires, not a vocabulary this project mints. Every *data*
/// IRI in the corpus stays under `example.org`.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// The XML Schema datatype namespace — again W3C standard vocabulary, used as
/// the specs require.
const XSD_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema#";

/// The datatypes typed-literal rows cycle over, as local names under
/// [`XSD_NAMESPACE`]. Positions here are the arms of [`write_typed_literal`].
const XSD_DATATYPES: [&str; 4] = ["integer", "decimal", "date", "boolean"];

/// Appends a typed literal — `"lexical"^^<datatype>` — to `out`.
///
/// Every lexical form is **valid for its datatype**, because the strict reader
/// is entitled to reject one that is not: integers are plain digits with an
/// optional leading `-`; decimals carry exactly one `.`; dates are
/// `YYYY-MM-DD` with month `1..=12` and day `1..=28`, so no day can overflow
/// its month and no leap-year rule can ever apply; booleans are exactly `true`
/// or `false`.
fn write_typed_literal(out: &mut String, payload: u64) {
    let index = usize::try_from(payload % XSD_DATATYPES.len() as u64).expect(
        "index is a remainder modulo XSD_DATATYPES.len() (4), so it is below 4 and fits usize \
         on every supported target",
    );
    let value = payload >> 8;
    match index {
        // xsd:integer
        0 => {
            let sign = if value & 1 == 1 { "-" } else { "" };
            let _ = write!(out, "\"{sign}{}\"", (value >> 1) % 1_000_000_000_000);
        }
        // xsd:decimal
        1 => {
            let _ = write!(
                out,
                "\"{}.{:06}\"",
                value % 1_000_000,
                (value >> 20) % 1_000_000
            );
        }
        // xsd:date
        2 => {
            let _ = write!(
                out,
                "\"{:04}-{:02}-{:02}\"",
                1_000 + value % 9_000,
                1 + (value >> 14) % 12,
                1 + (value >> 20) % 28
            );
        }
        // xsd:boolean
        3 => out.push_str(if value & 1 == 1 {
            "\"true\""
        } else {
            "\"false\""
        }),
        other => unreachable!(
            "typed-literal index {other} is past XSD_DATATYPES (len {})",
            XSD_DATATYPES.len()
        ),
    }
    let _ = write!(out, "^^<{XSD_NAMESPACE}{}>", XSD_DATATYPES[index]);
}

/// Appends `<subject> <predicate> ` — the prefix every non-reified,
/// non-blank-subject row shares.
fn write_subject_predicate(out: &mut String, seed: u64, subject: u64, predicate: &str) {
    out.push('<');
    write_entity_iri(out, seed, subject);
    out.push_str("> <");
    out.push_str(predicate);
    out.push_str("> ");
}

/// Appends a `@zh` language-tagged literal of three Han characters.
fn write_zh_literal(out: &mut String, payload: u64) {
    out.push('"');
    let mut v = payload | 1;
    for _ in 0..3 {
        out.push(han_char(v));
        v = splitmix64(v);
    }
    out.push_str("\"@zh");
}

/// How many named graphs the corpus spreads over — enough graph spread to
/// exercise the quad position without dominating the dictionary.
const NAMED_GRAPHS: u64 = 16;

/// One row in `NAMED_GRAPH_SHARE` sits in a named graph; the rest are in the
/// default graph. Drawn from its **own** stream tag, so graph membership is
/// independent of the row's kind.
const NAMED_GRAPH_SHARE: u64 = 6;

/// Appends quad `slot`'s N-Quads row (with trailing newline) to `out`.
///
/// **One slot emits exactly one line, for every row kind**, so
/// `lines == quads` holds unconditionally and shard concatenation stays
/// byte-exact.
///
/// The row's shape is drawn from [`ROW_MIX_PER_MILLE`] and dispatched over
/// [`RowKind`], so the compiler proves every kind is written. One sixth of
/// rows land in one of 16 named graphs (an internal spread, not a manifest
/// field — see `NAMED_GRAPHS`); that share is drawn from its own stream tag,
/// independent of the row kind, so (for example) literals appear inside named
/// graphs at the same rate as they do corpus-wide.
pub fn write_row(out: &mut String, spec: &CorpusSpec, slot: u64) {
    let seed = spec.seed;
    let kind = row_kind_of(seed, slot);
    let subject = skewed_entity(seed, tags::SUBJECT, slot, spec.iris);
    let object = skewed_entity(seed, tags::OBJECT, slot, spec.iris);
    let predicate = predicate_of(seed, slot);
    let payload = draw(seed, tags::LITERAL, slot);

    match kind {
        RowKind::EntityEdge => {
            write_subject_predicate(out, seed, subject, predicate);
            out.push('<');
            write_entity_iri(out, seed, object);
            out.push('>');
        }
        RowKind::PlainLiteral => {
            write_subject_predicate(out, seed, subject, predicate);
            let _ = write!(out, "\"value {}\"", payload >> 8);
        }
        RowKind::ZhLiteral => {
            write_subject_predicate(out, seed, subject, predicate);
            write_zh_literal(out, payload);
        }
        RowKind::TypedLiteral => {
            write_subject_predicate(out, seed, subject, predicate);
            write_typed_literal(out, payload);
        }
        RowKind::LongTextLiteral => {
            write_subject_predicate(out, seed, subject, predicate);
            let _ = write!(
                out,
                "\"text {:064x} {:064x}\"",
                payload,
                splitmix64(payload)
            );
        }
        // RDF 1.2: a reifier IRI bound to a triple term. The reifier lives in
        // its own `/r/` path segment, disjoint from every entity-IRI shape, so
        // no ordinary row can accidentally annotate one.
        RowKind::Reified => {
            let _ = write!(
                out,
                "<https://example.org/r/{}> <{RDF_REIFIES}> <<( ",
                draw(seed, tags::REIFIER, slot)
            );
            out.push('<');
            write_entity_iri(out, seed, subject);
            out.push_str("> <");
            out.push_str(predicate);
            out.push_str("> <");
            write_entity_iri(out, seed, object);
            out.push_str("> )>>");
        }
        // Two label families, one putting the blank node in SUBJECT position
        // and one in OBJECT position, so both parser paths are exercised. The
        // labels satisfy the N-Quads `BLANK_NODE_LABEL` production and are
        // derived purely from the slot, so shard concatenation stays
        // byte-identical to a whole run.
        RowKind::BlankNode => {
            let label = draw(seed, tags::BLANK, slot);
            if label & 1 == 0 {
                let _ = write!(out, "_:bs{} <{predicate}> <", label >> 1);
                write_entity_iri(out, seed, object);
                out.push('>');
            } else {
                out.push('<');
                write_entity_iri(out, seed, subject);
                let _ = write!(out, "> <{predicate}> _:bo{}", label >> 1);
            }
        }
    }

    if draw(seed, tags::GRAPH_MEMBER, slot).is_multiple_of(NAMED_GRAPH_SHARE) {
        let _ = write!(
            out,
            " <https://example.org/g/{}>",
            draw(seed, tags::GRAPH_ID, slot) % NAMED_GRAPHS
        );
    }
    out.push_str(" .\n");
}

/// Appends a per-mille `table` as a JSON object body (no surrounding braces).
fn write_per_mille_object(out: &mut String, table: &[(&str, u16)]) {
    for (position, (name, share)) in table.iter().enumerate() {
        let comma = if position + 1 == table.len() {
            ""
        } else {
            ", "
        };
        let _ = write!(out, "\"{name}\": {share}{comma}");
    }
}

/// Appends [`ROW_MIX_PER_MILLE`] as a per-mille JSON object body (no
/// surrounding braces), sourced through [`RowKind::name`] and
/// [`RowKind::share_per_mille`] rather than reading the table directly, so
/// the manifest and the enum share one path and cannot drift apart. The
/// values written are identical to [`write_per_mille_object`] over
/// [`ROW_MIX_PER_MILLE`] — [`RowKind::from_position`] and the accessors are
/// all plain reads of that same table.
fn write_row_mix_object(out: &mut String) {
    for position in 0..ROW_MIX_PER_MILLE.len() {
        let kind = RowKind::from_position(position);
        let comma = if position + 1 == ROW_MIX_PER_MILLE.len() {
            ""
        } else {
            ", "
        };
        let _ = write!(
            out,
            "\"{}\": {}{comma}",
            kind.name(),
            kind.share_per_mille()
        );
    }
}

/// The manifest: everything a capture needs to name this corpus exactly.
///
/// **It records; it does not digest.** No field here is a hash of anything,
/// and nothing in this crate hashes the output: a capture is this manifest
/// *plus* a digest of the bytes that were actually consumed, and neither half
/// is evidence on its own.
///
/// Each key, and exactly what it is a statement about:
///
/// * `profile` — [`CORPUS_PROFILE_ID`], the identity of the derivation.
/// * `seed`, `quads`, `shard`, `shards` — the spec's parameters, echoed.
/// * `iris` — the entity **index space**, a TARGET (see [`CorpusSpec::iris`]).
///   It is not an achieved distinct-entity count, and no field here is: the
///   achieved count is at most `min(iris, emitted_lines)` and in practice far
///   below it, and establishing it would mean enumerating the corpus, which is
///   exactly what streaming at full scale exists to avoid. A capacity claim
///   must name which of the two numbers it uses.
/// * `shard_rows` — this shard's half-open slot range, `[start, end)`.
/// * `emitted_lines` — `end - start`, the exact number of lines this shard
///   emits (one slot emits exactly one line, for every row kind). Free and
///   exact, because it is arithmetic on the range rather than a count of
///   output.
/// * `entity_class_mix_per_mille` — [`CLASS_MIX_PER_MILLE`], over the **entity
///   index space**. The key is named for its basis on purpose: `class_of` is a
///   function of the entity index, slots draw indexes under a skew, and so the
///   class distribution over emitted ROWS is a different number.
/// * `row_mix_per_mille` — [`ROW_MIX_PER_MILLE`], over the **emitted rows**.
///
/// Both mixes are reported, because they are independent axes: a capture that
/// recorded only the class mix would not name the row shapes its bytes
/// actually contain.
#[must_use]
pub fn manifest(spec: &CorpusSpec) -> String {
    let (start, end) = spec.shard_range();
    let emitted_lines = end - start;
    let mut m = String::new();
    let _ = write!(
        m,
        "{{\"profile\": \"{CORPUS_PROFILE_ID}\", \"seed\": {}, \"quads\": {}, \"iris\": {}, \
         \"shard\": {}, \"shards\": {}, \"shard_rows\": [{start}, {end}], \
         \"emitted_lines\": {emitted_lines}, \"entity_class_mix_per_mille\": {{",
        spec.seed, spec.quads, spec.iris, spec.shard, spec.shards
    );
    write_per_mille_object(&mut m, &CLASS_MIX_PER_MILLE);
    m.push_str("}, \"row_mix_per_mille\": {");
    write_row_mix_object(&mut m);
    m.push_str("}}\n");
    m
}

#[cfg(test)]
mod tests {
    use super::{
        CLASS_MIX_PER_MILLE, CORPUS_PROFILE_ID, CorpusSpec, RESERVED_OCTETS, ROW_MIX_PER_MILLE,
        RowKind, SpecError, XSD_DATATYPES, XSD_NAMESPACE, class_of, row_kind_of, skewed_entity,
        tags, write_entity_iri, write_row,
    };

    const SEED: u64 = 0x5EED_CAFE;

    /// Sample size for every measured-share test.
    ///
    /// The rarest row kind (`blank-node`, 40 per mille) draws ~8 000 of these
    /// slots; a binomial with `p = 0.04, n = 200_000` has a standard deviation
    /// of 0.44 per mille, so [`SHARE_TOLERANCE_PER_MILLE`] is over eleven
    /// sigma wide. The tolerance exists to absorb sampling noise, not to
    /// absorb a drifted table — a share that is wrong by a whole percentage
    /// point is more than twenty sigma out and cannot hide inside it.
    const SHARE_SAMPLE: u64 = 200_000;

    /// Tolerance, in per mille, for a measured share against its pinned share.
    const SHARE_TOLERANCE_PER_MILLE: u64 = 5;

    fn spec() -> CorpusSpec {
        CorpusSpec::new(SEED, 2_000, 1_000, 0, 1).expect("fixture spec must be valid")
    }

    fn corpus(spec: &CorpusSpec) -> String {
        let (start, end) = spec.shard_range();
        let mut out = String::new();
        for slot in start..end {
            write_row(&mut out, spec, slot);
        }
        out
    }

    /// A corpus large enough to measure shares on.
    fn share_corpus() -> String {
        corpus(&CorpusSpec::new(SEED, SHARE_SAMPLE, 20_000, 0, 1).expect("valid share spec"))
    }

    /// One emitted row, split into its object text and whether it carried a
    /// named graph. Subject and predicate are always the first two
    /// space-separated tokens (IRIs and blank-node labels never contain a
    /// space); only the object can, so it is everything that remains after the
    /// trailing ` .` and any named graph are peeled off.
    fn split_row(row: &str) -> (&str, bool) {
        let body = row
            .strip_suffix(" .")
            .unwrap_or_else(|| panic!("every row must end with ' .': {row}"));
        let mut fields = body.splitn(3, ' ');
        let _subject = fields.next().expect("subject token");
        let _predicate = fields.next().expect("predicate token");
        let rest = fields.next().expect("object token");
        // The named-graph IRI is always `https://example.org/g/N`, a shape no
        // entity or reifier IRI can take, so this suffix is unambiguous.
        match rest.rfind(" <https://example.org/g/") {
            Some(cut) if rest.ends_with('>') => (&rest[..cut], true),
            _ => (rest, false),
        }
    }

    /// Classifies a row by its emitted TEXT alone — never by re-running the
    /// derivation — so a test can check that the shape the mix chose is the
    /// shape that actually reached the bytes.
    fn kind_of_row_text(row: &str) -> RowKind {
        let (object, _) = split_row(row);
        if object.contains("<<(") {
            RowKind::Reified
        } else if row.starts_with("_:") || object.starts_with("_:") {
            RowKind::BlankNode
        } else if object.ends_with("\"@zh") {
            RowKind::ZhLiteral
        } else if object.contains(&format!("^^<{XSD_NAMESPACE}")) {
            RowKind::TypedLiteral
        } else if object.starts_with("\"value ") {
            RowKind::PlainLiteral
        } else if object.starts_with("\"text ") {
            RowKind::LongTextLiteral
        } else if object.starts_with('<') && object.ends_with('>') {
            RowKind::EntityEdge
        } else {
            panic!("row text matches no known RowKind: {row}")
        }
    }

    /// Whether a row kind puts a literal in object position.
    const fn is_literal_kind(kind: RowKind) -> bool {
        matches!(
            kind,
            RowKind::PlainLiteral
                | RowKind::ZhLiteral
                | RowKind::TypedLiteral
                | RowKind::LongTextLiteral
        )
    }

    /// `count` as a per-mille share of `total`, rounded to nearest.
    fn per_mille(count: u64, total: u64) -> u64 {
        (count * 2_000 + total) / (2 * total)
    }

    #[test]
    fn generation_is_deterministic_and_shard_concat_equals_whole() {
        let whole = corpus(&spec());
        assert_eq!(whole, corpus(&spec()), "two runs must be byte-identical");
        let mut stitched = String::new();
        for shard in 0..3 {
            let sharded = CorpusSpec::new(SEED, 2_000, 1_000, shard, 3).expect("valid shard spec");
            stitched.push_str(&corpus(&sharded));
        }
        assert_eq!(whole, stitched, "shard concatenation must equal one run");
    }

    #[test]
    fn every_row_parses_as_strict_nquads() {
        let text = corpus(&spec());
        let dataset = purrdf_rdf::parse_dataset(text.as_bytes(), "application/n-quads", None)
            .expect("generated corpus must satisfy the strict reader");

        // The corpus now emits RDF 1.2 reifier rows, and a reifier row is NOT
        // a base quad: the reader folds it into the reifier table, leaving
        // `quad_count()` untouched. The exact relationship is therefore a
        // three-way partition of the DISTINCT emitted lines, not a single
        // equality against `quad_count()`:
        //
        //   quad_count() + reifiers().count() + annotations().count()
        //       == distinct emitted lines
        //
        // with `annotations().count() == 0` by construction — an annotation is
        // a row whose subject is a reifier declared in the same graph, and the
        // reifier IRIs live under `example.org/r/`, a path segment no entity
        // IRI shape can produce. Each of the three terms is pinned separately
        // below, so a drop cannot hide by moving rows between tables.
        let lines: Vec<&str> = text.lines().collect();
        let distinct_lines = lines
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let reified_lines = lines
            .iter()
            .filter(|row| kind_of_row_text(row) == RowKind::Reified)
            .count();
        let distinct_reified = lines
            .iter()
            .filter(|row| kind_of_row_text(row) == RowKind::Reified)
            .collect::<std::collections::BTreeSet<_>>()
            .len();

        assert!(
            reified_lines > 0,
            "the identity below is vacuous unless reifier rows are actually present"
        );
        assert_eq!(
            dataset.annotations().count(),
            0,
            "no emitted row may annotate a reifier: reifier IRIs are disjoint from entity IRIs"
        );
        assert_eq!(
            dataset.reifiers().count(),
            distinct_reified,
            "every distinct reifier row must land in the reifier table ({distinct_reified} \
             distinct of {reified_lines} emitted)"
        );
        assert_eq!(
            dataset.quad_count(),
            distinct_lines - distinct_reified,
            "every distinct NON-reifier line must land in the quad table"
        );
        assert_eq!(
            dataset.rdf_row_count(),
            distinct_lines,
            "quads + reifiers + annotations must account for every distinct emitted line \
             ({distinct_lines} distinct of {} emitted): identical rows minted at different \
             slots collapse under set semantics, and nothing else may be lost",
            lines.len()
        );
    }

    #[test]
    fn class_mix_is_exercised_and_indexed_minting_is_stable() {
        let mut seen = [false; 5];
        for entity in 0..2_000 {
            seen[class_of(SEED, entity)] = true;
        }
        assert_eq!(seen, [true; 5], "every class must appear");
        let mut a = String::new();
        let mut b = String::new();
        write_entity_iri(&mut a, SEED, 42);
        write_entity_iri(&mut b, SEED, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn class_mix_measured_shares_match_the_pinned_table() {
        // The class mix partitions the ENTITY SPACE, so it is measured over
        // entity indexes, never over emitted rows (rows draw entities with a
        // deliberate skew, which would weight the classes differently).
        let mut counts = [0u64; CLASS_MIX_PER_MILLE.len()];
        for entity in 0..SHARE_SAMPLE {
            counts[class_of(SEED, entity)] += 1;
        }
        for (position, (name, share)) in CLASS_MIX_PER_MILLE.iter().enumerate() {
            let measured = per_mille(counts[position], SHARE_SAMPLE);
            assert!(
                measured.abs_diff(u64::from(*share)) <= SHARE_TOLERANCE_PER_MILLE,
                "class {name}: measured {measured} per mille over {SHARE_SAMPLE} entities, \
                 pinned {share}, tolerance {SHARE_TOLERANCE_PER_MILLE}"
            );
        }
    }

    #[test]
    fn row_mix_measured_shares_match_the_pinned_table() {
        let text = share_corpus();
        let mut counts = [0u64; ROW_MIX_PER_MILLE.len()];
        let mut rows = 0u64;
        for row in text.lines() {
            counts[kind_of_row_text(row) as usize] += 1;
            rows += 1;
        }
        assert_eq!(rows, SHARE_SAMPLE, "one slot must emit exactly one line");
        for (position, (name, share)) in ROW_MIX_PER_MILLE.iter().enumerate() {
            let measured = per_mille(counts[position], rows);
            assert!(
                measured.abs_diff(u64::from(*share)) <= SHARE_TOLERANCE_PER_MILLE,
                "row kind {name}: measured {measured} per mille over {rows} rows, pinned \
                 {share}, tolerance {SHARE_TOLERANCE_PER_MILLE}"
            );
        }
    }

    #[test]
    fn the_emitted_text_is_the_kind_the_mix_chose() {
        // Closes the loop between the mix table and the bytes: it is not
        // enough that the shares come out right if the row a slot emits has a
        // different shape from the kind that slot drew.
        let spec = CorpusSpec::new(SEED, 20_000, 5_000, 0, 1).expect("valid spec");
        for (slot, row) in corpus(&spec).lines().enumerate() {
            let slot = u64::try_from(slot).expect("slot fits u64");
            assert_eq!(
                kind_of_row_text(row),
                row_kind_of(SEED, slot),
                "slot {slot} emitted text of a different kind than it drew: {row}"
            );
        }
    }

    #[test]
    fn rdf12_and_term_kind_coverage_is_non_vacuous() {
        // A structurally-present but EMPTY class is the failure mode here: the
        // corpus used to contain zero triple terms, zero reifiers, zero typed
        // literals and zero blank nodes while every test stayed green.
        let text = share_corpus();
        let triple_terms = text.matches("<<(").count();
        let reifier_rows = text.matches(super::RDF_REIFIES).count();
        let typed_literals = text.matches("^^<").count();
        let blank_nodes = text.matches("_:").count();
        assert!(triple_terms > 0, "corpus must contain RDF 1.2 triple terms");
        assert!(reifier_rows > 0, "corpus must contain reifier rows");
        assert_eq!(
            triple_terms, reifier_rows,
            "every triple term must be bound by exactly one reifier row"
        );
        assert!(typed_literals > 0, "corpus must contain typed literals");
        assert!(blank_nodes > 0, "corpus must contain blank nodes");

        // Both blank-node label families must be exercised, not just one.
        assert!(
            text.contains("_:bs"),
            "blank nodes must appear in SUBJECT position"
        );
        assert!(
            text.contains("_:bo"),
            "blank nodes must appear in OBJECT position"
        );

        // Every datatype in the cycle must actually appear.
        for datatype in XSD_DATATYPES {
            assert!(
                text.contains(&format!("^^<{XSD_NAMESPACE}{datatype}>")),
                "typed literals must cycle over xsd:{datatype}"
            );
        }
    }

    #[test]
    fn every_typed_literal_lexical_form_is_valid_for_its_datatype() {
        // A strict reader is entitled to reject a lexical form that does not
        // belong to its datatype's lexical space, so the generator must never
        // mint one.
        let text = share_corpus();
        let mut checked = 0u64;
        for row in text.lines() {
            let (object, _) = split_row(row);
            let Some((lexical, datatype)) = object.split_once("\"^^<") else {
                continue;
            };
            let lexical = lexical
                .strip_prefix('"')
                .expect("a typed literal's lexical form is quoted");
            let datatype = datatype
                .strip_suffix('>')
                .and_then(|d| d.strip_prefix(XSD_NAMESPACE))
                .expect("typed literals use the XSD namespace");
            checked += 1;
            match datatype {
                "integer" => {
                    let digits = lexical.strip_prefix('-').unwrap_or(lexical);
                    assert!(
                        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()),
                        "xsd:integer lexical form {lexical:?} is not an optionally-signed \
                         run of digits"
                    );
                }
                "decimal" => {
                    let (whole, fraction) = lexical
                        .split_once('.')
                        .expect("xsd:decimal must carry exactly one '.'");
                    assert!(
                        !fraction.contains('.'),
                        "xsd:decimal must carry exactly one '.': {lexical:?}"
                    );
                    assert!(
                        !whole.is_empty() && whole.bytes().all(|b| b.is_ascii_digit()),
                        "xsd:decimal integral part {whole:?} must be digits"
                    );
                    assert!(
                        !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()),
                        "xsd:decimal fractional part {fraction:?} must be digits"
                    );
                }
                "date" => {
                    let parts: Vec<&str> = lexical.split('-').collect();
                    assert_eq!(parts.len(), 3, "xsd:date must be YYYY-MM-DD: {lexical:?}");
                    assert_eq!(parts[0].len(), 4, "xsd:date year must be 4 digits");
                    let year: u32 = parts[0].parse().expect("year digits");
                    let month: u32 = parts[1].parse().expect("month digits");
                    let day: u32 = parts[2].parse().expect("day digits");
                    assert_eq!(parts[1].len(), 2, "xsd:date month must be 2 digits");
                    assert_eq!(parts[2].len(), 2, "xsd:date day must be 2 digits");
                    assert!((1..=9_999).contains(&year), "year {year} out of range");
                    assert!((1..=12).contains(&month), "month {month} out of range");
                    // Day is capped at 28 so no month — February included, in
                    // any year, leap or not — can ever overflow.
                    assert!((1..=28).contains(&day), "day {day} out of range");
                }
                "boolean" => assert!(
                    lexical == "true" || lexical == "false",
                    "xsd:boolean lexical form {lexical:?} must be exactly true or false"
                ),
                other => panic!("unexpected datatype xsd:{other}"),
            }
        }
        assert!(checked > 0, "no typed literals were checked");
    }

    #[test]
    fn named_graph_membership_is_independent_of_row_kind() {
        // One draw used to drive object kind (`o % 3`), literal subtype
        // (`o % 5`), graph membership (`o % 6`) and graph id (`o >> 16`).
        // Because `o ≡ 0 (mod 6)` forces `o ≡ 0 (mod 3)`, only 14.3% of
        // named-graph rows carried a literal against 42.8% corpus-wide. With
        // graph membership on its own stream tag the two shares must agree.
        let text = share_corpus();
        let (mut rows, mut graph_rows, mut literals, mut graph_literals) = (0u64, 0u64, 0u64, 0u64);
        let mut graph_ids = std::collections::BTreeSet::new();
        for row in text.lines() {
            let (_object, in_graph) = split_row(row);
            let literal = is_literal_kind(kind_of_row_text(row));
            rows += 1;
            literals += u64::from(literal);
            if in_graph {
                graph_rows += 1;
                graph_literals += u64::from(literal);
                let graph = row
                    .rsplit_once(" <https://example.org/g/")
                    .and_then(|(_, tail)| tail.strip_suffix("> ."))
                    .expect("a named-graph row ends with its graph IRI");
                graph_ids.insert(graph.to_string());
            }
        }
        assert_eq!(
            u64::try_from(graph_ids.len()).expect("fits u64"),
            super::NAMED_GRAPHS,
            "every named graph must be used: the graph id draws from its own stream tag, so \
             the spread must not collapse"
        );

        let overall = per_mille(literals, rows);
        let inside = per_mille(graph_literals, graph_rows);
        // ~16 700 named-graph rows at p ≈ 0.4 has a standard deviation of
        // ~3.8 per mille, so 30 per mille is ~8 sigma: wide enough never to
        // flake, far too narrow to admit the 285-per-mille gap the correlated
        // derivation produced.
        assert!(
            inside.abs_diff(overall) <= 30,
            "literal share inside named graphs ({inside} per mille of {graph_rows} rows) must \
             match the corpus-wide literal share ({overall} per mille of {rows} rows)"
        );

        let membership = per_mille(graph_rows, rows);
        assert!(
            membership.abs_diff(1_000 / super::NAMED_GRAPH_SHARE) <= SHARE_TOLERANCE_PER_MILLE,
            "named-graph membership measured {membership} per mille; one row in \
             {} is the documented share",
            super::NAMED_GRAPH_SHARE
        );
    }

    #[test]
    fn skew_map_survives_iris_above_two_to_the_thirty_two() {
        // `[profile.dev]` sets `overflow-checks = true`, so the old
        // `(biased * iris) >> 32` PANICKED here rather than merely wrapping.
        // Ten billion is comfortably past 2^32.
        let iris = 10_000_000_000u64;
        let spec = CorpusSpec::new(SEED, 20_000, iris, 0, 1).expect("valid spec");
        for slot in 0..20_000 {
            for tag in [tags::SUBJECT, tags::OBJECT] {
                let entity = skewed_entity(SEED, tag, slot, iris);
                assert!(
                    entity < iris,
                    "entity index {entity} must be below iris {iris} (slot {slot})"
                );
            }
        }
        // And the whole generator path runs clean at that scale.
        let text = corpus(&spec);
        assert_eq!(text.lines().count(), 20_000);
    }

    #[test]
    fn skew_map_is_not_funnelled_through_a_thirty_two_bit_intermediate() {
        // THE discriminator for the distinct-count ceiling. The old
        // derivation took `r >> 32` before squaring, so `biased` had at most
        // 2^32 values and the map had at most 2^32 DISTINCT outputs however
        // large `iris` was — a cap a max-value assertion cannot see. At
        // `iris = u64::MAX`, for exactly the seed, tag and slot range below,
        // the corrected map returns 200_000 distinct results for 200_000
        // distinct slots; the funnelled one returns 199_968.
        let samples = 200_000u64;
        let distinct = (0..samples)
            .map(|slot| skewed_entity(SEED, tags::SUBJECT, slot, u64::MAX))
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        assert_eq!(
            u64::try_from(distinct).expect("fits u64"),
            samples,
            "at iris = u64::MAX every distinct slot must map to a distinct entity: a collision \
             here means the draw is being funnelled through a narrow intermediate"
        );
    }

    #[test]
    fn skew_map_accepts_the_boundary_iris_values() {
        // Over-refusal counter-check for the 2^32 fix: the neighbouring valid
        // inputs on both sides of the old overflow point must still work, and
        // so must the degenerate single-entity case.
        for iris in [
            1u64,
            4_294_967_295,
            4_294_967_296,
            4_294_967_297,
            10_000_000_000,
            u64::MAX,
        ] {
            let spec = CorpusSpec::new(SEED, 200, iris, 0, 1)
                .unwrap_or_else(|error| panic!("iris {iris} must be accepted: {error}"));
            assert_eq!(corpus(&spec).lines().count(), 200, "iris {iris}");
            for slot in 0..2_000 {
                let entity = skewed_entity(SEED, tags::SUBJECT, slot, iris);
                assert!(entity < iris, "iris {iris}: entity {entity} out of range");
            }
        }
    }

    #[test]
    fn skew_map_keeps_its_shape() {
        // The 128-bit correction must not change the DISTRIBUTION: the skew is
        // still the square of a unit draw, whose CDF is sqrt(k / iris).
        let iris = 1_000_000u64;
        let samples = 200_000u64;
        let mut entities: Vec<u64> = (0..samples)
            .map(|slot| skewed_entity(SEED, tags::SUBJECT, slot, iris))
            .collect();
        entities.sort_unstable();
        // At quantile q the entity index should be about q^2 * iris.
        for (numerator, denominator) in [(1u64, 10u64), (2, 10), (5, 10), (7071, 10_000)] {
            let index = usize::try_from(samples * numerator / denominator).expect("fits usize");
            let expected = iris * numerator * numerator / (denominator * denominator);
            let measured = entities[index];
            assert!(
                measured.abs_diff(expected) <= iris / 100,
                "quantile {numerator}/{denominator}: measured entity {measured}, expected \
                 about {expected} (sqrt CDF), tolerance {}",
                iris / 100
            );
        }
    }

    #[test]
    fn irregular_class_escapes_reserved_octets_in_genuinely_mixed_case() {
        // The class used to emit ONE '%' followed by bare literal hex, both
        // formatters uppercase, over bytes `0x41 + n % 26` — that is `A`-`Z`,
        // RFC 3986 §2.3 UNRESERVED. Percent-encoding an unreserved character
        // is normalization-equivalent (RFC 3986 §6.2.2.2), so a conformant
        // normalizer decodes it away and the "defeats suffix tricks" claim
        // evaporated. Reserved octets must survive normalization verbatim.
        const UNRESERVED_PUNCTUATION: [u8; 4] = *b"-._~";
        let mut samples = 0u64;
        let mut saw_uppercase_hex_letter = false;
        let mut saw_lowercase_hex_letter = false;
        for entity in 0..SHARE_SAMPLE {
            let mut iri = String::new();
            write_entity_iri(&mut iri, SEED, entity);
            let Some(tail) = iri.split("/x/").nth(1) else {
                continue;
            };
            samples += 1;
            let bytes = tail.as_bytes();
            assert_eq!(bytes[0], b'%', "the tail must open with a percent-escape");
            assert_eq!(
                bytes[3], b'%',
                "the tail must carry TWO percent-escapes, not one escape followed by bare \
                 literal hex text: {tail}"
            );
            let upper = &tail[1..3];
            let lower = &tail[4..6];
            assert_eq!(
                upper,
                upper.to_ascii_uppercase(),
                "the first escape must be uppercase hex: {tail}"
            );
            assert_eq!(
                lower,
                lower.to_ascii_lowercase(),
                "the second escape must be lowercase hex: {tail}"
            );
            saw_uppercase_hex_letter |= upper.bytes().any(|b| b.is_ascii_uppercase());
            saw_lowercase_hex_letter |= lower.bytes().any(|b| b.is_ascii_lowercase());

            for hex in [upper, lower] {
                let octet = u8::from_str_radix(hex, 16).expect("two hex digits");
                assert!(
                    RESERVED_OCTETS.contains(&octet),
                    "escaped octet {octet:#04x} must be an RFC 3986 reserved octet"
                );
                assert!(
                    !octet.is_ascii_alphanumeric()
                        && !UNRESERVED_PUNCTUATION.contains(&octet)
                        && octet.is_ascii(),
                    "escaped octet {octet:#04x} must NOT be in the RFC 3986 unreserved set, \
                     whose escape a normalizer is required to decode away"
                );
            }
        }
        assert!(samples > 0, "the irregular class must be exercised");
        assert!(
            saw_uppercase_hex_letter,
            "at least one first escape must use an uppercase hex digit"
        );
        assert!(
            saw_lowercase_hex_letter,
            "at least one second escape must use a lowercase hex digit"
        );
    }

    #[test]
    fn new_rejects_zero_quads() {
        assert_eq!(
            CorpusSpec::new(SEED, 0, 1_000, 0, 1),
            Err(SpecError::ZeroQuads)
        );
        // Neighbouring valid input: the smallest positive quad count.
        assert!(CorpusSpec::new(SEED, 1, 1, 0, 1).is_ok());
    }

    #[test]
    fn new_rejects_zero_iris() {
        assert_eq!(
            CorpusSpec::new(SEED, 2_000, 0, 0, 1),
            Err(SpecError::ZeroIris)
        );
        // Neighbouring valid input: the smallest positive IRI target.
        assert!(CorpusSpec::new(SEED, 2_000, 1_000, 0, 1).is_ok());
    }

    #[test]
    fn new_rejects_zero_shards() {
        assert_eq!(
            CorpusSpec::new(SEED, 2_000, 1_000, 0, 0),
            Err(SpecError::ZeroShards)
        );
        // Neighbouring valid input: one shard (the whole corpus).
        assert!(CorpusSpec::new(SEED, 2_000, 1_000, 0, 1).is_ok());
    }

    #[test]
    fn new_rejects_shard_at_or_past_shards() {
        assert_eq!(
            CorpusSpec::new(SEED, 2_000, 1_000, 7, 7),
            Err(SpecError::ShardOutOfRange {
                shard: 7,
                shards: 7
            }),
            "shard == shards must be rejected"
        );
        assert_eq!(
            CorpusSpec::new(SEED, 2_000, 1_000, 8, 7),
            Err(SpecError::ShardOutOfRange {
                shard: 8,
                shards: 7
            }),
            "shard > shards must be rejected"
        );
        // Neighbouring valid input: the last legal shard index, and shards
        // that legitimately outnumber quads.
        assert!(CorpusSpec::new(SEED, 2_000, 1_000, 6, 7).is_ok());
        assert!(CorpusSpec::new(SEED, 3, 10, 7, 8).is_ok());
    }

    #[test]
    fn shard_range_covers_shards_equal_one() {
        let spec = CorpusSpec::new(SEED, 2_000, 1_000, 0, 1).expect("valid spec");
        assert_eq!(spec.shard_range(), (0, 2_000));
    }

    #[test]
    fn shard_range_last_shard_absorbs_the_remainder() {
        let spec = CorpusSpec::new(SEED, 50_000, 1_000, 6, 7).expect("valid spec");
        assert_eq!(spec.shard_range(), (42_852, 50_000));
    }

    #[test]
    fn shard_range_stitches_exactly_when_shards_outnumber_quads() {
        let quads = 3u64;
        let shards = 8u64;
        let mut covered = Vec::new();
        for shard in 0..shards {
            let spec = CorpusSpec::new(SEED, quads, 1_000, shard, shards).expect("valid spec");
            let (start, end) = spec.shard_range();
            assert!(start <= end, "range must not invert");
            covered.push((start, end));
        }
        // No gap and no overlap: consecutive ranges must be contiguous.
        for window in covered.windows(2) {
            assert_eq!(
                window[0].1, window[1].0,
                "shard ranges must stitch with no gap and no overlap"
            );
        }
        assert_eq!(covered[0].0, 0, "first shard starts at 0");
        assert_eq!(
            covered.last().copied().map(|(_, end)| end),
            Some(quads),
            "last shard ends at quads"
        );
    }

    #[test]
    fn golden_digest_pins_the_profile() {
        // A change to any derivation AFTER PUBLICATION is a new corpus
        // profile: bump CORPUS_PROFILE_ID and re-pin, never silently
        // regenerate.
        //
        // `purrdf-scale-mixed-v1` is *defined* by this work and has never been
        // published: no capture anywhere names it, so correcting its
        // definition (the skew map's 128-bit fixed point, the reserved-octet
        // escapes, the explicit row mix, the independent stream tags) and
        // re-pinning the digest ONCE is fixing a definition before first
        // publication, not regenerating a profile someone measured against.
        // From here on the rule above binds without exception.
        let text = corpus(&spec());
        assert_eq!(
            text.lines().count(),
            2_000,
            "emitted line count is exact — every row kind, reified and
             blank-node included, emits exactly one line per slot (the dataset
             itself may hold fewer: identical rows deduplicate under set
             semantics)"
        );
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in text.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
        }
        assert_eq!(
            hash, 0xEA2E_E654_BA6F_44D3,
            "byte-level FNV pin moved: {CORPUS_PROFILE_ID} must be bumped"
        );
    }

    #[test]
    fn both_mix_tables_sum_to_one_thousand_and_name_their_entries_uniquely() {
        // The sums are already compile-time assertions; this pins the rest of
        // the tables' integrity — distinct names (the manifest is a JSON
        // object, so a duplicate key would silently lose a share) and the
        // enum/table bijection at runtime as well as at build time.
        for table in [&CLASS_MIX_PER_MILLE[..], &ROW_MIX_PER_MILLE[..]] {
            let total: u64 = table.iter().map(|(_, share)| u64::from(*share)).sum();
            assert_eq!(total, 1_000, "a per-mille table must sum to exactly 1000");
            let names: std::collections::BTreeSet<&str> =
                table.iter().map(|(name, _)| *name).collect();
            assert_eq!(names.len(), table.len(), "table names must be distinct");
        }
        for (position, (name, share)) in ROW_MIX_PER_MILLE.iter().enumerate() {
            let kind = RowKind::from_position(position);
            assert_eq!(kind as usize, position, "RowKind must match its position");
            assert_eq!(kind.name(), *name, "RowKind::name must read the table");
            assert_eq!(kind.share_per_mille(), *share);
        }
    }

    #[test]
    fn stream_tags_are_all_distinct() {
        // Independence is only real if no two decisions share a tag: two
        // decisions on the same tag would draw the SAME value and be perfectly
        // correlated, which is the defect this table exists to prevent.
        let all = [
            tags::CLASS,
            tags::ENTITY_SHAPE,
            tags::SUBJECT,
            tags::OBJECT,
            tags::PREDICATE,
            tags::ROW_KIND,
            tags::LITERAL,
            tags::GRAPH_MEMBER,
            tags::GRAPH_ID,
            tags::REIFIER,
            tags::BLANK,
        ];
        let distinct: std::collections::BTreeSet<u64> = all.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            all.len(),
            "every stream tag must be distinct; a shared tag makes two decisions identical"
        );
    }

    #[test]
    fn manifest_carries_both_mixes() {
        let text = super::manifest(&spec());
        // The class-mix key names its BASIS: the shares are of the entity
        // index space, not of the emitted rows, and an unqualified
        // `class_mix_per_mille` invited a capture to record it as the latter.
        assert!(
            text.contains("\"entity_class_mix_per_mille\""),
            "the class-mix key must name the entity space it partitions"
        );
        assert!(
            !text.contains("\"class_mix_per_mille\""),
            "the unqualified class-mix key must not be emitted: its basis was ambiguous"
        );
        assert!(text.contains("\"row_mix_per_mille\""));
        for (name, share) in ROW_MIX_PER_MILLE {
            assert!(
                text.contains(&format!("\"{name}\": {share}")),
                "manifest must carry row kind {name}"
            );
        }
        for (name, share) in CLASS_MIX_PER_MILLE {
            assert!(
                text.contains(&format!("\"{name}\": {share}")),
                "manifest must carry entity class {name}"
            );
        }
    }

    #[test]
    fn manifest_reports_the_shard_line_count_exactly() {
        // `emitted_lines` is `end - start` from `shard_range()` — free and
        // exact, because one slot emits exactly one line for every row kind.
        // Nothing else in the artifacts said how many lines a shard produced.
        for (shard, shards) in [(0u64, 1u64), (0, 7), (3, 7), (6, 7)] {
            let spec = CorpusSpec::new(SEED, 5_000, 1_000, shard, shards).expect("valid spec");
            let (start, end) = spec.shard_range();
            let text = super::manifest(&spec);
            assert!(
                text.contains(&format!("\"emitted_lines\": {}", end - start)),
                "shard {shard} of {shards}: manifest must report end - start; got {text}"
            );
            assert_eq!(
                u64::try_from(corpus(&spec).lines().count()).expect("fits u64"),
                end - start,
                "shard {shard} of {shards}: the reported line count must be the emitted one"
            );
        }
    }

    #[test]
    fn manifest_computes_no_digest() {
        // The doc comment used to claim "the manifest digests exactly these
        // fields"; it computes no digest at all, and a capture's digest is of
        // the OUTPUT BYTES, taken by whatever consumes them. This pins the
        // corrected claim: no field here is a hash.
        let text = super::manifest(&spec()).to_ascii_lowercase();
        for absent in ["digest", "hash", "checksum", "sha256", "blake3", "fnv"] {
            assert!(
                !text.contains(&format!("\"{absent}")),
                "the manifest must carry no {absent} field: it records, it does not digest"
            );
        }
    }

    /// Classifies a row's SUBJECT-position IRI text by which entity-IRI class
    /// (or the reifier shape) produced it. A text-shape classification only —
    /// never a re-derivation of [`class_of`] — used solely to bucket the
    /// byte-length bands below.
    fn iri_class_of_text(iri: &str) -> Option<&'static str> {
        if iri.starts_with("https://example.org/e/") {
            Some("plain")
        } else if iri.starts_with("https://example.org/n/") {
            Some("numeric-long")
        } else if iri.contains("/中文/") {
            Some("chinese")
        } else if iri.starts_with("https://s") && iri.contains(".example.org/x/") {
            Some("irregular")
        } else if iri.starts_with("https://example.org/long/") {
            Some("very-long")
        } else if iri.starts_with("https://example.org/r/") {
            Some("reifier")
        } else {
            None
        }
    }

    #[test]
    fn subject_iri_byte_length_bands_match_measurement() {
        // The doc comment on the very-long class once claimed "~512 bytes"
        // when the true figure is ~628 — a claim nothing here ever checked.
        // This test pins SUBJECT-IRI byte lengths only, and every class's
        // emitted subject-IRI byte length is pinned to a band with headroom;
        // the bands below come from an ACTUAL measurement over SHARE_SAMPLE
        // rows at this module's SEED (plain 23-27, numeric-long exactly 58,
        // chinese 41-45, irregular 54-58, very-long 627-630, reifier 38-42),
        // never from the prose. It does NOT cover literal payloads — an
        // identical doc-vs-code drift in a `@zh` literal's Han character
        // count survived here undetected; see
        // `literal_payload_shapes_match_measurement` below for that axis.
        let text = share_corpus();
        let mut bands: std::collections::BTreeMap<&'static str, (usize, usize)> =
            std::collections::BTreeMap::new();
        for row in text.lines() {
            if !row.starts_with('<') {
                continue; // a blank-node-subject row has no subject IRI to measure
            }
            let end = row
                .find("> ")
                .expect("a subject IRI must be closed by '> '");
            let subject = &row[1..end];
            let Some(class) = iri_class_of_text(subject) else {
                continue;
            };
            let len = subject.len();
            bands
                .entry(class)
                .and_modify(|(lo, hi)| {
                    *lo = (*lo).min(len);
                    *hi = (*hi).max(len);
                })
                .or_insert((len, len));
        }

        // (class, headroom-widened lower bound, headroom-widened upper bound).
        let expected: &[(&str, usize, usize)] = &[
            ("plain", 18, 32),
            ("numeric-long", 55, 61),
            ("chinese", 36, 50),
            ("irregular", 48, 63),
            ("very-long", 600, 650),
            ("reifier", 33, 47),
        ];
        for (class, lo, hi) in expected {
            let (measured_lo, measured_hi) = bands
                .get(class)
                .unwrap_or_else(|| panic!("class {class} must be exercised"));
            assert!(
                measured_lo >= lo && measured_hi <= hi,
                "class {class}: measured band [{measured_lo}, {measured_hi}] must fall inside \
                 [{lo}, {hi}] — bands come from measurement, not from doc prose"
            );
        }
        assert_eq!(
            bands.len(),
            expected.len(),
            "every class band must be observed exactly once; observed {bands:?}"
        );
    }

    /// Splits a typed-literal OBJECT into its XSD local name and lexical
    /// form, e.g. `"212"^^<http://www.w3.org/2001/XMLSchema#integer>` ->
    /// (`"integer"`, `"212"`). A text-shape read only, exactly like
    /// [`kind_of_row_text`] above — never a re-derivation of
    /// [`write_typed_literal`].
    fn typed_literal_parts(object: &str) -> (&str, &str) {
        let lexical = object
            .strip_prefix('"')
            .and_then(|rest| rest.split('"').next())
            .expect("typed literal must open with a quoted lexical form");
        let namespace_at = object
            .rfind(XSD_NAMESPACE)
            .expect("typed literal must carry an XSD datatype IRI");
        let local = object[namespace_at + XSD_NAMESPACE.len()..].trim_end_matches('>');
        (local, lexical)
    }

    /// Widens `band` (an inclusive `[lo, hi]`, or unset) to also cover
    /// `value`.
    fn widen_band(band: &mut Option<(usize, usize)>, value: usize) {
        *band = Some(match *band {
            Some((lo, hi)) => (lo.min(value), hi.max(value)),
            None => (value, value),
        });
    }

    #[test]
    fn literal_payload_shapes_match_measurement() {
        // The `@zh`-literal doc once claimed "four" Han characters when the
        // loop only ever emits three — a claim [`subject_iri_byte_length_bands_match_measurement`]
        // above never checked, because it pins subject IRIs only. This test
        // measures every literal-bearing row kind's PAYLOAD instead: the
        // `@zh` literal's Han character count, the plain-literal digit
        // shape, the long-text literal's fixed byte length, and each typed
        // literal's lexical form — all from an ACTUAL measurement over
        // SHARE_SAMPLE rows at this module's SEED, never from the prose.
        let text = share_corpus();

        let mut zh_chars: Option<(usize, usize)> = None;
        let mut plain_digits: Option<(usize, usize)> = None;
        let mut long_text_len: Option<usize> = None;
        let mut integer_digits: Option<(usize, usize)> = None;
        let mut decimal_int_digits: Option<(usize, usize)> = None;
        let mut date_rows = 0u64;
        let mut boolean_seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let (mut zh_rows, mut plain_rows, mut long_rows, mut typed_rows) = (0u64, 0u64, 0u64, 0u64);

        for row in text.lines() {
            let (object, _) = split_row(row);
            match kind_of_row_text(row) {
                RowKind::ZhLiteral => {
                    zh_rows += 1;
                    let lexical = object
                        .strip_prefix('"')
                        .and_then(|rest| rest.strip_suffix("\"@zh"))
                        .expect("zh-literal object must be a quoted @zh literal");
                    widen_band(&mut zh_chars, lexical.chars().count());
                }
                RowKind::PlainLiteral => {
                    plain_rows += 1;
                    let digits = object
                        .strip_prefix("\"value ")
                        .and_then(|rest| rest.strip_suffix('"'))
                        .expect("plain-literal object must be `\"value N\"`");
                    assert!(
                        digits.bytes().all(|b| b.is_ascii_digit()),
                        "plain-literal payload must be bare digits: {digits:?}"
                    );
                    widen_band(&mut plain_digits, digits.len());
                }
                RowKind::LongTextLiteral => {
                    long_rows += 1;
                    match long_text_len {
                        Some(len) => assert_eq!(
                            len,
                            object.len(),
                            "every long-text literal must share one fixed length"
                        ),
                        None => long_text_len = Some(object.len()),
                    }
                }
                RowKind::TypedLiteral => {
                    typed_rows += 1;
                    let (local, lexical) = typed_literal_parts(object);
                    match local {
                        "integer" => {
                            let digits = lexical.trim_start_matches('-');
                            assert!(
                                digits.bytes().all(|b| b.is_ascii_digit()),
                                "xsd:integer lexical form must be signed digits: {lexical:?}"
                            );
                            widen_band(&mut integer_digits, digits.len());
                        }
                        "decimal" => {
                            let (int_part, frac) = lexical
                                .split_once('.')
                                .expect("xsd:decimal lexical form must carry exactly one '.'");
                            assert_eq!(
                                frac.len(),
                                6,
                                "xsd:decimal fractional part must be zero-padded to 6 digits: \
                                 {lexical:?}"
                            );
                            widen_band(&mut decimal_int_digits, int_part.len());
                        }
                        "date" => {
                            assert_eq!(
                                lexical.len(),
                                10,
                                "xsd:date lexical form must be YYYY-MM-DD: {lexical:?}"
                            );
                            date_rows += 1;
                        }
                        "boolean" => {
                            assert!(
                                lexical == "true" || lexical == "false",
                                "xsd:boolean lexical form must be true or false: {lexical:?}"
                            );
                            boolean_seen.insert(lexical);
                        }
                        other => panic!("unknown XSD datatype local name: {other}"),
                    }
                }
                RowKind::EntityEdge | RowKind::Reified | RowKind::BlankNode => {}
            }
        }

        let (zh_lo, zh_hi) = zh_chars.expect("zh-literal rows must be exercised");
        assert_eq!(
            (zh_lo, zh_hi),
            (3, 3),
            "a @zh literal must carry exactly three Han characters"
        );

        let (plain_lo, plain_hi) = plain_digits.expect("plain-literal rows must be exercised");
        assert!(
            plain_lo >= 10 && plain_hi <= 17,
            "plain-literal digit-length band [{plain_lo}, {plain_hi}] must fall inside [10, 17]"
        );

        assert_eq!(
            long_text_len,
            Some(136),
            "a long-text literal object must be exactly 136 bytes (2 quotes + \"text \" + two \
             zero-padded 64-hex-digit words + 1 separating space)"
        );

        let (int_lo, int_hi) = integer_digits.expect("xsd:integer rows must be exercised");
        assert!(
            int_lo >= 6 && int_hi <= 12,
            "xsd:integer digit-length band [{int_lo}, {int_hi}] must fall inside [6, 12]"
        );

        let (dec_lo, dec_hi) = decimal_int_digits.expect("xsd:decimal rows must be exercised");
        assert!(
            dec_lo >= 1 && dec_hi <= 6,
            "xsd:decimal integer-part digit-length band [{dec_lo}, {dec_hi}] must fall inside \
             [1, 6]"
        );

        assert!(date_rows > 0, "xsd:date rows must be exercised");
        assert_eq!(
            boolean_seen,
            ["false", "true"].into_iter().collect(),
            "xsd:boolean must be observed emitting both true and false"
        );

        assert!(
            zh_rows > 0 && plain_rows > 0 && long_rows > 0 && typed_rows > 0,
            "every literal-bearing row kind must be exercised: zh={zh_rows} plain={plain_rows} \
             long={long_rows} typed={typed_rows}"
        );
    }

    /// Compares a (possibly zero-padded) decimal digit string against a
    /// bound's decimal digits, WITHOUT parsing either into a fixed-width
    /// integer: parsing a 36-digit numeral into `u64` would overflow by
    /// construction. Both operands are bare ASCII digit runs (no sign).
    fn digit_string_exceeds(digits: &str, bound: &str) -> bool {
        let trimmed = digits.trim_start_matches('0');
        let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
        match trimmed.len().cmp(&bound.len()) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => trimmed > bound,
        }
    }

    #[test]
    fn numeric_long_class_emits_exactly_36_digits_beyond_u64_and_2_53() {
        // The doc claims "36 digits... beyond u64 and 2^53"; nothing checked
        // the digit WIDTH or the MAGNITUDE before. `u64::MAX` has 20 decimal
        // digits and `2^53` has 16; comparing digit STRINGS (never parsing
        // into `u64`, which would overflow by construction at 36 digits) is
        // the only safe way to assert "exceeds" here.
        const U64_MAX_DIGITS: &str = "18446744073709551615";
        const TWO_POW_53_DIGITS: &str = "9007199254740992";
        let mut checked = 0u64;
        let mut saw_leading_zero = false;
        for entity in 0..SHARE_SAMPLE {
            if class_of(SEED, entity) != 1 {
                continue; // not the numeric-long class
            }
            let mut iri = String::new();
            write_entity_iri(&mut iri, SEED, entity);
            let digits = iri
                .strip_prefix("https://example.org/n/")
                .expect("numeric-long IRI must carry the /n/ prefix");
            assert_eq!(
                digits.len(),
                36,
                "numeric-long entity {entity} must emit exactly 36 digits: {digits:?}"
            );
            assert!(
                digits.bytes().all(|b| b.is_ascii_digit()),
                "numeric-long entity {entity} must be all decimal digits: {digits:?}"
            );
            saw_leading_zero |= digits.starts_with('0');
            assert!(
                digit_string_exceeds(digits, U64_MAX_DIGITS),
                "numeric-long entity {entity} value {digits} must exceed u64::MAX"
            );
            assert!(
                digit_string_exceeds(digits, TWO_POW_53_DIGITS),
                "numeric-long entity {entity} value {digits} must exceed 2^53"
            );
            checked += 1;
        }
        assert!(checked > 0, "the numeric-long class must be exercised");
        assert!(
            saw_leading_zero,
            "at least one numeric-long value must carry a leading zero that survives verbatim \
             (identity, not number)"
        );
    }

    #[test]
    fn generated_terms_survive_the_strict_reader_intact() {
        // `every_row_parses_as_strict_nquads` pins a COUNT identity across the
        // quad/reifier/annotation tables; it says nothing about whether an
        // individual TERM survives the round trip with its distinctive shape
        // intact. This resolves actual terms out of the parsed dataset and
        // checks every stress shape the corpus promises: a raw-Han IRI, a
        // @zh language-tagged literal, a named-graph name, and — the one
        // most likely to surprise — a reserved-octet percent-escape that a
        // normalizer must NOT silently decode.
        let text = share_corpus();
        let dataset = purrdf_rdf::parse_dataset(text.as_bytes(), "application/n-quads", None)
            .expect("generated corpus must satisfy the strict reader");

        let mut saw_raw_han_iri = false;
        let mut saw_zh_literal = false;
        let mut saw_named_graph = false;
        let mut saw_intact_percent_escape = false;
        let mut percent_escape_example: Option<String> = None;

        for quad in dataset.quad_refs() {
            for term in [quad.s, quad.p, quad.o] {
                if let purrdf_rdf::TermRef::Iri(iri) = term {
                    if iri.contains("中文") {
                        saw_raw_han_iri = true;
                    }
                    for octet in RESERVED_OCTETS {
                        let upper = format!("%{octet:02X}");
                        let lower = format!("%{octet:02x}");
                        if iri.contains(&upper) || iri.contains(&lower) {
                            saw_intact_percent_escape = true;
                            percent_escape_example.get_or_insert_with(|| iri.to_string());
                        }
                    }
                }
                if let purrdf_rdf::TermRef::Literal {
                    lexical,
                    language: Some(lang),
                    ..
                } = term
                    && lang == "zh"
                {
                    saw_zh_literal = true;
                    assert!(
                        lexical
                            .chars()
                            .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c)),
                        "a @zh literal's lexical form must survive as Han characters: {lexical:?}"
                    );
                }
            }
            if let Some(purrdf_rdf::TermRef::Iri(graph)) = quad.g
                && graph.starts_with("https://example.org/g/")
            {
                saw_named_graph = true;
            }
        }

        assert!(
            saw_raw_han_iri,
            "a raw-Han IRI must survive un-escaped through the strict reader"
        );
        assert!(
            saw_zh_literal,
            "a @zh language-tagged literal must survive with its tag intact"
        );
        assert!(
            saw_named_graph,
            "a named graph name must survive through the strict reader"
        );
        assert!(
            saw_intact_percent_escape,
            "a reserved-octet percent-escape must survive verbatim, not silently decoded; \
             found no such term (example seen: {percent_escape_example:?})"
        );
    }
}

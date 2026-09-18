// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **The prepared-product encoder is a pure function of the shapes graph, and its
//! output is pinned to committed bytes.**
//!
//! `PreparedShapes::to_product` documents that "two calls over equal inputs
//! produce byte-identical buffers". That is a claim, and the three ways it can be
//! false are progressively harder to see:
//!
//! 1. the encoder consults a clock or a source of randomness — loud, and nothing
//!    here does it;
//! 2. the encoder walks a hash map in *iteration* order, so the bytes depend on
//!    how the process that parsed the shapes graph happened to seed its tables —
//!    silent, and invisible to any test that encodes one parse twice, because the
//!    same parse has the same tables;
//! 3. the encoder is stable within a build but the format has quietly moved, so
//!    yesterday's cached product is no longer the bytes today's build writes —
//!    silent until somebody restores a product nobody re-prepared.
//!
//! One test here answers each: [`encode_is_byte_deterministic`] the first,
//! [`encode_is_stable_across_independent_parses`] the second — two *independent*
//! parses of one source text, each with its own tables — and
//! [`frozen_product_bytes`] the third, against an artifact committed to the
//! repository.
//!
//! # The byte length is an asserted fact
//!
//! [`GOLDEN_LEN`] pins how large the fixture's product actually is. It is asserted
//! rather than narrated in a benchmark log because a size claim that lives in
//! prose is a claim nobody re-checks: a codec change that doubled the intermediate
//! bytes would leave every equality assertion here passing (they compare against a
//! regenerated golden) and every sentence about compactness silently false. A
//! number the build enforces cannot rot that way.
//!
//! # Regeneration
//!
//! `fixtures/prepared-shapes-core.product` is this build's output for
//! [`product_fixture::SHAPES`]. When the container format legitimately changes,
//! [`frozen_product_bytes`] fails, and the artifact and [`GOLDEN_LEN`] are updated
//! together in the same reviewable commit as the format change.
//!
//! `fixtures/prepared-shapes-core-format-v1-frozen.product` is NOT that file and
//! is never regenerated — see [`the_frozen_format_epoch_product_stays_readable`].
//! As of the preparation stage that made the reusable class analysis travel inside
//! the artifact, that file is a genuine older-stage product, so forward
//! compatibility is now WITNESSED rather than merely set up: `admit` refuses it and
//! `rebuild` rescues it, executed on every run by
//! [`the_frozen_format_epoch_product_is_refused_then_rebuilt`].

mod product_fixture;

use purrdf_shapes::product::{ProductDimension, ShapesProduct, ShapesProfile};

/// The exact size, in bytes, of the prepared product this build writes for
/// [`product_fixture::SHAPES`].
///
/// This is the "intermediate bytes" measurement for the fixture, pinned as a fact
/// the build checks rather than a figure quoted from a run nobody can reproduce.
const GOLDEN_LEN: usize = 4_568;

/// The product artifact frozen by the commit that introduced the prepared-product
/// format, for the forward-compatibility proof. See
/// [`the_frozen_format_epoch_product_stays_readable`].
const FORMAT_EPOCH_GOLDEN: &[u8] =
    include_bytes!("fixtures/prepared-shapes-core-format-v1-frozen.product");

/// Assert two byte strings are equal, naming the first offset that differs.
///
/// `assert_eq!` over two multi-kilobyte `Vec<u8>` values prints both in full and
/// leaves the reader to find the divergence. The offset is the part that says
/// *which section* moved.
fn assert_same_bytes(left: &[u8], right: &[u8], claim: &str) {
    if left == right {
        return;
    }
    let at = left
        .iter()
        .zip(right.iter())
        .position(|(a, b)| a != b)
        .map_or_else(
            || {
                format!(
                    "one is a prefix of the other after {} bytes",
                    left.len().min(right.len())
                )
            },
            |at| {
                format!(
                    "first difference at byte {at}: {:#04x} vs {:#04x}",
                    left[at], right[at]
                )
            },
        );
    panic!(
        "{claim}\nleft is {} bytes, right is {} bytes; {at}",
        left.len(),
        right.len()
    );
}

// ── The encoder is a function ──────────────────────────────────────────────────

/// Encoding ONE preparation twice yields the same bytes.
///
/// The weakest of the three claims and still worth executing: it is the one a
/// clock, a counter or a `RandomState` read *inside* the encoder would break, and
/// it fails immediately rather than intermittently.
#[test]
fn encode_is_byte_deterministic() {
    let prepared = product_fixture::prepared();
    let first = prepared
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable");
    let second = prepared
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable");

    assert_same_bytes(
        &first,
        &second,
        "encoding one preparation twice produced different bytes, so something outside the \
         shapes graph reached the writer",
    );
}

/// Encoding two INDEPENDENT parses of the same source text yields the same bytes.
///
/// **This is the one that catches hash-iteration order leaking into the output.**
/// Each `parse` builds its own tables, and `std`'s default hasher seeds every map
/// differently, so a writer that emitted anything in map-iteration order produces
/// two different byte strings here while passing
/// [`encode_is_byte_deterministic`] every time.
///
/// The fixture is built for exactly this: several shapes, several document
/// prefixes, four target kinds and a box-role vocabulary, so there is real
/// map-shaped state for an ordering bug to leak from. A fixture with one shape and
/// one prefix would pass this test for free and prove nothing.
#[test]
fn encode_is_stable_across_independent_parses() {
    let first = product_fixture::encode();
    let second = product_fixture::encode();

    assert_same_bytes(
        &first,
        &second,
        "two independent parses of one shapes graph encoded to different products, so the \
         writer's output depends on how a parse laid its tables out rather than on the shapes \
         graph",
    );
}

/// Restoring a product and re-encoding it reproduces the original bytes exactly.
///
/// Encode-decode-encode is the round trip that catches a codec whose decoder is
/// merely *compatible* with its encoder rather than inverse to it: a field the
/// decoder defaults instead of reading, or normalizes on the way in, survives
/// every equality test over a single encode and shows up here as bytes that drift
/// one restore at a time. A cached product that is re-prepared from a restore is
/// exactly that loop.
#[test]
fn encode_decode_encode_is_idempotent() {
    let original = product_fixture::encode();
    let restored = ShapesProduct::open(&original)
        .expect("the product opens")
        .admit(&ShapesProfile::CORE, &product_fixture::host())
        .expect("the product admits");

    let round_tripped = restored
        .to_product(&ShapesProfile::CORE)
        .expect("a restored preparation re-encodes");

    assert_same_bytes(
        &round_tripped,
        &original,
        "re-encoding a restored product did not reproduce its own bytes, so the decoder is not \
         the encoder's inverse and a restore-then-cache loop drifts",
    );
}

// ── The bytes are pinned ───────────────────────────────────────────────────────

/// This build's product for the fixture is the committed golden, byte for byte,
/// and exactly [`GOLDEN_LEN`] bytes long.
#[test]
fn frozen_product_bytes() {
    let bytes = product_fixture::encode();

    assert_eq!(
        bytes.len(),
        GOLDEN_LEN,
        "the fixture's product is {} bytes, not the pinned {GOLDEN_LEN}; if the format changed \
         deliberately, update the artifact and this constant together",
        bytes.len()
    );
    assert_eq!(
        product_fixture::GOLDEN.len(),
        GOLDEN_LEN,
        "the committed golden artifact is not {GOLDEN_LEN} bytes, so the constant and the file \
         disagree about the same fact",
    );
    assert_same_bytes(
        &bytes,
        product_fixture::GOLDEN,
        "this build writes a different product for the fixture than the committed golden, so \
         the container format moved; re-prepare the artifact in the same commit as the format \
         change, and expect every cached product in the wild to need the same treatment",
    );
}

/// The committed golden is a product that actually restores and actually
/// validates.
///
/// Without this, [`frozen_product_bytes`] only proves the artifact equals what the
/// encoder emits — which a golden of pure garbage would also satisfy if the
/// encoder were emitting garbage.
#[test]
fn frozen_golden_admits() {
    let restored = ShapesProduct::open(product_fixture::GOLDEN)
        .expect("the committed golden opens")
        .admit(&ShapesProfile::CORE, &product_fixture::host())
        .expect("the committed golden admits");

    let restored_nt = product_fixture::report_nt(&restored);
    product_fixture::assert_non_vacuous(&restored_nt);
    assert_eq!(
        restored_nt,
        product_fixture::expected_report_nt(),
        "the golden restored to a validator that answers differently from a fresh parse of the \
         same shapes graph",
    );
}

// ── The cross-version reader proof ─────────────────────────────────────────────

/// **A product frozen by the format's introducing commit stays readable by every
/// later build.**
///
/// # Why this artifact is never regenerated
///
/// The module's other golden is regenerated whenever the container format
/// legitimately moves, which is what makes it a good tripwire and a useless
/// witness: a file that is rewritten by the build it is meant to test can only
/// ever say that today's encoder agrees with today's encoder. Forward
/// compatibility is a claim about a product written by a build that no longer
/// exists, so the only way to check it is to keep such a product — frozen, from
/// the change that introduced the format, with nothing in the repository
/// authorized to rewrite it.
///
/// `fixtures/prepared-shapes-core-format-v1-frozen.product` is that file. It was
/// frozen by the commit that introduced the prepared-product format, and it must
/// never be regenerated: regenerating it converts this test from a verification
/// back into the argument it replaced, and does so silently, because it would
/// keep passing.
///
/// # This is now a witness, not a rehearsal
///
/// The artifact began as a byte-identical copy of the regenerable golden, because
/// one build had written both and the preparation had not yet moved. That was the
/// honest starting state and it proved nothing: the two files differed in
/// *lifetime*, not content, and forward compatibility across a real change had only
/// been set up to be witnessed.
///
/// The preparation has since moved. A prepared product now CARRIES the reusable
/// class analysis rather than pinning only its digest, and the stage id — which is
/// derived from the model and from the class walk's own source — moved with it. So
/// this artifact is, for the first time, a genuine product of an earlier
/// preparation stage, and the seam it was frozen to test is executed rather than
/// described: the file this build cannot admit is the file this build rebuilds.
///
/// It remains READABLE rather than merely refusable because the container format
/// version did not move with the preparation. The class analysis travels inside the
/// existing AST section instead of a fourth section of its own, precisely so the
/// section directory and the format version stay where they are; a fourth section
/// would have made this product fail to OPEN, and a product that cannot open cannot
/// be rescued by anything.
///
/// # What every future reader owes it
///
/// Two outcomes are acceptable, and they are the two seams the reader documents:
///
/// * the stage id has moved on — the preparation this memo describes is not this
///   build's — so `admit` refuses with [`ProductDimension::StageId`] and `rebuild`
///   re-derives the shapes graph from the dataset the product carries; or
/// * some later change returns this build to the frozen artifact's own stage id, so
///   `admit` succeeds again.
///
/// The first is what happens today and the assertions below take that path. The
/// second is left acceptable on purpose: it is a fact about which stage this build
/// is at, not a weaker promise, and a test that forbade it would fail on a correct
/// revert. Either way the product must `open`, and either way the restored
/// validator must produce the same report as a fresh parse of the same shapes
/// graph. Every third outcome fails here: opening at all failing, `admit` refusing
/// on some *other* dimension (which would mean the envelope, the profile or the
/// identity binding broke rather than the preparation moving), `rebuild` refusing,
/// or either path restoring to a validator that answers differently.
#[test]
fn the_frozen_format_epoch_product_stays_readable() {
    let expected = product_fixture::expected_report_nt();

    let view = ShapesProduct::open(FORMAT_EPOCH_GOLDEN).expect(
        "the frozen format-epoch product must still OPEN; the envelope's framing, magic and version \
         are the layer that was never allowed to move",
    );

    let refusal = match view.admit(&ShapesProfile::CORE, &product_fixture::host()) {
        Ok(restored) => {
            assert_eq!(
                product_fixture::report_nt(&restored),
                expected,
                "the frozen format-epoch product admitted but answers differently from a fresh parse \
                 of the same shapes graph",
            );
            return;
        }
        Err(refusal) => refusal,
    };

    assert_eq!(
        refusal.dimension(),
        ProductDimension::StageId,
        "the frozen format-epoch product was refused on `{}`, and the ONLY refusal a later build may \
         answer it with is `StageId` — the memo describing a model this build no longer has. \
         Any other dimension means the envelope, the profile or the identity binding stopped \
         accepting a product this project promised to keep reading: {}",
        refusal.dimension().label(),
        refusal.message(),
    );

    let rebuilt = ShapesProduct::open(FORMAT_EPOCH_GOLDEN)
        .expect("the frozen format-epoch product opens")
        .rebuild(&ShapesProfile::CORE, &product_fixture::host())
        .expect(
            "`admit` refused on the stage id, so `rebuild` is the seam that exists for exactly \
             this product; a refusal here means the forward-compatibility path is not one",
        );

    let rebuilt_nt = product_fixture::report_nt(&rebuilt);
    product_fixture::assert_non_vacuous(&rebuilt_nt);
    assert_eq!(
        rebuilt_nt, expected,
        "the frozen format-epoch product rebuilt into a validator that answers differently from a \
         fresh parse of the same shapes graph, which is the quiet failure `rebuild` exists to \
         avoid: it restored, and it restored the wrong shapes graph",
    );
}

/// **The frozen format-epoch product is refused by `admit` and rescued by
/// `rebuild`, TODAY.**
///
/// [`the_frozen_format_epoch_product_stays_readable`] states the durable contract,
/// which is total over both stages a future build could be at. This states the
/// FACT, and the difference matters: a total contract is satisfied by whichever
/// branch happens to run, so it would keep passing if the frozen artifact quietly
/// stopped being an older-stage product and the rescue path went back to never
/// being executed. An escape hatch that has never been opened is a plan, not a
/// seam.
///
/// So this is written unconditionally. The frozen artifact was minted at a
/// preparation stage that pinned the class analysis by digest and carried no body;
/// this build carries the body, and the stage id moved accordingly, so `admit` must
/// refuse it on [`ProductDimension::StageId`] and `rebuild` must re-derive the whole
/// shapes graph from the dataset the product carries and answer exactly as a fresh
/// parse does.
///
/// A build that legitimately returns to the frozen artifact's own stage id will
/// fail here, and that is correct rather than brittle: on that day this test has
/// stopped being a witness to anything, and it should be rewritten deliberately
/// against whatever artifact IS then an older stage — not left passing while
/// proving nothing.
#[test]
fn the_frozen_format_epoch_product_is_refused_then_rebuilt() {
    let refusal = ShapesProduct::open(FORMAT_EPOCH_GOLDEN)
        .expect("the frozen format-epoch product opens")
        .admit(&ShapesProfile::CORE, &product_fixture::host())
        .expect_err(
            "the frozen format-epoch product was minted at an earlier preparation stage — before \
             the reusable class analysis travelled inside the artifact — so `admit` must refuse \
             it rather than execute a memo written against a preparation this build no longer \
             performs",
        );
    assert_eq!(
        refusal.dimension(),
        ProductDimension::StageId,
        "the frozen format-epoch product was refused on `{}`, and the only refusal an earlier \
         preparation stage may draw is `StageId`; any other dimension means the envelope, the \
         profile or the identity binding stopped accepting a product this project promised to \
         keep reading: {}",
        refusal.dimension().label(),
        refusal.message(),
    );

    let rebuilt = ShapesProduct::open(FORMAT_EPOCH_GOLDEN)
        .expect("the frozen format-epoch product opens")
        .rebuild(&ShapesProfile::CORE, &product_fixture::host())
        .expect(
            "`admit` refused on the stage id, so `rebuild` is the seam that exists for exactly \
             this product; a refusal here means the forward-compatibility path is not one",
        );

    let rebuilt_nt = product_fixture::report_nt(&rebuilt);
    product_fixture::assert_non_vacuous(&rebuilt_nt);
    assert_eq!(
        rebuilt_nt,
        product_fixture::expected_report_nt(),
        "rebuilding the frozen format-epoch product yielded a validator that answers differently \
         from a fresh parse of the same shapes graph",
    );
}

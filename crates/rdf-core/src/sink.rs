// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The workspace's one text-egress sink: a bounded staging window over a
//! pluggable byte drain.
//!
//! Every serializer in this workspace appends text fragments. Historically each
//! appended into a `String` that grew to the whole document, so peak residency
//! tracked the OUTPUT size rather than the working set. [`TextSink`] is that same
//! append target with the buffer bounded: fragments accumulate in a fixed window
//! and are handed to a [`ByteDrain`] whenever the window would overflow.
//!
//! # One emitter, many drains
//!
//! The eager whole-document spelling is not a second code path — it is a
//! [`TextSink::in_memory`] whose window is never drained. The same emitter, walked
//! the same way, produces the same fragments in the same order; only the drain
//! differs. Byte identity between the eager and streamed spellings is therefore a
//! property of there being ONE emitter, not something a test establishes after the
//! fact. `purrdf-cdt`'s renderer states the same theorem for its own walker.
//!
//! The drain is a trait rather than [`std::io::Write`] on purpose. Counting a
//! document's length, digesting it, and handing chunks to a JavaScript callback, a
//! C function pointer, or a Python file object are all drains; none of them is an
//! `io::Write`. [`WriterDrain`] adapts `io::Write` as one implementor among several.
//!
//! # What this does and does not bound
//!
//! Be precise about the claim. A draining `TextSink` allocates
//! [`DRAIN_BUFFER_BYTES`] once and never reallocates: `staged.len() <
//! DRAIN_BUFFER_BYTES` is an invariant of every method, independent of document
//! size, because a push that would cross the threshold drains first and a push
//! larger than the whole window is written straight through from the caller's
//! borrowed slice without ever being copied.
//!
//! It removes exactly ONE term from a serializer's peak — the finished document —
//! and nothing else. It does NOT make serialization constant-memory and nothing
//! here should be read as claiming that: the model a serializer walks is
//! proportional to its input, and several formats additionally hold a grouping
//! index named in their own documentation. The honest statement is: **peak
//! residency drops from `model + per-format index + whole output` to `model +
//! per-format index + 64 KiB`.**
//!
//! # Failure, and why `finish` is not optional
//!
//! Pushes are infallible. A serializer appends at many hundreds of call sites and
//! threading a `Result` through every one of them would be a larger and riskier
//! change than the bound it buys. Instead the FIRST drain failure is retained and
//! every later push is DISCARDED rather than staged — so the memory bound holds
//! even for an emitter that never checks [`TextSink::failed`]. The consequence is
//! that **consulting [`TextSink::finish`] is a correctness invariant, not a
//! convenience**: it is the only place a drain failure is reported. It is
//! `#[must_use]` for that reason, and it is the sole terminal — there is no
//! spelling that yields bytes while discarding a failure.
//!
//! Emitters should poll [`TextSink::failed`] at their innermost emission loop so a
//! dead drain costs one fragment of formatting rather than a whole document. The
//! sink never interprets a drain error; deciding that a broken downstream pipe is
//! success is a policy its owner holds, not the serializer.
//!
//! # Rewinds are not representable
//!
//! There is deliberately no `truncate`, `pop`, `clear`, or `insert`. A sink cannot
//! un-write bytes it has already drained, so an emitter that writes a header and
//! retracts it on discovering the body was empty is not expressible here — it must
//! decide before it emits. That is a constraint on emitters and it is the point:
//! the affordance that would silently break streaming does not exist.

use core::fmt;

/// The staging window a draining sink holds between writes.
///
/// Sized to amortize the per-window call and syscall without making the bounded
/// claim depend on a large constant: 64 KiB is the same order as a typical reader
/// buffer and is dwarfed by any document worth streaming. This is the workspace's
/// one egress window constant — the ingress side consumes it rather than defining
/// a second.
pub const DRAIN_BUFFER_BYTES: usize = 64 << 10;

/// Why a drain refused the bytes handed to it.
///
/// Deliberately not `std::io::Error`: a drain may be a JavaScript callback, a C
/// function pointer, or a Python file object, none of which produce one. The
/// `kind` is retained rather than flattened into prose so an owner can act on it —
/// treating a broken downstream pipe as clean success, for one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainError {
    kind: DrainErrorKind,
    message: String,
}

/// The classification a [`DrainError`] carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DrainErrorKind {
    /// The downstream reader closed before the document finished. Whether this is
    /// a failure is the owner's decision, not the sink's.
    BrokenPipe,
    /// Any other refusal.
    Other,
}

impl DrainError {
    /// A drain failure of `kind`, described by `message`.
    #[must_use]
    pub fn new(kind: DrainErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// This failure's classification.
    #[must_use]
    pub const fn kind(&self) -> DrainErrorKind {
        self.kind
    }

    /// This failure's description.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DrainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// An infallible append target for text fragments.
///
/// Implemented by [`String`] and by [`TextSink`]. Emitters take this as a GENERIC
/// bound and monomorphize, so shared writer code costs no indirect call and a
/// caller that already holds a whole document — a sort comparator rendering a key
/// into reusable scratch, say — keeps using a plain `String` at full speed.
///
/// That is the division of labour that makes one emitter serve both spellings. The
/// object-safe boundary sits ABOVE these writers, at the codec seam, where dispatch
/// happens once per document rather than once per fragment; below it, everything is
/// generic. Neither mechanism is right at both altitudes.
///
/// Note the absence of any read-back, rewind, or length accessor. An emitter
/// written against this trait cannot inspect or retract what it has emitted, which
/// is what makes it safe to point at a sink that has already drained.
pub trait TextOut: fmt::Write {
    /// Append `text`.
    fn push_str(&mut self, text: &str);

    /// Append one character.
    fn push(&mut self, ch: char);

    /// Whether the destination has failed and is discarding further fragments.
    ///
    /// Emitters poll this at their innermost loop so a dead drain costs one
    /// fragment of formatting rather than a whole document. Always `false` for an
    /// in-memory target, which cannot fail.
    fn failed(&self) -> bool {
        false
    }
}

impl TextOut for String {
    #[inline]
    fn push_str(&mut self, text: &str) {
        Self::push_str(self, text);
    }

    #[inline]
    fn push(&mut self, ch: char) {
        Self::push(self, ch);
    }
}

impl TextOut for TextSink<'_> {
    #[inline]
    fn push_str(&mut self, text: &str) {
        Self::push_str(self, text);
    }

    #[inline]
    fn push(&mut self, ch: char) {
        Self::push(self, ch);
    }

    #[inline]
    fn failed(&self) -> bool {
        Self::failed(self)
    }
}

/// Where a full staging window goes.
///
/// Called once per [`DRAIN_BUFFER_BYTES`] (or once per oversized fragment), so a
/// trait object here costs nothing measurable — which is precisely what lets the
/// implementors below exist without putting an indirect call on an emitter's hot
/// path.
pub trait ByteDrain {
    /// Accept one window of bytes.
    ///
    /// An implementor must consume `chunk` entirely or report a failure; there is
    /// no short-write protocol, because a sink cannot re-offer bytes it has
    /// already released from its window.
    fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError>;
}

impl<T: ByteDrain + ?Sized> ByteDrain for &mut T {
    fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError> {
        (**self).drain(chunk)
    }
}

/// A [`ByteDrain`] over any [`std::io::Write`].
///
/// The adapter that makes an ordinary writer — a file, a locked stdout, a
/// `Vec<u8>` — the destination of a streamed serialization. It is one implementor
/// among several rather than the drain seam itself, because several of this
/// workspace's real destinations (a JavaScript callback, a C function pointer, a
/// Python file object) are not `io::Write`.
///
/// `write_all` is used rather than `write`, so a short write is retried by the
/// standard library rather than silently truncating the document.
#[derive(Debug)]
pub struct WriterDrain<W>(pub W);

impl<W: std::io::Write> ByteDrain for WriterDrain<W> {
    fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError> {
        self.0.write_all(chunk).map_err(|error| {
            let kind = if error.kind() == std::io::ErrorKind::BrokenPipe {
                DrainErrorKind::BrokenPipe
            } else {
                DrainErrorKind::Other
            };
            DrainError::new(kind, error.to_string())
        })
    }
}

/// A [`ByteDrain`] that counts bytes and retains none of them.
///
/// The length of a document without the document. Exactly equal to the length of
/// the bytes the same emitter would produce into a [`TextSink::in_memory`],
/// because both drive the same walker — this one just never keeps the bytes. That
/// makes a size bound decidable BEFORE emission, rather than by rendering the very
/// thing the bound exists to avoid rendering.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Measure(u64);

impl Measure {
    /// A fresh counter.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Bytes counted so far.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.0
    }
}

impl ByteDrain for Measure {
    fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError> {
        self.0 = self.0.saturating_add(chunk.len() as u64);
        Ok(())
    }
}

/// A [`ByteDrain`] that forwards to two drains in order.
///
/// Writing a document while digesting it, in one pass over the emitter, rather
/// than materializing it and walking the bytes twice.
#[derive(Debug)]
pub struct Tee<A, B>(pub A, pub B);

impl<A: ByteDrain, B: ByteDrain> ByteDrain for Tee<A, B> {
    fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError> {
        self.0.drain(chunk)?;
        self.1.drain(chunk)
    }
}

/// A bounded staging window over an optional [`ByteDrain`].
///
/// See the module documentation for the memory bound this does and does not
/// achieve, and for why [`finish`](Self::finish) must always be consulted.
pub struct TextSink<'a> {
    /// Bytes not yet handed to `drain`. For an in-memory sink this IS the
    /// document.
    ///
    /// INVARIANT: `staged.len() < drain_at` on entry to and exit from every
    /// method.
    staged: Vec<u8>,
    /// INVARIANT: `drain.is_none()` exactly when `drain_at == usize::MAX`.
    drain: Option<&'a mut dyn ByteDrain>,
    /// The staged length at or above which the window is drained. `usize::MAX`
    /// for an in-memory sink, whose window is never drained and therefore grows —
    /// making the overflow comparison permanently false and the in-memory push
    /// exactly `Vec::extend_from_slice` plus two arithmetic operations.
    drain_at: usize,
    /// Bytes accepted before the first failure. Stops advancing once `error` is
    /// set, so a size check reading it never counts discarded bytes as delivered.
    written: u64,
    /// The FIRST drain failure. Sticky; every later push is discarded rather than
    /// staged, so the memory bound survives an emitter that ignores it.
    error: Option<DrainError>,
    #[cfg(test)]
    discarded_pushes: usize,
}

impl fmt::Debug for TextSink<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSink")
            .field("staged_len", &self.staged.len())
            .field("draining", &self.drain.is_some())
            .field("written", &self.written)
            .field("failed", &self.error.is_some())
            .finish()
    }
}

impl<'a> TextSink<'a> {
    /// A sink that accumulates the whole document in memory.
    ///
    /// The eager spelling: the same emitter, the same fragments, no drain.
    #[must_use]
    pub const fn in_memory() -> Self {
        Self {
            staged: Vec::new(),
            drain: None,
            drain_at: usize::MAX,
            written: 0,
            error: None,
            #[cfg(test)]
            discarded_pushes: 0,
        }
    }

    /// [`in_memory`](Self::in_memory) with the document buffer pre-sized.
    #[must_use]
    pub fn with_capacity(bytes: usize) -> Self {
        Self {
            staged: Vec::with_capacity(bytes),
            ..Self::in_memory()
        }
    }

    /// A sink that stages at most [`DRAIN_BUFFER_BYTES`] and hands full windows to
    /// `drain`.
    #[must_use]
    pub fn to_drain(drain: &'a mut dyn ByteDrain) -> Self {
        Self {
            staged: Vec::with_capacity(DRAIN_BUFFER_BYTES),
            drain: Some(drain),
            drain_at: DRAIN_BUFFER_BYTES,
            written: 0,
            error: None,
            #[cfg(test)]
            discarded_pushes: 0,
        }
    }

    /// Append `text`.
    ///
    /// Infallible by contract; see the module documentation on failure.
    #[inline]
    pub fn push_str(&mut self, text: &str) {
        // Check BEFORE appending, so the window invariant cannot be crossed even
        // transiently. Appending first and draining after would bound the buffer
        // at `DRAIN_BUFFER_BYTES + longest_single_push`, which is a function of
        // the document rather than a constant.
        if self.staged.len() >= self.drain_at.saturating_sub(text.len()) {
            self.overflow(text.as_bytes());
            return;
        }
        self.staged.extend_from_slice(text.as_bytes());
    }

    /// Append one character.
    #[inline]
    pub fn push(&mut self, ch: char) {
        let mut buffer = [0_u8; 4];
        self.push_str(ch.encode_utf8(&mut buffer));
    }

    /// Append raw bytes.
    ///
    /// For emitters that produce UTF-8 through a byte-oriented writer rather than
    /// as `&str` fragments.
    #[inline]
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        if self.staged.len() >= self.drain_at.saturating_sub(bytes.len()) {
            self.overflow(bytes);
            return;
        }
        self.staged.extend_from_slice(bytes);
    }

    /// Whether a drain has failed.
    ///
    /// Emitters poll this at their innermost emission loop so a dead drain costs
    /// one fragment of formatting, not a document.
    #[must_use]
    pub const fn failed(&self) -> bool {
        self.error.is_some()
    }

    /// Bytes accepted before the first failure.
    #[must_use]
    pub const fn written(&self) -> u64 {
        self.written
    }

    /// Drain everything staged and yield the document.
    ///
    /// The sole terminal. For a draining sink the returned vector is empty — the
    /// bytes went to the drain. Does NOT flush the drain's own downstream: the
    /// owner of that writer owns its buffering.
    ///
    /// # Errors
    ///
    /// The FIRST [`DrainError`] the sink saw, whether it occurred here or at any
    /// earlier push.
    #[must_use = "a drain failure is reported only here; discarding this Result discards the failure"]
    pub fn finish(mut self) -> Result<Vec<u8>, DrainError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if let Some(drain) = self.drain.as_mut() {
            if !self.staged.is_empty() {
                drain.drain(&self.staged)?;
                self.written = self.written.saturating_add(self.staged.len() as u64);
                self.staged.clear();
            }
            return Ok(Vec::new());
        }
        Ok(self.staged)
    }

    /// The slow path: the window cannot take `bytes` without crossing its bound.
    #[cold]
    #[inline(never)]
    fn overflow(&mut self, bytes: &[u8]) {
        if self.error.is_some() {
            // Discard rather than stage. This is what keeps the bound honest for
            // an emitter that never polls `failed`.
            #[cfg(test)]
            {
                self.discarded_pushes += 1;
            }
            return;
        }
        let Some(drain) = self.drain.as_mut() else {
            // In-memory: `drain_at` is `usize::MAX`, so arriving here means the
            // document would exceed the address space. Let the allocator decide.
            self.staged.extend_from_slice(bytes);
            return;
        };
        if !self.staged.is_empty() {
            if let Err(error) = drain.drain(&self.staged) {
                self.error = Some(error);
                self.staged.clear();
                return;
            }
            self.written = self.written.saturating_add(self.staged.len() as u64);
            self.staged.clear();
        }
        if bytes.len() >= self.drain_at {
            // Larger than the whole window: hand it straight through from the
            // caller's slice. Nothing is copied, and the window stays empty.
            if let Err(error) = drain.drain(bytes) {
                self.error = Some(error);
                return;
            }
            self.written = self.written.saturating_add(bytes.len() as u64);
            return;
        }
        self.staged.extend_from_slice(bytes);
    }

    #[cfg(test)]
    fn staged_len(&self) -> usize {
        self.staged.len()
    }

    #[cfg(test)]
    fn staged_capacity(&self) -> usize {
        self.staged.capacity()
    }

    #[cfg(test)]
    const fn discarded_pushes(&self) -> usize {
        self.discarded_pushes
    }
}

impl fmt::Write for TextSink<'_> {
    /// Appends `s`.
    ///
    /// Always reports success, including for bytes discarded after a sticky drain
    /// failure — the failure is reported by [`TextSink::finish`]. This is what
    /// lets `write!` be used against a sink without threading a second error
    /// channel through every emitter.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A drain that records every window it was handed.
    #[derive(Debug, Default)]
    struct Recorder {
        windows: Vec<usize>,
        total: u64,
    }

    impl ByteDrain for Recorder {
        fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError> {
            self.windows.push(chunk.len());
            self.total += chunk.len() as u64;
            Ok(())
        }
    }

    /// A drain that fails on its `fail_on`-th window.
    #[derive(Debug)]
    struct FailAfter {
        fail_on: usize,
        seen: usize,
    }

    impl ByteDrain for FailAfter {
        fn drain(&mut self, _chunk: &[u8]) -> Result<(), DrainError> {
            self.seen += 1;
            if self.seen >= self.fail_on {
                return Err(DrainError::new(DrainErrorKind::BrokenPipe, "closed"));
            }
            Ok(())
        }
    }

    /// The staging window never exceeds its bound and never reallocates, for any
    /// combination of push size and push count — including a single push larger
    /// than the whole window, which an append-then-drain sink would fail.
    #[test]
    fn staging_window_is_bounded_independently_of_document_size() {
        for push_len in [1_usize, 63 << 10, 64 << 10, 1 << 20] {
            for pushes in [1_usize, 10, 1_000] {
                let text = "x".repeat(push_len);
                let mut recorder = Recorder::default();
                let mut sink = TextSink::to_drain(&mut recorder);
                for _ in 0..pushes {
                    sink.push_str(&text);
                    assert!(
                        sink.staged_len() < DRAIN_BUFFER_BYTES,
                        "window {} >= bound for push_len {push_len}",
                        sink.staged_len()
                    );
                    assert_eq!(
                        sink.staged_capacity(),
                        DRAIN_BUFFER_BYTES,
                        "window reallocated for push_len {push_len}"
                    );
                }
                let rest = sink.finish().expect("recorder never fails");
                assert!(rest.is_empty(), "a draining sink yields no document");
                assert_eq!(
                    recorder.total,
                    (push_len * pushes) as u64,
                    "every byte reached the drain"
                );
            }
        }
    }

    /// The eager and streamed spellings produce identical bytes, because they are
    /// the same emitter.
    #[test]
    fn eager_and_streamed_agree_byte_for_byte() {
        let fragments = ["<a> ", "<b> ", "\"c\"", " .\n", &"z".repeat(200_000)];

        let mut eager = TextSink::in_memory();
        for fragment in fragments {
            eager.push_str(fragment);
        }
        let eager = eager.finish().expect("in-memory never fails");

        let mut collected: Vec<u8> = Vec::new();
        struct Collect<'a>(&'a mut Vec<u8>);
        impl ByteDrain for Collect<'_> {
            fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError> {
                self.0.extend_from_slice(chunk);
                Ok(())
            }
        }
        let mut collect = Collect(&mut collected);
        let mut streamed = TextSink::to_drain(&mut collect);
        for fragment in fragments {
            streamed.push_str(fragment);
        }
        assert_eq!(streamed.finish().expect("collect never fails"), [] as [u8; 0]);

        assert_eq!(eager, collected);
    }

    /// `Measure` reports exactly the length the same fragments produce in memory.
    #[test]
    fn measure_equals_the_length_of_the_document_it_never_keeps() {
        let fragments = ["alpha", "", "\u{4e2d}\u{6587}", &"q".repeat(100_000)];

        let mut eager = TextSink::in_memory();
        for fragment in fragments {
            eager.push_str(fragment);
        }
        let document = eager.finish().expect("in-memory never fails");

        let mut measure = Measure::new();
        {
            let mut sink = TextSink::to_drain(&mut measure);
            for fragment in fragments {
                sink.push_str(fragment);
            }
            assert_eq!(sink.finish().expect("measure never fails"), [] as [u8; 0]);
        }
        assert_eq!(measure.bytes(), document.len() as u64);
    }

    /// After a drain failure nothing further is staged, so the bound survives an
    /// emitter that never polls `failed`, and the failure surfaces at `finish`.
    #[test]
    fn a_failed_drain_is_sticky_and_discards_rather_than_staging() {
        let mut drain = FailAfter {
            fail_on: 1,
            seen: 0,
        };
        let big = "y".repeat(DRAIN_BUFFER_BYTES);
        let mut sink = TextSink::to_drain(&mut drain);
        sink.push_str(&big);
        assert!(sink.failed(), "the first window failed");
        for _ in 0..50 {
            sink.push_str(&big);
            assert_eq!(sink.staged_len(), 0, "nothing is staged after failure");
        }
        assert_eq!(sink.discarded_pushes(), 50);
        assert_eq!(sink.written(), 0, "no bytes were delivered");
        let error = sink.finish().expect_err("the failure is reported here");
        assert_eq!(error.kind(), DrainErrorKind::BrokenPipe);
    }

    /// `Tee` feeds both drains the same bytes in one pass.
    #[test]
    fn tee_feeds_both_drains() {
        let mut measure_a = Measure::new();
        let mut measure_b = Measure::new();
        {
            let mut tee = Tee(&mut measure_a, &mut measure_b);
            let mut sink = TextSink::to_drain(&mut tee);
            sink.push_str(&"t".repeat(150_000));
            assert_eq!(sink.finish().expect("measures never fail"), [] as [u8; 0]);
        }
        assert_eq!(measure_a.bytes(), 150_000);
        assert_eq!(measure_b.bytes(), measure_a.bytes());
    }

    /// `push` and `push_bytes` agree with `push_str` across a window boundary.
    #[test]
    fn character_and_byte_pushes_cross_the_window_boundary_intact() {
        let mut collected: Vec<u8> = Vec::new();
        struct Collect<'a>(&'a mut Vec<u8>);
        impl ByteDrain for Collect<'_> {
            fn drain(&mut self, chunk: &[u8]) -> Result<(), DrainError> {
                self.0.extend_from_slice(chunk);
                Ok(())
            }
        }
        let mut collect = Collect(&mut collected);
        {
            let mut sink = TextSink::to_drain(&mut collect);
            // Land the cursor one byte short of the window, then straddle it with
            // a multi-byte character.
            sink.push_str(&"a".repeat(DRAIN_BUFFER_BYTES - 1));
            sink.push('\u{4e2d}');
            sink.push_bytes(b"tail");
            assert_eq!(sink.finish().expect("collect never fails"), [] as [u8; 0]);
        }
        let mut expected = "a".repeat(DRAIN_BUFFER_BYTES - 1).into_bytes();
        expected.extend_from_slice("\u{4e2d}".as_bytes());
        expected.extend_from_slice(b"tail");
        assert_eq!(collected, expected);
    }
}

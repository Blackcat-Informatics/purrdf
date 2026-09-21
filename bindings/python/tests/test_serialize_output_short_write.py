# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""`serialize(output=...)` writes the whole document, whatever `write` accepts.

`write` on a Python file object is not obliged to consume everything it is handed.
`io.RawIOBase.write` returns the number of bytes actually written and may return less
than it was given — a raw unbuffered file on a full pipe is the ordinary case — and
any object satisfying the file-like protocol may do the same.

The binding called `write` once and discarded the count, so a sink that accepted less
than the whole document produced a SHORT FILE and no error. That is the silent drop
this package refuses on every other seam, reached through the one seam that hands
bytes to code it does not control.

A sink that cannot make progress must fail loudly instead, which is the other half:
looping until everything is written is only safe if "wrote nothing" terminates.
"""

from __future__ import annotations

import pytest

import purrdf


def _result() -> object:
    """A CONSTRUCT result large enough that a chunking sink takes many turns."""
    store = purrdf.Store()
    for index in range(2_000):
        store.add(
            purrdf.Quad(
                purrdf.NamedNode(f"https://example.org/s{index}"),
                purrdf.NamedNode("https://example.org/p"),
                purrdf.Literal(f"value {index} padded out so the document is sizeable"),
            )
        )
    return store.query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")


class ChunkingSink:
    """Accepts at most `limit` bytes per call, like a raw file on a busy pipe."""

    def __init__(self, limit: int) -> None:
        self.limit = limit
        self.buffer = bytearray()
        self.calls = 0

    def write(self, data: bytes) -> int:
        self.calls += 1
        taken = min(self.limit, len(data))
        self.buffer += data[:taken]
        return taken


class StalledSink:
    """Never accepts anything. Looping on this must terminate with an error."""

    def write(self, data: bytes) -> int:  # noqa: ARG002
        return 0


class SilentSink:
    """Returns `None`, as `io.TextIOBase` and many hand-rolled sinks do."""

    def __init__(self) -> None:
        self.buffer = bytearray()

    def write(self, data: bytes) -> None:
        self.buffer += data


def test_a_short_writing_sink_still_receives_the_whole_document() -> None:
    result = _result()
    whole = purrdf.serialize(result, None, purrdf.RdfFormat.N_TRIPLES)
    assert len(whole) > 100_000, "the fixture must be big enough to be chunked"

    sink = ChunkingSink(limit=4096)
    purrdf.serialize(result, sink, purrdf.RdfFormat.N_TRIPLES)  # type: ignore[func-returns-value]

    assert sink.calls > 1, "the sink must actually have been called more than once"
    assert bytes(sink.buffer) == whole, (
        "a sink that accepts part of each write must still receive every byte; "
        f"got {len(sink.buffer)} of {len(whole)}"
    )


def test_a_sink_that_never_progresses_raises_instead_of_looping() -> None:
    result = _result()
    with pytest.raises(ValueError, match="would not make progress"):
        purrdf.serialize(result, StalledSink(), purrdf.RdfFormat.N_TRIPLES)  # type: ignore[func-returns-value]


def test_a_sink_returning_none_is_taken_at_its_word() -> None:
    """The neighbour the short-write loop must not break.

    `None` means "consumed it all" for a large family of real sinks. Treating it as a
    count would make every one of them look like a zero-byte write and turn a working
    call into the error above.
    """
    result = _result()
    whole = purrdf.serialize(result, None, purrdf.RdfFormat.N_TRIPLES)

    sink = SilentSink()
    purrdf.serialize(result, sink, purrdf.RdfFormat.N_TRIPLES)  # type: ignore[func-returns-value]
    assert bytes(sink.buffer) == whole


def test_a_sink_returning_a_non_count_is_refused() -> None:
    """A `write` whose return cannot be checked is a refusal, not a guess.

    Guessing is how the truncation this test file exists for became invisible in the
    first place.
    """

    class WeirdSink:
        def write(self, data: bytes) -> str:  # noqa: ARG002
            return "ok"

    result = _result()
    with pytest.raises(TypeError, match="number of bytes written"):
        purrdf.serialize(result, WeirdSink(), purrdf.RdfFormat.N_TRIPLES)  # type: ignore[func-returns-value]

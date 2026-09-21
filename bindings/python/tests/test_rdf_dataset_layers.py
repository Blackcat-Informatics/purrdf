# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""What an ``RdfDataset`` counts, and which layer each row was sorted into.

``RdfDataset`` is the frozen, columnar dataset the GTS writer and the projections
run over, and building one from bytes is where a document is sorted into layers:
ordinary quads into the base quad table, and the RDF 1.2 statement layer —
reifier bindings and their annotations — into tables of their own.

The sorting is the part that can go wrong quietly. A build that classified an
``rdf:reifies`` row as an ordinary base quad would still parse, still serialize,
and still round-trip through GTS; every text-level assertion about the document
would pass. The only thing that changes is the count, so the count is what is
asserted here — including that it is **zero** where a document holds nothing but a
statement layer, which is the assertion a presence check cannot make.

``reifiers`` and ``annotations`` have no accessor on ``RdfDataset`` itself, so the
statement-layer counts are read where Python can reach them: through
``to_gts()`` and the fold view that reads it back, which is also the only place
the GTS round trip's own fidelity is visible from here.
"""

from __future__ import annotations

import purrdf
from purrdf import RdfDataset, RdfFormat

EX = "https://example.org/"
RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

#: Two ordinary quads over three distinct subjects/predicates/objects.
PLAIN_ROWS = f'<{EX}s> <{EX}p> <{EX}o> .\n<{EX}s> <{EX}p2> "lit" .\n'

#: A reifier binding plus one annotation on it, and NOTHING else: the base quad
#: table must come out empty.
STATEMENT_LAYER_ONLY = (
    f"<{EX}r> <{RDF_REIFIES}> <<( <{EX}s> <{EX}p> <{EX}o> )>> .\n"
    f'<{EX}r> <{EX}confidence> "0.9" .\n'
)

#: One ordinary quad, for the fold-back case.
ONE_ROW = f"<{EX}s> <{EX}p> <{EX}o> .\n"


def test_an_rdf_dataset_counts_the_quads_and_the_terms_it_interned() -> None:
    """Two rows are two quads, and the term table holds every distinct term.

    ``term_count`` is the interner's own size, and it is the number that says the
    dataset really is columnar rather than a list of rows wearing an interface: a
    term repeated across rows is interned once, so two three-term rows are backed
    by fewer than six entries plus whatever the literal's datatype adds.
    """
    dataset = RdfDataset(PLAIN_ROWS, RdfFormat.N_TRIPLES)

    assert dataset.quad_count() == 2
    assert len(dataset) == dataset.quad_count(), (
        "`len` and `quad_count` are one number, not two"
    )
    # s, p, o, p2, "lit", and the literal's `xsd:string` datatype IRI. The
    # subject appears in both rows and is interned once.
    assert dataset.term_count() == 6

    # Sharing is what the number measures, so the contrast is executed: two rows
    # that share nothing intern strictly more than two rows that share a subject.
    shared = RdfDataset(
        f"<{EX}s> <{EX}p> <{EX}o> .\n<{EX}s> <{EX}p2> <{EX}o2> .\n",
        RdfFormat.N_TRIPLES,
    )
    disjoint = RdfDataset(
        f"<{EX}s> <{EX}p> <{EX}o> .\n<{EX}s2> <{EX}p2> <{EX}o2> .\n",
        RdfFormat.N_TRIPLES,
    )
    assert shared.quad_count() == disjoint.quad_count() == 2
    assert shared.term_count() == 5, "five distinct IRIs across the two rows"
    assert disjoint.term_count() == 6, "six, because nothing is shared"


def test_a_reifier_row_is_the_statement_layer_and_not_a_base_quad() -> None:
    """A document of nothing but a statement layer has ZERO base quads.

    This is the assertion no presence check can make. A build that filed the
    ``rdf:reifies`` row and its annotation as ordinary quads would still serialize
    both, still round-trip both, and still satisfy every "is `reifies` in the
    output" test — while reporting a base quad count of two where RDF 1.2 says
    there are none, and handing every consumer of the quad table two rows that
    are not statements about the world.
    """
    dataset = RdfDataset(STATEMENT_LAYER_ONLY, RdfFormat.N_TRIPLES)
    assert dataset.quad_count() == 0, "a reifier binding is not a base quad"

    # The rows are not lost, they are elsewhere — read back through the one path
    # Python has to the statement layer.
    view = purrdf.GtsFoldViewNative.from_bytes(dataset.to_gts())
    assert view.reifier_count() == 1, "one reifier binding, carried"
    assert view.annotation_count() == 1, "one annotation on it, carried"
    assert view.quad_count() == 0, "and still no base quad"

    # The neighbouring document that IS two base quads counts two, so the zero
    # above is about the layer and not about the counter.
    plain = RdfDataset(PLAIN_ROWS, RdfFormat.N_TRIPLES)
    assert plain.quad_count() == 2
    plain_view = purrdf.GtsFoldViewNative.from_bytes(plain.to_gts())
    assert plain_view.quad_count() == 2
    assert plain_view.reifier_count() == 0
    assert plain_view.annotation_count() == 0


def test_a_dataset_written_to_gts_folds_back_to_the_rows_it_held() -> None:
    """The GTS round trip is lossless at the row level, and deterministic.

    Writing and reading back is the pair of operations a container has to survive
    for the format to be worth anything, and the count is what says it did: a
    writer that emitted the term dictionary and dropped the quad table would
    produce a readable container carrying nothing.
    """
    dataset = RdfDataset(ONE_ROW, RdfFormat.N_TRIPLES)
    data = dataset.to_gts()

    view = purrdf.GtsFoldViewNative.from_bytes(data)
    assert view.quad_count() == dataset.quad_count() == 1
    assert view.reifier_count() == 0

    # The same dataset written twice is the same bytes: the writer is a function
    # of content, never of a clock or an iteration order.
    assert dataset.to_gts() == data

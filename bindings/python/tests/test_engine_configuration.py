# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The per-call engine configuration a query kwarg installs.

``Store.query`` builds a fresh evaluator per call, and three of its keywords
change what the SPARQL **parser** and evaluator will accept:
``extension_namespaces``, ``property_fn_namespaces``, and
``standpoint_predicates``. A kwarg that was read but never threaded through would
be invisible: the call still succeeds, the query still runs, and the stricter
reading the caller asked for simply never happens.

So each one is pinned by the behaviour it changes, and pinned from both sides.

* **``extension_namespaces`` buys a parse-time reading.** Unset, an IRI in call
  position is an ordinary custom function and the query parses — whether or not
  this build can evaluate it. Declared, an unknown local name under that
  namespace is a hard error *at parse time*, which is the whole value of
  declaring it: a typo in a function name is caught before a row is touched
  rather than after.
* **``standpoint_predicates`` is required and never defaulted.** ``heldIn`` is
  evaluated against the ontology's own ``accordingTo``/``sharpens`` IRIs. PurRDF
  mints no vocabulary, so there is no pair to fall back on, and the refusal says
  exactly that rather than answering under a guess.

The diagnostic **code** is asserted, not just the failure, because the two
failure modes here are separated by nothing else: an unevaluable function and an
unparseable query are both a ``ValueError`` carrying a message, and only the code
says which happened.
"""

from __future__ import annotations

import pytest

import purrdf

EX = "https://example.org/"

#: A namespace a host might declare its extension functions under. It is the
#: HOST's, invented by this test for its own fixture.
FN_NS = f"{EX}fn/"
EXT_NS = f"{EX}ext/"

#: A filter calling an IRI in call position. The local name `nope` is registered
#: nowhere, which is the point: it is the shape a typo takes.
UNKNOWN_CALL = f"ASK {{ FILTER(<{FN_NS}nope>(1)) }}"

#: The shared diagnostic code for a SPARQL parse failure. One identity across
#: every surface, so a test may name it.
PARSE_CODE = "native-sparql-query-parse"

#: The code for a call-position IRI this build cannot evaluate. Distinct from the
#: parse code, and that distinctness is what the first pair of tests turns on.
UNSUPPORTED_CODE = "native-sparql-custom-function"

#: `heldIn` under a declared extension namespace, and the pair of predicates a
#: host supplies for it.
HELD_IN = (
    f"ASK {{ FILTER(<{EXT_NS}heldIn>(<{EX}r>, <{EX}s>)) }}"
)
STANDPOINT = (f"{EX}accordingTo", f"{EX}sharpens")


def test_an_undeclared_call_position_iri_is_an_ordinary_custom_function() -> None:
    """With no namespace declared, the query PARSES; only evaluation objects.

    This is the reading every pre-existing call keeps, and it has to stay
    reachable: a host that has not declared an extension namespace is writing
    standard SPARQL, where an IRI in call position is a perfectly legal extension
    function the processor may or may not know. Turning that into a parse error
    would refuse conformant queries.
    """
    with pytest.raises(ValueError) as refused:
        purrdf.Store().query(UNKNOWN_CALL)
    message = str(refused.value)
    assert UNSUPPORTED_CODE in message, (
        f"an unknown call-position IRI is an unevaluable function: {message}"
    )
    assert PARSE_CODE not in message, (
        f"…and never a parse failure with no namespace declared: {message}"
    )


def test_a_declared_extension_namespace_makes_an_unknown_local_name_a_parse_error() -> None:
    """Declaring the namespace is what buys the earlier, stricter refusal.

    The two codes are the whole assertion. Both readings raise ``ValueError``
    carrying a message, so "it raised" is satisfied by the undeclared reading too
    — and a kwarg that was accepted and dropped would leave exactly that.
    """
    with pytest.raises(ValueError) as refused:
        purrdf.Store().query(UNKNOWN_CALL, extension_namespaces=[FN_NS])
    message = str(refused.value)
    assert PARSE_CODE in message, (
        f"a declared namespace moves the refusal to parse time: {message}"
    )
    assert f"{FN_NS}nope" in message, "…and the refusal names the function"

    # The neighbouring valid case: declaring the namespace does not refuse
    # queries that never mention it.
    assert (
        bool(purrdf.Store().query("ASK { ?s ?p ?o }", extension_namespaces=[FN_NS]))
        is False
    )


def test_held_in_requires_the_hosts_standpoint_predicates_and_answers_with_them() -> None:
    """The pair is supplied or the call is refused; it is never invented.

    ``accordingTo`` and ``sharpens`` are terms of the caller's ontology. PurRDF
    mints no vocabulary IRIs, so there is no default pair, and a built-in one
    would silently evaluate every standpoint query against a vocabulary nobody
    uses. The refusal names the configuration it needs; the neighbouring call
    that supplies it answers.
    """
    store = purrdf.Store()

    with pytest.raises(ValueError, match="standpoint predicate configuration") as refused:
        store.query(HELD_IN, extension_namespaces=[EXT_NS])
    message = str(refused.value)
    assert "no built-in default" in message, (
        f"the refusal says there is nothing to fall back on: {message}"
    )

    # Supplied: the same query evaluates, over an empty store, to `false` — which
    # is an ANSWER, and is what proves the pair reached the evaluator rather than
    # being accepted and dropped.
    answered = store.query(
        HELD_IN, extension_namespaces=[EXT_NS], standpoint_predicates=STANDPOINT
    )
    assert isinstance(answered, purrdf.QueryBoolean)
    assert bool(answered) is False, (
        "an empty store holds nothing in any standpoint, and saying so is an "
        "answer rather than a refusal"
    )

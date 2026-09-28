# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The shapes-graph tools beside validation: ``purrdf.shapes.apply_rules``,
``purrdf.shapes.check_rules``, ``purrdf.shapes.eval_node_expr`` and
``purrdf.shapes.lint_shapes``.

Every shapes graph carries the W3C SHACL 1.2 declaration of
``sh:SPARQLExprExpression`` verbatim -- a built-in declared as a
``sh:NamedParameterExpressionFunction`` with no ``sh:bodyExpression``, which is not a
bodiless custom function -- beside shapes that call ``sh:sparqlExpr`` with
``sh:prefixes``.
"""

from __future__ import annotations

import pytest

import purrdf

_SNIPPET = '''
sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .
'''

_PREFIXES = """
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .
"""

# A sh:sparqlExpr node naming ex:yes through sh:prefixes, a labelled shnex:var
# node, a rule tagging every ex:Item through the same expression, and a counter
# rule stepping ex:n to 5 -- exactly four term-generating rounds.
_TOOLS = '''
ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/ns#"^^xsd:anyURI ] .
ex:Tag sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes .
_:suffix shnex:var "suffix" .

ex:Tagger a sh:NodeShape ;
  sh:targetClass ex:Item ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:tagged ;
            sh:object [ sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes ] ] .

ex:Counter a sh:NodeShape ;
  sh:targetSubjectsOf ex:n ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """PREFIX ex: <http://example.org/ns#>
CONSTRUCT { $this ex:n ?m } WHERE { $this ex:n ?k . FILTER(?k < 5) BIND(?k + 1 AS ?m) }""" ] .
'''

_SHAPES = _PREFIXES + _SNIPPET + _TOOLS

_DATA = (
    "<http://example.org/ns#a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/ns#Item> .\n"
    '<http://example.org/ns#a> <http://example.org/ns#n> "1"^^<http://www.w3.org/2001/XMLSchema#integer> .\n'
)

_INTEGER = "<http://www.w3.org/2001/XMLSchema#integer>"

_INFERRED = "".join(
    f'<http://example.org/ns#a> <http://example.org/ns#n> "{n}"^^{_INTEGER} .\n' for n in range(2, 6)
) + ("<http://example.org/ns#a> <http://example.org/ns#tagged> <http://example.org/ns#yes> .\n")


def test_py_apply_rules() -> None:
    out = purrdf.shapes.apply_rules(_DATA, _SHAPES)
    assert out == {"inferred": _INFERRED, "proof": None, "diagnostics": []}

    explained = purrdf.shapes.apply_rules(_DATA, _SHAPES, explain=True)
    assert explained["inferred"] == _INFERRED
    proof = explained["proof"]
    assert isinstance(proof, str)
    assert proof.count("derived ") == 5
    assert (
        "derived <http://example.org/ns#a> <http://example.org/ns#tagged> "
        "<http://example.org/ns#yes> .\n  rule _:" in proof
    )

    # The round limit: four term-generating rounds are needed. A passed limit names
    # this host's own keyword argument.
    with pytest.raises(ValueError) as rounds:
        purrdf.shapes.apply_rules(_DATA, _SHAPES, max_term_generating_rounds=3)
    assert "past the limit of 3 (the caller's limit)" in str(rounds.value)
    assert str(rounds.value).endswith(
        "raise the limit with apply_rules(max_term_generating_rounds=...)"
    )
    assert purrdf.shapes.apply_rules(_DATA, _SHAPES, max_term_generating_rounds=4)["inferred"] == _INFERRED

    # The generated-term budget: the rule set adds six terms.
    with pytest.raises(ValueError) as terms:
        purrdf.shapes.apply_rules(_DATA, _SHAPES, max_generated_terms=5)
    assert "past the budget of 5 (the caller's budget)" in str(terms.value)
    assert str(terms.value).endswith(
        "raise the budget with apply_rules(max_generated_terms=...)"
    )
    assert purrdf.shapes.apply_rules(_DATA, _SHAPES, max_generated_terms=6)["inferred"] == _INFERRED

    # The stored-fact and join-step limits name this host's own keyword arguments, and
    # the neighbour holding exactly the store the run needs completes.
    with pytest.raises(ValueError) as facts:
        purrdf.shapes.apply_rules(_DATA, _SHAPES, max_stored_facts=1)
    assert "the rules exceeded the stored-fact limit: " in str(facts.value)
    assert "1 permitted (the caller's limit)" in str(facts.value)
    assert str(facts.value).endswith("raise it with apply_rules(max_stored_facts=...)")
    limit = 1
    while True:
        try:
            ran_within = purrdf.shapes.apply_rules(_DATA, _SHAPES, max_stored_facts=limit)
        except ValueError as refused:
            observed = str(refused).split("the rules exceeded the stored-fact limit: ")[1]
            limit = int(observed.split(" ")[0])
            continue
        break
    assert ran_within["inferred"] == _INFERRED
    with pytest.raises(ValueError):
        purrdf.shapes.apply_rules(_DATA, _SHAPES, max_stored_facts=limit - 1)
    with pytest.raises(ValueError) as steps:
        purrdf.shapes.apply_rules(_DATA, _SHAPES, max_join_steps=1)
    assert "the rules exceeded the join-step limit: " in str(steps.value)
    assert str(steps.value).endswith("raise it with apply_rules(max_join_steps=...)")
    assert purrdf.shapes.apply_rules(_DATA, _SHAPES, max_join_steps=1_000_000)["inferred"] == _INFERRED

    # SPARQL 1.2 RL text through the same function.
    srl = (
        "PREFIX ex: <http://example.org/ns#>\n"
        "RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n"
        "DATA { ex:d ex:q 2 }\n"
    )
    ran = purrdf.shapes.apply_rules(_DATA, srl=srl, explain=True)
    assert ran["inferred"] == (
        f'<http://example.org/ns#a> <http://example.org/ns#q> "1"^^{_INTEGER} .\n'
        f'<http://example.org/ns#d> <http://example.org/ns#q> "2"^^{_INTEGER} .\n'
    )
    assert "  data-block\n" in ran["proof"]
    assert f'  premise <http://example.org/ns#a> <http://example.org/ns#n> "1"^^{_INTEGER} .\n' in ran["proof"]

    with pytest.raises(ValueError, match="two rule sources"):
        purrdf.shapes.apply_rules(_DATA, _SHAPES, srl=srl)
    with pytest.raises(ValueError, match="no rule source"):
        purrdf.shapes.apply_rules(_DATA)


def test_py_entail_limits() -> None:
    """``shapes.entail`` takes the four rule-evaluation limits ``apply_rules`` takes; each
    refusal names ``entail``'s own keyword, and the raised run materializes the closure."""
    for keyword in (
        "max_term_generating_rounds",
        "max_generated_terms",
        "max_stored_facts",
        "max_join_steps",
    ):
        with pytest.raises(ValueError) as refused:
            purrdf.shapes.entail(_SHAPES, _DATA, **{keyword: 1})
        assert str(refused.value).endswith(f"entail({keyword}=...)"), str(refused.value)
        assert "RuleOptions::" not in str(refused.value)
    closed = purrdf.shapes.entail(
        _SHAPES,
        _DATA,
        max_term_generating_rounds=64,
        max_generated_terms=64,
        max_stored_facts=64,
        max_join_steps=4096,
    )
    for line in _INFERRED.splitlines():
        assert line in closed["ntriples"], closed
    assert closed == purrdf.shapes.entail(_SHAPES, _DATA)


def test_py_eval_node_expr() -> None:
    assert purrdf.shapes.eval_node_expr(
        _SHAPES, _DATA, "http://example.org/ns#Tag", "http://example.org/ns#a"
    ) == ["<http://example.org/ns#yes>"]
    assert purrdf.shapes.eval_node_expr(
        _SHAPES,
        _DATA,
        "_:suffix",
        f'"-3"^^{_INTEGER}',
        scope={"suffix": '"!"@en'},
    ) == ['"!"@en']
    with pytest.raises(ValueError, match="mentions no blank node _:nosuch"):
        purrdf.shapes.eval_node_expr(_SHAPES, _DATA, "_:nosuch", "http://example.org/ns#a")
    with pytest.raises(ValueError, match="can never be read"):
        purrdf.shapes.eval_node_expr(
            _SHAPES, _DATA, "_:suffix", "http://example.org/ns#a", scope={"focusNode": '"!"'}
        )


_SH = "http://www.w3.org/ns/shacl#"


def test_py_eval_node_expr_selectors() -> None:
    """An anonymous expression is named by a walk from ex:Tagger, or inline as Turtle
    whose sh:prefixes resolves in the shapes graph; each refusal sits beside a valid
    neighbour."""
    focus = "http://example.org/ns#a"
    yes = ["<http://example.org/ns#yes>"]
    assert (
        purrdf.shapes.eval_node_expr(
            _SHAPES,
            _DATA,
            None,
            focus,
            expr_at="http://example.org/ns#Tagger",
            expr_via=[_SH + "rule", _SH + "object"],
        )
        == yes
    )
    assert (
        purrdf.shapes.eval_node_expr(
            _SHAPES,
            _DATA,
            None,
            focus,
            expr_turtle='[ sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes ] .',
        )
        == yes
    )
    # One value beside two.
    assert purrdf.shapes.eval_node_expr(
        _SHAPES,
        _DATA,
        None,
        focus,
        expr_at=_SH + "SPARQLExprExpression",
        expr_via=["http://www.w3.org/2000/01/rdf-schema#isDefinedBy"],
    ) == [f"<{_SH}>"]
    with pytest.raises(ValueError, match="reaches 2 values"):
        purrdf.shapes.eval_node_expr(
            _SHAPES,
            _DATA,
            None,
            focus,
            expr_at=_SH + "SPARQLExprExpression",
            expr_via=[_SH + "parameter"],
        )
    with pytest.raises(ValueError, match="has 2 root blank nodes"):
        purrdf.shapes.eval_node_expr(
            _SHAPES, _DATA, None, focus, expr_turtle='[ shnex:var "a" ] . [ shnex:var "b" ] .'
        )
    with pytest.raises(ValueError, match="2 of the expression node"):
        purrdf.shapes.eval_node_expr(
            _SHAPES, _DATA, "http://example.org/ns#Tag", focus, expr_turtle='[ shnex:var "a" ] .'
        )
    with pytest.raises(ValueError, match="0 of the expression node"):
        purrdf.shapes.eval_node_expr(_SHAPES, _DATA, None, focus)
    with pytest.raises(ValueError, match="no walk start"):
        purrdf.shapes.eval_node_expr(
            _SHAPES, _DATA, "http://example.org/ns#Tag", focus, expr_via=[_SH + "rule"]
        )


def test_py_lint_shapes() -> None:
    clean = purrdf.shapes.lint_shapes(_SHAPES)
    assert clean["clean"] is True
    assert clean["findings"] == 0
    assert clean["load_error"] is None
    assert clean["shacl_shacl"] == []
    assert {
        "binding": "native",
        "function": "http://www.w3.org/ns/shacl#SPARQLExprExpression",
        "owner": "sh:rule on <http://example.org/ns#Tagger>",
    } in clean["calls"]
    assert clean["alternatives"] == []
    assert clean["unexecuted"] == []
    assert clean["unanchored_imports"] == []
    assert clean["report"].endswith(
        "validators 0\nunexecuted 0\ndiagnostics 0\nunanchored-imports 0\nfindings 0\nclean true\n"
    )

    malformed = purrdf.shapes.lint_shapes(
        _PREFIXES + _SNIPPET + 'ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount "one" ] .\n'
    )
    assert malformed["clean"] is False
    assert malformed["findings"] >= 2
    assert isinstance(malformed["load_error"], str)
    assert malformed["calls"] is None
    assert malformed["alternatives"] is None
    assert malformed["unexecuted"] is None
    assert any(
        result["path"] == "<http://www.w3.org/ns/shacl#minCount>" and result["superseded"] is None
        for result in malformed["shacl_shacl"]
    )

    # An owl:imports on a node that is no anchor is data: listed, never a finding.
    unanchored = purrdf.shapes.lint_shapes(_SHAPES + "ex:Other <http://www.w3.org/2002/07/owl#imports> ex:Target .\n")
    assert unanchored["unanchored_imports"] == [
        {
            "document": None,
            "subject": "<http://example.org/ns#Other>",
            "object": "<http://example.org/ns#Target>",
        }
    ]
    assert unanchored["clean"] is True
    assert unanchored["findings"] == 0

    by_types = purrdf.shapes.lint_shapes(_PREFIXES + _SNIPPET + "ex:S a sh:NodeShape ; sh:closed sh:ByTypes .\n")
    assert by_types["clean"] is True
    assert [result["superseded"] for result in by_types["shacl_shacl"]] == ["closed-by-types"]

    with pytest.raises(ValueError):
        purrdf.shapes.lint_shapes("@@@ not turtle")


def test_py_lint_shapes_lists_superseded_builtin_validators() -> None:
    # A validator declared for a built-in component is an alternative the native
    # implementation supersedes: listed, never run, never a finding.
    report = purrdf.shapes.lint_shapes(
        _PREFIXES
        + "sh:MinCountConstraintComponent a sh:ConstraintComponent ; sh:validator ex:neverValid .\n"
        + 'ex:neverValid a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (false) }" .\n'
        + "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:n ; sh:minCount 1 ] .\n"
    )
    assert report["load_error"] is None, report["report"]
    assert report["alternatives"] == [
        {
            "component": "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
            "attachment": "http://www.w3.org/ns/shacl#validator",
            "validator": "<http://example.org/ns#neverValid>",
            "language": "sparql-ask",
        }
    ]
    assert (
        "alternative <http://www.w3.org/ns/shacl#MinCountConstraintComponent> "
        "<http://www.w3.org/ns/shacl#validator> <http://example.org/ns#neverValid> "
        "sparql-ask superseded-by-native\n"
    ) in report["report"]


_SRL_IMPORTING = (
    "PREFIX ex: <http://example.org/ns#>\n"
    "IMPORTS <http://example.org/more>\n"
    "RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n"
)
_SRL_IMPORTED = "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:counted true } WHERE { ?x ex:q ?y }\n"
_SRL_LONE = "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n"
_SRL_UNRESOLVED = (
    "SPARQL 1.2 RL import <http://example.org/more> failed: no import-table entry supplies "
    "the rule set it names, and PurRDF fetches nothing it was not handed; supply that rule "
    "set's text under this IRI"
)
_SRL_UNREACHED = (
    "the SPARQL 1.2 RL rule set's import closure never reaches <http://example.org/more>, so "
    "the import table's rule set would be read and never used; remove it"
)
_COUNTED = (
    "<http://example.org/ns#a> <http://example.org/ns#counted> "
    '"true"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n'
)


def test_py_apply_rules_resolves_srl_imports_from_the_table() -> None:
    ran = purrdf.shapes.apply_rules(
        _DATA, srl=_SRL_IMPORTING, imports=[("http://example.org/more", _SRL_IMPORTED)]
    )
    assert _COUNTED in ran["inferred"], ran["inferred"]

    with pytest.raises(ValueError) as unresolved:
        purrdf.shapes.apply_rules(_DATA, srl=_SRL_IMPORTING)
    assert str(unresolved.value) == _SRL_UNRESOLVED

    with pytest.raises(ValueError) as unreached:
        purrdf.shapes.apply_rules(
            _DATA, srl=_SRL_LONE, imports=[("http://example.org/more", _SRL_IMPORTED)]
        )
    assert str(unreached.value) == _SRL_UNREACHED
    assert _COUNTED not in purrdf.shapes.apply_rules(_DATA, srl=_SRL_LONE)["inferred"]


_SRL_CYCLIC = (
    "PREFIX ex: <http://example.org/ns#>\n"
    "RULE { ?x ex:p ex:z } WHERE { ?x ex:q ex:o NOT { ?x ex:p ex:z } }\n"
)
_SRL_ACYCLIC = (
    "PREFIX ex: <http://example.org/ns#>\n"
    "RULE { ?x ex:p ex:z } WHERE { ?x ex:q ex:o NOT { ?x ex:r ex:z } }\n"
)


def test_py_check_rules() -> None:
    """``check_rules`` checks a SPARQL 1.2 RL rule set to a level and evaluates nothing:
    the import table resolves ``IMPORTS``, each level answers its own question, and a
    refusal names its stage beside the neighbour that passes."""
    checked = purrdf.shapes.check_rules(
        _SRL_IMPORTING, imports=[("http://example.org/more", _SRL_IMPORTED)]
    )
    assert checked["level"] == "stratified"
    assert len(checked["rules"]) == 2
    assert checked["data_triples"] == 0
    assert checked["imported"] == ["http://example.org/more"]
    assert checked["versions"] == []
    assert checked["strata"] == 1
    assert checked["summary"] == (
        "SPARQL 1.2 RL rule set is well formed and stratified (level stratified): 2 rules, "
        "0 data triples, 1 imported rule set, 1 stratum, no VERSION"
    )
    # The same unresolved import a rules run refuses, with the same message.
    with pytest.raises(ValueError) as unresolved:
        purrdf.shapes.check_rules(_SRL_IMPORTING)
    assert str(unresolved.value) == _SRL_UNRESOLVED
    with pytest.raises(ValueError) as unreached:
        purrdf.shapes.check_rules(
            _SRL_LONE, imports=[("http://example.org/more", _SRL_IMPORTED)]
        )
    assert str(unreached.value) == _SRL_UNREACHED

    for level in ("syntax", "well-formed"):
        below = purrdf.shapes.check_rules(_SRL_CYCLIC, level=level)
        assert below["level"] == level
        assert below["strata"] is None
    with pytest.raises(ValueError, match="is not stratifiable"):
        purrdf.shapes.check_rules(_SRL_CYCLIC)
    assert purrdf.shapes.check_rules(_SRL_ACYCLIC)["strata"] == 1
    with pytest.raises(ValueError, match="^SPARQL 1.2 RL syntax error"):
        purrdf.shapes.check_rules("RULE {", level="syntax")
    with pytest.raises(ValueError, match="is not a SPARQL 1.2 RL check level"):
        purrdf.shapes.check_rules(_SRL_ACYCLIC, level="stratify")

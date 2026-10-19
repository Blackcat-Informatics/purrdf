# T19 triage: rule-set references and `validate_change` under entailment (G-09, G-10)

## Definition of "rule set" (G-09)

SHACL 1.2 Inference Rules (ED, "Entailment using Rule Sets"): "Any rule set can be used
as a value of sh:entailment, i.e. any IRI that is a SHACL instance of sh:RuleSet in the
shapes graph." "Rule Sets": "All SHACL instances of sh:RuleSet have an IRI."

`shacl.ttl` gives `sh:hasRule` and `sh:includesRuleSet` `rdfs:domain sh:RuleSet`, and
`sh:includesRuleSet` `rdfs:range sh:RuleSet`. SHACL does no RDFS inference, so a domain
declaration does not make an untyped subject an instance. Decision: a rule set is a
typed SHACL instance of `sh:RuleSet`, for both readers (`parse_rule_sets` and
`parse_entailment`). Before, `parse_rule_sets` also took any subject of `sh:hasRule` or
`sh:includesRuleSet`, so an untyped subject was a rule set to `RuleOptions::with_rule_set`
and an unsupported regime to `sh:entailment`. An untyped subject of `sh:hasRule` still
loads; its members remain global rules (`sh:hasRule`: "those values are rules").

## Include closure

Every `sh:includesRuleSet` value must be a declared rule set (range `sh:RuleSet`). This is
checked at load in `parse_rule_sets`, which `parse_rule_graph` always runs, so it holds
whichever regimes are named, including beside `sh:RulesEntailment` (which never walked
includes). Cycles are allowed: the text says rule sets "(transitively) include" others;
the closure is a set (`rule_set_members` keeps a seen-list). Tested, entering from
either member.

## validate_change (G-10)

With any `sh:entailment` declared, `entailed_for_validation` replaces the bound data with
the entailed dataset, which has no mutation snapshot. `affected_focus_raw` now answers
`FocusExpansion::Everything { reason: "rules entailment" }` first, so `validate_change`
reports `ChangeScope::Everything` and runs the full validation. A rule can derive or
retract triples anywhere from any read it makes, so no bounded superset exists. The
shapes graph without a declaration stays bounded (tested as the neighbour).

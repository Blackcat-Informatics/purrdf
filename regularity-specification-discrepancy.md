# Role regularity: published condition versus automaton theorem

Status: source evidence established; acceptance-policy adjustment proposed to the independent reviewer. No runtime or completion claim.

The [OWL 2 Structural Specification §11.2](https://www.w3.org/TR/owl2-syntax/#Global_Restrictions_on_Axioms_in_OWL_2_DL) asks for a strict order opposing no reversed simple-hierarchy path. It does not close strict comparisons through forward simple inclusions. Its axiom closure (§3.4) collects imported axioms with anonymous individuals standardized apart; it does not add inferred role-chain substitutions.

Our independent stress input is:

```text
a x <= b
y c <= d
b <= c
d <= a
```

The printed condition permits strict edges a<b, x<b, y<d, c<d and their inverse-source partners. None opposes a reversed simple path. But substitution yields y^n a x^n below a, with matched counts on opposite sides. A finite path scan or cyclic NFA shortcut would be unsound.

[Giorgio Stefanoni, Oxford DPhil thesis (2015), Chapter 12](https://www.cs.ox.ac.uk/files/7941/paper.pdf), Example 12.2, independently establishes the published condition's failure to guarantee regular role languages. Definition 12.3 proposes a dependency condition that accounts for simple inclusions; Theorem 12.4 establishes a semantics-preserving normalization into the original regular calculus. Pure simple cycles remain admissible. The original automaton decision theorem cannot justify acceptance merely from the printed OWL condition.

Proposed implementation separates the printed source-order check from theorem-backed automaton admission. It reports precisely which condition failed. The policy and public documentation must state the discrepancy explicitly; it must not masquerade as the literal W3C condition or silently broaden consistency answers. The implementation still owes all nine real GMEOW chains and every accepted regular case in the full issue contract.

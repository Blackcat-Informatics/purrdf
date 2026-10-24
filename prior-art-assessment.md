# Reuse the existing owners

The parser already owns a per-parse InFlight set distinguishing shape and node
expression domains. Reuse it, but cover property parsing from its entry rather
than only nested-property expansion after constraint parsing. Its two empty
shape substitutions and nested edge skip are the current implicit cycle policy.
The conformance memo stores completed answers keyed by node/shape/depth; it is
not an active-cycle detector. Its depth component preserves current exhaustion
behavior and must not be deleted to make a cycle look memoizable.

purrdf-stack is already a shapes dev dependency and the workspace stack home.
Promote/use that existing first-party edge for production admission instead of
writing a second stack detector or changing stack sizes/tooling. Existing
never_panic, custom node-expression and foreign-value controls provide actual
public test entry points. Extend them without a second validation engine.
No external engine code or implementation is copied. The sheval paper informs
explicit behavior/test categories; it does not supply an implementation.

Owner assessment only. Hosted independent review remains required before merge.

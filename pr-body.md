Both native xsd:string parse entry points accepted characters outside the XML character domain modeled by the datatype calculus. They now validate before allocating, preserving valid text verbatim and retaining typed invalid-literal errors. AnyURI reuses the same borrowed validator; its duplicate validation implementation is removed.

The XSD crate suite, warnings-as-errors all-target clippy, three actual WebAssembly text-primitive tests, and the full entailment library suite (718 passed, one ignored) pass. Invalid-input accounting confirms rejection before allocation. Full hosted CI remains separate.

Closes #575.

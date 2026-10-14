v4.0: make exact numerics the only public XSD tower

Remove the bounded public integer and decimal variants and propagate the exact
value path through SPARQL and every host binding. Default division computes a
terminating quotient exactly; a repeating quotient keeps 18 fractional digits
rounded half to even. Preserve explicit exact and fixed-scale policies. Store
temporal seconds exactly, advance governor profile and frozen vectors to v18,
and update Breaking, book, and zh-Hans guidance.

Defect-Class: API and numeric semantics
Closes #470

Qualification and integration notes must be finalized from the current PR head
before these notes are used for ghprsq. #443 kernels are outside this change.

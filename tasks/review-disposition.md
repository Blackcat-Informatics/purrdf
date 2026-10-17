The provenance finding is valid. The working tree now states that the public
API exposes keyed hashing, key derivation and extended output, while PurRDF
content identities continue to use unkeyed BLAKE3-256. This correction will be
published with the generated assembly counts; the inline thread remains open
until that commit is pushed.

The separate docstring coverage warning does not identify a missing public API
contract. Every added public constructor, function and reader method has
Rustdoc, and strict missing_docs compilation passes. Its 83-function denominator
also includes existing private helpers, test functions and inherited trait
methods whose signatures changed during mode propagation. I am retaining the
public-documentation requirement and existing bot configuration, rather than
adding blanket private/test boilerplate to satisfy this heuristic. A specific
missing public contract would still need a correction.

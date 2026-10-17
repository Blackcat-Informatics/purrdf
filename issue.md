# Issue #575: Validate the native xsd:string XML character domain at admission

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The new finite length-space comparison correctly counts the XML Char alphabet, but both xsd:string parse entry points currently accept every Rust string, including NUL and U+FFFE. Such accepted values contradict the modeled datatype domain. Enforce the XSD 1.1 string lexical domain through the shared XML terminal before allocation in ordinary and admitted parsing; preserve valid string whitespace verbatim. Reuse the same lexical validator for anyURI, retain typed lexical refusal and allocation accounting, and exercise both text primitives natively and on WASM. This is needed by the ongoing downstream anyURI/native-value cutover, not a deferred risk. Normative definitions: https://www.w3.org/TR/xmlschema11-2/#string and https://www.w3.org/TR/rdf12-concepts/#xsd-datatypes

## Comments (0)


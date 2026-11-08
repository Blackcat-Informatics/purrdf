# Introduction

The YAML-LD Test Suite is a set of tests that can
be used to verify YAML-LD Processor conformance to the set of specifications
that constitute YAML-LD. The goal of the suite is to provide an easy and
comprehensive YAML-LD testing solution for developers creating YAML-LD Processors.

More information and an RDFS definition of the test vocabulary can be found at [vocab](https://w3c.github.io/json-ld-api/tests/vocab).

# Design

Tests are driven from top-level [manifest](manifest.jsonld). Tests have _input_, _expected_ and may have _context_ or _frame_ documents.

Tests may have a `expandContext` option, which is treated
  as an IRI relative to the manifest.

The _expected_ results with file extension `.jsonld` or `.yamlld` can be compared using [YAML-LD object comparison](#yaml-ld-object-comparison) with the processor output. Additionally, if the `ordered` option is not set, the result should be expanded and compared with the expanded _expected_ document also using [YAML-LD object comparison](#yaml-ld-object-comparison).

Test results that include a context input presume that the context is provided locally, and not from the referenced location, thus the results will include the content of the context file, rather than a reference.

Tests of type `jld:ToRDFTest` have an _expected_ file extension of `.nq`. Their results can be compared using [RDF Dataset Isomorphism](https://www.w3.org/TR/rdf11-concepts/#dfn-dataset-isomorphism).

A **PositiveSyntaxTest** looks specifically for syntax-related issues. A **PositiveSyntaxTest** succeeds when no error is found when processing.

A **NegativeEvaluationTests** looks for a string value in _expectedErrorCode_, representing the error code resulting from running the test.

## YAML-LD object comparison

Similar to JSON-LD object comparison.

If algorithms are invoked with the `ordered` flag set to `true`, simple JSON Object comparison may be used, as the order of all arrays will be preserved (except for _fromRdf_, unless the input quads are also ordered). If `ordered` is `false`, then the following algorithm will ensure arrays other than values of `@list` are compared without regard to order.

YAML-LD object comparison recursively compares JSON objects, arrays, and values for equality.

* JSON objects are compared member by member without regard to the ordering of members within the object. Each member must have a corresponding member in the object being compared to. Values are compared recursively.
* JSON arrays are generally compared without regard to order (the lone exception being if the referencing key is `@list`). Each item within the array must be equivalent to an item in the array being compared to by recursively using the comparison algorithm. For values of `@list`, the order of these items is significant.
* JSON values are compared using strict equality.
* Values of `@language`, and other places where language tags may be used, are specified in lowercase in the test results. Implementations should either normalize language tags for testing purposes, or compare language tags in a case-independent way.

Note that some tests require re-expansion and comparison, as list values may exist as values of properties that have `@container: @list` and the comparison algorithm will not consider ordering significant.

## Updates to the JSON-LD Test Vocabulary

The following properties are treated as if they are defined in the [JSON-LD Test Vocabulary](https://w3c.github.io/json-ld-api/tests/vocab), but are not yet defined.

* `expectRE`: One or more regular expressions used on a `PositiveSyntax`. In addition to parsing properly, the result must match all listed [regular expressions](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_Expressions/Cheatsheet).
* `req`: The type of requirement, one of **must**, **should**, or **may**. If unspecified, the normative statement being tested is taken as **must**.

# Running tests

The top-level [manifest](manifest.jsonld) references each test associated with a particular type of behavior.

Implementations create their own infrastructure for running the test suite. In particular, the following should be considered:

* Test case properties identifying a file (_input_, _output_, _context_, _expectContext_, and _frame_) are presumed to have a media type appropriate for the file extension.
  * `application/ld+json` for `.jsonld`
  * `application/ld+yaml` for `.yamlld`
  * `application/n-quads` for `.nq`
* The media type for the file associated with the _input_ property can be overridden using the `contentType` option.
* Some algorithms, particularly _fromRdf_, may not preserve the order of statements listed in the input document, and provision should be made to perform unordered array comparison, for arrays other than values of `@list`. (This may be difficult for compacted results, where array value ordering is dependent on the associated term definition).
* Some _toRdf_ tests require the use of [JSON Canonicalization Scheme](https://tools.ietf.org/html/draft-rundgren-json-canonicalization-scheme-05) to properly generate RDF Literals from JSON literal values. This algorithm is to be used to properly compare results using [RDF Dataset Isomorphism](https://www.w3.org/TR/rdf11-concepts/#dfn-dataset-isomorphism). These tests are marked using the `useJCS` option.
* Some _toRdf_ tests may generate results as [Generalized RDF](https://www.w3.org/TR/rdf11-concepts/#section-generalized-rdf), specifically having a blank node predicate. Technically, these are in an invalid N-Quads format; provision must be made to parse and recognize quads having a blank node predicate.
* When comparing documents after flattening, framing or generating RDF, blank node identifiers may not be predictable. Implementations should take this into consideration. (One way to do this may be to reduce both results and _expected_ to datasets to extract a bijective mapping of blank node labels between the two datasets as described in [RDF Dataset Isomorphism](https://www.w3.org/TR/rdf11-concepts/#dfn-dataset-isomorphism)).
* Some tests may have a `requires` property, indicating some optional behavior described by a test vocabulary term.
* Tests with `normative: false` in their options are excluded from compliance requirements; they verify informative (Extended Profile) behavior. Implementations may skip these when asserting conformance to the YAML-LD standard.

# Building HTML Manifests

The HTML versions of the test manifests are built using the Node action `npm run generate`, which also cross-indexes tests with references from the specification.

# Contributing

If you would like to contribute a new test or a fix to an existing test,
please follow these steps:

1. Notify the JSON-LD Community Group that you will be creating
   a new test or fix and the purpose of the change
   by [creating an issue](https://github.com/json-ld/yaml-ld/issues) on the [GitHub Repository](https://github.com/json-ld/yaml-ld),
   or by sending a message to the group mailing list, [public-linked-json@w3.org](mailto:public-linked-json@w3.org).
2. Clone the git repository: git://github.com/json-ld/yaml-ld-wg.git
3. Make your changes on a separate branch and submit them via a [GitHub Pull Request](https://github.com/json-ld/yaml-ld/pulls), or via a 'git format-patch'
   to the [JSON-LD Community Group mailing list](mailto:public-linked-json@w3.org).

# Distribution
  Distributed under the [W3C Test Suite License](http://www.w3.org/Consortium/Legal/2008/04-testsuite-license). To contribute to a W3C Test Suite, see the [policies and contribution forms](http://www.w3.org/2004/10/27-testcases).

# Disclaimer
  UNDER THE EXCLUSIVE LICENSE, THIS DOCUMENT AND ALL DOCUMENTS, TESTS AND SOFTWARE THAT LINK THIS STATEMENT ARE PROVIDED "AS IS," AND COPYRIGHT HOLDERS MAKE NO REPRESENTATIONS OR WARRANTIES, EXPRESS OR IMPLIED, INCLUDING, BUT NOT LIMITED TO, WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, NON-INFRINGEMENT, OR TITLE; THAT THE CONTENTS OF THE DOCUMENT ARE SUITABLE FOR ANY PURPOSE; NOR THAT THE IMPLEMENTATION OF SUCH CONTENTS WILL NOT INFRINGE ANY THIRD PARTY PATENTS, COPYRIGHTS, TRADEMARKS OR OTHER RIGHTS.
  COPYRIGHT HOLDERS WILL NOT BE LIABLE FOR ANY DIRECT, INDIRECT, SPECIAL OR CONSEQUENTIAL DAMAGES ARISING OUT OF ANY USE OF THE DOCUMENT OR THE PERFORMANCE OR IMPLEMENTATION OF THE CONTENTS THEREOF.

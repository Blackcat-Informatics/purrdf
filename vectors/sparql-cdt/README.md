# Test Suite for the SPARQL-CDT Approach
This directory contains contains a test suite with a comprehensive collection of tests that developers of RDF and SPARQL systems can use to check whether their implementation of the different features of the SPARQL-CDTs approach is correct. The test suite includes tests:
* for [the FOLD aggregate](https://w3id.org/awslabs/neptune/SPARQL-CDTs/spec/latest.html#fold) (these tests are in the [fold](https://github.com/awslabs/SPARQL-CDTs/tree/main/tests/fold) directory),
* for [the UNFOLD operator](https://w3id.org/awslabs/neptune/SPARQL-CDTs/spec/latest.html#unfold) (these tests are in the [unfold](https://github.com/awslabs/SPARQL-CDTs/tree/main/tests/unfold) directory),
* for [the functions for cdt:List literals](https://w3id.org/awslabs/neptune/SPARQL-CDTs/spec/latest.html#list-functions) (these tests are in the [list-functions](https://github.com/awslabs/SPARQL-CDTs/tree/main/tests/list-functions) directory),
* for [the functions for cdt:Map literals](https://w3id.org/awslabs/neptune/SPARQL-CDTs/spec/latest.html#map-functions) (these tests are in the [map-functions](https://github.com/awslabs/SPARQL-CDTs/tree/main/tests/map-functions) directory), and
* for [the corresponding extension of the ORDER BY operator](https://w3id.org/awslabs/neptune/SPARQL-CDTs/spec/latest.html#extension-of-order-by) (these tests are in the [orderby](https://github.com/awslabs/SPARQL-CDTs/tree/main/tests/orderby) directory).

The test suite is described in RDF using the [framework](https://www.w3.org/2001/sw/DataAccess/tests/README.html) that was defined by the
[RDF Data Access Working Group](https://www.w3.org/2001/sw/DataAccess/homepage-20080115) of the [W3C](https://www.w3.org/). Since this framework is integrated into the test harnesses of many RDF and SPARQL systems, providing the test suite in this form should make it easy for implementers to use these tests if they want to extend their systems with support for the SPARQL-CDTs approach.

To run the complete test suite (i.e., the tests in all of the subdirectories) use the `manifest-all.ttl` file in this directory. To run an individual set of tests use the `manifest.ttl` file in the corresponding subdirectory (as listed above).

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Segmented stream order and bounded construction

Numeric dictionary order within a segmented image does not establish a shared
row order across images. Bounded construction and a streaming merge must account
for both dictionary order and statement order.

## What the current image orders

`crates/rdf-core/src/ir/segmented/build.rs::encode_dataset` writes the forward
dictionary in retained numeric term-ID order. It then sorts the same records by
their encoded term bytes for the reverse dictionary. Each statement stream is
sorted separately by `(subject ID, predicate ID, object ID, graph ID)`.
`SegmentedBuildLimits::with_first_term_index` promises that numeric IDs survive
sealing and reopening; they are not reassigned in semantic value order.

These are orders within one image. They are not a common row order across images.
The stack's `intern_source_term` merges equal values into query-owned identities
as they are encountered. That translation need not preserve either source's
numeric order. Adding an image-specific dictionary offset cannot repair a
permutation of its terms.

## A persisted counterexample

The regression `persisted_layer_row_orders_disagree_after_shared_identity_translation`
in `crates/rdf-core/tests/segmented_stack.rs` seals and reopens two valid images.
Both contain exactly `urn:a urn:p urn:value` and `urn:z urn:p urn:value` in the
default graph. They differ only in insertion order.

| Image | Forward dictionary | Numeric subject order | Subject values in row order |
|---|---|---|---|
| First | `a, p, value, z` | `0, 3` | `a, z` |
| Second | `z, p, value, a` | `0, 3` | `z, a` |

Both physical row streams are increasing in their own numeric IDs. After the
first source establishes the stack identities, the translated subject sequences
are increasing in the first image and decreasing in the second. Equal rows still
translate to equal identities. The current visibility path correctly emits two
rows, with no operational failure.

No strict common comparator over these two distinct rows can make both streams
increasing: one requires `a < z`, the other `z < a`. Thus a direct k-way merge
with adjacent duplicate/removal suppression is incorrect for general existing
images. Giving a layer an arbitrary table of shared IDs does not sort its rows.
The reverse dictionary's independent ordering does not sort the statement
streams either.

## Construction and scan work must agree on the premise

A bounded writer may consume streams already sorted in the orders required by
its output image. That is a useful interface, but it cannot assert that arbitrary
existing images already provide those streams. In particular, merging their
source-local row IDs directly would silently change RDF identity.

The implementation needs to distinguish these jobs:

1. Emit sorted output streams without retaining every term, row or output block.
   Keep only the current blocks and required directory/statistics/authentication
   metadata. A caller-owned sink must receive completed blocks and preserve
   actionable write, storage and cancellation failures.
2. Produce those streams from arbitrary source images. Shared identity mapping
   can require reordering rows. A common-order index or sorted runs in a
   caller-owned spill sink is needed; a whole-output `Vec` or `Seen` replacement
   would retain the linear memory cost.
3. Merge additions and removals under the same order and retain original layer
   chronology. Annotation classification also depends on the original and
   effective reifier declarations, not just on duplicate row equality.

The writer also emits a reverse dictionary in a second order, and the directory
and statistics are completed after reading the inputs. Authentication includes
the completed header and every output block. A bounded output interface therefore
needs either a caller-owned seekable sink or an explicit host protocol that can
finish those records without retaining the image. Writing blocks eagerly while
keeping a second complete image to compute the authentication tree would leave
the original memory problem intact.

## Certification is part of the memory boundary

`SegmentedImage::certify` takes a resident `Vec<u8>`. Its content validator also
collects every forward dictionary key, tracks all reverse dictionary IDs, rebuilds
the RDF dataset and recomputes statistics from it. A writer that emits blocks
incrementally and then calls this constructor would still require memory
proportional to the complete image and decoded dataset.

The receipt is authority for complete selective reads, so checksums alone cannot
replace content validation. Bounded production must establish the same dictionary
agreement, term/reference and role checks, sorted stream coverage, summaries,
statistics, capability flags and expanded-term footprint bound while keeping its
read and write working sets bounded. A caller-supplied spill or seekable store can
hold the required cross-order records; those records cannot become an unaccounted
resident copy. Native and WebAssembly callers need the same failure and
cancellation protocol. No receipt may escape from an incomplete output.

Acceptance must compare complete RDF values, including graph and blank-node
identity, reifiers, annotations, removals and reinsertion. Memory evidence must
separate required metadata from retained input rows and completed output blocks.
The counterexample above remains a control for the order assumption throughout
that implementation work; it establishes no scan speedup or memory reduction.

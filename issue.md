# Issue #560: SegmentedBuilder: pack dictionary blocks by bytes, and size resident capacities as input arrives

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Two behaviours a consumer has to work around

**1. A dictionary block holds a fixed number of terms, and refuses the image when they do not
fit.** `SegmentedBuildLimits` takes `block_bytes` and `records_per_block`; when
`records_per_block` adjacent terms are long (triple terms over long IRIs, say), the encoded block
passes `block_bytes` and `seal()` returns `ConstructionLimit`
(`crates/rdf-core/src/ir/segmented/format.rs`, the `out.len() > block_bytes` check). Whether it
happens depends on which terms sort next to each other, so a caller cannot tell in advance.

Katamari's persisted vault graph hit it when merging layers. Lowering `records_per_block` to
what the longest term allows made every block hold fewer terms: about six per 4 KiB instead of
32. Its layers came to 1.3 GB for 200,000 claims, against 191 MB for the same claims in one image
at 4 KiB / 32. Katamari now seals at 4 KiB / 32 and retries at twice the block size whenever a
seal is refused, which is a guess at a layout, repeated.

**Request:** close a dictionary block when its next term would not fit in `block_bytes`, so a
block holds as many terms as fit, and `records_per_block` is an upper bound (or goes). A term
longer than a block on its own is the only case left to refuse, or to give a block of its own.

**2. `SegmentedBuilder::new` reserves its build limits before it accepts input.** It calls
`reserve_for_replay(terms, term_bytes, rows)` with the limits. A caller that passes the format's
own widths, to set no ceiling, reserves for about four billion terms and rows on every build: a
three-row image took 13-45 s to build in a debug build. Katamari now sizes the limits from the
rows it is about to push, which means estimating purrdf's private `value_bound` conservatively.

**Request:** keep the limits as limits, and grow the resident capacities as input arrives (or
take a separate capacity hint), so a caller can set no chosen ceiling without paying for one.

## Evidence

Katamari `crates/katamari-query-purrdf/src/layers.rs` (`build_limits`, `seal_image`), and its
assessment ledger's fix-list item 2.


## Comments (0)


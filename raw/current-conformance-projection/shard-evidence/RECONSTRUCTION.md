# Conformance shard evidence reconstruction

Run37817482468/attempt1/headed14f23ead7a303a18b9ef610f1e6caeb4483a35. Four original ZIPs match API digests. jobs-api.json/full jobs/*.raw.log retain all exact successful job identities. results/ contains original sole-member payloads, not reserialized JSON. path-map.tsv records original paths, stored paths, size, mode and SHA256; archive-readback.txt records38 successful file readbacks. Every copied file also passed full cmp to its owned original.

After /opt cleanup, copy exactly results/{core,sparql,shapes,python}.json into a fresh empty reconstruction directory. Verify their path-map SHA256 values; original results.sha256 preserves historical absolute paths. ZIP sole members equal scanned extracted JSONs byte-for-byte. Use the existing source-bound Make conformance --from-results generator/parity; commands and actual exits remain one directory above. No suites or binding rebuild is required for this reconstruction.

Source-delta.patch proves three cfg(test) initializer additions only between measureded14 and accepted6ce: owning generator, registry, corpus, budget and Python paths unchanged. Full API/job/run admission and32 schema/budget row observations retained. New final-head CI remains separate.

Credential token/private-key filename-only scans of original plaintext/JSON/API/job logs and selected archive found no hits. ZIP contents were scanned through their byte-identical extracted payloads. Initial xargs scan123 maps underlying rg no-match1; initial empty inventory and corrected per-file status admission retained. Retrieval-terminals.txt records initial CLI ANSI refusal and corrected actual0 full log retrieval. No generator or source mutation occurred during this evidence copy.

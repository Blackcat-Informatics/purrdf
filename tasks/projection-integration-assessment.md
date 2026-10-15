# Provisional integration assessment

The actual PR base is main c2c1c8c78 and the published head is a5224d262.
git merge-tree returns success with candidate tree
d5b2760f76052ffa9ce5f1c3f3b35da09fb806c7. Main's segmented-query delivery is
already integrated in this branch. The present publication changes nine generated
assembly cells only; production source, measurement requirements and manifest are
unchanged from the previously reviewed and qualified source.

The complete producer's generation, full seven-configuration qualification and
matching-report replay establish the exact generated cells on its original
49295bd27 source. The intervening 025daaed5 delta changes only the optimized
package JS test; current-head CI remains the separate final qualification for
the actual published source and complete report identity.

This is preparation while CI runs, not merge readiness. Refresh required checks,
feedback and actual base/head before invoking ghprsq. If main advances, reassess
the changed inputs and any shared capacity-proof, carrier or cache interactions;
two independently green branches are not a combined-source test.

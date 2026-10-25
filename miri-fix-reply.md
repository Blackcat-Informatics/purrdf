The cancelled Miri job exceeded its 30-minute deadline, as confirmed by the
check annotation. Its broad `small` filter selected the new canonicalization
stack case, spending over 15 minutes there before the intended storage cases.

The gate now targets `--lib small::tests::` under each existing model. All 18
SmallVec cases remain in every model, and the BLAKE3 probe is unchanged. The
complete corrected `make miri` gate passes locally: four sets of 18 storage
cases plus the streaming probe. No deadline or assertion was relaxed. The
semantic canonicalization case remains in normal native coverage.

# First-trial failure and actual caller correction

The first production default trial on551db4827 exited2 before graph acceptance:
scripts/lane-common.sh:89 required a second certificate description argument.
The shared home was correct; scripts/lubm-lane.sh:289 was its incorrect caller.
The failed run produced no graph acceptance and no query results. Its original
run/input artifacts remain preserved and dependent trials were stopped.

The one-line correction supplies "complete native graph acceptance". It does not
weaken the shared function, alter graph verification, or treat missing acceptance
as success. Inspection of the other lane_certify callers finds their descriptions
already supplied.

Actual root7745 exited0 for the existing lane_common_laws integration test
a_certificate_does_not_outlive_the_run_that_wrote_it: one PASS,34 filtered. The
actual log is T3-certificate-law.log. This demonstrates certificate lifecycle,
not the still-required full default/nondefault/fault campaign.

Normal signed commit83869/hooks exited0 atd526fc530; normal push65404 exited0.
The corrected production campaign remains PENDING. A retry with the retained
input cache cannot be called a cold acquisition; preserve the isolated task-owned
cache before any deliberately fresh acquisition and retain both failed-run
identities and input checksums.

#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The one definition of how many bytes a streamed read takes at a time.

Six copies of this number lived under two names across five files, and two of them
had already drifted -- one to a quarter of the shared size and one to a sixteenth.
It survives because every chunk size produces a correct digest: no run fails, no
test reddens, and the only symptom is that one site issues sixty-four times as
many reads as its siblings.

A constant restated is a constant that diverges, and these diverged in the
direction nothing measures.

So the number lives here, once, and everything that streams reads it from here.
Shell callers cannot import a module, so `lane-common.sh` reads this value once
when it is sourced and passes it into its embedded blocks as an argument;
`scripts/check-gate-parity.py`'s sibling `scripts/check-stream-chunk.py` refuses a
new unnamed literal in a streamed read under `scripts/`.

The value itself: 4 MiB. Large enough that a gigabyte-scale corpus is a few
hundred reads rather than a few thousand, small enough to stay well inside any
plausible page cache. Nothing in this repository is sensitive to the exact figure
— which is precisely why it must not be restated, because a divergence in a
number nothing depends on is a divergence nothing reports.
"""

STREAM_CHUNK_BYTES = 1 << 22

if __name__ == "__main__":
    # Shell callers: `python3 scripts/lane_chunk.py` prints the number and nothing
    # else, so a lane can capture it without parsing.
    print(STREAM_CHUNK_BYTES)

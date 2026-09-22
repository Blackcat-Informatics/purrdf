#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The laws every streaming writer here shares, in one definition each.

Two of them: how many bytes a read takes at a time, and how a file is made durable
before a rename promotes it. Both had multiple implementations that drifted or were
about to.

Six copies of this number lived under two names across five files, and two of them
had already drifted -- one to a quarter of the shared size and one to a sixty-fourth.
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

import os
from pathlib import Path

STREAM_CHUNK_BYTES = 1 << 22


def fsync_path(path: Path) -> None:
    """Flush *path*'s contents to the storage device, before a rename promotes it.

    Here for the same reason the chunk size is: it had three implementations. The
    acquirer grew `_fsync_path`, the cnSchema probe re-implemented it inline, and the
    WatDiv candidates cache wrote a third -- "the same law, in the same shape" being an
    accurate description of duplication in a repository that argues a restated constant
    diverges. Every one of them already imports this module.

    ``os.replace`` is atomic with respect to other processes the moment it returns; it
    says nothing about a power loss between the write and the rename reaching the disk,
    and the outcome there is a file under the destination's name whose contents were
    never the verified bytes.

    Opened read-only deliberately: the writer has closed its handle by the time a
    candidate is verified, and reopening for writing to force a flush would be a second
    chance to truncate the file being made durable. POSIX permits ``fsync`` on a
    read-only descriptor and Linux and macOS both honour it.
    """
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)

if __name__ == "__main__":
    # Shell callers: `python3 scripts/lane_chunk.py` prints the number and nothing
    # else, so a lane can capture it without parsing.
    print(STREAM_CHUNK_BYTES)

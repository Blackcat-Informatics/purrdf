Directory compaction is split into #708 as an independently useful delivery:
remove repeated per-row prefix populations/padding and use sparse authenticated
prefix checkpoints with bounded admitted reconstruction and independent
certification. This preserves exact source addresses, digest ordering and work
accounting. Oversized out-of-line dictionary terms remain in #705 as the next
separate unit. Neither closes #564's streaming construction/certification and
common-order scan requirements. Both parents remain open.

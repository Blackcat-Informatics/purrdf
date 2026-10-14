# Main integration and resident reconstruction proofs

Self source assessment of the changed inputs to the previously independently
reviewed forty reconstruction rows. The unchanged checker reports forty stale
source proofs, each still matching one original sink. No additional sink is
being admitted and no rule or proof class is changed.

The Rust index supplies constructor identity source:ea5e88b4a8a7cd5f, with the
same149 closure members as the retained qualified index: no member is added or
removed. Core production changes from10316f2c1 to the integrated head are only
mutable.rs and small.rs. small.rs adds the SharedView re-export; allocation.rs
likewise adds that re-export and changes no capacity-check body.

mutable.rs changes graph-declaration lifetime defaults and adds
freeze_with_limits. The new method obtains snapshot_view_with_limits, rebuilds
through the original checked builder/importer, and propagates both Result
failures before publication. It does not mint an unchecked local or global ID.
Remembered graph declarations use the same admitted term storage. The two rows
which name this file are visit_blank_identities: both enumerate the concrete
frozen base term table, whose private count and checked interner mint are
unchanged. The original immutable constructor bounds therefore still support
those reconstructions. Existing compaction-limit5-dimension controls qualify
their actual caller behavior; no scope is inferred for arbitrary trait views.

The explicit changed authority inputs are allocation.rs and mutable.rs. The
planned registry refresh copies only their actual identities and the new closure
identity from the Rust index. All forty sink identities, symbols, locations,
classes and theorem reasons remain unchanged. Their original independent source
review remains preserved; this delta assessment is not a fresh independent audit.
Run the unchanged helper gate after the refresh and keep the stale-proof failure
as historical evidence. No current zero-copy gate pass is claimed in this note.

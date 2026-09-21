<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Benchmark lane laws: what a lane must refuse, and what its digest actually certifies

Referenced from `scripts/lane-common.sh`, which implements these, and from
[BENCHMARKS.md](../BENCHMARKS.md), which documents the lanes they govern.

The comparison lanes — `scripts/scale-corpus.sh`, `scripts/lubm-lane.sh` and
`scripts/watdiv-lane.sh` — are report-only where it counts: **no gate runs a lane past
step 1**, and no timing or answer row is asserted anywhere, which is the right posture
for numbers that depend on the host.

Be exact about the boundary, because it has moved. `make check` does drive both lanes —
fourteen tests invoke `make lubm`/`make watdiv` — but every one stops at the binary
probe or a knob validator, because past that point a lane wants the network and a JDK.
And several printed numbers ARE now asserted, against pins: both corpus digests, both
query-set digests, two counts, and LUBM's published answers for Q1 and Q14.

What remains unguarded is everything a lane does from step 1 onward on a real corpus.
**A defect there is invisible to every gate here**: the lane will print a well-formed
report about nothing, and the only reader who can tell is one who already knew what the
number should be.

Everything below exists because of that asymmetry. A lane cannot be trusted to be
correct because it passed; it can only be trusted because it refuses.

## The parity rule, and why the thin wrappers are not duplication

`scripts/lane-common.sh` holds the laws every lane shares, in one
implementation, and its header states the rule: *a law that holds in one lane and
not its siblings is not a law*. The failure that produced it is on the record in
`crates/bench/tests/make_bench_lanes.rs` — one repair reached two lanes and left
the third carrying the defect, and the binary check had drifted in five places
between two copies of itself.

Two of the three lanes then define a handful of same-named helpers — `write_checked`,
`mkdir_checked`, `require_nonempty_file`. The first two are **adapters, not copies**:
their whole body delegates to the `lane_*` implementation, passing the one thing that
genuinely differs — which knob supplied the path — so the diagnostic can quote `LUBM_OUT`
or `WATDIV_OUT` back at the operator instead of emitting a bare `mkdir: cannot create
directory`. `require_nonempty_file` injects nothing and is a bare alias, kept only so the
three read alike at their call sites; nothing would be lost by calling the shared helper
directly.

`scale-corpus.sh` does it the other way — it calls `lane_write_checked` and
`lane_mkdir_checked` directly and passes the knob name inline at each site. Both forms
keep the knob name, which is the property that matters; the wrappers only save
repeating it. So the instruction is narrower than it first read: do not collapse a
wrapper into a `lane_*` call *and drop the knob argument*, because that is what makes
a path failure anonymous. An earlier version of this paragraph claimed collapsing them
would necessarily discard the knob name, which the third lane already disproves.

The laws that are genuinely shared, and therefore hold for every lane. This table is an
INVENTORY, not an illustration: a law added to `lane-common.sh` and not added here is a
law the next lane author will not find, and five rows were missing when that was last
checked — including the two the immediately preceding changes had created.

| law | shared implementation |
| --- | --- |
| A lane names itself in every diagnostic | `die` |
| Collation is pinned, so a caller's locale cannot reorder what a digest covers | `export LC_ALL=C` |
| One streaming implementation of a file digest, rather than one per lane | `lane_sha256_file` |
| A diagnostic survives the tab-separated record it travels in, instead of being truncated | `lane_flatten_detail` |
| A certificate never outlives the run it certifies | `lane_certify`, `lane_revoke_certificates` |
| Scratch space is created and removed by the lane, not the caller | `lane_cleanup` |
| A path that cannot be written names the knob that supplied it | `lane_write_checked`, `lane_mkdir_checked` |
| Existing is not being produced — an artifact must be a regular, non-empty file | `lane_require_nonempty_file` |
| Non-empty is not "is what it claims to be" | `lane_require_magic`, `lane_require_nquads` |
| The binary that certifies every number is itself checked before it is trusted — before step 1 when a `*_BIN` knob supplies it, and at the build step when the lane builds it | `lane_require_executable`, `lane_run_probe`, `lane_require_probe_said_something` |
| A knob value bash cannot carry intact is refused by name, never replaced — bash integers are signed 64-bit and wrap silently, so an all-digits knob could become a negative seed | `lane_require_uint`, `lane_require_positive` |
| A pin that is *recorded nowhere* and a pin *lookup that is broken* are different states, and only the first is reportable — both arrived as one non-zero status behind `2>/dev/null`, and a skip is indistinguishable from a pass | `lane_require_pin`, `lane_lookup_pin` |
| A capture path is proven writable before the redirect that depends on it, so a scratch failure is never published as a failure of the binary under test | `lane_reset_capture`, `lane_capture_stderr` |
| A file written outside the scratch directory still owes the trap its path, so buying independence from `LANE_TMP` does not cost the cleanup it provided | `lane_track_stray` |
| One definition of how many bytes a streamed read takes, and one of how a file is made durable before a rename promotes it | `scripts/lane_chunk.py` (`STREAM_CHUNK_BYTES`, `fsync_path`) |

## A digest is a certificate only if every input to it is pinned

A lane that prints `sha256(...)` and calls it the determinism check is making a
falsifiable promise: *these named inputs, and nothing else, reproduce these
bytes*. The promise is only as good as the enumeration. Any input the lane does
not name is a free variable, and a free variable turns the certificate into a
coincidence.

Ordering is the input that is easiest to forget, because it is usually supplied
by a tool rather than written down. `sort(1)` obeys `LC_COLLATE`: under `LC_ALL=C`
it compares bytes, so `.` (0x2E) precedes `0` (0x30) and `University0_1.owl`
sorts before `University0_10.owl`; under a UTF-8 collation punctuation is
ignorable at the primary level and the two reverse. A lane that concatenates in
`find | sort` order therefore produces a different corpus — and a different
digest, and a different one-file rung — on two hosts that differ only in their
environment. The fix is not to document the collation but to remove it as a
variable: order bytes, with `LC_ALL=C`, or order in a language whose sort is
defined (Python's `sorted` on `str` is code-point ordered and never locale-aware).

The same applies to any ordering that reaches an emitted artifact: a filesystem
glob, a `set` iteration, a dictionary built in insertion order. Collect in
whatever order is convenient, then canonicalise before anything downstream can
observe it.

## Container identity is not corpus identity

A digest-pinned archive proves what was *downloaded*. It proves nothing about
what is currently *extracted*, because extraction is a separate event with its
own failure modes: a truncated write, a manual edit, an interrupted run, a
half-finished experiment left in the arena.

So a reuse stamp keyed on the archive's digest answers the wrong question. It
says "an extraction from this tarball happened here once", and a later run reads
that as "the bytes in this arena are that tarball's contents". Between those two
statements sits every way the extracted file can have changed since. The
consequence is specific and bad: the lane queries whatever is there now and
reports its results beside the *archive's* pinned digest, which is a provenance
claim the bytes it measured do not have.

A reuse stamp must therefore be keyed on a digest of the artifact the lane is
about to read, not of the container it came from — and where the artifact's true
shape is a known constant of a pinned release, that constant belongs in a tracked
file next to the pin, asserted rather than printed.

## A count that is known must be asserted, not reported

`> 0` is the weakest possible guard and it is almost never the right one. When a
pinned artifact has a known shape — twenty query templates, fourteen queries — the
lane knows the expected value, so printing the observed one is a missed refusal.
The failure it admits is not "nothing happened", which is loud; it is "less
happened than should have", which reads as success and is fast.

The exception is worth stating precisely, because it is easy to misread as an
excuse. A count need not be asserted where **something strictly stronger already
fixes it**: a verified digest over the same bytes pins every count those bytes
have, so adding the count as a second literal states one fact in two places and
invites them to disagree. "A digest covers it" is only a valid answer when that digest is verified against a
**pin** on the path in question. A digest re-derived and compared against a stamp
the arena itself wrote is trust-on-first-use: it detects later change, which is
worth having, but it cannot detect a first extraction that was already wrong,
because that extraction is what wrote the record.

The WatDiv corpus digest IS pinned now, so the row count beside it is no longer
justified by that exception — and it is kept anyway for a different and better reason:
a count is a cheaper and far more legible refusal than a digest mismatch. "10,916,457
triples, expected 10,916,457" tells an operator what is wrong; two 64-character hex
strings tell them only that something is. Where both are available, assert both.

This law was written before the code obeyed it, and briefly licensed its own
violation — the row count was left reported on the argument that the corpus digest
subsumed it, when that digest then answered to a stamp rather than a pin. The rule is
the one to keep; the exception was the mistake.

This extends past artifact counts to the rows of the report itself. A lane that
proves it wrote twenty queries and then prints however many rows it managed to
read has verified the artifacts and not the work: `executed of 20` is only a true
statement if the denominator is derived rather than typed.

## A diagnosis must not name a cause the lane has not established

A lane fails in front of an operator who does not know its internals, so the
first line it prints becomes the diagnosis. Two habits make that line lie:

**Merging stderr into a results stream.** Capturing `2>&1` and then parsing the
result means any notice the binary writes — a warning, a future governor line —
prepends non-JSON to well-formed output. The parse fails and the lane reports
that the results were unparseable, which is false; the results were fine. Capture
stderr separately and put the complete flattened diagnostic in the detail column —
not its first line, since the part that matters is often on a later one.

**Reading a file that may not exist.** A command substitution over a missing file
yields the empty string, and an empty query is a *usage* error from the engine.
The lane then prints the engine's parse complaint for what is actually a missing
artifact, blaming the thing under test for the harness's own fault. Check the file
exists, and name it.

**Blaming the binary for the scratch directory.** Both lanes captured the
binary's stderr into `LANE_TMP` and read it back unconditionally. When that
directory is unusable the *redirect* fails, so the binary never runs: the status
is 1, the capture is empty, and every row reads `CANNOT-EXECUTE … the binary
exited 1 without saying anything`, ending in a summary stating that not one query
executed. A harness failure published as a total failure of the thing under test.
The capture path is therefore proven writable *before* the redirect that depends
on it (`lane_reset_capture`), and read only when there is something in it
(`lane_capture_stderr`) — both shared, because both lanes had written the bug.

### The broadest diagnosis runs first

When two checks can fail on the same state, the one whose message describes that
state must run first, or the narrower one answers in its place and the broader
one becomes unreachable in exactly the case it was written for.

Both lanes had this backwards, and the second was found only because the first
was fixed. A wholly vacuous run — every query executed, every one matching zero
rows — is reported by a guard that names the dataset, the conversion and the
normalisation as suspects and says in as many words that zero everywhere is not a
fast run. Ahead of it sat a per-query oracle. A zero is a real answer, so the
oracle *has* a recorded value for a query that matched nothing, and it fired
first: the operator was told "Q1 answered 0 rows; LUBM publishes 4 … the corpus
or the conversion is wrong" about a corpus in which nothing matched at all, and
the paragraph written for precisely that case never printed.

Demonstrated in both directions with a stand-in that answers every query zero
rows while delegating conversion to the real binary — so the corpus still matches
its pin and the run is vacuous in exactly the way the guard describes. Before the
reordering the lane died on the oracle's message; after it, on the vacuous one.

## The query set: counted, certified, re-checked

These three began life in `scripts/watdiv-lane.sh` alone, which is the shape the
parity rule exists to catch — the LUBM lane went without them. They are shared
now, each taking the directory, the expected count and the arena knob as
parameters:

| law | shared implementation |
| --- | --- |
| A generator reporting success is not a full query set; no digest is published for a partial one | `lane_require_query_count` |
| A missing query file names itself, rather than becoming an empty query string the engine is then blamed for rejecting | `lane_require_query_file` |
| The set is digested, and re-verified before the first query, on a missing file, and after the last — so a concurrent run that rewrites the arena mid-loop is caught rather than measured | `lane_query_set_digest`, `lane_verify_query_set` |

The digest covers **every** file in the directory, not only the queries. The
index the result loop reads and the provenance record naming each candidate drawn
are part of what a run is; leaving them outside meant a change to what was
*recorded* was invisible to the certificate, and the index could be rewritten
underneath the loop without tripping the concurrency check.

Counting artifacts is not counting work, either. A lane that proves it wrote
twenty queries and then reports however many rows it managed to read has verified
the artifacts and not the run, so both lanes check that what the loop read is the
whole workload and report a denominator derived from that count rather than typed.

Concurrency deserves particular care, because the arena comes from a single knob.
Two runs with default knobs share it, and every step begins with a destructive
wipe — so "each run generates into its own directory" is a statement about where
a particular pathology's stray files land, never about concurrent safety. Two runs
at once want two arenas. The artifact *cache* is separate from the arena and is
shared regardless of it, so it must be safe under concurrent use on its own terms:
a scratch name unique per process, with the atomic rename doing the rest.

## Some guards are past the point a test can reach

A lane validates its knobs before step 1, and proves its binary before trusting it —
which is before step 1 when a `*_BIN` knob supplies one, and at the build step
otherwise. Those refusals are therefore testable offline and are tested. The guards that decide whether to compare a digest
against its pin sit at step 5, past generation and conversion — and every test that
drives a lane stops it before step 1, either at the binary probe or at a knob
validator, so that no test needs the network or a JDK. (Six of the fourteen point the arena somewhere
uncreatable; the rest do not need to, because the binary probe comes first.) Such a test
therefore cannot observe a step-5 guard at all, and asserting that its failure message
is absent is trivially true — which is why the step-5 laws are proved against the
shared helpers directly instead, in `crates/bench/tests/lane_common_laws.rs`.

That is worth writing down because it was got wrong twice in this file's own subject
matter. Both directions of the corpus-pin guard are established by RUNNING the lane:
at default knobs it prints that the digest matches the recorded pin, and with a
non-default `LUBM_ONTO` — an absolute IRI the generator stamps into every document,
and therefore an input to that digest — it completes and prints that no pin was
checked, naming the knob. Before the knob was added to the guard it died against the
default corpus's pin while blaming generation, conversion or concatenation order.

## A refusal is a claim in two directions

Every guard above rejects something, and a guard written slightly too eagerly
rejects everything — which looks identical from the inside, because a test that
only ever feeds it bad input passes just as happily. So each refusal is exercised
on both sides, and the valid neighbour is made to fail for a *different, named*
reason rather than to succeed: a run given a good knob and an uncreatable arena
must fail **on the arena**, which is what shows execution got past the knob check
rather than merely that it exited non-zero.

Two of these were caught by that discipline rather than by review. Refusing a
LUBM generation because nothing needed renaming would have rejected a corpus from
a *fixed* generator, where the load-bearing check — that a corpus exists — sat
two lines below unexecuted. And a prefix check written against raw template text
refused legal output, because the loose prefix pattern also matches inside a
quoted literal and inside an IRI path, which instantiation substitutes.

A control that cannot distinguish the case it exists for is not a control. The
fixture proving the LUBM block splitter survives the published file's own
`# Query 11, 12 and 13` prose comment had to *open* on those words: a first
version merely contained them later in the line, and a deliberately loosened
splitter still passed.

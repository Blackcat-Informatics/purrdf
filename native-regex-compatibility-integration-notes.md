# Native compatibility law: concrete integration packet

All proposals are Stage-only. Standard patch dry-runs passed; no compiler,
build, test, Git, forge, or shipping-source mutation was performed by this
support agent. Nullable captures and the complete real-entry matrix are
required qualification, not reported successes.

Apply in this coherent source batch:

1. `native-regex-compatibility-generator.patch`: existing Rust generator,
   exact frozen UnicodeData16 input, original corpus freeze ledger, provenance,
   and generated-output gate. The input's native BLAKE3 and SHA-256 identities
   were independently read with `b3sum` and `sha256sum`; the generator checks
   both UCD16 inputs' exact native BLAKE3 identities before deriving tables.
2. In the sole managed lane, generate `xpath-compatibility`, format it into
   `crates/rdf-core/src/xsd_regex/xpath/compatibility_tables.rs`, regenerate and
   compare. No generator binary was available to the support agent, so the
   final generated table is not present in this packet. It is a mandatory
   source input before a compile, not an optional deferred feature.
3. `native-regex-compatibility-law-draft.patch`: six existing/new native homes.
   The final refresh promotes existing Pike `find_from`/`search` and Ctx
   `may_start` from test-only to production; changes the existing Table test
   pattern; preserves the physical compiler/scanner diagnostic implementation.
4. `native-regex-compatibility-caller-draft.patch`: four evaluator homes. Uses
   shipping LexicalFrame/Shared/Publication/AdmittedMap and the existing LRU
   cache body. Removes the raw nested String/Arc regular-engine cache.
5. `native-regex-compatibility-fixtures.patch`: appends seven native physical/
   semantic tests to the actual counting-allocator fixture; three tests to
   the actual four-entry string-owner fixture; three tests inside the existing
   real segmented program-owner fixture. The last file's hunk also dry-runs
   against current source by offset; its intended base is the caller postimage.

The law is explicit inside the one native program: private `Law::Dated(Profile)`
or `Law::Compatibility`. Public `CompatibilityPattern` only exposes immutable
source/flag views and ownership-carrying execution APIs; it never exposes a
dated program or raw extraction. The wrapper delegates to the same native
parser, flat node/set arenas, compiler metadata, Pike control states, replacement
cursor, and actual physical Memory owner. No alternate matcher is introduced.

Compatibility semantics implemented:

| Contract | Actual home/change | Required fixture |
| --- | --- | --- |
| Frozen Unicode16 categories and simple fold | existing Unicode generator shares category parser/major unions and fold partition emitter; compiler selects compatibility tables | properties, literal folding, category/escape negation, subtraction, U+A7CE/U+A7CF remain unassigned/unfolded at16 |
| XML name escapes | shared `terminals::xml_name_*_ranges`, folded before complement | astral U+10000, positive/negative name escapes and subtraction |
| Block spelling/ranges | existing Unicode16 `blocks::lookup`, no alias table | exact Is names, surrogate empty scalar range, unknown IsGreek refused |
| All original flags; q/x rules | existing Modes parser/xflag normalization/quoted parser with explicit law | qsimx ignores m/s/x; q+i uses16 simple fold; x keeps class whitespace |
| Liberal ampersand/tilde escapes | same member/scalar classifiers admit them only under compatibility | standalone and range-end escapes; dated path continues to refuse |
| Backrefs and non-grammar constructs | same scanner diagnostics retained; compatibility parser rejects backrefs | flags/inline modifiers/lookaround/named groups/unsupported escapes remain lexical failures |
| m terminal-newline anchors | same Ctx start/end/candidate helpers condition on law | all captures including ^$ after a final LF; dated3.1 remains different |
| Nullable repeat priority/captures | same repeat nodes canonicalize compatibility x* as(x+)?; same Pike control table merges progress levels; repeat split is merged before queuing its exit | all capture spans, greedy/lazy, optional/plus/star, finite counts, nested nullable choices |
| Empty replacement law | same replacement body skips only the dated empty guard; output cursor and UTF8 search cursor separate; suppress adjacent empty after nonempty | empty input, multibyte text, terminal empty, `$N`, q verbatim, invalid template even without match |
| Actual physical admission/error precedence | all new variants inline/static; compiler/matcher/output use existing Memory and original LexicalFrame; Publication keeps original grants | independent allocator peaks/retained bytes, zero-capacity refusal, actual segmented first source cause |
| Production/direct/prepared/governed entry wiring | same Program owner carries either immutable public law wrapper, successful cache entries only; full law/source/flag comparison | SELECT duplicate bag, ASK, CONSTRUCT, DESCRIBE through all four fallible entries |
| Retained ownership | Shared control-before-factory publication and native original output Publication reused | program shallow clone and output original pointer remain live after workspace/capability drop |

Unselected requests use `Limits::without_presets`: no dated application preset
is silently applied. Native counters retain their checked u64 representation;
overflow remains a typed operational failure, and the actual caller physical
profile bounds every heap allocation. Explicit selected dated limits and cache
hit re-admission remain unchanged. Failed requests are recomputed in the current
request owner; neither a cache nor a linked failure verdict publishes them as
immutable artifacts.

The legacy public `xsd_regex::compile` and its `as_regex` representation remain
unchanged. The existing resident compiler stays only in the evaluator's
cfg(test) oracle helper/independent tree-walker. Production no longer calls it.
The native compatibility source contract preserves the existing 64 KiB check.
Old 1 MiB translated-source, folded-name-expansion and regular-engine NFA size
failures are implementation limits, not the native language or byte proof.
Root explicitly settled that they must not be imitated by a fabricated compact
program proxy. Concrete differential fixtures compare supported semantics;
actual physical refusal replaces those old opaque implementation-size failures.

Remaining validation obligations are concrete: generated table identity and
reproducibility; Rust type/docs/clippy/format qualification; differential capture
priority and replacement output; measured physical capacity and reservation
release; four-entry/all-form real queries. None have been run by this helper.
There is no opaque regex fallback, unsupported-producer stub, or arbitrary
term/output multiplier in the proposed production path.

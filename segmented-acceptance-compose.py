from pathlib import Path
import re
import difflib

stage=Path(__file__).parent
root=stage.parents[1]
relative=Path('crates/sparql-eval/tests/segmented_query.rs')
before=(root/relative).read_text()
s=before
s=s.replace('''            assert!(source.evidence().live_bytes() > before + 50_000);''','''            assert!(source.evidence().live_bytes() > before, "execution owns its working storage through the visitor");''')
# The EXPLAIN returned owner now retains its output. End that owner before
# comparing a subsequent run's released footprint against this baseline.
s=s.replace('''    // Repeat on the same healthy storage session. A retained execution guard''','''    let held = source.evidence().live_bytes();
    let shared = explanation.clone();
    drop(explanation);
    assert_eq!(source.evidence().live_bytes(), held);
    drop(shared);
    let released = source.evidence().live_bytes();
    assert!(released < held, "the final explanation owner releases its buffer grant");

    // Repeat on the same healthy storage session. A retained execution guard''')
s=s.replace('''        assert_eq!(live, after.live_bytes());
        assert!(receipt.live_bytes() > live);''','''        assert!(live > released, "the repeated explanation owns its native buffers");
        assert!(receipt.live_bytes() > live);
        drop(again);
        assert_eq!(source.evidence().live_bytes(), released, "completed execution and last explanation release their grants");''')
names=[
    'tiny_capacity_and_unpriced_order_refuse_before_any_row_probe_or_output',
    'governed_refusal_prices_its_reporting_owner_before_allocating',
    'unpriced_construct_refuses_before_allocating_a_destination_mint_prefix',
    'unpriced_request_inputs_refuse_before_copying_parameter_metadata',
    'fallible_explain_refuses_tight_capacity_and_unpriced_algebra_before_probing',
    'fallible_explain_refuses_unpriced_options_before_cache_lookup_or_data_reads',
    'prepared_governed_cold_sessions_match_text_receipts_and_release_reporting',
]
for name in names:
    a=s.index('fn '+name+'(')
    # Remove the attached test/documentation attributes and preserve adjacent tests.
    b=s.index('\n}', a)+2
    start=s.rfind('\n\n',0,a)+2
    s=s[:start]+s[b:]
s += '\n\n'+(stage/'segmented-acceptance-rewrites.rs').read_text()+'\n'+(stage/'segmented-receipt-rewrite.rs').read_text()
post=stage/'segmented-acceptance-postimages'/relative
post.parent.mkdir(parents=True,exist_ok=True)
post.write_text(s)
patch=''.join(difflib.unified_diff(before.splitlines(keepends=True),s.splitlines(keepends=True),fromfile='a/'+str(relative),tofile='b/'+str(relative)))
(stage/'segmented-acceptance-owner-draft.patch').write_text(patch)

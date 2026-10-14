# Duration add/sub admission checkpoint

Source: dirty numeric worktree, after `add_durations_admitted` and `subtract_durations_admitted`, before later whole-temporal work. No commit identity.

Command: `cargo test --locked -j8 -p purrdf-xsd --test exact_governance duration_add_sub_admit_seconds_before_allocation -- --exact`

First run: terminal 101, compiler `-D dead-code` for `decimal_add_exact`, now removed. Retry: terminal 0, 1 passed, 0 failed, 20 filtered out. The test covers 2,000-digit duration add/sub admission refusal before allocation and parity with resident operations. A subsequent tiny-value cost assertion was added after this receipt and remains to be rerun.

Scope: no calendar, duration scale/division, ADJUST, or full #470/#443 acceptance established.

Later duration source checkpoint: `cargo check --locked -j8 -p purrdf-xsd --lib --message-format=short` terminal 0, followed by `cargo test --locked -j8 -p purrdf-xsd --test exact_governance duration_ -- --nocapture` terminal 0 (5 passed, 0 failed, 18 filtered). This source includes borrowed/admitted duration multiplication, default-policy division, unary minus, bounded add/sub spill cost, and their focused refusal/parity controls. The broad XSD suite and all calendar/ADJUST production paths remain unqualified.

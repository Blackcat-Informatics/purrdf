# Exact temporal first runtime checkpoint

Source state: after exact seconds/date-time arithmetic conversion, before updating the historical bounded-scale assertion and before adding typed native storage propagation or exact D/H/M duration numerals.

Command: `cargo test --locked -j 8 -p purrdf-xsd --lib temporal::tests -- --test-threads=4`
Terminal: 101. Result: 79 passed, 1 failed, 336 filtered out. The sole failure was `temporal::tests::a_too_wide_field_is_out_of_range_only_in_an_otherwise_valid_lexical`, which still expected `parse_time("12:00:00.1234567890123456789012345")` to return OutOfRange; it returned `Ok(())`. The assertion was then updated to require exact acceptance. This checkpoint is not a passing suite and does not apply to later source edits.

Preceding command: `cargo test --locked -j 8 -p purrdf-xsd --no-run --message-format=short`; terminal 0, all XSD library/integration targets compiled on that prior source.

# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

.PHONY: check metadata test

metadata:
	cargo metadata --no-deps

check:
	cargo check --workspace --lib --tests

test:
	cargo test --workspace


<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Public mail corpus

These two original byte files are copied unchanged from CPython's public email
test data at commit `0ec3aee262b03276a18aeb23cb9957e1b57c9d08`:

- [msg_01.txt](https://github.com/python/cpython/blob/0ec3aee262b03276a18aeb23cb9957e1b57c9d08/Lib/test/test_email/data/msg_01.txt)
  retains a delivered Postfix message with a folded Received field.
- [msg_02.txt](https://github.com/python/cpython/blob/0ec3aee262b03276a18aeb23cb9957e1b57c9d08/Lib/test/test_email/data/msg_02.txt)
  retains a Mailman digest with nested multipart/digest messages and a footer.

The upstream LF line endings are retained. These files complement the authored
adversarial cases; they are not converted to CRLF or repaired. Both run through
the same production analysis, RDF projection, three serializers, RDF parser and
strict cover reconstruction. Their bytes and emitted RDF participate in the
frozen cross-target transcript. The message payloads are byte-frozen by the
existing corpus guard. The upstream LICENSE is retained verbatim in LICENSES.

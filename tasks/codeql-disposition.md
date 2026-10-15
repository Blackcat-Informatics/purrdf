# CodeQL alert 245: false positive

The hosted CodeQL summary flags rust/cleartext-logging at the native persisted
fixture's block-layout eprintln. Its source is SegmentedImage::certify, an RDF
image authentication operation, not credential acquisition. The fixture locally
constructs example.org IRIs and value N literals. The flagged sink prints only
six aggregate block-kind byte counts and an authentication byte count. It emits
no image contents, input strings, keys or credentials. These are the public
layout quantities requested by the issue.

The first dismissal request exceeded GitHub's 280-character comment limit and
failed HTTP 422. The subsequent concise request records the disposition. No
source suppression, changed query, disabled job or guard bypass was used.
Read back the resulting check state before merge; a dismissed finding is not
an executed analysis pass.

# Current correction preparation external refresh

Retrieved UTC: 2026-10-06T14:57:27.288559+00:00.

Official https://www.iana.org/assignments/cose/algorithms.csv still records
unassigned -256 to -54 (including -58) and standalone ML-DSA-65 -49/RFC9964.
Raw CSV SHA256 4dc4c64f84e6020a05403862219b66b0bfd9cc853245d87b82ed00f6d77600f3.
Pinned JOSE draft-04/LAMPS draft-19 remain unchanged.

Governed upstream from intake: Blackcat-Informatics/gmeow-gts. Current main
0d1c8299c9411ea4ead853e31721d42ea66f081e; canonical committed tree
bbac332e919339ab0b1a9b99dcba841f0b434464. Native forge API supplies these
commit/recursive-tree surfaces absent from stagectl. Complete canonical tree
is truncated:false; exact entries match the preserved commit-addressed tree
response, whose sha field echoes its requested commit rather than canonical
Git tree identity. Actual authoritative tree identity comes from the commit
and is independently retrieved at that tree OID.

Only vectors/cose/sign1-basic.json (cf90b6eb0b17c175fcd4d643f326174b245573ca)
and sign1-empty-id.json (3e83a8dc373b42b31a014d22b371d02d44ae7cac) are present
in the shared COSE directory. Complete composite/ML-DSA path search is empty.
The unchanged commit/tree identity qualifies reuse of prior complete corpus
absence audit; no newly published shared composite fixture exists there.
No governed vectors are fabricated/regenerated and no shared-engine
interoperability is inferred from primary independent known answers.

Commit JSON SHA256 e4d78ae07e93bff91c3eca21b4f60a647a64ddf60c53389ddf36ddbd4b6a3dbe.
Commit-addressed tree JSON SHA256 f644842d31e6234af02e8fda1945b7afbfed24be00608c3fd957ca9db53d54c8.
Canonical tree JSON SHA256 bee868726f81b1ff45e4b5a95c9a1c144ad4c0bce93705782b66ad0cf4cfe2b7.
Full coverage/blob entries in T6-R1-external-state.json; raw responses retained.
This is correction preparation refresh, not a later merge-boundary refresh.

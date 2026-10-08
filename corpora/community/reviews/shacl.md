<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Pre-observation SHACL oracle review (automated agent review)

Every review recorded in this document was performed by automated agent reviewers. None of them is an independent human review, and none is a W3C approval.

APPROVE the exact 64-case source and portable payloads identified below. No production or reference engine output was used to derive or approve the expected results.

Inventory SHA-256: `e6d46aaf720b492abe0ab2c7cc67bc3e255bd0194df0ff22a3144e83ee1d44b7`.
Generator SHA-256: `5ba6fe7e33813d4267357e6f0e556696624e3b74b778cb76ef63d6ae405355b5`.
Amendment record SHA-256: `7536e409bf22ec19897e0608c37211e98bf6643858119d3e2c6f60468c43aeaf`. All amendment payload hashes independently match current bytes.

The 40 absence controls cover ten patterns × IRI/anchored data blank × conforming/violating. Matched-leaf values insertion restricts each OPTIONAL/EXISTS/UNION/path/subquery/aggregate leaf to the current focus. With p present only at a, precisely b and c have an absent-value solution; giving each p selects none. Nested optional and aggregate controls preserve one absence solution per absent focus. Each emitted result requires the current focus/value, Violation, source shape and named sourceConstraint ex:Constraint. Blank correspondence is anchored independently by recordA/B/C in the same manifest document; it cannot be established by report-only isomorphism.

Four component cases select their declared required=ex:p parameter. ASK prebinds value to each node-shape value node and false creates one result; SELECT does not manufacture a value role and its absence row maps value to the node-shape focus. Both report ex:Component, with no standalone sh:sourceConstraint.

Predicate controls: IRI ex:p matches, ex:q does not; a blank cannot be an RDF predicate. The amended blank case targets the data blank using targetObjectsOf ex:focus. Same spelling in a separately parsed shapes document does not establish identity. The amendment repairs the original oracle before observations.

Twelve profile admission cases pair each construct with a dated contract: local VALUES and a nested projection hiding this are admitted only under the dated draft; VALUES mentioning this, AS this and MINUS fail under both. SERVICE refusal in the draft is PurRDF policy under the specified SHOULD/MUST failure rule. The Recommendation global VALUES/projection rules are not imposed on the draft. The common RDF/SPARQL1.2 kernel is distinct from these dated SHACL prebinding roles.

The explicit true failure row requires a semantic processor failure with reason solution-failure, never a normal violation or environmental refusal. The one bound message row requires row message, taking priority over declared message. SourceConstraint is the actual value of sh:sparql under both revisions.

The quoted-focus case has two distinct complete RDF1.2 terms and only the bad term lacks a p value. It is draft/RDF1.2-labelled. The complex path follows p then inverse q; a reaches z, b reaches nothing. Exactly b has MinCount, with no sh:value and the exact equivalent sequence/inverse RDF list topology.

The two AF rule cases infer exactly b missing true and c missing true, independently of their input triples. Their portable manifests use sht:Infer with an RDF list of inferred triple lists, matching the current upstream fixture representation; the separate expected RDF payload contains that exact delta. They carry explicit AF applicability and are not relabelled as Core.

Normative authorities: [REC20170720 Appendix A and §5.3.2](https://www.w3.org/TR/2017/REC-shacl-20170720/#sparql-constraints-variables), [WD20260918 Appendix A and §3.3.2](https://www.w3.org/TR/2026/WD-shacl12-sparql-20260918/#sparql-constraints-variables), [SHACL-AF Note20170608 rules](https://www.w3.org/TR/2017/NOTE-shacl-af-20170608/#SPARQLRule). Scope-labelled applicability and independent expected source links are required in every runner. This approval does not certify an engine, profile totals, bindings, upstream submission or any unexecuted case.

## Exact payload hashes

The table records the bytes approved by the agent re-review of 6 October 2026 below; the inventory row is that re-review's inventory identity.

| Path relative to corpora/community | SHA-256 |
| --- | --- |
| shacl/admission-hidden-prebound-subquery-draft/manifest.ttl | `a56323a6c527c69fb51ada16272c5d74d9d72f24332ac7bce5122d40b803fd8a` |
| shacl/admission-hidden-prebound-subquery-draft/shapes.ttl | `6b90a10a50a1ba9af1bb65341b0e9d1d6ef8ff1e41f4710c3bdf142828cc1b61` |
| shacl/admission-hidden-prebound-subquery-rec/manifest.ttl | `c6a3b6ee95bda894b14a1b37b4d3aa93956c26af91f6005bdb333b280f48aa4a` |
| shacl/admission-hidden-prebound-subquery-rec/shapes.ttl | `6b90a10a50a1ba9af1bb65341b0e9d1d6ef8ff1e41f4710c3bdf142828cc1b61` |
| shacl/admission-local-values-draft/manifest.ttl | `c08d11e78d3911c02aac7e570a1a79ebc85bfa9dc517878ca4b1da613236c385` |
| shacl/admission-local-values-draft/shapes.ttl | `8030a4e41fbd4b2bcb48c66ebb102d4d78b41c0d6ddcb953b5385a416f5e223f` |
| shacl/admission-local-values-rec/manifest.ttl | `bbee9076ab2b79d4b0cb7789115d2dc32dbe21175a0cd3c581380dccbb292f21` |
| shacl/admission-local-values-rec/shapes.ttl | `8030a4e41fbd4b2bcb48c66ebb102d4d78b41c0d6ddcb953b5385a416f5e223f` |
| shacl/admission-minus-draft/manifest.ttl | `ed8fac508337732b379af4847f57c4aa7efd849338c749d55c9c45f1afbacab4` |
| shacl/admission-minus-draft/shapes.ttl | `183a9f37f7116cf4b37dd2dc1550287088601ddf2d835cfec7340e8ad6d2c0a9` |
| shacl/admission-minus-rec/manifest.ttl | `7dc8904e0179c760ad57916d7949a17a79f4ff5639cb4650d44642c3986e3c70` |
| shacl/admission-minus-rec/shapes.ttl | `183a9f37f7116cf4b37dd2dc1550287088601ddf2d835cfec7340e8ad6d2c0a9` |
| shacl/admission-prebound-as-draft/manifest.ttl | `ed9a064b6ab17c3681a8c7f3f5600b3240eebfa8d01810bf076afa3c71dc1f68` |
| shacl/admission-prebound-as-draft/shapes.ttl | `3bc2fa9e4ca4f167eac50064b0f60f2a8c088a23bafeed74b83fce887ab9044c` |
| shacl/admission-prebound-as-rec/manifest.ttl | `73f0877e205f2f491463e48ed8c95e35c255f8f28eae0e6ce6037395ca2871d3` |
| shacl/admission-prebound-as-rec/shapes.ttl | `3bc2fa9e4ca4f167eac50064b0f60f2a8c088a23bafeed74b83fce887ab9044c` |
| shacl/admission-prebound-values-draft/manifest.ttl | `0de5e9d47d4416505e9c62c960120b3508a528ea4a34f4be54e099edad8cd015` |
| shacl/admission-prebound-values-draft/shapes.ttl | `6d9341b8fc3f4f24de3899b1e518ef8e9cc41b20bf9a74282c7d15e71ab9df09` |
| shacl/admission-prebound-values-rec/manifest.ttl | `60d4185ec4f8187207aed15ca1846c09881a8cb289f68f1ad669cd1eb23fb6c0` |
| shacl/admission-prebound-values-rec/shapes.ttl | `6d9341b8fc3f4f24de3899b1e518ef8e9cc41b20bf9a74282c7d15e71ab9df09` |
| shacl/admission-service-draft/manifest.ttl | `944642bb4398ea983b0178d2705bb4299d6bfc83d30649c7dbb77715a8aaaa03` |
| shacl/admission-service-draft/shapes.ttl | `727b00904183f0fb7c90ad030ea04b560f029ef726e2aafa79ba958818b0e223` |
| shacl/admission-service-rec/manifest.ttl | `481ed7ade20428b6c59ad2eab2b5bf778ed9763c77a4e6c5213d24d6070c1e72` |
| shacl/admission-service-rec/shapes.ttl | `727b00904183f0fb7c90ad030ea04b560f029ef726e2aafa79ba958818b0e223` |
| shacl/aggregate-subquery-blank-conforming/manifest.ttl | `2bdb37a4ffa79db77ed6cafea9b2c0b23d4f1ee05e3a6aab8d2dbde4b04b1f66` |
| shacl/aggregate-subquery-blank-conforming/shapes.ttl | `99a3c333b0011b306e5271e52563bb8498df1192715f0fbdd7483c27c3f02067` |
| shacl/aggregate-subquery-blank-violating/manifest.ttl | `261e1330737e1c1f2509bce61965f9786f8d97328dea06c1b7a4fab478382071` |
| shacl/aggregate-subquery-blank-violating/shapes.ttl | `99a3c333b0011b306e5271e52563bb8498df1192715f0fbdd7483c27c3f02067` |
| shacl/aggregate-subquery-iri-conforming/manifest.ttl | `28f5b485100f14cf2554bd2bb851d0b043c2f860c9bf65477cc2f9d4504d64f2` |
| shacl/aggregate-subquery-iri-conforming/shapes.ttl | `99a3c333b0011b306e5271e52563bb8498df1192715f0fbdd7483c27c3f02067` |
| shacl/aggregate-subquery-iri-violating/manifest.ttl | `71b7dac16fcaf2a9f5f3cd44242eb10c454f45473531af729664e65975162ef6` |
| shacl/aggregate-subquery-iri-violating/shapes.ttl | `99a3c333b0011b306e5271e52563bb8498df1192715f0fbdd7483c27c3f02067` |
| shacl/anchored-optional-blank-conforming/manifest.ttl | `195eda13c7ac0fb00fd2d89fe265c637fcb3be297e59df4f05a3893ad39b44f0` |
| shacl/anchored-optional-blank-conforming/shapes.ttl | `46b6bf4d73de8458d5813e5b2b3d90df7ac23e9114b8f3cf6755bd86868c3a63` |
| shacl/anchored-optional-blank-violating/manifest.ttl | `87a32314613422b3b622865104829c8df6e599fbe07ba79d2bb9eb9702b76569` |
| shacl/anchored-optional-blank-violating/shapes.ttl | `46b6bf4d73de8458d5813e5b2b3d90df7ac23e9114b8f3cf6755bd86868c3a63` |
| shacl/anchored-optional-iri-conforming/manifest.ttl | `b23159dad6d7f36af2fe27633910a1411bedd1b78bdacf730178aa9221f2c1cd` |
| shacl/anchored-optional-iri-conforming/shapes.ttl | `46b6bf4d73de8458d5813e5b2b3d90df7ac23e9114b8f3cf6755bd86868c3a63` |
| shacl/anchored-optional-iri-violating/manifest.ttl | `244d1f5b32359f65e26912fd929e7d08d73adbb5859838c47e5316569c432de3` |
| shacl/anchored-optional-iri-violating/shapes.ttl | `46b6bf4d73de8458d5813e5b2b3d90df7ac23e9114b8f3cf6755bd86868c3a63` |
| shacl/ask-validator-parameter-conforming/manifest.ttl | `ec9e52e95e20f2be4c8d04bd05fa2de56dbbc456795de8d4a22840c67a925d71` |
| shacl/ask-validator-parameter-conforming/shapes.ttl | `b2f7e31d962b16cc9df9f8ca2c995de2d8144a3721df83fa30920c6ee9f4e005` |
| shacl/ask-validator-parameter-violating/manifest.ttl | `eba5f290a49d32023693bb9e2efa11bce9858e45d25a519149c2851e6224b387` |
| shacl/ask-validator-parameter-violating/shapes.ttl | `b2f7e31d962b16cc9df9f8ca2c995de2d8144a3721df83fa30920c6ee9f4e005` |
| shacl/complex-sequence-inverse-path/manifest.ttl | `41bb4ea3f75deda6840546f22298b7780173ad9730a446649d14de4192fafd20` |
| shacl/complex-sequence-inverse-path/shapes.ttl | `e399c8eb57ec23e9f8885b5a0f97be7938a1dbf2793128ce1b09aa4b8c21bb05` |
| shacl/empty-left-optional-blank-conforming/manifest.ttl | `820950b2cd1497d3f5486f2bdf6764c2b5da44ea8b3a9ec7d640c94d978883ba` |
| shacl/empty-left-optional-blank-conforming/shapes.ttl | `49910d4201a8d958f249475ea4b9b2665e2fb809843496fcd3050480f35100ec` |
| shacl/empty-left-optional-blank-violating/manifest.ttl | `cb152927ec6490afdafdae6139a2defe10d222d912772ce6ea52489182eba94e` |
| shacl/empty-left-optional-blank-violating/shapes.ttl | `49910d4201a8d958f249475ea4b9b2665e2fb809843496fcd3050480f35100ec` |
| shacl/empty-left-optional-iri-conforming/manifest.ttl | `ba6977d4497416eaec561205e990119d3ddd395431c95c4bad5d74f52e0024e0` |
| shacl/empty-left-optional-iri-conforming/shapes.ttl | `49910d4201a8d958f249475ea4b9b2665e2fb809843496fcd3050480f35100ec` |
| shacl/empty-left-optional-iri-violating/manifest.ttl | `a36102f61c4a54a9c4354422f3a71fdf0b806f4f57c2a0a08dcbf81bd5df2d5e` |
| shacl/empty-left-optional-iri-violating/shapes.ttl | `49910d4201a8d958f249475ea4b9b2665e2fb809843496fcd3050480f35100ec` |
| shacl/exists-absence-blank-conforming/manifest.ttl | `4a6686b7555b84feb1b08df15941bd59b1a0ea8bee448573ff0fa5ada39b41c3` |
| shacl/exists-absence-blank-conforming/shapes.ttl | `063d91510b1d9bf113598990fef2da301dd26cff18d546938fab8191109293c7` |
| shacl/exists-absence-blank-violating/manifest.ttl | `fbe9f9cab1ad424c8a7cac4caff8a249829268a0b47035167701929369b9096b` |
| shacl/exists-absence-blank-violating/shapes.ttl | `063d91510b1d9bf113598990fef2da301dd26cff18d546938fab8191109293c7` |
| shacl/exists-absence-iri-conforming/manifest.ttl | `8745446f9e6af57c78ba503847e47bc023bdda498d2821b8c37ff7225e0ec681` |
| shacl/exists-absence-iri-conforming/shapes.ttl | `063d91510b1d9bf113598990fef2da301dd26cff18d546938fab8191109293c7` |
| shacl/exists-absence-iri-violating/manifest.ttl | `1a0d9ef9a44db90c5cb58eefecc46ec73fd6bccd0ab2140b87ab44387f0958a1` |
| shacl/exists-absence-iri-violating/shapes.ttl | `063d91510b1d9bf113598990fef2da301dd26cff18d546938fab8191109293c7` |
| shacl/inventory.json | `9ae8dda64e130af80e9e9e9c5e5df783d2430b67a07b195417fc976654f99d91` |
| shacl/manifest.ttl | `85b5c7782601f68653a33c5d8a81ccb0f8abaf3dc0a252bbeca42659d121a25a` |
| shacl/nested-not-exists-blank-conforming/manifest.ttl | `b7db8f394af532b7c8e44c9d7ec1b116ca3b060adcfd02b16fe4ad5dc82af68b` |
| shacl/nested-not-exists-blank-conforming/shapes.ttl | `bff1cbc1086f744f7bd1a6f01797876406520a0a21898879e27d15728b9977a1` |
| shacl/nested-not-exists-blank-violating/manifest.ttl | `0d2ca5f2701f460337073cf0bf7f292a916d4e102c3f43ca705c50dc64218d85` |
| shacl/nested-not-exists-blank-violating/shapes.ttl | `bff1cbc1086f744f7bd1a6f01797876406520a0a21898879e27d15728b9977a1` |
| shacl/nested-not-exists-iri-conforming/manifest.ttl | `563eeeabcb533b31d6c30884984d87b8043e28c2d3870bd20eae55ab6470211c` |
| shacl/nested-not-exists-iri-conforming/shapes.ttl | `bff1cbc1086f744f7bd1a6f01797876406520a0a21898879e27d15728b9977a1` |
| shacl/nested-not-exists-iri-violating/manifest.ttl | `2b531c2b9f4986cb949eef752ac57fd65dbf954da80c42b04f4b4196af966d15` |
| shacl/nested-not-exists-iri-violating/shapes.ttl | `bff1cbc1086f744f7bd1a6f01797876406520a0a21898879e27d15728b9977a1` |
| shacl/nested-optional-blank-conforming/manifest.ttl | `c668e075bd00c8d90670684d77dbdfc354c22767ef58782061a36af598c1e450` |
| shacl/nested-optional-blank-conforming/shapes.ttl | `4526cc039a7b8cf0d93923ae61934b814ba12505aef6aa74cb6da4df0d89d0cd` |
| shacl/nested-optional-blank-violating/manifest.ttl | `44a0b5f4688286e9c43f2fc9240992690ccf269a52e14d466632de44649ef0e7` |
| shacl/nested-optional-blank-violating/shapes.ttl | `4526cc039a7b8cf0d93923ae61934b814ba12505aef6aa74cb6da4df0d89d0cd` |
| shacl/nested-optional-iri-conforming/manifest.ttl | `a1431658f387352613729ccaf0460b995994614d91ed4681eafcfe9f5c022dea` |
| shacl/nested-optional-iri-conforming/shapes.ttl | `4526cc039a7b8cf0d93923ae61934b814ba12505aef6aa74cb6da4df0d89d0cd` |
| shacl/nested-optional-iri-violating/manifest.ttl | `69a8c15f9d875992e75f486cf191d46d980a6e73fc13dd28cda29770be0c9de0` |
| shacl/nested-optional-iri-violating/shapes.ttl | `4526cc039a7b8cf0d93923ae61934b814ba12505aef6aa74cb6da4df0d89d0cd` |
| shacl/not-exists-blank-conforming/manifest.ttl | `77a130c17cc33e99bf9256011018ac365005e3ec56648bc0fe5dbe4cd87491f2` |
| shacl/not-exists-blank-conforming/shapes.ttl | `d30bbb696a83e388eef855eb1a1b24cda960fdd54eda0721b51a18c6c382c2a5` |
| shacl/not-exists-blank-violating/manifest.ttl | `3eda2e6d38e722967ce43ff45d65964af6aaa00b817dab47a24b1baf27cc3b85` |
| shacl/not-exists-blank-violating/shapes.ttl | `d30bbb696a83e388eef855eb1a1b24cda960fdd54eda0721b51a18c6c382c2a5` |
| shacl/not-exists-iri-conforming/manifest.ttl | `4ea7d1bcd37cac98b8e6c8d960061504936e511850391516221aa284a1985550` |
| shacl/not-exists-iri-conforming/shapes.ttl | `d30bbb696a83e388eef855eb1a1b24cda960fdd54eda0721b51a18c6c382c2a5` |
| shacl/not-exists-iri-violating/manifest.ttl | `e4807dc651ffd9e64ac80d593937bbceaf8484f081ed808b6ba291fd119a5a20` |
| shacl/not-exists-iri-violating/shapes.ttl | `d30bbb696a83e388eef855eb1a1b24cda960fdd54eda0721b51a18c6c382c2a5` |
| shacl/path-absence-blank-conforming/manifest.ttl | `7a4898be6aa36910936b7703ba93711ea34817e45e30d95cfbda57b96b838aea` |
| shacl/path-absence-blank-conforming/shapes.ttl | `7948750a46545fcb1e9ae60bbf4c7404ea9a81169b26747f80e1721cc0b8374a` |
| shacl/path-absence-blank-violating/manifest.ttl | `1b24bfbe6696655ff3564ce661b9c6aba4e6415361cb97e47bc801aca2bb3df0` |
| shacl/path-absence-blank-violating/shapes.ttl | `7948750a46545fcb1e9ae60bbf4c7404ea9a81169b26747f80e1721cc0b8374a` |
| shacl/path-absence-iri-conforming/manifest.ttl | `4557a5a378ec718266f34883b389c0c7ef11dac8d1dcdcd377cc4b52fbf4fe1c` |
| shacl/path-absence-iri-conforming/shapes.ttl | `7948750a46545fcb1e9ae60bbf4c7404ea9a81169b26747f80e1721cc0b8374a` |
| shacl/path-absence-iri-violating/manifest.ttl | `b9f365ee9c7ee19ad1b675b06e1fc3f80fd61dd04e3dd55af4613d4a6d14a5f5` |
| shacl/path-absence-iri-violating/shapes.ttl | `7948750a46545fcb1e9ae60bbf4c7404ea9a81169b26747f80e1721cc0b8374a` |
| shacl/predicate-focus-blank/manifest.ttl | `470a5c0dbc0ee5cd5db9fd49d0dd0a8d15eaafefb6459e4ccad2f14b7dc4b859` |
| shacl/predicate-focus-blank/shapes.ttl | `ec073267ca71e067eb61a12819d26921d60cb57fc293f9fe602a3a668a788f21` |
| shacl/predicate-focus-iri/manifest.ttl | `47035b13afea3525dd40f35cd1ad0a82c9e5d901813c96f735386f874af3d5d3` |
| shacl/predicate-focus-iri/shapes.ttl | `9961acd7a2e3237597be15005c7f20feead5520ce421ca13199d95882868e3db` |
| shacl/projected-subquery-blank-conforming/manifest.ttl | `c616142f1f35982a22058d377eaa38ef823bb6565bcf9a3ae23485bbe55377ca` |
| shacl/projected-subquery-blank-conforming/shapes.ttl | `a258e3358d47acfe110808567222074e13fe41ac42677ad0346007c66c49df5e` |
| shacl/projected-subquery-blank-violating/manifest.ttl | `f9cbe2fb5c57dffab598a35affb890e8b082a34f97e63c9ec913f746c1fc4075` |
| shacl/projected-subquery-blank-violating/shapes.ttl | `a258e3358d47acfe110808567222074e13fe41ac42677ad0346007c66c49df5e` |
| shacl/projected-subquery-iri-conforming/manifest.ttl | `13ed6ae2519dad8cd4d1bfa9e87680bceb63ee8eddf5321f48f16bab1da413c2` |
| shacl/projected-subquery-iri-conforming/shapes.ttl | `a258e3358d47acfe110808567222074e13fe41ac42677ad0346007c66c49df5e` |
| shacl/projected-subquery-iri-violating/manifest.ttl | `5b6d01a1d74bf49c4f520fa3dcd9d02dfc8ffc3cc1d4cd869fba4ad5460ddbb3` |
| shacl/projected-subquery-iri-violating/shapes.ttl | `a258e3358d47acfe110808567222074e13fe41ac42677ad0346007c66c49df5e` |
| shacl/quoted-focus-optional/manifest.ttl | `18eb581da8da2b45af9abd428d58de91bac1a2b433bb4fcc4410f84098692faf` |
| shacl/quoted-focus-optional/shapes.ttl | `b904aa07f81106d8652f5f6ee913ef1ac013ea4d50ac0c9a7febd84eb7a5fc38` |
| shacl/rule-not-exists/data.ttl | `4db1f369b75aff48bcf8e90c50c7ddb72f62915348d038a68832d8c67caf63fd` |
| shacl/rule-not-exists/expected.ttl | `4e28ef4bca0ee8ad724a5c11f1f174284735e7295638fab3c7686acfbf7f8316` |
| shacl/rule-not-exists/manifest.ttl | `f57dce435a61ae01fdb698f48ace72098e7f818a81e1e97f90b5c1dc46adfdf5` |
| shacl/rule-not-exists/shapes.ttl | `d97238b35a28b6ea36ac3ee8938ed019d28f4daf18fe776f126ea5e6df4871fb` |
| shacl/rule-optional/data.ttl | `4db1f369b75aff48bcf8e90c50c7ddb72f62915348d038a68832d8c67caf63fd` |
| shacl/rule-optional/expected.ttl | `4e28ef4bca0ee8ad724a5c11f1f174284735e7295638fab3c7686acfbf7f8316` |
| shacl/rule-optional/manifest.ttl | `f87735efc8eeab8511e17cf042849dd34a82b5a0c7e33b7ff2047a287cfc38db` |
| shacl/rule-optional/shapes.ttl | `9a70c5adf47c97c54f9ca2685f3fc24b90de7a372f2fd4f57e0739368311eea5` |
| shacl/select-validator-parameter-conforming/manifest.ttl | `d52cb9852c9569d1f9e61bbea2e51c03748f1bf3cfe154e1eac545b0d66d230c` |
| shacl/select-validator-parameter-conforming/shapes.ttl | `1149c4c0b72d573a0a8e64e12634dd5c5c3514aa24724ca064a0e2fa7dee037b` |
| shacl/select-validator-parameter-violating/manifest.ttl | `92b19a4c3d70235509605ed11d90f1a7905f0a7a9b38415fbdc06be964df4a89` |
| shacl/select-validator-parameter-violating/shapes.ttl | `1149c4c0b72d573a0a8e64e12634dd5c5c3514aa24724ca064a0e2fa7dee037b` |
| shacl/solution-declared-failure/manifest.ttl | `2efcd6c7dd410f49214afe4d552528560249cf4fc620150d4c1cbbfac33672c1` |
| shacl/solution-declared-failure/shapes.ttl | `2e2360a9bf8452c0bd0d23a4c02736bdd576ce8146c75e6fe7219e3835ac49bf` |
| shacl/solution-message-priority/manifest.ttl | `2b1a82a9d89a10e4b525fe27c731d8b31ab4234381f0572a110b5521ec93ec18` |
| shacl/solution-message-priority/shapes.ttl | `8bc460b81f468e387f7fee7d1ddcb6be38ff8f9cec0530742fe6ab7d41d7e520` |
| shacl/union-absence-blank-conforming/manifest.ttl | `a47d746843e47af572d7a4994b55fa2913e47828be613af34bbbfc04f54387a4` |
| shacl/union-absence-blank-conforming/shapes.ttl | `94ca90aa729c18849c03b7dec38e7372e35747461c11af401df5f80a87d70a68` |
| shacl/union-absence-blank-violating/manifest.ttl | `3e801b3ffa954d5bb6b6e1747d7058a5430172585dbe7f363fecbb84e68db0c8` |
| shacl/union-absence-blank-violating/shapes.ttl | `94ca90aa729c18849c03b7dec38e7372e35747461c11af401df5f80a87d70a68` |
| shacl/union-absence-iri-conforming/manifest.ttl | `f3256d43f496d42ef84cd5020b2cf8f7c0b9e7a77c5fbcddce3340bebc5b8004` |
| shacl/union-absence-iri-conforming/shapes.ttl | `94ca90aa729c18849c03b7dec38e7372e35747461c11af401df5f80a87d70a68` |
| shacl/union-absence-iri-violating/manifest.ttl | `bbd49738fc3c9aceffe0a16d648a2c35a27518f8729390d7ae5595ac071dc371` |
| shacl/union-absence-iri-violating/shapes.ttl | `94ca90aa729c18849c03b7dec38e7372e35747461c11af401df5f80a87d70a68` |

## Agent admission-reason amendment approval

APPROVE the metadata-only Recommendation correction recorded in this amendment. Current inventory SHA-256: `621feb4d14b8b14a4860c84360a60eb2743ea84dde3feded846632e8faaf1a97`. The `admission-prebound-values-rec` reason is `explicit-values`, because REC Appendix A forbids every explicit VALUES clause, regardless of the variable mentioned. The draft retains its separate `prebound-values` rule. Independently rechecked all 133 non-inventory payload hashes above: every byte remains unchanged (134 total recorded entries include the changed inventory). Prior oracle approval and pre-observation amendment record are retained. No admission-case output or production grader was used to approve this change.

## Agent SHACL-AF profile amendment approval

APPROVE the metadata-only amendment recorded here. Current inventory SHA-256: `22cebc8d7caa2a51a01ca5444e6878b1b0d73712b054c608d4b5c6a322e1c587`. Independently reversing only the two added Recommendation profile entries and dated REC prebinding references reproduces prior inventory SHA-256 `621feb4d14b8b14a4860c84360a60eb2743ea84dde3feded846632e8faaf1a97` exactly. All 133 non-inventory payload hashes remain unchanged. OPTIONAL and NOT EXISTS are admitted under both dated prebinding contracts, and the SPARQL rule behavior remains separately governed by the pinned SHACL-AF NOTE20170608. No rule observation or production output was used in this approval.

## Agent ASK declaration correction approval

APPROVE the correction recorded in this amendment. The original agent review missed that `sh:nodeValidator` requires a SELECT validator under REC §6.2.3 and the dated draft. An ASK validator belongs at generic `sh:validator`; both ASK parameter shapes now have exactly that link. Independently reversing only `sh:validator` to `sh:nodeValidator` reproduces original SHA-256 `03b4c4a033ab369cb6d702d87e8ac96ce0df32d1f91f423b3c678851d0b8b38c` from current `b2f7e31d962b16cc9df9f8ca2c995de2d8144a3721df83fa30920c6ee9f4e005` for both files. All other RDF/source payloads remain byte-identical; inventory remains `22cebc8d7caa2a51a01ca5444e6878b1b0d73712b054c608d4b5c6a322e1c587`. The independent expected b/c violation reports are unchanged. The failed first observations and prior mistaken approval are retained; this is a prospective correction, not a retrospective approval of the invalid original declarations.

Reviewer: automated agent source review, 3 October 2026. This is a community oracle review, without W3C approval or an engine conformance certification.

## Agent re-review of the current bytes, 6 October 2026

The bytes approved above as inventory SHA-256 `22cebc8d7caa2a51a01ca5444e6878b1b0d73712b054c608d4b5c6a322e1c587` were not retained. The candidate inventory that survived differed from them, and ten admission manifests (hidden-prebound-subquery-rec, local-values-rec, and the minus, prebound-as, prebound-values and service pairs) differed from the earlier table only by a removed trailing blank line. Neither difference can be reconciled from retained evidence, so the earlier approval does not cover these bytes. All 64 cases were therefore re-derived from the dated clauses before any engine observation, by an automated agent reviewer that saw no PurRDF or reference-engine output.

The re-review confirms the result sets, their multiplicity and every result field: the 40 absence controls (REC §5.3.2, WD §3.3.2 values insertion), the four validator-parameter cases (REC §6.2.3, §6.3), the two predicate-position cases, the twelve admission cases (REC and WD Appendix A), the declared failure, the message priority, the RDF 1.2 quoted focus, the sequence/inverse result path, and the two SHACL-AF rule deltas, whose inline `mf:result` lists equal their expected graphs.

It amends three expectations before observation. The Recommendation's SERVICE refusal is an outright prohibition ("must not contain a federated query"), so `admission-service-rec` names the reason `service` rather than `service-policy`. The dated draft only advises against SERVICE and requires a processor that refuses it to report failure, so `admission-service-draft` is categorized `processor-policy` and its extension metadata states that it grades PurRDF's declared refusal policy, not a universal draft requirement. Its expected refusal is unchanged. The citations of `complex-sequence-inverse-path` now name the Core minCount, property path and result path clauses of both dated editions, and `quoted-focus-optional` cites only the dated draft that its profile selects.

Two readings are recorded rather than amended. `admission-local-values-draft` and `admission-hidden-prebound-subquery-draft` expect focus `a` because values insertion applies to the group pattern before any algebra simplification; SPARQL 1.1 §18.2.2.8 permits replacing Join(Z, A) by A, and applying that first would leave `this` unbound. The expectation follows the dated drafts' insertion order. `admission-hidden-prebound-subquery-draft` has one `?s ex:p ?o` triple, so it does not separate scoped insertion from an unscoped subquery, and the path-absence cases never exercise the `ex:q` alternative.

The catalog now lists only this SHACL suite, every reviewer label states that the review is an automated agent review, and the payload table above records the current bytes. Current inventory SHA-256: `9ae8dda64e130af80e9e9e9c5e5df783d2430b67a07b195417fc976654f99d91`.

Reviewer: automated agent source review, 6 October 2026. This is a community oracle review, without W3C approval or an engine conformance certification.

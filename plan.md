# Group output ownership

Implement the complete issue 481 contract on the settled main source. The production group worker currently owns per-chunk minted buffers, then a complete flattened minted buffer, then the final retained-row output. Input and group-member indices survive until workers finish; governed folds retain input for continuation. Plain folds release oversized input before output admission and reuse it only when capacity does not exceed grouped output length.

Keep original worker buffers ordered through output conversion. Share chunk and bounded-block scheduling with existing flat callers, retaining their original flat adapter and allocation shape. Check operational errors in source order before output sizing/admission. Check the total row count and chunk metadata allocation fallibly. An exact-length iterator feeds the existing admission and ItemLedger replay homes; no second governor implementation, cell representation, unsafe cast or survivor copy.

Qualify original-buffer identity/order, checked overflow and first-error selection, public numeric exact answers and GovernorEvidence, continuation/refusal boundaries, BIND and SHACL controls, native and actual portable runtime. Use existing allocation/instruction collectors at 1/4/32 workers against current main, reporting calls, requested/retained/peak bytes and instructions. No timing acceptance. Required hooks and hosted qualification remain mandatory. Do not claim improvement before measurement.

Independent repair review and sequential ghprsq integration follow the same queue workflow. Parent arithmetic and maintenance requirements remain separate.

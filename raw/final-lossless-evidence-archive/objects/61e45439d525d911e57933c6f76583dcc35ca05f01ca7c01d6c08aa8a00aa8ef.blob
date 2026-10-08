# Independent nested EXISTS root-cause diagnosis

Diagnosis: the observed reduced witness is rejected by erroneous compatibility Filter isolation before the innermost EXISTS runs. This is a confirmed source/value/oracle mismatch; acceptance of a repair remains pending. No shipping mutation or candidate build was performed by this reviewer.

## Observed evidence

The signed core commit is e72660e8769af222387ecc4fd4f84ea9887e43a1; current Task2 source is mutable. Inspected plan diagnostic SHA256 4dba89d2f03b4b538a4fced180f4053746644e6b7ccac3d1d74ade9148b59877 and runtime value diagnostic SHA256 7498103ce1a6c63369e8feb7334b037605b7856105ba4a60cca42b4017f7d4a7 are the actual failed attempt, not passes. Its source manifest records rdflib.rs ee1d383295601cd8bbe8eb3cd34ea5d613f07326babc8f435c6b2e3e5e137ceb, binop.rs df537c072517526fd057234e0838879e5714a37204e0b8ad81dc856c87dc2257, and reduced test d45a8239360126b680eb3e8dc22d4799da0304b798be6999e619a6a90b41556d.

The reduced original Rust case has outer s/q/z and s/p/x, middle s/p/y with x != y, and inner NOT EXISTS z/r/y. For the nonmatching fifth triple it should yield one row, but the actual candidate yields two. Runtime Apply probes show enclosing inputs C_s=a, C_x=b, C_z=z; intermediate mapping cells include s=a, x=b, y=b or c, z=z. Thus actual x and z values have reached the middle driver. The log contains no invocation of the innermost context Apply. This contradicts treating a mere absent transport column as the immediate observed cause.

The compiled middle inequality reads x as If(Bound(C_x), empty Coalesce/error, R_x). With C_x bound, it deliberately becomes an error, removing every middle witness. The enclosing NOT EXISTS consequently returns true for both outer rows. rdflib.rs Filter compilation chooses this forgetting expression whenever only that node's exists_root_filter fact is false; the compiler marks only the outermost source Filter of each EXISTS body. The native parser represents two same-group FILTER clauses as nested Filter wrappers.

Genuine rdflib 7.6.0 was inspected behaviorally and through metadata using the existing oracle environment. The independent receipt raw/t2-independent-exists-filter-oracle.log (SHA256 474848e38d4e579402884c9f5243a024b888a67319b278c9ec1b31b797c0a7ca) shows the reduced body's two same-group conditions become ONE ConditionalAndExpression in one Filter with no_isolated_scope=True. Both conditions therefore retain enclosing bindings. An independently executed genuine query returns one row for nonmatching and two for matching data. No implementation body was copied into shipping code.

## Group-boundary constraint and recommendation

Marking every leading Filter non-isolated is unsafe. The actual genuine counterexample

    SELECT ?x WHERE { VALUES ?x {1} FILTER EXISTS {
      FILTER(?x=1) { FILTER(?x=1) VALUES ?y {1} }
    } }

returns an empty bag. Its translated outer Filter is non-isolated and its nested brace group's Filter is isolated. The oracle receipt records both metadata values. Native parser.rs group_join delegates to join when the left operand is an empty BGP; join removes that identity operand. Thus two syntactic groups can become a consecutive Filter chain, so chain shape alone does not distinguish ownership.

The smallest coherent existing-home remedy is contextual-only filter grouping at the parser's actual GroupState finalization in parser/machine.rs: conjoin that group's own collected filters in written order with the existing Expression::and home, wrap one Filter, and report one group-owned filter to the existing OPTIONAL split_trailing_filters home. Keep the ordinary const-false parsing path's current wrapper loop and count unchanged. Existing root-only EXISTS marking then applies to the actual single group's conjunction, while a nested brace group remains a separate inner Filter. Do not add another parser/evaluator, runtime dependency, or dynamic ordinary-path check.

Actual inspected parser identities: parser.rs d2bbb8db903546272390232ca3c5d3c1ce1a30e3f3d0ae0d5ea032ee11fd20c3; parser/machine.rs 907e547db5206d58cee68bb21944c70214d6376efd6079eed6793bab876cabde. expr.rs f61cb32394a67e706b12d160e2d2bc847f14a04f5e16bc061ba54395ec8fb0c2. These identify pre-repair observations and are not final source qualification.

Existing VM compile.rs And compilation emits every operand and EBV conversion before Kleene combination. Genuine attribute evaluation evaluates list expressions before its conjunction reduction. This supports reusing the existing expression home; it does not replace executable callback/error work qualification.

## Required owned execution and proof

Execute the reduced positive/negative witness and original vendor failure after the coherent fix. Include same-group two/three FILTERs, source-order permutations, nested explicit-brace boundary, OPTIONAL's own versus nested filters, outer variable absent from immediate BGP, middle VALUES and BGP/nonlazy-Join cases. Compare configured callback counts/order and error/cutoff/ASK controls with genuine behavior; preserve the full 241-case matrix and labelled/unlabelled BNODE tests. Verify ordinary parser/evaluator/prebinding/SHACL laws remain unchanged. Rerun installed Python vendor/shadow/full affected qualification on the final module identity.

Remove the two failed context-driver/private-Project repairs unless a separate actual witness justifies them. Their failure does not prove them universally unnecessary, but this diagnosis gives no evidence they repair the observed failure. Extra physical context columns must never silently alter logical mapping domains, MINUS/DISTINCT/group keys, result width or allocation claims. Remove temporary diagnostic prints before any commit and keep attributable failed receipts.

A contextual-only parser specialization can preserve ordinary source operations by construction, but final native source/codegen cost qualification must bind the actual final delta. No timing measurements or additional wasm work are part of this diagnosis. Root-cause diagnosis is complete; repair execution and final acceptance remain pending.

Additional corroboration: writer-owned parsed source receipt raw/t2-nested-context-source-groups-diagnostic.log SHA256 67cb22eeb26bba68b2f32ab9494b817b3d57deca64a4f8a6b443c29b72c49481 was read at its two actual source-print lines. It confirms ordinary parsing produces identical Filter(Filter(Values)) structures for two same-group predicates versus the explicit nested-brace counterexample. The run still fails the original reduced case (2 versus 1); it is diagnostic evidence only. The parser per-group filter collection, before identity-join simplification loses ownership, is therefore the necessary discrimination point for the proposed contextual specialization.

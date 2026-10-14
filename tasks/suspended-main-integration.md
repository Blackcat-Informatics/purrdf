# Suspended evaluator integration

Maincb38600b6 is integrated in619597ca7 through a normal hooked merge. The
original synchronous181-depth assertion is restored in1057a7846; it remains
paired with exact synchronous/asynchronous endpoint equality.

Two conflicts were evaluated. Release notes retain both the ProjectStar source
break and remembered graph default/explicit opt-out migration. The old separate
finish_node_evaluation helper is superseded by main's NodeBoundary::finish,
which already keeps evidence completion outside the caller frame, restores the
entered cursor/cap, preserves the original source-error precedence and coalesces
evidence. Retaining both completion implementations would duplicate the job and
discard the suspended driver. ProjectStar dispatch, shared projected names and
its persistent projection ownership remain in the integrated evaluator.

Actual bounded standard cargo check for evaluator/WASM all targets completed
successfully in71609; receipt: integrated-suspended-projectstar-check.log.
The original nested-star first-appearance/binding runtime99586 is active. No
new runtime, optimized package or hosted pass is inferred from compilation.
Historical450/452 package failures remain attributed to their earlier source.

This is a self integration assessment, not a fresh independent task review.
Existing original-source task reviews remain relevant only to their unchanged
components; final package and complete-contract acceptance are still open.

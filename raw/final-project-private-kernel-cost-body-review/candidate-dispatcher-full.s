purrdf_sparql_eval::eval::eval_node::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin234:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	subq	$312, %rsp
	.cfi_def_cfa_offset 336
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movl	(%rsi), %eax
	movl	$1, %ecx
	movq	%rdx, %r10
	leaq	.LJTI234_0(%rip), %rdx
	subl	$10, %eax
	cmovael	%eax, %ecx
	movslq	(%rdx,%rcx,4), %rcx
	addq	%rdx, %rcx
	jmpq	*%rcx
.LBB234_1:
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	%r10, %rcx
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_dedup_with::<purrdf_core::ir::dataset::RdfDataset, false>@GOTPCREL(%rip)
.LBB234_3:
	.cfi_def_cfa_offset 336
	movq	24(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	32(%rsi), %rcx
	movq	8(%rsi), %r8
	movq	16(%rsi), %r9
	movq	purrdf_sparql_eval::modifier::eval_slice::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %rax
	movq	%r10, (%rsp)
	callq	*%rax
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB234_5:
	.cfi_def_cfa_offset 336
	movq	16(%rsi), %rax
	movq	24(%rsi), %rdx
	movq	40(%rsi), %rcx
	movq	48(%rsi), %r8
	movq	purrdf_sparql_eval::modifier::eval_values::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %r11
	movq	%rdi, %r14
	leaq	208(%rsp), %rdi
	leaq	216(%rsp), %rbx
	movq	%r10, %r9
	movq	%rax, %rsi
	callq	*%r11
	movq	208(%rsp), %rax
	cmpq	$-1, %rax
	je	.LBB234_28
	vmovups	248(%rsp), %ymm0
	vmovups	272(%rsp), %ymm1
	jmp	.LBB234_27
.LBB234_7:
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>@GOTPCREL(%rip)
.LBB234_10:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	16(%rsi), %rcx
	movq	24(%rsi), %r8
	movq	%r10, %r9
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_order_by::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_12:
	.cfi_def_cfa_offset 336
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_minus::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_15:
	.cfi_def_cfa_offset 336
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_join::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_18:
	.cfi_def_cfa_offset 336
	movq	72(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	80(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	xorl	%r8d, %r8d
	cmpl	$-1, 8(%rsi)
	leaq	8(%rsi), %rax
	movq	%r10, %r9
	cmovneq	%rax, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_left_join::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_21:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	leaq	8(%rsi), %rdx
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_graph_with::<purrdf_core::ir::dataset::RdfDataset, ()>@GOTPCREL(%rip)
.LBB234_23:
	.cfi_def_cfa_offset 336
	movq	16(%rsi), %rax
	movq	24(%rsi), %rdx
	movq	purrdf_sparql_eval::bgp::eval_bgp::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %r8
	movq	%rdi, %r14
	leaq	16(%rsp), %rdi
	leaq	24(%rsp), %rbx
	movq	%r10, %rcx
	movq	%rax, %rsi
	callq	*%r8
	movq	16(%rsp), %rax
	cmpq	$-1, %rax
	je	.LBB234_28
	vmovups	56(%rsp), %ymm0
	vmovups	80(%rsp), %ymm1
	jmp	.LBB234_27
.LBB234_25:
	movq	purrdf_sparql_eval::path::eval_path::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %r9
	movq	%rdi, %r14
	leaq	32(%rsi), %rax
	leaq	88(%rsi), %rcx
	leaq	112(%rsp), %rdi
	movq	%rsi, %rdx
	leaq	120(%rsp), %rbx
	movq	%r10, %r8
	movq	%rax, %rsi
	callq	*%r9
	movq	112(%rsp), %rax
	cmpq	$-1, %rax
	je	.LBB234_28
	vmovups	152(%rsp), %ymm0
	vmovups	176(%rsp), %ymm1
.LBB234_27:
	vmovups	%ymm1, 80(%r14)
	vmovups	(%rbx), %ymm1
	vmovups	%ymm0, 56(%r14)
	vmovups	%ymm1, 24(%r14)
	movq	%rax, 16(%r14)
	movq	$1, (%r14)
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.LBB234_28:
	.cfi_def_cfa_offset 336
	vmovups	(%rbx), %ymm0
	vmovups	%ymm0, 16(%r14)
	movq	$-1, 8(%r14)
	movq	$0, (%r14)
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.LBB234_29:
	.cfi_def_cfa_offset 336
	movq	56(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	16(%rsi), %rcx
	movq	24(%rsi), %r8
	movq	40(%rsi), %r9
	movq	48(%rsi), %rax
	movq	purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, ()>@GOTPCREL(%rip), %rbx
	movq	%r10, 8(%rsp)
	movq	%rax, (%rsp)
	jmp	.LBB234_46
.LBB234_31:
	movq	72(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	leaq	8(%rsi), %rdx
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_33:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	16(%rsi), %rcx
	movq	24(%rsi), %r8
	movq	%r10, %r9
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_35:
	.cfi_def_cfa_offset 336
	movq	16(%rsi), %rdx
	movq	24(%rsi), %rcx
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_union::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_36:
	.cfi_def_cfa_offset 336
	movq	16(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	24(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	movq	8(%rsi), %r8
	movq	%r10, %r9
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_apply::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_39:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB234_47
	movzbl	40(%rsi), %r8d
	leaq	8(%rsi), %rdx
	movq	%r10, %r9
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::remote::eval_service::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_41:
	.cfi_def_cfa_offset 336
	movq	88(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	leaq	8(%rsi), %r8
	leaq	72(%rsi), %rcx
	movq	%r10, %r9
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_43:
	.cfi_def_cfa_offset 336
	addq	$8, %rsi
	movq	%r10, %rdx
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::property_fn_eval::eval_property_function::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB234_44:
	.cfi_def_cfa_offset 336
	movq	88(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB234_47
	movq	96(%rsi), %rax
	movq	purrdf_sparql_eval::cdt_unfold::eval_unfold::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %rbx
	leaq	96(%rsi), %r9
	leaq	72(%rsi), %r8
	leaq	8(%rsi), %rcx
	movq	%r10, (%rsp)
	testq	%rax, %rax
	cmoveq	%rax, %r9
.LBB234_46:
	callq	*%rbx
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB234_47:
	.cfi_def_cfa_offset 336
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	anon.e5162873a9a3251d11c4df37a70e4654.2.llvm.2910935600939035342(%rip), %rdi
	leaq	anon.e5162873a9a3251d11c4df37a70e4654.4.llvm.2910935600939035342(%rip), %rdx
	movl	$48, %esi
	callq	*%rax
.Lfunc_end234:

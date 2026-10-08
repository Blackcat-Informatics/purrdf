purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}:
.Lfunc_begin1268:
	.cfi_startproc
	cmpq	%rcx, (%rdi)
	jae	.LBB1268_6
	pushq	%rbp
	.cfi_def_cfa_offset 16
	pushq	%r15
	.cfi_def_cfa_offset 24
	pushq	%r14
	.cfi_def_cfa_offset 32
	pushq	%r13
	.cfi_def_cfa_offset 40
	pushq	%r12
	.cfi_def_cfa_offset 48
	pushq	%rbx
	.cfi_def_cfa_offset 56
	subq	$88, %rsp
	.cfi_def_cfa_offset 144
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdx, %r14
	leaq	16(%rsp), %rbp
	addq	$888, %r14
	leaq	8(%rsp), %r13
	movq	%rcx, %rbx
	movq	%rsi, %r15
	movq	%rdi, %r12
	.p2align	4
.LBB1268_2:
	movq	8(%r15), %rax
	cmpq	24(%r15), %rax
	je	.LBB1268_5
	leaq	88(%rax), %rcx
	movq	%rcx, 8(%r15)
	movq	8(%rax), %rcx
	cmpq	$-1, %rcx
	je	.LBB1268_5
	movq	(%rax), %rsi
	movq	%rcx, 8(%rsp)
	movq	%r14, %rdi
	movq	%r13, %rdx
	movq	80(%rax), %rcx
	movq	%rcx, 64(%rbp)
	vmovups	16(%rax), %zmm0
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	vmovups	%zmm0, (%rbp)
	incq	(%r12)
	vzeroupper
	callq	*%rax
	cmpq	%rbx, (%r12)
	jb	.LBB1268_2
.LBB1268_5:
	addq	$88, %rsp
	.cfi_def_cfa_offset 56
	popq	%rbx
	.cfi_def_cfa_offset 48
	popq	%r12
	.cfi_def_cfa_offset 40
	popq	%r13
	.cfi_def_cfa_offset 32
	popq	%r14
	.cfi_def_cfa_offset 24
	popq	%r15
	.cfi_def_cfa_offset 16
	popq	%rbp
	.cfi_def_cfa_offset 8
	.cfi_restore %rbx
	.cfi_restore %r12
	.cfi_restore %r13
	.cfi_restore %r14
	.cfi_restore %r15
	.cfi_restore %rbp
.LBB1268_6:
	retq
.Lfunc_end1268:

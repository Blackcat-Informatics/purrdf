purrdf_sparql_eval::eval::eval_node::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin236:
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
	leaq	.LJTI236_0(%rip), %rdx
	subl	$10, %eax
	cmovael	%eax, %ecx
	movslq	(%rdx,%rcx,4), %rcx
	addq	%rdx, %rcx
	jmpq	*%rcx
.LBB236_1:
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	%r10, %rcx
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_dedup_with::<purrdf_core::ir::dataset::RdfDataset, false>@GOTPCREL(%rip)
.LBB236_3:
	.cfi_def_cfa_offset 336
	movq	24(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
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
.LBB236_5:
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
	je	.LBB236_28
	vmovups	248(%rsp), %ymm0
	vmovups	272(%rsp), %ymm1
	jmp	.LBB236_27
.LBB236_7:
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>@GOTPCREL(%rip)
.LBB236_10:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
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
.LBB236_12:
	.cfi_def_cfa_offset 336
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_minus::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB236_15:
	.cfi_def_cfa_offset 336
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_join::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB236_18:
	.cfi_def_cfa_offset 336
	movq	72(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	80(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
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
.LBB236_21:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
	leaq	8(%rsi), %rdx
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_graph_with::<purrdf_core::ir::dataset::RdfDataset, ()>@GOTPCREL(%rip)
.LBB236_23:
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
	je	.LBB236_28
	vmovups	56(%rsp), %ymm0
	vmovups	80(%rsp), %ymm1
	jmp	.LBB236_27
.LBB236_25:
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
	je	.LBB236_28
	vmovups	152(%rsp), %ymm0
	vmovups	176(%rsp), %ymm1
.LBB236_27:
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
.LBB236_28:
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
.LBB236_29:
	.cfi_def_cfa_offset 336
	movq	56(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	16(%rsi), %rcx
	movq	24(%rsi), %r8
	movq	40(%rsi), %r9
	movq	48(%rsi), %rax
	movq	purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, ()>@GOTPCREL(%rip), %rbx
	movq	%r10, 8(%rsp)
	movq	%rax, (%rsp)
	jmp	.LBB236_46
.LBB236_31:
	movq	72(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
	leaq	8(%rsi), %rdx
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB236_33:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
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
.LBB236_35:
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
.LBB236_36:
	.cfi_def_cfa_offset 336
	movq	16(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	24(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
	movq	8(%rsi), %r8
	movq	%r10, %r9
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_apply::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB236_39:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB236_47
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
.LBB236_41:
	.cfi_def_cfa_offset 336
	movq	88(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
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
.LBB236_43:
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
.LBB236_44:
	.cfi_def_cfa_offset 336
	movq	88(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB236_47
	movq	96(%rsi), %rax
	movq	purrdf_sparql_eval::cdt_unfold::eval_unfold::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %rbx
	leaq	96(%rsi), %r9
	leaq	72(%rsi), %r8
	leaq	8(%rsi), %rcx
	movq	%r10, (%rsp)
	testq	%rax, %rax
	cmoveq	%rax, %r9
.LBB236_46:
	callq	*%rbx
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB236_47:
	.cfi_def_cfa_offset 336
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	anon.e5162873a9a3251d11c4df37a70e4654.2.llvm.12908414067662811932(%rip), %rdi
	leaq	anon.e5162873a9a3251d11c4df37a70e4654.4.llvm.12908414067662811932(%rip), %rdx
	movl	$48, %esi
	callq	*%rax
.Lfunc_end236:
purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin243:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception164
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
	subq	$1960, %rsp
	.cfi_def_cfa_offset 2016
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, 88(%rsp)
	leaq	936(%rsp), %rdi
	movq	%r9, %r13
	movq	%r8, %r14
	movq	%rcx, %rbp
	movq	%rdx, %rbx
	movq	%rsi, %r12
	callq	*%rax
.Ltmp7186:
	leaq	1840(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r13, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp7187:
	cmpl	$1, 1840(%rsp)
	jne	.LBB243_25
	vmovdqu64	1856(%rsp), %zmm0
	vmovdqu64	1888(%rsp), %zmm1
	movq	88(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movq	1008(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB243_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1016(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_5
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB243_8:
	cmpq	%rax, %rsi
	jge	.LBB243_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB243_8
.LBB243_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB243_12:
	movq	936(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB243_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	944(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_15
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB243_18:
	cmpq	%rax, %rsi
	jge	.LBB243_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB243_18
.LBB243_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB243_22:
	movq	1032(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_386
	lock		decq	(%rax)
	jne	.LBB243_386
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1032(%rsp), %rdi
	#MEMBARRIER
	jmp	.LBB243_385
.LBB243_25:
	vmovdqu64	1880(%rsp), %zmm1
	vmovdqu64	1848(%rsp), %zmm0
	vmovdqu64	%zmm1, 272(%rsp)
	vmovdqu64	%zmm0, 240(%rsp)
.Ltmp7188:
	leaq	592(%rsp), %rdi
	leaq	936(%rsp), %rsi
	leaq	240(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp7189:
	cmpq	$-1, 592(%rsp)
	je	.LBB243_55
	vmovdqu	592(%rsp), %ymm0
	vmovdqu64	976(%rsp), %zmm1
	vmovdqu64	936(%rsp), %zmm2
	movb	$1, %al
	movl	%eax, 8(%rsp)
	vmovdqu64	%zmm1, 1080(%rsp)
	vmovdqu	%ymm0, 176(%rsp)
	vmovdqu64	%zmm2, 1040(%rsp)
.Ltmp7190:
	movq	%r13, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp7191:
	cmpb	$2, 472(%r13)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB243_34
	movq	616(%r13), %rax
	testq	%rax, %rax
	je	.LBB243_33
	cmpq	$-2, 24(%rax)
	jb	.LBB243_34
	cmpq	$-2, 32(%rax)
	jb	.LBB243_34
	cmpq	$-3, 48(%rax)
	jbe	.LBB243_34
.LBB243_33:
	movzbl	1234(%r13), %ebx
	cmpq	$1025, 192(%rsp)
	setae	%al
	notb	%bl
	andb	%al, %bl
	jmp	.LBB243_35
.LBB243_34:
	xorl	%ebx, %ebx
.LBB243_35:
	movq	192(%rsp), %r15
.Ltmp7192:
	movzbl	%bl, %edx
	leaq	1208(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r15, %rcx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7193:
	movq	200(%rsp), %rsi
	addq	$16, %rsi
.Ltmp7194:
	leaq	1152(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp7195:
	movq	(%rbp), %rsi
	lock		incq	(%rsi)
	jle	.LBB243_437
	movq	8(%rbp), %rdx
	movq	%r13, 16(%rsp)
.Ltmp7197:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	1152(%rsp), %rdi
	callq	*%rax
.Ltmp7198:
	vmovdqu	1176(%rsp), %ymm0
	vmovdqu	1152(%rsp), %ymm1
	movq	%rax, 520(%rsp)
	movq	1168(%rsp), %rax
	movq	malloc@GOTPCREL(%rip), %r13
	movl	$72, %edi
	movq	%rax, 232(%rsp)
	vmovdqu	%ymm0, 280(%rsp)
	vmovdqu	%ymm1, 256(%rsp)
	movq	$1, 240(%rsp)
	movq	$1, 248(%rsp)
	vzeroupper
	callq	*%r13
	testq	%rax, %rax
	je	.LBB243_433
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB243_42
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_42:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_48
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_42
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	addq	$72, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB243_45:
	cmpq	%rax, %rdx
	jle	.LBB243_47
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB243_45
.LBB243_47:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_48:
	vmovdqu64	240(%rsp), %zmm0
	movq	304(%rsp), %rax
	movq	%r15, 224(%rsp)
	movq	%rcx, 80(%rsp)
	movq	%rax, 64(%rcx)
	movb	$1, %al
	movl	%eax, 8(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp7202:
	movq	16(%rsp), %r15
	movq	%r12, %rsi
	movq	%r14, %rdx
	movq	%r15, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7203:
	movq	80(%rsp), %rcx
	addq	$16, %rcx
.Ltmp7204:
	leaq	1624(%rsp), %rbp
	movq	%rax, %rsi
	movq	%r14, %rdx
	movq	%r15, %r8
	movq	%rbp, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7205:
	testb	%bl, %bl
	je	.LBB243_60
	movq	16(%rsp), %r13
	movq	1392(%rsp), %rbx
	movq	224(%rsp), %rax
	movq	184(%rsp), %rdx
	cmpq	%rax, %rbx
	movq	1040(%r13), %rcx
	movq	%rdx, 8(%rsp)
	cmovaeq	%rax, %rbx
	addq	904(%r13), %rcx
	movq	%rcx, 872(%rsp)
.Ltmp7221:
	movq	%r13, %rdi
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp7222:
	movq	616(%r13), %rcx
	leaq	1208(%rsp), %rdi
	movq	%rax, 160(%rsp)
	testq	%rcx, %rcx
	je	.LBB243_125
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB243_126
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB243_126
.LBB243_55:
	movq	1032(%rsp), %r14
	testq	%r14, %r14
	je	.LBB243_70
	lock		incq	(%r14)
	jle	.LBB243_437
	movq	%r14, 240(%rsp)
	leaq	16(%r14), %rsi
.Ltmp7369:
	leaq	1624(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp7370:
	lock		decq	(%r14)
	jne	.LBB243_74
	#MEMBARRIER
.Ltmp7375:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	240(%rsp), %rdi
	callq	*%rax
.Ltmp7376:
	jmp	.LBB243_74
.LBB243_60:
	movq	224(%rsp), %rbx
	leaq	(,%rbx,8), %rax
	leaq	(%rax,%rax,4), %r15
	testq	%rbx, %rbx
	je	.LBB243_101
	movq	%r15, %rdi
	callq	*%r13
	testq	%rax, %rax
	je	.LBB243_435
	movq	%rax, %r14
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	leaq	(%r15,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%r15, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%r15, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB243_64
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_64:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_64
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%r15, (%rdx)
	movq	%r15, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%r15), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%r15, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB243_67:
	cmpq	%rax, %rdx
	jle	.LBB243_69
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_67
.LBB243_69:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jmp	.LBB243_102
.LBB243_70:
.Ltmp7377:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp7378:
	movq	%rax, %rsi
	movq	%rax, %rbx
	movq	%rax, 240(%rsp)
	addq	$16, %rsi
.Ltmp7379:
	leaq	1624(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp7380:
	lock		decq	(%rbx)
	jne	.LBB243_74
	#MEMBARRIER
.Ltmp7385:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	240(%rsp), %rdi
	callq	*%rax
.Ltmp7386:
.LBB243_74:
	movq	(%rbp), %rsi
	lock		incq	(%rsi)
	jle	.LBB243_437
	movq	8(%rbp), %rdx
.Ltmp7388:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	1624(%rsp), %rdi
	callq	*%rax
.Ltmp7389:
	vmovdqu64	976(%rsp), %zmm1
	vmovups	936(%rsp), %zmm0
	vmovdqu	1648(%rsp), %ymm2
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vmovdqu64	%zmm1, 280(%rsp)
	vmovups	%zmm0, 240(%rsp)
	vmovdqu	1624(%rsp), %ymm0
	vmovdqu	%ymm2, 632(%rsp)
	vmovdqu	%ymm0, 608(%rsp)
	movq	$1, 592(%rsp)
	movq	$1, 600(%rsp)
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB243_434
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	movabsq	$9223372036854775807, %rbx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rbx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB243_79
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_79:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_85
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_79
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	movq	(%rsi), %rax
	addq	$72, %rdx
	cmovoq	%rbx, %rdx
	.p2align	4
.LBB243_82:
	cmpq	%rax, %rdx
	jle	.LBB243_84
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB243_82
.LBB243_84:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_85:
	vmovups	592(%rsp), %zmm0
	movq	656(%rsp), %rax
	cmpq	$-1, 240(%rsp)
	movq	%rcx, 1232(%rsp)
	movq	$0, 1208(%rsp)
	movq	$8, 1216(%rsp)
	movq	$0, 1224(%rsp)
	movq	%rax, 64(%rcx)
	vmovups	%zmm0, (%rcx)
	je	.LBB243_87
	leaq	592(%rsp), %rdi
	leaq	1208(%rsp), %rsi
	leaq	936(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	312(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB243_88
	jmp	.LBB243_97
.LBB243_87:
	movq	1216(%rsp), %rcx
	movq	1208(%rsp), %rax
	movq	1224(%rsp), %rdx
	movq	%rcx, 608(%rsp)
	movq	1232(%rsp), %rcx
	movq	%rax, 600(%rsp)
	movq	%rdx, 616(%rsp)
	movq	%rcx, 624(%rsp)
	movq	$-1, 592(%rsp)
	movq	312(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB243_97
.LBB243_88:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	320(%rsp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_90
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_90:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_96
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_90
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB243_93:
	cmpq	%rax, %rdx
	jge	.LBB243_95
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_93
.LBB243_95:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_96:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB243_97:
	movq	336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_100
	lock		decq	(%rax)
	jne	.LBB243_100
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	336(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB243_100:
	vmovdqu64	592(%rsp), %zmm0
	vmovdqu64	624(%rsp), %zmm1
	movq	88(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB243_386
.LBB243_101:
	movl	$8, %r14d
.LBB243_102:
	movq	184(%rsp), %rax
	movq	176(%rsp), %rcx
	movq	%rbx, 128(%rsp)
	movq	%r14, 136(%rsp)
	movq	%r12, 168(%rsp)
	movq	$0, 144(%rsp)
	addq	%rax, %r15
	movq	%rax, 32(%rsp)
	movq	%rcx, 48(%rsp)
	movq	%rcx, 216(%rsp)
	movq	%rax, 840(%rsp)
	movq	%r15, 152(%rsp)
	movq	%r15, 56(%rsp)
	testq	%rbx, %rbx
	je	.LBB243_124
	leaq	600(%rsp), %r13
	leaq	240(%rsp), %rbx
	xorl	%r12d, %r12d
	jmp	.LBB243_106
	.p2align	4
.LBB243_104:
	movq	136(%rsp), %rdx
.LBB243_105:
	leaq	(%r12,%r12,4), %rax
	movq	%r15, %r12
	movq	%r14, (%rdx,%rax,8)
	movq	%r13, 8(%rdx,%rax,8)
	movq	%rdx, %r14
	movq	%rbp, %r13
	vmovdqa	240(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	256(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	8(%rsp), %rdx
	movq	%r15, 144(%rsp)
	movq	%rdx, %rax
	cmpq	152(%rsp), %rdx
	je	.LBB243_182
.LBB243_106:
	movq	(%rax), %rcx
	leaq	40(%rax), %r15
	testq	%rcx, %rcx
	je	.LBB243_183
	vmovups	8(%rax), %ymm0
	movq	%r15, 8(%rsp)
	leaq	1(%r12), %r15
	movq	%r12, %rbp
	movq	%rcx, 592(%rsp)
	vmovups	%ymm0, 528(%rsp)
	vmovups	%ymm0, (%r13)
.Ltmp7208:
	movq	16(%rsp), %rdx
	leaq	1208(%rsp), %rsi
	movq	%rbx, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7209:
	cmpb	$-1, 240(%rsp)
	jne	.LBB243_146
	movq	592(%rsp), %rcx
	movq	608(%rsp), %rdx
	movq	232(%rsp), %rax
	leaq	-1(%rcx), %rsi
	leaq	-1(%rdx), %rdi
	cmpq	$5, %rsi
	cmovbq	%rsi, %rdi
	movq	%rax, %rsi
	subq	%rdi, %rsi
	jbe	.LBB243_111
	movl	$2, 240(%rsp)
	movq	%rsi, 248(%rsp)
.Ltmp7210:
	leaq	592(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp7211:
	jmp	.LBB243_113
	.p2align	4
.LBB243_111:
	cmpq	$6, %rcx
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB243_113
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, 592(%rsp,%rdx)
.LBB243_113:
	movq	592(%rsp), %rcx
	movq	16(%rsp), %rax
	movq	%r13, %rdx
	decq	%rcx
	movq	%rbp, 568(%rax)
	cmpq	$5, %rcx
	jb	.LBB243_115
	movq	608(%rsp), %rcx
	movq	600(%rsp), %rdx
	decq	%rcx
.LBB243_115:
	movq	80(%rsp), %r8
	addq	$16, %r8
.Ltmp7212:
	movq	16(%rsp), %r9
	leaq	1624(%rsp), %rsi
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7213:
	movq	240(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB243_149
	movq	592(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB243_119
	movq	608(%rsp), %rsi
.LBB243_119:
	movq	520(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB243_432
	movq	%r14, %rdx
	movq	%r13, %rbp
	movq	%r13, %rcx
	cmpq	$6, %rax
	jb	.LBB243_122
	movq	600(%rsp), %rcx
.LBB243_122:
	vmovsd	248(%rsp), %xmm0
	vmovlps	%xmm0, (%rcx,%rdi,8)
	vmovups	8(%rbp), %xmm0
	movq	24(%rbp), %rax
	movq	592(%rsp), %r14
	movq	600(%rsp), %r13
	movq	%rax, 256(%rsp)
	vmovaps	%xmm0, 240(%rsp)
	cmpq	128(%rsp), %r12
	jne	.LBB243_105
.Ltmp7218:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	128(%rsp), %rdi
	callq	*%rax
.Ltmp7219:
	jmp	.LBB243_104
.LBB243_124:
	xorl	%r12d, %r12d
	movq	%rax, %r15
	jmp	.LBB243_183
.LBB243_125:
	xorl	%eax, %eax
.LBB243_126:
	leaq	160(%rsp), %rdx
	movq	%r13, 528(%rsp)
	movzbl	1234(%r13), %ecx
	leaq	232(%rsp), %rsi
	leaq	80(%rsp), %r8
	leaq	520(%rsp), %r9
	movq	%rdx, 536(%rsp)
	movq	8(%rsp), %rdx
	movq	%rdi, 544(%rsp)
	movq	%rbp, 552(%rsp)
	movq	%rdx, 32(%rsp)
	movq	%rbx, 40(%rsp)
	movq	%rsi, 48(%rsp)
	movq	%r8, 56(%rsp)
	leaq	872(%rsp), %r8
	movq	%r9, 64(%rsp)
	movq	%r8, 72(%rsp)
	testb	%al, %al
	je	.LBB243_128
.Ltmp7225:
	movq	%rdi, (%rsp)
	movzbl	%cl, %esi
	leaq	240(%rsp), %rdi
	leaq	528(%rsp), %r8
	leaq	32(%rsp), %r9
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7226:
	jmp	.LBB243_129
.LBB243_128:
.Ltmp7223:
	movq	%rdi, (%rsp)
	movzbl	%cl, %esi
	leaq	240(%rsp), %rdi
	leaq	528(%rsp), %r8
	leaq	32(%rsp), %r9
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7224:
.LBB243_129:
	movq	240(%rsp), %rdx
	cmpq	$-1, %rdx
	je	.LBB243_137
	vmovups	432(%rsp), %zmm0
	vmovups	416(%rsp), %zmm1
	movq	264(%rsp), %rax
	vmovdqu	248(%rsp), %xmm2
	movq	280(%rsp), %rsi
	movq	272(%rsp), %rcx
	movq	%rdx, 848(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rdx
	cmpq	$-2, %rdx
	movl	$1, %edx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rdx
	vmovups	%zmm0, 736(%rsp)
	vmovups	%zmm1, 720(%rsp)
	vmovdqu64	288(%rsp), %zmm0
	vmovdqu64	352(%rsp), %zmm1
	decq	%rax
	vmovdqu	%xmm2, 856(%rsp)
	vmovdqu64	736(%rsp), %zmm4
	vmovdqu64	720(%rsp), %zmm3
	vmovdqu64	%zmm0, 592(%rsp)
	vmovdqu64	%zmm1, 656(%rsp)
	vmovdqu64	%zmm1, 328(%rsp)
	vmovdqu64	%zmm0, 264(%rsp)
	vmovdqu64	%zmm4, 408(%rsp)
	vmovdqu64	%zmm3, 392(%rsp)
	movq	%rdi, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	%rdx, 256(%rsp)
	movq	$0, 472(%rsp)
	movq	%rax, 480(%rsp)
.Ltmp7228:
	leaq	592(%rsp), %rdi
	leaq	240(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7229:
	vmovups	624(%rsp), %zmm2
	vmovups	688(%rsp), %zmm1
	vmovdqu	592(%rsp), %ymm0
	movq	616(%r13), %rbx
	vmovups	%zmm2, 1408(%rsp)
	vmovups	%zmm1, 1472(%rsp)
	vmovdqu64	752(%rsp), %zmm2
	vmovdqu64	768(%rsp), %zmm1
	vmovdqu	%ymm0, 528(%rsp)
	vmovdqu64	%zmm2, 1536(%rsp)
	vmovdqu64	%zmm1, 1552(%rsp)
	testq	%rbx, %rbx
	je	.LBB243_138
	cmpb	$2, 1402(%rsp)
	jne	.LBB243_143
	movq	1408(%rsp), %rax
	vmovdqu64	1432(%rsp), %zmm0
	vmovdqu64	1496(%rsp), %zmm1
	vmovdqu64	1552(%rsp), %zmm2
	movq	1424(%rsp), %rdx
	movq	1416(%rsp), %rcx
	movl	$1, %edi
	cmpq	$3, %rax
	movq	%rax, %rsi
	cmovaeq	%rdx, %rsi
	cmovaeq	%rdi, %rdx
	cmovaeq	%rax, %rdi
	decq	%rsi
	vmovdqu64	%zmm1, 328(%rsp)
	vmovdqu64	%zmm0, 264(%rsp)
	vmovdqu64	%zmm2, 384(%rsp)
	movq	%rdi, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	%rdx, 256(%rsp)
	movq	$0, 448(%rsp)
	movq	%rsi, 456(%rsp)
.Ltmp7241:
	leaq	128(%rsp), %rdi
	leaq	240(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7242:
	movq	144(%rsp), %r8
	movq	136(%rsp), %rcx
	imulq	$200, %r8, %rsi
	addq	%rcx, %rsi
	testq	%r8, %r8
	je	.LBB243_240
	movl	%r8d, %edi
	andl	$3, %edi
	cmpq	$4, %r8
	jae	.LBB243_214
	xorl	%r9d, %r9d
	xorl	%r11d, %r11d
	jmp	.LBB243_233
.LBB243_137:
	vmovdqu64	288(%rsp), %zmm0
	vmovdqu	256(%rsp), %ymm1
	movq	88(%rsp), %rax
	vmovdqu64	%zmm0, 48(%rax)
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu64	%zmm0, 592(%rsp)
	movq	$1, (%rax)
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB243_262
	jmp	.LBB243_264
.LBB243_138:
	vmovdqa	176(%rsp), %xmm0
	movq	192(%rsp), %rax
	movzbl	1402(%rsp), %ebx
	movq	$0, 176(%rsp)
	movq	$8, 184(%rsp)
	movq	$0, 192(%rsp)
	movq	%rax, 112(%rsp)
	vmovdqa	%xmm0, 96(%rsp)
	testb	%bl, %bl
	jne	.LBB243_430
	vmovdqa	96(%rsp), %xmm0
	movq	112(%rsp), %rax
	movq	%rax, 48(%rsp)
	vmovdqa	%xmm0, 32(%rsp)
.Ltmp7259:
	leaq	240(%rsp), %rdi
	leaq	848(%rsp), %rdx
	leaq	32(%rsp), %rcx
	movq	%r13, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>
.Ltmp7260:
	vmovups	248(%rsp), %xmm0
	movq	264(%rsp), %rax
	movq	240(%rsp), %rbx
	movq	%rax, 144(%rsp)
	vmovaps	%xmm0, 128(%rsp)
	cmpq	$-1, %rbx
	je	.LBB243_209
	vmovdqa	272(%rsp), %xmm0
	vmovdqu	288(%rsp), %ymm3
	vmovdqa	128(%rsp), %xmm1
	vmovdqu	304(%rsp), %ymm2
	movq	144(%rsp), %rax
	movq	%rax, 512(%rsp)
	vmovdqu	%ymm3, 592(%rsp)
	vmovdqa	%xmm0, 16(%rsp)
	vmovdqu	%ymm2, 608(%rsp)
	vmovdqa	%xmm1, 496(%rsp)
.Ltmp7267:
	leaq	1408(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7268:
	vmovdqa	496(%rsp), %xmm0
	vmovups	608(%rsp), %ymm1
	movq	88(%rsp), %rcx
	movq	512(%rsp), %rax
	vmovdqu	592(%rsp), %ymm3
	movq	%rax, 896(%rsp)
	movq	896(%rsp), %rax
	vmovdqa	%xmm0, 880(%rsp)
	vmovups	%ymm1, 80(%rcx)
	vmovdqa	16(%rsp), %xmm1
	vmovdqu	%ymm3, 64(%rcx)
	vmovdqa	880(%rsp), %xmm2
	movq	%rax, 40(%rcx)
	vmovdqu	%xmm2, 24(%rcx)
	movq	%rbx, 16(%rcx)
	vmovdqa	%xmm1, 48(%rcx)
	movq	$1, (%rcx)
	jmp	.LBB243_260
.LBB243_143:
	movq	$0, 32(%rsp)
	movq	$8, 40(%rsp)
	movq	$0, 48(%rsp)
.Ltmp7233:
	leaq	240(%rsp), %rdi
	leaq	848(%rsp), %rdx
	leaq	32(%rsp), %rcx
	movq	%r13, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, &mut purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>
.Ltmp7234:
	vmovups	248(%rsp), %xmm0
	movq	264(%rsp), %rax
	movq	240(%rsp), %r13
	movq	%rax, 48(%rsp)
	vmovaps	%xmm0, 32(%rsp)
	cmpq	$-1, %r13
	je	.LBB243_211
	vmovdqu	288(%rsp), %ymm0
	vmovdqa	32(%rsp), %xmm1
	vmovdqu	304(%rsp), %ymm2
	movq	272(%rsp), %r15
	movq	48(%rsp), %rax
	movq	280(%rsp), %r14
	movq	%r15, %rbx
	movq	%rax, 112(%rsp)
	shrq	$8, %rbx
	vmovdqu	%ymm0, 592(%rsp)
	vmovdqu	%ymm2, 608(%rsp)
	vmovdqa	%xmm1, 96(%rsp)
	jmp	.LBB243_212
.LBB243_146:
	movq	8(%rsp), %rax
	movq	%rax, 40(%rsp)
	movq	592(%rsp), %rax
	movq	%r15, 64(%rsp)
	cmpq	$6, %rax
	jb	.LBB243_148
	movq	600(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB243_148:
	movq	152(%rsp), %rax
	movq	8(%rsp), %r15
	jmp	.LBB243_184
.LBB243_149:
	vmovdqu64	256(%rsp), %zmm0
	vmovdqu64	272(%rsp), %zmm1
	movq	88(%rsp), %rdx
	movq	8(%rsp), %r15
	movq	248(%rsp), %rcx
	movq	%r15, 40(%rsp)
	vmovdqu64	%zmm1, 48(%rdx)
	vmovdqu64	%zmm0, 32(%rdx)
	movq	%rax, 16(%rdx)
	movq	592(%rsp), %rax
	movq	%rcx, 24(%rdx)
	movq	$1, (%rdx)
	cmpq	$6, %rax
	jb	.LBB243_151
	movq	600(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB243_151:
	movq	152(%rsp), %rax
	subq	%r15, %rax
	je	.LBB243_164
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %rbx
	xorl	%r12d, %r12d
	imulq	%rax, %rbx
	jmp	.LBB243_156
.LBB243_153:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_154:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB243_155:
	incq	%r12
	cmpq	%rbx, %r12
	je	.LBB243_164
.LBB243_156:
	leaq	(%r12,%r12,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB243_155
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_159
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_154
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_159
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB243_162:
	cmpq	%rax, %rdx
	jge	.LBB243_153
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB243_162
	jmp	.LBB243_153
.LBB243_164:
	movq	216(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_166
	movq	840(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB243_166:
	testq	%rbp, %rbp
	je	.LBB243_179
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %r13
	xorl	%ebx, %ebx
	jmp	.LBB243_171
.LBB243_168:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_169:
	vzeroupper
	callq	*%r13
.LBB243_170:
	incq	%rbx
	cmpq	%rbx, %rbp
	je	.LBB243_179
.LBB243_171:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB243_170
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_174
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_174:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_169
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_174
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r15)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB243_177:
	cmpq	%rax, %rdx
	jge	.LBB243_168
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB243_177
	jmp	.LBB243_168
.LBB243_179:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_181
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB243_181:
	movl	$0, 8(%rsp)
	jmp	.LBB243_265
.LBB243_182:
	movq	%r15, %r12
	movq	152(%rsp), %r15
.LBB243_183:
	movq	152(%rsp), %rax
	movq	%r15, 40(%rsp)
	movq	%r12, 64(%rsp)
.LBB243_184:
	subq	%r15, %rax
	je	.LBB243_197
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %rbx
	xorl	%r14d, %r14d
	imulq	%rax, %rbx
	jmp	.LBB243_189
	.p2align	4
.LBB243_186:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_187:
	callq	*%rbp
.LBB243_188:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB243_197
.LBB243_189:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB243_188
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_192
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_192:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_187
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_192
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB243_195:
	cmpq	%rax, %rdx
	jge	.LBB243_186
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB243_195
	jmp	.LBB243_186
.LBB243_197:
	movq	216(%rsp), %rax
	movq	168(%rsp), %rbx
	movq	16(%rsp), %r14
	testq	%rax, %rax
	je	.LBB243_208
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_200
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB243_200:
	movq	840(%rsp), %rdi
	.p2align	4
.LBB243_201:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_207
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_201
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB243_204:
	cmpq	%rax, %rdx
	jge	.LBB243_206
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_204
.LBB243_206:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_207:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_208:
	vmovdqu	128(%rsp), %xmm0
	movq	144(%rsp), %rax
	xorl	%ebp, %ebp
	movq	%rax, 576(%rsp)
	vmovdqa	%xmm0, 560(%rsp)
	movq	696(%r14), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB243_342
	jmp	.LBB243_326
.LBB243_209:
	vmovdqa	128(%rsp), %xmm0
	movq	144(%rsp), %rax
	movq	%r12, 168(%rsp)
	movq	%rax, 512(%rsp)
	vmovdqa	%xmm0, 496(%rsp)
.Ltmp7272:
	leaq	1408(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7273:
	vmovaps	496(%rsp), %xmm0
	movq	512(%rsp), %rax
	movq	$0, 224(%rsp)
	movq	$0, 152(%rsp)
	movq	%rax, 144(%rsp)
	movl	$8, %eax
	movq	%rax, 8(%rsp)
	jmp	.LBB243_311
.LBB243_211:
	vmovdqa	32(%rsp), %xmm0
	movq	48(%rsp), %rax
	xorl	%ebx, %ebx
	xorl	%r15d, %r15d
	movq	%rax, 112(%rsp)
	vmovdqa	%xmm0, 96(%rsp)
.LBB243_212:
.Ltmp7238:
	leaq	1408(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7239:
	cmpq	$-1, %r13
	jne	.LBB243_259
	jmp	.LBB243_310
.LBB243_214:
	andq	$-4, %r8
	leaq	776(%rcx), %r10
	xorl	%r9d, %r9d
	xorl	%r11d, %r11d
	jmp	.LBB243_216
	.p2align	4
.LBB243_215:
	addq	$4, %r9
	addq	$800, %r10
	cmpq	%r9, %r8
	je	.LBB243_232
.LBB243_216:
	movq	-600(%r10), %rax
	mulq	-608(%r10)
	jo	.LBB243_225
	addq	%rax, %r11
	movq	$-1, %r14
	jb	.LBB243_218
.LBB243_226:
	movq	%r11, %r14
	movq	-400(%r10), %rax
	mulq	-408(%r10)
	jno	.LBB243_219
.LBB243_227:
	movq	$-1, %rax
	addq	%rax, %r14
	movq	$-1, %r11
	jae	.LBB243_228
	.p2align	4
.LBB243_220:
	movq	-200(%r10), %rax
	mulq	-208(%r10)
	jo	.LBB243_229
.LBB243_221:
	addq	%rax, %r11
	movq	$-1, %r14
	jb	.LBB243_222
.LBB243_230:
	movq	%r11, %r14
	movq	(%r10), %rax
	mulq	-8(%r10)
	jno	.LBB243_223
.LBB243_231:
	movq	$-1, %rax
	addq	%rax, %r14
	movq	$-1, %r11
	jb	.LBB243_215
	jmp	.LBB243_224
.LBB243_225:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r14
	jae	.LBB243_226
	.p2align	4
.LBB243_218:
	movq	-400(%r10), %rax
	mulq	-408(%r10)
	jo	.LBB243_227
.LBB243_219:
	addq	%rax, %r14
	movq	$-1, %r11
	jb	.LBB243_220
.LBB243_228:
	movq	%r14, %r11
	movq	-200(%r10), %rax
	mulq	-208(%r10)
	jno	.LBB243_221
.LBB243_229:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r14
	jae	.LBB243_230
	.p2align	4
.LBB243_222:
	movq	(%r10), %rax
	mulq	-8(%r10)
	jo	.LBB243_231
.LBB243_223:
	addq	%rax, %r14
	movq	$-1, %r11
	jb	.LBB243_215
.LBB243_224:
	movq	%r14, %r11
	jmp	.LBB243_215
.LBB243_232:
	testq	%rdi, %rdi
	je	.LBB243_237
.LBB243_233:
	imulq	$200, %r9, %rax
	imulq	$200, %rdi, %rdi
	movq	$-1, %r10
	xorl	%r9d, %r9d
	leaq	176(%rax,%rcx), %r8
	.p2align	4
.LBB243_234:
	movq	(%r8,%r9), %rax
	mulq	-8(%r8,%r9)
	jo	.LBB243_236
.LBB243_235:
	addq	%rax, %r11
	cmovbq	%r10, %r11
	addq	$200, %r9
	cmpq	%r9, %rdi
	jne	.LBB243_234
	jmp	.LBB243_237
.LBB243_236:
	movq	$-1, %rax
	jmp	.LBB243_235
.LBB243_237:
	testq	%r11, %r11
	je	.LBB243_240
	cmpq	$0, 336(%rbx)
	je	.LBB243_240
	lock		addq	%r11, 352(%rbx)
.LBB243_240:
	movq	128(%rsp), %rax
	movq	%rcx, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	%rax, 256(%rsp)
	movq	%rsi, 264(%rsp)
.Ltmp7244:
	leaq	904(%rsp), %rdi
	leaq	240(%rsp), %rsi
	callq	<purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>
.Ltmp7245:
	movzbl	1401(%rsp), %r9d
	movq	920(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_244
	movq	912(%rsp), %rcx
	cmpq	$8, %rax
	jae	.LBB243_245
	xorl	%edx, %edx
	xorl	%ebx, %ebx
	jmp	.LBB243_254
.LBB243_244:
	xorl	%ebx, %ebx
	jmp	.LBB243_256
.LBB243_245:
	cmpq	$32, %rax
	jae	.LBB243_247
	xorl	%edx, %edx
	xorl	%ebx, %ebx
	jmp	.LBB243_251
.LBB243_247:
	vmovdqa64	.LCPI243_0(%rip), %zmm1
	vpbroadcastq	.LCPI243_1(%rip), %zmm2
	vpbroadcastq	.LCPI243_2(%rip), %zmm3
	movq	%rax, %rdx
	andq	$-32, %rdx
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rdx, %rsi
.LBB243_248:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rsi
	vpgatherqq	64(%rcx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%rcx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%rcx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%rcx,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB243_248
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rdx, %rax
	je	.LBB243_256
	testb	$24, %al
	je	.LBB243_254
.LBB243_251:
	movq	%rdx, %rsi
	vpbroadcastq	%rsi, %zmm1
	vporq	.LCPI243_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI243_1(%rip), %zmm2
	vpbroadcastq	.LCPI243_3(%rip), %zmm3
	movq	%rax, %rdx
	andq	$-8, %rdx
	vmovq	%rbx, %xmm0
	subq	%rdx, %rsi
	.p2align	4
.LBB243_252:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rsi
	vpgatherqq	64(%rcx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB243_252
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rdx, %rax
	je	.LBB243_256
.LBB243_254:
	subq	%rdx, %rax
	leaq	(%rdx,%rdx,4), %rdx
	shlq	$5, %rdx
	leaq	64(%rdx,%rcx), %rcx
	.p2align	4
.LBB243_255:
	addq	(%rcx), %rbx
	addq	$160, %rcx
	decq	%rax
	jne	.LBB243_255
.LBB243_256:
	movzbl	928(%rsp), %ebp
	movzbl	1403(%rsp), %edx
.Ltmp7250:
	leaq	240(%rsp), %rdi
	leaq	904(%rsp), %rcx
	leaq	848(%rsp), %r8
	movq	%r13, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>
.Ltmp7251:
	vmovups	248(%rsp), %xmm0
	movq	264(%rsp), %rax
	movq	240(%rsp), %r13
	movzbl	272(%rsp), %r15d
	movq	%rax, 48(%rsp)
	vmovaps	%xmm0, 32(%rsp)
	cmpq	$-1, %r13
	je	.LBB243_308
	movzbl	279(%rsp), %eax
	movzwl	277(%rsp), %ecx
	vmovdqu	288(%rsp), %ymm0
	vmovdqa	32(%rsp), %xmm1
	vmovdqu	304(%rsp), %ymm2
	movl	273(%rsp), %ebx
	movq	280(%rsp), %r14
	shll	$16, %eax
	orl	%eax, %ecx
	movq	48(%rsp), %rax
	shlq	$32, %rcx
	vmovdqu	%ymm0, 592(%rsp)
	orq	%rcx, %rbx
	vmovdqu	%ymm2, 608(%rsp)
	vmovdqa	%xmm1, 96(%rsp)
	movq	%rax, 112(%rsp)
.LBB243_259:
	vmovdqa	96(%rsp), %xmm0
	vmovups	608(%rsp), %ymm1
	movq	88(%rsp), %rdx
	movq	112(%rsp), %rax
	vmovdqu	592(%rsp), %ymm2
	shlq	$8, %rbx
	movq	%rax, 512(%rsp)
	movzbl	%r15b, %eax
	movq	512(%rsp), %rcx
	orq	%rbx, %rax
	vmovdqa	%xmm0, 496(%rsp)
	vmovups	%ymm1, 80(%rdx)
	vmovdqu	%ymm2, 64(%rdx)
	vmovdqa	496(%rsp), %xmm1
	movq	%rcx, 40(%rdx)
	vmovdqu	%xmm1, 24(%rdx)
	movq	%r13, 16(%rdx)
	movq	%rax, 48(%rdx)
	movq	%r14, 56(%rdx)
	movq	$1, (%rdx)
.LBB243_260:
.Ltmp7269:
	leaq	528(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7270:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_264
.LBB243_262:
	lock		decq	(%rax)
	jne	.LBB243_264
	#MEMBARRIER
.Ltmp7334:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7335:
.LBB243_264:
	movb	$1, %al
	movl	%eax, 8(%rsp)
.LBB243_265:
.Ltmp7339:
	leaq	1624(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7340:
	movq	80(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB243_268
	#MEMBARRIER
.Ltmp7344:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	callq	*%rax
.Ltmp7345:
.LBB243_268:
.Ltmp7347:
	leaq	1208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7348:
	movq	1112(%rsp), %rax
	movl	8(%rsp), %ebx
	cmpq	$6, %rax
	jb	.LBB243_279
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1120(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_272
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_272:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_278
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_272
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB243_275:
	cmpq	%rax, %rdx
	jge	.LBB243_277
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_275
.LBB243_277:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_278:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_279:
	movq	1040(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB243_289
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1048(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_282
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_282:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_288
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_282
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB243_285:
	cmpq	%rax, %rdx
	jge	.LBB243_287
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_285
.LBB243_287:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_288:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_289:
	movq	1136(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_292
	lock		decq	(%rax)
	jne	.LBB243_292
	leaq	1136(%rsp), %rdi
	#MEMBARRIER
.Ltmp7350:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7351:
.LBB243_292:
	movq	200(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB243_294
	#MEMBARRIER
.Ltmp7353:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	callq	*%rax
.Ltmp7354:
.LBB243_294:
	testb	%bl, %bl
	je	.LBB243_386
	movq	184(%rsp), %r14
	movq	192(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB243_375
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB243_300
	.p2align	4
.LBB243_297:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_298:
	callq	*%rbp
.LBB243_299:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB243_375
.LBB243_300:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB243_299
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_303
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_303:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_298
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_303
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB243_306:
	cmpq	%rax, %rdx
	jge	.LBB243_297
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB243_306
	jmp	.LBB243_297
.LBB243_308:
	vmovdqa	32(%rsp), %xmm0
	movq	48(%rsp), %rax
	movq	%rax, 256(%rsp)
	vmovdqa	%xmm0, 240(%rsp)
.Ltmp7253:
	movq	16(%rsp), %rdi
	movzbl	%r15b, %esi
	movzbl	%bpl, %edx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7254:
	vmovdqa	240(%rsp), %xmm0
	movq	%rax, %r15
	movq	256(%rsp), %rax
	movq	%rdx, %r14
	movq	%rax, 112(%rsp)
	vmovdqa	%xmm0, 96(%rsp)
.LBB243_310:
	vmovaps	96(%rsp), %xmm0
	movq	112(%rsp), %rax
	movq	%r15, 152(%rsp)
	movq	%r14, 216(%rsp)
	movq	%r12, 168(%rsp)
	movq	%rax, 144(%rsp)
.LBB243_311:
	vmovaps	%xmm0, 128(%rsp)
	movq	144(%rsp), %rax
	movq	528(%rsp), %rcx
	vmovdqa	128(%rsp), %xmm0
	movq	544(%rsp), %rdx
	movq	552(%rsp), %rsi
	movl	$1, %r14d
	movl	$1, %edi
	movq	%rax, 112(%rsp)
	movq	536(%rsp), %rax
	cmpq	$3, %rcx
	movq	%rcx, %r13
	cmovaeq	%rdx, %r13
	cmovaeq	%rcx, %rdi
	cmovaeq	%r14, %rdx
	movq	%rdi, 240(%rsp)
	vmovdqa	%xmm0, 96(%rsp)
	movq	%rax, 248(%rsp)
	movq	%rdx, 256(%rsp)
	movq	%rsi, 264(%rsp)
	movq	%r13, %rsi
	decq	%rsi
	movq	$0, 272(%rsp)
	movq	%rsi, 280(%rsp)
	je	.LBB243_316
	cmpq	$3, %rcx
	movq	16(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %r12
	leaq	248(%rsp), %r15
	leaq	592(%rsp), %rbp
	cmovaeq	%rax, %r15
	leaq	640(%rcx), %rbx
	.p2align	4
.LBB243_314:
	movq	%r14, 272(%rsp)
	movq	16(%r15), %rax
	movq	%rax, 608(%rsp)
	vmovdqu	(%r15), %xmm0
	vmovdqa	%xmm0, 592(%rsp)
.Ltmp7277:
	movq	%rbx, %rdi
	movq	%rbp, %rsi
	callq	*%r12
.Ltmp7278:
	addq	$24, %r15
	incq	%r14
	cmpq	%r14, %r13
	jne	.LBB243_314
.LBB243_316:
.Ltmp7283:
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7284:
	movq	168(%rsp), %rbx
	movq	16(%rsp), %r14
	movq	224(%rsp), %r15
	movq	216(%rsp), %r12
	testb	$1, 152(%rsp)
	je	.LBB243_322
	movq	%r15, %rcx
	subq	%r12, %rcx
	jb	.LBB243_431
.Ltmp7285:
	leaq	240(%rsp), %rdi
	movq	%r14, %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7286:
	cmpq	%r12, %r15
	jne	.LBB243_397
.LBB243_321:
.Ltmp7310:
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7311:
	movq	16(%rsp), %r14
.LBB243_322:
	movq	112(%rsp), %rax
	vmovdqa	96(%rsp), %xmm0
	movq	%rax, 576(%rsp)
	movq	160(%rsp), %rax
	vmovdqa	%xmm0, 560(%rsp)
	testq	%rax, %rax
	je	.LBB243_325
	lock		decq	(%rax)
	jne	.LBB243_325
	#MEMBARRIER
.Ltmp7312:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp7313:
.LBB243_325:
	movb	$1, %bpl
	movq	696(%r14), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB243_342
.LBB243_326:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB243_342
	movb	%cl, 592(%rsp)
	movq	80(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 593(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 608(%rsp)
	lock		incq	(%rcx)
	jle	.LBB243_437
	movq	80(%rsp), %rcx
	movl	%ebp, 8(%rsp)
.Ltmp7314:
	leaq	240(%rsp), %rdi
	leaq	592(%rsp), %rdx
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp7315:
	vmovdqu64	240(%rsp), %zmm0
	vmovdqu64	272(%rsp), %zmm1
	movq	88(%rsp), %rax
	movq	568(%rsp), %rbx
	movq	576(%rsp), %r14
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	testq	%r14, %r14
	je	.LBB243_387
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB243_334
.LBB243_331:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_332:
	vzeroupper
	callq	*%rbp
.LBB243_333:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB243_387
.LBB243_334:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB243_333
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_337
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_337:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_332
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_337
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB243_340:
	cmpq	%rax, %rdx
	jge	.LBB243_331
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB243_340
	jmp	.LBB243_331
.LBB243_342:
	vmovups	1080(%rsp), %zmm1
	vmovdqu64	1040(%rsp), %zmm0
	movq	576(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	%rcx, 48(%rsp)
	vmovups	%zmm1, 280(%rsp)
	vmovdqa	560(%rsp), %xmm1
	vmovdqu64	%zmm0, 240(%rsp)
	cmpq	$-1, 240(%rsp)
	vmovdqa	%xmm1, 32(%rsp)
	movq	%rax, 56(%rsp)
	je	.LBB243_345
	leaq	592(%rsp), %rdi
	leaq	32(%rsp), %rsi
	leaq	1040(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	312(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB243_344
.LBB243_346:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	320(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_348
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_348:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_354
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_348
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB243_351:
	cmpq	%rax, %rdx
	jge	.LBB243_353
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_351
.LBB243_353:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_354:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	336(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB243_355
	jmp	.LBB243_357
.LBB243_345:
	vmovdqu	32(%rsp), %xmm0
	movq	48(%rsp), %rax
	movq	56(%rsp), %rcx
	movq	%rax, 616(%rsp)
	movq	%rcx, 624(%rsp)
	vmovdqu	%xmm0, 600(%rsp)
	movq	$-1, 592(%rsp)
	movq	312(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB243_346
.LBB243_344:
	movq	336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_357
.LBB243_355:
	lock		decq	(%rax)
	jne	.LBB243_357
	leaq	336(%rsp), %rdi
	#MEMBARRIER
.Ltmp7317:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp7318:
.LBB243_357:
	vmovdqu64	592(%rsp), %zmm0
	vmovdqu64	624(%rsp), %zmm1
	movq	88(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp7320:
	leaq	1624(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7321:
.Ltmp7323:
	leaq	1208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7324:
	movq	200(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB243_361
	#MEMBARRIER
.Ltmp7326:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	callq	*%rax
.Ltmp7327:
.LBB243_361:
	testb	%bpl, %bpl
	je	.LBB243_386
	movq	184(%rsp), %r14
	movq	192(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB243_375
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB243_367
	.p2align	4
.LBB243_364:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_365:
	callq	*%rbp
.LBB243_366:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB243_375
.LBB243_367:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB243_366
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_370
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_370:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_365
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_370
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB243_373:
	cmpq	%rax, %rdx
	jge	.LBB243_364
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB243_373
	jmp	.LBB243_364
.LBB243_375:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_386
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_378
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_378:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_384
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_378
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB243_381:
	cmpq	%rax, %rdx
	jge	.LBB243_383
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_381
.LBB243_383:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_384:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
.LBB243_385:
	vzeroupper
	callq	*%rax
.LBB243_386:
	movq	88(%rsp), %rax
	addq	$1960, %rsp
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
	vzeroupper
	retq
.LBB243_387:
	.cfi_def_cfa_offset 2016
	movq	560(%rsp), %rax
	testq	%rax, %rax
	je	.LBB243_265
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB243_390
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB243_390:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB243_396
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB243_390
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
.LBB243_393:
	cmpq	%rax, %rdx
	jge	.LBB243_395
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB243_393
.LBB243_395:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB243_396:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB243_265
.LBB243_397:
	movq	8(%rsp), %rcx
	shlq	$3, %r12
	shlq	$3, %r15
	leaq	40(%rsp), %rbp
	movq	%rbx, 168(%rsp)
	leaq	(%r12,%r12,4), %rax
	leaq	(%r15,%r15,4), %r14
	subq	%rax, %r14
	leaq	8(%rcx,%rax), %r13
	jmp	.LBB243_399
.LBB243_398:
	movq	104(%rsp), %rax
	leaq	(%r14,%r14,4), %rcx
	incq	%r14
	addq	$40, %r13
	addq	$-40, %r15
	movq	%r12, (%rax,%rcx,8)
	movq	%rbx, 8(%rax,%rcx,8)
	movq	168(%rsp), %rbx
	vmovdqa	592(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rax,%rcx,8)
	movq	608(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r14, 112(%rsp)
	movq	%r15, %r14
	je	.LBB243_321
.LBB243_399:
.Ltmp7287:
	movq	16(%rsp), %rdx
	leaq	592(%rsp), %rdi
	leaq	240(%rsp), %rsi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7288:
	cmpb	$-1, 592(%rsp)
	jne	.LBB243_321
	movq	232(%rsp), %rdx
	movq	$1, 592(%rsp)
	cmpq	$5, %rdx
	jae	.LBB243_423
.LBB243_402:
	vmovdqu	600(%rsp), %xmm0
	movq	624(%rsp), %rax
	movq	592(%rsp), %rdx
	movq	616(%rsp), %rcx
	movq	%r13, %r12
	movq	%rax, 64(%rsp)
	movq	%rdx, 32(%rsp)
	movq	%rcx, 56(%rsp)
	vmovdqu	%xmm0, 40(%rsp)
	movq	-8(%r13), %rbx
	decq	%rbx
	cmpq	$5, %rbx
	jb	.LBB243_404
	movq	8(%r13), %rbx
	movq	(%r13), %r12
	decq	%rbx
.LBB243_404:
	movq	32(%rsp), %rax
	movq	48(%rsp), %rsi
	movl	$4, %edx
	movq	%r14, 8(%rsp)
	leaq	-1(%rax), %rcx
	decq	%rsi
	cmpq	$5, %rcx
	cmovbq	%rcx, %rsi
	cmovbq	%rdx, %rcx
	subq	%rsi, %rcx
	cmpq	%rbx, %rcx
	jb	.LBB243_424
.LBB243_405:
	xorl	%r14d, %r14d
	movq	%rbp, %r15
	movq	%rbp, %rcx
	cmpq	$6, %rax
	setae	%al
	jb	.LBB243_407
	movq	40(%rsp), %rcx
.LBB243_407:
	movb	%al, %r14b
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	(,%rbx,8), %rdx
	movq	%r12, %rsi
	shll	$4, %r14d
	movq	32(%rsp,%r14), %rbp
	leaq	-8(%rcx,%rbp,8), %rdi
	callq	*%rax
	addq	%rbx, %rbp
	movq	232(%rsp), %rax
	movq	%rbp, 32(%rsp,%r14)
	movq	32(%rsp), %rcx
	movq	48(%rsp), %rdx
	leaq	-1(%rcx), %rsi
	leaq	-1(%rdx), %rdi
	cmpq	$5, %rsi
	cmovbq	%rsi, %rdi
	movq	%rax, %rsi
	subq	%rdi, %rsi
	jbe	.LBB243_410
	movl	$2, 592(%rsp)
	movq	%rsi, 600(%rsp)
.Ltmp7295:
	leaq	32(%rsp), %rdi
	leaq	592(%rsp), %rsi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp7296:
	movq	%r15, %rbp
	jmp	.LBB243_412
.LBB243_410:
	cmpq	$6, %rcx
	movq	%r15, %rbp
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB243_412
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, 32(%rsp,%rdx)
.LBB243_412:
	movq	32(%rsp), %rcx
	movq	%rbp, %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB243_414
	movq	48(%rsp), %rcx
	movq	40(%rsp), %rdx
	decq	%rcx
.LBB243_414:
	movq	80(%rsp), %r8
	addq	$16, %r8
.Ltmp7297:
	movq	16(%rsp), %r9
	leaq	592(%rsp), %rdi
	leaq	1624(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7298:
	vmovq	600(%rsp), %xmm0
	movq	592(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB243_426
	movq	32(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB243_418
	movq	48(%rsp), %rsi
.LBB243_418:
	movq	520(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB243_436
	movq	%rbp, %rcx
	cmpq	$6, %rax
	jb	.LBB243_421
	movq	40(%rsp), %rcx
.LBB243_421:
	vmovq	%xmm0, (%rcx,%rdi,8)
	leaq	48(%rsp), %rax
	movq	8(%rsp), %r15
	vmovdqu	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	32(%rsp), %r12
	movq	40(%rsp), %rbx
	movq	112(%rsp), %r14
	movq	%rax, 608(%rsp)
	vmovdqa	%xmm0, 592(%rsp)
	cmpq	96(%rsp), %r14
	jne	.LBB243_398
.Ltmp7305:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	callq	*%rax
.Ltmp7306:
	jmp	.LBB243_398
.LBB243_423:
.Ltmp7290:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	592(%rsp), %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp7291:
	jmp	.LBB243_402
.LBB243_424:
.Ltmp7293:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	movl	$1, %ecx
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp7294:
	movq	32(%rsp), %rax
	jmp	.LBB243_405
.LBB243_426:
	vmovups	608(%rsp), %zmm1
	vmovups	624(%rsp), %zmm2
	movq	88(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	32(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB243_428
	movq	40(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB243_428:
.Ltmp7300:
	leaq	240(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7301:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB243_262
	jmp	.LBB243_264
.LBB243_430:
.Ltmp7256:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.152(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.154(%rip), %rdx
	movl	$83, %esi
	vzeroupper
	callq	*%rax
.Ltmp7257:
	jmp	.LBB243_437
.LBB243_431:
.Ltmp7329:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.404(%rip), %rcx
	movq	%r12, %rdi
	movq	%r15, %rsi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp7330:
	jmp	.LBB243_437
.LBB243_432:
	movq	8(%rsp), %rax
	movq	%rax, 40(%rsp)
	movq	%r15, 64(%rsp)
.Ltmp7215:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.402(%rip), %rdx
	callq	*%rax
.Ltmp7216:
	jmp	.LBB243_437
.LBB243_433:
.Ltmp7356:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	256(%rsp), %rbx
	callq	*%rax
.Ltmp7357:
	jmp	.LBB243_437
.LBB243_434:
.Ltmp7395:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	608(%rsp), %rbx
	callq	*%rax
.Ltmp7396:
	jmp	.LBB243_437
.LBB243_435:
.Ltmp7206:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp7207:
	jmp	.LBB243_437
.LBB243_436:
.Ltmp7302:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.403(%rip), %rdx
	callq	*%rax
.Ltmp7303:
.LBB243_437:
	ud2
.LBB243_438:
.Ltmp7292:
	movq	%rax, 16(%rsp)
	movq	592(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB243_477
	movq	600(%rsp), %rdi
	jmp	.LBB243_475
.LBB243_440:
.Ltmp7255:
	leaq	240(%rsp), %rdi
	movq	%rax, 16(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB243_511
.LBB243_441:
.Ltmp7252:
	jmp	.LBB243_449
.LBB243_442:
.Ltmp7246:
	jmp	.LBB243_446
.LBB243_443:
.Ltmp7240:
	jmp	.LBB243_449
.LBB243_444:
.Ltmp7235:
	movq	%rax, 16(%rsp)
.Ltmp7236:
	leaq	1408(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7237:
	jmp	.LBB243_511
.LBB243_445:
.Ltmp7243:
.LBB243_446:
	movq	%rax, 16(%rsp)
.Ltmp7247:
	leaq	848(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7248:
	jmp	.LBB243_511
.LBB243_447:
.Ltmp7249:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_448:
.Ltmp7274:
.LBB243_449:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_511
.LBB243_450:
.Ltmp7261:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_508
.LBB243_451:
.Ltmp7316:
	leaq	560(%rsp), %rdi
	movq	%rax, 16(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB243_516
.LBB243_452:
.Ltmp7319:
	movl	%ebp, 8(%rsp)
	movq	%rax, 16(%rsp)
	xorl	%ebx, %ebx
	jmp	.LBB243_517
.LBB243_453:
.Ltmp7307:
	movq	%rax, 16(%rsp)
	cmpq	$6, %r12
	jb	.LBB243_477
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	jmp	.LBB243_476
.LBB243_455:
.Ltmp7352:
	jmp	.LBB243_470
.LBB243_456:
.Ltmp7271:
	jmp	.LBB243_479
.LBB243_457:
.Ltmp7328:
	movq	%rax, 16(%rsp)
	testb	%bpl, %bpl
	jne	.LBB243_528
	jmp	.LBB243_529
.LBB243_458:
.Ltmp7381:
	movq	%rax, 16(%rsp)
	lock		decq	(%rbx)
	jne	.LBB243_498
	#MEMBARRIER
.Ltmp7382:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	240(%rsp), %rdi
	callq	*%rax
.Ltmp7383:
	jmp	.LBB243_498
.LBB243_460:
.Ltmp7384:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_461:
.Ltmp7355:
	movq	%rax, 16(%rsp)
	testb	%bl, %bl
	jne	.LBB243_528
	jmp	.LBB243_529
.LBB243_462:
.Ltmp7346:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_490
.LBB243_463:
.Ltmp7304:
	jmp	.LBB243_473
.LBB243_464:
.Ltmp7230:
	movq	%rax, 16(%rsp)
.Ltmp7231:
	leaq	848(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7232:
	jmp	.LBB243_512
.LBB243_465:
.Ltmp7289:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_477
.LBB243_466:
.Ltmp7371:
	movq	%rax, 16(%rsp)
	lock		decq	(%r14)
	jne	.LBB243_498
	#MEMBARRIER
.Ltmp7372:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	240(%rsp), %rdi
	callq	*%rax
.Ltmp7373:
	jmp	.LBB243_498
.LBB243_468:
.Ltmp7374:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_469:
.Ltmp7325:
	movl	%ebp, 8(%rsp)
.LBB243_470:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_525
.LBB243_471:
.Ltmp7322:
	movl	%ebp, 8(%rsp)
	movq	%rax, 16(%rsp)
	jmp	.LBB243_521
.LBB243_472:
.Ltmp7299:
.LBB243_473:
	movq	%rax, 16(%rsp)
	movq	32(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB243_477
	movq	40(%rsp), %rdi
.LBB243_475:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB243_476:
	callq	__rustc::__rust_dealloc
.LBB243_477:
.Ltmp7308:
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7309:
	jmp	.LBB243_506
.LBB243_478:
.Ltmp7227:
.LBB243_479:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_512
.LBB243_480:
.Ltmp7397:
	movq	%rax, 16(%rsp)
.Ltmp7398:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp7399:
.Ltmp7401:
	leaq	936(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7402:
	jmp	.LBB243_529
.LBB243_482:
.Ltmp7400:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_483:
.Ltmp7390:
	movq	%rax, 16(%rsp)
.Ltmp7391:
	leaq	1624(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp7392:
	jmp	.LBB243_498
.LBB243_484:
.Ltmp7336:
	movq	%rax, 16(%rsp)
	movb	$1, %al
	movb	$1, %bl
	movl	%eax, 8(%rsp)
	jmp	.LBB243_517
.LBB243_485:
.Ltmp7358:
	movq	%rax, 16(%rsp)
.Ltmp7359:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp7360:
	jmp	.LBB243_489
.LBB243_486:
.Ltmp7361:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_487:
.Ltmp7199:
	movq	%rax, 16(%rsp)
.Ltmp7200:
	leaq	1152(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp7201:
	jmp	.LBB243_489
.LBB243_488:
.Ltmp7196:
	movq	%rax, 16(%rsp)
.LBB243_489:
	movb	$1, %al
	movl	%eax, 8(%rsp)
.LBB243_490:
	movb	$1, %bl
	jmp	.LBB243_522
.LBB243_491:
.Ltmp7220:
	movq	8(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 40(%rsp)
	movq	%r15, 64(%rsp)
	cmpq	$5, %r14
	ja	.LBB243_503
	jmp	.LBB243_504
.LBB243_492:
.Ltmp7279:
	movq	%rax, 16(%rsp)
.Ltmp7280:
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7281:
	jmp	.LBB243_506
.LBB243_493:
.Ltmp7282:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_494:
.Ltmp7217:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_501
.LBB243_495:
.Ltmp7341:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_519
.LBB243_496:
.Ltmp7349:
	movq	%rax, 16(%rsp)
	jmp	.LBB243_524
.LBB243_497:
.Ltmp7387:
	movq	%rax, 16(%rsp)
.LBB243_498:
.Ltmp7393:
	leaq	936(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7394:
	jmp	.LBB243_529
.LBB243_499:
.Ltmp7403:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_500:
.Ltmp7214:
	movq	8(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rcx, 40(%rsp)
	movq	%r15, 64(%rsp)
.LBB243_501:
	movq	592(%rsp), %r14
	cmpq	$6, %r14
	jb	.LBB243_504
	movq	600(%rsp), %r13
.LBB243_503:
	leaq	-8(,%r14,8), %rsi
	movl	$4, %edx
	movq	%r13, %rdi
	callq	__rustc::__rust_dealloc
.LBB243_504:
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	movl	$0, 8(%rsp)
	jmp	.LBB243_517
.LBB243_505:
.Ltmp7331:
	movq	%rax, 16(%rsp)
.LBB243_506:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB243_512
.LBB243_507:
.Ltmp7258:
	leaq	96(%rsp), %rdi
	movq	%rax, 16(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB243_508:
.Ltmp7262:
	leaq	1408(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7263:
	testb	%bl, %bl
	je	.LBB243_511
.Ltmp7264:
	leaq	848(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp7265:
.LBB243_511:
.Ltmp7275:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7276:
.LBB243_512:
	movq	160(%rsp), %rax
	movb	$1, %cl
	movl	%ecx, 8(%rsp)
	testq	%rax, %rax
	je	.LBB243_516
	lock		decq	(%rax)
	movb	$1, %al
	movl	%eax, 8(%rsp)
	jne	.LBB243_516
	movb	$1, %al
	#MEMBARRIER
	movl	%eax, 8(%rsp)
.Ltmp7332:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp7333:
	movb	$1, %bl
	jmp	.LBB243_517
.LBB243_516:
	movb	$1, %bl
.LBB243_517:
.Ltmp7337:
	leaq	1624(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7338:
	testb	%bl, %bl
	je	.LBB243_521
.LBB243_519:
	movq	80(%rsp), %rax
	movb	$1, %bl
	lock		decq	(%rax)
	jne	.LBB243_522
	#MEMBARRIER
.Ltmp7342:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	callq	*%rax
.Ltmp7343:
	jmp	.LBB243_522
.LBB243_521:
	xorl	%ebx, %ebx
.LBB243_522:
.Ltmp7362:
	leaq	1208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7363:
	testb	%bl, %bl
	je	.LBB243_525
.LBB243_524:
.Ltmp7364:
	leaq	1040(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7365:
.LBB243_525:
	movq	200(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB243_527
	leaq	200(%rsp), %rdi
	#MEMBARRIER
.Ltmp7366:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7367:
.LBB243_527:
	cmpb	$0, 8(%rsp)
	je	.LBB243_529
.LBB243_528:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB243_529:
	movq	16(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB243_530:
.Ltmp7266:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB243_531:
.Ltmp7368:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end243:
purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin244:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception165
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
	subq	$1880, %rsp
	.cfi_def_cfa_offset 1936
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %r12
	leaq	1240(%rsp), %rdi
	movq	%r8, %r13
	movq	%rcx, %r14
	movq	%rdx, %rbx
	movq	%rsi, %r15
	callq	*%rax
.Ltmp7404:
	leaq	1344(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r13, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp7405:
	cmpl	$1, 1344(%rsp)
	jne	.LBB244_25
	vmovdqu64	1392(%rsp), %zmm1
	vmovdqu64	1360(%rsp), %zmm0
	movq	1312(%rsp), %rax
	vmovdqu64	%zmm1, 48(%r12)
	vmovdqu64	%zmm0, 16(%r12)
	movq	$1, (%r12)
	cmpq	$6, %rax
	jb	.LBB244_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1320(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_5
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB244_8:
	cmpq	%rax, %rsi
	jge	.LBB244_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB244_8
.LBB244_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB244_12:
	movq	1240(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB244_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1248(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_15
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB244_18:
	cmpq	%rax, %rsi
	jge	.LBB244_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB244_18
.LBB244_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB244_22:
	movq	1336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_388
	lock		decq	(%rax)
	jne	.LBB244_388
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1336(%rsp), %rdi
	#MEMBARRIER
	jmp	.LBB244_387
.LBB244_25:
	vmovdqu64	1384(%rsp), %zmm1
	vmovdqu64	1352(%rsp), %zmm0
	vmovdqu64	%zmm1, 160(%rsp)
	vmovdqu64	%zmm0, 128(%rsp)
.Ltmp7406:
	leaq	512(%rsp), %rdi
	leaq	1240(%rsp), %rsi
	leaq	128(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp7407:
	cmpq	$-1, 512(%rsp)
	je	.LBB244_33
	vmovdqu	512(%rsp), %ymm0
	vmovdqu64	1280(%rsp), %zmm1
	vmovdqu64	1240(%rsp), %zmm2
	vmovdqu	%ymm0, 96(%rsp)
	vmovdqu64	%zmm1, 1176(%rsp)
	vmovdqu64	%zmm2, 1136(%rsp)
	movq	120(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB244_396
	movq	%rax, 24(%rsp)
	movb	$1, %al
	movl	%eax, 8(%rsp)
.Ltmp7412:
	movq	%r13, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp7413:
	cmpb	$2, 472(%r13)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB244_34
	movq	616(%r13), %rax
	testq	%rax, %rax
	je	.LBB244_35
	cmpq	$-2, 24(%rax)
	jb	.LBB244_34
	cmpq	$-2, 32(%rax)
	jae	.LBB244_38
.LBB244_34:
	xorl	%ebp, %ebp
	jmp	.LBB244_39
.LBB244_33:
	leaq	8(%r12), %rdi
	leaq	1240(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	movq	$0, (%r12)
	jmp	.LBB244_388
.LBB244_35:
	movb	$1, %bpl
	jmp	.LBB244_39
.LBB244_38:
	cmpq	$-2, 48(%rax)
	setae	%bpl
.LBB244_39:
	movq	112(%rsp), %rcx
.Ltmp7414:
	leaq	1464(%rsp), %r14
	movzbl	%bpl, %edx
	movq	%r13, %rsi
	movq	%rcx, 32(%rsp)
	movq	%r14, %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7415:
	movb	$1, %al
	movl	%eax, 8(%rsp)
.Ltmp7416:
	movb	$1, %al
	movq	%r13, %rdi
	movq	%r15, 448(%rsp)
	movq	%r15, %rsi
	movq	%rbx, %rdx
	movl	%eax, 92(%rsp)
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7417:
	movq	24(%rsp), %rcx
	movb	$1, %dl
	movl	%edx, 92(%rsp)
	addq	$16, %rcx
.Ltmp7418:
	leaq	1664(%rsp), %r15
	movb	$1, %dl
	movq	%rax, %rsi
	movq	%r13, %r8
	movl	%edx, 8(%rsp)
	movq	%r15, %rdi
	movq	%rbx, %rdx
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7419:
	movq	%r12, 40(%rsp)
	testb	%bpl, %bpl
	je	.LBB244_47
	movq	1648(%rsp), %rbx
	movq	32(%rsp), %rax
	movq	104(%rsp), %rbp
	cmpq	%rax, %rbx
	cmovaeq	%rax, %rbx
.Ltmp7430:
	movq	%r13, %rdi
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp7431:
	movq	616(%r13), %rcx
	movq	%rax, 80(%rsp)
	testq	%rcx, %rcx
	je	.LBB244_73
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB244_74
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB244_74
.LBB244_47:
	movq	32(%rsp), %rcx
	movq	104(%rsp), %rbp
	movq	96(%rsp), %rdx
	movabsq	$9223372036854775807, %rbx
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
	movq	%r13, 16(%rsp)
	leaq	(%rcx,%rcx,4), %rax
	movq	%rbp, 384(%rsp)
	movq	%rdx, 832(%rsp)
	movq	%rdx, 400(%rsp)
	movq	%rbp, 480(%rsp)
	leaq	(%rbp,%rax,8), %r12
	movq	%r12, 408(%rsp)
	testq	%rcx, %rcx
	je	.LBB244_91
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	480(%rsp), %rbp
	movl	$8, %eax
	movq	$0, 8(%rsp)
	movq	%rax, 32(%rsp)
	jmp	.LBB244_52
.LBB244_49:
	movq	56(%rsp), %rax
	movq	%rax, 32(%rsp)
.LBB244_50:
	movq	8(%rsp), %rsi
	movq	32(%rsp), %rdx
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 8(%rsp)
	movq	%r15, (%rdx,%rax,8)
	movq	%r14, 8(%rdx,%rax,8)
	vmovdqa	128(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	144(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 64(%rsp)
.LBB244_51:
	cmpq	%r12, %rbp
	je	.LBB244_90
.LBB244_52:
	vmovups	8(%rbp), %ymm0
	movq	(%rbp), %r15
	addq	$40, %rbp
	vmovups	%ymm0, 752(%rsp)
	testq	%r15, %r15
	je	.LBB244_91
	vmovdqu	752(%rsp), %ymm0
	leaq	936(%rsp), %rax
	movq	%r15, 928(%rsp)
	vmovdqu	%ymm0, (%rax)
.Ltmp7420:
	movq	16(%rsp), %rdx
	leaq	128(%rsp), %rdi
	leaq	1464(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7421:
	cmpb	$-1, 128(%rsp)
	jne	.LBB244_87
	movq	944(%rsp), %rcx
	movq	936(%rsp), %r14
	movq	24(%rsp), %r8
	leaq	-1(%r15), %rax
	leaq	936(%rsp), %rdx
	decq	%rcx
	cmpq	$5, %rax
	cmovaeq	%r14, %rdx
	cmovbq	%rax, %rcx
	addq	$16, %r8
.Ltmp7422:
	movq	16(%rsp), %r9
	leaq	128(%rsp), %rdi
	leaq	1664(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7423:
	movq	128(%rsp), %rax
	movl	136(%rsp), %edx
	movl	140(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB244_89
	cmpl	$2, %edx
	je	.LBB244_61
.Ltmp7424:
	movq	16(%rsp), %rsi
	leaq	128(%rsp), %rdi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7425:
	movq	128(%rsp), %rax
	movzbl	136(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB244_149
	testb	$1, %dl
	jne	.LBB244_71
.LBB244_61:
	cmpq	$6, %r15
	jb	.LBB244_51
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	leaq	-8(,%r15,8), %rcx
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_64
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_64:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_70
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_64
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB244_67:
	cmpq	%rax, %rdx
	jge	.LBB244_69
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB244_67
.LBB244_69:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_70:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB244_51
.LBB244_71:
	leaq	760(%rsp), %rcx
	vmovdqu	(%rcx), %xmm0
	movq	16(%rcx), %rax
	movq	8(%rsp), %rcx
	movq	%rax, 144(%rsp)
	vmovdqa	%xmm0, 128(%rsp)
	cmpq	48(%rsp), %rcx
	jne	.LBB244_50
.Ltmp7427:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	callq	*%rax
.Ltmp7428:
	jmp	.LBB244_49
.LBB244_73:
	xorl	%eax, %eax
.LBB244_74:
	movzbl	1234(%r13), %ecx
	leaq	80(%rsp), %rdx
	movq	%r13, 928(%rsp)
	movq	%rbp, 752(%rsp)
	movq	%rbx, 760(%rsp)
	movq	%rdx, 936(%rsp)
	leaq	24(%rsp), %rdx
	movq	%r14, 944(%rsp)
	movq	%r15, 952(%rsp)
	movq	%rdx, 768(%rsp)
	testb	%al, %al
	je	.LBB244_76
.Ltmp7434:
	movzbl	%cl, %esi
	leaq	128(%rsp), %rdi
	leaq	928(%rsp), %r8
	leaq	752(%rsp), %r9
	movq	%r14, (%rsp)
	movq	%rbp, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7435:
	jmp	.LBB244_77
.LBB244_76:
.Ltmp7432:
	movzbl	%cl, %esi
	leaq	128(%rsp), %rdi
	leaq	928(%rsp), %r8
	leaq	752(%rsp), %r9
	movq	%r14, (%rsp)
	movq	%rbp, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
.Ltmp7433:
.LBB244_77:
	movq	128(%rsp), %rdx
	cmpq	$-1, %rdx
	je	.LBB244_82
	vmovups	320(%rsp), %zmm0
	vmovups	304(%rsp), %zmm1
	movq	152(%rsp), %rax
	vmovdqu	136(%rsp), %xmm2
	movq	168(%rsp), %rsi
	movq	160(%rsp), %rcx
	movq	%rdx, 456(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rdx
	cmpq	$-2, %rdx
	movl	$1, %edx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rdx
	vmovups	%zmm0, 656(%rsp)
	vmovups	%zmm1, 640(%rsp)
	vmovdqu64	176(%rsp), %zmm0
	vmovdqu64	240(%rsp), %zmm1
	decq	%rax
	vmovdqu	%xmm2, 464(%rsp)
	vmovdqu64	656(%rsp), %zmm4
	vmovdqu64	640(%rsp), %zmm3
	vmovdqu64	%zmm0, 512(%rsp)
	vmovdqu64	%zmm1, 576(%rsp)
	vmovdqu64	%zmm1, 216(%rsp)
	vmovdqu64	%zmm0, 152(%rsp)
	vmovdqu64	%zmm4, 296(%rsp)
	vmovdqu64	%zmm3, 280(%rsp)
	movq	%rdi, 128(%rsp)
	movq	%rcx, 136(%rsp)
	movq	%rdx, 144(%rsp)
	movq	$0, 360(%rsp)
	movq	%rax, 368(%rsp)
.Ltmp7437:
	leaq	512(%rsp), %rdi
	leaq	128(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7438:
	vmovups	688(%rsp), %zmm2
	vmovups	672(%rsp), %zmm1
	vmovups	512(%rsp), %ymm0
	cmpb	$2, 1658(%rsp)
	movq	616(%r13), %rbx
	movq	%rbp, 8(%rsp)
	setne	%al
	testq	%rbx, %rbx
	sete	%cl
	orb	%al, %cl
	vmovups	%zmm2, 1072(%rsp)
	vmovups	%zmm1, 1056(%rsp)
	vmovdqu64	608(%rsp), %zmm2
	vmovdqu64	544(%rsp), %zmm1
	vmovups	%ymm0, 384(%rsp)
	vmovdqu64	%zmm2, 992(%rsp)
	vmovdqu64	%zmm1, 928(%rsp)
	cmpb	$1, %cl
	jne	.LBB244_83
	vmovdqu	456(%rsp), %xmm0
	movq	472(%rsp), %rax
	movq	%rax, 64(%rsp)
	vmovdqa	%xmm0, 48(%rsp)
.Ltmp7455:
	leaq	928(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp7456:
	movq	%r13, 16(%rsp)
	movq	$0, 480(%rsp)
	jmp	.LBB244_201
.LBB244_82:
	vmovdqu64	176(%rsp), %zmm0
	vmovdqu	144(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%r12)
	vmovdqu	%ymm1, 16(%r12)
	vmovdqu64	%zmm0, 512(%rsp)
	movq	$1, (%r12)
	movq	80(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB244_196
	jmp	.LBB244_198
.LBB244_83:
	movq	928(%rsp), %rax
	vmovdqu64	1072(%rsp), %zmm0
	vmovdqu64	952(%rsp), %zmm1
	vmovdqu64	1016(%rsp), %zmm2
	movq	944(%rsp), %rdx
	movq	936(%rsp), %rcx
	movl	$1, %edi
	movl	$1, %esi
	cmpq	$3, %rax
	cmovaeq	%rax, %rdi
	cmovaeq	%rdx, %rax
	cmovaeq	%rsi, %rdx
	decq	%rax
	vmovdqu64	%zmm0, 272(%rsp)
	vmovdqu64	%zmm2, 216(%rsp)
	vmovdqu64	%zmm1, 152(%rsp)
	movq	%rdi, 128(%rsp)
	movq	%rcx, 136(%rsp)
	movq	%rdx, 144(%rsp)
	movq	$0, 336(%rsp)
	movq	%rax, 344(%rsp)
.Ltmp7440:
	leaq	752(%rsp), %rdi
	leaq	128(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp7441:
	movq	768(%rsp), %r8
	movq	760(%rsp), %rcx
	imulq	$200, %r8, %rsi
	addq	%rcx, %rsi
	testq	%r8, %r8
	je	.LBB244_142
	movl	%r8d, %edi
	andl	$3, %edi
	cmpq	$4, %r8
	jae	.LBB244_116
	xorl	%r9d, %r9d
	xorl	%r11d, %r11d
	jmp	.LBB244_135
.LBB244_87:
	movq	%rbp, 392(%rsp)
	cmpq	$6, %r15
	jb	.LBB244_92
	movq	936(%rsp), %rdi
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB244_92
.LBB244_89:
	vmovups	160(%rsp), %zmm1
	vmovups	144(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 528(%rsp)
	vmovups	%zmm0, 512(%rsp)
	jmp	.LBB244_150
.LBB244_90:
	movq	%r12, %rbp
.LBB244_91:
	movq	%rbp, 392(%rsp)
.LBB244_92:
	subq	%rbp, %r12
	je	.LBB244_105
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %r14
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	xorl	%r15d, %r15d
	imulq	%r12, %r14
	movq	free@GOTPCREL(%rip), %r12
	jmp	.LBB244_97
	.p2align	4
.LBB244_94:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_95:
	vzeroupper
	callq	*%r12
.LBB244_96:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB244_105
.LBB244_97:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB244_96
	leaq	(%rbp,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rbx, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%rbx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_100
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_100:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_95
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_100
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB244_103:
	cmpq	%rax, %rdx
	jge	.LBB244_94
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB244_103
	jmp	.LBB244_94
.LBB244_105:
	movq	832(%rsp), %rax
	movq	40(%rsp), %r12
	movq	448(%rsp), %r14
	movq	16(%rsp), %r15
	testq	%rax, %rax
	je	.LBB244_115
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_108
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_108:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_114
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_108
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB244_111:
	cmpq	%rax, %rdx
	jge	.LBB244_113
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB244_111
.LBB244_113:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_114:
	movq	free@GOTPCREL(%rip), %rax
	movq	480(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB244_115:
	vmovdqu	48(%rsp), %xmm0
	movq	64(%rsp), %rax
	movl	$0, 8(%rsp)
	movq	%rax, 432(%rsp)
	vmovdqa	%xmm0, 416(%rsp)
	movq	696(%r15), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB244_240
	jmp	.LBB244_224
.LBB244_116:
	andq	$-4, %r8
	leaq	776(%rcx), %r10
	xorl	%r9d, %r9d
	xorl	%r11d, %r11d
	jmp	.LBB244_118
	.p2align	4
.LBB244_117:
	addq	$4, %r9
	addq	$800, %r10
	cmpq	%r9, %r8
	je	.LBB244_134
.LBB244_118:
	movq	-600(%r10), %rax
	mulq	-608(%r10)
	jo	.LBB244_127
	addq	%rax, %r11
	movq	$-1, %r14
	jb	.LBB244_120
.LBB244_128:
	movq	%r11, %r14
	movq	-400(%r10), %rax
	mulq	-408(%r10)
	jno	.LBB244_121
.LBB244_129:
	movq	$-1, %rax
	addq	%rax, %r14
	movq	$-1, %r11
	jae	.LBB244_130
	.p2align	4
.LBB244_122:
	movq	-200(%r10), %rax
	mulq	-208(%r10)
	jo	.LBB244_131
.LBB244_123:
	addq	%rax, %r11
	movq	$-1, %r14
	jb	.LBB244_124
.LBB244_132:
	movq	%r11, %r14
	movq	(%r10), %rax
	mulq	-8(%r10)
	jno	.LBB244_125
.LBB244_133:
	movq	$-1, %rax
	addq	%rax, %r14
	movq	$-1, %r11
	jb	.LBB244_117
	jmp	.LBB244_126
.LBB244_127:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r14
	jae	.LBB244_128
	.p2align	4
.LBB244_120:
	movq	-400(%r10), %rax
	mulq	-408(%r10)
	jo	.LBB244_129
.LBB244_121:
	addq	%rax, %r14
	movq	$-1, %r11
	jb	.LBB244_122
.LBB244_130:
	movq	%r14, %r11
	movq	-200(%r10), %rax
	mulq	-208(%r10)
	jno	.LBB244_123
.LBB244_131:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r14
	jae	.LBB244_132
	.p2align	4
.LBB244_124:
	movq	(%r10), %rax
	mulq	-8(%r10)
	jo	.LBB244_133
.LBB244_125:
	addq	%rax, %r14
	movq	$-1, %r11
	jb	.LBB244_117
.LBB244_126:
	movq	%r14, %r11
	jmp	.LBB244_117
.LBB244_134:
	testq	%rdi, %rdi
	je	.LBB244_139
.LBB244_135:
	imulq	$200, %r9, %rax
	imulq	$200, %rdi, %rdi
	movq	$-1, %r10
	xorl	%r9d, %r9d
	leaq	176(%rax,%rcx), %r8
	.p2align	4
.LBB244_136:
	movq	(%r8,%r9), %rax
	mulq	-8(%r8,%r9)
	jo	.LBB244_138
.LBB244_137:
	addq	%rax, %r11
	cmovbq	%r10, %r11
	addq	$200, %r9
	cmpq	%r9, %rdi
	jne	.LBB244_136
	jmp	.LBB244_139
.LBB244_138:
	movq	$-1, %rax
	jmp	.LBB244_137
.LBB244_139:
	testq	%r11, %r11
	je	.LBB244_142
	cmpq	$0, 336(%rbx)
	je	.LBB244_142
	lock		addq	%r11, 352(%rbx)
.LBB244_142:
	movq	752(%rsp), %rax
	movq	%rcx, 128(%rsp)
	movq	%rcx, 136(%rsp)
	movq	%rax, 144(%rsp)
	movq	%rsi, 152(%rsp)
.Ltmp7443:
	leaq	896(%rsp), %rdi
	leaq	128(%rsp), %rsi
	callq	<purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>
.Ltmp7444:
	movzbl	1657(%rsp), %r9d
	movq	912(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_146
	movq	904(%rsp), %rcx
	cmpq	$8, %rax
	jae	.LBB244_147
	xorl	%edx, %edx
	xorl	%ebx, %ebx
	jmp	.LBB244_190
.LBB244_146:
	xorl	%ebx, %ebx
	jmp	.LBB244_192
.LBB244_147:
	cmpq	$32, %rax
	jae	.LBB244_183
	xorl	%edx, %edx
	xorl	%ebx, %ebx
	jmp	.LBB244_187
.LBB244_149:
	movzbl	139(%rsp), %ecx
	movzwl	137(%rsp), %esi
	vmovups	144(%rsp), %zmm0
	vmovups	160(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	140(%rsp), %ecx
	vmovups	%zmm0, 512(%rsp)
	vmovups	%zmm1, 528(%rsp)
.LBB244_150:
	vmovdqu64	512(%rsp), %zmm0
	vmovdqu64	528(%rsp), %zmm1
	movq	40(%rsp), %rdi
	movw	%si, 25(%rdi)
	shrl	$16, %esi
	movb	%sil, 27(%rdi)
	movl	%ecx, 28(%rdi)
	vmovdqu64	%zmm0, 32(%rdi)
	vmovdqu64	%zmm1, 48(%rdi)
	movq	%rax, 16(%rdi)
	movb	%dl, 24(%rdi)
	movq	$1, (%rdi)
	cmpq	$6, %r15
	jb	.LBB244_152
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB244_152:
	subq	%rbp, %r12
	je	.LBB244_165
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %r14
	xorl	%r15d, %r15d
	imulq	%r12, %r14
	jmp	.LBB244_157
.LBB244_154:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_155:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB244_156:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB244_165
.LBB244_157:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB244_156
	leaq	(%rbp,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rbx, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%rbx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_160
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_160:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_155
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_160
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB244_163:
	cmpq	%rax, %rdx
	jge	.LBB244_154
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB244_163
	jmp	.LBB244_154
.LBB244_165:
	movq	832(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_167
	movq	480(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB244_167:
	movq	8(%rsp), %rbp
	movq	40(%rsp), %r12
	movq	32(%rsp), %r15
	testq	%rbp, %rbp
	je	.LBB244_180
	xorl	%r14d, %r14d
	jmp	.LBB244_172
.LBB244_169:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_170:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB244_171:
	incq	%r14
	cmpq	%rbp, %r14
	je	.LBB244_180
.LBB244_172:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB244_171
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rbx, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%rbx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_175
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_175:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_170
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_175
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB244_178:
	cmpq	%rax, %rdx
	jge	.LBB244_169
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB244_178
	jmp	.LBB244_169
.LBB244_180:
	movq	48(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_182
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r15, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB244_182:
	movl	$0, 8(%rsp)
	jmp	.LBB244_334
.LBB244_183:
	vmovdqa64	.LCPI244_0(%rip), %zmm1
	vpbroadcastq	.LCPI244_1(%rip), %zmm2
	vpbroadcastq	.LCPI244_2(%rip), %zmm3
	movq	%rax, %rdx
	andq	$-32, %rdx
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rdx, %rsi
	.p2align	4
.LBB244_184:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rsi
	vpgatherqq	64(%rcx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%rcx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%rcx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%rcx,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB244_184
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rdx, %rax
	je	.LBB244_192
	testb	$24, %al
	je	.LBB244_190
.LBB244_187:
	movq	%rdx, %rsi
	vpbroadcastq	%rsi, %zmm1
	vporq	.LCPI244_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI244_1(%rip), %zmm2
	vpbroadcastq	.LCPI244_3(%rip), %zmm3
	movq	%rax, %rdx
	andq	$-8, %rdx
	vmovq	%rbx, %xmm0
	subq	%rdx, %rsi
	.p2align	4
.LBB244_188:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rsi
	vpgatherqq	64(%rcx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB244_188
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rdx, %rax
	je	.LBB244_192
.LBB244_190:
	subq	%rdx, %rax
	leaq	(%rdx,%rdx,4), %rdx
	shlq	$5, %rdx
	leaq	64(%rdx,%rcx), %rcx
	.p2align	4
.LBB244_191:
	addq	(%rcx), %rbx
	addq	$160, %rcx
	decq	%rax
	jne	.LBB244_191
.LBB244_192:
	movzbl	920(%rsp), %ebp
	movzbl	1659(%rsp), %edx
.Ltmp7446:
	leaq	128(%rsp), %rdi
	leaq	896(%rsp), %rcx
	leaq	456(%rsp), %r8
	movq	%r13, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>
.Ltmp7447:
	vmovups	136(%rsp), %xmm0
	movq	152(%rsp), %rcx
	movq	128(%rsp), %rax
	movq	%rcx, 528(%rsp)
	vmovaps	%xmm0, 512(%rsp)
	cmpq	$-1, %rax
	je	.LBB244_199
	vmovdqu	192(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	vmovdqa	512(%rsp), %xmm2
	movq	528(%rsp), %rcx
	movq	%rcx, 880(%rsp)
	vmovdqu	%ymm1, 80(%r12)
	vmovups	%ymm0, 64(%r12)
	vmovdqa	160(%rsp), %xmm0
	vmovdqa	%xmm2, 864(%rsp)
	movq	%rcx, 40(%r12)
	vmovdqu	%xmm2, 24(%r12)
	movq	%rax, 16(%r12)
	vmovdqa	%xmm0, 48(%r12)
	movq	$1, (%r12)
.Ltmp7449:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7450:
	movq	80(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_198
.LBB244_196:
	lock		decq	(%rax)
	jne	.LBB244_198
	#MEMBARRIER
.Ltmp7508:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7509:
.LBB244_198:
	movb	$1, %al
	movl	%eax, 8(%rsp)
	jmp	.LBB244_334
.LBB244_199:
	vmovdqa	512(%rsp), %xmm0
	movq	528(%rsp), %rax
	movl	160(%rsp), %esi
	movq	%rax, 144(%rsp)
	vmovdqa	%xmm0, 128(%rsp)
.Ltmp7452:
	movzbl	%bpl, %edx
	movq	%r13, %rdi
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>
	movq	%rdx, 832(%rsp)
.Ltmp7453:
	vmovdqa	128(%rsp), %xmm0
	movq	%rax, 480(%rsp)
	movq	144(%rsp), %rax
	movq	%r13, 16(%rsp)
	movq	%rax, 64(%rsp)
	vmovdqa	%xmm0, 48(%rsp)
.LBB244_201:
	movq	64(%rsp), %rax
	vmovdqa	48(%rsp), %xmm0
	movq	384(%rsp), %rcx
	movq	400(%rsp), %rdx
	movq	408(%rsp), %rsi
	movl	$1, %ebp
	movl	$1, %edi
	movq	%rax, 880(%rsp)
	movq	%rax, 64(%rsp)
	movq	392(%rsp), %rax
	cmpq	$3, %rcx
	movq	%rcx, %r12
	cmovaeq	%rdx, %r12
	cmovaeq	%rcx, %rdi
	cmovaeq	%rbp, %rdx
	movq	%rdi, 128(%rsp)
	vmovdqa	%xmm0, 864(%rsp)
	vmovdqa	%xmm0, 48(%rsp)
	movq	%rax, 136(%rsp)
	movq	%rdx, 144(%rsp)
	movq	%rsi, 152(%rsp)
	movq	%r12, %rsi
	decq	%rsi
	movq	$0, 160(%rsp)
	movq	%rsi, 168(%rsp)
	je	.LBB244_205
	cmpq	$3, %rcx
	movq	16(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %r13
	leaq	136(%rsp), %r14
	leaq	512(%rsp), %rbx
	cmovaeq	%rax, %r14
	leaq	640(%rcx), %r15
	.p2align	4
.LBB244_203:
	movq	%rbp, 160(%rsp)
	movq	16(%r14), %rax
	movq	%rax, 528(%rsp)
	vmovdqu	(%r14), %xmm0
	vmovdqa	%xmm0, 512(%rsp)
.Ltmp7460:
	movq	%r15, %rdi
	movq	%rbx, %rsi
	callq	*%r13
.Ltmp7461:
	addq	$24, %r14
	incq	%rbp
	cmpq	%rbp, %r12
	jne	.LBB244_203
.LBB244_205:
.Ltmp7466:
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7467:
	movq	40(%rsp), %r12
	movq	8(%rsp), %r8
	testb	$1, 480(%rsp)
	movq	16(%rsp), %r15
	movq	32(%rsp), %rdx
	movq	832(%rsp), %rdi
	je	.LBB244_220
	cmpq	%rdx, %rdi
	ja	.LBB244_395
	movq	616(%r15), %rsi
	testq	%rsi, %rsi
	je	.LBB244_213
	cmpq	$0, 336(%rsi)
	leaq	288(%rsp), %r9
	leaq	200(%rsp), %rcx
	leaq	216(%rsp), %rax
	jne	.LBB244_214
	cmpq	$-1, 16(%rsi)
	jne	.LBB244_214
	cmpq	$-1, 40(%rsi)
	jne	.LBB244_214
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%r9)
	movq	$0, 16(%r9)
	movq	$-1, 312(%rsp)
	movl	$67108864, 320(%rsp)
	jmp	.LBB244_215
.LBB244_213:
	leaq	200(%rsp), %rcx
	leaq	216(%rsp), %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, 288(%rsp)
	movq	$0, 304(%rsp)
	movq	$-1, 312(%rsp)
	movl	$67108864, 320(%rsp)
	movq	$0, 128(%rsp)
	movq	$8, 136(%rsp)
	vmovdqu	%xmm0, 144(%rsp)
	movq	$8, 160(%rsp)
	vmovdqu	%xmm0, 168(%rsp)
	jmp	.LBB244_216
.LBB244_214:
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%r9)
	movq	$0, 16(%r9)
	movq	$-1, 312(%rsp)
	movl	$67174400, 320(%rsp)
.LBB244_215:
	movq	$0, 128(%rsp)
	movq	$8, 136(%rsp)
	vmovdqu	%xmm0, -144(%r9)
	movq	$8, 160(%rsp)
	vmovdqu	%xmm0, -120(%r9)
.LBB244_216:
	movq	$8, 184(%rsp)
	movq	$0, 192(%rsp)
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rcx)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, (%rax)
	movb	$0, 64(%rax)
	cmpq	%rdx, %rdi
	jne	.LBB244_284
.LBB244_218:
.Ltmp7485:
	leaq	128(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7486:
	movq	16(%rsp), %r15
.LBB244_220:
	movq	64(%rsp), %rax
	vmovdqa	48(%rsp), %xmm0
	movq	%rax, 432(%rsp)
	movq	80(%rsp), %rax
	vmovdqa	%xmm0, 416(%rsp)
	testq	%rax, %rax
	je	.LBB244_223
	lock		decq	(%rax)
	jne	.LBB244_223
	#MEMBARRIER
.Ltmp7487:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	callq	*%rax
.Ltmp7488:
.LBB244_223:
	movq	448(%rsp), %r14
	movb	$1, %al
	movl	%eax, 8(%rsp)
	movq	696(%r15), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB244_240
.LBB244_224:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB244_240
	movb	%cl, 512(%rsp)
	movq	24(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 513(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 528(%rsp)
	lock		incq	(%rcx)
	jle	.LBB244_396
	movq	24(%rsp), %rcx
.Ltmp7489:
	leaq	128(%rsp), %rdi
	leaq	512(%rsp), %rdx
	movq	%r14, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp7490:
	vmovdqu64	160(%rsp), %zmm1
	vmovdqu64	128(%rsp), %zmm0
	movq	424(%rsp), %rbx
	movq	432(%rsp), %r14
	vmovdqu64	%zmm1, 40(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	movq	$0, (%r12)
	testq	%r14, %r14
	je	.LBB244_273
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB244_232
	.p2align	4
.LBB244_229:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_230:
	vzeroupper
	callq	*%rbp
.LBB244_231:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB244_273
.LBB244_232:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB244_231
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_235
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_235:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_230
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_235
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB244_238:
	cmpq	%rax, %rdx
	jge	.LBB244_229
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB244_238
	jmp	.LBB244_229
.LBB244_240:
	vmovups	1176(%rsp), %zmm1
	vmovdqu64	1136(%rsp), %zmm0
	movq	432(%rsp), %rcx
	movq	24(%rsp), %rax
	movq	%rcx, 944(%rsp)
	vmovups	%zmm1, 168(%rsp)
	vmovdqa	416(%rsp), %xmm1
	vmovdqu64	%zmm0, 128(%rsp)
	cmpq	$-1, 128(%rsp)
	vmovdqa	%xmm1, 928(%rsp)
	movq	%rax, 952(%rsp)
	je	.LBB244_243
	leaq	512(%rsp), %rdi
	leaq	928(%rsp), %rsi
	leaq	1136(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	200(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB244_242
.LBB244_244:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	208(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_246
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_246:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_252
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_246
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB244_249:
	cmpq	%rax, %rsi
	jge	.LBB244_251
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB244_249
.LBB244_251:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_252:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	224(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB244_253
	jmp	.LBB244_255
.LBB244_243:
	vmovdqu	928(%rsp), %xmm0
	movq	944(%rsp), %rax
	movq	952(%rsp), %rcx
	movq	%rax, 536(%rsp)
	movq	%rcx, 544(%rsp)
	vmovdqu	%xmm0, 520(%rsp)
	movq	$-1, 512(%rsp)
	movq	200(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB244_244
.LBB244_242:
	movq	224(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_255
.LBB244_253:
	lock		decq	(%rax)
	jne	.LBB244_255
	leaq	224(%rsp), %rdi
	#MEMBARRIER
.Ltmp7492:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp7493:
.LBB244_255:
	vmovdqu64	544(%rsp), %zmm1
	vmovdqu64	512(%rsp), %zmm0
	movl	$0, 92(%rsp)
	vmovdqu64	%zmm1, 40(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	movq	$0, (%r12)
.Ltmp7495:
	leaq	1664(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7496:
.Ltmp7497:
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7498:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB244_259
	#MEMBARRIER
.Ltmp7500:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7501:
.LBB244_259:
	cmpb	$0, 8(%rsp)
	je	.LBB244_388
	movq	104(%rsp), %r14
	movq	112(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB244_377
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB244_265
	.p2align	4
.LBB244_262:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_263:
	callq	*%r13
.LBB244_264:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB244_377
.LBB244_265:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB244_264
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_268
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_268:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_263
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_268
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB244_271:
	cmpq	%rax, %rdx
	jge	.LBB244_262
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB244_271
	jmp	.LBB244_262
.LBB244_273:
	movq	416(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_333
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_276
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB244_276:
	movq	40(%rsp), %r12
	.p2align	4
.LBB244_277:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_283
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_277
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB244_280:
	cmpq	%rax, %rsi
	jge	.LBB244_282
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB244_280
.LBB244_282:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_283:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB244_334
.LBB244_284:
	leaq	520(%rsp), %rcx
	leaq	(%rdx,%rdx,4), %rax
	leaq	512(%rsp), %rbx
	leaq	128(%rsp), %r15
	vpbroadcastq	%rcx, %ymm0
	vpaddq	.LCPI244_4(%rip), %ymm0, %ymm1
	vpaddq	.LCPI244_5(%rip), %ymm0, %ymm0
	leaq	(%r8,%rax,8), %r14
	leaq	(%rdi,%rdi,4), %rax
	leaq	(%r8,%rax,8), %rbp
	movabsq	$2305843009213693920, %rax
	movq	%r14, 32(%rsp)
	addq	$31, %rax
	movq	%rax, 8(%rsp)
	vmovdqu	%ymm1, 480(%rsp)
	vmovdqu	%ymm0, 832(%rsp)
	jmp	.LBB244_287
.LBB244_285:
	movq	56(%rsp), %rax
	leaq	(%r15,%r15,4), %rcx
	incq	%r15
	movq	%r12, (%rax,%rcx,8)
	movq	%r14, 8(%rax,%rcx,8)
	movq	%rbx, 16(%rax,%rcx,8)
	movq	40(%rsp), %r12
	movq	32(%rsp), %r14
	leaq	512(%rsp), %rbx
	vmovdqa	752(%rsp), %xmm0
	vmovdqu	%xmm0, 24(%rax,%rcx,8)
	movq	%r15, 64(%rsp)
	leaq	128(%rsp), %r15
.LBB244_286:
	addq	$40, %rbp
	cmpq	%r14, %rbp
	je	.LBB244_218
.LBB244_287:
.Ltmp7468:
	movq	16(%rsp), %rdx
	movq	%rbx, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7469:
	cmpb	$-1, 512(%rsp)
	jne	.LBB244_218
	movq	(%rbp), %rcx
	leaq	8(%rbp), %r13
	movq	%r13, %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB244_291
	movq	16(%rbp), %rcx
	movq	8(%rbp), %rdx
	decq	%rcx
.LBB244_291:
	movq	24(%rsp), %r8
	addq	$16, %r8
.Ltmp7470:
	movq	16(%rsp), %r9
	leaq	1664(%rsp), %rsi
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7471:
	movq	512(%rsp), %rax
	movl	520(%rsp), %edx
	movl	524(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB244_391
	cmpl	$2, %edx
	je	.LBB244_286
.Ltmp7472:
	movq	16(%rsp), %rsi
	movq	%rbx, %rdi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp7473:
	movq	512(%rsp), %rax
	movzbl	520(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB244_392
	testb	$1, %dl
	je	.LBB244_286
	movq	(%rbp), %rax
	movq	16(%rbp), %r12
	leaq	-1(%rax), %r15
	decq	%r12
	cmpq	$5, %r15
	cmovbq	%r15, %r12
	cmpq	$4, %r12
	jbe	.LBB244_304
	cmpq	8(%rsp), %r12
	leaq	(,%r12,8), %rbx
	movabsq	$9223372036854775804, %rcx
	seta	%al
	cmpq	%rcx, %rbx
	seta	%cl
	orb	%al, %cl
	jne	.LBB244_389
	movl	$4, %esi
	movq	%rbx, %rdi
	callq	__rustc::__rust_alloc
	movl	$4, %edi
	testq	%rax, %rax
	je	.LBB244_390
	movq	%rax, %r14
	cmpq	$5, %r15
	jb	.LBB244_302
	movq	(%r13), %r13
.LBB244_302:
	cmpq	$8, %r12
	jb	.LBB244_303
	leaq	(%r13,%rbx), %rax
	cmpq	%rax, %r14
	jae	.LBB244_313
	movq	%r14, %rax
	addq	%rbx, %rax
	cmpq	%rax, %r13
	jae	.LBB244_313
.LBB244_303:
	xorl	%eax, %eax
.LBB244_323:
	movq	%r12, %rdx
	andq	$7, %rdx
	movq	%rax, %rcx
	je	.LBB244_327
	movq	%rax, %rcx
.LBB244_325:
	movl	(%r13,%rcx,8), %esi
	movl	4(%r13,%rcx,8), %edi
	movl	%esi, (%r14,%rcx,8)
	movl	%edi, 4(%r14,%rcx,8)
	incq	%rcx
	decq	%rdx
	jne	.LBB244_325
	leaq	-1(%rcx), %rbx
.LBB244_327:
	subq	%r12, %rax
	cmpq	$-8, %rax
	ja	.LBB244_330
	movq	%r12, %rax
	decq	%rcx
	negq	%rax
	movq	%rcx, %rbx
.LBB244_329:
	movl	8(%r13,%rbx,8), %ecx
	movl	12(%r13,%rbx,8), %edx
	movl	%ecx, 8(%r14,%rbx,8)
	movl	%edx, 12(%r14,%rbx,8)
	movl	16(%r13,%rbx,8), %ecx
	movl	20(%r13,%rbx,8), %edx
	movl	%ecx, 16(%r14,%rbx,8)
	movl	%edx, 20(%r14,%rbx,8)
	movl	24(%r13,%rbx,8), %ecx
	movl	28(%r13,%rbx,8), %edx
	movl	%ecx, 24(%r14,%rbx,8)
	movl	%edx, 28(%r14,%rbx,8)
	movl	32(%r13,%rbx,8), %ecx
	movl	36(%r13,%rbx,8), %edx
	movl	%ecx, 32(%r14,%rbx,8)
	movl	%edx, 36(%r14,%rbx,8)
	movl	40(%r13,%rbx,8), %ecx
	movl	44(%r13,%rbx,8), %edx
	movl	%ecx, 40(%r14,%rbx,8)
	movl	%edx, 44(%r14,%rbx,8)
	movl	48(%r13,%rbx,8), %ecx
	movl	52(%r13,%rbx,8), %edx
	movl	%ecx, 48(%r14,%rbx,8)
	movl	%edx, 52(%r14,%rbx,8)
	movl	56(%r13,%rbx,8), %ecx
	movl	60(%r13,%rbx,8), %edx
	movl	%ecx, 56(%r14,%rbx,8)
	movl	%edx, 60(%r14,%rbx,8)
	movl	64(%r13,%rbx,8), %ecx
	movl	68(%r13,%rbx,8), %edx
	movl	%ecx, 64(%r14,%rbx,8)
	movl	%edx, 68(%r14,%rbx,8)
	leaq	8(%rax,%rbx), %rdx
	addq	$8, %rbx
	cmpq	$-1, %rdx
	jne	.LBB244_329
	jmp	.LBB244_330
.LBB244_304:
	cmpq	$6, %rax
	jb	.LBB244_306
	movq	(%r13), %r13
.LBB244_306:
	testq	%r12, %r12
	je	.LBB244_308
	leaq	-1(%r12), %rax
	vpmovsxbd	.LCPI244_8(%rip), %xmm2
	vmovdqu	480(%rsp), %ymm3
	vpxor	%xmm1, %xmm1, %xmm1
	incq	%r12
	vpbroadcastq	%rax, %ymm0
	vpcmpnltuq	.LCPI244_6(%rip), %ymm0, %k1
	vpxor	%xmm0, %xmm0, %xmm0
	kmovq	%k1, %k2
	vpgatherdd	(%r13,%xmm2), %xmm0 {%k2}
	kmovq	%k1, %k2
	vpgatherdd	4(%r13,%xmm2), %xmm1 {%k2}
	kmovq	%k1, %k2
	vpscatterqd	%xmm0, (,%ymm3) {%k2}
	vmovdqu	832(%rsp), %ymm0
	vpscatterqd	%xmm1, (,%ymm0) {%k1}
	jmp	.LBB244_309
.LBB244_308:
	movl	$1, %r12d
.LBB244_309:
	movq	%r12, 512(%rsp)
	leaq	520(%rsp), %rax
	vmovdqu	16(%rax), %xmm0
	movq	520(%rsp), %r14
	movq	528(%rsp), %rbx
	vmovdqa	%xmm0, 752(%rsp)
	jmp	.LBB244_331
.LBB244_313:
	cmpq	$32, %r12
	jae	.LBB244_315
	xorl	%eax, %eax
	jmp	.LBB244_319
.LBB244_315:
	movabsq	$2305843009213693920, %rcx
	movq	%r12, %rax
	andq	%rcx, %rax
	xorl	%ecx, %ecx
.LBB244_316:
	vmovdqu64	(%r13,%rcx,8), %zmm0
	vmovdqu64	64(%r13,%rcx,8), %zmm1
	vmovdqu64	128(%r13,%rcx,8), %zmm2
	vmovdqu64	192(%r13,%rcx,8), %zmm3
	vmovdqu64	%zmm0, (%r14,%rcx,8)
	vmovdqu64	%zmm1, 64(%r14,%rcx,8)
	vmovdqu64	%zmm2, 128(%r14,%rcx,8)
	vmovdqu64	%zmm3, 192(%r14,%rcx,8)
	addq	$32, %rcx
	cmpq	%rcx, %rax
	jne	.LBB244_316
	cmpq	%rax, %r12
	je	.LBB244_322
	testb	$24, %r12b
	je	.LBB244_323
.LBB244_319:
	movq	%rax, %rcx
	movabsq	$2305843009213693920, %rax
	addq	$24, %rax
	andq	%r12, %rax
.LBB244_320:
	vmovdqu64	(%r13,%rcx,8), %zmm0
	vmovdqu64	%zmm0, (%r14,%rcx,8)
	addq	$8, %rcx
	cmpq	%rcx, %rax
	jne	.LBB244_320
	cmpq	%rax, %r12
	jne	.LBB244_323
.LBB244_322:
	decq	%rax
	movq	%rax, %rbx
.LBB244_330:
	addq	$2, %rbx
	incq	%r12
.LBB244_331:
	movq	64(%rsp), %r15
	cmpq	48(%rsp), %r15
	jne	.LBB244_285
.Ltmp7477:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp7478:
	jmp	.LBB244_285
.LBB244_333:
	movq	40(%rsp), %r12
.LBB244_334:
.Ltmp7513:
	leaq	1664(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7514:
.Ltmp7518:
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7519:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB244_338
	#MEMBARRIER
.Ltmp7523:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp7524:
.LBB244_338:
	movq	1208(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB244_348
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1216(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_341
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_341:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_347
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_341
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB244_344:
	cmpq	%rax, %rsi
	jge	.LBB244_346
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB244_344
.LBB244_346:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_347:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB244_348:
	movq	1136(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB244_358
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1144(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_351
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_351:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_357
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_351
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB244_354:
	cmpq	%rax, %rsi
	jge	.LBB244_356
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB244_354
.LBB244_356:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_357:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB244_358:
	movq	1232(%rsp), %rax
	testq	%rax, %rax
	je	.LBB244_361
	lock		decq	(%rax)
	jne	.LBB244_361
	leaq	1232(%rsp), %rdi
	#MEMBARRIER
.Ltmp7528:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp7529:
.LBB244_361:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB244_363
	#MEMBARRIER
.Ltmp7534:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7535:
.LBB244_363:
	cmpb	$0, 8(%rsp)
	je	.LBB244_388
	movq	104(%rsp), %r14
	movq	112(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB244_377
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB244_369
	.p2align	4
.LBB244_366:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_367:
	callq	*%r13
.LBB244_368:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB244_377
.LBB244_369:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB244_368
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_372
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_372:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_367
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_372
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB244_375:
	cmpq	%rax, %rdx
	jge	.LBB244_366
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB244_375
	jmp	.LBB244_366
.LBB244_377:
	movq	96(%rsp), %rax
	movq	40(%rsp), %r12
	testq	%rax, %rax
	je	.LBB244_388
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB244_380
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB244_380:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB244_386
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB244_380
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB244_383:
	cmpq	%rax, %rsi
	jge	.LBB244_385
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB244_383
.LBB244_385:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB244_386:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
.LBB244_387:
	vzeroupper
	callq	*%rax
.LBB244_388:
	movq	%r12, %rax
	addq	$1880, %rsp
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
	vzeroupper
	retq
.LBB244_389:
	.cfi_def_cfa_offset 1936
	xorl	%edi, %edi
.LBB244_390:
.Ltmp7480:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp7481:
	jmp	.LBB244_396
.LBB244_391:
	vmovups	544(%rsp), %zmm1
	vmovups	528(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 768(%rsp)
	vmovups	%zmm0, 752(%rsp)
	jmp	.LBB244_393
.LBB244_392:
	movzbl	523(%rsp), %ecx
	movzwl	521(%rsp), %esi
	vmovups	528(%rsp), %zmm0
	vmovups	544(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	524(%rsp), %ecx
	vmovups	%zmm0, 752(%rsp)
	vmovups	%zmm1, 768(%rsp)
.LBB244_393:
	vmovups	752(%rsp), %zmm0
	vmovups	768(%rsp), %zmm1
	movw	%si, 25(%r12)
	shrl	$16, %esi
	movb	%sil, 27(%r12)
	movl	%ecx, 28(%r12)
	vmovups	%zmm0, 32(%r12)
	vmovups	%zmm1, 48(%r12)
	movq	%rax, 16(%r12)
	movb	%dl, 24(%r12)
	movq	$1, (%r12)
.Ltmp7475:
	leaq	128(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7476:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	80(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB244_196
	jmp	.LBB244_198
.LBB244_395:
.Ltmp7503:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.405(%rip), %rcx
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp7504:
.LBB244_396:
	ud2
.LBB244_397:
.Ltmp7479:
	movq	%rax, %rbx
	cmpq	$6, %r12
	jb	.LBB244_436
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
	jmp	.LBB244_436
.LBB244_399:
.Ltmp7454:
	leaq	128(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB244_409
.LBB244_400:
.Ltmp7451:
	jmp	.LBB244_420
.LBB244_401:
.Ltmp7491:
	leaq	416(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movl	8(%rsp), %r14d
	jmp	.LBB244_443
.LBB244_402:
.Ltmp7494:
	movl	8(%rsp), %r14d
	movq	%rax, %rbx
	xorl	%ebp, %ebp
	jmp	.LBB244_444
.LBB244_403:
.Ltmp7448:
	jmp	.LBB244_408
.LBB244_404:
.Ltmp7445:
	jmp	.LBB244_406
.LBB244_405:
.Ltmp7442:
.LBB244_406:
	leaq	456(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB244_409
.LBB244_407:
.Ltmp7457:
.LBB244_408:
	movq	%rax, %rbx
.LBB244_409:
.Ltmp7458:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7459:
	jmp	.LBB244_439
.LBB244_410:
.Ltmp7530:
	jmp	.LBB244_418
.LBB244_411:
.Ltmp7502:
	jmp	.LBB244_414
.LBB244_412:
.Ltmp7429:
	movq	%rax, %rbx
	movq	%rbp, 392(%rsp)
	cmpq	$5, %r15
	ja	.LBB244_431
	jmp	.LBB244_432
.LBB244_413:
.Ltmp7536:
.LBB244_414:
	cmpb	$0, 8(%rsp)
	movq	%rax, %rbx
	jne	.LBB244_453
	jmp	.LBB244_454
.LBB244_415:
.Ltmp7525:
	movl	8(%rsp), %r14d
	movq	%rax, %rbx
	jmp	.LBB244_449
.LBB244_416:
.Ltmp7439:
	leaq	456(%rsp), %rdi
	movq	%rax, %rbx
	jmp	.LBB244_438
.LBB244_417:
.Ltmp7499:
.LBB244_418:
	movl	8(%rsp), %r14d
	movq	%rax, %rbx
	jmp	.LBB244_450
.LBB244_419:
.Ltmp7436:
.LBB244_420:
	movq	%rax, %rbx
	jmp	.LBB244_439
.LBB244_421:
.Ltmp7510:
	movq	%rax, %rbx
	movb	$1, %r14b
	jmp	.LBB244_443
.LBB244_422:
.Ltmp7474:
	jmp	.LBB244_435
.LBB244_423:
.Ltmp7520:
	movl	8(%rsp), %r14d
	movq	%rax, %rbx
	jmp	.LBB244_447
.LBB244_424:
.Ltmp7515:
	movl	92(%rsp), %ebp
	movl	8(%rsp), %r14d
	movq	%rax, %rbx
	jmp	.LBB244_445
.LBB244_425:
.Ltmp7462:
	movq	%rax, %rbx
.Ltmp7463:
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp7464:
	jmp	.LBB244_437
.LBB244_426:
.Ltmp7465:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB244_427:
.Ltmp7408:
	movq	%rax, %rbx
.Ltmp7409:
	leaq	1240(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7410:
	jmp	.LBB244_454
.LBB244_428:
.Ltmp7411:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB244_429:
.Ltmp7426:
	movq	%rax, %rbx
	movq	%rbp, 392(%rsp)
	cmpq	$6, %r15
	jb	.LBB244_432
	movq	936(%rsp), %r14
.LBB244_431:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB244_432:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bpl
	xorl	%r14d, %r14d
	jmp	.LBB244_444
.LBB244_433:
.Ltmp7505:
	movq	%rax, %rbx
	jmp	.LBB244_437
.LBB244_434:
.Ltmp7482:
.LBB244_435:
	movq	%rax, %rbx
.LBB244_436:
.Ltmp7483:
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7484:
.LBB244_437:
	leaq	48(%rsp), %rdi
.LBB244_438:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB244_439:
	movq	80(%rsp), %rax
	movb	$1, %r14b
	testq	%rax, %rax
	je	.LBB244_443
	lock		decq	(%rax)
	movb	$1, %r14b
	jne	.LBB244_443
	movb	$1, %r14b
	#MEMBARRIER
.Ltmp7506:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	callq	*%rax
.Ltmp7507:
	movb	$1, %bpl
	jmp	.LBB244_444
.LBB244_443:
	movb	$1, %bpl
.LBB244_444:
.Ltmp7511:
	leaq	1664(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp7512:
.LBB244_445:
.Ltmp7516:
	leaq	1464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp7517:
	testb	%bpl, %bpl
	je	.LBB244_450
.LBB244_447:
	movq	24(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB244_449
	#MEMBARRIER
.Ltmp7521:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %rdi
	callq	*%rax
.Ltmp7522:
.LBB244_449:
.Ltmp7526:
	leaq	1136(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp7527:
.LBB244_450:
	movq	120(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB244_452
	#MEMBARRIER
.Ltmp7531:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	120(%rsp), %rdi
	callq	*%rax
.Ltmp7532:
.LBB244_452:
	testb	%r14b, %r14b
	je	.LBB244_454
.LBB244_453:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB244_454:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB244_455:
.Ltmp7533:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end244:
purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, &purrdf_sparql_algebra::algebra::ApplicationPolicy>:
.Lfunc_begin314:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception221
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
	subq	$856, %rsp
	.cfi_def_cfa_offset 912
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %r14
	leaq	632(%rsp), %rdi
	movq	%r9, %r13
	movq	%r8, %r12
	movq	%rcx, %r15
	movq	%rdx, %rbx
	callq	*%rax
.Ltmp11134:
	leaq	736(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r13, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp11135:
	cmpl	$1, 736(%rsp)
	jne	.LBB314_26
	vmovdqu64	784(%rsp), %zmm1
	vmovdqu64	752(%rsp), %zmm0
	vmovdqu64	%zmm1, 48(%r14)
	vmovdqu64	%zmm0, 16(%r14)
	movq	$1, (%r14)
.LBB314_3:
	movq	704(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB314_13
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	712(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB314_6
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB314_6:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_12
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_6
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB314_9:
	cmpq	%rax, %rsi
	jge	.LBB314_11
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB314_9
.LBB314_11:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_12:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB314_13:
	movq	632(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB314_23
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	640(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB314_16
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB314_16:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_22
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_16
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB314_19:
	cmpq	%rax, %rsi
	jge	.LBB314_21
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB314_19
.LBB314_21:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_22:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB314_23:
	movq	728(%rsp), %rax
	testq	%rax, %rax
	je	.LBB314_75
	lock		decq	(%rax)
	jne	.LBB314_75
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	728(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB314_75
.LBB314_26:
	vmovdqu64	776(%rsp), %zmm1
	vmovdqu64	744(%rsp), %zmm0
	vmovdqu64	%zmm1, 208(%rsp)
	vmovdqu64	%zmm0, 176(%rsp)
.Ltmp11136:
	leaq	320(%rsp), %rdi
	leaq	632(%rsp), %rsi
	leaq	176(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11137:
	cmpq	$-1, 320(%rsp)
	je	.LBB314_32
	vmovdqu	320(%rsp), %ymm0
	movq	632(%rsp), %rbx
	movq	%r14, 72(%rsp)
	movq	%rbx, 48(%rsp)
	vmovdqu	%ymm0, 288(%rsp)
	cmpq	$-1, %rbx
	je	.LBB314_33
	vmovups	672(%rsp), %zmm1
	vmovups	632(%rsp), %zmm0
	movq	312(%rsp), %rax
	movq	%rax, 120(%rsp)
	movq	$0, 96(%rsp)
	movq	$8, 104(%rsp)
	movq	$0, 112(%rsp)
	vmovups	%zmm1, 216(%rsp)
	vmovups	%zmm0, 176(%rsp)
	cmpq	$-1, 176(%rsp)
	je	.LBB314_38
	leaq	320(%rsp), %rdi
	leaq	96(%rsp), %rsi
	leaq	632(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	248(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB314_31
.LBB314_39:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	256(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB314_41
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB314_41:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_47
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_41
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB314_44:
	cmpq	%rax, %rsi
	jge	.LBB314_46
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB314_44
.LBB314_46:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_47:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	272(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB314_48
	jmp	.LBB314_50
.LBB314_32:
	leaq	8(%r14), %rdi
	leaq	632(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	movq	$0, (%r14)
	jmp	.LBB314_75
.LBB314_33:
	movb	$1, %bpl
.Ltmp11142:
	leaq	176(%rsp), %rdi
	leaq	288(%rsp), %rsi
	movq	%r15, %rdx
	movq	%r13, %rcx
	vzeroupper
	callq	purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11143:
	cmpq	$-1, 176(%rsp)
	je	.LBB314_76
.LBB314_35:
	vmovdqu64	208(%rsp), %zmm1
	vmovdqu64	176(%rsp), %zmm0
	vmovdqu64	%zmm1, 48(%r14)
	vmovdqu64	%zmm0, 16(%r14)
	movq	$1, (%r14)
.LBB314_36:
	movq	312(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB314_51
	leaq	312(%rsp), %rdi
	#MEMBARRIER
.Ltmp11242:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11243:
	jmp	.LBB314_51
.LBB314_38:
	movq	104(%rsp), %rcx
	movq	96(%rsp), %rax
	movq	112(%rsp), %rdx
	movq	%rcx, 336(%rsp)
	movq	120(%rsp), %rcx
	movq	%rax, 328(%rsp)
	movq	%rdx, 344(%rsp)
	movq	%rcx, 352(%rsp)
	movq	$-1, 320(%rsp)
	movq	248(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB314_39
.LBB314_31:
	movq	272(%rsp), %rax
	testq	%rax, %rax
	je	.LBB314_50
.LBB314_48:
	lock		decq	(%rax)
	jne	.LBB314_50
	leaq	272(%rsp), %rdi
	#MEMBARRIER
.Ltmp11139:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11140:
.LBB314_50:
	vmovdqu64	352(%rsp), %zmm1
	vmovdqu64	320(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%r14)
	vmovdqu64	%zmm0, 8(%r14)
	movq	$0, (%r14)
.LBB314_51:
	movq	296(%rsp), %r14
	movq	304(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB314_64
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB314_56
	.p2align	4
.LBB314_53:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_54:
	vzeroupper
	callq	*%r13
.LBB314_55:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB314_64
.LBB314_56:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB314_55
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB314_59
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB314_59:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_54
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_59
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB314_62:
	cmpq	%rax, %rdx
	jge	.LBB314_53
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB314_62
	jmp	.LBB314_53
.LBB314_64:
	movq	288(%rsp), %rax
	movq	48(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB314_74
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB314_67
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB314_67:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_73
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_67
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB314_70:
	cmpq	%rax, %rsi
	jge	.LBB314_72
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB314_70
.LBB314_72:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_73:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB314_74:
	movq	72(%rsp), %r14
	cmpq	$-1, %rbx
	je	.LBB314_3
.LBB314_75:
	movq	%r14, %rax
	addq	$856, %rsp
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
	vzeroupper
	retq
.LBB314_76:
	.cfi_def_cfa_offset 912
	movq	584(%r13), %rdx
	testq	%rdx, %rdx
	je	.LBB314_88
	cmpq	$0, 40(%rdx)
	je	.LBB314_88
	vpbroadcastq	.LCPI314_6(%rip), %xmm0
	movabsq	$2746377873070565055, %rax
	movq	16(%rdx), %rcx
	movq	24(%rdx), %rdx
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	xorq	%r15, %rax
	vpinsrq	$0, %rax, %xmm0, %xmm0
	vaesenc	.LCPI314_1(%rip), %xmm0, %xmm0
	vaesenc	.LCPI314_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI314_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rax
	movq	%rax, %rsi
	shrq	$57, %rsi
	vpbroadcastb	%esi, %xmm0
	xorl	%esi, %esi
.LBB314_79:
	andq	%rdx, %rax
	vmovdqu	(%rcx,%rax), %xmm2
	vpcmpeqb	%xmm0, %xmm2, %k0
	kortestw	%k0, %k0
	je	.LBB314_83
	kmovd	%k0, %edi
.LBB314_81:
	xorl	%r8d, %r8d
	tzcntl	%edi, %r8d
	addq	%rax, %r8
	andq	%rdx, %r8
	negq	%r8
	leaq	(%r8,%r8,4), %r8
	cmpq	%r15, -40(%rcx,%r8,8)
	je	.LBB314_85
	leal	-1(%rdi), %r8d
	andw	%di, %r8w
	movl	%r8d, %edi
	jne	.LBB314_81
.LBB314_83:
	vpcmpeqb	%xmm1, %xmm2, %k0
	kortestw	%k0, %k0
	jne	.LBB314_88
	leaq	16(%rax,%rsi), %rax
	addq	$16, %rsi
	jmp	.LBB314_79
.LBB314_85:
	leaq	(%rcx,%r8,8), %rsi
	cmpl	$1, -32(%rsi)
	jne	.LBB314_88
	addq	$-24, %rsi
	leaq	528(%rsp), %rdi
	callq	<purrdf_sparql_eval::deferred_exists::DeferredLateral as core::clone::Clone>::clone
	cmpq	$0, 528(%rsp)
	je	.LBB314_89
	xorl	%ecx, %ecx
	jmp	.LBB314_96
.LBB314_88:
	movq	$0, 528(%rsp)
.LBB314_89:
	cmpl	$23, (%r15)
	movb	$1, %cl
	jne	.LBB314_96
	movq	32(%r15), %rax
	testq	%rax, %rax
	je	.LBB314_283
	cmpl	$21, (%rax)
	jne	.LBB314_96
	cmpq	$0, 48(%rax)
	jne	.LBB314_96
	cmpq	$1, 24(%rax)
	jne	.LBB314_96
	movq	16(%rax), %rax
	cmpq	$22, 8(%rax)
	jne	.LBB314_96
	movq	(%rax), %rax
	vmovdqu	22(%rax), %xmm1
	vmovdqu	16(%rax), %xmm0
	vpxor	.LCPI314_4(%rip), %xmm0, %xmm0
	vpternlogq	$246, .LCPI314_5(%rip), %xmm1, %xmm0
	vptest	%xmm0, %xmm0
	je	.LBB314_279
.LBB314_96:
	movq	312(%rsp), %rbp
	movq	%r15, 88(%rsp)
	lock		incq	(%rbp)
	jle	.LBB314_289
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	304(%rsp), %r15
	movabsq	$128102389400760776, %rax
	movq	$0, 576(%rsp)
	movq	$8, 584(%rsp)
	movq	$0, 592(%rsp)
	movq	%rbp, 472(%rsp)
	decq	%rax
	vmovdqu	%ymm0, 600(%rsp)
	cmpq	%rax, %r15
	jbe	.LBB314_100
	xorl	%r14d, %r14d
.LBB314_99:
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp11233:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11234:
	jmp	.LBB314_289
.LBB314_100:
	movq	%rbp, 24(%rsp)
	movq	%r13, 440(%rsp)
	testq	%r15, %r15
	je	.LBB314_144
	leaq	(,%r15,8), %rax
	movl	$8, %esi
	movl	%ecx, 448(%rsp)
	movq	%r12, 464(%rsp)
	movl	$8, %r14d
	leaq	(%rax,%rax,8), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB314_99
	movq	296(%rsp), %r14
	movq	%r15, 416(%rsp)
	movq	%rax, 424(%rsp)
	movq	%rax, %r12
	leaq	(%r15,%r15,4), %rax
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r15
	leaq	16(%rbp), %rcx
	leaq	576(%rsp), %rbp
	xorl	%ebx, %ebx
	movq	$0, 432(%rsp)
	movq	$0, 64(%rsp)
	movq	%rcx, 80(%rsp)
	leaq	(%r14,%rax,8), %rax
	movq	%rax, 456(%rsp)
	jmp	.LBB314_104
.LBB314_103:
	addq	$40, %r14
	cmpq	456(%rsp), %r14
	je	.LBB314_154
.LBB314_104:
	leaq	63(%rsp), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.457(%rip), %r10
	movq	%rax, 152(%rsp)
	movq	%r10, 160(%rsp)
	movb	$0, 168(%rsp)
.Ltmp11144:
	movq	88(%rsp), %rsi
	movq	80(%rsp), %rcx
	movq	464(%rsp), %r8
	leaq	176(%rsp), %rdi
	leaq	152(%rsp), %r9
	movq	%r14, %rdx
	movq	%r13, (%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::binop::evaluate_application_row_with::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp11145:
	cmpl	$1, 176(%rsp)
	je	.LBB314_145
	leaq	184(%rsp), %rcx
	movq	184(%rsp), %rax
	vmovups	32(%rcx), %zmm1
	vmovups	8(%rcx), %zmm0
	leaq	352(%rsp), %rcx
	vmovups	%zmm1, 344(%rsp)
	vmovups	%zmm0, 320(%rsp)
	vmovdqu	320(%rsp), %ymm0
	vmovdqu	24(%rcx), %ymm1
	vmovdqu	(%rcx), %ymm2
	vmovdqu	%ymm1, 120(%rsp)
	vmovdqu	%ymm0, 480(%rsp)
	vmovdqu	%ymm2, 96(%rsp)
	cmpq	$-1, %rax
	jne	.LBB314_151
	movq	504(%rsp), %rax
	movq	%rbx, 40(%rsp)
	movq	%r12, 16(%rsp)
	movq	496(%rsp), %rbx
	movq	32(%rax), %r12
	testq	%r12, %r12
	je	.LBB314_112
	movq	24(%rax), %r13
	shlq	$4, %r12
	addq	%r13, %r12
	.p2align	4
.LBB314_109:
	movq	(%r13), %rax
	lock		incq	(%rax)
	jle	.LBB314_289
	movq	8(%r13), %rdx
	movq	(%r13), %rsi
.Ltmp11152:
	movq	%rbp, %rdi
	vzeroupper
	callq	*%r15
.Ltmp11153:
	addq	$16, %r13
	cmpq	%r12, %r13
	jne	.LBB314_109
.LBB314_112:
	movq	64(%rsp), %rcx
	movq	(%r14), %r13
	movq	$-1, %rax
	addq	%rbx, %rcx
	leaq	-1(%r13), %rbx
	cmovbq	%rax, %rcx
	movq	%rcx, 64(%rsp)
	cmpq	$4, %rbx
	jbe	.LBB314_126
	movq	16(%r14), %r13
	movq	8(%r14), %r12
	leaq	-8(,%r13,8), %rbx
	leaq	-1(%r13), %rax
	cmpq	$5, %rax
	jb	.LBB314_127
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB314_287
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%rbx, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%rbx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB314_117
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB314_117:
	leaq	-1(%r13), %r8
	.p2align	4
.LBB314_118:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_124
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_118
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	%rbx, (%rdx)
	movq	%rbx, %rdx
	lock		xaddq	%rdx, (%rsi)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	addq	%rbx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB314_121:
	cmpq	%rax, %rdx
	jle	.LBB314_123
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB314_121
.LBB314_123:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_124:
	movb	$61, %al
	leaq	-2(%r13), %rsi
	bzhiq	%rax, %r8, %rax
	cmpq	%rsi, %rax
	cmovbq	%rax, %rsi
	cmpq	$16, %rsi
	jae	.LBB314_128
	movq	40(%rsp), %rbx
	movq	%r8, %rax
	movq	%r12, %rdx
	xorl	%esi, %esi
	jmp	.LBB314_130
.LBB314_126:
	leaq	8(%r14), %r12
	shlq	$3, %rbx
.LBB314_127:
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	320(%rsp), %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
	movq	16(%rsp), %r12
	movq	40(%rsp), %rbx
	jmp	.LBB314_134
.LBB314_128:
	incq	%rsi
	movl	$16, %edx
	movq	40(%rsp), %rbx
	movl	%esi, %eax
	andl	$15, %eax
	cmoveq	%rdx, %rax
	xorl	%edi, %edi
	subq	%rax, %rsi
	movq	%r8, %rax
	leaq	(%r12,%rsi,8), %rdx
	subq	%rsi, %rax
	.p2align	4
.LBB314_129:
	vmovdqu64	(%r12,%rdi,8), %zmm0
	vmovdqu64	64(%r12,%rdi,8), %zmm1
	vmovdqu64	%zmm1, 64(%rcx,%rdi,8)
	vmovdqu64	%zmm0, (%rcx,%rdi,8)
	addq	$16, %rdi
	cmpq	%rdi, %rsi
	jne	.LBB314_129
.LBB314_130:
	leaq	(%r12,%r8,8), %rdi
	leaq	4(%rcx,%rsi,8), %rsi
	xorl	%r8d, %r8d
	.p2align	4
.LBB314_131:
	cmpq	%rdi, %rdx
	je	.LBB314_133
	movl	(%rdx), %r9d
	movl	4(%rdx), %r10d
	addq	$8, %rdx
	movl	%r9d, -4(%rsi,%r8,8)
	movl	%r10d, (%rsi,%r8,8)
	incq	%r8
	cmpq	%r8, %rax
	jne	.LBB314_131
.LBB314_133:
	movq	16(%rsp), %r12
	movq	%rcx, 320(%rsp)
	movq	%r13, 328(%rsp)
.LBB314_134:
	vmovups	480(%rsp), %ymm0
	movq	320(%rsp), %rax
	vmovdqu	336(%rsp), %xmm1
	leaq	184(%rsp), %rcx
	vmovups	%ymm0, 32(%rcx)
	movq	%r13, 176(%rsp)
	movq	%rax, (%rcx)
	movq	328(%rsp), %rax
	movq	440(%rsp), %r13
	vmovdqu	%xmm1, 16(%rcx)
	movq	%rax, 8(%rcx)
	cmpq	416(%rsp), %rbx
	jne	.LBB314_137
.Ltmp11160:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::governor::soundness::NodeAnalysis>>::grow_one@GOTPCREL(%rip), %rax
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11161:
	movq	424(%rsp), %r12
.LBB314_137:
	movq	240(%rsp), %rcx
	leaq	(%rbx,%rbx,8), %rax
	incq	%rbx
	movq	%rcx, 64(%r12,%rax,8)
	vmovdqu64	176(%rsp), %zmm0
	vmovdqu64	%zmm0, (%r12,%rax,8)
	movq	%rbx, 432(%rsp)
	movq	624(%r13), %rax
	testq	%rax, %rax
	je	.LBB314_103
	testb	$1, 1200(%r13)
	je	.LBB314_103
	movq	1208(%r13), %rcx
	cmpq	40(%rax), %rcx
	jne	.LBB314_103
	movl	1216(%r13), %ecx
	subl	80(%rax), %ecx
	jb	.LBB314_103
	cmpq	%rcx, 32(%rax)
	jbe	.LBB314_103
	movq	24(%rax), %rax
	shlq	$4, %rcx
	cmpl	$1, (%rax,%rcx)
	jne	.LBB314_103
	movq	64(%rsp), %rdx
	cmpq	8(%rax,%rcx), %rdx
	jb	.LBB314_103
	jmp	.LBB314_154
.LBB314_144:
	movq	$0, 416(%rsp)
	movq	$8, 424(%rsp)
	movq	$0, 432(%rsp)
	movl	$8, %r12d
	xorl	%ebx, %ebx
	jmp	.LBB314_154
.LBB314_145:
	leaq	184(%rsp), %rax
	movq	72(%rsp), %r14
	vmovups	40(%rax), %zmm1
	vmovups	8(%rax), %zmm0
	movb	$1, %al
	movl	%eax, 16(%rsp)
	vmovups	%zmm1, 352(%rsp)
	vmovups	%zmm0, 320(%rsp)
	vmovdqu64	352(%rsp), %zmm1
	vmovdqu64	320(%rsp), %zmm0
	vmovdqu64	%zmm1, 48(%r14)
	vmovdqu64	%zmm0, 16(%r14)
	movq	$1, (%r14)
.Ltmp11166:
	movq	24(%rsp), %rbp
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11167:
	movb	$1, %bl
.Ltmp11168:
	leaq	576(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11169:
	lock		decq	(%rbp)
	jne	.LBB314_149
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp11170:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	callq	*%rax
.Ltmp11171:
.LBB314_149:
	cmpb	$0, 448(%rsp)
	jne	.LBB314_36
	movb	$1, %bpl
.Ltmp11172:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11173:
	jmp	.LBB314_36
.LBB314_151:
	vmovups	480(%rsp), %ymm0
	vmovups	96(%rsp), %ymm2
	vmovups	120(%rsp), %ymm1
	movq	%rax, 176(%rsp)
	vmovups	%ymm0, 184(%rsp)
	vmovups	%ymm2, 216(%rsp)
	vmovups	%ymm1, 240(%rsp)
.Ltmp11147:
	leaq	320(%rsp), %rdi
	leaq	632(%rsp), %rsi
	leaq	176(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11148:
	cmpq	$-1, 320(%rsp)
	je	.LBB314_154
.Ltmp11149:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11150:
.LBB314_154:
	movq	24(%rsp), %rbp
	movq	%rbx, 40(%rsp)
	movq	%r12, 16(%rsp)
	leaq	16(%rbp), %rsi
.Ltmp11174:
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp11175:
	movq	592(%rsp), %r14
	testq	%r14, %r14
	je	.LBB314_160
	movq	584(%rsp), %r15
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r12
	shlq	$4, %r14
	leaq	320(%rsp), %rbx
	addq	%r15, %r14
	.p2align	4
.LBB314_157:
	movq	(%r15), %rax
	lock		incq	(%rax)
	jle	.LBB314_289
	movq	8(%r15), %rdx
	movq	(%r15), %rsi
.Ltmp11177:
	movq	%rbx, %rdi
	callq	*%r12
.Ltmp11178:
	addq	$16, %r15
	cmpq	%r14, %r15
	jne	.LBB314_157
.LBB314_160:
	vmovdqu	344(%rsp), %ymm1
	vmovdqu	320(%rsp), %ymm0
	movl	$72, %edi
	movl	$8, %esi
	vmovdqu	%ymm1, 216(%rsp)
	vmovdqu	%ymm0, 192(%rsp)
	movq	$1, 176(%rsp)
	movq	$1, 184(%rsp)
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB314_281
	vmovdqu64	176(%rsp), %zmm0
	movq	%rax, %rcx
	movq	240(%rsp), %rax
	movq	%rcx, 568(%rsp)
	movq	40(%rsp), %rdi
	movq	%rcx, 32(%rsp)
	movq	%rax, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	32(%rbp), %rax
	movq	32(%rcx), %rsi
	movq	%rax, 64(%rsp)
	testq	%rsi, %rsi
	je	.LBB314_166
	movq	616(%r13), %rax
	testq	%rax, %rax
	je	.LBB314_166
	movq	32(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB314_166
	movq	%rax, %rdx
	orq	%rsi, %rdx
	movabsq	$230584300921369396, %rcx
	shrq	$32, %rdx
	je	.LBB314_274
	xorl	%edx, %edx
	divq	%rsi
	jmp	.LBB314_275
.LBB314_166:
	movq	16(%rsp), %r15
	movl	$0, 80(%rsp)
.LBB314_167:
	movq	%rdi, %r14
	movq	%rsi, 88(%rsp)
	testq	%r14, %r14
	je	.LBB314_171
.LBB314_169:
	leaq	(,%r14,8), %rax
	movl	$8, %esi
	leaq	(%rax,%rax,4), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB314_288
	movq	40(%rsp), %rdi
	jmp	.LBB314_172
.LBB314_274:
	xorl	%edx, %edx
	divl	%esi
.LBB314_275:
	movq	16(%rsp), %r15
	movq	%rax, 448(%rsp)
	cmpq	%rcx, %rax
	jae	.LBB314_277
	cmpq	%rdi, %rax
	movq	%rdi, %r14
	cmovbq	%rax, %r14
	movb	$1, %al
	movl	%eax, 80(%rsp)
	movq	%rsi, 88(%rsp)
	testq	%r14, %r14
	jne	.LBB314_169
.LBB314_171:
	movl	$8, %eax
.LBB314_172:
	movq	%r14, 152(%rsp)
	movq	72(%rsp), %r14
	movq	%rax, 160(%rsp)
	movq	$0, 168(%rsp)
	testq	%rdi, %rdi
	je	.LBB314_259
	movq	%rax, %rbx
	leaq	(%rdi,%rdi,8), %rax
	movq	32(%rsp), %rdx
	leaq	184(%rsp), %rcx
	xorl	%r13d, %r13d
	movq	%rcx, 512(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.449(%rip), %rcx
	leaq	(%r15,%rax,8), %rax
	movq	%rcx, 520(%rsp)
	movq	%rax, 552(%rsp)
	movq	64(%rsp), %rax
	addq	$16, %rdx
	movq	%rdx, 560(%rsp)
	leaq	(,%rax,8), %rax
	movq	%rax, 456(%rsp)
.LBB314_174:
	movq	64(%r15), %rsi
	addq	$16, %rsi
.Ltmp11186:
	movq	560(%rsp), %rdx
	movq	purrdf_sparql_eval::binop::right_to_out_map@GOTPCREL(%rip), %rax
	leaq	480(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11187:
	movq	56(%r15), %rax
	movq	%r15, 16(%rsp)
	movq	%rbx, %rdx
	testq	%rax, %rax
	je	.LBB314_231
	movq	48(%r15), %rbp
	movq	488(%rsp), %r12
	movq	496(%rsp), %r14
	leaq	(%rax,%rax,4), %rax
	leaq	8(%r15), %rcx
	movq	%rcx, 48(%rsp)
	leaq	(%rbp,%rax,8), %rax
	movq	%rax, 464(%rsp)
	jmp	.LBB314_179
	.p2align	4
.LBB314_177:
	movq	160(%rsp), %rdx
.LBB314_178:
	leaq	(%r13,%r13,4), %rax
	addq	$40, %rbp
	incq	%r13
	movq	%r15, (%rdx,%rax,8)
	movq	%rbx, 8(%rdx,%rax,8)
	vmovdqa	176(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	192(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%r13, 168(%rsp)
	cmpq	464(%rsp), %rbp
	je	.LBB314_231
.LBB314_179:
	movq	(%rbp), %rax
	leaq	8(%rbp), %rbx
	movq	%rdx, 40(%rsp)
	movq	%rbx, %rcx
	decq	%rax
	cmpq	$5, %rax
	jb	.LBB314_181
	movq	16(%rbp), %rax
	movq	8(%rbp), %rcx
	decq	%rax
.LBB314_181:
	testq	%rax, %rax
	je	.LBB314_200
	testq	%r14, %r14
	je	.LBB314_282
	movq	16(%rsp), %r8
	movl	(%rcx), %esi
	movl	4(%rcx), %edx
	shlq	$3, %rax
	movq	(%r8), %rdi
	decq	%rdi
	cmpq	$4, %rdi
	jbe	.LBB314_192
	movq	8(%r8), %rdi
	movq	16(%r8), %r8
	addq	$-8, %rax
	movq	%r14, %r9
	xorl	%r10d, %r10d
	decq	%r8
	.p2align	4
.LBB314_185:
	movq	(%r12,%r10), %r11
	cmpq	%r8, %r11
	jae	.LBB314_190
	cmpl	$2, %esi
	je	.LBB314_190
	movl	(%rdi,%r11,8), %r15d
	cmpl	$2, %r15d
	je	.LBB314_190
	cmpl	%r15d, %esi
	jne	.LBB314_200
	cmpl	4(%rdi,%r11,8), %edx
	jne	.LBB314_200
.LBB314_190:
	cmpq	%r10, %rax
	je	.LBB314_200
	movl	8(%rcx,%r10), %esi
	movl	12(%rcx,%r10), %edx
	addq	$8, %r10
	decq	%r9
	jne	.LBB314_185
	jmp	.LBB314_282
.LBB314_192:
	addq	$-8, %rax
	movq	%r14, %r8
	xorl	%r9d, %r9d
	.p2align	4
.LBB314_193:
	movq	(%r12,%r9), %r10
	cmpq	%rdi, %r10
	jae	.LBB314_198
	cmpl	$2, %esi
	je	.LBB314_198
	movq	48(%rsp), %r11
	movl	(%r11,%r10,8), %r11d
	cmpl	$2, %r11d
	je	.LBB314_198
	cmpl	%r11d, %esi
	jne	.LBB314_200
	movq	48(%rsp), %rsi
	cmpl	4(%rsi,%r10,8), %edx
	jne	.LBB314_200
.LBB314_198:
	cmpq	%r9, %rax
	je	.LBB314_200
	movl	8(%rcx,%r9), %esi
	movl	12(%rcx,%r9), %edx
	addq	$8, %r9
	decq	%r8
	jne	.LBB314_193
	jmp	.LBB314_282
	.p2align	4
.LBB314_200:
	cmpb	$0, 80(%rsp)
	je	.LBB314_202
	cmpq	448(%rsp), %r13
	jae	.LBB314_242
.LBB314_202:
	movq	88(%rsp), %rdx
	movq	$1, 176(%rsp)
	cmpq	$5, %rdx
	jae	.LBB314_229
	vmovdqu	184(%rsp), %xmm0
	movq	208(%rsp), %rax
	movq	176(%rsp), %rsi
	movq	200(%rsp), %rcx
	movq	%rax, 352(%rsp)
	movq	%rsi, 320(%rsp)
	movq	%rcx, 344(%rsp)
	vmovdqu	%xmm0, 328(%rsp)
	testq	%rdx, %rdx
	je	.LBB314_205
.LBB314_204:
	movl	$2, %eax
	jmp	.LBB314_206
	.p2align	4
.LBB314_205:
	movl	$-1, %eax
.LBB314_206:
	movl	%eax, 176(%rsp)
	movq	%rdx, 184(%rsp)
.Ltmp11197:
	leaq	320(%rsp), %rdi
	leaq	176(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp11198:
	vmovdqu	320(%rsp), %ymm0
	movq	352(%rsp), %rax
	movq	64(%rsp), %r8
	leaq	104(%rsp), %rdi
	movq	%rax, 128(%rsp)
	vmovdqu	%ymm0, 96(%rsp)
	movq	96(%rsp), %r15
	movq	%r15, %rdx
	cmpq	$6, %r15
	jb	.LBB314_209
	movq	104(%rsp), %rdi
	movq	112(%rsp), %rdx
.LBB314_209:
	decq	%rdx
	cmpq	%rdx, %r8
	ja	.LBB314_278
	movq	16(%rsp), %rcx
	movq	(%rcx), %rax
	movq	16(%rcx), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %r8
	jne	.LBB314_280
	movq	48(%rsp), %rsi
	cmpq	$5, %rax
	jb	.LBB314_213
	movq	48(%rsp), %rax
	movq	(%rax), %rsi
.LBB314_213:
	movq	456(%rsp), %rdx
	movq	memcpy@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	(%rbp), %rax
	decq	%rax
	cmpq	$5, %rax
	jb	.LBB314_215
	movq	16(%rbp), %rax
	movq	8(%rbp), %rbx
	decq	%rax
.LBB314_215:
	testq	%rax, %rax
	je	.LBB314_227
	shlq	$3, %rax
	leaq	104(%rsp), %r9
	xorl	%edi, %edi
	jmp	.LBB314_219
	.p2align	4
.LBB314_217:
	movl	4(%rbx,%rdi,8), %esi
	movl	%ecx, (%r8,%rdx,8)
	movl	%esi, 4(%r8,%rdx,8)
.LBB314_218:
	incq	%rdi
	addq	$-8, %rax
	je	.LBB314_226
.LBB314_219:
	movl	(%rbx,%rdi,8), %ecx
	cmpl	$2, %ecx
	je	.LBB314_218
	cmpq	%r14, %rdi
	jae	.LBB314_285
	movq	96(%rsp), %rsi
	movq	%rsi, %r8
	cmpq	$6, %rsi
	jb	.LBB314_223
	movq	112(%rsp), %r8
.LBB314_223:
	movq	(%r12,%rdi,8), %rdx
	decq	%r8
	cmpq	%r8, %rdx
	jae	.LBB314_284
	movq	%r9, %r8
	cmpq	$6, %rsi
	jb	.LBB314_217
	movq	104(%rsp), %r8
	jmp	.LBB314_217
	.p2align	4
.LBB314_226:
	movq	96(%rsp), %r15
.LBB314_227:
	leaq	104(%rsp), %rax
	movq	104(%rsp), %rbx
	movq	40(%rsp), %rdx
	vmovdqu	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 192(%rsp)
	vmovdqa	%xmm0, 176(%rsp)
	cmpq	152(%rsp), %r13
	jne	.LBB314_178
.Ltmp11204:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	152(%rsp), %rdi
	callq	*%rax
.Ltmp11205:
	jmp	.LBB314_177
.LBB314_229:
.Ltmp11194:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp11195:
	vmovdqu	176(%rsp), %ymm0
	movq	208(%rsp), %rax
	movq	88(%rsp), %rdx
	movq	%rax, 352(%rsp)
	vmovdqu	%ymm0, 320(%rsp)
	jmp	.LBB314_204
.LBB314_231:
	movq	480(%rsp), %rcx
	movq	72(%rsp), %r14
	movq	24(%rsp), %rbp
	movq	16(%rsp), %r15
	movq	%rdx, %rbx
	testq	%rcx, %rcx
	je	.LBB314_241
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	488(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB314_234
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB314_234:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_240
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_234
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
.LBB314_237:
	cmpq	%rax, %rdx
	jge	.LBB314_239
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB314_237
.LBB314_239:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_240:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB314_241:
	addq	$72, %r15
	cmpq	552(%rsp), %r15
	jne	.LBB314_174
	jmp	.LBB314_259
.LBB314_242:
	incq	%r13
	movq	%r13, %rax
	mulq	88(%rsp)
	jo	.LBB314_286
	movq	%rax, %rcx
.LBB314_244:
	movq	440(%rsp), %r8
	movq	72(%rsp), %r14
	movq	24(%rsp), %rbp
	movq	616(%r8), %rdx
	testq	%rdx, %rdx
	je	.LBB314_257
	cmpq	$-1, 32(%rdx)
	je	.LBB314_257
	cmpb	$0, 1238(%r8)
	je	.LBB314_252
	movq	632(%r8), %rax
	testq	%rax, %rax
	je	.LBB314_252
	movl	1228(%r8), %esi
	cmpq	%rsi, 104(%rax)
	jbe	.LBB314_252
	movq	96(%rax), %rdx
	movq	(%rdx,%rsi,8), %rax
.LBB314_250:
	cmpq	%rcx, %rax
	movq	%rcx, %rdi
	cmovaq	%rax, %rdi
	lock		cmpxchgq	%rdi, (%rdx,%rsi,8)
	jne	.LBB314_250
	movq	616(%r8), %rdx
.LBB314_252:
	movl	296(%rdx), %eax
	testl	%eax, %eax
	je	.LBB314_257
	movq	160(%rdx), %rax
	leaq	16(%rdx), %rsi
.LBB314_254:
	cmpq	%rcx, %rax
	movq	%rcx, %rdi
	cmovaq	%rax, %rdi
	lock		cmpxchgq	%rdi, 160(%rdx)
	jne	.LBB314_254
	movq	32(%rdx), %rax
	cmpq	%rax, %rcx
	jbe	.LBB314_257
	movq	%rax, 184(%rsp)
	movq	%rcx, 192(%rsp)
	movw	$512, 176(%rsp)
.Ltmp11191:
	leaq	320(%rsp), %rdi
	leaq	176(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp11192:
.LBB314_257:
	movq	480(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB314_259
	movq	488(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB314_259:
	vmovdqu64	672(%rsp), %zmm1
	vmovdqu64	632(%rsp), %zmm0
	movq	152(%rsp), %rcx
	movq	168(%rsp), %rax
	movq	160(%rsp), %rdx
	movq	%rcx, 96(%rsp)
	movq	32(%rsp), %rcx
	movq	%rax, 112(%rsp)
	movq	%rdx, 104(%rsp)
	vmovdqu64	%zmm1, 216(%rsp)
	vmovdqu64	%zmm0, 176(%rsp)
	cmpq	$-1, 176(%rsp)
	movq	%rcx, 120(%rsp)
	je	.LBB314_261
	leaq	320(%rsp), %rdi
	leaq	96(%rsp), %rsi
	leaq	632(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB314_262
.LBB314_261:
	movq	104(%rsp), %rcx
	movq	96(%rsp), %rax
	movq	112(%rsp), %rdx
	movq	%rcx, 336(%rsp)
	movq	120(%rsp), %rcx
	movq	%rax, 328(%rsp)
	movq	%rdx, 344(%rsp)
	movq	%rcx, 352(%rsp)
	movq	$-1, 320(%rsp)
.LBB314_262:
	movq	248(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB314_264
	movq	256(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB314_264:
	movq	272(%rsp), %rax
	testq	%rax, %rax
	je	.LBB314_267
	lock		decq	(%rax)
	jne	.LBB314_267
	leaq	272(%rsp), %rdi
	#MEMBARRIER
.Ltmp11212:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11213:
.LBB314_267:
	vmovdqu64	352(%rsp), %zmm1
	vmovdqu64	320(%rsp), %zmm0
	movl	$0, 16(%rsp)
	vmovdqu64	%zmm1, 40(%r14)
	vmovdqu64	%zmm0, 8(%r14)
	movq	$0, (%r14)
.Ltmp11215:
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11216:
	xorl	%ebx, %ebx
.Ltmp11217:
	leaq	576(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11218:
	lock		decq	(%rbp)
	jne	.LBB314_271
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp11220:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	callq	*%rax
.Ltmp11221:
.LBB314_271:
	cmpq	$0, 528(%rsp)
	je	.LBB314_273
	xorl	%ebp, %ebp
.Ltmp11222:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11223:
.LBB314_273:
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB314_75
.LBB314_277:
	movl	$0, 80(%rsp)
	jmp	.LBB314_167
.LBB314_278:
.Ltmp11207:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.451(%rip), %rcx
	xorl	%edi, %edi
	movq	%r8, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11208:
	jmp	.LBB314_289
.LBB314_279:
	movb	$1, %bpl
.Ltmp11240:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.443(%rip), %rsi
	leaq	176(%rsp), %rdi
	movl	$84, %edx
	callq	<purrdf_sparql_eval::error::EvalError>::internal::<&str>
.Ltmp11241:
	jmp	.LBB314_35
.LBB314_280:
.Ltmp11200:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.448(%rip), %rdx
	movq	%r8, %rdi
	vzeroupper
	callq	*%rax
.Ltmp11201:
	jmp	.LBB314_289
.LBB314_281:
.Ltmp11225:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	192(%rsp), %rbx
	callq	*%rax
.Ltmp11226:
	jmp	.LBB314_289
.LBB314_282:
.Ltmp11189:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	24(%rsp), %rbp
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.710(%rip), %rdx
	movq	%r14, %rdi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11190:
	jmp	.LBB314_289
.LBB314_283:
	movb	$1, %bpl
.Ltmp11245:
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	anon.e5162873a9a3251d11c4df37a70e4654.2.llvm.12908414067662811932(%rip), %rdi
	leaq	anon.e5162873a9a3251d11c4df37a70e4654.4.llvm.12908414067662811932(%rip), %rdx
	movl	$48, %esi
	callq	*%rax
.Ltmp11246:
	jmp	.LBB314_289
.LBB314_284:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.450(%rip), %rax
	movq	%rdx, %rdi
	movq	%r8, %r14
	movq	%rax, 520(%rsp)
.LBB314_285:
.Ltmp11202:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	520(%rsp), %rdx
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11203:
	jmp	.LBB314_289
.LBB314_286:
	movq	$-1, %rcx
	jmp	.LBB314_244
.LBB314_287:
.Ltmp11155:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp11156:
	jmp	.LBB314_289
.LBB314_288:
.Ltmp11183:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp11184:
.LBB314_289:
	ud2
.LBB314_290:
.Ltmp11196:
	movq	%rax, %r14
	movq	176(%rsp), %rax
	movq	24(%rsp), %rbp
	movq	32(%rsp), %rbx
	cmpq	$6, %rax
	jae	.LBB314_310
	jmp	.LBB314_323
.LBB314_291:
.Ltmp11214:
	movq	%rax, %r14
	movl	$0, 16(%rsp)
	jmp	.LBB314_328
.LBB314_292:
.Ltmp11151:
	movq	%rax, %r14
	jmp	.LBB314_318
.LBB314_293:
.Ltmp11185:
	movq	32(%rsp), %rbx
	movq	%rax, %r14
	jmp	.LBB314_326
.LBB314_294:
.Ltmp11176:
	movq	%rax, %r14
	movb	$1, %al
	movl	%eax, 16(%rsp)
	jmp	.LBB314_328
.LBB314_295:
.Ltmp11219:
	movq	%rax, %r14
	movl	%ebx, 16(%rsp)
	jmp	.LBB314_331
.LBB314_296:
.Ltmp11157:
	jmp	.LBB314_313
.LBB314_297:
.Ltmp11244:
	leaq	288(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB314_341
.LBB314_298:
.Ltmp11188:
	movq	32(%rsp), %rbx
	movq	%rax, %r14
	jmp	.LBB314_325
.LBB314_299:
.Ltmp11141:
	leaq	288(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB314_300:
.Ltmp11162:
	movq	%rax, %r14
.Ltmp11163:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>
.Ltmp11164:
	jmp	.LBB314_318
.LBB314_301:
.Ltmp11165:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB314_302:
.Ltmp11224:
	movq	%rax, %r14
	movl	%ebp, 16(%rsp)
	jmp	.LBB314_335
.LBB314_303:
.Ltmp11146:
	movq	%rax, %r14
	jmp	.LBB314_318
.LBB314_304:
.Ltmp11206:
	movq	%rax, %r14
	cmpq	$6, %r15
	jb	.LBB314_306
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB314_306:
	movq	24(%rsp), %rbp
	jmp	.LBB314_316
.LBB314_307:
.Ltmp11179:
	movq	%rax, %r14
.Ltmp11180:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11181:
	jmp	.LBB314_318
.LBB314_308:
.Ltmp11182:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB314_309:
.Ltmp11199:
	movq	%rax, %r14
	movq	320(%rsp), %rax
	movq	24(%rsp), %rbp
	movq	32(%rsp), %rbx
	leaq	328(%rsp), %rcx
	movq	%rcx, 512(%rsp)
	cmpq	$5, %rax
	jbe	.LBB314_323
.LBB314_310:
	movq	512(%rsp), %rcx
	movq	(%rcx), %rdi
	jmp	.LBB314_322
.LBB314_311:
.Ltmp11138:
	movq	%rax, %rbx
	jmp	.LBB314_341
.LBB314_312:
.Ltmp11154:
.LBB314_313:
	movq	%rax, %r14
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp11158:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11159:
	movq	24(%rsp), %rbp
	jmp	.LBB314_328
.LBB314_314:
.Ltmp11247:
	movq	%rax, %r14
	movl	%ebp, 16(%rsp)
	jmp	.LBB314_333
.LBB314_315:
.Ltmp11193:
	movq	%rax, %r14
.LBB314_316:
	movq	32(%rsp), %rbx
	jmp	.LBB314_323
.LBB314_317:
.Ltmp11227:
	movq	%rax, %r14
.Ltmp11228:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11229:
.LBB314_318:
	movq	24(%rsp), %rbp
	movb	$1, %al
	movl	%eax, 16(%rsp)
	jmp	.LBB314_328
.LBB314_319:
.Ltmp11230:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB314_320:
.Ltmp11209:
	movq	%rax, %r14
	movq	96(%rsp), %rax
	movq	24(%rsp), %rbp
	movq	32(%rsp), %rbx
	cmpq	$6, %rax
	jb	.LBB314_323
	movq	104(%rsp), %rdi
.LBB314_322:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB314_323:
	movq	480(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB314_325
	movq	488(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB314_325:
	leaq	152(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB314_326:
	lock		decq	(%rbx)
	movb	$1, %al
	movl	%eax, 16(%rsp)
	jne	.LBB314_328
	#MEMBARRIER
.Ltmp11210:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	callq	*%rax
.Ltmp11211:
.LBB314_328:
.Ltmp11231:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11232:
	jmp	.LBB314_330
.LBB314_329:
.Ltmp11235:
	movq	%rax, %r14
.LBB314_330:
.Ltmp11236:
	leaq	576(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11237:
.LBB314_331:
	lock		decq	(%rbp)
	jne	.LBB314_333
	#MEMBARRIER
.Ltmp11238:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	callq	*%rax
.Ltmp11239:
.LBB314_333:
	cmpq	$0, 528(%rsp)
	je	.LBB314_335
.Ltmp11248:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11249:
.LBB314_335:
	movq	312(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB314_337
	leaq	312(%rsp), %rdi
	#MEMBARRIER
.Ltmp11250:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp11251:
.LBB314_337:
	movq	%r14, 48(%rsp)
	movq	296(%rsp), %r14
	movq	304(%rsp), %rbx
	testq	%rbx, %rbx
	jne	.LBB314_343
.LBB314_338:
	movq	288(%rsp), %rax
	testq	%rax, %rax
	je	.LBB314_340
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB314_340:
	cmpb	$0, 16(%rsp)
	movq	48(%rsp), %rbx
	je	.LBB314_342
.LBB314_341:
.Ltmp11252:
	leaq	632(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp11253:
.LBB314_342:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB314_343:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r12
	movabsq	$9223372036854775807, %r13
	xorl	%r15d, %r15d
	jmp	.LBB314_347
	.p2align	4
.LBB314_344:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB314_345:
	callq	*%r12
.LBB314_346:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB314_338
.LBB314_347:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB314_346
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r13, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r13, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r13, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB314_350
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB314_350:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB314_345
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB314_350
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r13, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB314_353:
	cmpq	%rax, %rdx
	jge	.LBB314_344
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB314_353
	jmp	.LBB314_344
.LBB314_355:
.Ltmp11254:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end314:
purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin315:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception222
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
	subq	$1464, %rsp
	.cfi_def_cfa_offset 1520
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, 56(%rsp)
	leaq	888(%rsp), %rdi
	movq	%r8, %r12
	movq	%rcx, %r13
	movq	%rdx, %r14
	callq	*%rax
	movb	$1, %bpl
.Ltmp11255:
	leaq	1344(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r12, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp11256:
	cmpl	$1, 1344(%rsp)
	jne	.LBB315_3
	vmovups	1360(%rsp), %zmm0
	vmovups	1392(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
	jmp	.LBB315_95
.LBB315_3:
	vmovups	1384(%rsp), %zmm1
	vmovups	1352(%rsp), %zmm0
	vmovups	%zmm1, 96(%rsp)
	vmovups	%zmm0, 64(%rsp)
.Ltmp11257:
	leaq	224(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	64(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11258:
	cmpq	$-1, 224(%rsp)
	je	.LBB315_9
	vmovups	224(%rsp), %ymm0
	cmpq	$-1, 888(%rsp)
	vmovups	%ymm0, 480(%rsp)
	je	.LBB315_11
	vmovups	928(%rsp), %zmm1
	vmovups	888(%rsp), %zmm0
	movq	504(%rsp), %rax
	movq	%rax, 552(%rsp)
	movq	$0, 528(%rsp)
	movq	$8, 536(%rsp)
	movq	$0, 544(%rsp)
	vmovups	%zmm1, 104(%rsp)
	vmovups	%zmm0, 64(%rsp)
	cmpq	$-1, 64(%rsp)
	je	.LBB315_18
	leaq	224(%rsp), %rdi
	leaq	528(%rsp), %rsi
	leaq	888(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	136(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB315_8
.LBB315_19:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	144(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB315_21
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_21:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_27
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_21
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB315_24:
	cmpq	%rax, %rsi
	jge	.LBB315_26
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_24
.LBB315_26:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_27:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	160(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB315_28
	jmp	.LBB315_30
.LBB315_9:
	xorl	%ebp, %ebp
.Ltmp11475:
	leaq	64(%rsp), %rdi
	leaq	888(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp11476:
	vmovups	64(%rsp), %zmm0
	vmovups	96(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB315_118
.LBB315_11:
	cmpl	$28, (%r13)
	movq	%r12, 40(%rsp)
	jne	.LBB315_31
	movq	624(%r12), %rax
	xorl	%r14d, %r14d
	testq	%rax, %rax
	je	.LBB315_43
	testb	$1, 1200(%r12)
	je	.LBB315_44
	movq	1208(%r12), %rcx
	cmpq	40(%rax), %rcx
	jne	.LBB315_37
	movl	1216(%r12), %ecx
	subl	80(%rax), %ecx
	jb	.LBB315_37
	cmpq	%rcx, 32(%rax)
	jbe	.LBB315_37
	movq	24(%rax), %rax
	shlq	$4, %rcx
	movq	(%rax,%rcx), %r14
	movq	8(%rax,%rcx), %r15
	jmp	.LBB315_44
.LBB315_18:
	movq	536(%rsp), %rcx
	movq	528(%rsp), %rax
	movq	544(%rsp), %rdx
	movq	%rcx, 240(%rsp)
	movq	552(%rsp), %rcx
	movq	%rax, 232(%rsp)
	movq	%rdx, 248(%rsp)
	movq	%rcx, 256(%rsp)
	movq	$-1, 224(%rsp)
	movq	136(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB315_19
.LBB315_8:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB315_30
.LBB315_28:
	lock		decq	(%rax)
	jne	.LBB315_30
	leaq	160(%rsp), %rdi
	#MEMBARRIER
.Ltmp11259:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11260:
.LBB315_30:
	vmovups	224(%rsp), %zmm0
	vmovups	256(%rsp), %zmm1
	movq	56(%rsp), %rax
	movl	$0, 48(%rsp)
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB315_71
.LBB315_31:
	movb	$1, %bpl
.Ltmp11262:
	leaq	64(%rsp), %rdi
	leaq	480(%rsp), %rsi
	movq	%r13, 216(%rsp)
	movq	%r13, %rdx
	movq	%r12, %rcx
	vzeroupper
	callq	purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11263:
	cmpq	$-1, 64(%rsp)
	jne	.LBB315_42
	movq	40(%rsp), %rax
	movq	216(%rsp), %rbx
	movq	584(%rax), %rsi
	testq	%rsi, %rsi
	je	.LBB315_38
	movq	%rbx, %rdi
	callq	purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#0}
	testq	%rax, %rax
	je	.LBB315_38
	cmpl	$1, (%rax)
	jne	.LBB315_38
	addq	$8, %rax
	leaq	432(%rsp), %rdi
	movq	%rax, %rsi
	callq	<purrdf_sparql_eval::deferred_exists::DeferredLateral as core::clone::Clone>::clone
	cmpq	$0, 432(%rsp)
	jne	.LBB315_59
	jmp	.LBB315_39
.LBB315_37:
	xorl	%r14d, %r14d
.LBB315_43:
.LBB315_44:
	movb	$1, %bpl
.Ltmp11453:
	leaq	736(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_node
.Ltmp11454:
.Ltmp11455:
	leaq	1080(%rsp), %rdi
	movq	%r12, %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge
.Ltmp11456:
	cmpb	$-1, 1080(%rsp)
	leaq	1200(%r12), %rbx
	je	.LBB315_54
.Ltmp11457:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
.Ltmp11458:
	vmovups	1080(%rsp), %xmm0
	movq	%rax, 552(%rsp)
	movq	1096(%rsp), %rax
	leaq	64(%rsp), %rdi
	leaq	528(%rsp), %rsi
	leaq	224(%rsp), %rdx
	movq	$0, 528(%rsp)
	movq	$8, 536(%rsp)
	movq	$0, 224(%rsp)
	movq	$1, 232(%rsp)
	movq	$0, 240(%rsp)
	movq	$0, 544(%rsp)
	movq	%rax, 264(%rsp)
	vmovups	%xmm0, 248(%rsp)
	movq	$0, 272(%rsp)
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	96(%rsp), %rcx
	vmovups	80(%rsp), %xmm0
	movq	64(%rsp), %rax
	movq	72(%rsp), %rsi
	movq	104(%rsp), %rdx
	movq	%rcx, 720(%rsp)
	movzbl	112(%rsp), %ecx
.LBB315_49:
	vmovaps	%xmm0, 704(%rsp)
	vmovups	113(%rsp), %xmm0
	movq	128(%rsp), %rdi
	vmovups	136(%rsp), %xmm1
	vmovups	736(%rsp), %xmm2
	movq	%rax, 1248(%rsp)
	movq	720(%rsp), %rax
	movq	%rsi, 1256(%rsp)
	movq	40(%rsp), %r8
	movq	%rax, 1280(%rsp)
	vmovaps	%xmm0, 640(%rsp)
	movq	%rdi, 655(%rsp)
	movq	152(%rsp), %rdi
	vmovaps	%xmm1, 672(%rsp)
	vmovaps	704(%rsp), %xmm1
	vmovaps	%xmm2, (%rbx)
	vmovaps	640(%rsp), %xmm2
	movq	655(%rsp), %rax
	movq	%rdi, 688(%rsp)
	movq	752(%rsp), %rdi
	vmovups	%xmm1, 1264(%rsp)
	vmovaps	672(%rsp), %xmm1
	movq	%rdx, 1288(%rsp)
	movb	%cl, 1296(%rsp)
	vmovups	%xmm2, 1297(%rsp)
	movq	%rax, 1312(%rsp)
	movq	688(%rsp), %rax
	movq	%rdi, 16(%rbx)
	movl	760(%rsp), %edi
	movq	%rax, 1336(%rsp)
	movl	%edi, 1228(%r8)
	movzbl	764(%rsp), %edi
	vmovups	%xmm1, 1320(%rsp)
	movb	%dil, 1238(%r8)
.Ltmp11462:
	leaq	1216(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	1248(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11463:
	cmpq	$-1, 1216(%rsp)
	je	.LBB315_58
	vmovups	888(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	%zmm0, 64(%rsp)
	vmovups	%zmm1, 104(%rsp)
	cmpq	$-1, 64(%rsp)
	je	.LBB315_63
	leaq	224(%rsp), %rdi
	leaq	1216(%rsp), %rsi
	leaq	888(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB315_64
.LBB315_54:
	addq	$8, %r13
.Ltmp11459:
	leaq	224(%rsp), %rdi
	leaq	480(%rsp), %rdx
	movq	%r13, %rsi
	movq	%r14, %rcx
	movq	%r15, %r8
	movq	%r12, %r9
	callq	purrdf_sparql_eval::property_fn_eval::eval_call_over::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11460:
	vmovups	232(%rsp), %ymm0
	vmovups	265(%rsp), %xmm1
	movq	224(%rsp), %rcx
	movzbl	264(%rsp), %eax
	movq	280(%rsp), %rdx
	vmovaps	%xmm1, 800(%rsp)
	vmovups	%ymm0, 528(%rsp)
	movq	%rdx, 815(%rsp)
	cmpq	$-1, %rcx
	je	.LBB315_119
	vmovaps	288(%rsp), %xmm0
	movq	544(%rsp), %rsi
	movq	752(%rsp), %r11
	movq	815(%rsp), %rdi
	movl	760(%rsp), %r10d
	movq	40(%rsp), %r8
	movzbl	764(%rsp), %r9d
	movq	304(%rsp), %rdx
	movq	%rsi, 720(%rsp)
	movq	552(%rsp), %rsi
	movq	%r11, 16(%rbx)
	movq	%rdx, 688(%rsp)
	movq	312(%rsp), %rdx
	vmovaps	%xmm0, 672(%rsp)
	vmovaps	528(%rsp), %xmm0
	vmovaps	672(%rsp), %xmm1
	vmovaps	%xmm0, 704(%rsp)
	vmovaps	800(%rsp), %xmm0
	vmovaps	%xmm0, 640(%rsp)
	vmovups	736(%rsp), %xmm0
	movq	%rdi, 655(%rsp)
	movq	720(%rsp), %rdi
	vmovaps	640(%rsp), %xmm2
	vmovaps	%xmm0, (%rbx)
	vmovaps	704(%rsp), %xmm0
	movl	%r10d, 1228(%r8)
	movb	%r9b, 1238(%r8)
	movq	56(%rsp), %r9
	movq	%rdi, 40(%r9)
	vmovups	%xmm0, 24(%r9)
	movq	%rsi, 48(%r9)
	movq	655(%rsp), %rsi
	vmovups	%xmm2, 57(%r9)
	movq	%rsi, 72(%r9)
	movq	688(%rsp), %rsi
	vmovaps	%xmm1, 80(%r9)
	movq	%rsi, 96(%r9)
	movq	%rcx, 16(%r9)
	movb	%al, 56(%r9)
	movq	%rdx, 104(%r9)
	movq	$1, (%r9)
	jmp	.LBB315_57
.LBB315_38:
	movq	$0, 432(%rsp)
.LBB315_39:
	movb	$1, %bpl
.Ltmp11264:
	movq	purrdf_sparql_eval::deferred_exists::is_lateral_placeholder@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp11265:
	testb	%al, %al
	je	.LBB315_59
.Ltmp11448:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.443(%rip), %rsi
	leaq	64(%rsp), %rdi
	movl	$84, %edx
	callq	<purrdf_sparql_eval::error::EvalError>::internal::<&str>
.Ltmp11449:
.LBB315_42:
	vmovups	64(%rsp), %zmm0
	vmovups	96(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.LBB315_57:
	movb	$1, %bpl
	movq	504(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 48(%rsp)
	je	.LBB315_70
	jmp	.LBB315_71
.LBB315_58:
.Ltmp11467:
	leaq	224(%rsp), %rdi
	leaq	888(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp11468:
	jmp	.LBB315_69
.LBB315_59:
	movq	504(%rsp), %r15
	lock		incq	(%r15)
	jle	.LBB315_347
	vmovups	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	496(%rsp), %rbx
	movabsq	$128102389400760776, %rax
	movq	$0, 800(%rsp)
	movq	$8, 808(%rsp)
	movq	$0, 816(%rsp)
	movq	%r15, 520(%rsp)
	decq	%rax
	vmovups	%ymm0, 824(%rsp)
	cmpq	%rax, %rbx
	jbe	.LBB315_121
	xorl	%r13d, %r13d
.LBB315_62:
	movb	$1, %bpl
.Ltmp11441:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11442:
	jmp	.LBB315_347
.LBB315_63:
	vmovups	1216(%rsp), %ymm0
	vmovups	%ymm0, 232(%rsp)
	movq	$-1, 224(%rsp)
.LBB315_64:
	movq	136(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB315_66
	movq	144(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB315_66:
	movq	160(%rsp), %rax
	testq	%rax, %rax
	je	.LBB315_69
	lock		decq	(%rax)
	jne	.LBB315_69
	leaq	160(%rsp), %rdi
	#MEMBARRIER
.Ltmp11465:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11466:
.LBB315_69:
	vmovups	224(%rsp), %zmm0
	vmovups	256(%rsp), %zmm1
	movq	56(%rsp), %rax
	xorl	%ebp, %ebp
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
	movq	504(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 48(%rsp)
	jne	.LBB315_71
.LBB315_70:
	leaq	504(%rsp), %rdi
	#MEMBARRIER
.Ltmp11472:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11473:
.LBB315_71:
	movq	488(%rsp), %r14
	movq	496(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB315_84
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB315_76
	.p2align	4
.LBB315_73:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_74:
	vzeroupper
	callq	*%r13
.LBB315_75:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB315_84
.LBB315_76:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB315_75
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB315_79
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_79:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_74
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_79
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r12, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB315_82:
	cmpq	%rax, %rdx
	jge	.LBB315_73
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB315_82
	jmp	.LBB315_73
.LBB315_84:
	movq	480(%rsp), %rax
	movl	48(%rsp), %ebx
	testq	%rax, %rax
	je	.LBB315_94
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB315_87
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_87:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_93
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_87
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB315_90:
	cmpq	%rax, %rsi
	jge	.LBB315_92
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_90
.LBB315_92:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_93:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB315_94:
	testb	%bl, %bl
	je	.LBB315_118
.LBB315_95:
	movq	960(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB315_105
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	968(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB315_98
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_98:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_104
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_98
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB315_101:
	cmpq	%rax, %rsi
	jge	.LBB315_103
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_101
.LBB315_103:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_104:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB315_105:
	movq	888(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB315_115
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	896(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB315_108
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_108:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_114
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_108
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB315_111:
	cmpq	%rax, %rsi
	jge	.LBB315_113
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB315_111
.LBB315_113:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_114:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB315_115:
	movq	984(%rsp), %rax
	testq	%rax, %rax
	je	.LBB315_118
	lock		decq	(%rax)
	jne	.LBB315_118
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	984(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB315_118:
	movq	56(%rsp), %rax
	addq	$1464, %rsp
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
	vzeroupper
	retq
.LBB315_119:
	.cfi_def_cfa_offset 1520
	cmpb	$-1, %al
	je	.LBB315_240
	vmovaps	800(%rsp), %xmm0
	movq	815(%rsp), %rcx
	leaq	64(%rsp), %rdi
	leaq	528(%rsp), %rsi
	leaq	224(%rsp), %rdx
	movq	%rcx, 264(%rsp)
	vmovups	%xmm0, 249(%rsp)
	movq	$0, 224(%rsp)
	movq	$1, 232(%rsp)
	movq	$0, 240(%rsp)
	movb	%al, 248(%rsp)
	movq	$0, 272(%rsp)
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	64(%rsp), %rax
	movq	104(%rsp), %rdx
	movzbl	112(%rsp), %ecx
	jmp	.LBB315_241
.LBB315_121:
	movq	%r15, 16(%rsp)
	testq	%rbx, %rbx
	je	.LBB315_242
	leaq	(,%rbx,8), %rax
	movl	$8, %esi
	movl	$8, %r13d
	leaq	(%rax,%rax,8), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB315_62
	movq	488(%rsp), %rcx
	movq	%rbx, 456(%rsp)
	movq	%rax, 464(%rsp)
	movq	%rax, 208(%rsp)
	leaq	(%rbx,%rbx,4), %rax
	movq	40(%rsp), %rbx
	movq	$0, 472(%rsp)
	movq	$0, 32(%rsp)
	leaq	(%rcx,%rax,8), %rax
	movq	%rcx, 48(%rsp)
	leaq	1096(%rbx), %rdx
	leaq	16(%r15), %rcx
	movq	%rdx, 416(%rsp)
	movq	%rcx, 512(%rsp)
	movq	%rax, 344(%rsp)
	jmp	.LBB315_125
.LBB315_124:
	movq	32(%rsp), %rsi
	movq	128(%rsp), %rcx
	movq	208(%rsp), %rdx
	movq	48(%rsp), %rdi
	leaq	(%rsi,%rsi,8), %rax
	addq	$40, %rdi
	incq	%rsi
	movq	%rsi, 32(%rsp)
	movq	%rdi, 48(%rsp)
	movq	%rcx, 64(%rdx,%rax,8)
	vmovups	64(%rsp), %zmm0
	vmovups	%zmm0, (%rdx,%rax,8)
	movq	%rsi, 472(%rsp)
	cmpq	344(%rsp), %rdi
	je	.LBB315_246
.LBB315_125:
	movq	432(%rsp), %r12
	testq	%r12, %r12
	je	.LBB315_128
	movq	48(%rsp), %rax
	movq	(%rax), %r14
	decq	%r14
	cmpq	$4, %r14
	jbe	.LBB315_130
	movq	16(%rax), %r14
	movq	8(%rax), %r13
	decq	%r14
	jmp	.LBB315_131
.LBB315_128:
	movzbl	1237(%rbx), %eax
	incq	%rax
	movq	%rax, 528(%rsp)
	movq	48(%rsp), %rax
	movq	$0, 544(%rsp)
	movq	(%rax), %rcx
	decq	%rcx
	cmpq	$4, %rcx
	jbe	.LBB315_140
	movq	16(%rax), %rcx
	movq	8(%rax), %rdx
	decq	%rcx
	jmp	.LBB315_141
.LBB315_130:
	leaq	8(%rax), %r13
.LBB315_131:
	leaq	368(%rsp), %rax
	movb	$0, 368(%rsp)
	movq	%rax, 64(%rsp)
	leaq	64(%rsp), %rax
	#APP
	#NO_APP
	movq	purrdf_stack::FLOOR::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	64(%rsp), %rdi
	movq	%fs:(%rax), %rcx
	movq	%rdi, %rax
	subq	%rcx, %rax
	cmpq	$131072, %rax
	setb	%al
	cmpq	%rcx, %rdi
	jb	.LBB315_236
	testb	%al, %al
	jne	.LBB315_237
.LBB315_133:
	movq	16(%rsp), %rax
	movq	24(%rax), %rcx
	movq	32(%rax), %r8
.Ltmp11268:
	leaq	64(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r14, %rdx
	movq	%rbx, %r9
	vzeroupper
	callq	purrdf_sparql_eval::expr::outer_bindings_for_substitution::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11269:
	leaq	72(%rsp), %rdx
	movq	64(%rsp), %rax
	vmovups	(%rdx), %ymm0
	vmovups	16(%rdx), %ymm1
	vmovups	%ymm0, 368(%rsp)
	vmovups	%ymm1, 384(%rsp)
	cmpq	$-1, %rax
	je	.LBB315_137
	vmovups	48(%rdx), %ymm0
	vmovups	368(%rsp), %ymm2
	vmovups	384(%rsp), %ymm1
	movq	80(%rdx), %rcx
	leaq	240(%rsp), %rsi
	movq	%rcx, 88(%rsi)
	vmovups	%ymm0, 56(%rsi)
	vmovups	%ymm1, 24(%rsi)
	vmovups	%ymm2, 8(%rsi)
	movq	%rax, 240(%rsp)
.LBB315_136:
	movq	$1, 224(%rsp)
	jmp	.LBB315_199
.LBB315_137:
	vmovups	368(%rsp), %ymm0
	vmovups	384(%rsp), %ymm1
	movq	448(%rsp), %rsi
	movq	440(%rsp), %rdx
	leaq	16(%r12), %rax
	vmovups	%ymm0, 1024(%rsp)
	vmovups	%ymm1, 1040(%rsp)
	movq	80(%r12), %rcx
	movq	$3, 864(%rsp)
	movq	%rax, 872(%rsp)
	movl	$0, %eax
	addq	$16, %rcx
	cmpq	$1, %rsi
	adcq	$1, %rax
	testq	%rdx, %rdx
	movq	%rcx, 880(%rsp)
	cmoveq	%rdx, %rax
	testq	%rax, %rax
	je	.LBB315_144
	cmpq	$1, %rax
	jne	.LBB315_151
	addq	$16, %rsi
	leaq	48(%r12), %rcx
.Ltmp11270:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	1104(%rsp), %rdi
	leaq	1024(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11271:
	jmp	.LBB315_146
.LBB315_140:
	leaq	8(%rax), %rdx
.LBB315_141:
.Ltmp11357:
	movq	216(%rsp), %rsi
	movq	512(%rsp), %r8
	leaq	64(%rsp), %rdi
	leaq	528(%rsp), %r9
	movq	%rbx, (%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_correlated::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11358:
	cmpl	$1, 64(%rsp)
	je	.LBB315_328
	leaq	72(%rsp), %rcx
	movq	72(%rsp), %rax
	movq	16(%rsp), %r15
	vmovups	32(%rcx), %zmm1
	vmovups	8(%rcx), %zmm0
	leaq	240(%rsp), %rcx
	vmovups	%zmm1, 248(%rsp)
	vmovups	%zmm0, 224(%rsp)
	vmovups	224(%rsp), %ymm0
	vmovups	40(%rcx), %ymm1
	vmovups	16(%rcx), %ymm2
	jmp	.LBB315_201
.LBB315_144:
	leaq	72(%rsp), %rax
	movq	$0, 64(%rsp)
	movq	$8, 72(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	leaq	48(%r12), %rcx
	vmovups	%xmm0, 8(%rax)
	movq	$8, 96(%rsp)
	movq	$0, 104(%rsp)
.Ltmp11272:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	64(%rsp), %r14
	leaq	1104(%rsp), %rdi
	leaq	1024(%rsp), %rdx
	movq	%r14, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11273:
.Ltmp11277:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11278:
.LBB315_146:
	cmpq	$-1, 1104(%rsp)
	je	.LBB315_152
	vmovups	1120(%rsp), %ymm1
	vmovups	1104(%rsp), %ymm0
	cmpq	$0, 632(%rbx)
	vmovups	%ymm1, 384(%rsp)
	vmovups	%ymm0, 368(%rsp)
	je	.LBB315_177
	movq	80(%r12), %rax
	cmpq	$0, 40(%rax)
	je	.LBB315_177
	lock		incq	(%rax)
	jle	.LBB315_347
	movq	80(%r12), %rsi
	movb	$1, %bl
.Ltmp11279:
	movq	416(%rsp), %rdi
	vzeroupper
	callq	<alloc::vec::Vec<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>>::push_mut
.Ltmp11280:
	jmp	.LBB315_178
.LBB315_151:
	movq	$-1, 1104(%rsp)
.LBB315_152:
	movq	40(%r12), %rdi
.Ltmp11288:
	leaq	864(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::deferred_exists::nested_sites::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11289:
	movq	%rax, 624(%rsp)
	movq	%rdx, 184(%rsp)
	movq	%rdx, 632(%rsp)
	leaq	48(%r12), %rdx
	movq	%rax, 192(%rsp)
	movq	$0, 352(%rsp)
	movq	$0, 360(%rsp)
.Ltmp11290:
	movq	<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::then@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	leaq	1024(%rsp), %rsi
	xorl	%ebx, %ebx
	callq	*%rax
.Ltmp11291:
	movq	%rax, 368(%rsp)
	movq	%rdx, 376(%rsp)
.Ltmp11292:
	movq	<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::layers@GOTPCREL(%rip), %rax
	leaq	64(%rsp), %rdi
	leaq	368(%rsp), %rsi
	callq	*%rax
.Ltmp11293:
	movq	64(%rsp), %rax
	movq	%r12, 200(%rsp)
	movq	72(%rsp), %r12
	movq	%rax, 176(%rsp)
	movq	80(%rsp), %rax
	movq	%r12, 24(%rsp)
	testq	%rax, %rax
	je	.LBB315_185
	movq	192(%rsp), %r15
	leaq	(%r12,%rax,8), %rax
	xorl	%ebp, %ebp
	xorl	%r14d, %r14d
	movq	%rax, 424(%rsp)
	shlq	$4, %r15
	addq	184(%rsp), %r15
	jmp	.LBB315_160
.LBB315_157:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB315_158:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	callq	*%rax
.LBB315_159:
	addq	$8, %r12
	movq	%r13, 352(%rsp)
	movq	%r13, %rbp
	movq	%rbx, %r14
	cmpq	424(%rsp), %r12
	je	.LBB315_186
.LBB315_160:
	movq	%rbp, %rsi
	testq	%rbp, %rbp
	jne	.LBB315_162
	movq	200(%rsp), %rax
	movq	40(%rax), %rsi
.LBB315_162:
	vmovups	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	(%r12), %rdx
	leaq	16(%r14), %rax
	testq	%r14, %r14
	cmoveq	%r14, %rax
	movq	%rax, 1184(%rsp)
	movq	%r15, 1192(%rsp)
	movq	$0, 1200(%rsp)
	vmovups	%ymm0, 1152(%rsp)
.Ltmp11295:
	leaq	64(%rsp), %rdi
	leaq	1152(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::expr::substitute_pattern_deferring::<false>
.Ltmp11296:
	movq	64(%rsp), %rax
	movq	72(%rsp), %r13
	cmpq	$-1, %rax
	jne	.LBB315_225
.Ltmp11314:
	movq	<purrdf_sparql_eval::expr::Deferral>::into_placeholders@GOTPCREL(%rip), %rax
	leaq	1152(%rsp), %rdi
	callq	*%rax
.Ltmp11315:
	movq	%rax, %rbx
	testq	%r14, %r14
	je	.LBB315_168
	lock		decq	(%r14)
	jne	.LBB315_168
	#MEMBARRIER
.Ltmp11317:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11318:
.LBB315_168:
	movq	%rbx, 360(%rsp)
	testq	%rbp, %rbp
	je	.LBB315_159
.Ltmp11322:
	movq	%rbp, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::GraphPattern>
.Ltmp11323:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-144, %rcx
	movabsq	$-9223372036854775808, %rdx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB315_172
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB315_172:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB315_158
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB315_172
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-144, %rcx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		xaddq	%rcx, (%rax)
	movabsq	$-9223372036854775808, %rax
	addq	$-144, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB315_175:
	cmpq	%rax, %rcx
	jge	.LBB315_157
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB315_175
	jmp	.LBB315_157
.LBB315_177:
	xorl	%ebx, %ebx
.LBB315_178:
	movq	40(%r12), %rsi
.Ltmp11281:
	movq	40(%rsp), %r8
	leaq	64(%rsp), %rdi
	leaq	368(%rsp), %rdx
	leaq	864(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_substituted_delivered::<purrdf_core::ir::dataset::RdfDataset, false, false, ()>
.Ltmp11282:
	testb	%bl, %bl
	movq	40(%rsp), %rbx
	je	.LBB315_183
	movq	1112(%rbx), %rax
	testq	%rax, %rax
	je	.LBB315_183
	leaq	-1(%rax), %rcx
	movq	%rcx, 1112(%rbx)
	movq	1104(%rbx), %rcx
	movq	-8(%rcx,%rax,8), %rax
	movq	%rax, 624(%rsp)
	lock		decq	(%rax)
	jne	.LBB315_183
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB315_183:
	vmovups	112(%rsp), %zmm1
	vmovups	64(%rsp), %zmm0
	vmovups	%zmm1, 272(%rsp)
	vmovups	%zmm0, 224(%rsp)
.Ltmp11286:
	leaq	368(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11287:
.LBB315_184:
.Ltmp11312:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11313:
	jmp	.LBB315_199
.LBB315_185:
	xorl	%ebx, %ebx
	xorl	%r13d, %r13d
.LBB315_186:
	movq	176(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_188
	movq	24(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB315_188:
.Ltmp11327:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11328:
	movq	200(%rsp), %rax
	testq	%r13, %r13
	jne	.LBB315_191
	movq	40(%rax), %r13
.LBB315_191:
	movq	$0, 64(%rsp)
.Ltmp11332:
	movq	40(%rsp), %rsi
	leaq	368(%rsp), %rdi
	leaq	64(%rsp), %rdx
	movq	%rbx, %rcx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_substituted_exists
.Ltmp11333:
	movq	392(%rsp), %rdx
.Ltmp11334:
	leaq	224(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r13, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp11335:
.Ltmp11339:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp11340:
.Ltmp11344:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11345:
	movq	40(%rsp), %rbx
	cmpq	$0, 192(%rsp)
	movq	184(%rsp), %rax
	je	.LBB315_198
	lock		decq	(%rax)
	jne	.LBB315_198
	#MEMBARRIER
.Ltmp11349:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	632(%rsp), %rdi
	callq	*%rax
.Ltmp11350:
.LBB315_198:
.Ltmp11355:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11356:
.LBB315_199:
	cmpl	$1, 224(%rsp)
	movq	16(%rsp), %r15
	je	.LBB315_327
	leaq	240(%rsp), %rcx
	movq	232(%rsp), %rax
	vmovups	24(%rcx), %zmm1
	vmovups	(%rcx), %zmm0
	leaq	560(%rsp), %rcx
	vmovups	%zmm1, 552(%rsp)
	vmovups	%zmm0, 528(%rsp)
	vmovups	528(%rsp), %ymm0
	vmovups	24(%rcx), %ymm1
	vmovups	(%rcx), %ymm2
.LBB315_201:
	vmovups	%ymm1, 760(%rsp)
	vmovups	%ymm0, 992(%rsp)
	vmovups	%ymm2, 736(%rsp)
	cmpq	$-1, %rax
	jne	.LBB315_243
	movq	1016(%rsp), %rax
	movq	32(%rax), %rbx
	testq	%rbx, %rbx
	je	.LBB315_208
	movq	24(%rax), %r14
	shlq	$4, %rbx
	addq	%r14, %rbx
	.p2align	4
.LBB315_205:
	movq	(%r14), %rax
	lock		incq	(%rax)
	jle	.LBB315_347
	movq	8(%r14), %rdx
	movq	(%r14), %rsi
.Ltmp11365:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11366:
	addq	$16, %r14
	cmpq	%rbx, %r14
	jne	.LBB315_205
.LBB315_208:
	movq	48(%rsp), %rax
	movq	(%rax), %rbx
	leaq	-1(%rbx), %rdx
	cmpq	$4, %rdx
	jbe	.LBB315_211
	movq	16(%rax), %rbx
	movq	8(%rax), %rsi
	leaq	-8(,%rbx,8), %rdx
	leaq	-1(%rbx), %r15
	cmpq	$5, %r15
	jae	.LBB315_213
	movq	16(%rsp), %r15
	jmp	.LBB315_212
.LBB315_211:
	leaq	8(%rax), %rsi
	shlq	$3, %rdx
.LBB315_212:
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB315_222
.LBB315_213:
	movq	%rsi, %r12
	movl	$4, %esi
	movq	%rdx, %rdi
	movq	%rdx, %r14
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB315_345
	movb	$61, %cl
	leaq	-2(%rbx), %rsi
	movq	%r12, %rdi
	bzhiq	%rcx, %r15, %rcx
	cmpq	%rsi, %rcx
	cmovbq	%rcx, %rsi
	cmpq	$16, %rsi
	jae	.LBB315_216
	movq	%r15, %rcx
	movq	%rdi, %rdx
	xorl	%esi, %esi
	jmp	.LBB315_218
.LBB315_216:
	incq	%rsi
	movl	$16, %edx
	movl	%esi, %ecx
	andl	$15, %ecx
	cmoveq	%rdx, %rcx
	xorl	%r8d, %r8d
	subq	%rcx, %rsi
	movq	%r15, %rcx
	leaq	(%rdi,%rsi,8), %rdx
	subq	%rsi, %rcx
.LBB315_217:
	vmovups	(%rdi,%r8,8), %zmm0
	vmovups	64(%rdi,%r8,8), %zmm1
	vmovups	%zmm1, 64(%rax,%r8,8)
	vmovups	%zmm0, (%rax,%r8,8)
	addq	$16, %r8
	cmpq	%r8, %rsi
	jne	.LBB315_217
.LBB315_218:
	leaq	(%rdi,%r15,8), %rdi
	leaq	4(%rax,%rsi,8), %rsi
	xorl	%r8d, %r8d
.LBB315_219:
	cmpq	%rdi, %rdx
	je	.LBB315_221
	movl	(%rdx), %r9d
	movl	4(%rdx), %r10d
	addq	$8, %rdx
	movl	%r9d, -4(%rsi,%r8,8)
	movl	%r10d, (%rsi,%r8,8)
	incq	%r8
	cmpq	%r8, %rcx
	jne	.LBB315_219
.LBB315_221:
	movq	16(%rsp), %r15
	movq	%rax, 224(%rsp)
	movq	%rbx, 232(%rsp)
.LBB315_222:
	vmovups	992(%rsp), %ymm0
	leaq	72(%rsp), %rcx
	movq	224(%rsp), %rax
	movq	232(%rsp), %rdx
	vmovups	%ymm0, 32(%rcx)
	vmovups	240(%rsp), %xmm0
	movq	%rbx, 64(%rsp)
	movq	40(%rsp), %rbx
	movq	%rax, (%rcx)
	movq	%rdx, 8(%rcx)
	vmovups	%xmm0, 16(%rcx)
	movq	32(%rsp), %rcx
	cmpq	456(%rsp), %rcx
	jne	.LBB315_124
.Ltmp11373:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::governor::soundness::NodeAnalysis>>::grow_one@GOTPCREL(%rip), %rax
	leaq	456(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11374:
	movq	464(%rsp), %rax
	movq	%rax, 208(%rsp)
	jmp	.LBB315_124
.LBB315_225:
	leaq	72(%rsp), %rcx
	vmovups	8(%rcx), %zmm0
	vmovups	24(%rcx), %zmm1
	leaq	240(%rsp), %rcx
	vmovups	%zmm1, 32(%rcx)
	vmovups	%zmm0, 16(%rcx)
	movq	%rax, 240(%rsp)
	movq	%r13, 248(%rsp)
	movq	$1, 224(%rsp)
.Ltmp11300:
	leaq	1152(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp11301:
	movq	176(%rsp), %rsi
	movq	24(%rsp), %rdi
	testq	%rsi, %rsi
	je	.LBB315_228
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB315_228:
.Ltmp11303:
	leaq	368(%rsp), %rdi
	movq	%r14, %rbx
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11304:
	testq	%r14, %r14
	je	.LBB315_232
	lock		decq	(%r14)
	jne	.LBB315_232
	#MEMBARRIER
.Ltmp11305:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11306:
.LBB315_232:
.Ltmp11308:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11309:
	movq	40(%rsp), %rbx
	cmpq	$0, 192(%rsp)
	movq	184(%rsp), %rax
	je	.LBB315_184
	lock		decq	(%rax)
	jne	.LBB315_184
	#MEMBARRIER
.Ltmp11310:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	632(%rsp), %rdi
	callq	*%rax
.Ltmp11311:
	jmp	.LBB315_184
.LBB315_236:
	movb	$1, %al
	testb	%al, %al
	je	.LBB315_133
.LBB315_237:
.Ltmp11266:
	movq	purrdf_stack::is_low_cold@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11267:
	testb	%al, %al
	je	.LBB315_133
	movabsq	$-9223372036854775784, %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.438(%rip), %rcx
	movq	%rax, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	$41, 256(%rsp)
	jmp	.LBB315_136
.LBB315_240:
	vmovups	528(%rsp), %ymm0
	leaq	72(%rsp), %rax
	vmovups	%ymm0, (%rax)
	movq	$-1, %rax
.LBB315_241:
	vmovups	80(%rsp), %xmm0
	movq	72(%rsp), %rsi
	movq	96(%rsp), %rdi
	movq	%rdi, 720(%rsp)
	jmp	.LBB315_49
.LBB315_242:
	movq	40(%rsp), %rbx
	movl	$8, %eax
	movq	$0, 456(%rsp)
	movq	$8, 464(%rsp)
	movq	$0, 472(%rsp)
	movq	$0, 32(%rsp)
	movq	%rax, 208(%rsp)
	jmp	.LBB315_246
.LBB315_243:
	vmovups	992(%rsp), %ymm0
	vmovups	736(%rsp), %ymm2
	vmovups	760(%rsp), %ymm1
	movq	%rax, 64(%rsp)
	vmovups	%ymm0, 72(%rsp)
	vmovups	%ymm2, 104(%rsp)
	vmovups	%ymm1, 128(%rsp)
.Ltmp11360:
	leaq	224(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	64(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp11361:
	cmpq	$-1, 224(%rsp)
	je	.LBB315_246
.Ltmp11362:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11363:
.LBB315_246:
	leaq	16(%r15), %rsi
.Ltmp11387:
	movq	<purrdf_sparql_eval::solution::VarSchema>::union@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	leaq	800(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11388:
	vmovups	224(%rsp), %ymm0
	vmovups	248(%rsp), %ymm1
	leaq	80(%rsp), %r14
	movq	$1, 64(%rsp)
	movq	$1, 72(%rsp)
	vmovups	%ymm0, 80(%rsp)
	vmovups	%ymm1, 104(%rsp)
.Ltmp11390:
	movl	$8, %edi
	movl	$72, %esi
	vzeroupper
	callq	alloc::boxed::box_new_uninit
.Ltmp11391:
	movq	128(%rsp), %rcx
	movq	%rax, 24(%rsp)
	movq	%rcx, 64(%rax)
	vmovups	64(%rsp), %zmm0
	vmovups	%zmm0, (%rax)
	movq	%rax, 1104(%rsp)
	movq	32(%r15), %rsi
	movq	32(%rax), %rdi
	testq	%rdi, %rdi
	je	.LBB315_253
	movq	616(%rbx), %rax
	testq	%rax, %rax
	je	.LBB315_253
	movq	32(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB315_253
	movq	%rax, %rdx
	orq	%rdi, %rdx
	movabsq	$230584300921369396, %rcx
	shrq	$32, %rdx
	je	.LBB315_336
	xorl	%edx, %edx
	divq	%rdi
	jmp	.LBB315_337
.LBB315_253:
	movq	32(%rsp), %rbx
	movl	$0, 200(%rsp)
.LBB315_254:
	movq	%rdi, 344(%rsp)
	movq	%rsi, 424(%rsp)
	testq	%rbx, %rbx
	je	.LBB315_257
	leaq	(,%rbx,8), %rax
	movl	$8, %esi
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB315_346
	movq	%rax, %rdx
	jmp	.LBB315_258
.LBB315_257:
	movl	$8, %edx
.LBB315_258:
	movq	32(%rsp), %rax
	movq	208(%rsp), %r14
	movq	%rbx, 368(%rsp)
	movq	%rdx, 376(%rsp)
	movq	$0, 384(%rsp)
	testq	%rax, %rax
	je	.LBB315_319
	leaq	(%rax,%rax,8), %rax
	leaq	72(%rsp), %rcx
	movq	$0, 32(%rsp)
	movq	%rcx, 792(%rsp)
	movq	24(%rsp), %rcx
	leaq	(%r14,%rax,8), %rax
	movq	%rax, 176(%rsp)
	movq	424(%rsp), %rax
	addq	$16, %rcx
	movq	%rcx, 184(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.449(%rip), %rcx
	movq	%rcx, 416(%rsp)
	leaq	(,%rax,8), %rax
	movq	%rax, 512(%rsp)
	jmp	.LBB315_261
.LBB315_260:
	movq	%rbx, %r14
	addq	$72, %r14
	cmpq	176(%rsp), %r14
	je	.LBB315_319
.LBB315_261:
	movq	64(%r14), %rsi
	movq	%rdx, %r12
	addq	$16, %rsi
.Ltmp11399:
	movq	184(%rsp), %rdx
	movq	purrdf_sparql_eval::binop::right_to_out_map@GOTPCREL(%rip), %rax
	leaq	736(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11400:
	movq	56(%r14), %rax
	movq	%r14, %rbx
	testq	%rax, %rax
	je	.LBB315_313
	movq	48(%r14), %r13
	leaq	8(%r14), %rcx
	movq	744(%rsp), %rbp
	movq	752(%rsp), %r14
	leaq	(%rax,%rax,4), %rax
	movq	%r12, %rdx
	movq	%rcx, 48(%rsp)
	leaq	(%r13,%rax,8), %rax
	movq	%rax, 208(%rsp)
	jmp	.LBB315_266
.LBB315_264:
	movq	32(%rsp), %rsi
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 32(%rsp)
	movq	%r15, (%rdx,%rax,8)
	movq	%r12, 8(%rdx,%rax,8)
	movq	16(%rsp), %r15
	vmovaps	64(%rsp), %xmm0
	vmovups	%xmm0, 16(%rdx,%rax,8)
	movq	80(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 384(%rsp)
	addq	$40, %r13
	cmpq	208(%rsp), %r13
	je	.LBB315_314
.LBB315_266:
	movq	(%r13), %rcx
	leaq	8(%r13), %r12
	movq	%rdx, 216(%rsp)
	movq	%r12, %rax
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB315_268
	movq	16(%r13), %rcx
	movq	8(%r13), %rax
	decq	%rcx
.LBB315_268:
	testq	%rcx, %rcx
	je	.LBB315_280
	leaq	(%rax,%rcx,8), %rcx
	xorl	%edx, %edx
	jmp	.LBB315_271
	.p2align	4
.LBB315_270:
	leaq	(%rax,%rdx,8), %rsi
	incq	%rdx
	addq	$8, %rsi
	cmpq	%rcx, %rsi
	je	.LBB315_280
.LBB315_271:
	cmpq	%rdx, %r14
	je	.LBB315_342
	movq	(%rbx), %r8
	movq	48(%rsp), %rsi
	decq	%r8
	cmpq	$4, %r8
	jbe	.LBB315_274
	movq	16(%rbx), %r8
	movq	8(%rbx), %rsi
	decq	%r8
.LBB315_274:
	movq	(%rbp,%rdx,8), %rdi
	cmpq	%r8, %rdi
	jae	.LBB315_270
	movl	(%rax,%rdx,8), %r8d
	cmpl	$2, %r8d
	je	.LBB315_270
	movl	(%rsi,%rdi,8), %r9d
	cmpl	$2, %r9d
	je	.LBB315_270
	cmpl	%r9d, %r8d
	jne	.LBB315_310
	movl	4(%rsi,%rdi,8), %esi
	cmpl	%esi, 4(%rax,%rdx,8)
	je	.LBB315_270
.LBB315_310:
	movq	216(%rsp), %rdx
	addq	$40, %r13
	cmpq	208(%rsp), %r13
	jne	.LBB315_266
	jmp	.LBB315_314
.LBB315_280:
	cmpb	$0, 200(%rsp)
	je	.LBB315_282
	movq	32(%rsp), %rax
	cmpq	192(%rsp), %rax
	jae	.LBB315_316
.LBB315_282:
	movq	344(%rsp), %rdx
	movq	$1, 64(%rsp)
	cmpq	$5, %rdx
	jae	.LBB315_311
	vmovups	72(%rsp), %xmm0
	movq	96(%rsp), %rax
	movq	64(%rsp), %rsi
	movq	88(%rsp), %rcx
	movq	%rax, 256(%rsp)
	movq	%rsi, 224(%rsp)
	movq	%rcx, 248(%rsp)
	vmovups	%xmm0, 232(%rsp)
	testq	%rdx, %rdx
	je	.LBB315_285
.LBB315_284:
	movl	$2, %eax
	jmp	.LBB315_286
.LBB315_285:
	movl	$-1, %eax
.LBB315_286:
	movl	%eax, 64(%rsp)
	movq	%rdx, 72(%rsp)
.Ltmp11410:
	leaq	224(%rsp), %rdi
	leaq	64(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp11411:
	vmovups	224(%rsp), %ymm0
	movq	256(%rsp), %rax
	movq	424(%rsp), %r8
	leaq	536(%rsp), %rdi
	movq	%rax, 560(%rsp)
	vmovups	%ymm0, 528(%rsp)
	movq	528(%rsp), %r15
	movq	%r15, %rdx
	cmpq	$6, %r15
	jb	.LBB315_289
	movq	536(%rsp), %rdi
	movq	544(%rsp), %rdx
.LBB315_289:
	decq	%rdx
	cmpq	%rdx, %r8
	ja	.LBB315_340
	movq	(%rbx), %rax
	movq	16(%rbx), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %r8
	jne	.LBB315_341
	movq	48(%rsp), %rsi
	cmpq	$5, %rax
	jb	.LBB315_293
	movq	48(%rsp), %rax
	movq	(%rax), %rsi
.LBB315_293:
	movq	512(%rsp), %rdx
	movq	memcpy@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	(%r13), %rax
	decq	%rax
	cmpq	$5, %rax
	jb	.LBB315_295
	movq	16(%r13), %rax
	movq	8(%r13), %r12
	decq	%rax
.LBB315_295:
	testq	%rax, %rax
	je	.LBB315_307
	shlq	$3, %rax
	leaq	536(%rsp), %r9
	xorl	%edi, %edi
	jmp	.LBB315_299
	.p2align	4
.LBB315_297:
	movl	4(%r12,%rdi,8), %esi
	movl	%ecx, (%r8,%rdx,8)
	movl	%esi, 4(%r8,%rdx,8)
.LBB315_298:
	incq	%rdi
	addq	$-8, %rax
	je	.LBB315_306
.LBB315_299:
	movl	(%r12,%rdi,8), %ecx
	cmpl	$2, %ecx
	je	.LBB315_298
	cmpq	%r14, %rdi
	jae	.LBB315_344
	movq	528(%rsp), %rsi
	movq	%rsi, %r8
	cmpq	$6, %rsi
	jb	.LBB315_303
	movq	544(%rsp), %r8
.LBB315_303:
	movq	(%rbp,%rdi,8), %rdx
	decq	%r8
	cmpq	%r8, %rdx
	jae	.LBB315_343
	movq	%r9, %r8
	cmpq	$6, %rsi
	jb	.LBB315_297
	movq	536(%rsp), %r8
	jmp	.LBB315_297
.LBB315_306:
	movq	528(%rsp), %r15
.LBB315_307:
	leaq	536(%rsp), %rax
	movq	536(%rsp), %r12
	movq	216(%rsp), %rdx
	movq	32(%rsp), %rcx
	vmovups	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 80(%rsp)
	vmovaps	%xmm0, 64(%rsp)
	cmpq	368(%rsp), %rcx
	jne	.LBB315_264
.Ltmp11417:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdi
	callq	*%rax
.Ltmp11418:
	movq	376(%rsp), %rdx
	jmp	.LBB315_264
.LBB315_311:
.Ltmp11407:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	64(%rsp), %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp11408:
	vmovups	64(%rsp), %ymm0
	movq	96(%rsp), %rax
	movq	344(%rsp), %rdx
	movq	%rax, 256(%rsp)
	vmovups	%ymm0, 224(%rsp)
	jmp	.LBB315_284
.LBB315_313:
	movq	%r12, %rdx
.LBB315_314:
	movq	736(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_260
	movq	744(%rsp), %rdi
	movq	%rdx, %r14
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	movq	%r14, %rdx
	jmp	.LBB315_260
.LBB315_316:
	movq	32(%rsp), %rdx
	incq	%rdx
.Ltmp11404:
	movq	40(%rsp), %rsi
	movq	344(%rsp), %rcx
	leaq	64(%rsp), %rdi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::observe_cells
.Ltmp11405:
	movq	736(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_319
	shlq	$3, %rsi
	movl	$8, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB315_319:
	movq	368(%rsp), %rcx
	movq	384(%rsp), %rax
	movq	376(%rsp), %r8
	movq	%rcx, 224(%rsp)
	movq	24(%rsp), %rcx
	movq	%rax, 240(%rsp)
	movq	%r8, 232(%rsp)
	movq	%rcx, 248(%rsp)
.Ltmp11425:
	leaq	64(%rsp), %rdi
	leaq	888(%rsp), %rsi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
.Ltmp11426:
	vmovups	64(%rsp), %zmm0
	vmovups	96(%rsp), %zmm1
	movq	56(%rsp), %rax
	xorl	%ebp, %ebp
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp11430:
	leaq	456(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11431:
	xorl	%ebp, %ebp
.Ltmp11432:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11433:
	lock		decq	(%r15)
	jne	.LBB315_324
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp11435:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11436:
.LBB315_324:
	cmpq	$0, 432(%rsp)
	je	.LBB315_326
	xorl	%ebp, %ebp
.Ltmp11437:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11438:
.LBB315_326:
	xorl	%ebp, %ebp
.Ltmp11439:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11440:
	jmp	.LBB315_118
.LBB315_327:
	leaq	240(%rsp), %rax
	vmovups	32(%rax), %zmm1
	vmovups	(%rax), %zmm0
	movq	56(%rsp), %rax
	vmovups	%zmm1, 560(%rsp)
	vmovups	%zmm0, 528(%rsp)
	vmovups	560(%rsp), %zmm1
	vmovups	528(%rsp), %zmm0
	jmp	.LBB315_329
.LBB315_328:
	leaq	72(%rsp), %rax
	movq	16(%rsp), %r15
	vmovups	40(%rax), %zmm1
	vmovups	8(%rax), %zmm0
	movq	56(%rsp), %rax
	vmovups	%zmm1, 256(%rsp)
	vmovups	%zmm0, 224(%rsp)
	vmovups	256(%rsp), %zmm1
	vmovups	224(%rsp), %zmm0
.LBB315_329:
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movb	$1, %bpl
.Ltmp11379:
	leaq	456(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11380:
	movb	$1, %bpl
.Ltmp11381:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11382:
	lock		decq	(%r15)
	jne	.LBB315_334
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp11383:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11384:
.LBB315_334:
	cmpq	$0, 432(%rsp)
	je	.LBB315_57
	movb	$1, %bpl
.Ltmp11385:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11386:
	jmp	.LBB315_57
.LBB315_336:
	xorl	%edx, %edx
	divl	%edi
.LBB315_337:
	movq	32(%rsp), %rbx
	movq	%rax, 192(%rsp)
	cmpq	%rcx, %rax
	jae	.LBB315_339
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	movb	$1, %al
	movl	%eax, 200(%rsp)
	jmp	.LBB315_254
.LBB315_339:
	movl	$0, 200(%rsp)
	jmp	.LBB315_254
.LBB315_340:
.Ltmp11420:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.451(%rip), %rcx
	xorl	%edi, %edi
	movq	%r8, %rsi
	vzeroupper
	callq	*%rax
.Ltmp11421:
	jmp	.LBB315_347
.LBB315_341:
.Ltmp11413:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.448(%rip), %rdx
	movq	%r8, %rdi
	vzeroupper
	callq	*%rax
.Ltmp11414:
	jmp	.LBB315_347
.LBB315_342:
.Ltmp11402:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.710(%rip), %rdx
	movq	%r14, %rdi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11403:
	jmp	.LBB315_347
.LBB315_343:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.450(%rip), %rax
	movq	%rdx, %rdi
	movq	%r8, %r14
	movq	%rax, 416(%rsp)
.LBB315_344:
.Ltmp11415:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	416(%rsp), %rdx
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11416:
	jmp	.LBB315_347
.LBB315_345:
.Ltmp11368:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11369:
	jmp	.LBB315_347
.LBB315_346:
.Ltmp11396:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp11397:
.LBB315_347:
	ud2
.LBB315_348:
.Ltmp11409:
	movq	%rax, %r14
	movq	64(%rsp), %rax
	movq	24(%rsp), %rbx
	cmpq	$6, %rax
	jae	.LBB315_404
	jmp	.LBB315_414
.LBB315_349:
.Ltmp11307:
	jmp	.LBB315_366
.LBB315_350:
.Ltmp11302:
	movq	%rax, %r15
	jmp	.LBB315_384
.LBB315_351:
.Ltmp11364:
	jmp	.LBB315_360
.LBB315_352:
.Ltmp11398:
	movq	24(%rsp), %rbx
	movq	%rax, %r14
	jmp	.LBB315_417
.LBB315_353:
.Ltmp11274:
	movq	%rax, %r14
.Ltmp11275:
	leaq	64(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11276:
	movq	16(%rsp), %r15
	jmp	.LBB315_397
.LBB315_354:
.Ltmp11336:
	movq	%rax, %rbx
.Ltmp11337:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp11338:
	jmp	.LBB315_392
.LBB315_355:
.Ltmp11427:
	movq	%rax, %r14
	xorl	%ebp, %ebp
	jmp	.LBB315_419
.LBB315_356:
.Ltmp11392:
	movq	%rax, %rbx
.Ltmp11393:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11394:
	movq	16(%rsp), %r15
	movb	$1, %bpl
	movq	%rbx, %r14
	jmp	.LBB315_419
.LBB315_358:
.Ltmp11395:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB315_359:
.Ltmp11389:
.LBB315_360:
	movq	%rax, %r14
	movb	$1, %bpl
	jmp	.LBB315_419
.LBB315_361:
.Ltmp11283:
	movq	%rax, %r14
.Ltmp11284:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11285:
	movq	16(%rsp), %r15
	jmp	.LBB315_397
.LBB315_362:
.Ltmp11346:
	movq	%rax, %rbx
	jmp	.LBB315_393
.LBB315_363:
.Ltmp11294:
	movq	%rax, %r15
	xorl	%r14d, %r14d
	jmp	.LBB315_386
.LBB315_364:
.Ltmp11434:
	movq	%rax, %r14
	jmp	.LBB315_422
.LBB315_365:
.Ltmp11341:
.LBB315_366:
	movq	%rax, %rbx
	jmp	.LBB315_392
.LBB315_367:
.Ltmp11370:
	jmp	.LBB315_407
.LBB315_368:
.Ltmp11329:
	movq	%rax, %r15
	movq	%rbx, %r14
	jmp	.LBB315_387
.LBB315_369:
.Ltmp11401:
	movq	24(%rsp), %rbx
	movq	%rax, %r14
	jmp	.LBB315_416
.LBB315_370:
.Ltmp11319:
	movq	%rax, %r15
	movq	%rbx, 360(%rsp)
	movq	%rbx, %r14
	jmp	.LBB315_382
.LBB315_371:
.Ltmp11351:
	movq	16(%rsp), %r15
	movq	%rax, %r14
	jmp	.LBB315_397
.LBB315_372:
.Ltmp11375:
	movq	%rax, %r14
.Ltmp11376:
	leaq	64(%rsp), %rdi
	callq	core::ptr::drop_glue::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>
.Ltmp11377:
	jmp	.LBB315_379
.LBB315_373:
.Ltmp11378:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB315_374:
.Ltmp11469:
	movq	%rax, %r14
	xorl	%ebp, %ebp
	jmp	.LBB315_426
.LBB315_375:
.Ltmp11450:
	movq	%rax, %r14
	jmp	.LBB315_424
.LBB315_376:
.Ltmp11324:
	movl	$144, %esi
	movl	$8, %edx
	movq	%rbp, %rdi
	movq	%rax, %r15
	callq	__rustc::__rust_dealloc
	movq	%r13, 352(%rsp)
	movq	%rbx, %r14
	jmp	.LBB315_384
.LBB315_377:
.Ltmp11464:
	movq	%rax, %r14
	movb	$1, %bpl
	jmp	.LBB315_426
.LBB315_378:
.Ltmp11359:
	movq	%rax, %r14
.LBB315_379:
	movq	16(%rsp), %r15
	movb	$1, %bpl
	jmp	.LBB315_419
.LBB315_380:
.Ltmp11474:
	movq	%rax, %r14
	jmp	.LBB315_428
.LBB315_381:
.Ltmp11316:
	movq	%rax, %r15
.LBB315_382:
.Ltmp11320:
	movq	%r13, %rdi
	callq	core::ptr::drop_glue::<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>
.Ltmp11321:
	jmp	.LBB315_384
.LBB315_383:
.Ltmp11297:
	movq	%rax, %r15
.Ltmp11298:
	leaq	1152(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp11299:
.LBB315_384:
	cmpq	$0, 176(%rsp)
	je	.LBB315_386
	movq	176(%rsp), %rsi
	movq	24(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rsi
	callq	__rustc::__rust_dealloc
.LBB315_386:
.Ltmp11325:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp11326:
.LBB315_387:
	testq	%r14, %r14
	je	.LBB315_391
	lock		decq	(%r14)
	jne	.LBB315_391
	#MEMBARRIER
.Ltmp11330:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp11331:
	movq	%r15, %rbx
	jmp	.LBB315_392
.LBB315_391:
	movq	%r15, %rbx
.LBB315_392:
.Ltmp11342:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp11343:
.LBB315_393:
	cmpq	$0, 192(%rsp)
	je	.LBB315_396
	movq	184(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB315_396
	#MEMBARRIER
.Ltmp11347:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	632(%rsp), %rdi
	callq	*%rax
.Ltmp11348:
	movq	16(%rsp), %r15
	movq	%rbx, %r14
	jmp	.LBB315_397
.LBB315_396:
	movq	16(%rsp), %r15
	movq	%rbx, %r14
.LBB315_397:
	movb	$1, %bpl
.Ltmp11352:
	leaq	1024(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp11353:
	jmp	.LBB315_419
.LBB315_398:
.Ltmp11354:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB315_399:
.Ltmp11261:
	movq	%rax, %r14
	xorl	%ebp, %ebp
	jmp	.LBB315_428
.LBB315_400:
.Ltmp11419:
	movq	%rax, %r14
	cmpq	$6, %r15
	jb	.LBB315_402
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
.LBB315_402:
	movq	16(%rsp), %r15
	jmp	.LBB315_410
.LBB315_403:
.Ltmp11412:
	movq	%rax, %r14
	movq	224(%rsp), %rax
	movq	16(%rsp), %r15
	movq	24(%rsp), %rbx
	leaq	232(%rsp), %rcx
	movq	%rcx, 792(%rsp)
	cmpq	$5, %rax
	jbe	.LBB315_414
.LBB315_404:
	movq	792(%rsp), %rcx
	movq	(%rcx), %rdi
	jmp	.LBB315_413
.LBB315_405:
.Ltmp11461:
	movq	%rax, %r14
	jmp	.LBB315_426
.LBB315_406:
.Ltmp11367:
.LBB315_407:
	movb	$1, %bpl
	movq	%rax, %r14
.Ltmp11371:
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp11372:
	movq	16(%rsp), %r15
	jmp	.LBB315_419
.LBB315_408:
.Ltmp11477:
	movq	%rax, %r14
	jmp	.LBB315_429
.LBB315_409:
.Ltmp11406:
	movq	%rax, %r14
.LBB315_410:
	movq	24(%rsp), %rbx
	jmp	.LBB315_414
.LBB315_411:
.Ltmp11422:
	movq	%rax, %r14
	movq	528(%rsp), %rax
	movq	16(%rsp), %r15
	movq	24(%rsp), %rbx
	cmpq	$6, %rax
	jb	.LBB315_414
	movq	536(%rsp), %rdi
.LBB315_413:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB315_414:
	movq	736(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB315_416
	movq	744(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB315_416:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB315_417:
	lock		decq	(%rbx)
	movb	$1, %bpl
	jne	.LBB315_419
	#MEMBARRIER
.Ltmp11423:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1104(%rsp), %rdi
	callq	*%rax
.Ltmp11424:
.LBB315_419:
.Ltmp11428:
	leaq	456(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp11429:
	jmp	.LBB315_421
.LBB315_420:
.Ltmp11443:
	movq	%rax, %r14
.LBB315_421:
.Ltmp11444:
	leaq	800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp11445:
.LBB315_422:
	lock		decq	(%r15)
	jne	.LBB315_424
	#MEMBARRIER
.Ltmp11446:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	callq	*%rax
.Ltmp11447:
.LBB315_424:
	cmpq	$0, 432(%rsp)
	je	.LBB315_426
.Ltmp11451:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp11452:
.LBB315_426:
	movq	504(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB315_428
	leaq	504(%rsp), %rdi
	#MEMBARRIER
.Ltmp11470:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp11471:
.LBB315_428:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB315_429:
	testb	%bpl, %bpl
	je	.LBB315_431
.Ltmp11478:
	leaq	888(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp11479:
.LBB315_431:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB315_432:
.Ltmp11480:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end315:
purrdf_sparql_eval::binop::eval_application_yielding::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>:
.Lfunc_begin324:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception231
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
	subq	$664, %rsp
	.cfi_def_cfa_offset 720
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%r8, %r12
	movq	%rcx, %rbp
	movq	%rdx, %r15
	movq	%rdi, %rbx
	movq	%r9, 16(%rsp)
	movq	$1, 472(%rsp)
.Ltmp12297:
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	232(%rsp), %r13
	movq	%r13, %rdi
	callq	*%rax
.Ltmp12298:
	movq	720(%rsp), %r14
	movb	$0, 15(%rsp)
	vxorps	%xmm0, %xmm0, %xmm0
	movq	624(%r14), %rax
	testq	%rax, %rax
	je	.LBB324_7
	testb	$1, 1200(%r14)
	je	.LBB324_7
	movq	1208(%r14), %rcx
	cmpq	40(%rax), %rcx
	jne	.LBB324_7
	movl	1216(%r14), %ecx
	subl	80(%rax), %ecx
	jb	.LBB324_7
	cmpq	%rcx, 32(%rax)
	jbe	.LBB324_7
	movq	24(%rax), %rax
	shlq	$4, %rcx
	vmovups	(%rax,%rcx), %xmm0
.LBB324_7:
	leaq	15(%rsp), %rax
	leaq	472(%rsp), %r9
	leaq	16(%rsp), %r8
	vmovaps	%xmm0, 208(%rsp)
	movq	$0, 24(%rsp)
	movq	%rax, 144(%rsp)
	movq	%rbp, 152(%rsp)
	movq	%r9, 160(%rsp)
	movq	%r8, 168(%rsp)
	leaq	24(%rsp), %r9
	leaq	208(%rsp), %r8
	movq	%r9, 176(%rsp)
	movq	%r8, 184(%rsp)
	leaq	144(%rsp), %r9
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.461(%rip), %r8
	movq	%r12, 192(%rsp)
	movq	%r13, 200(%rsp)
	movq	%r9, 32(%rsp)
	movq	%r8, 40(%rsp)
	movb	$0, 48(%rsp)
.Ltmp12299:
	leaq	544(%rsp), %rdi
	leaq	32(%rsp), %rdx
	movq	%r15, %rsi
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_yielding::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp12300:
	cmpl	$1, 544(%rsp)
	jne	.LBB324_32
	vmovups	592(%rsp), %zmm1
	vmovups	560(%rsp), %zmm0
	movq	304(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB324_19
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	312(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB324_12
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB324_12:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB324_18
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB324_12
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB324_15:
	cmpq	%rax, %rsi
	jge	.LBB324_17
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB324_15
.LBB324_17:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB324_18:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB324_19:
	movq	232(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB324_29
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	240(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB324_22
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB324_22:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB324_28
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB324_22
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB324_25:
	cmpq	%rax, %rsi
	jge	.LBB324_27
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB324_25
.LBB324_27:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB324_28:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB324_29:
	movq	328(%rsp), %rax
	testq	%rax, %rax
	je	.LBB324_58
	lock		decq	(%rax)
	jne	.LBB324_58
	leaq	328(%rsp), %rdi
	#MEMBARRIER
.Ltmp12316:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp12317:
	jmp	.LBB324_58
.LBB324_32:
	leaq	552(%rsp), %rcx
.Ltmp12301:
	leaq	32(%rsp), %rdi
	leaq	232(%rsp), %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp12302:
	cmpq	$-1, 32(%rsp)
	je	.LBB324_56
.Ltmp12303:
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp12304:
	vmovups	272(%rsp), %zmm1
	vmovups	232(%rsp), %zmm0
	movq	536(%rsp), %rax
	movq	%rax, 400(%rsp)
	vmovups	%zmm1, 72(%rsp)
	vmovups	472(%rsp), %zmm1
	vmovups	%zmm0, 32(%rsp)
	vmovups	%zmm1, 336(%rsp)
.Ltmp12308:
	leaq	440(%rsp), %rdi
	leaq	336(%rsp), %rsi
	movq	%r14, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::binop::concat_union::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp12309:
	cmpq	$-1, 32(%rsp)
	je	.LBB324_38
	leaq	336(%rsp), %rdi
	leaq	440(%rsp), %rsi
	leaq	32(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB324_40
	jmp	.LBB324_49
.LBB324_56:
	vmovups	272(%rsp), %zmm1
	vmovups	232(%rsp), %zmm0
	vmovups	%zmm1, 72(%rsp)
	vmovups	%zmm0, 32(%rsp)
.Ltmp12313:
	leaq	336(%rsp), %rdi
	leaq	32(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp12314:
	vmovups	368(%rsp), %zmm1
	vmovups	336(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB324_58:
	leaq	472(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::solution::SolutionSeq; 2]>>
	jmp	.LBB324_53
.LBB324_38:
	vmovups	440(%rsp), %ymm0
	vmovups	%ymm0, 344(%rsp)
	movq	$-1, 336(%rsp)
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB324_49
.LBB324_40:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	112(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB324_42
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB324_42:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB324_48
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB324_42
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB324_45:
	cmpq	%rax, %rsi
	jge	.LBB324_47
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB324_45
.LBB324_47:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB324_48:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB324_49:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB324_52
	lock		decq	(%rax)
	jne	.LBB324_52
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	128(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB324_52:
	vmovups	368(%rsp), %zmm1
	vmovups	336(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB324_53:
	movq	%rbx, %rax
	addq	$664, %rsp
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
	vzeroupper
	retq
.LBB324_64:
	.cfi_def_cfa_offset 720
.Ltmp12315:
	jmp	.LBB324_61
.LBB324_54:
.Ltmp12310:
	movq	%rax, %rbx
.Ltmp12311:
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp12312:
	jmp	.LBB324_63
.LBB324_60:
.Ltmp12318:
.LBB324_61:
	movq	%rax, %rbx
	jmp	.LBB324_62
.LBB324_59:
.Ltmp12305:
	movq	%rax, %rbx
.Ltmp12306:
	leaq	232(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp12307:
.LBB324_62:
.Ltmp12319:
	leaq	472(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::solution::SolutionSeq; 2]>>
.Ltmp12320:
.LBB324_63:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB324_55:
.Ltmp12321:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end324:
purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin350:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception257
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
	subq	$744, %rsp
	.cfi_def_cfa_offset 800
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	520(%rsp), %rdi
	movq	%r9, %r14
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbp
	callq	*%rax
	movq	1136(%r14), %rax
	movq	%rbx, 72(%rsp)
	testq	%rax, %rax
	je	.LBB350_11
	movq	1128(%r14), %rcx
	movq	%rax, 368(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%r15, %r12
	shlq	$4, %r12
	movq	%r14, 64(%rsp)
	movq	%r15, 200(%rsp)
	movq	%r13, 24(%rsp)
	movq	%rbp, 360(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 352(%rsp)
	addq	%rcx, %rax
	movq	%rax, 16(%rsp)
	testq	%r15, %r15
	je	.LBB350_36
	movq	bcmp@GOTPCREL(%rip), %rbx
	movq	%rcx, %rax
.LBB350_3:
	movq	24(%rax), %rbp
	movq	32(%rax), %r14
	addq	$48, %rax
	movq	%rax, (%rsp)
	leaq	16(%rbp), %r15
	jmp	.LBB350_6
	.p2align	4
.LBB350_4:
	xorq	%r14, %rax
	xorq	%rbp, %rdi
	orq	%rax, %rdi
	je	.LBB350_8
.LBB350_5:
	addq	$16, %r13
	cmpq	%r12, %r13
	je	.LBB350_10
.LBB350_6:
	movq	(%r13), %rdi
	movq	8(%r13), %rax
	cmpq	%rbp, %rdi
	sete	%cl
	cmpq	%r14, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB350_4
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%rbx
	testl	%eax, %eax
	jne	.LBB350_5
.LBB350_8:
	movq	(%rsp), %rax
	movq	24(%rsp), %r13
	cmpq	16(%rsp), %rax
	jne	.LBB350_3
	movq	64(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 208(%rsp)
	movq	$0, 224(%rsp)
	movq	$8, 232(%rsp)
	jmp	.LBB350_68
.LBB350_10:
	movq	200(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB350_37
	jmp	.LBB350_161
.LBB350_11:
.Ltmp14308:
	leaq	624(%rsp), %rdi
	movq	%rbp, %rsi
	movq	%r14, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14309:
	cmpl	$1, 624(%rsp)
	jne	.LBB350_73
.LBB350_13:
	vmovups	672(%rsp), %zmm1
	vmovups	640(%rsp), %zmm0
	movq	592(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB350_23
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	600(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB350_16
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB350_16:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_22
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_16
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB350_19:
	cmpq	%rax, %rsi
	jge	.LBB350_21
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB350_19
.LBB350_21:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_22:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB350_23:
	movq	520(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB350_33
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	528(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB350_26
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB350_26:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_32
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_26
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB350_29:
	cmpq	%rax, %rsi
	jge	.LBB350_31
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB350_29
.LBB350_31:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_32:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB350_33:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB350_157
	lock		decq	(%rax)
	jne	.LBB350_157
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB350_157
.LBB350_36:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, (%rsp)
	lock		incq	(%rbp)
	jle	.LBB350_161
.LBB350_37:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 208(%rsp)
	movq	%r14, 216(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB350_158
	movq	%rax, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$64, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$64, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB350_40
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB350_40:
	movq	64(%rsp), %rax
	addq	$1120, %rax
	movq	%rax, 8(%rsp)
	.p2align	4
.LBB350_41:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_47
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_41
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$64, (%rdx)
	movl	$64, %edx
	lock		xaddq	%rdx, (%rdi)
	addq	$64, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB350_44:
	cmpq	%rax, %rdx
	jle	.LBB350_46
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB350_44
.LBB350_46:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_47:
	movq	(%rsp), %rax
	movq	%rbp, (%rsi)
	movq	$4, 80(%rsp)
	movq	%rsi, 88(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 96(%rsp)
	cmpq	16(%rsp), %rax
	je	.LBB350_62
	movl	$1, %r14d
	testq	%r15, %r15
	je	.LBB350_108
	movq	(%rsp), %rbx
	jmp	.LBB350_52
	.p2align	4
.LBB350_50:
	movq	88(%rsp), %rsi
.LBB350_51:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 96(%rsp)
	cmpq	16(%rsp), %rbx
	je	.LBB350_63
.LBB350_52:
	movq	%rsi, 32(%rsp)
.LBB350_53:
	movq	24(%rbx), %r13
	leaq	48(%rbx), %rax
	movq	32(%rbx), %rbp
	movq	24(%rsp), %rbx
	movq	%rax, (%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB350_56
	.p2align	4
.LBB350_54:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB350_58
.LBB350_55:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB350_59
.LBB350_56:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB350_54
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB350_55
.LBB350_58:
	movq	(%rsp), %rbx
	cmpq	16(%rsp), %rbx
	jne	.LBB350_53
	jmp	.LBB350_63
	.p2align	4
.LBB350_59:
	lock		incq	(%r13)
	movq	32(%rsp), %rsi
	jle	.LBB350_161
	movq	(%rsp), %rbx
	movq	%r13, 208(%rsp)
	movq	%rbp, 216(%rsp)
	cmpq	80(%rsp), %r14
	jne	.LBB350_51
.Ltmp14273:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	80(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp14274:
	jmp	.LBB350_50
.LBB350_62:
	movl	$1, %r14d
.LBB350_63:
	movq	64(%rsp), %rbp
	movq	88(%rsp), %r15
	movq	80(%rsp), %rax
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %r13
	shlq	$4, %r14
	movq	1136(%rbp), %rbx
	movq	%r15, 208(%rsp)
	movq	%rax, 224(%rsp)
	addq	%r15, %r14
	addq	$16, %r15
	movq	%r14, 232(%rsp)
	movq	%rbx, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB350_65
	.p2align	4
.LBB350_64:
	vmovups	80(%rsp), %ymm0
	vmovups	96(%rsp), %ymm1
	movq	1128(%rbp), %rcx
	leaq	-16(%r15), %rax
	incq	%rbx
	addq	$16, %r15
	addq	$16, %rax
	vmovups	%ymm1, 16(%rcx,%r12)
	vmovups	%ymm0, (%rcx,%r12)
	addq	$48, %r12
	movq	%rbx, 1136(%rbp)
	cmpq	%r14, %rax
	je	.LBB350_67
.LBB350_65:
	vmovups	-16(%r15), %xmm0
	movq	8(%rsp), %rax
	vmovups	%xmm0, 104(%rsp)
	movq	$2, 80(%rsp)
	movb	$0, 120(%rsp)
	cmpq	(%rax), %rbx
	jne	.LBB350_64
.Ltmp14284:
	movq	8(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp14285:
	jmp	.LBB350_64
.LBB350_67:
	movq	24(%rsp), %r13
.LBB350_68:
	movq	%r14, 216(%rsp)
.Ltmp14292:
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14293:
	movq	72(%rsp), %rbx
	movq	200(%rsp), %r15
	movq	368(%rsp), %r14
.Ltmp14294:
	movq	360(%rsp), %rcx
	leaq	80(%rsp), %rdi
	movq	%rbp, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14295:
	movq	1136(%rbp), %rsi
	subq	%r14, %rsi
	jb	.LBB350_72
	movq	352(%rsp), %rdi
	addq	1128(%rbp), %rdi
	movq	%r14, 1136(%rbp)
.Ltmp14296:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp14297:
.LBB350_72:
	vmovups	128(%rsp), %zmm1
	vmovups	80(%rsp), %zmm0
	vmovups	%zmm1, 672(%rsp)
	vmovups	%zmm0, 624(%rsp)
	cmpl	$1, 624(%rsp)
	je	.LBB350_13
.LBB350_73:
	leaq	632(%rsp), %rcx
.Ltmp14310:
	leaq	80(%rsp), %rdi
	leaq	520(%rsp), %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp14311:
	cmpq	$-1, 80(%rsp)
	je	.LBB350_105
	vmovups	80(%rsp), %ymm0
	vmovups	%ymm0, 480(%rsp)
.Ltmp14312:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp14313:
	vmovups	560(%rsp), %zmm1
	vmovups	520(%rsp), %zmm0
	movq	%rax, 376(%rsp)
	movq	%rax, 32(%rsp)
	vmovups	%zmm1, 248(%rsp)
	vmovups	%zmm0, 208(%rsp)
	movq	32(%rax), %rcx
	movq	%rcx, %r13
	shlq	$4, %r13
	movq	%rcx, 16(%rsp)
	testq	%rcx, %rcx
	je	.LBB350_114
	movq	24(%rax), %r15
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
	movq	%rax, 8(%rsp)
	testq	%rax, %rax
	je	.LBB350_159
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r13, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %r13
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%r13, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB350_80
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB350_80:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_86
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_80
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%r13, (%rdx)
	movq	%rcx, %rdx
	lock		xaddq	%rdx, (%rsi)
	movabsq	$-9223372036854775808, %rsi
	leaq	(%rdx,%rcx), %rax
	sarq	$63, %rax
	xorq	%rax, %rsi
	addq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rsi, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB350_83:
	cmpq	%rax, %rdx
	jle	.LBB350_85
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB350_83
.LBB350_85:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_86:
	movq	504(%rsp), %rbx
	movq	16(%rsp), %r12
	xorl	%r14d, %r14d
	addq	$16, %rbx
	.p2align	4
.LBB350_87:
	leaq	(%r15,%r14), %rsi
.Ltmp14315:
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)
.Ltmp14316:
	movq	8(%rsp), %rcx
	movq	%rax, (%rcx,%r14)
	movq	%rdx, 8(%rcx,%r14)
	addq	$16, %r14
	decq	%r12
	jne	.LBB350_87
	movq	496(%rsp), %r15
	movq	%r13, 24(%rsp)
	testq	%r15, %r15
	je	.LBB350_115
.LBB350_90:
	movq	malloc@GOTPCREL(%rip), %r14
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,4), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%r14
	testq	%rax, %rax
	je	.LBB350_160
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	leaq	(%rbx,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%rbx, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%rbx, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB350_93
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB350_93:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_99
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_93
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbx, (%rdx)
	movq	%rbx, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%rbx), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%rbx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB350_96:
	cmpq	%rax, %rdx
	jle	.LBB350_98
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB350_96
.LBB350_98:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_99:
	movq	8(%rsp), %rax
	movq	24(%rsp), %rcx
	movq	%r15, 40(%rsp)
	movq	488(%rsp), %r15
	leaq	384(%rsp), %r12
	movq	%r13, 48(%rsp)
	movq	$0, 56(%rsp)
	xorl	%r14d, %r14d
	xorl	%ebp, %ebp
	addq	%rcx, %rax
	movq	%rax, (%rsp)
	jmp	.LBB350_101
	.p2align	4
.LBB350_100:
	movq	112(%rsp), %rax
	incq	%rbp
	movq	%rax, 32(%r13,%r14)
	vmovups	80(%rsp), %ymm0
	vmovups	%ymm0, (%r13,%r14)
	addq	$40, %r14
	movq	%rbp, 56(%rsp)
	cmpq	%r14, %rbx
	je	.LBB350_116
.LBB350_101:
	movq	8(%rsp), %rcx
	movq	(%rsp), %rdx
	leaq	(%r15,%r14), %rax
	movq	%rcx, 384(%rsp)
	movq	%rdx, 392(%rsp)
	movq	%rax, 400(%rsp)
.Ltmp14321:
	leaq	80(%rsp), %rdi
	movq	%r12, %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::from_iter::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>
.Ltmp14322:
	cmpq	40(%rsp), %rbp
	jne	.LBB350_100
.Ltmp14324:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	40(%rsp), %rdi
	callq	*%rax
.Ltmp14325:
	movq	48(%rsp), %r13
	jmp	.LBB350_100
.LBB350_105:
.Ltmp14340:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14341:
	vmovups	560(%rsp), %zmm1
	vmovups	520(%rsp), %zmm0
	movq	%rax, 408(%rsp)
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
	movq	$0, 400(%rsp)
	vmovups	%zmm1, 120(%rsp)
	vmovups	%zmm0, 80(%rsp)
	cmpq	$-1, 80(%rsp)
	je	.LBB350_143
	leaq	208(%rsp), %rdi
	leaq	384(%rsp), %rsi
	leaq	520(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB350_144
	jmp	.LBB350_153
.LBB350_108:
	movq	(%rsp), %rax
	movl	$24, %ebx
	leaq	80(%rsp), %rbp
	jmp	.LBB350_110
	.p2align	4
.LBB350_109:
	addq	$48, %r12
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %rax
	movq	%r14, 96(%rsp)
	cmpq	16(%rsp), %r12
	je	.LBB350_63
.LBB350_110:
	movq	24(%rax), %r13
	movq	32(%rax), %r15
	lock		incq	(%r13)
	jle	.LBB350_161
	movq	%r13, 208(%rsp)
	movq	%rax, %r12
	movq	%r15, 216(%rsp)
	cmpq	80(%rsp), %r14
	jne	.LBB350_109
.Ltmp14276:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp14277:
	movq	88(%rsp), %rsi
	jmp	.LBB350_109
.LBB350_114:
	movl	$8, %eax
	movq	%rax, 8(%rsp)
	movq	496(%rsp), %r15
	movq	%r13, 24(%rsp)
	testq	%r15, %r15
	jne	.LBB350_90
.LBB350_115:
	movq	$0, 40(%rsp)
	movq	$8, 48(%rsp)
	movq	$0, 56(%rsp)
.LBB350_116:
	vmovups	248(%rsp), %zmm1
	vmovups	208(%rsp), %zmm0
	movq	40(%rsp), %rcx
	movq	56(%rsp), %rax
	movq	48(%rsp), %rdx
	movq	%rcx, 320(%rsp)
	movq	32(%rsp), %rcx
	movq	%rax, 336(%rsp)
	movq	%rdx, 328(%rsp)
	vmovups	%zmm1, 120(%rsp)
	vmovups	%zmm0, 80(%rsp)
	cmpq	$-1, 80(%rsp)
	movq	%rcx, 344(%rsp)
	je	.LBB350_118
	leaq	384(%rsp), %rdi
	leaq	320(%rsp), %rsi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB350_119
.LBB350_118:
	movq	328(%rsp), %rcx
	movq	320(%rsp), %rax
	movq	336(%rsp), %rdx
	movq	%rcx, 400(%rsp)
	movq	344(%rsp), %rcx
	movq	%rax, 392(%rsp)
	movq	%rdx, 408(%rsp)
	movq	%rcx, 416(%rsp)
	movq	$-1, 384(%rsp)
.LBB350_119:
	movq	72(%rsp), %rbx
	movq	24(%rsp), %r14
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB350_129
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	160(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB350_122
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB350_122:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_128
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_122
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB350_125:
	cmpq	%rax, %rsi
	jge	.LBB350_127
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB350_125
.LBB350_127:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_128:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB350_129:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB350_132
	lock		decq	(%rax)
	jne	.LBB350_132
	leaq	176(%rsp), %rdi
	#MEMBARRIER
.Ltmp14330:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp14331:
.LBB350_132:
	vmovups	416(%rsp), %zmm1
	vmovups	384(%rsp), %zmm0
	cmpq	$0, 16(%rsp)
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	je	.LBB350_142
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %r14
	cmovaeq	%rcx, %r14
	xorl	%edx, %edx
	cmpq	%r14, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%r14, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB350_135
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB350_135:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_141
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_135
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r14, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r14, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB350_138:
	cmpq	%rax, %rdx
	jge	.LBB350_140
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB350_138
.LBB350_140:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_141:
	movq	free@GOTPCREL(%rip), %rax
	movq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB350_142:
	leaq	480(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB350_157
.LBB350_143:
	movq	392(%rsp), %rcx
	movq	384(%rsp), %rax
	movq	400(%rsp), %rdx
	movq	%rcx, 224(%rsp)
	movq	408(%rsp), %rcx
	movq	%rax, 216(%rsp)
	movq	%rdx, 232(%rsp)
	movq	%rcx, 240(%rsp)
	movq	$-1, 208(%rsp)
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB350_153
.LBB350_144:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	160(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB350_146
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB350_146:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB350_152
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB350_146
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB350_149:
	cmpq	%rax, %rsi
	jge	.LBB350_151
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB350_149
.LBB350_151:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB350_152:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB350_153:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB350_156
	lock		decq	(%rax)
	jne	.LBB350_156
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB350_156:
	vmovups	240(%rsp), %zmm1
	vmovups	208(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB350_157:
	movq	%rbx, %rax
	addq	$744, %rsp
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
	vzeroupper
	retq
.LBB350_158:
	.cfi_def_cfa_offset 800
.Ltmp14302:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp14303:
	jmp	.LBB350_161
.LBB350_159:
.Ltmp14318:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp14319:
	jmp	.LBB350_161
.LBB350_160:
.Ltmp14327:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp14328:
.LBB350_161:
	ud2
.LBB350_162:
.Ltmp14332:
	movq	%rax, %rbx
	movb	$1, %bpl
	jmp	.LBB350_182
.LBB350_163:
.Ltmp14329:
	movq	%rax, %rbx
	jmp	.LBB350_181
.LBB350_164:
.Ltmp14320:
	movq	%rax, %rbx
	jmp	.LBB350_187
.LBB350_165:
.Ltmp14298:
	movq	%rax, %rbx
.Ltmp14299:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp14300:
	jmp	.LBB350_197
.LBB350_166:
.Ltmp14314:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB350_192
.LBB350_167:
.Ltmp14304:
	lock		decq	(%rbp)
	movq	%rax, %rbx
	jne	.LBB350_197
	#MEMBARRIER
.Ltmp14305:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	callq	*%rax
.Ltmp14306:
	jmp	.LBB350_197
.LBB350_169:
.Ltmp14275:
	jmp	.LBB350_171
.LBB350_170:
.Ltmp14278:
.LBB350_171:
	lock		decq	(%r13)
	movq	%rax, %rbx
	jne	.LBB350_173
	#MEMBARRIER
.Ltmp14279:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	callq	*%rax
.Ltmp14280:
.LBB350_173:
.Ltmp14282:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14283:
	jmp	.LBB350_197
.LBB350_174:
.Ltmp14281:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB350_175:
.Ltmp14307:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB350_176:
.Ltmp14342:
	movq	%rax, %rbx
	jmp	.LBB350_197
.LBB350_177:
.Ltmp14326:
	movq	%rax, %rbx
	movq	80(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB350_180
	movq	88(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB350_180
.LBB350_179:
.Ltmp14323:
	movq	%rax, %rbx
.LBB350_180:
	leaq	40(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB350_181:
	xorl	%ebp, %ebp
.LBB350_182:
	cmpq	$0, 16(%rsp)
	je	.LBB350_184
	movq	8(%rsp), %rdi
	movq	24(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB350_184:
	testb	%bpl, %bpl
	je	.LBB350_187
	jmp	.LBB350_191
.LBB350_186:
.Ltmp14317:
	movq	8(%rsp), %rdi
	movl	$8, %edx
	movq	%r13, %rsi
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
.LBB350_187:
.Ltmp14333:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14334:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB350_191
	#MEMBARRIER
.Ltmp14335:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	376(%rsp), %rdi
	callq	*%rax
.Ltmp14336:
.LBB350_191:
	xorl	%ebp, %ebp
.LBB350_192:
.Ltmp14338:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp14339:
	testb	%bpl, %bpl
	jne	.LBB350_197
	jmp	.LBB350_198
.LBB350_194:
.Ltmp14337:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB350_195:
.Ltmp14286:
	movq	%rax, %rbx
	movq	%r15, 216(%rsp)
.Ltmp14287:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp14288:
.Ltmp14290:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14291:
.LBB350_197:
.Ltmp14343:
	leaq	520(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14344:
.LBB350_198:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB350_199:
.Ltmp14301:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB350_200:
.Ltmp14289:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB350_201:
.Ltmp14345:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end350:
purrdf_sparql_eval::modifier::eval_dedup_with::<purrdf_core::ir::dataset::RdfDataset, false>:
.Lfunc_begin360:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception267
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
	subq	$760, %rsp
	.cfi_def_cfa_offset 816
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	504(%rsp), %rdi
	movq	%rcx, %r14
	movq	%rdx, %r15
	callq	*%rax
.Ltmp15031:
	leaq	640(%rsp), %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15032:
	cmpl	$1, 640(%rsp)
	jne	.LBB360_25
	vmovdqu64	688(%rsp), %zmm1
	vmovdqu64	656(%rsp), %zmm0
	movq	576(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rbx)
	vmovdqu64	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB360_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	584(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_5
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB360_8:
	cmpq	%rax, %rsi
	jge	.LBB360_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_8
.LBB360_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_12:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB360_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	512(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_15
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB360_18:
	cmpq	%rax, %rsi
	jge	.LBB360_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_18
.LBB360_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_22:
	movq	600(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_267
	lock		decq	(%rax)
	jne	.LBB360_267
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	600(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB360_267
.LBB360_25:
	vmovdqu64	680(%rsp), %zmm1
	vmovdqu64	648(%rsp), %zmm0
	vmovdqu64	%zmm1, 384(%rsp)
	vmovdqu64	%zmm0, 352(%rsp)
.Ltmp15033:
	leaq	112(%rsp), %rdi
	leaq	504(%rsp), %rsi
	leaq	352(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15034:
	cmpq	$-1, 112(%rsp)
	je	.LBB360_126
	vmovdqu	112(%rsp), %ymm0
	vmovdqu64	544(%rsp), %zmm1
	vmovdqu64	504(%rsp), %zmm2
	vmovdqu64	%zmm1, 392(%rsp)
	vmovdqu	%ymm0, 608(%rsp)
	vmovdqu64	%zmm2, 352(%rsp)
.Ltmp15038:
	leaq	320(%rsp), %rdi
	leaq	608(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::blank_scope::without_joined_blanks::<purrdf_core::ir::term::TermId>
.Ltmp15039:
	movq	336(%rsp), %r15
	movq	%rbx, 280(%rsp)
.Ltmp15041:
	leaq	112(%rsp), %rdi
	movl	$48, %esi
	movl	$1, %ecx
	movq	%r15, %rdx
	callq	<hashbrown::raw::RawTableInner>::fallible_with_capacity::<alloc::alloc::Global>
.Ltmp15042:
	vmovdqu	112(%rsp), %ymm0
	movq	328(%rsp), %rbp
	movq	320(%rsp), %rcx
	leaq	(%r15,%r15,4), %rax
	leaq	(%rbp,%rax,8), %rax
	movq	%rbp, 112(%rsp)
	movq	%rcx, 128(%rsp)
	movq	%rcx, 208(%rsp)
	movq	%rbp, (%rsp)
	movq	%rax, 24(%rsp)
	movq	%rax, 136(%rsp)
	vmovdqu	%ymm0, 240(%rsp)
	testq	%r15, %r15
	je	.LBB360_99
	movq	(%rsp), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB360_34
.LBB360_31:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_32:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_33:
	cmpq	24(%rsp), %rbp
	je	.LBB360_112
.LBB360_34:
	movq	%rbp, %rax
	movq	(%rax), %rbx
	addq	$40, %rbp
	testq	%rbx, %rbx
	je	.LBB360_99
	vmovups	8(%rax), %ymm0
	leaq	472(%rsp), %rdx
	leaq	-1(%rbx), %rcx
	movabsq	$2746377873070565055, %rsi
	cmpq	$5, %rcx
	vmovups	%ymm0, (%rdx)
	movq	%rbx, 464(%rsp)
	vpbroadcastq	.LCPI360_6(%rip), %xmm0
	movq	480(%rsp), %rax
	movq	472(%rsp), %r13
	movq	%rax, 40(%rsp)
	leaq	-1(%rax), %rax
	cmovbq	%rcx, %rax
	movq	%rdx, %rcx
	cmovaeq	%r13, %rcx
	movq	%rax, %rdx
	xorq	%rsi, %rdx
	vpinsrq	$0, %rdx, %xmm0, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	testq	%rax, %rax
	je	.LBB360_43
	leaq	(,%rax,8), %rsi
	addq	$-8, %rsi
	movl	%esi, %edi
	shrl	$3, %edi
	incl	%edi
	andl	$3, %edi
	je	.LBB360_41
	shll	$3, %edi
	movq	%rcx, %rdx
	jmp	.LBB360_39
	.p2align	4
.LBB360_38:
	addq	$8, %rdx
	addq	$-8, %rdi
	je	.LBB360_42
.LBB360_39:
	movl	(%rdx), %r8d
	xorl	%r9d, %r9d
	cmpq	$2, %r8
	setne	%r9b
	vmovd	%r9d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_38
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	4(%rdx), %r9d
	vmovq	%r8, %xmm1
	orl	$-2, %r8d
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%r8,%r9), %r8d
	vmovd	%r8d, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB360_38
	.p2align	4
.LBB360_41:
	movq	%rcx, %rdx
.LBB360_42:
	cmpq	$24, %rsi
	jae	.LBB360_84
.LBB360_43:
	vaesenc	.LCPI360_2(%rip), %xmm0, %xmm0
	leaq	1(%r15), %rdx
	movq	248(%rsp), %rsi
	movq	%rdx, 8(%rsp)
	movq	240(%rsp), %rdx
	vaesenc	.LCPI360_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %r12
	movq	%r12, %r14
	shrq	$57, %r14
	vpbroadcastb	%r14d, %xmm0
	testq	%rax, %rax
	je	.LBB360_62
	xorl	%edi, %edi
	movq	%r12, %r8
.LBB360_45:
	andq	%rsi, %r8
	vmovdqu	(%rdx,%r8), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB360_60
	kmovd	%k0, %r9d
	movq	%r13, 16(%rsp)
	movq	%rdi, 32(%rsp)
.LBB360_47:
	xorl	%edi, %edi
	tzcntl	%r9d, %edi
	addq	%r8, %rdi
	andq	%rsi, %rdi
	negq	%rdi
	leaq	(%rdi,%rdi,2), %rdi
	shlq	$4, %rdi
	movq	-48(%rdx,%rdi), %r11
	decq	%r11
	cmpq	$5, %r11
	jb	.LBB360_49
	movq	-32(%rdx,%rdi), %r11
	movq	-40(%rdx,%rdi), %r10
	decq	%r11
	jmp	.LBB360_50
	.p2align	4
.LBB360_49:
	leaq	-40(%rdx,%rdi), %r10
.LBB360_50:
	cmpq	%rax, %r11
	jne	.LBB360_59
	xorl	%r11d, %r11d
	jmp	.LBB360_53
	.p2align	4
.LBB360_52:
	incq	%r11
	cmpq	%r11, %rax
	je	.LBB360_71
.LBB360_53:
	movl	(%r10,%r11,8), %r13d
	movl	(%rcx,%r11,8), %edi
	cmpl	$2, %r13d
	je	.LBB360_57
	cmpl	$2, %edi
	je	.LBB360_57
	cmpl	%edi, %r13d
	jne	.LBB360_59
	movl	4(%rcx,%r11,8), %edi
	cmpl	%edi, 4(%r10,%r11,8)
	je	.LBB360_52
	jmp	.LBB360_59
	.p2align	4
.LBB360_57:
	cmpl	$2, %r13d
	jne	.LBB360_59
	cmpl	$2, %edi
	je	.LBB360_52
.LBB360_59:
	leal	-1(%r9), %edi
	movq	16(%rsp), %r13
	andw	%r9w, %di
	movl	%edi, %r9d
	movq	32(%rsp), %rdi
	jne	.LBB360_47
	.p2align	4
.LBB360_60:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB360_80
	leaq	16(%r8,%rdi), %r8
	addq	$16, %rdi
	jmp	.LBB360_45
	.p2align	4
.LBB360_62:
	xorl	%eax, %eax
	movq	%r12, %rcx
.LBB360_63:
	andq	%rsi, %rcx
	vmovdqu	(%rdx,%rcx), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB360_69
	kmovd	%k0, %edi
	movq	%r13, 16(%rsp)
.LBB360_65:
	xorl	%r8d, %r8d
	tzcntl	%edi, %r8d
	addq	%rcx, %r8
	andq	%rsi, %r8
	negq	%r8
	leaq	(%r8,%r8,2), %r8
	shlq	$4, %r8
	movq	-48(%rdx,%r8), %r9
	cmpq	$6, %r9
	jb	.LBB360_67
	movq	-32(%rdx,%r8), %r9
.LBB360_67:
	cmpq	$1, %r9
	je	.LBB360_71
	movq	16(%rsp), %r13
	leal	-1(%rdi), %r8d
	andw	%di, %r8w
	movl	%r8d, %edi
	jne	.LBB360_65
	.p2align	4
.LBB360_69:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB360_80
	leaq	16(%rcx,%rax), %rcx
	addq	$16, %rax
	jmp	.LBB360_63
	.p2align	4
.LBB360_71:
	movq	8(%rsp), %r15
	cmpq	$6, %rbx
	jb	.LBB360_33
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	leaq	-8(,%rbx,8), %rcx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_74
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB360_74:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r8
	movq	16(%rsp), %rdi
	.p2align	4
.LBB360_75:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_32
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_75
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r8), %rax
	.p2align	4
.LBB360_78:
	cmpq	%rax, %rdx
	jge	.LBB360_31
	lock		cmpxchgq	%rdx, (%r8)
	jne	.LBB360_78
	jmp	.LBB360_31
	.p2align	4
.LBB360_80:
	cmpq	$0, 256(%rsp)
	je	.LBB360_96
.LBB360_81:
	andq	%rsi, %r12
	vmovdqu	(%rdx,%r12), %xmm0
	vpmovmskb	%xmm0, %eax
	testl	%eax, %eax
	je	.LBB360_94
.LBB360_82:
	tzcntl	%eax, %eax
	addq	%r12, %rax
	andq	%rsi, %rax
	movzbl	(%rdx,%rax), %ecx
	testb	%cl, %cl
	jns	.LBB360_98
.LBB360_83:
	movq	40(%rsp), %r8
	leaq	-16(%rax), %rdi
	movb	%r14b, (%rdx,%rax)
	negq	%rax
	vpbroadcastb	.LCPI360_7(%rip), %xmm1
	andb	$1, %cl
	leaq	(%rax,%rax,2), %rax
	andq	%rsi, %rdi
	movzbl	%cl, %ecx
	movb	%r14b, 16(%rdx,%rdi)
	leaq	472(%rsp), %rdi
	shlq	$4, %rax
	movq	%rbx, -48(%rdx,%rax)
	movq	%r13, -40(%rdx,%rax)
	movq	%r8, -32(%rdx,%rax)
	vmovups	16(%rdi), %xmm0
	vpinsrq	$0, %rcx, %xmm1, %xmm1
	vmovups	%xmm0, -24(%rdx,%rax)
	movq	%r15, -8(%rdx,%rax)
	movq	8(%rsp), %r15
	vmovdqa	256(%rsp), %xmm0
	vpsubq	%xmm1, %xmm0, %xmm0
	vmovdqa	%xmm0, 256(%rsp)
	jmp	.LBB360_33
	.p2align	4
.LBB360_84:
	leaq	(%rcx,%rax,8), %rsi
	jmp	.LBB360_86
	.p2align	4
.LBB360_85:
	addq	$32, %rdx
	cmpq	%rsi, %rdx
	je	.LBB360_43
.LBB360_86:
	movl	(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_88
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	4(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB360_88:
	movl	8(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_90
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	12(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB360_90:
	movl	16(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_92
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	20(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB360_92:
	movl	24(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_85
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	28(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB360_85
.LBB360_94:
	movl	$16, %ecx
.LBB360_95:
	addq	%rcx, %r12
	addq	$16, %rcx
	andq	%rsi, %r12
	vmovdqu	(%rdx,%r12), %xmm0
	vpmovmskb	%xmm0, %eax
	testl	%eax, %eax
	jne	.LBB360_82
	jmp	.LBB360_95
.LBB360_96:
.Ltmp15044:
	movq	<hashbrown::raw::RawTable<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize)>>::reserve_rehash::<hashbrown::map::make_hasher<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %esi
	leaq	240(%rsp), %rdi
	movl	$1, %ecx
	vzeroupper
	callq	*%rax
.Ltmp15045:
	movq	240(%rsp), %rdx
	movq	248(%rsp), %rsi
	jmp	.LBB360_81
.LBB360_98:
	vmovdqa	(%rdx), %xmm0
	vpmovmskb	%xmm0, %eax
	tzcntl	%eax, %eax
	movzbl	(%rdx,%rax), %ecx
	jmp	.LBB360_83
.LBB360_99:
	subq	%rbp, 24(%rsp)
	je	.LBB360_112
	movq	24(%rsp), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$-3689348814741910323, %rax
	movabsq	$9223372036854775807, %r14
	xorl	%ebx, %ebx
	shrq	$3, %r15
	imulq	%rax, %r15
	jmp	.LBB360_104
	.p2align	4
.LBB360_101:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_102:
	vzeroupper
	callq	*%r13
.LBB360_103:
	incq	%rbx
	cmpq	%r15, %rbx
	je	.LBB360_112
.LBB360_104:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB360_103
	leaq	(%rbp,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r14, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r14, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r14, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_107
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_107:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_107
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r14, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB360_110:
	cmpq	%rax, %rdx
	jge	.LBB360_101
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB360_110
	jmp	.LBB360_101
.LBB360_112:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_122
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_115
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_115:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_121
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_115
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB360_118:
	cmpq	%rax, %rsi
	jge	.LBB360_120
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_118
.LBB360_120:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_121:
	movq	free@GOTPCREL(%rip), %rax
	movq	(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB360_122:
	movq	240(%rsp), %r15
	movq	248(%rsp), %rdi
	movq	264(%rsp), %rax
	testq	%rdi, %rdi
	je	.LBB360_127
	movq	%rdi, %rcx
	shlq	$4, %rcx
	movq	%r15, %rdx
	movl	$16, %r8d
	leaq	(%rcx,%rcx,2), %rcx
	subq	%rcx, %rdx
	leaq	65(%rdi,%rcx), %rsi
	addq	$-48, %rdx
	movq	%rdx, 32(%rsp)
	movq	%rsi, 40(%rsp)
	movq	%rdi, 8(%rsp)
	testq	%rax, %rax
	je	.LBB360_128
.LBB360_124:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r15), %xmm0, %k0
	leaq	16(%r15), %r14
	kortestw	%k0, %k0
	je	.LBB360_141
	kmovd	%k0, %ecx
	movq	%r15, %rbp
	jmp	.LBB360_144
.LBB360_126:
	leaq	8(%rbx), %rdi
	leaq	504(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	jmp	.LBB360_266
.LBB360_127:
	xorl	%r8d, %r8d
	movq	%rsi, 40(%rsp)
	movq	%rdi, 8(%rsp)
	testq	%rax, %rax
	jne	.LBB360_124
.LBB360_128:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
.LBB360_129:
	cmpq	$0, 8(%rsp)
	movq	40(%rsp), %rsi
	movabsq	$-3689348814741910323, %r15
	je	.LBB360_140
	testq	%rsi, %rsi
	je	.LBB360_140
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_133
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_133:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_139
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_133
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%rsi, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB360_136:
	cmpq	%rax, %rdx
	jge	.LBB360_138
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_136
.LBB360_138:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_139:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB360_140:
	movl	$8, %r13d
	xorl	%r12d, %r12d
	jmp	.LBB360_214
.LBB360_141:
	movq	%r15, %rbp
	.p2align	4
.LBB360_142:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_142
	kmovd	%k0, %ecx
.LBB360_144:
	xorl	%r12d, %r12d
	blsrl	%ecx, %r12d
	tzcntl	%ecx, %ecx
	leaq	-1(%rax), %rbx
	negq	%rcx
	leaq	(%rcx,%rcx,2), %rcx
	shlq	$4, %rcx
	movq	-48(%rbp,%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB360_148
	addq	%rbp, %rcx
	cmpq	$5, %rax
	movq	%rdx, 24(%rsp)
	movabsq	$192153584101141163, %rdx
	movq	-40(%rcx), %rsi
	movq	-8(%rcx), %r9
	movq	%rsi, 16(%rsp)
	movq	-16(%rcx), %rsi
	movq	%rsi, 304(%rsp)
	movl	$4, %esi
	vmovdqu	-32(%rcx), %xmm0
	cmovaeq	%rax, %rsi
	decq	%rdx
	movq	%rsi, %rcx
	shlq	$4, %rcx
	leaq	(%rcx,%rcx,2), %r13
	vmovdqa	%xmm0, 288(%rsp)
	cmpq	%rdx, %rax
	jbe	.LBB360_164
	xorl	%edi, %edi
.LBB360_147:
.Ltmp15053:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15054:
	jmp	.LBB360_293
.LBB360_148:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
	testq	%rbx, %rbx
	je	.LBB360_129
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB360_153
	.p2align	4
.LBB360_150:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_151:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB360_152:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB360_129
.LBB360_153:
	testw	%r12w, %r12w
	jne	.LBB360_156
	.p2align	4
.LBB360_154:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_154
	kmovd	%k0, %r12d
.LBB360_156:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_152
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	leaq	-8(,%rcx,8), %rcx
	addq	%rbp, %rax
	movq	-40(%rax), %rdi
	cmpq	%r15, %rcx
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rdx
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rdx
	cmovoq	%rsi, %rdx
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_159
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_151
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_159
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB360_162:
	cmpq	%rax, %rdx
	jge	.LBB360_150
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_162
	jmp	.LBB360_150
.LBB360_164:
	testq	%r13, %r13
	je	.LBB360_175
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rsi, 312(%rsp)
	movq	%r9, 208(%rsp)
	movq	%r8, (%rsp)
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB360_295
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	movabsq	$9223372036854775807, %rsi
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r13, %rax
	cmovbq	%rdx, %rax
	cmpq	%rsi, %r13
	movq	%rsi, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%r13, %rdx
	addq	%rdx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB360_168
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB360_168:
	movq	(%rsp), %r8
	movq	208(%rsp), %r9
	movq	312(%rsp), %r10
	.p2align	4
.LBB360_169:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_176
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_169
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	%r13, (%rsi)
	movq	%rdx, %rsi
	lock		xaddq	%rsi, (%rdi)
	movabsq	$-9223372036854775808, %rdi
	leaq	(%rsi,%rdx), %rax
	sarq	$63, %rax
	xorq	%rax, %rdi
	addq	%rdx, %rsi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	cmovoq	%rdi, %rsi
	movq	(%rdx), %rax
	.p2align	4
.LBB360_172:
	cmpq	%rax, %rsi
	jle	.LBB360_174
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB360_172
.LBB360_174:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jmp	.LBB360_176
.LBB360_175:
	movl	$8, %ecx
	xorl	%r10d, %r10d
.LBB360_176:
	movq	24(%rsp), %rdx
	movq	16(%rsp), %rdi
	movq	%r9, (%rcx)
	movq	40(%rsp), %rsi
	movq	8(%rsp), %rax
	movq	%rdx, 8(%rcx)
	movq	%rdi, 16(%rcx)
	leaq	1(%r15,%rax), %rax
	vmovdqa	288(%rsp), %xmm0
	vmovdqu	%xmm0, 24(%rcx)
	movq	304(%rsp), %rdx
	movq	%rdx, 40(%rcx)
	movq	%r8, 112(%rsp)
	movq	%rsi, 120(%rsp)
	movq	32(%rsp), %rsi
	movq	%r10, 216(%rsp)
	movq	%rcx, 224(%rsp)
	movq	$1, 232(%rsp)
	movq	%rsi, 128(%rsp)
	movq	%rbp, 136(%rsp)
	movq	%r14, 144(%rsp)
	movq	%rax, 152(%rsp)
	movw	%r12w, 160(%rsp)
	movq	%rbx, 168(%rsp)
	testq	%rbx, %rbx
	je	.LBB360_202
	movl	$1, %r15d
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	jmp	.LBB360_179
	.p2align	4
.LBB360_178:
	leaq	(%r15,%r15,2), %rax
	incq	%r15
	shlq	$4, %rax
	movq	%rdi, (%rcx,%rax)
	movq	%r13, 8(%rcx,%rax)
	movq	%rsi, 16(%rcx,%rax)
	vmovdqa	80(%rsp), %xmm0
	vmovdqu	%xmm0, 24(%rcx,%rax)
	movq	96(%rsp), %rdx
	movq	%rdx, 40(%rcx,%rax)
	movq	%r15, 232(%rsp)
	testq	%rbx, %rbx
	je	.LBB360_202
.LBB360_179:
	testw	%r12w, %r12w
	jne	.LBB360_182
	.p2align	4
.LBB360_180:
	vpcmpltb	(%r14), %xmm1, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_180
	kmovd	%k0, %r12d
.LBB360_182:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	decq	%rbx
	blsrl	%r12d, %r12d
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %r13
	testq	%r13, %r13
	je	.LBB360_186
	addq	%rbp, %rax
	movq	-16(%rax), %rdx
	movq	-8(%rax), %rdi
	movq	-40(%rax), %rsi
	movq	%rdx, 96(%rsp)
	vmovups	-32(%rax), %xmm0
	vmovaps	%xmm0, 80(%rsp)
	cmpq	216(%rsp), %r15
	jne	.LBB360_178
	movq	%rbx, %rdx
	incq	%rdx
	movq	$-1, %rax
	movq	%rdi, 16(%rsp)
	movq	%rsi, 24(%rsp)
	cmoveq	%rax, %rdx
.Ltmp15047:
	movl	$8, %ecx
	movl	$48, %r8d
	leaq	216(%rsp), %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15048:
	movq	224(%rsp), %rcx
	movq	24(%rsp), %rsi
	movq	16(%rsp), %rdi
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	jmp	.LBB360_178
.LBB360_186:
	movq	%r14, 144(%rsp)
	movq	%rbp, 136(%rsp)
	testq	%rbx, %rbx
	je	.LBB360_202
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB360_191
.LBB360_188:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_189:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB360_190:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB360_202
.LBB360_191:
	testw	%r12w, %r12w
	jne	.LBB360_194
	.p2align	4
.LBB360_192:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_192
	kmovd	%k0, %r12d
.LBB360_194:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_190
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	leaq	-8(,%rcx,8), %rcx
	addq	%rbp, %rax
	movq	-40(%rax), %rdi
	cmpq	%r15, %rcx
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rdx
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rdx
	cmovoq	%rsi, %rdx
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_197
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_197:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_189
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_197
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB360_200:
	cmpq	%rax, %rdx
	jge	.LBB360_188
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_200
	jmp	.LBB360_188
.LBB360_202:
	cmpq	$0, 8(%rsp)
	movq	40(%rsp), %rsi
	movabsq	$-3689348814741910323, %r15
	je	.LBB360_213
	testq	%rsi, %rsi
	je	.LBB360_213
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_206
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_206:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_212
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_206
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%rsi, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB360_209:
	cmpq	%rax, %rdx
	jge	.LBB360_211
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_209
.LBB360_211:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_212:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB360_213:
	vmovdqu	216(%rsp), %xmm0
	movq	232(%rsp), %r12
	movq	%r12, 64(%rsp)
	vmovdqa	%xmm0, 48(%rsp)
	movq	56(%rsp), %r13
	cmpq	$2, %r12
	jae	.LBB360_268
.LBB360_214:
	movq	48(%rsp), %r9
	movq	344(%rsp), %rax
	movq	%r13, %rbx
	movq	%r13, %r14
	movq	%r9, %rdx
	shlq	$4, %rdx
	movq	%rax, 8(%rsp)
	movq	%rax, 288(%rsp)
	movq	%r12, %rax
	shlq	$4, %rax
	leaq	(%rdx,%rdx,2), %rdx
	leaq	(%rax,%rax,2), %rax
	movq	%rdx, 24(%rsp)
	mulxq	%r15, %r8, %r8
	leaq	(%r13,%rax), %rcx
	testq	%r12, %r12
	je	.LBB360_222
	addq	$-48, %rax
	movabsq	$-6148914691236517205, %rsi
	movq	%rax, %rdx
	mulxq	%rsi, %rdx, %rdx
	shrl	$5, %edx
	incl	%edx
	andl	$7, %edx
	je	.LBB360_219
	shll	$3, %edx
	movq	%r13, %r14
	leaq	(%rdx,%rdx,4), %rsi
	movq	%r13, %rdx
	.p2align	4
.LBB360_217:
	vmovdqu	8(%rdx), %ymm0
	movq	40(%rdx), %rdi
	addq	$48, %rdx
	movq	%rdi, 32(%r14)
	vmovdqu	%ymm0, (%r14)
	addq	$40, %r14
	addq	$-40, %rsi
	jne	.LBB360_217
	movq	%rcx, %rbx
	cmpq	$336, %rax
	jae	.LBB360_220
	jmp	.LBB360_222
.LBB360_219:
	movq	%r13, %r14
	movq	%r13, %rdx
	movq	%rcx, %rbx
	cmpq	$336, %rax
	jb	.LBB360_222
	.p2align	4
.LBB360_220:
	vmovups	8(%rdx), %ymm0
	movq	40(%rdx), %rax
	movq	%rax, 32(%r14)
	vmovups	%ymm0, (%r14)
	vmovups	56(%rdx), %ymm0
	movq	88(%rdx), %rax
	movq	%rax, 72(%r14)
	vmovups	%ymm0, 40(%r14)
	vmovups	104(%rdx), %ymm0
	movq	136(%rdx), %rax
	movq	%rax, 112(%r14)
	vmovups	%ymm0, 80(%r14)
	vmovups	152(%rdx), %ymm0
	movq	184(%rdx), %rax
	movq	%rax, 152(%r14)
	vmovups	%ymm0, 120(%r14)
	vmovups	200(%rdx), %ymm0
	movq	232(%rdx), %rax
	movq	%rax, 192(%r14)
	vmovups	%ymm0, 160(%r14)
	vmovups	248(%rdx), %ymm0
	movq	280(%rdx), %rax
	movq	%rax, 232(%r14)
	vmovups	%ymm0, 200(%r14)
	vmovups	296(%rdx), %ymm0
	movq	328(%rdx), %rax
	movq	%rax, 272(%r14)
	vmovups	%ymm0, 240(%r14)
	vmovdqu	344(%rdx), %ymm0
	movq	376(%rdx), %rax
	addq	$384, %rdx
	movq	%rax, 312(%r14)
	vmovdqu	%ymm0, 280(%r14)
	addq	$320, %r14
	cmpq	%rcx, %rdx
	jne	.LBB360_220
	movq	%rcx, %rbx
.LBB360_222:
	vmovdqa	.LCPI360_5(%rip), %ymm0
	subq	%r13, %r14
	shrq	$5, %r8
	movq	%r13, 32(%rsp)
	movq	%r13, 80(%rsp)
	movq	%r9, 16(%rsp)
	shrq	$3, %r14
	movq	%r8, 40(%rsp)
	imulq	%r15, %r14
	subq	%rbx, %rcx
	movq	%r14, 88(%rsp)
	movq	%r9, 96(%rsp)
	vmovdqu	%ymm0, 112(%rsp)
	je	.LBB360_235
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	shrq	$4, %rcx
	movabsq	$-6148914691236517205, %r12
	movabsq	$9223372036854775807, %rbp
	xorl	%r15d, %r15d
	imulq	%rcx, %r12
	jmp	.LBB360_227
	.p2align	4
.LBB360_224:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_225:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_226:
	incq	%r15
	cmpq	%r12, %r15
	je	.LBB360_235
.LBB360_227:
	leaq	(%r15,%r15,2), %rax
	shlq	$4, %rax
	movq	8(%rbx,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_226
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	leaq	-8(,%rcx,8), %rcx
	addq	%rbx, %rax
	movq	16(%rax), %rdi
	cmpq	%rbp, %rcx
	cmovaeq	%rbp, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rdx
	setns	%sil
	addq	%rbp, %rsi
	subq	%rcx, %rdx
	cmovoq	%rsi, %rdx
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_230
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_230:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_225
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_230
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbp, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB360_233:
	cmpq	%rax, %rdx
	jge	.LBB360_224
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_233
	jmp	.LBB360_224
.LBB360_235:
	movq	40(%rsp), %rbx
	cmpq	$0, 16(%rsp)
	leaq	(,%rbx,8), %rcx
	setne	%al
	leaq	(%rcx,%rcx,4), %r13
	movq	24(%rsp), %rcx
	cmpq	%r13, %rcx
	setne	%dl
	andb	%al, %dl
	cmpb	$1, %dl
	jne	.LBB360_248
	cmpq	$39, %rcx
	ja	.LBB360_249
	movl	$8, %r12d
	testq	%rcx, %rcx
	je	.LBB360_250
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_240
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB360_240:
	movq	32(%rsp), %rdi
	.p2align	4
.LBB360_241:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_247
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_241
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB360_244:
	cmpq	%rax, %rdx
	jge	.LBB360_246
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_244
.LBB360_246:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_247:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB360_250
.LBB360_248:
	movq	32(%rsp), %r12
	jmp	.LBB360_250
.LBB360_249:
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rsi
	leaq	qualification_454_native_cost::GLOBAL (.llvm.1577329767756756036)(%rip), %rdi
	movl	$8, %edx
	movq	%r13, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %r12
	testq	%rax, %rax
	je	.LBB360_292
.LBB360_250:
	movq	8(%rsp), %rax
	cmpq	$-1, 352(%rsp)
	movq	%rax, 104(%rsp)
	movq	%rbx, 80(%rsp)
	movq	%r12, 88(%rsp)
	movq	%r14, 96(%rsp)
	je	.LBB360_252
	leaq	112(%rsp), %rdi
	leaq	80(%rsp), %rsi
	leaq	352(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	280(%rsp), %rbx
	movq	424(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB360_253
	jmp	.LBB360_262
.LBB360_252:
	movq	88(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	96(%rsp), %rdx
	movq	%rcx, 128(%rsp)
	movq	104(%rsp), %rcx
	movq	%rax, 120(%rsp)
	movq	%rdx, 136(%rsp)
	movq	%rcx, 144(%rsp)
	movq	$-1, 112(%rsp)
	movq	280(%rsp), %rbx
	movq	424(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB360_262
.LBB360_253:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	432(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_255
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_255:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_261
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_255
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB360_258:
	cmpq	%rax, %rsi
	jge	.LBB360_260
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_258
.LBB360_260:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_261:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_262:
	movq	448(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_265
	lock		decq	(%rax)
	jne	.LBB360_265
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	448(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB360_265:
	vmovdqu64	144(%rsp), %zmm1
	vmovdqu64	112(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
.LBB360_266:
	movq	$0, (%rbx)
.LBB360_267:
	movq	%rbx, %rax
	addq	$760, %rsp
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
	vzeroupper
	retq
.LBB360_268:
	.cfi_def_cfa_offset 816
	cmpq	$21, %r12
	jae	.LBB360_294
	shlq	$4, %r12
	movabsq	$-6148914691236517205, %rcx
	leaq	48(%r13), %rax
	leaq	-96(%r12,%r12,2), %rdx
	mulxq	%rcx, %rcx, %rcx
	btl	$5, %ecx
	jb	.LBB360_273
	movq	48(%r13), %rcx
	cmpq	(%r13), %rcx
	jae	.LBB360_272
	movq	88(%r13), %rsi
	movq	%rsi, 144(%rsp)
	vmovups	56(%r13), %ymm0
	vmovups	%ymm0, 112(%rsp)
	vmovups	(%r13), %ymm0
	vmovdqu	16(%r13), %ymm1
	vmovdqu	%ymm1, 16(%rax)
	vmovups	%ymm0, (%rax)
	movq	%rcx, (%r13)
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 8(%r13)
	movq	144(%rsp), %rcx
	movq	%rcx, 40(%r13)
.LBB360_272:
	leaq	96(%r13), %rcx
	jmp	.LBB360_274
.LBB360_273:
	movq	%rax, %rcx
	movq	%r13, %rax
.LBB360_274:
	cmpq	$48, %rdx
	jae	.LBB360_276
.LBB360_275:
	movq	56(%rsp), %r13
	movq	64(%rsp), %r12
	jmp	.LBB360_214
.LBB360_276:
	leaq	(%r12,%r12,2), %rdx
	addq	%r13, %rdx
	jmp	.LBB360_279
.LBB360_277:
	movq	%rsi, (%rdi)
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 8(%rdi)
	movq	144(%rsp), %rsi
	movq	%rsi, 40(%rdi)
.LBB360_278:
	addq	$96, %rcx
	cmpq	%rdx, %rcx
	je	.LBB360_275
.LBB360_279:
	movq	(%rcx), %rsi
	cmpq	(%rax), %rsi
	jae	.LBB360_280
	movq	88(%rax), %rdi
	movq	%rdi, 144(%rsp)
	movq	%r13, %rdi
	vmovups	56(%rax), %ymm0
	vmovups	%ymm0, 112(%rsp)
	vmovdqu	(%rax), %ymm0
	vmovdqu	16(%rax), %ymm1
	vmovdqu	%ymm1, 16(%rcx)
	vmovdqu	%ymm0, (%rcx)
	cmpq	%r13, %rax
	je	.LBB360_286
.LBB360_282:
	cmpq	-48(%rax), %rsi
	jae	.LBB360_285
	leaq	-48(%rax), %rdi
	vmovdqu	(%rdi), %ymm0
	vmovdqu	16(%rdi), %ymm1
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu	%ymm0, (%rax)
	movq	%rdi, %rax
	cmpq	%r13, %rdi
	jne	.LBB360_282
	movq	%r13, %rdi
	jmp	.LBB360_286
.LBB360_280:
	movq	48(%rcx), %rsi
	leaq	48(%rcx), %rax
	cmpq	(%rcx), %rsi
	jae	.LBB360_278
	jmp	.LBB360_287
.LBB360_285:
	movq	%rax, %rdi
.LBB360_286:
	movq	%rsi, (%rdi)
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 8(%rdi)
	movq	144(%rsp), %rax
	movq	%rax, 40(%rdi)
	movq	48(%rcx), %rsi
	leaq	48(%rcx), %rax
	cmpq	(%rcx), %rsi
	jae	.LBB360_278
.LBB360_287:
	movq	88(%rcx), %rdi
	movq	%rdi, 144(%rsp)
	movq	%r13, %rdi
	vmovups	56(%rcx), %ymm0
	vmovups	%ymm0, 112(%rsp)
	vmovdqu	(%rcx), %ymm0
	vmovdqu	16(%rcx), %ymm1
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu	%ymm0, (%rax)
	cmpq	%r13, %rcx
	je	.LBB360_277
	movq	%rcx, %rdi
.LBB360_289:
	cmpq	-48(%rdi), %rsi
	jae	.LBB360_277
	leaq	-48(%rdi), %r8
	vmovdqu	(%r8), %ymm0
	vmovdqu	16(%r8), %ymm1
	vmovdqu	%ymm1, 16(%rdi)
	vmovdqu	%ymm0, (%rdi)
	movq	%r8, %rdi
	cmpq	%r13, %r8
	jne	.LBB360_289
	movq	%r13, %rdi
	jmp	.LBB360_277
.LBB360_292:
.Ltmp15058:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp15059:
.LBB360_293:
	ud2
.LBB360_294:
.Ltmp15050:
	movq	core::slice::sort::unstable::ipnsort::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), <[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId, false>::{closure#1}>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15051:
	jmp	.LBB360_214
.LBB360_295:
	movl	$8, %edi
	jmp	.LBB360_147
.LBB360_296:
.Ltmp15052:
	leaq	48(%rsp), %rdi
	movq	%rax, (%rsp)
	jmp	.LBB360_303
.LBB360_297:
.Ltmp15046:
	movq	8(%rsp), %rcx
	movq	%rbp, 120(%rsp)
	movq	%rax, (%rsp)
	movq	%rcx, 144(%rsp)
	cmpq	$6, %rbx
	jb	.LBB360_299
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r13, %rdi
	callq	__rustc::__rust_dealloc
.LBB360_299:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>>
	jmp	.LBB360_331
.LBB360_300:
.Ltmp15049:
	movq	%r14, 144(%rsp)
	movq	%rbp, 136(%rsp)
	movw	%r12w, 160(%rsp)
	movq	%rax, (%rsp)
	movq	%rbx, 168(%rsp)
	cmpq	$6, %r13
	jb	.LBB360_302
	movq	24(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB360_302:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId, false>::{closure#0}>>
	leaq	216(%rsp), %rdi
.LBB360_303:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>>
	jmp	.LBB360_331
.LBB360_304:
.Ltmp15043:
	movb	$1, %bl
	movq	%rax, (%rsp)
	jmp	.LBB360_332
.LBB360_305:
.Ltmp15040:
	movq	%rax, (%rsp)
	jmp	.LBB360_336
.LBB360_306:
.Ltmp15035:
	movq	%rax, (%rsp)
.Ltmp15036:
	leaq	504(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15037:
	jmp	.LBB360_359
.LBB360_307:
.Ltmp15060:
	leaq	80(%rsp), %rdi
	movq	%rax, (%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId, false>::{closure#2}>>
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB360_336
	#MEMBARRIER
.Ltmp15061:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp15062:
	jmp	.LBB360_336
.LBB360_309:
.Ltmp15055:
	movq	%rax, (%rsp)
	movq	24(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB360_319
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_312
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_312:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_318
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_312
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB360_315:
	cmpq	%rax, %rsi
	jge	.LBB360_317
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_315
.LBB360_317:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_318:
	movq	free@GOTPCREL(%rip), %rax
	movq	16(%rsp), %rdi
	callq	*%rax
.LBB360_319:
	testq	%rbx, %rbx
	jne	.LBB360_360
.LBB360_320:
	xorl	%ebx, %ebx
	cmpq	$0, 8(%rsp)
	je	.LBB360_332
	movq	40(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB360_332
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_324
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_324:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_330
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_324
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%rsi, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB360_327:
	cmpq	%rax, %rdx
	jge	.LBB360_329
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_327
.LBB360_329:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_330:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	callq	*%rax
.LBB360_331:
	xorl	%ebx, %ebx
.LBB360_332:
	movq	344(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB360_334
	leaq	344(%rsp), %rdi
	#MEMBARRIER
.Ltmp15056:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15057:
.LBB360_334:
	testb	%bl, %bl
	je	.LBB360_336
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB360_336:
	movq	424(%rsp), %rax
	movabsq	$9223372036854775807, %rbx
	cmpq	$6, %rax
	jb	.LBB360_337
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	432(%rsp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_341
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_341:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_347
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_341
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB360_344:
	cmpq	%rax, %rdx
	jge	.LBB360_346
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_344
.LBB360_346:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_347:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB360_348
.LBB360_338:
	movq	448(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB360_357
	jmp	.LBB360_359
.LBB360_337:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB360_338
.LBB360_348:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	360(%rsp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_350
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_350:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_356
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_350
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbx, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB360_353:
	cmpq	%rax, %rdx
	jge	.LBB360_355
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_353
.LBB360_355:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_356:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	448(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_359
.LBB360_357:
	lock		decq	(%rax)
	jne	.LBB360_359
	leaq	448(%rsp), %rdi
	#MEMBARRIER
.Ltmp15064:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15065:
.LBB360_359:
	movq	(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB360_360:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB360_364
	.p2align	4
.LBB360_361:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_362:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB360_363:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB360_320
.LBB360_364:
	testw	%r12w, %r12w
	jne	.LBB360_367
	.p2align	4
.LBB360_365:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_365
	kmovd	%k0, %r12d
.LBB360_367:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_363
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	leaq	-8(,%rcx,8), %rcx
	addq	%rbp, %rax
	movq	-40(%rax), %rdi
	cmpq	%r15, %rcx
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rdx
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rdx
	cmovoq	%rsi, %rdx
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_370
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_370:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_362
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_370
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB360_373:
	cmpq	%rax, %rdx
	jge	.LBB360_361
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_373
	jmp	.LBB360_361
.LBB360_375:
.Ltmp15063:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB360_376:
.Ltmp15066:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end360:
purrdf_sparql_eval::modifier::eval_dedup_with::<purrdf_core::ir::dataset::RdfDataset, true>:
.Lfunc_begin361:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception268
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
	subq	$520, %rsp
	.cfi_def_cfa_offset 576
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	152(%rsp), %rdi
	movq	%rcx, %r14
	movq	%rdx, %r15
	callq	*%rax
.Ltmp15067:
	leaq	400(%rsp), %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15068:
	cmpl	$1, 400(%rsp)
	jne	.LBB361_25
	vmovups	448(%rsp), %zmm1
	vmovups	416(%rsp), %zmm0
	movq	224(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB361_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	232(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_5
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB361_8:
	cmpq	%rax, %rsi
	jge	.LBB361_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_8
.LBB361_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_12:
	movq	152(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB361_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	160(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_15
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB361_18:
	cmpq	%rax, %rsi
	jge	.LBB361_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_18
.LBB361_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_22:
	movq	248(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_100
	lock		decq	(%rax)
	jne	.LBB361_100
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	248(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB361_100
.LBB361_25:
	vmovups	440(%rsp), %zmm1
	vmovups	408(%rsp), %zmm0
	vmovups	%zmm1, 288(%rsp)
	vmovups	%zmm0, 256(%rsp)
.Ltmp15069:
	leaq	56(%rsp), %rdi
	leaq	152(%rsp), %rsi
	leaq	256(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15070:
	cmpq	$-1, 56(%rsp)
	je	.LBB361_104
	vmovups	56(%rsp), %ymm0
	vmovups	192(%rsp), %zmm1
	vmovups	152(%rsp), %zmm2
	vmovups	%zmm1, 296(%rsp)
	vmovups	%ymm0, 368(%rsp)
	vmovups	%zmm2, 256(%rsp)
.Ltmp15074:
	leaq	56(%rsp), %rdi
	leaq	368(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::blank_scope::without_joined_blanks::<purrdf_core::ir::term::TermId>
.Ltmp15075:
	movq	72(%rsp), %r15
	cmpq	$2, %r15
	jb	.LBB361_82
	movq	64(%rsp), %r12
	movl	$1, %r14d
	jmp	.LBB361_30
	.p2align	4
.LBB361_43:
	incq	%r14
	cmpq	%r15, %r14
	je	.LBB361_82
.LBB361_30:
	leaq	(%r14,%r14,4), %rsi
	movq	(%r12,%rsi,8), %rax
	movq	16(%r12,%rsi,8), %rcx
	movq	8(%r12,%rsi,8), %rdi
	leaq	8(%r12,%rsi,8), %rdx
	leaq	-1(%rax), %r8
	decq	%rcx
	cmpq	$5, %r8
	cmovbq	%r8, %rcx
	movq	-40(%r12,%rsi,8), %r8
	cmovaeq	%rdi, %rdx
	decq	%r8
	cmpq	$5, %r8
	jb	.LBB361_31
	movq	-24(%r12,%rsi,8), %r8
	movq	-32(%r12,%rsi,8), %rsi
	decq	%r8
	cmpq	%r8, %rcx
	jne	.LBB361_43
	jmp	.LBB361_34
	.p2align	4
.LBB361_31:
	leaq	-32(%r12,%rsi,8), %rsi
	cmpq	%r8, %rcx
	jne	.LBB361_43
.LBB361_34:
	testq	%rcx, %rcx
	je	.LBB361_44
	xorl	%r8d, %r8d
	jmp	.LBB361_37
	.p2align	4
.LBB361_36:
	incq	%r8
	cmpq	%r8, %rcx
	je	.LBB361_44
.LBB361_37:
	movl	(%rdx,%r8,8), %r10d
	movl	(%rsi,%r8,8), %r9d
	cmpl	$2, %r10d
	je	.LBB361_41
	cmpl	$2, %r9d
	je	.LBB361_41
	cmpl	%r9d, %r10d
	jne	.LBB361_43
	movl	4(%rsi,%r8,8), %r9d
	cmpl	%r9d, 4(%rdx,%r8,8)
	je	.LBB361_36
	jmp	.LBB361_43
	.p2align	4
.LBB361_41:
	cmpl	$2, %r10d
	jne	.LBB361_43
	cmpl	$2, %r9d
	je	.LBB361_36
	jmp	.LBB361_43
.LBB361_104:
	leaq	8(%rbx), %rdi
	leaq	152(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	jmp	.LBB361_99
.LBB361_44:
	cmpq	$6, %rax
	jb	.LBB361_54
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_47
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_47:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_53
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_47
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB361_50:
	cmpq	%rax, %rsi
	jge	.LBB361_52
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_50
.LBB361_52:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_53:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_54:
	leaq	1(%r14), %r13
	movq	%rbx, 8(%rsp)
	cmpq	%r15, %r13
	jae	.LBB361_81
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	jmp	.LBB361_56
	.p2align	4
.LBB361_69:
	leaq	(%r12,%rcx,8), %rax
	leaq	(%r12,%rdx,8), %rcx
	incq	%r14
	movq	32(%rax), %rdx
	movq	%rdx, 32(%rcx)
	vmovups	(%rax), %ymm0
	vmovups	%ymm0, (%rcx)
.LBB361_80:
	incq	%r13
	cmpq	%r15, %r13
	je	.LBB361_81
.LBB361_56:
	leaq	(%r13,%r13,4), %rcx
	leaq	(%r14,%r14,4), %rdx
	movq	(%r12,%rcx,8), %rax
	movq	16(%r12,%rcx,8), %rsi
	movq	8(%r12,%rcx,8), %rdi
	movq	-40(%r12,%rdx,8), %r10
	leaq	8(%r12,%rcx,8), %r8
	leaq	-1(%rax), %r9
	decq	%rsi
	cmpq	$5, %r9
	cmovbq	%r9, %rsi
	cmovaeq	%rdi, %r8
	decq	%r10
	cmpq	$5, %r10
	jb	.LBB361_57
	movq	-24(%r12,%rdx,8), %r10
	movq	-32(%r12,%rdx,8), %r9
	decq	%r10
	cmpq	%r10, %rsi
	jne	.LBB361_69
	jmp	.LBB361_60
	.p2align	4
.LBB361_57:
	leaq	-32(%r12,%rdx,8), %r9
	cmpq	%r10, %rsi
	jne	.LBB361_69
.LBB361_60:
	testq	%rsi, %rsi
	je	.LBB361_70
	xorl	%r10d, %r10d
	jmp	.LBB361_63
	.p2align	4
.LBB361_62:
	incq	%r10
	cmpq	%r10, %rsi
	je	.LBB361_70
.LBB361_63:
	movl	(%r8,%r10,8), %ebx
	movl	(%r9,%r10,8), %r11d
	cmpl	$2, %ebx
	je	.LBB361_67
	cmpl	$2, %r11d
	je	.LBB361_67
	cmpl	%r11d, %ebx
	jne	.LBB361_69
	movl	4(%r9,%r10,8), %r11d
	cmpl	%r11d, 4(%r8,%r10,8)
	je	.LBB361_62
	jmp	.LBB361_69
	.p2align	4
.LBB361_67:
	cmpl	$2, %ebx
	jne	.LBB361_69
	cmpl	$2, %r11d
	je	.LBB361_62
	jmp	.LBB361_69
	.p2align	4
.LBB361_70:
	cmpq	$6, %rax
	jb	.LBB361_80
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_73
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_73:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_79
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_73
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB361_76:
	cmpq	%rax, %rdx
	jge	.LBB361_78
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_76
.LBB361_78:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_79:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB361_80
.LBB361_81:
	movq	8(%rsp), %rbx
	movq	%r14, 72(%rsp)
.LBB361_82:
	vmovups	56(%rsp), %xmm0
	movq	72(%rsp), %rax
	movq	80(%rsp), %rcx
	cmpq	$-1, 256(%rsp)
	movq	%rax, 32(%rsp)
	movq	%rcx, 40(%rsp)
	vmovups	%xmm0, 16(%rsp)
	je	.LBB361_84
	leaq	56(%rsp), %rdi
	leaq	16(%rsp), %rsi
	leaq	152(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	328(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB361_86
	jmp	.LBB361_95
.LBB361_84:
	vmovups	16(%rsp), %xmm0
	movq	32(%rsp), %rax
	movq	40(%rsp), %rcx
	movq	%rax, 80(%rsp)
	movq	%rcx, 88(%rsp)
	vmovups	%xmm0, 64(%rsp)
	movq	$-1, 56(%rsp)
	movq	328(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_95
.LBB361_86:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	336(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_88
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_88:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_94
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_88
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB361_91:
	cmpq	%rax, %rsi
	jge	.LBB361_93
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_91
.LBB361_93:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_94:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_95:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_98
	lock		decq	(%rax)
	jne	.LBB361_98
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB361_98:
	vmovups	88(%rsp), %zmm1
	vmovups	56(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
.LBB361_99:
	movq	$0, (%rbx)
.LBB361_100:
	movq	%rbx, %rax
	addq	$520, %rsp
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
	vzeroupper
	retq
.LBB361_101:
	.cfi_def_cfa_offset 576
.Ltmp15076:
	movq	%rax, %rbx
.Ltmp15077:
	leaq	152(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15078:
	jmp	.LBB361_102
.LBB361_103:
.Ltmp15071:
	movq	%rax, %rbx
.Ltmp15072:
	leaq	152(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15073:
.LBB361_102:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB361_105:
.Ltmp15079:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end361:
purrdf_sparql_eval::modifier::eval_graph_with::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>:
.Lfunc_begin362:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception269
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
	subq	$1160, %rsp
	.cfi_def_cfa_offset 1216
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpb	$0, (%rdx)
	movq	%r9, %rbp
	movq	%rdx, %r13
	movq	%rdi, 136(%rsp)
	movq	%r8, 664(%rsp)
	movq	%rcx, 144(%rsp)
	je	.LBB362_16
	movq	%rcx, %r12
	movq	%r8, 456(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	936(%rsp), %rdi
	callq	*%rax
	movq	664(%rbp), %rax
	movl	$4, %edx
	movq	%rbp, 152(%rsp)
	movq	%rdx, 168(%rsp)
	movq	160(%rax), %rcx
	testq	%rcx, %rcx
	je	.LBB362_30
	movq	152(%rax), %r15
	cmpl	$1, 1144(%rbp)
	leaq	(%r15,%rcx,4), %r12
	jne	.LBB362_31
	movq	1152(%rbp), %rax
	testq	%rax, %rax
	je	.LBB362_314
	movq	1160(%rbp), %rcx
	xorl	%ebx, %ebx
.LBB362_5:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rcx, %rdx
	movq	%rax, %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	je	.LBB362_11
.LBB362_6:
	movl	%r8d, %r9d
	shll	$2, %r9d
	xorl	%edi, %edi
	.p2align	4
.LBB362_7:
	cmpl	8(%rsi,%rdi,4), %ebp
	seta	%r10b
	sbbb	$0, %r10b
	cmpb	$1, %r10b
	jne	.LBB362_10
	incq	%rdi
	addq	$-4, %r9
	jne	.LBB362_7
	jmp	.LBB362_11
.LBB362_10:
	movzbl	%r10b, %r8d
	testl	%r8d, %r8d
	je	.LBB362_32
	jmp	.LBB362_12
	.p2align	4
.LBB362_11:
	movq	%r8, %rdi
.LBB362_12:
	subq	$1, %rdx
	jb	.LBB362_14
	movq	56(%rsi,%rdi,8), %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	jne	.LBB362_6
	jmp	.LBB362_11
.LBB362_14:
	cmpq	%r12, %r15
	jne	.LBB362_5
	movq	144(%rsp), %r12
	movq	152(%rsp), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB362_63
.LBB362_16:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	672(%rsp), %rdi
	callq	*%rax
	movq	8(%r13), %r12
	movq	16(%r13), %rbx
	addq	$16, %r12
	testq	%rbx, %rbx
	jns	.LBB362_19
	xorl	%edi, %edi
.LBB362_18:
.Ltmp15103:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp15104:
	jmp	.LBB362_438
.LBB362_19:
	movq	664(%rbp), %r14
	movabsq	$-9223372036854775808, %r15
	je	.LBB362_204
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB362_433
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	$-1, %rdi
	leaq	(%rbx,%rax), %rcx
	sarq	$63, %rcx
	xorq	%r15, %rcx
	addq	%rbx, %rax
	cmovoq	%rcx, %rax
	incq	%rdx
	cmoveq	%rdi, %rdx
	addq	%rbx, %rsi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%rdi, %rsi
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB362_23
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_23:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_29
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_23
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	%rbx, (%rcx)
	movq	%rbx, %rcx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	leaq	(%rcx,%rbx), %rax
	sarq	$63, %rax
	xorq	%r15, %rax
	addq	%rbx, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB362_26:
	cmpq	%rax, %rcx
	jle	.LBB362_28
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB362_26
.LBB362_28:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_29:
	.cfi_escape 0x2e, 0x00
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	callq	*%rax
	jmp	.LBB362_205
.LBB362_30:
	xorl	%ebx, %ebx
	xorl	%r15d, %r15d
	jmp	.LBB362_63
.LBB362_31:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB362_32:
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$16, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB362_431
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$16, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$16, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB362_35
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_35:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_41
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_35
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$16, (%rsi)
	movl	$16, %esi
	lock		xaddq	%rsi, (%rdi)
	addq	$16, %rsi
	cmovoq	%rdx, %rsi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB362_38:
	cmpq	%rax, %rsi
	jle	.LBB362_40
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB362_38
.LBB362_40:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_41:
	movl	$1, %ebx
	movq	$4, 32(%rsp)
	movq	%rcx, 40(%rsp)
	movl	%ebp, (%rcx)
	movq	$1, 48(%rsp)
	cmpq	%r12, %r15
	je	.LBB362_61
	movq	152(%rsp), %rbp
	leaq	32(%rsp), %r14
	jmp	.LBB362_45
	.p2align	4
.LBB362_43:
	movq	40(%rsp), %rcx
.LBB362_44:
	movl	%ebp, (%rcx,%rbx,4)
	movq	152(%rsp), %rbp
	incq	%rbx
	movq	%rbx, 48(%rsp)
	cmpq	%r12, %r15
	je	.LBB362_62
.LBB362_45:
	cmpl	$1, 1144(%rbp)
	jne	.LBB362_58
	movq	1152(%rbp), %rax
	testq	%rax, %rax
	je	.LBB362_62
	movq	1160(%rbp), %rdx
.LBB362_48:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rdx, %rsi
	movq	%rax, %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	je	.LBB362_54
.LBB362_49:
	movl	%r9d, %r10d
	shll	$2, %r10d
	xorl	%r8d, %r8d
	.p2align	4
.LBB362_50:
	cmpl	8(%rdi,%r8,4), %ebp
	seta	%r11b
	sbbb	$0, %r11b
	cmpb	$1, %r11b
	jne	.LBB362_53
	incq	%r8
	addq	$-4, %r10
	jne	.LBB362_50
	jmp	.LBB362_54
	.p2align	4
.LBB362_53:
	movzbl	%r11b, %r9d
	testl	%r9d, %r9d
	je	.LBB362_59
	subq	$1, %rsi
	jae	.LBB362_56
	jmp	.LBB362_57
	.p2align	4
.LBB362_54:
	movq	%r9, %r8
	subq	$1, %rsi
	jb	.LBB362_57
.LBB362_56:
	movq	56(%rdi,%r8,8), %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	jne	.LBB362_49
	jmp	.LBB362_54
.LBB362_57:
	cmpq	%r12, %r15
	jne	.LBB362_48
	jmp	.LBB362_61
	.p2align	4
.LBB362_58:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB362_59:
	cmpq	32(%rsp), %rbx
	jne	.LBB362_44
.Ltmp15109:
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	movl	$4, %ecx
	movl	$4, %r8d
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15110:
	jmp	.LBB362_43
.LBB362_61:
	movq	152(%rsp), %rbp
.LBB362_62:
	movq	40(%rsp), %rax
	movq	32(%rsp), %r15
	movq	144(%rsp), %r12
	movq	%rax, 168(%rsp)
.LBB362_63:
.Ltmp15115:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::modifier::yields_nothing_without_rows_in_the_active_graph@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%r15, 16(%rsp)
	callq	*%rax
	movb	%al, 15(%rsp)
.Ltmp15116:
	movq	784(%rbp), %rcx
	leaq	8(%r13), %rax
	movq	$0, 304(%rsp)
	movq	$8, 312(%rsp)
	movq	$-1, 528(%rsp)
	movq	$0, 320(%rsp)
	movq	%r13, 432(%rsp)
	movq	%rax, 328(%rsp)
	movq	%rcx, 336(%rsp)
	testq	%rbx, %rbx
	je	.LBB362_202
	movq	168(%rsp), %rax
	movq	$0, 24(%rsp)
	leaq	(%rax,%rbx,4), %rcx
	movq	%rax, %rbx
	movq	%rcx, 440(%rsp)
	movq	8(%r13), %rcx
	movq	%rcx, 640(%rsp)
	movq	16(%r13), %rcx
	movq	%rcx, 632(%rsp)
	movl	$8, %ecx
	movq	%rcx, 360(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.538(%rip), %rcx
	movq	%rcx, 424(%rsp)
	xorl	%ecx, %ecx
.LBB362_66:
	movl	%ecx, 160(%rsp)
	jmp	.LBB362_70
.LBB362_67:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_68:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_69:
	cmpq	440(%rsp), %rbx
	je	.LBB362_203
.LBB362_70:
	movl	(%rbx), %r14d
	movq	456(%rsp), %rax
	movl	%r14d, 164(%rsp)
	movzbl	16(%rax), %r13d
	testb	%r13b, %r13b
	jne	.LBB362_244
	addq	$4, %rbx
	cmpb	$0, 15(%rsp)
	movq	%rbx, 352(%rsp)
	je	.LBB362_80
	movq	664(%rbp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::probe_plan@GOTPCREL(%rip), %rax
	movl	$2, %ecx
	xorl	%edi, %edi
	xorl	%esi, %esi
	xorl	%edx, %edx
	vzeroupper
	callq	*%rax
	movq	%rax, 176(%rsp)
	movb	%dl, 184(%rsp)
.Ltmp15118:
	.cfi_escape 0x2e, 0x10
	leaq	176(%rsp), %rdx
	leaq	32(%rsp), %rdi
	movq	%rbx, %rsi
	xorl	%ecx, %ecx
	xorl	%r8d, %r8d
	xorl	%r9d, %r9d
	pushq	%r14
	.cfi_adjust_cfa_offset 8
	pushq	$2
	.cfi_adjust_cfa_offset 8
	movq	<purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan@GOTPCREL(%rip), %rax
	callq	*%rax
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp15119:
.Ltmp15120:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	callq	<purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next
.Ltmp15121:
	testq	%rax, %rax
	jne	.LBB362_80
	cmpq	$0, 80(%rbx)
	je	.LBB362_78
.Ltmp15122:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158(%rip), %rsi
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15123:
	testl	%eax, %eax
	je	.LBB362_427
.LBB362_78:
.Ltmp15127:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158(%rip), %rsi
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15128:
	movq	%rbx, 40(%rsp)
	movl	%eax, 48(%rsp)
	movq	$0, 56(%rsp)
	movq	$0, 80(%rsp)
	movl	$2, 32(%rsp)
	movl	%r14d, 36(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	32(%rsp), %rsi
	callq	<core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next
	cmpl	$0, 176(%rsp)
	je	.LBB362_194
	.p2align	4
.LBB362_80:
	movq	328(%rsp), %r9
	movl	164(%rsp), %eax
	leaq	164(%rsp), %r8
	movl	$2, 784(%rbp)
	movq	%r9, 32(%rsp)
	movq	%r8, 40(%rsp)
	leaq	336(%rsp), %r9
	leaq	456(%rsp), %r8
	movl	%eax, 788(%rbp)
	movq	%r9, 48(%rsp)
	movq	%r8, 56(%rsp)
	leaq	32(%rsp), %r9
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.524(%rip), %r8
	movq	%r9, 176(%rsp)
	movq	%r8, 184(%rsp)
	movb	$0, 192(%rsp)
.Ltmp15129:
	.cfi_escape 0x2e, 0x00
	leaq	816(%rsp), %rdi
	leaq	176(%rsp), %rdx
	movq	%r12, %rsi
	movq	%rbp, %rcx
	vzeroupper
	callq	purrdf_sparql_eval::eval::eval_yielding::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15130:
	cmpl	$1, 816(%rsp)
	je	.LBB362_240
	cmpq	$-1, 824(%rsp)
	jne	.LBB362_241
	leaq	824(%rsp), %rax
	vmovdqu	8(%rax), %ymm0
	movl	164(%rsp), %eax
	movl	%eax, 368(%rsp)
	vmovdqu	%ymm0, 384(%rsp)
	movq	408(%rsp), %r12
	leaq	16(%r12), %rsi
.Ltmp15139:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15140:
.Ltmp15142:
	.cfi_escape 0x2e, 0x00
	movq	328(%rsp), %rsi
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)
.Ltmp15143:
	movq	%r12, 448(%rsp)
	cmpq	$1, %rax
	jne	.LBB362_110
	movq	400(%rsp), %rax
	movq	392(%rsp), %r14
	movq	%rdx, %r13
	movq	384(%rsp), %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%r14, 176(%rsp)
	movq	%rdx, %r12
	movq	%rdx, 192(%rsp)
	movq	%r14, 344(%rsp)
	leaq	(%r14,%rcx,8), %rbp
	movq	%rbp, 200(%rsp)
	testq	%rax, %rax
	je	.LBB362_129
	vmovd	368(%rsp), %xmm0
	vpshufb	.LCPI362_0(%rip), %xmm0, %xmm0
	vmovdqa	%xmm0, 608(%rsp)
	jmp	.LBB362_90
	.p2align	4
.LBB362_88:
	movq	24(%rsp), %rdx
	movq	312(%rsp), %rsi
	leaq	784(%rsp), %rcx
	leaq	(%rdx,%rdx,4), %rax
	incq	%rdx
	movq	%rsi, 360(%rsp)
	movq	%rdx, 24(%rsp)
	movq	%r15, (%rsi,%rax,8)
	movq	%rbx, 8(%rsi,%rax,8)
	movq	16(%rsp), %r15
	vmovdqu	8(%rcx), %xmm0
	vmovdqu	%xmm0, 16(%rsi,%rax,8)
	movq	24(%rcx), %rcx
	movq	%rcx, 32(%rsi,%rax,8)
	movq	%rdx, 320(%rsp)
.LBB362_89:
	cmpq	%rbp, %r14
	je	.LBB362_142
.LBB362_90:
	movq	%r14, %rax
	movq	(%rax), %r15
	addq	$40, %r14
	testq	%r15, %r15
	je	.LBB362_128
	movq	%r15, 776(%rsp)
	leaq	784(%rsp), %rdx
	leaq	-1(%r15), %rcx
	vmovdqu	8(%rax), %ymm0
	cmpq	$5, %rcx
	vmovdqu	%ymm0, (%rdx)
	movq	792(%rsp), %rax
	movq	784(%rsp), %rbx
	leaq	-1(%rax), %rsi
	cmovbq	%rcx, %rsi
	cmpq	%rsi, %r13
	jae	.LBB362_430
	cmpq	$5, %rcx
	movq	%rdx, %rcx
	cmovaeq	%rbx, %rcx
	movl	(%rcx,%r13,8), %edx
	testl	%edx, %edx
	je	.LBB362_97
	cmpl	$2, %edx
	jne	.LBB362_98
.LBB362_94:
	cmpq	$6, %r15
	cmovbq	%r15, %rax
	decq	%rax
	cmpq	%rax, %r13
	jae	.LBB362_429
	vmovdqa	608(%rsp), %xmm0
	cmpq	$6, %r15
	leaq	784(%rsp), %rax
	cmovbq	%rax, %rbx
	movq	24(%rsp), %rax
	vmovq	%xmm0, (%rbx,%r13,8)
	movq	776(%rsp), %r15
	movq	784(%rsp), %rbx
	cmpq	304(%rsp), %rax
	jne	.LBB362_88
.Ltmp15159:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15160:
	jmp	.LBB362_88
	.p2align	4
.LBB362_97:
	movl	368(%rsp), %edx
	cmpl	%edx, 4(%rcx,%r13,8)
	je	.LBB362_94
.LBB362_98:
	cmpq	$6, %r15
	jb	.LBB362_109
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	leaq	-8(,%r15,8), %rcx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_101
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB362_101:
	movq	16(%rsp), %r15
	.p2align	4
.LBB362_102:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_108
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_102
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_105:
	cmpq	%rax, %rdx
	jge	.LBB362_107
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_105
.LBB362_107:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_108:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB362_89
	.p2align	4
.LBB362_109:
	movq	16(%rsp), %r15
	jmp	.LBB362_89
	.p2align	4
.LBB362_110:
	movq	640(%rsp), %rsi
	lock		incq	(%rsi)
	jle	.LBB362_438
.Ltmp15144:
	.cfi_escape 0x2e, 0x00
	movq	632(%rsp), %rdx
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15145:
	movq	%rax, %rbx
	movq	400(%rsp), %rax
	movq	392(%rsp), %r14
	movq	384(%rsp), %rdx
	movq	48(%rsp), %r12
	leaq	(%rax,%rax,4), %rcx
	movq	%r14, 464(%rsp)
	movq	%rdx, 648(%rsp)
	movq	%rdx, 480(%rsp)
	movq	%r14, 656(%rsp)
	leaq	(%r14,%rcx,8), %r13
	movq	%r13, 488(%rsp)
	testq	%rax, %rax
	je	.LBB362_154
	leaq	1(%r12), %rax
	vmovd	368(%rsp), %xmm0
	addq	$40, %r14
	movq	%r12, 608(%rsp)
	movq	%rax, 344(%rsp)
	movq	24(%rsp), %rax
	vpshufb	.LCPI362_0(%rip), %xmm0, %xmm0
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %r15
	vmovdqa	%xmm0, 368(%rsp)
	jmp	.LBB362_115
	.p2align	4
.LBB362_114:
	movq	360(%rsp), %rdx
	leaq	-40(%r14), %rax
	addq	$40, %r14
	addq	$40, %rax
	movq	%r12, (%rdx,%r15)
	movq	%rbx, 8(%rdx,%r15)
	movq	608(%rsp), %r12
	movq	%rbp, %rbx
	vmovdqa	496(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%r15)
	movq	512(%rsp), %rcx
	movq	%rcx, 32(%rdx,%r15)
	movq	24(%rsp), %rcx
	addq	$40, %r15
	incq	%rcx
	movq	%rcx, 24(%rsp)
	movq	%rcx, 320(%rsp)
	cmpq	%r13, %rax
	je	.LBB362_167
.LBB362_115:
	vmovdqu	-32(%r14), %ymm0
	movq	-40(%r14), %rax
	vmovdqu	%ymm0, 272(%rsp)
	testq	%rax, %rax
	je	.LBB362_153
	vmovdqu	272(%rsp), %ymm0
	leaq	184(%rsp), %rcx
	movq	%rax, 176(%rsp)
	leaq	-1(%rax), %rdx
	cmpq	$5, %rdx
	vmovdqu	%ymm0, (%rcx)
	movq	192(%rsp), %rcx
	leaq	-1(%rcx), %rsi
	cmovbq	%rdx, %rsi
	movq	%r12, %rdx
	subq	%rsi, %rdx
	jbe	.LBB362_118
	movl	$2, 496(%rsp)
	movq	%rdx, 504(%rsp)
.Ltmp15147:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	496(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp15148:
	jmp	.LBB362_120
	.p2align	4
.LBB362_118:
	cmpq	$6, %rax
	cmovbq	%rax, %rcx
	decq	%rcx
	cmpq	%rcx, %r12
	jae	.LBB362_120
	movq	344(%rsp), %rdx
	xorl	%ecx, %ecx
	cmpq	$6, %rax
	setae	%cl
	shll	$4, %ecx
	movq	%rdx, 176(%rsp,%rcx)
.LBB362_120:
	movq	176(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB362_122
	movq	192(%rsp), %rsi
.LBB362_122:
	decq	%rsi
	cmpq	%rsi, %rbx
	jae	.LBB362_428
	leaq	184(%rsp), %rcx
	cmpq	$6, %rax
	jb	.LBB362_125
	movq	184(%rsp), %rcx
.LBB362_125:
	vmovaps	368(%rsp), %xmm0
	leaq	184(%rsp), %rax
	movq	%rbx, %rbp
	vmovlps	%xmm0, (%rcx,%rbx,8)
	movq	24(%rsp), %rcx
	vmovdqu	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	176(%rsp), %r12
	movq	184(%rsp), %rbx
	movq	%rax, 512(%rsp)
	vmovdqa	%xmm0, 496(%rsp)
	cmpq	304(%rsp), %rcx
	jne	.LBB362_114
.Ltmp15153:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15154:
	movq	312(%rsp), %rax
	movq	%rax, 360(%rsp)
	jmp	.LBB362_114
.LBB362_128:
	movq	16(%rsp), %r15
.LBB362_129:
	subq	%r14, %rbp
	je	.LBB362_142
	shrq	$3, %rbp
	movabsq	$-3689348814741910323, %rax
	xorl	%ebx, %ebx
	imulq	%rax, %rbp
	jmp	.LBB362_134
	.p2align	4
.LBB362_131:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_132:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB362_133:
	incq	%rbx
	cmpq	%rbp, %rbx
	je	.LBB362_142
.LBB362_134:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB362_133
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_137
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_137:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_132
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_137
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_140:
	cmpq	%rax, %rdx
	jge	.LBB362_131
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_140
	jmp	.LBB362_131
	.p2align	4
.LBB362_142:
	movq	%r12, %rax
	movq	152(%rsp), %rbp
	movq	352(%rsp), %rbx
	testq	%r12, %r12
	movq	448(%rsp), %r12
	je	.LBB362_179
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_145
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB362_145:
	movq	344(%rsp), %rdi
	.p2align	4
.LBB362_146:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_152
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_146
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_149:
	cmpq	%rax, %rdx
	jge	.LBB362_151
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_149
.LBB362_151:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_152:
	.cfi_escape 0x2e, 0x00
	jmp	.LBB362_178
.LBB362_153:
	movq	16(%rsp), %r15
.LBB362_154:
	subq	%r14, %r13
	je	.LBB362_168
	shrq	$3, %r13
	movabsq	$-3689348814741910323, %rax
	xorl	%ebx, %ebx
	imulq	%rax, %r13
	jmp	.LBB362_159
	.p2align	4
.LBB362_156:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_157:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB362_158:
	incq	%rbx
	cmpq	%r13, %rbx
	je	.LBB362_168
.LBB362_159:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB362_158
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_162
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_162:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_157
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_162
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_165:
	cmpq	%rax, %rdx
	jge	.LBB362_156
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_165
	jmp	.LBB362_156
.LBB362_167:
	movq	16(%rsp), %r15
.LBB362_168:
	movq	648(%rsp), %rax
	movq	152(%rsp), %rbp
	movq	352(%rsp), %rbx
	movq	448(%rsp), %r12
	movq	656(%rsp), %rdi
	testq	%rax, %rax
	je	.LBB362_179
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_171
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_171:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_177
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_171
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_174:
	cmpq	%rax, %rdx
	jge	.LBB362_176
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_174
.LBB362_176:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_177:
	.cfi_escape 0x2e, 0x00
.LBB362_178:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB362_179:
	vmovups	56(%rsp), %ymm1
	vmovdqu	32(%rsp), %ymm0
	vmovups	%ymm1, 200(%rsp)
	vmovdqu	%ymm0, 176(%rsp)
	lock		decq	(%r12)
	jne	.LBB362_181
	#MEMBARRIER
.Ltmp15167:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15168:
.LBB362_181:
	vmovups	200(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	cmpq	$-1, 528(%rsp)
	movq	144(%rsp), %r12
	vmovups	%ymm1, 56(%rsp)
	vmovups	%ymm0, 32(%rsp)
	je	.LBB362_183
.Ltmp15170:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15171:
.LBB362_183:
	vmovups	56(%rsp), %ymm1
	vmovdqu	32(%rsp), %ymm0
	cmpb	$0, 816(%rsp)
	vmovups	%ymm1, 552(%rsp)
	vmovdqu	%ymm0, 528(%rsp)
	jne	.LBB362_69
	cmpq	$-1, 824(%rsp)
	je	.LBB362_69
.Ltmp15175:
	.cfi_escape 0x2e, 0x00
	leaq	824(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15176:
	movq	856(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_69
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	864(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_189
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_189:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_68
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_189
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_192:
	cmpq	%rax, %rdx
	jge	.LBB362_67
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_192
	jmp	.LBB362_67
.LBB362_194:
	movq	96(%rbx), %rax
	testq	%rax, %rax
	je	.LBB362_200
	movq	88(%rbx), %rcx
	shlq	$4, %rax
	xorl	%edx, %edx
	jmp	.LBB362_197
	.p2align	4
.LBB362_196:
	addq	$16, %rdx
	cmpq	%rdx, %rax
	je	.LBB362_200
.LBB362_197:
	movl	12(%rcx,%rdx), %esi
	testl	%esi, %esi
	je	.LBB362_196
	cmpl	%r14d, %esi
	jne	.LBB362_196
	cmpl	$0, (%rcx,%rdx)
	je	.LBB362_196
	jmp	.LBB362_80
.LBB362_200:
	movq	352(%rsp), %rbx
	movb	$1, %cl
	cmpq	440(%rsp), %rbx
	jne	.LBB362_66
	movb	$1, %al
	movl	%eax, 160(%rsp)
	xorl	%r13d, %r13d
	testq	%r15, %r15
	jne	.LBB362_245
	jmp	.LBB362_254
.LBB362_202:
	movl	$0, 160(%rsp)
.LBB362_203:
	xorl	%r13d, %r13d
	testq	%r15, %r15
	jne	.LBB362_245
	jmp	.LBB362_254
.LBB362_204:
	movl	$1, %r13d
.LBB362_205:
	movq	%rbx, 536(%rsp)
	movq	%r13, 544(%rsp)
	movq	%rbx, 552(%rsp)
	movb	$1, %bl
	movq	%r15, 528(%rsp)
.Ltmp15080:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_value@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rsi
	movq	%r14, %rdi
	callq	*%rax
.Ltmp15081:
	movq	144(%rsp), %r10
	testl	%eax, %eax
	je	.LBB362_222
	movq	664(%rbp), %rcx
	movq	160(%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB362_222
	movq	152(%rcx), %rcx
	xorl	%esi, %esi
	cmpq	$1, %rdx
	je	.LBB362_210
	.p2align	4
.LBB362_209:
	movq	%rdx, %r8
	shrq	%r8
	movq	%rsi, %rdi
	addq	%r8, %rsi
	cmpl	%eax, (%rcx,%rsi,4)
	cmovaq	%rdi, %rsi
	subq	%r8, %rdx
	cmpq	$1, %rdx
	ja	.LBB362_209
.LBB362_210:
	cmpl	%eax, (%rcx,%rsi,4)
	jne	.LBB362_222
	cmpl	$1, 1144(%rbp)
	jne	.LBB362_315
	movq	1152(%rbp), %rcx
	testq	%rcx, %rcx
	je	.LBB362_222
	movq	1160(%rbp), %rdx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	jne	.LBB362_214
.LBB362_219:
	movq	%rdi, %rsi
.LBB362_220:
	subq	$1, %rdx
	jb	.LBB362_222
	movq	56(%rcx,%rsi,8), %rcx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	je	.LBB362_219
.LBB362_214:
	movl	%edi, %r8d
	shll	$2, %r8d
	xorl	%esi, %esi
	.p2align	4
.LBB362_215:
	cmpl	8(%rcx,%rsi,4), %eax
	seta	%r9b
	sbbb	$0, %r9b
	cmpb	$1, %r9b
	jne	.LBB362_218
	incq	%rsi
	addq	$-4, %r8
	jne	.LBB362_215
	jmp	.LBB362_219
.LBB362_218:
	movzbl	%r9b, %edi
	testl	%edi, %edi
	jne	.LBB362_220
.LBB362_315:
	movq	784(%rbp), %rcx
	movl	$2, 784(%rbp)
	movl	%eax, 788(%rbp)
	leaq	464(%rsp), %rax
	movq	%rax, 176(%rsp)
	leaq	664(%rsp), %rax
	movq	%rax, 184(%rsp)
	leaq	176(%rsp), %rax
	movq	%rax, 32(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.526(%rip), %rax
	movq	%rax, 40(%rsp)
	movb	$0, 48(%rsp)
	movq	%rcx, 464(%rsp)
.Ltmp15082:
	.cfi_escape 0x2e, 0x00
	leaq	1040(%rsp), %rdi
	leaq	32(%rsp), %rdx
	movq	%r10, %rsi
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_yielding::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15083:
	movq	464(%rsp), %rax
	cmpl	$1, 1040(%rsp)
	movq	%rax, 784(%rbp)
	jne	.LBB362_414
	vmovups	1056(%rsp), %zmm0
	vmovups	1088(%rsp), %zmm1
	movq	136(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.Ltmp15091:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.12908414067662811932)
.Ltmp15092:
	movq	744(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB362_320
	movq	752(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB362_320:
	movq	672(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB362_322
	movq	680(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB362_322:
	movq	768(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_298
	lock		decq	(%rax)
	jne	.LBB362_298
	leaq	768(%rsp), %rdi
	jmp	.LBB362_399
.LBB362_222:
	vmovups	712(%rsp), %zmm1
	vmovdqu64	672(%rsp), %zmm0
	vmovups	%zmm1, 72(%rsp)
	vmovdqu64	%zmm0, 32(%rsp)
.Ltmp15093:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r10, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15094:
	cmpq	$-1, 32(%rsp)
	movq	%rax, 296(%rsp)
	movq	$0, 272(%rsp)
	movq	$8, 280(%rsp)
	movq	$0, 288(%rsp)
	je	.LBB362_226
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	272(%rsp), %rsi
	leaq	672(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB362_225
.LBB362_227:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	112(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_229
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_229:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_235
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_229
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB362_232:
	cmpq	%rax, %rsi
	jge	.LBB362_234
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB362_232
.LBB362_234:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_235:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	128(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB362_236
	jmp	.LBB362_238
.LBB362_226:
	movq	280(%rsp), %rcx
	movq	272(%rsp), %rax
	movq	288(%rsp), %rdx
	movq	%rcx, 192(%rsp)
	movq	296(%rsp), %rcx
	movq	%rax, 184(%rsp)
	movq	%rdx, 200(%rsp)
	movq	%rcx, 208(%rsp)
	movq	$-1, 176(%rsp)
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB362_227
.LBB362_225:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_238
.LBB362_236:
	lock		decq	(%rax)
	jne	.LBB362_238
	leaq	128(%rsp), %rdi
	#MEMBARRIER
.Ltmp15098:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15099:
.LBB362_238:
	vmovdqu64	176(%rsp), %zmm0
	vmovups	208(%rsp), %zmm1
.LBB362_239:
	movq	136(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.12908414067662811932)
	jmp	.LBB362_298
.LBB362_240:
	leaq	824(%rsp), %rax
	movq	136(%rsp), %rcx
	movb	$1, %r12b
	vmovups	40(%rax), %zmm1
	vmovdqu64	8(%rax), %zmm0
	movq	336(%rsp), %rax
	movq	%rax, 784(%rbp)
	vmovups	%zmm1, 48(%rcx)
	vmovdqu64	%zmm0, 16(%rcx)
	movq	$1, (%rcx)
	testq	%r15, %r15
	jne	.LBB362_341
	jmp	.LBB362_350
.LBB362_241:
	movb	$1, %r14b
.Ltmp15132:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	936(%rsp), %rsi
	leaq	824(%rsp), %rcx
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15133:
	cmpq	$-1, 32(%rsp)
	je	.LBB362_339
.Ltmp15134:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15135:
.LBB362_244:
	xorb	$1, %r13b
	testq	%r15, %r15
	je	.LBB362_254
.LBB362_245:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$2, %r15
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %r15
	cmovaeq	%rcx, %r15
	xorl	%edx, %edx
	cmpq	%r15, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%r15, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_247
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_247:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_253
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_247
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r15, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r15, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r15, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_250:
	cmpq	%rax, %rdx
	jge	.LBB362_252
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_250
.LBB362_252:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_253:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	168(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB362_254:
	movq	528(%rsp), %r12
	movq	336(%rsp), %rax
	cmpq	$-1, %r12
	movq	%rax, 784(%rbp)
	sete	%al
	movl	%eax, 24(%rsp)
	je	.LBB362_268
	vmovups	552(%rsp), %ymm1
	vmovdqu	528(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB362_432
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB362_258
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_258:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_264
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_258
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_261:
	cmpq	%rax, %rdx
	jle	.LBB362_263
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_261
.LBB362_263:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_264:
	vmovdqu64	32(%rsp), %zmm0
	movq	96(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
.LBB362_265:
	vmovups	976(%rsp), %zmm1
	vmovdqu64	936(%rsp), %zmm0
	movq	320(%rsp), %rax
	movq	%rax, 288(%rsp)
	vmovups	%zmm1, 72(%rsp)
	vmovups	304(%rsp), %xmm1
	vmovdqu64	%zmm0, 32(%rsp)
	cmpq	$-1, 32(%rsp)
	vmovaps	%xmm1, 272(%rsp)
	movq	%rbx, 296(%rsp)
	je	.LBB362_285
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	272(%rsp), %rsi
	leaq	936(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB362_267
.LBB362_286:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	112(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_288
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_288:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_294
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_288
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB362_291:
	cmpq	%rax, %rsi
	jge	.LBB362_293
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB362_291
.LBB362_293:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_294:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	128(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB362_295
	jmp	.LBB362_297
.LBB362_268:
	testb	%r13b, %r13b
	je	.LBB362_299
	movq	1032(%rsp), %r15
	movq	432(%rsp), %rax
	testq	%r15, %r15
	je	.LBB362_400
	lock		incq	(%r15)
	jle	.LBB362_438
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
	movq	%r15, 272(%rsp)
	leaq	16(%r15), %rsi
.Ltmp15227:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15228:
	lock		incq	(%rbx)
	jle	.LBB362_438
.Ltmp15230:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp15231:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB362_434
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB362_277
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_277:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_283
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_277
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_280:
	cmpq	%rax, %rdx
	jle	.LBB362_282
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_280
.LBB362_282:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_283:
	vmovdqu64	32(%rsp), %zmm0
	movq	96(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
	lock		decq	(%r15)
	jne	.LBB362_265
	#MEMBARRIER
.Ltmp15235:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15236:
	jmp	.LBB362_265
.LBB362_285:
	vmovdqu	272(%rsp), %xmm0
	movq	288(%rsp), %rax
	movq	296(%rsp), %rcx
	movq	%rax, 200(%rsp)
	movq	%rcx, 208(%rsp)
	vmovdqu	%xmm0, 184(%rsp)
	movq	$-1, 176(%rsp)
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB362_286
.LBB362_267:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_297
.LBB362_295:
	lock		decq	(%rax)
	jne	.LBB362_297
	leaq	128(%rsp), %rdi
	#MEMBARRIER
.Ltmp15262:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15263:
.LBB362_297:
	vmovdqu64	176(%rsp), %zmm0
	vmovups	208(%rsp), %zmm1
	movq	136(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.LBB362_298:
	movq	136(%rsp), %rax
	addq	$1160, %rsp
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
	vzeroupper
	retq
.LBB362_299:
	.cfi_def_cfa_offset 1216
	testb	$1, 160(%rsp)
	movq	432(%rsp), %r14
	je	.LBB362_325
.Ltmp15207:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15208:
	movq	%rax, %rsi
	movq	%rax, %rbx
	movq	%rax, 32(%rsp)
	addq	$16, %rsi
.Ltmp15209:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15210:
	lock		decq	(%rbx)
	jne	.LBB362_304
	#MEMBARRIER
.Ltmp15214:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15215:
.LBB362_304:
	movq	328(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	jle	.LBB362_438
	movq	16(%r14), %rdx
.Ltmp15216:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	callq	*%rax
.Ltmp15217:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB362_435
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB362_309
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_309:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_264
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_309
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_312:
	cmpq	%rax, %rdx
	jle	.LBB362_263
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_312
	jmp	.LBB362_263
.LBB362_314:
	movq	144(%rsp), %r12
	xorl	%ebx, %ebx
	xorl	%r15d, %r15d
	jmp	.LBB362_63
.LBB362_325:
.Ltmp15187:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15188:
	movq	%rax, %rsi
	movq	%rax, %rbx
	movq	%rax, 32(%rsp)
	addq	$16, %rsi
.Ltmp15189:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15190:
	lock		decq	(%rbx)
	jne	.LBB362_329
	#MEMBARRIER
.Ltmp15194:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15195:
.LBB362_329:
	movq	328(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	jle	.LBB362_438
	movq	16(%r14), %rdx
.Ltmp15196:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	callq	*%rax
.Ltmp15197:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB362_436
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB362_334
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_334:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_264
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_334
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_337:
	cmpq	%rax, %rdx
	jle	.LBB362_263
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_337
	jmp	.LBB362_263
.LBB362_339:
	movq	336(%rsp), %rax
	xorl	%r14d, %r14d
	movq	%rax, 784(%rbp)
.Ltmp15136:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	936(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15137:
	vmovdqu64	32(%rsp), %zmm0
	vmovups	64(%rsp), %zmm1
	movq	136(%rsp), %rax
	xorl	%r12d, %r12d
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	testq	%r15, %r15
	je	.LBB362_350
.LBB362_341:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$2, %r15
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %r15
	cmovaeq	%rdx, %r15
	xorl	%ecx, %ecx
	cmpq	%r15, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%r15, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_343
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_343:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_349
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_343
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r15, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%r15, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%r15, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	.p2align	4
.LBB362_346:
	cmpq	%rax, %rcx
	jge	.LBB362_348
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB362_346
.LBB362_348:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_349:
	.cfi_escape 0x2e, 0x00
	movq	168(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB362_350:
	movq	312(%rsp), %rbx
	cmpq	$0, 24(%rsp)
	je	.LBB362_363
	xorl	%r14d, %r14d
	jmp	.LBB362_355
	.p2align	4
.LBB362_352:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_353:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB362_354:
	incq	%r14
	cmpq	24(%rsp), %r14
	je	.LBB362_363
.LBB362_355:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB362_354
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_358
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_358:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_353
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_358
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_361:
	cmpq	%rax, %rdx
	jge	.LBB362_352
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_361
	jmp	.LBB362_352
.LBB362_363:
	movq	304(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_373
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_366
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_366:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_372
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_366
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	.p2align	4
.LBB362_369:
	cmpq	%rax, %rdx
	jge	.LBB362_371
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_369
.LBB362_371:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_372:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB362_373:
	cmpq	$-1, 528(%rsp)
	je	.LBB362_375
.Ltmp15178:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15179:
.LBB362_375:
	testb	%r12b, %r12b
	je	.LBB362_298
	movq	1008(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB362_386
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1016(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_379
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_379:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_385
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_379
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	.p2align	4
.LBB362_382:
	cmpq	%rax, %rdx
	jge	.LBB362_384
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_382
.LBB362_384:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_385:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB362_386:
	movq	936(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB362_396
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	944(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_389
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_389:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_395
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_389
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	.p2align	4
.LBB362_392:
	cmpq	%rax, %rdx
	jge	.LBB362_394
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_392
.LBB362_394:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_395:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB362_396:
	movq	1032(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_298
	lock		decq	(%rax)
	jne	.LBB362_298
	leaq	1032(%rsp), %rdi
.LBB362_399:
	#MEMBARRIER
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB362_298
.LBB362_400:
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
.Ltmp15246:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15247:
	movq	%rax, %rsi
	movq	%rax, %r15
	movq	%rax, 32(%rsp)
	addq	$16, %rsi
.Ltmp15249:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15250:
	lock		decq	(%r15)
	jne	.LBB362_404
	#MEMBARRIER
.Ltmp15254:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15255:
.LBB362_404:
	lock		incq	(%rbx)
	jle	.LBB362_438
.Ltmp15256:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp15257:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB362_437
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB362_409
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_409:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_264
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_409
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB362_412:
	cmpq	%rax, %rdx
	jle	.LBB362_263
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB362_412
	jmp	.LBB362_263
.LBB362_414:
	leaq	1048(%rsp), %rcx
.Ltmp15084:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	672(%rsp), %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15085:
	cmpq	$-1, 32(%rsp)
	je	.LBB362_418
	vmovups	672(%rsp), %zmm2
	vmovups	32(%rsp), %ymm0
	vmovups	712(%rsp), %zmm1
	vmovups	%zmm2, 32(%rsp)
	vmovups	%ymm0, 272(%rsp)
	vmovups	%zmm1, 72(%rsp)
	cmpq	$-1, 32(%rsp)
	je	.LBB362_421
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	272(%rsp), %rsi
	leaq	672(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB362_422
.LBB362_418:
	xorl	%ebx, %ebx
.Ltmp15088:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	672(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15089:
	vmovups	32(%rsp), %zmm0
	vmovups	64(%rsp), %zmm1
	jmp	.LBB362_239
.LBB362_421:
	vmovups	272(%rsp), %ymm0
	vmovups	%ymm0, 184(%rsp)
	movq	$-1, 176(%rsp)
.LBB362_422:
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB362_424
	movq	112(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB362_424:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_238
	lock		decq	(%rax)
	jne	.LBB362_238
	xorl	%ebx, %ebx
	leaq	128(%rsp), %rdi
	#MEMBARRIER
.Ltmp15086:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15087:
	jmp	.LBB362_238
.LBB362_427:
	movq	80(%rbx), %rax
	leaq	176(%rsp), %rcx
	movq	%rcx, 32(%rsp)
	movq	%rax, 176(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 40(%rsp)
.Ltmp15124:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4165.llvm.6298868053391388158(%rip), %rdi
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4166.llvm.6298868053391388158(%rip), %rdx
	leaq	32(%rsp), %rsi
	callq	*%rax
.Ltmp15125:
	jmp	.LBB362_438
.LBB362_428:
	movq	%r14, 472(%rsp)
.Ltmp15150:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.537(%rip), %rdx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15151:
	jmp	.LBB362_438
.LBB362_429:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.539(%rip), %rcx
	movq	%rax, %rsi
	movq	%rcx, 424(%rsp)
.LBB362_430:
	movq	%r14, 184(%rsp)
.Ltmp15156:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	424(%rsp), %rdx
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15157:
	jmp	.LBB362_438
.LBB362_431:
.Ltmp15112:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movl	$16, %esi
	callq	*%rax
.Ltmp15113:
	jmp	.LBB362_438
.LBB362_432:
.Ltmp15181:
	leaq	48(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15182:
	jmp	.LBB362_438
.LBB362_433:
	movl	$1, %edi
	jmp	.LBB362_18
.LBB362_434:
.Ltmp15237:
	leaq	48(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15238:
	jmp	.LBB362_438
.LBB362_435:
.Ltmp15221:
	leaq	48(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15222:
	jmp	.LBB362_438
.LBB362_436:
.Ltmp15201:
	leaq	48(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15202:
	jmp	.LBB362_438
.LBB362_437:
.Ltmp15265:
	leaq	48(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15266:
.LBB362_438:
	ud2
.LBB362_439:
.Ltmp15267:
	movq	%rax, %rbp
.Ltmp15268:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15269:
	jmp	.LBB362_467
.LBB362_440:
.Ltmp15270:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_441:
.Ltmp15251:
	lock		decq	(%r15)
	movq	%rax, %rbp
	jne	.LBB362_459
	movb	$1, %bl
	#MEMBARRIER
.Ltmp15252:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15253:
	movb	$1, %r14b
	jmp	.LBB362_518
.LBB362_443:
.Ltmp15203:
	movq	%rax, %rbp
.Ltmp15204:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15205:
	jmp	.LBB362_467
.LBB362_444:
.Ltmp15206:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_445:
.Ltmp15223:
	movq	%rax, %rbp
.Ltmp15224:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15225:
	jmp	.LBB362_467
.LBB362_446:
.Ltmp15226:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_447:
.Ltmp15191:
	lock		decq	(%rbx)
	movq	%rax, %rbp
	jne	.LBB362_459
	movb	$1, %bl
	#MEMBARRIER
.Ltmp15192:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15193:
	movb	$1, %r14b
	jmp	.LBB362_518
.LBB362_449:
.Ltmp15211:
	lock		decq	(%rbx)
	movq	%rax, %rbp
	jne	.LBB362_459
	movb	$1, %bl
	#MEMBARRIER
.Ltmp15212:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15213:
	movb	$1, %r14b
	jmp	.LBB362_518
.LBB362_451:
.Ltmp15258:
	movq	%rax, %rbp
.Ltmp15259:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15260:
	jmp	.LBB362_467
.LBB362_452:
.Ltmp15261:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_453:
.Ltmp15239:
	movq	%rax, %rbp
.Ltmp15240:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15241:
	jmp	.LBB362_457
.LBB362_454:
.Ltmp15242:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_455:
.Ltmp15232:
	movq	%rax, %rbp
.Ltmp15233:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15234:
	jmp	.LBB362_457
.LBB362_456:
.Ltmp15229:
	movq	%rax, %rbp
.LBB362_457:
	lock		decq	(%r15)
	jne	.LBB362_459
	movb	$1, %bl
	#MEMBARRIER
.Ltmp15243:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp15244:
	movb	$1, %r14b
	jmp	.LBB362_518
.LBB362_459:
	movb	$1, %r14b
	movb	$1, %bl
	jmp	.LBB362_518
.LBB362_460:
.Ltmp15245:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_461:
.Ltmp15198:
	movq	%rax, %rbp
.Ltmp15199:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15200:
	jmp	.LBB362_467
.LBB362_462:
.Ltmp15218:
	movq	%rax, %rbp
.Ltmp15219:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15220:
	jmp	.LBB362_467
.LBB362_463:
.Ltmp15100:
	movq	%rax, %rbp
	xorl	%ebx, %ebx
	jmp	.LBB362_476
.LBB362_464:
.Ltmp15180:
	movq	%rax, %rbp
	jmp	.LBB362_522
.LBB362_465:
.Ltmp15264:
	movl	24(%rsp), %ebx
	movq	%rax, %rbp
	xorl	%r14d, %r14d
	jmp	.LBB362_519
.LBB362_466:
.Ltmp15248:
	movq	%rax, %rbp
.LBB362_467:
	movb	$1, %bl
	movb	$1, %r14b
	jmp	.LBB362_518
.LBB362_468:
.Ltmp15138:
	movq	%rax, %rbp
	jmp	.LBB362_516
.LBB362_469:
.Ltmp15095:
	movq	%rax, %rbp
.Ltmp15096:
	.cfi_escape 0x2e, 0x00
	leaq	672(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15097:
	xorl	%ebx, %ebx
	jmp	.LBB362_476
.LBB362_471:
.Ltmp15183:
	movq	%rax, %rbp
.Ltmp15184:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15185:
	movb	$1, %r14b
	xorl	%ebx, %ebx
	jmp	.LBB362_518
.LBB362_473:
.Ltmp15186:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_474:
.Ltmp15114:
	movq	%rax, %rbp
	jmp	.LBB362_523
.LBB362_475:
.Ltmp15090:
	movq	%rax, %rbp
.LBB362_476:
.Ltmp15101:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.12908414067662811932)
.Ltmp15102:
	testb	%bl, %bl
	jne	.LBB362_526
	jmp	.LBB362_549
.LBB362_478:
.Ltmp15117:
	movq	%rax, %rbp
	testq	%r15, %r15
	je	.LBB362_523
	movq	16(%rsp), %rsi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	168(%rsp), %rdi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB362_523
.LBB362_480:
.Ltmp15177:
	movq	%rax, %rbp
	movq	856(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_511
	movq	864(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB362_511
.LBB362_482:
.Ltmp15169:
	movq	%rax, %rbp
	jmp	.LBB362_508
.LBB362_483:
.Ltmp15172:
	vmovups	56(%rsp), %ymm1
	vmovdqu	32(%rsp), %ymm0
	movq	%rax, %rbp
	vmovups	%ymm1, 552(%rsp)
	vmovdqu	%ymm0, 528(%rsp)
	jmp	.LBB362_508
.LBB362_484:
.Ltmp15111:
	movq	32(%rsp), %rsi
	movq	%rax, %rbp
	testq	%rsi, %rsi
	je	.LBB362_523
	movq	40(%rsp), %rdi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB362_523
.LBB362_486:
.Ltmp15141:
	movb	$1, %bl
	movq	%rax, %rbp
	jmp	.LBB362_504
.LBB362_487:
.Ltmp15161:
	movq	%rax, %rbp
	movq	%r14, 184(%rsp)
	jmp	.LBB362_498
.LBB362_488:
.Ltmp15146:
	movb	$1, %bl
	movq	%rax, %rbp
	jmp	.LBB362_502
.LBB362_489:
.Ltmp15149:
	movq	%rax, %rbp
	movq	%r14, 472(%rsp)
	jmp	.LBB362_493
.LBB362_490:
.Ltmp15155:
	movq	%rax, %rbp
	movq	%r14, 472(%rsp)
	cmpq	$5, %r12
	ja	.LBB362_495
	jmp	.LBB362_496
.LBB362_491:
.Ltmp15131:
	jmp	.LBB362_514
.LBB362_492:
.Ltmp15152:
	movq	%rax, %rbp
.LBB362_493:
	movq	176(%rsp), %r12
	cmpq	$6, %r12
	jb	.LBB362_496
	movq	184(%rsp), %rbx
.LBB362_495:
	leaq	-8(,%r12,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB362_496:
	.cfi_escape 0x2e, 0x00
	leaq	464(%rsp), %rdi
	jmp	.LBB362_501
.LBB362_497:
.Ltmp15158:
	movq	%rax, %rbp
.LBB362_498:
	cmpq	$5, %r15
	jbe	.LBB362_500
	leaq	-8(,%r15,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB362_500:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
.LBB362_501:
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
.LBB362_502:
.Ltmp15162:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15163:
	movq	408(%rsp), %r12
.LBB362_504:
	lock		decq	(%r12)
	jne	.LBB362_506
	#MEMBARRIER
.Ltmp15164:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp15165:
.LBB362_506:
	testb	%bl, %bl
	je	.LBB362_508
	.cfi_escape 0x2e, 0x00
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB362_508:
	cmpb	$0, 816(%rsp)
	jne	.LBB362_511
	cmpq	$-1, 824(%rsp)
	je	.LBB362_511
.Ltmp15173:
	.cfi_escape 0x2e, 0x00
	leaq	824(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>>
.Ltmp15174:
	movq	16(%rsp), %r15
	movb	$1, %r14b
	jmp	.LBB362_516
.LBB362_511:
	movq	16(%rsp), %r15
	jmp	.LBB362_515
.LBB362_512:
.Ltmp15166:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_513:
.Ltmp15126:
.LBB362_514:
	movq	%rax, %rbp
.LBB362_515:
	movb	$1, %r14b
.LBB362_516:
	movb	$1, %bl
	testq	%r15, %r15
	je	.LBB362_518
	shlq	$2, %r15
	.cfi_escape 0x2e, 0x00
	movq	168(%rsp), %rdi
	movl	$4, %edx
	movq	%r15, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB362_518:
	.cfi_escape 0x2e, 0x00
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	528(%rsp), %r12
.LBB362_519:
	cmpq	$-1, %r12
	setne	%al
	testb	%bl, %al
	je	.LBB362_521
.Ltmp15271:
	.cfi_escape 0x2e, 0x00
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15272:
	movl	%r14d, %r12d
	jmp	.LBB362_522
.LBB362_521:
	movl	%r14d, %r12d
.LBB362_522:
	testb	%r12b, %r12b
	je	.LBB362_549
.LBB362_523:
.Ltmp15273:
	.cfi_escape 0x2e, 0x00
	leaq	936(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15274:
	jmp	.LBB362_549
.LBB362_524:
.Ltmp15275:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB362_525:
.Ltmp15105:
	movq	%rax, %rbp
.LBB362_526:
	movq	744(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB362_527
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	752(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_531
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_531:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_537
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_531
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB362_534:
	cmpq	%rax, %rsi
	jge	.LBB362_536
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB362_534
.LBB362_536:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_537:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	672(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB362_538
.LBB362_528:
	movq	768(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB362_547
	jmp	.LBB362_549
.LBB362_527:
	movq	672(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB362_528
.LBB362_538:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	680(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB362_540
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB362_540:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB362_546
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB362_540
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB362_543:
	cmpq	%rax, %rsi
	jge	.LBB362_545
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB362_543
.LBB362_545:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB362_546:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	768(%rsp), %rax
	testq	%rax, %rax
	je	.LBB362_549
.LBB362_547:
	lock		decq	(%rax)
	jne	.LBB362_549
	leaq	768(%rsp), %rdi
	#MEMBARRIER
.Ltmp15106:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15107:
.LBB362_549:
	.cfi_escape 0x2e, 0x00
	movq	%rbp, %rdi
	callq	_Unwind_Resume@PLT
.LBB362_550:
.Ltmp15108:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end362:
purrdf_sparql_eval::modifier::eval_graph_with::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin363:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception270
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
	subq	$1256, %rsp
	.cfi_def_cfa_offset 1312
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpb	$0, (%rdx)
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbx
	movq	%rdi, 24(%rsp)
	movq	%r8, 184(%rsp)
	je	.LBB363_16
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
	movq	664(%r15), %rax
	movl	$4, %edx
	movq	%rbx, 368(%rsp)
	movq	%rdx, 176(%rsp)
	movq	160(%rax), %rcx
	testq	%rcx, %rcx
	je	.LBB363_30
	movq	%r15, %rdx
	movq	152(%rax), %r15
	cmpl	$1, 1144(%rdx)
	leaq	(%r15,%rcx,4), %r12
	jne	.LBB363_31
	movq	1152(%rdx), %rax
	testq	%rax, %rax
	je	.LBB363_309
	movq	1160(%rdx), %rcx
	xorl	%ebx, %ebx
.LBB363_5:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rcx, %rdx
	movq	%rax, %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	je	.LBB363_11
.LBB363_6:
	movl	%r8d, %r9d
	shll	$2, %r9d
	xorl	%edi, %edi
	.p2align	4
.LBB363_7:
	cmpl	8(%rsi,%rdi,4), %ebp
	seta	%r10b
	sbbb	$0, %r10b
	cmpb	$1, %r10b
	jne	.LBB363_10
	incq	%rdi
	addq	$-4, %r9
	jne	.LBB363_7
	jmp	.LBB363_11
.LBB363_10:
	movzbl	%r10b, %r8d
	testl	%r8d, %r8d
	je	.LBB363_32
	jmp	.LBB363_12
	.p2align	4
.LBB363_11:
	movq	%r8, %rdi
.LBB363_12:
	subq	$1, %rdx
	jb	.LBB363_14
	movq	56(%rsi,%rdi,8), %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	jne	.LBB363_6
	jmp	.LBB363_11
.LBB363_14:
	cmpq	%r12, %r15
	jne	.LBB363_5
	movq	184(%rsp), %r15
	xorl	%ebp, %ebp
	jmp	.LBB363_62
.LBB363_16:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	664(%rsp), %rdi
	callq	*%rax
	movq	8(%rbx), %r12
	movq	16(%rbx), %rbx
	addq	$16, %r12
	testq	%rbx, %rbx
	jns	.LBB363_19
	xorl	%edi, %edi
.LBB363_18:
.Ltmp15299:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp15300:
	jmp	.LBB363_430
.LBB363_19:
	movq	664(%r15), %r14
	movabsq	$-9223372036854775808, %r15
	movq	%r13, %rbp
	je	.LBB363_257
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_426
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	$-1, %rdi
	leaq	(%rbx,%rax), %rcx
	sarq	$63, %rcx
	xorq	%r15, %rcx
	addq	%rbx, %rax
	cmovoq	%rcx, %rax
	incq	%rdx
	cmoveq	%rdi, %rdx
	addq	%rbx, %rsi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%rdi, %rsi
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB363_23
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_23:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_29
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_23
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	%rbx, (%rcx)
	movq	%rbx, %rcx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	leaq	(%rcx,%rbx), %rax
	sarq	$63, %rax
	xorq	%r15, %rax
	addq	%rbx, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB363_26:
	cmpq	%rax, %rcx
	jle	.LBB363_28
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB363_26
.LBB363_28:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_29:
	.cfi_escape 0x2e, 0x00
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	callq	*%rax
	jmp	.LBB363_258
.LBB363_30:
	xorl	%ebx, %ebx
	xorl	%ebp, %ebp
	jmp	.LBB363_62
.LBB363_31:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB363_32:
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$16, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_424
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$16, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$16, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB363_35
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_35:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_41
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_35
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$16, (%rsi)
	movl	$16, %esi
	lock		xaddq	%rsi, (%rdi)
	addq	$16, %rsi
	cmovoq	%rdx, %rsi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB363_38:
	cmpq	%rax, %rsi
	jle	.LBB363_40
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB363_38
.LBB363_40:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_41:
	movl	$1, %ebx
	movq	$4, 32(%rsp)
	movq	%rcx, 40(%rsp)
	movl	%ebp, (%rcx)
	movq	$1, 48(%rsp)
	cmpq	%r12, %r15
	je	.LBB363_61
	leaq	32(%rsp), %r14
	jmp	.LBB363_45
	.p2align	4
.LBB363_43:
	movq	40(%rsp), %rcx
.LBB363_44:
	movl	%ebp, (%rcx,%rbx,4)
	incq	%rbx
	movq	%rbx, 48(%rsp)
	cmpq	%r12, %r15
	je	.LBB363_61
.LBB363_45:
	movq	184(%rsp), %rdx
	cmpl	$1, 1144(%rdx)
	jne	.LBB363_58
	movq	1152(%rdx), %rax
	testq	%rax, %rax
	je	.LBB363_61
	movq	1160(%rdx), %rdx
.LBB363_48:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rdx, %rsi
	movq	%rax, %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	je	.LBB363_54
.LBB363_49:
	movl	%r9d, %r10d
	shll	$2, %r10d
	xorl	%r8d, %r8d
	.p2align	4
.LBB363_50:
	cmpl	8(%rdi,%r8,4), %ebp
	seta	%r11b
	sbbb	$0, %r11b
	cmpb	$1, %r11b
	jne	.LBB363_53
	incq	%r8
	addq	$-4, %r10
	jne	.LBB363_50
	jmp	.LBB363_54
	.p2align	4
.LBB363_53:
	movzbl	%r11b, %r9d
	testl	%r9d, %r9d
	je	.LBB363_59
	subq	$1, %rsi
	jae	.LBB363_56
	jmp	.LBB363_57
	.p2align	4
.LBB363_54:
	movq	%r9, %r8
	subq	$1, %rsi
	jb	.LBB363_57
.LBB363_56:
	movq	56(%rdi,%r8,8), %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	jne	.LBB363_49
	jmp	.LBB363_54
.LBB363_57:
	cmpq	%r12, %r15
	jne	.LBB363_48
	jmp	.LBB363_61
	.p2align	4
.LBB363_58:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB363_59:
	cmpq	32(%rsp), %rbx
	jne	.LBB363_44
.Ltmp15305:
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	movl	$4, %ecx
	movl	$4, %r8d
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15306:
	jmp	.LBB363_43
.LBB363_61:
	movq	40(%rsp), %rax
	movq	32(%rsp), %rbp
	movq	184(%rsp), %r15
	movq	%rax, 176(%rsp)
.LBB363_62:
.Ltmp15311:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::modifier::yields_nothing_without_rows_in_the_active_graph@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rbp, 8(%rsp)
	callq	*%rax
	movb	%al, 23(%rsp)
.Ltmp15312:
	movq	368(%rsp), %rcx
	movl	784(%r15), %esi
	movl	788(%r15), %edx
	movq	$0, 312(%rsp)
	movq	$8, 320(%rsp)
	movq	$-1, 496(%rsp)
	movq	$0, 328(%rsp)
	leaq	8(%rcx), %rax
	movl	%esi, 136(%rsp)
	movl	%edx, 140(%rsp)
	movq	%rax, 384(%rsp)
	testq	%rbx, %rbx
	je	.LBB363_197
	movq	176(%rsp), %r14
	movq	8(%rcx), %rax
	movq	16(%rcx), %rcx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	$0, 192(%rsp)
	movq	$0, 360(%rsp)
	movq	%r13, 376(%rsp)
	leaq	(%r14,%rbx,4), %rdx
	movq	%rax, 600(%rsp)
	movq	%rcx, 592(%rsp)
	movl	$8, %ecx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.538(%rip), %rax
	movq	%rcx, 408(%rsp)
	movq	%rax, 448(%rsp)
	movq	%rdx, 456(%rsp)
.LBB363_65:
	leaq	32(%rsp), %rbx
	jmp	.LBB363_69
.LBB363_66:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_67:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_68:
	cmpq	456(%rsp), %r14
	je	.LBB363_198
.LBB363_69:
	movl	(%r14), %eax
	addq	$4, %r14
	cmpb	$0, 23(%rsp)
	movq	%r14, 400(%rsp)
	movq	%rax, 336(%rsp)
	je	.LBB363_78
	movq	664(%r15), %r14
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::probe_plan@GOTPCREL(%rip), %rax
	movl	$2, %ecx
	xorl	%edi, %edi
	xorl	%esi, %esi
	xorl	%edx, %edx
	vzeroupper
	callq	*%rax
	movq	%rax, 208(%rsp)
	movb	%dl, 216(%rsp)
.Ltmp15314:
	.cfi_escape 0x2e, 0x10
	leaq	208(%rsp), %rdx
	movq	%rbx, %rdi
	movq	%r14, %rsi
	xorl	%ecx, %ecx
	xorl	%r8d, %r8d
	xorl	%r9d, %r9d
	pushq	336(%rsp)
	.cfi_adjust_cfa_offset 8
	pushq	$2
	.cfi_adjust_cfa_offset 8
	movq	<purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan@GOTPCREL(%rip), %rax
	callq	*%rax
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp15315:
.Ltmp15316:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	<purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next
.Ltmp15317:
	testq	%rax, %rax
	jne	.LBB363_78
	cmpq	$0, 80(%r14)
	je	.LBB363_76
.Ltmp15318:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158(%rip), %rsi
	movq	%r14, %rdi
	callq	*%rax
.Ltmp15319:
	testl	%eax, %eax
	je	.LBB363_420
.LBB363_76:
.Ltmp15323:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158(%rip), %rsi
	movq	%r14, %rdi
	callq	*%rax
.Ltmp15324:
	movq	%r14, 40(%rsp)
	movl	%eax, 48(%rsp)
	movq	336(%rsp), %rax
	movq	$0, 56(%rsp)
	movq	$0, 80(%rsp)
	movl	$2, 32(%rsp)
	movl	%eax, 36(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next
	cmpl	$0, 208(%rsp)
	je	.LBB363_190
	.p2align	4
.LBB363_78:
	movq	336(%rsp), %rax
	movl	$2, 784(%r15)
	movl	%eax, 788(%r15)
.Ltmp15325:
	.cfi_escape 0x2e, 0x00
	leaq	912(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r15, %rdx
	movq	%r13, %rcx
	vzeroupper
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15326:
	cmpl	$1, 912(%rsp)
	je	.LBB363_252
	cmpq	$-1, 920(%rsp)
	jne	.LBB363_253
	leaq	920(%rsp), %rax
	vmovdqu	8(%rax), %ymm0
	vmovdqu	%ymm0, 416(%rsp)
	movq	440(%rsp), %r15
	leaq	16(%r15), %rsi
.Ltmp15335:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15336:
.Ltmp15338:
	.cfi_escape 0x2e, 0x00
	movq	384(%rsp), %rsi
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)
.Ltmp15339:
	movq	%r15, 624(%rsp)
	cmpq	$1, %rax
	jne	.LBB363_107
	movq	432(%rsp), %rax
	movq	424(%rsp), %rbx
	movq	%rdx, %r14
	movq	416(%rsp), %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%rbx, 208(%rsp)
	movq	%rdx, %r13
	movq	%rdx, 224(%rsp)
	movq	%rbx, 392(%rsp)
	leaq	(%rbx,%rcx,8), %r15
	movq	%r15, 232(%rsp)
	testq	%rax, %rax
	je	.LBB363_126
	vmovd	336(%rsp), %xmm0
	vpshufb	.LCPI363_0(%rip), %xmm0, %xmm0
	vmovdqa	%xmm0, 576(%rsp)
	jmp	.LBB363_88
	.p2align	4
.LBB363_86:
	movq	192(%rsp), %rdx
	movq	320(%rsp), %rsi
	leaq	776(%rsp), %rcx
	leaq	(%rdx,%rdx,4), %rax
	incq	%rdx
	movq	%rsi, 408(%rsp)
	movq	%rdx, 192(%rsp)
	movq	%r12, (%rsi,%rax,8)
	movq	%rbp, 8(%rsi,%rax,8)
	vmovdqu	8(%rcx), %xmm0
	vmovdqu	%xmm0, 16(%rsi,%rax,8)
	movq	24(%rcx), %rcx
	movq	%rcx, 32(%rsi,%rax,8)
	movq	%rdx, 328(%rsp)
.LBB363_87:
	movq	8(%rsp), %rbp
	cmpq	%r15, %rbx
	je	.LBB363_139
.LBB363_88:
	movq	%rbx, %rax
	movq	(%rax), %r12
	addq	$40, %rbx
	testq	%r12, %r12
	je	.LBB363_125
	movq	%r12, 768(%rsp)
	leaq	776(%rsp), %rdx
	leaq	-1(%r12), %rcx
	vmovdqu	8(%rax), %ymm0
	cmpq	$5, %rcx
	vmovdqu	%ymm0, (%rdx)
	movq	784(%rsp), %rax
	movq	776(%rsp), %rbp
	leaq	-1(%rax), %rsi
	cmovbq	%rcx, %rsi
	cmpq	%rsi, %r14
	jae	.LBB363_423
	cmpq	$5, %rcx
	movq	%rdx, %rcx
	cmovaeq	%rbp, %rcx
	movl	(%rcx,%r14,8), %edx
	testl	%edx, %edx
	je	.LBB363_95
	cmpl	$2, %edx
	jne	.LBB363_96
.LBB363_92:
	cmpq	$6, %r12
	cmovbq	%r12, %rax
	decq	%rax
	cmpq	%rax, %r14
	jae	.LBB363_422
	vmovdqa	576(%rsp), %xmm0
	cmpq	$6, %r12
	leaq	776(%rsp), %rax
	cmovbq	%rax, %rbp
	movq	192(%rsp), %rax
	vmovq	%xmm0, (%rbp,%r14,8)
	movq	768(%rsp), %r12
	movq	776(%rsp), %rbp
	cmpq	312(%rsp), %rax
	jne	.LBB363_86
.Ltmp15355:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	312(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15356:
	jmp	.LBB363_86
	.p2align	4
.LBB363_95:
	movq	336(%rsp), %rdx
	cmpl	%edx, 4(%rcx,%r14,8)
	je	.LBB363_92
.LBB363_96:
	cmpq	$6, %r12
	jb	.LBB363_87
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	leaq	-8(,%r12,8), %rcx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_99
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB363_99:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rsi
	.p2align	4
.LBB363_100:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_106
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_100
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rdi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rdi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB363_103:
	cmpq	%rax, %rdx
	jge	.LBB363_105
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB363_103
.LBB363_105:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_106:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB363_87
	.p2align	4
.LBB363_107:
	movq	600(%rsp), %rsi
	lock		incq	(%rsi)
	jle	.LBB363_430
.Ltmp15340:
	.cfi_escape 0x2e, 0x00
	movq	592(%rsp), %rdx
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15341:
	movq	%rax, %r14
	movq	432(%rsp), %rax
	movq	424(%rsp), %rbx
	movq	416(%rsp), %rdx
	movq	48(%rsp), %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%rbx, 632(%rsp)
	movq	%rdx, 616(%rsp)
	movq	%rdx, 648(%rsp)
	movq	%rbx, 608(%rsp)
	leaq	(%rbx,%rcx,8), %r15
	movq	%r15, 656(%rsp)
	testq	%rax, %rax
	je	.LBB363_151
	leaq	1(%rsi), %rax
	vmovd	336(%rsp), %xmm0
	addq	$40, %rbx
	movq	%rsi, 576(%rsp)
	movq	%rax, 392(%rsp)
	movq	192(%rsp), %rax
	vpshufb	.LCPI363_0(%rip), %xmm0, %xmm0
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %r13
	vmovdqa	%xmm0, 336(%rsp)
	jmp	.LBB363_113
	.p2align	4
.LBB363_111:
	movq	320(%rsp), %rax
	movq	%rax, 408(%rsp)
.LBB363_112:
	movq	408(%rsp), %rdx
	leaq	-40(%rbx), %rax
	addq	$40, %rbx
	addq	$40, %rax
	movq	%r12, (%rdx,%r13)
	movq	%rbp, 8(%rdx,%r13)
	movq	8(%rsp), %rbp
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	vmovdqa	464(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%r13)
	movq	480(%rsp), %rcx
	movq	%rcx, 32(%rdx,%r13)
	movq	192(%rsp), %rcx
	addq	$40, %r13
	incq	%rcx
	movq	%rcx, 192(%rsp)
	movq	%rcx, 328(%rsp)
	cmpq	%r15, %rax
	je	.LBB363_164
.LBB363_113:
	vmovdqu	-32(%rbx), %ymm0
	movq	-40(%rbx), %rax
	vmovdqu	%ymm0, 144(%rsp)
	testq	%rax, %rax
	je	.LBB363_150
	vmovdqu	144(%rsp), %ymm0
	leaq	216(%rsp), %rcx
	movq	%rax, 208(%rsp)
	movq	576(%rsp), %rdi
	leaq	-1(%rax), %rdx
	cmpq	$5, %rdx
	vmovdqu	%ymm0, (%rcx)
	movq	224(%rsp), %rcx
	leaq	-1(%rcx), %rsi
	cmovbq	%rdx, %rsi
	movq	%rdi, %rdx
	subq	%rsi, %rdx
	jbe	.LBB363_116
	movl	$2, 464(%rsp)
	movq	%rdx, 472(%rsp)
.Ltmp15343:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	leaq	464(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp15344:
	jmp	.LBB363_118
	.p2align	4
.LBB363_116:
	cmpq	$6, %rax
	cmovbq	%rax, %rcx
	decq	%rcx
	cmpq	%rcx, %rdi
	jae	.LBB363_118
	movq	392(%rsp), %rdx
	xorl	%ecx, %ecx
	cmpq	$6, %rax
	setae	%cl
	shll	$4, %ecx
	movq	%rdx, 208(%rsp,%rcx)
.LBB363_118:
	movq	208(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB363_120
	movq	224(%rsp), %rsi
.LBB363_120:
	decq	%rsi
	cmpq	%rsi, %r14
	jae	.LBB363_421
	leaq	216(%rsp), %rcx
	cmpq	$6, %rax
	jb	.LBB363_123
	movq	216(%rsp), %rcx
.LBB363_123:
	vmovaps	336(%rsp), %xmm0
	leaq	216(%rsp), %rax
	vmovlps	%xmm0, (%rcx,%r14,8)
	movq	192(%rsp), %rcx
	vmovdqu	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	208(%rsp), %r12
	movq	216(%rsp), %rbp
	movq	%rax, 480(%rsp)
	vmovdqa	%xmm0, 464(%rsp)
	cmpq	312(%rsp), %rcx
	jne	.LBB363_112
.Ltmp15349:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	312(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15350:
	jmp	.LBB363_111
.LBB363_125:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
.LBB363_126:
	subq	%rbx, %r15
	je	.LBB363_139
	shrq	$3, %r15
	movabsq	$-3689348814741910323, %rax
	xorl	%r14d, %r14d
	imulq	%rax, %r15
	jmp	.LBB363_131
	.p2align	4
.LBB363_128:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_129:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_130:
	incq	%r14
	cmpq	%r15, %r14
	je	.LBB363_139
.LBB363_131:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB363_130
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_134
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_134:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_129
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_134
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB363_137:
	cmpq	%rax, %rdx
	jge	.LBB363_128
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB363_137
	jmp	.LBB363_128
	.p2align	4
.LBB363_139:
	movq	184(%rsp), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	400(%rsp), %r14
	leaq	32(%rsp), %rbx
	testq	%r13, %r13
	je	.LBB363_149
	shlq	$3, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%r13,%r13,4), %rcx
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_142
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_142:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_148
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_142
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB363_145:
	cmpq	%rax, %rdx
	jge	.LBB363_147
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB363_145
.LBB363_147:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_148:
	.cfi_escape 0x2e, 0x00
	movq	392(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_149:
	movq	376(%rsp), %r13
	jmp	.LBB363_175
.LBB363_150:
	movq	376(%rsp), %r13
.LBB363_151:
	subq	%rbx, %r15
	je	.LBB363_165
	shrq	$3, %r15
	movabsq	$-3689348814741910323, %rax
	xorl	%r14d, %r14d
	imulq	%rax, %r15
	jmp	.LBB363_156
	.p2align	4
.LBB363_153:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_154:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_155:
	incq	%r14
	cmpq	%r15, %r14
	je	.LBB363_165
.LBB363_156:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB363_155
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_159
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_154
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_159
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB363_162:
	cmpq	%rax, %rdx
	jge	.LBB363_153
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB363_162
	jmp	.LBB363_153
.LBB363_164:
	movq	376(%rsp), %r13
.LBB363_165:
	movq	616(%rsp), %rax
	movq	184(%rsp), %r15
	movq	400(%rsp), %r14
	leaq	32(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB363_175
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_168
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_168:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_174
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_168
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB363_171:
	cmpq	%rax, %rdx
	jge	.LBB363_173
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB363_171
.LBB363_173:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_174:
	.cfi_escape 0x2e, 0x00
	movq	608(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_175:
	vmovups	56(%rsp), %ymm1
	vmovdqu	32(%rsp), %ymm0
	movq	624(%rsp), %rax
	vmovups	%ymm1, 232(%rsp)
	vmovdqu	%ymm0, 208(%rsp)
	lock		decq	(%rax)
	jne	.LBB363_177
	#MEMBARRIER
.Ltmp15363:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15364:
.LBB363_177:
	vmovups	232(%rsp), %ymm1
	vmovups	208(%rsp), %ymm0
	cmpq	$-1, 496(%rsp)
	vmovups	%ymm1, 56(%rsp)
	vmovups	%ymm0, 32(%rsp)
	je	.LBB363_179
.Ltmp15366:
	.cfi_escape 0x2e, 0x00
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15367:
.LBB363_179:
	vmovups	56(%rsp), %ymm1
	vmovdqu	32(%rsp), %ymm0
	cmpb	$0, 912(%rsp)
	vmovups	%ymm1, 520(%rsp)
	vmovdqu	%ymm0, 496(%rsp)
	jne	.LBB363_68
	cmpq	$-1, 920(%rsp)
	je	.LBB363_68
.Ltmp15371:
	.cfi_escape 0x2e, 0x00
	leaq	920(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15372:
	movq	952(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_68
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	960(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_185
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_185:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_67
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_185
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB363_188:
	cmpq	%rax, %rdx
	jge	.LBB363_66
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB363_188
	jmp	.LBB363_66
.LBB363_190:
	movq	96(%r14), %rax
	testq	%rax, %rax
	je	.LBB363_196
	movq	88(%r14), %rcx
	shlq	$4, %rax
	xorl	%edx, %edx
	jmp	.LBB363_193
	.p2align	4
.LBB363_192:
	addq	$16, %rdx
	cmpq	%rdx, %rax
	je	.LBB363_196
.LBB363_193:
	movl	12(%rcx,%rdx), %esi
	testl	%esi, %esi
	je	.LBB363_192
	cmpl	336(%rsp), %esi
	jne	.LBB363_192
	cmpl	$0, (%rcx,%rdx)
	je	.LBB363_192
	jmp	.LBB363_78
.LBB363_196:
	movq	400(%rsp), %r14
	movb	$1, %al
	movb	$1, %bl
	movq	%rax, 360(%rsp)
	cmpq	456(%rsp), %r14
	jne	.LBB363_65
	jmp	.LBB363_199
.LBB363_197:
	movq	$0, 192(%rsp)
	movq	$0, 360(%rsp)
.LBB363_198:
	movb	$1, %bl
.LBB363_199:
	testq	%rbp, %rbp
	je	.LBB363_209
.LBB363_200:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$2, %rbp
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %rbp
	cmovaeq	%rcx, %rbp
	xorl	%edx, %edx
	cmpq	%rbp, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rbp, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_202
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_202:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_208
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_202
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rbp, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rbp, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%rbp, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB363_205:
	cmpq	%rax, %rdx
	jge	.LBB363_207
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_205
.LBB363_207:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_208:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	176(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB363_209:
	movl	136(%rsp), %eax
	movl	140(%rsp), %ecx
	movq	496(%rsp), %r12
	movl	%eax, 784(%r15)
	movl	%ecx, 788(%r15)
	cmpq	$-1, %r12
	sete	%bpl
	je	.LBB363_223
	vmovups	520(%rsp), %ymm1
	vmovdqu	496(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_425
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB363_213
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_213:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_219
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_213
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB363_216:
	cmpq	%rax, %rdx
	jle	.LBB363_218
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_216
.LBB363_218:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_219:
	vmovdqu64	32(%rsp), %zmm0
	movq	96(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
.LBB363_220:
	vmovups	848(%rsp), %zmm1
	vmovdqu64	808(%rsp), %zmm0
	movq	328(%rsp), %rax
	movq	%rax, 160(%rsp)
	vmovups	%zmm1, 72(%rsp)
	vmovups	312(%rsp), %xmm1
	vmovdqu64	%zmm0, 32(%rsp)
	cmpq	$-1, 32(%rsp)
	vmovaps	%xmm1, 144(%rsp)
	movq	%rbx, 168(%rsp)
	je	.LBB363_239
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	leaq	144(%rsp), %rsi
	leaq	808(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_222
.LBB363_240:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	112(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_242
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_242:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_248
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_242
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB363_245:
	cmpq	%rax, %rsi
	jge	.LBB363_247
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB363_245
.LBB363_247:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_248:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	128(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB363_249
	jmp	.LBB363_251
.LBB363_223:
	testb	%bl, %bl
	je	.LBB363_293
	testb	$1, 360(%rsp)
	movq	368(%rsp), %rbx
	movb	$1, %al
	je	.LBB363_382
	movl	%eax, 8(%rsp)
.Ltmp15447:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15448:
	movq	%rax, %rsi
	movq	%rax, %r14
	movq	%rax, 32(%rsp)
	addq	$16, %rsi
.Ltmp15450:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15451:
	lock		decq	(%r14)
	jne	.LBB363_229
	#MEMBARRIER
.Ltmp15455:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15456:
.LBB363_229:
	movq	384(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	jle	.LBB363_430
	movq	16(%rbx), %rdx
.Ltmp15457:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	callq	*%rax
.Ltmp15458:
	vmovups	232(%rsp), %ymm1
	vmovdqu	208(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_428
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB363_234
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_234:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_219
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_234
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB363_237:
	cmpq	%rax, %rdx
	jle	.LBB363_218
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_237
	jmp	.LBB363_218
.LBB363_239:
	vmovdqu	144(%rsp), %xmm0
	movq	160(%rsp), %rax
	movq	168(%rsp), %rcx
	movq	%rax, 232(%rsp)
	movq	%rcx, 240(%rsp)
	vmovdqu	%xmm0, 216(%rsp)
	movq	$-1, 208(%rsp)
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB363_240
.LBB363_222:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_251
.LBB363_249:
	lock		decq	(%rax)
	jne	.LBB363_251
	leaq	128(%rsp), %rdi
	#MEMBARRIER
.Ltmp15462:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15463:
.LBB363_251:
	vmovdqu64	208(%rsp), %zmm0
	vmovups	240(%rsp), %zmm1
	movq	24(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB363_381
.LBB363_252:
	leaq	920(%rsp), %rax
	movl	136(%rsp), %ecx
	movl	140(%rsp), %edx
	movb	$1, %sil
	vmovdqu64	8(%rax), %zmm0
	vmovups	40(%rax), %zmm1
	movq	24(%rsp), %rax
	movl	%ecx, 784(%r15)
	movl	%edx, 788(%r15)
	vmovups	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movl	%esi, 8(%rsp)
	testq	%rbp, %rbp
	jne	.LBB363_322
	jmp	.LBB363_331
.LBB363_253:
	movb	$1, %r14b
.Ltmp15328:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	808(%rsp), %rsi
	leaq	920(%rsp), %rcx
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15329:
	cmpq	$-1, 32(%rsp)
	je	.LBB363_320
.Ltmp15330:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15331:
	xorl	%ebx, %ebx
	testq	%rbp, %rbp
	jne	.LBB363_200
	jmp	.LBB363_209
.LBB363_257:
	movl	$1, %r13d
.LBB363_258:
	movq	%rbx, 504(%rsp)
	movq	%r13, 512(%rsp)
	movq	%rbx, 520(%rsp)
	movb	$1, %bl
	movq	%r15, 496(%rsp)
.Ltmp15276:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_value@GOTPCREL(%rip), %rax
	leaq	496(%rsp), %rsi
	movq	%r14, %rdi
	callq	*%rax
.Ltmp15277:
	movq	184(%rsp), %r14
	movq	%rbp, %r10
	testl	%eax, %eax
	je	.LBB363_275
	movq	664(%r14), %rcx
	movq	160(%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB363_275
	movq	152(%rcx), %rcx
	xorl	%esi, %esi
	cmpq	$1, %rdx
	je	.LBB363_263
	.p2align	4
.LBB363_262:
	movq	%rdx, %r8
	shrq	%r8
	movq	%rsi, %rdi
	addq	%r8, %rsi
	cmpl	%eax, (%rcx,%rsi,4)
	cmovaq	%rdi, %rsi
	subq	%r8, %rdx
	cmpq	$1, %rdx
	ja	.LBB363_262
.LBB363_263:
	cmpl	%eax, (%rcx,%rsi,4)
	jne	.LBB363_275
	cmpl	$1, 1144(%r14)
	jne	.LBB363_310
	movq	1152(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB363_275
	movq	1160(%r14), %rdx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	jne	.LBB363_267
.LBB363_272:
	movq	%rdi, %rsi
.LBB363_273:
	subq	$1, %rdx
	jb	.LBB363_275
	movq	56(%rcx,%rsi,8), %rcx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	je	.LBB363_272
.LBB363_267:
	movl	%edi, %r8d
	shll	$2, %r8d
	xorl	%esi, %esi
	.p2align	4
.LBB363_268:
	cmpl	8(%rcx,%rsi,4), %eax
	seta	%r9b
	sbbb	$0, %r9b
	cmpb	$1, %r9b
	jne	.LBB363_271
	incq	%rsi
	addq	$-4, %r8
	jne	.LBB363_268
	jmp	.LBB363_272
.LBB363_271:
	movzbl	%r9b, %edi
	testl	%edi, %edi
	jne	.LBB363_273
.LBB363_310:
	vmovsd	784(%r14), %xmm0
	movl	$2, 784(%r14)
	movl	%eax, 788(%r14)
	vmovaps	%xmm0, 192(%rsp)
.Ltmp15278:
	.cfi_escape 0x2e, 0x00
	leaq	1136(%rsp), %rdi
	movq	%r10, %rsi
	movq	%r14, %rdx
	movq	%r10, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15279:
	vmovaps	192(%rsp), %xmm0
	cmpl	$1, 1136(%rsp)
	vmovlps	%xmm0, 784(%r14)
	jne	.LBB363_399
	vmovups	1152(%rsp), %zmm0
	vmovups	1184(%rsp), %zmm1
	movq	24(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.Ltmp15287:
	.cfi_escape 0x2e, 0x00
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.12908414067662811932)
.Ltmp15288:
	movq	736(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_315
	movq	744(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB363_315:
	movq	664(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB363_317
	movq	672(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB363_317:
	movq	760(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_381
	lock		decq	(%rax)
	jne	.LBB363_381
	leaq	760(%rsp), %rdi
	jmp	.LBB363_380
.LBB363_275:
	vmovups	704(%rsp), %zmm1
	vmovdqu64	664(%rsp), %zmm0
	vmovups	%zmm1, 72(%rsp)
	vmovdqu64	%zmm0, 32(%rsp)
.Ltmp15289:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r10, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15290:
	cmpq	$-1, 32(%rsp)
	movq	%rax, 168(%rsp)
	movq	$0, 144(%rsp)
	movq	$8, 152(%rsp)
	movq	$0, 160(%rsp)
	je	.LBB363_279
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	leaq	144(%rsp), %rsi
	leaq	664(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_278
.LBB363_280:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	112(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_282
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_282:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_288
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_282
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB363_285:
	cmpq	%rax, %rsi
	jge	.LBB363_287
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB363_285
.LBB363_287:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_288:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	128(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB363_289
	jmp	.LBB363_291
.LBB363_279:
	movq	152(%rsp), %rcx
	movq	144(%rsp), %rax
	movq	160(%rsp), %rdx
	movq	%rcx, 224(%rsp)
	movq	168(%rsp), %rcx
	movq	%rax, 216(%rsp)
	movq	%rdx, 232(%rsp)
	movq	%rcx, 240(%rsp)
	movq	$-1, 208(%rsp)
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB363_280
.LBB363_278:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_291
.LBB363_289:
	lock		decq	(%rax)
	jne	.LBB363_291
	leaq	128(%rsp), %rdi
	#MEMBARRIER
.Ltmp15294:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15295:
.LBB363_291:
	vmovdqu64	208(%rsp), %zmm0
	vmovups	240(%rsp), %zmm1
.LBB363_292:
	movq	24(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	.cfi_escape 0x2e, 0x00
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.12908414067662811932)
	jmp	.LBB363_381
.LBB363_293:
	movq	904(%rsp), %r15
	movq	368(%rsp), %rax
	testq	%r15, %r15
	je	.LBB363_385
	lock		incq	(%r15)
	jle	.LBB363_430
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
	movq	%r15, 144(%rsp)
	leaq	16(%r15), %rsi
.Ltmp15380:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15381:
	lock		incq	(%rbx)
	jle	.LBB363_430
.Ltmp15383:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp15384:
	vmovups	232(%rsp), %ymm1
	vmovdqu	208(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_427
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB363_301
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_301:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_307
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_301
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB363_304:
	cmpq	%rax, %rdx
	jle	.LBB363_306
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_304
.LBB363_306:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_307:
	vmovdqu64	32(%rsp), %zmm0
	movq	96(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
	lock		decq	(%r15)
	jne	.LBB363_220
	movb	$1, %al
	#MEMBARRIER
	movl	%eax, 8(%rsp)
.Ltmp15388:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15389:
	jmp	.LBB363_220
.LBB363_309:
	xorl	%ebx, %ebx
	xorl	%ebp, %ebp
	movq	%rdx, %r15
	jmp	.LBB363_62
.LBB363_320:
	movl	136(%rsp), %eax
	movl	140(%rsp), %ecx
	xorl	%r14d, %r14d
	movl	%eax, 784(%r15)
	movl	%ecx, 788(%r15)
.Ltmp15332:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	808(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15333:
	vmovdqu64	32(%rsp), %zmm0
	vmovups	64(%rsp), %zmm1
	movq	24(%rsp), %rax
	xorl	%esi, %esi
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	movl	%esi, 8(%rsp)
	testq	%rbp, %rbp
	je	.LBB363_331
.LBB363_322:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$2, %rbp
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rbp
	cmovaeq	%rdx, %rbp
	xorl	%ecx, %ecx
	cmpq	%rbp, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rbp, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_324
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_324:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_330
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_324
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rbp, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rbp, %rcx
	setns	%al
	addq	%rdx, %rax
	subq	%rbp, %rcx
	cmovoq	%rax, %rcx
	movq	(%r12), %rax
	.p2align	4
.LBB363_327:
	cmpq	%rax, %rcx
	jge	.LBB363_329
	lock		cmpxchgq	%rcx, (%r12)
	jne	.LBB363_327
.LBB363_329:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_330:
	.cfi_escape 0x2e, 0x00
	movq	176(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_331:
	movq	320(%rsp), %rbx
	cmpq	$0, 192(%rsp)
	je	.LBB363_344
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	movabsq	$9223372036854775807, %r15
	xorl	%r14d, %r14d
	jmp	.LBB363_336
	.p2align	4
.LBB363_333:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_334:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	*%rbp
.LBB363_335:
	incq	%r14
	cmpq	192(%rsp), %r14
	je	.LBB363_344
.LBB363_336:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB363_335
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r15, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_339
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_339:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_334
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_339
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	lock		xaddq	%rdx, (%r12)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB363_342:
	cmpq	%rax, %rdx
	jge	.LBB363_333
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB363_342
	jmp	.LBB363_333
.LBB363_344:
	movq	312(%rsp), %rax
	movl	8(%rsp), %r14d
	testq	%rax, %rax
	je	.LBB363_354
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_347
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_347:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_353
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_347
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB363_350:
	cmpq	%rax, %rsi
	jge	.LBB363_352
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB363_350
.LBB363_352:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_353:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB363_354:
	cmpq	$-1, 496(%rsp)
	je	.LBB363_356
.Ltmp15444:
	.cfi_escape 0x2e, 0x00
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15445:
.LBB363_356:
	testb	%r14b, %r14b
	je	.LBB363_381
	movq	880(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_367
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	888(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_360
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_360:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_366
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_360
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB363_363:
	cmpq	%rax, %rsi
	jge	.LBB363_365
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB363_363
.LBB363_365:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_366:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_367:
	movq	808(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB363_377
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	816(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_370
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_370:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_376
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_370
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB363_373:
	cmpq	%rax, %rsi
	jge	.LBB363_375
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB363_373
.LBB363_375:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_376:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_377:
	movq	904(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_381
	lock		decq	(%rax)
	jne	.LBB363_381
	leaq	904(%rsp), %rdi
.LBB363_380:
	#MEMBARRIER
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB363_381:
	movq	24(%rsp), %rax
	addq	$1256, %rsp
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
	vzeroupper
	retq
.LBB363_382:
	.cfi_def_cfa_offset 1312
	movl	%eax, 8(%rsp)
.Ltmp15420:
	.cfi_escape 0x2e, 0x00
	leaq	1024(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r15, %rdx
	movq	%r13, %rcx
	vzeroupper
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15421:
	cmpl	$1, 1024(%rsp)
	jne	.LBB363_403
	vmovdqu64	1040(%rsp), %zmm0
	vmovups	1072(%rsp), %zmm1
	movq	24(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movb	$1, %al
	movl	%eax, 8(%rsp)
	jmp	.LBB363_331
.LBB363_385:
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
	movb	$1, %al
	movl	%eax, 8(%rsp)
.Ltmp15399:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15400:
	movq	%rax, %rsi
	movq	%rax, %r15
	movq	%rax, 32(%rsp)
	addq	$16, %rsi
.Ltmp15401:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15402:
	lock		decq	(%r15)
	jne	.LBB363_389
	#MEMBARRIER
.Ltmp15406:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15407:
.LBB363_389:
	lock		incq	(%rbx)
	jle	.LBB363_430
.Ltmp15408:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp15409:
	vmovups	232(%rsp), %ymm1
	vmovdqu	208(%rsp), %ymm0
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB363_429
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB363_394
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_394:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_219
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_394
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
.LBB363_397:
	cmpq	%rax, %rdx
	jle	.LBB363_218
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB363_397
	jmp	.LBB363_218
.LBB363_399:
	leaq	1144(%rsp), %rcx
.Ltmp15280:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	664(%rsp), %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15281:
	cmpq	$-1, 32(%rsp)
	je	.LBB363_410
	vmovups	664(%rsp), %zmm2
	vmovups	32(%rsp), %ymm0
	vmovups	704(%rsp), %zmm1
	vmovups	%zmm2, 32(%rsp)
	vmovups	%ymm0, 144(%rsp)
	vmovups	%zmm1, 72(%rsp)
	cmpq	$-1, 32(%rsp)
	je	.LBB363_414
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	leaq	144(%rsp), %rsi
	leaq	664(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB363_415
.LBB363_403:
	vmovups	1064(%rsp), %zmm1
	vmovdqu64	1032(%rsp), %zmm0
	vmovups	%zmm1, 64(%rsp)
	vmovdqu64	%zmm0, 32(%rsp)
.Ltmp15422:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	leaq	808(%rsp), %rsi
	leaq	32(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15423:
	cmpq	$-1, 208(%rsp)
	je	.LBB363_412
	vmovdqu	208(%rsp), %ymm0
	vmovdqu	%ymm0, 144(%rsp)
	movq	168(%rsp), %rsi
	addq	$16, %rsi
.Ltmp15424:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp15425:
	movq	384(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	jle	.LBB363_430
	movq	16(%rbx), %rdx
.Ltmp15427:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	callq	*%rax
.Ltmp15428:
	vmovups	232(%rsp), %ymm1
	vmovdqu	208(%rsp), %ymm0
	leaq	48(%rsp), %r14
	vmovups	%ymm1, 72(%rsp)
	vmovdqu	%ymm0, 48(%rsp)
	movq	$1, 32(%rsp)
	movq	$1, 40(%rsp)
.Ltmp15432:
	.cfi_escape 0x2e, 0x00
	movl	$8, %edi
	movl	$72, %esi
	vzeroupper
	callq	alloc::boxed::box_new_uninit
.Ltmp15433:
	movq	96(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 64(%rbx)
	vmovdqu64	32(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rbx)
.Ltmp15440:
	.cfi_escape 0x2e, 0x00
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15441:
	jmp	.LBB363_220
.LBB363_410:
	xorl	%ebx, %ebx
.Ltmp15284:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	664(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15285:
	vmovups	32(%rsp), %zmm0
	vmovups	64(%rsp), %zmm1
	jmp	.LBB363_292
.LBB363_412:
	movl	$0, 8(%rsp)
.Ltmp15442:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	leaq	808(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15443:
	vmovdqu64	32(%rsp), %zmm0
	vmovups	64(%rsp), %zmm1
	movq	24(%rsp), %rax
	movl	$0, 8(%rsp)
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB363_331
.LBB363_414:
	vmovups	144(%rsp), %ymm0
	vmovups	%ymm0, 216(%rsp)
	movq	$-1, 208(%rsp)
.LBB363_415:
	movq	104(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_417
	movq	112(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_417:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_291
	lock		decq	(%rax)
	jne	.LBB363_291
	xorl	%ebx, %ebx
	leaq	128(%rsp), %rdi
	#MEMBARRIER
.Ltmp15282:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15283:
	jmp	.LBB363_291
.LBB363_420:
	movq	80(%r14), %rax
	leaq	208(%rsp), %rcx
	movq	%rcx, 32(%rsp)
	movq	%rax, 208(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 40(%rsp)
.Ltmp15320:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4165.llvm.6298868053391388158(%rip), %rdi
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4166.llvm.6298868053391388158(%rip), %rdx
	leaq	32(%rsp), %rsi
	callq	*%rax
.Ltmp15321:
	jmp	.LBB363_430
.LBB363_421:
	movq	%rbx, 640(%rsp)
.Ltmp15346:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.537(%rip), %rdx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15347:
	jmp	.LBB363_430
.LBB363_422:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.539(%rip), %rcx
	movq	%rax, %rsi
	movq	%rcx, 448(%rsp)
.LBB363_423:
	movq	%rbx, 216(%rsp)
.Ltmp15352:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	448(%rsp), %rdx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15353:
	jmp	.LBB363_430
.LBB363_424:
.Ltmp15308:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movl	$16, %esi
	callq	*%rax
.Ltmp15309:
	jmp	.LBB363_430
.LBB363_425:
.Ltmp15374:
	leaq	48(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15375:
	jmp	.LBB363_430
.LBB363_426:
	movl	$1, %edi
	jmp	.LBB363_18
.LBB363_427:
.Ltmp15390:
	leaq	48(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15391:
	jmp	.LBB363_430
.LBB363_428:
.Ltmp15465:
	leaq	48(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15466:
	jmp	.LBB363_430
.LBB363_429:
.Ltmp15414:
	leaq	48(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15415:
.LBB363_430:
	ud2
.LBB363_431:
.Ltmp15434:
	movq	%rax, %r13
.Ltmp15435:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15436:
	jmp	.LBB363_435
.LBB363_432:
.Ltmp15437:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_433:
.Ltmp15429:
	movq	%rax, %r13
.Ltmp15430:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15431:
	jmp	.LBB363_435
.LBB363_434:
.Ltmp15426:
	movq	%rax, %r13
.LBB363_435:
	movb	$1, %bl
.Ltmp15438:
	.cfi_escape 0x2e, 0x00
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15439:
	movb	$1, %r14b
	jmp	.LBB363_512
.LBB363_436:
.Ltmp15416:
	movq	%rax, %r13
.Ltmp15417:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15418:
	jmp	.LBB363_457
.LBB363_437:
.Ltmp15419:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_438:
.Ltmp15403:
	lock		decq	(%r15)
	movb	$1, %bl
	movq	%rax, %r13
	jne	.LBB363_445
	movb	$1, %bl
	#MEMBARRIER
.Ltmp15404:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15405:
	movb	$1, %r14b
	jmp	.LBB363_512
.LBB363_441:
.Ltmp15467:
	movq	%rax, %r13
.Ltmp15468:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15469:
	jmp	.LBB363_457
.LBB363_442:
.Ltmp15470:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_443:
.Ltmp15452:
	lock		decq	(%r14)
	movb	$1, %bl
	movq	%rax, %r13
	jne	.LBB363_445
	movb	$1, %bl
	#MEMBARRIER
.Ltmp15453:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	32(%rsp), %rdi
	callq	*%rax
.Ltmp15454:
	movb	$1, %r14b
	jmp	.LBB363_512
.LBB363_446:
.Ltmp15410:
	movq	%rax, %r13
.Ltmp15411:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15412:
	jmp	.LBB363_457
.LBB363_447:
.Ltmp15413:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_448:
.Ltmp15392:
	movq	%rax, %r13
.Ltmp15393:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15394:
	jmp	.LBB363_452
.LBB363_449:
.Ltmp15395:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_450:
.Ltmp15385:
	movq	%rax, %r13
.Ltmp15386:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15387:
	jmp	.LBB363_452
.LBB363_451:
.Ltmp15382:
	movq	%rax, %r13
.LBB363_452:
	lock		decq	(%r15)
	movb	$1, %bl
	jne	.LBB363_445
	movb	$1, %bl
	#MEMBARRIER
.Ltmp15396:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.Ltmp15397:
	movb	$1, %r14b
	jmp	.LBB363_512
.LBB363_445:
	movb	$1, %r14b
	jmp	.LBB363_512
.LBB363_455:
.Ltmp15398:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_456:
.Ltmp15459:
	movq	%rax, %r13
.Ltmp15460:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15461:
.LBB363_457:
	movb	$1, %bl
	movb	$1, %r14b
	jmp	.LBB363_512
.LBB363_458:
.Ltmp15296:
	movq	%rax, %r13
	xorl	%ebx, %ebx
	jmp	.LBB363_470
.LBB363_459:
.Ltmp15464:
	movq	%rax, %r13
	xorl	%r14d, %r14d
	jmp	.LBB363_513
.LBB363_460:
.Ltmp15446:
	movq	%rax, %r13
	jmp	.LBB363_515
.LBB363_461:
.Ltmp15449:
	movl	8(%rsp), %r14d
	movb	$1, %bl
	movq	%rax, %r13
	jmp	.LBB363_512
.LBB363_462:
.Ltmp15334:
	movq	%rax, %r13
	jmp	.LBB363_510
.LBB363_463:
.Ltmp15291:
	movq	%rax, %r13
.Ltmp15292:
	.cfi_escape 0x2e, 0x00
	leaq	664(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15293:
	xorl	%ebx, %ebx
	jmp	.LBB363_470
.LBB363_465:
.Ltmp15376:
	movq	%rax, %r13
.Ltmp15377:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15378:
	movb	$1, %r14b
	xorl	%ebx, %ebx
	jmp	.LBB363_512
.LBB363_467:
.Ltmp15379:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_468:
.Ltmp15310:
	movq	%rax, %r13
	jmp	.LBB363_516
.LBB363_469:
.Ltmp15286:
	movq	%rax, %r13
.LBB363_470:
.Ltmp15297:
	.cfi_escape 0x2e, 0x00
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.12908414067662811932)
.Ltmp15298:
	testb	%bl, %bl
	jne	.LBB363_519
	jmp	.LBB363_542
.LBB363_472:
.Ltmp15313:
	movq	%rax, %r13
	testq	%rbp, %rbp
	je	.LBB363_516
	movq	8(%rsp), %rsi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	176(%rsp), %rdi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_516
.LBB363_474:
.Ltmp15373:
	movq	%rax, %r13
	movq	952(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_505
	movq	960(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_505
.LBB363_476:
.Ltmp15365:
	movq	%rax, %r13
	jmp	.LBB363_502
.LBB363_477:
.Ltmp15307:
	movq	32(%rsp), %rsi
	movq	%rax, %r13
	testq	%rsi, %rsi
	je	.LBB363_516
	movq	40(%rsp), %rdi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB363_516
.LBB363_479:
.Ltmp15368:
	vmovups	56(%rsp), %ymm1
	vmovdqu	32(%rsp), %ymm0
	movq	%rax, %r13
	vmovups	%ymm1, 520(%rsp)
	vmovdqu	%ymm0, 496(%rsp)
	jmp	.LBB363_502
.LBB363_480:
.Ltmp15337:
	movb	$1, %bl
	movq	%rax, %r13
	jmp	.LBB363_498
.LBB363_481:
.Ltmp15357:
	movq	%rax, %r13
	movq	%rbx, 216(%rsp)
	jmp	.LBB363_492
.LBB363_482:
.Ltmp15342:
	movb	$1, %bl
	movq	%rax, %r13
	jmp	.LBB363_496
.LBB363_483:
.Ltmp15345:
	movq	%rax, %r13
	movq	%rbx, 640(%rsp)
	jmp	.LBB363_487
.LBB363_484:
.Ltmp15351:
	movq	%rax, %r13
	movq	%rbx, 640(%rsp)
	cmpq	$5, %r12
	ja	.LBB363_489
	jmp	.LBB363_490
.LBB363_485:
.Ltmp15327:
	jmp	.LBB363_508
.LBB363_486:
.Ltmp15348:
	movq	%rax, %r13
.LBB363_487:
	movq	208(%rsp), %r12
	cmpq	$6, %r12
	jb	.LBB363_490
	movq	216(%rsp), %rbp
.LBB363_489:
	leaq	-8(,%r12,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB363_490:
	.cfi_escape 0x2e, 0x00
	leaq	632(%rsp), %rdi
	jmp	.LBB363_495
.LBB363_491:
.Ltmp15354:
	movq	%rax, %r13
.LBB363_492:
	cmpq	$5, %r12
	jbe	.LBB363_494
	leaq	-8(,%r12,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB363_494:
	.cfi_escape 0x2e, 0x00
	leaq	208(%rsp), %rdi
.LBB363_495:
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
.LBB363_496:
.Ltmp15358:
	.cfi_escape 0x2e, 0x00
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15359:
	movq	440(%rsp), %r15
.LBB363_498:
	lock		decq	(%r15)
	jne	.LBB363_500
	#MEMBARRIER
.Ltmp15360:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp15361:
.LBB363_500:
	testb	%bl, %bl
	je	.LBB363_502
	.cfi_escape 0x2e, 0x00
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB363_502:
	cmpb	$0, 912(%rsp)
	jne	.LBB363_505
	cmpq	$-1, 920(%rsp)
	je	.LBB363_505
.Ltmp15369:
	.cfi_escape 0x2e, 0x00
	leaq	920(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>>
.Ltmp15370:
	movq	8(%rsp), %rbp
	movb	$1, %r14b
	jmp	.LBB363_510
.LBB363_505:
	movq	8(%rsp), %rbp
	jmp	.LBB363_509
.LBB363_506:
.Ltmp15362:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_507:
.Ltmp15322:
.LBB363_508:
	movq	%rax, %r13
.LBB363_509:
	movb	$1, %r14b
.LBB363_510:
	movb	$1, %bl
	testq	%rbp, %rbp
	je	.LBB363_512
	shlq	$2, %rbp
	.cfi_escape 0x2e, 0x00
	movq	176(%rsp), %rdi
	movl	$4, %edx
	movq	%rbp, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB363_512:
	.cfi_escape 0x2e, 0x00
	leaq	312(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	496(%rsp), %r12
	movl	%ebx, %ebp
.LBB363_513:
	cmpq	$-1, %r12
	setne	%al
	testb	%bpl, %al
	je	.LBB363_515
.Ltmp15471:
	.cfi_escape 0x2e, 0x00
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15472:
.LBB363_515:
	testb	%r14b, %r14b
	je	.LBB363_542
.LBB363_516:
.Ltmp15473:
	.cfi_escape 0x2e, 0x00
	leaq	808(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15474:
	jmp	.LBB363_542
.LBB363_517:
.Ltmp15475:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB363_518:
.Ltmp15301:
	movq	%rax, %r13
.LBB363_519:
	movq	736(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB363_520
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	744(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_524
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_524:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_530
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_524
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB363_527:
	cmpq	%rax, %rsi
	jge	.LBB363_529
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB363_527
.LBB363_529:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_530:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	664(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB363_531
.LBB363_521:
	movq	760(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB363_540
	jmp	.LBB363_542
.LBB363_520:
	movq	664(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB363_521
.LBB363_531:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	672(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB363_533
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB363_533:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB363_539
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB363_533
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB363_536:
	cmpq	%rax, %rsi
	jge	.LBB363_538
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB363_536
.LBB363_538:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB363_539:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	760(%rsp), %rax
	testq	%rax, %rax
	je	.LBB363_542
.LBB363_540:
	lock		decq	(%rax)
	jne	.LBB363_542
	leaq	760(%rsp), %rdi
	#MEMBARRIER
.Ltmp15302:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15303:
.LBB363_542:
	.cfi_escape 0x2e, 0x00
	movq	%r13, %rdi
	callq	_Unwind_Resume@PLT
.LBB363_543:
.Ltmp15304:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end363:
purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>:
.Lfunc_begin364:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception271
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
	subq	$2008, %rsp
	.cfi_def_cfa_offset 2064
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovdqu	2072(%rsp), %xmm0
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm1
	movq	$0, 256(%rsp)
	movq	$8, 264(%rsp)
	movq	$0, 272(%rsp)
	movq	%r9, %rbp
	movq	%r8, %r13
	movq	%rdx, 64(%rsp)
	movq	%rsi, 112(%rsp)
	movq	%rdi, 48(%rsp)
	vmovdqu	%xmm0, 1152(%rsp)
	vmovdqu	%ymm1, 280(%rsp)
	testq	%r8, %r8
	je	.LBB364_5
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r15
	movq	%r13, %rbx
	shlq	$4, %rbx
	movq	%rcx, %r14
	leaq	256(%rsp), %r12
	addq	%rcx, %rbx
	.p2align	4
.LBB364_2:
	movq	(%r14), %rsi
	movq	8(%r14), %rdx
	lock		incq	(%rsi)
	jle	.LBB364_599
.Ltmp15476:
	movq	%r12, %rdi
	vzeroupper
	callq	*%r15
.Ltmp15477:
	addq	$16, %r14
	cmpq	%rbx, %r14
	jne	.LBB364_2
.LBB364_5:
	vmovdqu	280(%rsp), %ymm0
	movq	256(%rsp), %rax
	movq	264(%rsp), %rcx
	movq	272(%rsp), %rdx
	movq	280(%rsp), %rsi
	movq	2064(%rsp), %rdi
	movq	%r13, 864(%rsp)
	movq	%rax, 1168(%rsp)
	movq	%rcx, 1176(%rsp)
	movq	%rdx, 1184(%rsp)
	imulq	$120, %rdi, %rbx
	vmovdqu	%ymm0, 1192(%rsp)
	movq	%rsi, 1192(%rsp)
	leaq	(%rbp,%rbx), %rcx
	movq	1184(%rsp), %rax
	movq	%rcx, 1256(%rsp)
	movq	%rax, 248(%rsp)
	testq	%rdi, %rdi
	je	.LBB364_11
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r12
	leaq	1168(%rsp), %r14
	movq	%rbp, %r13
	.p2align	4
.LBB364_7:
	movq	(%r13), %rsi
	movq	1184(%rsp), %r15
	lock		incq	(%rsi)
	jle	.LBB364_599
	movq	8(%r13), %rdx
.Ltmp15482:
	movq	%r14, %rdi
	vzeroupper
	callq	*%r12
.Ltmp15483:
	cmpq	%r15, %rax
	jne	.LBB364_56
	addq	$120, %r13
	leaq	(%rbp,%rbx), %rax
	cmpq	%rax, %r13
	jne	.LBB364_7
.LBB364_11:
	movq	1168(%rsp), %rax
	vmovdqu	1200(%rsp), %xmm0
	movq	1192(%rsp), %rdi
	movq	1176(%rsp), %rcx
	movq	1184(%rsp), %rdx
	movq	1216(%rsp), %r8
	movq	1192(%rsp), %rsi
	movq	%rax, 272(%rsp)
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rdi, 296(%rsp)
	movl	$72, %edi
	movq	%r8, 320(%rsp)
	movq	%rcx, 280(%rsp)
	movq	%rdx, 288(%rsp)
	movq	%rsi, 296(%rsp)
	vmovdqu	%xmm0, 304(%rsp)
	movq	$1, 256(%rsp)
	movq	$1, 264(%rsp)
	vzeroupper
	callq	*%rax
	movq	%rax, 88(%rsp)
	testq	%rax, %rax
	je	.LBB364_592
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB364_14
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB364_14:
	movq	2088(%rsp), %r15
	movq	2064(%rsp), %rsi
	leaq	264(%rsp), %r14
	.p2align	4
.LBB364_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_21
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_15
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$72, (%rcx)
	movl	$72, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	$72, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB364_18:
	cmpq	%rax, %rcx
	jle	.LBB364_20
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB364_18
.LBB364_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_21:
	vmovdqu64	256(%rsp), %zmm0
	movq	88(%rsp), %rcx
	movq	320(%rsp), %rax
	movq	%rax, 64(%rcx)
	movq	%rcx, 936(%rsp)
	vmovdqu64	%zmm0, (%rcx)
	testq	%rsi, %rsi
	je	.LBB364_25
	xorl	%eax, %eax
	.p2align	4
.LBB364_23:
	cmpl	$7, 16(%rbp,%rax)
	je	.LBB364_97
	addq	$120, %rax
	cmpq	%rax, %rbx
	jne	.LBB364_23
.LBB364_25:
	vmovdqu	1152(%rsp), %xmm0
	movq	88(%rsp), %rax
	movq	%rbp, 512(%rsp)
	movb	$1, %bpl
	movq	%rsi, 520(%rsp)
	movq	%rax, 80(%rsp)
	vmovdqu	%xmm0, 1280(%rsp)
.Ltmp15674:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15675:
	movq	80(%rsp), %rsi
	movq	%rax, 104(%rsp)
	movq	520(%rsp), %rbx
	movq	32(%rsi), %rcx
	movq	%rcx, %r13
	subq	%rbx, %r13
	movq	%r13, 56(%rsp)
	movq	32(%rsi), %rdx
	cmpq	%rdx, %r13
	ja	.LBB364_579
	movl	$8, %r12d
	cmpq	%rbx, %rcx
	je	.LBB364_41
	movq	malloc@GOTPCREL(%rip), %rax
	movq	24(%rsi), %rbp
	movq	%r13, %r15
	shlq	$4, %r15
	movq	%r15, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB364_595
	movq	%rax, %r12
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r15, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r15, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB364_31
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_31:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_37
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_31
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	%r15, (%rcx)
	movq	%r15, %rcx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	%r15, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB364_34:
	cmpq	%rax, %rcx
	jle	.LBB364_36
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB364_34
.LBB364_36:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_37:
	movq	%rbx, 608(%rsp)
	xorl	%ebx, %ebx
	movq	%r13, %r14
	.p2align	4
.LBB364_38:
	movq	104(%rsp), %rdi
	leaq	(%rbp,%rbx), %rsi
	addq	$16, %rdi
.Ltmp15676:
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)
.Ltmp15677:
	movq	%rax, (%r12,%rbx)
	movq	%rdx, 8(%r12,%rbx)
	addq	$16, %rbx
	decq	%r14
	jne	.LBB364_38
	movq	104(%rsp), %rax
	movq	520(%rsp), %rcx
	movq	2088(%rsp), %r15
	movq	608(%rsp), %rbx
.LBB364_41:
	movq	512(%rsp), %rdx
	addq	$16, %rax
	movb	$1, %bpl
	movq	%r13, 560(%rsp)
	movq	%r12, 568(%rsp)
	movq	%r13, 576(%rsp)
.Ltmp15681:
	movq	112(%rsp), %rsi
	leaq	480(%rsp), %r14
	movq	%rax, %r8
	movq	%r15, %r9
	movq	%r14, %rdi
	callq	purrdf_sparql_eval::modifier::link_aggregates::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15682:
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	104(%rsp), %rdx
	movb	$-1, 584(%rsp)
	addq	$16, %rdx
	vmovdqu	%ymm0, 192(%rsp)
.Ltmp15683:
	movq	<&[purrdf_sparql_algebra::ast::Variable] as purrdf_sparql_eval::modifier::GroupDomain>::visible@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %r12
	leaq	1280(%rsp), %rsi
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15684:
	movq	112(%rsp), %rax
	leaq	104(%rsp), %r9
	leaq	560(%rsp), %r8
	leaq	584(%rsp), %r13
	movq	%rax, 944(%rsp)
	movq	%r9, 952(%rsp)
	movq	%r8, 960(%rsp)
	leaq	192(%rsp), %r9
	leaq	512(%rsp), %r8
	movq	%r9, 968(%rsp)
	movq	%r8, 976(%rsp)
	leaq	944(%rsp), %r9
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.542(%rip), %r8
	movq	%r13, 984(%rsp)
	movq	%r14, 992(%rsp)
	movq	%r12, 1000(%rsp)
	movq	%r9, 256(%rsp)
	movq	%r8, 264(%rsp)
	movb	$0, 272(%rsp)
.Ltmp15686:
	movq	64(%rsp), %rsi
	leaq	1712(%rsp), %rdi
	leaq	256(%rsp), %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::eval::eval_yielding::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15687:
	cmpl	$1, 1712(%rsp)
	jne	.LBB364_66
	vmovdqu64	1728(%rsp), %zmm0
	vmovdqu64	1760(%rsp), %zmm1
	movq	48(%rsp), %rax
	movq	528(%rsp), %rcx
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	testq	%rcx, %rcx
	jle	.LBB364_55
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	536(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_48
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_48:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_54
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_48
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_51:
	cmpq	%rax, %rdx
	jge	.LBB364_53
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_51
.LBB364_53:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_54:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_55:
	movb	$1, %bpl
	jmp	.LBB364_173
.LBB364_56:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$51, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB364_594
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$51, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$51, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB364_59
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_59:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_65
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_59
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$51, (%rsi)
	movl	$51, %esi
	lock		xaddq	%rsi, (%rdi)
	addq	$51, %rsi
	cmovoq	%rdx, %rsi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB364_62:
	cmpq	%rax, %rsi
	jle	.LBB364_64
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB364_62
.LBB364_64:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_65:
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528+19(%rip), %ymm0
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528(%rip), %ymm1
	movq	48(%rsp), %rdx
	movabsq	$9223372036854775793, %rax
	leaq	1168(%rsp), %rdi
	addq	$35, %rax
	movq	%rax, 16(%rdx)
	movq	$51, 24(%rdx)
	movq	%rcx, 32(%rdx)
	movq	$51, 40(%rdx)
	movq	$1, (%rdx)
	vmovups	%ymm0, 19(%rcx)
	vmovups	%ymm1, (%rcx)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
	jmp	.LBB364_533
.LBB364_66:
	leaq	1720(%rsp), %r14
.Ltmp15689:
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	112(%rsp), %rsi
	leaq	1496(%rsp), %rdi
	callq	*%rax
.Ltmp15690:
	movb	$1, %bpl
.Ltmp15694:
	leaq	256(%rsp), %rdi
	leaq	1496(%rsp), %rsi
	movb	$1, %r15b
	xorl	%edx, %edx
	movq	%r14, %rcx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15695:
	cmpq	$-1, 256(%rsp)
	je	.LBB364_140
.Ltmp15696:
	leaq	256(%rsp), %rdi
	movb	$1, %r15b
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15697:
	movzbl	584(%rsp), %eax
	leaq	585(%rsp), %r12
	cmpb	$-1, %al
	je	.LBB364_142
	vmovdqu	(%r12), %xmm0
	movq	15(%r12), %rcx
	movq	%rcx, 143(%rsp)
	vmovdqa	%xmm0, 128(%rsp)
.LBB364_72:
	vmovdqa	128(%rsp), %xmm0
	movb	%al, 656(%rsp)
	movq	143(%rsp), %rax
	movq	80(%rsp), %rcx
	xorl	%ebp, %ebp
	vmovdqu	%xmm0, 657(%rsp)
	movq	%rax, 672(%rsp)
.Ltmp15698:
	movq	112(%rsp), %rsi
	leaq	256(%rsp), %rdi
	leaq	656(%rsp), %rdx
	movb	$1, %r15b
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp15699:
.LBB364_73:
	vmovdqu64	256(%rsp), %zmm0
	vmovdqu64	288(%rsp), %zmm1
	movq	48(%rsp), %rax
	movb	$1, %r15b
	xorl	%ebp, %ebp
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.LBB364_74:
	movq	1568(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB364_84
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1576(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_77
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_77:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_83
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_77
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_80:
	cmpq	%rax, %rdx
	jge	.LBB364_82
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_80
.LBB364_82:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_83:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_84:
	movq	1496(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB364_94
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1504(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_87
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_87:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_93
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_87
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_90:
	cmpq	%rax, %rdx
	jge	.LBB364_92
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_90
.LBB364_92:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_93:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_94:
	movq	1592(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_162
	lock		decq	(%rax)
	jne	.LBB364_162
	leaq	1592(%rsp), %rdi
	#MEMBARRIER
.Ltmp15726:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15727:
	jmp	.LBB364_162
.LBB364_97:
	movb	$1, %r12b
	movq	%rbp, 120(%rsp)
.Ltmp15490:
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	112(%rsp), %rsi
	leaq	1368(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15491:
	movb	$1, %bpl
.Ltmp15492:
	movq	64(%rsp), %rcx
	leaq	1600(%rsp), %rdi
	movb	$1, %r12b
	movq	%r15, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15493:
	cmpl	$1, 1600(%rsp)
	jne	.LBB364_123
	vmovdqu64	1616(%rsp), %zmm0
	vmovdqu64	1648(%rsp), %zmm1
	movq	48(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movq	1440(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB364_110
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1448(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_103
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_103:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_109
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_103
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_106:
	cmpq	%rax, %rdx
	jge	.LBB364_108
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_106
.LBB364_108:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_109:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_110:
	movq	1368(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB364_120
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1376(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_113
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_113:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_119
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_113
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_116:
	cmpq	%rax, %rdx
	jge	.LBB364_118
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_116
.LBB364_118:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_119:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_120:
	movq	1464(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_531
	lock		decq	(%rax)
	jne	.LBB364_531
	movb	$1, %r12b
	leaq	1464(%rsp), %rdi
	#MEMBARRIER
.Ltmp15672:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15673:
	jmp	.LBB364_531
.LBB364_123:
	vmovdqu64	1640(%rsp), %zmm1
	vmovdqu64	1608(%rsp), %zmm0
	vmovdqu64	%zmm1, 288(%rsp)
	vmovdqu64	%zmm0, 256(%rsp)
.Ltmp15494:
	leaq	656(%rsp), %rdi
	leaq	1368(%rsp), %rsi
	leaq	256(%rsp), %rcx
	xorl	%edx, %edx
	movb	$1, %r12b
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15495:
	cmpq	$-1, 656(%rsp)
	je	.LBB364_146
	vmovdqu	656(%rsp), %ymm0
	vmovdqu	%ymm0, 624(%rsp)
	movq	648(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB364_599
	movq	88(%rsp), %rcx
	movq	648(%rsp), %rax
	movq	248(%rsp), %rsi
	movq	%r14, 920(%rsp)
	movq	32(%rcx), %rdx
	movq	%rax, 56(%rsp)
	cmpq	%rdx, %rsi
	ja	.LBB364_580
	movq	%rsi, %r14
	shlq	$4, %r14
	movq	%rsi, 232(%rsp)
	movq	%r14, 224(%rsp)
	testq	%rsi, %rsi
	je	.LBB364_203
	movq	88(%rsp), %rax
	movq	%r14, %rdi
	movq	24(%rax), %rbx
	movq	malloc@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	%rax, 96(%rsp)
	testq	%rax, %rax
	je	.LBB364_597
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r14, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %r14
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%r14, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB364_131
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_131:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_137
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_131
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rax
	movq	224(%rsp), %rdx
	lock		addq	%rdx, (%rax)
	movq	%rcx, %rdx
	lock		xaddq	%rdx, (%rsi)
	movabsq	$-9223372036854775808, %rsi
	leaq	(%rdx,%rcx), %rax
	sarq	$63, %rax
	xorq	%rax, %rsi
	addq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rsi, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_134:
	cmpq	%rax, %rdx
	jle	.LBB364_136
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_134
.LBB364_136:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_137:
	movq	232(%rsp), %r15
	xorl	%r14d, %r14d
	.p2align	4
.LBB364_138:
	movq	56(%rsp), %rdi
	leaq	(%rbx,%r14), %rsi
	addq	$16, %rdi
.Ltmp15496:
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)
.Ltmp15497:
	movq	96(%rsp), %rcx
	movq	%rax, (%rcx,%r14)
	movq	%rdx, 8(%rcx,%r14)
	addq	$16, %r14
	decq	%r15
	jne	.LBB364_138
	jmp	.LBB364_204
.LBB364_140:
	vmovdqu64	1536(%rsp), %zmm1
	vmovdqu64	1496(%rsp), %zmm0
	movq	80(%rsp), %rax
	movq	%rax, 152(%rsp)
	movq	$0, 128(%rsp)
	movq	$8, 136(%rsp)
	movq	$0, 144(%rsp)
	vmovdqu64	%zmm1, 296(%rsp)
	vmovdqu64	%zmm0, 256(%rsp)
	cmpq	$-1, 256(%rsp)
	je	.LBB364_148
	leaq	656(%rsp), %rdi
	leaq	128(%rsp), %rsi
	leaq	1496(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	328(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB364_149
	jmp	.LBB364_158
.LBB364_142:
	movq	2088(%rsp), %r9
	movq	696(%r9), %rcx
	movl	40(%rcx), %eax
	testl	%eax, %eax
	je	.LBB364_267
.LBB364_143:
	movq	216(%rsp), %rax
	orq	%rax, 864(%rsp)
	setne	%cl
	testq	%rbx, %rbx
	sete	%dl
	orb	%cl, %dl
	je	.LBB364_264
.LBB364_144:
	movq	192(%rsp), %rcx
	movq	200(%rsp), %rdx
	vmovdqa	(%rcx), %xmm0
	testq	%rdx, %rdx
	je	.LBB364_293
	leaq	(,%rdx,8), %rsi
	movq	%rcx, %rdi
	leaq	(%rsi,%rsi,8), %r8
	andq	$-16, %r8
	subq	%r8, %rdi
	leaq	97(%r8,%rdx), %rsi
	movl	$16, %r8d
	addq	$-80, %rdi
	jmp	.LBB364_294
.LBB364_146:
	vmovups	1408(%rsp), %zmm1
	vmovups	1368(%rsp), %zmm0
	movq	88(%rsp), %rax
	movq	%rax, 968(%rsp)
	movq	$0, 944(%rsp)
	movq	$8, 952(%rsp)
	movq	$0, 960(%rsp)
	vmovups	%zmm1, 296(%rsp)
	vmovups	%zmm0, 256(%rsp)
	cmpq	$-1, 256(%rsp)
	je	.LBB364_276
	leaq	656(%rsp), %rdi
	leaq	944(%rsp), %rsi
	leaq	1368(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB364_277
.LBB364_148:
	movq	136(%rsp), %rcx
	movq	128(%rsp), %rax
	movq	144(%rsp), %rdx
	movq	%rcx, 672(%rsp)
	movq	152(%rsp), %rcx
	movq	%rax, 664(%rsp)
	movq	%rdx, 680(%rsp)
	movq	%rcx, 688(%rsp)
	movq	$-1, 656(%rsp)
	movq	328(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB364_158
.LBB364_149:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	336(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_151
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_151:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_157
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_151
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_154:
	cmpq	%rax, %rdx
	jge	.LBB364_156
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_154
.LBB364_156:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_157:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_158:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_161
	lock		decq	(%rax)
	jne	.LBB364_161
	leaq	352(%rsp), %rdi
	#MEMBARRIER
.Ltmp15737:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15738:
.LBB364_161:
	vmovdqu64	656(%rsp), %zmm0
	vmovdqu64	688(%rsp), %zmm1
	movq	48(%rsp), %rax
	movb	$1, %r15b
	xorl	%ebp, %ebp
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.LBB364_162:
	movq	528(%rsp), %rcx
	testq	%rcx, %rcx
	jle	.LBB364_172
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	536(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_165
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_165:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_171
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_165
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_168:
	cmpq	%rax, %rdx
	jge	.LBB364_170
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_168
.LBB364_170:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_171:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_172:
	testb	%r15b, %r15b
	je	.LBB364_174
.LBB364_173:
.Ltmp15742:
	leaq	192(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>), purrdf_hash::fixed::FixedState>>
.Ltmp15743:
.LBB364_174:
	movq	488(%rsp), %r14
	movq	496(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_178
	movl	$1, %r12d
	movq	%r14, %r15
	subq	%rax, %r12
	.p2align	4
.LBB364_176:
.Ltmp15747:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp15748:
	incq	%r12
	addq	$24, %r15
	cmpq	$1, %r12
	jne	.LBB364_176
.LBB364_178:
	movq	480(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_188
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_181
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_181:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_187
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_181
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_184:
	cmpq	%rax, %rdx
	jge	.LBB364_186
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_184
.LBB364_186:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_187:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB364_188:
	movq	560(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB364_198
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$4, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	568(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_191
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_191:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_197
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_191
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_194:
	cmpq	%rax, %rdx
	jge	.LBB364_196
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_194
.LBB364_196:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_197:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_198:
	movq	104(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_200
	#MEMBARRIER
.Ltmp15753:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	104(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15754:
.LBB364_200:
	testb	%bpl, %bpl
	je	.LBB364_533
	movq	80(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_533
	xorl	%r12d, %r12d
	#MEMBARRIER
.Ltmp15756:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15757:
	jmp	.LBB364_533
.LBB364_203:
	movl	$8, %eax
	movq	%rax, 96(%rsp)
.LBB364_204:
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	640(%rsp), %rax
	leaq	256(%rsp), %r11
	vmovdqu	%ymm0, 880(%rsp)
	testq	%rax, %rax
	je	.LBB364_253
	movq	632(%rsp), %r15
	movq	96(%rsp), %rsi
	movq	224(%rsp), %rbp
	leaq	(%rax,%rax,4), %rax
	movl	$2, %edx
	leaq	264(%rsp), %r13
	xorl	%ecx, %ecx
	vmovd	%edx, %xmm0
	vmovdqa	%xmm0, 608(%rsp)
	leaq	(%r15,%rax,8), %rax
	movq	%rax, 1264(%rsp)
	leaq	(%rsi,%rbp), %rax
	negq	%rbp
	movq	%rax, 240(%rsp)
	jmp	.LBB364_208
	.p2align	4
.LBB364_206:
	movq	-16(%r12), %rax
	movq	872(%rsp), %rcx
	leaq	256(%rsp), %r11
	movq	%rcx, (%rax,%rbx,8)
	incq	%rbx
	movq	%rbx, -8(%r12)
	addq	$40, %r15
	incq	%rcx
	cmpq	1264(%rsp), %r15
	je	.LBB364_252
.LBB364_208:
	cmpq	$5, 232(%rsp)
	movl	$1, %r12d
	movl	$4, %eax
	movq	%rcx, 872(%rsp)
	movq	$1, 256(%rsp)
	movq	%r13, %rcx
	movq	%r11, %r14
	jae	.LBB364_219
	leaq	-1(%r12), %rdx
	cmpq	%rax, %rdx
	jae	.LBB364_221
.LBB364_210:
	movq	96(%rsp), %r9
	leaq	8(%r15), %rdx
	incq	%rax
	xorl	%r8d, %r8d
	jmp	.LBB364_214
	.p2align	4
.LBB364_211:
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB364_586
	vmovq	(%r10,%rdi,8), %xmm0
.LBB364_213:
	vmovq	%xmm0, -8(%rcx,%r12,8)
	addq	$16, %r9
	incq	%r12
	addq	$-16, %r8
	cmpq	%r12, %rax
	je	.LBB364_218
.LBB364_214:
	cmpq	%r8, %rbp
	je	.LBB364_232
	vmovdqa	608(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB364_213
	movq	(%r15), %rsi
	movq	%rdx, %r10
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB364_211
	movq	16(%r15), %rsi
	movq	8(%r15), %r10
	decq	%rsi
	jmp	.LBB364_211
	.p2align	4
.LBB364_232:
	movq	%r12, (%r14)
	jmp	.LBB364_233
	.p2align	4
.LBB364_218:
	movq	96(%rsp), %rbx
	subq	%r8, %rbx
	movq	%rax, (%r14)
	cmpq	240(%rsp), %rbx
	jne	.LBB364_222
	jmp	.LBB364_233
.LBB364_219:
.Ltmp15501:
	movq	232(%rsp), %rdx
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%r11, %rdi
	xorl	%esi, %esi
	movq	%r11, %rbx
	vzeroupper
	callq	*%rax
.Ltmp15502:
	movq	256(%rsp), %rax
	xorl	%edx, %edx
	movl	$4, %ecx
	leaq	272(%rsp), %rsi
	movq	%rbx, %r14
	movq	%rbx, %r11
	decq	%rax
	cmpq	$5, %rax
	cmovbq	%rcx, %rax
	movq	264(%rsp), %rcx
	setae	%dl
	cmovaeq	%rsi, %r14
	cmovbq	%r13, %rcx
	shll	$4, %edx
	movq	256(%rsp,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rax, %rdx
	jb	.LBB364_210
	.p2align	4
.LBB364_221:
	movq	96(%rsp), %rbx
	movq	%r12, %rax
	movq	%rax, (%r14)
	cmpq	240(%rsp), %rbx
	je	.LBB364_233
.LBB364_222:
	leaq	8(%r15), %r14
	.p2align	4
.LBB364_223:
	vmovdqa	608(%rsp), %xmm0
	cmpl	$1, (%rbx)
	vmovdqa	%xmm0, 64(%rsp)
	jne	.LBB364_228
	movq	(%r15), %rsi
	movq	%r14, %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB364_226
	movq	16(%r15), %rsi
	movq	8(%r15), %rax
	decq	%rsi
.LBB364_226:
	movq	8(%rbx), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB364_585
	vmovq	(%rax,%rdi,8), %xmm0
	vmovdqa	%xmm0, 64(%rsp)
.LBB364_228:
	movq	256(%rsp), %rsi
	movq	264(%rsp), %rax
	xorl	%edx, %edx
	leaq	272(%rsp), %rdi
	movq	%r11, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%r13, %rax
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	256(%rsp,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rsi, %rdx
	je	.LBB364_230
.LBB364_229:
	vmovdqa	64(%rsp), %xmm0
	addq	$16, %rbx
	vmovq	%xmm0, -8(%rax,%r12,8)
	incq	%r12
	movq	%r12, (%rcx)
	cmpq	240(%rsp), %rbx
	jne	.LBB364_223
	jmp	.LBB364_233
.LBB364_230:
.Ltmp15510:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r11, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15511:
	cmpq	$6, 256(%rsp)
	movq	264(%rsp), %rax
	leaq	256(%rsp), %r11
	leaq	272(%rsp), %rdx
	movq	%r11, %rcx
	cmovbq	%r13, %rax
	cmovaeq	%rdx, %rcx
	jmp	.LBB364_229
	.p2align	4
.LBB364_233:
	vmovdqu	256(%rsp), %ymm0
	movq	288(%rsp), %rax
	movq	904(%rsp), %r14
	movq	%rax, 688(%rsp)
	vmovdqu	%ymm0, 656(%rsp)
.Ltmp15513:
	leaq	880(%rsp), %rsi
	leaq	656(%rsp), %rdx
	movq	%r11, %rdi
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::rustc_entry
.Ltmp15514:
	movq	256(%rsp), %rax
	movq	264(%rsp), %r12
	testq	%rax, %rax
	je	.LBB364_247
	vmovdqu	16(%r13), %xmm0
	movq	%rax, 64(%rsp)
	movq	272(%rsp), %rcx
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r12, 928(%rsp)
	movq	296(%rsp), %r12
	movq	304(%rsp), %rbx
	movl	$8, %edi
	movq	%rcx, 1272(%rsp)
	vmovdqa	%xmm0, 1472(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB364_591
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	leaq	256(%rsp), %r11
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$8, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$8, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB364_238
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_238:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_244
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_238
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$8, (%rdx)
	movl	$8, %edx
	lock		xaddq	%rdx, (%rdi)
	addq	$8, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB364_241:
	cmpq	%rax, %rdx
	jle	.LBB364_243
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB364_241
.LBB364_243:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_244:
	movq	872(%rsp), %rax
	movq	%rax, (%rcx)
	movq	8(%r12), %rdx
	movq	(%r12), %rax
	movq	%rdx, %rsi
	andq	%rbx, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	je	.LBB364_249
.LBB364_245:
	tzcntl	%edi, %edi
	addq	%rsi, %rdi
	andq	%rdx, %rdi
	movzbl	(%rax,%rdi), %esi
	testb	%sil, %sil
	jns	.LBB364_251
.LBB364_246:
	shrq	$57, %rbx
	leaq	-16(%rdi), %r8
	andb	$1, %sil
	movb	%bl, (%rax,%rdi)
	negq	%rdi
	andq	%rdx, %r8
	movzbl	%sil, %esi
	leaq	(%rdi,%rdi,8), %rdx
	movq	64(%rsp), %rdi
	movb	%bl, 16(%rax,%r8)
	subq	%rsi, 16(%r12)
	movq	928(%rsp), %r8
	movq	%rdi, -72(%rax,%rdx,8)
	movq	1272(%rsp), %rdi
	movq	%r8, -64(%rax,%rdx,8)
	movq	%rdi, -56(%rax,%rdx,8)
	vmovdqa	1472(%rsp), %xmm0
	vmovdqu	%xmm0, -48(%rax,%rdx,8)
	movq	%r14, -32(%rax,%rdx,8)
	movq	$1, -24(%rax,%rdx,8)
	movq	%rcx, -16(%rax,%rdx,8)
	movq	$1, -8(%rax,%rdx,8)
	incq	24(%r12)
	movq	872(%rsp), %rcx
	addq	$40, %r15
	incq	%rcx
	cmpq	1264(%rsp), %r15
	jne	.LBB364_208
	jmp	.LBB364_252
	.p2align	4
.LBB364_247:
	movq	-8(%r12), %rbx
	cmpq	-24(%r12), %rbx
	jne	.LBB364_206
.Ltmp15518:
	movq	<alloc::raw_vec::RawVec<usize>>::grow_one@GOTPCREL(%rip), %rax
	leaq	-24(%r12), %rdi
	callq	*%rax
.Ltmp15519:
	jmp	.LBB364_206
.LBB364_249:
	movl	$16, %r8d
.LBB364_250:
	addq	%r8, %rsi
	addq	$16, %r8
	andq	%rdx, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	jne	.LBB364_245
	jmp	.LBB364_250
.LBB364_251:
	vmovdqa	(%rax), %xmm0
	vpmovmskb	%xmm0, %esi
	xorl	%edi, %edi
	tzcntl	%esi, %edi
	movzbl	(%rax,%rdi), %esi
	jmp	.LBB364_246
.LBB364_252:
	movq	904(%rsp), %rax
	movq	864(%rsp), %rcx
	orq	%rax, %rcx
	testq	%rcx, %rcx
	jne	.LBB364_258
	jmp	.LBB364_254
.LBB364_253:
	movq	864(%rsp), %rcx
	xorl	%eax, %eax
	testq	%rcx, %rcx
	jne	.LBB364_258
.LBB364_254:
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqa	%xmm0, 656(%rsp)
	movq	$8, 672(%rsp)
	movq	$1, 256(%rsp)
	movq	$0, 680(%rsp)
.Ltmp15521:
	leaq	944(%rsp), %rdi
	leaq	880(%rsp), %rsi
	leaq	256(%rsp), %rdx
	leaq	656(%rsp), %rcx
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::insert
.Ltmp15522:
	movq	952(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB364_257
	movq	960(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB364_257:
	movq	904(%rsp), %rax
.LBB364_258:
	movq	880(%rsp), %rcx
	movq	888(%rsp), %rsi
	vmovdqa	(%rcx), %xmm0
	testq	%rsi, %rsi
	je	.LBB364_260
	leaq	(,%rsi,8), %rdx
	movq	%rcx, %r8
	movl	$16, %r9d
	leaq	(%rdx,%rdx,8), %rdx
	andq	$-16, %rdx
	subq	%rdx, %r8
	leaq	97(%rdx,%rsi), %rdi
	addq	$-80, %r8
	jmp	.LBB364_261
.LBB364_260:
	xorl	%r9d, %r9d
.LBB364_261:
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	leaq	1(%rcx,%rsi), %rsi
	leaq	16(%rcx), %rdx
	movq	%r9, 944(%rsp)
	movq	%rdi, 952(%rsp)
	movq	%r8, 960(%rsp)
	movq	%rcx, 968(%rsp)
	vpcmpgtb	%xmm1, %xmm0, %k0
	movq	%rdx, 976(%rsp)
	movq	%rsi, 984(%rsp)
	kmovw	%k0, 992(%rsp)
	movq	%rax, 1000(%rsp)
	testq	%rax, %rax
	je	.LBB364_275
	kortestw	%k0, %k0
	je	.LBB364_268
	kmovd	%k0, %esi
	jmp	.LBB364_271
.LBB364_264:
	movq	520(%rsp), %rdx
	movq	512(%rsp), %rsi
.Ltmp15700:
	leaq	256(%rsp), %rdi
	leaq	584(%rsp), %r8
	movq	%r9, %rcx
	movb	$1, %r15b
	callq	purrdf_sparql_eval::modifier::contextual_lanes::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15701:
	vmovdqa	272(%rsp), %xmm0
	movq	256(%rsp), %rcx
	movq	264(%rsp), %rax
	vmovdqa	%xmm0, 1344(%rsp)
	cmpq	$-1, %rcx
	je	.LBB364_302
	vmovdqu64	288(%rsp), %zmm0
	vmovdqa	1344(%rsp), %xmm1
	movq	48(%rsp), %rdx
	movb	$1, %r15b
	movb	$1, %bpl
	vmovdqu64	%zmm0, 48(%rdx)
	vmovdqa	%xmm1, 32(%rdx)
	movq	%rcx, 16(%rdx)
	movq	%rax, 24(%rdx)
	movq	$1, (%rdx)
	jmp	.LBB364_74
.LBB364_267:
	movq	32(%rcx), %rdx
	movzbl	16(%rcx), %eax
	movq	%rdx, 143(%rsp)
	vmovdqu	17(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
	cmpb	$-1, %al
	jne	.LBB364_72
	jmp	.LBB364_143
.LBB364_268:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	.p2align	4
.LBB364_269:
	vpcmpltb	(%rdx), %xmm0, %k0
	addq	$-1152, %rcx
	addq	$16, %rdx
	kortestw	%k0, %k0
	je	.LBB364_269
	kmovd	%k0, %esi
	movq	%rdx, 976(%rsp)
	movq	%rcx, 968(%rsp)
.LBB364_271:
	xorl	%edx, %edx
	blsrl	%esi, %edx
	tzcntl	%esi, %esi
	leaq	-1(%rax), %rdi
	negq	%rsi
	movw	%dx, 992(%rsp)
	movq	%rdi, 1000(%rsp)
	leaq	(%rsi,%rsi,8), %rdx
	movq	-24(%rcx,%rdx,8), %r14
	cmpq	$-1, %r14
	je	.LBB364_275
	leaq	(%rcx,%rdx,8), %rcx
	cmpq	$5, %rax
	movl	$4, %r12d
	movabsq	$128102389400760776, %rdx
	cmovaeq	%rax, %r12
	decq	%rdx
	movq	-40(%rcx), %r8
	movq	-64(%rcx), %rsi
	movq	-32(%rcx), %rdi
	movq	-72(%rcx), %r13
	movq	%r8, 144(%rsp)
	movq	%rsi, 608(%rsp)
	vmovdqu	-56(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
	movq	-16(%rcx), %rsi
	movq	%rsi, 64(%rsp)
	leaq	(,%r12,8), %rsi
	leaq	(%rsi,%rsi,8), %rbx
	cmpq	%rdx, %rax
	jbe	.LBB364_291
	xorl	%ebp, %ebp
.LBB364_274:
.Ltmp15529:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15530:
	jmp	.LBB364_599
.LBB364_275:
	leaq	944(%rsp), %rdi
	movq	$0, 480(%rsp)
	movq	$8, 488(%rsp)
	movq	$0, 496(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	movl	$8, %ebx
	xorl	%r12d, %r12d
	jmp	.LBB364_321
.LBB364_276:
	movq	952(%rsp), %rcx
	movq	944(%rsp), %rax
	movq	960(%rsp), %rdx
	movq	%rcx, 672(%rsp)
	movq	968(%rsp), %rcx
	movq	%rax, 664(%rsp)
	movq	%rdx, 680(%rsp)
	movq	%rcx, 688(%rsp)
	movq	$-1, 656(%rsp)
.LBB364_277:
	movq	328(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB364_287
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	336(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_280
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_280:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_286
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_280
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
.LBB364_283:
	cmpq	%rax, %rdx
	jge	.LBB364_285
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_283
.LBB364_285:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_286:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB364_287:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_290
	lock		decq	(%rax)
	jne	.LBB364_290
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB364_290:
	vmovdqu64	656(%rsp), %zmm0
	vmovdqu64	688(%rsp), %zmm1
	movq	48(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB364_533
.LBB364_291:
	movq	-8(%rcx), %r15
	testq	%rbx, %rbx
	je	.LBB364_307
	movl	$8, %esi
	movq	%rdi, 240(%rsp)
	movq	%rbx, %rdi
	movl	$8, %ebp
	vzeroupper
	callq	__rustc::__rust_alloc
	movq	240(%rsp), %rdi
	testq	%rax, %rax
	jne	.LBB364_308
	jmp	.LBB364_274
.LBB364_293:
	xorl	%r8d, %r8d
.LBB364_294:
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	leaq	1(%rcx,%rdx), %rdx
	leaq	16(%rcx), %r9
	movq	%r8, 256(%rsp)
	movq	%rsi, 264(%rsp)
	movq	%rdi, 272(%rsp)
	movq	%rcx, 280(%rsp)
	xorl	%r15d, %r15d
	vpcmpgtb	%xmm1, %xmm0, %k0
	movq	%r9, 288(%rsp)
	movq	%rdx, 296(%rsp)
	kmovw	%k0, 304(%rsp)
	movq	%rax, 312(%rsp)
.Ltmp15710:
	leaq	624(%rsp), %rdi
	leaq	256(%rsp), %rsi
	callq	<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))> as alloc::vec::spec_from_iter::SpecFromIter<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)>>>::from_iter
.Ltmp15711:
	movq	632(%rsp), %rbx
	movq	640(%rsp), %r14
.Ltmp15713:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	<[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key::<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>
.Ltmp15714:
	movq	624(%rsp), %rax
	leaq	(%r14,%r14,8), %rcx
	vmovdqu	512(%rsp), %xmm0
	movq	%rbx, 656(%rsp)
	movq	%rbx, 664(%rsp)
	leaq	56(%rsp), %rdx
	leaq	(%rbx,%rcx,8), %rcx
	movq	%rax, 672(%rsp)
	movq	%rcx, 680(%rsp)
	leaq	80(%rsp), %rcx
	movq	%rcx, 688(%rsp)
	movq	2088(%rsp), %rcx
	movq	%r13, 696(%rsp)
	movq	%rdx, 704(%rsp)
	movq	%rcx, 712(%rsp)
	vmovdqu	%xmm0, 720(%rsp)
.Ltmp15718:
	leaq	256(%rsp), %rdi
	leaq	656(%rsp), %rsi
	callq	core::iter::adapters::try_process::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))>, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#4}>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::result::Result<!, purrdf_sparql_eval::error::EvalError>, <core::result::Result<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::error::EvalError> as core::iter::traits::collect::FromIterator<core::result::Result<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::error::EvalError>>>::from_iter<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))>, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#4}>>::{closure#0}, alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.Ltmp15719:
	vmovups	264(%rsp), %xmm0
	movq	280(%rsp), %rcx
	movq	256(%rsp), %rax
	movq	%rcx, 144(%rsp)
	vmovaps	%xmm0, 128(%rsp)
	cmpq	$-1, %rax
	je	.LBB364_299
	vmovdqu64	288(%rsp), %zmm0
	vmovdqa	128(%rsp), %xmm1
	movq	48(%rsp), %rdx
	movq	144(%rsp), %rcx
	movb	$1, %bpl
	xorl	%r15d, %r15d
	vmovdqu64	%zmm0, 48(%rdx)
	movq	%rcx, 40(%rdx)
	vmovdqu	%xmm1, 24(%rdx)
	movq	%rax, 16(%rdx)
	movq	$1, (%rdx)
	jmp	.LBB364_74
.LBB364_299:
	movq	144(%rsp), %rax
	vmovaps	128(%rsp), %xmm0
	movq	%rax, 896(%rsp)
	movzbl	584(%rsp), %eax
	vmovaps	%xmm0, 880(%rsp)
	cmpb	$-1, %al
	je	.LBB364_431
	vmovdqu	(%r12), %xmm0
	movb	%al, 656(%rsp)
	movq	15(%r12), %rax
	movq	80(%rsp), %rcx
	vmovdqu	%xmm0, 657(%rsp)
	movq	%rax, 672(%rsp)
.Ltmp15721:
	movq	112(%rsp), %rsi
	leaq	256(%rsp), %rdi
	leaq	656(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp15722:
	vmovdqu64	256(%rsp), %zmm0
	vmovdqu64	288(%rsp), %zmm1
	movq	48(%rsp), %rax
	leaq	880(%rsp), %rdi
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebp, %ebp
	xorl	%r15d, %r15d
	jmp	.LBB364_74
.LBB364_302:
	cmpq	$-1, %rax
	je	.LBB364_553
	vmovdqa	1344(%rsp), %xmm0
	movq	$1, 256(%rsp)
	vmovdqu	%xmm0, 672(%rsp)
	movq	$0, 656(%rsp)
	movq	%rax, 664(%rsp)
.Ltmp15702:
	leaq	128(%rsp), %rdi
	leaq	192(%rsp), %rsi
	leaq	256(%rsp), %rdx
	leaq	656(%rsp), %rcx
	movb	$1, %r15b
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>), purrdf_hash::fixed::FixedState>>::insert
.Ltmp15703:
	cmpq	$-1, 136(%rsp)
	je	.LBB364_306
.Ltmp15704:
	leaq	136(%rsp), %rdi
	movb	$1, %r15b
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>>
.Ltmp15705:
.LBB364_306:
	movq	216(%rsp), %rax
	jmp	.LBB364_144
.LBB364_307:
	movl	$8, %eax
	xorl	%r12d, %r12d
.LBB364_308:
	movq	608(%rsp), %rcx
	movq	%r13, (%rax)
	movq	64(%rsp), %rdx
	movq	%rcx, 8(%rax)
	vmovaps	128(%rsp), %xmm0
	vmovups	%xmm0, 16(%rax)
	movq	144(%rsp), %rcx
	movq	%rcx, 32(%rax)
	movq	%rdi, 40(%rax)
	movq	%r14, 48(%rax)
	movq	%rdx, 56(%rax)
	movq	%r15, 64(%rax)
	movq	%r12, 192(%rsp)
	movq	%rax, 200(%rsp)
	movq	$1, 208(%rsp)
	vmovdqu64	944(%rsp), %zmm0
	vmovdqu64	%zmm0, 656(%rsp)
	movq	712(%rsp), %rdx
	testq	%rdx, %rdx
	je	.LBB364_320
	movzwl	704(%rsp), %ebp
	movq	680(%rsp), %r15
	movq	688(%rsp), %r12
	leaq	312(%rsp), %r13
	movl	$1, %ebx
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jmp	.LBB364_312
.LBB364_310:
	movq	200(%rsp), %rax
.LBB364_311:
	movq	320(%rsp), %rdx
	leaq	(%rbx,%rbx,8), %rcx
	incq	%rbx
	movq	%rdx, 64(%rax,%rcx,8)
	movq	%r14, %rdx
	vmovdqu64	256(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rax,%rcx,8)
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movq	%rbx, 208(%rsp)
	testq	%r14, %r14
	je	.LBB364_318
.LBB364_312:
	testw	%bp, %bp
	jne	.LBB364_315
	.p2align	4
.LBB364_313:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-1152, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB364_313
	kmovd	%k0, %ebp
.LBB364_315:
	xorl	%ecx, %ecx
	tzcntl	%ebp, %ecx
	leaq	-1(%rdx), %r14
	blsrl	%ebp, %ebp
	negq	%rcx
	leaq	(%rcx,%rcx,8), %rsi
	movq	-24(%r15,%rsi,8), %rcx
	cmpq	$-1, %rcx
	je	.LBB364_319
	leaq	(%r15,%rsi,8), %rsi
	vmovups	-16(%rsi), %xmm0
	movq	-32(%rsi), %rdi
	vmovaps	%xmm0, 528(%rsp)
	vmovdqu	-72(%rsi), %ymm1
	movq	-40(%rsi), %rsi
	movq	%rsi, 288(%rsp)
	vmovdqu	%ymm1, 256(%rsp)
	movq	%rdi, 296(%rsp)
	movq	%rcx, 304(%rsp)
	vmovups	%xmm0, (%r13)
	cmpq	192(%rsp), %rbx
	jne	.LBB364_311
.Ltmp15524:
	movl	$8, %ecx
	movl	$72, %r8d
	leaq	192(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15525:
	jmp	.LBB364_310
.LBB364_318:
	xorl	%r14d, %r14d
.LBB364_319:
	movq	%r12, 688(%rsp)
	movq	%r15, 680(%rsp)
	movw	%bp, 704(%rsp)
	movq	%r14, 712(%rsp)
.LBB364_320:
	leaq	656(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	vmovdqu	192(%rsp), %xmm0
	movq	208(%rsp), %r12
	movq	%r12, 496(%rsp)
	vmovdqa	%xmm0, 480(%rsp)
	movq	488(%rsp), %rbx
	cmpq	$2, %r12
	jae	.LBB364_581
.LBB364_321:
	movq	88(%rsp), %rax
	movq	2088(%rsp), %rcx
	movq	32(%rax), %rax
	cmpb	$2, 472(%rcx)
	movq	%rax, 512(%rsp)
	jne	.LBB364_324
	movq	616(%rcx), %rax
	testq	%rax, %rax
	je	.LBB364_394
	cmpq	$-2, 24(%rax)
	jb	.LBB364_324
	cmpq	$-2, 32(%rax)
	jb	.LBB364_324
	cmpq	$-2, 48(%rax)
	jb	.LBB364_324
.LBB364_394:
	movq	120(%rsp), %r13
	leaq	584(%rcx), %rax
	leaq	256(%rsp), %rbp
	movq	%rax, 64(%rsp)
.LBB364_395:
	movq	56(%r13), %r15
	testq	%r15, %r15
	je	.LBB364_411
	movq	48(%r13), %r14
	shlq	$6, %r15
	jmp	.LBB364_400
.LBB364_399:
	addq	$64, %r14
	addq	$-64, %r15
	je	.LBB364_411
.LBB364_400:
	movq	2088(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB364_403
	movq	64(%rsp), %rax
	movq	%rax, 256(%rsp)
.Ltmp15532:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15533:
	jmp	.LBB364_404
.LBB364_403:
.Ltmp15534:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.12908414067662811932)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15535:
.LBB364_404:
	testb	%al, %al
	jne	.LBB364_324
	movq	2088(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB364_399
	cmpq	$0, 336(%rax)
	jne	.LBB364_409
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB364_409
	cmpq	$-1, 48(%rax)
	je	.LBB364_399
.LBB364_409:
.Ltmp15536:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15537:
	testb	%al, %al
	je	.LBB364_399
	jmp	.LBB364_324
.LBB364_411:
	movq	104(%r13), %rax
	testq	%rax, %rax
	je	.LBB364_425
	movq	96(%r13), %r14
	shlq	$3, %rax
	leaq	(%rax,%rax,8), %r15
	addq	$8, %r14
	jmp	.LBB364_414
.LBB364_413:
	addq	$72, %r14
	addq	$-72, %r15
	je	.LBB364_425
.LBB364_414:
	movq	2088(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB364_417
	movq	64(%rsp), %rax
	movq	%rax, 256(%rsp)
.Ltmp15539:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15540:
	jmp	.LBB364_418
.LBB364_417:
.Ltmp15541:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.12908414067662811932)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15542:
.LBB364_418:
	testb	%al, %al
	jne	.LBB364_324
	movq	2088(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB364_413
	cmpq	$0, 336(%rax)
	jne	.LBB364_423
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB364_423
	cmpq	$-1, 48(%rax)
	je	.LBB364_413
.LBB364_423:
.Ltmp15543:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15544:
	testb	%al, %al
	je	.LBB364_413
	jmp	.LBB364_324
.LBB364_425:
	cmpl	$8, 16(%r13)
	jne	.LBB364_430
	movq	2088(%rsp), %rax
	movq	24(%r13), %rsi
	movq	32(%r13), %rdx
	movq	688(%rax), %rdi
	addq	$16, %rsi
.Ltmp15546:
	movq	<purrdf_sparql_eval::agg_fn::AggregateRegistry>::resolve@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15547:
	testq	%rax, %rax
	je	.LBB364_324
	movq	(%rax), %rcx
	movq	8(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp15548:
	callq	*%rax
.Ltmp15549:
	testb	%al, %al
	jne	.LBB364_324
.LBB364_430:
	addq	$120, %r13
	movb	$1, %r14b
	cmpq	1256(%rsp), %r13
	jne	.LBB364_395
	jmp	.LBB364_325
.LBB364_324:
	xorl	%r14d, %r14d
.LBB364_325:
	movq	56(%rsp), %r8
	movb	$1, %bpl
	addq	$16, %r8
.Ltmp15551:
	movq	112(%rsp), %rsi
	movq	120(%rsp), %rdx
	movq	2064(%rsp), %rcx
	movq	2088(%rsp), %r9
	leaq	584(%rsp), %r15
	movq	%r15, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::modifier::link_aggregates::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15552:
	testb	%r14b, %r14b
	je	.LBB364_332
	movq	2088(%rsp), %r13
	movq	1040(%r13), %rax
	movq	616(%r13), %rsi
	addq	904(%r13), %rax
	movq	%rax, 1280(%rsp)
.Ltmp15574:
	leaq	944(%rsp), %r14
	movq	%r14, %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::for_items::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15575:
	movq	120(%rsp), %rbp
.Ltmp15576:
	movq	%r13, %rdi
	movq	%r12, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp15577:
	movq	616(%r13), %rcx
	movq	%rax, 560(%rsp)
	testq	%rcx, %rcx
	je	.LBB364_383
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB364_384
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB364_384
.LBB364_332:
	movq	120(%rsp), %r14
	testq	%r12, %r12
	je	.LBB364_381
	leaq	(,%r12,8), %rax
	movl	$8, %esi
	leaq	(%rax,%rax,4), %r15
	movq	%r15, %rdi
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB364_598
	addq	$16, %r14
	movq	%r12, 128(%rsp)
	movq	%rax, 136(%rsp)
	movq	%rax, %rbp
	leaq	(%r12,%r12,8), %rax
	movq	$0, 144(%rsp)
	movq	$0, 64(%rsp)
	movq	%r14, 120(%rsp)
	movq	2064(%rsp), %r14
	leaq	(%rbx,%rax,8), %rax
	movq	%rax, 240(%rsp)
	jmp	.LBB364_337
.LBB364_335:
	movq	136(%rsp), %rbp
.LBB364_336:
	movq	64(%rsp), %rdx
	addq	$72, %rbx
	leaq	(%rdx,%rdx,4), %rax
	incq	%rdx
	movq	%rdx, 64(%rsp)
	movq	%r15, (%rbp,%rax,8)
	movq	%r13, 8(%rbp,%rax,8)
	vmovdqa	256(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rbp,%rax,8)
	movq	272(%rsp), %rcx
	movq	%rcx, 32(%rbp,%rax,8)
	movq	%rdx, 144(%rsp)
	cmpq	240(%rsp), %rbx
	je	.LBB364_382
.LBB364_337:
	movq	512(%rsp), %r13
	movq	$1, 256(%rsp)
	cmpq	$5, %r13
	jae	.LBB364_362
	vmovdqu	264(%rsp), %xmm0
	movq	288(%rsp), %rax
	movq	256(%rsp), %rdx
	movq	280(%rsp), %rcx
	movq	%rax, 688(%rsp)
	movq	%rdx, 656(%rsp)
	movq	%rcx, 680(%rsp)
	vmovdqu	%xmm0, 664(%rsp)
	testq	%r13, %r13
	je	.LBB364_340
.LBB364_339:
	movl	$2, %eax
	jmp	.LBB364_341
.LBB364_340:
	movl	$-1, %eax
.LBB364_341:
	movl	%eax, 256(%rsp)
	movq	%r13, 264(%rsp)
.Ltmp15556:
	leaq	656(%rsp), %rdi
	leaq	256(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp15557:
	vmovdqu	656(%rsp), %ymm0
	movq	688(%rsp), %rax
	leaq	952(%rsp), %rdi
	movq	%rax, 976(%rsp)
	vmovdqu	%ymm0, 944(%rsp)
	movq	944(%rsp), %r15
	movq	%r15, %rax
	cmpq	$6, %r15
	jb	.LBB364_344
	movq	952(%rsp), %rdi
	movq	960(%rsp), %rax
.LBB364_344:
	movq	248(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB364_583
	movq	(%rbx), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB364_347
	movq	16(%rbx), %rsi
	movq	8(%rbx), %rax
	decq	%rsi
	jmp	.LBB364_348
.LBB364_347:
	leaq	8(%rbx), %rax
.LBB364_348:
	movq	%rbp, 608(%rsp)
	cmpq	%rsi, %rdx
	jne	.LBB364_584
	movq	memcpy@GOTPCREL(%rip), %r13
	shlq	$3, %rdx
	movq	%rax, %rsi
	vzeroupper
	callq	*%r13
	movq	600(%rsp), %rax
	cmpq	%rax, %r14
	movq	%rax, %r12
	cmovbq	%r14, %r12
	testq	%rax, %rax
	je	.LBB364_360
	movq	592(%rsp), %rbp
	movq	120(%rsp), %r13
	movq	2088(%rsp), %rsi
	xorl	%r14d, %r14d
	addq	$16, %rbp
	jmp	.LBB364_352
	.p2align	4
.LBB364_351:
	movq	2088(%rsp), %rsi
	incq	%r14
	addq	$24, %rbp
	addq	$120, %r13
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%r14, %r12
	je	.LBB364_359
.LBB364_352:
	vmovups	632(%rsp), %xmm0
	vmovdqu	1152(%rsp), %xmm1
	movq	56(%rsp), %rax
	movq	-8(%rbp), %rdx
	movq	(%rbp), %rcx
	movq	56(%rbx), %r8
	movq	64(%rbx), %r9
	addq	$16, %rax
.Ltmp15561:
	leaq	256(%rsp), %rdi
	movq	%rsi, 40(%rsp)
	vmovdqu	%xmm1, 24(%rsp)
	movq	%rax, 16(%rsp)
	movq	%r13, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>
.Ltmp15562:
	vmovq	264(%rsp), %xmm0
	movq	256(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB364_364
	movq	944(%rsp), %r15
	movq	%r15, %rsi
	cmpq	$6, %r15
	jb	.LBB364_356
	movq	960(%rsp), %rsi
.LBB364_356:
	movq	248(%rsp), %rdi
	decq	%rsi
	addq	%r14, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB364_593
	leaq	952(%rsp), %rax
	cmpq	$6, %r15
	jb	.LBB364_351
	movq	952(%rsp), %rax
	jmp	.LBB364_351
.LBB364_359:
	movq	944(%rsp), %r15
	movq	2064(%rsp), %r14
.LBB364_360:
	leaq	952(%rsp), %rax
	movq	952(%rsp), %r13
	movq	608(%rsp), %rbp
	movq	64(%rsp), %rcx
	vmovups	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 272(%rsp)
	vmovaps	%xmm0, 256(%rsp)
	cmpq	128(%rsp), %rcx
	jne	.LBB364_336
.Ltmp15566:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	128(%rsp), %rdi
	callq	*%rax
.Ltmp15567:
	jmp	.LBB364_335
.LBB364_362:
.Ltmp15553:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	xorl	%esi, %esi
	movq	%r13, %rdx
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp15554:
	vmovdqu	256(%rsp), %ymm0
	movq	288(%rsp), %rax
	movq	2064(%rsp), %r14
	movq	%rax, 688(%rsp)
	vmovdqu	%ymm0, 656(%rsp)
	jmp	.LBB364_339
.LBB364_364:
	vmovdqu64	272(%rsp), %zmm1
	vmovups	288(%rsp), %zmm2
	movq	48(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovdqu64	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	944(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB364_366
	movq	952(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB364_366:
	movq	64(%rsp), %rbp
	movq	608(%rsp), %r14
	testq	%rbp, %rbp
	je	.LBB364_379
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %r13
	xorl	%ebx, %ebx
	jmp	.LBB364_371
.LBB364_368:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_369:
	vzeroupper
	callq	*%r13
.LBB364_370:
	incq	%rbx
	cmpq	%rbp, %rbx
	je	.LBB364_379
.LBB364_371:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB364_370
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_374
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_374:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_369
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_374
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r15)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB364_377:
	cmpq	%rax, %rdx
	jge	.LBB364_368
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB364_377
	jmp	.LBB364_368
.LBB364_379:
	movq	128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_447
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
	jmp	.LBB364_447
.LBB364_381:
	movq	$0, 128(%rsp)
	movq	$8, 136(%rsp)
	movq	$0, 144(%rsp)
.LBB364_382:
	movq	144(%rsp), %rax
	movq	128(%rsp), %rdx
	movq	136(%rsp), %rcx
	movq	2088(%rsp), %rbx
	movq	%rax, 544(%rsp)
	movq	%rdx, 528(%rsp)
	movq	%rcx, 536(%rsp)
	jmp	.LBB364_542
.LBB364_383:
	xorl	%eax, %eax
.LBB364_384:
	movq	2088(%rsp), %rdx
	leaq	512(%rsp), %rdi
	leaq	560(%rsp), %r8
	movq	%r15, 192(%rsp)
	movq	%rdi, 128(%rsp)
	movq	2064(%rsp), %rdi
	movzbl	1234(%rdx), %ecx
	movq	%rdx, 200(%rsp)
	movq	%r8, 208(%rsp)
	leaq	248(%rsp), %r8
	movq	%rdx, %r15
	movq	%r14, 216(%rsp)
	movq	%r8, 136(%rsp)
	movq	%rbp, 144(%rsp)
	movq	%rdi, 152(%rsp)
	leaq	624(%rsp), %r8
	leaq	56(%rsp), %rdi
	movq	%r8, 160(%rsp)
	movq	%rdi, 168(%rsp)
	leaq	1152(%rsp), %r8
	leaq	1280(%rsp), %rdi
	movq	%r8, 176(%rsp)
	movq	%rdi, 184(%rsp)
	movzbl	%cl, %esi
	testb	%al, %al
	je	.LBB364_386
.Ltmp15580:
	leaq	256(%rsp), %rdi
	leaq	192(%rsp), %r8
	leaq	128(%rsp), %r9
	movq	%r14, (%rsp)
	movq	%rbx, %rdx
	movq	%r12, %rcx
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#8}>
.Ltmp15581:
	jmp	.LBB364_387
.LBB364_386:
.Ltmp15578:
	leaq	256(%rsp), %rdi
	leaq	192(%rsp), %r8
	leaq	128(%rsp), %r9
	movq	%r14, (%rsp)
	movq	%rbx, %rdx
	movq	%r12, %rcx
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#8}>
.Ltmp15579:
.LBB364_387:
	movq	256(%rsp), %rax
	cmpq	$-1, %rax
	je	.LBB364_391
	vmovups	416(%rsp), %zmm0
	vmovups	368(%rsp), %zmm2
	movq	280(%rsp), %rcx
	vmovdqu	264(%rsp), %xmm1
	movq	296(%rsp), %rsi
	movq	288(%rsp), %rdx
	movq	272(%rsp), %rbx
	movq	%rax, 1320(%rsp)
	movl	$1, %edi
	leaq	-3(%rcx), %rax
	cmpq	$-2, %rax
	movq	%rcx, %rax
	cmovbq	%rsi, %rax
	cmovbq	%rdi, %rsi
	cmovbq	%rcx, %rdi
	vmovups	%zmm0, 768(%rsp)
	vmovups	%zmm2, 720(%rsp)
	vmovdqu64	304(%rsp), %zmm0
	decq	%rax
	vmovdqu	%xmm1, 1328(%rsp)
	vmovups	768(%rsp), %zmm3
	vmovups	720(%rsp), %zmm2
	vmovdqu64	%zmm0, 656(%rsp)
	vmovdqu64	%zmm0, 280(%rsp)
	vmovups	%zmm3, 392(%rsp)
	vmovups	%zmm2, 344(%rsp)
	movq	%rdi, 256(%rsp)
	movq	%rdx, 264(%rsp)
	movq	%rsi, 272(%rsp)
	movq	$0, 456(%rsp)
	movq	%rax, 464(%rsp)
.Ltmp15583:
	leaq	656(%rsp), %rdi
	leaq	256(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15584:
	vmovups	688(%rsp), %zmm1
	vmovdqu	656(%rsp), %ymm0
	vmovups	752(%rsp), %zmm2
	cmpq	$0, 616(%r15)
	vmovups	%zmm1, 1824(%rsp)
	vmovdqu64	800(%rsp), %zmm1
	vmovups	%zmm2, 1888(%rsp)
	vmovdqu	%ymm0, 128(%rsp)
	vmovdqu64	%zmm1, 1936(%rsp)
	je	.LBB364_397
	movl	1104(%rsp), %esi
.Ltmp15588:
	leaq	256(%rsp), %rdi
	leaq	1320(%rsp), %rcx
	leaq	1824(%rsp), %r8
	movq	%r15, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#9}>
.Ltmp15589:
	jmp	.LBB364_441
.LBB364_391:
	vmovdqu64	304(%rsp), %zmm0
	vmovdqu	272(%rsp), %ymm1
	movq	48(%rsp), %rax
	vmovdqu64	%zmm0, 48(%rax)
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu64	%zmm0, 656(%rsp)
	movq	$1, (%rax)
	movq	560(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB364_444
	jmp	.LBB364_446
.LBB364_397:
	leaq	632(%rsp), %rcx
	movq	624(%rsp), %rax
	vmovups	(%rcx), %xmm0
	movq	$0, 624(%rsp)
	movq	$8, 632(%rsp)
	movq	$0, 640(%rsp)
	vmovaps	%xmm0, 192(%rsp)
	cmpq	%rbx, %rax
	jbe	.LBB364_439
	vmovdqa	192(%rsp), %xmm0
	leaq	256(%rsp), %rdi
	movq	%rax, 256(%rsp)
	vmovdqu	%xmm0, 264(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	$8, 1304(%rsp)
	movq	$0, 1312(%rsp)
	xorl	%eax, %eax
	jmp	.LBB364_440
.LBB364_431:
	vmovdqa	880(%rsp), %xmm0
	movq	80(%rsp), %rax
	movq	896(%rsp), %rcx
	movq	%rcx, 672(%rsp)
	vmovdqa	%xmm0, 656(%rsp)
	movq	%rax, 680(%rsp)
.Ltmp15729:
	leaq	256(%rsp), %rdi
	leaq	1496(%rsp), %rsi
	leaq	656(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
.Ltmp15730:
	vmovdqu64	256(%rsp), %zmm0
	vmovdqu64	288(%rsp), %zmm1
	movq	48(%rsp), %rax
	movq	528(%rsp), %rsi
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	testq	%rsi, %rsi
	jle	.LBB364_434
	movq	536(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB364_434:
	xorl	%ebp, %ebp
.Ltmp15732:
	leaq	480(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp15733:
	movq	560(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB364_437
	movq	568(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB364_437:
	movq	104(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_533
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp15735:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	104(%rsp), %rdi
	callq	*%rax
.Ltmp15736:
	jmp	.LBB364_533
.LBB364_439:
	vmovdqa	192(%rsp), %xmm0
	vmovdqu	%xmm0, 1304(%rsp)
.LBB364_440:
	movl	1104(%rsp), %esi
	movq	%rax, 1296(%rsp)
.Ltmp15591:
	leaq	256(%rsp), %rdi
	leaq	1320(%rsp), %rcx
	leaq	1824(%rsp), %r8
	leaq	1296(%rsp), %r9
	movq	%r15, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#9}>
.Ltmp15592:
.LBB364_441:
	vmovups	264(%rsp), %xmm0
	movq	280(%rsp), %rcx
	movq	256(%rsp), %rax
	movq	296(%rsp), %rbx
	movq	288(%rsp), %r14
	movq	%rcx, 672(%rsp)
	vmovaps	%xmm0, 656(%rsp)
	cmpq	$-1, %rax
	je	.LBB364_534
	vmovups	320(%rsp), %ymm1
	movq	48(%rsp), %rdx
	vmovdqu	304(%rsp), %ymm0
	movq	672(%rsp), %rcx
	vmovups	%ymm1, 80(%rdx)
	vmovdqa	656(%rsp), %xmm1
	vmovdqu	%ymm0, 64(%rdx)
	movq	%rcx, 40(%rdx)
	vmovdqu	%xmm1, 24(%rdx)
	movq	%rax, 16(%rdx)
	movq	%r14, 48(%rdx)
	movq	%rbx, 56(%rdx)
	movq	$1, (%rdx)
.Ltmp15596:
	leaq	128(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp15597:
	movq	560(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_446
.LBB364_444:
	lock		decq	(%rax)
	jne	.LBB364_446
	#MEMBARRIER
.Ltmp15639:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	560(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15640:
.LBB364_446:
.Ltmp15644:
	leaq	944(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp15645:
.LBB364_447:
	movb	$1, %bl
	movq	592(%rsp), %r12
	movq	600(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_451
.LBB364_448:
	movl	$1, %r14d
	movq	%r12, %r15
	subq	%rax, %r14
	.p2align	4
.LBB364_449:
.Ltmp15649:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp15650:
	incq	%r14
	addq	$24, %r15
	cmpq	$1, %r14
	jne	.LBB364_449
.LBB364_451:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_461
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_454
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_454:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_460
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_454
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_457:
	cmpq	%rax, %rdx
	jge	.LBB364_459
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_457
.LBB364_459:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_460:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.LBB364_461:
	movl	%ebx, 64(%rsp)
	movq	488(%rsp), %rbx
	movq	496(%rsp), %r15
	testq	%r15, %r15
	je	.LBB364_484
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r14
	xorl	%r12d, %r12d
	jmp	.LBB364_466
	.p2align	4
.LBB364_463:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_464:
	vzeroupper
	callq	*%r14
.LBB364_465:
	incq	%r12
	cmpq	%r15, %r12
	je	.LBB364_484
.LBB364_466:
	leaq	(%r12,%r12,8), %rax
	leaq	(%rbx,%rax,8), %r13
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB364_476
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	8(%r13), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_469
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_469:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_475
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_469
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB364_472:
	cmpq	%rax, %rdx
	jge	.LBB364_474
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB364_472
.LBB364_474:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_475:
	vzeroupper
	callq	*%r14
.LBB364_476:
	movq	48(%r13), %rcx
	testq	%rcx, %rcx
	je	.LBB364_465
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	56(%r13), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_479
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_479:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_464
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_479
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB364_482:
	cmpq	%rax, %rdx
	jge	.LBB364_463
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB364_482
	jmp	.LBB364_463
.LBB364_484:
	movq	480(%rsp), %rax
	movl	64(%rsp), %r12d
	testq	%rax, %rax
	je	.LBB364_494
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_487
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_487:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_493
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_487
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_490:
	cmpq	%rax, %rdx
	jge	.LBB364_492
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_490
.LBB364_492:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_493:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB364_494:
	cmpq	$0, 232(%rsp)
	je	.LBB364_504
	movq	224(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	cmpq	%rdi, %rax
	movq	%rdi, %rsi
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_497
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_497:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_503
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_497
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB364_500:
	cmpq	%rax, %rcx
	jge	.LBB364_502
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB364_500
.LBB364_502:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_503:
	movq	free@GOTPCREL(%rip), %rax
	movq	96(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB364_504:
	movq	56(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_506
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp15655:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15656:
.LBB364_506:
	movb	$1, %bpl
.Ltmp15658:
	leaq	624(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15659:
	movq	1440(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB364_517
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1448(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_510
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_510:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_516
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_510
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
.LBB364_513:
	cmpq	%rax, %rdx
	jge	.LBB364_515
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_513
.LBB364_515:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_516:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB364_517:
	movq	1368(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB364_527
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1376(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB364_520
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB364_520:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB364_526
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB364_520
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB364_523:
	cmpq	%rax, %rdx
	jge	.LBB364_525
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB364_523
.LBB364_525:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB364_526:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB364_527:
	movq	1464(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_530
	lock		decq	(%rax)
	jne	.LBB364_530
	leaq	1464(%rsp), %rdi
	#MEMBARRIER
.Ltmp15661:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15662:
.LBB364_530:
	testb	%r12b, %r12b
	je	.LBB364_533
.LBB364_531:
	movq	88(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_533
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	936(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB364_533:
	movq	48(%rsp), %rax
	addq	$2008, %rsp
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
	vzeroupper
	retq
.LBB364_534:
	.cfi_def_cfa_offset 2064
	vmovaps	656(%rsp), %xmm0
	movq	672(%rsp), %rax
	vmovdqu	128(%rsp), %ymm1
	movq	%rax, 1248(%rsp)
	movq	1248(%rsp), %rax
	vmovaps	%xmm0, 1232(%rsp)
	vmovdqu	%ymm1, 256(%rsp)
	vmovdqa	1232(%rsp), %xmm0
	movq	%rax, 672(%rsp)
	vmovdqa	%xmm0, 656(%rsp)
.Ltmp15599:
	leaq	256(%rsp), %rsi
	movq	%r15, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::absorb_worker_witnesses::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp15600:
	cmpq	$1, %r14
	jne	.LBB364_538
	movq	496(%rsp), %rdx
	cmpq	%rdx, %rbx
	ja	.LBB364_589
	jne	.LBB364_555
.LBB364_538:
	movq	672(%rsp), %rax
	vmovdqa	656(%rsp), %xmm0
	movq	2088(%rsp), %rbx
	movq	%rax, 544(%rsp)
	movq	560(%rsp), %rax
	vmovdqa	%xmm0, 528(%rsp)
	testq	%rax, %rax
	je	.LBB364_541
	lock		decq	(%rax)
	jne	.LBB364_541
	#MEMBARRIER
.Ltmp15617:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	560(%rsp), %rdi
	callq	*%rax
.Ltmp15618:
.LBB364_541:
.Ltmp15619:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp15620:
.LBB364_542:
	movq	696(%rbx), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	je	.LBB364_550
.LBB364_543:
	vmovdqa	528(%rsp), %xmm0
	movq	544(%rsp), %rax
	movq	88(%rsp), %rcx
	movq	%rax, 672(%rsp)
	vmovdqa	%xmm0, 656(%rsp)
	movq	%rcx, 680(%rsp)
.Ltmp15624:
	leaq	256(%rsp), %rdi
	leaq	1368(%rsp), %rsi
	leaq	656(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
.Ltmp15625:
	vmovdqu64	256(%rsp), %zmm0
	vmovdqu64	288(%rsp), %zmm1
	movq	48(%rsp), %rax
	xorl	%ebp, %ebp
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp15627:
	leaq	584(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp15628:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	cmpq	$0, 232(%rsp)
	je	.LBB364_547
	movq	96(%rsp), %rdi
	movq	224(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB364_547:
	movq	56(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_549
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp15630:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	xorl	%r12d, %r12d
	callq	*%rax
.Ltmp15631:
.LBB364_549:
	xorl	%ebp, %ebp
.Ltmp15632:
	leaq	624(%rsp), %rdi
	xorl	%r12d, %r12d
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15633:
	jmp	.LBB364_533
.LBB364_550:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB364_543
	movb	%cl, 656(%rsp)
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 657(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 672(%rsp)
.Ltmp15621:
	movq	112(%rsp), %rsi
	movq	88(%rsp), %rcx
	leaq	256(%rsp), %rdi
	leaq	656(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp15622:
	vmovdqu64	256(%rsp), %zmm0
	vmovdqu64	288(%rsp), %zmm1
	movq	48(%rsp), %rax
	leaq	528(%rsp), %rdi
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
	movq	592(%rsp), %r12
	movq	600(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB364_448
	jmp	.LBB364_451
.LBB364_553:
	movzbl	584(%rsp), %eax
	cmpb	$-1, %al
	je	.LBB364_590
	vmovdqu	(%r12), %xmm0
	movb	%al, 656(%rsp)
	movq	15(%r12), %rax
	movq	80(%rsp), %rcx
	xorl	%ebp, %ebp
	vmovdqu	%xmm0, 657(%rsp)
	movq	%rax, 672(%rsp)
.Ltmp15706:
	movq	112(%rsp), %rsi
	leaq	256(%rsp), %rdi
	leaq	656(%rsp), %rdx
	movb	$1, %r15b
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp15707:
	jmp	.LBB364_73
.LBB364_555:
	movq	488(%rsp), %rax
	leaq	(%rdx,%rdx,8), %rcx
	addq	$16, 120(%rsp)
	leaq	(%rax,%rcx,8), %rcx
	movq	%rcx, 64(%rsp)
	leaq	(%rbx,%rbx,8), %rcx
	leaq	1112(%rsp), %rbx
	leaq	(%rax,%rcx,8), %rbp
.LBB364_556:
	movq	512(%rsp), %rsi
.Ltmp15601:
	movq	%rbx, %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
.Ltmp15602:
	movq	1112(%rsp), %r13
	movq	2064(%rsp), %r14
	leaq	1120(%rsp), %rdi
	movq	%r13, %rax
	cmpq	$6, %r13
	jb	.LBB364_559
	movq	1120(%rsp), %rdi
	movq	1128(%rsp), %rax
.LBB364_559:
	movq	248(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB364_587
	movq	(%rbp), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB364_562
	movq	16(%rbp), %rsi
	movq	8(%rbp), %rax
	decq	%rsi
	jmp	.LBB364_563
.LBB364_562:
	leaq	8(%rbp), %rax
.LBB364_563:
	cmpq	%rsi, %rdx
	jne	.LBB364_588
	movq	memcpy@GOTPCREL(%rip), %r15
	shlq	$3, %rdx
	movq	%rax, %rsi
	callq	*%r15
	movq	600(%rsp), %rax
	cmpq	%rax, %r14
	movq	%rax, %rbx
	cmovbq	%r14, %rbx
	testq	%rax, %rax
	je	.LBB364_574
	movq	592(%rsp), %r15
	movq	120(%rsp), %r12
	xorl	%r14d, %r14d
	addq	$16, %r15
	jmp	.LBB364_567
.LBB364_566:
	incq	%r14
	addq	$24, %r15
	addq	$120, %r12
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%r14, %rbx
	je	.LBB364_574
.LBB364_567:
	vmovups	632(%rsp), %xmm0
	vmovdqu	1152(%rsp), %xmm1
	movq	56(%rsp), %rax
	movq	-8(%r15), %rdx
	movq	(%r15), %rcx
	movq	56(%rbp), %r8
	movq	64(%rbp), %r9
	addq	$16, %rax
.Ltmp15606:
	movq	2088(%rsp), %rsi
	leaq	256(%rsp), %rdi
	movq	%rsi, 40(%rsp)
	vmovdqu	%xmm1, 24(%rsp)
	movq	%rax, 16(%rsp)
	movq	%r12, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>
.Ltmp15607:
	vmovq	264(%rsp), %xmm0
	movq	256(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB364_576
	movq	1112(%rsp), %r13
	movq	%r13, %rsi
	cmpq	$6, %r13
	jb	.LBB364_571
	movq	1128(%rsp), %rsi
.LBB364_571:
	movq	248(%rsp), %rdi
	decq	%rsi
	addq	%r14, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB364_596
	leaq	1120(%rsp), %rax
	cmpq	$6, %r13
	jb	.LBB364_566
	movq	1120(%rsp), %rax
	jmp	.LBB364_566
.LBB364_574:
.Ltmp15611:
	leaq	1112(%rsp), %rbx
	leaq	656(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::push_mut
.Ltmp15612:
	addq	$72, %rbp
	cmpq	64(%rsp), %rbp
	jne	.LBB364_556
	jmp	.LBB364_538
.LBB364_576:
	vmovups	272(%rsp), %zmm1
	vmovups	288(%rsp), %zmm2
	movq	48(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1112(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB364_578
	movq	1120(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB364_578:
	leaq	656(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	560(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB364_444
	jmp	.LBB364_446
.LBB364_579:
.Ltmp15762:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.545(%rip), %rcx
	xorl	%edi, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp15763:
	jmp	.LBB364_599
.LBB364_580:
.Ltmp15663:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.536(%rip), %rcx
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
.Ltmp15664:
	jmp	.LBB364_599
.LBB364_581:
	cmpq	$21, %r12
	jae	.LBB364_600
	movq	%rbx, %rdi
	movq	%r12, %rsi
	callq	core::slice::sort::shared::smallsort::insertion_sort_shift_left::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>
	jmp	.LBB364_321
.LBB364_583:
.Ltmp15569:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.529(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	vzeroupper
	callq	*%r8
.Ltmp15570:
	jmp	.LBB364_599
.LBB364_584:
.Ltmp15559:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.530(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	vzeroupper
	callq	*%rcx
.Ltmp15560:
	jmp	.LBB364_599
.LBB364_585:
.Ltmp15507:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15508:
	jmp	.LBB364_599
.LBB364_586:
.Ltmp15504:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15505:
	jmp	.LBB364_599
.LBB364_587:
.Ltmp15614:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.532(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	callq	*%r8
.Ltmp15615:
	jmp	.LBB364_599
.LBB364_588:
.Ltmp15604:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.533(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	callq	*%rcx
.Ltmp15605:
	jmp	.LBB364_599
.LBB364_589:
.Ltmp15634:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.535(%rip), %rcx
	movq	%rbx, %rdi
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp15635:
	jmp	.LBB364_599
.LBB364_590:
.Ltmp15708:
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.543(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.544(%rip), %rdx
	movl	$49, %esi
	movb	$1, %r15b
	callq	*%rax
.Ltmp15709:
	jmp	.LBB364_599
.LBB364_591:
.Ltmp15515:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$8, %esi
	callq	*%rax
.Ltmp15516:
	jmp	.LBB364_599
.LBB364_592:
.Ltmp15770:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15771:
	jmp	.LBB364_599
.LBB364_593:
.Ltmp15564:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.531(%rip), %rdx
	callq	*%rax
.Ltmp15565:
	jmp	.LBB364_599
.LBB364_594:
.Ltmp15485:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$1, %edi
	movl	$51, %esi
	callq	*%rax
.Ltmp15486:
	jmp	.LBB364_599
.LBB364_595:
.Ltmp15679:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15680:
	jmp	.LBB364_599
.LBB364_596:
.Ltmp15609:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.534(%rip), %rdx
	callq	*%rax
.Ltmp15610:
	jmp	.LBB364_599
.LBB364_597:
.Ltmp15499:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15500:
	jmp	.LBB364_599
.LBB364_598:
.Ltmp15572:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15573:
.LBB364_599:
	ud2
.LBB364_600:
	movb	$1, %bpl
.Ltmp15527:
	movq	core::slice::sort::unstable::ipnsort::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp15528:
	jmp	.LBB364_321
.LBB364_601:
.Ltmp15555:
	movq	256(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$6, %r15
	jae	.LBB364_630
	jmp	.LBB364_705
.LBB364_602:
.Ltmp15503:
	jmp	.LBB364_692
.LBB364_603:
.Ltmp15613:
	jmp	.LBB364_664
.LBB364_604:
.Ltmp15731:
	movq	%rax, %rbx
	xorl	%r15d, %r15d
	xorl	%ebp, %ebp
	jmp	.LBB364_667
.LBB364_605:
.Ltmp15623:
	leaq	528(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bpl
	jmp	.LBB364_615
.LBB364_606:
.Ltmp15603:
	jmp	.LBB364_664
.LBB364_607:
.Ltmp15723:
	leaq	880(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebp, %ebp
	xorl	%r15d, %r15d
	jmp	.LBB364_666
.LBB364_608:
.Ltmp15593:
	jmp	.LBB364_613
.LBB364_609:
.Ltmp15550:
	jmp	.LBB364_636
.LBB364_610:
.Ltmp15512:
	jmp	.LBB364_692
.LBB364_611:
.Ltmp15598:
	jmp	.LBB364_624
.LBB364_612:
.Ltmp15590:
.LBB364_613:
	movq	%rax, %rbx
.Ltmp15594:
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp15595:
	jmp	.LBB364_676
.LBB364_614:
.Ltmp15626:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
.LBB364_615:
	xorl	%r12d, %r12d
	jmp	.LBB364_707
.LBB364_616:
.Ltmp15585:
	movq	%rax, %rbx
.Ltmp15586:
	leaq	1320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15587:
	jmp	.LBB364_676
.LBB364_617:
.Ltmp15568:
	movq	%rax, %rbx
	cmpq	$5, %r15
	ja	.LBB364_704
	jmp	.LBB364_705
.LBB364_618:
.Ltmp15523:
	jmp	.LBB364_662
.LBB364_619:
.Ltmp15720:
	movq	%rax, %rbx
	jmp	.LBB364_621
.LBB364_620:
.Ltmp15715:
	movq	%rax, %rbx
.Ltmp15716:
	leaq	624(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))>>
.Ltmp15717:
.LBB364_621:
	movb	$1, %bpl
	xorl	%r15d, %r15d
	jmp	.LBB364_666
.LBB364_622:
.Ltmp15657:
	movq	%rax, %rbx
	jmp	.LBB364_729
.LBB364_623:
.Ltmp15582:
.LBB364_624:
	movq	%rax, %rbx
	jmp	.LBB364_676
.LBB364_625:
.Ltmp15739:
	movq	%rax, %rbx
	movb	$1, %r15b
	xorl	%ebp, %ebp
	jmp	.LBB364_667
.LBB364_626:
.Ltmp15641:
	movq	%rax, %rbx
	jmp	.LBB364_679
.LBB364_627:
.Ltmp15728:
	movq	%rax, %rbx
	jmp	.LBB364_667
.LBB364_628:
.Ltmp15608:
	movq	1112(%rsp), %r13
	jmp	.LBB364_673
.LBB364_629:
.Ltmp15558:
	movq	656(%rsp), %r15
	movq	%rax, %rbx
	leaq	664(%rsp), %rax
	movq	%rax, 920(%rsp)
	cmpq	$5, %r15
	jbe	.LBB364_705
.LBB364_630:
	movq	920(%rsp), %rax
	movq	(%rax), %r13
	jmp	.LBB364_704
.LBB364_631:
.Ltmp15526:
	leaq	256(%rsp), %rdi
	movq	%r12, 688(%rsp)
	movq	%r15, 680(%rsp)
	movw	%bp, 704(%rsp)
	movq	%rax, %rbx
	movq	%r14, 712(%rsp)
	callq	core::ptr::drop_glue::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>
	leaq	656(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	leaq	192(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB364_724
.LBB364_632:
.Ltmp15629:
	movl	%ebp, %r12d
	movq	%rax, %rbx
	jmp	.LBB364_708
.LBB364_633:
.Ltmp15646:
	movq	%rax, %rbx
	jmp	.LBB364_706
.LBB364_634:
.Ltmp15545:
	jmp	.LBB364_636
.LBB364_635:
.Ltmp15538:
.LBB364_636:
	movq	%rax, %rbx
	movb	$1, %bpl
	movb	$1, %r12b
	jmp	.LBB364_708
.LBB364_637:
.Ltmp15691:
	movq	%rax, %rbx
.Ltmp15692:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>>
.Ltmp15693:
	jmp	.LBB364_643
.LBB364_638:
.Ltmp15487:
	jmp	.LBB364_698
.LBB364_639:
.Ltmp15744:
	movq	%rax, %rbx
	jmp	.LBB364_671
.LBB364_640:
.Ltmp15758:
	movq	%rax, %rbx
	jmp	.LBB364_732
.LBB364_641:
.Ltmp15660:
	movq	%rax, %rbx
	jmp	.LBB364_730
.LBB364_642:
.Ltmp15688:
	movq	%rax, %rbx
.LBB364_643:
	movb	$1, %r15b
	movb	$1, %bpl
	jmp	.LBB364_667
.LBB364_644:
.Ltmp15685:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB364_670
.LBB364_645:
.Ltmp15734:
	movq	%rax, %rbx
	jmp	.LBB364_688
.LBB364_646:
.Ltmp15563:
	movq	944(%rsp), %r15
	jmp	.LBB364_702
.LBB364_647:
.Ltmp15772:
	movq	%rax, %rbx
.Ltmp15773:
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15774:
	jmp	.LBB364_735
.LBB364_648:
.Ltmp15775:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB364_649:
.Ltmp15755:
	movq	%rax, %rbx
	jmp	.LBB364_715
.LBB364_650:
.Ltmp15651:
	movl	%ebx, %r13d
	movq	%rax, %rbx
	testq	%r14, %r14
	je	.LBB364_654
	negq	%r14
	addq	$24, %r15
.LBB364_652:
.Ltmp15652:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp15653:
	addq	$24, %r15
	decq	%r14
	jne	.LBB364_652
.LBB364_654:
	movq	584(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	jne	.LBB364_656
	movl	%r13d, %r12d
	jmp	.LBB364_708
.LBB364_656:
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r12, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	movl	%r13d, %r12d
	jmp	.LBB364_708
.LBB364_657:
.Ltmp15654:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB364_658:
.Ltmp15517:
	cmpq	$6, 64(%rsp)
	movq	%rax, %rbx
	jb	.LBB364_696
	movq	64(%rsp), %rax
	movq	928(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	jmp	.LBB364_695
.LBB364_660:
.Ltmp15498:
	movq	96(%rsp), %rdi
	movq	224(%rsp), %rsi
	movl	$8, %edx
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
	jmp	.LBB364_710
.LBB364_661:
.Ltmp15520:
.LBB364_662:
	movq	%rax, %rbx
	jmp	.LBB364_696
.LBB364_663:
.Ltmp15636:
.LBB364_664:
	movq	%rax, %rbx
	jmp	.LBB364_675
.LBB364_665:
.Ltmp15712:
	movq	%rax, %rbx
.LBB364_666:
.Ltmp15724:
	leaq	1496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15725:
.LBB364_667:
	movq	528(%rsp), %rsi
	testq	%rsi, %rsi
	jle	.LBB364_669
	movq	536(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB364_669:
	testb	%r15b, %r15b
	je	.LBB364_671
.LBB364_670:
.Ltmp15740:
	leaq	192(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>), purrdf_hash::fixed::FixedState>>
.Ltmp15741:
.LBB364_671:
.Ltmp15745:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp15746:
	jmp	.LBB364_688
.LBB364_672:
.Ltmp15616:
.LBB364_673:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB364_675
	movq	1120(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB364_675:
	leaq	656(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB364_676:
	movq	560(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_679
	lock		decq	(%rax)
	jne	.LBB364_679
	#MEMBARRIER
.Ltmp15637:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	560(%rsp), %rdi
	callq	*%rax
.Ltmp15638:
.LBB364_679:
	movb	$1, %bpl
.Ltmp15642:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp15643:
	movb	$1, %r12b
	jmp	.LBB364_707
.LBB364_680:
.Ltmp15506:
	movq	%rax, %rbx
	movq	%r12, (%r14)
	jmp	.LBB364_693
.LBB364_681:
.Ltmp15678:
	movl	$8, %edx
	movq	%rax, %rbx
	movq	%r12, %rdi
	movq	%r15, %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB364_712
.LBB364_682:
.Ltmp15749:
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB364_686
	negq	%r12
	addq	$24, %r15
	.p2align	4
.LBB364_684:
.Ltmp15750:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp15751:
	addq	$24, %r15
	decq	%r12
	jne	.LBB364_684
.LBB364_686:
	movq	480(%rsp), %rax
	testq	%rax, %rax
	je	.LBB364_688
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB364_688:
	movq	560(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB364_713
	movq	568(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB364_713
.LBB364_690:
.Ltmp15752:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB364_691:
.Ltmp15509:
.LBB364_692:
	movq	%rax, %rbx
.LBB364_693:
	movq	256(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB364_696
	movq	264(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB364_695:
	callq	__rustc::__rust_dealloc
.LBB364_696:
	leaq	880(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>
	jmp	.LBB364_724
.LBB364_697:
.Ltmp15484:
.LBB364_698:
	movq	%rax, %rbx
.Ltmp15488:
	leaq	1168(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15489:
	jmp	.LBB364_735
.LBB364_699:
.Ltmp15478:
	movq	%rax, %rbx
.Ltmp15479:
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15480:
	jmp	.LBB364_735
.LBB364_700:
.Ltmp15481:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB364_701:
.Ltmp15571:
.LBB364_702:
	movq	%rax, %rbx
	cmpq	$6, %r15
	jb	.LBB364_705
	movq	952(%rsp), %r13
.LBB364_704:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r13, %rdi
	callq	__rustc::__rust_dealloc
.LBB364_705:
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB364_706:
	movb	$1, %bpl
	movb	$1, %r12b
.LBB364_707:
.Ltmp15647:
	leaq	584(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp15648:
.LBB364_708:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB364_725
.LBB364_709:
.Ltmp15665:
	movq	%rax, %rbx
.LBB364_710:
	movb	$1, %bpl
	movb	$1, %r12b
	jmp	.LBB364_727
.LBB364_711:
.Ltmp15764:
	movq	%rax, %rbx
.LBB364_712:
	movb	$1, %bpl
.LBB364_713:
	movq	104(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_715
	#MEMBARRIER
.Ltmp15765:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	104(%rsp), %rdi
	callq	*%rax
.Ltmp15766:
.LBB364_715:
	testb	%bpl, %bpl
	je	.LBB364_735
	movq	80(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_735
	#MEMBARRIER
.Ltmp15767:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	callq	*%rax
.Ltmp15768:
	jmp	.LBB364_735
.LBB364_718:
.Ltmp15769:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB364_719:
.Ltmp15531:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB364_721
	movq	608(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB364_721:
	testq	%r14, %r14
	je	.LBB364_723
	movq	64(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	movq	%r14, %rsi
	callq	__rustc::__rust_dealloc
.LBB364_723:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
.LBB364_724:
	movb	$1, %r12b
	movb	$1, %bpl
.LBB364_725:
	cmpq	$0, 232(%rsp)
	je	.LBB364_727
	movq	96(%rsp), %rdi
	movq	224(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB364_727:
	movq	56(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_729
	#MEMBARRIER
.Ltmp15666:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	callq	*%rax
.Ltmp15667:
.LBB364_729:
.Ltmp15668:
	leaq	624(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15669:
.LBB364_730:
	testb	%bpl, %bpl
	je	.LBB364_732
.Ltmp15670:
	leaq	1368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15671:
.LBB364_732:
	testb	%r12b, %r12b
	je	.LBB364_735
	movq	88(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB364_735
	#MEMBARRIER
.Ltmp15759:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	936(%rsp), %rdi
	callq	*%rax
.Ltmp15760:
.LBB364_735:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB364_736:
.Ltmp15761:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end364:
purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin365:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception272
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
	subq	$2376, %rsp
	.cfi_def_cfa_offset 2432
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	$0, 320(%rsp)
	movq	$8, 328(%rsp)
	movq	$0, 336(%rsp)
	movq	%r9, 312(%rsp)
	movq	%rdx, %r14
	movq	%rsi, 1344(%rsp)
	movq	%rdi, 96(%rsp)
	movq	%r8, 112(%rsp)
	vmovdqu	%ymm0, 344(%rsp)
	testq	%r8, %r8
	je	.LBB365_5
	movq	112(%rsp), %rbx
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r13
	movq	%rcx, %r15
	leaq	320(%rsp), %r12
	shlq	$4, %rbx
	addq	%rcx, %rbx
	.p2align	4
.LBB365_2:
	movq	(%r15), %rsi
	movq	8(%r15), %rdx
	lock		incq	(%rsi)
	jle	.LBB365_959
.Ltmp15776:
	movq	%r12, %rdi
	vzeroupper
	callq	*%r13
.Ltmp15777:
	addq	$16, %r15
	cmpq	%rbx, %r15
	jne	.LBB365_2
.LBB365_5:
	vmovdqu	344(%rsp), %ymm0
	movq	320(%rsp), %rax
	movq	328(%rsp), %rcx
	movq	336(%rsp), %rdx
	movq	344(%rsp), %rsi
	movq	2432(%rsp), %rdi
	movq	%rcx, 1608(%rsp)
	movq	%rax, 1600(%rsp)
	movq	%rdx, 1616(%rsp)
	imulq	$120, %rdi, %rcx
	addq	312(%rsp), %rcx
	vmovdqu	%ymm0, 1624(%rsp)
	movq	%rsi, 1624(%rsp)
	movq	1616(%rsp), %rax
	movq	%rcx, 48(%rsp)
	movq	%rax, 648(%rsp)
	testq	%rdi, %rdi
	je	.LBB365_11
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rbx
	movq	312(%rsp), %r12
	leaq	1600(%rsp), %r15
	.p2align	4
.LBB365_7:
	movq	(%r12), %rsi
	movq	1616(%rsp), %r13
	lock		incq	(%rsi)
	jle	.LBB365_959
	movq	8(%r12), %rdx
.Ltmp15782:
	movq	%r15, %rdi
	vzeroupper
	callq	*%rbx
.Ltmp15783:
	cmpq	%r13, %rax
	jne	.LBB365_64
	addq	$120, %r12
	cmpq	48(%rsp), %r12
	jne	.LBB365_7
.LBB365_11:
	movq	1600(%rsp), %rax
	vmovdqu	1632(%rsp), %xmm0
	movq	1624(%rsp), %rdi
	movq	1608(%rsp), %rcx
	movq	1616(%rsp), %rdx
	movq	1648(%rsp), %r8
	movq	1624(%rsp), %rsi
	movq	%rax, 336(%rsp)
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rdi, 360(%rsp)
	movl	$72, %edi
	movq	%r8, 384(%rsp)
	movq	%rcx, 344(%rsp)
	movq	%rdx, 352(%rsp)
	movq	%rsi, 360(%rsp)
	vmovdqu	%xmm0, 368(%rsp)
	movq	$1, 320(%rsp)
	movq	$1, 328(%rsp)
	vzeroupper
	callq	*%rax
	movq	%rax, 304(%rsp)
	testq	%rax, %rax
	je	.LBB365_953
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB365_14
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB365_14:
	movq	96(%rsp), %rbx
	leaq	328(%rsp), %rax
	movq	%rax, 152(%rsp)
	.p2align	4
.LBB365_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_21
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_15
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$72, (%rcx)
	movl	$72, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	$72, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB365_18:
	cmpq	%rax, %rcx
	jle	.LBB365_20
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB365_18
.LBB365_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_21:
	vmovdqu64	320(%rsp), %zmm0
	movq	304(%rsp), %rcx
	movq	384(%rsp), %rax
	movb	$1, %r15b
	movq	%rax, 64(%rcx)
	movq	%rcx, 1464(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp15790:
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	1344(%rsp), %rsi
	leaq	1696(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15791:
	movb	$1, %r15b
.Ltmp15792:
	movq	2440(%rsp), %rdx
	leaq	2256(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15793:
	cmpl	$1, 2256(%rsp)
	jne	.LBB365_47
	vmovdqu64	2304(%rsp), %zmm1
	vmovdqu64	2272(%rsp), %zmm0
	movq	1768(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rbx)
	vmovdqu64	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB365_34
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1776(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_27
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_27:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_33
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_27
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_30:
	cmpq	%rax, %rdx
	jge	.LBB365_32
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_30
.LBB365_32:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_33:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB365_34:
	movq	1696(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB365_44
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1704(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_37
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_37:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_43
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_37
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_40:
	cmpq	%rax, %rdx
	jge	.LBB365_42
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_40
.LBB365_42:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_43:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB365_44:
	movq	1792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_556
	lock		decq	(%rax)
	jne	.LBB365_556
	movb	$1, %r15b
	leaq	1792(%rsp), %rdi
	#MEMBARRIER
.Ltmp16191:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16192:
	jmp	.LBB365_556
.LBB365_47:
	vmovdqu64	2296(%rsp), %zmm1
	vmovdqu64	2264(%rsp), %zmm0
	vmovdqu64	%zmm1, 352(%rsp)
	vmovdqu64	%zmm0, 320(%rsp)
.Ltmp15794:
	leaq	864(%rsp), %rdi
	leaq	1696(%rsp), %rsi
	leaq	320(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15795:
	cmpq	$-1, 864(%rsp)
	je	.LBB365_74
	vmovdqu	864(%rsp), %ymm0
	vmovdqu	%ymm0, 1264(%rsp)
	movq	1288(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB365_959
	movq	304(%rsp), %rcx
	movq	1288(%rsp), %rax
	movq	648(%rsp), %rsi
	movq	32(%rcx), %rdx
	movq	%rax, 176(%rsp)
	cmpq	%rdx, %rsi
	ja	.LBB365_885
	movq	%rsi, %r14
	shlq	$4, %r14
	movq	%rsi, 640(%rsp)
	movq	%r14, 632(%rsp)
	testq	%rsi, %rsi
	je	.LBB365_76
	movq	304(%rsp), %rax
	movq	%r14, %rdi
	movq	24(%rax), %rbx
	movq	malloc@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	%rax, 192(%rsp)
	testq	%rax, %rax
	je	.LBB365_957
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r14, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %r14
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%r14, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB365_55
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_55:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_61
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_55
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rax
	movq	632(%rsp), %rdx
	lock		addq	%rdx, (%rax)
	movq	%rcx, %rdx
	lock		xaddq	%rdx, (%rsi)
	movabsq	$-9223372036854775808, %rsi
	leaq	(%rdx,%rcx), %rax
	sarq	$63, %rax
	xorq	%rax, %rsi
	addq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rsi, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_58:
	cmpq	%rax, %rdx
	jle	.LBB365_60
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_58
.LBB365_60:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_61:
	movq	640(%rsp), %r15
	xorl	%r14d, %r14d
	.p2align	4
.LBB365_62:
	movq	176(%rsp), %rdi
	leaq	(%rbx,%r14), %rsi
	addq	$16, %rdi
.Ltmp15796:
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)
.Ltmp15797:
	movq	192(%rsp), %rcx
	movq	%rax, (%rcx,%r14)
	movq	%rdx, 8(%rcx,%r14)
	addq	$16, %r14
	decq	%r15
	jne	.LBB365_62
	jmp	.LBB365_77
.LBB365_64:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$51, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB365_954
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	movq	96(%rsp), %rbx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$51, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$51, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB365_67
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_67:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_73
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_67
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$51, (%rsi)
	movl	$51, %esi
	lock		xaddq	%rsi, (%rdi)
	addq	$51, %rsi
	cmovoq	%rdx, %rsi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB365_70:
	cmpq	%rax, %rsi
	jle	.LBB365_72
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB365_70
.LBB365_72:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_73:
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528+19(%rip), %ymm0
	vmovups	.Lanon.e5162873a9a3251d11c4df37a70e4654.528(%rip), %ymm1
	movabsq	$9223372036854775793, %rax
	leaq	1600(%rsp), %rdi
	addq	$35, %rax
	movq	%rax, 16(%rbx)
	movq	$51, 24(%rbx)
	movq	%rcx, 32(%rbx)
	movq	$51, 40(%rbx)
	movq	$1, (%rbx)
	vmovups	%ymm0, 19(%rcx)
	vmovups	%ymm1, (%rcx)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
	jmp	.LBB365_558
.LBB365_74:
	vmovups	1736(%rsp), %zmm1
	vmovups	1696(%rsp), %zmm0
	movq	304(%rsp), %rax
	movq	%rax, 712(%rsp)
	movq	$0, 688(%rsp)
	movq	$8, 696(%rsp)
	movq	$0, 704(%rsp)
	vmovups	%zmm1, 360(%rsp)
	vmovups	%zmm0, 320(%rsp)
	cmpq	$-1, 320(%rsp)
	je	.LBB365_154
	leaq	864(%rsp), %rdi
	leaq	688(%rsp), %rsi
	leaq	1696(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	392(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB365_155
	jmp	.LBB365_164
.LBB365_76:
	movl	$8, %eax
	movq	%rax, 192(%rsp)
.LBB365_77:
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	1280(%rsp), %rax
	leaq	320(%rsp), %r11
	vmovdqu	%ymm0, 1664(%rsp)
	testq	%rax, %rax
	je	.LBB365_126
	movq	1272(%rsp), %rbx
	movq	192(%rsp), %rdx
	movq	632(%rsp), %r15
	leaq	(%rax,%rax,4), %rax
	movl	$2, %ecx
	leaq	328(%rsp), %r13
	movq	$0, 40(%rsp)
	vmovd	%ecx, %xmm0
	vmovdqa	%xmm0, 128(%rsp)
	leaq	(%rbx,%rax,8), %rax
	movq	%rax, 168(%rsp)
	leaq	(%rdx,%r15), %rax
	negq	%r15
	movq	%rax, 80(%rsp)
	jmp	.LBB365_81
	.p2align	4
.LBB365_79:
	movq	-16(%r12), %rax
	movq	40(%rsp), %rcx
	leaq	320(%rsp), %r11
	movq	%rcx, (%rax,%r14,8)
	incq	%r14
	movq	%rcx, %rax
	movq	%r14, -8(%r12)
.LBB365_80:
	incq	%rax
	addq	$40, %rbx
	movq	%rax, 40(%rsp)
	cmpq	168(%rsp), %rbx
	je	.LBB365_125
.LBB365_81:
	cmpq	$5, 640(%rsp)
	movl	$1, %ebp
	movl	$4, %eax
	movq	$1, 320(%rsp)
	movq	%r13, %rcx
	movq	%r11, %r14
	jae	.LBB365_92
	leaq	-1(%rbp), %rdx
	cmpq	%rax, %rdx
	jae	.LBB365_94
.LBB365_83:
	movq	192(%rsp), %r9
	leaq	8(%rbx), %rdx
	incq	%rax
	xorl	%r8d, %r8d
	jmp	.LBB365_85
	.p2align	4
.LBB365_84:
	vmovq	%xmm0, -8(%rcx,%rbp,8)
	addq	$16, %r9
	incq	%rbp
	addq	$-16, %r8
	cmpq	%rbp, %rax
	je	.LBB365_91
.LBB365_85:
	cmpq	%r8, %r15
	je	.LBB365_105
	vmovdqa	128(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB365_84
	movq	(%rbx), %rsi
	movq	%rdx, %r10
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB365_89
	movq	16(%rbx), %rsi
	movq	8(%rbx), %r10
	decq	%rsi
.LBB365_89:
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB365_907
	vmovq	(%r10,%rdi,8), %xmm0
	jmp	.LBB365_84
	.p2align	4
.LBB365_105:
	movq	%rbp, (%r14)
	jmp	.LBB365_106
	.p2align	4
.LBB365_91:
	movq	192(%rsp), %r12
	subq	%r8, %r12
	movq	%rax, (%r14)
	cmpq	80(%rsp), %r12
	jne	.LBB365_95
	jmp	.LBB365_106
.LBB365_92:
.Ltmp15801:
	movq	640(%rsp), %rdx
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%r11, %rdi
	xorl	%esi, %esi
	movq	%r11, %r12
	vzeroupper
	callq	*%rax
.Ltmp15802:
	movq	320(%rsp), %rax
	xorl	%edx, %edx
	movl	$4, %ecx
	leaq	336(%rsp), %rsi
	movq	%r12, %r14
	movq	%r12, %r11
	decq	%rax
	cmpq	$5, %rax
	cmovbq	%rcx, %rax
	movq	328(%rsp), %rcx
	setae	%dl
	cmovaeq	%rsi, %r14
	cmovbq	%r13, %rcx
	shll	$4, %edx
	movq	320(%rsp,%rdx), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rax, %rdx
	jb	.LBB365_83
	.p2align	4
.LBB365_94:
	movq	192(%rsp), %r12
	movq	%rbp, %rax
	movq	%rax, (%r14)
	cmpq	80(%rsp), %r12
	je	.LBB365_106
.LBB365_95:
	leaq	8(%rbx), %r14
	.p2align	4
.LBB365_96:
	vmovdqa	128(%rsp), %xmm0
	cmpl	$1, (%r12)
	vmovdqa	%xmm0, 64(%rsp)
	jne	.LBB365_101
	movq	(%rbx), %rsi
	movq	%r14, %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB365_99
	movq	16(%rbx), %rsi
	movq	8(%rbx), %rax
	decq	%rsi
.LBB365_99:
	movq	8(%r12), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB365_906
	vmovq	(%rax,%rdi,8), %xmm0
	vmovdqa	%xmm0, 64(%rsp)
.LBB365_101:
	movq	320(%rsp), %rsi
	movq	328(%rsp), %rax
	xorl	%edx, %edx
	leaq	336(%rsp), %rdi
	movq	%r11, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%r13, %rax
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	320(%rsp,%rdx), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rsi, %rdx
	je	.LBB365_103
.LBB365_102:
	vmovdqa	64(%rsp), %xmm0
	addq	$16, %r12
	vmovq	%xmm0, -8(%rax,%rbp,8)
	incq	%rbp
	movq	%rbp, (%rcx)
	cmpq	80(%rsp), %r12
	jne	.LBB365_96
	jmp	.LBB365_106
.LBB365_103:
.Ltmp15810:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r11, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15811:
	cmpq	$6, 320(%rsp)
	movq	328(%rsp), %rax
	leaq	320(%rsp), %r11
	leaq	336(%rsp), %rdx
	movq	%r11, %rcx
	cmovbq	%r13, %rax
	cmovaeq	%rdx, %rcx
	jmp	.LBB365_102
	.p2align	4
.LBB365_106:
	vmovdqu	320(%rsp), %ymm0
	movq	352(%rsp), %rax
	movq	1688(%rsp), %r14
	movq	%rax, 896(%rsp)
	vmovdqu	%ymm0, 864(%rsp)
.Ltmp15813:
	leaq	1664(%rsp), %rsi
	leaq	864(%rsp), %rdx
	movq	%r11, %rdi
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::rustc_entry
.Ltmp15814:
	movq	320(%rsp), %rax
	movq	328(%rsp), %r12
	testq	%rax, %rax
	je	.LBB365_120
	vmovdqu	16(%r13), %xmm0
	movq	%rax, 64(%rsp)
	movq	336(%rsp), %rcx
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r12, 56(%rsp)
	movq	360(%rsp), %rbp
	movq	368(%rsp), %r12
	movl	$8, %edi
	movq	%rcx, 104(%rsp)
	vmovdqa	%xmm0, 2064(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB365_948
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	leaq	320(%rsp), %r11
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$8, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$8, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB365_111
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_111:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_117
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_111
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$8, (%rdx)
	movl	$8, %edx
	lock		xaddq	%rdx, (%rdi)
	addq	$8, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB365_114:
	cmpq	%rax, %rdx
	jle	.LBB365_116
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB365_114
.LBB365_116:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_117:
	movq	40(%rsp), %rax
	movq	%rax, (%rcx)
	movq	8(%rbp), %rdx
	movq	(%rbp), %rax
	movq	%rdx, %rsi
	andq	%r12, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	je	.LBB365_122
.LBB365_118:
	tzcntl	%edi, %edi
	addq	%rsi, %rdi
	andq	%rdx, %rdi
	movzbl	(%rax,%rdi), %esi
	testb	%sil, %sil
	jns	.LBB365_124
.LBB365_119:
	shrq	$57, %r12
	leaq	-16(%rdi), %r8
	andb	$1, %sil
	movb	%r12b, (%rax,%rdi)
	negq	%rdi
	andq	%rdx, %r8
	movzbl	%sil, %esi
	leaq	(%rdi,%rdi,8), %rdx
	movq	64(%rsp), %rdi
	movb	%r12b, 16(%rax,%r8)
	subq	%rsi, 16(%rbp)
	movq	56(%rsp), %r8
	movq	%rdi, -72(%rax,%rdx,8)
	movq	104(%rsp), %rdi
	movq	%r8, -64(%rax,%rdx,8)
	movq	%rdi, -56(%rax,%rdx,8)
	vmovdqa	2064(%rsp), %xmm0
	vmovdqu	%xmm0, -48(%rax,%rdx,8)
	movq	%r14, -32(%rax,%rdx,8)
	movq	$1, -24(%rax,%rdx,8)
	movq	%rcx, -16(%rax,%rdx,8)
	movq	$1, -8(%rax,%rdx,8)
	incq	24(%rbp)
	movq	40(%rsp), %rax
	jmp	.LBB365_80
	.p2align	4
.LBB365_120:
	movq	-8(%r12), %r14
	cmpq	-24(%r12), %r14
	jne	.LBB365_79
.Ltmp15818:
	movq	<alloc::raw_vec::RawVec<usize>>::grow_one@GOTPCREL(%rip), %rax
	leaq	-24(%r12), %rdi
	callq	*%rax
.Ltmp15819:
	jmp	.LBB365_79
.LBB365_122:
	movl	$16, %r8d
.LBB365_123:
	addq	%r8, %rsi
	addq	$16, %r8
	andq	%rdx, %rsi
	vmovdqu	(%rax,%rsi), %xmm0
	vpmovmskb	%xmm0, %edi
	testl	%edi, %edi
	jne	.LBB365_118
	jmp	.LBB365_123
.LBB365_124:
	vmovdqa	(%rax), %xmm0
	vpmovmskb	%xmm0, %esi
	xorl	%edi, %edi
	tzcntl	%esi, %edi
	movzbl	(%rax,%rdi), %esi
	jmp	.LBB365_119
.LBB365_125:
	movq	1688(%rsp), %rax
	orq	%rax, 112(%rsp)
	cmpq	$0, 2432(%rsp)
	jne	.LBB365_127
	jmp	.LBB365_140
.LBB365_126:
	xorl	%eax, %eax
	cmpq	$0, 2432(%rsp)
	je	.LBB365_140
.LBB365_127:
	cmpq	$0, 112(%rsp)
	jne	.LBB365_140
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqa	%xmm0, 864(%rsp)
	movq	$8, 880(%rsp)
	movq	$1, 320(%rsp)
	movq	$0, 888(%rsp)
.Ltmp15821:
	leaq	688(%rsp), %rdi
	leaq	1664(%rsp), %rsi
	leaq	320(%rsp), %rdx
	leaq	864(%rsp), %rcx
	vzeroupper
	callq	<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::insert
.Ltmp15822:
	movq	696(%rsp), %rcx
	testq	%rcx, %rcx
	jle	.LBB365_139
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	704(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_132
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_132:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_138
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_132
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_135:
	cmpq	%rax, %rdx
	jge	.LBB365_137
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_135
.LBB365_137:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_138:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_139:
	movq	1688(%rsp), %rax
.LBB365_140:
	movq	1664(%rsp), %rcx
	movq	1672(%rsp), %rsi
	vmovdqa	(%rcx), %xmm0
	testq	%rsi, %rsi
	je	.LBB365_142
	leaq	(,%rsi,8), %rdx
	movq	%rcx, %r8
	movl	$16, %r9d
	leaq	(%rdx,%rdx,8), %rdx
	andq	$-16, %rdx
	subq	%rdx, %r8
	leaq	97(%rdx,%rsi), %rdi
	addq	$-80, %r8
	jmp	.LBB365_143
.LBB365_142:
	xorl	%r9d, %r9d
.LBB365_143:
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	leaq	1(%rcx,%rsi), %rsi
	leaq	16(%rcx), %rdx
	movq	%r9, 688(%rsp)
	movq	%rdi, 696(%rsp)
	movq	%r8, 704(%rsp)
	movq	%rcx, 712(%rsp)
	vpcmpgtb	%xmm1, %xmm0, %k0
	movq	%rdx, 720(%rsp)
	movq	%rsi, 728(%rsp)
	kmovw	%k0, 736(%rsp)
	movq	%rax, 744(%rsp)
	testq	%rax, %rax
	je	.LBB365_153
	kortestw	%k0, %k0
	je	.LBB365_146
	kmovd	%k0, %esi
	jmp	.LBB365_149
.LBB365_146:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	.p2align	4
.LBB365_147:
	vpcmpltb	(%rdx), %xmm0, %k0
	addq	$-1152, %rcx
	addq	$16, %rdx
	kortestw	%k0, %k0
	je	.LBB365_147
	kmovd	%k0, %esi
	movq	%rdx, 720(%rsp)
	movq	%rcx, 712(%rsp)
.LBB365_149:
	xorl	%edx, %edx
	blsrl	%esi, %edx
	tzcntl	%esi, %esi
	leaq	-1(%rax), %rdi
	negq	%rsi
	movw	%dx, 736(%rsp)
	movq	%rdi, 744(%rsp)
	leaq	(%rsi,%rsi,8), %rdx
	movq	-24(%rcx,%rdx,8), %r14
	cmpq	$-1, %r14
	je	.LBB365_153
	leaq	(%rcx,%rdx,8), %rcx
	cmpq	$5, %rax
	movl	$4, %ebp
	movabsq	$128102389400760776, %rdx
	cmovaeq	%rax, %rbp
	decq	%rdx
	movq	-40(%rcx), %r8
	movq	-64(%rcx), %rsi
	movq	-32(%rcx), %rdi
	movq	-72(%rcx), %r12
	movq	%r8, 1904(%rsp)
	movq	%rsi, 128(%rsp)
	vmovdqu	-56(%rcx), %xmm0
	vmovdqa	%xmm0, 1888(%rsp)
	movq	-16(%rcx), %rsi
	movq	%rsi, 64(%rsp)
	leaq	(,%rbp,8), %rsi
	leaq	(%rsi,%rsi,8), %rbx
	cmpq	%rdx, %rax
	jbe	.LBB365_168
	xorl	%r13d, %r13d
.LBB365_152:
.Ltmp15829:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15830:
	jmp	.LBB365_959
.LBB365_153:
	leaq	688(%rsp), %rdi
	movq	$0, 1184(%rsp)
	movq	$8, 1192(%rsp)
	movq	$0, 1200(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	movl	$8, %eax
	xorl	%r15d, %r15d
	movq	%rax, 64(%rsp)
	jmp	.LBB365_184
.LBB365_154:
	movq	696(%rsp), %rcx
	movq	688(%rsp), %rax
	movq	704(%rsp), %rdx
	movq	%rcx, 880(%rsp)
	movq	712(%rsp), %rcx
	movq	%rax, 872(%rsp)
	movq	%rdx, 888(%rsp)
	movq	%rcx, 896(%rsp)
	movq	$-1, 864(%rsp)
	movq	392(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB365_164
.LBB365_155:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	400(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_157
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_157:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_163
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_157
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_160:
	cmpq	%rax, %rdx
	jge	.LBB365_162
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_160
.LBB365_162:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_163:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB365_164:
	movq	416(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_167
	lock		decq	(%rax)
	jne	.LBB365_167
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	416(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB365_167:
	vmovdqu64	896(%rsp), %zmm1
	vmovdqu64	864(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	jmp	.LBB365_558
.LBB365_168:
	movq	-8(%rcx), %r15
	testq	%rbx, %rbx
	je	.LBB365_170
	movl	$8, %esi
	movq	%rdi, 80(%rsp)
	movq	%rbx, %rdi
	movl	$8, %r13d
	vzeroupper
	callq	__rustc::__rust_alloc
	movq	80(%rsp), %rdi
	testq	%rax, %rax
	jne	.LBB365_171
	jmp	.LBB365_152
.LBB365_170:
	movl	$8, %eax
	xorl	%ebp, %ebp
.LBB365_171:
	movq	128(%rsp), %rcx
	movq	%r12, (%rax)
	movq	64(%rsp), %rdx
	movq	%rcx, 8(%rax)
	vmovaps	1888(%rsp), %xmm0
	vmovups	%xmm0, 16(%rax)
	movq	1904(%rsp), %rcx
	movq	%rcx, 32(%rax)
	movq	%rdi, 40(%rax)
	movq	%r14, 48(%rax)
	movq	%rdx, 56(%rax)
	movq	%r15, 64(%rax)
	movq	%rbp, 224(%rsp)
	movq	%rax, 232(%rsp)
	movq	$1, 240(%rsp)
	vmovdqu64	688(%rsp), %zmm0
	vmovdqu64	%zmm0, 864(%rsp)
	movq	920(%rsp), %rdx
	testq	%rdx, %rdx
	je	.LBB365_183
	movzwl	912(%rsp), %ebp
	movq	888(%rsp), %r15
	movq	896(%rsp), %r12
	leaq	376(%rsp), %r13
	movl	$1, %ebx
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jmp	.LBB365_174
	.p2align	4
.LBB365_173:
	movq	384(%rsp), %rdx
	leaq	(%rbx,%rbx,8), %rcx
	incq	%rbx
	movq	%rdx, 64(%rax,%rcx,8)
	movq	%r14, %rdx
	vmovdqu64	320(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rax,%rcx,8)
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movq	%rbx, 240(%rsp)
	testq	%r14, %r14
	je	.LBB365_181
.LBB365_174:
	testw	%bp, %bp
	jne	.LBB365_177
	.p2align	4
.LBB365_175:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-1152, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB365_175
	kmovd	%k0, %ebp
.LBB365_177:
	xorl	%ecx, %ecx
	tzcntl	%ebp, %ecx
	leaq	-1(%rdx), %r14
	blsrl	%ebp, %ebp
	negq	%rcx
	leaq	(%rcx,%rcx,8), %rsi
	movq	-24(%r15,%rsi,8), %rcx
	cmpq	$-1, %rcx
	je	.LBB365_182
	leaq	(%r15,%rsi,8), %rsi
	vmovups	-16(%rsi), %xmm0
	movq	-32(%rsi), %rdi
	vmovaps	%xmm0, 1136(%rsp)
	vmovdqu	-72(%rsi), %ymm1
	movq	-40(%rsi), %rsi
	movq	%rsi, 352(%rsp)
	vmovdqu	%ymm1, 320(%rsp)
	movq	%rdi, 360(%rsp)
	movq	%rcx, 368(%rsp)
	vmovups	%xmm0, (%r13)
	cmpq	224(%rsp), %rbx
	jne	.LBB365_173
.Ltmp15824:
	movl	$8, %ecx
	movl	$72, %r8d
	leaq	224(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15825:
	movq	232(%rsp), %rax
	jmp	.LBB365_173
.LBB365_181:
	xorl	%r14d, %r14d
.LBB365_182:
	movq	%r12, 896(%rsp)
	movq	%r15, 888(%rsp)
	movw	%bp, 912(%rsp)
	movq	%r14, 920(%rsp)
.LBB365_183:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	vmovdqu	224(%rsp), %xmm0
	movq	240(%rsp), %r15
	movq	%r15, 1200(%rsp)
	vmovdqa	%xmm0, 1184(%rsp)
	movq	1192(%rsp), %rdi
	movq	%rdi, 64(%rsp)
	cmpq	$2, %r15
	jae	.LBB365_901
.LBB365_184:
	movq	304(%rsp), %rax
	movq	32(%rax), %rax
	movq	%rax, 1352(%rsp)
	movq	2440(%rsp), %rax
	cmpb	$2, 472(%rax)
	jne	.LBB365_187
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB365_260
	cmpq	$-2, 24(%rax)
	jb	.LBB365_187
	cmpq	$-2, 32(%rax)
	jae	.LBB365_271
.LBB365_187:
	xorl	%ebx, %ebx
.LBB365_188:
	movq	176(%rsp), %r8
	addq	$16, %r8
.Ltmp15851:
	movq	1344(%rsp), %rsi
	movq	312(%rsp), %rdx
	movq	2432(%rsp), %rcx
	movq	2440(%rsp), %r9
	leaq	1240(%rsp), %r12
	movq	%r12, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::modifier::link_aggregates::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15852:
	testb	%bl, %bl
	je	.LBB365_195
	movq	2440(%rsp), %rbx
	movq	1040(%rbx), %rax
	movq	616(%rbx), %rsi
	addq	904(%rbx), %rax
	movq	%rax, 1848(%rsp)
.Ltmp15875:
	leaq	1888(%rsp), %r14
	movq	%r14, %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::for_items::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15876:
.Ltmp15877:
	movq	%rbx, %rdi
	movq	%r15, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp15878:
	movq	%rax, 568(%rsp)
	movq	616(%rbx), %rax
	testq	%rax, %rax
	je	.LBB365_264
	cmpq	$-2, 16(%rax)
	movb	$1, %cl
	jb	.LBB365_265
	cmpq	$-2, 40(%rax)
	setb	%cl
	jmp	.LBB365_265
.LBB365_195:
	testq	%r15, %r15
	je	.LBB365_262
	movq	malloc@GOTPCREL(%rip), %rbx
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	callq	*%rbx
	testq	%rax, %rax
	je	.LBB365_958
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	64(%rsp), %r12
	movq	$-1, %r8
	leaq	(%r14,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%r14, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%r14, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB365_199
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_199:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_205
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_199
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%r14, (%rdx)
	movq	%r14, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%r14), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%r14, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_202:
	cmpq	%rax, %rdx
	jle	.LBB365_204
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_202
.LBB365_204:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_205:
	addq	$16, 312(%rsp)
	leaq	(%r15,%r15,8), %rax
	movq	%r15, 1888(%rsp)
	movq	%r13, 1896(%rsp)
	xorl	%ebp, %ebp
	movq	$0, 1904(%rsp)
	leaq	(%r12,%rax,8), %rax
	movq	%rax, 80(%rsp)
	jmp	.LBB365_208
.LBB365_206:
	movq	1896(%rsp), %r13
.LBB365_207:
	leaq	(%rbp,%rbp,4), %rax
	addq	$72, %r12
	incq	%rbp
	movq	%r15, (%r13,%rax,8)
	movq	%r14, 8(%r13,%rax,8)
	vmovdqa	320(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%r13,%rax,8)
	movq	336(%rsp), %rcx
	movq	%rcx, 32(%r13,%rax,8)
	movq	%rbp, 1904(%rsp)
	cmpq	80(%rsp), %r12
	je	.LBB365_263
.LBB365_208:
	movq	1352(%rsp), %r14
	movq	$1, 320(%rsp)
	cmpq	$5, %r14
	jae	.LBB365_233
	vmovdqu	328(%rsp), %xmm0
	movq	352(%rsp), %rax
	movq	320(%rsp), %rdx
	movq	344(%rsp), %rcx
	movq	%rax, 896(%rsp)
	movq	%rdx, 864(%rsp)
	movq	%rcx, 888(%rsp)
	vmovdqu	%xmm0, 872(%rsp)
	testq	%r14, %r14
	je	.LBB365_211
.LBB365_210:
	movl	$2, %eax
	jmp	.LBB365_212
.LBB365_211:
	movl	$-1, %eax
.LBB365_212:
	movl	%eax, 320(%rsp)
	movq	%r14, 328(%rsp)
.Ltmp15857:
	leaq	864(%rsp), %rdi
	leaq	320(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp15858:
	vmovdqu	864(%rsp), %ymm0
	movq	896(%rsp), %rax
	leaq	696(%rsp), %rdi
	movq	%rax, 720(%rsp)
	vmovdqu	%ymm0, 688(%rsp)
	movq	688(%rsp), %r15
	movq	%r15, %rax
	cmpq	$6, %r15
	jb	.LBB365_215
	movq	696(%rsp), %rdi
	movq	704(%rsp), %rax
.LBB365_215:
	movq	648(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB365_903
	movq	(%r12), %rsi
	movq	%r13, 64(%rsp)
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB365_218
	movq	16(%r12), %rsi
	movq	8(%r12), %rax
	decq	%rsi
	jmp	.LBB365_219
.LBB365_218:
	leaq	8(%r12), %rax
.LBB365_219:
	cmpq	%rsi, %rdx
	jne	.LBB365_904
	movq	memcpy@GOTPCREL(%rip), %rbx
	shlq	$3, %rdx
	movq	%rax, %rsi
	movq	%r12, %r13
	vzeroupper
	callq	*%rbx
	movq	1256(%rsp), %r12
	movq	2432(%rsp), %rax
	cmpq	%r12, %rax
	cmovbq	%rax, %r12
	testq	%r12, %r12
	je	.LBB365_231
	movq	%rbp, 128(%rsp)
	movq	1248(%rsp), %rbp
	movq	312(%rsp), %r14
	xorl	%ebx, %ebx
	addq	$16, %rbp
	jmp	.LBB365_223
	.p2align	4
.LBB365_222:
	incq	%rbx
	addq	$24, %rbp
	addq	$120, %r14
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%rbx, %r12
	je	.LBB365_230
.LBB365_223:
	vmovups	1272(%rsp), %xmm0
	movq	176(%rsp), %rax
	movq	-8(%rbp), %rdx
	movq	(%rbp), %rcx
	movq	56(%r13), %r8
	movq	64(%r13), %r9
	addq	$16, %rax
.Ltmp15862:
	movq	2440(%rsp), %rsi
	leaq	320(%rsp), %rdi
	movq	%rax, 16(%rsp)
	movq	%rsi, 24(%rsp)
	movq	%r14, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
.Ltmp15863:
	vmovq	328(%rsp), %xmm0
	movq	320(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB365_235
	movq	688(%rsp), %r15
	movq	%r15, %rsi
	cmpq	$6, %r15
	jb	.LBB365_227
	movq	704(%rsp), %rsi
.LBB365_227:
	movq	648(%rsp), %rdi
	decq	%rsi
	addq	%rbx, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB365_950
	leaq	696(%rsp), %rax
	cmpq	$6, %r15
	jb	.LBB365_222
	movq	696(%rsp), %rax
	jmp	.LBB365_222
.LBB365_230:
	movq	688(%rsp), %r15
	movq	128(%rsp), %rbp
.LBB365_231:
	leaq	696(%rsp), %rax
	movq	%r13, %r12
	movq	696(%rsp), %r14
	movq	96(%rsp), %rbx
	movq	64(%rsp), %r13
	vmovups	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 336(%rsp)
	vmovaps	%xmm0, 320(%rsp)
	cmpq	1888(%rsp), %rbp
	jne	.LBB365_207
.Ltmp15867:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1888(%rsp), %rdi
	callq	*%rax
.Ltmp15868:
	jmp	.LBB365_206
.LBB365_233:
.Ltmp15854:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	320(%rsp), %rdi
	xorl	%esi, %esi
	movq	%r14, %rdx
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp15855:
	vmovdqu	320(%rsp), %ymm0
	movq	352(%rsp), %rax
	movq	%rax, 896(%rsp)
	vmovdqu	%ymm0, 864(%rsp)
	jmp	.LBB365_210
.LBB365_235:
	vmovdqu64	336(%rsp), %zmm1
	vmovdqu64	352(%rsp), %zmm2
	movq	96(%rsp), %rcx
	vmovdqu64	%zmm2, 48(%rcx)
	vmovdqu64	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	688(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB365_237
	movq	696(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_237:
	movq	128(%rsp), %rbp
	movq	64(%rsp), %r13
	testq	%rbp, %rbp
	je	.LBB365_250
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r14
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	movq	free@GOTPCREL(%rip), %r12
	xorl	%ebx, %ebx
	jmp	.LBB365_242
	.p2align	4
.LBB365_239:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_240:
	vzeroupper
	callq	*%r12
.LBB365_241:
	incq	%rbx
	cmpq	%rbp, %rbx
	je	.LBB365_250
.LBB365_242:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB365_241
	leaq	(%r13,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_245
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_245:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_240
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_245
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r14)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r15), %rax
	.p2align	4
.LBB365_248:
	cmpq	%rax, %rdx
	jge	.LBB365_239
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB365_248
	jmp	.LBB365_239
.LBB365_250:
	movq	1888(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_472
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_253
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_253:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_259
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_253
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_256:
	cmpq	%rax, %rdx
	jge	.LBB365_258
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_256
.LBB365_258:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_259:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB365_472
.LBB365_260:
	cmpq	$0, 2432(%rsp)
	jne	.LBB365_272
	movb	$1, %bl
	jmp	.LBB365_188
.LBB365_262:
	movq	96(%rsp), %rbx
	movq	$0, 1888(%rsp)
	movq	$8, 1896(%rsp)
	movq	$0, 1904(%rsp)
.LBB365_263:
	movq	1904(%rsp), %rax
	movq	1888(%rsp), %rdx
	movq	1896(%rsp), %rcx
	movq	%rax, 1440(%rsp)
	movq	%rdx, 1424(%rsp)
	movq	%rcx, 1432(%rsp)
	jmp	.LBB365_809
.LBB365_264:
	xorl	%ecx, %ecx
.LBB365_265:
	movq	2440(%rsp), %rdx
	leaq	568(%rsp), %rsi
	movq	%r12, 584(%rsp)
	leaq	1352(%rsp), %rdi
	movq	%rdi, 224(%rsp)
	movq	312(%rsp), %rdi
	movq	%rdx, 592(%rsp)
	movq	%rsi, 600(%rsp)
	leaq	648(%rsp), %rsi
	movzbl	1234(%rdx), %eax
	movq	%r14, 608(%rsp)
	movq	%rsi, 232(%rsp)
	movq	2432(%rsp), %rsi
	movq	%rdi, 240(%rsp)
	leaq	1264(%rsp), %rdi
	xorb	$1, %al
	movq	%rsi, 248(%rsp)
	leaq	176(%rsp), %rsi
	movq	%rdi, 256(%rsp)
	leaq	583(%rsp), %rdi
	movq	%rsi, 264(%rsp)
	leaq	1848(%rsp), %rsi
	movq	%rdi, 272(%rsp)
	movq	%rsi, 280(%rsp)
	testb	%cl, %cl
	je	.LBB365_268
	movq	64(%rsp), %rbx
	cmpq	$1025, %r15
	movq	%r14, 1568(%rsp)
	setae	%cl
	testb	%al, %cl
	jne	.LBB365_275
	vmovdqu	584(%rsp), %ymm0
	vmovdqu64	224(%rsp), %zmm1
	leaq	1472(%rsp), %rax
	movq	%rbx, 1104(%rsp)
	movq	%r15, 1112(%rsp)
	movq	%r14, 656(%rsp)
	movq	%rax, 1136(%rsp)
	leaq	1104(%rsp), %rax
	movq	%rax, 1144(%rsp)
	leaq	864(%rsp), %rax
	movq	%rax, 1152(%rsp)
	leaq	656(%rsp), %rax
	movq	%rax, 1160(%rsp)
	vmovdqu	%ymm0, 1472(%rsp)
	vmovdqu64	%zmm1, 864(%rsp)
.Ltmp15937:
	leaq	320(%rsp), %rdi
	leaq	1136(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
.Ltmp15938:
	jmp	.LBB365_337
.LBB365_268:
	movq	64(%rsp), %rbp
	leaq	584(%rsp), %rcx
	cmpq	$1025, %r15
	movq	%r14, 1296(%rsp)
	movq	%rcx, 1136(%rsp)
	leaq	1568(%rsp), %rcx
	movq	%rcx, 1144(%rsp)
	leaq	224(%rsp), %rcx
	movq	%rcx, 1152(%rsp)
	leaq	1296(%rsp), %rcx
	movq	%rcx, 1160(%rsp)
	setae	%cl
	movq	%rbp, 1568(%rsp)
	movq	%r15, 1576(%rsp)
	testb	%al, %cl
	jne	.LBB365_277
.Ltmp15901:
	leaq	320(%rsp), %rdi
	leaq	1136(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
.Ltmp15902:
	jmp	.LBB365_337
.LBB365_275:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %r12
	movq	%fs:(%r12), %rax
	testq	%rax, %rax
	je	.LBB365_311
	addq	$272, %rax
	jmp	.LBB365_312
.LBB365_277:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB365_314
	addq	$272, %rax
	movq	%r15, %r14
	jmp	.LBB365_316
.LBB365_271:
	cmpq	$0, 2432(%rsp)
	sete	%cl
	cmpq	$-2, 48(%rax)
	setb	%al
	setae	%bl
	orb	%cl, %al
	jne	.LBB365_188
.LBB365_272:
	movq	2440(%rsp), %rax
	movq	312(%rsp), %r12
	leaq	320(%rsp), %rbp
	addq	$584, %rax
	movq	%rax, 128(%rsp)
.LBB365_273:
	movq	56(%r12), %r13
	movq	128(%rsp), %rbx
	testq	%r13, %r13
	je	.LBB365_291
	movq	48(%r12), %r14
	shlq	$6, %r13
	jmp	.LBB365_280
	.p2align	4
.LBB365_279:
	addq	$64, %r14
	addq	$-64, %r13
	je	.LBB365_291
.LBB365_280:
	movq	2440(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB365_283
	movq	%rbx, 320(%rsp)
.Ltmp15832:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15833:
	jmp	.LBB365_284
.LBB365_283:
.Ltmp15834:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.12908414067662811932)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15835:
.LBB365_284:
	testb	%al, %al
	jne	.LBB365_187
	movq	2440(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB365_279
	cmpq	$0, 336(%rax)
	jne	.LBB365_289
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB365_289
	cmpq	$-1, 48(%rax)
	je	.LBB365_279
.LBB365_289:
.Ltmp15836:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15837:
	testb	%al, %al
	je	.LBB365_279
	jmp	.LBB365_187
.LBB365_291:
	movq	104(%r12), %rax
	testq	%rax, %rax
	je	.LBB365_305
	movq	96(%r12), %r14
	shlq	$3, %rax
	leaq	(%rax,%rax,8), %r13
	addq	$8, %r14
	jmp	.LBB365_294
	.p2align	4
.LBB365_293:
	addq	$72, %r14
	addq	$-72, %r13
	je	.LBB365_305
.LBB365_294:
	movq	2440(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB365_297
	movq	%rbx, 320(%rsp)
.Ltmp15839:
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	movq	%rbp, %r8
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15840:
	jmp	.LBB365_298
.LBB365_297:
.Ltmp15841:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.12908414067662811932)(%rip), %r9
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.12908414067662811932)
.Ltmp15842:
.LBB365_298:
	testb	%al, %al
	jne	.LBB365_187
	movq	2440(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB365_293
	cmpq	$0, 336(%rax)
	jne	.LBB365_303
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB365_303
	cmpq	$-1, 48(%rax)
	je	.LBB365_293
.LBB365_303:
.Ltmp15843:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15844:
	testb	%al, %al
	je	.LBB365_293
	jmp	.LBB365_187
.LBB365_305:
	cmpl	$8, 16(%r12)
	jne	.LBB365_310
	movq	2440(%rsp), %rax
	movq	24(%r12), %rsi
	movq	32(%r12), %rdx
	movq	688(%rax), %rdi
	addq	$16, %rsi
.Ltmp15846:
	movq	<purrdf_sparql_eval::agg_fn::AggregateRegistry>::resolve@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15847:
	testq	%rax, %rax
	je	.LBB365_187
	movq	(%rax), %rcx
	movq	8(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp15848:
	callq	*%rax
.Ltmp15849:
	testb	%al, %al
	jne	.LBB365_187
.LBB365_310:
	addq	$120, %r12
	movb	$1, %bl
	cmpq	48(%rsp), %r12
	jne	.LBB365_273
	jmp	.LBB365_188
.LBB365_311:
.Ltmp15903:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15904:
.LBB365_312:
	movq	(%rax), %rax
	movq	520(%rax), %rbp
	movq	%r15, %rax
	cmpq	$1, %rbp
	adcq	$0, %rbp
	movq	%rbp, %rcx
	shlq	$6, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB365_318
	movq	%r15, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB365_319
.LBB365_314:
.Ltmp15879:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15880:
	movq	1568(%rsp), %rbp
	movq	1576(%rsp), %r14
.LBB365_316:
	movq	(%rax), %rax
	movq	520(%rax), %rcx
	movq	%r15, %rax
	cmpq	$1, %rcx
	adcq	$0, %rcx
	shlq	$2, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB365_325
	movq	%r15, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB365_326
.LBB365_318:
	movl	%r15d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB365_319:
	cmpq	$65, %rax
	movl	$64, %ecx
	movq	%rbx, 864(%rsp)
	movq	%r15, 872(%rsp)
	cmovaeq	%rax, %rcx
	movq	%rcx, 880(%rsp)
.Ltmp15905:
	leaq	656(%rsp), %r14
	leaq	864(%rsp), %rsi
	movq	%r14, %rdi
	callq	<alloc::vec::Vec<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)]> as alloc::vec::spec_from_iter::SpecFromIter<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)], core::slice::iter::Chunks<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>>::from_iter
.Ltmp15906:
	movq	672(%rsp), %r13
	movabsq	$38430716820228232, %rax
	imulq	$240, %r13, %r15
	cmpq	%rax, %r13
	jbe	.LBB365_323
	xorl	%r12d, %r12d
.LBB365_322:
.Ltmp15934:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15935:
	jmp	.LBB365_959
.LBB365_323:
	movq	%r12, %rbx
	testq	%r15, %r15
	je	.LBB365_374
	movl	$16, %esi
	movq	%r15, %rdi
	movl	$16, %r12d
	callq	__rustc::__rust_alloc
	movq	%r13, %rcx
	testq	%rax, %rax
	jne	.LBB365_375
	jmp	.LBB365_322
.LBB365_325:
	movl	%r15d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB365_326:
	cmpq	$17, %rax
	movl	$16, %r12d
	movq	$0, 1360(%rsp)
	movq	$16, 1368(%rsp)
	movq	$0, 1376(%rsp)
	cmovaeq	%rax, %r12
	movq	%r14, %rax
	orq	%r12, %rax
	shrq	$32, %rax
	je	.LBB365_328
	movq	%r14, %rax
	xorl	%edx, %edx
	divq	%r12
	jmp	.LBB365_329
.LBB365_328:
	movl	%r14d, %eax
	xorl	%edx, %edx
	divl	%r12d
.LBB365_329:
	xorl	%r15d, %r15d
	testq	%rdx, %rdx
	setne	%r15b
	addq	%rax, %r15
	movq	%r15, 200(%rsp)
	jne	.LBB365_908
	xorl	%eax, %eax
	xorl	%r13d, %r13d
	subq	%r13, %rax
	cmpq	%r15, %rax
	jb	.LBB365_910
.LBB365_331:
	leaq	584(%rsp), %rax
	leaq	224(%rsp), %rcx
	movq	%rbp, 864(%rsp)
	movq	%r14, 872(%rsp)
	movq	%r12, 880(%rsp)
	movq	1368(%rsp), %rbx
	leaq	1296(%rsp), %rdx
	movq	%r12, 1120(%rsp)
	movq	%rbp, 1104(%rsp)
	movq	%r14, 1112(%rsp)
	movq	%rax, 888(%rsp)
	movq	%rcx, 896(%rsp)
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rcx
	movq	%rdx, 904(%rsp)
	movq	%fs:(%rcx), %rax
	testq	%rax, %rax
	je	.LBB365_333
	addq	$272, %rax
	jmp	.LBB365_334
.LBB365_333:
.Ltmp15883:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15884:
.LBB365_334:
	movq	(%rax), %rax
	leaq	888(%rsp), %rdx
	movq	520(%rax), %rcx
	imulq	$224, %r13, %rax
	movq	%rdx, 1472(%rsp)
	addq	%rax, %rbx
	movq	%rbx, 1480(%rsp)
	movq	%r15, 1488(%rsp)
.Ltmp15885:
	leaq	1472(%rsp), %rbx
	leaq	656(%rsp), %rdi
	leaq	1104(%rsp), %r9
	movl	$1, %r8d
	movq	%r15, %rsi
	xorl	%edx, %edx
	movq	%rbx, (%rsp)
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
.Ltmp15886:
	movq	672(%rsp), %r14
	movq	%r14, 1472(%rsp)
	cmpq	%r15, %r14
	jne	.LBB365_911
	vmovdqu	1360(%rsp), %xmm0
	addq	%r15, %r13
	movq	%r13, 880(%rsp)
	vmovdqa	%xmm0, 864(%rsp)
.Ltmp15893:
	leaq	320(%rsp), %rdi
	leaq	864(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
.Ltmp15894:
.LBB365_337:
	movq	96(%rsp), %rbx
.LBB365_338:
	movq	320(%rsp), %rcx
	cmpq	$-1, %rcx
	je	.LBB365_351
	vmovups	480(%rsp), %zmm0
	vmovups	432(%rsp), %zmm2
	movq	344(%rsp), %rax
	vmovdqu	328(%rsp), %xmm1
	movq	360(%rsp), %rsi
	movq	352(%rsp), %rdx
	movq	336(%rsp), %rbx
	movq	%rcx, 1400(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rcx
	cmpq	$-2, %rcx
	movl	$1, %ecx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rcx
	vmovups	%zmm0, 800(%rsp)
	vmovups	%zmm2, 752(%rsp)
	vmovdqu64	368(%rsp), %zmm0
	decq	%rax
	vmovdqu	%xmm1, 1408(%rsp)
	vmovdqu64	800(%rsp), %zmm3
	vmovdqu64	752(%rsp), %zmm2
	vmovdqu64	%zmm0, 688(%rsp)
	vmovdqu64	%zmm0, 344(%rsp)
	vmovdqu64	%zmm3, 456(%rsp)
	vmovdqu64	%zmm2, 408(%rsp)
	movq	%rdi, 320(%rsp)
	movq	%rdx, 328(%rsp)
	movq	%rcx, 336(%rsp)
	movq	$0, 520(%rsp)
	movq	%rax, 528(%rsp)
.Ltmp15940:
	leaq	864(%rsp), %rdi
	leaq	320(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15941:
	vmovups	896(%rsp), %zmm1
	movq	2440(%rsp), %rax
	vmovups	864(%rsp), %ymm0
	vmovdqu64	960(%rsp), %zmm2
	cmpq	$0, 616(%rax)
	vmovups	%zmm1, 2080(%rsp)
	vmovdqu64	1008(%rsp), %zmm1
	vmovdqu64	%zmm2, 2144(%rsp)
	vmovups	%ymm0, 1568(%rsp)
	vmovdqu64	%zmm1, 2192(%rsp)
	je	.LBB365_352
	vmovdqu	1400(%rsp), %xmm0
	vmovdqu64	2080(%rsp), %zmm3
	vmovdqu64	2192(%rsp), %zmm2
	vmovdqu64	2144(%rsp), %zmm1
	addq	$888, %rax
	movzbl	2048(%rsp), %ebp
	movq	%rax, 64(%rsp)
	movq	1416(%rsp), %rax
	movq	%rax, 1312(%rsp)
	vmovdqu64	%zmm2, 976(%rsp)
	vmovdqa	%xmm0, 1296(%rsp)
	vmovdqu64	%zmm1, 928(%rsp)
	vmovdqu64	%zmm3, 864(%rsp)
	testb	%bpl, %bpl
	je	.LBB365_354
	movq	864(%rsp), %rax
	vmovdqu64	2104(%rsp), %zmm0
	vmovdqu64	2192(%rsp), %zmm2
	vmovdqu64	2168(%rsp), %zmm1
	movq	880(%rsp), %rcx
	movq	872(%rsp), %r14
	movl	$1, %edx
	movl	$1, %esi
	movq	$0, 224(%rsp)
	movq	$8, 232(%rsp)
	movq	$0, 240(%rsp)
	cmpq	$3, %rax
	movq	%rax, %r15
	cmovaeq	%rcx, %r15
	cmovaeq	%rdx, %rcx
	cmovaeq	%rax, %rsi
	leaq	328(%rsp), %rdx
	decq	%r15
	vmovdqu64	%zmm2, 432(%rsp)
	vmovdqu64	%zmm1, 408(%rsp)
	vmovdqu64	%zmm0, 344(%rsp)
	movq	%rsi, 320(%rsp)
	movq	%r14, 328(%rsp)
	movq	%rcx, 336(%rsp)
	movq	$0, 496(%rsp)
	movq	%r15, 504(%rsp)
	je	.LBB365_363
	cmpq	$3, %rax
	leaq	696(%rsp), %r12
	movl	$8, %ecx
	movb	%bpl, 48(%rsp)
	cmovbq	%rdx, %r14
	xorl	%ebx, %ebx
	xorl	%ebp, %ebp
	addq	$8, %r14
.LBB365_344:
	leaq	1(%rbx), %r13
	movq	%r13, 496(%rsp)
	movq	-8(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB365_395
	vmovups	(%r14), %zmm0
	vmovups	64(%r14), %zmm1
	vmovups	88(%r14), %zmm2
	vmovups	%zmm2, 88(%r12)
	vmovups	%zmm1, 64(%r12)
	vmovups	%zmm0, (%r12)
	movq	%rax, 688(%rsp)
	movq	%rbx, %rax
	movzbl	840(%rsp), %ebx
	cmpq	224(%rsp), %rax
	jne	.LBB365_348
.Ltmp15970:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15971:
	movq	232(%rsp), %rcx
.LBB365_348:
	vmovdqu64	688(%rsp), %zmm0
	vmovdqu64	752(%rsp), %zmm1
	vmovdqu64	784(%rsp), %zmm2
	vmovdqu64	%zmm2, 96(%rcx,%rbp)
	vmovdqu64	%zmm1, 64(%rcx,%rbp)
	vmovdqu64	%zmm0, (%rcx,%rbp)
	movq	%r13, 240(%rsp)
	testb	%bl, %bl
	jne	.LBB365_396
	addq	$160, %rbp
	addq	$168, %r14
	movq	%r13, %rbx
	cmpq	%r13, %r15
	jne	.LBB365_344
	xorl	%r12d, %r12d
	movq	%r15, %rbx
	jmp	.LBB365_397
.LBB365_351:
	vmovdqu64	368(%rsp), %zmm0
	vmovdqu	336(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%rbx)
	vmovdqu	%ymm1, 16(%rbx)
	vmovdqu64	%zmm0, 688(%rsp)
	movq	$1, (%rbx)
	movq	568(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB365_469
	jmp	.LBB365_471
.LBB365_352:
	leaq	1272(%rsp), %rcx
	movq	1264(%rsp), %rax
	vmovups	(%rcx), %xmm0
	movq	$0, 1264(%rsp)
	movq	$8, 1272(%rsp)
	movq	$0, 1280(%rsp)
	vmovaps	%xmm0, 864(%rsp)
	cmpq	%rbx, %rax
	jbe	.LBB365_358
	vmovdqa	864(%rsp), %xmm0
	leaq	320(%rsp), %rdi
	movq	%rax, 320(%rsp)
	vmovdqu	%xmm0, 328(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	$8, 1872(%rsp)
	movq	$0, 1880(%rsp)
	xorl	%eax, %eax
	jmp	.LBB365_359
.LBB365_354:
	movq	1312(%rsp), %rbx
	movq	$0, 584(%rsp)
	movq	$8, 592(%rsp)
	movq	$0, 600(%rsp)
.Ltmp15945:
	leaq	320(%rsp), %rdi
	leaq	584(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp15946:
	movq	320(%rsp), %r13
	movq	328(%rsp), %rax
	movq	336(%rsp), %r12
	movq	344(%rsp), %r15
	cmpq	$-1, %r13
	je	.LBB365_364
	vmovdqu	368(%rsp), %ymm0
	vmovdqu	384(%rsp), %ymm1
	movq	%rax, 64(%rsp)
	movq	360(%rsp), %rax
	movq	352(%rsp), %rbp
	movq	%rax, 1096(%rsp)
	vmovdqu	%ymm0, 688(%rsp)
	vmovdqu	%ymm1, 704(%rsp)
.Ltmp15950:
	leaq	1400(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15951:
	movq	96(%rsp), %rbx
.LBB365_357:
	vmovups	688(%rsp), %ymm0
	vmovups	704(%rsp), %ymm1
	movq	%rbp, 80(%rsp)
	shrq	$8, %rbp
	vmovups	%ymm0, 1472(%rsp)
	vmovups	%ymm1, 1488(%rsp)
	jmp	.LBB365_443
.LBB365_358:
	vmovdqa	864(%rsp), %xmm0
	vmovdqu	%xmm0, 1872(%rsp)
.LBB365_359:
	movl	2048(%rsp), %esi
	movq	%rax, 1864(%rsp)
.Ltmp16109:
	movq	2440(%rsp), %rdx
	leaq	320(%rsp), %rdi
	leaq	1400(%rsp), %rcx
	leaq	2080(%rsp), %r8
	leaq	1864(%rsp), %r9
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#9}>
.Ltmp16110:
	movq	320(%rsp), %rax
	movq	328(%rsp), %rdx
	movq	336(%rsp), %r12
	movq	344(%rsp), %r15
	movq	360(%rsp), %r14
	movq	352(%rsp), %rbx
	cmpq	$-1, %rax
	je	.LBB365_362
	vmovdqu	368(%rsp), %ymm0
	vmovdqu	384(%rsp), %ymm1
	movq	96(%rsp), %rcx
	vmovdqu	%ymm1, 80(%rcx)
	vmovdqu	%ymm0, 64(%rcx)
	movq	%rax, 16(%rcx)
	movq	%rdx, 24(%rcx)
	movq	%r12, 32(%rcx)
	movq	%r15, 40(%rcx)
	movq	%rbx, 48(%rcx)
	movq	%r14, 56(%rcx)
	movq	$1, (%rcx)
	jmp	.LBB365_467
.LBB365_362:
	movq	%rdx, 64(%rsp)
	jmp	.LBB365_800
.LBB365_363:
	xorl	%ebx, %ebx
	movl	$8, %r15d
	xorl	%r12d, %r12d
	jmp	.LBB365_398
.LBB365_364:
	movb	%bpl, 48(%rsp)
	movq	1304(%rsp), %rbp
	movq	%rax, 1136(%rsp)
	movq	1296(%rsp), %rax
	leaq	(,%rbx,8), %rcx
	movq	%r12, 1144(%rsp)
	movq	%r12, %rdx
	movq	%r15, 1152(%rsp)
	leaq	(%rcx,%rcx,4), %r12
	leaq	(%rbp,%r12), %rcx
	movq	%rbp, 224(%rsp)
	movq	%rax, 240(%rsp)
	movq	%rcx, 248(%rsp)
	testq	%rbx, %rbx
	je	.LBB365_441
	leaq	(,%r15,8), %rax
	addq	$40, %rbp
	movq	%rcx, 40(%rsp)
	leaq	(%rax,%rax,4), %rbx
	jmp	.LBB365_368
.LBB365_366:
	movq	1144(%rsp), %rdx
.LBB365_367:
	vmovdqa	80(%rsp), %xmm0
	movq	128(%rsp), %rax
	movq	%r14, (%rdx,%rbx)
	incq	%r15
	addq	$40, %rbp
	movq	%rax, 8(%rdx,%rbx)
	vmovdqu	%xmm0, 16(%rdx,%rbx)
	movq	%r13, 32(%rdx,%rbx)
	addq	$40, %rbx
	addq	$-40, %r12
	movq	%r15, 1152(%rsp)
	je	.LBB365_440
.LBB365_368:
	movq	2440(%rsp), %rcx
	leaq	696(%rsp), %rsi
	movq	%rcx, 688(%rsp)
	movq	-8(%rbp), %rax
	movq	%rax, 32(%rsi)
	vmovdqu	-40(%rbp), %ymm0
	vmovdqu	%ymm0, (%rsi)
	cmpq	$0, 696(%rsp)
	je	.LBB365_370
	leaq	-40(%rbp), %rax
	leaq	328(%rsp), %rsi
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rsi)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rsi)
	jmp	.LBB365_372
.LBB365_370:
	movq	%rdx, %r14
	movq	664(%rcx), %rdx
.Ltmp15953:
	movq	64(%rsp), %rsi
	leaq	320(%rsp), %rdi
	leaq	704(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp15954:
	movq	320(%rsp), %r13
	movq	%r14, %rdx
	cmpq	$-1, %r13
	jne	.LBB365_575
.LBB365_372:
	vmovdqu	344(%rsp), %xmm0
	movq	336(%rsp), %rax
	movq	328(%rsp), %r14
	movq	360(%rsp), %r13
	movq	%rax, 128(%rsp)
	vmovdqa	%xmm0, 80(%rsp)
	cmpq	1136(%rsp), %r15
	jne	.LBB365_367
.Ltmp15958:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1136(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15959:
	jmp	.LBB365_366
.LBB365_374:
	movl	$16, %eax
	xorl	%ecx, %ecx
.LBB365_375:
	testq	%r13, %r13
	je	.LBB365_378
	movl	%r13d, %edx
	andl	$7, %edx
	movq	%rbx, %r9
	cmpq	$8, %r13
	jae	.LBB365_379
	xorl	%esi, %esi
	leaq	864(%rsp), %rbx
	jmp	.LBB365_382
.LBB365_378:
	movq	%rbx, %r9
	xorl	%r15d, %r15d
	leaq	864(%rsp), %rbx
	jmp	.LBB365_385
.LBB365_379:
	movabsq	$72057594037927928, %rdi
	leaq	1696(%rax), %r8
	leaq	864(%rsp), %rbx
	xorl	%esi, %esi
	andq	%r13, %rdi
.LBB365_380:
	movl	$0, -1696(%r8)
	movb	$0, -1692(%r8)
	movq	$2, -1680(%r8)
	movl	$0, -1456(%r8)
	movb	$0, -1452(%r8)
	movq	$2, -1440(%r8)
	movl	$0, -1216(%r8)
	movb	$0, -1212(%r8)
	movq	$2, -1200(%r8)
	movl	$0, -976(%r8)
	movb	$0, -972(%r8)
	movq	$2, -960(%r8)
	movl	$0, -736(%r8)
	movb	$0, -732(%r8)
	movq	$2, -720(%r8)
	movl	$0, -496(%r8)
	movb	$0, -492(%r8)
	movq	$2, -480(%r8)
	movl	$0, -256(%r8)
	movb	$0, -252(%r8)
	movq	$2, -240(%r8)
	movl	$0, -16(%r8)
	movb	$0, -12(%r8)
	movq	$2, (%r8)
	addq	$8, %rsi
	addq	$1920, %r8
	cmpq	%rsi, %rdi
	jne	.LBB365_380
	testq	%rdx, %rdx
	je	.LBB365_384
.LBB365_382:
	imulq	$240, %rsi, %rsi
	imulq	$240, %rdx, %rdx
	xorl	%edi, %edi
	addq	%rax, %rsi
.LBB365_383:
	movl	$0, (%rsi,%rdi)
	movb	$0, 4(%rsi,%rdi)
	movq	$2, 16(%rsi,%rdi)
	addq	$240, %rdi
	cmpq	%rdi, %rdx
	jne	.LBB365_383
.LBB365_384:
	movq	672(%rsp), %r15
.LBB365_385:
	movq	%rcx, 1104(%rsp)
	movq	%rax, 1112(%rsp)
	movq	%fs:(%r9), %rax
	leaq	1360(%rsp), %rcx
	cmpq	%r15, %rbp
	leaq	584(%rsp), %rdx
	movq	%r13, 1120(%rsp)
	movq	$0, 1360(%rsp)
	movq	%rcx, 864(%rsp)
	leaq	224(%rsp), %rcx
	movq	%r14, 872(%rsp)
	movq	%rdx, 880(%rsp)
	cmovbq	%rbp, %r15
	leaq	1568(%rsp), %rdx
	movq	%rcx, 888(%rsp)
	leaq	1104(%rsp), %rcx
	movq	%rdx, 896(%rsp)
	movq	%rcx, 904(%rsp)
	testq	%rax, %rax
	je	.LBB365_387
	addq	$272, %rax
	jmp	.LBB365_388
.LBB365_387:
.Ltmp15907:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15908:
.LBB365_388:
	movq	(%rax), %rax
	movq	520(%rax), %rdx
.Ltmp15909:
	movl	$1, %ecx
	movq	%rbx, (%rsp)
	movq	%r15, %rdi
	xorl	%esi, %esi
	xorl	%r8d, %r8d
	movq	%r15, %r9
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::range::IterProducer<usize>, rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
.Ltmp15910:
	movq	1104(%rsp), %r12
	movq	1120(%rsp), %rdi
	movq	1112(%rsp), %rbx
	movabsq	$2635249153387078803, %rsi
	imulq	$240, %r12, %r11
	imulq	$240, %rdi, %rcx
	movq	%rbx, %r14
	movq	%r11, %rdx
	shrq	$5, %rdx
	leaq	(%rbx,%rcx), %rax
	mulxq	%rsi, %r15, %r15
	movabsq	$-8608480567731124087, %rsi
	movq	%rbx, %rdx
	testq	%rdi, %rdi
	je	.LBB365_425
	addq	$-240, %rcx
	movq	%rcx, %rdx
	mulxq	%rsi, %rdx, %rdx
	shrl	$7, %edx
	incl	%edx
	andl	$7, %edx
	je	.LBB365_405
	imulq	$240, %rdx, %r8
	movq	%rbx, %rdi
	movq	%rbx, %rdx
	jmp	.LBB365_393
.LBB365_392:
	addq	$240, %rdi
	addq	$-240, %r8
	je	.LBB365_406
.LBB365_393:
	vmovdqu64	24(%rdi), %zmm0
	vmovdqu64	88(%rdi), %zmm1
	vmovdqu64	152(%rdi), %zmm2
	vmovdqu64	176(%rdi), %zmm3
	movq	16(%rdi), %r9
	vmovdqu64	%zmm3, 1016(%rsp)
	vmovdqu64	%zmm2, 992(%rsp)
	vmovdqu64	%zmm1, 928(%rsp)
	vmovdqu64	%zmm0, 864(%rsp)
	cmpq	$2, %r9
	je	.LBB365_392
	movq	%r9, (%rdx)
	vmovdqu64	864(%rsp), %zmm0
	vmovdqu64	928(%rsp), %zmm1
	vmovdqu64	992(%rsp), %zmm2
	vmovdqu64	1016(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%rdx)
	vmovdqu64	%zmm0, 8(%rdx)
	vmovdqu64	%zmm1, 72(%rdx)
	vmovdqu64	%zmm3, 160(%rdx)
	addq	$224, %rdx
	jmp	.LBB365_392
.LBB365_395:
	xorl	%r12d, %r12d
	jmp	.LBB365_397
.LBB365_396:
	movb	$1, %r12b
	movq	%r13, %rbx
.LBB365_397:
	movzbl	48(%rsp), %ebp
	movq	%rcx, %r15
.LBB365_398:
.Ltmp15978:
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15979:
	movq	224(%rsp), %rbp
	movq	%rbx, 560(%rsp)
	testq	%rbx, %rbx
	je	.LBB365_402
	cmpq	$8, %rbx
	jae	.LBB365_403
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB365_452
.LBB365_402:
	xorl	%ebx, %ebx
	jmp	.LBB365_454
.LBB365_403:
	cmpq	$32, %rbx
	jae	.LBB365_445
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB365_449
.LBB365_405:
	movq	%rbx, %rdi
	movq	%rbx, %rdx
.LBB365_406:
	movq	%rax, %r14
	cmpq	$1680, %rcx
	jae	.LBB365_408
	jmp	.LBB365_425
.LBB365_407:
	addq	$1920, %rdi
	cmpq	%rax, %rdi
	je	.LBB365_424
.LBB365_408:
	vmovups	24(%rdi), %zmm0
	vmovups	88(%rdi), %zmm1
	vmovups	152(%rdi), %zmm2
	vmovups	176(%rdi), %zmm3
	movq	16(%rdi), %rcx
	vmovups	%zmm3, 1016(%rsp)
	vmovups	%zmm2, 992(%rsp)
	vmovups	%zmm1, 928(%rsp)
	vmovups	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_410
	movq	%rcx, (%rdx)
	vmovups	864(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	992(%rsp), %zmm2
	vmovups	1016(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB365_410:
	vmovups	264(%rdi), %zmm0
	vmovups	328(%rdi), %zmm1
	vmovups	392(%rdi), %zmm2
	vmovups	416(%rdi), %zmm3
	movq	256(%rdi), %rcx
	vmovups	%zmm3, 1016(%rsp)
	vmovups	%zmm2, 992(%rsp)
	vmovups	%zmm1, 928(%rsp)
	vmovups	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_412
	movq	%rcx, (%rdx)
	vmovups	864(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	992(%rsp), %zmm2
	vmovups	1016(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB365_412:
	vmovups	504(%rdi), %zmm0
	vmovups	568(%rdi), %zmm1
	vmovups	632(%rdi), %zmm2
	vmovups	656(%rdi), %zmm3
	movq	496(%rdi), %rcx
	vmovups	%zmm3, 1016(%rsp)
	vmovups	%zmm2, 992(%rsp)
	vmovups	%zmm1, 928(%rsp)
	vmovups	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_414
	movq	%rcx, (%rdx)
	vmovups	864(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	992(%rsp), %zmm2
	vmovups	1016(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB365_414:
	vmovups	744(%rdi), %zmm0
	vmovups	808(%rdi), %zmm1
	vmovups	872(%rdi), %zmm2
	vmovups	896(%rdi), %zmm3
	movq	736(%rdi), %rcx
	vmovups	%zmm3, 1016(%rsp)
	vmovups	%zmm2, 992(%rsp)
	vmovups	%zmm1, 928(%rsp)
	vmovups	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_416
	movq	%rcx, (%rdx)
	vmovups	864(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	992(%rsp), %zmm2
	vmovups	1016(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB365_416:
	vmovups	984(%rdi), %zmm0
	vmovups	1048(%rdi), %zmm1
	vmovups	1112(%rdi), %zmm2
	vmovups	1136(%rdi), %zmm3
	movq	976(%rdi), %rcx
	vmovups	%zmm3, 1016(%rsp)
	vmovups	%zmm2, 992(%rsp)
	vmovups	%zmm1, 928(%rsp)
	vmovups	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_418
	movq	%rcx, (%rdx)
	vmovups	864(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	992(%rsp), %zmm2
	vmovups	1016(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB365_418:
	vmovups	1224(%rdi), %zmm0
	vmovups	1288(%rdi), %zmm1
	vmovups	1352(%rdi), %zmm2
	vmovups	1376(%rdi), %zmm3
	movq	1216(%rdi), %rcx
	vmovups	%zmm3, 1016(%rsp)
	vmovups	%zmm2, 992(%rsp)
	vmovups	%zmm1, 928(%rsp)
	vmovups	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_420
	movq	%rcx, (%rdx)
	vmovups	864(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	992(%rsp), %zmm2
	vmovups	1016(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB365_420:
	vmovups	1464(%rdi), %zmm0
	vmovups	1528(%rdi), %zmm1
	vmovups	1592(%rdi), %zmm2
	vmovups	1616(%rdi), %zmm3
	movq	1456(%rdi), %rcx
	vmovups	%zmm3, 1016(%rsp)
	vmovups	%zmm2, 992(%rsp)
	vmovups	%zmm1, 928(%rsp)
	vmovups	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_422
	movq	%rcx, (%rdx)
	vmovups	864(%rsp), %zmm0
	vmovups	928(%rsp), %zmm1
	vmovups	992(%rsp), %zmm2
	vmovups	1016(%rsp), %zmm3
	vmovups	%zmm2, 136(%rdx)
	vmovups	%zmm0, 8(%rdx)
	vmovups	%zmm1, 72(%rdx)
	vmovups	%zmm3, 160(%rdx)
	addq	$224, %rdx
.LBB365_422:
	vmovdqu64	1704(%rdi), %zmm0
	vmovdqu64	1768(%rdi), %zmm1
	vmovdqu64	1832(%rdi), %zmm2
	vmovdqu64	1856(%rdi), %zmm3
	movq	1696(%rdi), %rcx
	vmovdqu64	%zmm3, 1016(%rsp)
	vmovdqu64	%zmm2, 992(%rsp)
	vmovdqu64	%zmm1, 928(%rsp)
	vmovdqu64	%zmm0, 864(%rsp)
	cmpq	$2, %rcx
	je	.LBB365_407
	movq	%rcx, (%rdx)
	vmovdqu64	864(%rsp), %zmm0
	vmovdqu64	928(%rsp), %zmm1
	vmovdqu64	992(%rsp), %zmm2
	vmovdqu64	1016(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%rdx)
	vmovdqu64	%zmm0, 8(%rdx)
	vmovdqu64	%zmm1, 72(%rdx)
	vmovdqu64	%zmm3, 160(%rdx)
	addq	$224, %rdx
	jmp	.LBB365_407
.LBB365_424:
	movq	%rax, %r14
.LBB365_425:
	vmovdqa	.LCPI365_0(%rip), %ymm0
	subq	%rbx, %rdx
	movabsq	$7905747460161236407, %rbp
	movq	%r11, 64(%rsp)
	movq	%rbx, %r13
	shrq	$5, %rdx
	imulq	%rdx, %rbp
	subq	%r14, %rax
	movq	%rax, %rdx
	mulxq	%rsi, %rax, %rax
	movq	%rbx, 1136(%rsp)
	movq	%r12, %rbx
	movq	%rbp, 1144(%rsp)
	movq	%r12, 1152(%rsp)
	vmovdqu	%ymm0, 864(%rsp)
	je	.LBB365_430
	shrq	$7, %rax
	movl	$1, %r12d
	addq	$256, %r14
	subq	%rax, %r12
	jmp	.LBB365_428
	.p2align	4
.LBB365_427:
	addq	$240, %r14
	incq	%r12
	cmpq	$1, %r12
	je	.LBB365_430
.LBB365_428:
	cmpl	$2, -240(%r14)
	je	.LBB365_427
.Ltmp15915:
	leaq	-240(%r14), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
.Ltmp15916:
	jmp	.LBB365_427
.LBB365_430:
	movq	64(%rsp), %rcx
	testq	%rbx, %rbx
	setne	%al
	imulq	$224, %r15, %r14
	cmpq	%r14, %rcx
	setne	%dl
	andb	%al, %dl
	cmpb	$1, %dl
	jne	.LBB365_436
	cmpq	$223, %rcx
	ja	.LBB365_435
	testq	%rcx, %rcx
	je	.LBB365_434
	movl	$16, %edx
	movq	%r13, %rdi
	movq	%rcx, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_434:
	movl	$16, %r13d
	jmp	.LBB365_436
.LBB365_435:
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	leaq	qualification_454_native_cost::GLOBAL (.llvm.1577329767756756036)(%rip), %rdi
	movl	$16, %edx
	movq	%r13, %rsi
	movq	%r14, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %r13
	testq	%rax, %rax
	je	.LBB365_949
.LBB365_436:
	movq	%r15, 1472(%rsp)
	movq	%r13, 1480(%rsp)
	movq	%rbp, 1488(%rsp)
.Ltmp15929:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15930:
.Ltmp15931:
	leaq	320(%rsp), %rdi
	leaq	1472(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
.Ltmp15932:
	movq	656(%rsp), %rsi
	movq	96(%rsp), %rbx
	testq	%rsi, %rsi
	je	.LBB365_338
	movq	664(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB365_338
.LBB365_440:
	movq	40(%rsp), %rbp
.LBB365_441:
	movq	%rbp, 232(%rsp)
.Ltmp15964:
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15965:
	movq	96(%rsp), %rbx
	movq	1136(%rsp), %rax
	movq	1144(%rsp), %r12
	movq	$-1, %r13
	xorl	%ebp, %ebp
	movq	$0, 80(%rsp)
	movq	%rax, 64(%rsp)
.LBB365_443:
.Ltmp15967:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15968:
	cmpq	$-1, %r13
	jne	.LBB365_466
	jmp	.LBB365_799
.LBB365_445:
	vmovdqa64	.LCPI365_1(%rip), %zmm1
	vpbroadcastq	.LCPI365_2(%rip), %zmm2
	vpbroadcastq	.LCPI365_3(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB365_446:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	64(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%r15,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB365_446
	vpaddq	%zmm0, %zmm4, %zmm0
	movq	560(%rsp), %rcx
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %rcx
	je	.LBB365_454
	testb	$24, %cl
	je	.LBB365_452
.LBB365_449:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI365_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI365_2(%rip), %zmm2
	vpbroadcastq	.LCPI365_4(%rip), %zmm3
	movq	560(%rsp), %rax
	vmovq	%rbx, %xmm0
	andq	$-8, %rax
	subq	%rax, %rcx
.LBB365_450:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r15,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB365_450
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, 560(%rsp)
	je	.LBB365_454
.LBB365_452:
	movq	560(%rsp), %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r15), %rax
	.p2align	4
.LBB365_453:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB365_453
.LBB365_454:
	movq	560(%rsp), %rax
	movq	1312(%rsp), %r14
	movq	%rbp, 1360(%rsp)
	movq	%r15, 1368(%rsp)
	movq	$0, 688(%rsp)
	movq	$8, 696(%rsp)
	movq	$0, 704(%rsp)
	movq	%rax, 1376(%rsp)
	movb	%r12b, 1384(%rsp)
.Ltmp15989:
	leaq	320(%rsp), %rdi
	leaq	688(%rsp), %rsi
	movq	%r14, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp15990:
	movq	320(%rsp), %r13
	movq	328(%rsp), %rax
	movq	336(%rsp), %rsi
	movq	344(%rsp), %rdx
	movq	%r15, 552(%rsp)
	movq	%rbp, 1224(%rsp)
	movq	%rbx, 1096(%rsp)
	cmpq	$-1, %r13
	je	.LBB365_559
	movq	%rax, 64(%rsp)
	movzbl	352(%rsp), %eax
	vmovdqu	368(%rsp), %ymm0
	vmovdqu	384(%rsp), %ymm1
	movzbl	359(%rsp), %r14d
	movzwl	357(%rsp), %ebx
	movl	353(%rsp), %ebp
	movq	%rsi, 40(%rsp)
	movq	%rdx, 128(%rsp)
	movq	%rax, 80(%rsp)
	movq	360(%rsp), %rax
	vmovdqu	%ymm0, 1136(%rsp)
	vmovdqu	%ymm1, 1152(%rsp)
	movq	%rax, 48(%rsp)
.Ltmp15994:
	leaq	1400(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15995:
	shll	$16, %r14d
	orl	%r14d, %ebx
	shlq	$32, %rbx
	orq	%rbx, %rbp
	movq	96(%rsp), %rbx
.LBB365_458:
	movq	560(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_462
	movq	552(%rsp), %r14
	movl	$1, %r15d
	subq	%rax, %r15
	.p2align	4
.LBB365_460:
.Ltmp16100:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16101:
	incq	%r15
	addq	$160, %r14
	cmpq	$1, %r15
	jne	.LBB365_460
.LBB365_462:
	movq	1224(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_464
	movq	552(%rsp), %rdi
	shlq	$5, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB365_464:
	cmpq	$-1, %r13
	je	.LBB365_564
	vmovdqu	1152(%rsp), %ymm1
	vmovdqu	1136(%rsp), %ymm0
	movq	48(%rsp), %rax
	movq	128(%rsp), %r15
	movq	40(%rsp), %r12
	movq	%rax, 1096(%rsp)
	vmovdqu	%ymm1, 1488(%rsp)
	vmovdqu	%ymm0, 1472(%rsp)
.LBB365_466:
	vmovdqu	1488(%rsp), %ymm1
	vmovdqu	1472(%rsp), %ymm0
	movq	64(%rsp), %rcx
	movzbl	80(%rsp), %eax
	shlq	$8, %rbp
	orq	%rbp, %rax
	vmovdqu	%ymm1, 80(%rbx)
	vmovdqu	%ymm0, 64(%rbx)
	movq	%r13, 16(%rbx)
	movq	%rcx, 24(%rbx)
	movq	1096(%rsp), %rcx
	movq	%r12, 32(%rbx)
	movq	%r15, 40(%rbx)
	movq	%rax, 48(%rbx)
	movq	%rcx, 56(%rbx)
	movq	$1, (%rbx)
.LBB365_467:
.Ltmp16114:
	leaq	1568(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16115:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_471
.LBB365_469:
	lock		decq	(%rax)
	jne	.LBB365_471
	#MEMBARRIER
.Ltmp16158:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16159:
.LBB365_471:
.Ltmp16163:
	leaq	1888(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16164:
.LBB365_472:
	movb	$1, %bl
	movq	1248(%rsp), %r14
	movq	1256(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_476
.LBB365_473:
	movl	$1, %r12d
	movq	%r14, %r15
	subq	%rax, %r12
	.p2align	4
.LBB365_474:
.Ltmp16168:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16169:
	incq	%r12
	addq	$24, %r15
	cmpq	$1, %r12
	jne	.LBB365_474
.LBB365_476:
	movq	1240(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_486
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_479
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_479:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_485
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_479
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_482:
	cmpq	%rax, %rdx
	jge	.LBB365_484
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_482
.LBB365_484:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_485:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.LBB365_486:
	movl	%ebx, 64(%rsp)
	movq	1192(%rsp), %rbx
	movq	1200(%rsp), %r14
	testq	%r14, %r14
	je	.LBB365_509
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	xorl	%r15d, %r15d
	jmp	.LBB365_491
	.p2align	4
.LBB365_488:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_489:
	vzeroupper
	callq	*%r13
.LBB365_490:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB365_509
.LBB365_491:
	leaq	(%r15,%r15,8), %rax
	leaq	(%rbx,%rax,8), %r12
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB365_501
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	8(%r12), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_494
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_494:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_500
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_494
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB365_497:
	cmpq	%rax, %rdx
	jge	.LBB365_499
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB365_497
.LBB365_499:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_500:
	vzeroupper
	callq	*%r13
.LBB365_501:
	movq	48(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB365_490
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	56(%r12), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_504
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_504:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_489
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_504
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB365_507:
	cmpq	%rax, %rdx
	jge	.LBB365_488
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB365_507
	jmp	.LBB365_488
.LBB365_509:
	movq	1184(%rsp), %rax
	movl	64(%rsp), %r15d
	testq	%rax, %rax
	je	.LBB365_519
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_512
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_512:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_518
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_512
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_515:
	cmpq	%rax, %rdx
	jge	.LBB365_517
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_515
.LBB365_517:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_518:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB365_519:
	cmpq	$0, 640(%rsp)
	movq	96(%rsp), %rbx
	je	.LBB365_529
	movq	632(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	cmpq	%rdi, %rax
	movq	%rdi, %rsi
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_522
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_522:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_528
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_522
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB365_525:
	cmpq	%rax, %rcx
	jge	.LBB365_527
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB365_525
.LBB365_527:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_528:
	movq	free@GOTPCREL(%rip), %rax
	movq	192(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB365_529:
	movq	176(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB365_531
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp16174:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16175:
.LBB365_531:
.Ltmp16177:
	leaq	1264(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16178:
	movq	1768(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB365_542
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1776(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_535
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_535:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_541
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_535
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_538:
	cmpq	%rax, %rdx
	jge	.LBB365_540
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_538
.LBB365_540:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_541:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_542:
	movq	1696(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB365_552
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	1704(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_545
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_545:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_551
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_545
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB365_548:
	cmpq	%rax, %rdx
	jge	.LBB365_550
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB365_548
.LBB365_550:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_551:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_552:
	movq	1792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_555
	lock		decq	(%rax)
	jne	.LBB365_555
	leaq	1792(%rsp), %rdi
	#MEMBARRIER
.Ltmp16180:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16181:
.LBB365_555:
	testb	%r15b, %r15b
	je	.LBB365_558
.LBB365_556:
	movq	304(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB365_558
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1464(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB365_558:
	movq	%rbx, %rax
	addq	$2376, %rsp
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
	vzeroupper
	retq
.LBB365_559:
	.cfi_def_cfa_offset 2432
	movq	1304(%rsp), %rbp
	movq	%rax, 200(%rsp)
	movq	1296(%rsp), %rax
	leaq	(%r14,%r14,4), %rcx
	movq	%rsi, 208(%rsp)
	movq	%rdx, 216(%rsp)
	movl	%r12d, 152(%rsp)
	leaq	(%rbp,%rcx,8), %rcx
	movq	%rbp, 656(%rsp)
	movq	%rax, 672(%rsp)
	movq	%rbp, 664(%rsp)
	movq	%rcx, 680(%rsp)
	movq	2440(%rsp), %rcx
	movq	616(%rcx), %rax
	movq	%rax, 1336(%rsp)
	testq	%rax, %rax
	je	.LBB365_565
	lock		incq	(%rax)
	jle	.LBB365_959
	movq	2440(%rsp), %rcx
	movq	560(%rsp), %rbx
	movq	616(%rcx), %rcx
	testq	%rbx, %rbx
	sete	%al
	movq	%rcx, 1392(%rsp)
	movq	%rcx, 104(%rsp)
	movq	16(%rcx), %rdx
	movq	40(%rcx), %rcx
	cmpq	$-1, %rcx
	movq	%rcx, 1216(%rsp)
	movq	%rdx, 1088(%rsp)
	sete	%cl
	orb	%al, %cl
	jne	.LBB365_600
	cmpq	$8, %rbx
	jae	.LBB365_584
	movq	552(%rsp), %rdx
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB365_596
.LBB365_564:
	movq	80(%rsp), %rcx
	jmp	.LBB365_788
.LBB365_565:
	movq	664(%rsp), %rcx
	movq	656(%rsp), %rax
	movq	672(%rsp), %rdi
	movq	%rcx, 232(%rsp)
	movq	680(%rsp), %rcx
	movq	%rax, 224(%rsp)
	movq	%rdi, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	248(%rsp), %rax
	movq	232(%rsp), %rbp
	movq	%rax, 104(%rsp)
	cmpq	%rax, %rbp
	je	.LBB365_586
	leaq	(,%rdx,8), %rax
	movq	%rsi, %r15
	movq	%rdx, %rbx
	leaq	(%rax,%rax,4), %r14
	jmp	.LBB365_568
.LBB365_567:
	movq	80(%rsp), %rax
	shll	$16, %r12d
	movq	48(%rsp), %rdx
	addq	$40, %rbp
	orl	%r12d, %r15d
	shlq	$32, %r15
	orq	%r15, %rbx
	movq	%rcx, %r15
	movq	%rax, (%rcx,%r14)
	movq	40(%rsp), %rax
	movq	%rax, 8(%rcx,%r14)
	movzbl	56(%rsp), %eax
	movq	%r13, 16(%rcx,%r14)
	movb	%al, 24(%rcx,%r14)
	movq	%rbx, %rax
	movl	%ebx, 25(%rcx,%r14)
	shrq	$32, %rbx
	shrq	$48, %rax
	movw	%bx, 29(%rcx,%r14)
	movq	128(%rsp), %rbx
	movb	%al, 31(%rcx,%r14)
	movq	%rdx, 32(%rcx,%r14)
	addq	$40, %r14
	incq	%rbx
	movq	%rbx, 216(%rsp)
	cmpq	104(%rsp), %rbp
	je	.LBB365_587
.LBB365_568:
	movq	32(%rbp), %rax
	leaq	696(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	2440(%rsp), %rax
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 688(%rsp)
	cmpq	$0, 696(%rsp)
	je	.LBB365_570
	movq	32(%rbp), %rax
	leaq	328(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB365_572
.LBB365_570:
	movq	664(%rax), %rdx
.Ltmp16079:
	movq	64(%rsp), %rsi
	leaq	320(%rsp), %rdi
	leaq	704(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16080:
	movq	320(%rsp), %r13
	cmpq	$-1, %r13
	jne	.LBB365_883
.LBB365_572:
	movq	336(%rsp), %rdx
	movq	%rbx, %rsi
	movq	%r15, %rcx
	movq	328(%rsp), %rax
	movzbl	352(%rsp), %edi
	movq	344(%rsp), %r13
	movzbl	359(%rsp), %r12d
	movzwl	357(%rsp), %r15d
	movl	353(%rsp), %ebx
	movq	%rsi, 128(%rsp)
	movq	%rdx, 40(%rsp)
	movq	360(%rsp), %rdx
	movq	%rax, 80(%rsp)
	movb	%dil, 56(%rsp)
	movq	%rdx, 48(%rsp)
	cmpq	200(%rsp), %rsi
	jne	.LBB365_567
.Ltmp16087:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16088:
	movq	208(%rsp), %rcx
	jmp	.LBB365_567
.LBB365_575:
	vmovups	368(%rsp), %ymm0
	movq	336(%rsp), %rcx
	movq	%rbp, 232(%rsp)
	movq	328(%rsp), %rax
	movq	352(%rsp), %rdx
	movq	344(%rsp), %rbp
	movq	%rcx, 40(%rsp)
	movq	360(%rsp), %rcx
	movq	%rax, 64(%rsp)
	movq	%rdx, 80(%rsp)
	vmovups	%ymm0, 688(%rsp)
	vmovdqu	384(%rsp), %ymm0
	movq	%rcx, 1096(%rsp)
	vmovdqu	%ymm0, 704(%rsp)
.Ltmp15956:
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15957:
	movq	%r14, %rdi
	testq	%r15, %r15
	je	.LBB365_581
	leaq	8(%rdi), %rbx
	xorl	%r12d, %r12d
	jmp	.LBB365_579
.LBB365_578:
	incq	%r12
	addq	$40, %rbx
	cmpq	%r12, %r15
	je	.LBB365_581
.LBB365_579:
	movq	-8(%rbx), %rax
	cmpq	$6, %rax
	jb	.LBB365_578
	movq	(%rbx), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	movq	%r14, %rdi
	jmp	.LBB365_578
.LBB365_581:
	movq	1136(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_583
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB365_583:
	movq	%rbp, %r15
	movq	96(%rsp), %rbx
	movq	40(%rsp), %r12
	movq	80(%rsp), %rbp
	jmp	.LBB365_357
.LBB365_584:
	cmpq	$32, %rbx
	jae	.LBB365_589
	movq	552(%rsp), %rdx
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB365_593
.LBB365_586:
	movq	%rdx, %rbx
.LBB365_587:
	movb	$1, %r15b
	movq	%rbx, 128(%rsp)
	movq	%rbp, 232(%rsp)
.Ltmp16092:
	leaq	224(%rsp), %rdi
	movb	$1, %r14b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16093:
	movl	152(%rsp), %r12d
	movq	200(%rsp), %rax
	movq	208(%rsp), %rcx
	movq	96(%rsp), %rbx
	movq	$-1, %r13
	movq	%rax, 64(%rsp)
	movb	$2, %al
	movq	%rcx, 40(%rsp)
	movq	%rax, 80(%rsp)
	jmp	.LBB365_458
.LBB365_589:
	vmovdqa64	.LCPI365_1(%rip), %zmm1
	vpbroadcastq	.LCPI365_2(%rip), %zmm2
	vpbroadcastq	.LCPI365_3(%rip), %zmm3
	movq	552(%rsp), %rdx
	movq	%rbx, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB365_590:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%rdx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%rdx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%rdx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%rdx,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB365_590
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r14
	cmpq	%rax, %rbx
	je	.LBB365_598
	testb	$24, %bl
	je	.LBB365_596
.LBB365_593:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI365_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI365_2(%rip), %zmm2
	vpbroadcastq	.LCPI365_4(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-8, %rax
	vmovq	%r14, %xmm0
	subq	%rax, %rcx
.LBB365_594:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rdx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB365_594
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r14
	cmpq	%rax, %rbx
	je	.LBB365_598
.LBB365_596:
	movq	%rbx, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rdx), %rax
.LBB365_597:
	addq	(%rax), %r14
	addq	$160, %rax
	decq	%rcx
	jne	.LBB365_597
.LBB365_598:
	movq	2440(%rsp), %rcx
	movq	912(%rcx), %rax
	movq	928(%rcx), %rsi
	leaq	912(%rcx), %r13
	subq	%rsi, %rax
	cmpq	%rax, %r14
	ja	.LBB365_951
.LBB365_599:
	movq	2440(%rsp), %rax
	cmpq	1016(%rax), %r14
	ja	.LBB365_952
.LBB365_600:
	movq	552(%rsp), %rax
	movq	1224(%rsp), %rcx
	movq	104(%rsp), %r15
	leaq	(%rbx,%rbx,4), %rdx
	shlq	$5, %rdx
	addq	%rax, %rdx
	movq	%rax, 1104(%rsp)
	movq	%rax, 1112(%rsp)
	movq	%rcx, 1120(%rsp)
	movq	%rdx, 1800(%rsp)
	movq	%rdx, 1128(%rsp)
	testq	%rbx, %rbx
	je	.LBB365_783
	leaq	16(%r15), %rcx
	leaq	272(%r15), %rdx
	movq	$-1, %r14
	movq	%rax, %rbx
	movq	%rcx, 184(%rsp)
	movq	%rdx, 1208(%rsp)
.LBB365_602:
	leaq	160(%rbx), %rdx
	movq	%rdx, 1112(%rsp)
	movq	(%rbx), %rax
	cmpq	$-1, %rax
	je	.LBB365_783
	movq	%rax, 320(%rsp)
	leaq	328(%rsp), %rcx
	movq	%rdx, 1808(%rsp)
	vmovdqu64	8(%rbx), %zmm0
	vmovdqu64	72(%rbx), %zmm1
	vmovdqu64	96(%rbx), %zmm2
	vmovdqu64	%zmm2, 88(%rcx)
	vmovdqu64	%zmm1, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	328(%rsp), %rdx
	movq	360(%rsp), %rsi
	imulq	$88, 336(%rsp), %rdi
	movq	344(%rsp), %rbx
	movq	352(%rsp), %rcx
	movq	368(%rsp), %r13
	movq	%rsi, 1832(%rsp)
	movq	%rdx, 584(%rsp)
	movq	%rax, 600(%rsp)
	movq	376(%rsp), %rax
	movq	384(%rsp), %rsi
	movq	%rdx, 592(%rsp)
	movq	%rcx, 168(%rsp)
	movq	%rbx, 624(%rsp)
	addq	%rdx, %rdi
	movq	%rdi, 112(%rsp)
	movq	%rdi, 608(%rsp)
	movq	%rax, 1328(%rsp)
	testq	%rsi, %rsi
	je	.LBB365_771
	movq	416(%rsp), %rdi
	shlq	$5, %rsi
	addq	$8, %rcx
	movq	$0, 1232(%rsp)
	movq	%rdx, 288(%rsp)
	movq	%rdx, 160(%rsp)
	movq	%r13, 296(%rsp)
	addq	%rax, %rsi
	movq	%rcx, 1816(%rsp)
	movq	%rsi, 1824(%rsp)
	movq	%rdi, 120(%rsp)
	jmp	.LBB365_607
.LBB365_605:
	movq	624(%rsp), %rbx
	movq	56(%rsp), %rbp
.LBB365_606:
	movq	1840(%rsp), %rax
	movq	%rbp, 664(%rsp)
	addq	$32, %rax
	cmpq	1824(%rsp), %rax
	je	.LBB365_771
.LBB365_607:
	movq	1232(%rsp), %rcx
	movq	16(%rax), %rdx
	movq	24(%rax), %rsi
	movq	8(%rax), %rbx
	movq	%rax, %r13
	movq	%rcx, 40(%rsp)
	movq	(%rax), %rcx
	movq	%rdx, 48(%rsp)
	movq	%rsi, 128(%rsp)
	testq	%rcx, %rcx
	je	.LBB365_614
	cmpq	$-1, 1088(%rsp)
	je	.LBB365_614
	movq	80(%r15), %rax
.LBB365_610:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%r14, %rdx
	lock		cmpxchgq	%rdx, 80(%r15)
	jne	.LBB365_610
	movq	184(%rsp), %rdx
	addq	%rcx, %rax
	cmovbq	%r14, %rax
	movq	(%rdx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB365_614
	movq	%rcx, 696(%rsp)
	movq	%rax, 704(%rsp)
	movw	$0, 688(%rsp)
.Ltmp16001:
	movq	184(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp16002:
	cmpb	$-1, 224(%rsp)
	jne	.LBB365_905
.LBB365_614:
	movq	1832(%rsp), %rdx
	movq	40(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB365_946
	movq	1216(%rsp), %r15
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	movq	%r13, 1840(%rsp)
	movq	%rbp, 56(%rsp)
	cmovbq	%rbx, %rsi
	cmpq	%rdi, %rbx
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB365_943
	leaq	(,%rdi,8), %rax
	leaq	.LJTI365_0(%rip), %r8
	leaq	(%rax,%rax,2), %r13
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r10
	cmpq	%rsi, %rdi
	jne	.LBB365_623
	xorl	%r12d, %r12d
	xorl	%ebx, %ebx
	xorl	%r11d, %r11d
.LBB365_618:
	movq	$-1, %rbp
	movq	%rbx, 616(%rsp)
	movq	%r11, 1456(%rsp)
	movq	%r10, 80(%rsp)
	movq	%rsi, 1232(%rsp)
	cmpq	$-1, %r15
	je	.LBB365_629
	movq	48(%rsp), %rax
	movl	$0, %ecx
	movq	112(%rsp), %r15
	movl	$0, %ebx
	subq	120(%rsp), %rax
	cmovbq	%rcx, %rax
	subq	160(%rsp), %r15
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %r15
	imulq	%rcx, %r15
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	testq	%r15, %r15
	je	.LBB365_630
	movq	160(%rsp), %rax
	xorl	%ebx, %ebx
	leaq	8(%rax), %r14
.LBB365_621:
.Ltmp16004:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16005:
	addq	%rax, %rbx
	cmovbq	%rbp, %rbx
	addq	$88, %r14
	decq	%r15
	jne	.LBB365_621
	jmp	.LBB365_630
.LBB365_623:
	movq	%r10, %rdx
	subq	%r13, %rdx
	movabsq	$-6148914691236517205, %rax
	xorl	%r11d, %r11d
	xorl	%ebx, %ebx
	xorl	%r12d, %r12d
	mulxq	%rax, %rax, %rax
	movq	1816(%rsp), %rcx
	shrq	$4, %rax
	addq	%r13, %rcx
	jmp	.LBB365_626
.LBB365_624:
	addq	%rdx, %rbx
	cmovbq	%r14, %rbx
.LBB365_625:
	addq	$24, %rcx
	decq	%rax
	je	.LBB365_618
.LBB365_626:
	movzbl	-8(%rcx), %r9d
	movq	(%rcx), %rdx
	movslq	(%r8,%r9,4), %r9
	addq	%r8, %r9
	jmpq	*%r9
.LBB365_627:
	addq	%rdx, %r12
	cmovbq	%r14, %r12
	jmp	.LBB365_625
.LBB365_628:
	cmpq	%rdx, %r11
	cmovbeq	%rdx, %r11
	jmp	.LBB365_625
.LBB365_629:
	xorl	%ebx, %ebx
.LBB365_630:
	movq	104(%rsp), %rax
	movq	$-1, %r14
	movl	296(%rax), %eax
	movq	1216(%rsp), %r15
	testl	%eax, %eax
	je	.LBB365_647
.LBB365_631:
	cmpq	$-1, 1088(%rsp)
	je	.LBB365_633
	movq	104(%rsp), %rcx
	movq	80(%rcx), %rax
	addq	%r12, %rax
	cmovbq	%r14, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB365_648
.LBB365_633:
	cmpq	$-1, %r15
	je	.LBB365_635
	movq	2440(%rsp), %rcx
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	104(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%rbx, %rcx
	cmovbq	%r14, %rcx
	addq	616(%rsp), %rcx
	cmovbq	%r14, %rcx
	addq	1456(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%r14, %rcx
	addq	%rcx, %rax
	cmovbq	%r14, %rax
	cmpq	40(%rdx), %rax
	ja	.LBB365_648
.LBB365_635:
	cmpq	$-1, 1088(%rsp)
	movq	1232(%rsp), %r9
	movq	160(%rsp), %rbp
	movq	80(%rsp), %r10
	movq	616(%rsp), %rbx
	movq	40(%rsp), %r11
	je	.LBB365_637
	movq	%r13, %rax
	cmpq	%r9, %r11
	jne	.LBB365_642
.LBB365_637:
	movb	$1, %r14b
	cmpq	%r9, %r11
	je	.LBB365_640
.LBB365_638:
	movq	168(%rsp), %rax
	cmpb	$2, -24(%rax,%r10)
	je	.LBB365_706
	addq	$-24, %r10
	cmpq	%r10, %r13
	jne	.LBB365_638
.LBB365_640:
	movq	104(%rsp), %r15
	movq	296(%rsp), %r13
	jmp	.LBB365_721
.LBB365_641:
	addq	$24, %rax
	cmpq	%rax, %r10
	je	.LBB365_637
.LBB365_642:
	movq	168(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB365_641
	movq	168(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB365_641
	movq	2440(%rsp), %rdx
	movq	632(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB365_641
	movq	2440(%rsp), %rsi
	movl	1228(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB365_641
	movq	168(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB365_641
.LBB365_647:
	movq	1208(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB365_631
.LBB365_648:
	movq	40(%rsp), %rax
	cmpq	1232(%rsp), %rax
	jne	.LBB365_658
	movq	288(%rsp), %rbp
	movq	120(%rsp), %r12
.LBB365_650:
	movq	%rbp, 288(%rsp)
	movq	%r12, 120(%rsp)
	cmpq	$-1, %r15
	je	.LBB365_704
	movq	104(%rsp), %r15
	movq	296(%rsp), %r13
	movq	160(%rsp), %rbp
	cmpq	48(%rsp), %r12
	movl	152(%rsp), %r12d
	jae	.LBB365_761
	cmpq	112(%rsp), %rbp
	je	.LBB365_705
	movq	48(%rsp), %rax
	leaq	-1(%rax), %rbx
.LBB365_654:
	movq	8(%rbp), %rax
	movq	%rbp, %rdx
	cmpq	$-1, %rax
	je	.LBB365_714
	movq	(%rdx), %rsi
	movq	%rax, 688(%rsp)
	leaq	696(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16042:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16043:
	movq	120(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB365_713
	movq	%rbp, %rdx
	incq	%rax
	addq	$88, %rbp
	movq	%rax, 120(%rsp)
	cmpq	112(%rsp), %rbp
	jne	.LBB365_654
	jmp	.LBB365_714
.LBB365_658:
	movq	168(%rsp), %rax
	movq	80(%rsp), %rsi
	movq	288(%rsp), %rbp
	movq	120(%rsp), %r12
	addq	%rax, %rsi
	addq	%rax, %r13
	movq	%rsi, 80(%rsp)
	jmp	.LBB365_661
.LBB365_703:
	movq	80(%rsp), %rsi
.LBB365_660:
	addq	$24, %r13
	cmpq	%rsi, %r13
	je	.LBB365_650
.LBB365_661:
	movzbl	(%r13), %eax
	leaq	.LJTI365_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB365_662:
	cmpq	$-1, 1088(%rsp)
	je	.LBB365_660
	movq	16(%r13), %rax
	movq	104(%rsp), %rdx
	movzbl	1(%r13), %ebx
	movq	8(%r13), %r14
	movq	$-1, %rsi
	movq	%rax, 40(%rsp)
	movq	80(%rdx), %rax
	.p2align	4
.LBB365_664:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB365_664
	movq	184(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB365_668
	movq	%rcx, 696(%rsp)
	movq	%rax, 704(%rsp)
	movw	$0, 688(%rsp)
.Ltmp16036:
	movq	184(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp16037:
	cmpb	$-1, 224(%rsp)
	jne	.LBB365_886
.LBB365_668:
	cmpl	$255, %ebx
	je	.LBB365_659
	movq	2440(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB365_659
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB365_659
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r14, (%rcx,%rbx,8)
.LBB365_659:
	movq	80(%rsp), %rsi
	movq	$-1, %r14
	jmp	.LBB365_660
.LBB365_672:
	movq	8(%r13), %rbx
	cmpq	%rbx, %r12
	jae	.LBB365_699
	movq	160(%rsp), %rax
	cmpq	112(%rsp), %rax
	je	.LBB365_698
	movq	160(%rsp), %rax
	leaq	-1(%rbx), %r14
.LBB365_675:
	movq	%rax, %rbp
	movq	8(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB365_697
	movq	(%rbp), %rsi
	movq	%rax, 688(%rsp)
	leaq	696(%rsp), %rcx
	movq	80(%rbp), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rbp), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16025:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16026:
	cmpq	%r12, %r14
	je	.LBB365_696
	leaq	88(%rbp), %rax
	incq	%r12
	cmpq	112(%rsp), %rax
	jne	.LBB365_675
	jmp	.LBB365_697
.LBB365_679:
	cmpq	$-1, %r15
	je	.LBB365_660
	movq	104(%rsp), %rdx
	movq	8(%r13), %rcx
	movl	296(%rdx), %eax
	testl	%eax, %eax
	je	.LBB365_691
	movq	104(%rdx), %rax
	addq	%rcx, %rax
	movq	40(%rdx), %rcx
	cmovbq	%r14, %rax
	cmpq	%rcx, %rax
	jbe	.LBB365_660
	movq	%rcx, 696(%rsp)
	movq	%rax, 704(%rsp)
	movw	$768, 688(%rsp)
.Ltmp16023:
	movq	184(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp16024:
	movq	80(%rsp), %rsi
	jmp	.LBB365_692
.LBB365_684:
	cmpq	$-1, %r15
	je	.LBB365_660
	movq	104(%rsp), %rsi
	cmpq	$-1, 40(%rsi)
	je	.LBB365_703
	movq	8(%r13), %rcx
	movq	16(%r13), %rbx
	movl	296(%rsi), %eax
	testl	%eax, %eax
	je	.LBB365_693
	movq	104(%rsi), %rax
.LBB365_688:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%r14, %rdx
	lock		cmpxchgq	%rdx, 104(%rsi)
	jne	.LBB365_688
	addq	%rcx, %rax
	movq	40(%rsi), %rcx
	cmovbq	%r14, %rax
	cmpq	%rcx, %rax
	jbe	.LBB365_703
	movq	%rcx, 696(%rsp)
	movq	%rax, 704(%rsp)
	movw	$768, 688(%rsp)
.Ltmp16030:
	movq	184(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp16031:
	jmp	.LBB365_694
.LBB365_691:
	movq	1208(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 240(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 224(%rsp)
.LBB365_692:
	cmpb	$-1, 224(%rsp)
	je	.LBB365_660
	jmp	.LBB365_914
.LBB365_693:
	movq	1208(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 240(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 224(%rsp)
.LBB365_694:
	movzbl	224(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%rbx, %rbx
	setne	%dl
	testb	%cl, %dl
	jne	.LBB365_894
	cmpb	$-1, %al
	jmp	.LBB365_702
.LBB365_696:
	movq	%rbx, %r12
.LBB365_697:
	addq	$88, %rbp
	movq	$-1, %r14
	movq	%rbp, 160(%rsp)
.LBB365_698:
	movq	%rbp, 592(%rsp)
.LBB365_699:
	cmpq	$-1, %r15
	je	.LBB365_703
.Ltmp16028:
	movq	2440(%rsp), %rsi
	leaq	688(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp16029:
	cmpb	$-1, 688(%rsp)
.LBB365_702:
	movq	80(%rsp), %rsi
	je	.LBB365_660
	jmp	.LBB365_914
.LBB365_704:
	movl	152(%rsp), %r12d
	movq	104(%rsp), %r15
	movq	296(%rsp), %r13
	movq	160(%rsp), %rbp
	jmp	.LBB365_761
.LBB365_705:
	movq	288(%rsp), %rdx
	jmp	.LBB365_715
.LBB365_706:
	movq	168(%rsp), %rax
	movq	104(%rsp), %r15
	movq	296(%rsp), %r13
	movq	-16(%rax,%r10), %rbx
	cmpq	%rbx, 120(%rsp)
	jae	.LBB365_720
	cmpq	112(%rsp), %rbp
	je	.LBB365_716
	leaq	-1(%rbx), %r14
.LBB365_709:
	movq	8(%rbp), %rax
	movq	%rbp, %rdx
	cmpq	$-1, %rax
	je	.LBB365_718
	movq	(%rdx), %rsi
	movq	%rax, 688(%rsp)
	leaq	696(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16007:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16008:
	movq	120(%rsp), %rax
	cmpq	%rax, %r14
	je	.LBB365_717
	movq	%rbp, %rdx
	incq	%rax
	addq	$88, %rbp
	movq	%rax, 120(%rsp)
	cmpq	112(%rsp), %rbp
	jne	.LBB365_709
	jmp	.LBB365_718
.LBB365_713:
	movq	48(%rsp), %rax
	movq	%rbp, %rdx
	movq	%rax, 120(%rsp)
.LBB365_714:
	addq	$88, %rdx
	movq	%rdx, %rbp
.LBB365_715:
	movq	%rdx, 288(%rsp)
	movq	%rdx, 592(%rsp)
	jmp	.LBB365_761
.LBB365_716:
	movq	288(%rsp), %rdx
	jmp	.LBB365_719
.LBB365_717:
	movq	%rbp, %rdx
	movq	%rbx, 120(%rsp)
.LBB365_718:
	addq	$88, %rdx
	movq	%rdx, %rbp
.LBB365_719:
	movq	%rdx, 288(%rsp)
	movq	%rdx, 592(%rsp)
.LBB365_720:
	movq	616(%rsp), %rbx
	xorl	%r14d, %r14d
.LBB365_721:
	cmpq	$-1, 1088(%rsp)
	je	.LBB365_730
	testq	%r12, %r12
	je	.LBB365_730
	movq	80(%r15), %rax
	movq	$-1, %rdx
.LBB365_724:
	movq	%rax, %rcx
	addq	%r12, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%r15)
	jne	.LBB365_724
	movq	184(%rsp), %rcx
	addq	%r12, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB365_730
	movq	%rcx, 696(%rsp)
	movq	%rax, 704(%rsp)
	movw	$0, 688(%rsp)
.Ltmp16010:
	movq	184(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp16011:
	cmpb	$-1, 224(%rsp)
	je	.LBB365_730
	cmpq	$-1, 1216(%rsp)
	movl	152(%rsp), %r12d
	movb	$1, %cl
	movq	$-1, %r14
	jne	.LBB365_749
	jmp	.LBB365_729
.LBB365_730:
	cmpq	$-1, 1216(%rsp)
	je	.LBB365_737
	cmpq	$-1, 40(%r15)
	movl	152(%rsp), %r12d
	je	.LBB365_741
	movl	296(%r15), %eax
	testl	%eax, %eax
	je	.LBB365_738
	movq	104(%r15), %rax
	movq	$-1, %rdx
.LBB365_734:
	movq	%rax, %rcx
	addq	%rbx, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 104(%r15)
	jne	.LBB365_734
	movq	40(%r15), %rcx
	addq	%rbx, %rax
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB365_741
	movq	%rcx, 696(%rsp)
	movq	%rax, 704(%rsp)
	movw	$768, 688(%rsp)
.Ltmp16013:
	movq	184(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp16014:
	jmp	.LBB365_739
.LBB365_737:
	movl	152(%rsp), %r12d
	movq	$-1, %r14
	jmp	.LBB365_761
.LBB365_738:
	movq	1208(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 240(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 224(%rsp)
.LBB365_739:
	cmpb	$-1, 224(%rsp)
	je	.LBB365_741
	movb	$1, %cl
	jmp	.LBB365_747
.LBB365_741:
	testb	%r14b, %r14b
	movq	$-1, %r14
	jne	.LBB365_744
.Ltmp16015:
	movq	2440(%rsp), %rsi
	leaq	688(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp16016:
	cmpb	$-1, 688(%rsp)
	movb	$1, %cl
	movq	$-1, %r14
	jne	.LBB365_749
.LBB365_744:
	movq	1456(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB365_748
.Ltmp16017:
	movq	184(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	688(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp16018:
	cmpb	$-1, 688(%rsp)
	setne	%cl
.LBB365_747:
	movq	$-1, %r14
	jmp	.LBB365_749
.LBB365_748:
	xorl	%ecx, %ecx
.LBB365_749:
	movq	48(%rsp), %rax
	cmpq	%rax, 120(%rsp)
	jae	.LBB365_760
	cmpq	112(%rsp), %rbp
	je	.LBB365_756
	movq	48(%rsp), %rax
	movl	%ecx, 80(%rsp)
	leaq	-1(%rax), %rbx
.LBB365_752:
	movq	8(%rbp), %rax
	movq	%rbp, %rdx
	cmpq	$-1, %rax
	je	.LBB365_758
	movq	(%rdx), %rsi
	movq	%rax, 688(%rsp)
	leaq	696(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16020:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	688(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16021:
	movq	120(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB365_757
	movq	%rbp, %rdx
	incq	%rax
	addq	$88, %rbp
	movq	%rax, 120(%rsp)
	cmpq	112(%rsp), %rbp
	jne	.LBB365_752
	jmp	.LBB365_758
.LBB365_756:
	movq	288(%rsp), %rdx
	jmp	.LBB365_759
.LBB365_757:
	movq	48(%rsp), %rax
	movq	%rbp, %rdx
	movq	%rax, 120(%rsp)
.LBB365_758:
	movl	80(%rsp), %ecx
	addq	$88, %rdx
	movq	%rdx, %rbp
.LBB365_759:
	movq	%rdx, 288(%rsp)
	movq	%rdx, 592(%rsp)
.LBB365_760:
	testb	%cl, %cl
	jne	.LBB365_729
.LBB365_761:
	cmpq	$0, 128(%rsp)
	movq	%rbp, 160(%rsp)
	je	.LBB365_605
	movq	680(%rsp), %rax
	movq	624(%rsp), %rbx
	movq	56(%rsp), %rbp
	movq	%rax, 616(%rsp)
	jmp	.LBB365_764
.LBB365_763:
	movq	208(%rsp), %rax
	movq	80(%rsp), %rdx
	leaq	(%rbx,%rbx,4), %rcx
	shll	$16, %ebp
	movq	48(%rsp), %rdi
	movq	128(%rsp), %rsi
	incq	%rbx
	addq	$40, %r14
	orl	%ebp, %r13d
	movq	%r14, %rbp
	movq	$-1, %r14
	shlq	$32, %r13
	orq	%r13, %r15
	movq	296(%rsp), %r13
	movq	%rdx, (%rax,%rcx,8)
	movq	40(%rsp), %rdx
	decq	%rsi
	movq	%rsi, 128(%rsp)
	movq	%rdx, 8(%rax,%rcx,8)
	movzbl	56(%rsp), %edx
	movq	%r12, 16(%rax,%rcx,8)
	movl	152(%rsp), %r12d
	movb	%dl, 24(%rax,%rcx,8)
	movq	%r15, %rdx
	shrq	$48, %rdx
	movl	%r15d, 25(%rax,%rcx,8)
	shrq	$32, %r15
	movw	%r15w, 29(%rax,%rcx,8)
	movb	%dl, 31(%rax,%rcx,8)
	movq	%rdi, 32(%rax,%rcx,8)
	movq	%rbx, 216(%rsp)
	movq	104(%rsp), %r15
	movq	624(%rsp), %rbx
	testq	%rsi, %rsi
	je	.LBB365_606
.LBB365_764:
	cmpq	616(%rsp), %rbp
	je	.LBB365_606
	movq	32(%rbp), %rax
	leaq	232(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	2440(%rsp), %rax
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 224(%rsp)
	cmpq	$0, 232(%rsp)
	je	.LBB365_767
	movq	32(%rbp), %rax
	leaq	696(%rsp), %rcx
	movq	%rbp, %r14
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB365_769
.LBB365_767:
	movq	664(%rax), %rdx
	movq	%rbp, %r14
.Ltmp16045:
	movq	64(%rsp), %rsi
	leaq	688(%rsp), %rdi
	leaq	240(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16046:
	movq	688(%rsp), %r13
	cmpq	$-1, %r13
	jne	.LBB365_881
.LBB365_769:
	movq	696(%rsp), %rax
	movq	704(%rsp), %rsi
	movzbl	720(%rsp), %edx
	movq	728(%rsp), %rcx
	movq	712(%rsp), %r12
	movzbl	727(%rsp), %ebp
	movzwl	725(%rsp), %r13d
	movl	721(%rsp), %r15d
	movq	216(%rsp), %rbx
	movq	%rax, 80(%rsp)
	movq	%rsi, 40(%rsp)
	movb	%dl, 56(%rsp)
	movq	%rcx, 48(%rsp)
	cmpq	200(%rsp), %rbx
	jne	.LBB365_763
.Ltmp16055:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	200(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16056:
	jmp	.LBB365_763
.LBB365_771:
	testq	%r13, %r13
	je	.LBB365_773
	movq	1328(%rsp), %rdi
	shlq	$5, %r13
	movl	$8, %edx
	movq	%r13, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_773:
.Ltmp16065:
	leaq	584(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16066:
	testq	%rbx, %rbx
	je	.LBB365_776
	movq	168(%rsp), %rdi
	shlq	$3, %rbx
	movl	$8, %edx
	leaq	(%rbx,%rbx,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB365_776:
	movq	408(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_779
	lock		decq	(%rax)
	jne	.LBB365_779
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB365_779:
	movq	440(%rsp), %rax
	movq	1808(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB365_782
	lock		decq	(%rax)
	jne	.LBB365_782
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB365_782:
	cmpq	1800(%rsp), %rbx
	jne	.LBB365_602
.LBB365_783:
	movb	$1, %al
	xorl	%r15d, %r15d
	movl	%eax, 56(%rsp)
.Ltmp16070:
	leaq	1104(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16071:
	movq	208(%rsp), %rcx
	movq	200(%rsp), %rax
	movq	216(%rsp), %rdx
	movq	%rcx, 40(%rsp)
	movq	104(%rsp), %rcx
	movq	%rax, 64(%rsp)
	movq	%rdx, 128(%rsp)
	lock		decq	(%rcx)
	jne	.LBB365_786
	xorl	%r15d, %r15d
	#MEMBARRIER
.Ltmp16075:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1392(%rsp), %rdi
	xorl	%r14d, %r14d
	callq	*%rax
.Ltmp16076:
.LBB365_786:
	xorl	%r15d, %r15d
.Ltmp16077:
	leaq	656(%rsp), %rdi
	xorl	%r14d, %r14d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16078:
	movb	$2, %cl
.LBB365_788:
	movq	64(%rsp), %rax
	movq	40(%rsp), %rdx
	movq	128(%rsp), %rsi
	movq	%rax, 688(%rsp)
	movq	%rdx, 696(%rsp)
	movq	2440(%rsp), %rdx
	movq	%rsi, 704(%rsp)
	movq	616(%rdx), %rax
	testq	%rax, %rax
	je	.LBB365_797
	movl	296(%rax), %esi
	movb	$-1, %bpl
	movq	%rcx, %rdx
	testl	%esi, %esi
	jne	.LBB365_791
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebp
	movq	%rcx, 239(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 224(%rsp)
.LBB365_791:
	testb	%dl, %dl
	je	.LBB365_796
	movzbl	%dl, %eax
	cmpl	$2, %eax
	je	.LBB365_798
	cmpb	$-1, %bpl
	je	.LBB365_882
	movq	2440(%rsp), %rax
	cmpb	$2, 472(%rax)
	jne	.LBB365_796
	vmovdqa	224(%rsp), %xmm0
	movq	696(%rax), %rdi
	movq	239(%rsp), %rax
	movb	%bpl, 320(%rsp)
	vmovdqu	%xmm0, 321(%rsp)
	movq	%rax, 336(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	jne	.LBB365_955
.LBB365_796:
	xorl	%r12d, %r12d
	jmp	.LBB365_798
.LBB365_797:
	cmpb	$2, %cl
	movb	$-1, %bpl
	sete	%al
	andb	%r12b, %al
	movl	%eax, %r12d
.LBB365_798:
	cmpb	$-1, %bpl
	movq	128(%rsp), %r15
	sete	%al
	andb	%r12b, %al
	movq	40(%rsp), %r12
	movq	%rax, 80(%rsp)
.LBB365_799:
	movzbl	80(%rsp), %ebx
	movq	1096(%rsp), %r14
.LBB365_800:
	vmovdqu	1568(%rsp), %ymm0
	movq	64(%rsp), %rax
	movq	%rax, 864(%rsp)
	movq	%r12, 872(%rsp)
	movq	%r15, 880(%rsp)
	vmovdqu	%ymm0, 320(%rsp)
.Ltmp16117:
	movq	2440(%rsp), %rdi
	leaq	320(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::absorb_worker_witnesses::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16118:
	testb	$1, %bl
	je	.LBB365_804
	movq	1200(%rsp), %rdx
	cmpq	%rdx, %r14
	ja	.LBB365_947
	jne	.LBB365_857
.LBB365_804:
	movq	880(%rsp), %rax
	vmovdqu	864(%rsp), %xmm0
	movq	%rax, 1440(%rsp)
	movq	568(%rsp), %rax
	vmovdqa	%xmm0, 1424(%rsp)
	testq	%rax, %rax
	je	.LBB365_807
	lock		decq	(%rax)
	jne	.LBB365_807
	#MEMBARRIER
.Ltmp16135:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	callq	*%rax
.Ltmp16136:
.LBB365_807:
.Ltmp16137:
	leaq	1888(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16138:
	movq	96(%rsp), %rbx
.LBB365_809:
	movq	2440(%rsp), %rax
	movq	696(%rax), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	je	.LBB365_812
.LBB365_810:
	vmovdqa	1424(%rsp), %xmm0
	movq	1440(%rsp), %rax
	cmpq	$-1, 1696(%rsp)
	movq	304(%rsp), %rcx
	movq	%rax, 880(%rsp)
	vmovdqa	%xmm0, 864(%rsp)
	movq	%rcx, 888(%rsp)
	je	.LBB365_815
	leaq	320(%rsp), %rdi
	leaq	864(%rsp), %rsi
	leaq	1696(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB365_816
.LBB365_812:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB365_810
	movb	%cl, 864(%rsp)
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 865(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 880(%rsp)
.Ltmp16139:
	movq	1344(%rsp), %rsi
	movq	304(%rsp), %rcx
	leaq	320(%rsp), %rdi
	leaq	864(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp16140:
	vmovdqu64	352(%rsp), %zmm1
	vmovdqu64	320(%rsp), %zmm0
	leaq	1424(%rsp), %rdi
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
	movq	1248(%rsp), %r14
	movq	1256(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB365_473
	jmp	.LBB365_476
.LBB365_815:
	vmovdqu	864(%rsp), %xmm0
	movq	880(%rsp), %rax
	movq	888(%rsp), %rcx
	movq	%rax, 344(%rsp)
	movq	%rcx, 352(%rsp)
	vmovdqu	%xmm0, 328(%rsp)
	movq	$-1, 320(%rsp)
.LBB365_816:
	movq	1768(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB365_818
	movq	1776(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB365_818:
	movq	1792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_821
	lock		decq	(%rax)
	jne	.LBB365_821
	leaq	1792(%rsp), %rdi
	#MEMBARRIER
.Ltmp16142:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16143:
.LBB365_821:
	vmovdqu64	352(%rsp), %zmm1
	vmovdqu64	320(%rsp), %zmm0
	movq	1248(%rsp), %r14
	movq	1256(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	testq	%rax, %rax
	je	.LBB365_825
	movl	$1, %r12d
	movq	%r14, %r15
	subq	%rax, %r12
	.p2align	4
.LBB365_823:
.Ltmp16145:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16146:
	incq	%r12
	addq	$24, %r15
	cmpq	$1, %r12
	jne	.LBB365_823
.LBB365_825:
	movq	1240(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_827
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_827:
	movq	1192(%rsp), %rbx
	movq	1200(%rsp), %r14
	testq	%r14, %r14
	je	.LBB365_850
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB365_832
	.p2align	4
.LBB365_829:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_830:
	vzeroupper
	callq	*%rbp
.LBB365_831:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB365_850
.LBB365_832:
	leaq	(%r15,%r15,8), %rax
	leaq	(%rbx,%rax,8), %r12
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB365_842
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	8(%r12), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_835
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_835:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_841
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_835
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB365_838:
	cmpq	%rax, %rdx
	jge	.LBB365_840
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB365_838
.LBB365_840:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_841:
	vzeroupper
	callq	*%rbp
.LBB365_842:
	movq	48(%r12), %rcx
	testq	%rcx, %rcx
	je	.LBB365_831
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	56(%r12), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_845
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_845:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_830
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_845
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB365_848:
	cmpq	%rax, %rdx
	jge	.LBB365_829
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB365_848
	jmp	.LBB365_829
.LBB365_850:
	movq	1184(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_852
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,8), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_852:
	cmpq	$0, 640(%rsp)
	je	.LBB365_854
	movq	192(%rsp), %rdi
	movq	632(%rsp), %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_854:
	movq	176(%rsp), %rax
	lock		decq	(%rax)
	movq	96(%rsp), %rbx
	jne	.LBB365_856
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp16151:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	xorl	%r15d, %r15d
	vzeroupper
	callq	*%rax
.Ltmp16152:
.LBB365_856:
	leaq	1264(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB365_558
.LBB365_857:
	movq	1192(%rsp), %rax
	leaq	(%rdx,%rdx,8), %rcx
	addq	$16, 312(%rsp)
	leaq	1528(%rsp), %rbx
	leaq	(%rax,%rcx,8), %rcx
	movq	%rcx, 64(%rsp)
	leaq	(%r14,%r14,8), %rcx
	leaq	(%rax,%rcx,8), %rbp
.LBB365_858:
	movq	1352(%rsp), %rsi
.Ltmp16119:
	movq	%rbx, %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
.Ltmp16120:
	movq	1528(%rsp), %r13
	leaq	1536(%rsp), %rdi
	movq	%r13, %rax
	cmpq	$6, %r13
	jb	.LBB365_861
	movq	1536(%rsp), %rdi
	movq	1544(%rsp), %rax
.LBB365_861:
	movq	648(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB365_944
	movq	(%rbp), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB365_864
	movq	16(%rbp), %rsi
	movq	8(%rbp), %rax
	decq	%rsi
	jmp	.LBB365_865
.LBB365_864:
	leaq	8(%rbp), %rax
.LBB365_865:
	cmpq	%rsi, %rdx
	jne	.LBB365_945
	movq	memcpy@GOTPCREL(%rip), %r14
	shlq	$3, %rdx
	movq	%rax, %rsi
	callq	*%r14
	movq	1256(%rsp), %rbx
	movq	2432(%rsp), %rax
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	testq	%rbx, %rbx
	je	.LBB365_876
	movq	1248(%rsp), %r15
	movq	312(%rsp), %r12
	xorl	%r14d, %r14d
	addq	$16, %r15
	jmp	.LBB365_869
.LBB365_868:
	incq	%r14
	addq	$24, %r15
	addq	$120, %r12
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%r14, %rbx
	je	.LBB365_876
.LBB365_869:
	vmovups	1272(%rsp), %xmm0
	movq	176(%rsp), %rax
	movq	-8(%r15), %rdx
	movq	(%r15), %rcx
	movq	56(%rbp), %r8
	movq	64(%rbp), %r9
	addq	$16, %rax
.Ltmp16124:
	movq	2440(%rsp), %rsi
	leaq	320(%rsp), %rdi
	movq	%rax, 16(%rsp)
	movq	%rsi, 24(%rsp)
	movq	%r12, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
.Ltmp16125:
	vmovq	328(%rsp), %xmm0
	movq	320(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB365_878
	movq	1528(%rsp), %r13
	movq	%r13, %rsi
	cmpq	$6, %r13
	jb	.LBB365_873
	movq	1544(%rsp), %rsi
.LBB365_873:
	movq	648(%rsp), %rdi
	decq	%rsi
	addq	%r14, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB365_956
	leaq	1536(%rsp), %rax
	cmpq	$6, %r13
	jb	.LBB365_868
	movq	1536(%rsp), %rax
	jmp	.LBB365_868
.LBB365_876:
.Ltmp16129:
	leaq	1528(%rsp), %rbx
	leaq	864(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::push_mut
.Ltmp16130:
	addq	$72, %rbp
	cmpq	64(%rsp), %rbp
	jne	.LBB365_858
	jmp	.LBB365_804
.LBB365_878:
	vmovups	336(%rsp), %zmm1
	vmovups	352(%rsp), %zmm2
	movq	96(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1528(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB365_880
	movq	1536(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_880:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	568(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB365_469
	jmp	.LBB365_471
.LBB365_881:
	movq	704(%rsp), %rcx
	vmovups	736(%rsp), %ymm0
	movzbl	727(%rsp), %edx
	movq	696(%rsp), %rax
	movl	721(%rsp), %ebp
	movq	712(%rsp), %rsi
	addq	$40, %r14
	movb	$1, %r15b
	movq	%r14, 664(%rsp)
	movq	%rcx, 40(%rsp)
	movzwl	725(%rsp), %ecx
	shll	$16, %edx
	movq	%rax, 64(%rsp)
	movzbl	720(%rsp), %eax
	movq	%rsi, 128(%rsp)
	vmovups	%ymm0, 1136(%rsp)
	vmovdqu	752(%rsp), %ymm0
	orl	%edx, %ecx
	movq	%rax, 80(%rsp)
	shlq	$32, %rcx
	orq	%rcx, %rbp
	movq	728(%rsp), %rcx
	vmovdqu	%ymm0, 1152(%rsp)
	movq	%rcx, 48(%rsp)
	jmp	.LBB365_917
.LBB365_882:
	movb	$-1, %bpl
	xorl	%r12d, %r12d
	jmp	.LBB365_798
.LBB365_883:
	vmovups	368(%rsp), %ymm0
	movq	328(%rsp), %rax
	movq	336(%rsp), %rcx
	addq	$40, %rbp
	movq	344(%rsp), %rsi
	movzbl	359(%rsp), %edx
	movzwl	357(%rsp), %ebx
	movb	$1, %r15b
	movq	%rbp, 232(%rsp)
	movl	353(%rsp), %ebp
	movq	%rax, 64(%rsp)
	movzbl	352(%rsp), %eax
	movq	%rcx, 40(%rsp)
	movq	360(%rsp), %rcx
	movq	%rsi, 128(%rsp)
	movl	%edx, 56(%rsp)
	vmovups	%ymm0, 1136(%rsp)
	vmovdqu	384(%rsp), %ymm0
	movq	%rax, 80(%rsp)
	movq	%rcx, 48(%rsp)
	vmovdqu	%ymm0, 1152(%rsp)
.Ltmp16082:
	leaq	224(%rsp), %rdi
	movb	$1, %r14b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16083:
	movl	152(%rsp), %r12d
	movl	56(%rsp), %eax
	movb	$1, %r15b
	movb	$1, %r14b
	shll	$16, %eax
	orl	%eax, %ebx
	shlq	$32, %rbx
	orq	%rbx, %rbp
	jmp	.LBB365_932
.LBB365_885:
.Ltmp16182:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.536(%rip), %rcx
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
.Ltmp16183:
	jmp	.LBB365_959
.LBB365_886:
	movq	40(%rsp), %rbx
	movq	%rbx, %rax
	addq	$-1, %rax
	jae	.LBB365_914
	cmpq	%rax, %r12
	jae	.LBB365_914
	movq	160(%rsp), %rax
	cmpq	112(%rsp), %rax
	je	.LBB365_913
	subq	%r12, %rbx
	addq	$88, %rax
	leaq	688(%rsp), %r14
	addq	$-2, %rbx
.LBB365_890:
	movq	%rax, %rbp
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB365_913
	movq	-88(%rbp), %rsi
	movq	%rax, 688(%rsp)
	leaq	696(%rsp), %rcx
	movq	%rbp, %r15
	movq	-8(%rbp), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rbp), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16039:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16040:
	subq	$1, %rbx
	jb	.LBB365_912
	leaq	88(%r15), %rax
	movq	%r15, %rbp
	cmpq	112(%rsp), %r15
	jne	.LBB365_890
	jmp	.LBB365_913
.LBB365_894:
	leaq	-1(%rbx), %rax
	cmpq	%rax, %r12
	jae	.LBB365_914
	movq	160(%rsp), %rax
	cmpq	112(%rsp), %rax
	je	.LBB365_913
	subq	%r12, %rbx
	addq	$88, %rax
	leaq	688(%rsp), %r14
	addq	$-2, %rbx
.LBB365_897:
	movq	%rax, %rbp
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB365_913
	movq	-88(%rbp), %rsi
	movq	%rax, 688(%rsp)
	leaq	696(%rsp), %rcx
	movq	%rbp, %r15
	movq	-8(%rbp), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rbp), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16033:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16034:
	subq	$1, %rbx
	jb	.LBB365_912
	leaq	88(%r15), %rax
	movq	%r15, %rbp
	cmpq	112(%rsp), %r15
	jne	.LBB365_897
	jmp	.LBB365_913
.LBB365_729:
	movq	200(%rsp), %rax
	movq	208(%rsp), %rdx
	movq	216(%rsp), %rcx
	jmp	.LBB365_915
.LBB365_901:
	cmpq	$21, %r15
	jae	.LBB365_960
	movq	%r15, %rsi
	callq	core::slice::sort::shared::smallsort::insertion_sort_shift_left::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>
	jmp	.LBB365_184
.LBB365_903:
.Ltmp15870:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.529(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	vzeroupper
	callq	*%r8
.Ltmp15871:
	jmp	.LBB365_959
.LBB365_904:
.Ltmp15860:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.530(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	vzeroupper
	callq	*%rcx
.Ltmp15861:
	jmp	.LBB365_959
.LBB365_905:
	movq	200(%rsp), %rax
	movq	208(%rsp), %rdx
	movq	216(%rsp), %rcx
	movq	$-1, %r13
	movq	$0, 80(%rsp)
	xorl	%r15d, %r15d
	movq	%rax, 64(%rsp)
	movq	%rdx, 40(%rsp)
	movq	%rcx, 128(%rsp)
	jmp	.LBB365_916
.LBB365_906:
.Ltmp15807:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15808:
	jmp	.LBB365_959
.LBB365_912:
	movq	%r15, %rbp
.LBB365_913:
	movq	%rbp, 592(%rsp)
.LBB365_914:
	movq	200(%rsp), %rax
	movq	208(%rsp), %rdx
	movq	216(%rsp), %rcx
	movl	152(%rsp), %r12d
.LBB365_915:
	movq	$-1, %r13
	xorl	%r15d, %r15d
	movq	%rax, 64(%rsp)
	movb	$1, %al
	movq	%rdx, 40(%rsp)
	movq	%rcx, 128(%rsp)
	movq	%rax, 80(%rsp)
.LBB365_916:
.LBB365_917:
	movq	296(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB365_919
	movq	1328(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB365_919:
.Ltmp16048:
	leaq	584(%rsp), %rdi
	movl	%r15d, 56(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16049:
	movq	624(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_922
	movq	168(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB365_922:
	movq	408(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_925
	lock		decq	(%rax)
	jne	.LBB365_925
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB365_925:
	movq	440(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_928
	lock		decq	(%rax)
	jne	.LBB365_928
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB365_928:
	xorl	%r15d, %r15d
.Ltmp16051:
	leaq	1104(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16052:
	movq	104(%rsp), %rax
	lock		decq	(%rax)
	movl	56(%rsp), %r15d
	jne	.LBB365_931
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp16053:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1392(%rsp), %rdi
	callq	*%rax
.Ltmp16054:
.LBB365_931:
	xorl	%r14d, %r14d
.LBB365_932:
	movq	1336(%rsp), %rax
	movq	96(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB365_934
.Ltmp16084:
	leaq	656(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16085:
.LBB365_934:
	testb	%r15b, %r15b
	je	.LBB365_942
	movq	208(%rsp), %rax
	movq	216(%rsp), %rbx
	movq	%rax, 56(%rsp)
	testq	%rbx, %rbx
	je	.LBB365_940
	movq	56(%rsp), %rax
	leaq	8(%rax), %r15
	jmp	.LBB365_938
.LBB365_937:
	addq	$40, %r15
	decq	%rbx
	je	.LBB365_940
.LBB365_938:
	movq	-8(%r15), %rax
	cmpq	$6, %rax
	jb	.LBB365_937
	movq	(%r15), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB365_937
.LBB365_940:
	movq	200(%rsp), %rax
	movq	96(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB365_942
	movq	56(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB365_942:
	testb	%r14b, %r14b
	jne	.LBB365_458
	jmp	.LBB365_464
.LBB365_907:
.Ltmp15804:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.786(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15805:
	jmp	.LBB365_959
.LBB365_908:
.Ltmp15881:
	leaq	1360(%rsp), %rdi
	movl	$16, %ecx
	movl	$224, %r8d
	xorl	%esi, %esi
	movq	%r15, %rdx
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15882:
	movq	1360(%rsp), %rax
	movq	1376(%rsp), %r13
	subq	%r13, %rax
	cmpq	%r15, %rax
	jae	.LBB365_331
.LBB365_910:
.Ltmp15895:
	movq	core::panicking::panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1066(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1068(%rip), %rdx
	movl	$47, %esi
	callq	*%rax
.Ltmp15896:
	jmp	.LBB365_959
.LBB365_911:
	leaq	200(%rsp), %rax
	movq	%rax, 864(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 872(%rsp)
	movq	%rbx, 880(%rsp)
	movq	%rax, 888(%rsp)
.Ltmp15887:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.619(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.621(%rip), %rdx
	leaq	864(%rsp), %rsi
	callq	*%rax
.Ltmp15888:
	jmp	.LBB365_959
.LBB365_943:
.Ltmp16058:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp16059:
	jmp	.LBB365_959
.LBB365_944:
.Ltmp16132:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.532(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	callq	*%r8
.Ltmp16133:
	jmp	.LBB365_959
.LBB365_945:
.Ltmp16122:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.533(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	callq	*%rcx
.Ltmp16123:
	jmp	.LBB365_959
.LBB365_946:
	leaq	1856(%rsp), %rax
	leaq	224(%rsp), %rcx
	movq	%rdi, 1856(%rsp)
	movq	%rdx, 224(%rsp)
	movq	%rax, 688(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 696(%rsp)
	movq	%rcx, 704(%rsp)
	movq	%rax, 712(%rsp)
.Ltmp16060:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.2158(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.289(%rip), %rdx
	leaq	688(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp16061:
	jmp	.LBB365_959
.LBB365_947:
.Ltmp16153:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.535(%rip), %rcx
	movq	%r14, %rdi
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp16154:
	jmp	.LBB365_959
.LBB365_948:
.Ltmp15815:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$8, %esi
	callq	*%rax
.Ltmp15816:
	jmp	.LBB365_959
.LBB365_949:
.Ltmp15921:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$16, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15922:
	jmp	.LBB365_959
.LBB365_950:
.Ltmp15865:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.531(%rip), %rdx
	callq	*%rax
.Ltmp15866:
	jmp	.LBB365_959
.LBB365_951:
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp15997:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %r15b
	movq	%r13, %rdi
	movq	%r14, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15998:
	jmp	.LBB365_599
.LBB365_952:
	leaq	1000(%rax), %rdi
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp15999:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movb	$1, %r15b
	movq	%r14, %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16000:
	jmp	.LBB365_600
.LBB365_953:
.Ltmp16197:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16198:
	jmp	.LBB365_959
.LBB365_954:
.Ltmp15785:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$1, %edi
	movl	$51, %esi
	callq	*%rax
.Ltmp15786:
	jmp	.LBB365_959
.LBB365_955:
	addq	$16, %rdi
.Ltmp16106:
	leaq	320(%rsp), %rsi
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.12908414067662811932)
.Ltmp16107:
	jmp	.LBB365_796
.LBB365_956:
.Ltmp16127:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.534(%rip), %rdx
	callq	*%rax
.Ltmp16128:
	jmp	.LBB365_959
.LBB365_957:
.Ltmp15799:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15800:
	jmp	.LBB365_959
.LBB365_958:
.Ltmp15873:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp15874:
.LBB365_959:
	ud2
.LBB365_960:
.Ltmp15827:
	movq	core::slice::sort::unstable::ipnsort::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%r15, %rsi
	callq	*%rax
.Ltmp15828:
	jmp	.LBB365_184
.LBB365_961:
.Ltmp16108:
	leaq	688(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB365_1120
.LBB365_962:
.Ltmp15856:
	movq	320(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$6, %r15
	jae	.LBB365_1040
	jmp	.LBB365_1135
.LBB365_963:
.Ltmp16035:
	jmp	.LBB365_965
.LBB365_964:
.Ltmp16041:
.LBB365_965:
	movq	%rax, %rbx
	movq	%r15, 592(%rsp)
	jmp	.LBB365_1100
.LBB365_966:
.Ltmp16012:
	jmp	.LBB365_1099
.LBB365_967:
.Ltmp16067:
	movq	%rax, %rbx
	movb	$1, %al
	movl	%eax, 56(%rsp)
	jmp	.LBB365_1103
.LBB365_968:
.Ltmp16019:
	jmp	.LBB365_1099
.LBB365_969:
.Ltmp16050:
	movq	%rax, %rbx
	jmp	.LBB365_1103
.LBB365_970:
.Ltmp15803:
	jmp	.LBB365_1126
.LBB365_971:
.Ltmp16003:
	jmp	.LBB365_1099
.LBB365_972:
.Ltmp16009:
	jmp	.LBB365_1025
.LBB365_973:
.Ltmp16086:
	movq	%rax, %rbx
	movl	%r15d, 56(%rsp)
	jmp	.LBB365_1116
.LBB365_974:
.Ltmp16072:
	movl	%r15d, %r14d
	movq	%rax, %rbx
	jmp	.LBB365_1113
.LBB365_975:
.Ltmp16094:
	cmpq	$0, 1336(%rsp)
	movq	%rax, %rbx
	movl	%r15d, 56(%rsp)
	jne	.LBB365_1115
	jmp	.LBB365_1116
.LBB365_976:
.Ltmp16038:
	jmp	.LBB365_1099
.LBB365_977:
.Ltmp15966:
	movq	%rax, %rbx
	jmp	.LBB365_1010
.LBB365_978:
.Ltmp16022:
	jmp	.LBB365_1025
.LBB365_979:
.Ltmp15996:
	movq	%rax, %rbx
	jmp	.LBB365_1119
.LBB365_1018:
.Ltmp15952:
	movq	%rax, %rbx
	jmp	.LBB365_1019
.LBB365_980:
.Ltmp16131:
	jmp	.LBB365_1084
.LBB365_981:
.Ltmp16081:
	addq	$40, %rbp
	movq	%rax, %rbx
	movq	%rbp, 232(%rsp)
	jmp	.LBB365_994
.LBB365_982:
.Ltmp16141:
	leaq	1424(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bpl
	jmp	.LBB365_984
.LBB365_983:
.Ltmp16144:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
.LBB365_984:
	xorl	%r15d, %r15d
	jmp	.LBB365_1147
.LBB365_985:
.Ltmp16121:
	jmp	.LBB365_1084
.LBB365_986:
.Ltmp15969:
	jmp	.LBB365_999
.LBB365_987:
.Ltmp15991:
	movq	%rax, %rbx
.Ltmp15992:
	leaq	1400(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15993:
	jmp	.LBB365_1119
.LBB365_988:
.Ltmp15980:
	movb	%bpl, 48(%rsp)
	movq	%rax, %rbx
	jmp	.LBB365_1015
.LBB365_989:
.Ltmp15947:
	movq	%rax, %rbx
.Ltmp15948:
	leaq	1400(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15949:
	jmp	.LBB365_1019
.LBB365_990:
.Ltmp15911:
	movq	%rax, %rbx
.Ltmp15912:
	leaq	1104(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15913:
	jmp	.LBB365_1141
.LBB365_991:
.Ltmp15914:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_992:
.Ltmp16089:
	addq	$40, %rbp
	cmpq	$6, 80(%rsp)
	movq	%rax, %rbx
	movq	%rbp, 232(%rsp)
	jb	.LBB365_994
	movq	80(%rsp), %rax
	movq	40(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
.LBB365_994:
	movb	$1, %r14b
.Ltmp16090:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16091:
	jmp	.LBB365_1117
.LBB365_995:
.Ltmp16032:
	jmp	.LBB365_1099
.LBB365_996:
.Ltmp16044:
	jmp	.LBB365_1025
.LBB365_997:
.Ltmp15850:
	jmp	.LBB365_1054
.LBB365_998:
.Ltmp16111:
.LBB365_999:
	movq	%rax, %rbx
	jmp	.LBB365_1120
.LBB365_1000:
.Ltmp15933:
	jmp	.LBB365_1140
.LBB365_1001:
.Ltmp16047:
	addq	$40, %r14
	movq	%rax, %rbx
	movq	%r14, 664(%rsp)
	jmp	.LBB365_1100
.LBB365_1002:
.Ltmp16116:
	jmp	.LBB365_1028
.LBB365_1003:
.Ltmp15955:
	movq	%rax, %rbx
	movq	%rbp, 232(%rsp)
	jmp	.LBB365_1009
.LBB365_1004:
.Ltmp15812:
	jmp	.LBB365_1126
.LBB365_1005:
.Ltmp16057:
	addq	$40, %r14
	cmpq	$6, 80(%rsp)
	movq	%rax, %rbx
	movq	%r14, 664(%rsp)
	jb	.LBB365_1100
	movq	80(%rsp), %rax
	movq	40(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB365_1100
.LBB365_1007:
.Ltmp15960:
	movq	%rax, %rbx
	movq	%rbp, 232(%rsp)
	cmpq	$6, %r14
	jb	.LBB365_1009
	movq	128(%rsp), %rdi
	leaq	-8(,%r14,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB365_1009:
.Ltmp15961:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15962:
.LBB365_1010:
	leaq	1136(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB365_1016
.LBB365_1011:
.Ltmp15963:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1012:
.Ltmp15942:
	movq	%rax, %rbx
.Ltmp15943:
	leaq	1400(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15944:
	jmp	.LBB365_1143
.LBB365_1013:
.Ltmp15972:
	movq	%rax, %rbx
.Ltmp15973:
	leaq	688(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp15974:
.Ltmp15976:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15977:
.LBB365_1015:
.Ltmp15981:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp15982:
.LBB365_1016:
	cmpb	$0, 48(%rsp)
	je	.LBB365_1019
.Ltmp15986:
	leaq	1296(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp15987:
	jmp	.LBB365_1120
.LBB365_1019:
.Ltmp15984:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp15985:
	jmp	.LBB365_1120
.LBB365_1020:
.Ltmp15975:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1021:
.Ltmp15983:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1022:
.Ltmp15988:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1023:
.Ltmp15869:
	movq	%rax, %rbx
	cmpq	$5, %r15
	ja	.LBB365_1134
	jmp	.LBB365_1135
.LBB365_1024:
.Ltmp16027:
.LBB365_1025:
	addq	$88, %rbp
	movq	%rax, %rbx
	movq	%rbp, 592(%rsp)
	jmp	.LBB365_1100
.LBB365_1026:
.Ltmp16176:
	movq	%rax, %rbx
	jmp	.LBB365_1167
.LBB365_1027:
.Ltmp15939:
.LBB365_1028:
	movq	%rax, %rbx
	jmp	.LBB365_1143
.LBB365_1029:
.Ltmp15823:
	jmp	.LBB365_1086
.LBB365_1030:
.Ltmp15917:
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB365_1069
	negq	%r12
	jmp	.LBB365_1033
.LBB365_1032:
	addq	$240, %r14
	decq	%r12
	je	.LBB365_1069
.LBB365_1033:
	cmpl	$2, (%r14)
	je	.LBB365_1032
.Ltmp15918:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
.Ltmp15919:
	jmp	.LBB365_1032
.LBB365_1035:
.Ltmp15920:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1036:
.Ltmp16160:
	movq	%rax, %rbx
	jmp	.LBB365_1146
.LBB365_1037:
.Ltmp16006:
	jmp	.LBB365_1099
.LBB365_1038:
.Ltmp16126:
	movq	1528(%rsp), %r13
	jmp	.LBB365_1095
.LBB365_1039:
.Ltmp15859:
	movq	864(%rsp), %r15
	movq	%rax, %rbx
	leaq	872(%rsp), %rax
	movq	%rax, 152(%rsp)
	cmpq	$5, %r15
	jbe	.LBB365_1135
.LBB365_1040:
	movq	152(%rsp), %rax
	movq	(%rax), %r14
	jmp	.LBB365_1134
.LBB365_1041:
.Ltmp15853:
	jmp	.LBB365_1054
.LBB365_1042:
.Ltmp15826:
	leaq	320(%rsp), %rdi
	movq	%r12, 896(%rsp)
	movq	%r15, 888(%rsp)
	movw	%bp, 912(%rsp)
	movq	%rax, %rbx
	movq	%r14, 920(%rsp)
	callq	core::ptr::drop_glue::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB365_1154
.LBB365_1043:
.Ltmp16165:
	movq	%rax, %rbx
	jmp	.LBB365_1136
.LBB365_1044:
.Ltmp16102:
	movq	%rax, %rbx
	testq	%r15, %r15
	je	.LBB365_1048
	negq	%r15
	addq	$160, %r14
.LBB365_1046:
.Ltmp16103:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16104:
	addq	$160, %r14
	decq	%r15
	jne	.LBB365_1046
.LBB365_1048:
	cmpq	$0, 1224(%rsp)
	je	.LBB365_1120
	movq	1224(%rsp), %rax
	movq	552(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB365_1120
.LBB365_1050:
.Ltmp16105:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1051:
.Ltmp15787:
	jmp	.LBB365_1088
.LBB365_1052:
.Ltmp15845:
	jmp	.LBB365_1054
.LBB365_1053:
.Ltmp15838:
.LBB365_1054:
	movq	%rax, %rbx
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB365_1148
.LBB365_1055:
.Ltmp16199:
	movq	%rax, %rbx
.Ltmp16200:
	leaq	336(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp16201:
	jmp	.LBB365_1179
.LBB365_1056:
.Ltmp16202:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1057:
.Ltmp16147:
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB365_1061
	negq	%r12
	addq	$24, %r15
.LBB365_1059:
.Ltmp16148:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16149:
	addq	$24, %r15
	decq	%r12
	jne	.LBB365_1059
.LBB365_1061:
	movq	1240(%rsp), %rax
	xorl	%ebp, %ebp
	testq	%rax, %rax
	je	.LBB365_1063
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	xorl	%ebp, %ebp
.LBB365_1063:
	xorl	%r15d, %r15d
	jmp	.LBB365_1148
.LBB365_1064:
.Ltmp16150:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1065:
.Ltmp16193:
	movq	%rax, %rbx
	jmp	.LBB365_1176
.LBB365_1066:
.Ltmp16179:
	movq	%rax, %rbx
	jmp	.LBB365_1169
.LBB365_1067:
.Ltmp15864:
	movq	688(%rsp), %r15
	jmp	.LBB365_1132
.LBB365_1068:
.Ltmp15923:
	movq	%rax, %rbx
.LBB365_1069:
.Ltmp15924:
	leaq	1136(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15925:
.Ltmp15926:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp15927:
	jmp	.LBB365_1141
.LBB365_1071:
.Ltmp15928:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1072:
.Ltmp16170:
	movl	%ebx, %r13d
	movq	%rax, %rbx
	testq	%r12, %r12
	je	.LBB365_1076
	negq	%r12
	addq	$24, %r15
.LBB365_1074:
.Ltmp16171:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16172:
	addq	$24, %r15
	decq	%r12
	jne	.LBB365_1074
.LBB365_1076:
	movq	1240(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	jne	.LBB365_1078
	movl	%r13d, %r15d
	jmp	.LBB365_1148
.LBB365_1078:
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	movl	%r13d, %r15d
	jmp	.LBB365_1148
.LBB365_1079:
.Ltmp16173:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1080:
.Ltmp15817:
	cmpq	$6, 64(%rsp)
	movq	%rax, %rbx
	jb	.LBB365_1130
	movq	64(%rsp), %rax
	movq	56(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	jmp	.LBB365_1129
.LBB365_1082:
.Ltmp15798:
	movq	192(%rsp), %rdi
	movq	632(%rsp), %rsi
	movl	$8, %edx
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
	jmp	.LBB365_1138
.LBB365_1083:
.Ltmp16155:
.LBB365_1084:
	movq	%rax, %rbx
	jmp	.LBB365_1097
.LBB365_1085:
.Ltmp15820:
.LBB365_1086:
	movq	%rax, %rbx
	jmp	.LBB365_1130
.LBB365_1087:
.Ltmp15784:
.LBB365_1088:
	movq	%rax, %rbx
.Ltmp15788:
	leaq	1600(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15789:
	jmp	.LBB365_1179
.LBB365_1089:
.Ltmp15889:
	movq	656(%rsp), %rdi
	movq	%rax, %rbx
.Ltmp15890:
	movq	%r14, %rsi
	callq	core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15891:
	jmp	.LBB365_1092
.LBB365_1090:
.Ltmp15892:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1091:
.Ltmp15897:
	movq	%rax, %rbx
.LBB365_1092:
.Ltmp15898:
	leaq	1360(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp15899:
	jmp	.LBB365_1143
.LBB365_1093:
.Ltmp15900:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1094:
.Ltmp16134:
.LBB365_1095:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB365_1097
	movq	1536(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB365_1097:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB365_1143
.LBB365_1098:
.Ltmp16062:
.LBB365_1099:
	movq	%rax, %rbx
.LBB365_1100:
	cmpq	$0, 296(%rsp)
	je	.LBB365_1102
	movq	296(%rsp), %rsi
	movq	1328(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB365_1102:
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp16063:
	leaq	584(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16064:
.LBB365_1103:
	cmpq	$0, 624(%rsp)
	je	.LBB365_1105
	movq	624(%rsp), %rax
	movq	168(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB365_1105:
	movq	408(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_1108
	lock		decq	(%rax)
	jne	.LBB365_1108
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB365_1108:
	movq	440(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_1111
	lock		decq	(%rax)
	jne	.LBB365_1111
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB365_1111:
.Ltmp16068:
	leaq	1104(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16069:
	xorl	%r14d, %r14d
.LBB365_1113:
	movq	104(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB365_1115
	#MEMBARRIER
.Ltmp16073:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1392(%rsp), %rdi
	callq	*%rax
.Ltmp16074:
.LBB365_1115:
.Ltmp16095:
	leaq	656(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16096:
.LBB365_1116:
	cmpb	$0, 56(%rsp)
	je	.LBB365_1118
.LBB365_1117:
	leaq	200(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB365_1118:
	testb	%r14b, %r14b
	je	.LBB365_1120
.LBB365_1119:
.Ltmp16097:
	leaq	1360(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16098:
.LBB365_1120:
.Ltmp16112:
	leaq	1568(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16113:
	jmp	.LBB365_1143
.LBB365_1121:
.Ltmp16099:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1122:
.Ltmp15806:
	movq	%rax, %rbx
	movq	%rbp, (%r14)
	jmp	.LBB365_1127
.LBB365_1123:
.Ltmp15778:
	movq	%rax, %rbx
.Ltmp15779:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp15780:
	jmp	.LBB365_1179
.LBB365_1124:
.Ltmp15781:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB365_1125:
.Ltmp15809:
.LBB365_1126:
	movq	%rax, %rbx
.LBB365_1127:
	movq	320(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB365_1130
	movq	328(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB365_1129:
	callq	__rustc::__rust_dealloc
.LBB365_1130:
	leaq	1664(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>
	jmp	.LBB365_1154
.LBB365_1131:
.Ltmp15872:
.LBB365_1132:
	movq	%rax, %rbx
	cmpq	$6, %r15
	jb	.LBB365_1135
	movq	696(%rsp), %r14
.LBB365_1134:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB365_1135:
	leaq	1888(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB365_1136:
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB365_1147
.LBB365_1137:
.Ltmp16184:
	movq	%rax, %rbx
.LBB365_1138:
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB365_1165
.LBB365_1139:
.Ltmp15936:
.LBB365_1140:
	movq	%rax, %rbx
.LBB365_1141:
	movq	656(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB365_1143
	movq	664(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB365_1143:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_1146
	lock		decq	(%rax)
	jne	.LBB365_1146
	#MEMBARRIER
.Ltmp16156:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	callq	*%rax
.Ltmp16157:
.LBB365_1146:
	movb	$1, %bpl
.Ltmp16161:
	leaq	1888(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16162:
	movb	$1, %r15b
.LBB365_1147:
.Ltmp16166:
	leaq	1240(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp16167:
.LBB365_1148:
	leaq	1184(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB365_1155
.LBB365_1149:
.Ltmp15831:
	movq	%rax, %rbx
	cmpq	$6, %r12
	jb	.LBB365_1151
	movq	128(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB365_1151:
	testq	%r14, %r14
	je	.LBB365_1153
	movq	64(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	movq	%r14, %rsi
	callq	__rustc::__rust_dealloc
.LBB365_1153:
	leaq	688(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
.LBB365_1154:
	movb	$1, %r15b
	movb	$1, %bpl
.LBB365_1155:
	cmpq	$0, 640(%rsp)
	je	.LBB365_1165
	movq	632(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	cmpq	%rdi, %rax
	movq	%rdi, %rsi
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB365_1158
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB365_1158:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB365_1164
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB365_1158
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB365_1161:
	cmpq	%rax, %rcx
	jge	.LBB365_1163
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB365_1161
.LBB365_1163:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB365_1164:
	movq	free@GOTPCREL(%rip), %rax
	movq	192(%rsp), %rdi
	callq	*%rax
.LBB365_1165:
	movq	176(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB365_1167
	#MEMBARRIER
.Ltmp16185:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	callq	*%rax
.Ltmp16186:
.LBB365_1167:
.Ltmp16187:
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16188:
	testb	%bpl, %bpl
	je	.LBB365_1176
.LBB365_1169:
	movq	1768(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB365_1170
	movq	1776(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	movq	1696(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB365_1173
.LBB365_1171:
	movq	1792(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB365_1174
	jmp	.LBB365_1176
.LBB365_1170:
	movq	1696(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB365_1171
.LBB365_1173:
	movq	1704(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	movq	1792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB365_1176
.LBB365_1174:
	lock		decq	(%rax)
	jne	.LBB365_1176
	leaq	1792(%rsp), %rdi
	#MEMBARRIER
.Ltmp16189:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16190:
.LBB365_1176:
	testb	%r15b, %r15b
	je	.LBB365_1179
	movq	304(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB365_1179
	#MEMBARRIER
.Ltmp16194:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1464(%rsp), %rdi
	callq	*%rax
.Ltmp16195:
.LBB365_1179:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB365_1180:
.Ltmp16196:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end365:
purrdf_sparql_eval::modifier::eval_dedup_yielding::<purrdf_core::ir::dataset::RdfDataset, false, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>:
.Lfunc_begin368:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception275
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
	subq	$104, %rsp
	.cfi_def_cfa_offset 160
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	%rdi, %rbx
	movq	$0, 64(%rsp)
	vmovdqu	%ymm0, 32(%rsp)
.Ltmp16346:
	movq	purrdf_sparql_eval::eval::yield_transform::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>, purrdf_sparql_eval::modifier::eval_dedup_yielding<purrdf_core::ir::dataset::RdfDataset, false, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%r8, (%rsp)
	leaq	64(%rsp), %r8
	leaq	32(%rsp), %r9
	vzeroupper
	callq	*%rax
.Ltmp16347:
	movq	64(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB368_11
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	72(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB368_4
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB368_4:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB368_10
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB368_4
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB368_7:
	cmpq	%rax, %rsi
	jge	.LBB368_9
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB368_7
.LBB368_9:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB368_10:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB368_11:
	movq	40(%rsp), %rax
	testq	%rax, %rax
	je	.LBB368_39
	movq	56(%rsp), %r15
	movq	%rax, 16(%rsp)
	movq	%rbx, 24(%rsp)
	testq	%r15, %r15
	je	.LBB368_28
	movq	32(%rsp), %r12
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbx
	movabsq	$9223372036854775807, %rbp
	vpcmpltb	(%r12), %xmm0, %k0
	leaq	16(%r12), %r13
	kmovd	%k0, %r14d
	jmp	.LBB368_14
	.p2align	4
.LBB368_25:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB368_26:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB368_27:
	blsrl	%r14d, %r14d
	decq	%r15
	je	.LBB368_28
.LBB368_14:
	testw	%r14w, %r14w
	jne	.LBB368_17
	.p2align	4
.LBB368_15:
	vpcmpltb	(%r13), %xmm0, %k0
	addq	$-640, %r12
	addq	$16, %r13
	kortestw	%k0, %k0
	je	.LBB368_15
	kmovd	%k0, %r14d
.LBB368_17:
	xorl	%eax, %eax
	tzcntl	%r14d, %eax
	negq	%rax
	leaq	(%rax,%rax,4), %rcx
	movq	-40(%r12,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB368_27
	leaq	(%r12,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rbp, %rcx
	movq	-32(%rdx), %rdi
	cmovaeq	%rbp, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbp, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB368_20
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB368_20:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB368_26
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB368_20
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbp, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbx), %rax
	.p2align	4
.LBB368_23:
	cmpq	%rax, %rdx
	jge	.LBB368_25
	lock		cmpxchgq	%rdx, (%rbx)
	jne	.LBB368_23
	jmp	.LBB368_25
.LBB368_28:
	movq	16(%rsp), %r8
	movq	24(%rsp), %rbx
	leaq	(,%r8,8), %rax
	leaq	(%rax,%rax,4), %rax
	andq	$-16, %rax
	addq	%rax, %r8
	addq	$65, %r8
	je	.LBB368_39
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	movq	32(%rsp), %rdi
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rcx
	cmpq	%r8, %rdx
	setns	%sil
	addq	%rcx, %rsi
	subq	%r8, %rdx
	cmovoq	%rsi, %rdx
	subq	%rax, %rdi
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB368_31
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB368_31:
	addq	$-48, %rdi
	.p2align	4
.LBB368_32:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB368_38
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB368_32
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r8, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r8, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r8, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB368_35:
	cmpq	%rax, %rdx
	jge	.LBB368_37
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB368_35
.LBB368_37:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB368_38:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB368_39:
	movq	%rbx, %rax
	addq	$104, %rsp
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
	retq
.LBB368_40:
	.cfi_def_cfa_offset 160
.Ltmp16348:
	movq	%rax, %rbx
	movq	64(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB368_42
	movq	72(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB368_42:
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_hash::fixed::FixedState>>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end368:
purrdf_sparql_eval::modifier::eval_dedup_yielding::<purrdf_core::ir::dataset::RdfDataset, true, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>:
.Lfunc_begin369:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception276
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
	subq	$104, %rsp
	.cfi_def_cfa_offset 160
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovdqu	anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932(%rip), %ymm0
	movq	%rdi, %rbx
	movq	$0, 64(%rsp)
	vmovdqu	%ymm0, 32(%rsp)
.Ltmp16349:
	movq	purrdf_sparql_eval::eval::yield_transform::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>, purrdf_sparql_eval::modifier::eval_dedup_yielding<purrdf_core::ir::dataset::RdfDataset, true, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%r8, (%rsp)
	leaq	64(%rsp), %r8
	leaq	32(%rsp), %r9
	vzeroupper
	callq	*%rax
.Ltmp16350:
	movq	64(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB369_11
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	72(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB369_4
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB369_4:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB369_10
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB369_4
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB369_7:
	cmpq	%rax, %rsi
	jge	.LBB369_9
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB369_7
.LBB369_9:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB369_10:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB369_11:
	movq	40(%rsp), %rax
	testq	%rax, %rax
	je	.LBB369_39
	movq	56(%rsp), %r15
	movq	%rax, 16(%rsp)
	movq	%rbx, 24(%rsp)
	testq	%r15, %r15
	je	.LBB369_28
	movq	32(%rsp), %r12
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbx
	movabsq	$9223372036854775807, %rbp
	vpcmpltb	(%r12), %xmm0, %k0
	leaq	16(%r12), %r13
	kmovd	%k0, %r14d
	jmp	.LBB369_14
	.p2align	4
.LBB369_25:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB369_26:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB369_27:
	blsrl	%r14d, %r14d
	decq	%r15
	je	.LBB369_28
.LBB369_14:
	testw	%r14w, %r14w
	jne	.LBB369_17
	.p2align	4
.LBB369_15:
	vpcmpltb	(%r13), %xmm0, %k0
	addq	$-640, %r12
	addq	$16, %r13
	kortestw	%k0, %k0
	je	.LBB369_15
	kmovd	%k0, %r14d
.LBB369_17:
	xorl	%eax, %eax
	tzcntl	%r14d, %eax
	negq	%rax
	leaq	(%rax,%rax,4), %rcx
	movq	-40(%r12,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB369_27
	leaq	(%r12,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rbp, %rcx
	movq	-32(%rdx), %rdi
	cmovaeq	%rbp, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbp, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB369_20
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB369_20:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB369_26
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB369_20
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rbp, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbx), %rax
	.p2align	4
.LBB369_23:
	cmpq	%rax, %rdx
	jge	.LBB369_25
	lock		cmpxchgq	%rdx, (%rbx)
	jne	.LBB369_23
	jmp	.LBB369_25
.LBB369_28:
	movq	16(%rsp), %r8
	movq	24(%rsp), %rbx
	leaq	(,%r8,8), %rax
	leaq	(%rax,%rax,4), %rax
	andq	$-16, %rax
	addq	%rax, %r8
	addq	$65, %r8
	je	.LBB369_39
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	movq	32(%rsp), %rdi
	xorl	%esi, %esi
	movabsq	$9223372036854775807, %rcx
	cmpq	%r8, %rdx
	setns	%sil
	addq	%rcx, %rsi
	subq	%r8, %rdx
	cmovoq	%rsi, %rdx
	subq	%rax, %rdi
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB369_31
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB369_31:
	addq	$-48, %rdi
	.p2align	4
.LBB369_32:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB369_38
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB369_32
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r8, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r8, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r8, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB369_35:
	cmpq	%rax, %rdx
	jge	.LBB369_37
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB369_35
.LBB369_37:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB369_38:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB369_39:
	movq	%rbx, %rax
	addq	$104, %rsp
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
	retq
.LBB369_40:
	.cfi_def_cfa_offset 160
.Ltmp16351:
	movq	%rax, %rbx
	movq	64(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB369_42
	movq	72(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB369_42:
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::set::HashSet<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_hash::fixed::FixedState>>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end369:
purrdf_sparql_eval::expr::eval_extend_sequence::<purrdf_core::ir::dataset::RdfDataset>::{closure#0}:
.Lfunc_begin1287:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception860
	pushq	%rbp
	.cfi_def_cfa_offset 16
	pushq	%r15
	.cfi_def_cfa_offset 24
	pushq	%r14
	.cfi_def_cfa_offset 32
	pushq	%rbx
	.cfi_def_cfa_offset 40
	subq	$2968, %rsp
	.cfi_def_cfa_offset 3008
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rsi), %rcx
	movq	%rsi, %r14
	movq	8(%rsi), %rsi
	movq	%rdi, %rbx
	leaq	464(%rsp), %rdi
	movq	(%rsi), %rax
	movq	%rcx, %rdx
	testq	%rax, %rax
	cmoveq	%rax, %rsi
	callq	<core::option::Option<&alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>>::map_or_else::<purrdf_sparql_eval::eval::EvalCtx, <purrdf_sparql_eval::eval::EvalCtx>::fork_for_loop_worker::{closure#0}, <purrdf_sparql_eval::eval::EvalCtx>::fork_for_loop_worker::{closure#1}>
	movq	16(%r14), %rsi
	vmovups	160(%rsi), %ymm0
	vmovsd	192(%rsi), %xmm1
	movzbl	194(%rsi), %ebp
	vmovups	%ymm0, 16(%rsp)
	vmovaps	%xmm1, (%rsp)
.Ltmp24061:
	leaq	48(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger as core::clone::Clone>::clone
.Ltmp24062:
	vmovups	16(%rsp), %ymm0
	vmovaps	(%rsp), %xmm1
	vmovups	%ymm0, 208(%rsp)
	vmovss	%xmm1, 240(%rsp)
	cmpb	$2, %bpl
	jne	.LBB1287_4
	cmpq	$0, 1080(%rsp)
	je	.LBB1287_4
.Ltmp24064:
	leaq	48(%rsp), %rdi
	leaq	464(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::defer::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp24065:
.LBB1287_4:
	movq	memcpy@GOTPCREL(%rip), %r15
	leaq	1712(%rsp), %rdi
	leaq	464(%rsp), %rsi
	movl	$1248, %edx
	vzeroupper
	callq	*%r15
	movq	24(%r14), %rsi
.Ltmp24071:
	leaq	248(%rsp), %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::fresh
.Ltmp24072:
	leaq	464(%rsp), %rsi
	movl	$1248, %edx
	movq	%rbx, %rdi
	callq	*%r15
	vmovups	48(%rsp), %zmm0
	vmovups	112(%rsp), %zmm1
	vmovups	176(%rsp), %zmm2
	vmovups	376(%rsp), %zmm4
	vmovups	400(%rsp), %zmm3
	movq	240(%rsp), %rax
	movq	%rax, 1440(%rbx)
	vmovups	%zmm0, 1248(%rbx)
	vmovups	%zmm1, 1312(%rbx)
	vmovups	248(%rsp), %zmm0
	vmovups	312(%rsp), %zmm1
	vmovups	%zmm2, 1376(%rbx)
	vmovups	%zmm3, 1600(%rbx)
	vmovups	%zmm4, 1576(%rbx)
	vmovups	%zmm1, 1512(%rbx)
	vmovups	%zmm0, 1448(%rbx)
	addq	$2968, %rsp
	.cfi_def_cfa_offset 40
	popq	%rbx
	.cfi_def_cfa_offset 32
	popq	%r14
	.cfi_def_cfa_offset 24
	popq	%r15
	.cfi_def_cfa_offset 16
	popq	%rbp
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.LBB1287_9:
	.cfi_def_cfa_offset 3008
.Ltmp24066:
	movq	%rax, %rbx
.Ltmp24067:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp24068:
	jmp	.LBB1287_11
.LBB1287_7:
.Ltmp24073:
	movq	%rax, %rbx
.Ltmp24074:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp24075:
.Ltmp24076:
	leaq	1712(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp24077:
	jmp	.LBB1287_12
.LBB1287_10:
.Ltmp24063:
	movq	%rax, %rbx
.LBB1287_11:
.Ltmp24069:
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp24070:
.LBB1287_12:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1287_6:
.Ltmp24078:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1287:
purrdf_sparql_eval::expr::eval_extend_sequence::<purrdf_core::ir::dataset::RdfDataset>::{closure#1}:
.Lfunc_begin1288:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception861
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
	subq	$216, %rsp
	.cfi_def_cfa_offset 272
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdx, %r14
	movq	%r8, %rdx
	subq	(%rsi), %rdx
	movabsq	$-3689348814741910323, %rax
	movq	%r8, %rbp
	movq	%rcx, %r15
	movq	%rsi, %r13
	movq	%rdi, %rbx
	mulxq	%rax, %rax, %rax
	cmpb	$2, 1442(%r14)
	jne	.LBB1288_6
	cmpb	$0, 1400(%r14)
	jne	.LBB1288_49
	movq	1368(%r14), %rcx
	shrq	$5, %rax
	testq	%rcx, %rcx
	je	.LBB1288_5
	movq	16(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1288_5
	movb	$1, 1400(%r14)
	jmp	.LBB1288_49
.LBB1288_5:
	movq	%rax, 1376(%r14)
.LBB1288_6:
	leaq	1248(%r14), %rsi
	leaq	80(%rsp), %rdi
	movq	%r14, %rdx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
	cmpb	$-1, 80(%rsp)
	je	.LBB1288_7
.LBB1288_49:
	movq	$-1, (%rbx)
.LBB1288_50:
	addq	$216, %rsp
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
	vzeroupper
	retq
.LBB1288_7:
	.cfi_def_cfa_offset 272
	movq	16(%r13), %rax
	movq	(%rax), %rdx
	movq	%rax, 72(%rsp)
	movq	$1, 80(%rsp)
	cmpq	$5, %rdx
	jae	.LBB1288_8
.LBB1288_9:
	vmovups	88(%rsp), %xmm0
	movq	(%rbp), %r12
	movq	112(%rsp), %rax
	movq	80(%rsp), %rdx
	movq	104(%rsp), %rcx
	decq	%r12
	movq	%rax, 32(%rsp)
	movq	%rdx, (%rsp)
	movq	%rcx, 24(%rsp)
	vmovups	%xmm0, 8(%rsp)
	cmpq	$5, %r12
	jb	.LBB1288_10
	movq	16(%rbp), %r12
	movq	8(%rbp), %rbp
	decq	%r12
	jmp	.LBB1288_15
.LBB1288_10:
	addq	$8, %rbp
.LBB1288_15:
	movq	(%rsp), %rax
	movq	16(%rsp), %rsi
	movl	$4, %edx
	leaq	8(%rsp), %rdi
	movq	%rdi, 48(%rsp)
	leaq	-1(%rax), %rcx
	decq	%rsi
	cmpq	$5, %rcx
	cmovbq	%rcx, %rsi
	cmovaeq	%rcx, %rdx
	subq	%rsi, %rdx
	cmpq	%r12, %rdx
	jb	.LBB1288_16
.LBB1288_18:
	movq	%r15, 56(%rsp)
	movq	%rbx, 64(%rsp)
	xorl	%r15d, %r15d
	movq	%rdi, %rcx
	cmpq	$6, %rax
	setae	%al
	jb	.LBB1288_20
	movq	8(%rsp), %rcx
.LBB1288_20:
	movb	%al, %r15b
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	(,%r12,8), %rdx
	movq	%rbp, %rsi
	shll	$4, %r15d
	movq	(%rsp,%r15), %rbx
	leaq	-8(%rcx,%rbx,8), %rdi
	callq	*%rax
	addq	%r12, %rbx
	movq	72(%rsp), %rax
	movq	%rbx, (%rsp,%r15)
	movq	(%rsp), %rcx
	movq	16(%rsp), %rdx
	movq	(%rax), %rax
	leaq	-1(%rcx), %rsi
	leaq	-1(%rdx), %rdi
	cmpq	$5, %rsi
	cmovbq	%rsi, %rdi
	movq	%rax, %rsi
	subq	%rdi, %rsi
	jbe	.LBB1288_21
	movl	$2, 80(%rsp)
	movq	%rsi, 88(%rsp)
.Ltmp24084:
	leaq	80(%rsp), %rsi
	movq	%rsp, %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp24085:
	movq	64(%rsp), %rbx
	movq	56(%rsp), %r12
	jmp	.LBB1288_24
.LBB1288_21:
	movq	64(%rsp), %rbx
	movq	56(%rsp), %r12
	cmpq	$6, %rcx
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB1288_24
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, (%rsp,%rdx)
.LBB1288_24:
	movq	(%rsp), %rcx
	leaq	1448(%r14), %rsi
	leaq	8(%rsp), %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB1288_26
	movq	16(%rsp), %rcx
	movq	8(%rsp), %rdx
	decq	%rcx
.LBB1288_26:
	movq	24(%r13), %rax
	movq	(%rax), %r8
	addq	$16, %r8
.Ltmp24086:
	leaq	80(%rsp), %rdi
	movq	%r14, %r9
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp24087:
	movq	80(%rsp), %rdx
	movl	88(%rsp), %ecx
	movl	92(%rsp), %eax
	cmpq	$-1, %rdx
	je	.LBB1288_38
	vmovups	112(%rsp), %zmm1
	vmovups	96(%rsp), %zmm0
	vmovups	%zmm1, 32(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	%rdx, (%rbx)
	movl	%ecx, 8(%rbx)
	movl	%eax, 12(%rbx)
	movq	(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1288_50
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	8(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1288_31
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1288_31:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1288_37
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1288_31
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB1288_34:
	cmpq	%rax, %rsi
	jge	.LBB1288_36
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1288_34
.LBB1288_36:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1288_37:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB1288_50
.LBB1288_38:
	movq	32(%r13), %rdx
	movq	(%rdx), %rdi
	movq	(%rsp), %rdx
	movq	%rdx, %rsi
	cmpq	$6, %rdx
	jb	.LBB1288_40
	movq	16(%rsp), %rsi
.LBB1288_40:
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB1288_51
	cmpq	$6, %rdx
	jb	.LBB1288_43
	movq	8(%rsp), %rdx
	movq	%rdx, 48(%rsp)
.LBB1288_43:
	movq	48(%rsp), %rdx
	leaq	888(%r14), %rsi
	movl	%ecx, (%rdx,%rdi,8)
	movl	%eax, 4(%rdx,%rdi,8)
	leaq	176(%rsp), %rdi
	movq	40(%r13), %rax
	vmovups	(%rsp), %ymm0
	movq	32(%rsp), %rcx
	movq	(%rax), %rdx
	movq	%rcx, 112(%rsp)
	leaq	80(%rsp), %rcx
	vmovups	%ymm0, 80(%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::parallel::minted_row::<purrdf_core::ir::term::TermId>
	movq	16(%r12), %r15
	cmpq	(%r12), %r15
	jne	.LBB1288_45
.Ltmp24091:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
.Ltmp24092:
.LBB1288_45:
	movq	8(%r12), %rax
	movq	208(%rsp), %rdx
	leaq	(%r15,%r15,4), %rcx
	incq	%r15
	movq	%rdx, 32(%rax,%rcx,8)
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm0, (%rax,%rcx,8)
	movq	%r15, 16(%r12)
	cmpb	$2, 1442(%r14)
	jne	.LBB1288_48
	movq	1312(%r14), %rax
	testq	%rax, %rax
	je	.LBB1288_48
	movq	1304(%r14), %rcx
	shlq	$5, %rax
	incq	-8(%rcx,%rax)
.LBB1288_48:
	leaq	1248(%r14), %rdi
	movq	%r14, %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::settle::<purrdf_core::ir::dataset::RdfDataset>
	jmp	.LBB1288_49
.LBB1288_8:
.Ltmp24079:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	leaq	88(%rsp), %r12
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp24080:
	jmp	.LBB1288_9
.LBB1288_16:
.Ltmp24082:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%rsp, %rdi
	movq	%r12, %rdx
	callq	*%rax
.Ltmp24083:
	movq	(%rsp), %rax
	leaq	8(%rsp), %rdi
	jmp	.LBB1288_18
.LBB1288_51:
.Ltmp24088:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.700(%rip), %rdx
	callq	*%rax
.Ltmp24089:
	ud2
.LBB1288_11:
.Ltmp24081:
	movq	%rax, %rbx
	movq	80(%rsp), %rax
	movq	%r12, 48(%rsp)
	cmpq	$6, %rax
	jae	.LBB1288_12
	jmp	.LBB1288_13
.LBB1288_53:
.Ltmp24093:
	movq	%rax, %rbx
.Ltmp24094:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::parallel::MintedRow>
.Ltmp24095:
	jmp	.LBB1288_13
.LBB1288_54:
.Ltmp24096:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1288_55:
.Ltmp24090:
	movq	%rax, %rbx
	movq	(%rsp), %rax
	cmpq	$5, %rax
	jbe	.LBB1288_13
.LBB1288_12:
	movq	48(%rsp), %rcx
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	movq	(%rcx), %rdi
	callq	__rustc::__rust_dealloc
.LBB1288_13:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1288:
purrdf_sparql_eval::expr::eval_filter_sequence::<purrdf_core::ir::dataset::RdfDataset>::{closure#1}:
.Lfunc_begin1289:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception862
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
	subq	$184, %rsp
	.cfi_def_cfa_offset 240
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdx, %r14
	movq	%r8, %rdx
	subq	(%rsi), %rdx
	movabsq	$-3689348814741910323, %rax
	movq	%r8, %rbp
	movq	%rcx, %r15
	movq	%rsi, %r12
	movq	%rdi, %rbx
	mulxq	%rax, %rax, %rax
	cmpb	$2, 1442(%r14)
	jne	.LBB1289_6
	cmpb	$0, 1400(%r14)
	jne	.LBB1289_35
	movq	1368(%r14), %rcx
	shrq	$5, %rax
	testq	%rcx, %rcx
	je	.LBB1289_5
	movq	16(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1289_5
	movb	$1, 1400(%r14)
	movq	$-1, (%rbx)
	jmp	.LBB1289_16
.LBB1289_5:
	movq	%rax, 1376(%r14)
.LBB1289_6:
	leaq	1248(%r14), %rsi
	movq	%rsp, %rdi
	movq	%r14, %rdx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
	cmpb	$-1, (%rsp)
	je	.LBB1289_7
.LBB1289_35:
	movq	$-1, (%rbx)
	jmp	.LBB1289_16
.LBB1289_7:
	movq	(%rbp), %r13
	leaq	1448(%r14), %rsi
	decq	%r13
	cmpq	$5, %r13
	jb	.LBB1289_8
	movq	16(%rbp), %r13
	movq	8(%rbp), %rbp
	decq	%r13
	jmp	.LBB1289_10
.LBB1289_8:
	addq	$8, %rbp
.LBB1289_10:
	movq	16(%r12), %rax
	movq	%rsp, %rdi
	movq	%rbp, %rdx
	movq	%r13, %rcx
	movq	%r14, %r9
	movq	(%rax), %r8
	addq	$16, %r8
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
	movq	(%rsp), %rax
	movl	8(%rsp), %edx
	movl	12(%rsp), %ecx
	cmpq	$-1, %rax
	je	.LBB1289_12
	vmovups	32(%rsp), %zmm1
	vmovups	16(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 112(%rsp)
	vmovups	%zmm0, 96(%rsp)
.LBB1289_15:
	vmovups	96(%rsp), %zmm0
	vmovups	112(%rsp), %zmm1
	movw	%si, 9(%rbx)
	shrl	$16, %esi
	movb	%sil, 11(%rbx)
	movl	%ecx, 12(%rbx)
	vmovups	%zmm0, 16(%rbx)
	vmovups	%zmm1, 32(%rbx)
	movq	%rax, (%rbx)
	movb	%dl, 8(%rbx)
.LBB1289_16:
	addq	$184, %rsp
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
	vzeroupper
	retq
.LBB1289_12:
	.cfi_def_cfa_offset 240
	cmpl	$2, %edx
	jne	.LBB1289_13
.LBB1289_34:
	leaq	1248(%r14), %rdi
	movq	%r14, %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::settle::<purrdf_core::ir::dataset::RdfDataset>
	jmp	.LBB1289_35
.LBB1289_13:
	movq	%rsp, %rdi
	movq	%r14, %rsi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
	movq	(%rsp), %rax
	movzbl	8(%rsp), %edx
	cmpq	$-1, %rax
	je	.LBB1289_17
	movzbl	11(%rsp), %ecx
	movzwl	9(%rsp), %esi
	vmovups	16(%rsp), %zmm0
	vmovups	32(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	12(%rsp), %ecx
	vmovups	%zmm0, 96(%rsp)
	vmovups	%zmm1, 112(%rsp)
	jmp	.LBB1289_15
.LBB1289_17:
	testb	$1, %dl
	je	.LBB1289_34
	leaq	(,%r13,8), %r12
	cmpq	$5, %r13
	jae	.LBB1289_19
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%rbp, %rsi
	movq	%r12, %rdx
	callq	*%rax
	incq	%r13
	jmp	.LBB1289_29
.LBB1289_19:
	movl	$4, %esi
	movq	%r12, %rdi
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB1289_39
	movb	$61, %cl
	leaq	-1(%r13), %rsi
	bzhiq	%rcx, %r13, %rcx
	cmpq	%rsi, %rcx
	cmovbq	%rcx, %rsi
	cmpq	$16, %rsi
	jae	.LBB1289_22
	xorl	%esi, %esi
	movq	%r13, %rcx
	movq	%rbp, %rdx
	jmp	.LBB1289_24
.LBB1289_22:
	incq	%rsi
	movl	$16, %edx
	movl	%esi, %ecx
	andl	$15, %ecx
	cmovneq	%rcx, %rdx
	movq	%r13, %rcx
	xorl	%edi, %edi
	subq	%rdx, %rsi
	leaq	(%rbp,%rsi,8), %rdx
	subq	%rsi, %rcx
.LBB1289_23:
	vmovups	(%rbp,%rdi,8), %zmm0
	vmovups	64(%rbp,%rdi,8), %zmm1
	vmovups	%zmm1, 64(%rax,%rdi,8)
	vmovups	%zmm0, (%rax,%rdi,8)
	addq	$16, %rdi
	cmpq	%rdi, %rsi
	jne	.LBB1289_23
.LBB1289_24:
	leaq	(%rbp,%r13,8), %rdi
	leaq	4(%rax,%rsi,8), %rsi
	xorl	%r8d, %r8d
.LBB1289_25:
	cmpq	%rdi, %rdx
	je	.LBB1289_27
	movl	(%rdx), %r9d
	movl	4(%rdx), %r10d
	addq	$8, %rdx
	movl	%r9d, -4(%rsi,%r8,8)
	movl	%r10d, (%rsi,%r8,8)
	incq	%r8
	cmpq	%r8, %rcx
	jne	.LBB1289_25
.LBB1289_27:
	incq	%r13
	movq	%rax, (%rsp)
	movq	%r13, 8(%rsp)
.LBB1289_29:
	movq	16(%r15), %r12
	cmpq	(%r15), %r12
	jne	.LBB1289_31
.Ltmp24097:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.Ltmp24098:
.LBB1289_31:
	movq	8(%r15), %rax
	leaq	(%r12,%r12,4), %rcx
	incq	%r12
	movq	%r13, (%rax,%rcx,8)
	vmovups	(%rsp), %ymm0
	vmovups	%ymm0, 8(%rax,%rcx,8)
	movq	%r12, 16(%r15)
	cmpb	$2, 1442(%r14)
	jne	.LBB1289_34
	movq	1312(%r14), %rax
	testq	%rax, %rax
	je	.LBB1289_34
	movq	1304(%r14), %rcx
	shlq	$5, %rax
	incq	-8(%rcx,%rax)
	jmp	.LBB1289_34
.LBB1289_39:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movq	%r12, %rsi
	callq	*%rax
.LBB1289_36:
.Ltmp24099:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB1289_38
	movq	(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1289_38:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1289:
purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#0}:
.Lfunc_begin1295:
	.cfi_startproc
	cmpq	$0, 40(%rsi)
	je	.LBB1295_1
	vpbroadcastq	.LCPI1295_4(%rip), %xmm0
	movabsq	$2746377873070565055, %rax
	movq	16(%rsi), %rdx
	movq	24(%rsi), %rsi
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	xorl	%r8d, %r8d
	xorq	%rdi, %rax
	vpinsrq	$0, %rax, %xmm0, %xmm0
	vaesenc	.LCPI1295_1(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1295_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1295_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rax
	movq	%rax, %rcx
	shrq	$57, %rcx
	vpbroadcastb	%ecx, %xmm0
	xorl	%ecx, %ecx
.LBB1295_4:
	andq	%rsi, %rax
	vmovdqu	(%rdx,%rax), %xmm2
	vpcmpeqb	%xmm0, %xmm2, %k0
	kortestw	%k0, %k0
	je	.LBB1295_10
	kmovd	%k0, %r9d
.LBB1295_6:
	xorl	%r10d, %r10d
	tzcntl	%r9d, %r10d
	addq	%rax, %r10
	andq	%rsi, %r10
	negq	%r10
	leaq	(%r10,%r10,4), %r10
	cmpq	%rdi, -40(%rdx,%r10,8)
	je	.LBB1295_7
	leal	-1(%r9), %r10d
	andw	%r9w, %r10w
	movl	%r10d, %r9d
	jne	.LBB1295_6
	.p2align	4
.LBB1295_10:
	vpcmpeqb	%xmm1, %xmm2, %k0
	kortestw	%k0, %k0
	jne	.LBB1295_8
	leaq	16(%rax,%r8), %rax
	addq	$16, %r8
	jmp	.LBB1295_4
.LBB1295_7:
	leaq	(%rdx,%r10,8), %rcx
.LBB1295_8:
	leaq	-32(%rcx), %rax
	testq	%rcx, %rcx
	cmoveq	%rcx, %rax
	retq
.LBB1295_1:
	xorl	%eax, %eax
	retq
.Lfunc_end1295:
purrdf_sparql_eval::binop::eval_application_yielding::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>::{closure#0}:
.Lfunc_begin1301:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception869
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
	subq	$408, %rsp
	.cfi_def_cfa_offset 464
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rsi), %rax
	movq	%rdi, %rbx
	cmpb	$0, (%rax)
	je	.LBB1301_1
	movb	$1, 8(%rbx)
	jmp	.LBB1301_36
.LBB1301_1:
	movq	8(%rsi), %r15
	leaq	112(%rsp), %rdi
	movq	%rsi, %r14
	movq	%rcx, %r13
	movq	%rdx, %rbp
	movq	%rdx, %rsi
	movq	%rax, 8(%rsp)
	movq	%r15, %rdx
	callq	purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints::<purrdf_core::ir::dataset::RdfDataset>
	cmpq	$-1, 112(%rsp)
	je	.LBB1301_4
	vmovups	112(%rsp), %zmm0
	vmovups	144(%rsp), %zmm1
.LBB1301_3:
	vmovups	%zmm1, 32(%rbx)
	vmovups	%zmm0, (%rbx)
	jmp	.LBB1301_37
.LBB1301_4:
	movq	16(%rbp), %rax
	movq	%r15, 72(%rsp)
	movq	%r14, 16(%rsp)
	movq	%rbx, 24(%rsp)
	testq	%rax, %rax
	je	.LBB1301_33
	movq	16(%rsp), %rdi
	movq	24(%rbp), %rsi
	movq	8(%rbp), %r14
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rbx
	movq	48(%rdi), %rdx
	vpermpd	$201, 16(%rdi), %ymm0
	addq	$16, %rsi
	movq	16(%rdi), %r15
	movq	24(%rdi), %rcx
	movq	%rsi, 64(%rsp)
	movq	40(%rdi), %rsi
	movq	%rdx, 56(%rsp)
	movq	32(%rdi), %rdx
	movq	%rcx, 48(%rsp)
	movq	%rsi, 32(%rsp)
	vmovups	%ymm0, 256(%rsp)
	movq	%rdx, 40(%rsp)
	jmp	.LBB1301_6
	.p2align	4
.LBB1301_30:
	leaq	120(%rsp), %rdi
	addq	$40, %r14
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	addq	$-40, %rbx
	je	.LBB1301_33
.LBB1301_6:
	movq	(%r15), %r12
	decq	%r12
	cmpq	$3, %r12
	jb	.LBB1301_8
	movq	16(%r15), %r12
	decq	%r12
.LBB1301_8:
	vmovups	256(%rsp), %ymm0
	movq	72(%rsp), %rsi
	movq	64(%rsp), %rcx
	movq	56(%rsp), %r8
	leaq	208(%rsp), %rax
	leaq	288(%rsp), %rdi
	leaq	80(%rsp), %r9
	movq	%rbp, 208(%rsp)
	movq	%r14, 216(%rsp)
	movq	%r14, %rdx
	movq	%r13, (%rsp)
	movq	%rax, 80(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.714(%rip), %rax
	movq	%rax, 88(%rsp)
	movb	$0, 96(%rsp)
	vmovups	%ymm0, 224(%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::binop::evaluate_application_row_with::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>
	cmpl	$1, 288(%rsp)
	je	.LBB1301_9
	leaq	296(%rsp), %rax
	vmovups	32(%rax), %zmm1
	vmovups	(%rax), %zmm0
	vmovups	%zmm1, 144(%rsp)
	vmovups	%zmm0, 112(%rsp)
	cmpq	$-1, 112(%rsp)
	jne	.LBB1301_11
	movq	48(%rsp), %rax
	movq	(%rax), %rax
	cmpb	$0, 16(%rax)
	jne	.LBB1301_32
	movq	32(%rsp), %rax
	cmpb	$0, (%rax)
	je	.LBB1301_30
	movq	40(%rsp), %rax
	movq	32(%rsp), %rcx
	movq	(%rax), %rax
	cmpq	8(%rcx), %rax
	jb	.LBB1301_30
.LBB1301_32:
	movq	8(%rsp), %rax
	leaq	120(%rsp), %rdi
	movb	$1, (%rax)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB1301_33
.LBB1301_9:
	vmovups	304(%rsp), %zmm0
	vmovups	336(%rsp), %zmm1
	movq	24(%rsp), %rbx
	jmp	.LBB1301_3
.LBB1301_11:
	movq	(%r15), %rcx
	xorl	%eax, %eax
	cmpq	$4, %rcx
	setae	%al
	shll	$4, %eax
	movq	(%r15,%rax), %r14
	leaq	-1(%r14), %rdx
	cmpq	%rdx, %r12
	jae	.LBB1301_25
	cmpq	$4, %rcx
	jb	.LBB1301_13
	movq	8(%r15), %rcx
	jmp	.LBB1301_15
.LBB1301_13:
	leaq	8(%r15), %rcx
.LBB1301_15:
	subq	%r12, %r14
	leaq	1(%r12), %rdx
	shlq	$5, %r12
	movq	%rdx, (%r15,%rax)
	addq	%rcx, %r12
	addq	$-2, %r14
	.p2align	4
.LBB1301_17:
.Ltmp24216:
	movq	%r12, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp24217:
	addq	$32, %r12
	addq	$-1, %r14
	jb	.LBB1301_17
.LBB1301_25:
	movq	16(%rsp), %rax
	leaq	80(%rsp), %rdi
	movl	$1, %edx
	leaq	296(%rsp), %rcx
	movq	56(%rax), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
	cmpq	$-1, 80(%rsp)
	je	.LBB1301_27
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.LBB1301_27:
	movq	8(%rsp), %rax
	movb	$1, (%rax)
.LBB1301_33:
	movq	8(%rsp), %rax
	movq	24(%rsp), %rbx
	movzbl	(%rax), %eax
	movb	%al, 8(%rbx)
.LBB1301_36:
	movq	$-1, (%rbx)
.LBB1301_37:
	movq	%rbx, %rax
	addq	$408, %rsp
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
	vzeroupper
	retq
.LBB1301_18:
	.cfi_def_cfa_offset 464
.Ltmp24218:
	movq	%rax, %rbx
	testq	%r14, %r14
	je	.LBB1301_22
	addq	$32, %r12
.LBB1301_20:
.Ltmp24219:
	movq	%r12, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp24220:
	addq	$32, %r12
	decq	%r14
	jne	.LBB1301_20
.LBB1301_22:
.Ltmp24222:
	leaq	296(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>>
.Ltmp24223:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1301_24:
.Ltmp24221:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1301_34:
.Ltmp24224:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1301:
purrdf_sparql_eval::modifier::eval_graph_var::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>::{closure#1}:
.Lfunc_begin1313:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception877
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
	subq	$536, %rsp
	.cfi_def_cfa_offset 592
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	%rcx, %r15
	movq	%rsi, %r12
	movq	%rdi, %r14
	movq	$0, 64(%rsp)
.Ltmp24411:
	leaq	208(%rsp), %rdi
	movq	%rdx, %rsi
	callq	<purrdf_sparql_eval::solution::SolutionSeq as core::clone::Clone>::clone
.Ltmp24412:
	movq	8(%r12), %rax
	movq	232(%rsp), %r13
	movq	(%r12), %rbx
	movl	(%rax), %eax
	leaq	16(%r13), %rsi
	movl	%eax, 144(%rsp)
.Ltmp24413:
	leaq	320(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
.Ltmp24414:
.Ltmp24416:
	leaq	320(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)
.Ltmp24417:
	movq	%r14, 200(%rsp)
	movq	%r15, 192(%rsp)
	movq	%r12, 184(%rsp)
	movq	%r13, 176(%rsp)
	cmpq	$1, %rax
	jne	.LBB1313_21
	movq	224(%rsp), %rax
	movq	216(%rsp), %r12
	movq	208(%rsp), %rsi
	movq	%rdx, %rbp
	leaq	(%rax,%rax,4), %rcx
	movq	%r12, 400(%rsp)
	movq	%rsi, 72(%rsp)
	movq	%rsi, 416(%rsp)
	movq	%r12, 40(%rsp)
	leaq	(%r12,%rcx,8), %rdx
	movq	%rdx, 8(%rsp)
	movq	%rdx, 424(%rsp)
	testq	%rax, %rax
	je	.LBB1313_91
	vmovd	144(%rsp), %xmm0
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r14
	movq	40(%rsp), %r12
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.538(%rip), %rax
	leaq	88(%rsp), %r13
	movl	$8, %edi
	movq	$0, 16(%rsp)
	vpshufb	.LCPI1313_0(%rip), %xmm0, %xmm0
	movq	%rax, 168(%rsp)
	vmovdqa	%xmm0, 272(%rsp)
	jmp	.LBB1313_6
	.p2align	4
.LBB1313_129:
	movq	16(%rsp), %rdx
	leaq	(%rdx,%rdx,4), %rax
	incq	%rdx
	movq	%rdx, 16(%rsp)
	movq	%r15, (%rdi,%rax,8)
	movq	%rbx, 8(%rdi,%rax,8)
	vmovdqa	240(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdi,%rax,8)
	movq	256(%rsp), %rcx
	movq	%rcx, 32(%rdi,%rax,8)
	movq	%rdx, 64(%rsp)
.LBB1313_125:
	cmpq	8(%rsp), %r12
	je	.LBB1313_104
.LBB1313_6:
	movq	%r12, %rax
	movq	(%rax), %r15
	addq	$40, %r12
	testq	%r15, %r15
	je	.LBB1313_91
	movq	%r15, 80(%rsp)
	leaq	-1(%r15), %rcx
	vmovdqu	8(%rax), %ymm0
	cmpq	$5, %rcx
	vmovdqu	%ymm0, (%r13)
	movq	96(%rsp), %rax
	movq	88(%rsp), %rbx
	leaq	-1(%rax), %rsi
	cmovbq	%rcx, %rsi
	cmpq	%rsi, %rbp
	jae	.LBB1313_12
	cmpq	$5, %rcx
	movq	%r13, %rcx
	cmovaeq	%rbx, %rcx
	movl	(%rcx,%rbp,8), %edx
	testl	%edx, %edx
	je	.LBB1313_114
	cmpl	$2, %edx
	jne	.LBB1313_115
.LBB1313_10:
	cmpq	$6, %r15
	cmovbq	%r15, %rax
	decq	%rax
	cmpq	%rax, %rbp
	jae	.LBB1313_11
	vmovaps	272(%rsp), %xmm0
	cmpq	$6, %r15
	movq	16(%rsp), %rcx
	cmovbq	%r13, %rbx
	vmovlps	%xmm0, (%rbx,%rbp,8)
	vmovdqu	8(%r13), %xmm0
	movq	24(%r13), %rax
	movq	80(%rsp), %r15
	movq	88(%rsp), %rbx
	movq	%rax, 256(%rsp)
	vmovdqa	%xmm0, 240(%rsp)
	cmpq	48(%rsp), %rcx
	jne	.LBB1313_129
.Ltmp24433:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp24434:
	movq	56(%rsp), %rdi
	jmp	.LBB1313_129
	.p2align	4
.LBB1313_114:
	movl	144(%rsp), %edx
	cmpl	%edx, 4(%rcx,%rbp,8)
	je	.LBB1313_10
.LBB1313_115:
	cmpq	$6, %r15
	jb	.LBB1313_125
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	leaq	-8(,%r15,8), %rcx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1313_118
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1313_118:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1313_124
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1313_118
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r14), %rax
	.p2align	4
.LBB1313_121:
	cmpq	%rax, %rdx
	jge	.LBB1313_123
	lock		cmpxchgq	%rdx, (%r14)
	jne	.LBB1313_121
.LBB1313_123:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1313_124:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rdi, %r15
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	movq	%r15, %rdi
	jmp	.LBB1313_125
.LBB1313_21:
	movq	(%rbx), %rax
	lock		incq	(%rax)
	jle	.LBB1313_134
	movq	8(%rbx), %rdx
	movq	(%rbx), %rsi
.Ltmp24418:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	320(%rsp), %rdi
	callq	*%rax
	movq	%rax, 272(%rsp)
.Ltmp24419:
	movq	224(%rsp), %rax
	movq	216(%rsp), %rbx
	movq	208(%rsp), %rdx
	movq	336(%rsp), %r14
	leaq	(%rax,%rax,4), %rcx
	movq	%rbx, 240(%rsp)
	movq	%rdx, 72(%rsp)
	movq	%rdx, 256(%rsp)
	movq	%rbx, %r13
	movq	%rbx, 8(%rsp)
	leaq	(%rbx,%rcx,8), %r12
	movq	%r12, 264(%rsp)
	testq	%rax, %rax
	je	.LBB1313_44
	vmovd	144(%rsp), %xmm0
	leaq	1(%r14), %rax
	movl	$8, %ebp
	movl	$40, %r13d
	xorl	%r15d, %r15d
	vpshufb	.LCPI1313_0(%rip), %xmm0, %xmm0
	movq	%rax, 40(%rsp)
	vmovdqa	%xmm0, 144(%rsp)
	jmp	.LBB1313_25
	.p2align	4
.LBB1313_36:
	movq	56(%rsp), %rbp
.LBB1313_37:
	movq	16(%rsp), %rdx
	movq	%rbx, -40(%rbp,%r13)
	movq	8(%rsp), %rcx
	incq	%r15
	movq	%rdx, -32(%rbp,%r13)
	leaq	-40(%rcx,%r13), %rax
	movq	%rcx, %rbx
	vmovdqa	288(%rsp), %xmm0
	addq	$40, %rax
	vmovdqu	%xmm0, -24(%rbp,%r13)
	movq	304(%rsp), %rcx
	movq	%rcx, -8(%rbp,%r13)
	addq	$40, %r13
	movq	%r15, 64(%rsp)
	cmpq	%r12, %rax
	je	.LBB1313_57
.LBB1313_25:
	vmovups	-32(%rbx,%r13), %ymm0
	movq	-40(%rbx,%r13), %rax
	vmovups	%ymm0, 400(%rsp)
	testq	%rax, %rax
	je	.LBB1313_43
	vmovups	400(%rsp), %ymm0
	leaq	88(%rsp), %rcx
	movq	%rax, 80(%rsp)
	leaq	-1(%rax), %rdx
	cmpq	$5, %rdx
	vmovups	%ymm0, (%rcx)
	movq	96(%rsp), %rcx
	leaq	-1(%rcx), %rsi
	cmovbq	%rdx, %rsi
	movq	%r14, %rdx
	subq	%rsi, %rdx
	jbe	.LBB1313_27
	movl	$2, 288(%rsp)
	movq	%rdx, 296(%rsp)
.Ltmp24421:
	leaq	80(%rsp), %rdi
	leaq	288(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp24422:
	movq	272(%rsp), %rdi
	jmp	.LBB1313_29
	.p2align	4
.LBB1313_27:
	movq	272(%rsp), %rdi
	cmpq	$6, %rax
	cmovbq	%rax, %rcx
	decq	%rcx
	cmpq	%rcx, %r14
	jae	.LBB1313_29
	movq	40(%rsp), %rdx
	xorl	%ecx, %ecx
	cmpq	$6, %rax
	setae	%cl
	shll	$4, %ecx
	movq	%rdx, 80(%rsp,%rcx)
.LBB1313_29:
	movq	80(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB1313_31
	movq	96(%rsp), %rsi
.LBB1313_31:
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB1313_83
	leaq	88(%rsp), %rcx
	cmpq	$6, %rax
	jb	.LBB1313_34
	movq	88(%rsp), %rcx
.LBB1313_34:
	vmovaps	144(%rsp), %xmm0
	vmovlps	%xmm0, (%rcx,%rdi,8)
	leaq	88(%rsp), %rcx
	movq	88(%rsp), %rax
	vmovups	8(%rcx), %xmm0
	movq	80(%rsp), %rbx
	movq	%rax, 16(%rsp)
	movq	24(%rcx), %rax
	vmovaps	%xmm0, 288(%rsp)
	movq	%rax, 304(%rsp)
	cmpq	48(%rsp), %r15
	jne	.LBB1313_37
.Ltmp24427:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp24428:
	jmp	.LBB1313_36
.LBB1313_91:
	subq	%r12, 8(%rsp)
	je	.LBB1313_104
	movq	8(%rsp), %rax
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	movabsq	$-3689348814741910323, %rbx
	xorl	%r14d, %r14d
	shrq	$3, %rax
	imulq	%rax, %rbx
	jmp	.LBB1313_93
	.p2align	4
.LBB1313_101:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1313_102:
	vzeroupper
	callq	*%rbp
.LBB1313_103:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB1313_104
.LBB1313_93:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r12,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB1313_103
	leaq	(%r12,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1313_96
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1313_96:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1313_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1313_96
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r15)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r13), %rax
	.p2align	4
.LBB1313_99:
	cmpq	%rax, %rdx
	jge	.LBB1313_101
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB1313_99
	jmp	.LBB1313_101
.LBB1313_104:
	movq	72(%rsp), %rax
	movq	200(%rsp), %r14
	movq	192(%rsp), %r15
	movq	184(%rsp), %rbx
	movq	176(%rsp), %r12
	testq	%rax, %rax
	je	.LBB1313_68
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1313_107
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1313_107:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1313_113
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1313_107
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1313_110:
	cmpq	%rax, %rdx
	jge	.LBB1313_112
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1313_110
.LBB1313_112:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1313_113:
	movq	free@GOTPCREL(%rip), %rax
	movq	40(%rsp), %rdi
	jmp	.LBB1313_67
.LBB1313_43:
	addq	%rbx, %r13
.LBB1313_44:
	subq	%r13, %r12
	je	.LBB1313_57
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %rbx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r14d, %r14d
	imulq	%r12, %rbx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	jmp	.LBB1313_46
	.p2align	4
.LBB1313_54:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1313_55:
	vzeroupper
	callq	*%rbp
.LBB1313_56:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB1313_57
.LBB1313_46:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB1313_56
	leaq	(%r13,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1313_49
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1313_49:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1313_55
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1313_49
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r15)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%r12), %rax
	.p2align	4
.LBB1313_52:
	cmpq	%rax, %rdx
	jge	.LBB1313_54
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB1313_52
	jmp	.LBB1313_54
.LBB1313_57:
	movq	72(%rsp), %rax
	movq	200(%rsp), %r14
	movq	192(%rsp), %r15
	movq	184(%rsp), %rbx
	movq	176(%rsp), %r12
	testq	%rax, %rax
	je	.LBB1313_68
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1313_60
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1313_60:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1313_66
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1313_60
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1313_63:
	cmpq	%rax, %rdx
	jge	.LBB1313_65
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1313_63
.LBB1313_65:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1313_66:
	movq	free@GOTPCREL(%rip), %rax
	movq	8(%rsp), %rdi
.LBB1313_67:
	vzeroupper
	callq	*%rax
.LBB1313_68:
	vmovups	344(%rsp), %ymm1
	vmovups	320(%rsp), %ymm0
	vmovups	%ymm1, 104(%rsp)
	vmovups	%ymm0, 80(%rsp)
	lock		decq	(%r12)
	jne	.LBB1313_70
	#MEMBARRIER
.Ltmp24441:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	232(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp24442:
.LBB1313_70:
	movq	16(%rbx), %rax
	vmovq	784(%r15), %xmm0
	vmovups	80(%rsp), %ymm2
	vmovups	104(%rsp), %ymm1
	movq	24(%rbx), %rbx
	movl	$72, %edi
	movq	(%rax), %rax
	movq	$1, 320(%rsp)
	movq	$1, 328(%rsp)
	vmovups	%ymm2, 336(%rsp)
	vmovdqa	%xmm0, 16(%rsp)
	vmovups	%ymm1, 360(%rsp)
	movq	%rax, 784(%r15)
	movq	malloc@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1313_133
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB1313_73
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1313_73:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1313_79
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1313_73
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	addq	$72, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB1313_76:
	cmpq	%rax, %rdx
	jle	.LBB1313_78
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB1313_76
.LBB1313_78:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1313_79:
	movq	384(%rsp), %rax
	vmovups	320(%rsp), %zmm0
	vmovups	48(%rsp), %xmm1
	movq	%rax, 64(%rcx)
	movq	64(%rsp), %rax
	vmovups	%zmm0, (%rcx)
	vmovaps	%xmm1, 320(%rsp)
	movq	%rax, 336(%rsp)
	movq	%rcx, 344(%rsp)
	movq	(%rbx), %rsi
.Ltmp24444:
	leaq	432(%rsp), %rdi
	leaq	320(%rsp), %rdx
	movq	%r15, %rcx
	vzeroupper
	callq	<&mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset> as purrdf_sparql_eval::eval::RowDelivery<purrdf_core::ir::dataset::RdfDataset>>::deliver
.Ltmp24445:
.Ltmp24449:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp24450:
	vmovaps	16(%rsp), %xmm0
	movq	432(%rsp), %rax
	leaq	8(%r14), %rcx
	vmovlps	%xmm0, 784(%r15)
	cmpq	$-1, %rax
	je	.LBB1313_138
	vmovups	464(%rsp), %zmm1
	vmovups	440(%rsp), %zmm0
	vmovups	%zmm1, 24(%rcx)
	vmovups	%zmm0, (%rcx)
	jmp	.LBB1313_139
.LBB1313_138:
	movq	(%rbx), %rdx
	movzbl	16(%rdx), %edx
	movb	%dl, (%rcx)
.LBB1313_139:
	movq	%rax, (%r14)
	movq	%r14, %rax
	addq	$536, %rsp
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
	vzeroupper
	retq
.LBB1313_83:
	.cfi_def_cfa_offset 592
	addq	%r13, %rbx
	movq	%rbx, 248(%rsp)
.Ltmp24424:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.537(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp24425:
	jmp	.LBB1313_134
.LBB1313_11:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.539(%rip), %rcx
	movq	%rax, %rsi
	movq	%rcx, 168(%rsp)
.LBB1313_12:
	movq	%r12, 408(%rsp)
.Ltmp24430:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	168(%rsp), %rdx
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp24431:
	jmp	.LBB1313_134
.LBB1313_133:
.Ltmp24455:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	336(%rsp), %rbx
	callq	*%rax
.Ltmp24456:
.LBB1313_134:
	ud2
.LBB1313_140:
.Ltmp24451:
	cmpq	$-1, 432(%rsp)
	movq	%rax, %rbp
	je	.LBB1313_142
.Ltmp24452:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::error::EvalError>
.Ltmp24453:
	jmp	.LBB1313_142
.LBB1313_137:
.Ltmp24446:
	movq	%rax, %rbp
.Ltmp24447:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp24448:
.LBB1313_142:
	movq	%rbp, %rdi
	callq	_Unwind_Resume@PLT
.LBB1313_145:
.Ltmp24454:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1313_135:
.Ltmp24457:
	movq	%rax, %rbp
.Ltmp24458:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp24459:
	jmp	.LBB1313_144
.LBB1313_136:
.Ltmp24460:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1313_13:
.Ltmp24415:
	movb	$1, %bl
	movq	%rax, %rbp
	jmp	.LBB1313_14
.LBB1313_130:
.Ltmp24435:
	movq	%rax, %rbp
	movq	%r12, 408(%rsp)
	jmp	.LBB1313_132
.LBB1313_143:
.Ltmp24443:
	movq	%rax, %rbp
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%rbp, %rdi
	callq	_Unwind_Resume@PLT
.LBB1313_18:
.Ltmp24420:
	movb	$1, %bl
	movq	%rax, %rbp
	jmp	.LBB1313_19
.LBB1313_84:
.Ltmp24423:
	addq	%r13, %rbx
	movq	%rax, %rbp
	movq	%rbx, 248(%rsp)
	jmp	.LBB1313_86
.LBB1313_38:
.Ltmp24429:
	movq	8(%rsp), %rcx
	movq	%rax, %rbp
	addq	%r13, %rcx
	movq	%rcx, 248(%rsp)
	cmpq	$5, %rbx
	ja	.LBB1313_39
	jmp	.LBB1313_40
.LBB1313_85:
.Ltmp24426:
	movq	%rax, %rbp
.LBB1313_86:
	movq	80(%rsp), %rbx
	cmpq	$6, %rbx
	jb	.LBB1313_40
	movq	88(%rsp), %rax
	movq	%rax, 16(%rsp)
.LBB1313_39:
	movq	16(%rsp), %rdi
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1313_40:
	leaq	240(%rsp), %rdi
	jmp	.LBB1313_41
.LBB1313_131:
.Ltmp24432:
	movq	%rax, %rbp
.LBB1313_132:
	cmpq	$5, %r15
	jbe	.LBB1313_90
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB1313_90:
	leaq	400(%rsp), %rdi
.LBB1313_41:
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
.LBB1313_19:
.Ltmp24436:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp24437:
	movq	232(%rsp), %r13
.LBB1313_14:
	lock		decq	(%r13)
	jne	.LBB1313_16
	#MEMBARRIER
.Ltmp24438:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	232(%rsp), %rdi
	callq	*%rax
.Ltmp24439:
.LBB1313_16:
	testb	%bl, %bl
	je	.LBB1313_144
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB1313_144:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%rbp, %rdi
	callq	_Unwind_Resume@PLT
.LBB1313_88:
.Ltmp24440:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1313:
purrdf_sparql_eval::modifier::eval_graph_with::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>::{closure#0}:
.Lfunc_begin1314:
	.cfi_startproc
	pushq	%r15
	.cfi_def_cfa_offset 16
	pushq	%r14
	.cfi_def_cfa_offset 24
	pushq	%r12
	.cfi_def_cfa_offset 32
	pushq	%rbx
	.cfi_def_cfa_offset 40
	subq	$104, %rsp
	.cfi_def_cfa_offset 144
	.cfi_offset %rbx, -40
	.cfi_offset %r12, -32
	.cfi_offset %r14, -24
	.cfi_offset %r15, -16
	movq	(%rsi), %rax
	movq	8(%rsi), %r15
	movq	784(%rcx), %r12
	movq	%rdi, %rbx
	movq	%rcx, %r14
	movq	%rsp, %rdi
	movq	(%rax), %rax
	movq	%rax, 784(%rcx)
	movq	(%r15), %rsi
	callq	<&mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset> as purrdf_sparql_eval::eval::RowDelivery<purrdf_core::ir::dataset::RdfDataset>>::deliver
	cmpq	$-1, (%rsp)
	movq	%r12, 784(%r14)
	je	.LBB1314_2
	vmovups	32(%rsp), %zmm1
	vmovups	(%rsp), %zmm0
	vmovups	%zmm1, 32(%rbx)
	vmovups	%zmm0, (%rbx)
	jmp	.LBB1314_3
.LBB1314_2:
	movq	(%r15), %rax
	movzbl	16(%rax), %eax
	movb	%al, 8(%rbx)
	movq	$-1, (%rbx)
.LBB1314_3:
	movq	%rbx, %rax
	addq	$104, %rsp
	.cfi_def_cfa_offset 40
	popq	%rbx
	.cfi_def_cfa_offset 32
	popq	%r12
	.cfi_def_cfa_offset 24
	popq	%r14
	.cfi_def_cfa_offset 16
	popq	%r15
	.cfi_def_cfa_offset 8
	vzeroupper
	retq
.Lfunc_end1314:
purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#6}:
.Lfunc_begin1315:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception878
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
	subq	$1640, %rsp
	.cfi_def_cfa_offset 1696
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rsi), %rax
	movq	%rdi, 32(%rsp)
	movq	%rsi, 40(%rsp)
	movq	16(%rax), %r13
	testq	%r13, %r13
	je	.LBB1315_1
	movq	malloc@GOTPCREL(%rip), %rbx
	movq	8(%rax), %rbp
	leaq	(,%r13,8), %rax
	leaq	(%rax,%rax,2), %r12
	movq	%r12, %rdi
	callq	*%rbx
	testq	%rax, %rax
	je	.LBB1315_27
	movq	%rax, %r15
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	leaq	(%r12,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%r12, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%r12, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB1315_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1315_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1315_11
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1315_5
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%r12, (%rdx)
	movq	%r12, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%r12), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%r12, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1315_8:
	cmpq	%rax, %rdx
	jle	.LBB1315_10
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1315_8
.LBB1315_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1315_11:
	leaq	384(%rsp), %r12
	movq	%r13, 48(%rsp)
	movq	%r15, 56(%rsp)
	xorl	%r14d, %r14d
	xorl	%ebx, %ebx
	.p2align	4
.LBB1315_12:
	imulq	$216, 16(%rbp,%r14), %rdx
	movq	8(%rbp,%r14), %rsi
	addq	%rsi, %rdx
.Ltmp24461:
	movq	%r12, %rdi
	callq	<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>> as alloc::vec::spec_from_iter::SpecFromIter<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>, <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::fresh>>>::from_iter
.Ltmp24462:
	vmovups	384(%rsp), %xmm0
	movq	400(%rsp), %rax
	incq	%rbx
	movq	%rax, 16(%r15,%r14)
	vmovups	%xmm0, (%r15,%r14)
	addq	$24, %r14
	cmpq	%rbx, %r13
	jne	.LBB1315_12
	jmp	.LBB1315_14
.LBB1315_1:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
.LBB1315_14:
	movq	40(%rsp), %rbx
	movq	48(%rsp), %rax
	movq	56(%rsp), %rcx
	movq	%r13, 16(%rsp)
	movq	16(%rbx), %rsi
	movq	%rcx, 8(%rsp)
	movq	%rax, (%rsp)
	movq	8(%rbx), %rcx
	movq	(%rsi), %rax
	testq	%rax, %rax
	cmoveq	%rax, %rsi
.Ltmp24467:
	leaq	384(%rsp), %rdi
	movq	%rcx, %rdx
	callq	<core::option::Option<&alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>>::map_or_else::<purrdf_sparql_eval::eval::EvalCtx, <purrdf_sparql_eval::eval::EvalCtx>::fork_for_loop_worker::{closure#0}, <purrdf_sparql_eval::eval::EvalCtx>::fork_for_loop_worker::{closure#1}>
.Ltmp24468:
	movq	24(%rbx), %rsi
	movzbl	160(%rsi), %ebx
.Ltmp24470:
	leaq	224(%rsp), %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger as core::clone::Clone>::clone
.Ltmp24471:
	vmovups	224(%rsp), %zmm0
	vmovups	288(%rsp), %zmm1
	vmovups	320(%rsp), %zmm2
	movb	%bl, 208(%rsp)
	vmovups	%zmm1, 112(%rsp)
	vmovups	%zmm0, 48(%rsp)
	vmovups	%zmm2, 144(%rsp)
	testb	%bl, %bl
	je	.LBB1315_17
.Ltmp24473:
	leaq	48(%rsp), %rdi
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::defer::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp24474:
.LBB1315_17:
	movq	32(%rsp), %rbx
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rsi
	movl	$1248, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	vmovaps	(%rsp), %xmm0
	vmovups	48(%rsp), %zmm3
	vmovups	112(%rsp), %zmm1
	vmovups	152(%rsp), %zmm2
	movq	16(%rsp), %rax
	movq	%rax, 1264(%rbx)
	vmovaps	%xmm0, 1248(%rbx)
	vmovups	%zmm1, 1336(%rbx)
	vmovups	%zmm3, 1272(%rbx)
	vmovups	%zmm2, 1376(%rbx)
	addq	$1640, %rsp
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
	vzeroupper
	retq
.LBB1315_27:
	.cfi_def_cfa_offset 1696
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r12, %rsi
	callq	*%rax
.LBB1315_25:
.Ltmp24475:
	movq	%rax, %r14
.Ltmp24476:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp24477:
	jmp	.LBB1315_23
.LBB1315_22:
.Ltmp24472:
	movq	%rax, %r14
.LBB1315_23:
.Ltmp24478:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp24479:
	jmp	.LBB1315_21
.LBB1315_20:
.Ltmp24469:
	movq	%rax, %r14
.LBB1315_21:
.Ltmp24480:
	movq	%rsp, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp24481:
	jmp	.LBB1315_19
.LBB1315_26:
.Ltmp24482:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1315_18:
.Ltmp24463:
	movq	%rax, %r14
	movq	%rbx, 64(%rsp)
.Ltmp24464:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp24465:
.LBB1315_19:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB1315_28:
.Ltmp24466:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1315:
purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#7}:
.Lfunc_begin1316:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception879
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
	subq	$312, %rsp
	.cfi_def_cfa_offset 368
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpb	$1, 1432(%rdx)
	movq	%r8, %r13
	movq	%rcx, %rbp
	movq	%rdx, %r15
	movq	%rdi, %r12
	jne	.LBB1316_7
	cmpb	$0, 1424(%r15)
	jne	.LBB1316_5
	movq	40(%r13), %rax
	movq	1392(%r15), %rcx
	testq	%rcx, %rcx
	je	.LBB1316_6
	movq	16(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1316_6
	movb	$1, 1424(%r15)
.LBB1316_5:
	movq	$-1, (%r12)
	jmp	.LBB1316_32
.LBB1316_6:
	movq	%rax, 1400(%r15)
.LBB1316_7:
	movq	(%rsi), %rax
	movq	%rsi, 48(%rsp)
	movq	(%rax), %rbx
	movq	$1, 208(%rsp)
	cmpq	$5, %rbx
	jae	.LBB1316_44
	vmovups	216(%rsp), %xmm0
	movq	240(%rsp), %rax
	movq	208(%rsp), %rdx
	movq	232(%rsp), %rcx
	movq	%rax, 144(%rsp)
	movq	%rdx, 112(%rsp)
	movq	%rcx, 136(%rsp)
	vmovups	%xmm0, 120(%rsp)
	testq	%rbx, %rbx
	je	.LBB1316_10
.LBB1316_9:
	movl	$2, %eax
	jmp	.LBB1316_11
.LBB1316_10:
	movl	$-1, %eax
.LBB1316_11:
	leaq	120(%rsp), %r14
	movl	%eax, 208(%rsp)
	movq	%r15, 56(%rsp)
	movq	%rbx, 216(%rsp)
.Ltmp24486:
	leaq	112(%rsp), %rdi
	leaq	208(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp24487:
	movq	48(%rsp), %rbx
	vmovups	112(%rsp), %ymm0
	movq	144(%rsp), %rax
	leaq	72(%rsp), %rdi
	movq	8(%rbx), %rcx
	movq	%rax, 96(%rsp)
	vmovups	%ymm0, 64(%rsp)
	movq	64(%rsp), %r15
	movq	(%rcx), %rdx
	movq	%rcx, 200(%rsp)
	movq	%r15, %rax
	cmpq	$6, %r15
	jb	.LBB1316_14
	movq	72(%rsp), %rdi
	movq	80(%rsp), %rax
.LBB1316_14:
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB1316_43
	movq	(%r13), %rax
	movq	16(%r13), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %rdx
	jne	.LBB1316_46
	movq	%rbp, 160(%rsp)
	movq	%r12, 152(%rsp)
	cmpq	$5, %rax
	jb	.LBB1316_18
	movq	8(%r13), %rsi
	jmp	.LBB1316_19
.LBB1316_18:
	leaq	8(%r13), %rsi
.LBB1316_19:
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	56(%rsp), %r15
	shlq	$3, %rdx
	vzeroupper
	callq	*%rax
	movq	24(%rbx), %rax
	movq	1264(%r15), %r14
	cmpq	%r14, %rax
	cmovbq	%rax, %r14
	testq	%r14, %r14
	je	.LBB1316_29
	movq	48(%rsp), %rax
	movq	64(%r13), %rdx
	movq	1256(%r15), %r12
	movq	56(%r13), %rcx
	xorl	%ebx, %ebx
	movq	16(%rax), %rbp
	movq	%rdx, 184(%rsp)
	movq	32(%rax), %rsi
	movq	40(%rax), %rdx
	movq	48(%rax), %r13
	addq	$16, %r12
	movq	%rcx, 192(%rsp)
	addq	$16, %rbp
	movq	%rsi, 176(%rsp)
	movq	%rdx, 168(%rsp)
	jmp	.LBB1316_22
	.p2align	4
.LBB1316_21:
	movq	56(%rsp), %r15
	incq	%rbx
	addq	$24, %r12
	addq	$120, %rbp
	vmovlps	%xmm0, (%rax,%rdi,8)
	cmpq	%rbx, %r14
	je	.LBB1316_29
.LBB1316_22:
	movq	176(%rsp), %rax
	vmovups	(%r13), %xmm1
	movq	-8(%r12), %rdx
	movq	(%r12), %rcx
	vmovups	8(%rax), %xmm0
	movq	168(%rsp), %rax
	movq	(%rax), %rax
	addq	$16, %rax
.Ltmp24491:
	movq	%r15, 40(%rsp)
	vmovups	%xmm1, 24(%rsp)
	movq	%rax, 16(%rsp)
	vmovups	%xmm0, (%rsp)
	leaq	208(%rsp), %rdi
	movq	%rbp, %rsi
	movq	192(%rsp), %r8
	movq	184(%rsp), %r9
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>
.Ltmp24492:
	vmovsd	216(%rsp), %xmm0
	movq	208(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB1316_33
	movq	64(%rsp), %r15
	movq	%r15, %rsi
	cmpq	$6, %r15
	jb	.LBB1316_26
	movq	80(%rsp), %rsi
.LBB1316_26:
	movq	200(%rsp), %rax
	decq	%rsi
	movq	(%rax), %rdi
	addq	%rbx, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1316_47
	leaq	72(%rsp), %rax
	cmpq	$6, %r15
	jb	.LBB1316_21
	movq	72(%rsp), %rax
	jmp	.LBB1316_21
.LBB1316_29:
	movq	48(%rsp), %rax
	vmovups	64(%rsp), %ymm0
	movq	96(%rsp), %rcx
	leaq	888(%r15), %rsi
	leaq	112(%rsp), %rdi
	movq	56(%rax), %rax
	movq	(%rax), %rdx
	movq	%rcx, 240(%rsp)
	leaq	208(%rsp), %rcx
	vmovups	%ymm0, 208(%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::parallel::minted_row::<purrdf_core::ir::term::TermId>
	movq	160(%rsp), %r14
	movq	16(%r14), %rbx
	cmpq	(%r14), %rbx
	jne	.LBB1316_31
.Ltmp24496:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.Ltmp24497:
.LBB1316_31:
	movq	8(%r14), %rax
	movq	144(%rsp), %rdx
	leaq	(%rbx,%rbx,4), %rcx
	leaq	1272(%r15), %rdi
	incq	%rbx
	movq	%r15, %rsi
	movq	%rdx, 32(%rax,%rcx,8)
	movl	$1, %edx
	vmovups	112(%rsp), %ymm0
	vmovups	%ymm0, (%rax,%rcx,8)
	movl	$1, %ecx
	movq	%rbx, 16(%r14)
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::settle::<purrdf_core::ir::dataset::RdfDataset>
	movq	152(%rsp), %rax
	movq	$-1, (%rax)
.LBB1316_32:
	addq	$312, %rsp
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
	vzeroupper
	retq
.LBB1316_33:
	.cfi_def_cfa_offset 368
	vmovups	224(%rsp), %zmm1
	vmovups	240(%rsp), %zmm2
	movq	152(%rsp), %rcx
	vmovups	%zmm2, 32(%rcx)
	vmovups	%zmm1, 16(%rcx)
	movq	%rax, (%rcx)
	movq	64(%rsp), %rax
	vmovlps	%xmm0, 8(%rcx)
	cmpq	$6, %rax
	jb	.LBB1316_32
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	72(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1316_36
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1316_36:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1316_42
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1316_36
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB1316_39:
	cmpq	%rax, %rsi
	jge	.LBB1316_41
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1316_39
.LBB1316_41:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1316_42:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB1316_32
.LBB1316_43:
.Ltmp24502:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.750(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	vzeroupper
	callq	*%r8
.Ltmp24503:
	jmp	.LBB1316_48
.LBB1316_44:
.Ltmp24483:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	leaq	216(%rsp), %r14
	xorl	%esi, %esi
	movq	%rbx, %rdx
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp24484:
	vmovups	208(%rsp), %ymm0
	movq	240(%rsp), %rax
	movq	%rax, 144(%rsp)
	vmovups	%ymm0, 112(%rsp)
	jmp	.LBB1316_9
.LBB1316_46:
.Ltmp24489:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.748(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	vzeroupper
	callq	*%rcx
.Ltmp24490:
	jmp	.LBB1316_48
.LBB1316_47:
.Ltmp24494:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.749(%rip), %rdx
	callq	*%rax
.Ltmp24495:
.LBB1316_48:
	ud2
.LBB1316_49:
.Ltmp24485:
	movq	208(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$6, %r15
	jae	.LBB1316_53
	jmp	.LBB1316_60
.LBB1316_50:
.Ltmp24498:
	movq	%rax, %rbx
.Ltmp24499:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::parallel::MintedRow>
.Ltmp24500:
	jmp	.LBB1316_60
.LBB1316_51:
.Ltmp24501:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1316_52:
.Ltmp24488:
	movq	112(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$5, %r15
	jbe	.LBB1316_60
.LBB1316_53:
	movq	(%r14), %rdi
	jmp	.LBB1316_59
.LBB1316_54:
.Ltmp24493:
	movq	64(%rsp), %r15
	jmp	.LBB1316_56
.LBB1316_55:
.Ltmp24504:
.LBB1316_56:
	movq	%rax, %rbx
	cmpq	$6, %r15
	jb	.LBB1316_60
	movq	72(%rsp), %rdi
.LBB1316_59:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1316_60:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1316:
purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}:
.Lfunc_begin1317:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception880
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
	subq	$296, %rsp
	.cfi_def_cfa_offset 352
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpb	$1, 1432(%rdx)
	movq	%r8, %r13
	movq	%rcx, %rbp
	movq	%rdx, %r15
	movq	%rdi, %r12
	jne	.LBB1317_7
	cmpb	$0, 1424(%r15)
	jne	.LBB1317_5
	movq	40(%r13), %rax
	movq	1392(%r15), %rcx
	testq	%rcx, %rcx
	je	.LBB1317_6
	movq	16(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1317_6
	movb	$1, 1424(%r15)
.LBB1317_5:
	movq	$-1, (%r12)
	jmp	.LBB1317_32
.LBB1317_6:
	movq	%rax, 1400(%r15)
.LBB1317_7:
	movq	(%rsi), %rax
	movq	%rsi, 32(%rsp)
	movq	(%rax), %rbx
	movq	$1, 192(%rsp)
	cmpq	$5, %rbx
	jae	.LBB1317_44
	vmovups	200(%rsp), %xmm0
	movq	224(%rsp), %rax
	movq	192(%rsp), %rdx
	movq	216(%rsp), %rcx
	movq	%rax, 128(%rsp)
	movq	%rdx, 96(%rsp)
	movq	%rcx, 120(%rsp)
	vmovups	%xmm0, 104(%rsp)
	testq	%rbx, %rbx
	je	.LBB1317_10
.LBB1317_9:
	movl	$2, %eax
	jmp	.LBB1317_11
.LBB1317_10:
	movl	$-1, %eax
.LBB1317_11:
	leaq	104(%rsp), %r14
	movl	%eax, 192(%rsp)
	movq	%r15, 40(%rsp)
	movq	%rbx, 200(%rsp)
.Ltmp24508:
	leaq	96(%rsp), %rdi
	leaq	192(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp24509:
	movq	32(%rsp), %rbx
	vmovups	96(%rsp), %ymm0
	movq	128(%rsp), %rax
	leaq	56(%rsp), %rdi
	movq	8(%rbx), %rcx
	movq	%rax, 80(%rsp)
	vmovups	%ymm0, 48(%rsp)
	movq	48(%rsp), %r15
	movq	(%rcx), %rdx
	movq	%rcx, 184(%rsp)
	movq	%r15, %rax
	cmpq	$6, %r15
	jb	.LBB1317_14
	movq	56(%rsp), %rdi
	movq	64(%rsp), %rax
.LBB1317_14:
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB1317_43
	movq	(%r13), %rax
	movq	16(%r13), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %rdx
	jne	.LBB1317_46
	movq	%rbp, 152(%rsp)
	movq	%r12, 144(%rsp)
	cmpq	$5, %rax
	jb	.LBB1317_18
	movq	8(%r13), %rsi
	jmp	.LBB1317_19
.LBB1317_18:
	leaq	8(%r13), %rsi
.LBB1317_19:
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	40(%rsp), %r15
	shlq	$3, %rdx
	vzeroupper
	callq	*%rax
	movq	24(%rbx), %rax
	movq	1264(%r15), %r14
	cmpq	%r14, %rax
	cmovbq	%rax, %r14
	testq	%r14, %r14
	je	.LBB1317_29
	movq	32(%rsp), %rax
	movq	1256(%r15), %r12
	movq	56(%r13), %rcx
	movq	64(%r13), %rsi
	xorl	%r13d, %r13d
	movq	16(%rax), %rbp
	movq	32(%rax), %rdx
	movq	40(%rax), %rbx
	addq	$16, %r12
	movq	%rcx, 176(%rsp)
	movq	%rsi, 168(%rsp)
	addq	$16, %rbp
	movq	%rdx, 160(%rsp)
	jmp	.LBB1317_22
	.p2align	4
.LBB1317_21:
	movq	40(%rsp), %r15
	incq	%r13
	addq	$24, %r12
	addq	$120, %rbp
	vmovlps	%xmm0, (%rax,%rdi,8)
	cmpq	%r13, %r14
	je	.LBB1317_29
.LBB1317_22:
	movq	160(%rsp), %rax
	movq	-8(%r12), %rdx
	movq	(%r12), %rcx
	vmovups	8(%rax), %xmm0
	movq	(%rbx), %rax
	addq	$16, %rax
.Ltmp24513:
	movq	%r15, 24(%rsp)
	movq	%rax, 16(%rsp)
	vmovups	%xmm0, (%rsp)
	leaq	192(%rsp), %rdi
	movq	%rbp, %rsi
	movq	176(%rsp), %r8
	movq	168(%rsp), %r9
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
.Ltmp24514:
	vmovsd	200(%rsp), %xmm0
	movq	192(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB1317_33
	movq	48(%rsp), %r15
	movq	%r15, %rsi
	cmpq	$6, %r15
	jb	.LBB1317_26
	movq	64(%rsp), %rsi
.LBB1317_26:
	movq	184(%rsp), %rax
	decq	%rsi
	movq	(%rax), %rdi
	addq	%r13, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1317_47
	leaq	56(%rsp), %rax
	cmpq	$6, %r15
	jb	.LBB1317_21
	movq	56(%rsp), %rax
	jmp	.LBB1317_21
.LBB1317_29:
	movq	32(%rsp), %rax
	vmovups	48(%rsp), %ymm0
	movq	80(%rsp), %rcx
	leaq	888(%r15), %rsi
	leaq	96(%rsp), %rdi
	movq	56(%rax), %rax
	movq	(%rax), %rdx
	movq	%rcx, 224(%rsp)
	leaq	192(%rsp), %rcx
	vmovups	%ymm0, 192(%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::parallel::minted_row::<purrdf_core::ir::term::TermId>
	movq	152(%rsp), %r14
	movq	16(%r14), %rbx
	cmpq	(%r14), %rbx
	jne	.LBB1317_31
.Ltmp24518:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.Ltmp24519:
.LBB1317_31:
	movq	8(%r14), %rax
	movq	128(%rsp), %rdx
	leaq	(%rbx,%rbx,4), %rcx
	leaq	1272(%r15), %rdi
	incq	%rbx
	movq	%r15, %rsi
	movq	%rdx, 32(%rax,%rcx,8)
	movl	$1, %edx
	vmovups	96(%rsp), %ymm0
	vmovups	%ymm0, (%rax,%rcx,8)
	movl	$1, %ecx
	movq	%rbx, 16(%r14)
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::settle::<purrdf_core::ir::dataset::RdfDataset>
	movq	144(%rsp), %rax
	movq	$-1, (%rax)
.LBB1317_32:
	addq	$296, %rsp
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
	vzeroupper
	retq
.LBB1317_33:
	.cfi_def_cfa_offset 352
	vmovups	208(%rsp), %zmm1
	vmovups	224(%rsp), %zmm2
	movq	144(%rsp), %rcx
	vmovups	%zmm2, 32(%rcx)
	vmovups	%zmm1, 16(%rcx)
	movq	%rax, (%rcx)
	movq	48(%rsp), %rax
	vmovlps	%xmm0, 8(%rcx)
	cmpq	$6, %rax
	jb	.LBB1317_32
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	56(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1317_36
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1317_36:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1317_42
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1317_36
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rsi
	negq	%rsi
	lock		xaddq	%rsi, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rsi
	setns	%al
	addq	%rdx, %rax
	subq	%rcx, %rsi
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rsi
	movq	(%rcx), %rax
	.p2align	4
.LBB1317_39:
	cmpq	%rax, %rsi
	jge	.LBB1317_41
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1317_39
.LBB1317_41:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1317_42:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB1317_32
.LBB1317_43:
.Ltmp24524:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.750(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	vzeroupper
	callq	*%r8
.Ltmp24525:
	jmp	.LBB1317_48
.LBB1317_44:
.Ltmp24505:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	192(%rsp), %rdi
	leaq	200(%rsp), %r14
	xorl	%esi, %esi
	movq	%rbx, %rdx
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp24506:
	vmovups	192(%rsp), %ymm0
	movq	224(%rsp), %rax
	movq	%rax, 128(%rsp)
	vmovups	%ymm0, 96(%rsp)
	jmp	.LBB1317_9
.LBB1317_46:
.Ltmp24511:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.748(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	vzeroupper
	callq	*%rcx
.Ltmp24512:
	jmp	.LBB1317_48
.LBB1317_47:
.Ltmp24516:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.749(%rip), %rdx
	callq	*%rax
.Ltmp24517:
.LBB1317_48:
	ud2
.LBB1317_49:
.Ltmp24507:
	movq	192(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$6, %r15
	jae	.LBB1317_53
	jmp	.LBB1317_60
.LBB1317_50:
.Ltmp24520:
	movq	%rax, %rbx
.Ltmp24521:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::parallel::MintedRow>
.Ltmp24522:
	jmp	.LBB1317_60
.LBB1317_51:
.Ltmp24523:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1317_52:
.Ltmp24510:
	movq	96(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$5, %r15
	jbe	.LBB1317_60
.LBB1317_53:
	movq	(%r14), %rdi
	jmp	.LBB1317_59
.LBB1317_54:
.Ltmp24515:
	movq	48(%rsp), %r15
	jmp	.LBB1317_56
.LBB1317_55:
.Ltmp24526:
.LBB1317_56:
	movq	%rax, %rbx
	cmpq	$6, %r15
	jb	.LBB1317_60
	movq	56(%rsp), %rdi
.LBB1317_59:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1317_60:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1317:
purrdf_sparql_eval::binop::eval_application_yielding::<purrdf_core::ir::dataset::RdfDataset, &mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset>>::{closure#0}::{closure#0}:
.Lfunc_begin1361:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception923
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
	subq	$392, %rsp
	.cfi_def_cfa_offset 448
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rsi), %rax
	movq	%rsi, 8(%rsp)
	movq	24(%rdx), %r12
	movq	%rdi, 24(%rsp)
	leaq	256(%rsp), %rdi
	movq	%rcx, 360(%rsp)
	movq	%rdx, %r15
	movq	24(%rax), %rsi
	movq	%rax, 376(%rsp)
	addq	$16, %rsi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)
	movq	32(%r12), %r14
	testq	%r14, %r14
	je	.LBB1361_5
	movq	24(%r12), %r13
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rbp
	shlq	$4, %r14
	leaq	256(%rsp), %rbx
	addq	%r13, %r14
	.p2align	4
.LBB1361_2:
	movq	(%r13), %rax
	lock		incq	(%rax)
	jle	.LBB1361_101
	movq	8(%r13), %rdx
	movq	(%r13), %rsi
.Ltmp26337:
	movq	%rbx, %rdi
	callq	*%rbp
.Ltmp26338:
	addq	$16, %r13
	cmpq	%r14, %r13
	jne	.LBB1361_2
.LBB1361_5:
	vmovups	280(%rsp), %ymm1
	vmovups	256(%rsp), %ymm0
	movq	malloc@GOTPCREL(%rip), %r14
	movl	$72, %edi
	vmovups	%ymm1, 200(%rsp)
	vmovups	%ymm0, 176(%rsp)
	movq	$1, 160(%rsp)
	movq	$1, 168(%rsp)
	vzeroupper
	callq	*%r14
	testq	%rax, %rax
	je	.LBB1361_99
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB1361_8
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB1361_8:
	addq	$16, %r12
	.p2align	4
.LBB1361_9:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1361_15
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1361_9
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$72, (%rcx)
	movl	$72, %ecx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	$72, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1361_12:
	cmpq	%rax, %rcx
	jle	.LBB1361_14
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1361_12
.LBB1361_14:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1361_15:
	vmovups	160(%rsp), %zmm0
	movq	224(%rsp), %rax
	movq	%r13, %rdx
	addq	$16, %rdx
	movq	%r13, 56(%rsp)
	movq	%rax, 64(%r13)
	vmovups	%zmm0, (%r13)
.Ltmp26343:
	movq	purrdf_sparql_eval::binop::right_to_out_map@GOTPCREL(%rip), %rax
	leaq	64(%rsp), %rdi
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.Ltmp26344:
	lock		incq	(%r13)
	jle	.LBB1361_101
	movq	16(%r15), %rcx
	movq	%r13, 384(%rsp)
	movq	%r13, (%rsp)
	testq	%rcx, %rcx
	je	.LBB1361_55
	movq	8(%r15), %rax
	movq	8(%rsp), %rdx
	movq	%rcx, 40(%rsp)
	movq	%rax, 368(%rsp)
	movq	8(%rdx), %rax
	movq	%rax, 48(%rsp)
	leaq	(,%rcx,8), %rax
	leaq	(%rax,%rax,4), %rbx
	movq	%rbx, %rdi
	callq	*%r14
	movq	%rax, 32(%rsp)
	testq	%rax, %rax
	je	.LBB1361_100
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	leaq	(%rbx,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%rbx, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%rbx, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB1361_21
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1361_21:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1361_27
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1361_21
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbx, (%rdx)
	movq	%rbx, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%rbx), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%rbx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1361_24:
	cmpq	%rax, %rdx
	jle	.LBB1361_26
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1361_24
.LBB1361_26:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1361_27:
	movq	40(%rsp), %rax
	movq	48(%rsp), %rcx
	movq	32(%rsp), %rdx
	movq	72(%rsp), %r13
	movq	80(%rsp), %rbp
	leaq	120(%rsp), %r14
	xorl	%r12d, %r12d
	movq	%rax, 88(%rsp)
	leaq	8(%rcx), %rax
	movq	%rdx, 96(%rsp)
	movq	%rax, 16(%rsp)
	jmp	.LBB1361_29
	.p2align	4
.LBB1361_28:
	vmovups	112(%rsp), %ymm0
	movq	144(%rsp), %rax
	movq	32(%rsp), %rcx
	leaq	(%r12,%r12,4), %rdx
	incq	%r12
	movq	%rax, 32(%rcx,%rdx,8)
	movq	%rax, 192(%rsp)
	vmovups	%ymm0, (%rcx,%rdx,8)
	movq	40(%rsp), %rcx
	vmovups	%ymm0, 160(%rsp)
	cmpq	%rcx, %r12
	je	.LBB1361_56
.LBB1361_29:
	movq	376(%rsp), %rax
	movq	(%rsp), %rcx
	movq	24(%rax), %rax
	movq	32(%rcx), %rbx
	movq	32(%rax), %r15
	movq	$1, 160(%rsp)
	cmpq	$5, %rbx
	jae	.LBB1361_53
	vmovups	168(%rsp), %xmm0
	movq	192(%rsp), %rax
	movq	160(%rsp), %rdx
	movq	184(%rsp), %rcx
	movq	%rax, 288(%rsp)
	movq	%rdx, 256(%rsp)
	movq	%rcx, 280(%rsp)
	vmovups	%xmm0, 264(%rsp)
	testq	%rbx, %rbx
	je	.LBB1361_32
.LBB1361_31:
	movl	$2, %eax
	jmp	.LBB1361_33
	.p2align	4
.LBB1361_32:
	movl	$-1, %eax
.LBB1361_33:
	movl	%eax, 160(%rsp)
	movq	%rbx, 168(%rsp)
.Ltmp26349:
	leaq	256(%rsp), %rdi
	leaq	160(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp26350:
	vmovups	256(%rsp), %ymm0
	movq	288(%rsp), %rax
	movq	%r14, %rdi
	movq	%rax, 144(%rsp)
	vmovups	%ymm0, 112(%rsp)
	movq	112(%rsp), %rdx
	cmpq	$6, %rdx
	jb	.LBB1361_36
	movq	120(%rsp), %rdi
	movq	128(%rsp), %rdx
.LBB1361_36:
	decq	%rdx
	cmpq	%rdx, %r15
	ja	.LBB1361_89
	movq	48(%rsp), %rcx
	movq	(%rcx), %rax
	movq	16(%rcx), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %r15
	jne	.LBB1361_90
	movq	16(%rsp), %rsi
	cmpq	$5, %rax
	jb	.LBB1361_40
	movq	16(%rsp), %rax
	movq	(%rax), %rsi
.LBB1361_40:
	shlq	$3, %r15
	movq	368(%rsp), %rax
	leaq	(%r12,%r12,4), %rcx
	movq	%r15, %rdx
	movq	memcpy@GOTPCREL(%rip), %r15
	leaq	(%rax,%rcx,8), %rbx
	vzeroupper
	callq	*%r15
	movq	(%rbx), %rax
	decq	%rax
	cmpq	$5, %rax
	jb	.LBB1361_42
	movq	16(%rbx), %rax
	movq	8(%rbx), %rbx
	decq	%rax
	testq	%rax, %rax
	jne	.LBB1361_43
	jmp	.LBB1361_28
	.p2align	4
.LBB1361_42:
	addq	$8, %rbx
	testq	%rax, %rax
	je	.LBB1361_28
.LBB1361_43:
	shlq	$3, %rax
	xorl	%edi, %edi
	jmp	.LBB1361_46
	.p2align	4
.LBB1361_44:
	movl	4(%rbx,%rdi,8), %esi
	movl	%ecx, (%r8,%rdx,8)
	movl	%esi, 4(%r8,%rdx,8)
.LBB1361_45:
	incq	%rdi
	addq	$-8, %rax
	je	.LBB1361_28
.LBB1361_46:
	movl	(%rbx,%rdi,8), %ecx
	cmpl	$2, %ecx
	je	.LBB1361_45
	cmpq	%rbp, %rdi
	jae	.LBB1361_95
	movq	112(%rsp), %rsi
	movq	%rsi, %r8
	cmpq	$6, %rsi
	jb	.LBB1361_50
	movq	128(%rsp), %r8
.LBB1361_50:
	movq	(%r13,%rdi,8), %rdx
	decq	%r8
	cmpq	%r8, %rdx
	jae	.LBB1361_94
	movq	%r14, %r8
	cmpq	$6, %rsi
	jb	.LBB1361_44
	movq	120(%rsp), %r8
	jmp	.LBB1361_44
.LBB1361_53:
.Ltmp26346:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	xorl	%esi, %esi
	movq	%rbx, %rdx
	xorl	%ecx, %ecx
	vzeroupper
	callq	*%rax
.Ltmp26347:
	vmovups	160(%rsp), %ymm0
	movq	192(%rsp), %rax
	movq	%rax, 288(%rsp)
	vmovups	%ymm0, 256(%rsp)
	jmp	.LBB1361_31
.LBB1361_55:
	movq	$0, 88(%rsp)
	movq	$8, 96(%rsp)
.LBB1361_56:
	movq	8(%rsp), %rsi
	movq	88(%rsp), %rax
	movq	96(%rsp), %r8
	movq	(%rsp), %r13
	movq	%rcx, 336(%rsp)
	movq	16(%rsi), %rbx
	movq	%rax, 320(%rsp)
	movq	%r8, 328(%rsp)
	movq	%r13, 344(%rsp)
	movq	(%rbx), %rsi
.Ltmp26364:
	movq	360(%rsp), %rcx
	leaq	160(%rsp), %rdi
	leaq	320(%rsp), %rdx
	vzeroupper
	callq	<&mut purrdf_sparql_eval::eval::RowConsumer<purrdf_core::ir::dataset::RdfDataset> as purrdf_sparql_eval::eval::RowDelivery<purrdf_core::ir::dataset::RdfDataset>>::deliver
.Ltmp26365:
	cmpq	$-1, 160(%rsp)
	je	.LBB1361_72
	vmovups	160(%rsp), %zmm0
	vmovups	192(%rsp), %zmm1
	movq	24(%rsp), %rbx
	vmovups	%zmm1, 32(%rbx)
	vmovups	%zmm0, (%rbx)
.Ltmp26369:
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp26370:
	movq	64(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB1361_69
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	72(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1361_62
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1361_62:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1361_68
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1361_62
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1361_65:
	cmpq	%rax, %rdx
	jge	.LBB1361_67
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1361_65
.LBB1361_67:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1361_68:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1361_69:
	lock		decq	(%r13)
	jne	.LBB1361_71
.LBB1361_70:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB1361_71:
	movq	%rbx, %rax
	addq	$392, %rsp
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
	vzeroupper
	retq
.LBB1361_72:
	.cfi_def_cfa_offset 448
	movq	8(%rsp), %rdx
	movq	$-1, %rcx
	movl	$2, %esi
	movq	24(%rdx), %r15
	movq	(%r15), %rax
	addq	336(%rsp), %rax
	cmovaeq	%rax, %rcx
	xorl	%edi, %edi
	movq	%rcx, (%r15)
	vmovups	320(%rsp), %ymm0
	movq	32(%rdx), %r14
	leaq	8(%r14), %r8
	leaq	16(%r14), %r13
	movq	%r14, %rcx
	vmovups	%ymm0, 160(%rsp)
	movq	(%r14), %rdx
	movq	8(%r14), %rax
	decq	%rdx
	cmpq	$3, %rdx
	setae	%dil
	cmovbq	%r8, %rax
	cmovaeq	%r13, %rcx
	cmovaeq	%rdx, %rsi
	shll	$4, %edi
	movq	(%r14,%rdi), %rbp
	leaq	-1(%rbp), %r12
	cmpq	%rsi, %r12
	je	.LBB1361_91
.LBB1361_73:
	vmovups	160(%rsp), %ymm0
	shlq	$5, %r12
	incq	%rbp
	vmovups	%ymm0, (%rax,%r12)
	movq	%rbp, (%rcx)
	movq	(%rbx), %rax
	movq	(%rsp), %rbx
	cmpb	$0, 16(%rax)
	je	.LBB1361_75
.LBB1361_74:
	movb	$1, %al
	jmp	.LBB1361_78
.LBB1361_75:
	movq	8(%rsp), %rax
	movq	40(%rax), %rax
	cmpl	$1, (%rax)
	jne	.LBB1361_77
	movq	(%r15), %rcx
	cmpq	8(%rax), %rcx
	jae	.LBB1361_74
.LBB1361_77:
	xorl	%eax, %eax
.LBB1361_78:
	movq	24(%rsp), %rcx
	movb	%al, 8(%rcx)
	movq	$-1, (%rcx)
	movq	64(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB1361_88
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	72(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1361_81
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1361_81:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1361_87
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1361_81
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1361_84:
	cmpq	%rax, %rdx
	jge	.LBB1361_86
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1361_84
.LBB1361_86:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1361_87:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1361_88:
	lock		decq	(%rbx)
	movq	24(%rsp), %rbx
	je	.LBB1361_70
	jmp	.LBB1361_71
.LBB1361_89:
.Ltmp26356:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	movq	(%rsp), %r13
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.451(%rip), %rcx
	xorl	%edi, %edi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp26357:
	jmp	.LBB1361_101
.LBB1361_90:
.Ltmp26352:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rax
	movq	(%rsp), %r13
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.448(%rip), %rdx
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.Ltmp26353:
	jmp	.LBB1361_101
.LBB1361_91:
.Ltmp26372:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::solution::SolutionSeq; 2]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r14, %rdi
	movq	%r8, 16(%rsp)
	vzeroupper
	callq	*%rax
.Ltmp26373:
	cmpq	$4, (%r14)
	jb	.LBB1361_97
	movq	8(%r14), %rax
	jmp	.LBB1361_98
.LBB1361_94:
	movq	%rdx, %rdi
	movq	%r8, %rbp
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.450(%rip), %rdx
	jmp	.LBB1361_96
.LBB1361_95:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.449(%rip), %rdx
.LBB1361_96:
.Ltmp26354:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	(%rsp), %r13
	movq	%rbp, %rsi
	callq	*%rax
.Ltmp26355:
	jmp	.LBB1361_101
.LBB1361_97:
	movq	16(%rsp), %rax
	movq	%r14, %r13
.LBB1361_98:
	movq	%r13, %rcx
	jmp	.LBB1361_73
.LBB1361_99:
.Ltmp26381:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	176(%rsp), %rbx
	callq	*%rax
.Ltmp26382:
	jmp	.LBB1361_101
.LBB1361_100:
.Ltmp26359:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp26360:
.LBB1361_101:
	ud2
.LBB1361_102:
.Ltmp26374:
	movq	%rax, %r14
.Ltmp26375:
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp26376:
	movq	(%rsp), %r13
	jmp	.LBB1361_126
.LBB1361_104:
.Ltmp26377:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1361_105:
.Ltmp26348:
	movq	%rax, %r14
	movq	160(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1361_107
	leaq	168(%rsp), %rcx
	jmp	.LBB1361_116
.LBB1361_108:
.Ltmp26371:
	movq	%rax, %r14
	jmp	.LBB1361_126
.LBB1361_109:
.Ltmp26361:
	movq	%rax, %r14
	jmp	.LBB1361_124
.LBB1361_110:
.Ltmp26366:
	movq	%rax, %r14
.Ltmp26367:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp26368:
	jmp	.LBB1361_126
.LBB1361_111:
.Ltmp26345:
	movq	%rax, %r14
	jmp	.LBB1361_128
.LBB1361_112:
.Ltmp26383:
	movq	%rax, %r14
.Ltmp26384:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp26385:
	jmp	.LBB1361_130
.LBB1361_113:
.Ltmp26386:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1361_114:
.Ltmp26351:
	movq	%rax, %r14
	movq	256(%rsp), %rax
	cmpq	$5, %rax
	jbe	.LBB1361_107
	leaq	264(%rsp), %rcx
.LBB1361_116:
	movq	(%rcx), %rdi
	movq	(%rsp), %r13
	jmp	.LBB1361_122
.LBB1361_107:
	movq	(%rsp), %r13
	jmp	.LBB1361_123
.LBB1361_118:
.Ltmp26339:
	movq	%rax, %r14
.Ltmp26340:
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)
.Ltmp26341:
	jmp	.LBB1361_130
.LBB1361_119:
.Ltmp26342:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1361_120:
.Ltmp26358:
	movq	%rax, %r14
	movq	112(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1361_123
	movq	120(%rsp), %rdi
.LBB1361_122:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1361_123:
	leaq	88(%rsp), %rdi
	movq	%r12, 104(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB1361_124:
	lock		decq	(%r13)
	jne	.LBB1361_126
	#MEMBARRIER
.Ltmp26362:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	callq	*%rax
.Ltmp26363:
.LBB1361_126:
	movq	64(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1361_128
	movq	72(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB1361_128:
	lock		decq	(%r13)
	jne	.LBB1361_130
	#MEMBARRIER
.Ltmp26378:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	56(%rsp), %rdi
	callq	*%rax
.Ltmp26379:
.LBB1361_130:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB1361_131:
.Ltmp26380:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1361:
purrdf_sparql_eval::modifier::aggregate_numeric_cost:
.Lfunc_begin1927:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1283
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
	subq	$264, %rsp
	.cfi_def_cfa_offset 320
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	%rdx, %rax
	shlq	$4, %rax
	movq	%rcx, %r12
	shrq	$32, %r12
	movq	%rcx, %r14
	movq	%rdx, %r13
	movq	%rsi, %r15
	leaq	(%rax,%rax,4), %rbx
	leaq	(%rsi,%rbx), %rbp
	testq	%rdx, %rdx
	je	.LBB1927_9
	movq	%r15, %rax
	jmp	.LBB1927_3
	.p2align	4
.LBB1927_2:
	addq	$80, %rax
	cmpq	%rbp, %rax
	je	.LBB1927_9
.LBB1927_3:
	cmpq	$0, (%rax)
	js	.LBB1927_2
	cmpq	$20, 16(%rax)
	jb	.LBB1927_2
	movq	(%rdi), %rax
	leal	-3(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB1927_53
	cmpl	$1, %eax
	je	.LBB1927_99
	xorl	%ebx, %ebx
	cmpl	$2, %eax
	je	.LBB1927_10
	jmp	.LBB1927_101
.LBB1927_9:
	cmpl	$2, (%rdi)
	movb	$1, %bl
	jne	.LBB1927_100
.LBB1927_10:
	cmpl	$18, %r12d
	setne	%al
	orb	%r14b, %al
	testb	$1, %al
	jne	.LBB1927_12
	testl	$65280, %r14d
	sete	%al
	xorl	%ecx, %ecx
	testb	%al, %bl
	jne	.LBB1927_100
	testq	%r13, %r13
	je	.LBB1927_100
.LBB1927_14:
	movq	%rcx, 256(%rsp)
	movq	$0, 128(%rsp)
	xorl	%ebx, %ebx
	movq	$0, 24(%rsp)
	xorl	%esi, %esi
	xorl	%edi, %edi
	xorl	%eax, %eax
.LBB1927_15:
	addq	$80, %r15
	movq	%rax, 40(%rsp)
	movq	%rdi, 80(%rsp)
	movq	%rsi, 248(%rsp)
	.p2align	4
.LBB1927_16:
	cmpq	$0, -80(%r15)
	js	.LBB1927_20
	cmpq	$-1, -32(%r15)
	jne	.LBB1927_20
	movq	-40(%r15), %rsi
	cmpq	$33, %rsi
	jb	.LBB1927_20
	movq	-48(%r15), %rdi
	vmovdqu	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276(%rip), %ymm0
	movzbl	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276+32(%rip), %eax
	vpxor	(%rdi), %ymm0, %ymm0
	movzbl	32(%rdi), %ecx
	vmovd	%eax, %xmm1
	vmovd	%ecx, %xmm2
	vpternlogq	$246, %ymm2, %ymm1, %ymm0
	vptest	%ymm0, %ymm0
	je	.LBB1927_22
	.p2align	4
.LBB1927_20:
	movq	$0, 160(%rsp)
.LBB1927_21:
	leaq	-80(%r15), %rax
	addq	$80, %r15
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1927_16
	jmp	.LBB1927_45
.LBB1927_22:
	movq	<purrdf_xsd::datatype::XsdDatatype>::from_local@GOTPCREL(%rip), %rax
	addq	$-33, %rsi
	addq	$33, %rdi
	vzeroupper
	callq	*%rax
	cmpb	$-1, %al
	je	.LBB1927_20
	movzbl	%al, %ecx
	movq	-72(%r15), %rsi
	movq	-64(%r15), %rdx
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
	cmpb	$0, 160(%rsp)
	je	.LBB1927_21
	movq	40(%rsp), %rdx
	movl	196(%rsp), %eax
	movq	176(%rsp), %r11
	movq	184(%rsp), %rcx
	movq	248(%rsp), %rdi
	movq	$-1, %rsi
	movq	168(%rsp), %r8
	incq	%rdx
	movl	%eax, 52(%rsp)
	movq	%r11, %rax
	cmoveq	%rsi, %rdx
	subq	%rcx, %rax
	movl	$0, %esi
	cmovaeq	%rax, %rsi
	cmpq	%rdi, %rsi
	cmovbeq	%rdi, %rsi
	movq	80(%rsp), %rdi
	cmpq	%rdi, %rcx
	cmovaq	%rcx, %rdi
	testb	$1, 128(%rsp)
	je	.LBB1927_34
	movq	%rdx, %rax
	decq	%rax
	movq	%rdx, 40(%rsp)
	je	.LBB1927_128
	movabsq	$-2601111570856684097, %r10
	movq	%rax, %rdx
	movq	%rax, %r9
	shrq	$10, %r9
	mulxq	%r10, %rdx, %rdx
	movl	$10, %r10d
	movq	%r11, 240(%rsp)
	shrq	$33, %rdx
	cmpq	$9765625, %r9
	movl	$0, %r9d
	cmovael	%r10d, %r9d
	cmovbq	%rax, %rdx
	cmpq	$100000, %rdx
	jb	.LBB1927_28
	shrq	$5, %rdx
	movabsq	$755578637259143235, %rax
	orl	$5, %r9d
	mulxq	%rax, %rdx, %rdx
	shrq	$7, %rdx
.LBB1927_28:
	leal	393206(%rdx), %eax
	leal	524188(%rdx), %r11d
	movabsq	$2049638230412172401, %r10
	andl	%eax, %r11d
	leal	916504(%rdx), %eax
	addl	$514288, %edx
	andl	%eax, %edx
	xorl	%r11d, %edx
	shrl	$17, %edx
	leal	1(%rdx,%r9), %eax
	movq	$-1, %rdx
	movabsq	$-2049638230412172401, %r9
	addq	%rsi, %rax
	cmovbq	%rdx, %rax
	addq	%rdi, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %rdx
	mulxq	%r9, %rdx, %rdx
	movabsq	$-8198552921648689607, %r9
	movq	%rax, %r11
	imulq	%r9, %r11
	xorl	%r9d, %r9d
	shrq	$3, %rdx
	cmpq	%r10, %r11
	seta	%r9b
	addq	%rdx, %r9
	cmpq	$39, 32(%rsp)
	setb	%dl
	cmpq	$19, 16(%rsp)
	setb	%r11b
	testb	%r11b, %dl
	je	.LBB1927_32
	cmpq	$39, 240(%rsp)
	setb	%dl
	cmpq	$19, %rcx
	setb	%r10b
	testb	%r10b, %dl
	je	.LBB1927_32
	cmpq	$18, 80(%rsp)
	ja	.LBB1927_32
	cmpq	$38, %rax
	jbe	.LBB1927_41
.LBB1927_32:
	movq	16(%rsp), %r10
	movq	%r10, %rdx
	subq	%rcx, %rdx
	jae	.LBB1927_35
	subq	%r10, %rcx
	movabsq	$-2049638230412172401, %r11
	movq	$-1, %r10
	movq	%rcx, %rdx
	mulxq	%r11, %rcx, %rcx
	movq	8(%rsp), %rdx
	shrq	$3, %rcx
	incq	%rcx
	addq	%rcx, %rdx
	cmovbq	%r10, %rdx
	movq	%rdx, %rcx
	movq	%rdx, 8(%rsp)
	jmp	.LBB1927_36
.LBB1927_34:
	movl	192(%rsp), %eax
	movq	%r8, 8(%rsp)
	movq	%r11, 32(%rsp)
	movq	%rcx, 16(%rsp)
	movl	%eax, 4(%rsp)
	movq	%rdx, %rax
	jmp	.LBB1927_42
.LBB1927_35:
	movabsq	$-2049638230412172401, %rcx
	mulxq	%rcx, %rcx, %rcx
	movq	$-1, %rdx
	shrq	$3, %rcx
	incq	%rcx
	addq	%rcx, %r8
	cmovbq	%rdx, %r8
	movq	%r8, %rcx
.LBB1927_36:
	movq	%rcx, %rdx
	shrq	$62, %rdx
	jne	.LBB1927_43
	leaq	(,%rcx,4), %rdx
.LBB1927_38:
	movq	8(%rsp), %r10
	movabsq	$4611686018427387903, %r11
	cmpq	%r8, %r10
	cmovaq	%r10, %r8
	movq	$-1, %r10
	incq	%r8
	cmoveq	%r10, %r8
	cmpq	%r11, %r8
	ja	.LBB1927_44
	leaq	(,%r8,4), %r10
.LBB1927_40:
	addq	%r8, %rcx
	movq	24(%rsp), %r8
	movq	$-1, %r11
	cmovbq	%r11, %rcx
	addq	%r10, %rdx
	cmovbq	%r11, %rdx
	addq	%rcx, %rbx
	cmovbq	%r11, %rbx
	cmpq	%rdx, %r8
	cmovbeq	%rdx, %r8
	movq	%r8, 24(%rsp)
.LBB1927_41:
	movq	%rax, 32(%rsp)
	movq	40(%rsp), %rax
	movl	$-1, 4(%rsp)
	movq	%r9, 8(%rsp)
	movq	%rdi, 16(%rsp)
.LBB1927_42:
	movb	$1, %cl
	movq	%rcx, 128(%rsp)
	cmpq	%rbp, %r15
	jne	.LBB1927_15
	jmp	.LBB1927_123
.LBB1927_43:
	movq	$-1, %rdx
	jmp	.LBB1927_38
.LBB1927_44:
	movq	$-1, %r10
	jmp	.LBB1927_40
.LBB1927_45:
	testb	$1, 128(%rsp)
	movq	24(%rsp), %rdx
	je	.LBB1927_102
	movq	16(%rsp), %rsi
	movq	32(%rsp), %rbp
	cmpq	$18, %rsi
	ja	.LBB1927_49
.LBB1927_47:
	cmpq	$38, %rbp
	ja	.LBB1927_49
	movl	%r14d, %eax
	movq	%r12, %rcx
	andl	$1, %eax
	xorq	$18, %rcx
	orq	%rax, %rcx
	movl	%r14d, %eax
	andl	$65280, %eax
	orq	%rcx, %rax
	je	.LBB1927_102
.LBB1927_49:
	cmpb	$0, 256(%rsp)
	je	.LBB1927_103
	cmpq	%rbp, %rsi
	jae	.LBB1927_112
	movq	8(%rsp), %rax
	movq	%rdx, %r15
	testq	%rsi, %rsi
	je	.LBB1927_113
	incq	%rbp
	movq	$-1, %rcx
	cmoveq	%rcx, %rbp
	jmp	.LBB1927_113
.LBB1927_12:
	xorl	%ecx, %ecx
	testq	%r13, %r13
	jne	.LBB1927_14
.LBB1927_100:
	xorl	%ebx, %ebx
.LBB1927_101:
	xorl	%edx, %edx
.LBB1927_102:
	movq	%rbx, %rax
	addq	$264, %rsp
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
	vzeroupper
	retq
.LBB1927_53:
	.cfi_def_cfa_offset 320
	movq	<purrdf_xsd::datatype::XsdDatatype>::from_local@GOTPCREL(%rip), %r14
	addq	$80, %r15
	addq	$-80, %rbx
	leaq	88(%rsp), %r12
	xorl	%r13d, %r13d
	.p2align	4
.LBB1927_54:
	cmpq	$0, -80(%r15)
	js	.LBB1927_58
	cmpq	$-1, -32(%r15)
	jne	.LBB1927_58
	movq	-40(%r15), %rsi
	cmpq	$33, %rsi
	jb	.LBB1927_58
	movq	-48(%r15), %rdi
	vmovdqu	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276(%rip), %ymm1
	movzbl	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276+32(%rip), %eax
	vpxor	(%rdi), %ymm1, %ymm1
	movzbl	32(%rdi), %ecx
	vmovd	%eax, %xmm2
	vmovdqu	%ymm2, 128(%rsp)
	vmovd	%ecx, %xmm0
	vpternlogq	$246, %ymm0, %ymm2, %ymm1
	vptest	%ymm1, %ymm1
	je	.LBB1927_60
	.p2align	4
.LBB1927_58:
	movq	$0, 88(%rsp)
.LBB1927_59:
	leaq	-80(%r15), %rax
	addq	$80, %r15
	addq	$-80, %rbx
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1927_54
	jmp	.LBB1927_87
.LBB1927_60:
	addq	$-33, %rsi
	addq	$33, %rdi
	vzeroupper
	callq	*%r14
	cmpb	$-1, %al
	je	.LBB1927_58
	movzbl	%al, %ecx
	movq	-72(%r15), %rsi
	movq	-64(%r15), %rdx
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	cmpb	$0, 88(%rsp)
	je	.LBB1927_59
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movl	$128, %r13d
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1927_129
	movq	%rax, %r12
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	addq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %r13
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%rcx, %r13
	movabsq	$9223372036854775807, %rcx
	subq	$-128, %rax
	movq	%r13, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB1927_65
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB1927_65:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1927_71
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1927_65
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$128, (%rdx)
	movl	$128, %edx
	lock		xaddq	%rdx, (%rsi)
	subq	$-128, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
.LBB1927_68:
	cmpq	%rax, %rdx
	jle	.LBB1927_70
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1927_68
.LBB1927_70:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1927_71:
	vmovdqu	96(%rsp), %ymm0
	movq	$4, 56(%rsp)
	movq	%r12, 64(%rsp)
	movq	$1, 72(%rsp)
	vmovdqu	%ymm0, (%r12)
	testq	%rbx, %rbx
	je	.LBB1927_121
	movl	$1, %ebx
.LBB1927_73:
	addq	$80, %r15
.LBB1927_74:
	cmpq	$0, -80(%r15)
	js	.LBB1927_78
	cmpq	$-1, -32(%r15)
	jne	.LBB1927_78
	movq	-40(%r15), %rsi
	cmpq	$33, %rsi
	jb	.LBB1927_78
	movq	-48(%r15), %rdi
	vmovdqu	anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276(%rip), %ymm1
	movzbl	32(%rdi), %eax
	vpxor	(%rdi), %ymm1, %ymm1
	vmovd	%eax, %xmm0
	vpternlogq	$246, 128(%rsp), %ymm0, %ymm1
	vptest	%ymm1, %ymm1
	je	.LBB1927_80
.LBB1927_78:
	movq	$0, 160(%rsp)
.LBB1927_79:
	leaq	-80(%r15), %rax
	addq	$80, %r15
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1927_74
	jmp	.LBB1927_122
.LBB1927_80:
	movq	-72(%r15), %rax
	movq	-64(%r15), %r13
	addq	$-33, %rsi
	addq	$33, %rdi
	movq	%rax, 24(%rsp)
	vzeroupper
	callq	*%r14
	cmpb	$-1, %al
	je	.LBB1927_78
.Ltmp39192:
	movzbl	%al, %ecx
	movq	24(%rsp), %rsi
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	movq	%r13, %rdx
	callq	*%rax
.Ltmp39193:
	cmpb	$0, 160(%rsp)
	je	.LBB1927_79
	cmpq	56(%rsp), %rbx
	jne	.LBB1927_86
.Ltmp39195:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$32, %r8d
	leaq	56(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp39196:
	movq	64(%rsp), %r12
.LBB1927_86:
	leaq	168(%rsp), %rcx
	movq	%rbx, %rax
	shlq	$5, %rax
	incq	%rbx
	vmovdqu	(%rcx), %ymm0
	vmovdqu	%ymm0, (%r12,%rax)
	movq	%rbx, 72(%rsp)
	cmpq	%rbp, %r15
	jne	.LBB1927_73
	jmp	.LBB1927_122
.LBB1927_87:
	movl	$8, %r12d
	xorl	%ebx, %ebx
.LBB1927_88:
.Ltmp39198:
	movq	purrdf_xsd::exact::cost::compare_chain@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r12, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp39199:
	movq	%rax, %rbx
	testq	%r13, %r13
	je	.LBB1927_102
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$5, %r13
	movabsq	$9223372036854775807, %rcx
	movq	%rdx, %r14
	cmpq	%rcx, %r13
	cmovaeq	%rcx, %r13
	xorl	%edx, %edx
	cmpq	%r13, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%r13, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB1927_92
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB1927_92:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1927_98
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1927_92
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r13, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r13, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r13, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1927_95:
	cmpq	%rax, %rdx
	jge	.LBB1927_97
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1927_95
.LBB1927_97:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1927_98:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	movq	%r14, %rdx
	jmp	.LBB1927_102
.LBB1927_99:
	movb	$1, %cl
	testq	%r13, %r13
	jne	.LBB1927_14
	jmp	.LBB1927_100
.LBB1927_103:
	movq	%rdx, %r15
	movq	%r13, 176(%rsp)
	movq	$0, 184(%rsp)
	movw	$0, 160(%rsp)
.Ltmp39201:
	leaq	88(%rsp), %rdi
	leaq	160(%rsp), %rsi
	vzeroupper
	callq	purrdf_xsd::numeric::exact_path::shape_of (.llvm.9305710216504555276)
.Ltmp39202:
	cmpl	$1, 88(%rsp)
	jne	.LBB1927_125
	vmovups	96(%rsp), %ymm0
	leaq	160(%rsp), %rdi
	vmovups	%ymm0, 128(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
	movq	8(%rsp), %rax
	vmovups	128(%rsp), %ymm0
	movq	16(%rsp), %r13
	movl	4(%rsp), %edx
	movl	52(%rsp), %ecx
	leaq	88(%rsp), %rdi
	leaq	160(%rsp), %rsi
	movq	%rax, 88(%rsp)
	movq	purrdf_xsd::exact::cost::decimal_div@GOTPCREL(%rip), %rax
	movq	%rbp, 96(%rsp)
	movq	%r13, 104(%rsp)
	movl	%edx, 112(%rsp)
	movq	%r14, %rdx
	movl	%ecx, 116(%rsp)
	vmovups	%ymm0, 160(%rsp)
	vzeroupper
	callq	*%rax
	vmovdqu	128(%rsp), %ymm0
	movq	%r13, %rdi
	movq	%rax, %rcx
	movq	%rdx, %rsi
	testb	$1, %r14b
	je	.LBB1927_109
	vpextrq	$1, %xmm0, %rax
	movq	%rax, %rdx
	shrq	$62, %rdx
	jne	.LBB1927_127
	shlq	$2, %rax
.LBB1927_108:
	addq	%rdi, %rax
	movq	$-1, %r12
	cmovaeq	%rax, %r12
.LBB1927_109:
	xorl	%eax, %eax
	subq	%rdi, %rbp
	vextracti128	$1, %ymm0, %xmm0
	movq	$-1, %r8
	movabsq	$-8198552921648689607, %r9
	movabsq	$2049638230412172401, %r10
	cmovaeq	%rbp, %rax
	vmovq	%xmm0, %rdi
	addq	%rax, %rdi
	movabsq	$-2049638230412172401, %rax
	cmovbq	%r8, %rdi
	incq	%rdi
	cmoveq	%r8, %rdi
	addq	%r12, %rdi
	cmovbq	%r8, %rdi
	movq	%rdi, %rdx
	mulxq	%rax, %rdx, %rdx
	imulq	%rdi, %r9
	xorl	%eax, %eax
	shrq	$3, %rdx
	cmpq	%r10, %r9
	seta	%al
	addq	%rdx, %rax
	cmpq	%rdi, %r12
	jae	.LBB1927_117
	testq	%r12, %r12
	je	.LBB1927_118
	incq	%rdi
	movq	$-1, %rdx
	cmoveq	%rdx, %rdi
	jmp	.LBB1927_118
.LBB1927_112:
	movq	8(%rsp), %rax
	addq	$2, %rsi
	movq	$-1, %rbp
	movq	%rdx, %r15
	cmovaeq	%rsi, %rbp
.LBB1927_113:
	movl	$9, %ecx
	mulq	%rcx
	jo	.LBB1927_124
	cmpl	$0, 4(%rsp)
	jns	.LBB1927_116
	incq	%rbp
	movq	$-1, %rcx
	cmoveq	%rcx, %rbp
.LBB1927_116:
	addq	%rbp, %rax
	movq	$-1, %rdx
	cmovbq	%rdx, %rax
	movq	%rax, %rcx
	incq	%rcx
	cmoveq	%rdx, %rcx
	jmp	.LBB1927_120
.LBB1927_117:
	addq	$2, %r12
	cmovbq	%r8, %r12
	movq	%r12, %rdi
.LBB1927_118:
	movl	$9, %edx
	mulq	%rdx
	jo	.LBB1927_124
	incq	%rdi
	movq	$-1, %rdx
	cmoveq	%rdx, %rdi
	addq	%rdi, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %rdi
	incq	%rdi
	cmoveq	%rdx, %rdi
	addq	%rdi, %rcx
	cmovbq	%rdx, %rcx
	cmpq	%rax, %rsi
	cmovaq	%rsi, %rax
.LBB1927_120:
	addq	%rcx, %rbx
	movq	$-1, %rdx
	cmovbq	%rdx, %rbx
	cmpq	%rax, %r15
	movq	%r15, %rdx
	cmovbeq	%rax, %rdx
	jmp	.LBB1927_102
.LBB1927_121:
	movl	$1, %ebx
.LBB1927_122:
	movq	56(%rsp), %r13
	jmp	.LBB1927_88
.LBB1927_123:
	movq	24(%rsp), %rdx
	movq	16(%rsp), %rsi
	movq	32(%rsp), %rbp
	cmpq	$18, %rsi
	jbe	.LBB1927_47
	jmp	.LBB1927_49
.LBB1927_124:
	movq	$-1, %rcx
	movq	$-1, %rax
	jmp	.LBB1927_120
.LBB1927_125:
.Ltmp39203:
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.551(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.552(%rip), %rdx
	movl	$22, %esi
	callq	*%rax
.Ltmp39204:
	ud2
.LBB1927_127:
	movq	$-1, %rax
	jmp	.LBB1927_108
.LBB1927_128:
	movq	core::num::imp::int_log10::panic_for_nonpositive_argument@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.1110(%rip), %rdi
	callq	*%rax
.LBB1927_129:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$128, %esi
	callq	*%rax
.LBB1927_130:
.Ltmp39197:
	jmp	.LBB1927_132
.LBB1927_131:
.Ltmp39194:
.LBB1927_132:
	movq	56(%rsp), %rsi
	movq	%rax, %rbx
	testq	%rsi, %rsi
	je	.LBB1927_137
	movq	64(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	jmp	.LBB1927_136
.LBB1927_134:
.Ltmp39200:
	movq	%rax, %rbx
	testq	%r13, %r13
	je	.LBB1927_137
	shlq	$5, %r13
	movl	$8, %edx
	movq	%r12, %rdi
	movq	%r13, %rsi
.LBB1927_136:
	callq	__rustc::__rust_dealloc
.LBB1927_137:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1927_138:
.Ltmp39205:
	leaq	160(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1927:

purrdf_sparql_eval::eval::eval_node::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin904:
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
	leaq	.LJTI904_0(%rip), %rdx
	subl	$10, %eax
	cmovael	%eax, %ecx
	movslq	(%rdx,%rcx,4), %rcx
	addq	%rdx, %rcx
	jmpq	*%rcx
.LBB904_1:
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
	movq	%r10, %rcx
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_dedup::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB904_3:
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
	je	.LBB904_26
	vmovups	248(%rsp), %ymm0
	vmovups	272(%rsp), %ymm1
	jmp	.LBB904_25
.LBB904_5:
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB904_44
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_lateral::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB904_8:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
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
.LBB904_10:
	.cfi_def_cfa_offset 336
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB904_44
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_minus::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB904_13:
	.cfi_def_cfa_offset 336
	movq	8(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
	movq	16(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB904_44
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::binop::eval_join::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB904_16:
	.cfi_def_cfa_offset 336
	movq	72(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
	movq	80(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB904_44
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
.LBB904_19:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB904_44
	leaq	8(%rsi), %rdx
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::modifier::eval_graph::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB904_21:
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
	je	.LBB904_26
	vmovups	56(%rsp), %ymm0
	vmovups	80(%rsp), %ymm1
	jmp	.LBB904_25
.LBB904_23:
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
	je	.LBB904_26
	vmovups	152(%rsp), %ymm0
	vmovups	176(%rsp), %ymm1
.LBB904_25:
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
.LBB904_26:
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
.LBB904_27:
	.cfi_def_cfa_offset 336
	movq	56(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
	movq	16(%rsi), %rcx
	movq	24(%rsi), %r8
	movq	40(%rsi), %r9
	movq	48(%rsi), %rax
	movq	purrdf_sparql_eval::modifier::eval_group::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %rbx
	movq	%r10, 8(%rsp)
	movq	%rax, (%rsp)
	jmp	.LBB904_41
.LBB904_29:
	movq	72(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB904_44
	leaq	8(%rsi), %rdx
	movq	%r10, %r8
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	jmpq	*purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip)
.LBB904_31:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
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
.LBB904_33:
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
.LBB904_34:
	.cfi_def_cfa_offset 336
	movq	32(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB904_44
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
.LBB904_36:
	.cfi_def_cfa_offset 336
	movq	88(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
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
.LBB904_38:
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
.LBB904_39:
	.cfi_def_cfa_offset 336
	movq	88(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
	movq	96(%rsi), %rax
	movq	purrdf_sparql_eval::cdt_unfold::eval_unfold::<purrdf_core::ir::dataset::RdfDataset>@GOTPCREL(%rip), %rbx
	leaq	96(%rsi), %r9
	leaq	72(%rsi), %r8
	leaq	8(%rsi), %rcx
	movq	%r10, (%rsp)
	testq	%rax, %rax
	cmoveq	%rax, %r9
.LBB904_41:
	callq	*%rbx
	addq	$312, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.LBB904_42:
	.cfi_def_cfa_offset 336
	movq	24(%rsi), %rdx
	testq	%rdx, %rdx
	je	.LBB904_44
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
.LBB904_44:
	.cfi_def_cfa_offset 336
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522(%rip), %rdi
	leaq	anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522(%rip), %rdx
	movl	$48, %esi
	callq	*%rax
.Lfunc_end904:
purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin911:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception585
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
	subq	$2392, %rsp
	.cfi_def_cfa_offset 2448
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, 320(%rsp)
	leaq	1696(%rsp), %rdi
	movq	%r9, %r12
	movq	%r8, %r15
	movq	%rcx, %rbx
	movq	%rdx, %r13
	movq	%rsi, %r14
	callq	*%rax
.Ltmp10478:
	leaq	2272(%rsp), %rdi
	movq	%r13, %rsi
	movq	%r12, 40(%rsp)
	movq	%r12, %rdx
	movq	%r13, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp10479:
	cmpl	$1, 2272(%rsp)
	jne	.LBB911_26
	vmovdqu64	2288(%rsp), %zmm0
	vmovdqu64	2320(%rsp), %zmm1
	movq	320(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
.LBB911_3:
	movq	1768(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB911_13
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1776(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_6
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_6:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_12
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_6
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
.LBB911_9:
	cmpq	%rax, %rsi
	jge	.LBB911_11
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB911_9
.LBB911_11:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_12:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB911_13:
	movq	1696(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB911_23
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1704(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_16
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_16:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_22
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_16
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
.LBB911_19:
	cmpq	%rax, %rsi
	jge	.LBB911_21
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB911_19
.LBB911_21:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_22:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB911_23:
	movq	1792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_260
	lock		decq	(%rax)
	jne	.LBB911_260
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1792(%rsp), %rdi
	#MEMBARRIER
.LBB911_259:
	vzeroupper
	callq	*%rax
	jmp	.LBB911_260
.LBB911_26:
	vmovdqu64	2312(%rsp), %zmm1
	vmovdqu64	2280(%rsp), %zmm0
	vmovdqu64	%zmm1, 896(%rsp)
	vmovdqu64	%zmm0, 864(%rsp)
.Ltmp10480:
	leaq	528(%rsp), %rdi
	leaq	1696(%rsp), %rsi
	leaq	864(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp10481:
	cmpq	$-1, 528(%rsp)
	je	.LBB911_56
	vmovdqu	528(%rsp), %ymm0
	movb	$1, %al
	movq	%r14, 1320(%rsp)
	movl	%eax, 16(%rsp)
	vmovdqu	%ymm0, 464(%rsp)
.Ltmp10482:
	movq	40(%rsp), %r14
	movb	$1, %r13b
	movq	%r15, %rsi
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp10483:
	cmpb	$2, 472(%r14)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB911_35
	movq	616(%r14), %rax
	testq	%rax, %rax
	je	.LBB911_34
	cmpq	$-2, 24(%rax)
	jb	.LBB911_35
	cmpq	$-2, 32(%rax)
	jb	.LBB911_35
	cmpq	$-3, 48(%rax)
	jbe	.LBB911_35
.LBB911_34:
	movq	40(%rsp), %rax
	cmpq	$1025, 480(%rsp)
	movzbl	1234(%rax), %r14d
	setae	%al
	notb	%r14b
	andb	%al, %r14b
	jmp	.LBB911_36
.LBB911_35:
	xorl	%r14d, %r14d
.LBB911_36:
	movq	480(%rsp), %rbp
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp10484:
	movq	40(%rsp), %rsi
	movzbl	%r14b, %edx
	leaq	2072(%rsp), %rdi
	movq	%rbp, %rcx
	movb	$1, %r13b
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10485:
	movq	488(%rsp), %rsi
	addq	$16, %rsi
.Ltmp10486:
	leaq	1800(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp10487:
	movq	(%rbx), %rsi
	lock		incq	(%rsi)
	jle	.LBB911_882
	movq	8(%rbx), %rdx
.Ltmp10489:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	1800(%rsp), %rdi
	callq	*%rax
.Ltmp10490:
	vmovdqu	1824(%rsp), %ymm0
	vmovdqu	1800(%rsp), %ymm1
	movq	%rax, 1232(%rsp)
	movq	1816(%rsp), %rax
	movq	malloc@GOTPCREL(%rip), %r13
	movl	$72, %edi
	movq	%rax, 1152(%rsp)
	vmovdqu	%ymm0, 904(%rsp)
	vmovdqu	%ymm1, 880(%rsp)
	movq	$1, 864(%rsp)
	movq	$1, 872(%rsp)
	vzeroupper
	callq	*%r13
	testq	%rax, %rax
	je	.LBB911_878
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB911_43
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_43:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_49
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_43
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
.LBB911_46:
	cmpq	%rax, %rdx
	jle	.LBB911_48
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB911_46
.LBB911_48:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_49:
	vmovdqu64	864(%rsp), %zmm0
	movq	928(%rsp), %rax
	movq	%rcx, 304(%rsp)
	movq	%rax, 64(%rcx)
	movb	$1, %al
	movl	%eax, 16(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp10494:
	movq	40(%rsp), %r12
	movq	1320(%rsp), %rsi
	movq	%r15, %rdx
	movq	%r12, %rdi
	vzeroupper
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10495:
	movq	304(%rsp), %rcx
	addq	$16, %rcx
.Ltmp10496:
	leaq	1856(%rsp), %rbx
	movq	%rax, %rsi
	movq	%r15, %rdx
	movq	%r12, %r8
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10497:
	testb	%r14b, %r14b
	je	.LBB911_61
	movq	2256(%rsp), %rax
	movq	40(%rsp), %rdi
	movq	472(%rsp), %r15
	cmpq	%rbp, %rax
	movq	1040(%rdi), %rcx
	cmovbq	%rax, %rbp
	addq	904(%rdi), %rcx
	movq	%rcx, 1656(%rsp)
.Ltmp10513:
	movq	%rbp, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp10514:
	movq	40(%rsp), %rcx
	movq	%rax, 456(%rsp)
	leaq	2072(%rsp), %r13
	movq	616(%rcx), %rax
	testq	%rax, %rax
	je	.LBB911_126
	cmpq	$-2, 16(%rax)
	movb	$1, %cl
	jb	.LBB911_127
	cmpq	$-2, 40(%rax)
	setb	%cl
	jmp	.LBB911_127
.LBB911_56:
	movq	1792(%rsp), %r14
	testq	%r14, %r14
	je	.LBB911_71
	lock		incq	(%r14)
	jle	.LBB911_882
	movq	%r14, 864(%rsp)
	leaq	16(%r14), %rsi
.Ltmp10870:
	leaq	1856(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp10871:
	lock		decq	(%r14)
	jne	.LBB911_75
	#MEMBARRIER
.Ltmp10876:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	864(%rsp), %rdi
	callq	*%rax
.Ltmp10877:
	jmp	.LBB911_75
.LBB911_61:
	leaq	(,%rbp,8), %rax
	leaq	(%rax,%rax,4), %r12
	testq	%rbp, %rbp
	je	.LBB911_102
	movq	%r12, %rdi
	callq	*%r13
	testq	%rax, %rax
	je	.LBB911_880
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdi
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB911_65
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_65:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_103
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_65
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
.LBB911_68:
	cmpq	%rax, %rdx
	jle	.LBB911_70
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB911_68
.LBB911_70:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jmp	.LBB911_103
.LBB911_71:
.Ltmp10878:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
.Ltmp10879:
	movq	%rax, %rsi
	movq	%rax, %r14
	movq	%rax, 864(%rsp)
	addq	$16, %rsi
.Ltmp10880:
	leaq	1856(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp10881:
	lock		decq	(%r14)
	jne	.LBB911_75
	#MEMBARRIER
.Ltmp10886:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	864(%rsp), %rdi
	callq	*%rax
.Ltmp10887:
.LBB911_75:
	movq	(%rbx), %rsi
	lock		incq	(%rsi)
	jle	.LBB911_882
	movq	8(%rbx), %rdx
.Ltmp10889:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	1856(%rsp), %rdi
	callq	*%rax
.Ltmp10890:
	vmovdqu64	1736(%rsp), %zmm1
	vmovups	1696(%rsp), %zmm0
	vmovdqu	1880(%rsp), %ymm2
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vmovdqu64	%zmm1, 904(%rsp)
	vmovups	%zmm0, 864(%rsp)
	vmovdqu	1856(%rsp), %ymm0
	vmovdqu	%ymm2, 568(%rsp)
	vmovdqu	%ymm0, 544(%rsp)
	movq	$1, 528(%rsp)
	movq	$1, 536(%rsp)
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB911_879
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rdx
	movabsq	$9223372036854775807, %rbx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rbx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB911_80
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_80:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_86
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_80
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
.LBB911_83:
	cmpq	%rax, %rdx
	jle	.LBB911_85
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB911_83
.LBB911_85:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_86:
	vmovups	528(%rsp), %zmm0
	movq	592(%rsp), %rax
	cmpq	$-1, 864(%rsp)
	movq	%rcx, 1416(%rsp)
	movq	$0, 1392(%rsp)
	movq	$8, 1400(%rsp)
	movq	$0, 1408(%rsp)
	movq	%rax, 64(%rcx)
	vmovups	%zmm0, (%rcx)
	je	.LBB911_88
	leaq	528(%rsp), %rdi
	leaq	1392(%rsp), %rsi
	leaq	1696(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	936(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB911_89
	jmp	.LBB911_98
.LBB911_88:
	movq	1400(%rsp), %rcx
	movq	1392(%rsp), %rax
	movq	1408(%rsp), %rdx
	movq	%rcx, 544(%rsp)
	movq	1416(%rsp), %rcx
	movq	%rax, 536(%rsp)
	movq	%rdx, 552(%rsp)
	movq	%rcx, 560(%rsp)
	movq	$-1, 528(%rsp)
	movq	936(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB911_98
.LBB911_89:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	944(%rsp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_91
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_91:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_97
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_91
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
.LBB911_94:
	cmpq	%rax, %rdx
	jge	.LBB911_96
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB911_94
.LBB911_96:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_97:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB911_98:
	movq	960(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_101
	lock		decq	(%rax)
	jne	.LBB911_101
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	960(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB911_101:
	vmovdqu64	528(%rsp), %zmm0
	vmovdqu64	560(%rsp), %zmm1
	movq	320(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB911_260
.LBB911_102:
	movl	$8, %r13d
.LBB911_103:
	movq	472(%rsp), %rax
	movq	464(%rsp), %rcx
	movq	%rbp, 96(%rsp)
	movq	%r13, 104(%rsp)
	movq	$0, 112(%rsp)
	addq	%rax, %r12
	movq	%rax, 1392(%rsp)
	movq	%rcx, 1408(%rsp)
	movq	%rcx, 32(%rsp)
	movq	%rax, 64(%rsp)
	movq	%r12, 1416(%rsp)
	testq	%rbp, %rbp
	je	.LBB911_125
	leaq	536(%rsp), %rbx
	xorl	%ebp, %ebp
	jmp	.LBB911_107
	.p2align	4
.LBB911_105:
	movq	104(%rsp), %rdx
.LBB911_106:
	leaq	(%rbp,%rbp,4), %rax
	movq	%r15, %rbp
	movq	%r13, (%rdx,%rax,8)
	movq	%rbx, 8(%rdx,%rax,8)
	movq	%rdx, %r13
	movq	%r14, %rbx
	vmovdqa	864(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	880(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	16(%rsp), %rdx
	movq	%r15, 112(%rsp)
	movq	%rdx, %rax
	cmpq	%r12, %rdx
	je	.LBB911_172
.LBB911_107:
	movq	(%rax), %rcx
	leaq	40(%rax), %r15
	testq	%rcx, %rcx
	je	.LBB911_173
	vmovups	8(%rax), %ymm0
	movq	%r15, 16(%rsp)
	leaq	1(%rbp), %r15
	movq	%rbp, %r14
	movq	%rcx, 528(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovups	%ymm0, (%rbx)
.Ltmp10500:
	movq	40(%rsp), %rdx
	leaq	864(%rsp), %rdi
	leaq	2072(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10501:
	cmpb	$-1, 864(%rsp)
	jne	.LBB911_136
	movq	528(%rsp), %rcx
	movq	544(%rsp), %rdx
	movq	1152(%rsp), %rax
	leaq	-1(%rcx), %rsi
	leaq	-1(%rdx), %rdi
	cmpq	$5, %rsi
	cmovbq	%rsi, %rdi
	movq	%rax, %rsi
	subq	%rdi, %rsi
	jbe	.LBB911_112
	movl	$2, 864(%rsp)
	movq	%rsi, 872(%rsp)
.Ltmp10502:
	leaq	528(%rsp), %rdi
	leaq	864(%rsp), %rsi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp10503:
	jmp	.LBB911_114
	.p2align	4
.LBB911_112:
	cmpq	$6, %rcx
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB911_114
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, 528(%rsp,%rdx)
.LBB911_114:
	movq	528(%rsp), %rcx
	movq	40(%rsp), %rax
	movq	%rbx, %rdx
	decq	%rcx
	movq	%r14, 568(%rax)
	cmpq	$5, %rcx
	jb	.LBB911_116
	movq	544(%rsp), %rcx
	movq	536(%rsp), %rdx
	decq	%rcx
.LBB911_116:
	movq	304(%rsp), %r8
	addq	$16, %r8
.Ltmp10504:
	movq	40(%rsp), %r9
	leaq	864(%rsp), %rdi
	leaq	1856(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10505:
	movq	864(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB911_139
	movq	528(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB911_120
	movq	544(%rsp), %rsi
.LBB911_120:
	movq	1232(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB911_876
	movq	%r13, %rdx
	movq	%rbx, %r14
	movq	%rbx, %rcx
	cmpq	$6, %rax
	jb	.LBB911_123
	movq	536(%rsp), %rcx
.LBB911_123:
	vmovsd	872(%rsp), %xmm0
	vmovlps	%xmm0, (%rcx,%rdi,8)
	vmovups	8(%r14), %xmm0
	movq	24(%r14), %rax
	movq	528(%rsp), %r13
	movq	536(%rsp), %rbx
	movq	%rax, 880(%rsp)
	vmovaps	%xmm0, 864(%rsp)
	cmpq	96(%rsp), %rbp
	jne	.LBB911_106
.Ltmp10510:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	callq	*%rax
.Ltmp10511:
	jmp	.LBB911_105
.LBB911_125:
	xorl	%ebp, %ebp
	movq	%rax, %r15
	jmp	.LBB911_173
.LBB911_126:
	xorl	%ecx, %ecx
.LBB911_127:
	movq	40(%rsp), %rdx
	leaq	456(%rsp), %rsi
	leaq	1152(%rsp), %rdi
	movq	%r15, 176(%rsp)
	movq	%rbp, 184(%rsp)
	movq	%rdi, 192(%rsp)
	leaq	1232(%rsp), %rdi
	movzbl	1234(%rdx), %eax
	movq	%rdx, 1184(%rsp)
	movq	%rsi, 1192(%rsp)
	leaq	304(%rsp), %rsi
	movq	%r13, 1200(%rsp)
	movq	%rbx, 1208(%rsp)
	movq	%rsi, 200(%rsp)
	leaq	1656(%rsp), %rsi
	movq	%rdi, 208(%rsp)
	movq	%rsi, 216(%rsp)
	xorb	$1, %al
	testb	%cl, %cl
	je	.LBB911_130
	cmpq	$1025, %rbp
	movq	%r13, 1328(%rsp)
	setae	%cl
	testb	%al, %cl
	jne	.LBB911_132
	vmovdqu	1184(%rsp), %ymm0
	vmovdqu	192(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm2
	leaq	784(%rsp), %rax
	movq	%r15, 352(%rsp)
	movq	%rbp, 360(%rsp)
	movq	%r13, 832(%rsp)
	movq	%rax, 96(%rsp)
	leaq	352(%rsp), %rax
	movq	%rax, 104(%rsp)
	leaq	528(%rsp), %rax
	movq	%rax, 112(%rsp)
	leaq	832(%rsp), %rax
	movq	%rax, 120(%rsp)
	vmovdqu	%ymm1, 544(%rsp)
	vmovdqu	%ymm0, 784(%rsp)
	vmovdqu	%ymm2, 528(%rsp)
.Ltmp10571:
	leaq	864(%rsp), %rdi
	leaq	96(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#0}
.Ltmp10572:
	jmp	.LBB911_347
.LBB911_130:
	leaq	1184(%rsp), %rcx
	cmpq	$1025, %rbp
	leaq	1328(%rsp), %rsi
	leaq	176(%rsp), %rdi
	movq	%r15, 1328(%rsp)
	movq	%rbp, 1336(%rsp)
	movq	%r13, 328(%rsp)
	movq	%rcx, 96(%rsp)
	movq	%rsi, 104(%rsp)
	leaq	328(%rsp), %rsi
	setae	%dl
	movq	%rdi, 112(%rsp)
	movq	%rsi, 120(%rsp)
	testb	%al, %dl
	jne	.LBB911_134
.Ltmp10537:
	leaq	864(%rsp), %rdi
	leaq	96(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#0}
.Ltmp10538:
	jmp	.LBB911_347
.LBB911_132:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB911_271
	addq	$272, %rax
	jmp	.LBB911_272
.LBB911_134:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB911_274
	addq	$272, %rax
	movq	%rbp, %rbx
	jmp	.LBB911_276
.LBB911_136:
	movq	16(%rsp), %rax
	movq	%rax, 1400(%rsp)
	movq	528(%rsp), %rax
	movq	%r15, 1424(%rsp)
	cmpq	$6, %rax
	jb	.LBB911_138
	movq	536(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB911_138:
	movq	16(%rsp), %r15
	jmp	.LBB911_174
.LBB911_139:
	vmovdqu64	880(%rsp), %zmm0
	vmovdqu64	896(%rsp), %zmm1
	movq	320(%rsp), %rdx
	movq	16(%rsp), %r15
	movq	872(%rsp), %rcx
	movq	%r15, 1400(%rsp)
	vmovdqu64	%zmm1, 48(%rdx)
	vmovdqu64	%zmm0, 32(%rdx)
	movq	%rax, 16(%rdx)
	movq	528(%rsp), %rax
	movq	%rcx, 24(%rdx)
	movq	$1, (%rdx)
	cmpq	$6, %rax
	jb	.LBB911_141
	movq	536(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB911_141:
	subq	%r15, %r12
	je	.LBB911_154
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %rbx
	imulq	%r12, %rbx
	xorl	%r12d, %r12d
	jmp	.LBB911_146
.LBB911_143:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_144:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB911_145:
	incq	%r12
	cmpq	%rbx, %r12
	je	.LBB911_154
.LBB911_146:
	leaq	(%r12,%r12,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_145
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_149
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_149:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_144
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_149
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
.LBB911_152:
	cmpq	%rax, %rdx
	jge	.LBB911_143
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB911_152
	jmp	.LBB911_143
.LBB911_154:
	movq	32(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_156
	movq	64(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB911_156:
	testq	%r14, %r14
	je	.LBB911_169
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%ebx, %ebx
	jmp	.LBB911_161
.LBB911_158:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_159:
	vzeroupper
	callq	*%rbp
.LBB911_160:
	incq	%rbx
	cmpq	%rbx, %r14
	je	.LBB911_169
.LBB911_161:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_160
	leaq	(%r13,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_164
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_164:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_159
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_164
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
.LBB911_167:
	cmpq	%rax, %rdx
	jge	.LBB911_158
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB911_167
	jmp	.LBB911_158
.LBB911_169:
	movq	96(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_171
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r13, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB911_171:
	movl	$0, 16(%rsp)
	jmp	.LBB911_471
.LBB911_172:
	movq	%r15, %rbp
	movq	%r12, %r15
.LBB911_173:
	movq	%r15, 1400(%rsp)
	movq	%rbp, 1424(%rsp)
.LBB911_174:
	subq	%r15, %r12
	je	.LBB911_187
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %rbx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r14d, %r14d
	imulq	%r12, %rbx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	jmp	.LBB911_179
	.p2align	4
.LBB911_176:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_177:
	callq	*%rbp
.LBB911_178:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB911_187
.LBB911_179:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_178
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_182
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_182:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_177
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_182
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
.LBB911_185:
	cmpq	%rax, %rdx
	jge	.LBB911_176
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB911_185
	jmp	.LBB911_176
.LBB911_187:
	movq	32(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_198
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_190
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB911_190:
	movq	64(%rsp), %rdi
	.p2align	4
.LBB911_191:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_197
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_191
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
.LBB911_194:
	cmpq	%rax, %rdx
	jge	.LBB911_196
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB911_194
.LBB911_196:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_197:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_198:
	vmovdqu	96(%rsp), %xmm0
	movq	112(%rsp), %rax
	xorl	%ebp, %ebp
	movq	%rax, 1264(%rsp)
	vmovdqa	%xmm0, 1248(%rsp)
.LBB911_199:
	movq	40(%rsp), %rax
	movq	696(%rax), %rax
	movl	40(%rax), %ecx
	movl	%ebp, 16(%rsp)
	testl	%ecx, %ecx
	je	.LBB911_203
.LBB911_200:
	vmovups	1736(%rsp), %zmm1
	vmovdqu64	1696(%rsp), %zmm0
	movq	1264(%rsp), %rcx
	movq	304(%rsp), %rax
	movq	%rcx, 1408(%rsp)
	vmovups	%zmm1, 904(%rsp)
	vmovdqa	1248(%rsp), %xmm1
	vmovdqu64	%zmm0, 864(%rsp)
	cmpq	$-1, 864(%rsp)
	vmovdqa	%xmm1, 1392(%rsp)
	movq	%rax, 1416(%rsp)
	je	.LBB911_219
	leaq	528(%rsp), %rdi
	leaq	1392(%rsp), %rsi
	leaq	1696(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	936(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB911_202
.LBB911_220:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_222
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_222:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_228
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_222
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
.LBB911_225:
	cmpq	%rax, %rdx
	jge	.LBB911_227
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB911_225
.LBB911_227:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_228:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	960(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB911_229
	jmp	.LBB911_231
.LBB911_203:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB911_200
	movb	%cl, 528(%rsp)
	movq	304(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 529(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 544(%rsp)
	lock		incq	(%rcx)
	jle	.LBB911_882
	movq	304(%rsp), %rcx
.Ltmp10822:
	movq	1320(%rsp), %rsi
	leaq	864(%rsp), %rdi
	leaq	528(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp10823:
	vmovdqu64	864(%rsp), %zmm0
	vmovdqu64	896(%rsp), %zmm1
	movq	320(%rsp), %rax
	movq	1256(%rsp), %rbx
	movq	1264(%rsp), %r14
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	testq	%r14, %r14
	je	.LBB911_261
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	xorl	%r15d, %r15d
	jmp	.LBB911_211
.LBB911_208:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_209:
	vzeroupper
	callq	*%r13
.LBB911_210:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB911_261
.LBB911_211:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_210
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_214
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_214:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_209
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_214
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
	movq	(%rbp), %rax
	.p2align	4
.LBB911_217:
	cmpq	%rax, %rdx
	jge	.LBB911_208
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB911_217
	jmp	.LBB911_208
.LBB911_219:
	vmovdqu	1392(%rsp), %xmm0
	movq	1408(%rsp), %rax
	movq	1416(%rsp), %rcx
	movq	%rax, 552(%rsp)
	movq	%rcx, 560(%rsp)
	vmovdqu	%xmm0, 536(%rsp)
	movq	$-1, 528(%rsp)
	movq	936(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB911_220
.LBB911_202:
	movq	960(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_231
.LBB911_229:
	lock		decq	(%rax)
	jne	.LBB911_231
	leaq	960(%rsp), %rdi
	#MEMBARRIER
.Ltmp10825:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp10826:
.LBB911_231:
	vmovdqu64	528(%rsp), %zmm0
	vmovdqu64	560(%rsp), %zmm1
	movq	320(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp10828:
	leaq	1856(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp10829:
	xorl	%r13d, %r13d
.Ltmp10831:
	leaq	2072(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10832:
	movq	488(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB911_235
	#MEMBARRIER
.Ltmp10833:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	488(%rsp), %rdi
	callq	*%rax
.Ltmp10834:
.LBB911_235:
	cmpb	$0, 16(%rsp)
	je	.LBB911_260
	movq	472(%rsp), %rbx
	movq	480(%rsp), %r14
	testq	%r14, %r14
	je	.LBB911_249
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB911_241
	.p2align	4
.LBB911_238:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_239:
	callq	*%rbp
.LBB911_240:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB911_249
.LBB911_241:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_240
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_244
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_244:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_239
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_244
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
.LBB911_247:
	cmpq	%rax, %rdx
	jge	.LBB911_238
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB911_247
	jmp	.LBB911_238
.LBB911_249:
	movq	464(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_260
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_252
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_252:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_258
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_252
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
.LBB911_255:
	cmpq	%rax, %rdx
	jge	.LBB911_257
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB911_255
.LBB911_257:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_258:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	jmp	.LBB911_259
.LBB911_260:
	movq	320(%rsp), %rax
	addq	$2392, %rsp
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
.LBB911_261:
	.cfi_def_cfa_offset 2448
	movq	1248(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_471
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_264
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_264:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_270
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_264
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
.LBB911_267:
	cmpq	%rax, %rdx
	jge	.LBB911_269
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB911_267
.LBB911_269:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_270:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB911_471
.LBB911_271:
.Ltmp10539:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp10540:
.LBB911_272:
	movq	(%rax), %rax
	movq	520(%rax), %r12
	movq	%rbp, %rax
	cmpq	$1, %r12
	adcq	$0, %r12
	movq	%r12, %rcx
	shlq	$6, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB911_278
	movq	%rbp, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB911_279
.LBB911_274:
.Ltmp10515:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp10516:
	movq	1328(%rsp), %r15
	movq	1336(%rsp), %rbx
.LBB911_276:
	movq	(%rax), %rax
	movq	520(%rax), %rcx
	movq	%rbp, %rax
	cmpq	$1, %rcx
	adcq	$0, %rcx
	shlq	$2, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB911_285
	movq	%rbp, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB911_286
.LBB911_278:
	movl	%ebp, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB911_279:
	cmpq	$65, %rax
	movl	$64, %ecx
	movq	%r15, 528(%rsp)
	movq	%rbp, 536(%rsp)
	cmovaeq	%rax, %rcx
	movq	%rcx, 544(%rsp)
.Ltmp10541:
	leaq	832(%rsp), %rbx
	leaq	528(%rsp), %rsi
	movq	%rbx, %rdi
	callq	<alloc::vec::Vec<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>]> as alloc::vec::spec_from_iter::SpecFromIter<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>], core::slice::iter::Chunks<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>>::from_iter
.Ltmp10542:
	movq	848(%rsp), %r13
	movabsq	$33909456017848440, %rax
	imulq	$272, %r13, %r14
	cmpq	%rax, %r13
	jbe	.LBB911_283
	xorl	%r15d, %r15d
.LBB911_282:
.Ltmp10568:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp10569:
	jmp	.LBB911_882
.LBB911_283:
	testq	%r14, %r14
	je	.LBB911_297
	movl	$16, %esi
	movq	%r14, %rdi
	movl	$16, %r15d
	callq	__rustc::__rust_alloc
	movq	%r13, %rcx
	testq	%rax, %rax
	jne	.LBB911_298
	jmp	.LBB911_282
.LBB911_285:
	movl	%ebp, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB911_286:
	cmpq	$17, %rax
	movl	$16, %ebp
	movq	$0, 416(%rsp)
	movq	$16, 424(%rsp)
	movq	$0, 432(%rsp)
	cmovaeq	%rax, %rbp
	movq	%rbx, %rax
	orq	%rbp, %rax
	shrq	$32, %rax
	je	.LBB911_288
	movq	%rbx, %rax
	xorl	%edx, %edx
	divq	%rbp
	jmp	.LBB911_289
.LBB911_288:
	movl	%ebx, %eax
	xorl	%edx, %edx
	divl	%ebp
.LBB911_289:
	xorl	%r14d, %r14d
	testq	%rdx, %rdx
	setne	%r14b
	addq	%rax, %r14
	movq	%r14, 1360(%rsp)
	jne	.LBB911_867
	xorl	%eax, %eax
	xorl	%r13d, %r13d
	subq	%r13, %rax
	cmpq	%r14, %rax
	jb	.LBB911_869
.LBB911_291:
	leaq	1184(%rsp), %rax
	leaq	176(%rsp), %rcx
	movq	%r15, 528(%rsp)
	movq	%rbx, 536(%rsp)
	movq	%rbp, 544(%rsp)
	movq	424(%rsp), %r12
	leaq	328(%rsp), %rdx
	movq	%rbp, 368(%rsp)
	movq	%r15, 352(%rsp)
	movq	%rbx, 360(%rsp)
	movq	%rax, 552(%rsp)
	movq	%rcx, 560(%rsp)
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rcx
	movq	%rdx, 568(%rsp)
	movq	%fs:(%rcx), %rax
	testq	%rax, %rax
	je	.LBB911_293
	addq	$272, %rax
	jmp	.LBB911_294
.LBB911_293:
.Ltmp10519:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp10520:
.LBB911_294:
	movq	(%rax), %rax
	leaq	552(%rsp), %rdx
	movq	520(%rax), %rcx
	movq	%r13, %rax
	shlq	$8, %rax
	movq	%rdx, 784(%rsp)
	addq	%rax, %r12
	movq	%r12, 792(%rsp)
	movq	%r14, 800(%rsp)
.Ltmp10521:
	leaq	784(%rsp), %r15
	leaq	832(%rsp), %rdi
	leaq	352(%rsp), %r9
	movl	$1, %r8d
	movq	%r14, %rsi
	xorl	%edx, %edx
	movq	%r15, (%rsp)
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#1}>>
.Ltmp10522:
	movq	848(%rsp), %rbx
	movq	%rbx, 784(%rsp)
	cmpq	%r14, %rbx
	jne	.LBB911_870
	vmovdqu	416(%rsp), %xmm0
	addq	%r14, %r13
	movq	%r13, 544(%rsp)
	vmovdqa	%xmm0, 528(%rsp)
.Ltmp10529:
	leaq	864(%rsp), %rdi
	leaq	528(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
.Ltmp10530:
	jmp	.LBB911_347
.LBB911_297:
	movl	$16, %eax
	xorl	%ecx, %ecx
.LBB911_298:
	testq	%r13, %r13
	je	.LBB911_306
	movl	%r13d, %edx
	andl	$7, %edx
	cmpq	$8, %r13
	jae	.LBB911_301
	xorl	%esi, %esi
	jmp	.LBB911_304
.LBB911_301:
	movabsq	$36028797018963960, %rdi
	xorl	%esi, %esi
	movq	%rax, %r8
	andq	%r13, %rdi
.LBB911_302:
	movl	$0, (%r8)
	movb	$0, 4(%r8)
	movq	$2, 16(%r8)
	movl	$0, 272(%r8)
	movb	$0, 276(%r8)
	movq	$2, 288(%r8)
	movl	$0, 544(%r8)
	movb	$0, 548(%r8)
	movq	$2, 560(%r8)
	movl	$0, 816(%r8)
	movb	$0, 820(%r8)
	movq	$2, 832(%r8)
	movl	$0, 1088(%r8)
	movb	$0, 1092(%r8)
	movq	$2, 1104(%r8)
	movl	$0, 1360(%r8)
	movb	$0, 1364(%r8)
	movq	$2, 1376(%r8)
	movl	$0, 1632(%r8)
	movb	$0, 1636(%r8)
	movq	$2, 1648(%r8)
	movl	$0, 1904(%r8)
	movb	$0, 1908(%r8)
	movq	$2, 1920(%r8)
	addq	$8, %rsi
	addq	$2176, %r8
	cmpq	%rsi, %rdi
	jne	.LBB911_302
	testq	%rdx, %rdx
	je	.LBB911_306
.LBB911_304:
	imulq	$272, %rsi, %rsi
	imulq	$272, %rdx, %rdx
	xorl	%edi, %edi
	addq	%rax, %rsi
	.p2align	4
.LBB911_305:
	movl	$0, (%rsi,%rdi)
	movb	$0, 4(%rsi,%rdi)
	movq	$2, 16(%rsi,%rdi)
	addq	$272, %rdi
	cmpq	%rdi, %rdx
	jne	.LBB911_305
.LBB911_306:
	movq	848(%rsp), %rdi
	movq	%rcx, 352(%rsp)
	movq	%rax, 360(%rsp)
	leaq	416(%rsp), %rax
	movq	%r13, 368(%rsp)
	movq	$0, 416(%rsp)
	movq	%rax, 528(%rsp)
	leaq	1184(%rsp), %rax
	movq	%rbx, 536(%rsp)
	movq	%rax, 544(%rsp)
	leaq	176(%rsp), %rax
	movq	%rax, 552(%rsp)
	leaq	1328(%rsp), %rax
	movq	%rax, 560(%rsp)
	leaq	352(%rsp), %rax
	cmpq	%rdi, %r12
	movq	%rax, 568(%rsp)
	cmovbq	%r12, %rdi
.Ltmp10543:
	leaq	528(%rsp), %rsi
	callq	<rayon::range::Iter<usize> as rayon::iter::ParallelIterator>::drive_unindexed::<rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#1}>>
.Ltmp10544:
	movq	368(%rsp), %rsi
	movq	360(%rsp), %r14
	movq	352(%rsp), %rbx
	movabsq	$-1085102592571150095, %rcx
	imulq	$272, %rsi, %rdx
	movq	%r14, %r15
	movq	%r14, %r12
	leaq	(%r14,%rdx), %rax
	testq	%rsi, %rsi
	je	.LBB911_333
	addq	$-272, %rdx
	mulxq	%rcx, %rsi, %rsi
	shrl	$8, %esi
	incl	%esi
	andl	$7, %esi
	je	.LBB911_313
	imulq	$272, %rsi, %rdi
	movq	%r14, %rsi
	movq	%r14, %r12
	jmp	.LBB911_311
	.p2align	4
.LBB911_310:
	addq	$272, %rsi
	addq	$-272, %rdi
	je	.LBB911_314
.LBB911_311:
	vmovdqu64	24(%rsi), %zmm0
	vmovdqu64	88(%rsi), %zmm1
	vmovdqu64	152(%rsi), %zmm2
	vmovdqu64	208(%rsi), %zmm3
	movq	16(%rsi), %r8
	vmovdqu64	%zmm3, 712(%rsp)
	vmovdqu64	%zmm2, 656(%rsp)
	vmovdqu64	%zmm1, 592(%rsp)
	vmovdqu64	%zmm0, 528(%rsp)
	cmpq	$2, %r8
	je	.LBB911_310
	movq	%r8, (%r12)
	vmovdqu64	528(%rsp), %zmm0
	vmovdqu64	592(%rsp), %zmm1
	vmovdqu64	656(%rsp), %zmm2
	vmovdqu64	712(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	vmovdqu64	%zmm1, 72(%r12)
	vmovdqu64	%zmm3, 192(%r12)
	addq	$256, %r12
	jmp	.LBB911_310
.LBB911_313:
	movq	%r14, %rsi
	movq	%r14, %r12
.LBB911_314:
	movq	%rax, %r15
	cmpq	$1904, %rdx
	jae	.LBB911_316
	jmp	.LBB911_333
.LBB911_315:
	addq	$2176, %rsi
	cmpq	%rax, %rsi
	je	.LBB911_332
.LBB911_316:
	vmovups	24(%rsi), %zmm0
	vmovups	88(%rsi), %zmm1
	vmovups	152(%rsi), %zmm2
	vmovups	208(%rsi), %zmm3
	movq	16(%rsi), %rdx
	vmovups	%zmm3, 712(%rsp)
	vmovups	%zmm2, 656(%rsp)
	vmovups	%zmm1, 592(%rsp)
	vmovups	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_318
	movq	%rdx, (%r12)
	vmovups	528(%rsp), %zmm0
	vmovups	592(%rsp), %zmm1
	vmovups	656(%rsp), %zmm2
	vmovups	712(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB911_318:
	vmovups	296(%rsi), %zmm0
	vmovups	360(%rsi), %zmm1
	vmovups	424(%rsi), %zmm2
	vmovups	480(%rsi), %zmm3
	movq	288(%rsi), %rdx
	vmovups	%zmm3, 712(%rsp)
	vmovups	%zmm2, 656(%rsp)
	vmovups	%zmm1, 592(%rsp)
	vmovups	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_320
	movq	%rdx, (%r12)
	vmovups	528(%rsp), %zmm0
	vmovups	592(%rsp), %zmm1
	vmovups	656(%rsp), %zmm2
	vmovups	712(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB911_320:
	vmovups	568(%rsi), %zmm0
	vmovups	632(%rsi), %zmm1
	vmovups	696(%rsi), %zmm2
	vmovups	752(%rsi), %zmm3
	movq	560(%rsi), %rdx
	vmovups	%zmm3, 712(%rsp)
	vmovups	%zmm2, 656(%rsp)
	vmovups	%zmm1, 592(%rsp)
	vmovups	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_322
	movq	%rdx, (%r12)
	vmovups	528(%rsp), %zmm0
	vmovups	592(%rsp), %zmm1
	vmovups	656(%rsp), %zmm2
	vmovups	712(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB911_322:
	vmovups	840(%rsi), %zmm0
	vmovups	904(%rsi), %zmm1
	vmovups	968(%rsi), %zmm2
	vmovups	1024(%rsi), %zmm3
	movq	832(%rsi), %rdx
	vmovups	%zmm3, 712(%rsp)
	vmovups	%zmm2, 656(%rsp)
	vmovups	%zmm1, 592(%rsp)
	vmovups	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_324
	movq	%rdx, (%r12)
	vmovups	528(%rsp), %zmm0
	vmovups	592(%rsp), %zmm1
	vmovups	656(%rsp), %zmm2
	vmovups	712(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB911_324:
	vmovups	1112(%rsi), %zmm0
	vmovups	1176(%rsi), %zmm1
	vmovups	1240(%rsi), %zmm2
	vmovups	1296(%rsi), %zmm3
	movq	1104(%rsi), %rdx
	vmovups	%zmm3, 712(%rsp)
	vmovups	%zmm2, 656(%rsp)
	vmovups	%zmm1, 592(%rsp)
	vmovups	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_326
	movq	%rdx, (%r12)
	vmovups	528(%rsp), %zmm0
	vmovups	592(%rsp), %zmm1
	vmovups	656(%rsp), %zmm2
	vmovups	712(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB911_326:
	vmovups	1384(%rsi), %zmm0
	vmovups	1448(%rsi), %zmm1
	vmovups	1512(%rsi), %zmm2
	vmovups	1568(%rsi), %zmm3
	movq	1376(%rsi), %rdx
	vmovups	%zmm3, 712(%rsp)
	vmovups	%zmm2, 656(%rsp)
	vmovups	%zmm1, 592(%rsp)
	vmovups	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_328
	movq	%rdx, (%r12)
	vmovups	528(%rsp), %zmm0
	vmovups	592(%rsp), %zmm1
	vmovups	656(%rsp), %zmm2
	vmovups	712(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB911_328:
	vmovups	1656(%rsi), %zmm0
	vmovups	1720(%rsi), %zmm1
	vmovups	1784(%rsi), %zmm2
	vmovups	1840(%rsi), %zmm3
	movq	1648(%rsi), %rdx
	vmovups	%zmm3, 712(%rsp)
	vmovups	%zmm2, 656(%rsp)
	vmovups	%zmm1, 592(%rsp)
	vmovups	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_330
	movq	%rdx, (%r12)
	vmovups	528(%rsp), %zmm0
	vmovups	592(%rsp), %zmm1
	vmovups	656(%rsp), %zmm2
	vmovups	712(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB911_330:
	vmovdqu64	1928(%rsi), %zmm0
	vmovdqu64	1992(%rsi), %zmm1
	vmovdqu64	2056(%rsi), %zmm2
	vmovdqu64	2112(%rsi), %zmm3
	movq	1920(%rsi), %rdx
	vmovdqu64	%zmm3, 712(%rsp)
	vmovdqu64	%zmm2, 656(%rsp)
	vmovdqu64	%zmm1, 592(%rsp)
	vmovdqu64	%zmm0, 528(%rsp)
	cmpq	$2, %rdx
	je	.LBB911_315
	movq	%rdx, (%r12)
	vmovdqu64	528(%rsp), %zmm0
	vmovdqu64	592(%rsp), %zmm1
	vmovdqu64	656(%rsp), %zmm2
	vmovdqu64	712(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	vmovdqu64	%zmm1, 72(%r12)
	vmovdqu64	%zmm3, 192(%r12)
	addq	$256, %r12
	jmp	.LBB911_315
.LBB911_332:
	movq	%rax, %r15
.LBB911_333:
	vmovdqa	.LCPI911_0(%rip), %ymm0
	subq	%r14, %r12
	shrq	$8, %r12
	subq	%r15, %rax
	movq	%rax, %rdx
	mulxq	%rcx, %rax, %rax
	movq	%r14, 96(%rsp)
	movq	%r12, 104(%rsp)
	movq	%rbx, 112(%rsp)
	vmovdqu	%ymm0, 528(%rsp)
	je	.LBB911_338
	shrq	$8, %rax
	movl	$1, %r13d
	addq	$288, %r15
	subq	%rax, %r13
	jmp	.LBB911_336
	.p2align	4
.LBB911_335:
	addq	$272, %r15
	incq	%r13
	cmpq	$1, %r13
	je	.LBB911_338
.LBB911_336:
	cmpl	$2, -272(%r15)
	je	.LBB911_335
.Ltmp10549:
	leaq	-272(%r15), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
.Ltmp10550:
	jmp	.LBB911_335
.LBB911_338:
	imulq	$272, %rbx, %rbx
	testb	$-16, %bl
	je	.LBB911_343
	movq	%rbx, %r15
	andq	$-256, %r15
	je	.LBB911_342
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	leaq	qualification_454_native_cost::GLOBAL (.llvm.11174181910260379007)(%rip), %rdi
	movl	$16, %edx
	movq	%r14, %rsi
	movq	%rbx, %rcx
	movq	%r15, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %r14
	testq	%rax, %rax
	jne	.LBB911_343
.Ltmp10555:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$16, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp10556:
	jmp	.LBB911_882
.LBB911_342:
	movl	$16, %edx
	movq	%r14, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
	movl	$16, %r14d
.LBB911_343:
	shrq	$8, %rbx
	movq	%rbx, 784(%rsp)
	movq	%r14, 792(%rsp)
	movq	%r12, 800(%rsp)
.Ltmp10563:
	leaq	528(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#2}>>
.Ltmp10564:
.Ltmp10565:
	leaq	864(%rsp), %rdi
	leaq	784(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
.Ltmp10566:
	movq	832(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB911_347
	movq	840(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB911_347:
	movq	864(%rsp), %rdx
	cmpq	$-1, %rdx
	je	.LBB911_355
	vmovups	1056(%rsp), %zmm0
	vmovups	1040(%rsp), %zmm1
	movq	888(%rsp), %rax
	vmovdqu	872(%rsp), %xmm2
	movq	904(%rsp), %rsi
	movq	896(%rsp), %rcx
	movq	%rdx, 1160(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rdx
	cmpq	$-2, %rdx
	movl	$1, %edx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rdx
	vmovups	%zmm0, 1536(%rsp)
	vmovups	%zmm1, 1520(%rsp)
	vmovdqu64	912(%rsp), %zmm0
	vmovdqu64	976(%rsp), %zmm1
	decq	%rax
	vmovdqu	%xmm2, 1168(%rsp)
	vmovdqu64	1536(%rsp), %zmm4
	vmovdqu64	1520(%rsp), %zmm3
	vmovdqu64	%zmm0, 1392(%rsp)
	vmovdqu64	%zmm1, 1456(%rsp)
	vmovdqu64	%zmm1, 952(%rsp)
	vmovdqu64	%zmm0, 888(%rsp)
	vmovdqu64	%zmm4, 1032(%rsp)
	vmovdqu64	%zmm3, 1016(%rsp)
	movq	%rdi, 864(%rsp)
	movq	%rcx, 872(%rsp)
	movq	%rdx, 880(%rsp)
	movq	$0, 1096(%rsp)
	movq	%rax, 1104(%rsp)
.Ltmp10574:
	leaq	528(%rsp), %rdi
	leaq	864(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp10575:
	vmovups	704(%rsp), %zmm2
	vmovups	688(%rsp), %zmm1
	movq	40(%rsp), %rax
	vmovups	528(%rsp), %ymm0
	cmpq	$0, 616(%rax)
	leaq	888(%rax), %rcx
	movq	%rcx, 16(%rsp)
	vmovups	%zmm2, 1536(%rsp)
	vmovups	%zmm1, 1520(%rsp)
	vmovups	624(%rsp), %zmm2
	vmovups	560(%rsp), %zmm1
	vmovups	%ymm0, 1360(%rsp)
	vmovups	%zmm2, 1456(%rsp)
	vmovups	%zmm1, 1392(%rsp)
	je	.LBB911_356
	vmovdqu	1160(%rsp), %xmm0
	vmovdqu64	1392(%rsp), %zmm4
	vmovdqu64	1456(%rsp), %zmm1
	vmovdqu64	1536(%rsp), %zmm3
	vmovdqu64	1520(%rsp), %zmm2
	movq	1176(%rsp), %rax
	cmpb	$2, 2266(%rsp)
	movq	%rax, 1296(%rsp)
	vmovdqu64	%zmm3, 672(%rsp)
	vmovdqa	%xmm0, 1280(%rsp)
	vmovdqu64	%zmm2, 656(%rsp)
	vmovdqu64	%zmm1, 592(%rsp)
	vmovdqu64	%zmm4, 528(%rsp)
	jne	.LBB911_361
	movq	528(%rsp), %rax
	vmovdqu64	1416(%rsp), %zmm0
	vmovdqu64	1536(%rsp), %zmm2
	vmovdqu64	1480(%rsp), %zmm1
	movq	544(%rsp), %rdx
	movq	536(%rsp), %rcx
	movl	$1, %edi
	movl	$1, %esi
	cmpq	$3, %rax
	cmovaeq	%rax, %rdi
	cmovaeq	%rdx, %rax
	cmovaeq	%rsi, %rdx
	decq	%rax
	vmovdqu64	%zmm2, 1008(%rsp)
	vmovdqu64	%zmm1, 952(%rsp)
	vmovdqu64	%zmm0, 888(%rsp)
	movq	%rdi, 864(%rsp)
	movq	%rcx, 872(%rsp)
	movq	%rdx, 880(%rsp)
	movq	$0, 1072(%rsp)
	movq	%rax, 1080(%rsp)
.Ltmp10605:
	leaq	1672(%rsp), %rdi
	leaq	864(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp10606:
	movq	1688(%rsp), %rcx
	movq	1680(%rsp), %r14
	imulq	$200, %rcx, %rax
	addq	%r14, %rax
	movq	%rax, 32(%rsp)
	testq	%rcx, %rcx
	je	.LBB911_412
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB911_385
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB911_404
.LBB911_355:
	vmovdqu64	912(%rsp), %zmm0
	vmovdqu	880(%rsp), %ymm1
	movq	320(%rsp), %rax
	vmovdqu64	%zmm0, 48(%rax)
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu64	%zmm0, 1392(%rsp)
	movq	$1, (%rax)
	movq	456(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB911_468
	jmp	.LBB911_470
.LBB911_356:
	vmovups	1456(%rsp), %zmm1
	movq	1176(%rsp), %rax
	vmovdqu	1160(%rsp), %xmm0
	vmovdqu64	1392(%rsp), %zmm4
	vmovdqu64	1536(%rsp), %zmm3
	vmovdqu64	1520(%rsp), %zmm2
	movzbl	2266(%rsp), %r13d
	movq	%rax, 432(%rsp)
	movq	480(%rsp), %rax
	vmovups	%zmm1, 928(%rsp)
	vmovdqa	464(%rsp), %xmm1
	vmovdqu64	%zmm3, 1008(%rsp)
	movq	$0, 464(%rsp)
	movq	$8, 472(%rsp)
	vmovdqa	%xmm0, 416(%rsp)
	vmovdqu64	%zmm2, 992(%rsp)
	vmovdqu64	%zmm4, 864(%rsp)
	movq	$0, 480(%rsp)
	movq	%rax, 848(%rsp)
	vmovdqa	%xmm1, 832(%rsp)
	testb	%r13b, %r13b
	jne	.LBB911_866
	vmovdqa	832(%rsp), %xmm0
	movq	848(%rsp), %rax
	movq	432(%rsp), %rbx
	movq	%rax, 368(%rsp)
	vmovdqa	%xmm0, 352(%rsp)
.Ltmp10750:
	leaq	528(%rsp), %rdi
	leaq	352(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp10751:
	movq	528(%rsp), %r12
	movq	536(%rsp), %rax
	movq	544(%rsp), %r15
	movq	552(%rsp), %rbp
	cmpq	$-1, %r12
	je	.LBB911_365
	vmovdqu	576(%rsp), %ymm0
	vmovdqu	592(%rsp), %ymm1
	movq	560(%rsp), %r14
	movq	568(%rsp), %rbx
	movq	%rax, 16(%rsp)
	vmovdqu	%ymm0, 176(%rsp)
	vmovdqu	%ymm1, 192(%rsp)
.Ltmp10755:
	leaq	1160(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10756:
.LBB911_360:
	vmovups	176(%rsp), %ymm0
	vmovups	192(%rsp), %ymm1
	vmovups	%ymm0, 96(%rsp)
	vmovups	%ymm1, 112(%rsp)
	jmp	.LBB911_435
.LBB911_361:
	movq	1296(%rsp), %rbx
	movq	$0, 352(%rsp)
	movq	$8, 360(%rsp)
	movq	$0, 368(%rsp)
.Ltmp10579:
	leaq	864(%rsp), %rdi
	leaq	352(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp10580:
	movq	864(%rsp), %r12
	movq	872(%rsp), %rax
	movq	880(%rsp), %r15
	movq	888(%rsp), %rbp
	cmpq	$-1, %r12
	je	.LBB911_375
	vmovdqu	912(%rsp), %ymm0
	vmovdqu	928(%rsp), %ymm1
	movq	896(%rsp), %r14
	movq	904(%rsp), %rbx
	movq	%rax, 16(%rsp)
	vmovdqu	%ymm0, 176(%rsp)
	vmovdqu	%ymm1, 192(%rsp)
.Ltmp10584:
	leaq	1160(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10585:
.LBB911_364:
	vmovups	176(%rsp), %ymm0
	vmovups	192(%rsp), %ymm1
	movq	%r14, %r13
	shrq	$8, %r13
	vmovups	%ymm0, 1184(%rsp)
	vmovups	%ymm1, 1200(%rsp)
	jmp	.LBB911_442
.LBB911_365:
	movq	424(%rsp), %r14
	movq	%rax, 1184(%rsp)
	movq	416(%rsp), %rax
	leaq	(,%rbx,8), %rcx
	movb	%r13b, 56(%rsp)
	movq	%r15, 1192(%rsp)
	movq	%rbp, 1200(%rsp)
	leaq	(%rcx,%rcx,4), %r13
	leaq	(%r14,%r13), %rcx
	movq	%r14, 784(%rsp)
	movq	%rax, 800(%rsp)
	movq	%rcx, 808(%rsp)
	testq	%rbx, %rbx
	je	.LBB911_433
	leaq	(,%rbp,8), %rax
	movq	%rcx, 24(%rsp)
	movq	%r15, %rcx
	addq	$40, %r14
	movq	%rcx, %rdx
	leaq	(%rax,%rax,4), %r15
	jmp	.LBB911_369
.LBB911_367:
	movq	1192(%rsp), %rdx
.LBB911_368:
	vmovdqa	64(%rsp), %xmm0
	movq	32(%rsp), %rax
	movq	%rbx, (%rdx,%r15)
	incq	%rbp
	addq	$40, %r14
	movq	%rax, 8(%rdx,%r15)
	vmovdqu	%xmm0, 16(%rdx,%r15)
	movq	%r12, 32(%rdx,%r15)
	addq	$40, %r15
	addq	$-40, %r13
	movq	%rbp, 1200(%rsp)
	je	.LBB911_432
.LBB911_369:
	movq	-8(%r14), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	40(%rsp), %rax
	vmovdqu	-40(%r14), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 176(%rsp)
	cmpq	$0, 184(%rsp)
	je	.LBB911_371
	leaq	-40(%r14), %rax
	leaq	536(%rsp), %rsi
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rsi)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rsi)
	jmp	.LBB911_373
.LBB911_371:
	movq	%rdx, %rbx
	movq	664(%rax), %rdx
.Ltmp10758:
	movq	16(%rsp), %rsi
	leaq	528(%rsp), %rdi
	leaq	192(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10759:
	movq	528(%rsp), %r12
	movq	%rbx, %rdx
	cmpq	$-1, %r12
	jne	.LBB911_519
.LBB911_373:
	vmovdqu	552(%rsp), %xmm0
	movq	544(%rsp), %rax
	movq	536(%rsp), %rbx
	movq	568(%rsp), %r12
	movq	%rax, 32(%rsp)
	vmovdqa	%xmm0, 64(%rsp)
	cmpq	1184(%rsp), %rbp
	jne	.LBB911_368
.Ltmp10763:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1184(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp10764:
	jmp	.LBB911_367
.LBB911_375:
	movq	1288(%rsp), %r14
	movq	%rax, 784(%rsp)
	leaq	(,%rbx,8), %rcx
	movq	1280(%rsp), %rax
	movq	%r15, 792(%rsp)
	movq	%rbp, 800(%rsp)
	leaq	(%rcx,%rcx,4), %r13
	leaq	(%r14,%r13), %rcx
	movq	%r14, 96(%rsp)
	movq	%rax, 112(%rsp)
	movq	%rcx, 120(%rsp)
	testq	%rbx, %rbx
	je	.LBB911_440
	leaq	(,%rbp,8), %rax
	movq	%rcx, 24(%rsp)
	movq	%r15, %rcx
	addq	$40, %r14
	movq	%rcx, %rdx
	leaq	(%rax,%rax,4), %r15
	jmp	.LBB911_378
.LBB911_377:
	vmovdqa	64(%rsp), %xmm0
	movq	32(%rsp), %rax
	movq	%rbx, (%rdx,%r15)
	incq	%rbp
	addq	$40, %r14
	movq	%rax, 8(%rdx,%r15)
	vmovdqu	%xmm0, 16(%rdx,%r15)
	movq	%r12, 32(%rdx,%r15)
	addq	$40, %r15
	addq	$-40, %r13
	movq	%rbp, 800(%rsp)
	je	.LBB911_439
.LBB911_378:
	movq	40(%rsp), %rcx
	leaq	184(%rsp), %rsi
	movq	%rcx, 176(%rsp)
	movq	-8(%r14), %rax
	movq	%rax, 32(%rsi)
	vmovdqu	-40(%r14), %ymm0
	vmovdqu	%ymm0, (%rsi)
	cmpq	$0, 184(%rsp)
	je	.LBB911_380
	leaq	-40(%r14), %rax
	leaq	872(%rsp), %rsi
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rsi)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rsi)
	jmp	.LBB911_382
.LBB911_380:
	movq	%rdx, %rbx
	movq	664(%rcx), %rdx
.Ltmp10587:
	movq	16(%rsp), %rsi
	leaq	864(%rsp), %rdi
	leaq	192(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10588:
	movq	864(%rsp), %r12
	movq	%rbx, %rdx
	cmpq	$-1, %r12
	jne	.LBB911_536
.LBB911_382:
	vmovdqu	888(%rsp), %xmm0
	movq	880(%rsp), %rax
	movq	872(%rsp), %rbx
	movq	904(%rsp), %r12
	movq	%rax, 32(%rsp)
	vmovdqa	%xmm0, 64(%rsp)
	cmpq	784(%rsp), %rbp
	jne	.LBB911_377
.Ltmp10592:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp10593:
	movq	792(%rsp), %rdx
	jmp	.LBB911_377
.LBB911_385:
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%r14), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB911_387
	.p2align	4
.LBB911_386:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB911_403
.LBB911_387:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB911_396
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB911_389
.LBB911_397:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB911_390
.LBB911_398:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB911_399
	.p2align	4
.LBB911_391:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB911_400
.LBB911_392:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB911_393
.LBB911_401:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB911_394
.LBB911_402:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB911_386
	jmp	.LBB911_395
.LBB911_396:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB911_397
	.p2align	4
.LBB911_389:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB911_398
.LBB911_390:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB911_391
.LBB911_399:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB911_392
.LBB911_400:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB911_401
	.p2align	4
.LBB911_393:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB911_402
.LBB911_394:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB911_386
.LBB911_395:
	movq	%r11, %r10
	jmp	.LBB911_386
.LBB911_403:
	testq	%rsi, %rsi
	je	.LBB911_408
.LBB911_404:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%r14), %rdi
	.p2align	4
.LBB911_405:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB911_407
.LBB911_406:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB911_405
	jmp	.LBB911_408
.LBB911_407:
	movq	$-1, %rax
	jmp	.LBB911_406
.LBB911_408:
	testq	%r10, %r10
	je	.LBB911_412
	movq	40(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB911_412
	cmpq	$0, 336(%rax)
	je	.LBB911_412
	lock		addq	%r10, 352(%rax)
.LBB911_412:
	movq	1672(%rsp), %rax
	movq	32(%rsp), %rdx
	movq	%r14, 176(%rsp)
	movq	$0, 96(%rsp)
	movq	$8, 104(%rsp)
	movq	$0, 112(%rsp)
	movq	%rax, 192(%rsp)
	movq	%rdx, 200(%rsp)
	testq	%rcx, %rcx
	je	.LBB911_421
	leaq	872(%rsp), %r15
	addq	$200, %r14
	movl	$8, %ecx
	xorl	%r12d, %r12d
	xorl	%ebp, %ebp
.LBB911_414:
	movq	-200(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB911_422
	leaq	-200(%r14), %r13
	vmovups	8(%r13), %zmm0
	vmovups	72(%r13), %zmm1
	vmovups	96(%r13), %zmm2
	vmovups	%zmm2, 88(%r15)
	vmovups	%zmm1, 64(%r15)
	vmovups	%zmm0, (%r15)
	movq	%rax, 864(%rsp)
	movzbl	1016(%rsp), %ebx
	cmpq	96(%rsp), %r12
	jne	.LBB911_418
.Ltmp10608:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp10609:
	movq	104(%rsp), %rcx
.LBB911_418:
	vmovdqu64	864(%rsp), %zmm0
	vmovdqu64	928(%rsp), %zmm1
	vmovdqu64	960(%rsp), %zmm2
	leaq	1(%r12), %rax
	vmovdqu64	%zmm2, 96(%rcx,%rbp)
	vmovdqu64	%zmm1, 64(%rcx,%rbp)
	vmovdqu64	%zmm0, (%rcx,%rbp)
	movq	%rax, 112(%rsp)
	testb	%bl, %bl
	jne	.LBB911_424
	addq	$160, %rbp
	addq	$200, %r14
	addq	$200, %r13
	movq	%rax, %r12
	cmpq	32(%rsp), %r13
	jne	.LBB911_414
	movq	32(%rsp), %r14
	movq	%rax, %r13
	movq	%rcx, %r15
	jmp	.LBB911_423
.LBB911_421:
	movl	$8, %r15d
	xorl	%r13d, %r13d
	jmp	.LBB911_423
.LBB911_422:
	movq	%rcx, %r15
	movq	%r12, %r13
.LBB911_423:
	movq	%r14, 184(%rsp)
	xorl	%ebx, %ebx
	jmp	.LBB911_425
.LBB911_424:
	incq	%r12
	movb	$1, %bl
	movq	%r14, 184(%rsp)
	movq	%rcx, %r15
	movq	%r12, %r13
.LBB911_425:
.Ltmp10616:
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp10617:
	movq	96(%rsp), %rax
	movq	%rax, 1128(%rsp)
	testq	%r13, %r13
	je	.LBB911_429
	cmpq	$8, %r13
	jae	.LBB911_430
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB911_451
.LBB911_429:
	xorl	%edx, %edx
	jmp	.LBB911_453
.LBB911_430:
	cmpq	$32, %r13
	jae	.LBB911_444
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB911_448
.LBB911_432:
	movq	24(%rsp), %r14
.LBB911_433:
	movq	%r14, 792(%rsp)
.Ltmp10769:
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10770:
	movq	1184(%rsp), %rax
	movq	1192(%rsp), %r15
	movq	$-1, %r12
	xorl	%r14d, %r14d
	movq	%rax, 16(%rsp)
.LBB911_435:
.Ltmp10777:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp10778:
	cmpq	$-1, %r12
	je	.LBB911_438
	vmovdqu	96(%rsp), %ymm0
	vmovdqu	112(%rsp), %ymm1
	jmp	.LBB911_466
.LBB911_438:
	movq	%r14, 24(%rsp)
	movq	%rbx, 512(%rsp)
	jmp	.LBB911_761
.LBB911_439:
	movq	24(%rsp), %r14
.LBB911_440:
	movq	%r14, 104(%rsp)
.Ltmp10598:
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10599:
	movq	784(%rsp), %rax
	movq	792(%rsp), %r15
	movq	$-1, %r12
	xorl	%r13d, %r13d
	xorl	%r14d, %r14d
	movq	%rax, 16(%rsp)
.LBB911_442:
.Ltmp10603:
	leaq	528(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp10604:
	cmpq	$-1, %r12
	jne	.LBB911_465
	jmp	.LBB911_760
.LBB911_444:
	vmovdqa64	.LCPI911_1(%rip), %zmm1
	vpbroadcastq	.LCPI911_2(%rip), %zmm2
	vpbroadcastq	.LCPI911_3(%rip), %zmm3
	movq	%r13, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB911_445:
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
	jne	.LBB911_445
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %r13
	je	.LBB911_453
	testb	$24, %r13b
	je	.LBB911_451
.LBB911_448:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI911_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI911_2(%rip), %zmm2
	vpbroadcastq	.LCPI911_4(%rip), %zmm3
	movq	%r13, %rax
	andq	$-8, %rax
	vmovq	%rdx, %xmm0
	subq	%rax, %rcx
.LBB911_449:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r15,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB911_449
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %r13
	je	.LBB911_453
.LBB911_451:
	movq	%r13, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r15), %rax
	.p2align	4
.LBB911_452:
	addq	(%rax), %rdx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB911_452
.LBB911_453:
	movq	1128(%rsp), %rcx
	movzbl	2267(%rsp), %eax
	movzbl	2265(%rsp), %ebp
	movl	%ebx, 404(%rsp)
	movq	$0, 176(%rsp)
	movq	$8, 184(%rsp)
	movq	%rdx, 512(%rsp)
	movq	%r15, 448(%rsp)
	movq	$0, 192(%rsp)
	movq	%rcx, 1328(%rsp)
	movq	%r15, 1336(%rsp)
	movq	%r13, 1344(%rsp)
	movb	%bl, 1352(%rsp)
	movq	1296(%rsp), %rbx
	movq	%rax, 1640(%rsp)
.Ltmp10625:
	leaq	864(%rsp), %rdi
	leaq	176(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp10626:
	movq	864(%rsp), %r12
	movq	872(%rsp), %rax
	movq	880(%rsp), %r15
	movq	888(%rsp), %r14
	movq	%r13, 288(%rsp)
	cmpq	$-1, %r12
	je	.LBB911_501
	movq	%rax, 16(%rsp)
	movzbl	896(%rsp), %eax
	vmovdqu	912(%rsp), %ymm0
	vmovdqu	928(%rsp), %ymm1
	movzbl	903(%rsp), %ebp
	movzwl	901(%rsp), %ebx
	movl	897(%rsp), %r13d
	movq	%rax, 24(%rsp)
	movq	904(%rsp), %rax
	vmovdqu	%ymm0, 784(%rsp)
	vmovdqu	%ymm1, 800(%rsp)
	movq	%rax, 296(%rsp)
.Ltmp10630:
	leaq	1160(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10631:
	shll	$16, %ebp
	orl	%ebp, %ebx
	movq	%r14, %rbp
	shlq	$32, %rbx
	orq	%rbx, %r13
	movq	%r13, 56(%rsp)
	movq	288(%rsp), %r13
.LBB911_457:
	testq	%r13, %r13
	je	.LBB911_461
	movq	448(%rsp), %r14
	movl	$1, %ebx
	subq	%r13, %rbx
	.p2align	4
.LBB911_459:
.Ltmp10738:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10739:
	incq	%rbx
	addq	$160, %r14
	cmpq	$1, %rbx
	jne	.LBB911_459
.LBB911_461:
	movq	1128(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_463
	movq	448(%rsp), %rdi
	shlq	$5, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB911_463:
	cmpq	$-1, %r12
	je	.LBB911_506
	vmovdqu	800(%rsp), %ymm1
	vmovdqu	784(%rsp), %ymm0
	movq	296(%rsp), %rbx
	movq	24(%rsp), %r14
	movq	56(%rsp), %r13
	vmovdqu	%ymm1, 1200(%rsp)
	vmovdqu	%ymm0, 1184(%rsp)
.LBB911_465:
	vmovdqu	1184(%rsp), %ymm0
	vmovdqu	1200(%rsp), %ymm1
	shlq	$8, %r13
	movzbl	%r14b, %r14d
	orq	%r13, %r14
.LBB911_466:
	movq	320(%rsp), %rax
	movq	16(%rsp), %rcx
	vmovdqu	%ymm1, 80(%rax)
	vmovdqu	%ymm0, 64(%rax)
	movq	%r12, 16(%rax)
	movq	%rcx, 24(%rax)
	movq	%r15, 32(%rax)
	movq	%rbp, 40(%rax)
	movq	%r14, 48(%rax)
	movq	%rbx, 56(%rax)
	movq	$1, (%rax)
.Ltmp10782:
	leaq	1360(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp10783:
	movq	456(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_470
.LBB911_468:
	lock		decq	(%rax)
	jne	.LBB911_470
	#MEMBARRIER
.Ltmp10841:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	456(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp10842:
.LBB911_470:
	movb	$1, %al
	movl	%eax, 16(%rsp)
.LBB911_471:
.Ltmp10846:
	leaq	1856(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp10847:
	movq	304(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB911_474
	#MEMBARRIER
.Ltmp10851:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	callq	*%rax
.Ltmp10852:
.LBB911_474:
	movb	$1, %r13b
.Ltmp10854:
	leaq	2072(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10855:
	movq	488(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB911_477
	#MEMBARRIER
.Ltmp10857:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	488(%rsp), %rdi
	callq	*%rax
.Ltmp10858:
.LBB911_477:
	cmpb	$0, 16(%rsp)
	je	.LBB911_3
	movq	472(%rsp), %rbx
	movq	480(%rsp), %r14
	testq	%r14, %r14
	je	.LBB911_491
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB911_483
	.p2align	4
.LBB911_480:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_481:
	callq	*%rbp
.LBB911_482:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB911_491
.LBB911_483:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_482
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_486
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_486:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_481
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_486
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
.LBB911_489:
	cmpq	%rax, %rdx
	jge	.LBB911_480
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB911_489
	jmp	.LBB911_480
.LBB911_491:
	movq	464(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_3
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_494
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_494:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_500
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_494
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
.LBB911_497:
	cmpq	%rax, %rdx
	jge	.LBB911_499
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB911_497
.LBB911_499:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_500:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB911_3
.LBB911_501:
	movq	1288(%rsp), %rdx
	movq	%rax, 328(%rsp)
	movq	1280(%rsp), %rax
	leaq	(%rbx,%rbx,4), %rcx
	movq	%r15, 336(%rsp)
	movq	%r14, 344(%rsp)
	leaq	(%rdx,%rcx,8), %rcx
	movq	%rdx, 416(%rsp)
	movq	%rax, 432(%rsp)
	movq	%rdx, 424(%rsp)
	movq	%rcx, 440(%rsp)
	movq	40(%rsp), %rcx
	movq	616(%rcx), %rax
	movq	%rax, 1304(%rsp)
	testq	%rax, %rax
	je	.LBB911_507
	movb	%bpl, 159(%rsp)
	lock		incq	(%rax)
	jle	.LBB911_882
	movq	40(%rsp), %rcx
	testq	%r13, %r13
	movq	%rdx, %r12
	sete	%al
	movq	616(%rcx), %rbp
	movq	%rbp, 1240(%rsp)
	movq	%rbp, 160(%rsp)
	movq	16(%rbp), %rcx
	movq	%rcx, 504(%rsp)
	movq	40(%rbp), %rcx
	cmpq	$-1, %rcx
	movq	%rcx, 408(%rsp)
	sete	%cl
	orb	%al, %cl
	jne	.LBB911_558
	cmpq	$8, %r13
	jae	.LBB911_545
	movq	448(%rsp), %rdx
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB911_554
.LBB911_506:
	movl	404(%rsp), %ecx
	movq	24(%rsp), %rdx
	jmp	.LBB911_748
.LBB911_507:
	movq	424(%rsp), %rcx
	movq	416(%rsp), %rax
	movq	432(%rsp), %rdx
	movq	%r14, 32(%rsp)
	movq	%rcx, 104(%rsp)
	movq	440(%rsp), %rcx
	movq	%rax, 96(%rsp)
	movq	%rdx, 112(%rsp)
	movq	%rcx, 120(%rsp)
	movq	120(%rsp), %rax
	movq	104(%rsp), %rbp
	movq	%rax, 88(%rsp)
	cmpq	%rax, %rbp
	je	.LBB911_517
	movq	32(%rsp), %rax
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %r14
	jmp	.LBB911_511
.LBB911_509:
	movq	336(%rsp), %rcx
.LBB911_510:
	movq	64(%rsp), %rax
	shll	$16, %ebx
	movq	56(%rsp), %rdx
	addq	$40, %rbp
	orl	%ebx, %r13d
	shlq	$32, %r13
	orq	%r13, %r12
	movq	%rax, (%rcx,%r14)
	movq	24(%rsp), %rax
	movq	%rax, 8(%rcx,%r14)
	movq	296(%rsp), %rax
	movq	%rax, 16(%rcx,%r14)
	movq	%r12, %rax
	shrq	$48, %rax
	movb	%r15b, 24(%rcx,%r14)
	movl	%r12d, 25(%rcx,%r14)
	shrq	$32, %r12
	movq	%rcx, %r15
	movb	%al, 31(%rcx,%r14)
	movq	32(%rsp), %rax
	movw	%r12w, 29(%rcx,%r14)
	movq	%rdx, 32(%rcx,%r14)
	addq	$40, %r14
	incq	%rax
	movq	%rax, 32(%rsp)
	movq	%rax, 344(%rsp)
	cmpq	88(%rsp), %rbp
	je	.LBB911_517
.LBB911_511:
	movq	32(%rbp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	40(%rsp), %rax
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 176(%rsp)
	cmpq	$0, 184(%rsp)
	je	.LBB911_513
	movq	32(%rbp), %rax
	leaq	872(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB911_515
.LBB911_513:
	movq	664(%rax), %rdx
.Ltmp10717:
	movq	16(%rsp), %rsi
	leaq	864(%rsp), %rdi
	leaq	192(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10718:
	movq	864(%rsp), %r12
	cmpq	$-1, %r12
	jne	.LBB911_810
.LBB911_515:
	movq	880(%rsp), %rsi
	movq	888(%rsp), %rdx
	movq	%r15, %rcx
	movq	872(%rsp), %rax
	movzbl	896(%rsp), %r15d
	movzbl	903(%rsp), %ebx
	movzwl	901(%rsp), %r13d
	movl	897(%rsp), %r12d
	movq	%rsi, 24(%rsp)
	movq	904(%rsp), %rsi
	movq	%rdx, 296(%rsp)
	movq	32(%rsp), %rdx
	movq	%rax, 64(%rsp)
	movq	%rsi, 56(%rsp)
	cmpq	328(%rsp), %rdx
	jne	.LBB911_510
.Ltmp10725:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	328(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp10726:
	jmp	.LBB911_509
.LBB911_517:
	movb	$1, %r14b
	movq	%rbp, 104(%rsp)
.Ltmp10730:
	leaq	96(%rsp), %rdi
	movb	$1, %bpl
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10731:
	movq	288(%rsp), %r13
	movq	328(%rsp), %rax
	movq	336(%rsp), %r15
	movq	32(%rsp), %rbp
	movq	$-1, %r12
	movq	%rax, 16(%rsp)
	movb	$2, %al
	movq	%rax, 24(%rsp)
	jmp	.LBB911_457
.LBB911_519:
	vmovups	576(%rsp), %ymm0
	movq	544(%rsp), %rcx
	movq	536(%rsp), %rax
	movq	552(%rsp), %rdx
	movq	568(%rsp), %r13
	movq	%r14, 792(%rsp)
	movq	%rcx, 64(%rsp)
	movq	560(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rdx, 32(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovdqu	592(%rsp), %ymm0
	movq	%rcx, 24(%rsp)
	vmovdqu	%ymm0, 192(%rsp)
.Ltmp10761:
	leaq	784(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10762:
	movq	%rbx, %rdi
	testq	%rbp, %rbp
	je	.LBB911_533
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	xorl	%r14d, %r14d
	jmp	.LBB911_525
.LBB911_522:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_523:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	%rbx, %rdi
.LBB911_524:
	incq	%r14
	cmpq	%r14, %rbp
	je	.LBB911_533
.LBB911_525:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rdi,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_524
	leaq	(%rdi,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_528
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_528:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_523
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_528
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
	movq	(%r15), %rax
.LBB911_531:
	cmpq	%rax, %rdx
	jge	.LBB911_522
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB911_531
	jmp	.LBB911_522
.LBB911_533:
	movq	1184(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_535
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB911_535:
	movq	32(%rsp), %rbp
	movq	64(%rsp), %r15
	movq	24(%rsp), %r14
	movq	%r13, %rbx
	jmp	.LBB911_360
.LBB911_536:
	vmovups	912(%rsp), %ymm0
	movq	880(%rsp), %rcx
	movq	872(%rsp), %rax
	movq	888(%rsp), %rdx
	movq	904(%rsp), %r13
	movq	%r14, 104(%rsp)
	movq	%rcx, 64(%rsp)
	movq	896(%rsp), %rcx
	movq	%rax, 16(%rsp)
	movq	%rdx, 32(%rsp)
	vmovups	%ymm0, 176(%rsp)
	vmovdqu	928(%rsp), %ymm0
	movq	%rcx, 24(%rsp)
	vmovdqu	%ymm0, 192(%rsp)
.Ltmp10590:
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10591:
	movq	%rbx, %rdi
	testq	%rbp, %rbp
	je	.LBB911_542
	leaq	8(%rdi), %r14
	xorl	%r15d, %r15d
	jmp	.LBB911_540
.LBB911_539:
	incq	%r15
	addq	$40, %r14
	cmpq	%r15, %rbp
	je	.LBB911_542
.LBB911_540:
	movq	-8(%r14), %rax
	cmpq	$6, %rax
	jb	.LBB911_539
	movq	(%r14), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	movq	%rbx, %rdi
	jmp	.LBB911_539
.LBB911_542:
	movq	784(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_544
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB911_544:
	movq	32(%rsp), %rbp
	movq	64(%rsp), %r15
	movq	24(%rsp), %r14
	movq	%r13, %rbx
	jmp	.LBB911_364
.LBB911_545:
	cmpq	$32, %r13
	jae	.LBB911_547
	movq	448(%rsp), %rdx
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB911_551
.LBB911_547:
	vmovdqa64	.LCPI911_1(%rip), %zmm1
	vpbroadcastq	.LCPI911_2(%rip), %zmm2
	vpbroadcastq	.LCPI911_3(%rip), %zmm3
	movq	448(%rsp), %rdx
	movq	%r13, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB911_548:
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
	jne	.LBB911_548
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
	cmpq	%rax, %r13
	je	.LBB911_556
	testb	$24, %r13b
	je	.LBB911_554
.LBB911_551:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI911_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI911_2(%rip), %zmm2
	vpbroadcastq	.LCPI911_4(%rip), %zmm3
	movq	%r13, %rax
	andq	$-8, %rax
	vmovq	%rbx, %xmm0
	subq	%rax, %rcx
.LBB911_552:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rdx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB911_552
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %r13
	je	.LBB911_556
.LBB911_554:
	movq	%r13, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rdx), %rax
.LBB911_555:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB911_555
.LBB911_556:
	movq	40(%rsp), %rcx
	movq	912(%rcx), %rax
	movq	928(%rcx), %rsi
	leaq	912(%rcx), %r15
	subq	%rsi, %rax
	cmpq	%rax, %rbx
	ja	.LBB911_874
.LBB911_557:
	movq	40(%rsp), %rax
	cmpq	1016(%rax), %rbx
	ja	.LBB911_875
.LBB911_558:
	leaq	16(%rbp), %rax
	movq	1128(%rsp), %rcx
	leaq	(%r13,%r13,4), %rdx
	movq	%r12, 24(%rsp)
	movq	%rax, 280(%rsp)
	movq	448(%rsp), %rax
	shlq	$5, %rdx
	addq	%rax, %rdx
	movq	%rax, 832(%rsp)
	movq	%rax, 840(%rsp)
	movq	%rcx, 848(%rsp)
	movq	%rdx, 1608(%rsp)
	movq	%rdx, 856(%rsp)
	testq	%r13, %r13
	je	.LBB911_741
	leaq	272(%rbp), %rcx
	leaq	872(%rsp), %rbx
	movq	%rax, %r14
	movq	%rcx, 1144(%rsp)
.LBB911_560:
	leaq	160(%r14), %rdx
	movq	%rdx, 840(%rsp)
	movq	(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB911_741
	movq	%rax, 864(%rsp)
	movq	%rdx, 1616(%rsp)
	vmovdqu64	8(%r14), %zmm0
	vmovdqu64	72(%r14), %zmm1
	vmovdqu64	96(%r14), %zmm2
	vmovdqu64	%zmm2, 88(%rbx)
	vmovdqu64	%zmm1, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
	imulq	$88, 880(%rsp), %rsi
	movq	872(%rsp), %rcx
	movq	888(%rsp), %r14
	movq	896(%rsp), %r12
	movq	904(%rsp), %rdx
	movq	920(%rsp), %rdi
	movq	928(%rsp), %r8
	movq	%rcx, 352(%rsp)
	movq	%rax, 368(%rsp)
	movq	%rcx, 360(%rsp)
	movq	%rdx, 1632(%rsp)
	movq	%r12, 64(%rsp)
	movq	%r14, 272(%rsp)
	addq	%rcx, %rsi
	movq	%rsi, 168(%rsp)
	movq	%rsi, 376(%rsp)
	movq	912(%rsp), %rsi
	testq	%r8, %r8
	je	.LBB911_729
	movq	960(%rsp), %rax
	shlq	$5, %r8
	leaq	8(%r12), %rdx
	movq	%rdi, %r15
	movq	$0, 1120(%rsp)
	movq	%rcx, 312(%rsp)
	movq	%rcx, 80(%rsp)
	movq	%rsi, 520(%rsp)
	movq	%rdi, 1136(%rsp)
	addq	%rdi, %r8
	movq	%rdx, 1624(%rsp)
	movq	%r8, 48(%rsp)
	movq	%rax, 144(%rsp)
	jmp	.LBB911_565
.LBB911_563:
	movq	520(%rsp), %rsi
	movq	1136(%rsp), %rdi
.LBB911_564:
	movq	1648(%rsp), %r15
	movq	%rbx, 24(%rsp)
	movq	%rbx, 424(%rsp)
	addq	$32, %r15
	cmpq	%r8, %r15
	je	.LBB911_729
.LBB911_565:
	movq	16(%r15), %rax
	movq	24(%r15), %rcx
	movq	1120(%rsp), %r12
	movq	(%r15), %r14
	movq	8(%r15), %rbx
	movq	%rax, 56(%rsp)
	movq	%rcx, 32(%rsp)
	testq	%r14, %r14
	je	.LBB911_575
	cmpq	$-1, 504(%rsp)
	je	.LBB911_575
	movq	80(%rbp), %rax
	movq	$-1, %rdx
	.p2align	4
.LBB911_568:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%rbp)
	jne	.LBB911_568
	movq	280(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB911_572
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp10637:
	movq	280(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp10638:
	cmpb	$-1, 96(%rsp)
	jne	.LBB911_865
.LBB911_572:
	movq	40(%rsp), %rax
	movq	632(%rax), %rax
	testq	%rax, %rax
	je	.LBB911_575
	movq	40(%rsp), %rcx
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB911_575
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	1640(%rsp), %rax
	lock		addq	%r14, (%rcx,%rax,8)
.LBB911_575:
	movq	1632(%rsp), %rdx
	cmpq	%rdx, %r12
	ja	.LBB911_872
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	leaq	.LJTI911_0(%rip), %r8
	movq	%r15, 1648(%rsp)
	cmovbq	%rbx, %rsi
	cmpq	%r12, %rbx
	cmovbq	%r12, %rsi
	cmpq	%r12, %rsi
	jb	.LBB911_871
	leaq	(,%r12,8), %rax
	movq	%r12, %rcx
	movq	$-1, %rdi
	movq	%rcx, 296(%rsp)
	leaq	(%rax,%rax,2), %r12
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r10
	cmpq	%rsi, %rcx
	jne	.LBB911_584
	xorl	%r14d, %r14d
	xorl	%ebx, %ebx
	xorl	%r11d, %r11d
.LBB911_579:
	cmpq	$-1, 408(%rsp)
	movq	$-1, %r13
	movq	%rbx, 392(%rsp)
	movq	%r11, 1312(%rsp)
	movq	%rsi, 1120(%rsp)
	movq	%r10, 88(%rsp)
	je	.LBB911_590
	movq	56(%rsp), %rax
	movl	$0, %ecx
	movq	168(%rsp), %r15
	movl	$0, %ebp
	subq	144(%rsp), %rax
	cmovbq	%rcx, %rax
	subq	80(%rsp), %r15
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %r15
	imulq	%rcx, %r15
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	testq	%r15, %r15
	je	.LBB911_591
	movq	80(%rsp), %rax
	xorl	%ebp, %ebp
	leaq	8(%rax), %rbx
	.p2align	4
.LBB911_582:
.Ltmp10640:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp10641:
	addq	%rax, %rbp
	cmovbq	%r13, %rbp
	addq	$88, %rbx
	decq	%r15
	jne	.LBB911_582
	jmp	.LBB911_591
.LBB911_584:
	movq	%r10, %rdx
	subq	%r12, %rdx
	movabsq	$-6148914691236517205, %rax
	xorl	%r11d, %r11d
	xorl	%ebx, %ebx
	xorl	%r14d, %r14d
	mulxq	%rax, %rax, %rax
	movq	1624(%rsp), %rcx
	shrq	$4, %rax
	addq	%r12, %rcx
	jmp	.LBB911_587
.LBB911_585:
	addq	%rdx, %rbx
	cmovbq	%rdi, %rbx
.LBB911_586:
	addq	$24, %rcx
	decq	%rax
	je	.LBB911_579
.LBB911_587:
	movzbl	-8(%rcx), %r9d
	movq	(%rcx), %rdx
	movslq	(%r8,%r9,4), %r9
	addq	%r8, %r9
	jmpq	*%r9
.LBB911_588:
	addq	%rdx, %r14
	cmovbq	%rdi, %r14
	jmp	.LBB911_586
.LBB911_589:
	cmpq	%rdx, %r11
	cmovbeq	%rdx, %r11
	jmp	.LBB911_586
.LBB911_590:
	xorl	%ebp, %ebp
.LBB911_591:
	movq	160(%rsp), %rax
	movq	$-1, %rbx
	movl	296(%rax), %eax
	movq	88(%rsp), %r15
	testl	%eax, %eax
	je	.LBB911_616
.LBB911_592:
	cmpq	$-1, 504(%rsp)
	je	.LBB911_594
	movq	160(%rsp), %rcx
	movq	80(%rcx), %rax
	addq	%r14, %rax
	cmovbq	%rbx, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB911_617
.LBB911_594:
	cmpq	$-1, 408(%rsp)
	je	.LBB911_596
	movq	40(%rsp), %rcx
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	160(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rbx, %rcx
	addq	392(%rsp), %rcx
	cmovbq	%rbx, %rcx
	addq	1312(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%rbx, %rcx
	addq	%rcx, %rax
	cmovbq	%rbx, %rax
	cmpq	40(%rdx), %rax
	ja	.LBB911_617
.LBB911_596:
	cmpq	$-1, 504(%rsp)
	movq	160(%rsp), %rbp
	movq	144(%rsp), %r9
	je	.LBB911_598
	movq	296(%rsp), %rcx
	movq	%r12, %rax
	cmpq	1120(%rsp), %rcx
	jne	.LBB911_611
.LBB911_598:
	movb	$1, %bl
	movq	296(%rsp), %rax
	cmpq	1120(%rsp), %rax
	je	.LBB911_601
.LBB911_599:
	movq	64(%rsp), %rax
	cmpb	$2, -24(%rax,%r15)
	je	.LBB911_681
	addq	$-24, %r15
	cmpq	%r15, %r12
	jne	.LBB911_599
.LBB911_601:
	movq	288(%rsp), %r13
	movq	64(%rsp), %r12
	movq	392(%rsp), %r15
.LBB911_602:
	cmpq	$-1, 504(%rsp)
	je	.LBB911_671
	testq	%r14, %r14
	je	.LBB911_671
	movq	80(%rbp), %rax
	movq	$-1, %rdx
.LBB911_605:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%rbp)
	jne	.LBB911_605
	movq	280(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB911_671
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp10646:
	movq	280(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp10647:
	cmpb	$-1, 96(%rsp)
	je	.LBB911_671
	cmpq	$-1, 408(%rsp)
	movq	272(%rsp), %r14
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	movb	$1, %r15b
	jne	.LBB911_703
	jmp	.LBB911_864
.LBB911_610:
	addq	$24, %rax
	cmpq	%rax, %r15
	je	.LBB911_598
.LBB911_611:
	movq	64(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB911_610
	movq	64(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB911_610
	movq	40(%rsp), %rdx
	movq	632(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB911_610
	movq	40(%rsp), %rsi
	movl	1228(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB911_610
	movq	64(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB911_610
.LBB911_616:
	movq	1144(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB911_592
.LBB911_617:
	movq	144(%rsp), %r13
	movq	296(%rsp), %rax
	cmpq	1120(%rsp), %rax
	jne	.LBB911_627
	movq	160(%rsp), %rbp
.LBB911_619:
	movq	%r13, 144(%rsp)
	cmpq	$-1, 408(%rsp)
	movq	288(%rsp), %r13
	je	.LBB911_678
	movq	56(%rsp), %rax
	movq	64(%rsp), %r12
	movq	272(%rsp), %r14
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	cmpq	%rax, 144(%rsp)
	jae	.LBB911_715
	movq	80(%rsp), %rax
	cmpq	168(%rsp), %rax
	je	.LBB911_680
	movq	56(%rsp), %rax
	movq	80(%rsp), %rcx
	leaq	-1(%rax), %r15
.LBB911_623:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB911_689
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbx
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp10678:
	movq	16(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp10679:
	movq	144(%rsp), %rax
	cmpq	%rax, %r15
	je	.LBB911_688
	movq	%rbx, %rdx
	leaq	88(%rbx), %rcx
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	incq	%rax
	movq	%rax, 144(%rsp)
	cmpq	168(%rsp), %rcx
	jne	.LBB911_623
	jmp	.LBB911_689
.LBB911_627:
	movq	64(%rsp), %rax
	movq	160(%rsp), %rbp
	addq	%rax, %r12
	addq	%rax, %r15
	jmp	.LBB911_630
.LBB911_628:
	movq	160(%rsp), %rbp
	movq	$-1, %rbx
.LBB911_629:
	addq	$24, %r12
	cmpq	%r15, %r12
	je	.LBB911_619
.LBB911_630:
	movzbl	(%r12), %eax
	leaq	.LJTI911_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB911_631:
	cmpq	$-1, 504(%rsp)
	je	.LBB911_629
	movq	%rbp, %rdx
	movzbl	1(%r12), %ebx
	movq	8(%r12), %rbp
	movq	16(%r12), %r14
	movq	80(%rdx), %rax
	movq	$-1, %rsi
	.p2align	4
.LBB911_633:
	movq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB911_633
	movq	280(%rsp), %rcx
	addq	%rbp, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB911_637
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp10672:
	movq	280(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp10673:
	cmpb	$-1, 96(%rsp)
	jne	.LBB911_812
.LBB911_637:
	cmpl	$255, %ebx
	je	.LBB911_628
	movq	40(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB911_628
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB911_628
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%rbp, (%rcx,%rbx,8)
	jmp	.LBB911_628
.LBB911_641:
	movq	8(%r12), %rbx
	cmpq	%rbx, %r13
	jae	.LBB911_668
	movq	80(%rsp), %rax
	cmpq	168(%rsp), %rax
	je	.LBB911_661
	movq	80(%rsp), %rax
.LBB911_644:
	movq	%rax, %rdx
	movq	8(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB911_666
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%r15, %r14
	movq	%rbp, %r15
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp10661:
	movq	16(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp10662:
	leaq	-1(%rbx), %rax
	cmpq	%r13, %rax
	je	.LBB911_665
	leaq	88(%rbp), %rax
	incq	%r13
	movq	%rbp, %rdx
	movq	%r15, %rbp
	movq	%r14, %r15
	cmpq	168(%rsp), %rax
	jne	.LBB911_644
	jmp	.LBB911_666
.LBB911_648:
	cmpq	$-1, 408(%rsp)
	je	.LBB911_629
	movq	8(%r12), %rcx
	movl	296(%rbp), %eax
	testl	%eax, %eax
	je	.LBB911_659
	movq	104(%rbp), %rax
	addq	%rcx, %rax
	movq	40(%rbp), %rcx
	cmovbq	%rbx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB911_629
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp10659:
	movq	280(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp10660:
	jmp	.LBB911_660
.LBB911_652:
	cmpq	$-1, 408(%rsp)
	je	.LBB911_629
	cmpq	$-1, 40(%rbp)
	je	.LBB911_629
	movq	8(%r12), %rcx
	movq	16(%r12), %r14
	movl	296(%rbp), %eax
	testl	%eax, %eax
	je	.LBB911_662
	movq	104(%rbp), %rax
	.p2align	4
.LBB911_656:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rbx, %rdx
	lock		cmpxchgq	%rdx, 104(%rbp)
	jne	.LBB911_656
	addq	%rcx, %rax
	movq	40(%rbp), %rcx
	cmovbq	%rbx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB911_629
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp10666:
	movq	280(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp10667:
	jmp	.LBB911_663
.LBB911_659:
	movq	1144(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 112(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
.LBB911_660:
	cmpb	$-1, 96(%rsp)
	je	.LBB911_629
	jmp	.LBB911_828
.LBB911_661:
	movq	312(%rsp), %rdx
	jmp	.LBB911_667
.LBB911_662:
	movq	1144(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 112(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
.LBB911_663:
	movzbl	96(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%r14, %r14
	setne	%dl
	testb	%cl, %dl
	jne	.LBB911_820
	cmpb	$-1, %al
	je	.LBB911_629
	jmp	.LBB911_828
.LBB911_665:
	movq	%rbp, %rdx
	movq	%r15, %rbp
	movq	%rbx, %r13
	movq	%r14, %r15
.LBB911_666:
	addq	$88, %rdx
	movq	%rdx, 80(%rsp)
.LBB911_667:
	movq	%rdx, 312(%rsp)
	movq	%rdx, 360(%rsp)
.LBB911_668:
	cmpq	$-1, 408(%rsp)
	movq	$-1, %rbx
	je	.LBB911_629
.Ltmp10664:
	movq	40(%rsp), %rsi
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp10665:
	cmpb	$-1, 176(%rsp)
	je	.LBB911_629
	jmp	.LBB911_828
.LBB911_671:
	cmpq	$-1, 408(%rsp)
	je	.LBB911_679
	cmpq	$-1, 40(%rbp)
	movq	272(%rsp), %r14
	je	.LBB911_695
	movl	296(%rbp), %eax
	testl	%eax, %eax
	je	.LBB911_692
	movq	104(%rbp), %rax
	movq	$-1, %rdx
.LBB911_675:
	movq	%rax, %rcx
	addq	%r15, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 104(%rbp)
	jne	.LBB911_675
	movq	40(%rbp), %rcx
	addq	%r15, %rax
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB911_695
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp10649:
	movq	280(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp10650:
	jmp	.LBB911_693
.LBB911_678:
	movq	64(%rsp), %r12
.LBB911_679:
	movq	272(%rsp), %r14
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	jmp	.LBB911_715
.LBB911_680:
	movq	312(%rsp), %rdx
	jmp	.LBB911_690
.LBB911_681:
	movq	64(%rsp), %r12
	movq	-16(%r12,%r15), %rbx
	cmpq	%rbx, %r9
	jae	.LBB911_691
	movq	288(%rsp), %r13
	movq	392(%rsp), %r15
	movq	80(%rsp), %rax
	cmpq	168(%rsp), %rax
	je	.LBB911_725
	movq	80(%rsp), %rcx
.LBB911_684:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB911_727
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %r15
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp10643:
	movq	16(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp10644:
	movq	144(%rsp), %rax
	leaq	-1(%rbx), %rcx
	cmpq	%rax, %rcx
	je	.LBB911_726
	movq	%r15, %rdx
	leaq	88(%r15), %rcx
	movq	392(%rsp), %r15
	incq	%rax
	movq	%rax, 144(%rsp)
	cmpq	168(%rsp), %rcx
	jne	.LBB911_684
	jmp	.LBB911_727
.LBB911_688:
	movq	56(%rsp), %rax
	movq	%rbx, %rdx
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	movq	%rax, 144(%rsp)
.LBB911_689:
	addq	$88, %rdx
	movq	%rdx, 80(%rsp)
.LBB911_690:
	movq	%rdx, 312(%rsp)
	movq	%rdx, 360(%rsp)
	jmp	.LBB911_715
.LBB911_691:
	movq	288(%rsp), %r13
	movq	392(%rsp), %r15
	xorl	%ebx, %ebx
	jmp	.LBB911_602
.LBB911_692:
	movq	1144(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 112(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
.LBB911_693:
	cmpb	$-1, 96(%rsp)
	je	.LBB911_695
	movb	$1, %r15b
	jmp	.LBB911_701
.LBB911_695:
	movq	48(%rsp), %r8
	testb	%bl, %bl
	movq	24(%rsp), %rbx
	jne	.LBB911_698
.Ltmp10651:
	movq	40(%rsp), %rsi
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp10652:
	cmpb	$-1, 176(%rsp)
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	movb	$1, %r15b
	jne	.LBB911_703
.LBB911_698:
	movq	1312(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB911_702
.Ltmp10653:
	movq	280(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp10654:
	cmpb	$-1, 176(%rsp)
	setne	%r15b
.LBB911_701:
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	jmp	.LBB911_703
.LBB911_702:
	xorl	%r15d, %r15d
.LBB911_703:
	movq	56(%rsp), %rax
	cmpq	%rax, 144(%rsp)
	jae	.LBB911_714
	movq	80(%rsp), %rax
	cmpq	168(%rsp), %rax
	je	.LBB911_710
	movq	56(%rsp), %rax
	movq	80(%rsp), %rcx
	leaq	-1(%rax), %r14
.LBB911_706:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB911_712
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbx
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp10656:
	movq	16(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp10657:
	movq	144(%rsp), %rax
	cmpq	%rax, %r14
	je	.LBB911_711
	movq	%rbx, %rdx
	leaq	88(%rbx), %rcx
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	incq	%rax
	movq	%rax, 144(%rsp)
	cmpq	168(%rsp), %rcx
	jne	.LBB911_706
	jmp	.LBB911_712
.LBB911_710:
	movq	312(%rsp), %rdx
	jmp	.LBB911_713
.LBB911_711:
	movq	56(%rsp), %rax
	movq	%rbx, %rdx
	movq	48(%rsp), %r8
	movq	24(%rsp), %rbx
	movq	%rax, 144(%rsp)
.LBB911_712:
	movq	272(%rsp), %r14
	addq	$88, %rdx
	movq	%rdx, 80(%rsp)
.LBB911_713:
	movq	%rdx, 312(%rsp)
	movq	%rdx, 360(%rsp)
.LBB911_714:
	testb	%r15b, %r15b
	jne	.LBB911_864
.LBB911_715:
	cmpq	$0, 32(%rsp)
	je	.LBB911_563
	movq	440(%rsp), %rax
	movq	520(%rsp), %rsi
	movq	1136(%rsp), %rdi
	movq	%rax, 392(%rsp)
	jmp	.LBB911_718
.LBB911_717:
	movq	336(%rsp), %rax
	movq	24(%rsp), %rdx
	leaq	(%r15,%r15,4), %rcx
	movq	32(%rsp), %rsi
	shll	$16, %ebp
	movq	296(%rsp), %rdi
	incq	%r15
	addq	$40, %rbx
	movq	48(%rsp), %r8
	orl	%ebp, %r12d
	movq	160(%rsp), %rbp
	shlq	$32, %r12
	orq	%r12, %r13
	movq	64(%rsp), %r12
	movq	%rdx, (%rax,%rcx,8)
	movq	56(%rsp), %rdx
	decq	%rsi
	movq	%rsi, 32(%rsp)
	movq	%rdx, 8(%rax,%rcx,8)
	movzbl	88(%rsp), %edx
	movq	%r14, 16(%rax,%rcx,8)
	movq	272(%rsp), %r14
	movb	%dl, 24(%rax,%rcx,8)
	movq	%r13, %rdx
	shrq	$48, %rdx
	movl	%r13d, 25(%rax,%rcx,8)
	shrq	$32, %r13
	movw	%r13w, 29(%rax,%rcx,8)
	movb	%dl, 31(%rax,%rcx,8)
	movq	%rdi, 32(%rax,%rcx,8)
	movq	%r15, 344(%rsp)
	movq	288(%rsp), %r13
	movq	1136(%rsp), %rdi
	testq	%rsi, %rsi
	movq	520(%rsp), %rsi
	je	.LBB911_564
.LBB911_718:
	cmpq	392(%rsp), %rbx
	je	.LBB911_564
	movq	32(%rbx), %rax
	leaq	104(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	40(%rsp), %rax
	vmovdqu	(%rbx), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 96(%rsp)
	cmpq	$0, 104(%rsp)
	je	.LBB911_721
	movq	32(%rbx), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbx), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB911_723
.LBB911_721:
	movq	664(%rax), %rdx
.Ltmp10681:
	movq	16(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	112(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10682:
	movq	176(%rsp), %r12
	cmpq	$-1, %r12
	jne	.LBB911_808
.LBB911_723:
	movq	184(%rsp), %rax
	movq	192(%rsp), %rsi
	movzbl	208(%rsp), %edx
	movq	216(%rsp), %rcx
	movq	200(%rsp), %r14
	movzbl	215(%rsp), %ebp
	movzwl	213(%rsp), %r12d
	movl	209(%rsp), %r13d
	movq	344(%rsp), %r15
	movq	%rax, 24(%rsp)
	movq	%rsi, 56(%rsp)
	movb	%dl, 88(%rsp)
	movq	%rcx, 296(%rsp)
	cmpq	328(%rsp), %r15
	jne	.LBB911_717
.Ltmp10691:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	328(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp10692:
	jmp	.LBB911_717
.LBB911_725:
	movq	312(%rsp), %rdx
	jmp	.LBB911_728
.LBB911_726:
	movq	%r15, %rdx
	movq	392(%rsp), %r15
	movq	%rbx, 144(%rsp)
.LBB911_727:
	addq	$88, %rdx
	movq	%rdx, 80(%rsp)
.LBB911_728:
	xorl	%ebx, %ebx
	movq	%rdx, 312(%rsp)
	movq	%rdx, 360(%rsp)
	jmp	.LBB911_602
.LBB911_729:
	testq	%rsi, %rsi
	je	.LBB911_731
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB911_731:
.Ltmp10701:
	leaq	352(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp10702:
	testq	%r14, %r14
	je	.LBB911_734
	shlq	$3, %r14
	movl	$8, %edx
	movq	%r12, %rdi
	leaq	(%r14,%r14,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB911_734:
	movq	952(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_737
	lock		decq	(%rax)
	jne	.LBB911_737
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	952(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB911_737:
	movq	984(%rsp), %rax
	movq	1616(%rsp), %r14
	leaq	872(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB911_740
	lock		decq	(%rax)
	jne	.LBB911_740
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	984(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB911_740:
	cmpq	1608(%rsp), %r14
	jne	.LBB911_560
.LBB911_741:
	movb	$1, %al
	xorl	%r14d, %r14d
	movl	%eax, 48(%rsp)
.Ltmp10706:
	leaq	832(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp10707:
	cmpq	$-1, 504(%rsp)
	movzbl	159(%rsp), %ecx
	sete	%al
	xorb	$1, %cl
	orb	404(%rsp), %cl
	orb	%al, %cl
	cmpb	$1, %cl
	je	.LBB911_744
	movq	$1, 176(%rsp)
	xorl	%r14d, %r14d
	movq	$0, 184(%rsp)
.Ltmp10708:
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movq	280(%rsp), %rsi
	leaq	864(%rsp), %rdi
	leaq	176(%rsp), %rdx
	movl	$1, %ecx
	callq	*%rax
.Ltmp10709:
.LBB911_744:
	movq	328(%rsp), %rax
	movq	336(%rsp), %r15
	movq	344(%rsp), %rbx
	movq	%rax, 16(%rsp)
	lock		decq	(%rbp)
	jne	.LBB911_746
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp10713:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1240(%rsp), %rdi
	xorl	%ebp, %ebp
	callq	*%rax
.Ltmp10714:
.LBB911_746:
	xorl	%r14d, %r14d
.Ltmp10715:
	leaq	416(%rsp), %rdi
	movl	$0, 88(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10716:
	movl	404(%rsp), %ecx
	movb	$2, %dl
	movq	%rbx, %rbp
.LBB911_748:
	movq	16(%rsp), %rax
	movq	40(%rsp), %rsi
	movq	%rax, 176(%rsp)
	movq	616(%rsi), %rax
	movq	%r15, 184(%rsp)
	movq	%rbp, 192(%rsp)
	testq	%rax, %rax
	je	.LBB911_754
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	jne	.LBB911_751
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 111(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
.LBB911_751:
	testb	%dl, %dl
	je	.LBB911_758
	movzbl	%dl, %eax
	cmpl	$2, %eax
	jne	.LBB911_755
	movl	404(%rsp), %eax
	jmp	.LBB911_759
.LBB911_754:
	cmpb	$2, %dl
	movb	$-1, %bl
	sete	%al
	andb	%cl, %al
	jmp	.LBB911_759
.LBB911_755:
	cmpb	$-1, %bl
	je	.LBB911_809
	movq	40(%rsp), %rax
	cmpb	$2, 472(%rax)
	jne	.LBB911_758
	movq	40(%rsp), %rax
	vmovdqa	96(%rsp), %xmm0
	movb	%bl, 864(%rsp)
	movq	696(%rax), %rdi
	movq	111(%rsp), %rax
	vmovdqu	%xmm0, 865(%rsp)
	movq	%rax, 880(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	je	.LBB911_758
	addq	$16, %rdi
.Ltmp10744:
	leaq	864(%rsp), %rsi
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.13412714042204560522)
.Ltmp10745:
	jmp	.LBB911_758
.LBB911_808:
	movq	184(%rsp), %rax
	movq	200(%rsp), %rcx
	vmovups	224(%rsp), %ymm0
	movzbl	215(%rsp), %edx
	movq	192(%rsp), %r15
	movq	160(%rsp), %rbp
	addq	$40, %rbx
	movq	%rbx, 424(%rsp)
	movq	%rax, 16(%rsp)
	movzbl	208(%rsp), %eax
	movq	%rcx, 32(%rsp)
	movzwl	213(%rsp), %ecx
	shll	$16, %edx
	vmovups	%ymm0, 784(%rsp)
	vmovdqu	240(%rsp), %ymm0
	movq	%rax, 24(%rsp)
	movl	209(%rsp), %eax
	orl	%edx, %ecx
	shlq	$32, %rcx
	orq	%rcx, %rax
	movq	%rax, 56(%rsp)
	movq	216(%rsp), %rax
	vmovdqu	%ymm0, 800(%rsp)
	movq	%rax, 296(%rsp)
	movb	$1, %al
	movl	%eax, 48(%rsp)
	jmp	.LBB911_830
.LBB911_809:
	movb	$-1, %bl
.LBB911_758:
	xorl	%eax, %eax
.LBB911_759:
	cmpb	$-1, %bl
	movq	512(%rsp), %rbx
	sete	%r14b
	andb	%al, %r14b
.LBB911_760:
	movzbl	%r14b, %eax
	movq	%rbx, 512(%rsp)
	movq	%rax, 24(%rsp)
.LBB911_761:
	movq	16(%rsp), %rax
	movq	1360(%rsp), %rcx
	movq	1376(%rsp), %rdx
	movq	1384(%rsp), %rsi
	movl	$1, %r12d
	movl	$1, %edi
	movq	%r15, 64(%rsp)
	movq	%rbp, 32(%rsp)
	movq	%rax, 96(%rsp)
	movq	1368(%rsp), %rax
	cmpq	$3, %rcx
	movq	%r15, 104(%rsp)
	movq	%rcx, %r15
	movq	%rbp, 112(%rsp)
	cmovaeq	%rdx, %r15
	cmovaeq	%rcx, %rdi
	cmovaeq	%r12, %rdx
	movq	%rdi, 864(%rsp)
	movq	%rax, 872(%rsp)
	movq	%rdx, 880(%rsp)
	movq	%rsi, 888(%rsp)
	movq	%r15, %rsi
	decq	%rsi
	movq	$0, 896(%rsp)
	movq	%rsi, 904(%rsp)
	je	.LBB911_765
	cmpq	$3, %rcx
	movq	40(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %r13
	leaq	872(%rsp), %rbp
	leaq	528(%rsp), %r14
	cmovaeq	%rax, %rbp
	leaq	640(%rcx), %rbx
	.p2align	4
.LBB911_763:
	movq	%r12, 896(%rsp)
	movq	16(%rbp), %rax
	movq	%rax, 544(%rsp)
	vmovdqu	(%rbp), %xmm0
	vmovdqa	%xmm0, 528(%rsp)
.Ltmp10785:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	*%r13
.Ltmp10786:
	addq	$24, %rbp
	incq	%r12
	cmpq	%r12, %r15
	jne	.LBB911_763
.LBB911_765:
.Ltmp10791:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp10792:
	testb	$1, 24(%rsp)
	je	.LBB911_771
	movq	480(%rsp), %rbx
	movq	%rbx, %rcx
	subq	512(%rsp), %rcx
	jb	.LBB911_873
	movq	472(%rsp), %r15
.Ltmp10793:
	movq	40(%rsp), %rsi
	leaq	864(%rsp), %rdi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10794:
	movq	512(%rsp), %rcx
	movq	32(%rsp), %rbp
	cmpq	%rcx, %rbx
	jne	.LBB911_775
.LBB911_770:
.Ltmp10818:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10819:
.LBB911_771:
	movq	112(%rsp), %rax
	vmovdqu	96(%rsp), %xmm0
	movq	%rax, 1264(%rsp)
	movq	456(%rsp), %rax
	vmovdqa	%xmm0, 1248(%rsp)
	testq	%rax, %rax
	je	.LBB911_774
	lock		decq	(%rax)
	jne	.LBB911_774
	#MEMBARRIER
.Ltmp10820:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	456(%rsp), %rdi
	callq	*%rax
.Ltmp10821:
.LBB911_774:
	movb	$1, %bpl
	jmp	.LBB911_199
.LBB911_775:
	leaq	(,%rbp,8), %rax
	shlq	$3, %rcx
	shlq	$3, %rbx
	leaq	(%rax,%rax,4), %r14
	leaq	(%rcx,%rcx,4), %rax
	leaq	8(%r15,%rax), %r12
	leaq	(%rbx,%rbx,4), %r15
	subq	%rax, %r15
	jmp	.LBB911_777
.LBB911_776:
	movq	64(%rsp), %rcx
	incq	%rbp
	addq	$40, %r12
	movq	%r15, (%rcx,%r14)
	movq	%rbx, 8(%rcx,%r14)
	vmovdqa	528(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rcx,%r14)
	movq	544(%rsp), %rax
	movq	%rax, 32(%rcx,%r14)
	addq	$40, %r14
	addq	$-40, %rdx
	movq	%rbp, 112(%rsp)
	movq	%rdx, %r15
	je	.LBB911_770
.LBB911_777:
.Ltmp10795:
	movq	40(%rsp), %rdx
	leaq	528(%rsp), %rdi
	leaq	864(%rsp), %rsi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10796:
	cmpb	$-1, 528(%rsp)
	jne	.LBB911_770
	movq	1152(%rsp), %rdx
	movq	$1, 528(%rsp)
	cmpq	$5, %rdx
	jae	.LBB911_801
.LBB911_780:
	vmovdqu	536(%rsp), %xmm0
	movq	560(%rsp), %rax
	movq	528(%rsp), %rdx
	movq	552(%rsp), %rcx
	movq	%r12, %r13
	movq	%rax, 208(%rsp)
	movq	%rdx, 176(%rsp)
	movq	%rcx, 200(%rsp)
	vmovdqu	%xmm0, 184(%rsp)
	movq	-8(%r12), %rbx
	decq	%rbx
	cmpq	$5, %rbx
	jb	.LBB911_782
	movq	8(%r12), %rbx
	movq	(%r12), %r13
	decq	%rbx
.LBB911_782:
	movq	176(%rsp), %rax
	movq	192(%rsp), %rsi
	movl	$4, %edx
	movq	%r15, 16(%rsp)
	leaq	-1(%rax), %rcx
	decq	%rsi
	cmpq	$5, %rcx
	cmovbq	%rcx, %rsi
	cmovbq	%rdx, %rcx
	subq	%rsi, %rcx
	cmpq	%rbx, %rcx
	jb	.LBB911_802
.LBB911_783:
	movq	%rbp, 32(%rsp)
	xorl	%r15d, %r15d
	leaq	184(%rsp), %rcx
	cmpq	$6, %rax
	setae	%al
	jb	.LBB911_785
	movq	184(%rsp), %rcx
.LBB911_785:
	movb	%al, %r15b
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	(,%rbx,8), %rdx
	movq	%r13, %rsi
	shll	$4, %r15d
	movq	176(%rsp,%r15), %rbp
	leaq	-8(%rcx,%rbp,8), %rdi
	callq	*%rax
	addq	%rbx, %rbp
	movq	1152(%rsp), %rax
	movq	%rbp, 176(%rsp,%r15)
	movq	176(%rsp), %rcx
	movq	192(%rsp), %rdx
	leaq	-1(%rcx), %rsi
	leaq	-1(%rdx), %rdi
	cmpq	$5, %rsi
	cmovbq	%rsi, %rdi
	movq	%rax, %rsi
	subq	%rdi, %rsi
	jbe	.LBB911_787
	movl	$2, 528(%rsp)
	movq	%rsi, 536(%rsp)
.Ltmp10803:
	leaq	176(%rsp), %rdi
	leaq	528(%rsp), %rsi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp10804:
	movq	32(%rsp), %rbp
	jmp	.LBB911_789
.LBB911_787:
	movq	32(%rsp), %rbp
	cmpq	$6, %rcx
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB911_789
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, 176(%rsp,%rdx)
.LBB911_789:
	movq	176(%rsp), %rcx
	leaq	184(%rsp), %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB911_791
	movq	192(%rsp), %rcx
	movq	184(%rsp), %rdx
	decq	%rcx
.LBB911_791:
	movq	304(%rsp), %r8
	addq	$16, %r8
.Ltmp10805:
	movq	40(%rsp), %r9
	leaq	528(%rsp), %rdi
	leaq	1856(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10806:
	vmovq	536(%rsp), %xmm0
	movq	528(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB911_804
	movq	176(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB911_795
	movq	192(%rsp), %rsi
.LBB911_795:
	movq	1232(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB911_881
	leaq	184(%rsp), %rcx
	cmpq	$6, %rax
	jb	.LBB911_798
	movq	184(%rsp), %rcx
.LBB911_798:
	vmovq	%xmm0, (%rcx,%rdi,8)
	leaq	192(%rsp), %rax
	movq	16(%rsp), %rdx
	vmovdqu	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	176(%rsp), %r15
	movq	184(%rsp), %rbx
	movq	%rax, 544(%rsp)
	vmovdqa	%xmm0, 528(%rsp)
	cmpq	96(%rsp), %rbp
	jne	.LBB911_776
.Ltmp10813:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	callq	*%rax
.Ltmp10814:
	movq	104(%rsp), %rax
	movq	16(%rsp), %rdx
	movq	%rax, 64(%rsp)
	jmp	.LBB911_776
.LBB911_801:
.Ltmp10798:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	528(%rsp), %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp10799:
	jmp	.LBB911_780
.LBB911_802:
.Ltmp10801:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movl	$1, %ecx
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp10802:
	movq	176(%rsp), %rax
	jmp	.LBB911_783
.LBB911_804:
	vmovups	544(%rsp), %zmm1
	vmovups	560(%rsp), %zmm2
	movq	320(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	176(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB911_806
	movq	184(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB911_806:
.Ltmp10808:
	leaq	864(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10809:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	456(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB911_468
	jmp	.LBB911_470
.LBB911_810:
	vmovups	912(%rsp), %ymm0
	movq	872(%rsp), %rax
	movzbl	903(%rsp), %ecx
	movq	888(%rsp), %rdx
	movq	880(%rsp), %r15
	movzwl	901(%rsp), %ebx
	addq	$40, %rbp
	movb	$1, %r14b
	movq	%rbp, 104(%rsp)
	movq	%rax, 16(%rsp)
	movzbl	896(%rsp), %eax
	movl	%ecx, 64(%rsp)
	movq	904(%rsp), %rcx
	movq	%rdx, 32(%rsp)
	vmovups	%ymm0, 784(%rsp)
	vmovdqu	928(%rsp), %ymm0
	movq	%rax, 24(%rsp)
	movl	897(%rsp), %eax
	movq	%rcx, 296(%rsp)
	movq	%rax, 56(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
.Ltmp10720:
	leaq	96(%rsp), %rdi
	movb	$1, %bpl
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10721:
	movq	288(%rsp), %r13
	movl	64(%rsp), %eax
	movb	$1, %r14b
	shll	$16, %eax
	orl	%eax, %ebx
	movb	$1, %al
	shlq	$32, %rbx
	movl	%eax, 88(%rsp)
	addq	%rbx, 56(%rsp)
	jmp	.LBB911_845
.LBB911_812:
	movq	160(%rsp), %rbp
	movq	%r14, %rax
	addq	$-1, %rax
	jae	.LBB911_828
	cmpq	%rax, %r13
	jae	.LBB911_828
	movq	80(%rsp), %rax
	movq	312(%rsp), %r15
	cmpq	168(%rsp), %rax
	je	.LBB911_827
	subq	%r13, %r14
	addq	$88, %rax
	leaq	176(%rsp), %rbx
	addq	$-2, %r14
.LBB911_816:
	movq	%rax, %r15
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB911_827
	movq	-88(%r15), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	-8(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp10675:
	movq	16(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp10676:
	subq	$1, %r14
	jb	.LBB911_827
	leaq	88(%r15), %rax
	cmpq	168(%rsp), %r15
	jne	.LBB911_816
	jmp	.LBB911_827
.LBB911_820:
	leaq	-1(%r14), %rax
	cmpq	%rax, %r13
	jae	.LBB911_828
	movq	80(%rsp), %rax
	movq	312(%rsp), %r15
	cmpq	168(%rsp), %rax
	je	.LBB911_827
	subq	%r13, %r14
	addq	$88, %rax
	leaq	176(%rsp), %rbx
	addq	$-2, %r14
.LBB911_823:
	movq	%rax, %r15
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB911_827
	movq	-88(%r15), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	-8(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp10669:
	movq	16(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp10670:
	subq	$1, %r14
	jb	.LBB911_827
	leaq	88(%r15), %rax
	cmpq	168(%rsp), %r15
	jne	.LBB911_823
.LBB911_827:
	movq	%r15, 360(%rsp)
.LBB911_828:
	movq	328(%rsp), %rax
	movq	344(%rsp), %rcx
	movq	336(%rsp), %r15
	movq	288(%rsp), %r13
	movq	$-1, %r12
	movl	$0, 48(%rsp)
	movq	%rax, 16(%rsp)
	movb	$1, %al
	movq	%rcx, 32(%rsp)
	movq	%rax, 24(%rsp)
.LBB911_829:
.LBB911_830:
	movq	272(%rsp), %r14
	movq	520(%rsp), %rsi
	movq	1136(%rsp), %rdi
	testq	%rsi, %rsi
	je	.LBB911_832
.LBB911_831:
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB911_832:
.Ltmp10684:
	leaq	352(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp10685:
	testq	%r14, %r14
	je	.LBB911_835
	movq	64(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	leaq	(%r14,%r14,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB911_835:
	movq	952(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_838
	lock		decq	(%rax)
	jne	.LBB911_838
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	952(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB911_838:
	movq	984(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_841
	lock		decq	(%rax)
	jne	.LBB911_841
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	984(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB911_841:
	xorl	%r14d, %r14d
.Ltmp10687:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp10688:
	lock		decq	(%rbp)
	movl	48(%rsp), %r14d
	jne	.LBB911_844
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp10689:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1240(%rsp), %rdi
	callq	*%rax
.Ltmp10690:
.LBB911_844:
	movl	$0, 88(%rsp)
.LBB911_845:
	cmpq	$0, 1304(%rsp)
	movq	32(%rsp), %rbp
	je	.LBB911_847
.Ltmp10722:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10723:
.LBB911_847:
	movl	88(%rsp), %edx
	testb	%r14b, %r14b
	je	.LBB911_863
	movq	336(%rsp), %rbx
	movq	344(%rsp), %r13
	movq	%r15, 64(%rsp)
	testq	%r13, %r13
	je	.LBB911_861
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	movq	free@GOTPCREL(%rip), %r14
	xorl	%ebp, %ebp
	jmp	.LBB911_853
.LBB911_850:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB911_851:
	callq	*%r14
	movl	88(%rsp), %edx
.LBB911_852:
	incq	%rbp
	cmpq	%r13, %rbp
	je	.LBB911_861
.LBB911_853:
	leaq	(%rbp,%rbp,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB911_852
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB911_856
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB911_856:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB911_851
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB911_856
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
	movq	(%r15), %rax
.LBB911_859:
	cmpq	%rax, %rdx
	jge	.LBB911_850
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB911_859
	jmp	.LBB911_850
.LBB911_861:
	movq	328(%rsp), %rax
	movq	288(%rsp), %r13
	movq	32(%rsp), %rbp
	movq	64(%rsp), %r15
	testq	%rax, %rax
	je	.LBB911_863
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	movl	88(%rsp), %edx
.LBB911_863:
	testb	%dl, %dl
	jne	.LBB911_457
	jmp	.LBB911_463
.LBB911_864:
	movq	328(%rsp), %rax
	movq	344(%rsp), %rcx
	movq	336(%rsp), %r15
	movq	$-1, %r12
	movl	$0, 48(%rsp)
	movq	%rax, 16(%rsp)
	movb	$1, %al
	movq	%rcx, 32(%rsp)
	movq	%rax, 24(%rsp)
	movq	520(%rsp), %rsi
	movq	1136(%rsp), %rdi
	testq	%rsi, %rsi
	jne	.LBB911_831
	jmp	.LBB911_832
.LBB911_865:
	movq	328(%rsp), %rax
	movq	344(%rsp), %rcx
	movq	336(%rsp), %r15
	movq	$-1, %r12
	movq	$0, 24(%rsp)
	movl	$0, 48(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rcx, 32(%rsp)
	jmp	.LBB911_829
.LBB911_866:
.Ltmp10747:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.142(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.144(%rip), %rdx
	movl	$83, %esi
	movb	%r13b, 56(%rsp)
	vzeroupper
	callq	*%rax
.Ltmp10748:
	jmp	.LBB911_882
.LBB911_867:
.Ltmp10517:
	leaq	416(%rsp), %rdi
	movl	$16, %ecx
	movl	$256, %r8d
	xorl	%esi, %esi
	movq	%r14, %rdx
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp10518:
	movq	416(%rsp), %rax
	movq	432(%rsp), %r13
	subq	%r13, %rax
	cmpq	%r14, %rax
	jae	.LBB911_291
.LBB911_869:
.Ltmp10531:
	movq	core::panicking::panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.965(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.967(%rip), %rdx
	movl	$47, %esi
	callq	*%rax
.Ltmp10532:
	jmp	.LBB911_882
.LBB911_870:
	leaq	1360(%rsp), %rax
	movq	%rax, 528(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 536(%rsp)
	movq	%r15, 544(%rsp)
	movq	%rax, 552(%rsp)
.Ltmp10523:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.550(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.552(%rip), %rdx
	leaq	528(%rsp), %rsi
	callq	*%rax
.Ltmp10524:
	jmp	.LBB911_882
.LBB911_871:
.Ltmp10694:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.301(%rip), %rcx
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp10695:
	jmp	.LBB911_882
.LBB911_872:
	leaq	1664(%rsp), %rax
	leaq	96(%rsp), %rcx
	movq	%r12, 1664(%rsp)
	movq	%rdx, 96(%rsp)
	movq	%rax, 176(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 184(%rsp)
	movq	%rcx, 192(%rsp)
	movq	%rax, 200(%rsp)
.Ltmp10696:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.2054(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.300(%rip), %rdx
	leaq	176(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp10697:
	jmp	.LBB911_882
.LBB911_873:
.Ltmp10836:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	movq	512(%rsp), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.390(%rip), %rcx
	movq	%rbx, %rsi
	movq	%rbx, %rdx
	callq	*%rax
.Ltmp10837:
	jmp	.LBB911_882
.LBB911_874:
	movb	$1, %al
	movl	%eax, 48(%rsp)
.Ltmp10633:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %r14b
	movq	%r15, %rdi
	movq	%rbx, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp10634:
	jmp	.LBB911_557
.LBB911_875:
	movq	40(%rsp), %rax
	leaq	1000(%rax), %rdi
	movb	$1, %al
	movl	%eax, 48(%rsp)
.Ltmp10635:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movb	$1, %r14b
	movq	%rbx, %rsi
	movq	%r15, %rdx
	vzeroupper
	callq	*%rax
.Ltmp10636:
	jmp	.LBB911_558
.LBB911_876:
	movq	16(%rsp), %rax
	movq	%rax, 1400(%rsp)
	movq	%r15, 1424(%rsp)
.Ltmp10507:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.388(%rip), %rdx
	callq	*%rax
.Ltmp10508:
	jmp	.LBB911_882
.LBB911_878:
.Ltmp10860:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	880(%rsp), %rbx
	callq	*%rax
.Ltmp10861:
	jmp	.LBB911_882
.LBB911_879:
.Ltmp10896:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	leaq	544(%rsp), %rbx
	callq	*%rax
.Ltmp10897:
	jmp	.LBB911_882
.LBB911_880:
.Ltmp10498:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp10499:
	jmp	.LBB911_882
.LBB911_881:
.Ltmp10810:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.389(%rip), %rdx
	callq	*%rax
.Ltmp10811:
.LBB911_882:
	ud2
.LBB911_883:
.Ltmp10746:
	leaq	176(%rsp), %rdi
	movq	%rax, %r12
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB911_1049
.LBB911_884:
.Ltmp10800:
	movq	%rax, %r12
	movq	528(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB911_971
	movq	536(%rsp), %rdi
	jmp	.LBB911_969
.LBB911_886:
.Ltmp10671:
	jmp	.LBB911_888
.LBB911_887:
.Ltmp10677:
.LBB911_888:
	movq	%rax, %r12
	movq	%r15, 360(%rsp)
	jmp	.LBB911_1023
.LBB911_889:
.Ltmp10648:
	jmp	.LBB911_1022
.LBB911_890:
.Ltmp10703:
	movq	%rax, %r12
	movb	$1, %al
	movl	%eax, 48(%rsp)
	jmp	.LBB911_1026
.LBB911_891:
.Ltmp10655:
	jmp	.LBB911_1022
.LBB911_892:
.Ltmp10686:
	movq	%rax, %r12
	jmp	.LBB911_1026
.LBB911_893:
.Ltmp10639:
	jmp	.LBB911_1022
.LBB911_894:
.Ltmp10645:
	addq	$88, %r15
	movq	%rax, %r12
	movq	%r15, 360(%rsp)
	jmp	.LBB911_1023
.LBB911_895:
.Ltmp10724:
	movq	%rax, %r12
	movl	%r14d, 48(%rsp)
	jmp	.LBB911_1039
.LBB911_896:
.Ltmp10732:
	cmpq	$0, 1304(%rsp)
	movq	%rax, %r12
	movl	%ebp, 88(%rsp)
	movl	%r14d, 48(%rsp)
	jne	.LBB911_1038
	jmp	.LBB911_1039
.LBB911_897:
.Ltmp10710:
	movq	%rax, %r12
	movl	%r14d, 88(%rsp)
	jmp	.LBB911_1036
.LBB911_898:
.Ltmp10674:
	jmp	.LBB911_1022
.LBB911_899:
.Ltmp10600:
	movq	%rax, %r12
	jmp	.LBB911_939
.LBB911_900:
.Ltmp10771:
	movq	%rax, %r12
	jmp	.LBB911_950
.LBB911_901:
.Ltmp10658:
	jmp	.LBB911_917
.LBB911_902:
.Ltmp10632:
	movq	%rax, %r12
	jmp	.LBB911_1042
.LBB911_903:
.Ltmp10586:
	movq	%rax, %r12
	jmp	.LBB911_940
.LBB911_904:
.Ltmp10545:
	movq	%rax, %r12
.Ltmp10546:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp10547:
	jmp	.LBB911_1053
.LBB911_905:
.Ltmp10548:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_906:
.Ltmp10757:
	movq	%rax, %r12
	movb	%r13b, 56(%rsp)
	jmp	.LBB911_1046
.LBB911_907:
.Ltmp10719:
	addq	$40, %rbp
	movq	%rax, %r12
	movq	%rbp, 104(%rsp)
	jmp	.LBB911_914
.LBB911_908:
.Ltmp10627:
	movq	%rax, %r12
.Ltmp10628:
	leaq	1160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10629:
	jmp	.LBB911_1042
.LBB911_909:
.Ltmp10618:
	movq	%rax, %r12
	jmp	.LBB911_954
.LBB911_910:
.Ltmp10581:
	movq	%rax, %r12
.Ltmp10582:
	leaq	1160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10583:
	jmp	.LBB911_940
.LBB911_911:
.Ltmp10607:
	movq	%rax, %r12
	jmp	.LBB911_955
.LBB911_912:
.Ltmp10727:
	addq	$40, %rbp
	cmpq	$6, 64(%rsp)
	movq	%rax, %r12
	movq	%rbp, 104(%rsp)
	jb	.LBB911_914
	movq	64(%rsp), %rax
	movq	24(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
.LBB911_914:
	movb	$1, %al
	movl	%eax, 88(%rsp)
.Ltmp10728:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10729:
	jmp	.LBB911_1040
.LBB911_915:
.Ltmp10668:
	jmp	.LBB911_1022
.LBB911_916:
.Ltmp10680:
.LBB911_917:
	addq	$88, %rbx
	movq	%rax, %r12
	movq	%rbx, 360(%rsp)
	jmp	.LBB911_1023
.LBB911_918:
.Ltmp10752:
	movq	%rax, %r12
	movb	%r13b, 56(%rsp)
.Ltmp10753:
	leaq	1160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10754:
	jmp	.LBB911_1046
.LBB911_919:
.Ltmp10824:
	leaq	1248(%rsp), %rdi
	movq	%rax, %r12
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movl	16(%rsp), %ebp
	jmp	.LBB911_1059
.LBB911_920:
.Ltmp10827:
	movq	%rax, %r12
	xorl	%ebx, %ebx
	jmp	.LBB911_1060
.LBB911_921:
.Ltmp10567:
	jmp	.LBB911_1052
.LBB911_922:
.Ltmp10815:
	movq	%rax, %r12
	cmpq	$6, %r15
	jb	.LBB911_971
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	jmp	.LBB911_970
.LBB911_924:
.Ltmp10683:
	addq	$40, %rbx
	movq	%rax, %r12
	movq	%rbx, 424(%rsp)
	jmp	.LBB911_1023
.LBB911_925:
.Ltmp10784:
	jmp	.LBB911_973
.LBB911_926:
.Ltmp10589:
	movq	%rax, %r12
	movq	%r14, 104(%rsp)
	jmp	.LBB911_938
.LBB911_927:
.Ltmp10779:
	movq	%rax, %r12
	jmp	.LBB911_1049
.LBB911_928:
.Ltmp10835:
	cmpb	$0, 16(%rsp)
	movq	%rax, %r12
	je	.LBB911_1072
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%r12, %rdi
	callq	_Unwind_Resume@PLT
.LBB911_930:
.Ltmp10693:
	addq	$40, %rbx
	cmpq	$6, 24(%rsp)
	movq	%rax, %r12
	movq	%rbx, 424(%rsp)
	jb	.LBB911_1023
	movq	24(%rsp), %rax
	movq	56(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB911_1023
.LBB911_932:
.Ltmp10760:
	movq	%rax, %r12
	movq	%r14, 792(%rsp)
	jmp	.LBB911_949
.LBB911_933:
.Ltmp10882:
	lock		decq	(%r14)
	movq	%rax, %r12
	jne	.LBB911_1071
	#MEMBARRIER
.Ltmp10883:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	864(%rsp), %rdi
	callq	*%rax
.Ltmp10884:
	jmp	.LBB911_1071
.LBB911_935:
.Ltmp10885:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_936:
.Ltmp10594:
	movq	%rax, %r12
	movq	%r14, 104(%rsp)
	cmpq	$6, %rbx
	jb	.LBB911_938
	movq	32(%rsp), %rdi
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB911_938:
.Ltmp10595:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10596:
.LBB911_939:
	leaq	784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB911_940:
.Ltmp10601:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp10602:
	jmp	.LBB911_1049
.LBB911_941:
.Ltmp10597:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_942:
.Ltmp10859:
	cmpb	$0, 16(%rsp)
	movq	%rax, %r12
	je	.LBB911_1071
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB911_1071
.LBB911_944:
.Ltmp10853:
	movq	%rax, %r12
	movb	$1, %r13b
	jmp	.LBB911_965
.LBB911_945:
.Ltmp10576:
	movq	%rax, %r12
.Ltmp10577:
	leaq	1160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10578:
	jmp	.LBB911_1055
.LBB911_946:
.Ltmp10812:
	jmp	.LBB911_967
.LBB911_947:
.Ltmp10765:
	movq	%rax, %r12
	movq	%r14, 792(%rsp)
	cmpq	$6, %rbx
	jb	.LBB911_949
	movq	32(%rsp), %rdi
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB911_949:
.Ltmp10766:
	leaq	784(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10767:
.LBB911_950:
	leaq	1184(%rsp), %rdi
	jmp	.LBB911_1045
.LBB911_951:
.Ltmp10768:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_952:
.Ltmp10610:
	movq	%rax, %r12
	movq	%r14, 184(%rsp)
.Ltmp10611:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10612:
.Ltmp10614:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp10615:
.LBB911_954:
.Ltmp10619:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp10620:
.LBB911_955:
.Ltmp10622:
	leaq	1280(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10623:
	jmp	.LBB911_1049
.LBB911_956:
.Ltmp10613:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_957:
.Ltmp10621:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_958:
.Ltmp10797:
	movq	%rax, %r12
	jmp	.LBB911_971
.LBB911_959:
.Ltmp10663:
	addq	$88, %rbp
	movq	%rax, %r12
	movq	%rbp, 360(%rsp)
	jmp	.LBB911_1023
.LBB911_960:
.Ltmp10624:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_961:
.Ltmp10872:
	lock		decq	(%r14)
	movq	%rax, %r12
	jne	.LBB911_1071
	#MEMBARRIER
.Ltmp10873:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	864(%rsp), %rdi
	callq	*%rax
.Ltmp10874:
	jmp	.LBB911_1071
.LBB911_963:
.Ltmp10875:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_964:
.Ltmp10830:
	movq	%rax, %r12
	xorl	%r13d, %r13d
.LBB911_965:
	movl	16(%rsp), %ebp
	jmp	.LBB911_1065
.LBB911_966:
.Ltmp10807:
.LBB911_967:
	movq	%rax, %r12
	movq	176(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB911_971
	movq	184(%rsp), %rdi
.LBB911_969:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB911_970:
	callq	__rustc::__rust_dealloc
.LBB911_971:
.Ltmp10816:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10817:
	jmp	.LBB911_1015
.LBB911_972:
.Ltmp10573:
.LBB911_973:
	movq	%rax, %r12
	jmp	.LBB911_1055
.LBB911_974:
.Ltmp10898:
	movq	%rax, %r12
.Ltmp10899:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp10900:
.Ltmp10902:
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp10903:
	jmp	.LBB911_1072
.LBB911_976:
.Ltmp10901:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_977:
.Ltmp10891:
	movq	%rax, %r12
.Ltmp10892:
	leaq	1856(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp10893:
	jmp	.LBB911_1071
.LBB911_978:
.Ltmp10551:
	movq	%rax, %r12
	testq	%r13, %r13
	je	.LBB911_1011
	negq	%r13
	jmp	.LBB911_981
.LBB911_980:
	addq	$272, %r15
	decq	%r13
	je	.LBB911_1011
.LBB911_981:
	cmpl	$2, (%r15)
	je	.LBB911_980
.Ltmp10552:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
.Ltmp10553:
	jmp	.LBB911_980
.LBB911_983:
.Ltmp10554:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_984:
.Ltmp10642:
	jmp	.LBB911_1022
.LBB911_985:
.Ltmp10843:
	movq	%rax, %r12
	movb	$1, %bpl
	jmp	.LBB911_1059
.LBB911_986:
.Ltmp10862:
	movq	%rax, %r12
.Ltmp10863:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp10864:
	jmp	.LBB911_990
.LBB911_987:
.Ltmp10865:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_988:
.Ltmp10491:
	movq	%rax, %r12
.Ltmp10492:
	leaq	1800(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp10493:
	jmp	.LBB911_990
.LBB911_989:
.Ltmp10488:
	movq	%rax, %r12
.LBB911_990:
	movb	$1, %bpl
	movb	$1, %r13b
	jmp	.LBB911_1065
.LBB911_991:
.Ltmp10512:
	movq	16(%rsp), %rcx
	movq	%rax, %r12
	movq	%rcx, 1400(%rsp)
	movq	%r15, 1424(%rsp)
	cmpq	$5, %r13
	ja	.LBB911_1008
	jmp	.LBB911_1009
.LBB911_992:
.Ltmp10740:
	movq	%rax, %r12
	testq	%rbx, %rbx
	je	.LBB911_996
	negq	%rbx
	addq	$160, %r14
.LBB911_994:
.Ltmp10741:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10742:
	addq	$160, %r14
	decq	%rbx
	jne	.LBB911_994
.LBB911_996:
	cmpq	$0, 1128(%rsp)
	je	.LBB911_1049
	movq	1128(%rsp), %rax
	movq	448(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB911_1049
.LBB911_998:
.Ltmp10743:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_999:
.Ltmp10787:
	movq	%rax, %r12
.Ltmp10788:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp10789:
	jmp	.LBB911_1015
.LBB911_1000:
.Ltmp10790:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_1001:
.Ltmp10509:
	movq	%rax, %r12
	jmp	.LBB911_1006
.LBB911_1002:
.Ltmp10848:
	movl	16(%rsp), %ebp
	movq	%rax, %r12
	jmp	.LBB911_1062
.LBB911_1003:
.Ltmp10856:
	movl	16(%rsp), %ebp
	movq	%rax, %r12
	jmp	.LBB911_1066
.LBB911_1004:
.Ltmp10888:
	movq	%rax, %r12
	jmp	.LBB911_1071
.LBB911_1005:
.Ltmp10506:
	movq	16(%rsp), %rcx
	movq	%rax, %r12
	movq	%rcx, 1400(%rsp)
	movq	%r15, 1424(%rsp)
.LBB911_1006:
	movq	528(%rsp), %r13
	cmpq	$6, %r13
	jb	.LBB911_1009
	movq	536(%rsp), %rbx
.LBB911_1008:
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB911_1009:
	leaq	1392(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB911_1060
.LBB911_1010:
.Ltmp10557:
	movq	%rax, %r12
.LBB911_1011:
.Ltmp10558:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp10559:
.Ltmp10560:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#2}>>
.Ltmp10561:
	jmp	.LBB911_1053
.LBB911_1013:
.Ltmp10562:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_1014:
.Ltmp10838:
	movq	%rax, %r12
.LBB911_1015:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB911_1055
.LBB911_1016:
.Ltmp10525:
	movq	832(%rsp), %rdi
	movq	%rax, %r12
.Ltmp10526:
	movq	%rbx, %rsi
	callq	core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp10527:
	jmp	.LBB911_1019
.LBB911_1017:
.Ltmp10528:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_1018:
.Ltmp10533:
	movq	%rax, %r12
.LBB911_1019:
.Ltmp10534:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp10535:
	jmp	.LBB911_1055
.LBB911_1020:
.Ltmp10536:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_1021:
.Ltmp10698:
.LBB911_1022:
	movq	%rax, %r12
.LBB911_1023:
	cmpq	$0, 520(%rsp)
	je	.LBB911_1025
	movq	520(%rsp), %rsi
	movq	1136(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB911_1025:
	movb	$1, %al
	movl	%eax, 48(%rsp)
.Ltmp10699:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp10700:
.LBB911_1026:
	cmpq	$0, 272(%rsp)
	je	.LBB911_1028
	movq	272(%rsp), %rax
	movq	64(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB911_1028:
	movq	952(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_1031
	lock		decq	(%rax)
	jne	.LBB911_1031
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	952(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB911_1031:
	movq	984(%rsp), %rax
	testq	%rax, %rax
	je	.LBB911_1034
	lock		decq	(%rax)
	jne	.LBB911_1034
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	984(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB911_1034:
.Ltmp10704:
	leaq	832(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp10705:
	movl	$0, 88(%rsp)
.LBB911_1036:
	movq	160(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB911_1038
	#MEMBARRIER
.Ltmp10711:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1240(%rsp), %rdi
	callq	*%rax
.Ltmp10712:
.LBB911_1038:
.Ltmp10733:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10734:
.LBB911_1039:
	cmpb	$0, 48(%rsp)
	je	.LBB911_1041
.LBB911_1040:
	leaq	328(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB911_1041:
	cmpb	$0, 88(%rsp)
	je	.LBB911_1049
.LBB911_1042:
.Ltmp10735:
	leaq	1328(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp10736:
	jmp	.LBB911_1049
.LBB911_1043:
.Ltmp10737:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_1044:
.Ltmp10749:
	leaq	832(%rsp), %rdi
	movq	%rax, %r12
.LBB911_1045:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB911_1046:
.Ltmp10772:
	leaq	864(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp10773:
	cmpb	$0, 56(%rsp)
	je	.LBB911_1049
.Ltmp10774:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp10775:
.LBB911_1049:
.Ltmp10780:
	leaq	1360(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp10781:
	jmp	.LBB911_1055
.LBB911_1050:
.Ltmp10776:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB911_1051:
.Ltmp10570:
.LBB911_1052:
	movq	%rax, %r12
.LBB911_1053:
	movq	832(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB911_1055
	movq	840(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB911_1055:
	movq	456(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	je	.LBB911_1059
	lock		decq	(%rax)
	movb	$1, %bpl
	jne	.LBB911_1059
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp10839:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	456(%rsp), %rdi
	callq	*%rax
.Ltmp10840:
	movb	$1, %bl
	jmp	.LBB911_1060
.LBB911_1059:
	movb	$1, %bl
.LBB911_1060:
.Ltmp10844:
	leaq	1856(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp10845:
	testb	%bl, %bl
	je	.LBB911_1064
.LBB911_1062:
	movq	304(%rsp), %rax
	movb	$1, %r13b
	lock		decq	(%rax)
	jne	.LBB911_1065
	#MEMBARRIER
.Ltmp10849:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	callq	*%rax
.Ltmp10850:
	jmp	.LBB911_1065
.LBB911_1064:
	xorl	%r13d, %r13d
.LBB911_1065:
.Ltmp10866:
	leaq	2072(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10867:
.LBB911_1066:
	movq	488(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB911_1068
	leaq	488(%rsp), %rdi
	#MEMBARRIER
.Ltmp10868:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp10869:
.LBB911_1068:
	testb	%bpl, %bpl
	je	.LBB911_1070
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB911_1070:
	testb	%r13b, %r13b
	je	.LBB911_1072
.LBB911_1071:
.Ltmp10894:
	leaq	1696(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp10895:
.LBB911_1072:
	movq	%r12, %rdi
	callq	_Unwind_Resume@PLT
.LBB911_1073:
.Ltmp10904:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end911:
purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin912:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception586
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
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %r15
	leaq	1736(%rsp), %rdi
	movq	%r8, %r13
	movq	%rcx, %r14
	movq	%rdx, %rbx
	movq	%rsi, %r12
	callq	*%rax
.Ltmp10905:
	leaq	1840(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r13, 16(%rsp)
	movq	%r13, %rdx
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp10906:
	cmpl	$1, 1840(%rsp)
	jne	.LBB912_26
	vmovdqu64	1888(%rsp), %zmm1
	vmovdqu64	1856(%rsp), %zmm0
	vmovdqu64	%zmm1, 48(%r15)
	vmovdqu64	%zmm0, 16(%r15)
	movq	$1, (%r15)
.LBB912_3:
	movq	1808(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB912_13
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1816(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_6
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_6:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_12
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_6
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
.LBB912_9:
	cmpq	%rax, %rsi
	jge	.LBB912_11
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB912_9
.LBB912_11:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_12:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB912_13:
	movq	1736(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB912_23
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1744(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_16
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_16:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_22
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_16
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
.LBB912_19:
	cmpq	%rax, %rsi
	jge	.LBB912_21
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB912_19
.LBB912_21:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_22:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB912_23:
	movq	1832(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_694
	lock		decq	(%rax)
	jne	.LBB912_694
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1832(%rsp), %rdi
	#MEMBARRIER
.LBB912_693:
	vzeroupper
	callq	*%rax
	jmp	.LBB912_694
.LBB912_26:
	vmovdqu64	1880(%rsp), %zmm1
	vmovdqu64	1848(%rsp), %zmm0
	vmovdqu64	%zmm1, 752(%rsp)
	vmovdqu64	%zmm0, 720(%rsp)
.Ltmp10907:
	leaq	384(%rsp), %rdi
	leaq	1736(%rsp), %rsi
	leaq	720(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp10908:
	cmpq	$-1, 384(%rsp)
	je	.LBB912_34
	vmovdqu	384(%rsp), %ymm0
	movq	%r15, 88(%rsp)
	vmovdqu	%ymm0, 656(%rsp)
	movq	680(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB912_819
	movb	$1, %r15b
	movq	%rax, 144(%rsp)
.Ltmp10910:
	movq	16(%rsp), %r14
	movq	%rbx, %rsi
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
.Ltmp10911:
	cmpb	$2, 472(%r14)
	sete	%cl
	andb	%cl, %al
	cmpb	$1, %al
	jne	.LBB912_35
	movq	616(%r14), %rax
	testq	%rax, %rax
	je	.LBB912_36
	cmpq	$-2, 24(%rax)
	jb	.LBB912_35
	cmpq	$-2, 32(%rax)
	jae	.LBB912_39
.LBB912_35:
	xorl	%r13d, %r13d
	jmp	.LBB912_40
.LBB912_34:
	leaq	8(%r15), %rdi
	leaq	1736(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	movq	$0, (%r15)
	jmp	.LBB912_694
.LBB912_36:
	movb	$1, %r13b
	jmp	.LBB912_40
.LBB912_39:
	cmpq	$-2, 48(%rax)
	setae	%r13b
.LBB912_40:
	movq	672(%rsp), %r14
.Ltmp10912:
	movq	16(%rsp), %rbp
	movzbl	%r13b, %edx
	leaq	1960(%rsp), %rdi
	movq	%r14, %rcx
	movq	%rbp, %rsi
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10913:
	movb	$1, %r15b
.Ltmp10914:
	movb	$1, %al
	movq	%rbp, %rdi
	movq	%r12, 1272(%rsp)
	movq	%r12, %rsi
	movq	%rbx, %rdx
	movl	%eax, 316(%rsp)
	callq	purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10915:
	movq	144(%rsp), %rcx
	movb	$1, %dl
	movl	%edx, 316(%rsp)
	addq	$16, %rcx
.Ltmp10916:
	leaq	2160(%rsp), %r12
	movb	$1, %r15b
	movq	%rax, %rsi
	movq	%rbx, %rdx
	movq	%rbp, %r8
	movq	%r12, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10917:
	testb	%r13b, %r13b
	je	.LBB912_48
	movq	2144(%rsp), %rax
	movq	664(%rsp), %r13
	cmpq	%r14, %rax
	cmovbq	%rax, %r14
.Ltmp10928:
	movq	16(%rsp), %rdi
	movq	%r14, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp10929:
	movq	16(%rsp), %rcx
	movq	%rax, 304(%rsp)
	leaq	1960(%rsp), %rsi
	movq	616(%rcx), %rax
	testq	%rax, %rax
	je	.LBB912_74
	cmpq	$-2, 16(%rax)
	movb	$1, %cl
	jb	.LBB912_75
	cmpq	$-2, 40(%rax)
	setb	%cl
	jmp	.LBB912_75
.LBB912_48:
	movq	664(%rsp), %r12
	movq	656(%rsp), %rcx
	leaq	(%r14,%r14,4), %rax
	movabsq	$9223372036854775807, %rbx
	movq	$0, 320(%rsp)
	movq	$8, 328(%rsp)
	movq	$0, 336(%rsp)
	leaq	(%r12,%rax,8), %r13
	movq	%r12, 96(%rsp)
	movq	%rcx, 112(%rsp)
	movq	%rcx, 688(%rsp)
	movq	%r12, 256(%rsp)
	movq	%r13, 120(%rsp)
	testq	%r14, %r14
	je	.LBB912_88
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	256(%rsp), %r12
	movl	$8, %eax
	movq	$0, 24(%rsp)
	movq	%rax, 40(%rsp)
	jmp	.LBB912_53
.LBB912_50:
	movq	328(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB912_51:
	movq	24(%rsp), %rsi
	movq	40(%rsp), %rdx
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 24(%rsp)
	movq	%r15, (%rdx,%rax,8)
	movq	%r14, 8(%rdx,%rax,8)
	vmovdqa	720(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	736(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 336(%rsp)
.LBB912_52:
	cmpq	%r13, %r12
	je	.LBB912_87
.LBB912_53:
	vmovups	8(%r12), %ymm0
	movq	(%r12), %r15
	addq	$40, %r12
	vmovups	%ymm0, 1104(%rsp)
	testq	%r15, %r15
	je	.LBB912_88
	vmovdqu	1104(%rsp), %ymm0
	leaq	1528(%rsp), %rax
	movq	%r15, 1520(%rsp)
	vmovdqu	%ymm0, (%rax)
.Ltmp10918:
	movq	16(%rsp), %rdx
	leaq	720(%rsp), %rdi
	leaq	1960(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10919:
	cmpb	$-1, 720(%rsp)
	jne	.LBB912_84
	movq	1536(%rsp), %rcx
	movq	1528(%rsp), %r14
	movq	144(%rsp), %r8
	leaq	-1(%r15), %rax
	leaq	1528(%rsp), %rdx
	decq	%rcx
	cmpq	$5, %rax
	cmovaeq	%r14, %rdx
	cmovbq	%rax, %rcx
	addq	$16, %r8
.Ltmp10920:
	movq	16(%rsp), %r9
	leaq	720(%rsp), %rdi
	leaq	2160(%rsp), %rsi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10921:
	movq	720(%rsp), %rax
	movl	728(%rsp), %edx
	movl	732(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB912_86
	cmpl	$2, %edx
	je	.LBB912_62
.Ltmp10922:
	movq	16(%rsp), %rsi
	leaq	720(%rsp), %rdi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp10923:
	movq	720(%rsp), %rax
	movzbl	728(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB912_189
	testb	$1, %dl
	jne	.LBB912_72
.LBB912_62:
	cmpq	$6, %r15
	jb	.LBB912_52
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	leaq	-8(,%r15,8), %rcx
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_65
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_65:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_71
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_65
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
	movq	(%rbp), %rax
	.p2align	4
.LBB912_68:
	cmpq	%rax, %rdx
	jge	.LBB912_70
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_68
.LBB912_70:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_71:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
	jmp	.LBB912_52
.LBB912_72:
	leaq	1112(%rsp), %rcx
	vmovdqu	(%rcx), %xmm0
	movq	16(%rcx), %rax
	movq	24(%rsp), %rcx
	movq	%rax, 736(%rsp)
	vmovdqa	%xmm0, 720(%rsp)
	cmpq	320(%rsp), %rcx
	jne	.LBB912_51
.Ltmp10925:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	320(%rsp), %rdi
	callq	*%rax
.Ltmp10926:
	jmp	.LBB912_50
.LBB912_74:
	xorl	%ecx, %ecx
.LBB912_75:
	movq	16(%rsp), %rdx
	movq	%r13, 176(%rsp)
	movq	%r14, 184(%rsp)
	movzbl	1234(%rdx), %eax
	movq	%rdx, 96(%rsp)
	leaq	304(%rsp), %rdx
	movq	%rdx, 104(%rsp)
	leaq	144(%rsp), %rdx
	movq	%rsi, 112(%rsp)
	movq	%r12, 120(%rsp)
	movq	%rdx, 192(%rsp)
	xorb	$1, %al
	testb	%cl, %cl
	je	.LBB912_78
	cmpq	$1025, %r14
	movq	%rsi, 1184(%rsp)
	setae	%cl
	testb	%al, %cl
	jne	.LBB912_80
	movq	192(%rsp), %rax
	vmovdqu	96(%rsp), %ymm0
	vmovdqu	176(%rsp), %xmm1
	movq	%r13, 224(%rsp)
	movq	%r14, 232(%rsp)
	movq	%rsi, 1040(%rsp)
	movq	%rax, 336(%rsp)
	leaq	1104(%rsp), %rax
	movq	%rax, 384(%rsp)
	leaq	224(%rsp), %rax
	movq	%rax, 392(%rsp)
	leaq	320(%rsp), %rax
	movq	%rax, 400(%rsp)
	leaq	1040(%rsp), %rax
	vmovdqu	%ymm0, 1104(%rsp)
	vmovdqa	%xmm1, 320(%rsp)
	movq	%rax, 408(%rsp)
.Ltmp10986:
	leaq	720(%rsp), %rdi
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#0}
.Ltmp10987:
	movq	16(%rsp), %r15
	jmp	.LBB912_139
.LBB912_78:
	movq	%rsi, 352(%rsp)
	leaq	96(%rsp), %rcx
	cmpq	$1025, %r14
	leaq	1184(%rsp), %rsi
	leaq	176(%rsp), %rdi
	movq	%r13, 1184(%rsp)
	movq	%r14, 1192(%rsp)
	movq	%rcx, 1104(%rsp)
	movq	%rsi, 1112(%rsp)
	leaq	352(%rsp), %rsi
	setae	%dl
	movq	%rdi, 1120(%rsp)
	movq	%rsi, 1128(%rsp)
	testb	%al, %dl
	jne	.LBB912_82
.Ltmp10952:
	leaq	720(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#0}
.Ltmp10953:
	movq	16(%rsp), %r15
	jmp	.LBB912_139
.LBB912_80:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB912_113
	addq	$272, %rax
	jmp	.LBB912_114
.LBB912_82:
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	%fs:(%rax), %rax
	testq	%rax, %rax
	je	.LBB912_116
	addq	$272, %rax
	movq	%r14, %rbx
	jmp	.LBB912_118
.LBB912_84:
	movq	%r12, 104(%rsp)
	cmpq	$6, %r15
	jb	.LBB912_89
	movq	1528(%rsp), %rdi
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB912_89
.LBB912_86:
	vmovups	752(%rsp), %zmm1
	vmovups	736(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 400(%rsp)
	vmovups	%zmm0, 384(%rsp)
	jmp	.LBB912_190
.LBB912_87:
	movq	%r13, %r12
.LBB912_88:
	movq	%r12, 104(%rsp)
.LBB912_89:
	subq	%r12, %r13
	je	.LBB912_102
	shrq	$3, %r13
	movabsq	$-3689348814741910323, %r14
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	xorl	%r15d, %r15d
	imulq	%r13, %r14
	movq	free@GOTPCREL(%rip), %r13
	jmp	.LBB912_94
	.p2align	4
.LBB912_91:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_92:
	vzeroupper
	callq	*%r13
.LBB912_93:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB912_102
.LBB912_94:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r12,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_93
	leaq	(%r12,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rbx, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%rbx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_97
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_97:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_92
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_97
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
	movq	(%rbp), %rax
	.p2align	4
.LBB912_100:
	cmpq	%rax, %rdx
	jge	.LBB912_91
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_100
	jmp	.LBB912_91
.LBB912_102:
	movq	688(%rsp), %rax
	movq	1272(%rsp), %r14
	movq	16(%rsp), %r15
	testq	%rax, %rax
	je	.LBB912_112
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_105
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_105:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_111
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_105
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
.LBB912_108:
	cmpq	%rax, %rdx
	jge	.LBB912_110
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB912_108
.LBB912_110:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_111:
	movq	free@GOTPCREL(%rip), %rax
	movq	256(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB912_112:
	vmovdqu	320(%rsp), %xmm0
	movq	336(%rsp), %rax
	xorl	%ebp, %ebp
	movq	%rax, 1232(%rsp)
	vmovdqa	%xmm0, 1216(%rsp)
	movq	696(%r15), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB912_650
	jmp	.LBB912_634
.LBB912_113:
.Ltmp10954:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp10955:
.LBB912_114:
	movq	(%rax), %rax
	movq	520(%rax), %r12
	movq	%r14, %rax
	cmpq	$1, %r12
	adcq	$0, %r12
	movq	%r12, %rcx
	shlq	$6, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB912_120
	movq	%r14, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB912_121
.LBB912_116:
.Ltmp10930:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp10931:
	movq	1184(%rsp), %r13
	movq	1192(%rsp), %rbx
.LBB912_118:
	movq	(%rax), %rax
	movq	520(%rax), %rcx
	movq	%r14, %rax
	cmpq	$1, %rcx
	adcq	$0, %rcx
	shlq	$2, %rcx
	orq	%rcx, %rax
	shrq	$32, %rax
	je	.LBB912_127
	movq	%r14, %rax
	xorl	%edx, %edx
	divq	%rcx
	jmp	.LBB912_128
.LBB912_120:
	movl	%r14d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB912_121:
	cmpq	$65, %rax
	movl	$64, %ecx
	movq	%r13, 384(%rsp)
	movq	%r14, 392(%rsp)
	cmovaeq	%rax, %rcx
	movq	%rcx, 400(%rsp)
.Ltmp10956:
	leaq	1040(%rsp), %rbx
	leaq	384(%rsp), %rsi
	movq	%rbx, %rdi
	callq	<alloc::vec::Vec<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>]> as alloc::vec::spec_from_iter::SpecFromIter<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>], core::slice::iter::Chunks<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>>::from_iter
.Ltmp10957:
	movq	1056(%rsp), %r13
	movabsq	$33909456017848440, %rax
	imulq	$272, %r13, %r14
	cmpq	%rax, %r13
	jbe	.LBB912_125
	xorl	%r15d, %r15d
.LBB912_124:
.Ltmp10983:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp10984:
	jmp	.LBB912_819
.LBB912_125:
	testq	%r14, %r14
	je	.LBB912_231
	movl	$16, %esi
	movq	%r14, %rdi
	movl	$16, %r15d
	callq	__rustc::__rust_alloc
	movq	%r13, %rcx
	testq	%rax, %rax
	jne	.LBB912_232
	jmp	.LBB912_124
.LBB912_127:
	movl	%r14d, %eax
	xorl	%edx, %edx
	divl	%ecx
.LBB912_128:
	cmpq	$17, %rax
	movl	$16, %r15d
	movq	$0, 1008(%rsp)
	movq	$16, 1016(%rsp)
	movq	$0, 1024(%rsp)
	cmovaeq	%rax, %r15
	movq	%rbx, %rax
	orq	%r15, %rax
	shrq	$32, %rax
	je	.LBB912_130
	movq	%rbx, %rax
	xorl	%edx, %edx
	divq	%r15
	jmp	.LBB912_131
.LBB912_130:
	movl	%ebx, %eax
	xorl	%edx, %edx
	divl	%r15d
.LBB912_131:
	xorl	%r14d, %r14d
	testq	%rdx, %rdx
	setne	%r14b
	addq	%rax, %r14
	movq	%r14, 1392(%rsp)
	jne	.LBB912_814
	xorl	%eax, %eax
	xorl	%r12d, %r12d
	subq	%r12, %rax
	cmpq	%r14, %rax
	jb	.LBB912_816
.LBB912_133:
	leaq	96(%rsp), %rax
	leaq	176(%rsp), %rcx
	movq	%r13, 384(%rsp)
	movq	%rbx, 392(%rsp)
	movq	%r15, 400(%rsp)
	movq	1016(%rsp), %rbp
	leaq	352(%rsp), %rdx
	movq	%r15, 240(%rsp)
	movq	%r13, 224(%rsp)
	movq	%rbx, 232(%rsp)
	movq	%rax, 408(%rsp)
	movq	%rcx, 416(%rsp)
	movq	rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rcx
	movq	%rdx, 424(%rsp)
	movq	%fs:(%rcx), %rax
	testq	%rax, %rax
	je	.LBB912_135
	addq	$272, %rax
	jmp	.LBB912_136
.LBB912_135:
.Ltmp10934:
	movq	rayon_core::registry::global_registry@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp10935:
.LBB912_136:
	movq	(%rax), %rax
	leaq	408(%rsp), %rdx
	movq	520(%rax), %rcx
	movq	%r12, %rax
	shlq	$8, %rax
	movq	%rdx, 320(%rsp)
	addq	%rax, %rbp
	movq	%rbp, 328(%rsp)
	movq	%r14, 336(%rsp)
.Ltmp10936:
	leaq	320(%rsp), %rbx
	leaq	1040(%rsp), %rdi
	leaq	224(%rsp), %r9
	movl	$1, %r8d
	movq	%r14, %rsi
	xorl	%edx, %edx
	movq	%rbx, (%rsp)
	callq	rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#1}>>
.Ltmp10937:
	movq	1056(%rsp), %r15
	movq	%r15, 320(%rsp)
	cmpq	%r14, %r15
	jne	.LBB912_817
	vmovdqu	1008(%rsp), %xmm0
	addq	%r14, %r12
	movq	%r12, 400(%rsp)
	vmovdqa	%xmm0, 384(%rsp)
.Ltmp10944:
	leaq	720(%rsp), %rdi
	leaq	384(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
.Ltmp10945:
	movq	16(%rsp), %r15
.LBB912_139:
	movq	88(%rsp), %rbx
.LBB912_140:
	movq	720(%rsp), %rdx
	cmpq	$-1, %rdx
	je	.LBB912_148
	vmovups	912(%rsp), %zmm0
	vmovups	896(%rsp), %zmm1
	movq	744(%rsp), %rax
	vmovdqu	728(%rsp), %xmm2
	movq	760(%rsp), %rsi
	movq	752(%rsp), %rcx
	movq	%rdx, 1304(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rdx
	cmpq	$-2, %rdx
	movl	$1, %edx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rdx
	vmovups	%zmm0, 1664(%rsp)
	vmovups	%zmm1, 1648(%rsp)
	vmovdqu64	768(%rsp), %zmm0
	vmovdqu64	832(%rsp), %zmm1
	decq	%rax
	vmovdqu	%xmm2, 1312(%rsp)
	vmovdqu64	1664(%rsp), %zmm4
	vmovdqu64	1648(%rsp), %zmm3
	vmovdqu64	%zmm0, 1520(%rsp)
	vmovdqu64	%zmm1, 1584(%rsp)
	vmovdqu64	%zmm1, 808(%rsp)
	vmovdqu64	%zmm0, 744(%rsp)
	vmovdqu64	%zmm4, 888(%rsp)
	vmovdqu64	%zmm3, 872(%rsp)
	movq	%rdi, 720(%rsp)
	movq	%rcx, 728(%rsp)
	movq	%rdx, 736(%rsp)
	movq	$0, 952(%rsp)
	movq	%rax, 960(%rsp)
.Ltmp10989:
	leaq	384(%rsp), %rdi
	leaq	720(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp10990:
	vmovups	560(%rsp), %zmm3
	vmovups	544(%rsp), %zmm2
	vmovups	384(%rsp), %ymm0
	vmovdqu	1304(%rsp), %xmm1
	movq	1320(%rsp), %rax
	cmpb	$2, 2154(%rsp)
	movq	%rax, 1296(%rsp)
	vmovups	%zmm3, 1664(%rsp)
	vmovups	%zmm2, 1648(%rsp)
	vmovdqu64	480(%rsp), %zmm3
	vmovdqu64	416(%rsp), %zmm2
	vmovups	%ymm0, 1008(%rsp)
	vmovdqa	%xmm1, 1280(%rsp)
	vmovdqu64	%zmm3, 1584(%rsp)
	vmovdqu64	%zmm2, 1520(%rsp)
	jne	.LBB912_149
	cmpq	$0, 616(%r15)
	je	.LBB912_149
	movq	1520(%rsp), %rax
	vmovdqu64	1664(%rsp), %zmm0
	vmovdqu64	1544(%rsp), %zmm1
	vmovdqu64	1608(%rsp), %zmm2
	movq	1536(%rsp), %rdx
	movq	1528(%rsp), %rcx
	movl	$1, %edi
	movl	$1, %esi
	cmpq	$3, %rax
	cmovaeq	%rax, %rdi
	cmovaeq	%rdx, %rax
	cmovaeq	%rsi, %rdx
	decq	%rax
	vmovdqu64	%zmm0, 864(%rsp)
	vmovdqu64	%zmm2, 808(%rsp)
	vmovdqu64	%zmm1, 744(%rsp)
	movq	%rdi, 720(%rsp)
	movq	%rcx, 728(%rsp)
	movq	%rdx, 736(%rsp)
	movq	$0, 928(%rsp)
	movq	%rax, 936(%rsp)
.Ltmp10992:
	leaq	1496(%rsp), %rdi
	leaq	720(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp10993:
	movq	1512(%rsp), %rcx
	movq	1504(%rsp), %r14
	imulq	$200, %rcx, %rax
	addq	%r14, %rax
	movq	%rax, 40(%rsp)
	testq	%rcx, %rcx
	je	.LBB912_178
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB912_151
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB912_170
.LBB912_148:
	vmovdqu64	768(%rsp), %zmm0
	vmovdqu	736(%rsp), %ymm1
	vmovdqu64	%zmm0, 48(%rbx)
	vmovdqu	%ymm1, 16(%rbx)
	vmovdqu64	%zmm0, 1520(%rsp)
	movq	$1, (%rbx)
	movq	304(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB912_279
	jmp	.LBB912_281
.LBB912_149:
	vmovdqu	1304(%rsp), %xmm0
	movq	1320(%rsp), %rax
	movq	%rax, 1344(%rsp)
	vmovdqa	%xmm0, 1328(%rsp)
.Ltmp11110:
	leaq	1520(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp11111:
	movl	$0, 24(%rsp)
	jmp	.LBB912_612
.LBB912_151:
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%r14), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB912_153
	.p2align	4
.LBB912_152:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB912_169
.LBB912_153:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB912_162
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB912_155
.LBB912_163:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB912_156
.LBB912_164:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB912_165
	.p2align	4
.LBB912_157:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB912_166
.LBB912_158:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB912_159
.LBB912_167:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB912_160
.LBB912_168:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB912_152
	jmp	.LBB912_161
.LBB912_162:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB912_163
	.p2align	4
.LBB912_155:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB912_164
.LBB912_156:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB912_157
.LBB912_165:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB912_158
.LBB912_166:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB912_167
	.p2align	4
.LBB912_159:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB912_168
.LBB912_160:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB912_152
.LBB912_161:
	movq	%r11, %r10
	jmp	.LBB912_152
.LBB912_169:
	testq	%rsi, %rsi
	je	.LBB912_174
.LBB912_170:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%r14), %rdi
	.p2align	4
.LBB912_171:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB912_173
.LBB912_172:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB912_171
	jmp	.LBB912_174
.LBB912_173:
	movq	$-1, %rax
	jmp	.LBB912_172
.LBB912_174:
	testq	%r10, %r10
	je	.LBB912_178
	movq	616(%r15), %rax
	testq	%rax, %rax
	je	.LBB912_178
	cmpq	$0, 336(%rax)
	je	.LBB912_178
	lock		addq	%r10, 352(%rax)
.LBB912_178:
	movq	1496(%rsp), %rax
	movq	40(%rsp), %rdx
	movq	%r14, 384(%rsp)
	movq	$0, 1104(%rsp)
	movq	$8, 1112(%rsp)
	movq	$0, 1120(%rsp)
	movq	%rax, 400(%rsp)
	movq	%rdx, 408(%rsp)
	testq	%rcx, %rcx
	je	.LBB912_187
	movl	$8, %eax
	leaq	728(%rsp), %r13
	addq	$200, %r14
	xorl	%r12d, %r12d
	xorl	%ebp, %ebp
	movq	%rax, 24(%rsp)
	.p2align	4
.LBB912_180:
	movq	-200(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB912_188
	leaq	-200(%r14), %rbx
	vmovups	8(%rbx), %zmm0
	vmovups	72(%rbx), %zmm1
	vmovups	96(%rbx), %zmm2
	vmovups	%zmm2, 88(%r13)
	vmovups	%zmm1, 64(%r13)
	vmovups	%zmm0, (%r13)
	movq	%rax, 720(%rsp)
	movzbl	872(%rsp), %r15d
	cmpq	1104(%rsp), %r12
	jne	.LBB912_184
.Ltmp10995:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1104(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp10996:
	movq	1112(%rsp), %rax
	movq	%rax, 24(%rsp)
.LBB912_184:
	vmovdqu64	720(%rsp), %zmm0
	vmovdqu64	784(%rsp), %zmm1
	vmovdqu64	816(%rsp), %zmm2
	movq	24(%rsp), %rax
	vmovdqu64	%zmm2, 96(%rax,%rbp)
	vmovdqu64	%zmm1, 64(%rax,%rbp)
	vmovdqu64	%zmm0, (%rax,%rbp)
	leaq	1(%r12), %rax
	movq	%rax, 1120(%rsp)
	testb	%r15b, %r15b
	jne	.LBB912_223
	movq	16(%rsp), %r15
	addq	$160, %rbp
	addq	$200, %r14
	addq	$200, %rbx
	movq	%rax, %r12
	cmpq	40(%rsp), %rbx
	jne	.LBB912_180
	movq	40(%rsp), %r14
	movq	%rax, %r12
	jmp	.LBB912_188
.LBB912_187:
	movl	$8, %eax
	xorl	%r12d, %r12d
	movq	%rax, 24(%rsp)
.LBB912_188:
	movq	%r14, 392(%rsp)
	xorl	%ebp, %ebp
	jmp	.LBB912_224
.LBB912_189:
	movzbl	731(%rsp), %ecx
	movzwl	729(%rsp), %esi
	vmovups	736(%rsp), %zmm0
	vmovups	752(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	732(%rsp), %ecx
	vmovups	%zmm0, 384(%rsp)
	vmovups	%zmm1, 400(%rsp)
.LBB912_190:
	vmovdqu64	384(%rsp), %zmm0
	vmovdqu64	400(%rsp), %zmm1
	movq	88(%rsp), %rdi
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
	jb	.LBB912_192
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB912_192:
	subq	%r12, %r13
	je	.LBB912_205
	shrq	$3, %r13
	movabsq	$-3689348814741910323, %r14
	xorl	%r15d, %r15d
	imulq	%r13, %r14
	jmp	.LBB912_197
.LBB912_194:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_195:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB912_196:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB912_205
.LBB912_197:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r12,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_196
	leaq	(%r12,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rbx, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%rbx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_200
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_200:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_195
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_200
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
	movq	(%rbp), %rax
	.p2align	4
.LBB912_203:
	cmpq	%rax, %rdx
	jge	.LBB912_194
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_203
	jmp	.LBB912_194
.LBB912_205:
	movq	688(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_207
	movq	256(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB912_207:
	movq	24(%rsp), %r12
	movq	40(%rsp), %r15
	testq	%r12, %r12
	je	.LBB912_220
	xorl	%r14d, %r14d
	jmp	.LBB912_212
.LBB912_209:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_210:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB912_211:
	incq	%r14
	cmpq	%r12, %r14
	je	.LBB912_220
.LBB912_212:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_211
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rbx, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%rbx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_215
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_215:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_210
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_215
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
	movq	(%rbp), %rax
	.p2align	4
.LBB912_218:
	cmpq	%rax, %rdx
	jge	.LBB912_209
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_218
	jmp	.LBB912_209
.LBB912_220:
	movq	320(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_222
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r15, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB912_222:
	xorl	%r15d, %r15d
	jmp	.LBB912_755
.LBB912_223:
	movq	16(%rsp), %r15
	incq	%r12
	movb	$1, %bpl
	movq	%r14, 392(%rsp)
.LBB912_224:
.Ltmp11003:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp11004:
	movq	24(%rsp), %rdx
	movq	1104(%rsp), %rax
	movq	%rax, 40(%rsp)
	testq	%r12, %r12
	je	.LBB912_228
	cmpq	$8, %r12
	jae	.LBB912_229
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB912_242
.LBB912_228:
	xorl	%r14d, %r14d
	jmp	.LBB912_244
.LBB912_229:
	cmpq	$32, %r12
	jae	.LBB912_235
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB912_239
.LBB912_231:
	movl	$16, %eax
	xorl	%ecx, %ecx
.LBB912_232:
	testq	%r13, %r13
	je	.LBB912_292
	movl	%r13d, %edx
	andl	$7, %edx
	cmpq	$8, %r13
	jae	.LBB912_287
	xorl	%esi, %esi
	jmp	.LBB912_290
.LBB912_235:
	vmovdqa64	.LCPI912_1(%rip), %zmm1
	vpbroadcastq	.LCPI912_2(%rip), %zmm2
	vpbroadcastq	.LCPI912_3(%rip), %zmm3
	movq	%r12, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB912_236:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	64(%rdx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%rdx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%rdx,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%rdx,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB912_236
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
	cmpq	%rax, %r12
	je	.LBB912_244
	testb	$24, %r12b
	je	.LBB912_242
.LBB912_239:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI912_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI912_2(%rip), %zmm2
	vpbroadcastq	.LCPI912_4(%rip), %zmm3
	movq	24(%rsp), %rdx
	movq	%r12, %rax
	andq	$-8, %rax
	vmovq	%r14, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB912_240:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%rdx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB912_240
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r14
	cmpq	%rax, %r12
	je	.LBB912_244
.LBB912_242:
	movq	24(%rsp), %rdx
	movq	%r12, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%rdx), %rax
	.p2align	4
.LBB912_243:
	addq	(%rax), %r14
	addq	$160, %rax
	decq	%rcx
	jne	.LBB912_243
.LBB912_244:
	movzbl	2155(%rsp), %eax
	movq	40(%rsp), %r8
	movzbl	2153(%rsp), %r13d
	movq	1296(%rsp), %rbx
	movq	24(%rsp), %rcx
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
	movq	$0, 400(%rsp)
	movq	%r8, 1040(%rsp)
	movq	%rcx, 1048(%rsp)
	movq	%r12, 1056(%rsp)
	movq	%rax, 1440(%rsp)
	movb	%bpl, 1064(%rsp)
.Ltmp11009:
	leaq	720(%rsp), %rdi
	leaq	384(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp11010:
	vmovups	728(%rsp), %xmm0
	movq	720(%rsp), %rcx
	movq	744(%rsp), %rax
	movq	%r14, 1456(%rsp)
	movl	%ebp, 156(%rsp)
	movq	%rax, 112(%rsp)
	movq	%rcx, 256(%rsp)
	vmovaps	%xmm0, 96(%rsp)
	cmpq	$-1, %rcx
	je	.LBB912_282
	vmovups	768(%rsp), %ymm0
	movzbl	759(%rsp), %edx
	movzwl	757(%rsp), %ecx
	movzbl	752(%rsp), %eax
	movq	1288(%rsp), %r14
	shll	$16, %edx
	movl	%eax, 32(%rsp)
	movl	753(%rsp), %eax
	orl	%edx, %ecx
	movq	760(%rsp), %rdx
	shlq	$32, %rcx
	vmovups	%ymm0, 1104(%rsp)
	vmovups	784(%rsp), %ymm0
	movq	%rcx, 64(%rsp)
	movq	112(%rsp), %rcx
	movq	%rax, 688(%rsp)
	movq	%rdx, 56(%rsp)
	movq	%rcx, 1088(%rsp)
	vmovups	%ymm0, 1120(%rsp)
	vmovdqa	96(%rsp), %xmm0
	vmovdqa	%xmm0, 1072(%rsp)
	testq	%rbx, %rbx
	je	.LBB912_259
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %rbp
	xorl	%r15d, %r15d
	jmp	.LBB912_251
.LBB912_248:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_249:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB912_250:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB912_259
.LBB912_251:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r14,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_250
	leaq	(%r14,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rbp, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%rbp, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rbp, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_254
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_254:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_249
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_254
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
.LBB912_257:
	cmpq	%rax, %rdx
	jge	.LBB912_248
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB912_257
	jmp	.LBB912_248
.LBB912_259:
	movq	64(%rsp), %rax
	addq	%rax, 688(%rsp)
	movq	1280(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_261
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB912_261:
	movq	16(%rsp), %r15
	movl	156(%rsp), %ebp
	movl	32(%rsp), %r13d
	testq	%r12, %r12
	je	.LBB912_265
.LBB912_262:
	movq	24(%rsp), %r14
	movl	$1, %ebx
	subq	%r12, %rbx
	.p2align	4
.LBB912_263:
.Ltmp11098:
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11099:
	incq	%rbx
	addq	$160, %r14
	cmpq	$1, %rbx
	jne	.LBB912_263
.LBB912_265:
	movq	40(%rsp), %rax
	movq	88(%rsp), %rbx
	movq	256(%rsp), %r14
	testq	%rax, %rax
	je	.LBB912_276
	shlq	$5, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_268
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB912_268:
	movq	24(%rsp), %rdi
	.p2align	4
.LBB912_269:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_275
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_269
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
.LBB912_272:
	cmpq	%rax, %rsi
	jge	.LBB912_274
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB912_272
.LBB912_274:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_275:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB912_276:
	cmpq	$-1, %r14
	je	.LBB912_601
	vmovdqa	1072(%rsp), %xmm0
	movq	1088(%rsp), %rax
	vmovups	1120(%rsp), %ymm1
	vmovdqu	1104(%rsp), %ymm2
	movq	688(%rsp), %rdx
	movzbl	%r13b, %ecx
	movq	%rax, 1376(%rsp)
	shlq	$8, %rdx
	movq	1376(%rsp), %rax
	orq	%rdx, %rcx
	vmovdqa	%xmm0, 1360(%rsp)
	vmovups	%ymm1, 80(%rbx)
	vmovdqu	%ymm2, 64(%rbx)
	vmovdqa	1360(%rsp), %xmm1
	movq	%rax, 1488(%rsp)
	movq	%rax, 40(%rbx)
	movq	56(%rsp), %rax
	vmovdqu	%xmm1, 24(%rbx)
	movq	%r14, 16(%rbx)
	movq	%rcx, 48(%rbx)
	movq	%rax, 56(%rbx)
	vmovdqa	%xmm1, 1472(%rsp)
	movq	$1, (%rbx)
.Ltmp11104:
	leaq	1008(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp11105:
	movq	304(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_281
.LBB912_279:
	lock		decq	(%rax)
	jne	.LBB912_281
	#MEMBARRIER
.Ltmp11163:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11164:
.LBB912_281:
	movb	$1, %r15b
	jmp	.LBB912_755
.LBB912_282:
	movq	112(%rsp), %rax
	vmovdqa	96(%rsp), %xmm0
	movq	1288(%rsp), %rdx
	movq	1280(%rsp), %rsi
	leaq	(%rbx,%rbx,4), %rcx
	movq	%rax, 368(%rsp)
	movq	616(%r15), %rax
	leaq	(%rdx,%rcx,8), %rcx
	movq	%rdx, 224(%rsp)
	movq	%rsi, 240(%rsp)
	movq	%rdx, 232(%rsp)
	movq	%rdx, 640(%rsp)
	movq	%rcx, 248(%rsp)
	vmovdqa	%xmm0, 352(%rsp)
	testq	%rax, %rax
	je	.LBB912_299
	movb	%r13b, 79(%rsp)
	lock		incq	(%rax)
	jle	.LBB912_819
	movq	616(%r15), %r13
	testq	%r12, %r12
	sete	%al
	movq	%r13, 1096(%rsp)
	movq	%r13, 64(%rsp)
	movq	16(%r13), %rcx
	movq	%rcx, 632(%rsp)
	movq	40(%r13), %rcx
	cmpq	$-1, %rcx
	movq	%rcx, 208(%rsp)
	sete	%cl
	orb	%al, %cl
	jne	.LBB912_368
	cmpq	$8, %r12
	jae	.LBB912_355
	movq	16(%rsp), %rdx
	movq	24(%rsp), %rsi
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB912_364
.LBB912_287:
	movabsq	$36028797018963960, %rdi
	xorl	%esi, %esi
	movq	%rax, %r8
	andq	%r13, %rdi
.LBB912_288:
	movl	$0, (%r8)
	movb	$0, 4(%r8)
	movq	$2, 16(%r8)
	movl	$0, 272(%r8)
	movb	$0, 276(%r8)
	movq	$2, 288(%r8)
	movl	$0, 544(%r8)
	movb	$0, 548(%r8)
	movq	$2, 560(%r8)
	movl	$0, 816(%r8)
	movb	$0, 820(%r8)
	movq	$2, 832(%r8)
	movl	$0, 1088(%r8)
	movb	$0, 1092(%r8)
	movq	$2, 1104(%r8)
	movl	$0, 1360(%r8)
	movb	$0, 1364(%r8)
	movq	$2, 1376(%r8)
	movl	$0, 1632(%r8)
	movb	$0, 1636(%r8)
	movq	$2, 1648(%r8)
	movl	$0, 1904(%r8)
	movb	$0, 1908(%r8)
	movq	$2, 1920(%r8)
	addq	$8, %rsi
	addq	$2176, %r8
	cmpq	%rsi, %rdi
	jne	.LBB912_288
	testq	%rdx, %rdx
	je	.LBB912_292
.LBB912_290:
	imulq	$272, %rsi, %rsi
	imulq	$272, %rdx, %rdx
	xorl	%edi, %edi
	addq	%rax, %rsi
	.p2align	4
.LBB912_291:
	movl	$0, (%rsi,%rdi)
	movb	$0, 4(%rsi,%rdi)
	movq	$2, 16(%rsi,%rdi)
	addq	$272, %rdi
	cmpq	%rdi, %rdx
	jne	.LBB912_291
.LBB912_292:
	movq	1056(%rsp), %rdi
	movq	%rcx, 224(%rsp)
	movq	%rax, 232(%rsp)
	leaq	1008(%rsp), %rax
	movq	%r13, 240(%rsp)
	movq	$0, 1008(%rsp)
	movq	%rax, 384(%rsp)
	leaq	96(%rsp), %rax
	movq	%rbx, 392(%rsp)
	movq	%rax, 400(%rsp)
	leaq	176(%rsp), %rax
	movq	%rax, 408(%rsp)
	leaq	1184(%rsp), %rax
	movq	%rax, 416(%rsp)
	leaq	224(%rsp), %rax
	cmpq	%rdi, %r12
	movq	%rax, 424(%rsp)
	cmovbq	%r12, %rdi
.Ltmp10958:
	leaq	384(%rsp), %rsi
	callq	<rayon::range::Iter<usize> as rayon::iter::ParallelIterator>::drive_unindexed::<rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#1}>>
.Ltmp10959:
	movq	240(%rsp), %rsi
	movq	232(%rsp), %rbx
	movq	224(%rsp), %r15
	movabsq	$-1085102592571150095, %rcx
	imulq	$272, %rsi, %rdx
	movq	%rbx, %r14
	movq	%rbx, %r12
	leaq	(%rbx,%rdx), %rax
	testq	%rsi, %rsi
	je	.LBB912_341
	addq	$-272, %rdx
	mulxq	%rcx, %rsi, %rsi
	shrl	$8, %esi
	incl	%esi
	andl	$7, %esi
	je	.LBB912_321
	imulq	$272, %rsi, %rdi
	movq	%rbx, %rsi
	movq	%rbx, %r12
	jmp	.LBB912_297
	.p2align	4
.LBB912_296:
	addq	$272, %rsi
	addq	$-272, %rdi
	je	.LBB912_322
.LBB912_297:
	vmovdqu64	24(%rsi), %zmm0
	vmovdqu64	88(%rsi), %zmm1
	vmovdqu64	152(%rsi), %zmm2
	vmovdqu64	208(%rsi), %zmm3
	movq	16(%rsi), %r8
	vmovdqu64	%zmm3, 568(%rsp)
	vmovdqu64	%zmm2, 512(%rsp)
	vmovdqu64	%zmm1, 448(%rsp)
	vmovdqu64	%zmm0, 384(%rsp)
	cmpq	$2, %r8
	je	.LBB912_296
	movq	%r8, (%r12)
	vmovdqu64	384(%rsp), %zmm0
	vmovdqu64	448(%rsp), %zmm1
	vmovdqu64	512(%rsp), %zmm2
	vmovdqu64	568(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	vmovdqu64	%zmm1, 72(%r12)
	vmovdqu64	%zmm3, 192(%r12)
	addq	$256, %r12
	jmp	.LBB912_296
.LBB912_299:
	movq	232(%rsp), %rcx
	movq	224(%rsp), %rax
	movq	240(%rsp), %rdx
	movq	%rcx, 728(%rsp)
	movq	248(%rsp), %rcx
	movq	%rax, 720(%rsp)
	movq	%rdx, 736(%rsp)
	movq	%rcx, 744(%rsp)
	movq	728(%rsp), %rbx
	movq	744(%rsp), %rcx
	cmpq	%rcx, %rbx
	je	.LBB912_305
	leaq	352(%rsp), %rax
	addq	$40, %rbx
	movq	%r12, 216(%rsp)
	movq	%rax, 256(%rsp)
	movq	%rbx, %rax
	jmp	.LBB912_302
.LBB912_301:
	movq	360(%rsp), %rcx
	leaq	(%r13,%r13,4), %rax
	incq	%r13
	addq	$40, %r14
	movq	%rbp, (%rcx,%rax,8)
	movq	%r15, 8(%rcx,%rax,8)
	vmovdqa	1392(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rcx,%rax,8)
	movq	1408(%rsp), %rdx
	movq	%rdx, 32(%rcx,%rax,8)
	movq	%r13, 368(%rsp)
	leaq	40(%rbx), %rax
	movq	%r12, %rcx
	cmpq	%r12, %r14
	movq	216(%rsp), %r12
	je	.LBB912_305
.LBB912_302:
	movq	-40(%rax), %rbp
	movq	%rax, %rbx
	testq	%rbp, %rbp
	je	.LBB912_305
	leaq	-40(%rbx), %r14
	movq	-32(%rbx), %r15
	movq	368(%rsp), %r13
	movq	%rcx, %r12
	movq	32(%r14), %rax
	movq	%rax, 1408(%rsp)
	vmovdqu	16(%r14), %xmm0
	vmovdqa	%xmm0, 1392(%rsp)
	cmpq	352(%rsp), %r13
	jne	.LBB912_301
.Ltmp11092:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	callq	*%rax
.Ltmp11093:
	jmp	.LBB912_301
.LBB912_305:
	subq	%rbx, %rcx
	je	.LBB912_318
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	shrq	$3, %rcx
	movabsq	$-3689348814741910323, %r14
	movabsq	$9223372036854775807, %r13
	xorl	%r15d, %r15d
	imulq	%rcx, %r14
	jmp	.LBB912_310
.LBB912_307:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_308:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_309:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB912_318
.LBB912_310:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_309
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r13, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r13, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r13, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_313
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_313:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_308
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_313
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
.LBB912_316:
	cmpq	%rax, %rdx
	jge	.LBB912_307
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_316
	jmp	.LBB912_307
.LBB912_318:
	movq	736(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_320
	movq	720(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB912_320:
	vmovdqa	352(%rsp), %xmm0
	movq	368(%rsp), %rax
	movq	16(%rsp), %r15
	movl	156(%rsp), %ebp
	movb	$2, %r13b
	movq	$-1, 256(%rsp)
	movq	%rax, 1088(%rsp)
	vmovdqa	%xmm0, 1072(%rsp)
	testq	%r12, %r12
	jne	.LBB912_262
	jmp	.LBB912_265
.LBB912_321:
	movq	%rbx, %rsi
	movq	%rbx, %r12
.LBB912_322:
	movq	%rax, %r14
	cmpq	$1904, %rdx
	jae	.LBB912_324
	jmp	.LBB912_341
.LBB912_323:
	addq	$2176, %rsi
	cmpq	%rax, %rsi
	je	.LBB912_340
.LBB912_324:
	vmovups	24(%rsi), %zmm0
	vmovups	88(%rsi), %zmm1
	vmovups	152(%rsi), %zmm2
	vmovups	208(%rsi), %zmm3
	movq	16(%rsi), %rdx
	vmovups	%zmm3, 568(%rsp)
	vmovups	%zmm2, 512(%rsp)
	vmovups	%zmm1, 448(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_326
	movq	%rdx, (%r12)
	vmovups	384(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	512(%rsp), %zmm2
	vmovups	568(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB912_326:
	vmovups	296(%rsi), %zmm0
	vmovups	360(%rsi), %zmm1
	vmovups	424(%rsi), %zmm2
	vmovups	480(%rsi), %zmm3
	movq	288(%rsi), %rdx
	vmovups	%zmm3, 568(%rsp)
	vmovups	%zmm2, 512(%rsp)
	vmovups	%zmm1, 448(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_328
	movq	%rdx, (%r12)
	vmovups	384(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	512(%rsp), %zmm2
	vmovups	568(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB912_328:
	vmovups	568(%rsi), %zmm0
	vmovups	632(%rsi), %zmm1
	vmovups	696(%rsi), %zmm2
	vmovups	752(%rsi), %zmm3
	movq	560(%rsi), %rdx
	vmovups	%zmm3, 568(%rsp)
	vmovups	%zmm2, 512(%rsp)
	vmovups	%zmm1, 448(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_330
	movq	%rdx, (%r12)
	vmovups	384(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	512(%rsp), %zmm2
	vmovups	568(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB912_330:
	vmovups	840(%rsi), %zmm0
	vmovups	904(%rsi), %zmm1
	vmovups	968(%rsi), %zmm2
	vmovups	1024(%rsi), %zmm3
	movq	832(%rsi), %rdx
	vmovups	%zmm3, 568(%rsp)
	vmovups	%zmm2, 512(%rsp)
	vmovups	%zmm1, 448(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_332
	movq	%rdx, (%r12)
	vmovups	384(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	512(%rsp), %zmm2
	vmovups	568(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB912_332:
	vmovups	1112(%rsi), %zmm0
	vmovups	1176(%rsi), %zmm1
	vmovups	1240(%rsi), %zmm2
	vmovups	1296(%rsi), %zmm3
	movq	1104(%rsi), %rdx
	vmovups	%zmm3, 568(%rsp)
	vmovups	%zmm2, 512(%rsp)
	vmovups	%zmm1, 448(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_334
	movq	%rdx, (%r12)
	vmovups	384(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	512(%rsp), %zmm2
	vmovups	568(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB912_334:
	vmovups	1384(%rsi), %zmm0
	vmovups	1448(%rsi), %zmm1
	vmovups	1512(%rsi), %zmm2
	vmovups	1568(%rsi), %zmm3
	movq	1376(%rsi), %rdx
	vmovups	%zmm3, 568(%rsp)
	vmovups	%zmm2, 512(%rsp)
	vmovups	%zmm1, 448(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_336
	movq	%rdx, (%r12)
	vmovups	384(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	512(%rsp), %zmm2
	vmovups	568(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB912_336:
	vmovups	1656(%rsi), %zmm0
	vmovups	1720(%rsi), %zmm1
	vmovups	1784(%rsi), %zmm2
	vmovups	1840(%rsi), %zmm3
	movq	1648(%rsi), %rdx
	vmovups	%zmm3, 568(%rsp)
	vmovups	%zmm2, 512(%rsp)
	vmovups	%zmm1, 448(%rsp)
	vmovups	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_338
	movq	%rdx, (%r12)
	vmovups	384(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	512(%rsp), %zmm2
	vmovups	568(%rsp), %zmm3
	vmovups	%zmm2, 136(%r12)
	vmovups	%zmm0, 8(%r12)
	vmovups	%zmm1, 72(%r12)
	vmovups	%zmm3, 192(%r12)
	addq	$256, %r12
.LBB912_338:
	vmovdqu64	1928(%rsi), %zmm0
	vmovdqu64	1992(%rsi), %zmm1
	vmovdqu64	2056(%rsi), %zmm2
	vmovdqu64	2112(%rsi), %zmm3
	movq	1920(%rsi), %rdx
	vmovdqu64	%zmm3, 568(%rsp)
	vmovdqu64	%zmm2, 512(%rsp)
	vmovdqu64	%zmm1, 448(%rsp)
	vmovdqu64	%zmm0, 384(%rsp)
	cmpq	$2, %rdx
	je	.LBB912_323
	movq	%rdx, (%r12)
	vmovdqu64	384(%rsp), %zmm0
	vmovdqu64	448(%rsp), %zmm1
	vmovdqu64	512(%rsp), %zmm2
	vmovdqu64	568(%rsp), %zmm3
	vmovdqu64	%zmm2, 136(%r12)
	vmovdqu64	%zmm0, 8(%r12)
	vmovdqu64	%zmm1, 72(%r12)
	vmovdqu64	%zmm3, 192(%r12)
	addq	$256, %r12
	jmp	.LBB912_323
.LBB912_340:
	movq	%rax, %r14
.LBB912_341:
	vmovdqa	.LCPI912_0(%rip), %ymm0
	subq	%rbx, %r12
	shrq	$8, %r12
	subq	%r14, %rax
	movq	%rax, %rdx
	mulxq	%rcx, %rax, %rax
	movq	%rbx, 1104(%rsp)
	movq	%r12, 1112(%rsp)
	movq	%r15, 1120(%rsp)
	vmovdqu	%ymm0, 384(%rsp)
	je	.LBB912_346
	shrq	$8, %rax
	movl	$1, %r13d
	addq	$288, %r14
	subq	%rax, %r13
	jmp	.LBB912_344
	.p2align	4
.LBB912_343:
	addq	$272, %r14
	incq	%r13
	cmpq	$1, %r13
	je	.LBB912_346
.LBB912_344:
	cmpl	$2, -272(%r14)
	je	.LBB912_343
.Ltmp10964:
	leaq	-272(%r14), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
.Ltmp10965:
	jmp	.LBB912_343
.LBB912_346:
	imulq	$272, %r15, %r14
	testb	$-16, %r14b
	je	.LBB912_351
	movq	%r14, %r15
	andq	$-256, %r15
	je	.LBB912_350
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	leaq	qualification_454_native_cost::GLOBAL (.llvm.11174181910260379007)(%rip), %rdi
	movl	$16, %edx
	movq	%rbx, %rsi
	movq	%r14, %rcx
	movq	%r15, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %rbx
	testq	%rax, %rax
	jne	.LBB912_351
.Ltmp10970:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$16, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp10971:
	jmp	.LBB912_819
.LBB912_350:
	movl	$16, %edx
	movq	%rbx, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
	movl	$16, %ebx
.LBB912_351:
	shrq	$8, %r14
	movq	%r14, 320(%rsp)
	movq	%rbx, 328(%rsp)
	movq	%r12, 336(%rsp)
.Ltmp10978:
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#2}>>
.Ltmp10979:
.Ltmp10980:
	leaq	720(%rsp), %rdi
	leaq	320(%rsp), %rsi
	callq	purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
.Ltmp10981:
	movq	1040(%rsp), %rsi
	movq	88(%rsp), %rbx
	movq	16(%rsp), %r15
	testq	%rsi, %rsi
	je	.LBB912_140
	movq	1048(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB912_140
.LBB912_355:
	movq	16(%rsp), %rdx
	cmpq	$32, %r12
	jae	.LBB912_357
	movq	24(%rsp), %rsi
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB912_361
.LBB912_357:
	vmovdqa64	.LCPI912_1(%rip), %zmm1
	vpbroadcastq	.LCPI912_2(%rip), %zmm2
	vpbroadcastq	.LCPI912_3(%rip), %zmm3
	movq	24(%rsp), %rsi
	movq	%r12, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB912_358:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%rsi,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%rsi,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%rsi,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%rsi,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB912_358
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
	cmpq	%rax, %r12
	je	.LBB912_366
	testb	$24, %r12b
	je	.LBB912_364
.LBB912_361:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI912_1(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI912_2(%rip), %zmm2
	vpbroadcastq	.LCPI912_4(%rip), %zmm3
	movq	%r12, %rax
	andq	$-8, %rax
	vmovq	%rbx, %xmm0
	subq	%rax, %rcx
.LBB912_362:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rsi,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB912_362
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %r12
	je	.LBB912_366
.LBB912_364:
	movq	%r12, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rsi), %rax
.LBB912_365:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB912_365
.LBB912_366:
	movq	912(%rdx), %rax
	movq	928(%rdx), %rsi
	leaq	912(%rdx), %rbp
	subq	%rsi, %rax
	cmpq	%rax, %rbx
	ja	.LBB912_820
.LBB912_367:
	movq	16(%rsp), %rax
	cmpq	1016(%rax), %rbx
	ja	.LBB912_821
.LBB912_368:
	movq	24(%rsp), %rbx
	leaq	16(%r13), %rax
	movq	40(%rsp), %rcx
	movq	%rax, 128(%rsp)
	leaq	(%r12,%r12,4), %rax
	shlq	$5, %rax
	addq	%rbx, %rax
	movq	%rbx, 320(%rsp)
	movq	%rbx, 328(%rsp)
	movq	%rcx, 336(%rsp)
	movq	%rax, 1424(%rsp)
	movq	%rax, 344(%rsp)
	testq	%r12, %r12
	je	.LBB912_549
	movq	16(%rsp), %rcx
	movq	640(%rsp), %r12
	leaq	272(%r13), %rax
	leaq	728(%rsp), %r14
	movq	%rax, 992(%rsp)
	leaq	888(%rcx), %rax
	movq	%rax, 216(%rsp)
.LBB912_370:
	leaq	160(%rbx), %rdx
	movq	%rdx, 328(%rsp)
	movq	(%rbx), %rax
	cmpq	$-1, %rax
	je	.LBB912_550
	movq	%rax, 720(%rsp)
	movq	%rdx, 24(%rsp)
	vmovdqu64	8(%rbx), %zmm0
	vmovdqu64	72(%rbx), %zmm1
	vmovdqu64	96(%rbx), %zmm2
	vmovdqu64	%zmm2, 88(%r14)
	vmovdqu64	%zmm1, 64(%r14)
	vmovdqu64	%zmm0, (%r14)
	movq	744(%rsp), %rcx
	imulq	$88, 736(%rsp), %rsi
	movq	728(%rsp), %rdx
	movq	760(%rsp), %rdi
	movq	776(%rsp), %rbx
	movq	768(%rsp), %r14
	movq	784(%rsp), %rbp
	movq	%rcx, 984(%rsp)
	movq	752(%rsp), %rcx
	movq	%rdx, 96(%rsp)
	movq	%rax, 112(%rsp)
	movq	%rdx, 104(%rsp)
	addq	%rdx, %rsi
	movq	%rsi, 688(%rsp)
	movq	%rsi, 120(%rsp)
	movq	%rcx, 256(%rsp)
	testq	%rbp, %rbp
	je	.LBB912_537
	movq	816(%rsp), %rax
	shlq	$5, %rbp
	addq	$8, %rcx
	movq	%rdi, 1248(%rsp)
	movq	$0, 648(%rsp)
	movq	%rdx, 160(%rsp)
	movq	%rdx, 80(%rsp)
	movq	%rbx, 168(%rsp)
	movq	%r14, 48(%rsp)
	addq	%rbx, %rbp
	movq	%rcx, 1432(%rsp)
	movq	%rbp, 136(%rsp)
	movq	%rax, 56(%rsp)
	movq	%rbx, %rax
	jmp	.LBB912_375
.LBB912_373:
	movq	640(%rsp), %rcx
.LBB912_374:
	movq	1448(%rsp), %rax
	movq	%rcx, %r12
	movq	%rcx, 232(%rsp)
	addq	$32, %rax
	cmpq	%rbp, %rax
	je	.LBB912_537
.LBB912_375:
	movq	648(%rsp), %rcx
	movq	16(%rax), %rdx
	movq	(%rax), %r15
	movq	8(%rax), %r14
	movq	%rax, 1448(%rsp)
	movq	24(%rax), %rax
	movq	$-1, %rbx
	movq	%rcx, 1000(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rax, 40(%rsp)
	testq	%r15, %r15
	je	.LBB912_385
	cmpq	$-1, 632(%rsp)
	je	.LBB912_385
	movq	80(%r13), %rax
	.p2align	4
.LBB912_378:
	movq	%rax, %rcx
	addq	%r15, %rcx
	cmovbq	%rbx, %rcx
	lock		cmpxchgq	%rcx, 80(%r13)
	jne	.LBB912_378
	movq	128(%rsp), %rcx
	addq	%r15, %rax
	cmovbq	%rbx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB912_382
	movq	%rcx, 392(%rsp)
	movq	%rax, 400(%rsp)
	movw	$0, 384(%rsp)
.Ltmp11016:
	movq	128(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp11017:
	cmpb	$-1, 176(%rsp)
	jne	.LBB912_807
.LBB912_382:
	movq	16(%rsp), %rax
	movq	632(%rax), %rax
	testq	%rax, %rax
	je	.LBB912_385
	movq	16(%rsp), %rcx
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB912_385
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	1440(%rsp), %rax
	lock		addq	%r15, (%rcx,%rax,8)
.LBB912_385:
	movq	1248(%rsp), %rax
	movq	1000(%rsp), %rdi
	cmpq	%rax, %rdi
	ja	.LBB912_813
	cmpq	%rax, %r14
	movq	%rax, %rsi
	cmovbq	%r14, %rsi
	cmpq	%rdi, %r14
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB912_812
	leaq	(,%rdi,8), %rax
	movq	%r12, 640(%rsp)
	leaq	(%rax,%rax,2), %r15
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r9
	cmpq	%rsi, %rdi
	leaq	.LJTI912_0(%rip), %rdi
	jne	.LBB912_394
	xorl	%ebp, %ebp
	xorl	%r11d, %r11d
	xorl	%r10d, %r10d
.LBB912_389:
	cmpq	$-1, 208(%rsp)
	movq	$-1, %r14
	movq	%r11, 1256(%rsp)
	movq	%r10, 1264(%rsp)
	movq	%r9, 32(%rsp)
	movq	%rsi, 648(%rsp)
	je	.LBB912_400
	movq	296(%rsp), %rax
	movl	$0, %ecx
	movq	688(%rsp), %rbx
	movl	$0, %r13d
	subq	56(%rsp), %rax
	cmovbq	%rcx, %rax
	subq	80(%rsp), %rbx
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %rbx
	imulq	%rcx, %rbx
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	testq	%rbx, %rbx
	je	.LBB912_401
	movq	80(%rsp), %rax
	xorl	%r13d, %r13d
	leaq	8(%rax), %r12
	.p2align	4
.LBB912_392:
.Ltmp11019:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp11020:
	addq	%rax, %r13
	cmovbq	%r14, %r13
	addq	$88, %r12
	decq	%rbx
	jne	.LBB912_392
	jmp	.LBB912_401
.LBB912_394:
	movq	%r9, %rdx
	subq	%r15, %rdx
	movabsq	$-6148914691236517205, %rax
	xorl	%r10d, %r10d
	xorl	%r11d, %r11d
	xorl	%ebp, %ebp
	mulxq	%rax, %rax, %rax
	movq	1432(%rsp), %rcx
	shrq	$4, %rax
	addq	%r15, %rcx
	jmp	.LBB912_397
.LBB912_395:
	addq	%rdx, %r11
	cmovbq	%rbx, %r11
.LBB912_396:
	addq	$24, %rcx
	decq	%rax
	je	.LBB912_389
.LBB912_397:
	movzbl	-8(%rcx), %r8d
	movq	(%rcx), %rdx
	movslq	(%rdi,%r8,4), %r8
	addq	%rdi, %r8
	jmpq	*%r8
.LBB912_398:
	addq	%rdx, %rbp
	cmovbq	%rbx, %rbp
	jmp	.LBB912_396
.LBB912_399:
	cmpq	%rdx, %r10
	cmovbeq	%rdx, %r10
	jmp	.LBB912_396
.LBB912_400:
	xorl	%r13d, %r13d
.LBB912_401:
	movq	64(%rsp), %rax
	movq	$-1, %rsi
	movl	296(%rax), %eax
	testl	%eax, %eax
	je	.LBB912_418
.LBB912_402:
	cmpq	$-1, 632(%rsp)
	je	.LBB912_404
	movq	64(%rsp), %rcx
	movq	80(%rcx), %rax
	addq	%rbp, %rax
	cmovbq	%rsi, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB912_419
.LBB912_404:
	cmpq	$-1, 208(%rsp)
	je	.LBB912_406
	movq	16(%rsp), %rcx
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	64(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%r13, %rcx
	cmovbq	%rsi, %rcx
	addq	1256(%rsp), %rcx
	cmovbq	%rsi, %rcx
	addq	1264(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%rsi, %rcx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	cmpq	40(%rdx), %rax
	ja	.LBB912_419
.LBB912_406:
	cmpq	$-1, 632(%rsp)
	movq	64(%rsp), %r13
	movq	160(%rsp), %r9
	movq	80(%rsp), %r10
	movq	32(%rsp), %r11
	je	.LBB912_408
	movq	1000(%rsp), %rcx
	movq	%r15, %rax
	cmpq	648(%rsp), %rcx
	jne	.LBB912_413
.LBB912_408:
	movb	$1, %r12b
	movq	1000(%rsp), %rax
	cmpq	648(%rsp), %rax
	je	.LBB912_411
.LBB912_409:
	movq	256(%rsp), %rax
	cmpb	$2, -24(%rax,%r11)
	je	.LBB912_475
	addq	$-24, %r11
	cmpq	%r11, %r15
	jne	.LBB912_409
.LBB912_411:
	movq	16(%rsp), %r15
	jmp	.LBB912_489
	.p2align	4
.LBB912_412:
	addq	$24, %rax
	cmpq	%rax, %r11
	je	.LBB912_408
.LBB912_413:
	movq	256(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB912_412
	movq	256(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB912_412
	movq	16(%rsp), %rdx
	movq	632(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB912_412
	movq	16(%rsp), %rsi
	movl	1228(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB912_412
	movq	256(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB912_412
.LBB912_418:
	movq	992(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB912_402
.LBB912_419:
	movq	168(%rsp), %rbx
	movq	56(%rsp), %r14
	movq	1000(%rsp), %rax
	cmpq	648(%rsp), %rax
	jne	.LBB912_429
	movq	64(%rsp), %r13
	movq	136(%rsp), %rbp
.LBB912_421:
	cmpq	$-1, 208(%rsp)
	movq	%r14, 56(%rsp)
	je	.LBB912_473
	cmpq	296(%rsp), %r14
	movq	48(%rsp), %r14
	jae	.LBB912_528
	movq	80(%rsp), %rax
	cmpq	688(%rsp), %rax
	je	.LBB912_474
	movq	296(%rsp), %rax
	movq	80(%rsp), %rcx
	leaq	-1(%rax), %rbx
.LBB912_425:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB912_483
	movq	(%rdx), %rsi
	movq	%rax, 384(%rsp)
	leaq	392(%rsp), %rcx
	movq	%rdx, %r15
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp11064:
	movq	216(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11065:
	movq	56(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB912_482
	incq	%rax
	leaq	88(%r15), %rcx
	movq	%r15, %rdx
	movq	%rax, 56(%rsp)
	cmpq	688(%rsp), %rcx
	jne	.LBB912_425
	jmp	.LBB912_483
.LBB912_429:
	movq	256(%rsp), %rax
	movq	64(%rsp), %r13
	movq	136(%rsp), %rbp
	addq	%rax, 32(%rsp)
	addq	%rax, %r15
	jmp	.LBB912_432
.LBB912_430:
	movq	64(%rsp), %r13
	movq	136(%rsp), %rbp
	.p2align	4
.LBB912_431:
	addq	$24, %r15
	cmpq	32(%rsp), %r15
	je	.LBB912_421
.LBB912_432:
	movzbl	(%r15), %eax
	leaq	.LJTI912_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB912_433:
	cmpq	$-1, 632(%rsp)
	je	.LBB912_431
	movq	%r13, %rdx
	movzbl	1(%r15), %r13d
	movq	8(%r15), %rbp
	movq	16(%r15), %r12
	movq	80(%rdx), %rax
	movq	$-1, %rsi
	.p2align	4
.LBB912_435:
	movq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB912_435
	movq	128(%rsp), %rcx
	addq	%rbp, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB912_439
	movq	%rcx, 392(%rsp)
	movq	%rax, 400(%rsp)
	movw	$0, 384(%rsp)
.Ltmp11051:
	movq	128(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp11052:
	cmpb	$-1, 176(%rsp)
	jne	.LBB912_792
.LBB912_439:
	cmpl	$255, %r13d
	je	.LBB912_430
	movq	16(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB912_430
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB912_430
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%rbp, (%rcx,%r13,8)
	jmp	.LBB912_430
.LBB912_443:
	movq	8(%r15), %r12
	cmpq	%r12, %r14
	jae	.LBB912_470
	movq	80(%rsp), %rax
	cmpq	688(%rsp), %rax
	je	.LBB912_463
	movq	80(%rsp), %rax
	leaq	-1(%r12), %rbx
	.p2align	4
.LBB912_446:
	movq	%rax, %rdx
	movq	8(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB912_468
	movq	(%rdx), %rsi
	movq	%rax, 384(%rsp)
	leaq	392(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp11040:
	movq	216(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11041:
	cmpq	%r14, %rbx
	je	.LBB912_467
	movq	%rbp, %rdx
	leaq	88(%rbp), %rax
	movq	136(%rsp), %rbp
	incq	%r14
	cmpq	688(%rsp), %rax
	jne	.LBB912_446
	jmp	.LBB912_468
.LBB912_450:
	cmpq	$-1, 208(%rsp)
	je	.LBB912_431
	movq	8(%r15), %rcx
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB912_461
	movq	104(%r13), %rax
	movq	$-1, %rdx
	addq	%rcx, %rax
	movq	40(%r13), %rcx
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB912_431
	movq	%rcx, 392(%rsp)
	movq	%rax, 400(%rsp)
	movw	$768, 384(%rsp)
.Ltmp11038:
	movq	128(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp11039:
	jmp	.LBB912_462
.LBB912_454:
	cmpq	$-1, 208(%rsp)
	je	.LBB912_431
	cmpq	$-1, 40(%r13)
	je	.LBB912_431
	movq	8(%r15), %rcx
	movq	16(%r15), %r12
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB912_464
	movq	104(%r13), %rax
	movq	$-1, %rsi
	.p2align	4
.LBB912_458:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 104(%r13)
	jne	.LBB912_458
	addq	%rcx, %rax
	movq	40(%r13), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB912_431
	movq	%rcx, 392(%rsp)
	movq	%rax, 400(%rsp)
	movw	$768, 384(%rsp)
.Ltmp11045:
	movq	128(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp11046:
	jmp	.LBB912_465
.LBB912_461:
	movq	992(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 192(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB912_462:
	cmpb	$-1, 176(%rsp)
	je	.LBB912_431
	jmp	.LBB912_569
.LBB912_463:
	movq	160(%rsp), %rdx
	jmp	.LBB912_469
.LBB912_464:
	movq	992(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 192(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB912_465:
	movzbl	176(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%r12, %r12
	setne	%dl
	testb	%cl, %dl
	jne	.LBB912_800
	cmpb	$-1, %al
	je	.LBB912_431
	jmp	.LBB912_569
.LBB912_467:
	movq	%rbp, %rdx
	movq	136(%rsp), %rbp
	movq	%r12, %r14
.LBB912_468:
	movq	168(%rsp), %rbx
	addq	$88, %rdx
	movq	%rdx, 80(%rsp)
.LBB912_469:
	movq	%rdx, 160(%rsp)
	movq	%rdx, 104(%rsp)
.LBB912_470:
	cmpq	$-1, 208(%rsp)
	je	.LBB912_431
.Ltmp11043:
	movq	16(%rsp), %rsi
	leaq	384(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp11044:
	cmpb	$-1, 384(%rsp)
	je	.LBB912_431
	jmp	.LBB912_569
.LBB912_473:
	movq	48(%rsp), %r14
	jmp	.LBB912_528
.LBB912_474:
	movq	160(%rsp), %rdx
	jmp	.LBB912_484
.LBB912_475:
	movq	256(%rsp), %rax
	movq	16(%rsp), %r15
	movq	-16(%rax,%r11), %r14
	cmpq	%r14, 56(%rsp)
	jae	.LBB912_488
	cmpq	688(%rsp), %r10
	je	.LBB912_487
	movq	%r10, %rcx
	leaq	-1(%r14), %rbx
.LBB912_478:
	movq	8(%rcx), %rax
	movq	%rcx, %r9
	cmpq	$-1, %rax
	je	.LBB912_486
	movq	(%r9), %rsi
	movq	%rax, 384(%rsp)
	leaq	392(%rsp), %rcx
	movq	%r9, %r12
	movq	80(%r9), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%r9), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp11022:
	movq	216(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11023:
	movq	56(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB912_485
	incq	%rax
	leaq	88(%r12), %rcx
	movq	%r12, %r9
	movq	%rax, 56(%rsp)
	cmpq	688(%rsp), %rcx
	jne	.LBB912_478
	jmp	.LBB912_486
.LBB912_482:
	movq	296(%rsp), %rax
	movq	%r15, %rdx
	movq	%rax, 56(%rsp)
.LBB912_483:
	movq	168(%rsp), %rbx
	addq	$88, %rdx
	movq	%rdx, 80(%rsp)
.LBB912_484:
	movq	%rdx, 160(%rsp)
	movq	%rdx, 104(%rsp)
	jmp	.LBB912_528
.LBB912_485:
	movq	%r12, %r9
	movq	%r14, 56(%rsp)
.LBB912_486:
	addq	$88, %r9
	movq	%r9, 80(%rsp)
.LBB912_487:
	movq	%r9, 160(%rsp)
	movq	%r9, 104(%rsp)
.LBB912_488:
	xorl	%r12d, %r12d
.LBB912_489:
	movq	256(%rsp), %rbx
	movq	48(%rsp), %r14
	cmpq	$-1, 632(%rsp)
	je	.LBB912_497
	testq	%rbp, %rbp
	je	.LBB912_497
	movq	80(%r13), %rax
	movq	$-1, %rdx
	.p2align	4
.LBB912_492:
	movq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%r13)
	jne	.LBB912_492
	movq	128(%rsp), %rcx
	addq	%rbp, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB912_497
	movq	%rcx, 392(%rsp)
	movq	%rax, 400(%rsp)
	movw	$0, 384(%rsp)
.Ltmp11025:
	movq	128(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp11026:
	cmpb	$-1, 176(%rsp)
	je	.LBB912_497
	cmpq	$-1, 208(%rsp)
	movq	136(%rsp), %rbp
	movb	$1, %al
	movl	%eax, 32(%rsp)
	jne	.LBB912_516
	jmp	.LBB912_571
.LBB912_497:
	cmpq	$-1, 208(%rsp)
	je	.LBB912_504
	cmpq	$-1, 40(%r13)
	movq	136(%rsp), %rbp
	je	.LBB912_508
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB912_505
	movq	104(%r13), %rax
	movq	1256(%rsp), %rsi
	movq	$-1, %rdx
.LBB912_501:
	movq	%rax, %rcx
	addq	%rsi, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 104(%r13)
	jne	.LBB912_501
	movq	40(%r13), %rcx
	addq	%rsi, %rax
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB912_508
	movq	%rcx, 392(%rsp)
	movq	%rax, 400(%rsp)
	movw	$768, 384(%rsp)
.Ltmp11028:
	movq	128(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp11029:
	jmp	.LBB912_506
.LBB912_504:
	movq	168(%rsp), %rbx
	movq	136(%rsp), %rbp
	jmp	.LBB912_528
.LBB912_505:
	movq	992(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 192(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB912_506:
	cmpb	$-1, 176(%rsp)
	je	.LBB912_508
	movb	$1, %al
	jmp	.LBB912_514
.LBB912_508:
	testb	%r12b, %r12b
	jne	.LBB912_511
.Ltmp11030:
	leaq	384(%rsp), %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp11031:
	cmpb	$-1, 384(%rsp)
	movb	$1, %al
	movl	%eax, 32(%rsp)
	jne	.LBB912_516
.LBB912_511:
	movq	1264(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB912_515
.Ltmp11032:
	movq	128(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp11033:
	cmpb	$-1, 384(%rsp)
	setne	%al
.LBB912_514:
	movl	%eax, 32(%rsp)
	jmp	.LBB912_516
.LBB912_515:
	movl	$0, 32(%rsp)
.LBB912_516:
	movq	296(%rsp), %rax
	cmpq	%rax, 56(%rsp)
	jae	.LBB912_527
	movq	80(%rsp), %rcx
	cmpq	688(%rsp), %rcx
	je	.LBB912_523
	movq	296(%rsp), %rax
	leaq	-1(%rax), %rbx
.LBB912_519:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB912_525
	movq	(%rdx), %rsi
	movq	%rax, 384(%rsp)
	leaq	392(%rsp), %rcx
	movq	%rdx, %r15
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp11035:
	movq	216(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	384(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp11036:
	movq	56(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB912_524
	incq	%rax
	leaq	88(%r15), %rcx
	movq	%r15, %rdx
	movq	%rax, 56(%rsp)
	cmpq	688(%rsp), %rcx
	jne	.LBB912_519
	jmp	.LBB912_525
.LBB912_523:
	movq	160(%rsp), %rdx
	jmp	.LBB912_526
.LBB912_524:
	movq	296(%rsp), %rax
	movq	%r15, %rdx
	movq	%rax, 56(%rsp)
.LBB912_525:
	addq	$88, %rdx
	movq	%rdx, 80(%rsp)
.LBB912_526:
	movq	%rdx, 160(%rsp)
	movq	%rdx, 104(%rsp)
.LBB912_527:
	movq	168(%rsp), %rbx
	cmpb	$0, 32(%rsp)
	jne	.LBB912_808
.LBB912_528:
	cmpq	$0, 40(%rsp)
	je	.LBB912_373
	movq	248(%rsp), %r12
	movq	640(%rsp), %rdi
	jmp	.LBB912_531
	.p2align	4
.LBB912_530:
	movq	360(%rsp), %rax
	leaq	(%r13,%r13,4), %rcx
	movq	40(%rsp), %rsi
	incq	%r13
	leaq	40(%r14), %rdi
	movq	%rbx, (%rax,%rcx,8)
	movq	%r15, 8(%rax,%rcx,8)
	decq	%rsi
	vmovdqa	1184(%rsp), %xmm0
	movq	%rsi, 40(%rsp)
	vmovdqu	%xmm0, 16(%rax,%rcx,8)
	movq	1200(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r13, 368(%rsp)
	movq	64(%rsp), %r13
	testq	%rsi, %rsi
	je	.LBB912_535
.LBB912_531:
	movq	%rdi, %r14
	cmpq	%r12, %rdi
	je	.LBB912_536
	movq	(%r14), %rbx
	testq	%rbx, %rbx
	je	.LBB912_535
	movq	32(%r14), %rax
	movq	8(%r14), %r15
	movq	368(%rsp), %r13
	movq	%rax, 1200(%rsp)
	vmovdqu	16(%r14), %xmm0
	vmovdqa	%xmm0, 1184(%rsp)
	cmpq	352(%rsp), %r13
	jne	.LBB912_530
.Ltmp11067:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	352(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11068:
	jmp	.LBB912_530
.LBB912_535:
	addq	$40, %r14
.LBB912_536:
	movq	%r14, %rcx
	movq	48(%rsp), %r14
	movq	168(%rsp), %rbx
	jmp	.LBB912_374
.LBB912_537:
	testq	%r14, %r14
	je	.LBB912_539
	shlq	$5, %r14
	movl	$8, %edx
	movq	%rbx, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB912_539:
.Ltmp11077:
	movq	256(%rsp), %r14
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp11078:
	movq	24(%rsp), %rbx
	movq	984(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_542
	shlq	$3, %rax
	movl	$8, %edx
	movq	%r14, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB912_542:
	movq	808(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_545
	lock		decq	(%rax)
	jne	.LBB912_545
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB912_545:
	movq	840(%rsp), %rax
	leaq	728(%rsp), %r14
	testq	%rax, %rax
	je	.LBB912_548
	lock		decq	(%rax)
	jne	.LBB912_548
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	840(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB912_548:
	cmpq	1424(%rsp), %rbx
	jne	.LBB912_370
	jmp	.LBB912_550
.LBB912_549:
	movq	640(%rsp), %r12
.LBB912_550:
	movb	$1, %r15b
	movq	%r12, %rbp
	xorl	%r14d, %r14d
.Ltmp11082:
	leaq	320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp11083:
	cmpq	$-1, 632(%rsp)
	movzbl	79(%rsp), %ecx
	sete	%al
	xorb	$1, %cl
	orb	156(%rsp), %cl
	orb	%al, %cl
	cmpb	$1, %cl
	je	.LBB912_553
	movq	$1, 384(%rsp)
	xorl	%r14d, %r14d
	movq	$0, 392(%rsp)
.Ltmp11084:
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movq	128(%rsp), %rsi
	leaq	720(%rsp), %rdi
	leaq	384(%rsp), %rdx
	movl	$1, %ecx
	callq	*%rax
.Ltmp11085:
.LBB912_553:
	vmovdqa	352(%rsp), %xmm0
	movq	368(%rsp), %rax
	movq	%rax, 1088(%rsp)
	vmovdqa	%xmm0, 1072(%rsp)
	lock		decq	(%r13)
	jne	.LBB912_555
	#MEMBARRIER
.Ltmp11089:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1096(%rsp), %rdi
	callq	*%rax
.Ltmp11090:
.LBB912_555:
	movq	248(%rsp), %rax
	subq	%rbp, %rax
	je	.LBB912_568
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %r12
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %rbx
	movabsq	$9223372036854775807, %r15
	xorl	%r14d, %r14d
	imulq	%rax, %rbx
	jmp	.LBB912_560
.LBB912_557:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_558:
	callq	*%r12
.LBB912_559:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB912_568
.LBB912_560:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_559
	leaq	(%rbp,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r15, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_563
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_563:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_558
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_563
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
.LBB912_566:
	cmpq	%rax, %rdx
	jge	.LBB912_557
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB912_566
	jmp	.LBB912_557
.LBB912_568:
	movq	240(%rsp), %rax
	movb	$2, %r13b
	testq	%rax, %rax
	jne	.LBB912_599
	jmp	.LBB912_600
.LBB912_569:
	movb	$1, %al
	movl	%eax, 32(%rsp)
.LBB912_570:
	movq	256(%rsp), %rbx
	movq	48(%rsp), %r14
.LBB912_571:
	vmovdqa	352(%rsp), %xmm0
	movq	368(%rsp), %rax
	movq	%rax, 1088(%rsp)
	vmovdqa	%xmm0, 1072(%rsp)
	testq	%r14, %r14
	je	.LBB912_573
	movq	168(%rsp), %rdi
	shlq	$5, %r14
	movl	$8, %edx
	movq	%r14, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB912_573:
.Ltmp11057:
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp11058:
	movq	984(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_576
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB912_576:
	movq	808(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_579
	lock		decq	(%rax)
	jne	.LBB912_579
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB912_579:
	movq	840(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_582
	lock		decq	(%rax)
	jne	.LBB912_582
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	840(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB912_582:
	xorl	%r15d, %r15d
.Ltmp11060:
	leaq	320(%rsp), %rdi
	xorl	%r14d, %r14d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp11061:
	lock		decq	(%r13)
	jne	.LBB912_585
	#MEMBARRIER
.Ltmp11062:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1096(%rsp), %rdi
	callq	*%rax
.Ltmp11063:
.LBB912_585:
	movq	232(%rsp), %rbx
	movq	248(%rsp), %rax
	subq	%rbx, %rax
	je	.LBB912_598
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %r14
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	imulq	%rax, %r14
	jmp	.LBB912_590
.LBB912_587:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_588:
	callq	*%r13
.LBB912_589:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB912_598
.LBB912_590:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_589
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_593
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_593:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_588
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_593
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
.LBB912_596:
	cmpq	%rax, %rdx
	jge	.LBB912_587
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_596
	jmp	.LBB912_587
.LBB912_598:
	movq	240(%rsp), %rax
	movl	32(%rsp), %r13d
	testq	%rax, %rax
	je	.LBB912_600
.LBB912_599:
	movq	224(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB912_600:
	movq	16(%rsp), %r15
	movl	156(%rsp), %ebp
.LBB912_601:
	movq	1088(%rsp), %rax
	vmovdqa	1072(%rsp), %xmm0
	movq	%rax, 1376(%rsp)
	movq	%rax, 400(%rsp)
	movq	616(%r15), %rax
	vmovdqa	%xmm0, 1360(%rsp)
	vmovdqa	%xmm0, 384(%rsp)
	testq	%rax, %rax
	je	.LBB912_610
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	jne	.LBB912_604
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 1119(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 1104(%rsp)
.LBB912_604:
	testb	%r13b, %r13b
	je	.LBB912_609
	movzbl	%r13b, %eax
	cmpl	$2, %eax
	je	.LBB912_611
	cmpb	$-1, %bl
	je	.LBB912_788
	cmpb	$2, 472(%r15)
	jne	.LBB912_609
	vmovdqa	1104(%rsp), %xmm0
	movq	1119(%rsp), %rax
	movq	696(%r15), %rdi
	movb	%bl, 720(%rsp)
	vmovdqu	%xmm0, 721(%rsp)
	movq	%rax, 736(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	jne	.LBB912_822
.LBB912_609:
	xorl	%ebp, %ebp
	jmp	.LBB912_611
.LBB912_610:
	cmpb	$2, %r13b
	movb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, %ebp
.LBB912_611:
	cmpb	$-1, %bl
	vmovdqa	384(%rsp), %xmm0
	sete	%al
	andb	%bpl, %al
	movl	%eax, 24(%rsp)
	movq	400(%rsp), %rax
	vmovdqa	%xmm0, 1328(%rsp)
	movq	%rax, 1344(%rsp)
.LBB912_612:
	movq	1344(%rsp), %rax
	vmovdqa	1328(%rsp), %xmm0
	movq	1008(%rsp), %rcx
	movq	1024(%rsp), %rdx
	movq	1032(%rsp), %rsi
	movl	$1, %r12d
	movl	$1, %edi
	movq	%rax, 1488(%rsp)
	movq	%rax, 112(%rsp)
	movq	1016(%rsp), %rax
	cmpq	$3, %rcx
	movq	%rcx, %r15
	cmovaeq	%rdx, %r15
	cmovaeq	%rcx, %rdi
	cmovaeq	%r12, %rdx
	movq	%rdi, 720(%rsp)
	vmovdqa	%xmm0, 1472(%rsp)
	vmovdqa	%xmm0, 96(%rsp)
	movq	%rax, 728(%rsp)
	movq	%rdx, 736(%rsp)
	movq	%rsi, 744(%rsp)
	movq	%r15, %rsi
	decq	%rsi
	movq	$0, 752(%rsp)
	movq	%rsi, 760(%rsp)
	je	.LBB912_616
	cmpq	$3, %rcx
	movq	16(%rsp), %rcx
	movq	<purrdf_sparql_eval::witness::RelationWitness>::merge@GOTPCREL(%rip), %rbp
	leaq	728(%rsp), %r13
	leaq	384(%rsp), %r14
	cmovaeq	%rax, %r13
	leaq	640(%rcx), %rbx
	.p2align	4
.LBB912_614:
	movq	%r12, 752(%rsp)
	movq	16(%r13), %rax
	movq	%rax, 400(%rsp)
	vmovdqu	(%r13), %xmm0
	vmovdqa	%xmm0, 384(%rsp)
.Ltmp11115:
	movq	%rbx, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	*%rbp
.Ltmp11116:
	addq	$24, %r13
	incq	%r12
	cmpq	%r12, %r15
	jne	.LBB912_614
.LBB912_616:
.Ltmp11121:
	leaq	720(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp11122:
	movq	88(%rsp), %r14
	cmpb	$0, 24(%rsp)
	movq	1456(%rsp), %rdi
	je	.LBB912_630
	movq	672(%rsp), %rdx
	cmpq	%rdx, %rdi
	ja	.LBB912_818
	movq	16(%rsp), %rax
	movq	616(%rax), %r8
	testq	%r8, %r8
	je	.LBB912_624
	cmpq	$0, 336(%r8)
	leaq	880(%rsp), %rsi
	leaq	792(%rsp), %rcx
	leaq	808(%rsp), %rax
	jne	.LBB912_625
	cmpq	$-1, 16(%r8)
	jne	.LBB912_625
	cmpq	$-1, 40(%r8)
	jne	.LBB912_625
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rsi)
	movq	$0, 16(%rsi)
	movq	$-1, 904(%rsp)
	movl	$67108864, 912(%rsp)
	jmp	.LBB912_626
.LBB912_624:
	leaq	792(%rsp), %rcx
	leaq	808(%rsp), %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, 880(%rsp)
	movq	$0, 896(%rsp)
	movq	$-1, 904(%rsp)
	movl	$67108864, 912(%rsp)
	movq	$0, 720(%rsp)
	movq	$8, 728(%rsp)
	vmovdqu	%xmm0, 736(%rsp)
	movq	$8, 752(%rsp)
	vmovdqu	%xmm0, 760(%rsp)
	jmp	.LBB912_627
.LBB912_625:
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rsi)
	movq	$0, 16(%rsi)
	movq	$-1, 904(%rsp)
	movl	$67174400, 912(%rsp)
.LBB912_626:
	movq	$0, 720(%rsp)
	movq	$8, 728(%rsp)
	vmovdqu	%xmm0, -144(%rsi)
	movq	$8, 752(%rsp)
	vmovdqu	%xmm0, -120(%rsi)
.LBB912_627:
	movq	$8, 776(%rsp)
	movq	$0, 784(%rsp)
	movq	664(%rsp), %rsi
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rcx)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, (%rax)
	movb	$0, 64(%rax)
	cmpq	%rdx, %rdi
	jne	.LBB912_705
.LBB912_629:
.Ltmp11140:
	leaq	720(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11141:
.LBB912_630:
	movq	112(%rsp), %rax
	vmovdqa	96(%rsp), %xmm0
	movq	%rax, 1232(%rsp)
	movq	304(%rsp), %rax
	vmovdqa	%xmm0, 1216(%rsp)
	testq	%rax, %rax
	je	.LBB912_633
	lock		decq	(%rax)
	jne	.LBB912_633
	#MEMBARRIER
.Ltmp11142:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	callq	*%rax
.Ltmp11143:
.LBB912_633:
	movq	1272(%rsp), %r14
	movq	16(%rsp), %r15
	movb	$1, %bpl
	movq	696(%r15), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	jne	.LBB912_650
.LBB912_634:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB912_650
	movb	%cl, 384(%rsp)
	movq	144(%rsp), %rcx
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 385(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 400(%rsp)
	lock		incq	(%rcx)
	movq	88(%rsp), %rbx
	jle	.LBB912_819
	movq	144(%rsp), %rcx
.Ltmp11144:
	leaq	720(%rsp), %rdi
	leaq	384(%rsp), %rdx
	movq	%r14, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp11145:
	vmovdqu64	752(%rsp), %zmm1
	vmovdqu64	720(%rsp), %zmm0
	movq	1232(%rsp), %r14
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	movq	1224(%rsp), %rbx
	testq	%r14, %r14
	je	.LBB912_695
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB912_642
	.p2align	4
.LBB912_639:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_640:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB912_641:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB912_695
.LBB912_642:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_641
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_645
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_645:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_640
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_645
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
.LBB912_648:
	cmpq	%rax, %rdx
	jge	.LBB912_639
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB912_648
	jmp	.LBB912_639
.LBB912_650:
	vmovups	1776(%rsp), %zmm1
	vmovdqu64	1736(%rsp), %zmm0
	movq	1232(%rsp), %rcx
	movq	144(%rsp), %rax
	movq	88(%rsp), %rbx
	movq	%rcx, 1536(%rsp)
	vmovups	%zmm1, 760(%rsp)
	vmovdqa	1216(%rsp), %xmm1
	vmovdqu64	%zmm0, 720(%rsp)
	cmpq	$-1, 720(%rsp)
	vmovdqa	%xmm1, 1520(%rsp)
	movq	%rax, 1544(%rsp)
	je	.LBB912_653
	leaq	384(%rsp), %rdi
	leaq	1520(%rsp), %rsi
	leaq	1736(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	792(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB912_652
.LBB912_654:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	800(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_656
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_656:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_662
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_656
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
.LBB912_659:
	cmpq	%rax, %rsi
	jge	.LBB912_661
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB912_659
.LBB912_661:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_662:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	816(%rsp), %rax
	movl	%ebp, %r15d
	testq	%rax, %rax
	jne	.LBB912_663
	jmp	.LBB912_665
.LBB912_653:
	vmovdqu	1520(%rsp), %xmm0
	movq	1536(%rsp), %rax
	movq	1544(%rsp), %rcx
	movq	%rax, 408(%rsp)
	movq	%rcx, 416(%rsp)
	vmovdqu	%xmm0, 392(%rsp)
	movq	$-1, 384(%rsp)
	movq	792(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB912_654
.LBB912_652:
	movq	816(%rsp), %rax
	movl	%ebp, %r15d
	testq	%rax, %rax
	je	.LBB912_665
.LBB912_663:
	lock		decq	(%rax)
	jne	.LBB912_665
	leaq	816(%rsp), %rdi
	#MEMBARRIER
.Ltmp11147:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp11148:
.LBB912_665:
	vmovdqu64	416(%rsp), %zmm1
	vmovdqu64	384(%rsp), %zmm0
	movl	$0, 316(%rsp)
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.Ltmp11150:
	leaq	2160(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp11151:
.Ltmp11152:
	leaq	1960(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11153:
	movq	680(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB912_669
	#MEMBARRIER
.Ltmp11155:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	680(%rsp), %rdi
	callq	*%rax
.Ltmp11156:
.LBB912_669:
	testb	%r15b, %r15b
	movq	88(%rsp), %r15
	je	.LBB912_694
	movq	664(%rsp), %rbx
	movq	672(%rsp), %r14
	testq	%r14, %r14
	je	.LBB912_683
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB912_675
	.p2align	4
.LBB912_672:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_673:
	callq	*%r13
.LBB912_674:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB912_683
.LBB912_675:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_674
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_678
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_678:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_673
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_678
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
.LBB912_681:
	cmpq	%rax, %rdx
	jge	.LBB912_672
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_681
	jmp	.LBB912_672
.LBB912_683:
	movq	656(%rsp), %rax
	movq	88(%rsp), %r15
	testq	%rax, %rax
	je	.LBB912_694
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_686
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_686:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_692
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_686
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
.LBB912_689:
	cmpq	%rax, %rsi
	jge	.LBB912_691
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB912_689
.LBB912_691:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_692:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	jmp	.LBB912_693
.LBB912_694:
	movq	%r15, %rax
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
.LBB912_695:
	.cfi_def_cfa_offset 2432
	movq	1216(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_754
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_698
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_698:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_704
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_698
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
.LBB912_701:
	cmpq	%rax, %rsi
	jge	.LBB912_703
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB912_701
.LBB912_703:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_704:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	movl	%ebp, %r15d
	vzeroupper
	callq	*%rax
	jmp	.LBB912_755
.LBB912_705:
	leaq	392(%rsp), %rcx
	leaq	(%rdx,%rdx,4), %rax
	leaq	384(%rsp), %r13
	leaq	720(%rsp), %rbp
	vpbroadcastq	%rcx, %ymm0
	vpaddq	.LCPI912_5(%rip), %ymm0, %ymm1
	vpaddq	.LCPI912_6(%rip), %ymm0, %ymm0
	leaq	(%rsi,%rax,8), %r12
	leaq	(%rdi,%rdi,4), %rax
	leaq	(%rsi,%rax,8), %r15
	movabsq	$2305843009213693920, %rax
	movq	%r12, 24(%rsp)
	addq	$31, %rax
	movq	%rax, 40(%rsp)
	vmovdqu	%ymm1, 256(%rsp)
	vmovdqu	%ymm0, 688(%rsp)
	jmp	.LBB912_708
.LBB912_706:
	movq	104(%rsp), %rax
	leaq	(%rbx,%rbx,4), %rcx
	incq	%rbx
	movq	%r13, (%rax,%rcx,8)
	movq	%r14, 8(%rax,%rcx,8)
	movq	%r12, 16(%rax,%rcx,8)
	movq	88(%rsp), %r14
	movq	24(%rsp), %r12
	leaq	384(%rsp), %r13
	vmovdqa	1104(%rsp), %xmm0
	vmovdqu	%xmm0, 24(%rax,%rcx,8)
	movq	%rbx, 112(%rsp)
.LBB912_707:
	addq	$40, %r15
	cmpq	%r12, %r15
	je	.LBB912_629
.LBB912_708:
.Ltmp11123:
	movq	16(%rsp), %rdx
	movq	%r13, %rdi
	movq	%rbp, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11124:
	cmpb	$-1, 384(%rsp)
	jne	.LBB912_629
	movq	(%r15), %rcx
	leaq	8(%r15), %rbx
	movq	%rbx, %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB912_712
	movq	16(%r15), %rcx
	movq	8(%r15), %rdx
	decq	%rcx
.LBB912_712:
	movq	144(%rsp), %r8
	addq	$16, %r8
.Ltmp11125:
	movq	16(%rsp), %r9
	leaq	2160(%rsp), %rsi
	movq	%r13, %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11126:
	movq	384(%rsp), %rax
	movl	392(%rsp), %edx
	movl	396(%rsp), %ecx
	cmpq	$-1, %rax
	jne	.LBB912_787
	cmpl	$2, %edx
	je	.LBB912_707
.Ltmp11127:
	movq	16(%rsp), %rsi
	movq	%r13, %rdi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp11128:
	movq	384(%rsp), %rax
	movzbl	392(%rsp), %edx
	cmpq	$-1, %rax
	jne	.LBB912_789
	testb	$1, %dl
	je	.LBB912_707
	movq	(%r15), %rax
	movq	16(%r15), %r13
	leaq	-1(%rax), %rbp
	decq	%r13
	cmpq	$5, %rbp
	cmovbq	%rbp, %r13
	cmpq	$4, %r13
	jbe	.LBB912_725
	cmpq	40(%rsp), %r13
	leaq	(,%r13,8), %r12
	movabsq	$9223372036854775804, %rcx
	seta	%al
	cmpq	%rcx, %r12
	seta	%cl
	orb	%al, %cl
	jne	.LBB912_785
	movl	$4, %esi
	movq	%r12, %rdi
	callq	__rustc::__rust_alloc
	movl	$4, %edi
	testq	%rax, %rax
	je	.LBB912_786
	movq	%rax, %r14
	cmpq	$5, %rbp
	jb	.LBB912_723
	movq	(%rbx), %rbx
.LBB912_723:
	leaq	720(%rsp), %rbp
	cmpq	$8, %r13
	jb	.LBB912_724
	leaq	(%rbx,%r12), %rax
	cmpq	%rax, %r14
	jae	.LBB912_734
	movq	%r14, %rax
	addq	%r12, %rax
	cmpq	%rax, %rbx
	jae	.LBB912_734
.LBB912_724:
	xorl	%eax, %eax
.LBB912_744:
	movq	%r13, %rdx
	andq	$7, %rdx
	movq	%rax, %rcx
	je	.LBB912_748
	movq	%rax, %rcx
.LBB912_746:
	movl	(%rbx,%rcx,8), %esi
	movl	4(%rbx,%rcx,8), %edi
	movl	%esi, (%r14,%rcx,8)
	movl	%edi, 4(%r14,%rcx,8)
	incq	%rcx
	decq	%rdx
	jne	.LBB912_746
	leaq	-1(%rcx), %r12
.LBB912_748:
	subq	%r13, %rax
	cmpq	$-8, %rax
	ja	.LBB912_751
	movq	%r13, %rax
	decq	%rcx
	negq	%rax
	movq	%rcx, %r12
.LBB912_750:
	movl	8(%rbx,%r12,8), %ecx
	movl	12(%rbx,%r12,8), %edx
	movl	%ecx, 8(%r14,%r12,8)
	movl	%edx, 12(%r14,%r12,8)
	movl	16(%rbx,%r12,8), %ecx
	movl	20(%rbx,%r12,8), %edx
	movl	%ecx, 16(%r14,%r12,8)
	movl	%edx, 20(%r14,%r12,8)
	movl	24(%rbx,%r12,8), %ecx
	movl	28(%rbx,%r12,8), %edx
	movl	%ecx, 24(%r14,%r12,8)
	movl	%edx, 28(%r14,%r12,8)
	movl	32(%rbx,%r12,8), %ecx
	movl	36(%rbx,%r12,8), %edx
	movl	%ecx, 32(%r14,%r12,8)
	movl	%edx, 36(%r14,%r12,8)
	movl	40(%rbx,%r12,8), %ecx
	movl	44(%rbx,%r12,8), %edx
	movl	%ecx, 40(%r14,%r12,8)
	movl	%edx, 44(%r14,%r12,8)
	movl	48(%rbx,%r12,8), %ecx
	movl	52(%rbx,%r12,8), %edx
	movl	%ecx, 48(%r14,%r12,8)
	movl	%edx, 52(%r14,%r12,8)
	movl	56(%rbx,%r12,8), %ecx
	movl	60(%rbx,%r12,8), %edx
	movl	%ecx, 56(%r14,%r12,8)
	movl	%edx, 60(%r14,%r12,8)
	movl	64(%rbx,%r12,8), %ecx
	movl	68(%rbx,%r12,8), %edx
	movl	%ecx, 64(%r14,%r12,8)
	movl	%edx, 68(%r14,%r12,8)
	leaq	8(%rax,%r12), %rdx
	addq	$8, %r12
	cmpq	$-1, %rdx
	jne	.LBB912_750
	jmp	.LBB912_751
.LBB912_725:
	cmpq	$6, %rax
	jb	.LBB912_727
	movq	(%rbx), %rbx
.LBB912_727:
	leaq	720(%rsp), %rbp
	testq	%r13, %r13
	je	.LBB912_729
	leaq	-1(%r13), %rax
	vpmovsxbd	.LCPI912_9(%rip), %xmm2
	vmovdqu	256(%rsp), %ymm3
	vpxor	%xmm1, %xmm1, %xmm1
	incq	%r13
	vpbroadcastq	%rax, %ymm0
	vpcmpnltuq	.LCPI912_7(%rip), %ymm0, %k1
	vpxor	%xmm0, %xmm0, %xmm0
	kmovq	%k1, %k2
	vpgatherdd	(%rbx,%xmm2), %xmm0 {%k2}
	kmovq	%k1, %k2
	vpgatherdd	4(%rbx,%xmm2), %xmm1 {%k2}
	kmovq	%k1, %k2
	vpscatterqd	%xmm0, (,%ymm3) {%k2}
	vmovdqu	688(%rsp), %ymm0
	vpscatterqd	%xmm1, (,%ymm0) {%k1}
	jmp	.LBB912_730
.LBB912_729:
	movl	$1, %r13d
.LBB912_730:
	movq	%r13, 384(%rsp)
	leaq	392(%rsp), %rax
	vmovdqu	16(%rax), %xmm0
	movq	392(%rsp), %r14
	movq	400(%rsp), %r12
	vmovdqa	%xmm0, 1104(%rsp)
	jmp	.LBB912_752
.LBB912_734:
	cmpq	$32, %r13
	jae	.LBB912_736
	xorl	%eax, %eax
	jmp	.LBB912_740
.LBB912_736:
	movabsq	$2305843009213693920, %rcx
	movq	%r13, %rax
	andq	%rcx, %rax
	xorl	%ecx, %ecx
.LBB912_737:
	vmovdqu64	(%rbx,%rcx,8), %zmm0
	vmovdqu64	64(%rbx,%rcx,8), %zmm1
	vmovdqu64	128(%rbx,%rcx,8), %zmm2
	vmovdqu64	192(%rbx,%rcx,8), %zmm3
	vmovdqu64	%zmm0, (%r14,%rcx,8)
	vmovdqu64	%zmm1, 64(%r14,%rcx,8)
	vmovdqu64	%zmm2, 128(%r14,%rcx,8)
	vmovdqu64	%zmm3, 192(%r14,%rcx,8)
	addq	$32, %rcx
	cmpq	%rcx, %rax
	jne	.LBB912_737
	cmpq	%rax, %r13
	je	.LBB912_743
	testb	$24, %r13b
	je	.LBB912_744
.LBB912_740:
	movq	%rax, %rcx
	movabsq	$2305843009213693920, %rax
	addq	$24, %rax
	andq	%r13, %rax
.LBB912_741:
	vmovdqu64	(%rbx,%rcx,8), %zmm0
	vmovdqu64	%zmm0, (%r14,%rcx,8)
	addq	$8, %rcx
	cmpq	%rcx, %rax
	jne	.LBB912_741
	cmpq	%rax, %r13
	jne	.LBB912_744
.LBB912_743:
	decq	%rax
	movq	%rax, %r12
.LBB912_751:
	addq	$2, %r12
	incq	%r13
.LBB912_752:
	movq	112(%rsp), %rbx
	cmpq	96(%rsp), %rbx
	jne	.LBB912_706
.Ltmp11132:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp11133:
	jmp	.LBB912_706
.LBB912_754:
	movl	%ebp, %r15d
.LBB912_755:
.Ltmp11168:
	leaq	2160(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp11169:
.Ltmp11173:
	leaq	1960(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11174:
	movq	144(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB912_759
	#MEMBARRIER
.Ltmp11178:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.Ltmp11179:
.LBB912_759:
	movq	680(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB912_761
	#MEMBARRIER
.Ltmp11183:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	680(%rsp), %rdi
	callq	*%rax
.Ltmp11184:
.LBB912_761:
	testb	%r15b, %r15b
	movq	88(%rsp), %r15
	je	.LBB912_3
	movq	664(%rsp), %rbx
	movq	672(%rsp), %r14
	testq	%r14, %r14
	je	.LBB912_775
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB912_767
	.p2align	4
.LBB912_764:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_765:
	callq	*%r13
.LBB912_766:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB912_775
.LBB912_767:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB912_766
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_770
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_770:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_765
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_770
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
.LBB912_773:
	cmpq	%rax, %rdx
	jge	.LBB912_764
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB912_773
	jmp	.LBB912_764
.LBB912_775:
	movq	656(%rsp), %rax
	movq	88(%rsp), %r15
	testq	%rax, %rax
	je	.LBB912_3
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB912_778
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB912_778:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB912_784
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB912_778
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
.LBB912_781:
	cmpq	%rax, %rsi
	jge	.LBB912_783
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB912_781
.LBB912_783:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB912_784:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB912_3
.LBB912_785:
	xorl	%edi, %edi
.LBB912_786:
.Ltmp11135:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r12, %rsi
	callq	*%rax
.Ltmp11136:
	jmp	.LBB912_819
.LBB912_787:
	vmovups	416(%rsp), %zmm1
	vmovups	400(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 1120(%rsp)
	vmovups	%zmm0, 1104(%rsp)
	jmp	.LBB912_790
.LBB912_788:
	movb	$-1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB912_611
.LBB912_789:
	movzbl	395(%rsp), %ecx
	movzwl	393(%rsp), %esi
	vmovups	400(%rsp), %zmm0
	vmovups	416(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	396(%rsp), %ecx
	vmovups	%zmm0, 1104(%rsp)
	vmovups	%zmm1, 1120(%rsp)
.LBB912_790:
	vmovups	1104(%rsp), %zmm0
	vmovups	1120(%rsp), %zmm1
	movw	%si, 25(%r14)
	shrl	$16, %esi
	movb	%sil, 27(%r14)
	movl	%ecx, 28(%r14)
	vmovups	%zmm0, 32(%r14)
	vmovups	%zmm1, 48(%r14)
	movq	%rax, 16(%r14)
	movb	%dl, 24(%r14)
	movq	$1, (%r14)
.Ltmp11130:
	leaq	720(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11131:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	304(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB912_279
	jmp	.LBB912_281
.LBB912_792:
	movq	64(%rsp), %r13
	movq	%r12, %rax
	addq	$-1, %rax
	movb	$1, %cl
	movl	%ecx, 32(%rsp)
	jae	.LBB912_570
	movq	%r14, %rcx
	movq	256(%rsp), %rbx
	cmpq	%rax, %r14
	movq	48(%rsp), %r14
	jae	.LBB912_571
	movq	80(%rsp), %rax
	movq	160(%rsp), %rdx
	cmpq	688(%rsp), %rax
	je	.LBB912_811
	subq	%rcx, %r12
	addq	$88, %rax
	leaq	384(%rsp), %r15
	addq	$-2, %r12
.LBB912_796:
	movq	%rax, %rdx
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB912_811
	movq	-88(%rdx), %rsi
	movq	%rax, 384(%rsp)
	leaq	392(%rsp), %rcx
	movq	%rdx, %rbp
	movq	-8(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp11054:
	movq	216(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r15, %rdx
	vzeroupper
	callq	*%rax
.Ltmp11055:
	subq	$1, %r12
	jb	.LBB912_810
	leaq	88(%rbp), %rax
	movq	%rbp, %rdx
	cmpq	688(%rsp), %rbp
	jne	.LBB912_796
	jmp	.LBB912_811
.LBB912_800:
	movb	$1, %cl
	leaq	-1(%r12), %rax
	movq	256(%rsp), %rbx
	movl	%ecx, 32(%rsp)
	movq	%r14, %rcx
	cmpq	%rax, %r14
	movq	48(%rsp), %r14
	jae	.LBB912_571
	movq	80(%rsp), %rax
	movq	160(%rsp), %rdx
	cmpq	688(%rsp), %rax
	je	.LBB912_811
	subq	%rcx, %r12
	addq	$88, %rax
	leaq	384(%rsp), %r15
	addq	$-2, %r12
.LBB912_803:
	movq	%rax, %rdx
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB912_811
	movq	-88(%rdx), %rsi
	movq	%rax, 384(%rsp)
	leaq	392(%rsp), %rcx
	movq	%rdx, %rbp
	movq	-8(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp11048:
	movq	216(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r15, %rdx
	vzeroupper
	callq	*%rax
.Ltmp11049:
	subq	$1, %r12
	jb	.LBB912_810
	leaq	88(%rbp), %rax
	movq	%rbp, %rdx
	cmpq	688(%rsp), %rbp
	jne	.LBB912_803
	jmp	.LBB912_811
.LBB912_807:
	movl	$0, 32(%rsp)
	jmp	.LBB912_570
.LBB912_808:
	movq	256(%rsp), %rbx
	movb	$1, %al
	movl	%eax, 32(%rsp)
	jmp	.LBB912_571
.LBB912_810:
	movq	%rbp, %rdx
.LBB912_811:
	movb	$1, %al
	movq	%rdx, 104(%rsp)
	movl	%eax, 32(%rsp)
	jmp	.LBB912_571
.LBB912_812:
.Ltmp11070:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	movq	1248(%rsp), %rdx
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.301(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp11071:
	jmp	.LBB912_819
.LBB912_813:
	leaq	1464(%rsp), %rcx
	movq	%rax, 176(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rdi, 1464(%rsp)
	movq	%rcx, 384(%rsp)
	leaq	176(%rsp), %rcx
	movq	%rax, 392(%rsp)
	movq	%rcx, 400(%rsp)
	movq	%rax, 408(%rsp)
.Ltmp11072:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.2054(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.300(%rip), %rdx
	leaq	384(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp11073:
	jmp	.LBB912_819
.LBB912_814:
.Ltmp10932:
	leaq	1008(%rsp), %rdi
	movl	$16, %ecx
	movl	$256, %r8d
	xorl	%esi, %esi
	movq	%r14, %rdx
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp10933:
	movq	1008(%rsp), %rax
	movq	1024(%rsp), %r12
	subq	%r12, %rax
	cmpq	%r14, %rax
	jae	.LBB912_133
.LBB912_816:
.Ltmp10946:
	movq	core::panicking::panic@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.965(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.967(%rip), %rdx
	movl	$47, %esi
	callq	*%rax
.Ltmp10947:
	jmp	.LBB912_819
.LBB912_817:
	leaq	1392(%rsp), %rax
	movq	%rax, 384(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 392(%rsp)
	movq	%rbx, 400(%rsp)
	movq	%rax, 408(%rsp)
.Ltmp10938:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.550(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.552(%rip), %rdx
	leaq	384(%rsp), %rsi
	callq	*%rax
.Ltmp10939:
	jmp	.LBB912_819
.LBB912_818:
.Ltmp11158:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.391(%rip), %rcx
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp11159:
.LBB912_819:
	ud2
.LBB912_820:
	movb	$1, %r15b
.Ltmp11012:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %r14b
	movq	%rbp, %rdi
	movq	%rbx, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp11013:
	jmp	.LBB912_367
.LBB912_821:
	movq	16(%rsp), %rax
	movb	$1, %r15b
	leaq	1000(%rax), %rdi
.Ltmp11014:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movb	$1, %r14b
	movq	%rbx, %rsi
	movq	%rbp, %rdx
	vzeroupper
	callq	*%rax
.Ltmp11015:
	jmp	.LBB912_368
.LBB912_822:
	addq	$16, %rdi
.Ltmp11107:
	leaq	720(%rsp), %rsi
	vzeroupper
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.13412714042204560522)
.Ltmp11108:
	jmp	.LBB912_609
.LBB912_823:
.Ltmp11109:
	leaq	384(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_933
.LBB912_824:
.Ltmp11050:
	jmp	.LBB912_826
.LBB912_825:
.Ltmp11056:
.LBB912_826:
	movq	%rax, %r13
	movq	%rbp, 104(%rsp)
	jmp	.LBB912_914
.LBB912_827:
.Ltmp11027:
	jmp	.LBB912_913
.LBB912_828:
.Ltmp11091:
	leaq	224(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_933
.LBB912_829:
.Ltmp11079:
	movq	%rax, %r13
	movb	$1, %r15b
	jmp	.LBB912_917
.LBB912_830:
.Ltmp11059:
	movq	%rax, %r13
	xorl	%r15d, %r15d
	jmp	.LBB912_917
.LBB912_831:
.Ltmp11034:
	jmp	.LBB912_913
.LBB912_832:
.Ltmp11134:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB912_834
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB912_834:
	movq	%rbx, %r13
	jmp	.LBB912_937
.LBB912_835:
.Ltmp11018:
	jmp	.LBB912_913
.LBB912_836:
.Ltmp11086:
	movq	%rax, %r13
	jmp	.LBB912_927
.LBB912_837:
.Ltmp11024:
	addq	$88, %r12
	movq	%rax, %r13
	movq	%r12, 104(%rsp)
	jmp	.LBB912_914
.LBB912_838:
.Ltmp10960:
	movq	%rax, %r13
.Ltmp10961:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>>
.Ltmp10962:
	jmp	.LBB912_941
.LBB912_839:
.Ltmp10963:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_840:
.Ltmp11106:
	jmp	.LBB912_875
.LBB912_841:
.Ltmp11053:
	jmp	.LBB912_913
.LBB912_842:
.Ltmp11094:
	movq	%rax, %r13
	movq	%rbx, 728(%rsp)
	cmpq	$6, %rbp
	jb	.LBB912_844
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	movq	%r15, %rdi
	callq	__rustc::__rust_dealloc
.LBB912_844:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_849
.LBB912_845:
.Ltmp11037:
	jmp	.LBB912_862
.LBB912_846:
.Ltmp10982:
	jmp	.LBB912_940
.LBB912_847:
.Ltmp11146:
	leaq	1216(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_947
.LBB912_848:
.Ltmp11011:
	leaq	1304(%rsp), %rcx
	movq	%rax, %r13
	movq	%rcx, 256(%rsp)
.LBB912_849:
	movq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_932
.LBB912_850:
.Ltmp11005:
	movq	%rax, %r13
	jmp	.LBB912_870
.LBB912_851:
.Ltmp11112:
	movq	%rax, %r13
	jmp	.LBB912_933
.LBB912_852:
.Ltmp10994:
	movq	%rax, %r13
	jmp	.LBB912_871
.LBB912_853:
.Ltmp11149:
	movq	%rax, %r13
	xorl	%ebx, %ebx
	movl	%r15d, %ebp
	jmp	.LBB912_948
.LBB912_854:
.Ltmp11157:
	movq	%rax, %r13
	testb	%r15b, %r15b
	je	.LBB912_960
	leaq	656(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%r13, %rdi
	callq	_Unwind_Resume@PLT
.LBB912_856:
.Ltmp11185:
	movq	%rax, %r13
	testb	%r15b, %r15b
	je	.LBB912_959
	leaq	656(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_959
.LBB912_858:
.Ltmp11180:
	movq	%rax, %r13
	movb	$1, %bl
	jmp	.LBB912_867
.LBB912_859:
.Ltmp11047:
	jmp	.LBB912_913
.LBB912_860:
.Ltmp10927:
	movq	%rax, %r13
	movq	%r12, 104(%rsp)
	cmpq	$5, %r15
	ja	.LBB912_900
	jmp	.LBB912_901
.LBB912_861:
.Ltmp11066:
.LBB912_862:
	addq	$88, %r15
	movq	%rax, %r13
	movq	%r15, 104(%rsp)
	jmp	.LBB912_914
.LBB912_863:
.Ltmp10991:
	leaq	1304(%rsp), %rdi
	movq	%rax, %r13
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_943
.LBB912_864:
.Ltmp11069:
	addq	$40, %r14
	movq	%rax, %r13
	movq	%r14, 232(%rsp)
	cmpq	$6, %rbx
	jb	.LBB912_914
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r15, %rdi
	callq	__rustc::__rust_dealloc
	jmp	.LBB912_914
.LBB912_866:
.Ltmp11154:
	movq	%rax, %r13
	xorl	%ebx, %ebx
.LBB912_867:
	movl	%r15d, %ebp
	jmp	.LBB912_954
.LBB912_868:
.Ltmp10997:
	movq	%rax, %r13
	movq	%r14, 392(%rsp)
.Ltmp10998:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp10999:
.Ltmp11001:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp11002:
.LBB912_870:
.Ltmp11006:
	leaq	1104(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp11007:
.LBB912_871:
	leaq	1280(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_933
.LBB912_872:
.Ltmp11000:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_873:
.Ltmp11008:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_874:
.Ltmp10988:
.LBB912_875:
	movq	%rax, %r13
	jmp	.LBB912_943
.LBB912_876:
.Ltmp10966:
	movq	%rax, %rbx
	testq	%r13, %r13
	je	.LBB912_903
	negq	%r13
	jmp	.LBB912_879
.LBB912_878:
	addq	$272, %r14
	decq	%r13
	je	.LBB912_903
.LBB912_879:
	cmpl	$2, (%r14)
	je	.LBB912_878
.Ltmp10967:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
.Ltmp10968:
	jmp	.LBB912_878
.LBB912_881:
.Ltmp10969:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_882:
.Ltmp11165:
	movq	%rax, %r13
	movb	$1, %bpl
	jmp	.LBB912_947
.LBB912_883:
.Ltmp11129:
	jmp	.LBB912_936
.LBB912_884:
.Ltmp11042:
	addq	$88, %rbp
	movq	%rax, %r13
	movq	%rbp, 104(%rsp)
	jmp	.LBB912_914
.LBB912_885:
.Ltmp11021:
	jmp	.LBB912_913
.LBB912_886:
.Ltmp11100:
	movq	%rax, %r13
	testq	%rbx, %rbx
	je	.LBB912_890
	negq	%rbx
	addq	$160, %r14
.LBB912_888:
.Ltmp11101:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11102:
	addq	$160, %r14
	decq	%rbx
	jne	.LBB912_888
.LBB912_890:
	cmpq	$0, 40(%rsp)
	je	.LBB912_933
	movq	40(%rsp), %rax
	movq	24(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB912_933
.LBB912_892:
.Ltmp11103:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_893:
.Ltmp11175:
	movq	%rax, %r13
	movl	%r15d, %ebp
	jmp	.LBB912_951
.LBB912_894:
.Ltmp11170:
	movl	316(%rsp), %ebx
	movq	%rax, %r13
	movl	%r15d, %ebp
	jmp	.LBB912_949
.LBB912_895:
.Ltmp11117:
	movq	%rax, %r13
.Ltmp11118:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp11119:
	jmp	.LBB912_938
.LBB912_896:
.Ltmp11120:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_897:
.Ltmp10909:
	movq	%rax, %r13
	jmp	.LBB912_959
.LBB912_898:
.Ltmp10924:
	movq	%rax, %r13
	movq	%r12, 104(%rsp)
	cmpq	$6, %r15
	jb	.LBB912_901
	movq	1528(%rsp), %r14
.LBB912_900:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB912_901:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB912_948
.LBB912_902:
.Ltmp10972:
	movq	%rax, %rbx
.LBB912_903:
.Ltmp10973:
	leaq	1104(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp10974:
.Ltmp10975:
	leaq	384(%rsp), %rdi
	movq	%rbx, %r13
	callq	core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>::{closure#2}>>
.Ltmp10976:
	jmp	.LBB912_941
.LBB912_905:
.Ltmp10977:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_906:
.Ltmp11160:
	movq	%rax, %r13
	jmp	.LBB912_938
.LBB912_907:
.Ltmp10940:
	movq	1040(%rsp), %rdi
	movq	%rax, %r13
.Ltmp10941:
	movq	%r15, %rsi
	callq	core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp10942:
	jmp	.LBB912_910
.LBB912_908:
.Ltmp10943:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_909:
.Ltmp10948:
	movq	%rax, %r13
.LBB912_910:
.Ltmp10949:
	leaq	1008(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
.Ltmp10950:
	jmp	.LBB912_943
.LBB912_911:
.Ltmp10951:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_912:
.Ltmp11074:
.LBB912_913:
	movq	%rax, %r13
.LBB912_914:
	cmpq	$0, 48(%rsp)
	je	.LBB912_916
	movq	48(%rsp), %rsi
	movq	168(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB912_916:
	movb	$1, %r15b
.Ltmp11075:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp11076:
.LBB912_917:
	cmpq	$0, 984(%rsp)
	je	.LBB912_919
	movq	984(%rsp), %rax
	movq	256(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB912_919:
	movq	808(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_922
	lock		decq	(%rax)
	jne	.LBB912_922
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB912_922:
	movq	840(%rsp), %rax
	testq	%rax, %rax
	je	.LBB912_925
	lock		decq	(%rax)
	jne	.LBB912_925
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	840(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB912_925:
.Ltmp11080:
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp11081:
	xorl	%r14d, %r14d
.LBB912_927:
	movq	64(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB912_929
	#MEMBARRIER
.Ltmp11087:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1096(%rsp), %rdi
	callq	*%rax
.Ltmp11088:
.LBB912_929:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	testb	%r15b, %r15b
	je	.LBB912_931
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB912_931:
	testb	%r14b, %r14b
	je	.LBB912_933
.LBB912_932:
.Ltmp11095:
	leaq	1040(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp11096:
.LBB912_933:
.Ltmp11113:
	leaq	1008(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp11114:
	jmp	.LBB912_943
.LBB912_934:
.Ltmp11097:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB912_935:
.Ltmp11137:
.LBB912_936:
	movq	%rax, %r13
.LBB912_937:
.Ltmp11138:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11139:
.LBB912_938:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB912_943
.LBB912_939:
.Ltmp10985:
.LBB912_940:
	movq	%rax, %r13
.LBB912_941:
	movq	1040(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB912_943
	movq	1048(%rsp), %rdi
	shlq	$4, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB912_943:
	movq	304(%rsp), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	je	.LBB912_947
	lock		decq	(%rax)
	movb	$1, %bpl
	jne	.LBB912_947
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp11161:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	callq	*%rax
.Ltmp11162:
	movb	$1, %bl
	jmp	.LBB912_948
.LBB912_947:
	movb	$1, %bl
.LBB912_948:
.Ltmp11166:
	leaq	2160(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
.Ltmp11167:
.LBB912_949:
.Ltmp11171:
	leaq	1960(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp11172:
	testb	%bl, %bl
	je	.LBB912_953
.LBB912_951:
	movq	144(%rsp), %rax
	movb	$1, %bl
	lock		decq	(%rax)
	jne	.LBB912_954
	#MEMBARRIER
.Ltmp11176:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.Ltmp11177:
	jmp	.LBB912_954
.LBB912_953:
	xorl	%ebx, %ebx
.LBB912_954:
	movq	680(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB912_956
	#MEMBARRIER
.Ltmp11181:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	680(%rsp), %rdi
	callq	*%rax
.Ltmp11182:
.LBB912_956:
	testb	%bpl, %bpl
	je	.LBB912_958
	leaq	656(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB912_958:
	testb	%bl, %bl
	je	.LBB912_960
.LBB912_959:
.Ltmp11186:
	leaq	1736(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp11187:
.LBB912_960:
	movq	%r13, %rdi
	callq	_Unwind_Resume@PLT
.LBB912_961:
.Ltmp11188:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end912:
purrdf_sparql_eval::binop::eval_lateral::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin968:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception629
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
	subq	$1544, %rsp
	.cfi_def_cfa_offset 1600
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, 56(%rsp)
	leaq	1104(%rsp), %rdi
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbx
	callq	*%rax
	movb	$1, %bpl
.Ltmp12916:
	leaq	1424(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r15, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp12917:
	cmpl	$1, 1424(%rsp)
	jne	.LBB968_3
	vmovdqu64	1440(%rsp), %zmm0
	vmovdqu64	1472(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	jmp	.LBB968_55
.LBB968_3:
	vmovdqu64	1464(%rsp), %zmm1
	vmovdqu64	1432(%rsp), %zmm0
	vmovdqu64	%zmm1, 112(%rsp)
	vmovdqu64	%zmm0, 80(%rsp)
.Ltmp12918:
	leaq	224(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	leaq	80(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp12919:
	cmpq	$-1, 224(%rsp)
	je	.LBB968_9
	vmovdqu	224(%rsp), %ymm0
	cmpq	$-1, 1104(%rsp)
	vmovdqu	%ymm0, 384(%rsp)
	je	.LBB968_11
	vmovdqu64	1144(%rsp), %zmm1
	vmovdqu64	1104(%rsp), %zmm0
	movq	408(%rsp), %rax
	movq	%rax, 488(%rsp)
	movq	$0, 464(%rsp)
	movq	$8, 472(%rsp)
	movq	$0, 480(%rsp)
	vmovdqu64	%zmm1, 120(%rsp)
	vmovdqu64	%zmm0, 80(%rsp)
	cmpq	$-1, 80(%rsp)
	je	.LBB968_18
	leaq	224(%rsp), %rdi
	leaq	464(%rsp), %rsi
	leaq	1104(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB968_8
.LBB968_19:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_21
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_21:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_27
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_21
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
.LBB968_24:
	cmpq	%rax, %rsi
	jge	.LBB968_26
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB968_24
.LBB968_26:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_27:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	176(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB968_28
	jmp	.LBB968_30
.LBB968_9:
	xorl	%ebp, %ebp
.Ltmp13188:
	leaq	80(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp13189:
	vmovdqu64	80(%rsp), %zmm0
	vmovdqu64	112(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB968_78
.LBB968_11:
	movl	(%r13), %ebp
	movq	%r15, 40(%rsp)
	cmpl	$28, %ebp
	jne	.LBB968_79
	movq	624(%r15), %rax
	xorl	%ebx, %ebx
	testq	%rax, %rax
	je	.LBB968_90
	testb	$1, 1200(%r15)
	je	.LBB968_91
	movq	1208(%r15), %rcx
	cmpq	40(%rax), %rcx
	jne	.LBB968_89
	movl	1216(%r15), %ecx
	subl	80(%rax), %ecx
	jb	.LBB968_89
	cmpq	%rcx, 32(%rax)
	jbe	.LBB968_89
	movq	24(%rax), %rax
	shlq	$4, %rcx
	movq	(%rax,%rcx), %rbx
	movq	8(%rax,%rcx), %r14
	jmp	.LBB968_91
.LBB968_18:
	movq	472(%rsp), %rcx
	movq	464(%rsp), %rax
	movq	480(%rsp), %rdx
	movq	%rcx, 240(%rsp)
	movq	488(%rsp), %rcx
	movq	%rax, 232(%rsp)
	movq	%rdx, 248(%rsp)
	movq	%rcx, 256(%rsp)
	movq	$-1, 224(%rsp)
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB968_19
.LBB968_8:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB968_30
.LBB968_28:
	lock		decq	(%rax)
	jne	.LBB968_30
	leaq	176(%rsp), %rdi
	#MEMBARRIER
.Ltmp12920:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp12921:
.LBB968_30:
	vmovdqu64	224(%rsp), %zmm0
	vmovdqu64	256(%rsp), %zmm1
	movq	56(%rsp), %rax
	movl	$0, 48(%rsp)
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.LBB968_31:
	movq	392(%rsp), %rbx
	movq	400(%rsp), %r14
	testq	%r14, %r14
	je	.LBB968_44
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB968_36
	.p2align	4
.LBB968_33:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_34:
	vzeroupper
	callq	*%r13
.LBB968_35:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB968_44
.LBB968_36:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB968_35
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_39
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_39:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_34
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_39
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
.LBB968_42:
	cmpq	%rax, %rdx
	jge	.LBB968_33
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB968_42
	jmp	.LBB968_33
.LBB968_44:
	movq	384(%rsp), %rax
	movl	48(%rsp), %ebp
	testq	%rax, %rax
	je	.LBB968_54
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_47
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_47:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_53
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_47
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
.LBB968_50:
	cmpq	%rax, %rsi
	jge	.LBB968_52
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB968_50
.LBB968_52:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_53:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB968_54:
	testb	%bpl, %bpl
	je	.LBB968_78
.LBB968_55:
	movq	1176(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB968_65
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1184(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_58
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_58:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_64
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_58
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
.LBB968_61:
	cmpq	%rax, %rsi
	jge	.LBB968_63
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB968_61
.LBB968_63:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_64:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB968_65:
	movq	1104(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB968_75
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	1112(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_68
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_68:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_74
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_68
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
.LBB968_71:
	cmpq	%rax, %rsi
	jge	.LBB968_73
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB968_71
.LBB968_73:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_74:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB968_75:
	movq	1200(%rsp), %rax
	testq	%rax, %rax
	je	.LBB968_78
	lock		decq	(%rax)
	jne	.LBB968_78
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1200(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB968_78:
	movq	56(%rsp), %rax
	addq	$1544, %rsp
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
.LBB968_79:
	.cfi_def_cfa_offset 1600
	movq	352(%r15), %rax
	testq	%rax, %rax
	jne	.LBB968_81
	movq	<purrdf_sparql_eval::eval::EvalCtx<_>>::endpoint_scan::ABSENT@GOTPCREL(%rip), %rax
	cmpq	$0, (%rax)
	movq	%r13, 72(%rsp)
	jne	.LBB968_82
	jmp	.LBB968_175
.LBB968_81:
	addq	$184, %rax
	cmpq	$0, (%rax)
	movq	%r13, 72(%rsp)
	je	.LBB968_175
.LBB968_82:
	movq	400(%rsp), %rcx
	movq	%rcx, 64(%rsp)
	testq	%rcx, %rcx
	je	.LBB968_175
	movq	$0, 944(%rsp)
	movq	$8, 952(%rsp)
	movq	$0, 960(%rsp)
.Ltmp12923:
	movq	purrdf_sparql_eval::service_endpoints::lateral_endpoint_uses@GOTPCREL(%rip), %rax
	leaq	944(%rsp), %rsi
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp12924:
	movq	952(%rsp), %r14
	movq	960(%rsp), %r15
	testq	%r15, %r15
	je	.LBB968_112
	xorl	%r12d, %r12d
	movq	%r14, %rbx
	.p2align	4
.LBB968_86:
.Ltmp12926:
	movq	%r13, %rdi
	movq	%rbx, %rsi
	callq	<alloc::vec::Vec<(purrdf_sparql_algebra::ast::Variable, bool)>>::retain::<purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>::{closure#0}
.Ltmp12927:
	testb	%al, %al
	je	.LBB968_458
	incq	%r12
	addq	$24, %rbx
	cmpq	%r12, %r15
	jne	.LBB968_86
	jmp	.LBB968_113
.LBB968_89:
	xorl	%ebx, %ebx
.LBB968_90:
.LBB968_91:
	movb	$1, %bpl
.Ltmp13166:
	leaq	800(%rsp), %rdi
	movq	%r15, %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_node
.Ltmp13167:
.Ltmp13168:
	movq	40(%rsp), %rsi
	leaq	1208(%rsp), %rdi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge
.Ltmp13169:
	movq	40(%rsp), %r9
	cmpb	$-1, 1208(%rsp)
	leaq	1200(%r9), %r15
	je	.LBB968_101
.Ltmp13170:
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
.Ltmp13171:
	vmovups	1208(%rsp), %xmm0
	movq	%rax, 488(%rsp)
	movq	1224(%rsp), %rax
	leaq	80(%rsp), %rdi
	leaq	464(%rsp), %rsi
	leaq	224(%rsp), %rdx
	movq	$0, 464(%rsp)
	movq	$8, 472(%rsp)
	movq	$0, 224(%rsp)
	movq	$1, 232(%rsp)
	movq	$0, 240(%rsp)
	movq	$0, 480(%rsp)
	movq	%rax, 264(%rsp)
	vmovups	%xmm0, 248(%rsp)
	movq	$0, 272(%rsp)
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	112(%rsp), %rcx
	vmovups	96(%rsp), %xmm0
	movq	80(%rsp), %rax
	movq	88(%rsp), %rsi
	movq	120(%rsp), %rdx
	movq	%rcx, 928(%rsp)
	movzbl	128(%rsp), %ecx
.LBB968_96:
	vmovaps	%xmm0, 912(%rsp)
	vmovdqu	129(%rsp), %xmm0
	movq	144(%rsp), %rdi
	vmovups	152(%rsp), %xmm1
	vmovups	800(%rsp), %xmm2
	movq	%rax, 1328(%rsp)
	movq	928(%rsp), %rax
	movq	%rsi, 1336(%rsp)
	movq	40(%rsp), %r8
	movq	%rax, 1360(%rsp)
	vmovdqa	%xmm0, 768(%rsp)
	movq	%rdi, 783(%rsp)
	movq	168(%rsp), %rdi
	vmovaps	%xmm1, 880(%rsp)
	vmovaps	912(%rsp), %xmm1
	vmovaps	%xmm2, (%r15)
	vmovdqa	768(%rsp), %xmm2
	movq	783(%rsp), %rax
	movq	%rdi, 896(%rsp)
	movq	816(%rsp), %rdi
	vmovups	%xmm1, 1344(%rsp)
	vmovdqa	880(%rsp), %xmm1
	movq	%rdx, 1368(%rsp)
	movb	%cl, 1376(%rsp)
	vmovdqu	%xmm2, 1377(%rsp)
	movq	%rax, 1392(%rsp)
	movq	896(%rsp), %rax
	movq	%rdi, 16(%r15)
	movl	824(%rsp), %edi
	movq	%rax, 1416(%rsp)
	movl	%edi, 1228(%r8)
	movzbl	828(%rsp), %edi
	vmovdqu	%xmm1, 1400(%rsp)
	movb	%dil, 1238(%r8)
.Ltmp13175:
	leaq	1296(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	leaq	1328(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp13176:
	cmpq	$-1, 1296(%rsp)
	je	.LBB968_104
	vmovdqu64	1104(%rsp), %zmm0
	vmovdqu64	1144(%rsp), %zmm1
	vmovdqu64	%zmm0, 80(%rsp)
	vmovdqu64	%zmm1, 120(%rsp)
	cmpq	$-1, 80(%rsp)
	je	.LBB968_105
	leaq	224(%rsp), %rdi
	leaq	1296(%rsp), %rsi
	leaq	1104(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB968_106
.LBB968_101:
	addq	$8, %r13
.Ltmp13172:
	leaq	224(%rsp), %rdi
	leaq	384(%rsp), %rdx
	movq	%r13, %rsi
	movq	%rbx, %rcx
	movq	%r14, %r8
	callq	purrdf_sparql_eval::property_fn_eval::eval_call_over::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp13173:
	vmovups	232(%rsp), %ymm0
	vmovdqu	265(%rsp), %xmm1
	movq	224(%rsp), %rcx
	movzbl	264(%rsp), %eax
	movq	280(%rsp), %rdx
	vmovdqa	%xmm1, 992(%rsp)
	vmovups	%ymm0, 464(%rsp)
	movq	%rdx, 1007(%rsp)
	cmpq	$-1, %rcx
	je	.LBB968_153
	vmovaps	288(%rsp), %xmm0
	movq	480(%rsp), %rsi
	movq	816(%rsp), %r11
	movq	1007(%rsp), %rdi
	movl	824(%rsp), %r10d
	movq	40(%rsp), %r8
	movzbl	828(%rsp), %r9d
	movq	304(%rsp), %rdx
	movq	%rsi, 928(%rsp)
	movq	488(%rsp), %rsi
	movq	%r11, 16(%r15)
	movq	%rdx, 896(%rsp)
	movq	312(%rsp), %rdx
	vmovaps	%xmm0, 880(%rsp)
	vmovaps	464(%rsp), %xmm0
	vmovdqa	880(%rsp), %xmm1
	vmovaps	%xmm0, 912(%rsp)
	vmovaps	992(%rsp), %xmm0
	vmovaps	%xmm0, 768(%rsp)
	vmovups	800(%rsp), %xmm0
	movq	%rdi, 783(%rsp)
	movq	928(%rsp), %rdi
	vmovdqa	768(%rsp), %xmm2
	vmovaps	%xmm0, (%r15)
	vmovdqa	912(%rsp), %xmm0
	movl	%r10d, 1228(%r8)
	movb	%r9b, 1238(%r8)
	movq	56(%rsp), %r9
	movq	%rdi, 40(%r9)
	vmovdqu	%xmm0, 24(%r9)
	movq	%rsi, 48(%r9)
	movq	783(%rsp), %rsi
	vmovdqu	%xmm2, 57(%r9)
	movq	%rsi, 72(%r9)
	movq	896(%rsp), %rsi
	vmovdqa	%xmm1, 80(%r9)
	movq	%rsi, 96(%r9)
	movq	%rcx, 16(%r9)
	movb	%al, 56(%r9)
	movq	%rdx, 104(%r9)
	movq	$1, (%r9)
	jmp	.LBB968_448
.LBB968_104:
.Ltmp13180:
	leaq	224(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp13181:
	jmp	.LBB968_111
.LBB968_105:
	vmovdqu	1296(%rsp), %ymm0
	vmovdqu	%ymm0, 232(%rsp)
	movq	$-1, 224(%rsp)
.LBB968_106:
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB968_108
	movq	160(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB968_108:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB968_111
	lock		decq	(%rax)
	jne	.LBB968_111
	leaq	176(%rsp), %rdi
	#MEMBARRIER
.Ltmp13178:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp13179:
.LBB968_111:
	vmovdqu64	224(%rsp), %zmm0
	vmovdqu64	256(%rsp), %zmm1
	movq	56(%rsp), %rax
	xorl	%ebp, %ebp
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 48(%rsp)
	jne	.LBB968_31
	jmp	.LBB968_449
.LBB968_112:
	xorl	%r15d, %r15d
.LBB968_113:
	movq	944(%rsp), %rax
	leaq	(%r15,%r15,2), %rcx
	movq	%r14, 704(%rsp)
	movl	%ebp, 208(%rsp)
	leaq	(%r14,%rcx,8), %rcx
	movq	%rcx, 16(%rsp)
	movq	%rax, 720(%rsp)
	movq	%rcx, 728(%rsp)
	testq	%r15, %r15
	je	.LBB968_162
	movq	64(%rsp), %rax
	movq	392(%rsp), %rsi
	movq	408(%rsp), %rdi
	leaq	(,%rax,8), %rax
	addq	$8, %rsi
	addq	$16, %rdi
	leaq	(%rax,%rax,4), %rcx
	movq	%rcx, 200(%rsp)
	movq	%r14, %rcx
	movq	40(%rsp), %r14
	leaq	888(%r14), %rdx
	movq	%rdx, 48(%rsp)
	leaq	-40(%rax,%rax,4), %rdx
	movabsq	$-3689348814741910323, %rax
	mulxq	%rax, %rbx, %rbx
	movq	%rsi, 360(%rsp)
	leaq	416(%rsp), %rsi
	movq	%rdi, 352(%rsp)
	shrq	$5, %rbx
	incq	%rbx
	movq	%rbx, 216(%rsp)
.LBB968_115:
	movq	%rcx, %rax
	movzbl	16(%rax), %ebp
	addq	$24, %rcx
	movq	%rcx, 24(%rsp)
	cmpb	$2, %bpl
	je	.LBB968_161
	vmovdqu	(%rax), %xmm0
	vmovdqu	%xmm0, 416(%rsp)
.Ltmp12940:
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.13412714042204560522)
.Ltmp12941:
	cmpq	$1, %rax
	jne	.LBB968_125
	movq	%rdx, %r12
	testb	$1, %bpl
	je	.LBB968_135
	movq	200(%rsp), %rax
	movq	360(%rsp), %rcx
	movq	24(%rsp), %r14
	xorl	%ebx, %ebx
	.p2align	4
.LBB968_120:
	movq	-8(%rcx), %rsi
	movq	%rcx, %rdx
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB968_122
	movq	8(%rcx), %rsi
	movq	(%rcx), %rdx
	decq	%rsi
.LBB968_122:
	cmpq	%rsi, %r12
	jae	.LBB968_473
	xorl	%esi, %esi
	cmpl	$2, (%rdx,%r12,8)
	sete	%sil
	addq	$40, %rcx
	addq	%rsi, %rbx
	addq	$-40, %rax
	jne	.LBB968_120
.LBB968_124:
	movq	40(%rsp), %r14
	movq	72(%rsp), %r13
	testq	%rbx, %rbx
	je	.LBB968_146
.LBB968_125:
	movq	1136(%r14), %rax
	testq	%rax, %rax
	je	.LBB968_151
	movq	40(%rsp), %rcx
	movq	416(%rsp), %r13
	movq	424(%rsp), %r15
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %rbp
	movq	1128(%rcx), %r14
	leaq	16(%r13), %r12
	jmp	.LBB968_129
.LBB968_127:
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r12, %rsi
	movq	%r15, %rdx
	callq	*%rax
	testl	%eax, %eax
	je	.LBB968_133
.LBB968_128:
	addq	$-48, %rbp
	je	.LBB968_151
.LBB968_129:
	movq	-24(%r14,%rbp), %rdi
	movq	-16(%r14,%rbp), %rax
	cmpq	%r13, %rdi
	je	.LBB968_131
	cmpq	%r15, %rax
	je	.LBB968_127
.LBB968_131:
	cmpq	%r13, %rdi
	jne	.LBB968_128
	cmpq	%r15, %rax
	jne	.LBB968_128
.LBB968_133:
	movq	-48(%r14,%rbp), %rax
	testq	%rax, %rax
	jne	.LBB968_164
	lock		decq	(%r13)
	movq	40(%rsp), %r14
	movq	72(%rsp), %r13
	jmp	.LBB968_147
.LBB968_135:
	movq	200(%rsp), %r15
	movq	360(%rsp), %r13
	movq	24(%rsp), %r14
	movabsq	$9223372036854775792, %rbp
	xorl	%ebx, %ebx
	jmp	.LBB968_138
.LBB968_136:
	incq	%rbx
.LBB968_137:
	addq	$40, %r13
	addq	$-40, %r15
	je	.LBB968_124
.LBB968_138:
	movq	-8(%r13), %rsi
	movq	%r13, %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB968_140
	movq	8(%r13), %rsi
	movq	(%r13), %rax
	decq	%rsi
.LBB968_140:
	cmpq	%rsi, %r12
	jae	.LBB968_473
	movl	(%rax,%r12,8), %ecx
	cmpl	$2, %ecx
	je	.LBB968_136
	movq	40(%rsp), %rdx
	movl	4(%rax,%r12,8), %r8d
	movq	664(%rdx), %rdx
.Ltmp12943:
	movq	48(%rsp), %rsi
	leaq	224(%rsp), %rdi
	callq	<purrdf_sparql_eval::scratch::ScratchInterner>::try_value_of::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp12944:
	cmpq	$-1, 224(%rsp)
	je	.LBB968_157
	vmovups	240(%rsp), %zmm1
	vmovups	224(%rsp), %zmm0
	leaq	16(%rbp), %rax
	vmovups	%zmm1, 480(%rsp)
	vmovups	%zmm0, 464(%rsp)
	vmovups	480(%rsp), %zmm1
	vmovups	464(%rsp), %zmm0
	vmovups	%zmm1, 816(%rsp)
	vmovups	%zmm0, 800(%rsp)
	vmovdqu64	800(%rsp), %zmm0
	vmovdqu64	816(%rsp), %zmm1
	vmovdqu64	%zmm0, 992(%rsp)
	vmovdqu64	%zmm1, 1008(%rsp)
	cmpq	%rax, 992(%rsp)
	jne	.LBB968_159
.Ltmp12952:
	leaq	992(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.13412714042204560522)
.Ltmp12953:
	jmp	.LBB968_137
.LBB968_146:
	movq	416(%rsp), %rax
	lock		decq	(%rax)
.LBB968_147:
	leaq	416(%rsp), %rsi
	jne	.LBB968_149
	#MEMBARRIER
.Ltmp12969:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	movq	%rsi, %rdi
	movq	%rsi, %rbx
	callq	*%rax
	movq	%rbx, %rsi
.Ltmp12970:
.LBB968_149:
	movq	24(%rsp), %rcx
	movq	352(%rsp), %rdi
	movq	216(%rsp), %rbx
	cmpq	16(%rsp), %rcx
	jne	.LBB968_115
	movq	16(%rsp), %r14
	jmp	.LBB968_162
.LBB968_151:
	movq	24(%rsp), %rax
	movabsq	$9223372036854775792, %r14
	movq	%rax, 712(%rsp)
	cmpq	64(%rsp), %rbx
	jae	.LBB968_166
.LBB968_152:
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.332(%rip), %rax
	jmp	.LBB968_167
.LBB968_153:
	cmpb	$-1, %al
	je	.LBB968_155
	vmovaps	992(%rsp), %xmm0
	movq	1007(%rsp), %rcx
	leaq	80(%rsp), %rdi
	leaq	464(%rsp), %rsi
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
	movq	80(%rsp), %rax
	movq	120(%rsp), %rdx
	movzbl	128(%rsp), %ecx
	jmp	.LBB968_156
.LBB968_155:
	vmovups	464(%rsp), %ymm0
	leaq	88(%rsp), %rax
	vmovups	%ymm0, (%rax)
	movq	$-1, %rax
.LBB968_156:
	vmovups	96(%rsp), %xmm0
	movq	88(%rsp), %rsi
	movq	112(%rsp), %rdi
	movq	%rdi, 928(%rsp)
	jmp	.LBB968_96
.LBB968_157:
	movq	%r14, 712(%rsp)
.Ltmp12955:
	leaq	80(%rsp), %rdi
	callq	<purrdf_sparql_eval::error::EvalError>::source_read::<purrdf_core::dataset_view::TermLookupError>
.Ltmp12956:
	movq	72(%rsp), %r13
	vmovups	104(%rsp), %zmm0
	vmovups	88(%rsp), %zmm1
	movq	80(%rsp), %r14
	movq	168(%rsp), %rbx
	vmovups	%zmm0, 480(%rsp)
	vmovups	%zmm1, 464(%rsp)
	vmovups	480(%rsp), %zmm1
	vmovups	464(%rsp), %zmm0
	vmovups	%zmm1, 816(%rsp)
	vmovups	%zmm0, 800(%rsp)
	vmovdqu64	800(%rsp), %zmm0
	vmovdqu64	816(%rsp), %zmm1
	jmp	.LBB968_169
.LBB968_159:
	movq	%r14, 712(%rsp)
.Ltmp12945:
	movq	purrdf_sparql_eval::service_endpoints::non_iri_endpoint@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	leaq	416(%rsp), %rsi
	leaq	992(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp12946:
	movq	72(%rsp), %r13
	vmovdqu64	88(%rsp), %zmm0
	vmovdqu64	104(%rsp), %zmm1
	movq	80(%rsp), %r14
	movq	168(%rsp), %rbx
	vmovdqu64	%zmm0, 608(%rsp)
	vmovdqu64	%zmm1, 624(%rsp)
.Ltmp12950:
	leaq	992(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.13412714042204560522)
.Ltmp12951:
	jmp	.LBB968_170
.LBB968_161:
	movq	24(%rsp), %r14
.LBB968_162:
	movb	$1, %bpl
	movq	%r14, 712(%rsp)
.Ltmp12975:
	leaq	704(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::service_endpoints::Occurrence>>
.Ltmp12976:
	movq	40(%rsp), %r15
	movl	208(%rsp), %ebp
	jmp	.LBB968_175
.LBB968_164:
	movq	24(%rsp), %rcx
	movabsq	$9223372036854775792, %r14
	movq	%rcx, 712(%rsp)
	cmpq	64(%rsp), %rbx
	jb	.LBB968_152
	cmpl	$1, %eax
	je	.LBB968_152
.LBB968_166:
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.333(%rip), %rax
.LBB968_167:
	movq	416(%rsp), %r8
	movq	424(%rsp), %rcx
	movq	72(%rsp), %r13
	movq	%rax, 800(%rsp)
	leaq	<&str as core::fmt::Display>::fmt (.llvm.13412714042204560522)(%rip), %rax
	leaq	88(%rsp), %rbx
	movq	$56, 808(%rsp)
	addq	$16, %r8
	movq	%r8, 464(%rsp)
	movq	%rcx, 472(%rsp)
	leaq	464(%rsp), %rcx
	movq	%rcx, 224(%rsp)
	leaq	800(%rsp), %rcx
	movq	%rax, 232(%rsp)
	movq	%rcx, 240(%rsp)
	movq	%rax, 248(%rsp)
.Ltmp12959:
	movq	alloc::fmt::format::format_inner@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.1634(%rip), %rsi
	leaq	224(%rsp), %rdx
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp12960:
	movb	$-1, 112(%rsp)
	addq	$23, %r14
	vmovdqu64	(%rbx), %zmm0
	vmovdqu64	16(%rbx), %zmm1
	movq	168(%rsp), %rbx
.LBB968_169:
	vmovdqu64	%zmm0, 608(%rsp)
	vmovdqu64	%zmm1, 624(%rsp)
.LBB968_170:
	movq	416(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB968_172
	#MEMBARRIER
.Ltmp12964:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp12965:
.LBB968_172:
	movb	$1, %bpl
.Ltmp12967:
	leaq	704(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::service_endpoints::Occurrence>>
.Ltmp12968:
	movq	40(%rsp), %r15
	movl	208(%rsp), %ebp
	cmpq	$-1, %r14
	je	.LBB968_175
	vmovups	608(%rsp), %zmm0
	vmovups	624(%rsp), %zmm1
	movq	56(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovups	%zmm0, 24(%rax)
	movq	%r14, 16(%rax)
	movq	%rbx, 104(%rax)
	movq	$1, (%rax)
	jmp	.LBB968_448
.LBB968_175:
	movq	584(%r15), %rdx
	testq	%rdx, %rdx
	je	.LBB968_186
	cmpq	$0, 40(%rdx)
	je	.LBB968_186
	vpbroadcastq	.LCPI968_7(%rip), %xmm0
	movabsq	$2746377873070565055, %rax
	movq	16(%rdx), %rcx
	movq	24(%rdx), %rdx
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	xorq	%r13, %rax
	vpinsrq	$0, %rax, %xmm0, %xmm0
	vaesenc	.LCPI968_1(%rip), %xmm0, %xmm0
	vaesenc	.LCPI968_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI968_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rax
	movq	%rax, %rsi
	shrq	$57, %rsi
	vpbroadcastb	%esi, %xmm0
	xorl	%esi, %esi
.LBB968_178:
	andq	%rdx, %rax
	vmovdqu	(%rcx,%rax), %xmm2
	vpcmpeqb	%xmm0, %xmm2, %k0
	kortestw	%k0, %k0
	je	.LBB968_182
	kmovd	%k0, %edi
.LBB968_180:
	xorl	%r8d, %r8d
	tzcntl	%edi, %r8d
	addq	%rax, %r8
	andq	%rdx, %r8
	negq	%r8
	leaq	(%r8,%r8,4), %r8
	cmpq	%r13, -40(%rcx,%r8,8)
	je	.LBB968_184
	leal	-1(%rdi), %r8d
	andw	%di, %r8w
	movl	%r8d, %edi
	jne	.LBB968_180
.LBB968_182:
	vpcmpeqb	%xmm1, %xmm2, %k0
	kortestw	%k0, %k0
	jne	.LBB968_186
	leaq	16(%rax,%rsi), %rax
	addq	$16, %rsi
	jmp	.LBB968_178
.LBB968_184:
	leaq	(%rcx,%r8,8), %rsi
	cmpl	$1, -32(%rsi)
	jne	.LBB968_186
	addq	$-24, %rsi
	leaq	560(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::deferred_exists::DeferredLateral as core::clone::Clone>::clone
	cmpq	$0, 560(%rsp)
	jne	.LBB968_194
	jmp	.LBB968_187
.LBB968_186:
	movq	$0, 560(%rsp)
.LBB968_187:
	cmpl	$23, %ebp
	jne	.LBB968_194
	movq	32(%r13), %rax
	testq	%rax, %rax
	je	.LBB968_469
	cmpl	$21, (%rax)
	jne	.LBB968_194
	cmpq	$0, 48(%rax)
	jne	.LBB968_194
	cmpq	$1, 24(%rax)
	jne	.LBB968_194
	movq	16(%rax), %rax
	cmpq	$22, 8(%rax)
	jne	.LBB968_194
	movq	(%rax), %rax
	vmovdqu	22(%rax), %xmm1
	vmovdqu	16(%rax), %xmm0
	vpxor	.LCPI968_4(%rip), %xmm0, %xmm0
	vpternlogq	$246, .LCPI968_5(%rip), %xmm1, %xmm0
	vptest	%xmm0, %xmm0
	je	.LBB968_455
.LBB968_194:
	movq	408(%rsp), %r13
	lock		incq	(%r13)
	jle	.LBB968_477
	movq	%r13, 696(%rsp)
	vmovdqu	anon.a12f493ba210922c94e5446ac885c35e.31.llvm.13412714042204560522(%rip), %ymm0
	movq	400(%rsp), %r15
	movq	32(%r13), %rax
	movq	$0, 992(%rsp)
	movq	$8, 1000(%rsp)
	movq	$0, 1008(%rsp)
	movq	%rax, 208(%rsp)
	movabsq	$128102389400760776, %rax
	decq	%rax
	vmovdqu	%ymm0, 1016(%rsp)
	cmpq	%rax, %r15
	jbe	.LBB968_198
	xorl	%ebx, %ebx
.LBB968_197:
	movb	$1, %bpl
.Ltmp13152:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	*%rax
.Ltmp13153:
	jmp	.LBB968_477
.LBB968_198:
	leaq	16(%r13), %rax
	movq	%r13, 16(%rsp)
	movq	%rax, 352(%rsp)
	testq	%r15, %r15
	je	.LBB968_346
	leaq	(,%r15,8), %rax
	movl	$8, %esi
	movl	$8, %ebx
	leaq	(%rax,%rax,8), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB968_197
	movq	392(%rsp), %rcx
	movq	%r15, 584(%rsp)
	movq	%rax, 592(%rsp)
	movq	%rax, 64(%rsp)
	leaq	(%r15,%r15,4), %rax
	movq	40(%rsp), %r15
	movq	$0, 600(%rsp)
	movq	$0, 24(%rsp)
	leaq	(%rcx,%rax,8), %rax
	movq	%rcx, 48(%rsp)
	movabsq	$9223372036854775792, %rcx
	leaq	1096(%r15), %rdx
	movq	%rdx, 456(%rsp)
	movq	%rax, 360(%rsp)
	leaq	40(%rcx), %rax
	movq	%rax, 1072(%rsp)
	jmp	.LBB968_202
.LBB968_201:
	movq	24(%rsp), %rsi
	movq	144(%rsp), %rcx
	movq	64(%rsp), %rdx
	movq	48(%rsp), %rdi
	leaq	(%rsi,%rsi,8), %rax
	addq	$40, %rdi
	incq	%rsi
	movq	%rsi, 24(%rsp)
	movq	%rdi, 48(%rsp)
	movq	%rcx, 64(%rdx,%rax,8)
	vmovdqu64	80(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rdx,%rax,8)
	movq	%rsi, 600(%rsp)
	cmpq	360(%rsp), %rdi
	je	.LBB968_350
.LBB968_202:
	movq	560(%rsp), %r12
	testq	%r12, %r12
	je	.LBB968_205
	movq	48(%rsp), %rax
	movq	(%rax), %rbx
	decq	%rbx
	cmpq	$4, %rbx
	jbe	.LBB968_207
	movq	16(%rax), %rbx
	movq	8(%rax), %r14
	decq	%rbx
	jmp	.LBB968_208
.LBB968_205:
	movzbl	1237(%r15), %eax
	incq	%rax
	movq	%rax, 464(%rsp)
	movq	48(%rsp), %rax
	movq	$0, 480(%rsp)
	movq	(%rax), %rcx
	decq	%rcx
	cmpq	$4, %rcx
	jbe	.LBB968_217
	movq	16(%rax), %rcx
	movq	8(%rax), %rdx
	decq	%rcx
	jmp	.LBB968_218
.LBB968_207:
	leaq	8(%rax), %r14
.LBB968_208:
	leaq	608(%rsp), %rax
	movb	$0, 608(%rsp)
	movq	%rax, 80(%rsp)
	leaq	80(%rsp), %rax
	#APP
	#NO_APP
	movq	purrdf_stack::FLOOR::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL@GOTTPOFF(%rip), %rax
	movq	80(%rsp), %rdi
	movq	%fs:(%rax), %rcx
	movq	%rdi, %rax
	subq	%rcx, %rax
	cmpq	$131072, %rax
	setb	%al
	cmpq	%rcx, %rdi
	jb	.LBB968_342
	testb	%al, %al
	jne	.LBB968_343
.LBB968_210:
	movq	16(%rsp), %rax
	movq	24(%rax), %rcx
	movq	32(%rax), %r8
.Ltmp12979:
	leaq	80(%rsp), %rdi
	movq	%r14, %rsi
	movq	%rbx, %rdx
	movq	%r15, %r9
	vzeroupper
	callq	purrdf_sparql_eval::expr::outer_bindings_for_substitution::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp12980:
	leaq	88(%rsp), %rdx
	movq	80(%rsp), %rax
	vmovups	(%rdx), %ymm0
	vmovups	16(%rdx), %ymm1
	vmovups	%ymm0, 608(%rsp)
	vmovups	%ymm1, 624(%rsp)
	cmpq	$-1, %rax
	je	.LBB968_214
	vmovdqu	48(%rdx), %ymm0
	vmovdqu	608(%rsp), %ymm2
	vmovdqu	624(%rsp), %ymm1
	movq	80(%rdx), %rcx
	leaq	240(%rsp), %rsi
	movq	%rcx, 88(%rsi)
	vmovdqu	%ymm0, 56(%rsi)
	vmovdqu	%ymm1, 24(%rsi)
	vmovdqu	%ymm2, 8(%rsi)
	movq	%rax, 240(%rsp)
.LBB968_213:
	movq	$1, 224(%rsp)
	jmp	.LBB968_297
.LBB968_214:
	vmovdqu	608(%rsp), %ymm0
	vmovdqu	624(%rsp), %ymm1
	movq	576(%rsp), %rsi
	movq	568(%rsp), %rdx
	leaq	16(%r12), %rax
	vmovdqu	%ymm0, 944(%rsp)
	vmovdqu	%ymm1, 960(%rsp)
	movq	80(%r12), %rcx
	movq	$3, 1080(%rsp)
	movq	%rax, 1088(%rsp)
	movl	$0, %eax
	addq	$16, %rcx
	cmpq	$1, %rsi
	adcq	$1, %rax
	testq	%rdx, %rdx
	movq	%rcx, 1096(%rsp)
	cmoveq	%rdx, %rax
	testq	%rax, %rax
	je	.LBB968_221
	cmpq	$1, %rax
	jne	.LBB968_228
	addq	$16, %rsi
	leaq	48(%r12), %rcx
.Ltmp12981:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	704(%rsp), %rdi
	leaq	944(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp12982:
	jmp	.LBB968_223
.LBB968_217:
	leaq	8(%rax), %rdx
.LBB968_218:
.Ltmp13068:
	movq	72(%rsp), %rsi
	movq	352(%rsp), %r8
	leaq	80(%rsp), %rdi
	leaq	464(%rsp), %r9
	movq	%r15, (%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_correlated::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp13069:
	cmpl	$1, 80(%rsp)
	je	.LBB968_441
	leaq	88(%rsp), %rcx
	movq	88(%rsp), %rax
	vmovups	32(%rcx), %zmm1
	vmovups	8(%rcx), %zmm0
	leaq	240(%rsp), %rcx
	vmovups	%zmm1, 248(%rsp)
	vmovups	%zmm0, 224(%rsp)
	vmovups	224(%rsp), %ymm0
	vmovdqu	40(%rcx), %ymm1
	vmovups	%ymm0, 416(%rsp)
	vmovdqu	16(%rcx), %ymm0
	jmp	.LBB968_299
.LBB968_221:
	leaq	88(%rsp), %rax
	movq	$0, 80(%rsp)
	movq	$8, 88(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	leaq	48(%r12), %rcx
	vmovdqu	%xmm0, 8(%rax)
	movq	$8, 112(%rsp)
	movq	$0, 120(%rsp)
.Ltmp12983:
	movq	purrdf_sparql_eval::deferred_exists::with_row@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rbx
	leaq	704(%rsp), %rdi
	leaq	944(%rsp), %rdx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp12984:
.Ltmp12988:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp12989:
.LBB968_223:
	cmpq	$-1, 704(%rsp)
	je	.LBB968_229
	vmovdqu	720(%rsp), %ymm1
	vmovdqu	704(%rsp), %ymm0
	cmpq	$0, 632(%r15)
	vmovdqu	%ymm1, 624(%rsp)
	vmovdqu	%ymm0, 608(%rsp)
	je	.LBB968_240
	movq	80(%r12), %rax
	cmpq	$0, 40(%rax)
	je	.LBB968_240
	lock		incq	(%rax)
	jle	.LBB968_477
	movq	80(%r12), %rsi
	movb	$1, %bl
.Ltmp12990:
	movq	456(%rsp), %rdi
	vzeroupper
	callq	<alloc::vec::Vec<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>>::push_mut
.Ltmp12991:
	jmp	.LBB968_241
.LBB968_228:
	movq	$-1, 704(%rsp)
.LBB968_229:
	movq	40(%r12), %rdi
.Ltmp12999:
	leaq	1080(%rsp), %rsi
	movq	%r15, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::deferred_exists::nested_sites::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp13000:
	movq	%rax, 752(%rsp)
	movq	%rdx, 336(%rsp)
	movq	%rdx, 760(%rsp)
	leaq	48(%r12), %rdx
	movq	%rax, 344(%rsp)
	movq	$0, 368(%rsp)
	movq	$0, 376(%rsp)
.Ltmp13001:
	movq	<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::then@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	leaq	944(%rsp), %rsi
	xorl	%r15d, %r15d
	callq	*%rax
.Ltmp13002:
	movq	%rax, 608(%rsp)
	movq	$0, 80(%rsp)
	movq	$8, 88(%rsp)
	movq	%rax, %rbx
	movq	%rdx, 616(%rsp)
	movq	$0, 96(%rsp)
	testq	%rax, %rax
	je	.LBB968_248
	movq	%r12, 216(%rsp)
	movl	$8, %eax
	movl	$1, %ecx
	movq	$-64, %r12
	movq	$-32, %rbp
	movq	$-8, %r15
	xorl	%r14d, %r14d
	jmp	.LBB968_234
	.p2align	4
.LBB968_233:
	leaq	16(%rbx), %rcx
	addq	$8, %r12
	addq	$8, %rbp
	addq	$8, %r15
	movq	%rcx, (%rax,%r14,8)
	incq	%r14
	leaq	1(%r13), %rcx
	movq	%r14, 96(%rsp)
	movq	64(%rbx), %rbx
	testq	%rbx, %rbx
	je	.LBB968_237
.LBB968_234:
	movq	%rcx, %r13
	cmpq	80(%rsp), %r14
	jne	.LBB968_233
.Ltmp13003:
	movq	<alloc::raw_vec::RawVec<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	callq	*%rax
.Ltmp13004:
	movq	88(%rsp), %rax
	jmp	.LBB968_233
.LBB968_237:
	movq	88(%rsp), %rax
	movq	%rax, 32(%rsp)
	movq	%r14, %rax
	shrq	%rax
	je	.LBB968_260
	cmpq	$8, %r14
	jae	.LBB968_249
	xorl	%ecx, %ecx
	jmp	.LBB968_258
.LBB968_240:
	xorl	%ebx, %ebx
.LBB968_241:
	movq	40(%r12), %rsi
.Ltmp12992:
	leaq	80(%rsp), %rdi
	leaq	608(%rsp), %rdx
	leaq	1080(%rsp), %rcx
	movq	%r15, %r8
	vzeroupper
	callq	purrdf_sparql_eval::binop::eval_substituted::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp12993:
	testb	%bl, %bl
	je	.LBB968_246
	movq	1112(%r15), %rax
	testq	%rax, %rax
	je	.LBB968_246
	leaq	-1(%rax), %rcx
	movq	%rcx, 1112(%r15)
	movq	1104(%r15), %rcx
	movq	-8(%rcx,%rax,8), %rax
	movq	%rax, 752(%rsp)
	lock		decq	(%rax)
	jne	.LBB968_246
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	752(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB968_246:
	vmovdqu64	128(%rsp), %zmm1
	vmovdqu64	80(%rsp), %zmm0
	vmovdqu64	%zmm1, 272(%rsp)
	vmovdqu64	%zmm0, 224(%rsp)
.Ltmp12997:
	leaq	608(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp12998:
.LBB968_247:
.Ltmp13023:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp13024:
	jmp	.LBB968_297
.LBB968_248:
	xorl	%r14d, %r14d
	xorl	%r15d, %r15d
	jmp	.LBB968_286
.LBB968_249:
	cmpq	$32, %r14
	jae	.LBB968_251
	xorl	%ecx, %ecx
	jmp	.LBB968_255
.LBB968_251:
	vmovdqa64	.LCPI968_6(%rip), %zmm3
	movq	32(%rsp), %rdi
	movabsq	$9223372036854775792, %rdx
	movq	%rax, %rcx
	andq	%rdx, %rcx
	movq	%r13, %rdx
	shrq	$5, %rdx
	shlq	$4, %rdx
	negq	%rdx
	leaq	64(%rdi), %rsi
	addq	%rdi, %r12
	xorl	%edi, %edi
.LBB968_252:
	vpermq	(%r12,%rdi,8), %zmm3, %zmm0
	vpermq	-64(%r12,%rdi,8), %zmm3, %zmm1
	vpermq	-64(%rsi), %zmm3, %zmm2
	vmovdqu64	%zmm0, -64(%rsi)
	vpermq	(%rsi), %zmm3, %zmm0
	vmovdqu64	%zmm1, (%rsi)
	vmovdqu64	%zmm2, (%r12,%rdi,8)
	subq	$-128, %rsi
	vmovdqu64	%zmm0, -64(%r12,%rdi,8)
	addq	$-16, %rdi
	cmpq	%rdi, %rdx
	jne	.LBB968_252
	cmpq	%rcx, %rax
	je	.LBB968_260
	testb	$24, %r14b
	je	.LBB968_258
.LBB968_255:
	movq	32(%rsp), %rdi
	movq	%rcx, %rsi
	movq	%r13, %r8
	shrq	$3, %r8
	shlq	$2, %r8
	subq	%r8, %rsi
	xorl	%r8d, %r8d
	leaq	(%rdi,%rcx,8), %rdx
	shlq	$3, %rcx
	subq	%rcx, %rdi
	movabsq	$9223372036854775792, %rcx
	addq	$12, %rcx
	addq	%rbp, %rdi
	andq	%rax, %rcx
.LBB968_256:
	vpermq	$27, (%rdi,%r8,8), %ymm0
	vpermq	$27, (%rdx), %ymm1
	vmovdqu	%ymm0, (%rdx)
	vmovdqu	%ymm1, (%rdi,%r8,8)
	addq	$-4, %r8
	addq	$32, %rdx
	cmpq	%r8, %rsi
	jne	.LBB968_256
	cmpq	%rcx, %rax
	je	.LBB968_260
.LBB968_258:
	movq	32(%rsp), %rsi
	movq	%rcx, %rax
	shrq	%r13
	subq	%r13, %rax
	leaq	(%rsi,%rcx,8), %rdx
	shlq	$3, %rcx
	subq	%rcx, %rsi
	xorl	%ecx, %ecx
	addq	%r15, %rsi
	.p2align	4
.LBB968_259:
	movq	(%rdx), %rdi
	movq	(%rsi,%rcx,8), %r8
	movq	%r8, (%rdx)
	movq	%rdi, (%rsi,%rcx,8)
	decq	%rcx
	addq	$8, %rdx
	cmpq	%rcx, %rax
	jne	.LBB968_259
.LBB968_260:
	movq	80(%rsp), %rax
	movq	%rax, 192(%rsp)
	testq	%r14, %r14
	je	.LBB968_282
	movq	344(%rsp), %r13
	movq	32(%rsp), %rbp
	xorl	%r12d, %r12d
	xorl	%r15d, %r15d
	shlq	$4, %r13
	addq	336(%rsp), %r13
	leaq	(%rbp,%r14,8), %rax
	movq	%rax, 200(%rsp)
	jmp	.LBB968_265
.LBB968_262:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_263:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
.LBB968_264:
	addq	$8, %rbp
	movq	%r14, 368(%rsp)
	movq	%r14, %r12
	movq	%rbx, %r15
	cmpq	%rbp, 200(%rsp)
	je	.LBB968_283
.LBB968_265:
	movq	%r12, %rsi
	testq	%r12, %r12
	jne	.LBB968_267
	movq	216(%rsp), %rax
	movq	40(%rax), %rsi
.LBB968_267:
	vmovdqu	anon.a12f493ba210922c94e5446ac885c35e.31.llvm.13412714042204560522(%rip), %ymm0
	movq	(%rbp), %rdx
	leaq	16(%r15), %rax
	testq	%r15, %r15
	cmoveq	%r15, %rax
	movq	%rax, 1264(%rsp)
	movq	%r13, 1272(%rsp)
	movq	$0, 1280(%rsp)
	vmovdqu	%ymm0, 1232(%rsp)
.Ltmp13006:
	movq	purrdf_sparql_eval::expr::substitute_pattern_deferring@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	leaq	1232(%rsp), %rcx
	vzeroupper
	callq	*%rax
.Ltmp13007:
	movq	80(%rsp), %rax
	movq	88(%rsp), %r14
	cmpq	$-1, %rax
	jne	.LBB968_331
.Ltmp13025:
	movq	<purrdf_sparql_eval::expr::Deferral>::into_placeholders@GOTPCREL(%rip), %rax
	leaq	1232(%rsp), %rdi
	callq	*%rax
.Ltmp13026:
	movq	%rax, %rbx
	testq	%r15, %r15
	je	.LBB968_273
	lock		decq	(%r15)
	jne	.LBB968_273
	#MEMBARRIER
.Ltmp13028:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	376(%rsp), %rdi
	callq	*%rax
.Ltmp13029:
.LBB968_273:
	movq	%rbx, 376(%rsp)
	testq	%r12, %r12
	je	.LBB968_264
.Ltmp13033:
	movq	%r12, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::GraphPattern>
.Ltmp13034:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-144, %rcx
	movabsq	$-9223372036854775808, %rdx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_277
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_277:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_263
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_277
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	$-144, %rcx
	lock		xaddq	%rcx, (%rax)
	movabsq	$-9223372036854775808, %rax
	addq	$-144, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB968_280:
	cmpq	%rax, %rcx
	jge	.LBB968_262
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB968_280
	jmp	.LBB968_262
.LBB968_282:
	xorl	%ebx, %ebx
	xorl	%r14d, %r14d
.LBB968_283:
	movq	192(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB968_285
	movq	32(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB968_285:
	movq	216(%rsp), %r12
	movq	%rbx, %r15
.LBB968_286:
.Ltmp13038:
	leaq	608(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp13039:
	testq	%r14, %r14
	jne	.LBB968_289
	movq	40(%r12), %r14
.LBB968_289:
	movq	$0, 80(%rsp)
.Ltmp13043:
	movq	40(%rsp), %rsi
	leaq	608(%rsp), %rdi
	leaq	80(%rsp), %rdx
	movq	%r15, %rcx
	callq	<purrdf_sparql_eval::eval::EvalCtx>::enter_substituted_exists
.Ltmp13044:
	movq	632(%rsp), %rdx
.Ltmp13045:
	leaq	224(%rsp), %rdi
	movq	%r14, %rsi
	movq	%r14, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp13046:
.Ltmp13050:
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp13051:
.Ltmp13055:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp13056:
	movq	40(%rsp), %r15
	cmpq	$0, 344(%rsp)
	movq	336(%rsp), %rax
	je	.LBB968_296
	lock		decq	(%rax)
	jne	.LBB968_296
	#MEMBARRIER
.Ltmp13060:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	760(%rsp), %rdi
	callq	*%rax
.Ltmp13061:
.LBB968_296:
.Ltmp13066:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp13067:
.LBB968_297:
	cmpl	$1, 224(%rsp)
	je	.LBB968_440
	leaq	240(%rsp), %rcx
	movq	232(%rsp), %rax
	vmovups	24(%rcx), %zmm1
	vmovups	(%rcx), %zmm0
	leaq	496(%rsp), %rcx
	vmovups	%zmm1, 488(%rsp)
	vmovups	%zmm0, 464(%rsp)
	vmovups	464(%rsp), %ymm0
	vmovdqu	24(%rcx), %ymm1
	vmovups	%ymm0, 416(%rsp)
	vmovdqu	(%rcx), %ymm0
.LBB968_299:
	vmovdqu	%ymm1, 824(%rsp)
	vmovdqu	%ymm0, 800(%rsp)
	cmpq	$-1, %rax
	jne	.LBB968_347
	movq	440(%rsp), %rax
	movq	32(%rax), %rbx
	testq	%rbx, %rbx
	je	.LBB968_305
	movq	24(%rax), %r14
	shlq	$4, %rbx
	addq	%r14, %rbx
	.p2align	4
.LBB968_302:
	movq	(%r14), %rax
	lock		incq	(%rax)
	jle	.LBB968_477
	movq	8(%r14), %rdx
	movq	(%r14), %rsi
.Ltmp13076:
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	992(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp13077:
	addq	$16, %r14
	cmpq	%rbx, %r14
	jne	.LBB968_302
.LBB968_305:
	movq	48(%rsp), %rax
	movq	(%rax), %r14
	leaq	-1(%r14), %rdx
	cmpq	$4, %rdx
	jbe	.LBB968_308
	movq	16(%rax), %r14
	movq	8(%rax), %rsi
	leaq	-8(,%r14,8), %rdx
	leaq	-1(%r14), %r15
	cmpq	$5, %r15
	jae	.LBB968_310
	movq	40(%rsp), %r15
	jmp	.LBB968_309
.LBB968_308:
	leaq	8(%rax), %rsi
	shlq	$3, %rdx
.LBB968_309:
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB968_328
.LBB968_310:
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rdx, %rdi
	movq	%rsi, %r12
	movq	%rdx, %rbx
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB968_474
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rbx, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rbx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB968_313
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB968_313:
	movq	%r12, %rdx
	.p2align	4
.LBB968_314:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_320
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_314
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r8
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	%rbx, (%rsi)
	movq	%rbx, %rsi
	lock		xaddq	%rsi, (%r8)
	addq	%rbx, %rsi
	cmovoq	%rax, %rsi
	movq	(%rdi), %rax
	.p2align	4
.LBB968_317:
	cmpq	%rax, %rsi
	jle	.LBB968_319
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdi
	lock		cmpxchgq	%rsi, (%rdi)
	jne	.LBB968_317
.LBB968_319:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_320:
	movb	$61, %al
	leaq	-2(%r14), %rdi
	bzhiq	%rax, %r15, %rax
	cmpq	%rdi, %rax
	cmovbq	%rax, %rdi
	cmpq	$16, %rdi
	jae	.LBB968_322
	movq	%r15, %rax
	movq	%rdx, %rsi
	xorl	%edi, %edi
	jmp	.LBB968_324
.LBB968_322:
	incq	%rdi
	movl	$16, %esi
	movl	%edi, %eax
	andl	$15, %eax
	cmoveq	%rsi, %rax
	xorl	%r8d, %r8d
	subq	%rax, %rdi
	movq	%r15, %rax
	leaq	(%rdx,%rdi,8), %rsi
	subq	%rdi, %rax
	.p2align	4
.LBB968_323:
	vmovdqu64	(%rdx,%r8,8), %zmm0
	vmovdqu64	64(%rdx,%r8,8), %zmm1
	vmovdqu64	%zmm1, 64(%rcx,%r8,8)
	vmovdqu64	%zmm0, (%rcx,%r8,8)
	addq	$16, %r8
	cmpq	%r8, %rdi
	jne	.LBB968_323
.LBB968_324:
	leaq	(%rdx,%r15,8), %rdx
	leaq	4(%rcx,%rdi,8), %rdi
	xorl	%r8d, %r8d
	.p2align	4
.LBB968_325:
	cmpq	%rdx, %rsi
	je	.LBB968_327
	movl	(%rsi), %r9d
	movl	4(%rsi), %r10d
	addq	$8, %rsi
	movl	%r9d, -4(%rdi,%r8,8)
	movl	%r10d, (%rdi,%r8,8)
	incq	%r8
	cmpq	%r8, %rax
	jne	.LBB968_325
.LBB968_327:
	movq	40(%rsp), %r15
	movq	%rcx, 224(%rsp)
	movq	%r14, 232(%rsp)
.LBB968_328:
	vmovups	416(%rsp), %ymm0
	leaq	88(%rsp), %rcx
	movq	224(%rsp), %rax
	movq	232(%rsp), %rdx
	vmovups	%ymm0, 32(%rcx)
	vmovups	240(%rsp), %xmm0
	movq	%r14, 80(%rsp)
	movq	%rax, (%rcx)
	movq	%rdx, 8(%rcx)
	vmovups	%xmm0, 16(%rcx)
	movq	24(%rsp), %rcx
	cmpq	584(%rsp), %rcx
	jne	.LBB968_201
.Ltmp13084:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::governor::soundness::NodeAnalysis>>::grow_one@GOTPCREL(%rip), %rax
	leaq	584(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp13085:
	movq	592(%rsp), %rax
	movq	%rax, 64(%rsp)
	jmp	.LBB968_201
.LBB968_331:
	leaq	88(%rsp), %rcx
	vmovdqu64	8(%rcx), %zmm0
	vmovdqu64	24(%rcx), %zmm1
	leaq	240(%rsp), %rcx
	vmovdqu64	%zmm1, 32(%rcx)
	vmovdqu64	%zmm0, 16(%rcx)
	movq	%rax, 240(%rsp)
	movq	%r14, 248(%rsp)
	movq	$1, 224(%rsp)
.Ltmp13011:
	leaq	1232(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp13012:
	movq	192(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB968_334
	movq	32(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB968_334:
.Ltmp13014:
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp13015:
	testq	%r15, %r15
	je	.LBB968_338
	lock		decq	(%r15)
	jne	.LBB968_338
	#MEMBARRIER
.Ltmp13016:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	376(%rsp), %rdi
	callq	*%rax
.Ltmp13017:
.LBB968_338:
.Ltmp13019:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp13020:
	movq	40(%rsp), %r15
	cmpq	$0, 344(%rsp)
	movq	336(%rsp), %rax
	je	.LBB968_247
	lock		decq	(%rax)
	jne	.LBB968_247
	#MEMBARRIER
.Ltmp13021:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	760(%rsp), %rdi
	callq	*%rax
.Ltmp13022:
	jmp	.LBB968_247
.LBB968_342:
	movb	$1, %al
	testb	%al, %al
	je	.LBB968_210
.LBB968_343:
.Ltmp12977:
	movq	purrdf_stack::is_low_cold@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp12978:
	testb	%al, %al
	je	.LBB968_210
	movq	1072(%rsp), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.432(%rip), %rcx
	movq	%rax, 240(%rsp)
	movq	%rcx, 248(%rsp)
	movq	$41, 256(%rsp)
	jmp	.LBB968_213
.LBB968_346:
	movq	40(%rsp), %r15
	movl	$8, %eax
	movq	$0, 584(%rsp)
	movq	$8, 592(%rsp)
	movq	$0, 600(%rsp)
	movq	$0, 24(%rsp)
	movq	%rax, 64(%rsp)
	jmp	.LBB968_350
.LBB968_347:
	vmovups	416(%rsp), %ymm0
	vmovups	800(%rsp), %ymm2
	vmovups	824(%rsp), %ymm1
	movq	%rax, 80(%rsp)
	vmovups	%ymm0, 88(%rsp)
	vmovups	%ymm2, 120(%rsp)
	vmovups	%ymm1, 144(%rsp)
.Ltmp13071:
	leaq	224(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	leaq	80(%rsp), %rcx
	movl	$1, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp13072:
	cmpq	$-1, 224(%rsp)
	je	.LBB968_350
.Ltmp13073:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp13074:
.LBB968_350:
.Ltmp13098:
	movq	<purrdf_sparql_eval::solution::VarSchema>::union@GOTPCREL(%rip), %rax
	movq	352(%rsp), %rsi
	leaq	224(%rsp), %rdi
	leaq	992(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp13099:
	vmovdqu	224(%rsp), %ymm0
	vmovdqu	248(%rsp), %ymm1
	leaq	96(%rsp), %rbx
	movq	$1, 80(%rsp)
	movq	$1, 88(%rsp)
	vmovdqu	%ymm0, 96(%rsp)
	vmovdqu	%ymm1, 120(%rsp)
.Ltmp13101:
	movl	$8, %edi
	movl	$72, %esi
	vzeroupper
	callq	alloc::boxed::box_new_uninit
.Ltmp13102:
	movq	144(%rsp), %rcx
	movq	%rax, %rsi
	movq	%rsi, 32(%rsp)
	movq	%rcx, 64(%rsi)
	vmovdqu64	80(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rsi)
	movq	%rsi, 704(%rsp)
	movq	32(%rsi), %rdi
	testq	%rdi, %rdi
	je	.LBB968_357
	movq	616(%r15), %rax
	testq	%rax, %rax
	je	.LBB968_357
	movq	32(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB968_357
	movq	%rax, %rdx
	orq	%rdi, %rdx
	movabsq	$230584300921369396, %rcx
	shrq	$32, %rdx
	je	.LBB968_450
	xorl	%edx, %edx
	divq	%rdi
	jmp	.LBB968_451
.LBB968_357:
	movq	24(%rsp), %r15
	movq	64(%rsp), %rbp
	movl	$0, 200(%rsp)
.LBB968_358:
	movq	%r15, %r14
	movq	16(%rsp), %r13
	movq	%rdi, 72(%rsp)
	testq	%r14, %r14
	je	.LBB968_362
.LBB968_360:
	leaq	(,%r14,8), %rax
	movl	$8, %esi
	leaq	(%rax,%rax,4), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB968_475
	movq	%rax, %rdx
	jmp	.LBB968_363
.LBB968_440:
	leaq	240(%rsp), %rax
	vmovups	32(%rax), %zmm1
	vmovups	(%rax), %zmm0
	vmovups	%zmm1, 496(%rsp)
	vmovups	%zmm0, 464(%rsp)
	vmovups	464(%rsp), %zmm0
	vmovups	496(%rsp), %zmm1
	jmp	.LBB968_442
.LBB968_441:
	leaq	88(%rsp), %rax
	vmovups	40(%rax), %zmm1
	vmovups	8(%rax), %zmm0
	vmovups	%zmm1, 256(%rsp)
	vmovups	%zmm0, 224(%rsp)
	vmovdqu64	224(%rsp), %zmm0
	vmovdqu64	256(%rsp), %zmm1
.LBB968_442:
	movq	56(%rsp), %rax
	movb	$1, %bpl
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
.Ltmp13090:
	movq	16(%rsp), %r13
	leaq	584(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp13091:
	movb	$1, %bpl
.Ltmp13092:
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp13093:
	lock		decq	(%r13)
	jne	.LBB968_446
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp13094:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	696(%rsp), %rdi
	callq	*%rax
.Ltmp13095:
.LBB968_446:
	cmpq	$0, 560(%rsp)
	je	.LBB968_448
	movb	$1, %bpl
.Ltmp13096:
	leaq	560(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp13097:
.LBB968_448:
	movb	$1, %bpl
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	movl	%ebp, 48(%rsp)
	jne	.LBB968_31
.LBB968_449:
	leaq	408(%rsp), %rdi
	#MEMBARRIER
.Ltmp13185:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp13186:
	jmp	.LBB968_31
.LBB968_450:
	xorl	%edx, %edx
	divl	%edi
.LBB968_451:
	movq	64(%rsp), %rbp
	movq	24(%rsp), %r15
	movq	%rax, 216(%rsp)
	cmpq	%rcx, %rax
	jae	.LBB968_453
	cmpq	%r15, %rax
	movq	%r15, %r14
	cmovbq	%rax, %r14
	movb	$1, %al
	movl	%eax, 200(%rsp)
	movq	16(%rsp), %r13
	movq	%rdi, 72(%rsp)
	testq	%r14, %r14
	jne	.LBB968_360
.LBB968_362:
	movl	$8, %edx
.LBB968_363:
	movq	%r14, 608(%rsp)
	movq	%rdx, 616(%rsp)
	movq	$0, 624(%rsp)
	testq	%r15, %r15
	je	.LBB968_432
	leaq	88(%rsp), %rax
	movq	32(%rsp), %rsi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.428(%rip), %rcx
	movq	$0, 24(%rsp)
	movq	%rax, 456(%rsp)
	leaq	(%r15,%r15,8), %rax
	movq	%rcx, 192(%rsp)
	leaq	(%rbp,%rax,8), %rax
	movq	%rax, 336(%rsp)
	movq	208(%rsp), %rax
	addq	$16, %rsi
	movq	%rsi, 344(%rsp)
	leaq	(,%rax,8), %rax
	movq	%rax, 352(%rsp)
	jmp	.LBB968_368
.LBB968_365:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_366:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	%rbx, %rdx
.LBB968_367:
	addq	$72, %rbp
	cmpq	336(%rsp), %rbp
	je	.LBB968_432
.LBB968_368:
	movq	64(%rbp), %rsi
	movq	%rdx, %r13
	addq	$16, %rsi
.Ltmp13110:
	movq	344(%rsp), %rdx
	movq	purrdf_sparql_eval::binop::right_to_out_map@GOTPCREL(%rip), %rax
	leaq	800(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp13111:
	movq	56(%rbp), %rax
	movq	%rbp, %r15
	testq	%rax, %rax
	je	.LBB968_420
	movq	48(%rbp), %r12
	movq	%r13, %rdx
	movq	808(%rsp), %r14
	movq	816(%rsp), %rbx
	movq	16(%rsp), %r13
	leaq	(%rax,%rax,4), %rax
	leaq	8(%rbp), %rcx
	movq	%rcx, 48(%rsp)
	leaq	(%r12,%rax,8), %rax
	movq	%rax, 360(%rsp)
	jmp	.LBB968_373
	.p2align	4
.LBB968_371:
	movq	24(%rsp), %rsi
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 24(%rsp)
	movq	%r13, (%rdx,%rax,8)
	movq	%rbp, 8(%rdx,%rax,8)
	movq	16(%rsp), %r13
	vmovdqa	80(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	96(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 624(%rsp)
	addq	$40, %r12
	cmpq	360(%rsp), %r12
	je	.LBB968_421
.LBB968_373:
	movq	(%r12), %rcx
	leaq	8(%r12), %rbp
	movq	%rdx, 64(%rsp)
	movq	%rbp, %rax
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB968_375
	movq	16(%r12), %rcx
	movq	8(%r12), %rax
	decq	%rcx
.LBB968_375:
	testq	%rcx, %rcx
	je	.LBB968_387
	leaq	(%rax,%rcx,8), %rcx
	xorl	%edx, %edx
	jmp	.LBB968_378
	.p2align	4
.LBB968_377:
	leaq	(%rax,%rdx,8), %rsi
	incq	%rdx
	addq	$8, %rsi
	cmpq	%rcx, %rsi
	je	.LBB968_387
.LBB968_378:
	cmpq	%rdx, %rbx
	je	.LBB968_470
	movq	(%r15), %r8
	movq	48(%rsp), %rsi
	decq	%r8
	cmpq	$4, %r8
	jbe	.LBB968_381
	movq	16(%r15), %r8
	movq	8(%r15), %rsi
	decq	%r8
.LBB968_381:
	movq	(%r14,%rdx,8), %rdi
	cmpq	%r8, %rdi
	jae	.LBB968_377
	movl	(%rax,%rdx,8), %r8d
	cmpl	$2, %r8d
	je	.LBB968_377
	movl	(%rsi,%rdi,8), %r9d
	cmpl	$2, %r9d
	je	.LBB968_377
	cmpl	%r9d, %r8d
	jne	.LBB968_417
	movl	4(%rsi,%rdi,8), %esi
	cmpl	%esi, 4(%rax,%rdx,8)
	je	.LBB968_377
.LBB968_417:
	movq	64(%rsp), %rdx
	addq	$40, %r12
	cmpq	360(%rsp), %r12
	jne	.LBB968_373
	jmp	.LBB968_421
	.p2align	4
.LBB968_387:
	cmpb	$0, 200(%rsp)
	je	.LBB968_389
	movq	24(%rsp), %rax
	cmpq	216(%rsp), %rax
	jae	.LBB968_429
.LBB968_389:
	movq	72(%rsp), %rdx
	movq	$1, 80(%rsp)
	cmpq	$5, %rdx
	jae	.LBB968_418
	vmovdqu	88(%rsp), %xmm0
	movq	112(%rsp), %rax
	movq	80(%rsp), %rsi
	movq	104(%rsp), %rcx
	movq	%rax, 256(%rsp)
	movq	%rsi, 224(%rsp)
	movq	%rcx, 248(%rsp)
	vmovdqu	%xmm0, 232(%rsp)
	testq	%rdx, %rdx
	je	.LBB968_392
.LBB968_391:
	movl	$2, %eax
	jmp	.LBB968_393
	.p2align	4
.LBB968_392:
	movl	$-1, %eax
.LBB968_393:
	movl	%eax, 80(%rsp)
	movq	%rdx, 88(%rsp)
.Ltmp13121:
	leaq	224(%rsp), %rdi
	leaq	80(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp13122:
	vmovdqu	224(%rsp), %ymm0
	movq	256(%rsp), %rax
	leaq	472(%rsp), %rdi
	movq	%rax, 496(%rsp)
	vmovdqu	%ymm0, 464(%rsp)
	movq	464(%rsp), %r13
	movq	%r13, %rdx
	cmpq	$6, %r13
	jb	.LBB968_396
	movq	472(%rsp), %rdi
	movq	480(%rsp), %rdx
.LBB968_396:
	decq	%rdx
	cmpq	%rdx, 208(%rsp)
	ja	.LBB968_454
	movq	(%r15), %rsi
	movq	48(%rsp), %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB968_399
	movq	16(%r15), %rsi
	movq	8(%r15), %rax
	decq	%rsi
.LBB968_399:
	cmpq	%rsi, 208(%rsp)
	jne	.LBB968_457
	movq	%rax, %rsi
	movq	352(%rsp), %rdx
	movq	memcpy@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	(%r12), %rax
	decq	%rax
	cmpq	$5, %rax
	jb	.LBB968_402
	movq	16(%r12), %rax
	movq	8(%r12), %rbp
	decq	%rax
.LBB968_402:
	testq	%rax, %rax
	je	.LBB968_414
	shlq	$3, %rax
	leaq	472(%rsp), %r9
	xorl	%edi, %edi
	jmp	.LBB968_406
	.p2align	4
.LBB968_404:
	movl	4(%rbp,%rdi,8), %esi
	movl	%ecx, (%r8,%rdx,8)
	movl	%esi, 4(%r8,%rdx,8)
.LBB968_405:
	incq	%rdi
	addq	$-8, %rax
	je	.LBB968_413
.LBB968_406:
	movl	(%rbp,%rdi,8), %ecx
	cmpl	$2, %ecx
	je	.LBB968_405
	cmpq	%rbx, %rdi
	jae	.LBB968_472
	movq	464(%rsp), %rsi
	movq	%rsi, %r8
	cmpq	$6, %rsi
	jb	.LBB968_410
	movq	480(%rsp), %r8
.LBB968_410:
	movq	(%r14,%rdi,8), %rdx
	decq	%r8
	cmpq	%r8, %rdx
	jae	.LBB968_471
	movq	%r9, %r8
	cmpq	$6, %rsi
	jb	.LBB968_404
	movq	472(%rsp), %r8
	jmp	.LBB968_404
	.p2align	4
.LBB968_413:
	movq	464(%rsp), %r13
.LBB968_414:
	leaq	472(%rsp), %rax
	movq	472(%rsp), %rbp
	movq	64(%rsp), %rdx
	movq	24(%rsp), %rcx
	vmovdqu	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 96(%rsp)
	vmovdqa	%xmm0, 80(%rsp)
	cmpq	608(%rsp), %rcx
	jne	.LBB968_371
.Ltmp13128:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	608(%rsp), %rdi
	callq	*%rax
.Ltmp13129:
	movq	616(%rsp), %rdx
	jmp	.LBB968_371
.LBB968_418:
.Ltmp13118:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp13119:
	vmovdqu	80(%rsp), %ymm0
	movq	112(%rsp), %rax
	movq	72(%rsp), %rdx
	movq	%rax, 256(%rsp)
	vmovdqu	%ymm0, 224(%rsp)
	jmp	.LBB968_391
.LBB968_420:
	movq	%r13, %rdx
	movq	16(%rsp), %r13
.LBB968_421:
	movq	800(%rsp), %rcx
	movq	%r15, %rbp
	testq	%rcx, %rcx
	je	.LBB968_367
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	movq	808(%rsp), %rdi
	movq	%rdx, %rbx
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_424
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_424:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_366
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_424
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
.LBB968_427:
	cmpq	%rax, %rdx
	jge	.LBB968_365
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB968_427
	jmp	.LBB968_365
.LBB968_429:
	movq	24(%rsp), %rdx
	incq	%rdx
.Ltmp13115:
	movq	40(%rsp), %rsi
	movq	72(%rsp), %rcx
	leaq	80(%rsp), %rdi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::observe_cells
.Ltmp13116:
	movq	800(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB968_432
	shlq	$3, %rsi
	movl	$8, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB968_432:
	movq	608(%rsp), %rcx
	movq	624(%rsp), %rax
	movq	616(%rsp), %r8
	movq	%rcx, 224(%rsp)
	movq	32(%rsp), %rcx
	movq	%rax, 240(%rsp)
	movq	%r8, 232(%rsp)
	movq	%rcx, 248(%rsp)
.Ltmp13136:
	leaq	80(%rsp), %rdi
	leaq	1104(%rsp), %rsi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
.Ltmp13137:
	vmovdqu64	80(%rsp), %zmm0
	vmovdqu64	112(%rsp), %zmm1
	movq	56(%rsp), %rax
	xorl	%ebp, %ebp
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp13141:
	leaq	584(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp13142:
	xorl	%ebp, %ebp
.Ltmp13143:
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp13144:
	lock		decq	(%r13)
	jne	.LBB968_437
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp13146:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	696(%rsp), %rdi
	callq	*%rax
.Ltmp13147:
.LBB968_437:
	cmpq	$0, 560(%rsp)
	je	.LBB968_439
	xorl	%ebp, %ebp
.Ltmp13148:
	leaq	560(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp13149:
.LBB968_439:
	xorl	%ebp, %ebp
.Ltmp13150:
	leaq	384(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp13151:
	jmp	.LBB968_78
.LBB968_453:
	movl	$0, 200(%rsp)
	jmp	.LBB968_358
.LBB968_454:
.Ltmp13131:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	movq	208(%rsp), %rsi
	movq	16(%rsp), %r13
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.430(%rip), %rcx
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
.Ltmp13132:
	jmp	.LBB968_477
.LBB968_455:
	movl	$84, %edi
	movl	$1, %esi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB968_476
	vmovdqu64	.Lanon.a12f493ba210922c94e5446ac885c35e.431+20(%rip), %zmm0
	vmovdqu64	.Lanon.a12f493ba210922c94e5446ac885c35e.431(%rip), %zmm1
	movq	56(%rsp), %rdx
	movabsq	$9223372036854775792, %rcx
	addq	$25, %rcx
	movq	%rcx, 16(%rdx)
	movq	$84, 24(%rdx)
	movq	%rax, 32(%rdx)
	movq	$84, 40(%rdx)
	movq	$1, (%rdx)
	vmovdqu64	%zmm0, 20(%rax)
	vmovdqu64	%zmm1, (%rax)
	jmp	.LBB968_448
.LBB968_457:
.Ltmp13124:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rax
	movq	208(%rsp), %rdi
	movq	16(%rsp), %r13
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.427(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp13125:
	jmp	.LBB968_477
.LBB968_458:
	movq	(%rbx), %rax
	movq	%r14, 24(%rsp)
	movq	%r13, %r14
	leaq	1(%r12), %r13
	lock		decq	(%rax)
	jne	.LBB968_460
	#MEMBARRIER
.Ltmp12929:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp12930:
.LBB968_460:
	cmpq	%r15, %r13
	jae	.LBB968_468
	addq	$24, %rbx
	jmp	.LBB968_464
.LBB968_462:
	movq	16(%rbx), %rcx
	movq	24(%rsp), %rdx
	leaq	(%r12,%r12,2), %rax
	incq	%r12
	movq	%rcx, 16(%rdx,%rax,8)
	vmovdqu	(%rbx), %xmm0
	vmovdqu	%xmm0, (%rdx,%rax,8)
.LBB968_463:
	incq	%r13
	addq	$24, %rbx
	cmpq	%r15, %r13
	jae	.LBB968_468
.LBB968_464:
.Ltmp12932:
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	<alloc::vec::Vec<(purrdf_sparql_algebra::ast::Variable, bool)>>::retain::<purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>::{closure#0}
.Ltmp12933:
	testb	%al, %al
	jne	.LBB968_462
	movq	(%rbx), %rax
	lock		decq	(%rax)
	jne	.LBB968_463
	#MEMBARRIER
.Ltmp12935:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp12936:
	jmp	.LBB968_463
.LBB968_468:
	movq	%r14, %r13
	movq	24(%rsp), %r14
	movq	%r12, %r15
	movq	%r12, 960(%rsp)
	jmp	.LBB968_113
.LBB968_469:
	movb	$1, %bpl
.Ltmp13161:
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522(%rip), %rdi
	leaq	anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522(%rip), %rdx
	movl	$48, %esi
	vzeroupper
	callq	*%rax
.Ltmp13162:
	jmp	.LBB968_477
.LBB968_470:
.Ltmp13113:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.643(%rip), %rdx
	movq	%rbx, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp13114:
	jmp	.LBB968_477
.LBB968_471:
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.429(%rip), %rax
	movq	%rdx, %rdi
	movq	%r8, %rbx
	movq	%rax, 192(%rsp)
.LBB968_472:
.Ltmp13126:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	192(%rsp), %rdx
	movq	16(%rsp), %r13
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp13127:
	jmp	.LBB968_477
.LBB968_473:
	movq	%r14, 712(%rsp)
.Ltmp12957:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.627(%rip), %rdx
	movq	%r12, %rdi
	callq	*%rax
.Ltmp12958:
	jmp	.LBB968_477
.LBB968_474:
.Ltmp13079:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp13080:
	jmp	.LBB968_477
.LBB968_475:
.Ltmp13107:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp13108:
	jmp	.LBB968_477
.LBB968_476:
	movb	$1, %bpl
.Ltmp13159:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$1, %edi
	movl	$84, %esi
	callq	*%rax
.Ltmp13160:
.LBB968_477:
	ud2
.LBB968_478:
.Ltmp12931:
	jmp	.LBB968_484
.LBB968_479:
.Ltmp13120:
	movq	%rax, %rbx
	movq	80(%rsp), %rax
	movq	32(%rsp), %r14
	cmpq	$6, %rax
	jae	.LBB968_553
	jmp	.LBB968_564
.LBB968_480:
.Ltmp12937:
	movq	%rax, %rbx
	incq	%r13
	jmp	.LBB968_485
.LBB968_481:
.Ltmp13018:
	jmp	.LBB968_506
.LBB968_482:
.Ltmp12947:
	movq	%rax, %rbx
.Ltmp12948:
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.13412714042204560522)
.Ltmp12949:
	jmp	.LBB968_545
.LBB968_483:
.Ltmp12934:
.LBB968_484:
	movq	%rax, %rbx
.LBB968_485:
	movq	24(%rsp), %rcx
	leaq	(%r13,%r13,2), %rax
	subq	%r13, %r15
	movq	memmove@GOTPCREL(%rip), %r14
	leaq	(%rcx,%rax,8), %rsi
	leaq	(%r12,%r12,2), %rax
	leaq	(%rcx,%rax,8), %rdi
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,2), %rdx
	callq	*%r14
	addq	%r12, %r15
	movq	%r15, 960(%rsp)
	jmp	.LBB968_550
.LBB968_486:
.Ltmp13013:
	movq	%rax, %r13
	jmp	.LBB968_521
.LBB968_487:
.Ltmp12971:
	movq	24(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 712(%rsp)
	jmp	.LBB968_547
.LBB968_488:
.Ltmp12966:
	movq	%rax, %rbx
	jmp	.LBB968_547
.LBB968_489:
.Ltmp13075:
	jmp	.LBB968_523
.LBB968_490:
.Ltmp13109:
	movq	32(%rsp), %r14
	movq	%rax, %rbx
	jmp	.LBB968_567
.LBB968_491:
.Ltmp12985:
	movq	%rax, %rbx
.Ltmp12986:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp12987:
	movq	16(%rsp), %r13
	jmp	.LBB968_540
.LBB968_492:
.Ltmp13138:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
	jmp	.LBB968_569
.LBB968_493:
.Ltmp13047:
	movq	%rax, %rbx
.Ltmp13048:
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
.Ltmp13049:
	jmp	.LBB968_534
.LBB968_494:
.Ltmp13103:
	movq	%rax, %r14
.Ltmp13104:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp13105:
	movq	16(%rsp), %r13
	movb	$1, %bpl
	movq	%r14, %rbx
	jmp	.LBB968_569
.LBB968_496:
.Ltmp13106:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB968_497:
.Ltmp13100:
	jmp	.LBB968_523
.LBB968_498:
.Ltmp12925:
	jmp	.LBB968_549
.LBB968_499:
.Ltmp12994:
	movq	%rax, %rbx
.Ltmp12995:
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp12996:
	movq	16(%rsp), %r13
	jmp	.LBB968_540
.LBB968_500:
.Ltmp13057:
	movq	%rax, %rbx
	jmp	.LBB968_535
.LBB968_501:
.Ltmp13145:
	movq	%rax, %rbx
	jmp	.LBB968_572
.LBB968_502:
.Ltmp13182:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
	jmp	.LBB968_576
.LBB968_503:
.Ltmp12942:
	movq	24(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 712(%rsp)
	jmp	.LBB968_545
.LBB968_504:
.Ltmp13187:
	leaq	384(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB968_582
.LBB968_505:
.Ltmp13052:
.LBB968_506:
	movq	%rax, %rbx
	jmp	.LBB968_534
.LBB968_507:
.Ltmp13081:
	jmp	.LBB968_557
.LBB968_508:
.Ltmp13177:
	movq	%rax, %rbx
	movb	$1, %bpl
	jmp	.LBB968_576
.LBB968_509:
.Ltmp13030:
	movq	%rax, %r13
	movq	%rbx, 376(%rsp)
	movq	%rbx, %r15
	jmp	.LBB968_519
.LBB968_510:
.Ltmp13040:
	movq	%rax, %rbx
	jmp	.LBB968_531
.LBB968_511:
.Ltmp13112:
	movq	32(%rsp), %r14
	movq	16(%rsp), %r13
	movq	%rax, %rbx
	jmp	.LBB968_566
.LBB968_512:
.Ltmp12922:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
	jmp	.LBB968_578
.LBB968_513:
.Ltmp13062:
	movq	16(%rsp), %r13
	movq	%rax, %rbx
	jmp	.LBB968_540
.LBB968_514:
.Ltmp13086:
	movq	%rax, %rbx
.Ltmp13087:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>
.Ltmp13088:
	jmp	.LBB968_524
.LBB968_515:
.Ltmp13089:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB968_516:
.Ltmp13035:
	movl	$144, %esi
	movl	$8, %edx
	movq	%r12, %rdi
	movq	%rax, %r13
	callq	__rustc::__rust_dealloc
	movq	%r14, 368(%rsp)
	movq	%rbx, %r15
	jmp	.LBB968_521
.LBB968_517:
.Ltmp12954:
	movq	%rax, %rbx
	movq	%r14, 712(%rsp)
	jmp	.LBB968_545
.LBB968_518:
.Ltmp13027:
	movq	%rax, %r13
.LBB968_519:
.Ltmp13031:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>
.Ltmp13032:
	jmp	.LBB968_521
.LBB968_520:
.Ltmp13008:
	movq	%rax, %r13
.Ltmp13009:
	leaq	1232(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
.Ltmp13010:
.LBB968_521:
	cmpq	$0, 192(%rsp)
	movq	%r13, %rbx
	jne	.LBB968_529
	jmp	.LBB968_530
.LBB968_522:
.Ltmp13070:
.LBB968_523:
	movq	%rax, %rbx
.LBB968_524:
	movq	16(%rsp), %r13
	movb	$1, %bpl
	jmp	.LBB968_569
.LBB968_525:
.Ltmp13174:
	movq	%rax, %rbx
	jmp	.LBB968_576
.LBB968_526:
.Ltmp13005:
	movq	80(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 192(%rsp)
	testq	%rcx, %rcx
	jne	.LBB968_528
	xorl	%r15d, %r15d
	jmp	.LBB968_530
.LBB968_528:
	movq	88(%rsp), %rax
	xorl	%r15d, %r15d
	movq	%rax, 32(%rsp)
.LBB968_529:
	movq	192(%rsp), %rsi
	movq	32(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rsi
	callq	__rustc::__rust_dealloc
.LBB968_530:
.Ltmp13036:
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
.Ltmp13037:
.LBB968_531:
	testq	%r15, %r15
	je	.LBB968_534
	lock		decq	(%r15)
	jne	.LBB968_534
	#MEMBARRIER
.Ltmp13041:
	movq	<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	376(%rsp), %rdi
	callq	*%rax
.Ltmp13042:
.LBB968_534:
.Ltmp13053:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
.Ltmp13054:
.LBB968_535:
	cmpq	$0, 344(%rsp)
	je	.LBB968_539
	movq	336(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB968_539
	#MEMBARRIER
.Ltmp13058:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	760(%rsp), %rdi
	callq	*%rax
.Ltmp13059:
	movq	16(%rsp), %r13
	jmp	.LBB968_540
.LBB968_539:
	movq	16(%rsp), %r13
.LBB968_540:
	movb	$1, %bpl
.Ltmp13063:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
.Ltmp13064:
	jmp	.LBB968_569
.LBB968_541:
.Ltmp13130:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB968_543
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB968_543:
	movq	16(%rsp), %r13
	jmp	.LBB968_559
.LBB968_544:
.Ltmp12961:
	movq	%rax, %rbx
.LBB968_545:
	movq	416(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB968_547
	#MEMBARRIER
.Ltmp12962:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	416(%rsp), %rdi
	callq	*%rax
.Ltmp12963:
.LBB968_547:
	movb	$1, %bpl
.Ltmp12972:
	leaq	704(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::service_endpoints::Occurrence>>
.Ltmp12973:
	jmp	.LBB968_576
.LBB968_548:
.Ltmp12928:
.LBB968_549:
	movq	%rax, %rbx
.LBB968_550:
	movb	$1, %bpl
.Ltmp12938:
	leaq	944(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::service_endpoints::Occurrence>>
.Ltmp12939:
	jmp	.LBB968_576
.LBB968_551:
.Ltmp12974:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB968_552:
.Ltmp13123:
	movq	%rax, %rbx
	movq	224(%rsp), %rax
	movq	16(%rsp), %r13
	movq	32(%rsp), %r14
	leaq	232(%rsp), %rcx
	movq	%rcx, 456(%rsp)
	cmpq	$5, %rax
	jbe	.LBB968_564
.LBB968_553:
	movq	456(%rsp), %rcx
	movq	(%rcx), %rdi
	jmp	.LBB968_563
.LBB968_554:
.Ltmp13065:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB968_555:
.Ltmp13190:
	movq	%rax, %rbx
	jmp	.LBB968_582
.LBB968_556:
.Ltmp13078:
.LBB968_557:
	movb	$1, %bpl
	movq	%rax, %rbx
.Ltmp13082:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp13083:
	movq	16(%rsp), %r13
	jmp	.LBB968_569
.LBB968_558:
.Ltmp13117:
	movq	%rax, %rbx
.LBB968_559:
	movq	32(%rsp), %r14
	jmp	.LBB968_564
.LBB968_560:
.Ltmp13163:
	movq	%rax, %rbx
	jmp	.LBB968_574
.LBB968_561:
.Ltmp13133:
	movq	%rax, %rbx
	movq	464(%rsp), %rax
	movq	32(%rsp), %r14
	cmpq	$6, %rax
	jb	.LBB968_564
	movq	472(%rsp), %rdi
.LBB968_563:
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB968_564:
	movq	800(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB968_566
	movq	808(%rsp), %rdi
	shlq	$3, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB968_566:
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB968_567:
	lock		decq	(%r14)
	movb	$1, %bpl
	jne	.LBB968_569
	#MEMBARRIER
.Ltmp13134:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	704(%rsp), %rdi
	callq	*%rax
.Ltmp13135:
.LBB968_569:
.Ltmp13139:
	leaq	584(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
.Ltmp13140:
	jmp	.LBB968_571
.LBB968_570:
.Ltmp13154:
	movq	%rax, %rbx
.LBB968_571:
.Ltmp13155:
	leaq	992(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp13156:
.LBB968_572:
	lock		decq	(%r13)
	jne	.LBB968_574
	#MEMBARRIER
.Ltmp13157:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	696(%rsp), %rdi
	callq	*%rax
.Ltmp13158:
.LBB968_574:
	cmpq	$0, 560(%rsp)
	je	.LBB968_576
.Ltmp13164:
	leaq	560(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
.Ltmp13165:
.LBB968_576:
	movq	408(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB968_578
	leaq	408(%rsp), %rdi
	#MEMBARRIER
.Ltmp13183:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp13184:
.LBB968_578:
	movq	%rbx, 56(%rsp)
	movq	392(%rsp), %rbx
	movq	400(%rsp), %r14
	movl	%ebp, 48(%rsp)
	testq	%r14, %r14
	jne	.LBB968_585
.LBB968_579:
	movq	384(%rsp), %rax
	testq	%rax, %rax
	je	.LBB968_581
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB968_581:
	movq	56(%rsp), %rbx
	movl	48(%rsp), %ebp
.LBB968_582:
	testb	%bpl, %bpl
	je	.LBB968_584
.Ltmp13191:
	leaq	1104(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp13192:
.LBB968_584:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB968_585:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$9223372036854775807, %r12
	xorl	%r15d, %r15d
	jmp	.LBB968_589
	.p2align	4
.LBB968_586:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB968_587:
	callq	*%r13
.LBB968_588:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB968_579
.LBB968_589:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB968_588
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r12, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r12, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r12, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB968_592
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB968_592:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB968_587
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB968_592
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
.LBB968_595:
	cmpq	%rax, %rdx
	jge	.LBB968_586
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB968_595
	jmp	.LBB968_586
.LBB968_597:
.Ltmp13193:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end968:
purrdf_sparql_eval::modifier::eval_dedup::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin998:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception658
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
.Ltmp15881:
	leaq	640(%rsp), %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15882:
	cmpl	$1, 640(%rsp)
	jne	.LBB998_25
	vmovdqu64	688(%rsp), %zmm1
	vmovdqu64	656(%rsp), %zmm0
	movq	576(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rbx)
	vmovdqu64	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB998_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_5
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
.LBB998_8:
	cmpq	%rax, %rsi
	jge	.LBB998_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB998_8
.LBB998_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB998_12:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB998_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_15
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
.LBB998_18:
	cmpq	%rax, %rsi
	jge	.LBB998_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB998_18
.LBB998_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB998_22:
	movq	600(%rsp), %rax
	testq	%rax, %rax
	je	.LBB998_267
	lock		decq	(%rax)
	jne	.LBB998_267
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	600(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB998_267
.LBB998_25:
	vmovdqu64	680(%rsp), %zmm1
	vmovdqu64	648(%rsp), %zmm0
	vmovdqu64	%zmm1, 384(%rsp)
	vmovdqu64	%zmm0, 352(%rsp)
.Ltmp15883:
	leaq	112(%rsp), %rdi
	leaq	504(%rsp), %rsi
	leaq	352(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15884:
	cmpq	$-1, 112(%rsp)
	je	.LBB998_126
	vmovdqu	112(%rsp), %ymm0
	vmovdqu64	544(%rsp), %zmm1
	vmovdqu64	504(%rsp), %zmm2
	vmovdqu64	%zmm1, 392(%rsp)
	vmovdqu	%ymm0, 608(%rsp)
	vmovdqu64	%zmm2, 352(%rsp)
.Ltmp15888:
	leaq	320(%rsp), %rdi
	leaq	608(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::blank_scope::without_joined_blanks::<purrdf_core::ir::term::TermId>
.Ltmp15889:
	movq	336(%rsp), %r15
	movq	%rbx, 280(%rsp)
.Ltmp15891:
	leaq	112(%rsp), %rdi
	movl	$48, %esi
	movl	$1, %ecx
	movq	%r15, %rdx
	callq	<hashbrown::raw::RawTableInner>::fallible_with_capacity::<alloc::alloc::Global>
.Ltmp15892:
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
	je	.LBB998_99
	movq	(%rsp), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB998_34
.LBB998_31:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_32:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB998_33:
	cmpq	24(%rsp), %rbp
	je	.LBB998_112
.LBB998_34:
	movq	%rbp, %rax
	movq	(%rax), %rbx
	addq	$40, %rbp
	testq	%rbx, %rbx
	je	.LBB998_99
	vmovups	8(%rax), %ymm0
	leaq	472(%rsp), %rdx
	leaq	-1(%rbx), %rcx
	movabsq	$2746377873070565055, %rsi
	cmpq	$5, %rcx
	vmovups	%ymm0, (%rdx)
	movq	%rbx, 464(%rsp)
	vpbroadcastq	.LCPI998_6(%rip), %xmm0
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
	vaesenc	.LCPI998_1(%rip), %xmm0, %xmm0
	testq	%rax, %rax
	je	.LBB998_43
	leaq	(,%rax,8), %rsi
	addq	$-8, %rsi
	movl	%esi, %edi
	shrl	$3, %edi
	incl	%edi
	andl	$3, %edi
	je	.LBB998_41
	shll	$3, %edi
	movq	%rcx, %rdx
	jmp	.LBB998_39
	.p2align	4
.LBB998_38:
	addq	$8, %rdx
	addq	$-8, %rdi
	je	.LBB998_42
.LBB998_39:
	movl	(%rdx), %r8d
	xorl	%r9d, %r9d
	cmpq	$2, %r8
	setne	%r9b
	vmovd	%r9d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI998_1(%rip), %xmm0, %xmm0
	je	.LBB998_38
	vmovdqa	.LCPI998_1(%rip), %xmm2
	movl	4(%rdx), %r9d
	vmovq	%r8, %xmm1
	orl	$-2, %r8d
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%r8,%r9), %r8d
	vmovd	%r8d, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB998_38
	.p2align	4
.LBB998_41:
	movq	%rcx, %rdx
.LBB998_42:
	cmpq	$24, %rsi
	jae	.LBB998_84
.LBB998_43:
	vaesenc	.LCPI998_2(%rip), %xmm0, %xmm0
	leaq	1(%r15), %rdx
	movq	248(%rsp), %rsi
	movq	%rdx, 8(%rsp)
	movq	240(%rsp), %rdx
	vaesenc	.LCPI998_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %r12
	movq	%r12, %r14
	shrq	$57, %r14
	vpbroadcastb	%r14d, %xmm0
	testq	%rax, %rax
	je	.LBB998_62
	xorl	%edi, %edi
	movq	%r12, %r8
.LBB998_45:
	andq	%rsi, %r8
	vmovdqu	(%rdx,%r8), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB998_60
	kmovd	%k0, %r9d
	movq	%r13, 16(%rsp)
	movq	%rdi, 32(%rsp)
.LBB998_47:
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
	jb	.LBB998_49
	movq	-32(%rdx,%rdi), %r11
	movq	-40(%rdx,%rdi), %r10
	decq	%r11
	jmp	.LBB998_50
	.p2align	4
.LBB998_49:
	leaq	-40(%rdx,%rdi), %r10
.LBB998_50:
	cmpq	%rax, %r11
	jne	.LBB998_59
	xorl	%r11d, %r11d
	jmp	.LBB998_53
	.p2align	4
.LBB998_52:
	incq	%r11
	cmpq	%r11, %rax
	je	.LBB998_71
.LBB998_53:
	movl	(%r10,%r11,8), %r13d
	movl	(%rcx,%r11,8), %edi
	cmpl	$2, %r13d
	je	.LBB998_57
	cmpl	$2, %edi
	je	.LBB998_57
	cmpl	%edi, %r13d
	jne	.LBB998_59
	movl	4(%rcx,%r11,8), %edi
	cmpl	%edi, 4(%r10,%r11,8)
	je	.LBB998_52
	jmp	.LBB998_59
	.p2align	4
.LBB998_57:
	cmpl	$2, %r13d
	jne	.LBB998_59
	cmpl	$2, %edi
	je	.LBB998_52
.LBB998_59:
	leal	-1(%r9), %edi
	movq	16(%rsp), %r13
	andw	%r9w, %di
	movl	%edi, %r9d
	movq	32(%rsp), %rdi
	jne	.LBB998_47
	.p2align	4
.LBB998_60:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB998_80
	leaq	16(%r8,%rdi), %r8
	addq	$16, %rdi
	jmp	.LBB998_45
	.p2align	4
.LBB998_62:
	xorl	%eax, %eax
	movq	%r12, %rcx
.LBB998_63:
	andq	%rsi, %rcx
	vmovdqu	(%rdx,%rcx), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB998_69
	kmovd	%k0, %edi
	movq	%r13, 16(%rsp)
.LBB998_65:
	xorl	%r8d, %r8d
	tzcntl	%edi, %r8d
	addq	%rcx, %r8
	andq	%rsi, %r8
	negq	%r8
	leaq	(%r8,%r8,2), %r8
	shlq	$4, %r8
	movq	-48(%rdx,%r8), %r9
	cmpq	$6, %r9
	jb	.LBB998_67
	movq	-32(%rdx,%r8), %r9
.LBB998_67:
	cmpq	$1, %r9
	je	.LBB998_71
	movq	16(%rsp), %r13
	leal	-1(%rdi), %r8d
	andw	%di, %r8w
	movl	%r8d, %edi
	jne	.LBB998_65
	.p2align	4
.LBB998_69:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB998_80
	leaq	16(%rcx,%rax), %rcx
	addq	$16, %rax
	jmp	.LBB998_63
	.p2align	4
.LBB998_71:
	movq	8(%rsp), %r15
	cmpq	$6, %rbx
	jb	.LBB998_33
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_74
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB998_74:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r8
	movq	16(%rsp), %rdi
	.p2align	4
.LBB998_75:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_32
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_75
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
.LBB998_78:
	cmpq	%rax, %rdx
	jge	.LBB998_31
	lock		cmpxchgq	%rdx, (%r8)
	jne	.LBB998_78
	jmp	.LBB998_31
	.p2align	4
.LBB998_80:
	cmpq	$0, 256(%rsp)
	je	.LBB998_96
.LBB998_81:
	andq	%rsi, %r12
	vmovdqu	(%rdx,%r12), %xmm0
	vpmovmskb	%xmm0, %eax
	testl	%eax, %eax
	je	.LBB998_94
.LBB998_82:
	tzcntl	%eax, %eax
	addq	%r12, %rax
	andq	%rsi, %rax
	movzbl	(%rdx,%rax), %ecx
	testb	%cl, %cl
	jns	.LBB998_98
.LBB998_83:
	movq	40(%rsp), %r8
	leaq	-16(%rax), %rdi
	movb	%r14b, (%rdx,%rax)
	negq	%rax
	vpbroadcastb	.LCPI998_7(%rip), %xmm1
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
	jmp	.LBB998_33
	.p2align	4
.LBB998_84:
	leaq	(%rcx,%rax,8), %rsi
	jmp	.LBB998_86
	.p2align	4
.LBB998_85:
	addq	$32, %rdx
	cmpq	%rsi, %rdx
	je	.LBB998_43
.LBB998_86:
	movl	(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI998_1(%rip), %xmm0, %xmm0
	je	.LBB998_88
	vmovdqa	.LCPI998_1(%rip), %xmm2
	movl	4(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB998_88:
	movl	8(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI998_1(%rip), %xmm0, %xmm0
	je	.LBB998_90
	vmovdqa	.LCPI998_1(%rip), %xmm2
	movl	12(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB998_90:
	movl	16(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI998_1(%rip), %xmm0, %xmm0
	je	.LBB998_92
	vmovdqa	.LCPI998_1(%rip), %xmm2
	movl	20(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB998_92:
	movl	24(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI998_1(%rip), %xmm0, %xmm0
	je	.LBB998_85
	vmovdqa	.LCPI998_1(%rip), %xmm2
	movl	28(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB998_85
.LBB998_94:
	movl	$16, %ecx
.LBB998_95:
	addq	%rcx, %r12
	addq	$16, %rcx
	andq	%rsi, %r12
	vmovdqu	(%rdx,%r12), %xmm0
	vpmovmskb	%xmm0, %eax
	testl	%eax, %eax
	jne	.LBB998_82
	jmp	.LBB998_95
.LBB998_96:
.Ltmp15894:
	movq	<hashbrown::raw::RawTable<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize)>>::reserve_rehash::<hashbrown::map::make_hasher<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %esi
	leaq	240(%rsp), %rdi
	movl	$1, %ecx
	vzeroupper
	callq	*%rax
.Ltmp15895:
	movq	240(%rsp), %rdx
	movq	248(%rsp), %rsi
	jmp	.LBB998_81
.LBB998_98:
	vmovdqa	(%rdx), %xmm0
	vpmovmskb	%xmm0, %eax
	tzcntl	%eax, %eax
	movzbl	(%rdx,%rax), %ecx
	jmp	.LBB998_83
.LBB998_99:
	subq	%rbp, 24(%rsp)
	je	.LBB998_112
	movq	24(%rsp), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$-3689348814741910323, %rax
	movabsq	$9223372036854775807, %r14
	xorl	%ebx, %ebx
	shrq	$3, %r15
	imulq	%rax, %r15
	jmp	.LBB998_104
	.p2align	4
.LBB998_101:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_102:
	vzeroupper
	callq	*%r13
.LBB998_103:
	incq	%rbx
	cmpq	%r15, %rbx
	je	.LBB998_112
.LBB998_104:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB998_103
	leaq	(%rbp,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r14, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r14, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r14, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_107
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_107:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_107
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
.LBB998_110:
	cmpq	%rax, %rdx
	jge	.LBB998_101
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB998_110
	jmp	.LBB998_101
.LBB998_112:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB998_122
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_115
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_115:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_121
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_115
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
.LBB998_118:
	cmpq	%rax, %rsi
	jge	.LBB998_120
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB998_118
.LBB998_120:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_121:
	movq	free@GOTPCREL(%rip), %rax
	movq	(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB998_122:
	movq	240(%rsp), %r15
	movq	248(%rsp), %rdi
	movq	264(%rsp), %rax
	testq	%rdi, %rdi
	je	.LBB998_127
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
	je	.LBB998_128
.LBB998_124:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r15), %xmm0, %k0
	leaq	16(%r15), %r14
	kortestw	%k0, %k0
	je	.LBB998_141
	kmovd	%k0, %ecx
	movq	%r15, %rbp
	jmp	.LBB998_144
.LBB998_126:
	leaq	8(%rbx), %rdi
	leaq	504(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	jmp	.LBB998_266
.LBB998_127:
	xorl	%r8d, %r8d
	movq	%rsi, 40(%rsp)
	movq	%rdi, 8(%rsp)
	testq	%rax, %rax
	jne	.LBB998_124
.LBB998_128:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
.LBB998_129:
	cmpq	$0, 8(%rsp)
	movq	40(%rsp), %rsi
	movabsq	$-3689348814741910323, %r15
	je	.LBB998_140
	testq	%rsi, %rsi
	je	.LBB998_140
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_133
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_133:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_139
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_133
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
.LBB998_136:
	cmpq	%rax, %rdx
	jge	.LBB998_138
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB998_136
.LBB998_138:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_139:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB998_140:
	movl	$8, %r13d
	xorl	%r12d, %r12d
	jmp	.LBB998_214
.LBB998_141:
	movq	%r15, %rbp
	.p2align	4
.LBB998_142:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB998_142
	kmovd	%k0, %ecx
.LBB998_144:
	xorl	%r12d, %r12d
	blsrl	%ecx, %r12d
	tzcntl	%ecx, %ecx
	leaq	-1(%rax), %rbx
	negq	%rcx
	leaq	(%rcx,%rcx,2), %rcx
	shlq	$4, %rcx
	movq	-48(%rbp,%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB998_148
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
	jbe	.LBB998_164
	xorl	%edi, %edi
.LBB998_147:
.Ltmp15903:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15904:
	jmp	.LBB998_293
.LBB998_148:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
	testq	%rbx, %rbx
	je	.LBB998_129
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB998_153
	.p2align	4
.LBB998_150:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_151:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB998_152:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB998_129
.LBB998_153:
	testw	%r12w, %r12w
	jne	.LBB998_156
	.p2align	4
.LBB998_154:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB998_154
	kmovd	%k0, %r12d
.LBB998_156:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB998_152
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
	jge	.LBB998_159
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_151
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_159
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
.LBB998_162:
	cmpq	%rax, %rdx
	jge	.LBB998_150
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB998_162
	jmp	.LBB998_150
.LBB998_164:
	testq	%r13, %r13
	je	.LBB998_175
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rsi, 312(%rsp)
	movq	%r9, 208(%rsp)
	movq	%r8, (%rsp)
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB998_295
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rdx
	movabsq	$9223372036854775807, %rsi
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%r13, %rax
	cmovbq	%rdx, %rax
	cmpq	%rsi, %r13
	movq	%rsi, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmovbq	%r13, %rdx
	addq	%rdx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB998_168
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB998_168:
	movq	(%rsp), %r8
	movq	208(%rsp), %r9
	movq	312(%rsp), %r10
	.p2align	4
.LBB998_169:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_176
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_169
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
.LBB998_172:
	cmpq	%rax, %rsi
	jle	.LBB998_174
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB998_172
.LBB998_174:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jmp	.LBB998_176
.LBB998_175:
	movl	$8, %ecx
	xorl	%r10d, %r10d
.LBB998_176:
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
	je	.LBB998_202
	movl	$1, %r15d
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	jmp	.LBB998_179
	.p2align	4
.LBB998_178:
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
	je	.LBB998_202
.LBB998_179:
	testw	%r12w, %r12w
	jne	.LBB998_182
	.p2align	4
.LBB998_180:
	vpcmpltb	(%r14), %xmm1, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB998_180
	kmovd	%k0, %r12d
.LBB998_182:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	decq	%rbx
	blsrl	%r12d, %r12d
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %r13
	testq	%r13, %r13
	je	.LBB998_186
	addq	%rbp, %rax
	movq	-16(%rax), %rdx
	movq	-8(%rax), %rdi
	movq	-40(%rax), %rsi
	movq	%rdx, 96(%rsp)
	vmovups	-32(%rax), %xmm0
	vmovaps	%xmm0, 80(%rsp)
	cmpq	216(%rsp), %r15
	jne	.LBB998_178
	movq	%rbx, %rdx
	incq	%rdx
	movq	$-1, %rax
	movq	%rdi, 16(%rsp)
	movq	%rsi, 24(%rsp)
	cmoveq	%rax, %rdx
.Ltmp15897:
	movl	$8, %ecx
	movl	$48, %r8d
	leaq	216(%rsp), %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp15898:
	movq	224(%rsp), %rcx
	movq	24(%rsp), %rsi
	movq	16(%rsp), %rdi
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	jmp	.LBB998_178
.LBB998_186:
	movq	%r14, 144(%rsp)
	movq	%rbp, 136(%rsp)
	testq	%rbx, %rbx
	je	.LBB998_202
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB998_191
.LBB998_188:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_189:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB998_190:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB998_202
.LBB998_191:
	testw	%r12w, %r12w
	jne	.LBB998_194
	.p2align	4
.LBB998_192:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB998_192
	kmovd	%k0, %r12d
.LBB998_194:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB998_190
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
	jge	.LBB998_197
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_197:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_189
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_197
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
.LBB998_200:
	cmpq	%rax, %rdx
	jge	.LBB998_188
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB998_200
	jmp	.LBB998_188
.LBB998_202:
	cmpq	$0, 8(%rsp)
	movq	40(%rsp), %rsi
	movabsq	$-3689348814741910323, %r15
	je	.LBB998_213
	testq	%rsi, %rsi
	je	.LBB998_213
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_206
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_206:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_212
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_206
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
.LBB998_209:
	cmpq	%rax, %rdx
	jge	.LBB998_211
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB998_209
.LBB998_211:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_212:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB998_213:
	vmovdqu	216(%rsp), %xmm0
	movq	232(%rsp), %r12
	movq	%r12, 64(%rsp)
	vmovdqa	%xmm0, 48(%rsp)
	movq	56(%rsp), %r13
	cmpq	$2, %r12
	jae	.LBB998_268
.LBB998_214:
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
	je	.LBB998_222
	addq	$-48, %rax
	movabsq	$-6148914691236517205, %rsi
	movq	%rax, %rdx
	mulxq	%rsi, %rdx, %rdx
	shrl	$5, %edx
	incl	%edx
	andl	$7, %edx
	je	.LBB998_219
	shll	$3, %edx
	movq	%r13, %r14
	leaq	(%rdx,%rdx,4), %rsi
	movq	%r13, %rdx
	.p2align	4
.LBB998_217:
	vmovdqu	8(%rdx), %ymm0
	movq	40(%rdx), %rdi
	addq	$48, %rdx
	movq	%rdi, 32(%r14)
	vmovdqu	%ymm0, (%r14)
	addq	$40, %r14
	addq	$-40, %rsi
	jne	.LBB998_217
	movq	%rcx, %rbx
	cmpq	$336, %rax
	jae	.LBB998_220
	jmp	.LBB998_222
.LBB998_219:
	movq	%r13, %r14
	movq	%r13, %rdx
	movq	%rcx, %rbx
	cmpq	$336, %rax
	jb	.LBB998_222
	.p2align	4
.LBB998_220:
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
	jne	.LBB998_220
	movq	%rcx, %rbx
.LBB998_222:
	vmovdqa	.LCPI998_5(%rip), %ymm0
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
	je	.LBB998_235
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	shrq	$4, %rcx
	movabsq	$-6148914691236517205, %r12
	movabsq	$9223372036854775807, %rbp
	xorl	%r15d, %r15d
	imulq	%rcx, %r12
	jmp	.LBB998_227
	.p2align	4
.LBB998_224:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_225:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB998_226:
	incq	%r15
	cmpq	%r12, %r15
	je	.LBB998_235
.LBB998_227:
	leaq	(%r15,%r15,2), %rax
	shlq	$4, %rax
	movq	8(%rbx,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB998_226
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
	jge	.LBB998_230
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_230:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_225
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_230
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
.LBB998_233:
	cmpq	%rax, %rdx
	jge	.LBB998_224
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB998_233
	jmp	.LBB998_224
.LBB998_235:
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
	jne	.LBB998_248
	cmpq	$39, %rcx
	ja	.LBB998_249
	movl	$8, %r12d
	testq	%rcx, %rcx
	je	.LBB998_250
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_240
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB998_240:
	movq	32(%rsp), %rdi
	.p2align	4
.LBB998_241:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_247
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_241
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
.LBB998_244:
	cmpq	%rax, %rdx
	jge	.LBB998_246
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB998_244
.LBB998_246:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_247:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB998_250
.LBB998_248:
	movq	32(%rsp), %r12
	jmp	.LBB998_250
.LBB998_249:
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rsi
	leaq	qualification_454_native_cost::GLOBAL (.llvm.11174181910260379007)(%rip), %rdi
	movl	$8, %edx
	movq	%r13, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %r12
	testq	%rax, %rax
	je	.LBB998_292
.LBB998_250:
	movq	8(%rsp), %rax
	cmpq	$-1, 352(%rsp)
	movq	%rax, 104(%rsp)
	movq	%rbx, 80(%rsp)
	movq	%r12, 88(%rsp)
	movq	%r14, 96(%rsp)
	je	.LBB998_252
	leaq	112(%rsp), %rdi
	leaq	80(%rsp), %rsi
	leaq	352(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	280(%rsp), %rbx
	movq	424(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB998_253
	jmp	.LBB998_262
.LBB998_252:
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
	jb	.LBB998_262
.LBB998_253:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_255
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_255:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_261
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_255
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
.LBB998_258:
	cmpq	%rax, %rsi
	jge	.LBB998_260
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB998_258
.LBB998_260:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_261:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB998_262:
	movq	448(%rsp), %rax
	testq	%rax, %rax
	je	.LBB998_265
	lock		decq	(%rax)
	jne	.LBB998_265
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	448(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB998_265:
	vmovdqu64	144(%rsp), %zmm1
	vmovdqu64	112(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
.LBB998_266:
	movq	$0, (%rbx)
.LBB998_267:
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
.LBB998_268:
	.cfi_def_cfa_offset 816
	cmpq	$21, %r12
	jae	.LBB998_294
	shlq	$4, %r12
	movabsq	$-6148914691236517205, %rcx
	leaq	48(%r13), %rax
	leaq	-96(%r12,%r12,2), %rdx
	mulxq	%rcx, %rcx, %rcx
	btl	$5, %ecx
	jb	.LBB998_273
	movq	48(%r13), %rcx
	cmpq	(%r13), %rcx
	jae	.LBB998_272
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
.LBB998_272:
	leaq	96(%r13), %rcx
	jmp	.LBB998_274
.LBB998_273:
	movq	%rax, %rcx
	movq	%r13, %rax
.LBB998_274:
	cmpq	$48, %rdx
	jae	.LBB998_276
.LBB998_275:
	movq	56(%rsp), %r13
	movq	64(%rsp), %r12
	jmp	.LBB998_214
.LBB998_276:
	leaq	(%r12,%r12,2), %rdx
	addq	%r13, %rdx
	jmp	.LBB998_279
.LBB998_277:
	movq	%rsi, (%rdi)
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 8(%rdi)
	movq	144(%rsp), %rsi
	movq	%rsi, 40(%rdi)
.LBB998_278:
	addq	$96, %rcx
	cmpq	%rdx, %rcx
	je	.LBB998_275
.LBB998_279:
	movq	(%rcx), %rsi
	cmpq	(%rax), %rsi
	jae	.LBB998_280
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
	je	.LBB998_286
.LBB998_282:
	cmpq	-48(%rax), %rsi
	jae	.LBB998_285
	leaq	-48(%rax), %rdi
	vmovdqu	(%rdi), %ymm0
	vmovdqu	16(%rdi), %ymm1
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu	%ymm0, (%rax)
	movq	%rdi, %rax
	cmpq	%r13, %rdi
	jne	.LBB998_282
	movq	%r13, %rdi
	jmp	.LBB998_286
.LBB998_280:
	movq	48(%rcx), %rsi
	leaq	48(%rcx), %rax
	cmpq	(%rcx), %rsi
	jae	.LBB998_278
	jmp	.LBB998_287
.LBB998_285:
	movq	%rax, %rdi
.LBB998_286:
	movq	%rsi, (%rdi)
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 8(%rdi)
	movq	144(%rsp), %rax
	movq	%rax, 40(%rdi)
	movq	48(%rcx), %rsi
	leaq	48(%rcx), %rax
	cmpq	(%rcx), %rsi
	jae	.LBB998_278
.LBB998_287:
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
	je	.LBB998_277
	movq	%rcx, %rdi
.LBB998_289:
	cmpq	-48(%rdi), %rsi
	jae	.LBB998_277
	leaq	-48(%rdi), %r8
	vmovdqu	(%r8), %ymm0
	vmovdqu	16(%r8), %ymm1
	vmovdqu	%ymm1, 16(%rdi)
	vmovdqu	%ymm0, (%rdi)
	movq	%r8, %rdi
	cmpq	%r13, %r8
	jne	.LBB998_289
	movq	%r13, %rdi
	jmp	.LBB998_277
.LBB998_292:
.Ltmp15908:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp15909:
.LBB998_293:
	ud2
.LBB998_294:
.Ltmp15900:
	movq	core::slice::sort::unstable::ipnsort::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), <[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#1}>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15901:
	jmp	.LBB998_214
.LBB998_295:
	movl	$8, %edi
	jmp	.LBB998_147
.LBB998_296:
.Ltmp15902:
	leaq	48(%rsp), %rdi
	movq	%rax, (%rsp)
	jmp	.LBB998_303
.LBB998_297:
.Ltmp15896:
	movq	8(%rsp), %rcx
	movq	%rbp, 120(%rsp)
	movq	%rax, (%rsp)
	movq	%rcx, 144(%rsp)
	cmpq	$6, %rbx
	jb	.LBB998_299
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r13, %rdi
	callq	__rustc::__rust_dealloc
.LBB998_299:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>>
	jmp	.LBB998_331
.LBB998_300:
.Ltmp15899:
	movq	%r14, 144(%rsp)
	movq	%rbp, 136(%rsp)
	movw	%r12w, 160(%rsp)
	movq	%rax, (%rsp)
	movq	%rbx, 168(%rsp)
	cmpq	$6, %r13
	jb	.LBB998_302
	movq	24(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB998_302:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	216(%rsp), %rdi
.LBB998_303:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>>
	jmp	.LBB998_331
.LBB998_304:
.Ltmp15893:
	movb	$1, %bl
	movq	%rax, (%rsp)
	jmp	.LBB998_332
.LBB998_305:
.Ltmp15890:
	movq	%rax, (%rsp)
	jmp	.LBB998_336
.LBB998_306:
.Ltmp15885:
	movq	%rax, (%rsp)
.Ltmp15886:
	leaq	504(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15887:
	jmp	.LBB998_359
.LBB998_307:
.Ltmp15910:
	leaq	80(%rsp), %rdi
	movq	%rax, (%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#2}>>
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB998_336
	#MEMBARRIER
.Ltmp15911:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp15912:
	jmp	.LBB998_336
.LBB998_309:
.Ltmp15905:
	movq	%rax, (%rsp)
	movq	24(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB998_319
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_312
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_312:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_318
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_312
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
.LBB998_315:
	cmpq	%rax, %rsi
	jge	.LBB998_317
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB998_315
.LBB998_317:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_318:
	movq	free@GOTPCREL(%rip), %rax
	movq	16(%rsp), %rdi
	callq	*%rax
.LBB998_319:
	testq	%rbx, %rbx
	jne	.LBB998_360
.LBB998_320:
	xorl	%ebx, %ebx
	cmpq	$0, 8(%rsp)
	je	.LBB998_332
	movq	40(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB998_332
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_324
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_324:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_330
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_324
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
.LBB998_327:
	cmpq	%rax, %rdx
	jge	.LBB998_329
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB998_327
.LBB998_329:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_330:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	callq	*%rax
.LBB998_331:
	xorl	%ebx, %ebx
.LBB998_332:
	movq	344(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB998_334
	leaq	344(%rsp), %rdi
	#MEMBARRIER
.Ltmp15906:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15907:
.LBB998_334:
	testb	%bl, %bl
	je	.LBB998_336
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB998_336:
	movq	424(%rsp), %rax
	movabsq	$9223372036854775807, %rbx
	cmpq	$6, %rax
	jb	.LBB998_337
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	432(%rsp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_341
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_341:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_347
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_341
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
.LBB998_344:
	cmpq	%rax, %rdx
	jge	.LBB998_346
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB998_344
.LBB998_346:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_347:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB998_348
.LBB998_338:
	movq	448(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB998_357
	jmp	.LBB998_359
.LBB998_337:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB998_338
.LBB998_348:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	360(%rsp), %rdi
	cmpq	%rbx, %rcx
	cmovaeq	%rbx, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rbx, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB998_350
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_350:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_356
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_350
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
.LBB998_353:
	cmpq	%rax, %rdx
	jge	.LBB998_355
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB998_353
.LBB998_355:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_356:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	448(%rsp), %rax
	testq	%rax, %rax
	je	.LBB998_359
.LBB998_357:
	lock		decq	(%rax)
	jne	.LBB998_359
	leaq	448(%rsp), %rdi
	#MEMBARRIER
.Ltmp15914:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15915:
.LBB998_359:
	movq	(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB998_360:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB998_364
	.p2align	4
.LBB998_361:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB998_362:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB998_363:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB998_320
.LBB998_364:
	testw	%r12w, %r12w
	jne	.LBB998_367
	.p2align	4
.LBB998_365:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB998_365
	kmovd	%k0, %r12d
.LBB998_367:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB998_363
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
	jge	.LBB998_370
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB998_370:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB998_362
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB998_370
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
.LBB998_373:
	cmpq	%rax, %rdx
	jge	.LBB998_361
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB998_373
	jmp	.LBB998_361
.LBB998_375:
.Ltmp15913:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB998_376:
.Ltmp15916:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end998:
purrdf_sparql_eval::modifier::eval_graph::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin999:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception659
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
	movq	%r8, %r13
	movq	%rdx, %rbx
	movq	%rcx, 120(%rsp)
	movq	%rdi, 8(%rsp)
	movq	%r8, 272(%rsp)
	je	.LBB999_16
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	808(%rsp), %rdi
	callq	*%rax
	movq	664(%r13), %rax
	movl	$4, %edx
	movq	%rbx, 392(%rsp)
	movq	%rdx, 184(%rsp)
	movq	160(%rax), %rcx
	testq	%rcx, %rcx
	je	.LBB999_30
	movq	152(%rax), %r15
	cmpl	$1, 1144(%r13)
	leaq	(%r15,%rcx,4), %r12
	jne	.LBB999_31
	movq	1152(%r13), %rax
	testq	%rax, %rax
	je	.LBB999_30
	movq	1160(%r13), %rcx
	xorl	%ebx, %ebx
.LBB999_5:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rcx, %rdx
	movq	%rax, %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	je	.LBB999_11
.LBB999_6:
	movl	%r8d, %r9d
	shll	$2, %r9d
	xorl	%edi, %edi
	.p2align	4
.LBB999_7:
	cmpl	8(%rsi,%rdi,4), %ebp
	seta	%r10b
	sbbb	$0, %r10b
	cmpb	$1, %r10b
	jne	.LBB999_10
	incq	%rdi
	addq	$-4, %r9
	jne	.LBB999_7
	jmp	.LBB999_11
.LBB999_10:
	movzbl	%r10b, %r8d
	testl	%r8d, %r8d
	je	.LBB999_32
	jmp	.LBB999_12
	.p2align	4
.LBB999_11:
	movq	%r8, %rdi
.LBB999_12:
	subq	$1, %rdx
	jb	.LBB999_14
	movq	56(%rsi,%rdi,8), %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	jne	.LBB999_6
	jmp	.LBB999_11
.LBB999_14:
	cmpq	%r12, %r15
	jne	.LBB999_5
	xorl	%r14d, %r14d
	jmp	.LBB999_62
.LBB999_16:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	704(%rsp), %rdi
	callq	*%rax
	movq	8(%rbx), %r12
	movq	16(%rbx), %rbx
	addq	$16, %r12
	testq	%rbx, %rbx
	jns	.LBB999_19
	xorl	%edi, %edi
.LBB999_18:
.Ltmp15940:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp15941:
	jmp	.LBB999_416
.LBB999_19:
	movq	664(%r13), %r14
	movabsq	$-9223372036854775808, %r15
	je	.LBB999_244
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB999_412
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rsi
	movq	$-1, %rdi
	leaq	(%rbx,%rax), %rcx
	sarq	$63, %rcx
	xorq	%r15, %rcx
	addq	%rbx, %rax
	cmovoq	%rcx, %rax
	incq	%rdx
	cmoveq	%rdi, %rdx
	addq	%rbx, %rsi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmovbq	%rdi, %rsi
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB999_23
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_23:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_29
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_23
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
.LBB999_26:
	cmpq	%rax, %rcx
	jle	.LBB999_28
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB999_26
.LBB999_28:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_29:
	.cfi_escape 0x2e, 0x00
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	callq	*%rax
	jmp	.LBB999_245
.LBB999_30:
	xorl	%ebx, %ebx
	xorl	%r14d, %r14d
	jmp	.LBB999_62
.LBB999_31:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB999_32:
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$16, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB999_410
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$16, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$16, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB999_35
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_35:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_41
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_35
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
.LBB999_38:
	cmpq	%rax, %rsi
	jle	.LBB999_40
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB999_38
.LBB999_40:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_41:
	movl	$1, %ebx
	movq	$4, 16(%rsp)
	movq	%rcx, 24(%rsp)
	movl	%ebp, (%rcx)
	movq	$1, 32(%rsp)
	cmpq	%r12, %r15
	je	.LBB999_61
	leaq	16(%rsp), %r14
	jmp	.LBB999_45
	.p2align	4
.LBB999_43:
	movq	24(%rsp), %rcx
.LBB999_44:
	movl	%ebp, (%rcx,%rbx,4)
	incq	%rbx
	movq	%rbx, 32(%rsp)
	cmpq	%r12, %r15
	je	.LBB999_61
.LBB999_45:
	cmpl	$1, 1144(%r13)
	jne	.LBB999_58
	movq	1152(%r13), %rax
	testq	%rax, %rax
	je	.LBB999_61
	movq	1160(%r13), %rdx
.LBB999_48:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rdx, %rsi
	movq	%rax, %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	je	.LBB999_54
.LBB999_49:
	movl	%r9d, %r10d
	shll	$2, %r10d
	xorl	%r8d, %r8d
	.p2align	4
.LBB999_50:
	cmpl	8(%rdi,%r8,4), %ebp
	seta	%r11b
	sbbb	$0, %r11b
	cmpb	$1, %r11b
	jne	.LBB999_53
	incq	%r8
	addq	$-4, %r10
	jne	.LBB999_50
	jmp	.LBB999_54
	.p2align	4
.LBB999_53:
	movzbl	%r11b, %r9d
	testl	%r9d, %r9d
	je	.LBB999_59
	subq	$1, %rsi
	jae	.LBB999_56
	jmp	.LBB999_57
	.p2align	4
.LBB999_54:
	movq	%r9, %r8
	subq	$1, %rsi
	jb	.LBB999_57
.LBB999_56:
	movq	56(%rdi,%r8,8), %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	jne	.LBB999_49
	jmp	.LBB999_54
.LBB999_57:
	cmpq	%r12, %r15
	jne	.LBB999_48
	jmp	.LBB999_61
	.p2align	4
.LBB999_58:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB999_59:
	cmpq	16(%rsp), %rbx
	jne	.LBB999_44
.Ltmp15946:
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	movl	$4, %ecx
	movl	$4, %r8d
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp15947:
	jmp	.LBB999_43
.LBB999_61:
	movq	16(%rsp), %r14
	movq	24(%rsp), %rax
	movq	%rax, 184(%rsp)
.LBB999_62:
.Ltmp15952:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::modifier::yields_nothing_without_rows_in_the_active_graph@GOTPCREL(%rip), %rax
	movq	120(%rsp), %rdi
	movq	%r14, 264(%rsp)
	callq	*%rax
	movb	%al, 7(%rsp)
.Ltmp15953:
	movq	392(%rsp), %rax
	movl	784(%r13), %esi
	movl	788(%r13), %edx
	movq	$0, 216(%rsp)
	movq	$8, 224(%rsp)
	movq	$-1, 560(%rsp)
	movq	$0, 232(%rsp)
	leaq	8(%rax), %rcx
	movl	%esi, 136(%rsp)
	movl	%edx, 140(%rsp)
	movq	%rcx, 400(%rsp)
	testq	%rbx, %rbx
	je	.LBB999_184
	movq	184(%rsp), %rcx
	movq	8(%rax), %rsi
	movq	16(%rax), %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	leaq	496(%rsp), %rbp
	leaq	288(%rsp), %r14
	movq	$0, 192(%rsp)
	movq	$0, 384(%rsp)
	leaq	(%rcx,%rbx,4), %rdx
	movq	%rax, 640(%rsp)
	movl	$8, %eax
	movq	%rsi, 648(%rsp)
	movq	%rcx, 280(%rsp)
	movq	%rax, 128(%rsp)
	movq	%rdx, 456(%rsp)
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.511(%rip), %rdx
	movq	%rdx, 448(%rsp)
.LBB999_65:
	movq	280(%rsp), %rax
	jmp	.LBB999_67
	.p2align	4
.LBB999_66:
	movq	280(%rsp), %rax
	cmpq	456(%rsp), %rax
	je	.LBB999_185
.LBB999_67:
	movl	(%rax), %ecx
	addq	$4, %rax
	cmpb	$0, 7(%rsp)
	movq	%rax, 280(%rsp)
	movq	%rcx, 240(%rsp)
	je	.LBB999_76
	movq	664(%r13), %rbx
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::probe_plan@GOTPCREL(%rip), %rax
	movl	$2, %ecx
	xorl	%edi, %edi
	xorl	%esi, %esi
	xorl	%edx, %edx
	vzeroupper
	callq	*%rax
	movq	%rax, 288(%rsp)
	movb	%dl, 296(%rsp)
.Ltmp15955:
	.cfi_escape 0x2e, 0x10
	movq	%r14, %rdx
	leaq	16(%rsp), %rdi
	movq	%rbx, %rsi
	xorl	%ecx, %ecx
	xorl	%r8d, %r8d
	xorl	%r9d, %r9d
	pushq	240(%rsp)
	.cfi_adjust_cfa_offset 8
	pushq	$2
	.cfi_adjust_cfa_offset 8
	movq	<purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan@GOTPCREL(%rip), %rax
	callq	*%rax
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp15956:
.Ltmp15957:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	callq	<purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next
.Ltmp15958:
	testq	%rax, %rax
	jne	.LBB999_76
	cmpq	$0, 80(%rbx)
	je	.LBB999_74
.Ltmp15959:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.a9046ebb4da6daa9c28122cb9c29f82a.235.llvm.11857615832421379041(%rip), %rsi
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15960:
	testl	%eax, %eax
	je	.LBB999_406
.LBB999_74:
.Ltmp15964:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.a9046ebb4da6daa9c28122cb9c29f82a.235.llvm.11857615832421379041(%rip), %rsi
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15965:
	movq	%rbx, 24(%rsp)
	movl	%eax, 32(%rsp)
	movq	240(%rsp), %rax
	movq	$0, 40(%rsp)
	movq	$0, 64(%rsp)
	movl	$2, 16(%rsp)
	movl	%eax, 20(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rsi
	movq	%r14, %rdi
	callq	<core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next
	cmpl	$0, 288(%rsp)
	je	.LBB999_177
	.p2align	4
.LBB999_76:
	movq	240(%rsp), %rax
	movl	$2, 784(%r13)
	movl	%eax, 788(%r13)
.Ltmp15966:
	.cfi_escape 0x2e, 0x00
	movq	120(%rsp), %rcx
	leaq	912(%rsp), %rdi
	movq	%r13, %rdx
	movq	%rcx, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15967:
	cmpl	$1, 912(%rsp)
	je	.LBB999_239
	cmpq	$-1, 920(%rsp)
	jne	.LBB999_240
	leaq	920(%rsp), %rax
	vmovdqu	8(%rax), %ymm0
	vmovdqu	%ymm0, 416(%rsp)
	movq	440(%rsp), %rsi
	addq	$16, %rsi
.Ltmp15975:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp15976:
.Ltmp15978:
	.cfi_escape 0x2e, 0x00
	movq	400(%rsp), %rsi
	movq	%r14, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.13412714042204560522)
.Ltmp15979:
	cmpq	$1, %rax
	jne	.LBB999_105
	movq	432(%rsp), %rax
	movq	424(%rsp), %r13
	movq	%rdx, %r14
	movq	416(%rsp), %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%r13, 16(%rsp)
	movq	%rdx, %r12
	movq	%rdx, 32(%rsp)
	movq	%r13, 408(%rsp)
	leaq	(%r13,%rcx,8), %rbx
	movq	%rbx, 40(%rsp)
	testq	%rax, %rax
	je	.LBB999_124
	vmovd	240(%rsp), %xmm0
	vpshufb	.LCPI999_0(%rip), %xmm0, %xmm0
	vmovdqa	%xmm0, 464(%rsp)
	jmp	.LBB999_85
	.p2align	4
.LBB999_84:
	movq	192(%rsp), %rdx
	movq	224(%rsp), %rsi
	leaq	(%rdx,%rdx,4), %rax
	incq	%rdx
	movq	%rsi, 128(%rsp)
	movq	%rdx, 192(%rsp)
	movq	%r15, (%rsi,%rax,8)
	movq	%rbp, 8(%rsi,%rax,8)
	leaq	496(%rsp), %rbp
	vmovdqu	8(%rbp), %xmm0
	vmovdqu	%xmm0, 16(%rsi,%rax,8)
	movq	24(%rbp), %rcx
	movq	%rcx, 32(%rsi,%rax,8)
	movq	%rdx, 232(%rsp)
	cmpq	%rbx, %r13
	je	.LBB999_137
.LBB999_85:
	movq	%r13, %rax
	movq	(%rax), %r15
	addq	$40, %r13
	testq	%r15, %r15
	je	.LBB999_123
	movq	%r15, 488(%rsp)
	movq	%rbp, %rdx
	leaq	-1(%r15), %rcx
	vmovdqu	8(%rax), %ymm0
	cmpq	$5, %rcx
	vmovdqu	%ymm0, (%rbp)
	movq	504(%rsp), %rax
	movq	496(%rsp), %rbp
	leaq	-1(%rax), %rsi
	cmovbq	%rcx, %rsi
	cmpq	%rsi, %r14
	jae	.LBB999_409
	cmpq	$5, %rcx
	movq	%rdx, %rcx
	cmovaeq	%rbp, %rcx
	movl	(%rcx,%r14,8), %edx
	cmpl	$2, %edx
	je	.LBB999_90
	testl	%edx, %edx
	jne	.LBB999_93
	movq	240(%rsp), %rdx
	cmpl	%edx, 4(%rcx,%r14,8)
	jne	.LBB999_93
.LBB999_90:
	cmpq	$6, %r15
	cmovbq	%r15, %rax
	decq	%rax
	cmpq	%rax, %r14
	jae	.LBB999_408
	vmovdqa	464(%rsp), %xmm0
	cmpq	$6, %r15
	leaq	496(%rsp), %rax
	cmovbq	%rax, %rbp
	movq	192(%rsp), %rax
	vmovq	%xmm0, (%rbp,%r14,8)
	movq	488(%rsp), %r15
	movq	496(%rsp), %rbp
	cmpq	216(%rsp), %rax
	jne	.LBB999_84
.Ltmp15995:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	216(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15996:
	jmp	.LBB999_84
	.p2align	4
.LBB999_93:
	cmpq	$6, %r15
	jb	.LBB999_104
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_96
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB999_96:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rsi
	.p2align	4
.LBB999_97:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_103
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_97
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
.LBB999_100:
	cmpq	%rax, %rdx
	jge	.LBB999_102
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB999_100
.LBB999_102:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_103:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.LBB999_104:
	leaq	496(%rsp), %rbp
	cmpq	%rbx, %r13
	jne	.LBB999_85
	jmp	.LBB999_137
	.p2align	4
.LBB999_105:
	movq	648(%rsp), %rsi
	lock		incq	(%rsi)
	jle	.LBB999_416
.Ltmp15980:
	.cfi_escape 0x2e, 0x00
	movq	640(%rsp), %rdx
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp15981:
	movq	%rax, %r14
	movq	432(%rsp), %rax
	movq	424(%rsp), %r13
	movq	416(%rsp), %rdx
	movq	304(%rsp), %r12
	leaq	(%rax,%rax,4), %rcx
	movq	%r13, 672(%rsp)
	movq	%rdx, 664(%rsp)
	movq	%rdx, 688(%rsp)
	movq	%r13, 656(%rsp)
	leaq	(%r13,%rcx,8), %rbx
	movq	%rbx, 696(%rsp)
	testq	%rax, %rax
	je	.LBB999_148
	leaq	1(%r12), %rax
	vmovd	240(%rsp), %xmm0
	addq	$40, %r13
	movq	%rbx, 464(%rsp)
	movq	%rax, 408(%rsp)
	movq	192(%rsp), %rax
	vpshufb	.LCPI999_0(%rip), %xmm0, %xmm0
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %r15
	vmovdqa	%xmm0, 240(%rsp)
	jmp	.LBB999_110
	.p2align	4
.LBB999_109:
	movq	128(%rsp), %rdx
	leaq	-40(%r13), %rax
	addq	$40, %r13
	addq	$40, %rax
	movq	%r12, (%rdx,%r15)
	movq	%rbp, 8(%rdx,%r15)
	movq	%rbx, %r12
	vmovdqa	528(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%r15)
	movq	544(%rsp), %rcx
	movq	%rcx, 32(%rdx,%r15)
	movq	192(%rsp), %rcx
	addq	$40, %r15
	incq	%rcx
	movq	%rcx, 192(%rsp)
	movq	%rcx, 232(%rsp)
	cmpq	464(%rsp), %rax
	je	.LBB999_161
.LBB999_110:
	vmovdqu	-32(%r13), %ymm0
	movq	-40(%r13), %rax
	vmovdqu	%ymm0, 144(%rsp)
	testq	%rax, %rax
	je	.LBB999_147
	vmovdqu	144(%rsp), %ymm0
	leaq	24(%rsp), %rcx
	movq	%rax, 16(%rsp)
	leaq	-1(%rax), %rdx
	cmpq	$5, %rdx
	vmovdqu	%ymm0, (%rcx)
	movq	32(%rsp), %rcx
	leaq	-1(%rcx), %rsi
	cmovbq	%rdx, %rsi
	movq	%r12, %rdx
	subq	%rsi, %rdx
	jbe	.LBB999_113
	movl	$2, 528(%rsp)
	movq	%rdx, 536(%rsp)
.Ltmp15983:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	leaq	528(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp15984:
	jmp	.LBB999_115
	.p2align	4
.LBB999_113:
	cmpq	$6, %rax
	cmovbq	%rax, %rcx
	decq	%rcx
	cmpq	%rcx, %r12
	jae	.LBB999_115
	movq	408(%rsp), %rdx
	xorl	%ecx, %ecx
	cmpq	$6, %rax
	setae	%cl
	shll	$4, %ecx
	movq	%rdx, 16(%rsp,%rcx)
.LBB999_115:
	movq	16(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB999_117
	movq	32(%rsp), %rsi
.LBB999_117:
	decq	%rsi
	cmpq	%rsi, %r14
	jae	.LBB999_407
	leaq	24(%rsp), %rcx
	movq	%r12, %rbx
	cmpq	$6, %rax
	jb	.LBB999_120
	movq	24(%rsp), %rcx
.LBB999_120:
	vmovaps	240(%rsp), %xmm0
	leaq	24(%rsp), %rax
	vmovlps	%xmm0, (%rcx,%r14,8)
	movq	192(%rsp), %rcx
	vmovdqu	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	16(%rsp), %r12
	movq	24(%rsp), %rbp
	movq	%rax, 544(%rsp)
	vmovdqa	%xmm0, 528(%rsp)
	cmpq	216(%rsp), %rcx
	jne	.LBB999_109
.Ltmp15989:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	216(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15990:
	movq	224(%rsp), %rax
	movq	%rax, 128(%rsp)
	jmp	.LBB999_109
.LBB999_123:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
.LBB999_124:
	subq	%r13, %rbx
	je	.LBB999_137
	shrq	$3, %rbx
	movabsq	$-3689348814741910323, %rax
	xorl	%r14d, %r14d
	imulq	%rax, %rbx
	jmp	.LBB999_129
	.p2align	4
.LBB999_126:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_127:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB999_128:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB999_137
.LBB999_129:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB999_128
	leaq	(%r13,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_132
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_132:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_127
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_132
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
	movq	(%r15), %rax
	.p2align	4
.LBB999_135:
	cmpq	%rax, %rdx
	jge	.LBB999_126
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB999_135
	jmp	.LBB999_126
	.p2align	4
.LBB999_137:
	movq	272(%rsp), %r13
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	leaq	288(%rsp), %r14
	testq	%r12, %r12
	je	.LBB999_173
	shlq	$3, %r12
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%r12,%r12,4), %rcx
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_140
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_140:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_146
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_140
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
	movq	(%r15), %rax
	.p2align	4
.LBB999_143:
	cmpq	%rax, %rdx
	jge	.LBB999_145
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB999_143
.LBB999_145:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_146:
	.cfi_escape 0x2e, 0x00
	movq	408(%rsp), %rdi
	jmp	.LBB999_172
.LBB999_147:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	movq	464(%rsp), %rbx
	leaq	496(%rsp), %rbp
.LBB999_148:
	subq	%r13, %rbx
	je	.LBB999_162
	shrq	$3, %rbx
	movabsq	$-3689348814741910323, %rax
	xorl	%r14d, %r14d
	imulq	%rax, %rbx
	jmp	.LBB999_153
	.p2align	4
.LBB999_150:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_151:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB999_152:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB999_162
.LBB999_153:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB999_152
	leaq	(%r13,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_156
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_156:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_151
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_156
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
	movq	(%r15), %rax
	.p2align	4
.LBB999_159:
	cmpq	%rax, %rdx
	jge	.LBB999_150
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB999_159
	jmp	.LBB999_150
.LBB999_161:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	leaq	496(%rsp), %rbp
.LBB999_162:
	movq	664(%rsp), %rax
	movq	272(%rsp), %r13
	leaq	288(%rsp), %r14
	testq	%rax, %rax
	je	.LBB999_173
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_165
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_165:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_171
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_165
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
	movq	(%r15), %rax
	.p2align	4
.LBB999_168:
	cmpq	%rax, %rdx
	jge	.LBB999_170
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB999_168
.LBB999_170:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_171:
	.cfi_escape 0x2e, 0x00
	movq	656(%rsp), %rdi
.LBB999_172:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB999_173:
	vmovups	312(%rsp), %ymm1
	vmovups	288(%rsp), %ymm0
	cmpq	$-1, 560(%rsp)
	vmovups	%ymm1, 40(%rsp)
	vmovups	%ymm0, 16(%rsp)
	je	.LBB999_175
.Ltmp16000:
	.cfi_escape 0x2e, 0x00
	leaq	560(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16001:
.LBB999_175:
	vmovups	40(%rsp), %ymm1
	vmovdqu	16(%rsp), %ymm0
	movq	440(%rsp), %rax
	vmovups	%ymm1, 584(%rsp)
	vmovdqu	%ymm0, 560(%rsp)
	lock		decq	(%rax)
	jne	.LBB999_66
	#MEMBARRIER
.Ltmp16005:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16006:
	jmp	.LBB999_66
.LBB999_177:
	movq	96(%rbx), %rax
	testq	%rax, %rax
	je	.LBB999_183
	movq	88(%rbx), %rcx
	shlq	$4, %rax
	xorl	%edx, %edx
	jmp	.LBB999_180
	.p2align	4
.LBB999_179:
	addq	$16, %rdx
	cmpq	%rdx, %rax
	je	.LBB999_183
.LBB999_180:
	movl	12(%rcx,%rdx), %esi
	testl	%esi, %esi
	je	.LBB999_179
	cmpl	240(%rsp), %esi
	jne	.LBB999_179
	cmpl	$0, (%rcx,%rdx)
	je	.LBB999_179
	jmp	.LBB999_76
.LBB999_183:
	movb	$1, %al
	movb	$1, %bl
	movq	280(%rsp), %rcx
	movq	%rax, 384(%rsp)
	cmpq	456(%rsp), %rcx
	jne	.LBB999_65
	jmp	.LBB999_186
.LBB999_184:
	movq	$0, 192(%rsp)
	movq	$0, 384(%rsp)
.LBB999_185:
	movb	$1, %bl
.LBB999_186:
	movq	264(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB999_196
.LBB999_187:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	shlq	$2, %rsi
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %rsi
	cmovaeq	%rcx, %rsi
	xorl	%edx, %edx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_189
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_189:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_195
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_189
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
.LBB999_192:
	cmpq	%rax, %rdx
	jge	.LBB999_194
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB999_192
.LBB999_194:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_195:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	184(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB999_196:
	movl	136(%rsp), %eax
	movl	140(%rsp), %ecx
	movl	%eax, 784(%r13)
	movl	%ecx, 788(%r13)
	movq	560(%rsp), %r13
	cmpq	$-1, %r13
	sete	%bpl
	je	.LBB999_210
	vmovups	584(%rsp), %ymm1
	vmovdqu	560(%rsp), %ymm0
	vmovups	%ymm1, 56(%rsp)
	vmovdqu	%ymm0, 32(%rsp)
	movq	$1, 16(%rsp)
	movq	$1, 24(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB999_411
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB999_200
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_200:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_206
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_200
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
.LBB999_203:
	cmpq	%rax, %rdx
	jle	.LBB999_205
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB999_203
.LBB999_205:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_206:
	vmovdqu64	16(%rsp), %zmm0
	movq	80(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
.LBB999_207:
	vmovups	848(%rsp), %zmm1
	vmovdqu64	808(%rsp), %zmm0
	movq	232(%rsp), %rax
	movq	%rax, 160(%rsp)
	vmovups	%zmm1, 56(%rsp)
	vmovups	216(%rsp), %xmm1
	vmovdqu64	%zmm0, 16(%rsp)
	cmpq	$-1, 16(%rsp)
	vmovaps	%xmm1, 144(%rsp)
	movq	%rbx, 168(%rsp)
	je	.LBB999_226
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	leaq	144(%rsp), %rsi
	leaq	808(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	88(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB999_209
.LBB999_227:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	96(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_229
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_229:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_235
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_229
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
.LBB999_232:
	cmpq	%rax, %rsi
	jge	.LBB999_234
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB999_232
.LBB999_234:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_235:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	112(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB999_236
	jmp	.LBB999_238
.LBB999_210:
	testb	%bl, %bl
	je	.LBB999_280
	testb	$1, 384(%rsp)
	movq	392(%rsp), %rbx
	movb	$1, %r12b
	je	.LBB999_368
.Ltmp16081:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	120(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16082:
	movq	%rax, %rsi
	movq	%rax, %r14
	movq	%rax, 16(%rsp)
	addq	$16, %rsi
.Ltmp16084:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp16085:
	lock		decq	(%r14)
	jne	.LBB999_216
	#MEMBARRIER
.Ltmp16089:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.Ltmp16090:
.LBB999_216:
	movq	400(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	jle	.LBB999_416
	movq	16(%rbx), %rdx
.Ltmp16091:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp16092:
	vmovups	312(%rsp), %ymm1
	vmovdqu	288(%rsp), %ymm0
	vmovups	%ymm1, 56(%rsp)
	vmovdqu	%ymm0, 32(%rsp)
	movq	$1, 16(%rsp)
	movq	$1, 24(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB999_414
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB999_221
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_221:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_206
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_221
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
.LBB999_224:
	cmpq	%rax, %rdx
	jle	.LBB999_205
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB999_224
	jmp	.LBB999_205
.LBB999_226:
	vmovdqu	144(%rsp), %xmm0
	movq	160(%rsp), %rax
	movq	168(%rsp), %rcx
	movq	%rax, 312(%rsp)
	movq	%rcx, 320(%rsp)
	vmovdqu	%xmm0, 296(%rsp)
	movq	$-1, 288(%rsp)
	movq	88(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB999_227
.LBB999_209:
	movq	112(%rsp), %rax
	testq	%rax, %rax
	je	.LBB999_238
.LBB999_236:
	lock		decq	(%rax)
	jne	.LBB999_238
	leaq	112(%rsp), %rdi
	#MEMBARRIER
.Ltmp16096:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16097:
.LBB999_238:
	vmovdqu64	288(%rsp), %zmm0
	vmovups	320(%rsp), %zmm1
	movq	8(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB999_367
.LBB999_239:
	leaq	920(%rsp), %rax
	movl	136(%rsp), %ecx
	movl	140(%rsp), %edx
	movb	$1, %sil
	vmovdqu64	8(%rax), %zmm0
	vmovups	40(%rax), %zmm1
	movq	8(%rsp), %rax
	movl	%ecx, 784(%r13)
	movl	%edx, 788(%r13)
	vmovups	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movq	264(%rsp), %rdi
	movl	%esi, 128(%rsp)
	testq	%rdi, %rdi
	jne	.LBB999_308
	jmp	.LBB999_317
.LBB999_240:
	movb	$1, %r14b
.Ltmp15968:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	leaq	808(%rsp), %rsi
	leaq	920(%rsp), %rcx
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15969:
	cmpq	$-1, 16(%rsp)
	je	.LBB999_306
.Ltmp15970:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15971:
	xorl	%ebx, %ebx
	movq	264(%rsp), %rsi
	testq	%rsi, %rsi
	jne	.LBB999_187
	jmp	.LBB999_196
.LBB999_244:
	movl	$1, %r13d
.LBB999_245:
	movb	$1, %bpl
	movq	%rbx, 568(%rsp)
	movq	%r13, 576(%rsp)
	movq	%rbx, 584(%rsp)
	movq	%r15, 560(%rsp)
.Ltmp15917:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_value@GOTPCREL(%rip), %rax
	leaq	560(%rsp), %rsi
	movq	%r14, %rdi
	callq	*%rax
.Ltmp15918:
	movq	272(%rsp), %rbx
	testl	%eax, %eax
	je	.LBB999_262
	movq	664(%rbx), %rcx
	movq	160(%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB999_262
	movq	152(%rcx), %rcx
	xorl	%esi, %esi
	cmpq	$1, %rdx
	je	.LBB999_250
	.p2align	4
.LBB999_249:
	movq	%rdx, %r8
	shrq	%r8
	movq	%rsi, %rdi
	addq	%r8, %rsi
	cmpl	%eax, (%rcx,%rsi,4)
	cmovaq	%rdi, %rsi
	subq	%r8, %rdx
	cmpq	$1, %rdx
	ja	.LBB999_249
.LBB999_250:
	cmpl	%eax, (%rcx,%rsi,4)
	jne	.LBB999_262
	cmpl	$1, 1144(%rbx)
	jne	.LBB999_296
	movq	1152(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB999_262
	movq	1160(%rbx), %rdx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	jne	.LBB999_254
.LBB999_259:
	movq	%rdi, %rsi
.LBB999_260:
	subq	$1, %rdx
	jb	.LBB999_262
	movq	56(%rcx,%rsi,8), %rcx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	je	.LBB999_259
.LBB999_254:
	movl	%edi, %r8d
	shll	$2, %r8d
	xorl	%esi, %esi
	.p2align	4
.LBB999_255:
	cmpl	8(%rcx,%rsi,4), %eax
	seta	%r9b
	sbbb	$0, %r9b
	cmpb	$1, %r9b
	jne	.LBB999_258
	incq	%rsi
	addq	$-4, %r8
	jne	.LBB999_255
	jmp	.LBB999_259
.LBB999_258:
	movzbl	%r9b, %edi
	testl	%edi, %edi
	jne	.LBB999_260
.LBB999_296:
	vmovsd	784(%rbx), %xmm0
	movl	$2, 784(%rbx)
	movl	%eax, 788(%rbx)
	vmovaps	%xmm0, 192(%rsp)
.Ltmp15919:
	.cfi_escape 0x2e, 0x00
	movq	120(%rsp), %rcx
	leaq	1136(%rsp), %rdi
	movq	%rbx, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15920:
	vmovaps	192(%rsp), %xmm0
	cmpl	$1, 1136(%rsp)
	vmovlps	%xmm0, 784(%rbx)
	jne	.LBB999_385
	vmovups	1152(%rsp), %zmm0
	vmovups	1184(%rsp), %zmm1
	movq	8(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.Ltmp15928:
	.cfi_escape 0x2e, 0x00
	leaq	560(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.13412714042204560522)
.Ltmp15929:
	movq	776(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB999_301
	movq	784(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB999_301:
	movq	704(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB999_303
	movq	712(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB999_303:
	movq	800(%rsp), %rax
	testq	%rax, %rax
	je	.LBB999_367
	lock		decq	(%rax)
	jne	.LBB999_367
	leaq	800(%rsp), %rdi
	jmp	.LBB999_366
.LBB999_262:
	vmovups	744(%rsp), %zmm1
	vmovdqu64	704(%rsp), %zmm0
	vmovups	%zmm1, 56(%rsp)
	vmovdqu64	%zmm0, 16(%rsp)
.Ltmp15930:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	120(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15931:
	cmpq	$-1, 16(%rsp)
	movq	%rax, 168(%rsp)
	movq	$0, 144(%rsp)
	movq	$8, 152(%rsp)
	movq	$0, 160(%rsp)
	je	.LBB999_266
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	leaq	144(%rsp), %rsi
	leaq	704(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	88(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB999_265
.LBB999_267:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	96(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_269
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_269:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_275
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_269
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
.LBB999_272:
	cmpq	%rax, %rsi
	jge	.LBB999_274
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB999_272
.LBB999_274:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_275:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	112(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB999_276
	jmp	.LBB999_278
.LBB999_266:
	movq	152(%rsp), %rcx
	movq	144(%rsp), %rax
	movq	160(%rsp), %rdx
	movq	%rcx, 304(%rsp)
	movq	168(%rsp), %rcx
	movq	%rax, 296(%rsp)
	movq	%rdx, 312(%rsp)
	movq	%rcx, 320(%rsp)
	movq	$-1, 288(%rsp)
	movq	88(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB999_267
.LBB999_265:
	movq	112(%rsp), %rax
	testq	%rax, %rax
	je	.LBB999_278
.LBB999_276:
	lock		decq	(%rax)
	jne	.LBB999_278
	leaq	112(%rsp), %rdi
	#MEMBARRIER
.Ltmp15935:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15936:
.LBB999_278:
	vmovdqu64	288(%rsp), %zmm0
	vmovups	320(%rsp), %zmm1
.LBB999_279:
	movq	8(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	.cfi_escape 0x2e, 0x00
	leaq	560(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.13412714042204560522)
	jmp	.LBB999_367
.LBB999_280:
	movq	904(%rsp), %r15
	movq	392(%rsp), %rax
	testq	%r15, %r15
	je	.LBB999_371
	lock		incq	(%r15)
	jle	.LBB999_416
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
	movq	%r15, 144(%rsp)
	leaq	16(%r15), %rsi
.Ltmp16014:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp16015:
	lock		incq	(%rbx)
	jle	.LBB999_416
.Ltmp16017:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp16018:
	vmovups	312(%rsp), %ymm1
	vmovdqu	288(%rsp), %ymm0
	vmovups	%ymm1, 56(%rsp)
	vmovdqu	%ymm0, 32(%rsp)
	movq	$1, 16(%rsp)
	movq	$1, 24(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB999_413
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB999_288
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_288:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_294
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_288
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
.LBB999_291:
	cmpq	%rax, %rdx
	jle	.LBB999_293
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB999_291
.LBB999_293:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_294:
	vmovdqu64	16(%rsp), %zmm0
	movq	80(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
	lock		decq	(%r15)
	jne	.LBB999_207
	movb	$1, %r12b
	#MEMBARRIER
.Ltmp16022:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16023:
	jmp	.LBB999_207
.LBB999_306:
	movl	136(%rsp), %eax
	movl	140(%rsp), %ecx
	xorl	%r14d, %r14d
	movl	%eax, 784(%r13)
	movl	%ecx, 788(%r13)
.Ltmp15972:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	leaq	808(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15973:
	vmovdqu64	16(%rsp), %zmm0
	vmovups	48(%rsp), %zmm1
	movq	8(%rsp), %rax
	xorl	%esi, %esi
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	movq	264(%rsp), %rdi
	movl	%esi, 128(%rsp)
	testq	%rdi, %rdi
	je	.LBB999_317
.LBB999_308:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	shlq	$2, %rdi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	cmpq	%rdi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_310
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_310:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_316
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_310
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rdi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rdi, %rcx
	setns	%al
	addq	%rdx, %rax
	subq	%rdi, %rcx
	cmovoq	%rax, %rcx
	movq	(%r15), %rax
	.p2align	4
.LBB999_313:
	cmpq	%rax, %rcx
	jge	.LBB999_315
	lock		cmpxchgq	%rcx, (%r15)
	jne	.LBB999_313
.LBB999_315:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_316:
	.cfi_escape 0x2e, 0x00
	movq	184(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB999_317:
	movq	224(%rsp), %rbx
	cmpq	$0, 192(%rsp)
	je	.LBB999_330
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r12
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	movabsq	$9223372036854775807, %r15
	xorl	%r14d, %r14d
	jmp	.LBB999_322
	.p2align	4
.LBB999_319:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_320:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	*%rbp
.LBB999_321:
	incq	%r14
	cmpq	192(%rsp), %r14
	je	.LBB999_330
.LBB999_322:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB999_321
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%r15, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_325
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_325:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_320
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_325
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
.LBB999_328:
	cmpq	%rax, %rdx
	jge	.LBB999_319
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB999_328
	jmp	.LBB999_319
.LBB999_330:
	movq	216(%rsp), %rax
	movl	128(%rsp), %r14d
	testq	%rax, %rax
	je	.LBB999_340
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_333
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_333:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_339
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_333
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
.LBB999_336:
	cmpq	%rax, %rsi
	jge	.LBB999_338
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB999_336
.LBB999_338:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_339:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB999_340:
	cmpq	$-1, 560(%rsp)
	je	.LBB999_342
.Ltmp16078:
	.cfi_escape 0x2e, 0x00
	leaq	560(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16079:
.LBB999_342:
	testb	%r14b, %r14b
	je	.LBB999_367
	movq	880(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB999_353
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_346
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_346:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_352
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_346
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
.LBB999_349:
	cmpq	%rax, %rsi
	jge	.LBB999_351
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB999_349
.LBB999_351:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_352:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB999_353:
	movq	808(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB999_363
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_356
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_356:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_362
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_356
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
.LBB999_359:
	cmpq	%rax, %rsi
	jge	.LBB999_361
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB999_359
.LBB999_361:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_362:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB999_363:
	movq	904(%rsp), %rax
	testq	%rax, %rax
	je	.LBB999_367
	lock		decq	(%rax)
	jne	.LBB999_367
	leaq	904(%rsp), %rdi
.LBB999_366:
	#MEMBARRIER
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB999_367:
	movq	8(%rsp), %rax
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
.LBB999_368:
	.cfi_def_cfa_offset 1312
.Ltmp16054:
	.cfi_escape 0x2e, 0x00
	movq	120(%rsp), %rcx
	movq	272(%rsp), %rdx
	leaq	1024(%rsp), %rdi
	movq	%rcx, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp16055:
	cmpl	$1, 1024(%rsp)
	jne	.LBB999_389
	vmovdqu64	1040(%rsp), %zmm0
	vmovups	1072(%rsp), %zmm1
	movq	8(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movb	$1, %al
	movl	%eax, 128(%rsp)
	jmp	.LBB999_317
.LBB999_371:
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
	movb	$1, %r12b
.Ltmp16033:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	120(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16034:
	movq	%rax, %rsi
	movq	%rax, %r15
	movq	%rax, 16(%rsp)
	addq	$16, %rsi
.Ltmp16035:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp16036:
	lock		decq	(%r15)
	jne	.LBB999_375
	#MEMBARRIER
.Ltmp16040:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.Ltmp16041:
.LBB999_375:
	lock		incq	(%rbx)
	jle	.LBB999_416
.Ltmp16042:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp16043:
	vmovups	312(%rsp), %ymm1
	vmovdqu	288(%rsp), %ymm0
	vmovups	%ymm1, 56(%rsp)
	vmovdqu	%ymm0, 32(%rsp)
	movq	$1, 16(%rsp)
	movq	$1, 24(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB999_415
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB999_380
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_380:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_206
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_380
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
.LBB999_383:
	cmpq	%rax, %rdx
	jle	.LBB999_205
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB999_383
	jmp	.LBB999_205
.LBB999_385:
	leaq	1144(%rsp), %rcx
.Ltmp15921:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	leaq	704(%rsp), %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15922:
	cmpq	$-1, 16(%rsp)
	je	.LBB999_396
	vmovups	704(%rsp), %zmm2
	vmovups	16(%rsp), %ymm0
	vmovups	744(%rsp), %zmm1
	vmovups	%zmm2, 16(%rsp)
	vmovups	%ymm0, 144(%rsp)
	vmovups	%zmm1, 56(%rsp)
	cmpq	$-1, 16(%rsp)
	je	.LBB999_400
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	leaq	144(%rsp), %rsi
	leaq	704(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB999_401
.LBB999_389:
	vmovups	1064(%rsp), %zmm1
	vmovdqu64	1032(%rsp), %zmm0
	vmovups	%zmm1, 48(%rsp)
	vmovdqu64	%zmm0, 16(%rsp)
.Ltmp16056:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	leaq	808(%rsp), %rsi
	leaq	16(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp16057:
	cmpq	$-1, 288(%rsp)
	je	.LBB999_398
	vmovdqu	288(%rsp), %ymm0
	vmovdqu	%ymm0, 144(%rsp)
	movq	168(%rsp), %rsi
	addq	$16, %rsi
.Ltmp16058:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)
.Ltmp16059:
	movq	400(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	jle	.LBB999_416
	movq	16(%rbx), %rdx
.Ltmp16061:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp16062:
	vmovups	312(%rsp), %ymm1
	vmovdqu	288(%rsp), %ymm0
	leaq	32(%rsp), %r14
	vmovups	%ymm1, 56(%rsp)
	vmovdqu	%ymm0, 32(%rsp)
	movq	$1, 16(%rsp)
	movq	$1, 24(%rsp)
.Ltmp16066:
	.cfi_escape 0x2e, 0x00
	movl	$8, %edi
	movl	$72, %esi
	vzeroupper
	callq	alloc::boxed::box_new_uninit
.Ltmp16067:
	movq	80(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 64(%rbx)
	vmovdqu64	16(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rbx)
.Ltmp16074:
	.cfi_escape 0x2e, 0x00
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16075:
	jmp	.LBB999_207
.LBB999_396:
	xorl	%ebp, %ebp
.Ltmp15925:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	leaq	704(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15926:
	vmovups	16(%rsp), %zmm0
	vmovups	48(%rsp), %zmm1
	jmp	.LBB999_279
.LBB999_398:
	xorl	%r12d, %r12d
.Ltmp16076:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
	leaq	808(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp16077:
	vmovdqu64	16(%rsp), %zmm0
	vmovups	48(%rsp), %zmm1
	movq	8(%rsp), %rax
	movl	$0, 128(%rsp)
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB999_317
.LBB999_400:
	vmovups	144(%rsp), %ymm0
	vmovups	%ymm0, 296(%rsp)
	movq	$-1, 288(%rsp)
.LBB999_401:
	movq	88(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB999_403
	movq	96(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB999_403:
	movq	112(%rsp), %rax
	testq	%rax, %rax
	je	.LBB999_278
	lock		decq	(%rax)
	jne	.LBB999_278
	xorl	%ebp, %ebp
	leaq	112(%rsp), %rdi
	#MEMBARRIER
.Ltmp15923:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15924:
	jmp	.LBB999_278
.LBB999_406:
	movq	80(%rbx), %rax
	movq	%r14, 16(%rsp)
	movq	%rax, 288(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 24(%rsp)
.Ltmp15961:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	anon.a9046ebb4da6daa9c28122cb9c29f82a.4165.llvm.11857615832421379041(%rip), %rdi
	leaq	anon.a9046ebb4da6daa9c28122cb9c29f82a.4166.llvm.11857615832421379041(%rip), %rdx
	leaq	16(%rsp), %rsi
	callq	*%rax
.Ltmp15962:
	jmp	.LBB999_416
.LBB999_407:
	movq	%r13, 680(%rsp)
.Ltmp15986:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.510(%rip), %rdx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15987:
	jmp	.LBB999_416
.LBB999_408:
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.512(%rip), %rcx
	movq	%rax, %rsi
	movq	%rcx, 448(%rsp)
.LBB999_409:
	movq	%r13, 24(%rsp)
.Ltmp15992:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	448(%rsp), %rdx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15993:
	jmp	.LBB999_416
.LBB999_410:
.Ltmp15949:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movl	$16, %esi
	callq	*%rax
.Ltmp15950:
	jmp	.LBB999_416
.LBB999_411:
.Ltmp16008:
	leaq	32(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16009:
	jmp	.LBB999_416
.LBB999_412:
	movl	$1, %edi
	jmp	.LBB999_18
.LBB999_413:
.Ltmp16024:
	leaq	32(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16025:
	jmp	.LBB999_416
.LBB999_414:
.Ltmp16099:
	leaq	32(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16100:
	jmp	.LBB999_416
.LBB999_415:
.Ltmp16048:
	leaq	32(%rsp), %r14
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16049:
.LBB999_416:
	ud2
.LBB999_417:
.Ltmp16068:
	movq	%rax, %rbx
.Ltmp16069:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16070:
	jmp	.LBB999_421
.LBB999_418:
.Ltmp16071:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_419:
.Ltmp16063:
	movq	%rax, %rbx
.Ltmp16064:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16065:
	jmp	.LBB999_421
.LBB999_420:
.Ltmp16060:
	movq	%rax, %rbx
.LBB999_421:
	movb	$1, %bpl
.Ltmp16072:
	.cfi_escape 0x2e, 0x00
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16073:
	movb	$1, %r14b
	jmp	.LBB999_488
.LBB999_422:
.Ltmp16050:
	movq	%rax, %rbx
.Ltmp16051:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16052:
	jmp	.LBB999_443
.LBB999_423:
.Ltmp16053:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_424:
.Ltmp16037:
	lock		decq	(%r15)
	movb	$1, %bpl
	movq	%rax, %rbx
	jne	.LBB999_431
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp16038:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.Ltmp16039:
	movb	$1, %r14b
	jmp	.LBB999_488
.LBB999_427:
.Ltmp16101:
	movq	%rax, %rbx
.Ltmp16102:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16103:
	jmp	.LBB999_443
.LBB999_428:
.Ltmp16104:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_429:
.Ltmp16086:
	lock		decq	(%r14)
	movb	$1, %bpl
	movq	%rax, %rbx
	jne	.LBB999_431
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp16087:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	16(%rsp), %rdi
	callq	*%rax
.Ltmp16088:
	movb	$1, %r14b
	jmp	.LBB999_488
.LBB999_432:
.Ltmp16044:
	movq	%rax, %rbx
.Ltmp16045:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16046:
	jmp	.LBB999_443
.LBB999_433:
.Ltmp16047:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_434:
.Ltmp16026:
	movq	%rax, %rbx
.Ltmp16027:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16028:
	jmp	.LBB999_438
.LBB999_435:
.Ltmp16029:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_436:
.Ltmp16019:
	movq	%rax, %rbx
.Ltmp16020:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16021:
	jmp	.LBB999_438
.LBB999_437:
.Ltmp16016:
	movq	%rax, %rbx
.LBB999_438:
	lock		decq	(%r15)
	movb	$1, %bpl
	jne	.LBB999_431
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp16030:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	callq	*%rax
.Ltmp16031:
	movb	$1, %r14b
	jmp	.LBB999_488
.LBB999_431:
	movb	$1, %r14b
	jmp	.LBB999_488
.LBB999_441:
.Ltmp16032:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_442:
.Ltmp16093:
	movq	%rax, %rbx
.Ltmp16094:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16095:
.LBB999_443:
	movb	$1, %bpl
	movb	$1, %r14b
	jmp	.LBB999_488
.LBB999_444:
.Ltmp15937:
	movq	%rax, %rbx
	xorl	%ebp, %ebp
	jmp	.LBB999_456
.LBB999_445:
.Ltmp16098:
	movq	%rax, %rbx
	xorl	%r14d, %r14d
	jmp	.LBB999_489
.LBB999_446:
.Ltmp16080:
	movq	%rax, %rbx
	jmp	.LBB999_491
.LBB999_447:
.Ltmp16083:
	movb	$1, %bpl
	movq	%rax, %rbx
	movl	%r12d, %r14d
	jmp	.LBB999_488
.LBB999_448:
.Ltmp15974:
	movq	%rax, %rbx
	jmp	.LBB999_486
.LBB999_449:
.Ltmp15932:
	movq	%rax, %rbx
.Ltmp15933:
	.cfi_escape 0x2e, 0x00
	leaq	704(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15934:
	xorl	%ebp, %ebp
	jmp	.LBB999_456
.LBB999_451:
.Ltmp16010:
	movq	%rax, %rbx
.Ltmp16011:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16012:
	movb	$1, %r14b
	xorl	%ebp, %ebp
	jmp	.LBB999_488
.LBB999_453:
.Ltmp16013:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_454:
.Ltmp15951:
	movq	%rax, %rbx
	jmp	.LBB999_492
.LBB999_455:
.Ltmp15927:
	movq	%rax, %rbx
.LBB999_456:
.Ltmp15938:
	.cfi_escape 0x2e, 0x00
	leaq	560(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.13412714042204560522)
.Ltmp15939:
	testb	%bpl, %bpl
	jne	.LBB999_495
	jmp	.LBB999_518
.LBB999_458:
.Ltmp15954:
	movq	%rax, %rbx
	testq	%r14, %r14
	je	.LBB999_492
	movq	264(%rsp), %rsi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	184(%rsp), %rdi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB999_492
.LBB999_460:
.Ltmp15948:
	movq	16(%rsp), %rsi
	movq	%rax, %rbx
	testq	%rsi, %rsi
	je	.LBB999_492
	movq	24(%rsp), %rdi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB999_492
.LBB999_462:
.Ltmp16002:
	vmovups	40(%rsp), %ymm1
	vmovdqu	16(%rsp), %ymm0
	movq	%rax, %rbx
	xorl	%ebp, %ebp
	vmovups	%ymm1, 584(%rsp)
	vmovdqu	%ymm0, 560(%rsp)
	jmp	.LBB999_480
.LBB999_463:
.Ltmp15977:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB999_480
.LBB999_464:
.Ltmp15982:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB999_479
.LBB999_465:
.Ltmp15997:
	movq	%rax, %rbx
	movq	%r13, 24(%rsp)
	jmp	.LBB999_475
.LBB999_466:
.Ltmp15991:
	movq	%rax, %rbx
	movq	%r13, 680(%rsp)
	cmpq	$5, %r12
	ja	.LBB999_472
	jmp	.LBB999_473
.LBB999_467:
.Ltmp15985:
	movq	%rax, %rbx
	movq	%r13, 680(%rsp)
	jmp	.LBB999_470
.LBB999_468:
.Ltmp16007:
	jmp	.LBB999_485
.LBB999_469:
.Ltmp15988:
	movq	%rax, %rbx
.LBB999_470:
	movq	16(%rsp), %r12
	cmpq	$6, %r12
	jb	.LBB999_473
	movq	24(%rsp), %rbp
.LBB999_472:
	leaq	-8(,%r12,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB999_473:
	.cfi_escape 0x2e, 0x00
	leaq	672(%rsp), %rdi
	jmp	.LBB999_478
.LBB999_474:
.Ltmp15994:
	movq	%rax, %rbx
.LBB999_475:
	cmpq	$5, %r15
	jbe	.LBB999_477
	leaq	-8(,%r15,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB999_477:
	.cfi_escape 0x2e, 0x00
	leaq	16(%rsp), %rdi
.LBB999_478:
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebp, %ebp
.LBB999_479:
.Ltmp15998:
	.cfi_escape 0x2e, 0x00
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp15999:
.LBB999_480:
	movq	440(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB999_482
	#MEMBARRIER
.Ltmp16003:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16004:
.LBB999_482:
	movb	$1, %r14b
	testb	%bpl, %bpl
	je	.LBB999_486
	.cfi_escape 0x2e, 0x00
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB999_486
.LBB999_484:
.Ltmp15963:
.LBB999_485:
	movq	%rax, %rbx
	movb	$1, %r14b
.LBB999_486:
	movq	264(%rsp), %rsi
	movb	$1, %bpl
	testq	%rsi, %rsi
	je	.LBB999_488
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	184(%rsp), %rdi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB999_488:
	.cfi_escape 0x2e, 0x00
	leaq	216(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	560(%rsp), %r13
.LBB999_489:
	cmpq	$-1, %r13
	setne	%al
	testb	%bpl, %al
	je	.LBB999_491
.Ltmp16105:
	.cfi_escape 0x2e, 0x00
	leaq	560(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16106:
.LBB999_491:
	testb	%r14b, %r14b
	je	.LBB999_518
.LBB999_492:
.Ltmp16107:
	.cfi_escape 0x2e, 0x00
	leaq	808(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp16108:
	jmp	.LBB999_518
.LBB999_493:
.Ltmp16109:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB999_494:
.Ltmp15942:
	movq	%rax, %rbx
.LBB999_495:
	movq	776(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB999_496
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	784(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_500
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_500:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_506
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_500
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
.LBB999_503:
	cmpq	%rax, %rsi
	jge	.LBB999_505
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB999_503
.LBB999_505:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_506:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	704(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB999_507
.LBB999_497:
	movq	800(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB999_516
	jmp	.LBB999_518
.LBB999_496:
	movq	704(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB999_497
.LBB999_507:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB999_509
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB999_509:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB999_515
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB999_509
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
.LBB999_512:
	cmpq	%rax, %rsi
	jge	.LBB999_514
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB999_512
.LBB999_514:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB999_515:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	800(%rsp), %rax
	testq	%rax, %rax
	je	.LBB999_518
.LBB999_516:
	lock		decq	(%rax)
	jne	.LBB999_518
	leaq	800(%rsp), %rdi
	#MEMBARRIER
.Ltmp15943:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15944:
.LBB999_518:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB999_519:
.Ltmp15945:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end999:
purrdf_sparql_eval::modifier::eval_group::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin1000:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception660
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
	subq	$2824, %rsp
	.cfi_def_cfa_offset 2880
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovdqu	anon.a12f493ba210922c94e5446ac885c35e.31.llvm.13412714042204560522(%rip), %ymm0
	movq	$0, 704(%rsp)
	movq	$8, 712(%rsp)
	movq	$0, 720(%rsp)
	movq	%r9, 120(%rsp)
	movq	%rdx, %rbx
	movq	%rsi, 160(%rsp)
	movq	%rdi, 168(%rsp)
	movq	%r8, 136(%rsp)
	vmovdqu	%ymm0, 728(%rsp)
	testq	%r8, %r8
	je	.LBB1000_5
	movq	136(%rsp), %r12
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r13
	movq	%rcx, %r14
	leaq	704(%rsp), %r15
	shlq	$4, %r12
	addq	%rcx, %r12
	.p2align	4
.LBB1000_2:
	movq	(%r14), %rsi
	movq	8(%r14), %rdx
	lock		incq	(%rsi)
	jle	.LBB1000_987
.Ltmp16110:
	movq	%r15, %rdi
	vzeroupper
	callq	*%r13
.Ltmp16111:
	addq	$16, %r14
	cmpq	%r12, %r14
	jne	.LBB1000_2
.LBB1000_5:
	vmovdqu	728(%rsp), %ymm0
	movq	704(%rsp), %rax
	movq	712(%rsp), %rcx
	movq	720(%rsp), %rdx
	movq	728(%rsp), %rsi
	movq	2880(%rsp), %rdi
	movq	%rax, 1648(%rsp)
	movq	%rcx, 1656(%rsp)
	movq	%rdx, 1664(%rsp)
	imulq	$120, %rdi, %rbp
	addq	120(%rsp), %rbp
	vmovdqu	%ymm0, 1672(%rsp)
	movq	%rsi, 1672(%rsp)
	movq	1664(%rsp), %rax
	movq	%rax, 360(%rsp)
	testq	%rdi, %rdi
	je	.LBB1000_11
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %r15
	movq	120(%rsp), %r12
	leaq	1648(%rsp), %r14
	.p2align	4
.LBB1000_7:
	movq	(%r12), %rsi
	movq	1664(%rsp), %r13
	lock		incq	(%rsi)
	jle	.LBB1000_987
	movq	8(%r12), %rdx
.Ltmp16116:
	movq	%r14, %rdi
	vzeroupper
	callq	*%r15
.Ltmp16117:
	cmpq	%r13, %rax
	jne	.LBB1000_65
	addq	$120, %r12
	cmpq	%rbp, %r12
	jne	.LBB1000_7
.LBB1000_11:
	movq	1648(%rsp), %rax
	vmovdqu	1680(%rsp), %xmm0
	movq	1672(%rsp), %rdi
	movq	1656(%rsp), %rcx
	movq	1664(%rsp), %rdx
	movq	1696(%rsp), %r8
	movq	1672(%rsp), %rsi
	movq	%rax, 720(%rsp)
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rdi, 744(%rsp)
	movl	$72, %edi
	movq	%r8, 768(%rsp)
	movq	%rcx, 728(%rsp)
	movq	%rdx, 736(%rsp)
	movq	%rsi, 744(%rsp)
	vmovdqu	%xmm0, 752(%rsp)
	movq	$1, 704(%rsp)
	movq	$1, 712(%rsp)
	vzeroupper
	callq	*%rax
	movq	%rax, 256(%rsp)
	testq	%rax, %rax
	je	.LBB1000_972
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_14
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB1000_14:
	movq	2888(%rsp), %r15
	leaq	712(%rsp), %r14
	.p2align	4
.LBB1000_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_21
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_15
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
.LBB1000_18:
	cmpq	%rax, %rcx
	jle	.LBB1000_20
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1000_18
.LBB1000_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_21:
	vmovdqu64	704(%rsp), %zmm0
	movq	256(%rsp), %rcx
	movq	768(%rsp), %rax
	movb	$1, %r12b
	movq	%rbp, 128(%rsp)
	movq	%rax, 64(%rcx)
	movq	%rcx, 1424(%rsp)
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16124:
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	160(%rsp), %rsi
	leaq	2240(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16125:
	movb	$1, %bpl
.Ltmp16126:
	leaq	2704(%rsp), %rdi
	movb	$1, %r12b
	movq	%rbx, %rsi
	movq	%r15, %rdx
	movq	%rbx, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp16127:
	cmpl	$1, 2704(%rsp)
	jne	.LBB1000_47
	vmovdqu64	2720(%rsp), %zmm0
	vmovdqu64	2752(%rsp), %zmm1
	movq	168(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movq	2312(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1000_34
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	2320(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_27
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_27:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_33
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_27
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
.LBB1000_30:
	cmpq	%rax, %rdx
	jge	.LBB1000_32
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_30
.LBB1000_32:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_33:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1000_34:
	movq	2240(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB1000_44
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	2248(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_37
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_37:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_43
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_37
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
.LBB1000_40:
	cmpq	%rax, %rdx
	jge	.LBB1000_42
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_40
.LBB1000_42:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_43:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1000_44:
	movq	2336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_614
	lock		decq	(%rax)
	jne	.LBB1000_614
	movb	$1, %r12b
	leaq	2336(%rsp), %rdi
	#MEMBARRIER
.Ltmp16524:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16525:
	jmp	.LBB1000_614
.LBB1000_47:
	vmovdqu64	2744(%rsp), %zmm1
	vmovdqu64	2712(%rsp), %zmm0
	vmovdqu64	%zmm1, 736(%rsp)
	vmovdqu64	%zmm0, 704(%rsp)
.Ltmp16128:
	leaq	704(%rsp), %r13
	leaq	1792(%rsp), %rdi
	leaq	2240(%rsp), %rsi
	movb	$1, %r12b
	xorl	%edx, %edx
	movq	%r13, %rcx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp16129:
	cmpq	$-1, 1792(%rsp)
	je	.LBB1000_75
	vmovdqu	1792(%rsp), %ymm0
	vmovdqu	%ymm0, 1168(%rsp)
	movq	1192(%rsp), %rax
	lock		incq	(%rax)
	jle	.LBB1000_987
	movq	256(%rsp), %rcx
	movq	1192(%rsp), %rax
	movq	360(%rsp), %rsi
	movq	32(%rcx), %rdx
	movq	%rax, 232(%rsp)
	cmpq	%rdx, %rsi
	ja	.LBB1000_904
	movq	%r14, 312(%rsp)
	movq	%rsi, %r14
	shlq	$4, %r14
	movq	%rsi, 352(%rsp)
	movq	%r14, 344(%rsp)
	testq	%rsi, %rsi
	je	.LBB1000_77
	movq	256(%rsp), %rax
	movq	%r14, %rdi
	movq	24(%rax), %rbx
	movq	malloc@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	%rax, 248(%rsp)
	testq	%rax, %rax
	je	.LBB1000_979
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%r14, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %r14
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmovbq	%r14, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_55
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_55:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_61
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_55
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rax
	movq	344(%rsp), %rdx
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
.LBB1000_58:
	cmpq	%rax, %rdx
	jle	.LBB1000_60
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_58
.LBB1000_60:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_61:
	movq	352(%rsp), %r15
	xorl	%r14d, %r14d
	.p2align	4
.LBB1000_62:
	movq	232(%rsp), %rdi
	leaq	(%rbx,%r14), %rsi
	addq	$16, %rdi
.Ltmp16130:
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.13412714042204560522)
.Ltmp16131:
	movq	248(%rsp), %rcx
	movq	%rax, (%rcx,%r14)
	movq	%rdx, 8(%rcx,%r14)
	addq	$16, %r14
	decq	%r15
	jne	.LBB1000_62
	movq	2888(%rsp), %r15
	jmp	.LBB1000_78
.LBB1000_65:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$51, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1000_974
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$51, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$51, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_68
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_68:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_74
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_68
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
.LBB1000_71:
	cmpq	%rax, %rsi
	jle	.LBB1000_73
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB1000_71
.LBB1000_73:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_74:
	vmovups	.Lanon.a12f493ba210922c94e5446ac885c35e.487+19(%rip), %ymm0
	vmovups	.Lanon.a12f493ba210922c94e5446ac885c35e.487(%rip), %ymm1
	movq	168(%rsp), %rbx
	movabsq	$9223372036854775793, %rax
	leaq	1648(%rsp), %rdi
	addq	$35, %rax
	movq	%rax, 16(%rbx)
	movq	$51, 24(%rbx)
	movq	%rcx, 32(%rbx)
	movq	$51, 40(%rbx)
	movq	$1, (%rbx)
	vmovups	%ymm0, 19(%rcx)
	vmovups	%ymm1, (%rcx)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
	movq	%rbx, %rax
	jmp	.LBB1000_872
.LBB1000_75:
	vmovups	2280(%rsp), %zmm1
	vmovups	2240(%rsp), %zmm0
	movq	256(%rsp), %rax
	movq	%rax, 1496(%rsp)
	movq	$0, 1472(%rsp)
	movq	$8, 1480(%rsp)
	movq	$0, 1488(%rsp)
	vmovups	%zmm1, 744(%rsp)
	vmovups	%zmm0, 704(%rsp)
	cmpq	$-1, 704(%rsp)
	je	.LBB1000_210
	leaq	1792(%rsp), %rdi
	leaq	1472(%rsp), %rsi
	leaq	2240(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	776(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB1000_211
	jmp	.LBB1000_220
.LBB1000_77:
	movl	$8, %eax
	movq	%rax, 248(%rsp)
.LBB1000_78:
	vmovdqu	anon.a12f493ba210922c94e5446ac885c35e.31.llvm.13412714042204560522(%rip), %ymm0
	movq	1184(%rsp), %rax
	vmovdqu	%ymm0, 640(%rsp)
	testq	%rax, %rax
	je	.LBB1000_183
	movq	248(%rsp), %rdx
	movq	344(%rsp), %rcx
	movq	1176(%rsp), %r14
	leaq	(%rax,%rax,4), %rax
	xorl	%r9d, %r9d
	leaq	(%rdx,%rcx), %rbp
	negq	%rcx
	leaq	(%r14,%rax,8), %rax
	movq	%rcx, 152(%rsp)
	movl	$2, %ecx
	movq	%rbp, 88(%rsp)
	vmovd	%ecx, %xmm0
	movq	%rax, 64(%rsp)
	vmovdqa	%xmm0, 32(%rsp)
	jmp	.LBB1000_82
	.p2align	4
.LBB1000_80:
	leaq	(%r12,%r15,8), %rax
	movq	48(%rsp), %r9
	movq	2888(%rsp), %r15
	movq	-16(%rax), %rcx
	movq	%r9, (%rcx,%rbx,8)
	incq	%rbx
	movq	%rbx, -8(%rax)
	addq	$40, %r14
	incq	%r9
	cmpq	64(%rsp), %r14
	je	.LBB1000_182
.LBB1000_82:
	cmpq	$5, 352(%rsp)
	movl	$1, %r12d
	leaq	712(%rsp), %rcx
	movl	$4, %eax
	movq	%r13, %rbx
	movq	%r9, 48(%rsp)
	movq	$1, 704(%rsp)
	jae	.LBB1000_94
	leaq	-1(%r12), %rdx
	cmpq	%rax, %rdx
	jae	.LBB1000_96
.LBB1000_84:
	movq	248(%rsp), %r9
	movq	152(%rsp), %r11
	leaq	8(%r14), %rdx
	incq	%rax
	xorl	%r8d, %r8d
	jmp	.LBB1000_86
	.p2align	4
.LBB1000_85:
	vmovq	%xmm0, -8(%rcx,%r12,8)
	addq	$16, %r9
	incq	%r12
	addq	$-16, %r8
	cmpq	%r12, %rax
	je	.LBB1000_93
.LBB1000_86:
	cmpq	%r8, %r11
	je	.LBB1000_92
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB1000_85
	movq	(%r14), %rsi
	movq	%rdx, %r10
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1000_90
	movq	16(%r14), %rsi
	movq	8(%r14), %r10
	decq	%rsi
.LBB1000_90:
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1000_960
	vmovq	(%r10,%rdi,8), %xmm0
	jmp	.LBB1000_85
	.p2align	4
.LBB1000_92:
	movq	%r12, (%rbx)
	jmp	.LBB1000_107
	.p2align	4
.LBB1000_93:
	movq	248(%rsp), %r15
	subq	%r8, %r15
	movq	%rax, (%rbx)
	cmpq	%rbp, %r15
	jne	.LBB1000_97
	jmp	.LBB1000_107
.LBB1000_94:
.Ltmp16135:
	movq	352(%rsp), %rdx
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%r13, %rdi
	xorl	%esi, %esi
	vzeroupper
	callq	*%rax
.Ltmp16136:
	movq	704(%rsp), %rax
	xorl	%edx, %edx
	movl	$4, %ecx
	leaq	712(%rsp), %rsi
	leaq	720(%rsp), %rdi
	movq	%r13, %rbx
	decq	%rax
	cmpq	$5, %rax
	cmovbq	%rcx, %rax
	movq	712(%rsp), %rcx
	setae	%dl
	cmovaeq	%rdi, %rbx
	cmovbq	%rsi, %rcx
	shll	$4, %edx
	movq	704(%rsp,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rax, %rdx
	jb	.LBB1000_84
	.p2align	4
.LBB1000_96:
	movq	248(%rsp), %r15
	movq	%r12, %rax
	movq	%rax, (%rbx)
	cmpq	%rbp, %r15
	je	.LBB1000_107
.LBB1000_97:
	leaq	8(%r14), %rbx
	.p2align	4
.LBB1000_98:
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%r15)
	vmovdqa	%xmm0, 96(%rsp)
	jne	.LBB1000_103
	movq	(%r14), %rsi
	movq	%rbx, %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1000_101
	movq	16(%r14), %rsi
	movq	8(%r14), %rax
	decq	%rsi
.LBB1000_101:
	movq	8(%r15), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1000_957
	vmovq	(%rax,%rdi,8), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
.LBB1000_103:
	movq	704(%rsp), %rsi
	movq	712(%rsp), %rax
	xorl	%edx, %edx
	leaq	712(%rsp), %rcx
	leaq	720(%rsp), %rdi
	decq	%rsi
	cmpq	$5, %rsi
	cmovbq	%rcx, %rax
	movq	%r13, %rcx
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	704(%rsp,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rsi, %rdx
	je	.LBB1000_105
.LBB1000_104:
	vmovdqa	96(%rsp), %xmm0
	addq	$16, %r15
	vmovq	%xmm0, -8(%rax,%r12,8)
	incq	%r12
	movq	%r12, (%rcx)
	cmpq	%rbp, %r15
	jne	.LBB1000_98
	jmp	.LBB1000_107
.LBB1000_105:
.Ltmp16144:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16145:
	cmpq	$6, 704(%rsp)
	movq	712(%rsp), %rax
	leaq	712(%rsp), %rcx
	leaq	720(%rsp), %rdx
	cmovbq	%rcx, %rax
	movq	%r13, %rcx
	cmovaeq	%rdx, %rcx
	jmp	.LBB1000_104
	.p2align	4
.LBB1000_107:
	vmovups	704(%rsp), %ymm0
	movq	736(%rsp), %rax
	movabsq	$2746377873070565055, %rsi
	movq	%rax, 2400(%rsp)
	vmovups	%ymm0, 2368(%rsp)
	vpbroadcastq	.LCPI1000_11(%rip), %xmm0
	movq	2368(%rsp), %rdi
	movq	2384(%rsp), %rax
	movq	2376(%rsp), %rbx
	leaq	-1(%rdi), %rcx
	movq	%rax, 72(%rsp)
	decq	%rax
	movq	%rdi, 96(%rsp)
	cmpq	$5, %rcx
	cmovbq	%rcx, %rax
	leaq	2376(%rsp), %rcx
	cmovaeq	%rbx, %rcx
	movq	%rax, %rdx
	xorq	%rsi, %rdx
	vpinsrq	$0, %rdx, %xmm0, %xmm0
	vaesenc	.LCPI1000_1(%rip), %xmm0, %xmm0
	testq	%rax, %rax
	je	.LBB1000_115
	leaq	(,%rax,8), %rsi
	addq	$-8, %rsi
	movl	%esi, %edi
	shrl	$3, %edi
	incl	%edi
	andl	$3, %edi
	je	.LBB1000_113
	shll	$3, %edi
	movq	%rcx, %rdx
	jmp	.LBB1000_111
	.p2align	4
.LBB1000_110:
	addq	$8, %rdx
	addq	$-8, %rdi
	je	.LBB1000_114
.LBB1000_111:
	movl	(%rdx), %r8d
	xorl	%r9d, %r9d
	cmpq	$2, %r8
	setne	%r9b
	vmovd	%r9d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI1000_1(%rip), %xmm0, %xmm0
	je	.LBB1000_110
	vmovdqa	.LCPI1000_1(%rip), %xmm2
	movl	4(%rdx), %r9d
	vmovq	%r8, %xmm1
	orl	$-2, %r8d
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%r8,%r9), %r8d
	vmovd	%r8d, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB1000_110
	.p2align	4
.LBB1000_113:
	movq	%rcx, %rdx
.LBB1000_114:
	cmpq	$24, %rsi
	jae	.LBB1000_168
.LBB1000_115:
	vaesenc	.LCPI1000_2(%rip), %xmm0, %xmm0
	movq	664(%rsp), %rdx
	movq	640(%rsp), %r12
	vaesenc	.LCPI1000_3(%rip), %xmm0, %xmm0
	movq	%rdx, 80(%rsp)
	movq	648(%rsp), %rdx
	vmovq	%xmm0, %r13
	movq	%r13, %rbp
	shrq	$57, %rbp
	vpbroadcastb	%ebp, %xmm0
	testq	%rax, %rax
	je	.LBB1000_134
	movq	2888(%rsp), %r15
	xorl	%esi, %esi
	movq	%r13, %rdi
.LBB1000_117:
	andq	%rdx, %rdi
	vmovdqu	(%r12,%rdi), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB1000_132
	kmovd	%k0, %r8d
	movq	%rbx, 56(%rsp)
.LBB1000_119:
	xorl	%ebx, %ebx
	tzcntl	%r8d, %ebx
	addq	%rdi, %rbx
	andq	%rdx, %rbx
	negq	%rbx
	leaq	(%rbx,%rbx,8), %r9
	movq	-72(%r12,%r9,8), %r10
	decq	%r10
	cmpq	$5, %r10
	jb	.LBB1000_121
	movq	-56(%r12,%r9,8), %r10
	movq	-64(%r12,%r9,8), %r9
	decq	%r10
	jmp	.LBB1000_122
	.p2align	4
.LBB1000_121:
	leaq	-64(%r12,%r9,8), %r9
.LBB1000_122:
	cmpq	%rax, %r10
	jne	.LBB1000_131
	xorl	%r10d, %r10d
	jmp	.LBB1000_125
	.p2align	4
.LBB1000_124:
	incq	%r10
	cmpq	%r10, %rax
	je	.LBB1000_143
.LBB1000_125:
	movl	(%r9,%r10,8), %r15d
	movl	(%rcx,%r10,8), %r11d
	cmpl	$2, %r15d
	je	.LBB1000_129
	cmpl	$2, %r11d
	je	.LBB1000_129
	cmpl	%r11d, %r15d
	jne	.LBB1000_131
	movl	4(%rcx,%r10,8), %r11d
	cmpl	%r11d, 4(%r9,%r10,8)
	je	.LBB1000_124
	jmp	.LBB1000_131
	.p2align	4
.LBB1000_129:
	cmpl	$2, %r15d
	jne	.LBB1000_131
	cmpl	$2, %r11d
	je	.LBB1000_124
.LBB1000_131:
	movq	2888(%rsp), %r15
	movq	56(%rsp), %rbx
	leal	-1(%r8), %r9d
	andw	%r8w, %r9w
	movl	%r9d, %r8d
	jne	.LBB1000_119
	.p2align	4
.LBB1000_132:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB1000_155
	leaq	16(%rdi,%rsi), %rdi
	addq	$16, %rsi
	jmp	.LBB1000_117
	.p2align	4
.LBB1000_134:
	movq	2888(%rsp), %r15
	xorl	%eax, %eax
	movq	%r13, %rcx
.LBB1000_135:
	andq	%rdx, %rcx
	vmovdqu	(%r12,%rcx), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB1000_141
	kmovd	%k0, %esi
	movq	%rbx, 56(%rsp)
.LBB1000_137:
	xorl	%ebx, %ebx
	tzcntl	%esi, %ebx
	addq	%rcx, %rbx
	andq	%rdx, %rbx
	negq	%rbx
	leaq	(%rbx,%rbx,8), %r8
	movq	-72(%r12,%r8,8), %rdi
	cmpq	$6, %rdi
	jb	.LBB1000_139
	movq	-56(%r12,%r8,8), %rdi
.LBB1000_139:
	cmpq	$1, %rdi
	je	.LBB1000_143
	movq	56(%rsp), %rbx
	leal	-1(%rsi), %edi
	andw	%si, %di
	movl	%edi, %esi
	jne	.LBB1000_137
	.p2align	4
.LBB1000_141:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB1000_155
	leaq	16(%rcx,%rax), %rcx
	addq	$16, %rax
	jmp	.LBB1000_135
	.p2align	4
.LBB1000_143:
	movq	96(%rsp), %rax
	movq	88(%rsp), %rbp
	leaq	704(%rsp), %r13
	cmpq	$6, %rax
	jb	.LBB1000_153
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_146
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_146:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_152
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_146
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
.LBB1000_149:
	cmpq	%rax, %rdx
	jge	.LBB1000_151
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_149
.LBB1000_151:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_152:
	movq	56(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1000_153:
	leaq	(%rbx,%rbx,8), %r15
	movq	-8(%r12,%r15,8), %rbx
	cmpq	-24(%r12,%r15,8), %rbx
	jne	.LBB1000_80
.Ltmp16147:
	movq	<alloc::raw_vec::RawVec<usize>>::grow_one@GOTPCREL(%rip), %rax
	leaq	-24(%r12,%r15,8), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16148:
	jmp	.LBB1000_80
	.p2align	4
.LBB1000_155:
	cmpq	$0, 656(%rsp)
	je	.LBB1000_180
.LBB1000_156:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$8, %edi
	vzeroupper
	callq	*%rax
	movq	48(%rsp), %r9
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rsi
	testq	%rax, %rax
	je	.LBB1000_967
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$8, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$8, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_159
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_165
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_159
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$8, (%rdx)
	movl	$8, %edx
	lock		xaddq	%rdx, (%rdi)
	addq	$8, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB1000_162:
	cmpq	%rax, %rdx
	jle	.LBB1000_164
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB1000_162
.LBB1000_164:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_165:
	movq	648(%rsp), %rdx
	movq	640(%rsp), %rax
	movq	%r9, (%rcx)
	andq	%rdx, %r13
	vmovdqu	(%rax,%r13), %xmm0
	vpmovmskb	%xmm0, %esi
	testl	%esi, %esi
	je	.LBB1000_178
.LBB1000_166:
	tzcntl	%esi, %esi
	addq	%r13, %rsi
	andq	%rdx, %rsi
	movzbl	(%rax,%rsi), %edi
	testb	%dil, %dil
	jns	.LBB1000_181
.LBB1000_167:
	leaq	-16(%rsi), %r8
	movb	%bpl, (%rax,%rsi)
	movq	72(%rsp), %r10
	negq	%rsi
	andb	$1, %dil
	vpbroadcastb	.LCPI1000_12(%rip), %xmm1
	leaq	704(%rsp), %r13
	andq	%rdx, %r8
	leaq	(%rsi,%rsi,8), %rdx
	movzbl	%dil, %esi
	movq	80(%rsp), %rdi
	movb	%bpl, 16(%rax,%r8)
	movq	96(%rsp), %r8
	movq	88(%rsp), %rbp
	movq	%r8, -72(%rax,%rdx,8)
	movq	%rbx, -64(%rax,%rdx,8)
	movq	%r10, -56(%rax,%rdx,8)
	leaq	2376(%rsp), %r8
	vmovups	16(%r8), %xmm0
	vpinsrq	$0, %rsi, %xmm1, %xmm1
	vmovups	%xmm0, -48(%rax,%rdx,8)
	movq	%rdi, -32(%rax,%rdx,8)
	movq	$1, -24(%rax,%rdx,8)
	movq	%rcx, -16(%rax,%rdx,8)
	movq	$1, -8(%rax,%rdx,8)
	vmovdqa	656(%rsp), %xmm0
	vpsubq	%xmm1, %xmm0, %xmm0
	vmovdqa	%xmm0, 656(%rsp)
	addq	$40, %r14
	incq	%r9
	cmpq	64(%rsp), %r14
	jne	.LBB1000_82
	jmp	.LBB1000_182
	.p2align	4
.LBB1000_168:
	leaq	(%rcx,%rax,8), %rsi
	jmp	.LBB1000_170
	.p2align	4
.LBB1000_169:
	addq	$32, %rdx
	cmpq	%rsi, %rdx
	je	.LBB1000_115
.LBB1000_170:
	movl	(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI1000_1(%rip), %xmm0, %xmm0
	je	.LBB1000_172
	vmovdqa	.LCPI1000_1(%rip), %xmm2
	movl	4(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB1000_172:
	movl	8(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI1000_1(%rip), %xmm0, %xmm0
	je	.LBB1000_174
	vmovdqa	.LCPI1000_1(%rip), %xmm2
	movl	12(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB1000_174:
	movl	16(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI1000_1(%rip), %xmm0, %xmm0
	je	.LBB1000_176
	vmovdqa	.LCPI1000_1(%rip), %xmm2
	movl	20(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB1000_176:
	movl	24(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI1000_1(%rip), %xmm0, %xmm0
	je	.LBB1000_169
	vmovdqa	.LCPI1000_1(%rip), %xmm2
	movl	28(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB1000_169
.LBB1000_178:
	movl	$16, %edi
.LBB1000_179:
	addq	%rdi, %r13
	addq	$16, %rdi
	andq	%rdx, %r13
	vmovdqu	(%rax,%r13), %xmm0
	vpmovmskb	%xmm0, %esi
	testl	%esi, %esi
	jne	.LBB1000_166
	jmp	.LBB1000_179
.LBB1000_180:
.Ltmp16150:
	movq	<hashbrown::raw::RawTable<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>))>>::reserve_rehash::<hashbrown::map::make_hasher<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %esi
	leaq	640(%rsp), %rdi
	movl	$1, %ecx
	vzeroupper
	callq	*%rax
.Ltmp16151:
	jmp	.LBB1000_156
.LBB1000_181:
	vmovdqa	(%rax), %xmm0
	vpmovmskb	%xmm0, %esi
	tzcntl	%esi, %esi
	movzbl	(%rax,%rsi), %edi
	jmp	.LBB1000_167
.LBB1000_182:
	movq	136(%rsp), %rax
	orq	664(%rsp), %rax
	movq	%rax, 136(%rsp)
.LBB1000_183:
	cmpq	$0, 2880(%rsp)
	je	.LBB1000_226
	cmpq	$0, 136(%rsp)
	jne	.LBB1000_226
	cmpq	$0, 656(%rsp)
	je	.LBB1000_955
.LBB1000_186:
	vmovdqa	.LCPI1000_5(%rip), %xmm0
	movq	640(%rsp), %rax
	movq	648(%rsp), %rdx
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	xorl	%r9d, %r9d
	xorl	%r8d, %r8d
	vaesenc	.LCPI1000_1(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1000_2(%rip), %xmm0, %xmm0
	vaesenc	.LCPI1000_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %rdi
	movq	%rdi, %rcx
	shrq	$57, %rcx
	vpbroadcastb	%ecx, %xmm0
.LBB1000_187:
	andq	%rdx, %rdi
	vmovdqu	(%rax,%rdi), %xmm2
	vpcmpeqb	%xmm0, %xmm2, %k0
	kortestw	%k0, %k0
	je	.LBB1000_193
	kmovd	%k0, %r10d
.LBB1000_189:
	xorl	%r11d, %r11d
	tzcntl	%r10d, %r11d
	addq	%rdi, %r11
	andq	%rdx, %r11
	negq	%r11
	leaq	(%r11,%r11,8), %r11
	movq	-72(%rax,%r11,8), %rbx
	cmpq	$6, %rbx
	jb	.LBB1000_191
	movq	-56(%rax,%r11,8), %rbx
.LBB1000_191:
	cmpq	$1, %rbx
	je	.LBB1000_200
	leal	-1(%r10), %r11d
	andw	%r10w, %r11w
	movl	%r11d, %r10d
	jne	.LBB1000_189
.LBB1000_193:
	cmpq	$1, %r9
	je	.LBB1000_196
	vpmovmskb	%xmm2, %esi
	testl	%esi, %esi
	je	.LBB1000_198
	tzcntl	%esi, %esi
	addq	%rdi, %rsi
	andq	%rdx, %rsi
.LBB1000_196:
	vpcmpeqb	%xmm1, %xmm2, %k0
	kortestw	%k0, %k0
	jne	.LBB1000_224
	movl	$1, %r9d
	jmp	.LBB1000_199
.LBB1000_198:
	xorl	%r9d, %r9d
.LBB1000_199:
	leaq	16(%r8,%rdi), %rdi
	addq	$16, %r8
	jmp	.LBB1000_187
.LBB1000_200:
	leaq	(%rax,%r11,8), %rax
	vpxor	%xmm0, %xmm0, %xmm0
	movq	-24(%rax), %rcx
	movq	-16(%rax), %rdi
	vmovdqu	%xmm0, -32(%rax)
	movq	$8, -16(%rax)
	movq	$0, -8(%rax)
	leaq	-1(%rcx), %rax
	cmpq	$-3, %rax
	ja	.LBB1000_226
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	shlq	$3, %rcx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_203
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_203:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_209
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_203
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
.LBB1000_206:
	cmpq	%rax, %rdx
	jge	.LBB1000_208
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_206
.LBB1000_208:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_209:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB1000_226
.LBB1000_210:
	movq	1480(%rsp), %rcx
	movq	1472(%rsp), %rax
	movq	1488(%rsp), %rdx
	movq	%rcx, 1808(%rsp)
	movq	1496(%rsp), %rcx
	movq	%rax, 1800(%rsp)
	movq	%rdx, 1816(%rsp)
	movq	%rcx, 1824(%rsp)
	movq	$-1, 1792(%rsp)
	movq	776(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1000_220
.LBB1000_211:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	784(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_213
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_213:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_219
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_213
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
.LBB1000_216:
	cmpq	%rax, %rdx
	jge	.LBB1000_218
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_216
.LBB1000_218:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_219:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1000_220:
	movq	800(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_223
	lock		decq	(%rax)
	jne	.LBB1000_223
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	800(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB1000_223:
	vmovdqu64	1792(%rsp), %zmm0
	vmovdqu64	1824(%rsp), %zmm1
	movq	168(%rsp), %rax
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB1000_872
.LBB1000_224:
	movzbl	(%rax,%rsi), %edi
	testb	%dil, %dil
	jns	.LBB1000_961
.LBB1000_225:
	leaq	-16(%rsi), %r8
	movb	%cl, (%rax,%rsi)
	vpbroadcastb	.LCPI1000_12(%rip), %xmm1
	andb	$1, %dil
	negq	%rsi
	andq	%rdx, %r8
	movzbl	%dil, %edi
	movb	%cl, 16(%rax,%r8)
	leaq	(%rsi,%rsi,8), %rcx
	vmovdqa	656(%rsp), %xmm0
	vpinsrq	$0, %rdi, %xmm1, %xmm1
	vpsubq	%xmm1, %xmm0, %xmm0
	vmovdqa	%xmm0, 656(%rsp)
	movq	$1, -72(%rax,%rcx,8)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, -32(%rax,%rcx,8)
	movq	$8, -16(%rax,%rcx,8)
	movq	$0, -8(%rax,%rcx,8)
.LBB1000_226:
	movq	640(%rsp), %rcx
	movq	648(%rsp), %rsi
	movq	664(%rsp), %rax
	vmovdqa	(%rcx), %xmm0
	testq	%rsi, %rsi
	je	.LBB1000_228
	leaq	(,%rsi,8), %rdx
	movq	%rcx, %r8
	movl	$16, %r9d
	leaq	(%rdx,%rdx,8), %rdx
	andq	$-16, %rdx
	subq	%rdx, %r8
	leaq	97(%rdx,%rsi), %rdi
	addq	$-80, %r8
	jmp	.LBB1000_229
.LBB1000_228:
	xorl	%r9d, %r9d
.LBB1000_229:
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	leaq	1(%rcx,%rsi), %rsi
	leaq	16(%rcx), %rdx
	movq	%r9, 1472(%rsp)
	movq	%rdi, 1480(%rsp)
	movq	%r8, 1488(%rsp)
	movq	%rcx, 1496(%rsp)
	vpcmpgtb	%xmm1, %xmm0, %k0
	movq	%rdx, 1504(%rsp)
	movq	%rsi, 1512(%rsp)
	kmovw	%k0, 1520(%rsp)
	movq	%rax, 1528(%rsp)
	testq	%rax, %rax
	je	.LBB1000_239
	kortestw	%k0, %k0
	je	.LBB1000_232
	kmovd	%k0, %esi
	jmp	.LBB1000_235
.LBB1000_232:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	.p2align	4
.LBB1000_233:
	vpcmpltb	(%rdx), %xmm0, %k0
	addq	$-1152, %rcx
	addq	$16, %rdx
	kortestw	%k0, %k0
	je	.LBB1000_233
	kmovd	%k0, %esi
	movq	%rdx, 1504(%rsp)
	movq	%rcx, 1496(%rsp)
.LBB1000_235:
	xorl	%edx, %edx
	blsrl	%esi, %edx
	tzcntl	%esi, %esi
	leaq	-1(%rax), %rdi
	negq	%rsi
	movw	%dx, 1520(%rsp)
	movq	%rdi, 1528(%rsp)
	leaq	(%rsi,%rsi,8), %rdx
	movq	-24(%rcx,%rdx,8), %rbx
	cmpq	$-1, %rbx
	je	.LBB1000_239
	leaq	(%rcx,%rdx,8), %rcx
	cmpq	$5, %rax
	movl	$4, %ebp
	cmovaeq	%rax, %rbp
	movq	-40(%rcx), %rsi
	movq	-64(%rcx), %rdx
	movq	-32(%rcx), %rdi
	movq	-72(%rcx), %r15
	movq	%rsi, 384(%rsp)
	movq	%rdx, 32(%rsp)
	leaq	(,%rbp,8), %rdx
	vmovdqu	-56(%rcx), %xmm0
	leaq	(%rdx,%rdx,8), %r14
	movabsq	$128102389400760776, %rdx
	decq	%rdx
	vmovdqa	%xmm0, 368(%rsp)
	movq	-16(%rcx), %rsi
	movq	%rsi, 96(%rsp)
	cmpq	%rdx, %rax
	jbe	.LBB1000_240
	xorl	%r13d, %r13d
.LBB1000_238:
.Ltmp16164:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r14, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16165:
	jmp	.LBB1000_987
.LBB1000_239:
	leaq	1472(%rsp), %rdi
	movq	$0, 528(%rsp)
	movq	$8, 536(%rsp)
	movq	$0, 544(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>>
	jmp	.LBB1000_256
.LBB1000_240:
	movq	-8(%rcx), %r12
	testq	%r14, %r14
	je	.LBB1000_242
	movl	$8, %esi
	movq	%r15, 48(%rsp)
	movq	%rdi, %r15
	movq	%r14, %rdi
	movl	$8, %r13d
	vzeroupper
	callq	__rustc::__rust_alloc
	movq	%r15, %rdi
	movq	48(%rsp), %r15
	testq	%rax, %rax
	jne	.LBB1000_243
	jmp	.LBB1000_238
.LBB1000_242:
	movl	$8, %eax
	xorl	%ebp, %ebp
.LBB1000_243:
	movq	32(%rsp), %rcx
	movq	%r15, (%rax)
	movq	96(%rsp), %rdx
	movq	%rcx, 8(%rax)
	vmovaps	368(%rsp), %xmm0
	vmovups	%xmm0, 16(%rax)
	movq	384(%rsp), %rcx
	movq	%rcx, 32(%rax)
	movq	%rdi, 40(%rax)
	movq	%rbx, 48(%rax)
	movq	%rdx, 56(%rax)
	movq	%r12, 64(%rax)
	movq	%rbp, 176(%rsp)
	movq	%rax, 184(%rsp)
	movq	$1, 192(%rsp)
	vmovdqu64	1472(%rsp), %zmm0
	vmovdqu64	%zmm0, 1792(%rsp)
	movq	1848(%rsp), %rdx
	testq	%rdx, %rdx
	je	.LBB1000_255
	movzwl	1840(%rsp), %ebp
	movq	1816(%rsp), %r15
	movq	1824(%rsp), %r12
	leaq	760(%rsp), %r13
	movl	$1, %ebx
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	jmp	.LBB1000_247
	.p2align	4
.LBB1000_245:
	movq	184(%rsp), %rax
.LBB1000_246:
	movq	768(%rsp), %rdx
	leaq	(%rbx,%rbx,8), %rcx
	incq	%rbx
	movq	%rdx, 64(%rax,%rcx,8)
	movq	%r14, %rdx
	vmovdqu64	704(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rax,%rcx,8)
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movq	%rbx, 192(%rsp)
	testq	%r14, %r14
	je	.LBB1000_253
.LBB1000_247:
	testw	%bp, %bp
	jne	.LBB1000_250
	.p2align	4
.LBB1000_248:
	vpcmpltb	(%r12), %xmm0, %k0
	addq	$-1152, %r15
	addq	$16, %r12
	kortestw	%k0, %k0
	je	.LBB1000_248
	kmovd	%k0, %ebp
.LBB1000_250:
	xorl	%ecx, %ecx
	tzcntl	%ebp, %ecx
	leaq	-1(%rdx), %r14
	blsrl	%ebp, %ebp
	negq	%rcx
	leaq	(%rcx,%rcx,8), %rsi
	movq	-24(%r15,%rsi,8), %rcx
	cmpq	$-1, %rcx
	je	.LBB1000_254
	leaq	(%r15,%rsi,8), %rsi
	vmovups	-16(%rsi), %xmm0
	movq	-32(%rsi), %rdi
	vmovaps	%xmm0, 1264(%rsp)
	vmovdqu	-72(%rsi), %ymm1
	movq	-40(%rsi), %rsi
	movq	%rsi, 736(%rsp)
	vmovdqu	%ymm1, 704(%rsp)
	movq	%rdi, 744(%rsp)
	movq	%rcx, 752(%rsp)
	vmovups	%xmm0, (%r13)
	cmpq	176(%rsp), %rbx
	jne	.LBB1000_246
.Ltmp16159:
	movl	$8, %ecx
	movl	$72, %r8d
	leaq	176(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp16160:
	jmp	.LBB1000_245
.LBB1000_253:
	xorl	%r14d, %r14d
.LBB1000_254:
	movq	%r12, 1824(%rsp)
	movq	%r15, 1816(%rsp)
	movw	%bp, 1840(%rsp)
	movq	%r14, 1848(%rsp)
.LBB1000_255:
	leaq	1792(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>>
	vmovdqu	176(%rsp), %xmm0
	movq	192(%rsp), %rsi
	movq	2888(%rsp), %r15
	movq	%rsi, 544(%rsp)
	vmovdqa	%xmm0, 528(%rsp)
	cmpq	$2, %rsi
	jae	.LBB1000_915
.LBB1000_256:
	movq	256(%rsp), %rax
	cmpb	$2, 472(%r15)
	movq	32(%rax), %rax
	movq	%rax, 1216(%rsp)
	jne	.LBB1000_300
	movq	616(%r15), %rsi
	testq	%rsi, %rsi
	je	.LBB1000_264
	cmpq	$-2, 24(%rsi)
	jb	.LBB1000_300
	cmpq	$-2, 32(%rsi)
	jb	.LBB1000_300
	cmpq	$-2, 48(%rsi)
	jb	.LBB1000_300
.LBB1000_264:
	cmpq	$0, 2880(%rsp)
	je	.LBB1000_299
	movq	120(%rsp), %rbp
	leaq	584(%r15), %r12
	leaq	<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0}(%rip), %r14
	leaq	704(%rsp), %rbx
.LBB1000_266:
	movq	56(%rbp), %r15
	testq	%r15, %r15
	je	.LBB1000_279
	movq	48(%rbp), %r13
	shlq	$6, %r15
	jmp	.LBB1000_269
	.p2align	4
.LBB1000_268:
	addq	$64, %r13
	addq	$-64, %r15
	je	.LBB1000_279
.LBB1000_269:
	movq	2888(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB1000_271
	movq	%r12, 704(%rsp)
.Ltmp16167:
	xorl	%edi, %edi
	movq	%r13, %rsi
	movq	%rbx, %r8
	movq	%r14, %r9
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.13412714042204560522)
.Ltmp16168:
	jmp	.LBB1000_272
.LBB1000_271:
.Ltmp16169:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.13412714042204560522)(%rip), %r9
	xorl	%edi, %edi
	movq	%r13, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.13412714042204560522)
.Ltmp16170:
.LBB1000_272:
	testb	%al, %al
	jne	.LBB1000_298
	movq	2888(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB1000_268
	cmpq	$0, 336(%rax)
	jne	.LBB1000_277
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB1000_277
	cmpq	$-1, 48(%rax)
	je	.LBB1000_268
.LBB1000_277:
.Ltmp16171:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16172:
	testb	%al, %al
	je	.LBB1000_268
	jmp	.LBB1000_298
.LBB1000_279:
	movq	104(%rbp), %rax
	testq	%rax, %rax
	je	.LBB1000_292
	movq	96(%rbp), %r13
	shlq	$3, %rax
	leaq	(%rax,%rax,8), %r15
	addq	$8, %r13
	jmp	.LBB1000_282
	.p2align	4
.LBB1000_281:
	addq	$72, %r13
	addq	$-72, %r15
	je	.LBB1000_292
.LBB1000_282:
	movq	2888(%rsp), %rax
	cmpq	$0, 584(%rax)
	movq	672(%rax), %rdx
	movq	680(%rax), %rcx
	je	.LBB1000_284
	movq	%r12, 704(%rsp)
.Ltmp16174:
	xorl	%edi, %edi
	movq	%r13, %rsi
	movq	%rbx, %r8
	movq	%r14, %r9
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.13412714042204560522)
.Ltmp16175:
	jmp	.LBB1000_285
.LBB1000_284:
.Ltmp16176:
	movl	$1, %r8d
	leaq	purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.13412714042204560522)(%rip), %r9
	xorl	%edi, %edi
	movq	%r13, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.13412714042204560522)
.Ltmp16177:
.LBB1000_285:
	testb	%al, %al
	jne	.LBB1000_298
	movq	2888(%rsp), %rax
	movq	616(%rax), %rax
	testq	%rax, %rax
	je	.LBB1000_281
	cmpq	$0, 336(%rax)
	jne	.LBB1000_290
	vpcmpeqd	%ymm0, %ymm0, %ymm0
	vpcmpneqq	16(%rax), %ymm0, %k0
	kmovd	%k0, %ecx
	testb	$15, %cl
	jne	.LBB1000_290
	cmpq	$-1, 48(%rax)
	je	.LBB1000_281
.LBB1000_290:
.Ltmp16178:
	movq	purrdf_sparql_eval::parallel::expression_re_enters_evaluation@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16179:
	testb	%al, %al
	je	.LBB1000_281
	jmp	.LBB1000_298
.LBB1000_292:
	cmpl	$8, 16(%rbp)
	movq	2888(%rsp), %r15
	jne	.LBB1000_297
	movq	24(%rbp), %rsi
	movq	32(%rbp), %rdx
	movq	688(%r15), %rdi
	addq	$16, %rsi
.Ltmp16181:
	movq	<purrdf_sparql_eval::agg_fn::AggregateRegistry>::resolve@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16182:
	testq	%rax, %rax
	je	.LBB1000_300
	movq	(%rax), %rcx
	movq	8(%rax), %rax
	movq	16(%rax), %rdx
	movq	32(%rax), %rax
	decq	%rdx
	andq	$-16, %rdx
	leaq	16(%rcx,%rdx), %rdi
.Ltmp16183:
	callq	*%rax
.Ltmp16184:
	testb	%al, %al
	jne	.LBB1000_300
.LBB1000_297:
	addq	$120, %rbp
	movb	$1, %r13b
	cmpq	128(%rsp), %rbp
	jne	.LBB1000_266
	jmp	.LBB1000_301
.LBB1000_298:
	movq	2888(%rsp), %r15
.LBB1000_300:
	xorl	%r13d, %r13d
.LBB1000_301:
	movq	2880(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_389
	movq	232(%rsp), %rcx
	movq	malloc@GOTPCREL(%rip), %r14
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,2), %rbx
	movq	%rbx, %rdi
	movq	%rcx, 96(%rsp)
	vzeroupper
	callq	*%r14
	movq	%rax, 88(%rsp)
	testq	%rax, %rax
	je	.LBB1000_981
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdx
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rsi
	movq	$-1, %r8
	movq	%rbx, 64(%rsp)
	leaq	(%rbx,%rax), %rcx
	sarq	$63, %rcx
	xorq	%rsi, %rcx
	addq	%rbx, %rax
	cmovoq	%rcx, %rax
	incq	%rdx
	cmoveq	%r8, %rdx
	addq	%rbx, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_305
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB1000_305:
	addq	$16, 96(%rsp)
	.p2align	4
.LBB1000_306:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_312
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_306
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rax
	movq	64(%rsp), %rdx
	lock		addq	%rdx, (%rax)
	movq	%rdx, %rcx
	lock		xaddq	%rcx, (%rdi)
	leaq	(%rcx,%rdx), %rax
	sarq	$63, %rax
	xorq	%rsi, %rax
	addq	%rdx, %rcx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1000_309:
	cmpq	%rax, %rcx
	jle	.LBB1000_311
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1000_309
.LBB1000_311:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_312:
	leaq	anon.a12f493ba210922c94e5446ac885c35e.775.llvm.13412714042204560522(%rip), %rax
	movabsq	$42700796466920258, %rdx
	movq	$-1, %r12
	movl	%r13d, 152(%rsp)
	xorl	%r13d, %r13d
	movq	$0, 56(%rsp)
	movq	%rax, 136(%rsp)
.LBB1000_313:
	movq	120(%rsp), %rcx
	imulq	$120, %r13, %rax
	movq	56(%rcx,%rax), %r15
	movq	104(%rcx,%rax), %rsi
	leaq	(%rsi,%r15), %r14
	imulq	$216, %r14, %rbx
	cmpq	%rdx, %r14
	ja	.LBB1000_985
	movq	48(%rcx,%rax), %rdx
	movq	96(%rcx,%rax), %rax
	movq	%rsi, 48(%rsp)
	movq	%rdx, 72(%rsp)
	movq	%rax, 80(%rsp)
	testq	%rbx, %rbx
	je	.LBB1000_324
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1000_984
	movq	%rax, %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rbx, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %rbx
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmovbq	%rbx, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_318
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_318:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_325
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_318
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbx, (%rdx)
	movq	%rcx, %rdx
	lock		xaddq	%rdx, (%rsi)
	movabsq	$-9223372036854775808, %rsi
	leaq	(%rdx,%rcx), %rax
	sarq	$63, %rax
	xorq	%rsi, %rax
	addq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB1000_321:
	cmpq	%rax, %rdx
	jle	.LBB1000_323
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_321
.LBB1000_323:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jmp	.LBB1000_325
.LBB1000_324:
	movl	$8, %edi
	xorl	%r14d, %r14d
.LBB1000_325:
	movq	%r14, 1472(%rsp)
	movq	%rdi, 32(%rsp)
	movq	%rdi, 1480(%rsp)
	testq	%r15, %r15
	je	.LBB1000_356
	xorl	%r14d, %r14d
	.p2align	4
.LBB1000_327:
	movq	2888(%rsp), %rax
	movq	%r14, %rbx
	shlq	$6, %rbx
	addq	72(%rsp), %rbx
	movq	352(%rax), %rax
	testq	%rax, %rax
	je	.LBB1000_343
	movq	192(%rax), %rcx
	xorl	%ebp, %ebp
	cmpq	160(%rsp), %rcx
	jne	.LBB1000_333
.LBB1000_329:
.Ltmp16189:
	movq	160(%rsp), %rdi
	movq	purrdf_sparql_eval::vm::attached_position@GOTPCREL(%rip), %rax
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16190:
	cmpq	$1, %rax
	jne	.LBB1000_343
	movq	2888(%rsp), %rax
	movq	352(%rax), %rdi
	addq	$16, %rdi
.Ltmp16191:
	movq	<purrdf_sparql_eval::plan::PlanShape>::program@GOTPCREL(%rip), %rax
	movl	%ebp, %esi
	movq	%rbx, %rcx
	callq	*%rax
.Ltmp16192:
	movq	%rax, %rsi
	jmp	.LBB1000_354
	.p2align	4
.LBB1000_333:
	movq	32(%rax), %rsi
	testq	%rsi, %rsi
	je	.LBB1000_343
	movq	24(%rax), %rax
	cmpq	$1, %rsi
	jne	.LBB1000_336
	xorl	%ecx, %ecx
	jmp	.LBB1000_339
.LBB1000_336:
	movq	%rsi, %rdx
	xorl	%edi, %edi
	.p2align	4
.LBB1000_337:
	movq	%rdx, %r8
	shrq	%r8
	leaq	(%r8,%rdi), %rcx
	leaq	(%rcx,%rcx,4), %r9
	movl	32(%rax,%r9,8), %ebp
	cmpq	%rbp, %rsi
	jbe	.LBB1000_976
	movq	160(%rsp), %r10
	leaq	(%rbp,%rbp,4), %r9
	cmpq	%r10, 16(%rax,%r9,8)
	cmovaq	%rdi, %rcx
	subq	%r8, %rdx
	movq	%rcx, %rdi
	cmpq	$1, %rdx
	ja	.LBB1000_337
.LBB1000_339:
	leaq	(%rcx,%rcx,4), %rdx
	movl	32(%rax,%rdx,8), %ebp
	cmpq	%rbp, %rsi
	jbe	.LBB1000_976
	movq	160(%rsp), %rdi
	leaq	(%rbp,%rbp,4), %rdx
	cmpq	%rdi, 16(%rax,%rdx,8)
	jne	.LBB1000_343
	cmpq	%rsi, %rcx
	jae	.LBB1000_975
	testl	%ebp, %ebp
	jne	.LBB1000_329
	.p2align	4
.LBB1000_343:
	vmovdqa	.LCPI1000_6(%rip), %ymm0
	movq	$1, 1976(%rsp)
	movq	$1, 2016(%rsp)
	movq	$0, 1792(%rsp)
	movq	$8, 1800(%rsp)
	movq	$0, 1808(%rsp)
	movq	$1, 2136(%rsp)
	vmovdqu	%ymm0, 1816(%rsp)
	movl	$0, 2224(%rsp)
.Ltmp16193:
	leaq	1792(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::vm::compile::ExprProgram>::recompile
.Ltmp16194:
	vmovdqu64	2168(%rsp), %zmm0
	vmovups	2112(%rsp), %zmm2
	vmovups	2048(%rsp), %zmm1
	leaq	720(%rsp), %rax
	vmovdqu64	1792(%rsp), %zmm4
	vmovdqu64	1984(%rsp), %zmm3
	movq	malloc@GOTPCREL(%rip), %rbp
	movl	$456, %edi
	vmovdqu64	%zmm0, 376(%rax)
	vmovups	%zmm2, 320(%rax)
	vmovups	%zmm1, 256(%rax)
	vmovdqu64	1856(%rsp), %zmm1
	vmovdqu64	1920(%rsp), %zmm2
	vmovdqu64	%zmm3, 192(%rax)
	vmovdqu64	%zmm4, (%rax)
	vmovdqu64	%zmm2, 128(%rax)
	vmovdqu64	%zmm1, 64(%rax)
	movq	$1, 704(%rsp)
	movq	$1, 712(%rsp)
	vzeroupper
	callq	*%rbp
	testq	%rax, %rax
	je	.LBB1000_968
	movq	%rax, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	movl	$456, %edx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rdx, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rdx, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_347
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_347:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_353
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_347
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$456, (%rcx)
	movl	$456, %ecx
	lock		xaddq	%rcx, (%rdi)
	addq	$456, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1000_350:
	cmpq	%rax, %rcx
	jle	.LBB1000_352
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1000_350
.LBB1000_352:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_353:
	vmovups	1024(%rsp), %zmm2
	vmovups	960(%rsp), %zmm1
	vmovdqu64	1088(%rsp), %zmm0
	vmovdqu64	704(%rsp), %zmm4
	vmovdqu64	896(%rsp), %zmm3
	movq	1152(%rsp), %rax
	movq	%rax, 448(%rsi)
	vmovups	%zmm2, 320(%rsi)
	vmovups	%zmm1, 256(%rsi)
	vmovdqu64	768(%rsp), %zmm1
	vmovdqu64	832(%rsp), %zmm2
	vmovdqu64	%zmm0, 384(%rsi)
	vmovdqu64	%zmm3, 192(%rsi)
	vmovdqu64	%zmm4, (%rsi)
	vmovdqu64	%zmm2, 128(%rsi)
	vmovdqu64	%zmm1, 64(%rsi)
.LBB1000_354:
.Ltmp16199:
	movq	96(%rsp), %rcx
	movq	2888(%rsp), %r8
	leaq	704(%rsp), %rdi
	movq	%rbx, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16200:
	vmovdqu64	704(%rsp), %zmm0
	vmovdqu64	768(%rsp), %zmm1
	vmovdqu64	832(%rsp), %zmm2
	vmovdqu64	856(%rsp), %zmm3
	movq	32(%rsp), %rcx
	imulq	$216, %r14, %rax
	incq	%r14
	vmovdqu64	%zmm3, 152(%rcx,%rax)
	vmovdqu64	%zmm2, 128(%rcx,%rax)
	vmovdqu64	%zmm1, 64(%rcx,%rax)
	vmovdqu64	%zmm0, (%rcx,%rax)
	cmpq	%r15, %r14
	jne	.LBB1000_327
.LBB1000_356:
	cmpq	$0, 48(%rsp)
	je	.LBB1000_387
	xorl	%r14d, %r14d
	.p2align	4
.LBB1000_358:
	movq	80(%rsp), %rcx
	leaq	(%r14,%r14,8), %rax
	leaq	8(%rcx,%rax,8), %rbx
	movq	2888(%rsp), %rcx
	movq	352(%rcx), %rax
	testq	%rax, %rax
	je	.LBB1000_374
	movq	192(%rax), %rcx
	xorl	%ebp, %ebp
	cmpq	160(%rsp), %rcx
	jne	.LBB1000_364
.LBB1000_360:
.Ltmp16211:
	movq	160(%rsp), %rdi
	movq	purrdf_sparql_eval::vm::attached_position@GOTPCREL(%rip), %rax
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16212:
	cmpq	$1, %rax
	jne	.LBB1000_374
	movq	2888(%rsp), %rax
	movq	352(%rax), %rdi
	addq	$16, %rdi
.Ltmp16213:
	movq	<purrdf_sparql_eval::plan::PlanShape>::program@GOTPCREL(%rip), %rax
	movl	%ebp, %esi
	movq	%rbx, %rcx
	callq	*%rax
.Ltmp16214:
	movq	%rax, %rsi
	jmp	.LBB1000_385
	.p2align	4
.LBB1000_364:
	movq	32(%rax), %rsi
	testq	%rsi, %rsi
	je	.LBB1000_374
	movq	24(%rax), %rax
	cmpq	$1, %rsi
	jne	.LBB1000_367
	xorl	%ecx, %ecx
	jmp	.LBB1000_370
.LBB1000_367:
	movq	%rsi, %rdx
	xorl	%edi, %edi
	.p2align	4
.LBB1000_368:
	movq	%rdx, %r8
	shrq	%r8
	leaq	(%r8,%rdi), %rcx
	leaq	(%rcx,%rcx,4), %r9
	movl	32(%rax,%r9,8), %ebp
	cmpq	%rbp, %rsi
	jbe	.LBB1000_978
	movq	160(%rsp), %r10
	leaq	(%rbp,%rbp,4), %r9
	cmpq	%r10, 16(%rax,%r9,8)
	cmovaq	%rdi, %rcx
	subq	%r8, %rdx
	movq	%rcx, %rdi
	cmpq	$1, %rdx
	ja	.LBB1000_368
.LBB1000_370:
	leaq	(%rcx,%rcx,4), %rdx
	movl	32(%rax,%rdx,8), %ebp
	cmpq	%rbp, %rsi
	jbe	.LBB1000_978
	movq	160(%rsp), %rdi
	leaq	(%rbp,%rbp,4), %rdx
	cmpq	%rdi, 16(%rax,%rdx,8)
	jne	.LBB1000_374
	cmpq	%rsi, %rcx
	jae	.LBB1000_977
	testl	%ebp, %ebp
	jne	.LBB1000_360
	.p2align	4
.LBB1000_374:
	vmovdqa	.LCPI1000_6(%rip), %ymm0
	movq	$1, 1976(%rsp)
	movq	$1, 2016(%rsp)
	movq	$0, 1792(%rsp)
	movq	$8, 1800(%rsp)
	movq	$0, 1808(%rsp)
	movq	$1, 2136(%rsp)
	vmovdqu	%ymm0, 1816(%rsp)
	movl	$0, 2224(%rsp)
.Ltmp16215:
	leaq	1792(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::vm::compile::ExprProgram>::recompile
.Ltmp16216:
	vmovdqu64	2168(%rsp), %zmm0
	vmovups	2112(%rsp), %zmm2
	vmovups	2048(%rsp), %zmm1
	leaq	720(%rsp), %rax
	vmovdqu64	1792(%rsp), %zmm4
	vmovdqu64	1984(%rsp), %zmm3
	movq	malloc@GOTPCREL(%rip), %rbp
	movl	$456, %edi
	vmovdqu64	%zmm0, 376(%rax)
	vmovups	%zmm2, 320(%rax)
	vmovups	%zmm1, 256(%rax)
	vmovdqu64	1856(%rsp), %zmm1
	vmovdqu64	1920(%rsp), %zmm2
	vmovdqu64	%zmm3, 192(%rax)
	vmovdqu64	%zmm4, (%rax)
	vmovdqu64	%zmm2, 128(%rax)
	vmovdqu64	%zmm1, 64(%rax)
	movq	$1, 704(%rsp)
	movq	$1, 712(%rsp)
	vzeroupper
	callq	*%rbp
	testq	%rax, %rax
	je	.LBB1000_969
	movq	%rax, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	movl	$456, %edx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rdx, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rdx, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1000_378
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_378:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_384
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_378
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	movabsq	$9223372036854775807, %rax
	lock		addq	$456, (%rcx)
	movl	$456, %ecx
	lock		xaddq	%rcx, (%rdi)
	addq	$456, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1000_381:
	cmpq	%rax, %rcx
	jle	.LBB1000_383
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1000_381
.LBB1000_383:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_384:
	vmovups	1024(%rsp), %zmm2
	vmovups	960(%rsp), %zmm1
	vmovdqu64	1088(%rsp), %zmm0
	vmovdqu64	704(%rsp), %zmm4
	vmovdqu64	896(%rsp), %zmm3
	movq	1152(%rsp), %rax
	movq	%rax, 448(%rsi)
	vmovups	%zmm2, 320(%rsi)
	vmovups	%zmm1, 256(%rsi)
	vmovdqu64	768(%rsp), %zmm1
	vmovdqu64	832(%rsp), %zmm2
	vmovdqu64	%zmm0, 384(%rsi)
	vmovdqu64	%zmm3, 192(%rsi)
	vmovdqu64	%zmm4, (%rsi)
	vmovdqu64	%zmm2, 128(%rsi)
	vmovdqu64	%zmm1, 64(%rsi)
.LBB1000_385:
.Ltmp16221:
	movq	96(%rsp), %rcx
	movq	2888(%rsp), %r8
	leaq	704(%rsp), %rdi
	movq	%rbx, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16222:
	vmovdqu64	704(%rsp), %zmm0
	vmovdqu64	768(%rsp), %zmm1
	vmovdqu64	832(%rsp), %zmm2
	vmovdqu64	856(%rsp), %zmm3
	movq	32(%rsp), %rcx
	imulq	$216, %r15, %rax
	incq	%r15
	incq	%r14
	vmovdqu64	%zmm3, 152(%rcx,%rax)
	vmovdqu64	%zmm2, 128(%rcx,%rax)
	vmovdqu64	%zmm1, 64(%rcx,%rax)
	vmovdqu64	%zmm0, (%rcx,%rax)
	cmpq	48(%rsp), %r14
	jne	.LBB1000_358
.LBB1000_387:
	vmovdqu	1472(%rsp), %xmm0
	movq	88(%rsp), %rcx
	leaq	(%r13,%r13,2), %rax
	incq	%r13
	incq	%r12
	movabsq	$42700796466920258, %rdx
	movq	%r15, 720(%rsp)
	movq	%r15, 16(%rcx,%rax,8)
	vmovdqa	%xmm0, 704(%rsp)
	vmovdqu	%xmm0, (%rcx,%rax,8)
	cmpq	2880(%rsp), %r13
	jne	.LBB1000_313
	movq	120(%rsp), %r12
	movq	88(%rsp), %rcx
	movl	152(%rsp), %r13d
	movq	2888(%rsp), %r15
	jmp	.LBB1000_390
.LBB1000_389:
	movq	120(%rsp), %r12
	movl	$8, %ecx
.LBB1000_390:
	movq	2880(%rsp), %rax
	movq	%rax, 584(%rsp)
	movq	%rcx, 592(%rsp)
	movq	%rax, 600(%rsp)
	testb	%r13b, %r13b
	je	.LBB1000_397
	movq	616(%r15), %rsi
	jmp	.LBB1000_392
.LBB1000_397:
	movq	544(%rsp), %rbx
	testq	%rbx, %rbx
	je	.LBB1000_464
	leaq	(,%rbx,8), %rax
	movl	$8, %esi
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB1000_986
	movq	536(%rsp), %rbp
	leaq	(%rbx,%rbx,8), %rcx
	addq	$16, %r12
	movq	%rbx, 368(%rsp)
	movq	%rax, 376(%rsp)
	movq	$0, 384(%rsp)
	movq	%rax, %rbx
	movq	%r12, 120(%rsp)
	xorl	%r12d, %r12d
	leaq	(%rbp,%rcx,8), %rcx
	movq	%rcx, 48(%rsp)
	jmp	.LBB1000_401
.LBB1000_400:
	leaq	(%r12,%r12,4), %rax
	addq	$72, %rbp
	incq	%r12
	movq	%r14, (%rbx,%rax,8)
	movq	%r13, 8(%rbx,%rax,8)
	vmovdqa	704(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rbx,%rax,8)
	movq	720(%rsp), %rcx
	movq	%rcx, 32(%rbx,%rax,8)
	movq	%r12, 384(%rsp)
	cmpq	48(%rsp), %rbp
	je	.LBB1000_465
.LBB1000_401:
	movq	1216(%rsp), %r13
	movq	$1, 704(%rsp)
	cmpq	$5, %r13
	jae	.LBB1000_427
	vmovdqu	712(%rsp), %xmm0
	movq	736(%rsp), %rax
	movq	704(%rsp), %rdx
	movq	728(%rsp), %rcx
	movq	%rax, 1824(%rsp)
	movq	%rdx, 1792(%rsp)
	movq	%rcx, 1816(%rsp)
	vmovdqu	%xmm0, 1800(%rsp)
	testq	%r13, %r13
	je	.LBB1000_404
.LBB1000_403:
	movl	$2, %eax
	jmp	.LBB1000_405
.LBB1000_404:
	movl	$-1, %eax
.LBB1000_405:
	movl	%eax, 704(%rsp)
	movq	%r13, 712(%rsp)
.Ltmp16247:
	leaq	1792(%rsp), %rdi
	leaq	704(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp16248:
	vmovdqu	1792(%rsp), %ymm0
	movq	1824(%rsp), %rax
	leaq	1480(%rsp), %rdi
	movq	%rax, 1504(%rsp)
	vmovdqu	%ymm0, 1472(%rsp)
	movq	1472(%rsp), %r14
	movq	%r14, %rax
	cmpq	$6, %r14
	jb	.LBB1000_408
	movq	1480(%rsp), %rdi
	movq	1488(%rsp), %rax
.LBB1000_408:
	movq	360(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB1000_958
	movq	(%rbp), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB1000_411
	movq	16(%rbp), %rsi
	movq	8(%rbp), %rax
	decq	%rsi
	jmp	.LBB1000_412
.LBB1000_411:
	leaq	8(%rbp), %rax
.LBB1000_412:
	movq	%rbx, 96(%rsp)
	cmpq	%rsi, %rdx
	jne	.LBB1000_959
	movq	memcpy@GOTPCREL(%rip), %rbx
	shlq	$3, %rdx
	movq	%rax, %rsi
	vzeroupper
	callq	*%rbx
	movq	600(%rsp), %r15
	movq	2880(%rsp), %rax
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	testq	%r15, %r15
	je	.LBB1000_424
	movq	%r12, 32(%rsp)
	movq	592(%rsp), %r12
	movq	120(%rsp), %r13
	xorl	%ebx, %ebx
	addq	$16, %r12
	jmp	.LBB1000_416
	.p2align	4
.LBB1000_415:
	incq	%rbx
	addq	$24, %r12
	addq	$120, %r13
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%rbx, %r15
	je	.LBB1000_423
.LBB1000_416:
	vmovups	1176(%rsp), %xmm0
	movq	232(%rsp), %rax
	movq	-8(%r12), %rdx
	movq	(%r12), %rcx
	movq	56(%rbp), %r8
	movq	64(%rbp), %r9
	addq	$16, %rax
.Ltmp16252:
	movq	2888(%rsp), %rsi
	leaq	704(%rsp), %rdi
	movq	%rax, 16(%rsp)
	movq	%rsi, 24(%rsp)
	movq	%r13, %rsi
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16253:
	vmovq	712(%rsp), %xmm0
	movq	704(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB1000_429
	movq	1472(%rsp), %r14
	movq	%r14, %rsi
	cmpq	$6, %r14
	jb	.LBB1000_420
	movq	1488(%rsp), %rsi
.LBB1000_420:
	movq	360(%rsp), %rdi
	decq	%rsi
	addq	%rbx, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1000_973
	leaq	1480(%rsp), %rax
	cmpq	$6, %r14
	jb	.LBB1000_415
	movq	1480(%rsp), %rax
	jmp	.LBB1000_415
.LBB1000_423:
	movq	1472(%rsp), %r14
	movq	32(%rsp), %r12
.LBB1000_424:
	leaq	1480(%rsp), %rax
	movq	1480(%rsp), %r13
	movq	2888(%rsp), %r15
	movq	96(%rsp), %rbx
	vmovups	8(%rax), %xmm0
	movq	24(%rax), %rax
	movq	%rax, 720(%rsp)
	vmovaps	%xmm0, 704(%rsp)
	cmpq	368(%rsp), %r12
	jne	.LBB1000_400
.Ltmp16257:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdi
	callq	*%rax
.Ltmp16258:
	movq	376(%rsp), %rbx
	jmp	.LBB1000_400
.LBB1000_427:
.Ltmp16244:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	704(%rsp), %rdi
	xorl	%esi, %esi
	movq	%r13, %rdx
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp16245:
	vmovdqu	704(%rsp), %ymm0
	movq	736(%rsp), %rax
	movq	%rax, 1824(%rsp)
	vmovdqu	%ymm0, 1792(%rsp)
	jmp	.LBB1000_403
.LBB1000_429:
	vmovdqu64	720(%rsp), %zmm1
	vmovdqu64	736(%rsp), %zmm2
	movq	168(%rsp), %rcx
	vmovdqu64	%zmm2, 48(%rcx)
	vmovdqu64	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1472(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB1000_431
	movq	1480(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB1000_431:
	movq	32(%rsp), %r12
	movq	96(%rsp), %rbx
	testq	%r12, %r12
	je	.LBB1000_444
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	free@GOTPCREL(%rip), %rbp
	xorl	%r14d, %r14d
	jmp	.LBB1000_436
.LBB1000_433:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_434:
	vzeroupper
	callq	*%rbp
.LBB1000_435:
	incq	%r14
	cmpq	%r12, %r14
	je	.LBB1000_444
.LBB1000_436:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB1000_435
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_439
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_439:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_434
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_439
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
.LBB1000_442:
	cmpq	%rax, %rdx
	jge	.LBB1000_433
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB1000_442
	jmp	.LBB1000_433
.LBB1000_444:
	movq	368(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_530
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,4), %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
	jmp	.LBB1000_530
.LBB1000_299:
	movq	120(%rsp), %r12
	movq	$0, 584(%rsp)
	movq	$8, 592(%rsp)
	movq	$0, 600(%rsp)
.LBB1000_392:
	movq	1040(%r15), %rax
	addq	904(%r15), %rax
	movq	%rax, 1744(%rsp)
.Ltmp16265:
	leaq	1472(%rsp), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::for_items::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16266:
	movq	544(%rsp), %rbx
.Ltmp16267:
	movq	%r15, %rdi
	movq	%rbx, %rsi
	callq	<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
.Ltmp16268:
	movq	616(%r15), %rcx
	movq	%rax, 336(%rsp)
	testq	%rcx, %rcx
	je	.LBB1000_446
	cmpq	$-2, 16(%rcx)
	movb	$1, %al
	jb	.LBB1000_447
	cmpq	$-2, 40(%rcx)
	setb	%al
	jmp	.LBB1000_447
.LBB1000_446:
	xorl	%eax, %eax
.LBB1000_447:
	leaq	1216(%rsp), %rdi
	movzbl	1234(%r15), %ecx
	movq	536(%rsp), %rdx
	leaq	584(%rsp), %rsi
	leaq	336(%rsp), %r8
	movq	%rdi, 368(%rsp)
	movq	2880(%rsp), %rdi
	movq	%rsi, 176(%rsp)
	movq	%r15, 184(%rsp)
	movq	%r8, 192(%rsp)
	leaq	360(%rsp), %r8
	leaq	1744(%rsp), %rsi
	movq	%r14, 200(%rsp)
	movq	%r8, 376(%rsp)
	movq	%r12, 384(%rsp)
	leaq	1168(%rsp), %r8
	movq	%rdi, 392(%rsp)
	leaq	232(%rsp), %rdi
	movq	%r8, 400(%rsp)
	movq	%rdi, 408(%rsp)
	movq	%rsi, 416(%rsp)
	movzbl	%cl, %esi
	testb	%al, %al
	je	.LBB1000_449
.Ltmp16271:
	leaq	704(%rsp), %rdi
	leaq	176(%rsp), %r8
	leaq	368(%rsp), %r9
	movq	%rbx, %rcx
	movq	%r14, (%rsp)
	callq	purrdf_sparql_eval::parallel::par_blocks_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#5}, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#6}, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#7}>
.Ltmp16272:
	jmp	.LBB1000_450
.LBB1000_449:
.Ltmp16269:
	leaq	704(%rsp), %rdi
	leaq	176(%rsp), %r8
	leaq	368(%rsp), %r9
	movq	%rbx, %rcx
	movq	%r14, (%rsp)
	callq	purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#5}, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#6}, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#7}>
.Ltmp16270:
.LBB1000_450:
	movq	704(%rsp), %rcx
	cmpq	$-1, %rcx
	je	.LBB1000_463
	vmovups	864(%rsp), %zmm0
	vmovups	816(%rsp), %zmm2
	movq	728(%rsp), %rax
	vmovdqu	712(%rsp), %xmm1
	movq	744(%rsp), %rsi
	movq	736(%rsp), %rdx
	movq	720(%rsp), %rbx
	movq	%rcx, 1320(%rsp)
	movl	$1, %edi
	leaq	-3(%rax), %rcx
	cmpq	$-2, %rcx
	movl	$1, %ecx
	cmovbq	%rax, %rdi
	cmovbq	%rsi, %rax
	cmovaeq	%rsi, %rcx
	vmovups	%zmm0, 1904(%rsp)
	vmovups	%zmm2, 1856(%rsp)
	vmovdqu64	752(%rsp), %zmm0
	decq	%rax
	vmovdqu	%xmm1, 1328(%rsp)
	vmovdqu64	1904(%rsp), %zmm3
	vmovdqu64	1856(%rsp), %zmm2
	vmovdqu64	%zmm0, 1792(%rsp)
	vmovdqu64	%zmm0, 728(%rsp)
	vmovdqu64	%zmm3, 840(%rsp)
	vmovdqu64	%zmm2, 792(%rsp)
	movq	%rdi, 704(%rsp)
	movq	%rdx, 712(%rsp)
	movq	%rcx, 720(%rsp)
	movq	$0, 904(%rsp)
	movq	%rax, 912(%rsp)
.Ltmp16274:
	leaq	1792(%rsp), %rdi
	leaq	704(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp16275:
	vmovups	1824(%rsp), %zmm1
	vmovups	1792(%rsp), %ymm0
	vmovdqu64	1888(%rsp), %zmm2
	cmpq	$0, 616(%r15)
	vmovups	%zmm1, 2528(%rsp)
	vmovdqu64	1936(%rsp), %zmm1
	vmovdqu64	%zmm2, 2592(%rsp)
	vmovups	%ymm0, 2448(%rsp)
	vmovdqu64	%zmm1, 2640(%rsp)
	je	.LBB1000_466
	vmovdqu	1320(%rsp), %xmm0
	vmovdqu64	2528(%rsp), %zmm3
	vmovdqu64	2640(%rsp), %zmm2
	vmovdqu64	2592(%rsp), %zmm1
	leaq	888(%r15), %rax
	movzbl	1632(%rsp), %r14d
	movq	%rax, 96(%rsp)
	movq	1336(%rsp), %rax
	movq	%rax, 1392(%rsp)
	vmovdqu64	%zmm2, 1904(%rsp)
	vmovdqa	%xmm0, 1376(%rsp)
	vmovdqu64	%zmm1, 1856(%rsp)
	vmovdqu64	%zmm3, 1792(%rsp)
	testb	%r14b, %r14b
	je	.LBB1000_468
	movq	1792(%rsp), %rax
	vmovdqu64	2552(%rsp), %zmm0
	vmovdqu64	2640(%rsp), %zmm2
	vmovdqu64	2616(%rsp), %zmm1
	movq	1808(%rsp), %rcx
	movb	%r14b, 72(%rsp)
	movq	1800(%rsp), %r14
	movl	$1, %edx
	movl	$1, %esi
	movq	$0, 176(%rsp)
	movq	$8, 184(%rsp)
	movq	$0, 192(%rsp)
	cmpq	$3, %rax
	movq	%rax, %r15
	cmovaeq	%rcx, %r15
	cmovaeq	%rdx, %rcx
	cmovaeq	%rax, %rsi
	leaq	712(%rsp), %rdx
	decq	%r15
	vmovdqu64	%zmm2, 816(%rsp)
	vmovdqu64	%zmm1, 792(%rsp)
	vmovdqu64	%zmm0, 728(%rsp)
	movq	%rsi, 704(%rsp)
	movq	%r14, 712(%rsp)
	movq	%rcx, 720(%rsp)
	movq	$0, 880(%rsp)
	movq	%r15, 888(%rsp)
	je	.LBB1000_477
	cmpq	$3, %rax
	leaq	376(%rsp), %r12
	movl	$8, %ecx
	cmovbq	%rdx, %r14
	xorl	%ebx, %ebx
	xorl	%ebp, %ebp
	addq	$8, %r14
.LBB1000_456:
	leaq	1(%rbx), %r13
	movq	%r13, 880(%rsp)
	movq	-8(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB1000_488
	vmovups	(%r14), %zmm0
	vmovups	64(%r14), %zmm1
	vmovups	88(%r14), %zmm2
	vmovups	%zmm2, 88(%r12)
	vmovups	%zmm1, 64(%r12)
	vmovups	%zmm0, (%r12)
	movq	%rax, 368(%rsp)
	movq	%rbx, %rax
	movzbl	520(%rsp), %ebx
	cmpq	176(%rsp), %rax
	jne	.LBB1000_460
.Ltmp16304:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16305:
	movq	184(%rsp), %rcx
.LBB1000_460:
	vmovdqu64	368(%rsp), %zmm0
	vmovdqu64	432(%rsp), %zmm1
	vmovdqu64	464(%rsp), %zmm2
	vmovdqu64	%zmm2, 96(%rcx,%rbp)
	vmovdqu64	%zmm1, 64(%rcx,%rbp)
	vmovdqu64	%zmm0, (%rcx,%rbp)
	movq	%r13, 192(%rsp)
	testb	%bl, %bl
	jne	.LBB1000_489
	addq	$160, %rbp
	addq	$168, %r14
	movq	%r13, %rbx
	cmpq	%r13, %r15
	jne	.LBB1000_456
	xorl	%ebp, %ebp
	movq	%r15, %rbx
	jmp	.LBB1000_490
.LBB1000_463:
	vmovdqu64	752(%rsp), %zmm0
	vmovdqu	720(%rsp), %ymm1
	movq	168(%rsp), %rax
	vmovdqu64	%zmm0, 48(%rax)
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu64	%zmm0, 1792(%rsp)
	movq	$1, (%rax)
	movq	336(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1000_527
	jmp	.LBB1000_529
.LBB1000_464:
	movq	$0, 368(%rsp)
	movq	$8, 376(%rsp)
	movq	$0, 384(%rsp)
.LBB1000_465:
	movq	384(%rsp), %rax
	movq	368(%rsp), %rdx
	movq	376(%rsp), %rcx
	movq	%rax, 1360(%rsp)
	movq	%rdx, 1344(%rsp)
	movq	%rcx, 1352(%rsp)
	jmp	.LBB1000_863
.LBB1000_466:
	leaq	1176(%rsp), %rcx
	movq	1168(%rsp), %rax
	vmovups	(%rcx), %xmm0
	movq	$0, 1168(%rsp)
	movq	$8, 1176(%rsp)
	movq	$0, 1184(%rsp)
	vmovaps	%xmm0, 1792(%rsp)
	cmpq	%rbx, %rax
	jbe	.LBB1000_472
	vmovdqa	1792(%rsp), %xmm0
	leaq	704(%rsp), %rdi
	movq	%rax, 704(%rsp)
	vmovdqu	%xmm0, 712(%rsp)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	$8, 2352(%rsp)
	movq	$0, 2360(%rsp)
	xorl	%eax, %eax
	jmp	.LBB1000_473
.LBB1000_468:
	movq	1392(%rsp), %rbx
	movq	$0, 608(%rsp)
	movq	$8, 616(%rsp)
	movq	$0, 624(%rsp)
.Ltmp16279:
	leaq	704(%rsp), %rdi
	leaq	608(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp16280:
	movq	704(%rsp), %r13
	movq	712(%rsp), %rax
	movq	720(%rsp), %rdx
	movq	728(%rsp), %rbp
	cmpq	$-1, %r13
	je	.LBB1000_478
	vmovdqu	752(%rsp), %ymm0
	vmovdqu	768(%rsp), %ymm1
	movq	%rax, 48(%rsp)
	movq	744(%rsp), %rax
	movq	736(%rsp), %rbx
	movq	%rdx, 96(%rsp)
	movq	%rax, 576(%rsp)
	vmovdqu	%ymm0, 368(%rsp)
	vmovdqu	%ymm1, 384(%rsp)
.Ltmp16284:
	leaq	1320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16285:
.LBB1000_471:
	vmovups	368(%rsp), %ymm0
	vmovups	384(%rsp), %ymm1
	movq	%rbx, 72(%rsp)
	shrq	$8, %rbx
	movq	%rbx, 80(%rsp)
	vmovups	%ymm0, 2480(%rsp)
	vmovups	%ymm1, 2496(%rsp)
	jmp	.LBB1000_501
.LBB1000_472:
	vmovdqa	1792(%rsp), %xmm0
	vmovdqu	%xmm0, 2352(%rsp)
.LBB1000_473:
	movl	1632(%rsp), %esi
	movq	%rax, 2344(%rsp)
.Ltmp16443:
	leaq	704(%rsp), %rdi
	leaq	1320(%rsp), %rcx
	leaq	2528(%rsp), %r8
	leaq	2344(%rsp), %r9
	movq	%r15, %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#8}>
.Ltmp16444:
	movq	704(%rsp), %rax
	movq	712(%rsp), %rsi
	movq	720(%rsp), %rdx
	movq	728(%rsp), %rbp
	movq	744(%rsp), %r14
	movq	736(%rsp), %rbx
	cmpq	$-1, %rax
	je	.LBB1000_476
	vmovdqu	752(%rsp), %ymm0
	vmovdqu	768(%rsp), %ymm1
	movq	168(%rsp), %rcx
	vmovdqu	%ymm1, 80(%rcx)
	vmovdqu	%ymm0, 64(%rcx)
	movq	%rax, 16(%rcx)
	movq	%rsi, 24(%rcx)
	movq	%rdx, 32(%rcx)
	movq	%rbp, 40(%rcx)
	movq	%rbx, 48(%rcx)
	movq	%r14, 56(%rcx)
	jmp	.LBB1000_525
.LBB1000_476:
	movq	%rsi, 48(%rsp)
	movq	%rdx, 96(%rsp)
	jmp	.LBB1000_855
.LBB1000_477:
	xorl	%ebx, %ebx
	movl	$8, %r14d
	xorl	%ebp, %ebp
	jmp	.LBB1000_491
.LBB1000_478:
	movq	%rax, 1264(%rsp)
	movq	%rdx, 1272(%rsp)
	movq	%rbp, 1280(%rsp)
	movq	%rbp, %r15
	movq	1384(%rsp), %rbp
	movq	1376(%rsp), %rax
	leaq	(,%rbx,8), %rcx
	movb	%r14b, 72(%rsp)
	leaq	(%rcx,%rcx,4), %r14
	leaq	(%rbp,%r14), %rcx
	movq	%rbp, 176(%rsp)
	movq	%rax, 192(%rsp)
	movq	%rcx, 200(%rsp)
	testq	%rbx, %rbx
	je	.LBB1000_499
	leaq	(,%r15,8), %rax
	addq	$40, %rbp
	movq	%rcx, 80(%rsp)
	leaq	(%rax,%rax,4), %r12
	jmp	.LBB1000_481
.LBB1000_480:
	vmovdqa	32(%rsp), %xmm0
	movq	48(%rsp), %rax
	movq	%rbx, (%rdx,%r12)
	incq	%r15
	addq	$40, %rbp
	movq	%rax, 8(%rdx,%r12)
	vmovdqu	%xmm0, 16(%rdx,%r12)
	movq	%r13, 32(%rdx,%r12)
	addq	$40, %r12
	addq	$-40, %r14
	movq	%r15, 1280(%rsp)
	je	.LBB1000_498
.LBB1000_481:
	movq	2888(%rsp), %rcx
	leaq	376(%rsp), %rsi
	movq	%rcx, 368(%rsp)
	movq	-8(%rbp), %rax
	movq	%rax, 32(%rsi)
	vmovdqu	-40(%rbp), %ymm0
	vmovdqu	%ymm0, (%rsi)
	cmpq	$0, 376(%rsp)
	je	.LBB1000_483
	leaq	-40(%rbp), %rax
	leaq	712(%rsp), %rsi
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rsi)
	vmovdqu	(%rax), %ymm0
	vmovdqu	%ymm0, (%rsi)
	jmp	.LBB1000_485
.LBB1000_483:
	movq	%rdx, %r13
	movq	664(%rcx), %rdx
.Ltmp16287:
	movq	96(%rsp), %rsi
	leaq	704(%rsp), %rdi
	leaq	384(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16288:
	movq	704(%rsp), %rax
	movq	%r13, %rdx
	cmpq	$-1, %rax
	jne	.LBB1000_632
.LBB1000_485:
	vmovdqu	728(%rsp), %xmm0
	movq	720(%rsp), %rax
	movq	712(%rsp), %rbx
	movq	744(%rsp), %r13
	movq	%rax, 48(%rsp)
	vmovdqa	%xmm0, 32(%rsp)
	cmpq	1264(%rsp), %r15
	jne	.LBB1000_480
.Ltmp16292:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	1264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16293:
	movq	1272(%rsp), %rdx
	jmp	.LBB1000_480
.LBB1000_488:
	xorl	%ebp, %ebp
	jmp	.LBB1000_490
.LBB1000_489:
	movb	$1, %bpl
	movq	%r13, %rbx
.LBB1000_490:
	movq	120(%rsp), %r12
	movq	%rcx, %r14
.LBB1000_491:
.Ltmp16312:
	leaq	704(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp16313:
	movq	2888(%rsp), %r15
	movq	176(%rsp), %rax
	movq	%rax, 568(%rsp)
	testq	%rbx, %rbx
	je	.LBB1000_495
	cmpq	$8, %rbx
	jae	.LBB1000_496
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB1000_510
.LBB1000_495:
	xorl	%edx, %edx
	jmp	.LBB1000_512
.LBB1000_496:
	cmpq	$32, %rbx
	jae	.LBB1000_503
	xorl	%eax, %eax
	xorl	%edx, %edx
	jmp	.LBB1000_507
.LBB1000_498:
	movq	80(%rsp), %rbp
	movq	120(%rsp), %r12
.LBB1000_499:
	movq	%rbp, 184(%rsp)
.Ltmp16298:
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16299:
	movq	1264(%rsp), %rax
	movq	1272(%rsp), %rcx
	movq	%r15, %rbp
	movq	2888(%rsp), %r15
	movq	$-1, %r13
	movq	$0, 80(%rsp)
	movq	$0, 72(%rsp)
	movq	%rax, 48(%rsp)
	movq	%rcx, 96(%rsp)
.LBB1000_501:
.Ltmp16301:
	leaq	1792(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp16302:
	cmpq	$-1, %r13
	jne	.LBB1000_524
	jmp	.LBB1000_854
.LBB1000_503:
	vmovdqa64	.LCPI1000_7(%rip), %zmm1
	vpbroadcastq	.LCPI1000_8(%rip), %zmm2
	vpbroadcastq	.LCPI1000_9(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB1000_504:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	64(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1344(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2624(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3904(%r14,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB1000_504
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %rbx
	je	.LBB1000_512
	testb	$24, %bl
	je	.LBB1000_510
.LBB1000_507:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI1000_7(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI1000_8(%rip), %zmm2
	vpbroadcastq	.LCPI1000_10(%rip), %zmm3
	movq	%rbx, %rax
	andq	$-8, %rax
	vmovq	%rdx, %xmm0
	subq	%rax, %rcx
.LBB1000_508:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r14,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB1000_508
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rdx
	cmpq	%rax, %rbx
	je	.LBB1000_512
.LBB1000_510:
	movq	%rbx, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r14), %rax
.LBB1000_511:
	addq	(%rax), %rdx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB1000_511
.LBB1000_512:
	movq	568(%rsp), %rax
	movq	%rbx, 296(%rsp)
	movq	$0, 368(%rsp)
	movq	$8, 376(%rsp)
	movq	%rdx, 576(%rsp)
	movq	$0, 384(%rsp)
	movq	%rax, 2416(%rsp)
	movq	%r14, 2424(%rsp)
	movq	%rbx, 2432(%rsp)
	movq	1392(%rsp), %rbx
	movb	%bpl, 2440(%rsp)
.Ltmp16323:
	leaq	704(%rsp), %rdi
	leaq	368(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp16324:
	movq	704(%rsp), %r13
	movq	712(%rsp), %rax
	movq	720(%rsp), %rdi
	movq	728(%rsp), %rsi
	movl	%ebp, 292(%rsp)
	movq	%r14, 1208(%rsp)
	cmpq	$-1, %r13
	je	.LBB1000_616
	movq	%rax, 48(%rsp)
	movzbl	736(%rsp), %eax
	vmovdqu	752(%rsp), %ymm0
	vmovdqu	768(%rsp), %ymm1
	movzbl	743(%rsp), %ebp
	movzwl	741(%rsp), %ebx
	movl	737(%rsp), %r14d
	movq	%rdi, 96(%rsp)
	movq	%rsi, 32(%rsp)
	movq	%rax, 72(%rsp)
	movq	744(%rsp), %rax
	vmovdqu	%ymm0, 1264(%rsp)
	vmovdqu	%ymm1, 1280(%rsp)
	movq	%rax, 88(%rsp)
.Ltmp16328:
	leaq	1320(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16329:
	shll	$16, %ebp
	orl	%ebp, %ebx
	movl	292(%rsp), %ebp
	shlq	$32, %rbx
	orq	%rbx, %r14
	movq	%r14, 80(%rsp)
.LBB1000_516:
	movq	296(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_520
	movq	1208(%rsp), %rbx
	movl	$1, %r14d
	subq	%rax, %r14
.LBB1000_518:
.Ltmp16434:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16435:
	incq	%r14
	addq	$160, %rbx
	cmpq	$1, %r14
	jne	.LBB1000_518
.LBB1000_520:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_522
	movq	1208(%rsp), %rdi
	shlq	$5, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_522:
	cmpq	$-1, %r13
	je	.LBB1000_621
	vmovdqu	1280(%rsp), %ymm1
	vmovdqu	1264(%rsp), %ymm0
	movq	88(%rsp), %rax
	movq	32(%rsp), %rbp
	movq	%rax, 576(%rsp)
	vmovdqu	%ymm1, 2496(%rsp)
	vmovdqu	%ymm0, 2480(%rsp)
.LBB1000_524:
	movq	80(%rsp), %rcx
	movzbl	72(%rsp), %eax
	vmovdqu	2480(%rsp), %ymm0
	vmovdqu	2496(%rsp), %ymm1
	movq	48(%rsp), %rdx
	movq	96(%rsp), %rsi
	shlq	$8, %rcx
	orq	%rcx, %rax
	movq	168(%rsp), %rcx
	vmovdqu	%ymm1, 80(%rcx)
	vmovdqu	%ymm0, 64(%rcx)
	movq	%r13, 16(%rcx)
	movq	%rdx, 24(%rcx)
	movq	576(%rsp), %rdx
	movq	%rsi, 32(%rcx)
	movq	%rbp, 40(%rcx)
	movq	%rax, 48(%rcx)
	movq	%rdx, 56(%rcx)
.LBB1000_525:
	movq	$1, (%rcx)
.Ltmp16448:
	leaq	2448(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16449:
	movq	336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_529
.LBB1000_527:
	lock		decq	(%rax)
	jne	.LBB1000_529
	#MEMBARRIER
.Ltmp16491:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	336(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16492:
.LBB1000_529:
.Ltmp16496:
	leaq	1472(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16497:
.LBB1000_530:
	movb	$1, %bpl
	movq	592(%rsp), %rbx
	movq	600(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_534
.LBB1000_531:
	movl	$1, %r15d
	movq	%rbx, %r14
	subq	%rax, %r15
	.p2align	4
.LBB1000_532:
.Ltmp16501:
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16502:
	incq	%r15
	addq	$24, %r14
	cmpq	$1, %r15
	jne	.LBB1000_532
.LBB1000_534:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_544
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_537
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_537:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_543
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_537
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
.LBB1000_540:
	cmpq	%rax, %rdx
	jge	.LBB1000_542
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_540
.LBB1000_542:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_543:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB1000_544:
	movq	536(%rsp), %rbx
	movq	544(%rsp), %r14
	movl	%ebp, 96(%rsp)
	testq	%r14, %r14
	je	.LBB1000_567
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r15
	xorl	%r12d, %r12d
	jmp	.LBB1000_549
	.p2align	4
.LBB1000_546:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_547:
	vzeroupper
	callq	*%r15
.LBB1000_548:
	incq	%r12
	cmpq	%r14, %r12
	je	.LBB1000_567
.LBB1000_549:
	leaq	(%r12,%r12,8), %rax
	leaq	(%rbx,%rax,8), %r13
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB1000_559
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_552
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_552:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_558
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_552
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
.LBB1000_555:
	cmpq	%rax, %rdx
	jge	.LBB1000_557
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB1000_555
.LBB1000_557:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_558:
	vzeroupper
	callq	*%r15
.LBB1000_559:
	movq	48(%r13), %rcx
	testq	%rcx, %rcx
	je	.LBB1000_548
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_562
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_562:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_547
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_562
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
.LBB1000_565:
	cmpq	%rax, %rdx
	jge	.LBB1000_546
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB1000_565
	jmp	.LBB1000_546
.LBB1000_567:
	movq	528(%rsp), %rax
	movl	96(%rsp), %r12d
	movb	$1, %bpl
	testq	%rax, %rax
	je	.LBB1000_577
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_570
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_570:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_576
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_570
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
.LBB1000_573:
	cmpq	%rax, %rdx
	jge	.LBB1000_575
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_573
.LBB1000_575:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_576:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB1000_577:
	cmpq	$0, 352(%rsp)
	je	.LBB1000_587
	movq	344(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_580
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_580:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_586
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_580
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
.LBB1000_583:
	cmpq	%rax, %rcx
	jge	.LBB1000_585
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1000_583
.LBB1000_585:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_586:
	movq	free@GOTPCREL(%rip), %rax
	movq	248(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB1000_587:
	movq	232(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1000_589
	movb	$1, %bl
	#MEMBARRIER
.Ltmp16507:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	232(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16508:
.LBB1000_589:
.Ltmp16510:
	leaq	1168(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16511:
	movq	2312(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1000_600
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	2320(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_593
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_593:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_599
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_593
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
.LBB1000_596:
	cmpq	%rax, %rdx
	jge	.LBB1000_598
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_596
.LBB1000_598:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_599:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_600:
	movq	2240(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB1000_610
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	2248(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_603
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_603:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_609
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_603
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
.LBB1000_606:
	cmpq	%rax, %rdx
	jge	.LBB1000_608
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_606
.LBB1000_608:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_609:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_610:
	movq	2336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_613
	lock		decq	(%rax)
	jne	.LBB1000_613
	leaq	2336(%rsp), %rdi
	#MEMBARRIER
.Ltmp16513:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16514:
.LBB1000_613:
	testb	%r12b, %r12b
	je	.LBB1000_871
.LBB1000_614:
	movq	256(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1000_871
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1424(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB1000_871:
	movq	168(%rsp), %rax
.LBB1000_872:
	addq	$2824, %rsp
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
.LBB1000_616:
	.cfi_def_cfa_offset 2880
	movq	%rax, 264(%rsp)
	movq	1384(%rsp), %rdx
	movq	1376(%rsp), %rax
	leaq	(%rbx,%rbx,4), %rcx
	movq	%rdi, 272(%rsp)
	movq	%rsi, 280(%rsp)
	movq	%rdx, 1224(%rsp)
	movq	%rax, 1240(%rsp)
	movq	616(%r15), %rax
	leaq	(%rdx,%rcx,8), %rcx
	movq	%rdx, 1232(%rsp)
	movq	%rdx, 80(%rsp)
	movq	%rcx, 1248(%rsp)
	movq	%rax, 1408(%rsp)
	testq	%rax, %rax
	je	.LBB1000_622
	lock		incq	(%rax)
	movq	568(%rsp), %r13
	movq	296(%rsp), %rax
	jle	.LBB1000_987
	movq	616(%r15), %rcx
	testq	%rax, %rax
	sete	%al
	movq	%rcx, 1256(%rsp)
	movq	%rcx, 64(%rsp)
	movq	16(%rcx), %rdx
	movq	40(%rcx), %rcx
	cmpq	$-1, %rcx
	movq	%rcx, 304(%rsp)
	movq	%rdx, 560(%rsp)
	sete	%cl
	orb	%al, %cl
	jne	.LBB1000_657
	movq	296(%rsp), %rax
	cmpq	$8, %rax
	jae	.LBB1000_641
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB1000_653
.LBB1000_621:
	movq	72(%rsp), %rdx
	jmp	.LBB1000_843
.LBB1000_622:
	movq	1232(%rsp), %rcx
	movq	1224(%rsp), %rax
	movq	1240(%rsp), %rdx
	movq	%rcx, 184(%rsp)
	movq	1248(%rsp), %rcx
	movq	%rax, 176(%rsp)
	movq	%rdx, 192(%rsp)
	movq	%rcx, 200(%rsp)
	movq	200(%rsp), %rax
	movq	184(%rsp), %r15
	movq	%rax, 64(%rsp)
	cmpq	%rax, %r15
	je	.LBB1000_643
	leaq	(,%rsi,8), %rax
	movq	%rdi, %r14
	movq	%rsi, %r12
	leaq	(%rax,%rax,4), %rbx
	jmp	.LBB1000_626
.LBB1000_624:
	movq	272(%rsp), %rcx
.LBB1000_625:
	movq	48(%rsp), %rax
	shll	$16, %r14d
	movq	80(%rsp), %rdx
	addq	$40, %r15
	orl	%r14d, %ebp
	movq	%rcx, %r14
	shlq	$32, %rbp
	orq	%rbp, %r12
	movq	%rax, (%rcx,%rbx)
	movq	72(%rsp), %rax
	movq	%rax, 8(%rcx,%rbx)
	movzbl	88(%rsp), %eax
	movq	%r13, 16(%rcx,%rbx)
	movb	%al, 24(%rcx,%rbx)
	movq	%r12, %rax
	movl	%r12d, 25(%rcx,%rbx)
	shrq	$32, %r12
	shrq	$48, %rax
	movw	%r12w, 29(%rcx,%rbx)
	movq	32(%rsp), %r12
	movb	%al, 31(%rcx,%rbx)
	movq	%rdx, 32(%rcx,%rbx)
	addq	$40, %rbx
	incq	%r12
	movq	%r12, 280(%rsp)
	cmpq	64(%rsp), %r15
	je	.LBB1000_644
.LBB1000_626:
	movq	32(%r15), %rax
	leaq	376(%rsp), %rcx
	movq	%rax, 32(%rcx)
	movq	2888(%rsp), %rax
	vmovdqu	(%r15), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%rax, 368(%rsp)
	cmpq	$0, 376(%rsp)
	je	.LBB1000_628
	movq	32(%r15), %rax
	leaq	712(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%r15), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB1000_630
.LBB1000_628:
	movq	664(%rax), %rdx
.Ltmp16413:
	movq	96(%rsp), %rsi
	leaq	704(%rsp), %rdi
	leaq	384(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16414:
	movq	704(%rsp), %r13
	cmpq	$-1, %r13
	jne	.LBB1000_905
.LBB1000_630:
	movq	720(%rsp), %rdx
	movq	%r12, %rsi
	movq	%r14, %rcx
	movq	712(%rsp), %rax
	movzbl	736(%rsp), %edi
	movq	728(%rsp), %r13
	movzbl	743(%rsp), %r14d
	movzwl	741(%rsp), %ebp
	movl	737(%rsp), %r12d
	movq	%rsi, 32(%rsp)
	movq	%rdx, 72(%rsp)
	movq	744(%rsp), %rdx
	movq	%rax, 48(%rsp)
	movb	%dil, 88(%rsp)
	movq	%rdx, 80(%rsp)
	cmpq	264(%rsp), %rsi
	jne	.LBB1000_625
.Ltmp16421:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16422:
	jmp	.LBB1000_624
.LBB1000_632:
	movq	%rax, 32(%rsp)
	movq	712(%rsp), %rax
	vmovdqu	752(%rsp), %ymm0
	vmovdqu	768(%rsp), %ymm1
	movq	%rbp, 184(%rsp)
	movq	728(%rsp), %rbp
	movq	736(%rsp), %rbx
	movq	%rax, 48(%rsp)
	movq	720(%rsp), %rax
	vmovdqu	%ymm0, 368(%rsp)
	vmovdqu	%ymm1, 384(%rsp)
	movq	%rax, 96(%rsp)
	movq	744(%rsp), %rax
	movq	%rax, 576(%rsp)
.Ltmp16290:
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16291:
	movq	%r13, %rdi
	testq	%r15, %r15
	je	.LBB1000_638
	leaq	8(%rdi), %r14
	xorl	%r12d, %r12d
	jmp	.LBB1000_636
.LBB1000_635:
	incq	%r12
	addq	$40, %r14
	cmpq	%r12, %r15
	je	.LBB1000_638
.LBB1000_636:
	movq	-8(%r14), %rax
	cmpq	$6, %rax
	jb	.LBB1000_635
	movq	(%r14), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	movq	%r13, %rdi
	jmp	.LBB1000_635
.LBB1000_638:
	movq	1264(%rsp), %rax
	movq	2888(%rsp), %r15
	testq	%rax, %rax
	je	.LBB1000_640
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_640:
	movq	120(%rsp), %r12
	movq	32(%rsp), %r13
	jmp	.LBB1000_471
.LBB1000_641:
	cmpq	$32, %rax
	jae	.LBB1000_646
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB1000_650
.LBB1000_643:
	movq	%rsi, %r12
.LBB1000_644:
	movb	$1, %r14b
	movq	%r12, 32(%rsp)
	movq	%r15, 184(%rsp)
.Ltmp16426:
	leaq	176(%rsp), %rdi
	movb	$1, %bl
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16427:
	movq	120(%rsp), %r12
	movq	2888(%rsp), %r15
	movq	264(%rsp), %rax
	movq	272(%rsp), %rcx
	movl	292(%rsp), %ebp
	movq	$-1, %r13
	movq	%rax, 48(%rsp)
	movb	$2, %al
	movq	%rcx, 96(%rsp)
	movq	%rax, 72(%rsp)
	jmp	.LBB1000_516
.LBB1000_646:
	vmovdqa64	.LCPI1000_7(%rip), %zmm1
	vpbroadcastq	.LCPI1000_8(%rip), %zmm2
	vpbroadcastq	.LCPI1000_9(%rip), %zmm3
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
.LBB1000_647:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%r14,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%r14,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB1000_647
	vpaddq	%zmm0, %zmm4, %zmm0
	movq	296(%rsp), %rcx
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
	je	.LBB1000_655
	testb	$24, %cl
	je	.LBB1000_653
.LBB1000_650:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI1000_7(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI1000_8(%rip), %zmm2
	vpbroadcastq	.LCPI1000_10(%rip), %zmm3
	movq	296(%rsp), %rax
	vmovq	%rbx, %xmm0
	andq	$-8, %rax
	subq	%rax, %rcx
.LBB1000_651:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%r14,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB1000_651
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, 296(%rsp)
	je	.LBB1000_655
.LBB1000_653:
	movq	296(%rsp), %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%r14), %rax
.LBB1000_654:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB1000_654
.LBB1000_655:
	movq	912(%r15), %rax
	movq	928(%r15), %rsi
	leaq	912(%r15), %rdx
	subq	%rsi, %rax
	cmpq	%rax, %rbx
	ja	.LBB1000_970
.LBB1000_656:
	cmpq	1016(%r15), %rbx
	ja	.LBB1000_971
.LBB1000_657:
	movq	296(%rsp), %rax
	movq	%r14, 1752(%rsp)
	movq	%r14, 1760(%rsp)
	movq	%r13, 1768(%rsp)
	movq	64(%rsp), %r13
	leaq	(%rax,%rax,4), %rcx
	shlq	$5, %rcx
	addq	%r14, %rcx
	movq	%rcx, 1704(%rsp)
	movq	%rcx, 1776(%rsp)
	testq	%rax, %rax
	je	.LBB1000_838
	leaq	16(%r13), %rax
	leaq	272(%r13), %rcx
	movq	%rax, 240(%rsp)
	movq	%rcx, 688(%rsp)
.LBB1000_659:
	leaq	160(%r14), %rdx
	movq	%rdx, 1760(%rsp)
	movq	(%r14), %rax
	cmpq	$-1, %rax
	je	.LBB1000_838
	movq	%rax, 704(%rsp)
	leaq	712(%rsp), %rcx
	movq	%rdx, 1712(%rsp)
	vmovdqu64	8(%r14), %zmm0
	vmovdqu64	72(%r14), %zmm1
	vmovdqu64	96(%r14), %zmm2
	vmovdqu64	%zmm2, 88(%rcx)
	vmovdqu64	%zmm1, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	712(%rsp), %r14
	movq	728(%rsp), %rcx
	imulq	$88, 720(%rsp), %rsi
	movq	744(%rsp), %rdx
	movq	760(%rsp), %rdi
	movq	768(%rsp), %rbx
	movq	%rcx, 680(%rsp)
	movq	736(%rsp), %rcx
	movq	%r14, 608(%rsp)
	movq	%rax, 624(%rsp)
	movq	752(%rsp), %rax
	movq	%rdx, 1728(%rsp)
	movq	%r14, 616(%rsp)
	addq	%r14, %rsi
	movq	%rsi, 56(%rsp)
	movq	%rsi, 632(%rsp)
	movq	%rax, 320(%rsp)
	movq	%rcx, 328(%rsp)
	testq	%rbx, %rbx
	je	.LBB1000_825
	movq	800(%rsp), %rax
	movq	80(%rsp), %rbp
	shlq	$5, %rbx
	addq	$8, %rcx
	movq	$0, 552(%rsp)
	movq	%r14, 144(%rsp)
	movq	%rdi, 312(%rsp)
	addq	%rdi, %rbx
	movq	%rcx, 1720(%rsp)
	movq	%rbx, 136(%rsp)
	movq	%rax, 128(%rsp)
	movq	%rdi, %rax
	jmp	.LBB1000_664
.LBB1000_662:
	movq	312(%rsp), %rdi
.LBB1000_663:
	movq	1736(%rsp), %rax
	movq	%rbp, 1232(%rsp)
	addq	$32, %rax
	cmpq	%rbx, %rax
	je	.LBB1000_826
.LBB1000_664:
	movq	552(%rsp), %rcx
	movq	16(%rax), %rdx
	movq	24(%rax), %rsi
	movq	8(%rax), %rbx
	movq	%rax, %r8
	movq	%rcx, 72(%rsp)
	movq	(%rax), %rcx
	movq	%rdx, 88(%rsp)
	movq	%rsi, 32(%rsp)
	testq	%rcx, %rcx
	je	.LBB1000_671
	cmpq	$-1, 560(%rsp)
	je	.LBB1000_671
	movq	80(%r13), %rax
	movq	$-1, %rsi
.LBB1000_667:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 80(%r13)
	jne	.LBB1000_667
	movq	240(%rsp), %rdx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	movq	(%rdx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1000_671
	movq	%r8, %r13
	movq	%rcx, 376(%rsp)
	movq	%rax, 384(%rsp)
	movw	$0, 368(%rsp)
.Ltmp16335:
	movq	240(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp16336:
	cmpb	$-1, 176(%rsp)
	movq	%r13, %r8
	jne	.LBB1000_956
.LBB1000_671:
	movq	1728(%rsp), %rdx
	movq	72(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB1000_965
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	movq	%r8, 1736(%rsp)
	movq	%rbp, 80(%rsp)
	cmovbq	%rbx, %rsi
	cmpq	%rdi, %rbx
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB1000_962
	leaq	(,%rdi,8), %rax
	movq	%r14, 152(%rsp)
	leaq	.LJTI1000_0(%rip), %r8
	leaq	(%rax,%rax,2), %rbx
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r10
	cmpq	%rsi, %rdi
	movq	$-1, %rdi
	jne	.LBB1000_680
	xorl	%r14d, %r14d
	xorl	%r15d, %r15d
	xorl	%r11d, %r11d
.LBB1000_675:
	cmpq	$-1, 304(%rsp)
	movq	$-1, %rbp
	movq	%r15, 1416(%rsp)
	movq	%r11, 696(%rsp)
	movq	%r10, 48(%rsp)
	movq	%rsi, 552(%rsp)
	je	.LBB1000_686
	movq	88(%rsp), %rax
	movl	$0, %ecx
	movq	56(%rsp), %r12
	movl	$0, %r15d
	subq	128(%rsp), %rax
	cmovbq	%rcx, %rax
	subq	144(%rsp), %r12
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %r12
	imulq	%rcx, %r12
	cmpq	%r12, %rax
	cmovbq	%rax, %r12
	testq	%r12, %r12
	je	.LBB1000_687
	movq	144(%rsp), %rax
	xorl	%r15d, %r15d
	leaq	8(%rax), %r13
.LBB1000_678:
.Ltmp16338:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16339:
	addq	%rax, %r15
	cmovbq	%rbp, %r15
	addq	$88, %r13
	decq	%r12
	jne	.LBB1000_678
	jmp	.LBB1000_687
.LBB1000_680:
	movq	%r10, %rdx
	subq	%rbx, %rdx
	movabsq	$-6148914691236517205, %rax
	xorl	%r11d, %r11d
	xorl	%r15d, %r15d
	xorl	%r14d, %r14d
	mulxq	%rax, %rax, %rax
	movq	1720(%rsp), %rcx
	shrq	$4, %rax
	addq	%rbx, %rcx
	jmp	.LBB1000_683
.LBB1000_681:
	cmpq	%rdx, %r11
	cmovbeq	%rdx, %r11
.LBB1000_682:
	addq	$24, %rcx
	decq	%rax
	je	.LBB1000_675
.LBB1000_683:
	movzbl	-8(%rcx), %r9d
	movq	(%rcx), %rdx
	movslq	(%r8,%r9,4), %r9
	addq	%r8, %r9
	jmpq	*%r9
.LBB1000_684:
	addq	%rdx, %r14
	cmovbq	%rdi, %r14
	jmp	.LBB1000_682
.LBB1000_685:
	addq	%rdx, %r15
	cmovbq	%rdi, %r15
	jmp	.LBB1000_682
.LBB1000_686:
	xorl	%r15d, %r15d
.LBB1000_687:
	movq	64(%rsp), %rax
	movq	$-1, %r13
	movl	296(%rax), %eax
	testl	%eax, %eax
	je	.LBB1000_703
.LBB1000_688:
	cmpq	$-1, 560(%rsp)
	je	.LBB1000_690
	movq	64(%rsp), %rcx
	movq	80(%rcx), %rax
	addq	%r14, %rax
	cmovbq	%r13, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB1000_704
.LBB1000_690:
	cmpq	$-1, 304(%rsp)
	je	.LBB1000_692
	movq	2888(%rsp), %rcx
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	64(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%r15, %rcx
	cmovbq	%r13, %rcx
	addq	1416(%rsp), %rcx
	cmovbq	%r13, %rcx
	addq	696(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%r13, %rcx
	addq	%rcx, %rax
	cmovbq	%r13, %rax
	cmpq	40(%rdx), %rax
	ja	.LBB1000_704
.LBB1000_692:
	cmpq	$-1, 560(%rsp)
	movq	2888(%rsp), %r9
	movq	328(%rsp), %r10
	movq	128(%rsp), %r11
	movq	144(%rsp), %rbp
	movq	48(%rsp), %r15
	je	.LBB1000_694
	movq	72(%rsp), %rcx
	movq	%rbx, %rax
	cmpq	552(%rsp), %rcx
	jne	.LBB1000_698
.LBB1000_694:
	movq	120(%rsp), %r12
	movq	%r15, %rcx
	movb	$1, %r15b
	movq	72(%rsp), %rax
	cmpq	552(%rsp), %rax
	je	.LBB1000_777
.LBB1000_695:
	movq	328(%rsp), %rax
	cmpb	$2, -24(%rax,%rcx)
	je	.LBB1000_762
	addq	$-24, %rcx
	cmpq	%rcx, %rbx
	jne	.LBB1000_695
	jmp	.LBB1000_777
.LBB1000_697:
	addq	$24, %rax
	cmpq	%rax, %r15
	je	.LBB1000_694
.LBB1000_698:
	cmpb	$0, (%r10,%rax)
	jne	.LBB1000_697
	movzbl	1(%r10,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB1000_697
	movq	632(%r9), %rdx
	testq	%rdx, %rdx
	je	.LBB1000_697
	movl	1228(%r9), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB1000_697
	movq	%rsi, %r8
	shlq	$7, %r8
	movq	8(%r10,%rax), %rdi
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB1000_697
.LBB1000_703:
	movq	688(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB1000_688
.LBB1000_704:
	movq	128(%rsp), %rbp
	movq	72(%rsp), %rax
	cmpq	552(%rsp), %rax
	jne	.LBB1000_714
	movq	120(%rsp), %r12
	movq	64(%rsp), %r13
	movq	152(%rsp), %r14
.LBB1000_706:
	cmpq	$-1, 304(%rsp)
	movq	%rbp, 128(%rsp)
	je	.LBB1000_761
	movq	2888(%rsp), %r15
	movq	136(%rsp), %rbx
	cmpq	88(%rsp), %rbp
	movq	80(%rsp), %rbp
	jae	.LBB1000_815
	movq	144(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB1000_770
	movq	88(%rsp), %rax
	movq	144(%rsp), %rcx
	leaq	-1(%rax), %rbx
.LBB1000_710:
	movq	8(%rcx), %rax
	movq	%rcx, %r14
	cmpq	$-1, %rax
	je	.LBB1000_769
	movq	(%r14), %rsi
	movq	%rax, 368(%rsp)
	leaq	376(%rsp), %rcx
	movq	80(%r14), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%r14), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16376:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16377:
	movq	128(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB1000_768
	incq	%rax
	leaq	88(%r14), %rcx
	movq	%rax, 128(%rsp)
	cmpq	56(%rsp), %rcx
	jne	.LBB1000_710
	jmp	.LBB1000_769
.LBB1000_714:
	movq	328(%rsp), %rax
	movq	48(%rsp), %rdx
	movq	120(%rsp), %r12
	movq	64(%rsp), %r13
	movq	152(%rsp), %r14
	addq	%rax, %rdx
	addq	%rax, %rbx
	movq	%rdx, 48(%rsp)
	jmp	.LBB1000_717
.LBB1000_747:
	movq	48(%rsp), %rdx
.LBB1000_716:
	addq	$24, %rbx
	cmpq	%rdx, %rbx
	je	.LBB1000_706
.LBB1000_717:
	movzbl	(%rbx), %eax
	leaq	.LJTI1000_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB1000_718:
	cmpq	$-1, 560(%rsp)
	je	.LBB1000_716
	movq	%rbp, 128(%rsp)
	movq	%r14, %r15
	movq	%r13, %rdx
	movzbl	1(%rbx), %r14d
	movq	8(%rbx), %rbp
	movq	16(%rbx), %r13
	movq	$-1, %rsi
	movq	80(%rdx), %rax
.LBB1000_720:
	movq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB1000_720
	movq	240(%rsp), %rcx
	addq	%rbp, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1000_724
	movq	%rcx, 376(%rsp)
	movq	%rax, 384(%rsp)
	movw	$0, 368(%rsp)
.Ltmp16370:
	movq	240(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp16371:
	cmpb	$-1, 176(%rsp)
	jne	.LBB1000_907
.LBB1000_724:
	cmpl	$255, %r14d
	je	.LBB1000_715
	movq	2888(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB1000_715
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB1000_715
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%rbp, (%rcx,%r14,8)
.LBB1000_715:
	movq	64(%rsp), %r13
	movq	128(%rsp), %rbp
	movq	48(%rsp), %rdx
	movq	%r15, %r14
	jmp	.LBB1000_716
.LBB1000_728:
	cmpq	$-1, 304(%rsp)
	je	.LBB1000_716
	cmpq	$-1, 40(%r13)
	je	.LBB1000_747
	movq	8(%rbx), %rcx
	movq	16(%rbx), %r15
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB1000_750
	movq	104(%r13), %rax
	movq	$-1, %rsi
.LBB1000_732:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 104(%r13)
	jne	.LBB1000_732
	addq	%rcx, %rax
	movq	40(%r13), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB1000_747
	movq	%rcx, 376(%rsp)
	movq	%rax, 384(%rsp)
	movw	$768, 368(%rsp)
.Ltmp16364:
	movq	240(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp16365:
	jmp	.LBB1000_751
.LBB1000_735:
	movq	8(%rbx), %r15
	cmpq	%r15, %rbp
	jae	.LBB1000_756
	movq	144(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB1000_755
	movq	144(%rsp), %rax
	leaq	-1(%r15), %r12
.LBB1000_738:
	movq	%rax, %r14
	movq	8(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB1000_754
	movq	(%r14), %rsi
	movq	%rax, 368(%rsp)
	leaq	376(%rsp), %rcx
	movq	80(%r14), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%r14), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16359:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16360:
	cmpq	%rbp, %r12
	je	.LBB1000_753
	leaq	88(%r14), %rax
	incq	%rbp
	cmpq	56(%rsp), %rax
	jne	.LBB1000_738
	jmp	.LBB1000_754
.LBB1000_742:
	cmpq	$-1, 304(%rsp)
	je	.LBB1000_716
	movq	8(%rbx), %rcx
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB1000_748
	movq	104(%r13), %rax
	movq	$-1, %rsi
	addq	%rcx, %rax
	movq	40(%r13), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB1000_716
	movq	%rcx, 376(%rsp)
	movq	%rax, 384(%rsp)
	movw	$768, 368(%rsp)
.Ltmp16357:
	movq	240(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp16358:
	movq	48(%rsp), %rdx
	jmp	.LBB1000_749
.LBB1000_748:
	movq	688(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 192(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB1000_749:
	cmpb	$-1, 176(%rsp)
	je	.LBB1000_716
	jmp	.LBB1000_925
.LBB1000_750:
	movq	688(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 192(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB1000_751:
	movzbl	176(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%r15, %r15
	setne	%dl
	testb	%cl, %dl
	jne	.LBB1000_917
	cmpb	$-1, %al
	jmp	.LBB1000_759
.LBB1000_753:
	movq	%r15, %rbp
.LBB1000_754:
	movq	120(%rsp), %r12
	addq	$88, %r14
	movq	%r14, 144(%rsp)
.LBB1000_755:
	movq	%r14, 616(%rsp)
.LBB1000_756:
	movq	2888(%rsp), %rsi
	cmpq	$-1, 304(%rsp)
	je	.LBB1000_747
.Ltmp16362:
	leaq	368(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp16363:
	cmpb	$-1, 368(%rsp)
.LBB1000_759:
	movq	48(%rsp), %rdx
	je	.LBB1000_716
	jmp	.LBB1000_925
.LBB1000_761:
	movq	2888(%rsp), %r15
	movq	80(%rsp), %rbp
	movq	136(%rsp), %rbx
	jmp	.LBB1000_815
.LBB1000_762:
	movq	328(%rsp), %rax
	movq	-16(%rax,%rcx), %rbx
	cmpq	%rbx, %r11
	jae	.LBB1000_776
	cmpq	56(%rsp), %rbp
	je	.LBB1000_771
.LBB1000_764:
	movq	8(%rbp), %rax
	movq	%r11, %r15
	movq	%rbp, %rdx
	cmpq	$-1, %rax
	je	.LBB1000_772
	movq	(%rdx), %rsi
	movq	%rax, 368(%rsp)
	leaq	376(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16341:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16342:
	leaq	-1(%rbx), %rax
	cmpq	%r15, %rax
	je	.LBB1000_773
	movq	%rbp, %rdx
	incq	%r15
	addq	$88, %rbp
	movq	%r15, %r11
	cmpq	56(%rsp), %rbp
	jne	.LBB1000_764
	jmp	.LBB1000_774
.LBB1000_768:
	movq	88(%rsp), %rax
	movq	%rax, 128(%rsp)
.LBB1000_769:
	movq	136(%rsp), %rbx
	addq	$88, %r14
	movq	%r14, 144(%rsp)
.LBB1000_770:
	movq	%r14, 616(%rsp)
	jmp	.LBB1000_815
.LBB1000_771:
	movq	152(%rsp), %rdx
	jmp	.LBB1000_775
.LBB1000_772:
	movq	%r15, %r11
	jmp	.LBB1000_774
.LBB1000_773:
	movq	%rbp, %rdx
	movq	%rbx, %r11
.LBB1000_774:
	addq	$88, %rdx
	movq	%r11, 128(%rsp)
	movq	%rdx, 144(%rsp)
.LBB1000_775:
	movq	%rdx, 152(%rsp)
	movq	%rdx, 616(%rsp)
.LBB1000_776:
	xorl	%r15d, %r15d
.LBB1000_777:
	movq	80(%rsp), %rbp
	movq	136(%rsp), %rbx
	cmpq	$-1, 560(%rsp)
	je	.LBB1000_785
	testq	%r14, %r14
	je	.LBB1000_785
	movq	64(%rsp), %rdx
	movq	80(%rdx), %rax
.LBB1000_780:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%r13, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB1000_780
	movq	240(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%r13, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1000_785
	movq	%rcx, 376(%rsp)
	movq	%rax, 384(%rsp)
	movw	$0, 368(%rsp)
.Ltmp16344:
	movq	240(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp16345:
	cmpb	$-1, 176(%rsp)
	je	.LBB1000_785
	cmpq	$-1, 304(%rsp)
	movq	2888(%rsp), %r15
	movq	64(%rsp), %r13
	movq	152(%rsp), %r14
	movb	$1, %cl
	jne	.LBB1000_804
	jmp	.LBB1000_954
.LBB1000_785:
	cmpq	$-1, 304(%rsp)
	je	.LBB1000_792
	movq	64(%rsp), %r13
	movq	152(%rsp), %r14
	cmpq	$-1, 40(%r13)
	je	.LBB1000_796
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB1000_793
	movq	104(%r13), %rax
	movq	1416(%rsp), %rsi
	movq	$-1, %rdx
.LBB1000_789:
	movq	%rax, %rcx
	addq	%rsi, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 104(%r13)
	jne	.LBB1000_789
	movq	40(%r13), %rcx
	addq	%rsi, %rax
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB1000_796
	movq	%rcx, 376(%rsp)
	movq	%rax, 384(%rsp)
	movw	$768, 368(%rsp)
.Ltmp16347:
	movq	240(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)
.Ltmp16348:
	jmp	.LBB1000_794
.LBB1000_792:
	movq	2888(%rsp), %r15
	movq	64(%rsp), %r13
	movq	152(%rsp), %r14
	jmp	.LBB1000_815
.LBB1000_793:
	movq	688(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 192(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB1000_794:
	cmpb	$-1, 176(%rsp)
	je	.LBB1000_796
	movb	$1, %cl
	jmp	.LBB1000_802
.LBB1000_796:
	testb	%r15b, %r15b
	movq	2888(%rsp), %r15
	jne	.LBB1000_799
.Ltmp16349:
	leaq	368(%rsp), %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp16350:
	cmpb	$-1, 368(%rsp)
	movq	2888(%rsp), %r15
	movb	$1, %cl
	jne	.LBB1000_804
.LBB1000_799:
	movq	696(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB1000_803
.Ltmp16351:
	movq	240(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp16352:
	cmpb	$-1, 368(%rsp)
	setne	%cl
.LBB1000_802:
	movq	2888(%rsp), %r15
	jmp	.LBB1000_804
.LBB1000_803:
	xorl	%ecx, %ecx
.LBB1000_804:
	movq	88(%rsp), %rax
	cmpq	%rax, 128(%rsp)
	jae	.LBB1000_814
	movq	144(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB1000_813
	movq	88(%rsp), %rax
	movq	128(%rsp), %rbx
	movl	%ecx, 48(%rsp)
	leaq	-1(%rax), %r14
	movq	144(%rsp), %rax
.LBB1000_807:
	movq	%rax, %rdx
	movq	8(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB1000_812
	movq	(%rdx), %rsi
	movq	%rax, 368(%rsp)
	leaq	376(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16354:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	368(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16355:
	cmpq	%rbx, %r14
	je	.LBB1000_811
	leaq	88(%rbp), %rax
	incq	%rbx
	movq	%rbp, %rdx
	cmpq	56(%rsp), %rax
	jne	.LBB1000_807
	jmp	.LBB1000_812
.LBB1000_811:
	movq	88(%rsp), %rbx
	movq	%rbp, %rdx
.LBB1000_812:
	movq	%rbx, 128(%rsp)
	movq	80(%rsp), %rbp
	movq	136(%rsp), %rbx
	movl	48(%rsp), %ecx
	addq	$88, %rdx
	movq	%rdx, %r14
	movq	%rdx, 144(%rsp)
.LBB1000_813:
	movq	%r14, 616(%rsp)
.LBB1000_814:
	testb	%cl, %cl
	jne	.LBB1000_954
.LBB1000_815:
	cmpq	$0, 32(%rsp)
	je	.LBB1000_662
	movq	1248(%rsp), %rax
	movq	312(%rsp), %rdi
	movq	%r14, 152(%rsp)
	movq	%rax, 696(%rsp)
	jmp	.LBB1000_818
.LBB1000_817:
	movq	272(%rsp), %rax
	movq	48(%rsp), %rdx
	leaq	(%r12,%r12,4), %rcx
	shll	$16, %ebp
	movq	80(%rsp), %rdi
	movq	32(%rsp), %rsi
	addq	$40, %r14
	incq	%r12
	orl	%ebp, %r15d
	movq	%r14, %rbp
	movq	152(%rsp), %r14
	shlq	$32, %r15
	orq	%r15, %r13
	movq	2888(%rsp), %r15
	movq	%rdx, (%rax,%rcx,8)
	movq	72(%rsp), %rdx
	decq	%rsi
	movq	%rsi, 32(%rsp)
	movq	%rdx, 8(%rax,%rcx,8)
	movq	88(%rsp), %rdx
	movq	%rdx, 16(%rax,%rcx,8)
	movb	%bl, 24(%rax,%rcx,8)
	movq	%r13, %rdx
	shrq	$48, %rdx
	movl	%r13d, 25(%rax,%rcx,8)
	shrq	$32, %r13
	movq	136(%rsp), %rbx
	movw	%r13w, 29(%rax,%rcx,8)
	movb	%dl, 31(%rax,%rcx,8)
	movq	%rdi, 32(%rax,%rcx,8)
	movq	%r12, 280(%rsp)
	movq	120(%rsp), %r12
	movq	64(%rsp), %r13
	movq	312(%rsp), %rdi
	testq	%rsi, %rsi
	je	.LBB1000_663
.LBB1000_818:
	cmpq	696(%rsp), %rbp
	je	.LBB1000_663
	movq	32(%rbp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%r15, 176(%rsp)
	cmpq	$0, 184(%rsp)
	je	.LBB1000_821
	movq	32(%rbp), %rax
	leaq	376(%rsp), %rcx
	movq	%rbp, %r14
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB1000_823
.LBB1000_821:
	movq	664(%r15), %rdx
	movq	%rbp, %r14
.Ltmp16379:
	movq	96(%rsp), %rsi
	leaq	368(%rsp), %rdi
	leaq	192(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16380:
	movq	368(%rsp), %r13
	cmpq	$-1, %r13
	jne	.LBB1000_900
.LBB1000_823:
	movq	384(%rsp), %rcx
	movq	376(%rsp), %rax
	movq	392(%rsp), %rdx
	movzbl	400(%rsp), %ebx
	movzbl	407(%rsp), %ebp
	movzwl	405(%rsp), %r15d
	movl	401(%rsp), %r13d
	movq	280(%rsp), %r12
	movq	%rcx, 72(%rsp)
	movq	408(%rsp), %rcx
	movq	%rax, 48(%rsp)
	movq	%rdx, 88(%rsp)
	movq	%rcx, 80(%rsp)
	cmpq	264(%rsp), %r12
	jne	.LBB1000_817
.Ltmp16389:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	264(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp16390:
	jmp	.LBB1000_817
.LBB1000_825:
	movq	80(%rsp), %rbp
.LBB1000_826:
	movq	320(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB1000_828
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB1000_828:
.Ltmp16399:
	leaq	608(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16400:
	movq	1712(%rsp), %r14
	movq	680(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_831
	movq	328(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_831:
	movq	792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_834
	lock		decq	(%rax)
	jne	.LBB1000_834
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	792(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1000_834:
	movq	824(%rsp), %rax
	movq	%rbp, 80(%rsp)
	testq	%rax, %rax
	je	.LBB1000_837
	lock		decq	(%rax)
	jne	.LBB1000_837
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	824(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1000_837:
	cmpq	1704(%rsp), %r14
	jne	.LBB1000_659
.LBB1000_838:
	movb	$1, %al
	xorl	%ebp, %ebp
	movl	%eax, 56(%rsp)
.Ltmp16404:
	leaq	1752(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16405:
	movq	272(%rsp), %rcx
	movq	264(%rsp), %rax
	movq	280(%rsp), %rdx
	movq	%rcx, 96(%rsp)
	movq	64(%rsp), %rcx
	movq	%rax, 48(%rsp)
	movq	%rdx, 32(%rsp)
	lock		decq	(%rcx)
	jne	.LBB1000_841
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp16409:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1256(%rsp), %rdi
	xorl	%ebx, %ebx
	callq	*%rax
.Ltmp16410:
.LBB1000_841:
	xorl	%r14d, %r14d
.Ltmp16411:
	leaq	1224(%rsp), %rdi
	xorl	%ebx, %ebx
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16412:
	movl	292(%rsp), %ebp
	movb	$2, %dl
.LBB1000_843:
	movq	48(%rsp), %rax
	movq	96(%rsp), %rsi
	movq	32(%rsp), %rcx
	movq	%rax, 368(%rsp)
	movq	616(%r15), %rax
	movq	%rsi, 376(%rsp)
	movq	%rcx, 384(%rsp)
	testq	%rax, %rax
	je	.LBB1000_852
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	jne	.LBB1000_846
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 191(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 176(%rsp)
.LBB1000_846:
	testb	%dl, %dl
	je	.LBB1000_851
	movzbl	%dl, %eax
	cmpl	$2, %eax
	je	.LBB1000_853
	cmpb	$-1, %bl
	je	.LBB1000_901
	cmpb	$2, 472(%r15)
	jne	.LBB1000_851
	vmovdqa	176(%rsp), %xmm0
	movq	191(%rsp), %rax
	movq	696(%r15), %rdi
	movb	%bl, 704(%rsp)
	vmovdqu	%xmm0, 705(%rsp)
	movq	%rax, 720(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	jne	.LBB1000_980
.LBB1000_851:
	xorl	%ebp, %ebp
	jmp	.LBB1000_853
.LBB1000_852:
	cmpb	$2, %dl
	movb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, %ebp
.LBB1000_853:
	cmpb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movq	32(%rsp), %rbp
	movq	%rax, 72(%rsp)
.LBB1000_854:
	movzbl	72(%rsp), %ebx
	movq	576(%rsp), %r14
.LBB1000_855:
	vmovdqu	2448(%rsp), %ymm0
	movq	48(%rsp), %rax
	movq	96(%rsp), %rcx
	movq	%rax, 1792(%rsp)
	movq	%rcx, 1800(%rsp)
	movq	%rbp, 1808(%rsp)
	vmovdqu	%ymm0, 704(%rsp)
.Ltmp16451:
	leaq	704(%rsp), %rsi
	movq	%r15, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::absorb_worker_witnesses::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16452:
	testb	$1, %bl
	je	.LBB1000_859
	movq	544(%rsp), %rdx
	cmpq	%rdx, %r14
	ja	.LBB1000_966
	jne	.LBB1000_876
.LBB1000_859:
	movq	1808(%rsp), %rax
	vmovdqu	1792(%rsp), %xmm0
	movq	%rax, 1360(%rsp)
	movq	336(%rsp), %rax
	vmovdqa	%xmm0, 1344(%rsp)
	testq	%rax, %rax
	je	.LBB1000_862
	lock		decq	(%rax)
	jne	.LBB1000_862
	#MEMBARRIER
.Ltmp16469:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	336(%rsp), %rdi
	callq	*%rax
.Ltmp16470:
.LBB1000_862:
.Ltmp16471:
	leaq	1472(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16472:
.LBB1000_863:
	movq	696(%r15), %rax
	movl	40(%rax), %ecx
	testl	%ecx, %ecx
	je	.LBB1000_873
.LBB1000_864:
	vmovdqa	1344(%rsp), %xmm0
	movq	1360(%rsp), %rax
	movq	256(%rsp), %rcx
	movq	%rax, 1808(%rsp)
	vmovdqa	%xmm0, 1792(%rsp)
	movq	%rcx, 1816(%rsp)
.Ltmp16476:
	leaq	704(%rsp), %rdi
	leaq	2240(%rsp), %rsi
	leaq	1792(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
.Ltmp16477:
	vmovdqu64	704(%rsp), %zmm0
	vmovdqu64	736(%rsp), %zmm1
	movq	168(%rsp), %rax
	xorl	%ebp, %ebp
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
.Ltmp16479:
	leaq	584(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp16480:
	leaq	528(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	cmpq	$0, 352(%rsp)
	je	.LBB1000_868
	movq	248(%rsp), %rdi
	movq	344(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB1000_868:
	movq	232(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1000_870
	xorl	%ebx, %ebx
	#MEMBARRIER
.Ltmp16482:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	232(%rsp), %rdi
	xorl	%r12d, %r12d
	callq	*%rax
.Ltmp16483:
.LBB1000_870:
	xorl	%ebp, %ebp
.Ltmp16484:
	leaq	1168(%rsp), %rdi
	xorl	%r12d, %r12d
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16485:
	jmp	.LBB1000_871
.LBB1000_873:
	movzbl	16(%rax), %ecx
	cmpb	$-1, %cl
	je	.LBB1000_864
	movb	%cl, 1792(%rsp)
	vmovdqu	17(%rax), %xmm0
	vmovdqu	%xmm0, 1793(%rsp)
	movq	32(%rax), %rax
	movq	%rax, 1808(%rsp)
.Ltmp16473:
	movq	160(%rsp), %rsi
	movq	256(%rsp), %rcx
	leaq	704(%rsp), %rdi
	leaq	1792(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
.Ltmp16474:
	vmovdqu64	704(%rsp), %zmm0
	vmovdqu64	736(%rsp), %zmm1
	movq	168(%rsp), %rax
	leaq	1344(%rsp), %rdi
	vmovdqu64	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebp, %ebp
	movq	592(%rsp), %rbx
	movq	600(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1000_531
	jmp	.LBB1000_534
.LBB1000_876:
	movq	536(%rsp), %rax
	leaq	(%rdx,%rdx,8), %rcx
	addq	$16, %r12
	leaq	1432(%rsp), %rbx
	movq	%r12, 120(%rsp)
	leaq	(%rax,%rcx,8), %rcx
	movq	%rcx, 96(%rsp)
	leaq	(%r14,%r14,8), %rcx
	leaq	(%rax,%rcx,8), %r14
.LBB1000_877:
	movq	1216(%rsp), %rsi
.Ltmp16453:
	movq	%rbx, %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
.Ltmp16454:
	movq	1432(%rsp), %r15
	leaq	1440(%rsp), %rdi
	movq	%r15, %rax
	cmpq	$6, %r15
	jb	.LBB1000_880
	movq	1440(%rsp), %rdi
	movq	1448(%rsp), %rax
.LBB1000_880:
	movq	360(%rsp), %rdx
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB1000_963
	movq	(%r14), %rsi
	decq	%rsi
	cmpq	$4, %rsi
	jbe	.LBB1000_883
	movq	16(%r14), %rsi
	movq	8(%r14), %rax
	decq	%rsi
	jmp	.LBB1000_884
.LBB1000_883:
	leaq	8(%r14), %rax
.LBB1000_884:
	cmpq	%rsi, %rdx
	jne	.LBB1000_964
	movq	memcpy@GOTPCREL(%rip), %r12
	shlq	$3, %rdx
	movq	%rax, %rsi
	callq	*%r12
	movq	600(%rsp), %rbx
	movq	2880(%rsp), %rax
	movq	2888(%rsp), %r15
	cmpq	%rbx, %rax
	cmovbq	%rax, %rbx
	testq	%rbx, %rbx
	je	.LBB1000_895
	movq	592(%rsp), %r13
	movq	120(%rsp), %r12
	xorl	%ebp, %ebp
	addq	$16, %r13
	jmp	.LBB1000_888
.LBB1000_887:
	movq	2888(%rsp), %r15
	incq	%rbp
	addq	$24, %r13
	addq	$120, %r12
	vmovq	%xmm0, (%rax,%rdi,8)
	cmpq	%rbp, %rbx
	je	.LBB1000_895
.LBB1000_888:
	vmovups	1176(%rsp), %xmm0
	movq	232(%rsp), %rax
	movq	-8(%r13), %rdx
	movq	(%r13), %rcx
	movq	56(%r14), %r8
	movq	64(%r14), %r9
	addq	$16, %rax
.Ltmp16458:
	leaq	704(%rsp), %rdi
	movq	%r12, %rsi
	movq	%r15, 24(%rsp)
	movq	%rax, 16(%rsp)
	vmovups	%xmm0, (%rsp)
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp16459:
	vmovq	712(%rsp), %xmm0
	movq	704(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB1000_897
	movq	1432(%rsp), %r15
	movq	%r15, %rsi
	cmpq	$6, %r15
	jb	.LBB1000_892
	movq	1448(%rsp), %rsi
.LBB1000_892:
	movq	360(%rsp), %rdi
	decq	%rsi
	addq	%rbp, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1000_982
	leaq	1440(%rsp), %rax
	cmpq	$6, %r15
	jb	.LBB1000_887
	movq	1440(%rsp), %rax
	jmp	.LBB1000_887
.LBB1000_895:
.Ltmp16463:
	leaq	1432(%rsp), %rbx
	leaq	1792(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::push_mut
.Ltmp16464:
	addq	$72, %r14
	cmpq	96(%rsp), %r14
	jne	.LBB1000_877
	jmp	.LBB1000_859
.LBB1000_897:
	vmovups	720(%rsp), %zmm1
	vmovups	736(%rsp), %zmm2
	movq	168(%rsp), %rcx
	vmovups	%zmm2, 48(%rcx)
	vmovups	%zmm1, 32(%rcx)
	movq	%rax, 16(%rcx)
	movq	1432(%rsp), %rax
	vmovq	%xmm0, 24(%rcx)
	movq	$1, (%rcx)
	cmpq	$6, %rax
	jb	.LBB1000_899
	movq	1440(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB1000_899:
	leaq	1792(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	336(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1000_527
	jmp	.LBB1000_529
.LBB1000_900:
	movq	376(%rsp), %rax
	vmovups	416(%rsp), %ymm0
	movzbl	407(%rsp), %edx
	movzwl	405(%rsp), %ecx
	addq	$40, %r14
	movq	384(%rsp), %rbp
	movq	%r14, 1232(%rsp)
	movq	392(%rsp), %r14
	movq	%rax, 48(%rsp)
	movzbl	400(%rsp), %eax
	shll	$16, %edx
	orl	%edx, %ecx
	shlq	$32, %rcx
	vmovups	%ymm0, 1264(%rsp)
	vmovdqu	432(%rsp), %ymm0
	movq	%rax, 72(%rsp)
	movl	401(%rsp), %eax
	orq	%rcx, %rax
	movq	%rax, 80(%rsp)
	movq	408(%rsp), %rax
	vmovdqu	%ymm0, 1280(%rsp)
	movq	%rax, 88(%rsp)
	movb	$1, %al
	movl	%eax, 56(%rsp)
	jmp	.LBB1000_928
.LBB1000_901:
	movb	$-1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB1000_853
.LBB1000_904:
.Ltmp16515:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.496(%rip), %rcx
	xorl	%edi, %edi
	vzeroupper
	callq	*%rax
.Ltmp16516:
	jmp	.LBB1000_987
.LBB1000_905:
	vmovups	752(%rsp), %ymm0
	movq	712(%rsp), %rax
	movzbl	743(%rsp), %ecx
	movq	720(%rsp), %rsi
	movq	728(%rsp), %rdx
	movzwl	741(%rsp), %ebp
	addq	$40, %r15
	movb	$1, %r14b
	movq	%r15, 184(%rsp)
	movq	%rax, 48(%rsp)
	movzbl	736(%rsp), %eax
	movl	%ecx, 64(%rsp)
	movq	744(%rsp), %rcx
	movq	%rsi, 96(%rsp)
	movq	%rdx, 32(%rsp)
	vmovups	%ymm0, 1264(%rsp)
	vmovdqu	768(%rsp), %ymm0
	movq	%rax, 72(%rsp)
	movl	737(%rsp), %eax
	movq	%rcx, 88(%rsp)
	movq	%rax, 80(%rsp)
	vmovdqu	%ymm0, 1280(%rsp)
.Ltmp16416:
	leaq	176(%rsp), %rdi
	movb	$1, %bl
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16417:
	movq	120(%rsp), %r12
	movq	2888(%rsp), %r15
	movl	64(%rsp), %eax
	movb	$1, %r14b
	movb	$1, %bl
	shll	$16, %eax
	orl	%eax, %ebp
	shlq	$32, %rbp
	addq	%rbp, 80(%rsp)
	jmp	.LBB1000_943
.LBB1000_907:
	movq	128(%rsp), %rcx
	movq	%r13, %rax
	addq	$-1, %rax
	jae	.LBB1000_925
	cmpq	%rax, %rcx
	jae	.LBB1000_925
	movq	144(%rsp), %rax
	movq	%r15, %r14
	cmpq	56(%rsp), %rax
	je	.LBB1000_924
	subq	%rcx, %r13
	addq	$88, %rax
	leaq	368(%rsp), %rbx
	addq	$-2, %r13
.LBB1000_911:
	movq	%rax, %r14
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB1000_924
	movq	-88(%r14), %rsi
	movq	%rax, 368(%rsp)
	leaq	376(%rsp), %rcx
	movq	-8(%r14), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%r14), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16373:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16374:
	subq	$1, %r13
	jb	.LBB1000_924
	leaq	88(%r14), %rax
	cmpq	56(%rsp), %r14
	jne	.LBB1000_911
	jmp	.LBB1000_924
.LBB1000_915:
	movq	536(%rsp), %rdi
	cmpq	$21, %rsi
	jae	.LBB1000_983
	callq	core::slice::sort::shared::smallsort::insertion_sort_shift_left::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), <[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>::{closure#0}>
	jmp	.LBB1000_256
.LBB1000_917:
	leaq	-1(%r15), %rax
	cmpq	%rax, %rbp
	jae	.LBB1000_925
	movq	144(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB1000_924
	subq	%rbp, %r15
	addq	$88, %rax
	leaq	368(%rsp), %rbx
	addq	$-2, %r15
.LBB1000_920:
	movq	%rax, %r14
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB1000_924
	movq	-88(%r14), %rsi
	movq	%rax, 368(%rsp)
	leaq	376(%rsp), %rcx
	movq	-8(%r14), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%r14), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp16367:
	movq	96(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp16368:
	subq	$1, %r15
	jb	.LBB1000_924
	leaq	88(%r14), %rax
	cmpq	56(%rsp), %r14
	jne	.LBB1000_920
.LBB1000_924:
	movq	%r14, 616(%rsp)
.LBB1000_925:
	movq	264(%rsp), %rax
	movq	272(%rsp), %rbp
	movq	280(%rsp), %r14
	movq	2888(%rsp), %r15
.LBB1000_926:
	movq	$-1, %r13
	movl	$0, 56(%rsp)
	movq	%rax, 48(%rsp)
	movb	$1, %al
	movq	%rax, 72(%rsp)
.LBB1000_927:
.LBB1000_928:
	movq	320(%rsp), %rsi
	movq	680(%rsp), %rbx
	testq	%rsi, %rsi
	je	.LBB1000_930
	movq	312(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB1000_930:
.Ltmp16382:
	leaq	608(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16383:
	testq	%rbx, %rbx
	je	.LBB1000_933
	movq	328(%rsp), %rdi
	shlq	$3, %rbx
	movl	$8, %edx
	leaq	(%rbx,%rbx,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_933:
	movq	792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_936
	lock		decq	(%rax)
	jne	.LBB1000_936
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	792(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1000_936:
	movq	824(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_939
	lock		decq	(%rax)
	jne	.LBB1000_939
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	824(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1000_939:
	movq	%rbp, 96(%rsp)
	xorl	%ebp, %ebp
	movq	%r14, 32(%rsp)
.Ltmp16385:
	leaq	1752(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16386:
	movq	64(%rsp), %rax
	lock		decq	(%rax)
	movl	56(%rsp), %r14d
	jne	.LBB1000_942
	xorl	%ebx, %ebx
	#MEMBARRIER
.Ltmp16387:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1256(%rsp), %rdi
	callq	*%rax
.Ltmp16388:
.LBB1000_942:
	xorl	%ebx, %ebx
.LBB1000_943:
	cmpq	$0, 1408(%rsp)
	movl	292(%rsp), %ebp
	je	.LBB1000_945
.Ltmp16418:
	leaq	1224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16419:
.LBB1000_945:
	testb	%r14b, %r14b
	je	.LBB1000_953
	movq	272(%rsp), %rax
	movq	280(%rsp), %r14
	movq	%rax, 64(%rsp)
	testq	%r14, %r14
	je	.LBB1000_951
	movq	64(%rsp), %rax
	leaq	8(%rax), %r15
	jmp	.LBB1000_949
.LBB1000_948:
	addq	$40, %r15
	decq	%r14
	je	.LBB1000_951
.LBB1000_949:
	movq	-8(%r15), %rax
	cmpq	$6, %rax
	jb	.LBB1000_948
	movq	(%r15), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB1000_948
.LBB1000_951:
	movq	264(%rsp), %rax
	movq	2888(%rsp), %r15
	testq	%rax, %rax
	je	.LBB1000_953
	movq	64(%rsp), %rdi
	shlq	$3, %rax
	movl	$8, %edx
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_953:
	testb	%bl, %bl
	jne	.LBB1000_516
	jmp	.LBB1000_522
.LBB1000_954:
	movq	264(%rsp), %rax
	movq	272(%rsp), %rbp
	movq	280(%rsp), %r14
	jmp	.LBB1000_926
.LBB1000_955:
.Ltmp16156:
	movq	<hashbrown::raw::RawTable<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>))>>::reserve_rehash::<hashbrown::map::make_hasher<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>::{closure#0}>@GOTPCREL(%rip), %rax
	leaq	640(%rsp), %rdi
	movl	$1, %esi
	movl	$1, %ecx
	vzeroupper
	callq	*%rax
.Ltmp16157:
	jmp	.LBB1000_186
.LBB1000_956:
	movq	264(%rsp), %rax
	movq	272(%rsp), %rbp
	movq	280(%rsp), %r14
	movq	$-1, %r13
	movq	$0, 72(%rsp)
	movl	$0, 56(%rsp)
	movq	%rax, 48(%rsp)
	jmp	.LBB1000_927
.LBB1000_957:
.Ltmp16141:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.714(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16142:
	jmp	.LBB1000_987
.LBB1000_958:
.Ltmp16260:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.489(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	vzeroupper
	callq	*%r8
.Ltmp16261:
	jmp	.LBB1000_987
.LBB1000_959:
.Ltmp16250:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.490(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	vzeroupper
	callq	*%rcx
.Ltmp16251:
	jmp	.LBB1000_987
.LBB1000_960:
.Ltmp16138:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.714(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16139:
	jmp	.LBB1000_987
.LBB1000_961:
	vmovdqa	(%rax), %xmm0
	vpmovmskb	%xmm0, %esi
	tzcntl	%esi, %esi
	movzbl	(%rax,%rsi), %edi
	jmp	.LBB1000_225
.LBB1000_962:
.Ltmp16392:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.301(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp16393:
	jmp	.LBB1000_987
.LBB1000_963:
.Ltmp16466:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.492(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	callq	*%r8
.Ltmp16467:
	jmp	.LBB1000_987
.LBB1000_964:
.Ltmp16456:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.493(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	callq	*%rcx
.Ltmp16457:
	jmp	.LBB1000_987
.LBB1000_965:
	leaq	1784(%rsp), %rax
	leaq	176(%rsp), %rcx
	movq	%rdi, 1784(%rsp)
	movq	%rdx, 176(%rsp)
	movq	%rax, 368(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 376(%rsp)
	movq	%rcx, 384(%rsp)
	movq	%rax, 392(%rsp)
.Ltmp16394:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.2054(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.300(%rip), %rdx
	leaq	368(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp16395:
	jmp	.LBB1000_987
.LBB1000_966:
.Ltmp16486:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.495(%rip), %rcx
	movq	%r14, %rdi
	movq	%rdx, %rsi
	callq	*%rax
.Ltmp16487:
	jmp	.LBB1000_987
.LBB1000_967:
.Ltmp16153:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$8, %esi
	callq	*%rax
.Ltmp16154:
	jmp	.LBB1000_987
.LBB1000_968:
.Ltmp16202:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$456, %esi
	callq	*%rax
.Ltmp16203:
	jmp	.LBB1000_987
.LBB1000_969:
.Ltmp16224:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$456, %esi
	callq	*%rax
.Ltmp16225:
	jmp	.LBB1000_987
.LBB1000_970:
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp16331:
	movl	$8, %ecx
	movl	$80, %r8d
	movq	%rdx, %rdi
	movq	%rbx, %rdx
	movb	$1, %bpl
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
	leaq	912(%r15), %rdx
.Ltmp16332:
	jmp	.LBB1000_656
.LBB1000_971:
	movb	$1, %al
	leaq	1000(%r15), %rdi
	movl	%eax, 56(%rsp)
.Ltmp16333:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%rbx, %rsi
	movb	$1, %bpl
	vzeroupper
	callq	*%rax
.Ltmp16334:
	jmp	.LBB1000_657
.LBB1000_972:
.Ltmp16530:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp16531:
	jmp	.LBB1000_987
.LBB1000_973:
.Ltmp16255:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.491(%rip), %rdx
	callq	*%rax
.Ltmp16256:
	jmp	.LBB1000_987
.LBB1000_974:
.Ltmp16119:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$1, %edi
	movl	$51, %esi
	callq	*%rax
.Ltmp16120:
	jmp	.LBB1000_987
.LBB1000_975:
	leaq	anon.a12f493ba210922c94e5446ac885c35e.1037.llvm.13412714042204560522(%rip), %rax
	movq	%rcx, %rbp
	movq	%rax, 136(%rsp)
.LBB1000_976:
.Ltmp16186:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	136(%rsp), %rdx
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16187:
	jmp	.LBB1000_987
.LBB1000_977:
	leaq	anon.a12f493ba210922c94e5446ac885c35e.1037.llvm.13412714042204560522(%rip), %rax
	movq	%rcx, %rbp
	movq	%rax, 136(%rsp)
.LBB1000_978:
.Ltmp16208:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	136(%rsp), %rdx
	movq	%rbp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp16209:
	jmp	.LBB1000_987
.LBB1000_979:
.Ltmp16133:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp16134:
	jmp	.LBB1000_987
.LBB1000_980:
	addq	$16, %rdi
.Ltmp16440:
	leaq	704(%rsp), %rsi
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.13412714042204560522)
.Ltmp16441:
	jmp	.LBB1000_851
.LBB1000_981:
	movb	$1, %bpl
.Ltmp16242:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp16243:
	jmp	.LBB1000_987
.LBB1000_982:
.Ltmp16461:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.494(%rip), %rdx
	callq	*%rax
.Ltmp16462:
	jmp	.LBB1000_987
.LBB1000_983:
	movb	$1, %bpl
.Ltmp16162:
	movq	core::slice::sort::unstable::ipnsort::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), <[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>::{closure#0}>@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16163:
	jmp	.LBB1000_256
.LBB1000_984:
	movl	$8, %eax
	movq	%rax, 56(%rsp)
.LBB1000_985:
.Ltmp16233:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	56(%rsp), %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16234:
	jmp	.LBB1000_987
.LBB1000_986:
.Ltmp16263:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp16264:
.LBB1000_987:
	ud2
.LBB1000_988:
.Ltmp16442:
	leaq	368(%rsp), %rdi
	movq	%rax, 32(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB1000_1142
.LBB1000_989:
.Ltmp16246:
	movq	704(%rsp), %r14
	movq	%rax, 32(%rsp)
	cmpq	$6, %r14
	jae	.LBB1000_1057
	jmp	.LBB1000_1164
.LBB1000_990:
.Ltmp16158:
	jmp	.LBB1000_1099
.LBB1000_991:
.Ltmp16369:
	jmp	.LBB1000_993
.LBB1000_992:
.Ltmp16375:
.LBB1000_993:
	movq	%rax, 32(%rsp)
	movq	%r14, 616(%rsp)
	jmp	.LBB1000_1122
.LBB1000_994:
.Ltmp16346:
	jmp	.LBB1000_1121
.LBB1000_995:
.Ltmp16152:
	jmp	.LBB1000_1101
.LBB1000_996:
.Ltmp16401:
	movq	%rax, 32(%rsp)
	movb	$1, %al
	movl	%eax, 56(%rsp)
	jmp	.LBB1000_1125
.LBB1000_997:
.Ltmp16353:
	jmp	.LBB1000_1121
.LBB1000_998:
.Ltmp16384:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1125
.LBB1000_999:
.Ltmp16337:
	jmp	.LBB1000_1121
.LBB1000_1000:
.Ltmp16420:
	movl	%r14d, 56(%rsp)
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1138
.LBB1000_1001:
.Ltmp16343:
	jmp	.LBB1000_1008
.LBB1000_1002:
.Ltmp16406:
	movl	%ebp, %ebx
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1135
.LBB1000_1003:
.Ltmp16137:
	jmp	.LBB1000_1154
.LBB1000_1004:
.Ltmp16428:
	cmpq	$0, 1408(%rsp)
	movl	%r14d, 56(%rsp)
	movq	%rax, 32(%rsp)
	jne	.LBB1000_1137
	jmp	.LBB1000_1138
.LBB1000_1005:
.Ltmp16372:
	jmp	.LBB1000_1121
.LBB1000_1006:
.Ltmp16300:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1038
.LBB1000_1007:
.Ltmp16356:
.LBB1000_1008:
	addq	$88, %rbp
	movq	%rax, 32(%rsp)
	movq	%rbp, 616(%rsp)
	jmp	.LBB1000_1122
.LBB1000_1009:
.Ltmp16475:
	leaq	1344(%rsp), %rdi
	movq	%rax, 32(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bl
	jmp	.LBB1000_1029
.LBB1000_1010:
.Ltmp16330:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1141
.LBB1000_1011:
.Ltmp16286:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1041
.LBB1000_1012:
.Ltmp16465:
	jmp	.LBB1000_1106
.LBB1000_1013:
.Ltmp16415:
	addq	$40, %r15
	movq	%rax, 32(%rsp)
	movq	%r15, 184(%rsp)
	jmp	.LBB1000_1021
.LBB1000_1014:
.Ltmp16455:
	jmp	.LBB1000_1106
.LBB1000_1015:
.Ltmp16325:
	movq	%rax, 32(%rsp)
.Ltmp16326:
	leaq	1320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16327:
	jmp	.LBB1000_1141
.LBB1000_1016:
.Ltmp16303:
	jmp	.LBB1000_1025
.LBB1000_1017:
.Ltmp16314:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1046
.LBB1000_1018:
.Ltmp16281:
	movq	%rax, 32(%rsp)
.Ltmp16282:
	leaq	1320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16283:
	jmp	.LBB1000_1041
.LBB1000_1019:
.Ltmp16423:
	addq	$40, %r15
	cmpq	$6, 48(%rsp)
	movq	%rax, 32(%rsp)
	movq	%r15, 184(%rsp)
	jb	.LBB1000_1021
	movq	48(%rsp), %rax
	movq	72(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_1021:
	movb	$1, %bl
.Ltmp16424:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16425:
	jmp	.LBB1000_1139
.LBB1000_1022:
.Ltmp16366:
	jmp	.LBB1000_1121
.LBB1000_1023:
.Ltmp16378:
	jmp	.LBB1000_1052
.LBB1000_1024:
.Ltmp16445:
.LBB1000_1025:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1142
.LBB1000_1026:
.Ltmp16381:
	addq	$40, %r14
	movq	%rax, 32(%rsp)
	movq	%r14, 1232(%rsp)
	jmp	.LBB1000_1122
.LBB1000_1027:
.Ltmp16185:
	jmp	.LBB1000_1074
.LBB1000_1028:
.Ltmp16478:
	movq	%rax, 32(%rsp)
	xorl	%ebx, %ebx
.LBB1000_1029:
	xorl	%ebp, %ebp
	jmp	.LBB1000_1166
.LBB1000_1030:
.Ltmp16450:
	jmp	.LBB1000_1055
.LBB1000_1031:
.Ltmp16289:
	movq	%rax, 32(%rsp)
	movq	%rbp, 184(%rsp)
	jmp	.LBB1000_1037
.LBB1000_1032:
.Ltmp16259:
	movq	%rax, 32(%rsp)
	cmpq	$5, %r14
	ja	.LBB1000_1163
	jmp	.LBB1000_1164
.LBB1000_1033:
.Ltmp16391:
	addq	$40, %r14
	cmpq	$6, 48(%rsp)
	movq	%rax, 32(%rsp)
	movq	%r14, 1232(%rsp)
	jb	.LBB1000_1122
	movq	48(%rsp), %rax
	movq	72(%rsp), %rdi
	movl	$4, %edx
	leaq	-8(,%rax,8), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB1000_1122
.LBB1000_1035:
.Ltmp16294:
	movq	%rax, 32(%rsp)
	movq	%rbp, 184(%rsp)
	cmpq	$6, %rbx
	jb	.LBB1000_1037
	movq	48(%rsp), %rdi
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1000_1037:
.Ltmp16295:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16296:
.LBB1000_1038:
	leaq	1264(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB1000_1039:
	cmpb	$0, 72(%rsp)
	je	.LBB1000_1041
.Ltmp16320:
	leaq	1376(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16321:
	jmp	.LBB1000_1142
.LBB1000_1041:
.Ltmp16318:
	leaq	1792(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp16319:
	jmp	.LBB1000_1142
.LBB1000_1042:
.Ltmp16297:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1043:
.Ltmp16276:
	movq	%rax, 32(%rsp)
.Ltmp16277:
	leaq	1320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16278:
	jmp	.LBB1000_1143
.LBB1000_1044:
.Ltmp16306:
	movq	%rax, 32(%rsp)
.Ltmp16307:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16308:
.Ltmp16310:
	leaq	704(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
.Ltmp16311:
.LBB1000_1046:
.Ltmp16315:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16316:
	jmp	.LBB1000_1039
.LBB1000_1047:
.Ltmp16309:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1048:
.Ltmp16317:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1049:
.Ltmp16146:
	jmp	.LBB1000_1154
.LBB1000_1050:
.Ltmp16322:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1051:
.Ltmp16361:
.LBB1000_1052:
	addq	$88, %r14
	movq	%rax, 32(%rsp)
	movq	%r14, 616(%rsp)
	jmp	.LBB1000_1122
.LBB1000_1053:
.Ltmp16509:
	movl	%ebx, %ebp
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1236
.LBB1000_1054:
.Ltmp16273:
.LBB1000_1055:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1143
.LBB1000_1056:
.Ltmp16249:
	movq	1792(%rsp), %r14
	movq	%rax, 32(%rsp)
	leaq	1800(%rsp), %rax
	movq	%rax, 312(%rsp)
	cmpq	$5, %r14
	jbe	.LBB1000_1164
.LBB1000_1057:
	movq	312(%rsp), %rax
	movq	(%rax), %r13
	jmp	.LBB1000_1163
.LBB1000_1058:
.Ltmp16493:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1146
.LBB1000_1059:
.Ltmp16340:
	jmp	.LBB1000_1121
.LBB1000_1060:
.Ltmp16460:
	movq	1432(%rsp), %r15
	jmp	.LBB1000_1116
.LBB1000_1061:
.Ltmp16498:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1165
.LBB1000_1062:
.Ltmp16481:
	movl	%ebp, %ebx
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1190
.LBB1000_1063:
.Ltmp16436:
	movq	%rax, 32(%rsp)
	testq	%r14, %r14
	je	.LBB1000_1067
	negq	%r14
	addq	$160, %rbx
.LBB1000_1065:
.Ltmp16437:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16438:
	addq	$160, %rbx
	decq	%r14
	jne	.LBB1000_1065
.LBB1000_1067:
	cmpq	$0, 568(%rsp)
	je	.LBB1000_1142
	movq	568(%rsp), %rax
	movq	1208(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	jmp	.LBB1000_1142
.LBB1000_1069:
.Ltmp16439:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1070:
.Ltmp16161:
	leaq	704(%rsp), %rdi
	movq	%r12, 1824(%rsp)
	movq	%r15, 1816(%rsp)
	movw	%bp, 1840(%rsp)
	movq	%rax, 32(%rsp)
	movq	%r14, 1848(%rsp)
	callq	core::ptr::drop_glue::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>
	leaq	1792(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>>
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
	jmp	.LBB1000_1174
.LBB1000_1071:
.Ltmp16180:
	jmp	.LBB1000_1074
.LBB1000_1072:
.Ltmp16121:
	jmp	.LBB1000_1149
.LBB1000_1073:
.Ltmp16173:
.LBB1000_1074:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1189
.LBB1000_1075:
.Ltmp16254:
	movq	1472(%rsp), %r14
	jmp	.LBB1000_1160
.LBB1000_1076:
.Ltmp16532:
	movq	%rax, 32(%rsp)
.Ltmp16533:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16534:
	jmp	.LBB1000_1264
.LBB1000_1077:
.Ltmp16535:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1078:
.Ltmp16526:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1261
.LBB1000_1079:
.Ltmp16512:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1237
.LBB1000_1080:
.Ltmp16217:
	movq	%rax, %rbx
.Ltmp16218:
	leaq	1792(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::compile::ExprProgram>
.Ltmp16219:
	jmp	.LBB1000_1113
.LBB1000_1081:
.Ltmp16220:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1082:
.Ltmp16226:
	movq	%rax, %rbx
.Ltmp16227:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::compile::ExprProgram>
.Ltmp16228:
	jmp	.LBB1000_1113
.LBB1000_1083:
.Ltmp16229:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1084:
.Ltmp16204:
	movq	%rax, %rbx
.Ltmp16205:
	leaq	720(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::compile::ExprProgram>
.Ltmp16206:
	jmp	.LBB1000_1112
.LBB1000_1086:
.Ltmp16207:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1087:
.Ltmp16195:
	movq	%rax, %rbx
.Ltmp16196:
	leaq	1792(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::vm::compile::ExprProgram>
.Ltmp16197:
	jmp	.LBB1000_1112
.LBB1000_1089:
.Ltmp16198:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1090:
.Ltmp16503:
	movq	%rax, 32(%rsp)
	testq	%r15, %r15
	je	.LBB1000_1094
	negq	%r15
	addq	$24, %r14
	.p2align	4
.LBB1000_1092:
.Ltmp16504:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16505:
	addq	$24, %r14
	decq	%r15
	jne	.LBB1000_1092
.LBB1000_1094:
	movq	584(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1000_1096
	movb	$1, %bl
	jmp	.LBB1000_1190
.LBB1000_1096:
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
	movb	$1, %bl
	jmp	.LBB1000_1190
.LBB1000_1097:
.Ltmp16506:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1098:
.Ltmp16149:
.LBB1000_1099:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1158
.LBB1000_1100:
.Ltmp16155:
.LBB1000_1101:
	cmpq	$6, 96(%rsp)
	movq	%rax, 32(%rsp)
	jb	.LBB1000_1158
	movq	96(%rsp), %rax
	movl	$4, %edx
	movq	%rbx, %rdi
	leaq	-8(,%rax,8), %rsi
	jmp	.LBB1000_1157
.LBB1000_1103:
.Ltmp16223:
	jmp	.LBB1000_1109
.LBB1000_1104:
.Ltmp16201:
	jmp	.LBB1000_1111
.LBB1000_1105:
.Ltmp16488:
.LBB1000_1106:
	movq	%rax, 32(%rsp)
	jmp	.LBB1000_1119
.LBB1000_1107:
.Ltmp16132:
	movq	248(%rsp), %rdi
	movq	344(%rsp), %rsi
	movl	$8, %edx
	movq	%rax, 32(%rsp)
	callq	__rustc::__rust_dealloc
	jmp	.LBB1000_1168
.LBB1000_1108:
.Ltmp16210:
.LBB1000_1109:
	movq	%rax, %rbx
	jmp	.LBB1000_1113
.LBB1000_1110:
.Ltmp16188:
.LBB1000_1111:
	movq	%rax, %rbx
.LBB1000_1112:
	movq	%r14, %r15
.LBB1000_1113:
	movq	%r15, 1488(%rsp)
.Ltmp16230:
	leaq	1472(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16231:
	movq	%rbx, %r15
	jmp	.LBB1000_1176
.LBB1000_1114:
.Ltmp16232:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1115:
.Ltmp16468:
.LBB1000_1116:
	movq	%rax, 32(%rsp)
	cmpq	$6, %r15
	jb	.LBB1000_1119
	movq	1440(%rsp), %rdi
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1000_1119:
	leaq	1792(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB1000_1143
.LBB1000_1120:
.Ltmp16396:
.LBB1000_1121:
	movq	%rax, 32(%rsp)
.LBB1000_1122:
	cmpq	$0, 320(%rsp)
	je	.LBB1000_1124
	movq	320(%rsp), %rsi
	movq	312(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_1124:
	movb	$1, %al
	movl	%eax, 56(%rsp)
.Ltmp16397:
	leaq	608(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp16398:
.LBB1000_1125:
	cmpq	$0, 680(%rsp)
	je	.LBB1000_1127
	movq	680(%rsp), %rax
	movq	328(%rsp), %rdi
	movl	$8, %edx
	shlq	$3, %rax
	leaq	(%rax,%rax,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_1127:
	movq	792(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_1130
	lock		decq	(%rax)
	jne	.LBB1000_1130
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	792(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1000_1130:
	movq	824(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_1133
	lock		decq	(%rax)
	jne	.LBB1000_1133
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	824(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB1000_1133:
.Ltmp16402:
	leaq	1752(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16403:
	xorl	%ebx, %ebx
.LBB1000_1135:
	movq	64(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1000_1137
	#MEMBARRIER
.Ltmp16407:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1256(%rsp), %rdi
	callq	*%rax
.Ltmp16408:
.LBB1000_1137:
.Ltmp16429:
	leaq	1224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp16430:
.LBB1000_1138:
	cmpb	$0, 56(%rsp)
	je	.LBB1000_1140
.LBB1000_1139:
	leaq	264(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB1000_1140:
	testb	%bl, %bl
	je	.LBB1000_1142
.LBB1000_1141:
.Ltmp16431:
	leaq	2416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp16432:
.LBB1000_1142:
.Ltmp16446:
	leaq	2448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
.Ltmp16447:
.LBB1000_1143:
	movq	336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_1146
	lock		decq	(%rax)
	jne	.LBB1000_1146
	#MEMBARRIER
.Ltmp16489:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	336(%rsp), %rdi
	callq	*%rax
.Ltmp16490:
.LBB1000_1146:
	movb	$1, %bl
.Ltmp16494:
	leaq	1472(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp16495:
	movb	$1, %bpl
	jmp	.LBB1000_1166
.LBB1000_1147:
.Ltmp16433:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1148:
.Ltmp16118:
.LBB1000_1149:
	movq	%rax, 32(%rsp)
.Ltmp16122:
	leaq	1648(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16123:
	jmp	.LBB1000_1264
.LBB1000_1150:
.Ltmp16140:
	movq	%rax, 32(%rsp)
	movq	%r12, (%rbx)
	jmp	.LBB1000_1155
.LBB1000_1151:
.Ltmp16112:
	movq	%rax, 32(%rsp)
.Ltmp16113:
	leaq	704(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)
.Ltmp16114:
	jmp	.LBB1000_1264
.LBB1000_1152:
.Ltmp16115:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1153:
.Ltmp16143:
.LBB1000_1154:
	movq	%rax, 32(%rsp)
.LBB1000_1155:
	movq	704(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1000_1158
	movq	712(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
.LBB1000_1157:
	callq	__rustc::__rust_dealloc
.LBB1000_1158:
	leaq	640(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>
	jmp	.LBB1000_1174
.LBB1000_1159:
.Ltmp16262:
.LBB1000_1160:
	movq	%rax, 32(%rsp)
	cmpq	$6, %r14
	jb	.LBB1000_1164
	movq	1480(%rsp), %r13
.LBB1000_1163:
	leaq	-8(,%r14,8), %rsi
	movl	$4, %edx
	movq	%r13, %rdi
	callq	__rustc::__rust_dealloc
.LBB1000_1164:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB1000_1165:
	movb	$1, %bl
	movb	$1, %bpl
.LBB1000_1166:
.Ltmp16499:
	leaq	584(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp16500:
	jmp	.LBB1000_1190
.LBB1000_1167:
.Ltmp16517:
	movq	%rax, 32(%rsp)
.LBB1000_1168:
	movb	$1, %bpl
	movb	$1, %r12b
	jmp	.LBB1000_1234
.LBB1000_1169:
.Ltmp16166:
	movq	%rax, %r14
	cmpq	$6, %r15
	jb	.LBB1000_1171
	movq	32(%rsp), %rdi
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1000_1171:
	movq	%r14, 32(%rsp)
	testq	%rbx, %rbx
	je	.LBB1000_1173
	movq	96(%rsp), %rdi
	shlq	$3, %rbx
	movl	$8, %edx
	movq	%rbx, %rsi
	callq	__rustc::__rust_dealloc
.LBB1000_1173:
	leaq	1472(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>, purrdf_sparql_eval::modifier::eval_group<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>>
.LBB1000_1174:
	movb	$1, %r12b
	movb	$1, %bpl
	jmp	.LBB1000_1224
.LBB1000_1175:
.Ltmp16235:
	movq	%rax, %r15
.LBB1000_1176:
	testq	%r13, %r13
	je	.LBB1000_1180
	movq	88(%rsp), %rbx
	xorl	%r14d, %r14d
	.p2align	4
.LBB1000_1178:
.Ltmp16236:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16237:
	incq	%r14
	addq	$24, %rbx
	cmpq	%r14, %r13
	jne	.LBB1000_1178
.LBB1000_1180:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	64(%rsp), %rsi
	xorl	%ecx, %ecx
	movabsq	$9223372036854775807, %rdx
	cmpq	%rsi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rsi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_1182
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_1182:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_1188
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_1182
	movq	64(%rsp), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rsi, %rcx
	negq	%rcx
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
.LBB1000_1185:
	cmpq	%rax, %rcx
	jge	.LBB1000_1187
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1000_1185
.LBB1000_1187:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_1188:
	movq	free@GOTPCREL(%rip), %rax
	movq	88(%rsp), %rdi
	movq	%r15, 32(%rsp)
	callq	*%rax
.LBB1000_1189:
	movb	$1, %bl
	movb	$1, %bpl
.LBB1000_1190:
	movl	%ebx, 160(%rsp)
	movq	536(%rsp), %rbx
	movq	544(%rsp), %r14
	movl	%ebp, 96(%rsp)
	testq	%r14, %r14
	jne	.LBB1000_1193
.LBB1000_1191:
	movq	528(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1000_1215
	movl	160(%rsp), %ebp
	movl	96(%rsp), %r12d
	jmp	.LBB1000_1224
.LBB1000_1193:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r15
	xorl	%r12d, %r12d
	jmp	.LBB1000_1197
	.p2align	4
.LBB1000_1194:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_1195:
	callq	*%r15
.LBB1000_1196:
	incq	%r12
	cmpq	%r14, %r12
	je	.LBB1000_1191
.LBB1000_1197:
	leaq	(%r12,%r12,8), %rax
	leaq	(%rbx,%rax,8), %r13
	movq	(%rbx,%rax,8), %rax
	cmpq	$6, %rax
	jb	.LBB1000_1207
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_1200
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_1200:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_1206
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_1200
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
.LBB1000_1203:
	cmpq	%rax, %rdx
	jge	.LBB1000_1205
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB1000_1203
.LBB1000_1205:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_1206:
	callq	*%r15
.LBB1000_1207:
	movq	48(%r13), %rcx
	testq	%rcx, %rcx
	je	.LBB1000_1196
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_1210
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_1210:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_1195
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_1210
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
.LBB1000_1213:
	cmpq	%rax, %rdx
	jge	.LBB1000_1194
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB1000_1213
	jmp	.LBB1000_1194
.LBB1000_1215:
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	movl	160(%rsp), %ebp
	movl	96(%rsp), %r12d
	leaq	(%rax,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_1217
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_1217:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_1223
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_1217
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
.LBB1000_1220:
	cmpq	%rax, %rdx
	jge	.LBB1000_1222
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_1220
.LBB1000_1222:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_1223:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.LBB1000_1224:
	cmpq	$0, 352(%rsp)
	je	.LBB1000_1234
	movq	344(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_1227
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_1227:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_1233
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_1227
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
.LBB1000_1230:
	cmpq	%rax, %rcx
	jge	.LBB1000_1232
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1000_1230
.LBB1000_1232:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_1233:
	movq	free@GOTPCREL(%rip), %rax
	movq	248(%rsp), %rdi
	callq	*%rax
.LBB1000_1234:
	movq	232(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1000_1236
	#MEMBARRIER
.Ltmp16518:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	232(%rsp), %rdi
	callq	*%rax
.Ltmp16519:
.LBB1000_1236:
.Ltmp16520:
	leaq	1168(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16521:
.LBB1000_1237:
	testb	%bpl, %bpl
	je	.LBB1000_1261
	movq	2312(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1000_1239
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	2320(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_1243
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_1243:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_1249
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_1243
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
.LBB1000_1246:
	cmpq	%rax, %rdx
	jge	.LBB1000_1248
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_1246
.LBB1000_1248:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_1249:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	2240(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB1000_1250
.LBB1000_1240:
	movq	2336(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1000_1259
	jmp	.LBB1000_1261
.LBB1000_1239:
	movq	2240(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB1000_1240
.LBB1000_1250:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	2248(%rsp), %rdi
	cmpq	%rsi, %rcx
	cmovaeq	%rsi, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1000_1252
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1000_1252:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1000_1258
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1000_1252
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
.LBB1000_1255:
	cmpq	%rax, %rdx
	jge	.LBB1000_1257
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1000_1255
.LBB1000_1257:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1000_1258:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	2336(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1000_1261
.LBB1000_1259:
	lock		decq	(%rax)
	jne	.LBB1000_1261
	leaq	2336(%rsp), %rdi
	#MEMBARRIER
.Ltmp16522:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp16523:
.LBB1000_1261:
	testb	%r12b, %r12b
	je	.LBB1000_1264
	movq	256(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB1000_1264
	#MEMBARRIER
.Ltmp16527:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	1424(%rsp), %rdi
	callq	*%rax
.Ltmp16528:
.LBB1000_1264:
	movq	32(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB1000_1265:
.Ltmp16238:
	subq	%r14, %r12
	je	.LBB1000_1269
	addq	$24, %rbx
	.p2align	4
.LBB1000_1267:
.Ltmp16239:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp16240:
	addq	$24, %rbx
	decq	%r12
	jne	.LBB1000_1267
.LBB1000_1269:
	movq	88(%rsp), %rdi
	movq	64(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1270:
.Ltmp16241:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1000_1271:
.Ltmp16529:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1000:
purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin1003:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception663
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
	subq	$680, %rsp
	.cfi_def_cfa_offset 736
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	456(%rsp), %rdi
	movq	%r9, %r14
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbp
	callq	*%rax
	movq	1136(%r14), %rax
	movq	%rbx, 232(%rsp)
	testq	%rax, %rax
	je	.LBB1003_1
	movq	1128(%r14), %rcx
	movq	%rax, 72(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%r15, %r12
	shlq	$4, %r12
	movq	%r14, 8(%rsp)
	movq	%r15, 48(%rsp)
	movq	%r13, 16(%rsp)
	movq	%rbp, 64(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 240(%rsp)
	addq	%rcx, %rax
	movq	%rax, 24(%rsp)
	testq	%r15, %r15
	je	.LBB1003_41
	movq	bcmp@GOTPCREL(%rip), %rbx
	movq	%rcx, %rax
.LBB1003_4:
	movq	24(%rax), %rbp
	movq	32(%rax), %r14
	addq	$48, %rax
	movq	%rax, 32(%rsp)
	leaq	16(%rbp), %r15
	jmp	.LBB1003_5
	.p2align	4
.LBB1003_38:
	xorq	%r14, %rax
	xorq	%rbp, %rdi
	orq	%rax, %rdi
	je	.LBB1003_7
.LBB1003_39:
	addq	$16, %r13
	cmpq	%r12, %r13
	je	.LBB1003_40
.LBB1003_5:
	movq	(%r13), %rdi
	movq	8(%r13), %rax
	cmpq	%rbp, %rdi
	sete	%cl
	cmpq	%r14, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB1003_38
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%rbx
	testl	%eax, %eax
	jne	.LBB1003_39
.LBB1003_7:
	movq	32(%rsp), %rax
	movq	16(%rsp), %r13
	cmpq	24(%rsp), %rax
	jne	.LBB1003_4
	movq	8(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 272(%rsp)
	movq	$0, 288(%rsp)
	movq	$8, 296(%rsp)
	jmp	.LBB1003_9
.LBB1003_40:
	movq	48(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB1003_46
	jmp	.LBB1003_43
.LBB1003_1:
.Ltmp16605:
	leaq	560(%rsp), %rdi
	movq	%rbp, %rsi
	movq	%r14, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp16606:
	cmpl	$1, 560(%rsp)
	jne	.LBB1003_96
.LBB1003_15:
	vmovups	608(%rsp), %zmm1
	vmovups	576(%rsp), %zmm0
	movq	528(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB1003_25
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	536(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_18
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_18:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_24
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_18
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
.LBB1003_21:
	cmpq	%rax, %rsi
	jge	.LBB1003_23
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_21
.LBB1003_23:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_24:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1003_25:
	movq	456(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB1003_35
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	464(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_28
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_28:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_34
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_28
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
.LBB1003_31:
	cmpq	%rax, %rsi
	jge	.LBB1003_33
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_31
.LBB1003_33:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_34:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1003_35:
	movq	552(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1003_202
	lock		decq	(%rax)
	jne	.LBB1003_202
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	552(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB1003_202
.LBB1003_41:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, 32(%rsp)
	lock		incq	(%rbp)
	jle	.LBB1003_43
.LBB1003_46:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 272(%rsp)
	movq	%r14, 280(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1003_58
	movq	%rax, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$64, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	$64, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1003_49
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB1003_49:
	movq	8(%rsp), %rax
	addq	$1120, %rax
	movq	%rax, 256(%rsp)
	.p2align	4
.LBB1003_50:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_56
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_50
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
.LBB1003_53:
	cmpq	%rax, %rdx
	jle	.LBB1003_55
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_53
.LBB1003_55:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_56:
	movq	32(%rsp), %rax
	movq	%rbp, (%rsi)
	movq	$4, 112(%rsp)
	movq	%rsi, 120(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 128(%rsp)
	cmpq	24(%rsp), %rax
	je	.LBB1003_57
	movl	$1, %r14d
	testq	%r15, %r15
	je	.LBB1003_66
	movq	32(%rsp), %rbx
	jmp	.LBB1003_61
	.p2align	4
.LBB1003_88:
	movq	120(%rsp), %rsi
.LBB1003_77:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 128(%rsp)
	cmpq	24(%rsp), %rbx
	je	.LBB1003_78
.LBB1003_61:
	movq	%rsi, 248(%rsp)
.LBB1003_62:
	movq	24(%rbx), %r13
	leaq	48(%rbx), %rax
	movq	32(%rbx), %rbp
	movq	16(%rsp), %rbx
	movq	%rax, 32(%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB1003_63
	.p2align	4
.LBB1003_73:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB1003_65
.LBB1003_74:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB1003_75
.LBB1003_63:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB1003_73
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB1003_74
.LBB1003_65:
	movq	32(%rsp), %rbx
	cmpq	24(%rsp), %rbx
	jne	.LBB1003_62
	jmp	.LBB1003_78
	.p2align	4
.LBB1003_75:
	lock		incq	(%r13)
	movq	248(%rsp), %rsi
	jle	.LBB1003_43
	movq	32(%rsp), %rbx
	movq	%r13, 272(%rsp)
	movq	%rbp, 280(%rsp)
	cmpq	112(%rsp), %r14
	jne	.LBB1003_77
.Ltmp16570:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	112(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp16571:
	jmp	.LBB1003_88
.LBB1003_57:
	movl	$1, %r14d
.LBB1003_78:
	movq	8(%rsp), %rbp
	movq	120(%rsp), %rbx
	movq	112(%rsp), %rax
	movq	<alloc::raw_vec::RawVec<core::option::Option<(alloc::string::String, alloc::string::String)>>>::grow_one@GOTPCREL(%rip), %r13
	shlq	$4, %r14
	movq	1136(%rbp), %r15
	movq	%rbx, 272(%rsp)
	movq	%rax, 288(%rsp)
	addq	%rbx, %r14
	addq	$16, %rbx
	movq	%r14, 296(%rsp)
	movq	%r15, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB1003_79
	.p2align	4
.LBB1003_81:
	vmovdqu	112(%rsp), %ymm0
	vmovups	128(%rsp), %ymm1
	movq	1128(%rbp), %rcx
	leaq	-16(%rbx), %rax
	incq	%r15
	addq	$16, %rbx
	addq	$16, %rax
	vmovups	%ymm1, 16(%rcx,%r12)
	vmovdqu	%ymm0, (%rcx,%r12)
	addq	$48, %r12
	movq	%r15, 1136(%rbp)
	cmpq	%r14, %rax
	je	.LBB1003_82
.LBB1003_79:
	vmovups	-16(%rbx), %xmm0
	movq	256(%rsp), %rax
	vmovups	%xmm0, 136(%rsp)
	movq	$2, 112(%rsp)
	movb	$0, 152(%rsp)
	cmpq	(%rax), %r15
	jne	.LBB1003_81
.Ltmp16581:
	movq	256(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp16582:
	jmp	.LBB1003_81
.LBB1003_82:
	movq	16(%rsp), %r13
.LBB1003_9:
	movq	%r14, 280(%rsp)
.Ltmp16589:
	leaq	272(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp16590:
	movq	232(%rsp), %rbx
	movq	48(%rsp), %r15
	movq	72(%rsp), %r14
.Ltmp16591:
	movq	64(%rsp), %rcx
	leaq	112(%rsp), %rdi
	movq	%rbp, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp16592:
	movq	1136(%rbp), %rsi
	subq	%r14, %rsi
	jb	.LBB1003_13
	movq	240(%rsp), %rdi
	addq	1128(%rbp), %rdi
	movq	%r14, 1136(%rbp)
.Ltmp16593:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp16594:
.LBB1003_13:
	vmovups	160(%rsp), %zmm1
	vmovdqu64	112(%rsp), %zmm0
	vmovups	%zmm1, 608(%rsp)
	vmovdqu64	%zmm0, 560(%rsp)
	cmpl	$1, 560(%rsp)
	je	.LBB1003_15
.LBB1003_96:
	leaq	568(%rsp), %rcx
.Ltmp16607:
	leaq	112(%rsp), %rdi
	leaq	456(%rsp), %rsi
	xorl	%edx, %edx
	movq	%rdi, %rbp
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp16608:
	cmpq	$-1, 112(%rsp)
	je	.LBB1003_207
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 416(%rsp)
.Ltmp16609:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16610:
	movq	%rax, 408(%rsp)
	movq	%rax, 376(%rsp)
	movq	32(%rax), %rcx
	movq	%rcx, %rbx
	shlq	$4, %rbx
	movq	%rcx, 8(%rsp)
	movq	%rbx, 56(%rsp)
	testq	%rcx, %rcx
	je	.LBB1003_100
	movq	24(%rax), %r14
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1003_121
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	movabsq	$9223372036854775807, %rdx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	addq	%rbx, %rax
	cmovbq	%rcx, %rax
	cmpq	%rdx, %rbx
	movq	%rdx, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmovbq	%rbx, %rcx
	addq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1003_112
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_112:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_118
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_112
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%rbx, (%rdx)
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
.LBB1003_115:
	cmpq	%rax, %rdx
	jle	.LBB1003_117
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_115
.LBB1003_117:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_118:
	movq	440(%rsp), %rbx
	movq	8(%rsp), %r12
	xorl	%r15d, %r15d
	addq	$16, %rbx
	.p2align	4
.LBB1003_119:
	leaq	(%r14,%r15), %rsi
.Ltmp16612:
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.13412714042204560522)
.Ltmp16613:
	movq	%rax, (%r13,%r15)
	movq	%rdx, 8(%r13,%r15)
	addq	$16, %r15
	decq	%r12
	jne	.LBB1003_119
	movq	432(%rsp), %rax
	movq	%r13, 64(%rsp)
	testq	%rax, %rax
	je	.LBB1003_102
.LBB1003_123:
	movq	424(%rsp), %rcx
	movq	malloc@GOTPCREL(%rip), %r14
	movq	%rax, 48(%rsp)
	leaq	(,%rax,8), %rax
	leaq	(%rax,%rax,4), %rbx
	movq	%rbx, %rdi
	movq	%rcx, 240(%rsp)
	callq	*%r14
	movq	%rax, 72(%rsp)
	testq	%rax, %rax
	je	.LBB1003_138
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	movq	%rbp, %r10
	leaq	(%rbx,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%rbx, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%rbx, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmovbq	%r8, %rdi
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1003_126
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_126:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_132
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_126
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
.LBB1003_129:
	cmpq	%rax, %rdx
	jle	.LBB1003_131
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_129
.LBB1003_131:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_132:
	movq	56(%rsp), %rbp
	movq	48(%rsp), %rax
	movq	72(%rsp), %rsi
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %r11
	movl	$2, %ecx
	leaq	120(%rsp), %r14
	movq	$0, 16(%rsp)
	vmovd	%ecx, %xmm0
	vmovdqa	%xmm0, 32(%rsp)
	leaq	(%r13,%rbp), %rdx
	negq	%rbp
	movq	%rax, 384(%rsp)
	movq	%rsi, 392(%rsp)
	movq	%rdx, 24(%rsp)
	jmp	.LBB1003_133
	.p2align	4
.LBB1003_170:
	movq	%rbx, (%r12)
.LBB1003_171:
	vmovdqu	112(%rsp), %ymm0
	movq	144(%rsp), %rax
	movq	72(%rsp), %rcx
	movq	248(%rsp), %rdx
	movq	64(%rsp), %r13
	movq	%rax, 32(%rcx,%rdx,8)
	movq	%rax, 304(%rsp)
	movq	48(%rsp), %rax
	vmovdqu	%ymm0, (%rcx,%rdx,8)
	movq	16(%rsp), %rcx
	vmovdqu	%ymm0, 272(%rsp)
	incq	%rcx
	movq	%rcx, 16(%rsp)
	cmpq	%rax, %rcx
	je	.LBB1003_172
.LBB1003_133:
	cmpq	$5, 8(%rsp)
	movl	$1, %ebx
	movl	$4, %eax
	movq	$1, 112(%rsp)
	movq	%r14, %rcx
	movq	%r10, %r12
	jae	.LBB1003_134
.LBB1003_136:
	movq	16(%rsp), %rdx
	movq	240(%rsp), %rdi
	leaq	(%rdx,%rdx,4), %rsi
	leaq	-1(%rbx), %rdx
	leaq	(%rdi,%rsi,8), %r15
	movq	%rsi, 248(%rsp)
	cmpq	%rax, %rdx
	jae	.LBB1003_137
	movq	(%r15), %rsi
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1003_140
	movq	16(%r15), %rsi
	movq	8(%r15), %rdx
	decq	%rsi
	jmp	.LBB1003_142
	.p2align	4
.LBB1003_137:
	movq	%rbx, %rax
	movq	%rax, (%r12)
	cmpq	24(%rsp), %r13
	jne	.LBB1003_149
	jmp	.LBB1003_171
	.p2align	4
.LBB1003_140:
	leaq	8(%r15), %rdx
.LBB1003_142:
	incq	%rax
	xorl	%r8d, %r8d
	movq	%r13, %r9
	jmp	.LBB1003_143
	.p2align	4
.LBB1003_160:
	vmovq	(%rdx,%rdi,8), %xmm0
.LBB1003_161:
	vmovq	%xmm0, -8(%rcx,%rbx,8)
	addq	$16, %r9
	incq	%rbx
	addq	$-16, %r8
	cmpq	%rbx, %rax
	je	.LBB1003_147
.LBB1003_143:
	cmpq	%r8, %rbp
	je	.LBB1003_170
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB1003_161
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jb	.LBB1003_160
	jmp	.LBB1003_146
	.p2align	4
.LBB1003_147:
	subq	%r8, %r13
	movq	%rax, (%r12)
	cmpq	24(%rsp), %r13
	je	.LBB1003_171
.LBB1003_149:
	movq	(%r15), %rbx
	decq	%rbx
	cmpq	$5, %rbx
	jb	.LBB1003_150
	movq	16(%r15), %rbx
	movq	8(%r15), %r15
	decq	%rbx
	jmp	.LBB1003_152
	.p2align	4
.LBB1003_150:
	addq	$8, %r15
	jmp	.LBB1003_152
.LBB1003_157:
.Ltmp16627:
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r10, %rdi
	vzeroupper
	callq	*%r11
.Ltmp16628:
	cmpq	$6, 112(%rsp)
	movq	120(%rsp), %rax
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %r11
	leaq	112(%rsp), %r10
	leaq	128(%rsp), %rdx
	movq	%r10, %rcx
	cmovbq	%r14, %rax
	cmovaeq	%rdx, %rcx
	jmp	.LBB1003_159
	.p2align	4
.LBB1003_152:
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%r13)
	vmovdqa	%xmm0, 256(%rsp)
	jne	.LBB1003_156
	movq	8(%r13), %rdi
	cmpq	%rbx, %rdi
	jae	.LBB1003_154
	vmovq	(%r15,%rdi,8), %xmm0
	vmovdqa	%xmm0, 256(%rsp)
.LBB1003_156:
	movq	112(%rsp), %rsi
	movq	120(%rsp), %rax
	xorl	%edx, %edx
	leaq	128(%rsp), %rdi
	movq	%r10, %rcx
	decq	%rsi
	cmpq	$5, %rsi
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%r14, %rax
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	112(%rsp,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rsi, %rdx
	je	.LBB1003_157
.LBB1003_159:
	vmovdqa	256(%rsp), %xmm0
	addq	$16, %r13
	vmovq	%xmm0, -8(%rax,%r12,8)
	incq	%r12
	movq	%r12, (%rcx)
	cmpq	24(%rsp), %r13
	jne	.LBB1003_152
	jmp	.LBB1003_171
.LBB1003_134:
.Ltmp16618:
	movq	8(%rsp), %rdx
	movl	$1, %ecx
	movq	%r10, %rdi
	xorl	%esi, %esi
	movq	%r10, %r15
	movq	%r11, %r12
	vzeroupper
	callq	*%r11
.Ltmp16619:
	movq	112(%rsp), %rax
	xorl	%edx, %edx
	movl	$4, %ecx
	leaq	128(%rsp), %rsi
	movq	%r15, %rdi
	movq	%r15, %r10
	movq	%r12, %r11
	decq	%rax
	cmpq	$5, %rax
	cmovbq	%rcx, %rax
	movq	120(%rsp), %rcx
	setae	%dl
	cmovaeq	%rsi, %rdi
	movq	%rdi, %r12
	cmovbq	%r14, %rcx
	shll	$4, %edx
	movq	112(%rsp,%rdx), %rbx
	jmp	.LBB1003_136
.LBB1003_207:
.Ltmp16639:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp16640:
	vmovups	496(%rsp), %zmm1
	vmovups	456(%rsp), %zmm0
	movq	%rax, 104(%rsp)
	movq	$0, 80(%rsp)
	movq	$8, 88(%rsp)
	movq	$0, 96(%rsp)
	vmovups	%zmm1, 152(%rsp)
	vmovups	%zmm0, 112(%rsp)
	cmpq	$-1, 112(%rsp)
	je	.LBB1003_210
	leaq	272(%rsp), %rdi
	leaq	80(%rsp), %rsi
	leaq	456(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB1003_212
	jmp	.LBB1003_221
.LBB1003_66:
	movq	32(%rsp), %rax
	movl	$24, %ebx
	leaq	112(%rsp), %rbp
	jmp	.LBB1003_67
	.p2align	4
.LBB1003_71:
	addq	$48, %r12
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %rax
	movq	%r14, 128(%rsp)
	cmpq	24(%rsp), %r12
	je	.LBB1003_78
.LBB1003_67:
	movq	24(%rax), %r13
	movq	32(%rax), %r15
	lock		incq	(%r13)
	jle	.LBB1003_43
	movq	%r13, 272(%rsp)
	movq	%rax, %r12
	movq	%r15, 280(%rsp)
	cmpq	112(%rsp), %r14
	jne	.LBB1003_71
.Ltmp16573:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp16574:
	movq	120(%rsp), %rsi
	jmp	.LBB1003_71
.LBB1003_100:
	movl	$8, %r13d
	movq	432(%rsp), %rax
	movq	%r13, 64(%rsp)
	testq	%rax, %rax
	jne	.LBB1003_123
.LBB1003_102:
	movq	$0, 384(%rsp)
	movq	$8, 392(%rsp)
.LBB1003_172:
	vmovdqu64	456(%rsp), %zmm0
	vmovups	496(%rsp), %zmm1
	movq	%rax, 400(%rsp)
	movq	%rax, 96(%rsp)
	movq	376(%rsp), %rcx
	movq	384(%rsp), %rax
	movq	392(%rsp), %rdx
	movq	%rax, 80(%rsp)
	movq	%rdx, 88(%rsp)
	movq	%rcx, 104(%rsp)
	vmovdqu64	%zmm0, 112(%rsp)
	vmovups	%zmm1, 152(%rsp)
	cmpq	$-1, 112(%rsp)
	je	.LBB1003_177
	leaq	272(%rsp), %rdi
	leaq	80(%rsp), %rsi
	leaq	456(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	232(%rsp), %rbx
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB1003_179
.LBB1003_188:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB1003_189
	jmp	.LBB1003_191
.LBB1003_177:
	movq	88(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	96(%rsp), %rdx
	movq	%rcx, 288(%rsp)
	movq	104(%rsp), %rcx
	movq	%rax, 280(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rcx, 304(%rsp)
	movq	$-1, 272(%rsp)
	movq	232(%rsp), %rbx
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1003_188
.LBB1003_179:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	192(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_181
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_181:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_187
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_181
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
.LBB1003_184:
	cmpq	%rax, %rsi
	jge	.LBB1003_186
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_184
.LBB1003_186:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_187:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1003_191
.LBB1003_189:
	lock		decq	(%rax)
	jne	.LBB1003_191
	xorl	%ebp, %ebp
	leaq	208(%rsp), %rdi
	#MEMBARRIER
.Ltmp16632:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp16633:
.LBB1003_191:
	vmovups	304(%rsp), %zmm1
	vmovups	272(%rsp), %zmm0
	cmpq	$0, 8(%rsp)
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	je	.LBB1003_201
	movq	56(%rsp), %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %rdi
	cmovaeq	%rcx, %rdi
	xorl	%edx, %edx
	cmpq	%rdi, %rax
	movq	%rdi, %rsi
	setns	%dl
	addq	%rcx, %rdx
	subq	%rdi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_194
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_194:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_200
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_194
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
.LBB1003_197:
	cmpq	%rax, %rdx
	jge	.LBB1003_199
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1003_197
.LBB1003_199:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_200:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
.LBB1003_201:
	leaq	416(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB1003_202
.LBB1003_210:
	movq	88(%rsp), %rcx
	movq	80(%rsp), %rax
	movq	96(%rsp), %rdx
	movq	%rcx, 288(%rsp)
	movq	104(%rsp), %rcx
	movq	%rax, 280(%rsp)
	movq	%rdx, 296(%rsp)
	movq	%rcx, 304(%rsp)
	movq	$-1, 272(%rsp)
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1003_221
.LBB1003_212:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	192(%rsp), %rdi
	cmpq	%rdx, %rcx
	cmovaeq	%rdx, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdx, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1003_214
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1003_214:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1003_220
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1003_214
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
.LBB1003_217:
	cmpq	%rax, %rsi
	jge	.LBB1003_219
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1003_217
.LBB1003_219:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1003_220:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB1003_221:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB1003_224
	lock		decq	(%rax)
	jne	.LBB1003_224
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB1003_224:
	vmovups	304(%rsp), %zmm1
	vmovups	272(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB1003_202:
	movq	%rbx, %rax
	addq	$680, %rsp
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
.LBB1003_154:
	.cfi_def_cfa_offset 736
.Ltmp16624:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.717(%rip), %rdx
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp16625:
	jmp	.LBB1003_43
.LBB1003_146:
.Ltmp16621:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.717(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp16622:
	jmp	.LBB1003_43
.LBB1003_58:
.Ltmp16599:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp16600:
	jmp	.LBB1003_43
.LBB1003_121:
.Ltmp16615:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp16616:
	jmp	.LBB1003_43
.LBB1003_138:
	movb	$1, %bpl
.Ltmp16630:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp16631:
.LBB1003_43:
	ud2
.LBB1003_164:
.Ltmp16620:
	jmp	.LBB1003_166
.LBB1003_163:
.Ltmp16629:
	jmp	.LBB1003_166
.LBB1003_204:
.Ltmp16617:
	movq	%rax, %r14
	jmp	.LBB1003_205
.LBB1003_174:
.Ltmp16634:
	movq	%rax, %r14
	jmp	.LBB1003_175
.LBB1003_94:
.Ltmp16595:
	movq	%rax, %r14
.Ltmp16596:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp16597:
	jmp	.LBB1003_226
.LBB1003_103:
.Ltmp16611:
	movb	$1, %bl
	movq	%rax, %r14
	jmp	.LBB1003_104
.LBB1003_44:
.Ltmp16601:
	lock		decq	(%rbp)
	movq	%rax, %r14
	jne	.LBB1003_226
	#MEMBARRIER
.Ltmp16602:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp16603:
	jmp	.LBB1003_226
.LBB1003_83:
.Ltmp16572:
	jmp	.LBB1003_84
.LBB1003_72:
.Ltmp16575:
.LBB1003_84:
	lock		decq	(%r13)
	movq	%rax, %r14
	jne	.LBB1003_86
	#MEMBARRIER
.Ltmp16576:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp16577:
.LBB1003_86:
.Ltmp16579:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp16580:
	jmp	.LBB1003_226
.LBB1003_89:
.Ltmp16578:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_90:
.Ltmp16604:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_225:
.Ltmp16641:
	movq	%rax, %r14
	jmp	.LBB1003_226
.LBB1003_122:
.Ltmp16614:
	movq	56(%rsp), %rsi
	movl	$8, %edx
	movq	%r13, %rdi
	movq	%rax, %r14
	callq	__rustc::__rust_dealloc
	jmp	.LBB1003_205
.LBB1003_91:
.Ltmp16583:
	movq	%rax, %r14
	movq	%rbx, 280(%rsp)
.Ltmp16584:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp16585:
.Ltmp16587:
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp16588:
	jmp	.LBB1003_226
.LBB1003_95:
.Ltmp16598:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_93:
.Ltmp16586:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1003_162:
.Ltmp16623:
	movq	%rax, %r14
	movq	%rbx, (%r12)
	jmp	.LBB1003_167
.LBB1003_165:
.Ltmp16626:
.LBB1003_166:
	movq	%rax, %r14
.LBB1003_167:
	movq	112(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1003_169
	movq	120(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1003_169:
	movq	16(%rsp), %rax
	leaq	384(%rsp), %rdi
	movq	%rax, 400(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movb	$1, %bpl
.LBB1003_175:
	cmpq	$0, 8(%rsp)
	je	.LBB1003_107
	movq	64(%rsp), %rdi
	movq	56(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB1003_107:
	testb	%bpl, %bpl
	je	.LBB1003_108
.LBB1003_205:
	movq	376(%rsp), %rax
	movb	$1, %bl
	lock		decq	(%rax)
	jne	.LBB1003_104
	#MEMBARRIER
.Ltmp16635:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp16636:
	jmp	.LBB1003_104
.LBB1003_108:
	xorl	%ebx, %ebx
.LBB1003_104:
.Ltmp16637:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp16638:
	testb	%bl, %bl
	je	.LBB1003_106
.LBB1003_226:
.Ltmp16642:
	leaq	456(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp16643:
.LBB1003_106:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB1003_203:
.Ltmp16644:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1003:
purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>::{closure#2}:
.Lfunc_begin1204:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception789
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
.Ltmp20688:
	leaq	48(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger as core::clone::Clone>::clone
.Ltmp20689:
	vmovups	16(%rsp), %ymm0
	vmovaps	(%rsp), %xmm1
	vmovups	%ymm0, 208(%rsp)
	vmovss	%xmm1, 240(%rsp)
	cmpb	$2, %bpl
	jne	.LBB1204_4
	cmpq	$0, 1080(%rsp)
	je	.LBB1204_4
.Ltmp20691:
	leaq	48(%rsp), %rdi
	leaq	464(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::defer::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp20692:
.LBB1204_4:
	movq	memcpy@GOTPCREL(%rip), %r15
	leaq	1712(%rsp), %rdi
	leaq	464(%rsp), %rsi
	movl	$1248, %edx
	vzeroupper
	callq	*%r15
	movq	24(%r14), %rsi
.Ltmp20698:
	leaq	248(%rsp), %rdi
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::fresh
.Ltmp20699:
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
.LBB1204_9:
	.cfi_def_cfa_offset 3008
.Ltmp20693:
	movq	%rax, %rbx
.Ltmp20694:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp20695:
	jmp	.LBB1204_11
.LBB1204_7:
.Ltmp20700:
	movq	%rax, %rbx
.Ltmp20701:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp20702:
.Ltmp20703:
	leaq	1712(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp20704:
	jmp	.LBB1204_12
.LBB1204_10:
.Ltmp20690:
	movq	%rax, %rbx
.LBB1204_11:
.Ltmp20696:
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp20697:
.LBB1204_12:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1204_6:
.Ltmp20705:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1204:
purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>::{closure#3}:
.Lfunc_begin1205:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception790
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
	jne	.LBB1205_6
	cmpb	$0, 1400(%r14)
	jne	.LBB1205_49
	movq	1368(%r14), %rcx
	shrq	$5, %rax
	testq	%rcx, %rcx
	je	.LBB1205_5
	movq	16(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1205_5
	movb	$1, 1400(%r14)
	jmp	.LBB1205_49
.LBB1205_5:
	movq	%rax, 1376(%r14)
.LBB1205_6:
	leaq	1248(%r14), %rsi
	leaq	80(%rsp), %rdi
	movq	%r14, %rdx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
	cmpb	$-1, 80(%rsp)
	je	.LBB1205_7
.LBB1205_49:
	movq	$-1, (%rbx)
.LBB1205_50:
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
.LBB1205_7:
	.cfi_def_cfa_offset 272
	movq	16(%r13), %rax
	movq	(%rax), %rdx
	movq	%rax, 72(%rsp)
	movq	$1, 80(%rsp)
	cmpq	$5, %rdx
	jae	.LBB1205_8
.LBB1205_9:
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
	jb	.LBB1205_10
	movq	16(%rbp), %r12
	movq	8(%rbp), %rbp
	decq	%r12
	jmp	.LBB1205_15
.LBB1205_10:
	addq	$8, %rbp
.LBB1205_15:
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
	jb	.LBB1205_16
.LBB1205_18:
	movq	%r15, 56(%rsp)
	movq	%rbx, 64(%rsp)
	xorl	%r15d, %r15d
	movq	%rdi, %rcx
	cmpq	$6, %rax
	setae	%al
	jb	.LBB1205_20
	movq	8(%rsp), %rcx
.LBB1205_20:
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
	jbe	.LBB1205_21
	movl	$2, 80(%rsp)
	movq	%rsi, 88(%rsp)
.Ltmp20711:
	leaq	80(%rsp), %rsi
	movq	%rsp, %rdi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp20712:
	movq	64(%rsp), %rbx
	movq	56(%rsp), %r12
	jmp	.LBB1205_24
.LBB1205_21:
	movq	64(%rsp), %rbx
	movq	56(%rsp), %r12
	cmpq	$6, %rcx
	cmovbq	%rcx, %rdx
	decq	%rdx
	cmpq	%rdx, %rax
	jae	.LBB1205_24
	xorl	%edx, %edx
	cmpq	$6, %rcx
	setae	%dl
	incq	%rax
	shll	$4, %edx
	movq	%rax, (%rsp,%rdx)
.LBB1205_24:
	movq	(%rsp), %rcx
	leaq	1448(%r14), %rsi
	leaq	8(%rsp), %rdx
	decq	%rcx
	cmpq	$5, %rcx
	jb	.LBB1205_26
	movq	16(%rsp), %rcx
	movq	8(%rsp), %rdx
	decq	%rcx
.LBB1205_26:
	movq	24(%r13), %rax
	movq	(%rax), %r8
	addq	$16, %r8
.Ltmp20713:
	leaq	80(%rsp), %rdi
	movq	%r14, %r9
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp20714:
	movq	80(%rsp), %rdx
	movl	88(%rsp), %ecx
	movl	92(%rsp), %eax
	cmpq	$-1, %rdx
	je	.LBB1205_38
	vmovups	112(%rsp), %zmm1
	vmovups	96(%rsp), %zmm0
	vmovups	%zmm1, 32(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	%rdx, (%rbx)
	movl	%ecx, 8(%rbx)
	movl	%eax, 12(%rbx)
	movq	(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1205_50
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1205_31
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1205_31:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1205_37
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1205_31
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
.LBB1205_34:
	cmpq	%rax, %rsi
	jge	.LBB1205_36
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1205_34
.LBB1205_36:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1205_37:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB1205_50
.LBB1205_38:
	movq	32(%r13), %rdx
	movq	(%rdx), %rdi
	movq	(%rsp), %rdx
	movq	%rdx, %rsi
	cmpq	$6, %rdx
	jb	.LBB1205_40
	movq	16(%rsp), %rsi
.LBB1205_40:
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB1205_51
	cmpq	$6, %rdx
	jb	.LBB1205_43
	movq	8(%rsp), %rdx
	movq	%rdx, 48(%rsp)
.LBB1205_43:
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
	jne	.LBB1205_45
.Ltmp20718:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
.Ltmp20719:
.LBB1205_45:
	movq	8(%r12), %rax
	movq	208(%rsp), %rdx
	leaq	(%r15,%r15,4), %rcx
	incq	%r15
	movq	%rdx, 32(%rax,%rcx,8)
	vmovups	176(%rsp), %ymm0
	vmovups	%ymm0, (%rax,%rcx,8)
	movq	%r15, 16(%r12)
	cmpb	$2, 1442(%r14)
	jne	.LBB1205_48
	movq	1312(%r14), %rax
	testq	%rax, %rax
	je	.LBB1205_48
	movq	1304(%r14), %rcx
	shlq	$5, %rax
	incq	-8(%rcx,%rax)
.LBB1205_48:
	leaq	1248(%r14), %rdi
	movq	%r14, %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::settle::<purrdf_core::ir::dataset::RdfDataset>
	jmp	.LBB1205_49
.LBB1205_8:
.Ltmp20706:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	80(%rsp), %rdi
	leaq	88(%rsp), %r12
	xorl	%esi, %esi
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp20707:
	jmp	.LBB1205_9
.LBB1205_16:
.Ltmp20709:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%rsp, %rdi
	movq	%r12, %rdx
	callq	*%rax
.Ltmp20710:
	movq	(%rsp), %rax
	leaq	8(%rsp), %rdi
	jmp	.LBB1205_18
.LBB1205_51:
.Ltmp20715:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.637(%rip), %rdx
	callq	*%rax
.Ltmp20716:
	ud2
.LBB1205_11:
.Ltmp20708:
	movq	%rax, %rbx
	movq	80(%rsp), %rax
	movq	%r12, 48(%rsp)
	cmpq	$6, %rax
	jae	.LBB1205_12
	jmp	.LBB1205_13
.LBB1205_53:
.Ltmp20720:
	movq	%rax, %rbx
.Ltmp20721:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::parallel::MintedRow>
.Ltmp20722:
	jmp	.LBB1205_13
.LBB1205_54:
.Ltmp20723:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1205_55:
.Ltmp20717:
	movq	%rax, %rbx
	movq	(%rsp), %rax
	cmpq	$5, %rax
	jbe	.LBB1205_13
.LBB1205_12:
	movq	48(%rsp), %rcx
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	movq	(%rcx), %rdi
	callq	__rustc::__rust_dealloc
.LBB1205_13:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1205:
purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>::{closure#1}:
.Lfunc_begin1206:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception791
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
	jne	.LBB1206_6
	cmpb	$0, 1400(%r14)
	jne	.LBB1206_35
	movq	1368(%r14), %rcx
	shrq	$5, %rax
	testq	%rcx, %rcx
	je	.LBB1206_5
	movq	16(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1206_5
	movb	$1, 1400(%r14)
	movq	$-1, (%rbx)
	jmp	.LBB1206_16
.LBB1206_5:
	movq	%rax, 1376(%r14)
.LBB1206_6:
	leaq	1248(%r14), %rsi
	movq	%rsp, %rdi
	movq	%r14, %rdx
	callq	<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
	cmpb	$-1, (%rsp)
	je	.LBB1206_7
.LBB1206_35:
	movq	$-1, (%rbx)
	jmp	.LBB1206_16
.LBB1206_7:
	movq	(%rbp), %r13
	leaq	1448(%r14), %rsi
	decq	%r13
	cmpq	$5, %r13
	jb	.LBB1206_8
	movq	16(%rbp), %r13
	movq	8(%rbp), %rbp
	decq	%r13
	jmp	.LBB1206_10
.LBB1206_8:
	addq	$8, %rbp
.LBB1206_10:
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
	je	.LBB1206_12
	vmovups	32(%rsp), %zmm1
	vmovups	16(%rsp), %zmm0
	movl	%edx, %esi
	shrl	$8, %esi
	vmovups	%zmm1, 112(%rsp)
	vmovups	%zmm0, 96(%rsp)
.LBB1206_15:
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
.LBB1206_16:
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
.LBB1206_12:
	.cfi_def_cfa_offset 240
	cmpl	$2, %edx
	jne	.LBB1206_13
.LBB1206_34:
	leaq	1248(%r14), %rdi
	movq	%r14, %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::settle::<purrdf_core::ir::dataset::RdfDataset>
	jmp	.LBB1206_35
.LBB1206_13:
	movq	%rsp, %rdi
	movq	%r14, %rsi
	callq	purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
	movq	(%rsp), %rax
	movzbl	8(%rsp), %edx
	cmpq	$-1, %rax
	je	.LBB1206_17
	movzbl	11(%rsp), %ecx
	movzwl	9(%rsp), %esi
	vmovups	16(%rsp), %zmm0
	vmovups	32(%rsp), %zmm1
	shll	$16, %ecx
	orl	%ecx, %esi
	movl	12(%rsp), %ecx
	vmovups	%zmm0, 96(%rsp)
	vmovups	%zmm1, 112(%rsp)
	jmp	.LBB1206_15
.LBB1206_17:
	testb	$1, %dl
	je	.LBB1206_34
	leaq	(,%r13,8), %r12
	cmpq	$5, %r13
	jae	.LBB1206_19
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	movq	%rbp, %rsi
	movq	%r12, %rdx
	callq	*%rax
	incq	%r13
	jmp	.LBB1206_29
.LBB1206_19:
	movl	$4, %esi
	movq	%r12, %rdi
	callq	__rustc::__rust_alloc
	testq	%rax, %rax
	je	.LBB1206_39
	movb	$61, %cl
	leaq	-1(%r13), %rsi
	bzhiq	%rcx, %r13, %rcx
	cmpq	%rsi, %rcx
	cmovbq	%rcx, %rsi
	cmpq	$16, %rsi
	jae	.LBB1206_22
	xorl	%esi, %esi
	movq	%r13, %rcx
	movq	%rbp, %rdx
	jmp	.LBB1206_24
.LBB1206_22:
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
.LBB1206_23:
	vmovups	(%rbp,%rdi,8), %zmm0
	vmovups	64(%rbp,%rdi,8), %zmm1
	vmovups	%zmm1, 64(%rax,%rdi,8)
	vmovups	%zmm0, (%rax,%rdi,8)
	addq	$16, %rdi
	cmpq	%rdi, %rsi
	jne	.LBB1206_23
.LBB1206_24:
	leaq	(%rbp,%r13,8), %rdi
	leaq	4(%rax,%rsi,8), %rsi
	xorl	%r8d, %r8d
.LBB1206_25:
	cmpq	%rdi, %rdx
	je	.LBB1206_27
	movl	(%rdx), %r9d
	movl	4(%rdx), %r10d
	addq	$8, %rdx
	movl	%r9d, -4(%rsi,%r8,8)
	movl	%r10d, (%rsi,%r8,8)
	incq	%r8
	cmpq	%r8, %rcx
	jne	.LBB1206_25
.LBB1206_27:
	incq	%r13
	movq	%rax, (%rsp)
	movq	%r13, 8(%rsp)
.LBB1206_29:
	movq	16(%r15), %r12
	cmpq	(%r15), %r12
	jne	.LBB1206_31
.Ltmp20724:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.Ltmp20725:
.LBB1206_31:
	movq	8(%r15), %rax
	leaq	(%r12,%r12,4), %rcx
	incq	%r12
	movq	%r13, (%rax,%rcx,8)
	vmovups	(%rsp), %ymm0
	vmovups	%ymm0, 8(%rax,%rcx,8)
	movq	%r12, 16(%r15)
	cmpb	$2, 1442(%r14)
	jne	.LBB1206_34
	movq	1312(%r14), %rax
	testq	%rax, %rax
	je	.LBB1206_34
	movq	1304(%r14), %rcx
	shlq	$5, %rax
	incq	-8(%rcx,%rax)
	jmp	.LBB1206_34
.LBB1206_39:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movq	%r12, %rsi
	callq	*%rax
.LBB1206_36:
.Ltmp20726:
	movq	%rax, %rbx
	cmpq	$6, %r13
	jb	.LBB1206_38
	movq	(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1206_38:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1206:
purrdf_sparql_eval::modifier::eval_group::<purrdf_core::ir::dataset::RdfDataset>::{closure#5}:
.Lfunc_begin1227:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception807
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
	subq	$1672, %rsp
	.cfi_def_cfa_offset 1728
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	(%rsi), %rax
	movq	%rdi, 48(%rsp)
	movq	%rsi, 56(%rsp)
	movq	16(%rax), %r15
	testq	%r15, %r15
	je	.LBB1227_1
	movq	8(%rax), %rax
	movq	malloc@GOTPCREL(%rip), %rbx
	movq	%rax, 72(%rsp)
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,2), %r12
	movq	%r12, %rdi
	callq	*%rbx
	movq	%rax, 8(%rsp)
	testq	%rax, %rax
	je	.LBB1227_45
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rsi
	movabsq	$-9223372036854775808, %rdx
	leaq	(%r12,%rax), %rcx
	sarq	$63, %rcx
	xorq	%rdx, %rcx
	addq	%r12, %rax
	cmovoq	%rcx, %rax
	incq	%rdi
	movq	$-1, %rcx
	cmoveq	%rcx, %rdi
	addq	%r12, %rsi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmovbq	%rcx, %rsi
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1227_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1227_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1227_11
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1227_5
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	%r12, (%rcx)
	movq	%r12, %rcx
	lock		xaddq	%rcx, (%rdx)
	movabsq	$-9223372036854775808, %rdx
	leaq	(%rcx,%r12), %rax
	sarq	$63, %rax
	xorq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	%r12, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1227_8:
	cmpq	%rax, %rcx
	jle	.LBB1227_10
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1227_8
.LBB1227_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1227_11:
	movq	8(%rsp), %rax
	leaq	416(%rsp), %r12
	movq	%r15, 256(%rsp)
	xorl	%ebx, %ebx
	movq	%r15, 64(%rsp)
	movq	%rax, 264(%rsp)
	jmp	.LBB1227_12
	.p2align	4
.LBB1227_13:
	movq	$0, 80(%rsp)
	movq	$8, 88(%rsp)
.LBB1227_26:
	movq	8(%rsp), %rax
	vmovups	80(%rsp), %xmm0
	leaq	(%rbx,%rbx,2), %rcx
	movq	%r15, 432(%rsp)
	incq	%rbx
	movq	%r15, 16(%rax,%rcx,8)
	movq	64(%rsp), %r15
	vmovaps	%xmm0, 416(%rsp)
	vmovups	%xmm0, (%rax,%rcx,8)
	cmpq	%r15, %rbx
	je	.LBB1227_27
.LBB1227_12:
	movq	72(%rsp), %rax
	leaq	(%rbx,%rbx,2), %rcx
	movq	16(%rax,%rcx,8), %r15
	testq	%r15, %r15
	je	.LBB1227_13
	movq	8(%rax,%rcx,8), %r13
	movq	malloc@GOTPCREL(%rip), %rax
	imulq	$216, %r15, %r14
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1227_31
	movq	%rax, %rbp
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movabsq	$-9223372036854775808, %rdx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rdi
	movq	$-1, %rsi
	leaq	(%r14,%rax), %rcx
	sarq	$63, %rcx
	xorq	%rdx, %rcx
	addq	%r14, %rax
	cmovoq	%rcx, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rcx
	incq	%rdi
	cmoveq	%rsi, %rdi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	addq	%r14, %rcx
	cmovbq	%rsi, %rcx
	movq	%rcx, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1227_17
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1227_17:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1227_23
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1227_17
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	movabsq	$-9223372036854775808, %rdx
	lock		incq	(%rax)
	lock		addq	%r14, (%rcx)
	movq	%r14, %rcx
	lock		xaddq	%rcx, (%rsi)
	leaq	(%rcx,%r14), %rax
	sarq	$63, %rax
	xorq	%rdx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	addq	%r14, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB1227_20:
	cmpq	%rax, %rcx
	jle	.LBB1227_22
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB1227_20
.LBB1227_22:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1227_23:
	movq	%r15, 80(%rsp)
	xorl	%r14d, %r14d
	movq	%rbp, 88(%rsp)
	.p2align	4
.LBB1227_24:
.Ltmp20902:
	movq	%r12, %rdi
	movq	%r13, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::fresh
.Ltmp20903:
	vmovups	416(%rsp), %zmm0
	vmovups	480(%rsp), %zmm1
	vmovups	568(%rsp), %zmm3
	vmovups	544(%rsp), %zmm2
	incq	%r14
	addq	$216, %r13
	vmovups	%zmm3, 152(%rbp)
	vmovups	%zmm2, 128(%rbp)
	vmovups	%zmm1, 64(%rbp)
	vmovups	%zmm0, (%rbp)
	addq	$216, %rbp
	cmpq	%r14, %r15
	jne	.LBB1227_24
	jmp	.LBB1227_26
.LBB1227_1:
	movq	$0, 256(%rsp)
	movq	$8, 264(%rsp)
.LBB1227_27:
	movq	56(%rsp), %rbx
	movq	256(%rsp), %rax
	movq	264(%rsp), %rcx
	movq	%r15, 32(%rsp)
	movq	16(%rbx), %rsi
	movq	%rcx, 24(%rsp)
	movq	%rax, 16(%rsp)
	movq	8(%rbx), %rcx
	movq	(%rsi), %rax
	testq	%rax, %rax
	cmoveq	%rax, %rsi
.Ltmp20914:
	leaq	416(%rsp), %rdi
	movq	%rcx, %rdx
	vzeroupper
	callq	<core::option::Option<&alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>>::map_or_else::<purrdf_sparql_eval::eval::EvalCtx, <purrdf_sparql_eval::eval::EvalCtx>::fork_for_loop_worker::{closure#0}, <purrdf_sparql_eval::eval::EvalCtx>::fork_for_loop_worker::{closure#1}>
.Ltmp20915:
	movq	24(%rbx), %rsi
	movzbl	160(%rsi), %ebx
.Ltmp20917:
	leaq	256(%rsp), %rdi
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger as core::clone::Clone>::clone
.Ltmp20918:
	vmovups	256(%rsp), %zmm0
	vmovups	320(%rsp), %zmm1
	vmovups	352(%rsp), %zmm2
	movb	%bl, 240(%rsp)
	vmovups	%zmm1, 144(%rsp)
	vmovups	%zmm0, 80(%rsp)
	vmovups	%zmm2, 176(%rsp)
	testb	%bl, %bl
	je	.LBB1227_30
.Ltmp20920:
	leaq	80(%rsp), %rdi
	leaq	416(%rsp), %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::row_checkpoint::WorkerLedger>::defer::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp20921:
.LBB1227_30:
	movq	48(%rsp), %rbx
	movq	memcpy@GOTPCREL(%rip), %rax
	leaq	416(%rsp), %rsi
	movl	$1248, %edx
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	vmovaps	16(%rsp), %xmm0
	vmovups	80(%rsp), %zmm3
	vmovups	144(%rsp), %zmm1
	vmovups	184(%rsp), %zmm2
	movq	32(%rsp), %rax
	movq	%rax, 1264(%rbx)
	vmovaps	%xmm0, 1248(%rbx)
	vmovups	%zmm1, 1336(%rbx)
	vmovups	%zmm3, 1272(%rbx)
	vmovups	%zmm2, 1376(%rbx)
	addq	$1672, %rsp
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
.LBB1227_31:
	.cfi_def_cfa_offset 1728
.Ltmp20908:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp20909:
	ud2
.LBB1227_45:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r12, %rsi
	callq	*%rax
.LBB1227_43:
.Ltmp20922:
	movq	%rax, %r15
.Ltmp20923:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp20924:
	jmp	.LBB1227_41
.LBB1227_40:
.Ltmp20919:
	movq	%rax, %r15
.LBB1227_41:
.Ltmp20925:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalCtx>
.Ltmp20926:
	jmp	.LBB1227_39
.LBB1227_38:
.Ltmp20916:
	movq	%rax, %r15
.LBB1227_39:
.Ltmp20927:
	leaq	16(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp20928:
	jmp	.LBB1227_37
.LBB1227_44:
.Ltmp20929:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1227_35:
.Ltmp20910:
	movq	%rax, %r15
	jmp	.LBB1227_36
.LBB1227_33:
.Ltmp20904:
	movq	%rax, %r15
	movq	%r14, 96(%rsp)
.Ltmp20905:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
.Ltmp20906:
.LBB1227_36:
	movq	%rbx, 272(%rsp)
.Ltmp20911:
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
.Ltmp20912:
.LBB1227_37:
	movq	%r15, %rdi
	callq	_Unwind_Resume@PLT
.LBB1227_46:
.Ltmp20913:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1227_34:
.Ltmp20907:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end1227:
purrdf_sparql_eval::modifier::eval_group::<purrdf_core::ir::dataset::RdfDataset>::{closure#6}:
.Lfunc_begin1228:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception808
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
	jne	.LBB1228_7
	cmpb	$0, 1424(%r15)
	jne	.LBB1228_5
	movq	40(%r13), %rax
	movq	1392(%r15), %rcx
	testq	%rcx, %rcx
	je	.LBB1228_6
	movq	16(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB1228_6
	movb	$1, 1424(%r15)
.LBB1228_5:
	movq	$-1, (%r12)
	jmp	.LBB1228_32
.LBB1228_6:
	movq	%rax, 1400(%r15)
.LBB1228_7:
	movq	(%rsi), %rax
	movq	%rsi, 32(%rsp)
	movq	(%rax), %rbx
	movq	$1, 192(%rsp)
	cmpq	$5, %rbx
	jae	.LBB1228_44
	vmovups	200(%rsp), %xmm0
	movq	224(%rsp), %rax
	movq	192(%rsp), %rdx
	movq	216(%rsp), %rcx
	movq	%rax, 128(%rsp)
	movq	%rdx, 96(%rsp)
	movq	%rcx, 120(%rsp)
	vmovups	%xmm0, 104(%rsp)
	testq	%rbx, %rbx
	je	.LBB1228_10
.LBB1228_9:
	movl	$2, %eax
	jmp	.LBB1228_11
.LBB1228_10:
	movl	$-1, %eax
.LBB1228_11:
	leaq	104(%rsp), %r14
	movl	%eax, 192(%rsp)
	movq	%r15, 40(%rsp)
	movq	%rbx, 200(%rsp)
.Ltmp20933:
	leaq	96(%rsp), %rdi
	leaq	192(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
.Ltmp20934:
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
	jb	.LBB1228_14
	movq	56(%rsp), %rdi
	movq	64(%rsp), %rax
.LBB1228_14:
	decq	%rax
	cmpq	%rax, %rdx
	ja	.LBB1228_43
	movq	(%r13), %rax
	movq	16(%r13), %rsi
	decq	%rax
	decq	%rsi
	cmpq	$5, %rax
	cmovbq	%rax, %rsi
	cmpq	%rsi, %rdx
	jne	.LBB1228_46
	movq	%rbp, 152(%rsp)
	movq	%r12, 144(%rsp)
	cmpq	$5, %rax
	jb	.LBB1228_18
	movq	8(%r13), %rsi
	jmp	.LBB1228_19
.LBB1228_18:
	leaq	8(%r13), %rsi
.LBB1228_19:
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
	je	.LBB1228_29
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
	jmp	.LBB1228_22
	.p2align	4
.LBB1228_21:
	movq	40(%rsp), %r15
	incq	%r13
	addq	$24, %r12
	addq	$120, %rbp
	vmovlps	%xmm0, (%rax,%rdi,8)
	cmpq	%r13, %r14
	je	.LBB1228_29
.LBB1228_22:
	movq	160(%rsp), %rax
	movq	-8(%r12), %rdx
	movq	(%r12), %rcx
	vmovups	8(%rax), %xmm0
	movq	(%rbx), %rax
	addq	$16, %rax
.Ltmp20938:
	movq	%r15, 24(%rsp)
	movq	%rax, 16(%rsp)
	vmovups	%xmm0, (%rsp)
	leaq	192(%rsp), %rdi
	movq	%rbp, %rsi
	movq	176(%rsp), %r8
	movq	168(%rsp), %r9
	callq	purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp20939:
	vmovsd	200(%rsp), %xmm0
	movq	192(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB1228_33
	movq	48(%rsp), %r15
	movq	%r15, %rsi
	cmpq	$6, %r15
	jb	.LBB1228_26
	movq	64(%rsp), %rsi
.LBB1228_26:
	movq	184(%rsp), %rax
	decq	%rsi
	movq	(%rax), %rdi
	addq	%r13, %rdi
	cmpq	%rsi, %rdi
	jae	.LBB1228_47
	leaq	56(%rsp), %rax
	cmpq	$6, %r15
	jb	.LBB1228_21
	movq	56(%rsp), %rax
	jmp	.LBB1228_21
.LBB1228_29:
	movq	32(%rsp), %rax
	vmovups	48(%rsp), %ymm0
	movq	80(%rsp), %rcx
	leaq	888(%r15), %rsi
	leaq	96(%rsp), %rdi
	movq	48(%rax), %rax
	movq	(%rax), %rdx
	movq	%rcx, 224(%rsp)
	leaq	192(%rsp), %rcx
	vmovups	%ymm0, 192(%rsp)
	vzeroupper
	callq	purrdf_sparql_eval::parallel::minted_row::<purrdf_core::ir::term::TermId>
	movq	152(%rsp), %r14
	movq	16(%r14), %rbx
	cmpq	(%r14), %rbx
	jne	.LBB1228_31
.Ltmp20943:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	callq	*%rax
.Ltmp20944:
.LBB1228_31:
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
.LBB1228_32:
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
.LBB1228_33:
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
	jb	.LBB1228_32
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1228_36
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1228_36:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1228_42
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1228_36
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
.LBB1228_39:
	cmpq	%rax, %rsi
	jge	.LBB1228_41
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB1228_39
.LBB1228_41:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1228_42:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB1228_32
.LBB1228_43:
.Ltmp20949:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %r8
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.674(%rip), %rcx
	movq	%rdx, %rsi
	xorl	%edi, %edi
	movq	%rax, %rdx
	vzeroupper
	callq	*%r8
.Ltmp20950:
	jmp	.LBB1228_48
.LBB1228_44:
.Ltmp20930:
	movq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	192(%rsp), %rdi
	leaq	200(%rsp), %r14
	xorl	%esi, %esi
	movq	%rbx, %rdx
	xorl	%ecx, %ecx
	callq	*%rax
.Ltmp20931:
	vmovups	192(%rsp), %ymm0
	movq	224(%rsp), %rax
	movq	%rax, 128(%rsp)
	vmovups	%ymm0, 96(%rsp)
	jmp	.LBB1228_9
.LBB1228_46:
.Ltmp20936:
	movq	core::slice::copy_from_slice_impl::len_mismatch_fail@GOTPCREL(%rip), %rcx
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.672(%rip), %rax
	movq	%rdx, %rdi
	movq	%rax, %rdx
	vzeroupper
	callq	*%rcx
.Ltmp20937:
	jmp	.LBB1228_48
.LBB1228_47:
.Ltmp20941:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.673(%rip), %rdx
	callq	*%rax
.Ltmp20942:
.LBB1228_48:
	ud2
.LBB1228_49:
.Ltmp20932:
	movq	192(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$6, %r15
	jae	.LBB1228_53
	jmp	.LBB1228_60
.LBB1228_50:
.Ltmp20945:
	movq	%rax, %rbx
.Ltmp20946:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::parallel::MintedRow>
.Ltmp20947:
	jmp	.LBB1228_60
.LBB1228_51:
.Ltmp20948:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB1228_52:
.Ltmp20935:
	movq	96(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$5, %r15
	jbe	.LBB1228_60
.LBB1228_53:
	movq	(%r14), %rdi
	jmp	.LBB1228_59
.LBB1228_54:
.Ltmp20940:
	movq	48(%rsp), %r15
	jmp	.LBB1228_56
.LBB1228_55:
.Ltmp20951:
.LBB1228_56:
	movq	%rax, %rbx
	cmpq	$6, %r15
	jb	.LBB1228_60
	movq	56(%rsp), %rdi
.LBB1228_59:
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1228_60:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1228:
purrdf_sparql_eval::modifier::aggregate_numeric_cost:
.Lfunc_begin1780:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1171
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
	movq	%rcx, %r15
	movq	%rdx, %r13
	movq	%rsi, %r14
	leaq	(%rax,%rax,4), %rbx
	leaq	(%rsi,%rbx), %rbp
	testq	%rdx, %rdx
	je	.LBB1780_9
	movq	%r14, %rax
	jmp	.LBB1780_3
	.p2align	4
.LBB1780_2:
	addq	$80, %rax
	cmpq	%rbp, %rax
	je	.LBB1780_9
.LBB1780_3:
	cmpq	$0, (%rax)
	js	.LBB1780_2
	cmpq	$20, 16(%rax)
	jb	.LBB1780_2
	movq	(%rdi), %rax
	leal	-3(%rax), %ecx
	cmpl	$2, %ecx
	jb	.LBB1780_53
	cmpl	$1, %eax
	je	.LBB1780_99
	xorl	%ebx, %ebx
	cmpl	$2, %eax
	je	.LBB1780_10
	jmp	.LBB1780_101
.LBB1780_9:
	cmpl	$2, (%rdi)
	movb	$1, %bl
	jne	.LBB1780_100
.LBB1780_10:
	cmpl	$18, %r12d
	setne	%al
	orb	%r15b, %al
	testb	$1, %al
	jne	.LBB1780_12
	testl	$65280, %r15d
	sete	%al
	xorl	%ecx, %ecx
	testb	%al, %bl
	jne	.LBB1780_100
	testq	%r13, %r13
	je	.LBB1780_100
.LBB1780_14:
	movq	%rcx, 256(%rsp)
	xorl	%esi, %esi
	xorl	%edi, %edi
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	movq	$0, 128(%rsp)
	movq	$0, 24(%rsp)
.LBB1780_15:
	addq	$80, %r14
	movq	%rax, 40(%rsp)
	movq	%rdi, 80(%rsp)
	movq	%rsi, 248(%rsp)
	.p2align	4
.LBB1780_16:
	cmpq	$0, -80(%r14)
	js	.LBB1780_20
	cmpq	$-1, -32(%r14)
	jne	.LBB1780_20
	movq	-40(%r14), %rsi
	cmpq	$33, %rsi
	jb	.LBB1780_20
	movq	-48(%r14), %rdi
	vmovdqu	anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546(%rip), %ymm0
	movzbl	anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546+32(%rip), %eax
	vpxor	(%rdi), %ymm0, %ymm0
	movzbl	32(%rdi), %ecx
	vmovd	%eax, %xmm1
	vmovd	%ecx, %xmm2
	vpternlogq	$246, %ymm2, %ymm1, %ymm0
	vptest	%ymm0, %ymm0
	je	.LBB1780_22
	.p2align	4
.LBB1780_20:
	movq	$0, 160(%rsp)
.LBB1780_21:
	leaq	-80(%r14), %rax
	addq	$80, %r14
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1780_16
	jmp	.LBB1780_45
.LBB1780_22:
	movq	<purrdf_xsd::datatype::XsdDatatype>::from_local@GOTPCREL(%rip), %rax
	addq	$-33, %rsi
	addq	$33, %rdi
	vzeroupper
	callq	*%rax
	cmpb	$-1, %al
	je	.LBB1780_20
	movzbl	%al, %ecx
	movq	-72(%r14), %rsi
	movq	-64(%r14), %rdx
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
	cmpb	$0, 160(%rsp)
	je	.LBB1780_21
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
	je	.LBB1780_34
	movq	%rdx, %rax
	decq	%rax
	movq	%rdx, 40(%rsp)
	je	.LBB1780_127
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
	jb	.LBB1780_28
	shrq	$5, %rdx
	movabsq	$755578637259143235, %rax
	orl	$5, %r9d
	mulxq	%rax, %rdx, %rdx
	shrq	$7, %rdx
.LBB1780_28:
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
	je	.LBB1780_32
	cmpq	$39, 240(%rsp)
	setb	%dl
	cmpq	$19, %rcx
	setb	%r10b
	testb	%r10b, %dl
	je	.LBB1780_32
	cmpq	$18, 80(%rsp)
	ja	.LBB1780_32
	cmpq	$39, %rax
	jb	.LBB1780_41
.LBB1780_32:
	movq	16(%rsp), %r10
	movq	%r10, %rdx
	subq	%rcx, %rdx
	jae	.LBB1780_35
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
	jmp	.LBB1780_36
.LBB1780_34:
	movl	192(%rsp), %eax
	movq	%r8, 8(%rsp)
	movq	%r11, 32(%rsp)
	movq	%rcx, 16(%rsp)
	movl	%eax, 4(%rsp)
	movq	%rdx, %rax
	jmp	.LBB1780_42
.LBB1780_35:
	movabsq	$-2049638230412172401, %rcx
	mulxq	%rcx, %rcx, %rcx
	movq	$-1, %rdx
	shrq	$3, %rcx
	incq	%rcx
	addq	%rcx, %r8
	cmovbq	%rdx, %r8
	movq	%r8, %rcx
.LBB1780_36:
	movq	%rcx, %rdx
	shrq	$62, %rdx
	jne	.LBB1780_43
	leaq	(,%rcx,4), %rdx
.LBB1780_38:
	movq	8(%rsp), %r10
	movabsq	$4611686018427387903, %r11
	cmpq	%r8, %r10
	cmovaq	%r10, %r8
	movq	$-1, %r10
	incq	%r8
	cmoveq	%r10, %r8
	cmpq	%r11, %r8
	ja	.LBB1780_44
	leaq	(,%r8,4), %r10
.LBB1780_40:
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
.LBB1780_41:
	movq	%rax, 32(%rsp)
	movq	40(%rsp), %rax
	movl	$-1, 4(%rsp)
	movq	%r9, 8(%rsp)
	movq	%rdi, 16(%rsp)
.LBB1780_42:
	movb	$1, %cl
	movq	%rcx, 128(%rsp)
	cmpq	%rbp, %r14
	jne	.LBB1780_15
	jmp	.LBB1780_122
.LBB1780_43:
	movq	$-1, %rdx
	jmp	.LBB1780_38
.LBB1780_44:
	movq	$-1, %r10
	jmp	.LBB1780_40
.LBB1780_45:
	testb	$1, 128(%rsp)
	movq	24(%rsp), %rdx
	je	.LBB1780_102
	movq	16(%rsp), %rdi
	movq	32(%rsp), %rbp
	cmpq	$18, %rdi
	ja	.LBB1780_49
.LBB1780_47:
	cmpq	$38, %rbp
	ja	.LBB1780_49
	movl	%r15d, %eax
	movq	%r12, %rcx
	andl	$1, %eax
	xorq	$18, %rcx
	orq	%rax, %rcx
	movl	%r15d, %eax
	andl	$65280, %eax
	orq	%rcx, %rax
	je	.LBB1780_102
.LBB1780_49:
	cmpb	$0, 256(%rsp)
	je	.LBB1780_103
	cmpq	%rbp, %rdi
	jae	.LBB1780_112
	movq	8(%rsp), %rax
	movq	%rdx, %rsi
	testq	%rdi, %rdi
	je	.LBB1780_113
	incq	%rbp
	movq	$-1, %rcx
	cmoveq	%rcx, %rbp
	jmp	.LBB1780_113
.LBB1780_12:
	xorl	%ecx, %ecx
	testq	%r13, %r13
	jne	.LBB1780_14
.LBB1780_100:
	xorl	%ebx, %ebx
.LBB1780_101:
	xorl	%edx, %edx
.LBB1780_102:
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
.LBB1780_53:
	.cfi_def_cfa_offset 320
	movq	<purrdf_xsd::datatype::XsdDatatype>::from_local@GOTPCREL(%rip), %r15
	addq	$80, %r14
	addq	$-80, %rbx
	leaq	88(%rsp), %r12
	xorl	%r13d, %r13d
	.p2align	4
.LBB1780_54:
	cmpq	$0, -80(%r14)
	js	.LBB1780_58
	cmpq	$-1, -32(%r14)
	jne	.LBB1780_58
	movq	-40(%r14), %rsi
	cmpq	$33, %rsi
	jb	.LBB1780_58
	movq	-48(%r14), %rdi
	vmovdqu	anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546(%rip), %ymm1
	movzbl	anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546+32(%rip), %eax
	vpxor	(%rdi), %ymm1, %ymm1
	movzbl	32(%rdi), %ecx
	vmovd	%eax, %xmm2
	vmovdqu	%ymm2, 128(%rsp)
	vmovd	%ecx, %xmm0
	vpternlogq	$246, %ymm0, %ymm2, %ymm1
	vptest	%ymm1, %ymm1
	je	.LBB1780_60
	.p2align	4
.LBB1780_58:
	movq	$0, 88(%rsp)
.LBB1780_59:
	leaq	-80(%r14), %rax
	addq	$80, %r14
	addq	$-80, %rbx
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1780_54
	jmp	.LBB1780_87
.LBB1780_60:
	addq	$-33, %rsi
	addq	$33, %rdi
	vzeroupper
	callq	*%r15
	cmpb	$-1, %al
	je	.LBB1780_58
	movzbl	%al, %ecx
	movq	-72(%r14), %rsi
	movq	-64(%r14), %rdx
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	cmpb	$0, 88(%rsp)
	je	.LBB1780_59
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$128, %edi
	movl	$128, %r13d
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1780_128
	movq	%rax, %r12
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	addq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %r13
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	cmovbq	%rcx, %r13
	movabsq	$9223372036854775807, %rcx
	subq	$-128, %rax
	movq	%r13, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jle	.LBB1780_65
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
.LBB1780_65:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1780_71
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1780_65
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
.LBB1780_68:
	cmpq	%rax, %rdx
	jle	.LBB1780_70
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1780_68
.LBB1780_70:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1780_71:
	vmovdqu	96(%rsp), %ymm0
	movq	$4, 56(%rsp)
	movq	%r12, 64(%rsp)
	movq	$1, 72(%rsp)
	vmovdqu	%ymm0, (%r12)
	testq	%rbx, %rbx
	je	.LBB1780_120
	movl	$1, %ebx
.LBB1780_73:
	addq	$80, %r14
.LBB1780_74:
	cmpq	$0, -80(%r14)
	js	.LBB1780_78
	cmpq	$-1, -32(%r14)
	jne	.LBB1780_78
	movq	-40(%r14), %rsi
	cmpq	$33, %rsi
	jb	.LBB1780_78
	movq	-48(%r14), %rdi
	vmovdqu	anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546(%rip), %ymm1
	movzbl	32(%rdi), %eax
	vpxor	(%rdi), %ymm1, %ymm1
	vmovd	%eax, %xmm0
	vpternlogq	$246, 128(%rsp), %ymm0, %ymm1
	vptest	%ymm1, %ymm1
	je	.LBB1780_80
.LBB1780_78:
	movq	$0, 160(%rsp)
.LBB1780_79:
	leaq	-80(%r14), %rax
	addq	$80, %r14
	addq	$80, %rax
	cmpq	%rbp, %rax
	jne	.LBB1780_74
	jmp	.LBB1780_121
.LBB1780_80:
	movq	-72(%r14), %rax
	movq	-64(%r14), %r13
	addq	$-33, %rsi
	addq	$33, %rdi
	movq	%rax, 24(%rsp)
	vzeroupper
	callq	*%r15
	cmpb	$-1, %al
	je	.LBB1780_78
.Ltmp34718:
	movzbl	%al, %ecx
	movq	24(%rsp), %rsi
	movq	<purrdf_xsd::exact::cost::Shape>::of_lexical@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	movq	%r13, %rdx
	callq	*%rax
.Ltmp34719:
	cmpb	$0, 160(%rsp)
	je	.LBB1780_79
	cmpq	56(%rsp), %rbx
	jne	.LBB1780_86
.Ltmp34721:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$32, %r8d
	leaq	56(%rsp), %rdi
	movq	%rbx, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)
.Ltmp34722:
	movq	64(%rsp), %r12
.LBB1780_86:
	leaq	168(%rsp), %rcx
	movq	%rbx, %rax
	shlq	$5, %rax
	incq	%rbx
	vmovdqu	(%rcx), %ymm0
	vmovdqu	%ymm0, (%r12,%rax)
	movq	%rbx, 72(%rsp)
	cmpq	%rbp, %r14
	jne	.LBB1780_73
	jmp	.LBB1780_121
.LBB1780_87:
	movl	$8, %r12d
	xorl	%ebx, %ebx
.LBB1780_88:
.Ltmp34724:
	movq	purrdf_xsd::exact::cost::compare_chain@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movq	%r12, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp34725:
	movq	%rax, %rbx
	testq	%r13, %r13
	je	.LBB1780_102
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF, %rax
	jge	.LBB1780_92
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952)@TPOFF
	.p2align	4
.LBB1780_92:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1780_98
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
	jne	.LBB1780_92
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
.LBB1780_95:
	cmpq	%rax, %rdx
	jge	.LBB1780_97
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1780_95
.LBB1780_97:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952)(%rip)
.LBB1780_98:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	callq	*%rax
	movq	%r14, %rdx
	jmp	.LBB1780_102
.LBB1780_99:
	movb	$1, %cl
	testq	%r13, %r13
	jne	.LBB1780_14
	jmp	.LBB1780_100
.LBB1780_103:
	movq	%rdx, %r14
	movq	%r13, 176(%rsp)
	movq	$0, 184(%rsp)
	movw	$0, 160(%rsp)
.Ltmp34727:
	leaq	88(%rsp), %rdi
	leaq	160(%rsp), %rsi
	vzeroupper
	callq	purrdf_xsd::numeric::exact_path::shape_of (.llvm.13195840536648017546)
.Ltmp34728:
	cmpl	$1, 88(%rsp)
	jne	.LBB1780_124
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
	movq	%r15, %rdx
	movl	%ecx, 116(%rsp)
	vmovups	%ymm0, 160(%rsp)
	vzeroupper
	callq	*%rax
	vmovdqu	128(%rsp), %ymm0
	addq	%rax, %rbx
	movq	$-1, %rax
	movq	%rdx, %rcx
	movq	%r13, %rdi
	cmovbq	%rax, %rbx
	cmpq	%rdx, %r14
	cmovaq	%r14, %rcx
	testb	$1, %r15b
	je	.LBB1780_109
	vpextrq	$1, %xmm0, %rdx
	movq	%rdx, %rsi
	shrq	$62, %rsi
	jne	.LBB1780_126
	shlq	$2, %rdx
.LBB1780_108:
	addq	%rdi, %rdx
	movq	$-1, %r12
	cmovaeq	%rdx, %r12
.LBB1780_109:
	xorl	%edx, %edx
	subq	%rdi, %rbp
	vextracti128	$1, %ymm0, %xmm0
	movabsq	$-8198552921648689607, %rdi
	movabsq	$2049638230412172401, %r8
	cmovaeq	%rbp, %rdx
	vmovq	%xmm0, %rsi
	addq	%rdx, %rsi
	cmovbq	%rax, %rsi
	incq	%rsi
	cmoveq	%rax, %rsi
	addq	%r12, %rsi
	cmovbq	%rax, %rsi
	movabsq	$-2049638230412172401, %rax
	movq	%rsi, %rdx
	mulxq	%rax, %rdx, %rdx
	imulq	%rsi, %rdi
	xorl	%eax, %eax
	shrq	$3, %rdx
	cmpq	%r8, %rdi
	seta	%al
	addq	%rdx, %rax
	cmpq	%rsi, %r12
	jae	.LBB1780_117
	testq	%r12, %r12
	je	.LBB1780_118
	incq	%rsi
	movq	$-1, %rdx
	cmoveq	%rdx, %rsi
	jmp	.LBB1780_118
.LBB1780_112:
	movq	8(%rsp), %rax
	addq	$2, %rdi
	movq	$-1, %rbp
	movq	%rdx, %rsi
	cmovaeq	%rdi, %rbp
.LBB1780_113:
	movl	$9, %ecx
	mulq	%rcx
	jo	.LBB1780_123
	cmpl	$0, 4(%rsp)
	jns	.LBB1780_116
	incq	%rbp
	movq	$-1, %rcx
	cmoveq	%rcx, %rbp
.LBB1780_116:
	addq	%rbp, %rax
	movq	$-1, %rcx
	cmovbq	%rcx, %rax
	movq	%rax, %rdx
	incq	%rdx
	cmoveq	%rcx, %rdx
	addq	%rdx, %rbx
	movq	%rsi, %rdx
	cmovbq	%rcx, %rbx
	cmpq	%rax, %rsi
	cmovbeq	%rax, %rdx
	jmp	.LBB1780_102
.LBB1780_117:
	addq	$2, %r12
	movq	$-1, %rsi
	cmovaeq	%r12, %rsi
.LBB1780_118:
	movl	$9, %edx
	mulq	%rdx
	jo	.LBB1780_123
	incq	%rsi
	movq	$-1, %rdi
	cmoveq	%rdi, %rsi
	addq	%rsi, %rax
	cmovbq	%rdi, %rax
	movq	%rax, %rdx
	incq	%rdx
	cmoveq	%rdi, %rdx
	addq	%rdx, %rbx
	movq	%rax, %rdx
	cmovbq	%rdi, %rbx
	cmpq	%rax, %rcx
	cmovaq	%rcx, %rdx
	jmp	.LBB1780_102
.LBB1780_120:
	movl	$1, %ebx
.LBB1780_121:
	movq	56(%rsp), %r13
	jmp	.LBB1780_88
.LBB1780_122:
	movq	24(%rsp), %rdx
	movq	16(%rsp), %rdi
	movq	32(%rsp), %rbp
	cmpq	$18, %rdi
	jbe	.LBB1780_47
	jmp	.LBB1780_49
.LBB1780_123:
	movq	$-1, %rbx
	movq	$-1, %rdx
	jmp	.LBB1780_102
.LBB1780_124:
.Ltmp34729:
	movq	core::option::expect_failed@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.1836(%rip), %rdi
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.1837(%rip), %rdx
	movl	$22, %esi
	callq	*%rax
.Ltmp34730:
	ud2
.LBB1780_126:
	movq	$-1, %rdx
	jmp	.LBB1780_108
.LBB1780_127:
	movq	core::num::imp::int_log10::panic_for_nonpositive_argument@GOTPCREL(%rip), %rax
	leaq	.Lanon.a12f493ba210922c94e5446ac885c35e.554(%rip), %rdi
	callq	*%rax
.LBB1780_128:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$128, %esi
	callq	*%rax
.LBB1780_129:
.Ltmp34723:
	jmp	.LBB1780_131
.LBB1780_130:
.Ltmp34720:
.LBB1780_131:
	movq	56(%rsp), %rsi
	movq	%rax, %rbx
	testq	%rsi, %rsi
	je	.LBB1780_136
	movq	64(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	jmp	.LBB1780_135
.LBB1780_133:
.Ltmp34726:
	movq	%rax, %rbx
	testq	%r13, %r13
	je	.LBB1780_136
	shlq	$5, %r13
	movl	$8, %edx
	movq	%r12, %rdi
	movq	%r13, %rsi
.LBB1780_135:
	callq	__rustc::__rust_dealloc
.LBB1780_136:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB1780_137:
.Ltmp34731:
	leaq	160(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1780:

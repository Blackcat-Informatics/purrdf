purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin346:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception252
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
	je	.LBB346_11
	movq	1128(%r14), %rcx
	movq	%rax, 464(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%r15, %r12
	shlq	$4, %r12
	movq	%r14, 32(%rsp)
	movq	%r15, 200(%rsp)
	movq	%r13, 16(%rsp)
	movq	%rbp, 456(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 448(%rsp)
	addq	%rcx, %rax
	movq	%rax, 24(%rsp)
	testq	%r15, %r15
	je	.LBB346_36
	movq	bcmp@GOTPCREL(%rip), %rbx
	movq	%rcx, %rax
.LBB346_3:
	movq	24(%rax), %rbp
	movq	32(%rax), %r14
	addq	$48, %rax
	movq	%rax, (%rsp)
	leaq	16(%rbp), %r15
	jmp	.LBB346_6
	.p2align	4
.LBB346_4:
	xorq	%r14, %rax
	xorq	%rbp, %rdi
	orq	%rax, %rdi
	je	.LBB346_8
.LBB346_5:
	addq	$16, %r13
	cmpq	%r12, %r13
	je	.LBB346_10
.LBB346_6:
	movq	(%r13), %rdi
	movq	8(%r13), %rax
	cmpq	%rbp, %rdi
	sete	%cl
	cmpq	%r14, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB346_4
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%rbx
	testl	%eax, %eax
	jne	.LBB346_5
.LBB346_8:
	movq	(%rsp), %rax
	movq	16(%rsp), %r13
	cmpq	24(%rsp), %rax
	jne	.LBB346_3
	movq	32(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 208(%rsp)
	movq	$0, 224(%rsp)
	movq	$8, 232(%rsp)
	jmp	.LBB346_68
.LBB346_10:
	movq	200(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB346_37
	jmp	.LBB346_161
.LBB346_11:
.Ltmp14312:
	leaq	624(%rsp), %rdi
	movq	%rbp, %rsi
	movq	%r14, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14313:
	cmpl	$1, 624(%rsp)
	jne	.LBB346_73
.LBB346_13:
	vmovups	672(%rsp), %zmm1
	vmovups	640(%rsp), %zmm0
	movq	592(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB346_23
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
	jge	.LBB346_16
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_16:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_22
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_16
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
.LBB346_19:
	cmpq	%rax, %rsi
	jge	.LBB346_21
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB346_19
.LBB346_21:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_22:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB346_23:
	movq	520(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB346_33
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
	jge	.LBB346_26
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_26:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_32
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_26
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
.LBB346_29:
	cmpq	%rax, %rsi
	jge	.LBB346_31
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB346_29
.LBB346_31:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_32:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB346_33:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB346_157
	lock		decq	(%rax)
	jne	.LBB346_157
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB346_157
.LBB346_36:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, (%rsp)
	lock		incq	(%rbp)
	jle	.LBB346_161
.LBB346_37:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 208(%rsp)
	movq	%r14, 216(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB346_158
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
	jle	.LBB346_40
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB346_40:
	movq	32(%rsp), %rax
	addq	$1120, %rax
	movq	%rax, 8(%rsp)
	.p2align	4
.LBB346_41:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_47
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_41
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
.LBB346_44:
	cmpq	%rax, %rdx
	jle	.LBB346_46
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB346_44
.LBB346_46:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_47:
	movq	(%rsp), %rax
	movq	%rbp, (%rsi)
	movq	$4, 80(%rsp)
	movq	%rsi, 88(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 96(%rsp)
	cmpq	24(%rsp), %rax
	je	.LBB346_62
	movl	$1, %r14d
	testq	%r15, %r15
	je	.LBB346_108
	movq	(%rsp), %rbx
	jmp	.LBB346_52
	.p2align	4
.LBB346_50:
	movq	88(%rsp), %rsi
.LBB346_51:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 96(%rsp)
	cmpq	24(%rsp), %rbx
	je	.LBB346_63
.LBB346_52:
	movq	%rsi, 40(%rsp)
.LBB346_53:
	movq	24(%rbx), %r13
	leaq	48(%rbx), %rax
	movq	32(%rbx), %rbp
	movq	16(%rsp), %rbx
	movq	%rax, (%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB346_56
	.p2align	4
.LBB346_54:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB346_58
.LBB346_55:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB346_59
.LBB346_56:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB346_54
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB346_55
.LBB346_58:
	movq	(%rsp), %rbx
	cmpq	24(%rsp), %rbx
	jne	.LBB346_53
	jmp	.LBB346_63
	.p2align	4
.LBB346_59:
	lock		incq	(%r13)
	movq	40(%rsp), %rsi
	jle	.LBB346_161
	movq	(%rsp), %rbx
	movq	%r13, 208(%rsp)
	movq	%rbp, 216(%rsp)
	cmpq	80(%rsp), %r14
	jne	.LBB346_51
.Ltmp14277:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	80(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.4261325137610144415)
.Ltmp14278:
	jmp	.LBB346_50
.LBB346_62:
	movl	$1, %r14d
.LBB346_63:
	movq	32(%rsp), %rbp
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
	jmp	.LBB346_65
	.p2align	4
.LBB346_64:
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
	je	.LBB346_67
.LBB346_65:
	vmovups	-16(%r15), %xmm0
	movq	8(%rsp), %rax
	vmovups	%xmm0, 104(%rsp)
	movq	$2, 80(%rsp)
	movb	$0, 120(%rsp)
	cmpq	(%rax), %rbx
	jne	.LBB346_64
.Ltmp14288:
	movq	8(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp14289:
	jmp	.LBB346_64
.LBB346_67:
	movq	16(%rsp), %r13
.LBB346_68:
	movq	%r14, 216(%rsp)
.Ltmp14296:
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14297:
	movq	72(%rsp), %rbx
	movq	200(%rsp), %r15
	movq	464(%rsp), %r14
.Ltmp14298:
	movq	456(%rsp), %rcx
	leaq	80(%rsp), %rdi
	movq	%rbp, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14299:
	movq	1136(%rbp), %rsi
	subq	%r14, %rsi
	jb	.LBB346_72
	movq	448(%rsp), %rdi
	addq	1128(%rbp), %rdi
	movq	%r14, 1136(%rbp)
.Ltmp14300:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp14301:
.LBB346_72:
	vmovups	128(%rsp), %zmm1
	vmovups	80(%rsp), %zmm0
	vmovups	%zmm1, 672(%rsp)
	vmovups	%zmm0, 624(%rsp)
	cmpl	$1, 624(%rsp)
	je	.LBB346_13
.LBB346_73:
	leaq	632(%rsp), %rcx
.Ltmp14314:
	leaq	80(%rsp), %rdi
	leaq	520(%rsp), %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp14315:
	cmpq	$-1, 80(%rsp)
	je	.LBB346_105
	vmovups	80(%rsp), %ymm0
	vmovups	%ymm0, 480(%rsp)
.Ltmp14316:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp14317:
	vmovups	560(%rsp), %zmm1
	vmovups	520(%rsp), %zmm0
	movq	%rax, 472(%rsp)
	movq	%rax, 32(%rsp)
	vmovups	%zmm1, 248(%rsp)
	vmovups	%zmm0, 208(%rsp)
	movq	32(%rax), %rcx
	movq	%rcx, %r13
	shlq	$4, %r13
	movq	%rcx, 16(%rsp)
	testq	%rcx, %rcx
	je	.LBB346_114
	movq	24(%rax), %r15
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
	movq	%rax, 8(%rsp)
	testq	%rax, %rax
	je	.LBB346_159
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
	jle	.LBB346_80
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_80:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_86
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_80
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
.LBB346_83:
	cmpq	%rax, %rdx
	jle	.LBB346_85
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB346_83
.LBB346_85:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_86:
	movq	504(%rsp), %rbx
	movq	16(%rsp), %r12
	xorl	%r14d, %r14d
	addq	$16, %rbx
	.p2align	4
.LBB346_87:
	leaq	(%r15,%r14), %rsi
.Ltmp14319:
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.4261325137610144415)
.Ltmp14320:
	movq	8(%rsp), %rcx
	movq	%rax, (%rcx,%r14)
	movq	%rdx, 8(%rcx,%r14)
	addq	$16, %r14
	decq	%r12
	jne	.LBB346_87
	movq	496(%rsp), %r15
	movq	%r13, 40(%rsp)
	testq	%r15, %r15
	je	.LBB346_115
.LBB346_90:
	movq	malloc@GOTPCREL(%rip), %rbx
	leaq	(,%r15,8), %rax
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	vzeroupper
	callq	*%rbx
	testq	%rax, %rax
	je	.LBB346_160
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
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
	jle	.LBB346_93
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_93:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_99
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_93
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
.LBB346_96:
	cmpq	%rax, %rdx
	jle	.LBB346_98
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB346_96
.LBB346_98:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_99:
	movq	488(%rsp), %rax
	movq	8(%rsp), %rdx
	movq	40(%rsp), %rcx
	movq	%r15, 48(%rsp)
	movq	%r13, 56(%rsp)
	movq	$0, 64(%rsp)
	xorl	%r12d, %r12d
	xorl	%ebx, %ebx
	movq	%rax, (%rsp)
	leaq	(%rdx,%rcx), %rax
	movq	%rax, 24(%rsp)
	jmp	.LBB346_101
	.p2align	4
.LBB346_100:
	movq	%rbp, (%r13,%r12)
	movq	%r15, 8(%r13,%r12)
	incq	%rbx
	vmovaps	352(%rsp), %xmm0
	vmovups	%xmm0, 16(%r13,%r12)
	movq	368(%rsp), %rax
	movq	%rax, 32(%r13,%r12)
	addq	$40, %r12
	movq	%rbx, 64(%rsp)
	cmpq	%r12, %r14
	je	.LBB346_116
.LBB346_101:
	movq	(%rsp), %rax
	movq	8(%rsp), %rcx
	movq	24(%rsp), %rdx
	movq	$1, 80(%rsp)
	addq	%r12, %rax
	movq	%rcx, 352(%rsp)
	movq	%rdx, 360(%rsp)
	movq	%rax, 368(%rsp)
.Ltmp14325:
	leaq	80(%rsp), %rdi
	leaq	352(%rsp), %rsi
	callq	<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>
.Ltmp14326:
	leaq	96(%rsp), %rax
	movq	80(%rsp), %rbp
	movq	88(%rsp), %r15
	vmovups	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	%rax, 368(%rsp)
	vmovaps	%xmm0, 352(%rsp)
	cmpq	48(%rsp), %rbx
	jne	.LBB346_100
.Ltmp14328:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	callq	*%rax
.Ltmp14329:
	movq	56(%rsp), %r13
	jmp	.LBB346_100
.LBB346_105:
.Ltmp14344:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14345:
	vmovups	560(%rsp), %zmm1
	vmovups	520(%rsp), %zmm0
	movq	%rax, 376(%rsp)
	movq	$0, 352(%rsp)
	movq	$8, 360(%rsp)
	movq	$0, 368(%rsp)
	vmovups	%zmm1, 120(%rsp)
	vmovups	%zmm0, 80(%rsp)
	cmpq	$-1, 80(%rsp)
	je	.LBB346_143
	leaq	208(%rsp), %rdi
	leaq	352(%rsp), %rsi
	leaq	520(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB346_144
	jmp	.LBB346_153
.LBB346_108:
	movq	(%rsp), %rax
	movl	$24, %ebx
	leaq	80(%rsp), %rbp
	jmp	.LBB346_110
	.p2align	4
.LBB346_109:
	addq	$48, %r12
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %rax
	movq	%r14, 96(%rsp)
	cmpq	24(%rsp), %r12
	je	.LBB346_63
.LBB346_110:
	movq	24(%rax), %r13
	movq	32(%rax), %r15
	lock		incq	(%r13)
	jle	.LBB346_161
	movq	%r13, 208(%rsp)
	movq	%rax, %r12
	movq	%r15, 216(%rsp)
	cmpq	80(%rsp), %r14
	jne	.LBB346_109
.Ltmp14280:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.4261325137610144415)
.Ltmp14281:
	movq	88(%rsp), %rsi
	jmp	.LBB346_109
.LBB346_114:
	movl	$8, %eax
	movq	%rax, 8(%rsp)
	movq	496(%rsp), %r15
	movq	%r13, 40(%rsp)
	testq	%r15, %r15
	jne	.LBB346_90
.LBB346_115:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
.LBB346_116:
	vmovups	248(%rsp), %zmm1
	vmovups	208(%rsp), %zmm0
	movq	48(%rsp), %rcx
	movq	64(%rsp), %rax
	movq	56(%rsp), %rdx
	movq	%rcx, 320(%rsp)
	movq	32(%rsp), %rcx
	movq	%rax, 336(%rsp)
	movq	%rdx, 328(%rsp)
	vmovups	%zmm1, 120(%rsp)
	vmovups	%zmm0, 80(%rsp)
	cmpq	$-1, 80(%rsp)
	movq	%rcx, 344(%rsp)
	je	.LBB346_118
	leaq	352(%rsp), %rdi
	leaq	320(%rsp), %rsi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB346_119
.LBB346_118:
	movq	328(%rsp), %rcx
	movq	320(%rsp), %rax
	movq	336(%rsp), %rdx
	movq	%rcx, 368(%rsp)
	movq	344(%rsp), %rcx
	movq	%rax, 360(%rsp)
	movq	%rdx, 376(%rsp)
	movq	%rcx, 384(%rsp)
	movq	$-1, 352(%rsp)
.LBB346_119:
	movq	72(%rsp), %rbx
	movq	40(%rsp), %r14
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB346_129
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
	jge	.LBB346_122
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_122:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_128
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_122
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
.LBB346_125:
	cmpq	%rax, %rsi
	jge	.LBB346_127
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB346_125
.LBB346_127:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_128:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB346_129:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB346_132
	lock		decq	(%rax)
	jne	.LBB346_132
	leaq	176(%rsp), %rdi
	#MEMBARRIER
.Ltmp14334:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp14335:
.LBB346_132:
	vmovups	384(%rsp), %zmm1
	vmovups	352(%rsp), %zmm0
	cmpq	$0, 16(%rsp)
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	je	.LBB346_142
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
	jge	.LBB346_135
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_135:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_141
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_135
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
.LBB346_138:
	cmpq	%rax, %rdx
	jge	.LBB346_140
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB346_138
.LBB346_140:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_141:
	movq	free@GOTPCREL(%rip), %rax
	movq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB346_142:
	leaq	480(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB346_157
.LBB346_143:
	movq	360(%rsp), %rcx
	movq	352(%rsp), %rax
	movq	368(%rsp), %rdx
	movq	%rcx, 224(%rsp)
	movq	376(%rsp), %rcx
	movq	%rax, 216(%rsp)
	movq	%rdx, 232(%rsp)
	movq	%rcx, 240(%rsp)
	movq	$-1, 208(%rsp)
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB346_153
.LBB346_144:
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
	jge	.LBB346_146
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_146:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_152
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_146
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
.LBB346_149:
	cmpq	%rax, %rsi
	jge	.LBB346_151
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB346_149
.LBB346_151:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_152:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB346_153:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB346_156
	lock		decq	(%rax)
	jne	.LBB346_156
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB346_156:
	vmovups	240(%rsp), %zmm1
	vmovups	208(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB346_157:
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
.LBB346_158:
	.cfi_def_cfa_offset 800
.Ltmp14306:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp14307:
	jmp	.LBB346_161
.LBB346_159:
.Ltmp14322:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp14323:
	jmp	.LBB346_161
.LBB346_160:
.Ltmp14331:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp14332:
.LBB346_161:
	ud2
.LBB346_162:
.Ltmp14336:
	movq	%rax, %rbx
	movb	$1, %bpl
	jmp	.LBB346_183
.LBB346_163:
.Ltmp14333:
	movq	%rax, %rbx
	jmp	.LBB346_182
.LBB346_164:
.Ltmp14324:
	movq	%rax, %rbx
	jmp	.LBB346_188
.LBB346_165:
.Ltmp14302:
	movq	%rax, %rbx
.Ltmp14303:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp14304:
	jmp	.LBB346_198
.LBB346_166:
.Ltmp14318:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB346_193
.LBB346_167:
.Ltmp14308:
	lock		decq	(%rbp)
	movq	%rax, %rbx
	jne	.LBB346_198
	#MEMBARRIER
.Ltmp14309:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	callq	*%rax
.Ltmp14310:
	jmp	.LBB346_198
.LBB346_169:
.Ltmp14279:
	jmp	.LBB346_171
.LBB346_170:
.Ltmp14282:
.LBB346_171:
	lock		decq	(%r13)
	movq	%rax, %rbx
	jne	.LBB346_173
	#MEMBARRIER
.Ltmp14283:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	callq	*%rax
.Ltmp14284:
.LBB346_173:
.Ltmp14286:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14287:
	jmp	.LBB346_198
.LBB346_174:
.Ltmp14285:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_175:
.Ltmp14311:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_176:
.Ltmp14346:
	movq	%rax, %rbx
	jmp	.LBB346_198
.LBB346_177:
.Ltmp14330:
	movq	%rax, %rbx
	cmpq	$5, %rbp
	ja	.LBB346_180
	jmp	.LBB346_181
.LBB346_178:
.Ltmp14327:
	movq	80(%rsp), %rbp
	movq	%rax, %rbx
	cmpq	$6, %rbp
	jb	.LBB346_181
	movq	88(%rsp), %r15
.LBB346_180:
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	movq	%r15, %rdi
	callq	__rustc::__rust_dealloc
.LBB346_181:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB346_182:
	xorl	%ebp, %ebp
.LBB346_183:
	cmpq	$0, 16(%rsp)
	je	.LBB346_185
	movq	8(%rsp), %rdi
	movq	40(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB346_185:
	testb	%bpl, %bpl
	je	.LBB346_188
	jmp	.LBB346_192
.LBB346_187:
.Ltmp14321:
	movq	8(%rsp), %rdi
	movl	$8, %edx
	movq	%r13, %rsi
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
.LBB346_188:
.Ltmp14337:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14338:
	movq	32(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB346_192
	#MEMBARRIER
.Ltmp14339:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	472(%rsp), %rdi
	callq	*%rax
.Ltmp14340:
.LBB346_192:
	xorl	%ebp, %ebp
.LBB346_193:
.Ltmp14342:
	leaq	480(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp14343:
	testb	%bpl, %bpl
	jne	.LBB346_198
	jmp	.LBB346_199
.LBB346_195:
.Ltmp14341:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_196:
.Ltmp14290:
	movq	%rax, %rbx
	movq	%r15, 216(%rsp)
.Ltmp14291:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp14292:
.Ltmp14294:
	leaq	208(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14295:
.LBB346_198:
.Ltmp14347:
	leaq	520(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14348:
.LBB346_199:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB346_200:
.Ltmp14305:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_201:
.Ltmp14293:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_202:
.Ltmp14349:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end346:

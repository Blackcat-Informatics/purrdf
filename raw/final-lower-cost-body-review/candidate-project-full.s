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
	leaq	536(%rsp), %rdi
	movq	%r9, %r14
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbp
	callq	*%rax
	movq	1136(%r14), %rax
	movq	%rbx, 216(%rsp)
	testq	%rax, %rax
	je	.LBB346_11
	movq	1128(%r14), %rcx
	movq	%rax, 56(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%r15, %r12
	shlq	$4, %r12
	movq	%r14, 32(%rsp)
	movq	%r15, 64(%rsp)
	movq	%r13, 40(%rsp)
	movq	%rbp, 384(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 376(%rsp)
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
	movq	%rax, 8(%rsp)
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
	movq	8(%rsp), %rax
	movq	40(%rsp), %r13
	cmpq	24(%rsp), %rax
	jne	.LBB346_3
	movq	32(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 224(%rsp)
	movq	$0, 240(%rsp)
	movq	$8, 248(%rsp)
	jmp	.LBB346_68
.LBB346_10:
	movq	64(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB346_37
	jmp	.LBB346_175
.LBB346_11:
.Ltmp14298:
	leaq	640(%rsp), %rdi
	movq	%rbp, %rsi
	movq	%r14, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14299:
	cmpl	$1, 640(%rsp)
	jne	.LBB346_73
.LBB346_13:
	vmovups	688(%rsp), %zmm1
	vmovups	656(%rsp), %zmm0
	movq	608(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB346_23
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	616(%rsp), %rdi
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
	movq	536(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB346_33
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	544(%rsp), %rdi
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
	movq	632(%rsp), %rax
	testq	%rax, %rax
	je	.LBB346_170
	lock		decq	(%rax)
	jne	.LBB346_170
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	632(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB346_170
.LBB346_36:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, 8(%rsp)
	lock		incq	(%rbp)
	jle	.LBB346_175
.LBB346_37:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 224(%rsp)
	movq	%r14, 232(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB346_172
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
	movq	%rax, 16(%rsp)
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
	movq	8(%rsp), %rax
	movq	%rbp, (%rsi)
	movq	$4, 96(%rsp)
	movq	%rsi, 104(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 112(%rsp)
	cmpq	24(%rsp), %rax
	je	.LBB346_62
	movl	$1, %r14d
	testq	%r15, %r15
	je	.LBB346_122
	movq	8(%rsp), %rbx
	jmp	.LBB346_51
	.p2align	4
.LBB346_50:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 112(%rsp)
	cmpq	24(%rsp), %rbx
	je	.LBB346_63
.LBB346_51:
	movq	%rsi, 48(%rsp)
.LBB346_52:
	movq	24(%rbx), %r13
	leaq	48(%rbx), %rax
	movq	32(%rbx), %rbp
	movq	40(%rsp), %rbx
	movq	%rax, 8(%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB346_55
	.p2align	4
.LBB346_53:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB346_57
.LBB346_54:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB346_58
.LBB346_55:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB346_53
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB346_54
.LBB346_57:
	movq	8(%rsp), %rbx
	cmpq	24(%rsp), %rbx
	jne	.LBB346_52
	jmp	.LBB346_63
	.p2align	4
.LBB346_58:
	lock		incq	(%r13)
	movq	48(%rsp), %rsi
	jle	.LBB346_175
	movq	8(%rsp), %rbx
	movq	%r13, 224(%rsp)
	movq	%rbp, 232(%rsp)
	cmpq	96(%rsp), %r14
	jne	.LBB346_50
.Ltmp14263:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	96(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.11631829254914579133)
.Ltmp14264:
	movq	104(%rsp), %rsi
	jmp	.LBB346_50
.LBB346_62:
	movl	$1, %r14d
.LBB346_63:
	movq	32(%rsp), %rbp
	movq	104(%rsp), %r15
	movq	96(%rsp), %rax
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %r13
	shlq	$4, %r14
	movq	1136(%rbp), %rbx
	movq	%r15, 224(%rsp)
	movq	%rax, 240(%rsp)
	addq	%r15, %r14
	addq	$16, %r15
	movq	%r14, 248(%rsp)
	movq	%rbx, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB346_65
	.p2align	4
.LBB346_64:
	vmovups	96(%rsp), %ymm0
	vmovups	112(%rsp), %ymm1
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
	movq	16(%rsp), %rax
	vmovups	%xmm0, 120(%rsp)
	movq	$2, 96(%rsp)
	movb	$0, 136(%rsp)
	cmpq	(%rax), %rbx
	jne	.LBB346_64
.Ltmp14274:
	movq	16(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp14275:
	jmp	.LBB346_64
.LBB346_67:
	movq	40(%rsp), %r13
.LBB346_68:
	movq	%r14, 232(%rsp)
.Ltmp14282:
	leaq	224(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14283:
	movq	216(%rsp), %rbx
	movq	64(%rsp), %r15
	movq	56(%rsp), %r14
.Ltmp14284:
	movq	384(%rsp), %rcx
	leaq	96(%rsp), %rdi
	movq	%rbp, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14285:
	movq	1136(%rbp), %rsi
	subq	%r14, %rsi
	jb	.LBB346_72
	movq	376(%rsp), %rdi
	addq	1128(%rbp), %rdi
	movq	%r14, 1136(%rbp)
.Ltmp14286:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp14287:
.LBB346_72:
	vmovups	144(%rsp), %zmm1
	vmovups	96(%rsp), %zmm0
	vmovups	%zmm1, 688(%rsp)
	vmovups	%zmm0, 640(%rsp)
	cmpl	$1, 640(%rsp)
	je	.LBB346_13
.LBB346_73:
	leaq	648(%rsp), %rcx
.Ltmp14300:
	leaq	96(%rsp), %rdi
	leaq	536(%rsp), %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp14301:
	cmpq	$-1, 96(%rsp)
	je	.LBB346_119
	vmovups	96(%rsp), %ymm0
	vmovups	%ymm0, 496(%rsp)
.Ltmp14302:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp14303:
	vmovups	576(%rsp), %zmm1
	vmovups	536(%rsp), %zmm0
	movq	%rax, 392(%rsp)
	movq	%rax, 56(%rsp)
	vmovups	%zmm1, 264(%rsp)
	vmovups	%zmm0, 224(%rsp)
	movq	32(%rax), %r15
	movq	%r15, %r13
	shlq	$4, %r13
	movq	%r15, 32(%rsp)
	testq	%r15, %r15
	je	.LBB346_128
	movq	24(%rax), %r14
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
	movq	%rax, 16(%rsp)
	testq	%rax, %rax
	je	.LBB346_173
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
	movq	520(%rsp), %rbx
	movq	%r15, %r12
	xorl	%r15d, %r15d
	addq	$16, %rbx
	.p2align	4
.LBB346_87:
	leaq	(%r14,%r15), %rsi
.Ltmp14305:
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.11631829254914579133)
.Ltmp14306:
	movq	16(%rsp), %rcx
	movq	%rax, (%rcx,%r15)
	movq	%rdx, 8(%rcx,%r15)
	addq	$16, %r15
	decq	%r12
	jne	.LBB346_87
	movq	32(%rsp), %r15
	movq	512(%rsp), %rbx
	movq	%r13, 8(%rsp)
	testq	%rbx, %rbx
	je	.LBB346_129
.LBB346_90:
	movq	malloc@GOTPCREL(%rip), %r14
	leaq	(,%rbx,8), %rax
	leaq	(%rax,%rax,4), %rbp
	movq	%rbp, %rdi
	vzeroupper
	callq	*%r14
	movq	%rax, 48(%rsp)
	testq	%rax, %rax
	je	.LBB346_174
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r8
	leaq	(%rbp,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%rbp, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r8, %rsi
	addq	%rbp, %rdi
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
	lock		addq	%rbp, (%rdx)
	movq	%rbp, %rdx
	lock		xaddq	%rdx, (%rsi)
	leaq	(%rdx,%rbp), %rax
	sarq	$63, %rax
	xorq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	addq	%rbp, %rdx
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
	movq	504(%rsp), %r12
	movq	48(%rsp), %rax
	movq	%rbx, 72(%rsp)
	leaq	112(%rsp), %rbx
	movq	$0, 24(%rsp)
	addq	%r12, %rbp
	movq	%rax, 80(%rsp)
	movq	$0, 88(%rsp)
	movq	%rbp, 64(%rsp)
	jmp	.LBB346_101
	.p2align	4
.LBB346_100:
	movq	24(%rsp), %rsi
	movq	48(%rsp), %rdx
	movq	40(%rsp), %rcx
	addq	$40, %r12
	leaq	(%rsi,%rsi,4), %rax
	incq	%rsi
	movq	%rsi, 24(%rsp)
	movq	%r15, (%rdx,%rax,8)
	movq	%rcx, 8(%rdx,%rax,8)
	movq	32(%rsp), %r15
	vmovaps	400(%rsp), %xmm0
	vmovups	%xmm0, 16(%rdx,%rax,8)
	movq	416(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rsi, 88(%rsp)
	cmpq	%rbp, %r12
	je	.LBB346_130
.LBB346_101:
	movq	$1, 96(%rsp)
	cmpq	$5, %r15
	jae	.LBB346_118
	testq	%r15, %r15
	je	.LBB346_114
.LBB346_103:
	xorl	%r15d, %r15d
	.p2align	4
.LBB346_104:
	movq	16(%rsp), %rax
	movl	$2, %ebp
	leaq	96(%rsp), %r8
	cmpb	$0, (%rax,%r15)
	je	.LBB346_109
	movq	(%r12), %rsi
	leaq	8(%r12), %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB346_107
	movq	16(%r12), %rsi
	movq	8(%r12), %rax
	decq	%rsi
.LBB346_107:
	movq	16(%rsp), %rcx
	movq	8(%rcx,%r15), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB346_171
	movl	(%rax,%rdi,8), %ebp
	movl	4(%rax,%rdi,8), %r14d
.LBB346_109:
	movq	96(%rsp), %rsi
	movq	104(%rsp), %rax
	xorl	%edx, %edx
	leaq	104(%rsp), %rcx
	movl	$4, %edi
	decq	%rsi
	cmpq	$5, %rsi
	setae	%dl
	cmovbq	%rcx, %rax
	movq	%r8, %rcx
	cmovaeq	%rbx, %rcx
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	96(%rsp,%rdx), %r13
	leaq	-1(%r13), %rdx
	cmpq	%rsi, %rdx
	je	.LBB346_111
.LBB346_110:
	movl	%ebp, -8(%rax,%r13,8)
	movl	%r14d, -4(%rax,%r13,8)
	incq	%r13
	addq	$16, %r15
	movq	%r13, (%rcx)
	movq	8(%rsp), %r13
	cmpq	%r15, %r13
	jne	.LBB346_104
	jmp	.LBB346_113
.LBB346_111:
.Ltmp14317:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	leaq	96(%rsp), %rdi
	movl	$1, %ecx
	callq	*%rax
.Ltmp14318:
	cmpq	$6, 96(%rsp)
	movq	104(%rsp), %rax
	leaq	104(%rsp), %rcx
	cmovbq	%rcx, %rax
	leaq	96(%rsp), %rcx
	cmovaeq	%rbx, %rcx
	jmp	.LBB346_110
	.p2align	4
.LBB346_113:
	movq	96(%rsp), %r15
	movq	64(%rsp), %rbp
	jmp	.LBB346_115
	.p2align	4
.LBB346_114:
	movl	$1, %r15d
.LBB346_115:
	vmovups	(%rbx), %xmm0
	movq	104(%rsp), %rax
	movq	16(%rbx), %rdx
	movq	24(%rsp), %rcx
	movq	%rax, 40(%rsp)
	movq	%rdx, 416(%rsp)
	vmovaps	%xmm0, 400(%rsp)
	cmpq	72(%rsp), %rcx
	jne	.LBB346_100
.Ltmp14320:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	72(%rsp), %rdi
	callq	*%rax
.Ltmp14321:
	movq	80(%rsp), %rax
	movq	%rax, 48(%rsp)
	jmp	.LBB346_100
.LBB346_118:
.Ltmp14311:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	movl	$1, %ecx
	xorl	%esi, %esi
	movq	%r15, %rdx
	callq	*%rax
.Ltmp14312:
	jmp	.LBB346_103
.LBB346_119:
.Ltmp14336:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14337:
	vmovups	576(%rsp), %zmm1
	vmovups	536(%rsp), %zmm0
	movq	%rax, 424(%rsp)
	movq	$0, 400(%rsp)
	movq	$8, 408(%rsp)
	movq	$0, 416(%rsp)
	vmovups	%zmm1, 136(%rsp)
	vmovups	%zmm0, 96(%rsp)
	cmpq	$-1, 96(%rsp)
	je	.LBB346_156
	leaq	224(%rsp), %rdi
	leaq	400(%rsp), %rsi
	leaq	536(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	168(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB346_157
	jmp	.LBB346_166
.LBB346_122:
	movq	8(%rsp), %rax
	movl	$24, %ebx
	leaq	96(%rsp), %rbp
	jmp	.LBB346_125
	.p2align	4
.LBB346_123:
	movq	104(%rsp), %rsi
.LBB346_124:
	addq	$48, %r12
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %rax
	movq	%r14, 112(%rsp)
	cmpq	24(%rsp), %r12
	je	.LBB346_63
.LBB346_125:
	movq	24(%rax), %r13
	movq	32(%rax), %r15
	lock		incq	(%r13)
	jle	.LBB346_175
	movq	%r13, 224(%rsp)
	movq	%rax, %r12
	movq	%r15, 232(%rsp)
	cmpq	96(%rsp), %r14
	jne	.LBB346_124
.Ltmp14266:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.11631829254914579133)
.Ltmp14267:
	jmp	.LBB346_123
.LBB346_128:
	movl	$8, %eax
	movq	%rax, 16(%rsp)
	movq	512(%rsp), %rbx
	movq	%r13, 8(%rsp)
	testq	%rbx, %rbx
	jne	.LBB346_90
.LBB346_129:
	movq	$0, 72(%rsp)
	movq	$8, 80(%rsp)
	movq	$0, 88(%rsp)
.LBB346_130:
	vmovups	264(%rsp), %zmm1
	vmovups	224(%rsp), %zmm0
	movq	72(%rsp), %rcx
	movq	88(%rsp), %rax
	movq	80(%rsp), %rdx
	movq	%rcx, 336(%rsp)
	movq	56(%rsp), %rcx
	movq	%rax, 352(%rsp)
	movq	%rdx, 344(%rsp)
	vmovups	%zmm1, 136(%rsp)
	vmovups	%zmm0, 96(%rsp)
	cmpq	$-1, 96(%rsp)
	movq	%rcx, 360(%rsp)
	je	.LBB346_133
	leaq	400(%rsp), %rdi
	leaq	336(%rsp), %rsi
	leaq	224(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	216(%rsp), %rbx
	movq	168(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB346_132
.LBB346_134:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	176(%rsp), %rdi
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
	jge	.LBB346_136
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_136:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_142
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_136
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
.LBB346_139:
	cmpq	%rax, %rsi
	jge	.LBB346_141
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB346_139
.LBB346_141:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_142:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	192(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB346_143
	jmp	.LBB346_145
.LBB346_133:
	movq	344(%rsp), %rcx
	movq	336(%rsp), %rax
	movq	352(%rsp), %rdx
	movq	%rcx, 416(%rsp)
	movq	360(%rsp), %rcx
	movq	%rax, 408(%rsp)
	movq	%rdx, 424(%rsp)
	movq	%rcx, 432(%rsp)
	movq	$-1, 400(%rsp)
	movq	216(%rsp), %rbx
	movq	168(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB346_134
.LBB346_132:
	movq	192(%rsp), %rax
	testq	%rax, %rax
	je	.LBB346_145
.LBB346_143:
	lock		decq	(%rax)
	jne	.LBB346_145
	leaq	192(%rsp), %rdi
	#MEMBARRIER
.Ltmp14326:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp14327:
.LBB346_145:
	vmovups	432(%rsp), %zmm1
	vmovups	400(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	testq	%r15, %r15
	je	.LBB346_155
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rcx
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
	jge	.LBB346_148
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_148:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_154
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_148
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
.LBB346_151:
	cmpq	%rax, %rdx
	jge	.LBB346_153
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB346_151
.LBB346_153:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_154:
	movq	free@GOTPCREL(%rip), %rax
	movq	16(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB346_155:
	leaq	496(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB346_170
.LBB346_156:
	movq	408(%rsp), %rcx
	movq	400(%rsp), %rax
	movq	416(%rsp), %rdx
	movq	%rcx, 240(%rsp)
	movq	424(%rsp), %rcx
	movq	%rax, 232(%rsp)
	movq	%rdx, 248(%rsp)
	movq	%rcx, 256(%rsp)
	movq	$-1, 224(%rsp)
	movq	168(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB346_166
.LBB346_157:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	176(%rsp), %rdi
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
	jge	.LBB346_159
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB346_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB346_165
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB346_159
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
.LBB346_162:
	cmpq	%rax, %rsi
	jge	.LBB346_164
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB346_162
.LBB346_164:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB346_165:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB346_166:
	movq	192(%rsp), %rax
	testq	%rax, %rax
	je	.LBB346_169
	lock		decq	(%rax)
	jne	.LBB346_169
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	192(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB346_169:
	vmovups	256(%rsp), %zmm1
	vmovups	224(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB346_170:
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
.LBB346_171:
	.cfi_def_cfa_offset 816
.Ltmp14314:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.757(%rip), %rdx
	callq	*%rax
.Ltmp14315:
	jmp	.LBB346_175
.LBB346_172:
.Ltmp14292:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp14293:
	jmp	.LBB346_175
.LBB346_173:
.Ltmp14308:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp14309:
	jmp	.LBB346_175
.LBB346_174:
.Ltmp14323:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%rbp, %rsi
	callq	*%rax
.Ltmp14324:
.LBB346_175:
	ud2
.LBB346_176:
.Ltmp14313:
	jmp	.LBB346_200
.LBB346_177:
.Ltmp14319:
	jmp	.LBB346_200
.LBB346_178:
.Ltmp14328:
	movq	%rax, %rbx
	movb	$1, %bpl
	jmp	.LBB346_205
.LBB346_179:
.Ltmp14325:
	movq	%rax, %rbx
	jmp	.LBB346_204
.LBB346_180:
.Ltmp14310:
	movq	%rax, %rbx
	jmp	.LBB346_209
.LBB346_181:
.Ltmp14288:
	movq	%rax, %rbx
.Ltmp14289:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp14290:
	jmp	.LBB346_216
.LBB346_182:
.Ltmp14304:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB346_214
.LBB346_183:
.Ltmp14294:
	lock		decq	(%rbp)
	movq	%rax, %rbx
	jne	.LBB346_216
	#MEMBARRIER
.Ltmp14295:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	callq	*%rax
.Ltmp14296:
	jmp	.LBB346_216
.LBB346_185:
.Ltmp14265:
	jmp	.LBB346_187
.LBB346_186:
.Ltmp14268:
.LBB346_187:
	lock		decq	(%r13)
	movq	%rax, %rbx
	jne	.LBB346_189
	#MEMBARRIER
.Ltmp14269:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	224(%rsp), %rdi
	callq	*%rax
.Ltmp14270:
.LBB346_189:
.Ltmp14272:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14273:
	jmp	.LBB346_216
.LBB346_190:
.Ltmp14271:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_191:
.Ltmp14297:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_192:
.Ltmp14338:
	movq	%rax, %rbx
	jmp	.LBB346_216
.LBB346_193:
.Ltmp14322:
	movq	%rax, %rbx
	cmpq	$5, %r15
	ja	.LBB346_202
	jmp	.LBB346_203
.LBB346_194:
.Ltmp14307:
	movq	16(%rsp), %rdi
	movl	$8, %edx
	movq	%r13, %rsi
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
	jmp	.LBB346_209
.LBB346_195:
.Ltmp14276:
	movq	%rax, %rbx
	movq	%r15, 232(%rsp)
.Ltmp14277:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp14278:
.Ltmp14280:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14281:
	jmp	.LBB346_216
.LBB346_197:
.Ltmp14291:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_198:
.Ltmp14279:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_199:
.Ltmp14316:
.LBB346_200:
	movq	96(%rsp), %r15
	movq	%rax, %rbx
	cmpq	$6, %r15
	jb	.LBB346_203
	movq	104(%rsp), %rax
	movq	%rax, 40(%rsp)
.LBB346_202:
	movq	40(%rsp), %rdi
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB346_203:
	leaq	72(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB346_204:
	xorl	%ebp, %ebp
.LBB346_205:
	cmpq	$0, 32(%rsp)
	je	.LBB346_207
	movq	16(%rsp), %rdi
	movq	8(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB346_207:
	testb	%bpl, %bpl
	jne	.LBB346_213
.LBB346_209:
.Ltmp14329:
	leaq	224(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14330:
	movq	56(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB346_213
	#MEMBARRIER
.Ltmp14331:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	392(%rsp), %rdi
	callq	*%rax
.Ltmp14332:
.LBB346_213:
	xorl	%ebp, %ebp
.LBB346_214:
.Ltmp14334:
	leaq	496(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp14335:
	testb	%bpl, %bpl
	je	.LBB346_217
.LBB346_216:
.Ltmp14339:
	leaq	536(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14340:
.LBB346_217:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB346_218:
.Ltmp14333:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB346_219:
.Ltmp14341:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end346:

purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin348:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception254
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
	subq	$776, %rsp
	.cfi_def_cfa_offset 832
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	552(%rsp), %rdi
	movq	%r9, %r14
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbp
	callq	*%rax
	movq	1136(%r14), %rax
	movq	%rbx, 224(%rsp)
	testq	%rax, %rax
	je	.LBB348_11
	movq	1128(%r14), %rcx
	movq	%rax, 232(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%r15, %r12
	shlq	$4, %r12
	movq	%r14, 16(%rsp)
	movq	%r15, 80(%rsp)
	movq	%r13, 8(%rsp)
	movq	%rbp, 56(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 72(%rsp)
	addq	%rcx, %rax
	movq	%rax, 24(%rsp)
	testq	%r15, %r15
	je	.LBB348_36
	movq	bcmp@GOTPCREL(%rip), %rbx
	movq	%rcx, %rax
.LBB348_3:
	movq	24(%rax), %rbp
	movq	32(%rax), %r14
	addq	$48, %rax
	movq	%rax, 32(%rsp)
	leaq	16(%rbp), %r15
	jmp	.LBB348_6
	.p2align	4
.LBB348_4:
	xorq	%r14, %rax
	xorq	%rbp, %rdi
	orq	%rax, %rdi
	je	.LBB348_8
.LBB348_5:
	addq	$16, %r13
	cmpq	%r12, %r13
	je	.LBB348_10
.LBB348_6:
	movq	(%r13), %rdi
	movq	8(%r13), %rax
	cmpq	%rbp, %rdi
	sete	%cl
	cmpq	%r14, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB348_4
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%rbx
	testl	%eax, %eax
	jne	.LBB348_5
.LBB348_8:
	movq	32(%rsp), %rax
	movq	8(%rsp), %r13
	cmpq	24(%rsp), %rax
	jne	.LBB348_3
	movq	16(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 256(%rsp)
	movq	$0, 272(%rsp)
	movq	$8, 280(%rsp)
	jmp	.LBB348_68
.LBB348_10:
	movq	80(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB348_37
	jmp	.LBB348_190
.LBB348_11:
.Ltmp14235:
	leaq	656(%rsp), %rdi
	movq	%rbp, %rsi
	movq	%r14, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14236:
	cmpl	$1, 656(%rsp)
	jne	.LBB348_73
.LBB348_13:
	vmovups	704(%rsp), %zmm1
	vmovups	672(%rsp), %zmm0
	movq	624(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB348_23
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	632(%rsp), %rdi
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
	jge	.LBB348_16
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_16:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_22
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_16
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
.LBB348_19:
	cmpq	%rax, %rsi
	jge	.LBB348_21
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_19
.LBB348_21:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_22:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_23:
	movq	552(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB348_33
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	560(%rsp), %rdi
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
	jge	.LBB348_26
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_26:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_32
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_26
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
.LBB348_29:
	cmpq	%rax, %rsi
	jge	.LBB348_31
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_29
.LBB348_31:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_32:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_33:
	movq	648(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_184
	lock		decq	(%rax)
	jne	.LBB348_184
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	648(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB348_184
.LBB348_36:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, 32(%rsp)
	lock		incq	(%rbp)
	jle	.LBB348_190
.LBB348_37:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 256(%rsp)
	movq	%r14, 264(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB348_187
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
	jle	.LBB348_40
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB348_40:
	movq	16(%rsp), %rax
	addq	$1120, %rax
	movq	%rax, 240(%rsp)
	.p2align	4
.LBB348_41:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_47
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_41
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
.LBB348_44:
	cmpq	%rax, %rdx
	jle	.LBB348_46
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_44
.LBB348_46:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_47:
	movq	32(%rsp), %rax
	movq	%rbp, (%rsi)
	movq	$4, 112(%rsp)
	movq	%rsi, 120(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 128(%rsp)
	cmpq	24(%rsp), %rax
	je	.LBB348_62
	movl	$1, %r14d
	testq	%r15, %r15
	je	.LBB348_135
	movq	32(%rsp), %rbx
	jmp	.LBB348_52
	.p2align	4
.LBB348_50:
	movq	120(%rsp), %rsi
.LBB348_51:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 128(%rsp)
	cmpq	24(%rsp), %rbx
	je	.LBB348_63
.LBB348_52:
	movq	%rsi, 64(%rsp)
.LBB348_53:
	movq	24(%rbx), %r13
	leaq	48(%rbx), %rax
	movq	32(%rbx), %rbp
	movq	8(%rsp), %rbx
	movq	%rax, 32(%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB348_56
	.p2align	4
.LBB348_54:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB348_58
.LBB348_55:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB348_59
.LBB348_56:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB348_54
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB348_55
.LBB348_58:
	movq	32(%rsp), %rbx
	cmpq	24(%rsp), %rbx
	jne	.LBB348_53
	jmp	.LBB348_63
	.p2align	4
.LBB348_59:
	lock		incq	(%r13)
	movq	64(%rsp), %rsi
	jle	.LBB348_190
	movq	32(%rsp), %rbx
	movq	%r13, 256(%rsp)
	movq	%rbp, 264(%rsp)
	cmpq	112(%rsp), %r14
	jne	.LBB348_51
.Ltmp14200:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	112(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)
.Ltmp14201:
	jmp	.LBB348_50
.LBB348_62:
	movl	$1, %r14d
.LBB348_63:
	movq	16(%rsp), %rbp
	movq	120(%rsp), %r15
	movq	112(%rsp), %rax
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %r13
	shlq	$4, %r14
	movq	1136(%rbp), %rbx
	movq	%r15, 256(%rsp)
	movq	%rax, 272(%rsp)
	addq	%r15, %r14
	addq	$16, %r15
	movq	%r14, 280(%rsp)
	movq	%rbx, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB348_65
	.p2align	4
.LBB348_64:
	vmovdqu	112(%rsp), %ymm0
	vmovups	128(%rsp), %ymm1
	movq	1128(%rbp), %rcx
	leaq	-16(%r15), %rax
	incq	%rbx
	addq	$16, %r15
	addq	$16, %rax
	vmovups	%ymm1, 16(%rcx,%r12)
	vmovdqu	%ymm0, (%rcx,%r12)
	addq	$48, %r12
	movq	%rbx, 1136(%rbp)
	cmpq	%r14, %rax
	je	.LBB348_67
.LBB348_65:
	vmovups	-16(%r15), %xmm0
	movq	240(%rsp), %rax
	vmovups	%xmm0, 136(%rsp)
	movq	$2, 112(%rsp)
	movb	$0, 152(%rsp)
	cmpq	(%rax), %rbx
	jne	.LBB348_64
.Ltmp14211:
	movq	240(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp14212:
	jmp	.LBB348_64
.LBB348_67:
	movq	8(%rsp), %r13
.LBB348_68:
	movq	%r14, 264(%rsp)
.Ltmp14219:
	leaq	256(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14220:
	movq	224(%rsp), %rbx
	movq	80(%rsp), %r15
	movq	232(%rsp), %r14
.Ltmp14221:
	movq	56(%rsp), %rcx
	leaq	112(%rsp), %rdi
	movq	%rbp, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14222:
	movq	1136(%rbp), %rsi
	subq	%r14, %rsi
	jb	.LBB348_72
	movq	72(%rsp), %rdi
	addq	1128(%rbp), %rdi
	movq	%r14, 1136(%rbp)
.Ltmp14223:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp14224:
.LBB348_72:
	vmovups	160(%rsp), %zmm1
	vmovdqu64	112(%rsp), %zmm0
	vmovups	%zmm1, 704(%rsp)
	vmovdqu64	%zmm0, 656(%rsp)
	cmpl	$1, 656(%rsp)
	je	.LBB348_13
.LBB348_73:
	leaq	664(%rsp), %rcx
.Ltmp14237:
	leaq	112(%rsp), %rdi
	leaq	552(%rsp), %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp14238:
	cmpq	$-1, 112(%rsp)
	je	.LBB348_132
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 512(%rsp)
.Ltmp14239:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp14240:
	vmovups	592(%rsp), %zmm1
	vmovdqu64	552(%rsp), %zmm0
	movq	%rax, 408(%rsp)
	movq	%rax, %r15
	movq	%r15, 72(%rsp)
	vmovups	%zmm1, 296(%rsp)
	vmovdqu64	%zmm0, 256(%rsp)
	movq	32(%rax), %rax
	movq	%rax, %r13
	shlq	$4, %r13
	movq	%rax, 16(%rsp)
	testq	%rax, %rax
	je	.LBB348_141
	movq	malloc@GOTPCREL(%rip), %rax
	movq	24(%r15), %r14
	movq	%r13, %rdi
	vzeroupper
	callq	*%rax
	movq	%rax, 8(%rsp)
	testq	%rax, %rax
	je	.LBB348_188
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
	jle	.LBB348_80
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_80:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_86
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_80
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
.LBB348_83:
	cmpq	%rax, %rdx
	jle	.LBB348_85
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_83
.LBB348_85:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_86:
	movq	536(%rsp), %rbx
	movq	16(%rsp), %r12
	xorl	%r15d, %r15d
	addq	$16, %rbx
	.p2align	4
.LBB348_87:
	leaq	(%r14,%r15), %rsi
.Ltmp14242:
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.10720091597982897309)
.Ltmp14243:
	movq	8(%rsp), %rcx
	movq	%rax, (%rcx,%r15)
	movq	%rdx, 8(%rcx,%r15)
	addq	$16, %r15
	decq	%r12
	jne	.LBB348_87
	movq	528(%rsp), %rbx
	movq	%r13, 56(%rsp)
	testq	%rbx, %rbx
	je	.LBB348_142
.LBB348_90:
	movq	malloc@GOTPCREL(%rip), %r14
	leaq	(,%rbx,8), %rax
	leaq	(%rax,%rax,4), %r15
	movq	%r15, %rdi
	vzeroupper
	callq	*%r14
	movq	%rax, 64(%rsp)
	testq	%rax, %rax
	je	.LBB348_189
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
	jle	.LBB348_93
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_93:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_99
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_93
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
.LBB348_96:
	cmpq	%rax, %rdx
	jle	.LBB348_98
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_96
.LBB348_98:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_99:
	movq	64(%rsp), %rax
	movq	8(%rsp), %rdx
	movq	%r13, %rcx
	movq	520(%rsp), %r13
	movq	%rbx, 88(%rsp)
	movq	%rcx, %rbx
	negq	%rbx
	leaq	128(%rsp), %r14
	xorl	%r12d, %r12d
	movq	%rbx, 80(%rsp)
	movq	%rax, 96(%rsp)
	leaq	(%rdx,%rcx), %rax
	movl	$2, %ecx
	addq	%r13, %r15
	movq	$0, 104(%rsp)
	vmovd	%ecx, %xmm0
	movq	%rax, 24(%rsp)
	movq	%r15, 232(%rsp)
	vmovdqa	%xmm0, 32(%rsp)
	jmp	.LBB348_102
	.p2align	4
.LBB348_100:
	movq	96(%rsp), %rax
	movq	%rax, 64(%rsp)
.LBB348_101:
	movq	64(%rsp), %rdx
	leaq	(%r12,%r12,4), %rax
	addq	$40, %r13
	incq	%r12
	movq	%rbp, (%rdx,%rax,8)
	movq	%r15, 8(%rdx,%rax,8)
	movq	232(%rsp), %r15
	vmovdqa	416(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	432(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%r12, 104(%rsp)
	cmpq	%r15, %r13
	je	.LBB348_143
.LBB348_102:
	cmpq	$5, 16(%rsp)
	movq	$1, 112(%rsp)
	jae	.LBB348_130
	xorl	%edx, %edx
.LBB348_104:
	xorl	%r15d, %r15d
	movl	$4, %eax
	leaq	120(%rsp), %rcx
	cmpq	$5, %rdx
	setae	%sil
	jb	.LBB348_106
	movq	120(%rsp), %rcx
	movq	%rdx, %rax
.LBB348_106:
	movb	%sil, %r15b
	shll	$4, %r15d
	movq	112(%rsp,%r15), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rax, %rdx
	jae	.LBB348_115
	movq	8(%rsp), %r9
	leaq	8(%r13), %rdx
	incq	%rax
	xorl	%r8d, %r8d
	jmp	.LBB348_111
	.p2align	4
.LBB348_108:
	movq	8(%r9), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB348_186
	vmovq	(%r10,%rdi,8), %xmm0
.LBB348_110:
	vmovq	%xmm0, -8(%rcx,%rbp,8)
	addq	$16, %r9
	incq	%rbp
	addq	$-16, %r8
	cmpq	%rbp, %rax
	je	.LBB348_117
.LBB348_111:
	cmpq	%r8, %rbx
	je	.LBB348_116
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%r9)
	jne	.LBB348_110
	movq	(%r13), %rsi
	movq	%rdx, %r10
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB348_108
	movq	16(%r13), %rsi
	movq	8(%r13), %r10
	decq	%rsi
	jmp	.LBB348_108
	.p2align	4
.LBB348_115:
	movq	8(%rsp), %rbx
	movq	%rbp, %rax
	movq	%rax, 112(%rsp,%r15)
	cmpq	24(%rsp), %rbx
	jne	.LBB348_118
	jmp	.LBB348_128
	.p2align	4
.LBB348_116:
	movq	%rbp, 112(%rsp,%r15)
	jmp	.LBB348_128
	.p2align	4
.LBB348_117:
	movq	8(%rsp), %rbx
	subq	%r8, %rbx
	movq	%rax, 112(%rsp,%r15)
	cmpq	24(%rsp), %rbx
	je	.LBB348_128
.LBB348_118:
	leaq	8(%r13), %r15
	.p2align	4
.LBB348_119:
	vmovdqa	32(%rsp), %xmm0
	cmpl	$1, (%rbx)
	vmovdqa	%xmm0, 240(%rsp)
	jne	.LBB348_124
	movq	(%r13), %rsi
	movq	%r15, %rax
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB348_122
	movq	16(%r13), %rsi
	movq	8(%r13), %rax
	decq	%rsi
.LBB348_122:
	movq	8(%rbx), %rdi
	cmpq	%rsi, %rdi
	jae	.LBB348_185
	vmovq	(%rax,%rdi,8), %xmm0
	vmovdqa	%xmm0, 240(%rsp)
.LBB348_124:
	movq	112(%rsp), %rsi
	movq	120(%rsp), %rax
	xorl	%edx, %edx
	leaq	120(%rsp), %rcx
	movl	$4, %edi
	decq	%rsi
	cmpq	$5, %rsi
	cmovbq	%rcx, %rax
	leaq	112(%rsp), %rcx
	setae	%dl
	cmovbq	%rdi, %rsi
	cmovaeq	%r14, %rcx
	shll	$4, %edx
	movq	112(%rsp,%rdx), %rbp
	leaq	-1(%rbp), %rdx
	cmpq	%rsi, %rdx
	je	.LBB348_126
.LBB348_125:
	vmovdqa	240(%rsp), %xmm0
	addq	$16, %rbx
	vmovq	%xmm0, -8(%rax,%rbp,8)
	incq	%rbp
	movq	%rbp, (%rcx)
	cmpq	24(%rsp), %rbx
	jne	.LBB348_119
	jmp	.LBB348_128
.LBB348_126:
.Ltmp14257:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	leaq	112(%rsp), %rdi
	movl	$1, %ecx
	callq	*%rax
.Ltmp14258:
	cmpq	$6, 112(%rsp)
	movq	120(%rsp), %rax
	leaq	120(%rsp), %rcx
	cmovbq	%rcx, %rax
	leaq	112(%rsp), %rcx
	cmovaeq	%r14, %rcx
	jmp	.LBB348_125
	.p2align	4
.LBB348_128:
	vmovdqu	(%r14), %xmm0
	movq	16(%r14), %rax
	movq	112(%rsp), %rbp
	movq	120(%rsp), %r15
	movq	80(%rsp), %rbx
	movq	%rax, 432(%rsp)
	vmovdqa	%xmm0, 416(%rsp)
	cmpq	88(%rsp), %r12
	jne	.LBB348_101
.Ltmp14260:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	88(%rsp), %rdi
	callq	*%rax
.Ltmp14261:
	jmp	.LBB348_100
.LBB348_130:
.Ltmp14248:
	movq	16(%rsp), %rdx
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	112(%rsp), %rdi
	movl	$1, %ecx
	xorl	%esi, %esi
	callq	*%rax
.Ltmp14249:
	movq	112(%rsp), %rdx
	decq	%rdx
	jmp	.LBB348_104
.LBB348_132:
.Ltmp14276:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14277:
	vmovups	592(%rsp), %zmm1
	vmovups	552(%rsp), %zmm0
	movq	%rax, 440(%rsp)
	movq	$0, 416(%rsp)
	movq	$8, 424(%rsp)
	movq	$0, 432(%rsp)
	vmovups	%zmm1, 152(%rsp)
	vmovups	%zmm0, 112(%rsp)
	cmpq	$-1, 112(%rsp)
	je	.LBB348_170
	leaq	256(%rsp), %rdi
	leaq	416(%rsp), %rsi
	leaq	552(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB348_171
	jmp	.LBB348_180
.LBB348_135:
	movq	32(%rsp), %rax
	movl	$24, %ebx
	leaq	112(%rsp), %rbp
	jmp	.LBB348_137
	.p2align	4
.LBB348_136:
	addq	$48, %r12
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %rax
	movq	%r14, 128(%rsp)
	cmpq	24(%rsp), %r12
	je	.LBB348_63
.LBB348_137:
	movq	24(%rax), %r13
	movq	32(%rax), %r15
	lock		incq	(%r13)
	jle	.LBB348_190
	movq	%r13, 256(%rsp)
	movq	%rax, %r12
	movq	%r15, 264(%rsp)
	cmpq	112(%rsp), %r14
	jne	.LBB348_136
.Ltmp14203:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)
.Ltmp14204:
	movq	120(%rsp), %rsi
	jmp	.LBB348_136
.LBB348_141:
	movl	$8, %eax
	movq	%rax, 8(%rsp)
	movq	528(%rsp), %rbx
	movq	%r13, 56(%rsp)
	testq	%rbx, %rbx
	jne	.LBB348_90
.LBB348_142:
	movq	$0, 88(%rsp)
	movq	$8, 96(%rsp)
	movq	$0, 104(%rsp)
.LBB348_143:
	vmovups	296(%rsp), %zmm1
	vmovdqu64	256(%rsp), %zmm0
	movq	88(%rsp), %rcx
	movq	104(%rsp), %rax
	movq	96(%rsp), %rdx
	movq	%rcx, 368(%rsp)
	movq	72(%rsp), %rcx
	movq	%rax, 384(%rsp)
	movq	%rdx, 376(%rsp)
	vmovups	%zmm1, 152(%rsp)
	vmovdqu64	%zmm0, 112(%rsp)
	cmpq	$-1, 112(%rsp)
	movq	%rcx, 392(%rsp)
	je	.LBB348_145
	leaq	416(%rsp), %rdi
	leaq	368(%rsp), %rsi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB348_146
.LBB348_145:
	movq	376(%rsp), %rcx
	movq	368(%rsp), %rax
	movq	384(%rsp), %rdx
	movq	%rcx, 432(%rsp)
	movq	392(%rsp), %rcx
	movq	%rax, 424(%rsp)
	movq	%rdx, 440(%rsp)
	movq	%rcx, 448(%rsp)
	movq	$-1, 416(%rsp)
.LBB348_146:
	movq	224(%rsp), %rbx
	movq	56(%rsp), %r14
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB348_156
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB348_149
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_149:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_155
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_149
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
.LBB348_152:
	cmpq	%rax, %rsi
	jge	.LBB348_154
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_152
.LBB348_154:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_155:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_156:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_159
	lock		decq	(%rax)
	jne	.LBB348_159
	leaq	208(%rsp), %rdi
	#MEMBARRIER
.Ltmp14266:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp14267:
.LBB348_159:
	vmovups	448(%rsp), %zmm1
	vmovups	416(%rsp), %zmm0
	cmpq	$0, 16(%rsp)
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	je	.LBB348_169
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
	jge	.LBB348_162
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_162:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_168
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_162
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
.LBB348_165:
	cmpq	%rax, %rdx
	jge	.LBB348_167
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_165
.LBB348_167:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_168:
	movq	free@GOTPCREL(%rip), %rax
	movq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB348_169:
	leaq	512(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB348_184
.LBB348_170:
	movq	424(%rsp), %rcx
	movq	416(%rsp), %rax
	movq	432(%rsp), %rdx
	movq	%rcx, 272(%rsp)
	movq	440(%rsp), %rcx
	movq	%rax, 264(%rsp)
	movq	%rdx, 280(%rsp)
	movq	%rcx, 288(%rsp)
	movq	$-1, 256(%rsp)
	movq	184(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB348_180
.LBB348_171:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB348_173
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_173:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_179
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_173
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
.LBB348_176:
	cmpq	%rax, %rsi
	jge	.LBB348_178
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_176
.LBB348_178:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_179:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_180:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_183
	lock		decq	(%rax)
	jne	.LBB348_183
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB348_183:
	vmovups	288(%rsp), %zmm1
	vmovups	256(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB348_184:
	movq	%rbx, %rax
	addq	$776, %rsp
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
.LBB348_185:
	.cfi_def_cfa_offset 832
.Ltmp14254:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	callq	*%rax
.Ltmp14255:
	jmp	.LBB348_190
.LBB348_186:
.Ltmp14251:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.791(%rip), %rdx
	callq	*%rax
.Ltmp14252:
	jmp	.LBB348_190
.LBB348_187:
.Ltmp14229:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp14230:
	jmp	.LBB348_190
.LBB348_188:
.Ltmp14245:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp14246:
	jmp	.LBB348_190
.LBB348_189:
.Ltmp14263:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14264:
.LBB348_190:
	ud2
.LBB348_191:
.Ltmp14250:
	jmp	.LBB348_216
.LBB348_192:
.Ltmp14259:
	jmp	.LBB348_216
.LBB348_193:
.Ltmp14268:
	movq	%rax, %rbx
	movb	$1, %bpl
	jmp	.LBB348_222
.LBB348_194:
.Ltmp14265:
	movq	%rax, %rbx
	jmp	.LBB348_221
.LBB348_195:
.Ltmp14247:
	movq	%rax, %rbx
	jmp	.LBB348_226
.LBB348_196:
.Ltmp14225:
	movq	%rax, %rbx
.Ltmp14226:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp14227:
	jmp	.LBB348_233
.LBB348_197:
.Ltmp14241:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB348_231
.LBB348_198:
.Ltmp14231:
	lock		decq	(%rbp)
	movq	%rax, %rbx
	jne	.LBB348_233
	#MEMBARRIER
.Ltmp14232:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	callq	*%rax
.Ltmp14233:
	jmp	.LBB348_233
.LBB348_200:
.Ltmp14202:
	jmp	.LBB348_202
.LBB348_201:
.Ltmp14205:
.LBB348_202:
	lock		decq	(%r13)
	movq	%rax, %rbx
	jne	.LBB348_204
	#MEMBARRIER
.Ltmp14206:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdi
	callq	*%rax
.Ltmp14207:
.LBB348_204:
.Ltmp14209:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14210:
	jmp	.LBB348_233
.LBB348_205:
.Ltmp14208:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_206:
.Ltmp14234:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_207:
.Ltmp14278:
	movq	%rax, %rbx
	jmp	.LBB348_233
.LBB348_208:
.Ltmp14262:
	movq	%rax, %rbx
	cmpq	$5, %rbp
	ja	.LBB348_219
	jmp	.LBB348_220
.LBB348_209:
.Ltmp14244:
	movq	8(%rsp), %rdi
	movl	$8, %edx
	movq	%r13, %rsi
	movq	%rax, %rbx
	callq	__rustc::__rust_dealloc
	jmp	.LBB348_226
.LBB348_210:
.Ltmp14213:
	movq	%rax, %rbx
	movq	%r15, 264(%rsp)
.Ltmp14214:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp14215:
.Ltmp14217:
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14218:
	jmp	.LBB348_233
.LBB348_212:
.Ltmp14228:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_213:
.Ltmp14216:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_214:
.Ltmp14253:
	movq	%rax, %rbx
	movq	%rbp, 112(%rsp,%r15)
	jmp	.LBB348_217
.LBB348_215:
.Ltmp14256:
.LBB348_216:
	movq	%rax, %rbx
.LBB348_217:
	movq	112(%rsp), %rbp
	cmpq	$6, %rbp
	jb	.LBB348_220
	movq	120(%rsp), %r15
.LBB348_219:
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	movq	%r15, %rdi
	callq	__rustc::__rust_dealloc
.LBB348_220:
	leaq	88(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB348_221:
	xorl	%ebp, %ebp
.LBB348_222:
	cmpq	$0, 16(%rsp)
	je	.LBB348_224
	movq	8(%rsp), %rdi
	movq	56(%rsp), %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB348_224:
	testb	%bpl, %bpl
	jne	.LBB348_230
.LBB348_226:
.Ltmp14269:
	leaq	256(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14270:
	movq	72(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB348_230
	#MEMBARRIER
.Ltmp14271:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp14272:
.LBB348_230:
	xorl	%ebp, %ebp
.LBB348_231:
.Ltmp14274:
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp14275:
	testb	%bpl, %bpl
	je	.LBB348_234
.LBB348_233:
.Ltmp14279:
	leaq	552(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14280:
.LBB348_234:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB348_235:
.Ltmp14273:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_236:
.Ltmp14281:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end348:

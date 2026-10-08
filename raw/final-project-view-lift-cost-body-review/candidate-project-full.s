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
	subq	$680, %rsp
	.cfi_def_cfa_offset 736
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbp
	leaq	456(%rsp), %rdi
	movq	%r9, %r14
	movq	%r8, %rbx
	movq	%rcx, %r13
	movq	%rdx, %r15
	callq	*%rax
	movq	1136(%r14), %rax
	movq	%rbp, 216(%rsp)
	testq	%rax, %rax
	je	.LBB348_11
	movq	1128(%r14), %rcx
	movq	%rax, 224(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%rbx, %r12
	shlq	$4, %r12
	movq	%r15, 232(%rsp)
	movq	%r14, 192(%rsp)
	movq	%rbx, 8(%rsp)
	movq	%r13, 32(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 40(%rsp)
	addq	%rcx, %rax
	movq	%rax, (%rsp)
	testq	%rbx, %rbx
	je	.LBB348_36
	movq	bcmp@GOTPCREL(%rip), %rbx
.LBB348_3:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, 16(%rsp)
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
	movq	16(%rsp), %rcx
	movq	32(%rsp), %r13
	cmpq	(%rsp), %rcx
	jne	.LBB348_3
	movq	216(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 304(%rsp)
	movq	$0, 320(%rsp)
	movq	$8, 328(%rsp)
	jmp	.LBB348_73
.LBB348_10:
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB348_37
	jmp	.LBB348_186
.LBB348_11:
.Ltmp14235:
	leaq	560(%rsp), %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14236:
	cmpl	$1, 560(%rsp)
	jne	.LBB348_78
.LBB348_13:
	vmovups	608(%rsp), %zmm1
	vmovups	576(%rsp), %zmm0
	movq	528(%rsp), %rax
	vmovups	%zmm1, 48(%rbp)
	vmovups	%zmm0, 16(%rbp)
	movq	$1, (%rbp)
	cmpq	$6, %rax
	jb	.LBB348_23
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	456(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB348_33
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	552(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_180
	lock		decq	(%rax)
	jne	.LBB348_180
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	552(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB348_180
.LBB348_36:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %r15
	lock		incq	(%rbp)
	jle	.LBB348_186
.LBB348_37:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 304(%rsp)
	movq	%r14, 312(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB348_183
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
	movq	192(%rsp), %rax
	addq	$1120, %rax
	movq	%rax, 272(%rsp)
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
	movq	%rbp, (%rsi)
	movq	$4, 48(%rsp)
	movq	%rsi, 56(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 64(%rsp)
	cmpq	(%rsp), %r15
	je	.LBB348_67
	movl	$1, %r14d
	testq	%rbx, %rbx
	jne	.LBB348_57
	movl	$24, %ebx
	leaq	48(%rsp), %rbp
	jmp	.LBB348_51
	.p2align	4
.LBB348_50:
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	addq	$48, %r12
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %r15
	movq	%r14, 64(%rsp)
	cmpq	(%rsp), %r12
	je	.LBB348_68
.LBB348_51:
	movq	24(%r15), %r13
	movq	%r15, %r12
	movq	32(%r15), %r15
	lock		incq	(%r13)
	jle	.LBB348_186
	movq	%r13, 304(%rsp)
	movq	%r15, 312(%rsp)
	cmpq	48(%rsp), %r14
	jne	.LBB348_50
.Ltmp14203:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.18159039729619107857)
.Ltmp14204:
	movq	56(%rsp), %rsi
	jmp	.LBB348_50
	.p2align	4
.LBB348_55:
	movq	56(%rsp), %rsi
.LBB348_56:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 64(%rsp)
	cmpq	(%rsp), %r15
	je	.LBB348_68
.LBB348_57:
	movq	%rsi, 24(%rsp)
.LBB348_58:
	movq	24(%r15), %r13
	movq	32(%r15), %rbp
	movq	32(%rsp), %rbx
	leaq	48(%r15), %rax
	movq	%rax, 16(%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB348_61
	.p2align	4
.LBB348_59:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB348_63
.LBB348_60:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB348_64
.LBB348_61:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB348_59
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB348_60
.LBB348_63:
	movq	16(%rsp), %r15
	cmpq	(%rsp), %r15
	jne	.LBB348_58
	jmp	.LBB348_68
	.p2align	4
.LBB348_64:
	lock		incq	(%r13)
	movq	24(%rsp), %rsi
	jle	.LBB348_186
	movq	16(%rsp), %r15
	movq	%r13, 304(%rsp)
	movq	%rbp, 312(%rsp)
	cmpq	48(%rsp), %r14
	jne	.LBB348_56
.Ltmp14200:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	48(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.18159039729619107857)
.Ltmp14201:
	jmp	.LBB348_55
.LBB348_67:
	movl	$1, %r14d
.LBB348_68:
	movq	56(%rsp), %r15
	movq	48(%rsp), %rax
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %r13
	movq	216(%rsp), %rbp
	shlq	$4, %r14
	movq	%r15, 304(%rsp)
	movq	%rax, 320(%rsp)
	movq	192(%rsp), %rax
	addq	%r15, %r14
	addq	$16, %r15
	movq	%r14, 328(%rsp)
	movq	1136(%rax), %rbx
	movq	%rbx, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB348_70
	.p2align	4
.LBB348_69:
	movq	192(%rsp), %rdx
	vmovdqu	48(%rsp), %ymm0
	vmovups	64(%rsp), %ymm1
	leaq	-16(%r15), %rax
	incq	%rbx
	addq	$16, %r15
	addq	$16, %rax
	movq	1128(%rdx), %rcx
	vmovups	%ymm1, 16(%rcx,%r12)
	vmovdqu	%ymm0, (%rcx,%r12)
	addq	$48, %r12
	movq	%rbx, 1136(%rdx)
	cmpq	%r14, %rax
	je	.LBB348_72
.LBB348_70:
	vmovups	-16(%r15), %xmm0
	movq	272(%rsp), %rax
	vmovups	%xmm0, 72(%rsp)
	movq	$2, 48(%rsp)
	movb	$0, 88(%rsp)
	cmpq	(%rax), %rbx
	jne	.LBB348_69
.Ltmp14211:
	movq	272(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp14212:
	jmp	.LBB348_69
.LBB348_72:
	movq	32(%rsp), %r13
.LBB348_73:
	movq	%r14, 312(%rsp)
.Ltmp14219:
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14220:
	movq	8(%rsp), %rbx
	movq	192(%rsp), %r14
	movq	232(%rsp), %rcx
	movq	224(%rsp), %r15
.Ltmp14221:
	leaq	48(%rsp), %rdi
	movq	%rcx, %rsi
	movq	%r14, %rdx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14222:
	movq	1136(%r14), %rsi
	subq	%r15, %rsi
	jb	.LBB348_77
	movq	40(%rsp), %rdi
	addq	1128(%r14), %rdi
	movq	%r15, 1136(%r14)
.Ltmp14223:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp14224:
.LBB348_77:
	vmovups	96(%rsp), %zmm1
	vmovdqu64	48(%rsp), %zmm0
	vmovups	%zmm1, 608(%rsp)
	vmovdqu64	%zmm0, 560(%rsp)
	cmpl	$1, 560(%rsp)
	je	.LBB348_13
.LBB348_78:
	leaq	568(%rsp), %rcx
.Ltmp14237:
	leaq	48(%rsp), %rdi
	leaq	456(%rsp), %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp14238:
	cmpq	$-1, 48(%rsp)
	je	.LBB348_135
	vmovdqu	48(%rsp), %ymm0
	movb	$1, %bpl
	vmovdqu	%ymm0, 416(%rsp)
.Ltmp14239:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rbx, %rsi
	vzeroupper
	callq	*%rax
.Ltmp14240:
	movq	%rax, 408(%rsp)
	movq	%rax, 40(%rsp)
	movq	32(%rax), %rcx
	movq	%rcx, %r13
	shlq	$4, %r13
	movq	%rcx, 8(%rsp)
	testq	%rcx, %rcx
	je	.LBB348_138
	movq	24(%rax), %r14
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	callq	*%rax
	leaq	48(%rsp), %rbp
	movq	%rax, (%rsp)
	testq	%rax, %rax
	je	.LBB348_184
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
	jle	.LBB348_85
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_85:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_91
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_85
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
.LBB348_88:
	cmpq	%rax, %rdx
	jle	.LBB348_90
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_88
.LBB348_90:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_91:
	movq	440(%rsp), %rbx
	movq	8(%rsp), %r12
	xorl	%r15d, %r15d
	addq	$16, %rbx
	.p2align	4
.LBB348_92:
	leaq	(%r14,%r15), %rsi
.Ltmp14241:
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.18159039729619107857)
.Ltmp14242:
	movq	(%rsp), %rcx
	movq	%rax, (%rcx,%r15)
	movq	%rdx, 8(%rcx,%r15)
	addq	$16, %r15
	decq	%r12
	jne	.LBB348_92
	movq	432(%rsp), %rbx
	movq	%r13, 296(%rsp)
	testq	%rbx, %rbx
	je	.LBB348_139
.LBB348_95:
	movq	malloc@GOTPCREL(%rip), %r15
	leaq	(,%rbx,8), %rax
	leaq	(%rax,%rax,4), %r14
	movq	%r14, %rdi
	callq	*%r15
	movq	%rax, 24(%rsp)
	testq	%rax, %rax
	je	.LBB348_185
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
	jle	.LBB348_98
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_98:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_104
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_98
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
.LBB348_101:
	cmpq	%rax, %rdx
	jle	.LBB348_103
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_101
.LBB348_103:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_104:
	movq	%r13, %rcx
	movq	424(%rsp), %r13
	movq	24(%rsp), %rax
	movq	(%rsp), %rdx
	movq	%rbx, 168(%rsp)
	xorl	%ebx, %ebx
	addq	%r13, %r14
	movq	%rax, 176(%rsp)
	leaq	(%rdx,%rcx), %rax
	movq	$0, 184(%rsp)
	movq	%r14, 232(%rsp)
	movq	%rcx, %r14
	movl	$2, %ecx
	negq	%r14
	movq	%rax, 16(%rsp)
	vmovd	%ecx, %xmm0
	movq	%r14, 224(%rsp)
	vmovdqa	%xmm0, 192(%rsp)
	jmp	.LBB348_107
	.p2align	4
.LBB348_105:
	movq	176(%rsp), %rax
	movq	%rax, 24(%rsp)
.LBB348_106:
	movq	24(%rsp), %rdx
	leaq	(%rbx,%rbx,4), %rax
	addq	$40, %r13
	incq	%rbx
	movq	%r14, (%rdx,%rax,8)
	movq	%r15, 8(%rdx,%rax,8)
	movq	224(%rsp), %r14
	vmovdqa	304(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	320(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%rbx, 184(%rsp)
	cmpq	232(%rsp), %r13
	je	.LBB348_140
.LBB348_107:
	movq	(%r13), %r15
	movq	%rbx, 32(%rsp)
	decq	%r15
	cmpq	$5, %r15
	jb	.LBB348_109
	movq	16(%r13), %r15
	movq	8(%r13), %rbx
	decq	%r15
	jmp	.LBB348_110
	.p2align	4
.LBB348_109:
	leaq	8(%r13), %rbx
.LBB348_110:
	cmpq	$5, 8(%rsp)
	movq	$1, 48(%rsp)
	jae	.LBB348_133
	xorl	%edx, %edx
.LBB348_112:
	xorl	%ebp, %ebp
	movl	$4, %eax
	leaq	56(%rsp), %rcx
	cmpq	$5, %rdx
	setae	%sil
	jb	.LBB348_114
	movq	56(%rsp), %rcx
	movq	%rdx, %rax
.LBB348_114:
	movb	%sil, %bpl
	shll	$4, %ebp
	movq	48(%rsp,%rbp), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rax, %rdx
	jae	.LBB348_121
	movq	(%rsp), %rsi
	incq	%rax
	xorl	%edx, %edx
	jmp	.LBB348_118
	.p2align	4
.LBB348_116:
	vmovq	(%rbx,%rdi,8), %xmm0
.LBB348_117:
	vmovq	%xmm0, -8(%rcx,%r12,8)
	addq	$16, %rsi
	incq	%r12
	addq	$-16, %rdx
	cmpq	%r12, %rax
	je	.LBB348_123
.LBB348_118:
	cmpq	%rdx, %r14
	je	.LBB348_122
	vmovdqa	192(%rsp), %xmm0
	cmpl	$1, (%rsi)
	jne	.LBB348_117
	movq	8(%rsi), %rdi
	cmpq	%r15, %rdi
	jb	.LBB348_116
	jmp	.LBB348_182
	.p2align	4
.LBB348_121:
	movq	(%rsp), %r14
	movq	%r12, %rax
	movq	%rax, 48(%rsp,%rbp)
	leaq	48(%rsp), %rbp
	cmpq	16(%rsp), %r14
	jne	.LBB348_124
	jmp	.LBB348_131
	.p2align	4
.LBB348_122:
	movq	%r12, 48(%rsp,%rbp)
	leaq	48(%rsp), %rbp
	jmp	.LBB348_131
	.p2align	4
.LBB348_123:
	movq	(%rsp), %r14
	subq	%rdx, %r14
	movq	%rax, 48(%rsp,%rbp)
	leaq	48(%rsp), %rbp
	cmpq	16(%rsp), %r14
	jne	.LBB348_124
	jmp	.LBB348_131
.LBB348_129:
.Ltmp14256:
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	movl	$1, %ecx
	movq	%rbp, %rdi
	callq	*%rax
.Ltmp14257:
	cmpq	$6, 48(%rsp)
	movq	56(%rsp), %rax
	leaq	56(%rsp), %rcx
	leaq	64(%rsp), %rdx
	cmovbq	%rcx, %rax
	movq	%rbp, %rcx
	cmovaeq	%rdx, %rcx
	jmp	.LBB348_128
	.p2align	4
.LBB348_124:
	vmovdqa	192(%rsp), %xmm0
	cmpl	$1, (%r14)
	vmovdqa	%xmm0, 272(%rsp)
	jne	.LBB348_127
	movq	8(%r14), %rdi
	cmpq	%r15, %rdi
	jae	.LBB348_181
	vmovq	(%rbx,%rdi,8), %xmm0
	vmovdqa	%xmm0, 272(%rsp)
.LBB348_127:
	movq	48(%rsp), %rsi
	movq	56(%rsp), %rax
	xorl	%edx, %edx
	leaq	56(%rsp), %rcx
	leaq	64(%rsp), %rdi
	decq	%rsi
	cmpq	$5, %rsi
	cmovbq	%rcx, %rax
	movq	%rbp, %rcx
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	48(%rsp,%rdx), %r12
	leaq	-1(%r12), %rdx
	cmpq	%rsi, %rdx
	je	.LBB348_129
.LBB348_128:
	vmovdqa	272(%rsp), %xmm0
	addq	$16, %r14
	vmovq	%xmm0, -8(%rax,%r12,8)
	incq	%r12
	movq	%r12, (%rcx)
	cmpq	16(%rsp), %r14
	jne	.LBB348_124
.LBB348_131:
	leaq	64(%rsp), %rax
	movq	48(%rsp), %r14
	movq	56(%rsp), %r15
	movq	32(%rsp), %rbx
	vmovdqu	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	%rax, 320(%rsp)
	vmovdqa	%xmm0, 304(%rsp)
	cmpq	168(%rsp), %rbx
	jne	.LBB348_106
.Ltmp14259:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	168(%rsp), %rdi
	callq	*%rax
.Ltmp14260:
	jmp	.LBB348_105
.LBB348_133:
.Ltmp14247:
	movq	8(%rsp), %rdx
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%rbp, %rdi
	xorl	%esi, %esi
	callq	*%rax
.Ltmp14248:
	movq	48(%rsp), %rdx
	decq	%rdx
	jmp	.LBB348_112
.LBB348_135:
.Ltmp14273:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp14274:
	vmovups	496(%rsp), %zmm1
	vmovups	456(%rsp), %zmm0
	movq	%rax, 264(%rsp)
	movq	$0, 240(%rsp)
	movq	$8, 248(%rsp)
	movq	$0, 256(%rsp)
	vmovups	%zmm1, 88(%rsp)
	vmovups	%zmm0, 48(%rsp)
	cmpq	$-1, 48(%rsp)
	je	.LBB348_166
	leaq	304(%rsp), %rdi
	leaq	240(%rsp), %rsi
	leaq	456(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB348_167
	jmp	.LBB348_176
.LBB348_138:
	movl	$8, %eax
	leaq	48(%rsp), %rbp
	movq	%rax, (%rsp)
	movq	432(%rsp), %rbx
	movq	%r13, 296(%rsp)
	testq	%rbx, %rbx
	jne	.LBB348_95
.LBB348_139:
	movq	$0, 168(%rsp)
	movq	$8, 176(%rsp)
	movq	$0, 184(%rsp)
.LBB348_140:
	movq	168(%rsp), %rcx
	movq	184(%rsp), %rax
	movq	176(%rsp), %rdx
	cmpq	$0, 8(%rsp)
	movq	%rcx, 240(%rsp)
	movq	40(%rsp), %rcx
	movq	%rax, 256(%rsp)
	movq	%rdx, 248(%rsp)
	movq	%rcx, 264(%rsp)
	je	.LBB348_150
	movq	296(%rsp), %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %rsi
	cmovaeq	%rcx, %rsi
	xorl	%edx, %edx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB348_143
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_143:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_149
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_143
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
.LBB348_146:
	cmpq	%rax, %rdx
	jge	.LBB348_148
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_146
.LBB348_148:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_149:
	movq	free@GOTPCREL(%rip), %rax
	movq	(%rsp), %rdi
	callq	*%rax
.LBB348_150:
	vmovdqu64	456(%rsp), %zmm0
	vmovups	496(%rsp), %zmm1
	vmovdqu64	%zmm0, 48(%rsp)
	vmovups	%zmm1, 88(%rsp)
	cmpq	$-1, 48(%rsp)
	je	.LBB348_153
	leaq	304(%rsp), %rdi
	leaq	240(%rsp), %rsi
	leaq	456(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB348_152
.LBB348_154:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	128(%rsp), %rdi
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
	jge	.LBB348_156
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_156:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_162
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_156
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
.LBB348_159:
	cmpq	%rax, %rsi
	jge	.LBB348_161
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_159
.LBB348_161:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_162:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	144(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB348_163
	jmp	.LBB348_165
.LBB348_153:
	vmovdqu	240(%rsp), %ymm0
	vmovdqu	%ymm0, 312(%rsp)
	movq	$-1, 304(%rsp)
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB348_154
.LBB348_152:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_165
.LBB348_163:
	lock		decq	(%rax)
	jne	.LBB348_165
	xorl	%ebp, %ebp
	leaq	144(%rsp), %rdi
	#MEMBARRIER
.Ltmp14268:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp14269:
.LBB348_165:
	vmovups	304(%rsp), %zmm0
	vmovups	336(%rsp), %zmm1
	movq	216(%rsp), %rbp
	leaq	416(%rsp), %rdi
	vmovups	%zmm1, 40(%rbp)
	vmovups	%zmm0, 8(%rbp)
	movq	$0, (%rbp)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB348_180
.LBB348_166:
	movq	248(%rsp), %rcx
	movq	240(%rsp), %rax
	movq	256(%rsp), %rdx
	movq	%rcx, 320(%rsp)
	movq	264(%rsp), %rcx
	movq	%rax, 312(%rsp)
	movq	%rdx, 328(%rsp)
	movq	%rcx, 336(%rsp)
	movq	$-1, 304(%rsp)
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB348_176
.LBB348_167:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	128(%rsp), %rdi
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
	jge	.LBB348_169
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_169:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_175
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_169
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
.LBB348_172:
	cmpq	%rax, %rsi
	jge	.LBB348_174
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_172
.LBB348_174:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_175:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_176:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_179
	lock		decq	(%rax)
	jne	.LBB348_179
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB348_179:
	vmovups	336(%rsp), %zmm1
	vmovups	304(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbp)
	vmovups	%zmm0, 8(%rbp)
	movq	$0, (%rbp)
.LBB348_180:
	movq	%rbp, %rax
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
.LBB348_181:
	.cfi_def_cfa_offset 736
.Ltmp14253:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.757(%rip), %rdx
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14254:
	jmp	.LBB348_186
.LBB348_182:
.Ltmp14250:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.757(%rip), %rdx
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14251:
	jmp	.LBB348_186
.LBB348_183:
.Ltmp14229:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp14230:
	jmp	.LBB348_186
.LBB348_184:
.Ltmp14244:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp14245:
	jmp	.LBB348_186
.LBB348_185:
.Ltmp14262:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r14, %rsi
	callq	*%rax
.Ltmp14263:
.LBB348_186:
	ud2
.LBB348_187:
.Ltmp14249:
	jmp	.LBB348_211
.LBB348_188:
.Ltmp14258:
	jmp	.LBB348_211
.LBB348_189:
.Ltmp14264:
	movq	%rax, %rbx
	jmp	.LBB348_216
.LBB348_190:
.Ltmp14246:
	movq	%rax, %rbx
	jmp	.LBB348_219
.LBB348_191:
.Ltmp14225:
	movq	%rax, %rbx
.Ltmp14226:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp14227:
	jmp	.LBB348_223
.LBB348_192:
.Ltmp14270:
	movq	%rax, %rbx
	jmp	.LBB348_221
.LBB348_193:
.Ltmp14231:
	lock		decq	(%rbp)
	movq	%rax, %rbx
	jne	.LBB348_223
	#MEMBARRIER
.Ltmp14232:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	callq	*%rax
.Ltmp14233:
	jmp	.LBB348_223
.LBB348_195:
.Ltmp14202:
	jmp	.LBB348_197
.LBB348_196:
.Ltmp14205:
.LBB348_197:
	lock		decq	(%r13)
	movq	%rax, %rbx
	jne	.LBB348_199
	#MEMBARRIER
.Ltmp14206:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	304(%rsp), %rdi
	callq	*%rax
.Ltmp14207:
.LBB348_199:
.Ltmp14209:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14210:
	jmp	.LBB348_223
.LBB348_200:
.Ltmp14208:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_201:
.Ltmp14234:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_202:
.Ltmp14275:
	movq	%rax, %rbx
	jmp	.LBB348_223
.LBB348_203:
.Ltmp14261:
	movq	%rax, %rbx
	cmpq	$5, %r14
	ja	.LBB348_214
	jmp	.LBB348_215
.LBB348_204:
.Ltmp14243:
	movq	(%rsp), %rdi
	movl	$8, %edx
	movq	%rax, %rbx
	movq	%r13, %rsi
	jmp	.LBB348_218
.LBB348_205:
.Ltmp14213:
	movq	%rax, %rbx
	movq	%r15, 312(%rsp)
.Ltmp14214:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp14215:
.Ltmp14217:
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14218:
	jmp	.LBB348_223
.LBB348_207:
.Ltmp14228:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_208:
.Ltmp14216:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_209:
.Ltmp14252:
	movq	%rax, %rbx
	movq	%r12, 48(%rsp,%rbp)
	jmp	.LBB348_212
.LBB348_210:
.Ltmp14255:
.LBB348_211:
	movq	%rax, %rbx
.LBB348_212:
	movq	48(%rsp), %r14
	cmpq	$6, %r14
	jb	.LBB348_215
	movq	56(%rsp), %r15
.LBB348_214:
	leaq	-8(,%r14,8), %rsi
	movl	$4, %edx
	movq	%r15, %rdi
	callq	__rustc::__rust_dealloc
.LBB348_215:
	leaq	168(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB348_216:
	cmpq	$0, 8(%rsp)
	je	.LBB348_219
	movq	(%rsp), %rdi
	movq	296(%rsp), %rsi
	movl	$8, %edx
.LBB348_218:
	callq	__rustc::__rust_dealloc
.LBB348_219:
	movq	40(%rsp), %rax
	movb	$1, %bpl
	lock		decq	(%rax)
	jne	.LBB348_221
	#MEMBARRIER
.Ltmp14265:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp14266:
.LBB348_221:
.Ltmp14271:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp14272:
	testb	%bpl, %bpl
	je	.LBB348_224
.LBB348_223:
.Ltmp14276:
	leaq	456(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14277:
.LBB348_224:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB348_225:
.Ltmp14267:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_226:
.Ltmp14278:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end348:

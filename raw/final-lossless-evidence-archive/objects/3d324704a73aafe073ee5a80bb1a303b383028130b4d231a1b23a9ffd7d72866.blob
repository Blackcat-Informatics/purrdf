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
	subq	$632, %rsp
	.cfi_def_cfa_offset 688
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	movq	%rdi, %rbx
	leaq	408(%rsp), %rdi
	movq	%r9, %r14
	movq	%r8, %r15
	movq	%rcx, %r13
	movq	%rdx, %rbp
	callq	*%rax
	movq	1136(%r14), %rax
	movq	%rbx, 296(%rsp)
	testq	%rax, %rax
	je	.LBB348_1
	movq	1128(%r14), %rcx
	movq	%rax, 344(%rsp)
	leaq	(%rax,%rax,2), %rax
	movq	%r15, %r12
	shlq	$4, %r12
	movq	%r14, 32(%rsp)
	movq	%r15, 288(%rsp)
	movq	%r13, 40(%rsp)
	movq	%rbp, 336(%rsp)
	addq	%r13, %r12
	shlq	$4, %rax
	movq	%rax, 328(%rsp)
	addq	%rcx, %rax
	movq	%rax, 24(%rsp)
	testq	%r15, %r15
	je	.LBB348_41
	movq	bcmp@GOTPCREL(%rip), %rbx
	movq	%rcx, %rax
.LBB348_4:
	movq	24(%rax), %rbp
	movq	32(%rax), %r14
	addq	$48, %rax
	movq	%rax, 8(%rsp)
	leaq	16(%rbp), %r15
	jmp	.LBB348_5
	.p2align	4
.LBB348_38:
	xorq	%r14, %rax
	xorq	%rbp, %rdi
	orq	%rax, %rdi
	je	.LBB348_7
.LBB348_39:
	addq	$16, %r13
	cmpq	%r12, %r13
	je	.LBB348_40
.LBB348_5:
	movq	(%r13), %rdi
	movq	8(%r13), %rax
	cmpq	%rbp, %rdi
	sete	%cl
	cmpq	%r14, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB348_38
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*%rbx
	testl	%eax, %eax
	jne	.LBB348_39
.LBB348_7:
	movq	8(%rsp), %rax
	movq	40(%rsp), %r13
	cmpq	24(%rsp), %rax
	jne	.LBB348_4
	movq	32(%rsp), %rbp
	movl	$8, %r14d
	movq	$8, 192(%rsp)
	movq	$0, 208(%rsp)
	movq	$8, 216(%rsp)
	jmp	.LBB348_9
.LBB348_40:
	movq	288(%rsp), %r15
	lock		incq	(%rbp)
	jg	.LBB348_46
	jmp	.LBB348_43
.LBB348_1:
.Ltmp14235:
	leaq	512(%rsp), %rdi
	movq	%rbp, %rsi
	movq	%r14, %rdx
	movq	%rbp, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14236:
	cmpl	$1, 512(%rsp)
	jne	.LBB348_96
.LBB348_15:
	vmovups	560(%rsp), %zmm1
	vmovups	528(%rsp), %zmm0
	movq	480(%rsp), %rax
	vmovups	%zmm1, 48(%rbx)
	vmovups	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB348_25
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	488(%rsp), %rdi
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
	jge	.LBB348_18
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_18:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_24
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_18
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
.LBB348_21:
	cmpq	%rax, %rsi
	jge	.LBB348_23
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_21
.LBB348_23:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_24:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_25:
	movq	408(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB348_35
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	416(%rsp), %rdi
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
	jge	.LBB348_28
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_28:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_34
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_28
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
.LBB348_31:
	cmpq	%rax, %rsi
	jge	.LBB348_33
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_31
.LBB348_33:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_34:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_35:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_167
	lock		decq	(%rax)
	jne	.LBB348_167
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB348_167
.LBB348_41:
	movq	24(%rcx), %rbp
	movq	32(%rcx), %r14
	leaq	48(%rcx), %rax
	movq	%rax, 8(%rsp)
	lock		incq	(%rbp)
	jle	.LBB348_43
.LBB348_46:
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$64, %edi
	movq	%rbp, 192(%rsp)
	movq	%r14, 200(%rsp)
	callq	*%rax
	testq	%rax, %rax
	je	.LBB348_58
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
	jle	.LBB348_49
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB348_49:
	movq	32(%rsp), %rax
	addq	$1120, %rax
	movq	%rax, 16(%rsp)
	.p2align	4
.LBB348_50:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_56
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_50
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
.LBB348_53:
	cmpq	%rax, %rdx
	jle	.LBB348_55
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_53
.LBB348_55:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_56:
	movq	8(%rsp), %rax
	movq	%rbp, (%rsi)
	movq	$4, 80(%rsp)
	movq	%rsi, 88(%rsp)
	movq	%r14, 8(%rsi)
	movq	$1, 96(%rsp)
	cmpq	24(%rsp), %rax
	je	.LBB348_57
	movl	$1, %r14d
	testq	%r15, %r15
	je	.LBB348_66
	movq	8(%rsp), %rbx
	jmp	.LBB348_61
	.p2align	4
.LBB348_88:
	movq	88(%rsp), %rsi
.LBB348_77:
	movq	%r14, %rax
	shlq	$4, %rax
	incq	%r14
	movq	%r13, (%rsi,%rax)
	movq	%rbp, 8(%rsi,%rax)
	movq	%r14, 96(%rsp)
	cmpq	24(%rsp), %rbx
	je	.LBB348_78
.LBB348_61:
	movq	%rsi, 352(%rsp)
.LBB348_62:
	movq	24(%rbx), %r13
	leaq	48(%rbx), %rax
	movq	32(%rbx), %rbp
	movq	40(%rsp), %rbx
	movq	%rax, 8(%rsp)
	leaq	16(%r13), %r15
	jmp	.LBB348_63
	.p2align	4
.LBB348_73:
	xorq	%rbp, %rax
	xorq	%r13, %rdi
	orq	%rax, %rdi
	je	.LBB348_65
.LBB348_74:
	addq	$16, %rbx
	cmpq	%r12, %rbx
	je	.LBB348_75
.LBB348_63:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	cmpq	%r13, %rdi
	sete	%cl
	cmpq	%rbp, %rax
	setne	%dl
	orb	%cl, %dl
	jne	.LBB348_73
	movq	bcmp@GOTPCREL(%rip), %rax
	addq	$16, %rdi
	movq	%r15, %rsi
	movq	%rbp, %rdx
	callq	*%rax
	testl	%eax, %eax
	jne	.LBB348_74
.LBB348_65:
	movq	8(%rsp), %rbx
	cmpq	24(%rsp), %rbx
	jne	.LBB348_62
	jmp	.LBB348_78
	.p2align	4
.LBB348_75:
	lock		incq	(%r13)
	movq	352(%rsp), %rsi
	jle	.LBB348_43
	movq	8(%rsp), %rbx
	movq	%r13, 192(%rsp)
	movq	%rbp, 200(%rsp)
	cmpq	80(%rsp), %r14
	jne	.LBB348_77
.Ltmp14200:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	leaq	80(%rsp), %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.8174518965507137190)
.Ltmp14201:
	jmp	.LBB348_88
.LBB348_57:
	movl	$1, %r14d
.LBB348_78:
	movq	32(%rsp), %rbp
	movq	88(%rsp), %r15
	movq	80(%rsp), %rax
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one@GOTPCREL(%rip), %r13
	shlq	$4, %r14
	movq	1136(%rbp), %rbx
	movq	%r15, 192(%rsp)
	movq	%rax, 208(%rsp)
	addq	%r15, %r14
	addq	$16, %r15
	movq	%r14, 216(%rsp)
	movq	%rbx, %rax
	shlq	$4, %rax
	leaq	(%rax,%rax,2), %r12
	jmp	.LBB348_79
	.p2align	4
.LBB348_81:
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
	je	.LBB348_82
.LBB348_79:
	vmovups	-16(%r15), %xmm0
	movq	16(%rsp), %rax
	vmovups	%xmm0, 104(%rsp)
	movq	$2, 80(%rsp)
	movb	$0, 120(%rsp)
	cmpq	(%rax), %rbx
	jne	.LBB348_81
.Ltmp14211:
	movq	16(%rsp), %rdi
	vzeroupper
	callq	*%r13
.Ltmp14212:
	jmp	.LBB348_81
.LBB348_82:
	movq	40(%rsp), %r13
.LBB348_9:
	movq	%r14, 200(%rsp)
.Ltmp14219:
	leaq	192(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14220:
	movq	296(%rsp), %rbx
	movq	288(%rsp), %r15
	movq	344(%rsp), %r14
.Ltmp14221:
	movq	336(%rsp), %rcx
	leaq	80(%rsp), %rdi
	movq	%rbp, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp14222:
	movq	1136(%rbp), %rsi
	subq	%r14, %rsi
	jb	.LBB348_13
	movq	328(%rsp), %rdi
	addq	1128(%rbp), %rdi
	movq	%r14, 1136(%rbp)
.Ltmp14223:
	callq	core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
.Ltmp14224:
.LBB348_13:
	vmovups	128(%rsp), %zmm1
	vmovups	80(%rsp), %zmm0
	vmovups	%zmm1, 560(%rsp)
	vmovups	%zmm0, 512(%rsp)
	cmpl	$1, 512(%rsp)
	je	.LBB348_15
.LBB348_96:
	leaq	520(%rsp), %rcx
.Ltmp14237:
	leaq	80(%rsp), %rdi
	leaq	408(%rsp), %rsi
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp14238:
	cmpq	$-1, 80(%rsp)
	je	.LBB348_169
	vmovups	80(%rsp), %ymm0
	movb	$1, %bpl
	vmovups	%ymm0, 368(%rsp)
.Ltmp14239:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	*%rax
.Ltmp14240:
	movq	%rax, 360(%rsp)
	movl	$8, %edi
	movl	$8, %r13d
	movq	%rax, %r15
	movq	32(%rax), %rbx
	testq	%rbx, %rbx
	je	.LBB348_113
	movq	malloc@GOTPCREL(%rip), %rax
	movq	24(%r15), %r14
	movq	%rbx, %r12
	shlq	$4, %r12
	movq	%r12, %rdi
	callq	*%rax
	movq	%r15, 16(%rsp)
	testq	%rax, %rax
	je	.LBB348_140
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r12, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r12, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB348_103
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_103:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_109
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_103
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	%r12, (%rdx)
	movq	%r12, %rdx
	lock		xaddq	%rdx, (%rsi)
	addq	%r12, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB348_106:
	cmpq	%rax, %rdx
	jle	.LBB348_108
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_106
.LBB348_108:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_109:
	movq	392(%rsp), %rbp
	movq	%rbx, %r15
	xorl	%ebx, %ebx
	movq	%r15, 8(%rsp)
	addq	$16, %rbp
	.p2align	4
.LBB348_110:
	leaq	(%r14,%rbx), %rsi
.Ltmp14241:
	movq	%rbp, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.8174518965507137190)
.Ltmp14242:
	movq	%rax, (%r13,%rbx)
	movq	%rdx, 8(%r13,%rbx)
	addq	$16, %rbx
	decq	%r15
	jne	.LBB348_110
	movq	16(%rsp), %r15
	movq	8(%rsp), %rbx
	movl	$8, %edi
.LBB348_113:
	movq	384(%rsp), %r14
	movq	%rbx, 304(%rsp)
	movq	%r13, 312(%rsp)
	movq	%rbx, 320(%rsp)
	testq	%r14, %r14
	je	.LBB348_123
	movq	malloc@GOTPCREL(%rip), %r13
	leaq	(,%r14,8), %rax
	leaq	(%rax,%rax,4), %r12
	movq	%r12, %rdi
	callq	*%r13
	testq	%rax, %rax
	je	.LBB348_147
	movq	%rax, %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %r8
	movabsq	$-9223372036854775808, %rcx
	movq	$-1, %r9
	leaq	(%r12,%rax), %rdx
	sarq	$63, %rdx
	xorq	%rcx, %rdx
	addq	%r12, %rax
	cmovoq	%rdx, %rax
	incq	%rsi
	cmoveq	%r9, %rsi
	addq	%r12, %r8
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r9, %r8
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%r8, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB348_117
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_117:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_123
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_117
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
.LBB348_120:
	cmpq	%rax, %rdx
	jle	.LBB348_122
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB348_120
.LBB348_122:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_123:
	movq	376(%rsp), %rax
	leaq	(%r14,%r14,4), %rcx
	movq	%r14, 192(%rsp)
	movq	%rdi, 200(%rsp)
	movq	$0, 208(%rsp)
	leaq	(%rax,%rcx,8), %rcx
	movq	%rax, 80(%rsp)
	leaq	304(%rsp), %rax
	movq	%rcx, 88(%rsp)
	movq	%rax, 96(%rsp)
.Ltmp14250:
	leaq	192(%rsp), %rdi
	leaq	80(%rsp), %rsi
	callq	<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>> as alloc::vec::spec_extend::SpecExtend<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>>::spec_extend
.Ltmp14251:
	vmovups	192(%rsp), %xmm0
	movq	208(%rsp), %rax
	movq	304(%rsp), %rcx
	movq	296(%rsp), %rbx
	movq	%rax, 64(%rsp)
	vmovaps	%xmm0, 48(%rsp)
	movq	%r15, 72(%rsp)
	testq	%rcx, %rcx
	je	.LBB348_134
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$4, %rcx
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
	jge	.LBB348_127
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_127:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_133
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_127
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
.LBB348_130:
	cmpq	%rax, %rsi
	jge	.LBB348_132
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_130
.LBB348_132:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_133:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_134:
	vmovups	408(%rsp), %zmm0
	vmovups	448(%rsp), %zmm1
	vmovups	%zmm0, 80(%rsp)
	vmovups	%zmm1, 120(%rsp)
	cmpq	$-1, 80(%rsp)
	je	.LBB348_152
	leaq	192(%rsp), %rdi
	leaq	48(%rsp), %rsi
	leaq	408(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB348_154
.LBB348_163:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB348_164
	jmp	.LBB348_166
.LBB348_169:
.Ltmp14261:
	movq	<purrdf_sparql_eval::solution::VarSchema>::interned@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r15, %rsi
	callq	*%rax
.Ltmp14262:
	vmovups	448(%rsp), %zmm1
	vmovups	408(%rsp), %zmm0
	movq	%rax, 72(%rsp)
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
	vmovups	%zmm1, 120(%rsp)
	vmovups	%zmm0, 80(%rsp)
	cmpq	$-1, 80(%rsp)
	je	.LBB348_172
	leaq	192(%rsp), %rdi
	leaq	48(%rsp), %rsi
	leaq	408(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB348_174
	jmp	.LBB348_183
.LBB348_66:
	movq	8(%rsp), %rax
	movl	$24, %ebx
	leaq	80(%rsp), %rbp
	jmp	.LBB348_67
	.p2align	4
.LBB348_71:
	addq	$48, %r12
	movq	%r13, -8(%rsi,%rbx)
	movq	%r15, (%rsi,%rbx)
	incq	%r14
	addq	$16, %rbx
	movq	%r12, %rax
	movq	%r14, 96(%rsp)
	cmpq	24(%rsp), %r12
	je	.LBB348_78
.LBB348_67:
	movq	24(%rax), %r13
	movq	32(%rax), %r15
	lock		incq	(%r13)
	jle	.LBB348_43
	movq	%r13, 192(%rsp)
	movq	%rax, %r12
	movq	%r15, 200(%rsp)
	cmpq	80(%rsp), %r14
	jne	.LBB348_71
.Ltmp14203:
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	movq	%rbp, %rdi
	movq	%r14, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.8174518965507137190)
.Ltmp14204:
	movq	88(%rsp), %rsi
	jmp	.LBB348_71
.LBB348_152:
	vmovups	48(%rsp), %ymm0
	vmovups	%ymm0, 200(%rsp)
	movq	$-1, 192(%rsp)
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB348_163
.LBB348_154:
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
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_166
.LBB348_164:
	lock		decq	(%rax)
	jne	.LBB348_166
	xorl	%ebp, %ebp
	leaq	176(%rsp), %rdi
	#MEMBARRIER
.Ltmp14256:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp14257:
.LBB348_166:
	vmovups	224(%rsp), %zmm1
	vmovups	192(%rsp), %zmm0
	leaq	368(%rsp), %rdi
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
	jmp	.LBB348_167
.LBB348_172:
	movq	56(%rsp), %rcx
	movq	48(%rsp), %rax
	movq	64(%rsp), %rdx
	movq	%rcx, 208(%rsp)
	movq	72(%rsp), %rcx
	movq	%rax, 200(%rsp)
	movq	%rdx, 216(%rsp)
	movq	%rcx, 224(%rsp)
	movq	$-1, 192(%rsp)
	movq	152(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB348_183
.LBB348_174:
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
	jge	.LBB348_176
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB348_176:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB348_182
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB348_176
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
.LBB348_179:
	cmpq	%rax, %rsi
	jge	.LBB348_181
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB348_179
.LBB348_181:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB348_182:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB348_183:
	movq	176(%rsp), %rax
	testq	%rax, %rax
	je	.LBB348_186
	lock		decq	(%rax)
	jne	.LBB348_186
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB348_186:
	vmovups	224(%rsp), %zmm1
	vmovups	192(%rsp), %zmm0
	vmovups	%zmm1, 40(%rbx)
	vmovups	%zmm0, 8(%rbx)
	movq	$0, (%rbx)
.LBB348_167:
	movq	%rbx, %rax
	addq	$632, %rsp
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
.LBB348_58:
	.cfi_def_cfa_offset 688
.Ltmp14229:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$64, %esi
	callq	*%rax
.Ltmp14230:
	jmp	.LBB348_43
.LBB348_140:
.Ltmp14244:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r12, %rsi
	callq	*%rax
.Ltmp14245:
	jmp	.LBB348_43
.LBB348_147:
.Ltmp14247:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r12, %rsi
	movq	%rbx, %r14
	movq	%r15, 16(%rsp)
	callq	*%rax
.Ltmp14248:
.LBB348_43:
	ud2
.LBB348_142:
.Ltmp14249:
	movq	%rax, %rbx
	jmp	.LBB348_143
.LBB348_141:
.Ltmp14246:
	movq	%rax, %rbx
	jmp	.LBB348_150
.LBB348_145:
.Ltmp14252:
	leaq	192(%rsp), %rdi
	movq	%r15, 16(%rsp)
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	304(%rsp), %r14
.LBB348_143:
	testq	%r14, %r14
	je	.LBB348_150
	movq	312(%rsp), %rdi
	movq	%r14, %rsi
	shlq	$4, %rsi
	movl	$8, %edx
	jmp	.LBB348_149
.LBB348_94:
.Ltmp14225:
	movq	%rax, %rbx
.Ltmp14226:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
.Ltmp14227:
	jmp	.LBB348_188
.LBB348_136:
.Ltmp14258:
	movq	%rax, %rbx
	jmp	.LBB348_137
.LBB348_44:
.Ltmp14231:
	lock		decq	(%rbp)
	movq	%rax, %rbx
	jne	.LBB348_188
	#MEMBARRIER
.Ltmp14232:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	192(%rsp), %rdi
	callq	*%rax
.Ltmp14233:
	jmp	.LBB348_188
.LBB348_83:
.Ltmp14202:
	jmp	.LBB348_84
.LBB348_72:
.Ltmp14205:
.LBB348_84:
	lock		decq	(%r13)
	movq	%rax, %rbx
	jne	.LBB348_86
	#MEMBARRIER
.Ltmp14206:
	movq	<alloc::sync::Arc<str>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	192(%rsp), %rdi
	callq	*%rax
.Ltmp14207:
.LBB348_86:
.Ltmp14209:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14210:
	jmp	.LBB348_188
.LBB348_89:
.Ltmp14208:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_90:
.Ltmp14234:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_187:
.Ltmp14263:
	movq	%rax, %rbx
	jmp	.LBB348_188
.LBB348_148:
.Ltmp14243:
	movl	$8, %edx
	movq	%rax, %rbx
	movq	%r13, %rdi
	movq	%r12, %rsi
.LBB348_149:
	callq	__rustc::__rust_dealloc
.LBB348_150:
	movq	16(%rsp), %rax
	movb	$1, %bpl
	lock		decq	(%rax)
	jne	.LBB348_137
	#MEMBARRIER
.Ltmp14253:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	callq	*%rax
.Ltmp14254:
.LBB348_137:
.Ltmp14259:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp14260:
	testb	%bpl, %bpl
	jne	.LBB348_188
	jmp	.LBB348_139
.LBB348_146:
.Ltmp14255:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_91:
.Ltmp14213:
	movq	%rax, %rbx
	movq	%r15, 200(%rsp)
.Ltmp14214:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
.Ltmp14215:
.Ltmp14217:
	leaq	192(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
.Ltmp14218:
.LBB348_188:
.Ltmp14264:
	leaq	408(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp14265:
.LBB348_139:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB348_95:
.Ltmp14228:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_93:
.Ltmp14216:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB348_168:
.Ltmp14266:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end348:

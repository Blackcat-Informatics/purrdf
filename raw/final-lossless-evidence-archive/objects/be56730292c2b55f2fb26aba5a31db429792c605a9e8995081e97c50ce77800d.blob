<purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>:
.Lfunc_begin110:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception73
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
	subq	$248, %rsp
	.cfi_def_cfa_offset 304
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	vmovups	(%rsi), %ymm0
	movq	$0, 8(%rsp)
	movq	$8, 16(%rsp)
	movq	%rdi, 72(%rsp)
	movq	$0, 24(%rsp)
	vmovups	%ymm0, 32(%rsp)
	movq	56(%rsp), %rax
	movq	40(%rsp), %r15
	movq	%rax, 80(%rsp)
	cmpq	%rax, %r15
	je	.LBB110_8
	addq	$200, %r15
	leaq	96(%rsp), %r13
	movl	$8, %eax
	xorl	%ebp, %ebp
	xorl	%r14d, %r14d
	movq	%r15, %rcx
	.p2align	4
.LBB110_2:
	movq	%rcx, %r15
	movq	-200(%rcx), %rcx
	cmpq	$-1, %rcx
	je	.LBB110_8
	leaq	-200(%r15), %rbx
	vmovups	8(%rbx), %zmm0
	vmovups	72(%rbx), %zmm1
	vmovups	96(%rbx), %zmm2
	vmovups	%zmm2, 88(%r13)
	vmovups	%zmm1, 64(%r13)
	vmovups	%zmm0, (%r13)
	movq	%rcx, 88(%rsp)
	movzbl	240(%rsp), %r12d
	cmpq	8(%rsp), %r14
	jne	.LBB110_6
.Ltmp1352:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1353:
	movq	16(%rsp), %rax
.LBB110_6:
	vmovups	88(%rsp), %zmm0
	vmovups	152(%rsp), %zmm1
	vmovups	184(%rsp), %zmm2
	incq	%r14
	vmovups	%zmm2, 96(%rax,%rbp)
	vmovups	%zmm1, 64(%rax,%rbp)
	vmovups	%zmm0, (%rax,%rbp)
	movq	%r14, 24(%rsp)
	testb	%r12b, %r12b
	jne	.LBB110_18
	addq	$160, %rbp
	leaq	200(%r15), %rcx
	addq	$200, %rbx
	cmpq	80(%rsp), %rbx
	jne	.LBB110_2
.LBB110_8:
	xorl	%ebx, %ebx
	movq	%r15, 40(%rsp)
	jmp	.LBB110_9
.LBB110_18:
	movq	%r15, 40(%rsp)
	movb	$1, %bl
.LBB110_9:
.Ltmp1360:
	leaq	32(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1361:
	vmovups	8(%rsp), %xmm0
	movq	72(%rsp), %rcx
	movq	24(%rsp), %rax
	movq	%rax, 16(%rcx)
	vmovups	%xmm0, (%rcx)
	movb	%bl, 24(%rcx)
	addq	$248, %rsp
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
.LBB110_17:
	.cfi_def_cfa_offset 304
.Ltmp1362:
	movq	%rax, %rbx
	jmp	.LBB110_13
.LBB110_11:
.Ltmp1354:
	movq	%rax, %rbx
	movq	%r15, 40(%rsp)
.Ltmp1355:
	leaq	88(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp1356:
.Ltmp1358:
	leaq	32(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1359:
.LBB110_13:
.Ltmp1363:
	leaq	8(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1364:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB110_15:
.Ltmp1357:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_16:
.Ltmp1365:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end110:
purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>:
.Lfunc_begin191:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception117
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
	movq	16(%r8), %r12
	movq	$0, 256(%rsp)
	movq	$8, 264(%rsp)
	movq	%r9, %r15
	movq	%r8, %rbp
	movq	%rcx, 240(%rsp)
	movl	%edx, %ebx
	movq	%rsi, %r13
	movq	%rdi, %r14
	movq	$0, 272(%rsp)
.Ltmp4347:
	leaq	432(%rsp), %rdi
	leaq	256(%rsp), %rsi
	movq	%r12, %rdx
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp4348:
	vmovups	440(%rsp), %xmm0
	movq	456(%rsp), %rcx
	movq	432(%rsp), %rax
	movq	%rcx, 208(%rsp)
	vmovaps	%xmm0, 192(%rsp)
	cmpq	$-1, %rax
	je	.LBB191_25
	vmovups	464(%rsp), %zmm0
	vmovaps	192(%rsp), %xmm1
	movq	208(%rsp), %rcx
	movq	8(%rbp), %rbx
	movabsq	$9223372036854775807, %r15
	movq	%rbp, 16(%rsp)
	vmovups	%zmm0, 32(%r14)
	movq	%rcx, 24(%r14)
	vmovups	%xmm1, 8(%r14)
	movq	%rax, (%r14)
	testq	%r12, %r12
	je	.LBB191_15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r14
	xorl	%r13d, %r13d
	jmp	.LBB191_7
	.p2align	4
.LBB191_4:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_5:
	vzeroupper
	callq	*%r14
.LBB191_6:
	incq	%r13
	cmpq	%r12, %r13
	je	.LBB191_15
.LBB191_7:
	leaq	(%r13,%r13,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB191_6
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
	jge	.LBB191_10
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_10:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_5
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_10
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
	movq	(%rbp), %rax
	.p2align	4
.LBB191_13:
	cmpq	%rax, %rdx
	jge	.LBB191_4
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB191_13
	jmp	.LBB191_4
.LBB191_15:
	movq	16(%rsp), %rax
	movq	(%rax), %rax
	testq	%rax, %rax
	je	.LBB191_61
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r15, %rcx
	cmovaeq	%r15, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r15, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB191_18
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_18:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_24
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_18
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r15, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB191_21:
	cmpq	%rax, %rdx
	jge	.LBB191_23
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB191_21
.LBB191_23:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_24:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB191_61
.LBB191_25:
	movq	208(%rsp), %rax
	vmovdqa	192(%rsp), %xmm0
	movq	8(%rbp), %rcx
	movq	(%rbp), %rsi
	leaq	(%r12,%r12,4), %rdx
	movq	%r14, 384(%rsp)
	movq	%rax, 176(%rsp)
	movq	616(%r13), %rax
	leaq	(%rcx,%rdx,8), %rdx
	movq	%rcx, 352(%rsp)
	movq	%rsi, 368(%rsp)
	movq	%rcx, 360(%rsp)
	movq	%rdx, 376(%rsp)
	vmovdqa	%xmm0, 160(%rsp)
	testq	%rax, %rax
	je	.LBB191_31
	movq	%r15, 608(%rsp)
	lock		incq	(%rax)
	jle	.LBB191_363
	movq	616(%r13), %rax
	movq	%rax, 408(%rsp)
	movq	%rax, 8(%rsp)
	movq	16(%rax), %rcx
	movq	%rcx, 144(%rsp)
	movq	40(%rax), %rcx
	movq	240(%rsp), %rax
	movq	8(%rax), %rdx
	movq	16(%rax), %r15
	movq	%rdx, 120(%rsp)
	movq	%rcx, 88(%rsp)
	movq	%r13, 72(%rsp)
	cmpq	$-1, %rcx
	je	.LBB191_76
	testq	%r15, %r15
	je	.LBB191_62
	cmpq	$8, %r15
	jae	.LBB191_63
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB191_72
.LBB191_31:
	movq	360(%rsp), %rcx
	movq	352(%rsp), %rax
	movq	368(%rsp), %rdx
	movq	%rcx, 440(%rsp)
	movq	376(%rsp), %rcx
	movq	%rax, 432(%rsp)
	movq	%rdx, 448(%rsp)
	movq	%rcx, 456(%rsp)
	movq	440(%rsp), %r15
	movq	456(%rsp), %r12
	cmpq	%r12, %r15
	je	.LBB191_37
	addq	$40, %r15
	movq	%r15, %rax
	jmp	.LBB191_34
	.p2align	4
.LBB191_33:
	movq	168(%rsp), %rcx
	leaq	(%rbx,%rbx,4), %rax
	incq	%rbx
	addq	$40, %r13
	movq	%rbp, (%rcx,%rax,8)
	movq	%r14, 8(%rcx,%rax,8)
	vmovdqa	688(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rcx,%rax,8)
	movq	704(%rsp), %rdx
	movq	%rdx, 32(%rcx,%rax,8)
	leaq	40(%r15), %rax
	movq	%rbx, 176(%rsp)
	cmpq	%r12, %r13
	je	.LBB191_37
.LBB191_34:
	movq	-40(%rax), %rbp
	movq	%rax, %r15
	testq	%rbp, %rbp
	je	.LBB191_37
	leaq	-40(%r15), %r13
	movq	-32(%r15), %r14
	movq	176(%rsp), %rbx
	movq	32(%r13), %rax
	movq	%rax, 704(%rsp)
	vmovdqu	16(%r13), %xmm0
	vmovdqa	%xmm0, 688(%rsp)
	cmpq	160(%rsp), %rbx
	jne	.LBB191_33
.Ltmp4431:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	callq	*%rax
.Ltmp4432:
	jmp	.LBB191_33
.LBB191_37:
	subq	%r15, %r12
	je	.LBB191_50
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %rbx
	xorl	%r14d, %r14d
	imulq	%r12, %rbx
	movabsq	$9223372036854775807, %r12
	jmp	.LBB191_42
	.p2align	4
.LBB191_39:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_40:
	callq	*%r13
.LBB191_41:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB191_50
.LBB191_42:
	leaq	(%r14,%r14,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB191_41
	leaq	(%r15,%rcx,8), %rdx
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
	jge	.LBB191_45
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_45:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_40
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_45
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
.LBB191_48:
	cmpq	%rax, %rdx
	jge	.LBB191_39
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB191_48
	jmp	.LBB191_39
.LBB191_50:
	movq	448(%rsp), %rax
	movq	384(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB191_60
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rdx
	movq	432(%rsp), %rdi
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
	jge	.LBB191_53
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_53:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_59
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_53
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
.LBB191_56:
	cmpq	%rax, %rsi
	jge	.LBB191_58
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB191_56
.LBB191_58:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_59:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB191_60:
	vmovaps	160(%rsp), %xmm0
	movq	176(%rsp), %rax
	movq	%rax, 24(%rbx)
	vmovups	%xmm0, 8(%rbx)
	movb	$2, 32(%rbx)
	movq	$-1, (%rbx)
.LBB191_61:
	movq	240(%rsp), %rdi
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
	jmp	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.LBB191_62:
	.cfi_def_cfa_offset 800
	xorl	%r15d, %r15d
	jmp	.LBB191_76
.LBB191_63:
	cmpq	$32, %r15
	jae	.LBB191_65
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB191_69
.LBB191_65:
	vmovdqa64	.LCPI191_0(%rip), %zmm1
	vpbroadcastq	.LCPI191_1(%rip), %zmm2
	vpbroadcastq	.LCPI191_2(%rip), %zmm3
	movq	120(%rsp), %rdx
	movq	%r15, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB191_66:
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
	jne	.LBB191_66
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
	cmpq	%rax, %r15
	je	.LBB191_74
	testb	$24, %r15b
	je	.LBB191_72
.LBB191_69:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI191_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI191_1(%rip), %zmm2
	vpbroadcastq	.LCPI191_3(%rip), %zmm3
	movq	120(%rsp), %rdx
	movq	%r15, %rax
	andq	$-8, %rax
	vmovq	%r14, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB191_70:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rdx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB191_70
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r14
	cmpq	%rax, %r15
	je	.LBB191_74
.LBB191_72:
	movq	120(%rsp), %rdx
	movq	%r15, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rdx), %rax
	.p2align	4
.LBB191_73:
	addq	(%rax), %r14
	addq	$160, %rax
	decq	%rcx
	jne	.LBB191_73
.LBB191_74:
	movq	912(%r13), %rax
	movq	928(%r13), %rsi
	leaq	912(%r13), %r12
	subq	%rsi, %rax
	cmpq	%rax, %r14
	ja	.LBB191_364
.LBB191_75:
	movq	72(%rsp), %rax
	cmpq	1016(%rax), %r14
	ja	.LBB191_365
.LBB191_76:
	movq	8(%rsp), %r14
	leaq	(%r15,%r15,4), %rdx
	shlq	$5, %rdx
	leaq	16(%r14), %rax
	movq	%rax, 56(%rsp)
	movq	240(%rsp), %rax
	movq	120(%rsp), %rcx
	movq	(%rax), %rax
	addq	%rcx, %rdx
	movq	%rcx, 648(%rsp)
	movq	%rcx, 656(%rsp)
	movq	%rdx, 616(%rsp)
	movq	%rax, 664(%rsp)
	movq	%rdx, 672(%rsp)
	testq	%r15, %r15
	je	.LBB191_275
	movzbl	%bl, %eax
	leaq	272(%r14), %rcx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	movq	$-1, %rbp
	movq	%rax, 600(%rsp)
	movq	%rcx, 232(%rsp)
	movq	72(%rsp), %rax
	addq	$888, %rax
	movq	%rax, 152(%rsp)
.LBB191_78:
	movq	120(%rsp), %rcx
	leaq	160(%rcx), %rdx
	movq	%rdx, 656(%rsp)
	movq	%rdx, 120(%rsp)
	movq	(%rcx), %rax
	cmpq	$-1, %rax
	je	.LBB191_275
	movq	%rax, 432(%rsp)
	vmovdqu64	8(%rcx), %zmm0
	vmovdqu64	72(%rcx), %zmm1
	vmovdqu64	96(%rcx), %zmm2
	leaq	440(%rsp), %rcx
	vmovdqu64	%zmm2, 88(%rcx)
	vmovdqu64	%zmm1, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	456(%rsp), %rcx
	movq	464(%rsp), %rdx
	movq	440(%rsp), %r15
	movq	472(%rsp), %rsi
	movq	%rcx, 24(%rsp)
	movq	%rdx, 80(%rsp)
	imulq	$88, 448(%rsp), %rdx
	movq	%r15, 192(%rsp)
	movq	%rax, 208(%rsp)
	movq	488(%rsp), %rax
	movq	480(%rsp), %rcx
	movq	%rsi, 640(%rsp)
	movq	%r15, 200(%rsp)
	addq	%r15, %rdx
	movq	%rdx, 48(%rsp)
	movq	%rax, 336(%rsp)
	movq	%rcx, 32(%rsp)
	movq	496(%rsp), %rcx
	movq	%rdx, 216(%rsp)
	testq	%rcx, %rcx
	je	.LBB191_247
	movq	%rcx, %rax
	movq	336(%rsp), %rcx
	movq	528(%rsp), %rdx
	shlq	$5, %rax
	movq	360(%rsp), %r12
	addq	%rcx, %rax
	movq	%rax, 632(%rsp)
	movq	%rdx, 16(%rsp)
	movq	80(%rsp), %rax
	addq	$8, %rax
	movq	%rax, 624(%rsp)
	movq	$0, 248(%rsp)
	movq	%r15, 40(%rsp)
	jmp	.LBB191_83
	.p2align	4
.LBB191_81:
	movq	424(%rsp), %r12
.LBB191_82:
	addq	$32, %r13
	movq	%r12, 360(%rsp)
	movq	%r13, %rcx
	cmpq	632(%rsp), %r13
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	je	.LBB191_247
.LBB191_83:
	movq	%r15, 64(%rsp)
	movq	16(%rcx), %rdx
	movq	(%rcx), %r15
	movq	8(%rcx), %rbx
	movq	248(%rsp), %rax
	movq	%rax, 344(%rsp)
	movq	%rdx, 136(%rsp)
	movq	%rcx, 128(%rsp)
	movq	24(%rcx), %rcx
	movq	%rcx, 400(%rsp)
	testq	%r15, %r15
	je	.LBB191_93
	cmpq	$-1, 144(%rsp)
	je	.LBB191_93
	movq	80(%r14), %rax
	.p2align	4
.LBB191_86:
	movq	%rax, %rcx
	addq	%r15, %rcx
	cmovbq	%rbp, %rcx
	lock		cmpxchgq	%rcx, 80(%r14)
	jne	.LBB191_86
	movq	56(%rsp), %rcx
	addq	%r15, %rax
	cmovbq	%rbp, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB191_90
	movq	%rcx, 264(%rsp)
	movq	%rax, 272(%rsp)
	movw	$0, 256(%rsp)
.Ltmp4354:
	movq	56(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4355:
	cmpb	$-1, 96(%rsp)
	jne	.LBB191_303
.LBB191_90:
	movq	72(%rsp), %rax
	movq	632(%rax), %rax
	testq	%rax, %rax
	je	.LBB191_93
	movq	72(%rsp), %rcx
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB191_93
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	600(%rsp), %rax
	lock		addq	%r15, (%rcx,%rax,8)
	.p2align	4
.LBB191_93:
	movq	640(%rsp), %rdx
	movq	344(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB191_362
	movq	%r12, 424(%rsp)
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	movq	24(%rsp), %r15
	movq	32(%rsp), %r12
	cmovbq	%rbx, %rsi
	cmpq	%rdi, %rbx
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB191_361
	leaq	(,%rdi,8), %rax
	leaq	(%rax,%rax,2), %rbp
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r15
	cmpq	%rsi, %rdi
	jne	.LBB191_103
	movq	16(%rsp), %r8
	movq	40(%rsp), %r10
	xorl	%ebx, %ebx
	xorl	%r14d, %r14d
	xorl	%r11d, %r11d
.LBB191_97:
	movq	%r14, 416(%rsp)
	movq	%r11, 392(%rsp)
	movq	%rsi, 248(%rsp)
	cmpq	$-1, 88(%rsp)
	je	.LBB191_109
	movq	136(%rsp), %rax
	movq	48(%rsp), %r13
	movl	$0, %ecx
	movl	$0, %r14d
	subq	%r8, %rax
	cmovbq	%rcx, %rax
	subq	%r10, %r13
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %r13
	imulq	%rcx, %r13
	cmpq	%r13, %rax
	cmovbq	%rax, %r13
	testq	%r13, %r13
	je	.LBB191_102
	movq	40(%rsp), %rax
	xorl	%r14d, %r14d
	leaq	8(%rax), %r12
	.p2align	4
.LBB191_100:
.Ltmp4357:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
.Ltmp4358:
	addq	%rax, %r14
	movq	$-1, %rcx
	cmovbq	%rcx, %r14
	addq	$88, %r12
	decq	%r13
	jne	.LBB191_100
.LBB191_102:
	movq	8(%rsp), %rax
	movl	296(%rax), %eax
	testl	%eax, %eax
	jne	.LBB191_111
	jmp	.LBB191_110
	.p2align	4
.LBB191_103:
	movq	%r15, %rdx
	subq	%rbp, %rdx
	movabsq	$-6148914691236517205, %rax
	leaq	.LJTI191_0(%rip), %rdi
	xorl	%r11d, %r11d
	xorl	%r14d, %r14d
	xorl	%ebx, %ebx
	mulxq	%rax, %rax, %rax
	movq	624(%rsp), %rcx
	movq	16(%rsp), %r8
	movq	40(%rsp), %r10
	shrq	$4, %rax
	addq	%rbp, %rcx
	jmp	.LBB191_106
	.p2align	4
.LBB191_104:
	addq	%rdx, %r14
	movq	$-1, %r9
	cmovbq	%r9, %r14
.LBB191_105:
	addq	$24, %rcx
	decq	%rax
	je	.LBB191_97
.LBB191_106:
	movzbl	-8(%rcx), %r9d
	movq	(%rcx), %rdx
	movslq	(%rdi,%r9,4), %r9
	addq	%rdi, %r9
	jmpq	*%r9
.LBB191_107:
	addq	%rdx, %rbx
	movq	$-1, %r9
	cmovbq	%r9, %rbx
	jmp	.LBB191_105
	.p2align	4
.LBB191_108:
	cmpq	%rdx, %r11
	cmovbeq	%rdx, %r11
	jmp	.LBB191_105
	.p2align	4
.LBB191_109:
	xorl	%r14d, %r14d
	movq	8(%rsp), %rax
	movl	296(%rax), %eax
	testl	%eax, %eax
	jne	.LBB191_111
.LBB191_110:
	movq	232(%rsp), %rax
	cmpb	$-1, (%rax)
	jne	.LBB191_115
	.p2align	4
.LBB191_111:
	cmpq	$-1, 144(%rsp)
	je	.LBB191_113
	movq	8(%rsp), %rcx
	movq	$-1, %rdx
	movq	80(%rcx), %rax
	addq	%rbx, %rax
	cmovbq	%rdx, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB191_115
.LBB191_113:
	cmpq	$-1, 88(%rsp)
	je	.LBB191_125
	movq	72(%rsp), %rcx
	movq	$-1, %rsi
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	8(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rsi, %rcx
	addq	416(%rsp), %rcx
	cmovbq	%rsi, %rcx
	addq	392(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%rsi, %rcx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	cmpq	40(%rdx), %rax
	jbe	.LBB191_125
.LBB191_115:
	movq	40(%rsp), %r13
	movq	344(%rsp), %rax
	cmpq	248(%rsp), %rax
	jne	.LBB191_145
	movq	8(%rsp), %r14
.LBB191_117:
	movq	%r13, 40(%rsp)
	cmpq	$-1, 88(%rsp)
	je	.LBB191_198
	movq	16(%rsp), %r12
	movq	64(%rsp), %r15
	movq	128(%rsp), %r13
	movq	$-1, %rbp
	cmpq	136(%rsp), %r12
	jae	.LBB191_235
	movq	40(%rsp), %rax
	cmpq	48(%rsp), %rax
	je	.LBB191_209
	movq	136(%rsp), %rax
	leaq	-1(%rax), %rbx
	movq	40(%rsp), %rax
	.p2align	4
.LBB191_121:
	movq	%rax, %r15
	movq	8(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB191_208
	movq	(%r15), %rsi
	movq	%rax, 256(%rsp)
	leaq	264(%rsp), %rcx
	movq	80(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4402:
	movq	152(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4403:
	cmpq	%r12, %rbx
	je	.LBB191_207
	leaq	88(%r15), %rax
	incq	%r12
	cmpq	48(%rsp), %rax
	jne	.LBB191_121
	jmp	.LBB191_208
	.p2align	4
.LBB191_125:
	cmpq	$-1, 144(%rsp)
	movq	8(%rsp), %r14
	je	.LBB191_127
	movq	344(%rsp), %rcx
	movq	%rbp, %rax
	cmpq	248(%rsp), %rcx
	jne	.LBB191_140
.LBB191_127:
	movb	$1, %r12b
	movq	344(%rsp), %rax
	cmpq	248(%rsp), %rax
	je	.LBB191_130
	.p2align	4
.LBB191_128:
	movq	80(%rsp), %rax
	cmpb	$2, -24(%rax,%r15)
	je	.LBB191_200
	addq	$-24, %r15
	cmpq	%r15, %rbp
	jne	.LBB191_128
.LBB191_130:
	movq	64(%rsp), %r15
	movq	$-1, %rbp
.LBB191_131:
	cmpq	$-1, 144(%rsp)
	je	.LBB191_189
	testq	%rbx, %rbx
	je	.LBB191_189
	movq	80(%r14), %rax
	.p2align	4
.LBB191_134:
	movq	%rax, %rcx
	addq	%rbx, %rcx
	cmovbq	%rbp, %rcx
	lock		cmpxchgq	%rcx, 80(%r14)
	jne	.LBB191_134
	movq	56(%rsp), %rcx
	addq	%rbx, %rax
	cmovbq	%rbp, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB191_189
	movq	%rcx, 264(%rsp)
	movq	%rax, 272(%rsp)
	movw	$0, 256(%rsp)
.Ltmp4363:
	movq	56(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4364:
	cmpb	$-1, 96(%rsp)
	je	.LBB191_189
	cmpq	$-1, 88(%rsp)
	movb	$1, %bl
	jne	.LBB191_220
	jmp	.LBB191_305
	.p2align	4
.LBB191_139:
	addq	$24, %rax
	cmpq	%rax, %r15
	je	.LBB191_127
.LBB191_140:
	movq	80(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB191_139
	movq	80(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB191_139
	movq	72(%rsp), %rdx
	movq	632(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB191_139
	movq	72(%rsp), %rsi
	movl	1228(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB191_139
	movq	80(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB191_139
	.p2align	4
.LBB191_145:
	movq	80(%rsp), %rax
	movq	8(%rsp), %r14
	addq	%rax, %rbp
	addq	%rax, %r15
	jmp	.LBB191_148
.LBB191_146:
	movq	8(%rsp), %r14
	.p2align	4
.LBB191_147:
	addq	$24, %rbp
	cmpq	%r15, %rbp
	je	.LBB191_117
.LBB191_148:
	movzbl	(%rbp), %eax
	leaq	.LJTI191_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB191_149:
	cmpq	$-1, 144(%rsp)
	je	.LBB191_147
	movq	%r14, %rdx
	movzbl	1(%rbp), %ebx
	movq	8(%rbp), %r14
	movq	16(%rbp), %r12
	movq	80(%rdx), %rax
	movq	$-1, %rsi
	.p2align	4
.LBB191_151:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB191_151
	movq	56(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB191_155
	movq	%rcx, 264(%rsp)
	movq	%rax, 272(%rsp)
	movw	$0, 256(%rsp)
.Ltmp4389:
	movq	56(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4390:
	cmpb	$-1, 96(%rsp)
	jne	.LBB191_286
.LBB191_155:
	cmpl	$255, %ebx
	je	.LBB191_146
	movq	72(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB191_146
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB191_146
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r14, (%rcx,%rbx,8)
	jmp	.LBB191_146
	.p2align	4
.LBB191_159:
	movq	8(%rbp), %rbx
	cmpq	%rbx, 16(%rsp)
	jae	.LBB191_186
	cmpq	48(%rsp), %r13
	je	.LBB191_179
	leaq	-1(%rbx), %r14
	.p2align	4
.LBB191_162:
	movq	8(%r13), %rax
	movq	%r13, %rdx
	cmpq	$-1, %rax
	je	.LBB191_184
	movq	(%rdx), %rsi
	movq	%rax, 256(%rsp)
	leaq	264(%rsp), %rcx
	movq	%rdx, %r12
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4378:
	movq	152(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4379:
	movq	16(%rsp), %rax
	cmpq	%rax, %r14
	je	.LBB191_183
	incq	%rax
	leaq	88(%r12), %r13
	movq	%r12, %rdx
	movq	%rax, 16(%rsp)
	cmpq	48(%rsp), %r13
	jne	.LBB191_162
	jmp	.LBB191_184
	.p2align	4
.LBB191_166:
	cmpq	$-1, 88(%rsp)
	je	.LBB191_147
	movq	8(%rbp), %rcx
	movl	296(%r14), %eax
	testl	%eax, %eax
	je	.LBB191_178
	movq	104(%r14), %rax
	movq	$-1, %rdx
	addq	%rcx, %rax
	movq	40(%r14), %rcx
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB191_147
	movq	%rcx, 264(%rsp)
	movq	%rax, 272(%rsp)
	movw	$768, 256(%rsp)
.Ltmp4376:
	movq	56(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4377:
	cmpb	$-1, 96(%rsp)
	je	.LBB191_147
	jmp	.LBB191_304
	.p2align	4
.LBB191_171:
	cmpq	$-1, 88(%rsp)
	je	.LBB191_147
	cmpq	$-1, 40(%r14)
	je	.LBB191_147
	movq	8(%rbp), %rcx
	movq	16(%rbp), %r12
	movl	296(%r14), %eax
	testl	%eax, %eax
	je	.LBB191_180
	movq	104(%r14), %rax
	movq	$-1, %rsi
	.p2align	4
.LBB191_175:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 104(%r14)
	jne	.LBB191_175
	addq	%rcx, %rax
	movq	40(%r14), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB191_147
	movq	%rcx, 264(%rsp)
	movq	%rax, 272(%rsp)
	movw	$768, 256(%rsp)
.Ltmp4383:
	movq	56(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4384:
	jmp	.LBB191_181
.LBB191_178:
	movq	232(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 112(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
	cmpb	$-1, 96(%rsp)
	je	.LBB191_147
	jmp	.LBB191_304
.LBB191_179:
	movq	64(%rsp), %rdx
	jmp	.LBB191_185
.LBB191_180:
	movq	232(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 112(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
.LBB191_181:
	movzbl	96(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%r12, %r12
	setne	%dl
	testb	%cl, %dl
	jne	.LBB191_294
	cmpb	$-1, %al
	je	.LBB191_147
	jmp	.LBB191_304
.LBB191_183:
	movq	%r12, %rdx
	movq	%rbx, 16(%rsp)
.LBB191_184:
	movq	8(%rsp), %r14
	addq	$88, %rdx
	movq	%rdx, %r13
.LBB191_185:
	movq	%rdx, 64(%rsp)
	movq	%rdx, 200(%rsp)
.LBB191_186:
	cmpq	$-1, 88(%rsp)
	je	.LBB191_147
.Ltmp4381:
	movq	72(%rsp), %rsi
	leaq	256(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp4382:
	cmpb	$-1, 256(%rsp)
	je	.LBB191_147
	jmp	.LBB191_304
	.p2align	4
.LBB191_189:
	cmpq	$-1, 88(%rsp)
	je	.LBB191_199
	cmpq	$-1, 40(%r14)
	je	.LBB191_212
	movl	296(%r14), %eax
	testl	%eax, %eax
	je	.LBB191_211
	movq	104(%r14), %rax
	movq	416(%rsp), %rdx
	.p2align	4
.LBB191_193:
	movq	%rax, %rcx
	addq	%rdx, %rcx
	cmovbq	%rbp, %rcx
	lock		cmpxchgq	%rcx, 104(%r14)
	jne	.LBB191_193
	movq	40(%r14), %rcx
	addq	%rdx, %rax
	cmovbq	%rbp, %rax
	cmpq	%rcx, %rax
	jbe	.LBB191_212
	movq	%rcx, 264(%rsp)
	movq	%rax, 272(%rsp)
	movw	$768, 256(%rsp)
.Ltmp4366:
	movq	56(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4367:
	cmpb	$-1, 96(%rsp)
	je	.LBB191_212
.LBB191_197:
	movb	$1, %bl
	movq	16(%rsp), %r12
	cmpq	136(%rsp), %r12
	jb	.LBB191_221
	jmp	.LBB191_233
.LBB191_198:
	movq	64(%rsp), %r15
	movq	$-1, %rbp
.LBB191_199:
	movq	16(%rsp), %r12
	movq	128(%rsp), %r13
	jmp	.LBB191_235
.LBB191_200:
	movq	80(%rsp), %rax
	movq	$-1, %rbp
	movq	-16(%rax,%r15), %r14
	cmpq	%r14, 16(%rsp)
	jae	.LBB191_210
	movq	40(%rsp), %rcx
	movq	64(%rsp), %r15
	cmpq	48(%rsp), %rcx
	je	.LBB191_246
	leaq	-1(%r14), %r15
	.p2align	4
.LBB191_203:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB191_245
	movq	(%rdx), %rsi
	movq	%rax, 256(%rsp)
	leaq	264(%rsp), %rcx
	movq	%rdx, %r12
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4360:
	movq	152(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4361:
	movq	16(%rsp), %rax
	cmpq	%rax, %r15
	je	.LBB191_244
	incq	%rax
	leaq	88(%r12), %rcx
	movq	%r12, %rdx
	movq	%rax, 16(%rsp)
	cmpq	48(%rsp), %rcx
	jne	.LBB191_203
	jmp	.LBB191_245
.LBB191_207:
	movq	136(%rsp), %r12
.LBB191_208:
	addq	$88, %r15
	movq	%r15, 40(%rsp)
.LBB191_209:
	movq	%r15, 200(%rsp)
	jmp	.LBB191_235
.LBB191_210:
	movq	8(%rsp), %r14
	movq	64(%rsp), %r15
	xorl	%r12d, %r12d
	jmp	.LBB191_131
.LBB191_211:
	movq	232(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 112(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
	cmpb	$-1, 96(%rsp)
	jne	.LBB191_197
.LBB191_212:
	testb	%r12b, %r12b
	jne	.LBB191_215
.Ltmp4368:
	movq	72(%rsp), %rsi
	leaq	256(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp4369:
	cmpb	$-1, 256(%rsp)
	movb	$1, %bl
	jne	.LBB191_220
.LBB191_215:
	cmpq	$0, 392(%rsp)
	je	.LBB191_219
	movl	296(%r14), %eax
	testl	%eax, %eax
	je	.LBB191_230
	movq	104(%r14), %rax
	movq	40(%r14), %rcx
	addq	392(%rsp), %rax
	cmovbq	%rbp, %rax
	cmpq	%rcx, %rax
	jbe	.LBB191_231
	movq	%rcx, 264(%rsp)
	movq	%rax, 272(%rsp)
	movw	$768, 256(%rsp)
.Ltmp4370:
	movq	56(%rsp), %rsi
	leaq	96(%rsp), %rdi
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4371:
	jmp	.LBB191_232
.LBB191_219:
	xorl	%ebx, %ebx
	.p2align	4
.LBB191_220:
	movq	16(%rsp), %r12
	cmpq	136(%rsp), %r12
	jae	.LBB191_233
.LBB191_221:
	movq	128(%rsp), %r13
	movq	40(%rsp), %rax
	cmpq	48(%rsp), %rax
	je	.LBB191_229
	movq	136(%rsp), %rax
	leaq	-1(%rax), %r14
	movq	40(%rsp), %rax
	.p2align	4
.LBB191_223:
	movq	%rax, %r15
	movq	8(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB191_228
	movq	(%r15), %rsi
	movq	%rax, 256(%rsp)
	leaq	264(%rsp), %rcx
	movq	80(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4373:
	movq	152(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	256(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4374:
	cmpq	%r12, %r14
	je	.LBB191_227
	leaq	88(%r15), %rax
	incq	%r12
	cmpq	48(%rsp), %rax
	jne	.LBB191_223
	jmp	.LBB191_228
.LBB191_227:
	movq	136(%rsp), %r12
.LBB191_228:
	addq	$88, %r15
	movq	%r15, 40(%rsp)
	movq	8(%rsp), %r14
.LBB191_229:
	movq	%r15, 200(%rsp)
	jmp	.LBB191_234
.LBB191_230:
	movq	232(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 112(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 96(%rsp)
	jmp	.LBB191_232
.LBB191_231:
	movb	$-1, 96(%rsp)
.LBB191_232:
	cmpb	$-1, 96(%rsp)
	setne	%bl
	movq	16(%rsp), %r12
	cmpq	136(%rsp), %r12
	jb	.LBB191_221
.LBB191_233:
	movq	128(%rsp), %r13
.LBB191_234:
	testb	%bl, %bl
	jne	.LBB191_304
.LBB191_235:
	movq	%r12, 16(%rsp)
	cmpq	$0, 400(%rsp)
	je	.LBB191_81
	movq	376(%rsp), %rbx
	movq	424(%rsp), %rax
	movq	%r15, %r13
	jmp	.LBB191_238
	.p2align	4
.LBB191_237:
	movq	168(%rsp), %rax
	leaq	(%r12,%r12,4), %rcx
	movq	400(%rsp), %rdi
	leaq	40(%r14), %rsi
	incq	%r12
	movq	%r15, (%rax,%rcx,8)
	movq	%rbp, 8(%rax,%rcx,8)
	decq	%rdi
	movq	$-1, %rbp
	vmovdqa	720(%rsp), %xmm0
	movq	%rdi, 400(%rsp)
	vmovdqu	%xmm0, 16(%rax,%rcx,8)
	movq	736(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r12, 176(%rsp)
	movq	%rsi, %rax
	testq	%rdi, %rdi
	je	.LBB191_242
.LBB191_238:
	movq	%rax, %r14
	cmpq	%rbx, %rax
	je	.LBB191_243
	movq	(%r14), %r15
	testq	%r15, %r15
	je	.LBB191_242
	movq	32(%r14), %rax
	movq	8(%r14), %rbp
	movq	176(%rsp), %r12
	movq	%rax, 736(%rsp)
	vmovdqu	16(%r14), %xmm0
	vmovdqa	%xmm0, 720(%rsp)
	cmpq	160(%rsp), %r12
	jne	.LBB191_237
.Ltmp4405:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp4406:
	jmp	.LBB191_237
	.p2align	4
.LBB191_242:
	addq	$40, %r14
.LBB191_243:
	movq	%r14, %r12
	movq	%r13, %r15
	movq	8(%rsp), %r14
	movq	128(%rsp), %r13
	jmp	.LBB191_82
.LBB191_244:
	movq	%r12, %rdx
	movq	%r14, 16(%rsp)
.LBB191_245:
	addq	$88, %rdx
	movq	%rdx, %r15
	movq	%rdx, 40(%rsp)
.LBB191_246:
	movq	8(%rsp), %r14
	xorl	%r12d, %r12d
	movq	%r15, 200(%rsp)
	jmp	.LBB191_131
.LBB191_247:
	movq	32(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB191_257
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$5, %rdi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	movq	%rdi, %rsi
	cmpq	%rdi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB191_250
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_250:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_256
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_250
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%r13), %rax
	.p2align	4
.LBB191_253:
	cmpq	%rax, %rcx
	jge	.LBB191_255
	lock		cmpxchgq	%rcx, (%r13)
	jne	.LBB191_253
.LBB191_255:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_256:
	movq	336(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB191_257:
.Ltmp4415:
	leaq	192(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp4416:
	movq	24(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_268
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
	jge	.LBB191_261
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_261:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_267
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_261
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
.LBB191_264:
	cmpq	%rax, %rdx
	jge	.LBB191_266
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB191_264
.LBB191_266:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_267:
	movq	80(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB191_268:
	movq	520(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_271
	lock		decq	(%rax)
	jne	.LBB191_271
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB191_271:
	movq	552(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_274
	lock		decq	(%rax)
	jne	.LBB191_274
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	552(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB191_274:
	movq	616(%rsp), %rax
	cmpq	%rax, 120(%rsp)
	jne	.LBB191_78
.LBB191_275:
	movb	$1, %bpl
	xorl	%r13d, %r13d
.Ltmp4420:
	leaq	648(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4421:
	movq	608(%rsp), %rdx
	cmpq	$-1, 144(%rsp)
	movq	240(%rsp), %rcx
	movq	8(%rsp), %r14
	sete	%al
	notb	%dl
	orb	24(%rcx), %dl
	orb	%al, %dl
	testb	$1, %dl
	jne	.LBB191_281
	movq	80(%r14), %rax
	movq	$-1, %rcx
	.p2align	4
.LBB191_278:
	movq	%rax, %rdx
	incq	%rdx
	cmoveq	%rcx, %rdx
	lock		cmpxchgq	%rdx, 80(%r14)
	jne	.LBB191_278
	movq	56(%rsp), %rsi
	incq	%rax
	movq	$-1, %rcx
	cmovneq	%rax, %rcx
	movq	(%rsi), %rax
	cmpq	%rax, %rcx
	jbe	.LBB191_281
	movq	%rax, 440(%rsp)
	movq	%rcx, 448(%rsp)
	movw	$0, 432(%rsp)
.Ltmp4423:
	leaq	256(%rsp), %rdi
	leaq	432(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4424:
.LBB191_281:
	vmovdqa	160(%rsp), %xmm0
	movq	384(%rsp), %rcx
	movq	176(%rsp), %rax
	movq	%rax, 24(%rcx)
	vmovdqu	%xmm0, 8(%rcx)
	movb	$2, 32(%rcx)
	movq	$-1, (%rcx)
	lock		decq	(%r14)
	jne	.LBB191_283
	#MEMBARRIER
.Ltmp4428:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp4429:
.LBB191_283:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB191_360
.LBB191_286:
	movq	8(%rsp), %r14
	movq	%r12, %rax
	addq	$-1, %rax
	movb	$1, %bl
	jae	.LBB191_305
	movq	64(%rsp), %r15
	cmpq	%rax, 16(%rsp)
	jae	.LBB191_305
	cmpq	48(%rsp), %r13
	je	.LBB191_301
	subq	16(%rsp), %r12
	addq	$88, %r13
	leaq	256(%rsp), %r14
	addq	$-2, %r12
.LBB191_290:
	movq	-80(%r13), %rax
	movq	%r13, %r15
	cmpq	$-1, %rax
	je	.LBB191_301
	movq	-88(%r15), %rsi
	movq	%rax, 256(%rsp)
	leaq	264(%rsp), %rcx
	movq	-8(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4392:
	movq	152(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp4393:
	subq	$1, %r12
	jb	.LBB191_301
	leaq	88(%r15), %r13
	cmpq	48(%rsp), %r15
	jne	.LBB191_290
	jmp	.LBB191_301
.LBB191_294:
	movq	64(%rsp), %r15
	leaq	-1(%r12), %rax
	movb	$1, %bl
	cmpq	%rax, 16(%rsp)
	jae	.LBB191_305
	cmpq	48(%rsp), %r13
	je	.LBB191_301
	subq	16(%rsp), %r12
	addq	$88, %r13
	leaq	256(%rsp), %r14
	addq	$-2, %r12
.LBB191_297:
	movq	-80(%r13), %rax
	movq	%r13, %r15
	cmpq	$-1, %rax
	je	.LBB191_301
	movq	-88(%r15), %rsi
	movq	%rax, 256(%rsp)
	leaq	264(%rsp), %rcx
	movq	-8(%r15), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%r15), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4386:
	movq	152(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp4387:
	subq	$1, %r12
	jb	.LBB191_301
	leaq	88(%r15), %r13
	cmpq	48(%rsp), %r15
	jne	.LBB191_297
.LBB191_301:
	movq	8(%rsp), %r14
	movq	%r15, 200(%rsp)
.LBB191_304:
	movb	$1, %bl
.LBB191_305:
	vmovdqa	160(%rsp), %xmm0
	movq	384(%rsp), %rcx
	movq	176(%rsp), %rax
	movq	32(%rsp), %rdi
	movq	%rax, 24(%rcx)
	vmovdqu	%xmm0, 8(%rcx)
	movb	%bl, 32(%rcx)
	movq	$-1, (%rcx)
	testq	%rdi, %rdi
	je	.LBB191_316
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$5, %rdi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rdi
	cmovaeq	%rdx, %rdi
	xorl	%ecx, %ecx
	movq	%rdi, %rsi
	cmpq	%rdi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rdi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB191_308
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB191_308:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdi
	.p2align	4
.LBB191_309:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_315
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_309
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rsi, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rdx
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%rsi, %rcx
	setns	%al
	addq	%rdx, %rax
	subq	%rsi, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdi), %rax
	.p2align	4
.LBB191_312:
	cmpq	%rax, %rcx
	jge	.LBB191_314
	lock		cmpxchgq	%rcx, (%rdi)
	jne	.LBB191_312
.LBB191_314:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_315:
	movq	336(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB191_316:
.Ltmp4395:
	leaq	192(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp4396:
	movq	24(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_328
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
	jge	.LBB191_320
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB191_320:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdi
	.p2align	4
.LBB191_321:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_327
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_321
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
	movq	(%rdi), %rax
	.p2align	4
.LBB191_324:
	cmpq	%rax, %rdx
	jge	.LBB191_326
	lock		cmpxchgq	%rdx, (%rdi)
	jne	.LBB191_324
.LBB191_326:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_327:
	movq	80(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB191_328:
	movq	520(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_331
	lock		decq	(%rax)
	jne	.LBB191_331
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB191_331:
	movq	552(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_334
	lock		decq	(%rax)
	jne	.LBB191_334
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	552(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB191_334:
	xorl	%ebp, %ebp
.Ltmp4398:
	leaq	648(%rsp), %rdi
	xorl	%r13d, %r13d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4399:
	movq	8(%rsp), %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	lock		decq	(%rax)
	jne	.LBB191_337
	#MEMBARRIER
.Ltmp4400:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp4401:
.LBB191_337:
	movq	360(%rsp), %rbx
	movq	376(%rsp), %rax
	movabsq	$9223372036854775807, %r13
	subq	%rbx, %rax
	je	.LBB191_350
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %r14
	xorl	%r15d, %r15d
	imulq	%rax, %r14
	jmp	.LBB191_342
	.p2align	4
.LBB191_339:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_340:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB191_341:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB191_350
.LBB191_342:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB191_341
	leaq	(%rbx,%rcx,8), %rdx
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
	jge	.LBB191_345
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_345:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_340
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_345
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
	movq	(%r12), %rax
	.p2align	4
.LBB191_348:
	cmpq	%rax, %rdx
	jge	.LBB191_339
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB191_348
	jmp	.LBB191_339
.LBB191_350:
	movq	368(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_360
	shlq	$3, %rax
	movq	352(%rsp), %rdi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r13, %rcx
	cmovaeq	%r13, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r13, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB191_353
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB191_353:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB191_359
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB191_353
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
	movq	(%r12), %rax
	.p2align	4
.LBB191_356:
	cmpq	%rax, %rdx
	jge	.LBB191_358
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB191_356
.LBB191_358:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB191_359:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB191_360:
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
	retq
.LBB191_303:
	.cfi_def_cfa_offset 800
	xorl	%ebx, %ebx
	jmp	.LBB191_305
.LBB191_361:
.Ltmp4408:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp4409:
	jmp	.LBB191_363
.LBB191_362:
	leaq	680(%rsp), %rax
	leaq	96(%rsp), %rcx
	movq	%rdi, 680(%rsp)
	movq	%rdx, 96(%rsp)
	movq	%rax, 256(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 264(%rsp)
	movq	%rcx, 272(%rsp)
	movq	%rax, 280(%rsp)
.Ltmp4410:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	movq	24(%rsp), %r15
	movq	32(%rsp), %r12
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.2158(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.289(%rip), %rdx
	leaq	256(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp4411:
.LBB191_363:
	ud2
.LBB191_364:
	movb	$1, %bpl
.Ltmp4350:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %r13b
	movq	%r12, %rdi
	movq	%r14, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp4351:
	jmp	.LBB191_75
.LBB191_365:
	movb	$1, %bpl
	leaq	1000(%rax), %rdi
.Ltmp4352:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movb	$1, %r13b
	movq	%r14, %rsi
	movq	%r12, %rdx
	vzeroupper
	callq	*%rax
.Ltmp4353:
	jmp	.LBB191_76
.LBB191_366:
.Ltmp4425:
	movb	$1, %bpl
	movq	%rax, %rbx
	jmp	.LBB191_410
.LBB191_367:
.Ltmp4388:
	jmp	.LBB191_369
.LBB191_368:
.Ltmp4394:
.LBB191_369:
	movq	8(%rsp), %r14
	movq	%rax, %rbx
	movq	%r15, 200(%rsp)
	jmp	.LBB191_396
.LBB191_370:
.Ltmp4365:
	jmp	.LBB191_395
.LBB191_371:
.Ltmp4430:
	leaq	352(%rsp), %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB191_372:
.Ltmp4417:
	movq	24(%rsp), %r15
	movq	%rax, %rbx
	movb	$1, %bpl
	jmp	.LBB191_401
.LBB191_373:
.Ltmp4397:
	movq	24(%rsp), %r15
	movq	%rax, %rbx
	xorl	%ebp, %ebp
	jmp	.LBB191_401
.LBB191_374:
.Ltmp4372:
	jmp	.LBB191_395
.LBB191_375:
.Ltmp4422:
	movq	8(%rsp), %r14
	movq	%rax, %rbx
	jmp	.LBB191_411
.LBB191_376:
.Ltmp4356:
	jmp	.LBB191_395
.LBB191_377:
.Ltmp4362:
	jmp	.LBB191_392
.LBB191_378:
.Ltmp4391:
	jmp	.LBB191_394
.LBB191_379:
.Ltmp4433:
	movq	%rax, %rbx
	movq	%r15, 440(%rsp)
	cmpq	$6, %rbp
	jb	.LBB191_381
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	__rustc::__rust_dealloc
.LBB191_381:
	leaq	432(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	160(%rsp), %rdi
	jmp	.LBB191_384
.LBB191_382:
.Ltmp4375:
	movq	8(%rsp), %r14
	addq	$88, %r15
	movq	%rax, %rbx
	movq	%r15, 200(%rsp)
	jmp	.LBB191_396
.LBB191_383:
.Ltmp4349:
	movq	%rax, %rbx
	movq	%rbp, %rdi
.LBB191_384:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB191_416
.LBB191_385:
.Ltmp4385:
	jmp	.LBB191_395
.LBB191_387:
.Ltmp4404:
	addq	$88, %r15
	movq	%rax, %rbx
	movq	%r15, 200(%rsp)
	jmp	.LBB191_396
.LBB191_388:
.Ltmp4407:
	addq	$40, %r14
	movq	%rax, %rbx
	movq	%r14, 360(%rsp)
	cmpq	$6, %r15
	jb	.LBB191_390
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%rbp, %rdi
	callq	__rustc::__rust_dealloc
.LBB191_390:
	movq	8(%rsp), %r14
	jmp	.LBB191_396
.LBB191_391:
.Ltmp4380:
.LBB191_392:
	movq	8(%rsp), %r14
	addq	$88, %r12
	movq	%rax, %rbx
	movq	%r12, 200(%rsp)
	jmp	.LBB191_396
.LBB191_393:
.Ltmp4359:
.LBB191_394:
	movq	8(%rsp), %r14
.LBB191_395:
	movq	%rax, %rbx
.LBB191_396:
	movq	24(%rsp), %r15
	movq	32(%rsp), %r12
	jmp	.LBB191_398
.LBB191_397:
.Ltmp4412:
	movq	%rax, %rbx
.LBB191_398:
	testq	%r12, %r12
	je	.LBB191_400
	movq	336(%rsp), %rdi
	shlq	$5, %r12
	movl	$8, %edx
	movq	%r12, %rsi
	callq	__rustc::__rust_dealloc
.LBB191_400:
	movb	$1, %bpl
.Ltmp4413:
	leaq	192(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp4414:
.LBB191_401:
	testq	%r15, %r15
	je	.LBB191_403
	movq	80(%rsp), %rdi
	shlq	$3, %r15
	movl	$8, %edx
	leaq	(%r15,%r15,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB191_403:
	movq	520(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_406
	lock		decq	(%rax)
	jne	.LBB191_406
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	520(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB191_406:
	movq	552(%rsp), %rax
	testq	%rax, %rax
	je	.LBB191_409
	lock		decq	(%rax)
	jne	.LBB191_409
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	552(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB191_409:
.Ltmp4418:
	leaq	648(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4419:
.LBB191_410:
	xorl	%r13d, %r13d
.LBB191_411:
	lock		decq	(%r14)
	jne	.LBB191_413
	#MEMBARRIER
.Ltmp4426:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	408(%rsp), %rdi
	callq	*%rax
.Ltmp4427:
.LBB191_413:
	leaq	352(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	testb	%bpl, %bpl
	je	.LBB191_415
	leaq	160(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB191_415:
	testb	%r13b, %r13b
	je	.LBB191_417
.LBB191_416:
.Ltmp4434:
	movq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4435:
.LBB191_417:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB191_418:
.Ltmp4436:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end191:

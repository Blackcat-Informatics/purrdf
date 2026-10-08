<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>:
.Lfunc_begin109:
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
	subq	$392, %rsp
	.cfi_def_cfa_offset 448
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpq	$0, 608(%rdx)
	movq	%r9, %r12
	movq	%r8, %r13
	movq	%rcx, %rbp
	movq	%rdi, %r14
	movq	%rdx, 32(%rsp)
	setne	%r15b
	testb	%sil, %sil
	jne	.LBB109_1
	testb	%r15b, %r15b
	jne	.LBB109_3
.LBB109_5:
	vmovups	(%r12), %xmm0
	movq	16(%r12), %rax
	movq	16(%rbp), %r12
	movb	%r15b, 15(%rsp)
	movq	%rax, 256(%rsp)
	vmovaps	%xmm0, 240(%rsp)
.Ltmp1352:
	leaq	144(%rsp), %rdi
	leaq	240(%rsp), %rsi
	movq	%r12, %rdx
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp1353:
	movq	144(%rsp), %rax
	movq	152(%rsp), %rcx
	movq	160(%rsp), %r15
	movq	168(%rsp), %rbx
	cmpq	$-1, %rax
	je	.LBB109_10
	vmovups	176(%rsp), %zmm0
	movq	%rcx, %r12
	movq	%rax, 24(%rsp)
	vmovups	%zmm0, 320(%rsp)
.Ltmp1357:
	movq	%rbp, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1358:
.LBB109_8:
	vmovups	320(%rsp), %zmm0
	movq	24(%rsp), %rax
	vmovups	%zmm0, 32(%r14)
	movq	%rax, (%r14)
	movq	%r12, 8(%r14)
	movq	%r15, 16(%r14)
	movq	%rbx, 24(%r14)
	jmp	.LBB109_9
.LBB109_10:
	movq	%r14, 72(%rsp)
	movq	8(%rbp), %r14
	movq	%rcx, 48(%rsp)
	movq	(%rbp), %rax
	leaq	(,%r12,8), %rcx
	movq	%r15, 56(%rsp)
	movq	%rbx, 64(%rsp)
	movq	%rbx, %rdx
	movq	%rbp, 128(%rsp)
	leaq	(%rcx,%rcx,4), %rbx
	leaq	(%r14,%rbx), %rcx
	movq	%r14, 80(%rsp)
	movq	%rax, 96(%rsp)
	movq	%rcx, 104(%rsp)
	testq	%r12, %r12
	je	.LBB109_11
	movq	%rcx, 136(%rsp)
	movq	%r13, 16(%rsp)
	addq	$40, %r14
	movq	%r15, %rsi
	movq	%rdx, %rbp
	movq	32(%rsp), %rcx
	leaq	880(%rcx), %rax
	movq	%rax, 40(%rsp)
	leaq	(,%rdx,8), %rax
	leaq	(%rax,%rax,4), %r13
	jmp	.LBB109_18
	.p2align	4
.LBB109_50:
	movq	56(%rsp), %rsi
.LBB109_51:
	movq	24(%rsp), %rax
	movq	%r12, (%rsi,%r13)
	movq	32(%rsp), %rcx
	incq	%rbp
	addq	$40, %r14
	movq	%rax, 8(%rsi,%r13)
	movq	%r15, 16(%rsi,%r13)
	vmovaps	112(%rsp), %xmm0
	vmovups	%xmm0, 24(%rsi,%r13)
	addq	$40, %r13
	addq	$-40, %rbx
	movq	%rbp, 64(%rsp)
	je	.LBB109_52
.LBB109_18:
	movq	-8(%r14), %rax
	leaq	280(%rsp), %rdx
	movq	%rax, 32(%rdx)
	vmovups	-40(%r14), %ymm0
	vmovups	%ymm0, (%rdx)
	movq	%rcx, 272(%rsp)
	cmpq	$0, 280(%rsp)
	je	.LBB109_19
	leaq	-40(%r14), %rax
	leaq	152(%rsp), %rdx
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rdx)
	vmovups	(%rax), %ymm0
	movq	%rdx, %rax
	vmovups	%ymm0, (%rdx)
	jmp	.LBB109_48
	.p2align	4
.LBB109_19:
	movq	656(%rcx), %rdx
	movq	%rsi, %r15
.Ltmp1360:
	movq	40(%rsp), %rsi
	leaq	144(%rsp), %rdi
	leaq	288(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp1361:
	movq	144(%rsp), %r12
	leaq	152(%rsp), %rax
	movq	%r15, %rsi
	cmpq	$-1, %r12
	jne	.LBB109_21
.LBB109_48:
	movq	160(%rsp), %rcx
	movq	%rax, %rdx
	movq	24(%rax), %rax
	movq	152(%rsp), %r12
	movq	168(%rsp), %r15
	movq	%rcx, 24(%rsp)
	movq	32(%rdx), %rcx
	movq	%rax, 112(%rsp)
	movq	%rcx, 120(%rsp)
	cmpq	48(%rsp), %rbp
	jne	.LBB109_51
.Ltmp1365:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1366:
	jmp	.LBB109_50
.LBB109_52:
	movq	136(%rsp), %r14
	movq	16(%rsp), %r13
	jmp	.LBB109_12
.LBB109_11:
	movq	%rdx, %rbp
.LBB109_12:
	movq	%r14, 88(%rsp)
.Ltmp1371:
	leaq	80(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1372:
	vmovups	48(%rsp), %xmm0
	movq	72(%rsp), %rax
	vmovups	%xmm0, 8(%rax)
	movq	%rbp, 24(%rax)
	movq	$0, 32(%rax)
	movq	$-1, (%rax)
.LBB109_9:
	movq	%r13, %rdi
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
	jmp	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.LBB109_21:
	.cfi_def_cfa_offset 448
	movq	%rax, %rcx
	vmovaps	24(%rcx), %xmm0
	vmovups	208(%rsp), %ymm1
	vmovups	192(%rsp), %ymm2
	movq	%r12, 24(%rsp)
	movq	160(%rsp), %rax
	movq	152(%rsp), %r12
	movq	168(%rsp), %rbx
	movq	%r14, 88(%rsp)
	movq	%rax, 32(%rsp)
	vmovups	%ymm1, 352(%rsp)
	vmovups	%ymm2, 336(%rsp)
	vmovaps	%xmm0, 112(%rsp)
	vmovaps	%xmm0, 320(%rsp)
.Ltmp1363:
	movq	16(%rsp), %r13
	leaq	80(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1364:
	movq	%rbx, 40(%rsp)
	movabsq	$9223372036854775807, %rbx
	movq	%r15, %rdi
	testq	%rbp, %rbp
	je	.LBB109_35
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	xorl	%r14d, %r14d
	jmp	.LBB109_24
	.p2align	4
.LBB109_32:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB109_33:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	%r15, %rdi
.LBB109_34:
	incq	%r14
	cmpq	%r14, %rbp
	je	.LBB109_35
.LBB109_24:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rdi,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB109_34
	leaq	(%rdi,%rcx,8), %rdx
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
	jge	.LBB109_27
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB109_27:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB109_33
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB109_27
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
.LBB109_30:
	cmpq	%rax, %rdx
	jge	.LBB109_32
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB109_30
	jmp	.LBB109_32
.LBB109_35:
	movq	48(%rsp), %rax
	movq	16(%rsp), %r13
	movq	72(%rsp), %r14
	movq	32(%rsp), %r15
	testq	%rax, %rax
	je	.LBB109_45
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
	jge	.LBB109_38
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB109_38:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB109_44
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB109_38
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
.LBB109_41:
	cmpq	%rax, %rdx
	jge	.LBB109_43
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB109_41
.LBB109_43:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB109_44:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB109_45:
	movq	40(%rsp), %rbx
	jmp	.LBB109_8
.LBB109_1:
	movb	$1, %r15b
	testb	%r15b, %r15b
	je	.LBB109_5
.LBB109_3:
.Ltmp1374:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.151(%rip), %rdi
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.153(%rip), %rdx
	movl	$83, %esi
	callq	*%rax
.Ltmp1375:
	ud2
.LBB109_46:
.Ltmp1373:
	movq	%r13, 16(%rsp)
	movq	%rax, %rbx
	jmp	.LBB109_16
.LBB109_58:
.Ltmp1359:
	movq	%rax, %rbx
	jmp	.LBB109_59
.LBB109_56:
.Ltmp1354:
	movq	%rax, %rbx
.Ltmp1355:
	movq	%rbp, %rdi
	movq	%rbp, %r14
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1356:
	movq	%r14, %rbp
	jmp	.LBB109_59
.LBB109_14:
.Ltmp1362:
	movq	%rax, %rbx
	movq	%r14, 88(%rsp)
	jmp	.LBB109_15
.LBB109_53:
.Ltmp1367:
	movq	%rax, %rbx
	movq	%r14, 88(%rsp)
	cmpq	$6, %r12
	jb	.LBB109_15
	movq	24(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB109_15:
.Ltmp1368:
	leaq	80(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1369:
.LBB109_16:
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	16(%rsp), %r13
	movq	128(%rsp), %rbp
.LBB109_59:
	movzbl	15(%rsp), %r15d
	jmp	.LBB109_60
.LBB109_55:
.Ltmp1370:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB109_63:
.Ltmp1376:
	movq	%r12, %rdi
	movq	%rax, %rbx
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB109_60:
.Ltmp1377:
	movq	%r13, %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp1378:
	testb	%r15b, %r15b
	je	.LBB109_62
.Ltmp1379:
	movq	%rbp, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1380:
.LBB109_62:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.LBB109_64:
.Ltmp1381:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end109:
<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>:
.Lfunc_begin110:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception74
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
	subq	$968, %rsp
	.cfi_def_cfa_offset 1024
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpb	$2, 194(%rsi)
	movq	%rdi, %rbx
	jne	.LBB110_6
	cmpq	$0, 608(%rdx)
	movq	%rdx, %r12
	je	.LBB110_6
	movq	(%r8), %rax
	vmovdqu64	24(%r8), %zmm0
	vmovdqu64	88(%r8), %zmm2
	vmovdqu64	144(%r8), %zmm1
	movq	16(%r8), %rdx
	movq	%rcx, 112(%rsp)
	movq	8(%r8), %rcx
	movl	$1, %edi
	movq	%rsi, %r14
	movl	$1, %esi
	cmpq	$3, %rax
	cmovaeq	%rax, %rdi
	cmovaeq	%rdx, %rax
	cmovaeq	%rsi, %rdx
	decq	%rax
	vmovdqu64	%zmm2, 536(%rsp)
	vmovdqu64	%zmm0, 472(%rsp)
	vmovdqu64	%zmm1, 592(%rsp)
	movq	%rdi, 448(%rsp)
	movq	%rcx, 456(%rsp)
	movq	%rdx, 464(%rsp)
	movq	$0, 656(%rsp)
	movq	%rax, 664(%rsp)
.Ltmp1382:
	leaq	912(%rsp), %rdi
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1383:
	movq	928(%rsp), %rcx
	movq	920(%rsp), %r15
	imulq	$200, %rcx, %rax
	addq	%r15, %rax
	movq	%rax, (%rsp)
	testq	%rcx, %rcx
	je	.LBB110_34
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB110_7
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB110_26
.LBB110_6:
	vmovups	(%rcx), %xmm0
	movq	16(%rcx), %rax
	movq	%r8, %rdi
	movq	%rax, 24(%rbx)
	vmovups	%xmm0, 8(%rbx)
	movq	$0, 32(%rbx)
	movq	$-1, (%rbx)
	addq	$968, %rsp
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
	jmp	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.LBB110_7:
	.cfi_def_cfa_offset 1024
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%r15), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB110_9
	.p2align	4
.LBB110_8:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB110_25
.LBB110_9:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB110_18
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB110_11
.LBB110_19:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB110_12
.LBB110_20:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB110_21
	.p2align	4
.LBB110_13:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB110_22
.LBB110_14:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB110_15
.LBB110_23:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB110_16
.LBB110_24:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB110_8
	jmp	.LBB110_17
.LBB110_18:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB110_19
	.p2align	4
.LBB110_11:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB110_20
.LBB110_12:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB110_13
.LBB110_21:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB110_14
.LBB110_22:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB110_23
	.p2align	4
.LBB110_15:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB110_24
.LBB110_16:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB110_8
.LBB110_17:
	movq	%r11, %r10
	jmp	.LBB110_8
.LBB110_25:
	testq	%rsi, %rsi
	je	.LBB110_30
.LBB110_26:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%r15), %rdi
	.p2align	4
.LBB110_27:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB110_29
.LBB110_28:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB110_27
	jmp	.LBB110_30
.LBB110_29:
	movq	$-1, %rax
	jmp	.LBB110_28
.LBB110_30:
	testq	%r10, %r10
	je	.LBB110_34
	movq	608(%r12), %rax
	testq	%rax, %rax
	je	.LBB110_34
	cmpq	$0, 336(%rax)
	je	.LBB110_34
	lock		addq	%r10, 352(%rax)
.LBB110_34:
	movq	%r14, 8(%rsp)
	movq	%rbx, 344(%rsp)
	movq	912(%rsp), %rax
	movq	%r15, 176(%rsp)
	movq	$0, 672(%rsp)
	movq	$8, 680(%rsp)
	movq	$0, 688(%rsp)
	movq	(%rsp), %rdx
	movq	%r12, 24(%rsp)
	movq	%rax, 192(%rsp)
	movq	%rdx, 200(%rsp)
	testq	%rcx, %rcx
	je	.LBB110_43
	movl	$8, %eax
	leaq	456(%rsp), %rbx
	addq	$200, %r15
	xorl	%r14d, %r14d
	xorl	%r12d, %r12d
	movq	%rax, 64(%rsp)
	.p2align	4
.LBB110_36:
	movq	-200(%r15), %rax
	cmpq	$-1, %rax
	je	.LBB110_44
	leaq	-200(%r15), %r13
	vmovups	8(%r13), %zmm0
	vmovups	72(%r13), %zmm1
	vmovups	96(%r13), %zmm2
	vmovups	%zmm2, 88(%rbx)
	vmovups	%zmm1, 64(%rbx)
	vmovups	%zmm0, (%rbx)
	movq	%rax, 448(%rsp)
	movzbl	600(%rsp), %ebp
	cmpq	672(%rsp), %r14
	jne	.LBB110_40
.Ltmp1385:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	672(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1386:
	movq	680(%rsp), %rax
	movq	%rax, 64(%rsp)
.LBB110_40:
	vmovdqu64	448(%rsp), %zmm0
	vmovdqu64	512(%rsp), %zmm1
	vmovdqu64	544(%rsp), %zmm2
	movq	64(%rsp), %rax
	vmovdqu64	%zmm2, 96(%rax,%r12)
	vmovdqu64	%zmm1, 64(%rax,%r12)
	vmovdqu64	%zmm0, (%rax,%r12)
	leaq	1(%r14), %rax
	movq	%rax, 688(%rsp)
	testb	%bpl, %bpl
	jne	.LBB110_46
	addq	$160, %r12
	addq	$200, %r15
	addq	$200, %r13
	movq	%rax, %r14
	cmpq	(%rsp), %r13
	jne	.LBB110_36
	movq	(%rsp), %r15
	movq	24(%rsp), %r12
	movq	%rax, %r14
	jmp	.LBB110_45
.LBB110_43:
	movl	$8, %eax
	xorl	%r14d, %r14d
	movq	%rax, 64(%rsp)
	jmp	.LBB110_45
.LBB110_44:
	movq	24(%rsp), %r12
.LBB110_45:
	movq	%r15, 184(%rsp)
	xorl	%ebx, %ebx
	jmp	.LBB110_47
.LBB110_46:
	movq	24(%rsp), %r12
	incq	%r14
	movb	$1, %bl
	movq	%r15, 184(%rsp)
.LBB110_47:
.Ltmp1393:
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1394:
	movq	672(%rsp), %rsi
	testq	%r14, %r14
	je	.LBB110_51
	cmpq	$8, %r14
	jae	.LBB110_52
	xorl	%eax, %eax
	xorl	%r15d, %r15d
	jmp	.LBB110_61
.LBB110_51:
	xorl	%r15d, %r15d
	jmp	.LBB110_63
.LBB110_52:
	cmpq	$32, %r14
	jae	.LBB110_54
	xorl	%eax, %eax
	xorl	%r15d, %r15d
	jmp	.LBB110_58
.LBB110_54:
	vmovdqa64	.LCPI110_0(%rip), %zmm1
	vpbroadcastq	.LCPI110_1(%rip), %zmm2
	vpbroadcastq	.LCPI110_2(%rip), %zmm3
	movq	64(%rsp), %rdx
	movq	%r14, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB110_55:
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
	jne	.LBB110_55
	vpaddq	%zmm0, %zmm4, %zmm0
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r15
	cmpq	%rax, %r14
	je	.LBB110_63
	testb	$24, %r14b
	je	.LBB110_61
.LBB110_58:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI110_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI110_1(%rip), %zmm2
	vpbroadcastq	.LCPI110_3(%rip), %zmm3
	movq	64(%rsp), %rdx
	movq	%r14, %rax
	andq	$-8, %rax
	vmovq	%r15, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB110_59:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%rdx,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB110_59
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r15
	cmpq	%rax, %r14
	je	.LBB110_63
.LBB110_61:
	movq	64(%rsp), %rdx
	movq	%r14, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%rdx), %rax
	.p2align	4
.LBB110_62:
	addq	(%rax), %r15
	addq	$160, %rax
	decq	%rcx
	jne	.LBB110_62
.LBB110_63:
	movq	8(%rsp), %rax
	movq	$0, 176(%rsp)
	movq	$8, 184(%rsp)
	movq	%rsi, 936(%rsp)
	movq	$0, 192(%rsp)
	movzbl	193(%rax), %ebp
	movzbl	195(%rax), %eax
	movq	%rax, 776(%rsp)
	movq	%rsi, (%rsp)
	movq	112(%rsp), %rcx
	movq	64(%rsp), %rax
	movq	16(%rcx), %r13
	movq	%rax, 944(%rsp)
	movq	%r14, 952(%rsp)
	movb	%bl, 960(%rsp)
.Ltmp1399:
	leaq	448(%rsp), %rdi
	leaq	176(%rsp), %rsi
	movq	%r13, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp1400:
	vmovups	456(%rsp), %xmm0
	movq	448(%rsp), %rcx
	movq	472(%rsp), %rax
	movq	%rcx, 8(%rsp)
	movl	%ebx, 72(%rsp)
	movq	%rax, 320(%rsp)
	movq	%r15, 792(%rsp)
	vmovaps	%xmm0, 304(%rsp)
	cmpq	$-1, %rcx
	je	.LBB110_89
	movzbl	480(%rsp), %eax
	vmovdqu	481(%rsp), %ymm0
	vmovdqa	304(%rsp), %xmm1
	vmovdqu	512(%rsp), %ymm2
	movl	%eax, 32(%rsp)
	movq	320(%rsp), %rax
	vmovdqu	%ymm0, 672(%rsp)
	vmovdqu	%ymm2, 703(%rsp)
	vmovdqa	%xmm1, 400(%rsp)
	movq	%rax, 416(%rsp)
	movq	112(%rsp), %rax
	movq	8(%rax), %r12
	testq	%r13, %r13
	je	.LBB110_78
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %rbx
	xorl	%r15d, %r15d
	jmp	.LBB110_70
	.p2align	4
.LBB110_67:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_68:
	vzeroupper
	callq	*%rbx
.LBB110_69:
	incq	%r15
	cmpq	%r13, %r15
	je	.LBB110_78
.LBB110_70:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r12,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB110_69
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
	jge	.LBB110_73
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_73:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_68
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_73
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
.LBB110_76:
	cmpq	%rax, %rdx
	jge	.LBB110_67
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB110_76
	jmp	.LBB110_67
.LBB110_78:
	movq	112(%rsp), %rax
	movq	(%rax), %rax
	testq	%rax, %rax
	je	.LBB110_94
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	movq	(%rsp), %r13
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
	jge	.LBB110_81
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB110_81:
	movl	72(%rsp), %ebp
	.p2align	4
.LBB110_82:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_88
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_82
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
.LBB110_85:
	cmpq	%rax, %rdx
	jge	.LBB110_87
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB110_85
.LBB110_87:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_88:
	movq	free@GOTPCREL(%rip), %rax
	movq	%r12, %rdi
	vzeroupper
	callq	*%rax
	movq	24(%rsp), %r12
	testq	%r14, %r14
	jne	.LBB110_125
	jmp	.LBB110_128
.LBB110_89:
	movq	320(%rsp), %rax
	movq	112(%rsp), %rcx
	vmovdqa	304(%rsp), %xmm0
	movq	%rax, 288(%rsp)
	movq	8(%rcx), %rdx
	movq	(%rcx), %rax
	leaq	(%r13,%r13,4), %rcx
	vmovdqa	%xmm0, 272(%rsp)
	movq	%rdx, 368(%rsp)
	movq	%rax, 384(%rsp)
	movq	608(%r12), %rax
	leaq	(%rdx,%rcx,8), %rcx
	movq	%rdx, 376(%rsp)
	movq	%rcx, 392(%rsp)
	testq	%rax, %rax
	je	.LBB110_95
	movb	%bpl, 79(%rsp)
	movq	%rdx, 352(%rsp)
	lock		incq	(%rax)
	jle	.LBB110_475
	movq	608(%r12), %r13
	testq	%r14, %r14
	sete	%al
	movq	%r13, 432(%rsp)
	movq	16(%r13), %rcx
	movq	%rcx, 256(%rsp)
	movq	40(%r13), %rcx
	cmpq	$-1, %rcx
	movq	%rcx, 120(%rsp)
	sete	%cl
	orb	%al, %cl
	jne	.LBB110_155
	cmpq	$8, %r14
	jae	.LBB110_142
	movq	24(%rsp), %rdx
	movq	64(%rsp), %rsi
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB110_151
.LBB110_94:
	movl	72(%rsp), %ebp
	movq	(%rsp), %r13
	movq	24(%rsp), %r12
	testq	%r14, %r14
	jne	.LBB110_125
	jmp	.LBB110_128
.LBB110_95:
	movq	376(%rsp), %rcx
	movq	368(%rsp), %rax
	movq	384(%rsp), %rdx
	movq	%rcx, 456(%rsp)
	movq	392(%rsp), %rcx
	movq	%rax, 448(%rsp)
	movq	%rdx, 464(%rsp)
	movq	%rcx, 472(%rsp)
	movq	456(%rsp), %r13
	movq	472(%rsp), %rcx
	cmpq	%rcx, %r13
	je	.LBB110_101
	addq	$40, %r13
	movq	%r14, 56(%rsp)
	movq	%r13, %rax
	jmp	.LBB110_98
	.p2align	4
.LBB110_97:
	movq	280(%rsp), %rcx
	leaq	(%r15,%r15,4), %rax
	incq	%r15
	addq	$40, %rbx
	movq	%rbp, (%rcx,%rax,8)
	movq	%r12, 8(%rcx,%rax,8)
	vmovdqa	848(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rcx,%rax,8)
	movq	864(%rsp), %rdx
	movq	%rdx, 32(%rcx,%rax,8)
	movq	%r15, 288(%rsp)
	leaq	40(%r13), %rax
	movq	%r14, %rcx
	cmpq	%r14, %rbx
	movq	56(%rsp), %r14
	je	.LBB110_101
.LBB110_98:
	movq	-40(%rax), %rbp
	movq	%rax, %r13
	testq	%rbp, %rbp
	je	.LBB110_101
	leaq	-40(%r13), %rbx
	movq	-32(%r13), %r12
	movq	288(%rsp), %r15
	movq	%rcx, %r14
	movq	32(%rbx), %rax
	movq	%rax, 864(%rsp)
	vmovdqu	16(%rbx), %xmm0
	vmovdqa	%xmm0, 848(%rsp)
	cmpq	272(%rsp), %r15
	jne	.LBB110_97
.Ltmp1483:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp1484:
	jmp	.LBB110_97
.LBB110_101:
	subq	%r13, %rcx
	je	.LBB110_114
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r12
	shrq	$3, %rcx
	movabsq	$-3689348814741910323, %rbx
	xorl	%r15d, %r15d
	imulq	%rcx, %rbx
	jmp	.LBB110_106
	.p2align	4
.LBB110_103:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_104:
	callq	*%r12
.LBB110_105:
	incq	%r15
	cmpq	%rbx, %r15
	je	.LBB110_114
.LBB110_106:
	leaq	(%r15,%r15,4), %rcx
	movq	(%r13,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB110_105
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
	jge	.LBB110_109
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_109:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_104
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_109
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
.LBB110_112:
	cmpq	%rax, %rdx
	jge	.LBB110_103
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB110_112
	jmp	.LBB110_103
.LBB110_114:
	movq	464(%rsp), %rax
	movq	(%rsp), %r13
	testq	%rax, %rax
	je	.LBB110_124
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	movq	448(%rsp), %rdi
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
	jge	.LBB110_117
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_117:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_123
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_117
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
.LBB110_120:
	cmpq	%rax, %rdx
	jge	.LBB110_122
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB110_120
.LBB110_122:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_123:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_124:
	movq	288(%rsp), %rax
	vmovdqa	272(%rsp), %xmm0
	movq	%rax, 416(%rsp)
	movb	$2, %al
	movl	%eax, 32(%rsp)
	movq	$-1, 8(%rsp)
	movl	72(%rsp), %ebp
	vmovdqa	%xmm0, 400(%rsp)
	movq	24(%rsp), %r12
	testq	%r14, %r14
	je	.LBB110_128
.LBB110_125:
	movq	64(%rsp), %rbx
	movl	$1, %r15d
	subq	%r14, %r15
	.p2align	4
.LBB110_126:
.Ltmp1489:
	movq	%rbx, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp1490:
	incq	%r15
	addq	$160, %rbx
	cmpq	$1, %r15
	jne	.LBB110_126
.LBB110_128:
	movq	344(%rsp), %r14
	testq	%r13, %r13
	je	.LBB110_139
	shlq	$5, %r13
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
	jge	.LBB110_131
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB110_131:
	movq	64(%rsp), %rdi
	.p2align	4
.LBB110_132:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_138
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_132
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
.LBB110_135:
	cmpq	%rax, %rdx
	jge	.LBB110_137
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB110_135
.LBB110_137:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_138:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB110_139:
	movq	8(%rsp), %rcx
	cmpq	$-1, %rcx
	je	.LBB110_141
	vmovaps	400(%rsp), %xmm0
	movq	416(%rsp), %rax
	vmovups	703(%rsp), %ymm1
	vmovdqu	672(%rsp), %ymm2
	movq	%rax, 752(%rsp)
	movq	752(%rsp), %rax
	vmovaps	%xmm0, 736(%rsp)
	vmovups	%ymm1, 64(%r14)
	vmovdqu	%ymm2, 33(%r14)
	vmovdqa	736(%rsp), %xmm1
	movq	%rax, 24(%r14)
	movl	32(%rsp), %eax
	vmovdqu	%xmm1, 8(%r14)
	movq	%rcx, (%r14)
	movb	%al, 32(%r14)
	jmp	.LBB110_450
.LBB110_141:
	movl	32(%rsp), %r15d
	jmp	.LBB110_439
.LBB110_142:
	movq	24(%rsp), %rdx
	cmpq	$32, %r14
	jae	.LBB110_144
	movq	64(%rsp), %rsi
	xorl	%eax, %eax
	xorl	%ebx, %ebx
	jmp	.LBB110_148
.LBB110_144:
	vmovdqa64	.LCPI110_0(%rip), %zmm1
	vpbroadcastq	.LCPI110_1(%rip), %zmm2
	vpbroadcastq	.LCPI110_2(%rip), %zmm3
	movq	64(%rsp), %rsi
	movq	%r14, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB110_145:
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
	jne	.LBB110_145
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
	cmpq	%rax, %r14
	je	.LBB110_153
	testb	$24, %r14b
	je	.LBB110_151
.LBB110_148:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI110_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI110_1(%rip), %zmm2
	vpbroadcastq	.LCPI110_3(%rip), %zmm3
	movq	%r14, %rax
	andq	$-8, %rax
	vmovq	%rbx, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB110_149:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rsi,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB110_149
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbx
	cmpq	%rax, %r14
	je	.LBB110_153
.LBB110_151:
	movq	%r14, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rsi), %rax
	.p2align	4
.LBB110_152:
	addq	(%rax), %rbx
	addq	$160, %rax
	decq	%rcx
	jne	.LBB110_152
.LBB110_153:
	movq	904(%rdx), %rax
	movq	920(%rdx), %rsi
	leaq	904(%rdx), %r12
	subq	%rsi, %rax
	cmpq	%rax, %rbx
	ja	.LBB110_476
.LBB110_154:
	movq	24(%rsp), %rax
	cmpq	1008(%rax), %rbx
	ja	.LBB110_477
.LBB110_155:
	leaq	16(%r13), %rax
	movq	%rax, 104(%rsp)
	leaq	(%r14,%r14,4), %rax
	movq	64(%rsp), %r15
	movq	(%rsp), %rcx
	shlq	$5, %rax
	addq	%r15, %rax
	movq	%r15, 808(%rsp)
	movq	%r15, 816(%rsp)
	movq	%rcx, 824(%rsp)
	movq	%rax, 760(%rsp)
	movq	%rax, 832(%rsp)
	testq	%r14, %r14
	je	.LBB110_359
	leaq	272(%r13), %rax
	leaq	456(%rsp), %rbx
	movq	%rax, 336(%rsp)
	movq	24(%rsp), %r12
	leaq	880(%r12), %rax
	movq	%rax, 264(%rsp)
	movq	%r13, 8(%rsp)
.LBB110_157:
	leaq	160(%r15), %rdx
	movq	%rdx, 816(%rsp)
	movq	(%r15), %rax
	cmpq	$-1, %rax
	je	.LBB110_359
	movq	%rax, 448(%rsp)
	movq	%rdx, 64(%rsp)
	vmovdqu64	8(%r15), %zmm0
	vmovdqu64	72(%r15), %zmm1
	vmovdqu64	96(%r15), %zmm2
	vmovdqu64	%zmm2, 88(%rbx)
	vmovdqu64	%zmm1, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
	movq	456(%rsp), %r15
	imulq	$88, 464(%rsp), %rdi
	movq	488(%rsp), %rdx
	movq	472(%rsp), %rbx
	movq	480(%rsp), %rcx
	movq	496(%rsp), %rsi
	movq	512(%rsp), %rbp
	movq	%r15, 304(%rsp)
	movq	%rax, 320(%rsp)
	movq	504(%rsp), %rax
	movq	%rdx, 784(%rsp)
	movq	%r15, 312(%rsp)
	addq	%r15, %rdi
	movq	%rdi, 80(%rsp)
	movq	%rsi, 48(%rsp)
	movq	%rcx, 168(%rsp)
	movq	%rdi, 328(%rsp)
	movq	%rax, 40(%rsp)
	testq	%rbp, %rbp
	je	.LBB110_331
	movq	544(%rsp), %rdx
	shlq	$5, %rbp
	addq	$8, %rcx
	addq	%rax, %rbp
	movq	%rdx, (%rsp)
	movq	%rcx, 768(%rsp)
	movq	$0, 360(%rsp)
	movq	%r15, 96(%rsp)
	movq	%rbx, 16(%rsp)
	movq	%rbp, 56(%rsp)
	jmp	.LBB110_162
.LBB110_160:
	movq	352(%rsp), %rax
.LBB110_161:
	movq	800(%rsp), %rcx
	movq	%rax, 352(%rsp)
	movq	%rax, 376(%rsp)
	addq	$32, %rcx
	movq	%rcx, %rax
	cmpq	%rbp, %rcx
	je	.LBB110_331
.LBB110_162:
	movq	%r15, 88(%rsp)
	movq	16(%rax), %rdx
	movq	(%rax), %r14
	movq	8(%rax), %rbx
	movq	360(%rsp), %rcx
	movq	%rcx, 32(%rsp)
	movq	%rdx, 160(%rsp)
	movq	%rax, 800(%rsp)
	movq	24(%rax), %rax
	movq	%rax, 112(%rsp)
	testq	%r14, %r14
	je	.LBB110_172
	cmpq	$-1, 256(%rsp)
	je	.LBB110_172
	movq	80(%r13), %rax
	movq	$-1, %rdx
	.p2align	4
.LBB110_165:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%r13)
	jne	.LBB110_165
	movq	104(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB110_169
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp1406:
	movq	104(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1407:
	cmpb	$-1, 128(%rsp)
	jne	.LBB110_467
.LBB110_169:
	movq	624(%r12), %rax
	testq	%rax, %rax
	je	.LBB110_172
	movl	1220(%r12), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB110_172
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	776(%rsp), %rax
	lock		addq	%r14, (%rcx,%rax,8)
	.p2align	4
.LBB110_172:
	movq	784(%rsp), %rdx
	movq	32(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB110_474
	movq	40(%rsp), %r12
	movq	48(%rsp), %rbp
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	cmovbq	%rbx, %rsi
	cmpq	%rdi, %rbx
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB110_473
	leaq	(,%rdi,8), %rax
	movq	%rsi, %r9
	leaq	(%rax,%rax,2), %rbx
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r13
	cmpq	%rsi, %rdi
	jne	.LBB110_182
	movq	88(%rsp), %r10
	xorl	%r12d, %r12d
	xorl	%r15d, %r15d
	xorl	%r14d, %r14d
.LBB110_176:
	movq	%r15, 440(%rsp)
	movq	%r14, 424(%rsp)
	movq	%r9, 360(%rsp)
	cmpq	$-1, 120(%rsp)
	je	.LBB110_188
	movq	160(%rsp), %rax
	movq	80(%rsp), %r15
	movl	$0, %ecx
	movl	$0, %ebp
	subq	(%rsp), %rax
	cmovbq	%rcx, %rax
	subq	%r10, %r15
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %r15
	imulq	%rcx, %r15
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	testq	%r15, %r15
	je	.LBB110_181
	movq	88(%rsp), %rax
	xorl	%ebp, %ebp
	leaq	8(%rax), %r14
	.p2align	4
.LBB110_179:
.Ltmp1409:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp1410:
	addq	%rax, %rbp
	movq	$-1, %rcx
	cmovbq	%rcx, %rbp
	addq	$88, %r14
	decq	%r15
	jne	.LBB110_179
.LBB110_181:
	movq	8(%rsp), %rax
	movl	296(%rax), %eax
	testl	%eax, %eax
	jne	.LBB110_190
	jmp	.LBB110_189
	.p2align	4
.LBB110_182:
	movq	%r13, %rdx
	subq	%rbx, %rdx
	movabsq	$-6148914691236517205, %rax
	movq	$-1, %rdi
	leaq	.LJTI110_0(%rip), %r8
	xorl	%r14d, %r14d
	xorl	%r15d, %r15d
	xorl	%r12d, %r12d
	mulxq	%rax, %rax, %rax
	movq	768(%rsp), %rcx
	movq	88(%rsp), %r10
	shrq	$4, %rax
	addq	%rbx, %rcx
	jmp	.LBB110_185
	.p2align	4
.LBB110_183:
	addq	%rdx, %r15
	cmovbq	%rdi, %r15
.LBB110_184:
	addq	$24, %rcx
	decq	%rax
	je	.LBB110_176
.LBB110_185:
	movzbl	-8(%rcx), %esi
	movq	(%rcx), %rdx
	movslq	(%r8,%rsi,4), %rsi
	addq	%r8, %rsi
	jmpq	*%rsi
.LBB110_186:
	addq	%rdx, %r12
	cmovbq	%rdi, %r12
	jmp	.LBB110_184
	.p2align	4
.LBB110_187:
	cmpq	%rdx, %r14
	cmovbeq	%rdx, %r14
	jmp	.LBB110_184
	.p2align	4
.LBB110_188:
	xorl	%ebp, %ebp
	movq	8(%rsp), %rax
	movl	296(%rax), %eax
	testl	%eax, %eax
	jne	.LBB110_190
.LBB110_189:
	movq	336(%rsp), %rax
	cmpb	$-1, (%rax)
	jne	.LBB110_194
	.p2align	4
.LBB110_190:
	cmpq	$-1, 256(%rsp)
	je	.LBB110_192
	movq	8(%rsp), %rcx
	movq	$-1, %rdx
	movq	80(%rcx), %rax
	addq	%r12, %rax
	cmovbq	%rdx, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB110_194
.LBB110_192:
	cmpq	$-1, 120(%rsp)
	je	.LBB110_204
	movq	24(%rsp), %rcx
	movq	$-1, %rsi
	movq	1040(%rcx), %rax
	movq	1048(%rcx), %rcx
	movq	8(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rsi, %rcx
	addq	440(%rsp), %rcx
	cmovbq	%rsi, %rcx
	addq	424(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%rsi, %rcx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	cmpq	40(%rdx), %rax
	jbe	.LBB110_204
.LBB110_194:
	movq	88(%rsp), %r15
	movq	32(%rsp), %rax
	cmpq	360(%rsp), %rax
	jne	.LBB110_224
	movq	24(%rsp), %r12
	movq	56(%rsp), %rbp
.LBB110_196:
	cmpq	$-1, 120(%rsp)
	je	.LBB110_276
	movq	160(%rsp), %rax
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbx
	cmpq	%rax, (%rsp)
	jae	.LBB110_318
	cmpq	80(%rsp), %r15
	je	.LBB110_278
	movq	160(%rsp), %rax
	leaq	-1(%rax), %rbx
	.p2align	4
.LBB110_200:
	movq	8(%r15), %rax
	movq	%r15, %rdx
	cmpq	$-1, %rax
	je	.LBB110_287
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1454:
	movq	264(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1455:
	movq	(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB110_286
	incq	%rax
	movq	%rbp, %rdx
	leaq	88(%rbp), %r15
	movq	%rax, (%rsp)
	movq	56(%rsp), %rbp
	cmpq	80(%rsp), %r15
	jne	.LBB110_200
	jmp	.LBB110_287
.LBB110_204:
	cmpq	$-1, 256(%rsp)
	movq	168(%rsp), %r9
	movq	360(%rsp), %r10
	je	.LBB110_206
	movq	%rbx, %rax
	cmpq	%r10, 32(%rsp)
	jne	.LBB110_219
.LBB110_206:
	movq	56(%rsp), %rbp
	movb	$1, %r14b
	cmpq	%r10, 32(%rsp)
	je	.LBB110_209
	.p2align	4
.LBB110_207:
	movq	168(%rsp), %rax
	cmpb	$2, -24(%rax,%r13)
	je	.LBB110_279
	addq	$-24, %r13
	cmpq	%r13, %rbx
	jne	.LBB110_207
.LBB110_209:
	movq	8(%rsp), %r13
.LBB110_210:
	movq	16(%rsp), %rbx
	cmpq	$-1, 256(%rsp)
	je	.LBB110_269
	testq	%r12, %r12
	je	.LBB110_269
	movq	80(%r13), %rax
	movq	$-1, %rdx
	.p2align	4
.LBB110_213:
	movq	%rax, %rcx
	addq	%r12, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%r13)
	jne	.LBB110_213
	movq	104(%rsp), %rsi
	addq	%r12, %rax
	cmovbq	%rdx, %rax
	movq	(%rsi), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB110_269
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp1415:
	leaq	128(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1416:
	cmpb	$-1, 128(%rsp)
	je	.LBB110_269
	movb	$1, %al
	movl	%eax, 32(%rsp)
	cmpq	$-1, 120(%rsp)
	movq	24(%rsp), %r12
	jne	.LBB110_305
	jmp	.LBB110_385
	.p2align	4
.LBB110_218:
	addq	$24, %rax
	cmpq	%rax, %r13
	je	.LBB110_206
.LBB110_219:
	cmpb	$0, (%r9,%rax)
	jne	.LBB110_218
	movzbl	1(%r9,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB110_218
	movq	24(%rsp), %rdx
	movq	624(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB110_218
	movq	24(%rsp), %rsi
	movl	1220(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB110_218
	movq	%rsi, %r8
	shlq	$7, %r8
	movq	8(%r9,%rax), %rdi
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB110_218
.LBB110_224:
	movq	168(%rsp), %rax
	movq	24(%rsp), %r12
	movq	56(%rsp), %rbp
	addq	%rax, %rbx
	addq	%rax, %r13
	jmp	.LBB110_227
.LBB110_225:
	movq	24(%rsp), %r12
	movq	56(%rsp), %rbp
	.p2align	4
.LBB110_226:
	addq	$24, %rbx
	cmpq	%r13, %rbx
	je	.LBB110_196
.LBB110_227:
	movzbl	(%rbx), %eax
	leaq	.LJTI110_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB110_228:
	cmpq	$-1, 256(%rsp)
	je	.LBB110_226
	movq	8(%rsp), %rdx
	movzbl	1(%rbx), %r12d
	movq	8(%rbx), %rbp
	movq	16(%rbx), %r14
	movq	$-1, %rsi
	movq	80(%rdx), %rax
	.p2align	4
.LBB110_230:
	movq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB110_230
	movq	104(%rsp), %rcx
	addq	%rbp, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB110_234
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp1441:
	movq	104(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1442:
	cmpb	$-1, 128(%rsp)
	jne	.LBB110_452
.LBB110_234:
	cmpl	$255, %r12d
	je	.LBB110_225
	movq	24(%rsp), %rcx
	movq	624(%rcx), %rax
	testq	%rax, %rax
	je	.LBB110_263
	movl	1220(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB110_225
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%rbp, (%rcx,%r12,8)
	jmp	.LBB110_225
	.p2align	4
.LBB110_238:
	movq	8(%rbx), %r14
	movq	%r15, %rsi
	cmpq	%r14, (%rsp)
	jae	.LBB110_257
	cmpq	80(%rsp), %rsi
	je	.LBB110_259
	leaq	-1(%r14), %r15
	movq	%rsi, %rcx
	.p2align	4
.LBB110_241:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB110_265
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1430:
	movq	264(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1431:
	movq	(%rsp), %rax
	cmpq	%rax, %r15
	je	.LBB110_264
	incq	%rax
	movq	%rbp, %rdx
	leaq	88(%rbp), %rcx
	movq	%rax, (%rsp)
	movq	56(%rsp), %rbp
	cmpq	80(%rsp), %rcx
	jne	.LBB110_241
	jmp	.LBB110_265
	.p2align	4
.LBB110_245:
	cmpq	$-1, 120(%rsp)
	je	.LBB110_226
	movq	8(%rsp), %rdx
	movq	8(%rbx), %rcx
	movl	296(%rdx), %eax
	testl	%eax, %eax
	je	.LBB110_258
	movq	104(%rdx), %rax
	movq	$-1, %rsi
	addq	%rcx, %rax
	movq	40(%rdx), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB110_226
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp1428:
	movq	104(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1429:
	cmpb	$-1, 128(%rsp)
	je	.LBB110_226
	jmp	.LBB110_382
	.p2align	4
.LBB110_250:
	cmpq	$-1, 120(%rsp)
	je	.LBB110_226
	movq	8(%rsp), %rsi
	cmpq	$-1, 40(%rsi)
	je	.LBB110_226
	movq	8(%rbx), %rcx
	movq	16(%rbx), %r14
	movl	296(%rsi), %eax
	testl	%eax, %eax
	je	.LBB110_260
	movq	104(%rsi), %rax
	movq	$-1, %rdi
	.p2align	4
.LBB110_254:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rdi, %rdx
	lock		cmpxchgq	%rdx, 104(%rsi)
	jne	.LBB110_254
	addq	%rcx, %rax
	movq	40(%rsi), %rcx
	cmovbq	%rdi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB110_226
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp1435:
	movq	104(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1436:
	jmp	.LBB110_261
.LBB110_257:
	movq	%rsi, %r15
	cmpq	$-1, 120(%rsp)
	jne	.LBB110_267
	jmp	.LBB110_226
.LBB110_258:
	movq	336(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
	cmpb	$-1, 128(%rsp)
	je	.LBB110_226
	jmp	.LBB110_382
.LBB110_259:
	movq	96(%rsp), %rdx
	movq	%rsi, %r15
	movq	%rdx, 96(%rsp)
	movq	%rdx, 312(%rsp)
	cmpq	$-1, 120(%rsp)
	jne	.LBB110_267
	jmp	.LBB110_226
.LBB110_260:
	movq	336(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
.LBB110_261:
	movzbl	128(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%r14, %r14
	setne	%dl
	testb	%cl, %dl
	jne	.LBB110_460
	cmpb	$-1, %al
	je	.LBB110_226
	jmp	.LBB110_382
.LBB110_263:
	movq	56(%rsp), %rbp
	movq	%rcx, %r12
	jmp	.LBB110_226
.LBB110_264:
	movq	%r14, (%rsp)
	movq	%rbp, %rdx
	movq	56(%rsp), %rbp
.LBB110_265:
	addq	$88, %rdx
	movq	%rdx, %r15
	movq	%rdx, 96(%rsp)
	movq	%rdx, 312(%rsp)
	cmpq	$-1, 120(%rsp)
	je	.LBB110_226
.LBB110_267:
.Ltmp1433:
	leaq	176(%rsp), %rdi
	movq	%r12, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp1434:
	cmpb	$-1, 176(%rsp)
	je	.LBB110_226
	jmp	.LBB110_382
.LBB110_269:
	cmpq	$-1, 120(%rsp)
	je	.LBB110_277
	cmpq	$-1, 40(%r13)
	movq	24(%rsp), %r12
	je	.LBB110_293
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB110_290
	movq	104(%r13), %rax
	movq	440(%rsp), %rsi
	movq	$-1, %rdx
	.p2align	4
.LBB110_273:
	movq	%rax, %rcx
	addq	%rsi, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 104(%r13)
	jne	.LBB110_273
	movq	40(%r13), %rcx
	addq	%rsi, %rax
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB110_293
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp1418:
	movq	104(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1419:
	jmp	.LBB110_291
.LBB110_276:
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbx
	jmp	.LBB110_318
.LBB110_277:
	movq	24(%rsp), %r12
	movq	88(%rsp), %r15
	jmp	.LBB110_318
.LBB110_278:
	movq	96(%rsp), %rdx
	jmp	.LBB110_288
.LBB110_279:
	movq	168(%rsp), %rax
	movq	-16(%rax,%r13), %rbx
	cmpq	%rbx, (%rsp)
	jae	.LBB110_289
	movq	88(%rsp), %rcx
	cmpq	80(%rsp), %rcx
	je	.LBB110_327
	movq	8(%rsp), %r13
	leaq	-1(%rbx), %r14
.LBB110_282:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB110_329
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1412:
	movq	264(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1413:
	movq	(%rsp), %rax
	cmpq	%rax, %r14
	je	.LBB110_328
	incq	%rax
	movq	%rbp, %rdx
	leaq	88(%rbp), %rcx
	movq	%rax, (%rsp)
	movq	56(%rsp), %rbp
	cmpq	80(%rsp), %rcx
	jne	.LBB110_282
	jmp	.LBB110_329
.LBB110_286:
	movq	160(%rsp), %rax
	movq	%rbp, %rdx
	movq	%rax, (%rsp)
	movq	56(%rsp), %rbp
.LBB110_287:
	movq	16(%rsp), %rbx
	addq	$88, %rdx
	movq	%rdx, %r15
.LBB110_288:
	movq	%rdx, 96(%rsp)
	movq	%rdx, 312(%rsp)
	jmp	.LBB110_318
.LBB110_289:
	movq	8(%rsp), %r13
	xorl	%r14d, %r14d
	jmp	.LBB110_210
.LBB110_290:
	movq	336(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
.LBB110_291:
	cmpb	$-1, 128(%rsp)
	je	.LBB110_293
	movb	$1, %al
	jmp	.LBB110_304
.LBB110_293:
	testb	%r14b, %r14b
	jne	.LBB110_296
.Ltmp1420:
	leaq	176(%rsp), %rdi
	movq	%r12, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp1421:
	cmpb	$-1, 176(%rsp)
	movb	$1, %al
	movl	%eax, 32(%rsp)
	jne	.LBB110_305
.LBB110_296:
	cmpq	$0, 424(%rsp)
	je	.LBB110_300
	movl	296(%r13), %eax
	testl	%eax, %eax
	je	.LBB110_301
	movq	104(%r13), %rax
	movq	$-1, %rcx
	addq	424(%rsp), %rax
	cmovbq	%rcx, %rax
	movq	40(%r13), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB110_302
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp1422:
	movq	104(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1423:
	jmp	.LBB110_303
.LBB110_300:
	movl	$0, 32(%rsp)
	jmp	.LBB110_305
.LBB110_301:
	movq	336(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
	jmp	.LBB110_303
.LBB110_302:
	movb	$-1, 128(%rsp)
.LBB110_303:
	cmpb	$-1, 128(%rsp)
	setne	%al
.LBB110_304:
	movl	%eax, 32(%rsp)
.LBB110_305:
	movq	160(%rsp), %rax
	cmpq	%rax, (%rsp)
	jae	.LBB110_312
	movq	88(%rsp), %r15
	cmpq	80(%rsp), %r15
	je	.LBB110_313
	movq	160(%rsp), %rax
	leaq	-1(%rax), %rbx
	.p2align	4
.LBB110_308:
	movq	8(%r15), %rax
	movq	%r15, %rdx
	cmpq	$-1, %rax
	je	.LBB110_315
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbp
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1425:
	movq	264(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1426:
	movq	(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB110_314
	incq	%rax
	movq	%rbp, %rdx
	leaq	88(%rbp), %r15
	movq	%rax, (%rsp)
	movq	56(%rsp), %rbp
	cmpq	80(%rsp), %r15
	jne	.LBB110_308
	jmp	.LBB110_315
.LBB110_312:
	movq	88(%rsp), %r15
	jmp	.LBB110_317
.LBB110_313:
	movq	96(%rsp), %rdx
	jmp	.LBB110_316
.LBB110_314:
	movq	160(%rsp), %rax
	movq	%rbp, %rdx
	movq	%rax, (%rsp)
	movq	56(%rsp), %rbp
.LBB110_315:
	movq	16(%rsp), %rbx
	addq	$88, %rdx
	movq	%rdx, %r15
.LBB110_316:
	movq	%rdx, 96(%rsp)
	movq	%rdx, 312(%rsp)
.LBB110_317:
	cmpb	$0, 32(%rsp)
	jne	.LBB110_468
.LBB110_318:
	cmpq	$0, 112(%rsp)
	je	.LBB110_160
	movq	%r15, 88(%rsp)
	movq	392(%rsp), %r14
	movq	352(%rsp), %rsi
	jmp	.LBB110_321
	.p2align	4
.LBB110_320:
	movq	280(%rsp), %rax
	leaq	(%r13,%r13,4), %rcx
	movq	112(%rsp), %rdi
	incq	%r13
	leaq	40(%r12), %rsi
	movq	%r15, (%rax,%rcx,8)
	movq	%rbx, 8(%rax,%rcx,8)
	decq	%rdi
	vmovdqa	880(%rsp), %xmm0
	movq	%rdi, 112(%rsp)
	movq	16(%rsp), %rbx
	vmovdqu	%xmm0, 16(%rax,%rcx,8)
	movq	896(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r13, 288(%rsp)
	movq	8(%rsp), %r13
	testq	%rdi, %rdi
	je	.LBB110_325
.LBB110_321:
	movq	%rsi, %r12
	cmpq	%r14, %rsi
	je	.LBB110_326
	movq	(%r12), %r15
	testq	%r15, %r15
	je	.LBB110_325
	movq	32(%r12), %rax
	movq	8(%r12), %rbx
	movq	288(%rsp), %r13
	movq	%rax, 896(%rsp)
	vmovdqu	16(%r12), %xmm0
	vmovdqa	%xmm0, 880(%rsp)
	cmpq	272(%rsp), %r13
	jne	.LBB110_320
.Ltmp1457:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1458:
	jmp	.LBB110_320
.LBB110_325:
	addq	$40, %r12
.LBB110_326:
	movq	%r12, %rax
	movq	24(%rsp), %r12
	movq	88(%rsp), %r15
	jmp	.LBB110_161
.LBB110_327:
	movq	8(%rsp), %r13
	movq	96(%rsp), %rdx
	jmp	.LBB110_330
.LBB110_328:
	movq	%rbx, (%rsp)
	movq	%rbp, %rdx
	movq	56(%rsp), %rbp
.LBB110_329:
	addq	$88, %rdx
	movq	%rdx, 88(%rsp)
.LBB110_330:
	xorl	%r14d, %r14d
	movq	%rdx, 96(%rsp)
	movq	%rdx, 312(%rsp)
	jmp	.LBB110_210
.LBB110_331:
	movq	48(%rsp), %rdi
	movq	64(%rsp), %r15
	testq	%rdi, %rdi
	je	.LBB110_341
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
	jge	.LBB110_334
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_334:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_340
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_334
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
.LBB110_337:
	cmpq	%rax, %rcx
	jge	.LBB110_339
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB110_337
.LBB110_339:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_340:
	movq	40(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB110_341:
.Ltmp1467:
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp1468:
	testq	%rbx, %rbx
	je	.LBB110_352
	shlq	$3, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rbx,%rbx,2), %rcx
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
	jge	.LBB110_345
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_345:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_351
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_345
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
.LBB110_348:
	cmpq	%rax, %rdx
	jge	.LBB110_350
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB110_348
.LBB110_350:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_351:
	movq	168(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_352:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB110_355
	lock		decq	(%rax)
	jne	.LBB110_355
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB110_355:
	movq	568(%rsp), %rax
	leaq	456(%rsp), %rbx
	testq	%rax, %rax
	je	.LBB110_358
	lock		decq	(%rax)
	jne	.LBB110_358
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB110_358:
	cmpq	760(%rsp), %r15
	jne	.LBB110_157
.LBB110_359:
	movb	$1, %bpl
	xorl	%r15d, %r15d
.Ltmp1472:
	leaq	808(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1473:
	cmpq	$-1, 256(%rsp)
	movzbl	79(%rsp), %ecx
	movq	352(%rsp), %rbp
	movq	104(%rsp), %rsi
	sete	%al
	xorb	$1, %cl
	orb	72(%rsp), %cl
	orb	%al, %cl
	jne	.LBB110_365
	movq	80(%r13), %rax
	movq	$-1, %rcx
	.p2align	4
.LBB110_362:
	movq	%rax, %rdx
	incq	%rdx
	cmoveq	%rcx, %rdx
	lock		cmpxchgq	%rdx, 80(%r13)
	jne	.LBB110_362
	incq	%rax
	movq	$-1, %rcx
	cmovneq	%rax, %rcx
	movq	(%rsi), %rax
	cmpq	%rax, %rcx
	jbe	.LBB110_365
	movq	%rax, 456(%rsp)
	movq	%rcx, 464(%rsp)
	movw	$0, 448(%rsp)
.Ltmp1475:
	leaq	176(%rsp), %rdi
	leaq	448(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1476:
.LBB110_365:
	vmovdqa	272(%rsp), %xmm0
	movq	288(%rsp), %rax
	movq	%rax, 416(%rsp)
	vmovdqa	%xmm0, 400(%rsp)
	lock		decq	(%r13)
	jne	.LBB110_367
	#MEMBARRIER
.Ltmp1480:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	432(%rsp), %rdi
	callq	*%rax
.Ltmp1481:
.LBB110_367:
	movq	392(%rsp), %rax
	subq	%rbp, %rax
	je	.LBB110_380
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %r13
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %rbx
	xorl	%r14d, %r14d
	imulq	%rax, %rbx
	jmp	.LBB110_372
	.p2align	4
.LBB110_369:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_370:
	callq	*%r13
.LBB110_371:
	incq	%r14
	cmpq	%rbx, %r14
	je	.LBB110_380
.LBB110_372:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB110_371
	leaq	(%rbp,%rcx,8), %rdx
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
	jge	.LBB110_375
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_375:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_370
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_375
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
.LBB110_378:
	cmpq	%rax, %rdx
	jge	.LBB110_369
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB110_378
	jmp	.LBB110_369
.LBB110_380:
	movq	384(%rsp), %rax
	movb	$2, %r15b
	testq	%rax, %rax
	je	.LBB110_438
	movq	344(%rsp), %r14
	movq	24(%rsp), %r12
	movl	72(%rsp), %ebp
	jmp	.LBB110_429
.LBB110_382:
	movb	$1, %al
	movl	%eax, 32(%rsp)
.LBB110_383:
	movq	8(%rsp), %r13
.LBB110_384:
	movq	16(%rsp), %rbx
.LBB110_385:
	vmovdqa	272(%rsp), %xmm0
	movq	288(%rsp), %rax
	movq	48(%rsp), %rdi
	movq	%rax, 416(%rsp)
	vmovdqa	%xmm0, 400(%rsp)
	testq	%rdi, %rdi
	je	.LBB110_395
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
	jge	.LBB110_388
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_388:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_394
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_388
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
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	.p2align	4
.LBB110_391:
	cmpq	%rax, %rcx
	jge	.LBB110_393
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB110_391
.LBB110_393:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_394:
	movq	40(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB110_395:
.Ltmp1447:
	leaq	304(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp1448:
	testq	%rbx, %rbx
	je	.LBB110_406
	shlq	$3, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rbx,%rbx,2), %rcx
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
	jge	.LBB110_399
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_399:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_405
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_399
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
.LBB110_402:
	cmpq	%rax, %rdx
	jge	.LBB110_404
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB110_402
.LBB110_404:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_405:
	movq	168(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_406:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB110_409
	lock		decq	(%rax)
	jne	.LBB110_409
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB110_409:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB110_412
	lock		decq	(%rax)
	jne	.LBB110_412
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB110_412:
	xorl	%ebp, %ebp
.Ltmp1450:
	leaq	808(%rsp), %rdi
	xorl	%r15d, %r15d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1451:
	lock		decq	(%r13)
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	jne	.LBB110_415
	#MEMBARRIER
.Ltmp1452:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	432(%rsp), %rdi
	callq	*%rax
.Ltmp1453:
.LBB110_415:
	movq	376(%rsp), %rbx
	movq	392(%rsp), %rax
	movq	24(%rsp), %r12
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %r13
	subq	%rbx, %rax
	je	.LBB110_428
	shrq	$3, %rax
	movabsq	$-3689348814741910323, %r14
	xorl	%r15d, %r15d
	imulq	%rax, %r14
	jmp	.LBB110_420
	.p2align	4
.LBB110_417:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_418:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_419:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB110_428
.LBB110_420:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB110_419
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
	jge	.LBB110_423
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_423:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_418
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_423
	movq	%rcx, %rdx
	negq	%rdx
	xorl	%eax, %eax
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rdx, (%r13)
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rsi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rbp), %rax
	.p2align	4
.LBB110_426:
	cmpq	%rax, %rdx
	jge	.LBB110_417
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB110_426
	jmp	.LBB110_417
.LBB110_428:
	movq	384(%rsp), %rax
	movq	344(%rsp), %r14
	movl	72(%rsp), %ebp
	movl	32(%rsp), %r15d
	testq	%rax, %rax
	je	.LBB110_439
.LBB110_429:
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	movq	368(%rsp), %rdi
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
	jge	.LBB110_431
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB110_431:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB110_437
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB110_431
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
.LBB110_434:
	cmpq	%rax, %rdx
	jge	.LBB110_436
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB110_434
.LBB110_436:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB110_437:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	jmp	.LBB110_439
.LBB110_438:
	movq	344(%rsp), %r14
	movq	24(%rsp), %r12
	movl	72(%rsp), %ebp
.LBB110_439:
	movq	416(%rsp), %rax
	vmovaps	400(%rsp), %xmm0
	movq	%rax, 752(%rsp)
	movq	%rax, 192(%rsp)
	movq	608(%r12), %rax
	vmovaps	%xmm0, 736(%rsp)
	vmovaps	%xmm0, 176(%rsp)
	testq	%rax, %rax
	je	.LBB110_442
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	je	.LBB110_443
	testb	%r15b, %r15b
	jne	.LBB110_444
	jmp	.LBB110_448
.LBB110_442:
	cmpb	$2, %r15b
	movb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movl	%eax, %ebp
	jmp	.LBB110_449
.LBB110_443:
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 687(%rsp)
	vmovups	273(%rax), %xmm0
	vmovaps	%xmm0, 672(%rsp)
	testb	%r15b, %r15b
	je	.LBB110_448
.LBB110_444:
	movzbl	%r15b, %eax
	cmpl	$2, %eax
	je	.LBB110_449
	cmpb	$-1, %bl
	je	.LBB110_451
	cmpb	$2, 472(%r12)
	jne	.LBB110_448
	vmovaps	672(%rsp), %xmm0
	movq	687(%rsp), %rax
	movq	688(%r12), %rdi
	movb	%bl, 448(%rsp)
	vmovups	%xmm0, 449(%rsp)
	movq	%rax, 464(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	je	.LBB110_448
	addq	$16, %rdi
.Ltmp1495:
	leaq	448(%rsp), %rsi
	vzeroupper
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.1794586459888082020)
.Ltmp1496:
	jmp	.LBB110_448
.LBB110_451:
	movb	$-1, %bl
.LBB110_448:
	xorl	%ebp, %ebp
.LBB110_449:
	movq	192(%rsp), %rcx
	vmovaps	176(%rsp), %xmm0
	cmpb	$-1, %bl
	sete	%al
	andb	%bpl, %al
	movzbl	%al, %eax
	movq	%rcx, 24(%r14)
	movq	792(%rsp), %rcx
	vmovups	%xmm0, 8(%r14)
	movq	%rax, 32(%r14)
	movq	%rcx, 40(%r14)
	movq	$-1, (%r14)
.LBB110_450:
	addq	$968, %rsp
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
.LBB110_452:
	.cfi_def_cfa_offset 1024
	movq	%r14, %rax
	addq	$-1, %rax
	movb	$1, %cl
	movl	%ecx, 32(%rsp)
	jae	.LBB110_383
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbx
	cmpq	%rax, (%rsp)
	jae	.LBB110_385
	movq	96(%rsp), %rdx
	cmpq	80(%rsp), %r15
	je	.LBB110_472
	subq	(%rsp), %r14
	addq	$88, %r15
	leaq	176(%rsp), %rbx
	addq	$-2, %r14
.LBB110_456:
	movq	-80(%r15), %rax
	movq	%r15, %rdx
	cmpq	$-1, %rax
	je	.LBB110_472
	movq	-88(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %r15
	movq	-8(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1444:
	movq	264(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp1445:
	subq	$1, %r14
	jb	.LBB110_471
	movq	%r15, %rdx
	addq	$88, %r15
	cmpq	80(%rsp), %rdx
	jne	.LBB110_456
	jmp	.LBB110_472
.LBB110_460:
	movb	$1, %cl
	leaq	-1(%r14), %rax
	movl	%ecx, 32(%rsp)
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbx
	cmpq	%rax, (%rsp)
	jae	.LBB110_385
	movq	96(%rsp), %rdx
	cmpq	80(%rsp), %r15
	je	.LBB110_472
	subq	(%rsp), %r14
	addq	$88, %r15
	leaq	176(%rsp), %rbx
	addq	$-2, %r14
.LBB110_463:
	movq	-80(%r15), %rax
	movq	%r15, %rdx
	cmpq	$-1, %rax
	je	.LBB110_472
	movq	-88(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %r15
	movq	-8(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1438:
	movq	264(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp1439:
	subq	$1, %r14
	jb	.LBB110_471
	movq	%r15, %rdx
	addq	$88, %r15
	cmpq	80(%rsp), %rdx
	jne	.LBB110_463
	jmp	.LBB110_472
.LBB110_467:
	movl	$0, 32(%rsp)
	jmp	.LBB110_384
.LBB110_468:
	movb	$1, %al
	movl	%eax, 32(%rsp)
	jmp	.LBB110_385
.LBB110_471:
	movq	%r15, %rdx
.LBB110_472:
	movb	$1, %al
	movq	%rdx, 312(%rsp)
	movl	%eax, 32(%rsp)
	jmp	.LBB110_384
.LBB110_473:
.Ltmp1460:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.289(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp1461:
	jmp	.LBB110_475
.LBB110_474:
	leaq	840(%rsp), %rax
	leaq	128(%rsp), %rcx
	movq	%rdi, 840(%rsp)
	movq	%rdx, 128(%rsp)
	movq	%rax, 176(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 184(%rsp)
	movq	%rcx, 192(%rsp)
	movq	%rax, 200(%rsp)
.Ltmp1462:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	movq	40(%rsp), %r12
	movq	48(%rsp), %rbp
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.2156(%rip), %rdi
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.288(%rip), %rdx
	leaq	176(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp1463:
.LBB110_475:
	ud2
.LBB110_476:
	movb	$1, %bpl
.Ltmp1402:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %r15b
	movq	%r12, %rdi
	movq	%rbx, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.1794586459888082020)
.Ltmp1403:
	jmp	.LBB110_154
.LBB110_477:
	movq	24(%rsp), %rax
	movb	$1, %bpl
	leaq	992(%rax), %rdi
.Ltmp1404:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movb	$1, %r15b
	movq	%rbx, %rsi
	movq	%r12, %rdx
	vzeroupper
	callq	*%rax
.Ltmp1405:
	jmp	.LBB110_155
.LBB110_479:
.Ltmp1497:
	leaq	176(%rsp), %rdi
	movq	%rax, %r14
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB110_480:
.Ltmp1477:
	movb	$1, %bpl
	movq	%rax, %r14
	jmp	.LBB110_538
.LBB110_481:
.Ltmp1440:
	jmp	.LBB110_483
.LBB110_482:
.Ltmp1446:
.LBB110_483:
	movq	16(%rsp), %rbx
	movq	%rax, %r14
	movq	%r15, 312(%rsp)
	jmp	.LBB110_517
.LBB110_484:
.Ltmp1417:
	jmp	.LBB110_489
.LBB110_485:
.Ltmp1482:
	leaq	368(%rsp), %rdi
	movq	%rax, %r14
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB110_486:
.Ltmp1469:
	movq	%rax, %r14
	movb	$1, %bpl
	jmp	.LBB110_529
.LBB110_487:
.Ltmp1449:
	movq	%rax, %r14
	xorl	%ebp, %ebp
	jmp	.LBB110_529
.LBB110_488:
.Ltmp1424:
.LBB110_489:
	movq	%rax, %r14
	jmp	.LBB110_517
.LBB110_490:
.Ltmp1474:
	movq	%rax, %r14
	jmp	.LBB110_539
.LBB110_491:
.Ltmp1408:
	movq	16(%rsp), %rbx
	movq	%rax, %r14
	jmp	.LBB110_517
.LBB110_492:
.Ltmp1414:
	jmp	.LBB110_504
.LBB110_493:
.Ltmp1443:
	jmp	.LBB110_515
.LBB110_494:
.Ltmp1485:
	movq	%rax, %r14
	movq	%r13, 456(%rsp)
	cmpq	$6, %rbp
	jb	.LBB110_496
	leaq	-8(,%rbp,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
.LBB110_496:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	272(%rsp), %rdi
	jmp	.LBB110_499
.LBB110_497:
.Ltmp1427:
	jmp	.LBB110_504
.LBB110_498:
.Ltmp1401:
	movq	112(%rsp), %rdi
	movq	%rax, %r14
.LBB110_499:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	jmp	.LBB110_544
.LBB110_500:
.Ltmp1395:
	movq	%rax, %r14
	jmp	.LBB110_509
.LBB110_501:
.Ltmp1384:
	movq	%rax, %r14
	movq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB110_502:
.Ltmp1437:
	jmp	.LBB110_515
.LBB110_503:
.Ltmp1456:
.LBB110_504:
	movq	16(%rsp), %rbx
	addq	$88, %rbp
	movq	%rax, %r14
	movq	%rbp, 312(%rsp)
	jmp	.LBB110_517
.LBB110_505:
.Ltmp1459:
	addq	$40, %r12
	movq	%rax, %r14
	movq	%r12, 376(%rsp)
	cmpq	$6, %r15
	jb	.LBB110_516
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
	jmp	.LBB110_516
.LBB110_507:
.Ltmp1387:
	movq	%rax, %r14
	movq	%r15, 184(%rsp)
.Ltmp1388:
	leaq	448(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp1389:
.Ltmp1391:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1392:
.LBB110_509:
.Ltmp1396:
	leaq	672(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1397:
	movq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB110_511:
.Ltmp1390:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_512:
.Ltmp1398:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_513:
.Ltmp1432:
	addq	$88, %rbp
	movq	%rax, %r14
	movq	%rbp, 312(%rsp)
	jmp	.LBB110_516
.LBB110_514:
.Ltmp1411:
.LBB110_515:
	movq	%rax, %r14
.LBB110_516:
	movq	8(%rsp), %r13
	movq	16(%rsp), %rbx
.LBB110_517:
	movq	40(%rsp), %r12
	movq	48(%rsp), %rbp
	jmp	.LBB110_526
.LBB110_518:
.Ltmp1491:
	movq	%rax, %r14
	testq	%r15, %r15
	je	.LBB110_522
	negq	%r15
	addq	$160, %rbx
	.p2align	4
.LBB110_520:
.Ltmp1492:
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp1493:
	addq	$160, %rbx
	decq	%r15
	jne	.LBB110_520
.LBB110_522:
	cmpq	$0, (%rsp)
	je	.LBB110_545
	movq	(%rsp), %rax
	movq	64(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB110_524:
.Ltmp1494:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB110_525:
.Ltmp1464:
	movq	16(%rsp), %rbx
	movq	%rax, %r14
.LBB110_526:
	testq	%rbp, %rbp
	je	.LBB110_528
	shlq	$5, %rbp
	movl	$8, %edx
	movq	%r12, %rdi
	movq	%rbp, %rsi
	callq	__rustc::__rust_dealloc
.LBB110_528:
	movb	$1, %bpl
.Ltmp1465:
	leaq	304(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp1466:
.LBB110_529:
	testq	%rbx, %rbx
	je	.LBB110_531
	movq	168(%rsp), %rdi
	shlq	$3, %rbx
	movl	$8, %edx
	leaq	(%rbx,%rbx,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB110_531:
	movq	536(%rsp), %rax
	testq	%rax, %rax
	je	.LBB110_534
	lock		decq	(%rax)
	jne	.LBB110_534
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	536(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB110_534:
	movq	568(%rsp), %rax
	testq	%rax, %rax
	je	.LBB110_537
	lock		decq	(%rax)
	jne	.LBB110_537
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	568(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB110_537:
.Ltmp1470:
	leaq	808(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1471:
.LBB110_538:
	xorl	%r15d, %r15d
.LBB110_539:
	lock		decq	(%r13)
	jne	.LBB110_541
	#MEMBARRIER
.Ltmp1478:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	432(%rsp), %rdi
	callq	*%rax
.Ltmp1479:
.LBB110_541:
	leaq	368(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	testb	%bpl, %bpl
	je	.LBB110_543
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB110_543:
	testb	%r15b, %r15b
	je	.LBB110_545
.LBB110_544:
.Ltmp1486:
	leaq	936(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1487:
.LBB110_545:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB110_546:
.Ltmp1488:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end110:
<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>:
.Lfunc_begin112:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception75
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
	subq	$968, %rsp
	.cfi_def_cfa_offset 1024
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpb	$2, 194(%rsi)
	movq	%rcx, %r15
	movq	%rdx, %r13
	movq	%rdi, %r12
	movq	%rdx, 48(%rsp)
	jne	.LBB112_6
	cmpq	$0, 608(%r13)
	je	.LBB112_6
	movq	(%r8), %rax
	vmovdqu64	24(%r8), %zmm0
	vmovdqu64	88(%r8), %zmm2
	vmovdqu64	144(%r8), %zmm1
	movq	16(%r8), %rdx
	movq	8(%r8), %rcx
	movl	$1, %edi
	movq	%rsi, %rbx
	movl	$1, %esi
	movq	%r15, 24(%rsp)
	movq	%r12, 400(%rsp)
	cmpq	$3, %rax
	cmovaeq	%rax, %rdi
	cmovaeq	%rdx, %rax
	cmovaeq	%rsi, %rdx
	decq	%rax
	vmovdqu64	%zmm2, 664(%rsp)
	vmovdqu64	%zmm0, 600(%rsp)
	vmovdqu64	%zmm1, 720(%rsp)
	movq	%rdi, 576(%rsp)
	movq	%rcx, 584(%rsp)
	movq	%rdx, 592(%rsp)
	movq	$0, 784(%rsp)
	movq	%rax, 792(%rsp)
.Ltmp1498:
	leaq	912(%rsp), %rdi
	leaq	576(%rsp), %rsi
	vzeroupper
	callq	<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1499:
	movq	928(%rsp), %rcx
	movq	920(%rsp), %r15
	imulq	$200, %rcx, %rax
	addq	%r15, %rax
	movq	%rax, (%rsp)
	testq	%rcx, %rcx
	je	.LBB112_48
	movl	%ecx, %esi
	andl	$3, %esi
	cmpq	$4, %rcx
	jae	.LBB112_21
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB112_40
.LBB112_6:
	movq	16(%r15), %rbx
	movq	$0, 824(%rsp)
	movq	$8, 832(%rsp)
	movq	%r8, 24(%rsp)
	movq	$0, 840(%rsp)
.Ltmp1639:
	leaq	576(%rsp), %rdi
	leaq	824(%rsp), %rsi
	movq	%rbx, %rdx
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp1640:
	movq	576(%rsp), %rbp
	movq	584(%rsp), %r14
	movq	592(%rsp), %rsi
	movq	600(%rsp), %rdx
	cmpq	$-1, %rbp
	je	.LBB112_10
	vmovdqu64	608(%rsp), %zmm0
	movq	%rsi, (%rsp)
	movq	%rdx, 48(%rsp)
	vmovdqu64	%zmm0, 176(%rsp)
.Ltmp1644:
	movq	%r15, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1645:
.LBB112_9:
	vmovups	176(%rsp), %zmm0
	movq	(%rsp), %rax
	movq	48(%rsp), %rcx
	vmovups	%zmm0, 32(%r12)
	movq	%rbp, (%r12)
	movq	%r14, 8(%r12)
	movq	%rax, 16(%r12)
	movq	%rcx, 24(%r12)
	jmp	.LBB112_61
.LBB112_10:
	movq	(%r15), %rax
	movq	8(%r15), %r15
	leaq	(,%rbx,8), %rcx
	movq	%r14, 360(%rsp)
	movq	%rsi, 368(%rsp)
	movq	%rdx, 376(%rsp)
	leaq	(%rcx,%rcx,4), %r14
	leaq	(%r15,%r14), %rcx
	movq	%r15, 288(%rsp)
	movq	%rax, 304(%rsp)
	movq	%rcx, 8(%rsp)
	movq	%rcx, 312(%rsp)
	testq	%rbx, %rbx
	je	.LBB112_58
	leaq	880(%r13), %rax
	movq	%r12, 400(%rsp)
	addq	$40, %r15
	movq	%rdx, %r13
	movq	%rax, 136(%rsp)
	leaq	(,%rdx,8), %rax
	leaq	(%rax,%rax,4), %rbx
	jmp	.LBB112_14
	.p2align	4
.LBB112_12:
	movq	368(%rsp), %rsi
.LBB112_13:
	movq	(%rsp), %rax
	movq	%r12, (%rsi,%rbx)
	incq	%r13
	addq	$40, %r15
	movq	%rax, 8(%rsi,%rbx)
	movq	%rbp, 16(%rsi,%rbx)
	vmovdqa	416(%rsp), %xmm0
	vmovdqu	%xmm0, 24(%rsi,%rbx)
	addq	$40, %rbx
	addq	$-40, %r14
	movq	%r13, 376(%rsp)
	je	.LBB112_20
.LBB112_14:
	movq	48(%rsp), %rcx
	leaq	520(%rsp), %rdx
	movq	%rcx, 512(%rsp)
	movq	-8(%r15), %rax
	movq	%rax, 32(%rdx)
	vmovups	-40(%r15), %ymm0
	vmovups	%ymm0, (%rdx)
	cmpq	$0, 520(%rsp)
	je	.LBB112_16
	leaq	-40(%r15), %rax
	leaq	584(%rsp), %rdx
	movq	32(%rax), %rcx
	movq	%rcx, 32(%rdx)
	vmovups	(%rax), %ymm0
	movq	%rdx, %rax
	vmovups	%ymm0, (%rdx)
	jmp	.LBB112_18
	.p2align	4
.LBB112_16:
	movq	656(%rcx), %rdx
	movq	%rsi, %r12
.Ltmp1647:
	movq	136(%rsp), %rsi
	leaq	576(%rsp), %rdi
	leaq	528(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp1648:
	movq	576(%rsp), %rbp
	leaq	584(%rsp), %rax
	movq	%r12, %rsi
	cmpq	$-1, %rbp
	jne	.LBB112_107
.LBB112_18:
	movq	592(%rsp), %rcx
	movq	%rax, %rdx
	movq	24(%rax), %rax
	movq	584(%rsp), %r12
	movq	600(%rsp), %rbp
	movq	%rcx, (%rsp)
	movq	32(%rdx), %rcx
	movq	%rax, 416(%rsp)
	movq	%rcx, 424(%rsp)
	cmpq	360(%rsp), %r13
	jne	.LBB112_13
.Ltmp1652:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	360(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1653:
	jmp	.LBB112_12
.LBB112_20:
	movq	8(%rsp), %r15
	movq	400(%rsp), %r12
	jmp	.LBB112_59
.LBB112_21:
	movq	%rcx, %r8
	andq	$-4, %r8
	leaq	776(%r15), %r9
	xorl	%edi, %edi
	xorl	%r10d, %r10d
	jmp	.LBB112_23
	.p2align	4
.LBB112_22:
	addq	$4, %rdi
	addq	$800, %r9
	cmpq	%rdi, %r8
	je	.LBB112_39
.LBB112_23:
	movq	-600(%r9), %rax
	mulq	-608(%r9)
	jo	.LBB112_32
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB112_25
.LBB112_33:
	movq	%r10, %r11
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jno	.LBB112_26
.LBB112_34:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jae	.LBB112_35
	.p2align	4
.LBB112_27:
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jo	.LBB112_36
.LBB112_28:
	addq	%rax, %r10
	movq	$-1, %r11
	jb	.LBB112_29
.LBB112_37:
	movq	%r10, %r11
	movq	(%r9), %rax
	mulq	-8(%r9)
	jno	.LBB112_30
.LBB112_38:
	movq	$-1, %rax
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB112_22
	jmp	.LBB112_31
.LBB112_32:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB112_33
	.p2align	4
.LBB112_25:
	movq	-400(%r9), %rax
	mulq	-408(%r9)
	jo	.LBB112_34
.LBB112_26:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB112_27
.LBB112_35:
	movq	%r11, %r10
	movq	-200(%r9), %rax
	mulq	-208(%r9)
	jno	.LBB112_28
.LBB112_36:
	movq	$-1, %rax
	addq	%rax, %r10
	movq	$-1, %r11
	jae	.LBB112_37
	.p2align	4
.LBB112_29:
	movq	(%r9), %rax
	mulq	-8(%r9)
	jo	.LBB112_38
.LBB112_30:
	addq	%rax, %r11
	movq	$-1, %r10
	jb	.LBB112_22
.LBB112_31:
	movq	%r11, %r10
	jmp	.LBB112_22
.LBB112_39:
	testq	%rsi, %rsi
	je	.LBB112_44
.LBB112_40:
	imulq	$200, %rdi, %rax
	imulq	$200, %rsi, %rsi
	movq	$-1, %r9
	xorl	%r8d, %r8d
	leaq	176(%rax,%r15), %rdi
	.p2align	4
.LBB112_41:
	movq	(%rdi,%r8), %rax
	mulq	-8(%rdi,%r8)
	jo	.LBB112_43
.LBB112_42:
	addq	%rax, %r10
	cmovbq	%r9, %r10
	addq	$200, %r8
	cmpq	%r8, %rsi
	jne	.LBB112_41
	jmp	.LBB112_44
.LBB112_43:
	movq	$-1, %rax
	jmp	.LBB112_42
.LBB112_44:
	testq	%r10, %r10
	je	.LBB112_48
	movq	608(%r13), %rax
	testq	%rax, %rax
	je	.LBB112_48
	cmpq	$0, 336(%rax)
	je	.LBB112_48
	lock		addq	%r10, 352(%rax)
.LBB112_48:
	movq	%rbx, 8(%rsp)
	movq	912(%rsp), %rax
	movq	%r15, 176(%rsp)
	movq	$0, 512(%rsp)
	movq	$8, 520(%rsp)
	movq	$0, 528(%rsp)
	movq	(%rsp), %rdx
	movq	%rax, 192(%rsp)
	movq	%rdx, 200(%rsp)
	testq	%rcx, %rcx
	je	.LBB112_57
	movl	$8, %eax
	leaq	584(%rsp), %r14
	addq	$200, %r15
	xorl	%r12d, %r12d
	xorl	%r13d, %r13d
	movq	%rax, 136(%rsp)
	.p2align	4
.LBB112_50:
	movq	-200(%r15), %rax
	cmpq	$-1, %rax
	je	.LBB112_62
	leaq	-200(%r15), %rbp
	vmovups	8(%rbp), %zmm0
	vmovups	72(%rbp), %zmm1
	vmovups	96(%rbp), %zmm2
	vmovups	%zmm2, 88(%r14)
	vmovups	%zmm1, 64(%r14)
	vmovups	%zmm0, (%r14)
	movq	%rax, 576(%rsp)
	movzbl	728(%rsp), %ebx
	cmpq	512(%rsp), %r12
	jne	.LBB112_54
.Ltmp1501:
	movq	<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one@GOTPCREL(%rip), %rax
	leaq	512(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1502:
	movq	520(%rsp), %rax
	movq	%rax, 136(%rsp)
.LBB112_54:
	vmovdqu64	576(%rsp), %zmm0
	vmovdqu64	640(%rsp), %zmm1
	vmovdqu64	672(%rsp), %zmm2
	movq	136(%rsp), %rax
	vmovdqu64	%zmm2, 96(%rax,%r13)
	vmovdqu64	%zmm1, 64(%rax,%r13)
	vmovdqu64	%zmm0, (%rax,%r13)
	leaq	1(%r12), %rax
	movq	%rax, 528(%rsp)
	testb	%bl, %bl
	jne	.LBB112_64
	addq	$160, %r13
	addq	$200, %r15
	addq	$200, %rbp
	movq	%rax, %r12
	cmpq	(%rsp), %rbp
	jne	.LBB112_50
	movq	(%rsp), %r15
	movq	48(%rsp), %r13
	movq	%rax, %r14
	jmp	.LBB112_63
.LBB112_57:
	movl	$8, %eax
	xorl	%r14d, %r14d
	movq	%rax, 136(%rsp)
	jmp	.LBB112_63
.LBB112_58:
	movq	%rdx, %r13
.LBB112_59:
	movq	%r15, 296(%rsp)
.Ltmp1658:
	leaq	288(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1659:
	vmovdqu	360(%rsp), %xmm0
	vmovdqu	%xmm0, 8(%r12)
	movq	%r13, 24(%r12)
	movq	$0, 32(%r12)
	movq	$-1, (%r12)
.LBB112_61:
	movq	24(%rsp), %rdi
	addq	$968, %rsp
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
	jmp	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.LBB112_62:
	.cfi_def_cfa_offset 1024
	movq	48(%rsp), %r13
	movq	%r12, %r14
.LBB112_63:
	movq	%r15, 184(%rsp)
	xorl	%ebx, %ebx
	jmp	.LBB112_65
.LBB112_64:
	movq	48(%rsp), %r13
	movq	%r12, %rcx
	incq	%rcx
	movb	$1, %bl
	movq	%r15, 184(%rsp)
	movq	%rcx, %r14
.LBB112_65:
.Ltmp1509:
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1510:
	movq	136(%rsp), %r15
	movq	512(%rsp), %rax
	movq	%r14, 88(%rsp)
	movq	%rax, 408(%rsp)
	testq	%r14, %r14
	je	.LBB112_69
	cmpq	$8, %r14
	jae	.LBB112_70
	xorl	%eax, %eax
	xorl	%ebp, %ebp
	jmp	.LBB112_79
.LBB112_69:
	xorl	%ebp, %ebp
	jmp	.LBB112_81
.LBB112_70:
	cmpq	$32, %r14
	jae	.LBB112_72
	xorl	%eax, %eax
	xorl	%ebp, %ebp
	jmp	.LBB112_76
.LBB112_72:
	vmovdqa64	.LCPI112_0(%rip), %zmm1
	vpbroadcastq	.LCPI112_1(%rip), %zmm2
	vpbroadcastq	.LCPI112_2(%rip), %zmm3
	movq	%r14, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB112_73:
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
	jne	.LBB112_73
	vpaddq	%zmm0, %zmm4, %zmm0
	movq	88(%rsp), %rcx
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbp
	cmpq	%rax, %rcx
	je	.LBB112_81
	testb	$24, %cl
	je	.LBB112_79
.LBB112_76:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI112_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI112_1(%rip), %zmm2
	vpbroadcastq	.LCPI112_3(%rip), %zmm3
	movq	88(%rsp), %rax
	vmovq	%rbp, %xmm0
	andq	$-8, %rax
	subq	%rax, %rcx
	.p2align	4
.LBB112_77:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	64(%r15,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB112_77
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rbp
	cmpq	%rax, 88(%rsp)
	je	.LBB112_81
.LBB112_79:
	movq	88(%rsp), %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	64(%rax,%r15), %rax
	.p2align	4
.LBB112_80:
	addq	(%rax), %rbp
	addq	$160, %rax
	decq	%rcx
	jne	.LBB112_80
.LBB112_81:
	movq	8(%rsp), %rax
	movq	$0, 176(%rsp)
	movq	$8, 184(%rsp)
	movq	%r13, %r14
	movq	$0, 192(%rsp)
	movzbl	193(%rax), %ecx
	movzbl	195(%rax), %eax
	movb	%cl, 87(%rsp)
	movq	%rax, 888(%rsp)
	movq	408(%rsp), %rax
	movq	88(%rsp), %r8
	movl	%ebx, 172(%rsp)
	movq	24(%rsp), %rcx
	movq	%rax, 936(%rsp)
	movq	%r15, 944(%rsp)
	movq	%r8, 952(%rsp)
	movb	%bl, 960(%rsp)
	movq	16(%rcx), %rbx
.Ltmp1517:
	leaq	576(%rsp), %rdi
	leaq	176(%rsp), %rsi
	movq	%rbx, %rdx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp1518:
	movq	576(%rsp), %rcx
	movq	584(%rsp), %rax
	movq	592(%rsp), %r13
	movq	600(%rsp), %r12
	movq	%rcx, (%rsp)
	cmpq	$-1, %rcx
	je	.LBB112_101
	vmovdqu	609(%rsp), %ymm0
	vmovdqu	640(%rsp), %ymm1
	movq	%rax, 64(%rsp)
	movzbl	608(%rsp), %eax
	movl	%eax, 32(%rsp)
	vmovdqu	%ymm0, 512(%rsp)
	vmovdqu	%ymm1, 543(%rsp)
.Ltmp1522:
	movq	24(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1523:
	movq	%r12, %rbx
	movq	%r13, %r12
	movq	%r14, %r13
.LBB112_84:
	movq	88(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_88
	movq	136(%rsp), %r14
	movl	$1, %r15d
	subq	%rax, %r15
	.p2align	4
.LBB112_86:
.Ltmp1630:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp1631:
	incq	%r15
	addq	$160, %r14
	cmpq	$1, %r15
	jne	.LBB112_86
.LBB112_88:
	movq	408(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_99
	shlq	$5, %rax
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
	jge	.LBB112_91
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB112_91:
	movq	136(%rsp), %rdi
	.p2align	4
.LBB112_92:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_98
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_92
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
.LBB112_95:
	cmpq	%rax, %rsi
	jge	.LBB112_97
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB112_95
.LBB112_97:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_98:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_99:
	movq	(%rsp), %rax
	cmpq	$-1, %rax
	je	.LBB112_106
	vmovups	512(%rsp), %ymm0
	vmovdqu	543(%rsp), %ymm1
	movq	400(%rsp), %rcx
	vmovdqu	%ymm1, 64(%rcx)
	vmovups	%ymm0, 33(%rcx)
	movq	%rax, (%rcx)
	movq	64(%rsp), %rax
	movq	%rax, 8(%rcx)
	movl	32(%rsp), %eax
	movq	%r12, 16(%rcx)
	movq	%rbx, 24(%rcx)
	movb	%al, 32(%rcx)
	jmp	.LBB112_374
.LBB112_101:
	movq	24(%rsp), %rcx
	movq	%rax, 144(%rsp)
	movq	%r13, 152(%rsp)
	movq	%r12, 160(%rsp)
	movq	%rbp, 112(%rsp)
	movq	8(%rcx), %rdx
	movq	(%rcx), %rax
	leaq	(%rbx,%rbx,4), %rcx
	movq	%rdx, 416(%rsp)
	movq	%rax, 432(%rsp)
	movq	608(%r14), %rax
	leaq	(%rdx,%rcx,8), %rcx
	movq	%rdx, 424(%rsp)
	movq	%rcx, 440(%rsp)
	movq	%rax, 800(%rsp)
	testq	%rax, %rax
	je	.LBB112_131
	lock		incq	(%rax)
	jle	.LBB112_453
	movq	608(%r14), %rcx
	movq	%rdx, %rbx
	cmpq	$0, 88(%rsp)
	movq	%r14, %r13
	movq	%rcx, 504(%rsp)
	sete	%al
	movq	16(%rcx), %rdx
	movq	%rdx, 352(%rsp)
	movq	%rcx, 8(%rsp)
	movq	40(%rcx), %rcx
	cmpq	$-1, %rcx
	movq	%rcx, 344(%rsp)
	sete	%cl
	orb	%al, %cl
	jne	.LBB112_157
	movq	88(%rsp), %rax
	cmpq	$8, %rax
	jae	.LBB112_141
	xorl	%eax, %eax
	xorl	%r12d, %r12d
	jmp	.LBB112_153
.LBB112_106:
	movl	172(%rsp), %ecx
	movl	32(%rsp), %edx
	jmp	.LBB112_362
.LBB112_107:
	movq	%rax, %rcx
	movq	592(%rsp), %rax
	vmovdqa	24(%rcx), %xmm0
	vmovups	640(%rsp), %ymm1
	vmovups	624(%rsp), %ymm2
	movq	584(%rsp), %r14
	movq	%r15, 296(%rsp)
	movq	%rax, (%rsp)
	movq	600(%rsp), %rax
	vmovups	%ymm1, 208(%rsp)
	vmovups	%ymm2, 192(%rsp)
	vmovdqa	%xmm0, 416(%rsp)
	vmovdqa	%xmm0, 176(%rsp)
	movq	%rax, 48(%rsp)
.Ltmp1650:
	leaq	288(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1651:
	movq	%r12, %rdi
	testq	%r13, %r13
	je	.LBB112_121
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbx
	xorl	%r15d, %r15d
	jmp	.LBB112_113
	.p2align	4
.LBB112_110:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_111:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	%r12, %rdi
.LBB112_112:
	incq	%r15
	cmpq	%r15, %r13
	je	.LBB112_121
.LBB112_113:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rdi,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB112_112
	leaq	(%rdi,%rcx,8), %rdx
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
	jge	.LBB112_116
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB112_116:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_111
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_116
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
	movq	(%rbx), %rax
	.p2align	4
.LBB112_119:
	cmpq	%rax, %rdx
	jge	.LBB112_110
	lock		cmpxchgq	%rdx, (%rbx)
	jne	.LBB112_119
	jmp	.LBB112_110
.LBB112_121:
	movq	360(%rsp), %rax
	movq	400(%rsp), %r12
	testq	%rax, %rax
	je	.LBB112_9
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
	jge	.LBB112_124
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB112_124:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_130
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_124
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
.LBB112_127:
	cmpq	%rax, %rdx
	jge	.LBB112_129
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB112_127
.LBB112_129:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_130:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	jmp	.LBB112_9
.LBB112_131:
	movq	424(%rsp), %rcx
	movq	416(%rsp), %rax
	movq	432(%rsp), %rdx
	movq	%rcx, 296(%rsp)
	movq	440(%rsp), %rcx
	movq	%rax, 288(%rsp)
	movq	%rdx, 304(%rsp)
	movq	%rcx, 312(%rsp)
	movq	312(%rsp), %rax
	movq	296(%rsp), %rbp
	movq	%rax, 24(%rsp)
	cmpq	%rax, %rbp
	je	.LBB112_143
	leaq	880(%r14), %rax
	movq	%r12, %rbx
	movq	%rax, 8(%rsp)
	leaq	(,%r12,8), %rax
	movq	%r13, %r12
	movq	%r14, %r13
	leaq	(%rax,%rax,4), %r15
	jmp	.LBB112_135
	.p2align	4
.LBB112_133:
	movq	152(%rsp), %rsi
.LBB112_134:
	movq	(%rsp), %rax
	movq	%r12, (%rsi,%r15)
	addq	$40, %rbp
	movq	%rsi, %r12
	movq	%rax, 8(%rsi,%r15)
	movq	%r13, 16(%rsi,%r15)
	movb	%bl, 24(%rsi,%r15)
	movq	48(%rsp), %r13
	movq	%r14, %rbx
	incq	%rbx
	movq	464(%rsp), %rax
	movq	471(%rsp), %rcx
	movq	%rax, 25(%rsi,%r15)
	movq	%rcx, 32(%rsi,%r15)
	addq	$40, %r15
	movq	%rbx, 160(%rsp)
	cmpq	24(%rsp), %rbp
	je	.LBB112_144
.LBB112_135:
	movq	32(%rbp), %rax
	leaq	184(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%r13, 176(%rsp)
	cmpq	$0, 184(%rsp)
	je	.LBB112_137
	movq	32(%rbp), %rax
	leaq	584(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB112_139
	.p2align	4
.LBB112_137:
	movq	656(%r13), %rdx
.Ltmp1609:
	movq	8(%rsp), %rsi
	leaq	576(%rsp), %rdi
	leaq	192(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp1610:
	leaq	584(%rsp), %rcx
	movq	576(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB112_377
.LBB112_139:
	movq	%rbx, %rdx
	movq	25(%rcx), %rdi
	movq	%r12, %rsi
	movq	592(%rsp), %rax
	movq	584(%rsp), %r12
	movq	600(%rsp), %r13
	movzbl	608(%rsp), %ebx
	movq	32(%rcx), %rcx
	movq	%rdx, %r14
	movq	%rdi, 464(%rsp)
	movq	%rax, (%rsp)
	movq	%rcx, 471(%rsp)
	cmpq	144(%rsp), %rdx
	jne	.LBB112_134
.Ltmp1617:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1618:
	jmp	.LBB112_133
.LBB112_141:
	cmpq	$32, %rax
	jae	.LBB112_146
	xorl	%eax, %eax
	xorl	%r12d, %r12d
	jmp	.LBB112_150
.LBB112_143:
	movq	%r12, %rbx
	movq	%r14, %r13
.LBB112_144:
	movb	$1, %r14b
	movq	%rbp, 296(%rsp)
.Ltmp1622:
	leaq	288(%rsp), %rdi
	movb	$1, %r15b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1623:
	movq	112(%rsp), %rbp
	movq	144(%rsp), %rax
	movq	152(%rsp), %r12
	movq	%rax, 64(%rsp)
	movb	$2, %al
	movq	$-1, (%rsp)
	movl	%eax, 32(%rsp)
	jmp	.LBB112_84
.LBB112_146:
	vmovdqa64	.LCPI112_0(%rip), %zmm1
	vpbroadcastq	.LCPI112_1(%rip), %zmm2
	vpbroadcastq	.LCPI112_2(%rip), %zmm3
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB112_147:
	vpmullq	%zmm2, %zmm1, %zmm7
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm8, %xmm8, %xmm8
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$-32, %rcx
	vpgatherqq	16(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm0, %zmm8, %zmm0
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	1296(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm4, %zmm8, %zmm4
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	2576(%r15,%zmm7), %zmm8 {%k1}
	kxnorb	%k0, %k0, %k1
	vpaddq	%zmm5, %zmm8, %zmm5
	vpxor	%xmm8, %xmm8, %xmm8
	vpgatherqq	3856(%r15,%zmm7), %zmm8 {%k1}
	vpaddq	%zmm6, %zmm8, %zmm6
	jne	.LBB112_147
	vpaddq	%zmm0, %zmm4, %zmm0
	movq	88(%rsp), %rcx
	vpaddq	%zmm0, %zmm5, %zmm0
	vpaddq	%zmm0, %zmm6, %zmm0
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r12
	cmpq	%rax, %rcx
	je	.LBB112_155
	testb	$24, %cl
	je	.LBB112_153
.LBB112_150:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI112_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI112_1(%rip), %zmm2
	vpbroadcastq	.LCPI112_3(%rip), %zmm3
	movq	88(%rsp), %rax
	vmovq	%r12, %xmm0
	andq	$-8, %rax
	subq	%rax, %rcx
	.p2align	4
.LBB112_151:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%r15,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB112_151
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r12
	cmpq	%rax, 88(%rsp)
	je	.LBB112_155
.LBB112_153:
	movq	88(%rsp), %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%r15), %rax
	.p2align	4
.LBB112_154:
	addq	(%rax), %r12
	addq	$160, %rax
	decq	%rcx
	jne	.LBB112_154
.LBB112_155:
	movq	904(%r13), %rax
	movq	920(%r13), %rsi
	leaq	904(%r13), %rdx
	subq	%rsi, %rax
	cmpq	%rax, %r12
	ja	.LBB112_454
.LBB112_156:
	cmpq	1008(%r13), %r12
	ja	.LBB112_455
.LBB112_157:
	movq	8(%rsp), %rax
	movq	%r15, 824(%rsp)
	movq	%r15, 832(%rsp)
	addq	$16, %rax
	movq	%rax, 96(%rsp)
	movq	88(%rsp), %rax
	movq	408(%rsp), %rcx
	leaq	(%rax,%rax,4), %rdx
	movq	%rcx, 840(%rsp)
	shlq	$5, %rdx
	addq	%r15, %rdx
	movq	%rdx, 856(%rsp)
	movq	%rdx, 848(%rsp)
	testq	%rax, %rax
	je	.LBB112_355
	movq	8(%rsp), %rax
	leaq	880(%r13), %rcx
	movq	$-1, %r12
	movq	%r15, %r14
	addq	$272, %rax
	movq	%rax, 456(%rsp)
	movq	%rcx, 64(%rsp)
.LBB112_159:
	movq	%r14, %rcx
	addq	$160, %r14
	movq	%r14, 832(%rsp)
	movq	(%rcx), %rax
	cmpq	$-1, %rax
	je	.LBB112_355
	movq	%rax, 576(%rsp)
	vmovdqu64	8(%rcx), %zmm0
	vmovdqu64	72(%rcx), %zmm1
	vmovdqu64	96(%rcx), %zmm2
	leaq	584(%rsp), %rcx
	vmovdqu64	%zmm2, 88(%rcx)
	vmovdqu64	%zmm1, 64(%rcx)
	vmovdqu64	%zmm0, (%rcx)
	movq	584(%rsp), %rsi
	movq	600(%rsp), %rcx
	movq	616(%rsp), %rdx
	imulq	$88, 592(%rsp), %r8
	movq	624(%rsp), %rdi
	movq	%rcx, 40(%rsp)
	movq	%rdx, 880(%rsp)
	movq	%rsi, 360(%rsp)
	movq	%rax, 376(%rsp)
	movq	608(%rsp), %rcx
	movq	632(%rsp), %rax
	movq	640(%rsp), %rdx
	movq	%rsi, 368(%rsp)
	addq	%rsi, %r8
	movq	%r8, 32(%rsp)
	movq	%rdi, 56(%rsp)
	movq	%r8, 384(%rsp)
	movq	%rcx, 16(%rsp)
	movq	%rax, 496(%rsp)
	testq	%rdx, %rdx
	je	.LBB112_326
	movq	672(%rsp), %rdi
	shlq	$5, %rdx
	addq	$8, %rcx
	addq	%rax, %rdx
	movq	%rdx, 872(%rsp)
	movq	%rdi, (%rsp)
	movq	%rcx, 864(%rsp)
	movq	$0, 392(%rsp)
	movq	%rsi, 120(%rsp)
	movq	%rsi, 72(%rsp)
	movq	8(%rsp), %r15
	movq	%r14, 128(%rsp)
	jmp	.LBB112_163
.LBB112_162:
	movq	896(%rsp), %rax
	movq	$-1, %r12
	movq	%rbx, 424(%rsp)
	addq	$32, %rax
	cmpq	872(%rsp), %rax
	je	.LBB112_327
.LBB112_163:
	movq	%rbx, 272(%rsp)
	movq	16(%rax), %rdx
	movq	(%rax), %r14
	movq	8(%rax), %rbx
	movq	392(%rsp), %rcx
	movq	%rcx, 104(%rsp)
	movq	%rdx, 280(%rsp)
	movq	%rax, 896(%rsp)
	movq	24(%rax), %rax
	movq	%rax, 24(%rsp)
	testq	%r14, %r14
	je	.LBB112_173
	cmpq	$-1, 352(%rsp)
	je	.LBB112_173
	movq	80(%r15), %rax
	.p2align	4
.LBB112_166:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%r12, %rcx
	lock		cmpxchgq	%rcx, 80(%r15)
	jne	.LBB112_166
	movq	96(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%r12, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB112_170
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp1529:
	movq	96(%rsp), %rsi
	leaq	288(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1530:
	cmpb	$-1, 288(%rsp)
	jne	.LBB112_395
.LBB112_170:
	movq	624(%r13), %rax
	testq	%rax, %rax
	je	.LBB112_173
	movl	1220(%r13), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB112_173
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	888(%rsp), %rax
	lock		addq	%r14, (%rcx,%rax,8)
	.p2align	4
.LBB112_173:
	movq	880(%rsp), %rdx
	movq	104(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB112_452
	cmpq	%rdx, %rbx
	movq	%rdx, %rsi
	movq	16(%rsp), %r12
	movq	40(%rsp), %r13
	cmovbq	%rbx, %rsi
	cmpq	%rdi, %rbx
	movq	56(%rsp), %rbx
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB112_451
	leaq	(,%rdi,8), %rax
	movq	$-1, %r8
	movq	%rsi, 392(%rsp)
	leaq	(%rax,%rax,2), %r14
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r15
	cmpq	%rsi, %rdi
	jne	.LBB112_182
	movq	344(%rsp), %rdi
	xorl	%ebp, %ebp
	xorl	%r11d, %r11d
	xorl	%r10d, %r10d
.LBB112_177:
	movq	%r11, 808(%rsp)
	movq	%r10, 816(%rsp)
	cmpq	$-1, %rdi
	je	.LBB112_188
	movq	280(%rsp), %rax
	movl	$0, %ecx
	movq	32(%rsp), %r12
	movl	$0, %r13d
	subq	(%rsp), %rax
	cmovbq	%rcx, %rax
	subq	72(%rsp), %r12
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %r12
	imulq	%rcx, %r12
	cmpq	%r12, %rax
	cmovbq	%rax, %r12
	testq	%r12, %r12
	je	.LBB112_189
	movq	72(%rsp), %rax
	xorl	%r13d, %r13d
	leaq	8(%rax), %rbx
	.p2align	4
.LBB112_180:
.Ltmp1532:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.Ltmp1533:
	addq	%rax, %r13
	movq	$-1, %rcx
	cmovbq	%rcx, %r13
	addq	$88, %rbx
	decq	%r12
	jne	.LBB112_180
	jmp	.LBB112_189
.LBB112_182:
	movq	%r15, %rdx
	subq	%r14, %rdx
	movabsq	$-6148914691236517205, %rax
	leaq	.LJTI112_0(%rip), %r9
	xorl	%r10d, %r10d
	xorl	%r11d, %r11d
	xorl	%ebp, %ebp
	mulxq	%rax, %rax, %rax
	movq	864(%rsp), %rcx
	movq	344(%rsp), %rdi
	shrq	$4, %rax
	addq	%r14, %rcx
	jmp	.LBB112_185
	.p2align	4
.LBB112_183:
	addq	%rdx, %r11
	cmovbq	%r8, %r11
.LBB112_184:
	addq	$24, %rcx
	decq	%rax
	je	.LBB112_177
.LBB112_185:
	movzbl	-8(%rcx), %esi
	movq	(%rcx), %rdx
	movslq	(%r9,%rsi,4), %rsi
	addq	%r9, %rsi
	jmpq	*%rsi
.LBB112_186:
	addq	%rdx, %rbp
	cmovbq	%r8, %rbp
	jmp	.LBB112_184
	.p2align	4
.LBB112_187:
	cmpq	%rdx, %r10
	cmovbeq	%rdx, %r10
	jmp	.LBB112_184
.LBB112_188:
	xorl	%r13d, %r13d
.LBB112_189:
	movq	8(%rsp), %rax
	movl	296(%rax), %eax
	movq	344(%rsp), %r12
	testl	%eax, %eax
	je	.LBB112_214
.LBB112_190:
	cmpq	$-1, 352(%rsp)
	je	.LBB112_192
	movq	8(%rsp), %rcx
	movq	$-1, %rdx
	movq	80(%rcx), %rax
	addq	%rbp, %rax
	cmovbq	%rdx, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB112_215
.LBB112_192:
	cmpq	$-1, %r12
	je	.LBB112_194
	movq	48(%rsp), %rcx
	movq	$-1, %rsi
	movq	1040(%rcx), %rax
	movq	1048(%rcx), %rcx
	movq	8(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%r13, %rcx
	cmovbq	%rsi, %rcx
	addq	808(%rsp), %rcx
	cmovbq	%rsi, %rcx
	addq	816(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%rsi, %rcx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	cmpq	40(%rdx), %rax
	ja	.LBB112_215
.LBB112_194:
	cmpq	$-1, 352(%rsp)
	movq	48(%rsp), %r13
	je	.LBB112_196
	movq	104(%rsp), %rcx
	movq	%r14, %rax
	cmpq	392(%rsp), %rcx
	jne	.LBB112_209
.LBB112_196:
	movb	$1, %r12b
	movq	104(%rsp), %rax
	cmpq	392(%rsp), %rax
	je	.LBB112_199
	.p2align	4
.LBB112_197:
	movq	16(%rsp), %rax
	cmpb	$2, -24(%rax,%r15)
	je	.LBB112_279
	addq	$-24, %r15
	cmpq	%r15, %r14
	jne	.LBB112_197
.LBB112_199:
	movq	8(%rsp), %r15
	movq	272(%rsp), %rbx
	movq	128(%rsp), %r14
.LBB112_200:
	cmpq	$-1, 352(%rsp)
	je	.LBB112_269
	testq	%rbp, %rbp
	je	.LBB112_269
	movq	80(%r15), %rax
	movq	$-1, %rdx
	.p2align	4
.LBB112_203:
	movq	%rax, %rcx
	addq	%rbp, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 80(%r15)
	jne	.LBB112_203
	movq	96(%rsp), %rcx
	addq	%rbp, %rax
	cmovbq	%rdx, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB112_269
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp1538:
	movq	96(%rsp), %rsi
	leaq	288(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1539:
	cmpb	$-1, 288(%rsp)
	je	.LBB112_269
	cmpq	$-1, 344(%rsp)
	movq	112(%rsp), %rbp
	movb	$1, %r12b
	jne	.LBB112_300
	jmp	.LBB112_394
	.p2align	4
.LBB112_208:
	addq	$24, %rax
	cmpq	%rax, %r15
	je	.LBB112_196
.LBB112_209:
	movq	16(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB112_208
	movq	16(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB112_208
	movq	624(%r13), %rdx
	testq	%rdx, %rdx
	je	.LBB112_208
	movl	1220(%r13), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB112_208
	movq	16(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB112_208
.LBB112_214:
	movq	456(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB112_190
.LBB112_215:
	movq	104(%rsp), %rax
	cmpq	392(%rsp), %rax
	jne	.LBB112_225
	movq	48(%rsp), %r13
	movq	120(%rsp), %rbp
.LBB112_217:
	movq	%rbp, 120(%rsp)
	cmpq	$-1, %r12
	je	.LBB112_276
	movq	280(%rsp), %rax
	movq	112(%rsp), %rbp
	movq	8(%rsp), %r15
	movq	272(%rsp), %rbx
	movq	128(%rsp), %r14
	cmpq	%rax, (%rsp)
	jae	.LBB112_312
	movq	72(%rsp), %rax
	cmpq	32(%rsp), %rax
	je	.LBB112_278
	movq	280(%rsp), %rax
	leaq	184(%rsp), %r12
	decq	%rax
	movq	%rax, 104(%rsp)
	movq	72(%rsp), %rdx
	.p2align	4
.LBB112_221:
	movq	8(%rdx), %rax
	movq	%rdx, %rcx
	cmpq	$-1, %rax
	je	.LBB112_287
	movq	(%rcx), %rsi
	movq	%rax, 176(%rsp)
	movq	%rcx, %r14
	movq	80(%rcx), %rax
	movq	%rax, 64(%r12)
	vmovdqu64	16(%rcx), %zmm0
	vmovdqu64	%zmm0, (%r12)
.Ltmp1570:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1571:
	movq	(%rsp), %rax
	cmpq	%rax, 104(%rsp)
	je	.LBB112_286
	incq	%rax
	leaq	88(%r14), %rdx
	movq	%r14, %rcx
	movq	%rax, (%rsp)
	cmpq	32(%rsp), %rdx
	jne	.LBB112_221
	jmp	.LBB112_287
.LBB112_225:
	movq	16(%rsp), %rax
	movq	48(%rsp), %r13
	movq	120(%rsp), %rbp
	addq	%rax, %r14
	addq	%rax, %r15
	jmp	.LBB112_228
.LBB112_226:
	movq	48(%rsp), %r13
	.p2align	4
.LBB112_227:
	addq	$24, %r14
	cmpq	%r15, %r14
	je	.LBB112_217
.LBB112_228:
	movzbl	(%r14), %eax
	leaq	.LJTI112_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB112_229:
	cmpq	$-1, 352(%rsp)
	je	.LBB112_227
	movq	16(%r14), %rax
	movzbl	1(%r14), %ebx
	movq	8(%r14), %r13
	movq	$-1, %rsi
	movq	%rax, 104(%rsp)
	movq	8(%rsp), %rdx
	movq	80(%rdx), %rax
	.p2align	4
.LBB112_231:
	movq	%rax, %rcx
	addq	%r13, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB112_231
	movq	96(%rsp), %rcx
	addq	%r13, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB112_235
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$0, 176(%rsp)
.Ltmp1564:
	movq	96(%rsp), %rsi
	leaq	288(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1565:
	cmpb	$-1, 288(%rsp)
	jne	.LBB112_379
.LBB112_235:
	cmpl	$255, %ebx
	je	.LBB112_226
	movq	48(%rsp), %rcx
	movq	624(%rcx), %rax
	testq	%rax, %rax
	je	.LBB112_262
	movl	1220(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB112_226
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r13, (%rcx,%rbx,8)
	jmp	.LBB112_226
	.p2align	4
.LBB112_239:
	movq	8(%r14), %rbx
	cmpq	%rbx, (%rsp)
	jae	.LBB112_266
	movq	72(%rsp), %rax
	cmpq	32(%rsp), %rax
	je	.LBB112_265
	movq	72(%rsp), %rcx
	leaq	-1(%rbx), %r12
	.p2align	4
.LBB112_242:
	movq	8(%rcx), %rax
	movq	%rcx, %rbp
	cmpq	$-1, %rax
	je	.LBB112_264
	movq	(%rbp), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	80(%rbp), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rbp), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1553:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1554:
	movq	(%rsp), %rax
	cmpq	%rax, %r12
	je	.LBB112_263
	incq	%rax
	leaq	88(%rbp), %rcx
	movq	%rax, (%rsp)
	cmpq	32(%rsp), %rcx
	jne	.LBB112_242
	jmp	.LBB112_264
	.p2align	4
.LBB112_246:
	cmpq	$-1, %r12
	je	.LBB112_227
	movq	8(%rsp), %rdx
	movq	8(%r14), %rcx
	movl	296(%rdx), %eax
	testl	%eax, %eax
	je	.LBB112_258
	movq	104(%rdx), %rax
	movq	$-1, %rsi
	addq	%rcx, %rax
	movq	40(%rdx), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB112_227
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp1551:
	movq	96(%rsp), %rsi
	leaq	288(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1552:
	cmpb	$-1, 288(%rsp)
	je	.LBB112_227
	jmp	.LBB112_400
	.p2align	4
.LBB112_251:
	cmpq	$-1, %r12
	je	.LBB112_227
	movq	8(%rsp), %rsi
	cmpq	$-1, 40(%rsi)
	je	.LBB112_227
	movq	8(%r14), %rcx
	movq	16(%r14), %rbx
	movl	296(%rsi), %eax
	testl	%eax, %eax
	je	.LBB112_259
	movq	104(%rsi), %rax
	movq	$-1, %rdi
	.p2align	4
.LBB112_255:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rdi, %rdx
	lock		cmpxchgq	%rdx, 104(%rsi)
	jne	.LBB112_255
	addq	%rcx, %rax
	movq	40(%rsi), %rcx
	cmovbq	%rdi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB112_227
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp1558:
	movq	96(%rsp), %rsi
	leaq	288(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1559:
	jmp	.LBB112_260
.LBB112_258:
	movq	456(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 304(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 288(%rsp)
	cmpb	$-1, 288(%rsp)
	je	.LBB112_227
	jmp	.LBB112_400
.LBB112_259:
	movq	456(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 304(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 288(%rsp)
.LBB112_260:
	movzbl	288(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%rbx, %rbx
	setne	%dl
	testb	%cl, %dl
	jne	.LBB112_387
	cmpb	$-1, %al
	je	.LBB112_227
	jmp	.LBB112_400
.LBB112_262:
	movq	%rcx, %r13
	jmp	.LBB112_227
.LBB112_263:
	movq	%rbx, (%rsp)
.LBB112_264:
	addq	$88, %rbp
	movq	%rbp, 72(%rsp)
	movq	344(%rsp), %r12
.LBB112_265:
	movq	%rbp, 368(%rsp)
.LBB112_266:
	cmpq	$-1, %r12
	je	.LBB112_227
.Ltmp1556:
	leaq	176(%rsp), %rdi
	movq	%r13, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp1557:
	cmpb	$-1, 176(%rsp)
	je	.LBB112_227
	jmp	.LBB112_400
.LBB112_269:
	cmpq	$-1, 344(%rsp)
	je	.LBB112_277
	cmpq	$-1, 40(%r15)
	movq	112(%rsp), %rbp
	je	.LBB112_293
	movl	296(%r15), %eax
	testl	%eax, %eax
	je	.LBB112_290
	movq	104(%r15), %rax
	movq	808(%rsp), %rsi
	movq	$-1, %rdx
	.p2align	4
.LBB112_273:
	movq	%rax, %rcx
	addq	%rsi, %rcx
	cmovbq	%rdx, %rcx
	lock		cmpxchgq	%rcx, 104(%r15)
	jne	.LBB112_273
	movq	40(%r15), %rcx
	addq	%rsi, %rax
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB112_293
	movq	%rcx, 184(%rsp)
	movq	%rax, 192(%rsp)
	movw	$768, 176(%rsp)
.Ltmp1541:
	movq	96(%rsp), %rsi
	leaq	288(%rsp), %rdi
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.1794586459888082020)
.Ltmp1542:
	jmp	.LBB112_291
.LBB112_276:
	movq	112(%rsp), %rbp
	movq	8(%rsp), %r15
	movq	272(%rsp), %rbx
	movq	128(%rsp), %r14
	jmp	.LBB112_312
.LBB112_277:
	movq	112(%rsp), %rbp
	jmp	.LBB112_312
.LBB112_278:
	movq	120(%rsp), %rcx
	jmp	.LBB112_288
.LBB112_279:
	movq	16(%rsp), %rax
	movq	272(%rsp), %rbx
	movq	128(%rsp), %r14
	movq	-16(%rax,%r15), %r12
	cmpq	%r12, (%rsp)
	jae	.LBB112_289
	movq	72(%rsp), %rax
	cmpq	32(%rsp), %rax
	je	.LBB112_322
	movq	8(%rsp), %r15
	movq	72(%rsp), %rcx
	leaq	-1(%r12), %r14
.LBB112_282:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB112_324
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbx
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1535:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1536:
	movq	(%rsp), %rax
	cmpq	%rax, %r14
	je	.LBB112_323
	incq	%rax
	leaq	88(%rbx), %rcx
	movq	%rbx, %rdx
	movq	%rax, (%rsp)
	cmpq	32(%rsp), %rcx
	jne	.LBB112_282
	jmp	.LBB112_324
.LBB112_286:
	movq	280(%rsp), %rax
	movq	%r14, %rcx
	movq	%rax, (%rsp)
.LBB112_287:
	addq	$88, %rcx
	movq	%rcx, 72(%rsp)
	movq	128(%rsp), %r14
.LBB112_288:
	movq	%rcx, 120(%rsp)
	movq	%rcx, 368(%rsp)
	jmp	.LBB112_312
.LBB112_289:
	movq	8(%rsp), %r15
	xorl	%r12d, %r12d
	jmp	.LBB112_200
.LBB112_290:
	movq	456(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 304(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 288(%rsp)
.LBB112_291:
	cmpb	$-1, 288(%rsp)
	je	.LBB112_293
	movb	$1, %r12b
	jmp	.LBB112_300
.LBB112_293:
	testb	%r12b, %r12b
	jne	.LBB112_296
.Ltmp1543:
	leaq	176(%rsp), %rdi
	movq	%r13, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp1544:
	cmpb	$-1, 176(%rsp)
	movb	$1, %r12b
	jne	.LBB112_300
.LBB112_296:
	movq	816(%rsp), %rcx
	testq	%rcx, %rcx
	je	.LBB112_299
.Ltmp1545:
	movq	96(%rsp), %rsi
	movq	<purrdf_sparql_eval::governor::GovernorState>::admit_transient@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movl	$3, %edx
	vzeroupper
	callq	*%rax
.Ltmp1546:
	cmpb	$-1, 176(%rsp)
	setne	%r12b
	jmp	.LBB112_300
.LBB112_299:
	xorl	%r12d, %r12d
.LBB112_300:
	movq	280(%rsp), %rax
	cmpq	%rax, (%rsp)
	jae	.LBB112_311
	movq	72(%rsp), %rax
	cmpq	32(%rsp), %rax
	je	.LBB112_307
	movq	280(%rsp), %rax
	movq	72(%rsp), %rcx
	leaq	-1(%rax), %r14
.LBB112_303:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB112_309
	movq	(%rdx), %rsi
	movq	%rax, 176(%rsp)
	leaq	184(%rsp), %rcx
	movq	%rdx, %rbx
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp1548:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp1549:
	movq	(%rsp), %rax
	cmpq	%rax, %r14
	je	.LBB112_308
	incq	%rax
	leaq	88(%rbx), %rcx
	movq	%rbx, %rdx
	movq	%rax, (%rsp)
	cmpq	32(%rsp), %rcx
	jne	.LBB112_303
	jmp	.LBB112_309
.LBB112_307:
	movq	120(%rsp), %rdx
	jmp	.LBB112_310
.LBB112_308:
	movq	280(%rsp), %rax
	movq	%rbx, %rdx
	movq	%rax, (%rsp)
.LBB112_309:
	addq	$88, %rdx
	movq	%rdx, 72(%rsp)
	movq	272(%rsp), %rbx
	movq	128(%rsp), %r14
.LBB112_310:
	movq	%rdx, 120(%rsp)
	movq	%rdx, 368(%rsp)
.LBB112_311:
	testb	%r12b, %r12b
	jne	.LBB112_394
.LBB112_312:
	cmpq	$0, 24(%rsp)
	je	.LBB112_162
	movq	440(%rsp), %rax
	leaq	184(%rsp), %r12
	movq	%rax, 104(%rsp)
	jmp	.LBB112_315
	.p2align	4
.LBB112_314:
	movq	24(%rsp), %rdi
	movq	152(%rsp), %rcx
	leaq	(%r12,%r12,4), %rax
	addq	$40, %r14
	incq	%r12
	decq	%rdi
	movq	%r15, (%rcx,%rax,8)
	movq	%rbx, 8(%rcx,%rax,8)
	movq	%r13, 16(%rcx,%rax,8)
	movb	%bpl, 24(%rcx,%rax,8)
	movq	%r14, %rbx
	movq	%rdi, 24(%rsp)
	movq	480(%rsp), %rdx
	movq	487(%rsp), %rsi
	movq	48(%rsp), %r13
	movq	112(%rsp), %rbp
	movq	8(%rsp), %r15
	movq	128(%rsp), %r14
	movq	%rdx, 25(%rcx,%rax,8)
	movq	%rsi, 32(%rcx,%rax,8)
	movq	%r12, 160(%rsp)
	leaq	184(%rsp), %r12
	testq	%rdi, %rdi
	je	.LBB112_162
.LBB112_315:
	cmpq	104(%rsp), %rbx
	je	.LBB112_162
	movq	32(%rbx), %rax
	leaq	296(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbx), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%r13, 288(%rsp)
	cmpq	$0, 296(%rsp)
	je	.LBB112_318
	movq	32(%rbx), %rax
	movq	%rbx, %r14
	movq	%rax, 32(%r12)
	vmovdqu	(%rbx), %ymm0
	vmovdqu	%ymm0, (%r12)
	jmp	.LBB112_320
	.p2align	4
.LBB112_318:
	movq	656(%r13), %rdx
	movq	%rbx, %r14
.Ltmp1573:
	movq	64(%rsp), %rsi
	leaq	176(%rsp), %rdi
	leaq	304(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp1574:
	movq	176(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB112_375
.LBB112_320:
	movq	25(%r12), %rax
	movq	32(%r12), %rcx
	movq	184(%rsp), %r15
	movq	192(%rsp), %rbx
	movq	200(%rsp), %r13
	movzbl	208(%rsp), %ebp
	movq	160(%rsp), %r12
	movq	%rax, 480(%rsp)
	movq	%rcx, 487(%rsp)
	cmpq	144(%rsp), %r12
	jne	.LBB112_314
.Ltmp1583:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	144(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp1584:
	jmp	.LBB112_314
.LBB112_322:
	movq	8(%rsp), %r15
	movq	120(%rsp), %rdx
	jmp	.LBB112_325
.LBB112_323:
	movq	%rbx, %rdx
	movq	%r12, (%rsp)
.LBB112_324:
	addq	$88, %rdx
	movq	%rdx, 72(%rsp)
	movq	272(%rsp), %rbx
	movq	128(%rsp), %r14
.LBB112_325:
	xorl	%r12d, %r12d
	movq	%rdx, 120(%rsp)
	movq	%rdx, 368(%rsp)
	jmp	.LBB112_200
.LBB112_326:
	movq	8(%rsp), %r15
.LBB112_327:
	movq	56(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB112_337
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
	jge	.LBB112_330
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB112_330:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_336
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_330
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
.LBB112_333:
	cmpq	%rax, %rcx
	jge	.LBB112_335
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB112_333
.LBB112_335:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_336:
	movq	496(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB112_337:
.Ltmp1593:
	leaq	360(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp1594:
	movq	40(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_348
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
	jge	.LBB112_341
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB112_341:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_347
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_341
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
.LBB112_344:
	cmpq	%rax, %rdx
	jge	.LBB112_346
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB112_344
.LBB112_346:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_347:
	movq	16(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_348:
	movq	664(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_351
	lock		decq	(%rax)
	jne	.LBB112_351
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	664(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB112_351:
	movq	696(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_354
	lock		decq	(%rax)
	jne	.LBB112_354
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	696(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB112_354:
	cmpq	856(%rsp), %r14
	jne	.LBB112_159
.LBB112_355:
	movb	$1, %al
	xorl	%r14d, %r14d
	movl	%eax, 24(%rsp)
.Ltmp1598:
	leaq	824(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1599:
	cmpq	$-1, 352(%rsp)
	movzbl	87(%rsp), %ecx
	sete	%al
	xorb	$1, %cl
	orb	172(%rsp), %cl
	orb	%al, %cl
	cmpb	$1, %cl
	je	.LBB112_358
	movq	$1, 176(%rsp)
	xorl	%r14d, %r14d
	movq	$0, 184(%rsp)
.Ltmp1600:
	movq	<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items@GOTPCREL(%rip), %rax
	movq	96(%rsp), %rsi
	leaq	576(%rsp), %rdi
	leaq	176(%rsp), %rdx
	movl	$1, %ecx
	callq	*%rax
.Ltmp1601:
.LBB112_358:
	movq	144(%rsp), %rax
	movq	152(%rsp), %r12
	movq	160(%rsp), %rbx
	movq	%rax, 64(%rsp)
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB112_360
	xorl	%r14d, %r14d
	#MEMBARRIER
.Ltmp1605:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	xorl	%r15d, %r15d
	callq	*%rax
.Ltmp1606:
.LBB112_360:
	xorl	%r14d, %r14d
.Ltmp1607:
	leaq	416(%rsp), %rdi
	xorl	%r15d, %r15d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1608:
	movl	172(%rsp), %ecx
	movb	$2, %dl
.LBB112_362:
	movq	64(%rsp), %rax
	movq	%rax, 176(%rsp)
	movq	608(%r13), %rax
	movq	%r12, 184(%rsp)
	movq	%rbx, 192(%rsp)
	testq	%rax, %rax
	je	.LBB112_368
	movl	296(%rax), %ecx
	movb	$-1, %bl
	testl	%ecx, %ecx
	jne	.LBB112_365
	movq	288(%rax), %rcx
	movzbl	272(%rax), %ebx
	movq	%rcx, 527(%rsp)
	vmovdqu	273(%rax), %xmm0
	vmovdqa	%xmm0, 512(%rsp)
.LBB112_365:
	testb	%dl, %dl
	je	.LBB112_372
	movzbl	%dl, %eax
	cmpl	$2, %eax
	jne	.LBB112_369
	movl	172(%rsp), %ecx
	jmp	.LBB112_373
.LBB112_368:
	cmpb	$2, %dl
	movb	$-1, %bl
	sete	%al
	andb	%cl, %al
	movl	%eax, %ecx
	jmp	.LBB112_373
.LBB112_369:
	cmpb	$-1, %bl
	je	.LBB112_376
	cmpb	$2, 472(%r13)
	jne	.LBB112_372
	vmovdqa	512(%rsp), %xmm0
	movq	527(%rsp), %rax
	movq	688(%r13), %rdi
	movb	%bl, 576(%rsp)
	vmovdqu	%xmm0, 577(%rsp)
	movq	%rax, 592(%rsp)
	movl	40(%rdi), %eax
	testl	%eax, %eax
	je	.LBB112_372
	addq	$16, %rdi
.Ltmp1636:
	leaq	576(%rsp), %rsi
	callq	<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.1794586459888082020)
.Ltmp1637:
	jmp	.LBB112_372
.LBB112_375:
	vmovups	224(%rsp), %ymm0
	movq	%rax, (%rsp)
	movq	184(%rsp), %rax
	movq	25(%r12), %rsi
	movq	32(%r12), %rcx
	movq	192(%rsp), %rdx
	movq	200(%rsp), %rbx
	addq	$40, %r14
	movq	%r14, 424(%rsp)
	movb	$1, %r14b
	movq	%rax, 64(%rsp)
	movzbl	208(%rsp), %eax
	movq	%rsi, 480(%rsp)
	movq	%rcx, 487(%rsp)
	movq	%rdx, %r12
	movq	487(%rsp), %rcx
	vmovups	%ymm0, 527(%rsp)
	vmovdqu	240(%rsp), %ymm0
	movl	%eax, 32(%rsp)
	movq	480(%rsp), %rax
	movq	8(%rsp), %r15
	vmovdqu	%ymm0, 543(%rsp)
	movq	%rax, 512(%rsp)
	movq	%rcx, 519(%rsp)
	jmp	.LBB112_401
.LBB112_376:
	movb	$-1, %bl
.LBB112_372:
	xorl	%ecx, %ecx
.LBB112_373:
	vmovups	176(%rsp), %xmm0
	cmpb	$-1, %bl
	movq	400(%rsp), %rdx
	sete	%al
	andb	%cl, %al
	movq	192(%rsp), %rcx
	movzbl	%al, %eax
	movq	%rcx, 24(%rdx)
	vmovups	%xmm0, 8(%rdx)
	movq	%rax, 32(%rdx)
	movq	%rbp, 40(%rdx)
	movq	$-1, (%rdx)
.LBB112_374:
	addq	$968, %rsp
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
.LBB112_377:
	.cfi_def_cfa_offset 1024
	vmovups	624(%rsp), %ymm0
	movq	%rax, (%rsp)
	movq	584(%rsp), %rax
	movq	25(%rcx), %rdx
	movq	32(%rcx), %rcx
	movq	592(%rsp), %r12
	movq	600(%rsp), %rbx
	addq	$40, %rbp
	movb	$1, %r14b
	movq	%rbp, 296(%rsp)
	movq	%rax, 64(%rsp)
	movzbl	608(%rsp), %eax
	movq	%rdx, 464(%rsp)
	movq	%rcx, 471(%rsp)
	movq	471(%rsp), %rcx
	vmovups	%ymm0, 527(%rsp)
	vmovdqu	640(%rsp), %ymm0
	movl	%eax, 32(%rsp)
	movq	464(%rsp), %rax
	vmovdqu	%ymm0, 543(%rsp)
	movq	%rax, 512(%rsp)
	movq	%rcx, 519(%rsp)
.Ltmp1612:
	leaq	288(%rsp), %rdi
	movb	$1, %r15b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1613:
	movq	112(%rsp), %rbp
	movb	$1, %r14b
	movb	$1, %r15b
	jmp	.LBB112_432
.LBB112_379:
	movq	104(%rsp), %r12
	movq	48(%rsp), %r13
	movq	%r12, %rax
	addq	$-1, %rax
	jae	.LBB112_400
	cmpq	%rax, (%rsp)
	jae	.LBB112_400
	movq	72(%rsp), %rax
	leaq	184(%rsp), %r14
	cmpq	32(%rsp), %rax
	je	.LBB112_399
	subq	(%rsp), %r12
	addq	$88, %rax
	leaq	176(%rsp), %rbx
	addq	$-2, %r12
.LBB112_383:
	movq	%rax, %rbp
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB112_399
	movq	-88(%rbp), %rsi
	movq	%rax, 176(%rsp)
	movq	%rbp, %r15
	movq	-8(%rbp), %rax
	movq	%rax, 64(%r14)
	vmovdqu64	-72(%rbp), %zmm0
	vmovdqu64	%zmm0, (%r14)
.Ltmp1567:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp1568:
	subq	$1, %r12
	jb	.LBB112_397
	leaq	88(%r15), %rax
	movq	%r15, %rbp
	cmpq	32(%rsp), %r15
	jne	.LBB112_383
	jmp	.LBB112_399
.LBB112_387:
	leaq	-1(%rbx), %rax
	cmpq	%rax, (%rsp)
	jae	.LBB112_400
	movq	72(%rsp), %rax
	leaq	184(%rsp), %r14
	cmpq	32(%rsp), %rax
	je	.LBB112_399
	subq	(%rsp), %rbx
	addq	$88, %rax
	leaq	176(%rsp), %r15
	addq	$-2, %rbx
.LBB112_390:
	movq	%rax, %rbp
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB112_399
	movq	-88(%rbp), %rsi
	movq	%rax, 176(%rsp)
	movq	%rbp, %r12
	movq	-8(%rbp), %rax
	movq	%rax, 64(%r14)
	vmovdqu64	-72(%rbp), %zmm0
	vmovdqu64	%zmm0, (%r14)
.Ltmp1561:
	movq	64(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r15, %rdx
	vzeroupper
	callq	*%rax
.Ltmp1562:
	subq	$1, %rbx
	jb	.LBB112_398
	leaq	88(%r12), %rax
	movq	%r12, %rbp
	cmpq	32(%rsp), %r12
	jne	.LBB112_390
	jmp	.LBB112_399
.LBB112_394:
	movq	144(%rsp), %rax
	movq	152(%rsp), %r12
	movq	160(%rsp), %rbx
	movq	%rax, 64(%rsp)
	movb	$1, %al
	movq	$-1, (%rsp)
	movl	%eax, 32(%rsp)
	jmp	.LBB112_396
.LBB112_395:
	movq	144(%rsp), %rax
	movq	152(%rsp), %r12
	movq	160(%rsp), %rbx
	movq	%rax, 64(%rsp)
	movq	$-1, (%rsp)
	movl	$0, 32(%rsp)
.LBB112_396:
	xorl	%r14d, %r14d
	jmp	.LBB112_401
.LBB112_397:
	movq	%r15, %rbp
	jmp	.LBB112_399
.LBB112_398:
	movq	%r12, %rbp
.LBB112_399:
	movq	%rbp, 368(%rsp)
.LBB112_400:
	movq	144(%rsp), %rax
	movq	152(%rsp), %r12
	movq	160(%rsp), %rbx
	xorl	%r14d, %r14d
	movq	%rax, 64(%rsp)
	movb	$1, %al
	movq	$-1, (%rsp)
	movl	%eax, 32(%rsp)
	movq	112(%rsp), %rbp
	movq	8(%rsp), %r15
.LBB112_401:
	movq	56(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB112_411
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
	jge	.LBB112_404
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB112_404:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_410
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_404
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
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	.p2align	4
.LBB112_407:
	cmpq	%rax, %rcx
	jge	.LBB112_409
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB112_407
.LBB112_409:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_410:
	movq	496(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB112_411:
.Ltmp1576:
	leaq	360(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp1577:
	movq	40(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_422
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
	jge	.LBB112_415
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB112_415:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_421
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_415
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
.LBB112_418:
	cmpq	%rax, %rdx
	jge	.LBB112_420
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB112_418
.LBB112_420:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_421:
	movq	16(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_422:
	movq	664(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_425
	lock		decq	(%rax)
	jne	.LBB112_425
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	664(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB112_425:
	movq	696(%rsp), %rax
	movl	%r14d, 24(%rsp)
	testq	%rax, %rax
	je	.LBB112_428
	lock		decq	(%rax)
	jne	.LBB112_428
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	696(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB112_428:
	xorl	%r14d, %r14d
.Ltmp1579:
	leaq	824(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1580:
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	movl	24(%rsp), %r14d
	jne	.LBB112_431
	xorl	%r15d, %r15d
	#MEMBARRIER
.Ltmp1581:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	callq	*%rax
.Ltmp1582:
.LBB112_431:
	xorl	%r15d, %r15d
.LBB112_432:
	cmpq	$0, 800(%rsp)
	je	.LBB112_434
.Ltmp1614:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1615:
.LBB112_434:
	movq	%rbx, 48(%rsp)
	testb	%r14b, %r14b
	je	.LBB112_450
	movq	152(%rsp), %rbx
	movq	160(%rsp), %r14
	movq	%r12, 8(%rsp)
	movl	%r15d, 24(%rsp)
	testq	%r14, %r14
	je	.LBB112_448
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r15
	movabsq	$9223372036854775807, %r12
	xorl	%ebp, %ebp
	jmp	.LBB112_440
.LBB112_437:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB112_438:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_439:
	incq	%rbp
	cmpq	%r14, %rbp
	je	.LBB112_448
.LBB112_440:
	leaq	(%rbp,%rbp,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB112_439
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
	jge	.LBB112_443
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB112_443:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB112_438
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB112_443
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
	movq	(%r15), %rax
	.p2align	4
.LBB112_446:
	cmpq	%rax, %rdx
	jge	.LBB112_437
	lock		cmpxchgq	%rdx, (%r15)
	jne	.LBB112_446
	jmp	.LBB112_437
.LBB112_448:
	movq	144(%rsp), %rax
	movq	112(%rsp), %rbp
	movl	24(%rsp), %r15d
	movq	8(%rsp), %r12
	testq	%rax, %rax
	je	.LBB112_450
	shlq	$3, %rax
	movl	$8, %edx
	movq	%rbx, %rdi
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
.LBB112_450:
	movq	48(%rsp), %rbx
	testb	%r15b, %r15b
	jne	.LBB112_84
	jmp	.LBB112_99
.LBB112_451:
.Ltmp1586:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.289(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp1587:
	jmp	.LBB112_453
.LBB112_452:
	leaq	904(%rsp), %rax
	leaq	288(%rsp), %rcx
	movq	%rdi, 904(%rsp)
	movq	%rdx, 288(%rsp)
	movq	%rax, 176(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 184(%rsp)
	movq	%rcx, 192(%rsp)
	movq	%rax, 200(%rsp)
.Ltmp1588:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	movq	16(%rsp), %r12
	movq	40(%rsp), %r13
	movq	56(%rsp), %rbx
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.2156(%rip), %rdi
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.288(%rip), %rdx
	leaq	176(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp1589:
.LBB112_453:
	ud2
.LBB112_454:
	movb	$1, %al
	movl	%eax, 24(%rsp)
.Ltmp1525:
	movl	$8, %ecx
	movl	$80, %r8d
	movq	%rdx, %rdi
	movq	%r12, %rdx
	movb	$1, %r14b
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.1794586459888082020)
	leaq	904(%r13), %rdx
.Ltmp1526:
	jmp	.LBB112_156
.LBB112_455:
	movb	$1, %al
	leaq	992(%r13), %rdi
	movl	%eax, 24(%rsp)
.Ltmp1527:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%r12, %rsi
	movb	$1, %r14b
	vzeroupper
	callq	*%rax
.Ltmp1528:
	jmp	.LBB112_157
.LBB112_457:
.Ltmp1638:
	leaq	176(%rsp), %rdi
	movq	%rax, %rbp
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%rbp, %rdi
	callq	_Unwind_Resume@PLT
.LBB112_458:
.Ltmp1563:
	movq	8(%rsp), %r15
	movq	%rax, %rbp
	movq	%r12, 368(%rsp)
	jmp	.LBB112_509
.LBB112_459:
.Ltmp1569:
	movq	%r15, 368(%rsp)
	jmp	.LBB112_507
.LBB112_460:
.Ltmp1540:
	jmp	.LBB112_508
.LBB112_461:
.Ltmp1595:
	movq	%rax, %rbp
	movb	$1, %r14b
	jmp	.LBB112_464
.LBB112_462:
.Ltmp1547:
	jmp	.LBB112_508
.LBB112_463:
.Ltmp1578:
	movq	%rax, %rbp
.LBB112_464:
	movq	16(%rsp), %r12
	movq	40(%rsp), %r13
	jmp	.LBB112_521
.LBB112_465:
.Ltmp1531:
	jmp	.LBB112_508
.LBB112_467:
.Ltmp1537:
	jmp	.LBB112_473
.LBB112_468:
.Ltmp1616:
	movq	%rax, %rbp
	jmp	.LBB112_535
.LBB112_469:
.Ltmp1624:
	cmpq	$0, 800(%rsp)
	movq	%rax, %rbp
	jne	.LBB112_534
	jmp	.LBB112_535
.LBB112_470:
.Ltmp1602:
	movq	8(%rsp), %r15
	movq	%rax, %rbp
	jmp	.LBB112_531
.LBB112_471:
.Ltmp1566:
	jmp	.LBB112_507
.LBB112_472:
.Ltmp1550:
.LBB112_473:
	addq	$88, %rbx
	movq	%rax, %rbp
	movq	%rbx, 368(%rsp)
	jmp	.LBB112_509
.LBB112_474:
.Ltmp1524:
	movq	%rax, %rbp
	jmp	.LBB112_538
.LBB112_475:
.Ltmp1611:
	addq	$40, %rbp
	movq	%rax, %rbx
	movq	%rbp, 296(%rsp)
	jmp	.LBB112_482
.LBB112_476:
.Ltmp1660:
	movq	%rax, %rbp
	jmp	.LBB112_502
.LBB112_477:
.Ltmp1519:
	movq	%rax, %rbp
.Ltmp1520:
	movq	24(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1521:
	jmp	.LBB112_538
.LBB112_478:
.Ltmp1511:
	movq	%rax, %rbp
	jmp	.LBB112_493
.LBB112_479:
.Ltmp1500:
	movq	%rax, %rbp
	jmp	.LBB112_494
.LBB112_480:
.Ltmp1619:
	addq	$40, %rbp
	movq	%rax, %rbx
	movq	%rbp, 296(%rsp)
	cmpq	$6, %r12
	jb	.LBB112_482
	movq	(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB112_482:
	movb	$1, %r15b
.Ltmp1620:
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1621:
	movq	%rbx, %rbp
	jmp	.LBB112_536
.LBB112_483:
.Ltmp1560:
	jmp	.LBB112_507
.LBB112_484:
.Ltmp1572:
	addq	$88, %r14
	movq	%rax, %rbp
	movq	%r14, 368(%rsp)
	jmp	.LBB112_509
.LBB112_485:
.Ltmp1646:
	movq	%rax, %rbp
	jmp	.LBB112_503
.LBB112_486:
.Ltmp1575:
	movq	8(%rsp), %r15
	addq	$40, %r14
	movq	%rax, %rbp
	movq	%r14, 424(%rsp)
	jmp	.LBB112_509
.LBB112_487:
.Ltmp1641:
	movq	%rax, %rbp
.Ltmp1642:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1643:
	jmp	.LBB112_503
.LBB112_488:
.Ltmp1585:
	addq	$40, %r14
	movq	%rax, %rbp
	movq	%r14, 424(%rsp)
	cmpq	$6, %r15
	jb	.LBB112_490
	leaq	-8(,%r15,8), %rsi
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB112_490:
	movq	8(%rsp), %r15
	jmp	.LBB112_509
.LBB112_491:
.Ltmp1503:
	movq	%rax, %rbp
	movq	%r15, 184(%rsp)
.Ltmp1504:
	leaq	576(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp1505:
.Ltmp1507:
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
.Ltmp1508:
.LBB112_493:
.Ltmp1512:
	leaq	512(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1513:
.LBB112_494:
.Ltmp1515:
	movq	24(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1516:
	jmp	.LBB112_539
.LBB112_495:
.Ltmp1506:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_496:
.Ltmp1514:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_497:
.Ltmp1555:
	movq	8(%rsp), %r15
	addq	$88, %rbp
	movq	%rbp, 368(%rsp)
	jmp	.LBB112_508
.LBB112_498:
.Ltmp1649:
	movq	%rax, %rbp
	movq	%r15, 296(%rsp)
	jmp	.LBB112_501
.LBB112_499:
.Ltmp1654:
	movq	%rax, %rbp
	movq	%r15, 296(%rsp)
	cmpq	$6, %r12
	jb	.LBB112_501
	movq	(%rsp), %rdi
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB112_501:
.Ltmp1655:
	leaq	288(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1656:
.LBB112_502:
	leaq	360(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB112_503:
.Ltmp1661:
	movq	24(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
.Ltmp1662:
	jmp	.LBB112_539
.LBB112_504:
.Ltmp1657:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_505:
.Ltmp1663:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_506:
.Ltmp1534:
.LBB112_507:
	movq	8(%rsp), %r15
.LBB112_508:
	movq	%rax, %rbp
.LBB112_509:
	movq	16(%rsp), %r12
	movq	40(%rsp), %r13
	movq	56(%rsp), %rbx
	jmp	.LBB112_518
.LBB112_510:
.Ltmp1632:
	movq	%rax, %rbp
	testq	%r15, %r15
	je	.LBB112_514
	negq	%r15
	addq	$160, %r14
	.p2align	4
.LBB112_512:
.Ltmp1633:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
.Ltmp1634:
	addq	$160, %r14
	decq	%r15
	jne	.LBB112_512
.LBB112_514:
	cmpq	$0, 408(%rsp)
	je	.LBB112_539
	movq	408(%rsp), %rax
	movq	136(%rsp), %rdi
	movl	$8, %edx
	shlq	$5, %rax
	leaq	(%rax,%rax,4), %rsi
	callq	__rustc::__rust_dealloc
	movq	%rbp, %rdi
	callq	_Unwind_Resume@PLT
.LBB112_516:
.Ltmp1635:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB112_517:
.Ltmp1590:
	movq	%rax, %rbp
.LBB112_518:
	testq	%rbx, %rbx
	je	.LBB112_520
	movq	496(%rsp), %rdi
	shlq	$5, %rbx
	movl	$8, %edx
	movq	%rbx, %rsi
	callq	__rustc::__rust_dealloc
.LBB112_520:
	movb	$1, %r14b
.Ltmp1591:
	leaq	360(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp1592:
.LBB112_521:
	testq	%r13, %r13
	je	.LBB112_523
	shlq	$3, %r13
	movl	$8, %edx
	movq	%r12, %rdi
	leaq	(%r13,%r13,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB112_523:
	movq	664(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_526
	lock		decq	(%rax)
	jne	.LBB112_526
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	664(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB112_526:
	movq	696(%rsp), %rax
	testq	%rax, %rax
	je	.LBB112_529
	lock		decq	(%rax)
	jne	.LBB112_529
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	696(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB112_529:
.Ltmp1596:
	leaq	824(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1597:
	movl	%r14d, 24(%rsp)
	xorl	%r14d, %r14d
.LBB112_531:
	lock		decq	(%r15)
	jne	.LBB112_533
	#MEMBARRIER
.Ltmp1603:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	504(%rsp), %rdi
	callq	*%rax
.Ltmp1604:
	movl	%r14d, %r15d
	movl	24(%rsp), %r14d
	jmp	.LBB112_534
.LBB112_533:
	movl	%r14d, %r15d
	movl	24(%rsp), %r14d
.LBB112_534:
.Ltmp1625:
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp1626:
.LBB112_535:
	testb	%r14b, %r14b
	je	.LBB112_537
.LBB112_536:
	leaq	144(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB112_537:
	testb	%r15b, %r15b
	je	.LBB112_539
.LBB112_538:
.Ltmp1627:
	leaq	936(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp1628:
.LBB112_539:
	movq	%rbp, %rdi
	callq	_Unwind_Resume@PLT
.LBB112_540:
.Ltmp1629:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end112:

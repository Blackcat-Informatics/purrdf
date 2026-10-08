purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>:
.Lfunc_begin189:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception115
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
	movq	16(%rdx), %r12
	movq	%rdx, %r15
	movq	%rsi, %r14
	movq	%rdi, %rbx
.Ltmp4309:
	leaq	160(%rsp), %rdi
	movq	%rcx, %rsi
	movq	%r12, %rdx
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp4310:
	vmovups	168(%rsp), %xmm0
	movq	184(%rsp), %rcx
	movq	160(%rsp), %rax
	movq	%rcx, 128(%rsp)
	vmovaps	%xmm0, 112(%rsp)
	cmpq	$-1, %rax
	je	.LBB189_2
	vmovups	192(%rsp), %zmm0
	vmovaps	112(%rsp), %xmm1
	movq	128(%rsp), %rcx
	movq	%r15, %rdi
	vmovups	%zmm0, 32(%rbx)
	movq	%rcx, 24(%rbx)
	vmovups	%xmm1, 8(%rbx)
	movq	%rax, (%rbx)
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
	jmp	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.LBB189_2:
	.cfi_def_cfa_offset 320
	vmovaps	112(%rsp), %xmm0
	movq	8(%r15), %r13
	movq	128(%rsp), %rax
	leaq	(,%r12,8), %rcx
	movq	(%r15), %rdx
	movq	%rbx, 24(%rsp)
	leaq	(%rcx,%rcx,4), %rbp
	leaq	(%r13,%rbp), %rcx
	movq	%r13, 56(%rsp)
	movq	%rdx, 72(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rcx, 80(%rsp)
	vmovaps	%xmm0, (%rsp)
	testq	%r12, %r12
	je	.LBB189_41
	leaq	168(%rsp), %r12
	addq	$40, %r13
	leaq	888(%r14), %rax
	movq	%rcx, 88(%rsp)
	movq	%rax, 96(%rsp)
	movq	%r14, 104(%rsp)
	jmp	.LBB189_4
	.p2align	4
.LBB189_39:
	movq	8(%rsp), %rax
	leaq	(%r14,%r14,4), %rcx
	incq	%r14
	addq	$40, %r13
	addq	$-40, %rbp
	movq	%rbx, (%rax,%rcx,8)
	movq	%r15, 8(%rax,%rcx,8)
	vmovaps	32(%rsp), %xmm0
	vmovups	%xmm0, 16(%rax,%rcx,8)
	movq	48(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r14, 16(%rsp)
	movq	104(%rsp), %r14
	je	.LBB189_40
.LBB189_4:
	movq	-8(%r13), %rax
	leaq	120(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovups	-40(%r13), %ymm0
	vmovups	%ymm0, (%rcx)
	movq	%r14, 112(%rsp)
	cmpq	$0, 120(%rsp)
	je	.LBB189_5
	leaq	-40(%r13), %rax
	movq	32(%rax), %rcx
	movq	%rcx, 32(%r12)
	vmovups	(%rax), %ymm0
	vmovups	%ymm0, (%r12)
	jmp	.LBB189_37
	.p2align	4
.LBB189_5:
	movq	664(%r14), %rdx
.Ltmp4314:
	movq	96(%rsp), %rsi
	leaq	160(%rsp), %rdi
	leaq	128(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp4315:
	movq	160(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB189_7
.LBB189_37:
	vmovups	16(%r12), %xmm0
	movq	32(%r12), %rax
	movq	168(%rsp), %rbx
	movq	176(%rsp), %r15
	movq	16(%rsp), %r14
	movq	%rax, 48(%rsp)
	vmovaps	%xmm0, 32(%rsp)
	cmpq	(%rsp), %r14
	jne	.LBB189_39
.Ltmp4319:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp4320:
	jmp	.LBB189_39
.LBB189_40:
	movq	88(%rsp), %r13
.LBB189_41:
	movq	%r13, 64(%rsp)
.Ltmp4325:
	leaq	56(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4326:
	vmovaps	(%rsp), %xmm0
	movq	24(%rsp), %rcx
	movq	16(%rsp), %rax
	movq	%rax, 24(%rcx)
	vmovups	%xmm0, 8(%rcx)
	movq	$-1, (%rcx)
.LBB189_43:
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
	retq
.LBB189_7:
	.cfi_def_cfa_offset 320
	vmovups	16(%r12), %xmm1
	movq	32(%r12), %rcx
	vmovups	168(%rsp), %xmm0
	vmovups	224(%rsp), %ymm4
	vmovups	208(%rsp), %ymm3
	movq	24(%rsp), %rdx
	movq	%r13, 64(%rsp)
	movq	%rcx, 48(%rsp)
	movq	48(%rsp), %rcx
	vmovaps	%xmm1, 32(%rsp)
	vmovups	%ymm4, 64(%rdx)
	vmovups	%ymm3, 48(%rdx)
	vmovaps	32(%rsp), %xmm2
	movq	%rcx, 40(%rdx)
	vmovups	%xmm2, 24(%rdx)
	movq	%rax, (%rdx)
	vmovups	%xmm0, 8(%rdx)
.Ltmp4317:
	leaq	56(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4318:
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r15
	movabsq	$9223372036854775807, %r14
	testq	%r15, %r15
	je	.LBB189_21
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	xorl	%r12d, %r12d
	jmp	.LBB189_10
	.p2align	4
.LBB189_18:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB189_19:
	callq	*%r13
.LBB189_20:
	incq	%r12
	cmpq	%r15, %r12
	je	.LBB189_21
.LBB189_10:
	leaq	(%r12,%r12,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB189_20
	leaq	(%rbx,%rcx,8), %rdx
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
	jge	.LBB189_13
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB189_13:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB189_19
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB189_13
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
	movq	(%rbp), %rax
	.p2align	4
.LBB189_16:
	cmpq	%rax, %rdx
	jge	.LBB189_18
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB189_16
	jmp	.LBB189_18
.LBB189_21:
	movq	(%rsp), %rax
	testq	%rax, %rax
	je	.LBB189_43
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r14, %rcx
	cmovaeq	%r14, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r14, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB189_24
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB189_24:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB189_30
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB189_24
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r14, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB189_27:
	cmpq	%rax, %rdx
	jge	.LBB189_29
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB189_27
.LBB189_29:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB189_30:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB189_43
.LBB189_35:
.Ltmp4327:
	movq	%rax, %r14
	jmp	.LBB189_33
.LBB189_47:
.Ltmp4311:
	movq	%rax, %r14
.Ltmp4312:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4313:
	jmp	.LBB189_34
.LBB189_31:
.Ltmp4316:
	movq	%rax, %r14
	movq	%r13, 64(%rsp)
	jmp	.LBB189_32
.LBB189_44:
.Ltmp4321:
	movq	%rax, %r14
	movq	%r13, 64(%rsp)
	cmpq	$6, %rbx
	jb	.LBB189_32
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r15, %rdi
	callq	__rustc::__rust_dealloc
.LBB189_32:
.Ltmp4322:
	leaq	56(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4323:
.LBB189_33:
	movq	%rsp, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB189_34:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB189_46:
.Ltmp4324:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end189:
purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, &mut purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>:
.Lfunc_begin190:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception116
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
	movq	16(%rdx), %r12
	movq	%rdx, %r15
	movq	%rsi, %r14
	movq	%rdi, %rbx
.Ltmp4328:
	leaq	160(%rsp), %rdi
	movq	%rcx, %rsi
	movq	%r12, %rdx
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp4329:
	vmovups	168(%rsp), %xmm0
	movq	184(%rsp), %rcx
	movq	160(%rsp), %rax
	movq	%rcx, 128(%rsp)
	vmovaps	%xmm0, 112(%rsp)
	cmpq	$-1, %rax
	je	.LBB190_2
	vmovups	192(%rsp), %zmm0
	vmovaps	112(%rsp), %xmm1
	movq	128(%rsp), %rcx
	movq	%r15, %rdi
	vmovups	%zmm0, 32(%rbx)
	movq	%rcx, 24(%rbx)
	vmovups	%xmm1, 8(%rbx)
	movq	%rax, (%rbx)
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
	jmp	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.LBB190_2:
	.cfi_def_cfa_offset 320
	vmovaps	112(%rsp), %xmm0
	movq	8(%r15), %r13
	movq	128(%rsp), %rax
	leaq	(,%r12,8), %rcx
	movq	(%r15), %rdx
	movq	%rbx, 24(%rsp)
	leaq	(%rcx,%rcx,4), %rbp
	leaq	(%r13,%rbp), %rcx
	movq	%r13, 56(%rsp)
	movq	%rdx, 72(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rcx, 80(%rsp)
	vmovaps	%xmm0, (%rsp)
	testq	%r12, %r12
	je	.LBB190_41
	leaq	168(%rsp), %r12
	addq	$40, %r13
	leaq	888(%r14), %rax
	movq	%rcx, 88(%rsp)
	movq	%rax, 96(%rsp)
	movq	%r14, 104(%rsp)
	jmp	.LBB190_4
	.p2align	4
.LBB190_39:
	movq	8(%rsp), %rax
	leaq	(%r14,%r14,4), %rcx
	incq	%r14
	addq	$40, %r13
	addq	$-40, %rbp
	movq	%rbx, (%rax,%rcx,8)
	movq	%r15, 8(%rax,%rcx,8)
	vmovaps	32(%rsp), %xmm0
	vmovups	%xmm0, 16(%rax,%rcx,8)
	movq	48(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r14, 16(%rsp)
	movq	104(%rsp), %r14
	je	.LBB190_40
.LBB190_4:
	movq	%r14, 112(%rsp)
	leaq	120(%rsp), %rcx
	movq	-8(%r13), %rax
	movq	%rax, 32(%rcx)
	vmovups	-40(%r13), %ymm0
	vmovups	%ymm0, (%rcx)
	cmpq	$0, 120(%rsp)
	je	.LBB190_5
	leaq	-40(%r13), %rax
	movq	32(%rax), %rcx
	movq	%rcx, 32(%r12)
	vmovups	(%rax), %ymm0
	vmovups	%ymm0, (%r12)
	jmp	.LBB190_37
	.p2align	4
.LBB190_5:
	movq	664(%r14), %rdx
.Ltmp4333:
	movq	96(%rsp), %rsi
	leaq	160(%rsp), %rdi
	leaq	128(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp4334:
	movq	160(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB190_7
.LBB190_37:
	vmovups	16(%r12), %xmm0
	movq	32(%r12), %rax
	movq	168(%rsp), %rbx
	movq	176(%rsp), %r15
	movq	16(%rsp), %r14
	movq	%rax, 48(%rsp)
	vmovaps	%xmm0, 32(%rsp)
	cmpq	(%rsp), %r14
	jne	.LBB190_39
.Ltmp4338:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	movq	%rsp, %rdi
	vzeroupper
	callq	*%rax
.Ltmp4339:
	jmp	.LBB190_39
.LBB190_40:
	movq	88(%rsp), %r13
.LBB190_41:
	movq	%r13, 64(%rsp)
.Ltmp4344:
	leaq	56(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4345:
	vmovaps	(%rsp), %xmm0
	movq	24(%rsp), %rcx
	movq	16(%rsp), %rax
	movq	%rax, 24(%rcx)
	vmovups	%xmm0, 8(%rcx)
	movq	$-1, (%rcx)
.LBB190_43:
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
	retq
.LBB190_7:
	.cfi_def_cfa_offset 320
	vmovups	16(%r12), %xmm1
	movq	32(%r12), %rcx
	vmovups	168(%rsp), %xmm0
	vmovups	224(%rsp), %ymm4
	vmovups	208(%rsp), %ymm3
	movq	24(%rsp), %rdx
	movq	%r13, 64(%rsp)
	movq	%rcx, 48(%rsp)
	movq	48(%rsp), %rcx
	vmovaps	%xmm1, 32(%rsp)
	vmovups	%ymm4, 64(%rdx)
	vmovups	%ymm3, 48(%rdx)
	vmovaps	32(%rsp), %xmm2
	movq	%rcx, 40(%rdx)
	vmovups	%xmm2, 24(%rdx)
	movq	%rax, (%rdx)
	vmovups	%xmm0, 8(%rdx)
.Ltmp4336:
	leaq	56(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4337:
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r15
	movabsq	$9223372036854775807, %r14
	testq	%r15, %r15
	je	.LBB190_21
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r13
	xorl	%r12d, %r12d
	jmp	.LBB190_10
	.p2align	4
.LBB190_18:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB190_19:
	callq	*%r13
.LBB190_20:
	incq	%r12
	cmpq	%r15, %r12
	je	.LBB190_21
.LBB190_10:
	leaq	(%r12,%r12,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB190_20
	leaq	(%rbx,%rcx,8), %rdx
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
	jge	.LBB190_13
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB190_13:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB190_19
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB190_13
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
	movq	(%rbp), %rax
	.p2align	4
.LBB190_16:
	cmpq	%rax, %rdx
	jge	.LBB190_18
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB190_16
	jmp	.LBB190_18
.LBB190_21:
	movq	(%rsp), %rax
	testq	%rax, %rax
	je	.LBB190_43
	shlq	$3, %rax
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r14, %rcx
	cmovaeq	%r14, %rcx
	xorl	%edx, %edx
	cmpq	%rcx, %rax
	setns	%dl
	addq	%r14, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB190_24
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB190_24:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB190_30
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB190_24
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%r14, %rax
	subq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB190_27:
	cmpq	%rax, %rdx
	jge	.LBB190_29
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB190_27
.LBB190_29:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB190_30:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	jmp	.LBB190_43
.LBB190_35:
.Ltmp4346:
	movq	%rax, %r14
	jmp	.LBB190_33
.LBB190_47:
.Ltmp4330:
	movq	%rax, %r14
.Ltmp4331:
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4332:
	jmp	.LBB190_34
.LBB190_31:
.Ltmp4335:
	movq	%rax, %r14
	movq	%r13, 64(%rsp)
	jmp	.LBB190_32
.LBB190_44:
.Ltmp4340:
	movq	%rax, %r14
	movq	%r13, 64(%rsp)
	cmpq	$6, %rbx
	jb	.LBB190_32
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r15, %rdi
	callq	__rustc::__rust_dealloc
.LBB190_32:
.Ltmp4341:
	leaq	56(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4342:
.LBB190_33:
	movq	%rsp, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB190_34:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB190_46:
.Ltmp4343:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end190:
purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>:
.Lfunc_begin192:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception118
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
	subq	$792, %rsp
	.cfi_def_cfa_offset 848
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	16(%r8), %r15
	movq	$0, 208(%rsp)
	movq	$8, 216(%rsp)
	movq	%r9, %r13
	movq	%r8, %r14
	movq	%rcx, 312(%rsp)
	movl	%edx, %ebx
	movq	%rsi, %r12
	movq	%rdi, 88(%rsp)
	movq	$0, 224(%rsp)
.Ltmp4437:
	leaq	528(%rsp), %rdi
	leaq	208(%rsp), %rsi
	movq	%r15, %rdx
	callq	purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
.Ltmp4438:
	vmovups	536(%rsp), %xmm0
	movq	552(%rsp), %rcx
	movq	528(%rsp), %rax
	movq	%rcx, 144(%rsp)
	vmovaps	%xmm0, 128(%rsp)
	cmpq	$-1, %rax
	je	.LBB192_5
	vmovdqu64	560(%rsp), %zmm0
	vmovdqa	128(%rsp), %xmm1
	movq	88(%rsp), %rdx
	movq	144(%rsp), %rcx
	vmovdqu64	%zmm0, 32(%rdx)
	movq	%rcx, 24(%rdx)
	vmovdqu	%xmm1, 8(%rdx)
	movq	%rax, (%rdx)
.Ltmp4442:
	movq	%r14, %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4443:
.LBB192_3:
	movq	312(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.LBB192_4:
	addq	$792, %rsp
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
.LBB192_5:
	.cfi_def_cfa_offset 848
	movq	8(%r14), %rcx
	movq	(%r14), %rsi
	leaq	(%r15,%r15,4), %rdx
	vmovdqa	128(%rsp), %xmm0
	movq	144(%rsp), %rax
	movq	%r12, 48(%rsp)
	leaq	(%rcx,%rdx,8), %rdx
	movq	%rcx, 376(%rsp)
	movq	%rsi, 392(%rsp)
	movq	%rcx, 384(%rsp)
	movq	%rax, 112(%rsp)
	movq	%rdx, 400(%rsp)
	movq	616(%r12), %rdx
	vmovdqa	%xmm0, 96(%rsp)
	movq	%rdx, 440(%rsp)
	testq	%rdx, %rdx
	je	.LBB192_11
	movq	%r13, 696(%rsp)
	lock		incq	(%rdx)
	jle	.LBB192_332
	movq	616(%r12), %rax
	movq	312(%rsp), %rdx
	movq	%rax, 432(%rsp)
	movq	8(%rdx), %rsi
	movq	16(%rdx), %r13
	movq	%rax, 8(%rsp)
	movq	16(%rax), %rcx
	movq	40(%rax), %rdi
	movq	%rcx, 200(%rsp)
	movq	%rdi, 456(%rsp)
	cmpq	$-1, %rdi
	je	.LBB192_22
	testq	%r13, %r13
	je	.LBB192_23
	cmpq	$8, %r13
	jae	.LBB192_24
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB192_35
.LBB192_11:
	movq	384(%rsp), %rcx
	movq	376(%rsp), %rax
	movq	392(%rsp), %rdx
	movq	%rcx, 136(%rsp)
	movq	400(%rsp), %rcx
	movq	%rax, 128(%rsp)
	movq	%rdx, 144(%rsp)
	movq	%rcx, 152(%rsp)
	movq	136(%rsp), %rbp
	movq	152(%rsp), %r14
	cmpq	%r14, %rbp
	je	.LBB192_20
	leaq	536(%rsp), %rbx
	leaq	888(%r12), %rax
	movq	%rax, 24(%rsp)
	jmp	.LBB192_14
	.p2align	4
.LBB192_13:
	movq	104(%rsp), %rax
	leaq	(%r15,%r15,4), %rcx
	addq	$40, %rbp
	incq	%r15
	movq	%r12, (%rax,%rcx,8)
	movq	%r13, 8(%rax,%rcx,8)
	movq	48(%rsp), %r12
	vmovdqa	464(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rax,%rcx,8)
	movq	480(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r15, 112(%rsp)
	cmpq	%r14, %rbp
	je	.LBB192_20
.LBB192_14:
	movq	32(%rbp), %rax
	leaq	216(%rsp), %rcx
	movq	%rax, 32(%rcx)
	vmovups	(%rbp), %ymm0
	vmovups	%ymm0, (%rcx)
	movq	%r12, 208(%rsp)
	cmpq	$0, 216(%rsp)
	je	.LBB192_16
	movq	32(%rbp), %rax
	movq	%rax, 32(%rbx)
	vmovups	(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	jmp	.LBB192_18
	.p2align	4
.LBB192_16:
	movq	664(%r12), %rdx
.Ltmp4530:
	movq	24(%rsp), %rsi
	leaq	528(%rsp), %rdi
	leaq	224(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp4531:
	movq	528(%rsp), %rax
	cmpq	$-1, %rax
	jne	.LBB192_26
.LBB192_18:
	vmovups	16(%rbx), %xmm0
	movq	32(%rbx), %rax
	movq	536(%rsp), %r12
	movq	544(%rsp), %r13
	movq	112(%rsp), %r15
	movq	%rax, 480(%rsp)
	vmovaps	%xmm0, 464(%rsp)
	cmpq	96(%rsp), %r15
	jne	.LBB192_13
.Ltmp4538:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp4539:
	jmp	.LBB192_13
.LBB192_20:
	movq	%rbp, 136(%rsp)
	movb	$1, %bpl
.Ltmp4543:
	leaq	128(%rsp), %rdi
	movb	$1, %r15b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4544:
	vmovdqa	96(%rsp), %xmm0
	movq	88(%rsp), %rcx
	movq	112(%rsp), %rax
	movq	%rax, 24(%rcx)
	vmovdqu	%xmm0, 8(%rcx)
	movb	$2, 32(%rcx)
	movq	$-1, (%rcx)
	jmp	.LBB192_3
.LBB192_22:
	movq	%rsi, %r12
	jmp	.LBB192_39
.LBB192_23:
	movq	%rsi, %r12
	xorl	%r13d, %r13d
	jmp	.LBB192_39
.LBB192_24:
	cmpq	$32, %r13
	jae	.LBB192_28
	xorl	%eax, %eax
	xorl	%r14d, %r14d
	jmp	.LBB192_32
.LBB192_26:
	vmovdqu	16(%rbx), %xmm1
	movq	32(%rbx), %rcx
	vmovdqu	536(%rsp), %xmm0
	vmovdqu	592(%rsp), %ymm4
	vmovdqu	576(%rsp), %ymm3
	movq	88(%rsp), %rdx
	addq	$40, %rbp
	movq	%rbp, 136(%rsp)
	movb	$1, %bpl
	movq	%rcx, 480(%rsp)
	movq	480(%rsp), %rcx
	vmovdqa	%xmm1, 464(%rsp)
	vmovdqu	%ymm4, 64(%rdx)
	vmovdqu	%ymm3, 48(%rdx)
	vmovdqa	464(%rsp), %xmm2
	movq	%rcx, 40(%rdx)
	vmovdqu	%xmm2, 24(%rdx)
	movq	%rax, (%rdx)
	vmovdqu	%xmm0, 8(%rdx)
.Ltmp4533:
	leaq	128(%rsp), %rdi
	movb	$1, %r15b
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4534:
	movb	$1, %bpl
	movb	$1, %r15b
	jmp	.LBB192_303
.LBB192_28:
	vmovdqa64	.LCPI192_0(%rip), %zmm1
	vpbroadcastq	.LCPI192_1(%rip), %zmm2
	vpbroadcastq	.LCPI192_2(%rip), %zmm3
	movq	%r13, %rax
	andq	$-32, %rax
	vpxor	%xmm0, %xmm0, %xmm0
	vpxor	%xmm4, %xmm4, %xmm4
	vpxor	%xmm5, %xmm5, %xmm5
	vpxor	%xmm6, %xmm6, %xmm6
	movq	%rax, %rcx
	.p2align	4
.LBB192_29:
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
	jne	.LBB192_29
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
	cmpq	%rax, %r13
	je	.LBB192_37
	testb	$24, %r13b
	je	.LBB192_35
.LBB192_32:
	movq	%rax, %rcx
	vpbroadcastq	%rcx, %zmm1
	vporq	.LCPI192_0(%rip), %zmm1, %zmm1
	vpbroadcastq	.LCPI192_1(%rip), %zmm2
	vpbroadcastq	.LCPI192_3(%rip), %zmm3
	movq	%r13, %rax
	andq	$-8, %rax
	vmovq	%r14, %xmm0
	subq	%rax, %rcx
	.p2align	4
.LBB192_33:
	vpmullq	%zmm2, %zmm1, %zmm4
	kxnorb	%k0, %k0, %k1
	vpxor	%xmm5, %xmm5, %xmm5
	vpaddq	%zmm3, %zmm1, %zmm1
	addq	$8, %rcx
	vpgatherqq	16(%rsi,%zmm4), %zmm5 {%k1}
	vpaddq	%zmm0, %zmm5, %zmm0
	jne	.LBB192_33
	vextracti64x4	$1, %zmm0, %ymm1
	vpaddq	%zmm1, %zmm0, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %r14
	cmpq	%rax, %r13
	je	.LBB192_37
.LBB192_35:
	movq	%r13, %rcx
	subq	%rax, %rcx
	leaq	(%rax,%rax,4), %rax
	shlq	$5, %rax
	leaq	16(%rax,%rsi), %rax
	.p2align	4
.LBB192_36:
	addq	(%rax), %r14
	addq	$160, %rax
	decq	%rcx
	jne	.LBB192_36
.LBB192_37:
	movq	48(%rsp), %rcx
	movq	%rsi, %r12
	movq	912(%rcx), %rax
	movq	928(%rcx), %rsi
	leaq	912(%rcx), %rdx
	subq	%rsi, %rax
	cmpq	%rax, %r14
	ja	.LBB192_333
.LBB192_38:
	movq	48(%rsp), %rax
	cmpq	1016(%rax), %r14
	ja	.LBB192_334
.LBB192_39:
	movq	8(%rsp), %rax
	movq	312(%rsp), %rcx
	movq	%r12, 752(%rsp)
	movq	%r12, 760(%rsp)
	leaq	16(%rax), %r14
	movq	(%rcx), %rax
	leaq	(%r13,%r13,4), %rcx
	shlq	$5, %rcx
	addq	%r12, %rcx
	movq	%rcx, 704(%rsp)
	movq	%rax, 768(%rsp)
	movq	%rcx, 776(%rsp)
	testq	%r13, %r13
	movq	48(%rsp), %r13
	je	.LBB192_240
	movq	8(%rsp), %rdx
	movzbl	%bl, %eax
	movq	%r12, %r15
	leaq	536(%rsp), %r12
	leaq	888(%r13), %rcx
	movq	%r14, 72(%rsp)
	movq	%rax, 688(%rsp)
	movq	%rcx, 192(%rsp)
	leaq	272(%rdx), %rax
	movq	%rax, 304(%rsp)
.LBB192_41:
	leaq	160(%r15), %rdx
	movq	%rdx, 760(%rsp)
	movq	(%r15), %rax
	cmpq	$-1, %rax
	je	.LBB192_240
	movq	%rax, 528(%rsp)
	movq	%rdx, 712(%rsp)
	movq	8(%rsp), %rbx
	vmovdqu64	8(%r15), %zmm0
	vmovdqu64	72(%r15), %zmm1
	vmovdqu64	96(%r15), %zmm2
	vmovdqu64	%zmm2, 88(%r12)
	vmovdqu64	%zmm1, 64(%r12)
	vmovdqu64	%zmm0, (%r12)
	movq	$-1, %r12
	movq	536(%rsp), %rdx
	movq	560(%rsp), %rsi
	movq	552(%rsp), %rcx
	movq	568(%rsp), %rdi
	movq	%rdx, 344(%rsp)
	movq	%rax, 360(%rsp)
	movq	584(%rsp), %rax
	movq	%rsi, 80(%rsp)
	imulq	$88, 544(%rsp), %rsi
	movq	%rcx, 16(%rsp)
	movq	592(%rsp), %rcx
	movq	%rdi, 736(%rsp)
	movq	%rdx, 64(%rsp)
	movq	%rdx, 352(%rsp)
	movq	%rax, 320(%rsp)
	movq	576(%rsp), %rax
	addq	%rdx, %rsi
	movq	%rsi, 56(%rsp)
	movq	%rsi, 368(%rsp)
	movq	%rax, 32(%rsp)
	testq	%rcx, %rcx
	je	.LBB192_212
	movq	320(%rsp), %rbp
	movq	%rcx, %rax
	shlq	$5, %rax
	movq	64(%rsp), %rcx
	addq	%rbp, %rax
	movq	%rcx, 40(%rsp)
	movq	%rax, 728(%rsp)
	movq	624(%rsp), %rax
	movq	%rax, 24(%rsp)
	movq	384(%rsp), %rax
	movq	%rax, 416(%rsp)
	movq	80(%rsp), %rax
	addq	$8, %rax
	movq	%rax, 720(%rsp)
	xorl	%eax, %eax
	jmp	.LBB192_46
	.p2align	4
.LBB192_44:
	movq	416(%rsp), %rbp
.LBB192_45:
	movq	744(%rsp), %rax
	movq	%rbp, 416(%rsp)
	movq	%rbp, 384(%rsp)
	addq	$32, %rax
	movq	%rax, %rbp
	cmpq	728(%rsp), %rax
	movq	328(%rsp), %rax
	je	.LBB192_212
.LBB192_46:
	movq	16(%rbp), %rdx
	movq	24(%rbp), %rcx
	movq	(%rbp), %r15
	movq	8(%rbp), %r14
	movq	%rax, 336(%rsp)
	movq	%rdx, 184(%rsp)
	movq	%rcx, 424(%rsp)
	testq	%r15, %r15
	je	.LBB192_56
	cmpq	$-1, 200(%rsp)
	je	.LBB192_56
	movq	80(%rbx), %rax
	.p2align	4
.LBB192_49:
	movq	%rax, %rcx
	addq	%r15, %rcx
	cmovbq	%r12, %rcx
	lock		cmpxchgq	%rcx, 80(%rbx)
	jne	.LBB192_49
	movq	72(%rsp), %rcx
	addq	%r15, %rax
	cmovbq	%r12, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB192_53
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$0, 208(%rsp)
.Ltmp4449:
	movq	72(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4450:
	cmpb	$-1, 128(%rsp)
	jne	.LBB192_266
.LBB192_53:
	movq	632(%r13), %rax
	testq	%rax, %rax
	je	.LBB192_56
	movl	1228(%r13), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB192_56
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	movq	688(%rsp), %rax
	lock		addq	%r15, (%rcx,%rax,8)
	.p2align	4
.LBB192_56:
	movq	736(%rsp), %rdx
	movq	336(%rsp), %rdi
	cmpq	%rdx, %rdi
	ja	.LBB192_331
	cmpq	%rdx, %r14
	movq	%rdx, %rsi
	cmovbq	%r14, %rsi
	cmpq	%rdi, %r14
	cmovbq	%rdi, %rsi
	cmpq	%rdi, %rsi
	jb	.LBB192_330
	leaq	(,%rdi,8), %rax
	movq	%rsi, 328(%rsp)
	leaq	(%rax,%rax,2), %r12
	leaq	(,%rsi,8), %rax
	leaq	(%rax,%rax,2), %r13
	cmpq	%rsi, %rdi
	jne	.LBB192_65
	xorl	%r14d, %r14d
	xorl	%r9d, %r9d
	xorl	%r8d, %r8d
.LBB192_60:
	cmpq	$-1, 456(%rsp)
	movq	%r9, 448(%rsp)
	movq	%r8, 408(%rsp)
	movq	%rbp, 744(%rsp)
	je	.LBB192_71
	movq	184(%rsp), %rax
	movl	$0, %ecx
	movq	56(%rsp), %rbp
	movl	$0, %ebx
	subq	24(%rsp), %rax
	cmovbq	%rcx, %rax
	subq	40(%rsp), %rbp
	movabsq	$3353953467947191203, %rcx
	shrq	$3, %rbp
	imulq	%rcx, %rbp
	cmpq	%rbp, %rax
	cmovbq	%rax, %rbp
	testq	%rbp, %rbp
	je	.LBB192_72
	movq	40(%rsp), %rax
	xorl	%ebx, %ebx
	leaq	8(%rax), %r15
	.p2align	4
.LBB192_63:
.Ltmp4452:
	movq	purrdf_sparql_eval::scratch::value_bytes@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	vzeroupper
	callq	*%rax
.Ltmp4453:
	addq	%rax, %rbx
	movq	$-1, %rcx
	cmovbq	%rcx, %rbx
	addq	$88, %r15
	decq	%rbp
	jne	.LBB192_63
	jmp	.LBB192_72
	.p2align	4
.LBB192_65:
	movq	%r13, %rdx
	subq	%r12, %rdx
	movabsq	$-6148914691236517205, %rax
	xorl	%r8d, %r8d
	xorl	%r9d, %r9d
	xorl	%r14d, %r14d
	mulxq	%rax, %rax, %rax
	movq	720(%rsp), %rcx
	shrq	$4, %rax
	addq	%r12, %rcx
	jmp	.LBB192_68
	.p2align	4
.LBB192_66:
	addq	%rdx, %r9
	movq	$-1, %rsi
	cmovbq	%rsi, %r9
.LBB192_67:
	addq	$24, %rcx
	decq	%rax
	je	.LBB192_60
.LBB192_68:
	movzbl	-8(%rcx), %esi
	leaq	.LJTI192_0(%rip), %rdi
	movq	(%rcx), %rdx
	movslq	(%rdi,%rsi,4), %rsi
	addq	%rdi, %rsi
	jmpq	*%rsi
.LBB192_69:
	addq	%rdx, %r14
	movq	$-1, %rsi
	cmovbq	%rsi, %r14
	jmp	.LBB192_67
	.p2align	4
.LBB192_70:
	cmpq	%rdx, %r8
	cmovbeq	%rdx, %r8
	jmp	.LBB192_67
	.p2align	4
.LBB192_71:
	xorl	%ebx, %ebx
.LBB192_72:
	movq	8(%rsp), %rax
	movl	296(%rax), %eax
	movq	456(%rsp), %rbp
	testl	%eax, %eax
	je	.LBB192_97
.LBB192_73:
	cmpq	$-1, 200(%rsp)
	je	.LBB192_75
	movq	8(%rsp), %rcx
	movq	$-1, %rdx
	movq	80(%rcx), %rax
	addq	%r14, %rax
	cmovbq	%rdx, %rax
	cmpq	16(%rcx), %rax
	ja	.LBB192_98
.LBB192_75:
	cmpq	$-1, %rbp
	je	.LBB192_77
	movq	48(%rsp), %rcx
	movq	$-1, %rsi
	movq	1048(%rcx), %rax
	movq	1056(%rcx), %rcx
	movq	8(%rsp), %rdx
	subq	%rcx, %rax
	movl	$0, %ecx
	cmovaeq	%rax, %rcx
	addq	%rbx, %rcx
	cmovbq	%rsi, %rcx
	addq	448(%rsp), %rcx
	cmovbq	%rsi, %rcx
	addq	408(%rsp), %rcx
	movq	104(%rdx), %rax
	cmovbq	%rsi, %rcx
	addq	%rcx, %rax
	cmovbq	%rsi, %rax
	cmpq	40(%rdx), %rax
	ja	.LBB192_98
.LBB192_77:
	cmpq	$-1, 200(%rsp)
	movq	8(%rsp), %rbx
	je	.LBB192_79
	movq	336(%rsp), %rcx
	movq	%r12, %rax
	cmpq	328(%rsp), %rcx
	jne	.LBB192_92
.LBB192_79:
	movb	$1, %r15b
	movq	336(%rsp), %rax
	cmpq	328(%rsp), %rax
	je	.LBB192_82
	.p2align	4
.LBB192_80:
	movq	80(%rsp), %rax
	cmpb	$2, -24(%rax,%r13)
	je	.LBB192_161
	addq	$-24, %r13
	cmpq	%r13, %r12
	jne	.LBB192_80
.LBB192_82:
	movq	48(%rsp), %r13
	movq	$-1, %r12
.LBB192_83:
	cmpq	$-1, 200(%rsp)
	je	.LBB192_152
	testq	%r14, %r14
	je	.LBB192_152
	movq	80(%rbx), %rax
	.p2align	4
.LBB192_86:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%r12, %rcx
	lock		cmpxchgq	%rcx, 80(%rbx)
	jne	.LBB192_86
	movq	72(%rsp), %rsi
	addq	%r14, %rax
	cmovbq	%r12, %rax
	movq	(%rsi), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB192_152
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$0, 208(%rsp)
.Ltmp4458:
	leaq	128(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4459:
	cmpb	$-1, 128(%rsp)
	je	.LBB192_152
	movb	$1, %r14b
	cmpq	$-1, %rbp
	jne	.LBB192_183
	jmp	.LBB192_265
	.p2align	4
.LBB192_91:
	addq	$24, %rax
	cmpq	%rax, %r13
	je	.LBB192_79
.LBB192_92:
	movq	80(%rsp), %rcx
	cmpb	$0, (%rcx,%rax)
	jne	.LBB192_91
	movq	80(%rsp), %rcx
	movzbl	1(%rcx,%rax), %ecx
	cmpq	$255, %rcx
	je	.LBB192_91
	movq	48(%rsp), %rdx
	movq	632(%rdx), %rdx
	testq	%rdx, %rdx
	je	.LBB192_91
	movq	48(%rsp), %rsi
	movl	1228(%rsi), %esi
	cmpq	%rsi, 56(%rdx)
	jbe	.LBB192_91
	movq	80(%rsp), %rdi
	movq	%rsi, %r8
	shlq	$7, %r8
	leaq	(%r8,%rsi,8), %rsi
	addq	48(%rdx), %rsi
	movq	8(%rdi,%rax), %rdi
	lock		addq	%rdi, (%rsi,%rcx,8)
	jmp	.LBB192_91
	.p2align	4
.LBB192_97:
	movq	304(%rsp), %rax
	cmpb	$-1, (%rax)
	je	.LBB192_73
.LBB192_98:
	movq	336(%rsp), %rax
	cmpq	328(%rsp), %rax
	jne	.LBB192_108
	movq	8(%rsp), %rbx
.LBB192_100:
	cmpq	$-1, %rbp
	je	.LBB192_159
	movq	184(%rsp), %rax
	movq	48(%rsp), %r13
	movq	$-1, %r12
	cmpq	%rax, 24(%rsp)
	jae	.LBB192_195
	movq	40(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB192_160
	movq	184(%rsp), %rax
	movq	40(%rsp), %rdx
	leaq	216(%rsp), %r14
	leaq	-1(%rax), %rbx
	.p2align	4
.LBB192_104:
	movq	8(%rdx), %rax
	movq	%rdx, %rcx
	cmpq	$-1, %rax
	je	.LBB192_169
	movq	(%rcx), %rsi
	movq	%rax, 208(%rsp)
	movq	%rcx, %r15
	movq	80(%rcx), %rax
	movq	%rax, 64(%r14)
	vmovdqu64	16(%rcx), %zmm0
	vmovdqu64	%zmm0, (%r14)
.Ltmp4490:
	movq	192(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4491:
	movq	24(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB192_168
	incq	%rax
	leaq	88(%r15), %rdx
	movq	%r15, %rcx
	movq	%rax, 24(%rsp)
	cmpq	56(%rsp), %rdx
	jne	.LBB192_104
	jmp	.LBB192_169
	.p2align	4
.LBB192_108:
	movq	80(%rsp), %rax
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r14
	addq	%rax, %r12
	addq	%rax, %r13
	jmp	.LBB192_111
.LBB192_109:
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r14
	.p2align	4
.LBB192_110:
	addq	$24, %r12
	cmpq	%r13, %r12
	je	.LBB192_100
.LBB192_111:
	movzbl	(%r12), %eax
	leaq	.LJTI192_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB192_112:
	cmpq	$-1, 200(%rsp)
	je	.LBB192_110
	movq	%rbx, %rdx
	movzbl	1(%r12), %ebx
	movq	8(%r12), %r14
	movq	16(%r12), %r15
	movq	80(%rdx), %rax
	movq	$-1, %rsi
	.p2align	4
.LBB192_114:
	movq	%rax, %rcx
	addq	%r14, %rcx
	cmovbq	%rsi, %rcx
	lock		cmpxchgq	%rcx, 80(%rdx)
	jne	.LBB192_114
	movq	72(%rsp), %rcx
	addq	%r14, %rax
	cmovbq	%rsi, %rax
	movq	(%rcx), %rcx
	cmpq	%rcx, %rax
	jbe	.LBB192_118
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$0, 208(%rsp)
.Ltmp4484:
	movq	72(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4485:
	cmpb	$-1, 128(%rsp)
	jne	.LBB192_250
.LBB192_118:
	cmpl	$255, %ebx
	je	.LBB192_109
	movq	48(%rsp), %rcx
	movq	632(%rcx), %rax
	testq	%rax, %rax
	je	.LBB192_109
	movl	1228(%rcx), %ecx
	cmpq	%rcx, 56(%rax)
	jbe	.LBB192_109
	movq	%rcx, %rdx
	shlq	$7, %rdx
	leaq	(%rdx,%rcx,8), %rcx
	addq	48(%rax), %rcx
	lock		addq	%r14, (%rcx,%rbx,8)
	jmp	.LBB192_109
	.p2align	4
.LBB192_122:
	movq	8(%r12), %rbx
	cmpq	%rbx, 24(%rsp)
	jae	.LBB192_149
	movq	40(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB192_142
	movq	40(%rsp), %rcx
	leaq	-1(%rbx), %r14
	.p2align	4
.LBB192_125:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB192_147
	movq	(%rdx), %rsi
	movq	%rax, 208(%rsp)
	leaq	216(%rsp), %rcx
	movq	%rdx, %r15
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4473:
	movq	192(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4474:
	movq	24(%rsp), %rax
	cmpq	%rax, %r14
	je	.LBB192_146
	incq	%rax
	leaq	88(%r15), %rcx
	movq	%r15, %rdx
	movq	%rax, 24(%rsp)
	cmpq	56(%rsp), %rcx
	jne	.LBB192_125
	jmp	.LBB192_147
	.p2align	4
.LBB192_129:
	cmpq	$-1, %rbp
	je	.LBB192_110
	movq	8(%r12), %rcx
	movl	296(%rbx), %eax
	testl	%eax, %eax
	je	.LBB192_141
	movq	104(%rbx), %rax
	movq	$-1, %rdx
	addq	%rcx, %rax
	movq	40(%rbx), %rcx
	cmovbq	%rdx, %rax
	cmpq	%rcx, %rax
	jbe	.LBB192_110
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$768, 208(%rsp)
.Ltmp4471:
	movq	72(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4472:
	cmpb	$-1, 128(%rsp)
	je	.LBB192_110
	jmp	.LBB192_271
	.p2align	4
.LBB192_134:
	cmpq	$-1, %rbp
	je	.LBB192_110
	cmpq	$-1, 40(%rbx)
	je	.LBB192_110
	movq	8(%r12), %rcx
	movq	16(%r12), %r15
	movl	296(%rbx), %eax
	testl	%eax, %eax
	je	.LBB192_143
	movq	104(%rbx), %rax
	movq	$-1, %rsi
	.p2align	4
.LBB192_138:
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmovbq	%rsi, %rdx
	lock		cmpxchgq	%rdx, 104(%rbx)
	jne	.LBB192_138
	addq	%rcx, %rax
	movq	40(%rbx), %rcx
	cmovbq	%rsi, %rax
	cmpq	%rcx, %rax
	jbe	.LBB192_110
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$768, 208(%rsp)
.Ltmp4478:
	movq	72(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4479:
	jmp	.LBB192_144
.LBB192_141:
	movq	304(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
	cmpb	$-1, 128(%rsp)
	je	.LBB192_110
	jmp	.LBB192_271
.LBB192_142:
	movq	64(%rsp), %rdx
	jmp	.LBB192_148
.LBB192_143:
	movq	304(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
.LBB192_144:
	movzbl	128(%rsp), %eax
	cmpb	$-1, %al
	setne	%cl
	testq	%r15, %r15
	setne	%dl
	testb	%cl, %dl
	jne	.LBB192_258
	cmpb	$-1, %al
	je	.LBB192_110
	jmp	.LBB192_271
.LBB192_146:
	movq	%r15, %rdx
	movq	%rbx, 24(%rsp)
.LBB192_147:
	movq	16(%rsp), %r14
	addq	$88, %rdx
	movq	%rdx, 40(%rsp)
.LBB192_148:
	movq	%rdx, 64(%rsp)
	movq	%rdx, 352(%rsp)
.LBB192_149:
	movq	8(%rsp), %rbx
	cmpq	$-1, %rbp
	je	.LBB192_110
.Ltmp4476:
	movq	48(%rsp), %rsi
	leaq	208(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp4477:
	cmpb	$-1, 208(%rsp)
	je	.LBB192_110
	jmp	.LBB192_271
.LBB192_152:
	cmpq	$-1, %rbp
	je	.LBB192_195
	cmpq	$-1, 40(%rbx)
	je	.LBB192_175
	movl	296(%rbx), %eax
	testl	%eax, %eax
	je	.LBB192_172
	movq	104(%rbx), %rax
	movq	448(%rsp), %rdx
	.p2align	4
.LBB192_156:
	movq	%rax, %rcx
	addq	%rdx, %rcx
	cmovbq	%r12, %rcx
	lock		cmpxchgq	%rcx, 104(%rbx)
	jne	.LBB192_156
	movq	40(%rbx), %rcx
	addq	%rdx, %rax
	cmovbq	%r12, %rax
	cmpq	%rcx, %rax
	jbe	.LBB192_175
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$768, 208(%rsp)
.Ltmp4461:
	movq	72(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4462:
	jmp	.LBB192_173
.LBB192_159:
	movq	48(%rsp), %r13
	movq	$-1, %r12
	jmp	.LBB192_195
.LBB192_160:
	movq	64(%rsp), %rcx
	jmp	.LBB192_170
.LBB192_161:
	movq	80(%rsp), %rax
	movq	$-1, %r12
	movq	-16(%rax,%r13), %rbx
	cmpq	%rbx, 24(%rsp)
	jae	.LBB192_171
	movq	48(%rsp), %r13
	movq	40(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB192_208
	movq	40(%rsp), %rcx
	.p2align	4
.LBB192_164:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB192_210
	movq	(%rdx), %rsi
	movq	%rax, 208(%rsp)
	leaq	216(%rsp), %rcx
	movq	%rdx, %r15
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4455:
	movq	192(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4456:
	movq	24(%rsp), %rax
	leaq	-1(%rbx), %rcx
	cmpq	%rax, %rcx
	je	.LBB192_209
	incq	%rax
	leaq	88(%r15), %rcx
	movq	%r15, %rdx
	movq	%rax, 24(%rsp)
	cmpq	56(%rsp), %rcx
	jne	.LBB192_164
	jmp	.LBB192_210
.LBB192_168:
	movq	184(%rsp), %rax
	movq	%r15, %rcx
	movq	%rax, 24(%rsp)
.LBB192_169:
	movq	8(%rsp), %rbx
	addq	$88, %rcx
	movq	%rcx, 40(%rsp)
.LBB192_170:
	movq	%rcx, 64(%rsp)
	movq	%rcx, 352(%rsp)
	jmp	.LBB192_195
.LBB192_171:
	movq	8(%rsp), %rbx
	movq	48(%rsp), %r13
	xorl	%r15d, %r15d
	jmp	.LBB192_83
.LBB192_172:
	movq	304(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
.LBB192_173:
	cmpb	$-1, 128(%rsp)
	je	.LBB192_175
	movb	$1, %r14b
	movq	184(%rsp), %rax
	cmpq	%rax, 24(%rsp)
	jb	.LBB192_184
	jmp	.LBB192_194
.LBB192_175:
	testb	%r15b, %r15b
	jne	.LBB192_178
.Ltmp4463:
	leaq	208(%rsp), %rdi
	movq	%r13, %rsi
	vzeroupper
	callq	<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
.Ltmp4464:
	cmpb	$-1, 208(%rsp)
	movb	$1, %r14b
	jne	.LBB192_183
.LBB192_178:
	cmpq	$0, 408(%rsp)
	je	.LBB192_182
	movl	296(%rbx), %eax
	testl	%eax, %eax
	je	.LBB192_205
	movq	104(%rbx), %rax
	movq	40(%rbx), %rcx
	addq	408(%rsp), %rax
	cmovbq	%r12, %rax
	cmpq	%rcx, %rax
	jbe	.LBB192_206
	movq	%rcx, 216(%rsp)
	movq	%rax, 224(%rsp)
	movw	$768, 208(%rsp)
.Ltmp4465:
	movq	72(%rsp), %rsi
	leaq	128(%rsp), %rdi
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4466:
	jmp	.LBB192_207
.LBB192_182:
	xorl	%r14d, %r14d
.LBB192_183:
	movq	184(%rsp), %rax
	cmpq	%rax, 24(%rsp)
	jae	.LBB192_194
.LBB192_184:
	movq	40(%rsp), %rax
	cmpq	56(%rsp), %rax
	je	.LBB192_190
	movq	184(%rsp), %rax
	movq	40(%rsp), %rcx
	leaq	-1(%rax), %rbx
	.p2align	4
.LBB192_186:
	movq	8(%rcx), %rax
	movq	%rcx, %rdx
	cmpq	$-1, %rax
	je	.LBB192_192
	movq	(%rdx), %rsi
	movq	%rax, 208(%rsp)
	leaq	216(%rsp), %rcx
	movq	%rdx, %r15
	movq	80(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	16(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4468:
	movq	192(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	leaq	208(%rsp), %rdx
	vzeroupper
	callq	*%rax
.Ltmp4469:
	movq	24(%rsp), %rax
	cmpq	%rax, %rbx
	je	.LBB192_191
	incq	%rax
	leaq	88(%r15), %rcx
	movq	%r15, %rdx
	movq	%rax, 24(%rsp)
	cmpq	56(%rsp), %rcx
	jne	.LBB192_186
	jmp	.LBB192_192
.LBB192_190:
	movq	64(%rsp), %rdx
	jmp	.LBB192_193
.LBB192_191:
	movq	184(%rsp), %rax
	movq	%r15, %rdx
	movq	%rax, 24(%rsp)
.LBB192_192:
	movq	8(%rsp), %rbx
	addq	$88, %rdx
	movq	%rdx, 40(%rsp)
.LBB192_193:
	movq	%rdx, 64(%rsp)
	movq	%rdx, 352(%rsp)
.LBB192_194:
	testb	%r14b, %r14b
	jne	.LBB192_265
.LBB192_195:
	cmpq	$0, 424(%rsp)
	je	.LBB192_44
	movq	400(%rsp), %r14
	movq	416(%rsp), %rbp
	jmp	.LBB192_198
	.p2align	4
.LBB192_197:
	movq	104(%rsp), %rax
	leaq	(%r15,%r15,4), %rcx
	movq	424(%rsp), %rsi
	addq	$40, %rbp
	incq	%r15
	movq	%rbx, (%rax,%rcx,8)
	movq	%r12, 8(%rax,%rcx,8)
	movq	8(%rsp), %rbx
	decq	%rsi
	movq	$-1, %r12
	vmovdqa	496(%rsp), %xmm0
	movq	%rsi, 424(%rsp)
	vmovdqu	%xmm0, 16(%rax,%rcx,8)
	movq	512(%rsp), %rdx
	movq	%rdx, 32(%rax,%rcx,8)
	movq	%r15, 112(%rsp)
	testq	%rsi, %rsi
	je	.LBB192_45
.LBB192_198:
	cmpq	%r14, %rbp
	je	.LBB192_45
	movq	32(%rbp), %rax
	leaq	136(%rsp), %rcx
	movq	%rbp, %rdx
	movq	%rax, 32(%rcx)
	vmovdqu	(%rbp), %ymm0
	vmovdqu	%ymm0, (%rcx)
	movq	%r13, 128(%rsp)
	cmpq	$0, 136(%rsp)
	je	.LBB192_201
	movq	32(%rdx), %rax
	leaq	216(%rsp), %rcx
	movq	%rdx, %rbp
	movq	%rax, 32(%rcx)
	vmovdqu	(%rdx), %ymm0
	vmovdqu	%ymm0, (%rcx)
	jmp	.LBB192_203
	.p2align	4
.LBB192_201:
	movq	%rdx, %rbp
	movq	664(%r13), %rdx
.Ltmp4493:
	movq	192(%rsp), %rsi
	leaq	208(%rsp), %rdi
	leaq	144(%rsp), %rcx
	vzeroupper
	callq	purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
.Ltmp4494:
	movq	208(%rsp), %rax
	leaq	216(%rsp), %rcx
	cmpq	$-1, %rax
	jne	.LBB192_249
.LBB192_203:
	vmovdqu	16(%rcx), %xmm0
	movq	32(%rcx), %rax
	movq	216(%rsp), %rbx
	movq	224(%rsp), %r12
	movq	112(%rsp), %r15
	movq	%rax, 512(%rsp)
	vmovdqa	%xmm0, 496(%rsp)
	cmpq	96(%rsp), %r15
	jne	.LBB192_197
.Ltmp4503:
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	96(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp4504:
	jmp	.LBB192_197
.LBB192_205:
	movq	304(%rsp), %rcx
	movq	16(%rcx), %rax
	movq	%rax, 144(%rsp)
	vmovdqu	(%rcx), %xmm0
	vmovdqa	%xmm0, 128(%rsp)
	jmp	.LBB192_207
.LBB192_206:
	movb	$-1, 128(%rsp)
.LBB192_207:
	cmpb	$-1, 128(%rsp)
	setne	%r14b
	movq	184(%rsp), %rax
	cmpq	%rax, 24(%rsp)
	jb	.LBB192_184
	jmp	.LBB192_194
.LBB192_208:
	movq	64(%rsp), %rdx
	jmp	.LBB192_211
.LBB192_209:
	movq	%r15, %rdx
	movq	%rbx, 24(%rsp)
.LBB192_210:
	addq	$88, %rdx
	movq	%rdx, 40(%rsp)
.LBB192_211:
	movq	8(%rsp), %rbx
	xorl	%r15d, %r15d
	movq	%rdx, 64(%rsp)
	movq	%rdx, 352(%rsp)
	jmp	.LBB192_83
.LBB192_212:
	movq	32(%rsp), %rsi
	movq	16(%rsp), %r14
	movq	712(%rsp), %r15
	leaq	536(%rsp), %r12
	testq	%rsi, %rsi
	je	.LBB192_222
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$5, %rsi
	movabsq	$9223372036854775807, %rdx
	cmpq	%rdx, %rsi
	cmovaeq	%rdx, %rsi
	xorl	%ecx, %ecx
	cmpq	%rsi, %rax
	setns	%cl
	addq	%rdx, %rcx
	subq	%rsi, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB192_215
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB192_215:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB192_221
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB192_215
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
.LBB192_218:
	cmpq	%rax, %rcx
	jge	.LBB192_220
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rdx
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB192_218
.LBB192_220:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB192_221:
	movq	320(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB192_222:
.Ltmp4513:
	leaq	344(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp4514:
	testq	%r14, %r14
	je	.LBB192_233
	shlq	$3, %r14
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%r14,%r14,2), %rcx
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
	jge	.LBB192_226
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB192_226:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB192_232
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB192_226
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
.LBB192_229:
	cmpq	%rax, %rdx
	jge	.LBB192_231
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB192_229
.LBB192_231:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB192_232:
	movq	80(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB192_233:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB192_236
	lock		decq	(%rax)
	jne	.LBB192_236
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB192_236:
	movq	648(%rsp), %rax
	movq	72(%rsp), %r14
	testq	%rax, %rax
	je	.LBB192_239
	lock		decq	(%rax)
	jne	.LBB192_239
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	648(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB192_239:
	cmpq	704(%rsp), %r15
	jne	.LBB192_41
.LBB192_240:
	movb	$1, %bpl
	xorl	%r15d, %r15d
.Ltmp4518:
	leaq	752(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4519:
	movq	696(%rsp), %rdx
	cmpq	$-1, 200(%rsp)
	movq	312(%rsp), %rcx
	movq	8(%rsp), %rbx
	sete	%al
	notb	%dl
	orb	24(%rcx), %dl
	orb	%al, %dl
	testb	$1, %dl
	jne	.LBB192_246
	movq	80(%rbx), %rax
	movq	$-1, %rcx
	.p2align	4
.LBB192_243:
	movq	%rax, %rdx
	incq	%rdx
	cmoveq	%rcx, %rdx
	lock		cmpxchgq	%rdx, 80(%rbx)
	jne	.LBB192_243
	incq	%rax
	movq	$-1, %rcx
	cmovneq	%rax, %rcx
	movq	(%r14), %rax
	cmpq	%rax, %rcx
	jbe	.LBB192_246
	movq	%rax, 536(%rsp)
	movq	%rcx, 544(%rsp)
	movw	$0, 528(%rsp)
.Ltmp4521:
	leaq	208(%rsp), %rdi
	leaq	528(%rsp), %rdx
	movq	%r14, %rsi
	callq	<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)
.Ltmp4522:
.LBB192_246:
	vmovdqa	96(%rsp), %xmm0
	movq	88(%rsp), %rcx
	movq	112(%rsp), %rax
	movq	%rax, 24(%rcx)
	vmovdqu	%xmm0, 8(%rcx)
	movb	$2, 32(%rcx)
	movq	$-1, (%rcx)
	lock		decq	(%rbx)
	jne	.LBB192_248
	xorl	%ebp, %ebp
	#MEMBARRIER
.Ltmp4526:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	432(%rsp), %rdi
	xorl	%r15d, %r15d
	callq	*%rax
.Ltmp4527:
.LBB192_248:
	xorl	%ebp, %ebp
.Ltmp4528:
	leaq	376(%rsp), %rdi
	xorl	%r15d, %r15d
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4529:
	jmp	.LBB192_4
.LBB192_249:
	vmovdqu	16(%rcx), %xmm1
	movq	32(%rcx), %rcx
	vmovdqu	216(%rsp), %xmm0
	vmovdqu	272(%rsp), %ymm4
	vmovdqu	256(%rsp), %ymm3
	movq	88(%rsp), %rdx
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r14
	addq	$40, %rbp
	movq	%rbp, 384(%rsp)
	movb	$1, %bpl
	movq	%rcx, 512(%rsp)
	movq	512(%rsp), %rcx
	vmovdqa	%xmm1, 496(%rsp)
	vmovdqu	%ymm4, 64(%rdx)
	vmovdqu	%ymm3, 48(%rdx)
	vmovdqa	496(%rsp), %xmm2
	movq	%rcx, 40(%rdx)
	vmovdqu	%xmm2, 24(%rdx)
	movq	%rax, (%rdx)
	vmovdqu	%xmm0, 8(%rdx)
	jmp	.LBB192_272
.LBB192_250:
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r14
	movq	%r15, %rax
	addq	$-1, %rax
	jae	.LBB192_271
	cmpq	%rax, 24(%rsp)
	jae	.LBB192_271
	movq	40(%rsp), %rax
	movq	64(%rsp), %rdx
	cmpq	56(%rsp), %rax
	je	.LBB192_270
	subq	24(%rsp), %r15
	addq	$88, %rax
	leaq	208(%rsp), %r14
	addq	$-2, %r15
.LBB192_254:
	movq	%rax, %rdx
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB192_270
	movq	-88(%rdx), %rsi
	movq	%rax, 208(%rsp)
	leaq	216(%rsp), %rcx
	movq	%rdx, %r12
	movq	-8(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4487:
	movq	192(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp4488:
	subq	$1, %r15
	jb	.LBB192_269
	leaq	88(%r12), %rax
	movq	%r12, %rdx
	cmpq	56(%rsp), %r12
	jne	.LBB192_254
	jmp	.LBB192_270
.LBB192_258:
	leaq	-1(%r15), %rax
	cmpq	%rax, 24(%rsp)
	jae	.LBB192_271
	movq	40(%rsp), %rax
	movq	64(%rsp), %rdx
	cmpq	56(%rsp), %rax
	je	.LBB192_270
	subq	24(%rsp), %r15
	addq	$88, %rax
	leaq	208(%rsp), %r14
	addq	$-2, %r15
.LBB192_261:
	movq	%rax, %rdx
	movq	-80(%rax), %rax
	cmpq	$-1, %rax
	je	.LBB192_270
	movq	-88(%rdx), %rsi
	movq	%rax, 208(%rsp)
	leaq	216(%rsp), %rcx
	movq	%rdx, %r12
	movq	-8(%rdx), %rax
	movq	%rax, 64(%rcx)
	vmovdqu64	-72(%rdx), %zmm0
	vmovdqu64	%zmm0, (%rcx)
.Ltmp4481:
	movq	192(%rsp), %rdi
	movq	<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint@GOTPCREL(%rip), %rax
	movq	%r14, %rdx
	vzeroupper
	callq	*%rax
.Ltmp4482:
	subq	$1, %r15
	jb	.LBB192_269
	leaq	88(%r12), %rax
	movq	%r12, %rdx
	cmpq	56(%rsp), %r12
	jne	.LBB192_261
	jmp	.LBB192_270
.LBB192_265:
	vmovdqa	96(%rsp), %xmm0
	movq	88(%rsp), %rcx
	movq	112(%rsp), %rax
	movq	%rax, 24(%rcx)
	vmovdqu	%xmm0, 8(%rcx)
	movb	$1, 32(%rcx)
	jmp	.LBB192_267
.LBB192_266:
	vmovdqa	96(%rsp), %xmm0
	movq	88(%rsp), %rcx
	movq	112(%rsp), %rax
	movq	%rax, 24(%rcx)
	vmovdqu	%xmm0, 8(%rcx)
	movb	$0, 32(%rcx)
.LBB192_267:
	movq	16(%rsp), %r14
	movq	$-1, (%rcx)
	xorl	%ebp, %ebp
	jmp	.LBB192_272
.LBB192_269:
	movq	%r12, %rdx
.LBB192_270:
	movq	16(%rsp), %r14
	movq	%rdx, 352(%rsp)
.LBB192_271:
	vmovdqa	96(%rsp), %xmm0
	movq	88(%rsp), %rcx
	movq	112(%rsp), %rax
	xorl	%ebp, %ebp
	movq	%rax, 24(%rcx)
	vmovdqu	%xmm0, 8(%rcx)
	movb	$1, 32(%rcx)
	movq	$-1, (%rcx)
.LBB192_272:
	movq	32(%rsp), %rdi
	testq	%rdi, %rdi
	je	.LBB192_282
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
	jge	.LBB192_275
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB192_275:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB192_281
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB192_275
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
.LBB192_278:
	cmpq	%rax, %rcx
	jge	.LBB192_280
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB192_278
.LBB192_280:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB192_281:
	movq	320(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB192_282:
.Ltmp4496:
	leaq	344(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp4497:
	testq	%r14, %r14
	je	.LBB192_293
	shlq	$3, %r14
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%r14,%r14,2), %rcx
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
	jge	.LBB192_286
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB192_286:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB192_292
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB192_286
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
.LBB192_289:
	cmpq	%rax, %rdx
	jge	.LBB192_291
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB192_289
.LBB192_291:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB192_292:
	movq	80(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB192_293:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB192_296
	lock		decq	(%rax)
	jne	.LBB192_296
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB192_296:
	movq	648(%rsp), %rax
	testq	%rax, %rax
	je	.LBB192_299
	lock		decq	(%rax)
	jne	.LBB192_299
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	648(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB192_299:
	xorl	%r15d, %r15d
.Ltmp4499:
	leaq	752(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4500:
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB192_302
	xorl	%r15d, %r15d
	#MEMBARRIER
.Ltmp4501:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	432(%rsp), %rdi
	callq	*%rax
.Ltmp4502:
.LBB192_302:
	xorl	%r15d, %r15d
.LBB192_303:
	cmpq	$0, 440(%rsp)
	je	.LBB192_305
.Ltmp4535:
	leaq	376(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4536:
.LBB192_305:
	testb	%bpl, %bpl
	je	.LBB192_329
	movq	104(%rsp), %rbx
	movq	112(%rsp), %r14
	movl	%r15d, 24(%rsp)
	testq	%r14, %r14
	je	.LBB192_319
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r12
	movabsq	$9223372036854775807, %r13
	xorl	%r15d, %r15d
	jmp	.LBB192_311
	.p2align	4
.LBB192_308:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB192_309:
	callq	*%r12
.LBB192_310:
	incq	%r15
	cmpq	%r14, %r15
	je	.LBB192_319
.LBB192_311:
	leaq	(%r15,%r15,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB192_310
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
	jge	.LBB192_314
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB192_314:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB192_309
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB192_314
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
.LBB192_317:
	cmpq	%rax, %rdx
	jge	.LBB192_308
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB192_317
	jmp	.LBB192_308
.LBB192_319:
	movq	96(%rsp), %rax
	movl	24(%rsp), %r15d
	testq	%rax, %rax
	je	.LBB192_329
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
	jge	.LBB192_322
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB192_322:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB192_328
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB192_322
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
.LBB192_325:
	cmpq	%rax, %rsi
	jge	.LBB192_327
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB192_325
.LBB192_327:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB192_328:
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
.LBB192_329:
	testb	%r15b, %r15b
	jne	.LBB192_3
	jmp	.LBB192_4
.LBB192_330:
.Ltmp4506:
	movq	core::slice::index::slice_index_fail@GOTPCREL(%rip), %rax
	movq	16(%rsp), %r14
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.290(%rip), %rcx
	vzeroupper
	callq	*%rax
.Ltmp4507:
	jmp	.LBB192_332
.LBB192_331:
	leaq	784(%rsp), %rax
	leaq	128(%rsp), %rcx
	movq	%rdi, 784(%rsp)
	movq	%rdx, 128(%rsp)
	movq	%rax, 208(%rsp)
	leaq	<usize as core::fmt::Debug>::fmt(%rip), %rax
	movq	%rax, 216(%rsp)
	movq	%rcx, 224(%rsp)
	movq	%rax, 232(%rsp)
.Ltmp4508:
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	movq	16(%rsp), %r14
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.2158(%rip), %rdi
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.289(%rip), %rdx
	leaq	208(%rsp), %rsi
	vzeroupper
	callq	*%rax
.Ltmp4509:
.LBB192_332:
	ud2
.LBB192_333:
	movb	$1, %bpl
.Ltmp4445:
	movl	$8, %ecx
	movl	$80, %r8d
	movb	$1, %r15b
	movq	%rdx, %rdi
	movq	%rdx, 24(%rsp)
	movq	%r14, %rdx
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
	movq	24(%rsp), %rdx
.Ltmp4446:
	jmp	.LBB192_38
.LBB192_334:
	movq	48(%rsp), %rax
	movb	$1, %bpl
	leaq	1000(%rax), %rdi
.Ltmp4447:
	movq	<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %ecx
	movq	%r14, %rsi
	movb	$1, %r15b
	vzeroupper
	callq	*%rax
.Ltmp4448:
	jmp	.LBB192_39
.LBB192_335:
.Ltmp4523:
	movb	$1, %bpl
	movq	%rax, %r13
	jmp	.LBB192_381
.LBB192_336:
.Ltmp4483:
	jmp	.LBB192_338
.LBB192_337:
.Ltmp4489:
.LBB192_338:
	movq	16(%rsp), %r14
	movq	%rax, %r13
	movq	%r12, 352(%rsp)
	jmp	.LBB192_369
.LBB192_339:
.Ltmp4460:
	jmp	.LBB192_344
.LBB192_340:
.Ltmp4467:
	jmp	.LBB192_344
.LBB192_341:
.Ltmp4515:
	movq	%rax, %r13
	movb	$1, %bpl
	jmp	.LBB192_372
.LBB192_342:
.Ltmp4498:
	movq	%rax, %r13
	jmp	.LBB192_372
.LBB192_343:
.Ltmp4451:
.LBB192_344:
	movq	16(%rsp), %r14
	jmp	.LBB192_368
.LBB192_345:
.Ltmp4457:
	jmp	.LBB192_363
.LBB192_346:
.Ltmp4537:
	movq	%rax, %r13
	jmp	.LBB192_385
.LBB192_347:
.Ltmp4520:
	movq	8(%rsp), %rbx
	movq	%rax, %r13
	jmp	.LBB192_382
.LBB192_348:
.Ltmp4545:
	cmpq	$0, 440(%rsp)
	movq	%rax, %r13
	jne	.LBB192_384
	jmp	.LBB192_385
.LBB192_349:
.Ltmp4486:
	jmp	.LBB192_365
.LBB192_350:
.Ltmp4470:
	jmp	.LBB192_363
.LBB192_351:
.Ltmp4444:
	movq	%rax, %r13
	jmp	.LBB192_388
.LBB192_352:
.Ltmp4532:
	addq	$40, %rbp
	movq	%rax, %rbx
	movq	%rbp, 136(%rsp)
	jmp	.LBB192_356
.LBB192_353:
.Ltmp4439:
	movq	%rax, %r13
.Ltmp4440:
	movq	%r14, %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4441:
	jmp	.LBB192_388
.LBB192_354:
.Ltmp4540:
	addq	$40, %rbp
	movq	%rax, %rbx
	movq	%rbp, 136(%rsp)
	cmpq	$6, %r12
	jb	.LBB192_356
	leaq	-8(,%r12,8), %rsi
	movl	$4, %edx
	movq	%r13, %rdi
	callq	__rustc::__rust_dealloc
.LBB192_356:
	movb	$1, %r15b
.Ltmp4541:
	leaq	128(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4542:
	movq	%rbx, %r13
	jmp	.LBB192_386
.LBB192_357:
.Ltmp4480:
	jmp	.LBB192_368
.LBB192_358:
.Ltmp4492:
	jmp	.LBB192_363
.LBB192_359:
.Ltmp4495:
	addq	$40, %rbp
	movq	%rax, %r13
	movq	%rbp, 384(%rsp)
	jmp	.LBB192_366
.LBB192_360:
.Ltmp4505:
	addq	$40, %rbp
	movq	%rax, %r13
	movq	%rbp, 384(%rsp)
	cmpq	$6, %rbx
	jb	.LBB192_366
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r12, %rdi
	callq	__rustc::__rust_dealloc
	jmp	.LBB192_366
.LBB192_362:
.Ltmp4475:
.LBB192_363:
	addq	$88, %r15
	movq	%rax, %r13
	movq	%r15, 352(%rsp)
	jmp	.LBB192_366
.LBB192_364:
.Ltmp4454:
.LBB192_365:
	movq	%rax, %r13
.LBB192_366:
	movq	8(%rsp), %rbx
	movq	16(%rsp), %r14
	jmp	.LBB192_369
.LBB192_367:
.Ltmp4510:
.LBB192_368:
	movq	%rax, %r13
.LBB192_369:
	movq	32(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB192_371
	movq	320(%rsp), %rdi
	shlq	$5, %rsi
	movl	$8, %edx
	callq	__rustc::__rust_dealloc
.LBB192_371:
	movb	$1, %bpl
.Ltmp4511:
	leaq	344(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
.Ltmp4512:
.LBB192_372:
	testq	%r14, %r14
	je	.LBB192_374
	movq	80(%rsp), %rdi
	shlq	$3, %r14
	movl	$8, %edx
	leaq	(%r14,%r14,2), %rsi
	callq	__rustc::__rust_dealloc
.LBB192_374:
	movq	616(%rsp), %rax
	testq	%rax, %rax
	je	.LBB192_377
	lock		decq	(%rax)
	jne	.LBB192_377
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	616(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB192_377:
	movq	648(%rsp), %rax
	testq	%rax, %rax
	je	.LBB192_380
	lock		decq	(%rax)
	jne	.LBB192_380
	movq	<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	648(%rsp), %rdi
	#MEMBARRIER
	callq	*%rax
.LBB192_380:
.Ltmp4516:
	leaq	752(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4517:
.LBB192_381:
	xorl	%r15d, %r15d
.LBB192_382:
	lock		decq	(%rbx)
	jne	.LBB192_384
	#MEMBARRIER
.Ltmp4524:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	432(%rsp), %rdi
	callq	*%rax
.Ltmp4525:
.LBB192_384:
.Ltmp4546:
	leaq	376(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
.Ltmp4547:
.LBB192_385:
	testb	%bpl, %bpl
	je	.LBB192_387
.LBB192_386:
	leaq	96(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB192_387:
	testb	%r15b, %r15b
	je	.LBB192_389
.LBB192_388:
.Ltmp4548:
	movq	312(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
.Ltmp4549:
.LBB192_389:
	movq	%r13, %rdi
	callq	_Unwind_Resume@PLT
.LBB192_390:
.Ltmp4550:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end192:

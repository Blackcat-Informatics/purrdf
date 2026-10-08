<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>> as alloc::vec::spec_from_iter::SpecFromIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, core::iter::adapters::map::Map<core::slice::iter::Iter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>>::from_iter:
.Lfunc_begin1966:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1302
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
	subq	$200, %rsp
	.cfi_def_cfa_offset 256
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	8(%rsi), %rax
	movq	(%rsi), %rbp
	movabsq	$9223372036854775800, %rcx
	movq	%rax, %r15
	subq	%rbp, %r15
	cmpq	%rcx, %r15
	jbe	.LBB1966_3
	xorl	%edi, %edi
.LBB1966_2:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r15, %rsi
	callq	*%rax
.LBB1966_3:
	cmpq	%rbp, %rax
	je	.LBB1966_4
	movq	%rdi, %r14
	movq	%rsi, %rbx
	cmpq	$7, %r15
	jbe	.LBB1966_6
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r15, %rdi
	callq	*%rax
	movq	%rax, %r8
	jmp	.LBB1966_9
.LBB1966_4:
	movq	$0, 72(%rsp)
	movq	$8, 80(%rsp)
	xorl	%r9d, %r9d
	jmp	.LBB1966_61
.LBB1966_6:
	movq	posix_memalign@GOTPCREL(%rip), %rax
	leaq	8(%rsp), %rdi
	movl	$8, %esi
	movq	%r15, %rdx
	movq	$0, 8(%rsp)
	callq	*%rax
	movl	$8, %edi
	testl	%eax, %eax
	jne	.LBB1966_2
	movq	8(%rsp), %r8
.LBB1966_9:
	testq	%r8, %r8
	je	.LBB1966_10
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdi
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %r10
	movabsq	$-9223372036854775808, %rcx
	movq	%r15, %r9
	shrq	$3, %r9
	movq	$-1, %r11
	movabsq	$-3689348814741910323, %rax
	leaq	(%r15,%rdx), %rsi
	sarq	$63, %rsi
	xorq	%rcx, %rsi
	addq	%r15, %rdx
	cmovoq	%rsi, %rdx
	incq	%rdi
	cmoveq	%r11, %rdi
	addq	%r15, %r10
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%r11, %r10
	movq	%rdi, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%r10, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jle	.LBB1966_13
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB1966_13:
	imulq	%rax, %r9
	.p2align	4
.LBB1966_14:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB1966_20
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB1966_14
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
.LBB1966_17:
	cmpq	%rax, %rdx
	jle	.LBB1966_19
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB1966_17
.LBB1966_19:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB1966_20:
	movq	16(%rbx), %rax
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %r11
	movl	$2, %ecx
	leaq	8(%rsp), %r15
	movq	%r9, 72(%rsp)
	movq	%r14, 96(%rsp)
	movq	%r8, 80(%rsp)
	movq	$0, (%rsp)
	movq	%rbp, 120(%rsp)
	movq	%r8, 112(%rsp)
	movq	%r9, 104(%rsp)
	vmovd	%ecx, %xmm0
	vmovdqa	%xmm0, 144(%rsp)
	movq	%rax, 128(%rsp)
	jmp	.LBB1966_21
	.p2align	4
.LBB1966_58:
	movq	48(%rsp), %rax
	movq	%r14, (%rax)
.LBB1966_59:
	vmovdqu	8(%rsp), %ymm0
	movq	40(%rsp), %rax
	movq	112(%rsp), %r8
	movq	136(%rsp), %rcx
	movq	104(%rsp), %r9
	movq	120(%rsp), %rbp
	movq	%rax, 192(%rsp)
	vmovdqu	%ymm0, 160(%rsp)
	movq	%rax, 32(%r8,%rcx,8)
	movq	(%rsp), %rax
	vmovdqu	%ymm0, (%r8,%rcx,8)
	incq	%rax
	movq	%rax, (%rsp)
	cmpq	%r9, %rax
	je	.LBB1966_60
.LBB1966_21:
	movq	128(%rsp), %rax
	movl	$1, %r14d
	leaq	16(%rsp), %rcx
	movq	%r15, %r8
	movq	8(%rax), %r13
	movq	16(%rax), %r12
	movl	$4, %eax
	movq	$1, 8(%rsp)
	cmpq	$5, %r12
	jae	.LBB1966_22
.LBB1966_24:
	movq	(%rsp), %rdx
	shlq	$4, %r12
	leaq	(%rdx,%rdx,4), %rdi
	leaq	-1(%r14), %rdx
	leaq	(%rbp,%rdi,8), %rbp
	movq	%rdi, 136(%rsp)
	cmpq	%rax, %rdx
	jae	.LBB1966_25
	movq	(%rbp), %rsi
	movq	%r8, 48(%rsp)
	decq	%rsi
	cmpq	$5, %rsi
	jb	.LBB1966_27
	movq	16(%rbp), %rsi
	movq	8(%rbp), %rdx
	decq	%rsi
	jmp	.LBB1966_29
	.p2align	4
.LBB1966_25:
	movq	%r13, %rbx
	movq	%r14, %rax
	addq	%r12, %r13
	movq	%rax, (%r8)
	cmpq	%r13, %rbx
	jne	.LBB1966_37
	jmp	.LBB1966_59
	.p2align	4
.LBB1966_27:
	leaq	8(%rbp), %rdx
.LBB1966_29:
	movq	%r12, %r8
	incq	%rax
	negq	%r8
	xorl	%r9d, %r9d
	movq	%r13, %r10
	jmp	.LBB1966_30
	.p2align	4
.LBB1966_48:
	vmovq	(%rdx,%rdi,8), %xmm0
.LBB1966_49:
	vmovq	%xmm0, -8(%rcx,%r14,8)
	addq	$16, %r10
	incq	%r14
	addq	$-16, %r9
	cmpq	%r14, %rax
	je	.LBB1966_35
.LBB1966_30:
	cmpq	%r9, %r8
	je	.LBB1966_58
	vmovdqa	144(%rsp), %xmm0
	cmpl	$1, (%r10)
	jne	.LBB1966_49
	movq	8(%r10), %rdi
	cmpq	%rsi, %rdi
	jb	.LBB1966_48
	jmp	.LBB1966_33
	.p2align	4
.LBB1966_35:
	movq	48(%rsp), %r8
	movq	%r13, %rbx
	subq	%r9, %rbx
	addq	%r12, %r13
	movq	%rax, (%r8)
	cmpq	%r13, %rbx
	je	.LBB1966_59
.LBB1966_37:
	movq	(%rbp), %r12
	decq	%r12
	cmpq	$5, %r12
	jb	.LBB1966_38
	movq	16(%rbp), %r12
	movq	8(%rbp), %rbp
	decq	%r12
	jmp	.LBB1966_40
	.p2align	4
.LBB1966_38:
	addq	$8, %rbp
	jmp	.LBB1966_40
.LBB1966_45:
.Ltmp40784:
	movl	$1, %edx
	movl	$1, %ecx
	movq	%r15, %rdi
	vzeroupper
	callq	*%r11
.Ltmp40785:
	cmpq	$6, 8(%rsp)
	movq	16(%rsp), %rax
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %r11
	leaq	16(%rsp), %rcx
	leaq	24(%rsp), %rdx
	cmovbq	%rcx, %rax
	movq	%r15, %rcx
	cmovaeq	%rdx, %rcx
	jmp	.LBB1966_47
	.p2align	4
.LBB1966_40:
	vmovdqa	144(%rsp), %xmm0
	cmpl	$1, (%rbx)
	vmovdqa	%xmm0, 48(%rsp)
	jne	.LBB1966_44
	movq	8(%rbx), %rdi
	cmpq	%r12, %rdi
	jae	.LBB1966_42
	vmovq	(%rbp,%rdi,8), %xmm0
	vmovdqa	%xmm0, 48(%rsp)
.LBB1966_44:
	movq	8(%rsp), %rsi
	movq	16(%rsp), %rax
	xorl	%edx, %edx
	leaq	16(%rsp), %rcx
	leaq	24(%rsp), %rdi
	decq	%rsi
	cmpq	$5, %rsi
	cmovbq	%rcx, %rax
	movq	%r15, %rcx
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	setae	%dl
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	8(%rsp,%rdx), %r14
	leaq	-1(%r14), %rdx
	cmpq	%rsi, %rdx
	je	.LBB1966_45
.LBB1966_47:
	vmovdqa	48(%rsp), %xmm0
	addq	$16, %rbx
	vmovq	%xmm0, -8(%rax,%r14,8)
	incq	%r14
	movq	%r14, (%rcx)
	cmpq	%r13, %rbx
	jne	.LBB1966_40
	jmp	.LBB1966_59
.LBB1966_22:
.Ltmp40775:
	movl	$1, %ecx
	movq	%r15, %rdi
	xorl	%esi, %esi
	movq	%r12, %rdx
	movq	%r11, %rbx
	vzeroupper
	callq	*%r11
.Ltmp40776:
	movq	8(%rsp), %rax
	xorl	%edx, %edx
	movl	$4, %ecx
	leaq	16(%rsp), %rsi
	leaq	24(%rsp), %rdi
	movq	%r15, %r8
	movq	%rbx, %r11
	decq	%rax
	cmpq	$5, %rax
	cmovbq	%rcx, %rax
	movq	16(%rsp), %rcx
	setae	%dl
	cmovaeq	%rdi, %r8
	cmovbq	%rsi, %rcx
	shll	$4, %edx
	movq	8(%rsp,%rdx), %r14
	jmp	.LBB1966_24
.LBB1966_60:
	movq	96(%rsp), %rdi
.LBB1966_61:
	movq	%r9, 88(%rsp)
	movq	%r9, 16(%rdi)
	movq	72(%rsp), %rax
	movq	80(%rsp), %rcx
	movq	%rax, (%rdi)
	movq	%rcx, 8(%rdi)
	addq	$200, %rsp
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
.LBB1966_42:
	.cfi_def_cfa_offset 256
.Ltmp40781:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.792(%rip), %rdx
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.Ltmp40782:
	jmp	.LBB1966_34
.LBB1966_33:
.Ltmp40778:
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.68dd637f94a7f528fe69f6876e3d956b.792(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp40779:
.LBB1966_34:
	ud2
.LBB1966_10:
	movl	$8, %edi
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r15, %rsi
	callq	*%rax
.LBB1966_52:
.Ltmp40777:
	jmp	.LBB1966_54
.LBB1966_51:
.Ltmp40786:
	jmp	.LBB1966_54
.LBB1966_50:
.Ltmp40780:
	movq	48(%rsp), %rcx
	movq	%rax, %rbx
	movq	%r14, (%rcx)
	jmp	.LBB1966_55
.LBB1966_53:
.Ltmp40783:
.LBB1966_54:
	movq	%rax, %rbx
.LBB1966_55:
	movq	8(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB1966_57
	movq	16(%rsp), %rdi
	leaq	-8(,%rax,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB1966_57:
	movq	(%rsp), %rax
	leaq	72(%rsp), %rdi
	movq	%rax, 88(%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1966:

purrdf_sparql_eval::modifier::eval_dedup_with::<purrdf_core::ir::dataset::RdfDataset, false>:
.Lfunc_begin360:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception267
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
.Ltmp15031:
	leaq	640(%rsp), %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	movq	%r15, %rcx
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15032:
	cmpl	$1, 640(%rsp)
	jne	.LBB360_25
	vmovdqu64	688(%rsp), %zmm1
	vmovdqu64	656(%rsp), %zmm0
	movq	576(%rsp), %rax
	vmovdqu64	%zmm1, 48(%rbx)
	vmovdqu64	%zmm0, 16(%rbx)
	movq	$1, (%rbx)
	cmpq	$6, %rax
	jb	.LBB360_12
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_5
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_5:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_11
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_5
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
.LBB360_8:
	cmpq	%rax, %rsi
	jge	.LBB360_10
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_8
.LBB360_10:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_11:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_12:
	movq	504(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB360_22
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_15
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_15:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_21
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_15
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
.LBB360_18:
	cmpq	%rax, %rsi
	jge	.LBB360_20
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_18
.LBB360_20:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_21:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_22:
	movq	600(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_267
	lock		decq	(%rax)
	jne	.LBB360_267
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	600(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
	jmp	.LBB360_267
.LBB360_25:
	vmovdqu64	680(%rsp), %zmm1
	vmovdqu64	648(%rsp), %zmm0
	vmovdqu64	%zmm1, 384(%rsp)
	vmovdqu64	%zmm0, 352(%rsp)
.Ltmp15033:
	leaq	112(%rsp), %rdi
	leaq	504(%rsp), %rsi
	leaq	352(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15034:
	cmpq	$-1, 112(%rsp)
	je	.LBB360_126
	vmovdqu	112(%rsp), %ymm0
	vmovdqu64	544(%rsp), %zmm1
	vmovdqu64	504(%rsp), %zmm2
	vmovdqu64	%zmm1, 392(%rsp)
	vmovdqu	%ymm0, 608(%rsp)
	vmovdqu64	%zmm2, 352(%rsp)
.Ltmp15038:
	leaq	320(%rsp), %rdi
	leaq	608(%rsp), %rsi
	vzeroupper
	callq	purrdf_sparql_eval::blank_scope::without_joined_blanks::<purrdf_core::ir::term::TermId>
.Ltmp15039:
	movq	336(%rsp), %r15
	movq	%rbx, 280(%rsp)
.Ltmp15041:
	leaq	112(%rsp), %rdi
	movl	$48, %esi
	movl	$1, %ecx
	movq	%r15, %rdx
	callq	<hashbrown::raw::RawTableInner>::fallible_with_capacity::<alloc::alloc::Global>
.Ltmp15042:
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
	je	.LBB360_99
	movq	(%rsp), %rbp
	xorl	%r15d, %r15d
	jmp	.LBB360_34
.LBB360_31:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_32:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_33:
	cmpq	24(%rsp), %rbp
	je	.LBB360_112
.LBB360_34:
	movq	%rbp, %rax
	movq	(%rax), %rbx
	addq	$40, %rbp
	testq	%rbx, %rbx
	je	.LBB360_99
	vmovups	8(%rax), %ymm0
	leaq	472(%rsp), %rdx
	leaq	-1(%rbx), %rcx
	movabsq	$2746377873070565055, %rsi
	cmpq	$5, %rcx
	vmovups	%ymm0, (%rdx)
	movq	%rbx, 464(%rsp)
	vpbroadcastq	.LCPI360_6(%rip), %xmm0
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
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	testq	%rax, %rax
	je	.LBB360_43
	leaq	(,%rax,8), %rsi
	addq	$-8, %rsi
	movl	%esi, %edi
	shrl	$3, %edi
	incl	%edi
	andl	$3, %edi
	je	.LBB360_41
	shll	$3, %edi
	movq	%rcx, %rdx
	jmp	.LBB360_39
	.p2align	4
.LBB360_38:
	addq	$8, %rdx
	addq	$-8, %rdi
	je	.LBB360_42
.LBB360_39:
	movl	(%rdx), %r8d
	xorl	%r9d, %r9d
	cmpq	$2, %r8
	setne	%r9b
	vmovd	%r9d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_38
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	4(%rdx), %r9d
	vmovq	%r8, %xmm1
	orl	$-2, %r8d
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%r8,%r9), %r8d
	vmovd	%r8d, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB360_38
	.p2align	4
.LBB360_41:
	movq	%rcx, %rdx
.LBB360_42:
	cmpq	$24, %rsi
	jae	.LBB360_84
.LBB360_43:
	vaesenc	.LCPI360_2(%rip), %xmm0, %xmm0
	leaq	1(%r15), %rdx
	movq	248(%rsp), %rsi
	movq	%rdx, 8(%rsp)
	movq	240(%rsp), %rdx
	vaesenc	.LCPI360_3(%rip), %xmm0, %xmm0
	vmovq	%xmm0, %r12
	movq	%r12, %r14
	shrq	$57, %r14
	vpbroadcastb	%r14d, %xmm0
	testq	%rax, %rax
	je	.LBB360_62
	xorl	%edi, %edi
	movq	%r12, %r8
.LBB360_45:
	andq	%rsi, %r8
	vmovdqu	(%rdx,%r8), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB360_60
	kmovd	%k0, %r9d
	movq	%r13, 16(%rsp)
	movq	%rdi, 32(%rsp)
.LBB360_47:
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
	jb	.LBB360_49
	movq	-32(%rdx,%rdi), %r11
	movq	-40(%rdx,%rdi), %r10
	decq	%r11
	jmp	.LBB360_50
	.p2align	4
.LBB360_49:
	leaq	-40(%rdx,%rdi), %r10
.LBB360_50:
	cmpq	%rax, %r11
	jne	.LBB360_59
	xorl	%r11d, %r11d
	jmp	.LBB360_53
	.p2align	4
.LBB360_52:
	incq	%r11
	cmpq	%r11, %rax
	je	.LBB360_71
.LBB360_53:
	movl	(%r10,%r11,8), %r13d
	movl	(%rcx,%r11,8), %edi
	cmpl	$2, %r13d
	je	.LBB360_57
	cmpl	$2, %edi
	je	.LBB360_57
	cmpl	%edi, %r13d
	jne	.LBB360_59
	movl	4(%rcx,%r11,8), %edi
	cmpl	%edi, 4(%r10,%r11,8)
	je	.LBB360_52
	jmp	.LBB360_59
	.p2align	4
.LBB360_57:
	cmpl	$2, %r13d
	jne	.LBB360_59
	cmpl	$2, %edi
	je	.LBB360_52
.LBB360_59:
	leal	-1(%r9), %edi
	movq	16(%rsp), %r13
	andw	%r9w, %di
	movl	%edi, %r9d
	movq	32(%rsp), %rdi
	jne	.LBB360_47
	.p2align	4
.LBB360_60:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB360_80
	leaq	16(%r8,%rdi), %r8
	addq	$16, %rdi
	jmp	.LBB360_45
	.p2align	4
.LBB360_62:
	xorl	%eax, %eax
	movq	%r12, %rcx
.LBB360_63:
	andq	%rsi, %rcx
	vmovdqu	(%rdx,%rcx), %xmm1
	vpcmpeqb	%xmm0, %xmm1, %k0
	kortestw	%k0, %k0
	je	.LBB360_69
	kmovd	%k0, %edi
	movq	%r13, 16(%rsp)
.LBB360_65:
	xorl	%r8d, %r8d
	tzcntl	%edi, %r8d
	addq	%rcx, %r8
	andq	%rsi, %r8
	negq	%r8
	leaq	(%r8,%r8,2), %r8
	shlq	$4, %r8
	movq	-48(%rdx,%r8), %r9
	cmpq	$6, %r9
	jb	.LBB360_67
	movq	-32(%rdx,%r8), %r9
.LBB360_67:
	cmpq	$1, %r9
	je	.LBB360_71
	movq	16(%rsp), %r13
	leal	-1(%rdi), %r8d
	andw	%di, %r8w
	movl	%r8d, %edi
	jne	.LBB360_65
	.p2align	4
.LBB360_69:
	vpcmpeqd	%xmm2, %xmm2, %xmm2
	vpcmpeqb	%xmm2, %xmm1, %k0
	kortestw	%k0, %k0
	jne	.LBB360_80
	leaq	16(%rcx,%rax), %rcx
	addq	$16, %rax
	jmp	.LBB360_63
	.p2align	4
.LBB360_71:
	movq	8(%rsp), %r15
	cmpq	$6, %rbx
	jb	.LBB360_33
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_74
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB360_74:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r8
	movq	16(%rsp), %rdi
	.p2align	4
.LBB360_75:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_32
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_75
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
.LBB360_78:
	cmpq	%rax, %rdx
	jge	.LBB360_31
	lock		cmpxchgq	%rdx, (%r8)
	jne	.LBB360_78
	jmp	.LBB360_31
	.p2align	4
.LBB360_80:
	cmpq	$0, 256(%rsp)
	je	.LBB360_96
.LBB360_81:
	andq	%rsi, %r12
	vmovdqu	(%rdx,%r12), %xmm0
	vpmovmskb	%xmm0, %eax
	testl	%eax, %eax
	je	.LBB360_94
.LBB360_82:
	tzcntl	%eax, %eax
	addq	%r12, %rax
	andq	%rsi, %rax
	movzbl	(%rdx,%rax), %ecx
	testb	%cl, %cl
	jns	.LBB360_98
.LBB360_83:
	movq	40(%rsp), %r8
	leaq	-16(%rax), %rdi
	movb	%r14b, (%rdx,%rax)
	negq	%rax
	vpbroadcastb	.LCPI360_7(%rip), %xmm1
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
	jmp	.LBB360_33
	.p2align	4
.LBB360_84:
	leaq	(%rcx,%rax,8), %rsi
	jmp	.LBB360_86
	.p2align	4
.LBB360_85:
	addq	$32, %rdx
	cmpq	%rsi, %rdx
	je	.LBB360_43
.LBB360_86:
	movl	(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_88
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	4(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB360_88:
	movl	8(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_90
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	12(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB360_90:
	movl	16(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_92
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	20(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
.LBB360_92:
	movl	24(%rdx), %edi
	xorl	%r8d, %r8d
	cmpq	$2, %rdi
	setne	%r8b
	vmovd	%r8d, %xmm1
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	.LCPI360_1(%rip), %xmm0, %xmm0
	je	.LBB360_85
	vmovdqa	.LCPI360_1(%rip), %xmm2
	movl	28(%rdx), %r8d
	vmovq	%rdi, %xmm1
	orl	$-2, %edi
	vpxor	%xmm0, %xmm1, %xmm0
	leal	1(%rdi,%r8), %edi
	vmovd	%edi, %xmm1
	vaesenc	%xmm2, %xmm0, %xmm0
	vpxor	%xmm0, %xmm1, %xmm0
	vaesenc	%xmm2, %xmm0, %xmm0
	jmp	.LBB360_85
.LBB360_94:
	movl	$16, %ecx
.LBB360_95:
	addq	%rcx, %r12
	addq	$16, %rcx
	andq	%rsi, %r12
	vmovdqu	(%rdx,%r12), %xmm0
	vpmovmskb	%xmm0, %eax
	testl	%eax, %eax
	jne	.LBB360_82
	jmp	.LBB360_95
.LBB360_96:
.Ltmp15044:
	movq	<hashbrown::raw::RawTable<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize)>>::reserve_rehash::<hashbrown::map::make_hasher<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>::{closure#0}>@GOTPCREL(%rip), %rax
	movl	$1, %esi
	leaq	240(%rsp), %rdi
	movl	$1, %ecx
	vzeroupper
	callq	*%rax
.Ltmp15045:
	movq	240(%rsp), %rdx
	movq	248(%rsp), %rsi
	jmp	.LBB360_81
.LBB360_98:
	vmovdqa	(%rdx), %xmm0
	vpmovmskb	%xmm0, %eax
	tzcntl	%eax, %eax
	movzbl	(%rdx,%rax), %ecx
	jmp	.LBB360_83
.LBB360_99:
	subq	%rbp, 24(%rsp)
	je	.LBB360_112
	movq	24(%rsp), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r12
	movq	free@GOTPCREL(%rip), %r13
	movabsq	$-3689348814741910323, %rax
	movabsq	$9223372036854775807, %r14
	xorl	%ebx, %ebx
	shrq	$3, %r15
	imulq	%rax, %r15
	jmp	.LBB360_104
	.p2align	4
.LBB360_101:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_102:
	vzeroupper
	callq	*%r13
.LBB360_103:
	incq	%rbx
	cmpq	%r15, %rbx
	je	.LBB360_112
.LBB360_104:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%rbp,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB360_103
	leaq	(%rbp,%rcx,8), %rdx
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
	jge	.LBB360_107
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_107:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_102
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_107
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
.LBB360_110:
	cmpq	%rax, %rdx
	jge	.LBB360_101
	lock		cmpxchgq	%rdx, (%r12)
	jne	.LBB360_110
	jmp	.LBB360_101
.LBB360_112:
	movq	208(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_122
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
	jge	.LBB360_115
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_115:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_121
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_115
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
.LBB360_118:
	cmpq	%rax, %rsi
	jge	.LBB360_120
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_118
.LBB360_120:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_121:
	movq	free@GOTPCREL(%rip), %rax
	movq	(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB360_122:
	movq	240(%rsp), %r15
	movq	248(%rsp), %rdi
	movq	264(%rsp), %rax
	testq	%rdi, %rdi
	je	.LBB360_127
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
	je	.LBB360_128
.LBB360_124:
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	vpcmpltb	(%r15), %xmm0, %k0
	leaq	16(%r15), %r14
	kortestw	%k0, %k0
	je	.LBB360_141
	kmovd	%k0, %ecx
	movq	%r15, %rbp
	jmp	.LBB360_144
.LBB360_126:
	leaq	8(%rbx), %rdi
	leaq	504(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
	jmp	.LBB360_266
.LBB360_127:
	xorl	%r8d, %r8d
	movq	%rsi, 40(%rsp)
	movq	%rdi, 8(%rsp)
	testq	%rax, %rax
	jne	.LBB360_124
.LBB360_128:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
.LBB360_129:
	cmpq	$0, 8(%rsp)
	movq	40(%rsp), %rsi
	movabsq	$-3689348814741910323, %r15
	je	.LBB360_140
	testq	%rsi, %rsi
	je	.LBB360_140
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_133
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_133:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_139
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_133
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
.LBB360_136:
	cmpq	%rax, %rdx
	jge	.LBB360_138
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_136
.LBB360_138:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_139:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB360_140:
	movl	$8, %r13d
	xorl	%r12d, %r12d
	jmp	.LBB360_214
.LBB360_141:
	movq	%r15, %rbp
	.p2align	4
.LBB360_142:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_142
	kmovd	%k0, %ecx
.LBB360_144:
	xorl	%r12d, %r12d
	blsrl	%ecx, %r12d
	tzcntl	%ecx, %ecx
	leaq	-1(%rax), %rbx
	negq	%rcx
	leaq	(%rcx,%rcx,2), %rcx
	shlq	$4, %rcx
	movq	-48(%rbp,%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB360_148
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
	jbe	.LBB360_164
	xorl	%edi, %edi
.LBB360_147:
.Ltmp15053:
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%r13, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15054:
	jmp	.LBB360_293
.LBB360_148:
	movq	$0, 48(%rsp)
	movq	$8, 56(%rsp)
	movq	$0, 64(%rsp)
	testq	%rbx, %rbx
	je	.LBB360_129
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB360_153
	.p2align	4
.LBB360_150:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_151:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB360_152:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB360_129
.LBB360_153:
	testw	%r12w, %r12w
	jne	.LBB360_156
	.p2align	4
.LBB360_154:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_154
	kmovd	%k0, %r12d
.LBB360_156:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_152
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_159
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_159:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_151
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_159
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
.LBB360_162:
	cmpq	%rax, %rdx
	jge	.LBB360_150
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_162
	jmp	.LBB360_150
.LBB360_164:
	testq	%r13, %r13
	je	.LBB360_175
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%rsi, 312(%rsp)
	movq	%r9, 208(%rsp)
	movq	%r8, (%rsp)
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB360_295
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	movabsq	$9223372036854775807, %rsi
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	%r13, %rax
	cmovbq	%rdx, %rax
	cmpq	%rsi, %r13
	movq	%rsi, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmovbq	%r13, %rdx
	addq	%rdx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB360_168
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB360_168:
	movq	(%rsp), %r8
	movq	208(%rsp), %r9
	movq	312(%rsp), %r10
	.p2align	4
.LBB360_169:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_176
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_169
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
.LBB360_172:
	cmpq	%rax, %rsi
	jle	.LBB360_174
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB360_172
.LBB360_174:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jmp	.LBB360_176
.LBB360_175:
	movl	$8, %ecx
	xorl	%r10d, %r10d
.LBB360_176:
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
	je	.LBB360_202
	movl	$1, %r15d
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	jmp	.LBB360_179
	.p2align	4
.LBB360_178:
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
	je	.LBB360_202
.LBB360_179:
	testw	%r12w, %r12w
	jne	.LBB360_182
	.p2align	4
.LBB360_180:
	vpcmpltb	(%r14), %xmm1, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_180
	kmovd	%k0, %r12d
.LBB360_182:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	decq	%rbx
	blsrl	%r12d, %r12d
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %r13
	testq	%r13, %r13
	je	.LBB360_186
	addq	%rbp, %rax
	movq	-16(%rax), %rdx
	movq	-8(%rax), %rdi
	movq	-40(%rax), %rsi
	movq	%rdx, 96(%rsp)
	vmovups	-32(%rax), %xmm0
	vmovaps	%xmm0, 80(%rsp)
	cmpq	216(%rsp), %r15
	jne	.LBB360_178
	movq	%rbx, %rdx
	incq	%rdx
	movq	$-1, %rax
	movq	%rdi, 16(%rsp)
	movq	%rsi, 24(%rsp)
	cmoveq	%rax, %rdx
.Ltmp15047:
	movl	$8, %ecx
	movl	$48, %r8d
	leaq	216(%rsp), %rdi
	movq	%r15, %rsi
	vzeroupper
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)
.Ltmp15048:
	movq	224(%rsp), %rcx
	movq	24(%rsp), %rsi
	movq	16(%rsp), %rdi
	vpcmpeqd	%xmm1, %xmm1, %xmm1
	jmp	.LBB360_178
.LBB360_186:
	movq	%r14, 144(%rsp)
	movq	%rbp, 136(%rsp)
	testq	%rbx, %rbx
	je	.LBB360_202
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB360_191
.LBB360_188:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_189:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB360_190:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB360_202
.LBB360_191:
	testw	%r12w, %r12w
	jne	.LBB360_194
	.p2align	4
.LBB360_192:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_192
	kmovd	%k0, %r12d
.LBB360_194:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_190
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_197
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_197:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_189
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_197
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
.LBB360_200:
	cmpq	%rax, %rdx
	jge	.LBB360_188
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_200
	jmp	.LBB360_188
.LBB360_202:
	cmpq	$0, 8(%rsp)
	movq	40(%rsp), %rsi
	movabsq	$-3689348814741910323, %r15
	je	.LBB360_213
	testq	%rsi, %rsi
	je	.LBB360_213
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_206
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_206:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_212
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_206
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
.LBB360_209:
	cmpq	%rax, %rdx
	jge	.LBB360_211
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_209
.LBB360_211:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_212:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB360_213:
	vmovdqu	216(%rsp), %xmm0
	movq	232(%rsp), %r12
	movq	%r12, 64(%rsp)
	vmovdqa	%xmm0, 48(%rsp)
	movq	56(%rsp), %r13
	cmpq	$2, %r12
	jae	.LBB360_268
.LBB360_214:
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
	je	.LBB360_222
	addq	$-48, %rax
	movabsq	$-6148914691236517205, %rsi
	movq	%rax, %rdx
	mulxq	%rsi, %rdx, %rdx
	shrl	$5, %edx
	incl	%edx
	andl	$7, %edx
	je	.LBB360_219
	shll	$3, %edx
	movq	%r13, %r14
	leaq	(%rdx,%rdx,4), %rsi
	movq	%r13, %rdx
	.p2align	4
.LBB360_217:
	vmovdqu	8(%rdx), %ymm0
	movq	40(%rdx), %rdi
	addq	$48, %rdx
	movq	%rdi, 32(%r14)
	vmovdqu	%ymm0, (%r14)
	addq	$40, %r14
	addq	$-40, %rsi
	jne	.LBB360_217
	movq	%rcx, %rbx
	cmpq	$336, %rax
	jae	.LBB360_220
	jmp	.LBB360_222
.LBB360_219:
	movq	%r13, %r14
	movq	%r13, %rdx
	movq	%rcx, %rbx
	cmpq	$336, %rax
	jb	.LBB360_222
	.p2align	4
.LBB360_220:
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
	jne	.LBB360_220
	movq	%rcx, %rbx
.LBB360_222:
	vmovdqa	.LCPI360_5(%rip), %ymm0
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
	je	.LBB360_235
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	shrq	$4, %rcx
	movabsq	$-6148914691236517205, %r12
	movabsq	$9223372036854775807, %rbp
	xorl	%r15d, %r15d
	imulq	%rcx, %r12
	jmp	.LBB360_227
	.p2align	4
.LBB360_224:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_225:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_226:
	incq	%r15
	cmpq	%r12, %r15
	je	.LBB360_235
.LBB360_227:
	leaq	(%r15,%r15,2), %rax
	shlq	$4, %rax
	movq	8(%rbx,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_226
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_230
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_230:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_225
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_230
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
.LBB360_233:
	cmpq	%rax, %rdx
	jge	.LBB360_224
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_233
	jmp	.LBB360_224
.LBB360_235:
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
	jne	.LBB360_248
	cmpq	$39, %rcx
	ja	.LBB360_249
	movl	$8, %r12d
	testq	%rcx, %rcx
	je	.LBB360_250
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rsi
	cmpq	%rcx, %rax
	setns	%dl
	addq	%rsi, %rdx
	subq	%rcx, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_240
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB360_240:
	movq	32(%rsp), %rdi
	.p2align	4
.LBB360_241:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_247
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_241
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
.LBB360_244:
	cmpq	%rax, %rdx
	jge	.LBB360_246
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_244
.LBB360_246:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_247:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	jmp	.LBB360_250
.LBB360_248:
	movq	32(%rsp), %r12
	jmp	.LBB360_250
.LBB360_249:
	movq	<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rsi
	leaq	qualification_454_native_cost::GLOBAL (.llvm.1577329767756756036)(%rip), %rdi
	movl	$8, %edx
	movq	%r13, %r8
	vzeroupper
	callq	*%rax
	movq	%rax, %r12
	testq	%rax, %rax
	je	.LBB360_292
.LBB360_250:
	movq	8(%rsp), %rax
	cmpq	$-1, 352(%rsp)
	movq	%rax, 104(%rsp)
	movq	%rbx, 80(%rsp)
	movq	%r12, 88(%rsp)
	movq	%r14, 96(%rsp)
	je	.LBB360_252
	leaq	112(%rsp), %rdi
	leaq	80(%rsp), %rsi
	leaq	352(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	280(%rsp), %rbx
	movq	424(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB360_253
	jmp	.LBB360_262
.LBB360_252:
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
	jb	.LBB360_262
.LBB360_253:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_255
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_255:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_261
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_255
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
.LBB360_258:
	cmpq	%rax, %rsi
	jge	.LBB360_260
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_258
.LBB360_260:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_261:
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB360_262:
	movq	448(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_265
	lock		decq	(%rax)
	jne	.LBB360_265
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	448(%rsp), %rdi
	#MEMBARRIER
	vzeroupper
	callq	*%rax
.LBB360_265:
	vmovdqu64	144(%rsp), %zmm1
	vmovdqu64	112(%rsp), %zmm0
	vmovdqu64	%zmm1, 40(%rbx)
	vmovdqu64	%zmm0, 8(%rbx)
.LBB360_266:
	movq	$0, (%rbx)
.LBB360_267:
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
.LBB360_268:
	.cfi_def_cfa_offset 816
	cmpq	$21, %r12
	jae	.LBB360_294
	shlq	$4, %r12
	movabsq	$-6148914691236517205, %rcx
	leaq	48(%r13), %rax
	leaq	-96(%r12,%r12,2), %rdx
	mulxq	%rcx, %rcx, %rcx
	btl	$5, %ecx
	jb	.LBB360_273
	movq	48(%r13), %rcx
	cmpq	(%r13), %rcx
	jae	.LBB360_272
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
.LBB360_272:
	leaq	96(%r13), %rcx
	jmp	.LBB360_274
.LBB360_273:
	movq	%rax, %rcx
	movq	%r13, %rax
.LBB360_274:
	cmpq	$48, %rdx
	jae	.LBB360_276
.LBB360_275:
	movq	56(%rsp), %r13
	movq	64(%rsp), %r12
	jmp	.LBB360_214
.LBB360_276:
	leaq	(%r12,%r12,2), %rdx
	addq	%r13, %rdx
	jmp	.LBB360_279
.LBB360_277:
	movq	%rsi, (%rdi)
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 8(%rdi)
	movq	144(%rsp), %rsi
	movq	%rsi, 40(%rdi)
.LBB360_278:
	addq	$96, %rcx
	cmpq	%rdx, %rcx
	je	.LBB360_275
.LBB360_279:
	movq	(%rcx), %rsi
	cmpq	(%rax), %rsi
	jae	.LBB360_280
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
	je	.LBB360_286
.LBB360_282:
	cmpq	-48(%rax), %rsi
	jae	.LBB360_285
	leaq	-48(%rax), %rdi
	vmovdqu	(%rdi), %ymm0
	vmovdqu	16(%rdi), %ymm1
	vmovdqu	%ymm1, 16(%rax)
	vmovdqu	%ymm0, (%rax)
	movq	%rdi, %rax
	cmpq	%r13, %rdi
	jne	.LBB360_282
	movq	%r13, %rdi
	jmp	.LBB360_286
.LBB360_280:
	movq	48(%rcx), %rsi
	leaq	48(%rcx), %rax
	cmpq	(%rcx), %rsi
	jae	.LBB360_278
	jmp	.LBB360_287
.LBB360_285:
	movq	%rax, %rdi
.LBB360_286:
	movq	%rsi, (%rdi)
	vmovdqu	112(%rsp), %ymm0
	vmovdqu	%ymm0, 8(%rdi)
	movq	144(%rsp), %rax
	movq	%rax, 40(%rdi)
	movq	48(%rcx), %rsi
	leaq	48(%rcx), %rax
	cmpq	(%rcx), %rsi
	jae	.LBB360_278
.LBB360_287:
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
	je	.LBB360_277
	movq	%rcx, %rdi
.LBB360_289:
	cmpq	-48(%rdi), %rsi
	jae	.LBB360_277
	leaq	-48(%rdi), %r8
	vmovdqu	(%r8), %ymm0
	vmovdqu	16(%r8), %ymm1
	vmovdqu	%ymm1, 16(%rdi)
	vmovdqu	%ymm0, (%rdi)
	movq	%r8, %rdi
	cmpq	%r13, %r8
	jne	.LBB360_289
	movq	%r13, %rdi
	jmp	.LBB360_277
.LBB360_292:
.Ltmp15058:
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movq	%r13, %rsi
	callq	*%rax
.Ltmp15059:
.LBB360_293:
	ud2
.LBB360_294:
.Ltmp15050:
	movq	core::slice::sort::unstable::ipnsort::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), <[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId, false>::{closure#1}>::{closure#0}>@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	vzeroupper
	callq	*%rax
.Ltmp15051:
	jmp	.LBB360_214
.LBB360_295:
	movl	$8, %edi
	jmp	.LBB360_147
.LBB360_296:
.Ltmp15052:
	leaq	48(%rsp), %rdi
	movq	%rax, (%rsp)
	jmp	.LBB360_303
.LBB360_297:
.Ltmp15046:
	movq	8(%rsp), %rcx
	movq	%rbp, 120(%rsp)
	movq	%rax, (%rsp)
	movq	%rcx, 144(%rsp)
	cmpq	$6, %rbx
	jb	.LBB360_299
	leaq	-8(,%rbx,8), %rsi
	movl	$4, %edx
	movq	%r13, %rdi
	callq	__rustc::__rust_dealloc
.LBB360_299:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
	leaq	240(%rsp), %rdi
	callq	core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>>
	jmp	.LBB360_331
.LBB360_300:
.Ltmp15049:
	movq	%r14, 144(%rsp)
	movq	%rbp, 136(%rsp)
	movw	%r12w, 160(%rsp)
	movq	%rax, (%rsp)
	movq	%rbx, 168(%rsp)
	cmpq	$6, %r13
	jb	.LBB360_302
	movq	24(%rsp), %rdi
	leaq	-8(,%r13,8), %rsi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
.LBB360_302:
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId, false>::{closure#0}>>
	leaq	216(%rsp), %rdi
.LBB360_303:
	callq	core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>>
	jmp	.LBB360_331
.LBB360_304:
.Ltmp15043:
	movb	$1, %bl
	movq	%rax, (%rsp)
	jmp	.LBB360_332
.LBB360_305:
.Ltmp15040:
	movq	%rax, (%rsp)
	jmp	.LBB360_336
.LBB360_306:
.Ltmp15035:
	movq	%rax, (%rsp)
.Ltmp15036:
	leaq	504(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15037:
	jmp	.LBB360_359
.LBB360_307:
.Ltmp15060:
	leaq	80(%rsp), %rdi
	movq	%rax, (%rsp)
	callq	core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	leaq	112(%rsp), %rdi
	callq	core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId, false>::{closure#2}>>
	movq	8(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB360_336
	#MEMBARRIER
.Ltmp15061:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	288(%rsp), %rdi
	callq	*%rax
.Ltmp15062:
	jmp	.LBB360_336
.LBB360_309:
.Ltmp15055:
	movq	%rax, (%rsp)
	movq	24(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB360_319
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
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
	jge	.LBB360_312
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_312:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_318
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_312
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
.LBB360_315:
	cmpq	%rax, %rsi
	jge	.LBB360_317
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB360_315
.LBB360_317:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_318:
	movq	free@GOTPCREL(%rip), %rax
	movq	16(%rsp), %rdi
	callq	*%rax
.LBB360_319:
	testq	%rbx, %rbx
	jne	.LBB360_360
.LBB360_320:
	xorl	%ebx, %ebx
	cmpq	$0, 8(%rsp)
	je	.LBB360_332
	movq	40(%rsp), %rsi
	testq	%rsi, %rsi
	je	.LBB360_332
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	xorl	%edx, %edx
	movabsq	$9223372036854775807, %rcx
	cmpq	%rsi, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%rsi, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB360_324
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_324:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_330
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_324
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
.LBB360_327:
	cmpq	%rax, %rdx
	jge	.LBB360_329
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_327
.LBB360_329:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_330:
	movq	free@GOTPCREL(%rip), %rax
	movq	32(%rsp), %rdi
	callq	*%rax
.LBB360_331:
	xorl	%ebx, %ebx
.LBB360_332:
	movq	344(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB360_334
	leaq	344(%rsp), %rdi
	#MEMBARRIER
.Ltmp15056:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15057:
.LBB360_334:
	testb	%bl, %bl
	je	.LBB360_336
	leaq	320(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB360_336:
	movq	424(%rsp), %rax
	movabsq	$9223372036854775807, %rbx
	cmpq	$6, %rax
	jb	.LBB360_337
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	432(%rsp), %rdi
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
	jge	.LBB360_341
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_341:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_347
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_341
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
.LBB360_344:
	cmpq	%rax, %rdx
	jge	.LBB360_346
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_344
.LBB360_346:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_347:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB360_348
.LBB360_338:
	movq	448(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB360_357
	jmp	.LBB360_359
.LBB360_337:
	movq	352(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB360_338
.LBB360_348:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	360(%rsp), %rdi
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
	jge	.LBB360_350
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_350:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_356
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_350
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
.LBB360_353:
	cmpq	%rax, %rdx
	jge	.LBB360_355
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB360_353
.LBB360_355:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_356:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	448(%rsp), %rax
	testq	%rax, %rax
	je	.LBB360_359
.LBB360_357:
	lock		decq	(%rax)
	jne	.LBB360_359
	leaq	448(%rsp), %rdi
	#MEMBARRIER
.Ltmp15064:
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15065:
.LBB360_359:
	movq	(%rsp), %rdi
	callq	_Unwind_Resume@PLT
.LBB360_360:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %r13
	vpcmpeqd	%xmm0, %xmm0, %xmm0
	movabsq	$9223372036854775807, %r15
	jmp	.LBB360_364
	.p2align	4
.LBB360_361:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB360_362:
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	vpcmpeqd	%xmm0, %xmm0, %xmm0
.LBB360_363:
	blsrl	%r12d, %r12d
	decq	%rbx
	je	.LBB360_320
.LBB360_364:
	testw	%r12w, %r12w
	jne	.LBB360_367
	.p2align	4
.LBB360_365:
	vpcmpltb	(%r14), %xmm0, %k0
	addq	$-768, %rbp
	addq	$16, %r14
	kortestw	%k0, %k0
	je	.LBB360_365
	kmovd	%k0, %r12d
.LBB360_367:
	xorl	%eax, %eax
	tzcntl	%r12d, %eax
	negq	%rax
	leaq	(%rax,%rax,2), %rax
	shlq	$4, %rax
	movq	-48(%rbp,%rax), %rcx
	cmpq	$6, %rcx
	jb	.LBB360_363
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
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
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	jge	.LBB360_370
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB360_370:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB360_362
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB360_370
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
.LBB360_373:
	cmpq	%rax, %rdx
	jge	.LBB360_361
	lock		cmpxchgq	%rdx, (%r13)
	jne	.LBB360_373
	jmp	.LBB360_361
.LBB360_375:
.Ltmp15063:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB360_376:
.Ltmp15066:
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end360:

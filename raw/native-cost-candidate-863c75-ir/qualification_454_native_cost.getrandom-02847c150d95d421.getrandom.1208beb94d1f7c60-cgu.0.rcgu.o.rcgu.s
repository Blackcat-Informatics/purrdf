	.att_syntax
	.file	"getrandom.1208beb94d1f7c60-cgu.0"
	.section	.text._RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback17use_file_fallback,"ax",@progbits
	.globl	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback17use_file_fallback
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback17use_file_fallback,@function
_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback17use_file_fallback:
.Lfunc_begin0:
	.cfi_startproc
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movl	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD(%rip), %ebp
	movq	%rsi, %rbx
	movq	%rdi, %r14
	cmpl	$-2, %ebp
	jae	.LBB0_13
.LBB0_1:
	testq	%rbx, %rbx
	je	.LBB0_9
	movq	read@GOTPCREL(%rip), %r15
	movq	__errno_location@GOTPCREL(%rip), %r12
	movl	$65537, %r13d
.LBB0_3:
	movl	%ebp, %edi
	movq	%r14, %rsi
	movq	%rbx, %rdx
	callq	*%r15
	testq	%rax, %rax
	jle	.LBB0_7
	subq	%rax, %rbx
	jb	.LBB0_12
	addq	%rax, %r14
.LBB0_6:
	testq	%rbx, %rbx
	jne	.LBB0_3
	jmp	.LBB0_9
	.p2align	4
.LBB0_7:
	cmpq	$-1, %rax
	jne	.LBB0_12
	callq	*%r12
	movl	(%rax), %ecx
	movl	%ecx, %eax
	negl	%eax
	testl	%ecx, %ecx
	cmovlel	%r13d, %eax
	cmpl	$-4, %eax
	jne	.LBB0_10
	jmp	.LBB0_6
.LBB0_9:
	xorl	%eax, %eax
.LBB0_10:
	addq	$8, %rsp
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
.LBB0_12:
	.cfi_def_cfa_offset 64
	movl	$65538, %eax
	jmp	.LBB0_10
.LBB0_13:
	callq	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait
	movl	%eax, %ecx
	movl	%edx, %ebp
	movl	%edx, %eax
	testb	$1, %cl
	je	.LBB0_1
	jmp	.LBB0_10
.Lfunc_end0:
	.size	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback17use_file_fallback, .Lfunc_end0-_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback17use_file_fallback
	.cfi_endproc

	.section	.text.unlikely._RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback4init,"ax",@progbits
	.globl	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback4init
	.prefalign	4, .Lfunc_end1, nop
	.type	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback4init,@function
_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback4init:
.Lfunc_begin1:
	.cfi_startproc
	pushq	%rbx
	.cfi_def_cfa_offset 16
	.cfi_offset %rbx, -16
	movq	dlsym@GOTPCREL(%rip), %rax
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.2(%rip), %rsi
	xorl	%edi, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB1_7
	movl	$1, %edi
	xorl	%esi, %esi
	xorl	%edx, %edx
	movq	%rax, %rbx
	callq	*%rax
	testq	%rax, %rax
	js	.LBB1_3
	movq	%rbx, %rax
.LBB1_8:
	movq	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN@GOTPCREL(%rip), %rcx
	movq	%rax, (%rcx)
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB1_3:
	.cfi_def_cfa_offset 16
	movq	__errno_location@GOTPCREL(%rip), %rax
	callq	*%rax
	movl	(%rax), %eax
	movl	$65537, %ecx
	movl	%eax, %edx
	negl	%edx
	testl	%eax, %eax
	cmovgl	%edx, %ecx
	testl	%ecx, %ecx
	js	.LBB1_5
	movq	%rbx, %rax
	movq	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN@GOTPCREL(%rip), %rcx
	movq	%rax, (%rcx)
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.LBB1_5:
	.cfi_def_cfa_offset 16
	cmpl	$-1, %ecx
	je	.LBB1_7
	movq	%rbx, %rax
	cmpl	$-38, %ecx
	jne	.LBB1_8
.LBB1_7:
	movq	$-1, %rax
	movq	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN@GOTPCREL(%rip), %rcx
	movq	%rax, (%rcx)
	popq	%rbx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end1:
	.size	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback4init, .Lfunc_end1-_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback4init
	.cfi_endproc

	.section	.text.unlikely._RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait,"ax",@progbits
	.prefalign	4, .Lfunc_end2, nop
	.type	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait,@function
_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait:
.Lfunc_begin2:
	.cfi_startproc
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
	pushq	%rax
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	syscall@GOTPCREL(%rip), %r15
	leaq	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD(%rip), %r14
	movl	$-2, %ebp
	jmp	.LBB2_1
	.p2align	4
.LBB2_19:
	movl	$202, %edi
	movl	$128, %edx
	movl	$-2, %ecx
	movq	%r14, %rsi
	xorl	%r8d, %r8d
	xorl	%eax, %eax
	callq	*%r15
.LBB2_1:
	movl	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD(%rip), %ebx
	cmpl	$-2, %ebx
	je	.LBB2_19
	cmpl	$-1, %ebx
	jne	.LBB2_17
	movl	$-1, %eax
	lock		cmpxchgl	%ebp, _RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD(%rip)
	jne	.LBB2_1
	movq	__errno_location@GOTPCREL(%rip), %r13
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.1(%rip), %r14
	movl	$65537, %r12d
	movl	$1, %ebp
.LBB2_5:
	movq	open@GOTPCREL(%rip), %rcx
	movl	$524288, %esi
	movq	%r14, %rdi
	xorl	%eax, %eax
	callq	*%rcx
	testl	%eax, %eax
	jns	.LBB2_7
	callq	*%r13
	movl	(%rax), %eax
	movl	%eax, %ebx
	negl	%ebx
	testl	%eax, %eax
	cmovlel	%r12d, %ebx
	cmpl	$-4, %ebx
	je	.LBB2_5
.LBB2_16:
	testl	%ebp, %ebp
	movl	$-1, %eax
	leaq	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD(%rip), %rsi
	movl	$202, %edi
	movl	$129, %edx
	movl	$2147483647, %ecx
	cmovel	%ebx, %eax
	movl	%eax, _RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD(%rip)
	xorl	%eax, %eax
	callq	*%r15
	jmp	.LBB2_18
.LBB2_17:
	xorl	%ebp, %ebp
.LBB2_18:
	movl	%ebp, %eax
	movl	%ebx, %edx
	addq	$8, %rsp
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
.LBB2_7:
	.cfi_def_cfa_offset 64
	movl	$65537, %r12d
	movl	%eax, (%rsp)
	movl	%eax, %ebp
	movq	%rsp, %r14
	movl	$1, 4(%rsp)
.LBB2_8:
	movq	poll@GOTPCREL(%rip), %rax
	movl	$1, %esi
	movl	$-1, %edx
	movq	%r14, %rdi
	callq	*%rax
	testl	%eax, %eax
	jns	.LBB2_11
	callq	*%r13
	movl	(%rax), %eax
	movl	%eax, %ebx
	negl	%ebx
	testl	%eax, %eax
	cmovlel	%r12d, %ebx
	cmpl	$-4, %ebx
	je	.LBB2_8
	movq	close@GOTPCREL(%rip), %rax
	movl	%ebp, %edi
	callq	*%rax
	movl	$1, %ebp
	jmp	.LBB2_16
.LBB2_11:
	movq	close@GOTPCREL(%rip), %rax
	movl	%ebp, %edi
	callq	*%rax
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.0(%rip), %r14
	movl	$65537, %r12d
.LBB2_12:
	movq	open@GOTPCREL(%rip), %rcx
	movl	$524288, %esi
	movq	%r14, %rdi
	xorl	%eax, %eax
	callq	*%rcx
	movl	%eax, %ebp
	testl	%eax, %eax
	jns	.LBB2_13
	callq	*%r13
	movl	(%rax), %eax
	movl	%eax, %ebx
	negl	%ebx
	testl	%eax, %eax
	cmovlel	%r12d, %ebx
	cmpl	$-4, %ebx
	je	.LBB2_12
.LBB2_15:
	shrl	$31, %ebp
	jmp	.LBB2_16
.LBB2_13:
	movl	%ebp, %ebx
	jmp	.LBB2_15
.Lfunc_end2:
	.size	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait, .Lfunc_end2-_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file12open_or_wait
	.cfi_endproc

	.section	.text._RNvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc13last_os_error,"ax",@progbits
	.globl	_RNvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc13last_os_error
	.prefalign	4, .Lfunc_end3, nop
	.type	_RNvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc13last_os_error,@function
_RNvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc13last_os_error:
.Lfunc_begin3:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	movq	__errno_location@GOTPCREL(%rip), %rax
	callq	*%rax
	movl	(%rax), %eax
	movl	%eax, %ecx
	negl	%ecx
	testl	%eax, %eax
	movl	$65537, %eax
	cmovgl	%ecx, %eax
	popq	%rcx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end3:
	.size	_RNvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc13last_os_error, .Lfunc_end3-_RNvNtNtNtCs1xZImt19Kx0_9getrandom8backends8use_file9util_libc13last_os_error
	.cfi_endproc

	.section	.text._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom,"ax",@progbits
	.prefalign	4, .Lfunc_end4, nop
	.type	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom,@function
_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom:
.Lfunc_begin4:
	.cfi_startproc
	movq	%rsi, %rdx
	movq	(%rdi), %rax
	movq	8(%rdi), %rsi
	movq	%rax, %rdi
	jmpq	*_RNvXsh_NtCs2k2z8Zem4rB_4core3fmteNtB5_5Debug3fmt@GOTPCREL(%rip)
.Lfunc_end4:
	.size	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom, .Lfunc_end4-_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom
	.cfi_endproc

	.section	.text._RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt,"ax",@progbits
	.prefalign	4, .Lfunc_end5, nop
	.type	_RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt,@function
_RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt:
.Lfunc_begin5:
	.cfi_startproc
	pushq	%rax
	.cfi_def_cfa_offset 16
	movl	16(%rsi), %eax
	testl	$33554432, %eax
	jne	.LBB5_3
	testl	$67108864, %eax
	jne	.LBB5_5
	popq	%rax
	.cfi_def_cfa_offset 8
	jmpq	*_RNvXs9_NtNtNtCs2k2z8Zem4rB_4core3fmt3num3implNtB9_7Display3fmt@GOTPCREL(%rip)
.LBB5_3:
	.cfi_def_cfa_offset 16
	movl	(%rdi), %edx
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.2.llvm.9794848731438112354(%rip), %rax
	xorl	%r9d, %r9d
	movl	%edx, %ecx
	.p2align	4
.LBB5_4:
	andl	$15, %edx
	shrl	$4, %ecx
	movzbl	(%rdx,%rax), %edx
	movb	%dl, 7(%rsp,%r9)
	decq	%r9
	movl	%ecx, %edx
	testl	%ecx, %ecx
	jne	.LBB5_4
	jmp	.LBB5_7
.LBB5_5:
	movl	(%rdi), %edx
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.408.llvm.9794848731438112354(%rip), %rax
	xorl	%r9d, %r9d
	movl	%edx, %ecx
	.p2align	4
.LBB5_6:
	andl	$15, %edx
	shrl	$4, %ecx
	movzbl	(%rdx,%rax), %edx
	movb	%dl, 7(%rsp,%r9)
	decq	%r9
	movl	%ecx, %edx
	testl	%ecx, %ecx
	jne	.LBB5_6
.LBB5_7:
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter12pad_integral@GOTPCREL(%rip), %rax
	leaq	8(%rsp,%r9), %r8
	movq	%rsi, %rdi
	negq	%r9
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.394.llvm.9794848731438112354(%rip), %rdx
	movl	$2, %ecx
	movl	$1, %esi
	callq	*%rax
	popq	%rcx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end5:
	.size	_RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt, .Lfunc_end5-_RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt
	.cfi_endproc

	.section	.text._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt,"ax",@progbits
	.globl	_RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt
	.prefalign	4, .Lfunc_end6, nop
	.type	_RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt,@function
_RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt:
.Lfunc_begin6:
	.cfi_startproc
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	subq	$40, %rsp
	.cfi_def_cfa_offset 64
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movq	8(%rsi), %rax
	movq	%rdi, %r14
	movq	(%rsi), %rdi
	movq	%rsi, %rbx
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.8(%rip), %rsi
	movl	$5, %edx
	movq	24(%rax), %rax
	callq	*%rax
	movq	%rbx, (%rsp)
	movb	%al, 8(%rsp)
	movl	(%r14), %eax
	movb	$0, 9(%rsp)
	cmpl	$-2147483647, %eax
	jae	.LBB6_3
	leal	-65536(%rax), %ecx
	cmpl	$3, %ecx
	jae	.LBB6_4
	leaq	.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel(%rip), %rdx
	movl	%ecx, %ecx
	movq	_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field@GOTPCREL(%rip), %r14
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.9(%rip), %r8
	movq	%rsp, %rbx
	movl	%eax, 20(%rsp)
	movq	%rbx, %rdi
	movslq	(%rdx,%rcx,4), %rsi
	addq	%rdx, %rsi
	leaq	.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.11(%rip), %rdx
	movzbl	(%rcx,%rdx), %ecx
	movq	%rsi, 24(%rsp)
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.11(%rip), %rsi
	movl	$13, %edx
	movq	%rcx, 32(%rsp)
	leaq	20(%rsp), %rcx
	callq	*%r14
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.13(%rip), %rsi
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.12(%rip), %r8
	leaq	24(%rsp), %rcx
	movl	$11, %edx
	movq	%rbx, %rdi
	callq	*%r14
	jmp	.LBB6_6
.LBB6_3:
	negl	%eax
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.10(%rip), %rsi
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.9(%rip), %r8
	leaq	24(%rsp), %rcx
	movl	$8, %edx
	movq	%rsp, %rdi
	movl	%eax, 24(%rsp)
	movq	_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field@GOTPCREL(%rip), %rax
	jmp	.LBB6_5
.LBB6_4:
	movl	%eax, 24(%rsp)
	movq	_RNvMs2_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_11DebugStruct5field@GOTPCREL(%rip), %rax
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.14(%rip), %rsi
	leaq	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.9(%rip), %r8
	movq	%rsp, %rdi
	leaq	24(%rsp), %rcx
	movl	$12, %edx
.LBB6_5:
	callq	*%rax
.LBB6_6:
	movzbl	9(%rsp), %eax
	movzbl	8(%rsp), %ecx
	movl	%eax, %edx
	notb	%dl
	orb	%cl, %dl
	testb	$1, %dl
	je	.LBB6_8
	orb	%cl, %al
	jmp	.LBB6_13
.LBB6_8:
	movq	(%rsp), %rax
	testb	$-128, 18(%rax)
	jne	.LBB6_10
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.91.llvm.9794848731438112354(%rip), %rsi
	movl	$2, %edx
	jmp	.LBB6_11
.LBB6_10:
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.37.llvm.9794848731438112354(%rip), %rsi
	movl	$1, %edx
.LBB6_11:
	movq	24(%rax), %rax
	callq	*%rax
.LBB6_13:
	andb	$1, %al
	addq	$40, %rsp
	.cfi_def_cfa_offset 24
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end6:
	.size	_RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt, .Lfunc_end6-_RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt
	.cfi_endproc

	.type	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN,@object
	.section	.bss._RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN,"aw",@nobits
	.globl	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN
	.p2align	3, 0x0
_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN:
	.zero	8
	.size	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends27linux_android_with_fallback12GETRANDOM_FN, 8

	.type	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD,@object
	.section	.data._RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD,"aw",@progbits
	.p2align	2, 0x0
_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD:
	.zero	4,255
	.size	_RNvNtNtCs1xZImt19Kx0_9getrandom8backends8use_file2FD, 4

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.0,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.0:
	.asciz	"/dev/urandom"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.0, 13

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.1,@object
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.1:
	.asciz	"/dev/random"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.1, 12

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.2,@object
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.2:
	.asciz	"getrandom"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.2, 10

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.4,@object
	.section	.rodata..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.4,"a",@progbits
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.4:
	.ascii	"getrandom: this target is not supported"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.4, 39

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.5,@object
	.section	.rodata..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.5,"a",@progbits
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.5:
	.ascii	"errno: did not return a positive value"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.5, 38

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.6,@object
	.section	.rodata..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.6,"a",@progbits
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.6:
	.ascii	"unexpected situation"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.6, 20

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.8,@object
	.section	.rodata..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.8,"a",@progbits
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.8:
	.ascii	"Error"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.8, 5

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.9,@object
	.section	.data.rel.ro..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.9,"aw",@progbits
	.p2align	3, 0x0
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.9:
	.asciz	"\000\000\000\000\000\000\000\000\004\000\000\000\000\000\000\000\004\000\000\000\000\000\000"
	.quad	_RNvXsQ_NtNtCs2k2z8Zem4rB_4core3fmt3numlNtB7_5Debug3fmt
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.9, 32

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.10,@object
	.section	.rodata.cst8,"aM",@progbits,8
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.10:
	.ascii	"os_error"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.10, 8

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.11,@object
	.section	.rodata..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.11,"a",@progbits
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.11:
	.ascii	"internal_code"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.11, 13

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.12,@object
	.section	.data.rel.ro..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.12,"aw",@progbits
	.p2align	3, 0x0
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.12:
	.asciz	"\000\000\000\000\000\000\000\000\020\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs1xZImt19Kx0_9getrandom
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.12, 32

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.13,@object
	.section	.rodata..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.13,"a",@progbits
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.13:
	.ascii	"description"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.13, 11

	.type	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.14,@object
	.section	.rodata..Lanon.c1f65b2e263de8b258326b6bc7df4cfe.14,"a",@progbits
.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.14:
	.ascii	"unknown_code"
	.size	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.14, 12

	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.2.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.37.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.91.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.394.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.408.llvm.9794848731438112354
	.type	.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel,@object
	.section	.rodata..Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel,"a",@progbits
	.p2align	2, 0x0
.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel:
	.long	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.4-.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel
	.long	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.5-.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel
	.long	.Lanon.c1f65b2e263de8b258326b6bc7df4cfe.6-.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel
	.size	.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.rel, 12

	.type	.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.11,@object
	.section	.rodata..Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.11,"a",@progbits
	.p2align	3, 0x0
.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.11:
	.ascii	"'&\024"
	.size	.Lswitch.table._RNvXs_NtCs1xZImt19Kx0_9getrandom5errorNtB4_5ErrorNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.11, 3

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits

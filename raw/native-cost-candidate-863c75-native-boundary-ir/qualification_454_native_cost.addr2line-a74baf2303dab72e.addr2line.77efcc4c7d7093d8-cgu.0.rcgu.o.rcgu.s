	.att_syntax
	.file	"addr2line.77efcc4c7d7093d8-cgu.0"
	.section	.text.unlikely._RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line,"ax",@progbits
	.globl	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line
	.type	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line,@function
_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line:
.Lfunc_begin0:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	callq	*_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	cmpq	$-1, %rax
	jne	.LBB0_2
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB0_2:
	.cfi_def_cfa %rbp, 16
	movq	%rax, %rdi
	movq	%rdx, %rsi
	callq	*_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip)
.Lfunc_end0:
	.size	_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line, .Lfunc_end0-_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line
	.cfi_endproc

	.section	.text._RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines13find_location,"ax",@progbits
	.globl	_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines13find_location
	.type	_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines13find_location,@function
_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines13find_location:
.Lfunc_begin1:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%rbx
	pushq	%rax
	.cfi_offset %rbx, -24
	movq	24(%rsi), %rax
	testq	%rax, %rax
	je	.LBB1_22
	movq	16(%rsi), %r8
	xorl	%ecx, %ecx
	cmpq	$1, %rax
	je	.LBB1_2
	movq	%rax, %r9
.LBB1_14:
	movq	%rcx, %r10
	movq	%r9, %r11
	shrq	%r11
	addq	%r11, %rcx
	movq	%rcx, %rbx
	shlq	$5, %rbx
	cmpq	16(%r8,%rbx), %rdx
	cmovbq	%r10, %rcx
	subq	%r11, %r9
	cmpq	$1, %r9
	ja	.LBB1_14
.LBB1_2:
	movq	%rcx, %r9
	shlq	$5, %r9
	cmpq	16(%r8,%r9), %rdx
	jb	.LBB1_22
	cmpq	24(%r8,%r9), %rdx
	jae	.LBB1_22
	cmpq	%rax, %rcx
	jae	.LBB1_17
	addq	%r9, %r8
	movq	8(%r8), %rax
	testq	%rax, %rax
	je	.LBB1_22
	movq	(%r8), %r8
	xorl	%ecx, %ecx
	cmpq	$1, %rax
	je	.LBB1_7
	movq	%rax, %r9
.LBB1_16:
	movq	%rcx, %r10
	movq	%r9, %r11
	shrq	%r11
	addq	%r11, %rcx
	leaq	(%rcx,%rcx,2), %rbx
	cmpq	%rdx, (%r8,%rbx,8)
	cmovaq	%r10, %rcx
	subq	%r11, %r9
	cmpq	$1, %r9
	ja	.LBB1_16
.LBB1_7:
	leaq	(%rcx,%rcx,2), %r9
	cmpq	%rdx, (%r8,%r9,8)
	je	.LBB1_10
	adcq	$0, %rcx
	je	.LBB1_22
	decq	%rcx
.LBB1_10:
	cmpq	%rax, %rcx
	jae	.LBB1_21
	leaq	(%rcx,%rcx,2), %rax
	leaq	(%r8,%rax,8), %rdx
	movq	8(%rdx), %rax
	cmpq	8(%rsi), %rax
	jae	.LBB1_12
	movq	(%rsi), %rcx
	leaq	(%rax,%rax,2), %rsi
	movq	8(%rcx,%rsi,8), %rax
	movq	16(%rcx,%rsi,8), %rcx
	jmp	.LBB1_20
.LBB1_22:
	movl	$2, (%rdi)
.LBB1_23:
	movq	%rdi, %rax
	addq	$8, %rsp
	popq	%rbx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB1_12:
	.cfi_def_cfa %rbp, 16
	xorl	%eax, %eax
.LBB1_20:
	movl	16(%rdx), %esi
	movl	20(%rdx), %edx
	xorl	%r8d, %r8d
	testl	%esi, %esi
	setne	%r8b
	movl	%r8d, (%rdi)
	movl	%esi, 4(%rdi)
	movl	%r8d, 8(%rdi)
	movl	%edx, 12(%rdi)
	movq	%rax, 16(%rdi)
	movq	%rcx, 24(%rdi)
	jmp	.LBB1_23
.LBB1_17:
	leaq	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.1(%rip), %rdx
	jmp	.LBB1_18
.LBB1_21:
	leaq	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.2(%rip), %rdx
.LBB1_18:
	movq	%rcx, %rdi
	movq	%rax, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.Lfunc_end1:
	.size	_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines13find_location, .Lfunc_end1-_RNvMs_NtCsaiq3CZtZqOa_9addr2line4lineNtB4_5Lines13find_location
	.cfi_endproc

	.section	.text._RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root,"ax",@progbits
	.type	_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root,@function
_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root:
.Lfunc_begin2:
	.cfi_startproc
	testq	%rsi, %rsi
	je	.LBB2_8
	movb	$1, %al
	cmpb	$92, (%rdi)
	je	.LBB2_2
	cmpq	$3, %rsi
	jb	.LBB2_8
	cmpb	$-65, 1(%rdi)
	jle	.LBB2_8
	cmpq	$3, %rsi
	je	.LBB2_7
	cmpb	$-64, 3(%rdi)
	jl	.LBB2_8
.LBB2_7:
	cmpw	$23610, 1(%rdi)
	sete	%al
.LBB2_2:
	retq
.LBB2_8:
	xorl	%eax, %eax
	retq
.Lfunc_end2:
	.size	_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root, .Lfunc_end2-_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root
	.cfi_endproc

	.section	.text._RNvNtCsaiq3CZtZqOa_9addr2line4line9path_push,"ax",@progbits
	.globl	_RNvNtCsaiq3CZtZqOa_9addr2line4line9path_push
	.type	_RNvNtCsaiq3CZtZqOa_9addr2line4line9path_push,@function
_RNvNtCsaiq3CZtZqOa_9addr2line4line9path_push:
.Lfunc_begin3:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r13
	pushq	%r12
	pushq	%rbx
	subq	$24, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdx, %rbx
	movq	%rsi, %r15
	movq	%rdi, %r14
	testq	%rdx, %rdx
	je	.LBB3_7
	cmpb	$47, (%r15)
	je	.LBB3_8
	cmpq	$3, %rbx
	jb	.LBB3_7
	cmpb	$-65, 1(%r15)
	jle	.LBB3_7
	cmpq	$3, %rbx
	je	.LBB3_6
	cmpb	$-64, 3(%r15)
	jl	.LBB3_7
.LBB3_6:
	cmpw	$12090, 1(%r15)
	je	.LBB3_8
.LBB3_7:
	movq	%r15, %rdi
	movq	%rbx, %rsi
	callq	_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root
	testb	%al, %al
	je	.LBB3_12
.LBB3_8:
	leaq	-64(%rbp), %r13
	movl	$1, %ecx
	movl	$1, %r8d
	movq	%r13, %rdi
	movq	%rbx, %rsi
	xorl	%edx, %edx
	callq	*_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	movq	8(%r13), %r12
	cmpl	$1, (%r13)
	je	.LBB3_24
	movq	-48(%rbp), %r13
	testq	%rbx, %rbx
	je	.LBB3_11
	movq	%r13, %rdi
	movq	%r15, %rsi
	movq	%rbx, %rdx
	callq	*memcpy@GOTPCREL(%rip)
.LBB3_11:
	movl	$1, %esi
	movl	$1, %edx
	movq	%r14, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	movq	%r12, (%r14)
	movq	%r13, 8(%r14)
	movq	%rbx, 16(%r14)
	jmp	.LBB3_20
.LBB3_12:
	movq	16(%r14), %r12
	testq	%r12, %r12
	je	.LBB3_13
	movq	8(%r14), %r13
	movq	%r13, %rdi
	movq	%r12, %rsi
	callq	_RNvNtCsaiq3CZtZqOa_9addr2line4line23has_backward_slash_root
	movq	%r13, %rcx
	testb	%al, %al
	movl	$92, %eax
	movl	$47, %r13d
	cmovnel	%eax, %r13d
	cmpb	-1(%rcx,%r12), %r13b
	je	.LBB3_15
	cmpq	%r12, (%r14)
	je	.LBB3_22
.LBB3_23:
	movb	%r13b, (%rcx,%r12)
	incq	%r12
	movq	%r12, 16(%r14)
	jmp	.LBB3_15
.LBB3_13:
	xorl	%r12d, %r12d
.LBB3_15:
	movq	(%r14), %rax
	subq	%r12, %rax
	cmpq	%rax, %rbx
	ja	.LBB3_16
	testq	%rbx, %rbx
	je	.LBB3_19
.LBB3_18:
	movq	8(%r14), %rdi
	addq	%r12, %rdi
	movq	%r15, %rsi
	movq	%rbx, %rdx
	callq	*memcpy@GOTPCREL(%rip)
.LBB3_19:
	addq	%rbx, %r12
	movq	%r12, 16(%r14)
.LBB3_20:
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB3_24:
	.cfi_def_cfa %rbp, 16
	movq	-48(%rbp), %rsi
	movq	%r12, %rdi
	callq	*_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip)
.LBB3_16:
	movl	$1, %ecx
	movl	$1, %r8d
	movq	%r14, %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	callq	*_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line@GOTPCREL(%rip)
	movq	16(%r14), %r12
	jmp	.LBB3_18
.LBB3_22:
	movl	$1, %edx
	movl	$1, %ecx
	movl	$1, %r8d
	movq	%r14, %rdi
	movq	%r12, %rsi
	callq	*_RINvNvMs2_NtCsc70TAahYccp_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsaiq3CZtZqOa_9addr2line@GOTPCREL(%rip)
	movq	8(%r14), %rcx
	jmp	.LBB3_23
.Lfunc_end3:
	.size	_RNvNtCsaiq3CZtZqOa_9addr2line4line9path_push, .Lfunc_end3-_RNvNtCsaiq3CZtZqOa_9addr2line4line9path_push
	.cfi_endproc

	.type	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.0,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.0:
	.asciz	"/cargo/registry/25cdd57fae9f0462/addr2line-0.27.1/src/line.rs"
	.size	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.0, 62

	.type	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.1,@object
	.section	.data.rel.ro..Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.1,"aw",@progbits
	.p2align	3, 0x0
.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.1:
	.quad	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.0
	.asciz	"=\000\000\000\000\000\000\000\236\000\000\000\031\000\000"
	.size	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.1, 24

	.type	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.2,@object
	.section	.data.rel.ro..Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.2,"aw",@progbits
	.p2align	3, 0x0
.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.2:
	.quad	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.0
	.asciz	"=\000\000\000\000\000\000\000\250\000\000\000$\000\000"
	.size	.Lanon.9ab48ae7fdf266c6dfa6017d5efa634b.2, 24

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits

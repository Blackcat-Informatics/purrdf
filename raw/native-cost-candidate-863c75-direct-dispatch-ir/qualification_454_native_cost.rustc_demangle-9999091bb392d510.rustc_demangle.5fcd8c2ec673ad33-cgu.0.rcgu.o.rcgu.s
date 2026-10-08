	.att_syntax
	.file	"rustc_demangle.5fcd8c2ec673ad33-cgu.0"
	.section	.text._RINvMNtCs2k2z8Zem4rB_4core3stre18trim_start_matchesReECs8dXjxA0JZyF_14rustc_demangle,"ax",@progbits
	.globl	_RINvMNtCs2k2z8Zem4rB_4core3stre18trim_start_matchesReECs8dXjxA0JZyF_14rustc_demangle
	.type	_RINvMNtCs2k2z8Zem4rB_4core3stre18trim_start_matchesReECs8dXjxA0JZyF_14rustc_demangle,@function
_RINvMNtCs2k2z8Zem4rB_4core3stre18trim_start_matchesReECs8dXjxA0JZyF_14rustc_demangle:
.Lfunc_begin0:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r12
	pushq	%rbx
	subq	$128, %rsp
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rcx, %r8
	movq	%rdx, %rcx
	movq	%rsi, %rbx
	movq	%rdi, %r14
	leaq	-160(%rbp), %r15
	movq	%r15, %rdi
	movq	%r14, %rsi
	movq	%rbx, %rdx
	callq	*_RNvMsu_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcher3new@GOTPCREL(%rip)
	leaq	-56(%rbp), %r12
.LBB0_1:
	movq	%r12, %rdi
	movq	%r15, %rsi
	callq	_RNvXsv_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcherNtB5_8Searcher4next
	movq	-56(%rbp), %rax
	testq	%rax, %rax
	je	.LBB0_1
	cmpl	$1, %eax
	jne	.LBB0_4
	movq	-48(%rbp), %rax
	jmp	.LBB0_5
.LBB0_4:
	movq	%rbx, %rax
.LBB0_5:
	subq	%rax, %rbx
	addq	%rax, %r14
	movq	%r14, %rax
	movq	%rbx, %rdx
	addq	$128, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end0:
	.size	_RINvMNtCs2k2z8Zem4rB_4core3stre18trim_start_matchesReECs8dXjxA0JZyF_14rustc_demangle, .Lfunc_end0-_RINvMNtCs2k2z8Zem4rB_4core3stre18trim_start_matchesReECs8dXjxA0JZyF_14rustc_demangle
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_10print_paths_0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_10print_paths_0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_10print_paths_0EB8_:
.Lfunc_begin1:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%rbx
	subq	$72, %rsp
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %rbx
	cmpq	$0, (%rdi)
	je	.LBB1_4
	movl	%esi, %r14d
	leaq	-56(%rbp), %r15
	movq	%r15, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref
	cmpq	$0, (%r15)
	je	.LBB1_6
	cmpq	$0, 32(%rbx)
	je	.LBB1_10
	vmovups	(%rbx), %ymm0
	vmovups	%ymm0, -96(%rbp)
	vmovups	-56(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	movzbl	%r14b, %esi
	movq	%rbx, %rdi
	vzeroupper
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	vmovups	-96(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	jmp	.LBB1_12
.LBB1_4:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB1_10
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$72, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB1_6:
	.cfi_def_cfa %rbp, 16
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB1_9
	movb	-48(%rbp), %al
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
	movzbl	%al, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movl	%eax, %ecx
	movb	$1, %al
	testb	%cl, %cl
	jne	.LBB1_11
.LBB1_9:
	vmovups	-56(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
.LBB1_10:
	xorl	%eax, %eax
.LBB1_11:
.LBB1_12:
	addq	$72, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.Lfunc_end1:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_10print_paths_0EB8_, .Lfunc_end1-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_10print_paths_0EB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_11print_consts4_0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_11print_consts4_0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_11print_consts4_0EB8_:
.Lfunc_begin2:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%rbx
	subq	$72, %rsp
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %rbx
	cmpq	$0, (%rdi)
	je	.LBB2_4
	movl	%esi, %r14d
	leaq	-56(%rbp), %r15
	movq	%r15, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref
	cmpq	$0, (%r15)
	je	.LBB2_6
	cmpq	$0, 32(%rbx)
	je	.LBB2_10
	vmovups	(%rbx), %ymm0
	vmovups	%ymm0, -96(%rbp)
	vmovups	-56(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	movzbl	%r14b, %esi
	movq	%rbx, %rdi
	vzeroupper
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	vmovups	-96(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	jmp	.LBB2_12
.LBB2_4:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB2_10
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$72, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB2_6:
	.cfi_def_cfa %rbp, 16
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB2_9
	movb	-48(%rbp), %al
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
	movzbl	%al, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movl	%eax, %ecx
	movb	$1, %al
	testb	%cl, %cl
	jne	.LBB2_11
.LBB2_9:
	vmovups	-56(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
.LBB2_10:
	xorl	%eax, %eax
.LBB2_11:
.LBB2_12:
	addq	$72, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.Lfunc_end2:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_11print_consts4_0EB8_, .Lfunc_end2-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_11print_consts4_0EB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNvB2_10print_typeEB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNvB2_10print_typeEB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNvB2_10print_typeEB8_:
.Lfunc_begin3:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	subq	$64, %rsp
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rdi, %rbx
	cmpq	$0, (%rdi)
	je	.LBB3_4
	leaq	-48(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref
	cmpq	$0, (%r14)
	je	.LBB3_6
	cmpq	$0, 32(%rbx)
	je	.LBB3_10
	vmovups	(%rbx), %ymm0
	vmovups	%ymm0, -80(%rbp)
	vmovups	-48(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	movq	%rbx, %rdi
	vzeroupper
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
	vmovups	-80(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	jmp	.LBB3_12
.LBB3_4:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB3_10
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$64, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB3_6:
	.cfi_def_cfa %rbp, 16
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB3_9
	movb	-40(%rbp), %al
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
	movzbl	%al, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movl	%eax, %ecx
	movb	$1, %al
	testb	%cl, %cl
	jne	.LBB3_11
.LBB3_9:
	vmovups	-48(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
.LBB3_10:
	xorl	%eax, %eax
.LBB3_11:
.LBB3_12:
	addq	$64, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.Lfunc_end3:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNvB2_10print_typeEB8_, .Lfunc_end3-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNvB2_10print_typeEB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts0_0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts0_0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts0_0EB8_:
.Lfunc_begin4:
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
	pushq	%rax
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB4_1
	movq	%rdi, %r14
	xorl	%ebx, %ebx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %r15
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r12
	xorl	%r13d, %r13d
.LBB4_3:
	movq	16(%r14), %rcx
	cmpq	8(%r14), %rcx
	jae	.LBB4_6
	cmpb	$69, (%rax,%rcx)
	je	.LBB4_5
.LBB4_6:
	subq	$1, %r13
	jb	.LBB4_10
	movq	32(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB4_10
	movl	$2, %edx
	movq	%r15, %rsi
	callq	*%r12
	testb	%al, %al
	jne	.LBB4_9
.LBB4_10:
	movq	%r14, %rdi
	movl	$1, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	testb	%al, %al
	jne	.LBB4_9
	movq	(%r14), %rax
	testq	%rax, %rax
	jne	.LBB4_3
	jmp	.LBB4_12
.LBB4_9:
	movl	$1, %ebx
	jmp	.LBB4_12
.LBB4_5:
	incq	%rcx
	movq	%rcx, 16(%r14)
.LBB4_1:
	xorl	%ebx, %ebx
.LBB4_12:
	movq	%rbx, %rax
	addq	$8, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end4:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts0_0EB8_, .Lfunc_end4-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts0_0EB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts1_0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts1_0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts1_0EB8_:
.Lfunc_begin5:
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
	pushq	%rax
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB5_9
	movq	%rdi, %r15
	xorl	%r14d, %r14d
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %r12
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r13
	xorl	%ebx, %ebx
.LBB5_2:
	movq	16(%r15), %rcx
	cmpq	8(%r15), %rcx
	jae	.LBB5_4
	cmpb	$69, (%rax,%rcx)
	je	.LBB5_11
.LBB5_4:
	testq	%rbx, %rbx
	je	.LBB5_7
	movq	32(%r15), %rdi
	testq	%rdi, %rdi
	je	.LBB5_7
	movl	$2, %edx
	movq	%r12, %rsi
	callq	*%r13
	testb	%al, %al
	jne	.LBB5_10
.LBB5_7:
	movq	%r15, %rdi
	movl	$1, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	testb	%al, %al
	jne	.LBB5_10
	incq	%rbx
	movq	(%r15), %rax
	testq	%rax, %rax
	jne	.LBB5_2
	jmp	.LBB5_13
.LBB5_9:
	xorl	%ebx, %ebx
	jmp	.LBB5_12
.LBB5_10:
	movl	$1, %r14d
	jmp	.LBB5_13
.LBB5_11:
	incq	%rcx
	movq	%rcx, 16(%r15)
.LBB5_12:
	xorl	%r14d, %r14d
.LBB5_13:
	movq	%r14, %rax
	movq	%rbx, %rdx
	addq	$8, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end5:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts1_0EB8_, .Lfunc_end5-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts1_0EB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts3_0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts3_0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts3_0EB8_:
.Lfunc_begin6:
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
	subq	$72, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB6_27
	movq	%rdi, %rbx
	leaq	-72(%rbp), %r14
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r12
	xorl	%r13d, %r13d
.LBB6_2:
	movq	16(%rbx), %rcx
	cmpq	8(%rbx), %rcx
	jae	.LBB6_5
	cmpb	$69, (%rax,%rcx)
	je	.LBB6_4
.LBB6_5:
	subq	$1, %r13
	jb	.LBB6_12
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB6_12
	movl	$2, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %rsi
	callq	*%r12
	testb	%al, %al
	jne	.LBB6_8
	cmpq	$0, (%rbx)
	je	.LBB6_10
.LBB6_12:
	movq	%r14, %rdi
	movq	%rbx, %rsi
	movl	$115, %edx
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62
	cmpb	$1, -72(%rbp)
	je	.LBB6_13
	cmpq	$0, (%rbx)
	je	.LBB6_10
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident
	cmpq	$0, -72(%rbp)
	je	.LBB6_19
	vmovups	-72(%rbp), %ymm0
	vmovups	%ymm0, -112(%rbp)
	movq	32(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB6_25
	leaq	-112(%rbp), %rdi
	vzeroupper
	callq	*_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB6_8
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB6_25
	movl	$2, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.28(%rip), %rsi
	callq	*%r12
	testb	%al, %al
	jne	.LBB6_8
.LBB6_25:
	movq	%rbx, %rdi
	movl	$1, %esi
	vzeroupper
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	testb	%al, %al
	jne	.LBB6_8
	jmp	.LBB6_26
.LBB6_10:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB6_26
	movl	$1, %r15d
	movl	$1, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	callq	*%r12
	testb	%al, %al
	jne	.LBB6_28
.LBB6_26:
	movq	(%rbx), %rax
	testq	%rax, %rax
	jne	.LBB6_2
	jmp	.LBB6_27
.LBB6_13:
	movb	-71(%rbp), %r14b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB6_16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%r14b, %r14b
	cmovneq	%rax, %rsi
	movzbl	%r14b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	jmp	.LBB6_15
.LBB6_19:
	movb	-64(%rbp), %r14b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB6_16
	movzbl	%r14b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
.LBB6_15:
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	je	.LBB6_16
.LBB6_8:
	movl	$1, %r15d
	jmp	.LBB6_28
.LBB6_16:
	movq	$0, (%rbx)
	movb	%r14b, 8(%rbx)
	jmp	.LBB6_27
.LBB6_4:
	incq	%rcx
	movq	%rcx, 16(%rbx)
.LBB6_27:
	xorl	%r15d, %r15d
.LBB6_28:
	movq	%r15, %rax
	addq	$72, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end6:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts3_0EB8_, .Lfunc_end6-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts3_0EB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_10print_typeEB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_10print_typeEB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_10print_typeEB8_:
.Lfunc_begin7:
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
	pushq	%rax
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB7_9
	movq	%rdi, %r15
	xorl	%r14d, %r14d
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %r12
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r13
	xorl	%ebx, %ebx
.LBB7_2:
	movq	16(%r15), %rcx
	cmpq	8(%r15), %rcx
	jae	.LBB7_4
	cmpb	$69, (%rax,%rcx)
	je	.LBB7_11
.LBB7_4:
	testq	%rbx, %rbx
	je	.LBB7_7
	movq	32(%r15), %rdi
	testq	%rdi, %rdi
	je	.LBB7_7
	movl	$2, %edx
	movq	%r12, %rsi
	callq	*%r13
	testb	%al, %al
	jne	.LBB7_10
.LBB7_7:
	movq	%r15, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
	testb	%al, %al
	jne	.LBB7_10
	incq	%rbx
	movq	(%r15), %rax
	testq	%rax, %rax
	jne	.LBB7_2
	jmp	.LBB7_13
.LBB7_9:
	xorl	%ebx, %ebx
	jmp	.LBB7_12
.LBB7_10:
	movl	$1, %r14d
	jmp	.LBB7_13
.LBB7_11:
	incq	%rcx
	movq	%rcx, 16(%r15)
.LBB7_12:
	xorl	%r14d, %r14d
.LBB7_13:
	movq	%r14, %rax
	movq	%rbx, %rdx
	addq	$8, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end7:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_10print_typeEB8_, .Lfunc_end7-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_10print_typeEB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_15print_dyn_traitEB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_15print_dyn_traitEB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_15print_dyn_traitEB8_:
.Lfunc_begin8:
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
	subq	$72, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB8_28
	movq	%rdi, %rbx
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r14
	leaq	-72(%rbp), %r13
	leaq	-112(%rbp), %r15
	xorl	%r12d, %r12d
.LBB8_2:
	movq	16(%rbx), %rcx
	cmpq	8(%rbx), %rcx
	jae	.LBB8_5
	cmpb	$69, (%rax,%rcx)
	je	.LBB8_4
.LBB8_5:
	testq	%r12, %r12
	je	.LBB8_10
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB8_10
	movl	$3, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.27(%rip), %rsi
	callq	*%r14
	testb	%al, %al
	jne	.LBB8_8
.LBB8_10:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer30print_path_maybe_open_generics
	cmpb	$2, %al
	je	.LBB8_8
	movq	(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB8_14
.LBB8_12:
	movq	16(%rbx), %rdx
	cmpq	8(%rbx), %rdx
	jae	.LBB8_14
	cmpb	$112, (%rcx,%rdx)
	jne	.LBB8_14
	incq	%rdx
	movq	%rdx, 16(%rbx)
	testb	$1, %al
	je	.LBB8_18
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB8_23
	movl	$2, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %rsi
	jmp	.LBB8_22
.LBB8_18:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB8_23
	movl	$1, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54(%rip), %rsi
.LBB8_22:
	callq	*%r14
	testb	%al, %al
	jne	.LBB8_8
.LBB8_23:
	cmpq	$0, (%rbx)
	je	.LBB8_24
	movq	%r13, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident
	cmpq	$0, -72(%rbp)
	je	.LBB8_30
	vmovups	-72(%rbp), %ymm0
	vmovups	%ymm0, -112(%rbp)
	movq	32(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB8_37
	movq	%r15, %rdi
	vzeroupper
	callq	*_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB8_8
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB8_37
	movl	$3, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.74(%rip), %rsi
	callq	*%r14
	testb	%al, %al
	jne	.LBB8_8
.LBB8_37:
	movq	(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB8_41
	movq	16(%rbx), %rax
	cmpq	8(%rbx), %rax
	jae	.LBB8_41
	cmpb	$75, (%rcx,%rax)
	jne	.LBB8_41
	incq	%rax
	movq	%rax, 16(%rbx)
	movq	%rbx, %rdi
	xorl	%esi, %esi
	vzeroupper
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	jmp	.LBB8_42
.LBB8_41:
	movq	%rbx, %rdi
	vzeroupper
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
.LBB8_42:
	testb	%al, %al
	jne	.LBB8_8
	movq	(%rbx), %rcx
	movb	$1, %al
	testq	%rcx, %rcx
	jne	.LBB8_12
	jmp	.LBB8_15
.LBB8_14:
	testb	$1, %al
	je	.LBB8_27
.LBB8_15:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB8_27
	movl	$1, %r15d
	movl	$1, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.56(%rip), %rsi
.LBB8_26:
	callq	*%r14
	testb	%al, %al
	jne	.LBB8_9
.LBB8_27:
	movq	(%rbx), %rax
	incq	%r12
	testq	%rax, %rax
	leaq	-112(%rbp), %r15
	jne	.LBB8_2
	jmp	.LBB8_28
.LBB8_24:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB8_27
	movl	$1, %r15d
	movl	$1, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	jmp	.LBB8_26
.LBB8_30:
	movb	-64(%rbp), %r14b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB8_32
	movzbl	%r14b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	je	.LBB8_32
.LBB8_8:
	movl	$1, %r15d
	jmp	.LBB8_9
.LBB8_32:
	movq	$0, (%rbx)
	movb	%r14b, 8(%rbx)
	jmp	.LBB8_28
.LBB8_4:
	incq	%rcx
	movq	%rcx, 16(%rbx)
.LBB8_28:
	xorl	%r15d, %r15d
.LBB8_9:
	movq	%r15, %rax
	addq	$72, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end8:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_15print_dyn_traitEB8_, .Lfunc_end8-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_15print_dyn_traitEB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_17print_generic_argEB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_17print_generic_argEB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_17print_generic_argEB8_:
.Lfunc_begin9:
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
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB9_1
	movq	%rdi, %rbx
	xorl	%r14d, %r14d
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %r12
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r13
	xorl	%r15d, %r15d
.LBB9_4:
	movq	8(%rbx), %rdx
	movq	16(%rbx), %rcx
	cmpq	%rdx, %rcx
	jae	.LBB9_7
	cmpb	$69, (%rax,%rcx)
	je	.LBB9_6
.LBB9_7:
	subq	$1, %r15
	jb	.LBB9_13
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB9_13
	movl	$2, %edx
	movq	%r12, %rsi
	callq	*%r13
	testb	%al, %al
	jne	.LBB9_10
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB9_16
	movq	8(%rbx), %rdx
	movq	16(%rbx), %rcx
.LBB9_13:
	cmpq	%rdx, %rcx
	jae	.LBB9_16
	movzbl	(%rax,%rcx), %eax
	cmpl	$75, %eax
	je	.LBB9_19
	cmpl	$76, %eax
	jne	.LBB9_16
	incq	%rcx
	movq	%rcx, 16(%rbx)
	leaq	-56(%rbp), %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62
	cmpb	$1, -56(%rbp)
	je	.LBB9_21
	movq	-48(%rbp), %rsi
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index
	jmp	.LBB9_17
.LBB9_16:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
.LBB9_17:
	testb	%al, %al
	jne	.LBB9_10
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB9_2
	jmp	.LBB9_4
.LBB9_19:
	incq	%rcx
	movq	%rcx, 16(%rbx)
	movq	%rbx, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	jmp	.LBB9_17
.LBB9_6:
	incq	%rcx
	movq	%rcx, 16(%rbx)
.LBB9_1:
	xorl	%r14d, %r14d
	jmp	.LBB9_2
.LBB9_21:
	movb	-55(%rbp), %r14b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB9_23
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%r14b, %r14b
	cmovneq	%rax, %rsi
	movzbl	%r14b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	je	.LBB9_23
.LBB9_10:
	movl	$1, %r14d
.LBB9_2:
	movq	%r14, %rax
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB9_23:
	.cfi_def_cfa %rbp, 16
	movq	$0, (%rbx)
	movb	%r14b, 8(%rbx)
	jmp	.LBB9_1
.Lfunc_end9:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_17print_generic_argEB8_, .Lfunc_end9-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_17print_generic_argEB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer17skipping_printingNCNvB2_10print_path0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer17skipping_printingNCNvB2_10print_path0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer17skipping_printingNCNvB2_10print_path0EB8_:
.Lfunc_begin10:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	subq	$16, %rsp
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rdi, %rbx
	movq	32(%rdi), %r14
	movq	$0, 32(%rdi)
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	testb	%al, %al
	jne	.LBB10_2
	movq	%r14, 32(%rbx)
	addq	$16, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB10_2:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.3(%rip), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.39(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.5(%rip), %r8
	leaq	-17(%rbp), %rdx
	movl	$61, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip)
.Lfunc_end10:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer17skipping_printingNCNvB2_10print_path0EB8_, .Lfunc_end10-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer17skipping_printingNCNvB2_10print_path0EB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer26print_quoted_escaped_charsINtNtNtNtCs2k2z8Zem4rB_4core4iter7sources4once4OncecEEB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer26print_quoted_escaped_charsINtNtNtNtCs2k2z8Zem4rB_4core4iter7sources4once4OncecEEB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer26print_quoted_escaped_charsINtNtNtNtCs2k2z8Zem4rB_4core4iter7sources4once4OncecEEB8_:
.Lfunc_begin11:
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
	subq	$40, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	testq	%rdi, %rdi
	je	.LBB11_1
	movl	%esi, %r12d
	movq	%rdi, %rbx
	movq	(%rdi), %rdi
	movq	8(%rbx), %rax
	movl	$39, %esi
	callq	*32(%rax)
	testb	%al, %al
	je	.LBB11_3
.LBB11_13:
	movb	$1, %al
	jmp	.LBB11_14
.LBB11_1:
	xorl	%eax, %eax
.LBB11_14:
	addq	$40, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB11_3:
	.cfi_def_cfa %rbp, 16
	movq	%rbx, -48(%rbp)
.LBB11_4:
	cmpl	$34, %r12d
	je	.LBB11_8
	cmpl	$-1, %r12d
	je	.LBB11_16
	leaq	-64(%rbp), %rdi
	movl	%r12d, %esi
	callq	_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc16escape_debug_ext
	movzbl	-51(%rbp), %eax
	movl	%eax, %r13d
	movzbl	-52(%rbp), %r15d
	cmpb	%al, %r15b
	cmoval	%r15d, %eax
	jae	.LBB11_15
	movl	-64(%rbp), %ebx
	movzbl	%al, %eax
	movq	%rax, -72(%rbp)
	movq	-48(%rbp), %rax
	movq	(%rax), %r12
	movq	8(%rax), %rax
	movq	32(%rax), %r14
.LBB11_10:
	movl	%ebx, %esi
	cmpb	$-128, %r13b
	ja	.LBB11_12
	movzbl	-64(%rbp,%r15), %esi
.LBB11_12:
	movq	%r12, %rdi
	callq	*%r14
	testb	%al, %al
	jne	.LBB11_13
	incq	%r15
	cmpq	%r15, -72(%rbp)
	jne	.LBB11_10
.LBB11_15:
	movl	$-1, %r12d
	movq	-48(%rbp), %rbx
	jmp	.LBB11_4
.LBB11_8:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	movl	$34, %esi
	callq	*32(%rax)
	movl	$-1, %r12d
	testb	%al, %al
	je	.LBB11_4
	jmp	.LBB11_13
.LBB11_16:
	movq	(%rbx), %rdi
	movq	8(%rbx), %rax
	movl	$39, %esi
	addq	$40, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*32(%rax)
.Lfunc_end11:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer26print_quoted_escaped_charsINtNtNtNtCs2k2z8Zem4rB_4core4iter7sources4once4OncecEEB8_, .Lfunc_end11-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer26print_quoted_escaped_charsINtNtNtNtCs2k2z8Zem4rB_4core4iter7sources4once4OncecEEB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_type0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_type0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_type0EB8_:
.Lfunc_begin12:
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
	movq	%rdi, %rbx
	cmpq	$0, (%rdi)
	je	.LBB12_1
	leaq	-56(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	movl	$71, %edx
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62
	cmpb	$1, (%r14)
	jne	.LBB12_8
	movb	-55(%rbp), %r15b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB12_7
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%r15b, %r15b
	cmovneq	%rax, %rsi
	movzbl	%r15b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB12_18
.LBB12_7:
	movq	$0, (%rbx)
	movb	%r15b, 8(%rbx)
	jmp	.LBB12_2
.LBB12_1:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB12_2
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB12_2:
	.cfi_def_cfa %rbp, 16
	xorl	%r14d, %r14d
.LBB12_18:
	movl	%r14d, %eax
.LBB12_19:
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB12_8:
	.cfi_def_cfa %rbp, 16
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB12_22
	movq	-48(%rbp), %r12
	testq	%r12, %r12
	je	.LBB12_10
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.6(%rip), %rsi
	movl	$4, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB12_18
	xorl	%r13d, %r13d
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r15
.LBB12_13:
	testq	%r13, %r13
	je	.LBB12_14
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB12_14
	movl	$2, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %rsi
	callq	*%r15
	testb	%al, %al
	jne	.LBB12_18
.LBB12_14:
	incl	40(%rbx)
	movl	$1, %esi
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index
	testb	%al, %al
	jne	.LBB12_18
	incq	%r13
	cmpq	%r13, %r12
	jne	.LBB12_13
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB12_10
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.7(%rip), %rsi
	movl	$2, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB12_18
.LBB12_10:
	movq	%rbx, %rdi
	callq	_RNCNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB7_7Printer10print_type0B9_
	subl	%r12d, 40(%rbx)
	jmp	.LBB12_19
.LBB12_22:
	movq	%rbx, %rdi
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmp	_RNCNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB7_7Printer10print_type0B9_
.Lfunc_end12:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_type0EB8_, .Lfunc_end12-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_type0EB8_
	.cfi_endproc

	.section	.text._RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_types_0EB8_,"ax",@progbits
	.type	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_types_0EB8_,@function
_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_types_0EB8_:
.Lfunc_begin13:
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
	movq	%rdi, %rbx
	cmpq	$0, (%rdi)
	je	.LBB13_1
	leaq	-56(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	movl	$71, %edx
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62
	cmpb	$1, (%r14)
	jne	.LBB13_8
	movb	-55(%rbp), %r15b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB13_7
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%r15b, %r15b
	cmovneq	%rax, %rsi
	movzbl	%r15b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB13_19
.LBB13_7:
	movq	$0, (%rbx)
	movb	%r15b, 8(%rbx)
	jmp	.LBB13_2
.LBB13_1:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB13_2
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB13_2:
	.cfi_def_cfa %rbp, 16
	xorl	%r14d, %r14d
	jmp	.LBB13_19
.LBB13_8:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB13_18
	movq	-48(%rbp), %r12
	testq	%r12, %r12
	je	.LBB13_10
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.6(%rip), %rsi
	movl	$4, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB13_19
	xorl	%r13d, %r13d
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r15
.LBB13_13:
	testq	%r13, %r13
	je	.LBB13_14
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB13_14
	movl	$2, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8(%rip), %rsi
	callq	*%r15
	testb	%al, %al
	jne	.LBB13_19
.LBB13_14:
	incl	40(%rbx)
	movl	$1, %esi
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index
	testb	%al, %al
	jne	.LBB13_19
	incq	%r13
	cmpq	%r13, %r12
	jne	.LBB13_13
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB13_10
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.7(%rip), %rsi
	movl	$2, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB13_19
.LBB13_10:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_15print_dyn_traitEB8_
	movq	%rax, %r14
	subl	%r12d, 40(%rbx)
	jmp	.LBB13_19
.LBB13_18:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_15print_dyn_traitEB8_
	movq	%rax, %r14
.LBB13_19:
	movl	%r14d, %eax
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end13:
	.size	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_types_0EB8_, .Lfunc_end13-_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_types_0EB8_
	.cfi_endproc

	.section	.text._RNCNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB7_7Printer10print_type0B9_,"ax",@progbits
	.type	_RNCNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB7_7Printer10print_type0B9_,@function
_RNCNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB7_7Printer10print_type0B9_:
.Lfunc_begin14:
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
	subq	$88, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, -64(%rbp)
	movq	(%rdi), %rcx
	testq	%rcx, %rcx
	je	.LBB14_41
	movq	-64(%rbp), %r13
	movq	8(%r13), %rdx
	movq	16(%r13), %rax
	cmpq	%rdx, %rax
	jae	.LBB14_4
	cmpb	$85, (%rcx,%rax)
	jne	.LBB14_4
	incq	%rax
	movq	%rax, 16(%r13)
	movb	$1, %r15b
	jmp	.LBB14_5
.LBB14_4:
	xorl	%r15d, %r15d
.LBB14_5:
	cmpq	%rdx, %rax
	jae	.LBB14_10
	cmpb	$75, (%rcx,%rax)
	jne	.LBB14_10
	leaq	1(%rax), %rsi
	movq	%rsi, 16(%r13)
	cmpq	%rdx, %rsi
	jae	.LBB14_15
	cmpb	$67, (%rcx,%rsi)
	jne	.LBB14_15
	addq	$2, %rax
	movq	%rax, 16(%r13)
	movl	$1, %ebx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.19(%rip), %rax
	movq	%rax, -56(%rbp)
	jmp	.LBB14_18
.LBB14_10:
	testb	%r15b, %r15b
	je	.LBB14_41
	movq	$0, -56(%rbp)
.LBB14_12:
	movq	-64(%rbp), %rax
	movq	32(%rax), %rdi
	testq	%rdi, %rdi
	je	.LBB14_14
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.20(%rip), %rsi
	movl	$7, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
.LBB14_14:
	cmpq	$0, -56(%rbp)
	jne	.LBB14_19
	jmp	.LBB14_41
.LBB14_15:
	leaq	-120(%rbp), %r14
	movq	%r14, %rdi
	movq	%r13, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident
	movq	(%r14), %rcx
	movq	%rcx, -56(%rbp)
	testq	%rcx, %rcx
	je	.LBB14_53
	movq	-112(%rbp), %rbx
	testq	%rbx, %rbx
	je	.LBB14_56
	cmpq	$0, -96(%rbp)
	jne	.LBB14_56
.LBB14_18:
	testb	%r15b, %r15b
	jne	.LBB14_12
.LBB14_19:
	movq	-64(%rbp), %rax
	movq	32(%rax), %rdi
	testq	%rdi, %rdi
	movq	%rdi, -48(%rbp)
	je	.LBB14_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.21(%rip), %rsi
	movl	$8, %edx
	movq	-48(%rbp), %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movq	-48(%rbp), %rdi
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
.LBB14_21:
	xorl	%r8d, %r8d
	xorl	%r12d, %r12d
.LBB14_22:
	movq	%rbx, %rax
	subq	%r12, %rax
	movq	-56(%rbp), %rcx
	leaq	(%rcx,%r12), %rsi
	cmpq	$15, %rax
	ja	.LBB14_27
	movb	$1, %r14b
	cmpq	%r12, %rbx
	je	.LBB14_33
	xorl	%edx, %edx
.LBB14_25:
	cmpb	$95, (%rsi,%rdx)
	je	.LBB14_29
	incq	%rdx
	cmpq	%rdx, %rax
	jne	.LBB14_25
	jmp	.LBB14_33
.LBB14_27:
	movl	$95, %edi
	movq	%rax, %rdx
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice6memchr14memchr_aligned@GOTPCREL(%rip)
	testb	$1, %al
	je	.LBB14_85
	movq	-48(%rbp), %rdi
	xorl	%r8d, %r8d
.LBB14_29:
	leaq	(%rdx,%r12), %r13
	incq	%r13
	addq	%r12, %rdx
	cmpq	%rbx, %rdx
	jae	.LBB14_31
	movq	-56(%rbp), %rax
	cmpb	$95, (%rax,%rdx)
	je	.LBB14_86
.LBB14_31:
	movq	%r13, %r12
	cmpq	%rbx, %r13
	jbe	.LBB14_22
	movb	$1, %r14b
	jmp	.LBB14_34
.LBB14_33:
	movq	%rbx, %r13
.LBB14_34:
	movq	%rbx, %rdx
.LBB14_35:
	testq	%rdi, %rdi
	movq	%rbx, -80(%rbp)
	je	.LBB14_37
	movq	-48(%rbp), %rdi
	movq	-56(%rbp), %rsi
	movq	%r8, %rbx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movq	%rbx, %r8
	movq	-80(%rbp), %rbx
	movq	-48(%rbp), %rdi
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
.LBB14_37:
	testb	%r14b, %r14b
	je	.LBB14_61
	movq	%rdi, -72(%rbp)
.LBB14_39:
	cmpq	$0, -72(%rbp)
	je	.LBB14_41
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.22(%rip), %rsi
	movl	$2, %edx
	movq	-72(%rbp), %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
.LBB14_41:
	movq	-64(%rbp), %rax
	movq	32(%rax), %rdi
	testq	%rdi, %rdi
	je	.LBB14_43
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.24(%rip), %rsi
	movl	$3, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
.LBB14_43:
	movq	-64(%rbp), %rbx
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_10print_typeEB8_
	movb	$1, %r12b
	testb	$1, %al
	jne	.LBB14_60
	movq	32(%rbx), %r14
	testq	%r14, %r14
	je	.LBB14_46
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25(%rip), %rsi
	movl	$1, %edx
	movq	%r14, %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB14_60
.LBB14_46:
	movq	(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB14_50
	movq	16(%rbx), %rax
	cmpq	8(%rbx), %rax
	jae	.LBB14_50
	cmpb	$117, (%rcx,%rax)
	jne	.LBB14_50
	incq	%rax
	movq	%rax, 16(%rbx)
	jmp	.LBB14_59
.LBB14_50:
	testq	%r14, %r14
	je	.LBB14_52
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.26(%rip), %rsi
	movl	$4, %edx
	movq	%r14, %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB14_60
.LBB14_52:
	movq	-64(%rbp), %rdi
	addq	$88, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmp	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
.LBB14_53:
	.cfi_def_cfa %rbp, 16
	movb	-112(%rbp), %bl
	movq	32(%r13), %rdi
	testq	%rdi, %rdi
	je	.LBB14_55
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%bl, %bl
	cmovneq	%rax, %rsi
	movzbl	%bl, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
.LBB14_55:
	movq	$0, (%r13)
	movb	%bl, 8(%r13)
	jmp	.LBB14_59
.LBB14_56:
	movq	32(%r13), %rdi
	testq	%rdi, %rdi
	je	.LBB14_58
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
.LBB14_58:
	movq	-64(%rbp), %rax
	movq	$0, (%rax)
	movb	$0, 8(%rax)
.LBB14_59:
	xorl	%r12d, %r12d
.LBB14_60:
	movl	%r12d, %eax
	addq	$88, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB14_61:
	.cfi_def_cfa %rbp, 16
	movq	%rdi, -72(%rbp)
.LBB14_62:
	movq	%r8, %r12
	movb	$1, %r9b
	cmpq	%r13, %rbx
	jb	.LBB14_75
.LBB14_63:
	movq	%rbx, %rdx
	subq	%r13, %rdx
	movq	-56(%rbp), %rax
	leaq	(%rax,%r13), %rsi
	cmpq	$15, %rdx
	ja	.LBB14_68
	cmpq	%r13, %rbx
	je	.LBB14_74
	xorl	%r14d, %r14d
.LBB14_66:
	cmpb	$95, (%rsi,%r14)
	je	.LBB14_70
	incq	%r14
	cmpq	%r14, %rdx
	jne	.LBB14_66
	jmp	.LBB14_74
.LBB14_68:
	movq	%rdi, %r15
	movl	$95, %edi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice6memchr14memchr_aligned@GOTPCREL(%rip)
	testb	$1, %al
	je	.LBB14_83
	movq	%rdx, %r14
	movq	%r15, %rdi
	movb	$1, %r9b
.LBB14_70:
	leaq	(%r14,%r13), %r8
	incq	%r8
	addq	%r13, %r14
	cmpq	%rbx, %r14
	jae	.LBB14_72
	movq	-56(%rbp), %rax
	cmpb	$95, (%rax,%r14)
	je	.LBB14_84
.LBB14_72:
	movq	%r8, %r13
	cmpq	%rbx, %r8
	jbe	.LBB14_63
	movq	%r8, %r13
	jmp	.LBB14_75
.LBB14_74:
	movq	%rbx, %r13
.LBB14_75:
	movq	%r12, %r8
	movq	%rbx, %r14
.LBB14_76:
	movl	%r9d, -84(%rbp)
	testq	%rdi, %rdi
	je	.LBB14_80
	movq	%r12, %r15
	movq	%r8, %rbx
	movl	$1, %edx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.23(%rip), %rsi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB14_60
	cmpq	$0, -48(%rbp)
	je	.LBB14_81
	subq	%r15, %r14
	addq	-56(%rbp), %r15
	movq	-48(%rbp), %rdi
	movq	%r15, %rsi
	movq	%r14, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movq	-48(%rbp), %rcx
	movq	%rcx, -72(%rbp)
	movq	%rcx, %rdi
	testb	%al, %al
	movq	%rbx, %r8
	movq	-80(%rbp), %rbx
	jne	.LBB14_60
	jmp	.LBB14_82
.LBB14_80:
	xorl	%edi, %edi
	jmp	.LBB14_82
.LBB14_81:
	movq	$0, -72(%rbp)
	xorl	%edi, %edi
	movq	%rbx, %r8
	movq	-80(%rbp), %rbx
.LBB14_82:
	cmpb	$0, -84(%rbp)
	jne	.LBB14_39
	jmp	.LBB14_62
.LBB14_83:
	movq	%rbx, %r13
	movq	%r12, %r8
	movq	%rbx, %r14
	movq	%r15, %rdi
	movb	$1, %r9b
	jmp	.LBB14_76
.LBB14_84:
	xorl	%r9d, %r9d
	movq	%r8, %r13
	jmp	.LBB14_76
.LBB14_85:
	movb	$1, %r14b
	movq	%rbx, %r13
	movq	%rbx, %rdx
	movq	-48(%rbp), %rdi
	xorl	%r8d, %r8d
	jmp	.LBB14_35
.LBB14_86:
	xorl	%r14d, %r14d
	movq	%r13, %r8
	jmp	.LBB14_35
.Lfunc_end14:
	.size	_RNCNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB7_7Printer10print_type0B9_, .Lfunc_end14-_RNCNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB7_7Printer10print_type0B9_
	.cfi_endproc

	.section	.text._RNvCs8dXjxA0JZyF_14rustc_demangle12try_demangle,"ax",@progbits
	.globl	_RNvCs8dXjxA0JZyF_14rustc_demangle12try_demangle
	.type	_RNvCs8dXjxA0JZyF_14rustc_demangle12try_demangle,@function
_RNvCs8dXjxA0JZyF_14rustc_demangle12try_demangle:
.Lfunc_begin15:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	subq	$64, %rsp
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rdi, %rbx
	leaq	-80(%rbp), %r14
	movq	%r14, %rdi
	callq	*_RNvCs8dXjxA0JZyF_14rustc_demangle8demangle@GOTPCREL(%rip)
	cmpb	$0, (%r14)
	je	.LBB15_1
	vmovups	-80(%rbp), %zmm0
	vmovups	%zmm0, (%rbx)
	jmp	.LBB15_3
.LBB15_1:
	movq	$2, (%rbx)
.LBB15_3:
	movq	%rbx, %rax
	addq	$64, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.Lfunc_end15:
	.size	_RNvCs8dXjxA0JZyF_14rustc_demangle12try_demangle, .Lfunc_end15-_RNvCs8dXjxA0JZyF_14rustc_demangle12try_demangle
	.cfi_endproc

	.section	.text._RNvCs8dXjxA0JZyF_14rustc_demangle8demangle,"ax",@progbits
	.globl	_RNvCs8dXjxA0JZyF_14rustc_demangle8demangle
	.type	_RNvCs8dXjxA0JZyF_14rustc_demangle8demangle,@function
_RNvCs8dXjxA0JZyF_14rustc_demangle8demangle:
.Lfunc_begin16:
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
	subq	$184, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %r15
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.31(%rip), %rcx
	leaq	-176(%rbp), %r12
	movl	$6, %r8d
	movq	%r12, %rdi
	movq	%rsi, -56(%rbp)
	movq	%rdx, %r14
	callq	*_RNvMsu_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcher3new@GOTPCREL(%rip)
	movq	(%r12), %rax
	cmpq	$2, %rax
	je	.LBB16_7
	cmpl	$1, %eax
	jne	.LBB16_25
	movq	-168(%rbp), %rbx
	movq	-96(%rbp), %rdx
	subq	%rbx, %rdx
	jbe	.LBB16_53
	movb	-152(%rbp), %al
	movq	-104(%rbp), %rsi
	addq	%rbx, %rsi
	cmpq	$15, %rdx
	ja	.LBB16_46
	xorl	%ecx, %ecx
.LBB16_5:
	cmpb	%al, (%rsi,%rcx)
	je	.LBB16_48
	incq	%rcx
	cmpq	%rcx, %rdx
	jne	.LBB16_5
	jmp	.LBB16_53
.LBB16_7:
	movq	-120(%rbp), %r10
	movq	-104(%rbp), %rax
	movq	-80(%rbp), %r13
	leaq	-1(%r13), %r11
	movq	-96(%rbp), %r8
	movq	-88(%rbp), %r9
	cmpq	$-1, %r10
	je	.LBB16_29
	movq	-136(%rbp), %rcx
	leaq	(%rcx,%r11), %rdi
	cmpq	%r8, %rdi
	jae	.LBB16_53
	movq	-144(%rbp), %rdx
	movq	-168(%rbp), %rbx
	movq	-152(%rbp), %rsi
	movq	%r13, %r12
	movq	%rsi, -72(%rbp)
	subq	%rsi, %r12
	movq	%r12, -192(%rbp)
	movl	$1, %esi
	subq	%rbx, %rsi
	movq	%rsi, -184(%rbp)
	movq	%rdx, -64(%rbp)
.LBB16_10:
	movzbl	(%rax,%rdi), %esi
	btq	%rsi, %rdx
	jae	.LBB16_20
	cmpq	%r10, %rbx
	movq	%r10, %rdi
	cmovaq	%rbx, %rdi
	cmpq	%r13, %rdi
	movq	%r13, %rsi
	cmovaq	%rdi, %rsi
	jae	.LBB16_15
	leaq	(%rax,%rcx), %r12
.LBB16_13:
	movb	(%r9,%rdi), %dl
	cmpb	(%r12,%rdi), %dl
	jne	.LBB16_22
	incq	%rdi
	cmpq	%rdi, %rsi
	jne	.LBB16_13
.LBB16_15:
	cmpq	%rbx, %r10
	jae	.LBB16_49
	leaq	(%rax,%rcx), %rsi
	leaq	-1(%rbx), %rdi
.LBB16_17:
	cmpq	%r13, %rdi
	jae	.LBB16_190
	movb	(%r9,%rdi), %dl
	cmpb	(%rsi,%rdi), %dl
	jne	.LBB16_21
	cmpq	%rdi, %r10
	leaq	-1(%rdi), %rdi
	jae	.LBB16_49
	jmp	.LBB16_17
.LBB16_20:
	addq	%r13, %rcx
	xorl	%r10d, %r10d
	jmp	.LBB16_24
.LBB16_21:
	addq	-72(%rbp), %rcx
	movq	-192(%rbp), %r10
	jmp	.LBB16_23
.LBB16_22:
	addq	-184(%rbp), %rcx
	addq	%rdi, %rcx
	xorl	%r10d, %r10d
.LBB16_23:
	movq	-64(%rbp), %rdx
.LBB16_24:
	leaq	(%rcx,%r11), %rdi
	cmpq	%r8, %rdi
	jb	.LBB16_10
	jmp	.LBB16_53
.LBB16_25:
	leaq	-216(%rbp), %r12
	leaq	-176(%rbp), %r13
.LBB16_26:
	movq	%r12, %rdi
	movq	%r13, %rsi
	callq	_RNvXsv_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcherNtB5_8Searcher4next
	movq	-216(%rbp), %rax
	cmpq	$1, %rax
	je	.LBB16_26
	testq	%rax, %rax
	jne	.LBB16_53
	movq	-208(%rbp), %rcx
	jmp	.LBB16_49
.LBB16_29:
	movq	-136(%rbp), %rcx
	leaq	(%rcx,%r11), %rdi
	cmpq	%r8, %rdi
	jae	.LBB16_53
	movq	-144(%rbp), %r10
	movq	-168(%rbp), %r11
	movq	-152(%rbp), %rdx
	movq	%rdx, -72(%rbp)
	leaq	-1(%r11), %r12
	leaq	(%r9,%r11), %rdx
	xorl	%ebx, %ebx
	movq	%r13, %rsi
	subq	%r11, %rsi
	cmovaeq	%rsi, %rbx
	movq	%r10, -64(%rbp)
.LBB16_31:
	movzbl	(%rax,%rdi), %esi
	btq	%rsi, %r10
	jae	.LBB16_41
	cmpq	%r13, %r11
	jae	.LBB16_36
	leaq	(%rax,%r11), %rsi
	addq	%rcx, %rsi
	xorl	%edi, %edi
.LBB16_34:
	movb	(%rdx,%rdi), %r10b
	cmpb	(%rsi,%rdi), %r10b
	jne	.LBB16_43
	incq	%rdi
	cmpq	%rdi, %rbx
	jne	.LBB16_34
.LBB16_36:
	testq	%r11, %r11
	je	.LBB16_49
	leaq	(%rax,%rcx), %rsi
	movq	%r12, %rdi
.LBB16_38:
	cmpq	%r13, %r12
	jae	.LBB16_190
	movb	(%r9,%rdi), %r10b
	cmpb	(%rsi,%rdi), %r10b
	jne	.LBB16_42
	addq	$-1, %rdi
	jae	.LBB16_49
	jmp	.LBB16_38
.LBB16_41:
	addq	%r13, %rcx
	jmp	.LBB16_45
.LBB16_42:
	addq	-72(%rbp), %rcx
	jmp	.LBB16_44
.LBB16_43:
	addq	%rdi, %rcx
	incq	%rcx
.LBB16_44:
	movq	-64(%rbp), %r10
.LBB16_45:
	leaq	-1(%r13), %rsi
	leaq	(%rcx,%rsi), %rdi
	cmpq	%r8, %rdi
	jb	.LBB16_31
	jmp	.LBB16_53
.LBB16_46:
	movzbl	%al, %edi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice6memchr14memchr_aligned@GOTPCREL(%rip)
	testb	$1, %al
	je	.LBB16_53
	movq	%rdx, %rcx
.LBB16_48:
	addq	%rbx, %rcx
.LBB16_49:
	movq	%r14, %rsi
	movq	%rcx, %rdx
	addq	$6, %rdx
	movq	-56(%rbp), %r14
	je	.LBB16_55
	cmpq	%rsi, %rdx
	jae	.LBB16_54
	cmpb	$-65, (%r14,%rdx)
	jg	.LBB16_55
.LBB16_52:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.33(%rip), %r8
	jmp	.LBB16_196
.LBB16_53:
	movq	%r14, %rsi
	movq	-56(%rbp), %r14
	jmp	.LBB16_71
.LBB16_54:
	jne	.LBB16_52
.LBB16_55:
	addq	%r14, %rdx
	leaq	(%r14,%rsi), %rax
.LBB16_56:
	cmpq	%rax, %rdx
	je	.LBB16_65
	movzbl	(%rdx), %r10d
	testb	%r10b, %r10b
	js	.LBB16_59
	incq	%rdx
	jmp	.LBB16_64
.LBB16_59:
	movl	%r10d, %edi
	andl	$31, %edi
	movzbl	1(%rdx), %r9d
	andl	$63, %r9d
	cmpb	$-33, %r10b
	jbe	.LBB16_62
	movzbl	2(%rdx), %r8d
	shll	$6, %r9d
	andl	$63, %r8d
	orl	%r9d, %r8d
	cmpb	$-16, %r10b
	jb	.LBB16_63
	movzbl	3(%rdx), %r10d
	addq	$4, %rdx
	andl	$7, %edi
	shll	$18, %edi
	shll	$6, %r8d
	andl	$63, %r10d
	orl	%r8d, %r10d
	orl	%edi, %r10d
	jmp	.LBB16_64
.LBB16_62:
	addq	$2, %rdx
	shll	$6, %edi
	orl	%r9d, %edi
	movl	%edi, %r10d
	jmp	.LBB16_64
.LBB16_63:
	addq	$3, %rdx
	shll	$12, %edi
	orl	%edi, %r8d
	movl	%r8d, %r10d
.LBB16_64:
	leal	-58(%r10), %edi
	cmpl	$-10, %edi
	setb	%dil
	addl	$-71, %r10d
	cmpl	$-7, %r10d
	setb	%r8b
	testb	%dil, %r8b
	je	.LBB16_56
	jmp	.LBB16_71
.LBB16_65:
	testq	%rcx, %rcx
	je	.LBB16_69
	cmpq	%rsi, %rcx
	jae	.LBB16_70
	cmpb	$-65, (%r14,%rcx)
	jle	.LBB16_192
	movq	%rcx, %rsi
	jmp	.LBB16_71
.LBB16_69:
	movl	$1, %ebx
	xorl	%esi, %esi
	xorl	%r12d, %r12d
	jmp	.LBB16_158
.LBB16_70:
	jne	.LBB16_192
.LBB16_71:
	cmpq	$3, %rsi
	jae	.LBB16_74
	cmpq	$2, %rsi
	jne	.LBB16_135
	cmpw	$20058, (%r14)
	je	.LBB16_86
	jmp	.LBB16_132
.LBB16_74:
	movzwl	(%r14), %eax
	xorl	$23135, %eax
	movzbl	2(%r14), %ecx
	xorl	$78, %ecx
	orw	%ax, %cx
	je	.LBB16_81
	cmpw	$20058, (%r14)
	je	.LBB16_85
	cmpq	$3, %rsi
	je	.LBB16_128
	cmpl	$1314545503, (%r14)
	jne	.LBB16_128
	movl	$4, %r8d
	movq	$-4, %r12
	cmpq	$5, %rsi
	jb	.LBB16_87
	cmpb	$-65, 4(%r14)
	jg	.LBB16_87
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.113(%rip), %r8
	movl	$4, %edx
	jmp	.LBB16_196
.LBB16_81:
	movl	$3, %r8d
	movq	$-3, %r12
	cmpq	$3, %rsi
	je	.LBB16_87
	cmpb	$-65, 3(%r14)
	jg	.LBB16_87
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.115(%rip), %r8
.LBB16_84:
	movl	$3, %edx
	jmp	.LBB16_196
.LBB16_85:
	cmpb	$-65, 2(%r14)
	jle	.LBB16_194
.LBB16_86:
	movl	$2, %r8d
	movq	$-2, %r12
.LBB16_87:
	movq	%rsi, -64(%rbp)
	addq	%rsi, %r12
	je	.LBB16_127
	addq	%r14, %r8
	leaq	(%r8,%r12), %rcx
	xorl	%eax, %eax
.LBB16_89:
	cmpb	$0, (%r8,%rax)
	js	.LBB16_127
	incq	%rax
	cmpq	%rax, %r12
	jne	.LBB16_89
	movzbl	(%r8), %r9d
	testb	%r9b, %r9b
	js	.LBB16_93
	leaq	1(%r8), %rbx
	jmp	.LBB16_98
.LBB16_93:
	movl	%r9d, %eax
	andl	$31, %eax
	movzbl	1(%r8), %esi
	andl	$63, %esi
	cmpb	$-33, %r9b
	jbe	.LBB16_96
	movzbl	2(%r8), %edx
	shll	$6, %esi
	andl	$63, %edx
	orl	%esi, %edx
	cmpb	$-16, %r9b
	jb	.LBB16_97
	leaq	4(%r8), %rbx
	movzbl	3(%r8), %r9d
	andl	$7, %eax
	shll	$18, %eax
	shll	$6, %edx
	andl	$63, %r9d
	orl	%edx, %r9d
	orl	%eax, %r9d
	jmp	.LBB16_98
.LBB16_96:
	leaq	2(%r8), %rbx
	shll	$6, %eax
	orl	%esi, %eax
	movl	%eax, %r9d
	jmp	.LBB16_98
.LBB16_97:
	leaq	3(%r8), %rbx
	shll	$12, %eax
	orl	%eax, %edx
	movl	%edx, %r9d
.LBB16_98:
	xorl	%r13d, %r13d
	cmpl	$69, %r9d
	jne	.LBB16_100
.LBB16_99:
	subq	%rbx, %rcx
	movq	-64(%rbp), %rsi
	jmp	.LBB16_164
.LBB16_100:
	movl	$10, %r11d
.LBB16_101:
	addl	$-48, %r9d
	cmpl	$9, %r9d
	ja	.LBB16_127
	xorl	%eax, %eax
.LBB16_103:
	movq	%rax, %rsi
	mulq	%r11
	jo	.LBB16_127
	movl	%r9d, %r10d
	addq	%r10, %rax
	jb	.LBB16_127
	cmpq	%rcx, %rbx
	je	.LBB16_127
	movzbl	(%rbx), %edx
	testb	%dl, %dl
	js	.LBB16_108
	incq	%rbx
	jmp	.LBB16_114
.LBB16_108:
	movl	%edx, %r9d
	andl	$31, %r9d
	movzbl	1(%rbx), %edi
	andl	$63, %edi
	cmpb	$-33, %dl
	jbe	.LBB16_111
	movzbl	2(%rbx), %r11d
	shll	$6, %edi
	andl	$63, %r11d
	orl	%edi, %r11d
	cmpb	$-16, %dl
	jb	.LBB16_112
	movzbl	3(%rbx), %edx
	addq	$4, %rbx
	andl	$7, %r9d
	shll	$18, %r9d
	shll	$6, %r11d
	andl	$63, %edx
	orl	%r11d, %edx
	orl	%r9d, %edx
	jmp	.LBB16_113
.LBB16_111:
	addq	$2, %rbx
	shll	$6, %r9d
	orl	%edi, %r9d
	movl	%r9d, %edx
	jmp	.LBB16_114
.LBB16_112:
	addq	$3, %rbx
	shll	$12, %r9d
	orl	%r9d, %r11d
	movl	%r11d, %edx
.LBB16_113:
	movl	$10, %r11d
.LBB16_114:
	leal	-48(%rdx), %r9d
	cmpl	$10, %r9d
	jb	.LBB16_103
	testq	%rax, %rax
	je	.LBB16_126
	leaq	(%rsi,%rsi,4), %rax
	leaq	(%r10,%rax,2), %rax
.LBB16_117:
	cmpq	%rcx, %rbx
	je	.LBB16_127
	movzbl	(%rbx), %edx
	testb	%dl, %dl
	js	.LBB16_120
	incq	%rbx
	jmp	.LBB16_125
.LBB16_120:
	movl	%edx, %esi
	andl	$31, %esi
	movzbl	1(%rbx), %edi
	andl	$63, %edi
	cmpb	$-33, %dl
	jbe	.LBB16_123
	movzbl	2(%rbx), %r9d
	shll	$6, %edi
	andl	$63, %r9d
	orl	%edi, %r9d
	cmpb	$-16, %dl
	jb	.LBB16_124
	movzbl	3(%rbx), %edx
	addq	$4, %rbx
	andl	$7, %esi
	shll	$18, %esi
	shll	$6, %r9d
	andl	$63, %edx
	orl	%r9d, %edx
	orl	%esi, %edx
	jmp	.LBB16_125
.LBB16_123:
	addq	$2, %rbx
	shll	$6, %esi
	orl	%edi, %esi
	movl	%esi, %edx
	jmp	.LBB16_125
.LBB16_124:
	addq	$3, %rbx
	shll	$12, %esi
	orl	%esi, %r9d
	movl	%r9d, %edx
.LBB16_125:
	decq	%rax
	jne	.LBB16_117
.LBB16_126:
	incq	%r13
	movl	%edx, %r9d
	cmpl	$69, %edx
	jne	.LBB16_101
	jmp	.LBB16_99
.LBB16_127:
	movq	-64(%rbp), %rsi
	cmpq	$3, %rsi
	jb	.LBB16_132
.LBB16_128:
	cmpw	$21087, (%r14)
	je	.LBB16_136
	cmpb	$82, (%r14)
	je	.LBB16_133
	movl	$1, %ebx
	cmpq	$3, %rsi
	jne	.LBB16_156
	movl	$3, %esi
	jmp	.LBB16_157
.LBB16_132:
	movl	$2, %esi
	cmpb	$82, (%r14)
	jne	.LBB16_135
.LBB16_133:
	movb	1(%r14), %al
	cmpb	$-65, %al
	jle	.LBB16_191
	leaq	1(%r14), %r12
	movq	$-1, %r13
	jmp	.LBB16_138
.LBB16_135:
	movl	$1, %ebx
	jmp	.LBB16_157
.LBB16_136:
	movb	2(%r14), %al
	cmpb	$-65, %al
	jle	.LBB16_193
	leaq	2(%r14), %r12
	movq	$-2, %r13
.LBB16_138:
	addb	$-65, %al
	movl	$1, %ebx
	cmpb	$25, %al
	ja	.LBB16_157
	addq	%rsi, %r13
	je	.LBB16_143
	xorl	%r8d, %r8d
	xorl	%eax, %eax
.LBB16_141:
	cmpb	$0, (%r12,%rax)
	js	.LBB16_154
	incq	%rax
	cmpq	%rax, %r13
	jne	.LBB16_141
.LBB16_143:
	movq	%rsi, %r14
	leaq	-176(%rbp), %rdi
	movq	%r12, (%rdi)
	movq	%r13, 8(%rdi)
	xorl	%eax, %eax
	movq	%rax, 16(%rdi)
	movl	%eax, 24(%rdi)
	movq	%rax, 32(%rdi)
	movl	%eax, 40(%rdi)
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	testb	%al, %al
	jne	.LBB16_189
	movq	-176(%rbp), %rdi
	testq	%rdi, %rdi
	je	.LBB16_155
	movq	-168(%rbp), %rcx
	movq	%rcx, %rax
	shrq	$8, %rax
	movq	-160(%rbp), %rdx
	cmpq	%rcx, %rdx
	movq	%r14, %rsi
	jae	.LBB16_150
	movb	(%rdi,%rdx), %r8b
	addb	$-65, %r8b
	cmpb	$26, %r8b
	jae	.LBB16_150
	leaq	-176(%rbp), %rax
	vmovsd	24(%rax), %xmm0
	movq	%rdi, (%rax)
	movq	%rcx, 8(%rax)
	movq	%rdx, 16(%rax)
	vmovlps	%xmm0, 24(%rax)
	movq	$0, 32(%rax)
	movl	$0, 40(%rax)
	movq	%rax, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	testb	%al, %al
	jne	.LBB16_189
	movq	-176(%rbp), %rdi
	testq	%rdi, %rdi
	movq	%r14, %rsi
	je	.LBB16_188
	movb	-168(%rbp), %cl
	movzbl	-161(%rbp), %eax
	shll	$16, %eax
	movzwl	-163(%rbp), %edx
	orl	%eax, %edx
	shlq	$32, %rdx
	movl	-167(%rbp), %eax
	orq	%rdx, %rax
	movq	-160(%rbp), %rdx
.LBB16_150:
	movq	-56(%rbp), %r14
	shlq	$8, %rax
	movzbl	%cl, %ecx
	orq	%rax, %rcx
	testq	%rdx, %rdx
	je	.LBB16_163
	cmpq	%rdx, %rcx
	jbe	.LBB16_162
	cmpb	$-65, (%rdi,%rdx)
	jg	.LBB16_163
.LBB16_153:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.107(%rip), %r8
	movq	%rcx, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB16_154:
	xorl	%r12d, %r12d
	jmp	.LBB16_159
.LBB16_155:
	xorl	%r12d, %r12d
	xorl	%r8d, %r8d
	xorl	%eax, %eax
	xorl	%ecx, %ecx
	jmp	.LBB16_184
.LBB16_156:
	movzwl	(%r14), %eax
	xorl	$24415, %eax
	movzbl	2(%r14), %ecx
	xorl	$82, %ecx
	orw	%ax, %cx
	je	.LBB16_186
.LBB16_157:
	xorl	%r12d, %r12d
.LBB16_158:
	xorl	%r8d, %r8d
.LBB16_159:
	xorl	%eax, %eax
.LBB16_160:
	xorl	%ecx, %ecx
.LBB16_161:
	movq	%rax, (%r15)
	movq	%r8, 8(%r15)
	movq	%r12, 16(%r15)
	movq	%r13, 24(%r15)
	movq	%r14, 32(%r15)
	movq	%rsi, 40(%r15)
	movq	%rbx, 48(%r15)
	movq	%rcx, 56(%r15)
	movq	%r15, %rax
	addq	$184, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB16_162:
	.cfi_def_cfa %rbp, 16
	jne	.LBB16_153
.LBB16_163:
	addq	%rdx, %rdi
	subq	%rdx, %rcx
	xorl	%r8d, %r8d
	movq	%rdi, %rbx
.LBB16_164:
	testq	%rcx, %rcx
	je	.LBB16_181
	cmpb	$46, (%rbx)
	jne	.LBB16_182
	movq	%rsi, %r14
	leaq	(%rbx,%rcx), %rdx
	movb	$46, %r10b
	xorl	%eax, %eax
	movq	%rbx, %rdi
.LBB16_167:
	testb	%r10b, %r10b
	js	.LBB16_169
	incq	%rdi
	movzbl	%r10b, %r9d
	jmp	.LBB16_174
.LBB16_169:
	movl	%r10d, %esi
	andb	$31, %sil
	movzbl	%sil, %r9d
	movzbl	1(%rdi), %esi
	andl	$63, %esi
	cmpb	$-33, %r10b
	jbe	.LBB16_172
	movzbl	2(%rdi), %r11d
	shll	$6, %esi
	andl	$63, %r11d
	orl	%esi, %r11d
	cmpb	$-16, %r10b
	jb	.LBB16_173
	movzbl	3(%rdi), %esi
	addq	$4, %rdi
	andl	$7, %r9d
	shll	$18, %r9d
	shll	$6, %r11d
	andl	$63, %esi
	orl	%r11d, %esi
	orl	%r9d, %esi
	movl	%esi, %r9d
	jmp	.LBB16_174
.LBB16_172:
	addq	$2, %rdi
	shll	$6, %r9d
	orl	%esi, %r9d
	jmp	.LBB16_174
.LBB16_173:
	addq	$3, %rdi
	shll	$12, %r9d
	orl	%r9d, %r11d
	movl	%r11d, %r9d
.LBB16_174:
	movl	%r9d, %esi
	andl	$2097119, %esi
	addl	$-65, %esi
	cmpl	$26, %esi
	setb	%sil
	leal	-48(%r9), %r10d
	cmpl	$10, %r10d
	setb	%r10b
	leal	-33(%r9), %r11d
	cmpl	$15, %r11d
	setb	%r11b
	orb	%r10b, %r11b
	orb	%sil, %r11b
	jne	.LBB16_177
	leal	-58(%r9), %esi
	cmpl	$38, %esi
	ja	.LBB16_179
	movabsq	$541165879423, %r10
	btq	%rsi, %r10
	jae	.LBB16_179
.LBB16_177:
	cmpq	%rdx, %rdi
	je	.LBB16_183
	movb	(%rdi), %r10b
	jmp	.LBB16_167
.LBB16_179:
	addl	$-127, %r9d
	cmpl	$-4, %r9d
	jae	.LBB16_177
	xorl	%ecx, %ecx
	movl	$1, %ebx
	jmp	.LBB16_184
.LBB16_181:
	movl	$1, %eax
	jmp	.LBB16_160
.LBB16_182:
	xorl	%eax, %eax
	xorl	%ecx, %ecx
	movl	$1, %ebx
	jmp	.LBB16_161
.LBB16_183:
	movl	$1, %eax
.LBB16_184:
	movq	%r14, %rsi
.LBB16_185:
	movq	-56(%rbp), %r14
	jmp	.LBB16_161
.LBB16_186:
	movb	3(%r14), %al
	cmpb	$-65, %al
	jle	.LBB16_197
	leaq	3(%r14), %r12
	movq	$-3, %r13
	jmp	.LBB16_138
.LBB16_188:
	xorl	%r12d, %r12d
	xorl	%r8d, %r8d
	xorl	%eax, %eax
	xorl	%ecx, %ecx
	jmp	.LBB16_185
.LBB16_189:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.3(%rip), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.39(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.106(%rip), %r8
	leaq	-41(%rbp), %rdx
	movl	$61, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip)
.LBB16_190:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.11(%rip), %rdx
	movq	%r13, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.LBB16_191:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.104(%rip), %r8
	movl	$1, %edx
	jmp	.LBB16_196
.LBB16_192:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.34(%rip), %r8
	movq	%r14, %rdi
	xorl	%edx, %edx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB16_193:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.105(%rip), %r8
	jmp	.LBB16_195
.LBB16_194:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.114(%rip), %r8
.LBB16_195:
	movl	$2, %edx
.LBB16_196:
	movq	%r14, %rdi
	movq	%rsi, %rcx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB16_197:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.103(%rip), %r8
	jmp	.LBB16_84
.Lfunc_end16:
	.size	_RNvCs8dXjxA0JZyF_14rustc_demangle8demangle, .Lfunc_end16-_RNvCs8dXjxA0JZyF_14rustc_demangle8demangle
	.cfi_endproc

	.section	.text._RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc11is_assigned,"ax",@progbits
	.type	_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc11is_assigned,@function
_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc11is_assigned:
.Lfunc_begin17:
	.cfi_startproc
	movb	$1, %al
	cmpl	$888, %edi
	jae	.LBB17_1
.LBB17_4:
	retq
.LBB17_1:
	cmpl	$262142, %edi
	jae	.LBB17_2
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	callq	*_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data13cn_planes_0_311lookup_slow@GOTPCREL(%rip)
	xorb	$1, %al
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	.cfi_restore %rbp
	retq
.LBB17_2:
	cmpl	$917505, %edi
	sete	%cl
	leal	-917536(%rdi), %edx
	cmpl	$96, %edx
	setb	%dl
	leal	-917760(%rdi), %esi
	cmpl	$240, %esi
	setb	%sil
	orb	%dl, %sil
	leal	-983040(%rdi), %edx
	cmpl	$65534, %edx
	setb	%dl
	orb	%cl, %dl
	orb	%sil, %dl
	jne	.LBB17_4
	addl	$-1048576, %edi
	cmpl	$65534, %edi
	setb	%al
	retq
.Lfunc_end17:
	.size	_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc11is_assigned, .Lfunc_end17-_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc11is_assigned
	.cfi_endproc

	.section	.rodata.cst16,"aM",@progbits,16
	.p2align	4, 0x0
.LCPI18_0:
	.long	0
	.long	4294967169
	.long	4294909952
	.long	4293984256
.LCPI18_1:
	.long	32
	.long	33
	.long	6400
	.long	65534
	.section	.text._RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc16escape_debug_ext,"ax",@progbits
	.type	_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc16escape_debug_ext,@function
_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc16escape_debug_ext:
.Lfunc_begin18:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	subq	$16, %rsp
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movl	%esi, %r14d
	movq	%rdi, %rbx
	cmpl	$12, %esi
	jle	.LBB18_1
	cmpl	$38, %r14d
	jg	.LBB18_9
	cmpl	$13, %r14d
	je	.LBB18_15
	cmpl	$34, %r14d
	jne	.LBB18_11
	movw	$8796, (%rbx)
	jmp	.LBB18_17
.LBB18_1:
	testl	%r14d, %r14d
	je	.LBB18_16
	cmpl	$9, %r14d
	je	.LBB18_14
	cmpl	$10, %r14d
	jne	.LBB18_11
	movw	$28252, (%rbx)
	jmp	.LBB18_17
.LBB18_9:
	cmpl	$39, %r14d
	je	.LBB18_19
	cmpl	$92, %r14d
	jne	.LBB18_11
	movw	$23644, (%rbx)
	jmp	.LBB18_17
.LBB18_16:
	movw	$12380, (%rbx)
	jmp	.LBB18_17
.LBB18_11:
	leal	-32(%r14), %eax
	cmpl	$95, %eax
	setb	%al
	movl	%r14d, %ecx
	andl	$2097150, %ecx
	cmpl	$65438, %ecx
	sete	%cl
	orb	%al, %cl
	je	.LBB18_20
.LBB18_12:
	movl	%r14d, (%rbx)
	movb	$-127, %cl
	movb	$-128, %al
	jmp	.LBB18_18
.LBB18_15:
	movw	$29276, (%rbx)
	jmp	.LBB18_17
.LBB18_19:
	movw	$10076, (%rbx)
	jmp	.LBB18_17
.LBB18_14:
	movw	$29788, (%rbx)
.LBB18_17:
	movq	$0, 2(%rbx)
	movb	$2, %cl
	xorl	%eax, %eax
.LBB18_18:
	movb	%al, 12(%rbx)
	movb	%cl, 13(%rbx)
	addq	$16, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB18_20:
	.cfi_def_cfa %rbp, 16
	vpbroadcastd	%r14d, %xmm0
	vpaddd	.LCPI18_0(%rip), %xmm0, %xmm0
	vpcmpltud	.LCPI18_1(%rip), %xmm0, %k0
	kortestb	%k0, %k0
	jne	.LBB18_30
	leal	-1048576(%r14), %eax
	cmpl	$65534, %eax
	setb	%al
	cmpl	$32, %r14d
	sete	%cl
	orb	%al, %cl
	jne	.LBB18_30
	cmpl	$133, %r14d
	jae	.LBB18_23
.LBB18_29:
	movl	%r14d, %edi
	callq	_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc11is_assigned
	testb	%al, %al
	jne	.LBB18_12
	jmp	.LBB18_30
.LBB18_23:
	movl	%r14d, %edi
	callq	_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space6lookup
	testb	%al, %al
	je	.LBB18_24
.LBB18_30:
	movl	%r14d, %eax
	orl	$1, %eax
	lzcntl	%eax, %ecx
	shrl	$2, %ecx
	leaq	-2(%rcx), %rax
	movb	$0, -24(%rbp)
	movw	$0, -26(%rbp)
	movl	%r14d, %esi
	shrl	$20, %esi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.12(%rip), %rdx
	movb	(%rsi,%rdx), %sil
	movb	%sil, -23(%rbp)
	movl	%r14d, %esi
	shrl	$16, %esi
	andl	$15, %esi
	movb	(%rsi,%rdx), %sil
	movb	%sil, -22(%rbp)
	movl	%r14d, %esi
	shrl	$12, %esi
	andl	$15, %esi
	movb	(%rsi,%rdx), %sil
	movb	%sil, -21(%rbp)
	movl	%r14d, %esi
	shrl	$8, %esi
	andl	$15, %esi
	movb	(%rsi,%rdx), %sil
	movb	%sil, -20(%rbp)
	movl	%r14d, %esi
	shrl	$4, %esi
	andl	$15, %esi
	movb	(%rsi,%rdx), %sil
	movb	%sil, -19(%rbp)
	andl	$15, %r14d
	movb	(%r14,%rdx), %dl
	movb	%dl, -18(%rbp)
	movb	$125, -17(%rbp)
	movw	$30044, -28(%rbp,%rcx)
	movb	$123, -26(%rbp,%rcx)
	movzwl	-18(%rbp), %ecx
	movw	%cx, 8(%rbx)
	movq	-26(%rbp), %rcx
	movq	%rcx, (%rbx)
	movb	$10, %cl
	jmp	.LBB18_18
.LBB18_24:
	cmpl	$768, %r14d
	jb	.LBB18_26
	movl	%r14d, %edi
	callq	*_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data15grapheme_extend11lookup_slow@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB18_30
	jmp	.LBB18_27
.LBB18_26:
	cmpl	$173, %r14d
	jb	.LBB18_29
.LBB18_27:
	movl	%r14d, %edi
	callq	*_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data28default_ignorable_code_point11lookup_slow@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB18_30
	movl	%r14d, %edi
	callq	*_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data2cf11lookup_slow@GOTPCREL(%rip)
	testb	%al, %al
	je	.LBB18_29
	jmp	.LBB18_30
.Lfunc_end18:
	.size	_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc16escape_debug_ext, .Lfunc_end18-_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc16escape_debug_ext
	.cfi_endproc

	.section	.text._RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint,"ax",@progbits
	.type	_RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint,@function
_RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint:
.Lfunc_begin19:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%rbx
	pushq	%rax
	.cfi_offset %rbx, -24
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.40(%rip), %rdx
	movl	$1, %ebx
	movl	$1, %ecx
	callq	*_RINvMNtCs2k2z8Zem4rB_4core3stre18trim_start_matchesReECs8dXjxA0JZyF_14rustc_demangle@GOTPCREL(%rip)
	cmpq	$16, %rdx
	jbe	.LBB19_3
	xorl	%ebx, %ebx
	jmp	.LBB19_2
.LBB19_3:
	movq	%rdx, %rcx
	testq	%rdx, %rdx
	je	.LBB19_4
	addq	%rax, %rcx
	xorl	%edx, %edx
.LBB19_6:
	movzbl	(%rax), %esi
	testb	%sil, %sil
	js	.LBB19_8
	incq	%rax
	jmp	.LBB19_13
.LBB19_8:
	movl	%esi, %edi
	andl	$31, %edi
	movzbl	1(%rax), %r9d
	andl	$63, %r9d
	cmpb	$-33, %sil
	jbe	.LBB19_9
	movzbl	2(%rax), %r8d
	shll	$6, %r9d
	andl	$63, %r8d
	orl	%r9d, %r8d
	cmpb	$-16, %sil
	jb	.LBB19_11
	movzbl	3(%rax), %esi
	addq	$4, %rax
	andl	$7, %edi
	shll	$18, %edi
	shll	$6, %r8d
	andl	$63, %esi
	orl	%r8d, %esi
	orl	%edi, %esi
	jmp	.LBB19_13
.LBB19_9:
	addq	$2, %rax
	shll	$6, %edi
	orl	%r9d, %edi
	movl	%edi, %esi
	jmp	.LBB19_13
.LBB19_11:
	addq	$3, %rax
	shll	$12, %edi
	orl	%edi, %r8d
	movl	%r8d, %esi
.LBB19_13:
	leal	-65(%rsi), %r8d
	andl	$-33, %r8d
	addl	$10, %r8d
	leal	-48(%rsi), %edi
	cmpl	$58, %esi
	cmovael	%r8d, %edi
	cmpl	$16, %edi
	jae	.LBB19_15
	movq	%rdx, %rsi
	shlq	$4, %rsi
	movl	%edi, %edx
	orq	%rsi, %rdx
	cmpq	%rcx, %rax
	je	.LBB19_2
	jmp	.LBB19_6
.LBB19_4:
	xorl	%edx, %edx
.LBB19_2:
	movq	%rbx, %rax
	addq	$8, %rsp
	popq	%rbx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB19_15:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.41(%rip), %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip)
.Lfunc_end19:
	.size	_RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint, .Lfunc_end19-_RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint
	.cfi_endproc

	.section	.text._RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62,"ax",@progbits
	.type	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62,@function
_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62:
.Lfunc_begin20:
	.cfi_startproc
	movq	(%rsi), %r8
	movq	8(%rsi), %r9
	movq	16(%rsi), %rcx
	cmpq	%r9, %rcx
	jae	.LBB20_2
	cmpb	$95, (%r8,%rcx)
	jne	.LBB20_2
	incq	%rcx
	movq	%rcx, 16(%rsi)
	movq	$0, 8(%rdi)
	jmp	.LBB20_5
.LBB20_2:
	cmpq	%r9, %rcx
	cmovaq	%rcx, %r9
	jae	.LBB20_19
	incq	%rcx
	negq	%r9
	xorl	%eax, %eax
	movl	$62, %r10d
.LBB20_8:
	movb	-1(%r8,%rcx), %dl
	cmpb	$95, %dl
	je	.LBB20_13
	leal	-48(%rdx), %r11d
	cmpb	$10, %r11b
	jb	.LBB20_17
	leal	-97(%rdx), %r11d
	cmpb	$26, %r11b
	jae	.LBB20_11
	addb	$-87, %dl
	jmp	.LBB20_16
.LBB20_11:
	leal	-65(%rdx), %r11d
	cmpb	$26, %r11b
	jae	.LBB20_19
	addb	$-29, %dl
.LBB20_16:
	movl	%edx, %r11d
.LBB20_17:
	movq	%rcx, 16(%rsi)
	mulq	%r10
	jo	.LBB20_19
	movzbl	%r11b, %edx
	addq	%rdx, %rax
	jb	.LBB20_19
	leaq	(%r9,%rcx), %rdx
	incq	%rdx
	incq	%rcx
	cmpq	$1, %rdx
	jne	.LBB20_8
.LBB20_19:
	movb	$0, 1(%rdi)
	movb	$1, %al
	jmp	.LBB20_6
.LBB20_13:
	movq	%rcx, 16(%rsi)
	cmpq	$-1, %rax
	je	.LBB20_19
	incq	%rax
	movq	%rax, 8(%rdi)
.LBB20_5:
	xorl	%eax, %eax
.LBB20_6:
	movb	%al, (%rdi)
	retq
.Lfunc_end20:
	.size	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62, .Lfunc_end20-_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62
	.cfi_endproc

	.section	.text._RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles,"ax",@progbits
	.type	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles,@function
_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles:
.Lfunc_begin21:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rsi, %r8
	movq	16(%rsi), %rdx
	movq	8(%rsi), %rsi
	cmpq	%rsi, %rdx
	movq	%rsi, %r9
	cmovaq	%rdx, %r9
	jae	.LBB21_12
	movq	(%r8), %rax
	movq	%rdx, %rcx
	subq	%r9, %rcx
	xorl	%r9d, %r9d
	movq	%rax, %r10
.LBB21_3:
	movb	(%r10,%rdx), %r11b
	leal	-48(%r11), %ebx
	cmpb	$10, %bl
	setb	%bl
	leal	-97(%r11), %r14d
	cmpb	$6, %r14b
	setb	%r14b
	orb	%bl, %r14b
	je	.LBB21_4
	decq	%r9
	incq	%r10
	cmpq	%r9, %rcx
	jne	.LBB21_3
	subq	%r9, %rdx
	movq	%rdx, 16(%r8)
	jmp	.LBB21_12
.LBB21_4:
	movq	%rdx, %rcx
	subq	%r9, %rcx
	leaq	1(%rcx), %r10
	movq	%r10, 16(%r8)
	cmpb	$95, %r11b
	jne	.LBB21_12
	cmpq	%rsi, %rcx
	ja	.LBB21_13
	testq	%rdx, %rdx
	je	.LBB21_9
	cmpq	%rsi, %rdx
	je	.LBB21_9
	cmpb	$-64, (%rax,%rdx)
	jl	.LBB21_13
.LBB21_9:
	addq	%rdx, %rax
	negq	%r9
	movq	%rax, (%rdi)
	movq	%r9, 8(%rdi)
	jmp	.LBB21_10
.LBB21_12:
	movb	$0, 8(%rdi)
	movq	$0, (%rdi)
.LBB21_10:
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB21_13:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.42(%rip), %r8
	movq	%rax, %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.Lfunc_end21:
	.size	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles, .Lfunc_end21-_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles
	.cfi_endproc

	.section	.text._RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62,"ax",@progbits
	.type	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62,@function
_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62:
.Lfunc_begin22:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	subq	$16, %rsp
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rdi, %rbx
	movq	16(%rsi), %rax
	cmpq	8(%rsi), %rax
	jae	.LBB22_4
	movq	(%rsi), %rcx
	cmpb	%dl, (%rcx,%rax)
	jne	.LBB22_4
	incq	%rax
	movq	%rax, 16(%rsi)
	leaq	-32(%rbp), %r14
	movq	%r14, %rdi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62
	cmpb	$0, (%r14)
	je	.LBB22_7
	movb	-31(%rbp), %al
	movb	%al, 1(%rbx)
.LBB22_10:
	movb	$1, %al
	jmp	.LBB22_6
.LBB22_4:
	movq	$0, 8(%rbx)
.LBB22_5:
	xorl	%eax, %eax
.LBB22_6:
	movb	%al, (%rbx)
	addq	$16, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB22_7:
	.cfi_def_cfa %rbp, 16
	movq	-24(%rbp), %rax
	cmpq	$-1, %rax
	je	.LBB22_9
	incq	%rax
	movq	%rax, 8(%rbx)
	jmp	.LBB22_5
.LBB22_9:
	movb	$0, 1(%rbx)
	jmp	.LBB22_10
.Lfunc_end22:
	.size	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62, .Lfunc_end22-_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62
	.cfi_endproc

	.section	.text._RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident,"ax",@progbits
	.type	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident,@function
_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident:
.Lfunc_begin23:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%rbx
	pushq	%rax
	.cfi_offset %rbx, -24
	movq	%rsi, %r9
	movq	8(%rsi), %rsi
	movq	16(%r9), %r11
	cmpq	%rsi, %r11
	jae	.LBB23_1
	movq	(%r9), %rax
	cmpb	$117, (%rax,%r11)
	jne	.LBB23_1
	incq	%r11
	movq	%r11, 16(%r9)
	movb	$1, %r8b
	jmp	.LBB23_4
.LBB23_1:
	xorl	%r8d, %r8d
.LBB23_4:
	cmpq	%rsi, %r11
	jae	.LBB23_27
	movq	(%r9), %r10
	movb	(%r10,%r11), %al
	addb	$-48, %al
	cmpb	$10, %al
	jae	.LBB23_27
	incq	%r11
	movq	%r11, 16(%r9)
	testb	%al, %al
	je	.LBB23_7
	movzbl	%al, %eax
	cmpq	%rsi, %r11
	je	.LBB23_12
	movl	$10, %ecx
.LBB23_24:
	movb	(%r10,%r11), %bl
	addb	$-48, %bl
	cmpb	$9, %bl
	ja	.LBB23_8
	incq	%r11
	movq	%r11, 16(%r9)
	mulq	%rcx
	jo	.LBB23_27
	movzbl	%bl, %edx
	addq	%rdx, %rax
	jb	.LBB23_27
	cmpq	%r11, %rsi
	jne	.LBB23_24
.LBB23_12:
	movq	%rsi, %r11
	jmp	.LBB23_13
.LBB23_7:
	xorl	%eax, %eax
.LBB23_8:
	cmpq	%rsi, %r11
	jae	.LBB23_13
	cmpb	$95, (%r10,%r11)
	jne	.LBB23_13
	incq	%r11
	movq	%r11, 16(%r9)
.LBB23_13:
	movq	%r11, %rcx
	addq	%rax, %rcx
	jb	.LBB23_27
	movq	%rcx, 16(%r9)
	cmpq	%rsi, %rcx
	jbe	.LBB23_15
.LBB23_27:
	movb	$0, 8(%rdi)
	movq	$0, (%rdi)
.LBB23_21:
	addq	$8, %rsp
	popq	%rbx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB23_15:
	.cfi_def_cfa %rbp, 16
	cmpq	%rsi, %r11
	je	.LBB23_19
	testq	%r11, %r11
	je	.LBB23_17
	cmpb	$-65, (%r10,%r11)
	jle	.LBB23_29
.LBB23_17:
	cmpq	%rsi, %rcx
	je	.LBB23_19
	cmpb	$-65, (%r10,%rcx)
	jle	.LBB23_29
.LBB23_19:
	addq	%r11, %r10
	testb	%r8b, %r8b
	je	.LBB23_20
	movl	$1, %edx
	testq	%rax, %rax
	je	.LBB23_31
	leaq	(%r10,%rax), %r8
	xorl	%ecx, %ecx
	xorl	%esi, %esi
.LBB23_34:
	cmpb	$95, -1(%r8,%rsi)
	je	.LBB23_35
	decq	%rsi
	movq	%rax, %r9
	addq	%rsi, %r9
	jne	.LBB23_34
	jmp	.LBB23_46
.LBB23_20:
	movq	%r10, (%rdi)
	movq	%rax, 8(%rdi)
	movq	$1, 16(%rdi)
	movq	$0, 24(%rdi)
	jmp	.LBB23_21
.LBB23_31:
	xorl	%ecx, %ecx
	jmp	.LBB23_46
.LBB23_35:
	leaq	(%rax,%rsi), %rdx
	movq	%rdx, %rcx
	decq	%rcx
	je	.LBB23_40
	cmpq	%rax, %rcx
	jae	.LBB23_37
	cmpb	$-65, -1(%r8,%rsi)
	jg	.LBB23_40
	jmp	.LBB23_49
.LBB23_37:
	cmpq	$1, %rsi
	jne	.LBB23_49
	testq	%rdx, %rdx
	je	.LBB23_39
.LBB23_40:
	cmpq	%rax, %rdx
	jae	.LBB23_41
	cmpb	$-64, (%r8,%rsi)
	jl	.LBB23_42
	movq	%rdx, %r8
	jmp	.LBB23_45
.LBB23_41:
	movq	%rax, %r8
	testq	%rsi, %rsi
	je	.LBB23_45
.LBB23_42:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.45(%rip), %r8
	movq	%r10, %rdi
	movq	%rax, %rsi
	movq	%rax, %rcx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB23_39:
	xorl	%r8d, %r8d
.LBB23_45:
	subq	%r8, %rax
	addq	%r10, %r8
	movq	%r10, %rdx
	movq	%r8, %r10
.LBB23_46:
	testq	%rax, %rax
	je	.LBB23_27
	movq	%rdx, (%rdi)
	movq	%rcx, 8(%rdi)
	movq	%r10, 16(%rdi)
	movq	%rax, 24(%rdi)
	jmp	.LBB23_21
.LBB23_29:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.43(%rip), %r8
	movq	%r10, %rdi
	movq	%r11, %rdx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB23_49:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.44(%rip), %r8
	movq	%r10, %rdi
	movq	%rax, %rsi
	xorl	%edx, %edx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.Lfunc_end23:
	.size	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident, .Lfunc_end23-_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident
	.cfi_endproc

	.section	.text._RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref,"ax",@progbits
	.type	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref,@function
_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref:
.Lfunc_begin24:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r12
	pushq	%rbx
	subq	$16, %rsp
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r14
	movq	%rdi, %rbx
	movq	16(%rsi), %r12
	leaq	-48(%rbp), %r15
	movq	%r15, %rdi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62
	cmpb	$1, (%r15)
	jne	.LBB24_2
	movb	-47(%rbp), %al
	movb	%al, 8(%rbx)
	jmp	.LBB24_6
.LBB24_2:
	decq	%r12
	movq	-40(%rbp), %rax
	cmpq	%r12, %rax
	jae	.LBB24_8
	movl	24(%r14), %ecx
	incl	%ecx
	cmpl	$500, %ecx
	jbe	.LBB24_4
	movb	$1, 8(%rbx)
	jmp	.LBB24_6
.LBB24_8:
	movb	$0, 8(%rbx)
.LBB24_6:
	movq	$0, (%rbx)
.LBB24_7:
	addq	$16, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB24_4:
	.cfi_def_cfa %rbp, 16
	vmovups	(%r14), %xmm0
	vmovups	%xmm0, (%rbx)
	movq	%rax, 16(%rbx)
	movl	%ecx, 24(%rbx)
	jmp	.LBB24_7
.Lfunc_end24:
	.size	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref, .Lfunc_end24-_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref
	.cfi_endproc

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path:
.Lfunc_begin25:
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
	subq	$104, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %rbx
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB25_1
	movl	24(%rbx), %ecx
	incl	%ecx
	movl	%ecx, 24(%rbx)
	cmpl	$501, %ecx
	jb	.LBB25_7
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_6
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rsi
	movl	$25, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
.LBB25_6:
	movq	$0, (%rbx)
	movb	$1, 8(%rbx)
	jmp	.LBB25_32
.LBB25_7:
	movl	%esi, %r15d
	movq	8(%rbx), %rsi
	movq	16(%rbx), %rcx
	cmpq	%rsi, %rcx
	jae	.LBB25_8
	movb	(%rax,%rcx), %r12b
	leaq	1(%rcx), %rdi
	movq	%rdi, 16(%rbx)
	movzbl	%r12b, %edx
	cmpl	$76, %edx
	jle	.LBB25_11
	cmpl	$87, %edx
	jg	.LBB25_24
	cmpl	$77, %edx
	je	.LBB25_98
	cmpl	$78, %edx
	jne	.LBB25_8
	cmpq	%rsi, %rdi
	jae	.LBB25_23
	movzbl	1(%rax,%rcx), %r13d
	addq	$2, %rcx
	movq	%rcx, 16(%rbx)
	leal	-65(%r13), %eax
	leal	-123(%r13), %ecx
	cmpb	$-26, %cl
	movl	$1, %ecx
	movabsq	$-4294967296, %rdx
	cmovbq	%rcx, %rdx
	shlq	$32, %r13
	cmpb	$26, %al
	cmovaeq	%rdx, %r13
	cmpb	$1, %r13b
	jne	.LBB25_57
	shrq	$8, %r13
	jmp	.LBB25_53
.LBB25_11:
	cmpl	$66, %edx
	je	.LBB25_26
	cmpl	$67, %edx
	je	.LBB25_27
	cmpl	$73, %edx
	jne	.LBB25_8
	movzbl	%r15b, %esi
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
	testb	%r15b, %r15b
	je	.LBB25_16
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48(%rip), %rsi
	movl	$2, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
.LBB25_16:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_18
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
.LBB25_18:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_17print_generic_argEB8_
	testq	%rax, %rax
	je	.LBB25_104
	jmp	.LBB25_111
.LBB25_24:
	cmpl	$88, %edx
	je	.LBB25_98
	cmpl	$89, %edx
	je	.LBB25_100
.LBB25_8:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_56
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
.LBB25_55:
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
.LBB25_56:
	movq	$0, (%rbx)
	movb	$0, 8(%rbx)
	jmp	.LBB25_32
.LBB25_98:
	leaq	-72(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	movl	$115, %edx
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62
	cmpb	$1, (%r14)
	jne	.LBB25_99
.LBB25_63:
	movb	-71(%rbp), %r15b
.LBB25_36:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_39
	movzbl	%r15b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
	jmp	.LBB25_38
.LBB25_26:
	movzbl	%r15b, %esi
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_10print_paths_0EB8_
	movb	$1, %r14b
	jmp	.LBB25_78
.LBB25_99:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer17skipping_printingNCNvB2_10print_path0EB8_
.LBB25_100:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_102
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
.LBB25_102:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
	cmpb	$77, %r12b
	je	.LBB25_104
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_108
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.55(%rip), %rsi
	movl	$4, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
.LBB25_108:
	movq	%rbx, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	testb	%al, %al
	jne	.LBB25_111
.LBB25_104:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_49
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.56(%rip), %rsi
.LBB25_97:
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB25_78:
	testb	%al, %al
	jne	.LBB25_111
.LBB25_49:
	cmpq	$0, (%rbx)
	je	.LBB25_32
	decl	24(%rbx)
	jmp	.LBB25_32
.LBB25_27:
	leaq	-72(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	movl	$115, %edx
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62
	cmpb	$1, (%r14)
	jne	.LBB25_30
	movb	-71(%rbp), %r15b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_39
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%r15b, %r15b
	cmovneq	%rax, %rsi
	movzbl	%r15b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
.LBB25_38:
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
.LBB25_39:
	movq	$0, (%rbx)
	movb	%r15b, 8(%rbx)
	jmp	.LBB25_32
.LBB25_23:
	xorl	%r13d, %r13d
.LBB25_53:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_56
	movl	%r13d, %eax
	andl	$1, %r13d
	leaq	16(,%r13,8), %rdx
	addq	%r13, %rdx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	$1, %al
	cmovneq	%rcx, %rsi
	jmp	.LBB25_55
.LBB25_30:
	movq	-64(%rbp), %r15
	movq	%r15, -80(%rbp)
	cmpq	$0, (%rbx)
	je	.LBB25_31
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident
	cmpq	$0, (%r14)
	je	.LBB25_35
	vmovups	-72(%rbp), %ymm0
	vmovups	%ymm0, -144(%rbp)
	movq	32(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB25_49
	leaq	-144(%rbp), %rdi
	vzeroupper
	callq	*_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
	movq	32(%rbx), %rax
	testq	%rax, %rax
	je	.LBB25_49
	testq	%r15, %r15
	je	.LBB25_49
	movl	$8388608, %ecx
	andl	16(%rax), %ecx
	jne	.LBB25_49
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.46(%rip), %rsi
	movl	$1, %edx
	callq	*24(%rax)
	testb	%al, %al
	jne	.LBB25_48
	movq	32(%rbx), %rsi
	leaq	-80(%rbp), %rdi
	callq	*_RNvXsC_NtNtCs2k2z8Zem4rB_4core3fmt3numyNtB7_8LowerHex3fmt@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_48
	movq	32(%rbx), %rax
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.47(%rip), %rsi
	movl	$1, %edx
	callq	*24(%rax)
	testb	%al, %al
	je	.LBB25_49
.LBB25_48:
	movb	$1, %r14b
	jmp	.LBB25_111
.LBB25_57:
	movzbl	%r15b, %esi
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB25_111
	cmpq	$0, (%rbx)
	je	.LBB25_59
.LBB25_62:
	leaq	-72(%rbp), %r15
	movq	%r15, %rdi
	movq	%rbx, %rsi
	movl	$115, %edx
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser14opt_integer_62
	cmpb	$1, (%r15)
	je	.LBB25_63
	cmpq	$0, (%rbx)
	je	.LBB25_1
	movq	8(%r15), %r12
	movq	%r15, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser5ident
	cmpq	$0, (%r15)
	je	.LBB25_35
	shrq	$32, %r13
	vmovups	-72(%rbp), %ymm0
	vmovups	%ymm0, -112(%rbp)
	cmpl	$-1, %r13d
	jne	.LBB25_67
	movq	-88(%rbp), %rax
	orq	-104(%rbp), %rax
	je	.LBB25_49
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_49
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48(%rip), %rsi
	movl	$2, %edx
	vzeroupper
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
	movq	32(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB25_49
	leaq	-112(%rbp), %rdi
	callq	*_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt@GOTPCREL(%rip)
	jmp	.LBB25_78
.LBB25_31:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_32
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movl	%eax, %r14d
	jmp	.LBB25_111
.LBB25_35:
	movb	-64(%rbp), %r15b
	jmp	.LBB25_36
.LBB25_59:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_32
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48(%rip), %rsi
	movl	$2, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
	cmpq	$0, (%rbx)
	jne	.LBB25_62
.LBB25_1:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_32
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$104, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB25_32:
	.cfi_def_cfa %rbp, 16
	xorl	%r14d, %r14d
.LBB25_111:
	movl	%r14d, %eax
	addq	$104, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB25_67:
	.cfi_def_cfa %rbp, 16
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_69
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.49(%rip), %rsi
	movl	$3, %edx
	vzeroupper
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
.LBB25_69:
	cmpl	$67, %r13d
	je	.LBB25_79
	cmpl	$83, %r13d
	jne	.LBB25_71
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_85
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.51(%rip), %rsi
	movl	$4, %edx
	jmp	.LBB25_83
.LBB25_79:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_85
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.50(%rip), %rsi
	movl	$7, %edx
.LBB25_83:
	vzeroupper
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	jmp	.LBB25_84
.LBB25_71:
	movl	%r13d, -72(%rbp)
	movq	32(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB25_85
	leaq	-72(%rbp), %rdi
	vzeroupper
	callq	*_RNvXsk_NtCs2k2z8Zem4rB_4core3fmtcNtB5_7Display3fmt@GOTPCREL(%rip)
.LBB25_84:
	testb	%al, %al
	jne	.LBB25_111
.LBB25_85:
	movq	-88(%rbp), %rax
	orq	-104(%rbp), %rax
	movq	32(%rbx), %rdi
	jne	.LBB25_86
.LBB25_91:
	testq	%rdi, %rdi
	je	.LBB25_49
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.53(%rip), %rsi
	movl	$1, %edx
	vzeroupper
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
	movq	32(%rbx), %rsi
	movq	%r12, -72(%rbp)
	testq	%rsi, %rsi
	je	.LBB25_49
	leaq	-72(%rbp), %rdi
	callq	*_RNvXsd_NtNtNtCs2k2z8Zem4rB_4core3fmt3num3impyNtB9_7Display3fmt@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB25_49
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.30(%rip), %rsi
	jmp	.LBB25_97
.LBB25_86:
	testq	%rdi, %rdi
	je	.LBB25_49
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.52(%rip), %rsi
	movl	$1, %edx
	vzeroupper
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
	movq	32(%rbx), %rsi
	testq	%rsi, %rsi
	je	.LBB25_49
	leaq	-112(%rbp), %rdi
	callq	*_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB25_111
	movq	32(%rbx), %rdi
	jmp	.LBB25_91
.Lfunc_end25:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path, .Lfunc_end25-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	.cfi_endproc

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type:
.Lfunc_begin26:
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
	movq	%rdi, %rbx
	movq	(%rdi), %r14
	testq	%r14, %r14
	je	.LBB26_12
	movq	8(%rbx), %r12
	movq	16(%rbx), %r13
	cmpq	%r12, %r13
	jae	.LBB26_7
	cmpb	$119, (%r14,%r13)
	jne	.LBB26_7
	incq	%r13
	movq	%r13, 16(%rbx)
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_7
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.57(%rip), %rsi
	movl	$9, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
	movq	(%rbx), %r14
	testq	%r14, %r14
	je	.LBB26_12
	movq	8(%rbx), %r12
	movq	16(%rbx), %r13
.LBB26_7:
	cmpq	%r12, %r13
	jae	.LBB26_8
	movzbl	(%r14,%r13), %r15d
	leaq	1(%r13), %rax
	movq	%rax, 16(%rbx)
	movl	%r15d, %edi
	callq	_RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type
	testq	%rax, %rax
	je	.LBB26_18
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_60
	movq	%rax, %rsi
	jmp	.LBB26_17
.LBB26_12:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_60
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
.LBB26_17:
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB26_8:
	.cfi_def_cfa %rbp, 16
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_69
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
.LBB26_10:
	testb	%al, %al
	jne	.LBB26_11
.LBB26_69:
	movq	$0, (%rbx)
	movb	$0, 8(%rbx)
.LBB26_60:
	xorl	%r14d, %r14d
.LBB26_11:
	movl	%r14d, %eax
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB26_18:
	.cfi_def_cfa %rbp, 16
	leaq	1(%r13), %rdx
	movl	24(%rbx), %eax
	incl	%eax
	movl	%eax, 24(%rbx)
	cmpl	$500, %eax
	jbe	.LBB26_19
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_23
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rsi
	movl	$25, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
.LBB26_23:
	movq	$0, (%rbx)
	movb	$1, 8(%rbx)
	jmp	.LBB26_60
.LBB26_19:
	leal	-65(%r15), %eax
	cmpl	$22, %eax
	ja	.LBB26_91
	leaq	.LJTI26_0(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB26_37:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_39
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.46(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
.LBB26_39:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
	cmpl	$65, %r15d
	jne	.LBB26_44
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_43
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.63(%rip), %rsi
	movl	$2, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB26_11
.LBB26_43:
	movq	%rbx, %rdi
	movl	$1, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	testb	%al, %al
	jne	.LBB26_11
.LBB26_44:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_58
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.47(%rip), %rsi
	jmp	.LBB26_46
.LBB26_32:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_34
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.61(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
.LBB26_34:
	cmpl	$80, %r15d
	jne	.LBB26_83
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_31
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.62(%rip), %rsi
	movl	$6, %edx
	jmp	.LBB26_85
.LBB26_24:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_28
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.58(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
	movq	(%rbx), %r14
	testq	%r14, %r14
	je	.LBB26_30
	movq	8(%rbx), %r12
	movq	16(%rbx), %rdx
.LBB26_28:
	cmpq	%r12, %rdx
	jae	.LBB26_30
	cmpb	$76, (%r14,%rdx)
	jne	.LBB26_30
	incq	%rdx
	movq	%rdx, 16(%rbx)
	leaq	-56(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62
	cmpb	$1, (%r14)
	jne	.LBB26_79
.LBB26_76:
	movb	-55(%rbp), %r15b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_78
	movzbl	%r15b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
.LBB26_78:
	movq	$0, (%rbx)
	movb	%r15b, 8(%rbx)
	jmp	.LBB26_60
.LBB26_47:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_49
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
.LBB26_49:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_10print_typeEB8_
	movb	$1, %r14b
	testb	$1, %al
	jne	.LBB26_11
	cmpq	$1, %rdx
	jne	.LBB26_53
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_58
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.65(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB26_11
.LBB26_53:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_58
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25(%rip), %rsi
.LBB26_46:
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	jmp	.LBB26_57
.LBB26_71:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_74
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.67(%rip), %rsi
	movl	$4, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB26_11
.LBB26_74:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat
	jmp	.LBB26_57
.LBB26_70:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNvB2_10print_typeEB8_
	jmp	.LBB26_56
.LBB26_55:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_type0EB8_
	jmp	.LBB26_56
.LBB26_61:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_63
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.66(%rip), %rsi
	movl	$4, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
.LBB26_63:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer9in_binderNCNvB2_10print_types_0EB8_
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
	movq	(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB26_67
	movq	16(%rbx), %rax
	cmpq	8(%rbx), %rax
	jae	.LBB26_67
	cmpb	$76, (%rcx,%rax)
	jne	.LBB26_67
	incq	%rax
	movq	%rax, 16(%rbx)
	leaq	-56(%rbp), %r15
	movq	%r15, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser10integer_62
	cmpb	$1, (%r15)
	je	.LBB26_76
	movq	-48(%rbp), %r15
	testq	%r15, %r15
	je	.LBB26_58
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_90
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.27(%rip), %rsi
	movl	$3, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB26_11
.LBB26_90:
	movq	%rbx, %rdi
	movq	%r15, %rsi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index
	jmp	.LBB26_57
.LBB26_91:
	movq	%r13, 16(%rbx)
	movq	%rbx, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	jmp	.LBB26_56
.LBB26_67:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_69
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	jmp	.LBB26_10
.LBB26_79:
	movq	-48(%rbp), %rsi
	testq	%rsi, %rsi
	je	.LBB26_30
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_30
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.59(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB26_11
.LBB26_30:
	cmpl	$82, %r15d
	je	.LBB26_31
.LBB26_83:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB26_31
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.60(%rip), %rsi
	movl	$4, %edx
.LBB26_85:
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB26_11
.LBB26_31:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
.LBB26_56:
	movb	$1, %r14b
.LBB26_57:
	testb	%al, %al
	jne	.LBB26_11
.LBB26_58:
	cmpq	$0, (%rbx)
	je	.LBB26_60
	decl	24(%rbx)
	jmp	.LBB26_60
.Lfunc_end26:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type, .Lfunc_end26-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type
	.cfi_endproc
	.section	.rodata._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_type,"a",@progbits
	.p2align	2, 0x0
.LJTI26_0:
	.long	.LBB26_37-.LJTI26_0
	.long	.LBB26_70-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_61-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_55-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_32-.LJTI26_0
	.long	.LBB26_32-.LJTI26_0
	.long	.LBB26_24-.LJTI26_0
	.long	.LBB26_24-.LJTI26_0
	.long	.LBB26_37-.LJTI26_0
	.long	.LBB26_47-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_91-.LJTI26_0
	.long	.LBB26_71-.LJTI26_0

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const:
.Lfunc_begin27:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r12
	pushq	%rbx
	subq	$16, %rsp
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %rbx
	movq	(%rdi), %rcx
	testq	%rcx, %rcx
	je	.LBB27_1
	movl	%esi, %r14d
	movq	8(%rbx), %rsi
	movq	16(%rbx), %rax
	cmpq	%rsi, %rax
	jae	.LBB27_4
	movzbl	(%rcx,%rax), %r12d
	leaq	1(%rax), %rdx
	movq	%rdx, 16(%rbx)
	movl	24(%rbx), %edi
	incl	%edi
	movl	%edi, 24(%rbx)
	cmpl	$500, %edi
	jbe	.LBB27_9
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_15
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rsi
	movl	$25, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_15:
	movq	$0, (%rbx)
	movb	$1, 8(%rbx)
	jmp	.LBB27_23
.LBB27_9:
	leal	-81(%r12), %edi
	cmpl	$40, %edi
	ja	.LBB27_10
	leaq	.LJTI27_0(%rip), %r8
	movslq	(%r8,%rdi,4), %rdi
	addq	%r8, %rdi
	jmpq	*%rdi
.LBB27_24:
	cmpq	%rsi, %rdx
	jae	.LBB27_28
	cmpb	$110, (%rcx,%rdx)
	jne	.LBB27_28
	addq	$2, %rax
	movq	%rax, 16(%rbx)
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_28
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.23(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_28:
	movq	%rbx, %rdi
	movl	%r12d, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer16print_const_uint
.LBB27_20:
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_21:
	cmpq	$0, (%rbx)
	je	.LBB27_23
	decl	24(%rbx)
	jmp	.LBB27_23
.LBB27_10:
	cmpl	$65, %r12d
	je	.LBB27_70
	cmpl	$66, %r12d
	jne	.LBB27_4
	movzbl	%r14b, %esi
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer13print_backrefNCNvB2_11print_consts4_0EB8_
	jmp	.LBB27_20
.LBB27_29:
	leaq	-48(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles
	movq	(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB27_30
	movq	-40(%rbp), %rsi
	callq	_RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint
	cmpq	$1, %rax
	jne	.LBB27_4
	testq	%rdx, %rdx
	je	.LBB27_40
	cmpq	$1, %rdx
	jne	.LBB27_4
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.70(%rip), %rsi
	movl	$4, %edx
	jmp	.LBB27_19
.LBB27_16:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68(%rip), %rsi
	jmp	.LBB27_18
.LBB27_59:
	cmpq	%rsi, %rdx
	jae	.LBB27_61
	cmpb	$101, (%rcx,%rdx)
	jne	.LBB27_61
	addq	$2, %rax
	movq	%rax, 16(%rbx)
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer23print_const_str_literal
	jmp	.LBB27_20
.LBB27_61:
	testb	%r14b, %r14b
	jne	.LBB27_64
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_64
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_64:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_66
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.58(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_66:
	cmpl	$82, %r12d
	je	.LBB27_67
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_67
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.60(%rip), %rsi
	movl	$4, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_67:
	movq	%rbx, %rdi
	movl	$1, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	jmp	.LBB27_54
.LBB27_42:
	leaq	-48(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles
	movq	(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB27_43
	movq	-40(%rbp), %rsi
	callq	_RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint
	movq	%rdx, %rcx
	shrq	$32, %rcx
	sete	%cl
	andb	%al, %cl
	cmpb	$1, %cl
	jne	.LBB27_4
	movl	%edx, %eax
	xorl	$55296, %eax
	addl	$-1114112, %eax
	cmpl	$-1112064, %eax
	jae	.LBB27_47
.LBB27_4:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_39
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
.LBB27_6:
	testb	%al, %al
	jne	.LBB27_7
.LBB27_39:
	movq	$0, (%rbx)
	movb	$0, 8(%rbx)
	jmp	.LBB27_23
.LBB27_80:
	testb	%r14b, %r14b
	jne	.LBB27_83
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_83
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_83:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_85
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_85:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts1_0EB8_
	movb	$1, %r15b
	testb	$1, %al
	jne	.LBB27_7
	cmpq	$1, %rdx
	jne	.LBB27_89
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.65(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB27_7
.LBB27_89:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25(%rip), %rsi
	jmp	.LBB27_78
.LBB27_91:
	testb	%r14b, %r14b
	jne	.LBB27_94
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_94
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_94:
	movq	%rbx, %rdi
	movl	$1, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
	movq	(%rbx), %rax
	testq	%rax, %rax
	je	.LBB27_1
	movq	16(%rbx), %rcx
	cmpq	8(%rbx), %rcx
	jae	.LBB27_97
	movzbl	(%rax,%rcx), %eax
	incq	%rcx
	movq	%rcx, 16(%rbx)
	cmpl	$83, %eax
	je	.LBB27_105
	cmpl	$84, %eax
	je	.LBB27_102
	cmpl	$85, %eax
	je	.LBB27_56
.LBB27_97:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_39
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	jmp	.LBB27_6
.LBB27_1:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_23
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$16, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB27_48:
	.cfi_def_cfa %rbp, 16
	testb	%r14b, %r14b
	jne	.LBB27_51
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_51
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_51:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_53
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.61(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_53:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer23print_const_str_literal
.LBB27_54:
	movb	$1, %r15b
.LBB27_55:
	testb	%al, %al
	jne	.LBB27_7
.LBB27_56:
	testb	%r14b, %r14b
	jne	.LBB27_21
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.30(%rip), %rsi
.LBB27_18:
	movl	$1, %edx
.LBB27_19:
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	jmp	.LBB27_20
.LBB27_70:
	testb	%r14b, %r14b
	jne	.LBB27_73
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_73
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_73:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_75
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.46(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_75:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts0_0EB8_
	movb	$1, %r15b
	testq	%rax, %rax
	jne	.LBB27_7
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.47(%rip), %rsi
.LBB27_78:
	movl	$1, %edx
.LBB27_79:
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	jmp	.LBB27_55
.LBB27_30:
	movb	-40(%rbp), %r14b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_33
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%r14b, %r14b
	cmovneq	%rax, %rsi
	movzbl	%r14b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	jmp	.LBB27_32
.LBB27_43:
	movb	-40(%rbp), %r14b
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_33
	movzbl	%r14b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
.LBB27_32:
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB27_7
.LBB27_33:
	movq	$0, (%rbx)
	movb	%r14b, 8(%rbx)
.LBB27_23:
	xorl	%r15d, %r15d
.LBB27_7:
	movl	%r15d, %eax
	addq	$16, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB27_47:
	.cfi_def_cfa %rbp, 16
	movq	32(%rbx), %rdi
	movl	%edx, %esi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer26print_quoted_escaped_charsINtNtNtNtCs2k2z8Zem4rB_4core4iter7sources4once4OncecEEB8_
	jmp	.LBB27_20
.LBB27_40:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.69(%rip), %rsi
	movl	$5, %edx
	jmp	.LBB27_19
.LBB27_102:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_104
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB27_7
.LBB27_104:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts0_0EB8_
	testq	%rax, %rax
	je	.LBB27_89
	jmp	.LBB27_7
.LBB27_105:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_107
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.72(%rip), %rsi
	movl	$3, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB27_7
.LBB27_107:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNCNvB2_11print_consts3_0EB8_
	testq	%rax, %rax
	jne	.LBB27_7
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB27_21
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.73(%rip), %rsi
	movl	$2, %edx
	jmp	.LBB27_79
.Lfunc_end27:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const, .Lfunc_end27-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	.cfi_endproc
	.section	.rodata._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const,"a",@progbits
	.p2align	2, 0x0
.LJTI27_0:
	.long	.LBB27_61-.LJTI27_0
	.long	.LBB27_59-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_80-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_91-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_24-.LJTI27_0
	.long	.LBB27_29-.LJTI27_0
	.long	.LBB27_42-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_48-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_28-.LJTI27_0
	.long	.LBB27_24-.LJTI27_0
	.long	.LBB27_28-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_24-.LJTI27_0
	.long	.LBB27_28-.LJTI27_0
	.long	.LBB27_24-.LJTI27_0
	.long	.LBB27_28-.LJTI27_0
	.long	.LBB27_16-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_24-.LJTI27_0
	.long	.LBB27_28-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_4-.LJTI27_0
	.long	.LBB27_24-.LJTI27_0
	.long	.LBB27_28-.LJTI27_0

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer16print_const_uint,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer16print_const_uint,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer16print_const_uint:
.Lfunc_begin28:
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
	movq	%rdi, %r14
	cmpq	$0, (%rdi)
	je	.LBB28_1
	movl	%esi, %ebx
	leaq	-56(%rbp), %r15
	movq	%r15, %rdi
	movq	%r14, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles
	movq	(%r15), %r12
	testq	%r12, %r12
	je	.LBB28_7
	movq	-48(%rbp), %r13
	movq	%r12, %rdi
	movq	%r13, %rsi
	callq	_RNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_10HexNibbles14try_parse_uint
	cmpq	$1, %rax
	jne	.LBB28_13
	movq	%rdx, -56(%rbp)
	movq	32(%r14), %r14
	testq	%r14, %r14
	je	.LBB28_2
	leaq	-56(%rbp), %rdi
	movq	%r14, %rsi
	callq	*_RNvXsd_NtNtNtCs2k2z8Zem4rB_4core3fmt3num3impyNtB9_7Display3fmt@GOTPCREL(%rip)
	movb	$1, %r15b
	jmp	.LBB28_16
.LBB28_1:
	movq	32(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB28_2
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
.LBB28_5:
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB28_7:
	.cfi_def_cfa %rbp, 16
	movb	-48(%rbp), %bl
	movq	32(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB28_9
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%bl, %bl
	cmovneq	%rax, %rsi
	movzbl	%bl, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB28_3
.LBB28_9:
	movq	$0, (%r14)
	movb	%bl, 8(%r14)
	jmp	.LBB28_2
.LBB28_13:
	movq	32(%r14), %r14
	testq	%r14, %r14
	je	.LBB28_2
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.75(%rip), %rsi
	movl	$2, %edx
	movq	%r14, %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r15b
	testb	%al, %al
	jne	.LBB28_3
	movq	%r14, %rdi
	movq	%r12, %rsi
	movq	%r13, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB28_16:
	testb	%al, %al
	jne	.LBB28_3
	testb	$-128, 18(%r14)
	jne	.LBB28_2
	movzbl	%bl, %edi
	callq	_RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type
	testq	%rax, %rax
	je	.LBB28_20
	movq	%r14, %rdi
	movq	%rax, %rsi
	jmp	.LBB28_5
.LBB28_2:
	xorl	%r15d, %r15d
.LBB28_3:
	movl	%r15d, %eax
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB28_20:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.76(%rip), %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip)
.Lfunc_end28:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer16print_const_uint, .Lfunc_end28-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer16print_const_uint
	.cfi_endproc

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer23print_const_str_literal,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer23print_const_str_literal,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer23print_const_str_literal:
.Lfunc_begin29:
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
	subq	$88, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %r14
	cmpq	$0, (%rdi)
	je	.LBB29_1
	leaq	-120(%rbp), %rbx
	movq	%rbx, %rdi
	movq	%r14, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser11hex_nibbles
	movq	(%rbx), %r15
	testq	%r15, %r15
	je	.LBB29_4
	movq	-112(%rbp), %r12
	testb	$1, %r12b
	jne	.LBB29_27
	movabsq	$9223372036854775806, %rax
	andq	%rax, %r12
	leaq	(%r15,%r12), %r13
	movq	%r15, (%rbx)
	movq	%r12, 8(%rbx)
	movq	%r13, 16(%rbx)
	movq	$0, 24(%rbx)
	movq	$2, 32(%rbx)
.LBB29_9:
	movq	%rbx, %rdi
	callq	_RNvXNtNtNtCs2k2z8Zem4rB_4core4iter7sources7from_fnINtB2_6FromFnNCNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB17_10HexNibbles19try_parse_str_charss0_0ENtNtNtB6_6traits8iterator8Iterator4nextB19_
	cmpl	$-2, %eax
	jb	.LBB29_9
	jne	.LBB29_27
	movq	32(%r14), %r14
	testq	%r14, %r14
	je	.LBB29_30
	movq	(%r14), %rdi
	movq	8(%r14), %rax
	movl	$34, %esi
	callq	*32(%rax)
	movb	$1, %bl
	testb	%al, %al
	jne	.LBB29_31
	leaq	-120(%rbp), %rax
	movq	%r15, (%rax)
	movq	%rax, %r15
	movq	%r12, 8(%rax)
	movq	%r13, 16(%rax)
	movq	$0, 24(%rax)
	movq	$2, 32(%rax)
.LBB29_14:
	leaq	-72(%rbp), %r12
.LBB29_15:
	movq	%r15, %rdi
	callq	_RNvXNtNtNtCs2k2z8Zem4rB_4core4iter7sources7from_fnINtB2_6FromFnNCNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB17_10HexNibbles19try_parse_str_charss0_0ENtNtNtB6_6traits8iterator8Iterator4nextB19_
	cmpl	$-2, %eax
	je	.LBB29_19
	cmpl	$39, %eax
	jne	.LBB29_17
	movq	(%r14), %rdi
	movq	8(%r14), %rax
	movl	$39, %esi
	callq	*32(%rax)
	testb	%al, %al
	je	.LBB29_15
	jmp	.LBB29_31
.LBB29_17:
	movq	%r14, -56(%rbp)
	cmpl	$-1, %eax
	je	.LBB29_18
	movq	%r12, %rdi
	movl	%eax, %esi
	callq	_RNvMNtNtCs2k2z8Zem4rB_4core4char7methodsc16escape_debug_ext
	movzbl	-59(%rbp), %eax
	movl	%eax, %r15d
	movzbl	-60(%rbp), %r14d
	cmpb	%al, %r14b
	cmoval	%r14d, %eax
	jae	.LBB29_32
	movl	-72(%rbp), %ecx
	movl	%ecx, -48(%rbp)
	movzbl	%al, %eax
	movq	%rax, -80(%rbp)
	movq	-56(%rbp), %rax
	movq	(%rax), %r12
	movq	8(%rax), %rax
	movq	32(%rax), %r13
.LBB29_24:
	movl	-48(%rbp), %esi
	cmpb	$-128, %r15b
	ja	.LBB29_26
	movzbl	-72(%rbp,%r14), %esi
.LBB29_26:
	movq	%r12, %rdi
	callq	*%r13
	testb	%al, %al
	jne	.LBB29_31
	incq	%r14
	cmpq	%r14, -80(%rbp)
	jne	.LBB29_24
.LBB29_32:
	movq	-56(%rbp), %r14
	leaq	-120(%rbp), %r15
	jmp	.LBB29_14
.LBB29_1:
	movq	32(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB29_30
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	addq	$88, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB29_27:
	.cfi_def_cfa %rbp, 16
	movq	32(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB29_29
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %bl
	testb	%al, %al
	jne	.LBB29_31
.LBB29_29:
	movq	$0, (%r14)
	movb	$0, 8(%r14)
	jmp	.LBB29_30
.LBB29_4:
	movb	-112(%rbp), %r15b
	movq	32(%r14), %rdi
	testq	%rdi, %rdi
	je	.LBB29_6
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%r15b, %r15b
	cmovneq	%rax, %rsi
	movzbl	%r15b, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %bl
	testb	%al, %al
	jne	.LBB29_31
.LBB29_6:
	movq	$0, (%r14)
	movb	%r15b, 8(%r14)
.LBB29_30:
	xorl	%ebx, %ebx
.LBB29_31:
	movl	%ebx, %eax
	addq	$88, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB29_19:
	.cfi_def_cfa %rbp, 16
	movq	(%r14), %rdi
	movq	8(%r14), %rax
	movl	$34, %esi
	addq	$88, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*32(%rax)
.LBB29_18:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.36(%rip), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.35(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.161(%rip), %r8
	leaq	-41(%rbp), %rdx
	movl	$43, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip)
.Lfunc_end29:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer23print_const_str_literal, .Lfunc_end29-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer23print_const_str_literal
	.cfi_endproc

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index:
.Lfunc_begin30:
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
	movq	32(%rdi), %rbx
	testq	%rbx, %rbx
	je	.LBB30_6
	movq	%rsi, %r12
	movq	%rdi, %r15
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.77(%rip), %rsi
	movl	$1, %edx
	movq	%rbx, %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB30_7
	testq	%r12, %r12
	je	.LBB30_8
	movl	40(%r15), %r13d
	subq	%r12, %r13
	jae	.LBB30_9
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	movq	%rbx, %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB30_7
	movq	$0, (%r15)
	movb	$0, 8(%r15)
.LBB30_6:
	xorl	%r14d, %r14d
.LBB30_7:
	movl	%r14d, %eax
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB30_8:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68(%rip), %rsi
	movl	$1, %edx
	movq	%rbx, %rdi
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB30_9:
	.cfi_def_cfa %rbp, 16
	cmpq	$26, %r13
	jae	.LBB30_11
	addl	$97, %r13d
	leaq	-44(%rbp), %rdi
	movl	%r13d, (%rdi)
	movq	%rbx, %rsi
	callq	*_RNvXsk_NtCs2k2z8Zem4rB_4core3fmtcNtB5_7Display3fmt@GOTPCREL(%rip)
	jmp	.LBB30_13
.LBB30_11:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68(%rip), %rsi
	movl	$1, %edx
	movq	%rbx, %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB30_7
	leaq	-56(%rbp), %rdi
	movq	%r13, (%rdi)
	movq	%rbx, %rsi
	callq	*_RNvXsd_NtNtNtCs2k2z8Zem4rB_4core3fmt3num3impyNtB9_7Display3fmt@GOTPCREL(%rip)
.LBB30_13:
	movl	%eax, %r14d
	jmp	.LBB30_7
.Lfunc_end30:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index, .Lfunc_end30-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer25print_lifetime_from_index
	.cfi_endproc

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer30print_path_maybe_open_generics,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer30print_path_maybe_open_generics,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer30print_path_maybe_open_generics:
.Lfunc_begin31:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	subq	$64, %rsp
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rdi, %rbx
	movq	(%rdi), %rcx
	testq	%rcx, %rcx
	je	.LBB31_14
	movq	16(%rbx), %rax
	cmpq	8(%rbx), %rax
	jae	.LBB31_14
	movzbl	(%rcx,%rax), %ecx
	cmpl	$73, %ecx
	je	.LBB31_15
	cmpl	$66, %ecx
	jne	.LBB31_14
	incq	%rax
	movq	%rax, 16(%rbx)
	leaq	-48(%rbp), %r14
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	_RNvMs2_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_6Parser7backref
	cmpq	$0, (%r14)
	je	.LBB31_5
	cmpq	$0, 32(%rbx)
	je	.LBB31_13
	vmovups	(%rbx), %ymm0
	vmovups	%ymm0, -80(%rbp)
	vmovups	-48(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	movq	%rbx, %rdi
	vzeroupper
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer30print_path_maybe_open_generics
	vmovups	-80(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
	jmp	.LBB31_9
.LBB31_14:
	movq	%rbx, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	movl	%eax, %r14d
	addb	%r14b, %r14b
.LBB31_8:
	movl	%r14d, %eax
.LBB31_9:
	addq	$64, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB31_15:
	.cfi_def_cfa %rbp, 16
	incq	%rax
	movq	%rax, 16(%rbx)
	movq	%rbx, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	movb	$2, %r14b
	testb	%al, %al
	jne	.LBB31_8
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB31_18
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54(%rip), %rsi
	movl	$1, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB31_8
.LBB31_18:
	movq	%rbx, %rdi
	callq	_RINvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB6_7Printer14print_sep_listNvB2_17print_generic_argEB8_
	movq	%rax, %r14
	incb	%r14b
	jmp	.LBB31_8
.LBB31_5:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB31_12
	movb	-40(%rbp), %al
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	testb	%al, %al
	cmovneq	%rcx, %rsi
	movzbl	%al, %eax
	leaq	(%rax,%rax,8), %rdx
	addq	$16, %rdx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	je	.LBB31_12
	movb	$2, %r14b
	jmp	.LBB31_8
.LBB31_12:
	vmovups	-48(%rbp), %ymm0
	vmovups	%ymm0, (%rbx)
.LBB31_13:
	xorl	%r14d, %r14d
	jmp	.LBB31_8
.Lfunc_end31:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer30print_path_maybe_open_generics, .Lfunc_end31-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer30print_path_maybe_open_generics
	.cfi_endproc

	.section	.text._RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat,"ax",@progbits
	.type	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat,@function
_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat:
.Lfunc_begin32:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r12
	pushq	%rbx
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %rbx
	movq	(%rdi), %rax
	testq	%rax, %rax
	je	.LBB32_1
	movq	16(%rbx), %rcx
	cmpq	8(%rbx), %rcx
	jae	.LBB32_4
	movzbl	(%rax,%rcx), %eax
	incq	%rcx
	movq	%rcx, 16(%rbx)
	cmpl	$78, %eax
	je	.LBB32_13
	cmpl	$79, %eax
	je	.LBB32_16
	cmpl	$82, %eax
	jne	.LBB32_4
	movq	%rbx, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB32_34
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB32_12
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.78(%rip), %rsi
	movl	$3, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	testb	%al, %al
	jne	.LBB32_34
.LBB32_12:
	movq	%rbx, %rdi
	xorl	%esi, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer11print_const
	jmp	.LBB32_15
.LBB32_4:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB32_31
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
.LBB32_30:
	testb	%al, %al
	jne	.LBB32_34
.LBB32_31:
	movq	$0, (%rbx)
	movb	$0, 8(%rbx)
	jmp	.LBB32_33
.LBB32_1:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB32_33
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2(%rip), %rsi
	movl	$1, %edx
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.LBB32_16:
	.cfi_def_cfa %rbp, 16
	movl	24(%rbx), %eax
	incl	%eax
	movl	%eax, 24(%rbx)
	cmpl	$500, %eax
	jbe	.LBB32_17
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB32_27
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1(%rip), %rsi
	movl	$25, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB32_34
.LBB32_27:
	movq	$0, (%rbx)
	movb	$1, 8(%rbx)
	jmp	.LBB32_33
.LBB32_13:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB32_33
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.80(%rip), %rsi
	movl	$5, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	movb	$1, %r14b
.LBB32_15:
	testb	%al, %al
	jne	.LBB32_34
.LBB32_33:
	xorl	%r14d, %r14d
.LBB32_34:
	movl	%r14d, %eax
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB32_17:
	.cfi_def_cfa %rbp, 16
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat
	movb	$1, %r14b
	testb	%al, %al
	jne	.LBB32_34
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.79(%rip), %r15
	movq	_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip), %r12
.LBB32_19:
	movq	(%rbx), %rcx
	testq	%rcx, %rcx
	je	.LBB32_28
	movq	16(%rbx), %rax
	cmpq	8(%rbx), %rax
	jae	.LBB32_22
	cmpb	$69, (%rcx,%rax)
	je	.LBB32_32
.LBB32_22:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB32_24
	movl	$3, %edx
	movq	%r15, %rsi
	callq	*%r12
	testb	%al, %al
	jne	.LBB32_34
.LBB32_24:
	movq	%rbx, %rdi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat
	testb	%al, %al
	je	.LBB32_19
	jmp	.LBB32_34
.LBB32_28:
	movq	32(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB32_31
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0(%rip), %rsi
	movl	$16, %edx
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
	jmp	.LBB32_30
.LBB32_32:
	incq	%rax
	movq	%rax, 16(%rbx)
	decl	24(%rbx)
	jmp	.LBB32_33
.Lfunc_end32:
	.size	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat, .Lfunc_end32-_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer9print_pat
	.cfi_endproc

	.section	.text._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type,"ax",@progbits
	.type	_RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type,@function
_RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type:
.Lfunc_begin33:
	.cfi_startproc
	addb	$-97, %dil
	cmpb	$25, %dil
	ja	.LBB33_1
	movzbl	%dil, %eax
	leaq	.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type(%rip), %rcx
	movzbl	(%rax,%rcx), %edx
	leaq	.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type.41(%rip), %rcx
	movq	(%rcx,%rax,8), %rax
	retq
.LBB33_1:
	xorl	%eax, %eax
	retq
.Lfunc_end33:
	.size	_RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type, .Lfunc_end33-_RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type
	.cfi_endproc

	.section	.text._RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space6lookup,"ax",@progbits
	.type	_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space6lookup,@function
_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space6lookup:
.Lfunc_begin34:
	.cfi_startproc
	movl	%edi, %ecx
	shrl	$8, %ecx
	xorl	%eax, %eax
	cmpl	$31, %ecx
	jg	.LBB34_5
	testl	%ecx, %ecx
	je	.LBB34_8
	cmpl	$22, %ecx
	jne	.LBB34_10
	cmpl	$5760, %edi
	jmp	.LBB34_4
.LBB34_5:
	cmpl	$32, %ecx
	je	.LBB34_9
	cmpl	$48, %ecx
	jne	.LBB34_10
	cmpl	$12288, %edi
.LBB34_4:
	sete	%al
	jmp	.LBB34_10
.LBB34_8:
	movzbl	%dil, %eax
	movq	_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space14WHITESPACE_MAP@GOTPCREL(%rip), %rcx
	movb	(%rcx,%rax), %al
	jmp	.LBB34_10
.LBB34_9:
	movzbl	%dil, %eax
	movq	_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space14WHITESPACE_MAP@GOTPCREL(%rip), %rcx
	movb	(%rcx,%rax), %al
	shrb	%al
.LBB34_10:
	andb	$1, %al
	retq
.Lfunc_end34:
	.size	_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space6lookup, .Lfunc_end34-_RNvNtNtNtCs2k2z8Zem4rB_4core7unicode12unicode_data11white_space6lookup
	.cfi_endproc

	.section	.text._RNvXNtCs8dXjxA0JZyF_14rustc_demangle6legacyNtB2_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,"ax",@progbits
	.globl	_RNvXNtCs8dXjxA0JZyF_14rustc_demangle6legacyNtB2_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.type	_RNvXNtCs8dXjxA0JZyF_14rustc_demangle6legacyNtB2_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,@function
_RNvXNtCs8dXjxA0JZyF_14rustc_demangle6legacyNtB2_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt:
.Lfunc_begin35:
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
	subq	$72, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	16(%rdi), %rax
	movq	%rax, -88(%rbp)
	testq	%rax, %rax
	je	.LBB35_170
	movl	16(%rsi), %eax
	movq	(%rsi), %rcx
	movq	%rcx, -48(%rbp)
	movq	%rsi, -96(%rbp)
	movq	8(%rsi), %rcx
	movq	%rcx, -56(%rbp)
	movq	(%rdi), %r12
	movq	8(%rdi), %rsi
	xorl	%r13d, %r13d
	andl	$8388608, %eax
	movl	%eax, -76(%rbp)
.LBB35_2:
	testq	%rsi, %rsi
	je	.LBB35_178
	leaq	1(%r13), %rax
	movq	%rax, -112(%rbp)
	movb	(%r12), %al
	movzbl	%al, %ecx
	leaq	-1(%rsi), %rdx
	xorl	%r8d, %r8d
	xorl	%ebx, %ebx
	movl	%eax, %r9d
	movq	%rsi, %rdi
.LBB35_4:
	testb	%r9b, %r9b
	js	.LBB35_6
	movzbl	%r9b, %r10d
	jmp	.LBB35_12
.LBB35_6:
	movl	%r9d, %r10d
	andb	$31, %r10b
	movzbl	%r10b, %r10d
	movzbl	1(%r12,%rbx), %r14d
	andl	$63, %r14d
	cmpb	$-33, %r9b
	jbe	.LBB35_9
	movzbl	2(%r12,%rbx), %r11d
	shll	$6, %r14d
	andl	$63, %r11d
	orl	%r14d, %r11d
	cmpb	$-16, %r9b
	jb	.LBB35_11
	movzbl	3(%r12,%rbx), %r14d
	andl	$7, %r10d
	shll	$18, %r10d
	shll	$6, %r11d
	andl	$63, %r14d
	orl	%r11d, %r14d
	jmp	.LBB35_10
.LBB35_9:
	shll	$6, %r10d
.LBB35_10:
	orl	%r14d, %r10d
	jmp	.LBB35_12
.LBB35_11:
	shll	$12, %r10d
	orl	%r11d, %r10d
.LBB35_12:
	addl	$-48, %r10d
	cmpl	$10, %r10d
	jae	.LBB35_16
	cmpq	%rbx, %rdx
	je	.LBB35_178
	movb	1(%r12,%rbx), %r9b
	cmpb	$-65, %r9b
	jle	.LBB35_179
	decq	%rdi
	incq	%rbx
	decq	%r8
	jmp	.LBB35_4
.LBB35_16:
	testq	%rbx, %rbx
	je	.LBB35_183
	cmpb	$-65, (%r12,%rbx)
	jle	.LBB35_184
	cmpq	$1, %rbx
	jne	.LBB35_21
	movb	$1, %r10b
	cmpl	$43, %ecx
	je	.LBB35_177
	cmpl	$45, %ecx
	je	.LBB35_177
.LBB35_21:
	movq	%rsi, %rcx
	subq	%rbx, %rcx
	xorl	%r10d, %r10d
	cmpb	$43, %al
	sete	%r10b
	movq	%r10, %r11
	negq	%r11
	movq	%rbx, %rax
	subq	%r10, %rax
	addq	%r12, %r10
	cmpq	$17, %rax
	jae	.LBB35_26
	testq	%rax, %rax
	je	.LBB35_35
	addq	%rbx, %r11
	xorl	%edx, %edx
	xorl	%eax, %eax
.LBB35_24:
	movzbl	(%r10,%rdx), %r15d
	addl	$-48, %r15d
	cmpl	$9, %r15d
	ja	.LBB35_181
	leaq	(%rax,%rax,4), %rax
	movl	%r15d, %r15d
	leaq	(%r15,%rax,2), %rax
	incq	%rdx
	cmpq	%rdx, %r11
	jne	.LBB35_24
	jmp	.LBB35_31
.LBB35_26:
	addq	%rbx, %r11
	xorl	%r15d, %r15d
	xorl	%eax, %eax
.LBB35_27:
	movl	$10, %edx
	mulq	%rdx
	movzbl	(%r10,%r15), %edx
	jo	.LBB35_175
	addl	$-48, %edx
	cmpl	$10, %edx
	jae	.LBB35_181
	movl	%edx, %edx
	addq	%rdx, %rax
	jb	.LBB35_176
	incq	%r15
	cmpq	%r15, %r11
	jne	.LBB35_27
.LBB35_31:
	testq	%rax, %rax
	je	.LBB35_35
	movq	%rsi, %r14
	subq	%rax, %r14
	cmpq	%rcx, %rax
	jae	.LBB35_36
	leaq	(%r12,%rax), %r11
	cmpb	$-65, (%rbx,%r11)
	jle	.LBB35_185
	addq	%rbx, %r11
	subq	%rbx, %r14
	movq	%rax, %rcx
	leaq	(%r12,%rbx), %rsi
	jmp	.LBB35_38
.LBB35_35:
	xorl	%r15d, %r15d
	movq	%rcx, %r14
	leaq	(%r12,%rbx), %rsi
	movq	%rsi, -72(%rbp)
	jmp	.LBB35_54
.LBB35_36:
	cmpq	%rbx, %r14
	jne	.LBB35_185
	leaq	(%r12,%rbx), %rsi
	leaq	(%rsi,%rcx), %r11
	xorl	%r14d, %r14d
.LBB35_38:
	cmpb	$104, %r9b
	jne	.LBB35_53
	leaq	1(%r13), %rdx
	cmpq	-88(%rbp), %rdx
	jne	.LBB35_53
	cmpl	$0, -76(%rbp)
	je	.LBB35_53
	cmpq	$1, %rcx
	je	.LBB35_43
	cmpb	$-64, 1(%r12,%rbx)
	jl	.LBB35_190
.LBB35_43:
	leaq	(%r12,%rbx), %rdx
	incq	%rdx
	cmpq	%rdi, %rax
	cmovbq	%rax, %rdi
	addq	%r12, %rdi
	subq	%r8, %rdi
.LBB35_44:
	cmpq	%rdx, %rdi
	je	.LBB35_170
	movzbl	(%rdx), %eax
	testb	%al, %al
	js	.LBB35_47
	incq	%rdx
	jmp	.LBB35_52
.LBB35_47:
	movl	%eax, %r8d
	andl	$31, %r8d
	movzbl	1(%rdx), %r10d
	andl	$63, %r10d
	cmpb	$-33, %al
	jbe	.LBB35_50
	movzbl	2(%rdx), %r9d
	shll	$6, %r10d
	andl	$63, %r9d
	orl	%r10d, %r9d
	cmpb	$-16, %al
	jb	.LBB35_51
	movzbl	3(%rdx), %eax
	addq	$4, %rdx
	andl	$7, %r8d
	shll	$18, %r8d
	shll	$6, %r9d
	andl	$63, %eax
	orl	%r9d, %eax
	orl	%r8d, %eax
	jmp	.LBB35_52
.LBB35_50:
	addq	$2, %rdx
	shll	$6, %r8d
	orl	%r10d, %r8d
	movl	%r8d, %eax
	jmp	.LBB35_52
.LBB35_51:
	addq	$3, %rdx
	shll	$12, %r8d
	orl	%r8d, %r9d
	movl	%r9d, %eax
.LBB35_52:
	leal	-65(%rax), %r8d
	andl	$-34, %r8d
	addl	$10, %r8d
	leal	-48(%rax), %r9d
	cmpl	$58, %eax
	cmovael	%r8d, %r9d
	cmpl	$15, %r9d
	jbe	.LBB35_44
.LBB35_53:
	movq	%r11, -72(%rbp)
	movq	%rcx, %r15
.LBB35_54:
	testq	%r13, %r13
	je	.LBB35_56
	movl	$2, %edx
	movq	-48(%rbp), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48(%rip), %rsi
	movq	-56(%rbp), %rax
	callq	*24(%rax)
	leaq	(%r12,%rbx), %rsi
	testb	%al, %al
	jne	.LBB35_174
.LBB35_56:
	cmpq	$1, %r15
	movq	%r14, -104(%rbp)
	jbe	.LBB35_60
	cmpw	$9311, (%rsi)
	jne	.LBB35_60
	cmpb	$-64, 1(%r12,%rbx)
	jl	.LBB35_189
	leaq	(%r12,%rbx), %rsi
	incq	%rsi
	decq	%r15
.LBB35_60:
	movq	%r15, %r13
	movq	%rsi, %r12
	testq	%r15, %r15
	je	.LBB35_65
	movzbl	(%r12), %eax
	cmpl	$36, %eax
	je	.LBB35_81
	cmpl	$46, %eax
	jne	.LBB35_65
	cmpq	$1, %r13
	jne	.LBB35_100
	movl	$1, %edx
	movq	-48(%rbp), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.139(%rip), %rsi
	movq	-56(%rbp), %rax
	callq	*24(%rax)
	testb	%al, %al
	je	.LBB35_114
	jmp	.LBB35_174
.LBB35_65:
	leaq	(%r12,%r13), %rax
	xorl	%edi, %edi
	movq	%r12, %rcx
.LBB35_66:
	cmpq	%rax, %rcx
	je	.LBB35_168
	movq	%rdi, %rbx
	movzbl	(%rcx), %edx
	testb	%dl, %dl
	js	.LBB35_69
	leaq	1(%rcx), %rsi
	jmp	.LBB35_74
.LBB35_69:
	movl	%edx, %edi
	andl	$31, %edi
	movzbl	1(%rcx), %r9d
	andl	$63, %r9d
	cmpb	$-33, %dl
	jbe	.LBB35_72
	movzbl	2(%rcx), %r8d
	shll	$6, %r9d
	andl	$63, %r8d
	orl	%r9d, %r8d
	cmpb	$-16, %dl
	jb	.LBB35_73
	leaq	4(%rcx), %rsi
	movzbl	3(%rcx), %edx
	andl	$7, %edi
	shll	$18, %edi
	shll	$6, %r8d
	andl	$63, %edx
	orl	%r8d, %edx
	orl	%edi, %edx
	jmp	.LBB35_74
.LBB35_72:
	leaq	2(%rcx), %rsi
	shll	$6, %edi
	orl	%r9d, %edi
	movl	%edi, %edx
	jmp	.LBB35_74
.LBB35_73:
	leaq	3(%rcx), %rsi
	shll	$12, %edi
	orl	%edi, %r8d
	movl	%r8d, %edx
.LBB35_74:
	cmpl	$36, %edx
	je	.LBB35_76
	movq	%rbx, %rdi
	subq	%rcx, %rdi
	addq	%rsi, %rdi
	movq	%rsi, %rcx
	cmpl	$46, %edx
	jne	.LBB35_66
.LBB35_76:
	testq	%rbx, %rbx
	je	.LBB35_95
	cmpq	%r13, %rbx
	jae	.LBB35_96
	cmpb	$-65, (%r12,%rbx)
	jle	.LBB35_172
	movq	-48(%rbp), %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	movq	-56(%rbp), %rax
	callq	*24(%rax)
	testb	%al, %al
	jne	.LBB35_174
	cmpb	$-64, (%r12,%rbx)
	jge	.LBB35_99
	jmp	.LBB35_186
.LBB35_81:
	cmpq	$1, %r13
	je	.LBB35_83
	cmpb	$-65, 1(%r12)
	jle	.LBB35_188
.LBB35_83:
	leaq	-1(%r13), %r14
	leaq	1(%r12), %rbx
	xorl	%r15d, %r15d
.LBB35_84:
	movq	%r14, %rdx
	subq	%r15, %rdx
	leaq	(%rbx,%r15), %rsi
	cmpq	$15, %rdx
	ja	.LBB35_89
	cmpq	%r15, %r14
	je	.LBB35_168
	xorl	%ecx, %ecx
.LBB35_87:
	cmpb	$36, (%rsi,%rcx)
	je	.LBB35_91
	incq	%rcx
	cmpq	%rcx, %rdx
	jne	.LBB35_87
	jmp	.LBB35_168
.LBB35_89:
	movl	$36, %edi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice6memchr14memchr_aligned@GOTPCREL(%rip)
	testb	$1, %al
	je	.LBB35_168
	movq	%rdx, %rcx
.LBB35_91:
	leaq	(%rcx,%r15), %rax
	incq	%rax
	addq	%r15, %rcx
	cmpq	%r14, %rcx
	jae	.LBB35_94
	movb	(%rbx,%rcx), %dl
	cmpq	%r14, %rax
	ja	.LBB35_116
	movq	%rax, %r15
	cmpb	$36, %dl
	jne	.LBB35_84
	jmp	.LBB35_116
.LBB35_94:
	movq	%rax, %r15
	cmpq	%r14, %rax
	jbe	.LBB35_84
	jmp	.LBB35_168
.LBB35_95:
	movq	-48(%rbp), %rdi
	movq	%r12, %rsi
	xorl	%edx, %edx
	movq	-56(%rbp), %rax
	callq	*24(%rax)
	movl	$0, %ebx
	jmp	.LBB35_98
.LBB35_96:
	jne	.LBB35_172
	movq	-48(%rbp), %rdi
	movq	%r12, %rsi
	movq	%r13, %rdx
	movq	-56(%rbp), %rax
	callq	*24(%rax)
	movq	%r13, %rbx
.LBB35_98:
	testb	%al, %al
	jne	.LBB35_174
.LBB35_99:
	subq	%rbx, %r13
	addq	%rbx, %r12
	jmp	.LBB35_115
.LBB35_100:
	movzbl	1(%r12), %eax
	cmpb	$-65, %al
	jle	.LBB35_187
	testb	%al, %al
	jns	.LBB35_107
	movl	%eax, %ecx
	andl	$31, %ecx
	movzbl	2(%r12), %esi
	andl	$63, %esi
	cmpb	$-33, %al
	jbe	.LBB35_105
	movzbl	3(%r12), %edx
	shll	$6, %esi
	andl	$63, %edx
	orl	%esi, %edx
	cmpb	$-16, %al
	jb	.LBB35_106
	movzbl	4(%r12), %eax
	andl	$7, %ecx
	shll	$18, %ecx
	shll	$6, %edx
	andl	$63, %eax
	orl	%edx, %eax
	orl	%ecx, %eax
	jmp	.LBB35_107
.LBB35_105:
	shll	$6, %ecx
	orl	%esi, %ecx
	movl	%ecx, %eax
	jmp	.LBB35_107
.LBB35_106:
	shll	$12, %ecx
	orl	%ecx, %edx
	movl	%edx, %eax
.LBB35_107:
	movq	-56(%rbp), %rcx
	movq	24(%rcx), %rcx
	cmpl	$46, %eax
	jne	.LBB35_112
	movl	$2, %edx
	movq	-48(%rbp), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48(%rip), %rsi
	callq	*%rcx
	testb	%al, %al
	jne	.LBB35_174
	cmpq	$3, %r13
	jb	.LBB35_111
	cmpb	$-64, 2(%r12)
	jl	.LBB35_196
.LBB35_111:
	addq	$-2, %r13
	addq	$2, %r12
	jmp	.LBB35_115
.LBB35_112:
	movl	$1, %edx
	movq	-48(%rbp), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.139(%rip), %rsi
	callq	*%rcx
	testb	%al, %al
	jne	.LBB35_174
	cmpb	$-64, 1(%r12)
	jl	.LBB35_191
.LBB35_114:
	decq	%r13
	incq	%r12
.LBB35_115:
	movq	%r12, %rsi
	movq	%r13, %r15
	jmp	.LBB35_60
.LBB35_116:
	cmpb	$36, %dl
	jne	.LBB35_168
	movzbl	(%rbx), %eax
	cmpb	$-64, %al
	jl	.LBB35_182
	leaq	2(%rcx), %r14
	movq	%r13, %r15
	subq	%r14, %r15
	jbe	.LBB35_120
	cmpb	$-64, (%r12,%r14)
	jl	.LBB35_197
.LBB35_120:
	addq	%r12, %r14
	cmpq	$1, %rcx
	je	.LBB35_132
	cmpq	$2, %rcx
	je	.LBB35_125
	testq	%rcx, %rcx
	je	.LBB35_168
.LBB35_123:
	cmpl	$117, %eax
	jne	.LBB35_168
	cmpb	$-64, 2(%r12)
	jge	.LBB35_134
	jmp	.LBB35_198
.LBB35_125:
	cmpw	$20563, (%rbx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.136(%rip), %rsi
	je	.LBB35_147
	cmpw	$20546, (%rbx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.61(%rip), %rsi
	je	.LBB35_147
	cmpw	$18002, (%rbx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.58(%rip), %rsi
	je	.LBB35_147
	cmpw	$21580, (%rbx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54(%rip), %rsi
	je	.LBB35_147
	cmpw	$21575, (%rbx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.56(%rip), %rsi
	je	.LBB35_147
	cmpw	$20556, (%rbx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64(%rip), %rsi
	je	.LBB35_147
	cmpw	$20562, (%rbx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25(%rip), %rsi
	jne	.LBB35_123
	jmp	.LBB35_147
.LBB35_132:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.65(%rip), %rsi
	cmpl	$67, %eax
	je	.LBB35_147
	cmpl	$117, %eax
	jne	.LBB35_168
.LBB35_134:
	addq	%rcx, %rbx
	decq	%rcx
	leaq	2(%r12), %rdx
	movq	%rdx, %rdi
.LBB35_135:
	movq	%rdi, %rax
	cmpq	%rbx, %rdi
	je	.LBB35_144
	movzbl	(%rax), %esi
	testb	%sil, %sil
	js	.LBB35_138
	leaq	1(%rax), %rdi
	jmp	.LBB35_143
.LBB35_138:
	movl	%esi, %r8d
	andl	$31, %r8d
	movzbl	1(%rax), %r10d
	andl	$63, %r10d
	cmpb	$-33, %sil
	jbe	.LBB35_141
	movzbl	2(%rax), %r9d
	shll	$6, %r10d
	andl	$63, %r9d
	orl	%r10d, %r9d
	cmpb	$-16, %sil
	jb	.LBB35_142
	leaq	4(%rax), %rdi
	movzbl	3(%rax), %esi
	andl	$7, %r8d
	shll	$18, %r8d
	shll	$6, %r9d
	andl	$63, %esi
	orl	%r9d, %esi
	orl	%r8d, %esi
	jmp	.LBB35_143
.LBB35_141:
	leaq	2(%rax), %rdi
	shll	$6, %r8d
	orl	%r10d, %r8d
	movl	%r8d, %esi
	jmp	.LBB35_143
.LBB35_142:
	leaq	3(%rax), %rdi
	shll	$12, %r8d
	orl	%r8d, %r9d
	movl	%r9d, %esi
.LBB35_143:
	leal	-58(%rsi), %r8d
	cmpl	$-10, %r8d
	setb	%r8b
	addl	$-103, %esi
	cmpl	$-6, %esi
	setb	%sil
	andb	%r8b, %sil
	cmpb	$1, %sil
	jne	.LBB35_135
.LBB35_144:
	cmpq	$1, %rcx
	je	.LBB35_148
	testq	%rcx, %rcx
	je	.LBB35_168
	movb	(%rdx), %sil
	jmp	.LBB35_150
.LBB35_147:
	movl	$1, %edx
	movq	-48(%rbp), %rdi
	movq	-56(%rbp), %rax
	callq	*24(%rax)
	jmp	.LBB35_166
.LBB35_148:
	movzbl	(%rdx), %esi
	cmpl	$43, %esi
	je	.LBB35_168
	cmpl	$45, %esi
	je	.LBB35_168
.LBB35_150:
	xorl	%edi, %edi
	cmpb	$43, %sil
	sete	%dil
	subq	%rdi, %rcx
	addq	%rdi, %rdx
	cmpq	$9, %rcx
	jae	.LBB35_155
	xorl	%esi, %esi
	testq	%rcx, %rcx
	je	.LBB35_159
	xorl	%edi, %edi
.LBB35_153:
	movzbl	(%rdx,%rdi), %r9d
	leal	-65(%r9), %r10d
	andl	$-33, %r10d
	addl	$10, %r10d
	leal	-48(%r9), %r8d
	cmpl	$58, %r9d
	cmovael	%r10d, %r8d
	cmpl	$15, %r8d
	ja	.LBB35_168
	shll	$4, %esi
	orl	%r8d, %esi
	incq	%rdi
	cmpq	%rdi, %rcx
	je	.LBB35_159
	jmp	.LBB35_153
.LBB35_155:
	xorl	%r8d, %r8d
	xorl	%edi, %edi
.LBB35_156:
	movzbl	(%rdx,%rdi), %r10d
	leal	-65(%r10), %r9d
	leal	-48(%r10), %esi
	cmpl	$268435455, %r8d
	ja	.LBB35_167
	andl	$-33, %r9d
	addl	$10, %r9d
	cmpb	$58, %r10b
	cmovael	%r9d, %esi
	cmpl	$16, %esi
	jae	.LBB35_168
	shll	$4, %r8d
	orl	%r8d, %esi
	incq	%rdi
	movl	%esi, %r8d
	cmpq	%rdi, %rcx
	jne	.LBB35_156
.LBB35_159:
	shlq	$32, %rsi
.LBB35_160:
	testb	$1, %sil
	jne	.LBB35_168
	movq	%rsi, %rcx
	shrq	$32, %rcx
	movl	%ecx, %edx
	xorl	$55296, %edx
	addl	$-1114112, %edx
	cmpl	$-1112064, %edx
	jb	.LBB35_168
	cmpq	%rbx, %rax
	jne	.LBB35_168
	movl	%ecx, -80(%rbp)
	movq	%rsi, %rax
	shrq	$37, %rax
	je	.LBB35_168
	movabsq	$-545460846592, %rax
	addq	%rax, %rsi
	shrq	$32, %rsi
	cmpl	$33, %esi
	jb	.LBB35_168
	leaq	-80(%rbp), %rdi
	movq	-96(%rbp), %rsi
	callq	*_RNvXsk_NtCs2k2z8Zem4rB_4core3fmtcNtB5_7Display3fmt@GOTPCREL(%rip)
.LBB35_166:
	testb	%al, %al
	movq	%r14, %rsi
	je	.LBB35_60
	jmp	.LBB35_174
.LBB35_167:
	andl	$-34, %r9d
	addl	$10, %r9d
	cmpb	$58, %r10b
	cmovael	%r9d, %esi
	xorl	%ecx, %ecx
	cmpl	$16, %esi
	setb	%cl
	shll	$8, %ecx
	addq	$257, %rcx
	movq	%rcx, %rsi
	jmp	.LBB35_160
.LBB35_168:
	movq	-48(%rbp), %rdi
	movq	%r12, %rsi
	movq	%r13, %rdx
	movq	-56(%rbp), %rax
	callq	*24(%rax)
	testb	%al, %al
	jne	.LBB35_174
	movq	-112(%rbp), %rax
	movq	%rax, %r13
	movq	-72(%rbp), %r12
	cmpq	-88(%rbp), %rax
	movq	-104(%rbp), %rsi
	jne	.LBB35_2
.LBB35_170:
	xorl	%eax, %eax
.LBB35_171:
	addq	$72, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB35_172:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.123(%rip), %r8
	movq	%r12, %rdi
	movq	%r13, %rsi
.LBB35_173:
	xorl	%edx, %edx
	movq	%rbx, %rcx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB35_174:
	movb	$1, %al
	jmp	.LBB35_171
.LBB35_175:
	addb	$-48, %dl
	xorl	%r10d, %r10d
	cmpb	$10, %dl
	adcb	$1, %r10b
	jmp	.LBB35_177
.LBB35_176:
	movb	$2, %r10b
.LBB35_177:
	leaq	-57(%rbp), %rdx
	movb	%r10b, (%rdx)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.36(%rip), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.37(%rip), %rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.118(%rip), %r8
	movl	$43, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip)
.LBB35_178:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.116(%rip), %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip)
.LBB35_179:
	subq	%rbx, %rsi
	addq	%rbx, %r12
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.141(%rip), %r8
	movl	$1, %edx
	movq	%r12, %rdi
.LBB35_180:
	movq	%rsi, %rcx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB35_181:
	movb	$1, %r10b
	jmp	.LBB35_177
.LBB35_182:
	incq	%rcx
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.126(%rip), %r8
	movl	$1, %edx
	movq	%r12, %rdi
	movq	%r13, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB35_183:
	xorl	%r10d, %r10d
	jmp	.LBB35_177
.LBB35_184:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.117(%rip), %r8
	movq	%r12, %rdi
	jmp	.LBB35_173
.LBB35_185:
	subq	%rbx, %rsi
	addq	%rbx, %r12
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.119(%rip), %r8
	movq	%r12, %rdi
	movq	%rax, %rdx
	jmp	.LBB35_180
.LBB35_186:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.124(%rip), %r8
	movq	%r12, %rdi
	movq	%r13, %rsi
	movq	%rbx, %rdx
	jmp	.LBB35_195
.LBB35_187:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.137(%rip), %r8
	jmp	.LBB35_192
.LBB35_188:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.125(%rip), %r8
	jmp	.LBB35_192
.LBB35_189:
	addq	%rbx, %r12
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.122(%rip), %r8
	movl	$1, %edx
	movq	%r12, %rdi
	movq	%r15, %rsi
	movq	%r15, %rcx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB35_190:
	addq	%rbx, %r12
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.109(%rip), %r8
	movl	$1, %edx
	movq	%r12, %rdi
.LBB35_199:
	movq	%rcx, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB35_191:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.140(%rip), %r8
.LBB35_192:
	movl	$1, %edx
.LBB35_193:
	movq	%r12, %rdi
.LBB35_194:
	movq	%r13, %rsi
.LBB35_195:
	movq	%r13, %rcx
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB35_196:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.138(%rip), %r8
	movl	$2, %edx
	jmp	.LBB35_193
.LBB35_197:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.127(%rip), %r8
	movq	%r12, %rdi
	movq	%r14, %rdx
	jmp	.LBB35_194
.LBB35_198:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.135(%rip), %r8
	movl	$1, %edx
	movq	%rbx, %rdi
	jmp	.LBB35_199
.Lfunc_end35:
	.size	_RNvXNtCs8dXjxA0JZyF_14rustc_demangle6legacyNtB2_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt, .Lfunc_end35-_RNvXNtCs8dXjxA0JZyF_14rustc_demangle6legacyNtB2_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.cfi_endproc

	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
.LCPI36_0:
	.quad	0
	.quad	1
	.quad	2
	.quad	3
	.quad	4
	.quad	5
	.quad	6
	.quad	7
.LCPI36_1:
	.byte	191
	.section	.rodata.cst8,"aM",@progbits,8
	.p2align	3, 0x0
.LCPI36_2:
	.quad	8
.LCPI36_3:
	.byte	0
	.byte	1
	.byte	2
	.byte	3
	.byte	4
	.byte	5
	.byte	6
	.byte	7
	.section	.text._RNvXNtNtCs2k2z8Zem4rB_4core3str4iterNtB2_5CharsNtNtNtNtB6_4iter6traits8iterator8Iterator5count,"ax",@progbits
	.type	_RNvXNtNtCs2k2z8Zem4rB_4core3str4iterNtB2_5CharsNtNtNtNtB6_4iter6traits8iterator8Iterator5count,@function
_RNvXNtNtCs2k2z8Zem4rB_4core3str4iterNtB2_5CharsNtNtNtNtB6_4iter6traits8iterator8Iterator5count:
.Lfunc_begin36:
	.cfi_startproc
	movq	%rsi, %rax
	subq	%rdi, %rsi
	cmpq	$32, %rsi
	jae	.LBB36_7
	cmpq	%rdi, %rax
	je	.LBB36_2
	leal	7(%rsi), %eax
	andl	$56, %eax
	decq	%rsi
	vpbroadcastq	%rsi, %zmm0
	vpmovsxbq	.LCPI36_3(%rip), %zmm1
	vpxor	%xmm5, %xmm5, %xmm5
	xorl	%ecx, %ecx
	vpbroadcastb	.LCPI36_1(%rip), %xmm2
	vpbroadcastq	.LCPI36_2(%rip), %zmm3
.LBB36_4:
	vmovdqa64	%zmm5, %zmm4
	vpcmpleuq	%zmm0, %zmm1, %k1
	vmovdqu8	(%rdi,%rcx), %xmm5 {%k1} {z}
	vpcmpgtb	%xmm2, %xmm5, %k0
	vpmovm2q	%k0, %zmm5
	vpsubq	%zmm5, %zmm4, %zmm5
	addq	$8, %rcx
	vpaddq	%zmm3, %zmm1, %zmm1
	cmpq	%rcx, %rax
	jne	.LBB36_4
	vmovdqa64	%zmm5, %zmm4 {%k1}
	vextracti64x4	$1, %zmm4, %ymm0
	vpaddq	%zmm0, %zmm4, %zmm0
	vextracti128	$1, %ymm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vpshufd	$238, %xmm0, %xmm1
	vpaddq	%xmm1, %xmm0, %xmm0
	vmovq	%xmm0, %rax
	vzeroupper
	retq
.LBB36_7:
	jmpq	*_RNvNtNtCs2k2z8Zem4rB_4core3str5count14do_count_chars@GOTPCREL(%rip)
.LBB36_2:
	xorl	%eax, %eax
	retq
.Lfunc_end36:
	.size	_RNvXNtNtCs2k2z8Zem4rB_4core3str4iterNtB2_5CharsNtNtNtNtB6_4iter6traits8iterator8Iterator5count, .Lfunc_end36-_RNvXNtNtCs2k2z8Zem4rB_4core3str4iterNtB2_5CharsNtNtNtNtB6_4iter6traits8iterator8Iterator5count
	.cfi_endproc

	.section	.text._RNvXNtNtNtCs2k2z8Zem4rB_4core4iter7sources7from_fnINtB2_6FromFnNCNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB17_10HexNibbles19try_parse_str_charss0_0ENtNtNtB6_6traits8iterator8Iterator4nextB19_,"ax",@progbits
	.type	_RNvXNtNtNtCs2k2z8Zem4rB_4core4iter7sources7from_fnINtB2_6FromFnNCNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB17_10HexNibbles19try_parse_str_charss0_0ENtNtNtB6_6traits8iterator8Iterator4nextB19_,@function
_RNvXNtNtNtCs2k2z8Zem4rB_4core4iter7sources7from_fnINtB2_6FromFnNCNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB17_10HexNibbles19try_parse_str_charss0_0ENtNtNtB6_6traits8iterator8Iterator4nextB19_:
.Lfunc_begin37:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	subq	$96, %rsp
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	8(%rdi), %rcx
	movq	32(%rdi), %r8
	movl	$-2, %eax
	subq	%r8, %rcx
	jb	.LBB37_12
	movq	(%rdi), %rdx
	leaq	(%rdx,%r8), %rsi
	movq	%rsi, (%rdi)
	movq	%rcx, 8(%rdi)
	cmpq	$2, %r8
	jne	.LBB37_30
	movzbl	(%rdx), %r8d
	leal	-65(%r8), %r9d
	andl	$-33, %r9d
	addl	$10, %r9d
	leal	-48(%r8), %eax
	cmpl	$58, %r8d
	cmovael	%r9d, %eax
	cmpl	$15, %eax
	ja	.LBB37_31
	movzbl	1(%rdx), %edx
	leal	-65(%rdx), %r9d
	andl	$-33, %r9d
	addl	$10, %r9d
	leal	-48(%rdx), %r8d
	cmpl	$58, %edx
	cmovael	%r9d, %r8d
	cmpl	$16, %r8d
	jae	.LBB37_31
	shlb	$4, %al
	orb	%al, %r8b
	js	.LBB37_5
	leaq	-20(%rbp), %rax
	movb	%r8b, (%rax)
	movw	$0, 1(%rax)
	movb	$0, 3(%rax)
	movq	%rax, -40(%rbp)
	movq	$1, -32(%rbp)
	movl	$1, %edx
.LBB37_14:
	leaq	-112(%rbp), %rbx
	leaq	-20(%rbp), %rsi
	movq	%rbx, %rdi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core3str8converts9from_utf8@GOTPCREL(%rip)
	cmpb	$0, (%rbx)
	je	.LBB37_15
.LBB37_11:
	movl	$-1, %eax
	jmp	.LBB37_12
.LBB37_5:
	movl	$-1, %eax
	cmpb	$-64, %r8b
	jb	.LBB37_12
	movl	$2, %edx
	cmpb	$-32, %r8b
	jb	.LBB37_9
	movl	$3, %edx
	cmpb	$-16, %r8b
	jb	.LBB37_9
	movl	$4, %edx
	cmpb	$-8, %r8b
	jae	.LBB37_12
.LBB37_9:
	leaq	-19(%rbp), %rax
	movb	%r8b, -1(%rax)
	movw	$0, (%rax)
	movb	$0, 2(%rax)
	leaq	-20(%rbp), %r8
	movq	%r8, -40(%rbp)
	movq	%rdx, -32(%rbp)
	leaq	-2(,%rdx,2), %r8
	xorl	%r9d, %r9d
.LBB37_10:
	cmpq	$2, %rcx
	jb	.LBB37_11
	leaq	(%rsi,%r9), %r10
	addq	$2, %r10
	addq	$-2, %rcx
	movq	%r10, (%rdi)
	movq	%rcx, 8(%rdi)
	movzbl	-2(%r10), %r11d
	leal	-65(%r11), %ebx
	andl	$-33, %ebx
	addl	$10, %ebx
	leal	-48(%r11), %r10d
	cmpl	$58, %r11d
	cmovael	%ebx, %r10d
	cmpl	$15, %r10d
	ja	.LBB37_31
	movzbl	1(%rsi,%r9), %ebx
	leal	-65(%rbx), %r14d
	andl	$-33, %r14d
	addl	$10, %r14d
	leal	-48(%rbx), %r11d
	cmpl	$58, %ebx
	cmovael	%r14d, %r11d
	cmpl	$16, %r11d
	jae	.LBB37_31
	shlb	$4, %r10b
	orb	%r10b, %r11b
	movb	%r11b, (%rax)
	addq	$2, %r9
	incq	%rax
	cmpq	%r9, %r8
	jne	.LBB37_10
	jmp	.LBB37_14
.LBB37_15:
	movq	-104(%rbp), %rdi
	movq	-96(%rbp), %rax
	movq	%rdi, -56(%rbp)
	movq	%rax, -48(%rbp)
	leaq	(%rdi,%rax), %rsi
	testq	%rax, %rax
	je	.LBB37_26
	movzbl	(%rdi), %eax
	testb	%al, %al
	js	.LBB37_18
	leaq	1(%rdi), %rdx
	jmp	.LBB37_23
.LBB37_18:
	movl	%eax, %ecx
	andl	$31, %ecx
	movzbl	1(%rdi), %r9d
	andl	$63, %r9d
	cmpb	$-33, %al
	jbe	.LBB37_19
	movzbl	2(%rdi), %r8d
	shll	$6, %r9d
	andl	$63, %r8d
	orl	%r9d, %r8d
	cmpb	$-16, %al
	jb	.LBB37_21
	leaq	4(%rdi), %rdx
	movzbl	3(%rdi), %eax
	andl	$7, %ecx
	shll	$18, %ecx
	shll	$6, %r8d
	andl	$63, %eax
	orl	%r8d, %eax
	orl	%ecx, %eax
	jmp	.LBB37_23
.LBB37_19:
	leaq	2(%rdi), %rdx
	shll	$6, %ecx
	orl	%r9d, %ecx
	movl	%ecx, %eax
	jmp	.LBB37_23
.LBB37_21:
	leaq	3(%rdi), %rdx
	shll	$12, %ecx
	orl	%ecx, %r8d
	movl	%r8d, %eax
.LBB37_23:
	cmpq	%rsi, %rdx
	jne	.LBB37_24
.LBB37_12:
	addq	$96, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB37_31:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.18(%rip), %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip)
.LBB37_30:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.16(%rip), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.17(%rip), %rdx
	movl	$40, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip)
.LBB37_24:
	movb	(%rdx), %al
	testb	%al, %al
	jns	.LBB37_26
	cmpb	$-32, %al
.LBB37_26:
	callq	_RNvXNtNtCs2k2z8Zem4rB_4core3str4iterNtB2_5CharsNtNtNtNtB6_4iter6traits8iterator8Iterator5count
	leaq	-64(%rbp), %rcx
	movq	%rax, (%rcx)
	leaq	-40(%rbp), %rax
	leaq	-112(%rbp), %rsi
	movq	%rax, (%rsi)
	movq	_RNvXs1h_NtCs2k2z8Zem4rB_4core3fmtQShNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle@GOTPCREL(%rip), %rax
	movq	%rax, 8(%rsi)
	leaq	-56(%rbp), %rax
	movq	%rax, 16(%rsi)
	movq	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle@GOTPCREL(%rip), %rax
	movq	%rax, 24(%rsi)
	movq	%rcx, 32(%rsi)
	movq	_RNvXsi_NtNtNtCs2k2z8Zem4rB_4core3fmt3num3impjNtB9_7Display3fmt@GOTPCREL(%rip), %rax
	movq	%rax, 40(%rsi)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.14(%rip), %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.15(%rip), %rdx
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt@GOTPCREL(%rip)
.Lfunc_end37:
	.size	_RNvXNtNtNtCs2k2z8Zem4rB_4core4iter7sources7from_fnINtB2_6FromFnNCNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB17_10HexNibbles19try_parse_str_charss0_0ENtNtNtB6_6traits8iterator8Iterator4nextB19_, .Lfunc_end37-_RNvXNtNtNtCs2k2z8Zem4rB_4core4iter7sources7from_fnINtB2_6FromFnNCNvMs1_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB17_10HexNibbles19try_parse_str_charss0_0ENtNtNtB6_6traits8iterator8Iterator4nextB19_
	.cfi_endproc

	.section	.text._RNvXs0_Cs8dXjxA0JZyF_14rustc_demangleINtB5_21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtB15_5Write9write_strB5_,"ax",@progbits
	.globl	_RNvXs0_Cs8dXjxA0JZyF_14rustc_demangleINtB5_21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtB15_5Write9write_strB5_
	.type	_RNvXs0_Cs8dXjxA0JZyF_14rustc_demangleINtB5_21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtB15_5Write9write_strB5_,@function
_RNvXs0_Cs8dXjxA0JZyF_14rustc_demangleINtB5_21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtB15_5Write9write_strB5_:
.Lfunc_begin38:
	.cfi_startproc
	movq	8(%rdi), %rax
	subq	%rdx, %rax
	setb	%cl
	orb	(%rdi), %cl
	movzbl	%cl, %ecx
	movq	%rcx, (%rdi)
	movq	%rax, 8(%rdi)
	testb	%cl, %cl
	je	.LBB38_2
	movb	$1, %al
	retq
.LBB38_2:
	movq	16(%rdi), %rax
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	jmpq	*24(%rax)
.Lfunc_end38:
	.size	_RNvXs0_Cs8dXjxA0JZyF_14rustc_demangleINtB5_21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtB15_5Write9write_strB5_, .Lfunc_end38-_RNvXs0_Cs8dXjxA0JZyF_14rustc_demangleINtB5_21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtB15_5Write9write_strB5_
	.cfi_endproc

	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
.LCPI39_0:
	.quad	8
	.quad	9
	.quad	10
	.quad	11
	.quad	12
	.quad	13
	.quad	14
	.quad	15
.LCPI39_1:
	.quad	0
	.quad	1
	.quad	2
	.quad	3
	.quad	4
	.quad	5
	.quad	6
	.quad	7
	.section	.rodata.cst8,"aM",@progbits,8
	.p2align	3, 0x0
.LCPI39_2:
	.quad	16
.LCPI39_3:
	.byte	8
	.byte	9
	.byte	10
	.byte	11
	.byte	12
	.byte	13
	.byte	14
	.byte	15
.LCPI39_4:
	.byte	0
	.byte	1
	.byte	2
	.byte	3
	.byte	4
	.byte	5
	.byte	6
	.byte	7
	.section	.text._RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,"ax",@progbits
	.globl	_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.type	_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,@function
_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt:
.Lfunc_begin39:
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
	subq	$600, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r8
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, -192(%rbp)
	vmovdqu64	%zmm0, -256(%rbp)
	vmovdqu64	%zmm0, -320(%rbp)
	vmovdqu64	%zmm0, -384(%rbp)
	vmovdqu64	%zmm0, -448(%rbp)
	vmovdqu64	%zmm0, -512(%rbp)
	vmovdqu64	%zmm0, -576(%rbp)
	vmovdqu64	%zmm0, -640(%rbp)
	movq	24(%rdi), %rcx
	movq	%rcx, -96(%rbp)
	testq	%rcx, %rcx
	je	.LBB39_51
	movq	%r8, -88(%rbp)
	movq	16(%rdi), %rax
	movq	%rax, -80(%rbp)
	movb	(%rax), %r9b
	movq	(%rdi), %rcx
	movq	8(%rdi), %rax
	testq	%rax, %rax
	movq	%rcx, -120(%rbp)
	movq	%rax, -128(%rbp)
	je	.LBB39_2
	addq	%rcx, %rax
	xorl	%ebx, %ebx
.LBB39_8:
	movzbl	(%rcx), %edx
	testb	%dl, %dl
	js	.LBB39_10
	incq	%rcx
	jmp	.LBB39_15
.LBB39_10:
	movl	%edx, %esi
	andl	$31, %esi
	movzbl	1(%rcx), %r8d
	andl	$63, %r8d
	cmpb	$-33, %dl
	jbe	.LBB39_11
	movzbl	2(%rcx), %edi
	shll	$6, %r8d
	andl	$63, %edi
	orl	%r8d, %edi
	cmpb	$-16, %dl
	jb	.LBB39_13
	movzbl	3(%rcx), %edx
	addq	$4, %rcx
	andl	$7, %esi
	shll	$18, %esi
	shll	$6, %edi
	andl	$63, %edx
	orl	%edi, %edx
	orl	%esi, %edx
	jmp	.LBB39_15
.LBB39_11:
	addq	$2, %rcx
	shll	$6, %esi
	orl	%r8d, %esi
	movl	%esi, %edx
	jmp	.LBB39_15
.LBB39_13:
	addq	$3, %rcx
	shll	$12, %esi
	orl	%esi, %edi
	movl	%edi, %edx
.LBB39_15:
	cmpq	$128, %rbx
	je	.LBB39_45
	movl	%edx, -640(%rbp,%rbx,4)
	incq	%rbx
	cmpq	%rax, %rcx
	je	.LBB39_3
	jmp	.LBB39_8
.LBB39_51:
	movq	(%rdi), %rsi
	movq	8(%rdi), %rdx
	movq	(%r8), %rdi
	movq	8(%r8), %rax
	addq	$600, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	jmpq	*24(%rax)
.LBB39_2:
	.cfi_def_cfa %rbp, 16
	xorl	%ebx, %ebx
.LBB39_3:
	movq	-96(%rbp), %rax
	movq	-80(%rbp), %r14
	addq	%r14, %rax
	movq	%rax, -104(%rbp)
	cmpq	$129, %rbx
	movl	$128, %eax
	movq	%rax, -48(%rbp)
	cmovaeq	%rbx, %rax
	movq	%rax, -112(%rbp)
	leaq	4(,%rbx,4), %rax
	movq	%rax, -64(%rbp)
	leaq	-700(,%rbx,4), %r15
	addq	%rbp, %r15
	movl	$700, %eax
	movq	%rax, -72(%rbp)
	movl	$72, %r13d
	movl	$26, %r12d
	vpmovsxbq	.LCPI39_3(%rip), %zmm0
	vpbroadcastq	.LCPI39_2(%rip), %zmm1
	vpmovsxbq	.LCPI39_4(%rip), %zmm2
	xorl	%edi, %edi
.LBB39_4:
	incq	%r14
	movl	$1, %esi
	xorl	%eax, %eax
	movl	$36, %r10d
	xorl	%ecx, %ecx
.LBB39_5:
	movq	%r10, %rdx
	subq	%r13, %rdx
	movl	$0, %r11d
	cmovaeq	%rdx, %r11
	cmpq	$1, %r11
	adcq	$0, %r11
	cmpq	$26, %r11
	cmovaeq	%r12, %r11
	testb	$1, %al
	je	.LBB39_6
	cmpq	-104(%rbp), %r14
	je	.LBB39_45
	movb	(%r14), %al
	incq	%r14
	jmp	.LBB39_19
.LBB39_6:
	movl	%r9d, %eax
.LBB39_19:
	leal	-97(%rax), %edx
	cmpb	$26, %dl
	jb	.LBB39_22
	leal	-48(%rax), %edx
	cmpb	$9, %dl
	ja	.LBB39_45
	addb	$-22, %al
	movl	%eax, %edx
.LBB39_22:
	movzbl	%dl, %r8d
	movq	%r8, %rax
	mulq	%rsi
	jo	.LBB39_45
	addq	%rax, %rcx
	jb	.LBB39_45
	cmpq	%r8, %r11
	ja	.LBB39_44
	movl	$36, %edx
	subq	%r11, %rdx
	movq	%rsi, %rax
	mulq	%rdx
	jo	.LBB39_45
	movq	%rax, %rsi
	addq	$36, %r10
	movb	$1, %al
	jmp	.LBB39_5
.LBB39_44:
	addq	%rcx, %rdi
	jb	.LBB39_45
	leaq	1(%rbx), %rsi
	movq	%rdi, %rax
	xorl	%edx, %edx
	divq	%rsi
	addq	%rax, -48(%rbp)
	jb	.LBB39_45
	movq	-48(%rbp), %rax
	shrq	$32, %rax
	jne	.LBB39_45
	movq	-48(%rbp), %rax
	xorl	$55296, %eax
	addl	$-1114112, %eax
	cmpl	$-1112064, %eax
	jb	.LBB39_45
	cmpq	-112(%rbp), %rbx
	je	.LBB39_45
	movq	%rdx, %rdi
	subq	%rdx, %rbx
	jbe	.LBB39_34
	leaq	15(%rbx), %rax
	andq	$-16, %rax
	decq	%rbx
	vpbroadcastq	%rbx, %zmm3
	negq	%rax
	xorl	%edx, %edx
	vmovdqa64	%zmm2, %zmm4
	vmovdqa64	%zmm0, %zmm5
.LBB39_33:
	vpcmpleuq	%zmm3, %zmm4, %k0
	vpcmpleuq	%zmm3, %zmm5, %k1
	kunpckbw	%k0, %k1, %k0
	vpmovm2d	%k0, %zmm6
	vpshufd	$27, %zmm6, %zmm6
	vshufi64x2	$27, %zmm6, %zmm6, %zmm6
	vpmovd2m	%zmm6, %k1
	vmovdqu32	-4(%r15,%rdx,4), %zmm6 {%k1} {z}
	vmovdqu32	%zmm6, (%r15,%rdx,4) {%k1}
	vpaddq	%zmm1, %zmm4, %zmm4
	vpaddq	%zmm1, %zmm5, %zmm5
	addq	$-16, %rdx
	cmpq	%rdx, %rax
	jne	.LBB39_33
	jmp	.LBB39_35
.LBB39_34:
	cmpq	$128, %rdi
	jae	.LBB39_52
.LBB39_35:
	movq	-48(%rbp), %rax
	movl	%eax, -640(%rbp,%rdi,4)
	cmpq	-104(%rbp), %r14
	je	.LBB39_41
	movb	(%r14), %r9b
	movq	%rcx, %rax
	xorl	%edx, %edx
	divq	-72(%rbp)
	movq	%rax, %rcx
	xorl	%edx, %edx
	divq	%rsi
	movq	%rax, %rdx
	addq	%rcx, %rdx
	cmpq	$456, %rdx
	jb	.LBB39_37
	xorl	%ecx, %ecx
	movabsq	$-1581149492032247281, %rax
.LBB39_39:
	mulxq	%rax, %r8, %r8
	shrq	$5, %r8
	addq	$36, %rcx
	cmpq	$15959, %rdx
	movq	%r8, %rdx
	ja	.LBB39_39
	jmp	.LBB39_40
.LBB39_37:
	movq	%rdx, %r8
	xorl	%ecx, %ecx
.LBB39_40:
	incq	%rdi
	leal	(,%r8,4), %eax
	leal	(%rax,%rax,8), %eax
	addl	$38, %r8d
	xorl	%edx, %edx
	divw	%r8w
	movzwl	%ax, %r13d
	addq	%rcx, %r13
	addq	$4, -64(%rbp)
	addq	$4, %r15
	movl	$2, %eax
	movq	%rax, -72(%rbp)
	movq	%rsi, %rbx
	jmp	.LBB39_4
.LBB39_45:
	movq	-88(%rbp), %rax
	movq	(%rax), %r13
	movq	8(%rax), %rax
	movq	24(%rax), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.29(%rip), %rsi
	movl	$9, %edx
	movq	%r13, %rdi
	movq	%rax, %rbx
	vzeroupper
	callq	*%rax
	movb	$1, %r12b
	testb	%al, %al
	jne	.LBB39_50
	movq	-128(%rbp), %rdx
	testq	%rdx, %rdx
	je	.LBB39_49
	movq	%r13, %rdi
	movq	-120(%rbp), %rsi
	movq	%rbx, %r14
	callq	*%rbx
	testb	%al, %al
	jne	.LBB39_50
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.23(%rip), %rsi
	movl	$1, %edx
	movq	%r13, %rdi
	callq	*%r14
	testb	%al, %al
	jne	.LBB39_50
.LBB39_49:
	movq	%r13, %rdi
	movq	-80(%rbp), %rsi
	movq	-96(%rbp), %rdx
	movq	%rbx, %r14
	callq	*%rbx
	testb	%al, %al
	je	.LBB39_53
.LBB39_50:
	movl	%r12d, %eax
	addq	$600, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB39_53:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.30(%rip), %rsi
	movl	$1, %edx
	movq	%r13, %rdi
	movq	%r14, %rax
	addq	$600, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*%rax
.LBB39_41:
	.cfi_def_cfa %rbp, 16
	leaq	-640(%rbp), %r14
	leaq	-52(%rbp), %rbx
	movq	_RNvXsk_NtCs2k2z8Zem4rB_4core3fmtcNtB5_7Display3fmt@GOTPCREL(%rip), %r15
.LBB39_42:
	movl	(%r14), %eax
	movl	%eax, -52(%rbp)
	movq	%rbx, %rdi
	movq	-88(%rbp), %rsi
	vzeroupper
	callq	*%r15
	movl	%eax, %r12d
	testb	%al, %al
	jne	.LBB39_50
	addq	$4, %r14
	addq	$-4, -64(%rbp)
	jne	.LBB39_42
	jmp	.LBB39_50
.LBB39_52:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.13(%rip), %rdx
	movl	$128, %esi
	vzeroupper
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.Lfunc_end39:
	.size	_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt, .Lfunc_end39-_RNvXs0_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_5IdentNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.cfi_endproc

	.section	.text._RNvXs1_Cs8dXjxA0JZyF_14rustc_demangleNtB5_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,"ax",@progbits
	.globl	_RNvXs1_Cs8dXjxA0JZyF_14rustc_demangleNtB5_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.type	_RNvXs1_Cs8dXjxA0JZyF_14rustc_demangleNtB5_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,@function
_RNvXs1_Cs8dXjxA0JZyF_14rustc_demangleNtB5_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt:
.Lfunc_begin40:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%rbx
	subq	$56, %rsp
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r14
	movq	%rdi, %rbx
	cmpl	$1, (%rdi)
	jne	.LBB40_3
	leaq	8(%rbx), %rax
	btl	$23, 16(%r14)
	movq	%rax, -40(%rbp)
	movq	$0, -64(%rbp)
	movq	$1000000, -56(%rbp)
	movq	%r14, -48(%rbp)
	leaq	-40(%rbp), %rax
	leaq	-80(%rbp), %rcx
	jb	.LBB40_6
	movq	%rax, (%rcx)
	movq	_RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_@GOTPCREL(%rip), %rax
	movq	%rax, 8(%rcx)
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714(%rip), %rsi
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.142.llvm.424815601038267714(%rip), %rdx
	jmp	.LBB40_7
.LBB40_3:
	movq	32(%rbx), %rsi
	movq	40(%rbx), %rdx
	movq	(%r14), %r15
	movq	8(%r14), %rax
	movq	24(%rax), %r14
	movq	%r15, %rdi
	callq	*%r14
	movl	%eax, %ecx
	movb	$1, %al
	testb	%cl, %cl
	je	.LBB40_4
	jmp	.LBB40_5
.LBB40_6:
	movq	%rax, (%rcx)
	movq	_RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_@GOTPCREL(%rip), %rax
	movq	%rax, 8(%rcx)
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714(%rip), %rsi
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.144.llvm.424815601038267714(%rip), %rdx
.LBB40_7:
	leaq	-64(%rbp), %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core3fmt5write@GOTPCREL(%rip)
	cmpl	$1, -64(%rbp)
	jne	.LBB40_10
	testb	%al, %al
	je	.LBB40_13
	movq	(%r14), %rdi
	movq	8(%r14), %rax
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.145.llvm.424815601038267714(%rip), %rsi
	movl	$20, %edx
	callq	*24(%rax)
.LBB40_10:
	testb	%al, %al
	je	.LBB40_11
	movb	$1, %al
	jmp	.LBB40_5
.LBB40_11:
	movq	(%r14), %r15
	movq	8(%r14), %rax
	movq	24(%rax), %r14
.LBB40_4:
	movq	48(%rbx), %rsi
	movq	56(%rbx), %rdx
	movq	%r15, %rdi
	callq	*%r14
.LBB40_5:
	addq	$56, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB40_13:
	.cfi_def_cfa %rbp, 16
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.146.llvm.424815601038267714(%rip), %rdi
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.38.llvm.424815601038267714(%rip), %rcx
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.147.llvm.424815601038267714(%rip), %r8
	leaq	-25(%rbp), %rdx
	movl	$55, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6result13unwrap_failed@GOTPCREL(%rip)
.Lfunc_end40:
	.size	_RNvXs1_Cs8dXjxA0JZyF_14rustc_demangleNtB5_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt, .Lfunc_end40-_RNvXs1_Cs8dXjxA0JZyF_14rustc_demangleNtB5_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.cfi_endproc

	.section	.text._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,"ax",@progbits
	.globl	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.type	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,@function
_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle:
.Lfunc_begin41:
	.cfi_startproc
	movq	(%rdi), %rax
	movzbl	(%rax), %eax
	leaq	.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle(%rip), %rcx
	movzbl	(%rax,%rcx), %edx
	leaq	.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	movq	(%rsi), %rdi
	movq	8(%rsi), %rcx
	movq	%rax, %rsi
	jmpq	*24(%rcx)
.Lfunc_end41:
	.size	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle, .Lfunc_end41-_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.cfi_endproc

	.section	.text._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,"ax",@progbits
	.globl	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.type	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,@function
_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle:
.Lfunc_begin42:
	.cfi_startproc
	movq	%rsi, %rdx
	movq	(%rdi), %rax
	movq	8(%rdi), %rsi
	movq	%rax, %rdi
	jmpq	*_RNvXsh_NtCs2k2z8Zem4rB_4core3fmteNtB5_5Debug3fmt@GOTPCREL(%rip)
.Lfunc_end42:
	.size	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle, .Lfunc_end42-_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtReNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.cfi_endproc

	.section	.text._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRhNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,"ax",@progbits
	.globl	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRhNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.type	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRhNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,@function
_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRhNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle:
.Lfunc_begin43:
	.cfi_startproc
	movq	(%rdi), %rdi
	movl	16(%rsi), %eax
	btl	$25, %eax
	jb	.LBB43_3
	btl	$26, %eax
	jb	.LBB43_2
	jmpq	*_RNvXNtNtNtCs2k2z8Zem4rB_4core3fmt3num3imphNtB6_7Display3fmt@GOTPCREL(%rip)
.LBB43_3:
	jmpq	*_RNvXse_NtNtCs2k2z8Zem4rB_4core3fmt3numhNtB7_8LowerHex3fmt@GOTPCREL(%rip)
.LBB43_2:
	jmpq	*_RNvXsg_NtNtCs2k2z8Zem4rB_4core3fmt3numhNtB7_8UpperHex3fmt@GOTPCREL(%rip)
.Lfunc_end43:
	.size	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRhNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle, .Lfunc_end43-_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRhNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.cfi_endproc

	.section	.text._RNvXs1h_NtCs2k2z8Zem4rB_4core3fmtQShNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,"ax",@progbits
	.globl	_RNvXs1h_NtCs2k2z8Zem4rB_4core3fmtQShNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.type	_RNvXs1h_NtCs2k2z8Zem4rB_4core3fmtQShNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,@function
_RNvXs1h_NtCs2k2z8Zem4rB_4core3fmtQShNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle:
.Lfunc_begin44:
	.cfi_startproc
	movq	%rsi, %rdx
	movq	(%rdi), %rax
	movq	8(%rdi), %rsi
	movq	%rax, %rdi
	jmpq	*_RNvXsr_NtCs2k2z8Zem4rB_4core3fmtShNtB5_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle@GOTPCREL(%rip)
.Lfunc_end44:
	.size	_RNvXs1h_NtCs2k2z8Zem4rB_4core3fmtQShNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle, .Lfunc_end44-_RNvXs1h_NtCs2k2z8Zem4rB_4core3fmtQShNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.cfi_endproc

	.section	.text._RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_,"ax",@progbits
	.globl	_RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_
	.type	_RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_,@function
_RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_:
.Lfunc_begin45:
	.cfi_startproc
	movq	(%rdi), %rdi
	jmpq	*_RNvXs_Cs8dXjxA0JZyF_14rustc_demangleNtB4_13DemangleStyleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt@GOTPCREL(%rip)
.Lfunc_end45:
	.size	_RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_, .Lfunc_end45-_RNvXs1i_NtCs2k2z8Zem4rB_4core3fmtRNtCs8dXjxA0JZyF_14rustc_demangle13DemangleStyleNtB6_7Display3fmtBy_
	.cfi_endproc

	.section	.text._RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714,"ax",@progbits
	.hidden	_RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714
	.globl	_RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714
	.type	_RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714,@function
_RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714:
.Lfunc_begin46:
	.cfi_startproc
	movq	(%rsi), %rdi
	movq	8(%rsi), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.149(%rip), %rsi
	movl	$18, %edx
	jmpq	*24(%rax)
.Lfunc_end46:
	.size	_RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714, .Lfunc_end46-_RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714
	.cfi_endproc

	.section	.text._RNvXsK_NtCs2k2z8Zem4rB_4core3fmtNtB5_5ErrorNtB5_5Debug3fmt,"ax",@progbits
	.type	_RNvXsK_NtCs2k2z8Zem4rB_4core3fmtNtB5_5ErrorNtB5_5Debug3fmt,@function
_RNvXsK_NtCs2k2z8Zem4rB_4core3fmtNtB5_5ErrorNtB5_5Debug3fmt:
.Lfunc_begin47:
	.cfi_startproc
	movq	(%rsi), %rdi
	movq	8(%rsi), %rax
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.150(%rip), %rsi
	movl	$5, %edx
	jmpq	*24(%rax)
.Lfunc_end47:
	.size	_RNvXsK_NtCs2k2z8Zem4rB_4core3fmtNtB5_5ErrorNtB5_5Debug3fmt, .Lfunc_end47-_RNvXsK_NtCs2k2z8Zem4rB_4core3fmtNtB5_5ErrorNtB5_5Debug3fmt
	.cfi_endproc

	.section	.text._RNvXs_Cs8dXjxA0JZyF_14rustc_demangleNtB4_13DemangleStyleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,"ax",@progbits
	.globl	_RNvXs_Cs8dXjxA0JZyF_14rustc_demangleNtB4_13DemangleStyleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.type	_RNvXs_Cs8dXjxA0JZyF_14rustc_demangleNtB4_13DemangleStyleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt,@function
_RNvXs_Cs8dXjxA0JZyF_14rustc_demangleNtB4_13DemangleStyleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt:
.Lfunc_begin48:
	.cfi_startproc
	cmpq	$0, (%rdi)
	je	.LBB48_1
	jmpq	*_RNvXNtCs8dXjxA0JZyF_14rustc_demangle6legacyNtB2_8DemangleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt@GOTPCREL(%rip)
.LBB48_1:
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	subq	$48, %rsp
	vmovups	8(%rdi), %xmm0
	leaq	-48(%rbp), %rdi
	vmovups	%xmm0, (%rdi)
	movq	$0, 16(%rdi)
	xorl	%eax, %eax
	movl	%eax, 24(%rdi)
	movq	%rsi, 32(%rdi)
	movl	%eax, 40(%rdi)
	movl	$1, %esi
	callq	_RNvMs4_NtCs8dXjxA0JZyF_14rustc_demangle2v0NtB5_7Printer10print_path
	addq	$48, %rsp
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end48:
	.size	_RNvXs_Cs8dXjxA0JZyF_14rustc_demangleNtB4_13DemangleStyleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt, .Lfunc_end48-_RNvXs_Cs8dXjxA0JZyF_14rustc_demangleNtB4_13DemangleStyleNtNtCs2k2z8Zem4rB_4core3fmt7Display3fmt
	.cfi_endproc

	.section	.text._RNvXsc_NtNtCs2k2z8Zem4rB_4core3num5errorNtB5_13ParseIntErrorNtNtB9_3fmt5Debug3fmt,"ax",@progbits
	.type	_RNvXsc_NtNtCs2k2z8Zem4rB_4core3num5errorNtB5_13ParseIntErrorNtNtB9_3fmt5Debug3fmt,@function
_RNvXsc_NtNtCs2k2z8Zem4rB_4core3num5errorNtB5_13ParseIntErrorNtNtB9_3fmt5Debug3fmt:
.Lfunc_begin49:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	subq	$16, %rsp
	movq	%rsi, %rax
	leaq	-8(%rbp), %r9
	movq	%rdi, (%r9)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.148(%rip), %rcx
	movq	%rcx, (%rsp)
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.151(%rip), %rsi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.152(%rip), %rcx
	movl	$13, %edx
	movl	$4, %r8d
	movq	%rax, %rdi
	callq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter26debug_struct_field1_finish@GOTPCREL(%rip)
	addq	$16, %rsp
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end49:
	.size	_RNvXsc_NtNtCs2k2z8Zem4rB_4core3num5errorNtB5_13ParseIntErrorNtNtB9_3fmt5Debug3fmt, .Lfunc_end49-_RNvXsc_NtNtCs2k2z8Zem4rB_4core3num5errorNtB5_13ParseIntErrorNtNtB9_3fmt5Debug3fmt
	.cfi_endproc

	.section	.text._RNvXsr_NtCs2k2z8Zem4rB_4core3fmtShNtB5_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,"ax",@progbits
	.globl	_RNvXsr_NtCs2k2z8Zem4rB_4core3fmtShNtB5_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.type	_RNvXsr_NtCs2k2z8Zem4rB_4core3fmtShNtB5_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,@function
_RNvXsr_NtCs2k2z8Zem4rB_4core3fmtShNtB5_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle:
.Lfunc_begin50:
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
	movq	%rdx, %r15
	movq	%rsi, %rbx
	movq	%rdi, %r14
	movq	(%rdx), %rdi
	movq	8(%rdx), %rax
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.198.llvm.9794848731438112354(%rip), %rsi
	movl	$1, %edx
	callq	*24(%rax)
	movq	%r15, -56(%rbp)
	movb	%al, -48(%rbp)
	movb	$0, -47(%rbp)
	testq	%rbx, %rbx
	je	.LBB50_4
	leaq	-56(%rbp), %r12
	leaq	-64(%rbp), %r13
	movq	_RNvMs6_NtNtCs2k2z8Zem4rB_4core3fmt8buildersNtB5_9DebugList5entry@GOTPCREL(%rip), %r15
.LBB50_2:
	movq	%r14, -64(%rbp)
	incq	%r14
	movq	%r12, %rdi
	movq	%r13, %rsi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.9(%rip), %rdx
	callq	*%r15
	decq	%rbx
	jne	.LBB50_2
	movb	$1, %al
	cmpb	$0, -48(%rbp)
	jne	.LBB50_6
.LBB50_5:
	movq	-56(%rbp), %rax
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	anon.a647af78948fdeb0321157bdb82ef7e0.54.llvm.9794848731438112354(%rip), %rsi
	movl	$1, %edx
	callq	*24(%rax)
.LBB50_6:
	addq	$24, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB50_4:
	.cfi_def_cfa %rbp, 16
	movl	%eax, %ecx
	movb	$1, %al
	testb	%cl, %cl
	je	.LBB50_5
	jmp	.LBB50_6
.Lfunc_end50:
	.size	_RNvXsr_NtCs2k2z8Zem4rB_4core3fmtShNtB5_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle, .Lfunc_end50-_RNvXsr_NtCs2k2z8Zem4rB_4core3fmtShNtB5_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.cfi_endproc

	.section	.text._RNvXss_NtCs2k2z8Zem4rB_4core3fmtuNtB5_5Debug3fmt,"ax",@progbits
	.type	_RNvXss_NtCs2k2z8Zem4rB_4core3fmtuNtB5_5Debug3fmt,@function
_RNvXss_NtCs2k2z8Zem4rB_4core3fmtuNtB5_5Debug3fmt:
.Lfunc_begin51:
	.cfi_startproc
	movq	%rsi, %rdi
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.84(%rip), %rsi
	movl	$2, %edx
	jmpq	*_RNvMsa_NtCs2k2z8Zem4rB_4core3fmtNtB5_9Formatter3pad@GOTPCREL(%rip)
.Lfunc_end51:
	.size	_RNvXss_NtCs2k2z8Zem4rB_4core3fmtuNtB5_5Debug3fmt, .Lfunc_end51-_RNvXss_NtCs2k2z8Zem4rB_4core3fmtuNtB5_5Debug3fmt
	.cfi_endproc

	.section	.text._RNvXsv_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcherNtB5_8Searcher4next,"ax",@progbits
	.type	_RNvXsv_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcherNtB5_8Searcher4next,@function
_RNvXsv_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcherNtB5_8Searcher4next:
.Lfunc_begin52:
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
	subq	$40, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	(%rsi), %rax
	testq	%rax, %rax
	je	.LBB52_5
	cmpl	$1, %eax
	jne	.LBB52_10
	movq	8(%rsi), %rax
	movq	80(%rsi), %rcx
	movl	$2, %r9d
	cmpq	%rcx, %rax
	jae	.LBB52_59
	movq	72(%rsi), %r8
	movb	(%r8,%rax), %r9b
	leaq	1(%rax), %rdx
	cmpb	24(%rsi), %r9b
	jne	.LBB52_32
	movq	%rdx, 8(%rsi)
	movq	%rax, 8(%rdi)
	jmp	.LBB52_57
.LBB52_5:
	movl	$2, %r9d
	cmpb	$0, 26(%rsi)
	jne	.LBB52_59
	movb	24(%rsi), %r8b
	movl	%r8d, %eax
	xorb	$1, %al
	movb	%al, 24(%rsi)
	movq	8(%rsi), %rdx
	movq	72(%rsi), %rax
	movq	80(%rsi), %rcx
	testq	%rdx, %rdx
	je	.LBB52_39
	cmpq	%rcx, %rdx
	jae	.LBB52_38
	cmpb	$-64, (%rax,%rdx)
	jge	.LBB52_39
.LBB52_9:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.159(%rip), %r8
	movq	%rax, %rdi
	movq	%rcx, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core3str16slice_error_fail@GOTPCREL(%rip)
.LBB52_10:
	movq	40(%rsi), %rdx
	movq	80(%rsi), %r8
	movl	$2, %r9d
	cmpq	%r8, %rdx
	je	.LBB52_59
	movq	96(%rsi), %rax
	leaq	(%rax,%rdx), %r13
	decq	%r13
	cmpq	%r8, %r13
	jae	.LBB52_30
	movq	56(%rsi), %r9
	movq	72(%rsi), %rbx
	movq	88(%rsi), %r11
	movq	32(%rsi), %rcx
	movq	%rcx, -80(%rbp)
	movq	8(%rsi), %r12
	movq	24(%rsi), %rcx
	movq	%rax, %r10
	subq	%rcx, %r10
	movq	%r10, -64(%rbp)
	movq	%rdx, %r10
	subq	%r12, %r10
	incq	%r10
	movq	%r10, -56(%rbp)
	addq	%rdx, %rcx
	movq	%rcx, -72(%rbp)
	leaq	(%rax,%rdx), %r14
	leaq	(%rbx,%rdx), %r15
	movq	%r9, %r10
	movq	%rdx, %rcx
	movq	%rbx, -48(%rbp)
.LBB52_13:
	cmpq	%rcx, %rdx
	jne	.LBB52_50
	movzbl	(%rbx,%r13), %ecx
	movq	-80(%rbp), %r13
	btq	%rcx, %r13
	jae	.LBB52_22
	cmpq	%r10, %r12
	movq	%r10, %rcx
	cmovaq	%r12, %rcx
	cmpq	$-1, %r9
	cmoveq	%r12, %rcx
	cmpq	%rax, %rcx
	movq	%rax, %r13
	cmovaq	%rcx, %r13
	jae	.LBB52_18
.LBB52_16:
	movb	(%r11,%rcx), %bl
	cmpb	(%r15,%rcx), %bl
	jne	.LBB52_25
	incq	%rcx
	cmpq	%rcx, %r13
	jne	.LBB52_16
.LBB52_18:
	cmpq	$-1, %r9
	movq	%r10, %r13
	movl	$0, %ecx
	cmoveq	%rcx, %r13
	leaq	-1(%r12), %rcx
	cmpq	%r12, %r13
	jae	.LBB52_35
.LBB52_19:
	cmpq	%rax, %rcx
	jae	.LBB52_65
	movb	(%r11,%rcx), %bl
	cmpb	(%r15,%rcx), %bl
	jne	.LBB52_24
	cmpq	%rcx, %r13
	leaq	-1(%rcx), %rcx
	jb	.LBB52_19
	jmp	.LBB52_35
.LBB52_22:
	movq	%r14, 40(%rsi)
	movq	%r14, %rcx
	cmpq	$-1, %r9
	je	.LBB52_28
	xorl	%r13d, %r13d
	movq	%r14, %rcx
	jmp	.LBB52_27
.LBB52_24:
	movq	-72(%rbp), %rcx
	movq	%rcx, 40(%rsi)
	movq	-64(%rbp), %r13
	cmpq	$-1, %r9
	movq	-48(%rbp), %rbx
	jne	.LBB52_27
	jmp	.LBB52_28
.LBB52_25:
	addq	-56(%rbp), %rcx
	movq	%rcx, 40(%rsi)
	cmpq	$-1, %r9
	je	.LBB52_29
	xorl	%r13d, %r13d
	movq	-48(%rbp), %rbx
.LBB52_27:
	movq	%r13, 56(%rsi)
	movq	%r13, %r10
.LBB52_28:
	leaq	-1(%rax), %r13
	addq	%rcx, %r13
	cmpq	%r8, %r13
	jb	.LBB52_13
	jmp	.LBB52_30
.LBB52_29:
	movq	-48(%rbp), %rbx
	jmp	.LBB52_28
.LBB52_30:
	movq	%r8, %rcx
.LBB52_31:
	cmpq	%rcx, %r8
	cmovaq	%r8, %rcx
	movq	%rcx, 40(%rsi)
	movq	%rdx, 8(%rdi)
	movq	%r8, 16(%rdi)
	jmp	.LBB52_48
.LBB52_32:
	cmpq	%rcx, %rdx
	jae	.LBB52_47
.LBB52_33:
	cmpb	$-65, (%r8,%rdx)
	jg	.LBB52_46
	incq	%rdx
	cmpq	%rdx, %rcx
	jne	.LBB52_33
	jmp	.LBB52_47
.LBB52_35:
	movq	%r14, 40(%rsi)
	cmpq	$-1, %r9
	je	.LBB52_37
	movq	$0, 56(%rsi)
.LBB52_37:
	movq	%rdx, 8(%rdi)
	movq	%r14, 16(%rdi)
	jmp	.LBB52_58
.LBB52_38:
	jne	.LBB52_9
.LBB52_39:
	cmpq	%rcx, %rdx
	jne	.LBB52_42
	testb	%r8b, %r8b
	jne	.LBB52_56
	movb	$1, 26(%rsi)
	jmp	.LBB52_59
.LBB52_42:
	movzbl	(%rax,%rdx), %ecx
	testb	%cl, %cl
	jns	.LBB52_55
	movl	%ecx, %r9d
	andl	$31, %r9d
	movzbl	1(%rax,%rdx), %r11d
	andl	$63, %r11d
	cmpb	$-33, %cl
	jbe	.LBB52_49
	movzbl	2(%rax,%rdx), %r10d
	shll	$6, %r11d
	andl	$63, %r10d
	orl	%r11d, %r10d
	cmpb	$-16, %cl
	jb	.LBB52_54
	movzbl	3(%rax,%rdx), %ecx
	andl	$7, %r9d
	shll	$18, %r9d
	shll	$6, %r10d
	andl	$63, %ecx
	orl	%r10d, %ecx
	orl	%r9d, %ecx
	jmp	.LBB52_55
.LBB52_46:
	movq	%rdx, %rcx
.LBB52_47:
	movq	%rcx, 8(%rsi)
	movq	%rax, 8(%rdi)
	movq	%rcx, 16(%rdi)
.LBB52_48:
	movl	$1, %r9d
	jmp	.LBB52_59
.LBB52_49:
	shll	$6, %r9d
	orl	%r11d, %r9d
	movl	%r9d, %ecx
	jmp	.LBB52_55
.LBB52_50:
	cmpq	%r8, %rcx
	jae	.LBB52_31
	movq	%rcx, %rax
.LBB52_52:
	cmpb	$-65, (%rbx,%rax)
	jg	.LBB52_64
	incq	%rax
	cmpq	%rax, %r8
	je	.LBB52_31
	jmp	.LBB52_52
.LBB52_54:
	shll	$12, %r9d
	orl	%r9d, %r10d
	movl	%r10d, %ecx
.LBB52_55:
	testb	%r8b, %r8b
	je	.LBB52_60
.LBB52_56:
	movq	%rdx, 8(%rdi)
.LBB52_57:
	movq	%rdx, 16(%rdi)
.LBB52_58:
	xorl	%r9d, %r9d
.LBB52_59:
	movq	%r9, (%rdi)
	addq	$40, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB52_60:
	.cfi_def_cfa %rbp, 16
	movl	$1, %r9d
	movl	$1, %eax
	cmpl	$128, %ecx
	jb	.LBB52_63
	movl	$2, %eax
	cmpl	$2048, %ecx
	jb	.LBB52_63
	cmpl	$65536, %ecx
	movl	$4, %eax
	sbbq	$0, %rax
.LBB52_63:
	addq	%rdx, %rax
	movq	%rax, 8(%rsi)
	movq	%rdx, 8(%rdi)
	movq	%rax, 16(%rdi)
	jmp	.LBB52_59
.LBB52_64:
	movq	%rax, %r8
	jmp	.LBB52_31
.LBB52_65:
	leaq	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.11(%rip), %rdx
	movq	%rcx, %rdi
	movq	%rax, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.Lfunc_end52:
	.size	_RNvXsv_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcherNtB5_8Searcher4next, .Lfunc_end52-_RNvXsv_NtNtCs2k2z8Zem4rB_4core3str7patternNtB5_11StrSearcherNtB5_8Searcher4next
	.cfi_endproc

	.section	.rodata.cst16,"aM",@progbits,16
	.p2align	4, 0x0
.LCPI53_0:
	.long	18
	.long	12
	.long	6
	.long	0
.LCPI53_1:
	.byte	240
	.byte	128
	.byte	128
	.byte	128
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
.LCPI53_2:
	.byte	255
	.byte	63
	.byte	63
	.byte	63
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
	.zero	1
.LCPI53_3:
	.byte	240
	.byte	128
	.byte	128
	.byte	128
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.section	.rodata.cst4,"aM",@progbits,4
	.p2align	2, 0x0
.LCPI53_4:
	.byte	255
	.byte	63
	.byte	63
	.byte	63
	.section	.text._RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write10write_charB5_,"ax",@progbits
	.globl	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write10write_charB5_
	.type	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write10write_charB5_,@function
_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write10write_charB5_:
.Lfunc_begin53:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	subq	$16, %rsp
	movl	$0, -4(%rbp)
	cmpl	$128, %esi
	jae	.LBB53_1
	movb	%sil, -4(%rbp)
	movl	$1, %edx
	jmp	.LBB53_6
.LBB53_1:
	vpbroadcastd	%esi, %xmm0
	vpsrlvd	.LCPI53_0(%rip), %xmm0, %xmm0
	vpmovdb	%xmm0, %xmm1
	vmovd	.LCPI53_3(%rip), %xmm0
	vpternlogd	$248, .LCPI53_4(%rip){1to4}, %xmm1, %xmm0
	cmpl	$2048, %esi
	jae	.LBB53_3
	vpextrb	$2, %xmm1, %eax
	orb	$-64, %al
	movb	%al, -4(%rbp)
	vpextrb	$3, %xmm0, -3(%rbp)
	movl	$2, %edx
	jmp	.LBB53_6
.LBB53_3:
	cmpl	$65535, %esi
	ja	.LBB53_5
	vpextrb	$1, %xmm1, %eax
	orb	$-32, %al
	movb	%al, -4(%rbp)
	vpextrb	$2, %xmm0, -3(%rbp)
	vpextrb	$3, %xmm0, -2(%rbp)
	movl	$3, %edx
	jmp	.LBB53_6
.LBB53_5:
	vmovd	%xmm0, -4(%rbp)
	movl	$4, %edx
.LBB53_6:
	movq	8(%rdi), %rax
	subq	%rdx, %rax
	setb	%cl
	orb	(%rdi), %cl
	movzbl	%cl, %ecx
	movq	%rcx, (%rdi)
	movq	%rax, 8(%rdi)
	movb	$1, %al
	testb	%cl, %cl
	jne	.LBB53_8
	movq	16(%rdi), %rax
	movq	(%rax), %rdi
	movq	8(%rax), %rax
	leaq	-4(%rbp), %rsi
	callq	*24(%rax)
.LBB53_8:
	addq	$16, %rsp
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end53:
	.size	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write10write_charB5_, .Lfunc_end53-_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write10write_charB5_
	.cfi_endproc

	.section	.text._RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write9write_fmtB5_,"ax",@progbits
	.globl	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write9write_fmtB5_
	.type	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write9write_fmtB5_,@function
_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write9write_fmtB5_:
.Lfunc_begin54:
	.cfi_startproc
	movq	%rdx, %rcx
	movq	%rsi, %rdx
	leaq	anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714(%rip), %rsi
	jmpq	*_RNvNtCs2k2z8Zem4rB_4core3fmt5write@GOTPCREL(%rip)
.Lfunc_end54:
	.size	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write9write_fmtB5_, .Lfunc_end54-_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write9write_fmtB5_
	.cfi_endproc

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0,@object
	.section	.rodata.cst16,"aM",@progbits,16
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0:
	.ascii	"{invalid syntax}"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.0, 16

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1:
	.ascii	"{recursion limit reached}"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.1, 25

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2:
	.byte	63
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.2, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.3,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.3,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.3:
	.ascii	"`fmt::Error`s should be impossible without a `fmt::Formatter`"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.3, 61

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4:
	.asciz	"/cargo/registry/25cdd57fae9f0462/rustc-demangle-0.1.28/src/v0.rs"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4, 65

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.5,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.5,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.5:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000\207\002\000\000\021\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.5, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.6,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.6:
	.ascii	"for<"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.6, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.7,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.7,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.7:
	.ascii	"> "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.7, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8:
	.ascii	", "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.8, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.9,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.9,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.9:
	.asciz	"\000\000\000\000\000\000\000\000\b\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRhNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.9, 32

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.10,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.10:
	.asciz	"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/core/src/str/pattern.rs"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.10, 80

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.11,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.11,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.11:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.10
	.asciz	"O\000\000\000\000\000\000\000A\006\000\000\024\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.11, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.12,@object
	.section	.rodata.cst16,"aM",@progbits,16
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.12:
	.ascii	"0123456789abcdef"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.12, 16

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.13,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.13,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.13:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000\212\000\000\000\r\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.13, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.14,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.14:
	.asciz	"9internal error: entered unreachable code: str::from_utf8(\300\004) = \300\" was expected to have 1 char, but \300\021 chars were found"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.14, 120

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.15,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.15,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.15:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000\\\001\000\000\032\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.15, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.16,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.16,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.16:
	.ascii	"internal error: entered unreachable code"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.16, 40

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.17,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.17,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.17:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\0001\001\000\000\026\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.17, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.18,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.18,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.18:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\0004\001\000\000G\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.18, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.19,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.19,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.19:
	.byte	67
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.19, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.20,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.20,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.20:
	.ascii	"unsafe "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.20, 7

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.21,@object
	.section	.rodata.cst8,"aM",@progbits,8
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.21:
	.ascii	"extern \""
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.21, 8

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.22,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.22,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.22:
	.ascii	"\" "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.22, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.23,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.23,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.23:
	.byte	45
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.23, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.24,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.24,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.24:
	.ascii	"fn("
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.24, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25:
	.byte	41
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.25, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.26,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.26:
	.ascii	" -> "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.26, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.27,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.27,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.27:
	.ascii	" + "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.27, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.28,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.28,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.28:
	.ascii	": "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.28, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.29,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.29,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.29:
	.ascii	"punycode{"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.29, 9

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.30,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.30,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.30:
	.byte	125
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.30, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.31,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.31,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.31:
	.ascii	".llvm."
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.31, 6

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714
anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714:
	.asciz	"/cargo/registry/25cdd57fae9f0462/rustc-demangle-0.1.28/src/lib.rs"
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714, 66

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.33,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.33,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.33:
	.quad	anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714
	.asciz	"A\000\000\000\000\000\000\000b\000\000\000\033\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.33, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.34,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.34,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.34:
	.quad	anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714
	.asciz	"A\000\000\000\000\000\000\000i\000\000\000\023\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.34, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.35,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.35,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.35:
	.asciz	"\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\001\000\000\000\000\000\000"
	.quad	_RNvXss_NtCs2k2z8Zem4rB_4core3fmtuNtB5_5Debug3fmt
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.35, 32

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.36,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.36,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.36:
	.ascii	"called `Result::unwrap()` on an `Err` value"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.36, 43

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.37,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.37,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.37:
	.asciz	"\000\000\000\000\000\000\000\000\001\000\000\000\000\000\000\000\001\000\000\000\000\000\000"
	.quad	_RNvXsc_NtNtCs2k2z8Zem4rB_4core3num5errorNtB5_13ParseIntErrorNtNtB9_3fmt5Debug3fmt
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.37, 32

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.38.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.38.llvm.424815601038267714,@object
	.section	.data.rel.ro.anon.544799d83aded4a3c8e9d51f48a8ae0a.38.llvm.424815601038267714,"aw",@progbits
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.38.llvm.424815601038267714
	.p2align	3, 0x0
anon.544799d83aded4a3c8e9d51f48a8ae0a.38.llvm.424815601038267714:
	.asciz	"\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\001\000\000\000\000\000\000"
	.quad	_RNvXs8_Cs8dXjxA0JZyF_14rustc_demangleNtB5_18SizeLimitExhaustedNtNtCs2k2z8Zem4rB_4core3fmt5Debug3fmt.llvm.424815601038267714
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.38.llvm.424815601038267714, 32

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.39,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.39,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.39:
	.asciz	"\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\001\000\000\000\000\000\000"
	.quad	_RNvXsK_NtCs2k2z8Zem4rB_4core3fmtNtB5_5ErrorNtB5_5Debug3fmt
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.39, 32

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.40,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.40,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.40:
	.byte	48
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.40, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.41,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.41,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.41:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000\036\001\000\0001\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.41, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.42,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.42,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.42:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000\277\001\000\000\037\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.42, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.43,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.43,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.43:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000\036\002\000\000\036\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.43, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.44,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.44,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.44:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000#\002\000\000\"\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.44, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.45,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.45,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.45:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000$\002\000\000%\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.45, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.46,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.46,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.46:
	.byte	91
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.46, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.47,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.47,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.47:
	.byte	93
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.47, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48:
	.zero	2,58
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.48, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.49,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.49,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.49:
	.ascii	"::{"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.49, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.50,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.50,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.50:
	.ascii	"closure"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.50, 7

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.51,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.51:
	.ascii	"shim"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.51, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.52,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.52,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.52:
	.byte	58
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.52, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.53,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.53,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.53:
	.byte	35
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.53, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54:
	.byte	60
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.54, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.55,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.55:
	.ascii	" as "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.55, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.56,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.56,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.56:
	.byte	62
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.56, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.57,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.57,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.57:
	.ascii	"#[splat] "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.57, 9

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.58,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.58,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.58:
	.byte	38
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.58, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.59,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.59,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.59:
	.byte	32
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.59, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.60,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.60:
	.ascii	"mut "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.60, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.61,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.61,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.61:
	.byte	42
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.61, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.62,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.62,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.62:
	.ascii	"const "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.62, 6

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.63,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.63,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.63:
	.ascii	"; "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.63, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64:
	.byte	40
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.64, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.65,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.65,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.65:
	.byte	44
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.65, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.66,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.66:
	.ascii	"dyn "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.66, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.67,@object
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.67:
	.ascii	" is "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.67, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68:
	.byte	95
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.69,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.69,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.69:
	.ascii	"false"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.69, 5

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.70,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.70:
	.ascii	"true"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.70, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71:
	.byte	123
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.71, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.72,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.72,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.72:
	.ascii	" { "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.72, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.73,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.73,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.73:
	.ascii	" }"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.73, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.74,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.74,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.74:
	.ascii	" = "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.74, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.75,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.75,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.75:
	.ascii	"0x"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.75, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.76,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.76,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.76:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000\371\004\000\000-\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.76, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.77,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.77,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.77:
	.byte	39
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.77, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.78,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.78,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.78:
	.ascii	"..="
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.78, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.79,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.79,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.79:
	.ascii	" | "
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.79, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.80,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.80,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.80:
	.ascii	"!null"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.80, 5

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.81,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.81:
	.ascii	"bool"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.81, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.82,@object
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.82:
	.ascii	"char"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.82, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.83,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.83,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.83:
	.ascii	"str"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.83, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.84,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.84,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.84:
	.ascii	"()"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.84, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.85,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.85,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.85:
	.ascii	"i8"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.85, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.86,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.86,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.86:
	.ascii	"i16"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.86, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.87,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.87,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.87:
	.ascii	"i32"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.87, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.88,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.88,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.88:
	.ascii	"i64"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.88, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.89,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.89:
	.ascii	"i128"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.89, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.90,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.90,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.90:
	.ascii	"isize"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.90, 5

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.91,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.91,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.91:
	.ascii	"u8"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.91, 2

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.92,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.92,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.92:
	.ascii	"u16"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.92, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.93,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.93,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.93:
	.ascii	"u32"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.93, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.94,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.94,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.94:
	.ascii	"u64"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.94, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.95,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.95:
	.ascii	"u128"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.95, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.96,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.96,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.96:
	.ascii	"usize"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.96, 5

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.97,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.97,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.97:
	.ascii	"f32"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.97, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.98,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.98,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.98:
	.ascii	"f64"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.98, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.99,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.99,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.99:
	.byte	33
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.99, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.100,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.100,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.100:
	.zero	3,46
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.100, 3

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.103,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.103,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.103:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\0002\000\000\000\023\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.103, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.104,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.104,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.104:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000/\000\000\000\023\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.104, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.105,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.105,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.105:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000+\000\000\000\023\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.105, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.106,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.106,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.106:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000K\000\000\000\016\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.106, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.107,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.107,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.107:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.4
	.asciz	"@\000\000\000\000\000\000\000Z\000\000\000(\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.107, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108:
	.asciz	"/cargo/registry/25cdd57fae9f0462/rustc-demangle-0.1.28/src/legacy.rs"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108, 69

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.109,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.109,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.109:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000f\000\000\000\034\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.109, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.113,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.113,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.113:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000=\000\000\000\013\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.113, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.114,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.114,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.114:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000:\000\000\000\013\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.114, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.115,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.115,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.115:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\0006\000\000\000\013\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.115, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.116,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.116,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.116:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000o\000\000\000'\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.116, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.117,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.117,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.117:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000r\000\000\000!\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.117, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.118,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.118,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.118:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000r\000\000\000H\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.118, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.119,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.119,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.119:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000s\000\000\000\032\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.119, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.122,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.122,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.122:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000~\000\000\000\035\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.122, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.123,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.123,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.123:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\264\000\000\000&\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.123, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.124,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.124,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.124:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\265\000\000\000!\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.124, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.125,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.125,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.125:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\212\000\000\000I\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.125, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.126,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.126,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.126:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\213\000\000\000\037\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.126, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.127,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.127,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.127:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\213\000\000\000/\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.127, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.135,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.135,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.135:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\235\000\000\0005\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.135, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.136,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.136,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.136:
	.byte	64
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.136, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.137,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.137,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.137:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\202\000\000\000,\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.137, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.138,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.138,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.138:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\204\000\000\000%\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.138, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.139,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.139,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.139:
	.byte	46
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.139, 1

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.140,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.140,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.140:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000\207\000\000\000%\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.140, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.141,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.141,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.141:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.108
	.asciz	"D\000\000\000\000\000\000\000p\000\000\000\035\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.141, 24

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.142.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.142.llvm.424815601038267714,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.142.llvm.424815601038267714
anon.544799d83aded4a3c8e9d51f48a8ae0a.142.llvm.424815601038267714:
	.asciz	"\300"
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.142.llvm.424815601038267714, 2

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714,@object
	.section	.data.rel.ro.anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714,"aw",@progbits
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714
	.p2align	3, 0x0
anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714:
	.asciz	"\000\000\000\000\000\000\000\000\030\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXs0_Cs8dXjxA0JZyF_14rustc_demangleINtB5_21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtB15_5Write9write_strB5_
	.quad	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write10write_charB5_
	.quad	_RNvYINtCs8dXjxA0JZyF_14rustc_demangle21SizeLimitedFmtAdapterQNtNtCs2k2z8Zem4rB_4core3fmt9FormatterENtBZ_5Write9write_fmtB5_
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.143.llvm.424815601038267714, 48

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.144.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.144.llvm.424815601038267714,@object
	.section	.rodata.anon.544799d83aded4a3c8e9d51f48a8ae0a.144.llvm.424815601038267714,"a",@progbits
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.144.llvm.424815601038267714
anon.544799d83aded4a3c8e9d51f48a8ae0a.144.llvm.424815601038267714:
	.asciz	"\301 \000\200`"
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.144.llvm.424815601038267714, 6

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.145.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.145.llvm.424815601038267714,@object
	.section	.rodata.anon.544799d83aded4a3c8e9d51f48a8ae0a.145.llvm.424815601038267714,"a",@progbits
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.145.llvm.424815601038267714
anon.544799d83aded4a3c8e9d51f48a8ae0a.145.llvm.424815601038267714:
	.ascii	"{size limit reached}"
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.145.llvm.424815601038267714, 20

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.146.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.146.llvm.424815601038267714,@object
	.section	.rodata.anon.544799d83aded4a3c8e9d51f48a8ae0a.146.llvm.424815601038267714,"a",@progbits
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.146.llvm.424815601038267714
anon.544799d83aded4a3c8e9d51f48a8ae0a.146.llvm.424815601038267714:
	.ascii	"`fmt::Error` from `SizeLimitedFmtAdapter` was discarded"
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.146.llvm.424815601038267714, 55

	.hidden	anon.544799d83aded4a3c8e9d51f48a8ae0a.147.llvm.424815601038267714
	.type	anon.544799d83aded4a3c8e9d51f48a8ae0a.147.llvm.424815601038267714,@object
	.section	.data.rel.ro.anon.544799d83aded4a3c8e9d51f48a8ae0a.147.llvm.424815601038267714,"aw",@progbits
	.globl	anon.544799d83aded4a3c8e9d51f48a8ae0a.147.llvm.424815601038267714
	.p2align	3, 0x0
anon.544799d83aded4a3c8e9d51f48a8ae0a.147.llvm.424815601038267714:
	.quad	anon.544799d83aded4a3c8e9d51f48a8ae0a.32.llvm.424815601038267714
	.asciz	"A\000\000\000\000\000\000\000S\001\000\000\036\000\000"
	.size	anon.544799d83aded4a3c8e9d51f48a8ae0a.147.llvm.424815601038267714, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.148,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.148,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.148:
	.asciz	"\000\000\000\000\000\000\000\000\b\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	_RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.148, 32

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.149,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.149,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.149:
	.ascii	"SizeLimitExhausted"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.149, 18

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.150,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.150,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.150:
	.ascii	"Error"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.150, 5

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.151,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.151,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.151:
	.ascii	"ParseIntError"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.151, 13

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.152,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.152:
	.ascii	"kind"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.152, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.153,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.153,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.153:
	.ascii	"Empty"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.153, 5

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.154,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.154,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.154:
	.ascii	"InvalidDigit"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.154, 12

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.155,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.155,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.155:
	.ascii	"PosOverflow"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.155, 11

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.156,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.156,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.156:
	.ascii	"NegOverflow"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.156, 11

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.157,@object
	.section	.rodata.cst4,"aM",@progbits,4
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.157:
	.ascii	"Zero"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.157, 4

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.158,@object
	.section	.rodata..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.158,"a",@progbits
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.158:
	.ascii	"NotAPowerOfTwo"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.158, 14

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.159,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.159,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.159:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.10
	.asciz	"O\000\000\000\000\000\000\000|\004\000\000$\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.159, 24

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.160,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.160:
	.asciz	"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/core/src/ops/function.rs"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.160, 81

	.type	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.161,@object
	.section	.data.rel.ro..Lanon.544799d83aded4a3c8e9d51f48a8ae0a.161,"aw",@progbits
	.p2align	3, 0x0
.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.161:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.160
	.asciz	"P\000\000\000\000\000\000\000\246\000\000\000\005\000\000"
	.size	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.161, 24

	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.54.llvm.9794848731438112354
	.hidden	anon.a647af78948fdeb0321157bdb82ef7e0.198.llvm.9794848731438112354
	.type	.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type,@object
	.section	.rodata..Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type,"a",@progbits
	.p2align	3, 0x0
.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type:
	.byte	2
	.byte	4
	.byte	4
	.byte	3
	.byte	3
	.byte	3
	.zero	1
	.byte	2
	.byte	5
	.byte	5
	.zero	1
	.byte	3
	.byte	3
	.byte	4
	.byte	4
	.byte	1
	.zero	1
	.zero	1
	.byte	3
	.byte	3
	.byte	2
	.byte	3
	.zero	1
	.byte	3
	.byte	3
	.byte	1
	.size	.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type, 26

	.type	.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type.41,@object
	.section	.data.rel.ro..Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type.41,"aw",@progbits
	.p2align	3, 0x0
.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type.41:
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.85
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.81
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.82
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.98
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.83
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.97
	.quad	0
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.91
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.90
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.96
	.quad	0
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.87
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.93
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.89
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.95
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.68
	.quad	0
	.quad	0
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.86
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.92
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.84
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.100
	.quad	0
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.88
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.94
	.quad	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.99
	.size	.Lswitch.table._RNvNtCs8dXjxA0JZyF_14rustc_demangle2v010basic_type.41, 208

	.type	.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,@object
	.section	.rodata..Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle,"a",@progbits
	.p2align	3, 0x0
.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle:
	.ascii	"\005\f\013\013\004\016"
	.size	.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle, 6

	.type	.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel,@object
	.section	.rodata..Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel,"a",@progbits
	.p2align	2, 0x0
.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel:
	.long	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.153-.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel
	.long	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.154-.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel
	.long	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.155-.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel
	.long	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.156-.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel
	.long	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.157-.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel
	.long	.Lanon.544799d83aded4a3c8e9d51f48a8ae0a.158-.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel
	.size	.Lswitch.table._RNvXs1g_NtCs2k2z8Zem4rB_4core3fmtRNtNtNtB8_3num5error12IntErrorKindNtB6_5Debug3fmtCs8dXjxA0JZyF_14rustc_demangle.42.rel, 24

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits

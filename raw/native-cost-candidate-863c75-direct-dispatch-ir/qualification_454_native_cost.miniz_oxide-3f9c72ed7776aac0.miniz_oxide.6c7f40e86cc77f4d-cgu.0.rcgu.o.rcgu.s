	.att_syntax
	.file	"miniz_oxide.6c7f40e86cc77f4d-cgu.0"
	.section	.text._RINvNtNtCs2k2z8Zem4rB_4core5slice5index5rangeINtNtNtB6_3ops5range14RangeInclusivejEECs9jwHs5ahBUL_11miniz_oxide,"ax",@progbits
	.globl	_RINvNtNtCs2k2z8Zem4rB_4core5slice5index5rangeINtNtNtB6_3ops5range14RangeInclusivejEECs9jwHs5ahBUL_11miniz_oxide
	.type	_RINvNtNtCs2k2z8Zem4rB_4core5slice5index5rangeINtNtNtB6_3ops5range14RangeInclusivejEECs9jwHs5ahBUL_11miniz_oxide,@function
_RINvNtNtCs2k2z8Zem4rB_4core5slice5index5rangeINtNtNtB6_3ops5range14RangeInclusivejEECs9jwHs5ahBUL_11miniz_oxide:
	.cfi_startproc
	movq	(%rdi), %rax
	movzbl	16(%rdi), %ecx
	movl	%ecx, %r8d
	xorl	$1, %r8d
	movq	(%rdi,%r8,8), %r8
	testl	%ecx, %ecx
	je	.LBB0_1
	cmpq	%rsi, %r8
	jbe	.LBB0_5
	jmp	.LBB0_2
.LBB0_1:
	cmpq	%rsi, %r8
	jae	.LBB0_2
	incq	%r8
.LBB0_5:
	movq	%r8, %rcx
	cmpq	%r8, %rax
	ja	.LBB0_6
	movq	%rcx, %rdx
	retq
.LBB0_2:
	xorl	%r9d, %r9d
	jmp	.LBB0_7
.LBB0_6:
	movl	$1, %r9d
	movq	%rax, %r8
.LBB0_7:
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	subq	$32, %rsp
	leaq	-24(%rbp), %rdi
	movq	%r9, (%rdi)
	movq	%r8, 8(%rdi)
	movq	%rcx, 16(%rdi)
	callq	*_RNvMsc_NtNtCs2k2z8Zem4rB_4core5slice5indexNtB5_10RangeError6report@GOTPCREL(%rip)
.Lfunc_end0:
	.size	_RINvNtNtCs2k2z8Zem4rB_4core5slice5index5rangeINtNtNtB6_3ops5range14RangeInclusivejEECs9jwHs5ahBUL_11miniz_oxide, .Lfunc_end0-_RINvNtNtCs2k2z8Zem4rB_4core5slice5index5rangeINtNtNtB6_3ops5range14RangeInclusivejEECs9jwHs5ahBUL_11miniz_oxide
	.cfi_endproc

	.section	.text._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core10decompress,"ax",@progbits
	.globl	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core10decompress
	.type	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core10decompress,@function
_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core10decompress:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%rbx
	pushq	%rax
	.cfi_offset %rbx, -24
	movq	%rdi, %rbx
	movl	24(%rbp), %eax
	subq	$8, %rsp
	pushq	%rax
	pushq	$-1
	pushq	16(%rbp)
	callq	*_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit@GOTPCREL(%rip)
	addq	$32, %rsp
	movq	%rbx, %rax
	addq	$8, %rsp
	popq	%rbx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end1:
	.size	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core10decompress, .Lfunc_end1-_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core10decompress
	.cfi_endproc

	.section	.text._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core11apply_match,"ax",@progbits
	.type	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core11apply_match,@function
_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core11apply_match:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	movq	%rdx, %r10
	movq	%rsi, %rax
	subq	%rcx, %rdx
	andq	%r9, %rdx
	cmpq	$3, %r8
	jne	.LBB2_7
	movq	%r10, %rcx
	subq	$-3, %rcx
	setae	%sil
	cmpq	%rax, %rcx
	seta	%cl
	orb	%sil, %cl
	jne	.LBB2_6
	leaq	1(%rdx), %rcx
	andq	%r9, %rcx
	cmpq	%rax, %rcx
	jae	.LBB2_6
	cmpq	%rdx, %rax
	jbe	.LBB2_6
	leaq	2(%rdx), %rsi
	andq	%r9, %rsi
	cmpq	%rax, %rsi
	jae	.LBB2_6
	movb	(%rdi,%rdx), %al
	movb	%al, (%rdi,%r10)
	movb	(%rdi,%rcx), %al
	movb	%al, 1(%rdi,%r10)
	movb	(%rdi,%rsi), %al
	movb	%al, 2(%rdi,%r10)
.LBB2_6:
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB2_7:
	.cfi_def_cfa %rbp, 16
	movq	%rdx, %rsi
	subq	%r10, %rsi
	setae	%r11b
	cmpq	%r8, %rsi
	setb	%sil
	testb	%sil, %r11b
	jne	.LBB2_16
	cmpq	%rcx, %r8
	ja	.LBB2_16
	leaq	(%rdx,%r8), %rsi
	cmpq	%rax, %rsi
	jae	.LBB2_16
	cmpq	%r10, %rdx
	jae	.LBB2_17
	subq	%r10, %rax
	jb	.LBB2_28
	cmpq	%rax, %r8
	ja	.LBB2_23
	cmpq	%rdx, %rsi
	jb	.LBB2_27
	cmpq	%r10, %rsi
	ja	.LBB2_27
	addq	%rdi, %r10
	addq	%rdi, %rdx
	jmp	.LBB2_22
.LBB2_16:
	movq	%rax, %rsi
	movq	%r10, %rcx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmp	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer
.LBB2_17:
	.cfi_def_cfa %rbp, 16
	subq	%rdx, %rax
	jb	.LBB2_29
	movq	%r8, %rsi
	addq	%r10, %rsi
	jb	.LBB2_24
	cmpq	%rdx, %rsi
	ja	.LBB2_24
	cmpq	%rax, %r8
	ja	.LBB2_25
	addq	%rdi, %rdx
	addq	%rdi, %r10
.LBB2_22:
	movq	%r10, %rdi
	movq	%rdx, %rsi
	movq	%r8, %rdx
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	jmpq	*memcpy@GOTPCREL(%rip)
.LBB2_23:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.17(%rip), %rcx
	jmp	.LBB2_26
.LBB2_24:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.14(%rip), %rcx
	movq	%r10, %rdi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB2_25:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.13(%rip), %rcx
.LBB2_26:
	xorl	%edi, %edi
	movq	%r8, %rsi
	movq	%rax, %rdx
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB2_27:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.16(%rip), %rcx
	movq	%rdx, %rdi
	movq	%r10, %rdx
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB2_28:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.8(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.15(%rip), %rdx
	jmp	.LBB2_30
.LBB2_29:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.8(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.12(%rip), %rdx
.LBB2_30:
	movl	$19, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt@GOTPCREL(%rip)
.Lfunc_end2:
	.size	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core11apply_match, .Lfunc_end2-_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core11apply_match
	.cfi_endproc

	.section	.text._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core15decompress_fast,"ax",@progbits
	.type	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core15decompress_fast,@function
_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core15decompress_fast:
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
	movq	%r9, -136(%rbp)
	movl	%ecx, -84(%rbp)
	movq	%rsi, %r11
	movq	%rdx, %rsi
	movq	(%r8), %r13
	movl	8(%r8), %ebx
	movq	16(%rdx), %r14
	movq	24(%rdx), %rax
	movq	8(%r11), %r15
	movq	%rax, -112(%rbp)
	subq	%r14, %rax
	cmpq	$259, %rax
	setb	%al
	cmpq	$14, %r15
	setb	%cl
	orb	%al, %cl
	movl	12(%r8), %r9d
	movb	$12, %dl
	movb	20(%r8), %r10b
	je	.LBB3_3
	movl	16(%r8), %edi
.LBB3_2:
	xorl	%eax, %eax
	jmp	.LBB3_51
.LBB3_3:
	movq	%rdi, %r12
	movq	%r11, -72(%rbp)
	movq	(%r11), %rax
	movq	%rax, -48(%rbp)
	movq	(%rsi), %rax
	movq	%rax, -80(%rbp)
	movq	8(%rsi), %rax
	movq	%rax, -56(%rbp)
	movq	%rsi, -120(%rbp)
	movq	-112(%rbp), %r11
	movq	%r8, -96(%rbp)
.LBB3_4:
	cmpl	$29, %ebx
	ja	.LBB3_6
	movq	-48(%rbp), %rdx
	movl	(%rdx), %eax
	addq	$-4, %r15
	addq	$4, %rdx
	movq	-72(%rbp), %rcx
	movq	%rdx, -48(%rbp)
	movq	%rdx, (%rcx)
	movq	%r15, 8(%rcx)
	shlxq	%rbx, %rax, %rax
	orq	%rax, %r13
	orl	$32, %ebx
.LBB3_6:
	movl	%r13d, %eax
	andl	$1023, %eax
	movswl	512(%r12,%rax,2), %ecx
	testl	%ecx, %ecx
	js	.LBB3_8
	movl	%ecx, %eax
	shrl	$9, %eax
	jmp	.LBB3_13
.LBB3_8:
	movb	$10, %al
.LBB3_9:
	notl	%ecx
	movzbl	%al, %edx
	btq	%rdx, %r13
	adcl	$0, %ecx
	cmpl	$576, %ecx
	jae	.LBB3_11
	movl	%ecx, %ecx
	movswl	2560(%r12,%rcx,2), %ecx
	incb	%al
	testl	%ecx, %ecx
	js	.LBB3_9
	jmp	.LBB3_12
.LBB3_11:
	incb	%al
	movl	$32767, %ecx
.LBB3_12:
	movzbl	%al, %eax
.LBB3_13:
	shrxq	%rax, %r13, %r13
	subl	%eax, %ebx
	btl	$8, %ecx
	jb	.LBB3_27
	movl	%r13d, %eax
	andl	$1023, %eax
	movswl	512(%r12,%rax,2), %eax
	testl	%eax, %eax
	js	.LBB3_16
	movl	%eax, %edx
	shrl	$9, %edx
	jmp	.LBB3_21
.LBB3_16:
	movb	$10, %dl
.LBB3_17:
	notl	%eax
	movzbl	%dl, %esi
	btq	%rsi, %r13
	adcl	$0, %eax
	cmpl	$576, %eax
	jae	.LBB3_19
	movl	%eax, %eax
	movswl	2560(%r12,%rax,2), %eax
	incb	%dl
	testl	%eax, %eax
	js	.LBB3_17
	jmp	.LBB3_20
.LBB3_19:
	incb	%dl
	movl	$32767, %eax
.LBB3_20:
	movzbl	%dl, %edx
	movq	-120(%rbp), %rsi
.LBB3_21:
	cmpq	-56(%rbp), %r14
	jae	.LBB3_58
	shrxq	%rdx, %r13, %r13
	subl	%edx, %ebx
	movq	-80(%rbp), %rdx
	movb	%cl, (%rdx,%r14)
	leaq	1(%r14), %rdi
	movq	%rdi, 16(%rsi)
	btl	$8, %eax
	jb	.LBB3_26
	cmpq	-56(%rbp), %rdi
	jae	.LBB3_59
	movq	-80(%rbp), %rdx
	movb	%al, 1(%rdx,%r14)
	addq	$2, %r14
	movq	%r14, 16(%rsi)
	movq	%r11, %rdx
	subq	%r14, %rdx
	xorl	%eax, %eax
	cmpq	$259, %rdx
	jb	.LBB3_50
	cmpq	$14, %r15
	jae	.LBB3_4
	jmp	.LBB3_50
.LBB3_26:
	movq	%rdi, %r14
	movl	%eax, %ecx
.LBB3_27:
	movl	%ecx, %edi
	andl	$511, %edi
	cmpl	$256, %edi
	movq	-72(%rbp), %rsi
	je	.LBB3_52
	movb	$-1, %al
	cmpl	$285, %edi
	ja	.LBB3_53
	decl	%ecx
	andl	$31, %ecx
	cmpl	$29, %ebx
	ja	.LBB3_31
	movq	-48(%rbp), %rdi
	movl	(%rdi), %edx
	addq	$-4, %r15
	addq	$4, %rdi
	movq	%rdi, -48(%rbp)
	movq	%rdi, (%rsi)
	movq	%r15, 8(%rsi)
	shlxq	%rbx, %rdx, %rdx
	orq	%rdx, %r13
	orl	$32, %ebx
.LBB3_31:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.18(%rip), %rdx
	movb	(%rcx,%rdx), %r10b
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.19(%rip), %rdx
	movzwl	(%rdx,%rcx,2), %edi
	addl	$-28, %ecx
	cmpl	$-20, %ecx
	jb	.LBB3_33
	movzbl	%r10b, %ecx
	movl	%ecx, %edx
	andl	$63, %edx
	bzhil	%edx, %r13d, %edx
	shrxq	%rcx, %r13, %r13
	subl	%ecx, %ebx
	addl	%edx, %edi
.LBB3_33:
	movl	%r13d, %ecx
	andl	$1023, %ecx
	movswl	3712(%r12,%rcx,2), %ecx
	testl	%ecx, %ecx
	js	.LBB3_35
	movl	%ecx, %esi
	shrl	$9, %esi
	jmp	.LBB3_40
.LBB3_35:
	movb	$10, %dl
.LBB3_36:
	notl	%ecx
	movzbl	%dl, %esi
	btq	%rsi, %r13
	adcl	$0, %ecx
	cmpl	$576, %ecx
	jae	.LBB3_38
	movl	%ecx, %ecx
	movswl	5760(%r12,%rcx,2), %ecx
	incb	%dl
	testl	%ecx, %ecx
	js	.LBB3_36
	jmp	.LBB3_39
.LBB3_38:
	incb	%dl
	movl	$32767, %ecx
.LBB3_39:
	movzbl	%dl, %esi
.LBB3_40:
	movl	%ecx, %edx
	andl	$511, %edx
	shrxq	%rsi, %r13, %r13
	subl	%esi, %ebx
	cmpl	$29, %edx
	ja	.LBB3_54
	movl	%ecx, %esi
	shrb	%sil
	subb	$1, %sil
	movzbl	%sil, %r10d
	movl	$0, %esi
	cmovbl	%esi, %r10d
	movl	%edx, %edx
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.7(%rip), %rsi
	movzwl	(%rsi,%rdx,2), %r9d
	cmpb	$4, %cl
	jb	.LBB3_45
	cmpl	$30, %ebx
	jae	.LBB3_44
	movq	-48(%rbp), %rsi
	movl	(%rsi), %ecx
	addq	$-4, %r15
	addq	$4, %rsi
	movq	-72(%rbp), %rdx
	movq	%rsi, -48(%rbp)
	movq	%rsi, (%rdx)
	movq	%r15, 8(%rdx)
	shlxq	%rbx, %rcx, %rcx
	orq	%rcx, %r13
	orl	$32, %ebx
.LBB3_44:
	bzhil	%r10d, %r13d, %ecx
	shrxq	%r10, %r13, %r13
	movzbl	%r10b, %edx
	subl	%edx, %ebx
	addl	%ecx, %r9d
.LBB3_45:
	movl	-84(%rbp), %ecx
	movl	%ecx, %edx
	shrb	$2, %dl
	movl	%r9d, %ecx
	cmpq	%rcx, %r14
	setb	%sil
	testb	%sil, %dl
	jne	.LBB3_55
	movq	-56(%rbp), %rsi
	cmpq	%rcx, %rsi
	jb	.LBB3_55
	movq	%r10, -104(%rbp)
	movl	%r9d, -64(%rbp)
	movl	%edi, -60(%rbp)
	movl	%edi, %r8d
	movq	%r8, -128(%rbp)
	movq	-80(%rbp), %rdi
	movq	%r14, %rdx
	movq	-136(%rbp), %r9
	callq	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core11apply_match
	movq	-112(%rbp), %r11
	addq	-128(%rbp), %r14
	movq	-120(%rbp), %rsi
	movq	%r14, 16(%rsi)
	movq	%r11, %rcx
	subq	%r14, %rcx
	xorl	%eax, %eax
	cmpq	$259, %rcx
	jb	.LBB3_56
	cmpq	$13, %r15
	movq	-96(%rbp), %r8
	movl	-64(%rbp), %r9d
	movq	-104(%rbp), %r10
	ja	.LBB3_4
	movb	$12, %dl
	jmp	.LBB3_57
.LBB3_50:
	movl	%ecx, %edi
	movb	$12, %dl
.LBB3_51:
	movq	%r13, (%r8)
	movl	%ebx, 8(%r8)
	movl	%r9d, 12(%r8)
	movl	%edi, 16(%r8)
	movb	%r10b, 20(%r8)
	addq	$104, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB3_52:
	.cfi_def_cfa %rbp, 16
	movl	$256, %edi
	movb	$20, %dl
	jmp	.LBB3_2
.LBB3_53:
	movb	$33, %dl
	jmp	.LBB3_51
.LBB3_54:
	movb	$34, %dl
	jmp	.LBB3_51
.LBB3_55:
	movb	$30, %dl
	jmp	.LBB3_51
.LBB3_56:
	movq	-96(%rbp), %r8
	movb	$12, %dl
	movl	-64(%rbp), %r9d
	movq	-104(%rbp), %r10
.LBB3_57:
	movl	-60(%rbp), %edi
	jmp	.LBB3_51
.LBB3_58:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10(%rip), %rdx
	movq	%r14, %rdi
	jmp	.LBB3_60
.LBB3_59:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10(%rip), %rdx
.LBB3_60:
	movq	-56(%rbp), %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.Lfunc_end3:
	.size	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core15decompress_fast, .Lfunc_end3-_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core15decompress_fast
	.cfi_endproc

	.section	.rodata,"a",@progbits
.LCPI4_0:
	.byte	8
.LCPI4_1:
	.byte	9
.LCPI4_2:
	.byte	7
.LCPI4_3:
	.byte	5
	.section	.rodata.cst16,"aM",@progbits,16
	.p2align	4, 0x0
.LCPI4_4:
	.long	0
	.long	0
	.long	1
	.long	1
	.section	.rodata.cst4,"aM",@progbits,4
.LCPI4_5:
	.byte	0
	.byte	0
	.byte	1
	.byte	1
	.section	.text._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit,"ax",@progbits
	.globl	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit
	.type	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit,@function
_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit:
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
	subq	$216, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r15
	movq	%rdi, %rbx
	movl	32(%rbp), %r14d
	movq	16(%rbp), %rdi
	movl	%r14d, %esi
	andl	$4, %esi
	xorl	%r10d, %r10d
	movq	%r9, %rax
	subq	$1, %rax
	cmovbq	%r10, %rax
	negl	%esi
	sbbq	%r10, %r10
	cmpq	%r9, %rdi
	ja	.LBB4_246
	orq	%rax, %r10
	leaq	1(%r10), %rsi
	movq	%rsi, %rax
	andq	%r10, %rax
	jne	.LBB4_246
	movq	%rsi, -232(%rbp)
	movq	%r10, -160(%rbp)
	movq	24(%rbp), %rax
	movq	%rdx, -240(%rbp)
	movq	%rdx, -80(%rbp)
	addq	%rdi, %rax
	movq	$-1, %rsi
	cmovaeq	%rax, %rsi
	movq	%rcx, -136(%rbp)
	movq	%rcx, -72(%rbp)
	cmpq	%r9, %rsi
	cmovaeq	%r9, %rsi
	movb	10500(%r15), %dl
	movq	%r8, -120(%rbp)
	movq	%r9, -112(%rbp)
	movq	%rdi, -104(%rbp)
	movq	%rsi, -96(%rbp)
	movq	10432(%r15), %rax
	movl	10440(%r15), %ecx
	movb	10480(%r15), %sil
	movq	%rax, -64(%rbp)
	movl	%ecx, -56(%rbp)
	movq	10460(%r15), %rax
	movq	%rax, -52(%rbp)
	movb	%sil, -44(%rbp)
	testb	$2, %r14b
	movl	$252, %eax
	movl	$1, %r12d
	cmovel	%eax, %r12d
	testb	$1, %r14b
	sete	%al
	leaq	10112(%r15), %rcx
	movq	%rcx, -176(%rbp)
	leaq	10400(%r15), %rcx
	movq	%rcx, -168(%rbp)
	leaq	10481(%r15), %rcx
	movq	%rcx, -184(%rbp)
	leaq	10256(%r15), %rcx
	movq	%rcx, -224(%rbp)
	leaq	10368(%r15), %rcx
	movq	%rcx, -216(%rbp)
	addb	%al, %al
	incb	%al
	movb	%al, -85(%rbp)
	movq	%r15, -144(%rbp)
	movq	%rbx, -128(%rbp)
.LBB4_3:
	movb	$-1, %r13b
	cmpb	$24, %dl
	ja	.LBB4_266
	movzbl	%dl, %eax
	leaq	.LJTI4_0(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB4_5:
	vxorps	%xmm0, %xmm0, %xmm0
	vmovaps	%xmm0, -64(%rbp)
	movq	$0, -51(%rbp)
	vpmovsxbd	.LCPI4_5(%rip), %xmm0
	vmovdqu	%xmm0, 10444(%r15)
	movb	-85(%rbp), %al
	movl	%eax, %edx
	jmp	.LBB4_3
.LBB4_6:
	movl	-48(%rbp), %edi
	movzwl	10476(%r15), %eax
	cmpl	%eax, %edi
	jae	.LBB4_13
	movl	-56(%rbp), %ecx
	movq	-64(%rbp), %rax
	cmpq	$2, %rcx
	ja	.LBB4_10
	movq	-72(%rbp), %rdx
	testq	%rdx, %rdx
	je	.LBB4_15
	movq	-80(%rbp), %rsi
	decq	%rdx
	leaq	1(%rsi), %r8
	movq	%r8, -80(%rbp)
	movq	%rdx, -72(%rbp)
	movzbl	(%rsi), %edx
	shlxq	%rcx, %rdx, %rdx
	orq	%rdx, %rax
	addl	$8, %ecx
.LBB4_10:
	movq	%rax, %rdx
	shrq	$3, %rdx
	movq	%rdx, -64(%rbp)
	addl	$-3, %ecx
	movl	%ecx, -56(%rbp)
	cmpl	$19, %edi
	jae	.LBB4_300
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.4(%rip), %rcx
	movzbl	(%rdi,%rcx), %ecx
	andb	$7, %al
	movq	-184(%rbp), %rdx
	movb	%al, (%rdx,%rcx)
	leal	1(%rdi), %eax
	movl	%eax, -48(%rbp)
	xorl	%eax, %eax
.LBB4_12:
	movl	%r12d, %edx
	jmp	.LBB4_14
.LBB4_13:
	movw	$19, 10476(%r15)
	leaq	-64(%rbp), %rsi
	movq	%r15, %rdi
	callq	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree
	cmpb	$-1, %al
	je	.LBB4_256
.LBB4_14:
	testb	%al, %al
	je	.LBB4_6
	jmp	.LBB4_178
.LBB4_15:
	movb	$2, %al
	jmp	.LBB4_12
.LBB4_16:
	movl	-48(%rbp), %ecx
	movzwl	10472(%r15), %eax
	movzwl	10474(%r15), %edx
	addl	%eax, %edx
	cmpl	%edx, %ecx
	jae	.LBB4_29
	movl	-56(%rbp), %eax
	cmpq	$15, %rax
	jae	.LBB4_34
	movq	-72(%rbp), %rsi
	cmpq	$1, %rsi
	ja	.LBB4_35
	movq	-64(%rbp), %rdx
	movq	-80(%rbp), %r8
	testq	%rsi, %rsi
	sete	%r9b
	movq	%rax, %rsi
.LBB4_20:
	movl	%edx, %edi
	andl	$1023, %edi
	movswl	6912(%r15,%rdi,2), %edi
	testl	%edi, %edi
	js	.LBB4_22
	shrl	$9, %edi
	decl	%edi
	cmpq	%rdi, %rsi
	jbe	.LBB4_27
	jmp	.LBB4_36
.LBB4_22:
	cmpq	$11, %rsi
	jb	.LBB4_27
	movl	$12, %r10d
.LBB4_24:
	leal	-2(%r10), %r11d
	notl	%edi
	btq	%r11, %rdx
	adcl	$0, %edi
	cmpl	$575, %edi
	ja	.LBB4_302
	movswl	8960(%r15,%rdi,2), %edi
	testl	%edi, %edi
	jns	.LBB4_36
	movl	%r10d, %r11d
	incl	%r10d
	cmpq	%r11, %rsi
	jae	.LBB4_24
.LBB4_27:
	testb	$1, %r9b
	jne	.LBB4_50
	leaq	1(%r8), %rdi
	movq	%rdi, -80(%rbp)
	movq	$0, -72(%rbp)
	movzbl	(%r8), %r8d
	shlxq	%rsi, %r8, %r8
	orq	%r8, %rdx
	movq	%rdx, -64(%rbp)
	addl	$8, %eax
	movb	$1, %r9b
	movq	%rdi, %r8
	cmpq	$6, %rsi
	leaq	8(%rsi), %rsi
	movl	%esi, -56(%rbp)
	jbe	.LBB4_20
	jmp	.LBB4_37
.LBB4_29:
	cmpl	%edx, %ecx
	jne	.LBB4_219
	cmpl	$289, %eax
	jae	.LBB4_291
	movq	-176(%rbp), %rdi
	movq	%r15, %rsi
	movq	%rax, %rdx
	callq	*memcpy@GOTPCREL(%rip)
	movzwl	10472(%r15), %esi
	movzwl	10474(%r15), %edx
	leal	(%rdx,%rsi), %eax
	andl	$511, %esi
	andl	$511, %eax
	cmpw	%si, %ax
	jb	.LBB4_292
	andl	$31, %edx
	subq	%rsi, %rax
	cmpq	%rdx, %rax
	jne	.LBB4_294
	addq	%r15, %rsi
	movq	-168(%rbp), %rdi
	callq	*memcpy@GOTPCREL(%rip)
	decb	10479(%r15)
	leaq	-64(%rbp), %rsi
	movq	%r15, %rdi
	callq	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree
	cmpb	$-1, %al
	jne	.LBB4_49
	jmp	.LBB4_277
.LBB4_34:
	movq	-64(%rbp), %rdx
	jmp	.LBB4_37
.LBB4_35:
	movq	-80(%rbp), %rdx
	movzwl	(%rdx), %edi
	addq	$-2, %rsi
	addq	$2, %rdx
	movq	%rdx, -80(%rbp)
	movq	%rsi, -72(%rbp)
	shlxq	%rax, %rdi, %rdx
	orq	-64(%rbp), %rdx
	addl	$16, %eax
	jmp	.LBB4_37
.LBB4_36:
	movl	%esi, %eax
.LBB4_37:
	movl	%edx, %esi
	andl	$1023, %esi
	movswl	6912(%r15,%rsi,2), %esi
	testl	%esi, %esi
	js	.LBB4_39
	movl	%esi, %edi
	shrl	$9, %edi
	andl	$511, %esi
	jmp	.LBB4_44
.LBB4_39:
	movb	$10, %dil
.LBB4_40:
	notl	%esi
	movzbl	%dil, %r8d
	btq	%r8, %rdx
	adcl	$0, %esi
	cmpl	$576, %esi
	jae	.LBB4_42
	movl	%esi, %esi
	movswl	8960(%r15,%rsi,2), %esi
	incb	%dil
	testl	%esi, %esi
	js	.LBB4_40
	jmp	.LBB4_43
.LBB4_42:
	incb	%dil
	movl	$32767, %esi
.LBB4_43:
	movzbl	%dil, %edi
.LBB4_44:
	shrxq	%rdi, %rdx, %rdx
	movq	%rdx, -64(%rbp)
	subl	%edi, %eax
	movl	%eax, -56(%rbp)
	movl	%esi, -52(%rbp)
	cmpl	$16, %esi
	jae	.LBB4_46
	movl	%ecx, %eax
	andl	$511, %eax
	movb	%sil, (%r15,%rax)
	incl	%ecx
	movl	%ecx, -48(%rbp)
	xorl	%eax, %eax
	jmp	.LBB4_49
.LBB4_46:
	movb	$1, %al
	testq	%rcx, %rcx
	jne	.LBB4_48
	movb	$32, %dl
	cmpl	$16, %esi
	je	.LBB4_49
.LBB4_48:
	movl	$459522, -208(%rbp)
	andl	$3, %esi
	movb	-208(%rbp,%rsi), %cl
	movb	%cl, -44(%rbp)
	movb	$11, %dl
.LBB4_49:
	testb	%al, %al
	je	.LBB4_16
	jmp	.LBB4_179
.LBB4_50:
	movb	$2, %al
	movl	%r12d, %edx
	jmp	.LBB4_49
.LBB4_51:
	movzbl	-44(%rbp), %eax
	movl	-56(%rbp), %ecx
	cmpl	%eax, %ecx
	jae	.LBB4_180
	movq	-72(%rbp), %r8
	testq	%r8, %r8
	je	.LBB4_255
	movq	-80(%rbp), %rsi
	movq	-64(%rbp), %rdx
	incq	%rsi
	movl	$1, %edi
	subq	%r8, %rdi
.LBB4_54:
	movq	%rdx, %r8
	movzbl	-1(%rsi), %edx
	shlxq	%rcx, %rdx, %rdx
	orq	%r8, %rdx
	leal	8(%rcx), %r8d
	cmpl	%eax, %r8d
	jae	.LBB4_216
	incq	%rsi
	incq	%rdi
	addl	$8, %ecx
	cmpq	$1, %rdi
	jne	.LBB4_54
	jmp	.LBB4_254
.LBB4_56:
	movzbl	-44(%rbp), %ecx
	movl	-56(%rbp), %eax
	movq	-64(%rbp), %rdx
	cmpl	%ecx, %eax
	jae	.LBB4_61
	movq	-80(%rbp), %rsi
	movq	-72(%rbp), %rdi
.LBB4_58:
	testq	%rdi, %rdi
	je	.LBB4_253
	decq	%rdi
	movzbl	(%rsi), %r8d
	incq	%rsi
	shlxq	%rax, %r8, %r8
	orq	%r8, %rdx
	movq	%rdx, -64(%rbp)
	addl	$8, %eax
	movl	%eax, -56(%rbp)
	cmpl	%ecx, %eax
	jb	.LBB4_58
	movq	%rsi, -80(%rbp)
	movq	%rdi, -72(%rbp)
.LBB4_61:
	movl	%ecx, %esi
	andl	$63, %esi
	bzhil	%esi, %edx, %esi
	shrxq	%rcx, %rdx, %rdx
	movq	%rdx, -64(%rbp)
	subl	%ecx, %eax
	movl	%eax, -56(%rbp)
	addl	%esi, -48(%rbp)
	movb	$15, %dl
	jmp	.LBB4_3
.LBB4_62:
	movzbl	-44(%rbp), %eax
	movl	-56(%rbp), %edx
	movq	-64(%rbp), %rcx
	cmpl	%eax, %edx
	jae	.LBB4_66
	movq	-80(%rbp), %rsi
	movq	-72(%rbp), %rdi
	decq	%rdi
	incq	%rsi
.LBB4_64:
	cmpq	$-1, %rdi
	je	.LBB4_252
	movq	%rsi, -80(%rbp)
	movq	%rdi, -72(%rbp)
	movzbl	-1(%rsi), %r8d
	shlxq	%rdx, %r8, %r8
	orq	%r8, %rcx
	movq	%rcx, -64(%rbp)
	addl	$8, %edx
	movl	%edx, -56(%rbp)
	decq	%rdi
	incq	%rsi
	cmpl	%eax, %edx
	jb	.LBB4_64
.LBB4_66:
	movl	%r12d, -84(%rbp)
	movq	%rbx, %r12
	shrxq	%rax, %rcx, %rsi
	movq	%rsi, -64(%rbp)
	subl	%eax, %edx
	movl	%edx, -56(%rbp)
	movq	$3, -208(%rbp)
	movq	$11, -192(%rbp)
	movl	-52(%rbp), %edx
	movl	-48(%rbp), %ebx
	movl	%r14d, %r13d
	cmpl	$16, %edx
	jne	.LBB4_183
	leal	-1(%rbx), %esi
	andl	$511, %esi
	movb	(%r15,%rsi), %r8b
	jmp	.LBB4_184
.LBB4_68:
	movl	-56(%rbp), %eax
	movl	%eax, %ecx
	andb	$7, %cl
	shrq	%cl, -64(%rbp)
	andl	$-8, %eax
	movl	%eax, -56(%rbp)
	movl	$0, -48(%rbp)
	movb	$5, %dl
	jmp	.LBB4_3
.LBB4_69:
	movq	-104(%rbp), %rdi
	movq	-72(%rbp), %rcx
	cmpq	$4, %rcx
	jb	.LBB4_72
	movq	-96(%rbp), %rax
	movq	-80(%rbp), %rdx
	movq	%rax, -152(%rbp)
	subq	%rdi, %rax
	cmpq	$2, %rax
	jae	.LBB4_190
.LBB4_71:
	movq	%rdx, -80(%rbp)
	movq	%rcx, -72(%rbp)
.LBB4_72:
	movq	%rdi, -104(%rbp)
	movl	-56(%rbp), %eax
	cmpq	$15, %rax
	jae	.LBB4_182
	cmpq	$1, %rcx
	ja	.LBB4_218
	movq	-64(%rbp), %rdx
	movq	-80(%rbp), %rsi
	testq	%rcx, %rcx
	sete	%r8b
	movl	%eax, %ecx
.LBB4_75:
	movl	%edx, %edi
	andl	$1023, %edi
	movswl	512(%r15,%rdi,2), %edi
	testl	%edi, %edi
	js	.LBB4_77
	shrl	$9, %edi
	decl	%edi
	cmpq	%rdi, %rax
	jbe	.LBB4_82
	jmp	.LBB4_221
.LBB4_77:
	cmpq	$11, %rax
	jb	.LBB4_82
	movl	$12, %r9d
.LBB4_79:
	leal	-2(%r9), %r10d
	notl	%edi
	btq	%r10, %rdx
	adcl	$0, %edi
	cmpl	$575, %edi
	ja	.LBB4_301
	movswl	2560(%r15,%rdi,2), %edi
	testl	%edi, %edi
	jns	.LBB4_221
	movl	%r9d, %r10d
	incl	%r9d
	cmpq	%r10, %rax
	jae	.LBB4_79
.LBB4_82:
	testb	$1, %r8b
	jne	.LBB4_279
	movq	$0, -72(%rbp)
	movzbl	(%rsi), %edi
	incq	%rsi
	shlxq	%rax, %rdi, %rdi
	orq	%rdi, %rdx
	movq	%rdx, -64(%rbp)
	addl	$8, %ecx
	movb	$1, %r8b
	cmpq	$6, %rax
	leaq	8(%rax), %rax
	movl	%eax, -56(%rbp)
	jbe	.LBB4_75
	movq	%rsi, -80(%rbp)
	movl	%ecx, %eax
	jmp	.LBB4_222
.LBB4_85:
	movq	-72(%rbp), %rax
	testq	%rax, %rax
	je	.LBB4_283
	movq	-80(%rbp), %rcx
	decq	%rax
	leaq	1(%rcx), %rdx
	movq	%rdx, -80(%rbp)
	movq	%rax, -72(%rbp)
	movzbl	(%rcx), %ecx
	movl	10444(%r15), %eax
	movl	%eax, %edx
	shll	$8, %edx
	orl	%ecx, %edx
	movabsq	$595056260559142912, %rsi
	mulxq	%rsi, %rsi, %rsi
	movl	%esi, %edi
	shll	$5, %edi
	subl	%edi, %esi
	addl	%edx, %esi
	movl	%ecx, 10448(%r15)
	andl	$32, %ecx
	orl	%esi, %ecx
	movl	%eax, %edx
	andl	$15, %edx
	xorl	$8, %edx
	orl	%ecx, %edx
	setne	%cl
	shrl	$4, %eax
	addl	$8, %eax
	andl	$63, %eax
	testb	$4, %r14b
	jne	.LBB4_88
	shrxq	%rax, -232(%rbp), %rdx
	testq	%rdx, %rdx
	sete	%dl
	orb	%dl, %cl
.LBB4_88:
	testb	%cl, %cl
	movl	$29, %ecx
	movl	$3, %edx
	cmovnel	%ecx, %edx
	cmpl	$16, %eax
	cmovael	%ecx, %edx
	jmp	.LBB4_3
.LBB4_89:
	movl	-56(%rbp), %eax
	cmpq	$15, %rax
	jae	.LBB4_181
	movq	-72(%rbp), %rdx
	cmpq	$1, %rdx
	ja	.LBB4_215
	movq	-64(%rbp), %rcx
	movq	-80(%rbp), %rsi
	testq	%rdx, %rdx
	sete	%r9b
	movq	%rax, %r8
.LBB4_92:
	movl	%ecx, %edi
	andl	$1023, %edi
	movswl	3712(%r15,%rdi,2), %edi
	testl	%edi, %edi
	js	.LBB4_94
	shrl	$9, %edi
	decl	%edi
	cmpq	%rdi, %r8
	jbe	.LBB4_99
	jmp	.LBB4_235
.LBB4_94:
	cmpq	$11, %r8
	jb	.LBB4_99
	movl	$12, %r10d
.LBB4_96:
	leal	-2(%r10), %r11d
	notl	%edi
	btq	%r11, %rcx
	adcl	$0, %edi
	cmpl	$575, %edi
	ja	.LBB4_302
	movswl	5760(%r15,%rdi,2), %edi
	testl	%edi, %edi
	jns	.LBB4_220
	movl	%r10d, %r11d
	incl	%r10d
	cmpq	%r11, %r8
	jae	.LBB4_96
.LBB4_99:
	testb	$1, %r9b
	jne	.LBB4_278
	movzbl	(%rsi), %edx
	incq	%rsi
	shlxq	%r8, %rdx, %rdx
	orq	%rdx, %rcx
	movq	%rcx, -64(%rbp)
	addl	$8, %eax
	movb	$1, %r9b
	xorl	%edx, %edx
	cmpq	$6, %r8
	leaq	8(%r8), %r8
	movl	%r8d, -56(%rbp)
	jbe	.LBB4_92
	jmp	.LBB4_236
.LBB4_101:
	movq	-80(%rbp), %r14
	movq	-72(%rbp), %rbx
.LBB4_102:
	movl	-56(%rbp), %edx
	movq	-64(%rbp), %rax
	cmpq	$2, %rdx
	ja	.LBB4_105
	testq	%rbx, %rbx
	je	.LBB4_286
	decq	%rbx
	leaq	1(%r14), %rcx
	movq	%rcx, -80(%rbp)
	movq	%rbx, -72(%rbp)
	movzbl	(%r14), %esi
	shlxq	%rdx, %rsi, %rsi
	orq	%rax, %rsi
	leal	8(%rdx), %eax
	movq	%rsi, %rdx
	shrq	$3, %rdx
	movq	%rdx, -64(%rbp)
	addl	$-3, %eax
	movl	%eax, -56(%rbp)
	movl	%esi, %eax
	andb	$1, %al
	movb	%al, 10478(%r15)
	shrb	%sil
	andb	$3, %sil
	movb	%sil, 10479(%r15)
	movzbl	%sil, %eax
	leaq	.LJTI4_1(%rip), %rdx
	movslq	(%rdx,%rax,4), %rax
	addq	%rdx, %rax
	movq	%rcx, %r14
	jmpq	*%rax
.LBB4_105:
	movq	%rax, %rcx
	shrq	$3, %rcx
	movq	%rcx, -64(%rbp)
	leal	-3(%rdx), %ecx
	movl	%ecx, -56(%rbp)
	movl	%eax, %ecx
	andb	$1, %cl
	movb	%cl, 10478(%r15)
	shrb	%al
	andb	$3, %al
	movb	%al, 10479(%r15)
	movzbl	%al, %eax
	leaq	.LJTI4_1(%rip), %rcx
	movslq	(%rcx,%rax,4), %rax
	addq	%rcx, %rax
	jmpq	*%rax
.LBB4_106:
	movl	$2097440, 10472(%r15)
	vpbroadcastb	.LCPI4_0(%rip), %zmm0
	movq	-176(%rbp), %rax
	vmovdqu64	%zmm0, 80(%rax)
	vmovdqu64	%zmm0, 64(%rax)
	vmovdqu64	%zmm0, (%rax)
	vpbroadcastb	.LCPI4_1(%rip), %zmm0
	movq	-224(%rbp), %rax
	vmovdqu64	%zmm0, 48(%rax)
	vmovdqu64	%zmm0, (%rax)
	movabsq	$506381209866536711, %rax
	movq	-216(%rbp), %rcx
	movq	%rax, 16(%rcx)
	vpbroadcastb	.LCPI4_2(%rip), %xmm0
	vmovdqu	%xmm0, (%rcx)
	movabsq	$578721382704613384, %rax
	movq	%rax, 10392(%r15)
	vpbroadcastb	.LCPI4_3(%rip), %ymm0
	movq	-168(%rbp), %rax
	vmovdqu	%ymm0, (%rax)
	leaq	-64(%rbp), %rsi
	movq	%r15, %rdi
	vzeroupper
	callq	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree
	testb	%al, %al
	je	.LBB4_102
	movl	%eax, %r13d
	movzbl	%al, %eax
	cmpl	$1, %eax
	je	.LBB4_189
	jmp	.LBB4_290
.LBB4_108:
	movq	-72(%rbp), %r15
	testq	%r15, %r15
	je	.LBB4_281
	movl	%r12d, -84(%rbp)
	movq	-96(%rbp), %r13
	movq	-112(%rbp), %rdx
	movq	-104(%rbp), %rdi
	subq	%rdi, %r13
	cmpq	%r15, %r13
	cmovaeq	%r15, %r13
	movl	-48(%rbp), %r12d
	cmpq	%r12, %r13
	cmovaeq	%r12, %r13
	movq	%r13, %rbx
	addq	%rdi, %rbx
	jb	.LBB4_296
	cmpq	%rdx, %rbx
	ja	.LBB4_296
	movq	-80(%rbp), %r14
	addq	-120(%rbp), %rdi
	movq	%r14, %rsi
	movq	%r13, %rdx
	callq	*memcpy@GOTPCREL(%rip)
	movq	%rbx, -104(%rbp)
	subq	%r13, %r15
	addq	%r13, %r14
	movq	%r14, -80(%rbp)
	movq	%r15, -72(%rbp)
	subl	%r13d, %r12d
	movl	%r12d, -48(%rbp)
	movb	$6, %dl
	jmp	.LBB4_166
.LBB4_112:
	movq	-72(%rbp), %rax
	testq	%rax, %rax
	je	.LBB4_282
	movq	-80(%rbp), %rcx
	decq	%rax
	leaq	1(%rcx), %rdx
	movq	%rdx, -80(%rbp)
	movq	%rax, -72(%rbp)
	movzbl	(%rcx), %eax
	movl	%eax, 10444(%r15)
	movb	$2, %dl
	jmp	.LBB4_3
.LBB4_114:
	movl	-56(%rbp), %eax
	movq	-64(%rbp), %rcx
	cmpq	$7, %rax
	ja	.LBB4_117
	movq	-72(%rbp), %rdx
	testq	%rdx, %rdx
	je	.LBB4_288
	movq	-80(%rbp), %rsi
	decq	%rdx
	leaq	1(%rsi), %rdi
	movq	%rdi, -80(%rbp)
	movq	%rdx, -72(%rbp)
	movzbl	(%rsi), %edx
	shlxq	%rax, %rdx, %rdx
	orq	%rdx, %rcx
	addl	$8, %eax
.LBB4_117:
	movq	%rcx, %rdx
	shrq	$8, %rdx
	movq	%rdx, -64(%rbp)
	addl	$-8, %eax
	movl	%eax, -56(%rbp)
	movzbl	%cl, %eax
	movl	%eax, -52(%rbp)
	movb	$18, %dl
	jmp	.LBB4_3
.LBB4_118:
	movb	$3, %dl
	cmpb	$0, 10478(%r15)
	je	.LBB4_3
	movl	-56(%rbp), %ecx
	movq	-64(%rbp), %rdx
	movl	%ecx, %eax
	movq	-136(%rbp), %r9
	movq	%r9, %rdi
	subq	-72(%rbp), %rdi
	andl	$-8, %eax
	movl	%ecx, %esi
	shrl	$3, %esi
	cmpl	%edi, %esi
	cmovael	%edi, %esi
	leal	(,%rsi,8), %r8d
	subl	%r8d, %eax
	movl	%eax, -56(%rbp)
	subq	%rsi, %rdi
	movq	%r9, %rsi
	subq	%rdi, %rsi
	jb	.LBB4_299
	andb	$7, %cl
	shrxq	%rcx, %rdx, %rcx
	addq	-240(%rbp), %rdi
	andl	$56, %eax
	bzhiq	%rax, %rcx, %rax
	movq	%rdi, -80(%rbp)
	movq	%rsi, -72(%rbp)
	movq	%rax, -64(%rbp)
	movb	$24, %dl
	testb	$1, %r14b
	je	.LBB4_3
	movl	$0, -48(%rbp)
	movb	$23, %dl
	jmp	.LBB4_3
.LBB4_122:
	movl	-56(%rbp), %eax
	movl	-48(%rbp), %ecx
	cmpq	$3, %rcx
	ja	.LBB4_132
	movq	-80(%rbp), %rdi
	movq	-72(%rbp), %rdx
	movq	-64(%rbp), %rsi
.LBB4_124:
	testl	%eax, %eax
	je	.LBB4_129
	cmpl	$7, %eax
	ja	.LBB4_128
	testq	%rdx, %rdx
	je	.LBB4_257
	decq	%rdx
	leaq	1(%rdi), %r8
	movq	%r8, -80(%rbp)
	movq	%rdx, -72(%rbp)
	movzbl	(%rdi), %edi
	shlxq	%rax, %rdi, %rdi
	orq	%rdi, %rsi
	orl	$8, %eax
	movq	%r8, %rdi
.LBB4_128:
	movb	%sil, 10468(%r15,%rcx)
	shrq	$8, %rsi
	movq	%rsi, -64(%rbp)
	addl	$-8, %eax
	movl	%eax, -56(%rbp)
	leal	1(%rcx), %r9d
	leal	1(%rcx), %r8d
	jmp	.LBB4_131
.LBB4_129:
	testq	%rdx, %rdx
	je	.LBB4_257
	decq	%rdx
	leaq	1(%rdi), %r9
	movq	%r9, -80(%rbp)
	movq	%rdx, -72(%rbp)
	movb	(%rdi), %al
	movb	%al, 10468(%r15,%rcx)
	leal	1(%rcx), %r8d
	xorl	%eax, %eax
	movq	%r9, %rdi
	movl	%r8d, %r9d
.LBB4_131:
	movl	%r8d, -48(%rbp)
	incq	%rcx
	cmpl	$4, %r9d
	jb	.LBB4_124
.LBB4_132:
	movzwl	10468(%r15), %ecx
	movzwl	10470(%r15), %esi
	xorw	%cx, %si
	movl	%ecx, -48(%rbp)
	movb	$31, %dl
	cmpw	$-1, %si
	jne	.LBB4_3
	movb	$20, %dl
	testw	%cx, %cx
	je	.LBB4_3
	testl	%eax, %eax
	movl	$6, %eax
	movl	$17, %edx
	cmovel	%eax, %edx
	jmp	.LBB4_3
.LBB4_135:
	movl	-48(%rbp), %eax
	movl	%eax, %ecx
	andl	$511, %ecx
	movl	%ecx, -48(%rbp)
	movb	$20, %dl
	cmpl	$256, %ecx
	je	.LBB4_3
	movb	$33, %dl
	cmpl	$285, %ecx
	ja	.LBB4_3
	decl	%eax
	andl	$31, %eax
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.18(%rip), %rcx
	movb	(%rax,%rcx), %cl
	movb	%cl, -44(%rbp)
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.19(%rip), %rcx
	movzwl	(%rcx,%rax,2), %ecx
	movl	%ecx, -48(%rbp)
	addl	$-28, %eax
	cmpl	$-20, %eax
	setb	%dl
	orb	$14, %dl
	jmp	.LBB4_3
.LBB4_138:
	movl	-48(%rbp), %eax
	movb	$21, %dl
	cmpl	$255, %eax
	ja	.LBB4_3
	movq	-104(%rbp), %rdi
	cmpq	%rdi, -96(%rbp)
	je	.LBB4_289
	movq	-112(%rbp), %rsi
	cmpq	%rsi, %rdi
	jae	.LBB4_303
	movq	-120(%rbp), %rcx
	movb	%al, (%rcx,%rdi)
	incq	%rdi
	movq	%rdi, -104(%rbp)
	movb	$12, %dl
	jmp	.LBB4_3
.LBB4_142:
	movb	$20, %dl
	cmpl	$0, -48(%rbp)
	je	.LBB4_3
	movq	-96(%rbp), %rax
	movb	$7, %dl
	cmpq	-104(%rbp), %rax
	jne	.LBB4_3
	jmp	.LBB4_287
.LBB4_144:
	movq	-104(%rbp), %rdi
	cmpq	%rdi, -96(%rbp)
	je	.LBB4_284
	movq	-112(%rbp), %rsi
	cmpq	%rsi, %rdi
	jae	.LBB4_303
	movl	-52(%rbp), %eax
	movq	-120(%rbp), %rcx
	movb	%al, (%rcx,%rdi)
	incq	%rdi
	cmpl	$0, -56(%rbp)
	movl	$6, %eax
	movl	$17, %edx
	cmovel	%eax, %edx
	decl	-48(%rbp)
	movq	%rdi, -104(%rbp)
	cmovel	%eax, %edx
	jmp	.LBB4_3
.LBB4_147:
	movl	-48(%rbp), %eax
	cmpq	$2, %rax
	ja	.LBB4_156
	movq	-80(%rbp), %rcx
	movq	-72(%rbp), %rsi
	movq	-64(%rbp), %r8
	movl	-56(%rbp), %edi
.LBB4_149:
	movabsq	$21474836485, %rdx
	movq	%rdx, -208(%rbp)
	movl	$4, -200(%rbp)
	movl	-208(%rbp,%rax,4), %r9d
	cmpl	%r9d, %edi
	jae	.LBB4_155
	movb	$8, %dl
	testq	%rsi, %rsi
	je	.LBB4_259
	movl	%r14d, %r13d
	movq	%rbx, %r14
	xorl	%r10d, %r10d
.LBB4_152:
	movq	%r8, %r11
	movzbl	(%rcx,%r10), %r8d
	shlxq	%rdi, %r8, %r8
	orq	%r11, %r8
	leal	8(%rdi), %r11d
	cmpl	%r9d, %r11d
	jae	.LBB4_154
	incq	%r10
	movq	%rsi, %rbx
	movl	%r11d, %edi
	subq	%r10, %rbx
	jne	.LBB4_152
	jmp	.LBB4_249
.LBB4_154:
	addq	%r10, %rcx
	incq	%rcx
	movq	%rcx, -80(%rbp)
	notq	%r10
	addq	%r10, %rsi
	movq	%rsi, -72(%rbp)
	movq	%r8, -64(%rbp)
	addl	$8, %edi
	movl	%edi, -56(%rbp)
	movq	%r14, %rbx
	movl	%r13d, %r14d
.LBB4_155:
	shrxq	%r9, %r8, %rdx
	movq	%rdx, -64(%rbp)
	subl	%r9d, %edi
	movl	%edi, -56(%rbp)
	andl	$63, %r9d
	bzhil	%r9d, %r8d, %r8d
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.3(%rip), %r9
	addw	(%r9,%rax,2), %r8w
	movw	%r8w, 10472(%r15,%rax,2)
	incq	%rax
	movl	%eax, -48(%rbp)
	movq	%rdx, %r8
	cmpl	$3, %eax
	jne	.LBB4_149
.LBB4_156:
	xorl	%eax, %eax
	movq	-184(%rbp), %rcx
	movl	%eax, 15(%rcx)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%xmm0, (%rcx)
	movl	%eax, -48(%rbp)
	cmpw	$31, 10474(%r15)
	movl	$9, %edx
	movl	$27, %eax
	cmovael	%eax, %edx
	cmpw	$287, 10472(%r15)
	cmovael	%eax, %edx
	jmp	.LBB4_3
.LBB4_157:
	testb	$4, %r14b
	sete	%dl
	movl	-52(%rbp), %ecx
	movq	-104(%rbp), %rax
	movq	%rax, %r9
	subq	%rcx, %r9
	setae	%sil
	orb	%dl, %sil
	movb	$30, %dl
	je	.LBB4_3
	movq	-120(%rbp), %rdi
	movq	-112(%rbp), %rsi
	cmpq	%rcx, %rsi
	jb	.LBB4_3
	movl	%r14d, %r13d
	movq	%rbx, %r14
	movq	-96(%rbp), %rdx
	subq	%rax, %rdx
	movl	-48(%rbp), %r8d
	leaq	(%rax,%r8), %rbx
	cmpq	%rdx, %rbx
	ja	.LBB4_161
	andq	-160(%rbp), %r9
	subq	%rax, %r9
	setae	%dl
	cmpq	%r8, %r9
	setb	%r9b
	andb	%dl, %r9b
	cmpb	$1, %r9b
	jne	.LBB4_232
.LBB4_161:
	testq	%r8, %r8
	movl	$12, %eax
	movl	$19, %edx
	cmovel	%eax, %edx
	jmp	.LBB4_233
.LBB4_162:
	movl	%r12d, -84(%rbp)
	movq	-96(%rbp), %rax
	movq	%rax, -152(%rbp)
	movq	-104(%rbp), %r13
	movl	-52(%rbp), %eax
	movq	%rax, -248(%rbp)
	movl	-48(%rbp), %r12d
	movq	-120(%rbp), %r14
	movq	-112(%rbp), %rbx
.LBB4_163:
	movq	-152(%rbp), %rax
	subq	%r13, %rax
	je	.LBB4_247
	movq	%r13, %rdx
	subq	-248(%rbp), %rdx
	movq	-160(%rbp), %r9
	andq	%r9, %rdx
	movl	%r12d, %r15d
	cmpq	%r15, %rax
	cmovbq	%rax, %r15
	movq	%r14, %rdi
	movq	%rbx, %rsi
	movq	%r13, %rcx
	movq	%r15, %r8
	callq	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer
	addq	%r15, %r13
	subl	%r15d, %r12d
	movl	%r12d, -48(%rbp)
	jne	.LBB4_163
	movq	%r13, -104(%rbp)
	movb	$12, %dl
.LBB4_166:
	movq	-128(%rbp), %rbx
	movq	-144(%rbp), %r15
	movl	32(%rbp), %r14d
	movl	-84(%rbp), %r12d
	jmp	.LBB4_3
.LBB4_167:
	movl	-48(%rbp), %edx
	movq	-80(%rbp), %rax
	movq	-72(%rbp), %rcx
	cmpl	$3, %edx
	ja	.LBB4_177
	movl	10452(%r15), %esi
	movq	-64(%rbp), %rdi
	movl	-56(%rbp), %r8d
	decl	%edx
.LBB4_169:
	testl	%r8d, %r8d
	je	.LBB4_174
	cmpl	$7, %r8d
	ja	.LBB4_173
	testq	%rcx, %rcx
	je	.LBB4_258
	decq	%rcx
	movzbl	(%rax), %r9d
	incq	%rax
	shlxq	%r8, %r9, %r9
	orq	%r9, %rdi
	orl	$8, %r8d
.LBB4_173:
	movzbl	%dil, %r9d
	shrq	$8, %rdi
	movq	%rdi, -64(%rbp)
	addl	$-8, %r8d
	movl	%r8d, -56(%rbp)
	shll	$8, %esi
	orl	%r9d, %esi
	jmp	.LBB4_176
.LBB4_174:
	testq	%rcx, %rcx
	je	.LBB4_258
	decq	%rcx
	movzbl	(%rax), %r8d
	incq	%rax
	shll	$8, %esi
	orl	%r8d, %esi
	xorl	%r8d, %r8d
.LBB4_176:
	movl	%esi, 10452(%r15)
	leal	2(%rdx), %r9d
	movl	%r9d, -48(%rbp)
	incl	%edx
	cmpl	$3, %edx
	jb	.LBB4_169
.LBB4_177:
	movq	%rax, -80(%rbp)
	movq	%rcx, -72(%rbp)
	movb	$24, %dl
	jmp	.LBB4_3
.LBB4_178:
	movzbl	%al, %eax
	cmpl	$1, %eax
	je	.LBB4_3
	jmp	.LBB4_250
.LBB4_179:
	movzbl	%al, %eax
	cmpl	$2, %eax
	jne	.LBB4_3
	jmp	.LBB4_251
.LBB4_180:
	movq	-64(%rbp), %rdx
	jmp	.LBB4_217
.LBB4_181:
	movq	-64(%rbp), %rcx
	jmp	.LBB4_237
.LBB4_182:
	movq	-64(%rbp), %rdx
	jmp	.LBB4_222
.LBB4_183:
	xorl	%r8d, %r8d
.LBB4_184:
	andl	$63, %eax
	bzhiq	%rax, %rcx, %r14
	andl	$2, %edx
	addq	-208(%rbp,%rdx,8), %r14
	leal	(%r14,%rbx), %esi
	movl	%ebx, %edi
	andl	$511, %edi
	andl	$511, %esi
	movq	%rsi, %rdx
	subq	%rdi, %rdx
	jb	.LBB4_298
	addq	%r15, %rdi
	movzbl	%r8b, %esi
	callq	*memset@GOTPCREL(%rip)
	addl	%ebx, %r14d
	movl	%r14d, -48(%rbp)
	movb	$10, %dl
	movq	%r12, %rbx
	movl	-84(%rbp), %r12d
	movl	%r13d, %r14d
	jmp	.LBB4_3
.LBB4_186:
	movl	$0, -48(%rbp)
	movb	$8, %dl
	jmp	.LBB4_189
.LBB4_187:
	movb	$25, %dl
	jmp	.LBB4_189
.LBB4_188:
	movb	$4, %dl
.LBB4_189:
	movq	-128(%rbp), %rbx
	movl	32(%rbp), %r14d
	jmp	.LBB4_3
.LBB4_190:
	movq	-120(%rbp), %r9
	movq	-112(%rbp), %rsi
.LBB4_191:
	cmpq	$14, %rcx
	jb	.LBB4_193
	cmpq	$258, %rax
	ja	.LBB4_230
.LBB4_193:
	movl	-56(%rbp), %eax
	movq	-64(%rbp), %r11
	cmpq	$29, %rax
	ja	.LBB4_195
	movl	(%rdx), %r10d
	addq	$-4, %rcx
	addq	$4, %rdx
	shlxq	%rax, %r10, %r10
	orq	%r10, %r11
	addl	$32, %eax
.LBB4_195:
	movl	%r14d, %r8d
	movq	%rbx, %r13
	movl	%r11d, %r10d
	andl	$1023, %r10d
	movswl	512(%r15,%r10,2), %r10d
	testl	%r10d, %r10d
	js	.LBB4_197
	movl	%r10d, %r14d
	shrl	$9, %r14d
	jmp	.LBB4_202
.LBB4_197:
	movb	$10, %bl
.LBB4_198:
	notl	%r10d
	movzbl	%bl, %r14d
	btq	%r14, %r11
	adcl	$0, %r10d
	cmpl	$576, %r10d
	jae	.LBB4_200
	movl	%r10d, %r10d
	movswl	2560(%r15,%r10,2), %r10d
	incb	%bl
	testl	%r10d, %r10d
	js	.LBB4_198
	jmp	.LBB4_201
.LBB4_200:
	incb	%bl
	movl	$32767, %r10d
.LBB4_201:
	movzbl	%bl, %r14d
.LBB4_202:
	movl	%r10d, -48(%rbp)
	shrxq	%r14, %r11, %rbx
	movq	%rbx, -64(%rbp)
	subl	%r14d, %eax
	movl	%eax, -56(%rbp)
	btl	$8, %r10d
	jb	.LBB4_231
	movl	%ebx, %r11d
	andl	$1023, %r11d
	movswl	512(%r15,%r11,2), %r11d
	testl	%r11d, %r11d
	js	.LBB4_205
	movl	%r11d, %r14d
	shrl	$9, %r14d
	jmp	.LBB4_210
.LBB4_205:
	movb	$10, %r14b
.LBB4_206:
	notl	%r11d
	movzbl	%r14b, %r15d
	btq	%r15, %rbx
	adcl	$0, %r11d
	cmpl	$576, %r11d
	jae	.LBB4_208
	movl	%r11d, %r11d
	movq	-144(%rbp), %r15
	movswl	2560(%r15,%r11,2), %r11d
	incb	%r14b
	testl	%r11d, %r11d
	js	.LBB4_206
	jmp	.LBB4_209
.LBB4_208:
	incb	%r14b
	movl	$32767, %r11d
	movq	-144(%rbp), %r15
.LBB4_209:
	movzbl	%r14b, %r14d
.LBB4_210:
	shrxq	%r14, %rbx, %rbx
	movq	%rbx, -64(%rbp)
	subl	%r14d, %eax
	movl	%eax, -56(%rbp)
	cmpq	%rsi, %rdi
	jae	.LBB4_303
	movb	%r10b, (%r9,%rdi)
	leaq	1(%rdi), %rax
	btl	$8, %r11d
	movq	%r13, %rbx
	movl	%r8d, %r14d
	jb	.LBB4_234
	cmpq	%rsi, %rax
	jae	.LBB4_304
	movb	%r11b, 1(%r9,%rdi)
	addq	$2, %rdi
	cmpq	$4, %rcx
	jb	.LBB4_71
	movq	-152(%rbp), %rax
	subq	%rdi, %rax
	cmpq	$2, %rax
	jae	.LBB4_191
	jmp	.LBB4_71
.LBB4_215:
	movq	-80(%rbp), %rcx
	movzwl	(%rcx), %esi
	addq	$-2, %rdx
	addq	$2, %rcx
	movq	%rcx, -80(%rbp)
	movq	%rdx, -72(%rbp)
	shlxq	%rax, %rsi, %rcx
	orq	-64(%rbp), %rcx
	addl	$16, %eax
	jmp	.LBB4_237
.LBB4_216:
	movq	%rsi, -80(%rbp)
	negq	%rdi
	movq	%rdi, -72(%rbp)
	movl	%r8d, %ecx
.LBB4_217:
	movl	%eax, %esi
	andl	$63, %esi
	bzhil	%esi, %edx, %esi
	shrxq	%rax, %rdx, %rdx
	movq	%rdx, -64(%rbp)
	subl	%eax, %ecx
	movl	%ecx, -56(%rbp)
	addl	%esi, -52(%rbp)
	movb	$22, %dl
	jmp	.LBB4_3
.LBB4_218:
	movq	-80(%rbp), %rdx
	movzwl	(%rdx), %esi
	addq	$-2, %rcx
	addq	$2, %rdx
	movq	%rdx, -80(%rbp)
	movq	%rcx, -72(%rbp)
	shlxq	%rax, %rsi, %rdx
	orq	-64(%rbp), %rdx
	addl	$16, %eax
	jmp	.LBB4_222
.LBB4_219:
	movb	$26, %dl
	jmp	.LBB4_3
.LBB4_220:
	movq	%rsi, -80(%rbp)
	movq	%rdx, -72(%rbp)
	movl	%r8d, %eax
	jmp	.LBB4_237
.LBB4_221:
	movq	%rsi, -80(%rbp)
.LBB4_222:
	movl	%edx, %ecx
	andl	$1023, %ecx
	movswl	512(%r15,%rcx,2), %ecx
	testl	%ecx, %ecx
	js	.LBB4_224
	movl	%ecx, %esi
	shrl	$9, %esi
	andl	$511, %ecx
	jmp	.LBB4_229
.LBB4_224:
	movb	$10, %sil
.LBB4_225:
	notl	%ecx
	movzbl	%sil, %edi
	btq	%rdi, %rdx
	adcl	$0, %ecx
	cmpl	$576, %ecx
	jae	.LBB4_227
	movl	%ecx, %ecx
	movswl	2560(%r15,%rcx,2), %ecx
	incb	%sil
	testl	%ecx, %ecx
	js	.LBB4_225
	jmp	.LBB4_228
.LBB4_227:
	incb	%sil
	movl	$32767, %ecx
.LBB4_228:
	movzbl	%sil, %esi
.LBB4_229:
	shrxq	%rsi, %rdx, %rdx
	movq	%rdx, -64(%rbp)
	subl	%esi, %eax
	movl	%eax, -56(%rbp)
	movl	%ecx, -48(%rbp)
	movb	$13, %dl
	jmp	.LBB4_3
.LBB4_230:
	leaq	-80(%rbp), %rsi
	movq	%rdx, (%rsi)
	movq	%rcx, 8(%rsi)
	leaq	-120(%rbp), %rdx
	movq	%rdi, 16(%rdx)
	leaq	-64(%rbp), %r8
	movq	%r15, %rdi
	movl	%r14d, %ecx
	movq	-160(%rbp), %r9
	callq	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core15decompress_fast
	testb	%al, %al
	je	.LBB4_3
	jmp	.LBB4_295
.LBB4_231:
	movq	%rdx, -80(%rbp)
	movq	%rcx, -72(%rbp)
	movq	%rdi, -104(%rbp)
	movb	$21, %dl
	movq	%r13, %rbx
	movl	%r8d, %r14d
	jmp	.LBB4_3
.LBB4_232:
	movq	%rax, %rdx
	movq	-160(%rbp), %r9
	callq	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core11apply_match
	movq	%rbx, -104(%rbp)
	movb	$12, %dl
.LBB4_233:
	movq	%r14, %rbx
	movl	%r13d, %r14d
	jmp	.LBB4_3
.LBB4_234:
	movq	%rdx, -80(%rbp)
	movq	%rcx, -72(%rbp)
	movq	%rax, -104(%rbp)
	movl	%r11d, -48(%rbp)
	movb	$21, %dl
	jmp	.LBB4_3
.LBB4_235:
	movl	%r8d, %eax
.LBB4_236:
	movq	%rsi, -80(%rbp)
	movq	%rdx, -72(%rbp)
.LBB4_237:
	movl	%ecx, %edx
	andl	$1023, %edx
	movswl	3712(%r15,%rdx,2), %esi
	testl	%esi, %esi
	js	.LBB4_239
	movl	%esi, %edx
	shrl	$9, %edx
	andl	$511, %esi
	jmp	.LBB4_244
.LBB4_239:
	movb	$10, %dl
.LBB4_240:
	notl	%esi
	movzbl	%dl, %edi
	btq	%rdi, %rcx
	adcl	$0, %esi
	cmpl	$576, %esi
	jae	.LBB4_242
	movl	%esi, %esi
	movswl	5760(%r15,%rsi,2), %esi
	incb	%dl
	testl	%esi, %esi
	js	.LBB4_240
	jmp	.LBB4_243
.LBB4_242:
	incb	%dl
	movl	$32767, %esi
.LBB4_243:
	movzbl	%dl, %edx
.LBB4_244:
	shrxq	%rdx, %rcx, %rcx
	movq	%rcx, -64(%rbp)
	subl	%edx, %eax
	movl	%eax, -56(%rbp)
	movb	$34, %dl
	cmpl	$29, %esi
	ja	.LBB4_3
	movl	%esi, %eax
	movl	%esi, %ecx
	shrb	%cl
	xorl	%edx, %edx
	subb	$1, %cl
	movzbl	%cl, %ecx
	cmovbl	%edx, %ecx
	movb	%cl, -44(%rbp)
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.7(%rip), %rcx
	movzwl	(%rcx,%rax,2), %eax
	movl	%eax, -52(%rbp)
	cmpl	$4, %esi
	movl	$22, %eax
	movl	$16, %edx
	cmovbl	%eax, %edx
	jmp	.LBB4_3
.LBB4_246:
	movb	$-3, 8(%rbx)
	xorl	%eax, %eax
	movq	%rax, (%rbx)
	jmp	.LBB4_276
.LBB4_247:
	movq	-152(%rbp), %rax
	movq	%rax, -104(%rbp)
	movb	$19, %dl
	movb	$2, %r13b
.LBB4_248:
	movq	-128(%rbp), %rbx
	movl	32(%rbp), %r14d
	jmp	.LBB4_266
.LBB4_249:
	addq	%r10, %rcx
	movq	%rcx, -80(%rbp)
	movq	%rbx, -72(%rbp)
	movq	%r8, -64(%rbp)
	movl	%r11d, -56(%rbp)
	movq	%r14, %rbx
	movl	%r13d, %r14d
	jmp	.LBB4_259
.LBB4_250:
	movl	%edx, %r12d
	movb	$9, %dl
	jmp	.LBB4_259
.LBB4_251:
	movl	%edx, %r12d
	movb	$10, %dl
	jmp	.LBB4_259
.LBB4_252:
	movb	$11, %dl
	jmp	.LBB4_259
.LBB4_253:
	movq	%rsi, -80(%rbp)
	movq	$0, -72(%rbp)
	movb	$14, %dl
	jmp	.LBB4_259
.LBB4_254:
	movq	%rdx, -64(%rbp)
	movl	%ecx, -56(%rbp)
.LBB4_255:
	movq	$0, -72(%rbp)
	movb	$16, %dl
	jmp	.LBB4_259
.LBB4_256:
	movb	$9, %dl
	jmp	.LBB4_266
.LBB4_257:
	movb	$5, %dl
	jmp	.LBB4_259
.LBB4_258:
	movq	$0, -72(%rbp)
	movb	$23, %dl
.LBB4_259:
	cmpb	$1, %r12b
	je	.LBB4_262
	movzbl	%r12b, %eax
	cmpl	$252, %eax
	jne	.LBB4_265
	movb	$1, %r15b
	movl	%r14d, %r8d
	jmp	.LBB4_264
.LBB4_262:
	movl	%r14d, %r8d
	movq	-96(%rbp), %rax
	cmpq	-104(%rbp), %rax
	jne	.LBB4_267
	cmpb	$23, %dl
	sete	%al
	movb	$2, %r12b
	subb	%al, %r12b
	movb	$1, %r15b
.LBB4_264:
	xorl	%r14d, %r14d
	jmp	.LBB4_268
.LBB4_265:
	movl	%r12d, %r13d
.LBB4_266:
	testb	$1, %r14b
	sete	%al
	movl	-72(%rbp), %ecx
	movq	-136(%rbp), %rdi
	movl	%edi, %esi
	subl	%ecx, %esi
	movl	-56(%rbp), %ecx
	movl	%r14d, %r8d
	movl	%ecx, %r14d
	shrl	$3, %r14d
	cmpl	%esi, %r14d
	cmovael	%esi, %r14d
	leal	(,%r14,8), %esi
	subl	%esi, %ecx
	movl	%ecx, -56(%rbp)
	testb	%r13b, %r13b
	setne	%r15b
	orb	%al, %r15b
	movl	%r13d, %r12d
	movq	%rdi, %r13
	jmp	.LBB4_269
.LBB4_267:
	movb	$1, %r15b
	xorl	%r14d, %r14d
	movb	$1, %r12b
.LBB4_268:
	movq	-136(%rbp), %r13
.LBB4_269:
	movq	16(%rbp), %rdi
	movq	-144(%rbp), %rsi
	movb	%dl, 10500(%rsi)
	movl	-56(%rbp), %eax
	movl	%eax, 10440(%rsi)
	andl	$63, %eax
	bzhiq	%rax, -64(%rbp), %rax
	movq	-52(%rbp), %rcx
	movq	%rcx, 10460(%rsi)
	movb	-44(%rbp), %cl
	movb	%cl, 10480(%rsi)
	movq	%rax, 10432(%rsi)
	testb	$64, %r8b
	jne	.LBB4_275
	testb	$9, %r8b
	je	.LBB4_275
	testb	%r12b, %r12b
	js	.LBB4_275
	movq	-112(%rbp), %rax
	movq	-104(%rbp), %rsi
	movq	%rsi, %rdx
	subq	%rdi, %rdx
	jb	.LBB4_297
	cmpq	%rax, %rsi
	ja	.LBB4_297
	movl	%r12d, -84(%rbp)
	movq	-144(%rbp), %r12
	movl	10456(%r12), %eax
	movq	-120(%rbp), %rsi
	addq	%rdi, %rsi
	leaq	-208(%rbp), %rbx
	movl	%eax, (%rbx)
	movq	%rbx, %rdi
	callq	*_RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice@GOTPCREL(%rip)
	movq	16(%rbp), %rdi
	movl	(%rbx), %eax
	movq	-128(%rbp), %rbx
	movl	%eax, 10456(%r12)
	cmpl	10452(%r12), %eax
	movzbl	-84(%rbp), %eax
	movl	$254, %r12d
	cmovel	%eax, %r12d
	testb	%r15b, %r15b
	cmovnel	%eax, %r12d
.LBB4_275:
	addq	-72(%rbp), %r14
	subq	%r14, %r13
	movq	-104(%rbp), %rax
	subq	%rdi, %rax
	movb	%r12b, 8(%rbx)
	movq	%r13, (%rbx)
.LBB4_276:
	movq	%rax, 16(%rbx)
	movq	%rbx, %rax
	addq	$216, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB4_277:
	.cfi_def_cfa %rbp, 16
	movb	$10, %dl
	jmp	.LBB4_266
.LBB4_278:
	movq	%rdx, -72(%rbp)
	movb	$15, %dl
	jmp	.LBB4_259
.LBB4_279:
	movq	%rsi, -80(%rbp)
	movb	$12, %dl
	jmp	.LBB4_259
.LBB4_280:
	xorl	%r13d, %r13d
	jmp	.LBB4_266
.LBB4_281:
	movb	$7, %dl
	jmp	.LBB4_259
.LBB4_282:
	movb	$1, %dl
	jmp	.LBB4_259
.LBB4_283:
	movb	$2, %dl
	jmp	.LBB4_259
.LBB4_284:
	movb	$18, %dl
	jmp	.LBB4_285
.LBB4_286:
	movb	$3, %dl
	movq	-128(%rbp), %rbx
	movl	32(%rbp), %r14d
	jmp	.LBB4_259
.LBB4_287:
	movb	$6, %dl
	jmp	.LBB4_285
.LBB4_288:
	movb	$17, %dl
	jmp	.LBB4_259
.LBB4_289:
	movb	$13, %dl
.LBB4_285:
	movb	$2, %r13b
	jmp	.LBB4_266
.LBB4_290:
	movb	$3, %dl
	jmp	.LBB4_248
.LBB4_291:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.22(%rip), %rcx
	movl	$288, %edx
	xorl	%edi, %edi
	jmp	.LBB4_293
.LBB4_292:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.21(%rip), %rcx
	movl	$512, %edx
	movq	%rsi, %rdi
.LBB4_293:
	movq	%rax, %rsi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB4_294:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.20(%rip), %rcx
	movq	%rdx, %rdi
	movq	%rax, %rsi
	movq	%rcx, %rdx
	callq	*_RNvNvNtCs2k2z8Zem4rB_4core5slice20copy_from_slice_impl17len_mismatch_fail@GOTPCREL(%rip)
.LBB4_295:
	movl	%eax, %r12d
	jmp	.LBB4_259
.LBB4_296:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.11(%rip), %rcx
	movq	%rbx, %rsi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB4_297:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.24(%rip), %rcx
	movq	%rax, %rdx
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB4_298:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.6(%rip), %rcx
	movl	$512, %edx
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB4_299:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.23(%rip), %rcx
	movq	-136(%rbp), %rdx
	movq	%rdx, %rsi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB4_300:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.5(%rip), %rdx
	movl	$19, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.LBB4_301:
	movq	%rsi, -80(%rbp)
.LBB4_302:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.2(%rip), %rdx
	movl	$576, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.LBB4_303:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10(%rip), %rdx
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.LBB4_304:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10(%rip), %rdx
	movq	%rax, %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.Lfunc_end4:
	.size	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit, .Lfunc_end4-_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit
	.cfi_endproc
	.section	.rodata._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core21decompress_with_limit,"a",@progbits
	.p2align	2, 0x0
.LJTI4_0:
	.long	.LBB4_5-.LJTI4_0
	.long	.LBB4_112-.LJTI4_0
	.long	.LBB4_85-.LJTI4_0
	.long	.LBB4_101-.LJTI4_0
	.long	.LBB4_68-.LJTI4_0
	.long	.LBB4_122-.LJTI4_0
	.long	.LBB4_142-.LJTI4_0
	.long	.LBB4_108-.LJTI4_0
	.long	.LBB4_147-.LJTI4_0
	.long	.LBB4_6-.LJTI4_0
	.long	.LBB4_16-.LJTI4_0
	.long	.LBB4_62-.LJTI4_0
	.long	.LBB4_69-.LJTI4_0
	.long	.LBB4_138-.LJTI4_0
	.long	.LBB4_56-.LJTI4_0
	.long	.LBB4_89-.LJTI4_0
	.long	.LBB4_51-.LJTI4_0
	.long	.LBB4_114-.LJTI4_0
	.long	.LBB4_144-.LJTI4_0
	.long	.LBB4_162-.LJTI4_0
	.long	.LBB4_118-.LJTI4_0
	.long	.LBB4_135-.LJTI4_0
	.long	.LBB4_157-.LJTI4_0
	.long	.LBB4_167-.LJTI4_0
	.long	.LBB4_280-.LJTI4_0
.LJTI4_1:
	.long	.LBB4_188-.LJTI4_1
	.long	.LBB4_106-.LJTI4_1
	.long	.LBB4_186-.LJTI4_1
	.long	.LBB4_187-.LJTI4_1

	.section	.text._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer,"ax",@progbits
	.type	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer,@function
_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer:
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
	subq	$56, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rcx, %r13
	movq	%rdx, %r14
	movq	%rsi, %rbx
	movq	%rdi, %r15
	movq	%rdx, %rcx
	subq	%r13, %rcx
	movq	%r13, %rax
	subq	%rdx, %rax
	cmovaeq	%rax, %rcx
	cmpq	$-1, %r9
	sete	%dl
	leaq	(%r14,%r8), %rsi
	addq	$-3, %rsi
	cmpq	%rbx, %rsi
	setb	%sil
	orb	%dl, %sil
	movl	%r8d, %edx
	andl	$-4, %edx
	leaq	(%rdx,%r13), %r12
	cmpq	%r13, %r14
	jae	.LBB5_7
	testb	%sil, %sil
	je	.LBB5_7
	cmpq	$1, %rcx
	jne	.LBB5_7
	leaq	-1(%r13), %rdi
	cmpq	%rbx, %rdi
	jae	.LBB5_56
	cmpq	%r13, %r12
	jb	.LBB5_47
	cmpq	%rbx, %r12
	ja	.LBB5_47
	leaq	(%r15,%r13), %rdi
	movzbl	-1(%r15,%r13), %esi
	movq	%r9, %r14
	movq	%r8, %r13
	callq	*memset@GOTPCREL(%rip)
	movq	%r14, %r9
	leaq	-1(%r12), %r14
	andl	$3, %r13d
	leaq	.LJTI5_0(%rip), %rax
	movslq	(%rax,%r13,4), %rcx
	addq	%rax, %rcx
	jmpq	*%rcx
.LBB5_7:
	cmpq	%r13, %r14
	setb	%cl
	andb	%sil, %cl
	cmpq	$4, %rax
	setae	%al
	xorl	%r10d, %r10d
	movq	%rbx, %rdx
	subq	$3, %rdx
	cmovaeq	%rdx, %r10
	cmpq	%r10, %r12
	cmovbq	%r12, %r10
	testb	%al, %cl
	je	.LBB5_13
	cmpq	%r10, %r13
	jae	.LBB5_24
	movq	%r8, -56(%rbp)
	movq	%r9, -64(%rbp)
	movq	%r15, -48(%rbp)
.LBB5_10:
	movq	%r10, %r12
	movq	%r14, %r15
	addq	$3, %r14
	movq	%r15, -88(%rbp)
	movq	%r14, -80(%rbp)
	movb	$0, -72(%rbp)
	leaq	-88(%rbp), %rdi
	movq	%rbx, %rsi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.50(%rip), %rdx
	callq	*_RINvNtNtCs2k2z8Zem4rB_4core5slice5index5rangeINtNtNtB6_3ops5range14RangeInclusivejEECs9jwHs5ahBUL_11miniz_oxide@GOTPCREL(%rip)
	subq	%rax, %rdx
	movq	%rbx, %rcx
	subq	%rdx, %rcx
	cmpq	%rcx, %r13
	ja	.LBB5_44
	movq	-48(%rbp), %rcx
	addq	%rcx, %rax
	leaq	(%rcx,%r13), %rdi
	movq	%rax, %rsi
	callq	*memmove@GOTPCREL(%rip)
	addq	$4, %r13
	incq	%r14
	movq	%r12, %r10
	cmpq	%r12, %r13
	jb	.LBB5_10
	addq	$4, %r15
	movq	%r13, %r12
	movq	%r15, %r14
	movq	-48(%rbp), %r15
	movq	-64(%rbp), %r9
	movq	-56(%rbp), %r8
	jmp	.LBB5_26
.LBB5_13:
	cmpq	%r10, %r13
	jae	.LBB5_24
	leaq	3(%r13), %rax
	cmpq	%rbx, %rax
	jae	.LBB5_22
	addq	%r15, %rax
	xorl	%ecx, %ecx
.LBB5_16:
	leaq	(%r14,%rcx), %rdx
	addq	$3, %rdx
	andq	%r9, %rdx
	cmpq	%rbx, %rdx
	jae	.LBB5_45
	leaq	(%r14,%rcx), %rdi
	andq	%r9, %rdi
	cmpq	%rbx, %rdi
	jae	.LBB5_55
	movb	(%r15,%rdi), %sil
	movb	%sil, -3(%rax,%rcx)
	leaq	(%r14,%rcx), %rdi
	incq	%rdi
	andq	%r9, %rdi
	cmpq	%rbx, %rdi
	jae	.LBB5_54
	movb	(%r15,%rdi), %sil
	movb	%sil, -2(%rax,%rcx)
	leaq	(%r14,%rcx), %rdi
	addq	$2, %rdi
	andq	%r9, %rdi
	cmpq	%rbx, %rdi
	jae	.LBB5_52
	movb	(%r15,%rdi), %sil
	movb	%sil, -1(%rax,%rcx)
	movb	(%r15,%rdx), %dl
	movb	%dl, (%rax,%rcx)
	leaq	(%rcx,%r13), %r12
	addq	$4, %r12
	cmpq	%r10, %r12
	jae	.LBB5_25
	leaq	(%rcx,%r13), %rdx
	addq	$7, %rdx
	addq	$4, %rcx
	cmpq	%rbx, %rdx
	jb	.LBB5_16
.LBB5_22:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.27(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.28(%rip), %rdx
.LBB5_23:
	movl	$47, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip)
.LBB5_24:
	movq	%r13, %r12
	jmp	.LBB5_26
.LBB5_25:
	addq	%rcx, %r14
	addq	$4, %r14
.LBB5_26:
	andl	$3, %r8d
	leaq	.LJTI5_0(%rip), %rax
	movslq	(%rax,%r8,4), %rcx
	addq	%rax, %rcx
	jmpq	*%rcx
.LBB5_27:
	andq	%r9, %r14
	cmpq	%rbx, %r14
	jae	.LBB5_57
	cmpq	%rbx, %r12
	jb	.LBB5_42
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.35(%rip), %rdx
	jmp	.LBB5_64
.LBB5_30:
	leaq	1(%r12), %rax
	cmpq	%rbx, %rax
	jae	.LBB5_48
	leaq	1(%r14), %rcx
	andq	%r9, %rcx
	cmpq	%rbx, %rcx
	jae	.LBB5_50
	andq	%r9, %r14
	cmpq	%rbx, %r14
	jae	.LBB5_58
	cmpq	%rbx, %r12
	jb	.LBB5_41
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.41(%rip), %rdx
	jmp	.LBB5_64
.LBB5_35:
	leaq	2(%r12), %rax
	cmpq	%rbx, %rax
	jae	.LBB5_49
	leaq	2(%r14), %rcx
	andq	%r9, %rcx
	cmpq	%rbx, %rcx
	jae	.LBB5_51
	movq	%r14, %rdi
	andq	%r9, %rdi
	cmpq	%rbx, %rdi
	jae	.LBB5_59
	cmpq	%rbx, %r12
	jae	.LBB5_60
	movb	(%r15,%rdi), %dl
	movb	%dl, (%r15,%r12)
	incq	%r14
	andq	%r9, %r14
	cmpq	%rbx, %r14
	jae	.LBB5_61
	incq	%r12
	cmpq	%rbx, %r12
	jae	.LBB5_63
.LBB5_41:
	movb	(%r15,%r14), %dl
	movb	%dl, (%r15,%r12)
	movq	%rcx, %r14
	movq	%rax, %r12
.LBB5_42:
	movb	(%r15,%r14), %al
	movb	%al, (%r15,%r12)
.LBB5_43:
	addq	$56, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB5_44:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.0(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.50(%rip), %rdx
	movl	$43, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt@GOTPCREL(%rip)
.LBB5_45:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.29(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.30(%rip), %rdx
.LBB5_46:
	movl	$72, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip)
.LBB5_47:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.26(%rip), %rcx
	movq	%r13, %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.LBB5_48:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.36(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.37(%rip), %rdx
	jmp	.LBB5_23
.LBB5_49:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.42(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.43(%rip), %rdx
	jmp	.LBB5_23
.LBB5_50:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.38(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.39(%rip), %rdx
	jmp	.LBB5_46
.LBB5_51:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.44(%rip), %rdi
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.45(%rip), %rdx
	jmp	.LBB5_46
.LBB5_52:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.33(%rip), %rdx
	jmp	.LBB5_53
.LBB5_54:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.32(%rip), %rdx
	jmp	.LBB5_53
.LBB5_55:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.31(%rip), %rdx
	jmp	.LBB5_53
.LBB5_56:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.25(%rip), %rdx
	jmp	.LBB5_53
.LBB5_57:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.34(%rip), %rdx
	jmp	.LBB5_62
.LBB5_58:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.40(%rip), %rdx
	jmp	.LBB5_62
.LBB5_59:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.46(%rip), %rdx
	jmp	.LBB5_53
.LBB5_60:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.47(%rip), %rdx
	jmp	.LBB5_64
.LBB5_61:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.48(%rip), %rdx
.LBB5_62:
	movq	%r14, %rdi
	jmp	.LBB5_53
.LBB5_63:
	leaq	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.49(%rip), %rdx
.LBB5_64:
	movq	%r12, %rdi
.LBB5_53:
	movq	%rbx, %rsi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.Lfunc_end5:
	.size	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer, .Lfunc_end5-_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer
	.cfi_endproc
	.section	.rodata._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core8transfer,"a",@progbits
	.p2align	2, 0x0
.LJTI5_0:
	.long	.LBB5_43-.LJTI5_0
	.long	.LBB5_27-.LJTI5_0
	.long	.LBB5_30-.LJTI5_0
	.long	.LBB5_35-.LJTI5_0

	.section	.rodata,"a",@progbits
	.p2align	1, 0x0
.LCPI6_0:
	.short	798
	.section	.rodata.cst32,"aM",@progbits,32
	.p2align	5, 0x0
.LCPI6_1:
	.quad	0
	.quad	16
	.quad	0
	.quad	1
	.section	.rodata.cst16,"aM",@progbits,16
	.p2align	4, 0x0
.LCPI6_2:
	.byte	1
	.byte	2
	.byte	4
	.byte	8
	.byte	16
	.byte	32
	.byte	64
	.byte	128
	.byte	1
	.byte	2
	.byte	4
	.byte	8
	.byte	16
	.byte	32
	.byte	64
	.byte	128
	.section	.rodata.cst4,"aM",@progbits,4
.LCPI6_3:
	.byte	0
	.byte	16
	.byte	0
	.byte	1
	.section	.rodata.cst8,"aM",@progbits,8
	.p2align	3, 0x0
.LCPI6_4:
	.byte	1
	.byte	2
	.byte	4
	.byte	8
	.byte	16
	.byte	32
	.byte	64
	.byte	128
	.section	.text._RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree,"ax",@progbits
	.type	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree,@function
_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree:
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
	subq	$280, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, -56(%rbp)
	movq	%rdi, %r12
	leaq	10481(%rdi), %r11
	leaq	6912(%rdi), %rax
	movq	%rax, -136(%rbp)
	leaq	10400(%rdi), %rax
	movq	%rax, -128(%rbp)
	leaq	10112(%rdi), %rax
	movq	%rax, -120(%rbp)
	leaq	512(%rdi), %rcx
	leaq	-208(%rbp), %r9
	movb	10479(%rdi), %al
	vpbroadcastw	.LCPI6_0(%rip), %zmm1
	movq	memset@GOTPCREL(%rip), %r8
	vpmovsxbq	.LCPI6_3(%rip), %ymm2
	movq	%r11, -112(%rbp)
	movq	%rcx, -104(%rbp)
.LBB6_1:
	movzbl	%al, %edi
	testb	%dil, %dil
	je	.LBB6_2
	cmpl	$2, %edi
	je	.LBB6_9
	cmpl	$1, %edi
	jne	.LBB6_38
	movl	$32, %r14d
	movq	-128(%rbp), %rax
	jmp	.LBB6_6
.LBB6_9:
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%ymm0, -240(%rbp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, -320(%rbp)
	movl	$0, -256(%rbp)
	xorl	%eax, %eax
.LBB6_10:
	vmovdqu64	%zmm1, 6912(%r12,%rax,2)
	addq	$32, %rax
	cmpq	$1024, %rax
	jne	.LBB6_10
	movl	$19, %r14d
	movq	%r11, %r10
	movq	-136(%rbp), %rbx
	jmp	.LBB6_12
.LBB6_2:
	movl	$288, %r14d
	movq	-120(%rbp), %rax
.LBB6_6:
	movq	%rax, -48(%rbp)
	movq	%r9, %r15
	movq	%rdi, %r13
	imulq	$3200, %rdi, %rbx
	addq	%rcx, %rbx
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%ymm0, -240(%rbp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, -320(%rbp)
	movl	$0, -256(%rbp)
	xorl	%eax, %eax
.LBB6_7:
	vmovdqu64	%zmm1, (%rbx,%rax,2)
	addq	$32, %rax
	cmpq	$1024, %rax
	jne	.LBB6_7
	leaq	2048(%rbx), %rdi
	movl	$1152, %edx
	xorl	%esi, %esi
	vzeroupper
	callq	*%r8
	movq	%r15, %r9
	vpmovsxbq	.LCPI6_3(%rip), %ymm2
	movq	-48(%rbp), %r10
	movq	%r13, %rdi
.LBB6_12:
	movzwl	10472(%r12,%rdi,2), %r15d
	movb	$-1, %al
	movb	$28, %r8b
	cmpl	%r15d, %r14d
	jb	.LBB6_40
	testq	%r15, %r15
	vpbroadcastw	.LCPI6_0(%rip), %zmm1
	vpbroadcastq	.LCPI6_4(%rip), %xmm3
	je	.LBB6_17
	xorl	%ecx, %ecx
.LBB6_15:
	movzbl	(%r10,%rcx), %edx
	cmpq	$15, %rdx
	ja	.LBB6_39
	incw	-240(%rbp,%rdx,2)
	incq	%rcx
	cmpq	%rcx, %r15
	jne	.LBB6_15
.LBB6_17:
	movq	%r12, -96(%rbp)
	leaq	-240(%rbp), %rax
	movq	%rax, -200(%rbp)
	movq	%r9, -192(%rbp)
	leaq	-316(%rbp), %rax
	movq	%rax, -184(%rbp)
	leaq	-252(%rbp), %rax
	movq	%rax, -176(%rbp)
	vmovups	%ymm2, -168(%rbp)
	xorl	%r13d, %r13d
	movl	$1, %r14d
	movl	$1, %edx
	xorl	%r12d, %r12d
.LBB6_18:
	testq	%rdx, %rdx
	jne	.LBB6_23
	movq	-168(%rbp), %rcx
	cmpq	-160(%rbp), %rcx
	jae	.LBB6_20
	leaq	1(%rcx), %rax
	movq	%rax, -168(%rbp)
	leaq	(%rcx,%rcx), %rax
	addq	-200(%rbp), %rax
	shlq	$2, %rcx
	addq	-184(%rbp), %rcx
	movq	-152(%rbp), %rdx
	leaq	1(%rdx), %rsi
	movq	%rsi, -152(%rbp)
	movq	%rdx, -80(%rbp)
	movq	%rcx, -64(%rbp)
	jmp	.LBB6_22
.LBB6_20:
	xorl	%eax, %eax
.LBB6_22:
	movq	%rax, -72(%rbp)
.LBB6_24:
	testq	%rax, %rax
	je	.LBB6_27
	movzwl	(%rax), %ecx
	movq	-80(%rbp), %rax
	movq	-64(%rbp), %rdx
	addl	%ecx, %r12d
	addl	%r12d, %r12d
	movl	%r12d, (%rdx)
	addl	%r14d, %r14d
	subl	%ecx, %r14d
	js	.LBB6_60
	testw	%cx, %cx
	cmovel	%r13d, %eax
	movq	-144(%rbp), %rdx
	movl	%eax, %r13d
	jmp	.LBB6_18
.LBB6_23:
	movq	$0, -144(%rbp)
	movq	%rdi, -88(%rbp)
	leaq	-80(%rbp), %rdi
	leaq	-200(%rbp), %rsi
	movq	%r10, -48(%rbp)
	vzeroupper
	callq	_RNvXs_NtNtNtCs2k2z8Zem4rB_4core4iter8adapters9enumerateINtB4_9EnumerateINtNtB6_3zip3ZipINtNtNtBa_5slice4iter4ItertEINtB1q_7IterMutmEEENtNtNtB8_6traits8iterator8Iterator3nthCs9jwHs5ahBUL_11miniz_oxide
	movb	$28, %r8b
	movq	-88(%rbp), %rdi
	movq	-48(%rbp), %r10
	vpbroadcastq	.LCPI6_4(%rip), %xmm3
	vpmovsxbq	.LCPI6_3(%rip), %ymm2
	vpbroadcastw	.LCPI6_0(%rip), %zmm1
	leaq	-208(%rbp), %r9
	movq	-72(%rbp), %rax
	jmp	.LBB6_24
.LBB6_27:
	cmpl	$65536, %r12d
	movl	$1, %r14d
	je	.LBB6_30
	movb	$1, %al
	cmpl	$2, %edi
	je	.LBB6_40
	cmpl	$1, %r13d
	ja	.LBB6_40
.LBB6_30:
	movq	%r9, %r13
	testw	%r15w, %r15w
	movq	-96(%rbp), %r12
	movq	-112(%rbp), %r11
	je	.LBB6_33
	movw	$-1, %dx
	xorl	%eax, %eax
.LBB6_42:
	movq	%rax, %rcx
	incq	%rax
	movzbl	(%r10,%rcx), %edi
	movl	%edi, %r9d
	andl	$15, %r9d
	je	.LBB6_41
	movl	-320(%rbp,%r9,4), %esi
	leal	1(%rsi), %r8d
	movl	%r8d, -320(%rbp,%r9,4)
	bzhil	%r9d, %esi, %esi
	vmovd	%esi, %xmm0
	vgf2p8affineqb	$0, %xmm3, %xmm0, %xmm0
	vmovd	%xmm0, %esi
	rolw	$8, %si
	movzwl	%si, %esi
	movb	$16, %r8b
	subb	%r9b, %r8b
	shrxl	%r8d, %esi, %r8d
	movzwl	%r8w, %esi
	cmpb	$10, %r9b
	ja	.LBB6_47
	cmpl	$1023, %esi
	ja	.LBB6_41
	movzbl	%r9b, %r8d
	shlxl	%r8d, %r14d, %edi
	shll	$9, %r8d
	orl	%r8d, %ecx
.LBB6_46:
	movw	%cx, (%rbx,%rsi,2)
	addq	%rdi, %rsi
	cmpq	$1024, %rsi
	jb	.LBB6_46
.LBB6_41:
	cmpq	%r15, %rax
	je	.LBB6_33
	jmp	.LBB6_42
.LBB6_47:
	movq	%r10, -48(%rbp)
	andl	$1023, %r8d
	movzwl	(%rbx,%r8,2), %r10d
	cmpl	$798, %r10d
	jne	.LBB6_48
	movw	%dx, (%rbx,%r8,2)
	leal	-2(%rdx), %r8d
	movl	%edx, %r10d
	jmp	.LBB6_50
.LBB6_48:
	movl	%edx, %r8d
.LBB6_50:
	shrl	$9, %esi
	cmpb	$11, %r9b
	jne	.LBB6_54
	movl	%r8d, %edx
	movl	%r10d, %r9d
	jmp	.LBB6_52
.LBB6_54:
	andb	$15, %dil
	addb	$-11, %dil
.LBB6_55:
	notl	%r10d
	btl	$1, %esi
	adcw	$0, %r10w
	movzwl	%r10w, %edx
	cmpl	$575, %edx
	ja	.LBB6_53
	movzwl	2048(%rbx,%rdx,2), %r9d
	testw	%r9w, %r9w
	je	.LBB6_58
	movl	%r8d, %edx
	movl	%r9d, %r8d
	jmp	.LBB6_59
.LBB6_58:
	movw	%r8w, 2048(%rbx,%rdx,2)
	leal	-2(%r8), %edx
.LBB6_59:
	movzwl	%si, %esi
	shrl	%esi
	movl	%r8d, %r9d
	movl	%r8d, %r10d
	movl	%edx, %r8d
	decb	%dil
	jne	.LBB6_55
.LBB6_52:
	notl	%r9d
	btl	$1, %esi
	adcw	$0, %r9w
	movzwl	%r9w, %esi
	cmpl	$575, %esi
	ja	.LBB6_53
	movw	%cx, 2048(%rbx,%rsi,2)
	cmpq	%r15, %rax
	movq	-48(%rbp), %r10
	jb	.LBB6_42
.LBB6_33:
	movzbl	10479(%r12), %eax
	testl	%eax, %eax
	je	.LBB6_36
	cmpl	$2, %eax
	je	.LBB6_35
	decb	%al
	movb	%al, 10479(%r12)
	movq	-104(%rbp), %rcx
	movq	memset@GOTPCREL(%rip), %r8
	movq	%r13, %r9
	jmp	.LBB6_1
.LBB6_60:
	movb	$1, %al
	jmp	.LBB6_40
.LBB6_53:
	movb	$10, %r8b
	movb	$-1, %al
	jmp	.LBB6_40
.LBB6_38:
	movb	$-1, %al
.LBB6_39:
.LBB6_40:
	movl	%r8d, %edx
	addq	$280, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB6_36:
	.cfi_def_cfa %rbp, 16
	movq	-56(%rbp), %rax
	movl	$0, 16(%rax)
	movb	$1, %al
	movb	$12, %r8b
	jmp	.LBB6_40
.LBB6_35:
	movq	-56(%rbp), %rax
	movl	$0, 16(%rax)
	movb	$1, %al
	movb	$10, %r8b
	jmp	.LBB6_40
.Lfunc_end6:
	.size	_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree, .Lfunc_end6-_RNvNtNtCs9jwHs5ahBUL_11miniz_oxide7inflate4core9init_tree
	.cfi_endproc

	.section	.text._RNvXs_NtNtNtCs2k2z8Zem4rB_4core4iter8adapters9enumerateINtB4_9EnumerateINtNtB6_3zip3ZipINtNtNtBa_5slice4iter4ItertEINtB1q_7IterMutmEEENtNtNtB8_6traits8iterator8Iterator3nthCs9jwHs5ahBUL_11miniz_oxide,"ax",@progbits
	.type	_RNvXs_NtNtNtCs2k2z8Zem4rB_4core4iter8adapters9enumerateINtB4_9EnumerateINtNtB6_3zip3ZipINtNtNtBa_5slice4iter4ItertEINtB1q_7IterMutmEEENtNtNtB8_6traits8iterator8Iterator3nthCs9jwHs5ahBUL_11miniz_oxide,@function
_RNvXs_NtNtNtCs2k2z8Zem4rB_4core4iter8adapters9enumerateINtB4_9EnumerateINtNtB6_3zip3ZipINtNtNtBa_5slice4iter4ItertEINtB1q_7IterMutmEEENtNtNtB8_6traits8iterator8Iterator3nthCs9jwHs5ahBUL_11miniz_oxide:
	.cfi_startproc
	movq	32(%rsi), %rax
	movq	40(%rsi), %r8
	movq	%r8, %r9
	subq	%rax, %r9
	cmpq	%r9, %rdx
	cmovbq	%rdx, %r9
	leaq	(%r9,%rax), %rcx
	cmpq	%rcx, %rax
	jae	.LBB7_2
	movq	%rcx, 32(%rsi)
	movq	%rcx, %rax
.LBB7_2:
	pushq	%r14
	.cfi_def_cfa_offset 16
	pushq	%rbx
	.cfi_def_cfa_offset 24
	.cfi_offset %rbx, -24
	.cfi_offset %r14, -16
	movl	$8, %ecx
	movq	%rax, %rbx
	subq	%r8, %rbx
	jae	.LBB7_3
	subq	%rdx, %r9
	leaq	(,%rax,4), %r10
	negq	%r10
	leaq	(%rax,%rax), %r11
	negq	%r11
	xorl	%r14d, %r14d
.LBB7_6:
	cmpq	%r14, %r9
	je	.LBB7_7
	decq	%r14
	addq	$-4, %r10
	addq	$-2, %r11
	cmpq	%r14, %rbx
	jne	.LBB7_6
	movq	%r8, 32(%rsi)
.LBB7_3:
	xorl	%eax, %eax
	jmp	.LBB7_8
.LBB7_7:
	subq	%r14, %rax
	incq	%rax
	movq	%rax, 32(%rsi)
	movq	(%rsi), %rcx
	movq	16(%rsi), %rax
	subq	%r11, %rcx
	subq	%r10, %rax
	movq	48(%rsi), %r8
	leaq	(%r8,%rdx), %r9
	addq	%r8, %rdx
	incq	%rdx
	movq	%rdx, 48(%rsi)
	movq	%r9, (%rdi)
	movq	%rcx, 8(%rdi)
	movl	$16, %ecx
.LBB7_8:
	movq	%rax, (%rdi,%rcx)
	popq	%rbx
	.cfi_def_cfa_offset 16
	popq	%r14
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end7:
	.size	_RNvXs_NtNtNtCs2k2z8Zem4rB_4core4iter8adapters9enumerateINtB4_9EnumerateINtNtB6_3zip3ZipINtNtNtBa_5slice4iter4ItertEINtB1q_7IterMutmEEENtNtNtB8_6traits8iterator8Iterator3nthCs9jwHs5ahBUL_11miniz_oxide, .Lfunc_end7-_RNvXs_NtNtNtCs2k2z8Zem4rB_4core4iter8adapters9enumerateINtB4_9EnumerateINtNtB6_3zip3ZipINtNtNtBa_5slice4iter4ItertEINtB1q_7IterMutmEEENtNtNtB8_6traits8iterator8Iterator3nthCs9jwHs5ahBUL_11miniz_oxide
	.cfi_endproc

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.0,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.0,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.0:
	.ascii	"dest is out of bounds"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.0, 21

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1:
	.asciz	"/cargo/registry/25cdd57fae9f0462/miniz_oxide-0.9.1/src/inflate/core.rs"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1, 71

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.2,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.2,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.2:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\237\002\000\000\035\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.2, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.3,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.3,"a",@progbits
	.p2align	1, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.3:
	.asciz	"\001\001\001\000\004"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.3, 6

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.4,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.4,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.4:
	.ascii	"\020\021\022\000\b\007\t\006\n\005\013\004\f\003\r\002\016\001\017"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.4, 19

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.5,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.5,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.5:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000{\006\000\000-\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.5, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.6,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.6,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.6:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\303\006\000\000 \000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.6, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.7,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.7,"a",@progbits
	.p2align	1, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.7:
	.ascii	"\001\000\002\000\003\000\004\000\005\000\007\000\t\000\r\000\021\000\031\000!\0001\000A\000a\000\201\000\301\000\001\001\201\001\001\002\001\003\001\004\001\006\001\b\001\f\001\020\001\030\001 \0010\001@\001`"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.7, 60

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.8,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.8,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.8:
	.ascii	"mid > len"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.8, 9

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.9,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.9:
	.asciz	"/cargo/registry/25cdd57fae9f0462/miniz_oxide-0.9.1/src/inflate/output_buffer.rs"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.9, 80

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.9
	.asciz	"O\000\000\000\000\000\000\000-\000\000\000\t\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.10, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.11,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.11,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.11:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.9
	.asciz	"O\000\000\000\000\000\000\0007\000\000\000\023\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.11, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.12,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.12,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.12:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\264\004\000\0004\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.12, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.13,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.13,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.13:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\265\004\000\000O\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.13, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.14,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.14,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.14:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\265\004\000\000\025\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.14, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.15,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.15,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.15:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\261\004\000\0004\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.15, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.16,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.16,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.16:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\262\004\000\000>\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.16, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.17,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.17,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.17:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\262\004\000\000\025\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.17, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.18,@object
	.section	.rodata.cst32,"aM",@progbits,32
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.18:
	.asciz	"\000\000\000\000\000\000\000\000\001\001\001\001\002\002\002\002\003\003\003\003\004\004\004\004\005\005\005\005\000\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.18, 32

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.19,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.19,"a",@progbits
	.p2align	1, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.19:
	.ascii	"\003\000\004\000\005\000\006\000\007\000\b\000\t\000\n\000\013\000\r\000\017\000\021\000\023\000\027\000\033\000\037\000#\000+\0003\000;\000C\000S\000c\000s\000\203\000\243\000\303\000\343\000\002\001\000\002\000\002\000\002"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.19, 64

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.20,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.20,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.20:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\252\006\000\000\032\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.20, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.21,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.21,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.21:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\252\006\000\0006\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.21, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.22,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.22,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.22:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\235\006\000\000(\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.22, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.23,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.23,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.23:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\262\007\000\000>\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.23, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.24,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.24,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.24:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000\030\b\000\000M\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.24, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.25,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.25,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.25:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000L\004\000\000\024\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.25, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.26,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.26,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.26:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000M\004\000\000\022\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.26, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.27,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.27,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.27:
	.ascii	"assertion failed: out_pos + 3 < out_slice.len()"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.27, 47

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.28,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.28,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.28:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000`\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.28, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.29,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.29,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.29:
	.ascii	"assertion failed: (source_pos + 3) & out_buf_size_mask < out_slice.len()"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.29, 72

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.30,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.30,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.30:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000a\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.30, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.31,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.31,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.31:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000c\004\000\000\"\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.31, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.32,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.32,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.32:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000d\004\000\000&\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.32, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.33,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.33,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.33:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000e\004\000\000&\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.33, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.34,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.34,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.34:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000n\004\000\000#\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.34, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.35,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.35,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.35:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000n\004\000\000\016\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.35, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.36,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.36,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.36:
	.ascii	"assertion failed: out_pos + 1 < out_slice.len()"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.36, 47

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.37,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.37,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.37:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000p\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.37, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.38,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.38,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.38:
	.ascii	"assertion failed: (source_pos + 1) & out_buf_size_mask < out_slice.len()"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.38, 72

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.39,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.39,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.39:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000q\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.39, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.40,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.40,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.40:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000r\004\000\000\"\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.40, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.41,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.41,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.41:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000r\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.41, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.42,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.42,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.42:
	.ascii	"assertion failed: out_pos + 2 < out_slice.len()"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.42, 47

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.43,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.43,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.43:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000v\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.43, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.44,@object
	.section	.rodata..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.44,"a",@progbits
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.44:
	.ascii	"assertion failed: (source_pos + 2) & out_buf_size_mask < out_slice.len()"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.44, 72

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.45,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.45,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.45:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000w\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.45, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.46,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.46,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.46:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000x\004\000\000\"\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.46, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.47,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.47,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.47:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000x\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.47, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.48,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.48,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.48:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000y\004\000\000&\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.48, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.49,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.49,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.49:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000y\004\000\000\r\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.49, 24

	.type	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.50,@object
	.section	.data.rel.ro..Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.50,"aw",@progbits
	.p2align	3, 0x0
.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.50:
	.quad	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.1
	.asciz	"F\000\000\000\000\000\000\000V\004\000\000\027\000\000"
	.size	.Lanon.6a8fa1ed73cc86253dcb3bc4f7245a7a.50, 24

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits

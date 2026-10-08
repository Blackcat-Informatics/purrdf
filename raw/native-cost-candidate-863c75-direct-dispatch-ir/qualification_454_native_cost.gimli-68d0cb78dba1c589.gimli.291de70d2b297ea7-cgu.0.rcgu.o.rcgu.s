	.att_syntax
	.file	"gimli.291de70d2b297ea7-cgu.0"
	.section	.text._RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_,"ax",@progbits
	.globl	_RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_
	.type	_RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_,@function
_RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_:
.Lfunc_begin0:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception0
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%rbx
	pushq	%rax
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r14
	movq	%rdi, %r15
	callq	*_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_@GOTPCREL(%rip)
	movq	%rax, %rbx
	movq	$0, 1232(%rax)
	movw	$0, 1330(%rax)
	movq	%r15, 1336(%rax)
	incq	%r14
	je	.LBB0_1
	movq	%rbx, 1232(%r15)
	movw	$0, 1328(%r15)
	movq	%rbx, %rax
	movq	%r14, %rdx
	addq	$8, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB0_1:
	.cfi_def_cfa %rbp, 16
.Ltmp0:
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.3(%rip), %rdi
	callq	*_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip)
.Ltmp1:
	ud2
.LBB0_3:
.Ltmp2:
	movq	%rax, %r14
	movl	$1432, %esi
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end0:
	.size	_RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_, .Lfunc_end0-_RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_
	.cfi_endproc
	.section	.gcc_except_table._RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table0:
.Lexception0:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end0-.Lcst_begin0
.Lcst_begin0:
	.uleb128 .Lfunc_begin0-.Lfunc_begin0
	.uleb128 .Ltmp0-.Lfunc_begin0
	.byte	0
	.byte	0
	.uleb128 .Ltmp0-.Lfunc_begin0
	.uleb128 .Ltmp1-.Ltmp0
	.uleb128 .Ltmp2-.Lfunc_begin0
	.byte	0
	.uleb128 .Ltmp1-.Lfunc_begin0
	.uleb128 .Lfunc_end0-.Ltmp1
	.byte	0
	.byte	0
.Lcst_end0:
	.p2align	2, 0x0

	.section	.text._RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_,"ax",@progbits
	.globl	_RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_
	.type	_RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_,@function
_RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_:
.Lfunc_begin1:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception1
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
	subq	$232, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rcx, %r14
	movq	%rdx, %r15
	movq	%rdi, %rbx
	movq	(%rsi), %rax
	cmpw	$11, 1330(%rax)
	jb	.LBB1_3
	movq	16(%rsi), %r12
	cmpq	$5, %r12
	jae	.LBB1_4
	movl	$4, %ecx
	xorl	%r13d, %r13d
	jmp	.LBB1_9
.LBB1_3:
	leaq	-264(%rbp), %r12
	movq	%r12, %rdi
	movq	%r15, %rdx
	movq	%r14, %rcx
	callq	*_RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_@GOTPCREL(%rip)
	movq	(%r12), %rax
	movq	$2, 8(%rbx)
	movq	%rax, 152(%rbx)
	vmovups	8(%r12), %xmm0
	jmp	.LBB1_11
.LBB1_4:
	je	.LBB1_5
	cmpq	$6, %r12
	jne	.LBB1_7
	movl	$5, %ecx
	movb	$1, %r13b
	xorl	%r12d, %r12d
	jmp	.LBB1_9
.LBB1_5:
	xorl	%r13d, %r13d
	movq	%r12, %rcx
	jmp	.LBB1_9
.LBB1_7:
	addq	$-7, %r12
	movl	$6, %ecx
	movb	$1, %r13b
.LBB1_9:
	movq	8(%rsi), %rdx
	leaq	-64(%rbp), %rsi
	movq	%rax, (%rsi)
	movq	%rdx, 8(%rsi)
	movq	%rcx, 16(%rsi)
.Ltmp3:
	leaq	-264(%rbp), %rdi
	callq	*_RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_@GOTPCREL(%rip)
.Ltmp4:
	movzbl	%r13b, %eax
	shll	$4, %eax
	vmovups	-144(%rbp,%rax), %xmm0
	leaq	-88(%rbp), %rsi
	vmovups	%xmm0, (%rsi)
	movq	%r12, 16(%rsi)
	leaq	-112(%rbp), %r12
	movq	%r12, %rdi
	movq	%r15, %rdx
	movq	%r14, %rcx
	callq	*_RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_@GOTPCREL(%rip)
	movq	(%r12), %rax
	vmovups	8(%r12), %xmm0
	vmovups	-264(%rbp), %zmm1
	vmovups	-200(%rbp), %zmm2
	vmovups	-176(%rbp), %zmm3
	vmovups	%zmm3, 88(%rbx)
	vmovups	%zmm2, 64(%rbx)
	vmovups	%zmm1, (%rbx)
	movq	%rax, 152(%rbx)
.LBB1_11:
	vmovups	%xmm0, 160(%rbx)
	movq	%rbx, %rax
	addq	$232, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB1_12:
	.cfi_def_cfa %rbp, 16
.Ltmp5:
	movq	%rax, %rbx
	cmpq	$0, (%r14)
	je	.LBB1_14
	addq	$8, %r14
	movl	$8, %esi
	movl	$16, %edx
	movq	%r14, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB1_14:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end1:
	.size	_RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_, .Lfunc_end1-_RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_
	.cfi_endproc
	.section	.gcc_except_table._RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table1:
.Lexception1:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end1-.Lcst_begin1
.Lcst_begin1:
	.uleb128 .Ltmp3-.Lfunc_begin1
	.uleb128 .Ltmp4-.Ltmp3
	.uleb128 .Ltmp5-.Lfunc_begin1
	.byte	0
	.uleb128 .Ltmp4-.Lfunc_begin1
	.uleb128 .Lfunc_end1-.Ltmp4
	.byte	0
	.byte	0
.Lcst_end1:
	.p2align	2, 0x0

	.section	.text._RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_,"ax",@progbits
	.globl	_RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_
	.type	_RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_,@function
_RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_:
.Lfunc_begin2:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception2
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
	movq	%rdx, %r12
	movq	8(%rsi), %rax
	leaq	-1(%rax), %rdx
	cmpq	%rdx, %r9
	jne	.LBB2_1
	movq	%r8, %r15
	movq	%rdi, %r14
	movq	(%rsi), %rdi
	cmpw	$11, 1330(%rdi)
	jae	.LBB2_4
	movq	%rsi, %rdi
	movq	%r12, %rsi
	movq	%rcx, %rdx
	movq	%r15, %rcx
	callq	*_RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_@GOTPCREL(%rip)
	movq	$2, 8(%r14)
	jmp	.LBB2_14
.LBB2_4:
	movq	16(%rsi), %r13
	cmpq	$5, %r13
	movq	%rcx, -48(%rbp)
	jae	.LBB2_7
	movl	$4, %edx
	xorl	%ebx, %ebx
	jmp	.LBB2_12
.LBB2_7:
	je	.LBB2_8
	cmpq	$6, %r13
	jne	.LBB2_10
	movb	$1, %bl
	movl	$5, %edx
	xorl	%r13d, %r13d
	jmp	.LBB2_12
.LBB2_8:
	xorl	%ebx, %ebx
	movq	%r13, %rdx
	jmp	.LBB2_12
.LBB2_10:
	addq	$-7, %r13
	movb	$1, %bl
	movl	$6, %edx
.LBB2_12:
	leaq	-72(%rbp), %rsi
	movq	%rdi, (%rsi)
	movq	%rax, 8(%rsi)
	movq	%rdx, 16(%rsi)
.Ltmp8:
	leaq	-248(%rbp), %rdi
	callq	*_RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_@GOTPCREL(%rip)
.Ltmp9:
	movzbl	%bl, %eax
	shll	$4, %eax
	vmovups	-128(%rbp,%rax), %xmm0
	leaq	-96(%rbp), %rdi
	vmovups	%xmm0, (%rdi)
	movq	%r13, 16(%rdi)
	movq	%r12, %rsi
	movq	-48(%rbp), %rdx
	movq	%r15, %rcx
	callq	*_RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_@GOTPCREL(%rip)
	vmovups	-248(%rbp), %zmm0
	vmovups	-184(%rbp), %zmm1
	vmovups	-160(%rbp), %zmm2
	vmovups	%zmm2, 88(%r14)
	vmovups	%zmm1, 64(%r14)
	vmovups	%zmm0, (%r14)
.LBB2_14:
	movq	%r14, %rax
	addq	$216, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB2_1:
	.cfi_def_cfa %rbp, 16
.Ltmp6:
	movq	%rcx, -48(%rbp)
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.4(%rip), %rdi
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.5(%rip), %rdx
	movl	$53, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip)
.Ltmp7:
	ud2
.LBB2_15:
.Ltmp10:
	movq	%rax, %r14
	movq	-48(%rbp), %rdi
	cmpq	$0, (%rdi)
	je	.LBB2_17
	addq	$8, %rdi
	movl	$8, %esi
	movl	$16, %edx
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB2_17:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end2:
	.size	_RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_, .Lfunc_end2-_RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_
	.cfi_endproc
	.section	.gcc_except_table._RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table2:
.Lexception2:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end2-.Lcst_begin2
.Lcst_begin2:
	.uleb128 .Ltmp8-.Lfunc_begin2
	.uleb128 .Ltmp7-.Ltmp8
	.uleb128 .Ltmp10-.Lfunc_begin2
	.byte	0
	.uleb128 .Ltmp7-.Lfunc_begin2
	.uleb128 .Lfunc_end2-.Ltmp7
	.byte	0
	.byte	0
.Lcst_end2:
	.p2align	2, 0x0

	.section	.text._RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_,"ax",@progbits
	.globl	_RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_
	.type	_RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_,@function
_RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_:
.Lfunc_begin3:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception3
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
	subq	$360, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%r8, %r15
	movq	%rdi, %r14
	leaq	-280(%rbp), %rbx
	movq	%rbx, %rdi
	callq	*_RINvMsK_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_@GOTPCREL(%rip)
	cmpl	$2, 8(%rbx)
	jne	.LBB3_1
	movq	-128(%rbp), %rax
	movq	%rax, (%r14)
	vmovups	-120(%rbp), %xmm0
	vmovups	%xmm0, 8(%r14)
	jmp	.LBB3_12
.LBB3_1:
	movq	%r15, -80(%rbp)
	movq	%r14, -72(%rbp)
	leaq	-272(%rbp), %rbx
	movq	120(%rbx), %rax
	movq	-8(%rbx), %r15
	movq	112(%rbx), %rcx
	vmovups	(%rbx), %zmm0
	vmovups	48(%rbx), %zmm1
	vmovups	%zmm0, -400(%rbp)
	vmovups	%zmm1, -352(%rbp)
	movq	136(%rbx), %r12
	movq	128(%rbx), %r13
	movq	152(%rbx), %rdx
	movq	%rdx, -64(%rbp)
	movq	144(%rbx), %rdx
	movq	%rdx, -48(%rbp)
	movq	160(%rbx), %rdx
	movq	%rdx, -56(%rbp)
	movq	1232(%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB3_5
	movq	_RINvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_4EdgeE6insertNtNtBc_5alloc6GlobalEB1K_@GOTPCREL(%rip), %r14
.LBB3_3:
	incq	%rax
	movzwl	1328(%rcx), %ecx
	movq	%rdx, -104(%rbp)
	movq	%rax, -96(%rbp)
	movq	%rcx, -88(%rbp)
	leaq	-280(%rbp), %rdi
	leaq	-104(%rbp), %rsi
	movq	%r15, %rdx
	leaq	-400(%rbp), %rcx
	movq	%r13, %r8
	movq	%r12, %r9
	vzeroupper
	callq	*%r14
	cmpl	$2, -272(%rbp)
	je	.LBB3_11
	movq	-152(%rbp), %rax
	movq	-280(%rbp), %r15
	movq	-160(%rbp), %rcx
	vmovups	(%rbx), %zmm0
	vmovups	48(%rbx), %zmm1
	vmovups	%zmm0, -400(%rbp)
	vmovups	%zmm1, -352(%rbp)
	movq	-136(%rbp), %r12
	movq	-144(%rbp), %r13
	movq	1232(%rcx), %rdx
	testq	%rdx, %rdx
	jne	.LBB3_3
.LBB3_5:
	vmovups	-400(%rbp), %zmm0
	vmovups	-352(%rbp), %zmm1
	vmovups	%zmm1, -224(%rbp)
	vmovups	%zmm0, -272(%rbp)
	movq	%r15, -280(%rbp)
	movq	%rcx, -160(%rbp)
	movq	%rax, -152(%rbp)
	movq	%r13, -144(%rbp)
	movq	%r12, -136(%rbp)
	movq	-80(%rbp), %rax
	movq	(%rax), %rbx
	movq	(%rbx), %rdi
	testq	%rdi, %rdi
	je	.LBB3_6
	movq	8(%rbx), %rsi
.Ltmp11:
	vzeroupper
	callq	*_RINvMs9_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_7NodeRefNtNtB6_6marker5OwnedyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1a_8InternalE12new_internalNtNtBc_5alloc6GlobalEB1z_@GOTPCREL(%rip)
.Ltmp12:
	leaq	-272(%rbp), %rcx
	movq	%rax, (%rbx)
	movq	%rdx, 8(%rbx)
	leaq	-104(%rbp), %rdi
	movq	%rdx, 8(%rdi)
	movq	%rax, (%rdi)
	movq	%r15, %rsi
	movq	%rcx, %rdx
	movq	%r13, %rcx
	movq	%r12, %r8
	callq	*_RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_@GOTPCREL(%rip)
.LBB3_11:
	movq	-72(%rbp), %r14
	movq	-48(%rbp), %rax
	movq	%rax, (%r14)
	movq	-64(%rbp), %rax
	movq	%rax, 8(%r14)
	movq	-56(%rbp), %rax
	movq	%rax, 16(%r14)
.LBB3_12:
	movq	%r14, %rax
	addq	$360, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB3_6:
	.cfi_def_cfa %rbp, 16
.Ltmp14:
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.10(%rip), %rdi
	vzeroupper
	callq	*_RNvNtCs2k2z8Zem4rB_4core6option13unwrap_failed@GOTPCREL(%rip)
.Ltmp15:
	ud2
.LBB3_8:
.Ltmp13:
	ud2
.LBB3_13:
.Ltmp16:
	movq	%rax, %rbx
	cmpq	$0, -272(%rbp)
	je	.LBB3_15
	leaq	-264(%rbp), %rdi
	movl	$8, %esi
	movl	$16, %edx
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB3_15:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end3:
	.size	_RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_, .Lfunc_end3-_RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_
	.cfi_endproc
	.section	.gcc_except_table._RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table3:
.Lexception3:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end3-.Lcst_begin3
.Lcst_begin3:
	.uleb128 .Lfunc_begin3-.Lfunc_begin3
	.uleb128 .Ltmp11-.Lfunc_begin3
	.byte	0
	.byte	0
	.uleb128 .Ltmp11-.Lfunc_begin3
	.uleb128 .Ltmp12-.Ltmp11
	.uleb128 .Ltmp13-.Lfunc_begin3
	.byte	0
	.uleb128 .Ltmp12-.Lfunc_begin3
	.uleb128 .Ltmp14-.Ltmp12
	.byte	0
	.byte	0
	.uleb128 .Ltmp14-.Lfunc_begin3
	.uleb128 .Ltmp15-.Ltmp14
	.uleb128 .Ltmp16-.Lfunc_begin3
	.byte	0
	.uleb128 .Ltmp15-.Lfunc_begin3
	.uleb128 .Lfunc_end3-.Ltmp15
	.byte	0
	.byte	0
.Lcst_end3:
	.p2align	2, 0x0

	.section	.text._RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_,"ax",@progbits
	.globl	_RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_
	.type	_RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_,@function
_RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_:
.Lfunc_begin4:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception4
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%rbx
	subq	$120, %rsp
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r15
	movq	%rdi, %r14
	callq	*_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_@GOTPCREL(%rip)
	movq	%rax, %rbx
	movq	$0, 1232(%rax)
	movw	$0, 1330(%rax)
.Ltmp17:
	leaq	-144(%rbp), %rdi
	movq	%r15, %rsi
	movq	%rax, %rdx
	callq	*_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_@GOTPCREL(%rip)
.Ltmp18:
	vmovups	(%r15), %xmm0
	vmovups	-144(%rbp), %zmm1
	vmovups	-88(%rbp), %zmm2
	vmovups	%zmm1, (%r14)
	vmovups	%zmm2, 56(%r14)
	vmovups	%xmm0, 120(%r14)
	movq	%rbx, 136(%r14)
	movq	$0, 144(%r14)
	movq	%r14, %rax
	addq	$120, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB4_2:
	.cfi_def_cfa %rbp, 16
.Ltmp19:
	movq	%rax, %r14
	movl	$1336, %esi
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end4:
	.size	_RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_, .Lfunc_end4-_RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_
	.cfi_endproc
	.section	.gcc_except_table._RINvMsV_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table4:
.Lexception4:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end4-.Lcst_begin4
.Lcst_begin4:
	.uleb128 .Lfunc_begin4-.Lfunc_begin4
	.uleb128 .Ltmp17-.Lfunc_begin4
	.byte	0
	.byte	0
	.uleb128 .Ltmp17-.Lfunc_begin4
	.uleb128 .Ltmp18-.Ltmp17
	.uleb128 .Ltmp19-.Lfunc_begin4
	.byte	0
	.uleb128 .Ltmp18-.Lfunc_begin4
	.uleb128 .Lfunc_end4-.Ltmp18
	.byte	0
	.byte	0
.Lcst_end4:
	.p2align	2, 0x0

	.section	.text._RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_,"ax",@progbits
	.globl	_RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_
	.type	_RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_,@function
_RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_:
.Lfunc_begin5:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception5
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
	subq	$136, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r12
	movq	%rdi, %r14
	movq	(%rsi), %rax
	movq	%rax, -48(%rbp)
	movzwl	1330(%rax), %r13d
	callq	*_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_@GOTPCREL(%rip)
	movq	%rax, %rbx
	movq	$0, 1232(%rax)
	movw	$0, 1330(%rax)
.Ltmp20:
	leaq	-168(%rbp), %rdi
	movq	%r12, %rsi
	movq	%rax, %rdx
	callq	*_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_@GOTPCREL(%rip)
.Ltmp21:
	movzwl	1330(%rbx), %eax
	leaq	1(%rax), %r15
	cmpq	$11, %rax
	ja	.LBB5_10
	movq	16(%r12), %rax
	subq	%rax, %r13
	cmpq	%r15, %r13
	jne	.LBB5_3
	movq	%rbx, %rdi
	addq	$1336, %rdi
	movq	-48(%rbp), %r13
	leaq	1344(,%rax,8), %rsi
	addq	%r13, %rsi
	leal	(,%r15,8), %edx
	callq	*memcpy@GOTPCREL(%rip)
	movq	8(%r12), %rax
	xorl	%ecx, %ecx
.LBB5_8:
	movq	1336(%rbx,%rcx,8), %rdx
	movq	%rbx, 1232(%rdx)
	movw	%cx, 1328(%rdx)
	incq	%rcx
	cmpq	%rcx, %r15
	jne	.LBB5_8
	vmovups	-168(%rbp), %zmm0
	vmovups	-112(%rbp), %zmm1
	vmovups	%zmm1, 56(%r14)
	vmovups	%zmm0, (%r14)
	movq	%r13, 120(%r14)
	movq	%rax, 128(%r14)
	movq	%rbx, 136(%r14)
	movq	%rax, 144(%r14)
	movq	%r14, %rax
	addq	$136, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB5_10:
	.cfi_def_cfa %rbp, 16
.Ltmp23:
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.6(%rip), %rcx
	movl	$12, %edx
	xorl	%edi, %edi
	movq	%r15, %rsi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.Ltmp24:
	jmp	.LBB5_4
.LBB5_3:
.Ltmp25:
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.7(%rip), %rdi
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.8(%rip), %rdx
	movl	$40, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip)
.Ltmp26:
.LBB5_4:
	ud2
.LBB5_11:
.Ltmp22:
	movq	%rax, %r14
	jmp	.LBB5_12
.LBB5_5:
.Ltmp27:
	movq	%rax, %r14
	cmpq	$0, -160(%rbp)
	je	.LBB5_12
	leaq	-152(%rbp), %rdi
	movl	$8, %esi
	movl	$16, %edx
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB5_12:
	movl	$1432, %esi
	movl	$8, %edx
	movq	%rbx, %rdi
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end5:
	.size	_RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_, .Lfunc_end5-_RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_
	.cfi_endproc
	.section	.gcc_except_table._RINvMsW_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_8InternalENtB1n_2KVE5splitNtNtBc_5alloc6GlobalEB1K_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table5:
.Lexception5:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end5-.Lcst_begin5
.Lcst_begin5:
	.uleb128 .Lfunc_begin5-.Lfunc_begin5
	.uleb128 .Ltmp20-.Lfunc_begin5
	.byte	0
	.byte	0
	.uleb128 .Ltmp20-.Lfunc_begin5
	.uleb128 .Ltmp21-.Ltmp20
	.uleb128 .Ltmp22-.Lfunc_begin5
	.byte	0
	.uleb128 .Ltmp21-.Lfunc_begin5
	.uleb128 .Ltmp23-.Ltmp21
	.byte	0
	.byte	0
	.uleb128 .Ltmp23-.Lfunc_begin5
	.uleb128 .Ltmp26-.Ltmp23
	.uleb128 .Ltmp27-.Lfunc_begin5
	.byte	0
	.uleb128 .Ltmp26-.Lfunc_begin5
	.uleb128 .Lfunc_end5-.Ltmp26
	.byte	0
	.byte	0
.Lcst_end5:
	.p2align	2, 0x0

	.section	.text._RINvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB6_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE3getyEB1f_,"ax",@progbits
	.globl	_RINvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB6_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE3getyEB1f_
	.type	_RINvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB6_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE3getyEB1f_,@function
_RINvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB6_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE3getyEB1f_:
.Lfunc_begin6:
	.cfi_startproc
	movq	(%rdi), %rcx
	testq	%rcx, %rcx
	je	.LBB6_1
	movq	8(%rdi), %rdx
	movq	(%rsi), %rsi
	xorl	%eax, %eax
.LBB6_4:
	movzwl	1330(%rcx), %r8d
	testq	%r8, %r8
	je	.LBB6_5
	imulq	$112, %r8, %r10
	xorl	%r9d, %r9d
	xorl	%edi, %edi
.LBB6_8:
	cmpq	1240(%rcx,%rdi,8), %rsi
	seta	%r11b
	sbbb	$0, %r11b
	cmpb	$1, %r11b
	jne	.LBB6_9
	incq	%rdi
	addq	$-112, %r9
	movq	%r10, %r11
	addq	%r9, %r11
	jne	.LBB6_8
.LBB6_5:
	movq	%r8, %rdi
	jmp	.LBB6_10
.LBB6_9:
	movzbl	%r11b, %r8d
	testl	%r8d, %r8d
	je	.LBB6_12
.LBB6_10:
	subq	$1, %rdx
	jb	.LBB6_2
	movq	1336(%rcx,%rdi,8), %rcx
	jmp	.LBB6_4
.LBB6_12:
	subq	%r9, %rcx
	movq	%rcx, %rax
	retq
.LBB6_1:
	xorl	%eax, %eax
.LBB6_2:
	retq
.Lfunc_end6:
	.size	_RINvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB6_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE3getyEB1f_, .Lfunc_end6-_RINvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB6_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE3getyEB1f_
	.cfi_endproc

	.section	.text._RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli,"ax",@progbits
	.globl	_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli
	.type	_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli,@function
_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli:
.Lfunc_begin7:
	.cfi_startproc
	leaq	-16(%rsp), %rax
	testq	%rdx, %rdx
	je	.LBB7_1
	movq	(%rdi), %rcx
	testq	%rcx, %rcx
	je	.LBB7_1
	imulq	%rdx, %rcx
	movq	8(%rdi), %rdi
	movq	%rsi, -16(%rsp)
	leaq	-8(%rsp), %rax
	jmp	.LBB7_4
.LBB7_1:
	xorl	%ecx, %ecx
.LBB7_4:
	movq	%rcx, (%rax)
	movq	-16(%rsp), %rdx
	testq	%rdx, %rdx
	je	.LBB7_6
	movq	-8(%rsp), %rsi
	testq	%rsi, %rsi
	jne	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
.LBB7_6:
	retq
.Lfunc_end7:
	.size	_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli, .Lfunc_end7-_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli
	.cfi_endproc

	.section	.text._RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE8grow_oneBS_,"ax",@progbits
	.globl	_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE8grow_oneBS_
	.type	_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE8grow_oneBS_,@function
_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE8grow_oneBS_:
.Lfunc_begin8:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	movq	(%rdi), %rsi
	movl	$1, %edx
	movl	$8, %ecx
	movl	$112, %r8d
	callq	*_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	cmpq	$-1, %rax
	jne	.LBB8_2
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB8_2:
	.cfi_def_cfa %rbp, 16
	movq	%rax, %rdi
	movq	%rdx, %rsi
	callq	*_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip)
.Lfunc_end8:
	.size	_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE8grow_oneBS_, .Lfunc_end8-_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE8grow_oneBS_
	.cfi_endproc

	.section	.text._RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_,"ax",@progbits
	.globl	_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_
	.type	_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_,@function
_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_:
.Lfunc_begin9:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	movq	(%rdi), %rsi
	movl	$1, %edx
	movl	$8, %ecx
	movl	$16, %r8d
	callq	*_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	cmpq	$-1, %rax
	jne	.LBB9_2
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB9_2:
	.cfi_def_cfa %rbp, 16
	movq	%rax, %rdi
	movq	%rdx, %rsi
	callq	*_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip)
.Lfunc_end9:
	.size	_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_, .Lfunc_end9-_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_
	.cfi_endproc

	.section	.text._RNvMs4_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_13Abbreviations6insert,"ax",@progbits
	.globl	_RNvMs4_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_13Abbreviations6insert
	.type	_RNvMs4_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_13Abbreviations6insert,@function
_RNvMs4_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_13Abbreviations6insert:
.Lfunc_begin10:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception6
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r12
	pushq	%rbx
	subq	$80, %rsp
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %rbx
	addq	$96, %rsi
	movq	96(%rbx), %r15
	leaq	-1(%r15), %r12
	cmpq	16(%rdi), %r12
	jae	.LBB10_1
.LBB10_7:
	movb	$1, %r14b
	cmpq	$0, (%rbx)
	je	.LBB10_12
	addq	$8, %rbx
	movl	$8, %esi
	movl	$16, %edx
	movq	%rbx, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	jmp	.LBB10_12
.LBB10_1:
	movq	%rdi, %r14
	jne	.LBB10_9
	cmpq	$0, 40(%r14)
	je	.LBB10_4
	leaq	24(%r14), %rdi
	callq	*_RINvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB6_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE3getyEB1f_@GOTPCREL(%rip)
	testq	%rax, %rax
	jne	.LBB10_7
.LBB10_4:
	cmpq	(%r14), %r12
	jne	.LBB10_6
.Ltmp28:
	movq	%r14, %rdi
	callq	*_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE8grow_oneBS_@GOTPCREL(%rip)
.Ltmp29:
.LBB10_6:
	movq	8(%r14), %rax
	imulq	$112, %r12, %rcx
	vmovups	(%rbx), %zmm0
	vmovups	48(%rbx), %zmm1
	vmovups	%zmm1, 48(%rax,%rcx)
	vmovups	%zmm0, (%rax,%rcx)
	movq	%r15, 16(%r14)
	jmp	.LBB10_11
.LBB10_9:
	addq	$24, %r14
	leaq	-72(%rbp), %r12
	movq	%r12, %rdi
	movq	%r14, %rsi
	movq	%r15, %rdx
	callq	*_RNvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB5_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE5entryB1e_@GOTPCREL(%rip)
	cmpq	$0, (%r12)
	je	.LBB10_7
	leaq	-104(%rbp), %rdi
	leaq	-72(%rbp), %rsi
	movq	%rbx, %rdx
	callq	*_RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_@GOTPCREL(%rip)
.LBB10_11:
	xorl	%r14d, %r14d
.LBB10_12:
	movl	%r14d, %eax
	addq	$80, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB10_13:
	.cfi_def_cfa %rbp, 16
.Ltmp30:
	movq	%rax, %r14
	cmpq	$0, (%rbx)
	je	.LBB10_15
	addq	$8, %rbx
	movl	$8, %esi
	movl	$16, %edx
	movq	%rbx, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB10_15:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end10:
	.size	_RNvMs4_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_13Abbreviations6insert, .Lfunc_end10-_RNvMs4_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_13Abbreviations6insert
	.cfi_endproc
	.section	.gcc_except_table._RNvMs4_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_13Abbreviations6insert,"a",@progbits
	.p2align	2, 0x0
GCC_except_table10:
.Lexception6:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end6-.Lcst_begin6
.Lcst_begin6:
	.uleb128 .Ltmp28-.Lfunc_begin10
	.uleb128 .Ltmp29-.Ltmp28
	.uleb128 .Ltmp30-.Lfunc_begin10
	.byte	0
	.uleb128 .Ltmp29-.Lfunc_begin10
	.uleb128 .Lfunc_end10-.Ltmp29
	.byte	0
	.byte	0
.Lcst_end6:
	.p2align	2, 0x0

	.section	.text._RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_,"ax",@progbits
	.globl	_RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_
	.type	_RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_,@function
_RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_:
.Lfunc_begin11:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception7
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r12
	pushq	%rbx
	subq	$48, %rsp
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdx, %r14
	movq	%rsi, %r15
	movq	%rdi, %rbx
	cmpq	$0, 16(%rsi)
	je	.LBB11_1
	leaq	16(%r15), %rsi
	movq	8(%r15), %rdx
	leaq	-56(%rbp), %rdi
	movq	%r14, %rcx
	movq	%r15, %r8
	callq	*_RINvMsN_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB6_6HandleINtB6_7NodeRefNtNtB6_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1n_4LeafENtB1n_4EdgeE16insert_recursingNtNtBc_5alloc6GlobalNCNvMs4_NtNtB8_3map5entryINtB3C_11VacantEntryyB1E_E12insert_entry0EB1K_@GOTPCREL(%rip)
	movq	(%r15), %r12
	jmp	.LBB11_3
.LBB11_1:
	movq	(%r15), %r12
.Ltmp31:
	callq	*_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_@GOTPCREL(%rip)
.Ltmp32:
	xorl	%ecx, %ecx
	movq	%rcx, 1232(%rax)
	movw	$0, 1330(%rax)
	movq	%rax, (%r12)
	movq	%rcx, 8(%r12)
	leaq	-72(%rbp), %rsi
	movq	%rcx, 8(%rsi)
	movq	%rax, (%rsi)
	movq	8(%r15), %rdx
	leaq	-56(%rbp), %rdi
	movq	%r14, %rcx
	callq	*_RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_@GOTPCREL(%rip)
.LBB11_3:
	incq	16(%r12)
	movq	-56(%rbp), %rax
	movq	%rax, (%rbx)
	vmovups	-48(%rbp), %xmm0
	vmovups	%xmm0, 8(%rbx)
	movq	%r12, 24(%rbx)
	movq	%rbx, %rax
	addq	$48, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB11_5:
	.cfi_def_cfa %rbp, 16
.Ltmp33:
	movq	%rax, %rbx
	cmpq	$0, (%r14)
	je	.LBB11_7
	addq	$8, %r14
	movl	$8, %esi
	movl	$16, %edx
	movq	%r14, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB11_7:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end11:
	.size	_RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_, .Lfunc_end11-_RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_
	.cfi_endproc
	.section	.gcc_except_table._RNvMs4_NtNtNtNtCsc70TAahYccp_5alloc11collections5btree3map5entryINtB5_11VacantEntryyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE12insert_entryB1q_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table11:
.Lexception7:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end7-.Lcst_begin7
.Lcst_begin7:
	.uleb128 .Lfunc_begin11-.Lfunc_begin11
	.uleb128 .Ltmp31-.Lfunc_begin11
	.byte	0
	.byte	0
	.uleb128 .Ltmp31-.Lfunc_begin11
	.uleb128 .Ltmp32-.Ltmp31
	.uleb128 .Ltmp33-.Lfunc_begin11
	.byte	0
	.uleb128 .Ltmp32-.Lfunc_begin11
	.uleb128 .Lfunc_end11-.Ltmp32
	.byte	0
	.byte	0
.Lcst_end7:
	.p2align	2, 0x0

	.section	.text.unlikely._RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs3wRyrdzSNKt_5gimli,"ax",@progbits
	.globl	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs3wRyrdzSNKt_5gimli
	.type	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs3wRyrdzSNKt_5gimli,@function
_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs3wRyrdzSNKt_5gimli:
.Lfunc_begin12:
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
	movq	%rcx, %r15
	movq	%rdi, %rbx
	movq	%r8, %rax
	mulq	%rdx
	movq	%rax, %r14
	seto	%al
	movabsq	$-9223372036854775808, %rcx
	subq	%r15, %rcx
	cmpq	%rcx, %r14
	seta	%cl
	orb	%al, %cl
	movl	$1, %r12d
	je	.LBB12_2
	movl	$8, %eax
	xorl	%r14d, %r14d
	jmp	.LBB12_17
.LBB12_2:
	leaq	-40(%rbp), %rax
	testq	%r8, %r8
	je	.LBB12_3
	movq	(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB12_3
	imulq	%r8, %rcx
	movq	8(%rsi), %rdi
	movq	%r15, -40(%rbp)
	leaq	-48(%rbp), %rax
	jmp	.LBB12_6
.LBB12_3:
	xorl	%ecx, %ecx
.LBB12_6:
	movq	%rcx, (%rax)
	cmpq	$0, -40(%rbp)
	je	.LBB12_11
	movq	-48(%rbp), %rsi
	testq	%rsi, %rsi
	je	.LBB12_8
	movq	%r15, %rdx
	movq	%r14, %rcx
	callq	_RNvCs2NWS7XDLE6y_7___rustc14___rust_realloc
	jmp	.LBB12_15
.LBB12_11:
	testq	%r14, %r14
	jne	.LBB12_14
	movq	%r15, %rax
	jmp	.LBB12_13
.LBB12_8:
	testq	%r14, %r14
	je	.LBB12_9
.LBB12_14:
	callq	_RNvCs2NWS7XDLE6y_7___rustc35___rust_no_alloc_shim_is_unstable_v2@PLT
	movq	%r14, %rdi
	movq	%r15, %rsi
	callq	_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc
.LBB12_15:
	testq	%rax, %rax
	je	.LBB12_16
.LBB12_13:
	movq	%rax, 8(%rbx)
	movl	$16, %eax
	xorl	%r12d, %r12d
	jmp	.LBB12_17
.LBB12_16:
	movq	%r15, 8(%rbx)
	movl	$16, %eax
.LBB12_17:
	movq	%r14, (%rbx,%rax)
	movq	%r12, (%rbx)
	movq	%rbx, %rax
	addq	$16, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB12_9:
	.cfi_def_cfa %rbp, 16
	movq	%r15, %rax
	jmp	.LBB12_15
.Lfunc_end12:
	.size	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs3wRyrdzSNKt_5gimli, .Lfunc_end12-_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs3wRyrdzSNKt_5gimli
	.cfi_endproc

	.section	.text._RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli,"ax",@progbits
	.globl	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli
	.type	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli,@function
_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli:
.Lfunc_begin13:
	.cfi_startproc
	testq	%r8, %r8
	jne	.LBB13_1
.LBB13_2:
	xorl	%eax, %eax
	retq
.LBB13_1:
	addq	%rsi, %rdx
	jb	.LBB13_2
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%rbx
	subq	$24, %rsp
	.cfi_offset %rbx, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rdi, %rbx
	movq	(%rdi), %rax
	addq	%rax, %rax
	cmpq	%rdx, %rax
	cmovbeq	%rdx, %rax
	xorl	%edx, %edx
	cmpq	$1025, %r8
	setb	%dl
	cmpq	$1, %r8
	leaq	1(%rdx,%rdx,2), %rdx
	movl	$8, %r14d
	cmovneq	%rdx, %r14
	cmpq	%rax, %r14
	cmovbeq	%rax, %r14
	leaq	-48(%rbp), %r15
	movq	%r15, %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	cmpl	$1, (%r15)
	jne	.LBB13_6
	movq	-40(%rbp), %rax
	movq	-32(%rbp), %rdx
	jmp	.LBB13_7
.LBB13_6:
	movq	-40(%rbp), %rax
	movq	%rax, 8(%rbx)
	movq	%r14, (%rbx)
	movq	$-1, %rax
.LBB13_7:
	addq	$24, %rsp
	popq	%rbx
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	.cfi_restore %rbx
	.cfi_restore %r14
	.cfi_restore %r15
	.cfi_restore %rbp
	retq
.Lfunc_end13:
	.size	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli, .Lfunc_end13-_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner14grow_amortizedCs3wRyrdzSNKt_5gimli
	.cfi_endproc

	.section	.text._RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli,"ax",@progbits
	.globl	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli
	.type	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli,@function
_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli:
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
	pushq	%rax
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%r8, %rax
	movq	%rcx, %r14
	movl	%edx, %r13d
	movq	%rdi, %rbx
	mulq	%rsi
	movq	%rax, %r12
	seto	%al
	movabsq	$-9223372036854775808, %rcx
	subq	%r14, %rcx
	cmpq	%rcx, %r12
	seta	%cl
	orb	%al, %cl
	je	.LBB14_3
	movq	$0, 8(%rbx)
.LBB14_2:
	movl	$1, %eax
	jmp	.LBB14_11
.LBB14_3:
	testq	%r12, %r12
	je	.LBB14_6
	movq	%rsi, %r15
	callq	_RNvCs2NWS7XDLE6y_7___rustc35___rust_no_alloc_shim_is_unstable_v2@PLT
	movq	%r12, %rdi
	movq	%r14, %rsi
	testb	%r13b, %r13b
	je	.LBB14_7
	callq	_RNvCs2NWS7XDLE6y_7___rustc19___rust_alloc_zeroed
	jmp	.LBB14_8
.LBB14_6:
	movq	$0, 8(%rbx)
	movq	%r14, 16(%rbx)
	jmp	.LBB14_10
.LBB14_7:
	callq	_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc
.LBB14_8:
	testq	%rax, %rax
	je	.LBB14_12
	movq	%r15, 8(%rbx)
	movq	%rax, 16(%rbx)
.LBB14_10:
	xorl	%eax, %eax
.LBB14_11:
	movq	%rax, (%rbx)
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
.LBB14_12:
	.cfi_def_cfa %rbp, 16
	movq	%r14, 8(%rbx)
	movq	%r12, 16(%rbx)
	jmp	.LBB14_2
.Lfunc_end14:
	.size	_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli, .Lfunc_end14-_RNvMs5_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner15try_allocate_inCs3wRyrdzSNKt_5gimli
	.cfi_endproc

	.section	.text._RNvMs5_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_12Abbreviation3new,"ax",@progbits
	.globl	_RNvMs5_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_12Abbreviation3new
	.type	_RNvMs5_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_12Abbreviation3new,@function
_RNvMs5_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_12Abbreviation3new:
.Lfunc_begin15:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception8
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
	movq	%r8, %rbx
	movq	%rsi, -24(%rbp)
	testq	%rsi, %rsi
	je	.LBB15_1
	movq	%rsi, 96(%rdi)
	movw	%dx, 104(%rdi)
	movb	%cl, 106(%rdi)
	vmovups	(%rbx), %zmm0
	vmovups	32(%rbx), %zmm1
	vmovups	%zmm0, (%rdi)
	vmovups	%zmm1, 32(%rdi)
	movq	%rdi, %rax
	addq	$16, %rsp
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB15_1:
	.cfi_def_cfa %rbp, 16
.Ltmp34:
	leaq	anon.886c1caabaaea59b9677b076c74ddc9f.751.llvm.14355606994395624983(%rip), %rdx
	leaq	anon.886c1caabaaea59b9677b076c74ddc9f.753.llvm.14355606994395624983(%rip), %r9
	leaq	-24(%rbp), %rsi
	movl	$1, %edi
	xorl	%ecx, %ecx
	callq	*_RINvNtCs2k2z8Zem4rB_4core9panicking13assert_failedyyEB4_@GOTPCREL(%rip)
.Ltmp35:
	ud2
.LBB15_4:
.Ltmp36:
	movq	%rax, %r14
	cmpq	$0, (%rbx)
	je	.LBB15_6
	addq	$8, %rbx
	movl	$8, %esi
	movl	$16, %edx
	movq	%rbx, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB15_6:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end15:
	.size	_RNvMs5_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_12Abbreviation3new, .Lfunc_end15-_RNvMs5_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_12Abbreviation3new
	.cfi_endproc
	.section	.gcc_except_table._RNvMs5_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_12Abbreviation3new,"a",@progbits
	.p2align	2, 0x0
GCC_except_table15:
.Lexception8:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end8-.Lcst_begin8
.Lcst_begin8:
	.uleb128 .Ltmp34-.Lfunc_begin15
	.uleb128 .Ltmp35-.Ltmp34
	.uleb128 .Ltmp36-.Lfunc_begin15
	.byte	0
	.uleb128 .Ltmp35-.Lfunc_begin15
	.uleb128 .Lfunc_end15-.Ltmp35
	.byte	0
	.byte	0
.Lcst_end8:
	.p2align	2, 0x0

	.section	.text._RNvMs6_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10Attributes4push,"ax",@progbits
	.globl	_RNvMs6_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10Attributes4push
	.type	_RNvMs6_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10Attributes4push,@function
_RNvMs6_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10Attributes4push:
.Lfunc_begin16:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception9
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r15
	pushq	%r14
	pushq	%r12
	pushq	%rbx
	subq	$32, %rsp
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r14
	movq	%rdi, %rbx
	leaq	8(%rdi), %r15
	cmpl	$1, (%rdi)
	jne	.LBB16_4
	movq	24(%rbx), %r12
	cmpq	8(%rbx), %r12
	jne	.LBB16_3
	movq	%r15, %rdi
	callq	*_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_@GOTPCREL(%rip)
.LBB16_3:
	movq	16(%rbx), %rax
	movq	%r12, %rcx
	shlq	$4, %rcx
	vmovups	(%r14), %xmm0
	vmovups	%xmm0, (%rax,%rcx)
	incq	%r12
	movq	%r12, 24(%rbx)
	jmp	.LBB16_8
.LBB16_4:
	movq	(%r15), %rdi
	cmpq	$5, %rdi
	jne	.LBB16_11
	callq	_RNvCs2NWS7XDLE6y_7___rustc35___rust_no_alloc_shim_is_unstable_v2@PLT
	movl	$80, %edi
	movl	$8, %esi
	callq	_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc
	testq	%rax, %rax
	je	.LBB16_13
	movl	$5, %ecx
	leaq	-56(%rbp), %rdi
	movq	%rcx, (%rdi)
	movq	%rax, 8(%rdi)
	vmovups	16(%rbx), %zmm0
	vmovups	32(%rbx), %zmm1
	vmovups	%zmm1, 16(%rax)
	vmovups	%zmm0, (%rax)
	movq	%rcx, 16(%rdi)
.Ltmp37:
	vzeroupper
	callq	*_RNvMs4_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVecNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev22AttributeSpecificationE8grow_oneBS_@GOTPCREL(%rip)
.Ltmp38:
	movq	-48(%rbp), %rax
	vmovups	(%r14), %xmm0
	vmovups	%xmm0, 80(%rax)
	movq	$6, -40(%rbp)
	movq	$1, (%rbx)
	movq	-40(%rbp), %rax
	movq	%rax, 16(%r15)
	vmovups	-56(%rbp), %xmm0
	vmovups	%xmm0, (%r15)
	jmp	.LBB16_8
.LBB16_11:
	jae	.LBB16_12
	movq	%rdi, %rax
	shlq	$4, %rax
	vmovups	(%r14), %xmm0
	vmovups	%xmm0, 16(%rbx,%rax)
	incq	%rdi
	movq	%rdi, 8(%rbx)
.LBB16_8:
	addq	$32, %rsp
	popq	%rbx
	popq	%r12
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB16_13:
	.cfi_def_cfa %rbp, 16
	movl	$8, %edi
	movl	$80, %esi
	callq	*_RNvNtCsc70TAahYccp_5alloc7raw_vec12handle_error@GOTPCREL(%rip)
.LBB16_12:
	leaq	anon.886c1caabaaea59b9677b076c74ddc9f.770.llvm.14355606994395624983(%rip), %rdx
	movl	$5, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking18panic_bounds_check@GOTPCREL(%rip)
.LBB16_9:
.Ltmp39:
	movq	%rax, %rbx
	leaq	-56(%rbp), %rdi
	movl	$8, %esi
	movl	$16, %edx
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end16:
	.size	_RNvMs6_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10Attributes4push, .Lfunc_end16-_RNvMs6_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10Attributes4push
	.cfi_endproc
	.section	.gcc_except_table._RNvMs6_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10Attributes4push,"a",@progbits
	.p2align	2, 0x0
GCC_except_table16:
.Lexception9:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end9-.Lcst_begin9
.Lcst_begin9:
	.uleb128 .Lfunc_begin16-.Lfunc_begin16
	.uleb128 .Ltmp37-.Lfunc_begin16
	.byte	0
	.byte	0
	.uleb128 .Ltmp37-.Lfunc_begin16
	.uleb128 .Ltmp38-.Ltmp37
	.uleb128 .Ltmp39-.Lfunc_begin16
	.byte	0
	.uleb128 .Ltmp38-.Lfunc_begin16
	.uleb128 .Lfunc_end16-.Ltmp38
	.byte	0
	.byte	0
.Lcst_end9:
	.p2align	2, 0x0

	.section	.text._RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_,"ax",@progbits
	.globl	_RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_
	.type	_RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_,@function
_RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_:
.Lfunc_begin17:
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
	subq	$136, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rsi, %r8
	movq	%rdi, %rbx
	movq	(%rsi), %r12
	movq	16(%rsi), %r13
	movzwl	1330(%r12), %r14d
	leaq	1(%r13), %rdi
	leaq	(%r12,%r13,8), %rsi
	addq	$1240, %rsi
	cmpq	%r14, %rdi
	jbe	.LBB17_2
	movq	%rdx, (%rsi)
	vmovups	(%rcx), %zmm0
	vmovups	48(%rcx), %zmm1
	vmovups	%zmm1, -128(%rbp)
	vmovups	%zmm0, -176(%rbp)
	jmp	.LBB17_3
.LBB17_2:
	movq	%rcx, -56(%rbp)
	leaq	1240(%r12), %rax
	leaq	(%rax,%rdi,8), %rdi
	movq	%r14, %r15
	subq	%r13, %r15
	movq	%rdx, -48(%rbp)
	leaq	(,%r15,8), %rdx
	movq	%r8, -64(%rbp)
	callq	*memmove@GOTPCREL(%rip)
	movq	-48(%rbp), %rax
	movq	%rax, 1240(%r12,%r13,8)
	movq	-56(%rbp), %rax
	vmovups	(%rax), %zmm0
	vmovups	48(%rax), %zmm1
	vmovups	%zmm1, -128(%rbp)
	vmovups	%zmm0, -176(%rbp)
	imulq	$112, %r13, %rsi
	addq	%r12, %rsi
	leaq	1(%r13), %rax
	imulq	$112, %rax, %rdi
	addq	%r12, %rdi
	imulq	$112, %r15, %rdx
	vzeroupper
	callq	*memmove@GOTPCREL(%rip)
	movq	-64(%rbp), %r8
.LBB17_3:
	incl	%r14d
	imulq	$112, %r13, %rax
	vmovups	-176(%rbp), %zmm0
	vmovups	-128(%rbp), %zmm1
	vmovups	%zmm1, 48(%r12,%rax)
	vmovups	%zmm0, (%r12,%rax)
	movw	%r14w, 1330(%r12)
	movq	8(%r8), %rax
	movq	%r12, (%rbx)
	movq	%rax, 8(%rbx)
	movq	%r13, 16(%rbx)
	movq	%rbx, %rax
	addq	$136, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.Lfunc_end17:
	.size	_RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_, .Lfunc_end17-_RNvMsJ_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_4EdgeE10insert_fitB1J_
	.cfi_endproc

	.section	.text._RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_,"ax",@progbits
	.globl	_RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_
	.type	_RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_,@function
_RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_:
.Lfunc_begin18:
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
	subq	$136, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	%rcx, %r15
	movq	%rdx, %r14
	movq	%rsi, %rdx
	movq	(%rdi), %r12
	movq	16(%rdi), %r13
	movzwl	1330(%r12), %ebx
	leal	1(%rbx), %eax
	movl	%eax, -44(%rbp)
	leaq	1(%r13), %rcx
	leaq	(%r12,%r13,8), %rsi
	addq	$1240, %rsi
	cmpq	%rbx, %rcx
	jbe	.LBB18_2
	movq	%rdx, (%rsi)
	vmovups	(%r14), %zmm0
	vmovups	48(%r14), %zmm1
	vmovups	%zmm1, -128(%rbp)
	vmovups	%zmm0, -176(%rbp)
	jmp	.LBB18_3
.LBB18_2:
	leaq	1240(%r12), %rax
	leaq	(%rax,%rcx,8), %rdi
	movq	%r15, -64(%rbp)
	movq	%rbx, %r15
	subq	%r13, %r15
	movq	%rdx, -56(%rbp)
	leaq	(,%r15,8), %rdx
	callq	*memmove@GOTPCREL(%rip)
	movq	-56(%rbp), %rax
	movq	%rax, 1240(%r12,%r13,8)
	vmovups	(%r14), %zmm0
	vmovups	48(%r14), %zmm1
	vmovups	%zmm1, -128(%rbp)
	vmovups	%zmm0, -176(%rbp)
	imulq	$112, %r13, %rsi
	addq	%r12, %rsi
	leaq	1(%r13), %rax
	imulq	$112, %rax, %rdi
	addq	%r12, %rdi
	imulq	$112, %r15, %rdx
	movq	-64(%rbp), %r15
	vzeroupper
	callq	*memmove@GOTPCREL(%rip)
.LBB18_3:
	imulq	$112, %r13, %rax
	vmovups	-176(%rbp), %zmm0
	vmovups	-128(%rbp), %zmm1
	vmovups	%zmm1, 48(%r12,%rax)
	vmovups	%zmm0, (%r12,%rax)
	leaq	2(%rbx), %r14
	leaq	2(%r13), %rax
	cmpq	%rax, %r14
	jbe	.LBB18_5
	leaq	1336(%r12), %rcx
	leaq	1(%r13), %rdx
	leaq	(%rcx,%rdx,8), %rsi
	leaq	(%rcx,%rax,8), %rdi
	movq	%rbx, %rdx
	subq	%r13, %rdx
	shlq	$3, %rdx
	vzeroupper
	callq	*memmove@GOTPCREL(%rip)
.LBB18_5:
	movq	%r15, 1344(%r12,%r13,8)
	movl	-44(%rbp), %eax
	movw	%ax, 1330(%r12)
	leaq	1(%r13), %rax
	cmpq	%r14, %rax
	jae	.LBB18_8
	incq	%rbx
.LBB18_7:
	movq	1344(%r12,%r13,8), %rax
	movq	%r12, 1232(%rax)
	incq	%r13
	movw	%r13w, 1328(%rax)
	cmpq	%r13, %rbx
	jne	.LBB18_7
.LBB18_8:
	addq	$136, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.Lfunc_end18:
	.size	_RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_, .Lfunc_end18-_RNvMsM_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_4EdgeE10insert_fitB1J_
	.cfi_endproc

	.section	.text._RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_,"ax",@progbits
	.globl	_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_
	.type	_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_,@function
_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_:
.Lfunc_begin19:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception10
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
	subq	$120, %rsp
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	movq	(%rsi), %r12
	movq	16(%rsi), %r13
	movzwl	1330(%r12), %eax
	movq	%r13, %r14
	notq	%r14
	addq	%rax, %r14
	movw	%r14w, 1330(%rdx)
	movq	1240(%r12,%r13,8), %rcx
	imulq	$112, %r13, %rax
	vmovups	(%r12,%rax), %zmm0
	vmovups	48(%r12,%rax), %zmm1
	vmovups	%zmm1, -112(%rbp)
	vmovups	%zmm0, -160(%rbp)
	cmpq	$12, %r14
	jae	.LBB19_1
	movq	%rdx, %r15
	movq	%rdi, %rbx
	leaq	1240(%r12), %rax
	leaq	(%rax,%r13,8), %rsi
	addq	$8, %rsi
	leaq	1240(%rdx), %rdi
	leaq	(,%r14,8), %rdx
	movq	%rcx, -48(%rbp)
	vzeroupper
	callq	*memcpy@GOTPCREL(%rip)
	leaq	1(%r13), %rax
	imulq	$112, %rax, %rsi
	addq	%r12, %rsi
	imulq	$112, %r14, %rdx
	movq	%r15, %rdi
	callq	*memcpy@GOTPCREL(%rip)
	movw	%r13w, 1330(%r12)
	vmovups	-160(%rbp), %zmm0
	vmovups	-112(%rbp), %zmm1
	vmovups	%zmm0, 8(%rbx)
	vmovups	%zmm1, 56(%rbx)
	movq	-48(%rbp), %rax
	movq	%rax, (%rbx)
	movq	%rbx, %rax
	addq	$120, %rsp
	popq	%rbx
	popq	%r12
	popq	%r13
	popq	%r14
	popq	%r15
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB19_1:
	.cfi_def_cfa %rbp, 16
.Ltmp40:
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1122(%rip), %rcx
	movl	$11, %edx
	xorl	%edi, %edi
	movq	%r14, %rsi
	vzeroupper
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.Ltmp41:
	ud2
.LBB19_3:
.Ltmp42:
	movq	%rax, %rbx
	cmpq	$0, -160(%rbp)
	je	.LBB19_5
	leaq	-152(%rbp), %rdi
	movl	$8, %esi
	movl	$16, %edx
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB19_5:
	movq	%rbx, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end19:
	.size	_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_, .Lfunc_end19-_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_
	.cfi_endproc
	.section	.gcc_except_table._RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table19:
.Lexception10:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end10-.Lcst_begin10
.Lcst_begin10:
	.uleb128 .Lfunc_begin19-.Lfunc_begin19
	.uleb128 .Ltmp40-.Lfunc_begin19
	.byte	0
	.byte	0
	.uleb128 .Ltmp40-.Lfunc_begin19
	.uleb128 .Ltmp41-.Ltmp40
	.uleb128 .Ltmp42-.Lfunc_begin19
	.byte	0
	.uleb128 .Ltmp41-.Lfunc_begin19
	.uleb128 .Lfunc_end19-.Ltmp41
	.byte	0
	.byte	0
.Lcst_end10:
	.p2align	2, 0x0

	.section	.text._RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_,"ax",@progbits
	.globl	_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_
	.type	_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_,@function
_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_:
.Lfunc_begin20:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	callq	_RNvCs2NWS7XDLE6y_7___rustc35___rust_no_alloc_shim_is_unstable_v2@PLT
	movl	$1432, %edi
	movl	$8, %esi
	callq	_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc
	testq	%rax, %rax
	je	.LBB20_2
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB20_2:
	.cfi_def_cfa %rbp, 16
	movl	$8, %edi
	movl	$1432, %esi
	callq	*_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip)
.Lfunc_end20:
	.size	_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_, .Lfunc_end20-_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node12InternalNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1B_
	.cfi_endproc

	.section	.text._RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_,"ax",@progbits
	.globl	_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_
	.type	_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_,@function
_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_:
.Lfunc_begin21:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	callq	_RNvCs2NWS7XDLE6y_7___rustc35___rust_no_alloc_shim_is_unstable_v2@PLT
	movl	$1336, %edi
	movl	$8, %esi
	callq	_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc
	testq	%rax, %rax
	je	.LBB21_2
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.LBB21_2:
	.cfi_def_cfa %rbp, 16
	movl	$8, %edi
	movl	$1336, %esi
	callq	*_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error@GOTPCREL(%rip)
.Lfunc_end21:
	.size	_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_, .Lfunc_end21-_RNvMs_NtCsc70TAahYccp_5alloc5boxedINtB4_3BoxINtNtNtNtB6_11collections5btree4node8LeafNodeyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationEE13new_uninit_inB1w_
	.cfi_endproc

	.section	.text._RNvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB5_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE5entryB1e_,"ax",@progbits
	.globl	_RNvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB5_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE5entryB1e_
	.type	_RNvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB5_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE5entryB1e_,@function
_RNvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB5_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE5entryB1e_:
.Lfunc_begin22:
	.cfi_startproc
	movq	%rdi, %rax
	movq	(%rsi), %rcx
	testq	%rcx, %rcx
	je	.LBB22_11
	movq	8(%rsi), %rdi
.LBB22_2:
	movzwl	1330(%rcx), %r9d
	testl	%r9d, %r9d
	je	.LBB22_3
	movl	%r9d, %r10d
	shll	$3, %r10d
	xorl	%r8d, %r8d
.LBB22_6:
	cmpq	1240(%rcx,%r8,8), %rdx
	seta	%r11b
	sbbb	$0, %r11b
	cmpb	$1, %r11b
	jne	.LBB22_7
	incq	%r8
	addq	$-8, %r10
	jne	.LBB22_6
.LBB22_3:
	movq	%r9, %r8
	jmp	.LBB22_8
.LBB22_7:
	movzbl	%r11b, %r9d
	testl	%r9d, %r9d
	je	.LBB22_10
.LBB22_8:
	subq	$1, %rdi
	jb	.LBB22_13
	movq	1336(%rcx,%r8,8), %rcx
	jmp	.LBB22_2
.LBB22_10:
	movq	%rcx, 8(%rax)
	movq	%rdi, 16(%rax)
	movq	%r8, 24(%rax)
	movq	%rsi, 32(%rax)
	movq	$0, (%rax)
	retq
.LBB22_11:
	movq	%rsi, (%rax)
	movq	%rdx, 8(%rax)
	movq	$0, 16(%rax)
	retq
.LBB22_13:
	movq	%rsi, (%rax)
	movq	%rdx, 8(%rax)
	movq	%rcx, 16(%rax)
	movq	$0, 24(%rax)
	movq	%r8, 32(%rax)
	retq
.Lfunc_end22:
	.size	_RNvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB5_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE5entryB1e_, .Lfunc_end22-_RNvMsi_NtNtNtCsc70TAahYccp_5alloc11collections5btree3mapINtB5_8BTreeMapyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationE5entryB1e_
	.cfi_endproc

	.section	.text._RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_,"ax",@progbits
	.globl	_RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_
	.type	_RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_,@function
_RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_:
.Lfunc_begin23:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception11
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rcx, %rbx
	movq	(%rsi), %rcx
	movzwl	1330(%rcx), %eax
	cmpq	$11, %rax
	jae	.LBB23_1
	leal	1(%rax), %r8d
	movw	%r8w, 1330(%rcx)
	movq	%rdx, 1240(%rcx,%rax,8)
	imulq	$112, %rax, %rdx
	vmovups	(%rbx), %zmm0
	vmovups	48(%rbx), %zmm1
	vmovups	%zmm1, 48(%rcx,%rdx)
	vmovups	%zmm0, (%rcx,%rdx)
	movq	8(%rsi), %rdx
	movq	%rcx, (%rdi)
	movq	%rdx, 8(%rdi)
	movq	%rax, 16(%rdi)
	movq	%rdi, %rax
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB23_1:
	.cfi_def_cfa %rbp, 16
.Ltmp43:
	leaq	anon.886c1caabaaea59b9677b076c74ddc9f.1546.llvm.14355606994395624983(%rip), %rdi
	leaq	anon.886c1caabaaea59b9677b076c74ddc9f.1547.llvm.14355606994395624983(%rip), %rdx
	movl	$32, %esi
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip)
.Ltmp44:
	ud2
.LBB23_4:
.Ltmp45:
	movq	%rax, %r14
	cmpq	$0, (%rbx)
	je	.LBB23_6
	addq	$8, %rbx
	movl	$8, %esi
	movl	$16, %edx
	movq	%rbx, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB23_6:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end23:
	.size	_RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_, .Lfunc_end23-_RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_
	.cfi_endproc
	.section	.gcc_except_table._RNvMsu_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_4LeafE16push_with_handleB1w_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table23:
.Lexception11:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end11-.Lcst_begin11
.Lcst_begin11:
	.uleb128 .Ltmp43-.Lfunc_begin23
	.uleb128 .Ltmp44-.Ltmp43
	.uleb128 .Ltmp45-.Lfunc_begin23
	.byte	0
	.uleb128 .Ltmp44-.Lfunc_begin23
	.uleb128 .Lfunc_end23-.Ltmp44
	.byte	0
	.byte	0
.Lcst_end11:
	.p2align	2, 0x0

	.section	.text._RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_,"ax",@progbits
	.globl	_RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_
	.type	_RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_,@function
_RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_:
.Lfunc_begin24:
	.cfi_startproc
	.cfi_personality 155, DW.ref.rust_eh_personality
	.cfi_lsda 27, .Lexception12
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	pushq	%r14
	pushq	%rbx
	.cfi_offset %rbx, -32
	.cfi_offset %r14, -24
	movq	%rdx, %rbx
	movq	8(%rdi), %rax
	decq	%rax
	cmpq	%rax, %r8
	jne	.LBB24_1
	movq	(%rdi), %rdx
	movzwl	1330(%rdx), %eax
	cmpq	$10, %rax
	ja	.LBB24_5
	leal	1(%rax), %edi
	movw	%di, 1330(%rdx)
	movq	%rsi, 1240(%rdx,%rax,8)
	imulq	$112, %rax, %rsi
	vmovups	(%rbx), %zmm0
	vmovups	48(%rbx), %zmm1
	vmovups	%zmm1, 48(%rdx,%rsi)
	vmovups	%zmm0, (%rdx,%rsi)
	movq	%rcx, 1344(%rdx,%rax,8)
	incq	%rax
	movq	%rdx, 1232(%rcx)
	movw	%ax, 1328(%rcx)
	popq	%rbx
	popq	%r14
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	vzeroupper
	retq
.LBB24_1:
	.cfi_def_cfa %rbp, 16
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1549(%rip), %rdx
	movl	$48, %esi
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1548(%rip), %rdi
	jmp	.LBB24_2
.LBB24_5:
	leaq	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1550(%rip), %rdx
	movl	$32, %esi
	leaq	anon.886c1caabaaea59b9677b076c74ddc9f.1546.llvm.14355606994395624983(%rip), %rdi
.LBB24_2:
.Ltmp46:
	callq	*_RNvNtCs2k2z8Zem4rB_4core9panicking5panic@GOTPCREL(%rip)
.Ltmp47:
	ud2
.LBB24_7:
.Ltmp48:
	movq	%rax, %r14
	cmpq	$0, (%rbx)
	je	.LBB24_9
	addq	$8, %rbx
	movl	$8, %esi
	movl	$16, %edx
	movq	%rbx, %rdi
	callq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.LBB24_9:
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.Lfunc_end24:
	.size	_RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_, .Lfunc_end24-_RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_
	.cfi_endproc
	.section	.gcc_except_table._RNvMsv_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB19_8InternalE4pushB1w_,"a",@progbits
	.p2align	2, 0x0
GCC_except_table24:
.Lexception12:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 .Lcst_end12-.Lcst_begin12
.Lcst_begin12:
	.uleb128 .Ltmp46-.Lfunc_begin24
	.uleb128 .Ltmp47-.Ltmp46
	.uleb128 .Ltmp48-.Lfunc_begin24
	.byte	0
	.uleb128 .Ltmp47-.Lfunc_begin24
	.uleb128 .Lfunc_end24-.Ltmp47
	.byte	0
	.byte	0
.Lcst_end12:
	.p2align	2, 0x0

	.section	.text._RNvXs1_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli,"ax",@progbits
	.globl	_RNvXs1_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli
	.type	_RNvXs1_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli,@function
_RNvXs1_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli:
.Lfunc_begin25:
	.cfi_startproc
	movl	$1, %esi
	movl	$1, %edx
	jmpq	*_RNvMs3_NtCsc70TAahYccp_5alloc7raw_vecNtB5_11RawVecInner10deallocateCs3wRyrdzSNKt_5gimli@GOTPCREL(%rip)
.Lfunc_end25:
	.size	_RNvXs1_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli, .Lfunc_end25-_RNvXs1_NtCsc70TAahYccp_5alloc7raw_vecINtB5_6RawVechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli
	.cfi_endproc

	.section	.text._RNvXsa_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10AttributesNtNtNtCs2k2z8Zem4rB_4core3ops5deref5Deref5deref,"ax",@progbits
	.globl	_RNvXsa_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10AttributesNtNtNtCs2k2z8Zem4rB_4core3ops5deref5Deref5deref
	.type	_RNvXsa_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10AttributesNtNtNtCs2k2z8Zem4rB_4core3ops5deref5Deref5deref,@function
_RNvXsa_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10AttributesNtNtNtCs2k2z8Zem4rB_4core3ops5deref5Deref5deref:
.Lfunc_begin26:
	.cfi_startproc
	cmpl	$1, (%rdi)
	jne	.LBB26_2
	movq	16(%rdi), %rax
	movq	24(%rdi), %rsi
	jmp	.LBB26_4
.LBB26_2:
	movq	8(%rdi), %rsi
	cmpq	$5, %rsi
	ja	.LBB26_5
	addq	$16, %rdi
	movq	%rdi, %rax
.LBB26_4:
	movq	%rsi, %rdx
	retq
.LBB26_5:
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	leaq	anon.886c1caabaaea59b9677b076c74ddc9f.1812.llvm.14355606994395624983(%rip), %rcx
	movl	$5, %edx
	xorl	%edi, %edi
	callq	*_RNvNtNtCs2k2z8Zem4rB_4core5slice5index16slice_index_fail@GOTPCREL(%rip)
.Lfunc_end26:
	.size	_RNvXsa_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10AttributesNtNtNtCs2k2z8Zem4rB_4core3ops5deref5Deref5deref, .Lfunc_end26-_RNvXsa_NtNtCs3wRyrdzSNKt_5gimli4read6abbrevNtB5_10AttributesNtNtNtCs2k2z8Zem4rB_4core3ops5deref5Deref5deref
	.cfi_endproc

	.section	.text._RNvXsp_NtCsc70TAahYccp_5alloc3vecINtB5_3VechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli,"ax",@progbits
	.globl	_RNvXsp_NtCsc70TAahYccp_5alloc3vecINtB5_3VechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli
	.type	_RNvXsp_NtCsc70TAahYccp_5alloc3vecINtB5_3VechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli,@function
_RNvXsp_NtCsc70TAahYccp_5alloc3vecINtB5_3VechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli:
.Lfunc_begin27:
	.cfi_startproc
	retq
.Lfunc_end27:
	.size	_RNvXsp_NtCsc70TAahYccp_5alloc3vecINtB5_3VechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli, .Lfunc_end27-_RNvXsp_NtCsc70TAahYccp_5alloc3vecINtB5_3VechENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropCs3wRyrdzSNKt_5gimli
	.cfi_endproc

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983:
	.asciz	"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/alloc/src/collections/btree/node.rs"
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983, 92

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.3,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.3,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.3:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000\360\000\000\000M\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.3, 24

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.4,@object
	.section	.rodata..Lanon.886c1caabaaea59b9677b076c74ddc9f.4,"a",@progbits
.Lanon.886c1caabaaea59b9677b076c74ddc9f.4:
	.ascii	"assertion failed: edge.height == self.node.height - 1"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.4, 53

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.5,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.5,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.5:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000#\004\000\000\t\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.5, 24

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.6,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.6,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.6:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000E\005\000\000$\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.6, 24

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.7,@object
	.section	.rodata..Lanon.886c1caabaaea59b9677b076c74ddc9f.7,"a",@progbits
.Lanon.886c1caabaaea59b9677b076c74ddc9f.7:
	.ascii	"assertion failed: src.len() == dst.len()"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.7, 40

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.8,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.8,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.8:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000\232\007\000\000\005\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.8, 24

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.9,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lanon.886c1caabaaea59b9677b076c74ddc9f.9:
	.asciz	"/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/alloc/src/collections/btree/map/entry.rs"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.9, 97

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.10,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.10,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.10:
	.quad	.Lanon.886c1caabaaea59b9677b076c74ddc9f.9
	.asciz	"`\000\000\000\000\000\000\000\321\001\000\000.\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.10, 24

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.751.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.751.llvm.14355606994395624983,@object
	.section	.rodata.cst8,"aM",@progbits,8
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.751.llvm.14355606994395624983
	.p2align	3, 0x0
anon.886c1caabaaea59b9677b076c74ddc9f.751.llvm.14355606994395624983:
	.zero	8
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.751.llvm.14355606994395624983, 8

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983
anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983:
	.asciz	"/cargo/registry/25cdd57fae9f0462/gimli-0.34.0/src/read/abbrev.rs"
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983, 65

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.753.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.753.llvm.14355606994395624983,@object
	.section	.data.rel.ro.anon.886c1caabaaea59b9677b076c74ddc9f.753.llvm.14355606994395624983,"aw",@progbits
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.753.llvm.14355606994395624983
	.p2align	3, 0x0
anon.886c1caabaaea59b9677b076c74ddc9f.753.llvm.14355606994395624983:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983
	.asciz	"@\000\000\000\000\000\000\000.\001\000\000\t\000\000"
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.753.llvm.14355606994395624983, 24

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.770.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.770.llvm.14355606994395624983,@object
	.section	.data.rel.ro.anon.886c1caabaaea59b9677b076c74ddc9f.770.llvm.14355606994395624983,"aw",@progbits
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.770.llvm.14355606994395624983
	.p2align	3, 0x0
anon.886c1caabaaea59b9677b076c74ddc9f.770.llvm.14355606994395624983:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983
	.asciz	"@\000\000\000\000\000\000\000\253\001\000\000\021\000\000"
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.770.llvm.14355606994395624983, 24

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1122,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.1122,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.1122:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000\000\005\000\000#\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1122, 24

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.1546.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.1546.llvm.14355606994395624983,@object
	.section	.rodata.cst32,"aM",@progbits,32
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.1546.llvm.14355606994395624983
anon.886c1caabaaea59b9677b076c74ddc9f.1546.llvm.14355606994395624983:
	.ascii	"assertion failed: idx < CAPACITY"
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.1546.llvm.14355606994395624983, 32

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.1547.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.1547.llvm.14355606994395624983,@object
	.section	.data.rel.ro.anon.886c1caabaaea59b9677b076c74ddc9f.1547.llvm.14355606994395624983,"aw",@progbits
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.1547.llvm.14355606994395624983
	.p2align	3, 0x0
anon.886c1caabaaea59b9677b076c74ddc9f.1547.llvm.14355606994395624983:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000\260\002\000\000\t\000\000"
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.1547.llvm.14355606994395624983, 24

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1548,@object
	.section	.rodata..Lanon.886c1caabaaea59b9677b076c74ddc9f.1548,"a",@progbits
.Lanon.886c1caabaaea59b9677b076c74ddc9f.1548:
	.ascii	"assertion failed: edge.height == self.height - 1"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1548, 48

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1549,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.1549,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.1549:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000\311\002\000\000\t\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1549, 24

	.type	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1550,@object
	.section	.data.rel.ro..Lanon.886c1caabaaea59b9677b076c74ddc9f.1550,"aw",@progbits
	.p2align	3, 0x0
.Lanon.886c1caabaaea59b9677b076c74ddc9f.1550:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.2.llvm.14355606994395624983
	.asciz	"[\000\000\000\000\000\000\000\315\002\000\000\t\000\000"
	.size	.Lanon.886c1caabaaea59b9677b076c74ddc9f.1550, 24

	.hidden	anon.886c1caabaaea59b9677b076c74ddc9f.1812.llvm.14355606994395624983
	.type	anon.886c1caabaaea59b9677b076c74ddc9f.1812.llvm.14355606994395624983,@object
	.section	.data.rel.ro.anon.886c1caabaaea59b9677b076c74ddc9f.1812.llvm.14355606994395624983,"aw",@progbits
	.globl	anon.886c1caabaaea59b9677b076c74ddc9f.1812.llvm.14355606994395624983
	.p2align	3, 0x0
anon.886c1caabaaea59b9677b076c74ddc9f.1812.llvm.14355606994395624983:
	.quad	anon.886c1caabaaea59b9677b076c74ddc9f.752.llvm.14355606994395624983
	.asciz	"@\000\000\000\000\000\000\000\304\001\000\0004\000\000"
	.size	anon.886c1caabaaea59b9677b076c74ddc9f.1812.llvm.14355606994395624983, 24

	.hidden	_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc
	.hidden	_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc
	.hidden	_RNvCs2NWS7XDLE6y_7___rustc14___rust_realloc
	.hidden	_RNvCs2NWS7XDLE6y_7___rustc19___rust_alloc_zeroed
	.globl	_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_2KVE15split_leaf_dataB1J_
	.type	_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_2KVE15split_leaf_dataB1J_,@function
_RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_8InternalENtB1m_2KVE15split_leaf_dataB1J_ = _RNvMsU_NtNtNtCsc70TAahYccp_5alloc11collections5btree4nodeINtB5_6HandleINtB5_7NodeRefNtNtB5_6marker3MutyNtNtNtCs3wRyrdzSNKt_5gimli4read6abbrev12AbbreviationNtB1m_4LeafENtB1m_2KVE15split_leaf_dataB1J_
	.hidden	DW.ref.rust_eh_personality
	.weak	DW.ref.rust_eh_personality
	.section	.data.DW.ref.rust_eh_personality,"awG",@progbits,DW.ref.rust_eh_personality,comdat
	.p2align	3, 0x0
	.type	DW.ref.rust_eh_personality,@object
	.size	DW.ref.rust_eh_personality, 8
DW.ref.rust_eh_personality:
	.quad	rust_eh_personality
	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits

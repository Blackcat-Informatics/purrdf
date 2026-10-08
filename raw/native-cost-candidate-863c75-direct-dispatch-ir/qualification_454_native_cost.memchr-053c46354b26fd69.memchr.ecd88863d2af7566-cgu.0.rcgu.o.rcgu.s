	.att_syntax
	.file	"memchr.ecd88863d2af7566-cgu.0"
	.section	.text._RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect,"ax",@progbits
	.globl	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect,@function
_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect:
.Lfunc_begin0:
	.file	1 "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3" "src/arch/x86_64/memchr.rs"
	.loc	1 109 0
	.cfi_startproc
	movq	%rdx, %rcx
	movq	%rsi, %rdx
.Ltmp0:
	.file	2 "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad" "library/core/src/sync/atomic.rs"
	.loc	2 4303 24 prologue_end
	leaq	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2(%rip), %rax
	movq	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN@GOTPCREL(%rip), %rsi
	movq	%rax, (%rsi)
.Ltmp1:
	.file	3 "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3" "src/arch/x86_64/sse2/memchr.rs"
	.loc	3 161 12
	cmpq	%rcx, %rdx
	jae	.LBB0_19
.Ltmp2:
	.file	4 "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad" "library/core/src/ptr/const_ptr.rs"
	.loc	4 624 18
	movq	%rcx, %r8
	subq	%rdx, %r8
.Ltmp3:
	.loc	3 164 12
	cmpq	$15, %r8
	ja	.LBB0_6
.Ltmp4:
	.file	5 "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3" "src/arch/generic/memchr.rs"
	.loc	5 1161 11
	addq	%rdx, %r8
	xorl	%eax, %eax
	.loc	5 0 11 is_stmt 0
.Ltmp5:
	.p2align	4
.LBB0_3:
.Ltmp6:
	.loc	3 167 17 is_stmt 1
	cmpb	%dil, (%rdx)
.Ltmp7:
	.loc	5 1162 12
	je	.LBB0_8
.Ltmp8:
	.loc	4 390 18
	incq	%rdx
.Ltmp9:
	.loc	5 1161 11
	cmpq	%rcx, %rdx
	jne	.LBB0_3
.Ltmp10:
	.loc	5 0 11 is_stmt 0
	movq	%r8, %rdx
	.loc	1 145 10 is_stmt 1
	retq
.LBB0_6:
.Ltmp11:
	.loc	1 0 0 is_stmt 0
	vpbroadcastb	%edi, %xmm0
.Ltmp12:
	.file	6 "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad" "library/core/src/../../stdarch/crates/core_arch/src/x86/sse2.rs"
	.loc	6 920 36 is_stmt 1
	vpcmpeqb	(%rdx), %xmm0, %k0
.Ltmp13:
	.file	7 "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3" "src/vector.rs"
	.loc	7 147 9
	kortestw	%k0, %k0
.Ltmp14:
	.loc	5 423 12
	je	.LBB0_9
.Ltmp15:
	.loc	5 0 0 is_stmt 0
	kmovd	%k0, %eax
.Ltmp16:
	tzcntl	%eax, %eax
	.loc	5 155 9 is_stmt 1
	addq	%rax, %rdx
.Ltmp17:
.LBB0_8:
	.loc	5 0 9 is_stmt 0
	movl	$1, %eax
	.loc	1 145 10 is_stmt 1
	retq
.LBB0_9:
.Ltmp18:
	.loc	4 872 18
	movq	%rdx, %rsi
	andq	$-16, %rsi
	addq	$16, %rsi
.Ltmp19:
	.loc	5 171 12
	cmpq	$64, %r8
	setae	%dil
	leaq	-64(%rcx), %rax
	cmpq	%rax, %rsi
	setbe	%r8b
	andb	%dil, %r8b
	cmpb	$1, %r8b
	jne	.LBB0_14
.Ltmp20:
	.loc	5 0 0 is_stmt 0
	movl	%edx, %esi
	andl	$15, %esi
.Ltmp21:
	.loc	5 186 20 is_stmt 1
	subq	%rsi, %rdx
	addq	$64, %rdx
.Ltmp22:
	.loc	5 0 20 is_stmt 0
.Ltmp23:
	.p2align	4
.LBB0_11:
	.loc	6 920 36 is_stmt 1
	vpcmpeqb	-48(%rdx), %xmm0, %k3
.Ltmp24:
	.loc	6 920 36 is_stmt 0
	vpcmpeqb	-32(%rdx), %xmm0, %k2
.Ltmp25:
	.loc	6 920 36
	vpcmpeqb	-16(%rdx), %xmm0, %k0
.Ltmp26:
	.loc	6 920 36
	vpcmpeqb	(%rdx), %xmm0, %k1
.Ltmp27:
	.loc	6 895 14 is_stmt 1
	korw	%k2, %k3, %k4
.Ltmp28:
	.loc	6 895 14 is_stmt 0
	korw	%k4, %k0, %k4
.Ltmp29:
	.loc	7 147 9 is_stmt 1
	kortestw	%k1, %k4
.Ltmp30:
	.loc	5 186 20
	jne	.LBB0_20
.Ltmp31:
	.loc	5 0 0 is_stmt 0
	leaq	-48(%rdx), %rsi
	.loc	5 172 19 is_stmt 1
	addq	$64, %rdx
	addq	$64, %rsi
	cmpq	%rax, %rsi
	jbe	.LBB0_11
	.loc	5 0 19 is_stmt 0
	addq	$-48, %rdx
	movq	%rdx, %rsi
.LBB0_14:
	leaq	-16(%rcx), %rax
	.loc	5 212 15 is_stmt 1
	cmpq	%rax, %rsi
	ja	.LBB0_17
	.loc	5 0 15 is_stmt 0
.Ltmp32:
	.p2align	4
.LBB0_15:
.Ltmp33:
	.loc	6 920 36 is_stmt 1
	vpcmpeqb	(%rsi), %xmm0, %k0
.Ltmp34:
	.loc	7 147 9
	kortestw	%k0, %k0
.Ltmp35:
	.loc	5 423 12
	jne	.LBB0_22
.Ltmp36:
	.loc	4 872 18
	addq	$16, %rsi
.Ltmp37:
	.loc	5 212 15
	cmpq	%rax, %rsi
	jbe	.LBB0_15
.LBB0_17:
	.loc	5 223 12
	cmpq	%rcx, %rsi
	jae	.LBB0_19
.Ltmp38:
	.loc	6 920 36
	vpcmpeqb	-16(%rcx), %xmm0, %k0
.Ltmp39:
	.loc	6 1570 9
	kmovw	%k0, %edx
.Ltmp40:
	.loc	7 147 9
	xorl	%eax, %eax
	kortestw	%k0, %k0
	setne	%al
.Ltmp41:
	.loc	5 423 12
	tzcntl	%edx, %edx
	addq	%rcx, %rdx
	addq	$-16, %rdx
.Ltmp42:
	.loc	1 145 10
	retq
.LBB0_19:
	.loc	1 0 10 is_stmt 0
	xorl	%eax, %eax
	.loc	1 145 10 is_stmt 1
	retq
.LBB0_20:
.Ltmp43:
	.loc	7 147 9
	kortestw	%k3, %k3
.Ltmp44:
	.loc	5 188 24
	je	.LBB0_23
.Ltmp45:
	.loc	5 0 0 is_stmt 0
	kmovd	%k3, %eax
.Ltmp46:
	.file	8 "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad" "library/core/src/num/uint_macros.rs"
	.loc	8 178 20 is_stmt 1
	tzcntl	%eax, %eax
.Ltmp47:
	.file	9 "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3" "src/lib.rs"
	.loc	9 0 0 is_stmt 0
	addq	%rax, %rdx
	addq	$-48, %rdx
	movl	$1, %eax
.Ltmp48:
	.loc	1 145 10 is_stmt 1
	retq
.LBB0_22:
.Ltmp49:
	.loc	5 0 0 is_stmt 0
	kmovd	%k0, %eax
.Ltmp50:
	.loc	8 178 20 is_stmt 1
	tzcntl	%eax, %eax
.Ltmp51:
	.loc	4 872 18
	addq	%rax, %rsi
	movl	$1, %eax
	movq	%rsi, %rdx
.Ltmp52:
	.loc	1 145 10
	retq
.LBB0_23:
.Ltmp53:
	.loc	7 147 9
	kortestw	%k2, %k2
.Ltmp54:
	.loc	5 193 24
	je	.LBB0_25
.Ltmp55:
	.loc	5 0 0 is_stmt 0
	kmovd	%k2, %eax
.Ltmp56:
	.loc	8 178 20 is_stmt 1
	tzcntl	%eax, %eax
.Ltmp57:
	.loc	9 0 0 is_stmt 0
	addq	%rax, %rdx
	addq	$-32, %rdx
	movl	$1, %eax
.Ltmp58:
	.loc	1 145 10 is_stmt 1
	retq
.LBB0_25:
.Ltmp59:
	.loc	7 147 9
	kortestw	%k0, %k0
.Ltmp60:
	.loc	5 198 24
	je	.LBB0_27
.Ltmp61:
	.loc	5 0 0 is_stmt 0
	kmovd	%k0, %eax
.Ltmp62:
	.loc	8 178 20 is_stmt 1
	tzcntl	%eax, %eax
.Ltmp63:
	.loc	9 0 0 is_stmt 0
	addq	%rax, %rdx
	addq	$-16, %rdx
	movl	$1, %eax
.Ltmp64:
	.loc	1 145 10 is_stmt 1
	retq
.LBB0_27:
.Ltmp65:
	.loc	6 1570 9
	kmovw	%k1, %eax
.Ltmp66:
	.loc	5 0 0 is_stmt 0
	tzcntl	%eax, %eax
	.loc	5 155 9 is_stmt 1
	addq	%rax, %rdx
	movl	$1, %eax
.Ltmp67:
	.loc	1 145 10
	retq
.Ltmp68:
.Lfunc_end0:
	.size	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect, .Lfunc_end0-_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect
	.cfi_endproc
	.file	10 "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3" "src/ext.rs"
	.file	11 "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad" "library/core/src/ops/function.rs"

	.section	.text._RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2,"ax",@progbits
	.prefalign	4, .Lfunc_end1, nop
	.type	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2,@function
_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2:
.Lfunc_begin1:
	.loc	1 90 0
	.cfi_startproc
	movq	%rdx, %rcx
.Ltmp69:
	.loc	3 161 12 prologue_end
	cmpq	%rdx, %rsi
	jae	.LBB1_19
	.loc	3 0 12 is_stmt 0
	movq	%rsi, %rdx
.Ltmp70:
	.loc	4 624 18 is_stmt 1
	movq	%rcx, %r8
	subq	%rsi, %r8
.Ltmp71:
	.loc	3 164 12
	cmpq	$15, %r8
	ja	.LBB1_6
.Ltmp72:
	.loc	5 1161 11
	addq	%rdx, %r8
	xorl	%eax, %eax
	.loc	5 0 11 is_stmt 0
.Ltmp73:
	.p2align	4
.LBB1_3:
.Ltmp74:
	.loc	3 167 17 is_stmt 1
	cmpb	%dil, (%rdx)
.Ltmp75:
	.loc	5 1162 12
	je	.LBB1_8
.Ltmp76:
	.loc	4 390 18
	incq	%rdx
.Ltmp77:
	.loc	5 1161 11
	cmpq	%rcx, %rdx
	jne	.LBB1_3
.Ltmp78:
	.loc	5 0 11 is_stmt 0
	movq	%r8, %rdx
	.loc	1 98 10 is_stmt 1
	retq
.LBB1_6:
	.loc	1 0 0 is_stmt 0
	movd	%edi, %xmm0
	punpcklbw	%xmm0, %xmm0
	pshuflw	$0, %xmm0, %xmm0
	pshufd	$68, %xmm0, %xmm0
.Ltmp79:
	.file	12 "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad" "library/core/src/ptr/mod.rs"
	.loc	12 574 14 is_stmt 1
	movdqu	(%rdx), %xmm1
.Ltmp80:
	.loc	6 1570 9
	pcmpeqb	%xmm0, %xmm1
	pmovmskb	%xmm1, %eax
.Ltmp81:
	.loc	7 147 9
	testl	%eax, %eax
.Ltmp82:
	.loc	5 423 12
	je	.LBB1_9
.Ltmp83:
	.loc	8 178 20
	rep		bsfl	%eax, %eax
.Ltmp84:
	.loc	4 872 18
	addq	%rax, %rdx
.Ltmp85:
.LBB1_8:
	.loc	4 0 18 is_stmt 0
	movl	$1, %eax
	.loc	1 98 10 is_stmt 1
	retq
.LBB1_9:
.Ltmp86:
	.loc	4 872 18
	movq	%rdx, %rsi
	andq	$-16, %rsi
	addq	$16, %rsi
.Ltmp87:
	.loc	5 171 12
	cmpq	$64, %r8
	setae	%dil
	leaq	-64(%rcx), %rax
	cmpq	%rax, %rsi
	setbe	%r8b
	andb	%dil, %r8b
	cmpb	$1, %r8b
	jne	.LBB1_14
.Ltmp88:
	.loc	5 0 0 is_stmt 0
	movl	%edx, %esi
	andl	$15, %esi
.Ltmp89:
	.loc	5 186 20 is_stmt 1
	subq	%rsi, %rdx
	addq	$64, %rdx
.Ltmp90:
	.loc	5 0 20 is_stmt 0
.Ltmp91:
	.p2align	4
.LBB1_11:
	movdqa	-48(%rdx), %xmm4
.Ltmp92:
	.loc	6 920 36 is_stmt 1
	pcmpeqb	%xmm0, %xmm4
	movdqa	-32(%rdx), %xmm3
.Ltmp93:
	.loc	6 920 36 is_stmt 0
	pcmpeqb	%xmm0, %xmm3
	movdqa	-16(%rdx), %xmm2
.Ltmp94:
	.loc	6 920 36
	pcmpeqb	%xmm0, %xmm2
	movdqa	(%rdx), %xmm1
.Ltmp95:
	.loc	6 920 36
	pcmpeqb	%xmm0, %xmm1
.Ltmp96:
	.loc	6 895 14 is_stmt 1
	movdqa	%xmm4, %xmm5
	por	%xmm3, %xmm5
.Ltmp97:
	.loc	6 895 14 is_stmt 0
	movdqa	%xmm2, %xmm6
	por	%xmm1, %xmm6
	por	%xmm5, %xmm6
.Ltmp98:
	.loc	6 1570 9 is_stmt 1
	pmovmskb	%xmm6, %esi
.Ltmp99:
	.loc	7 147 9
	testl	%esi, %esi
.Ltmp100:
	.loc	5 186 20
	jne	.LBB1_20
.Ltmp101:
	.loc	5 0 0 is_stmt 0
	leaq	-48(%rdx), %rsi
	.loc	5 172 19 is_stmt 1
	addq	$64, %rdx
	addq	$64, %rsi
	cmpq	%rax, %rsi
	jbe	.LBB1_11
	.loc	5 0 19 is_stmt 0
	addq	$-48, %rdx
	movq	%rdx, %rsi
.LBB1_14:
	leaq	-16(%rcx), %rax
	.loc	5 212 15 is_stmt 1
	cmpq	%rax, %rsi
	ja	.LBB1_17
	.loc	5 0 15 is_stmt 0
.Ltmp102:
	.p2align	4
.LBB1_15:
.Ltmp103:
	.loc	12 574 14 is_stmt 1
	movdqu	(%rsi), %xmm1
.Ltmp104:
	.loc	6 1570 9
	pcmpeqb	%xmm0, %xmm1
	pmovmskb	%xmm1, %edx
.Ltmp105:
	.loc	7 147 9
	testl	%edx, %edx
.Ltmp106:
	.loc	5 423 12
	jne	.LBB1_22
.Ltmp107:
	.loc	4 872 18
	addq	$16, %rsi
.Ltmp108:
	.loc	5 212 15
	cmpq	%rax, %rsi
	jbe	.LBB1_15
.LBB1_17:
	.loc	5 223 12
	cmpq	%rcx, %rsi
	jae	.LBB1_19
.Ltmp109:
	.loc	12 574 14
	movdqu	-16(%rcx), %xmm1
.Ltmp110:
	.loc	6 1570 9
	pcmpeqb	%xmm0, %xmm1
	pmovmskb	%xmm1, %edx
.Ltmp111:
	.loc	7 147 9
	xorl	%eax, %eax
	testl	%edx, %edx
	setne	%al
.Ltmp112:
	.loc	5 423 12
	movl	$32, %esi
	rep		bsfl	%edx, %esi
	leaq	(%rcx,%rsi), %rdx
	addq	$-16, %rdx
.Ltmp113:
	.loc	1 98 10
	retq
.LBB1_19:
	.loc	1 0 10 is_stmt 0
	xorl	%eax, %eax
	.loc	1 98 10 is_stmt 1
	retq
.LBB1_20:
.Ltmp114:
	.loc	6 1570 9
	pmovmskb	%xmm4, %eax
.Ltmp115:
	.loc	7 147 9
	testl	%eax, %eax
.Ltmp116:
	.loc	5 188 24
	je	.LBB1_23
.Ltmp117:
	.loc	8 178 20
	rep		bsfl	%eax, %eax
.Ltmp118:
	.loc	9 0 0 is_stmt 0
	addq	%rax, %rdx
	addq	$-48, %rdx
	movl	$1, %eax
.Ltmp119:
	.loc	1 98 10 is_stmt 1
	retq
.LBB1_22:
.Ltmp120:
	.loc	8 178 20
	rep		bsfl	%edx, %eax
.Ltmp121:
	.loc	4 872 18
	addq	%rax, %rsi
	movl	$1, %eax
	movq	%rsi, %rdx
.Ltmp122:
	.loc	1 98 10
	retq
.LBB1_23:
.Ltmp123:
	.loc	6 1570 9
	pmovmskb	%xmm3, %eax
.Ltmp124:
	.loc	7 147 9
	testl	%eax, %eax
.Ltmp125:
	.loc	5 193 24
	je	.LBB1_25
.Ltmp126:
	.loc	8 178 20
	rep		bsfl	%eax, %eax
.Ltmp127:
	.loc	9 0 0 is_stmt 0
	addq	%rax, %rdx
	addq	$-32, %rdx
	movl	$1, %eax
.Ltmp128:
	.loc	1 98 10 is_stmt 1
	retq
.LBB1_25:
.Ltmp129:
	.loc	6 1570 9
	pmovmskb	%xmm2, %eax
.Ltmp130:
	.loc	7 147 9
	testl	%eax, %eax
.Ltmp131:
	.loc	5 198 24
	je	.LBB1_27
.Ltmp132:
	.loc	8 178 20
	rep		bsfl	%eax, %eax
.Ltmp133:
	.loc	9 0 0 is_stmt 0
	addq	%rax, %rdx
	addq	$-16, %rdx
	movl	$1, %eax
.Ltmp134:
	.loc	1 98 10 is_stmt 1
	retq
.LBB1_27:
.Ltmp135:
	.loc	6 1570 9
	pmovmskb	%xmm1, %eax
.Ltmp136:
	.loc	8 178 20
	movl	$32, %ecx
	rep		bsfl	%eax, %ecx
.Ltmp137:
	.loc	9 0 0 is_stmt 0
	addq	%rcx, %rdx
	movl	$1, %eax
.Ltmp138:
	.loc	1 98 10 is_stmt 1
	retq
.Ltmp139:
.Lfunc_end1:
	.size	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2, .Lfunc_end1-_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2
	.cfi_endproc

	.type	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN,@object
	.section	.data._RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN,"aw",@progbits
	.globl	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN
	.p2align	3, 0x0
_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN:
	.quad	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect
	.size	_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN, 8

	.section	.debug_abbrev,"",@progbits
	.byte	1
	.byte	17
	.byte	1
	.byte	37
	.byte	14
	.byte	19
	.byte	5
	.byte	3
	.byte	14
	.byte	16
	.byte	23
	.byte	27
	.byte	14
	.byte	17
	.byte	1
	.byte	85
	.byte	23
	.byte	0
	.byte	0
	.byte	2
	.byte	57
	.byte	1
	.byte	3
	.byte	14
	.byte	0
	.byte	0
	.byte	3
	.byte	46
	.byte	0
	.byte	110
	.byte	14
	.byte	3
	.byte	14
	.byte	58
	.byte	11
	.byte	59
	.byte	5
	.byte	32
	.byte	11
	.byte	0
	.byte	0
	.byte	4
	.byte	46
	.byte	0
	.byte	110
	.byte	14
	.byte	3
	.byte	14
	.byte	58
	.byte	11
	.byte	59
	.byte	5
	.byte	54
	.byte	11
	.byte	32
	.byte	11
	.byte	0
	.byte	0
	.byte	5
	.byte	46
	.byte	0
	.byte	110
	.byte	14
	.byte	3
	.byte	14
	.byte	58
	.byte	11
	.byte	59
	.byte	11
	.byte	32
	.byte	11
	.byte	0
	.byte	0
	.byte	6
	.byte	46
	.byte	0
	.byte	110
	.byte	14
	.byte	3
	.byte	14
	.byte	58
	.byte	11
	.byte	59
	.byte	11
	.byte	54
	.byte	11
	.byte	32
	.byte	11
	.byte	0
	.byte	0
	.byte	7
	.byte	46
	.byte	1
	.byte	17
	.byte	1
	.byte	18
	.byte	6
	.byte	64
	.byte	24
	.byte	110
	.byte	14
	.byte	3
	.byte	14
	.byte	58
	.byte	11
	.byte	59
	.byte	11
	.byte	63
	.byte	25
	.byte	0
	.byte	0
	.byte	8
	.byte	29
	.byte	1
	.byte	49
	.byte	19
	.byte	17
	.byte	1
	.byte	18
	.byte	6
	.byte	88
	.byte	11
	.byte	89
	.byte	11
	.byte	87
	.byte	11
	.byte	0
	.byte	0
	.byte	9
	.byte	29
	.byte	0
	.byte	49
	.byte	19
	.byte	17
	.byte	1
	.byte	18
	.byte	6
	.byte	88
	.byte	11
	.byte	89
	.byte	5
	.byte	87
	.byte	11
	.byte	0
	.byte	0
	.byte	10
	.byte	29
	.byte	1
	.byte	49
	.byte	19
	.byte	85
	.byte	23
	.byte	88
	.byte	11
	.byte	89
	.byte	11
	.byte	87
	.byte	11
	.byte	0
	.byte	0
	.byte	11
	.byte	29
	.byte	0
	.byte	49
	.byte	19
	.byte	17
	.byte	1
	.byte	18
	.byte	6
	.byte	88
	.byte	11
	.byte	89
	.byte	11
	.byte	87
	.byte	11
	.byte	0
	.byte	0
	.byte	12
	.byte	29
	.byte	1
	.byte	49
	.byte	19
	.byte	85
	.byte	23
	.byte	88
	.byte	11
	.byte	89
	.byte	5
	.byte	87
	.byte	11
	.byte	0
	.byte	0
	.byte	13
	.byte	29
	.byte	1
	.byte	49
	.byte	19
	.byte	17
	.byte	1
	.byte	18
	.byte	6
	.byte	88
	.byte	11
	.byte	89
	.byte	5
	.byte	87
	.byte	11
	.byte	0
	.byte	0
	.byte	14
	.byte	46
	.byte	1
	.byte	17
	.byte	1
	.byte	18
	.byte	6
	.byte	64
	.byte	24
	.byte	49
	.byte	19
	.byte	0
	.byte	0
	.byte	0
	.section	.debug_info,"",@progbits
.Lcu_begin0:
	.long	.Ldebug_info_end0-.Ldebug_info_start0
.Ldebug_info_start0:
	.short	4
	.long	.debug_abbrev
	.byte	8
	.byte	1
	.long	.Linfo_string0
	.short	28
	.long	.Linfo_string1
	.long	.Lline_table_start0
	.long	.Linfo_string2
	.quad	0
	.long	.Ldebug_ranges7
	.byte	2
	.long	.Linfo_string3
	.byte	2
	.long	.Linfo_string4
	.byte	2
	.long	.Linfo_string5
	.byte	3
	.long	.Linfo_string6
	.long	.Linfo_string7
	.byte	2
	.short	4299
	.byte	1
	.byte	2
	.long	.Linfo_string8
	.byte	3
	.long	.Linfo_string9
	.long	.Linfo_string10
	.byte	2
	.short	1950
	.byte	1
	.byte	0
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string21
	.byte	2
	.long	.Linfo_string22
	.byte	2
	.long	.Linfo_string23
	.byte	3
	.long	.Linfo_string24
	.long	.Linfo_string25
	.byte	4
	.short	617
	.byte	1
	.byte	3
	.long	.Linfo_string34
	.long	.Linfo_string35
	.byte	4
	.short	355
	.byte	1
	.byte	3
	.long	.Linfo_string53
	.long	.Linfo_string54
	.byte	4
	.short	838
	.byte	1
	.byte	3
	.long	.Linfo_string53
	.long	.Linfo_string54
	.byte	4
	.short	838
	.byte	1
	.byte	0
	.byte	0
	.byte	3
	.long	.Linfo_string77
	.long	.Linfo_string78
	.byte	12
	.short	553
	.byte	1
	.byte	0
	.byte	2
	.long	.Linfo_string36
	.byte	2
	.long	.Linfo_string37
	.byte	2
	.long	.Linfo_string14
	.byte	4
	.long	.Linfo_string38
	.long	.Linfo_string39
	.byte	6
	.short	919
	.byte	3
	.byte	1
	.byte	4
	.long	.Linfo_string55
	.long	.Linfo_string56
	.byte	6
	.short	894
	.byte	3
	.byte	1
	.byte	4
	.long	.Linfo_string62
	.long	.Linfo_string63
	.byte	6
	.short	1566
	.byte	3
	.byte	1
	.byte	3
	.long	.Linfo_string79
	.long	.Linfo_string80
	.byte	6
	.short	1339
	.byte	1
	.byte	0
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string66
	.byte	2
	.long	.Linfo_string67
	.byte	5
	.long	.Linfo_string68
	.long	.Linfo_string69
	.byte	8
	.byte	177
	.byte	1
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string72
	.byte	2
	.long	.Linfo_string73
	.byte	2
	.long	.Linfo_string74
	.byte	6
	.long	.Linfo_string75
	.long	.Linfo_string76
	.byte	11
	.byte	79
	.byte	3
	.byte	1
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string11
	.byte	2
	.long	.Linfo_string12
	.byte	2
	.long	.Linfo_string13
	.byte	2
	.long	.Linfo_string14
	.byte	2
	.long	.Linfo_string11
	.byte	2
	.long	.Linfo_string15
	.byte	5
	.long	.Linfo_string16
	.long	.Linfo_string17
	.byte	3
	.byte	156
	.byte	1
	.byte	4
	.long	.Linfo_string48
	.long	.Linfo_string49
	.byte	3
	.short	285
	.byte	3
	.byte	1
	.byte	0
	.byte	2
	.long	.Linfo_string23
	.byte	2
	.long	.Linfo_string17
	.byte	6
	.long	.Linfo_string32
	.long	.Linfo_string33
	.byte	3
	.byte	166
	.byte	3
	.byte	1
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string11
	.byte	2
	.long	.Linfo_string18
	.byte	5
	.long	.Linfo_string19
	.long	.Linfo_string20
	.byte	1
	.byte	90
	.byte	1
	.byte	7
	.quad	.Lfunc_begin0
	.long	.Lfunc_end0-.Lfunc_begin0
	.byte	1
	.byte	87
	.long	.Linfo_string83
	.long	.Linfo_string84
	.byte	1
	.byte	109

	.byte	8
	.long	75
	.quad	.Ltmp0
	.long	.Ltmp1-.Ltmp0
	.byte	1
	.byte	138
	.byte	16
	.byte	9
	.long	57
	.quad	.Ltmp0
	.long	.Ltmp1-.Ltmp0
	.byte	2
	.short	1953
	.byte	13
	.byte	0
	.byte	10
	.long	397
	.long	.Ldebug_ranges0
	.byte	1
	.byte	144
	.byte	13
	.byte	10
	.long	333
	.long	.Ldebug_ranges1
	.byte	1
	.byte	97
	.byte	18
	.byte	8
	.long	3149
	.quad	.Ltmp2
	.long	.Ltmp3-.Ltmp2
	.byte	3
	.byte	164
	.byte	16
	.byte	11
	.long	106
	.quad	.Ltmp2
	.long	.Ltmp3-.Ltmp2
	.byte	10
	.byte	23
	.byte	30
	.byte	0
	.byte	8
	.long	3092
	.quad	.Ltmp4
	.long	.Ltmp10-.Ltmp4
	.byte	3
	.byte	166
	.byte	20
	.byte	9
	.long	370
	.quad	.Ltmp6
	.long	.Ltmp7-.Ltmp6
	.byte	5
	.short	1162
	.byte	12
	.byte	9
	.long	119
	.quad	.Ltmp8
	.long	.Ltmp9-.Ltmp8
	.byte	5
	.short	1165
	.byte	19
	.byte	0
	.byte	10
	.long	345
	.long	.Ldebug_ranges2
	.byte	3
	.byte	185
	.byte	14
	.byte	12
	.long	3123
	.long	.Ldebug_ranges2
	.byte	3
	.short	290
	.byte	16
	.byte	8
	.long	3110
	.quad	.Ltmp12
	.long	.Ltmp16-.Ltmp12
	.byte	5
	.byte	165
	.byte	33
	.byte	13
	.long	3178
	.quad	.Ltmp12
	.long	.Ltmp13-.Ltmp12
	.byte	5
	.short	422
	.byte	28
	.byte	11
	.long	189
	.quad	.Ltmp12
	.long	.Ltmp13-.Ltmp12
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	9
	.long	3233
	.quad	.Ltmp13
	.long	.Ltmp14-.Ltmp13
	.byte	5
	.short	423
	.byte	17
	.byte	0
	.byte	11
	.long	132
	.quad	.Ltmp18
	.long	.Ltmp19-.Ltmp18
	.byte	5
	.byte	169
	.byte	29
	.byte	8
	.long	3263
	.quad	.Ltmp29
	.long	.Ltmp30-.Ltmp29
	.byte	5
	.byte	186
	.byte	24
	.byte	11
	.long	3233
	.quad	.Ltmp29
	.long	.Ltmp30-.Ltmp29
	.byte	7
	.byte	64
	.byte	25
	.byte	0
	.byte	11
	.long	3233
	.quad	.Ltmp43
	.long	.Ltmp44-.Ltmp43
	.byte	5
	.byte	188
	.byte	29
	.byte	8
	.long	3245
	.quad	.Ltmp46
	.long	.Ltmp47-.Ltmp46
	.byte	5
	.byte	189
	.byte	45
	.byte	11
	.long	257
	.quad	.Ltmp46
	.long	.Ltmp47-.Ltmp46
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	11
	.long	3233
	.quad	.Ltmp53
	.long	.Ltmp54-.Ltmp53
	.byte	5
	.byte	193
	.byte	29
	.byte	8
	.long	3245
	.quad	.Ltmp56
	.long	.Ltmp57-.Ltmp56
	.byte	5
	.byte	194
	.byte	63
	.byte	11
	.long	257
	.quad	.Ltmp56
	.long	.Ltmp57-.Ltmp56
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	11
	.long	3233
	.quad	.Ltmp59
	.long	.Ltmp60-.Ltmp59
	.byte	5
	.byte	198
	.byte	29
	.byte	8
	.long	3245
	.quad	.Ltmp62
	.long	.Ltmp63-.Ltmp62
	.byte	5
	.byte	199
	.byte	63
	.byte	11
	.long	257
	.quad	.Ltmp62
	.long	.Ltmp63-.Ltmp62
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	8
	.long	3202
	.quad	.Ltmp65
	.long	.Ltmp66-.Ltmp65
	.byte	5
	.byte	202
	.byte	36
	.byte	11
	.long	217
	.quad	.Ltmp65
	.long	.Ltmp66-.Ltmp65
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	8
	.long	3190
	.quad	.Ltmp28
	.long	.Ltmp29-.Ltmp28
	.byte	5
	.byte	185
	.byte	31
	.byte	11
	.long	203
	.quad	.Ltmp28
	.long	.Ltmp29-.Ltmp28
	.byte	7
	.byte	238
	.byte	13
	.byte	0
	.byte	8
	.long	3190
	.quad	.Ltmp27
	.long	.Ltmp28-.Ltmp27
	.byte	5
	.byte	183
	.byte	31
	.byte	11
	.long	203
	.quad	.Ltmp27
	.long	.Ltmp28-.Ltmp27
	.byte	7
	.byte	238
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp26
	.long	.Ltmp27-.Ltmp26
	.byte	5
	.byte	182
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp26
	.long	.Ltmp27-.Ltmp26
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp25
	.long	.Ltmp26-.Ltmp25
	.byte	5
	.byte	181
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp25
	.long	.Ltmp26-.Ltmp25
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp24
	.long	.Ltmp25-.Ltmp24
	.byte	5
	.byte	180
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp24
	.long	.Ltmp25-.Ltmp24
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp22
	.long	.Ltmp24-.Ltmp22
	.byte	5
	.byte	179
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp22
	.long	.Ltmp24-.Ltmp22
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	10
	.long	3110
	.long	.Ldebug_ranges3
	.byte	5
	.byte	214
	.byte	37
	.byte	13
	.long	3178
	.quad	.Ltmp33
	.long	.Ltmp34-.Ltmp33
	.byte	5
	.short	422
	.byte	28
	.byte	11
	.long	189
	.quad	.Ltmp33
	.long	.Ltmp34-.Ltmp33
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	9
	.long	3233
	.quad	.Ltmp34
	.long	.Ltmp35-.Ltmp34
	.byte	5
	.short	423
	.byte	17
	.byte	13
	.long	286
	.quad	.Ltmp50
	.long	.Ltmp51-.Ltmp50
	.byte	5
	.short	424
	.byte	26
	.byte	8
	.long	3245
	.quad	.Ltmp50
	.long	.Ltmp51-.Ltmp50
	.byte	11
	.byte	79
	.byte	5
	.byte	11
	.long	257
	.quad	.Ltmp50
	.long	.Ltmp51-.Ltmp50
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	0
	.byte	9
	.long	145
	.quad	.Ltmp51
	.long	.Ltmp52-.Ltmp51
	.byte	5
	.short	424
	.byte	22
	.byte	0
	.byte	11
	.long	132
	.quad	.Ltmp36
	.long	.Ltmp37-.Ltmp36
	.byte	5
	.byte	217
	.byte	23
	.byte	8
	.long	3110
	.quad	.Ltmp38
	.long	.Ltmp42-.Ltmp38
	.byte	5
	.byte	227
	.byte	25
	.byte	13
	.long	3178
	.quad	.Ltmp38
	.long	.Ltmp39-.Ltmp38
	.byte	5
	.short	422
	.byte	28
	.byte	11
	.long	189
	.quad	.Ltmp38
	.long	.Ltmp39-.Ltmp38
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	13
	.long	3202
	.quad	.Ltmp39
	.long	.Ltmp40-.Ltmp39
	.byte	5
	.short	422
	.byte	41
	.byte	11
	.long	217
	.quad	.Ltmp39
	.long	.Ltmp40-.Ltmp39
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	9
	.long	3233
	.quad	.Ltmp40
	.long	.Ltmp41-.Ltmp40
	.byte	5
	.short	423
	.byte	17
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	14
	.quad	.Lfunc_begin1
	.long	.Lfunc_end1-.Lfunc_begin1
	.byte	1
	.byte	87
	.long	397
	.byte	10
	.long	333
	.long	.Ldebug_ranges4
	.byte	1
	.byte	97
	.byte	18
	.byte	8
	.long	3149
	.quad	.Ltmp70
	.long	.Ltmp71-.Ltmp70
	.byte	3
	.byte	164
	.byte	16
	.byte	11
	.long	106
	.quad	.Ltmp70
	.long	.Ltmp71-.Ltmp70
	.byte	10
	.byte	23
	.byte	30
	.byte	0
	.byte	8
	.long	3092
	.quad	.Ltmp72
	.long	.Ltmp78-.Ltmp72
	.byte	3
	.byte	166
	.byte	20
	.byte	9
	.long	370
	.quad	.Ltmp74
	.long	.Ltmp75-.Ltmp74
	.byte	5
	.short	1162
	.byte	12
	.byte	9
	.long	119
	.quad	.Ltmp76
	.long	.Ltmp77-.Ltmp76
	.byte	5
	.short	1165
	.byte	19
	.byte	0
	.byte	10
	.long	345
	.long	.Ldebug_ranges5
	.byte	3
	.byte	185
	.byte	14
	.byte	12
	.long	3123
	.long	.Ldebug_ranges5
	.byte	3
	.short	290
	.byte	16
	.byte	8
	.long	3110
	.quad	.Ltmp79
	.long	.Ltmp85-.Ltmp79
	.byte	5
	.byte	165
	.byte	33
	.byte	13
	.long	3214
	.quad	.Ltmp79
	.long	.Ltmp80-.Ltmp79
	.byte	5
	.short	421
	.byte	21
	.byte	8
	.long	231
	.quad	.Ltmp79
	.long	.Ltmp80-.Ltmp79
	.byte	7
	.byte	218
	.byte	13
	.byte	9
	.long	160
	.quad	.Ltmp79
	.long	.Ltmp80-.Ltmp79
	.byte	6
	.short	1341
	.byte	5
	.byte	0
	.byte	0
	.byte	13
	.long	3202
	.quad	.Ltmp80
	.long	.Ltmp81-.Ltmp80
	.byte	5
	.short	422
	.byte	41
	.byte	11
	.long	217
	.quad	.Ltmp80
	.long	.Ltmp81-.Ltmp80
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	9
	.long	3233
	.quad	.Ltmp81
	.long	.Ltmp82-.Ltmp81
	.byte	5
	.short	423
	.byte	17
	.byte	13
	.long	286
	.quad	.Ltmp83
	.long	.Ltmp84-.Ltmp83
	.byte	5
	.short	424
	.byte	26
	.byte	8
	.long	3245
	.quad	.Ltmp83
	.long	.Ltmp84-.Ltmp83
	.byte	11
	.byte	79
	.byte	5
	.byte	11
	.long	257
	.quad	.Ltmp83
	.long	.Ltmp84-.Ltmp83
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	0
	.byte	9
	.long	145
	.quad	.Ltmp84
	.long	.Ltmp85-.Ltmp84
	.byte	5
	.short	424
	.byte	22
	.byte	0
	.byte	11
	.long	132
	.quad	.Ltmp86
	.long	.Ltmp87-.Ltmp86
	.byte	5
	.byte	169
	.byte	29
	.byte	8
	.long	3263
	.quad	.Ltmp98
	.long	.Ltmp100-.Ltmp98
	.byte	5
	.byte	186
	.byte	24
	.byte	8
	.long	3202
	.quad	.Ltmp98
	.long	.Ltmp99-.Ltmp98
	.byte	7
	.byte	64
	.byte	14
	.byte	11
	.long	217
	.quad	.Ltmp98
	.long	.Ltmp99-.Ltmp98
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	11
	.long	3233
	.quad	.Ltmp99
	.long	.Ltmp100-.Ltmp99
	.byte	7
	.byte	64
	.byte	25
	.byte	0
	.byte	8
	.long	3202
	.quad	.Ltmp114
	.long	.Ltmp115-.Ltmp114
	.byte	5
	.byte	187
	.byte	36
	.byte	11
	.long	217
	.quad	.Ltmp114
	.long	.Ltmp115-.Ltmp114
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	11
	.long	3233
	.quad	.Ltmp115
	.long	.Ltmp116-.Ltmp115
	.byte	5
	.byte	188
	.byte	29
	.byte	8
	.long	3245
	.quad	.Ltmp117
	.long	.Ltmp118-.Ltmp117
	.byte	5
	.byte	189
	.byte	45
	.byte	11
	.long	257
	.quad	.Ltmp117
	.long	.Ltmp118-.Ltmp117
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	8
	.long	3202
	.quad	.Ltmp123
	.long	.Ltmp124-.Ltmp123
	.byte	5
	.byte	192
	.byte	36
	.byte	11
	.long	217
	.quad	.Ltmp123
	.long	.Ltmp124-.Ltmp123
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	11
	.long	3233
	.quad	.Ltmp124
	.long	.Ltmp125-.Ltmp124
	.byte	5
	.byte	193
	.byte	29
	.byte	8
	.long	3245
	.quad	.Ltmp126
	.long	.Ltmp127-.Ltmp126
	.byte	5
	.byte	194
	.byte	63
	.byte	11
	.long	257
	.quad	.Ltmp126
	.long	.Ltmp127-.Ltmp126
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	8
	.long	3202
	.quad	.Ltmp129
	.long	.Ltmp130-.Ltmp129
	.byte	5
	.byte	197
	.byte	36
	.byte	11
	.long	217
	.quad	.Ltmp129
	.long	.Ltmp130-.Ltmp129
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	11
	.long	3233
	.quad	.Ltmp130
	.long	.Ltmp131-.Ltmp130
	.byte	5
	.byte	198
	.byte	29
	.byte	8
	.long	3245
	.quad	.Ltmp132
	.long	.Ltmp133-.Ltmp132
	.byte	5
	.byte	199
	.byte	63
	.byte	11
	.long	257
	.quad	.Ltmp132
	.long	.Ltmp133-.Ltmp132
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	8
	.long	3202
	.quad	.Ltmp135
	.long	.Ltmp136-.Ltmp135
	.byte	5
	.byte	202
	.byte	36
	.byte	11
	.long	217
	.quad	.Ltmp135
	.long	.Ltmp136-.Ltmp135
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	8
	.long	3245
	.quad	.Ltmp136
	.long	.Ltmp137-.Ltmp136
	.byte	5
	.byte	204
	.byte	59
	.byte	11
	.long	257
	.quad	.Ltmp136
	.long	.Ltmp137-.Ltmp136
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	8
	.long	3190
	.quad	.Ltmp97
	.long	.Ltmp98-.Ltmp97
	.byte	5
	.byte	185
	.byte	31
	.byte	11
	.long	203
	.quad	.Ltmp97
	.long	.Ltmp98-.Ltmp97
	.byte	7
	.byte	238
	.byte	13
	.byte	0
	.byte	8
	.long	3190
	.quad	.Ltmp96
	.long	.Ltmp97-.Ltmp96
	.byte	5
	.byte	183
	.byte	31
	.byte	11
	.long	203
	.quad	.Ltmp96
	.long	.Ltmp97-.Ltmp96
	.byte	7
	.byte	238
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp95
	.long	.Ltmp96-.Ltmp95
	.byte	5
	.byte	182
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp95
	.long	.Ltmp96-.Ltmp95
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp94
	.long	.Ltmp95-.Ltmp94
	.byte	5
	.byte	181
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp94
	.long	.Ltmp95-.Ltmp94
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp93
	.long	.Ltmp94-.Ltmp93
	.byte	5
	.byte	180
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp93
	.long	.Ltmp94-.Ltmp93
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	8
	.long	3178
	.quad	.Ltmp92
	.long	.Ltmp93-.Ltmp92
	.byte	5
	.byte	179
	.byte	35
	.byte	11
	.long	189
	.quad	.Ltmp92
	.long	.Ltmp93-.Ltmp92
	.byte	7
	.byte	228
	.byte	13
	.byte	0
	.byte	10
	.long	3110
	.long	.Ldebug_ranges6
	.byte	5
	.byte	214
	.byte	37
	.byte	13
	.long	3214
	.quad	.Ltmp103
	.long	.Ltmp104-.Ltmp103
	.byte	5
	.short	421
	.byte	21
	.byte	8
	.long	231
	.quad	.Ltmp103
	.long	.Ltmp104-.Ltmp103
	.byte	7
	.byte	218
	.byte	13
	.byte	9
	.long	160
	.quad	.Ltmp103
	.long	.Ltmp104-.Ltmp103
	.byte	6
	.short	1341
	.byte	5
	.byte	0
	.byte	0
	.byte	13
	.long	3202
	.quad	.Ltmp104
	.long	.Ltmp105-.Ltmp104
	.byte	5
	.short	422
	.byte	41
	.byte	11
	.long	217
	.quad	.Ltmp104
	.long	.Ltmp105-.Ltmp104
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	9
	.long	3233
	.quad	.Ltmp105
	.long	.Ltmp106-.Ltmp105
	.byte	5
	.short	423
	.byte	17
	.byte	13
	.long	286
	.quad	.Ltmp120
	.long	.Ltmp121-.Ltmp120
	.byte	5
	.short	424
	.byte	26
	.byte	8
	.long	3245
	.quad	.Ltmp120
	.long	.Ltmp121-.Ltmp120
	.byte	11
	.byte	79
	.byte	5
	.byte	11
	.long	257
	.quad	.Ltmp120
	.long	.Ltmp121-.Ltmp120
	.byte	7
	.byte	178
	.byte	31
	.byte	0
	.byte	0
	.byte	9
	.long	145
	.quad	.Ltmp121
	.long	.Ltmp122-.Ltmp121
	.byte	5
	.short	424
	.byte	22
	.byte	0
	.byte	11
	.long	132
	.quad	.Ltmp107
	.long	.Ltmp108-.Ltmp107
	.byte	5
	.byte	217
	.byte	23
	.byte	8
	.long	3110
	.quad	.Ltmp109
	.long	.Ltmp113-.Ltmp109
	.byte	5
	.byte	227
	.byte	25
	.byte	13
	.long	3214
	.quad	.Ltmp109
	.long	.Ltmp110-.Ltmp109
	.byte	5
	.short	421
	.byte	21
	.byte	8
	.long	231
	.quad	.Ltmp109
	.long	.Ltmp110-.Ltmp109
	.byte	7
	.byte	218
	.byte	13
	.byte	9
	.long	160
	.quad	.Ltmp109
	.long	.Ltmp110-.Ltmp109
	.byte	6
	.short	1341
	.byte	5
	.byte	0
	.byte	0
	.byte	13
	.long	3202
	.quad	.Ltmp110
	.long	.Ltmp111-.Ltmp110
	.byte	5
	.short	422
	.byte	41
	.byte	11
	.long	217
	.quad	.Ltmp110
	.long	.Ltmp111-.Ltmp110
	.byte	7
	.byte	223
	.byte	30
	.byte	0
	.byte	9
	.long	3233
	.quad	.Ltmp111
	.long	.Ltmp112-.Ltmp111
	.byte	5
	.short	423
	.byte	17
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string29
	.byte	2
	.long	.Linfo_string11
	.byte	3
	.long	.Linfo_string30
	.long	.Linfo_string31
	.byte	5
	.short	1154
	.byte	1
	.byte	2
	.long	.Linfo_string15
	.byte	3
	.long	.Linfo_string44
	.long	.Linfo_string45
	.byte	5
	.short	416
	.byte	1
	.byte	5
	.long	.Linfo_string46
	.long	.Linfo_string47
	.byte	5
	.byte	143
	.byte	1
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string26
	.byte	2
	.long	.Linfo_string23
	.byte	5
	.long	.Linfo_string27
	.long	.Linfo_string28
	.byte	10
	.byte	21
	.byte	1
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string40
	.byte	2
	.long	.Linfo_string41
	.byte	2
	.long	.Linfo_string23
	.byte	5
	.long	.Linfo_string42
	.long	.Linfo_string43
	.byte	7
	.byte	227
	.byte	1
	.byte	5
	.long	.Linfo_string57
	.long	.Linfo_string58
	.byte	7
	.byte	237
	.byte	1
	.byte	5
	.long	.Linfo_string64
	.long	.Linfo_string65
	.byte	7
	.byte	222
	.byte	1
	.byte	5
	.long	.Linfo_string81
	.long	.Linfo_string82
	.byte	7
	.byte	217
	.byte	1
	.byte	0
	.byte	0
	.byte	2
	.long	.Linfo_string50
	.byte	5
	.long	.Linfo_string51
	.long	.Linfo_string52
	.byte	7
	.byte	146
	.byte	1
	.byte	5
	.long	.Linfo_string70
	.long	.Linfo_string71
	.byte	7
	.byte	171
	.byte	1
	.byte	0
	.byte	2
	.long	.Linfo_string59
	.byte	6
	.long	.Linfo_string60
	.long	.Linfo_string61
	.byte	7
	.byte	63
	.byte	3
	.byte	1
	.byte	0
	.byte	0
	.byte	0
	.byte	0
.Ldebug_info_end0:
	.section	.text._RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect,"ax",@progbits
.Lsec_end0:
	.section	.text._RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2,"ax",@progbits
.Lsec_end1:
	.section	.debug_aranges,"",@progbits
	.long	60
	.short	2
	.long	.Lcu_begin0
	.byte	8
	.byte	0
	.zero	4,255
	.quad	.Lfunc_begin0
	.quad	.Lsec_end0-.Lfunc_begin0
	.quad	.Lfunc_begin1
	.quad	.Lsec_end1-.Lfunc_begin1
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.Ltmp1
	.quad	.Ltmp10
	.quad	.Ltmp11
	.quad	.Ltmp17
	.quad	.Ltmp18
	.quad	.Ltmp42
	.quad	.Ltmp43
	.quad	.Ltmp48
	.quad	.Ltmp49
	.quad	.Ltmp52
	.quad	.Ltmp53
	.quad	.Ltmp58
	.quad	.Ltmp59
	.quad	.Ltmp64
	.quad	.Ltmp65
	.quad	.Ltmp67
	.quad	0
	.quad	0
.Ldebug_ranges1:
	.quad	.Ltmp1
	.quad	.Ltmp10
	.quad	.Ltmp12
	.quad	.Ltmp17
	.quad	.Ltmp18
	.quad	.Ltmp42
	.quad	.Ltmp43
	.quad	.Ltmp48
	.quad	.Ltmp49
	.quad	.Ltmp52
	.quad	.Ltmp53
	.quad	.Ltmp58
	.quad	.Ltmp59
	.quad	.Ltmp64
	.quad	.Ltmp65
	.quad	.Ltmp67
	.quad	0
	.quad	0
.Ldebug_ranges2:
	.quad	.Ltmp12
	.quad	.Ltmp17
	.quad	.Ltmp18
	.quad	.Ltmp42
	.quad	.Ltmp43
	.quad	.Ltmp48
	.quad	.Ltmp49
	.quad	.Ltmp52
	.quad	.Ltmp53
	.quad	.Ltmp58
	.quad	.Ltmp59
	.quad	.Ltmp64
	.quad	.Ltmp65
	.quad	.Ltmp67
	.quad	0
	.quad	0
.Ldebug_ranges3:
	.quad	.Ltmp33
	.quad	.Ltmp36
	.quad	.Ltmp49
	.quad	.Ltmp52
	.quad	0
	.quad	0
.Ldebug_ranges4:
	.quad	.Ltmp69
	.quad	.Ltmp78
	.quad	.Ltmp79
	.quad	.Ltmp85
	.quad	.Ltmp86
	.quad	.Ltmp113
	.quad	.Ltmp114
	.quad	.Ltmp119
	.quad	.Ltmp120
	.quad	.Ltmp122
	.quad	.Ltmp123
	.quad	.Ltmp128
	.quad	.Ltmp129
	.quad	.Ltmp134
	.quad	.Ltmp135
	.quad	.Ltmp138
	.quad	0
	.quad	0
.Ldebug_ranges5:
	.quad	.Ltmp79
	.quad	.Ltmp85
	.quad	.Ltmp86
	.quad	.Ltmp113
	.quad	.Ltmp114
	.quad	.Ltmp119
	.quad	.Ltmp120
	.quad	.Ltmp122
	.quad	.Ltmp123
	.quad	.Ltmp128
	.quad	.Ltmp129
	.quad	.Ltmp134
	.quad	.Ltmp135
	.quad	.Ltmp138
	.quad	0
	.quad	0
.Ldebug_ranges6:
	.quad	.Ltmp103
	.quad	.Ltmp107
	.quad	.Ltmp120
	.quad	.Ltmp122
	.quad	0
	.quad	0
.Ldebug_ranges7:
	.quad	.Lfunc_begin0
	.quad	.Lfunc_end0
	.quad	.Lfunc_begin1
	.quad	.Lfunc_end1
	.quad	0
	.quad	0
	.section	.debug_str,"MS",@progbits,1
.Linfo_string0:
	.asciz	"clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))"
.Linfo_string1:
	.asciz	"/cargo/registry/25cdd57fae9f0462/memchr-2.8.3/src/lib.rs/@/memchr.ecd88863d2af7566-cgu.0"
.Linfo_string2:
	.asciz	"/cargo/registry/25cdd57fae9f0462/memchr-2.8.3"
.Linfo_string3:
	.asciz	"core"
.Linfo_string4:
	.asciz	"sync"
.Linfo_string5:
	.asciz	"atomic"
.Linfo_string6:
	.asciz	"_RINvNtNtCs2k2z8Zem4rB_4core4sync6atomic12atomic_storeOuKb0_ECskkIW8vVChzC_6memchr"
.Linfo_string7:
	.asciz	"atomic_store<*mut (), false>"
.Linfo_string8:
	.asciz	"Atomic"
.Linfo_string9:
	.asciz	"_RNvMs3_NtNtCs2k2z8Zem4rB_4core4sync6atomicINtB5_6AtomicOuE5storeCskkIW8vVChzC_6memchr"
.Linfo_string10:
	.asciz	"store<()>"
.Linfo_string11:
	.asciz	"memchr"
.Linfo_string12:
	.asciz	"arch"
.Linfo_string13:
	.asciz	"x86_64"
.Linfo_string14:
	.asciz	"sse2"
.Linfo_string15:
	.asciz	"One"
.Linfo_string16:
	.asciz	"_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One8find_raw"
.Linfo_string17:
	.asciz	"find_raw"
.Linfo_string18:
	.asciz	"memchr_raw"
.Linfo_string19:
	.asciz	"_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2"
.Linfo_string20:
	.asciz	"find_sse2"
.Linfo_string21:
	.asciz	"ptr"
.Linfo_string22:
	.asciz	"const_ptr"
.Linfo_string23:
	.asciz	"{impl#0}"
.Linfo_string24:
	.asciz	"_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh11offset_fromCskkIW8vVChzC_6memchr"
.Linfo_string25:
	.asciz	"offset_from<u8>"
.Linfo_string26:
	.asciz	"ext"
.Linfo_string27:
	.asciz	"_RNvXNtCskkIW8vVChzC_6memchr3extPhNtB2_7Pointer8distanceB4_"
.Linfo_string28:
	.asciz	"distance<u8>"
.Linfo_string29:
	.asciz	"generic"
.Linfo_string30:
	.asciz	"_RINvNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchr16fwd_byte_by_byteNCNvMNtNtNtB6_6x86_644sse26memchrNtB1a_3One8find_raw0EB8_"
.Linfo_string31:
	.asciz	"fwd_byte_by_byte<memchr::arch::x86_64::sse2::memchr::{impl#0}::find_raw::{closure_env#0}>"
.Linfo_string32:
	.asciz	"_RNCNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB4_3One8find_raw0Bc_"
.Linfo_string33:
	.asciz	"{closure#0}"
.Linfo_string34:
	.asciz	"_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh6offsetCskkIW8vVChzC_6memchr"
.Linfo_string35:
	.asciz	"offset<u8>"
.Linfo_string36:
	.asciz	"core_arch"
.Linfo_string37:
	.asciz	"x86"
.Linfo_string38:
	.asciz	"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse214__mm_cmpeq_epi8"
.Linfo_string39:
	.asciz	"_mm_cmpeq_epi8"
.Linfo_string40:
	.asciz	"vector"
.Linfo_string41:
	.asciz	"x86sse2"
.Linfo_string42:
	.asciz	"_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector5cmpeq"
.Linfo_string43:
	.asciz	"cmpeq"
.Linfo_string44:
	.asciz	"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_"
.Linfo_string45:
	.asciz	"search_chunk<core::core_arch::x86::__m128i, fn(memchr::vector::SensibleMoveMask) -> usize>"
.Linfo_string46:
	.asciz	"_RNvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB2_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE8find_rawB8_"
.Linfo_string47:
	.asciz	"find_raw<core::core_arch::x86::__m128i>"
.Linfo_string48:
	.asciz	"_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One13find_raw_impl"
.Linfo_string49:
	.asciz	"find_raw_impl"
.Linfo_string50:
	.asciz	"{impl#1}"
.Linfo_string51:
	.asciz	"_RNvXs_NtCskkIW8vVChzC_6memchr6vectorNtB4_16SensibleMoveMaskNtB4_8MoveMask12has_non_zero"
.Linfo_string52:
	.asciz	"has_non_zero"
.Linfo_string53:
	.asciz	"_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh3addCskkIW8vVChzC_6memchr"
.Linfo_string54:
	.asciz	"add<u8>"
.Linfo_string55:
	.asciz	"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse212__mm_or_si128"
.Linfo_string56:
	.asciz	"_mm_or_si128"
.Linfo_string57:
	.asciz	"_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector2or"
.Linfo_string58:
	.asciz	"or"
.Linfo_string59:
	.asciz	"Vector"
.Linfo_string60:
	.asciz	"_RNvYNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtNtCskkIW8vVChzC_6memchr6vector6Vector27movemask_will_have_non_zeroBS_"
.Linfo_string61:
	.asciz	"movemask_will_have_non_zero<core::core_arch::x86::__m128i>"
.Linfo_string62:
	.asciz	"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse217__mm_movemask_epi8"
.Linfo_string63:
	.asciz	"_mm_movemask_epi8"
.Linfo_string64:
	.asciz	"_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector8movemask"
.Linfo_string65:
	.asciz	"movemask"
.Linfo_string66:
	.asciz	"num"
.Linfo_string67:
	.asciz	"{impl#8}"
.Linfo_string68:
	.asciz	"_RNvMs6_NtCs2k2z8Zem4rB_4core3numm14trailing_zeros"
.Linfo_string69:
	.asciz	"trailing_zeros"
.Linfo_string70:
	.asciz	"_RNvXs_NtCskkIW8vVChzC_6memchr6vectorNtB4_16SensibleMoveMaskNtB4_8MoveMask12first_offset"
.Linfo_string71:
	.asciz	"first_offset"
.Linfo_string72:
	.asciz	"ops"
.Linfo_string73:
	.asciz	"function"
.Linfo_string74:
	.asciz	"Fn"
.Linfo_string75:
	.asciz	"_RNvYNvYNtNtCskkIW8vVChzC_6memchr6vector16SensibleMoveMaskNtB7_8MoveMask12first_offsetINtNtNtCs2k2z8Zem4rB_4core3ops8function2FnTB5_EE4callB9_"
.Linfo_string76:
	.asciz	"call<fn(memchr::vector::SensibleMoveMask) -> usize, (memchr::vector::SensibleMoveMask)>"
.Linfo_string77:
	.asciz	"_RINvNtCs2k2z8Zem4rB_4core3ptr19copy_nonoverlappinghECskkIW8vVChzC_6memchr"
.Linfo_string78:
	.asciz	"copy_nonoverlapping<u8>"
.Linfo_string79:
	.asciz	"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"
.Linfo_string80:
	.asciz	"_mm_loadu_si128"
.Linfo_string81:
	.asciz	"_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector14load_unaligned"
.Linfo_string82:
	.asciz	"load_unaligned"
.Linfo_string83:
	.asciz	"_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect"
.Linfo_string84:
	.asciz	"detect"
	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits
	.section	.debug_line,"",@progbits
.Lline_table_start0:

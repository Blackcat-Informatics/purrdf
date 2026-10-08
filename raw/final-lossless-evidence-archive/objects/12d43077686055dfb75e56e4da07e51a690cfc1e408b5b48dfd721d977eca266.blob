	.att_syntax
	.file	"sha2.fb014c68ad287afc-cgu.0"
	.section	.rodata.cst16,"aM",@progbits,16
	.p2align	4, 0x0
.LCPI0_0:
	.byte	3
	.byte	2
	.byte	1
	.byte	0
	.byte	7
	.byte	6
	.byte	5
	.byte	4
	.byte	11
	.byte	10
	.byte	9
	.byte	8
	.byte	15
	.byte	14
	.byte	13
	.byte	12
.LCPI0_1:
	.long	1116352408
	.long	1899447441
	.long	3049323471
	.long	3921009573
.LCPI0_2:
	.long	961987163
	.long	1508970993
	.long	2453635748
	.long	2870763221
.LCPI0_3:
	.long	3624381080
	.long	310598401
	.long	607225278
	.long	1426881987
.LCPI0_4:
	.long	1925078388
	.long	2162078206
	.long	2614888103
	.long	3248222580
.LCPI0_5:
	.long	3835390401
	.long	4022224774
	.long	264347078
	.long	604807628
.LCPI0_6:
	.long	770255983
	.long	1249150122
	.long	1555081692
	.long	1996064986
.LCPI0_7:
	.long	2554220882
	.long	2821834349
	.long	2952996808
	.long	3210313671
.LCPI0_8:
	.long	3336571891
	.long	3584528711
	.long	113926993
	.long	338241895
.LCPI0_9:
	.long	666307205
	.long	773529912
	.long	1294757372
	.long	1396182291
.LCPI0_10:
	.long	1695183700
	.long	1986661051
	.long	2177026350
	.long	2456956037
.LCPI0_11:
	.long	2730485921
	.long	2820302411
	.long	3259730800
	.long	3345764771
.LCPI0_12:
	.long	3516065817
	.long	3600352804
	.long	4094571909
	.long	275423344
.LCPI0_13:
	.long	430227734
	.long	506948616
	.long	659060556
	.long	883997877
.LCPI0_14:
	.long	958139571
	.long	1322822218
	.long	1537002063
	.long	1747873779
.LCPI0_15:
	.long	1955562222
	.long	2024104815
	.long	2227730452
	.long	2361852424
.LCPI0_16:
	.long	2428436474
	.long	2756734187
	.long	3204031479
	.long	3329325298
	.section	.text._RNvNtCsly5PKRZWayS_4sha26sha25611compress256,"ax",@progbits
	.globl	_RNvNtCsly5PKRZWayS_4sha26sha25611compress256
	.prefalign	4, .Lfunc_end0, nop
	.type	_RNvNtCsly5PKRZWayS_4sha26sha25611compress256,@function
_RNvNtCsly5PKRZWayS_4sha26sha25611compress256:
	.cfi_startproc
	vmovups	(%rdi), %xmm0
	vmovups	16(%rdi), %xmm1
	vshufps	$17, %xmm0, %xmm1, %xmm4
	vshufps	$187, %xmm0, %xmm1, %xmm20
	testq	%rdx, %rdx
	je	.LBB0_3
	vmovdqa	.LCPI0_0(%rip), %xmm3
	vmovdqa64	.LCPI0_1(%rip), %xmm21
	vmovdqa64	.LCPI0_2(%rip), %xmm22
	vmovdqa64	.LCPI0_3(%rip), %xmm23
	vmovdqa64	.LCPI0_4(%rip), %xmm24
	vmovdqa64	.LCPI0_5(%rip), %xmm25
	vmovdqa64	.LCPI0_6(%rip), %xmm26
	vmovdqa	.LCPI0_7(%rip), %xmm10
	vmovdqa	.LCPI0_8(%rip), %xmm11
	vmovdqa	.LCPI0_9(%rip), %xmm12
	vmovdqa	.LCPI0_10(%rip), %xmm13
	vmovdqa	.LCPI0_11(%rip), %xmm14
	vmovdqa	.LCPI0_12(%rip), %xmm15
	vmovdqa64	.LCPI0_13(%rip), %xmm16
	vmovdqa64	.LCPI0_14(%rip), %xmm17
	vmovdqa64	.LCPI0_15(%rip), %xmm18
	vmovdqa64	.LCPI0_16(%rip), %xmm19
	shlq	$6, %rdx
	addq	%rsi, %rdx
	.p2align	4
.LBB0_2:
	vmovdqu	(%rsi), %xmm0
	vmovdqu	32(%rsi), %xmm2
	vmovdqu	16(%rsi), %xmm1
	vmovdqu	48(%rsi), %xmm5
	addq	$64, %rsi
	vpshufb	%xmm3, %xmm0, %xmm7
	vpshufb	%xmm3, %xmm2, %xmm6
	vmovaps	%xmm20, %xmm2
	vpshufb	%xmm3, %xmm1, %xmm8
	vmovaps	%xmm4, %xmm1
	vpshufb	%xmm3, %xmm5, %xmm5
	vpaddd	%xmm21, %xmm7, %xmm0
	sha256msg1	%xmm8, %xmm7
	sha256rnds2	%xmm0, %xmm4, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm22, %xmm8, %xmm0
	sha256msg1	%xmm6, %xmm8
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm23, %xmm6, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm24, %xmm5, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm6, %xmm5, %xmm0
	sha256msg1	%xmm5, %xmm6
	vpaddd	%xmm0, %xmm7, %xmm7
	sha256msg2	%xmm5, %xmm7
	vpaddd	%xmm25, %xmm7, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm5, %xmm7, %xmm0
	sha256msg1	%xmm7, %xmm5
	vpaddd	%xmm0, %xmm8, %xmm8
	sha256msg2	%xmm7, %xmm8
	vpaddd	%xmm26, %xmm8, %xmm0
	vpalignr	$4, %xmm7, %xmm8, %xmm9
	sha256msg1	%xmm8, %xmm7
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	vpaddd	%xmm6, %xmm9, %xmm6
	sha256msg2	%xmm8, %xmm6
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm6, %xmm10, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm8, %xmm6, %xmm0
	sha256msg1	%xmm6, %xmm8
	vpaddd	%xmm0, %xmm5, %xmm9
	sha256msg2	%xmm6, %xmm9
	vpaddd	%xmm11, %xmm9, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm6, %xmm9, %xmm0
	sha256msg1	%xmm9, %xmm6
	vpaddd	%xmm0, %xmm7, %xmm7
	sha256msg2	%xmm9, %xmm7
	vpaddd	%xmm7, %xmm12, %xmm0
	vpalignr	$4, %xmm9, %xmm7, %xmm5
	sha256msg1	%xmm7, %xmm9
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	vpaddd	%xmm5, %xmm8, %xmm5
	sha256msg2	%xmm7, %xmm5
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm5, %xmm13, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm7, %xmm5, %xmm0
	sha256msg1	%xmm5, %xmm7
	vpaddd	%xmm0, %xmm6, %xmm6
	sha256msg2	%xmm5, %xmm6
	vpaddd	%xmm6, %xmm14, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm5, %xmm6, %xmm0
	sha256msg1	%xmm6, %xmm5
	vpaddd	%xmm0, %xmm9, %xmm8
	sha256msg2	%xmm6, %xmm8
	vpaddd	%xmm15, %xmm8, %xmm0
	vpalignr	$4, %xmm6, %xmm8, %xmm9
	sha256msg1	%xmm8, %xmm6
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	vpaddd	%xmm7, %xmm9, %xmm7
	sha256msg2	%xmm8, %xmm7
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm16, %xmm7, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm8, %xmm7, %xmm0
	sha256msg1	%xmm7, %xmm8
	vpaddd	%xmm0, %xmm5, %xmm5
	sha256msg2	%xmm7, %xmm5
	vpaddd	%xmm17, %xmm5, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpalignr	$4, %xmm7, %xmm5, %xmm0
	vpaddd	%xmm0, %xmm6, %xmm6
	sha256msg2	%xmm5, %xmm6
	vpaddd	%xmm18, %xmm6, %xmm0
	vpalignr	$4, %xmm5, %xmm6, %xmm5
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	vpaddd	%xmm5, %xmm8, %xmm5
	sha256msg2	%xmm6, %xmm5
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm19, %xmm5, %xmm0
	sha256rnds2	%xmm0, %xmm1, %xmm2
	vpshufd	$14, %xmm0, %xmm0
	sha256rnds2	%xmm0, %xmm2, %xmm1
	vpaddd	%xmm20, %xmm2, %xmm20
	vpaddd	%xmm4, %xmm1, %xmm4
	cmpq	%rdx, %rsi
	jne	.LBB0_2
.LBB0_3:
	vshufps	$187, %xmm20, %xmm4, %xmm0
	vshufps	$17, %xmm20, %xmm4, %xmm1
	vmovups	%xmm0, (%rdi)
	vmovups	%xmm1, 16(%rdi)
	retq
.Lfunc_end0:
	.size	_RNvNtCsly5PKRZWayS_4sha26sha25611compress256, .Lfunc_end0-_RNvNtCsly5PKRZWayS_4sha26sha25611compress256
	.cfi_endproc

	.section	.rodata.cst16,"aM",@progbits,16
	.p2align	4, 0x0
.LCPI1_0:
	.byte	7
	.byte	6
	.byte	5
	.byte	4
	.byte	3
	.byte	2
	.byte	1
	.byte	0
	.byte	15
	.byte	14
	.byte	13
	.byte	12
	.byte	11
	.byte	10
	.byte	9
	.byte	8
.LCPI1_1:
	.quad	4794697086780616226
	.quad	8158064640168781261
.LCPI1_2:
	.quad	-5349999486874862801
	.quad	-1606136188198331460
.LCPI1_3:
	.quad	4131703408338449720
	.quad	6480981068601479193
.LCPI1_4:
	.quad	-7908458776815382629
	.quad	-6116909921290321640
.LCPI1_5:
	.quad	-2880145864133508542
	.quad	1334009975649890238
.LCPI1_6:
	.quad	2608012711638119052
	.quad	6128411473006802146
.LCPI1_7:
	.quad	8268148722764581231
	.quad	-9160688886553864527
.LCPI1_8:
	.quad	-7215885187991268811
	.quad	-4495734319001033068
	.section	.rodata.cst32,"aM",@progbits,32
	.p2align	5, 0x0
.LCPI1_9:
	.byte	7
	.byte	6
	.byte	5
	.byte	4
	.byte	3
	.byte	2
	.byte	1
	.byte	0
	.byte	15
	.byte	14
	.byte	13
	.byte	12
	.byte	11
	.byte	10
	.byte	9
	.byte	8
	.byte	7
	.byte	6
	.byte	5
	.byte	4
	.byte	3
	.byte	2
	.byte	1
	.byte	0
	.byte	15
	.byte	14
	.byte	13
	.byte	12
	.byte	11
	.byte	10
	.byte	9
	.byte	8
.LCPI1_10:
	.quad	4794697086780616226
	.quad	8158064640168781261
	.quad	4794697086780616226
	.quad	8158064640168781261
.LCPI1_11:
	.quad	-5349999486874862801
	.quad	-1606136188198331460
	.quad	-5349999486874862801
	.quad	-1606136188198331460
.LCPI1_12:
	.quad	4131703408338449720
	.quad	6480981068601479193
	.quad	4131703408338449720
	.quad	6480981068601479193
.LCPI1_13:
	.quad	-7908458776815382629
	.quad	-6116909921290321640
	.quad	-7908458776815382629
	.quad	-6116909921290321640
.LCPI1_14:
	.quad	-2880145864133508542
	.quad	1334009975649890238
	.quad	-2880145864133508542
	.quad	1334009975649890238
.LCPI1_15:
	.quad	2608012711638119052
	.quad	6128411473006802146
	.quad	2608012711638119052
	.quad	6128411473006802146
.LCPI1_16:
	.quad	8268148722764581231
	.quad	-9160688886553864527
	.quad	8268148722764581231
	.quad	-9160688886553864527
.LCPI1_17:
	.quad	-7215885187991268811
	.quad	-4495734319001033068
	.quad	-7215885187991268811
	.quad	-4495734319001033068
	.section	.text._RNvNtCsly5PKRZWayS_4sha26sha51211compress512,"ax",@progbits
	.globl	_RNvNtCsly5PKRZWayS_4sha26sha51211compress512
	.prefalign	4, .Lfunc_end1, nop
	.type	_RNvNtCsly5PKRZWayS_4sha26sha51211compress512,@function
_RNvNtCsly5PKRZWayS_4sha26sha51211compress512:
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
	subq	$776, %rsp
	.cfi_def_cfa_offset 832
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	xorl	%r10d, %r10d
	movq	%rsi, -24(%rsp)
	movq	%rdi, -32(%rsp)
	testb	$1, %dl
	je	.LBB1_1
	vmovdqu	(%rsi), %xmm0
	vmovdqa	.LCPI1_0(%rip), %xmm7
	movq	%rdx, -112(%rsp)
	movq	(%rdi), %r8
	movq	8(%rdi), %r12
	movq	16(%rdi), %r13
	movq	24(%rdi), %r9
	movq	32(%rdi), %rbx
	movq	40(%rdi), %r14
	movq	48(%rdi), %r11
	movq	56(%rdi), %rdx
	movl	$16, %eax
	leaq	.Lanon.deeb35d3f195dc5f9a0c48b58e3e09c8.0(%rip), %rcx
	movq	%rdx, -80(%rsp)
	movq	%rbx, -72(%rsp)
	movq	%r11, -96(%rsp)
	movq	%r14, -104(%rsp)
	movq	%r8, -48(%rsp)
	movq	%r12, -120(%rsp)
	movq	%r13, -56(%rsp)
	movq	%r9, -88(%rsp)
	vpshufb	%xmm7, %xmm0, %xmm0
	vpaddq	.LCPI1_1(%rip), %xmm0, %xmm1
	vmovdqa	%xmm1, 128(%rsp)
	vmovdqu	16(%rsi), %xmm1
	vpshufb	%xmm7, %xmm1, %xmm5
	vpaddq	.LCPI1_2(%rip), %xmm5, %xmm1
	vmovdqa	%xmm1, 144(%rsp)
	vmovdqu	32(%rsi), %xmm1
	vpshufb	%xmm7, %xmm1, %xmm4
	vpaddq	.LCPI1_3(%rip), %xmm4, %xmm1
	vmovdqa	%xmm1, 160(%rsp)
	vmovdqu	48(%rsi), %xmm1
	vpshufb	%xmm7, %xmm1, %xmm1
	vpaddq	.LCPI1_4(%rip), %xmm1, %xmm2
	vmovdqa	%xmm2, 176(%rsp)
	vmovdqu	64(%rsi), %xmm2
	vpshufb	%xmm7, %xmm2, %xmm2
	vpaddq	.LCPI1_5(%rip), %xmm2, %xmm3
	vmovdqa	%xmm3, 192(%rsp)
	vmovdqu	80(%rsi), %xmm3
	vpshufb	%xmm7, %xmm3, %xmm6
	vpaddq	.LCPI1_6(%rip), %xmm6, %xmm3
	vmovdqa	%xmm3, 208(%rsp)
	vmovdqu	96(%rsi), %xmm3
	vpshufb	%xmm7, %xmm3, %xmm3
	vpaddq	.LCPI1_7(%rip), %xmm3, %xmm8
	vmovdqa	%xmm8, 224(%rsp)
	vmovdqu	112(%rsi), %xmm8
	vpshufb	%xmm7, %xmm8, %xmm7
	vpaddq	.LCPI1_8(%rip), %xmm7, %xmm8
	vmovdqa	%xmm8, 240(%rsp)
	.p2align	4
.LBB1_19:
	leaq	(,%rax,8), %rsi
	movq	%r10, -40(%rsp)
	movq	%rax, -64(%rsp)
	xorl	%edi, %edi
	vmovdqa	%xmm5, %xmm8
	vmovdqa	%xmm6, %xmm9
	vmovdqa	%xmm4, %xmm10
	movq	%rdx, %r15
	.p2align	4
.LBB1_20:
	vmovdqa	%xmm3, %xmm6
	vmovdqa	%xmm7, %xmm3
	vmovdqa	%xmm1, %xmm4
	vmovdqa	%xmm2, %xmm1
	vmovdqa	%xmm9, %xmm2
	vmovdqa	%xmm0, %xmm7
	vmovdqa	%xmm8, %xmm0
	vpalignr	$8, %xmm7, %xmm8, %xmm8
	vpalignr	$8, %xmm1, %xmm9, %xmm9
	vmovdqa	%xmm10, %xmm5
	vpsrlq	$19, %xmm3, %xmm12
	movq	%r13, %r10
	movq	%r8, %r13
	leaq	(%rsi,%rdi,8), %r8
	movq	%r11, %rbp
	movq	%rbx, %r11
	movq	%r14, %rdx
	andq	%r11, %r14
	movq	%r9, %rax
	movq	%r12, %r9
	vpaddq	%xmm7, %xmm9, %xmm7
	vpsrlq	$1, %xmm8, %xmm10
	vpsrlq	$7, %xmm8, %xmm9
	vpsllq	$56, %xmm8, %xmm11
	vpternlogq	$150, %xmm9, %xmm10, %xmm11
	vpsrlq	$8, %xmm8, %xmm9
	vpsllq	$63, %xmm8, %xmm8
	vpsllq	$3, %xmm3, %xmm10
	vpternlogq	$150, %xmm9, %xmm11, %xmm8
	vpsrlq	$6, %xmm3, %xmm9
	vpternlogq	$150, %xmm9, %xmm10, %xmm12
	vpsllq	$45, %xmm3, %xmm9
	vpsrlq	$61, %xmm3, %xmm10
	vpternlogq	$150, %xmm9, %xmm12, %xmm10
	vmovdqa	%xmm6, %xmm9
	vpaddq	%xmm7, %xmm10, %xmm7
	vmovdqa	%xmm4, %xmm10
	vpaddq	%xmm7, %xmm8, %xmm7
	vpaddq	(%rcx,%r8), %xmm7, %xmm8
	rorxq	$14, %rbx, %r8
	rorxq	$18, %rbx, %rbx
	xorq	%r8, %rbx
	rorxq	$41, %r11, %r8
	xorq	%rbx, %r8
	andnq	%rbp, %r11, %rbx
	addq	136(%rsp,%rdi,8), %rbp
	orq	%rbx, %r14
	rorxq	$34, %r13, %rbx
	addq	%r15, %r14
	addq	%r8, %r14
	rorxq	$28, %r13, %r8
	addq	128(%rsp,%rdi,8), %r14
	xorq	%r8, %rbx
	rorxq	$39, %r13, %r8
	xorq	%rbx, %r8
	movq	%r10, %rbx
	xorq	%r12, %rbx
	movq	%r10, %r12
	andq	%r9, %r12
	andq	%r13, %rbx
	vmovdqa	%xmm8, 128(%rsp,%rdi,8)
	addq	$2, %rdi
	vmovdqa	%xmm5, %xmm8
	xorq	%rbx, %r12
	movq	%r11, %rbx
	addq	%r8, %r12
	addq	%r14, %r12
	addq	%rax, %r14
	rorxq	$14, %r14, %rax
	rorxq	$18, %r14, %r8
	rorxq	$41, %r14, %r15
	andq	%r14, %rbx
	xorq	%rax, %r8
	xorq	%r8, %r15
	andnq	%rdx, %r14, %r8
	orq	%r8, %rbx
	rorxq	$34, %r12, %r8
	addq	%rbx, %rbp
	rorxq	$28, %r12, %rbx
	addq	%r15, %rbp
	xorq	%rbx, %r8
	rorxq	$39, %r12, %r15
	movq	%r9, %rbx
	xorq	%r13, %rbx
	xorq	%r8, %r15
	movq	%r9, %r8
	andq	%r12, %rbx
	andq	%r13, %r8
	xorq	%rbx, %r8
	movq	%rbp, %rbx
	addq	%r10, %rbx
	addq	%r15, %r8
	movq	%rdx, %r15
	addq	%rbp, %r8
	cmpq	$16, %rdi
	jne	.LBB1_20
	movq	-40(%rsp), %r10
	movq	-64(%rsp), %rax
	incl	%r10d
	addq	%rdi, %rax
	cmpl	$4, %r10d
	jne	.LBB1_19
	xorl	%eax, %eax
	.p2align	4
.LBB1_16:
	movq	%r14, %r15
	movq	%r8, %rcx
	rorxq	$14, %rbx, %rsi
	rorxq	$18, %rbx, %rdi
	andnq	%r11, %rbx, %r8
	andq	%rbx, %r15
	rorxq	$41, %rbx, %r10
	orq	%r8, %r15
	xorq	%rsi, %rdi
	rorxq	$34, %rcx, %rsi
	rorxq	$39, %rcx, %r8
	addq	%rdx, %r15
	xorq	%rdi, %r10
	rorxq	$28, %rcx, %rdx
	addq	%r10, %r15
	addq	128(%rsp,%rax,8), %r15
	xorq	%rdx, %rsi
	xorq	%rsi, %r8
	movq	%r15, %rdx
	addq	%r9, %rdx
	rorxq	$14, %rdx, %rsi
	rorxq	$18, %rdx, %r9
	rorxq	$41, %rdx, %r10
	xorq	%rsi, %r9
	movq	%rbx, %rsi
	andq	%rdx, %rsi
	xorq	%r9, %r10
	andnq	%r14, %rdx, %r9
	orq	%r9, %rsi
	movq	%r13, %r9
	andq	%r12, %r9
	addq	%r11, %rsi
	addq	%r10, %rsi
	addq	136(%rsp,%rax,8), %rsi
	movq	%rsi, %r11
	addq	%r13, %r11
	xorq	%r12, %r13
	andq	%rcx, %r13
	rorxq	$18, %r11, %rbp
	xorq	%r13, %r9
	andnq	%rbx, %r11, %r13
	addq	%r8, %r9
	addq	%r15, %r9
	rorxq	$41, %r11, %r15
	rorxq	$28, %r9, %rdi
	rorxq	$34, %r9, %r10
	rorxq	$39, %r9, %r8
	xorq	%rdi, %r10
	rorxq	$14, %r11, %rdi
	xorq	%rdi, %rbp
	movq	%rdx, %rdi
	andq	%r11, %rdi
	xorq	%r10, %r8
	orq	%r13, %rdi
	xorq	%rbp, %r15
	movq	%r12, %r13
	andq	%rcx, %r13
	addq	%r14, %rdi
	addq	%r15, %rdi
	addq	144(%rsp,%rax,8), %rdi
	movq	%rdi, %r14
	addq	%r12, %r14
	xorq	%rcx, %r12
	andq	%r9, %r12
	rorxq	$18, %r14, %rbp
	rorxq	$41, %r14, %r15
	xorq	%r12, %r13
	andnq	%rdx, %r14, %r12
	addq	%r8, %r13
	addq	%rsi, %r13
	rorxq	$28, %r13, %rsi
	rorxq	$34, %r13, %r10
	rorxq	$39, %r13, %r8
	xorq	%rsi, %r10
	rorxq	$14, %r14, %rsi
	xorq	%rsi, %rbp
	movq	%r11, %rsi
	andq	%r14, %rsi
	xorq	%r10, %r8
	movq	%r9, %r10
	xorq	%r13, %r10
	orq	%r12, %rsi
	xorq	%rbp, %r15
	movq	%rcx, %r12
	andq	%r9, %r12
	addq	%rbx, %rsi
	addq	%r15, %rsi
	addq	152(%rsp,%rax,8), %rsi
	addq	$4, %rax
	movq	%rsi, %rbx
	addq	%rcx, %rbx
	xorq	%r9, %rcx
	andq	%r13, %rcx
	xorq	%rcx, %r12
	addq	%r8, %r12
	movq	%r9, %r8
	andq	%r13, %r8
	addq	%rdi, %r12
	rorxq	$28, %r12, %rcx
	rorxq	$34, %r12, %rdi
	andq	%r12, %r10
	rorxq	$39, %r12, %r15
	xorq	%rcx, %rdi
	xorq	%r10, %r8
	xorq	%rdi, %r15
	addq	%r15, %r8
	addq	%rsi, %r8
	cmpq	$16, %rax
	jne	.LBB1_16
	addq	-48(%rsp), %r8
	movq	-32(%rsp), %rdi
	addq	-120(%rsp), %r12
	addq	-56(%rsp), %r13
	addq	-88(%rsp), %r9
	addq	-72(%rsp), %rbx
	addq	-104(%rsp), %r14
	addq	-96(%rsp), %r11
	addq	-80(%rsp), %rdx
	movq	-24(%rsp), %rsi
	movl	$1, %r10d
	movq	%r8, (%rdi)
	movq	%r12, 8(%rdi)
	movq	%r13, 16(%rdi)
	movq	%r9, 24(%rdi)
	movq	%rbx, 32(%rdi)
	movq	%r14, 40(%rdi)
	movq	%r11, 48(%rdi)
	movq	%rdx, 56(%rdi)
	movq	-112(%rsp), %rdx
.LBB1_1:
	xorl	%ecx, %ecx
	subq	%r10, %rdx
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu64	%zmm0, 704(%rsp)
	vmovdqu64	%zmm0, 640(%rsp)
	vmovdqu64	%zmm0, 576(%rsp)
	vmovdqu64	%zmm0, 512(%rsp)
	vmovdqu64	%zmm0, 448(%rsp)
	vmovdqu64	%zmm0, 384(%rsp)
	vmovdqu64	%zmm0, 320(%rsp)
	vmovdqu64	%zmm0, 256(%rsp)
	cmovaeq	%rdx, %rcx
	movq	%rcx, %rax
	shrq	%rax
	subq	%rax, %rcx
	je	.LBB1_13
	vbroadcasti128	.LCPI1_0(%rip), %ymm0
	vbroadcasti128	.LCPI1_1(%rip), %ymm1
	vbroadcasti128	.LCPI1_2(%rip), %ymm2
	vbroadcasti128	.LCPI1_3(%rip), %ymm3
	vbroadcasti128	.LCPI1_4(%rip), %ymm4
	vbroadcasti128	.LCPI1_5(%rip), %ymm5
	vbroadcasti128	.LCPI1_6(%rip), %ymm6
	vbroadcasti128	.LCPI1_7(%rip), %ymm7
	vbroadcasti128	.LCPI1_8(%rip), %ymm8
	movq	(%rdi), %rax
	movq	8(%rdi), %rdx
	movq	16(%rdi), %r15
	movq	24(%rdi), %rbp
	movq	32(%rdi), %r8
	movq	40(%rdi), %r13
	movq	48(%rdi), %r14
	movq	56(%rdi), %rbx
	movq	%rdx, %rdi
	movq	%rax, -120(%rsp)
	.p2align	4
.LBB1_3:
	movq	%r10, %rax
	shlq	$7, %rax
	movq	%rcx, -8(%rsp)
	movq	-120(%rsp), %rcx
	movl	$1, %r11d
	movl	$16, %r12d
	leaq	256(%rsp), %r9
	movq	%r10, -40(%rsp)
	movq	%rbx, -96(%rsp)
	movq	%r8, -112(%rsp)
	movq	%r8, %r10
	movq	%r14, -80(%rsp)
	movq	%r13, -104(%rsp)
	movq	%rdi, -16(%rsp)
	movq	%r15, -48(%rsp)
	movq	%rbp, -88(%rsp)
	movq	%rbp, %rdx
	vmovdqu	(%rsi,%rax), %xmm9
	vinserti128	$1, 128(%rsi,%rax), %ymm9, %ymm9
	vpshufb	%ymm0, %ymm9, %ymm9
	vpaddq	%ymm1, %ymm9, %ymm10
	vmovdqa	%xmm10, (%rsp)
	vextracti128	$1, %ymm10, 128(%rsp)
	vmovdqu	16(%rsi,%rax), %xmm10
	vinserti128	$1, 144(%rsi,%rax), %ymm10, %ymm10
	vpshufb	%ymm0, %ymm10, %ymm12
	vpaddq	%ymm2, %ymm12, %ymm10
	vmovdqa	%xmm10, 16(%rsp)
	vextracti128	$1, %ymm10, 144(%rsp)
	vmovdqu	32(%rsi,%rax), %xmm10
	vinserti128	$1, 160(%rsi,%rax), %ymm10, %ymm10
	vpshufb	%ymm0, %ymm10, %ymm13
	vpaddq	%ymm3, %ymm13, %ymm10
	vmovdqa	%xmm10, 32(%rsp)
	vextracti128	$1, %ymm10, 160(%rsp)
	vmovdqu	48(%rsi,%rax), %xmm10
	vinserti128	$1, 176(%rsi,%rax), %ymm10, %ymm10
	vpshufb	%ymm0, %ymm10, %ymm10
	vpaddq	%ymm4, %ymm10, %ymm11
	vmovdqa	%xmm11, 48(%rsp)
	vextracti128	$1, %ymm11, 176(%rsp)
	vmovdqu	64(%rsi,%rax), %xmm11
	vinserti128	$1, 192(%rsi,%rax), %ymm11, %ymm11
	vpshufb	%ymm0, %ymm11, %ymm11
	vpaddq	%ymm5, %ymm11, %ymm14
	vmovdqa	%xmm14, 64(%rsp)
	vextracti128	$1, %ymm14, 192(%rsp)
	vmovdqu	80(%rsi,%rax), %xmm14
	vinserti128	$1, 208(%rsi,%rax), %ymm14, %ymm14
	vpshufb	%ymm0, %ymm14, %ymm15
	vpaddq	%ymm6, %ymm15, %ymm14
	vmovdqa	%xmm14, 80(%rsp)
	vextracti128	$1, %ymm14, 208(%rsp)
	vmovdqu	96(%rsi,%rax), %xmm14
	vinserti128	$1, 224(%rsi,%rax), %ymm14, %ymm14
	vpshufb	%ymm0, %ymm14, %ymm14
	vpaddq	%ymm7, %ymm14, %ymm16
	vmovdqa64	%xmm16, 96(%rsp)
	vextracti32x4	$1, %ymm16, 224(%rsp)
	vmovdqu64	112(%rsi,%rax), %xmm16
	vinserti32x4	$1, 240(%rsi,%rax), %ymm16, %ymm16
	movq	%r14, %rsi
	movq	%rdi, %r14
	movq	%r15, %rax
	vpshufb	%ymm0, %ymm16, %ymm16
	vpaddq	%ymm8, %ymm16, %ymm17
	vmovdqa64	%xmm17, 112(%rsp)
	vextracti32x4	$1, %ymm17, 240(%rsp)
	.p2align	4
.LBB1_4:
	leaq	(,%r12,8), %rdi
	movq	%r11, -72(%rsp)
	leaq	.Lanon.deeb35d3f195dc5f9a0c48b58e3e09c8.0(%rip), %r11
	movq	%r12, -56(%rsp)
	xorl	%r8d, %r8d
	vmovdqa64	%ymm12, %ymm17
	vmovdqa64	%ymm15, %ymm18
	vmovdqa64	%ymm13, %ymm19
	movq	%rbx, %rbp
	movq	%rdi, -64(%rsp)
	.p2align	4
.LBB1_5:
	movq	%rax, %r15
	movq	%rcx, %rax
	movq	-64(%rsp), %rcx
	movq	%rsi, %r12
	vmovdqa	%ymm14, %ymm15
	vmovdqa64	%ymm16, %ymm14
	vmovdqa64	%ymm9, %ymm16
	movq	%r10, %rsi
	vmovdqa64	%ymm17, %ymm9
	movq	%r13, %rbx
	andq	%rsi, %r13
	movq	%rdx, %rdi
	movq	%r14, %rdx
	vmovdqa	%ymm10, %ymm13
	vmovdqa	%ymm11, %ymm10
	vmovdqa64	%ymm18, %ymm11
	vpalignr	$8, %ymm16, %ymm9, %ymm18
	vmovdqa64	%ymm19, %ymm12
	vpalignr	$8, %ymm10, %ymm11, %ymm19
	vpaddq	%ymm19, %ymm16, %ymm16
	vpsrlq	$1, %ymm18, %ymm20
	vpsrlq	$7, %ymm18, %ymm19
	vpsllq	$56, %ymm18, %ymm21
	vpternlogq	$150, %ymm19, %ymm20, %ymm21
	vpsrlq	$8, %ymm18, %ymm19
	vpsllq	$63, %ymm18, %ymm18
	vpsllq	$3, %ymm14, %ymm20
	vpternlogq	$150, %ymm19, %ymm21, %ymm18
	vpsrlq	$6, %ymm14, %ymm19
	vpsrlq	$19, %ymm14, %ymm21
	leaq	(%rcx,%r8,8), %rcx
	vpternlogq	$150, %ymm19, %ymm20, %ymm21
	vpsllq	$45, %ymm14, %ymm19
	vpsrlq	$61, %ymm14, %ymm20
	vpternlogq	$150, %ymm19, %ymm21, %ymm20
	vmovdqa64	%ymm13, %ymm19
	vbroadcasti32x4	(%r11,%rcx), %ymm17
	rorxq	$14, %r10, %rcx
	rorxq	$18, %r10, %r10
	vpaddq	%ymm20, %ymm16, %ymm16
	xorq	%rcx, %r10
	rorxq	$41, %rsi, %rcx
	vpaddq	%ymm18, %ymm16, %ymm16
	vmovdqa64	%ymm15, %ymm18
	xorq	%r10, %rcx
	andnq	%r12, %rsi, %r10
	addq	8(%rsp,%r8,8), %r12
	orq	%r10, %r13
	rorxq	$34, %rax, %r10
	addq	%rbp, %r13
	movq	%rdx, %rbp
	xorq	%rax, %rbp
	addq	%rcx, %r13
	rorxq	$28, %rax, %rcx
	addq	(%rsp,%r8,8), %r13
	xorq	%rcx, %r10
	rorxq	$39, %rax, %rcx
	xorq	%r10, %rcx
	movq	%r15, %r10
	xorq	%r14, %r10
	movq	%r15, %r14
	andq	%rdx, %r14
	andq	%rax, %r10
	vpaddq	%ymm17, %ymm16, %ymm17
	xorq	%r10, %r14
	movq	%rsi, %r10
	vmovdqa64	%xmm17, (%rsp,%r8,8)
	vextracti32x4	$1, %ymm17, (%r9,%r8,8)
	addq	$2, %r8
	vmovdqa64	%ymm12, %ymm17
	addq	%rcx, %r14
	addq	%r13, %r14
	addq	%rdi, %r13
	rorxq	$14, %r13, %rcx
	rorxq	$18, %r13, %rdi
	andq	%r13, %r10
	andq	%r14, %rbp
	xorq	%rcx, %rdi
	rorxq	$41, %r13, %rcx
	xorq	%rdi, %rcx
	andnq	%rbx, %r13, %rdi
	orq	%rdi, %r10
	rorxq	$34, %r14, %rdi
	addq	%r10, %r12
	rorxq	$39, %r14, %r10
	addq	%rcx, %r12
	rorxq	$28, %r14, %rcx
	xorq	%rcx, %rdi
	movq	%rdx, %rcx
	andq	%rax, %rcx
	xorq	%rbp, %rcx
	xorq	%rdi, %r10
	movq	%rbx, %rbp
	addq	%r10, %rcx
	movq	%r12, %r10
	addq	%r15, %r10
	addq	%r12, %rcx
	cmpq	$16, %r8
	jne	.LBB1_5
	movq	-72(%rsp), %r11
	movq	-56(%rsp), %r12
	subq	$-128, %r9
	incq	%r11
	addq	%r8, %r12
	cmpq	$5, %r11
	jne	.LBB1_4
	xorl	%r8d, %r8d
	.p2align	4
.LBB1_8:
	rorxq	$14, %r10, %rdi
	rorxq	$18, %r10, %r9
	andnq	%rsi, %r10, %r15
	rorxq	$41, %r10, %r11
	xorq	%rdi, %r9
	movq	%r13, %rdi
	andq	%r10, %rdi
	orq	%r15, %rdi
	xorq	%r9, %r11
	rorxq	$28, %rcx, %r9
	addq	%rbx, %rdi
	addq	%r11, %rdi
	addq	(%rsp,%r8,8), %rdi
	rorxq	$34, %rcx, %r11
	xorq	%r9, %r11
	rorxq	$39, %rcx, %r9
	xorq	%r11, %r9
	movq	%rdi, %rbx
	addq	%rdx, %rbx
	rorxq	$14, %rbx, %rdx
	rorxq	$18, %rbx, %r11
	rorxq	$41, %rbx, %r15
	movq	%rbx, %r12
	xorq	%rdx, %r11
	andnq	%r13, %rbx, %rdx
	xorq	%r11, %r15
	movq	%r10, %r11
	andq	%rbx, %r11
	orq	%rdx, %r11
	movq	%rax, %rdx
	andq	%r14, %rdx
	addq	%rsi, %r11
	addq	%r15, %r11
	addq	8(%rsp,%r8,8), %r11
	movq	%r11, %rsi
	addq	%rax, %rsi
	xorq	%r14, %rax
	andq	%rcx, %rax
	rorxq	$18, %rsi, %rbp
	andq	%rsi, %r12
	rorxq	$41, %rsi, %r15
	xorq	%rax, %rdx
	addq	%r9, %rdx
	addq	%rdi, %rdx
	rorxq	$28, %rdx, %rax
	rorxq	$34, %rdx, %rdi
	rorxq	$39, %rdx, %r9
	xorq	%rax, %rdi
	rorxq	$14, %rsi, %rax
	xorq	%rax, %rbp
	andnq	%r10, %rsi, %rax
	xorq	%rdi, %r9
	orq	%rax, %r12
	xorq	%rbp, %r15
	movq	%r14, %rax
	andq	%rcx, %rax
	addq	%r13, %r12
	addq	%r15, %r12
	addq	16(%rsp,%r8,8), %r12
	movq	%r12, %r13
	addq	%r14, %r13
	xorq	%rcx, %r14
	andq	%rdx, %r14
	rorxq	$41, %r13, %r15
	xorq	%r14, %rax
	addq	%r9, %rax
	addq	%r11, %rax
	rorxq	$28, %rax, %r9
	rorxq	$34, %rax, %r14
	rorxq	$39, %rax, %r11
	xorq	%r9, %r14
	rorxq	$14, %r13, %r9
	xorq	%r14, %r11
	rorxq	$18, %r13, %r14
	xorq	%r9, %r14
	movq	%rsi, %r9
	andq	%r13, %r9
	xorq	%r14, %r15
	andnq	%rbx, %r13, %r14
	orq	%r14, %r9
	movq	%rcx, %r14
	andq	%rdx, %r14
	addq	%r10, %r9
	addq	%r15, %r9
	addq	24(%rsp,%r8,8), %r9
	movq	%rdx, %r15
	xorq	%rax, %r15
	addq	$4, %r8
	movq	%r9, %r10
	addq	%rcx, %r10
	xorq	%rdx, %rcx
	andq	%rax, %rcx
	xorq	%rcx, %r14
	addq	%r11, %r14
	addq	%r12, %r14
	rorxq	$28, %r14, %rcx
	rorxq	$34, %r14, %rdi
	rorxq	$39, %r14, %r11
	andq	%r14, %r15
	xorq	%rcx, %rdi
	movq	%rdx, %rcx
	andq	%rax, %rcx
	xorq	%rdi, %r11
	xorq	%r15, %rcx
	addq	%r11, %rcx
	addq	%r9, %rcx
	cmpq	$16, %r8
	jne	.LBB1_8
	addq	-112(%rsp), %r10
	addq	-104(%rsp), %r13
	addq	-96(%rsp), %rbx
	addq	-120(%rsp), %rcx
	addq	-16(%rsp), %r14
	addq	-48(%rsp), %rax
	addq	-88(%rsp), %rdx
	addq	-80(%rsp), %rsi
	movl	$24, %r11d
	movq	%rbx, -96(%rsp)
	movq	%r10, -112(%rsp)
	movq	%rsi, %r9
	movq	%r13, -104(%rsp)
	movq	%rcx, %r12
	movq	%r14, %rdi
	movq	%rax, %r15
	movq	%rdx, %rbp
	.p2align	4
.LBB1_10:
	movq	%rbp, -48(%rsp)
	movq	-112(%rsp), %rbp
	movq	%r12, %r8
	movq	%r15, -120(%rsp)
	movq	%rdi, -56(%rsp)
	movq	%r9, -80(%rsp)
	movq	%r11, -64(%rsp)
	movq	%r8, -72(%rsp)
	movq	%rbp, %r12
	movq	%rbp, -112(%rsp)
	rorxq	$14, %rbp, %r15
	rorxq	$18, %rbp, %rbp
	rorxq	$41, %r12, %rdi
	xorq	%r15, %rbp
	movq	-104(%rsp), %r15
	xorq	%rbp, %rdi
	andnq	%r9, %r12, %rbp
	movq	%r11, %r9
	andq	%r12, %r15
	orq	%rbp, %r15
	addq	-96(%rsp), %r15
	movq	-112(%rsp), %rbp
	addq	%rdi, %r15
	addq	104(%rsp,%r11), %r15
	movq	%r8, %r11
	rorxq	$28, %r8, %rdi
	rorxq	$34, %r8, %r8
	xorq	%rdi, %r8
	rorxq	$39, %r11, %rdi
	movq	%rbp, %r11
	xorq	%r8, %rdi
	movq	%rdi, -88(%rsp)
	movq	%r15, %r12
	addq	-48(%rsp), %r12
	rorxq	$14, %r12, %rdi
	rorxq	$18, %r12, %r8
	andq	%r12, %r11
	movq	%r12, -96(%rsp)
	xorq	%rdi, %r8
	rorxq	$41, %r12, %rdi
	xorq	%r8, %rdi
	andnq	-104(%rsp), %r12, %r8
	movq	-120(%rsp), %r12
	orq	%r8, %r11
	addq	-80(%rsp), %r11
	movq	%r12, %r8
	addq	%rdi, %r11
	addq	112(%rsp,%r9), %r11
	movq	-56(%rsp), %rdi
	movq	%r11, %r9
	addq	%r12, %r9
	xorq	%rdi, %r12
	andq	-72(%rsp), %r12
	andq	%rdi, %r8
	movq	%r9, -80(%rsp)
	xorq	%r12, %r8
	movq	%r8, %r12
	addq	-88(%rsp), %r12
	addq	%r15, %r12
	andnq	%rbp, %r9, %r15
	movq	-96(%rsp), %rbp
	rorxq	$28, %r12, %rdi
	rorxq	$34, %r12, %r8
	movq	%r12, -88(%rsp)
	xorq	%rdi, %r8
	rorxq	$39, %r12, %rdi
	xorq	%r8, %rdi
	rorxq	$18, %r9, %r8
	movq	%rdi, -120(%rsp)
	rorxq	$14, %r9, %rdi
	andq	%r9, %rbp
	xorq	%rdi, %r8
	rorxq	$41, %r9, %rdi
	movq	-72(%rsp), %r9
	orq	%r15, %rbp
	addq	-104(%rsp), %rbp
	xorq	%r8, %rdi
	movq	-56(%rsp), %r8
	addq	%rdi, %rbp
	movq	-64(%rsp), %rdi
	movq	%r8, %r15
	andq	%r9, %r15
	addq	120(%rsp,%rdi), %rbp
	movq	%r8, %rdi
	xorq	%r9, %rdi
	andq	-88(%rsp), %rdi
	xorq	%rdi, %r15
	addq	-120(%rsp), %r15
	movq	%rbp, %r12
	addq	%r8, %r12
	rorxq	$41, %r12, %r9
	movq	%r12, -104(%rsp)
	addq	%r11, %r15
	rorxq	$28, %r15, %rdi
	rorxq	$34, %r15, %r8
	rorxq	$39, %r15, %r11
	xorq	%rdi, %r8
	rorxq	$14, %r12, %rdi
	xorq	%r8, %r11
	rorxq	$18, %r12, %r8
	xorq	%rdi, %r8
	andnq	-96(%rsp), %r12, %rdi
	xorq	%r8, %r9
	movq	-80(%rsp), %r8
	andq	%r12, %r8
	movq	-72(%rsp), %r12
	orq	%rdi, %r8
	addq	-112(%rsp), %r8
	movq	%r12, %rdi
	addq	%r9, %r8
	movq	-64(%rsp), %r9
	addq	128(%rsp,%r9), %r8
	movq	%r8, %r9
	addq	%r12, %r9
	movq	%r9, -112(%rsp)
	movq	%r12, %r9
	movq	-88(%rsp), %r12
	xorq	%r12, %r9
	andq	%r12, %rdi
	andq	%r15, %r9
	xorq	%r9, %rdi
	addq	%r11, %rdi
	addq	%rbp, %rdi
	movq	%r12, %rbp
	rorxq	$28, %rdi, %r11
	rorxq	$34, %rdi, %r9
	xorq	%r11, %r9
	rorxq	$39, %rdi, %r11
	xorq	%r9, %r11
	movq	%r12, %r9
	xorq	%r15, %r9
	andq	%r15, %r12
	andq	%rdi, %r9
	xorq	%r9, %r12
	movq	-80(%rsp), %r9
	addq	%r11, %r12
	addq	%r8, %r12
	movq	-64(%rsp), %r8
	addq	$32, %r8
	movq	%r8, %r11
	cmpq	$664, %r8
	jne	.LBB1_10
	movq	-112(%rsp), %r8
	addq	%rcx, %r12
	movq	-96(%rsp), %r11
	addq	%rax, %r15
	movq	-104(%rsp), %rax
	movq	-8(%rsp), %rcx
	addq	%rsi, %r9
	movq	-24(%rsp), %rsi
	addq	%r14, %rdi
	addq	%rdx, %rbp
	movq	%r12, -120(%rsp)
	movq	%r9, %r14
	addq	%r10, %r8
	movq	-40(%rsp), %r10
	addq	%r13, %rax
	addq	%rbx, %r11
	movq	%r11, %rbx
	movq	%rax, %r13
	addq	$2, %r10
	decq	%rcx
	jne	.LBB1_3
	movq	-32(%rsp), %rax
	movq	-120(%rsp), %rcx
	movq	%rcx, (%rax)
	movq	%rdi, 8(%rax)
	movq	%r15, 16(%rax)
	movq	%rbp, 24(%rax)
	movq	%r8, 32(%rax)
	movq	%r13, 40(%rax)
	movq	%r14, 48(%rax)
	movq	%rbx, 56(%rax)
.LBB1_13:
	addq	$776, %rsp
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
.Lfunc_end1:
	.size	_RNvNtCsly5PKRZWayS_4sha26sha51211compress512, .Lfunc_end1-_RNvNtCsly5PKRZWayS_4sha26sha51211compress512
	.cfi_endproc

	.type	.Lanon.deeb35d3f195dc5f9a0c48b58e3e09c8.0,@object
	.section	.rodata..Lanon.deeb35d3f195dc5f9a0c48b58e3e09c8.0,"a",@progbits
	.p2align	3, 0x0
.Lanon.deeb35d3f195dc5f9a0c48b58e3e09c8.0:
	.ascii	"\"\256(\327\230/\212B\315e\357#\221D7q/;M\354\317\373\300\265\274\333\211\201\245\333\265\3518\265H\363[\302V9\031\320\005\266\361\021\361Y\233O\031\257\244\202?\222\030\201m\332\325^\034\253B\002\003\243\230\252\007\330\276opE\001[\203\022\214\262\344N\276\2051$\342\264\377\325\303}\fUo\211{\362t]\276r\261\226\026;\376\261\336\2005\022\307%\247\006\334\233\224&i\317t\361\233\301\322J\361\236\301i\233\344\343%O8\206G\276\357\265\325\214\213\306\235\301\017e\234\254w\314\241\f$u\002+Yo,\351-\203\344\246n\252\204tJ\324\373A\275\334\251\260\\\265S\021\203\332\210\371v\253\337f\356RQ>\230\0202\264-m\3061\250?!\373\230\310'\003\260\344\016\357\276\307\177Y\277\302\217\250=\363\013\340\306%\247\n\223G\221\247\325o\202\003\340Qc\312\006pn\016\ng))\024\374/\322F\205\n\267'&\311&\\8!\033.\355*\304Z\374m,M\337\263\225\235\023\r8S\336c\257\213Ts\ne\250\262w<\273\njv\346\256\355G.\311\302\201;5\202\024\205,r\222d\003\361L\241\350\277\242\0010B\274Kf\032\250\221\227\370\320p\213K\3020\276T\006\243Ql\307\030R\357\326\031\350\222\321\020\251eU$\006\231\326* qW\2055\016\364\270\321\2732p\240j\020\310\320\322\270\026\301\244\031S\253AQ\bl7\036\231\353\216\337LwH'\250H\233\341\265\274\2604cZ\311\305\263\f\0349\313\212A\343J\252\330Ns\343cwO\312\234[\243\270\262\326\363o.h\374\262\357]\356\202\217t`/\027Coc\245xr\253\360\241\024x\310\204\3549d\032\b\002\307\214(\036c#\372\377\276\220\351\275\202\336\353lP\244\025y\306\262\367\243\371\276+Sr\343\362xq\306\234a&\352\316>'\312\007\302\300!\307\270\206\321\036\353\340\315\326}\332\352x\321n\356\177O}\365\272o\027r\252g\360\006\246\230\310\242\305}c\n\256\r\371\276\004\230?\021\033G\034\0235\013q\033\204}\004#\365w\333(\223$\307@{\253\3122\274\276\311\025\n\276\236<L\r\020\234\304g\035C\266B>\313\276\324\305L*~e\374\234)\177Y\354\372\326:\253o\313_\027XGJ\214\031Dl"
	.size	.Lanon.deeb35d3f195dc5f9a0c48b58e3e09c8.0, 640

	.ident	"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"
	.section	".note.GNU-stack","",@progbits

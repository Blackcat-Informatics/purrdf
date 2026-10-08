purrdf_sparql_eval::modifier::eval_graph_with::<purrdf_core::ir::dataset::RdfDataset, ()>:
.Lfunc_begin361:
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
	subq	$1240, %rsp
	.cfi_def_cfa_offset 1296
	.cfi_offset %rbx, -56
	.cfi_offset %r12, -48
	.cfi_offset %r13, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	cmpb	$0, (%rdx)
	movq	%r8, %r14
	movq	%rdx, %rbx
	movq	%rcx, 160(%rsp)
	movq	%rdi, 32(%rsp)
	movq	%r8, 312(%rsp)
	je	.LBB361_16
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	792(%rsp), %rdi
	callq	*%rax
	movq	664(%r14), %rax
	movl	$4, %edx
	movq	%rbx, 368(%rsp)
	movq	%rdx, 304(%rsp)
	movq	160(%rax), %rcx
	testq	%rcx, %rcx
	je	.LBB361_30
	movq	152(%rax), %r15
	cmpl	$1, 1144(%r14)
	leaq	(%r15,%rcx,4), %r12
	jne	.LBB361_31
	movq	1152(%r14), %rax
	testq	%rax, %rax
	je	.LBB361_30
	movq	1160(%r14), %rcx
	xorl	%ebx, %ebx
.LBB361_5:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rcx, %rdx
	movq	%rax, %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	je	.LBB361_11
.LBB361_6:
	movl	%r8d, %r9d
	shll	$2, %r9d
	xorl	%edi, %edi
	.p2align	4
.LBB361_7:
	cmpl	8(%rsi,%rdi,4), %ebp
	seta	%r10b
	sbbb	$0, %r10b
	cmpb	$1, %r10b
	jne	.LBB361_10
	incq	%rdi
	addq	$-4, %r9
	jne	.LBB361_7
	jmp	.LBB361_11
.LBB361_10:
	movzbl	%r10b, %r8d
	testl	%r8d, %r8d
	je	.LBB361_32
	jmp	.LBB361_12
	.p2align	4
.LBB361_11:
	movq	%r8, %rdi
.LBB361_12:
	subq	$1, %rdx
	jb	.LBB361_14
	movq	56(%rsi,%rdi,8), %rsi
	movzwl	54(%rsi), %r8d
	testl	%r8d, %r8d
	jne	.LBB361_6
	jmp	.LBB361_11
.LBB361_14:
	cmpq	%r12, %r15
	jne	.LBB361_5
	movq	312(%rsp), %r14
	xorl	%r15d, %r15d
	jmp	.LBB361_62
.LBB361_16:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::governor::lift::Lift>::at@GOTPCREL(%rip), %rax
	leaq	624(%rsp), %rdi
	callq	*%rax
	movq	8(%rbx), %r12
	movq	16(%rbx), %rbx
	addq	$16, %r12
	testq	%rbx, %rbx
	jns	.LBB361_19
	xorl	%edi, %edi
.LBB361_18:
.Ltmp15235:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movq	%rbx, %rsi
	callq	*%rax
.Ltmp15236:
	jmp	.LBB361_444
.LBB361_19:
	movq	664(%r14), %r14
	movabsq	$-9223372036854775808, %r15
	je	.LBB361_272
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_440
	movq	%rax, %r13
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rdx
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rsi
	movq	$-1, %rdi
	leaq	(%rbx,%rax), %rcx
	sarq	$63, %rcx
	xorq	%r15, %rcx
	addq	%rbx, %rax
	cmovoq	%rcx, %rax
	incq	%rdx
	cmoveq	%rdi, %rdx
	addq	%rbx, %rsi
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmovbq	%rdi, %rsi
	movq	%rdx, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%rsi, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_23
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_23:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_29
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_23
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rcx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdx
	lock		incq	(%rax)
	lock		addq	%rbx, (%rcx)
	movq	%rbx, %rcx
	lock		xaddq	%rcx, (%rdx)
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	leaq	(%rcx,%rbx), %rax
	sarq	$63, %rax
	xorq	%r15, %rax
	addq	%rbx, %rcx
	cmovoq	%rax, %rcx
	movq	(%rdx), %rax
	.p2align	4
.LBB361_26:
	cmpq	%rax, %rcx
	jle	.LBB361_28
	lock		cmpxchgq	%rcx, (%rdx)
	jne	.LBB361_26
.LBB361_28:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_29:
	.cfi_escape 0x2e, 0x00
	movq	memcpy@GOTPCREL(%rip), %rax
	movq	%r13, %rdi
	movq	%r12, %rsi
	movq	%rbx, %rdx
	callq	*%rax
	jmp	.LBB361_273
.LBB361_30:
	xorl	%ebx, %ebx
	xorl	%r15d, %r15d
	jmp	.LBB361_62
.LBB361_31:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB361_32:
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$16, %edi
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_438
	movq	%rax, %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rdx
	incq	%rax
	cmoveq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$16, %rax
	cmovbq	%rdx, %rax
	movabsq	$9223372036854775807, %rdx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$16, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_35
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_35:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_41
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_35
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rsi
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rdi
	lock		incq	(%rax)
	lock		addq	$16, (%rsi)
	movl	$16, %esi
	lock		xaddq	%rsi, (%rdi)
	addq	$16, %rsi
	cmovoq	%rdx, %rsi
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rdx
	movq	(%rdx), %rax
	.p2align	4
.LBB361_38:
	cmpq	%rax, %rsi
	jle	.LBB361_40
	lock		cmpxchgq	%rsi, (%rdx)
	jne	.LBB361_38
.LBB361_40:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_41:
	movl	$1, %ebx
	movq	$4, 48(%rsp)
	movq	%rcx, 56(%rsp)
	movl	%ebp, (%rcx)
	movq	$1, 64(%rsp)
	cmpq	%r12, %r15
	je	.LBB361_61
	leaq	48(%rsp), %r14
	jmp	.LBB361_45
	.p2align	4
.LBB361_43:
	movq	56(%rsp), %rcx
.LBB361_44:
	movl	%ebp, (%rcx,%rbx,4)
	incq	%rbx
	movq	%rbx, 64(%rsp)
	cmpq	%r12, %r15
	je	.LBB361_61
.LBB361_45:
	movq	312(%rsp), %rdx
	cmpl	$1, 1144(%rdx)
	jne	.LBB361_58
	movq	1152(%rdx), %rax
	testq	%rax, %rax
	je	.LBB361_61
	movq	1160(%rdx), %rdx
.LBB361_48:
	movl	(%r15), %ebp
	addq	$4, %r15
	movq	%rdx, %rsi
	movq	%rax, %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	je	.LBB361_54
.LBB361_49:
	movl	%r9d, %r10d
	shll	$2, %r10d
	xorl	%r8d, %r8d
	.p2align	4
.LBB361_50:
	cmpl	8(%rdi,%r8,4), %ebp
	seta	%r11b
	sbbb	$0, %r11b
	cmpb	$1, %r11b
	jne	.LBB361_53
	incq	%r8
	addq	$-4, %r10
	jne	.LBB361_50
	jmp	.LBB361_54
	.p2align	4
.LBB361_53:
	movzbl	%r11b, %r9d
	testl	%r9d, %r9d
	je	.LBB361_59
	subq	$1, %rsi
	jae	.LBB361_56
	jmp	.LBB361_57
	.p2align	4
.LBB361_54:
	movq	%r9, %r8
	subq	$1, %rsi
	jb	.LBB361_57
.LBB361_56:
	movq	56(%rdi,%r8,8), %rdi
	movzwl	54(%rdi), %r9d
	testl	%r9d, %r9d
	jne	.LBB361_49
	jmp	.LBB361_54
.LBB361_57:
	cmpq	%r12, %r15
	jne	.LBB361_48
	jmp	.LBB361_61
	.p2align	4
.LBB361_58:
	movl	(%r15), %ebp
	addq	$4, %r15
.LBB361_59:
	cmpq	48(%rsp), %rbx
	jne	.LBB361_44
.Ltmp15241:
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	movl	$4, %ecx
	movl	$4, %r8d
	movq	%r14, %rdi
	movq	%rbx, %rsi
	callq	<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)
.Ltmp15242:
	jmp	.LBB361_43
.LBB361_61:
	movq	56(%rsp), %rax
	movq	48(%rsp), %r15
	movq	312(%rsp), %r14
	movq	%rax, 304(%rsp)
.LBB361_62:
.Ltmp15247:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::modifier::yields_nothing_without_rows_in_the_active_graph@GOTPCREL(%rip), %rax
	movq	160(%rsp), %rdi
	movq	%r15, 40(%rsp)
	callq	*%rax
	movb	%al, 15(%rsp)
.Ltmp15248:
	movq	368(%rsp), %rcx
	movl	784(%r14), %esi
	movl	788(%r14), %edx
	movq	$0, 328(%rsp)
	movq	$8, 336(%rsp)
	movq	$-1, 464(%rsp)
	movq	$0, 344(%rsp)
	leaq	8(%rcx), %rax
	movl	%esi, 168(%rsp)
	movl	%edx, 172(%rsp)
	movq	%rax, 392(%rsp)
	testq	%rbx, %rbx
	je	.LBB361_212
	movq	304(%rsp), %rax
	movq	8(%rcx), %rsi
	movq	16(%rcx), %rcx
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	leaq	176(%rsp), %r12
	xorl	%r13d, %r13d
	movq	$0, 376(%rsp)
	leaq	(%rax,%rbx,4), %rdx
	movq	%rcx, 568(%rsp)
	movl	$8, %ecx
	movq	%rsi, 576(%rsp)
	movq	%rax, 16(%rsp)
	movq	%rcx, 408(%rsp)
	movq	%rdx, 456(%rsp)
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.538(%rip), %rdx
	movq	%rdx, 448(%rsp)
.LBB361_65:
	movq	16(%rsp), %rbx
	jmp	.LBB361_69
.LBB361_66:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_67:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_68:
	cmpq	456(%rsp), %rbx
	je	.LBB361_213
.LBB361_69:
	movl	(%rbx), %eax
	addq	$4, %rbx
	cmpb	$0, 15(%rsp)
	movq	%rbx, 16(%rsp)
	movq	%rax, 352(%rsp)
	je	.LBB361_78
	movq	664(%r14), %rbx
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::probe_plan@GOTPCREL(%rip), %rax
	movl	$2, %ecx
	xorl	%edi, %edi
	xorl	%esi, %esi
	xorl	%edx, %edx
	vzeroupper
	callq	*%rax
	movq	%rax, 176(%rsp)
	movb	%dl, 184(%rsp)
.Ltmp15250:
	.cfi_escape 0x2e, 0x10
	movq	%r12, %rdx
	leaq	48(%rsp), %rdi
	movq	%rbx, %rsi
	xorl	%ecx, %ecx
	xorl	%r8d, %r8d
	xorl	%r9d, %r9d
	pushq	352(%rsp)
	.cfi_adjust_cfa_offset 8
	pushq	$2
	.cfi_adjust_cfa_offset 8
	movq	<purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan@GOTPCREL(%rip), %rax
	callq	*%rax
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
.Ltmp15251:
.Ltmp15252:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	callq	<purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next
.Ltmp15253:
	testq	%rax, %rax
	jne	.LBB361_78
	cmpq	$0, 80(%rbx)
	je	.LBB361_76
.Ltmp15254:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.12030959638369354654(%rip), %rsi
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15255:
	testl	%eax, %eax
	je	.LBB361_434
.LBB361_76:
.Ltmp15259:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	movl	$50, %edx
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.12030959638369354654(%rip), %rsi
	movq	%rbx, %rdi
	callq	*%rax
.Ltmp15260:
	movq	%rbx, 56(%rsp)
	movl	%eax, 64(%rsp)
	movq	352(%rsp), %rax
	movq	$0, 72(%rsp)
	movq	$0, 96(%rsp)
	movl	$2, 48(%rsp)
	movl	%eax, 52(%rsp)
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rsi
	movq	%r12, %rdi
	callq	<core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next
	cmpl	$0, 176(%rsp)
	je	.LBB361_205
	.p2align	4
.LBB361_78:
	movq	352(%rsp), %rax
	movl	$2, 784(%r14)
	movl	%eax, 788(%r14)
.Ltmp15261:
	.cfi_escape 0x2e, 0x00
	movq	160(%rsp), %rcx
	leaq	896(%rsp), %rdi
	movq	%r14, %rdx
	movq	%rcx, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15262:
	cmpl	$1, 896(%rsp)
	je	.LBB361_267
	cmpq	$-1, 904(%rsp)
	jne	.LBB361_268
	leaq	904(%rsp), %rax
	vmovdqu	8(%rax), %ymm0
	vmovdqu	%ymm0, 416(%rsp)
	movq	440(%rsp), %rax
	movq	%rax, 320(%rsp)
	leaq	16(%rax), %rsi
.Ltmp15271:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rbx
	movq	%rbx, %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.10720091597982897309)
.Ltmp15272:
.Ltmp15274:
	.cfi_escape 0x2e, 0x00
	movq	392(%rsp), %rsi
	movq	%rbx, %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.10720091597982897309)
.Ltmp15275:
	cmpq	$1, %rax
	jne	.LBB361_107
	movq	432(%rsp), %rax
	movq	424(%rsp), %r15
	movq	%rdx, %r14
	movq	416(%rsp), %rdx
	leaq	(%rax,%rax,4), %rcx
	movq	%r15, 176(%rsp)
	movq	%rdx, 400(%rsp)
	movq	%rdx, 192(%rsp)
	movq	%r15, 384(%rsp)
	leaq	(%r15,%rcx,8), %r12
	movq	%r12, 200(%rsp)
	testq	%rax, %rax
	je	.LBB361_165
	vmovd	352(%rsp), %xmm0
	vpshufb	.LCPI361_0(%rip), %xmm0, %xmm0
	vmovdqa	%xmm0, 544(%rsp)
	jmp	.LBB361_88
	.p2align	4
.LBB361_86:
	movq	336(%rsp), %rdx
	leaq	(%r13,%r13,4), %rax
	leaq	736(%rsp), %rcx
	incq	%r13
	movq	%rbp, (%rdx,%rax,8)
	movq	%rbx, 8(%rdx,%rax,8)
	movq	%rdx, 408(%rsp)
	vmovdqu	8(%rcx), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	24(%rcx), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%r13, 344(%rsp)
.LBB361_87:
	movq	16(%rsp), %rbx
	cmpq	%r12, %r15
	je	.LBB361_179
.LBB361_88:
	movq	%r15, %rax
	movq	(%rax), %rbp
	addq	$40, %r15
	testq	%rbp, %rbp
	je	.LBB361_164
	movq	%rbp, 728(%rsp)
	leaq	736(%rsp), %rdx
	leaq	-1(%rbp), %rcx
	vmovdqu	8(%rax), %ymm0
	cmpq	$5, %rcx
	vmovdqu	%ymm0, (%rdx)
	movq	744(%rsp), %rax
	movq	736(%rsp), %rbx
	leaq	-1(%rax), %rsi
	cmovbq	%rcx, %rsi
	cmpq	%rsi, %r14
	jae	.LBB361_437
	cmpq	$5, %rcx
	movq	%rdx, %rcx
	cmovaeq	%rbx, %rcx
	movl	(%rcx,%r14,8), %edx
	testl	%edx, %edx
	je	.LBB361_95
	cmpl	$2, %edx
	jne	.LBB361_96
.LBB361_92:
	cmpq	$6, %rbp
	cmovbq	%rbp, %rax
	decq	%rax
	cmpq	%rax, %r14
	jae	.LBB361_436
	vmovdqa	544(%rsp), %xmm0
	cmpq	$6, %rbp
	leaq	736(%rsp), %rax
	cmovbq	%rax, %rbx
	vmovq	%xmm0, (%rbx,%r14,8)
	movq	728(%rsp), %rbp
	movq	736(%rsp), %rbx
	cmpq	328(%rsp), %r13
	jne	.LBB361_86
.Ltmp15294:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	328(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15295:
	jmp	.LBB361_86
	.p2align	4
.LBB361_95:
	movq	352(%rsp), %rdx
	cmpl	%edx, 4(%rcx,%r14,8)
	je	.LBB361_92
.LBB361_96:
	cmpq	$6, %rbp
	jb	.LBB361_87
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	leaq	-8(,%rbp,8), %rcx
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
	jge	.LBB361_99
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
.LBB361_99:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rsi
	.p2align	4
.LBB361_100:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_106
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_100
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%rcx, %rdx
	negq	%rdx
	movabsq	$9223372036854775807, %rdi
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%rcx, %rdx
	setns	%al
	addq	%rdi, %rax
	subq	%rcx, %rdx
	cmovoq	%rax, %rdx
	movq	(%rsi), %rax
	.p2align	4
.LBB361_103:
	cmpq	%rax, %rdx
	jge	.LBB361_105
	lock		cmpxchgq	%rdx, (%rsi)
	jne	.LBB361_103
.LBB361_105:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_106:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
	jmp	.LBB361_87
	.p2align	4
.LBB361_107:
	movq	576(%rsp), %rsi
	lock		incq	(%rsi)
	jle	.LBB361_444
.Ltmp15276:
	.cfi_escape 0x2e, 0x00
	movq	568(%rsp), %rdx
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	callq	*%rax
	movq	%rax, 544(%rsp)
.Ltmp15277:
	movq	432(%rsp), %rax
	movq	424(%rsp), %r15
	movq	416(%rsp), %rdx
	movq	64(%rsp), %rsi
	movq	16(%rsp), %rbx
	leaq	(%rax,%rax,4), %rcx
	movq	%r15, 592(%rsp)
	movq	%rdx, 384(%rsp)
	movq	%rdx, 608(%rsp)
	movq	%r15, 584(%rsp)
	leaq	(%r15,%rcx,8), %r12
	movq	%r12, 616(%rsp)
	testq	%rax, %rax
	je	.LBB361_139
	vmovd	352(%rsp), %xmm0
	leaq	1(%rsi), %rax
	movq	%rsi, 320(%rsp)
	vpshufb	.LCPI361_0(%rip), %xmm0, %xmm0
	movq	%rax, 400(%rsp)
	vmovdqa	%xmm0, 352(%rsp)
	jmp	.LBB361_112
	.p2align	4
.LBB361_111:
	movq	408(%rsp), %rdx
	leaq	(%r12,%r12,4), %rax
	incq	%r12
	movq	%r12, %r13
	movq	%rbp, (%rdx,%rax,8)
	movq	%rbx, 8(%rdx,%rax,8)
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	16(%rsp), %rbx
	vmovdqa	768(%rsp), %xmm0
	vmovdqu	%xmm0, 16(%rdx,%rax,8)
	movq	784(%rsp), %rcx
	movq	%rcx, 32(%rdx,%rax,8)
	movq	%r12, 344(%rsp)
	movq	%r14, %r12
	cmpq	%r14, %r15
	je	.LBB361_153
.LBB361_112:
	vmovdqu	8(%r15), %ymm0
	movq	(%r15), %rax
	addq	$40, %r15
	vmovdqu	%ymm0, 272(%rsp)
	testq	%rax, %rax
	je	.LBB361_139
	vmovdqu	272(%rsp), %ymm0
	leaq	184(%rsp), %rcx
	movq	%rax, 176(%rsp)
	movq	320(%rsp), %rdx
	leaq	-1(%rax), %rdi
	movq	%r12, %r14
	cmpq	$5, %rdi
	movq	%rdx, %rbx
	vmovdqu	%ymm0, (%rcx)
	movq	192(%rsp), %rcx
	leaq	-1(%rcx), %rsi
	cmovbq	%rdi, %rsi
	subq	%rsi, %rbx
	jbe	.LBB361_122
	cmpq	$5, %rdi
	movl	$4, %eax
	cmovaeq	%rdi, %rax
	movq	%rax, %rcx
	subq	%rsi, %rcx
	cmpq	%rbx, %rcx
	jb	.LBB361_137
.LBB361_115:
	xorl	%ecx, %ecx
	leaq	184(%rsp), %rdx
	cmpq	$5, %rdi
	setae	%sil
	jb	.LBB361_117
	movq	184(%rsp), %rdx
.LBB361_117:
	movb	%sil, %cl
	movl	$2, %ebp
	movq	%r13, %r12
	shll	$4, %ecx
	movq	176(%rsp,%rcx), %rsi
	leaq	-1(%rsi), %rdi
	cmpq	%rax, %rdi
	jae	.LBB361_127
	incq	%rax
	movl	$2, %edi
	.p2align	4
.LBB361_119:
	cmpl	$-1, %edi
	je	.LBB361_124
	leaq	-1(%rbx), %r8
	movl	%edi, %ebp
	movl	%ebp, -8(%rdx,%rsi,8)
	cmpq	$1, %r8
	sbbl	%edi, %edi
	cmpq	$1, %r8
	adcq	$-1, %rbx
	orl	%ebp, %edi
	incq	%rsi
	cmpq	%rsi, %rax
	jne	.LBB361_119
	movq	%rax, 176(%rsp,%rcx)
	testq	%r8, %r8
	jne	.LBB361_127
	jmp	.LBB361_129
	.p2align	4
.LBB361_122:
	cmpq	$6, %rax
	movq	%r13, %r12
	cmovbq	%rax, %rcx
	decq	%rcx
	cmpq	%rcx, %rdx
	jae	.LBB361_129
	movq	400(%rsp), %rdx
	xorl	%ecx, %ecx
	cmpq	$6, %rax
	setae	%cl
	shll	$4, %ecx
	movq	%rdx, 176(%rsp,%rcx)
	jmp	.LBB361_129
.LBB361_124:
	movq	%rsi, 176(%rsp,%rcx)
	jmp	.LBB361_129
.LBB361_125:
.Ltmp15282:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	movl	$1, %edx
	leaq	176(%rsp), %rdi
	movl	$1, %ecx
	vzeroupper
	callq	*%rax
.Ltmp15283:
	cmpq	$6, 176(%rsp)
	movq	184(%rsp), %rax
	leaq	184(%rsp), %rcx
	leaq	192(%rsp), %rdx
	cmovbq	%rcx, %rax
	leaq	176(%rsp), %rcx
	cmovaeq	%rdx, %rcx
	jmp	.LBB361_128
	.p2align	4
.LBB361_127:
	movq	176(%rsp), %rsi
	movq	184(%rsp), %rax
	xorl	%edx, %edx
	leaq	184(%rsp), %rcx
	leaq	192(%rsp), %rdi
	decq	%rsi
	cmpq	$5, %rsi
	cmovbq	%rcx, %rax
	leaq	176(%rsp), %rcx
	setae	%dl
	cmovaeq	%rdi, %rcx
	movl	$4, %edi
	cmovbq	%rdi, %rsi
	shll	$4, %edx
	movq	176(%rsp,%rdx), %r13
	leaq	-1(%r13), %rdx
	cmpq	%rsi, %rdx
	je	.LBB361_125
.LBB361_128:
	movl	%ebp, -8(%rax,%r13,8)
	incq	%r13
	decq	%rbx
	movq	%r13, (%rcx)
	testq	%rbx, %rbx
	jne	.LBB361_127
.LBB361_129:
	movq	176(%rsp), %rax
	movq	%rax, %rsi
	cmpq	$6, %rax
	jb	.LBB361_131
	movq	192(%rsp), %rsi
.LBB361_131:
	movq	544(%rsp), %rdi
	decq	%rsi
	cmpq	%rsi, %rdi
	jae	.LBB361_435
	leaq	184(%rsp), %rcx
	movq	%r12, %rdx
	cmpq	$6, %rax
	jb	.LBB361_134
	movq	184(%rsp), %rcx
.LBB361_134:
	vmovaps	352(%rsp), %xmm0
	leaq	192(%rsp), %rax
	movq	%rdx, %r12
	vmovlps	%xmm0, (%rcx,%rdi,8)
	vmovdqu	(%rax), %xmm0
	movq	16(%rax), %rax
	movq	176(%rsp), %rbp
	movq	184(%rsp), %rbx
	movq	%rax, 784(%rsp)
	vmovdqa	%xmm0, 768(%rsp)
	cmpq	328(%rsp), %rdx
	jne	.LBB361_111
.Ltmp15288:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one@GOTPCREL(%rip), %rax
	leaq	328(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15289:
	movq	336(%rsp), %rax
	movq	%rax, 408(%rsp)
	jmp	.LBB361_111
.LBB361_137:
.Ltmp15279:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movl	$1, %ecx
	movq	%rbx, %rdx
	vzeroupper
	callq	*%rax
.Ltmp15280:
	movq	176(%rsp), %rdi
	movl	$4, %eax
	decq	%rdi
	cmpq	$5, %rdi
	cmovaeq	%rdi, %rax
	jmp	.LBB361_115
	.p2align	4
.LBB361_139:
	movq	free@GOTPCREL(%rip), %r14
	subq	%r15, %r12
	je	.LBB361_153
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %rax
	xorl	%ebx, %ebx
	imulq	%rax, %r12
	jmp	.LBB361_144
	.p2align	4
.LBB361_141:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_142:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	*%r14
.LBB361_143:
	incq	%rbx
	cmpq	%r12, %rbx
	je	.LBB361_152
.LBB361_144:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB361_143
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_147
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_147:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_142
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_147
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
	movq	(%rbp), %rax
	.p2align	4
.LBB361_150:
	cmpq	%rax, %rdx
	jge	.LBB361_141
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_150
	jmp	.LBB361_141
.LBB361_152:
	movq	16(%rsp), %rbx
.LBB361_153:
	movq	384(%rsp), %rax
	movq	312(%rsp), %r14
	movq	40(%rsp), %r15
	leaq	176(%rsp), %r12
	testq	%rax, %rax
	je	.LBB361_163
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	jge	.LBB361_156
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_156:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_162
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_156
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
	movq	(%rbp), %rax
	.p2align	4
.LBB361_159:
	cmpq	%rax, %rdx
	jge	.LBB361_161
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_159
.LBB361_161:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_162:
	.cfi_escape 0x2e, 0x00
	movq	584(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_163:
	movq	440(%rsp), %rax
	jmp	.LBB361_190
.LBB361_164:
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
.LBB361_165:
	movq	free@GOTPCREL(%rip), %r14
	subq	%r15, %r12
	je	.LBB361_178
	shrq	$3, %r12
	movabsq	$-3689348814741910323, %rax
	xorl	%ebx, %ebx
	imulq	%rax, %r12
	jmp	.LBB361_170
	.p2align	4
.LBB361_167:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_168:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	*%r14
.LBB361_169:
	incq	%rbx
	cmpq	%r12, %rbx
	je	.LBB361_178
.LBB361_170:
	leaq	(%rbx,%rbx,4), %rcx
	movq	(%r15,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB361_169
	leaq	(%r15,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdi
	cmpq	%rdi, %rcx
	cmovaeq	%rdi, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%rdi, %rsi
	movq	8(%rdx), %rdi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_173
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_173:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_168
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_173
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
	movq	(%rbp), %rax
	.p2align	4
.LBB361_176:
	cmpq	%rax, %rdx
	jge	.LBB361_167
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_176
	jmp	.LBB361_167
	.p2align	4
.LBB361_178:
	movq	16(%rsp), %rbx
.LBB361_179:
	movq	400(%rsp), %rax
	movq	312(%rsp), %r14
	movq	40(%rsp), %r15
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	leaq	176(%rsp), %r12
	testq	%rax, %rax
	je	.LBB361_189
	shlq	$3, %rax
	movabsq	$9223372036854775807, %rsi
	leaq	(%rax,%rax,4), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
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
	jge	.LBB361_182
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_182:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_188
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_182
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
	movq	(%rbp), %rax
	.p2align	4
.LBB361_185:
	cmpq	%rax, %rdx
	jge	.LBB361_187
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_185
.LBB361_187:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_188:
	.cfi_escape 0x2e, 0x00
	movq	384(%rsp), %rdi
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_189:
	movq	320(%rsp), %rax
.LBB361_190:
	vmovups	72(%rsp), %ymm1
	vmovdqu	48(%rsp), %ymm0
	vmovups	%ymm1, 200(%rsp)
	vmovdqu	%ymm0, 176(%rsp)
	lock		decq	(%rax)
	jne	.LBB361_192
	#MEMBARRIER
.Ltmp15302:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15303:
.LBB361_192:
	vmovups	200(%rsp), %ymm1
	vmovups	176(%rsp), %ymm0
	cmpq	$-1, 464(%rsp)
	vmovups	%ymm1, 72(%rsp)
	vmovups	%ymm0, 48(%rsp)
	je	.LBB361_194
.Ltmp15305:
	.cfi_escape 0x2e, 0x00
	leaq	464(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15306:
.LBB361_194:
	vmovups	72(%rsp), %ymm1
	vmovdqu	48(%rsp), %ymm0
	cmpb	$0, 896(%rsp)
	vmovups	%ymm1, 488(%rsp)
	vmovdqu	%ymm0, 464(%rsp)
	jne	.LBB361_68
	cmpq	$-1, 904(%rsp)
	je	.LBB361_68
.Ltmp15310:
	.cfi_escape 0x2e, 0x00
	leaq	904(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15311:
	movq	936(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_68
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rsi
	movq	944(%rsp), %rdi
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
	jge	.LBB361_200
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_200:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_67
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_200
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
	movq	(%rbp), %rax
	.p2align	4
.LBB361_203:
	cmpq	%rax, %rdx
	jge	.LBB361_66
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_203
	jmp	.LBB361_66
.LBB361_205:
	movq	96(%rbx), %rax
	testq	%rax, %rax
	je	.LBB361_211
	movq	88(%rbx), %rcx
	shlq	$4, %rax
	xorl	%edx, %edx
	jmp	.LBB361_208
	.p2align	4
.LBB361_207:
	addq	$16, %rdx
	cmpq	%rdx, %rax
	je	.LBB361_211
.LBB361_208:
	movl	12(%rcx,%rdx), %esi
	testl	%esi, %esi
	je	.LBB361_207
	cmpl	352(%rsp), %esi
	jne	.LBB361_207
	cmpl	$0, (%rcx,%rdx)
	je	.LBB361_207
	jmp	.LBB361_78
.LBB361_211:
	movb	$1, %al
	movb	$1, %bl
	movq	16(%rsp), %rcx
	movq	%rax, 376(%rsp)
	cmpq	456(%rsp), %rcx
	jne	.LBB361_65
	jmp	.LBB361_214
.LBB361_212:
	xorl	%r13d, %r13d
	movq	$0, 376(%rsp)
.LBB361_213:
	movb	$1, %bl
.LBB361_214:
	testq	%r15, %r15
	je	.LBB361_224
.LBB361_215:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$2, %r15
	movabsq	$9223372036854775807, %rcx
	cmpq	%rcx, %r15
	cmovaeq	%rcx, %r15
	xorl	%edx, %edx
	cmpq	%r15, %rax
	setns	%dl
	addq	%rcx, %rdx
	subq	%r15, %rax
	cmovoq	%rdx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_217
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_217:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_223
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_217
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r15, %rdx
	negq	%rdx
	lock		xaddq	%rdx, (%rax)
	xorl	%eax, %eax
	cmpq	%r15, %rdx
	setns	%al
	addq	%rcx, %rax
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rcx
	subq	%r15, %rdx
	cmovoq	%rax, %rdx
	movq	(%rcx), %rax
	.p2align	4
.LBB361_220:
	cmpq	%rax, %rdx
	jge	.LBB361_222
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_220
.LBB361_222:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_223:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	304(%rsp), %rdi
	vzeroupper
	callq	*%rax
.LBB361_224:
	movl	168(%rsp), %eax
	movl	172(%rsp), %ecx
	movq	464(%rsp), %r12
	movl	%eax, 784(%r14)
	movl	%ecx, 788(%r14)
	cmpq	$-1, %r12
	sete	%bpl
	je	.LBB361_238
	vmovups	488(%rsp), %ymm1
	vmovdqu	464(%rsp), %ymm0
	vmovups	%ymm1, 88(%rsp)
	vmovdqu	%ymm0, 64(%rsp)
	movq	$1, 48(%rsp)
	movq	$1, 56(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_439
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_228
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_228:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_234
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_228
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB361_231:
	cmpq	%rax, %rdx
	jle	.LBB361_233
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_231
.LBB361_233:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_234:
	vmovdqu64	48(%rsp), %zmm0
	movq	112(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
.LBB361_235:
	vmovups	832(%rsp), %zmm1
	vmovdqu64	792(%rsp), %zmm0
	movq	344(%rsp), %rax
	movq	%rax, 288(%rsp)
	vmovups	%zmm1, 88(%rsp)
	vmovups	328(%rsp), %xmm1
	vmovdqu64	%zmm0, 48(%rsp)
	cmpq	$-1, 48(%rsp)
	vmovaps	%xmm1, 272(%rsp)
	movq	%rbx, 296(%rsp)
	je	.LBB361_254
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	272(%rsp), %rsi
	leaq	792(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_237
.LBB361_255:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	128(%rsp), %rdi
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
	jge	.LBB361_257
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_257:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_263
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_257
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
.LBB361_260:
	cmpq	%rax, %rsi
	jge	.LBB361_262
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_260
.LBB361_262:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_263:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
	movq	144(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB361_264
	jmp	.LBB361_266
.LBB361_238:
	testb	%bl, %bl
	je	.LBB361_308
	testb	$1, 376(%rsp)
	je	.LBB361_396
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp15386:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15387:
	movq	%rax, %rsi
	movq	%rax, %rbx
	movq	%rax, 48(%rsp)
	addq	$16, %rsi
.Ltmp15389:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.10720091597982897309)
.Ltmp15390:
	lock		decq	(%rbx)
	jne	.LBB361_244
	#MEMBARRIER
.Ltmp15394:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	callq	*%rax
.Ltmp15395:
.LBB361_244:
	movq	392(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	movq	368(%rsp), %rax
	jle	.LBB361_444
	movq	16(%rax), %rdx
.Ltmp15396:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	callq	*%rax
.Ltmp15397:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	vmovups	%ymm1, 88(%rsp)
	vmovdqu	%ymm0, 64(%rsp)
	movq	$1, 48(%rsp)
	movq	$1, 56(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_442
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_249
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_249:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_234
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_249
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB361_252:
	cmpq	%rax, %rdx
	jle	.LBB361_233
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_252
	jmp	.LBB361_233
.LBB361_254:
	vmovdqu	272(%rsp), %xmm0
	movq	288(%rsp), %rax
	movq	296(%rsp), %rcx
	movq	%rax, 200(%rsp)
	movq	%rcx, 208(%rsp)
	vmovdqu	%xmm0, 184(%rsp)
	movq	$-1, 176(%rsp)
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB361_255
.LBB361_237:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_266
.LBB361_264:
	lock		decq	(%rax)
	jne	.LBB361_266
	leaq	144(%rsp), %rdi
	#MEMBARRIER
.Ltmp15401:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15402:
.LBB361_266:
	vmovdqu64	176(%rsp), %zmm0
	vmovups	208(%rsp), %zmm1
	movq	32(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB361_395
.LBB361_267:
	leaq	904(%rsp), %rax
	movl	168(%rsp), %ecx
	movl	172(%rsp), %edx
	movb	$1, %dil
	vmovdqu64	8(%rax), %zmm0
	vmovups	40(%rax), %zmm1
	movq	32(%rsp), %rax
	movl	%ecx, 784(%r14)
	movl	%edx, 788(%r14)
	vmovups	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movq	free@GOTPCREL(%rip), %rdx
	movl	%edi, 16(%rsp)
	testq	%r15, %r15
	jne	.LBB361_336
	jmp	.LBB361_345
.LBB361_268:
	movb	$1, %bl
.Ltmp15264:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	leaq	792(%rsp), %rsi
	leaq	904(%rsp), %rcx
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15265:
	cmpq	$-1, 48(%rsp)
	je	.LBB361_334
.Ltmp15266:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15267:
	xorl	%ebx, %ebx
	testq	%r15, %r15
	jne	.LBB361_215
	jmp	.LBB361_224
.LBB361_272:
	movl	$1, %r13d
.LBB361_273:
	movq	%rbx, 472(%rsp)
	movq	%r13, 480(%rsp)
	movq	%rbx, 488(%rsp)
	movb	$1, %bl
	movq	%r15, 464(%rsp)
.Ltmp15212:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_value@GOTPCREL(%rip), %rax
	leaq	464(%rsp), %rsi
	movq	%r14, %rdi
	callq	*%rax
.Ltmp15213:
	movq	312(%rsp), %r14
	testl	%eax, %eax
	je	.LBB361_290
	movq	664(%r14), %rcx
	movq	160(%rcx), %rdx
	testq	%rdx, %rdx
	je	.LBB361_290
	movq	152(%rcx), %rcx
	xorl	%esi, %esi
	cmpq	$1, %rdx
	je	.LBB361_278
	.p2align	4
.LBB361_277:
	movq	%rdx, %r8
	shrq	%r8
	movq	%rsi, %rdi
	addq	%r8, %rsi
	cmpl	%eax, (%rcx,%rsi,4)
	cmovaq	%rdi, %rsi
	subq	%r8, %rdx
	cmpq	$1, %rdx
	ja	.LBB361_277
.LBB361_278:
	cmpl	%eax, (%rcx,%rsi,4)
	jne	.LBB361_290
	cmpl	$1, 1144(%r14)
	jne	.LBB361_324
	movq	1152(%r14), %rcx
	testq	%rcx, %rcx
	je	.LBB361_290
	movq	1160(%r14), %rdx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	jne	.LBB361_282
.LBB361_287:
	movq	%rdi, %rsi
.LBB361_288:
	subq	$1, %rdx
	jb	.LBB361_290
	movq	56(%rcx,%rsi,8), %rcx
	movzwl	54(%rcx), %edi
	testl	%edi, %edi
	je	.LBB361_287
.LBB361_282:
	movl	%edi, %r8d
	shll	$2, %r8d
	xorl	%esi, %esi
	.p2align	4
.LBB361_283:
	cmpl	8(%rcx,%rsi,4), %eax
	seta	%r9b
	sbbb	$0, %r9b
	cmpb	$1, %r9b
	jne	.LBB361_286
	incq	%rsi
	addq	$-4, %r8
	jne	.LBB361_283
	jmp	.LBB361_287
.LBB361_286:
	movzbl	%r9b, %edi
	testl	%edi, %edi
	jne	.LBB361_288
.LBB361_324:
	vmovsd	784(%r14), %xmm0
	movl	$2, 784(%r14)
	movl	%eax, 788(%r14)
	vmovaps	%xmm0, 16(%rsp)
.Ltmp15214:
	.cfi_escape 0x2e, 0x00
	movq	160(%rsp), %rcx
	leaq	1120(%rsp), %rdi
	movq	%r14, %rdx
	movq	%rcx, %rsi
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15215:
	vmovaps	16(%rsp), %xmm0
	cmpl	$1, 1120(%rsp)
	vmovlps	%xmm0, 784(%r14)
	jne	.LBB361_413
	vmovups	1136(%rsp), %zmm0
	vmovups	1168(%rsp), %zmm1
	movq	32(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovups	%zmm0, 16(%rax)
	movq	$1, (%rax)
.Ltmp15223:
	.cfi_escape 0x2e, 0x00
	leaq	464(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.10720091597982897309)
.Ltmp15224:
	movq	696(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_329
	movq	704(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB361_329:
	movq	624(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB361_331
	movq	632(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
.LBB361_331:
	movq	720(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_395
	lock		decq	(%rax)
	jne	.LBB361_395
	leaq	720(%rsp), %rdi
	jmp	.LBB361_394
.LBB361_290:
	vmovups	664(%rsp), %zmm1
	vmovdqu64	624(%rsp), %zmm0
	vmovups	%zmm1, 88(%rsp)
	vmovdqu64	%zmm0, 48(%rsp)
.Ltmp15225:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15226:
	cmpq	$-1, 48(%rsp)
	movq	%rax, 296(%rsp)
	movq	$0, 272(%rsp)
	movq	$8, 280(%rsp)
	movq	$0, 288(%rsp)
	je	.LBB361_294
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	272(%rsp), %rsi
	leaq	624(%rsp), %rdx
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_293
.LBB361_295:
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	128(%rsp), %rdi
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
	jge	.LBB361_297
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_297:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_303
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_297
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
.LBB361_300:
	cmpq	%rax, %rsi
	jge	.LBB361_302
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_300
.LBB361_302:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_303:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	144(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB361_304
	jmp	.LBB361_306
.LBB361_294:
	movq	280(%rsp), %rcx
	movq	272(%rsp), %rax
	movq	288(%rsp), %rdx
	movq	%rcx, 192(%rsp)
	movq	296(%rsp), %rcx
	movq	%rax, 184(%rsp)
	movq	%rdx, 200(%rsp)
	movq	%rcx, 208(%rsp)
	movq	$-1, 176(%rsp)
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jae	.LBB361_295
.LBB361_293:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_306
.LBB361_304:
	lock		decq	(%rax)
	jne	.LBB361_306
	leaq	144(%rsp), %rdi
	#MEMBARRIER
.Ltmp15230:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15231:
.LBB361_306:
	vmovdqu64	176(%rsp), %zmm0
	vmovups	208(%rsp), %zmm1
.LBB361_307:
	movq	32(%rsp), %rax
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	.cfi_escape 0x2e, 0x00
	leaq	464(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.10720091597982897309)
	jmp	.LBB361_395
.LBB361_308:
	movq	888(%rsp), %r15
	movq	368(%rsp), %rax
	testq	%r15, %r15
	je	.LBB361_399
	lock		incq	(%r15)
	jle	.LBB361_444
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
	movq	%r15, 272(%rsp)
	leaq	16(%r15), %rsi
.Ltmp15319:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.10720091597982897309)
.Ltmp15320:
	lock		incq	(%rbx)
	jle	.LBB361_444
.Ltmp15322:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp15323:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	vmovups	%ymm1, 88(%rsp)
	vmovdqu	%ymm0, 64(%rsp)
	movq	$1, 48(%rsp)
	movq	$1, 56(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_441
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_316
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_316:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_322
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_316
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
	.p2align	4
.LBB361_319:
	cmpq	%rax, %rdx
	jle	.LBB361_321
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_319
.LBB361_321:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_322:
	vmovdqu64	48(%rsp), %zmm0
	movq	112(%rsp), %rax
	movq	%rax, 64(%rbx)
	vmovdqu64	%zmm0, (%rbx)
	lock		decq	(%r15)
	jne	.LBB361_235
	movb	$1, %al
	#MEMBARRIER
	movl	%eax, 16(%rsp)
.Ltmp15327:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15328:
	jmp	.LBB361_235
.LBB361_334:
	movl	168(%rsp), %eax
	movl	172(%rsp), %ecx
	xorl	%ebx, %ebx
	movl	%eax, 784(%r14)
	movl	%ecx, 788(%r14)
.Ltmp15268:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	leaq	792(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15269:
	vmovdqu64	48(%rsp), %zmm0
	vmovups	80(%rsp), %zmm1
	movq	32(%rsp), %rax
	xorl	%edi, %edi
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	movq	free@GOTPCREL(%rip), %rdx
	movl	%edi, 16(%rsp)
	testq	%r15, %r15
	je	.LBB361_345
.LBB361_336:
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	shlq	$2, %r15
	movabsq	$9223372036854775807, %rsi
	cmpq	%rsi, %r15
	cmovaeq	%rsi, %r15
	xorl	%ecx, %ecx
	cmpq	%r15, %rax
	setns	%cl
	addq	%rsi, %rcx
	subq	%r15, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_338
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_338:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_344
	leaq	1(%rax), %rcx
	lock		cmpxchgq	%rcx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_338
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rax
	movq	%r15, %rcx
	negq	%rcx
	movabsq	$9223372036854775807, %rsi
	lock		xaddq	%rcx, (%rax)
	xorl	%eax, %eax
	cmpq	%r15, %rcx
	setns	%al
	addq	%rsi, %rax
	subq	%r15, %rcx
	cmovoq	%rax, %rcx
	movq	(%rbp), %rax
	.p2align	4
.LBB361_341:
	cmpq	%rax, %rcx
	jge	.LBB361_343
	lock		cmpxchgq	%rcx, (%rbp)
	jne	.LBB361_341
.LBB361_343:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_344:
	.cfi_escape 0x2e, 0x00
	movq	304(%rsp), %rdi
	vzeroupper
	callq	*%rdx
.LBB361_345:
	movq	336(%rsp), %rbx
	testq	%r13, %r13
	je	.LBB361_358
	movq	purrdf_alloc_probe::PROCESS_TROUGH_BYTES@GOTPCREL(%rip), %rbp
	movq	free@GOTPCREL(%rip), %r12
	movabsq	$9223372036854775807, %r15
	xorl	%r14d, %r14d
	jmp	.LBB361_350
	.p2align	4
.LBB361_347:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_348:
	.cfi_escape 0x2e, 0x00
	vzeroupper
	callq	*%r12
.LBB361_349:
	incq	%r14
	cmpq	%r13, %r14
	je	.LBB361_358
.LBB361_350:
	leaq	(%r14,%r14,4), %rcx
	movq	(%rbx,%rcx,8), %rax
	cmpq	$6, %rax
	jb	.LBB361_349
	leaq	(%rbx,%rcx,8), %rdx
	leaq	-8(,%rax,8), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	cmpq	%r15, %rcx
	movq	8(%rdx), %rdi
	cmovaeq	%r15, %rcx
	xorl	%esi, %esi
	cmpq	%rcx, %rax
	setns	%sil
	addq	%r15, %rsi
	subq	%rcx, %rax
	cmovoq	%rsi, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jge	.LBB361_353
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_353:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_348
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_353
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
	movq	(%rbp), %rax
	.p2align	4
.LBB361_356:
	cmpq	%rax, %rdx
	jge	.LBB361_347
	lock		cmpxchgq	%rdx, (%rbp)
	jne	.LBB361_356
	jmp	.LBB361_347
.LBB361_358:
	movq	328(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_368
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
	jge	.LBB361_361
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_361:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_367
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_361
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
.LBB361_364:
	cmpq	%rax, %rsi
	jge	.LBB361_366
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_364
.LBB361_366:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_367:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	movq	%rbx, %rdi
	vzeroupper
	callq	*%rax
.LBB361_368:
	cmpq	$-1, 464(%rsp)
	je	.LBB361_370
.Ltmp15383:
	.cfi_escape 0x2e, 0x00
	leaq	464(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15384:
.LBB361_370:
	cmpb	$0, 16(%rsp)
	je	.LBB361_395
	movq	864(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_381
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	872(%rsp), %rdi
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
	jge	.LBB361_374
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_374:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_380
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_374
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
.LBB361_377:
	cmpq	%rax, %rsi
	jge	.LBB361_379
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_377
.LBB361_379:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_380:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_381:
	movq	792(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB361_391
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	800(%rsp), %rdi
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
	jge	.LBB361_384
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_384:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_390
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_384
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
.LBB361_387:
	cmpq	%rax, %rsi
	jge	.LBB361_389
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_387
.LBB361_389:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_390:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_391:
	movq	888(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_395
	lock		decq	(%rax)
	jne	.LBB361_395
	leaq	888(%rsp), %rdi
.LBB361_394:
	#MEMBARRIER
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.LBB361_395:
	movq	32(%rsp), %rax
	addq	$1240, %rsp
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
.LBB361_396:
	.cfi_def_cfa_offset 1296
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp15359:
	.cfi_escape 0x2e, 0x00
	movq	160(%rsp), %rcx
	leaq	1008(%rsp), %rdi
	movq	%r14, %rdx
	movq	%rcx, %rsi
	vzeroupper
	callq	purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
.Ltmp15360:
	cmpl	$1, 1008(%rsp)
	jne	.LBB361_417
	vmovdqu64	1024(%rsp), %zmm0
	vmovups	1056(%rsp), %zmm1
	movq	32(%rsp), %rax
	vmovups	%zmm1, 48(%rax)
	vmovdqu64	%zmm0, 16(%rax)
	movq	$1, (%rax)
	movb	$1, %al
	movl	%eax, 16(%rsp)
	jmp	.LBB361_345
.LBB361_399:
	movq	8(%rax), %rbx
	movq	16(%rax), %r14
	movb	$1, %al
	movl	%eax, 16(%rsp)
.Ltmp15338:
	.cfi_escape 0x2e, 0x00
	movq	purrdf_sparql_eval::eval::syntactic_schema@GOTPCREL(%rip), %rax
	movq	160(%rsp), %rdi
	vzeroupper
	callq	*%rax
.Ltmp15339:
	movq	%rax, %rsi
	movq	%rax, %r15
	movq	%rax, 48(%rsp)
	addq	$16, %rsi
.Ltmp15340:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.10720091597982897309)
.Ltmp15341:
	lock		decq	(%r15)
	jne	.LBB361_403
	#MEMBARRIER
.Ltmp15345:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	callq	*%rax
.Ltmp15346:
.LBB361_403:
	lock		incq	(%rbx)
	jle	.LBB361_444
.Ltmp15347:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	movq	%rbx, %rsi
	movq	%r14, %rdx
	callq	*%rax
.Ltmp15348:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	vmovups	%ymm1, 88(%rsp)
	vmovdqu	%ymm0, 64(%rsp)
	movq	$1, 48(%rsp)
	movq	$1, 56(%rsp)
	.cfi_escape 0x2e, 0x00
	movq	malloc@GOTPCREL(%rip), %rax
	movl	$72, %edi
	vzeroupper
	callq	*%rax
	testq	%rax, %rax
	je	.LBB361_443
	movq	%rax, %rbx
	movq	%fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movq	$-1, %rcx
	incq	%rax
	cmoveq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovbq	%rcx, %rax
	movabsq	$9223372036854775807, %rcx
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	addq	$72, %rax
	cmovoq	%rcx, %rax
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	cmpq	%fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	jle	.LBB361_408
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_408:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_234
	leaq	1(%rax), %rdx
	lock		cmpxchgq	%rdx, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_408
	movq	purrdf_alloc_probe::PROCESS_ALLOCATIONS@GOTPCREL(%rip), %rax
	movq	purrdf_alloc_probe::PROCESS_REQUESTED_BYTES@GOTPCREL(%rip), %rdx
	movq	purrdf_alloc_probe::PROCESS_LIVE_BYTES@GOTPCREL(%rip), %rsi
	lock		incq	(%rax)
	lock		addq	$72, (%rdx)
	movl	$72, %edx
	lock		xaddq	%rdx, (%rsi)
	addq	$72, %rdx
	cmovoq	%rcx, %rdx
	movq	purrdf_alloc_probe::PROCESS_PEAK_BYTES@GOTPCREL(%rip), %rcx
	movq	(%rcx), %rax
.LBB361_411:
	cmpq	%rax, %rdx
	jle	.LBB361_233
	lock		cmpxchgq	%rdx, (%rcx)
	jne	.LBB361_411
	jmp	.LBB361_233
.LBB361_413:
	leaq	1128(%rsp), %rcx
.Ltmp15216:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	leaq	624(%rsp), %rsi
	xorl	%edx, %edx
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15217:
	cmpq	$-1, 48(%rsp)
	je	.LBB361_424
	vmovups	624(%rsp), %zmm2
	vmovups	48(%rsp), %ymm0
	vmovups	664(%rsp), %zmm1
	vmovups	%zmm2, 48(%rsp)
	vmovups	%ymm0, 272(%rsp)
	vmovups	%zmm1, 88(%rsp)
	cmpq	$-1, 48(%rsp)
	je	.LBB361_428
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	272(%rsp), %rsi
	leaq	624(%rsp), %rdx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
	jmp	.LBB361_429
.LBB361_417:
	vmovups	1048(%rsp), %zmm1
	vmovdqu64	1016(%rsp), %zmm0
	vmovups	%zmm1, 80(%rsp)
	vmovdqu64	%zmm0, 48(%rsp)
.Ltmp15361:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	leaq	792(%rsp), %rsi
	leaq	48(%rsp), %rcx
	xorl	%edx, %edx
	vzeroupper
	callq	<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
.Ltmp15362:
	cmpq	$-1, 176(%rsp)
	movq	368(%rsp), %rbx
	je	.LBB361_426
	vmovdqu	176(%rsp), %ymm0
	vmovdqu	%ymm0, 272(%rsp)
	movq	296(%rsp), %rsi
	addq	$16, %rsi
.Ltmp15363:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	vzeroupper
	callq	<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.10720091597982897309)
.Ltmp15364:
	movq	392(%rsp), %rax
	movq	(%rax), %rsi
	lock		incq	(%rsi)
	jle	.LBB361_444
	movq	16(%rbx), %rdx
.Ltmp15366:
	.cfi_escape 0x2e, 0x00
	movq	<purrdf_sparql_eval::solution::VarSchema>::push@GOTPCREL(%rip), %rax
	leaq	176(%rsp), %rdi
	callq	*%rax
.Ltmp15367:
	vmovups	200(%rsp), %ymm1
	vmovdqu	176(%rsp), %ymm0
	leaq	64(%rsp), %r15
	vmovups	%ymm1, 88(%rsp)
	vmovdqu	%ymm0, 64(%rsp)
	movq	$1, 48(%rsp)
	movq	$1, 56(%rsp)
.Ltmp15371:
	.cfi_escape 0x2e, 0x00
	movl	$8, %edi
	movl	$72, %esi
	vzeroupper
	callq	alloc::boxed::box_new_uninit
.Ltmp15372:
	movq	112(%rsp), %rcx
	movq	%rax, %rbx
	movq	%rcx, 64(%rbx)
	vmovdqu64	48(%rsp), %zmm0
	vmovdqu64	%zmm0, (%rbx)
.Ltmp15379:
	.cfi_escape 0x2e, 0x00
	leaq	272(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15380:
	jmp	.LBB361_235
.LBB361_424:
	xorl	%ebx, %ebx
.Ltmp15220:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	leaq	624(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15221:
	vmovups	48(%rsp), %zmm0
	vmovups	80(%rsp), %zmm1
	jmp	.LBB361_307
.LBB361_426:
	movl	$0, 16(%rsp)
.Ltmp15381:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	leaq	792(%rsp), %rsi
	callq	<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
.Ltmp15382:
	vmovdqu64	48(%rsp), %zmm0
	vmovups	80(%rsp), %zmm1
	movq	32(%rsp), %rax
	movl	$0, 16(%rsp)
	vmovups	%zmm1, 40(%rax)
	vmovdqu64	%zmm0, 8(%rax)
	movq	$0, (%rax)
	jmp	.LBB361_345
.LBB361_428:
	vmovups	272(%rsp), %ymm0
	vmovups	%ymm0, 184(%rsp)
	movq	$-1, 176(%rsp)
.LBB361_429:
	movq	120(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_431
	movq	128(%rsp), %rdi
	leaq	-3(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_431:
	movq	144(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_306
	lock		decq	(%rax)
	jne	.LBB361_306
	xorl	%ebx, %ebx
	leaq	144(%rsp), %rdi
	#MEMBARRIER
.Ltmp15218:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	vzeroupper
	callq	*%rax
.Ltmp15219:
	jmp	.LBB361_306
.LBB361_434:
	movq	80(%rbx), %rax
	movq	%r12, 48(%rsp)
	movq	%rax, 176(%rsp)
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rax
	movq	%rax, 56(%rsp)
.Ltmp15256:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rax
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4165.llvm.12030959638369354654(%rip), %rdi
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4166.llvm.12030959638369354654(%rip), %rdx
	leaq	48(%rsp), %rsi
	callq	*%rax
.Ltmp15257:
	jmp	.LBB361_444
.LBB361_435:
	movq	%r15, 600(%rsp)
.Ltmp15285:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.537(%rip), %rdx
	vzeroupper
	callq	*%rax
.Ltmp15286:
	jmp	.LBB361_444
.LBB361_436:
	leaq	.Lanon.e5162873a9a3251d11c4df37a70e4654.539(%rip), %rcx
	movq	%rax, %rsi
	movq	%rcx, 448(%rsp)
.LBB361_437:
	movq	%r15, 184(%rsp)
.Ltmp15291:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_bounds_check@GOTPCREL(%rip), %rax
	movq	448(%rsp), %rdx
	movq	%r14, %rdi
	vzeroupper
	callq	*%rax
.Ltmp15292:
	jmp	.LBB361_444
.LBB361_438:
.Ltmp15244:
	.cfi_escape 0x2e, 0x00
	movq	alloc::raw_vec::handle_error@GOTPCREL(%rip), %rax
	movl	$4, %edi
	movl	$16, %esi
	callq	*%rax
.Ltmp15245:
	jmp	.LBB361_444
.LBB361_439:
.Ltmp15313:
	leaq	64(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15314:
	jmp	.LBB361_444
.LBB361_440:
	movl	$1, %edi
	jmp	.LBB361_18
.LBB361_441:
.Ltmp15329:
	leaq	64(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15330:
	jmp	.LBB361_444
.LBB361_442:
.Ltmp15404:
	leaq	64(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15405:
	jmp	.LBB361_444
.LBB361_443:
.Ltmp15353:
	leaq	64(%rsp), %rbx
	.cfi_escape 0x2e, 0x00
	movq	alloc::alloc::handle_alloc_error@GOTPCREL(%rip), %rax
	movl	$8, %edi
	movl	$72, %esi
	callq	*%rax
.Ltmp15354:
.LBB361_444:
	ud2
.LBB361_445:
.Ltmp15281:
	jmp	.LBB361_473
.LBB361_446:
.Ltmp15373:
	movq	%rax, %r14
.Ltmp15374:
	.cfi_escape 0x2e, 0x00
	movq	%r15, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15375:
	jmp	.LBB361_450
.LBB361_447:
.Ltmp15376:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_448:
.Ltmp15368:
	movq	%rax, %r14
.Ltmp15369:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15370:
	jmp	.LBB361_450
.LBB361_449:
.Ltmp15365:
	movq	%rax, %r14
.LBB361_450:
	movb	$1, %bpl
.Ltmp15377:
	.cfi_escape 0x2e, 0x00
	leaq	272(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
.Ltmp15378:
	movb	$1, %bl
	jmp	.LBB361_527
.LBB361_451:
.Ltmp15355:
	movq	%rax, %r14
.Ltmp15356:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15357:
	jmp	.LBB361_470
.LBB361_452:
.Ltmp15358:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_453:
.Ltmp15342:
	lock		decq	(%r15)
	movq	%rax, %r14
	jne	.LBB361_467
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp15343:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	callq	*%rax
.Ltmp15344:
	movb	$1, %bl
	jmp	.LBB361_527
.LBB361_455:
.Ltmp15406:
	movq	%rax, %r14
.Ltmp15407:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15408:
	jmp	.LBB361_470
.LBB361_456:
.Ltmp15409:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_457:
.Ltmp15391:
	lock		decq	(%rbx)
	movq	%rax, %r14
	jne	.LBB361_467
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp15392:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	48(%rsp), %rdi
	callq	*%rax
.Ltmp15393:
	movb	$1, %bl
	jmp	.LBB361_527
.LBB361_459:
.Ltmp15349:
	movq	%rax, %r14
.Ltmp15350:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15351:
	jmp	.LBB361_470
.LBB361_460:
.Ltmp15352:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_461:
.Ltmp15331:
	movq	%rax, %r14
.Ltmp15332:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15333:
	jmp	.LBB361_465
.LBB361_462:
.Ltmp15334:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_463:
.Ltmp15324:
	movq	%rax, %r14
.Ltmp15325:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15326:
	jmp	.LBB361_465
.LBB361_464:
.Ltmp15321:
	movq	%rax, %r14
.LBB361_465:
	lock		decq	(%r15)
	jne	.LBB361_467
	movb	$1, %bpl
	#MEMBARRIER
.Ltmp15335:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	272(%rsp), %rdi
	callq	*%rax
.Ltmp15336:
	movb	$1, %bl
	jmp	.LBB361_527
.LBB361_467:
	movb	$1, %bl
	jmp	.LBB361_477
.LBB361_468:
.Ltmp15337:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_469:
.Ltmp15398:
	movq	%rax, %r14
.Ltmp15399:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15400:
.LBB361_470:
	movb	$1, %bpl
	movb	$1, %bl
	jmp	.LBB361_527
.LBB361_471:
.Ltmp15232:
	movq	%rax, %r14
	xorl	%ebx, %ebx
	jmp	.LBB361_486
.LBB361_472:
.Ltmp15284:
.LBB361_473:
	movq	%rax, %r14
	movq	%r15, 600(%rsp)
	jmp	.LBB361_502
.LBB361_474:
.Ltmp15403:
	movq	%rax, %r14
	xorl	%ebx, %ebx
	jmp	.LBB361_528
.LBB361_475:
.Ltmp15385:
	movl	16(%rsp), %ebx
	movq	%rax, %r14
	jmp	.LBB361_530
.LBB361_476:
.Ltmp15388:
	movl	16(%rsp), %ebx
	movq	%rax, %r14
.LBB361_477:
	movb	$1, %bpl
	jmp	.LBB361_527
.LBB361_478:
.Ltmp15270:
	movq	%rax, %r14
	jmp	.LBB361_525
.LBB361_479:
.Ltmp15227:
	movq	%rax, %r14
.Ltmp15228:
	.cfi_escape 0x2e, 0x00
	leaq	624(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15229:
	xorl	%ebx, %ebx
	jmp	.LBB361_486
.LBB361_481:
.Ltmp15315:
	movq	%rax, %r14
.Ltmp15316:
	.cfi_escape 0x2e, 0x00
	movq	%rbx, %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15317:
	movb	$1, %bl
	xorl	%ebp, %ebp
	jmp	.LBB361_527
.LBB361_483:
.Ltmp15318:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_484:
.Ltmp15246:
	movq	%rax, %r14
	jmp	.LBB361_531
.LBB361_485:
.Ltmp15222:
	movq	%rax, %r14
.LBB361_486:
.Ltmp15233:
	.cfi_escape 0x2e, 0x00
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.10720091597982897309)
.Ltmp15234:
	testb	%bl, %bl
	jne	.LBB361_534
	jmp	.LBB361_557
.LBB361_488:
.Ltmp15249:
	movq	%rax, %r14
	testq	%r15, %r15
	je	.LBB361_531
	movq	40(%rsp), %rsi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movq	304(%rsp), %rdi
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_531
.LBB361_490:
.Ltmp15312:
	movq	%rax, %r14
	movq	936(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_520
	movq	944(%rsp), %rdi
	leaq	(%rax,%rax,2), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$1, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_520
.LBB361_492:
.Ltmp15304:
	movq	%rax, %r14
	jmp	.LBB361_517
.LBB361_493:
.Ltmp15243:
	movq	48(%rsp), %rsi
	movq	%rax, %r14
	testq	%rsi, %rsi
	je	.LBB361_531
	movq	56(%rsp), %rdi
	shlq	$2, %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	callq	__rustc::__rust_dealloc
	jmp	.LBB361_531
.LBB361_495:
.Ltmp15307:
	vmovups	72(%rsp), %ymm1
	vmovdqu	48(%rsp), %ymm0
	movq	%rax, %r14
	vmovups	%ymm1, 488(%rsp)
	vmovdqu	%ymm0, 464(%rsp)
	jmp	.LBB361_517
.LBB361_496:
.Ltmp15273:
	movb	$1, %bl
	movq	%rax, %r14
	jmp	.LBB361_513
.LBB361_497:
.Ltmp15296:
	movq	%rax, %r14
	movq	%r15, 184(%rsp)
	jmp	.LBB361_507
.LBB361_498:
.Ltmp15278:
	movb	$1, %bl
	movq	%rax, %r14
	jmp	.LBB361_511
.LBB361_499:
.Ltmp15290:
	movq	%rax, %r14
	movq	%r15, 600(%rsp)
	cmpq	$5, %rbp
	ja	.LBB361_504
	jmp	.LBB361_505
.LBB361_500:
.Ltmp15263:
	jmp	.LBB361_523
.LBB361_501:
.Ltmp15287:
	movq	%rax, %r14
.LBB361_502:
	movq	176(%rsp), %rbp
	cmpq	$6, %rbp
	jb	.LBB361_505
	movq	184(%rsp), %rbx
.LBB361_504:
	leaq	-8(,%rbp,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB361_505:
	.cfi_escape 0x2e, 0x00
	leaq	592(%rsp), %rdi
	jmp	.LBB361_510
.LBB361_506:
.Ltmp15293:
	movq	%rax, %r14
.LBB361_507:
	cmpq	$5, %rbp
	jbe	.LBB361_509
	leaq	-8(,%rbp,8), %rsi
	.cfi_escape 0x2e, 0x00
	movl	$4, %edx
	movq	%rbx, %rdi
	callq	__rustc::__rust_dealloc
.LBB361_509:
	.cfi_escape 0x2e, 0x00
	leaq	176(%rsp), %rdi
.LBB361_510:
	callq	core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	xorl	%ebx, %ebx
.LBB361_511:
.Ltmp15297:
	.cfi_escape 0x2e, 0x00
	leaq	48(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15298:
	movq	440(%rsp), %rax
	movq	%rax, 320(%rsp)
.LBB361_513:
	movq	320(%rsp), %rax
	lock		decq	(%rax)
	jne	.LBB361_515
	#MEMBARRIER
.Ltmp15299:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	leaq	440(%rsp), %rdi
	callq	*%rax
.Ltmp15300:
.LBB361_515:
	testb	%bl, %bl
	je	.LBB361_517
	.cfi_escape 0x2e, 0x00
	leaq	416(%rsp), %rdi
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
.LBB361_517:
	cmpb	$0, 896(%rsp)
	jne	.LBB361_520
	cmpq	$-1, 904(%rsp)
	je	.LBB361_520
.Ltmp15308:
	.cfi_escape 0x2e, 0x00
	leaq	904(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>>
.Ltmp15309:
	movq	40(%rsp), %r15
	movb	$1, %bl
	jmp	.LBB361_525
.LBB361_520:
	movq	40(%rsp), %r15
	jmp	.LBB361_524
.LBB361_521:
.Ltmp15301:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_522:
.Ltmp15258:
.LBB361_523:
	movq	%rax, %r14
.LBB361_524:
	movb	$1, %bl
.LBB361_525:
	movb	$1, %bpl
	testq	%r15, %r15
	je	.LBB361_527
	shlq	$2, %r15
	.cfi_escape 0x2e, 0x00
	movq	304(%rsp), %rdi
	movl	$4, %edx
	movq	%r15, %rsi
	vzeroupper
	callq	__rustc::__rust_dealloc
.LBB361_527:
	.cfi_escape 0x2e, 0x00
	leaq	328(%rsp), %rdi
	vzeroupper
	callq	core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
	movq	464(%rsp), %r12
.LBB361_528:
	cmpq	$-1, %r12
	setne	%al
	testb	%bpl, %al
	je	.LBB361_530
.Ltmp15410:
	.cfi_escape 0x2e, 0x00
	leaq	464(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.10720091597982897309)
.Ltmp15411:
.LBB361_530:
	testb	%bl, %bl
	je	.LBB361_557
.LBB361_531:
.Ltmp15412:
	.cfi_escape 0x2e, 0x00
	leaq	792(%rsp), %rdi
	callq	core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
.Ltmp15413:
	jmp	.LBB361_557
.LBB361_532:
.Ltmp15414:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.LBB361_533:
.Ltmp15237:
	movq	%rax, %r14
.LBB361_534:
	movq	696(%rsp), %rax
	cmpq	$6, %rax
	jb	.LBB361_535
	leaq	-3(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	704(%rsp), %rdi
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
	jge	.LBB361_539
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_539:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_545
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_539
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
.LBB361_542:
	cmpq	%rax, %rsi
	jge	.LBB361_544
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_542
.LBB361_544:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_545:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	624(%rsp), %rax
	testq	%rax, %rax
	jg	.LBB361_546
.LBB361_536:
	movq	720(%rsp), %rax
	testq	%rax, %rax
	jne	.LBB361_555
	jmp	.LBB361_557
.LBB361_535:
	movq	624(%rsp), %rax
	testq	%rax, %rax
	jle	.LBB361_536
.LBB361_546:
	leaq	(%rax,%rax,2), %rcx
	movq	%fs:purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF, %rax
	movabsq	$9223372036854775807, %rdx
	movq	632(%rsp), %rdi
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
	jge	.LBB361_548
	movq	%rax, %fs:purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601)@TPOFF
	.p2align	4
.LBB361_548:
	movq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip), %rax
	testq	%rax, %rax
	jns	.LBB361_554
	leaq	1(%rax), %rsi
	lock		cmpxchgq	%rsi, purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
	jne	.LBB361_548
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
.LBB361_551:
	cmpq	%rax, %rsi
	jge	.LBB361_553
	lock		cmpxchgq	%rsi, (%rcx)
	jne	.LBB361_551
.LBB361_553:
	lock		decq	purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601)(%rip)
.LBB361_554:
	.cfi_escape 0x2e, 0x00
	movq	free@GOTPCREL(%rip), %rax
	callq	*%rax
	movq	720(%rsp), %rax
	testq	%rax, %rax
	je	.LBB361_557
.LBB361_555:
	lock		decq	(%rax)
	jne	.LBB361_557
	leaq	720(%rsp), %rdi
	#MEMBARRIER
.Ltmp15238:
	.cfi_escape 0x2e, 0x00
	movq	<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow@GOTPCREL(%rip), %rax
	callq	*%rax
.Ltmp15239:
.LBB361_557:
	.cfi_escape 0x2e, 0x00
	movq	%r14, %rdi
	callq	_Unwind_Resume@PLT
.LBB361_558:
.Ltmp15240:
	.cfi_escape 0x2e, 0x00
	movq	core::panicking::panic_in_cleanup@GOTPCREL(%rip), %rax
	callq	*%rax
.Lfunc_end361:

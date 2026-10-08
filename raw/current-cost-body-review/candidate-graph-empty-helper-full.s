purrdf_sparql_eval::modifier::graph_holds_no_rows::<purrdf_core::ir::dataset::RdfDataset>:
.Lfunc_begin367:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	pushq	%r15
	.cfi_def_cfa_offset 24
	pushq	%r14
	.cfi_def_cfa_offset 32
	pushq	%r12
	.cfi_def_cfa_offset 40
	pushq	%rbx
	.cfi_def_cfa_offset 48
	subq	$112, %rsp
	.cfi_def_cfa_offset 160
	.cfi_offset %rbx, -48
	.cfi_offset %r12, -40
	.cfi_offset %r14, -32
	.cfi_offset %r15, -24
	.cfi_offset %rbp, -16
	movq	<purrdf_core::ir::dataset::RdfDataset>::probe_plan@GOTPCREL(%rip), %rax
	movl	$2, %ecx
	movl	%esi, %ebx
	movq	%rdi, %r14
	xorl	%ebp, %ebp
	xorl	%edi, %edi
	xorl	%esi, %esi
	xorl	%edx, %edx
	callq	*%rax
	movq	%rax, 8(%rsp)
	movq	<purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan@GOTPCREL(%rip), %rax
	leaq	24(%rsp), %r12
	leaq	8(%rsp), %r15
	movb	%dl, 16(%rsp)
	movq	%r14, %rsi
	xorl	%ecx, %ecx
	xorl	%r8d, %r8d
	xorl	%r9d, %r9d
	movq	%r12, %rdi
	movq	%r15, %rdx
	pushq	%rbx
	.cfi_adjust_cfa_offset 8
	pushq	$2
	.cfi_adjust_cfa_offset 8
	callq	*%rax
	addq	$16, %rsp
	.cfi_adjust_cfa_offset -16
	movq	%r12, %rdi
	callq	<purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next
	testq	%rax, %rax
	jne	.LBB367_5
	cmpq	$0, 80(%r14)
	je	.LBB367_3
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158(%rip), %rsi
	movl	$50, %edx
	movq	%r14, %rdi
	callq	*%rax
	testl	%eax, %eax
	je	.LBB367_13
.LBB367_3:
	movq	<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri@GOTPCREL(%rip), %rax
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158(%rip), %rsi
	movl	$50, %edx
	movq	%r14, %rdi
	callq	*%rax
	leaq	8(%rsp), %rdi
	leaq	24(%rsp), %rsi
	movq	%r14, 32(%rsp)
	movl	%eax, 40(%rsp)
	movq	$0, 48(%rsp)
	movq	$0, 72(%rsp)
	movl	$2, 24(%rsp)
	movl	%ebx, 28(%rsp)
	callq	<core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next
	cmpl	$0, 8(%rsp)
	je	.LBB367_6
.LBB367_4:
	xorl	%ebp, %ebp
.LBB367_5:
	movl	%ebp, %eax
	addq	$112, %rsp
	.cfi_def_cfa_offset 48
	popq	%rbx
	.cfi_def_cfa_offset 40
	popq	%r12
	.cfi_def_cfa_offset 32
	popq	%r14
	.cfi_def_cfa_offset 24
	popq	%r15
	.cfi_def_cfa_offset 16
	popq	%rbp
	.cfi_def_cfa_offset 8
	retq
.LBB367_6:
	.cfi_def_cfa_offset 160
	movq	96(%r14), %rax
	movb	$1, %bpl
	testq	%rax, %rax
	je	.LBB367_5
	movq	88(%r14), %rcx
	shlq	$4, %rax
	xorl	%edx, %edx
	jmp	.LBB367_9
	.p2align	4
.LBB367_8:
	addq	$16, %rdx
	cmpq	%rdx, %rax
	je	.LBB367_5
.LBB367_9:
	movl	12(%rcx,%rdx), %esi
	testl	%esi, %esi
	je	.LBB367_8
	cmpl	%ebx, %esi
	jne	.LBB367_8
	cmpl	$0, (%rcx,%rdx)
	je	.LBB367_8
	jmp	.LBB367_4
.LBB367_13:
	movq	80(%r14), %rax
	movq	<usize as core::fmt::Display>::fmt@GOTPCREL(%rip), %rcx
	movq	core::panicking::panic_fmt@GOTPCREL(%rip), %rbx
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4165.llvm.6298868053391388158(%rip), %rdi
	leaq	anon.4c9120963acf3a9d820f038c36ebab01.4166.llvm.6298868053391388158(%rip), %rdx
	leaq	24(%rsp), %rsi
	movq	%r15, 24(%rsp)
	movq	%rax, 8(%rsp)
	movq	%rcx, 32(%rsp)
	callq	*%rbx
.Lfunc_end367:

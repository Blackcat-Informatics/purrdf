
.stage/sparql-eval-datalog-per-row-evidence/distinct-fixed-work-current:     file format elf64-x86-64


Disassembly of section .text:

00000000007068b0 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push>:
  7068b0:	55                   	push   %rbp
  7068b1:	41 57                	push   %r15
  7068b3:	41 56                	push   %r14
  7068b5:	41 55                	push   %r13
  7068b7:	41 54                	push   %r12
  7068b9:	53                   	push   %rbx
  7068ba:	48 81 ec f8 00 00 00 	sub    $0xf8,%rsp
  7068c1:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  7068c6:	49 89 f7             	mov    %rsi,%r15
  7068c9:	48 89 7c 24 38       	mov    %rdi,0x38(%rsp)
  7068ce:	48 b8 0b 00 00 00 00 	movabs $0x800000000000000b,%rax
  7068d5:	00 00 80 
  7068d8:	4c 8b 76 30          	mov    0x30(%rsi),%r14
  7068dc:	48 8b 5e 18          	mov    0x18(%rsi),%rbx
  7068e0:	4c 8b 6e 28          	mov    0x28(%rsi),%r13
  7068e4:	4c 8d 60 ff          	lea    -0x1(%rax),%r12
  7068e8:	4c 89 64 24 40       	mov    %r12,0x40(%rsp)
  7068ed:	48 8d 7c 24 40       	lea    0x40(%rsp),%rdi
  7068f2:	e8 09 4f d1 ff       	call   41b800 <core::ptr::drop_glue::<purrdf_sparql_eval::error::EvalError>>
  7068f7:	49 8d 6f 18          	lea    0x18(%r15),%rbp
  7068fb:	49 39 dd             	cmp    %rbx,%r13
  7068fe:	72 6c                	jb     70696c <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xbc>
  706900:	49 8d 45 01          	lea    0x1(%r13),%rax
  706904:	48 8d 0c 1b          	lea    (%rbx,%rbx,1),%rcx
  706908:	48 39 c1             	cmp    %rax,%rcx
  70690b:	48 0f 47 c1          	cmova  %rcx,%rax
  70690f:	48 83 f8 05          	cmp    $0x5,%rax
  706913:	bd 04 00 00 00       	mov    $0x4,%ebp
  706918:	48 0f 43 e8          	cmovae %rax,%rbp
  70691c:	48 85 db             	test   %rbx,%rbx
  70691f:	48 0f 45 e8          	cmovne %rax,%rbp
  706923:	48 39 dd             	cmp    %rbx,%rbp
  706926:	76 40                	jbe    706968 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xb8>
  706928:	48 b9 34 33 33 33 33 	movabs $0x333333333333334,%rcx
  70692f:	33 33 03 
  706932:	48 ff c9             	dec    %rcx
  706935:	48 39 cd             	cmp    %rcx,%rbp
  706938:	0f 87 13 01 00 00    	ja     706a51 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x1a1>
  70693e:	4d 85 f6             	test   %r14,%r14
  706941:	0f 84 3c 02 00 00    	je     706b83 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2d3>
  706947:	49 8b 3e             	mov    (%r14),%rdi
  70694a:	49 8b 46 08          	mov    0x8(%r14),%rax
  70694e:	ff 50 20             	call   *0x20(%rax)
  706951:	84 c0                	test   %al,%al
  706953:	74 77                	je     7069cc <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x11c>
  706955:	49 bc 0b 00 00 00 00 	movabs $0x800000000000000b,%r12
  70695c:	00 00 80 
  70695f:	49 83 c4 fb          	add    $0xfffffffffffffffb,%r12
  706963:	e9 a9 00 00 00       	jmp    706a11 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x161>
  706968:	49 8d 6f 18          	lea    0x18(%r15),%rbp
  70696c:	49 39 dd             	cmp    %rbx,%r13
  70696f:	75 09                	jne    70697a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xca>
  706971:	48 89 ef             	mov    %rbp,%rdi
  706974:	ff 15 ae 55 29 00    	call   *0x2955ae(%rip)        # 99bf28 <_DYNAMIC+0x1550>
  70697a:	49 8b 47 20          	mov    0x20(%r15),%rax
  70697e:	4a 8d 0c ad 00 00 00 	lea    0x0(,%r13,4),%rcx
  706985:	00 
  706986:	4c 01 e9             	add    %r13,%rcx
  706989:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  70698e:	48 8b 56 20          	mov    0x20(%rsi),%rdx
  706992:	48 89 54 c8 20       	mov    %rdx,0x20(%rax,%rcx,8)
  706997:	0f 10 06             	movups (%rsi),%xmm0
  70699a:	0f 10 4e 10          	movups 0x10(%rsi),%xmm1
  70699e:	0f 11 4c c8 10       	movups %xmm1,0x10(%rax,%rcx,8)
  7069a3:	0f 11 04 c8          	movups %xmm0,(%rax,%rcx,8)
  7069a7:	49 ff c5             	inc    %r13
  7069aa:	4d 89 6f 28          	mov    %r13,0x28(%r15)
  7069ae:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
  7069b3:	48 c7 00 ff ff ff ff 	movq   $0xffffffffffffffff,(%rax)
  7069ba:	48 81 c4 f8 00 00 00 	add    $0xf8,%rsp
  7069c1:	5b                   	pop    %rbx
  7069c2:	41 5c                	pop    %r12
  7069c4:	41 5d                	pop    %r13
  7069c6:	41 5e                	pop    %r14
  7069c8:	41 5f                	pop    %r15
  7069ca:	5d                   	pop    %rbp
  7069cb:	c3                   	ret
  7069cc:	48 8d 04 ed 00 00 00 	lea    0x0(,%rbp,8),%rax
  7069d3:	00 
  7069d4:	48 8d 14 80          	lea    (%rax,%rax,4),%rdx
  7069d8:	49 8b 36             	mov    (%r14),%rsi
  7069db:	49 8b 46 08          	mov    0x8(%r14),%rax
  7069df:	48 8d 7c 24 40       	lea    0x40(%rsp),%rdi
  7069e4:	ff 50 28             	call   *0x28(%rax)
  7069e7:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
  7069ec:	4c 8b 64 24 48       	mov    0x48(%rsp),%r12
  7069f1:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
  7069f6:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
  7069fa:	0f 84 86 01 00 00    	je     706b86 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2d6>
  706a00:	48 89 c2             	mov    %rax,%rdx
  706a03:	48 8b 44 24 58       	mov    0x58(%rsp),%rax
  706a08:	49 89 d6             	mov    %rdx,%r14
  706a0b:	4d 89 e7             	mov    %r12,%r15
  706a0e:	49 89 cc             	mov    %rcx,%r12
  706a11:	0f 28 44 24 60       	movaps 0x60(%rsp),%xmm0
  706a16:	0f 28 4c 24 70       	movaps 0x70(%rsp),%xmm1
  706a1b:	0f 28 94 24 80 00 00 	movaps 0x80(%rsp),%xmm2
  706a22:	00 
  706a23:	0f 28 9c 24 90 00 00 	movaps 0x90(%rsp),%xmm3
  706a2a:	00 
  706a2b:	0f 29 9c 24 e0 00 00 	movaps %xmm3,0xe0(%rsp)
  706a32:	00 
  706a33:	0f 29 94 24 d0 00 00 	movaps %xmm2,0xd0(%rsp)
  706a3a:	00 
  706a3b:	0f 29 8c 24 c0 00 00 	movaps %xmm1,0xc0(%rsp)
  706a42:	00 
  706a43:	0f 29 84 24 b0 00 00 	movaps %xmm0,0xb0(%rsp)
  706a4a:	00 
  706a4b:	4c 89 f2             	mov    %r14,%rdx
  706a4e:	4c 89 fe             	mov    %r15,%rsi
  706a51:	48 8b 4c 24 38       	mov    0x38(%rsp),%rcx
  706a56:	4c 89 21             	mov    %r12,(%rcx)
  706a59:	48 89 71 08          	mov    %rsi,0x8(%rcx)
  706a5d:	48 89 51 10          	mov    %rdx,0x10(%rcx)
  706a61:	48 89 41 18          	mov    %rax,0x18(%rcx)
  706a65:	0f 28 84 24 b0 00 00 	movaps 0xb0(%rsp),%xmm0
  706a6c:	00 
  706a6d:	0f 28 8c 24 c0 00 00 	movaps 0xc0(%rsp),%xmm1
  706a74:	00 
  706a75:	0f 28 94 24 d0 00 00 	movaps 0xd0(%rsp),%xmm2
  706a7c:	00 
  706a7d:	0f 28 9c 24 e0 00 00 	movaps 0xe0(%rsp),%xmm3
  706a84:	00 
  706a85:	0f 29 59 50          	movaps %xmm3,0x50(%rcx)
  706a89:	0f 29 51 40          	movaps %xmm2,0x40(%rcx)
  706a8d:	0f 29 49 30          	movaps %xmm1,0x30(%rcx)
  706a91:	0f 29 41 20          	movaps %xmm0,0x20(%rcx)
  706a95:	48 8b 4c 24 10       	mov    0x10(%rsp),%rcx
  706a9a:	48 8b 01             	mov    (%rcx),%rax
  706a9d:	48 85 c0             	test   %rax,%rax
  706aa0:	74 19                	je     706abb <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x20b>
  706aa2:	48 83 f8 06          	cmp    $0x6,%rax
  706aa6:	0f 82 0e ff ff ff    	jb     7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706aac:	48 8b 79 08          	mov    0x8(%rcx),%rdi
  706ab0:	ff 15 2a 41 29 00    	call   *0x29412a(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706ab6:	e9 ff fe ff ff       	jmp    7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706abb:	48 8b 79 10          	mov    0x10(%rcx),%rdi
  706abf:	80 79 08 00          	cmpb   $0x0,0x8(%rcx)
  706ac3:	74 60                	je     706b25 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x275>
  706ac5:	f0 48 ff 4f 40       	lock decq 0x40(%rdi)
  706aca:	0f 85 ea fe ff ff    	jne    7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706ad0:	0f 10 07             	movups (%rdi),%xmm0
  706ad3:	0f 10 4f 10          	movups 0x10(%rdi),%xmm1
  706ad7:	0f 10 57 20          	movups 0x20(%rdi),%xmm2
  706adb:	0f 10 5f 30          	movups 0x30(%rdi),%xmm3
  706adf:	0f 29 5c 24 70       	movaps %xmm3,0x70(%rsp)
  706ae4:	0f 29 54 24 60       	movaps %xmm2,0x60(%rsp)
  706ae9:	0f 29 4c 24 50       	movaps %xmm1,0x50(%rsp)
  706aee:	0f 29 44 24 40       	movaps %xmm0,0x40(%rsp)
  706af3:	ff 15 e7 40 29 00    	call   *0x2940e7(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706af9:	48 8d 7c 24 40       	lea    0x40(%rsp),%rdi
  706afe:	e8 cd ae ce ff       	call   3f19d0 <core::ptr::drop_glue::<purrdf_sparql_eval::solution::TrackedRow<purrdf_core::ir::term::TermId>>>
  706b03:	48 8b 5c 24 70       	mov    0x70(%rsp),%rbx
  706b08:	48 85 db             	test   %rbx,%rbx
  706b0b:	0f 84 a9 fe ff ff    	je     7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706b11:	4c 8b 7c 24 78       	mov    0x78(%rsp),%r15
  706b16:	49 8b 07             	mov    (%r15),%rax
  706b19:	48 85 c0             	test   %rax,%rax
  706b1c:	74 4c                	je     706b6a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2ba>
  706b1e:	48 89 df             	mov    %rbx,%rdi
  706b21:	ff d0                	call   *%rax
  706b23:	eb 45                	jmp    706b6a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2ba>
  706b25:	f0 48 ff 4f 38       	lock decq 0x38(%rdi)
  706b2a:	0f 85 8a fe ff ff    	jne    7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706b30:	4c 8b 27             	mov    (%rdi),%r12
  706b33:	4c 8b 77 08          	mov    0x8(%rdi),%r14
  706b37:	48 8b 5f 28          	mov    0x28(%rdi),%rbx
  706b3b:	4c 8b 7f 30          	mov    0x30(%rdi),%r15
  706b3f:	ff 15 9b 40 29 00    	call   *0x29409b(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706b45:	49 83 fc 06          	cmp    $0x6,%r12
  706b49:	72 09                	jb     706b54 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2a4>
  706b4b:	4c 89 f7             	mov    %r14,%rdi
  706b4e:	ff 15 8c 40 29 00    	call   *0x29408c(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706b54:	48 85 db             	test   %rbx,%rbx
  706b57:	0f 84 5d fe ff ff    	je     7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706b5d:	49 8b 07             	mov    (%r15),%rax
  706b60:	48 85 c0             	test   %rax,%rax
  706b63:	74 05                	je     706b6a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2ba>
  706b65:	48 89 df             	mov    %rbx,%rdi
  706b68:	ff d0                	call   *%rax
  706b6a:	49 83 7f 08 00       	cmpq   $0x0,0x8(%r15)
  706b6f:	0f 84 45 fe ff ff    	je     7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706b75:	48 89 df             	mov    %rbx,%rdi
  706b78:	ff 15 62 40 29 00    	call   *0x294062(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706b7e:	e9 37 fe ff ff       	jmp    7069ba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x10a>
  706b83:	45 31 e4             	xor    %r12d,%r12d
  706b86:	4c 89 a4 24 a0 00 00 	mov    %r12,0xa0(%rsp)
  706b8d:	00 
  706b8e:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  706b93:	48 89 84 24 a8 00 00 	mov    %rax,0xa8(%rsp)
  706b9a:	00 
  706b9b:	48 8d 7c 24 40       	lea    0x40(%rsp),%rdi
  706ba0:	ba 08 00 00 00       	mov    $0x8,%edx
  706ba5:	41 b8 08 00 00 00    	mov    $0x8,%r8d
  706bab:	41 b9 28 00 00 00    	mov    $0x28,%r9d
  706bb1:	31 f6                	xor    %esi,%esi
  706bb3:	48 89 e9             	mov    %rbp,%rcx
  706bb6:	e8 65 5e f9 ff       	call   69ca20 <<alloc::raw_vec::RawVecInner>::finish_grow>
  706bbb:	80 7c 24 40 00       	cmpb   $0x0,0x40(%rsp)
  706bc0:	74 51                	je     706c13 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x363>
  706bc2:	4c 8d 3d 51 17 99 ff 	lea    -0x66e8af(%rip),%r15        # 9831a <anon.44049c1f6db1c90388586f067021d866.358.llvm.15447233937092479704+0x14d>
  706bc9:	4d 85 e4             	test   %r12,%r12
  706bcc:	0f 84 22 01 00 00    	je     706cf4 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x444>
  706bd2:	4c 89 e3             	mov    %r12,%rbx
  706bd5:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  706bda:	48 8b 00             	mov    (%rax),%rax
  706bdd:	48 85 c0             	test   %rax,%rax
  706be0:	49 bc 0b 00 00 00 00 	movabs $0x800000000000000b,%r12
  706be7:	00 00 80 
  706bea:	74 05                	je     706bf1 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x341>
  706bec:	48 89 df             	mov    %rbx,%rdi
  706bef:	ff d0                	call   *%rax
  706bf1:	41 be 16 00 00 00    	mov    $0x16,%r14d
  706bf7:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  706bfc:	48 83 78 08 00       	cmpq   $0x0,0x8(%rax)
  706c01:	74 09                	je     706c0c <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x35c>
  706c03:	48 89 df             	mov    %rbx,%rdi
  706c06:	ff 15 d4 3f 29 00    	call   *0x293fd4(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706c0c:	31 c0                	xor    %eax,%eax
  706c0e:	e9 38 fe ff ff       	jmp    706a4b <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x19b>
  706c13:	48 8b 44 24 48       	mov    0x48(%rsp),%rax
  706c18:	48 89 6c 24 18       	mov    %rbp,0x18(%rsp)
  706c1d:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
  706c22:	48 c7 44 24 28 00 00 	movq   $0x0,0x28(%rsp)
  706c29:	00 00 
  706c2b:	49 8b 4f 20          	mov    0x20(%r15),%rcx
  706c2f:	48 89 4c 24 30       	mov    %rcx,0x30(%rsp)
  706c34:	49 39 ed             	cmp    %rbp,%r13
  706c37:	0f 87 ce 00 00 00    	ja     706d0b <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x45b>
  706c3d:	45 31 f6             	xor    %r14d,%r14d
  706c40:	4d 85 ed             	test   %r13,%r13
  706c43:	49 8d 6f 18          	lea    0x18(%r15),%rbp
  706c47:	74 1f                	je     706c68 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3b8>
  706c49:	4b 8d 0c b6          	lea    (%r14,%r14,4),%rcx
  706c4d:	48 8d 3c c8          	lea    (%rax,%rcx,8),%rdi
  706c51:	4a 8d 04 ed 00 00 00 	lea    0x0(,%r13,8),%rax
  706c58:	00 
  706c59:	48 8d 14 80          	lea    (%rax,%rax,4),%rdx
  706c5d:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
  706c62:	ff 15 88 3f 29 00    	call   *0x293f88(%rip)        # 99abf0 <memcpy@GLIBC_2.14>
  706c68:	4d 01 ee             	add    %r13,%r14
  706c6b:	4c 89 74 24 28       	mov    %r14,0x28(%rsp)
  706c70:	49 c7 47 28 00 00 00 	movq   $0x0,0x28(%r15)
  706c77:	00 
  706c78:	48 85 db             	test   %rbx,%rbx
  706c7b:	74 0b                	je     706c88 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x3d8>
  706c7d:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
  706c82:	ff 15 58 3f 29 00    	call   *0x293f58(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706c88:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
  706c8d:	48 89 45 10          	mov    %rax,0x10(%rbp)
  706c91:	0f 10 44 24 18       	movups 0x18(%rsp),%xmm0
  706c96:	0f 11 45 00          	movups %xmm0,0x0(%rbp)
  706c9a:	49 83 3f 00          	cmpq   $0x0,(%r15)
  706c9e:	74 2a                	je     706cca <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x41a>
  706ca0:	4d 8b 6f 08          	mov    0x8(%r15),%r13
  706ca4:	4d 85 ed             	test   %r13,%r13
  706ca7:	74 21                	je     706cca <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x41a>
  706ca9:	49 8b 5f 10          	mov    0x10(%r15),%rbx
  706cad:	48 8b 03             	mov    (%rbx),%rax
  706cb0:	48 85 c0             	test   %rax,%rax
  706cb3:	74 05                	je     706cba <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x40a>
  706cb5:	4c 89 ef             	mov    %r13,%rdi
  706cb8:	ff d0                	call   *%rax
  706cba:	48 83 7b 08 00       	cmpq   $0x0,0x8(%rbx)
  706cbf:	74 09                	je     706cca <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x41a>
  706cc1:	4c 89 ef             	mov    %r13,%rdi
  706cc4:	ff 15 16 3f 29 00    	call   *0x293f16(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706cca:	49 c7 07 01 00 00 00 	movq   $0x1,(%r15)
  706cd1:	4d 89 67 08          	mov    %r12,0x8(%r15)
  706cd5:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  706cda:	49 89 47 10          	mov    %rax,0x10(%r15)
  706cde:	49 8b 5f 18          	mov    0x18(%r15),%rbx
  706ce2:	4d 8b 6f 28          	mov    0x28(%r15),%r13
  706ce6:	49 39 dd             	cmp    %rbx,%r13
  706ce9:	0f 84 82 fc ff ff    	je     706971 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xc1>
  706cef:	e9 86 fc ff ff       	jmp    70697a <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xca>
  706cf4:	41 be 16 00 00 00    	mov    $0x16,%r14d
  706cfa:	31 c0                	xor    %eax,%eax
  706cfc:	49 bc 0b 00 00 00 00 	movabs $0x800000000000000b,%r12
  706d03:	00 00 80 
  706d06:	e9 40 fd ff ff       	jmp    706a4b <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x19b>
  706d0b:	48 8d 7c 24 18       	lea    0x18(%rsp),%rdi
  706d10:	b9 08 00 00 00       	mov    $0x8,%ecx
  706d15:	41 b8 28 00 00 00    	mov    $0x28,%r8d
  706d1b:	31 f6                	xor    %esi,%esi
  706d1d:	4c 89 ea             	mov    %r13,%rdx
  706d20:	e8 fb 13 ed ff       	call   5d8120 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
  706d25:	48 8b 44 24 20       	mov    0x20(%rsp),%rax
  706d2a:	4c 8b 74 24 28       	mov    0x28(%rsp),%r14
  706d2f:	49 8d 6f 18          	lea    0x18(%r15),%rbp
  706d33:	e9 11 ff ff ff       	jmp    706c49 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x399>
  706d38:	49 89 c6             	mov    %rax,%r14
  706d3b:	48 8d 7c 24 18       	lea    0x18(%rsp),%rdi
  706d40:	e8 2b dd cf ff       	call   404a70 <core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::solution::RetainedRow>>>
  706d45:	48 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%rdi
  706d4c:	00 
  706d4d:	e8 de 7d d1 ff       	call   41eb30 <core::ptr::drop_glue::<purrdf_sparql_eval::workspace::WorkspaceAllocation>>
  706d52:	e9 93 00 00 00       	jmp    706dea <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x53a>
  706d57:	ff 15 4b 3f 29 00    	call   *0x293f4b(%rip)        # 99aca8 <_DYNAMIC+0x2d0>
  706d5d:	49 89 c6             	mov    %rax,%r14
  706d60:	48 83 7b 08 00       	cmpq   $0x0,0x8(%rbx)
  706d65:	74 09                	je     706d70 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4c0>
  706d67:	4c 89 ef             	mov    %r13,%rdi
  706d6a:	ff 15 70 3e 29 00    	call   *0x293e70(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706d70:	49 c7 07 01 00 00 00 	movq   $0x1,(%r15)
  706d77:	4d 89 67 08          	mov    %r12,0x8(%r15)
  706d7b:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  706d80:	49 89 47 10          	mov    %rax,0x10(%r15)
  706d84:	eb 64                	jmp    706dea <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x53a>
  706d86:	49 89 c6             	mov    %rax,%r14
  706d89:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  706d8e:	48 83 78 08 00       	cmpq   $0x0,0x8(%rax)
  706d93:	74 55                	je     706dea <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x53a>
  706d95:	48 89 df             	mov    %rbx,%rdi
  706d98:	ff 15 42 3e 29 00    	call   *0x293e42(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706d9e:	eb 4a                	jmp    706dea <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x53a>
  706da0:	eb 00                	jmp    706da2 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x4f2>
  706da2:	49 89 c6             	mov    %rax,%r14
  706da5:	49 83 7f 08 00       	cmpq   $0x0,0x8(%r15)
  706daa:	74 48                	je     706df4 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x544>
  706dac:	48 89 df             	mov    %rbx,%rdi
  706daf:	ff 15 2b 3e 29 00    	call   *0x293e2b(%rip)        # 99abe0 <free@GLIBC_2.2.5>
  706db5:	4c 89 f7             	mov    %r14,%rdi
  706db8:	e8 63 39 25 00       	call   95a720 <_Unwind_Resume@plt>
  706dbd:	49 89 c6             	mov    %rax,%r14
  706dc0:	48 8d 7c 24 70       	lea    0x70(%rsp),%rdi
  706dc5:	e8 66 7d d1 ff       	call   41eb30 <core::ptr::drop_glue::<purrdf_sparql_eval::workspace::WorkspaceAllocation>>
  706dca:	eb 28                	jmp    706df4 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x544>
  706dcc:	ff 15 d6 3e 29 00    	call   *0x293ed6(%rip)        # 99aca8 <_DYNAMIC+0x2d0>
  706dd2:	49 89 c6             	mov    %rax,%r14
  706dd5:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  706dda:	e8 f1 72 d1 ff       	call   41e0d0 <core::ptr::drop_glue::<purrdf_sparql_eval::solution::RetainedRow>>
  706ddf:	eb 13                	jmp    706df4 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x544>
  706de1:	ff 15 c1 3e 29 00    	call   *0x293ec1(%rip)        # 99aca8 <_DYNAMIC+0x2d0>
  706de7:	49 89 c6             	mov    %rax,%r14
  706dea:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  706def:	e8 dc 72 d1 ff       	call   41e0d0 <core::ptr::drop_glue::<purrdf_sparql_eval::solution::RetainedRow>>
  706df4:	4c 89 f7             	mov    %r14,%rdi
  706df7:	e8 24 39 25 00       	call   95a720 <_Unwind_Resume@plt>
  706dfc:	ff 15 a6 3e 29 00    	call   *0x293ea6(%rip)        # 99aca8 <_DYNAMIC+0x2d0>


/home/paudley/Active/purrdf/.worktrees/purrdf-evidence-default-current/build/release/build/purrdf-sparql-eval/8e4ffe4ed9e13b7f/out/query_eval-8e4ffe4ed9e13b7f:     file format elf64-x86-64


Disassembly of section .text:

0000000000707a00 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push>:
  707a00:	55                   	push   %rbp
  707a01:	41 57                	push   %r15
  707a03:	41 56                	push   %r14
  707a05:	41 55                	push   %r13
  707a07:	41 54                	push   %r12
  707a09:	53                   	push   %rbx
  707a0a:	48 81 ec f8 00 00 00 	sub    $0xf8,%rsp
  707a11:	48 89 d3             	mov    %rdx,%rbx
  707a14:	49 89 f7             	mov    %rsi,%r15
  707a17:	49 89 fe             	mov    %rdi,%r14
  707a1a:	48 8d 7e 18          	lea    0x18(%rsi),%rdi
  707a1e:	4c 8b 66 18          	mov    0x18(%rsi),%r12
  707a22:	48 8b 6e 28          	mov    0x28(%rsi),%rbp
  707a26:	4c 39 e5             	cmp    %r12,%rbp
  707a29:	72 51                	jb     707a7c <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x7c>
  707a2b:	48 8d 45 01          	lea    0x1(%rbp),%rax
  707a2f:	4b 8d 0c 24          	lea    (%r12,%r12,1),%rcx
  707a33:	48 39 c1             	cmp    %rax,%rcx
  707a36:	48 0f 47 c1          	cmova  %rcx,%rax
  707a3a:	48 83 f8 05          	cmp    $0x5,%rax
  707a3e:	41 bd 04 00 00 00    	mov    $0x4,%r13d
  707a44:	4c 0f 43 e8          	cmovae %rax,%r13
  707a48:	4d 85 e4             	test   %r12,%r12
  707a4b:	4c 0f 45 e8          	cmovne %rax,%r13
  707a4f:	4d 39 e5             	cmp    %r12,%r13
  707a52:	76 24                	jbe    707a78 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x78>
  707a54:	48 b8 34 33 33 33 33 	movabs $0x333333333333334,%rax
  707a5b:	33 33 03 
  707a5e:	48 b9 0b 00 00 00 00 	movabs $0x800000000000000b,%rcx
  707a65:	00 00 80 
  707a68:	48 ff c8             	dec    %rax
  707a6b:	49 39 c5             	cmp    %rax,%r13
  707a6e:	76 5f                	jbe    707acf <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0xcf>
  707a70:	48 ff c9             	dec    %rcx
  707a73:	e9 0a 01 00 00       	jmp    707b82 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x182>
  707a78:	49 8d 7f 18          	lea    0x18(%r15),%rdi
  707a7c:	4c 39 e5             	cmp    %r12,%rbp
  707a7f:	75 06                	jne    707a87 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x87>
  707a81:	ff 15 e9 f5 29 00    	call   *0x29f5e9(%rip)        # 9a7070 <_DYNAMIC+0x1530>
  707a87:	49 8b 47 20          	mov    0x20(%r15),%rax
  707a8b:	48 8d 0c ad 00 00 00 	lea    0x0(,%rbp,4),%rcx
  707a92:	00 
  707a93:	48 01 e9             	add    %rbp,%rcx
  707a96:	48 8b 53 20          	mov    0x20(%rbx),%rdx
  707a9a:	48 89 54 c8 20       	mov    %rdx,0x20(%rax,%rcx,8)
  707a9f:	0f 10 03             	movups (%rbx),%xmm0
  707aa2:	0f 10 4b 10          	movups 0x10(%rbx),%xmm1
  707aa6:	0f 11 4c c8 10       	movups %xmm1,0x10(%rax,%rcx,8)
  707aab:	0f 11 04 c8          	movups %xmm0,(%rax,%rcx,8)
  707aaf:	48 ff c5             	inc    %rbp
  707ab2:	49 89 6f 28          	mov    %rbp,0x28(%r15)
  707ab6:	49 c7 06 ff ff ff ff 	movq   $0xffffffffffffffff,(%r14)
  707abd:	48 81 c4 f8 00 00 00 	add    $0xf8,%rsp
  707ac4:	5b                   	pop    %rbx
  707ac5:	41 5c                	pop    %r12
  707ac7:	41 5d                	pop    %r13
  707ac9:	41 5e                	pop    %r14
  707acb:	41 5f                	pop    %r15
  707acd:	5d                   	pop    %rbp
  707ace:	c3                   	ret
  707acf:	48 89 5c 24 08       	mov    %rbx,0x8(%rsp)
  707ad4:	49 8b 5f 30          	mov    0x30(%r15),%rbx
  707ad8:	48 85 db             	test   %rbx,%rbx
  707adb:	0f 84 c7 01 00 00    	je     707ca8 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x2a8>
  707ae1:	48 8b 3b             	mov    (%rbx),%rdi
  707ae4:	48 8b 43 08          	mov    0x8(%rbx),%rax
  707ae8:	ff 50 20             	call   *0x20(%rax)
  707aeb:	84 c0                	test   %al,%al
  707aed:	74 15                	je     707b04 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x104>
  707aef:	48 b9 0b 00 00 00 00 	movabs $0x800000000000000b,%rcx
  707af6:	00 00 80 
  707af9:	48 83 c1 fb          	add    $0xfffffffffffffffb,%rcx
  707afd:	48 8b 5c 24 08       	mov    0x8(%rsp),%rbx
  707b02:	eb 44                	jmp    707b48 <<purrdf_sparql_eval::workspace::AdmittedVec<purrdf_sparql_eval::solution::RetainedRow>>::push+0x148>
  707b04:	4a 8d 04 ed 00 00 00 	lea    0x0(,%r13,8),%rax
  707b0b:	00 
  707b0c:	48 8d 14 80          	lea    (%rax,%rax,4),%rdx
  707b10:	48 8b 33             	mov    (%rbx),%rsi
  707b13:	48                   	rex.W
  707b14:	8b                   	.byte 0x8b

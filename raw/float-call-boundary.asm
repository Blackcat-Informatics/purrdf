
/opt/.cargo/target/comparison-666b5c3f/release/numeric-source-comparison:     file format elf64-x86-64


Disassembly of section .text:

0000000000091dc0 <<purrdf_xsd::exact::decimal::Decimal>::to_f64>:
   91dc0:	55                   	push   %rbp
   91dc1:	41 57                	push   %r15
   91dc3:	41 56                	push   %r14
   91dc5:	41 55                	push   %r13
   91dc7:	41 54                	push   %r12
   91dc9:	53                   	push   %rbx
   91dca:	48 83 ec 28          	sub    $0x28,%rsp
   91dce:	4c 8b 3f             	mov    (%rdi),%r15
   91dd1:	48 8b 5f 08          	mov    0x8(%rdi),%rbx
   91dd5:	0f b6 6f 38          	movzbl 0x38(%rdi),%ebp
   91dd9:	49 89 fe             	mov    %rdi,%r14
   91ddc:	40 80 fd 02          	cmp    $0x2,%bpl
   91de0:	75 23                	jne    91e05 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x45>
   91de2:	41 8b 56 40          	mov    0x40(%r14),%edx
   91de6:	83 fa 27             	cmp    $0x27,%edx
   91de9:	73 26                	jae    91e11 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x51>
   91deb:	4c 89 ff             	mov    %r15,%rdi
   91dee:	48 89 de             	mov    %rbx,%rsi
   91df1:	48 83 c4 28          	add    $0x28,%rsp
   91df5:	5b                   	pop    %rbx
   91df6:	41 5c                	pop    %r12
   91df8:	41 5d                	pop    %r13
   91dfa:	41 5e                	pop    %r14
   91dfc:	41 5f                	pop    %r15
   91dfe:	5d                   	pop    %rbp
   91dff:	ff 25 f3 f1 04 00    	jmp    *0x4f1f3(%rip)        # e0ff8 <_DYNAMIC+0xa18>
   91e05:	4c 89 f8             	mov    %r15,%rax
   91e08:	48 09 d8             	or     %rbx,%rax
   91e0b:	41 0f 94 c5          	sete   %r13b
   91e0f:	eb 0f                	jmp    91e20 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x60>
   91e11:	4c 89 f8             	mov    %r15,%rax
   91e14:	48 09 d8             	or     %rbx,%rax
   91e17:	0f 84 55 01 00 00    	je     91f72 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x1b2>
   91e1d:	45 31 ed             	xor    %r13d,%r13d
   91e20:	48 89 e7             	mov    %rsp,%rdi
   91e23:	4c 89 f6             	mov    %r14,%rsi
   91e26:	e8 45 f9 ff ff       	call   91770 <<purrdf_xsd::exact::decimal::Decimal>::shape>
   91e2b:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
   91e30:	31 c0                	xor    %eax,%eax
   91e32:	48 b9 00 00 00 00 00 	movabs $0x8000000000000000,%rcx
   91e39:	00 00 80 
   91e3c:	48 c7 c6 ff ff ff ff 	mov    $0xffffffffffffffff,%rsi
   91e43:	48 c7 c2 ff ff ff ff 	mov    $0xffffffffffffffff,%rdx
   91e4a:	4c 2b 64 24 18       	sub    0x18(%rsp),%r12
   91e4f:	48 19 c0             	sbb    %rax,%rax
   91e52:	4c 39 e1             	cmp    %r12,%rcx
   91e55:	48 19 c6             	sbb    %rax,%rsi
   91e58:	48 0f 4d c2          	cmovge %rdx,%rax
   91e5c:	4c 0f 4d e1          	cmovge %rcx,%r12
   91e60:	48 ba ff ff ff ff ff 	movabs $0x7fffffffffffffff,%rdx
   91e67:	ff ff 7f 
   91e6a:	49 39 d4             	cmp    %rdx,%r12
   91e6d:	48 83 d8 00          	sbb    $0x0,%rax
   91e71:	4c 0f 4d e2          	cmovge %rdx,%r12
   91e75:	80 7c 24 24 00       	cmpb   $0x0,0x24(%rsp)
   91e7a:	4c 89 e1             	mov    %r12,%rcx
   91e7d:	0f 85 dd 00 00 00    	jne    91f60 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x1a0>
   91e83:	40 80 fd 02          	cmp    $0x2,%bpl
   91e87:	75 43                	jne    91ecc <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x10c>
   91e89:	b9 01 00 00 00       	mov    $0x1,%ecx
   91e8e:	45 84 ed             	test   %r13b,%r13b
   91e91:	0f 85 c2 00 00 00    	jne    91f59 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x199>
   91e97:	48 89 d8             	mov    %rbx,%rax
   91e9a:	48 c1 f8 3f          	sar    $0x3f,%rax
   91e9e:	48 89 d9             	mov    %rbx,%rcx
   91ea1:	ba 80 00 00 00       	mov    $0x80,%edx
   91ea6:	49 31 c7             	xor    %rax,%r15
   91ea9:	48 31 c1             	xor    %rax,%rcx
   91eac:	49 29 c7             	sub    %rax,%r15
   91eaf:	48 19 c1             	sbb    %rax,%rcx
   91eb2:	f3 49 0f bd f7       	lzcnt  %r15,%rsi
   91eb7:	f3 48 0f bd c1       	lzcnt  %rcx,%rax
   91ebc:	48 83 c6 40          	add    $0x40,%rsi
   91ec0:	48 85 c9             	test   %rcx,%rcx
   91ec3:	48 0f 45 f0          	cmovne %rax,%rsi
   91ec7:	48 29 f2             	sub    %rsi,%rdx
   91eca:	eb 30                	jmp    91efc <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x13c>
   91ecc:	48 8b 05 dd ed 04 00 	mov    0x4eddd(%rip),%rax        # e0cb0 <_DYNAMIC+0x6d0>
   91ed3:	4c 89 f7             	mov    %r14,%rdi
   91ed6:	ff d0                	call   *%rax
   91ed8:	b9 01 00 00 00       	mov    $0x1,%ecx
   91edd:	48 85 d2             	test   %rdx,%rdx
   91ee0:	74 77                	je     91f59 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x199>
   91ee2:	48 89 d6             	mov    %rdx,%rsi
   91ee5:	f3 48 0f bd 44 f0 f8 	lzcnt  -0x8(%rax,%rsi,8),%rax
   91eec:	48 ff ca             	dec    %rdx
   91eef:	48 c1 e2 06          	shl    $0x6,%rdx
   91ef3:	48 29 c2             	sub    %rax,%rdx
   91ef6:	48 83 c2 40          	add    $0x40,%rdx
   91efa:	74 5d                	je     91f59 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x199>
   91efc:	48 ff ca             	dec    %rdx
   91eff:	48 b8 8b fa 49 a6 2f 	movabs $0x42d792fa649fa8b,%rax
   91f06:	79 2d 04 
   91f09:	48 b9 e8 ea 90 74 74 	movabs $0x49c97747490eae8,%rcx
   91f10:	97 9c 04 
   91f13:	49 b8 ff 1c 12 73 91 	movabs $0x39d7f99173121cff,%r8
   91f1a:	f9 d7 39 
   91f1d:	c4 e2 eb f6 c0       	mulx   %rax,%rdx,%rax
   91f22:	c4 e2 cb f6 f9       	mulx   %rcx,%rsi,%rdi
   91f27:	c4 42 b3 f6 c8       	mulx   %r8,%r9,%r9
   91f2c:	48 89 c2             	mov    %rax,%rdx
   91f2f:	c4 e2 ab f6 c9       	mulx   %rcx,%r10,%rcx
   91f34:	c4 c2 fb f6 d0       	mulx   %r8,%rax,%rdx
   91f39:	49 01 f1             	add    %rsi,%r9
   91f3c:	48 83 d7 00          	adc    $0x0,%rdi
   91f40:	4c 01 c8             	add    %r9,%rax
   91f43:	48 11 fa             	adc    %rdi,%rdx
   91f46:	48 83 d1 00          	adc    $0x0,%rcx
   91f4a:	4c 01 d2             	add    %r10,%rdx
   91f4d:	48 83 d1 00          	adc    $0x0,%rcx
   91f51:	48 0f a4 d1 0a       	shld   $0xa,%rdx,%rcx
   91f56:	48 ff c1             	inc    %rcx
   91f59:	41 8b 46 40          	mov    0x40(%r14),%eax
   91f5d:	48 29 c1             	sub    %rax,%rcx
   91f60:	48 81 f9 35 01 00 00 	cmp    $0x135,%rcx
   91f67:	7f 1f                	jg     91f88 <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x1c8>
   91f69:	49 81 fc bc fe ff ff 	cmp    $0xfffffffffffffebc,%r12
   91f70:	7e 3d                	jle    91faf <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x1ef>
   91f72:	48 8d 35 27 9a f7 ff 	lea    -0x865d9(%rip),%rsi        # b9a0 <anon.8eed28602b91edc8c46173e0d2e060c2.6.llvm.13836761361530713107>
   91f79:	4c 89 f7             	mov    %r14,%rdi
   91f7c:	e8 4f e8 ff ff       	call   907d0 <<purrdf_xsd::exact::decimal::Decimal>::ratio_bits>
   91f81:	c4 e1 f9 6e c0       	vmovq  %rax,%xmm0
   91f86:	eb 44                	jmp    91fcc <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x20c>
   91f88:	c5 fb 10 05 e8 9d f7 	vmovsd -0x86218(%rip),%xmm0        # bd78 <anon.8eed28602b91edc8c46173e0d2e060c2.8.llvm.13836761361530713107+0x48>
   91f8f:	ff 
   91f90:	48 c1 eb 3f          	shr    $0x3f,%rbx
   91f94:	40 80 fd 02          	cmp    $0x2,%bpl
   91f98:	40 0f b6 c5          	movzbl %bpl,%eax
   91f9c:	0f 45 d8             	cmovne %eax,%ebx
   91f9f:	c5 fb 92 cb          	kmovd  %ebx,%k1
   91fa3:	62 f1 ff 09 10 05 33 	vmovsd -0x861cd(%rip),%xmm0{%k1}        # bde0 <anon.ba079ea26cac504a1586018250e73142.751.llvm.9424401259597866006+0x10>
   91faa:	9e f7 ff 
   91fad:	eb 1d                	jmp    91fcc <<purrdf_xsd::exact::decimal::Decimal>::to_f64+0x20c>
   91faf:	48 c1 eb 3f          	shr    $0x3f,%rbx
   91fb3:	40 80 fd 02          	cmp    $0x2,%bpl
   91fb7:	40 0f b6 c5          	movzbl %bpl,%eax
   91fbb:	0f 45 d8             	cmovne %eax,%ebx
   91fbe:	c5 fb 92 cb          	kmovd  %ebx,%k1
   91fc2:	62 f1 ff 89 10 05 34 	vmovsd -0x861cc(%rip),%xmm0{%k1}{z}        # be00 <anon.ba079ea26cac504a1586018250e73142.751.llvm.9424401259597866006+0x30>
   91fc9:	9e f7 ff 
   91fcc:	48 83 c4 28          	add    $0x28,%rsp
   91fd0:	5b                   	pop    %rbx
   91fd1:	41 5c                	pop    %r12
   91fd3:	41 5d                	pop    %r13
   91fd5:	41 5e                	pop    %r14
   91fd7:	41 5f                	pop    %r15
   91fd9:	5d                   	pop    %rbp
   91fda:	c3                   	ret

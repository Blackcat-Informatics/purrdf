; ModuleID = 'panic_unwind-ae09b68a0f0fcfb4.panic_unwind.32a08e97824dbd90-cgu.0.rcgu.o'
source_filename = "panic_unwind.32a08e97824dbd90-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@_RNvNtCs4lud3M4JRf0_12panic_unwind3imp6CANARY.llvm.13212232956059130102 = hidden constant [1 x i8] zeroinitializer, align 1, !guid !0

; core::ptr::drop_glue::<panic_unwind::imp::Exception>
; Function Attrs: nonlazybind uwtable
define hidden fastcc void @_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionEBF_.llvm.13212232956059130102(ptr %0, ptr nofree readonly captures(none) %1) unnamed_addr #0 personality ptr @rust_eh_personality !dbg !17 !guid !23 {
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1) ]
  %3 = load ptr, ptr %1, align 8, !dbg !24, !invariant.load !22
  %4 = icmp eq ptr %3, null, !dbg !24
  br i1 %4, label %6, label %5, !dbg !24

5:                                                ; preds = %2
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %0) ]
  invoke void %3(ptr noundef nonnull %0)
          to label %6 unwind label %13, !dbg !24

6:                                                ; preds = %5, %2
  %7 = getelementptr inbounds nuw i8, ptr %1, i64 8, !dbg !27
  %8 = load i64, ptr %7, align 8, !dbg !27, !range !46, !invariant.load !22
  %9 = icmp eq i64 %8, 0, !dbg !47
  br i1 %9, label %22, label %10, !dbg !47

10:                                               ; preds = %6
  %11 = getelementptr inbounds nuw i8, ptr %1, i64 16, !dbg !27
  %12 = load i64, ptr %11, align 8, !dbg !49, !range !57, !invariant.load !22
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %0) ]
; call __rustc::__rust_dealloc
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr noundef nonnull %0, i64 noundef range(i64 1, -9223372036854775808) %8, i64 noundef range(i64 1, 536870913) %12) #11, !dbg !58
  br label %22, !dbg !71

13:                                               ; preds = %5
  %14 = landingpad { ptr, i32 }
          cleanup
  %15 = getelementptr inbounds nuw i8, ptr %1, i64 8, !dbg !72
  %16 = load i64, ptr %15, align 8, !dbg !72, !range !46, !invariant.load !22
  %17 = icmp eq i64 %16, 0, !dbg !76
  br i1 %17, label %21, label %18, !dbg !76

18:                                               ; preds = %13
  %19 = getelementptr inbounds nuw i8, ptr %1, i64 16, !dbg !72
  %20 = load i64, ptr %19, align 8, !dbg !77, !range !57, !invariant.load !22
; call __rustc::__rust_dealloc
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr noundef nonnull %0, i64 noundef range(i64 1, -9223372036854775808) %16, i64 noundef range(i64 1, 536870913) %20) #11, !dbg !80
  br label %21, !dbg !85

21:                                               ; preds = %18, %13
  resume { ptr, i32 } %14, !dbg !24

22:                                               ; preds = %10, %6
  ret void, !dbg !86
}

; __rustc::__rust_start_panic
; Function Attrs: nonlazybind uwtable
define noundef range(i32 0, 10) i32 @_RNvCs2NWS7XDLE6y_7___rustc18___rust_start_panic(ptr noundef nonnull %0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(56) %1) unnamed_addr #0 personality ptr @rust_eh_personality !dbg !87 !guid !90 {
  %3 = getelementptr inbounds nuw i8, ptr %1, i64 32, !dbg !91
  %4 = load ptr, ptr %3, align 8, !dbg !91
  %5 = tail call { ptr, ptr } %4(ptr noundef nonnull %0) #12, !dbg !92, !inline_history !97
  %6 = extractvalue { ptr, ptr } %5, 0, !dbg !92
  %7 = extractvalue { ptr, ptr } %5, 1, !dbg !92
; call __rustc::__rust_no_alloc_shim_is_unstable_v2
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc35___rust_no_alloc_shim_is_unstable_v2() #11, !dbg !98, !noalias !113
; call __rustc::__rust_alloc
  %8 = tail call noundef align 16 dereferenceable_or_null(64) ptr @_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc(i64 noundef 64, i64 noundef 16) #11, !dbg !116, !noalias !113
  %9 = icmp eq ptr %8, null, !dbg !117
  br i1 %9, label %10, label %17, !dbg !118, !prof !119

10:                                               ; preds = %2
; invoke alloc::alloc::handle_alloc_error
  invoke void @_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error(i64 noundef 16, i64 noundef 64) #13
          to label %11 unwind label %12, !dbg !120

11:                                               ; preds = %10
  unreachable, !dbg !120

12:                                               ; preds = %10
  %13 = landingpad { ptr, i32 }
          cleanup
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %7) ]
; invoke core::ptr::drop_glue::<panic_unwind::imp::Exception>
  invoke fastcc void @_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionEBF_.llvm.13212232956059130102(ptr %6, ptr nonnull %7) #14
          to label %16 unwind label %14, !dbg !121

14:                                               ; preds = %12
  %15 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup() #15, !dbg !122
  unreachable, !dbg !122

16:                                               ; preds = %12
  resume { ptr, i32 } %13, !dbg !122

17:                                               ; preds = %2
  store i64 6076294132934528845, ptr %8, align 16, !dbg !123
  %18 = getelementptr inbounds nuw i8, ptr %8, i64 8, !dbg !123
  store ptr @_RNvNvNtCs4lud3M4JRf0_12panic_unwind3imp5panic17exception_cleanup.llvm.13212232956059130102, ptr %18, align 8, !dbg !123
  %19 = getelementptr inbounds nuw i8, ptr %8, i64 16, !dbg !123
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 16 dereferenceable(16) %19, i8 0, i64 16, i1 false), !dbg !123
  %20 = getelementptr inbounds nuw i8, ptr %8, i64 32, !dbg !123
  store ptr @_RNvNtCs4lud3M4JRf0_12panic_unwind3imp6CANARY.llvm.13212232956059130102, ptr %20, align 16, !dbg !123
  %21 = getelementptr inbounds nuw i8, ptr %8, i64 40, !dbg !123
  store ptr %6, ptr %21, align 8, !dbg !123
  %22 = getelementptr inbounds nuw i8, ptr %8, i64 48, !dbg !123
  store ptr %7, ptr %22, align 16, !dbg !123
  %23 = tail call noundef range(i32 0, 10) i32 @_Unwind_RaiseException(ptr noundef nonnull %8), !dbg !125
  ret i32 %23, !dbg !128
}

; __rustc::__rust_panic_cleanup
; Function Attrs: nonlazybind uwtable
define { ptr, ptr } @_RNvCs2NWS7XDLE6y_7___rustc20___rust_panic_cleanup(ptr noundef %0) unnamed_addr #0 !dbg !129 !guid !130 {
  %2 = load i64, ptr %0, align 16, !dbg !131, !noundef !22
  %3 = icmp eq i64 %2, 6076294132934528845, !dbg !131
  br i1 %3, label %4, label %8, !dbg !131

4:                                                ; preds = %1
  %5 = getelementptr inbounds nuw i8, ptr %0, i64 32, !dbg !135
  %6 = load ptr, ptr %5, align 16, !dbg !135, !noundef !22
  %7 = icmp eq ptr %6, @_RNvNtCs4lud3M4JRf0_12panic_unwind3imp6CANARY.llvm.13212232956059130102, !dbg !144
  br i1 %7, label %10, label %9, !dbg !148

8:                                                ; preds = %1
  tail call void @_Unwind_DeleteException(ptr noundef nonnull %0) #11, !dbg !149
; call __rustc::__rust_foreign_exception
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc24___rust_foreign_exception() #13, !dbg !150
  unreachable, !dbg !150

9:                                                ; preds = %4
; call __rustc::__rust_foreign_exception
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc24___rust_foreign_exception() #13, !dbg !151
  unreachable, !dbg !151

10:                                               ; preds = %4
  %11 = getelementptr inbounds nuw i8, ptr %0, i64 40, !dbg !152
  %12 = load ptr, ptr %11, align 8, !dbg !152, !nonnull !22, !noundef !22
  %13 = getelementptr inbounds nuw i8, ptr %0, i64 48, !dbg !152
  %14 = load ptr, ptr %13, align 16, !dbg !152, !nonnull !22, !align !154, !noundef !22
; call __rustc::__rust_dealloc
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr noundef nonnull %0, i64 noundef 64, i64 noundef 16) #11, !dbg !155
  %15 = insertvalue { ptr, ptr } poison, ptr %12, 0, !dbg !164
  %16 = insertvalue { ptr, ptr } %15, ptr %14, 1, !dbg !164
  ret { ptr, ptr } %16, !dbg !165
}

; panic_unwind::imp::panic::exception_cleanup
; Function Attrs: noreturn nounwind nonlazybind uwtable
define hidden void @_RNvNvNtCs4lud3M4JRf0_12panic_unwind3imp5panic17exception_cleanup.llvm.13212232956059130102(i32 range(i32 0, 10) %0, ptr noundef captures(address) %1) unnamed_addr #1 personality ptr @rust_eh_personality !dbg !166 !guid !168 {
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1) ]
  %3 = getelementptr i8, ptr %1, i64 40, !dbg !169
  %4 = load ptr, ptr %3, align 8, !dbg !169
  %5 = getelementptr i8, ptr %1, i64 48, !dbg !169
  %6 = load ptr, ptr %5, align 8, !dbg !169, !nonnull !22, !align !154, !noundef !22
  %7 = load ptr, ptr %6, align 8, !dbg !172, !invariant.load !22
  %8 = icmp eq ptr %7, null, !dbg !172
  br i1 %8, label %10, label %9, !dbg !172

9:                                                ; preds = %2
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %4) ]
  invoke void %7(ptr noundef nonnull %4)
          to label %10 unwind label %17, !dbg !172

10:                                               ; preds = %9, %2
  %11 = getelementptr inbounds nuw i8, ptr %6, i64 8, !dbg !175
  %12 = load i64, ptr %11, align 8, !dbg !175, !range !46, !invariant.load !22
  %13 = icmp eq i64 %12, 0, !dbg !179
  br i1 %13, label %26, label %14, !dbg !179

14:                                               ; preds = %10
  %15 = getelementptr inbounds nuw i8, ptr %6, i64 16, !dbg !175
  %16 = load i64, ptr %15, align 8, !dbg !180, !range !57, !invariant.load !22
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %4) ]
; call __rustc::__rust_dealloc
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr noundef nonnull %4, i64 noundef range(i64 1, -9223372036854775808) %12, i64 noundef range(i64 1, 536870913) %16) #11, !dbg !183
  br label %26, !dbg !188

17:                                               ; preds = %9
  %18 = landingpad { ptr, i32 }
          cleanup
  %19 = getelementptr inbounds nuw i8, ptr %6, i64 8, !dbg !189
  %20 = load i64, ptr %19, align 8, !dbg !189, !range !46, !invariant.load !22
  %21 = icmp eq i64 %20, 0, !dbg !193
  br i1 %21, label %25, label %22, !dbg !193

22:                                               ; preds = %17
  %23 = getelementptr inbounds nuw i8, ptr %6, i64 16, !dbg !189
  %24 = load i64, ptr %23, align 8, !dbg !194, !range !57, !invariant.load !22
; call __rustc::__rust_dealloc
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr noundef nonnull %4, i64 noundef range(i64 1, -9223372036854775808) %20, i64 noundef range(i64 1, 536870913) %24) #11, !dbg !197
  br label %25, !dbg !202

25:                                               ; preds = %22, %17
; call __rustc::__rust_dealloc
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr noundef nonnull %1, i64 noundef 64, i64 noundef 16) #11, !dbg !203
; call core::panicking::panic_cannot_unwind
  tail call void @_RNvNtCs2k2z8Zem4rB_4core9panicking19panic_cannot_unwind() #16, !dbg !209
  unreachable, !dbg !209

26:                                               ; preds = %14, %10
; call __rustc::__rust_dealloc
  tail call void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr noundef nonnull %1, i64 noundef 64, i64 noundef 16) #11, !dbg !210
; invoke __rustc::__rust_drop_panic
  invoke void @_RNvCs2NWS7XDLE6y_7___rustc17___rust_drop_panic() #13
          to label %29 unwind label %27, !dbg !216

27:                                               ; preds = %26
  %28 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_cannot_unwind
  tail call void @_RNvNtCs2k2z8Zem4rB_4core9panicking19panic_cannot_unwind() #15, !dbg !209
  unreachable, !dbg !209

29:                                               ; preds = %26
  unreachable
}

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #2

; core::panicking::panic_in_cleanup
; Function Attrs: cold minsize noinline noreturn nounwind nonlazybind optsize uwtable
declare void @_RNvNtCs2k2z8Zem4rB_4core9panicking16panic_in_cleanup() unnamed_addr #3

; Function Attrs: nonlazybind uwtable
declare noundef range(i32 0, 10) i32 @_Unwind_RaiseException(ptr noundef) unnamed_addr #0

; Function Attrs: nounwind nonlazybind uwtable
declare void @_Unwind_DeleteException(ptr noundef) unnamed_addr #4

; core::panicking::panic_cannot_unwind
; Function Attrs: cold minsize noinline noreturn nounwind nonlazybind optsize uwtable
declare void @_RNvNtCs2k2z8Zem4rB_4core9panicking19panic_cannot_unwind() unnamed_addr #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr writeonly captures(none), i8, i64, i1 immarg) #5

; __rustc::__rust_no_alloc_shim_is_unstable_v2
; Function Attrs: uwtable
declare void @_RNvCs2NWS7XDLE6y_7___rustc35___rust_no_alloc_shim_is_unstable_v2() unnamed_addr #6

; alloc::alloc::handle_alloc_error
; Function Attrs: cold minsize noreturn nonlazybind optsize uwtable
declare void @_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error(i64 noundef range(i64 1, -9223372036854775807), i64 noundef) unnamed_addr #7

; __rustc::__rust_alloc
; Function Attrs: nonlazybind allockind("alloc,uninitialized,aligned") allocsize(0) uwtable
declare hidden noalias noundef ptr @_RNvCs2NWS7XDLE6y_7___rustc12___rust_alloc(i64 noundef, i64 allocalign noundef range(i64 1, -9223372036854775807)) unnamed_addr #8

; __rustc::__rust_dealloc
; Function Attrs: nonlazybind allockind("free") uwtable
declare hidden void @_RNvCs2NWS7XDLE6y_7___rustc14___rust_dealloc(ptr allocptr noundef captures(address), i64 noundef, i64 noundef range(i64 1, -9223372036854775807)) unnamed_addr #9

; Function Attrs: nounwind nonlazybind uwtable
declare noundef range(i32 2, 9) i32 @rust_eh_personality(i32 noundef, i32 noundef, i64, ptr noundef, ptr noundef) unnamed_addr #4

; __rustc::__rust_drop_panic
; Function Attrs: cold noreturn nonlazybind uwtable
declare void @_RNvCs2NWS7XDLE6y_7___rustc17___rust_drop_panic() unnamed_addr #10

; __rustc::__rust_foreign_exception
; Function Attrs: cold noreturn nonlazybind uwtable
declare void @_RNvCs2NWS7XDLE6y_7___rustc24___rust_foreign_exception() unnamed_addr #10

attributes #0 = { nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #1 = { noreturn nounwind nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #2 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }
attributes #3 = { cold minsize noinline noreturn nounwind nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #4 = { nounwind nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #5 = { nocallback nofree nosync nounwind willreturn memory(argmem: write) }
attributes #6 = { uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #7 = { cold minsize noreturn nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #8 = { nonlazybind allockind("alloc,uninitialized,aligned") allocsize(0) uwtable "alloc-family"="__rust_alloc" "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #9 = { nonlazybind allockind("free") uwtable "alloc-family"="__rust_alloc" "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #10 = { cold noreturn nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #11 = { nounwind }
attributes #12 = { inlinehint }
attributes #13 = { noreturn }
attributes #14 = { cold }
attributes #15 = { cold noreturn nounwind }
attributes #16 = { cold noinline noreturn nounwind }

!llvm.module.flags = !{!1, !2, !3, !4, !5, !6, !7}
!llvm.ident = !{!8}
!llvm.dbg.cu = !{!9, !11, !13, !15}

!0 = !{i64 1322945213280544450}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 2, !"RtLibUseGOT", i32 1}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{i32 7, !"frame-pointer", i32 1}
!5 = !{i32 7, !"Dwarf Version", i32 4}
!6 = !{i32 2, !"Debug Info Version", i32 3}
!7 = !{i32 7, !"PIE Level", i32 2}
!8 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!9 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !10, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!10 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/panic_unwind/src/lib.rs/@/panic_unwind.32a08e97824dbd90-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!11 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !12, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!12 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/alloc/src/lib.rs/@/alloc.8d16d54ddffdc8a3-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!13 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !14, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!14 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/core/src/lib.rs/@/core.1b0f708be8c133d1-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!15 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !16, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!16 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/std/src/lib.rs/@/std.552422bcccb5c833-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!17 = distinct !DISubprogram(name: "drop_glue<panic_unwind::imp::Exception>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionEBF_", scope: !19, file: !18, line: 848, type: !21, scopeLine: 848, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!18 = !DIFile(filename: "library/core/src/ptr/mod.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "880a13f9557399be745f3ca5a0b3d5e4")
!19 = !DINamespace(name: "ptr", scope: !20)
!20 = !DINamespace(name: "core", scope: null)
!21 = !DISubroutineType(cc: DW_CC_nocall, types: !22)
!22 = !{}
!23 = !{i64 8185499177958873478}
!24 = !DILocation(line: 848, column: 1, scope: !25, inlinedAt: !26)
!25 = distinct !DISubprogram(name: "drop_glue<alloc::boxed::Box<(dyn core::any::Any + core::marker::Send), alloc::alloc::Global>>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_EECs4lud3M4JRf0_12panic_unwind", scope: !19, file: !18, line: 848, type: !21, scopeLine: 848, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!26 = distinct !DILocation(line: 848, column: 1, scope: !17)
!27 = !DILocation(line: 468, column: 14, scope: !28, inlinedAt: !32)
!28 = distinct !DISubprogram(name: "size_of_val_raw<(dyn core::any::Any + core::marker::Send)>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3mem15size_of_val_rawDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_ECs4lud3M4JRf0_12panic_unwind", scope: !30, file: !29, line: 466, type: !31, scopeLine: 466, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!29 = !DIFile(filename: "library/core/src/mem/mod.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "38eadfce92d729d699dc6755ee4dbde2")
!30 = !DINamespace(name: "mem", scope: !20)
!31 = !DISubroutineType(types: !22)
!32 = distinct !DILocation(line: 261, column: 43, scope: !33, inlinedAt: !38)
!33 = distinct !DISubprogram(name: "for_value_raw<(dyn core::any::Any + core::marker::Send)>", linkageName: "_RINvMNtNtCs2k2z8Zem4rB_4core5alloc6layoutNtB3_6Layout13for_value_rawDNtNtB7_3any3AnyNtNtB7_6marker4SendEL_ECs4lud3M4JRf0_12panic_unwind", scope: !35, file: !34, line: 259, type: !31, scopeLine: 259, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!34 = !DIFile(filename: "library/core/src/alloc/layout.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "044b929685d8564035fcb902806519e4")
!35 = !DINamespace(name: "Layout", scope: !36)
!36 = !DINamespace(name: "layout", scope: !37)
!37 = !DINamespace(name: "alloc", scope: !20)
!38 = distinct !DILocation(line: 2039, column: 31, scope: !39, inlinedAt: !45)
!39 = distinct !DILexicalBlock(scope: !41, file: !40, line: 2034, column: 9)
!40 = !DIFile(filename: "library/alloc/src/boxed.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "6108af3e98a62d47bb01ee5dc9f44ffe")
!41 = distinct !DISubprogram(name: "drop<(dyn core::any::Any + core::marker::Send), alloc::alloc::Global>", linkageName: "_RNvXs8_NtCsc70TAahYccp_5alloc5boxedINtB5_3BoxDNtNtCs2k2z8Zem4rB_4core3any3AnyNtNtBM_6marker4SendEL_ENtNtNtBM_3ops4drop4Drop4dropCs4lud3M4JRf0_12panic_unwind", scope: !42, file: !40, line: 2031, type: !21, scopeLine: 2031, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!42 = !DINamespace(name: "{impl#10}", scope: !43)
!43 = !DINamespace(name: "boxed", scope: !44)
!44 = !DINamespace(name: "alloc", scope: null)
!45 = distinct !DILocation(line: 848, column: 1, scope: !25, inlinedAt: !26)
!46 = !{i64 0, i64 -9223372036854775808}
!47 = !DILocation(line: 2040, column: 12, scope: !48, inlinedAt: !45)
!48 = distinct !DILexicalBlock(scope: !39, file: !40, line: 2039, column: 9)
!49 = !DILocation(line: 642, column: 14, scope: !50, inlinedAt: !51)
!50 = distinct !DISubprogram(name: "align_of_val_raw<(dyn core::any::Any + core::marker::Send)>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3mem16align_of_val_rawDNtNtB4_3any3AnyNtNtB4_6marker4SendEL_ECs4lud3M4JRf0_12panic_unwind", scope: !30, file: !29, line: 640, type: !31, scopeLine: 640, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!51 = distinct !DILocation(line: 159, column: 30, scope: !52, inlinedAt: !56)
!52 = distinct !DISubprogram(name: "of_val_raw<(dyn core::any::Any + core::marker::Send)>", linkageName: "_RINvMNtNtCs2k2z8Zem4rB_4core3mem9alignmentNtB3_9Alignment10of_val_rawDNtNtB7_3any3AnyNtNtB7_6marker4SendEL_ECs4lud3M4JRf0_12panic_unwind", scope: !54, file: !53, line: 157, type: !31, scopeLine: 157, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!53 = !DIFile(filename: "library/core/src/mem/alignment.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "a4e1b46f32511e964757f2024f38c90f")
!54 = !DINamespace(name: "Alignment", scope: !55)
!55 = !DINamespace(name: "alignment", scope: !30)
!56 = distinct !DILocation(line: 261, column: 70, scope: !33, inlinedAt: !38)
!57 = !{i64 1, i64 536870913}
!58 = !DILocation(line: 178, column: 14, scope: !59, inlinedAt: !62)
!59 = distinct !DISubprogram(name: "dealloc_nonnull", linkageName: "_RNvNtCsc70TAahYccp_5alloc5alloc15dealloc_nonnull", scope: !61, file: !60, line: 176, type: !31, scopeLine: 176, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!60 = !DIFile(filename: "library/alloc/src/alloc.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "678754501574f3b67215a21690a1978c")
!61 = !DINamespace(name: "alloc", scope: !44)
!62 = distinct !DILocation(line: 327, column: 22, scope: !63, inlinedAt: !65)
!63 = distinct !DISubprogram(name: "deallocate_impl_runtime", linkageName: "_RNvMs0_NtCsc70TAahYccp_5alloc5allocNtB5_6Global23deallocate_impl_runtime", scope: !64, file: !60, line: 317, type: !31, scopeLine: 317, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!64 = !DINamespace(name: "Global", scope: !61)
!65 = distinct !DILocation(line: 442, column: 9, scope: !66, inlinedAt: !67)
!66 = distinct !DISubprogram(name: "deallocate_impl", linkageName: "_RNvMs0_NtCsc70TAahYccp_5alloc5allocNtB5_6Global15deallocate_impl", scope: !64, file: !60, line: 441, type: !31, scopeLine: 441, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!67 = distinct !DILocation(line: 561, column: 23, scope: !68, inlinedAt: !70)
!68 = distinct !DISubprogram(name: "deallocate", linkageName: "_RNvXs1_NtCsc70TAahYccp_5alloc5allocNtB5_6GlobalNtNtCs2k2z8Zem4rB_4core5alloc9Allocator10deallocate", scope: !69, file: !60, line: 559, type: !21, scopeLine: 559, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!69 = !DINamespace(name: "{impl#3}", scope: !61)
!70 = distinct !DILocation(line: 2045, column: 24, scope: !48, inlinedAt: !45)
!71 = !DILocation(line: 2040, column: 9, scope: !48, inlinedAt: !45)
!72 = !DILocation(line: 468, column: 14, scope: !28, inlinedAt: !73)
!73 = distinct !DILocation(line: 261, column: 43, scope: !33, inlinedAt: !74)
!74 = distinct !DILocation(line: 2039, column: 31, scope: !39, inlinedAt: !75)
!75 = distinct !DILocation(line: 848, column: 1, scope: !25, inlinedAt: !26)
!76 = !DILocation(line: 2040, column: 12, scope: !48, inlinedAt: !75)
!77 = !DILocation(line: 642, column: 14, scope: !50, inlinedAt: !78)
!78 = distinct !DILocation(line: 159, column: 30, scope: !52, inlinedAt: !79)
!79 = distinct !DILocation(line: 261, column: 70, scope: !33, inlinedAt: !74)
!80 = !DILocation(line: 178, column: 14, scope: !59, inlinedAt: !81)
!81 = distinct !DILocation(line: 327, column: 22, scope: !63, inlinedAt: !82)
!82 = distinct !DILocation(line: 442, column: 9, scope: !66, inlinedAt: !83)
!83 = distinct !DILocation(line: 561, column: 23, scope: !68, inlinedAt: !84)
!84 = distinct !DILocation(line: 2045, column: 24, scope: !48, inlinedAt: !75)
!85 = !DILocation(line: 2040, column: 9, scope: !48, inlinedAt: !75)
!86 = !DILocation(line: 848, column: 1, scope: !17)
!87 = distinct !DISubprogram(name: "__rust_start_panic", linkageName: "_RNvCs2NWS7XDLE6y_7___rustc18___rust_start_panic", scope: !89, file: !88, line: 90, type: !31, scopeLine: 90, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!88 = !DIFile(filename: "library/panic_unwind/src/lib.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "9ef5995b86b83083fc5b0dd4c3cb06f7")
!89 = !DINamespace(name: "panic_unwind", scope: null)
!90 = !{i64 8144272593875718699}
!91 = !DILocation(line: 91, column: 5, scope: !87)
!92 = !DILocation(line: 70, column: 21, scope: !93, inlinedAt: !96)
!93 = distinct !DISubprogram(name: "panic", linkageName: "_RNvNtCs4lud3M4JRf0_12panic_unwind3imp5panic", scope: !95, file: !94, line: 62, type: !21, scopeLine: 62, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!94 = !DIFile(filename: "library/panic_unwind/src/gcc.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "5c77c49263cb8dea442df0ed753a02ff")
!95 = !DINamespace(name: "imp", scope: !89)
!96 = distinct !DILocation(line: 91, column: 5, scope: !87)
!97 = distinct !{null}
!98 = !DILocation(line: 129, column: 9, scope: !99, inlinedAt: !100)
!99 = distinct !DISubprogram(name: "alloc", linkageName: "_RNvNtCsc70TAahYccp_5alloc5alloc5alloc", scope: !61, file: !60, line: 124, type: !31, scopeLine: 124, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!100 = distinct !DILocation(line: 308, column: 73, scope: !101, inlinedAt: !103)
!101 = distinct !DILexicalBlock(scope: !102, file: !60, line: 307, column: 13)
!102 = distinct !DISubprogram(name: "alloc_impl_runtime", linkageName: "_RNvMs0_NtCsc70TAahYccp_5alloc5allocNtB5_6Global18alloc_impl_runtime", scope: !64, file: !60, line: 303, type: !31, scopeLine: 303, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!103 = distinct !DILocation(line: 430, column: 9, scope: !104, inlinedAt: !105)
!104 = distinct !DISubprogram(name: "alloc_impl", linkageName: "_RNvMs0_NtCsc70TAahYccp_5alloc5allocNtB5_6Global10alloc_impl", scope: !64, file: !60, line: 429, type: !31, scopeLine: 429, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!105 = distinct !DILocation(line: 548, column: 14, scope: !106, inlinedAt: !107)
!106 = distinct !DISubprogram(name: "allocate", linkageName: "_RNvXs1_NtCsc70TAahYccp_5alloc5allocNtB5_6GlobalNtNtCs2k2z8Zem4rB_4core5alloc9Allocator8allocate", scope: !69, file: !60, line: 547, type: !31, scopeLine: 547, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!107 = distinct !DILocation(line: 251, column: 18, scope: !108, inlinedAt: !109)
!108 = distinct !DISubprogram(name: "box_new_uninit", linkageName: "_RNvNtCsc70TAahYccp_5alloc5boxed14box_new_uninit", scope: !43, file: !40, line: 250, type: !31, scopeLine: 250, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!109 = distinct !DILocation(line: 292, column: 19, scope: !110, inlinedAt: !112)
!110 = distinct !DISubprogram(name: "new<panic_unwind::imp::Exception>", linkageName: "_RNvMNtCsc70TAahYccp_5alloc5boxedINtB2_3BoxNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionE3newBI_", scope: !111, file: !40, line: 290, type: !31, scopeLine: 290, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!111 = !DINamespace(name: "{impl#0}", scope: !43)
!112 = distinct !DILocation(line: 63, column: 21, scope: !93, inlinedAt: !96)
!113 = !{!114}
!114 = distinct !{!114, !115, !"_RNvMNtCsc70TAahYccp_5alloc5boxedINtB2_3BoxNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionE3newBI_: argument 0"}
!115 = distinct !{!115, !"_RNvMNtCsc70TAahYccp_5alloc5boxedINtB2_3BoxNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionE3newBI_"}
!116 = !DILocation(line: 131, column: 9, scope: !99, inlinedAt: !100)
!117 = !DILocation(line: 251, column: 11, scope: !108, inlinedAt: !109)
!118 = !DILocation(line: 251, column: 5, scope: !108, inlinedAt: !109)
!119 = !{!"branch_weights", !"expected", i32 1, i32 2000}
!120 = !DILocation(line: 253, column: 19, scope: !108, inlinedAt: !109)
!121 = !DILocation(line: 298, column: 5, scope: !110, inlinedAt: !112)
!122 = !DILocation(line: 290, column: 5, scope: !110, inlinedAt: !112)
!123 = !DILocation(line: 295, column: 56, scope: !124, inlinedAt: !112)
!124 = distinct !DILexicalBlock(scope: !110, file: !40, line: 292, column: 9)
!125 = !DILocation(line: 73, column: 21, scope: !126, inlinedAt: !96)
!126 = distinct !DILexicalBlock(scope: !127, file: !94, line: 72, column: 5)
!127 = distinct !DILexicalBlock(scope: !93, file: !94, line: 63, column: 5)
!128 = !DILocation(line: 92, column: 2, scope: !87)
!129 = distinct !DISubprogram(name: "__rust_panic_cleanup", linkageName: "_RNvCs2NWS7XDLE6y_7___rustc20___rust_panic_cleanup", scope: !89, file: !88, line: 83, type: !31, scopeLine: 83, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!130 = !{i64 6571528735637060577}
!131 = !DILocation(line: 87, column: 12, scope: !132, inlinedAt: !134)
!132 = distinct !DILexicalBlock(scope: !133, file: !94, line: 86, column: 9)
!133 = distinct !DISubprogram(name: "cleanup", linkageName: "_RNvNtCs4lud3M4JRf0_12panic_unwind3imp7cleanup", scope: !95, file: !94, line: 84, type: !31, scopeLine: 84, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!134 = distinct !DILocation(line: 84, column: 14, scope: !129)
!135 = !DILocation(line: 1758, column: 9, scope: !136, inlinedAt: !137)
!136 = distinct !DISubprogram(name: "read<*const u8>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3ptr4readPhECs4lud3M4JRf0_12panic_unwind", scope: !19, file: !18, line: 1719, type: !31, scopeLine: 1719, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!137 = distinct !DILocation(line: 1154, column: 18, scope: !138, inlinedAt: !142)
!138 = distinct !DISubprogram(name: "read<*const u8>", linkageName: "_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPPh4readCs4lud3M4JRf0_12panic_unwind", scope: !140, file: !139, line: 1149, type: !31, scopeLine: 1149, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!139 = !DIFile(filename: "library/core/src/ptr/const_ptr.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "3476ea789cc24546d3b3809125701ad2")
!140 = !DINamespace(name: "{impl#0}", scope: !141)
!141 = !DINamespace(name: "const_ptr", scope: !19)
!142 = distinct !DILocation(line: 95, column: 55, scope: !143, inlinedAt: !134)
!143 = distinct !DILexicalBlock(scope: !132, file: !94, line: 92, column: 9)
!144 = !DILocation(line: 2511, column: 5, scope: !145, inlinedAt: !146)
!145 = distinct !DISubprogram(name: "eq<u8>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3ptr2eqhECs4lud3M4JRf0_12panic_unwind", scope: !19, file: !18, line: 2510, type: !31, scopeLine: 2510, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!146 = distinct !DILocation(line: 96, column: 13, scope: !147, inlinedAt: !134)
!147 = distinct !DILexicalBlock(scope: !143, file: !94, line: 95, column: 9)
!148 = !DILocation(line: 96, column: 13, scope: !147, inlinedAt: !134)
!149 = !DILocation(line: 88, column: 13, scope: !132, inlinedAt: !134)
!150 = !DILocation(line: 89, column: 13, scope: !132, inlinedAt: !134)
!151 = !DILocation(line: 101, column: 13, scope: !147, inlinedAt: !134)
!152 = !DILocation(line: 105, column: 9, scope: !153, inlinedAt: !134)
!153 = distinct !DILexicalBlock(scope: !147, file: !94, line: 104, column: 9)
!154 = !{i64 8}
!155 = !DILocation(line: 178, column: 14, scope: !59, inlinedAt: !156)
!156 = distinct !DILocation(line: 327, column: 22, scope: !63, inlinedAt: !157)
!157 = distinct !DILocation(line: 442, column: 9, scope: !66, inlinedAt: !158)
!158 = distinct !DILocation(line: 561, column: 23, scope: !68, inlinedAt: !159)
!159 = distinct !DILocation(line: 2045, column: 24, scope: !160, inlinedAt: !163)
!160 = distinct !DILexicalBlock(scope: !161, file: !40, line: 2039, column: 9)
!161 = distinct !DILexicalBlock(scope: !162, file: !40, line: 2034, column: 9)
!162 = distinct !DISubprogram(name: "drop<panic_unwind::imp::Exception, alloc::alloc::Global>", linkageName: "_RNvXs8_NtCsc70TAahYccp_5alloc5boxedINtB5_3BoxNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionENtNtNtCs2k2z8Zem4rB_4core3ops4drop4Drop4dropBL_", scope: !42, file: !40, line: 2031, type: !21, scopeLine: 2031, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!163 = distinct !DILocation(line: 106, column: 5, scope: !147, inlinedAt: !134)
!164 = !DILocation(line: 107, column: 2, scope: !133, inlinedAt: !134)
!165 = !DILocation(line: 85, column: 2, scope: !129)
!166 = distinct !DISubprogram(name: "exception_cleanup", linkageName: "_RNvNvNtCs4lud3M4JRf0_12panic_unwind3imp5panic17exception_cleanup", scope: !167, file: !94, line: 75, type: !31, scopeLine: 75, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!167 = !DINamespace(name: "panic", scope: !95)
!168 = !{i64 -8292739656946577334}
!169 = !DILocation(line: 848, column: 1, scope: !170, inlinedAt: !171)
!170 = distinct !DISubprogram(name: "drop_glue<alloc::boxed::Box<panic_unwind::imp::Exception, alloc::alloc::Global>>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3ptr9drop_glueINtNtCsc70TAahYccp_5alloc5boxed3BoxNtNtCs4lud3M4JRf0_12panic_unwind3imp9ExceptionEEB1e_", scope: !19, file: !18, line: 848, type: !21, scopeLine: 848, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !9, templateParams: !22)
!171 = distinct !DILocation(line: 79, column: 86, scope: !166)
!172 = !DILocation(line: 848, column: 1, scope: !25, inlinedAt: !173)
!173 = distinct !DILocation(line: 848, column: 1, scope: !17, inlinedAt: !174)
!174 = distinct !DILocation(line: 848, column: 1, scope: !170, inlinedAt: !171)
!175 = !DILocation(line: 468, column: 14, scope: !28, inlinedAt: !176)
!176 = distinct !DILocation(line: 261, column: 43, scope: !33, inlinedAt: !177)
!177 = distinct !DILocation(line: 2039, column: 31, scope: !39, inlinedAt: !178)
!178 = distinct !DILocation(line: 848, column: 1, scope: !25, inlinedAt: !173)
!179 = !DILocation(line: 2040, column: 12, scope: !48, inlinedAt: !178)
!180 = !DILocation(line: 642, column: 14, scope: !50, inlinedAt: !181)
!181 = distinct !DILocation(line: 159, column: 30, scope: !52, inlinedAt: !182)
!182 = distinct !DILocation(line: 261, column: 70, scope: !33, inlinedAt: !177)
!183 = !DILocation(line: 178, column: 14, scope: !59, inlinedAt: !184)
!184 = distinct !DILocation(line: 327, column: 22, scope: !63, inlinedAt: !185)
!185 = distinct !DILocation(line: 442, column: 9, scope: !66, inlinedAt: !186)
!186 = distinct !DILocation(line: 561, column: 23, scope: !68, inlinedAt: !187)
!187 = distinct !DILocation(line: 2045, column: 24, scope: !48, inlinedAt: !178)
!188 = !DILocation(line: 2040, column: 9, scope: !48, inlinedAt: !178)
!189 = !DILocation(line: 468, column: 14, scope: !28, inlinedAt: !190)
!190 = distinct !DILocation(line: 261, column: 43, scope: !33, inlinedAt: !191)
!191 = distinct !DILocation(line: 2039, column: 31, scope: !39, inlinedAt: !192)
!192 = distinct !DILocation(line: 848, column: 1, scope: !25, inlinedAt: !173)
!193 = !DILocation(line: 2040, column: 12, scope: !48, inlinedAt: !192)
!194 = !DILocation(line: 642, column: 14, scope: !50, inlinedAt: !195)
!195 = distinct !DILocation(line: 159, column: 30, scope: !52, inlinedAt: !196)
!196 = distinct !DILocation(line: 261, column: 70, scope: !33, inlinedAt: !191)
!197 = !DILocation(line: 178, column: 14, scope: !59, inlinedAt: !198)
!198 = distinct !DILocation(line: 327, column: 22, scope: !63, inlinedAt: !199)
!199 = distinct !DILocation(line: 442, column: 9, scope: !66, inlinedAt: !200)
!200 = distinct !DILocation(line: 561, column: 23, scope: !68, inlinedAt: !201)
!201 = distinct !DILocation(line: 2045, column: 24, scope: !48, inlinedAt: !192)
!202 = !DILocation(line: 2040, column: 9, scope: !48, inlinedAt: !192)
!203 = !DILocation(line: 178, column: 14, scope: !59, inlinedAt: !204)
!204 = distinct !DILocation(line: 327, column: 22, scope: !63, inlinedAt: !205)
!205 = distinct !DILocation(line: 442, column: 9, scope: !66, inlinedAt: !206)
!206 = distinct !DILocation(line: 561, column: 23, scope: !68, inlinedAt: !207)
!207 = distinct !DILocation(line: 2045, column: 24, scope: !160, inlinedAt: !208)
!208 = distinct !DILocation(line: 848, column: 1, scope: !170, inlinedAt: !171)
!209 = !DILocation(line: 75, column: 5, scope: !166)
!210 = !DILocation(line: 178, column: 14, scope: !59, inlinedAt: !211)
!211 = distinct !DILocation(line: 327, column: 22, scope: !63, inlinedAt: !212)
!212 = distinct !DILocation(line: 442, column: 9, scope: !66, inlinedAt: !213)
!213 = distinct !DILocation(line: 561, column: 23, scope: !68, inlinedAt: !214)
!214 = distinct !DILocation(line: 2045, column: 24, scope: !160, inlinedAt: !215)
!215 = distinct !DILocation(line: 848, column: 1, scope: !170, inlinedAt: !171)
!216 = !DILocation(line: 80, column: 9, scope: !217)
!217 = distinct !DILexicalBlock(scope: !166, file: !94, line: 79, column: 9)

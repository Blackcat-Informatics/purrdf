; ModuleID = 'hashbrown-6e536cf7fc5f3281.hashbrown.32bd1dc6e5d86b0c-cgu.0.rcgu.o'
source_filename = "hashbrown.32bd1dc6e5d86b0c-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433 = hidden unnamed_addr constant [28 x i8] c"Hash table capacity overflow", align 1, !guid !0
@anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433 = hidden unnamed_addr constant [88 x i8] c"/kache/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.17.1/src/raw.rs\00", align 1, !guid !1
@anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433 = hidden unnamed_addr constant <{ ptr, [16 x i8] }> <{ ptr @anon.f083946614dbed2886867a2b7043ff2f.1.llvm.11433797203420380433, [16 x i8] c"W\00\00\00\00\00\00\00$\00\00\00(\00\00\00" }>, align 8, !guid !2

; <hashbrown::raw::Fallibility>::capacity_overflow
; Function Attrs: nonlazybind uwtable
define { i64, i64 } @_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility17capacity_overflow(i1 noundef zeroext %0) unnamed_addr #0 !guid !14 {
  br i1 %0, label %2, label %3, !prof !15

2:                                                ; preds = %1
; call core::panicking::panic_fmt
  tail call void @_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt(ptr noundef nonnull @anon.f083946614dbed2886867a2b7043ff2f.0.llvm.11433797203420380433, ptr noundef nonnull inttoptr (i64 57 to ptr), ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.f083946614dbed2886867a2b7043ff2f.2.llvm.11433797203420380433) #3
  unreachable

3:                                                ; preds = %1
  ret { i64, i64 } { i64 0, i64 undef }
}

; <hashbrown::raw::Fallibility>::alloc_err
; Function Attrs: nonlazybind uwtable
define { i64, i64 } @_RNvMNtCs4m51Kb4I71M_9hashbrown3rawNtB2_11Fallibility9alloc_err(i1 noundef zeroext %0, i64 noundef range(i64 1, -9223372036854775807) %1, i64 noundef %2) unnamed_addr #0 !guid !16 {
  br i1 %0, label %4, label %5, !prof !15

4:                                                ; preds = %3
; call alloc::alloc::handle_alloc_error
  tail call void @_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error(i64 noundef %1, i64 noundef %2) #4
  unreachable

5:                                                ; preds = %3
  %6 = insertvalue { i64, i64 } poison, i64 %1, 0
  %7 = insertvalue { i64, i64 } %6, i64 %2, 1
  ret { i64, i64 } %7
}

; core::panicking::panic_fmt
; Function Attrs: cold noinline noreturn nonlazybind uwtable
declare void @_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt(ptr noundef nonnull, ptr noundef nonnull, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24)) unnamed_addr #1

; alloc::alloc::handle_alloc_error
; Function Attrs: cold minsize noreturn nonlazybind optsize uwtable
declare void @_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error(i64 noundef range(i64 1, -9223372036854775807), i64 noundef) unnamed_addr #2

attributes #0 = { nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #1 = { cold noinline noreturn nonlazybind uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #2 = { cold minsize noreturn nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #3 = { noinline noreturn }
attributes #4 = { noreturn }

!llvm.module.flags = !{!3, !4, !5, !6, !7, !8}
!llvm.ident = !{!9}
!llvm.dbg.cu = !{!10, !12}

!0 = !{i64 -3655717827879521846}
!1 = !{i64 7227100168686298963}
!2 = !{i64 -485960117238966684}
!3 = !{i32 8, !"PIC Level", i32 2}
!4 = !{i32 2, !"RtLibUseGOT", i32 1}
!5 = !{i32 7, !"uwtable", i32 2}
!6 = !{i32 7, !"frame-pointer", i32 1}
!7 = !{i32 7, !"Dwarf Version", i32 4}
!8 = !{i32 2, !"Debug Info Version", i32 3}
!9 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!10 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !11, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!11 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/alloc/src/lib.rs/@/alloc.8d16d54ddffdc8a3-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!12 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !13, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!13 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/std/src/lib.rs/@/std.552422bcccb5c833-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!14 = !{i64 -6819202737583916981}
!15 = !{!"branch_weights", !"expected", i32 1, i32 2000}
!16 = !{i64 4405557268934996245}

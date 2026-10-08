; ModuleID = 'hashbrown-a15a51a57d67e08b.hashbrown.7e172c837a6a8ce9-cgu.0.rcgu.o'
source_filename = "hashbrown.7e172c837a6a8ce9-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@anon.4101bb5dfac2026d99153c8951e5b276.0.llvm.2690330406010747703 = hidden unnamed_addr constant [28 x i8] c"Hash table capacity overflow", align 1, !guid !0
@anon.4101bb5dfac2026d99153c8951e5b276.1.llvm.2690330406010747703 = hidden unnamed_addr constant [61 x i8] c"/cargo/registry/25cdd57fae9f0462/hashbrown-0.17.1/src/raw.rs\00", align 1, !guid !1
@anon.4101bb5dfac2026d99153c8951e5b276.2.llvm.2690330406010747703 = hidden unnamed_addr constant <{ ptr, [16 x i8] }> <{ ptr @anon.4101bb5dfac2026d99153c8951e5b276.1.llvm.2690330406010747703, [16 x i8] c"<\00\00\00\00\00\00\00$\00\00\00(\00\00\00" }>, align 8, !guid !2

; <hashbrown::raw::Fallibility>::capacity_overflow
; Function Attrs: nonlazybind uwtable
define { i64, i64 } @_RNvMNtCsaPaXZ31LQcT_9hashbrown3rawNtB2_11Fallibility17capacity_overflow(i1 noundef zeroext %0) unnamed_addr #0 !dbg !16 !guid !23 {
  br i1 %0, label %2, label %3, !dbg !24, !prof !25

2:                                                ; preds = %1
; call core::panicking::panic_fmt
  tail call void @_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt(ptr noundef nonnull @anon.4101bb5dfac2026d99153c8951e5b276.0.llvm.2690330406010747703, ptr noundef nonnull inttoptr (i64 57 to ptr), ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.4101bb5dfac2026d99153c8951e5b276.2.llvm.2690330406010747703) #3, !dbg !26
  unreachable, !dbg !26

3:                                                ; preds = %1
  ret { i64, i64 } { i64 0, i64 undef }, !dbg !27
}

; <hashbrown::raw::Fallibility>::alloc_err
; Function Attrs: nonlazybind uwtable
define { i64, i64 } @_RNvMNtCsaPaXZ31LQcT_9hashbrown3rawNtB2_11Fallibility9alloc_err(i1 noundef zeroext %0, i64 noundef range(i64 1, -9223372036854775807) %1, i64 noundef %2) unnamed_addr #0 !dbg !28 !guid !29 {
  br i1 %0, label %4, label %5, !dbg !30, !prof !25

4:                                                ; preds = %3
; call alloc::alloc::handle_alloc_error
  tail call void @_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error(i64 noundef %1, i64 noundef %2) #4, !dbg !31
  unreachable, !dbg !31

5:                                                ; preds = %3
  %6 = insertvalue { i64, i64 } poison, i64 %1, 0, !dbg !32
  %7 = insertvalue { i64, i64 } %6, i64 %2, 1, !dbg !32
  ret { i64, i64 } %7, !dbg !32
}

; core::panicking::panic_fmt
; Function Attrs: cold noinline noreturn nonlazybind uwtable
declare void @_RNvNtCs2k2z8Zem4rB_4core9panicking9panic_fmt(ptr noundef nonnull, ptr noundef nonnull, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24)) unnamed_addr #1

; alloc::alloc::handle_alloc_error
; Function Attrs: cold minsize noreturn nonlazybind optsize uwtable
declare void @_RNvNtCsc70TAahYccp_5alloc5alloc18handle_alloc_error(i64 noundef range(i64 1, -9223372036854775807), i64 noundef) unnamed_addr #2

attributes #0 = { nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #1 = { cold noinline noreturn nonlazybind uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #2 = { cold minsize noreturn nonlazybind optsize uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #3 = { noinline noreturn }
attributes #4 = { noreturn }

!llvm.module.flags = !{!3, !4, !5, !6, !7, !8}
!llvm.ident = !{!9}
!llvm.dbg.cu = !{!10, !12, !14}

!0 = !{i64 2924196544895364935}
!1 = !{i64 7732501625998638219}
!2 = !{i64 526129655266384600}
!3 = !{i32 8, !"PIC Level", i32 2}
!4 = !{i32 2, !"RtLibUseGOT", i32 1}
!5 = !{i32 7, !"uwtable", i32 2}
!6 = !{i32 7, !"frame-pointer", i32 1}
!7 = !{i32 7, !"Dwarf Version", i32 4}
!8 = !{i32 2, !"Debug Info Version", i32 3}
!9 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!10 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !11, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!11 = !DIFile(filename: "/cargo/registry/25cdd57fae9f0462/hashbrown-0.17.1/src/lib.rs/@/hashbrown.7e172c837a6a8ce9-cgu.0", directory: "/cargo/registry/25cdd57fae9f0462/hashbrown-0.17.1")
!12 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !13, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!13 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/alloc/src/lib.rs/@/alloc.8d16d54ddffdc8a3-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!14 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !15, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!15 = !DIFile(filename: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad/library/std/src/lib.rs/@/std.552422bcccb5c833-cgu.0", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad")
!16 = distinct !DISubprogram(name: "capacity_overflow", linkageName: "_RNvMNtCsaPaXZ31LQcT_9hashbrown3rawNtB2_11Fallibility17capacity_overflow", scope: !18, file: !17, line: 33, type: !21, scopeLine: 33, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !10, templateParams: !22)
!17 = !DIFile(filename: "src/raw.rs", directory: "/cargo/registry/25cdd57fae9f0462/hashbrown-0.17.1", checksumkind: CSK_MD5, checksum: "da1352104b4938bc7289a2cad1b5e1e6")
!18 = !DINamespace(name: "Fallibility", scope: !19)
!19 = !DINamespace(name: "raw", scope: !20)
!20 = !DINamespace(name: "hashbrown", scope: null)
!21 = !DISubroutineType(types: !22)
!22 = !{}
!23 = !{i64 -3361737598883843025}
!24 = !DILocation(line: 34, column: 9, scope: !16)
!25 = !{!"branch_weights", !"expected", i32 1, i32 2000}
!26 = !DILocation(line: 36, column: 40, scope: !16)
!27 = !DILocation(line: 38, column: 6, scope: !16)
!28 = distinct !DISubprogram(name: "alloc_err", linkageName: "_RNvMNtCsaPaXZ31LQcT_9hashbrown3rawNtB2_11Fallibility9alloc_err", scope: !18, file: !17, line: 42, type: !21, scopeLine: 42, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !10, templateParams: !22)
!29 = !{i64 1015319816181842799}
!30 = !DILocation(line: 43, column: 9, scope: !28)
!31 = !DILocation(line: 45, column: 40, scope: !28)
!32 = !DILocation(line: 47, column: 6, scope: !28)

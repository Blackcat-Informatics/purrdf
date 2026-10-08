; ModuleID = 'adler2-1b328b3a9fbc18c2.adler2.e3b6bd9f192b25d9-cgu.0.rcgu.o'
source_filename = "adler2.e3b6bd9f192b25d9-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

; <adler2::Adler32>::write_slice
; Function Attrs: nofree norecurse nosync nounwind nonlazybind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define void @_RNvMCsjy79vW79x0H_6adler2NtB2_7Adler3211write_slice(ptr noalias nofree noundef align 2 captures(none) dereferenceable(4) %0, ptr noalias nofree noundef nonnull readonly captures(address) %1, i64 noundef range(i64 0, -9223372036854775808) %2) unnamed_addr #0 !guid !5 {
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !9)
  %4 = load i16, ptr %0, align 2, !alias.scope !6, !noalias !9, !noundef !11
  %5 = zext i16 %4 to i32
  %6 = getelementptr inbounds nuw i8, ptr %0, i64 2
  %7 = load i16, ptr %6, align 2, !alias.scope !6, !noalias !9, !noundef !11
  %8 = zext i16 %7 to i32
  %9 = and i64 %2, 9223372036854775804
  %10 = getelementptr inbounds nuw i8, ptr %1, i64 %9
  %11 = and i64 %2, 3
  %12 = urem i64 %9, 22208
  %13 = sub nuw nsw i64 %9, %12
  %14 = getelementptr inbounds nuw i8, ptr %1, i64 %13
  %15 = icmp samesign ult i64 %13, 22208
  br i1 %15, label %.loopexit6, label %16

16:                                               ; preds = %3
  %17 = mul nuw nsw i32 %5, 22208
  br label %36

.loopexit6.loopexit:                              ; preds = %129
  %18 = extractelement <4 x i32> %134, i64 3
  %19 = extractelement <4 x i32> %134, i64 2
  %20 = extractelement <4 x i32> %134, i64 1
  %21 = extractelement <4 x i32> %134, i64 0
  %22 = extractelement <4 x i32> %133, i64 3
  %23 = extractelement <4 x i32> %133, i64 2
  %24 = extractelement <4 x i32> %133, i64 1
  %25 = extractelement <4 x i32> %133, i64 0
  br label %.loopexit6

.loopexit6:                                       ; preds = %.loopexit6.loopexit, %3
  %26 = phi i32 [ 0, %3 ], [ %20, %.loopexit6.loopexit ]
  %27 = phi i32 [ 0, %3 ], [ %19, %.loopexit6.loopexit ]
  %28 = phi i32 [ 0, %3 ], [ %18, %.loopexit6.loopexit ]
  %29 = phi i32 [ 0, %3 ], [ %22, %.loopexit6.loopexit ]
  %30 = phi i32 [ 0, %3 ], [ %23, %.loopexit6.loopexit ]
  %31 = phi i32 [ 0, %3 ], [ %24, %.loopexit6.loopexit ]
  %32 = phi i32 [ 0, %3 ], [ %21, %.loopexit6.loopexit ]
  %33 = phi i32 [ 0, %3 ], [ %25, %.loopexit6.loopexit ]
  %34 = phi i32 [ %8, %3 ], [ %135, %.loopexit6.loopexit ]
  %35 = icmp eq i64 %12, 0
  br i1 %35, label %51, label %.preheader5

36:                                               ; preds = %129, %16
  %37 = phi i32 [ %8, %16 ], [ %135, %129 ]
  %38 = phi i64 [ %13, %16 ], [ %131, %129 ]
  %39 = phi ptr [ %1, %16 ], [ %130, %129 ]
  %40 = phi <4 x i32> [ zeroinitializer, %16 ], [ %134, %129 ]
  %41 = phi <4 x i32> [ zeroinitializer, %16 ], [ %133, %129 ]
  br label %137

42:                                               ; preds = %.preheader5
  %43 = urem i32 %104, 65521
  %44 = urem i32 %105, 65521
  %45 = urem i32 %106, 65521
  %46 = urem i32 %107, 65521
  %47 = urem i32 %108, 65521
  %48 = urem i32 %109, 65521
  %49 = urem i32 %110, 65521
  %50 = urem i32 %111, 65521
  br label %51

51:                                               ; preds = %42, %.loopexit6
  %52 = phi i32 [ %26, %.loopexit6 ], [ %48, %42 ]
  %53 = phi i32 [ %27, %.loopexit6 ], [ %49, %42 ]
  %54 = phi i32 [ %28, %.loopexit6 ], [ %50, %42 ]
  %55 = phi i32 [ %29, %.loopexit6 ], [ %46, %42 ]
  %56 = phi i32 [ %30, %.loopexit6 ], [ %45, %42 ]
  %57 = phi i32 [ %31, %.loopexit6 ], [ %44, %42 ]
  %58 = phi i32 [ %32, %.loopexit6 ], [ %47, %42 ]
  %59 = phi i32 [ %33, %.loopexit6 ], [ %43, %42 ]
  %60 = sub nuw nsw i32 65521, %55
  %61 = mul nuw nsw i32 %60, 3
  %62 = trunc nuw nsw i64 %12 to i32
  %63 = mul nuw nsw i32 %5, %62
  %64 = add nuw nsw i32 %34, %63
  %65 = urem i32 %64, 65521
  %66 = add nuw nsw i32 %58, %54
  %67 = shl nuw nsw i32 %66, 2
  %68 = add nuw nsw i32 %53, %52
  %69 = shl nuw nsw i32 %68, 2
  %.neg4 = add nuw nsw i32 %65, 196563
  %70 = add nuw nsw i32 %.neg4, %69
  %71 = add nuw nsw i32 %70, %61
  %72 = shl nuw nsw i32 %56, 1
  %73 = add nuw nsw i32 %57, %72
  %74 = sub nuw nsw i32 %71, %73
  %75 = add nuw nsw i32 %74, %67
  %76 = add nuw nsw i32 %55, %5
  %77 = add nuw nsw i32 %76, %56
  %78 = add nuw nsw i32 %77, %57
  %79 = add nuw nsw i32 %78, %59
  %80 = icmp samesign eq i64 %11, 0
  br i1 %80, label %.loopexit, label %.preheader

.preheader5:                                      ; preds = %.loopexit6, %.preheader5
  %81 = phi i32 [ %111, %.preheader5 ], [ %28, %.loopexit6 ]
  %82 = phi i32 [ %110, %.preheader5 ], [ %27, %.loopexit6 ]
  %83 = phi i32 [ %109, %.preheader5 ], [ %26, %.loopexit6 ]
  %84 = phi i32 [ %107, %.preheader5 ], [ %29, %.loopexit6 ]
  %85 = phi i32 [ %106, %.preheader5 ], [ %30, %.loopexit6 ]
  %86 = phi i32 [ %105, %.preheader5 ], [ %31, %.loopexit6 ]
  %87 = phi ptr [ %91, %.preheader5 ], [ %14, %.loopexit6 ]
  %88 = phi i64 [ %92, %.preheader5 ], [ %12, %.loopexit6 ]
  %89 = phi i32 [ %104, %.preheader5 ], [ %33, %.loopexit6 ]
  %90 = phi i32 [ %108, %.preheader5 ], [ %32, %.loopexit6 ]
  %91 = getelementptr inbounds nuw i8, ptr %87, i64 4
  %92 = add i64 %88, -4
  %93 = load i8, ptr %87, align 1, !alias.scope !12, !noalias !15, !noundef !11
  %94 = getelementptr inbounds nuw i8, ptr %87, i64 1
  %95 = load i8, ptr %94, align 1, !alias.scope !12, !noalias !15, !noundef !11
  %96 = getelementptr inbounds nuw i8, ptr %87, i64 2
  %97 = load i8, ptr %96, align 1, !alias.scope !12, !noalias !15, !noundef !11
  %98 = zext i8 %97 to i32
  %99 = zext i8 %95 to i32
  %100 = zext i8 %93 to i32
  %101 = getelementptr inbounds nuw i8, ptr %87, i64 3
  %102 = load i8, ptr %101, align 1, !alias.scope !12, !noalias !15, !noundef !11
  %103 = zext i8 %102 to i32
  %104 = add i32 %89, %100
  %105 = add i32 %86, %99
  %106 = add i32 %85, %98
  %107 = add i32 %84, %103
  %108 = add i32 %104, %90
  %109 = add i32 %105, %83
  %110 = add i32 %106, %82
  %111 = add i32 %107, %81
  %112 = icmp eq i64 %92, 0
  br i1 %112, label %42, label %.preheader5

.preheader:                                       ; preds = %51
  %113 = load i8, ptr %10, align 1, !alias.scope !9, !noalias !6, !noundef !11
  %114 = zext i8 %113 to i32
  %115 = add i32 %79, %114
  %116 = add i32 %115, %75
  %117 = icmp samesign eq i64 %11, 1
  br i1 %117, label %.loopexit, label %.preheader.1

.preheader.1:                                     ; preds = %.preheader
  %118 = getelementptr inbounds nuw i8, ptr %10, i64 1
  %119 = load i8, ptr %118, align 1, !alias.scope !9, !noalias !6, !noundef !11
  %120 = zext i8 %119 to i32
  %121 = add i32 %115, %120
  %122 = add i32 %121, %116
  %123 = icmp samesign eq i64 %11, 2
  br i1 %123, label %.loopexit, label %.preheader.2

.preheader.2:                                     ; preds = %.preheader.1
  %124 = getelementptr inbounds nuw i8, ptr %10, i64 2
  %125 = load i8, ptr %124, align 1, !alias.scope !9, !noalias !6, !noundef !11
  %126 = zext i8 %125 to i32
  %127 = add i32 %121, %126
  %128 = add i32 %127, %122
  br label %.loopexit

129:                                              ; preds = %137
  %130 = getelementptr inbounds nuw i8, ptr %39, i64 22208
  %131 = add nsw i64 %38, -22208
  %132 = add nuw nsw i32 %37, %17
  %133 = urem <4 x i32> %161, splat (i32 65521)
  %134 = urem <4 x i32> %162, splat (i32 65521)
  %135 = urem i32 %132, 65521
  %136 = icmp ult i64 %131, 22208
  br i1 %136, label %.loopexit6.loopexit, label %36

137:                                              ; preds = %137, %36
  %138 = phi ptr [ %39, %36 ], [ %157, %137 ]
  %139 = phi i64 [ 22208, %36 ], [ %158, %137 ]
  %140 = phi <4 x i32> [ %40, %36 ], [ %162, %137 ]
  %141 = phi <4 x i32> [ %41, %36 ], [ %161, %137 ]
  %142 = getelementptr inbounds nuw i8, ptr %138, i64 4
  %143 = load <4 x i8>, ptr %138, align 1, !alias.scope !17, !noalias !20
  %144 = zext <4 x i8> %143 to <4 x i32>
  %145 = add <4 x i32> %141, %144
  %146 = add <4 x i32> %145, %140
  %147 = getelementptr inbounds nuw i8, ptr %138, i64 8
  %148 = load <4 x i8>, ptr %142, align 1, !alias.scope !17, !noalias !20
  %149 = zext <4 x i8> %148 to <4 x i32>
  %150 = add <4 x i32> %145, %149
  %151 = add <4 x i32> %150, %146
  %152 = getelementptr inbounds nuw i8, ptr %138, i64 12
  %153 = load <4 x i8>, ptr %147, align 1, !alias.scope !17, !noalias !20
  %154 = zext <4 x i8> %153 to <4 x i32>
  %155 = add <4 x i32> %150, %154
  %156 = add <4 x i32> %155, %151
  %157 = getelementptr inbounds nuw i8, ptr %138, i64 16
  %158 = add nsw i64 %139, -16
  %159 = load <4 x i8>, ptr %152, align 1, !alias.scope !17, !noalias !20
  %160 = zext <4 x i8> %159 to <4 x i32>
  %161 = add <4 x i32> %155, %160
  %162 = add <4 x i32> %161, %156
  %163 = icmp eq i64 %158, 0
  br i1 %163, label %129, label %137

.loopexit:                                        ; preds = %.preheader, %.preheader.1, %.preheader.2, %51
  %164 = phi i32 [ %75, %51 ], [ %116, %.preheader ], [ %122, %.preheader.1 ], [ %128, %.preheader.2 ]
  %165 = phi i32 [ %79, %51 ], [ %115, %.preheader ], [ %121, %.preheader.1 ], [ %127, %.preheader.2 ]
  %166 = urem i32 %165, 65521
  %167 = trunc nuw i32 %166 to i16
  store i16 %167, ptr %0, align 2, !alias.scope !6, !noalias !9
  %168 = urem i32 %164, 65521
  %169 = trunc nuw i32 %168 to i16
  store i16 %169, ptr %6, align 2, !alias.scope !6, !noalias !9
  ret void
}

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite)
declare void @llvm.experimental.noalias.scope.decl(metadata) #1

attributes #0 = { nofree norecurse nosync nounwind nonlazybind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #1 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite) }

!llvm.module.flags = !{!0, !1, !2, !3}
!llvm.ident = !{!4}

!0 = !{i32 8, !"PIC Level", i32 2}
!1 = !{i32 2, !"RtLibUseGOT", i32 1}
!2 = !{i32 7, !"uwtable", i32 2}
!3 = !{i32 7, !"frame-pointer", i32 1}
!4 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!5 = !{i64 8835789193618863045}
!6 = !{!7}
!7 = distinct !{!7, !8, !"_RNvMNtCsjy79vW79x0H_6adler24algoNtB4_7Adler327compute: argument 0"}
!8 = distinct !{!8, !"_RNvMNtCsjy79vW79x0H_6adler24algoNtB4_7Adler327compute"}
!9 = !{!10}
!10 = distinct !{!10, !8, !"_RNvMNtCsjy79vW79x0H_6adler24algoNtB4_7Adler327compute: argument 1"}
!11 = !{}
!12 = !{!13, !10}
!13 = distinct !{!13, !14, !"_RNvMs_NtCsjy79vW79x0H_6adler24algoNtB4_5U32X44from: argument 1"}
!14 = distinct !{!14, !"_RNvMs_NtCsjy79vW79x0H_6adler24algoNtB4_5U32X44from"}
!15 = !{!16, !7}
!16 = distinct !{!16, !14, !"_RNvMs_NtCsjy79vW79x0H_6adler24algoNtB4_5U32X44from: argument 0"}
!17 = !{!18, !10}
!18 = distinct !{!18, !19, !"_RNvMs_NtCsjy79vW79x0H_6adler24algoNtB4_5U32X44from: argument 1"}
!19 = distinct !{!19, !"_RNvMs_NtCsjy79vW79x0H_6adler24algoNtB4_5U32X44from"}
!20 = !{!21, !7}
!21 = distinct !{!21, !19, !"_RNvMs_NtCsjy79vW79x0H_6adler24algoNtB4_5U32X44from: argument 0"}

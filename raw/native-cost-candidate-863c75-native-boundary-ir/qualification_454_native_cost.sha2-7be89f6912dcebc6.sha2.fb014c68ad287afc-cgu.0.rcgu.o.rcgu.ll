; ModuleID = 'sha2-7be89f6912dcebc6.sha2.fb014c68ad287afc-cgu.0.rcgu.o'
source_filename = "sha2.fb014c68ad287afc-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@anon.deeb35d3f195dc5f9a0c48b58e3e09c8.0 = private unnamed_addr constant [640 x i8] c"\22\AE(\D7\98/\8AB\CDe\EF#\91D7q/;M\EC\CF\FB\C0\B5\BC\DB\89\81\A5\DB\B5\E98\B5H\F3[\C2V9\19\D0\05\B6\F1\11\F1Y\9BO\19\AF\A4\82?\92\18\81m\DA\D5^\1C\ABB\02\03\A3\98\AA\07\D8\BEopE\01[\83\12\8C\B2\E4N\BE\851$\E2\B4\FF\D5\C3}\0CUo\89{\F2t]\BEr\B1\96\16;\FE\B1\DE\805\12\C7%\A7\06\DC\9B\94&i\CFt\F1\9B\C1\D2J\F1\9E\C1i\9B\E4\E3%O8\86G\BE\EF\B5\D5\8C\8B\C6\9D\C1\0Fe\9C\ACw\CC\A1\0C$u\02+Yo,\E9-\83\E4\A6n\AA\84tJ\D4\FBA\BD\DC\A9\B0\\\B5S\11\83\DA\88\F9v\AB\DFf\EERQ>\98\102\B4-m\C61\A8?!\FB\98\C8'\03\B0\E4\0E\EF\BE\C7\7FY\BF\C2\8F\A8=\F3\0B\E0\C6%\A7\0A\93G\91\A7\D5o\82\03\E0Qc\CA\06pn\0E\0Ag))\14\FC/\D2F\85\0A\B7'&\C9&\\8!\1B.\ED*\C4Z\FCm,M\DF\B3\95\9D\13\0D8S\DEc\AF\8BTs\0Ae\A8\B2w<\BB\0Ajv\E6\AE\EDG.\C9\C2\81;5\82\14\85,r\92d\03\F1L\A1\E8\BF\A2\010B\BCKf\1A\A8\91\97\F8\D0p\8BK\C20\BET\06\A3Ql\C7\18R\EF\D6\19\E8\92\D1\10\A9eU$\06\99\D6* qW\855\0E\F4\B8\D1\BB2p\A0j\10\C8\D0\D2\B8\16\C1\A4\19S\ABAQ\08l7\1E\99\EB\8E\DFLwH'\A8H\9B\E1\B5\BC\B04cZ\C9\C5\B3\0C\1C9\CB\8AA\E3J\AA\D8Ns\E3cwO\CA\9C[\A3\B8\B2\D6\F3o.h\FC\B2\EF]\EE\82\8Ft`/\17Coc\A5xr\AB\F0\A1\14x\C8\84\EC9d\1A\08\02\C7\8C(\1Ec#\FA\FF\BE\90\E9\BD\82\DE\EBlP\A4\15y\C6\B2\F7\A3\F9\BE+Sr\E3\F2xq\C6\9Ca&\EA\CE>'\CA\07\C2\C0!\C7\B8\86\D1\1E\EB\E0\CD\D6}\DA\EAx\D1n\EE\7FO}\F5\BAo\17r\AAg\F0\06\A6\98\C8\A2\C5}c\0A\AE\0D\F9\BE\04\98?\11\1BG\1C\135\0Bq\1B\84}\04#\F5w\DB(\93$\C7@{\AB\CA2\BC\BE\C9\15\0A\BE\9E<L\0D\10\9C\C4g\1DC\B6B>\CB\BE\D4\C5L*~e\FC\9C)\7FY\EC\FA\D6:\ABo\CB_\17XGJ\8C\19Dl", align 8, !guid !0

; sha2::sha256::compress256
; Function Attrs: nofree norecurse nosync nounwind nonlazybind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define void @_RNvNtCsly5PKRZWayS_4sha26sha25611compress256(ptr noalias nofree noundef align 4 captures(none) dereferenceable(32) %0, ptr noalias nofree noundef nonnull readonly captures(address) %1, i64 noundef range(i64 0, 144115188075855872) %2) unnamed_addr #0 !guid !5 {
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !9)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !11)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !14)
  %4 = load <4 x i32>, ptr %0, align 4, !alias.scope !16, !noalias !17
  %5 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %6 = load <4 x i32>, ptr %5, align 4, !alias.scope !16, !noalias !20
  %7 = shufflevector <4 x i32> %6, <4 x i32> %4, <4 x i32> <i32 1, i32 0, i32 5, i32 4>
  %8 = shufflevector <4 x i32> %6, <4 x i32> %4, <4 x i32> <i32 3, i32 2, i32 7, i32 6>
  %9 = shl nuw nsw i64 %2, 6
  %10 = getelementptr inbounds nuw i8, ptr %1, i64 %9
  %11 = icmp eq i64 %2, 0
  br i1 %11, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %3, %.preheader
  %12 = phi <4 x i32> [ %145, %.preheader ], [ %7, %3 ]
  %13 = phi <4 x i32> [ %146, %.preheader ], [ %8, %3 ]
  %14 = phi ptr [ %15, %.preheader ], [ %1, %3 ]
  %15 = getelementptr inbounds nuw i8, ptr %14, i64 64
  %16 = load <16 x i8>, ptr %14, align 1, !alias.scope !23, !noalias !24
  %17 = shufflevector <16 x i8> %16, <16 x i8> poison, <16 x i32> <i32 3, i32 2, i32 1, i32 0, i32 7, i32 6, i32 5, i32 4, i32 11, i32 10, i32 9, i32 8, i32 15, i32 14, i32 13, i32 12>
  %18 = getelementptr inbounds nuw i8, ptr %14, i64 16
  %19 = load <16 x i8>, ptr %18, align 1, !alias.scope !23, !noalias !27
  %20 = shufflevector <16 x i8> %19, <16 x i8> poison, <16 x i32> <i32 3, i32 2, i32 1, i32 0, i32 7, i32 6, i32 5, i32 4, i32 11, i32 10, i32 9, i32 8, i32 15, i32 14, i32 13, i32 12>
  %21 = getelementptr inbounds nuw i8, ptr %14, i64 32
  %22 = load <16 x i8>, ptr %21, align 1, !alias.scope !23, !noalias !30
  %23 = shufflevector <16 x i8> %22, <16 x i8> poison, <16 x i32> <i32 3, i32 2, i32 1, i32 0, i32 7, i32 6, i32 5, i32 4, i32 11, i32 10, i32 9, i32 8, i32 15, i32 14, i32 13, i32 12>
  %24 = getelementptr inbounds nuw i8, ptr %14, i64 48
  %25 = load <16 x i8>, ptr %24, align 1, !alias.scope !23, !noalias !33
  %26 = shufflevector <16 x i8> %25, <16 x i8> poison, <16 x i32> <i32 3, i32 2, i32 1, i32 0, i32 7, i32 6, i32 5, i32 4, i32 11, i32 10, i32 9, i32 8, i32 15, i32 14, i32 13, i32 12>
  %27 = bitcast <16 x i8> %17 to <4 x i32>
  %28 = add <4 x i32> %27, <i32 1116352408, i32 1899447441, i32 -1245643825, i32 -373957723>
  %29 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %13, <4 x i32> %12, <4 x i32> %28)
  %30 = shufflevector <4 x i32> %28, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %31 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %12, <4 x i32> %29, <4 x i32> %30)
  %32 = bitcast <16 x i8> %20 to <4 x i32>
  %33 = add <4 x i32> %32, <i32 961987163, i32 1508970993, i32 -1841331548, i32 -1424204075>
  %34 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %29, <4 x i32> %31, <4 x i32> %33)
  %35 = shufflevector <4 x i32> %33, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %36 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %31, <4 x i32> %34, <4 x i32> %35)
  %37 = bitcast <16 x i8> %23 to <4 x i32>
  %38 = add <4 x i32> %37, <i32 -670586216, i32 310598401, i32 607225278, i32 1426881987>
  %39 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %34, <4 x i32> %36, <4 x i32> %38)
  %40 = shufflevector <4 x i32> %38, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %41 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %36, <4 x i32> %39, <4 x i32> %40)
  %42 = bitcast <16 x i8> %26 to <4 x i32>
  %43 = add <4 x i32> %42, <i32 1925078388, i32 -2132889090, i32 -1680079193, i32 -1046744716>
  %44 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %39, <4 x i32> %41, <4 x i32> %43)
  %45 = shufflevector <4 x i32> %43, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %46 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %41, <4 x i32> %44, <4 x i32> %45)
  %47 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %27, <4 x i32> %32)
  %48 = shufflevector <16 x i8> %23, <16 x i8> %26, <16 x i32> <i32 4, i32 5, i32 6, i32 7, i32 8, i32 9, i32 10, i32 11, i32 12, i32 13, i32 14, i32 15, i32 16, i32 17, i32 18, i32 19>
  %49 = bitcast <16 x i8> %48 to <4 x i32>
  %50 = add <4 x i32> %47, %49
  %51 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %50, <4 x i32> %42)
  %52 = add <4 x i32> %51, <i32 -459576895, i32 -272742522, i32 264347078, i32 604807628>
  %53 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %44, <4 x i32> %46, <4 x i32> %52)
  %54 = shufflevector <4 x i32> %52, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %55 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %46, <4 x i32> %53, <4 x i32> %54)
  %56 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %32, <4 x i32> %37)
  %57 = bitcast <16 x i8> %26 to <4 x i32>
  %58 = shufflevector <4 x i32> %57, <4 x i32> %51, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %59 = add <4 x i32> %56, %58
  %60 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %59, <4 x i32> %51)
  %61 = add <4 x i32> %60, <i32 770255983, i32 1249150122, i32 1555081692, i32 1996064986>
  %62 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %53, <4 x i32> %55, <4 x i32> %61)
  %63 = shufflevector <4 x i32> %61, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %64 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %55, <4 x i32> %62, <4 x i32> %63)
  %65 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %37, <4 x i32> %42)
  %66 = shufflevector <4 x i32> %51, <4 x i32> %60, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %67 = add <4 x i32> %65, %66
  %68 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %67, <4 x i32> %60)
  %69 = add <4 x i32> %68, <i32 -1740746414, i32 -1473132947, i32 -1341970488, i32 -1084653625>
  %70 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %62, <4 x i32> %64, <4 x i32> %69)
  %71 = shufflevector <4 x i32> %69, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %72 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %64, <4 x i32> %70, <4 x i32> %71)
  %73 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %42, <4 x i32> %51)
  %74 = shufflevector <4 x i32> %60, <4 x i32> %68, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %75 = add <4 x i32> %73, %74
  %76 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %75, <4 x i32> %68)
  %77 = add <4 x i32> %76, <i32 -958395405, i32 -710438585, i32 113926993, i32 338241895>
  %78 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %70, <4 x i32> %72, <4 x i32> %77)
  %79 = shufflevector <4 x i32> %77, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %80 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %72, <4 x i32> %78, <4 x i32> %79)
  %81 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %51, <4 x i32> %60)
  %82 = shufflevector <4 x i32> %68, <4 x i32> %76, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %83 = add <4 x i32> %81, %82
  %84 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %83, <4 x i32> %76)
  %85 = add <4 x i32> %84, <i32 666307205, i32 773529912, i32 1294757372, i32 1396182291>
  %86 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %78, <4 x i32> %80, <4 x i32> %85)
  %87 = shufflevector <4 x i32> %85, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %88 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %80, <4 x i32> %86, <4 x i32> %87)
  %89 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %60, <4 x i32> %68)
  %90 = shufflevector <4 x i32> %76, <4 x i32> %84, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %91 = add <4 x i32> %89, %90
  %92 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %91, <4 x i32> %84)
  %93 = add <4 x i32> %92, <i32 1695183700, i32 1986661051, i32 -2117940946, i32 -1838011259>
  %94 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %86, <4 x i32> %88, <4 x i32> %93)
  %95 = shufflevector <4 x i32> %93, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %96 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %88, <4 x i32> %94, <4 x i32> %95)
  %97 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %68, <4 x i32> %76)
  %98 = shufflevector <4 x i32> %84, <4 x i32> %92, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %99 = add <4 x i32> %97, %98
  %100 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %99, <4 x i32> %92)
  %101 = add <4 x i32> %100, <i32 -1564481375, i32 -1474664885, i32 -1035236496, i32 -949202525>
  %102 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %94, <4 x i32> %96, <4 x i32> %101)
  %103 = shufflevector <4 x i32> %101, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %104 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %96, <4 x i32> %102, <4 x i32> %103)
  %105 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %76, <4 x i32> %84)
  %106 = shufflevector <4 x i32> %92, <4 x i32> %100, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %107 = add <4 x i32> %105, %106
  %108 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %107, <4 x i32> %100)
  %109 = add <4 x i32> %108, <i32 -778901479, i32 -694614492, i32 -200395387, i32 275423344>
  %110 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %102, <4 x i32> %104, <4 x i32> %109)
  %111 = shufflevector <4 x i32> %109, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %112 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %104, <4 x i32> %110, <4 x i32> %111)
  %113 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %84, <4 x i32> %92)
  %114 = shufflevector <4 x i32> %100, <4 x i32> %108, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %115 = add <4 x i32> %113, %114
  %116 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %115, <4 x i32> %108)
  %117 = add <4 x i32> %116, <i32 430227734, i32 506948616, i32 659060556, i32 883997877>
  %118 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %110, <4 x i32> %112, <4 x i32> %117)
  %119 = shufflevector <4 x i32> %117, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %120 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %112, <4 x i32> %118, <4 x i32> %119)
  %121 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %92, <4 x i32> %100)
  %122 = shufflevector <4 x i32> %108, <4 x i32> %116, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %123 = add <4 x i32> %121, %122
  %124 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %123, <4 x i32> %116)
  %125 = add <4 x i32> %124, <i32 958139571, i32 1322822218, i32 1537002063, i32 1747873779>
  %126 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %118, <4 x i32> %120, <4 x i32> %125)
  %127 = shufflevector <4 x i32> %125, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %128 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %120, <4 x i32> %126, <4 x i32> %127)
  %129 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %100, <4 x i32> %108)
  %130 = shufflevector <4 x i32> %116, <4 x i32> %124, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %131 = add <4 x i32> %129, %130
  %132 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %131, <4 x i32> %124)
  %133 = add <4 x i32> %132, <i32 1955562222, i32 2024104815, i32 -2067236844, i32 -1933114872>
  %134 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %126, <4 x i32> %128, <4 x i32> %133)
  %135 = shufflevector <4 x i32> %133, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %136 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %128, <4 x i32> %134, <4 x i32> %135)
  %137 = tail call <4 x i32> @llvm.x86.sha256msg1(<4 x i32> %108, <4 x i32> %116)
  %138 = shufflevector <4 x i32> %124, <4 x i32> %132, <4 x i32> <i32 1, i32 2, i32 3, i32 4>
  %139 = add <4 x i32> %137, %138
  %140 = tail call <4 x i32> @llvm.x86.sha256msg2(<4 x i32> %139, <4 x i32> %132)
  %141 = add <4 x i32> %140, <i32 -1866530822, i32 -1538233109, i32 -1090935817, i32 -965641998>
  %142 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %134, <4 x i32> %136, <4 x i32> %141)
  %143 = shufflevector <4 x i32> %141, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 0>
  %144 = tail call <4 x i32> @llvm.x86.sha256rnds2(<4 x i32> %136, <4 x i32> %142, <4 x i32> %143)
  %145 = add <4 x i32> %144, %12
  %146 = add <4 x i32> %142, %13
  %147 = icmp eq ptr %15, %10
  br i1 %147, label %.loopexit, label %.preheader

.loopexit:                                        ; preds = %.preheader, %3
  %148 = phi <4 x i32> [ %8, %3 ], [ %146, %.preheader ]
  %149 = phi <4 x i32> [ %7, %3 ], [ %145, %.preheader ]
  %150 = shufflevector <4 x i32> %149, <4 x i32> %148, <4 x i32> <i32 3, i32 2, i32 7, i32 6>
  %151 = shufflevector <4 x i32> %149, <4 x i32> %148, <4 x i32> <i32 1, i32 0, i32 5, i32 4>
  store <4 x i32> %150, ptr %0, align 4, !alias.scope !16, !noalias !36
  store <4 x i32> %151, ptr %5, align 4, !alias.scope !16, !noalias !39
  ret void
}

; sha2::sha512::compress512
; Function Attrs: nofree norecurse nosync nounwind nonlazybind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define void @_RNvNtCsly5PKRZWayS_4sha26sha51211compress512(ptr noalias nofree noundef align 8 captures(none) dereferenceable(64) %0, ptr noalias nofree noundef nonnull readonly captures(none) %1, i64 noundef range(i64 0, 72057594037927936) %2) unnamed_addr #0 personality ptr @rust_eh_personality !guid !42 {
  %4 = alloca [128 x i8], align 16
  %5 = alloca [640 x i8], align 16
  %6 = alloca [128 x i8], align 16
  tail call void @llvm.experimental.noalias.scope.decl(metadata !43)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !46)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !48)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !51)
  %7 = and i64 %2, 1
  %8 = icmp eq i64 %7, 0
  br i1 %8, label %9, label %16

9:                                                ; preds = %318, %3
  %10 = phi i64 [ 0, %3 ], [ 1, %318 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !53
  call void @llvm.lifetime.start.p0(ptr nonnull %5), !noalias !53
  %11 = getelementptr inbounds nuw i8, ptr %5, i64 128
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 16 dereferenceable(512) %11, i8 0, i64 512, i1 false), !noalias !53
  %12 = tail call i64 @llvm.usub.sat.i64(i64 range(i64 0, 72057594037927936) %2, i64 range(i64 0, 2) %10)
  %13 = lshr i64 %12, 1
  %14 = sub nsw i64 %12, %13
  %15 = icmp eq i64 %14, 0
  br i1 %15, label %839, label %327

16:                                               ; preds = %3
  tail call void @llvm.experimental.noalias.scope.decl(metadata !54)
  call void @llvm.lifetime.start.p0(ptr nonnull %4), !noalias !57
  %17 = load i64, ptr %0, align 8, !alias.scope !59, !noalias !60
  %18 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %19 = load i64, ptr %18, align 8, !alias.scope !59, !noalias !60
  %20 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %21 = load i64, ptr %20, align 8, !alias.scope !59, !noalias !60
  %22 = getelementptr inbounds nuw i8, ptr %0, i64 24
  %23 = load i64, ptr %22, align 8, !alias.scope !59, !noalias !60
  %24 = getelementptr inbounds nuw i8, ptr %0, i64 32
  %25 = load i64, ptr %24, align 8, !alias.scope !59, !noalias !60
  %26 = getelementptr inbounds nuw i8, ptr %0, i64 40
  %27 = load i64, ptr %26, align 8, !alias.scope !59, !noalias !60
  %28 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %29 = load i64, ptr %28, align 8, !alias.scope !59, !noalias !60
  %30 = getelementptr inbounds nuw i8, ptr %0, i64 56
  %31 = load i64, ptr %30, align 8, !alias.scope !59, !noalias !60
  tail call void @llvm.experimental.noalias.scope.decl(metadata !61)
  %32 = load <16 x i8>, ptr %1, align 1, !alias.scope !64, !noalias !65
  %33 = shufflevector <16 x i8> %32, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %34 = bitcast <16 x i8> %33 to <2 x i64>
  %35 = add <2 x i64> %34, <i64 4794697086780616226, i64 8158064640168781261>
  store <2 x i64> %35, ptr %4, align 16, !alias.scope !61, !noalias !69
  %36 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %37 = load <16 x i8>, ptr %36, align 1, !alias.scope !64, !noalias !70
  %38 = shufflevector <16 x i8> %37, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %39 = bitcast <16 x i8> %38 to <2 x i64>
  %40 = add <2 x i64> %39, <i64 -5349999486874862801, i64 -1606136188198331460>
  %41 = getelementptr inbounds nuw i8, ptr %4, i64 16
  store <2 x i64> %40, ptr %41, align 16, !alias.scope !61, !noalias !69
  %42 = getelementptr inbounds nuw i8, ptr %1, i64 32
  %43 = load <16 x i8>, ptr %42, align 1, !alias.scope !64, !noalias !73
  %44 = shufflevector <16 x i8> %43, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %45 = bitcast <16 x i8> %44 to <2 x i64>
  %46 = add <2 x i64> %45, <i64 4131703408338449720, i64 6480981068601479193>
  %47 = getelementptr inbounds nuw i8, ptr %4, i64 32
  store <2 x i64> %46, ptr %47, align 16, !alias.scope !61, !noalias !69
  %48 = getelementptr inbounds nuw i8, ptr %1, i64 48
  %49 = load <16 x i8>, ptr %48, align 1, !alias.scope !64, !noalias !76
  %50 = shufflevector <16 x i8> %49, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %51 = bitcast <16 x i8> %50 to <2 x i64>
  %52 = add <2 x i64> %51, <i64 -7908458776815382629, i64 -6116909921290321640>
  %53 = getelementptr inbounds nuw i8, ptr %4, i64 48
  store <2 x i64> %52, ptr %53, align 16, !alias.scope !61, !noalias !69
  %54 = getelementptr inbounds nuw i8, ptr %1, i64 64
  %55 = load <16 x i8>, ptr %54, align 1, !alias.scope !64, !noalias !79
  %56 = shufflevector <16 x i8> %55, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %57 = bitcast <16 x i8> %56 to <2 x i64>
  %58 = add <2 x i64> %57, <i64 -2880145864133508542, i64 1334009975649890238>
  %59 = getelementptr inbounds nuw i8, ptr %4, i64 64
  store <2 x i64> %58, ptr %59, align 16, !alias.scope !61, !noalias !69
  %60 = getelementptr inbounds nuw i8, ptr %1, i64 80
  %61 = load <16 x i8>, ptr %60, align 1, !alias.scope !64, !noalias !82
  %62 = shufflevector <16 x i8> %61, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %63 = bitcast <16 x i8> %62 to <2 x i64>
  %64 = add <2 x i64> %63, <i64 2608012711638119052, i64 6128411473006802146>
  %65 = getelementptr inbounds nuw i8, ptr %4, i64 80
  store <2 x i64> %64, ptr %65, align 16, !alias.scope !61, !noalias !69
  %66 = getelementptr inbounds nuw i8, ptr %1, i64 96
  %67 = load <16 x i8>, ptr %66, align 1, !alias.scope !64, !noalias !85
  %68 = shufflevector <16 x i8> %67, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %69 = bitcast <16 x i8> %68 to <2 x i64>
  %70 = add <2 x i64> %69, <i64 8268148722764581231, i64 -9160688886553864527>
  %71 = getelementptr inbounds nuw i8, ptr %4, i64 96
  store <2 x i64> %70, ptr %71, align 16, !alias.scope !61, !noalias !69
  %72 = getelementptr inbounds nuw i8, ptr %1, i64 112
  %73 = load <16 x i8>, ptr %72, align 1, !alias.scope !64, !noalias !88
  %74 = shufflevector <16 x i8> %73, <16 x i8> poison, <16 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8>
  %75 = bitcast <16 x i8> %74 to <2 x i64>
  %76 = add <2 x i64> %75, <i64 -7215885187991268811, i64 -4495734319001033068>
  %77 = getelementptr inbounds nuw i8, ptr %4, i64 112
  store <2 x i64> %76, ptr %77, align 16, !alias.scope !61, !noalias !69
  tail call void @llvm.experimental.noalias.scope.decl(metadata !91)
  br label %81

78:                                               ; preds = %100
  %79 = add nuw nsw i32 %83, 1
  %80 = icmp eq i32 %79, 4
  br i1 %80, label %.preheader4, label %81

81:                                               ; preds = %78, %16
  %82 = phi i64 [ 16, %16 ], [ %198, %78 ]
  %83 = phi i32 [ 0, %16 ], [ %79, %78 ]
  %84 = phi <2 x i64> [ %39, %16 ], [ %108, %78 ]
  %85 = phi <2 x i64> [ %34, %16 ], [ %103, %78 ]
  %86 = phi <2 x i64> [ %63, %16 ], [ %110, %78 ]
  %87 = phi <2 x i64> [ %57, %16 ], [ %105, %78 ]
  %88 = phi <2 x i64> [ %75, %16 ], [ %144, %78 ]
  %89 = phi <2 x i64> [ %45, %16 ], [ %109, %78 ]
  %90 = phi <2 x i64> [ %51, %16 ], [ %106, %78 ]
  %91 = phi <2 x i64> [ %69, %16 ], [ %107, %78 ]
  %92 = phi i64 [ %31, %16 ], [ %114, %78 ]
  %93 = phi i64 [ %25, %16 ], [ %196, %78 ]
  %94 = phi i64 [ %29, %16 ], [ %112, %78 ]
  %95 = phi i64 [ %27, %16 ], [ %171, %78 ]
  %96 = phi i64 [ %17, %16 ], [ %195, %78 ]
  %97 = phi i64 [ %19, %16 ], [ %170, %78 ]
  %98 = phi i64 [ %21, %16 ], [ %115, %78 ]
  %99 = phi i64 [ %23, %16 ], [ %116, %78 ]
  br label %100

100:                                              ; preds = %100, %81
  %101 = phi i64 [ %82, %81 ], [ %198, %100 ]
  %102 = phi i64 [ 0, %81 ], [ %119, %100 ]
  %103 = phi <2 x i64> [ %84, %81 ], [ %108, %100 ]
  %104 = phi <2 x i64> [ %85, %81 ], [ %103, %100 ]
  %105 = phi <2 x i64> [ %86, %81 ], [ %110, %100 ]
  %106 = phi <2 x i64> [ %87, %81 ], [ %105, %100 ]
  %107 = phi <2 x i64> [ %88, %81 ], [ %144, %100 ]
  %108 = phi <2 x i64> [ %89, %81 ], [ %109, %100 ]
  %109 = phi <2 x i64> [ %90, %81 ], [ %106, %100 ]
  %110 = phi <2 x i64> [ %91, %81 ], [ %107, %100 ]
  %111 = phi i64 [ %92, %81 ], [ %114, %100 ]
  %112 = phi i64 [ %93, %81 ], [ %196, %100 ]
  %113 = phi i64 [ %94, %81 ], [ %112, %100 ]
  %114 = phi i64 [ %95, %81 ], [ %171, %100 ]
  %115 = phi i64 [ %96, %81 ], [ %195, %100 ]
  %116 = phi i64 [ %97, %81 ], [ %170, %100 ]
  %117 = phi i64 [ %98, %81 ], [ %115, %100 ]
  %118 = phi i64 [ %99, %81 ], [ %116, %100 ]
  %119 = add nuw nsw i64 %102, 1
  %120 = getelementptr inbounds nuw [8 x i8], ptr @anon.deeb35d3f195dc5f9a0c48b58e3e09c8.0, i64 %101
  %121 = load <2 x i64>, ptr %120, align 8, !noalias !94
  %122 = shufflevector <2 x i64> %104, <2 x i64> %103, <2 x i32> <i32 1, i32 2>
  %123 = shufflevector <2 x i64> %106, <2 x i64> %105, <2 x i32> <i32 1, i32 2>
  %124 = lshr <2 x i64> %122, splat (i64 1)
  %125 = add <2 x i64> %104, %123
  %126 = lshr <2 x i64> %122, splat (i64 7)
  %127 = shl <2 x i64> %122, splat (i64 56)
  %128 = xor <2 x i64> %124, %126
  %129 = lshr <2 x i64> %122, splat (i64 8)
  %130 = xor <2 x i64> %128, %127
  %131 = shl <2 x i64> %122, splat (i64 63)
  %132 = xor <2 x i64> %130, %129
  %133 = xor <2 x i64> %132, %131
  %134 = lshr <2 x i64> %107, splat (i64 6)
  %135 = shl <2 x i64> %107, splat (i64 3)
  %136 = add <2 x i64> %125, %133
  %137 = lshr <2 x i64> %107, splat (i64 19)
  %138 = xor <2 x i64> %135, %134
  %139 = shl <2 x i64> %107, splat (i64 45)
  %140 = xor <2 x i64> %138, %137
  %141 = lshr <2 x i64> %107, splat (i64 61)
  %142 = xor <2 x i64> %140, %139
  %143 = xor <2 x i64> %142, %141
  %144 = add <2 x i64> %136, %143
  %145 = add <2 x i64> %121, %144
  %146 = shl nuw nsw i64 %102, 4
  %147 = getelementptr inbounds nuw i8, ptr %4, i64 %146
  %148 = load i64, ptr %147, align 16, !alias.scope !91, !noalias !97, !noundef !100
  %149 = tail call i64 @llvm.fshl.i64(i64 %112, i64 %112, i64 50)
  %150 = tail call i64 @llvm.fshl.i64(i64 %112, i64 %112, i64 46)
  %151 = xor i64 %149, %150
  %152 = tail call i64 @llvm.fshl.i64(i64 %112, i64 %112, i64 23)
  %153 = xor i64 %151, %152
  %154 = xor i64 %114, %113
  %155 = and i64 %154, %112
  %156 = xor i64 %155, %113
  %157 = add i64 %153, %111
  %158 = add i64 %157, %156
  %159 = add i64 %158, %148
  %160 = tail call i64 @llvm.fshl.i64(i64 %115, i64 %115, i64 36)
  %161 = tail call i64 @llvm.fshl.i64(i64 %115, i64 %115, i64 30)
  %162 = xor i64 %160, %161
  %163 = tail call i64 @llvm.fshl.i64(i64 %115, i64 %115, i64 25)
  %164 = xor i64 %162, %163
  %165 = xor i64 %117, %116
  %166 = and i64 %165, %115
  %167 = and i64 %117, %116
  %168 = xor i64 %166, %167
  %169 = add i64 %168, %164
  %170 = add i64 %169, %159
  %171 = add i64 %159, %118
  %172 = getelementptr inbounds nuw i8, ptr %147, i64 8
  %173 = load i64, ptr %172, align 8, !alias.scope !91, !noalias !97, !noundef !100
  %174 = tail call i64 @llvm.fshl.i64(i64 %171, i64 %171, i64 50)
  %175 = tail call i64 @llvm.fshl.i64(i64 %171, i64 %171, i64 46)
  %176 = xor i64 %174, %175
  %177 = tail call i64 @llvm.fshl.i64(i64 %171, i64 %171, i64 23)
  %178 = xor i64 %176, %177
  %179 = xor i64 %114, %112
  %180 = and i64 %171, %179
  %181 = xor i64 %180, %114
  %182 = add i64 %173, %113
  %183 = add i64 %182, %181
  %184 = add i64 %183, %178
  %185 = tail call i64 @llvm.fshl.i64(i64 %170, i64 %170, i64 36)
  %186 = tail call i64 @llvm.fshl.i64(i64 %170, i64 %170, i64 30)
  %187 = xor i64 %185, %186
  %188 = tail call i64 @llvm.fshl.i64(i64 %170, i64 %170, i64 25)
  %189 = xor i64 %187, %188
  %190 = xor i64 %116, %115
  %191 = and i64 %170, %190
  %192 = and i64 %116, %115
  %193 = xor i64 %191, %192
  %194 = add i64 %189, %193
  %195 = add i64 %194, %184
  %196 = add i64 %184, %117
  %197 = getelementptr inbounds nuw [16 x i8], ptr %4, i64 %102
  store <2 x i64> %145, ptr %197, align 16, !alias.scope !91, !noalias !97
  %198 = add i64 %101, 2
  %199 = icmp eq i64 %119, 8
  br i1 %199, label %78, label %100

.preheader4:                                      ; preds = %78, %.preheader4
  %200 = phi i64 [ %290, %.preheader4 ], [ 64, %78 ]
  %201 = phi i64 [ %234, %.preheader4 ], [ %114, %78 ]
  %202 = phi i64 [ %316, %.preheader4 ], [ %196, %78 ]
  %203 = phi i64 [ %261, %.preheader4 ], [ %112, %78 ]
  %204 = phi i64 [ %288, %.preheader4 ], [ %171, %78 ]
  %205 = phi i64 [ %315, %.preheader4 ], [ %195, %78 ]
  %206 = phi i64 [ %287, %.preheader4 ], [ %170, %78 ]
  %207 = phi i64 [ %260, %.preheader4 ], [ %115, %78 ]
  %208 = phi i64 [ %233, %.preheader4 ], [ %116, %78 ]
  %209 = and i64 %200, 12
  %210 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %209
  %211 = load i64, ptr %210, align 16, !alias.scope !101, !noalias !104, !noundef !100
  %212 = tail call i64 @llvm.fshl.i64(i64 %202, i64 %202, i64 50)
  %213 = tail call i64 @llvm.fshl.i64(i64 %202, i64 %202, i64 46)
  %214 = xor i64 %212, %213
  %215 = tail call i64 @llvm.fshl.i64(i64 %202, i64 %202, i64 23)
  %216 = xor i64 %214, %215
  %217 = xor i64 %204, %203
  %218 = and i64 %217, %202
  %219 = xor i64 %218, %203
  %220 = add i64 %216, %201
  %221 = add i64 %220, %219
  %222 = add i64 %221, %211
  %223 = tail call i64 @llvm.fshl.i64(i64 %205, i64 %205, i64 36)
  %224 = tail call i64 @llvm.fshl.i64(i64 %205, i64 %205, i64 30)
  %225 = xor i64 %223, %224
  %226 = tail call i64 @llvm.fshl.i64(i64 %205, i64 %205, i64 25)
  %227 = xor i64 %225, %226
  %228 = xor i64 %207, %206
  %229 = and i64 %228, %205
  %230 = and i64 %207, %206
  %231 = xor i64 %229, %230
  %232 = add i64 %231, %227
  %233 = add i64 %232, %222
  %234 = add i64 %222, %208
  %235 = and i64 %200, 12
  %236 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %235
  %237 = getelementptr inbounds nuw i8, ptr %236, i64 8
  %238 = load i64, ptr %237, align 8, !alias.scope !101, !noalias !104, !noundef !100
  %239 = tail call i64 @llvm.fshl.i64(i64 %234, i64 %234, i64 50)
  %240 = tail call i64 @llvm.fshl.i64(i64 %234, i64 %234, i64 46)
  %241 = xor i64 %239, %240
  %242 = tail call i64 @llvm.fshl.i64(i64 %234, i64 %234, i64 23)
  %243 = xor i64 %241, %242
  %244 = xor i64 %202, %204
  %245 = and i64 %244, %234
  %246 = xor i64 %245, %204
  %247 = add i64 %243, %203
  %248 = add i64 %247, %246
  %249 = add i64 %248, %238
  %250 = tail call i64 @llvm.fshl.i64(i64 %233, i64 %233, i64 36)
  %251 = tail call i64 @llvm.fshl.i64(i64 %233, i64 %233, i64 30)
  %252 = xor i64 %250, %251
  %253 = tail call i64 @llvm.fshl.i64(i64 %233, i64 %233, i64 25)
  %254 = xor i64 %252, %253
  %255 = xor i64 %206, %205
  %256 = and i64 %255, %233
  %257 = and i64 %206, %205
  %258 = xor i64 %256, %257
  %259 = add i64 %258, %254
  %260 = add i64 %259, %249
  %261 = add i64 %249, %207
  %262 = and i64 %200, 12
  %263 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %262
  %264 = getelementptr inbounds nuw i8, ptr %263, i64 16
  %265 = load i64, ptr %264, align 16, !alias.scope !101, !noalias !104, !noundef !100
  %266 = tail call i64 @llvm.fshl.i64(i64 %261, i64 %261, i64 50)
  %267 = tail call i64 @llvm.fshl.i64(i64 %261, i64 %261, i64 46)
  %268 = xor i64 %266, %267
  %269 = tail call i64 @llvm.fshl.i64(i64 %261, i64 %261, i64 23)
  %270 = xor i64 %268, %269
  %271 = xor i64 %234, %202
  %272 = and i64 %271, %261
  %273 = xor i64 %272, %202
  %274 = add i64 %270, %204
  %275 = add i64 %274, %273
  %276 = add i64 %275, %265
  %277 = tail call i64 @llvm.fshl.i64(i64 %260, i64 %260, i64 36)
  %278 = tail call i64 @llvm.fshl.i64(i64 %260, i64 %260, i64 30)
  %279 = xor i64 %277, %278
  %280 = tail call i64 @llvm.fshl.i64(i64 %260, i64 %260, i64 25)
  %281 = xor i64 %279, %280
  %282 = xor i64 %205, %233
  %283 = and i64 %282, %260
  %284 = and i64 %205, %233
  %285 = xor i64 %283, %284
  %286 = add i64 %285, %281
  %287 = add i64 %286, %276
  %288 = add i64 %276, %206
  %289 = and i64 %200, 12
  %290 = add nuw nsw i64 %200, 4
  %291 = getelementptr inbounds nuw [8 x i8], ptr %4, i64 %289
  %292 = getelementptr inbounds nuw i8, ptr %291, i64 24
  %293 = load i64, ptr %292, align 8, !alias.scope !101, !noalias !104, !noundef !100
  %294 = tail call i64 @llvm.fshl.i64(i64 %288, i64 %288, i64 50)
  %295 = tail call i64 @llvm.fshl.i64(i64 %288, i64 %288, i64 46)
  %296 = xor i64 %294, %295
  %297 = tail call i64 @llvm.fshl.i64(i64 %288, i64 %288, i64 23)
  %298 = xor i64 %296, %297
  %299 = xor i64 %261, %234
  %300 = and i64 %299, %288
  %301 = xor i64 %300, %234
  %302 = add i64 %298, %202
  %303 = add i64 %302, %301
  %304 = add i64 %303, %293
  %305 = tail call i64 @llvm.fshl.i64(i64 %287, i64 %287, i64 36)
  %306 = tail call i64 @llvm.fshl.i64(i64 %287, i64 %287, i64 30)
  %307 = xor i64 %305, %306
  %308 = tail call i64 @llvm.fshl.i64(i64 %287, i64 %287, i64 25)
  %309 = xor i64 %307, %308
  %310 = xor i64 %233, %260
  %311 = and i64 %310, %287
  %312 = and i64 %233, %260
  %313 = xor i64 %311, %312
  %314 = add i64 %313, %309
  %315 = add i64 %314, %304
  %316 = add i64 %304, %205
  %317 = icmp eq i64 %290, 80
  br i1 %317, label %318, label %.preheader4

318:                                              ; preds = %.preheader4
  %319 = add i64 %315, %17
  store i64 %319, ptr %0, align 8, !alias.scope !106, !noalias !109
  %320 = add i64 %287, %19
  store i64 %320, ptr %18, align 8, !alias.scope !106, !noalias !109
  %321 = add i64 %260, %21
  store i64 %321, ptr %20, align 8, !alias.scope !106, !noalias !109
  %322 = add i64 %233, %23
  store i64 %322, ptr %22, align 8, !alias.scope !106, !noalias !109
  %323 = add i64 %316, %25
  store i64 %323, ptr %24, align 8, !alias.scope !106, !noalias !109
  %324 = add i64 %288, %27
  store i64 %324, ptr %26, align 8, !alias.scope !106, !noalias !109
  %325 = add i64 %261, %29
  store i64 %325, ptr %28, align 8, !alias.scope !106, !noalias !109
  %326 = add i64 %234, %31
  store i64 %326, ptr %30, align 8, !alias.scope !106, !noalias !109
  call void @llvm.lifetime.end.p0(ptr nonnull %4), !noalias !57
  br label %9

327:                                              ; preds = %9
  %328 = getelementptr inbounds nuw i8, ptr %6, i64 16
  %329 = getelementptr inbounds nuw i8, ptr %5, i64 16
  %330 = getelementptr inbounds nuw i8, ptr %6, i64 32
  %331 = getelementptr inbounds nuw i8, ptr %5, i64 32
  %332 = getelementptr inbounds nuw i8, ptr %6, i64 48
  %333 = getelementptr inbounds nuw i8, ptr %5, i64 48
  %334 = getelementptr inbounds nuw i8, ptr %6, i64 64
  %335 = getelementptr inbounds nuw i8, ptr %5, i64 64
  %336 = getelementptr inbounds nuw i8, ptr %6, i64 80
  %337 = getelementptr inbounds nuw i8, ptr %5, i64 80
  %338 = getelementptr inbounds nuw i8, ptr %6, i64 96
  %339 = getelementptr inbounds nuw i8, ptr %5, i64 96
  %340 = getelementptr inbounds nuw i8, ptr %6, i64 112
  %341 = getelementptr inbounds nuw i8, ptr %5, i64 112
  %342 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %343 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %344 = getelementptr inbounds nuw i8, ptr %0, i64 24
  %345 = getelementptr inbounds nuw i8, ptr %0, i64 32
  %346 = getelementptr inbounds nuw i8, ptr %0, i64 40
  %347 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %348 = getelementptr inbounds nuw i8, ptr %0, i64 56
  %349 = load i64, ptr %0, align 8, !alias.scope !59, !noalias !64
  %350 = load i64, ptr %342, align 8, !alias.scope !59, !noalias !64
  %351 = load i64, ptr %343, align 8, !alias.scope !59, !noalias !64
  %352 = load i64, ptr %344, align 8, !alias.scope !59, !noalias !64
  %353 = load i64, ptr %345, align 8, !alias.scope !59, !noalias !64
  %354 = load i64, ptr %346, align 8, !alias.scope !59, !noalias !64
  %355 = load i64, ptr %347, align 8, !alias.scope !59, !noalias !64
  %356 = load i64, ptr %348, align 8, !alias.scope !59, !noalias !64
  br label %357

357:                                              ; preds = %826, %327
  %358 = phi i64 [ %356, %327 ], [ %834, %826 ]
  %359 = phi i64 [ %355, %327 ], [ %833, %826 ]
  %360 = phi i64 [ %354, %327 ], [ %832, %826 ]
  %361 = phi i64 [ %353, %327 ], [ %831, %826 ]
  %362 = phi i64 [ %352, %327 ], [ %830, %826 ]
  %363 = phi i64 [ %351, %327 ], [ %829, %826 ]
  %364 = phi i64 [ %350, %327 ], [ %828, %826 ]
  %365 = phi i64 [ %349, %327 ], [ %827, %826 ]
  %366 = phi i64 [ %10, %327 ], [ %835, %826 ]
  %367 = phi i64 [ %14, %327 ], [ %836, %826 ]
  %368 = getelementptr inbounds nuw [128 x i8], ptr %1, i64 %366
  tail call void @llvm.experimental.noalias.scope.decl(metadata !111)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !114)
  %369 = getelementptr inbounds nuw i8, ptr %368, i64 128
  %370 = load <2 x i64>, ptr %369, align 1, !alias.scope !64, !noalias !116
  %371 = load <2 x i64>, ptr %368, align 1, !alias.scope !64, !noalias !120
  %372 = shufflevector <2 x i64> %371, <2 x i64> %370, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %373 = bitcast <4 x i64> %372 to <32 x i8>
  %374 = shufflevector <32 x i8> %373, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %375 = bitcast <32 x i8> %374 to <4 x i64>
  %376 = add <4 x i64> %375, <i64 4794697086780616226, i64 8158064640168781261, i64 4794697086780616226, i64 8158064640168781261>
  %377 = shufflevector <4 x i64> %376, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %377, ptr %6, align 16, !alias.scope !111, !noalias !123
  %378 = shufflevector <4 x i64> %376, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %378, ptr %5, align 16, !alias.scope !114, !noalias !124
  %379 = getelementptr inbounds nuw i8, ptr %368, i64 144
  %380 = load <2 x i64>, ptr %379, align 1, !alias.scope !64, !noalias !125
  %381 = getelementptr inbounds nuw i8, ptr %368, i64 16
  %382 = load <2 x i64>, ptr %381, align 1, !alias.scope !64, !noalias !128
  %383 = shufflevector <2 x i64> %382, <2 x i64> %380, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %384 = bitcast <4 x i64> %383 to <32 x i8>
  %385 = shufflevector <32 x i8> %384, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %386 = bitcast <32 x i8> %385 to <4 x i64>
  %387 = add <4 x i64> %386, <i64 -5349999486874862801, i64 -1606136188198331460, i64 -5349999486874862801, i64 -1606136188198331460>
  %388 = shufflevector <4 x i64> %387, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %388, ptr %328, align 16, !alias.scope !111, !noalias !123
  %389 = shufflevector <4 x i64> %387, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %389, ptr %329, align 16, !alias.scope !114, !noalias !124
  %390 = getelementptr inbounds nuw i8, ptr %368, i64 160
  %391 = load <2 x i64>, ptr %390, align 1, !alias.scope !64, !noalias !131
  %392 = getelementptr inbounds nuw i8, ptr %368, i64 32
  %393 = load <2 x i64>, ptr %392, align 1, !alias.scope !64, !noalias !134
  %394 = shufflevector <2 x i64> %393, <2 x i64> %391, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %395 = bitcast <4 x i64> %394 to <32 x i8>
  %396 = shufflevector <32 x i8> %395, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %397 = bitcast <32 x i8> %396 to <4 x i64>
  %398 = add <4 x i64> %397, <i64 4131703408338449720, i64 6480981068601479193, i64 4131703408338449720, i64 6480981068601479193>
  %399 = shufflevector <4 x i64> %398, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %399, ptr %330, align 16, !alias.scope !111, !noalias !123
  %400 = shufflevector <4 x i64> %398, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %400, ptr %331, align 16, !alias.scope !114, !noalias !124
  %401 = getelementptr inbounds nuw i8, ptr %368, i64 176
  %402 = load <2 x i64>, ptr %401, align 1, !alias.scope !64, !noalias !137
  %403 = getelementptr inbounds nuw i8, ptr %368, i64 48
  %404 = load <2 x i64>, ptr %403, align 1, !alias.scope !64, !noalias !140
  %405 = shufflevector <2 x i64> %404, <2 x i64> %402, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %406 = bitcast <4 x i64> %405 to <32 x i8>
  %407 = shufflevector <32 x i8> %406, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %408 = bitcast <32 x i8> %407 to <4 x i64>
  %409 = add <4 x i64> %408, <i64 -7908458776815382629, i64 -6116909921290321640, i64 -7908458776815382629, i64 -6116909921290321640>
  %410 = shufflevector <4 x i64> %409, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %410, ptr %332, align 16, !alias.scope !111, !noalias !123
  %411 = shufflevector <4 x i64> %409, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %411, ptr %333, align 16, !alias.scope !114, !noalias !124
  %412 = getelementptr inbounds nuw i8, ptr %368, i64 192
  %413 = load <2 x i64>, ptr %412, align 1, !alias.scope !64, !noalias !143
  %414 = getelementptr inbounds nuw i8, ptr %368, i64 64
  %415 = load <2 x i64>, ptr %414, align 1, !alias.scope !64, !noalias !146
  %416 = shufflevector <2 x i64> %415, <2 x i64> %413, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %417 = bitcast <4 x i64> %416 to <32 x i8>
  %418 = shufflevector <32 x i8> %417, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %419 = bitcast <32 x i8> %418 to <4 x i64>
  %420 = add <4 x i64> %419, <i64 -2880145864133508542, i64 1334009975649890238, i64 -2880145864133508542, i64 1334009975649890238>
  %421 = shufflevector <4 x i64> %420, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %421, ptr %334, align 16, !alias.scope !111, !noalias !123
  %422 = shufflevector <4 x i64> %420, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %422, ptr %335, align 16, !alias.scope !114, !noalias !124
  %423 = getelementptr inbounds nuw i8, ptr %368, i64 208
  %424 = load <2 x i64>, ptr %423, align 1, !alias.scope !64, !noalias !149
  %425 = getelementptr inbounds nuw i8, ptr %368, i64 80
  %426 = load <2 x i64>, ptr %425, align 1, !alias.scope !64, !noalias !152
  %427 = shufflevector <2 x i64> %426, <2 x i64> %424, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %428 = bitcast <4 x i64> %427 to <32 x i8>
  %429 = shufflevector <32 x i8> %428, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %430 = bitcast <32 x i8> %429 to <4 x i64>
  %431 = add <4 x i64> %430, <i64 2608012711638119052, i64 6128411473006802146, i64 2608012711638119052, i64 6128411473006802146>
  %432 = shufflevector <4 x i64> %431, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %432, ptr %336, align 16, !alias.scope !111, !noalias !123
  %433 = shufflevector <4 x i64> %431, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %433, ptr %337, align 16, !alias.scope !114, !noalias !124
  %434 = getelementptr inbounds nuw i8, ptr %368, i64 224
  %435 = load <2 x i64>, ptr %434, align 1, !alias.scope !64, !noalias !155
  %436 = getelementptr inbounds nuw i8, ptr %368, i64 96
  %437 = load <2 x i64>, ptr %436, align 1, !alias.scope !64, !noalias !158
  %438 = shufflevector <2 x i64> %437, <2 x i64> %435, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %439 = bitcast <4 x i64> %438 to <32 x i8>
  %440 = shufflevector <32 x i8> %439, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %441 = bitcast <32 x i8> %440 to <4 x i64>
  %442 = add <4 x i64> %441, <i64 8268148722764581231, i64 -9160688886553864527, i64 8268148722764581231, i64 -9160688886553864527>
  %443 = shufflevector <4 x i64> %442, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %443, ptr %338, align 16, !alias.scope !111, !noalias !123
  %444 = shufflevector <4 x i64> %442, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %444, ptr %339, align 16, !alias.scope !114, !noalias !124
  %445 = getelementptr inbounds nuw i8, ptr %368, i64 240
  %446 = load <2 x i64>, ptr %445, align 1, !alias.scope !64, !noalias !161
  %447 = getelementptr inbounds nuw i8, ptr %368, i64 112
  %448 = load <2 x i64>, ptr %447, align 1, !alias.scope !64, !noalias !164
  %449 = shufflevector <2 x i64> %448, <2 x i64> %446, <4 x i32> <i32 0, i32 1, i32 2, i32 3>
  %450 = bitcast <4 x i64> %449 to <32 x i8>
  %451 = shufflevector <32 x i8> %450, <32 x i8> poison, <32 x i32> <i32 7, i32 6, i32 5, i32 4, i32 3, i32 2, i32 1, i32 0, i32 15, i32 14, i32 13, i32 12, i32 11, i32 10, i32 9, i32 8, i32 23, i32 22, i32 21, i32 20, i32 19, i32 18, i32 17, i32 16, i32 31, i32 30, i32 29, i32 28, i32 27, i32 26, i32 25, i32 24>
  %452 = bitcast <32 x i8> %451 to <4 x i64>
  %453 = add <4 x i64> %452, <i64 -7215885187991268811, i64 -4495734319001033068, i64 -7215885187991268811, i64 -4495734319001033068>
  %454 = shufflevector <4 x i64> %453, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  store <2 x i64> %454, ptr %340, align 16, !alias.scope !111, !noalias !123
  %455 = shufflevector <4 x i64> %453, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  store <2 x i64> %455, ptr %341, align 16, !alias.scope !114, !noalias !124
  tail call void @llvm.experimental.noalias.scope.decl(metadata !167)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !170)
  br label %459

456:                                              ; preds = %480
  %457 = add nuw nsw i64 %461, 1
  %458 = icmp eq i64 %457, 5
  br i1 %458, label %.preheader, label %459

459:                                              ; preds = %456, %357
  %460 = phi i64 [ 16, %357 ], [ %582, %456 ]
  %461 = phi i64 [ 1, %357 ], [ %457, %456 ]
  %462 = phi <4 x i64> [ %386, %357 ], [ %488, %456 ]
  %463 = phi <4 x i64> [ %375, %357 ], [ %483, %456 ]
  %464 = phi <4 x i64> [ %430, %357 ], [ %490, %456 ]
  %465 = phi <4 x i64> [ %419, %357 ], [ %485, %456 ]
  %466 = phi <4 x i64> [ %452, %357 ], [ %525, %456 ]
  %467 = phi <4 x i64> [ %397, %357 ], [ %489, %456 ]
  %468 = phi <4 x i64> [ %408, %357 ], [ %486, %456 ]
  %469 = phi <4 x i64> [ %441, %357 ], [ %487, %456 ]
  %470 = phi i64 [ %358, %357 ], [ %494, %456 ]
  %471 = phi i64 [ %361, %357 ], [ %577, %456 ]
  %472 = phi i64 [ %359, %357 ], [ %492, %456 ]
  %473 = phi i64 [ %360, %357 ], [ %552, %456 ]
  %474 = phi i64 [ %365, %357 ], [ %576, %456 ]
  %475 = phi i64 [ %364, %357 ], [ %551, %456 ]
  %476 = phi i64 [ %363, %357 ], [ %495, %456 ]
  %477 = phi i64 [ %362, %357 ], [ %496, %456 ]
  %478 = shl nuw nsw i64 %461, 7
  %479 = getelementptr inbounds nuw i8, ptr %5, i64 %478
  br label %480

480:                                              ; preds = %480, %459
  %481 = phi i64 [ %460, %459 ], [ %582, %480 ]
  %482 = phi i64 [ 0, %459 ], [ %499, %480 ]
  %483 = phi <4 x i64> [ %462, %459 ], [ %488, %480 ]
  %484 = phi <4 x i64> [ %463, %459 ], [ %483, %480 ]
  %485 = phi <4 x i64> [ %464, %459 ], [ %490, %480 ]
  %486 = phi <4 x i64> [ %465, %459 ], [ %485, %480 ]
  %487 = phi <4 x i64> [ %466, %459 ], [ %525, %480 ]
  %488 = phi <4 x i64> [ %467, %459 ], [ %489, %480 ]
  %489 = phi <4 x i64> [ %468, %459 ], [ %486, %480 ]
  %490 = phi <4 x i64> [ %469, %459 ], [ %487, %480 ]
  %491 = phi i64 [ %470, %459 ], [ %494, %480 ]
  %492 = phi i64 [ %471, %459 ], [ %577, %480 ]
  %493 = phi i64 [ %472, %459 ], [ %492, %480 ]
  %494 = phi i64 [ %473, %459 ], [ %552, %480 ]
  %495 = phi i64 [ %474, %459 ], [ %576, %480 ]
  %496 = phi i64 [ %475, %459 ], [ %551, %480 ]
  %497 = phi i64 [ %476, %459 ], [ %495, %480 ]
  %498 = phi i64 [ %477, %459 ], [ %496, %480 ]
  %499 = add nuw nsw i64 %482, 1
  %500 = getelementptr inbounds nuw [8 x i8], ptr @anon.deeb35d3f195dc5f9a0c48b58e3e09c8.0, i64 %481
  %501 = load <2 x i64>, ptr %500, align 8, !noalias !172
  %502 = shufflevector <2 x i64> %501, <2 x i64> poison, <4 x i32> <i32 0, i32 1, i32 0, i32 1>
  %503 = shufflevector <4 x i64> %484, <4 x i64> %483, <4 x i32> <i32 1, i32 4, i32 3, i32 6>
  %504 = shufflevector <4 x i64> %486, <4 x i64> %485, <4 x i32> <i32 1, i32 4, i32 3, i32 6>
  %505 = lshr <4 x i64> %503, splat (i64 1)
  %506 = add <4 x i64> %484, %504
  %507 = lshr <4 x i64> %503, splat (i64 7)
  %508 = shl <4 x i64> %503, splat (i64 56)
  %509 = xor <4 x i64> %505, %507
  %510 = lshr <4 x i64> %503, splat (i64 8)
  %511 = xor <4 x i64> %509, %508
  %512 = shl <4 x i64> %503, splat (i64 63)
  %513 = xor <4 x i64> %511, %510
  %514 = xor <4 x i64> %513, %512
  %515 = lshr <4 x i64> %487, splat (i64 6)
  %516 = shl <4 x i64> %487, splat (i64 3)
  %517 = add <4 x i64> %506, %514
  %518 = lshr <4 x i64> %487, splat (i64 19)
  %519 = xor <4 x i64> %516, %515
  %520 = shl <4 x i64> %487, splat (i64 45)
  %521 = xor <4 x i64> %519, %518
  %522 = lshr <4 x i64> %487, splat (i64 61)
  %523 = xor <4 x i64> %521, %520
  %524 = xor <4 x i64> %523, %522
  %525 = add <4 x i64> %517, %524
  %526 = add <4 x i64> %525, %502
  %527 = shl nuw nsw i64 %482, 4
  %528 = getelementptr inbounds nuw i8, ptr %6, i64 %527
  %529 = load i64, ptr %528, align 16, !alias.scope !167, !noalias !175, !noundef !100
  %530 = tail call i64 @llvm.fshl.i64(i64 %492, i64 %492, i64 50)
  %531 = tail call i64 @llvm.fshl.i64(i64 %492, i64 %492, i64 46)
  %532 = xor i64 %530, %531
  %533 = tail call i64 @llvm.fshl.i64(i64 %492, i64 %492, i64 23)
  %534 = xor i64 %532, %533
  %535 = xor i64 %494, %493
  %536 = and i64 %535, %492
  %537 = xor i64 %536, %493
  %538 = add i64 %534, %491
  %539 = add i64 %538, %537
  %540 = add i64 %539, %529
  %541 = tail call i64 @llvm.fshl.i64(i64 %495, i64 %495, i64 36)
  %542 = tail call i64 @llvm.fshl.i64(i64 %495, i64 %495, i64 30)
  %543 = xor i64 %541, %542
  %544 = tail call i64 @llvm.fshl.i64(i64 %495, i64 %495, i64 25)
  %545 = xor i64 %543, %544
  %546 = xor i64 %497, %496
  %547 = and i64 %546, %495
  %548 = and i64 %497, %496
  %549 = xor i64 %547, %548
  %550 = add i64 %549, %545
  %551 = add i64 %550, %540
  %552 = add i64 %540, %498
  %553 = getelementptr inbounds nuw i8, ptr %528, i64 8
  %554 = load i64, ptr %553, align 8, !alias.scope !167, !noalias !175, !noundef !100
  %555 = tail call i64 @llvm.fshl.i64(i64 %552, i64 %552, i64 50)
  %556 = tail call i64 @llvm.fshl.i64(i64 %552, i64 %552, i64 46)
  %557 = xor i64 %555, %556
  %558 = tail call i64 @llvm.fshl.i64(i64 %552, i64 %552, i64 23)
  %559 = xor i64 %557, %558
  %560 = xor i64 %494, %492
  %561 = and i64 %552, %560
  %562 = xor i64 %561, %494
  %563 = add i64 %554, %493
  %564 = add i64 %563, %562
  %565 = add i64 %564, %559
  %566 = tail call i64 @llvm.fshl.i64(i64 %551, i64 %551, i64 36)
  %567 = tail call i64 @llvm.fshl.i64(i64 %551, i64 %551, i64 30)
  %568 = xor i64 %566, %567
  %569 = tail call i64 @llvm.fshl.i64(i64 %551, i64 %551, i64 25)
  %570 = xor i64 %568, %569
  %571 = xor i64 %496, %495
  %572 = and i64 %551, %571
  %573 = and i64 %496, %495
  %574 = xor i64 %572, %573
  %575 = add i64 %570, %574
  %576 = add i64 %575, %565
  %577 = add i64 %565, %497
  %578 = shufflevector <4 x i64> %526, <4 x i64> poison, <2 x i32> <i32 0, i32 1>
  %579 = getelementptr inbounds nuw [16 x i8], ptr %6, i64 %482
  store <2 x i64> %578, ptr %579, align 16, !alias.scope !167, !noalias !175
  %580 = shufflevector <4 x i64> %526, <4 x i64> poison, <2 x i32> <i32 2, i32 3>
  %581 = getelementptr inbounds nuw [16 x i8], ptr %479, i64 %482
  store <2 x i64> %580, ptr %581, align 16, !alias.scope !170, !noalias !178
  %582 = add i64 %481, 2
  %583 = icmp eq i64 %499, 8
  br i1 %583, label %456, label %480

.preheader:                                       ; preds = %456, %.preheader
  %584 = phi i64 [ %674, %.preheader ], [ 64, %456 ]
  %585 = phi i64 [ %618, %.preheader ], [ %494, %456 ]
  %586 = phi i64 [ %700, %.preheader ], [ %577, %456 ]
  %587 = phi i64 [ %645, %.preheader ], [ %492, %456 ]
  %588 = phi i64 [ %672, %.preheader ], [ %552, %456 ]
  %589 = phi i64 [ %699, %.preheader ], [ %576, %456 ]
  %590 = phi i64 [ %671, %.preheader ], [ %551, %456 ]
  %591 = phi i64 [ %644, %.preheader ], [ %495, %456 ]
  %592 = phi i64 [ %617, %.preheader ], [ %496, %456 ]
  %593 = and i64 %584, 12
  %594 = getelementptr inbounds nuw [8 x i8], ptr %6, i64 %593
  %595 = load i64, ptr %594, align 16, !alias.scope !179, !noalias !182, !noundef !100
  %596 = tail call i64 @llvm.fshl.i64(i64 %586, i64 %586, i64 50)
  %597 = tail call i64 @llvm.fshl.i64(i64 %586, i64 %586, i64 46)
  %598 = xor i64 %596, %597
  %599 = tail call i64 @llvm.fshl.i64(i64 %586, i64 %586, i64 23)
  %600 = xor i64 %598, %599
  %601 = xor i64 %588, %587
  %602 = and i64 %601, %586
  %603 = xor i64 %602, %587
  %604 = add i64 %600, %585
  %605 = add i64 %604, %603
  %606 = add i64 %605, %595
  %607 = tail call i64 @llvm.fshl.i64(i64 %589, i64 %589, i64 36)
  %608 = tail call i64 @llvm.fshl.i64(i64 %589, i64 %589, i64 30)
  %609 = xor i64 %607, %608
  %610 = tail call i64 @llvm.fshl.i64(i64 %589, i64 %589, i64 25)
  %611 = xor i64 %609, %610
  %612 = xor i64 %591, %590
  %613 = and i64 %612, %589
  %614 = and i64 %591, %590
  %615 = xor i64 %613, %614
  %616 = add i64 %615, %611
  %617 = add i64 %616, %606
  %618 = add i64 %606, %592
  %619 = and i64 %584, 12
  %620 = getelementptr inbounds nuw [8 x i8], ptr %6, i64 %619
  %621 = getelementptr inbounds nuw i8, ptr %620, i64 8
  %622 = load i64, ptr %621, align 8, !alias.scope !179, !noalias !182, !noundef !100
  %623 = tail call i64 @llvm.fshl.i64(i64 %618, i64 %618, i64 50)
  %624 = tail call i64 @llvm.fshl.i64(i64 %618, i64 %618, i64 46)
  %625 = xor i64 %623, %624
  %626 = tail call i64 @llvm.fshl.i64(i64 %618, i64 %618, i64 23)
  %627 = xor i64 %625, %626
  %628 = xor i64 %586, %588
  %629 = and i64 %628, %618
  %630 = xor i64 %629, %588
  %631 = add i64 %627, %587
  %632 = add i64 %631, %630
  %633 = add i64 %632, %622
  %634 = tail call i64 @llvm.fshl.i64(i64 %617, i64 %617, i64 36)
  %635 = tail call i64 @llvm.fshl.i64(i64 %617, i64 %617, i64 30)
  %636 = xor i64 %634, %635
  %637 = tail call i64 @llvm.fshl.i64(i64 %617, i64 %617, i64 25)
  %638 = xor i64 %636, %637
  %639 = xor i64 %590, %589
  %640 = and i64 %639, %617
  %641 = and i64 %590, %589
  %642 = xor i64 %640, %641
  %643 = add i64 %642, %638
  %644 = add i64 %643, %633
  %645 = add i64 %633, %591
  %646 = and i64 %584, 12
  %647 = getelementptr inbounds nuw [8 x i8], ptr %6, i64 %646
  %648 = getelementptr inbounds nuw i8, ptr %647, i64 16
  %649 = load i64, ptr %648, align 16, !alias.scope !179, !noalias !182, !noundef !100
  %650 = tail call i64 @llvm.fshl.i64(i64 %645, i64 %645, i64 50)
  %651 = tail call i64 @llvm.fshl.i64(i64 %645, i64 %645, i64 46)
  %652 = xor i64 %650, %651
  %653 = tail call i64 @llvm.fshl.i64(i64 %645, i64 %645, i64 23)
  %654 = xor i64 %652, %653
  %655 = xor i64 %618, %586
  %656 = and i64 %655, %645
  %657 = xor i64 %656, %586
  %658 = add i64 %654, %588
  %659 = add i64 %658, %657
  %660 = add i64 %659, %649
  %661 = tail call i64 @llvm.fshl.i64(i64 %644, i64 %644, i64 36)
  %662 = tail call i64 @llvm.fshl.i64(i64 %644, i64 %644, i64 30)
  %663 = xor i64 %661, %662
  %664 = tail call i64 @llvm.fshl.i64(i64 %644, i64 %644, i64 25)
  %665 = xor i64 %663, %664
  %666 = xor i64 %589, %617
  %667 = and i64 %666, %644
  %668 = and i64 %589, %617
  %669 = xor i64 %667, %668
  %670 = add i64 %669, %665
  %671 = add i64 %670, %660
  %672 = add i64 %660, %590
  %673 = and i64 %584, 12
  %674 = add nuw nsw i64 %584, 4
  %675 = getelementptr inbounds nuw [8 x i8], ptr %6, i64 %673
  %676 = getelementptr inbounds nuw i8, ptr %675, i64 24
  %677 = load i64, ptr %676, align 8, !alias.scope !179, !noalias !182, !noundef !100
  %678 = tail call i64 @llvm.fshl.i64(i64 %672, i64 %672, i64 50)
  %679 = tail call i64 @llvm.fshl.i64(i64 %672, i64 %672, i64 46)
  %680 = xor i64 %678, %679
  %681 = tail call i64 @llvm.fshl.i64(i64 %672, i64 %672, i64 23)
  %682 = xor i64 %680, %681
  %683 = xor i64 %645, %618
  %684 = and i64 %683, %672
  %685 = xor i64 %684, %618
  %686 = add i64 %682, %586
  %687 = add i64 %686, %685
  %688 = add i64 %687, %677
  %689 = tail call i64 @llvm.fshl.i64(i64 %671, i64 %671, i64 36)
  %690 = tail call i64 @llvm.fshl.i64(i64 %671, i64 %671, i64 30)
  %691 = xor i64 %689, %690
  %692 = tail call i64 @llvm.fshl.i64(i64 %671, i64 %671, i64 25)
  %693 = xor i64 %691, %692
  %694 = xor i64 %617, %644
  %695 = and i64 %694, %671
  %696 = and i64 %617, %644
  %697 = xor i64 %695, %696
  %698 = add i64 %697, %693
  %699 = add i64 %698, %688
  %700 = add i64 %688, %589
  %701 = icmp eq i64 %674, 80
  br i1 %701, label %702, label %.preheader

702:                                              ; preds = %.preheader
  %703 = add i64 %699, %365
  %704 = add i64 %671, %364
  %705 = add i64 %644, %363
  %706 = add i64 %617, %362
  %707 = add i64 %700, %361
  %708 = add i64 %672, %360
  %709 = add i64 %645, %359
  %710 = add i64 %618, %358
  br label %711

711:                                              ; preds = %711, %702
  %712 = phi i64 [ 0, %702 ], [ %800, %711 ]
  %713 = phi i64 [ %710, %702 ], [ %745, %711 ]
  %714 = phi i64 [ %707, %702 ], [ %824, %711 ]
  %715 = phi i64 [ %709, %702 ], [ %771, %711 ]
  %716 = phi i64 [ %708, %702 ], [ %797, %711 ]
  %717 = phi i64 [ %703, %702 ], [ %823, %711 ]
  %718 = phi i64 [ %704, %702 ], [ %796, %711 ]
  %719 = phi i64 [ %705, %702 ], [ %770, %711 ]
  %720 = phi i64 [ %706, %702 ], [ %744, %711 ]
  %721 = getelementptr inbounds nuw i8, ptr %5, i64 %712
  %722 = load i64, ptr %721, align 16, !alias.scope !184, !noalias !187, !noundef !100
  %723 = tail call i64 @llvm.fshl.i64(i64 %714, i64 %714, i64 50)
  %724 = tail call i64 @llvm.fshl.i64(i64 %714, i64 %714, i64 46)
  %725 = xor i64 %723, %724
  %726 = tail call i64 @llvm.fshl.i64(i64 %714, i64 %714, i64 23)
  %727 = xor i64 %725, %726
  %728 = xor i64 %716, %715
  %729 = and i64 %728, %714
  %730 = xor i64 %729, %715
  %731 = add i64 %727, %713
  %732 = add i64 %731, %730
  %733 = add i64 %732, %722
  %734 = tail call i64 @llvm.fshl.i64(i64 %717, i64 %717, i64 36)
  %735 = tail call i64 @llvm.fshl.i64(i64 %717, i64 %717, i64 30)
  %736 = xor i64 %734, %735
  %737 = tail call i64 @llvm.fshl.i64(i64 %717, i64 %717, i64 25)
  %738 = xor i64 %736, %737
  %739 = xor i64 %719, %718
  %740 = and i64 %739, %717
  %741 = and i64 %719, %718
  %742 = xor i64 %740, %741
  %743 = add i64 %742, %738
  %744 = add i64 %743, %733
  %745 = add i64 %733, %720
  %746 = getelementptr inbounds nuw i8, ptr %5, i64 %712
  %747 = getelementptr inbounds nuw i8, ptr %746, i64 8
  %748 = load i64, ptr %747, align 8, !alias.scope !184, !noalias !187, !noundef !100
  %749 = tail call i64 @llvm.fshl.i64(i64 %745, i64 %745, i64 50)
  %750 = tail call i64 @llvm.fshl.i64(i64 %745, i64 %745, i64 46)
  %751 = xor i64 %749, %750
  %752 = tail call i64 @llvm.fshl.i64(i64 %745, i64 %745, i64 23)
  %753 = xor i64 %751, %752
  %754 = xor i64 %714, %716
  %755 = and i64 %754, %745
  %756 = xor i64 %755, %716
  %757 = add i64 %753, %715
  %758 = add i64 %757, %756
  %759 = add i64 %758, %748
  %760 = tail call i64 @llvm.fshl.i64(i64 %744, i64 %744, i64 36)
  %761 = tail call i64 @llvm.fshl.i64(i64 %744, i64 %744, i64 30)
  %762 = xor i64 %760, %761
  %763 = tail call i64 @llvm.fshl.i64(i64 %744, i64 %744, i64 25)
  %764 = xor i64 %762, %763
  %765 = xor i64 %718, %717
  %766 = and i64 %765, %744
  %767 = and i64 %718, %717
  %768 = xor i64 %766, %767
  %769 = add i64 %768, %764
  %770 = add i64 %769, %759
  %771 = add i64 %759, %719
  %772 = getelementptr inbounds nuw i8, ptr %5, i64 %712
  %773 = getelementptr inbounds nuw i8, ptr %772, i64 16
  %774 = load i64, ptr %773, align 16, !alias.scope !184, !noalias !187, !noundef !100
  %775 = tail call i64 @llvm.fshl.i64(i64 %771, i64 %771, i64 50)
  %776 = tail call i64 @llvm.fshl.i64(i64 %771, i64 %771, i64 46)
  %777 = xor i64 %775, %776
  %778 = tail call i64 @llvm.fshl.i64(i64 %771, i64 %771, i64 23)
  %779 = xor i64 %777, %778
  %780 = xor i64 %745, %714
  %781 = and i64 %780, %771
  %782 = xor i64 %781, %714
  %783 = add i64 %779, %716
  %784 = add i64 %783, %782
  %785 = add i64 %784, %774
  %786 = tail call i64 @llvm.fshl.i64(i64 %770, i64 %770, i64 36)
  %787 = tail call i64 @llvm.fshl.i64(i64 %770, i64 %770, i64 30)
  %788 = xor i64 %786, %787
  %789 = tail call i64 @llvm.fshl.i64(i64 %770, i64 %770, i64 25)
  %790 = xor i64 %788, %789
  %791 = xor i64 %717, %744
  %792 = and i64 %791, %770
  %793 = and i64 %717, %744
  %794 = xor i64 %792, %793
  %795 = add i64 %794, %790
  %796 = add i64 %795, %785
  %797 = add i64 %785, %718
  %798 = getelementptr inbounds nuw i8, ptr %5, i64 %712
  %799 = getelementptr inbounds nuw i8, ptr %798, i64 24
  %800 = add nuw nsw i64 %712, 32
  %801 = load i64, ptr %799, align 8, !alias.scope !184, !noalias !187, !noundef !100
  %802 = tail call i64 @llvm.fshl.i64(i64 %797, i64 %797, i64 50)
  %803 = tail call i64 @llvm.fshl.i64(i64 %797, i64 %797, i64 46)
  %804 = xor i64 %802, %803
  %805 = tail call i64 @llvm.fshl.i64(i64 %797, i64 %797, i64 23)
  %806 = xor i64 %804, %805
  %807 = xor i64 %771, %745
  %808 = and i64 %807, %797
  %809 = xor i64 %808, %745
  %810 = add i64 %806, %714
  %811 = add i64 %810, %809
  %812 = add i64 %811, %801
  %813 = tail call i64 @llvm.fshl.i64(i64 %796, i64 %796, i64 36)
  %814 = tail call i64 @llvm.fshl.i64(i64 %796, i64 %796, i64 30)
  %815 = xor i64 %813, %814
  %816 = tail call i64 @llvm.fshl.i64(i64 %796, i64 %796, i64 25)
  %817 = xor i64 %815, %816
  %818 = xor i64 %744, %770
  %819 = and i64 %818, %796
  %820 = and i64 %744, %770
  %821 = xor i64 %819, %820
  %822 = add i64 %821, %817
  %823 = add i64 %822, %812
  %824 = add i64 %812, %717
  %825 = icmp eq i64 %800, 640
  br i1 %825, label %826, label %711

826:                                              ; preds = %711
  %827 = add i64 %823, %703
  %828 = add i64 %796, %704
  %829 = add i64 %770, %705
  %830 = add i64 %744, %706
  %831 = add i64 %824, %707
  %832 = add i64 %797, %708
  %833 = add i64 %771, %709
  %834 = add i64 %745, %710
  %835 = add i64 %366, 2
  %836 = add i64 %367, -1
  %837 = icmp eq i64 %836, 0
  br i1 %837, label %838, label %357

838:                                              ; preds = %826
  store i64 %827, ptr %0, align 8, !alias.scope !59, !noalias !64
  store i64 %828, ptr %342, align 8, !alias.scope !59, !noalias !64
  store i64 %829, ptr %343, align 8, !alias.scope !59, !noalias !64
  store i64 %830, ptr %344, align 8, !alias.scope !59, !noalias !64
  store i64 %831, ptr %345, align 8, !alias.scope !59, !noalias !64
  store i64 %832, ptr %346, align 8, !alias.scope !59, !noalias !64
  store i64 %833, ptr %347, align 8, !alias.scope !59, !noalias !64
  store i64 %834, ptr %348, align 8, !alias.scope !59, !noalias !64
  br label %839

839:                                              ; preds = %838, %9
  call void @llvm.lifetime.end.p0(ptr nonnull %5), !noalias !53
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !53
  ret void
}

; Function Attrs: nonlazybind
declare i32 @rust_eh_personality(...) unnamed_addr #1

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(ptr captures(none)) #2

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(ptr captures(none)) #2

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(none)
declare <4 x i32> @llvm.x86.sha256msg1(<4 x i32>, <4 x i32>) #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(none)
declare <4 x i32> @llvm.x86.sha256msg2(<4 x i32>, <4 x i32>) #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(none)
declare <4 x i32> @llvm.x86.sha256rnds2(<4 x i32>, <4 x i32>, <4 x i32>) #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite)
declare void @llvm.experimental.noalias.scope.decl(metadata) #4

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #5

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.fshl.i64(i64, i64, i64) #5

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr writeonly captures(none), i8, i64, i1 immarg) #6

attributes #0 = { nofree norecurse nosync nounwind nonlazybind memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable "probe-stack"="inline-asm" "target-cpu"="znver5" "target-features"="+prfchw,-cldemote,+avx,+aes,+sahf,+pclmul,-xop,+crc32,-amx-fp8,+xsaves,-avx512fp16,-usermsr,-sm4,-egpr,+sse4.1,-avx10.1,+avx512ifma,+xsave,+sse4.2,-tsxldtrk,-sm3,-ptwrite,-widekl,-movrs,+invpcid,+64bit,+xsavec,+avx512vpopcntdq,+cmov,+avx512vp2intersect,-avxvnniint8,+avx512cd,+movbe,-ccmp,-amx-int8,-kl,-avx512bmm,-sha512,+avxvnni,-rtm,+adx,+avx2,-hreset,+movdiri,-serialize,+vpclmulqdq,+avx512vl,-uintr,-cf,-jmpabs,+clflushopt,-raoint,-cmpccxadd,+bmi,-amx-tile,+sse,+gfni,-avxvnniint16,-amx-fp16,-zu,-ndd,+xsaveopt,+rdrnd,+avx512f,-amx-bf16,+avx512bf16,+avx512vnni,-push2pop2,+cx8,+avx512bw,+sse3,+pku,-nf,-amx-avx512,+fsgsbase,+clzero,+mwaitx,-lwp,+lzcnt,+sha,+movdir64b,-ppx,+wbnoinvd,-enqcmd,-avxneconvert,-tbm,-pconfig,-amx-complex,+ssse3,+cx16,-avx10.2,+bmi2,+fma,+popcnt,-avxifma,+f16c,+avx512bitalg,+rdpru,+clwb,+mmx,+sse2,+rdseed,+avx512vbmi2,+prefetchi,+rdpid,-amx-movrs,-fma4,+avx512vbmi,+shstk,+vaes,-waitpkg,-sgx,+fxsr,+avx512dq,+sse4a" }
attributes #1 = { nonlazybind "target-cpu"="znver5" }
attributes #2 = { nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #3 = { nocallback nofree nosync nounwind willreturn memory(none) }
attributes #4 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite) }
attributes #5 = { nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
attributes #6 = { nocallback nofree nosync nounwind willreturn memory(argmem: write) }

!llvm.module.flags = !{!1, !2, !3}
!llvm.ident = !{!4}

!0 = !{i64 6596768270730610785}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 2, !"RtLibUseGOT", i32 1}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!5 = !{i64 704025864162193572}
!6 = !{!7}
!7 = distinct !{!7, !8, !"_RNvNtCsly5PKRZWayS_4sha26sha2568compress: argument 0"}
!8 = distinct !{!8, !"_RNvNtCsly5PKRZWayS_4sha26sha2568compress"}
!9 = !{!10}
!10 = distinct !{!10, !8, !"_RNvNtCsly5PKRZWayS_4sha26sha2568compress: argument 1"}
!11 = !{!12}
!12 = distinct !{!12, !13, !"_RNvNtNtCsly5PKRZWayS_4sha26sha2567x86_sha8compress: argument 0"}
!13 = distinct !{!13, !"_RNvNtNtCsly5PKRZWayS_4sha26sha2567x86_sha8compress"}
!14 = !{!15}
!15 = distinct !{!15, !13, !"_RNvNtNtCsly5PKRZWayS_4sha26sha2567x86_sha8compress: argument 1"}
!16 = !{!12, !7}
!17 = !{!18, !15, !10}
!18 = distinct !{!18, !19, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!19 = distinct !{!19, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!20 = !{!21, !15, !10}
!21 = distinct !{!21, !22, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!22 = distinct !{!22, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!23 = !{!15, !10}
!24 = !{!25, !12, !7}
!25 = distinct !{!25, !26, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!26 = distinct !{!26, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!27 = !{!28, !12, !7}
!28 = distinct !{!28, !29, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!29 = distinct !{!29, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!30 = !{!31, !12, !7}
!31 = distinct !{!31, !32, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!32 = distinct !{!32, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!33 = !{!34, !12, !7}
!34 = distinct !{!34, !35, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!35 = distinct !{!35, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!36 = !{!37, !15, !10}
!37 = distinct !{!37, !38, !"_RNvMNtNtCs2k2z8Zem4rB_4core3ptr7mut_ptrONtNtNtB6_9core_arch3x867___m128i15write_unalignedCsly5PKRZWayS_4sha2: argument 0"}
!38 = distinct !{!38, !"_RNvMNtNtCs2k2z8Zem4rB_4core3ptr7mut_ptrONtNtNtB6_9core_arch3x867___m128i15write_unalignedCsly5PKRZWayS_4sha2"}
!39 = !{!40, !15, !10}
!40 = distinct !{!40, !41, !"_RNvMNtNtCs2k2z8Zem4rB_4core3ptr7mut_ptrONtNtNtB6_9core_arch3x867___m128i15write_unalignedCsly5PKRZWayS_4sha2: argument 0"}
!41 = distinct !{!41, !"_RNvMNtNtCs2k2z8Zem4rB_4core3ptr7mut_ptrONtNtNtB6_9core_arch3x867___m128i15write_unalignedCsly5PKRZWayS_4sha2"}
!42 = !{i64 -4605789078293946145}
!43 = !{!44}
!44 = distinct !{!44, !45, !"_RNvNtCsly5PKRZWayS_4sha26sha5128compress: argument 0"}
!45 = distinct !{!45, !"_RNvNtCsly5PKRZWayS_4sha26sha5128compress"}
!46 = !{!47}
!47 = distinct !{!47, !45, !"_RNvNtCsly5PKRZWayS_4sha26sha5128compress: argument 1"}
!48 = !{!49}
!49 = distinct !{!49, !50, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx28compress: argument 0"}
!50 = distinct !{!50, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx28compress"}
!51 = !{!52}
!52 = distinct !{!52, !50, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx28compress: argument 1"}
!53 = !{!49, !52, !44, !47}
!54 = !{!55}
!55 = distinct !{!55, !56, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx226sha512_compress_x86_64_avx: argument 0"}
!56 = distinct !{!56, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx226sha512_compress_x86_64_avx"}
!57 = !{!55, !58, !49, !52, !44, !47}
!58 = distinct !{!58, !56, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx226sha512_compress_x86_64_avx: argument 1"}
!59 = !{!49, !44}
!60 = !{!58, !52, !47}
!61 = !{!62}
!62 = distinct !{!62, !63, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx213load_data_avx: argument 1"}
!63 = distinct !{!63, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx213load_data_avx"}
!64 = !{!52, !47}
!65 = !{!66, !68, !62, !55, !49, !44}
!66 = distinct !{!66, !67, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!67 = distinct !{!67, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!68 = distinct !{!68, !63, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx213load_data_avx: argument 0"}
!69 = !{!68, !55, !58, !49, !52, !44, !47}
!70 = !{!71, !68, !62, !55, !49, !44}
!71 = distinct !{!71, !72, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!72 = distinct !{!72, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!73 = !{!74, !68, !62, !55, !49, !44}
!74 = distinct !{!74, !75, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!75 = distinct !{!75, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!76 = !{!77, !68, !62, !55, !49, !44}
!77 = distinct !{!77, !78, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!78 = distinct !{!78, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!79 = !{!80, !68, !62, !55, !49, !44}
!80 = distinct !{!80, !81, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!81 = distinct !{!81, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!82 = !{!83, !68, !62, !55, !49, !44}
!83 = distinct !{!83, !84, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!84 = distinct !{!84, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!85 = !{!86, !68, !62, !55, !49, !44}
!86 = distinct !{!86, !87, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!87 = distinct !{!87, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!88 = !{!89, !68, !62, !55, !49, !44}
!89 = distinct !{!89, !90, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!90 = distinct !{!90, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!91 = !{!92}
!92 = distinct !{!92, !93, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx215rounds_0_63_avx: argument 2"}
!93 = distinct !{!93, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx215rounds_0_63_avx"}
!94 = !{!95, !92, !55, !49, !52, !44, !47}
!95 = distinct !{!95, !96, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!96 = distinct !{!96, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!97 = !{!98, !99, !55, !58, !49, !52, !44, !47}
!98 = distinct !{!98, !93, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx215rounds_0_63_avx: argument 0"}
!99 = distinct !{!99, !93, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx215rounds_0_63_avx: argument 1"}
!100 = !{}
!101 = !{!102}
!102 = distinct !{!102, !103, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx212rounds_64_79: argument 1"}
!103 = distinct !{!103, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx212rounds_64_79"}
!104 = !{!105, !55, !58, !49, !52, !44, !47}
!105 = distinct !{!105, !103, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx212rounds_64_79: argument 0"}
!106 = !{!107, !55, !49, !44}
!107 = distinct !{!107, !108, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216accumulate_state: argument 0"}
!108 = distinct !{!108, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216accumulate_state"}
!109 = !{!110, !58, !52, !47}
!110 = distinct !{!110, !108, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216accumulate_state: argument 1"}
!111 = !{!112}
!112 = distinct !{!112, !113, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx214load_data_avx2: argument 1"}
!113 = distinct !{!113, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx214load_data_avx2"}
!114 = !{!115}
!115 = distinct !{!115, !113, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx214load_data_avx2: argument 2"}
!116 = !{!117, !119, !112, !115, !49, !44}
!117 = distinct !{!117, !118, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!118 = distinct !{!118, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!119 = distinct !{!119, !113, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx214load_data_avx2: argument 0"}
!120 = !{!121, !119, !112, !115, !49, !44}
!121 = distinct !{!121, !122, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!122 = distinct !{!122, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!123 = !{!119, !115, !49, !52, !44, !47}
!124 = !{!119, !112, !49, !52, !44, !47}
!125 = !{!126, !119, !112, !115, !49, !44}
!126 = distinct !{!126, !127, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!127 = distinct !{!127, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!128 = !{!129, !119, !112, !115, !49, !44}
!129 = distinct !{!129, !130, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!130 = distinct !{!130, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!131 = !{!132, !119, !112, !115, !49, !44}
!132 = distinct !{!132, !133, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!133 = distinct !{!133, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!134 = !{!135, !119, !112, !115, !49, !44}
!135 = distinct !{!135, !136, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!136 = distinct !{!136, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!137 = !{!138, !119, !112, !115, !49, !44}
!138 = distinct !{!138, !139, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!139 = distinct !{!139, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!140 = !{!141, !119, !112, !115, !49, !44}
!141 = distinct !{!141, !142, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!142 = distinct !{!142, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!143 = !{!144, !119, !112, !115, !49, !44}
!144 = distinct !{!144, !145, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!145 = distinct !{!145, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!146 = !{!147, !119, !112, !115, !49, !44}
!147 = distinct !{!147, !148, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!148 = distinct !{!148, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!149 = !{!150, !119, !112, !115, !49, !44}
!150 = distinct !{!150, !151, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!151 = distinct !{!151, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!152 = !{!153, !119, !112, !115, !49, !44}
!153 = distinct !{!153, !154, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!154 = distinct !{!154, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!155 = !{!156, !119, !112, !115, !49, !44}
!156 = distinct !{!156, !157, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!157 = distinct !{!157, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!158 = !{!159, !119, !112, !115, !49, !44}
!159 = distinct !{!159, !160, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!160 = distinct !{!160, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!161 = !{!162, !119, !112, !115, !49, !44}
!162 = distinct !{!162, !163, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!163 = distinct !{!163, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!164 = !{!165, !119, !112, !115, !49, !44}
!165 = distinct !{!165, !166, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!166 = distinct !{!166, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!167 = !{!168}
!168 = distinct !{!168, !169, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216rounds_0_63_avx2: argument 2"}
!169 = distinct !{!169, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216rounds_0_63_avx2"}
!170 = !{!171}
!171 = distinct !{!171, !169, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216rounds_0_63_avx2: argument 3"}
!172 = !{!173, !168, !171, !49, !52, !44, !47}
!173 = distinct !{!173, !174, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!174 = distinct !{!174, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!175 = !{!176, !177, !171, !49, !52, !44, !47}
!176 = distinct !{!176, !169, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216rounds_0_63_avx2: argument 0"}
!177 = distinct !{!177, !169, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx216rounds_0_63_avx2: argument 1"}
!178 = !{!176, !177, !168, !49, !52, !44, !47}
!179 = !{!180}
!180 = distinct !{!180, !181, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx212rounds_64_79: argument 1"}
!181 = distinct !{!181, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx212rounds_64_79"}
!182 = !{!183, !49, !52, !44, !47}
!183 = distinct !{!183, !181, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx212rounds_64_79: argument 0"}
!184 = !{!185}
!185 = distinct !{!185, !186, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx220process_second_block: argument 1"}
!186 = distinct !{!186, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx220process_second_block"}
!187 = !{!188, !49, !52, !44, !47}
!188 = distinct !{!188, !186, !"_RNvNtNtCsly5PKRZWayS_4sha26sha5128x86_avx220process_second_block: argument 0"}

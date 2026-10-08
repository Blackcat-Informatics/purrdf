; ModuleID = 'memchr-053c46354b26fd69.memchr.ecd88863d2af7566-cgu.0.rcgu.o'
source_filename = "memchr.ecd88863d2af7566-cgu.0"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

@_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN = local_unnamed_addr global ptr @_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect, align 8, !guid !0

; memchr::arch::x86_64::memchr::memchr_raw::detect
; Function Attrs: norecurse nounwind nonlazybind memory(readwrite, argmem: read, inaccessiblemem: write, target_mem: none) uwtable
define { i64, ptr } @_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect(i8 noundef %0, ptr noundef %1, ptr noundef %2) unnamed_addr #0 personality ptr @rust_eh_personality !dbg !10 !guid !19 {
  store atomic ptr @_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2, ptr @_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw2FN monotonic, align 8, !dbg !20
  %4 = insertelement <16 x i8> poison, i8 %0, i64 0, !dbg !31
  %5 = shufflevector <16 x i8> %4, <16 x i8> poison, <16 x i32> zeroinitializer, !dbg !31
  %6 = icmp ult ptr %1, %2, !dbg !63
  br i1 %6, label %7, label %.loopexit, !dbg !63

7:                                                ; preds = %3
  %8 = ptrtoint ptr %2 to i64, !dbg !66
  %9 = ptrtoint ptr %1 to i64, !dbg !66
  %10 = sub i64 %8, %9, !dbg !66
  %11 = icmp sgt i64 %10, -1, !dbg !79
  tail call void @llvm.assume(i1 %11), !dbg !79
  %12 = icmp samesign ult i64 %10, 16, !dbg !85
  br i1 %12, label %13, label %15, !dbg !85

13:                                               ; preds = %7
  %14 = getelementptr i8, ptr %1, i64 %10, !dbg !86
  br label %115, !dbg !86

15:                                               ; preds = %7
  %16 = load <16 x i8>, ptr %1, align 1, !dbg !90, !noalias !109
  %17 = icmp eq <16 x i8> %16, %5, !dbg !118
  %18 = bitcast <16 x i1> %17 to i16, !dbg !124
  %19 = icmp eq i16 %18, 0, !dbg !131
  br i1 %19, label %24, label %20, !dbg !136

20:                                               ; preds = %15
  %21 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %18, i1 true), !dbg !137
  %22 = zext nneg i16 %21 to i64, !dbg !151
  %23 = getelementptr inbounds nuw i8, ptr %1, i64 %22, !dbg !152
  br label %.loopexit, !dbg !155

24:                                               ; preds = %15
  %25 = and i64 %9, 15, !dbg !158
  %26 = sub nuw nsw i64 16, %25, !dbg !159
  %27 = getelementptr inbounds nuw i8, ptr %1, i64 %26, !dbg !160
  %28 = icmp samesign ugt i64 %10, 63, !dbg !163
  %29 = getelementptr inbounds i8, ptr %2, i64 -64
  %30 = icmp ule ptr %27, %29
  %31 = select i1 %28, i1 %30, i1 false, !dbg !163
  br i1 %31, label %.preheader19, label %.loopexit20, !dbg !163

.loopexit20:                                      ; preds = %82, %24
  %32 = phi ptr [ %27, %24 ], [ %83, %82 ], !dbg !165
  %33 = getelementptr inbounds i8, ptr %2, i64 -16
  %34 = icmp ugt ptr %32, %33, !dbg !166
  br i1 %34, label %.loopexit18, label %.preheader, !dbg !166

.preheader19:                                     ; preds = %24, %82
  %35 = phi ptr [ %83, %82 ], [ %27, %24 ], !dbg !165
  %36 = load <16 x i8>, ptr %35, align 16, !dbg !167, !noalias !170
  %37 = getelementptr inbounds nuw i8, ptr %35, i64 16, !dbg !171
  %38 = load <16 x i8>, ptr %37, align 16, !dbg !174, !noalias !170
  %39 = getelementptr inbounds nuw i8, ptr %35, i64 32, !dbg !176
  %40 = load <16 x i8>, ptr %39, align 16, !dbg !179, !noalias !170
  %41 = getelementptr inbounds nuw i8, ptr %35, i64 48, !dbg !181
  %42 = load <16 x i8>, ptr %41, align 16, !dbg !184, !noalias !170
  %43 = icmp eq <16 x i8> %36, %5, !dbg !186
  %44 = icmp eq <16 x i8> %38, %5, !dbg !190
  %45 = icmp eq <16 x i8> %40, %5, !dbg !194
  %46 = icmp eq <16 x i8> %42, %5, !dbg !198
  %47 = or <16 x i1> %43, %44, !dbg !202
  %48 = or <16 x i1> %45, %47, !dbg !208
  %49 = or <16 x i1> %48, %46, !dbg !208
  %50 = bitcast <16 x i1> %49 to i16, !dbg !213
  %51 = icmp eq i16 %50, 0, !dbg !220
  br i1 %51, label %82, label %85, !dbg !222

.loopexit18:                                      ; preds = %79, %.loopexit20
  %52 = phi ptr [ %32, %.loopexit20 ], [ %80, %79 ], !dbg !223
  %53 = icmp ult ptr %52, %2, !dbg !224
  br i1 %53, label %59, label %.loopexit, !dbg !224

.preheader:                                       ; preds = %.loopexit20, %79
  %54 = phi ptr [ %80, %79 ], [ %32, %.loopexit20 ]
  %55 = load <16 x i8>, ptr %54, align 1, !dbg !225, !noalias !231
  %56 = icmp eq <16 x i8> %55, %5, !dbg !236
  %57 = bitcast <16 x i1> %56 to i16, !dbg !239
  %58 = icmp eq i16 %57, 0, !dbg !242
  br i1 %58, label %79, label %75, !dbg !244

59:                                               ; preds = %.loopexit18
  %60 = ptrtoint ptr %52 to i64, !dbg !245
  %61 = sub i64 %8, %60, !dbg !245
  %62 = icmp sgt i64 %61, -1, !dbg !248
  tail call void @llvm.assume(i1 %62), !dbg !248, !noalias !250
  %63 = getelementptr i8, ptr %52, i64 %61, !dbg !251
  %64 = getelementptr i8, ptr %63, i64 -16, !dbg !251
  %65 = load <16 x i8>, ptr %64, align 1, !dbg !254, !noalias !259
  %66 = icmp eq <16 x i8> %65, %5, !dbg !264
  %67 = bitcast <16 x i1> %66 to i16, !dbg !267
  %68 = zext i16 %67 to i32, !dbg !267
  %69 = icmp ne i16 %67, 0, !dbg !270
  %70 = tail call range(i32 0, 33) i32 @llvm.cttz.i32(i32 range(i32 0, 65536) %68, i1 false), !dbg !272
  %71 = zext nneg i32 %70 to i64, !dbg !272
  %72 = getelementptr inbounds nuw i8, ptr %64, i64 %71, !dbg !272
  %73 = select i1 %69, ptr %72, ptr undef, !dbg !272
  %74 = zext i1 %69 to i64, !dbg !272
  br label %.loopexit, !dbg !273

75:                                               ; preds = %.preheader
  %76 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %57, i1 true), !dbg !275
  %77 = zext nneg i16 %76 to i64, !dbg !279
  %78 = getelementptr inbounds nuw i8, ptr %54, i64 %77, !dbg !280
  br label %.loopexit, !dbg !273

79:                                               ; preds = %.preheader
  %80 = getelementptr inbounds nuw i8, ptr %54, i64 16, !dbg !282
  %81 = icmp ugt ptr %80, %33, !dbg !166
  br i1 %81, label %.loopexit18, label %.preheader, !dbg !166

82:                                               ; preds = %.preheader19
  %83 = getelementptr inbounds nuw i8, ptr %35, i64 64, !dbg !284
  %84 = icmp ugt ptr %83, %29, !dbg !286
  br i1 %84, label %.loopexit20, label %.preheader19, !dbg !286

85:                                               ; preds = %.preheader19
  %86 = getelementptr inbounds nuw i8, ptr %35, i64 16
  %87 = getelementptr inbounds nuw i8, ptr %35, i64 32
  %88 = getelementptr inbounds nuw i8, ptr %35, i64 48
  %89 = bitcast <16 x i1> %43 to i16, !dbg !287
  %90 = icmp eq i16 %89, 0, !dbg !290
  br i1 %90, label %91, label %94, !dbg !293

91:                                               ; preds = %85
  %92 = bitcast <16 x i1> %44 to i16, !dbg !294
  %93 = icmp eq i16 %92, 0, !dbg !297
  br i1 %93, label %98, label %101, !dbg !300

94:                                               ; preds = %85
  %95 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %89, i1 true), !dbg !301
  %96 = zext nneg i16 %95 to i64, !dbg !304
  %97 = getelementptr inbounds nuw i8, ptr %35, i64 %96, !dbg !305
  br label %.loopexit, !dbg !307

98:                                               ; preds = %91
  %99 = bitcast <16 x i1> %45 to i16, !dbg !309
  %100 = icmp eq i16 %99, 0, !dbg !312
  br i1 %100, label %105, label %111, !dbg !315

101:                                              ; preds = %91
  %102 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %92, i1 true), !dbg !316
  %103 = zext nneg i16 %102 to i64, !dbg !319
  %104 = getelementptr inbounds nuw i8, ptr %86, i64 %103, !dbg !320
  br label %.loopexit, !dbg !322

105:                                              ; preds = %98
  %106 = bitcast <16 x i1> %46 to i16, !dbg !324
  %107 = zext i16 %106 to i32, !dbg !324
  %108 = tail call range(i32 0, 33) i32 @llvm.cttz.i32(i32 %107, i1 false), !dbg !327
  %109 = zext nneg i32 %108 to i64, !dbg !331
  %110 = getelementptr inbounds nuw i8, ptr %88, i64 %109, !dbg !332
  br label %.loopexit, !dbg !334

111:                                              ; preds = %98
  %112 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %99, i1 true), !dbg !336
  %113 = zext nneg i16 %112 to i64, !dbg !339
  %114 = getelementptr inbounds nuw i8, ptr %87, i64 %113, !dbg !340
  br label %.loopexit, !dbg !334

115:                                              ; preds = %119, %13
  %116 = phi ptr [ %1, %13 ], [ %120, %119 ]
  %117 = load i8, ptr %116, align 1, !dbg !342, !noalias !170, !noundef !18
  %118 = icmp eq i8 %117, %0, !dbg !343
  br i1 %118, label %.loopexit, label %119, !dbg !348

119:                                              ; preds = %115
  %120 = getelementptr inbounds nuw i8, ptr %116, i64 1, !dbg !349
  %121 = icmp eq ptr %120, %2, !dbg !86
  br i1 %121, label %.loopexit, label %115, !dbg !86

.loopexit:                                        ; preds = %119, %115, %111, %105, %101, %94, %75, %59, %.loopexit18, %20, %3
  %122 = phi ptr [ undef, %3 ], [ undef, %.loopexit18 ], [ %23, %20 ], [ %97, %94 ], [ %104, %101 ], [ %114, %111 ], [ %110, %105 ], [ %78, %75 ], [ %73, %59 ], [ %116, %115 ], [ %14, %119 ], !dbg !352
  %123 = phi i64 [ 0, %3 ], [ 0, %.loopexit18 ], [ 1, %20 ], [ 1, %94 ], [ 1, %101 ], [ 1, %111 ], [ 1, %105 ], [ 1, %75 ], [ %74, %59 ], [ 1, %115 ], [ 0, %119 ], !dbg !352
  %124 = insertvalue { i64, ptr } poison, i64 %123, 0, !dbg !353
  %125 = insertvalue { i64, ptr } %124, ptr %122, 1, !dbg !353
  ret { i64, ptr } %125, !dbg !354
}

; memchr::arch::x86_64::memchr::memchr_raw::find_sse2
; Function Attrs: nofree norecurse nosync nounwind nonlazybind memory(argmem: read, inaccessiblemem: write) uwtable
define internal { i64, ptr } @_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2(i8 noundef %0, ptr noundef %1, ptr noundef %2) unnamed_addr #1 personality ptr @rust_eh_personality !dbg !61 !guid !355 {
  %4 = insertelement <16 x i8> poison, i8 %0, i64 0, !dbg !356
  %5 = shufflevector <16 x i8> %4, <16 x i8> poison, <16 x i32> zeroinitializer, !dbg !356
  %6 = icmp ult ptr %1, %2, !dbg !362
  br i1 %6, label %7, label %.loopexit, !dbg !362

7:                                                ; preds = %3
  %8 = ptrtoint ptr %2 to i64, !dbg !364
  %9 = ptrtoint ptr %1 to i64, !dbg !364
  %10 = sub i64 %8, %9, !dbg !364
  %11 = icmp sgt i64 %10, -1, !dbg !367
  tail call void @llvm.assume(i1 %11), !dbg !367
  %12 = icmp samesign ult i64 %10, 16, !dbg !369
  br i1 %12, label %13, label %15, !dbg !369

13:                                               ; preds = %7
  %14 = getelementptr i8, ptr %1, i64 %10, !dbg !370
  br label %115, !dbg !370

15:                                               ; preds = %7
  %16 = load <16 x i8>, ptr %1, align 1, !dbg !372, !noalias !379
  %17 = icmp eq <16 x i8> %16, %5, !dbg !388
  %18 = bitcast <16 x i1> %17 to i16, !dbg !391
  %19 = icmp eq i16 %18, 0, !dbg !394
  br i1 %19, label %24, label %20, !dbg !396

20:                                               ; preds = %15
  %21 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %18, i1 true), !dbg !397
  %22 = zext nneg i16 %21 to i64, !dbg !401
  %23 = getelementptr inbounds nuw i8, ptr %1, i64 %22, !dbg !402
  br label %.loopexit, !dbg !404

24:                                               ; preds = %15
  %25 = and i64 %9, 15, !dbg !405
  %26 = sub nuw nsw i64 16, %25, !dbg !406
  %27 = getelementptr inbounds nuw i8, ptr %1, i64 %26, !dbg !407
  %28 = icmp samesign ugt i64 %10, 63, !dbg !409
  %29 = getelementptr inbounds i8, ptr %2, i64 -64
  %30 = icmp ule ptr %27, %29
  %31 = select i1 %28, i1 %30, i1 false, !dbg !409
  br i1 %31, label %.preheader19, label %.loopexit20, !dbg !409

.loopexit20:                                      ; preds = %82, %24
  %32 = phi ptr [ %27, %24 ], [ %83, %82 ], !dbg !410
  %33 = getelementptr inbounds i8, ptr %2, i64 -16
  %34 = icmp ugt ptr %32, %33, !dbg !411
  br i1 %34, label %.loopexit18, label %.preheader, !dbg !411

.preheader19:                                     ; preds = %24, %82
  %35 = phi ptr [ %83, %82 ], [ %27, %24 ], !dbg !410
  %36 = load <16 x i8>, ptr %35, align 16, !dbg !412, !noalias !414
  %37 = getelementptr inbounds nuw i8, ptr %35, i64 16, !dbg !415
  %38 = load <16 x i8>, ptr %37, align 16, !dbg !417, !noalias !414
  %39 = getelementptr inbounds nuw i8, ptr %35, i64 32, !dbg !419
  %40 = load <16 x i8>, ptr %39, align 16, !dbg !421, !noalias !414
  %41 = getelementptr inbounds nuw i8, ptr %35, i64 48, !dbg !423
  %42 = load <16 x i8>, ptr %41, align 16, !dbg !425, !noalias !414
  %43 = icmp eq <16 x i8> %36, %5, !dbg !427
  %44 = icmp eq <16 x i8> %38, %5, !dbg !430
  %45 = icmp eq <16 x i8> %40, %5, !dbg !433
  %46 = icmp eq <16 x i8> %42, %5, !dbg !436
  %47 = or <16 x i1> %43, %44, !dbg !439
  %48 = or <16 x i1> %45, %47, !dbg !442
  %49 = or <16 x i1> %48, %46, !dbg !442
  %50 = bitcast <16 x i1> %49 to i16, !dbg !445
  %51 = icmp eq i16 %50, 0, !dbg !449
  br i1 %51, label %82, label %85, !dbg !451

.loopexit18:                                      ; preds = %79, %.loopexit20
  %52 = phi ptr [ %32, %.loopexit20 ], [ %80, %79 ], !dbg !452
  %53 = icmp ult ptr %52, %2, !dbg !453
  br i1 %53, label %59, label %.loopexit, !dbg !453

.preheader:                                       ; preds = %.loopexit20, %79
  %54 = phi ptr [ %80, %79 ], [ %32, %.loopexit20 ]
  %55 = load <16 x i8>, ptr %54, align 1, !dbg !454, !noalias !459
  %56 = icmp eq <16 x i8> %55, %5, !dbg !464
  %57 = bitcast <16 x i1> %56 to i16, !dbg !467
  %58 = icmp eq i16 %57, 0, !dbg !470
  br i1 %58, label %79, label %75, !dbg !472

59:                                               ; preds = %.loopexit18
  %60 = ptrtoint ptr %52 to i64, !dbg !473
  %61 = sub i64 %8, %60, !dbg !473
  %62 = icmp sgt i64 %61, -1, !dbg !476
  tail call void @llvm.assume(i1 %62), !dbg !476, !noalias !478
  %63 = getelementptr i8, ptr %52, i64 %61, !dbg !479
  %64 = getelementptr i8, ptr %63, i64 -16, !dbg !479
  %65 = load <16 x i8>, ptr %64, align 1, !dbg !481, !noalias !486
  %66 = icmp eq <16 x i8> %65, %5, !dbg !491
  %67 = bitcast <16 x i1> %66 to i16, !dbg !494
  %68 = zext i16 %67 to i32, !dbg !494
  %69 = icmp ne i16 %67, 0, !dbg !497
  %70 = tail call range(i32 0, 33) i32 @llvm.cttz.i32(i32 range(i32 0, 65536) %68, i1 false), !dbg !499
  %71 = zext nneg i32 %70 to i64, !dbg !499
  %72 = getelementptr inbounds nuw i8, ptr %64, i64 %71, !dbg !499
  %73 = select i1 %69, ptr %72, ptr undef, !dbg !499
  %74 = zext i1 %69 to i64, !dbg !499
  br label %.loopexit, !dbg !500

75:                                               ; preds = %.preheader
  %76 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %57, i1 true), !dbg !501
  %77 = zext nneg i16 %76 to i64, !dbg !505
  %78 = getelementptr inbounds nuw i8, ptr %54, i64 %77, !dbg !506
  br label %.loopexit, !dbg !500

79:                                               ; preds = %.preheader
  %80 = getelementptr inbounds nuw i8, ptr %54, i64 16, !dbg !508
  %81 = icmp ugt ptr %80, %33, !dbg !411
  br i1 %81, label %.loopexit18, label %.preheader, !dbg !411

82:                                               ; preds = %.preheader19
  %83 = getelementptr inbounds nuw i8, ptr %35, i64 64, !dbg !510
  %84 = icmp ugt ptr %83, %29, !dbg !512
  br i1 %84, label %.loopexit20, label %.preheader19, !dbg !512

85:                                               ; preds = %.preheader19
  %86 = getelementptr inbounds nuw i8, ptr %35, i64 16
  %87 = getelementptr inbounds nuw i8, ptr %35, i64 32
  %88 = getelementptr inbounds nuw i8, ptr %35, i64 48
  %89 = bitcast <16 x i1> %43 to i16, !dbg !513
  %90 = icmp eq i16 %89, 0, !dbg !516
  br i1 %90, label %91, label %94, !dbg !518

91:                                               ; preds = %85
  %92 = bitcast <16 x i1> %44 to i16, !dbg !519
  %93 = icmp eq i16 %92, 0, !dbg !522
  br i1 %93, label %98, label %101, !dbg !524

94:                                               ; preds = %85
  %95 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %89, i1 true), !dbg !525
  %96 = zext nneg i16 %95 to i64, !dbg !528
  %97 = getelementptr inbounds nuw i8, ptr %35, i64 %96, !dbg !529
  br label %.loopexit, !dbg !531

98:                                               ; preds = %91
  %99 = bitcast <16 x i1> %45 to i16, !dbg !532
  %100 = icmp eq i16 %99, 0, !dbg !535
  br i1 %100, label %105, label %111, !dbg !537

101:                                              ; preds = %91
  %102 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %92, i1 true), !dbg !538
  %103 = zext nneg i16 %102 to i64, !dbg !541
  %104 = getelementptr inbounds nuw i8, ptr %86, i64 %103, !dbg !542
  br label %.loopexit, !dbg !544

105:                                              ; preds = %98
  %106 = bitcast <16 x i1> %46 to i16, !dbg !545
  %107 = zext i16 %106 to i32, !dbg !545
  %108 = tail call range(i32 0, 33) i32 @llvm.cttz.i32(i32 %107, i1 false), !dbg !548
  %109 = zext nneg i32 %108 to i64, !dbg !551
  %110 = getelementptr inbounds nuw i8, ptr %88, i64 %109, !dbg !552
  br label %.loopexit, !dbg !554

111:                                              ; preds = %98
  %112 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %99, i1 true), !dbg !555
  %113 = zext nneg i16 %112 to i64, !dbg !558
  %114 = getelementptr inbounds nuw i8, ptr %87, i64 %113, !dbg !559
  br label %.loopexit, !dbg !554

115:                                              ; preds = %119, %13
  %116 = phi ptr [ %1, %13 ], [ %120, %119 ]
  %117 = load i8, ptr %116, align 1, !dbg !561, !noalias !414, !noundef !18
  %118 = icmp eq i8 %117, %0, !dbg !562
  br i1 %118, label %.loopexit, label %119, !dbg !564

119:                                              ; preds = %115
  %120 = getelementptr inbounds nuw i8, ptr %116, i64 1, !dbg !565
  %121 = icmp eq ptr %120, %2, !dbg !370
  br i1 %121, label %.loopexit, label %115, !dbg !370

.loopexit:                                        ; preds = %119, %115, %111, %105, %101, %94, %75, %59, %.loopexit18, %20, %3
  %122 = phi ptr [ undef, %3 ], [ undef, %.loopexit18 ], [ %23, %20 ], [ %97, %94 ], [ %104, %101 ], [ %114, %111 ], [ %110, %105 ], [ %78, %75 ], [ %73, %59 ], [ %14, %119 ], [ %116, %115 ], !dbg !567
  %123 = phi i64 [ 0, %3 ], [ 0, %.loopexit18 ], [ 1, %20 ], [ 1, %94 ], [ 1, %101 ], [ 1, %111 ], [ 1, %105 ], [ 1, %75 ], [ %74, %59 ], [ 0, %119 ], [ 1, %115 ], !dbg !567
  %124 = insertvalue { i64, ptr } poison, i64 %123, 0, !dbg !568
  %125 = insertvalue { i64, ptr } %124, ptr %122, 1, !dbg !568
  ret { i64, ptr } %125, !dbg !569
}

; Function Attrs: nonlazybind
declare i32 @rust_eh_personality(...) unnamed_addr #2

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.cttz.i32(i32, i1 immarg) #4

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i16 @llvm.cttz.i16(i16, i1 immarg) #4

attributes #0 = { norecurse nounwind nonlazybind memory(readwrite, argmem: read, inaccessiblemem: write, target_mem: none) uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" }
attributes #1 = { nofree norecurse nosync nounwind nonlazybind memory(argmem: read, inaccessiblemem: write) uwtable "frame-pointer"="non-leaf" "probe-stack"="inline-asm" "target-cpu"="x86-64" "target-features"="+sse,+sse2" }
attributes #2 = { nonlazybind "target-cpu"="x86-64" }
attributes #3 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }
attributes #4 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.module.flags = !{!1, !2, !3, !4, !5, !6}
!llvm.ident = !{!7}
!llvm.dbg.cu = !{!8}

!0 = !{i64 -2873580271327952025}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 2, !"RtLibUseGOT", i32 1}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{i32 7, !"frame-pointer", i32 1}
!5 = !{i32 7, !"Dwarf Version", i32 4}
!6 = !{i32 2, !"Debug Info Version", i32 3}
!7 = !{!"rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)"}
!8 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !9, producer: "clang LLVM (rustc version 1.100.0-nightly (4b6d04e70 2026-09-13))", isOptimized: true, runtimeVersion: 0, emissionKind: FullDebug, splitDebugInlining: false, nameTableKind: None)
!9 = !DIFile(filename: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3/src/lib.rs/@/memchr.ecd88863d2af7566-cgu.0", directory: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3")
!10 = distinct !DISubprogram(name: "detect", linkageName: "_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw6detect", scope: !12, file: !11, line: 109, type: !17, scopeLine: 109, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!11 = !DIFile(filename: "src/arch/x86_64/memchr.rs", directory: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3", checksumkind: CSK_MD5, checksum: "ce3e0392ca7090e9b0bab95d909cb08a")
!12 = !DINamespace(name: "memchr_raw", scope: !13)
!13 = !DINamespace(name: "memchr", scope: !14)
!14 = !DINamespace(name: "x86_64", scope: !15)
!15 = !DINamespace(name: "arch", scope: !16)
!16 = !DINamespace(name: "memchr", scope: null)
!17 = !DISubroutineType(types: !18)
!18 = !{}
!19 = !{i64 3018688349592522503}
!20 = !DILocation(line: 4303, column: 24, scope: !21, inlinedAt: !26)
!21 = distinct !DISubprogram(name: "atomic_store<*mut (), false>", linkageName: "_RINvNtNtCs2k2z8Zem4rB_4core4sync6atomic12atomic_storeOuKb0_ECskkIW8vVChzC_6memchr", scope: !23, file: !22, line: 4299, type: !17, scopeLine: 4299, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!22 = !DIFile(filename: "library/core/src/sync/atomic.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "1954a2b0884b1010966540d03161d55a")
!23 = !DINamespace(name: "atomic", scope: !24)
!24 = !DINamespace(name: "sync", scope: !25)
!25 = !DINamespace(name: "core", scope: null)
!26 = distinct !DILocation(line: 1953, column: 13, scope: !27, inlinedAt: !29)
!27 = distinct !DISubprogram(name: "store<()>", linkageName: "_RNvMs3_NtNtCs2k2z8Zem4rB_4core4sync6atomicINtB5_6AtomicOuE5storeCskkIW8vVChzC_6memchr", scope: !28, file: !22, line: 1950, type: !17, scopeLine: 1950, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!28 = !DINamespace(name: "Atomic", scope: !23)
!29 = !DILocation(line: 138, column: 16, scope: !30)
!30 = distinct !DILexicalBlock(scope: !10, file: !11, line: 114, column: 13)
!31 = !DILocation(line: 59, column: 18, scope: !32, inlinedAt: !37)
!32 = distinct !DISubprogram(name: "splat<i8, 16>", linkageName: "_RNvMsb_NtNtCs2k2z8Zem4rB_4core9core_arch4simdINtB5_4SimdaKj10_E5splatCskkIW8vVChzC_6memchr", scope: !34, file: !33, line: 58, type: !17, scopeLine: 58, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!33 = !DIFile(filename: "library/core/src/../../stdarch/crates/core_arch/src/simd.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "356b26e41e54b4de8bd823faee13875e")
!34 = !DINamespace(name: "Simd", scope: !35)
!35 = !DINamespace(name: "simd", scope: !36)
!36 = !DINamespace(name: "core_arch", scope: !25)
!37 = distinct !DILocation(line: 1221, column: 5, scope: !38, inlinedAt: !42)
!38 = distinct !DISubprogram(name: "_mm_set1_epi8", linkageName: "_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse213__mm_set1_epi8", scope: !40, file: !39, line: 1220, type: !17, scopeLine: 1220, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!39 = !DIFile(filename: "library/core/src/../../stdarch/crates/core_arch/src/x86/sse2.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "aa60afdbf31136b7cbaf061cd4f57a6c")
!40 = !DINamespace(name: "sse2", scope: !41)
!41 = !DINamespace(name: "x86", scope: !36)
!42 = distinct !DILocation(line: 208, column: 13, scope: !43, inlinedAt: !48)
!43 = distinct !DISubprogram(name: "splat", linkageName: "_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector5splat", scope: !45, file: !44, line: 207, type: !17, scopeLine: 207, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!44 = !DIFile(filename: "src/vector.rs", directory: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3", checksumkind: CSK_MD5, checksum: "ecc471d477aa3d11ebdcf4464510ea29")
!45 = !DINamespace(name: "{impl#0}", scope: !46)
!46 = !DINamespace(name: "x86sse2", scope: !47)
!47 = !DINamespace(name: "vector", scope: !16)
!48 = distinct !DILocation(line: 112, column: 31, scope: !49, inlinedAt: !54)
!49 = distinct !DISubprogram(name: "new<core::core_arch::x86::__m128i>", linkageName: "_RNvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB2_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE3newB8_", scope: !51, file: !50, line: 111, type: !17, scopeLine: 111, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!50 = !DIFile(filename: "src/arch/generic/memchr.rs", directory: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3", checksumkind: CSK_MD5, checksum: "b39ab98f848850dfe7d7e948f527651e")
!51 = !DINamespace(name: "One", scope: !52)
!52 = !DINamespace(name: "memchr", scope: !53)
!53 = !DINamespace(name: "generic", scope: !15)
!54 = distinct !DILocation(line: 64, column: 13, scope: !55, inlinedAt: !60)
!55 = distinct !DISubprogram(name: "new_unchecked", linkageName: "_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One13new_unchecked", scope: !57, file: !56, line: 63, type: !17, scopeLine: 63, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!56 = !DIFile(filename: "src/arch/x86_64/sse2/memchr.rs", directory: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3", checksumkind: CSK_MD5, checksum: "8e73ea8500307019f00e4df2ad17f08c")
!57 = !DINamespace(name: "One", scope: !58)
!58 = !DINamespace(name: "memchr", scope: !59)
!59 = !DINamespace(name: "sse2", scope: !14)
!60 = distinct !DILocation(line: 96, column: 13, scope: !61, inlinedAt: !62)
!61 = distinct !DISubprogram(name: "find_sse2", linkageName: "_RNvNvNtNtNtCskkIW8vVChzC_6memchr4arch6x86_646memchr10memchr_raw9find_sse2", scope: !12, file: !11, line: 90, type: !17, scopeLine: 90, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!62 = distinct !DILocation(line: 144, column: 13, scope: !30)
!63 = !DILocation(line: 161, column: 12, scope: !64, inlinedAt: !65)
!64 = distinct !DISubprogram(name: "find_raw", linkageName: "_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One8find_raw", scope: !57, file: !56, line: 156, type: !17, scopeLine: 156, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!65 = distinct !DILocation(line: 97, column: 18, scope: !61, inlinedAt: !62)
!66 = !DILocation(line: 624, column: 18, scope: !67, inlinedAt: !73)
!67 = distinct !DILexicalBlock(scope: !69, file: !68, line: 621, column: 9)
!68 = !DIFile(filename: "library/core/src/ptr/const_ptr.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "3476ea789cc24546d3b3809125701ad2")
!69 = distinct !DISubprogram(name: "offset_from<u8>", linkageName: "_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh11offset_fromCskkIW8vVChzC_6memchr", scope: !70, file: !68, line: 617, type: !17, scopeLine: 617, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!70 = !DINamespace(name: "{impl#0}", scope: !71)
!71 = !DINamespace(name: "const_ptr", scope: !72)
!72 = !DINamespace(name: "ptr", scope: !25)
!73 = distinct !DILocation(line: 23, column: 30, scope: !74, inlinedAt: !78)
!74 = distinct !DISubprogram(name: "distance<u8>", linkageName: "_RNvXNtCskkIW8vVChzC_6memchr3extPhNtB2_7Pointer8distanceB4_", scope: !76, file: !75, line: 21, type: !17, scopeLine: 21, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!75 = !DIFile(filename: "src/ext.rs", directory: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3", checksumkind: CSK_MD5, checksum: "8bac517d6efe29b428a754b1cdecf123")
!76 = !DINamespace(name: "{impl#0}", scope: !77)
!77 = !DINamespace(name: "ext", scope: !16)
!78 = distinct !DILocation(line: 164, column: 16, scope: !64, inlinedAt: !65)
!79 = !DILocation(line: 1648, column: 9, scope: !80, inlinedAt: !84)
!80 = distinct !DISubprogram(name: "unwrap_unchecked<usize, core::num::error::TryFromIntError>", linkageName: "_RNvMNtCs2k2z8Zem4rB_4core6resultINtB2_6ResultjNtNtNtB4_3num5error15TryFromIntErrorE16unwrap_uncheckedCskkIW8vVChzC_6memchr", scope: !82, file: !81, line: 1647, type: !17, scopeLine: 1647, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!81 = !DIFile(filename: "library/core/src/result.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "92ab1ec144e579029b4ec17e105b17eb")
!82 = !DINamespace(name: "Result", scope: !83)
!83 = !DINamespace(name: "result", scope: !25)
!84 = distinct !DILocation(line: 23, column: 51, scope: !74, inlinedAt: !78)
!85 = !DILocation(line: 164, column: 12, scope: !64, inlinedAt: !65)
!86 = !DILocation(line: 1161, column: 11, scope: !87, inlinedAt: !89)
!87 = distinct !DILexicalBlock(scope: !88, file: !50, line: 1160, column: 5)
!88 = distinct !DISubprogram(name: "fwd_byte_by_byte<memchr::arch::x86_64::sse2::memchr::{impl#0}::find_raw::{closure_env#0}>", linkageName: "_RINvNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchr16fwd_byte_by_byteNCNvMNtNtNtB6_6x86_644sse26memchrNtB1a_3One8find_raw0EB8_", scope: !52, file: !50, line: 1154, type: !17, scopeLine: 1154, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!89 = distinct !DILocation(line: 166, column: 20, scope: !64, inlinedAt: !65)
!90 = !DILocation(line: 574, column: 14, scope: !91, inlinedAt: !93)
!91 = distinct !DISubprogram(name: "copy_nonoverlapping<u8>", linkageName: "_RINvNtCs2k2z8Zem4rB_4core3ptr19copy_nonoverlappinghECskkIW8vVChzC_6memchr", scope: !72, file: !92, line: 553, type: !17, scopeLine: 553, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!92 = !DIFile(filename: "library/core/src/ptr/mod.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "880a13f9557399be745f3ca5a0b3d5e4")
!93 = distinct !DILocation(line: 1341, column: 5, scope: !94, inlinedAt: !96)
!94 = distinct !DILexicalBlock(scope: !95, file: !39, line: 1340, column: 5)
!95 = distinct !DISubprogram(name: "_mm_loadu_si128", linkageName: "_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128", scope: !40, file: !39, line: 1339, type: !17, scopeLine: 1339, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!96 = distinct !DILocation(line: 218, column: 13, scope: !97, inlinedAt: !98)
!97 = distinct !DISubprogram(name: "load_unaligned", linkageName: "_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector14load_unaligned", scope: !45, file: !44, line: 217, type: !17, scopeLine: 217, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!98 = distinct !DILocation(line: 421, column: 21, scope: !99, inlinedAt: !100)
!99 = distinct !DISubprogram(name: "search_chunk<core::core_arch::x86::__m128i, fn(memchr::vector::SensibleMoveMask) -> usize>", linkageName: "_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_", scope: !51, file: !50, line: 416, type: !17, scopeLine: 416, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!100 = distinct !DILocation(line: 165, column: 33, scope: !101, inlinedAt: !105)
!101 = distinct !DILexicalBlock(scope: !102, file: !50, line: 165, column: 60)
!102 = distinct !DILexicalBlock(scope: !103, file: !50, line: 155, column: 9)
!103 = distinct !DILexicalBlock(scope: !104, file: !50, line: 154, column: 9)
!104 = distinct !DISubprogram(name: "find_raw<core::core_arch::x86::__m128i>", linkageName: "_RNvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB2_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE8find_rawB8_", scope: !51, file: !50, line: 143, type: !17, scopeLine: 143, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!105 = distinct !DILocation(line: 290, column: 16, scope: !106, inlinedAt: !108)
!106 = distinct !DISubprogram(name: "find_raw_impl", linkageName: "_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One13find_raw_impl", scope: !57, file: !56, line: 285, type: !107, scopeLine: 285, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!107 = !DISubroutineType(cc: DW_CC_nocall, types: !18)
!108 = distinct !DILocation(line: 185, column: 14, scope: !64, inlinedAt: !65)
!109 = !{!110, !112, !114, !116}
!110 = distinct !{!110, !111, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!111 = distinct !{!111, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!112 = distinct !{!112, !113, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_: argument 0"}
!113 = distinct !{!113, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_"}
!114 = distinct !{!114, !115, !"_RNvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB2_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE8find_rawB8_: argument 0"}
!115 = distinct !{!115, !"_RNvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB2_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE8find_rawB8_"}
!116 = distinct !{!116, !117, !"_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One8find_raw: argument 0"}
!117 = distinct !{!117, !"_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One8find_raw"}
!118 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !120)
!119 = distinct !DISubprogram(name: "_mm_cmpeq_epi8", linkageName: "_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse214__mm_cmpeq_epi8", scope: !40, file: !39, line: 919, type: !107, scopeLine: 919, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!120 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !122)
!121 = distinct !DISubprogram(name: "cmpeq", linkageName: "_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector5cmpeq", scope: !45, file: !44, line: 227, type: !17, scopeLine: 227, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!122 = distinct !DILocation(line: 422, column: 28, scope: !123, inlinedAt: !100)
!123 = distinct !DILexicalBlock(scope: !99, file: !50, line: 421, column: 9)
!124 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !128)
!125 = distinct !DILexicalBlock(scope: !126, file: !39, line: 1569, column: 9)
!126 = distinct !DILexicalBlock(scope: !127, file: !39, line: 1568, column: 9)
!127 = distinct !DISubprogram(name: "_mm_movemask_epi8", linkageName: "_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse217__mm_movemask_epi8", scope: !40, file: !39, line: 1566, type: !107, scopeLine: 1566, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!128 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !130)
!129 = distinct !DISubprogram(name: "movemask", linkageName: "_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector8movemask", scope: !45, file: !44, line: 222, type: !17, scopeLine: 222, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!130 = distinct !DILocation(line: 422, column: 41, scope: !123, inlinedAt: !100)
!131 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !134)
!132 = distinct !DISubprogram(name: "has_non_zero", linkageName: "_RNvXs_NtCskkIW8vVChzC_6memchr6vectorNtB4_16SensibleMoveMaskNtB4_8MoveMask12has_non_zero", scope: !133, file: !44, line: 146, type: !17, scopeLine: 146, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!133 = !DINamespace(name: "{impl#1}", scope: !47)
!134 = distinct !DILocation(line: 423, column: 17, scope: !135, inlinedAt: !100)
!135 = distinct !DILexicalBlock(scope: !123, file: !50, line: 422, column: 9)
!136 = !DILocation(line: 423, column: 12, scope: !135, inlinedAt: !100)
!137 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !142)
!138 = distinct !DISubprogram(name: "trailing_zeros", linkageName: "_RNvMs6_NtCs2k2z8Zem4rB_4core3numm14trailing_zeros", scope: !140, file: !139, line: 177, type: !17, scopeLine: 177, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!139 = !DIFile(filename: "library/core/src/num/uint_macros.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "3c8c34b1702d7a857393ccdf9a4f21cb")
!140 = !DINamespace(name: "{impl#8}", scope: !141)
!141 = !DINamespace(name: "num", scope: !25)
!142 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !144)
!143 = distinct !DISubprogram(name: "first_offset", linkageName: "_RNvXs_NtCskkIW8vVChzC_6memchr6vectorNtB4_16SensibleMoveMaskNtB4_8MoveMask12first_offset", scope: !133, file: !44, line: 171, type: !17, scopeLine: 171, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!144 = distinct !DILocation(line: 79, column: 5, scope: !145, inlinedAt: !150)
!145 = distinct !DISubprogram(name: "call<fn(memchr::vector::SensibleMoveMask) -> usize, (memchr::vector::SensibleMoveMask)>", linkageName: "_RNvYNvYNtNtCskkIW8vVChzC_6memchr6vector16SensibleMoveMaskNtB7_8MoveMask12first_offsetINtNtNtCs2k2z8Zem4rB_4core3ops8function2FnTB5_EE4callB9_", scope: !147, file: !146, line: 79, type: !107, scopeLine: 79, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!146 = !DIFile(filename: "library/core/src/ops/function.rs", directory: "/rustc/4b6d04e706108ccfeafe2547fbe857dfe8972bad", checksumkind: CSK_MD5, checksum: "5fd63130e402556f5b2ba11cb847a9a0")
!147 = !DINamespace(name: "Fn", scope: !148)
!148 = !DINamespace(name: "function", scope: !149)
!149 = !DINamespace(name: "ops", scope: !25)
!150 = distinct !DILocation(line: 424, column: 26, scope: !135, inlinedAt: !100)
!151 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !144)
!152 = !DILocation(line: 872, column: 18, scope: !153, inlinedAt: !154)
!153 = distinct !DISubprogram(name: "add<u8>", linkageName: "_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh3addCskkIW8vVChzC_6memchr", scope: !70, file: !68, line: 838, type: !17, scopeLine: 838, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!154 = distinct !DILocation(line: 424, column: 22, scope: !135, inlinedAt: !100)
!155 = !DILocation(line: 0, scope: !156, inlinedAt: !105)
!156 = !DILexicalBlockFile(scope: !102, file: !157, discriminator: 0)
!157 = !DIFile(filename: "src/lib.rs", directory: "/cargo/registry/25cdd57fae9f0462/memchr-2.8.3", checksumkind: CSK_MD5, checksum: "f5820940cb026bac626d0f8d44caa95d")
!158 = !DILocation(line: 169, column: 44, scope: !102, inlinedAt: !105)
!159 = !DILocation(line: 169, column: 33, scope: !102, inlinedAt: !105)
!160 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !162)
!161 = distinct !DISubprogram(name: "add<u8>", linkageName: "_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh3addCskkIW8vVChzC_6memchr", scope: !70, file: !68, line: 838, type: !17, scopeLine: 838, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!162 = distinct !DILocation(line: 169, column: 29, scope: !102, inlinedAt: !105)
!163 = !DILocation(line: 171, column: 12, scope: !164, inlinedAt: !105)
!164 = distinct !DILexicalBlock(scope: !102, file: !50, line: 169, column: 9)
!165 = !DILocation(line: 0, scope: !102, inlinedAt: !105)
!166 = !DILocation(line: 212, column: 15, scope: !164, inlinedAt: !105)
!167 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !169)
!168 = distinct !DISubprogram(name: "load_aligned", linkageName: "_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector12load_aligned", scope: !45, file: !44, line: 212, type: !17, scopeLine: 212, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!169 = distinct !DILocation(line: 175, column: 25, scope: !164, inlinedAt: !105)
!170 = !{!116}
!171 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !172)
!172 = distinct !DILocation(line: 176, column: 45, scope: !173, inlinedAt: !105)
!173 = distinct !DILexicalBlock(scope: !164, file: !50, line: 175, column: 17)
!174 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !175)
!175 = distinct !DILocation(line: 176, column: 25, scope: !173, inlinedAt: !105)
!176 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !177)
!177 = distinct !DILocation(line: 177, column: 45, scope: !178, inlinedAt: !105)
!178 = distinct !DILexicalBlock(scope: !173, file: !50, line: 176, column: 17)
!179 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !180)
!180 = distinct !DILocation(line: 177, column: 25, scope: !178, inlinedAt: !105)
!181 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !182)
!182 = distinct !DILocation(line: 178, column: 45, scope: !183, inlinedAt: !105)
!183 = distinct !DILexicalBlock(scope: !178, file: !50, line: 177, column: 17)
!184 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !185)
!185 = distinct !DILocation(line: 178, column: 25, scope: !183, inlinedAt: !105)
!186 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !187)
!187 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !188)
!188 = distinct !DILocation(line: 179, column: 35, scope: !189, inlinedAt: !105)
!189 = distinct !DILexicalBlock(scope: !183, file: !50, line: 178, column: 17)
!190 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !191)
!191 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !192)
!192 = distinct !DILocation(line: 180, column: 35, scope: !193, inlinedAt: !105)
!193 = distinct !DILexicalBlock(scope: !189, file: !50, line: 179, column: 17)
!194 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !195)
!195 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !196)
!196 = distinct !DILocation(line: 181, column: 35, scope: !197, inlinedAt: !105)
!197 = distinct !DILexicalBlock(scope: !193, file: !50, line: 180, column: 17)
!198 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !199)
!199 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !200)
!200 = distinct !DILocation(line: 182, column: 35, scope: !201, inlinedAt: !105)
!201 = distinct !DILexicalBlock(scope: !197, file: !50, line: 181, column: 17)
!202 = !DILocation(line: 895, column: 14, scope: !203, inlinedAt: !204)
!203 = distinct !DISubprogram(name: "_mm_or_si128", linkageName: "_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse212__mm_or_si128", scope: !40, file: !39, line: 894, type: !107, scopeLine: 894, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!204 = distinct !DILocation(line: 238, column: 13, scope: !205, inlinedAt: !206)
!205 = distinct !DISubprogram(name: "or", linkageName: "_RNvXNtNtCskkIW8vVChzC_6memchr6vector7x86sse2NtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtB4_6Vector2or", scope: !45, file: !44, line: 237, type: !17, scopeLine: 237, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!206 = distinct !DILocation(line: 183, column: 31, scope: !207, inlinedAt: !105)
!207 = distinct !DILexicalBlock(scope: !201, file: !50, line: 182, column: 17)
!208 = !DILocation(line: 895, column: 14, scope: !203, inlinedAt: !209)
!209 = distinct !DILocation(line: 238, column: 13, scope: !205, inlinedAt: !210)
!210 = distinct !DILocation(line: 185, column: 31, scope: !211, inlinedAt: !105)
!211 = distinct !DILexicalBlock(scope: !212, file: !50, line: 184, column: 17)
!212 = distinct !DILexicalBlock(scope: !207, file: !50, line: 183, column: 17)
!213 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !214)
!214 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !215)
!215 = distinct !DILocation(line: 64, column: 14, scope: !216, inlinedAt: !218)
!216 = distinct !DISubprogram(name: "movemask_will_have_non_zero<core::core_arch::x86::__m128i>", linkageName: "_RNvYNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iNtNtCskkIW8vVChzC_6memchr6vector6Vector27movemask_will_have_non_zeroBS_", scope: !217, file: !44, line: 63, type: !107, scopeLine: 63, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!217 = !DINamespace(name: "Vector", scope: !47)
!218 = distinct !DILocation(line: 186, column: 24, scope: !219, inlinedAt: !105)
!219 = distinct !DILexicalBlock(scope: !211, file: !50, line: 185, column: 17)
!220 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !221)
!221 = distinct !DILocation(line: 64, column: 25, scope: !216, inlinedAt: !218)
!222 = !DILocation(line: 186, column: 20, scope: !219, inlinedAt: !105)
!223 = !DILocation(line: 0, scope: !164, inlinedAt: !105)
!224 = !DILocation(line: 223, column: 12, scope: !164, inlinedAt: !105)
!225 = !DILocation(line: 574, column: 14, scope: !91, inlinedAt: !226)
!226 = distinct !DILocation(line: 1341, column: 5, scope: !94, inlinedAt: !227)
!227 = distinct !DILocation(line: 218, column: 13, scope: !97, inlinedAt: !228)
!228 = distinct !DILocation(line: 421, column: 21, scope: !99, inlinedAt: !229)
!229 = distinct !DILocation(line: 214, column: 37, scope: !230, inlinedAt: !105)
!230 = distinct !DILexicalBlock(scope: !164, file: !50, line: 214, column: 62)
!231 = !{!232, !234, !114, !116}
!232 = distinct !{!232, !233, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!233 = distinct !{!233, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!234 = distinct !{!234, !235, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_: argument 0"}
!235 = distinct !{!235, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_"}
!236 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !237)
!237 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !238)
!238 = distinct !DILocation(line: 422, column: 28, scope: !123, inlinedAt: !229)
!239 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !240)
!240 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !241)
!241 = distinct !DILocation(line: 422, column: 41, scope: !123, inlinedAt: !229)
!242 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !243)
!243 = distinct !DILocation(line: 423, column: 17, scope: !135, inlinedAt: !229)
!244 = !DILocation(line: 423, column: 12, scope: !135, inlinedAt: !229)
!245 = !DILocation(line: 624, column: 18, scope: !67, inlinedAt: !246)
!246 = distinct !DILocation(line: 23, column: 30, scope: !74, inlinedAt: !247)
!247 = distinct !DILocation(line: 225, column: 42, scope: !164, inlinedAt: !105)
!248 = !DILocation(line: 1648, column: 9, scope: !80, inlinedAt: !249)
!249 = distinct !DILocation(line: 23, column: 51, scope: !74, inlinedAt: !247)
!250 = !{!114}
!251 = !DILocation(line: 956, column: 22, scope: !252, inlinedAt: !253)
!252 = distinct !DISubprogram(name: "sub<u8>", linkageName: "_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh3subCskkIW8vVChzC_6memchr", scope: !70, file: !68, line: 917, type: !17, scopeLine: 917, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!253 = distinct !DILocation(line: 225, column: 23, scope: !164, inlinedAt: !105)
!254 = !DILocation(line: 574, column: 14, scope: !91, inlinedAt: !255)
!255 = distinct !DILocation(line: 1341, column: 5, scope: !94, inlinedAt: !256)
!256 = distinct !DILocation(line: 218, column: 13, scope: !97, inlinedAt: !257)
!257 = distinct !DILocation(line: 421, column: 21, scope: !99, inlinedAt: !258)
!258 = distinct !DILocation(line: 227, column: 25, scope: !164, inlinedAt: !105)
!259 = !{!260, !262, !114, !116}
!260 = distinct !{!260, !261, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!261 = distinct !{!261, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!262 = distinct !{!262, !263, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_: argument 0"}
!263 = distinct !{!263, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_"}
!264 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !265)
!265 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !266)
!266 = distinct !DILocation(line: 422, column: 28, scope: !123, inlinedAt: !258)
!267 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !268)
!268 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !269)
!269 = distinct !DILocation(line: 422, column: 41, scope: !123, inlinedAt: !258)
!270 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !271)
!271 = distinct !DILocation(line: 423, column: 17, scope: !135, inlinedAt: !258)
!272 = !DILocation(line: 423, column: 12, scope: !135, inlinedAt: !258)
!273 = !DILocation(line: 0, scope: !274, inlinedAt: !105)
!274 = !DILexicalBlockFile(scope: !164, file: !157, discriminator: 0)
!275 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !276)
!276 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !277)
!277 = distinct !DILocation(line: 79, column: 5, scope: !145, inlinedAt: !278)
!278 = distinct !DILocation(line: 424, column: 26, scope: !135, inlinedAt: !229)
!279 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !277)
!280 = !DILocation(line: 872, column: 18, scope: !153, inlinedAt: !281)
!281 = distinct !DILocation(line: 424, column: 22, scope: !135, inlinedAt: !229)
!282 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !283)
!283 = distinct !DILocation(line: 217, column: 23, scope: !164, inlinedAt: !105)
!284 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !285)
!285 = distinct !DILocation(line: 206, column: 27, scope: !219, inlinedAt: !105)
!286 = !DILocation(line: 172, column: 19, scope: !164, inlinedAt: !105)
!287 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !288)
!288 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !289)
!289 = distinct !DILocation(line: 187, column: 36, scope: !219, inlinedAt: !105)
!290 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !291)
!291 = distinct !DILocation(line: 188, column: 29, scope: !292, inlinedAt: !105)
!292 = distinct !DILexicalBlock(scope: !219, file: !50, line: 187, column: 21)
!293 = !DILocation(line: 188, column: 24, scope: !292, inlinedAt: !105)
!294 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !295)
!295 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !296)
!296 = distinct !DILocation(line: 192, column: 36, scope: !292, inlinedAt: !105)
!297 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !298)
!298 = distinct !DILocation(line: 193, column: 29, scope: !299, inlinedAt: !105)
!299 = distinct !DILexicalBlock(scope: !292, file: !50, line: 192, column: 21)
!300 = !DILocation(line: 193, column: 24, scope: !299, inlinedAt: !105)
!301 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !302)
!302 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !303)
!303 = distinct !DILocation(line: 189, column: 45, scope: !292, inlinedAt: !105)
!304 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !303)
!305 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !306)
!306 = distinct !DILocation(line: 189, column: 41, scope: !292, inlinedAt: !105)
!307 = !DILocation(line: 0, scope: !308, inlinedAt: !105)
!308 = !DILexicalBlockFile(scope: !292, file: !157, discriminator: 0)
!309 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !310)
!310 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !311)
!311 = distinct !DILocation(line: 197, column: 36, scope: !299, inlinedAt: !105)
!312 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !313)
!313 = distinct !DILocation(line: 198, column: 29, scope: !314, inlinedAt: !105)
!314 = distinct !DILexicalBlock(scope: !299, file: !50, line: 197, column: 21)
!315 = !DILocation(line: 198, column: 24, scope: !314, inlinedAt: !105)
!316 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !317)
!317 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !318)
!318 = distinct !DILocation(line: 194, column: 63, scope: !299, inlinedAt: !105)
!319 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !318)
!320 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !321)
!321 = distinct !DILocation(line: 194, column: 59, scope: !299, inlinedAt: !105)
!322 = !DILocation(line: 0, scope: !323, inlinedAt: !105)
!323 = !DILexicalBlockFile(scope: !299, file: !157, discriminator: 0)
!324 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !325)
!325 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !326)
!326 = distinct !DILocation(line: 202, column: 36, scope: !314, inlinedAt: !105)
!327 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !328)
!328 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !329)
!329 = distinct !DILocation(line: 204, column: 59, scope: !330, inlinedAt: !105)
!330 = distinct !DILexicalBlock(scope: !314, file: !50, line: 202, column: 21)
!331 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !329)
!332 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !333)
!333 = distinct !DILocation(line: 204, column: 55, scope: !330, inlinedAt: !105)
!334 = !DILocation(line: 0, scope: !335, inlinedAt: !105)
!335 = !DILexicalBlockFile(scope: !314, file: !157, discriminator: 0)
!336 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !337)
!337 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !338)
!338 = distinct !DILocation(line: 199, column: 63, scope: !314, inlinedAt: !105)
!339 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !338)
!340 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !341)
!341 = distinct !DILocation(line: 199, column: 59, scope: !314, inlinedAt: !105)
!342 = !DILocation(line: 1162, column: 20, scope: !87, inlinedAt: !89)
!343 = !DILocation(line: 167, column: 17, scope: !344, inlinedAt: !347)
!344 = distinct !DISubprogram(name: "{closure#0}", linkageName: "_RNCNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB4_3One8find_raw0Bc_", scope: !345, file: !56, line: 166, type: !107, scopeLine: 166, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!345 = !DINamespace(name: "find_raw", scope: !346)
!346 = !DINamespace(name: "{impl#0}", scope: !58)
!347 = distinct !DILocation(line: 1162, column: 12, scope: !87, inlinedAt: !89)
!348 = !DILocation(line: 1162, column: 12, scope: !87, inlinedAt: !89)
!349 = !DILocation(line: 390, column: 18, scope: !350, inlinedAt: !351)
!350 = distinct !DISubprogram(name: "offset<u8>", linkageName: "_RNvMNtNtCs2k2z8Zem4rB_4core3ptr9const_ptrPh6offsetCskkIW8vVChzC_6memchr", scope: !70, file: !68, line: 355, type: !17, scopeLine: 355, flags: DIFlagPrototyped, spFlags: DISPFlagLocalToUnit | DISPFlagDefinition | DISPFlagOptimized, unit: !8, templateParams: !18)
!351 = distinct !DILocation(line: 1165, column: 19, scope: !87, inlinedAt: !89)
!352 = !DILocation(line: 0, scope: !64, inlinedAt: !65)
!353 = !DILocation(line: 186, column: 6, scope: !64, inlinedAt: !65)
!354 = !DILocation(line: 145, column: 10, scope: !10)
!355 = !{i64 3352607776364591725}
!356 = !DILocation(line: 59, column: 18, scope: !32, inlinedAt: !357)
!357 = distinct !DILocation(line: 1221, column: 5, scope: !38, inlinedAt: !358)
!358 = distinct !DILocation(line: 208, column: 13, scope: !43, inlinedAt: !359)
!359 = distinct !DILocation(line: 112, column: 31, scope: !49, inlinedAt: !360)
!360 = distinct !DILocation(line: 64, column: 13, scope: !55, inlinedAt: !361)
!361 = !DILocation(line: 96, column: 13, scope: !61)
!362 = !DILocation(line: 161, column: 12, scope: !64, inlinedAt: !363)
!363 = distinct !DILocation(line: 97, column: 18, scope: !61)
!364 = !DILocation(line: 624, column: 18, scope: !67, inlinedAt: !365)
!365 = distinct !DILocation(line: 23, column: 30, scope: !74, inlinedAt: !366)
!366 = distinct !DILocation(line: 164, column: 16, scope: !64, inlinedAt: !363)
!367 = !DILocation(line: 1648, column: 9, scope: !80, inlinedAt: !368)
!368 = distinct !DILocation(line: 23, column: 51, scope: !74, inlinedAt: !366)
!369 = !DILocation(line: 164, column: 12, scope: !64, inlinedAt: !363)
!370 = !DILocation(line: 1161, column: 11, scope: !87, inlinedAt: !371)
!371 = distinct !DILocation(line: 166, column: 20, scope: !64, inlinedAt: !363)
!372 = !DILocation(line: 574, column: 14, scope: !91, inlinedAt: !373)
!373 = distinct !DILocation(line: 1341, column: 5, scope: !94, inlinedAt: !374)
!374 = distinct !DILocation(line: 218, column: 13, scope: !97, inlinedAt: !375)
!375 = distinct !DILocation(line: 421, column: 21, scope: !99, inlinedAt: !376)
!376 = distinct !DILocation(line: 165, column: 33, scope: !101, inlinedAt: !377)
!377 = distinct !DILocation(line: 290, column: 16, scope: !106, inlinedAt: !378)
!378 = distinct !DILocation(line: 185, column: 14, scope: !64, inlinedAt: !363)
!379 = !{!380, !382, !384, !386}
!380 = distinct !{!380, !381, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!381 = distinct !{!381, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!382 = distinct !{!382, !383, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_: argument 0"}
!383 = distinct !{!383, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_"}
!384 = distinct !{!384, !385, !"_RNvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB2_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE8find_rawB8_: argument 0"}
!385 = distinct !{!385, !"_RNvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB2_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE8find_rawB8_"}
!386 = distinct !{!386, !387, !"_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One8find_raw: argument 0"}
!387 = distinct !{!387, !"_RNvMNtNtNtNtCskkIW8vVChzC_6memchr4arch6x86_644sse26memchrNtB2_3One8find_raw"}
!388 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !389)
!389 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !390)
!390 = distinct !DILocation(line: 422, column: 28, scope: !123, inlinedAt: !376)
!391 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !392)
!392 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !393)
!393 = distinct !DILocation(line: 422, column: 41, scope: !123, inlinedAt: !376)
!394 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !395)
!395 = distinct !DILocation(line: 423, column: 17, scope: !135, inlinedAt: !376)
!396 = !DILocation(line: 423, column: 12, scope: !135, inlinedAt: !376)
!397 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !398)
!398 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !399)
!399 = distinct !DILocation(line: 79, column: 5, scope: !145, inlinedAt: !400)
!400 = distinct !DILocation(line: 424, column: 26, scope: !135, inlinedAt: !376)
!401 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !399)
!402 = !DILocation(line: 872, column: 18, scope: !153, inlinedAt: !403)
!403 = distinct !DILocation(line: 424, column: 22, scope: !135, inlinedAt: !376)
!404 = !DILocation(line: 0, scope: !156, inlinedAt: !377)
!405 = !DILocation(line: 169, column: 44, scope: !102, inlinedAt: !377)
!406 = !DILocation(line: 169, column: 33, scope: !102, inlinedAt: !377)
!407 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !408)
!408 = distinct !DILocation(line: 169, column: 29, scope: !102, inlinedAt: !377)
!409 = !DILocation(line: 171, column: 12, scope: !164, inlinedAt: !377)
!410 = !DILocation(line: 0, scope: !102, inlinedAt: !377)
!411 = !DILocation(line: 212, column: 15, scope: !164, inlinedAt: !377)
!412 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !413)
!413 = distinct !DILocation(line: 175, column: 25, scope: !164, inlinedAt: !377)
!414 = !{!386}
!415 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !416)
!416 = distinct !DILocation(line: 176, column: 45, scope: !173, inlinedAt: !377)
!417 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !418)
!418 = distinct !DILocation(line: 176, column: 25, scope: !173, inlinedAt: !377)
!419 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !420)
!420 = distinct !DILocation(line: 177, column: 45, scope: !178, inlinedAt: !377)
!421 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !422)
!422 = distinct !DILocation(line: 177, column: 25, scope: !178, inlinedAt: !377)
!423 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !424)
!424 = distinct !DILocation(line: 178, column: 45, scope: !183, inlinedAt: !377)
!425 = !DILocation(line: 213, column: 13, scope: !168, inlinedAt: !426)
!426 = distinct !DILocation(line: 178, column: 25, scope: !183, inlinedAt: !377)
!427 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !428)
!428 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !429)
!429 = distinct !DILocation(line: 179, column: 35, scope: !189, inlinedAt: !377)
!430 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !431)
!431 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !432)
!432 = distinct !DILocation(line: 180, column: 35, scope: !193, inlinedAt: !377)
!433 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !434)
!434 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !435)
!435 = distinct !DILocation(line: 181, column: 35, scope: !197, inlinedAt: !377)
!436 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !437)
!437 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !438)
!438 = distinct !DILocation(line: 182, column: 35, scope: !201, inlinedAt: !377)
!439 = !DILocation(line: 895, column: 14, scope: !203, inlinedAt: !440)
!440 = distinct !DILocation(line: 238, column: 13, scope: !205, inlinedAt: !441)
!441 = distinct !DILocation(line: 183, column: 31, scope: !207, inlinedAt: !377)
!442 = !DILocation(line: 895, column: 14, scope: !203, inlinedAt: !443)
!443 = distinct !DILocation(line: 238, column: 13, scope: !205, inlinedAt: !444)
!444 = distinct !DILocation(line: 185, column: 31, scope: !211, inlinedAt: !377)
!445 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !446)
!446 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !447)
!447 = distinct !DILocation(line: 64, column: 14, scope: !216, inlinedAt: !448)
!448 = distinct !DILocation(line: 186, column: 24, scope: !219, inlinedAt: !377)
!449 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !450)
!450 = distinct !DILocation(line: 64, column: 25, scope: !216, inlinedAt: !448)
!451 = !DILocation(line: 186, column: 20, scope: !219, inlinedAt: !377)
!452 = !DILocation(line: 0, scope: !164, inlinedAt: !377)
!453 = !DILocation(line: 223, column: 12, scope: !164, inlinedAt: !377)
!454 = !DILocation(line: 574, column: 14, scope: !91, inlinedAt: !455)
!455 = distinct !DILocation(line: 1341, column: 5, scope: !94, inlinedAt: !456)
!456 = distinct !DILocation(line: 218, column: 13, scope: !97, inlinedAt: !457)
!457 = distinct !DILocation(line: 421, column: 21, scope: !99, inlinedAt: !458)
!458 = distinct !DILocation(line: 214, column: 37, scope: !230, inlinedAt: !377)
!459 = !{!460, !462, !384, !386}
!460 = distinct !{!460, !461, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!461 = distinct !{!461, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!462 = distinct !{!462, !463, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_: argument 0"}
!463 = distinct !{!463, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_"}
!464 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !465)
!465 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !466)
!466 = distinct !DILocation(line: 422, column: 28, scope: !123, inlinedAt: !458)
!467 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !468)
!468 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !469)
!469 = distinct !DILocation(line: 422, column: 41, scope: !123, inlinedAt: !458)
!470 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !471)
!471 = distinct !DILocation(line: 423, column: 17, scope: !135, inlinedAt: !458)
!472 = !DILocation(line: 423, column: 12, scope: !135, inlinedAt: !458)
!473 = !DILocation(line: 624, column: 18, scope: !67, inlinedAt: !474)
!474 = distinct !DILocation(line: 23, column: 30, scope: !74, inlinedAt: !475)
!475 = distinct !DILocation(line: 225, column: 42, scope: !164, inlinedAt: !377)
!476 = !DILocation(line: 1648, column: 9, scope: !80, inlinedAt: !477)
!477 = distinct !DILocation(line: 23, column: 51, scope: !74, inlinedAt: !475)
!478 = !{!384}
!479 = !DILocation(line: 956, column: 22, scope: !252, inlinedAt: !480)
!480 = distinct !DILocation(line: 225, column: 23, scope: !164, inlinedAt: !377)
!481 = !DILocation(line: 574, column: 14, scope: !91, inlinedAt: !482)
!482 = distinct !DILocation(line: 1341, column: 5, scope: !94, inlinedAt: !483)
!483 = distinct !DILocation(line: 218, column: 13, scope: !97, inlinedAt: !484)
!484 = distinct !DILocation(line: 421, column: 21, scope: !99, inlinedAt: !485)
!485 = distinct !DILocation(line: 227, column: 25, scope: !164, inlinedAt: !377)
!486 = !{!487, !489, !384, !386}
!487 = distinct !{!487, !488, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128: argument 0"}
!488 = distinct !{!488, !"_RNvNtNtNtCs2k2z8Zem4rB_4core9core_arch3x864sse215__mm_loadu_si128"}
!489 = distinct !{!489, !490, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_: argument 0"}
!490 = distinct !{!490, !"_RINvMNtNtNtCskkIW8vVChzC_6memchr4arch7generic6memchrINtB3_3OneNtNtNtCs2k2z8Zem4rB_4core9core_arch3x867___m128iE12search_chunkNvYNtNtB9_6vector16SensibleMoveMaskNtB24_8MoveMask12first_offsetEB9_"}
!491 = !DILocation(line: 920, column: 36, scope: !119, inlinedAt: !492)
!492 = distinct !DILocation(line: 228, column: 13, scope: !121, inlinedAt: !493)
!493 = distinct !DILocation(line: 422, column: 28, scope: !123, inlinedAt: !485)
!494 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !495)
!495 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !496)
!496 = distinct !DILocation(line: 422, column: 41, scope: !123, inlinedAt: !485)
!497 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !498)
!498 = distinct !DILocation(line: 423, column: 17, scope: !135, inlinedAt: !485)
!499 = !DILocation(line: 423, column: 12, scope: !135, inlinedAt: !485)
!500 = !DILocation(line: 0, scope: !274, inlinedAt: !377)
!501 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !502)
!502 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !503)
!503 = distinct !DILocation(line: 79, column: 5, scope: !145, inlinedAt: !504)
!504 = distinct !DILocation(line: 424, column: 26, scope: !135, inlinedAt: !458)
!505 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !503)
!506 = !DILocation(line: 872, column: 18, scope: !153, inlinedAt: !507)
!507 = distinct !DILocation(line: 424, column: 22, scope: !135, inlinedAt: !458)
!508 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !509)
!509 = distinct !DILocation(line: 217, column: 23, scope: !164, inlinedAt: !377)
!510 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !511)
!511 = distinct !DILocation(line: 206, column: 27, scope: !219, inlinedAt: !377)
!512 = !DILocation(line: 172, column: 19, scope: !164, inlinedAt: !377)
!513 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !514)
!514 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !515)
!515 = distinct !DILocation(line: 187, column: 36, scope: !219, inlinedAt: !377)
!516 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !517)
!517 = distinct !DILocation(line: 188, column: 29, scope: !292, inlinedAt: !377)
!518 = !DILocation(line: 188, column: 24, scope: !292, inlinedAt: !377)
!519 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !520)
!520 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !521)
!521 = distinct !DILocation(line: 192, column: 36, scope: !292, inlinedAt: !377)
!522 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !523)
!523 = distinct !DILocation(line: 193, column: 29, scope: !299, inlinedAt: !377)
!524 = !DILocation(line: 193, column: 24, scope: !299, inlinedAt: !377)
!525 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !526)
!526 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !527)
!527 = distinct !DILocation(line: 189, column: 45, scope: !292, inlinedAt: !377)
!528 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !527)
!529 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !530)
!530 = distinct !DILocation(line: 189, column: 41, scope: !292, inlinedAt: !377)
!531 = !DILocation(line: 0, scope: !308, inlinedAt: !377)
!532 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !533)
!533 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !534)
!534 = distinct !DILocation(line: 197, column: 36, scope: !299, inlinedAt: !377)
!535 = !DILocation(line: 147, column: 9, scope: !132, inlinedAt: !536)
!536 = distinct !DILocation(line: 198, column: 29, scope: !314, inlinedAt: !377)
!537 = !DILocation(line: 198, column: 24, scope: !314, inlinedAt: !377)
!538 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !539)
!539 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !540)
!540 = distinct !DILocation(line: 194, column: 63, scope: !299, inlinedAt: !377)
!541 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !540)
!542 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !543)
!543 = distinct !DILocation(line: 194, column: 59, scope: !299, inlinedAt: !377)
!544 = !DILocation(line: 0, scope: !323, inlinedAt: !377)
!545 = !DILocation(line: 1570, column: 9, scope: !125, inlinedAt: !546)
!546 = distinct !DILocation(line: 223, column: 30, scope: !129, inlinedAt: !547)
!547 = distinct !DILocation(line: 202, column: 36, scope: !314, inlinedAt: !377)
!548 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !549)
!549 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !550)
!550 = distinct !DILocation(line: 204, column: 59, scope: !330, inlinedAt: !377)
!551 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !550)
!552 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !553)
!553 = distinct !DILocation(line: 204, column: 55, scope: !330, inlinedAt: !377)
!554 = !DILocation(line: 0, scope: !335, inlinedAt: !377)
!555 = !DILocation(line: 178, column: 20, scope: !138, inlinedAt: !556)
!556 = distinct !DILocation(line: 178, column: 31, scope: !143, inlinedAt: !557)
!557 = distinct !DILocation(line: 199, column: 63, scope: !314, inlinedAt: !377)
!558 = !DILocation(line: 178, column: 9, scope: !143, inlinedAt: !557)
!559 = !DILocation(line: 872, column: 18, scope: !161, inlinedAt: !560)
!560 = distinct !DILocation(line: 199, column: 59, scope: !314, inlinedAt: !377)
!561 = !DILocation(line: 1162, column: 20, scope: !87, inlinedAt: !371)
!562 = !DILocation(line: 167, column: 17, scope: !344, inlinedAt: !563)
!563 = distinct !DILocation(line: 1162, column: 12, scope: !87, inlinedAt: !371)
!564 = !DILocation(line: 1162, column: 12, scope: !87, inlinedAt: !371)
!565 = !DILocation(line: 390, column: 18, scope: !350, inlinedAt: !566)
!566 = distinct !DILocation(line: 1165, column: 19, scope: !87, inlinedAt: !371)
!567 = !DILocation(line: 0, scope: !64, inlinedAt: !363)
!568 = !DILocation(line: 186, column: 6, scope: !64, inlinedAt: !363)
!569 = !DILocation(line: 98, column: 10, scope: !61)

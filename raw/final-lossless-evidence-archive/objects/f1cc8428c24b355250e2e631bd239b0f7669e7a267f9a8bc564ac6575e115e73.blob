define internal fastcc noundef align 8 ptr @purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#0}(ptr noundef nonnull align 8 %0, ptr nofree nonnull readonly captures(none) %1) unnamed_addr #6 personality ptr @rust_eh_personality !guid !67384 {
  %3 = ptrtoint ptr %0 to i64
  tail call void @llvm.experimental.noalias.scope.decl(metadata !67385)
  %4 = getelementptr inbounds nuw i8, ptr %1, i64 40
  %5 = load i64, ptr %4, align 8, !alias.scope !67385, !noundef !1740
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %55, label %7

7:                                                ; preds = %2
  %8 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %9 = xor i64 %3, 2746377873070565055
  %10 = insertelement <2 x i64> <i64 poison, i64 8385202752464708517>, i64 %9, i64 0
  %11 = tail call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %10, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %12 = tail call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %11, <2 x i64> <i64 2770419649501688277, i64 4912192508662405875>)
  %13 = tail call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %12, <2 x i64> <i64 4028393671950310427, i64 7405497087257227243>)
  %14 = extractelement <2 x i64> %13, i64 0
  tail call void @llvm.experimental.noalias.scope.decl(metadata !67388)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !67391)
  %15 = lshr i64 %14, 57
  %16 = trunc nuw nsw i64 %15 to i8
  %17 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %18 = load i64, ptr %17, align 8, !alias.scope !67394, !noalias !67395, !noundef !1740
  %19 = load ptr, ptr %8, align 8, !alias.scope !67394, !noalias !67395, !nonnull !1740, !noundef !1740
  %20 = insertelement <16 x i8> poison, i8 %16, i64 0
  %21 = shufflevector <16 x i8> %20, <16 x i8> poison, <16 x i32> zeroinitializer
  br label %22

22:                                               ; preds = %48, %7
  %23 = phi i64 [ 0, %7 ], [ %49, %48 ]
  %24 = phi i64 [ %14, %7 ], [ %50, %48 ]
  %25 = and i64 %24, %18
  %26 = getelementptr inbounds nuw i8, ptr %19, i64 %25
  %27 = load <16 x i8>, ptr %26, align 1, !noalias !67397
  %28 = icmp eq <16 x i8> %27, %21
  %29 = bitcast <16 x i1> %28 to i16
  %30 = icmp eq i16 %29, 0
  br i1 %30, label %.loopexit2, label %.preheader

.preheader:                                       ; preds = %22, %44
  %31 = phi i16 [ %46, %44 ], [ %29, %22 ]
  %32 = tail call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %31, i1 true)
  %33 = zext nneg i16 %32 to i64
  %34 = add i64 %25, %33
  %35 = and i64 %34, %18
  %36 = sub nsw i64 0, %35
  %37 = getelementptr inbounds [40 x i8], ptr %19, i64 %36
  %38 = getelementptr inbounds i8, ptr %37, i64 -40
  %39 = load i64, ptr %38, align 8, !noalias !67400, !noundef !1740
  %40 = icmp eq i64 %39, %3
  br i1 %40, label %.loopexit, label %44, !prof !1953

.loopexit2:                                       ; preds = %44, %22
  %41 = icmp eq <16 x i8> %27, splat (i8 -1)
  %42 = bitcast <16 x i1> %41 to i16
  %43 = icmp eq i16 %42, 0
  br i1 %43, label %48, label %.loopexit, !prof !1742

44:                                               ; preds = %.preheader
  %45 = add i16 %31, -1
  %46 = and i16 %45, %31
  %47 = icmp eq i16 %46, 0
  br i1 %47, label %.loopexit2, label %.preheader

48:                                               ; preds = %.loopexit2
  %49 = add i64 %23, 16
  %50 = add i64 %25, %49
  br label %22

.loopexit:                                        ; preds = %.loopexit2, %.preheader
  %51 = phi ptr [ %37, %.preheader ], [ null, %.loopexit2 ]
  %52 = icmp eq ptr %51, null
  %53 = getelementptr inbounds i8, ptr %51, i64 -32
  %54 = select i1 %52, ptr null, ptr %53
  br label %55

55:                                               ; preds = %.loopexit, %2
  %56 = phi ptr [ %54, %.loopexit ], [ null, %2 ]
  ret ptr %56
}

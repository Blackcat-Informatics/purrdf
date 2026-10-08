define internal fastcc void @purrdf_sparql_eval::eval::eval_node::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef nonnull writable align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) unnamed_addr #2 !guid !32862 {
  %4 = alloca [96 x i8], align 16
  %5 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %6 = alloca [96 x i8], align 16
  %7 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %8 = alloca [96 x i8], align 16
  %9 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %10 = load i32, ptr %1, align 8, !range !2161, !noundef !1708
  %11 = icmp ne i32 %10, 11
  tail call void @llvm.assume(i1 %11)
  %12 = add nsw i32 %10, -10
  %13 = icmp samesign ugt i32 %10, 9
  %14 = select i1 %13, i32 %12, i32 1
  switch i32 %14, label %15 [
    i32 0, label %16
    i32 1, label %23
    i32 2, label %28
    i32 3, label %32
    i32 4, label %36
    i32 5, label %40
    i32 6, label %44
    i32 7, label %49
    i32 8, label %53
    i32 9, label %57
    i32 10, label %61
    i32 11, label %65
    i32 12, label %76
    i32 13, label %80
    i32 14, label %183
    i32 15, label %183
    i32 16, label %84
    i32 17, label %88
    i32 18, label %92
    i32 19, label %94
  ]

15:                                               ; preds = %3
  unreachable

16:                                               ; preds = %3
  %17 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %18 = load ptr, ptr %17, align 8, !nonnull !1708, !noundef !1708
  %19 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %20 = load i64, ptr %19, align 8, !noundef !1708
; call purrdf_sparql_eval::bgp::eval_bgp::<purrdf_core::ir::dataset::RdfDataset>
  call void @purrdf_sparql_eval::bgp::eval_bgp::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([96 x i8]) align 16 captures(address) dereferenceable(96) %8, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %18, i64 noundef %20, ptr noundef nonnull align 16 %2) #87
  %21 = load i64, ptr %8, align 16, !range !2530, !noundef !1708
  %22 = icmp eq i64 %21, -1
  br i1 %22, label %103, label %98

23:                                               ; preds = %3
  %24 = getelementptr inbounds nuw i8, ptr %1, i64 32
  %25 = getelementptr inbounds nuw i8, ptr %1, i64 88
; call purrdf_sparql_eval::path::eval_path::<purrdf_core::ir::dataset::RdfDataset>
  call void @purrdf_sparql_eval::path::eval_path::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([96 x i8]) align 16 captures(none) dereferenceable(96) %6, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(32) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %25, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  %26 = load i64, ptr %6, align 16, !range !2530, !noundef !1708
  %27 = icmp eq i64 %26, -1
  br i1 %27, label %112, label %107

28:                                               ; preds = %3
  %29 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %30 = load ptr, ptr %29, align 8, !align !1818, !noundef !1708
  %31 = icmp eq ptr %30, null
  br i1 %31, label %119, label %115, !prof !1803

32:                                               ; preds = %3
  %33 = getelementptr inbounds nuw i8, ptr %1, i64 72
  %34 = load ptr, ptr %33, align 8, !align !1818, !noundef !1708
  %35 = icmp eq ptr %34, null
  br i1 %35, label %126, label %122, !prof !1803

36:                                               ; preds = %3
  %37 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %38 = load ptr, ptr %37, align 8, !align !1818, !noundef !1708
  %39 = icmp eq ptr %38, null
  br i1 %39, label %137, label %133, !prof !1803

40:                                               ; preds = %3
  %41 = getelementptr inbounds nuw i8, ptr %1, i64 72
  %42 = load ptr, ptr %41, align 8, !align !1818, !noundef !1708
  %43 = icmp eq ptr %42, null
  br i1 %43, label %142, label %140, !prof !1803

44:                                               ; preds = %3
  %45 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %46 = load ptr, ptr %45, align 8, !nonnull !1708, !noundef !1708
  %47 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %48 = load i64, ptr %47, align 8, !noundef !1708
; call purrdf_sparql_eval::binop::eval_union::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::binop::eval_union::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(address) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %46, i64 noundef %48, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

49:                                               ; preds = %3
  %50 = getelementptr inbounds nuw i8, ptr %1, i64 32
  %51 = load ptr, ptr %50, align 8, !align !1818, !noundef !1708
  %52 = icmp eq ptr %51, null
  br i1 %52, label %145, label %143, !prof !1803

53:                                               ; preds = %3
  %54 = getelementptr inbounds nuw i8, ptr %1, i64 88
  %55 = load ptr, ptr %54, align 8, !align !1818, !noundef !1708
  %56 = icmp eq ptr %55, null
  br i1 %56, label %149, label %146, !prof !1803

57:                                               ; preds = %3
  %58 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %59 = load ptr, ptr %58, align 8, !align !1818, !noundef !1708
  %60 = icmp eq ptr %59, null
  br i1 %60, label %154, label %150, !prof !1803

61:                                               ; preds = %3
  %62 = getelementptr inbounds nuw i8, ptr %1, i64 32
  %63 = load ptr, ptr %62, align 8, !align !1818, !noundef !1708
  %64 = icmp eq ptr %63, null
  br i1 %64, label %162, label %157, !prof !1803

65:                                               ; preds = %3
  %66 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %67 = load ptr, ptr %66, align 8, !nonnull !1708, !noundef !1708
  %68 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %69 = load i64, ptr %68, align 8, !noundef !1708
  %70 = getelementptr inbounds nuw i8, ptr %1, i64 40
  %71 = load ptr, ptr %70, align 8, !nonnull !1708, !noundef !1708
  %72 = getelementptr inbounds nuw i8, ptr %1, i64 48
  %73 = load i64, ptr %72, align 8, !noundef !1708
; call purrdf_sparql_eval::modifier::eval_values::<purrdf_core::ir::dataset::RdfDataset>
  call void @purrdf_sparql_eval::modifier::eval_values::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([96 x i8]) align 16 captures(none) dereferenceable(96) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %67, i64 noundef %69, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %71, i64 noundef %73, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  %74 = load i64, ptr %4, align 16, !range !2530, !noundef !1708
  %75 = icmp eq i64 %74, -1
  br i1 %75, label %168, label %163

76:                                               ; preds = %3
  %77 = getelementptr inbounds nuw i8, ptr %1, i64 32
  %78 = load ptr, ptr %77, align 8, !align !1818, !noundef !1708
  %79 = icmp eq ptr %78, null
  br i1 %79, label %176, label %171, !prof !1803

80:                                               ; preds = %3
  %81 = getelementptr inbounds nuw i8, ptr %1, i64 32
  %82 = load ptr, ptr %81, align 8, !align !1818, !noundef !1708
  %83 = icmp eq ptr %82, null
  br i1 %83, label %182, label %177, !prof !1803

84:                                               ; preds = %3
  %85 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %86 = load ptr, ptr %85, align 8, !align !1818, !noundef !1708
  %87 = icmp eq ptr %86, null
  br i1 %87, label %196, label %189, !prof !1803

88:                                               ; preds = %3
  %89 = getelementptr inbounds nuw i8, ptr %1, i64 56
  %90 = load ptr, ptr %89, align 8, !align !1818, !noundef !1708
  %91 = icmp eq ptr %90, null
  br i1 %91, label %206, label %197, !prof !1803

92:                                               ; preds = %3
  %93 = getelementptr inbounds nuw i8, ptr %1, i64 8
; call purrdf_sparql_eval::property_fn_eval::eval_property_function::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::property_fn_eval::eval_property_function::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(72) %93, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

94:                                               ; preds = %3
  %95 = getelementptr inbounds nuw i8, ptr %1, i64 88
  %96 = load ptr, ptr %95, align 8, !align !1818, !noundef !1708
  %97 = icmp eq ptr %96, null
  br i1 %97, label %214, label %207, !prof !1803

98:                                               ; preds = %16
  %99 = getelementptr inbounds nuw i8, ptr %8, i64 40
  %100 = getelementptr inbounds nuw i8, ptr %0, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %100, ptr noundef nonnull align 8 dereferenceable(56) %99, i64 56, i1 false)
  %101 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %101, ptr noundef nonnull align 8 dereferenceable(32) %9, i64 32, i1 false)
  %102 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %21, ptr %102, align 16
  store i64 1, ptr %0, align 16
  br label %106

103:                                              ; preds = %16
  %104 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(32) %104, ptr noundef nonnull align 8 dereferenceable(32) %9, i64 32, i1 false)
  %105 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 -1, ptr %105, align 8
  store i64 0, ptr %0, align 16
  br label %106

106:                                              ; preds = %207, %197, %189, %187, %177, %171, %168, %163, %157, %155, %146, %143, %140, %138, %127, %120, %112, %107, %103, %98, %92, %44
  ret void

107:                                              ; preds = %23
  %108 = getelementptr inbounds nuw i8, ptr %6, i64 40
  %109 = getelementptr inbounds nuw i8, ptr %0, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %109, ptr noundef nonnull align 8 dereferenceable(56) %108, i64 56, i1 false)
  %110 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %110, ptr noundef nonnull align 8 dereferenceable(32) %7, i64 32, i1 false)
  %111 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %26, ptr %111, align 16
  store i64 1, ptr %0, align 16
  br label %106

112:                                              ; preds = %23
  %113 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(32) %113, ptr noundef nonnull align 8 dereferenceable(32) %7, i64 32, i1 false)
  %114 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 -1, ptr %114, align 8
  store i64 0, ptr %0, align 16
  br label %106

115:                                              ; preds = %28
  %116 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %117 = load ptr, ptr %116, align 8, !align !1818, !noundef !1708
  %118 = icmp eq ptr %117, null
  br i1 %118, label %121, label %120, !prof !1803

119:                                              ; preds = %28
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

120:                                              ; preds = %115
; call purrdf_sparql_eval::binop::eval_join::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::binop::eval_join::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(address) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %30, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %117, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

121:                                              ; preds = %115
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

122:                                              ; preds = %32
  %123 = getelementptr inbounds nuw i8, ptr %1, i64 80
  %124 = load ptr, ptr %123, align 8, !align !1818, !noundef !1708
  %125 = icmp eq ptr %124, null
  br i1 %125, label %132, label %127, !prof !1803

126:                                              ; preds = %32
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

127:                                              ; preds = %122
  %128 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %129 = load i64, ptr %128, align 8, !range !20277, !noundef !1708
  %130 = icmp eq i64 %129, -1
  %131 = select i1 %130, ptr null, ptr %128
; call purrdf_sparql_eval::binop::eval_left_join::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::binop::eval_left_join::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(address) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %34, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %124, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(64) %131, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

132:                                              ; preds = %122
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

133:                                              ; preds = %36
  %134 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %135 = load ptr, ptr %134, align 8, !align !1818, !noundef !1708
  %136 = icmp eq ptr %135, null
  br i1 %136, label %139, label %138, !prof !1803

137:                                              ; preds = %36
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

138:                                              ; preds = %133
; call purrdf_sparql_eval::binop::eval_lateral::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::binop::eval_lateral::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %38, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %135, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

139:                                              ; preds = %133
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

140:                                              ; preds = %40
  %141 = getelementptr inbounds nuw i8, ptr %1, i64 8
; call purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %141, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %42, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

142:                                              ; preds = %40
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

143:                                              ; preds = %49
  %144 = getelementptr inbounds nuw i8, ptr %1, i64 8
; call purrdf_sparql_eval::modifier::eval_graph::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::modifier::eval_graph::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(address) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) %144, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %51, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

145:                                              ; preds = %49
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

146:                                              ; preds = %53
  %147 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %148 = getelementptr inbounds nuw i8, ptr %1, i64 72
; call purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %55, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %148, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %147, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

149:                                              ; preds = %53
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

150:                                              ; preds = %57
  %151 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %152 = load ptr, ptr %151, align 8, !align !1818, !noundef !1708
  %153 = icmp eq ptr %152, null
  br i1 %153, label %156, label %155, !prof !1803

154:                                              ; preds = %57
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

155:                                              ; preds = %150
; call purrdf_sparql_eval::binop::eval_minus::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::binop::eval_minus::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %59, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %152, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

156:                                              ; preds = %150
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

157:                                              ; preds = %61
  %158 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %159 = getelementptr inbounds nuw i8, ptr %1, i64 40
  %160 = load i8, ptr %159, align 8, !range !1746, !noundef !1708
  %161 = trunc nuw i8 %160 to i1
; call purrdf_sparql_eval::remote::eval_service::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::remote::eval_service::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(address) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) %158, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %63, i1 noundef zeroext %161, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

162:                                              ; preds = %61
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

163:                                              ; preds = %65
  %164 = getelementptr inbounds nuw i8, ptr %4, i64 40
  %165 = getelementptr inbounds nuw i8, ptr %0, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %165, ptr noundef nonnull align 8 dereferenceable(56) %164, i64 56, i1 false)
  %166 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %166, ptr noundef nonnull align 8 dereferenceable(32) %5, i64 32, i1 false)
  %167 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %74, ptr %167, align 16
  store i64 1, ptr %0, align 16
  br label %106

168:                                              ; preds = %65
  %169 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(32) %169, ptr noundef nonnull align 8 dereferenceable(32) %5, i64 32, i1 false)
  %170 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store i64 -1, ptr %170, align 8
  store i64 0, ptr %0, align 16
  br label %106

171:                                              ; preds = %76
  %172 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %173 = load ptr, ptr %172, align 8, !nonnull !1708, !noundef !1708
  %174 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %175 = load i64, ptr %174, align 8, !noundef !1708
; call purrdf_sparql_eval::modifier::eval_order_by::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::modifier::eval_order_by::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %78, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %173, i64 noundef %175, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

176:                                              ; preds = %76
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

177:                                              ; preds = %80
  %178 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %179 = load ptr, ptr %178, align 8, !nonnull !1708, !noundef !1708
  %180 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %181 = load i64, ptr %180, align 8, !noundef !1708
; call purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %82, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %179, i64 noundef %181, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

182:                                              ; preds = %80
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

183:                                              ; preds = %3, %3
  %184 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %185 = load ptr, ptr %184, align 8, !align !1818, !noundef !1708
  %186 = icmp eq ptr %185, null
  br i1 %186, label %188, label %187, !prof !1803

187:                                              ; preds = %183
; call purrdf_sparql_eval::modifier::eval_dedup::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::modifier::eval_dedup::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %185, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

188:                                              ; preds = %183
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

189:                                              ; preds = %84
  %190 = getelementptr inbounds nuw i8, ptr %1, i64 32
  %191 = load i64, ptr %190, align 8, !noundef !1708
  %192 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %193 = load i64, ptr %192, align 8, !range !1855, !noundef !1708
  %194 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %195 = load i64, ptr %194, align 8
; call purrdf_sparql_eval::modifier::eval_slice::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::modifier::eval_slice::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %86, i64 noundef %191, i64 noundef %193, i64 %195, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

196:                                              ; preds = %84
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

197:                                              ; preds = %88
  %198 = getelementptr inbounds nuw i8, ptr %1, i64 16
  %199 = load ptr, ptr %198, align 8, !nonnull !1708, !noundef !1708
  %200 = getelementptr inbounds nuw i8, ptr %1, i64 24
  %201 = load i64, ptr %200, align 8, !noundef !1708
  %202 = getelementptr inbounds nuw i8, ptr %1, i64 40
  %203 = load ptr, ptr %202, align 8, !nonnull !1708, !noundef !1708
  %204 = getelementptr inbounds nuw i8, ptr %1, i64 48
  %205 = load i64, ptr %204, align 8, !noundef !1708
; call purrdf_sparql_eval::modifier::eval_group::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::modifier::eval_group::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %90, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %199, i64 noundef %201, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %203, i64 noundef %205, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

206:                                              ; preds = %88
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable

207:                                              ; preds = %94
  %208 = getelementptr inbounds nuw i8, ptr %1, i64 72
  %209 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %210 = getelementptr inbounds nuw i8, ptr %1, i64 96
  %211 = load ptr, ptr %210, align 8, !noundef !1708
  %212 = icmp eq ptr %211, null
  %213 = select i1 %212, ptr null, ptr %210
; call purrdf_sparql_eval::cdt_unfold::eval_unfold::<purrdf_core::ir::dataset::RdfDataset>
  tail call void @purrdf_sparql_eval::cdt_unfold::eval_unfold::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %96, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %209, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %208, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable_or_null(16) %213, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %2) #87
  br label %106

214:                                              ; preds = %94
; call core::option::expect_failed
  tail call void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.2.llvm.13412714042204560522, i64 noundef 48, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.4.llvm.13412714042204560522) #92
  unreachable
}

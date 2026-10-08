define void @purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef align 16 dereferenceable(1248) %4) unnamed_addr #8 personality ptr @rust_eh_personality !guid !30872 {
  %6 = alloca [8 x i8], align 8
  %7 = alloca [1 x i8], align 1
  %8 = alloca [40 x i8], align 8
  %9 = alloca [16 x i8], align 8
  %10 = alloca [40 x i8], align 8
  %11 = alloca [32 x i8], align 8
  %12 = alloca [32 x i8], align 8
  %13 = alloca [56 x i8], align 8
  %14 = alloca [96 x i8], align 16
  %15 = alloca [16 x i8], align 8
  %16 = alloca [24 x i8], align 8
  %17 = alloca [8 x i8], align 8
  %18 = alloca [8 x i8], align 8
  %19 = alloca [16 x i8], align 8
  %20 = alloca [8 x i8], align 8
  %21 = alloca [112 x i8], align 16
  %22 = alloca [48 x i8], align 8
  %23 = alloca [48 x i8], align 8
  %24 = alloca [48 x i8], align 8
  %25 = alloca [24 x i8], align 8
  %26 = alloca [96 x i8], align 16
  %27 = alloca [48 x i8], align 8
  %28 = alloca [48 x i8], align 8
  %29 = alloca [64 x i8], align 8
  %30 = alloca [23 x i8], align 1
  %31 = alloca [32 x i8], align 8
  %32 = alloca [64 x i8], align 8
  %33 = alloca [96 x i8], align 8
  %34 = alloca [96 x i8], align 16
  %35 = alloca [40 x i8], align 8
  %36 = alloca [72 x i8], align 8
  %37 = alloca [112 x i8], align 16
  %38 = alloca [32 x i8], align 8
  %39 = alloca [104 x i8], align 8
  %40 = alloca [96 x i8], align 8
  %41 = alloca [24 x i8], align 8
  %42 = alloca [24 x i8], align 8
  %43 = alloca [24 x i8], align 8
  %44 = alloca [24 x i8], align 8
  %45 = alloca [56 x i8], align 8
  %46 = alloca [8 x i8], align 8
  %47 = alloca [32 x i8], align 8
  %48 = alloca [72 x i8], align 8
  %49 = alloca [32 x i8], align 8
  %50 = alloca [96 x i8], align 8
  %51 = alloca [112 x i8], align 16
  %52 = alloca [96 x i8], align 16
  %53 = alloca [24 x i8], align 8
  %54 = alloca [112 x i8], align 16
  %55 = alloca [96 x i8], align 16
  %56 = alloca [32 x i8], align 8
  %57 = alloca [56 x i8], align 8
  %58 = alloca [24 x i8], align 8
  %59 = alloca [56 x i8], align 8
  %60 = alloca [8 x i8], align 8
  %61 = alloca [96 x i8], align 16
  %62 = alloca [24 x i8], align 8
  %63 = alloca [96 x i8], align 16
  %64 = alloca [104 x i8], align 8
  %65 = alloca [32 x i8], align 8
  %66 = alloca [96 x i8], align 8
  %67 = alloca [96 x i8], align 8
  %.sroa.0 = alloca [24 x i8], align 8
  %68 = alloca [23 x i8], align 1
  %69 = alloca [24 x i8], align 16
  %70 = alloca [32 x i8], align 8
  %71 = alloca [96 x i8], align 8
  %72 = alloca [24 x i8], align 8
  %73 = alloca [32 x i8], align 8
  %74 = alloca [32 x i8], align 8
  %75 = alloca [104 x i8], align 8
  %76 = alloca [96 x i8], align 8
  %77 = alloca [104 x i8], align 8
  %78 = alloca [32 x i8], align 8
  %79 = alloca [32 x i8], align 8
  %80 = alloca [96 x i8], align 8
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %39, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %78)
  call void @llvm.lifetime.start.p0(ptr nonnull %77)
  %81 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %82 = getelementptr inbounds nuw i8, ptr %8, i64 8
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %37, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %90 unwind label %87, !inline_history !13429

83:                                               ; preds = %1390, %1244, %87
  %84 = phi i8 [ %88, %87 ], [ %1392, %1390 ], [ %268, %1244 ]
  %85 = phi { ptr, i32 } [ %89, %87 ], [ %1391, %1390 ], [ %1245, %1244 ]
  %86 = trunc nuw i8 %84 to i1
  br i1 %86, label %1404, label %1403

87:                                               ; preds = %1231, %101, %96, %5
  %88 = phi i8 [ 1, %5 ], [ 0, %1231 ], [ 0, %101 ], [ 1, %96 ]
  %89 = landingpad { ptr, i32 }
          cleanup
  br label %83

90:                                               ; preds = %5
  %91 = load i64, ptr %37, align 16, !range !1739, !noundef !1740
  %92 = trunc nuw i64 %91 to i1
  br i1 %92, label %93, label %96

93:                                               ; preds = %90
  %94 = getelementptr inbounds nuw i8, ptr %37, i64 16
  %95 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %95, ptr noundef nonnull align 16 dereferenceable(96) %94, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %77)
  call void @llvm.lifetime.end.p0(ptr nonnull %78)
  br label %1309

96:                                               ; preds = %90
  %97 = getelementptr inbounds nuw i8, ptr %37, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %77, ptr noundef nonnull align 8 dereferenceable(96) %97, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %78, ptr noalias nofree noundef align 8 dereferenceable(104) %39, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %77)
          to label %98 unwind label %87

98:                                               ; preds = %96
  %99 = load i64, ptr %78, align 8, !range !2059, !noundef !1740
  %100 = icmp eq i64 %99, -1
  br i1 %100, label %101, label %105

101:                                              ; preds = %98
  call void @llvm.lifetime.end.p0(ptr nonnull %77)
  call void @llvm.lifetime.end.p0(ptr nonnull %78)
  call void @llvm.lifetime.start.p0(ptr nonnull %80)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %80, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %39)
          to label %1401 unwind label %87

102:                                              ; preds = %1230, %841, %156, %154, %150, %147, %146
  %103 = phi i8 [ 1, %841 ], [ 0, %1230 ], [ 1, %154 ], [ 1, %156 ], [ 1, %150 ], [ 1, %147 ], [ 1, %146 ]
  %104 = landingpad { ptr, i32 }
          cleanup
  br label %1393

105:                                              ; preds = %98
  call void @llvm.lifetime.start.p0(ptr nonnull %79)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %79, ptr noundef nonnull align 8 dereferenceable(32) %78, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %77)
  call void @llvm.lifetime.end.p0(ptr nonnull %78)
  %106 = load i64, ptr %39, align 8, !range !2059, !noundef !1740
  %107 = icmp eq i64 %106, -1
  br i1 %107, label %108, label %1248

108:                                              ; preds = %105
  %109 = load i32, ptr %3, align 8, !range !2158, !noundef !1740
  %110 = icmp ne i32 %109, 11
  tail call void @llvm.assume(i1 %110)
  %111 = icmp eq i32 %109, 28
  br i1 %111, label %112, label %146

112:                                              ; preds = %108
  %113 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %114 = getelementptr inbounds nuw i8, ptr %4, i64 624
  %115 = load ptr, ptr %114, align 16, !noundef !1740
  %116 = icmp ne ptr %115, null
  %117 = getelementptr inbounds nuw i8, ptr %4, i64 1200
  %118 = load i64, ptr %117, align 16
  %119 = trunc nuw i64 %118 to i1
  %120 = select i1 %116, i1 %119, i1 false
  br i1 %120, label %121, label %147

121:                                              ; preds = %112
  %122 = getelementptr inbounds nuw i8, ptr %4, i64 1216
  %123 = load i32, ptr %122, align 16
  %124 = getelementptr inbounds nuw i8, ptr %4, i64 1208
  %125 = load i64, ptr %124, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30873)
  %126 = getelementptr inbounds nuw i8, ptr %115, i64 40
  %127 = load i64, ptr %126, align 8, !alias.scope !30873, !noundef !1740
  %128 = icmp eq i64 %125, %127
  br i1 %128, label %129, label %147

129:                                              ; preds = %121
  %130 = getelementptr inbounds nuw i8, ptr %115, i64 80
  %131 = load i32, ptr %130, align 8, !alias.scope !30873, !noundef !1740
  %132 = icmp ult i32 %123, %131
  br i1 %132, label %147, label %133

133:                                              ; preds = %129
  %134 = sub nuw i32 %123, %131
  %135 = zext i32 %134 to i64
  %136 = getelementptr inbounds nuw i8, ptr %115, i64 32
  %137 = load i64, ptr %136, align 8, !alias.scope !30873, !noundef !1740
  %138 = icmp ugt i64 %137, %135
  br i1 %138, label %139, label %147

139:                                              ; preds = %133
  %140 = getelementptr inbounds nuw i8, ptr %115, i64 24
  %141 = load ptr, ptr %140, align 8, !alias.scope !30873, !nonnull !1740, !noundef !1740
  %142 = getelementptr inbounds nuw [16 x i8], ptr %141, i64 %135
  %143 = load i64, ptr %142, align 8, !range !1739, !noalias !30873, !noundef !1740
  %144 = getelementptr inbounds nuw i8, ptr %142, i64 8
  %145 = load i64, ptr %144, align 8, !noalias !30873
  br label %147

146:                                              ; preds = %108
  call void @llvm.lifetime.start.p0(ptr nonnull %63)
; invoke purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::service_endpoints::admit_lateral_endpoints::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %63, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) %79, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noundef nonnull align 16 %4)
          to label %274 unwind label %102

147:                                              ; preds = %139, %133, %129, %121, %112
  %148 = phi i64 [ undef, %121 ], [ undef, %112 ], [ undef, %133 ], [ %145, %139 ], [ undef, %129 ]
  %149 = phi i64 [ 0, %121 ], [ 0, %112 ], [ 0, %133 ], [ %143, %139 ], [ 0, %129 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %73)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::enter_node
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::enter_node(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %73, ptr noalias nofree noundef align 16 dereferenceable(1248) %4, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3)
          to label %150 unwind label %102

150:                                              ; preds = %147
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %72, ptr noundef nonnull align 16 %4, i8 noundef 0)
          to label %151 unwind label %102

151:                                              ; preds = %150
  %152 = load i8, ptr %72, align 8, !range !1743, !noundef !1740
  %153 = icmp eq i8 %152, -1
  br i1 %153, label %156, label %154

154:                                              ; preds = %151
  call void @llvm.lifetime.start.p0(ptr nonnull %71)
  call void @llvm.lifetime.start.p0(ptr nonnull %70)
; invoke purrdf_sparql_eval::eval::syntactic_schema
  %155 = invoke noundef nonnull ptr @purrdf_sparql_eval::eval::syntactic_schema(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3)
          to label %.thread unwind label %102

156:                                              ; preds = %151
  call void @llvm.lifetime.start.p0(ptr nonnull %30)
  call void @llvm.lifetime.start.p0(ptr nonnull %31)
  call void @llvm.lifetime.start.p0(ptr nonnull %33)
  %157 = getelementptr inbounds nuw i8, ptr %33, i64 8
  %158 = getelementptr inbounds nuw i8, ptr %33, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !30876
; invoke purrdf_sparql_eval::property_fn_eval::eval_call_over::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::property_fn_eval::eval_call_over::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %34, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(72) %113, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(32) %79, i64 noundef range(i64 0, 2) %149, i64 %148, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %159 unwind label %102

159:                                              ; preds = %156
  %160 = load i64, ptr %34, align 16, !range !2527, !noalias !30876, !noundef !1740
  %.not = icmp eq i64 %160, -1
  %161 = getelementptr inbounds nuw i8, ptr %34, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %31, ptr noundef nonnull align 8 dereferenceable(32) %161, i64 32, i1 false), !noalias !30876
  %162 = getelementptr inbounds nuw i8, ptr %34, i64 40
  %163 = load i8, ptr %162, align 8, !noalias !30876
  %164 = getelementptr inbounds nuw i8, ptr %34, i64 41
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %30, ptr noundef nonnull align 1 dereferenceable(23) %164, i64 23, i1 false), !noalias !30876
  br i1 %.not, label %165, label %200

165:                                              ; preds = %159
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !30876
  %166 = icmp eq i8 %163, -1
  br i1 %166, label %174, label %167

167:                                              ; preds = %165
  %168 = getelementptr inbounds nuw i8, ptr %32, i64 25
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !30876
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %168, ptr noundef nonnull align 1 dereferenceable(23) %30, i64 23, i1 false), !noalias !30876
  store i64 0, ptr %32, align 8, !noalias !30876
  %169 = getelementptr inbounds nuw i8, ptr %32, i64 8
  store ptr inttoptr (i64 1 to ptr), ptr %169, align 8, !noalias !30876
  %170 = getelementptr inbounds nuw i8, ptr %32, i64 16
  store i64 0, ptr %170, align 8, !noalias !30876
  %171 = getelementptr inbounds nuw i8, ptr %32, i64 24
  store i8 %163, ptr %171, align 8, !noalias !30876
  %172 = getelementptr inbounds nuw i8, ptr %32, i64 48
  store ptr null, ptr %172, align 8, !noalias !30876
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef align 8 captures(none) dereferenceable(96) %33, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %31, ptr noalias nofree noundef align 8 captures(address) dereferenceable(64) %32), !noalias !30882
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !30876
  %173 = load i64, ptr %33, align 8, !noalias !30876
  %.pre = load i64, ptr %158, align 8, !noalias !30883
  %.phi.trans.insert = getelementptr inbounds nuw i8, ptr %33, i64 48
  %.pre439 = load i8, ptr %.phi.trans.insert, align 8, !noalias !30883
  br label %192

174:                                              ; preds = %165
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %157, ptr noundef nonnull align 8 dereferenceable(32) %31, i64 32, i1 false), !noalias !30876
  br label %192

.thread:                                          ; preds = %154
  %175 = getelementptr inbounds nuw i8, ptr %70, i64 24
  store ptr %155, ptr %175, align 8, !alias.scope !30884
  store i64 0, ptr %70, align 8, !alias.scope !30884
  %176 = getelementptr inbounds nuw i8, ptr %70, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %176, align 8, !alias.scope !30884
  %177 = getelementptr inbounds nuw i8, ptr %70, i64 16
  store i64 0, ptr %177, align 8, !alias.scope !30884
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !30887
  store i64 0, ptr %29, align 8, !noalias !30887
  %178 = getelementptr inbounds nuw i8, ptr %29, i64 8
  store ptr inttoptr (i64 1 to ptr), ptr %178, align 8, !noalias !30887
  %179 = getelementptr inbounds nuw i8, ptr %29, i64 16
  store i64 0, ptr %179, align 8, !noalias !30887
  %180 = getelementptr inbounds nuw i8, ptr %29, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %180, ptr noundef nonnull align 8 dereferenceable(24) %72, i64 24, i1 false)
  %181 = getelementptr inbounds nuw i8, ptr %29, i64 48
  store ptr null, ptr %181, align 8, !noalias !30887
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %71, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %70, ptr noalias nofree noundef align 8 captures(address) dereferenceable(64) %29), !noalias !30892
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !30887
  call void @llvm.lifetime.end.p0(ptr nonnull %70)
  %182 = load i64, ptr %71, align 8
  %183 = getelementptr inbounds nuw i8, ptr %71, i64 8
  %184 = load i64, ptr %183, align 8
  %185 = getelementptr inbounds nuw i8, ptr %71, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %.sroa.0, ptr noundef nonnull align 8 dereferenceable(24) %185, i64 24, i1 false)
  %.sroa.5.0..sroa_idx83 = getelementptr inbounds nuw i8, ptr %71, i64 40
  %.sroa.5.0.copyload84 = load i64, ptr %.sroa.5.0..sroa_idx83, align 8
  %186 = getelementptr inbounds nuw i8, ptr %71, i64 48
  %187 = load i8, ptr %186, align 8
  %188 = getelementptr inbounds nuw i8, ptr %71, i64 49
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %68, ptr noundef nonnull align 1 dereferenceable(23) %188, i64 23, i1 false)
  %189 = getelementptr inbounds nuw i8, ptr %71, i64 72
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(24) %69, ptr noundef nonnull align 8 dereferenceable(24) %189, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %71)
  br label %216

190:                                              ; preds = %216
  %191 = landingpad { ptr, i32 }
          cleanup
  br label %1393

192:                                              ; preds = %167, %174
  %193 = phi i8 [ %.pre439, %167 ], [ undef, %174 ]
  %194 = phi i64 [ %.pre, %167 ], [ undef, %174 ]
  %195 = phi i64 [ %173, %167 ], [ -1, %174 ]
  %196 = load i64, ptr %157, align 8, !noalias !30883
  %197 = getelementptr inbounds nuw i8, ptr %33, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %.sroa.0, ptr noundef nonnull align 8 dereferenceable(24) %197, i64 24, i1 false)
  %198 = getelementptr inbounds nuw i8, ptr %33, i64 49
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %68, ptr noundef nonnull align 1 dereferenceable(23) %198, i64 23, i1 false)
  %199 = getelementptr inbounds nuw i8, ptr %33, i64 72
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(24) %69, ptr noundef nonnull align 8 dereferenceable(24) %199, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  call void @llvm.lifetime.end.p0(ptr nonnull %33)
  br label %216

200:                                              ; preds = %159
  %201 = getelementptr inbounds nuw i8, ptr %34, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(24) %69, ptr noundef nonnull align 16 dereferenceable(24) %201, i64 24, i1 false)
  %202 = getelementptr inbounds nuw i8, ptr %34, i64 88
  %203 = load i64, ptr %202, align 8, !noalias !30883
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !30876
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %.sroa.0, ptr noundef nonnull align 8 dereferenceable(24) %31, i64 24, i1 false)
  %.sroa.5.0..sroa_idx = getelementptr inbounds nuw i8, ptr %31, i64 24
  %.sroa.5.0.copyload = load i64, ptr %.sroa.5.0..sroa_idx, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %68, ptr noundef nonnull align 1 dereferenceable(23) %30, i64 23, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  call void @llvm.lifetime.end.p0(ptr nonnull %33)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30893)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30896)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(24) %117, ptr noundef nonnull readonly align 8 dereferenceable(32) %73, i64 24, i1 false), !alias.scope !30898
  %204 = getelementptr inbounds nuw i8, ptr %73, i64 24
  %205 = load i32, ptr %204, align 8, !alias.scope !30896, !noalias !30893, !noundef !1740
  %206 = getelementptr inbounds nuw i8, ptr %4, i64 1228
  store i32 %205, ptr %206, align 4, !alias.scope !30893, !noalias !30896
  %207 = getelementptr inbounds nuw i8, ptr %73, i64 28
  %208 = load i8, ptr %207, align 4, !range !1747, !alias.scope !30896, !noalias !30893, !noundef !1740
  %209 = getelementptr inbounds nuw i8, ptr %4, i64 1238
  store i8 %208, ptr %209, align 2, !alias.scope !30893, !noalias !30896
  %210 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %210, ptr noundef nonnull align 8 dereferenceable(24) %.sroa.0, i64 24, i1 false)
  %.sroa.5.0..sroa_idx85 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %.sroa.5.0.copyload, ptr %.sroa.5.0..sroa_idx85, align 16
  %211 = getelementptr inbounds nuw i8, ptr %0, i64 57
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %211, ptr noundef nonnull align 1 dereferenceable(23) %68, i64 23, i1 false)
  %212 = getelementptr inbounds nuw i8, ptr %0, i64 80
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(24) %212, ptr noundef nonnull align 16 dereferenceable(24) %69, i64 24, i1 false)
  %213 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %160, ptr %213, align 16
  %214 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i8 %163, ptr %214, align 8
  %215 = getelementptr inbounds nuw i8, ptr %0, i64 104
  store i64 %203, ptr %215, align 8
  store i64 1, ptr %0, align 16
  br label %263

216:                                              ; preds = %192, %.thread
  %217 = phi i64 [ %182, %.thread ], [ %195, %192 ]
  %218 = phi i64 [ %184, %.thread ], [ %196, %192 ]
  %219 = phi i8 [ %187, %.thread ], [ %193, %192 ]
  %.sroa.5.193 = phi i64 [ %.sroa.5.0.copyload84, %.thread ], [ %194, %192 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(24) %117, ptr noundef nonnull readonly align 8 dereferenceable(32) %73, i64 24, i1 false), !alias.scope !30899
  %220 = getelementptr inbounds nuw i8, ptr %73, i64 24
  %221 = load i32, ptr %220, align 8, !alias.scope !30904, !noalias !1740, !noundef !1740
  %222 = getelementptr inbounds nuw i8, ptr %4, i64 1228
  store i32 %221, ptr %222, align 4, !alias.scope !30905, !noalias !1740
  %223 = getelementptr inbounds nuw i8, ptr %73, i64 28
  %224 = load i8, ptr %223, align 4, !range !1747, !alias.scope !30904, !noalias !1740, !noundef !1740
  %225 = getelementptr inbounds nuw i8, ptr %4, i64 1238
  store i8 %224, ptr %225, align 2, !alias.scope !30905, !noalias !1740
  store i64 %217, ptr %67, align 8
  %226 = getelementptr inbounds nuw i8, ptr %67, i64 8
  store i64 %218, ptr %226, align 8
  %227 = getelementptr inbounds nuw i8, ptr %67, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %227, ptr noundef nonnull align 8 dereferenceable(24) %.sroa.0, i64 24, i1 false)
  %.sroa.5.0..sroa_idx87 = getelementptr inbounds nuw i8, ptr %67, i64 40
  store i64 %.sroa.5.193, ptr %.sroa.5.0..sroa_idx87, align 8
  %228 = getelementptr inbounds nuw i8, ptr %67, i64 48
  store i8 %219, ptr %228, align 8
  %229 = getelementptr inbounds nuw i8, ptr %67, i64 49
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %229, ptr noundef nonnull align 1 dereferenceable(23) %68, i64 23, i1 false)
  %230 = getelementptr inbounds nuw i8, ptr %67, i64 72
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %230, ptr noundef nonnull align 16 dereferenceable(24) %69, i64 24, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %65, ptr noalias nofree noundef align 8 dereferenceable(104) %39, i64 noundef 1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %67)
          to label %231 unwind label %190

231:                                              ; preds = %216
  call void @llvm.lifetime.start.p0(ptr nonnull %66)
  %232 = load i64, ptr %65, align 8, !range !2059, !noundef !1740
  %233 = icmp eq i64 %232, -1
  br i1 %233, label %257, label %234

234:                                              ; preds = %231
  call void @llvm.lifetime.start.p0(ptr nonnull %64)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %64, ptr noundef nonnull align 8 dereferenceable(104) %39, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30906)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30909)
  %235 = load i64, ptr %64, align 8, !range !2059, !alias.scope !30909, !noalias !30911, !noundef !1740
  %236 = icmp eq i64 %235, -1
  br i1 %236, label %238, label %237

237:                                              ; preds = %234
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %66, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %65, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %39)
  br label %240

238:                                              ; preds = %234
  %239 = getelementptr inbounds nuw i8, ptr %66, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %239, ptr noundef nonnull align 8 dereferenceable(32) %65, i64 32, i1 false)
  store i64 -1, ptr %66, align 8, !alias.scope !30906, !noalias !30913
  br label %240

240:                                              ; preds = %238, %237
  %241 = getelementptr inbounds nuw i8, ptr %64, i64 72
  %242 = load i64, ptr %241, align 8, !range !1778, !alias.scope !30914, !noalias !30911, !noundef !1740
  %243 = icmp ugt i64 %242, 5
  br i1 %243, label %244, label %249

244:                                              ; preds = %240
  %245 = getelementptr inbounds nuw i8, ptr %64, i64 80
  %246 = load ptr, ptr %245, align 8, !alias.scope !30909, !noalias !30911, !nonnull !1740, !noundef !1740
  %247 = mul i64 %242, 3
  %248 = add i64 %247, -3
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %246, i64 noundef %248, i64 noundef range(i64 1, -9223372036854775807) 1) #92, !noalias !30917
  br label %249

249:                                              ; preds = %244, %240
  %250 = getelementptr inbounds nuw i8, ptr %64, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30920)
  %251 = load ptr, ptr %250, align 8, !alias.scope !30923, !noalias !30911, !noundef !1740
  %252 = icmp eq ptr %251, null
  br i1 %252, label %262, label %253

253:                                              ; preds = %249
  %254 = atomicrmw sub ptr %251, i64 1 release, align 8, !noalias !30924
  %255 = icmp eq i64 %254, 1
  br i1 %255, label %256, label %262

256:                                              ; preds = %253
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %250) #91
          to label %262 unwind label %258

257:                                              ; preds = %231
; invoke <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %66, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %39)
          to label %260 unwind label %258

258:                                              ; preds = %257, %256
  %259 = landingpad { ptr, i32 }
          cleanup
  br label %1393

260:                                              ; preds = %262, %257
  %261 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %261, ptr noundef nonnull align 8 dereferenceable(96) %66, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %66)
  br label %263

262:                                              ; preds = %256, %253, %249
  call void @llvm.lifetime.end.p0(ptr nonnull %64)
  br label %260

263:                                              ; preds = %260, %200
  %264 = phi i8 [ 1, %200 ], [ 0, %260 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %73)
  br label %267

265:                                              ; preds = %1404, %1400, %1241, %821, %689, %339, %318, %306
  %266 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

267:                                              ; preds = %1242, %842, %277, %263
  %268 = phi i8 [ %264, %263 ], [ 1, %277 ], [ 1, %1242 ], [ 1, %842 ]
  %269 = getelementptr inbounds nuw i8, ptr %79, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !30929)
  call void @llvm.experimental.noalias.scope.decl(metadata !30932)
  %270 = load ptr, ptr %269, align 8, !alias.scope !30935, !nonnull !1740, !noundef !1740
  %271 = atomicrmw sub ptr %270, i64 1 release, align 8, !noalias !30935
  %272 = icmp eq i64 %271, 1
  br i1 %272, label %273, label %843

273:                                              ; preds = %267
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %269) #91
          to label %843 unwind label %1244

274:                                              ; preds = %146
  %275 = load i64, ptr %63, align 16, !range !2527, !noundef !1740
  %276 = icmp eq i64 %275, -1
  br i1 %276, label %279, label %277

277:                                              ; preds = %274
  %278 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %278, ptr noundef nonnull align 16 dereferenceable(96) %63, i64 96, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %63)
  store i64 1, ptr %0, align 16
  br label %267

279:                                              ; preds = %274
  call void @llvm.lifetime.end.p0(ptr nonnull %63)
  call void @llvm.lifetime.start.p0(ptr nonnull %62)
  %280 = getelementptr inbounds nuw i8, ptr %4, i64 584
  %281 = load ptr, ptr %280, align 8, !noundef !1740
  %282 = icmp eq ptr %281, null
  br i1 %282, label %286, label %283

283:                                              ; preds = %279
; call purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#0}
  %284 = tail call fastcc noundef align 8 ptr @purrdf_sparql_eval::binop::eval_application::<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#0}(ptr noundef nonnull align 8 %3, ptr nonnull %281)
  %285 = icmp eq ptr %284, null
  br i1 %285, label %286, label %287

286:                                              ; preds = %287, %283, %279
  store ptr null, ptr %62, align 8
  br label %299

287:                                              ; preds = %283
  %288 = load i64, ptr %284, align 8, !range !1739, !alias.scope !30936, !noundef !1740
  %289 = trunc nuw i64 %288 to i1
  br i1 %289, label %290, label %286

290:                                              ; preds = %287
  %291 = getelementptr inbounds nuw i8, ptr %284, i64 8
; call <purrdf_sparql_eval::deferred_exists::DeferredLateral as core::clone::Clone>::clone
  call fastcc void @<purrdf_sparql_eval::deferred_exists::DeferredLateral as core::clone::Clone>::clone(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %62, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) %291)
  %292 = load ptr, ptr %62, align 8
  %293 = icmp eq ptr %292, null
  br i1 %293, label %299, label %294

294:                                              ; preds = %310, %290
  call void @llvm.lifetime.start.p0(ptr nonnull %60)
  %295 = getelementptr inbounds nuw i8, ptr %79, i64 24
  %296 = load ptr, ptr %295, align 8, !nonnull !1740, !noundef !1740
  %297 = atomicrmw add ptr %296, i64 1 monotonic, align 8
  %298 = icmp slt i64 %297, 0
  br i1 %298, label %312, label %322

299:                                              ; preds = %290, %286
; invoke purrdf_sparql_eval::deferred_exists::is_lateral_placeholder
  %300 = invoke noundef zeroext i1 @purrdf_sparql_eval::deferred_exists::is_lateral_placeholder(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3)
          to label %310 unwind label %307

301:                                              ; preds = %318, %313, %307
  %302 = phi i8 [ %308, %307 ], [ %314, %318 ], [ %314, %313 ]
  %303 = phi { ptr, i32 } [ %309, %307 ], [ %315, %318 ], [ %315, %313 ]
  %304 = load ptr, ptr %62, align 8, !alias.scope !30939, !noundef !1740
  %305 = icmp eq ptr %304, null
  br i1 %305, label %1393, label %306

306:                                              ; preds = %301
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %62)
          to label %1393 unwind label %265

307:                                              ; preds = %1226, %837, %311, %299
  %308 = phi i8 [ 1, %311 ], [ 0, %1226 ], [ 1, %837 ], [ 1, %299 ]
  %309 = landingpad { ptr, i32 }
          cleanup
  br label %301

310:                                              ; preds = %299
  br i1 %300, label %311, label %294

311:                                              ; preds = %310
  call void @llvm.lifetime.start.p0(ptr nonnull %61)
; invoke <purrdf_sparql_eval::error::EvalError>::internal::<&str>
  invoke fastcc void @<purrdf_sparql_eval::error::EvalError>::internal::<&str>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %61, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.e5162873a9a3251d11c4df37a70e4654.443, i64 noundef 84)
          to label %1242 unwind label %307

312:                                              ; preds = %294
  tail call void @llvm.trap()
  unreachable

313:                                              ; preds = %339, %319
  %314 = phi i8 [ %340, %339 ], [ %320, %319 ]
  %315 = phi { ptr, i32 } [ %341, %339 ], [ %321, %319 ]
  %316 = atomicrmw sub ptr %296, i64 1 release, align 8, !noalias !30942
  %317 = icmp eq i64 %316, 1
  br i1 %317, label %318, label %301

318:                                              ; preds = %313
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %60) #91
          to label %301 unwind label %265

319:                                              ; preds = %1222, %833
  %320 = phi i8 [ 0, %1222 ], [ 1, %833 ]
  %321 = landingpad { ptr, i32 }
          cleanup
  br label %313

322:                                              ; preds = %294
  store ptr %296, ptr %60, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %59)
  store i64 0, ptr %59, align 8, !alias.scope !30947
  %323 = getelementptr inbounds nuw i8, ptr %59, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %323, align 8, !alias.scope !30947
  %324 = getelementptr inbounds nuw i8, ptr %59, i64 16
  store i64 0, ptr %324, align 8, !alias.scope !30947
  %325 = getelementptr inbounds nuw i8, ptr %59, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %325, ptr noundef nonnull align 8 dereferenceable(32) @anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932, i64 32, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %58)
  %326 = getelementptr inbounds nuw i8, ptr %79, i64 16
  %327 = load i64, ptr %326, align 8, !noundef !1740
  %328 = icmp ult i64 %327, 230584300921369396
  tail call void @llvm.assume(i1 %328)
  %329 = mul nuw i64 %327, 72
  %330 = icmp samesign ugt i64 %327, 128102389400760775
  br i1 %330, label %345, label %331, !prof !6193

331:                                              ; preds = %322
  %332 = icmp eq i64 %327, 0
  br i1 %332, label %333, label %336

333:                                              ; preds = %331
  store i64 0, ptr %58, align 8
  %334 = getelementptr inbounds nuw i8, ptr %58, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %334, align 8
  %335 = getelementptr inbounds nuw i8, ptr %58, i64 16
  store i64 0, ptr %335, align 8
  br label %.loopexit108

336:                                              ; preds = %331
; call __rustc::__rust_alloc
  %337 = tail call noundef align 8 ptr @__rustc::__rust_alloc(i64 noundef %329, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !30950
  %338 = icmp eq ptr %337, null
  br i1 %338, label %345, label %348

339:                                              ; preds = %689, %342
  %340 = phi i8 [ %343, %342 ], [ %690, %689 ]
  %341 = phi { ptr, i32 } [ %344, %342 ], [ %691, %689 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 dereferenceable(56) %59) #89
          to label %313 unwind label %265

342:                                              ; preds = %1220, %706, %345
  %343 = phi i8 [ 1, %345 ], [ 0, %1220 ], [ 1, %706 ]
  %344 = landingpad { ptr, i32 }
          cleanup
  br label %339

345:                                              ; preds = %336, %322
  %346 = phi i64 [ %329, %336 ], [ undef, %322 ]
  %347 = phi i64 [ 8, %336 ], [ 0, %322 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %347, i64 %346) #93
          to label %1236 unwind label %342

348:                                              ; preds = %336
  store i64 %327, ptr %58, align 8
  %349 = getelementptr inbounds nuw i8, ptr %58, i64 8
  store ptr %337, ptr %349, align 8
  %350 = getelementptr inbounds nuw i8, ptr %58, i64 16
  store i64 0, ptr %350, align 8
  %351 = getelementptr inbounds nuw i8, ptr %79, i64 8
  %352 = load ptr, ptr %351, align 8, !nonnull !1740, !noundef !1740
  %353 = mul nuw nsw i64 %327, 40
  %354 = getelementptr inbounds nuw i8, ptr %352, i64 %353
  %355 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_stack::FLOOR::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %356 = getelementptr i8, ptr %296, i64 24
  %357 = getelementptr i8, ptr %296, i64 32
  %358 = getelementptr inbounds nuw i8, ptr %26, i64 8
  %359 = getelementptr inbounds nuw i8, ptr %26, i64 56
  %360 = getelementptr inbounds nuw i8, ptr %54, i64 16
  %361 = getelementptr inbounds nuw i8, ptr %54, i64 24
  %362 = getelementptr inbounds nuw i8, ptr %54, i64 72
  %363 = getelementptr inbounds nuw i8, ptr %25, i64 8
  %364 = getelementptr inbounds nuw i8, ptr %25, i64 16
  %365 = getelementptr inbounds nuw i8, ptr %62, i64 8
  %366 = getelementptr inbounds nuw i8, ptr %62, i64 16
  %367 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %368 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %369 = getelementptr inbounds nuw i8, ptr %23, i64 32
  %370 = getelementptr inbounds nuw i8, ptr %23, i64 40
  %371 = getelementptr inbounds nuw i8, ptr %4, i64 632
  %372 = getelementptr inbounds nuw i8, ptr %4, i64 1096
  %373 = getelementptr inbounds nuw i8, ptr %4, i64 1112
  %374 = getelementptr inbounds nuw i8, ptr %4, i64 1104
  %375 = getelementptr inbounds nuw i8, ptr %19, i64 8
  %376 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %377 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %378 = getelementptr inbounds nuw i8, ptr %16, i64 16
  %379 = getelementptr inbounds nuw i8, ptr %13, i64 32
  %380 = getelementptr inbounds nuw i8, ptr %13, i64 40
  %381 = getelementptr inbounds nuw i8, ptr %13, i64 48
  %382 = getelementptr inbounds nuw i8, ptr %14, i64 8
  %383 = getelementptr inbounds nuw i8, ptr %14, i64 16
  %384 = getelementptr inbounds nuw i8, ptr %54, i64 32
  %385 = getelementptr inbounds nuw i8, ptr %12, i64 24
  %386 = getelementptr inbounds nuw i8, ptr %54, i64 8
  %387 = getelementptr inbounds nuw i8, ptr %4, i64 1237
  %388 = getelementptr inbounds nuw i8, ptr %53, i64 16
  %389 = getelementptr inbounds nuw i8, ptr %296, i64 16
  %390 = getelementptr inbounds nuw i8, ptr %51, i64 8
  %391 = getelementptr inbounds nuw i8, ptr %48, i64 8
  %392 = getelementptr inbounds nuw i8, ptr %48, i64 40
  %393 = getelementptr inbounds nuw i8, ptr %55, i64 32
  %394 = getelementptr inbounds nuw i8, ptr %51, i64 16
  %395 = getelementptr inbounds nuw i8, ptr %52, i64 32
  %396 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %397 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %.sroa.4.0..sroa_idx = getelementptr inbounds nuw i8, ptr %56, i64 24
  %.8..8..8..sroa_idx = getelementptr inbounds nuw i8, ptr %47, i64 8
  br label %398

398:                                              ; preds = %812, %348
  %399 = phi ptr [ %337, %348 ], [ %813, %812 ]
  %400 = phi i64 [ 0, %348 ], [ %815, %812 ]
  %401 = phi ptr [ %352, %348 ], [ %402, %812 ]
  %402 = getelementptr inbounds nuw i8, ptr %401, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %56)
  call void @llvm.lifetime.start.p0(ptr nonnull %57)
  %403 = load ptr, ptr %62, align 8, !noundef !1740
  %404 = icmp eq ptr %403, null
  br i1 %404, label %410, label %405

405:                                              ; preds = %398
  call void @llvm.lifetime.start.p0(ptr nonnull %55)
  call void @llvm.lifetime.start.p0(ptr nonnull %54)
  %406 = load i64, ptr %401, align 8, !range !1778, !noundef !1740
  %407 = add i64 %406, -1
  %408 = icmp ugt i64 %407, 4
  %409 = getelementptr inbounds nuw i8, ptr %401, i64 8
  br i1 %408, label %418, label %423

410:                                              ; preds = %398
  %411 = load i8, ptr %387, align 1, !range !1747, !noundef !1740
  %412 = trunc nuw i8 %411 to i1
  %413 = select i1 %412, i64 2, i64 1
  call void @llvm.lifetime.start.p0(ptr nonnull %53)
  store i64 %413, ptr %53, align 8
  store ptr null, ptr %388, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %52)
  call void @llvm.lifetime.start.p0(ptr nonnull %51)
  %414 = load i64, ptr %401, align 8, !range !1778, !noundef !1740
  %415 = add i64 %414, -1
  %416 = icmp ugt i64 %415, 4
  %417 = getelementptr inbounds nuw i8, ptr %401, i64 8
  br i1 %416, label %707, label %712

418:                                              ; preds = %405
  %419 = load ptr, ptr %409, align 8, !nonnull !1740, !noundef !1740
  %420 = getelementptr inbounds nuw i8, ptr %401, i64 16
  %421 = load i64, ptr %420, align 8, !noundef !1740
  %422 = add i64 %421, -1
  br label %423

423:                                              ; preds = %418, %405
  %424 = phi i64 [ %422, %418 ], [ %407, %405 ]
  %425 = phi ptr [ %419, %418 ], [ %409, %405 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !30953)
  call void @llvm.experimental.noalias.scope.decl(metadata !30956)
  call void @llvm.experimental.noalias.scope.decl(metadata !30958)
  call void @llvm.experimental.noalias.scope.decl(metadata !30960)
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !30962
  store i8 0, ptr %7, align 1, !noalias !30962
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !30962
  store ptr %7, ptr %6, align 8, !noalias !30962
  call void asm sideeffect "", "r,~{memory}"(ptr nonnull %6) #92, !noalias !30962
  %426 = load ptr, ptr %6, align 8, !noalias !30962, !noundef !1740
  %427 = ptrtoint ptr %426 to i64
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !30962
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !30962
  %428 = load i64, ptr %355, align 8, !noalias !30962, !noundef !1740
  %429 = icmp ugt i64 %428, %427
  %430 = sub nuw i64 %427, %428
  %431 = icmp ult i64 %430, 131072
  %432 = select i1 %429, i1 true, i1 %431, !prof !10952
  br i1 %432, label %433, label %437, !prof !10952

433:                                              ; preds = %423
; invoke purrdf_stack::is_low_cold
  %434 = invoke noundef zeroext i1 @purrdf_stack::is_low_cold(i64 noundef %427) #91
          to label %435 unwind label %692

435:                                              ; preds = %433
  br i1 %434, label %436, label %437

436:                                              ; preds = %435
  store i64 -9223372036854775784, ptr %360, align 16, !noalias !30966
  store ptr @anon.e5162873a9a3251d11c4df37a70e4654.438, ptr %361, align 8, !noalias !30966
  store i64 41, ptr %384, align 16, !noalias !30966
  store i64 1, ptr %54, align 16, !alias.scope !30953, !noalias !30966
  br label %696

437:                                              ; preds = %435, %423
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !30968
  call void @llvm.lifetime.start.p0(ptr nonnull %27)
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !30968
  %438 = load ptr, ptr %356, align 8, !alias.scope !30958, !noalias !30969, !nonnull !1740, !noundef !1740
  %439 = load i64, ptr %357, align 8, !alias.scope !30958, !noalias !30969, !noundef !1740
; invoke purrdf_sparql_eval::expr::outer_bindings_for_substitution::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::outer_bindings_for_substitution::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %26, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %425, i64 noundef range(i64 0, 1152921504606846976) %424, ptr nonnull %438, i64 %439, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %440 unwind label %692, !inline_history !30970

440:                                              ; preds = %437
  %441 = load i64, ptr %26, align 16, !range !2527, !noalias !30968, !noundef !1740
  %442 = icmp eq i64 %441, -1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %27, ptr noundef nonnull align 8 dereferenceable(48) %358, i64 48, i1 false), !noalias !30968
  br i1 %442, label %448, label %443

443:                                              ; preds = %440
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %362, ptr noundef nonnull align 8 dereferenceable(40) %359, i64 40, i1 false), !noalias !30966
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !30968
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %361, ptr noundef nonnull align 8 dereferenceable(48) %27, i64 48, i1 false), !noalias !30966
  store i64 %441, ptr %360, align 16, !alias.scope !30953, !noalias !30966
  store i64 1, ptr %54, align 16, !alias.scope !30953, !noalias !30966
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  br label %678

444:                                              ; preds = %587, %584, %581, %499, %467, %446
  %445 = phi { ptr, i32 } [ %447, %446 ], [ %500, %499 ], [ %468, %467 ], [ %582, %587 ], [ %582, %581 ], [ %582, %584 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>(ptr noalias nofree noundef align 8 dereferenceable(48) %28) #89
          to label %689 unwind label %474, !noalias !30971, !inline_history !30970

446:                                              ; preds = %676, %595, %501, %479, %469, %459
  %447 = landingpad { ptr, i32 }
          cleanup
  br label %444

448:                                              ; preds = %440
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !30968
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %28, ptr noundef nonnull align 8 dereferenceable(48) %27, i64 48, i1 false), !noalias !30968
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !30968
  %449 = getelementptr inbounds nuw i8, ptr %403, i64 16
  %450 = getelementptr inbounds nuw i8, ptr %403, i64 80
  %451 = load ptr, ptr %450, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
  %452 = getelementptr inbounds nuw i8, ptr %451, i64 16
  store i64 3, ptr %25, align 8, !noalias !30968
  store ptr %449, ptr %363, align 8, !noalias !30968
  store ptr %452, ptr %364, align 8, !noalias !30968
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !30968
  %453 = load ptr, ptr %365, align 8, !alias.scope !30956, !noalias !30972, !noundef !1740
  %454 = load ptr, ptr %366, align 8, !alias.scope !30956, !noalias !30972
  %455 = icmp eq ptr %453, null
  %456 = icmp eq ptr %454, null
  %457 = select i1 %456, i64 2, i64 1
  %458 = select i1 %455, i64 0, i64 %457
  switch i64 %458, label %default.unreachable544 [
    i64 0, label %465
    i64 1, label %459
    i64 2, label %464
  ]

default.unreachable544:                           ; preds = %448
  unreachable

459:                                              ; preds = %448
  %460 = select i1 %455, i1 true, i1 %456
  %461 = getelementptr inbounds nuw i8, ptr %454, i64 16
  %462 = select i1 %460, ptr undef, ptr %461
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %462) ]
  %463 = getelementptr inbounds nuw i8, ptr %403, i64 48
; invoke purrdf_sparql_eval::deferred_exists::with_row
  invoke void @purrdf_sparql_eval::deferred_exists::with_row(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(address) dereferenceable(48) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %462, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %28, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(32) %463)
          to label %471 unwind label %446, !noalias !30971, !inline_history !30970

464:                                              ; preds = %448
  store i64 -1, ptr %24, align 8, !noalias !30968
  br label %479

465:                                              ; preds = %448
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !30968
  store i64 0, ptr %23, align 8, !alias.scope !30973, !noalias !30968
  store ptr inttoptr (i64 8 to ptr), ptr %367, align 8, !alias.scope !30973, !noalias !30968
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %368, i8 0, i64 16, i1 false), !alias.scope !30973, !noalias !30968
  store ptr inttoptr (i64 8 to ptr), ptr %369, align 8, !alias.scope !30973, !noalias !30968
  store i64 0, ptr %370, align 8, !alias.scope !30973, !noalias !30968
  %466 = getelementptr inbounds nuw i8, ptr %403, i64 48
; invoke purrdf_sparql_eval::deferred_exists::with_row
  invoke void @purrdf_sparql_eval::deferred_exists::with_row(ptr noalias nofree noundef nonnull sret([48 x i8]) align 8 captures(address) dereferenceable(48) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %23, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %28, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(32) %466)
          to label %469 unwind label %467, !noalias !30971, !inline_history !30970

467:                                              ; preds = %465
  %468 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>(ptr noalias nofree noundef align 8 dereferenceable(48) %23) #89
          to label %444 unwind label %474, !noalias !30971, !inline_history !30970

469:                                              ; preds = %465
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>(ptr noalias nofree noundef align 8 dereferenceable(48) %23)
          to label %470 unwind label %446, !noalias !30971, !inline_history !30970

470:                                              ; preds = %469
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !30968
  br label %471

471:                                              ; preds = %470, %459
  %472 = load i64, ptr %24, align 8, !range !2059, !noalias !30968, !noundef !1740
  %473 = icmp eq i64 %472, -1
  br i1 %473, label %479, label %476

474:                                              ; preds = %688, %679, %666, %653, %587, %575, %532, %499, %467, %444
  %475 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !30971, !inline_history !30970
  unreachable

476:                                              ; preds = %471
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !30968
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %22, ptr noundef nonnull align 8 dereferenceable(48) %24, i64 48, i1 false), !noalias !30968
  %477 = load ptr, ptr %371, align 8, !alias.scope !30960, !noalias !30971, !noundef !1740
  %478 = icmp eq ptr %477, null
  br i1 %478, label %488, label %483

479:                                              ; preds = %471, %464
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !30968
  %480 = getelementptr inbounds nuw i8, ptr %403, i64 40
  %481 = load ptr, ptr %480, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
; invoke purrdf_sparql_eval::deferred_exists::nested_sites::<purrdf_core::ir::dataset::RdfDataset>
  %482 = invoke fastcc { i64, ptr } @purrdf_sparql_eval::deferred_exists::nested_sites::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %481, ptr noalias nofree noundef readonly align 8 captures(address) dereferenceable(24) %25, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %519 unwind label %446, !noalias !30971, !inline_history !30970

483:                                              ; preds = %476
  %484 = load ptr, ptr %450, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
  %485 = getelementptr inbounds nuw i8, ptr %484, i64 40
  %486 = load i64, ptr %485, align 8, !noalias !30971, !noundef !1740
  %487 = icmp eq i64 %486, 0
  br i1 %487, label %488, label %493

488:                                              ; preds = %496, %483, %476
  %489 = phi i1 [ false, %483 ], [ true, %496 ], [ false, %476 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !30968
  %490 = getelementptr inbounds nuw i8, ptr %403, i64 40
  %491 = load ptr, ptr %490, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
; invoke purrdf_sparql_eval::binop::eval_substituted_delivered::<purrdf_core::ir::dataset::RdfDataset, false, false, ()>
  invoke fastcc void @purrdf_sparql_eval::binop::eval_substituted_delivered::<purrdf_core::ir::dataset::RdfDataset, false, false, ()>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(112) %21, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %491, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %22, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %25, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %492 unwind label %499, !noalias !30971, !inline_history !30970

492:                                              ; preds = %488
  br i1 %489, label %502, label %501

493:                                              ; preds = %483
  %494 = atomicrmw add ptr %484, i64 1 monotonic, align 8, !noalias !30971
  %495 = icmp slt i64 %494, 0
  br i1 %495, label %498, label %496

496:                                              ; preds = %493
  %497 = load ptr, ptr %450, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
; invoke <alloc::vec::Vec<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>>::push_mut
  invoke fastcc void @<alloc::vec::Vec<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>>::push_mut(ptr noalias nofree noundef align 8 dereferenceable(24) %372, ptr noundef nonnull %497)
          to label %488 unwind label %499

498:                                              ; preds = %493
  call void @llvm.trap()
  unreachable

499:                                              ; preds = %496, %488
  %500 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>(ptr noalias nofree noundef align 8 dereferenceable(48) %22) #89
          to label %444 unwind label %474, !noalias !30971, !inline_history !30970

501:                                              ; preds = %516, %492
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(112) %54, ptr noundef nonnull align 16 dereferenceable(112) %21, i64 112, i1 false), !noalias !30966
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !30968
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>(ptr noalias nofree noundef align 8 dereferenceable(48) %22)
          to label %517 unwind label %446, !noalias !30971, !inline_history !30970

502:                                              ; preds = %492
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !30968
  %503 = load i64, ptr %373, align 8, !alias.scope !30960, !noalias !30971, !noundef !1740
  %504 = icmp eq i64 %503, 0
  br i1 %504, label %516, label %505

505:                                              ; preds = %502
  %506 = add nsw i64 %503, -1
  store i64 %506, ptr %373, align 8, !alias.scope !30960, !noalias !30971
  %507 = load i64, ptr %372, align 8, !range !1835, !alias.scope !30960, !noalias !30971, !noundef !1740
  %508 = icmp samesign ult i64 %506, %507
  call void @llvm.assume(i1 %508)
  %509 = load ptr, ptr %374, align 16, !alias.scope !30960, !noalias !30971, !nonnull !1740, !noundef !1740
  %510 = icmp ult i64 %503, 1152921504606846977
  call void @llvm.assume(i1 %510)
  %511 = getelementptr inbounds nuw [8 x i8], ptr %509, i64 %506
  %512 = load ptr, ptr %511, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
  store ptr %512, ptr %20, align 8, !noalias !30968
  %513 = atomicrmw sub ptr %512, i64 1 release, align 8, !noalias !30976
  %514 = icmp eq i64 %513, 1
  br i1 %514, label %515, label %516

515:                                              ; preds = %505
  fence acquire, !noalias !30971
; call <alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>::drop_slow
  call void @<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::expr::SubstitutionSource, purrdf_hash::fixed::FixedState>>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %20) #91, !noalias !30971
  br label %516

516:                                              ; preds = %515, %505, %502
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !30968
  br label %501

517:                                              ; preds = %501
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !30968
  br label %518

518:                                              ; preds = %677, %517
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !30968
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !30968
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>(ptr noalias nofree noundef align 8 dereferenceable(48) %28)
          to label %678 unwind label %692, !inline_history !30970

519:                                              ; preds = %479
  %520 = extractvalue { i64, ptr } %482, 0
  %521 = extractvalue { i64, ptr } %482, 1
  store i64 %520, ptr %19, align 8, !noalias !30968
  store ptr %521, ptr %375, align 8, !noalias !30968
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !30968
  store ptr null, ptr %18, align 8, !noalias !30968
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !30968
  store ptr null, ptr %17, align 8, !noalias !30968
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !30968
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !30968
  %522 = getelementptr inbounds nuw i8, ptr %403, i64 48
; invoke <purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::then
  %523 = invoke { ptr, ptr } @<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::then(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %365, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %28, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(32) %522)
          to label %529 unwind label %524, !noalias !30983, !inline_history !30970

524:                                              ; preds = %660, %568, %519
  %525 = phi ptr [ null, %519 ], [ %563, %568 ], [ %558, %660 ]
  %526 = landingpad { ptr, i32 }
          cleanup
  br label %681

527:                                              ; preds = %579, %573
  %528 = landingpad { ptr, i32 }
          cleanup
  br label %666

529:                                              ; preds = %519
  %530 = extractvalue { ptr, ptr } %523, 0
  %531 = extractvalue { ptr, ptr } %523, 1
  store ptr %530, ptr %15, align 8, !noalias !30968
  store ptr %531, ptr %376, align 8, !noalias !30968
; invoke <purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::layers
  invoke void @<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>::layers(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %16, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %15)
          to label %537 unwind label %535, !noalias !30971, !inline_history !30970

532:                                              ; preds = %552, %548, %535
  %533 = phi ptr [ null, %535 ], [ %549, %548 ], [ %549, %552 ]
  %534 = phi { ptr, i32 } [ %536, %535 ], [ %550, %548 ], [ %550, %552 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>(ptr noalias nofree noundef align 8 dereferenceable(16) %15) #89
          to label %681 unwind label %474, !noalias !30971, !inline_history !30970

535:                                              ; preds = %529
  %536 = landingpad { ptr, i32 }
          cleanup
  br label %532

537:                                              ; preds = %529
  %538 = load ptr, ptr %377, align 8, !noalias !30968, !nonnull !1740, !noundef !1740
  %539 = load i64, ptr %16, align 8, !range !1835, !noalias !30968, !noundef !1740
  %540 = load i64, ptr %378, align 8, !noalias !30968, !noundef !1740
  %541 = icmp ult i64 %540, 1152921504606846976
  call void @llvm.assume(i1 %541)
  %542 = shl nuw nsw i64 %540, 3
  %543 = getelementptr inbounds nuw i8, ptr %538, i64 %542
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !30968
  %544 = icmp eq i64 %540, 0
  br i1 %544, label %.loopexit107, label %545

545:                                              ; preds = %537
  %546 = shl nuw nsw i64 %520, 4
  %547 = getelementptr inbounds nuw i8, ptr %521, i64 %546
  br label %556

548:                                              ; preds = %679, %653, %623, %554
  %549 = phi ptr [ %558, %554 ], [ %558, %679 ], [ %611, %623 ], [ %654, %653 ]
  %550 = phi { ptr, i32 } [ %555, %554 ], [ %680, %679 ], [ %624, %623 ], [ %655, %653 ]
  %551 = icmp eq i64 %539, 0
  br i1 %551, label %532, label %552

552:                                              ; preds = %548
  %553 = shl nuw i64 %539, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %538, i64 noundef %553, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !30984
  br label %532

554:                                              ; preds = %609
  %555 = landingpad { ptr, i32 }
          cleanup
  br label %548

556:                                              ; preds = %651, %545
  %557 = phi ptr [ null, %545 ], [ %608, %651 ]
  %558 = phi ptr [ null, %545 ], [ %611, %651 ]
  %559 = phi ptr [ %538, %545 ], [ %560, %651 ]
  %560 = getelementptr inbounds nuw i8, ptr %559, i64 8
  %561 = load ptr, ptr %559, align 8, !noalias !30987, !nonnull !1740, !align !1836, !noundef !1740
  %562 = icmp eq ptr %557, null
  br i1 %562, label %598, label %600

.loopexit107:                                     ; preds = %651, %537
  %563 = phi ptr [ null, %537 ], [ %611, %651 ]
  %564 = phi ptr [ null, %537 ], [ %608, %651 ]
  %565 = icmp eq i64 %539, 0
  br i1 %565, label %568, label %566

566:                                              ; preds = %.loopexit107
  %567 = shl nuw i64 %539, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %538, i64 noundef %567, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !30990
  br label %568

568:                                              ; preds = %566, %.loopexit107
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>(ptr noalias nofree noundef align 8 dereferenceable(16) %15)
          to label %569 unwind label %524, !noalias !30971, !inline_history !30970

569:                                              ; preds = %568
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !30968
  %570 = icmp eq ptr %564, null
  br i1 %570, label %571, label %573

571:                                              ; preds = %569
  %572 = load ptr, ptr %480, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
  br label %573

573:                                              ; preds = %571, %569
  %574 = phi ptr [ %572, %571 ], [ %564, %569 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !30968
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !30968
  store ptr null, ptr %11, align 8, !noalias !30968
; invoke <purrdf_sparql_eval::eval::EvalCtx>::enter_substituted_exists
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::enter_substituted_exists(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %12, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(32) %11, ptr noundef %563)
          to label %577 unwind label %527, !noalias !30971, !inline_history !30970

575:                                              ; preds = %577
  %576 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12) #89
          to label %666 unwind label %474, !noalias !30971, !inline_history !30970

577:                                              ; preds = %573
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !30968
  %578 = load ptr, ptr %385, align 8, !noalias !30968, !nonnull !1740, !align !30238, !noundef !1740
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %54, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %574, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %578, ptr noundef nonnull readonly align 8 dereferenceable(144) %574)
          to label %579 unwind label %575, !inline_history !30993

579:                                              ; preds = %577
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::eval::EvalScopeGuard<purrdf_core::ir::dataset::RdfDataset>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12)
          to label %580 unwind label %527, !noalias !30971, !inline_history !30970

580:                                              ; preds = %579
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !30968
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !30968
; invoke core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
  invoke fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>(ptr noalias nofree noundef align 8 dereferenceable(8) %18)
          to label %590 unwind label %588, !noalias !30971, !inline_history !30970

581:                                              ; preds = %666, %588
  %582 = phi { ptr, i32 } [ %589, %588 ], [ %667, %666 ]
  %583 = icmp eq i64 %520, 0
  br i1 %583, label %444, label %584

584:                                              ; preds = %581
  %585 = atomicrmw sub ptr %521, i64 1 release, align 8, !noalias !30994
  %586 = icmp eq i64 %585, 1
  br i1 %586, label %587, label %444

587:                                              ; preds = %584
  fence acquire, !noalias !31001
; invoke <alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %375) #91
          to label %444 unwind label %474

588:                                              ; preds = %670, %580
  %589 = landingpad { ptr, i32 }
          cleanup
  br label %581

590:                                              ; preds = %580
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !30968
  %591 = icmp eq i64 %520, 0
  br i1 %591, label %596, label %592

592:                                              ; preds = %590
  %593 = atomicrmw sub ptr %521, i64 1 release, align 8, !noalias !31002
  %594 = icmp eq i64 %593, 1
  br i1 %594, label %595, label %596

595:                                              ; preds = %592
  fence acquire, !noalias !31009
; invoke <alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %375) #91
          to label %596 unwind label %446

596:                                              ; preds = %595, %592, %590
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !30968
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !30968
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !30968
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::expr::SubstitutionRow>(ptr noalias nofree noundef align 8 dereferenceable(48) %28)
          to label %597 unwind label %692, !inline_history !30970

597:                                              ; preds = %596
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !30968
  br label %696

598:                                              ; preds = %556
  %599 = load ptr, ptr %480, align 8, !noalias !30971, !nonnull !1740, !noundef !1740
  br label %600

600:                                              ; preds = %598, %556
  %601 = phi ptr [ %599, %598 ], [ %557, %556 ]
  %602 = icmp eq ptr %558, null
  %603 = getelementptr inbounds nuw i8, ptr %558, i64 16
  %604 = select i1 %602, ptr null, ptr %603
  store ptr %604, ptr %379, align 8
  store ptr %547, ptr %380, align 8
  store i64 0, ptr %381, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %13, ptr noundef nonnull align 8 dereferenceable(32) @anon.e5162873a9a3251d11c4df37a70e4654.29.llvm.12908414067662811932, i64 32, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !30968
; invoke purrdf_sparql_eval::expr::substitute_pattern_deferring::<false>
  invoke fastcc void @purrdf_sparql_eval::expr::substitute_pattern_deferring::<false>(ptr noalias nofree noundef align 16 captures(address) dereferenceable(96) %14, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %601, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(48) %561, ptr noalias nofree noundef align 8 dereferenceable(56) %13)
          to label %605 unwind label %679, !inline_history !30970

605:                                              ; preds = %600
  %606 = load i64, ptr %14, align 16, !range !2527, !noalias !30968, !noundef !1740
  %607 = icmp eq i64 %606, -1
  %608 = load ptr, ptr %382, align 8, !noalias !30968
  br i1 %607, label %610, label %609

609:                                              ; preds = %605
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %384, ptr noundef nonnull align 16 dereferenceable(80) %383, i64 80, i1 false), !noalias !30966
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !30968
  store i64 %606, ptr %360, align 16, !alias.scope !30953, !noalias !30966
  store ptr %608, ptr %361, align 8, !alias.scope !30953, !noalias !30966
  store i64 1, ptr %54, align 16, !alias.scope !30953, !noalias !30966
; invoke core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
  invoke fastcc void @core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(56) %13)
          to label %656 unwind label %554

610:                                              ; preds = %605
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !30968
; invoke <purrdf_sparql_eval::expr::Deferral>::into_placeholders
  %611 = invoke noundef ptr @<purrdf_sparql_eval::expr::Deferral>::into_placeholders(ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(56) %13)
          to label %614 unwind label %612

612:                                              ; preds = %610
  %613 = landingpad { ptr, i32 }
          cleanup
  br label %653

614:                                              ; preds = %610
  br i1 %602, label %621, label %615

615:                                              ; preds = %614
  %616 = atomicrmw sub ptr %558, i64 1 release, align 8, !noalias !31010
  %617 = icmp eq i64 %616, 1
  br i1 %617, label %618, label %621

618:                                              ; preds = %615
  fence acquire, !noalias !30971
; invoke <alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow
  invoke void @<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %17) #91
          to label %621 unwind label %619

619:                                              ; preds = %618
  %620 = landingpad { ptr, i32 }
          cleanup
  store ptr %611, ptr %17, align 8, !noalias !30968
  br label %653

621:                                              ; preds = %618, %615, %614
  store ptr %611, ptr %17, align 8, !noalias !30968
  br i1 %562, label %651, label %622

622:                                              ; preds = %621
; invoke core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::GraphPattern>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_algebra::algebra::GraphPattern>(ptr noalias nofree noundef nonnull align 8 dereferenceable(144) %557) #94
          to label %625 unwind label %623, !noalias !31017, !inline_history !31022

623:                                              ; preds = %622
  %624 = landingpad { ptr, i32 }
          cleanup
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %557, i64 noundef 144, i64 noundef 8) #92, !noalias !31017, !inline_history !31022
  store ptr %608, ptr %18, align 8, !noalias !30968
  br label %548

625:                                              ; preds = %622
  %626 = load i64, ptr %396, align 8, !noalias !31017, !noundef !1740
  %627 = call i64 @llvm.sadd.sat.i64(i64 %626, i64 -144)
  store i64 %627, ptr %396, align 8, !noalias !31017
  %628 = load i64, ptr %397, align 8, !noalias !31017, !noundef !1740
  %629 = icmp slt i64 %627, %628
  br i1 %629, label %630, label %.preheader921

630:                                              ; preds = %625
  store i64 %627, ptr %397, align 8, !noalias !31017
  br label %.preheader921

.preheader921:                                    ; preds = %630, %625
  br label %631

631:                                              ; preds = %.preheader921, %634
  %632 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !31017
  %633 = icmp slt i64 %632, 0
  br i1 %633, label %634, label %__rustc::__rust_dealloc (.exit)

634:                                              ; preds = %631
  %635 = add nsw i64 %632, 1
  %636 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %632, i64 %635 acq_rel acquire, align 8, !noalias !31017
  %637 = extractvalue { i64, i1 } %636, 1
  br i1 %637, label %638, label %631

638:                                              ; preds = %634
  %639 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 144 monotonic, align 8, !noalias !31017
  %640 = call i64 @llvm.sadd.sat.i64(i64 %639, i64 -144)
  %641 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !31017
  br label %642

642:                                              ; preds = %645, %638
  %643 = phi i64 [ %641, %638 ], [ %648, %645 ]
  %644 = icmp slt i64 %640, %643
  br i1 %644, label %645, label %649

645:                                              ; preds = %642
  %646 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %643, i64 %640 monotonic monotonic, align 8, !noalias !31017
  %647 = extractvalue { i64, i1 } %646, 1
  %648 = extractvalue { i64, i1 } %646, 0
  br i1 %647, label %649, label %642

649:                                              ; preds = %645, %642
  %650 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !31017
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %631, %649
  call void @free(ptr noundef nonnull %557) #92, !noalias !31017
  br label %651

651:                                              ; preds = %__rustc::__rust_dealloc (.exit), %621
  store ptr %608, ptr %18, align 8, !noalias !30968
  %652 = icmp eq ptr %560, %543
  br i1 %652, label %.loopexit107, label %556

653:                                              ; preds = %619, %612
  %654 = phi ptr [ %558, %612 ], [ %611, %619 ]
  %655 = phi { ptr, i32 } [ %613, %612 ], [ %620, %619 ]
; invoke core::ptr::drop_glue::<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>(ptr %608) #89
          to label %548 unwind label %474, !noalias !30971, !inline_history !30970

656:                                              ; preds = %609
  %657 = icmp eq i64 %539, 0
  br i1 %657, label %660, label %658

658:                                              ; preds = %656
  %659 = shl nuw i64 %539, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %538, i64 noundef %659, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !31023
  br label %660

660:                                              ; preds = %658, %656
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::SubstitutionEnv>(ptr noalias nofree noundef align 8 dereferenceable(16) %15)
          to label %661 unwind label %524, !noalias !30971, !inline_history !30970

661:                                              ; preds = %660
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !30968
  br i1 %602, label %670, label %662

662:                                              ; preds = %661
  %663 = atomicrmw sub ptr %558, i64 1 release, align 8, !noalias !31026
  %664 = icmp eq i64 %663, 1
  br i1 %664, label %665, label %670

665:                                              ; preds = %662
  fence acquire, !noalias !30971
; invoke <alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow
  invoke void @<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %17) #91
          to label %670 unwind label %668

666:                                              ; preds = %688, %685, %681, %668, %575, %527
  %667 = phi { ptr, i32 } [ %669, %668 ], [ %576, %575 ], [ %528, %527 ], [ %682, %688 ], [ %682, %681 ], [ %682, %685 ]
; invoke core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
  invoke fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>(ptr noalias nofree noundef align 8 dereferenceable(8) %18) #89
          to label %581 unwind label %474, !noalias !30971, !inline_history !30970

668:                                              ; preds = %665
  %669 = landingpad { ptr, i32 }
          cleanup
  br label %666

670:                                              ; preds = %665, %662, %661
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !30968
; invoke core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>
  invoke fastcc void @core::ptr::drop_glue::<core::option::Option<alloc::boxed::Box<purrdf_sparql_algebra::algebra::GraphPattern>>>(ptr noalias nofree noundef align 8 dereferenceable(8) %18)
          to label %671 unwind label %588, !noalias !30971, !inline_history !30970

671:                                              ; preds = %670
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !30968
  %672 = icmp eq i64 %520, 0
  br i1 %672, label %677, label %673

673:                                              ; preds = %671
  %674 = atomicrmw sub ptr %521, i64 1 release, align 8, !noalias !31033
  %675 = icmp eq i64 %674, 1
  br i1 %675, label %676, label %677

676:                                              ; preds = %673
  fence acquire, !noalias !31040
; invoke <alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::deferred_exists::NestedSites>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %375) #91
          to label %677 unwind label %446

677:                                              ; preds = %676, %673, %671
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !30968
  br label %518

678:                                              ; preds = %518, %443
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !30968
  br label %696

679:                                              ; preds = %600
  %680 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>
  invoke fastcc void @core::ptr::drop_glue::<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(56) %13)
          to label %548 unwind label %474

681:                                              ; preds = %532, %524
  %682 = phi { ptr, i32 } [ %526, %524 ], [ %534, %532 ]
  %683 = phi ptr [ %525, %524 ], [ %533, %532 ]
  %684 = icmp eq ptr %683, null
  br i1 %684, label %666, label %685

685:                                              ; preds = %681
  %686 = atomicrmw sub ptr %683, i64 1 release, align 8, !noalias !31041
  %687 = icmp eq i64 %686, 1
  br i1 %687, label %688, label %666

688:                                              ; preds = %685
  fence acquire, !noalias !30971
; invoke <alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow
  invoke void @<alloc::sync::Arc<std::collections::hash::map::HashMap<usize, purrdf_sparql_eval::deferred_exists::Deferred, purrdf_hash::fixed::FixedState>>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %17) #91
          to label %666 unwind label %474

689:                                              ; preds = %1241, %1237, %995, %928, %823, %821, %808, %694, %692, %444
  %690 = phi i8 [ 0, %995 ], [ 1, %928 ], [ 1, %1237 ], [ 1, %1241 ], [ 1, %444 ], [ 1, %823 ], [ 1, %821 ], [ 1, %808 ], [ 1, %694 ], [ 1, %692 ]
  %691 = phi { ptr, i32 } [ %996, %995 ], [ %929, %928 ], [ %1238, %1237 ], [ %1238, %1241 ], [ %445, %444 ], [ %824, %823 ], [ %822, %821 ], [ %809, %808 ], [ %695, %694 ], [ %693, %692 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %58) #89
          to label %339 unwind label %265

692:                                              ; preds = %712, %596, %518, %437, %433
  %693 = landingpad { ptr, i32 }
          cleanup
  br label %689

694:                                              ; preds = %.loopexit108
  %695 = landingpad { ptr, i32 }
          cleanup
  br label %689

696:                                              ; preds = %678, %597, %436
  %697 = load i64, ptr %54, align 16, !range !1739, !noundef !1740
  %698 = trunc nuw i64 %697 to i1
  br i1 %698, label %699, label %701

699:                                              ; preds = %696
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %55, ptr noundef nonnull align 16 dereferenceable(96) %360, i64 96, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  %700 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %700, ptr noundef nonnull align 16 dereferenceable(96) %55, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
  br label %706

701:                                              ; preds = %696
  %702 = load i64, ptr %386, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(88) %55, ptr noundef nonnull align 16 dereferenceable(88) %360, i64 88, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %56, ptr noundef nonnull align 16 dereferenceable(32) %55, i64 32, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %57, ptr noundef nonnull align 16 dereferenceable(56) %393, i64 56, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
  br label %703

703:                                              ; preds = %720, %701
  %704 = phi i64 [ %721, %720 ], [ %702, %701 ]
  %705 = icmp eq i64 %704, -1
  br i1 %705, label %725, label %722

706:                                              ; preds = %718, %699
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  call void @llvm.lifetime.end.p0(ptr nonnull %57)
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %58)
          to label %833 unwind label %342

707:                                              ; preds = %410
  %708 = load ptr, ptr %417, align 8, !nonnull !1740, !noundef !1740
  %709 = getelementptr inbounds nuw i8, ptr %401, i64 16
  %710 = load i64, ptr %709, align 8, !noundef !1740
  %711 = add i64 %710, -1
  br label %712

712:                                              ; preds = %707, %410
  %713 = phi i64 [ %711, %707 ], [ %415, %410 ]
  %714 = phi ptr [ %708, %707 ], [ %417, %410 ]
; invoke purrdf_sparql_eval::binop::eval_correlated::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::binop::eval_correlated::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(address) dereferenceable(112) %51, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %714, i64 noundef %713, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %389, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %53, ptr noalias nofree noundef align 16 dereferenceable(1248) %4)
          to label %715 unwind label %692

715:                                              ; preds = %712
  %716 = load i64, ptr %51, align 16, !range !1739, !noundef !1740
  %717 = trunc nuw i64 %716 to i1
  br i1 %717, label %718, label %720

718:                                              ; preds = %715
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %52, ptr noundef nonnull align 16 dereferenceable(96) %394, i64 96, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %51)
  %719 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %719, ptr noundef nonnull align 16 dereferenceable(96) %52, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %52)
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  br label %706

720:                                              ; preds = %715
  %721 = load i64, ptr %390, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(88) %52, ptr noundef nonnull align 16 dereferenceable(88) %394, i64 88, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %51)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %56, ptr noundef nonnull align 16 dereferenceable(32) %52, i64 32, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %57, ptr noundef nonnull align 16 dereferenceable(56) %395, i64 56, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %52)
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  br label %703

722:                                              ; preds = %703
  call void @llvm.lifetime.start.p0(ptr nonnull %50)
  store i64 %704, ptr %50, align 8
  %723 = getelementptr inbounds nuw i8, ptr %50, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %723, ptr noundef nonnull align 8 dereferenceable(32) %56, i64 32, i1 false)
  %724 = getelementptr inbounds nuw i8, ptr %50, i64 40
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %724, ptr noundef nonnull align 8 dereferenceable(56) %57, i64 56, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %49)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %49, ptr noalias nofree noundef align 8 dereferenceable(104) %39, i64 noundef 1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %50)
          to label %825 unwind label %823

725:                                              ; preds = %703
  %.sroa.4.0.copyload = load ptr, ptr %.sroa.4.0..sroa_idx, align 8, !nonnull !1740, !noundef !1740
  %726 = getelementptr i8, ptr %.sroa.4.0.copyload, i64 24
  %727 = load ptr, ptr %726, align 8, !nonnull !1740, !noundef !1740
  %728 = getelementptr i8, ptr %.sroa.4.0.copyload, i64 32
  %729 = load i64, ptr %728, align 8, !noundef !1740
  %.idx = shl nuw nsw i64 %729, 4
  %730 = getelementptr inbounds nuw i8, ptr %727, i64 %.idx
  %731 = icmp eq i64 %729, 0
  br i1 %731, label %._crit_edge, label %.lr.ph

732:                                              ; preds = %743
  %733 = getelementptr inbounds nuw i8, ptr %735, i64 16
  %734 = icmp eq ptr %733, %730
  br i1 %734, label %._crit_edge, label %.lr.ph

.lr.ph:                                           ; preds = %725, %732
  %735 = phi ptr [ %733, %732 ], [ %727, %725 ]
  %736 = load ptr, ptr %735, align 8, !nonnull !1740, !noundef !1740
  %737 = atomicrmw add ptr %736, i64 1 monotonic, align 8
  %738 = icmp slt i64 %737, 0
  br i1 %738, label %748, label %743

._crit_edge:                                      ; preds = %732, %725
  call void @llvm.lifetime.start.p0(ptr nonnull %48)
  call void @llvm.lifetime.start.p0(ptr nonnull %47)
  %739 = load i64, ptr %401, align 8, !range !1778, !noundef !1740
  %740 = add i64 %739, -1
  %741 = icmp ugt i64 %740, 4
  %742 = getelementptr inbounds nuw i8, ptr %401, i64 8
  br i1 %741, label %751, label %749

743:                                              ; preds = %.lr.ph
  %744 = getelementptr inbounds nuw i8, ptr %735, i64 8
  %745 = load ptr, ptr %735, align 8, !nonnull !1740, !noundef !1740
  %746 = load i64, ptr %744, align 8, !noundef !1740
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %747 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %59, ptr noundef nonnull %745, i64 noundef %746)
          to label %732 unwind label %817

748:                                              ; preds = %.lr.ph
  call void @llvm.trap()
  unreachable

749:                                              ; preds = %._crit_edge
  %750 = shl nuw nsw i64 %740, 3
  br label %797

751:                                              ; preds = %._crit_edge
  %752 = load ptr, ptr %742, align 8, !nonnull !1740, !noundef !1740
  %753 = getelementptr inbounds nuw i8, ptr %401, i64 16
  %754 = load i64, ptr %753, align 8, !noundef !1740
  %755 = add i64 %754, -1
  call void @llvm.experimental.noalias.scope.decl(metadata !31048)
  call void @llvm.experimental.noalias.scope.decl(metadata !31051)
  %756 = icmp samesign ult i64 %755, 5
  %757 = shl nuw nsw i64 %755, 3
  br i1 %756, label %797, label %758

758:                                              ; preds = %751
  call void @llvm.experimental.noalias.scope.decl(metadata !31053)
; call __rustc::__rust_alloc
  %759 = call noundef align 4 ptr @__rustc::__rust_alloc(i64 noundef %757, i64 noundef range(i64 1, 17) 4) #92, !noalias !31056
  %760 = icmp eq ptr %759, null
  br i1 %760, label %761, label %763

761:                                              ; preds = %758
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 4, i64 %757) #93
          to label %762 unwind label %819

762:                                              ; preds = %761
  unreachable

763:                                              ; preds = %758
  %764 = getelementptr inbounds nuw [8 x i8], ptr %752, i64 %755
  %765 = add i64 %754, 2305843009213693951
  %766 = and i64 %765, 2305843009213693951
  %767 = add i64 %754, -2
  %768 = call i64 @llvm.umin.i64(i64 %766, i64 %767)
  %min.iters.check = icmp samesign ult i64 %768, 16
  br i1 %min.iters.check, label %scalar.ph.preheader, label %vector.ph

vector.ph:                                        ; preds = %763
  %769 = add nuw nsw i64 %768, 1
  %n.mod.vf = and i64 %769, 15
  %770 = icmp eq i64 %n.mod.vf, 0
  %771 = select i1 %770, i64 16, i64 %n.mod.vf
  %n.vec = sub nsw i64 %769, %771
  %772 = sub i64 %755, %n.vec
  %773 = shl i64 %n.vec, 3
  %774 = getelementptr i8, ptr %752, i64 %773
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %775 = shl i64 %index, 3
  %next.gep = getelementptr i8, ptr %752, i64 %775
  %wide.vec = load <32 x i32>, ptr %next.gep, align 4, !alias.scope !31060, !noalias !31061
  %strided.vec = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec879 = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %776 = icmp eq <16 x i32> %strided.vec, splat (i32 2)
  %777 = select <16 x i1> %776, <16 x i32> undef, <16 x i32> %strided.vec879
  %778 = getelementptr inbounds nuw [8 x i8], ptr %759, i64 %index
  %interleaved.vec = shufflevector <16 x i32> %strided.vec, <16 x i32> %777, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec, ptr %778, align 4, !noalias !31062
  %index.next = add nuw i64 %index, 16
  %779 = icmp eq i64 %index.next, %n.vec
  br i1 %779, label %scalar.ph.preheader, label %vector.body, !llvm.loop !31063

scalar.ph.preheader:                              ; preds = %vector.body, %763
  %.ph = phi i64 [ %755, %763 ], [ %772, %vector.body ]
  %.ph922 = phi ptr [ %752, %763 ], [ %774, %vector.body ]
  %.ph923 = phi i64 [ 0, %763 ], [ %n.vec, %vector.body ]
  br label %scalar.ph

scalar.ph:                                        ; preds = %scalar.ph.preheader, %784
  %780 = phi i64 [ %785, %784 ], [ %.ph, %scalar.ph.preheader ]
  %781 = phi ptr [ %787, %784 ], [ %.ph922, %scalar.ph.preheader ]
  %782 = phi i64 [ %786, %784 ], [ %.ph923, %scalar.ph.preheader ]
  %783 = icmp eq ptr %781, %764
  br i1 %783, label %796, label %784

784:                                              ; preds = %scalar.ph
  %785 = add nsw i64 %780, -1
  %786 = add nuw nsw i64 %782, 1
  %787 = getelementptr inbounds nuw i8, ptr %781, i64 8
  %788 = load i32, ptr %781, align 4, !range !1785, !alias.scope !31060, !noalias !31061, !noundef !1740
  %789 = getelementptr i8, ptr %781, i64 4
  %790 = load i32, ptr %789, align 4, !alias.scope !31060, !noalias !31061
  %791 = icmp eq i32 %788, 2
  %792 = select i1 %791, i32 undef, i32 %790
  %793 = getelementptr inbounds nuw [8 x i8], ptr %759, i64 %782
  store i32 %788, ptr %793, align 4, !noalias !31062
  %794 = getelementptr inbounds nuw i8, ptr %793, i64 4
  store i32 %792, ptr %794, align 4, !noalias !31062
  %795 = icmp eq i64 %785, 0
  br i1 %795, label %796, label %scalar.ph, !llvm.loop !31064

796:                                              ; preds = %784, %scalar.ph
  store ptr %759, ptr %47, align 8, !alias.scope !31048, !noalias !31065
  store i64 %754, ptr %.8..8..8..sroa_idx, align 8, !alias.scope !31048, !noalias !31065
  br label %801

797:                                              ; preds = %751, %749
  %798 = phi i64 [ %750, %749 ], [ %757, %751 ]
  %799 = phi ptr [ %742, %749 ], [ %752, %751 ]
  %800 = phi i64 [ %739, %749 ], [ %754, %751 ]
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 8 %47, ptr nonnull readonly align 4 %799, i64 %798, i1 false), !alias.scope !31068
  br label %801

801:                                              ; preds = %797, %796
  %802 = phi i64 [ %800, %797 ], [ %754, %796 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %392, ptr noundef nonnull align 8 dereferenceable(32) %56, i64 32, i1 false)
  store i64 %802, ptr %48, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %391, ptr noundef nonnull align 8 dereferenceable(32) %47, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  call void @llvm.experimental.noalias.scope.decl(metadata !31069)
  %803 = load i64, ptr %58, align 8, !range !1835, !alias.scope !31069, !noalias !31072, !noundef !1740
  %804 = icmp eq i64 %400, %803
  br i1 %804, label %805, label %812

805:                                              ; preds = %801
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::governor::soundness::NodeAnalysis>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::governor::soundness::NodeAnalysis>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %58)
          to label %806 unwind label %808, !noalias !31072

806:                                              ; preds = %805
  %807 = load ptr, ptr %349, align 8, !alias.scope !31069, !noalias !31072
  br label %812

808:                                              ; preds = %805
  %809 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>
  invoke fastcc void @core::ptr::drop_glue::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(72) %48) #89
          to label %689 unwind label %810, !noalias !31069

810:                                              ; preds = %808
  %811 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !31074
  unreachable

812:                                              ; preds = %806, %801
  %813 = phi ptr [ %807, %806 ], [ %399, %801 ]
  %814 = getelementptr inbounds nuw [72 x i8], ptr %813, i64 %400
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %814, ptr noundef nonnull readonly align 8 dereferenceable(72) %48, i64 72, i1 false), !noalias !31069
  %815 = add nuw nsw i64 %400, 1
  store i64 %815, ptr %350, align 8, !alias.scope !31069, !noalias !31072
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  call void @llvm.lifetime.end.p0(ptr nonnull %57)
  %816 = icmp eq ptr %402, %354
  br i1 %816, label %.loopexit108, label %398

817:                                              ; preds = %743
  %818 = landingpad { ptr, i32 }
          cleanup
  br label %821

819:                                              ; preds = %761
  %820 = landingpad { ptr, i32 }
          cleanup
  br label %821

821:                                              ; preds = %819, %817
  %822 = phi { ptr, i32 } [ %818, %817 ], [ %820, %819 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %56) #89
          to label %689 unwind label %265

823:                                              ; preds = %828, %722
  %824 = landingpad { ptr, i32 }
          cleanup
  br label %689

825:                                              ; preds = %722
  %826 = load i64, ptr %49, align 8, !range !2059, !alias.scope !31075, !noundef !1740
  %827 = icmp eq i64 %826, -1
  br i1 %827, label %829, label %828

828:                                              ; preds = %825
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %49)
          to label %829 unwind label %823

829:                                              ; preds = %828, %825
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  call void @llvm.lifetime.end.p0(ptr nonnull %50)
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  call void @llvm.lifetime.end.p0(ptr nonnull %57)
  br label %.loopexit108

.loopexit108:                                     ; preds = %812, %829, %333
  %830 = phi ptr [ %399, %829 ], [ inttoptr (i64 8 to ptr), %333 ], [ %813, %812 ]
  %831 = phi i64 [ %400, %829 ], [ 0, %333 ], [ %815, %812 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %46)
  call void @llvm.lifetime.start.p0(ptr nonnull %45)
  %832 = getelementptr inbounds nuw i8, ptr %296, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema>::union
  invoke void @<purrdf_sparql_eval::solution::VarSchema>::union(ptr noalias nofree noundef nonnull sret([56 x i8]) align 8 captures(none) dereferenceable(56) %45, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %832, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %59)
          to label %924 unwind label %694

833:                                              ; preds = %706
  call void @llvm.lifetime.end.p0(ptr nonnull %58)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 dereferenceable(56) %59)
          to label %834 unwind label %319

834:                                              ; preds = %833
  call void @llvm.lifetime.end.p0(ptr nonnull %59)
  %835 = atomicrmw sub ptr %296, i64 1 release, align 8, !noalias !31078
  %836 = icmp eq i64 %835, 1
  br i1 %836, label %837, label %838

837:                                              ; preds = %834
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %60) #91
          to label %838 unwind label %307

838:                                              ; preds = %837, %834
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  %839 = load ptr, ptr %62, align 8, !alias.scope !31083, !noundef !1740
  %840 = icmp eq ptr %839, null
  br i1 %840, label %842, label %841

841:                                              ; preds = %838
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %62)
          to label %842 unwind label %102

842:                                              ; preds = %841, %838
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  br label %267

843:                                              ; preds = %1305, %273, %267
  %844 = phi i8 [ 0, %1305 ], [ %268, %273 ], [ %268, %267 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !31086)
  %845 = getelementptr inbounds nuw i8, ptr %79, i64 8
  %846 = load ptr, ptr %845, align 8, !alias.scope !31086, !nonnull !1740, !noundef !1740
  %847 = getelementptr inbounds nuw i8, ptr %79, i64 16
  %848 = load i64, ptr %847, align 8, !alias.scope !31086, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !31089)
  %849 = icmp eq i64 %848, 0
  br i1 %849, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %843
  %850 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %851 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %852

852:                                              ; preds = %.preheader, %890
  %853 = phi i64 [ %855, %890 ], [ 0, %.preheader ]
  %854 = getelementptr inbounds nuw [40 x i8], ptr %846, i64 %853
  %855 = add nuw nsw i64 %853, 1
  %856 = load i64, ptr %854, align 8, !range !1778, !alias.scope !31092, !noalias !31086, !noundef !1740
  %857 = icmp ugt i64 %856, 5
  br i1 %857, label %858, label %890

858:                                              ; preds = %852
  %859 = getelementptr i8, ptr %854, i64 8
  %860 = load ptr, ptr %859, align 8, !alias.scope !31089, !noalias !31086, !nonnull !1740, !noundef !1740
  %861 = shl i64 %856, 3
  %862 = add i64 %861, -8
  %863 = load i64, ptr %850, align 8, !noalias !31095, !noundef !1740
  %864 = call i64 @llvm.umin.i64(i64 %862, i64 9223372036854775807)
  %865 = call i64 @llvm.ssub.sat.i64(i64 %863, i64 %864)
  store i64 %865, ptr %850, align 8, !noalias !31095
  %866 = load i64, ptr %851, align 8, !noalias !31095, !noundef !1740
  %867 = icmp slt i64 %865, %866
  br i1 %867, label %868, label %.preheader885

868:                                              ; preds = %858
  store i64 %865, ptr %851, align 8, !noalias !31095
  br label %.preheader885

.preheader885:                                    ; preds = %868, %858
  br label %869

869:                                              ; preds = %.preheader885, %872
  %870 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !31095
  %871 = icmp slt i64 %870, 0
  br i1 %871, label %872, label %__rustc::__rust_dealloc (.exit78)

872:                                              ; preds = %869
  %873 = add nsw i64 %870, 1
  %874 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %870, i64 %873 acq_rel acquire, align 8, !noalias !31095
  %875 = extractvalue { i64, i1 } %874, 1
  br i1 %875, label %876, label %869

876:                                              ; preds = %872
  %877 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %864 monotonic, align 8, !noalias !31095
  %878 = call i64 @llvm.ssub.sat.i64(i64 %877, i64 %864)
  %879 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !31095
  br label %880

880:                                              ; preds = %883, %876
  %881 = phi i64 [ %879, %876 ], [ %886, %883 ]
  %882 = icmp slt i64 %878, %881
  br i1 %882, label %883, label %887

883:                                              ; preds = %880
  %884 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %881, i64 %878 monotonic monotonic, align 8, !noalias !31095
  %885 = extractvalue { i64, i1 } %884, 1
  %886 = extractvalue { i64, i1 } %884, 0
  br i1 %885, label %887, label %880

887:                                              ; preds = %883, %880
  %888 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !31095
  br label %__rustc::__rust_dealloc (.exit78)

__rustc::__rust_dealloc (.exit78): ; preds = %869, %887
  %889 = icmp ne i64 %862, 0
  call void @llvm.assume(i1 %889), !noalias !31095
  call void @free(ptr noundef nonnull %860) #92, !noalias !31095
  br label %890

890:                                              ; preds = %__rustc::__rust_dealloc (.exit78), %852
  %891 = icmp eq i64 %855, %848
  br i1 %891, label %.loopexit, label %852

.loopexit:                                        ; preds = %890, %843
  %892 = load i64, ptr %79, align 8, !alias.scope !31086
  %893 = icmp eq i64 %892, 0
  br i1 %893, label %1307, label %894

894:                                              ; preds = %.loopexit
  %895 = mul nuw i64 %892, 40
  %896 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %897 = load i64, ptr %896, align 8, !noalias !31086, !noundef !1740
  %898 = call i64 @llvm.umin.i64(i64 %895, i64 9223372036854775807)
  %899 = call i64 @llvm.ssub.sat.i64(i64 %897, i64 %898)
  store i64 %899, ptr %896, align 8, !noalias !31086
  %900 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %901 = load i64, ptr %900, align 8, !noalias !31086, !noundef !1740
  %902 = icmp slt i64 %899, %901
  br i1 %902, label %903, label %.preheader884

903:                                              ; preds = %894
  store i64 %899, ptr %900, align 8, !noalias !31086
  br label %.preheader884

.preheader884:                                    ; preds = %903, %894
  br label %904

904:                                              ; preds = %.preheader884, %907
  %905 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !31086
  %906 = icmp slt i64 %905, 0
  br i1 %906, label %907, label %__rustc::__rust_dealloc (.exit79)

907:                                              ; preds = %904
  %908 = add nsw i64 %905, 1
  %909 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %905, i64 %908 acq_rel acquire, align 8, !noalias !31086
  %910 = extractvalue { i64, i1 } %909, 1
  br i1 %910, label %911, label %904

911:                                              ; preds = %907
  %912 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %898 monotonic, align 8, !noalias !31086
  %913 = call i64 @llvm.ssub.sat.i64(i64 %912, i64 %898)
  %914 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !31086
  br label %915

915:                                              ; preds = %918, %911
  %916 = phi i64 [ %914, %911 ], [ %921, %918 ]
  %917 = icmp slt i64 %913, %916
  br i1 %917, label %918, label %922

918:                                              ; preds = %915
  %919 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %916, i64 %913 monotonic monotonic, align 8, !noalias !31086
  %920 = extractvalue { i64, i1 } %919, 1
  %921 = extractvalue { i64, i1 } %919, 0
  br i1 %920, label %922, label %915

922:                                              ; preds = %918, %915
  %923 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !31086
  br label %__rustc::__rust_dealloc (.exit79)

__rustc::__rust_dealloc (.exit79): ; preds = %904, %922
  call void @free(ptr noundef nonnull %846) #92, !noalias !31086
  br label %1307

924:                                              ; preds = %.loopexit108
  call void @llvm.lifetime.start.p0(ptr nonnull %36)
  store i64 1, ptr %36, align 8
  %925 = getelementptr inbounds nuw i8, ptr %36, i64 8
  store i64 1, ptr %925, align 8
  %926 = getelementptr inbounds nuw i8, ptr %36, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %926, ptr noundef nonnull align 8 dereferenceable(56) %45, i64 56, i1 false)
; invoke alloc::boxed::box_new_uninit
  %927 = invoke fastcc noundef ptr @alloc::boxed::box_new_uninit(i64 noundef 8, i64 noundef 72)
          to label %934 unwind label %928, !noalias !31098

928:                                              ; preds = %924
  %929 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %926)
          to label %689 unwind label %930

930:                                              ; preds = %928
  %931 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

932:                                              ; preds = %970
  %933 = landingpad { ptr, i32 }
          cleanup
  br label %1237

934:                                              ; preds = %924
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %927, ptr noundef nonnull align 8 dereferenceable(72) %36, i64 72, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %36)
  store ptr %927, ptr %46, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  %935 = getelementptr i8, ptr %296, i64 32
  %936 = load i64, ptr %935, align 8, !noundef !1740
  %937 = icmp ult i64 %936, 576460752303423488
  call void @llvm.assume(i1 %937)
  %938 = getelementptr inbounds nuw i8, ptr %927, i64 16
  %939 = getelementptr i8, ptr %927, i64 32
  %940 = load i64, ptr %939, align 8, !noundef !1740
  %941 = icmp ult i64 %940, 576460752303423488
  call void @llvm.assume(i1 %941)
  %942 = icmp eq i64 %940, 0
  br i1 %942, label %951, label %943

943:                                              ; preds = %934
  %944 = getelementptr inbounds nuw i8, ptr %4, i64 616
  %945 = load ptr, ptr %944, align 8, !noundef !1740
  %946 = icmp eq ptr %945, null
  br i1 %946, label %951, label %947

947:                                              ; preds = %943
  %948 = getelementptr inbounds nuw i8, ptr %945, i64 32
  %949 = load i64, ptr %948, align 8
  %950 = icmp eq i64 %949, -1
  br i1 %950, label %951, label %953

951:                                              ; preds = %947, %943, %934
  call void @llvm.lifetime.start.p0(ptr nonnull %44)
  %952 = icmp ult i64 %831, 128102389400760776
  call void @llvm.assume(i1 %952)
  br label %959

953:                                              ; preds = %947
  %954 = udiv i64 %949, %940
  %955 = icmp ult i64 %954, 230584300921369396
  call void @llvm.lifetime.start.p0(ptr nonnull %44)
  %956 = icmp ult i64 %831, 128102389400760776
  call void @llvm.assume(i1 %956)
  br i1 %955, label %957, label %959

957:                                              ; preds = %953
  %958 = call noundef range(i64 0, 128102389400760776) i64 @llvm.umin.i64(i64 %954, i64 %831)
  br label %959

959:                                              ; preds = %957, %953, %951
  %960 = phi i1 [ true, %957 ], [ false, %953 ], [ false, %951 ]
  %961 = phi i64 [ %954, %957 ], [ %954, %953 ], [ undef, %951 ]
  %962 = phi i64 [ %958, %957 ], [ %831, %953 ], [ %831, %951 ]
  %963 = mul nuw nsw i64 %962, 40
  %964 = icmp eq i64 %962, 0
  br i1 %964, label %<alloc::raw_vec::RawVecInner>::try_allocate_in (.exit), label %965

965:                                              ; preds = %959
; call __rustc::__rust_alloc
  %966 = call noundef align 8 ptr @__rustc::__rust_alloc(i64 noundef %963, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !31101
  %967 = icmp eq ptr %966, null
  br i1 %967, label %970, label %968

968:                                              ; preds = %965
  %969 = ptrtoint ptr %966 to i64
  br label %<alloc::raw_vec::RawVecInner>::try_allocate_in (.exit)

970:                                              ; preds = %965
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %963) #93
          to label %1236 unwind label %932

<alloc::raw_vec::RawVecInner>::try_allocate_in (.exit): ; preds = %968, %959
  %.sroa.9.0 = phi i64 [ %969, %968 ], [ 8, %959 ]
  %971 = inttoptr i64 %.sroa.9.0 to ptr
  store i64 %962, ptr %44, align 8
  %972 = getelementptr inbounds nuw i8, ptr %44, i64 8
  store ptr %971, ptr %972, align 8
  %973 = getelementptr inbounds nuw i8, ptr %44, i64 16
  store i64 0, ptr %973, align 8
  %974 = mul nuw nsw i64 %831, 72
  %975 = getelementptr inbounds nuw i8, ptr %830, i64 %974
  %976 = icmp eq i64 %831, 0
  br i1 %976, label %.loopexit106, label %977

977:                                              ; preds = %<alloc::raw_vec::RawVecInner>::try_allocate_in (.exit)
  %978 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %979 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %980 = icmp samesign ugt i64 %940, 4
  %981 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %982 = getelementptr inbounds nuw i8, ptr %35, i64 8
  %983 = getelementptr inbounds nuw i8, ptr %35, i64 16
  %984 = shl nuw nsw i64 %936, 3
  br label %985

985:                                              ; preds = %1218, %977
  %986 = phi ptr [ %971, %977 ], [ %1211, %1218 ]
  %987 = phi i64 [ 0, %977 ], [ %1212, %1218 ]
  %988 = phi ptr [ %830, %977 ], [ %989, %1218 ]
  %989 = getelementptr inbounds nuw i8, ptr %988, i64 72
  call void @llvm.lifetime.start.p0(ptr nonnull %43)
  %990 = getelementptr inbounds nuw i8, ptr %988, i64 64
  %991 = load ptr, ptr %990, align 8, !nonnull !1740, !noundef !1740
  %992 = getelementptr inbounds nuw i8, ptr %991, i64 16
; invoke purrdf_sparql_eval::binop::right_to_out_map
  invoke void @purrdf_sparql_eval::binop::right_to_out_map(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(address) dereferenceable(24) %43, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %992, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %938)
          to label %997 unwind label %993

993:                                              ; preds = %985
  %994 = landingpad { ptr, i32 }
          cleanup
  br label %1234

995:                                              ; preds = %.loopexit106
  %996 = landingpad { ptr, i32 }
          cleanup
  br label %689

997:                                              ; preds = %985
  %998 = getelementptr inbounds nuw i8, ptr %988, i64 48
  %999 = load ptr, ptr %998, align 8, !nonnull !1740, !noundef !1740
  %1000 = getelementptr inbounds nuw i8, ptr %988, i64 56
  %1001 = load i64, ptr %1000, align 8, !noundef !1740
  %1002 = mul nuw nsw i64 %1001, 40
  %1003 = getelementptr inbounds nuw i8, ptr %999, i64 %1002
  %1004 = icmp eq i64 %1001, 0
  br i1 %1004, label %.loopexit105, label %1005

1005:                                             ; preds = %997
  %1006 = getelementptr inbounds nuw i8, ptr %988, i64 8
  %1007 = getelementptr inbounds nuw i8, ptr %988, i64 16
  %1008 = load i64, ptr %978, align 8
  %1009 = load ptr, ptr %979, align 8, !nonnull !1740
  br label %1010

1010:                                             ; preds = %.loopexit103, %1005
  %1011 = phi ptr [ %986, %1005 ], [ %1201, %.loopexit103 ]
  %1012 = phi i64 [ %987, %1005 ], [ %1202, %.loopexit103 ]
  %1013 = phi ptr [ %999, %1005 ], [ %1014, %.loopexit103 ]
  %1014 = getelementptr inbounds nuw i8, ptr %1013, i64 40
  %1015 = load i64, ptr %1013, align 8, !range !1778, !noundef !1740
  %1016 = add i64 %1015, -1
  %1017 = icmp ugt i64 %1016, 4
  %1018 = getelementptr inbounds nuw i8, ptr %1013, i64 8
  br i1 %1017, label %1019, label %1024

1019:                                             ; preds = %1010
  %1020 = load ptr, ptr %1018, align 8, !nonnull !1740, !noundef !1740
  %1021 = getelementptr inbounds nuw i8, ptr %1013, i64 16
  %1022 = load i64, ptr %1021, align 8, !noundef !1740
  %1023 = add i64 %1022, -1
  br label %1024

1024:                                             ; preds = %1019, %1010
  %1025 = phi i64 [ %1023, %1019 ], [ %1016, %1010 ]
  %1026 = phi ptr [ %1020, %1019 ], [ %1018, %1010 ]
  %1027 = shl nuw nsw i64 %1025, 3
  %1028 = getelementptr inbounds nuw i8, ptr %1026, i64 %1027
  %1029 = icmp eq i64 %1025, 0
  br i1 %1029, label %.loopexit104, label %.preheader102

.preheader102:                                    ; preds = %1024, %1065
  %1030 = phi i64 [ %1066, %1065 ], [ 0, %1024 ]
  %1031 = phi ptr [ %1032, %1065 ], [ %1026, %1024 ]
  %1032 = getelementptr inbounds nuw i8, ptr %1031, i64 8
  %1033 = load i32, ptr %1031, align 4, !noalias !31104
  %1034 = getelementptr i8, ptr %1031, i64 4
  %1035 = load i32, ptr %1034, align 4, !noalias !31104
  %1036 = icmp eq i64 %1030, %1008
  br i1 %1036, label %1043, label %1037

1037:                                             ; preds = %.preheader102
  %1038 = getelementptr inbounds nuw [8 x i8], ptr %1009, i64 %1030
  %1039 = load i64, ptr %1038, align 8, !noalias !31108, !noundef !1740
  %1040 = load i64, ptr %988, align 8, !range !1778, !noalias !31108, !noundef !1740
  %1041 = add i64 %1040, -1
  %1042 = icmp ugt i64 %1041, 4
  br i1 %1042, label %1045, label %1049

1043:                                             ; preds = %.preheader102
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %1008, i64 noundef %1008, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.710) #88
          to label %1044 unwind label %1068

1044:                                             ; preds = %1043
  unreachable

1045:                                             ; preds = %1037
  %1046 = load ptr, ptr %1006, align 8, !noalias !31108, !nonnull !1740, !noundef !1740
  %1047 = load i64, ptr %1007, align 8, !noalias !31108, !noundef !1740
  %1048 = add i64 %1047, -1
  br label %1049

1049:                                             ; preds = %1045, %1037
  %1050 = phi ptr [ %1046, %1045 ], [ %1006, %1037 ]
  %1051 = phi i64 [ %1048, %1045 ], [ %1041, %1037 ]
  %1052 = icmp ult i64 %1039, %1051
  br i1 %1052, label %1053, label %1065

1053:                                             ; preds = %1049
  %1054 = getelementptr inbounds nuw [8 x i8], ptr %1050, i64 %1039
  %1055 = load i32, ptr %1054, align 4, !range !1785, !noalias !31108, !noundef !1740
  %1056 = icmp eq i32 %1033, 2
  %1057 = icmp eq i32 %1055, 2
  %1058 = or i1 %1056, %1057
  br i1 %1058, label %1065, label %1059

1059:                                             ; preds = %1053
  %1060 = getelementptr inbounds nuw i8, ptr %1054, i64 4
  %1061 = load i32, ptr %1060, align 4, !noalias !31108
  %1062 = icmp ne i32 %1033, %1055
  %1063 = icmp ne i32 %1035, %1061
  %1064 = select i1 %1062, i1 true, i1 %1063
  br i1 %1064, label %.loopexit103, label %1065

1065:                                             ; preds = %1059, %1053, %1049
  %1066 = add nuw nsw i64 %1030, 1
  %1067 = icmp eq ptr %1032, %1028
  br i1 %1067, label %.loopexit104, label %.preheader102

1068:                                             ; preds = %1177, %1043
  %1069 = landingpad { ptr, i32 }
          cleanup
  br label %1070

1070:                                             ; preds = %1192, %1189, %1123, %1119, %1093, %1084, %1080, %1068
  %1071 = phi { ptr, i32 } [ %1087, %1084 ], [ %1120, %1119 ], [ %1069, %1068 ], [ %1081, %1080 ], [ %1094, %1093 ], [ %1120, %1123 ], [ %1190, %1192 ], [ %1190, %1189 ]
  %1072 = load i64, ptr %43, align 8
  %1073 = icmp eq i64 %1072, 0
  br i1 %1073, label %1234, label %1074

1074:                                             ; preds = %1070
  %1075 = load ptr, ptr %979, align 8, !nonnull !1740, !noundef !1740
  %1076 = shl nuw i64 %1072, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1075, i64 noundef %1076, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %1234

.loopexit104:                                     ; preds = %1065, %1024
  br i1 %960, label %1174, label %1077

1077:                                             ; preds = %1174, %.loopexit104
  call void @llvm.lifetime.start.p0(ptr nonnull %41)
  call void @llvm.experimental.noalias.scope.decl(metadata !31113)
  call void @llvm.experimental.noalias.scope.decl(metadata !31116)
  call void @llvm.experimental.noalias.scope.decl(metadata !31118)
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !31120
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !31122
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !31125
  store i64 1, ptr %8, align 8, !noalias !31125
  br i1 %980, label %1078, label %1091, !prof !1742

1078:                                             ; preds = %1077
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %8, i64 noundef 0, i64 noundef range(i64 0, 576460752303423488) %940, i1 noundef zeroext false) #91
          to label %1079 unwind label %1080, !noalias !31125

1079:                                             ; preds = %1078
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %10, ptr noundef nonnull align 8 dereferenceable(40) %8, i64 40, i1 false), !noalias !31122
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !31125
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !31122
  br label %1092

1080:                                             ; preds = %1078
  %1081 = landingpad { ptr, i32 }
          cleanup
  %1082 = load i64, ptr %8, align 8, !range !1778, !alias.scope !31128, !noalias !31125, !noundef !1740
  %1083 = icmp ugt i64 %1082, 5
  br i1 %1083, label %1084, label %1070

1084:                                             ; preds = %1093, %1080
  %1085 = phi ptr [ %81, %1093 ], [ %82, %1080 ]
  %1086 = phi i64 [ %1095, %1093 ], [ %1082, %1080 ]
  %1087 = phi { ptr, i32 } [ %1094, %1093 ], [ %1081, %1080 ]
  %1088 = load ptr, ptr %1085, align 8, !noalias !31122, !nonnull !1740, !noundef !1740
  %1089 = shl i64 %1086, 3
  %1090 = add i64 %1089, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1088, i64 noundef %1090, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !31122
  br label %1070

1091:                                             ; preds = %1077
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %10, ptr noundef nonnull align 8 dereferenceable(40) %8, i64 40, i1 false), !noalias !31122
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !31125
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !31122
  br i1 %942, label %1097, label %1092

1092:                                             ; preds = %1091, %1079
  br label %1097

1093:                                             ; preds = %1097
  %1094 = landingpad { ptr, i32 }
          cleanup
  %1095 = load i64, ptr %10, align 8, !range !1778, !alias.scope !31131, !noalias !31122, !noundef !1740
  %1096 = icmp ugt i64 %1095, 5
  br i1 %1096, label %1084, label %1070

1097:                                             ; preds = %1092, %1091
  %1098 = phi i32 [ 2, %1092 ], [ -1, %1091 ]
  store i32 %1098, ptr %9, align 8, !noalias !31122
  store i64 %940, ptr %981, align 8, !noalias !31122
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>(ptr noalias nofree noundef align 8 dereferenceable(40) %10, ptr noalias nofree noundef align 8 captures(address) dereferenceable(16) %9)
          to label %1099 unwind label %1093, !noalias !31122

1099:                                             ; preds = %1097
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !31122
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %35, ptr noundef nonnull align 8 dereferenceable(40) %10, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !31122
  %1100 = load i64, ptr %35, align 8, !range !1778, !noalias !31120, !noundef !1740
  %1101 = icmp ugt i64 %1100, 5
  %1102 = load ptr, ptr %982, align 8, !noalias !31120, !nonnull !1740
  %1103 = select i1 %1101, ptr %1102, ptr %982
  %1104 = load i64, ptr %983, align 8
  %1105 = select i1 %1101, i64 %1104, i64 %1100
  %1106 = add i64 %1105, -1
  %1107 = icmp ugt i64 %936, %1106
  br i1 %1107, label %1108, label %1109, !prof !10952

1108:                                             ; preds = %1099
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef 0, i64 noundef range(i64 0, 576460752303423488) %936, i64 noundef %1106, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.451) #93
          to label %1163 unwind label %1119, !noalias !31120

1109:                                             ; preds = %1099
  %1110 = load i64, ptr %988, align 8, !range !1778, !alias.scope !31113, !noalias !31134, !noundef !1740
  %1111 = add i64 %1110, -1
  %1112 = icmp ugt i64 %1111, 4
  %1113 = load i64, ptr %1007, align 8, !alias.scope !31113, !noalias !31134
  %1114 = add i64 %1113, -1
  %1115 = select i1 %1112, i64 %1114, i64 %1111
  %1116 = icmp eq i64 %936, %1115
  br i1 %1116, label %1127, label %1117, !prof !1953

1117:                                             ; preds = %1109
; invoke core::slice::copy_from_slice_impl::len_mismatch_fail
  invoke void @core::slice::copy_from_slice_impl::len_mismatch_fail(i64 noundef range(i64 0, 576460752303423488) %936, i64 noundef range(i64 0, 1152921504606846976) %1115, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.448) #88
          to label %1118 unwind label %1119

1118:                                             ; preds = %1117
  unreachable

1119:                                             ; preds = %1169, %1117, %1108
  %1120 = landingpad { ptr, i32 }
          cleanup
  %1121 = load i64, ptr %35, align 8, !range !1778, !alias.scope !19667, !noundef !1740
  %1122 = icmp ugt i64 %1121, 5
  br i1 %1122, label %1123, label %1070

1123:                                             ; preds = %1119
  %1124 = load ptr, ptr %982, align 8, !nonnull !1740, !noundef !1740
  %1125 = shl i64 %1121, 3
  %1126 = add i64 %1125, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1124, i64 noundef %1126, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !31135
  br label %1070

1127:                                             ; preds = %1109
  %1128 = load ptr, ptr %1006, align 8, !alias.scope !31113, !noalias !31134, !nonnull !1740
  %1129 = select i1 %1112, ptr %1128, ptr %1006
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 4 %1103, ptr nonnull readonly align 4 %1129, i64 %984, i1 false), !alias.scope !31138, !noalias !31142
  %1130 = load i64, ptr %1013, align 8, !range !1778, !alias.scope !31116, !noalias !31144, !noundef !1740
  %1131 = add i64 %1130, -1
  %1132 = icmp ugt i64 %1131, 4
  %1133 = load ptr, ptr %1018, align 8, !alias.scope !31116, !noalias !31144, !nonnull !1740
  %1134 = getelementptr inbounds nuw i8, ptr %1013, i64 16
  %1135 = load i64, ptr %1134, align 8, !alias.scope !31116, !noalias !31144
  %1136 = add i64 %1135, -1
  %1137 = select i1 %1132, i64 %1136, i64 %1131
  %1138 = select i1 %1132, ptr %1133, ptr %1018
  %1139 = shl nuw nsw i64 %1137, 3
  %1140 = getelementptr inbounds nuw i8, ptr %1138, i64 %1139
  %1141 = icmp eq i64 %1137, 0
  br i1 %1141, label %1181, label %.preheader101

.preheader101:                                    ; preds = %1127, %1152
  %1142 = phi i64 [ %1145, %1152 ], [ 0, %1127 ]
  %1143 = phi ptr [ %1144, %1152 ], [ %1138, %1127 ]
  %1144 = getelementptr inbounds nuw i8, ptr %1143, i64 8
  %1145 = add nuw nsw i64 %1142, 1
  %1146 = load i32, ptr %1143, align 4, !range !1785, !noalias !31145, !noundef !1740
  %1147 = icmp eq i32 %1146, 2
  br i1 %1147, label %1152, label %1148

1148:                                             ; preds = %.preheader101
  %1149 = getelementptr inbounds nuw i8, ptr %1143, i64 4
  %1150 = load i32, ptr %1149, align 4, !noalias !31145, !noundef !1740
  %1151 = icmp ult i64 %1142, %1008
  br i1 %1151, label %1154, label %1169

1152:                                             ; preds = %1164, %.preheader101
  %1153 = icmp eq ptr %1144, %1140
  br i1 %1153, label %1179, label %.preheader101

1154:                                             ; preds = %1148
  %1155 = getelementptr inbounds nuw [8 x i8], ptr %1009, i64 %1142
  %1156 = load i64, ptr %1155, align 8, !alias.scope !31118, !noalias !31146, !noundef !1740
  %1157 = load i64, ptr %35, align 8, !range !1778, !noalias !31120, !noundef !1740
  %1158 = icmp ugt i64 %1157, 5
  %1159 = load i64, ptr %983, align 8
  %1160 = select i1 %1158, i64 %1159, i64 %1157
  %1161 = add i64 %1160, -1
  %1162 = icmp ult i64 %1156, %1161
  br i1 %1162, label %1164, label %1169

1163:                                             ; preds = %1108
  unreachable

1164:                                             ; preds = %1154
  %1165 = load ptr, ptr %982, align 8, !noalias !31120, !nonnull !1740
  %1166 = select i1 %1158, ptr %1165, ptr %982
  %1167 = getelementptr inbounds nuw [8 x i8], ptr %1166, i64 %1156
  store i32 %1146, ptr %1167, align 4, !noalias !31145
  %1168 = getelementptr inbounds nuw i8, ptr %1167, i64 4
  store i32 %1150, ptr %1168, align 4, !noalias !31145
  br label %1152

1169:                                             ; preds = %1154, %1148
  %1170 = phi i64 [ %1142, %1148 ], [ %1156, %1154 ]
  %1171 = phi i64 [ %1008, %1148 ], [ %1161, %1154 ]
  %1172 = phi ptr [ @anon.e5162873a9a3251d11c4df37a70e4654.449, %1148 ], [ @anon.e5162873a9a3251d11c4df37a70e4654.450, %1154 ]
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %1170, i64 noundef %1171, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) %1172) #93
          to label %1173 unwind label %1119, !noalias !31145

1173:                                             ; preds = %1169
  unreachable

1174:                                             ; preds = %.loopexit104
  %1175 = icmp ult i64 %1012, 230584300921369396
  call void @llvm.assume(i1 %1175)
  %1176 = icmp ult i64 %1012, %961
  br i1 %1176, label %1077, label %1177

1177:                                             ; preds = %1174
  call void @llvm.lifetime.start.p0(ptr nonnull %42)
  %1178 = add nuw nsw i64 %1012, 1
; invoke <purrdf_sparql_eval::eval::EvalCtx>::observe_cells
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::observe_cells(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %42, ptr noundef nonnull align 16 %4, i64 noundef %1178, i64 noundef %940)
          to label %1204 unwind label %1068

1179:                                             ; preds = %1152
  %1180 = load i64, ptr %35, align 8, !noalias !31147
  br label %1181

1181:                                             ; preds = %1179, %1127
  %1182 = phi i64 [ %1180, %1179 ], [ %1100, %1127 ]
  %1183 = load ptr, ptr %982, align 8, !noalias !31147
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %41, ptr noundef nonnull align 8 dereferenceable(24) %983, i64 24, i1 false), !noalias !31147
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !31120
  call void @llvm.experimental.noalias.scope.decl(metadata !31148)
  %1184 = load i64, ptr %44, align 8, !range !1835, !alias.scope !31148, !noalias !31151, !noundef !1740
  %1185 = icmp eq i64 %1012, %1184
  br i1 %1185, label %1186, label %1195

1186:                                             ; preds = %1181
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %44)
          to label %1187 unwind label %1189, !noalias !31151

1187:                                             ; preds = %1186
  %1188 = load ptr, ptr %972, align 8, !alias.scope !31148, !noalias !31151
  br label %1195

1189:                                             ; preds = %1186
  %1190 = landingpad { ptr, i32 }
          cleanup
  %1191 = icmp ugt i64 %1182, 5
  br i1 %1191, label %1192, label %1070

1192:                                             ; preds = %1189
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1183) ]
  %1193 = shl i64 %1182, 3
  %1194 = add i64 %1193, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1183, i64 noundef %1194, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !31153
  br label %1070

1195:                                             ; preds = %1187, %1181
  %1196 = phi ptr [ %1188, %1187 ], [ %1011, %1181 ]
  %1197 = getelementptr inbounds nuw [40 x i8], ptr %1196, i64 %1012
  store i64 %1182, ptr %1197, align 8, !noalias !31148
  %1198 = getelementptr inbounds nuw i8, ptr %1197, i64 8
  store ptr %1183, ptr %1198, align 8, !noalias !31148
  %1199 = getelementptr inbounds nuw i8, ptr %1197, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1199, ptr noundef nonnull align 8 dereferenceable(24) %41, i64 24, i1 false), !noalias !31148
  %1200 = add i64 %1012, 1
  store i64 %1200, ptr %973, align 8, !alias.scope !31148, !noalias !31151
  call void @llvm.lifetime.end.p0(ptr nonnull %41)
  br label %.loopexit103

.loopexit103:                                     ; preds = %1059, %1195
  %1201 = phi ptr [ %1196, %1195 ], [ %1011, %1059 ]
  %1202 = phi i64 [ %1200, %1195 ], [ %1012, %1059 ]
  %1203 = icmp eq ptr %1014, %1003
  br i1 %1203, label %.loopexit105, label %1010

1204:                                             ; preds = %1177
  call void @llvm.lifetime.end.p0(ptr nonnull %42)
  %1205 = load i64, ptr %43, align 8
  %1206 = icmp eq i64 %1205, 0
  br i1 %1206, label %1209, label %1207

1207:                                             ; preds = %1204
  %1208 = shl nuw i64 %1205, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1009, i64 noundef %1208, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %1209

1209:                                             ; preds = %1207, %1204
  call void @llvm.lifetime.end.p0(ptr nonnull %43)
  br label %.loopexit106

.loopexit106:                                     ; preds = %1218, %1209, %<alloc::raw_vec::RawVecInner>::try_allocate_in (.exit)
  call void @llvm.lifetime.start.p0(ptr nonnull %40)
  call void @llvm.lifetime.start.p0(ptr nonnull %38)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %38, ptr noundef nonnull align 8 dereferenceable(24) %44, i64 24, i1 false)
  %1210 = getelementptr inbounds nuw i8, ptr %38, i64 24
  store ptr %927, ptr %1210, align 8
; invoke <purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::finish::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(96) %40, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %39, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %38)
          to label %1220 unwind label %995

.loopexit105:                                     ; preds = %.loopexit103, %997
  %1211 = phi ptr [ %986, %997 ], [ %1201, %.loopexit103 ]
  %1212 = phi i64 [ %987, %997 ], [ %1202, %.loopexit103 ]
  %1213 = load i64, ptr %43, align 8
  %1214 = icmp eq i64 %1213, 0
  br i1 %1214, label %1218, label %1215

1215:                                             ; preds = %.loopexit105
  %1216 = load ptr, ptr %979, align 8, !nonnull !1740, !noundef !1740
  %1217 = shl nuw i64 %1213, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1216, i64 noundef %1217, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %1218

1218:                                             ; preds = %1215, %.loopexit105
  call void @llvm.lifetime.end.p0(ptr nonnull %43)
  %1219 = icmp eq ptr %989, %975
  br i1 %1219, label %.loopexit106, label %985

1220:                                             ; preds = %.loopexit106
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
  %1221 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1221, ptr noundef nonnull align 8 dereferenceable(96) %40, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
  call void @llvm.lifetime.end.p0(ptr nonnull %44)
  call void @llvm.lifetime.end.p0(ptr nonnull %46)
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::solution::SolutionSeq)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %58)
          to label %1222 unwind label %342

1222:                                             ; preds = %1220
  call void @llvm.lifetime.end.p0(ptr nonnull %58)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 dereferenceable(56) %59)
          to label %1223 unwind label %319

1223:                                             ; preds = %1222
  call void @llvm.lifetime.end.p0(ptr nonnull %59)
  %1224 = atomicrmw sub ptr %296, i64 1 release, align 8, !noalias !31156
  %1225 = icmp eq i64 %1224, 1
  br i1 %1225, label %1226, label %1227

1226:                                             ; preds = %1223
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %60) #91
          to label %1227 unwind label %307

1227:                                             ; preds = %1226, %1223
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  %1228 = load ptr, ptr %62, align 8, !alias.scope !31161, !noundef !1740
  %1229 = icmp eq ptr %1228, null
  br i1 %1229, label %1231, label %1230

1230:                                             ; preds = %1227
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::deferred_exists::DeferredLateral>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %62)
          to label %1231 unwind label %102

1231:                                             ; preds = %1230, %1227
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %79)
          to label %1232 unwind label %87

1232:                                             ; preds = %1231
  call void @llvm.lifetime.end.p0(ptr nonnull %79)
  br label %1233

1233:                                             ; preds = %1401, %1389, %1386, %1382, %1307, %1232
  ret void

1234:                                             ; preds = %1074, %1070, %993
  %1235 = phi { ptr, i32 } [ %994, %993 ], [ %1071, %1074 ], [ %1071, %1070 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %44) #89
  br label %1237

1236:                                             ; preds = %970, %345
  unreachable

1237:                                             ; preds = %1234, %932
  %1238 = phi { ptr, i32 } [ %1235, %1234 ], [ %933, %932 ]
  %1239 = atomicrmw sub ptr %927, i64 1 release, align 8, !noalias !31164
  %1240 = icmp eq i64 %1239, 1
  br i1 %1240, label %1241, label %689

1241:                                             ; preds = %1237
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %46) #91
          to label %689 unwind label %265

1242:                                             ; preds = %311
  %1243 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %1243, ptr noundef nonnull align 16 dereferenceable(96) %61, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %61)
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  br label %267

1244:                                             ; preds = %273
  %1245 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %79) #89
  br label %83

1246:                                             ; preds = %1304
  %1247 = landingpad { ptr, i32 }
          cleanup
  br label %1390

1248:                                             ; preds = %105
  call void @llvm.lifetime.start.p0(ptr nonnull %76)
  call void @llvm.lifetime.start.p0(ptr nonnull %75)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %75, ptr noundef nonnull align 8 dereferenceable(104) %39, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %74)
  %1249 = getelementptr inbounds nuw i8, ptr %79, i64 24
  %1250 = load ptr, ptr %1249, align 8, !nonnull !1740, !noundef !1740
  %1251 = getelementptr inbounds nuw i8, ptr %74, i64 24
  store ptr %1250, ptr %1251, align 8, !alias.scope !31169
  store i64 0, ptr %74, align 8, !alias.scope !31169
  %1252 = getelementptr inbounds nuw i8, ptr %74, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1252, align 8, !alias.scope !31169
  %1253 = getelementptr inbounds nuw i8, ptr %74, i64 16
  store i64 0, ptr %1253, align 8, !alias.scope !31169
  tail call void @llvm.experimental.noalias.scope.decl(metadata !31172)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !31175)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !31177)
  %1254 = load i64, ptr %75, align 8, !range !2059, !alias.scope !31175, !noalias !31179, !noundef !1740
  %1255 = icmp eq i64 %1254, -1
  br i1 %1255, label %1257, label %1256

1256:                                             ; preds = %1248
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %76, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %74, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %39)
  br label %1259

1257:                                             ; preds = %1248
  %1258 = getelementptr inbounds nuw i8, ptr %76, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %1258, ptr noundef nonnull readonly align 8 dereferenceable(32) %74, i64 32, i1 false), !alias.scope !31179, !noalias !31175
  store i64 -1, ptr %76, align 8, !alias.scope !31172, !noalias !31180
  br label %1259

1259:                                             ; preds = %1257, %1256
  %1260 = getelementptr inbounds nuw i8, ptr %75, i64 72
  %1261 = load i64, ptr %1260, align 8, !range !1778, !alias.scope !31181, !noalias !31179, !noundef !1740
  %1262 = icmp ugt i64 %1261, 5
  br i1 %1262, label %1263, label %1297

1263:                                             ; preds = %1259
  %1264 = getelementptr inbounds nuw i8, ptr %75, i64 80
  %1265 = load ptr, ptr %1264, align 8, !alias.scope !31175, !noalias !31179, !nonnull !1740, !noundef !1740
  %1266 = mul i64 %1261, 3
  %1267 = add i64 %1266, -3
  %1268 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1269 = load i64, ptr %1268, align 8, !noalias !31184, !noundef !1740
  %1270 = tail call i64 @llvm.umin.i64(i64 %1267, i64 9223372036854775807)
  %1271 = tail call i64 @llvm.ssub.sat.i64(i64 %1269, i64 %1270)
  store i64 %1271, ptr %1268, align 8, !noalias !31184
  %1272 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1273 = load i64, ptr %1272, align 8, !noalias !31184, !noundef !1740
  %1274 = icmp slt i64 %1271, %1273
  br i1 %1274, label %1275, label %.preheader1055

1275:                                             ; preds = %1263
  store i64 %1271, ptr %1272, align 8, !noalias !31184
  br label %.preheader1055

.preheader1055:                                   ; preds = %1275, %1263
  br label %1276

1276:                                             ; preds = %.preheader1055, %1279
  %1277 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !31184
  %1278 = icmp slt i64 %1277, 0
  br i1 %1278, label %1279, label %__rustc::__rust_dealloc (.exit80)

1279:                                             ; preds = %1276
  %1280 = add nsw i64 %1277, 1
  %1281 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1277, i64 %1280 acq_rel acquire, align 8, !noalias !31184
  %1282 = extractvalue { i64, i1 } %1281, 1
  br i1 %1282, label %1283, label %1276

1283:                                             ; preds = %1279
  %1284 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1270 monotonic, align 8, !noalias !31184
  %1285 = tail call i64 @llvm.ssub.sat.i64(i64 %1284, i64 %1270)
  %1286 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !31184
  br label %1287

1287:                                             ; preds = %1290, %1283
  %1288 = phi i64 [ %1286, %1283 ], [ %1293, %1290 ]
  %1289 = icmp slt i64 %1285, %1288
  br i1 %1289, label %1290, label %1294

1290:                                             ; preds = %1287
  %1291 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1288, i64 %1285 monotonic monotonic, align 8, !noalias !31184
  %1292 = extractvalue { i64, i1 } %1291, 1
  %1293 = extractvalue { i64, i1 } %1291, 0
  br i1 %1292, label %1294, label %1287

1294:                                             ; preds = %1290, %1287
  %1295 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !31184
  br label %__rustc::__rust_dealloc (.exit80)

__rustc::__rust_dealloc (.exit80): ; preds = %1276, %1294
  %1296 = icmp ne i64 %1267, 0
  tail call void @llvm.assume(i1 %1296), !noalias !31184
  tail call void @free(ptr noundef nonnull %1265) #92, !noalias !31184
  br label %1297

1297:                                             ; preds = %__rustc::__rust_dealloc (.exit80), %1259
  %1298 = getelementptr inbounds nuw i8, ptr %75, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !31187)
  %1299 = load ptr, ptr %1298, align 8, !alias.scope !31190, !noalias !31179, !noundef !1740
  %1300 = icmp eq ptr %1299, null
  br i1 %1300, label %1305, label %1301

1301:                                             ; preds = %1297
  %1302 = atomicrmw sub ptr %1299, i64 1 release, align 8, !noalias !31191
  %1303 = icmp eq i64 %1302, 1
  br i1 %1303, label %1304, label %1305

1304:                                             ; preds = %1301
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1298) #91
          to label %1305 unwind label %1246

1305:                                             ; preds = %1304, %1301, %1297
  call void @llvm.lifetime.end.p0(ptr nonnull %74)
  call void @llvm.lifetime.end.p0(ptr nonnull %75)
  %1306 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1306, ptr noundef nonnull align 8 dereferenceable(96) %76, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %76)
  br label %843

1307:                                             ; preds = %__rustc::__rust_dealloc (.exit79), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %79)
  %1308 = trunc nuw i8 %844 to i1
  br i1 %1308, label %1309, label %1233

1309:                                             ; preds = %1307, %93
  %1310 = getelementptr inbounds nuw i8, ptr %39, i64 72
  %1311 = load i64, ptr %1310, align 8, !range !1778, !noundef !1740
  %1312 = icmp ugt i64 %1311, 5
  br i1 %1312, label %1313, label %1347

1313:                                             ; preds = %1309
  %1314 = getelementptr inbounds nuw i8, ptr %39, i64 80
  %1315 = load ptr, ptr %1314, align 8, !nonnull !1740, !noundef !1740
  %1316 = mul i64 %1311, 3
  %1317 = add i64 %1316, -3
  %1318 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1319 = load i64, ptr %1318, align 8, !noalias !31196, !noundef !1740
  %1320 = call i64 @llvm.umin.i64(i64 %1317, i64 9223372036854775807)
  %1321 = call i64 @llvm.ssub.sat.i64(i64 %1319, i64 %1320)
  store i64 %1321, ptr %1318, align 8, !noalias !31196
  %1322 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1323 = load i64, ptr %1322, align 8, !noalias !31196, !noundef !1740
  %1324 = icmp slt i64 %1321, %1323
  br i1 %1324, label %1325, label %.preheader883

1325:                                             ; preds = %1313
  store i64 %1321, ptr %1322, align 8, !noalias !31196
  br label %.preheader883

.preheader883:                                    ; preds = %1325, %1313
  br label %1326

1326:                                             ; preds = %.preheader883, %1329
  %1327 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !31196
  %1328 = icmp slt i64 %1327, 0
  br i1 %1328, label %1329, label %__rustc::__rust_dealloc (.exit81)

1329:                                             ; preds = %1326
  %1330 = add nsw i64 %1327, 1
  %1331 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1327, i64 %1330 acq_rel acquire, align 8, !noalias !31196
  %1332 = extractvalue { i64, i1 } %1331, 1
  br i1 %1332, label %1333, label %1326

1333:                                             ; preds = %1329
  %1334 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1320 monotonic, align 8, !noalias !31196
  %1335 = call i64 @llvm.ssub.sat.i64(i64 %1334, i64 %1320)
  %1336 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !31196
  br label %1337

1337:                                             ; preds = %1340, %1333
  %1338 = phi i64 [ %1336, %1333 ], [ %1343, %1340 ]
  %1339 = icmp slt i64 %1335, %1338
  br i1 %1339, label %1340, label %1344

1340:                                             ; preds = %1337
  %1341 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1338, i64 %1335 monotonic monotonic, align 8, !noalias !31196
  %1342 = extractvalue { i64, i1 } %1341, 1
  %1343 = extractvalue { i64, i1 } %1341, 0
  br i1 %1342, label %1344, label %1337

1344:                                             ; preds = %1340, %1337
  %1345 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !31196
  br label %__rustc::__rust_dealloc (.exit81)

__rustc::__rust_dealloc (.exit81): ; preds = %1326, %1344
  %1346 = icmp ne i64 %1317, 0
  call void @llvm.assume(i1 %1346), !noalias !31196
  call void @free(ptr noundef nonnull %1315) #92, !noalias !31196
  br label %1347

1347:                                             ; preds = %__rustc::__rust_dealloc (.exit81), %1309
  %1348 = load i64, ptr %39, align 8, !range !2059, !noundef !1740
  %1349 = icmp sgt i64 %1348, 0
  br i1 %1349, label %1350, label %1382

1350:                                             ; preds = %1347
  %1351 = getelementptr inbounds nuw i8, ptr %39, i64 8
  %1352 = load ptr, ptr %1351, align 8, !nonnull !1740, !noundef !1740
  %1353 = mul nuw i64 %1348, 3
  %1354 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1355 = load i64, ptr %1354, align 8, !noalias !31201, !noundef !1740
  %1356 = call i64 @llvm.umin.i64(i64 %1353, i64 9223372036854775807)
  %1357 = call i64 @llvm.ssub.sat.i64(i64 %1355, i64 %1356)
  store i64 %1357, ptr %1354, align 8, !noalias !31201
  %1358 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1359 = load i64, ptr %1358, align 8, !noalias !31201, !noundef !1740
  %1360 = icmp slt i64 %1357, %1359
  br i1 %1360, label %1361, label %.preheader882

1361:                                             ; preds = %1350
  store i64 %1357, ptr %1358, align 8, !noalias !31201
  br label %.preheader882

.preheader882:                                    ; preds = %1361, %1350
  br label %1362

1362:                                             ; preds = %.preheader882, %1365
  %1363 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !31201
  %1364 = icmp slt i64 %1363, 0
  br i1 %1364, label %1365, label %__rustc::__rust_dealloc (.exit82)

1365:                                             ; preds = %1362
  %1366 = add nsw i64 %1363, 1
  %1367 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1363, i64 %1366 acq_rel acquire, align 8, !noalias !31201
  %1368 = extractvalue { i64, i1 } %1367, 1
  br i1 %1368, label %1369, label %1362

1369:                                             ; preds = %1365
  %1370 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1356 monotonic, align 8, !noalias !31201
  %1371 = call i64 @llvm.ssub.sat.i64(i64 %1370, i64 %1356)
  %1372 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !31201
  br label %1373

1373:                                             ; preds = %1376, %1369
  %1374 = phi i64 [ %1372, %1369 ], [ %1379, %1376 ]
  %1375 = icmp slt i64 %1371, %1374
  br i1 %1375, label %1376, label %1380

1376:                                             ; preds = %1373
  %1377 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1374, i64 %1371 monotonic monotonic, align 8, !noalias !31201
  %1378 = extractvalue { i64, i1 } %1377, 1
  %1379 = extractvalue { i64, i1 } %1377, 0
  br i1 %1378, label %1380, label %1373

1380:                                             ; preds = %1376, %1373
  %1381 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !31201
  br label %__rustc::__rust_dealloc (.exit82)

__rustc::__rust_dealloc (.exit82): ; preds = %1362, %1380
  call void @free(ptr noundef nonnull %1352) #92, !noalias !31201
  br label %1382

1382:                                             ; preds = %__rustc::__rust_dealloc (.exit82), %1347
  %1383 = getelementptr inbounds nuw i8, ptr %39, i64 96
  %1384 = load ptr, ptr %1383, align 8, !noundef !1740
  %1385 = icmp eq ptr %1384, null
  br i1 %1385, label %1233, label %1386

1386:                                             ; preds = %1382
  %1387 = atomicrmw sub ptr %1384, i64 1 release, align 8, !noalias !31202
  %1388 = icmp eq i64 %1387, 1
  br i1 %1388, label %1389, label %1233

1389:                                             ; preds = %1386
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1383) #91
  br label %1233

1390:                                             ; preds = %1400, %1393, %1246
  %1391 = phi { ptr, i32 } [ %1247, %1246 ], [ %1395, %1400 ], [ %1395, %1393 ]
  %1392 = phi i8 [ 0, %1246 ], [ %1394, %1400 ], [ %1394, %1393 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %79) #89
  br label %83

1393:                                             ; preds = %306, %301, %258, %190, %102
  %1394 = phi i8 [ %302, %301 ], [ %103, %102 ], [ %302, %306 ], [ 1, %190 ], [ 0, %258 ]
  %1395 = phi { ptr, i32 } [ %303, %301 ], [ %104, %102 ], [ %303, %306 ], [ %191, %190 ], [ %259, %258 ]
  %1396 = getelementptr inbounds nuw i8, ptr %79, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !31209)
  call void @llvm.experimental.noalias.scope.decl(metadata !31212)
  %1397 = load ptr, ptr %1396, align 8, !alias.scope !31215, !nonnull !1740, !noundef !1740
  %1398 = atomicrmw sub ptr %1397, i64 1 release, align 8, !noalias !31215
  %1399 = icmp eq i64 %1398, 1
  br i1 %1399, label %1400, label %1390

1400:                                             ; preds = %1393
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1396) #91
          to label %1390 unwind label %265

1401:                                             ; preds = %101
  %1402 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1402, ptr noundef nonnull align 8 dereferenceable(96) %80, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %80)
  br label %1233

1403:                                             ; preds = %1404, %83
  resume { ptr, i32 } %85

1404:                                             ; preds = %83
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %39) #89
          to label %1403 unwind label %265
}

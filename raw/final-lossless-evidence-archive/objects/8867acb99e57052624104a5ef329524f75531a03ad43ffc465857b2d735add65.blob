define { i64, i64 } @purrdf_sparql_eval::modifier::aggregate_numeric_cost(ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %1, i64 noundef range(i64 0, 115292150460684698) %2, i64 %3) unnamed_addr #2 personality ptr @rust_eh_personality !guid !100451 {
  %5 = alloca [40 x i8], align 8
  %6 = alloca [40 x i8], align 8
  %7 = alloca [24 x i8], align 8
  %8 = alloca [40 x i8], align 8
  %9 = alloca [32 x i8], align 8
  %10 = alloca [32 x i8], align 8
  %11 = alloca [80 x i8], align 16
  %12 = alloca [40 x i8], align 8
  %13 = lshr i64 %3, 32
  %14 = mul nuw nsw i64 %2, 80
  %15 = getelementptr inbounds nuw i8, ptr %1, i64 %14
  %16 = icmp eq i64 %2, 0
  br i1 %16, label %._crit_edge, label %.lr.ph

17:                                               ; preds = %.lr.ph
  %18 = getelementptr inbounds nuw i8, ptr %20, i64 80
  %19 = icmp eq ptr %18, %15
  br i1 %19, label %._crit_edge, label %.lr.ph

.lr.ph:                                           ; preds = %4, %17
  %20 = phi ptr [ %18, %17 ], [ %1, %4 ]
  %21 = load i64, ptr %20, align 8, !range !5040, !noalias !100452, !noundef !1708
  %22 = getelementptr i8, ptr %20, i64 16
  %23 = load i64, ptr %22, align 8, !noalias !100452
  %24 = icmp ne i64 %21, -9223372036854775806
  tail call void @llvm.assume(i1 %24)
  %25 = icmp slt i64 %21, 0
  %26 = icmp samesign ult i64 %23, 20
  %27 = select i1 %25, i1 true, i1 %26
  br i1 %27, label %17, label %30

._crit_edge:                                      ; preds = %17, %4
  %28 = load i64, ptr %0, align 8, !range !20447, !noundef !1708
  %29 = icmp eq i64 %28, 2
  br i1 %29, label %32, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)

30:                                               ; preds = %.lr.ph
  %31 = load i64, ptr %0, align 8, !range !20447, !noundef !1708
  switch i64 %31, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit) [
    i64 2, label %32
    i64 1, label %42
    i64 4, label %188
    i64 3, label %188
  ]

32:                                               ; preds = %30, %._crit_edge
  %33 = phi i1 [ false, %30 ], [ true, %._crit_edge ]
  %34 = and i64 %3, 1
  %35 = icmp eq i64 %34, 0
  %36 = icmp eq i64 %13, 18
  %37 = and i1 %35, %36
  br i1 %37, label %38, label %42

38:                                               ; preds = %32
  %39 = and i64 %3, 65280
  %40 = icmp eq i64 %39, 0
  %41 = select i1 %33, i1 %40, i1 false
  br i1 %41, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), label %42

42:                                               ; preds = %38, %32, %30
  %43 = phi i1 [ false, %38 ], [ true, %30 ], [ false, %32 ]
  %44 = getelementptr inbounds nuw i8, ptr %8, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !100455
  %45 = icmp eq i64 %2, 0
  br i1 %45, label %46, label %.preheader

.preheader:                                       ; preds = %42
  %.sroa.962.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 16
  %.sroa.1265.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 24
  %.sroa.16.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 32
  %.sroa.18.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 36
  br label %47

46:                                               ; preds = %42
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !100455
  br label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)

47:                                               ; preds = %.preheader, %106
  %.sroa.073.0 = phi i64 [ %.sroa.060.0, %106 ], [ undef, %.preheader ]
  %.sroa.774.0 = phi i64 [ %.sroa.962.0, %106 ], [ undef, %.preheader ]
  %.sroa.975.0 = phi i64 [ %.sroa.1265.0, %106 ], [ undef, %.preheader ]
  %.sroa.1176.0 = phi i32 [ %.sroa.16.0, %106 ], [ undef, %.preheader ]
  %.sroa.13.0 = phi i32 [ %.sroa.18.0.copyload, %106 ], [ undef, %.preheader ]
  %48 = phi i64 [ %101, %106 ], [ 0, %.preheader ]
  %49 = phi i64 [ %102, %106 ], [ 0, %.preheader ]
  %50 = phi i64 [ %99, %106 ], [ 0, %.preheader ]
  %51 = phi i1 [ true, %106 ], [ false, %.preheader ]
  %52 = phi i64 [ %108, %106 ], [ 0, %.preheader ]
  %53 = phi i64 [ %107, %106 ], [ 0, %.preheader ]
  %54 = phi ptr [ %57, %106 ], [ %1, %.preheader ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100458)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100461)
  br label %55

55:                                               ; preds = %96, %47
  %56 = phi ptr [ %57, %96 ], [ %54, %47 ]
  %57 = getelementptr inbounds nuw i8, ptr %56, i64 80
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100464)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100467)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100469)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100472)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100474)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100477)
  %58 = load i64, ptr %56, align 8, !range !5040, !alias.scope !100479, !noalias !100480, !noundef !1708
  %59 = icmp ne i64 %58, -9223372036854775806
  tail call void @llvm.assume(i1 %59)
  %60 = icmp sgt i64 %58, -1
  %61 = getelementptr inbounds nuw i8, ptr %56, i64 48
  %62 = load i64, ptr %61, align 8, !range !2062, !alias.scope !100479, !noalias !100480
  %63 = icmp eq i64 %62, -1
  %64 = select i1 %60, i1 %63, i1 false
  br i1 %64, label %65, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread)

65:                                               ; preds = %55
  %66 = getelementptr inbounds nuw i8, ptr %56, i64 32
  %67 = load ptr, ptr %66, align 8, !alias.scope !100479, !noalias !100480, !nonnull !1708, !noundef !1708
  %68 = getelementptr inbounds nuw i8, ptr %56, i64 40
  %69 = load i64, ptr %68, align 8, !alias.scope !100479, !noalias !100480, !noundef !1708
  %70 = icmp samesign ult i64 %69, 33
  br i1 %70, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread), label %71

71:                                               ; preds = %65
  %72 = load i256, ptr @anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546, align 1
  %73 = load i256, ptr %67, align 1
  %74 = xor i256 %72, %73
  %75 = getelementptr i8, ptr %67, i64 32
  %76 = load i8, ptr getelementptr (i8, ptr @anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546, i64 32), align 1
  %77 = load i8, ptr %75, align 1
  %78 = zext i8 %76 to i256
  %79 = zext i8 %77 to i256
  %80 = xor i256 %78, %79
  %81 = or i256 %74, %80
  %82 = icmp ne i256 %81, 0
  %83 = zext i1 %82 to i32
  %84 = icmp eq i32 %83, 0
  br i1 %84, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit), label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread)

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit): ; preds = %71
  %85 = add i64 %69, -33
  %86 = getelementptr inbounds nuw i8, ptr %67, i64 33
; call <purrdf_xsd::datatype::XsdDatatype>::from_local
  %87 = tail call noundef i8 @<purrdf_xsd::datatype::XsdDatatype>::from_local(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %86, i64 noundef %85), !alias.scope !100483, !noalias !100486
  %88 = icmp eq i8 %87, -1
  br i1 %88, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread), label %89

89:                                               ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit)
  %90 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %91 = load i64, ptr %90, align 8, !alias.scope !100479, !noalias !100480, !noundef !1708
  %92 = getelementptr inbounds nuw i8, ptr %56, i64 8
  %93 = load ptr, ptr %92, align 8, !alias.scope !100479, !noalias !100480, !nonnull !1708, !noundef !1708
; call <purrdf_xsd::exact::cost::Shape>::of_lexical
  call void @<purrdf_xsd::exact::cost::Shape>::of_lexical(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %8, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %93, i64 noundef %91, i8 noundef %87), !noalias !100487
  %94 = load i64, ptr %8, align 8, !range !1855, !alias.scope !100488, !noalias !100489
  %95 = trunc nuw i64 %94 to i1
  br i1 %95, label %98, label %96

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread): ; preds = %65, %71, %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit), %55
  store i64 0, ptr %8, align 8, !alias.scope !100490, !noalias !100487
  br label %96

96:                                               ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread), %89
  %97 = icmp eq ptr %57, %15
  br i1 %97, label %187, label %55

98:                                               ; preds = %89
  %.sroa.060.0.copyload = load i64, ptr %44, align 8, !noalias !100455
  %.sroa.962.0.copyload = load i64, ptr %.sroa.962.0..sroa_idx, align 8, !noalias !100455
  %.sroa.1265.0.copyload = load i64, ptr %.sroa.1265.0..sroa_idx, align 8, !noalias !100455
  %.sroa.16.0.copyload = load i32, ptr %.sroa.16.0..sroa_idx, align 8, !noalias !100455
  %.sroa.18.0.copyload = load i32, ptr %.sroa.18.0..sroa_idx, align 4, !noalias !100455
  %99 = tail call i64 @llvm.uadd.sat.i64(i64 %50, i64 1)
  %100 = tail call i64 @llvm.usub.sat.i64(i64 %.sroa.962.0.copyload, i64 %.sroa.1265.0.copyload)
  %101 = tail call i64 @llvm.umax.i64(i64 %100, i64 %48)
  %102 = tail call i64 @llvm.umax.i64(i64 %.sroa.1265.0.copyload, i64 %49)
  br i1 %51, label %103, label %106

103:                                              ; preds = %98
  %104 = add i64 %99, -1
  %105 = icmp eq i64 %104, 0
  br i1 %105, label %147, label %111, !prof !1803

106:                                              ; preds = %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit), %183, %98
  %.sroa.060.0 = phi i64 [ %.sroa.060.0.copyload, %98 ], [ %143, %183 ], [ %143, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %.sroa.962.0 = phi i64 [ %.sroa.962.0.copyload, %98 ], [ %138, %183 ], [ %138, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %.sroa.1265.0 = phi i64 [ %.sroa.1265.0.copyload, %98 ], [ %102, %183 ], [ %102, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %.sroa.16.0 = phi i32 [ %.sroa.16.0.copyload, %98 ], [ -1, %183 ], [ -1, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %107 = phi i64 [ %53, %98 ], [ %53, %183 ], [ %181, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %108 = phi i64 [ %52, %98 ], [ %52, %183 ], [ %182, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !100455
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !100455
  %109 = icmp eq ptr %57, %15
  br i1 %109, label %110, label %47

110:                                              ; preds = %106
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !100455
  br label %355

111:                                              ; preds = %103
  %112 = icmp ne i64 %99, 1
  tail call void @llvm.assume(i1 %112)
  %113 = icmp ugt i64 %104, 9999999999
  %114 = udiv i64 %104, 10000000000
  %115 = select i1 %113, i32 10, i32 0
  %116 = select i1 %113, i64 %114, i64 %104
  %117 = icmp samesign ugt i64 %116, 99999
  br i1 %117, label %118, label %121

118:                                              ; preds = %111
  %119 = udiv i64 %116, 100000
  %120 = or disjoint i32 %115, 5
  br label %121

121:                                              ; preds = %118, %111
  %122 = phi i32 [ %120, %118 ], [ %115, %111 ]
  %123 = phi i64 [ %119, %118 ], [ %116, %111 ]
  %124 = trunc nuw nsw i64 %123 to i32
  %125 = add nuw nsw i32 %124, 393206
  %126 = add nuw nsw i32 %124, 524188
  %127 = and i32 %125, %126
  %128 = add nuw nsw i32 %124, 916504
  %129 = add nuw nsw i32 %124, 514288
  %130 = and i32 %128, %129
  %131 = xor i32 %127, %130
  %132 = lshr i32 %131, 17
  %133 = add nuw nsw i32 %132, %122
  %134 = icmp samesign ult i32 %133, 20
  tail call void @llvm.assume(i1 %134)
  %135 = add nuw nsw i32 %133, 1
  %136 = zext nneg i32 %135 to i64
  %137 = tail call i64 @llvm.uadd.sat.i64(i64 %101, i64 %136)
  %138 = tail call i64 @llvm.uadd.sat.i64(i64 %137, i64 %102)
  %139 = udiv i64 %138, 9
  %140 = urem i64 %138, 9
  %141 = icmp ne i64 %140, 0
  %142 = zext i1 %141 to i64
  %143 = add nuw nsw i64 %139, %142
  %144 = icmp ult i64 %.sroa.774.0, 39
  %145 = icmp ult i64 %.sroa.975.0, 19
  %146 = and i1 %144, %145
  br i1 %146, label %148, label %152

147:                                              ; preds = %103
; call core::num::imp::int_log10::panic_for_nonpositive_argument
  tail call void @core::num::imp::int_log10::panic_for_nonpositive_argument(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.554) #90, !noalias !100455
  unreachable

148:                                              ; preds = %121
  %149 = icmp ult i64 %.sroa.962.0.copyload, 39
  %150 = icmp ult i64 %.sroa.1265.0.copyload, 19
  %151 = and i1 %149, %150
  br i1 %151, label %183, label %152

152:                                              ; preds = %183, %148, %121
  %153 = icmp ult i64 %.sroa.975.0, %.sroa.1265.0.copyload
  br i1 %153, label %159, label %154

154:                                              ; preds = %152
  %155 = sub nuw i64 %.sroa.975.0, %.sroa.1265.0.copyload
  %156 = udiv i64 %155, 9
  %157 = add nuw nsw i64 %156, 1
  %158 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.060.0.copyload, i64 %157)
  br label %164

159:                                              ; preds = %152
  %160 = sub nuw i64 %.sroa.1265.0.copyload, %.sroa.975.0
  %161 = udiv i64 %160, 9
  %162 = add nuw nsw i64 %161, 1
  %163 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.073.0, i64 %162)
  br label %164

164:                                              ; preds = %159, %154
  %165 = phi i64 [ %163, %159 ], [ %158, %154 ]
  %166 = phi i64 [ %.sroa.060.0.copyload, %159 ], [ %158, %154 ]
  %167 = phi i64 [ %163, %159 ], [ %.sroa.073.0, %154 ]
  %168 = shl nuw i64 %165, 2
  %169 = icmp ugt i64 %165, 4611686018427387903
  br i1 %169, label %170, label %171, !prof !1803

170:                                              ; preds = %164
  br label %171

171:                                              ; preds = %170, %164
  %172 = phi i64 [ -1, %170 ], [ %168, %164 ]
  %173 = tail call i64 @llvm.umax.i64(i64 %167, i64 %166)
  %174 = tail call i64 @llvm.uadd.sat.i64(i64 %173, i64 1)
  %175 = shl nuw i64 %174, 2
  %176 = icmp ugt i64 %174, 4611686018427387903
  br i1 %176, label %177, label %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit), !prof !1803

177:                                              ; preds = %171
  br label %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit)

<purrdf_xsd::exact::cost::Shape>::add_cost (.exit): ; preds = %171, %177
  %178 = phi i64 [ -1, %177 ], [ %175, %171 ]
  %179 = tail call i64 @llvm.uadd.sat.i64(i64 %165, i64 %174)
  %180 = tail call i64 @llvm.uadd.sat.i64(i64 %172, i64 %178)
  %181 = tail call i64 @llvm.uadd.sat.i64(i64 %53, i64 %179)
  %182 = tail call i64 @llvm.umax.i64(i64 %52, i64 %180)
  br label %106

183:                                              ; preds = %148
  %184 = icmp ult i64 %138, 39
  %185 = icmp ult i64 %49, 19
  %186 = and i1 %185, %184
  br i1 %186, label %106, label %152

187:                                              ; preds = %96
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !100455
  br i1 %51, label %355, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)

188:                                              ; preds = %30, %30
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !100491
  br label %189

189:                                              ; preds = %230, %188
  %190 = phi ptr [ %191, %230 ], [ %1, %188 ]
  %191 = getelementptr inbounds nuw i8, ptr %190, i64 80
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100494)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100497)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100500)
  %192 = load i64, ptr %190, align 8, !range !5040, !alias.scope !100503, !noalias !100504, !noundef !1708
  %193 = icmp ne i64 %192, -9223372036854775806
  tail call void @llvm.assume(i1 %193)
  %194 = icmp sgt i64 %192, -1
  %195 = getelementptr inbounds nuw i8, ptr %190, i64 48
  %196 = load i64, ptr %195, align 8, !range !2062, !alias.scope !100503, !noalias !100504
  %197 = icmp eq i64 %196, -1
  %198 = select i1 %194, i1 %197, i1 false
  br i1 %198, label %199, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread)

199:                                              ; preds = %189
  %200 = getelementptr inbounds nuw i8, ptr %190, i64 32
  %201 = load ptr, ptr %200, align 8, !alias.scope !100503, !noalias !100504, !nonnull !1708, !noundef !1708
  %202 = getelementptr inbounds nuw i8, ptr %190, i64 40
  %203 = load i64, ptr %202, align 8, !alias.scope !100503, !noalias !100504, !noundef !1708
  %204 = icmp samesign ult i64 %203, 33
  br i1 %204, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread), label %205

205:                                              ; preds = %199
  %206 = load i256, ptr @anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546, align 1
  %207 = load i256, ptr %201, align 1
  %208 = xor i256 %206, %207
  %209 = getelementptr i8, ptr %201, i64 32
  %210 = load i8, ptr getelementptr (i8, ptr @anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546, i64 32), align 1
  %211 = load i8, ptr %209, align 1
  %212 = zext i8 %210 to i256
  %213 = zext i8 %211 to i256
  %214 = xor i256 %212, %213
  %215 = or i256 %208, %214
  %216 = icmp ne i256 %215, 0
  %217 = zext i1 %216 to i32
  %218 = icmp eq i32 %217, 0
  br i1 %218, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18), label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread)

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18): ; preds = %205
  %219 = add i64 %203, -33
  %220 = getelementptr inbounds nuw i8, ptr %201, i64 33
; call <purrdf_xsd::datatype::XsdDatatype>::from_local
  %221 = tail call noundef i8 @<purrdf_xsd::datatype::XsdDatatype>::from_local(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %220, i64 noundef %219), !alias.scope !100514, !noalias !100517
  %222 = icmp eq i8 %221, -1
  br i1 %222, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread), label %223

223:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18)
  %224 = getelementptr inbounds nuw i8, ptr %190, i64 16
  %225 = load i64, ptr %224, align 8, !alias.scope !100503, !noalias !100504, !noundef !1708
  %226 = getelementptr inbounds nuw i8, ptr %190, i64 8
  %227 = load ptr, ptr %226, align 8, !alias.scope !100503, !noalias !100504, !nonnull !1708, !noundef !1708
; call <purrdf_xsd::exact::cost::Shape>::of_lexical
  call void @<purrdf_xsd::exact::cost::Shape>::of_lexical(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %6, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %227, i64 noundef %225, i8 noundef %221), !noalias !100491
  %228 = load i64, ptr %6, align 8, !range !1855, !noalias !100491
  %229 = trunc nuw i64 %228 to i1
  br i1 %229, label %232, label %230

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread): ; preds = %199, %205, %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18), %189
  store i64 0, ptr %6, align 8, !noalias !100491
  br label %230

230:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread), %223
  %231 = icmp eq ptr %191, %15
  br i1 %231, label %.loopexit85, label %189

232:                                              ; preds = %223
  %233 = tail call noundef dereferenceable_or_null(128) ptr @malloc(i64 noundef range(i64 1, 0) 128) #88, !noalias !100518
  %234 = icmp eq ptr %233, null
  br i1 %234, label %__rustc::__rust_alloc (.exit.thread), label %235

235:                                              ; preds = %232
  %236 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %237 = load i64, ptr %236, align 8, !noalias !100518, !noundef !1708
  %238 = tail call i64 @llvm.uadd.sat.i64(i64 %237, i64 1)
  store i64 %238, ptr %236, align 8, !noalias !100518
  %239 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %240 = load i64, ptr %239, align 8, !noalias !100518, !noundef !1708
  %241 = tail call i64 @llvm.uadd.sat.i64(i64 %240, i64 128)
  store i64 %241, ptr %239, align 8, !noalias !100518
  %242 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %243 = load i64, ptr %242, align 8, !noalias !100518, !noundef !1708
  %244 = tail call i64 @llvm.sadd.sat.i64(i64 %243, i64 128)
  store i64 %244, ptr %242, align 8, !noalias !100518
  %245 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %246 = load i64, ptr %245, align 8, !noalias !100518, !noundef !1708
  %247 = icmp sgt i64 %244, %246
  br i1 %247, label %248, label %.preheader320

248:                                              ; preds = %235
  store i64 %244, ptr %245, align 8, !noalias !100518
  br label %.preheader320

.preheader320:                                    ; preds = %248, %235
  br label %249

249:                                              ; preds = %.preheader320, %252
  %250 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !100518
  %251 = icmp slt i64 %250, 0
  br i1 %251, label %252, label %__rustc::__rust_alloc (.exit)

252:                                              ; preds = %249
  %253 = add nsw i64 %250, 1
  %254 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %250, i64 %253 acq_rel acquire, align 8, !noalias !100518
  %255 = extractvalue { i64, i1 } %254, 1
  br i1 %255, label %256, label %249

256:                                              ; preds = %252
  %257 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !100518
  %258 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 128 monotonic, align 8, !noalias !100518
  %259 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 128 monotonic, align 8, !noalias !100518
  %260 = tail call i64 @llvm.sadd.sat.i64(i64 %259, i64 128)
  %261 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !100518
  br label %262

262:                                              ; preds = %265, %256
  %263 = phi i64 [ %261, %256 ], [ %268, %265 ]
  %264 = icmp sgt i64 %260, %263
  br i1 %264, label %265, label %269

265:                                              ; preds = %262
  %266 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %263, i64 %260 monotonic monotonic, align 8, !noalias !100518
  %267 = extractvalue { i64, i1 } %266, 1
  %268 = extractvalue { i64, i1 } %266, 0
  br i1 %267, label %269, label %262

269:                                              ; preds = %265, %262
  %270 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !100518
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %232
; call alloc::raw_vec::handle_error
  tail call void @alloc::raw_vec::handle_error(i64 noundef 8, i64 128) #90, !noalias !100491
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %249, %269
  %271 = getelementptr inbounds nuw i8, ptr %6, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %233, ptr noundef nonnull align 8 dereferenceable(32) %271, i64 32, i1 false), !noalias !100491
  store i64 4, ptr %7, align 8, !noalias !100491
  %272 = getelementptr inbounds nuw i8, ptr %7, i64 8
  store ptr %233, ptr %272, align 8, !noalias !100491
  %273 = getelementptr inbounds nuw i8, ptr %7, i64 16
  store i64 1, ptr %273, align 8, !noalias !100491
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100521)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !100524)
  call void @llvm.lifetime.start.p0(ptr nonnull %5), !noalias !100527
  %274 = getelementptr inbounds nuw i8, ptr %5, i64 8
  %275 = icmp eq ptr %191, %15
  br i1 %275, label %.loopexit, label %.preheader83

.preheader83:                                     ; preds = %__rustc::__rust_alloc (.exit), %330
  %276 = phi ptr [ %331, %330 ], [ %233, %__rustc::__rust_alloc (.exit) ]
  %277 = phi i64 [ %333, %330 ], [ 1, %__rustc::__rust_alloc (.exit) ]
  %278 = phi ptr [ %281, %330 ], [ %191, %__rustc::__rust_alloc (.exit) ]
  br label %279

279:                                              ; preds = %321, %.preheader83
  %280 = phi ptr [ %281, %321 ], [ %278, %.preheader83 ]
  %281 = getelementptr inbounds nuw i8, ptr %280, i64 80
  %282 = load i64, ptr %280, align 8, !range !5040, !alias.scope !100528, !noalias !100535, !noundef !1708
  %283 = icmp ne i64 %282, -9223372036854775806
  tail call void @llvm.assume(i1 %283)
  %284 = icmp sgt i64 %282, -1
  %285 = getelementptr inbounds nuw i8, ptr %280, i64 48
  %286 = load i64, ptr %285, align 8, !range !2062, !alias.scope !100528, !noalias !100535
  %287 = icmp eq i64 %286, -1
  %288 = select i1 %284, i1 %287, i1 false
  br i1 %288, label %289, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread)

289:                                              ; preds = %279
  %290 = getelementptr inbounds nuw i8, ptr %280, i64 8
  %291 = load ptr, ptr %290, align 8, !alias.scope !100528, !noalias !100535, !nonnull !1708, !noundef !1708
  %292 = getelementptr inbounds nuw i8, ptr %280, i64 16
  %293 = load i64, ptr %292, align 8, !alias.scope !100528, !noalias !100535, !noundef !1708
  %294 = getelementptr inbounds nuw i8, ptr %280, i64 32
  %295 = load ptr, ptr %294, align 8, !alias.scope !100528, !noalias !100535, !nonnull !1708, !noundef !1708
  %296 = getelementptr inbounds nuw i8, ptr %280, i64 40
  %297 = load i64, ptr %296, align 8, !alias.scope !100528, !noalias !100535, !noundef !1708
  %298 = icmp samesign ult i64 %297, 33
  br i1 %298, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread), label %299

299:                                              ; preds = %289
  %300 = load i256, ptr @anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546, align 1
  %301 = load i256, ptr %295, align 1
  %302 = xor i256 %300, %301
  %303 = getelementptr i8, ptr %295, i64 32
  %304 = load i8, ptr getelementptr (i8, ptr @anon.20c7abfb087b18349414c00ff0e99331.131.llvm.13195840536648017546, i64 32), align 1
  %305 = load i8, ptr %303, align 1
  %306 = zext i8 %304 to i256
  %307 = zext i8 %305 to i256
  %308 = xor i256 %306, %307
  %309 = or i256 %302, %308
  %310 = icmp ne i256 %309, 0
  %311 = zext i1 %310 to i32
  %312 = icmp eq i32 %311, 0
  br i1 %312, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19), label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread)

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19): ; preds = %299
  %313 = add i64 %297, -33
  %314 = getelementptr inbounds nuw i8, ptr %295, i64 33
; call <purrdf_xsd::datatype::XsdDatatype>::from_local
  %315 = tail call noundef i8 @<purrdf_xsd::datatype::XsdDatatype>::from_local(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %314, i64 noundef %313), !alias.scope !100545, !noalias !100491
  %316 = icmp eq i8 %315, -1
  br i1 %316, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread), label %317

317:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19)
; invoke <purrdf_xsd::exact::cost::Shape>::of_lexical
  invoke void @<purrdf_xsd::exact::cost::Shape>::of_lexical(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %5, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %291, i64 noundef %293, i8 noundef %315)
          to label %318 unwind label %335, !noalias !100491

318:                                              ; preds = %317
  %319 = load i64, ptr %5, align 8, !range !1855, !noalias !100548
  %320 = trunc nuw i64 %319 to i1
  br i1 %320, label %323, label %321

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread): ; preds = %289, %299, %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19), %279
  store i64 0, ptr %5, align 8, !noalias !100548
  br label %321

321:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread), %318
  %322 = icmp eq ptr %281, %15
  br i1 %322, label %.loopexit, label %279

323:                                              ; preds = %318
  %324 = icmp samesign ult i64 %277, 288230376151711744
  tail call void @llvm.assume(i1 %324)
  %325 = load i64, ptr %7, align 8, !range !1817, !alias.scope !100549, !noalias !100491, !noundef !1708
  %326 = icmp eq i64 %277, %325
  br i1 %326, label %327, label %330

327:                                              ; preds = %323
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %7, i64 noundef %277, i64 noundef 1, i64 noundef 8, i64 noundef 32)
          to label %328 unwind label %337, !noalias !100491

328:                                              ; preds = %327
  %329 = load ptr, ptr %272, align 8, !alias.scope !100549, !noalias !100491
  br label %330

330:                                              ; preds = %328, %323
  %331 = phi ptr [ %329, %328 ], [ %276, %323 ]
  %332 = getelementptr inbounds nuw [32 x i8], ptr %331, i64 %277
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %332, ptr noundef nonnull align 8 dereferenceable(32) %274, i64 32, i1 false), !noalias !100548
  %333 = add nuw nsw i64 %277, 1
  store i64 %333, ptr %273, align 8, !alias.scope !100549, !noalias !100491
  %334 = icmp eq ptr %281, %15
  br i1 %334, label %.loopexit, label %.preheader83

335:                                              ; preds = %317
  %336 = landingpad { ptr, i32 }
          cleanup
  br label %339

337:                                              ; preds = %327
  %338 = landingpad { ptr, i32 }
          cleanup
  br label %339

339:                                              ; preds = %337, %335
  %340 = phi { ptr, i32 } [ %336, %335 ], [ %338, %337 ]
  %341 = load i64, ptr %7, align 8, !noalias !100491
  %342 = icmp eq i64 %341, 0
  br i1 %342, label %349, label %343

343:                                              ; preds = %339
  %344 = load ptr, ptr %272, align 8, !noalias !100491, !nonnull !1708, !noundef !1708
  %345 = shl nuw i64 %341, 5
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %344, i64 noundef %345, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !100491
  br label %349

.loopexit:                                        ; preds = %330, %321, %__rustc::__rust_alloc (.exit)
  %346 = phi i64 [ %277, %321 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %333, %330 ]
  %347 = phi ptr [ %276, %321 ], [ %233, %__rustc::__rust_alloc (.exit) ], [ %331, %330 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %5), !noalias !100527
  %348 = load i64, ptr %7, align 8
  br label %.loopexit85

349:                                              ; preds = %455, %452, %395, %343, %339
  %350 = phi { ptr, i32 } [ %340, %339 ], [ %340, %343 ], [ %396, %395 ], [ %453, %452 ], [ %453, %455 ]
  resume { ptr, i32 } %350

.loopexit85:                                      ; preds = %230, %.loopexit
  %351 = phi i64 [ %348, %.loopexit ], [ 0, %230 ]
  %352 = phi ptr [ %347, %.loopexit ], [ inttoptr (i64 8 to ptr), %230 ]
  %353 = phi i64 [ %346, %.loopexit ], [ 0, %230 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !100491
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
; invoke purrdf_xsd::exact::cost::compare_chain
  %354 = invoke { i64, i64 } @purrdf_xsd::exact::cost::compare_chain(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %352, i64 noundef %353, i64 noundef 1)
          to label %457 unwind label %452

355:                                              ; preds = %187, %110
  %.sroa.12.0 = phi i32 [ %.sroa.13.0, %187 ], [ %.sroa.18.0.copyload, %110 ]
  %.sroa.11.0 = phi i32 [ %.sroa.1176.0, %187 ], [ %.sroa.16.0, %110 ]
  %.sroa.1031.0 = phi i64 [ %.sroa.975.0, %187 ], [ %.sroa.1265.0, %110 ]
  %.sroa.926.0 = phi i64 [ %.sroa.774.0, %187 ], [ %.sroa.962.0, %110 ]
  %.sroa.023.0 = phi i64 [ %.sroa.073.0, %187 ], [ %.sroa.060.0, %110 ]
  %356 = phi i64 [ %52, %187 ], [ %108, %110 ]
  %357 = phi i64 [ %53, %187 ], [ %107, %110 ]
  %358 = icmp ult i64 %.sroa.926.0, 39
  %359 = icmp ult i64 %.sroa.1031.0, 19
  %360 = and i1 %359, %358
  br i1 %360, label %361, label %369

361:                                              ; preds = %355
  %362 = and i64 %3, 1
  %363 = icmp eq i64 %362, 0
  %364 = icmp eq i64 %13, 18
  %365 = and i1 %363, %364
  %366 = and i64 %3, 65280
  %367 = icmp eq i64 %366, 0
  %368 = select i1 %365, i1 %367, i1 false
  br i1 %368, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), label %369

369:                                              ; preds = %361, %355
  br i1 %43, label %374, label %370

370:                                              ; preds = %369
  call void @llvm.lifetime.start.p0(ptr nonnull %12)
  call void @llvm.lifetime.start.p0(ptr nonnull %11)
  %371 = zext nneg i64 %2 to i128
  %372 = getelementptr inbounds nuw i8, ptr %11, i64 16
  store i128 %371, ptr %372, align 16
  %373 = getelementptr inbounds nuw i8, ptr %11, i64 1
  store i8 0, ptr %373, align 1
  store i8 0, ptr %11, align 16
; invoke purrdf_xsd::numeric::exact_path::shape_of
  invoke fastcc void @purrdf_xsd::numeric::exact_path::shape_of (.llvm.13195840536648017546)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(40) %12, ptr noalias nofree noundef nonnull readonly align 16 captures(address, read_provenance) dereferenceable(80) %11)
          to label %<purrdf_xsd::exact::cost::Shape>::of_value (.exit) unwind label %395

374:                                              ; preds = %369
  %375 = icmp ult i64 %.sroa.1031.0, %.sroa.926.0
  br i1 %375, label %376, label %378

376:                                              ; preds = %374
  %377 = icmp eq i64 %.sroa.1031.0, 0
  br i1 %377, label %382, label %380

378:                                              ; preds = %374
  %379 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.1031.0, i64 2)
  br label %382

380:                                              ; preds = %376
  %381 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.926.0, i64 1)
  br label %382

382:                                              ; preds = %380, %378, %376
  %383 = phi i64 [ %379, %378 ], [ %381, %380 ], [ %.sroa.926.0, %376 ]
  %384 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %.sroa.023.0, i64 9)
  %385 = extractvalue { i64, i1 } %384, 1
  br i1 %385, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), label %386, !prof !1803

386:                                              ; preds = %382
  %387 = icmp slt i32 %.sroa.11.0, 0
  %388 = tail call i64 @llvm.uadd.sat.i64(i64 %383, i64 1)
  %389 = select i1 %387, i64 %388, i64 %383
  %390 = extractvalue { i64, i1 } %384, 0
  %391 = tail call i64 @llvm.uadd.sat.i64(i64 %390, i64 %389)
  %392 = tail call i64 @llvm.uadd.sat.i64(i64 %391, i64 1)
  %393 = tail call i64 @llvm.uadd.sat.i64(i64 %357, i64 %392)
  %394 = tail call i64 @llvm.umax.i64(i64 %356, i64 %391)
  br label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)

395:                                              ; preds = %370, %446
  %396 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
  call fastcc void @core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>(ptr noalias nofree noundef align 16 dereferenceable(80) %11) #89
  br label %349

<purrdf_xsd::exact::cost::Shape>::of_value (.exit): ; preds = %370
  %397 = load i64, ptr %12, align 8, !range !1855, !noundef !1708
  %398 = trunc nuw i64 %397 to i1
  br i1 %398, label %399, label %446, !prof !1974

399:                                              ; preds = %<purrdf_xsd::exact::cost::Shape>::of_value (.exit)
  %400 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %.sroa.926.0..sroa_idx29 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %.sroa.1031.0..sroa_idx34 = getelementptr inbounds nuw i8, ptr %10, i64 16
  %.sroa.11.0..sroa_idx38 = getelementptr inbounds nuw i8, ptr %10, i64 24
  %.sroa.12.0..sroa_idx42 = getelementptr inbounds nuw i8, ptr %10, i64 28
  %401 = load <4 x i64>, ptr %400, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %12)
; call core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
  call fastcc void @core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>(ptr noalias nofree noundef align 16 dereferenceable(80) %11)
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  store i64 %.sroa.023.0, ptr %10, align 8
  store i64 %.sroa.926.0, ptr %.sroa.926.0..sroa_idx29, align 8
  store i64 %.sroa.1031.0, ptr %.sroa.1031.0..sroa_idx34, align 8
  store i32 %.sroa.11.0, ptr %.sroa.11.0..sroa_idx38, align 8
  store i32 %.sroa.12.0, ptr %.sroa.12.0..sroa_idx42, align 4
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  store <4 x i64> %401, ptr %9, align 8
; call purrdf_xsd::exact::cost::decimal_div
  %402 = call { i64, i64 } @purrdf_xsd::exact::cost::decimal_div(ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(32) %10, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(32) %9, i64 %3), !alias.scope !100550
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %403 = extractvalue { i64, i64 } %402, 0
  %404 = extractvalue { i64, i64 } %402, 1
  %405 = tail call i64 @llvm.uadd.sat.i64(i64 %357, i64 %403)
  %406 = tail call i64 @llvm.umax.i64(i64 %356, i64 %404)
  %407 = trunc i64 %3 to i1
  br i1 %407, label %408, label %416

408:                                              ; preds = %399
  %409 = extractelement <4 x i64> %401, i64 1
  %410 = shl nuw i64 %409, 2
  %411 = icmp ugt i64 %409, 4611686018427387903
  br i1 %411, label %412, label %413, !prof !1803

412:                                              ; preds = %408
  br label %413

413:                                              ; preds = %412, %408
  %414 = phi i64 [ -1, %412 ], [ %410, %408 ]
  %415 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.1031.0, i64 %414)
  br label %416

416:                                              ; preds = %413, %399
  %417 = phi i64 [ %415, %413 ], [ %13, %399 ]
  %418 = tail call i64 @llvm.usub.sat.i64(i64 %.sroa.926.0, i64 %.sroa.1031.0)
  %419 = extractelement <4 x i64> %401, i64 2
  %420 = tail call i64 @llvm.uadd.sat.i64(i64 %418, i64 %419)
  %421 = tail call i64 @llvm.uadd.sat.i64(i64 %420, i64 1)
  %422 = tail call i64 @llvm.uadd.sat.i64(i64 %421, i64 %417)
  %423 = udiv i64 %422, 9
  %424 = urem i64 %422, 9
  %425 = icmp ne i64 %424, 0
  %426 = zext i1 %425 to i64
  %427 = add nuw nsw i64 %423, %426
  %428 = icmp ult i64 %417, %422
  br i1 %428, label %429, label %431

429:                                              ; preds = %416
  %430 = icmp eq i64 %417, 0
  br i1 %430, label %435, label %433

431:                                              ; preds = %416
  %432 = tail call i64 @llvm.uadd.sat.i64(i64 %417, i64 2)
  br label %435

433:                                              ; preds = %429
  %434 = tail call i64 @llvm.uadd.sat.i64(i64 %422, i64 1)
  br label %435

435:                                              ; preds = %433, %431, %429
  %436 = phi i64 [ %432, %431 ], [ %434, %433 ], [ %422, %429 ]
  %437 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %427, i64 9)
  %438 = extractvalue { i64, i1 } %437, 1
  br i1 %438, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), label %439, !prof !1803

439:                                              ; preds = %435
  %440 = tail call i64 @llvm.uadd.sat.i64(i64 %436, i64 1)
  %441 = extractvalue { i64, i1 } %437, 0
  %442 = tail call i64 @llvm.uadd.sat.i64(i64 %441, i64 %440)
  %443 = tail call i64 @llvm.uadd.sat.i64(i64 %442, i64 1)
  %444 = tail call i64 @llvm.uadd.sat.i64(i64 %405, i64 %443)
  %445 = tail call i64 @llvm.umax.i64(i64 %406, i64 %442)
  br label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)

446:                                              ; preds = %<purrdf_xsd::exact::cost::Shape>::of_value (.exit)
; invoke core::option::expect_failed
  invoke void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.1836, i64 noundef 22, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.1837) #90
          to label %447 unwind label %395

447:                                              ; preds = %446
  unreachable

<purrdf_xsd::exact::cost::Shape>::render_cost (.exit): ; preds = %439, %435, %386, %382, %46, %187, %361, %__rustc::__rust_dealloc (.exit), %457, %38, %30, %._crit_edge
  %448 = phi i64 [ 0, %38 ], [ %458, %__rustc::__rust_dealloc (.exit) ], [ -1, %382 ], [ 0, %._crit_edge ], [ 0, %30 ], [ %458, %457 ], [ 0, %46 ], [ %53, %187 ], [ %357, %361 ], [ %393, %386 ], [ %444, %439 ], [ -1, %435 ]
  %449 = phi i64 [ 0, %38 ], [ %459, %__rustc::__rust_dealloc (.exit) ], [ -1, %382 ], [ 0, %._crit_edge ], [ 0, %30 ], [ %459, %457 ], [ 0, %46 ], [ %52, %187 ], [ %356, %361 ], [ %394, %386 ], [ %445, %439 ], [ -1, %435 ]
  %450 = insertvalue { i64, i64 } poison, i64 %448, 0
  %451 = insertvalue { i64, i64 } %450, i64 %449, 1
  ret { i64, i64 } %451

452:                                              ; preds = %.loopexit85
  %453 = landingpad { ptr, i32 }
          cleanup
  %454 = icmp eq i64 %351, 0
  br i1 %454, label %349, label %455

455:                                              ; preds = %452
  %456 = shl nuw i64 %351, 5
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %352) ]
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %352, i64 noundef %456, i64 noundef range(i64 1, -9223372036854775807) 8) #88
  br label %349

457:                                              ; preds = %.loopexit85
  %458 = extractvalue { i64, i64 } %354, 0
  %459 = extractvalue { i64, i64 } %354, 1
  %460 = icmp eq i64 %351, 0
  br i1 %460, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), label %461

461:                                              ; preds = %457
  %462 = shl nuw i64 %351, 5
  %463 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %464 = load i64, ptr %463, align 8, !noundef !1708
  %465 = tail call i64 @llvm.umin.i64(i64 %462, i64 9223372036854775807)
  %466 = tail call i64 @llvm.ssub.sat.i64(i64 %464, i64 %465)
  store i64 %466, ptr %463, align 8
  %467 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %468 = load i64, ptr %467, align 8, !noundef !1708
  %469 = icmp slt i64 %466, %468
  br i1 %469, label %470, label %.preheader305

470:                                              ; preds = %461
  store i64 %466, ptr %467, align 8
  br label %.preheader305

.preheader305:                                    ; preds = %470, %461
  br label %471

471:                                              ; preds = %.preheader305, %474
  %472 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8
  %473 = icmp slt i64 %472, 0
  br i1 %473, label %474, label %__rustc::__rust_dealloc (.exit)

474:                                              ; preds = %471
  %475 = add nsw i64 %472, 1
  %476 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %472, i64 %475 acq_rel acquire, align 8
  %477 = extractvalue { i64, i1 } %476, 1
  br i1 %477, label %478, label %471

478:                                              ; preds = %474
  %479 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %465 monotonic, align 8
  %480 = tail call i64 @llvm.ssub.sat.i64(i64 %479, i64 %465)
  %481 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %482

482:                                              ; preds = %485, %478
  %483 = phi i64 [ %481, %478 ], [ %488, %485 ]
  %484 = icmp slt i64 %480, %483
  br i1 %484, label %485, label %489

485:                                              ; preds = %482
  %486 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %483, i64 %480 monotonic monotonic, align 8
  %487 = extractvalue { i64, i1 } %486, 1
  %488 = extractvalue { i64, i1 } %486, 0
  br i1 %487, label %489, label %482

489:                                              ; preds = %485, %482
  %490 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %471, %489
  tail call void @free(ptr noundef nonnull %352) #88
  br label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)
}

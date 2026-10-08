define { i64, i64 } @purrdf_sparql_eval::modifier::aggregate_numeric_cost(ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %1, i64 noundef range(i64 0, 115292150460684698) %2, i64 %3) unnamed_addr #0 personality ptr @rust_eh_personality !guid !109361 {
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
  %21 = load i64, ptr %20, align 8, !range !4657, !noalias !109362, !noundef !1733
  %22 = getelementptr i8, ptr %20, i64 16
  %23 = load i64, ptr %22, align 8, !noalias !109362
  %24 = icmp ne i64 %21, -9223372036854775806
  tail call void @llvm.assume(i1 %24)
  %25 = icmp slt i64 %21, 0
  %26 = icmp samesign ult i64 %23, 20
  %27 = select i1 %25, i1 true, i1 %26
  br i1 %27, label %17, label %30

._crit_edge:                                      ; preds = %17, %4
  %28 = load i64, ptr %0, align 8, !range !14951, !noundef !1733
  %29 = icmp eq i64 %28, 2
  br i1 %29, label %32, label %446

30:                                               ; preds = %.lr.ph
  %31 = load i64, ptr %0, align 8, !range !14951, !noundef !1733
  switch i64 %31, label %446 [
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
  br i1 %41, label %446, label %42

42:                                               ; preds = %38, %32, %30
  %43 = phi i1 [ false, %38 ], [ true, %30 ], [ false, %32 ]
  %44 = getelementptr inbounds nuw i8, ptr %8, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !109365
  %45 = icmp eq i64 %2, 0
  br i1 %45, label %46, label %.preheader

.preheader:                                       ; preds = %42
  %.sroa.676.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 16
  %.sroa.977.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 24
  %.sroa.13.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 32
  %.sroa.15.0..sroa_idx = getelementptr inbounds nuw i8, ptr %8, i64 36
  br label %47

46:                                               ; preds = %42
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !109365
  br label %446

47:                                               ; preds = %.preheader, %182
  %.sroa.060.0 = phi i64 [ %.sroa.075.0, %182 ], [ undef, %.preheader ]
  %.sroa.862.0 = phi i64 [ %.sroa.676.0, %182 ], [ undef, %.preheader ]
  %.sroa.1065.0 = phi i64 [ %.sroa.977.0, %182 ], [ undef, %.preheader ]
  %.sroa.1268.0 = phi i32 [ %.sroa.13.0, %182 ], [ undef, %.preheader ]
  %.sroa.1471.0 = phi i32 [ %.sroa.15.0.copyload, %182 ], [ undef, %.preheader ]
  %48 = phi i1 [ true, %182 ], [ false, %.preheader ]
  %49 = phi i64 [ %184, %182 ], [ 0, %.preheader ]
  %50 = phi i64 [ %183, %182 ], [ 0, %.preheader ]
  %51 = phi i64 [ %101, %182 ], [ 0, %.preheader ]
  %52 = phi i64 [ %102, %182 ], [ 0, %.preheader ]
  %53 = phi i64 [ %99, %182 ], [ 0, %.preheader ]
  %54 = phi ptr [ %57, %182 ], [ %1, %.preheader ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109368)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109371)
  br label %55

55:                                               ; preds = %96, %47
  %56 = phi ptr [ %57, %96 ], [ %54, %47 ]
  %57 = getelementptr inbounds nuw i8, ptr %56, i64 80
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109374)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109377)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109379)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109382)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109384)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109387)
  %58 = load i64, ptr %56, align 8, !range !4657, !alias.scope !109389, !noalias !109390, !noundef !1733
  %59 = icmp ne i64 %58, -9223372036854775806
  tail call void @llvm.assume(i1 %59)
  %60 = icmp sgt i64 %58, -1
  %61 = getelementptr inbounds nuw i8, ptr %56, i64 48
  %62 = load i64, ptr %61, align 8, !range !2052, !alias.scope !109389, !noalias !109390
  %63 = icmp eq i64 %62, -1
  %64 = select i1 %60, i1 %63, i1 false
  br i1 %64, label %65, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread)

65:                                               ; preds = %55
  %66 = getelementptr inbounds nuw i8, ptr %56, i64 32
  %67 = load ptr, ptr %66, align 8, !alias.scope !109389, !noalias !109390, !nonnull !1733, !noundef !1733
  %68 = getelementptr inbounds nuw i8, ptr %56, i64 40
  %69 = load i64, ptr %68, align 8, !alias.scope !109389, !noalias !109390, !noundef !1733
  %70 = icmp samesign ult i64 %69, 33
  br i1 %70, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread), label %71

71:                                               ; preds = %65
  %72 = load i256, ptr @anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276, align 1
  %73 = load i256, ptr %67, align 1
  %74 = xor i256 %72, %73
  %75 = getelementptr i8, ptr %67, i64 32
  %76 = load i8, ptr getelementptr (i8, ptr @anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276, i64 32), align 1
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
  %87 = tail call noundef i8 @<purrdf_xsd::datatype::XsdDatatype>::from_local(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %86, i64 noundef %85), !alias.scope !109393, !noalias !109396
  %88 = icmp eq i8 %87, -1
  br i1 %88, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread), label %89

89:                                               ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit)
  %90 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %91 = load i64, ptr %90, align 8, !alias.scope !109389, !noalias !109390, !noundef !1733
  %92 = getelementptr inbounds nuw i8, ptr %56, i64 8
  %93 = load ptr, ptr %92, align 8, !alias.scope !109389, !noalias !109390, !nonnull !1733, !noundef !1733
; call <purrdf_xsd::exact::cost::Shape>::of_lexical
  call void @<purrdf_xsd::exact::cost::Shape>::of_lexical(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %8, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %93, i64 noundef %91, i8 noundef %87), !noalias !109397
  %94 = load i64, ptr %8, align 8, !range !1732, !alias.scope !109398, !noalias !109399
  %95 = trunc nuw i64 %94 to i1
  br i1 %95, label %98, label %96

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread): ; preds = %65, %71, %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit), %55
  store i64 0, ptr %8, align 8, !alias.scope !109400, !noalias !109397
  br label %96

96:                                               ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit.thread), %89
  %97 = icmp eq ptr %57, %15
  br i1 %97, label %187, label %55

98:                                               ; preds = %89
  %.sroa.075.0.copyload = load i64, ptr %44, align 8, !noalias !109365
  %.sroa.676.0.copyload = load i64, ptr %.sroa.676.0..sroa_idx, align 8, !noalias !109365
  %.sroa.977.0.copyload = load i64, ptr %.sroa.977.0..sroa_idx, align 8, !noalias !109365
  %.sroa.13.0.copyload = load i32, ptr %.sroa.13.0..sroa_idx, align 8, !noalias !109365
  %.sroa.15.0.copyload = load i32, ptr %.sroa.15.0..sroa_idx, align 4, !noalias !109365
  %99 = tail call i64 @llvm.uadd.sat.i64(i64 %53, i64 1)
  %100 = tail call i64 @llvm.usub.sat.i64(i64 %.sroa.676.0.copyload, i64 %.sroa.977.0.copyload)
  %101 = tail call i64 @llvm.umax.i64(i64 %100, i64 %51)
  %102 = tail call i64 @llvm.umax.i64(i64 %.sroa.977.0.copyload, i64 %52)
  br i1 %48, label %103, label %182

103:                                              ; preds = %98
  %104 = add i64 %99, -1
  %105 = icmp eq i64 %104, 0
  br i1 %105, label %142, label %106, !prof !1735

106:                                              ; preds = %103
  %107 = icmp ne i64 %99, 1
  tail call void @llvm.assume(i1 %107)
  %108 = icmp ugt i64 %104, 9999999999
  %109 = udiv i64 %104, 10000000000
  %110 = select i1 %108, i32 10, i32 0
  %111 = select i1 %108, i64 %109, i64 %104
  %112 = icmp samesign ugt i64 %111, 99999
  br i1 %112, label %113, label %116

113:                                              ; preds = %106
  %114 = udiv i64 %111, 100000
  %115 = or disjoint i32 %110, 5
  br label %116

116:                                              ; preds = %113, %106
  %117 = phi i32 [ %115, %113 ], [ %110, %106 ]
  %118 = phi i64 [ %114, %113 ], [ %111, %106 ]
  %119 = trunc nuw nsw i64 %118 to i32
  %120 = add nuw nsw i32 %119, 393206
  %121 = add nuw nsw i32 %119, 524188
  %122 = and i32 %120, %121
  %123 = add nuw nsw i32 %119, 916504
  %124 = add nuw nsw i32 %119, 514288
  %125 = and i32 %123, %124
  %126 = xor i32 %122, %125
  %127 = lshr i32 %126, 17
  %128 = add nuw nsw i32 %127, %117
  %129 = icmp samesign ult i32 %128, 20
  tail call void @llvm.assume(i1 %129)
  %130 = add nuw nsw i32 %128, 1
  %131 = zext nneg i32 %130 to i64
  %132 = tail call i64 @llvm.uadd.sat.i64(i64 %101, i64 %131)
  %133 = tail call i64 @llvm.uadd.sat.i64(i64 %132, i64 %102)
  %134 = udiv i64 %133, 9
  %135 = urem i64 %133, 9
  %136 = icmp ne i64 %135, 0
  %137 = zext i1 %136 to i64
  %138 = add nuw nsw i64 %134, %137
  %139 = icmp ult i64 %.sroa.862.0, 39
  %140 = icmp ult i64 %.sroa.1065.0, 19
  %141 = and i1 %139, %140
  br i1 %141, label %143, label %147

142:                                              ; preds = %103
; call core::num::imp::int_log10::panic_for_nonpositive_argument
  tail call void @core::num::imp::int_log10::panic_for_nonpositive_argument(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.1108) #93, !noalias !109401
  unreachable

143:                                              ; preds = %116
  %144 = icmp ult i64 %.sroa.676.0.copyload, 39
  %145 = icmp ult i64 %.sroa.977.0.copyload, 19
  %146 = and i1 %144, %145
  br i1 %146, label %178, label %147

147:                                              ; preds = %178, %143, %116
  %148 = icmp ult i64 %.sroa.1065.0, %.sroa.977.0.copyload
  br i1 %148, label %154, label %149

149:                                              ; preds = %147
  %150 = sub nuw i64 %.sroa.1065.0, %.sroa.977.0.copyload
  %151 = udiv i64 %150, 9
  %152 = add nuw nsw i64 %151, 1
  %153 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.075.0.copyload, i64 %152)
  br label %159

154:                                              ; preds = %147
  %155 = sub nuw i64 %.sroa.977.0.copyload, %.sroa.1065.0
  %156 = udiv i64 %155, 9
  %157 = add nuw nsw i64 %156, 1
  %158 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.060.0, i64 %157)
  br label %159

159:                                              ; preds = %154, %149
  %160 = phi i64 [ %158, %154 ], [ %153, %149 ]
  %161 = phi i64 [ %.sroa.075.0.copyload, %154 ], [ %153, %149 ]
  %162 = phi i64 [ %158, %154 ], [ %.sroa.060.0, %149 ]
  %163 = shl nuw i64 %160, 2
  %164 = icmp ugt i64 %160, 4611686018427387903
  br i1 %164, label %165, label %166, !prof !1735

165:                                              ; preds = %159
  br label %166

166:                                              ; preds = %165, %159
  %167 = phi i64 [ -1, %165 ], [ %163, %159 ]
  %168 = tail call i64 @llvm.umax.i64(i64 %162, i64 %161)
  %169 = tail call i64 @llvm.uadd.sat.i64(i64 %168, i64 1)
  %170 = shl nuw i64 %169, 2
  %171 = icmp ugt i64 %169, 4611686018427387903
  br i1 %171, label %172, label %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit), !prof !1735

172:                                              ; preds = %166
  br label %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit)

<purrdf_xsd::exact::cost::Shape>::add_cost (.exit): ; preds = %166, %172
  %173 = phi i64 [ -1, %172 ], [ %170, %166 ]
  %174 = tail call i64 @llvm.uadd.sat.i64(i64 %160, i64 %169)
  %175 = tail call i64 @llvm.uadd.sat.i64(i64 %167, i64 %173)
  %176 = tail call i64 @llvm.uadd.sat.i64(i64 %49, i64 %174)
  %177 = tail call i64 @llvm.umax.i64(i64 %50, i64 %175)
  br label %182

178:                                              ; preds = %143
  %179 = icmp ult i64 %133, 39
  %180 = icmp ult i64 %52, 19
  %181 = and i1 %180, %179
  br i1 %181, label %182, label %147

182:                                              ; preds = %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit), %178, %98
  %.sroa.075.0 = phi i64 [ %.sroa.075.0.copyload, %98 ], [ %138, %178 ], [ %138, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %.sroa.676.0 = phi i64 [ %.sroa.676.0.copyload, %98 ], [ %133, %178 ], [ %133, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %.sroa.977.0 = phi i64 [ %.sroa.977.0.copyload, %98 ], [ %102, %178 ], [ %102, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %.sroa.13.0 = phi i32 [ %.sroa.13.0.copyload, %98 ], [ -1, %178 ], [ -1, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %183 = phi i64 [ %50, %98 ], [ %50, %178 ], [ %177, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  %184 = phi i64 [ %49, %98 ], [ %49, %178 ], [ %176, %<purrdf_xsd::exact::cost::Shape>::add_cost (.exit) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !109365
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !109365
  %185 = icmp eq ptr %57, %15
  br i1 %185, label %186, label %47

186:                                              ; preds = %182
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !109365
  br label %355

187:                                              ; preds = %96
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !109365
  br i1 %48, label %355, label %446

188:                                              ; preds = %30, %30
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !109405
  br label %189

189:                                              ; preds = %230, %188
  %190 = phi ptr [ %191, %230 ], [ %1, %188 ]
  %191 = getelementptr inbounds nuw i8, ptr %190, i64 80
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109408)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109411)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109414)
  %192 = load i64, ptr %190, align 8, !range !4657, !alias.scope !109417, !noalias !109418, !noundef !1733
  %193 = icmp ne i64 %192, -9223372036854775806
  tail call void @llvm.assume(i1 %193)
  %194 = icmp sgt i64 %192, -1
  %195 = getelementptr inbounds nuw i8, ptr %190, i64 48
  %196 = load i64, ptr %195, align 8, !range !2052, !alias.scope !109417, !noalias !109418
  %197 = icmp eq i64 %196, -1
  %198 = select i1 %194, i1 %197, i1 false
  br i1 %198, label %199, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread)

199:                                              ; preds = %189
  %200 = getelementptr inbounds nuw i8, ptr %190, i64 32
  %201 = load ptr, ptr %200, align 8, !alias.scope !109417, !noalias !109418, !nonnull !1733, !noundef !1733
  %202 = getelementptr inbounds nuw i8, ptr %190, i64 40
  %203 = load i64, ptr %202, align 8, !alias.scope !109417, !noalias !109418, !noundef !1733
  %204 = icmp samesign ult i64 %203, 33
  br i1 %204, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread), label %205

205:                                              ; preds = %199
  %206 = load i256, ptr @anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276, align 1
  %207 = load i256, ptr %201, align 1
  %208 = xor i256 %206, %207
  %209 = getelementptr i8, ptr %201, i64 32
  %210 = load i8, ptr getelementptr (i8, ptr @anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276, i64 32), align 1
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
  %221 = tail call noundef i8 @<purrdf_xsd::datatype::XsdDatatype>::from_local(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %220, i64 noundef %219), !alias.scope !109428, !noalias !109431
  %222 = icmp eq i8 %221, -1
  br i1 %222, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread), label %223

223:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18)
  %224 = getelementptr inbounds nuw i8, ptr %190, i64 16
  %225 = load i64, ptr %224, align 8, !alias.scope !109417, !noalias !109418, !noundef !1733
  %226 = getelementptr inbounds nuw i8, ptr %190, i64 8
  %227 = load ptr, ptr %226, align 8, !alias.scope !109417, !noalias !109418, !nonnull !1733, !noundef !1733
; call <purrdf_xsd::exact::cost::Shape>::of_lexical
  call void @<purrdf_xsd::exact::cost::Shape>::of_lexical(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %6, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %227, i64 noundef %225, i8 noundef %221), !noalias !109405
  %228 = load i64, ptr %6, align 8, !range !1732, !noalias !109405
  %229 = trunc nuw i64 %228 to i1
  br i1 %229, label %232, label %230

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread): ; preds = %199, %205, %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18), %189
  store i64 0, ptr %6, align 8, !noalias !109405
  br label %230

230:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit18.thread), %223
  %231 = icmp eq ptr %191, %15
  br i1 %231, label %.loopexit90, label %189

232:                                              ; preds = %223
  %233 = tail call noundef dereferenceable_or_null(128) ptr @malloc(i64 noundef range(i64 1, 0) 128) #92, !noalias !109432
  %234 = icmp eq ptr %233, null
  br i1 %234, label %__rustc::__rust_alloc (.exit.thread), label %235

235:                                              ; preds = %232
  %236 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %237 = load i64, ptr %236, align 8, !noalias !109432, !noundef !1733
  %238 = tail call i64 @llvm.uadd.sat.i64(i64 %237, i64 1)
  store i64 %238, ptr %236, align 8, !noalias !109432
  %239 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %240 = load i64, ptr %239, align 8, !noalias !109432, !noundef !1733
  %241 = tail call i64 @llvm.uadd.sat.i64(i64 %240, i64 128)
  store i64 %241, ptr %239, align 8, !noalias !109432
  %242 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %243 = load i64, ptr %242, align 8, !noalias !109432, !noundef !1733
  %244 = tail call i64 @llvm.sadd.sat.i64(i64 %243, i64 128)
  store i64 %244, ptr %242, align 8, !noalias !109432
  %245 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %246 = load i64, ptr %245, align 8, !noalias !109432, !noundef !1733
  %247 = icmp sgt i64 %244, %246
  br i1 %247, label %248, label %.preheader325

248:                                              ; preds = %235
  store i64 %244, ptr %245, align 8, !noalias !109432
  br label %.preheader325

.preheader325:                                    ; preds = %248, %235
  br label %249

249:                                              ; preds = %.preheader325, %252
  %250 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !109432
  %251 = icmp slt i64 %250, 0
  br i1 %251, label %252, label %__rustc::__rust_alloc (.exit)

252:                                              ; preds = %249
  %253 = add nsw i64 %250, 1
  %254 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %250, i64 %253 acq_rel acquire, align 8, !noalias !109432
  %255 = extractvalue { i64, i1 } %254, 1
  br i1 %255, label %256, label %249

256:                                              ; preds = %252
  %257 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !109432
  %258 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 128 monotonic, align 8, !noalias !109432
  %259 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 128 monotonic, align 8, !noalias !109432
  %260 = tail call i64 @llvm.sadd.sat.i64(i64 %259, i64 128)
  %261 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !109432
  br label %262

262:                                              ; preds = %265, %256
  %263 = phi i64 [ %261, %256 ], [ %268, %265 ]
  %264 = icmp sgt i64 %260, %263
  br i1 %264, label %265, label %269

265:                                              ; preds = %262
  %266 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %263, i64 %260 monotonic monotonic, align 8, !noalias !109432
  %267 = extractvalue { i64, i1 } %266, 1
  %268 = extractvalue { i64, i1 } %266, 0
  br i1 %267, label %269, label %262

269:                                              ; preds = %265, %262
  %270 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !109432
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %232
; call alloc::raw_vec::handle_error
  tail call void @alloc::raw_vec::handle_error(i64 noundef 8, i64 128) #93, !noalias !109405
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %249, %269
  %271 = getelementptr inbounds nuw i8, ptr %6, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %233, ptr noundef nonnull align 8 dereferenceable(32) %271, i64 32, i1 false), !noalias !109405
  store i64 4, ptr %7, align 8, !noalias !109405
  %272 = getelementptr inbounds nuw i8, ptr %7, i64 8
  store ptr %233, ptr %272, align 8, !noalias !109405
  %273 = getelementptr inbounds nuw i8, ptr %7, i64 16
  store i64 1, ptr %273, align 8, !noalias !109405
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109435)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !109438)
  call void @llvm.lifetime.start.p0(ptr nonnull %5), !noalias !109441
  %274 = getelementptr inbounds nuw i8, ptr %5, i64 8
  %275 = icmp eq ptr %191, %15
  br i1 %275, label %.loopexit, label %.preheader88

.preheader88:                                     ; preds = %__rustc::__rust_alloc (.exit), %330
  %276 = phi ptr [ %331, %330 ], [ %233, %__rustc::__rust_alloc (.exit) ]
  %277 = phi i64 [ %333, %330 ], [ 1, %__rustc::__rust_alloc (.exit) ]
  %278 = phi ptr [ %281, %330 ], [ %191, %__rustc::__rust_alloc (.exit) ]
  br label %279

279:                                              ; preds = %321, %.preheader88
  %280 = phi ptr [ %281, %321 ], [ %278, %.preheader88 ]
  %281 = getelementptr inbounds nuw i8, ptr %280, i64 80
  %282 = load i64, ptr %280, align 8, !range !4657, !alias.scope !109442, !noalias !109449, !noundef !1733
  %283 = icmp ne i64 %282, -9223372036854775806
  tail call void @llvm.assume(i1 %283)
  %284 = icmp sgt i64 %282, -1
  %285 = getelementptr inbounds nuw i8, ptr %280, i64 48
  %286 = load i64, ptr %285, align 8, !range !2052, !alias.scope !109442, !noalias !109449
  %287 = icmp eq i64 %286, -1
  %288 = select i1 %284, i1 %287, i1 false
  br i1 %288, label %289, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread)

289:                                              ; preds = %279
  %290 = getelementptr inbounds nuw i8, ptr %280, i64 8
  %291 = load ptr, ptr %290, align 8, !alias.scope !109442, !noalias !109449, !nonnull !1733, !noundef !1733
  %292 = getelementptr inbounds nuw i8, ptr %280, i64 16
  %293 = load i64, ptr %292, align 8, !alias.scope !109442, !noalias !109449, !noundef !1733
  %294 = getelementptr inbounds nuw i8, ptr %280, i64 32
  %295 = load ptr, ptr %294, align 8, !alias.scope !109442, !noalias !109449, !nonnull !1733, !noundef !1733
  %296 = getelementptr inbounds nuw i8, ptr %280, i64 40
  %297 = load i64, ptr %296, align 8, !alias.scope !109442, !noalias !109449, !noundef !1733
  %298 = icmp samesign ult i64 %297, 33
  br i1 %298, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread), label %299

299:                                              ; preds = %289
  %300 = load i256, ptr @anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276, align 1
  %301 = load i256, ptr %295, align 1
  %302 = xor i256 %300, %301
  %303 = getelementptr i8, ptr %295, i64 32
  %304 = load i8, ptr getelementptr (i8, ptr @anon.8c4c8f20c49bb342a4ce83ac1e804353.131.llvm.9305710216504555276, i64 32), align 1
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
  %315 = tail call noundef i8 @<purrdf_xsd::datatype::XsdDatatype>::from_local(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %314, i64 noundef %313), !alias.scope !109459, !noalias !109405
  %316 = icmp eq i8 %315, -1
  br i1 %316, label %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread), label %317

317:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19)
; invoke <purrdf_xsd::exact::cost::Shape>::of_lexical
  invoke void @<purrdf_xsd::exact::cost::Shape>::of_lexical(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(none) dereferenceable(40) %5, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %291, i64 noundef %293, i8 noundef %315)
          to label %318 unwind label %335, !noalias !109405

318:                                              ; preds = %317
  %319 = load i64, ptr %5, align 8, !range !1732, !noalias !109462
  %320 = trunc nuw i64 %319 to i1
  br i1 %320, label %323, label %321

<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread): ; preds = %289, %299, %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19), %279
  store i64 0, ptr %5, align 8, !noalias !109462
  br label %321

321:                                              ; preds = %<purrdf_xsd::datatype::XsdDatatype>::from_iri (.exit19.thread), %318
  %322 = icmp eq ptr %281, %15
  br i1 %322, label %.loopexit, label %279

323:                                              ; preds = %318
  %324 = icmp samesign ult i64 %277, 288230376151711744
  tail call void @llvm.assume(i1 %324)
  %325 = load i64, ptr %7, align 8, !range !1828, !alias.scope !109463, !noalias !109405, !noundef !1733
  %326 = icmp eq i64 %277, %325
  br i1 %326, label %327, label %330

327:                                              ; preds = %323
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.1794586459888082020)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %7, i64 noundef %277, i64 noundef 1, i64 noundef 8, i64 noundef 32)
          to label %328 unwind label %337, !noalias !109405

328:                                              ; preds = %327
  %329 = load ptr, ptr %272, align 8, !alias.scope !109463, !noalias !109405
  br label %330

330:                                              ; preds = %328, %323
  %331 = phi ptr [ %329, %328 ], [ %276, %323 ]
  %332 = getelementptr inbounds nuw [32 x i8], ptr %331, i64 %277
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %332, ptr noundef nonnull align 8 dereferenceable(32) %274, i64 32, i1 false), !noalias !109462
  %333 = add nuw nsw i64 %277, 1
  store i64 %333, ptr %273, align 8, !alias.scope !109463, !noalias !109405
  %334 = icmp eq ptr %281, %15
  br i1 %334, label %.loopexit, label %.preheader88

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
  %341 = load i64, ptr %7, align 8, !noalias !109405
  %342 = icmp eq i64 %341, 0
  br i1 %342, label %349, label %343

343:                                              ; preds = %339
  %344 = load ptr, ptr %272, align 8, !noalias !109405, !nonnull !1733, !noundef !1733
  %345 = shl nuw i64 %341, 5
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %344, i64 noundef %345, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !109405
  br label %349

.loopexit:                                        ; preds = %330, %321, %__rustc::__rust_alloc (.exit)
  %346 = phi i64 [ %277, %321 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %333, %330 ]
  %347 = phi ptr [ %276, %321 ], [ %233, %__rustc::__rust_alloc (.exit) ], [ %331, %330 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %5), !noalias !109441
  %348 = load i64, ptr %7, align 8
  br label %.loopexit90

349:                                              ; preds = %454, %451, %393, %343, %339
  %350 = phi { ptr, i32 } [ %340, %339 ], [ %394, %393 ], [ %340, %343 ], [ %452, %454 ], [ %452, %451 ]
  resume { ptr, i32 } %350

.loopexit90:                                      ; preds = %230, %.loopexit
  %351 = phi i64 [ %348, %.loopexit ], [ 0, %230 ]
  %352 = phi ptr [ %347, %.loopexit ], [ inttoptr (i64 8 to ptr), %230 ]
  %353 = phi i64 [ %346, %.loopexit ], [ 0, %230 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !109405
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
; invoke purrdf_xsd::exact::cost::compare_chain
  %354 = invoke { i64, i64 } @purrdf_xsd::exact::cost::compare_chain(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %352, i64 noundef %353, i64 noundef 1)
          to label %456 unwind label %451

355:                                              ; preds = %187, %186
  %.sroa.12.0 = phi i32 [ %.sroa.1471.0, %187 ], [ %.sroa.15.0.copyload, %186 ]
  %.sroa.11.0 = phi i32 [ %.sroa.1268.0, %187 ], [ %.sroa.13.0, %186 ]
  %.sroa.1031.0 = phi i64 [ %.sroa.1065.0, %187 ], [ %.sroa.977.0, %186 ]
  %.sroa.926.0 = phi i64 [ %.sroa.862.0, %187 ], [ %.sroa.676.0, %186 ]
  %.sroa.023.0 = phi i64 [ %.sroa.060.0, %187 ], [ %.sroa.075.0, %186 ]
  %356 = phi i64 [ %49, %187 ], [ %184, %186 ]
  %357 = phi i64 [ %50, %187 ], [ %183, %186 ]
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
  br i1 %368, label %446, label %369

369:                                              ; preds = %361, %355
  br i1 %43, label %374, label %370

370:                                              ; preds = %369
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !109464
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !109464
  %371 = zext nneg i64 %2 to i128
  %372 = getelementptr inbounds nuw i8, ptr %11, i64 16
  store i128 %371, ptr %372, align 16, !noalias !109464
  %373 = getelementptr inbounds nuw i8, ptr %11, i64 1
  store i8 0, ptr %373, align 1, !noalias !109464
  store i8 0, ptr %11, align 16, !noalias !109464
; invoke purrdf_xsd::numeric::exact_path::shape_of
  invoke fastcc void @purrdf_xsd::numeric::exact_path::shape_of (.llvm.9305710216504555276)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(40) %12, ptr noalias nofree noundef nonnull readonly align 16 captures(address, read_provenance) dereferenceable(80) %11)
          to label %<purrdf_xsd::exact::cost::Shape>::of_value (.exit) unwind label %393

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
  br i1 %385, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), label %386, !prof !1735

386:                                              ; preds = %382
  %387 = icmp slt i32 %.sroa.11.0, 0
  %388 = tail call i64 @llvm.uadd.sat.i64(i64 %383, i64 1)
  %389 = select i1 %387, i64 %388, i64 %383
  %390 = extractvalue { i64, i1 } %384, 0
  %391 = tail call i64 @llvm.uadd.sat.i64(i64 %390, i64 %389)
  %392 = tail call i64 @llvm.uadd.sat.i64(i64 %391, i64 1)
  br label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)

393:                                              ; preds = %370, %442
  %394 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
  call fastcc void @core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>(ptr noalias nofree noundef align 16 dereferenceable(80) %11) #89, !noalias !109464
  br label %349

<purrdf_xsd::exact::cost::Shape>::of_value (.exit): ; preds = %370
  %395 = load i64, ptr %12, align 8, !range !1732, !noalias !109464, !noundef !1733
  %396 = trunc nuw i64 %395 to i1
  br i1 %396, label %397, label %442, !prof !1946

397:                                              ; preds = %<purrdf_xsd::exact::cost::Shape>::of_value (.exit)
  %398 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %.sroa.926.0..sroa_idx29 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %.sroa.1031.0..sroa_idx34 = getelementptr inbounds nuw i8, ptr %10, i64 16
  %.sroa.11.0..sroa_idx38 = getelementptr inbounds nuw i8, ptr %10, i64 24
  %.sroa.12.0..sroa_idx42 = getelementptr inbounds nuw i8, ptr %10, i64 28
  %399 = load <4 x i64>, ptr %398, align 8, !noalias !109464
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !109464
; call core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>
  call fastcc void @core::ptr::drop_glue::<purrdf_xsd::value::XsdValue>(ptr noalias nofree noundef align 16 dereferenceable(80) %11), !noalias !109464
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !109464
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !109464
  store i64 %.sroa.023.0, ptr %10, align 8
  store i64 %.sroa.926.0, ptr %.sroa.926.0..sroa_idx29, align 8
  store i64 %.sroa.1031.0, ptr %.sroa.1031.0..sroa_idx34, align 8
  store i32 %.sroa.11.0, ptr %.sroa.11.0..sroa_idx38, align 8
  store i32 %.sroa.12.0, ptr %.sroa.12.0..sroa_idx42, align 4
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !109464
  store <4 x i64> %399, ptr %9, align 8, !noalias !109464
; call purrdf_xsd::exact::cost::decimal_div
  %400 = call { i64, i64 } @purrdf_xsd::exact::cost::decimal_div(ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(32) %10, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(32) %9, i64 %3), !alias.scope !109468, !noalias !109464
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !109464
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !109464
  %401 = extractvalue { i64, i64 } %400, 0
  %402 = extractvalue { i64, i64 } %400, 1
  %403 = trunc i64 %3 to i1
  br i1 %403, label %404, label %412

404:                                              ; preds = %397
  %405 = extractelement <4 x i64> %399, i64 1
  %406 = shl nuw i64 %405, 2
  %407 = icmp ugt i64 %405, 4611686018427387903
  br i1 %407, label %408, label %409, !prof !1735

408:                                              ; preds = %404
  br label %409

409:                                              ; preds = %408, %404
  %410 = phi i64 [ -1, %408 ], [ %406, %404 ]
  %411 = tail call i64 @llvm.uadd.sat.i64(i64 %.sroa.1031.0, i64 %410)
  br label %412

412:                                              ; preds = %409, %397
  %413 = phi i64 [ %411, %409 ], [ %13, %397 ]
  %414 = tail call i64 @llvm.usub.sat.i64(i64 %.sroa.926.0, i64 %.sroa.1031.0)
  %415 = extractelement <4 x i64> %399, i64 2
  %416 = tail call i64 @llvm.uadd.sat.i64(i64 %414, i64 %415)
  %417 = tail call i64 @llvm.uadd.sat.i64(i64 %416, i64 1)
  %418 = tail call i64 @llvm.uadd.sat.i64(i64 %417, i64 %413)
  %419 = udiv i64 %418, 9
  %420 = urem i64 %418, 9
  %421 = icmp ne i64 %420, 0
  %422 = zext i1 %421 to i64
  %423 = add nuw nsw i64 %419, %422
  %424 = icmp ult i64 %413, %418
  br i1 %424, label %425, label %427

425:                                              ; preds = %412
  %426 = icmp eq i64 %413, 0
  br i1 %426, label %431, label %429

427:                                              ; preds = %412
  %428 = tail call i64 @llvm.uadd.sat.i64(i64 %413, i64 2)
  br label %431

429:                                              ; preds = %425
  %430 = tail call i64 @llvm.uadd.sat.i64(i64 %418, i64 1)
  br label %431

431:                                              ; preds = %429, %427, %425
  %432 = phi i64 [ %428, %427 ], [ %430, %429 ], [ %418, %425 ]
  %433 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %423, i64 9)
  %434 = extractvalue { i64, i1 } %433, 1
  br i1 %434, label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), label %435, !prof !1735

435:                                              ; preds = %431
  %436 = tail call i64 @llvm.uadd.sat.i64(i64 %432, i64 1)
  %437 = extractvalue { i64, i1 } %433, 0
  %438 = tail call i64 @llvm.uadd.sat.i64(i64 %437, i64 %436)
  %439 = tail call i64 @llvm.uadd.sat.i64(i64 %438, i64 1)
  %440 = tail call i64 @llvm.uadd.sat.i64(i64 %401, i64 %439)
  %441 = tail call i64 @llvm.umax.i64(i64 %402, i64 %438)
  br label %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit)

442:                                              ; preds = %<purrdf_xsd::exact::cost::Shape>::of_value (.exit)
; invoke core::option::expect_failed
  invoke void @core::option::expect_failed(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.68dd637f94a7f528fe69f6876e3d956b.550, i64 noundef 22, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.68dd637f94a7f528fe69f6876e3d956b.551) #93
          to label %443 unwind label %393, !noalias !109464

443:                                              ; preds = %442
  unreachable

<purrdf_xsd::exact::cost::Shape>::render_cost (.exit): ; preds = %435, %431, %386, %382
  %.pn87 = phi i64 [ -1, %382 ], [ %392, %386 ], [ %440, %435 ], [ -1, %431 ]
  %.pn85 = phi i64 [ -1, %382 ], [ %391, %386 ], [ %441, %435 ], [ -1, %431 ]
  %444 = tail call i64 @llvm.uadd.sat.i64(i64 %356, i64 %.pn87)
  %445 = tail call i64 @llvm.umax.i64(i64 %357, i64 %.pn85)
  br label %446

446:                                              ; preds = %46, %187, %361, %__rustc::__rust_dealloc (.exit), %456, %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit), %38, %30, %._crit_edge
  %447 = phi i64 [ 0, %38 ], [ %457, %__rustc::__rust_dealloc (.exit) ], [ %444, %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit) ], [ 0, %._crit_edge ], [ 0, %30 ], [ %457, %456 ], [ %49, %187 ], [ 0, %46 ], [ %356, %361 ]
  %448 = phi i64 [ 0, %38 ], [ %458, %__rustc::__rust_dealloc (.exit) ], [ %445, %<purrdf_xsd::exact::cost::Shape>::render_cost (.exit) ], [ 0, %._crit_edge ], [ 0, %30 ], [ %458, %456 ], [ %50, %187 ], [ 0, %46 ], [ %357, %361 ]
  %449 = insertvalue { i64, i64 } poison, i64 %447, 0
  %450 = insertvalue { i64, i64 } %449, i64 %448, 1
  ret { i64, i64 } %450

451:                                              ; preds = %.loopexit90
  %452 = landingpad { ptr, i32 }
          cleanup
  %453 = icmp eq i64 %351, 0
  br i1 %453, label %349, label %454

454:                                              ; preds = %451
  %455 = shl nuw i64 %351, 5
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %352) ]
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %352, i64 noundef %455, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %349

456:                                              ; preds = %.loopexit90
  %457 = extractvalue { i64, i64 } %354, 0
  %458 = extractvalue { i64, i64 } %354, 1
  %459 = icmp eq i64 %351, 0
  br i1 %459, label %446, label %460

460:                                              ; preds = %456
  %461 = shl nuw i64 %351, 5
  %462 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %463 = load i64, ptr %462, align 8, !noundef !1733
  %464 = tail call i64 @llvm.umin.i64(i64 %461, i64 9223372036854775807)
  %465 = tail call i64 @llvm.ssub.sat.i64(i64 %463, i64 %464)
  store i64 %465, ptr %462, align 8
  %466 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %467 = load i64, ptr %466, align 8, !noundef !1733
  %468 = icmp slt i64 %465, %467
  br i1 %468, label %469, label %.preheader310

469:                                              ; preds = %460
  store i64 %465, ptr %466, align 8
  br label %.preheader310

.preheader310:                                    ; preds = %469, %460
  br label %470

470:                                              ; preds = %.preheader310, %473
  %471 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %472 = icmp slt i64 %471, 0
  br i1 %472, label %473, label %__rustc::__rust_dealloc (.exit)

473:                                              ; preds = %470
  %474 = add nsw i64 %471, 1
  %475 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %471, i64 %474 acq_rel acquire, align 8
  %476 = extractvalue { i64, i1 } %475, 1
  br i1 %476, label %477, label %470

477:                                              ; preds = %473
  %478 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %464 monotonic, align 8
  %479 = tail call i64 @llvm.ssub.sat.i64(i64 %478, i64 %464)
  %480 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %481

481:                                              ; preds = %484, %477
  %482 = phi i64 [ %480, %477 ], [ %487, %484 ]
  %483 = icmp slt i64 %479, %482
  br i1 %483, label %484, label %488

484:                                              ; preds = %481
  %485 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %482, i64 %479 monotonic monotonic, align 8
  %486 = extractvalue { i64, i1 } %485, 1
  %487 = extractvalue { i64, i1 } %485, 0
  br i1 %486, label %488, label %481

488:                                              ; preds = %484, %481
  %489 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %470, %488
  tail call void @free(ptr noundef nonnull %352) #92
  br label %446
}

define void @purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(16) %3, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %4, ptr noalias nofree noundef align 16 dereferenceable(1248) %5) unnamed_addr #8 personality ptr @rust_eh_personality !guid !23019 {
  %7 = alloca [8 x i8], align 8
  %8 = alloca [8 x i8], align 8
  %9 = alloca [16 x i8], align 8
  %10 = alloca [40 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [48 x i8], align 8
  %13 = alloca [16 x i8], align 8
  %14 = alloca [24 x i8], align 8
  %15 = alloca [24 x i8], align 8
  %16 = alloca [32 x i8], align 8
  %17 = alloca [96 x i8], align 16
  %18 = alloca [24 x i8], align 8
  %19 = alloca [32 x i8], align 8
  %20 = alloca [224 x i8], align 8
  %21 = alloca [24 x i8], align 8
  %22 = alloca [24 x i8], align 8
  %23 = alloca [96 x i8], align 16
  %24 = alloca [24 x i8], align 8
  %25 = alloca [24 x i8], align 8
  %26 = alloca [96 x i8], align 16
  %27 = alloca [24 x i8], align 8
  %28 = alloca [24 x i8], align 8
  %29 = alloca [72 x i8], align 8
  %30 = alloca [32 x i8], align 8
  %31 = alloca [104 x i8], align 8
  %32 = alloca [96 x i8], align 8
  %33 = alloca [96 x i8], align 8
  %34 = alloca [24 x i8], align 8
  %35 = alloca [24 x i8], align 8
  %36 = alloca [96 x i8], align 16
  %37 = alloca [24 x i8], align 8
  %38 = alloca [40 x i8], align 8
  %39 = alloca [32 x i8], align 8
  %40 = alloca [40 x i8], align 8
  %41 = alloca [24 x i8], align 8
  %42 = alloca [24 x i8], align 8
  %43 = alloca [96 x i8], align 16
  %44 = alloca [40 x i8], align 8
  %45 = alloca [24 x i8], align 8
  %46 = alloca [200 x i8], align 8
  %47 = alloca [24 x i8], align 8
  %48 = alloca [48 x i8], align 16
  %49 = alloca [24 x i8], align 8
  %50 = alloca [24 x i8], align 8
  %51 = alloca [208 x i8], align 8
  %52 = alloca [24 x i8], align 8
  %53 = alloca [48 x i8], align 16
  %54 = alloca [24 x i8], align 8
  %55 = alloca [24 x i8], align 8
  %56 = alloca [248 x i8], align 8
  %57 = alloca [240 x i8], align 8
  %58 = alloca [32 x i8], align 8
  %59 = alloca [48 x i8], align 8
  %60 = alloca [32 x i8], align 8
  %61 = alloca [256 x i8], align 16
  %62 = alloca [208 x i8], align 16
  %63 = alloca [8 x i8], align 8
  %64 = alloca [8 x i8], align 8
  %65 = alloca [24 x i8], align 8
  %66 = alloca [216 x i8], align 8
  %67 = alloca [8 x i8], align 8
  %68 = alloca [8 x i8], align 8
  %69 = alloca [8 x i8], align 8
  %70 = alloca [56 x i8], align 8
  %71 = alloca [200 x i8], align 8
  %72 = alloca [112 x i8], align 16
  %73 = alloca [72 x i8], align 8
  %74 = alloca [104 x i8], align 8
  %75 = alloca [104 x i8], align 8
  %76 = alloca [32 x i8], align 8
  %77 = alloca [32 x i8], align 8
  %78 = alloca [32 x i8], align 8
  %79 = alloca [104 x i8], align 8
  %80 = alloca [96 x i8], align 8
  %81 = alloca [56 x i8], align 8
  %82 = alloca [104 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %82)
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %82, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %76)
  call void @llvm.lifetime.start.p0(ptr nonnull %75)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %72, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %85 unwind label %83, !inline_history !13429

83:                                               ; preds = %1676, %1664, %1660, %171, %6
  %84 = landingpad { ptr, i32 }
          cleanup
  br label %1799

85:                                               ; preds = %6
  %86 = load i64, ptr %72, align 16, !range !1739, !noundef !1740
  %87 = trunc nuw i64 %86 to i1
  br i1 %87, label %88, label %171

88:                                               ; preds = %85
  %89 = getelementptr inbounds nuw i8, ptr %72, i64 16
  %90 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %90, ptr noundef nonnull align 16 dereferenceable(96) %89, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %75)
  call void @llvm.lifetime.end.p0(ptr nonnull %76)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23020)
  %91 = getelementptr inbounds nuw i8, ptr %82, i64 72
  %92 = load i64, ptr %91, align 8, !range !1778, !alias.scope !23023, !noundef !1740
  %93 = icmp ugt i64 %92, 5
  br i1 %93, label %94, label %128

94:                                               ; preds = %88
  %95 = getelementptr inbounds nuw i8, ptr %82, i64 80
  %96 = load ptr, ptr %95, align 8, !alias.scope !23020, !nonnull !1740, !noundef !1740
  %97 = mul i64 %92, 3
  %98 = add i64 %97, -3
  %99 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %100 = load i64, ptr %99, align 8, !noalias !23026, !noundef !1740
  %101 = tail call i64 @llvm.umin.i64(i64 %98, i64 9223372036854775807)
  %102 = tail call i64 @llvm.ssub.sat.i64(i64 %100, i64 %101)
  store i64 %102, ptr %99, align 8, !noalias !23026
  %103 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %104 = load i64, ptr %103, align 8, !noalias !23026, !noundef !1740
  %105 = icmp slt i64 %102, %104
  br i1 %105, label %106, label %.preheader470

106:                                              ; preds = %94
  store i64 %102, ptr %103, align 8, !noalias !23026
  br label %.preheader470

.preheader470:                                    ; preds = %106, %94
  br label %107

107:                                              ; preds = %.preheader470, %110
  %108 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23026
  %109 = icmp slt i64 %108, 0
  br i1 %109, label %110, label %__rustc::__rust_dealloc (.exit)

110:                                              ; preds = %107
  %111 = add nsw i64 %108, 1
  %112 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %108, i64 %111 acq_rel acquire, align 8, !noalias !23026
  %113 = extractvalue { i64, i1 } %112, 1
  br i1 %113, label %114, label %107

114:                                              ; preds = %110
  %115 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %101 monotonic, align 8, !noalias !23026
  %116 = tail call i64 @llvm.ssub.sat.i64(i64 %115, i64 %101)
  %117 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23026
  br label %118

118:                                              ; preds = %121, %114
  %119 = phi i64 [ %117, %114 ], [ %124, %121 ]
  %120 = icmp slt i64 %116, %119
  br i1 %120, label %121, label %125

121:                                              ; preds = %118
  %122 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %119, i64 %116 monotonic monotonic, align 8, !noalias !23026
  %123 = extractvalue { i64, i1 } %122, 1
  %124 = extractvalue { i64, i1 } %122, 0
  br i1 %123, label %125, label %118

125:                                              ; preds = %121, %118
  %126 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23026
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %107, %125
  %127 = icmp ne i64 %98, 0
  tail call void @llvm.assume(i1 %127), !noalias !23026
  tail call void @free(ptr noundef nonnull %96) #92, !noalias !23026
  br label %128

128:                                              ; preds = %__rustc::__rust_dealloc (.exit), %88
  %129 = load i64, ptr %82, align 8, !range !2059, !alias.scope !23020, !noundef !1740
  %130 = icmp sgt i64 %129, 0
  br i1 %130, label %131, label %163

131:                                              ; preds = %128
  %132 = getelementptr inbounds nuw i8, ptr %82, i64 8
  %133 = load ptr, ptr %132, align 8, !alias.scope !23020, !nonnull !1740, !noundef !1740
  %134 = mul nuw i64 %129, 3
  %135 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %136 = load i64, ptr %135, align 8, !noalias !23020, !noundef !1740
  %137 = tail call i64 @llvm.umin.i64(i64 %134, i64 9223372036854775807)
  %138 = tail call i64 @llvm.ssub.sat.i64(i64 %136, i64 %137)
  store i64 %138, ptr %135, align 8, !noalias !23020
  %139 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %140 = load i64, ptr %139, align 8, !noalias !23020, !noundef !1740
  %141 = icmp slt i64 %138, %140
  br i1 %141, label %142, label %.preheader469

142:                                              ; preds = %131
  store i64 %138, ptr %139, align 8, !noalias !23020
  br label %.preheader469

.preheader469:                                    ; preds = %142, %131
  br label %143

143:                                              ; preds = %.preheader469, %146
  %144 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23020
  %145 = icmp slt i64 %144, 0
  br i1 %145, label %146, label %__rustc::__rust_dealloc (.exit79)

146:                                              ; preds = %143
  %147 = add nsw i64 %144, 1
  %148 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %144, i64 %147 acq_rel acquire, align 8, !noalias !23020
  %149 = extractvalue { i64, i1 } %148, 1
  br i1 %149, label %150, label %143

150:                                              ; preds = %146
  %151 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %137 monotonic, align 8, !noalias !23020
  %152 = tail call i64 @llvm.ssub.sat.i64(i64 %151, i64 %137)
  %153 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23020
  br label %154

154:                                              ; preds = %157, %150
  %155 = phi i64 [ %153, %150 ], [ %160, %157 ]
  %156 = icmp slt i64 %152, %155
  br i1 %156, label %157, label %161

157:                                              ; preds = %154
  %158 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %155, i64 %152 monotonic monotonic, align 8, !noalias !23020
  %159 = extractvalue { i64, i1 } %158, 1
  %160 = extractvalue { i64, i1 } %158, 0
  br i1 %159, label %161, label %154

161:                                              ; preds = %157, %154
  %162 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23020
  br label %__rustc::__rust_dealloc (.exit79)

__rustc::__rust_dealloc (.exit79): ; preds = %143, %161
  tail call void @free(ptr noundef nonnull %133) #92, !noalias !23020
  br label %163

163:                                              ; preds = %__rustc::__rust_dealloc (.exit79), %128
  %164 = getelementptr inbounds nuw i8, ptr %82, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23029)
  %165 = load ptr, ptr %164, align 8, !alias.scope !23032, !noundef !1740
  %166 = icmp eq ptr %165, null
  br i1 %166, label %1649, label %167

167:                                              ; preds = %163
  %168 = atomicrmw sub ptr %165, i64 1 release, align 8, !noalias !23033
  %169 = icmp eq i64 %168, 1
  br i1 %169, label %170, label %1649

170:                                              ; preds = %167
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %164) #91
  br label %1649

171:                                              ; preds = %85
  %172 = getelementptr inbounds nuw i8, ptr %72, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %75, ptr noundef nonnull align 8 dereferenceable(96) %172, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %76, ptr noalias nofree noundef align 8 dereferenceable(104) %82, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %75)
          to label %173 unwind label %83

173:                                              ; preds = %171
  %174 = load i64, ptr %76, align 8, !range !2059, !noundef !1740
  %175 = icmp eq i64 %174, -1
  br i1 %175, label %1609, label %176

176:                                              ; preds = %173
  call void @llvm.lifetime.start.p0(ptr nonnull %77)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %77, ptr noundef nonnull align 8 dereferenceable(32) %76, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %75)
  call void @llvm.lifetime.end.p0(ptr nonnull %76)
  call void @llvm.lifetime.start.p0(ptr nonnull %74)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %74, ptr noundef nonnull align 8 dereferenceable(104) %82, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23038)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23041)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23043)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23045)
  call void @llvm.lifetime.start.p0(ptr nonnull %28)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
  %177 = invoke fastcc noundef zeroext i1 @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop(ptr noundef nonnull align 16 dereferenceable(1248) %5, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %4)
          to label %184 unwind label %179, !noalias !23047

178:                                              ; preds = %224
  br i1 %226, label %1604, label %1527

179:                                              ; preds = %1447, %205, %176
  %180 = phi i8 [ 1, %176 ], [ 1, %205 ], [ %1366, %1447 ]
  %181 = landingpad { ptr, i32 }
          cleanup
  br label %1604

182:                                              ; preds = %1369
  %183 = landingpad { ptr, i32 }
          cleanup
  br label %1527

184:                                              ; preds = %176
  %185 = getelementptr inbounds nuw i8, ptr %5, i64 472
  %186 = load i8, ptr %185, align 8, !range !3730
  %187 = icmp eq i8 %186, 2
  %188 = select i1 %177, i1 %187, i1 false
  br i1 %188, label %189, label %205

189:                                              ; preds = %184
  %190 = getelementptr inbounds nuw i8, ptr %5, i64 616
  %191 = load ptr, ptr %190, align 8, !noalias !23050, !noundef !1740
  %192 = icmp eq ptr %191, null
  br i1 %192, label %210, label %193

193:                                              ; preds = %189
  %194 = getelementptr inbounds nuw i8, ptr %191, i64 24
  %195 = load i64, ptr %194, align 8, !noalias !23051
  %196 = getelementptr inbounds nuw i8, ptr %191, i64 48
  %197 = icmp ult i64 %195, -2
  br i1 %197, label %205, label %198

198:                                              ; preds = %193
  %199 = getelementptr inbounds nuw i8, ptr %191, i64 32
  %200 = load i64, ptr %199, align 8, !noalias !23051
  %201 = icmp ult i64 %200, -2
  br i1 %201, label %205, label %202

202:                                              ; preds = %198
  %203 = load i64, ptr %196, align 8, !noalias !23051
  %204 = icmp ugt i64 %203, -3
  br i1 %204, label %210, label %205

205:                                              ; preds = %210, %202, %198, %193, %184
  %206 = phi i1 [ false, %184 ], [ false, %202 ], [ %219, %210 ], [ false, %193 ], [ false, %198 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %71), !noalias !23057
  %207 = getelementptr inbounds nuw i8, ptr %77, i64 16
  %208 = load i64, ptr %207, align 8, !alias.scope !23043, !noalias !23059, !noundef !1740
  %209 = icmp ult i64 %208, 230584300921369396
  tail call void @llvm.assume(i1 %209)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %71, ptr noundef nonnull align 16 dereferenceable(1248) %5, i1 noundef zeroext %206, i64 noundef %208)
          to label %220 unwind label %179

210:                                              ; preds = %202, %189
  %211 = getelementptr inbounds nuw i8, ptr %5, i64 1234
  %212 = load i8, ptr %211, align 2, !range !1747, !noundef !1740
  %213 = trunc nuw i8 %212 to i1
  %214 = getelementptr inbounds nuw i8, ptr %77, i64 16
  %215 = load i64, ptr %214, align 8, !alias.scope !23043, !noalias !23059, !noundef !1740
  %216 = icmp ult i64 %215, 230584300921369396
  tail call void @llvm.assume(i1 %216)
  %217 = icmp samesign ugt i64 %215, 1024
  %218 = xor i1 %213, true
  %219 = select i1 %218, i1 %217, i1 false
  br label %205

220:                                              ; preds = %205
  call void @llvm.lifetime.start.p0(ptr nonnull %70), !noalias !23057
  %221 = getelementptr inbounds nuw i8, ptr %77, i64 24
  %222 = load ptr, ptr %221, align 8, !alias.scope !23043, !noalias !23059, !nonnull !1740, !noundef !1740
  %223 = getelementptr inbounds nuw i8, ptr %222, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %70, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %223)
          to label %230 unwind label %228, !noalias !23050

224:                                              ; preds = %1602, %1601, %1595, %295, %291, %286, %239, %228
  %225 = phi i8 [ 1, %1602 ], [ %1366, %239 ], [ 1, %228 ], [ 1, %286 ], [ %341, %291 ], [ %1597, %1601 ], [ %1597, %1595 ], [ %652, %295 ]
  %226 = phi i1 [ true, %1602 ], [ true, %239 ], [ true, %228 ], [ true, %286 ], [ false, %291 ], [ true, %1601 ], [ true, %1595 ], [ false, %295 ]
  %227 = phi { ptr, i32 } [ %1603, %1602 ], [ %240, %239 ], [ %229, %228 ], [ %287, %286 ], [ %343, %291 ], [ %1596, %1601 ], [ %1596, %1595 ], [ %296, %295 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %71)
          to label %178 unwind label %668

228:                                              ; preds = %220
  %229 = landingpad { ptr, i32 }
          cleanup
  br label %224

230:                                              ; preds = %220
  call void @llvm.lifetime.start.p0(ptr nonnull %69), !noalias !23057
  %231 = load ptr, ptr %3, align 8, !alias.scope !23041, !noalias !23060, !nonnull !1740, !noundef !1740
  %232 = atomicrmw add ptr %231, i64 1 monotonic, align 8, !noalias !23050
  %233 = icmp slt i64 %232, 0
  br i1 %233, label %238, label %234

234:                                              ; preds = %230
  %235 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %236 = load i64, ptr %235, align 8, !alias.scope !23041, !noalias !23060, !noundef !1740
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %237 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %70, ptr noundef nonnull %231, i64 noundef %236)
          to label %241 unwind label %1602, !noalias !23050

238:                                              ; preds = %230
  tail call void @llvm.trap()
  unreachable

239:                                              ; preds = %1446
  %240 = landingpad { ptr, i32 }
          cleanup
  br label %224

241:                                              ; preds = %234
  store i64 %237, ptr %69, align 8, !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %68), !noalias !23057
  %242 = getelementptr inbounds nuw i8, ptr %70, i64 16
  %243 = load i64, ptr %242, align 8, !noundef !1740
  %244 = icmp ult i64 %243, 576460752303423488
  call void @llvm.assume(i1 %244)
  store i64 %243, ptr %68, align 8, !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %67), !noalias !23057
  %245 = getelementptr inbounds nuw i8, ptr %29, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %245, ptr noundef nonnull align 8 dereferenceable(56) %70, i64 56, i1 false), !noalias !23057
  store i64 1, ptr %29, align 8, !noalias !23057
  %246 = getelementptr inbounds nuw i8, ptr %29, i64 8
  store i64 1, ptr %246, align 8, !noalias !23057
  %247 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #92, !noalias !23061
  %248 = icmp eq ptr %247, null
  br i1 %248, label %__rustc::__rust_alloc (.exit.thread), label %249

249:                                              ; preds = %241
  %250 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %251 = load i64, ptr %250, align 8, !noalias !23061, !noundef !1740
  %252 = call i64 @llvm.uadd.sat.i64(i64 %251, i64 1)
  store i64 %252, ptr %250, align 8, !noalias !23061
  %253 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %254 = load i64, ptr %253, align 8, !noalias !23061, !noundef !1740
  %255 = call i64 @llvm.uadd.sat.i64(i64 %254, i64 72)
  store i64 %255, ptr %253, align 8, !noalias !23061
  %256 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %257 = load i64, ptr %256, align 8, !noalias !23061, !noundef !1740
  %258 = call i64 @llvm.sadd.sat.i64(i64 %257, i64 72)
  store i64 %258, ptr %256, align 8, !noalias !23061
  %259 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %260 = load i64, ptr %259, align 8, !noalias !23061, !noundef !1740
  %261 = icmp sgt i64 %258, %260
  br i1 %261, label %262, label %.preheader539

262:                                              ; preds = %249
  store i64 %258, ptr %259, align 8, !noalias !23061
  br label %.preheader539

.preheader539:                                    ; preds = %262, %249
  br label %263

263:                                              ; preds = %.preheader539, %266
  %264 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23061
  %265 = icmp slt i64 %264, 0
  br i1 %265, label %266, label %__rustc::__rust_alloc (.exit)

266:                                              ; preds = %263
  %267 = add nsw i64 %264, 1
  %268 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %264, i64 %267 acq_rel acquire, align 8, !noalias !23061
  %269 = extractvalue { i64, i1 } %268, 1
  br i1 %269, label %270, label %263

270:                                              ; preds = %266
  %271 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !23061
  %272 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !23061
  %273 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !23061
  %274 = call i64 @llvm.sadd.sat.i64(i64 %273, i64 72)
  %275 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !23061
  br label %276

276:                                              ; preds = %279, %270
  %277 = phi i64 [ %275, %270 ], [ %282, %279 ]
  %278 = icmp sgt i64 %274, %277
  br i1 %278, label %279, label %283

279:                                              ; preds = %276
  %280 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %277, i64 %274 monotonic monotonic, align 8, !noalias !23061
  %281 = extractvalue { i64, i1 } %280, 1
  %282 = extractvalue { i64, i1 } %280, 0
  br i1 %281, label %283, label %276

283:                                              ; preds = %279, %276
  %284 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23061
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %241
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #93
          to label %285 unwind label %286

285:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

286:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  %287 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %245)
          to label %224 unwind label %288

288:                                              ; preds = %286
  %289 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23050
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %263, %283
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %247, ptr noundef nonnull align 8 dereferenceable(72) %29, i64 72, i1 false), !noalias !23050
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !23057
  store ptr %247, ptr %67, align 8, !noalias !23057
; invoke purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
  %290 = invoke fastcc noundef nonnull ptr @purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 dereferenceable(1248) %5, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %4)
          to label %297 unwind label %292, !noalias !23050

291:                                              ; preds = %340
  br i1 %342, label %1595, label %224

292:                                              ; preds = %1365, %297, %__rustc::__rust_alloc (.exit)
  %293 = phi i8 [ 1, %__rustc::__rust_alloc (.exit) ], [ 1, %297 ], [ %1366, %1365 ]
  %294 = landingpad { ptr, i32 }
          cleanup
  br label %1595

295:                                              ; preds = %1367
  %296 = landingpad { ptr, i32 }
          cleanup
  br label %224

297:                                              ; preds = %__rustc::__rust_alloc (.exit)
  call void @llvm.lifetime.start.p0(ptr nonnull %66), !noalias !23057
  %298 = load ptr, ptr %67, align 8, !noalias !23057, !nonnull !1740, !noundef !1740
  %299 = getelementptr inbounds nuw i8, ptr %298, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(216) %66, ptr noundef nonnull %290, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %4, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %299, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5)
          to label %300 unwind label %292, !noalias !23050

300:                                              ; preds = %297
  call void @llvm.lifetime.start.p0(ptr nonnull %65), !noalias !23057
  br i1 %206, label %670, label %301

301:                                              ; preds = %300
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !23057
  %302 = mul nuw nsw i64 %208, 40
  %303 = icmp eq i64 %208, 0
  br i1 %303, label %346, label %304

304:                                              ; preds = %301
  %305 = call noundef ptr @malloc(i64 noundef range(i64 1, 0) %302) #92, !noalias !23064
  %306 = icmp eq ptr %305, null
  br i1 %306, label %__rustc::__rust_alloc (.exit80.thread), label %307

307:                                              ; preds = %304
  %308 = load i64, ptr %250, align 8, !noalias !23064, !noundef !1740
  %309 = call i64 @llvm.uadd.sat.i64(i64 %308, i64 1)
  store i64 %309, ptr %250, align 8, !noalias !23064
  %310 = load i64, ptr %253, align 8, !noalias !23064, !noundef !1740
  %311 = call i64 @llvm.uadd.sat.i64(i64 %310, i64 %302)
  store i64 %311, ptr %253, align 8, !noalias !23064
  %312 = load i64, ptr %256, align 8, !noalias !23064, !noundef !1740
  %313 = call i64 @llvm.sadd.sat.i64(i64 %312, i64 %302)
  store i64 %313, ptr %256, align 8, !noalias !23064
  %314 = load i64, ptr %259, align 8, !noalias !23064, !noundef !1740
  %315 = icmp sgt i64 %313, %314
  br i1 %315, label %316, label %.preheader538

316:                                              ; preds = %307
  store i64 %313, ptr %259, align 8, !noalias !23064
  br label %.preheader538

.preheader538:                                    ; preds = %316, %307
  br label %317

317:                                              ; preds = %.preheader538, %320
  %318 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23064
  %319 = icmp slt i64 %318, 0
  br i1 %319, label %320, label %__rustc::__rust_alloc (.exit80)

320:                                              ; preds = %317
  %321 = add nsw i64 %318, 1
  %322 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %318, i64 %321 acq_rel acquire, align 8, !noalias !23064
  %323 = extractvalue { i64, i1 } %322, 1
  br i1 %323, label %324, label %317

324:                                              ; preds = %320
  %325 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !23064
  %326 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %302 monotonic, align 8, !noalias !23064
  %327 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %302 monotonic, align 8, !noalias !23064
  %328 = call i64 @llvm.sadd.sat.i64(i64 %327, i64 %302)
  %329 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !23064
  br label %330

330:                                              ; preds = %333, %324
  %331 = phi i64 [ %329, %324 ], [ %336, %333 ]
  %332 = icmp sgt i64 %328, %331
  br i1 %332, label %333, label %337

333:                                              ; preds = %330
  %334 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %331, i64 %328 monotonic monotonic, align 8, !noalias !23064
  %335 = extractvalue { i64, i1 } %334, 1
  %336 = extractvalue { i64, i1 } %334, 0
  br i1 %335, label %337, label %330

337:                                              ; preds = %333, %330
  %338 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23064
  br label %__rustc::__rust_alloc (.exit80)

__rustc::__rust_alloc (.exit80): ; preds = %317, %337
  %339 = ptrtoint ptr %305 to i64
  br label %346

340:                                              ; preds = %1428, %1218, %702, %699, %695, %378, %344
  %341 = phi i8 [ 1, %344 ], [ 0, %378 ], [ %652, %1428 ], [ %652, %1218 ], [ 1, %702 ], [ 1, %695 ], [ 1, %699 ]
  %342 = phi i1 [ true, %344 ], [ true, %378 ], [ true, %1428 ], [ false, %1218 ], [ true, %702 ], [ true, %695 ], [ true, %699 ]
  %343 = phi { ptr, i32 } [ %345, %344 ], [ %379, %378 ], [ %1429, %1428 ], [ %1219, %1218 ], [ %696, %702 ], [ %696, %695 ], [ %696, %699 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %66) #89
          to label %291 unwind label %668, !noalias !23050

344:                                              ; preds = %1436, %1050, %670, %__rustc::__rust_alloc (.exit80.thread)
  %345 = landingpad { ptr, i32 }
          cleanup
  br label %340

__rustc::__rust_alloc (.exit80.thread): ; preds = %304
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %302) #93
          to label %544 unwind label %344, !noalias !23050

346:                                              ; preds = %__rustc::__rust_alloc (.exit80), %301
  %347 = phi i64 [ %339, %__rustc::__rust_alloc (.exit80) ], [ 8, %301 ]
  %348 = inttoptr i64 %347 to ptr
  store i64 %208, ptr %41, align 8, !noalias !23057
  %349 = getelementptr inbounds nuw i8, ptr %41, i64 8
  store ptr %348, ptr %349, align 8, !noalias !23057
  %350 = getelementptr inbounds nuw i8, ptr %41, i64 16
  store i64 0, ptr %350, align 8, !noalias !23057
  %351 = getelementptr inbounds nuw i8, ptr %77, i64 8
  %352 = load ptr, ptr %351, align 8, !alias.scope !23043, !noalias !23059, !nonnull !1740, !noundef !1740
  %353 = load i64, ptr %77, align 8, !range !1835, !alias.scope !23043, !noalias !23059, !noundef !1740
  %354 = getelementptr inbounds nuw i8, ptr %352, i64 %302
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !23057
  store ptr %352, ptr %40, align 8, !noalias !23057
  %355 = getelementptr inbounds nuw i8, ptr %40, i64 8
  %356 = getelementptr inbounds nuw i8, ptr %40, i64 16
  store i64 %353, ptr %356, align 8, !noalias !23057
  %357 = getelementptr inbounds nuw i8, ptr %40, i64 24
  store ptr %354, ptr %357, align 8, !noalias !23057
  %358 = getelementptr inbounds nuw i8, ptr %40, i64 32
  call void @llvm.lifetime.start.p0(ptr nonnull %39)
  br i1 %303, label %.loopexit110, label %359

359:                                              ; preds = %346
  %360 = getelementptr inbounds nuw i8, ptr %38, i64 8
  %361 = getelementptr inbounds nuw i8, ptr %38, i64 16
  %362 = getelementptr inbounds nuw i8, ptr %13, i64 8
  %363 = getelementptr inbounds nuw i8, ptr %5, i64 568
  %364 = getelementptr inbounds nuw i8, ptr %36, i64 8
  br label %365

365:                                              ; preds = %545, %359
  %366 = phi ptr [ %348, %359 ], [ %546, %545 ]
  %367 = phi i64 [ 0, %359 ], [ %382, %545 ]
  %368 = phi ptr [ %352, %359 ], [ %369, %545 ]
  %369 = getelementptr inbounds nuw i8, ptr %368, i64 40
  %370 = load i64, ptr %368, align 8, !noalias !23067
  %371 = icmp eq i64 %370, 0
  br i1 %371, label %.loopexit110, label %380

372:                                              ; preds = %666, %542
  %373 = phi i64 [ %664, %666 ], [ %532, %542 ]
  %374 = phi ptr [ %667, %666 ], [ %533, %542 ]
  %375 = phi { ptr, i32 } [ %663, %666 ], [ %540, %542 ]
  %376 = shl i64 %373, 3
  %377 = add i64 %376, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %374, i64 noundef %377, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !1740
  br label %378

378:                                              ; preds = %662, %539, %372
  %379 = phi { ptr, i32 } [ %540, %539 ], [ %663, %662 ], [ %375, %372 ]
; call core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
  call fastcc void @core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>(ptr noalias nofree noundef align 8 dereferenceable(40) %40) #89, !noalias !23050
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %41) #89, !noalias !23050
  br label %340

380:                                              ; preds = %365
  %381 = getelementptr inbounds nuw i8, ptr %368, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %39, ptr noundef nonnull align 8 dereferenceable(32) %381, i64 32, i1 false), !noalias !23073
  %382 = add nuw nsw i64 %367, 1
  call void @llvm.lifetime.start.p0(ptr nonnull %38), !noalias !23057
  store i64 %370, ptr %38, align 8, !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %360, ptr noundef nonnull align 8 dereferenceable(32) %39, i64 32, i1 false), !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !23057
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %37, ptr noalias nofree noundef align 8 dereferenceable(200) %71, ptr noundef nonnull align 16 dereferenceable(1248) %5)
          to label %463 unwind label %658, !noalias !23050

.loopexit110:                                     ; preds = %545, %365, %346
  %383 = phi i64 [ 0, %346 ], [ %367, %365 ], [ %382, %545 ]
  %384 = phi ptr [ %352, %346 ], [ %369, %365 ], [ %354, %545 ]
  store ptr %384, ptr %355, align 8
  store i64 %383, ptr %358, align 8
  br label %385

385:                                              ; preds = %649, %.loopexit110
  %386 = phi ptr [ %369, %649 ], [ %384, %.loopexit110 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %39)
  %387 = ptrtoint ptr %354 to i64
  %388 = ptrtoint ptr %386 to i64
  %389 = sub nuw i64 %387, %388
  %390 = udiv exact i64 %389, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23074), !noalias !23050
  %391 = icmp eq ptr %354, %386
  br i1 %391, label %.loopexit105, label %.preheader104

.preheader104:                                    ; preds = %385
  %392 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %393

393:                                              ; preds = %.preheader104, %431
  %394 = phi i64 [ %396, %431 ], [ 0, %.preheader104 ]
  %395 = getelementptr inbounds nuw [40 x i8], ptr %386, i64 %394
  %396 = add nuw nsw i64 %394, 1
  %397 = load i64, ptr %395, align 8, !range !1778, !alias.scope !23077, !noalias !23080, !noundef !1740
  %398 = icmp ugt i64 %397, 5
  br i1 %398, label %399, label %431

399:                                              ; preds = %393
  %400 = getelementptr i8, ptr %395, i64 8
  %401 = load ptr, ptr %400, align 8, !alias.scope !23074, !noalias !23080, !nonnull !1740, !noundef !1740
  %402 = shl i64 %397, 3
  %403 = add i64 %402, -8
  %404 = load i64, ptr %256, align 8, !noalias !23087, !noundef !1740
  %405 = call i64 @llvm.umin.i64(i64 %403, i64 9223372036854775807)
  %406 = call i64 @llvm.ssub.sat.i64(i64 %404, i64 %405)
  store i64 %406, ptr %256, align 8, !noalias !23087
  %407 = load i64, ptr %392, align 8, !noalias !23087, !noundef !1740
  %408 = icmp slt i64 %406, %407
  br i1 %408, label %409, label %.preheader501

409:                                              ; preds = %399
  store i64 %406, ptr %392, align 8, !noalias !23087
  br label %.preheader501

.preheader501:                                    ; preds = %409, %399
  br label %410

410:                                              ; preds = %.preheader501, %413
  %411 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23087
  %412 = icmp slt i64 %411, 0
  br i1 %412, label %413, label %__rustc::__rust_dealloc (.exit81)

413:                                              ; preds = %410
  %414 = add nsw i64 %411, 1
  %415 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %411, i64 %414 acq_rel acquire, align 8, !noalias !23087
  %416 = extractvalue { i64, i1 } %415, 1
  br i1 %416, label %417, label %410

417:                                              ; preds = %413
  %418 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %405 monotonic, align 8, !noalias !23087
  %419 = call i64 @llvm.ssub.sat.i64(i64 %418, i64 %405)
  %420 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23087
  br label %421

421:                                              ; preds = %424, %417
  %422 = phi i64 [ %420, %417 ], [ %427, %424 ]
  %423 = icmp slt i64 %419, %422
  br i1 %423, label %424, label %428

424:                                              ; preds = %421
  %425 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %422, i64 %419 monotonic monotonic, align 8, !noalias !23087
  %426 = extractvalue { i64, i1 } %425, 1
  %427 = extractvalue { i64, i1 } %425, 0
  br i1 %426, label %428, label %421

428:                                              ; preds = %424, %421
  %429 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23087
  br label %__rustc::__rust_dealloc (.exit81)

__rustc::__rust_dealloc (.exit81): ; preds = %410, %428
  %430 = icmp ne i64 %403, 0
  call void @llvm.assume(i1 %430), !noalias !23087
  call void @free(ptr noundef nonnull %401) #92, !noalias !23087
  br label %431

431:                                              ; preds = %__rustc::__rust_dealloc (.exit81), %393
  %432 = icmp eq i64 %396, %390
  br i1 %432, label %.loopexit105, label %393

.loopexit105:                                     ; preds = %431, %385
  %433 = icmp eq i64 %353, 0
  br i1 %433, label %650, label %434

434:                                              ; preds = %.loopexit105
  %435 = mul nuw i64 %353, 40
  %436 = load i64, ptr %256, align 8, !noalias !23080, !noundef !1740
  %437 = call i64 @llvm.umin.i64(i64 %435, i64 9223372036854775807)
  %438 = call i64 @llvm.ssub.sat.i64(i64 %436, i64 %437)
  store i64 %438, ptr %256, align 8, !noalias !23080
  %439 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %440 = load i64, ptr %439, align 8, !noalias !23080, !noundef !1740
  %441 = icmp slt i64 %438, %440
  br i1 %441, label %442, label %.preheader500

442:                                              ; preds = %434
  store i64 %438, ptr %439, align 8, !noalias !23080
  br label %.preheader500

.preheader500:                                    ; preds = %442, %434
  br label %443

443:                                              ; preds = %.preheader500, %446
  %444 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23080
  %445 = icmp slt i64 %444, 0
  br i1 %445, label %446, label %__rustc::__rust_dealloc (.exit82)

446:                                              ; preds = %443
  %447 = add nsw i64 %444, 1
  %448 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %444, i64 %447 acq_rel acquire, align 8, !noalias !23080
  %449 = extractvalue { i64, i1 } %448, 1
  br i1 %449, label %450, label %443

450:                                              ; preds = %446
  %451 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %437 monotonic, align 8, !noalias !23080
  %452 = call i64 @llvm.ssub.sat.i64(i64 %451, i64 %437)
  %453 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23080
  br label %454

454:                                              ; preds = %457, %450
  %455 = phi i64 [ %453, %450 ], [ %460, %457 ]
  %456 = icmp slt i64 %452, %455
  br i1 %456, label %457, label %461

457:                                              ; preds = %454
  %458 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %455, i64 %452 monotonic monotonic, align 8, !noalias !23080
  %459 = extractvalue { i64, i1 } %458, 1
  %460 = extractvalue { i64, i1 } %458, 0
  br i1 %459, label %461, label %454

461:                                              ; preds = %457, %454
  %462 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23080
  br label %__rustc::__rust_dealloc (.exit82)

__rustc::__rust_dealloc (.exit82): ; preds = %443, %461
  call void @free(ptr noundef nonnull %352) #92, !noalias !23080
  br label %650

463:                                              ; preds = %380
  %464 = load i8, ptr %37, align 8, !range !1743, !noalias !23057, !noundef !1740
  %465 = icmp eq i8 %464, -1
  br i1 %465, label %466, label %486

466:                                              ; preds = %463
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !23057
  %467 = load i64, ptr %68, align 8, !noalias !23057, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23090)
  %468 = load i64, ptr %38, align 8, !range !1778, !alias.scope !23090, !noalias !23050, !noundef !1740
  %469 = add i64 %468, -1
  %470 = icmp ugt i64 %469, 4
  %471 = load i64, ptr %361, align 8, !alias.scope !23090, !noalias !23050
  %472 = add i64 %471, -1
  %473 = select i1 %470, i64 %472, i64 %469
  %474 = icmp ugt i64 %467, %473
  br i1 %474, label %483, label %475

475:                                              ; preds = %466
  %476 = icmp ugt i64 %468, 5
  %477 = select i1 %476, i64 %471, i64 %468
  %478 = add i64 %477, -1
  %479 = icmp ult i64 %467, %478
  br i1 %479, label %480, label %493

480:                                              ; preds = %475
  %481 = select i1 %476, ptr %361, ptr %38
  %482 = add nuw i64 %467, 1
  store i64 %482, ptr %481, align 8, !alias.scope !23093, !noalias !23050
  br label %493

483:                                              ; preds = %466
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !23096
  %484 = sub nuw i64 %467, %473
  store i32 2, ptr %13, align 8, !noalias !23096
  store i64 %484, ptr %362, align 8, !noalias !23096
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %38, ptr noalias nofree noundef align 8 captures(address) dereferenceable(16) %13)
          to label %485 unwind label %658

485:                                              ; preds = %483
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !23096
  br label %493

486:                                              ; preds = %463
  store ptr %369, ptr %355, align 8
  store i64 %382, ptr %358, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !23057
  %487 = load i64, ptr %38, align 8, !range !1778, !alias.scope !19667, !noundef !1740
  %488 = icmp ugt i64 %487, 5
  br i1 %488, label %489, label %649

489:                                              ; preds = %486
  %490 = load ptr, ptr %360, align 8, !nonnull !1740, !noundef !1740
  %491 = shl i64 %487, 3
  %492 = add i64 %491, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %490, i64 noundef %492, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23097
  br label %649

493:                                              ; preds = %485, %480, %475
  store i64 %367, ptr %363, align 8, !alias.scope !23045, !noalias !23100
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !23057
  %494 = load i64, ptr %38, align 8, !range !1778, !noalias !23057, !noundef !1740
  %495 = add i64 %494, -1
  %496 = icmp ugt i64 %495, 4
  %497 = load ptr, ptr %360, align 8, !noalias !23057, !nonnull !1740
  %498 = load i64, ptr %361, align 8, !noalias !23057
  %499 = add i64 %498, -1
  %500 = select i1 %496, i64 %499, i64 %495
  %501 = select i1 %496, ptr %497, ptr %360
  %502 = load ptr, ptr %67, align 8, !noalias !23057, !nonnull !1740, !noundef !1740
  %503 = getelementptr inbounds nuw i8, ptr %502, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %36, ptr noalias nofree noundef align 8 dereferenceable(216) %66, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %501, i64 noundef %500, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %503, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5)
          to label %504 unwind label %658, !noalias !23050

504:                                              ; preds = %493
  %505 = load i64, ptr %36, align 16, !range !2527, !noalias !23057, !noundef !1740
  %506 = icmp eq i64 %505, -1
  br i1 %506, label %519, label %507

507:                                              ; preds = %504
  store ptr %369, ptr %355, align 8
  %508 = getelementptr inbounds nuw i8, ptr %36, i64 16
  %509 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %509, ptr noundef nonnull align 16 dereferenceable(80) %508, i64 80, i1 false), !noalias !23101
  %510 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %511 = getelementptr inbounds nuw i8, ptr %0, i64 24
  %512 = load <2 x i32>, ptr %364, align 8, !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23057
  store i64 %505, ptr %510, align 16, !alias.scope !23038, !noalias !23101
  store <2 x i32> %512, ptr %511, align 8, !alias.scope !23038, !noalias !23101
  store i64 1, ptr %0, align 16, !alias.scope !23038, !noalias !23101
  %513 = load i64, ptr %38, align 8, !range !1778, !alias.scope !19667, !noundef !1740
  %514 = icmp ugt i64 %513, 5
  br i1 %514, label %515, label %551

515:                                              ; preds = %507
  %516 = load ptr, ptr %360, align 8, !nonnull !1740, !noundef !1740
  %517 = shl i64 %513, 3
  %518 = add i64 %517, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %516, i64 noundef %518, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23102
  br label %551

519:                                              ; preds = %504
  %520 = load <2 x i32>, ptr %364, align 8, !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23057
  %521 = load i64, ptr %38, align 8, !range !1778, !noalias !23057, !noundef !1740
  %522 = icmp ugt i64 %521, 5
  %523 = load i64, ptr %361, align 8
  %524 = select i1 %522, i64 %523, i64 %521
  %525 = add i64 %524, -1
  %526 = load i64, ptr %69, align 8, !noalias !23057, !noundef !1740
  %527 = icmp ult i64 %526, %525
  br i1 %527, label %528, label %543

528:                                              ; preds = %519
  %529 = load ptr, ptr %360, align 8, !noalias !23057, !nonnull !1740
  %530 = select i1 %522, ptr %529, ptr %360
  %531 = getelementptr inbounds nuw [8 x i8], ptr %530, i64 %526
  store <2 x i32> %520, ptr %531, align 4, !noalias !23050
  call void @llvm.lifetime.start.p0(ptr nonnull %35)
  %532 = load i64, ptr %38, align 8, !noalias !23057
  %533 = load ptr, ptr %360, align 8, !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %35, ptr noundef nonnull align 8 dereferenceable(24) %361, i64 24, i1 false), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23105)
  %534 = load i64, ptr %41, align 8, !range !1835, !alias.scope !23105, !noalias !23108, !noundef !1740
  %535 = icmp eq i64 %367, %534
  br i1 %535, label %536, label %545

536:                                              ; preds = %528
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %41)
          to label %537 unwind label %539, !noalias !23108

537:                                              ; preds = %536
  %538 = load ptr, ptr %349, align 8, !alias.scope !23105, !noalias !23108
  br label %545

539:                                              ; preds = %536
  %540 = landingpad { ptr, i32 }
          cleanup
  store ptr %369, ptr %355, align 8
  store i64 %382, ptr %358, align 8
  %541 = icmp ugt i64 %532, 5
  br i1 %541, label %542, label %378

542:                                              ; preds = %539
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %533) ]
  br label %372

543:                                              ; preds = %519
  store ptr %369, ptr %355, align 8
  store i64 %382, ptr %358, align 8
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %526, i64 noundef %525, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.402) #93
          to label %544 unwind label %660, !noalias !23050

544:                                              ; preds = %1193, %1053, %543, %__rustc::__rust_alloc (.exit80.thread)
  unreachable

545:                                              ; preds = %537, %528
  %546 = phi ptr [ %538, %537 ], [ %366, %528 ]
  %547 = getelementptr inbounds nuw [40 x i8], ptr %546, i64 %367
  store i64 %532, ptr %547, align 8, !noalias !23110
  %548 = getelementptr inbounds nuw i8, ptr %547, i64 8
  store ptr %533, ptr %548, align 8, !noalias !23110
  %549 = getelementptr inbounds nuw i8, ptr %547, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %549, ptr noundef nonnull align 8 dereferenceable(24) %35, i64 24, i1 false), !noalias !23110
  store i64 %382, ptr %350, align 8, !alias.scope !23105, !noalias !23108
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %39)
  call void @llvm.lifetime.start.p0(ptr nonnull %39)
  %550 = icmp eq ptr %369, %354
  br i1 %550, label %.loopexit110, label %365

551:                                              ; preds = %515, %507
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %39)
  %552 = ptrtoint ptr %354 to i64
  %553 = ptrtoint ptr %369 to i64
  %554 = sub nuw i64 %552, %553
  %555 = udiv exact i64 %554, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23111), !noalias !23050
  %556 = icmp eq ptr %354, %369
  br i1 %556, label %.loopexit109, label %.preheader108

.preheader108:                                    ; preds = %551
  %557 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %558

558:                                              ; preds = %.preheader108, %596
  %559 = phi i64 [ %561, %596 ], [ 0, %.preheader108 ]
  %560 = getelementptr inbounds nuw [40 x i8], ptr %369, i64 %559
  %561 = add nuw nsw i64 %559, 1
  %562 = load i64, ptr %560, align 8, !range !1778, !alias.scope !23114, !noalias !23117, !noundef !1740
  %563 = icmp ugt i64 %562, 5
  br i1 %563, label %564, label %596

564:                                              ; preds = %558
  %565 = getelementptr i8, ptr %560, i64 8
  %566 = load ptr, ptr %565, align 8, !alias.scope !23111, !noalias !23117, !nonnull !1740, !noundef !1740
  %567 = shl i64 %562, 3
  %568 = add i64 %567, -8
  %569 = load i64, ptr %256, align 8, !noalias !23124, !noundef !1740
  %570 = call i64 @llvm.umin.i64(i64 %568, i64 9223372036854775807)
  %571 = call i64 @llvm.ssub.sat.i64(i64 %569, i64 %570)
  store i64 %571, ptr %256, align 8, !noalias !23124
  %572 = load i64, ptr %557, align 8, !noalias !23124, !noundef !1740
  %573 = icmp slt i64 %571, %572
  br i1 %573, label %574, label %.preheader503

574:                                              ; preds = %564
  store i64 %571, ptr %557, align 8, !noalias !23124
  br label %.preheader503

.preheader503:                                    ; preds = %574, %564
  br label %575

575:                                              ; preds = %.preheader503, %578
  %576 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23124
  %577 = icmp slt i64 %576, 0
  br i1 %577, label %578, label %__rustc::__rust_dealloc (.exit83)

578:                                              ; preds = %575
  %579 = add nsw i64 %576, 1
  %580 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %576, i64 %579 acq_rel acquire, align 8, !noalias !23124
  %581 = extractvalue { i64, i1 } %580, 1
  br i1 %581, label %582, label %575

582:                                              ; preds = %578
  %583 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %570 monotonic, align 8, !noalias !23124
  %584 = call i64 @llvm.ssub.sat.i64(i64 %583, i64 %570)
  %585 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23124
  br label %586

586:                                              ; preds = %589, %582
  %587 = phi i64 [ %585, %582 ], [ %592, %589 ]
  %588 = icmp slt i64 %584, %587
  br i1 %588, label %589, label %593

589:                                              ; preds = %586
  %590 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %587, i64 %584 monotonic monotonic, align 8, !noalias !23124
  %591 = extractvalue { i64, i1 } %590, 1
  %592 = extractvalue { i64, i1 } %590, 0
  br i1 %591, label %593, label %586

593:                                              ; preds = %589, %586
  %594 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23124
  br label %__rustc::__rust_dealloc (.exit83)

__rustc::__rust_dealloc (.exit83): ; preds = %575, %593
  %595 = icmp ne i64 %568, 0
  call void @llvm.assume(i1 %595), !noalias !23124
  call void @free(ptr noundef nonnull %566) #92, !noalias !23124
  br label %596

596:                                              ; preds = %__rustc::__rust_dealloc (.exit83), %558
  %597 = icmp eq i64 %561, %555
  br i1 %597, label %.loopexit109, label %558

.loopexit109:                                     ; preds = %596, %551
  %598 = icmp eq i64 %353, 0
  br i1 %598, label %601, label %599

599:                                              ; preds = %.loopexit109
  %600 = mul nuw i64 %353, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %352, i64 noundef %600, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !23117
  br label %601

601:                                              ; preds = %599, %.loopexit109
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23127)
  call void @llvm.experimental.noalias.scope.decl(metadata !23130), !noalias !23050
  %602 = icmp eq i64 %367, 0
  br i1 %602, label %.loopexit107, label %.preheader106

.preheader106:                                    ; preds = %601
  %603 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %604

604:                                              ; preds = %.preheader106, %642
  %605 = phi i64 [ %607, %642 ], [ 0, %.preheader106 ]
  %606 = getelementptr inbounds nuw [40 x i8], ptr %366, i64 %605
  %607 = add nuw nsw i64 %605, 1
  %608 = load i64, ptr %606, align 8, !range !1778, !alias.scope !23133, !noalias !23136, !noundef !1740
  %609 = icmp ugt i64 %608, 5
  br i1 %609, label %610, label %642

610:                                              ; preds = %604
  %611 = getelementptr i8, ptr %606, i64 8
  %612 = load ptr, ptr %611, align 8, !alias.scope !23130, !noalias !23136, !nonnull !1740, !noundef !1740
  %613 = shl i64 %608, 3
  %614 = add i64 %613, -8
  %615 = load i64, ptr %256, align 8, !noalias !23137, !noundef !1740
  %616 = call i64 @llvm.umin.i64(i64 %614, i64 9223372036854775807)
  %617 = call i64 @llvm.ssub.sat.i64(i64 %615, i64 %616)
  store i64 %617, ptr %256, align 8, !noalias !23137
  %618 = load i64, ptr %603, align 8, !noalias !23137, !noundef !1740
  %619 = icmp slt i64 %617, %618
  br i1 %619, label %620, label %.preheader502

620:                                              ; preds = %610
  store i64 %617, ptr %603, align 8, !noalias !23137
  br label %.preheader502

.preheader502:                                    ; preds = %620, %610
  br label %621

621:                                              ; preds = %.preheader502, %624
  %622 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23137
  %623 = icmp slt i64 %622, 0
  br i1 %623, label %624, label %__rustc::__rust_dealloc (.exit84)

624:                                              ; preds = %621
  %625 = add nsw i64 %622, 1
  %626 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %622, i64 %625 acq_rel acquire, align 8, !noalias !23137
  %627 = extractvalue { i64, i1 } %626, 1
  br i1 %627, label %628, label %621

628:                                              ; preds = %624
  %629 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %616 monotonic, align 8, !noalias !23137
  %630 = call i64 @llvm.ssub.sat.i64(i64 %629, i64 %616)
  %631 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23137
  br label %632

632:                                              ; preds = %635, %628
  %633 = phi i64 [ %631, %628 ], [ %638, %635 ]
  %634 = icmp slt i64 %630, %633
  br i1 %634, label %635, label %639

635:                                              ; preds = %632
  %636 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %633, i64 %630 monotonic monotonic, align 8, !noalias !23137
  %637 = extractvalue { i64, i1 } %636, 1
  %638 = extractvalue { i64, i1 } %636, 0
  br i1 %637, label %639, label %632

639:                                              ; preds = %635, %632
  %640 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23137
  br label %__rustc::__rust_dealloc (.exit84)

__rustc::__rust_dealloc (.exit84): ; preds = %621, %639
  %641 = icmp ne i64 %614, 0
  call void @llvm.assume(i1 %641), !noalias !23137
  call void @free(ptr noundef nonnull %612) #92, !noalias !23137
  br label %642

642:                                              ; preds = %__rustc::__rust_dealloc (.exit84), %604
  %643 = icmp eq i64 %607, %367
  br i1 %643, label %.loopexit107, label %604

.loopexit107:                                     ; preds = %642, %601
  %644 = load i64, ptr %41, align 8, !alias.scope !23127, !noalias !23050
  %645 = icmp eq i64 %644, 0
  br i1 %645, label %648, label %646

646:                                              ; preds = %.loopexit107
  %647 = mul nuw i64 %644, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %366, i64 noundef %647, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !23136
  br label %648

648:                                              ; preds = %646, %.loopexit107
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !23057
  br label %1365

649:                                              ; preds = %489, %486
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !23057
  br label %385

650:                                              ; preds = %__rustc::__rust_dealloc (.exit82), %.loopexit105
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %65, ptr noundef nonnull align 8 dereferenceable(24) %41, i64 24, i1 false), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !23057
  br label %651

651:                                              ; preds = %1217, %650
  %652 = phi i8 [ 1, %1217 ], [ 0, %650 ]
  %653 = getelementptr inbounds nuw i8, ptr %5, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !23140)
  %654 = load ptr, ptr %653, align 8, !alias.scope !23140, !noalias !23143, !nonnull !1740, !noundef !1740
  %655 = getelementptr inbounds nuw i8, ptr %654, i64 40
  %656 = load atomic i32, ptr %655 acquire, align 4, !noalias !23145
  %657 = icmp eq i32 %656, 0
  br i1 %657, label %1220, label %1230

658:                                              ; preds = %493, %483, %380
  %659 = landingpad { ptr, i32 }
          cleanup
  store ptr %369, ptr %355, align 8
  store i64 %382, ptr %358, align 8
  br label %662

660:                                              ; preds = %543
  %661 = landingpad { ptr, i32 }
          cleanup
  br label %662

662:                                              ; preds = %660, %658
  %663 = phi { ptr, i32 } [ %659, %658 ], [ %661, %660 ]
  %664 = load i64, ptr %38, align 8, !range !1778, !alias.scope !19667, !noundef !1740
  %665 = icmp ugt i64 %664, 5
  br i1 %665, label %666, label %378

666:                                              ; preds = %662
  %667 = load ptr, ptr %360, align 8, !nonnull !1740, !noundef !1740
  br label %372

668:                                              ; preds = %1604, %1602, %1601, %1534, %1439, %1437, %1068, %702, %340, %224
  %669 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23146
  unreachable

670:                                              ; preds = %300
  %671 = getelementptr inbounds nuw i8, ptr %71, i64 184
  %672 = load i64, ptr %671, align 8, !noundef !1740
  %673 = call noundef range(i64 0, 230584300921369396) i64 @llvm.umin.i64(i64 %672, i64 range(i64 0, 230584300921369396) %208)
  %674 = getelementptr inbounds nuw i8, ptr %77, i64 8
  %675 = load ptr, ptr %674, align 8, !alias.scope !23043, !noalias !23059, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %64), !noalias !23057
  %676 = getelementptr inbounds nuw i8, ptr %5, i64 904
  %677 = load i64, ptr %676, align 8, !noundef !1740
  %678 = getelementptr inbounds nuw i8, ptr %5, i64 1040
  %679 = load i64, ptr %678, align 16, !noundef !1740
  %680 = icmp ult i64 %677, 115292150460684698
  call void @llvm.assume(i1 %680)
  %681 = add i64 %679, %677
  store i64 %681, ptr %64, align 8, !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %63), !noalias !23057
; invoke <purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
  %682 = invoke fastcc noundef ptr @<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot(ptr noundef nonnull align 16 dereferenceable(1248) %5, i64 noundef %673)
          to label %683 unwind label %344, !noalias !23050

683:                                              ; preds = %670
  store ptr %682, ptr %63, align 8, !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %62)
  call void @llvm.lifetime.start.p0(ptr nonnull %61), !noalias !23057
  %684 = getelementptr inbounds nuw i8, ptr %5, i64 616
  %685 = load ptr, ptr %684, align 8, !noundef !1740
  %686 = icmp eq ptr %685, null
  br i1 %686, label %705, label %687

687:                                              ; preds = %683
  %688 = getelementptr inbounds nuw i8, ptr %685, i64 16
  %689 = load i64, ptr %688, align 8
  %690 = icmp ugt i64 %689, -3
  br i1 %690, label %691, label %705

691:                                              ; preds = %687
  %692 = getelementptr inbounds nuw i8, ptr %685, i64 40
  %693 = load i64, ptr %692, align 8
  %694 = icmp ult i64 %693, -2
  br label %705

695:                                              ; preds = %1439, %1437, %1037, %751, %703
  %696 = phi { ptr, i32 } [ %1440, %1439 ], [ %752, %751 ], [ %704, %703 ], [ %1438, %1437 ], [ %1038, %1037 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23147)
  %697 = load ptr, ptr %63, align 8, !alias.scope !23147, !noalias !23050, !noundef !1740
  %698 = icmp eq ptr %697, null
  br i1 %698, label %340, label %699

699:                                              ; preds = %695
  %700 = atomicrmw sub ptr %697, i64 1 release, align 8, !noalias !23150
  %701 = icmp eq i64 %700, 1
  br i1 %701, label %702, label %340

702:                                              ; preds = %699
  fence acquire, !noalias !23050
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %63) #91
          to label %340 unwind label %668, !inline_history !2025

703:                                              ; preds = %719, %718
  %704 = landingpad { ptr, i32 }
          cleanup
  br label %695

705:                                              ; preds = %691, %687, %683
  %706 = phi i1 [ false, %683 ], [ true, %687 ], [ %694, %691 ]
  %707 = getelementptr inbounds nuw i8, ptr %5, i64 1234
  %708 = load i8, ptr %707, align 2, !range !1747, !noundef !1740
  %709 = trunc nuw i8 %708 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %60), !noalias !23057
  store ptr %5, ptr %60, align 8, !noalias !23057
  %710 = getelementptr inbounds nuw i8, ptr %60, i64 8
  store ptr %63, ptr %710, align 8, !noalias !23057
  %711 = getelementptr inbounds nuw i8, ptr %60, i64 16
  store ptr %71, ptr %711, align 8, !noalias !23057
  %712 = getelementptr inbounds nuw i8, ptr %60, i64 24
  store ptr %66, ptr %712, align 8, !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %59), !noalias !23057
  store ptr %675, ptr %59, align 8, !noalias !23057
  %713 = getelementptr inbounds nuw i8, ptr %59, i64 8
  store i64 %673, ptr %713, align 8, !noalias !23057
  %714 = getelementptr inbounds nuw i8, ptr %59, i64 16
  store ptr %68, ptr %714, align 8, !noalias !23057
  %715 = getelementptr inbounds nuw i8, ptr %59, i64 24
  store ptr %67, ptr %715, align 8, !noalias !23057
  %716 = getelementptr inbounds nuw i8, ptr %59, i64 32
  store ptr %69, ptr %716, align 8, !noalias !23057
  %717 = getelementptr inbounds nuw i8, ptr %59, i64 40
  store ptr %64, ptr %717, align 8, !noalias !23057
  br i1 %706, label %719, label %718

718:                                              ; preds = %705
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %61, i1 noundef zeroext %709, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %675, i64 noundef range(i64 0, 230584300921369396) %673, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %60, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(48) %59, ptr noundef nonnull align 8 %71)
          to label %720 unwind label %703, !inline_history !23153

719:                                              ; preds = %705
; invoke purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %61, i1 noundef zeroext %709, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %675, i64 noundef range(i64 0, 230584300921369396) %673, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %60, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(48) %59, ptr noundef nonnull align 8 %71)
          to label %720 unwind label %703, !inline_history !23153

720:                                              ; preds = %719, %718
  call void @llvm.lifetime.end.p0(ptr nonnull %59), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %60), !noalias !23057
  %721 = load i64, ptr %61, align 16, !range !2059, !noalias !23057, !noundef !1740
  %722 = icmp eq i64 %721, -1
  br i1 %722, label %723, label %729

723:                                              ; preds = %720
  %724 = getelementptr inbounds nuw i8, ptr %61, i64 16
  %725 = getelementptr inbounds nuw i8, ptr %61, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %62, ptr noundef nonnull align 16 dereferenceable(64) %725, i64 64, i1 false), !noalias !23057
  %726 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %727 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %728 = load <4 x i64>, ptr %724, align 16, !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %61), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %726, ptr noundef nonnull align 16 dereferenceable(64) %62, i64 64, i1 false), !noalias !23101
  store <4 x i64> %728, ptr %727, align 16, !alias.scope !23038, !noalias !23101
  store i64 1, ptr %0, align 16, !alias.scope !23038, !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  br label %1430

729:                                              ; preds = %720
  %730 = getelementptr inbounds nuw i8, ptr %61, i64 8
  %731 = getelementptr inbounds nuw i8, ptr %61, i64 24
  %732 = load i64, ptr %731, align 8, !noalias !23057
  %733 = getelementptr inbounds nuw i8, ptr %61, i64 32
  %734 = load i64, ptr %733, align 16, !noalias !23057
  %735 = getelementptr inbounds nuw i8, ptr %61, i64 40
  %736 = load i64, ptr %735, align 8, !noalias !23057
  %737 = getelementptr inbounds nuw i8, ptr %61, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(208) %62, ptr noundef nonnull align 16 dereferenceable(208) %737, i64 208, i1 false), !noalias !23057
  %738 = getelementptr inbounds nuw i8, ptr %56, i64 24
  %739 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %740 = load <2 x i64>, ptr %730, align 8, !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %61), !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %56), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %738, ptr noundef nonnull align 16 dereferenceable(208) %62, i64 208, i1 false), !noalias !23057
  store i64 %721, ptr %15, align 8
  store <2 x i64> %740, ptr %739, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %62)
  call void @llvm.lifetime.start.p0(ptr nonnull %57), !noalias !23057
  %741 = add i64 %732, -3
  %742 = icmp ult i64 %741, -2
  %743 = select i1 %742, i64 %736, i64 %732
  %744 = add i64 %743, -1
  %745 = select i1 %742, i64 %732, i64 1
  %746 = select i1 %742, i64 1, i64 %736
  store i64 %745, ptr %56, align 8, !noalias !23057
  %747 = getelementptr inbounds nuw i8, ptr %56, i64 8
  store i64 %734, ptr %747, align 8, !noalias !23057
  %748 = getelementptr inbounds nuw i8, ptr %56, i64 16
  store i64 %746, ptr %748, align 8, !noalias !23057
  %749 = getelementptr inbounds nuw i8, ptr %56, i64 232
  store i64 0, ptr %749, align 8, !noalias !23057
  %750 = getelementptr inbounds nuw i8, ptr %56, i64 240
  store i64 %744, ptr %750, align 8, !noalias !23057
; invoke <purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(240) %57, ptr noalias nofree noundef align 8 captures(address) dereferenceable(248) %56)
          to label %753 unwind label %1439, !noalias !23050

751:                                              ; preds = %1030
  %752 = landingpad { ptr, i32 }
          cleanup
  br label %695

753:                                              ; preds = %729
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %58), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %58, ptr noundef nonnull align 8 dereferenceable(32) %57, i64 32, i1 false), !noalias !23057
  %754 = getelementptr inbounds nuw i8, ptr %57, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %51, ptr noundef nonnull align 8 dereferenceable(208) %754, i64 208, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %57), !noalias !23057
  %755 = load ptr, ptr %684, align 8, !noalias !23050, !noundef !1740
  %756 = icmp eq ptr %755, null
  %757 = getelementptr inbounds nuw i8, ptr %71, i64 194
  br i1 %756, label %948, label %758

758:                                              ; preds = %753
  call void @llvm.lifetime.start.p0(ptr nonnull %49)
  call void @llvm.lifetime.start.p0(ptr nonnull %47)
  call void @llvm.lifetime.start.p0(ptr nonnull %48)
  %.sroa.0.0.copyload = load i64, ptr %51, align 8
  %.sroa.6.0..sroa_idx = getelementptr inbounds nuw i8, ptr %51, i64 8
  %.sroa.6.0.copyload = load i64, ptr %.sroa.6.0..sroa_idx, align 8
  %.sroa.7.0..sroa_idx = getelementptr inbounds nuw i8, ptr %51, i64 16
  %.sroa.7.0.copyload = load i64, ptr %.sroa.7.0..sroa_idx, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !23154)
  %759 = load i8, ptr %757, align 2, !range !3730, !alias.scope !23154, !noalias !23157, !noundef !1740
  %760 = icmp eq i8 %759, 2
  br i1 %760, label %764, label %761

761:                                              ; preds = %758
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !23162
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !23162
  store i64 0, ptr %22, align 8, !noalias !23162
  %762 = getelementptr inbounds nuw i8, ptr %22, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %762, align 8, !noalias !23162
  %763 = getelementptr inbounds nuw i8, ptr %22, i64 16
  store i64 0, ptr %763, align 8, !noalias !23162
; invoke purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, &mut purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, &mut purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %23, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %15, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %22)
          to label %921 unwind label %943

764:                                              ; preds = %758
  %.sroa.8.0..sroa_idx = getelementptr inbounds nuw i8, ptr %51, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !23162
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !23162
  %765 = getelementptr inbounds nuw i8, ptr %20, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %765, ptr noundef nonnull align 8 dereferenceable(184) %.sroa.8.0..sroa_idx, i64 184, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !23163)
  %766 = icmp ugt i64 %.sroa.0.0.copyload, 2
  %767 = select i1 %766, i64 %.sroa.7.0.copyload, i64 %.sroa.0.0.copyload
  %768 = add i64 %767, -1
  %769 = select i1 %766, i64 1, i64 %.sroa.7.0.copyload
  %770 = select i1 %766, i64 %.sroa.0.0.copyload, i64 1
  store i64 %770, ptr %20, align 8, !alias.scope !23166, !noalias !23168
  %771 = getelementptr inbounds nuw i8, ptr %20, i64 8
  store i64 %.sroa.6.0.copyload, ptr %771, align 8, !alias.scope !23166, !noalias !23168
  %772 = getelementptr inbounds nuw i8, ptr %20, i64 16
  store i64 %769, ptr %772, align 8, !alias.scope !23166, !noalias !23168
  %773 = getelementptr inbounds nuw i8, ptr %20, i64 208
  store i64 0, ptr %773, align 8, !alias.scope !23169, !noalias !23170
  %774 = getelementptr inbounds nuw i8, ptr %20, i64 216
  store i64 %768, ptr %774, align 8, !alias.scope !23169, !noalias !23170
; invoke <purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %21, ptr noalias nofree noundef align 8 captures(address) dereferenceable(224) %20)
          to label %775 unwind label %941, !noalias !23168

775:                                              ; preds = %764
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !23162
  %776 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %777 = load ptr, ptr %776, align 8, !noalias !23162, !nonnull !1740, !noundef !1740
  %778 = getelementptr inbounds nuw i8, ptr %21, i64 16
  %779 = load i64, ptr %778, align 8, !noalias !23162, !noundef !1740
  %780 = mul nuw nsw i64 %779, 200
  %781 = getelementptr inbounds nuw i8, ptr %777, i64 %780
  %782 = icmp eq i64 %779, 0
  br i1 %782, label %850, label %.preheader103.preheader

.preheader103.preheader:                          ; preds = %775
  %xtraiter = and i64 %779, 3
  %783 = icmp ult i64 %779, 4
  br i1 %783, label %.preheader103.epil.preheader, label %.preheader103.preheader.new

.preheader103.preheader.new:                      ; preds = %.preheader103.preheader
  %unroll_iter = and i64 %779, -4
  br label %.preheader103

.preheader103:                                    ; preds = %828, %.preheader103.preheader.new
  %784 = phi i64 [ 0, %.preheader103.preheader.new ], [ %831, %828 ]
  %785 = phi i64 [ 0, %.preheader103.preheader.new ], [ %830, %828 ]
  %niter = phi i64 [ 0, %.preheader103.preheader.new ], [ %niter.next.3, %828 ]
  %786 = getelementptr inbounds nuw [200 x i8], ptr %777, i64 %784
  %787 = getelementptr i8, ptr %786, i64 168
  %788 = load i64, ptr %787, align 8, !noalias !23168, !noundef !1740
  %789 = getelementptr i8, ptr %786, i64 176
  %790 = load i64, ptr %789, align 8, !noalias !23168, !noundef !1740
  %791 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %790, i64 %788)
  %792 = extractvalue { i64, i1 } %791, 0
  %793 = extractvalue { i64, i1 } %791, 1
  br i1 %793, label %794, label %.preheader103.1, !prof !1742

794:                                              ; preds = %.preheader103
  br label %.preheader103.1

.preheader103.1:                                  ; preds = %794, %.preheader103
  %795 = phi i64 [ -1, %794 ], [ %792, %.preheader103 ]
  %796 = call noundef i64 @llvm.uadd.sat.i64(i64 %785, i64 %795)
  %797 = getelementptr inbounds nuw [200 x i8], ptr %777, i64 %784
  %798 = getelementptr i8, ptr %797, i64 368
  %799 = load i64, ptr %798, align 8, !noalias !23168, !noundef !1740
  %800 = getelementptr i8, ptr %797, i64 376
  %801 = load i64, ptr %800, align 8, !noalias !23168, !noundef !1740
  %802 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %801, i64 %799)
  %803 = extractvalue { i64, i1 } %802, 0
  %804 = extractvalue { i64, i1 } %802, 1
  br i1 %804, label %805, label %.preheader103.2, !prof !1742

805:                                              ; preds = %.preheader103.1
  br label %.preheader103.2

.preheader103.2:                                  ; preds = %805, %.preheader103.1
  %806 = phi i64 [ -1, %805 ], [ %803, %.preheader103.1 ]
  %807 = call noundef i64 @llvm.uadd.sat.i64(i64 %796, i64 %806)
  %808 = getelementptr inbounds nuw [200 x i8], ptr %777, i64 %784
  %809 = getelementptr i8, ptr %808, i64 568
  %810 = load i64, ptr %809, align 8, !noalias !23168, !noundef !1740
  %811 = getelementptr i8, ptr %808, i64 576
  %812 = load i64, ptr %811, align 8, !noalias !23168, !noundef !1740
  %813 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %812, i64 %810)
  %814 = extractvalue { i64, i1 } %813, 0
  %815 = extractvalue { i64, i1 } %813, 1
  br i1 %815, label %816, label %.preheader103.3, !prof !1742

816:                                              ; preds = %.preheader103.2
  br label %.preheader103.3

.preheader103.3:                                  ; preds = %816, %.preheader103.2
  %817 = phi i64 [ -1, %816 ], [ %814, %.preheader103.2 ]
  %818 = call noundef i64 @llvm.uadd.sat.i64(i64 %807, i64 %817)
  %819 = getelementptr inbounds nuw [200 x i8], ptr %777, i64 %784
  %820 = getelementptr i8, ptr %819, i64 768
  %821 = load i64, ptr %820, align 8, !noalias !23168, !noundef !1740
  %822 = getelementptr i8, ptr %819, i64 776
  %823 = load i64, ptr %822, align 8, !noalias !23168, !noundef !1740
  %824 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %823, i64 %821)
  %825 = extractvalue { i64, i1 } %824, 0
  %826 = extractvalue { i64, i1 } %824, 1
  br i1 %826, label %827, label %828, !prof !1742

827:                                              ; preds = %.preheader103.3
  br label %828

828:                                              ; preds = %827, %.preheader103.3
  %829 = phi i64 [ -1, %827 ], [ %825, %.preheader103.3 ]
  %830 = call noundef i64 @llvm.uadd.sat.i64(i64 %818, i64 %829)
  %831 = add nuw i64 %784, 4
  %niter.next.3 = add i64 %niter, 4
  %niter.ncmp.3 = icmp eq i64 %niter.next.3, %unroll_iter
  br i1 %niter.ncmp.3, label %.unr-lcssa, label %.preheader103

832:                                              ; preds = %850
  %833 = landingpad { ptr, i32 }
          cleanup
  br label %945

.unr-lcssa:                                       ; preds = %828
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.epilog-lcssa, label %.preheader103.epil.preheader

.preheader103.epil.preheader:                     ; preds = %.unr-lcssa, %.preheader103.preheader
  %.epil.init = phi i64 [ 0, %.preheader103.preheader ], [ %831, %.unr-lcssa ]
  %.epil.init541 = phi i64 [ 0, %.preheader103.preheader ], [ %830, %.unr-lcssa ]
  %lcmp.mod543 = icmp ne i64 %xtraiter, 0
  call void @llvm.assume(i1 %lcmp.mod543)
  br label %.preheader103.epil

.preheader103.epil:                               ; preds = %845, %.preheader103.epil.preheader
  %834 = phi i64 [ %848, %845 ], [ %.epil.init, %.preheader103.epil.preheader ]
  %835 = phi i64 [ %847, %845 ], [ %.epil.init541, %.preheader103.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %845 ], [ 0, %.preheader103.epil.preheader ]
  %836 = getelementptr inbounds nuw [200 x i8], ptr %777, i64 %834
  %837 = getelementptr i8, ptr %836, i64 168
  %838 = load i64, ptr %837, align 8, !noalias !23168, !noundef !1740
  %839 = getelementptr i8, ptr %836, i64 176
  %840 = load i64, ptr %839, align 8, !noalias !23168, !noundef !1740
  %841 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %840, i64 %838)
  %842 = extractvalue { i64, i1 } %841, 0
  %843 = extractvalue { i64, i1 } %841, 1
  br i1 %843, label %844, label %845, !prof !1742

844:                                              ; preds = %.preheader103.epil
  br label %845

845:                                              ; preds = %844, %.preheader103.epil
  %846 = phi i64 [ -1, %844 ], [ %842, %.preheader103.epil ]
  %847 = call noundef i64 @llvm.uadd.sat.i64(i64 %835, i64 %846)
  %848 = add nuw i64 %834, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.epilog-lcssa, label %.preheader103.epil, !llvm.loop !23171

.epilog-lcssa:                                    ; preds = %845, %.unr-lcssa
  %.lcssa499 = phi i64 [ %830, %.unr-lcssa ], [ %847, %845 ]
  %849 = icmp eq i64 %.lcssa499, 0
  br i1 %849, label %850, label %856

850:                                              ; preds = %860, %856, %.epilog-lcssa, %775
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !23162
  %851 = load i64, ptr %21, align 8, !range !1835, !noalias !23162, !noundef !1740
  %852 = icmp ult i64 %779, 46116860184273880
  call void @llvm.assume(i1 %852)
  store ptr %777, ptr %19, align 8, !noalias !23162
  %853 = getelementptr inbounds nuw i8, ptr %19, i64 8
  store ptr %777, ptr %853, align 8, !noalias !23162
  %854 = getelementptr inbounds nuw i8, ptr %19, i64 16
  store i64 %851, ptr %854, align 8, !noalias !23162
  %855 = getelementptr inbounds nuw i8, ptr %19, i64 24
  store ptr %781, ptr %855, align 8, !noalias !23162
; invoke <purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %16, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %19)
          to label %863 unwind label %832

856:                                              ; preds = %.epilog-lcssa
  %857 = getelementptr inbounds nuw i8, ptr %755, i64 336
  %858 = load ptr, ptr %857, align 8, !noalias !23168, !noundef !1740
  %859 = icmp eq ptr %858, null
  br i1 %859, label %850, label %860

860:                                              ; preds = %856
  %861 = getelementptr inbounds nuw i8, ptr %755, i64 352
  %862 = atomicrmw add ptr %861, i64 %.lcssa499 monotonic, align 8, !noalias !23168
  br label %850

863:                                              ; preds = %850
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !23162
  %864 = getelementptr inbounds nuw i8, ptr %71, i64 193
  %865 = load i8, ptr %864, align 1, !range !1747, !alias.scope !23154, !noalias !23157, !noundef !1740
  %866 = zext nneg i8 %865 to i64
  %867 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %868 = load ptr, ptr %867, align 8, !nonnull !1740, !noundef !1740
  %869 = getelementptr inbounds nuw i8, ptr %16, i64 16
  %870 = load i64, ptr %869, align 8, !noundef !1740
  %871 = icmp eq i64 %870, 0
  br i1 %871, label %.loopexit102, label %iter.check

iter.check:                                       ; preds = %863
  %min.iters.check = icmp ult i64 %870, 8
  br i1 %min.iters.check, label %.preheader101.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check441 = icmp ult i64 %870, 32
  br i1 %min.iters.check441, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %870, 24
  %n.vec = and i64 %870, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %872, %vector.body ]
  %vec.phi442 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %873, %vector.body ]
  %vec.phi443 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %874, %vector.body ]
  %vec.phi444 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %875, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %868, <8 x i64> %vec.ind
  %wide.gep445 = getelementptr inbounds nuw [160 x i8], ptr %868, <8 x i64> %step.add
  %wide.gep446 = getelementptr inbounds nuw [160 x i8], ptr %868, <8 x i64> %step.add.2
  %wide.gep447 = getelementptr inbounds nuw [160 x i8], ptr %868, <8 x i64> %step.add.3
  %wide.gep448 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep449 = getelementptr i8, <8 x ptr> %wide.gep445, i64 64
  %wide.gep450 = getelementptr i8, <8 x ptr> %wide.gep446, i64 64
  %wide.gep451 = getelementptr i8, <8 x ptr> %wide.gep447, i64 64
  %wide.masked.gather = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep448, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23172
  %wide.masked.gather452 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep449, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23172
  %wide.masked.gather453 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep450, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23172
  %wide.masked.gather454 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep451, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23172
  %872 = add <8 x i64> %wide.masked.gather, %vec.phi
  %873 = add <8 x i64> %wide.masked.gather452, %vec.phi442
  %874 = add <8 x i64> %wide.masked.gather453, %vec.phi443
  %875 = add <8 x i64> %wide.masked.gather454, %vec.phi444
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %876 = icmp eq i64 %index.next, %n.vec
  br i1 %876, label %middle.block, label %vector.body, !llvm.loop !23175

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %873, %872
  %bin.rdx455 = add <8 x i64> %874, %bin.rdx
  %bin.rdx456 = add <8 x i64> %875, %bin.rdx455
  %877 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx456)
  %cmp.n = icmp eq i64 %870, %n.vec
  br i1 %cmp.n, label %.loopexit102, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader101.preheader, label %vec.epilog.ph, !prof !11074

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %877, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec458 = and i64 %870, -8
  %878 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index459 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next465, %vec.epilog.vector.body ]
  %vec.ind460 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next466, %vec.epilog.vector.body ]
  %vec.phi461 = phi <8 x i64> [ %878, %vec.epilog.ph ], [ %879, %vec.epilog.vector.body ]
  %wide.gep462 = getelementptr inbounds nuw [160 x i8], ptr %868, <8 x i64> %vec.ind460
  %wide.gep463 = getelementptr i8, <8 x ptr> %wide.gep462, i64 64
  %wide.masked.gather464 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep463, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23172
  %879 = add <8 x i64> %wide.masked.gather464, %vec.phi461
  %index.next465 = add nuw i64 %index459, 8
  %vec.ind.next466 = add nuw <8 x i64> %vec.ind460, splat (i64 8)
  %880 = icmp eq i64 %index.next465, %n.vec458
  br i1 %880, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !23176

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %881 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %879)
  %cmp.n467 = icmp eq i64 %870, %n.vec458
  br i1 %cmp.n467, label %.loopexit102, label %.preheader101.preheader

.preheader101.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec458, %vec.epilog.middle.block ]
  %.ph492 = phi i64 [ 0, %iter.check ], [ %877, %vec.epilog.iter.check ], [ %881, %vec.epilog.middle.block ]
  br label %.preheader101

.preheader101:                                    ; preds = %.preheader101.preheader, %.preheader101
  %882 = phi i64 [ %889, %.preheader101 ], [ %.ph, %.preheader101.preheader ]
  %883 = phi i64 [ %888, %.preheader101 ], [ %.ph492, %.preheader101.preheader ]
  %884 = getelementptr inbounds nuw [160 x i8], ptr %868, i64 %882
  %885 = getelementptr i8, ptr %884, i64 64
  %886 = load i64, ptr %885, align 8, !noalias !23172, !noundef !1740
  %887 = icmp ult i64 %886, 288230376151711744
  call void @llvm.assume(i1 %887), !noalias !23168
  %888 = add i64 %886, %883
  %889 = add nuw i64 %882, 1
  %890 = icmp eq i64 %889, %870
  br i1 %890, label %.loopexit102, label %.preheader101, !llvm.loop !23177

891:                                              ; preds = %.loopexit102
  %892 = landingpad { ptr, i32 }
          cleanup
  br label %1437

.loopexit102:                                     ; preds = %.preheader101, %middle.block, %vec.epilog.middle.block, %863
  %893 = phi i64 [ 0, %863 ], [ %881, %vec.epilog.middle.block ], [ %877, %middle.block ], [ %888, %.preheader101 ]
  %894 = getelementptr inbounds nuw i8, ptr %16, i64 24
  %895 = load i8, ptr %894, align 8, !range !1747, !noundef !1740
  %896 = trunc nuw i8 %895 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %18)
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !23162
  %897 = getelementptr inbounds nuw i8, ptr %71, i64 195
  %898 = load i8, ptr %897, align 1, !range !6143, !alias.scope !23154, !noalias !23157, !noundef !1740
; invoke purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %17, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, i8 noundef %898, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %16, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %15, i64 noundef %866)
          to label %899 unwind label %891

899:                                              ; preds = %.loopexit102
  %900 = load i64, ptr %17, align 16, !range !2527, !noalias !23162, !noundef !1740
  %901 = icmp eq i64 %900, -1
  %902 = getelementptr inbounds nuw i8, ptr %17, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %18, ptr noundef nonnull align 8 dereferenceable(24) %902, i64 24, i1 false), !noalias !23162
  %903 = getelementptr inbounds nuw i8, ptr %17, i64 32
  %904 = load i8, ptr %903, align 16, !noalias !23162
  br i1 %901, label %911, label %905

905:                                              ; preds = %899
  %906 = getelementptr inbounds nuw i8, ptr %17, i64 33
  %907 = load i56, ptr %906, align 1, !noalias !23162
  %908 = getelementptr inbounds nuw i8, ptr %17, i64 40
  %909 = load i64, ptr %908, align 8, !noalias !23162
  %910 = getelementptr inbounds nuw i8, ptr %17, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %48, ptr noundef nonnull align 16 dereferenceable(48) %910, i64 48, i1 false), !noalias !23178
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !23162
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %47, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false), !noalias !23178
  call void @llvm.lifetime.end.p0(ptr nonnull %18)
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !23162
  br label %975

911:                                              ; preds = %899
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !23162
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !23162
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %14, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false), !noalias !23162
  call void @llvm.lifetime.end.p0(ptr nonnull %18)
; invoke purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>
  %912 = invoke fastcc { i64, i64 } @purrdf_sparql_eval::row_checkpoint::settle_commit::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 dereferenceable(1248) %5, i8 noundef %904, i1 noundef zeroext %896, i64 noundef %893)
          to label %915 unwind label %913, !noalias !23179

913:                                              ; preds = %911
  %914 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %14) #89, !noalias !23179
  br label %1437

915:                                              ; preds = %911
  %916 = extractvalue { i64, i64 } %912, 0
  %917 = extractvalue { i64, i64 } %912, 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %47, ptr noundef nonnull align 8 dereferenceable(24) %14, i64 24, i1 false), !noalias !23178
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !23162
  %918 = trunc nuw nsw i64 %916 to i8
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !23162
  br label %989

919:                                              ; preds = %945, %943
  %920 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23180
  unreachable

921:                                              ; preds = %761
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !23162
  %922 = load i64, ptr %23, align 16, !range !2527, !noalias !23162, !noundef !1740
  %923 = icmp eq i64 %922, -1
  %924 = getelementptr inbounds nuw i8, ptr %23, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %24, ptr noundef nonnull align 8 dereferenceable(24) %924, i64 24, i1 false), !noalias !23162
  br i1 %923, label %934, label %925

925:                                              ; preds = %921
  %926 = getelementptr inbounds nuw i8, ptr %23, i64 32
  %927 = load i64, ptr %926, align 16, !noalias !23162
  %928 = getelementptr inbounds nuw i8, ptr %23, i64 40
  %929 = load i64, ptr %928, align 8, !noalias !23162
  %930 = getelementptr inbounds nuw i8, ptr %23, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %48, ptr noundef nonnull align 16 dereferenceable(48) %930, i64 48, i1 false), !noalias !23178
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !23162
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %47, ptr noundef nonnull align 8 dereferenceable(24) %24, i64 24, i1 false), !noalias !23178
  %931 = trunc i64 %927 to i8
  %932 = lshr i64 %927, 8
  %933 = trunc nuw i64 %932 to i56
  br label %935

934:                                              ; preds = %921
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !23162
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %47, ptr noundef nonnull align 8 dereferenceable(24) %24, i64 24, i1 false), !noalias !23178
  br label %935

935:                                              ; preds = %934, %925
  %936 = phi i56 [ 0, %934 ], [ %933, %925 ]
  %937 = phi i8 [ 0, %934 ], [ %931, %925 ]
  %938 = phi i64 [ undef, %934 ], [ %929, %925 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(208) %51)
          to label %947 unwind label %939

939:                                              ; preds = %935
  %940 = landingpad { ptr, i32 }
          cleanup
  br label %1437

941:                                              ; preds = %764
  %942 = landingpad { ptr, i32 }
          cleanup
  br label %945

943:                                              ; preds = %761
  %944 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(208) %51) #89
          to label %1437 unwind label %919

945:                                              ; preds = %941, %832
  %946 = phi { ptr, i32 } [ %942, %941 ], [ %833, %832 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %15) #89
          to label %1437 unwind label %919

947:                                              ; preds = %935
  br i1 %923, label %989, label %975

948:                                              ; preds = %753
  call void @llvm.lifetime.start.p0(ptr nonnull %54)
  call void @llvm.lifetime.start.p0(ptr nonnull %52)
  call void @llvm.lifetime.start.p0(ptr nonnull %53)
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %50, ptr noundef nonnull align 8 dereferenceable(32) %77, i64 24, i1 false), !noalias !23059
  store i64 0, ptr %77, align 8, !alias.scope !23043, !noalias !23059
  store ptr inttoptr (i64 8 to ptr), ptr %674, align 8, !alias.scope !23043, !noalias !23059
  store i64 0, ptr %207, align 8, !alias.scope !23043, !noalias !23059
  call void @llvm.experimental.noalias.scope.decl(metadata !23181)
  %949 = load i8, ptr %757, align 2, !range !3730, !alias.scope !23181, !noalias !23184, !noundef !1740
  %950 = icmp eq i8 %949, 0
  br i1 %950, label %952, label %951, !prof !11429

951:                                              ; preds = %948
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.e5162873a9a3251d11c4df37a70e4654.152, ptr noundef nonnull inttoptr (i64 83 to ptr), ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.154) #93
          to label %967 unwind label %970, !noalias !23190

952:                                              ; preds = %948
  call void @llvm.lifetime.start.p0(ptr nonnull %27)
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !23191
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !23191
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %25, ptr noundef nonnull align 8 dereferenceable(24) %50, i64 24, i1 false), !noalias !23192
; invoke purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::admit_rows::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_sparql_eval::expr::eval_extend_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#3}>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %26, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %15, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %25)
          to label %955 unwind label %953

953:                                              ; preds = %952
  %954 = landingpad { ptr, i32 }
          cleanup
  br label %968

955:                                              ; preds = %952
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !23191
  %956 = load i64, ptr %26, align 16, !range !2527, !noalias !23191, !noundef !1740
  %957 = icmp eq i64 %956, -1
  %958 = getelementptr inbounds nuw i8, ptr %26, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %27, ptr noundef nonnull align 8 dereferenceable(24) %958, i64 24, i1 false), !noalias !23191
  br i1 %957, label %963, label %959

959:                                              ; preds = %955
  %960 = getelementptr inbounds nuw i8, ptr %26, i64 32
  %961 = load <2 x i64>, ptr %960, align 16, !noalias !23191
  %962 = getelementptr inbounds nuw i8, ptr %26, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %53, ptr noundef nonnull align 16 dereferenceable(48) %962, i64 48, i1 false), !noalias !23193
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !23191
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %52, ptr noundef nonnull align 8 dereferenceable(24) %27, i64 24, i1 false), !noalias !23193
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(208) %51)
          to label %1031 unwind label %965

963:                                              ; preds = %955
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !23191
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %52, ptr noundef nonnull align 8 dereferenceable(24) %27, i64 24, i1 false), !noalias !23193
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(208) %51)
          to label %1036 unwind label %965

964:                                              ; preds = %968
  br i1 %950, label %1437, label %974

965:                                              ; preds = %963, %959
  %966 = landingpad { ptr, i32 }
          cleanup
  br label %1437

967:                                              ; preds = %951
  unreachable

968:                                              ; preds = %970, %953
  %969 = phi { ptr, i32 } [ %971, %970 ], [ %954, %953 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(208) %51) #89
          to label %964 unwind label %972

970:                                              ; preds = %951
  %971 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %50) #89, !noalias !23194
  br label %968

972:                                              ; preds = %974, %968
  %973 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23195
  unreachable

974:                                              ; preds = %964
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %15) #89
          to label %1437 unwind label %972

975:                                              ; preds = %947, %905
  %976 = phi i64 [ %900, %905 ], [ %922, %947 ]
  %977 = phi i64 [ %909, %905 ], [ %938, %947 ]
  %978 = phi i8 [ %904, %905 ], [ %937, %947 ]
  %979 = phi i56 [ %907, %905 ], [ %936, %947 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %49, ptr noundef nonnull align 8 dereferenceable(24) %47, i64 24, i1 false), !noalias !23057
  %980 = zext i56 %979 to i64
  %981 = shl nuw i64 %980, 8
  %982 = zext i8 %978 to i64
  %983 = or disjoint i64 %981, %982
  %984 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %984, ptr noundef nonnull align 16 dereferenceable(48) %48, i64 48, i1 false), !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  %985 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %985, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false), !noalias !23101
  %986 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %976, ptr %986, align 16, !alias.scope !23038, !noalias !23101
  %987 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %983, ptr %987, align 16, !alias.scope !23038, !noalias !23101
  %988 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i64 %977, ptr %988, align 8, !alias.scope !23038, !noalias !23101
  store i64 1, ptr %0, align 16, !alias.scope !23038, !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  br label %1030

989:                                              ; preds = %947, %915
  %990 = phi i64 [ %917, %915 ], [ %938, %947 ]
  %991 = phi i8 [ %918, %915 ], [ %937, %947 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %49, ptr noundef nonnull align 8 dereferenceable(24) %47, i64 24, i1 false), !noalias !23057
  %992 = trunc i8 %991 to i1
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  call void @llvm.lifetime.end.p0(ptr nonnull %48)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %28, ptr noundef nonnull align 8 dereferenceable(24) %49, i64 24, i1 false), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  br label %993

993:                                              ; preds = %1036, %989
  %994 = phi ptr [ inttoptr (i64 8 to ptr), %1036 ], [ %675, %989 ]
  %995 = phi i64 [ 0, %1036 ], [ %208, %989 ]
  %996 = phi i1 [ false, %1036 ], [ %992, %989 ]
  %997 = phi i64 [ undef, %1036 ], [ %990, %989 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %55), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %55, ptr noundef nonnull align 8 dereferenceable(24) %28, i64 24, i1 false), !noalias !23057
  %998 = load i64, ptr %58, align 8, !noalias !23057
  %999 = getelementptr inbounds nuw i8, ptr %58, i64 8
  %1000 = load i64, ptr %999, align 8, !noalias !23057
  %1001 = getelementptr inbounds nuw i8, ptr %58, i64 16
  %1002 = load i64, ptr %1001, align 8, !noalias !23057
  %1003 = getelementptr inbounds nuw i8, ptr %58, i64 24
  %1004 = load i64, ptr %1003, align 8, !noalias !23057
  %1005 = icmp ugt i64 %998, 2
  %1006 = select i1 %1005, i64 %1002, i64 %998
  %1007 = add i64 %1006, -1
  %1008 = select i1 %1005, i64 %998, i64 1
  %1009 = select i1 %1005, i64 1, i64 %1002
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !23196
  store i64 %1008, ptr %12, align 8, !noalias !23200
  %1010 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store i64 %1000, ptr %1010, align 8, !noalias !23200
  %1011 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %1009, ptr %1011, align 8, !noalias !23200
  %1012 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store i64 %1004, ptr %1012, align 8, !noalias !23200
  %1013 = getelementptr inbounds nuw i8, ptr %12, i64 32
  store i64 0, ptr %1013, align 8, !noalias !23196
  %1014 = getelementptr inbounds nuw i8, ptr %12, i64 40
  store i64 %1007, ptr %1014, align 8, !noalias !23196
  %1015 = icmp eq i64 %1007, 0
  br i1 %1015, label %.loopexit100, label %1016

1016:                                             ; preds = %993
  %1017 = inttoptr i64 %1000 to ptr
  %1018 = select i1 %1005, ptr %1017, ptr %1010
  %1019 = getelementptr inbounds nuw i8, ptr %5, i64 640
  br label %1022

1020:                                             ; preds = %1022
  %1021 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %12) #89
          to label %1037 unwind label %1028, !noalias !23201

1022:                                             ; preds = %1026, %1016
  %1023 = phi i64 [ 0, %1016 ], [ %1024, %1026 ]
  %1024 = add nuw i64 %1023, 1
  store i64 %1024, ptr %1013, align 8, !alias.scope !23202, !noalias !23205
  %1025 = getelementptr inbounds nuw [24 x i8], ptr %1018, i64 %1023
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !23196
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %11, ptr noundef nonnull align 8 dereferenceable(24) %1025, i64 24, i1 false), !noalias !23201
; invoke <purrdf_sparql_eval::witness::RelationWitness>::merge
  invoke void @<purrdf_sparql_eval::witness::RelationWitness>::merge(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1019, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %11)
          to label %1026 unwind label %1020, !noalias !23201

.loopexit100:                                     ; preds = %1026, %993
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %12)
          to label %1041 unwind label %1039

1026:                                             ; preds = %1022
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !23196
  %1027 = icmp eq i64 %1024, %1007
  br i1 %1027, label %.loopexit100, label %1022

1028:                                             ; preds = %1020
  %1029 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !23201
  unreachable

1030:                                             ; preds = %1031, %975
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %58)
          to label %1203 unwind label %751, !noalias !23050

1031:                                             ; preds = %959
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %54, ptr noundef nonnull align 8 dereferenceable(24) %52, i64 24, i1 false), !noalias !23057
  %1032 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %1032, ptr noundef nonnull align 16 dereferenceable(48) %53, i64 48, i1 false), !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %52)
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  %1033 = getelementptr inbounds nuw i8, ptr %0, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1033, ptr noundef nonnull align 8 dereferenceable(24) %54, i64 24, i1 false), !noalias !23101
  %1034 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %956, ptr %1034, align 16, !alias.scope !23038, !noalias !23101
  %1035 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store <2 x i64> %961, ptr %1035, align 16, !alias.scope !23038, !noalias !23101
  store i64 1, ptr %0, align 16, !alias.scope !23038, !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  br label %1030

1036:                                             ; preds = %963
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %54, ptr noundef nonnull align 8 dereferenceable(24) %52, i64 24, i1 false), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %52)
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %28, ptr noundef nonnull align 8 dereferenceable(24) %54, i64 24, i1 false), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  br label %993

1037:                                             ; preds = %1068, %1039, %1020
  %1038 = phi { ptr, i32 } [ %1021, %1020 ], [ %1040, %1039 ], [ %1069, %1068 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %55) #89, !noalias !23050
  br label %695

1039:                                             ; preds = %.loopexit99, %1201, %1053, %1051, %.loopexit100
  %1040 = landingpad { ptr, i32 }
          cleanup
  br label %1037

1041:                                             ; preds = %.loopexit100
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !23196
  br i1 %996, label %1042, label %1044

1042:                                             ; preds = %1041
  %1043 = icmp ugt i64 %997, %995
  br i1 %1043, label %1053, label %1051, !prof !1742

1044:                                             ; preds = %1216, %1041
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %65, ptr noundef nonnull align 8 dereferenceable(24) %55, i64 24, i1 false), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %58), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23207)
  %1045 = load ptr, ptr %63, align 8, !alias.scope !23207, !noalias !23050, !noundef !1740
  %1046 = icmp eq ptr %1045, null
  br i1 %1046, label %1217, label %1047

1047:                                             ; preds = %1044
  %1048 = atomicrmw sub ptr %1045, i64 1 release, align 8, !noalias !23210
  %1049 = icmp eq i64 %1048, 1
  br i1 %1049, label %1050, label %1217

1050:                                             ; preds = %1047
  fence acquire, !noalias !23050
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %63) #91
          to label %1217 unwind label %344, !inline_history !2025

1051:                                             ; preds = %1042
  %1052 = sub nuw nsw i64 %995, %997
  call void @llvm.lifetime.start.p0(ptr nonnull %46), !noalias !23057
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %46, ptr noundef nonnull align 16 dereferenceable(1248) %5, i1 noundef zeroext false, i64 noundef %1052)
          to label %1054 unwind label %1039

1053:                                             ; preds = %1042
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %997, i64 noundef %995, i64 noundef %995, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.404) #93
          to label %544 unwind label %1039, !noalias !23050

1054:                                             ; preds = %1051
  %1055 = getelementptr inbounds nuw [40 x i8], ptr %994, i64 %995
  %1056 = icmp samesign eq i64 %997, %995
  br i1 %1056, label %.loopexit99, label %1057

1057:                                             ; preds = %1054
  %1058 = getelementptr inbounds nuw [40 x i8], ptr %994, i64 %997
  %1059 = getelementptr inbounds nuw i8, ptr %44, i64 16
  %1060 = getelementptr inbounds nuw i8, ptr %44, i64 8
  %1061 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %1062 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %1063 = getelementptr inbounds nuw i8, ptr %55, i64 16
  %1064 = getelementptr inbounds nuw i8, ptr %55, i64 8
  br label %1065

1065:                                             ; preds = %1194, %1057
  %1066 = phi ptr [ %1058, %1057 ], [ %1067, %1194 ]
  %1067 = getelementptr inbounds nuw i8, ptr %1066, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !23057
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %45, ptr noalias nofree noundef align 8 dereferenceable(200) %46, ptr noundef nonnull align 16 dereferenceable(1248) %5)
          to label %1072 unwind label %1070, !noalias !23050

1068:                                             ; preds = %1212, %1208, %1190, %1187, %1083, %1079, %1070
  %1069 = phi { ptr, i32 } [ %1080, %1079 ], [ %1188, %1190 ], [ %1071, %1070 ], [ %1080, %1083 ], [ %1188, %1187 ], [ %1209, %1208 ], [ %1209, %1212 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %46)
          to label %1037 unwind label %668

1070:                                             ; preds = %1065
  %1071 = landingpad { ptr, i32 }
          cleanup
  br label %1068

1072:                                             ; preds = %1065
  %1073 = load i8, ptr %45, align 8, !range !1743, !noalias !23057, !noundef !1740
  %1074 = icmp eq i8 %1073, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !23057
  br i1 %1074, label %1075, label %.loopexit99

1075:                                             ; preds = %1072
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !23057
  %1076 = load i64, ptr %68, align 8, !noalias !23057, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !23213
  store i64 1, ptr %10, align 8, !noalias !23213
  %1077 = icmp ugt i64 %1076, 4
  br i1 %1077, label %1078, label %1088, !prof !1742

1078:                                             ; preds = %1075
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %10, i64 noundef 0, i64 noundef %1076, i1 noundef zeroext false) #91
          to label %1088 unwind label %1079, !noalias !23213

1079:                                             ; preds = %1078
  %1080 = landingpad { ptr, i32 }
          cleanup
  %1081 = load i64, ptr %10, align 8, !range !1778, !alias.scope !23216, !noalias !23213, !noundef !1740
  %1082 = icmp ugt i64 %1081, 5
  br i1 %1082, label %1083, label %1068

1083:                                             ; preds = %1079
  %1084 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %1085 = load ptr, ptr %1084, align 8, !noalias !23213, !nonnull !1740, !noundef !1740
  %1086 = shl i64 %1081, 3
  %1087 = add i64 %1086, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1085, i64 noundef %1087, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23219
  br label %1068

1088:                                             ; preds = %1078, %1075
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %44, ptr noundef nonnull align 8 dereferenceable(40) %10, i64 40, i1 false), !noalias !23050
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !23213
  %1089 = load i64, ptr %1066, align 8, !range !1778, !noalias !23050, !noundef !1740
  %1090 = add i64 %1089, -1
  %1091 = icmp ugt i64 %1090, 4
  %1092 = getelementptr inbounds nuw i8, ptr %1066, i64 8
  br i1 %1091, label %1093, label %1098

1093:                                             ; preds = %1088
  %1094 = load ptr, ptr %1092, align 8, !noalias !23050, !nonnull !1740, !noundef !1740
  %1095 = getelementptr inbounds nuw i8, ptr %1066, i64 16
  %1096 = load i64, ptr %1095, align 8, !noalias !23050, !noundef !1740
  %1097 = add i64 %1096, -1
  br label %1098

1098:                                             ; preds = %1093, %1088
  %1099 = phi i64 [ %1097, %1093 ], [ %1090, %1088 ]
  %1100 = phi ptr [ %1094, %1093 ], [ %1092, %1088 ]
  %1101 = load i64, ptr %44, align 8, !range !1778, !alias.scope !23222, !noalias !23227, !noundef !1740
  %1102 = add i64 %1101, -1
  %1103 = icmp ugt i64 %1102, 4
  %1104 = load i64, ptr %1059, align 8, !alias.scope !23222, !noalias !23227
  %1105 = add i64 %1104, -1
  %1106 = select i1 %1103, i64 %1105, i64 %1102
  %1107 = call i64 @llvm.umax.i64(i64 %1102, i64 4)
  %1108 = sub i64 %1107, %1106
  %1109 = icmp ult i64 %1108, %1099
  br i1 %1109, label %1110, label %1113, !prof !1742

1110:                                             ; preds = %1098
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %44, i64 noundef %1106, i64 noundef range(i64 0, 1152921504606846976) %1099, i1 noundef zeroext true) #91
          to label %1111 unwind label %1204

1111:                                             ; preds = %1110
  %1112 = load i64, ptr %44, align 8, !range !1778, !alias.scope !23229, !noalias !23227
  br label %1113

1113:                                             ; preds = %1111, %1098
  %1114 = phi i64 [ %1101, %1098 ], [ %1112, %1111 ]
  %1115 = icmp ugt i64 %1114, 5
  %1116 = load ptr, ptr %1060, align 8, !alias.scope !23229, !noalias !23227, !nonnull !1740
  %1117 = select i1 %1115, ptr %1116, ptr %1060
  %1118 = select i1 %1115, ptr %1059, ptr %44
  %1119 = load i64, ptr %1118, align 8, !alias.scope !23229, !noalias !23227, !noundef !1740
  %1120 = getelementptr [8 x i8], ptr %1117, i64 %1119
  %1121 = getelementptr i8, ptr %1120, i64 -8
  %1122 = shl nuw nsw i64 %1099, 3
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 4 %1121, ptr nonnull readonly align 4 %1100, i64 %1122, i1 false), !noalias !23050
  %1123 = add i64 %1119, %1099
  store i64 %1123, ptr %1118, align 8, !alias.scope !23229, !noalias !23227
  %1124 = load i64, ptr %68, align 8, !noalias !23057, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23230)
  %1125 = load i64, ptr %44, align 8, !range !1778, !alias.scope !23230, !noalias !23050, !noundef !1740
  %1126 = add i64 %1125, -1
  %1127 = icmp ugt i64 %1126, 4
  %1128 = load i64, ptr %1059, align 8, !alias.scope !23230, !noalias !23050
  %1129 = add i64 %1128, -1
  %1130 = select i1 %1127, i64 %1129, i64 %1126
  %1131 = icmp ugt i64 %1124, %1130
  br i1 %1131, label %1140, label %1132

1132:                                             ; preds = %1113
  %1133 = icmp ugt i64 %1125, 5
  %1134 = select i1 %1133, i64 %1128, i64 %1125
  %1135 = add i64 %1134, -1
  %1136 = icmp ult i64 %1124, %1135
  br i1 %1136, label %1137, label %1143

1137:                                             ; preds = %1132
  %1138 = select i1 %1133, ptr %1059, ptr %44
  %1139 = add nuw i64 %1124, 1
  store i64 %1139, ptr %1138, align 8, !alias.scope !23233, !noalias !23050
  br label %1143

1140:                                             ; preds = %1113
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !23236
  %1141 = sub nuw i64 %1124, %1130
  store i32 2, ptr %9, align 8, !noalias !23236
  store i64 %1141, ptr %1061, align 8, !noalias !23236
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %44, ptr noalias nofree noundef align 8 captures(address) dereferenceable(16) %9)
          to label %1142 unwind label %1204

1142:                                             ; preds = %1140
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !23236
  br label %1143

1143:                                             ; preds = %1142, %1137, %1132
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !23057
  %1144 = load i64, ptr %44, align 8, !range !1778, !noalias !23057, !noundef !1740
  %1145 = add i64 %1144, -1
  %1146 = icmp ugt i64 %1145, 4
  %1147 = load ptr, ptr %1060, align 8, !noalias !23057, !nonnull !1740
  %1148 = load i64, ptr %1059, align 8, !noalias !23057
  %1149 = add i64 %1148, -1
  %1150 = select i1 %1146, i64 %1149, i64 %1145
  %1151 = select i1 %1146, ptr %1147, ptr %1060
  %1152 = load ptr, ptr %67, align 8, !noalias !23057, !nonnull !1740, !noundef !1740
  %1153 = getelementptr inbounds nuw i8, ptr %1152, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %43, ptr noalias nofree noundef align 8 dereferenceable(216) %66, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %1151, i64 noundef %1150, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1153, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5)
          to label %1154 unwind label %1204, !noalias !23050

1154:                                             ; preds = %1143
  %1155 = load i64, ptr %43, align 16, !range !2527, !noalias !23057, !noundef !1740
  %1156 = icmp eq i64 %1155, -1
  %1157 = load <2 x i32>, ptr %1062, align 8, !noalias !23057
  br i1 %1156, label %1169, label %1158

1158:                                             ; preds = %1154
  %1159 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %1160 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %1160, ptr noundef nonnull align 16 dereferenceable(80) %1159, i64 80, i1 false), !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !23057
  %1161 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %1155, ptr %1161, align 16, !alias.scope !23038, !noalias !23101
  %1162 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store <2 x i32> %1157, ptr %1162, align 8, !alias.scope !23038, !noalias !23101
  store i64 1, ptr %0, align 16, !alias.scope !23038, !noalias !23101
  %1163 = load i64, ptr %44, align 8, !range !1778, !alias.scope !19667, !noundef !1740
  %1164 = icmp ugt i64 %1163, 5
  br i1 %1164, label %1165, label %1201

1165:                                             ; preds = %1158
  %1166 = load ptr, ptr %1060, align 8, !nonnull !1740, !noundef !1740
  %1167 = shl i64 %1163, 3
  %1168 = add i64 %1167, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1166, i64 noundef %1168, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23237
  br label %1201

1169:                                             ; preds = %1154
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !23057
  %1170 = load i64, ptr %44, align 8, !range !1778, !noalias !23057, !noundef !1740
  %1171 = icmp ugt i64 %1170, 5
  %1172 = load i64, ptr %1059, align 8
  %1173 = select i1 %1171, i64 %1172, i64 %1170
  %1174 = add i64 %1173, -1
  %1175 = load i64, ptr %69, align 8, !noalias !23057, !noundef !1740
  %1176 = icmp ult i64 %1175, %1174
  br i1 %1176, label %1177, label %1193

1177:                                             ; preds = %1169
  %1178 = load ptr, ptr %1060, align 8, !noalias !23057, !nonnull !1740
  %1179 = select i1 %1171, ptr %1178, ptr %1060
  %1180 = getelementptr inbounds nuw [8 x i8], ptr %1179, i64 %1175
  store <2 x i32> %1157, ptr %1180, align 4, !noalias !23050
  call void @llvm.lifetime.start.p0(ptr nonnull %42)
  %1181 = load i64, ptr %44, align 8, !noalias !23057
  %1182 = load ptr, ptr %1060, align 8, !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %42, ptr noundef nonnull align 8 dereferenceable(24) %1059, i64 24, i1 false), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23240)
  %1183 = load i64, ptr %1063, align 8, !alias.scope !23240, !noalias !23243, !noundef !1740
  %1184 = load i64, ptr %55, align 8, !range !1835, !alias.scope !23240, !noalias !23243, !noundef !1740
  %1185 = icmp eq i64 %1183, %1184
  br i1 %1185, label %1186, label %1194

1186:                                             ; preds = %1177
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %55)
          to label %1194 unwind label %1187, !noalias !23243

1187:                                             ; preds = %1186
  %1188 = landingpad { ptr, i32 }
          cleanup
  %1189 = icmp ugt i64 %1181, 5
  br i1 %1189, label %1190, label %1068

1190:                                             ; preds = %1187
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1182) ]
  %1191 = shl i64 %1181, 3
  %1192 = add i64 %1191, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1182, i64 noundef %1192, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23245
  br label %1068

1193:                                             ; preds = %1169
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %1175, i64 noundef %1174, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.403) #93
          to label %544 unwind label %1206, !noalias !23050

1194:                                             ; preds = %1186, %1177
  %1195 = load ptr, ptr %1064, align 8, !alias.scope !23240, !noalias !23243, !nonnull !1740, !noundef !1740
  %1196 = getelementptr inbounds nuw [40 x i8], ptr %1195, i64 %1183
  store i64 %1181, ptr %1196, align 8, !noalias !23248
  %1197 = getelementptr inbounds nuw i8, ptr %1196, i64 8
  store ptr %1182, ptr %1197, align 8, !noalias !23248
  %1198 = getelementptr inbounds nuw i8, ptr %1196, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1198, ptr noundef nonnull align 8 dereferenceable(24) %42, i64 24, i1 false), !noalias !23248
  %1199 = add i64 %1183, 1
  store i64 %1199, ptr %1063, align 8, !alias.scope !23240, !noalias !23243
  call void @llvm.lifetime.end.p0(ptr nonnull %42)
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !23057
  %1200 = icmp eq ptr %1067, %1055
  br i1 %1200, label %.loopexit99, label %1065

1201:                                             ; preds = %1165, %1158
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !23057
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %46)
          to label %1202 unwind label %1039

1202:                                             ; preds = %1201
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !23057
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %55), !noalias !23050
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !23057
  br label %1203

1203:                                             ; preds = %1202, %1030
  call void @llvm.lifetime.end.p0(ptr nonnull %58), !noalias !23057
  br label %1430

1204:                                             ; preds = %1143, %1140, %1110
  %1205 = landingpad { ptr, i32 }
          cleanup
  br label %1208

1206:                                             ; preds = %1193
  %1207 = landingpad { ptr, i32 }
          cleanup
  br label %1208

1208:                                             ; preds = %1206, %1204
  %1209 = phi { ptr, i32 } [ %1205, %1204 ], [ %1207, %1206 ]
  %1210 = load i64, ptr %44, align 8, !range !1778, !alias.scope !19667, !noundef !1740
  %1211 = icmp ugt i64 %1210, 5
  br i1 %1211, label %1212, label %1068

1212:                                             ; preds = %1208
  %1213 = load ptr, ptr %1060, align 8, !nonnull !1740, !noundef !1740
  %1214 = shl i64 %1210, 3
  %1215 = add i64 %1214, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1213, i64 noundef %1215, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !23249
  br label %1068

.loopexit99:                                      ; preds = %1194, %1072, %1054
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %46)
          to label %1216 unwind label %1039

1216:                                             ; preds = %.loopexit99
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !23057
  br label %1044

1217:                                             ; preds = %1050, %1047, %1044
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %64), !noalias !23057
  br label %651

1218:                                             ; preds = %1282
  %1219 = landingpad { ptr, i32 }
          cleanup
  br label %340

1220:                                             ; preds = %651
  %1221 = getelementptr inbounds nuw i8, ptr %654, i64 16
  %1222 = load i8, ptr %1221, align 8, !noalias !23252
  %1223 = icmp eq i8 %1222, -1
  br i1 %1223, label %1230, label %1224

1224:                                             ; preds = %1220
  %1225 = getelementptr inbounds nuw i8, ptr %654, i64 17
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !23057
  store i8 %1222, ptr %34, align 8, !noalias !23057
  %1226 = getelementptr inbounds nuw i8, ptr %34, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %1226, ptr noundef nonnull align 1 dereferenceable(23) %1225, i64 23, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !23057
  %1227 = load ptr, ptr %67, align 8, !noalias !23057, !nonnull !1740, !noundef !1740
  %1228 = atomicrmw add ptr %1227, i64 1 monotonic, align 8, !noalias !23050
  %1229 = icmp slt i64 %1228, 0
  br i1 %1229, label %1285, label %1283

1230:                                             ; preds = %1220, %651
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !23057
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !23057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %31, ptr noundef nonnull align 8 dereferenceable(104) %74, i64 104, i1 false), !noalias !23253
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !23057
  %1231 = load ptr, ptr %67, align 8, !noalias !23057, !nonnull !1740, !noundef !1740
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %30, ptr noundef nonnull align 8 dereferenceable(24) %65, i64 24, i1 false), !noalias !23057
  %1232 = getelementptr inbounds nuw i8, ptr %30, i64 24
  store ptr %1231, ptr %1232, align 8, !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23254)
  call void @llvm.experimental.noalias.scope.decl(metadata !23257)
  call void @llvm.experimental.noalias.scope.decl(metadata !23259)
  %1233 = load i64, ptr %31, align 8, !range !2059, !alias.scope !23257, !noalias !23261, !noundef !1740
  %1234 = icmp eq i64 %1233, -1
  br i1 %1234, label %1236, label %1235

1235:                                             ; preds = %1230
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %32, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %30, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %74), !noalias !23262
  br label %1238

1236:                                             ; preds = %1230
  %1237 = getelementptr inbounds nuw i8, ptr %32, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %1237, ptr noundef nonnull readonly align 8 dereferenceable(32) %30, i64 32, i1 false), !alias.scope !23263, !noalias !23264
  store i64 -1, ptr %32, align 8, !alias.scope !23254, !noalias !23265
  br label %1238

1238:                                             ; preds = %1236, %1235
  %1239 = getelementptr inbounds nuw i8, ptr %31, i64 72
  %1240 = load i64, ptr %1239, align 8, !range !1778, !alias.scope !23266, !noalias !23261, !noundef !1740
  %1241 = icmp ugt i64 %1240, 5
  br i1 %1241, label %1242, label %1275

1242:                                             ; preds = %1238
  %1243 = getelementptr inbounds nuw i8, ptr %31, i64 80
  %1244 = load ptr, ptr %1243, align 8, !alias.scope !23257, !noalias !23261, !nonnull !1740, !noundef !1740
  %1245 = mul i64 %1240, 3
  %1246 = add i64 %1245, -3
  %1247 = load i64, ptr %256, align 8, !noalias !23269, !noundef !1740
  %1248 = call i64 @llvm.umin.i64(i64 %1246, i64 9223372036854775807)
  %1249 = call i64 @llvm.ssub.sat.i64(i64 %1247, i64 %1248)
  store i64 %1249, ptr %256, align 8, !noalias !23269
  %1250 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1251 = load i64, ptr %1250, align 8, !noalias !23269, !noundef !1740
  %1252 = icmp slt i64 %1249, %1251
  br i1 %1252, label %1253, label %.preheader478

1253:                                             ; preds = %1242
  store i64 %1249, ptr %1250, align 8, !noalias !23269
  br label %.preheader478

.preheader478:                                    ; preds = %1253, %1242
  br label %1254

1254:                                             ; preds = %.preheader478, %1257
  %1255 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23269
  %1256 = icmp slt i64 %1255, 0
  br i1 %1256, label %1257, label %__rustc::__rust_dealloc (.exit85)

1257:                                             ; preds = %1254
  %1258 = add nsw i64 %1255, 1
  %1259 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1255, i64 %1258 acq_rel acquire, align 8, !noalias !23269
  %1260 = extractvalue { i64, i1 } %1259, 1
  br i1 %1260, label %1261, label %1254

1261:                                             ; preds = %1257
  %1262 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1248 monotonic, align 8, !noalias !23269
  %1263 = call i64 @llvm.ssub.sat.i64(i64 %1262, i64 %1248)
  %1264 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23269
  br label %1265

1265:                                             ; preds = %1268, %1261
  %1266 = phi i64 [ %1264, %1261 ], [ %1271, %1268 ]
  %1267 = icmp slt i64 %1263, %1266
  br i1 %1267, label %1268, label %1272

1268:                                             ; preds = %1265
  %1269 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1266, i64 %1263 monotonic monotonic, align 8, !noalias !23269
  %1270 = extractvalue { i64, i1 } %1269, 1
  %1271 = extractvalue { i64, i1 } %1269, 0
  br i1 %1270, label %1272, label %1265

1272:                                             ; preds = %1268, %1265
  %1273 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23269
  br label %__rustc::__rust_dealloc (.exit85)

__rustc::__rust_dealloc (.exit85): ; preds = %1254, %1272
  %1274 = icmp ne i64 %1246, 0
  call void @llvm.assume(i1 %1274), !noalias !23269
  call void @free(ptr noundef nonnull %1244) #92, !noalias !23269
  br label %1275

1275:                                             ; preds = %__rustc::__rust_dealloc (.exit85), %1238
  %1276 = getelementptr inbounds nuw i8, ptr %31, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23272), !noalias !23050
  %1277 = load ptr, ptr %1276, align 8, !alias.scope !23275, !noalias !23261, !noundef !1740
  %1278 = icmp eq ptr %1277, null
  br i1 %1278, label %1367, label %1279

1279:                                             ; preds = %1275
  %1280 = atomicrmw sub ptr %1277, i64 1 release, align 8, !noalias !23276
  %1281 = icmp eq i64 %1280, 1
  br i1 %1281, label %1282, label %1367

1282:                                             ; preds = %1279
  fence acquire, !noalias !23050
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1276) #91
          to label %1367 unwind label %1218

1283:                                             ; preds = %1224
  %1284 = load ptr, ptr %67, align 8, !noalias !23057, !nonnull !1740, !noundef !1740
; invoke <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %33, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %34, ptr noundef nonnull %1284)
          to label %1286 unwind label %1428, !noalias !23050

1285:                                             ; preds = %1224
  call void @llvm.trap()
  unreachable

1286:                                             ; preds = %1283
  %1287 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1287, ptr noundef nonnull align 8 dereferenceable(96) %33, i64 96, i1 false), !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !23057
  store i64 0, ptr %0, align 16, !alias.scope !23038, !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23281)
  %1288 = getelementptr inbounds nuw i8, ptr %65, i64 8
  %1289 = load ptr, ptr %1288, align 8, !alias.scope !23281, !noalias !23050, !nonnull !1740, !noundef !1740
  %1290 = getelementptr inbounds nuw i8, ptr %65, i64 16
  %1291 = load i64, ptr %1290, align 8, !alias.scope !23281, !noalias !23050, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23284), !noalias !23050
  %1292 = icmp eq i64 %1291, 0
  br i1 %1292, label %.loopexit98, label %.preheader97

.preheader97:                                     ; preds = %1286
  %1293 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1294

1294:                                             ; preds = %.preheader97, %1332
  %1295 = phi i64 [ %1297, %1332 ], [ 0, %.preheader97 ]
  %1296 = getelementptr inbounds nuw [40 x i8], ptr %1289, i64 %1295
  %1297 = add nuw nsw i64 %1295, 1
  %1298 = load i64, ptr %1296, align 8, !range !1778, !alias.scope !23287, !noalias !23290, !noundef !1740
  %1299 = icmp ugt i64 %1298, 5
  br i1 %1299, label %1300, label %1332

1300:                                             ; preds = %1294
  %1301 = getelementptr i8, ptr %1296, i64 8
  %1302 = load ptr, ptr %1301, align 8, !alias.scope !23284, !noalias !23290, !nonnull !1740, !noundef !1740
  %1303 = shl i64 %1298, 3
  %1304 = add i64 %1303, -8
  %1305 = load i64, ptr %256, align 8, !noalias !23291, !noundef !1740
  %1306 = call i64 @llvm.umin.i64(i64 %1304, i64 9223372036854775807)
  %1307 = call i64 @llvm.ssub.sat.i64(i64 %1305, i64 %1306)
  store i64 %1307, ptr %256, align 8, !noalias !23291
  %1308 = load i64, ptr %1293, align 8, !noalias !23291, !noundef !1740
  %1309 = icmp slt i64 %1307, %1308
  br i1 %1309, label %1310, label %.preheader480

1310:                                             ; preds = %1300
  store i64 %1307, ptr %1293, align 8, !noalias !23291
  br label %.preheader480

.preheader480:                                    ; preds = %1310, %1300
  br label %1311

1311:                                             ; preds = %.preheader480, %1314
  %1312 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23291
  %1313 = icmp slt i64 %1312, 0
  br i1 %1313, label %1314, label %__rustc::__rust_dealloc (.exit86)

1314:                                             ; preds = %1311
  %1315 = add nsw i64 %1312, 1
  %1316 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1312, i64 %1315 acq_rel acquire, align 8, !noalias !23291
  %1317 = extractvalue { i64, i1 } %1316, 1
  br i1 %1317, label %1318, label %1311

1318:                                             ; preds = %1314
  %1319 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1306 monotonic, align 8, !noalias !23291
  %1320 = call i64 @llvm.ssub.sat.i64(i64 %1319, i64 %1306)
  %1321 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23291
  br label %1322

1322:                                             ; preds = %1325, %1318
  %1323 = phi i64 [ %1321, %1318 ], [ %1328, %1325 ]
  %1324 = icmp slt i64 %1320, %1323
  br i1 %1324, label %1325, label %1329

1325:                                             ; preds = %1322
  %1326 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1323, i64 %1320 monotonic monotonic, align 8, !noalias !23291
  %1327 = extractvalue { i64, i1 } %1326, 1
  %1328 = extractvalue { i64, i1 } %1326, 0
  br i1 %1327, label %1329, label %1322

1329:                                             ; preds = %1325, %1322
  %1330 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23291
  br label %__rustc::__rust_dealloc (.exit86)

__rustc::__rust_dealloc (.exit86): ; preds = %1311, %1329
  %1331 = icmp ne i64 %1304, 0
  call void @llvm.assume(i1 %1331), !noalias !23291
  call void @free(ptr noundef nonnull %1302) #92, !noalias !23291
  br label %1332

1332:                                             ; preds = %__rustc::__rust_dealloc (.exit86), %1294
  %1333 = icmp eq i64 %1297, %1291
  br i1 %1333, label %.loopexit98, label %1294

.loopexit98:                                      ; preds = %1332, %1286
  %1334 = load i64, ptr %65, align 8, !alias.scope !23281, !noalias !23050
  %1335 = icmp eq i64 %1334, 0
  br i1 %1335, label %1365, label %1336

1336:                                             ; preds = %.loopexit98
  %1337 = mul nuw i64 %1334, 40
  %1338 = load i64, ptr %256, align 8, !noalias !23290, !noundef !1740
  %1339 = call i64 @llvm.umin.i64(i64 %1337, i64 9223372036854775807)
  %1340 = call i64 @llvm.ssub.sat.i64(i64 %1338, i64 %1339)
  store i64 %1340, ptr %256, align 8, !noalias !23290
  %1341 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1342 = load i64, ptr %1341, align 8, !noalias !23290, !noundef !1740
  %1343 = icmp slt i64 %1340, %1342
  br i1 %1343, label %1344, label %.preheader479

1344:                                             ; preds = %1336
  store i64 %1340, ptr %1341, align 8, !noalias !23290
  br label %.preheader479

.preheader479:                                    ; preds = %1344, %1336
  br label %1345

1345:                                             ; preds = %.preheader479, %1348
  %1346 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23290
  %1347 = icmp slt i64 %1346, 0
  br i1 %1347, label %1348, label %__rustc::__rust_dealloc (.exit87)

1348:                                             ; preds = %1345
  %1349 = add nsw i64 %1346, 1
  %1350 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1346, i64 %1349 acq_rel acquire, align 8, !noalias !23290
  %1351 = extractvalue { i64, i1 } %1350, 1
  br i1 %1351, label %1352, label %1345

1352:                                             ; preds = %1348
  %1353 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1339 monotonic, align 8, !noalias !23290
  %1354 = call i64 @llvm.ssub.sat.i64(i64 %1353, i64 %1339)
  %1355 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23290
  br label %1356

1356:                                             ; preds = %1359, %1352
  %1357 = phi i64 [ %1355, %1352 ], [ %1362, %1359 ]
  %1358 = icmp slt i64 %1354, %1357
  br i1 %1358, label %1359, label %1363

1359:                                             ; preds = %1356
  %1360 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1357, i64 %1354 monotonic monotonic, align 8, !noalias !23290
  %1361 = extractvalue { i64, i1 } %1360, 1
  %1362 = extractvalue { i64, i1 } %1360, 0
  br i1 %1361, label %1363, label %1356

1363:                                             ; preds = %1359, %1356
  %1364 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23290
  br label %__rustc::__rust_dealloc (.exit87)

__rustc::__rust_dealloc (.exit87): ; preds = %1345, %1363
  call void @free(ptr noundef nonnull %1289) #92, !noalias !23290
  br label %1365

1365:                                             ; preds = %1441, %__rustc::__rust_dealloc (.exit87), %.loopexit98, %648
  %1366 = phi i8 [ 1, %1441 ], [ 0, %648 ], [ %652, %.loopexit98 ], [ %652, %__rustc::__rust_dealloc (.exit87) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !23057
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %66)
          to label %1442 unwind label %292, !noalias !23050

1367:                                             ; preds = %1282, %1279, %1275
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !23057
  %1368 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1368, ptr noundef nonnull align 8 dereferenceable(96) %32, i64 96, i1 false), !noalias !23101
  store i64 0, ptr %0, align 16, !alias.scope !23038, !noalias !23101
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !23057
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %66)
          to label %1369 unwind label %295, !noalias !23050

1369:                                             ; preds = %1367
  call void @llvm.lifetime.end.p0(ptr nonnull %66), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %67), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %68), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !23057
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %71)
          to label %1370 unwind label %182

1370:                                             ; preds = %1369
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23294)
  call void @llvm.experimental.noalias.scope.decl(metadata !23297), !noalias !23300
  %1371 = load ptr, ptr %221, align 8, !alias.scope !23301, !noalias !23300, !nonnull !1740, !noundef !1740
  %1372 = atomicrmw sub ptr %1371, i64 1 release, align 8, !noalias !23302
  %1373 = icmp eq i64 %1372, 1
  br i1 %1373, label %1374, label %1378

1374:                                             ; preds = %1370
  fence acquire, !noalias !23300
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %221) #91
          to label %1378 unwind label %1375

1375:                                             ; preds = %1374
  %1376 = landingpad { ptr, i32 }
          cleanup
  %1377 = trunc nuw i8 %652 to i1
  br i1 %1377, label %1795, label %1797

1378:                                             ; preds = %1374, %1370
  %1379 = trunc nuw i8 %652 to i1
  br i1 %1379, label %1380, label %1648

1380:                                             ; preds = %1378
  call void @llvm.experimental.noalias.scope.decl(metadata !23303)
  %1381 = getelementptr inbounds nuw i8, ptr %77, i64 8
  %1382 = load ptr, ptr %1381, align 8, !alias.scope !23303, !nonnull !1740, !noundef !1740
  %1383 = load i64, ptr %207, align 8, !alias.scope !23303, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23306)
  %1384 = icmp eq i64 %1383, 0
  br i1 %1384, label %.loopexit96, label %.preheader95

.preheader95:                                     ; preds = %1380
  %1385 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1386

1386:                                             ; preds = %.preheader95, %1424
  %1387 = phi i64 [ %1389, %1424 ], [ 0, %.preheader95 ]
  %1388 = getelementptr inbounds nuw [40 x i8], ptr %1382, i64 %1387
  %1389 = add nuw nsw i64 %1387, 1
  %1390 = load i64, ptr %1388, align 8, !range !1778, !alias.scope !23309, !noalias !23303, !noundef !1740
  %1391 = icmp ugt i64 %1390, 5
  br i1 %1391, label %1392, label %1424

1392:                                             ; preds = %1386
  %1393 = getelementptr i8, ptr %1388, i64 8
  %1394 = load ptr, ptr %1393, align 8, !alias.scope !23306, !noalias !23303, !nonnull !1740, !noundef !1740
  %1395 = shl i64 %1390, 3
  %1396 = add i64 %1395, -8
  %1397 = load i64, ptr %256, align 8, !noalias !23312, !noundef !1740
  %1398 = call i64 @llvm.umin.i64(i64 %1396, i64 9223372036854775807)
  %1399 = call i64 @llvm.ssub.sat.i64(i64 %1397, i64 %1398)
  store i64 %1399, ptr %256, align 8, !noalias !23312
  %1400 = load i64, ptr %1385, align 8, !noalias !23312, !noundef !1740
  %1401 = icmp slt i64 %1399, %1400
  br i1 %1401, label %1402, label %.preheader477

1402:                                             ; preds = %1392
  store i64 %1399, ptr %1385, align 8, !noalias !23312
  br label %.preheader477

.preheader477:                                    ; preds = %1402, %1392
  br label %1403

1403:                                             ; preds = %.preheader477, %1406
  %1404 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23312
  %1405 = icmp slt i64 %1404, 0
  br i1 %1405, label %1406, label %__rustc::__rust_dealloc (.exit88)

1406:                                             ; preds = %1403
  %1407 = add nsw i64 %1404, 1
  %1408 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1404, i64 %1407 acq_rel acquire, align 8, !noalias !23312
  %1409 = extractvalue { i64, i1 } %1408, 1
  br i1 %1409, label %1410, label %1403

1410:                                             ; preds = %1406
  %1411 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1398 monotonic, align 8, !noalias !23312
  %1412 = call i64 @llvm.ssub.sat.i64(i64 %1411, i64 %1398)
  %1413 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23312
  br label %1414

1414:                                             ; preds = %1417, %1410
  %1415 = phi i64 [ %1413, %1410 ], [ %1420, %1417 ]
  %1416 = icmp slt i64 %1412, %1415
  br i1 %1416, label %1417, label %1421

1417:                                             ; preds = %1414
  %1418 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1415, i64 %1412 monotonic monotonic, align 8, !noalias !23312
  %1419 = extractvalue { i64, i1 } %1418, 1
  %1420 = extractvalue { i64, i1 } %1418, 0
  br i1 %1419, label %1421, label %1414

1421:                                             ; preds = %1417, %1414
  %1422 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23312
  br label %__rustc::__rust_dealloc (.exit88)

__rustc::__rust_dealloc (.exit88): ; preds = %1403, %1421
  %1423 = icmp ne i64 %1396, 0
  call void @llvm.assume(i1 %1423), !noalias !23312
  call void @free(ptr noundef nonnull %1394) #92, !noalias !23312
  br label %1424

1424:                                             ; preds = %__rustc::__rust_dealloc (.exit88), %1386
  %1425 = icmp eq i64 %1389, %1383
  br i1 %1425, label %.loopexit96, label %1386

.loopexit96:                                      ; preds = %1424, %1380
  %1426 = load i64, ptr %77, align 8, !alias.scope !23303
  %1427 = icmp eq i64 %1426, 0
  br i1 %1427, label %1648, label %1617

1428:                                             ; preds = %1283
  %1429 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %65) #89, !noalias !23050
  br label %340

1430:                                             ; preds = %1203, %723
  call void @llvm.experimental.noalias.scope.decl(metadata !23315)
  %1431 = load ptr, ptr %63, align 8, !alias.scope !23315, !noalias !23050, !noundef !1740
  %1432 = icmp eq ptr %1431, null
  br i1 %1432, label %1441, label %1433

1433:                                             ; preds = %1430
  %1434 = atomicrmw sub ptr %1431, i64 1 release, align 8, !noalias !23318
  %1435 = icmp eq i64 %1434, 1
  br i1 %1435, label %1436, label %1441

1436:                                             ; preds = %1433
  fence acquire, !noalias !23050
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %63) #91
          to label %1441 unwind label %344, !inline_history !2025

1437:                                             ; preds = %974, %965, %964, %945, %943, %939, %913, %891
  %1438 = phi { ptr, i32 } [ %969, %964 ], [ %969, %974 ], [ %966, %965 ], [ %946, %945 ], [ %940, %939 ], [ %892, %891 ], [ %914, %913 ], [ %944, %943 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %58) #89
          to label %695 unwind label %668, !noalias !23050

1439:                                             ; preds = %729
  %1440 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %15) #89
          to label %695 unwind label %668

1441:                                             ; preds = %1436, %1433, %1430
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %64), !noalias !23057
  br label %1365

1442:                                             ; preds = %1365
  call void @llvm.lifetime.end.p0(ptr nonnull %66), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23321)
  call void @llvm.experimental.noalias.scope.decl(metadata !23324), !noalias !23050
  %1443 = load ptr, ptr %67, align 8, !alias.scope !23327, !noalias !23050, !nonnull !1740, !noundef !1740
  %1444 = atomicrmw sub ptr %1443, i64 1 release, align 8, !noalias !23328
  %1445 = icmp eq i64 %1444, 1
  br i1 %1445, label %1446, label %1447

1446:                                             ; preds = %1442
  fence acquire, !noalias !23050
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %67) #91
          to label %1447 unwind label %239

1447:                                             ; preds = %1446, %1442
  call void @llvm.lifetime.end.p0(ptr nonnull %67), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %68), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !23057
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !23057
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %71)
          to label %1448 unwind label %179

1448:                                             ; preds = %1447
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !23057
  call void @llvm.experimental.noalias.scope.decl(metadata !23329)
  %1449 = getelementptr inbounds nuw i8, ptr %74, i64 72
  %1450 = load i64, ptr %1449, align 8, !range !1778, !alias.scope !23332, !noalias !23262, !noundef !1740
  %1451 = icmp ugt i64 %1450, 5
  br i1 %1451, label %1452, label %1485

1452:                                             ; preds = %1448
  %1453 = getelementptr inbounds nuw i8, ptr %74, i64 80
  %1454 = load ptr, ptr %1453, align 8, !alias.scope !23329, !noalias !23262, !nonnull !1740, !noundef !1740
  %1455 = mul i64 %1450, 3
  %1456 = add i64 %1455, -3
  %1457 = load i64, ptr %256, align 8, !noalias !23335, !noundef !1740
  %1458 = call i64 @llvm.umin.i64(i64 %1456, i64 9223372036854775807)
  %1459 = call i64 @llvm.ssub.sat.i64(i64 %1457, i64 %1458)
  store i64 %1459, ptr %256, align 8, !noalias !23335
  %1460 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1461 = load i64, ptr %1460, align 8, !noalias !23335, !noundef !1740
  %1462 = icmp slt i64 %1459, %1461
  br i1 %1462, label %1463, label %.preheader476

1463:                                             ; preds = %1452
  store i64 %1459, ptr %1460, align 8, !noalias !23335
  br label %.preheader476

.preheader476:                                    ; preds = %1463, %1452
  br label %1464

1464:                                             ; preds = %.preheader476, %1467
  %1465 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23335
  %1466 = icmp slt i64 %1465, 0
  br i1 %1466, label %1467, label %__rustc::__rust_dealloc (.exit89)

1467:                                             ; preds = %1464
  %1468 = add nsw i64 %1465, 1
  %1469 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1465, i64 %1468 acq_rel acquire, align 8, !noalias !23335
  %1470 = extractvalue { i64, i1 } %1469, 1
  br i1 %1470, label %1471, label %1464

1471:                                             ; preds = %1467
  %1472 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1458 monotonic, align 8, !noalias !23335
  %1473 = call i64 @llvm.ssub.sat.i64(i64 %1472, i64 %1458)
  %1474 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23335
  br label %1475

1475:                                             ; preds = %1478, %1471
  %1476 = phi i64 [ %1474, %1471 ], [ %1481, %1478 ]
  %1477 = icmp slt i64 %1473, %1476
  br i1 %1477, label %1478, label %1482

1478:                                             ; preds = %1475
  %1479 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1476, i64 %1473 monotonic monotonic, align 8, !noalias !23335
  %1480 = extractvalue { i64, i1 } %1479, 1
  %1481 = extractvalue { i64, i1 } %1479, 0
  br i1 %1480, label %1482, label %1475

1482:                                             ; preds = %1478, %1475
  %1483 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23335
  br label %__rustc::__rust_dealloc (.exit89)

__rustc::__rust_dealloc (.exit89): ; preds = %1464, %1482
  %1484 = icmp ne i64 %1456, 0
  call void @llvm.assume(i1 %1484), !noalias !23335
  call void @free(ptr noundef nonnull %1454) #92, !noalias !23335
  br label %1485

1485:                                             ; preds = %__rustc::__rust_dealloc (.exit89), %1448
  %1486 = load i64, ptr %74, align 8, !range !2059, !alias.scope !23329, !noalias !23262, !noundef !1740
  %1487 = icmp sgt i64 %1486, 0
  br i1 %1487, label %1488, label %1519

1488:                                             ; preds = %1485
  %1489 = getelementptr inbounds nuw i8, ptr %74, i64 8
  %1490 = load ptr, ptr %1489, align 8, !alias.scope !23329, !noalias !23262, !nonnull !1740, !noundef !1740
  %1491 = mul nuw i64 %1486, 3
  %1492 = load i64, ptr %256, align 8, !noalias !23338, !noundef !1740
  %1493 = call i64 @llvm.umin.i64(i64 %1491, i64 9223372036854775807)
  %1494 = call i64 @llvm.ssub.sat.i64(i64 %1492, i64 %1493)
  store i64 %1494, ptr %256, align 8, !noalias !23338
  %1495 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1496 = load i64, ptr %1495, align 8, !noalias !23338, !noundef !1740
  %1497 = icmp slt i64 %1494, %1496
  br i1 %1497, label %1498, label %.preheader475

1498:                                             ; preds = %1488
  store i64 %1494, ptr %1495, align 8, !noalias !23338
  br label %.preheader475

.preheader475:                                    ; preds = %1498, %1488
  br label %1499

1499:                                             ; preds = %.preheader475, %1502
  %1500 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23338
  %1501 = icmp slt i64 %1500, 0
  br i1 %1501, label %1502, label %__rustc::__rust_dealloc (.exit90)

1502:                                             ; preds = %1499
  %1503 = add nsw i64 %1500, 1
  %1504 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1500, i64 %1503 acq_rel acquire, align 8, !noalias !23338
  %1505 = extractvalue { i64, i1 } %1504, 1
  br i1 %1505, label %1506, label %1499

1506:                                             ; preds = %1502
  %1507 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1493 monotonic, align 8, !noalias !23338
  %1508 = call i64 @llvm.ssub.sat.i64(i64 %1507, i64 %1493)
  %1509 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23338
  br label %1510

1510:                                             ; preds = %1513, %1506
  %1511 = phi i64 [ %1509, %1506 ], [ %1516, %1513 ]
  %1512 = icmp slt i64 %1508, %1511
  br i1 %1512, label %1513, label %1517

1513:                                             ; preds = %1510
  %1514 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1511, i64 %1508 monotonic monotonic, align 8, !noalias !23338
  %1515 = extractvalue { i64, i1 } %1514, 1
  %1516 = extractvalue { i64, i1 } %1514, 0
  br i1 %1515, label %1517, label %1510

1517:                                             ; preds = %1513, %1510
  %1518 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23338
  br label %__rustc::__rust_dealloc (.exit90)

__rustc::__rust_dealloc (.exit90): ; preds = %1499, %1517
  call void @free(ptr noundef nonnull %1490) #92, !noalias !23338
  br label %1519

1519:                                             ; preds = %__rustc::__rust_dealloc (.exit90), %1485
  %1520 = getelementptr inbounds nuw i8, ptr %74, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23339), !noalias !23262
  %1521 = load ptr, ptr %1520, align 8, !alias.scope !23342, !noalias !23262, !noundef !1740
  %1522 = icmp eq ptr %1521, null
  br i1 %1522, label %1537, label %1523

1523:                                             ; preds = %1519
  %1524 = atomicrmw sub ptr %1521, i64 1 release, align 8, !noalias !23343
  %1525 = icmp eq i64 %1524, 1
  br i1 %1525, label %1526, label %1537

1526:                                             ; preds = %1523
  fence acquire, !noalias !23262
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1520) #91
          to label %1537 unwind label %1535

1527:                                             ; preds = %1604, %1535, %182, %178
  %1528 = phi i8 [ %1366, %1535 ], [ %1606, %1604 ], [ %225, %178 ], [ %652, %182 ]
  %1529 = phi { ptr, i32 } [ %1536, %1535 ], [ %1605, %1604 ], [ %227, %178 ], [ %183, %182 ]
  %1530 = getelementptr inbounds nuw i8, ptr %77, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !23348)
  call void @llvm.experimental.noalias.scope.decl(metadata !23351), !noalias !23146
  %1531 = load ptr, ptr %1530, align 8, !alias.scope !23354, !noalias !23146, !nonnull !1740, !noundef !1740
  %1532 = atomicrmw sub ptr %1531, i64 1 release, align 8, !noalias !23355
  %1533 = icmp eq i64 %1532, 1
  br i1 %1533, label %1534, label %1607

1534:                                             ; preds = %1527
  fence acquire, !noalias !23146
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1530) #91
          to label %1607 unwind label %668

1535:                                             ; preds = %1526
  %1536 = landingpad { ptr, i32 }
          cleanup
  br label %1527

1537:                                             ; preds = %1526, %1523, %1519
  call void @llvm.experimental.noalias.scope.decl(metadata !23356)
  call void @llvm.experimental.noalias.scope.decl(metadata !23359), !noalias !23146
  %1538 = load ptr, ptr %221, align 8, !alias.scope !23362, !noalias !23146, !nonnull !1740, !noundef !1740
  %1539 = atomicrmw sub ptr %1538, i64 1 release, align 8, !noalias !23363
  %1540 = icmp eq i64 %1539, 1
  br i1 %1540, label %1541, label %1545

1541:                                             ; preds = %1537
  fence acquire, !noalias !23146
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %221) #91
          to label %1545 unwind label %1542

1542:                                             ; preds = %1541
  %1543 = landingpad { ptr, i32 }
          cleanup
  %1544 = trunc nuw i8 %1366 to i1
  br i1 %1544, label %1795, label %1797

1545:                                             ; preds = %1541, %1537
  %1546 = trunc nuw i8 %1366 to i1
  br i1 %1546, label %1547, label %1648

1547:                                             ; preds = %1545
  call void @llvm.experimental.noalias.scope.decl(metadata !23364)
  %1548 = getelementptr inbounds nuw i8, ptr %77, i64 8
  %1549 = load ptr, ptr %1548, align 8, !alias.scope !23364, !nonnull !1740, !noundef !1740
  %1550 = load i64, ptr %207, align 8, !alias.scope !23364, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23367)
  %1551 = icmp eq i64 %1550, 0
  br i1 %1551, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %1547
  %1552 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1553

1553:                                             ; preds = %.preheader, %1591
  %1554 = phi i64 [ %1556, %1591 ], [ 0, %.preheader ]
  %1555 = getelementptr inbounds nuw [40 x i8], ptr %1549, i64 %1554
  %1556 = add nuw nsw i64 %1554, 1
  %1557 = load i64, ptr %1555, align 8, !range !1778, !alias.scope !23370, !noalias !23364, !noundef !1740
  %1558 = icmp ugt i64 %1557, 5
  br i1 %1558, label %1559, label %1591

1559:                                             ; preds = %1553
  %1560 = getelementptr i8, ptr %1555, i64 8
  %1561 = load ptr, ptr %1560, align 8, !alias.scope !23367, !noalias !23364, !nonnull !1740, !noundef !1740
  %1562 = shl i64 %1557, 3
  %1563 = add i64 %1562, -8
  %1564 = load i64, ptr %256, align 8, !noalias !23373, !noundef !1740
  %1565 = call i64 @llvm.umin.i64(i64 %1563, i64 9223372036854775807)
  %1566 = call i64 @llvm.ssub.sat.i64(i64 %1564, i64 %1565)
  store i64 %1566, ptr %256, align 8, !noalias !23373
  %1567 = load i64, ptr %1552, align 8, !noalias !23373, !noundef !1740
  %1568 = icmp slt i64 %1566, %1567
  br i1 %1568, label %1569, label %.preheader474

1569:                                             ; preds = %1559
  store i64 %1566, ptr %1552, align 8, !noalias !23373
  br label %.preheader474

.preheader474:                                    ; preds = %1569, %1559
  br label %1570

1570:                                             ; preds = %.preheader474, %1573
  %1571 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23373
  %1572 = icmp slt i64 %1571, 0
  br i1 %1572, label %1573, label %__rustc::__rust_dealloc (.exit91)

1573:                                             ; preds = %1570
  %1574 = add nsw i64 %1571, 1
  %1575 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1571, i64 %1574 acq_rel acquire, align 8, !noalias !23373
  %1576 = extractvalue { i64, i1 } %1575, 1
  br i1 %1576, label %1577, label %1570

1577:                                             ; preds = %1573
  %1578 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1565 monotonic, align 8, !noalias !23373
  %1579 = call i64 @llvm.ssub.sat.i64(i64 %1578, i64 %1565)
  %1580 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23373
  br label %1581

1581:                                             ; preds = %1584, %1577
  %1582 = phi i64 [ %1580, %1577 ], [ %1587, %1584 ]
  %1583 = icmp slt i64 %1579, %1582
  br i1 %1583, label %1584, label %1588

1584:                                             ; preds = %1581
  %1585 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1582, i64 %1579 monotonic monotonic, align 8, !noalias !23373
  %1586 = extractvalue { i64, i1 } %1585, 1
  %1587 = extractvalue { i64, i1 } %1585, 0
  br i1 %1586, label %1588, label %1581

1588:                                             ; preds = %1584, %1581
  %1589 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23373
  br label %__rustc::__rust_dealloc (.exit91)

__rustc::__rust_dealloc (.exit91): ; preds = %1570, %1588
  %1590 = icmp ne i64 %1563, 0
  call void @llvm.assume(i1 %1590), !noalias !23373
  call void @free(ptr noundef nonnull %1561) #92, !noalias !23373
  br label %1591

1591:                                             ; preds = %__rustc::__rust_dealloc (.exit91), %1553
  %1592 = icmp eq i64 %1556, %1550
  br i1 %1592, label %.loopexit, label %1553

.loopexit:                                        ; preds = %1591, %1547
  %1593 = load i64, ptr %77, align 8, !alias.scope !23364
  %1594 = icmp eq i64 %1593, 0
  br i1 %1594, label %1648, label %1617

1595:                                             ; preds = %292, %291
  %1596 = phi { ptr, i32 } [ %294, %292 ], [ %343, %291 ]
  %1597 = phi i8 [ %293, %292 ], [ %341, %291 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23376)
  call void @llvm.experimental.noalias.scope.decl(metadata !23379), !noalias !23050
  %1598 = load ptr, ptr %67, align 8, !alias.scope !23382, !noalias !23050, !nonnull !1740, !noundef !1740
  %1599 = atomicrmw sub ptr %1598, i64 1 release, align 8, !noalias !23383
  %1600 = icmp eq i64 %1599, 1
  br i1 %1600, label %1601, label %224

1601:                                             ; preds = %1595
  fence acquire, !noalias !23050
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %67) #91
          to label %224 unwind label %668

1602:                                             ; preds = %234
  %1603 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 dereferenceable(56) %70) #89
          to label %224 unwind label %668, !noalias !23050

1604:                                             ; preds = %179, %178
  %1605 = phi { ptr, i32 } [ %181, %179 ], [ %227, %178 ]
  %1606 = phi i8 [ %180, %179 ], [ %225, %178 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %74) #89
          to label %1527 unwind label %668, !noalias !23262

1607:                                             ; preds = %1534, %1527
  %1608 = trunc nuw i8 %1528 to i1
  br i1 %1608, label %1795, label %1797

1609:                                             ; preds = %173
  call void @llvm.lifetime.end.p0(ptr nonnull %75)
  call void @llvm.lifetime.end.p0(ptr nonnull %76)
  call void @llvm.lifetime.start.p0(ptr nonnull %81)
  %1610 = getelementptr inbounds nuw i8, ptr %82, i64 96
  %1611 = load ptr, ptr %1610, align 8, !noundef !1740
  %1612 = icmp eq ptr %1611, null
  br i1 %1612, label %1664, label %1613

1613:                                             ; preds = %1609
  %1614 = atomicrmw add ptr %1611, i64 1 monotonic, align 8
  %1615 = icmp slt i64 %1614, 0
  br i1 %1615, label %1616, label %1650

1616:                                             ; preds = %1613
  tail call void @llvm.trap()
  unreachable

1617:                                             ; preds = %.loopexit, %.loopexit96
  %1618 = phi i64 [ %1426, %.loopexit96 ], [ %1593, %.loopexit ]
  %1619 = phi ptr [ %1382, %.loopexit96 ], [ %1549, %.loopexit ]
  %1620 = mul nuw i64 %1618, 40
  %1621 = load i64, ptr %256, align 8, !noalias !1740, !noundef !1740
  %1622 = call i64 @llvm.umin.i64(i64 %1620, i64 9223372036854775807)
  %1623 = call i64 @llvm.ssub.sat.i64(i64 %1621, i64 %1622)
  store i64 %1623, ptr %256, align 8, !noalias !1740
  %1624 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1625 = load i64, ptr %1624, align 8, !noalias !1740, !noundef !1740
  %1626 = icmp slt i64 %1623, %1625
  br i1 %1626, label %1627, label %.preheader473

1627:                                             ; preds = %1617
  store i64 %1623, ptr %1624, align 8, !noalias !1740
  br label %.preheader473

.preheader473:                                    ; preds = %1627, %1617
  br label %1628

1628:                                             ; preds = %.preheader473, %1631
  %1629 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !1740
  %1630 = icmp slt i64 %1629, 0
  br i1 %1630, label %1631, label %__rustc::__rust_dealloc (.exit92)

1631:                                             ; preds = %1628
  %1632 = add nsw i64 %1629, 1
  %1633 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1629, i64 %1632 acq_rel acquire, align 8, !noalias !1740
  %1634 = extractvalue { i64, i1 } %1633, 1
  br i1 %1634, label %1635, label %1628

1635:                                             ; preds = %1631
  %1636 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1622 monotonic, align 8, !noalias !1740
  %1637 = call i64 @llvm.ssub.sat.i64(i64 %1636, i64 %1622)
  %1638 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !1740
  br label %1639

1639:                                             ; preds = %1642, %1635
  %1640 = phi i64 [ %1638, %1635 ], [ %1645, %1642 ]
  %1641 = icmp slt i64 %1637, %1640
  br i1 %1641, label %1642, label %1646

1642:                                             ; preds = %1639
  %1643 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1640, i64 %1637 monotonic monotonic, align 8, !noalias !1740
  %1644 = extractvalue { i64, i1 } %1643, 1
  %1645 = extractvalue { i64, i1 } %1643, 0
  br i1 %1644, label %1646, label %1639

1646:                                             ; preds = %1642, %1639
  %1647 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !1740
  br label %__rustc::__rust_dealloc (.exit92)

__rustc::__rust_dealloc (.exit92): ; preds = %1628, %1646
  call void @free(ptr noundef nonnull %1619) #92, !noalias !1740
  br label %1648

1648:                                             ; preds = %__rustc::__rust_dealloc (.exit92), %.loopexit, %1545, %.loopexit96, %1378
  call void @llvm.lifetime.end.p0(ptr nonnull %28)
  call void @llvm.lifetime.end.p0(ptr nonnull %74)
  call void @llvm.lifetime.end.p0(ptr nonnull %77)
  br label %1649

1649:                                             ; preds = %1788, %1648, %170, %167, %163
  call void @llvm.lifetime.end.p0(ptr nonnull %82)
  ret void

1650:                                             ; preds = %1613
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !23384
  store ptr %1611, ptr %8, align 8, !noalias !23387
  %1651 = getelementptr inbounds nuw i8, ptr %1611, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(56) %81, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1651)
          to label %1657 unwind label %1652

1652:                                             ; preds = %1650
  %1653 = landingpad { ptr, i32 }
          cleanup
  %1654 = atomicrmw sub ptr %1611, i64 1 release, align 8, !noalias !23390
  %1655 = icmp eq i64 %1654, 1
  br i1 %1655, label %1656, label %1799

1656:                                             ; preds = %1652
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %8) #91
          to label %1799 unwind label %1661, !noalias !23387

1657:                                             ; preds = %1650
  %1658 = atomicrmw sub ptr %1611, i64 1 release, align 8, !noalias !23395
  %1659 = icmp eq i64 %1658, 1
  br i1 %1659, label %1660, label %1663

1660:                                             ; preds = %1657
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %8) #91
          to label %1663 unwind label %83

1661:                                             ; preds = %1656
  %1662 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !23387
  unreachable

1663:                                             ; preds = %1660, %1657
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !23384
  br label %1680

1664:                                             ; preds = %1609
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !23400
; invoke purrdf_sparql_eval::eval::syntactic_schema
  %1665 = invoke noundef nonnull ptr @purrdf_sparql_eval::eval::syntactic_schema(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2)
          to label %1666 unwind label %83

1666:                                             ; preds = %1664
  store ptr %1665, ptr %7, align 8, !noalias !23400
  %1667 = getelementptr inbounds nuw i8, ptr %1665, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(56) %81, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1667)
          to label %1673 unwind label %1668

1668:                                             ; preds = %1666
  %1669 = landingpad { ptr, i32 }
          cleanup
  %1670 = atomicrmw sub ptr %1665, i64 1 release, align 8, !noalias !23403
  %1671 = icmp eq i64 %1670, 1
  br i1 %1671, label %1672, label %1799

1672:                                             ; preds = %1668
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %7) #91
          to label %1799 unwind label %1677, !noalias !23400

1673:                                             ; preds = %1666
  %1674 = atomicrmw sub ptr %1665, i64 1 release, align 8, !noalias !23408
  %1675 = icmp eq i64 %1674, 1
  br i1 %1675, label %1676, label %1679

1676:                                             ; preds = %1673
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %7) #91
          to label %1679 unwind label %83

1677:                                             ; preds = %1672
  %1678 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !23400
  unreachable

1679:                                             ; preds = %1676, %1673
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !23400
  br label %1680

1680:                                             ; preds = %1679, %1663
  %1681 = load ptr, ptr %3, align 8, !nonnull !1740, !noundef !1740
  %1682 = atomicrmw add ptr %1681, i64 1 monotonic, align 8
  %1683 = icmp slt i64 %1682, 0
  br i1 %1683, label %1688, label %1684

1684:                                             ; preds = %1680
  %1685 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %1686 = load i64, ptr %1685, align 8, !noundef !1740
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %1687 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %81, ptr noundef nonnull %1681, i64 noundef %1686)
          to label %1689 unwind label %1793

1688:                                             ; preds = %1680
  tail call void @llvm.trap()
  unreachable

1689:                                             ; preds = %1684
  call void @llvm.lifetime.start.p0(ptr nonnull %80)
  call void @llvm.lifetime.start.p0(ptr nonnull %79)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %79, ptr noundef nonnull align 8 dereferenceable(104) %82, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %78)
  %1690 = getelementptr inbounds nuw i8, ptr %73, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %73)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %1690, ptr noundef nonnull align 8 dereferenceable(56) %81, i64 56, i1 false)
  store i64 1, ptr %73, align 8
  %1691 = getelementptr inbounds nuw i8, ptr %73, i64 8
  store i64 1, ptr %1691, align 8
  %1692 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #92, !noalias !23413
  %1693 = icmp eq ptr %1692, null
  br i1 %1693, label %__rustc::__rust_alloc (.exit93.thread), label %1694

1694:                                             ; preds = %1689
  %1695 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1696 = load i64, ptr %1695, align 8, !noalias !23413, !noundef !1740
  %1697 = call i64 @llvm.uadd.sat.i64(i64 %1696, i64 1)
  store i64 %1697, ptr %1695, align 8, !noalias !23413
  %1698 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1699 = load i64, ptr %1698, align 8, !noalias !23413, !noundef !1740
  %1700 = call i64 @llvm.uadd.sat.i64(i64 %1699, i64 72)
  store i64 %1700, ptr %1698, align 8, !noalias !23413
  %1701 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1702 = load i64, ptr %1701, align 8, !noalias !23413, !noundef !1740
  %1703 = call i64 @llvm.sadd.sat.i64(i64 %1702, i64 72)
  store i64 %1703, ptr %1701, align 8, !noalias !23413
  %1704 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1705 = load i64, ptr %1704, align 8, !noalias !23413, !noundef !1740
  %1706 = icmp sgt i64 %1703, %1705
  br i1 %1706, label %1707, label %.preheader472

1707:                                             ; preds = %1694
  store i64 %1703, ptr %1704, align 8, !noalias !23413
  br label %.preheader472

.preheader472:                                    ; preds = %1707, %1694
  br label %1708

1708:                                             ; preds = %.preheader472, %1711
  %1709 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23413
  %1710 = icmp slt i64 %1709, 0
  br i1 %1710, label %1711, label %__rustc::__rust_alloc (.exit93)

1711:                                             ; preds = %1708
  %1712 = add nsw i64 %1709, 1
  %1713 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1709, i64 %1712 acq_rel acquire, align 8, !noalias !23413
  %1714 = extractvalue { i64, i1 } %1713, 1
  br i1 %1714, label %1715, label %1708

1715:                                             ; preds = %1711
  %1716 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !23413
  %1717 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !23413
  %1718 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !23413
  %1719 = call i64 @llvm.sadd.sat.i64(i64 %1718, i64 72)
  %1720 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !23413
  br label %1721

1721:                                             ; preds = %1724, %1715
  %1722 = phi i64 [ %1720, %1715 ], [ %1727, %1724 ]
  %1723 = icmp sgt i64 %1719, %1722
  br i1 %1723, label %1724, label %1728

1724:                                             ; preds = %1721
  %1725 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %1722, i64 %1719 monotonic monotonic, align 8, !noalias !23413
  %1726 = extractvalue { i64, i1 } %1725, 1
  %1727 = extractvalue { i64, i1 } %1725, 0
  br i1 %1726, label %1728, label %1721

1728:                                             ; preds = %1724, %1721
  %1729 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23413
  br label %__rustc::__rust_alloc (.exit93)

__rustc::__rust_alloc (.exit93.thread): ; preds = %1689
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #93
          to label %1730 unwind label %1731

1730:                                             ; preds = %__rustc::__rust_alloc (.exit93.thread)
  unreachable

1731:                                             ; preds = %__rustc::__rust_alloc (.exit93.thread)
  %1732 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %1690)
          to label %1790 unwind label %1733

1733:                                             ; preds = %1731
  %1734 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

__rustc::__rust_alloc (.exit93): ; preds = %1708, %1728
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1692, ptr noundef nonnull align 8 dereferenceable(72) %73, i64 72, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %73)
  %1735 = getelementptr inbounds nuw i8, ptr %78, i64 24
  store ptr %1692, ptr %1735, align 8, !alias.scope !23416
  store i64 0, ptr %78, align 8, !alias.scope !23416
  %1736 = getelementptr inbounds nuw i8, ptr %78, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1736, align 8, !alias.scope !23416
  %1737 = getelementptr inbounds nuw i8, ptr %78, i64 16
  store i64 0, ptr %1737, align 8, !alias.scope !23416
  call void @llvm.experimental.noalias.scope.decl(metadata !23419)
  call void @llvm.experimental.noalias.scope.decl(metadata !23422)
  call void @llvm.experimental.noalias.scope.decl(metadata !23424)
  %1738 = load i64, ptr %79, align 8, !range !2059, !alias.scope !23422, !noalias !23426, !noundef !1740
  %1739 = icmp eq i64 %1738, -1
  br i1 %1739, label %1741, label %1740

1740:                                             ; preds = %__rustc::__rust_alloc (.exit93)
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %80, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %78, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %82)
  br label %1743

1741:                                             ; preds = %__rustc::__rust_alloc (.exit93)
  %1742 = getelementptr inbounds nuw i8, ptr %80, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %1742, ptr noundef nonnull readonly align 8 dereferenceable(32) %78, i64 32, i1 false), !alias.scope !23426, !noalias !23422
  store i64 -1, ptr %80, align 8, !alias.scope !23419, !noalias !23427
  br label %1743

1743:                                             ; preds = %1741, %1740
  %1744 = getelementptr inbounds nuw i8, ptr %79, i64 72
  %1745 = load i64, ptr %1744, align 8, !range !1778, !alias.scope !23428, !noalias !23426, !noundef !1740
  %1746 = icmp ugt i64 %1745, 5
  br i1 %1746, label %1747, label %1780

1747:                                             ; preds = %1743
  %1748 = getelementptr inbounds nuw i8, ptr %79, i64 80
  %1749 = load ptr, ptr %1748, align 8, !alias.scope !23422, !noalias !23426, !nonnull !1740, !noundef !1740
  %1750 = mul i64 %1745, 3
  %1751 = add i64 %1750, -3
  %1752 = load i64, ptr %1701, align 8, !noalias !23431, !noundef !1740
  %1753 = call i64 @llvm.umin.i64(i64 %1751, i64 9223372036854775807)
  %1754 = call i64 @llvm.ssub.sat.i64(i64 %1752, i64 %1753)
  store i64 %1754, ptr %1701, align 8, !noalias !23431
  %1755 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1756 = load i64, ptr %1755, align 8, !noalias !23431, !noundef !1740
  %1757 = icmp slt i64 %1754, %1756
  br i1 %1757, label %1758, label %.preheader471

1758:                                             ; preds = %1747
  store i64 %1754, ptr %1755, align 8, !noalias !23431
  br label %.preheader471

.preheader471:                                    ; preds = %1758, %1747
  br label %1759

1759:                                             ; preds = %.preheader471, %1762
  %1760 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23431
  %1761 = icmp slt i64 %1760, 0
  br i1 %1761, label %1762, label %__rustc::__rust_dealloc (.exit94)

1762:                                             ; preds = %1759
  %1763 = add nsw i64 %1760, 1
  %1764 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1760, i64 %1763 acq_rel acquire, align 8, !noalias !23431
  %1765 = extractvalue { i64, i1 } %1764, 1
  br i1 %1765, label %1766, label %1759

1766:                                             ; preds = %1762
  %1767 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1753 monotonic, align 8, !noalias !23431
  %1768 = call i64 @llvm.ssub.sat.i64(i64 %1767, i64 %1753)
  %1769 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23431
  br label %1770

1770:                                             ; preds = %1773, %1766
  %1771 = phi i64 [ %1769, %1766 ], [ %1776, %1773 ]
  %1772 = icmp slt i64 %1768, %1771
  br i1 %1772, label %1773, label %1777

1773:                                             ; preds = %1770
  %1774 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1771, i64 %1768 monotonic monotonic, align 8, !noalias !23431
  %1775 = extractvalue { i64, i1 } %1774, 1
  %1776 = extractvalue { i64, i1 } %1774, 0
  br i1 %1775, label %1777, label %1770

1777:                                             ; preds = %1773, %1770
  %1778 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23431
  br label %__rustc::__rust_dealloc (.exit94)

__rustc::__rust_dealloc (.exit94): ; preds = %1759, %1777
  %1779 = icmp ne i64 %1751, 0
  call void @llvm.assume(i1 %1779), !noalias !23431
  call void @free(ptr noundef nonnull %1749) #92, !noalias !23431
  br label %1780

1780:                                             ; preds = %__rustc::__rust_dealloc (.exit94), %1743
  %1781 = getelementptr inbounds nuw i8, ptr %79, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23434)
  %1782 = load ptr, ptr %1781, align 8, !alias.scope !23437, !noalias !23426, !noundef !1740
  %1783 = icmp eq ptr %1782, null
  br i1 %1783, label %1788, label %1784

1784:                                             ; preds = %1780
  %1785 = atomicrmw sub ptr %1782, i64 1 release, align 8, !noalias !23438
  %1786 = icmp eq i64 %1785, 1
  br i1 %1786, label %1787, label %1788

1787:                                             ; preds = %1784
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1781) #91
  br label %1788

1788:                                             ; preds = %1787, %1784, %1780
  call void @llvm.lifetime.end.p0(ptr nonnull %78)
  call void @llvm.lifetime.end.p0(ptr nonnull %79)
  %1789 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1789, ptr noundef nonnull align 8 dereferenceable(96) %80, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %80)
  call void @llvm.lifetime.end.p0(ptr nonnull %81)
  br label %1649

1790:                                             ; preds = %1731
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %82) #89
          to label %1797 unwind label %1791

1791:                                             ; preds = %1799, %1793, %1790
  %1792 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

1793:                                             ; preds = %1684
  %1794 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 dereferenceable(56) %81) #89
          to label %1799 unwind label %1791

1795:                                             ; preds = %1607, %1542, %1375
  %1796 = phi { ptr, i32 } [ %1376, %1375 ], [ %1543, %1542 ], [ %1529, %1607 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %77) #89, !noalias !23146
  br label %1797

1797:                                             ; preds = %1799, %1795, %1790, %1607, %1542, %1375
  %1798 = phi { ptr, i32 } [ %1800, %1799 ], [ %1376, %1375 ], [ %1529, %1607 ], [ %1543, %1542 ], [ %1732, %1790 ], [ %1796, %1795 ]
  resume { ptr, i32 } %1798

1799:                                             ; preds = %1793, %1672, %1668, %1656, %1652, %83
  %1800 = phi { ptr, i32 } [ %1669, %1668 ], [ %84, %83 ], [ %1653, %1652 ], [ %1653, %1656 ], [ %1669, %1672 ], [ %1794, %1793 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %82) #89
          to label %1797 unwind label %1791
}

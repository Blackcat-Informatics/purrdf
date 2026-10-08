define void @purrdf_sparql_eval::expr::eval_extend::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(16) %3, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %4, ptr noalias nofree noundef align 16 dereferenceable(1248) %5) unnamed_addr #10 personality ptr @rust_eh_personality !guid !32976 {
  %7 = alloca [24 x i8], align 8
  %8 = alloca [24 x i8], align 8
  %9 = alloca [48 x i8], align 8
  %10 = alloca [32 x i8], align 8
  %11 = alloca [8 x i8], align 8
  %12 = alloca [24 x i8], align 8
  %13 = alloca [8 x i8], align 8
  %14 = alloca [24 x i8], align 8
  %15 = alloca [24 x i8], align 8
  %16 = alloca [32 x i8], align 8
  %17 = alloca [8 x i8], align 8
  %18 = alloca [16 x i8], align 8
  %19 = alloca [248 x i8], align 8
  %20 = alloca [24 x i8], align 8
  %21 = alloca [32 x i8], align 8
  %22 = alloca [8 x i8], align 8
  %23 = alloca [16 x i8], align 8
  %24 = alloca [32 x i8], align 8
  %25 = alloca [24 x i8], align 8
  %26 = alloca [48 x i8], align 8
  %27 = alloca [8 x i8], align 8
  %28 = alloca [24 x i8], align 8
  %29 = alloca [24 x i8], align 8
  %30 = alloca [24 x i8], align 8
  %31 = alloca [48 x i8], align 8
  %32 = alloca [32 x i8], align 8
  %33 = alloca [8 x i8], align 8
  %34 = alloca [8 x i8], align 8
  %35 = alloca [8 x i8], align 8
  %36 = alloca [16 x i8], align 8
  %37 = alloca [40 x i8], align 8
  %38 = alloca [24 x i8], align 8
  %39 = alloca [48 x i8], align 8
  %40 = alloca [48 x i8], align 8
  %41 = alloca [96 x i8], align 16
  %42 = alloca [32 x i8], align 8
  %43 = alloca [96 x i8], align 16
  %44 = alloca [24 x i8], align 8
  %45 = alloca [24 x i8], align 8
  %46 = alloca [48 x i8], align 8
  %47 = alloca [24 x i8], align 8
  %48 = alloca [24 x i8], align 8
  %49 = alloca [23 x i8], align 1
  %50 = alloca [24 x i8], align 8
  %51 = alloca [80 x i8], align 8
  %52 = alloca [24 x i8], align 8
  %53 = alloca [24 x i8], align 8
  %54 = alloca [80 x i8], align 8
  %55 = alloca [80 x i8], align 8
  %56 = alloca [24 x i8], align 8
  %57 = alloca [80 x i8], align 8
  %58 = alloca [80 x i8], align 8
  %59 = alloca [24 x i8], align 8
  %60 = alloca [24 x i8], align 8
  %61 = alloca [80 x i8], align 8
  %62 = alloca [24 x i8], align 8
  %63 = alloca [24 x i8], align 8
  %64 = alloca [32 x i8], align 8
  %65 = alloca [8 x i8], align 8
  %66 = alloca [8 x i8], align 8
  %67 = alloca [16 x i8], align 8
  %68 = alloca [40 x i8], align 8
  %69 = alloca [48 x i8], align 8
  %70 = alloca [96 x i8], align 16
  %71 = alloca [24 x i8], align 8
  %72 = alloca [24 x i8], align 8
  %73 = alloca [24 x i8], align 8
  %74 = alloca [24 x i8], align 8
  %75 = alloca [24 x i8], align 8
  %76 = alloca [24 x i8], align 8
  %77 = alloca [32 x i8], align 8
  %78 = alloca [160 x i8], align 8
  %79 = alloca [32 x i8], align 8
  %80 = alloca [8 x i8], align 8
  %81 = alloca [48 x i8], align 8
  %82 = alloca [96 x i8], align 16
  %83 = alloca [32 x i8], align 8
  %84 = alloca [32 x i8], align 8
  %85 = alloca [24 x i8], align 8
  %86 = alloca [96 x i8], align 16
  %87 = alloca [24 x i8], align 8
  %88 = alloca [160 x i8], align 8
  %89 = alloca [32 x i8], align 8
  %90 = alloca [24 x i8], align 8
  %91 = alloca [48 x i8], align 8
  %92 = alloca [96 x i8], align 16
  %93 = alloca [32 x i8], align 8
  %94 = alloca [96 x i8], align 16
  %95 = alloca [24 x i8], align 8
  %96 = alloca [24 x i8], align 8
  %97 = alloca [32 x i8], align 8
  %98 = alloca [48 x i8], align 1
  %99 = alloca [224 x i8], align 8
  %100 = alloca [24 x i8], align 8
  %101 = alloca [24 x i8], align 8
  %102 = alloca [48 x i8], align 8
  %103 = alloca [16 x i8], align 8
  %104 = alloca [72 x i8], align 8
  %105 = alloca [112 x i8], align 16
  %106 = alloca [72 x i8], align 8
  %107 = alloca [32 x i8], align 8
  %108 = alloca [104 x i8], align 8
  %109 = alloca [96 x i8], align 8
  %110 = alloca [96 x i8], align 8
  %111 = alloca [24 x i8], align 8
  %112 = alloca [24 x i8], align 8
  %113 = alloca [96 x i8], align 16
  %114 = alloca [24 x i8], align 8
  %115 = alloca [40 x i8], align 8
  %116 = alloca [32 x i8], align 8
  %117 = alloca [40 x i8], align 8
  %118 = alloca [24 x i8], align 8
  %119 = alloca [24 x i8], align 8
  %120 = alloca [96 x i8], align 16
  %121 = alloca [40 x i8], align 8
  %122 = alloca [24 x i8], align 8
  %123 = alloca [200 x i8], align 8
  %124 = alloca [208 x i8], align 8
  %125 = alloca [24 x i8], align 8
  %126 = alloca [48 x i8], align 16
  %127 = alloca [24 x i8], align 8
  %128 = alloca [208 x i8], align 8
  %129 = alloca [24 x i8], align 8
  %130 = alloca [48 x i8], align 16
  %131 = alloca [24 x i8], align 8
  %132 = alloca [248 x i8], align 8
  %133 = alloca [240 x i8], align 8
  %134 = alloca [208 x i8], align 8
  %135 = alloca [32 x i8], align 8
  %136 = alloca [48 x i8], align 8
  %137 = alloca [32 x i8], align 8
  %138 = alloca [256 x i8], align 16
  %139 = alloca [208 x i8], align 16
  %140 = alloca [24 x i8], align 8
  %141 = alloca [8 x i8], align 8
  %142 = alloca [8 x i8], align 8
  %143 = alloca [24 x i8], align 8
  %144 = alloca [216 x i8], align 8
  %145 = alloca [8 x i8], align 8
  %146 = alloca [8 x i8], align 8
  %147 = alloca [8 x i8], align 8
  %148 = alloca [56 x i8], align 8
  %149 = alloca [200 x i8], align 8
  %150 = alloca [104 x i8], align 8
  %151 = alloca [32 x i8], align 8
  %152 = alloca [32 x i8], align 8
  %153 = alloca [32 x i8], align 8
  %154 = alloca [104 x i8], align 8
  %155 = alloca [96 x i8], align 8
  %156 = alloca [56 x i8], align 8
  %157 = alloca [104 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %157)
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %157, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %151)
  call void @llvm.lifetime.start.p0(ptr nonnull %150)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %105, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %161 unwind label %159, !inline_history !24788

158:                                              ; preds = %3508, %3506
  br i1 %184, label %3654, label %3652

159:                                              ; preds = %3535, %3523, %3519, %167, %6
  %160 = landingpad { ptr, i32 }
          cleanup
  br label %3654

161:                                              ; preds = %6
  %162 = load i64, ptr %105, align 16, !range !1855, !noundef !1708
  %163 = trunc nuw i64 %162 to i1
  br i1 %163, label %164, label %167

164:                                              ; preds = %161
  %165 = getelementptr inbounds nuw i8, ptr %105, i64 16
  %166 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %166, ptr noundef nonnull align 16 dereferenceable(96) %165, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %150)
  call void @llvm.lifetime.end.p0(ptr nonnull %151)
  br label %3415

167:                                              ; preds = %161
  %168 = getelementptr inbounds nuw i8, ptr %105, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %150, ptr noundef nonnull align 8 dereferenceable(96) %168, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %151, ptr noalias nofree noundef align 8 dereferenceable(104) %157, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %150)
          to label %169 unwind label %159

169:                                              ; preds = %167
  %170 = load i64, ptr %151, align 8, !range !2062, !noundef !1708
  %171 = icmp eq i64 %170, -1
  br i1 %171, label %174, label %172

172:                                              ; preds = %169
  call void @llvm.lifetime.start.p0(ptr nonnull %152)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %152, ptr noundef nonnull align 8 dereferenceable(32) %151, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %150)
  call void @llvm.lifetime.end.p0(ptr nonnull %151)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
  %173 = invoke fastcc noundef zeroext i1 @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop(ptr noundef nonnull align 16 %5, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %4)
          to label %195 unwind label %191

174:                                              ; preds = %169
  call void @llvm.lifetime.end.p0(ptr nonnull %150)
  call void @llvm.lifetime.end.p0(ptr nonnull %151)
  call void @llvm.lifetime.start.p0(ptr nonnull %156)
  %175 = getelementptr inbounds nuw i8, ptr %157, i64 96
  %176 = load ptr, ptr %175, align 8, !noundef !1708
  %177 = icmp eq ptr %176, null
  br i1 %177, label %3523, label %178

178:                                              ; preds = %174
  %179 = atomicrmw add ptr %176, i64 1 monotonic, align 8
  %180 = icmp slt i64 %179, 0
  br i1 %180, label %181, label %3509

181:                                              ; preds = %178
  tail call void @llvm.trap()
  unreachable

182:                                              ; preds = %235, %191
  %183 = phi i8 [ %192, %191 ], [ %236, %235 ]
  %184 = phi i1 [ %193, %191 ], [ %237, %235 ]
  %185 = phi { ptr, i32 } [ %194, %191 ], [ %238, %235 ]
  %186 = getelementptr inbounds nuw i8, ptr %152, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !32977)
  call void @llvm.experimental.noalias.scope.decl(metadata !32980)
  %187 = load ptr, ptr %186, align 8, !alias.scope !32983, !nonnull !1708, !noundef !1708
  %188 = atomicrmw sub ptr %187, i64 1 release, align 8, !noalias !32983
  %189 = icmp eq i64 %188, 1
  br i1 %189, label %190, label %3506

190:                                              ; preds = %182
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %186) #87
          to label %3506 unwind label %679

191:                                              ; preds = %3326, %3214, %216, %172
  %192 = phi i8 [ %3211, %3326 ], [ %663, %3214 ], [ 1, %216 ], [ 1, %172 ]
  %193 = phi i1 [ true, %3326 ], [ false, %3214 ], [ true, %216 ], [ true, %172 ]
  %194 = landingpad { ptr, i32 }
          cleanup
  br label %182

195:                                              ; preds = %172
  %196 = getelementptr inbounds nuw i8, ptr %5, i64 472
  %197 = load i8, ptr %196, align 8, !range !3634
  %198 = icmp eq i8 %197, 2
  %199 = select i1 %173, i1 %198, i1 false
  br i1 %199, label %200, label %216

200:                                              ; preds = %195
  %201 = getelementptr inbounds nuw i8, ptr %5, i64 616
  %202 = load ptr, ptr %201, align 8, !noundef !1708
  %203 = icmp eq ptr %202, null
  br i1 %203, label %221, label %204

204:                                              ; preds = %200
  %205 = getelementptr inbounds nuw i8, ptr %202, i64 24
  %206 = load i64, ptr %205, align 8, !noalias !32984
  %207 = getelementptr inbounds nuw i8, ptr %202, i64 48
  %208 = icmp ult i64 %206, -2
  br i1 %208, label %216, label %209

209:                                              ; preds = %204
  %210 = getelementptr inbounds nuw i8, ptr %202, i64 32
  %211 = load i64, ptr %210, align 8, !noalias !32984
  %212 = icmp ult i64 %211, -2
  br i1 %212, label %216, label %213

213:                                              ; preds = %209
  %214 = load i64, ptr %207, align 8, !noalias !32984
  %215 = icmp ugt i64 %214, -3
  br i1 %215, label %221, label %216

216:                                              ; preds = %221, %213, %209, %204, %195
  %217 = phi i1 [ false, %195 ], [ false, %213 ], [ %230, %221 ], [ false, %204 ], [ false, %209 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %149)
  %218 = getelementptr inbounds nuw i8, ptr %152, i64 16
  %219 = load i64, ptr %218, align 8, !noundef !1708
  %220 = icmp ult i64 %219, 230584300921369396
  tail call void @llvm.assume(i1 %220)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %149, ptr noundef nonnull align 16 %5, i1 noundef zeroext %217, i64 noundef %219)
          to label %231 unwind label %191

221:                                              ; preds = %213, %200
  %222 = getelementptr inbounds nuw i8, ptr %5, i64 1234
  %223 = load i8, ptr %222, align 2, !range !1746, !noundef !1708
  %224 = trunc nuw i8 %223 to i1
  %225 = getelementptr inbounds nuw i8, ptr %152, i64 16
  %226 = load i64, ptr %225, align 8, !noundef !1708
  %227 = icmp ult i64 %226, 230584300921369396
  tail call void @llvm.assume(i1 %227)
  %228 = icmp samesign ugt i64 %226, 1024
  %229 = xor i1 %224, true
  %230 = select i1 %229, i1 %228, i1 false
  br label %216

231:                                              ; preds = %216
  call void @llvm.lifetime.start.p0(ptr nonnull %148)
  %232 = getelementptr inbounds nuw i8, ptr %152, i64 24
  %233 = load ptr, ptr %232, align 8, !nonnull !1708, !noundef !1708
  %234 = getelementptr inbounds nuw i8, ptr %233, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %148, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %234)
          to label %241 unwind label %239

235:                                              ; preds = %3504, %3503, %3497, %306, %302, %297, %250, %239
  %236 = phi i8 [ 1, %3504 ], [ %3211, %250 ], [ 1, %239 ], [ 1, %297 ], [ %352, %302 ], [ %3499, %3503 ], [ %3499, %3497 ], [ %663, %306 ]
  %237 = phi i1 [ true, %3504 ], [ true, %250 ], [ true, %239 ], [ true, %297 ], [ false, %302 ], [ true, %3503 ], [ true, %3497 ], [ false, %306 ]
  %238 = phi { ptr, i32 } [ %3505, %3504 ], [ %251, %250 ], [ %240, %239 ], [ %298, %297 ], [ %354, %302 ], [ %3498, %3503 ], [ %3498, %3497 ], [ %307, %306 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %149)
          to label %182 unwind label %679

239:                                              ; preds = %231
  %240 = landingpad { ptr, i32 }
          cleanup
  br label %235

241:                                              ; preds = %231
  call void @llvm.lifetime.start.p0(ptr nonnull %147)
  %242 = load ptr, ptr %3, align 8, !nonnull !1708, !noundef !1708
  %243 = atomicrmw add ptr %242, i64 1 monotonic, align 8
  %244 = icmp slt i64 %243, 0
  br i1 %244, label %249, label %245

245:                                              ; preds = %241
  %246 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %247 = load i64, ptr %246, align 8, !noundef !1708
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %248 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %148, ptr noundef nonnull %242, i64 noundef %247)
          to label %252 unwind label %3504

249:                                              ; preds = %241
  tail call void @llvm.trap()
  unreachable

250:                                              ; preds = %3325
  %251 = landingpad { ptr, i32 }
          cleanup
  br label %235

252:                                              ; preds = %245
  store i64 %248, ptr %147, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %146)
  %253 = getelementptr inbounds nuw i8, ptr %148, i64 16
  %254 = load i64, ptr %253, align 8, !noundef !1708
  %255 = icmp ult i64 %254, 576460752303423488
  call void @llvm.assume(i1 %255)
  store i64 %254, ptr %146, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %145)
  %256 = getelementptr inbounds nuw i8, ptr %104, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %104)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %256, ptr noundef nonnull align 8 dereferenceable(56) %148, i64 56, i1 false)
  store i64 1, ptr %104, align 8
  %257 = getelementptr inbounds nuw i8, ptr %104, i64 8
  store i64 1, ptr %257, align 8
  %258 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #88, !noalias !32990
  %259 = icmp eq ptr %258, null
  br i1 %259, label %__rustc::__rust_alloc (.exit.thread), label %260

260:                                              ; preds = %252
  %261 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %262 = load i64, ptr %261, align 8, !noalias !32990, !noundef !1708
  %263 = call i64 @llvm.uadd.sat.i64(i64 %262, i64 1)
  store i64 %263, ptr %261, align 8, !noalias !32990
  %264 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %265 = load i64, ptr %264, align 8, !noalias !32990, !noundef !1708
  %266 = call i64 @llvm.uadd.sat.i64(i64 %265, i64 72)
  store i64 %266, ptr %264, align 8, !noalias !32990
  %267 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %268 = load i64, ptr %267, align 8, !noalias !32990, !noundef !1708
  %269 = call i64 @llvm.sadd.sat.i64(i64 %268, i64 72)
  store i64 %269, ptr %267, align 8, !noalias !32990
  %270 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %271 = load i64, ptr %270, align 8, !noalias !32990, !noundef !1708
  %272 = icmp sgt i64 %269, %271
  br i1 %272, label %273, label %.preheader2535

273:                                              ; preds = %260
  store i64 %269, ptr %270, align 8, !noalias !32990
  br label %.preheader2535

.preheader2535:                                   ; preds = %273, %260
  br label %274

274:                                              ; preds = %.preheader2535, %277
  %275 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !32990
  %276 = icmp slt i64 %275, 0
  br i1 %276, label %277, label %__rustc::__rust_alloc (.exit)

277:                                              ; preds = %274
  %278 = add nsw i64 %275, 1
  %279 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %275, i64 %278 acq_rel acquire, align 8, !noalias !32990
  %280 = extractvalue { i64, i1 } %279, 1
  br i1 %280, label %281, label %274

281:                                              ; preds = %277
  %282 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !32990
  %283 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !32990
  %284 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !32990
  %285 = call i64 @llvm.sadd.sat.i64(i64 %284, i64 72)
  %286 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !32990
  br label %287

287:                                              ; preds = %290, %281
  %288 = phi i64 [ %286, %281 ], [ %293, %290 ]
  %289 = icmp sgt i64 %285, %288
  br i1 %289, label %290, label %294

290:                                              ; preds = %287
  %291 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %288, i64 %285 monotonic monotonic, align 8, !noalias !32990
  %292 = extractvalue { i64, i1 } %291, 1
  %293 = extractvalue { i64, i1 } %291, 0
  br i1 %292, label %294, label %287

294:                                              ; preds = %290, %287
  %295 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !32990
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %252
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #90
          to label %296 unwind label %297

296:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

297:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  %298 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %256)
          to label %235 unwind label %299

299:                                              ; preds = %297
  %300 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %274, %294
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %258, ptr noundef nonnull align 8 dereferenceable(72) %104, i64 72, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %104)
  store ptr %258, ptr %145, align 8
; invoke purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
  %301 = invoke fastcc noundef nonnull ptr @purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 %5, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %4)
          to label %308 unwind label %303

302:                                              ; preds = %351
  br i1 %353, label %3497, label %235

303:                                              ; preds = %3210, %308, %__rustc::__rust_alloc (.exit)
  %304 = phi i8 [ 1, %__rustc::__rust_alloc (.exit) ], [ 1, %308 ], [ %3211, %3210 ]
  %305 = landingpad { ptr, i32 }
          cleanup
  br label %3497

306:                                              ; preds = %3212
  %307 = landingpad { ptr, i32 }
          cleanup
  br label %235

308:                                              ; preds = %__rustc::__rust_alloc (.exit)
  call void @llvm.lifetime.start.p0(ptr nonnull %144)
  %309 = load ptr, ptr %145, align 8, !nonnull !1708, !noundef !1708
  %310 = getelementptr inbounds nuw i8, ptr %309, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(216) %144, ptr noundef nonnull %301, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %4, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %310, ptr noalias nofree noundef align 16 dereferenceable(1248) %5)
          to label %311 unwind label %303

311:                                              ; preds = %308
  call void @llvm.lifetime.start.p0(ptr nonnull %143)
  br i1 %217, label %681, label %312

312:                                              ; preds = %311
  call void @llvm.lifetime.start.p0(ptr nonnull %118)
  %313 = mul nuw nsw i64 %219, 40
  %314 = icmp eq i64 %219, 0
  br i1 %314, label %357, label %315

315:                                              ; preds = %312
  %316 = call noundef ptr @malloc(i64 noundef range(i64 1, 0) %313) #88, !noalias !32993
  %317 = icmp eq ptr %316, null
  br i1 %317, label %__rustc::__rust_alloc (.exit238.thread), label %318

318:                                              ; preds = %315
  %319 = load i64, ptr %261, align 8, !noalias !32993, !noundef !1708
  %320 = call i64 @llvm.uadd.sat.i64(i64 %319, i64 1)
  store i64 %320, ptr %261, align 8, !noalias !32993
  %321 = load i64, ptr %264, align 8, !noalias !32993, !noundef !1708
  %322 = call i64 @llvm.uadd.sat.i64(i64 %321, i64 %313)
  store i64 %322, ptr %264, align 8, !noalias !32993
  %323 = load i64, ptr %267, align 8, !noalias !32993, !noundef !1708
  %324 = call i64 @llvm.sadd.sat.i64(i64 %323, i64 %313)
  store i64 %324, ptr %267, align 8, !noalias !32993
  %325 = load i64, ptr %270, align 8, !noalias !32993, !noundef !1708
  %326 = icmp sgt i64 %324, %325
  br i1 %326, label %327, label %.preheader2534

327:                                              ; preds = %318
  store i64 %324, ptr %270, align 8, !noalias !32993
  br label %.preheader2534

.preheader2534:                                   ; preds = %327, %318
  br label %328

328:                                              ; preds = %.preheader2534, %331
  %329 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !32993
  %330 = icmp slt i64 %329, 0
  br i1 %330, label %331, label %__rustc::__rust_alloc (.exit238)

331:                                              ; preds = %328
  %332 = add nsw i64 %329, 1
  %333 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %329, i64 %332 acq_rel acquire, align 8, !noalias !32993
  %334 = extractvalue { i64, i1 } %333, 1
  br i1 %334, label %335, label %328

335:                                              ; preds = %331
  %336 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !32993
  %337 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %313 monotonic, align 8, !noalias !32993
  %338 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %313 monotonic, align 8, !noalias !32993
  %339 = call i64 @llvm.sadd.sat.i64(i64 %338, i64 %313)
  %340 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !32993
  br label %341

341:                                              ; preds = %344, %335
  %342 = phi i64 [ %340, %335 ], [ %347, %344 ]
  %343 = icmp sgt i64 %339, %342
  br i1 %343, label %344, label %348

344:                                              ; preds = %341
  %345 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %342, i64 %339 monotonic monotonic, align 8, !noalias !32993
  %346 = extractvalue { i64, i1 } %345, 1
  %347 = extractvalue { i64, i1 } %345, 0
  br i1 %346, label %348, label %341

348:                                              ; preds = %344, %341
  %349 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !32993
  br label %__rustc::__rust_alloc (.exit238)

__rustc::__rust_alloc (.exit238): ; preds = %328, %348
  %350 = ptrtoint ptr %316 to i64
  br label %357

351:                                              ; preds = %3305, %3063, %714, %711, %707, %389, %355
  %352 = phi i8 [ 1, %355 ], [ 0, %389 ], [ %663, %3305 ], [ %663, %3063 ], [ 1, %714 ], [ 1, %707 ], [ 1, %711 ]
  %353 = phi i1 [ true, %355 ], [ true, %389 ], [ true, %3305 ], [ false, %3063 ], [ true, %714 ], [ true, %707 ], [ true, %711 ]
  %354 = phi { ptr, i32 } [ %356, %355 ], [ %390, %389 ], [ %3306, %3305 ], [ %3064, %3063 ], [ %708, %714 ], [ %708, %707 ], [ %708, %711 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %144) #89
          to label %302 unwind label %679

355:                                              ; preds = %3313, %2893, %681, %__rustc::__rust_alloc (.exit238.thread)
  %356 = landingpad { ptr, i32 }
          cleanup
  br label %351

__rustc::__rust_alloc (.exit238.thread): ; preds = %315
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %313) #90
          to label %555 unwind label %355

357:                                              ; preds = %__rustc::__rust_alloc (.exit238), %312
  %358 = phi i64 [ %350, %__rustc::__rust_alloc (.exit238) ], [ 8, %312 ]
  %359 = inttoptr i64 %358 to ptr
  store i64 %219, ptr %118, align 8
  %360 = getelementptr inbounds nuw i8, ptr %118, i64 8
  store ptr %359, ptr %360, align 8
  %361 = getelementptr inbounds nuw i8, ptr %118, i64 16
  store i64 0, ptr %361, align 8
  %362 = getelementptr inbounds nuw i8, ptr %152, i64 8
  %363 = load ptr, ptr %362, align 8, !nonnull !1708, !noundef !1708
  %364 = load i64, ptr %152, align 8, !range !1817, !noundef !1708
  %365 = getelementptr inbounds nuw i8, ptr %363, i64 %313
  call void @llvm.lifetime.start.p0(ptr nonnull %117)
  store ptr %363, ptr %117, align 8
  %366 = getelementptr inbounds nuw i8, ptr %117, i64 8
  %367 = getelementptr inbounds nuw i8, ptr %117, i64 16
  store i64 %364, ptr %367, align 8
  %368 = getelementptr inbounds nuw i8, ptr %117, i64 24
  store ptr %365, ptr %368, align 8
  %369 = getelementptr inbounds nuw i8, ptr %117, i64 32
  call void @llvm.lifetime.start.p0(ptr nonnull %116)
  br i1 %314, label %.loopexit319, label %370

370:                                              ; preds = %357
  %371 = getelementptr inbounds nuw i8, ptr %115, i64 8
  %372 = getelementptr inbounds nuw i8, ptr %115, i64 16
  %373 = getelementptr inbounds nuw i8, ptr %103, i64 8
  %374 = getelementptr inbounds nuw i8, ptr %5, i64 568
  %375 = getelementptr inbounds nuw i8, ptr %113, i64 8
  br label %376

376:                                              ; preds = %556, %370
  %377 = phi ptr [ %359, %370 ], [ %557, %556 ]
  %378 = phi i64 [ 0, %370 ], [ %393, %556 ]
  %379 = phi ptr [ %363, %370 ], [ %380, %556 ]
  %380 = getelementptr inbounds nuw i8, ptr %379, i64 40
  %381 = load i64, ptr %379, align 8, !noalias !32996
  %382 = icmp eq i64 %381, 0
  br i1 %382, label %.loopexit319, label %391

383:                                              ; preds = %677, %553
  %384 = phi i64 [ %675, %677 ], [ %543, %553 ]
  %385 = phi ptr [ %678, %677 ], [ %544, %553 ]
  %386 = phi { ptr, i32 } [ %674, %677 ], [ %551, %553 ]
  %387 = shl i64 %384, 3
  %388 = add i64 %387, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %385, i64 noundef %388, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !1708
  br label %389

389:                                              ; preds = %673, %550, %383
  %390 = phi { ptr, i32 } [ %551, %550 ], [ %674, %673 ], [ %386, %383 ]
; call core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
  call fastcc void @core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>(ptr noalias nofree noundef align 8 dereferenceable(40) %117) #89
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %118) #89
  br label %351

391:                                              ; preds = %376
  %392 = getelementptr inbounds nuw i8, ptr %379, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %116, ptr noundef nonnull align 8 dereferenceable(32) %392, i64 32, i1 false), !noalias !33002
  %393 = add nuw nsw i64 %378, 1
  call void @llvm.lifetime.start.p0(ptr nonnull %115)
  store i64 %381, ptr %115, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %371, ptr noundef nonnull align 8 dereferenceable(32) %116, i64 32, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %114)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %114, ptr noalias nofree noundef align 8 dereferenceable(200) %149, ptr noundef nonnull align 16 %5)
          to label %474 unwind label %669

.loopexit319:                                     ; preds = %556, %376, %357
  %394 = phi i64 [ 0, %357 ], [ %378, %376 ], [ %393, %556 ]
  %395 = phi ptr [ %363, %357 ], [ %380, %376 ], [ %365, %556 ]
  store ptr %395, ptr %366, align 8
  store i64 %394, ptr %369, align 8
  br label %396

396:                                              ; preds = %660, %.loopexit319
  %397 = phi ptr [ %380, %660 ], [ %395, %.loopexit319 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %116)
  %398 = ptrtoint ptr %365 to i64
  %399 = ptrtoint ptr %397 to i64
  %400 = sub nuw i64 %398, %399
  %401 = udiv exact i64 %400, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !33003)
  %402 = icmp eq ptr %365, %397
  br i1 %402, label %.loopexit314, label %.preheader313

.preheader313:                                    ; preds = %396
  %403 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %404

404:                                              ; preds = %.preheader313, %442
  %405 = phi i64 [ %407, %442 ], [ 0, %.preheader313 ]
  %406 = getelementptr inbounds nuw [40 x i8], ptr %397, i64 %405
  %407 = add nuw nsw i64 %405, 1
  %408 = load i64, ptr %406, align 8, !range !1940, !alias.scope !33006, !noalias !33009, !noundef !1708
  %409 = icmp ugt i64 %408, 5
  br i1 %409, label %410, label %442

410:                                              ; preds = %404
  %411 = getelementptr i8, ptr %406, i64 8
  %412 = load ptr, ptr %411, align 8, !alias.scope !33003, !noalias !33009, !nonnull !1708, !noundef !1708
  %413 = shl i64 %408, 3
  %414 = add i64 %413, -8
  %415 = load i64, ptr %267, align 8, !noalias !33016, !noundef !1708
  %416 = call i64 @llvm.umin.i64(i64 %414, i64 9223372036854775807)
  %417 = call i64 @llvm.ssub.sat.i64(i64 %415, i64 %416)
  store i64 %417, ptr %267, align 8, !noalias !33016
  %418 = load i64, ptr %403, align 8, !noalias !33016, !noundef !1708
  %419 = icmp slt i64 %417, %418
  br i1 %419, label %420, label %.preheader2497

420:                                              ; preds = %410
  store i64 %417, ptr %403, align 8, !noalias !33016
  br label %.preheader2497

.preheader2497:                                   ; preds = %420, %410
  br label %421

421:                                              ; preds = %.preheader2497, %424
  %422 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33016
  %423 = icmp slt i64 %422, 0
  br i1 %423, label %424, label %__rustc::__rust_dealloc (.exit)

424:                                              ; preds = %421
  %425 = add nsw i64 %422, 1
  %426 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %422, i64 %425 acq_rel acquire, align 8, !noalias !33016
  %427 = extractvalue { i64, i1 } %426, 1
  br i1 %427, label %428, label %421

428:                                              ; preds = %424
  %429 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %416 monotonic, align 8, !noalias !33016
  %430 = call i64 @llvm.ssub.sat.i64(i64 %429, i64 %416)
  %431 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33016
  br label %432

432:                                              ; preds = %435, %428
  %433 = phi i64 [ %431, %428 ], [ %438, %435 ]
  %434 = icmp slt i64 %430, %433
  br i1 %434, label %435, label %439

435:                                              ; preds = %432
  %436 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %433, i64 %430 monotonic monotonic, align 8, !noalias !33016
  %437 = extractvalue { i64, i1 } %436, 1
  %438 = extractvalue { i64, i1 } %436, 0
  br i1 %437, label %439, label %432

439:                                              ; preds = %435, %432
  %440 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33016
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %421, %439
  %441 = icmp ne i64 %414, 0
  call void @llvm.assume(i1 %441), !noalias !33016
  call void @free(ptr noundef nonnull %412) #88, !noalias !33016
  br label %442

442:                                              ; preds = %__rustc::__rust_dealloc (.exit), %404
  %443 = icmp eq i64 %407, %401
  br i1 %443, label %.loopexit314, label %404

.loopexit314:                                     ; preds = %442, %396
  %444 = icmp eq i64 %364, 0
  br i1 %444, label %661, label %445

445:                                              ; preds = %.loopexit314
  %446 = mul nuw i64 %364, 40
  %447 = load i64, ptr %267, align 8, !noalias !33009, !noundef !1708
  %448 = call i64 @llvm.umin.i64(i64 %446, i64 9223372036854775807)
  %449 = call i64 @llvm.ssub.sat.i64(i64 %447, i64 %448)
  store i64 %449, ptr %267, align 8, !noalias !33009
  %450 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %451 = load i64, ptr %450, align 8, !noalias !33009, !noundef !1708
  %452 = icmp slt i64 %449, %451
  br i1 %452, label %453, label %.preheader2496

453:                                              ; preds = %445
  store i64 %449, ptr %450, align 8, !noalias !33009
  br label %.preheader2496

.preheader2496:                                   ; preds = %453, %445
  br label %454

454:                                              ; preds = %.preheader2496, %457
  %455 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33009
  %456 = icmp slt i64 %455, 0
  br i1 %456, label %457, label %__rustc::__rust_dealloc (.exit239)

457:                                              ; preds = %454
  %458 = add nsw i64 %455, 1
  %459 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %455, i64 %458 acq_rel acquire, align 8, !noalias !33009
  %460 = extractvalue { i64, i1 } %459, 1
  br i1 %460, label %461, label %454

461:                                              ; preds = %457
  %462 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %448 monotonic, align 8, !noalias !33009
  %463 = call i64 @llvm.ssub.sat.i64(i64 %462, i64 %448)
  %464 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33009
  br label %465

465:                                              ; preds = %468, %461
  %466 = phi i64 [ %464, %461 ], [ %471, %468 ]
  %467 = icmp slt i64 %463, %466
  br i1 %467, label %468, label %472

468:                                              ; preds = %465
  %469 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %466, i64 %463 monotonic monotonic, align 8, !noalias !33009
  %470 = extractvalue { i64, i1 } %469, 1
  %471 = extractvalue { i64, i1 } %469, 0
  br i1 %470, label %472, label %465

472:                                              ; preds = %468, %465
  %473 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33009
  br label %__rustc::__rust_dealloc (.exit239)

__rustc::__rust_dealloc (.exit239): ; preds = %454, %472
  call void @free(ptr noundef nonnull %363) #88, !noalias !33009
  br label %661

474:                                              ; preds = %391
  %475 = load i8, ptr %114, align 8, !range !1906, !noundef !1708
  %476 = icmp eq i8 %475, -1
  br i1 %476, label %477, label %497

477:                                              ; preds = %474
  call void @llvm.lifetime.end.p0(ptr nonnull %114)
  %478 = load i64, ptr %146, align 8, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33019)
  %479 = load i64, ptr %115, align 8, !range !1940, !alias.scope !33019, !noundef !1708
  %480 = add i64 %479, -1
  %481 = icmp ugt i64 %480, 4
  %482 = load i64, ptr %372, align 8, !alias.scope !33019
  %483 = add i64 %482, -1
  %484 = select i1 %481, i64 %483, i64 %480
  %485 = icmp ugt i64 %478, %484
  br i1 %485, label %494, label %486

486:                                              ; preds = %477
  %487 = icmp ugt i64 %479, 5
  %488 = select i1 %487, i64 %482, i64 %479
  %489 = add i64 %488, -1
  %490 = icmp ult i64 %478, %489
  br i1 %490, label %491, label %504

491:                                              ; preds = %486
  %492 = select i1 %487, ptr %372, ptr %115
  %493 = add nuw i64 %478, 1
  store i64 %493, ptr %492, align 8, !alias.scope !33022
  br label %504

494:                                              ; preds = %477
  call void @llvm.lifetime.start.p0(ptr nonnull %103), !noalias !33019
  %495 = sub nuw i64 %478, %484
  store i32 2, ptr %103, align 8, !noalias !33019
  store i64 %495, ptr %373, align 8, !noalias !33019
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %115, ptr noalias nofree noundef align 8 captures(address) dereferenceable(16) %103)
          to label %496 unwind label %669

496:                                              ; preds = %494
  call void @llvm.lifetime.end.p0(ptr nonnull %103), !noalias !33019
  br label %504

497:                                              ; preds = %474
  store ptr %380, ptr %366, align 8
  store i64 %393, ptr %369, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %114)
  %498 = load i64, ptr %115, align 8, !range !1940, !alias.scope !24096, !noundef !1708
  %499 = icmp ugt i64 %498, 5
  br i1 %499, label %500, label %660

500:                                              ; preds = %497
  %501 = load ptr, ptr %371, align 8, !nonnull !1708, !noundef !1708
  %502 = shl i64 %498, 3
  %503 = add i64 %502, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %501, i64 noundef %503, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33025
  br label %660

504:                                              ; preds = %496, %491, %486
  store i64 %378, ptr %374, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %113)
  %505 = load i64, ptr %115, align 8, !range !1940, !noundef !1708
  %506 = add i64 %505, -1
  %507 = icmp ugt i64 %506, 4
  %508 = load ptr, ptr %371, align 8, !nonnull !1708
  %509 = load i64, ptr %372, align 8
  %510 = add i64 %509, -1
  %511 = select i1 %507, i64 %510, i64 %506
  %512 = select i1 %507, ptr %508, ptr %371
  %513 = load ptr, ptr %145, align 8, !nonnull !1708, !noundef !1708
  %514 = getelementptr inbounds nuw i8, ptr %513, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %113, ptr noalias nofree noundef align 8 dereferenceable(216) %144, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %512, i64 noundef %511, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %514, ptr noalias nofree noundef align 16 dereferenceable(1248) %5)
          to label %515 unwind label %669

515:                                              ; preds = %504
  %516 = load i64, ptr %113, align 16, !range !2530, !noundef !1708
  %517 = icmp eq i64 %516, -1
  br i1 %517, label %530, label %518

518:                                              ; preds = %515
  store ptr %380, ptr %366, align 8
  %519 = getelementptr inbounds nuw i8, ptr %113, i64 16
  %520 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %520, ptr noundef nonnull align 16 dereferenceable(80) %519, i64 80, i1 false)
  %521 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %522 = getelementptr inbounds nuw i8, ptr %0, i64 24
  %523 = load <2 x i32>, ptr %375, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %113)
  store i64 %516, ptr %521, align 16
  store <2 x i32> %523, ptr %522, align 8
  store i64 1, ptr %0, align 16
  %524 = load i64, ptr %115, align 8, !range !1940, !alias.scope !24096, !noundef !1708
  %525 = icmp ugt i64 %524, 5
  br i1 %525, label %526, label %562

526:                                              ; preds = %518
  %527 = load ptr, ptr %371, align 8, !nonnull !1708, !noundef !1708
  %528 = shl i64 %524, 3
  %529 = add i64 %528, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %527, i64 noundef %529, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33028
  br label %562

530:                                              ; preds = %515
  %531 = load <2 x i32>, ptr %375, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %113)
  %532 = load i64, ptr %115, align 8, !range !1940, !noundef !1708
  %533 = icmp ugt i64 %532, 5
  %534 = load i64, ptr %372, align 8
  %535 = select i1 %533, i64 %534, i64 %532
  %536 = add i64 %535, -1
  %537 = load i64, ptr %147, align 8, !noundef !1708
  %538 = icmp ult i64 %537, %536
  br i1 %538, label %539, label %554

539:                                              ; preds = %530
  %540 = load ptr, ptr %371, align 8, !nonnull !1708
  %541 = select i1 %533, ptr %540, ptr %371
  %542 = getelementptr inbounds nuw [8 x i8], ptr %541, i64 %537
  store <2 x i32> %531, ptr %542, align 4
  call void @llvm.lifetime.start.p0(ptr nonnull %112)
  %543 = load i64, ptr %115, align 8
  %544 = load ptr, ptr %371, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %112, ptr noundef nonnull align 8 dereferenceable(24) %372, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !33031)
  %545 = load i64, ptr %118, align 8, !range !1817, !alias.scope !33031, !noalias !33034, !noundef !1708
  %546 = icmp eq i64 %378, %545
  br i1 %546, label %547, label %556

547:                                              ; preds = %539
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %118)
          to label %548 unwind label %550, !noalias !33034

548:                                              ; preds = %547
  %549 = load ptr, ptr %360, align 8, !alias.scope !33031, !noalias !33034
  br label %556

550:                                              ; preds = %547
  %551 = landingpad { ptr, i32 }
          cleanup
  store ptr %380, ptr %366, align 8
  store i64 %393, ptr %369, align 8
  %552 = icmp ugt i64 %543, 5
  br i1 %552, label %553, label %389

553:                                              ; preds = %550
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %544) ]
  br label %383

554:                                              ; preds = %530
  store ptr %380, ptr %366, align 8
  store i64 %393, ptr %369, align 8
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %537, i64 noundef %536, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.388) #90
          to label %555 unwind label %671

555:                                              ; preds = %3038, %2897, %554, %__rustc::__rust_alloc (.exit238.thread)
  unreachable

556:                                              ; preds = %548, %539
  %557 = phi ptr [ %549, %548 ], [ %377, %539 ]
  %558 = getelementptr inbounds nuw [40 x i8], ptr %557, i64 %378
  store i64 %543, ptr %558, align 8, !noalias !33031
  %559 = getelementptr inbounds nuw i8, ptr %558, i64 8
  store ptr %544, ptr %559, align 8, !noalias !33031
  %560 = getelementptr inbounds nuw i8, ptr %558, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %560, ptr noundef nonnull align 8 dereferenceable(24) %112, i64 24, i1 false), !noalias !33031
  store i64 %393, ptr %361, align 8, !alias.scope !33031, !noalias !33034
  call void @llvm.lifetime.end.p0(ptr nonnull %112)
  call void @llvm.lifetime.end.p0(ptr nonnull %115)
  call void @llvm.lifetime.end.p0(ptr nonnull %116)
  call void @llvm.lifetime.start.p0(ptr nonnull %116)
  %561 = icmp eq ptr %380, %365
  br i1 %561, label %.loopexit319, label %376

562:                                              ; preds = %526, %518
  call void @llvm.lifetime.end.p0(ptr nonnull %115)
  call void @llvm.lifetime.end.p0(ptr nonnull %116)
  %563 = ptrtoint ptr %365 to i64
  %564 = ptrtoint ptr %380 to i64
  %565 = sub nuw i64 %563, %564
  %566 = udiv exact i64 %565, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !33036)
  %567 = icmp eq ptr %365, %380
  br i1 %567, label %.loopexit318, label %.preheader317

.preheader317:                                    ; preds = %562
  %568 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %569

569:                                              ; preds = %.preheader317, %607
  %570 = phi i64 [ %572, %607 ], [ 0, %.preheader317 ]
  %571 = getelementptr inbounds nuw [40 x i8], ptr %380, i64 %570
  %572 = add nuw nsw i64 %570, 1
  %573 = load i64, ptr %571, align 8, !range !1940, !alias.scope !33039, !noalias !33042, !noundef !1708
  %574 = icmp ugt i64 %573, 5
  br i1 %574, label %575, label %607

575:                                              ; preds = %569
  %576 = getelementptr i8, ptr %571, i64 8
  %577 = load ptr, ptr %576, align 8, !alias.scope !33036, !noalias !33042, !nonnull !1708, !noundef !1708
  %578 = shl i64 %573, 3
  %579 = add i64 %578, -8
  %580 = load i64, ptr %267, align 8, !noalias !33049, !noundef !1708
  %581 = call i64 @llvm.umin.i64(i64 %579, i64 9223372036854775807)
  %582 = call i64 @llvm.ssub.sat.i64(i64 %580, i64 %581)
  store i64 %582, ptr %267, align 8, !noalias !33049
  %583 = load i64, ptr %568, align 8, !noalias !33049, !noundef !1708
  %584 = icmp slt i64 %582, %583
  br i1 %584, label %585, label %.preheader2499

585:                                              ; preds = %575
  store i64 %582, ptr %568, align 8, !noalias !33049
  br label %.preheader2499

.preheader2499:                                   ; preds = %585, %575
  br label %586

586:                                              ; preds = %.preheader2499, %589
  %587 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33049
  %588 = icmp slt i64 %587, 0
  br i1 %588, label %589, label %__rustc::__rust_dealloc (.exit240)

589:                                              ; preds = %586
  %590 = add nsw i64 %587, 1
  %591 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %587, i64 %590 acq_rel acquire, align 8, !noalias !33049
  %592 = extractvalue { i64, i1 } %591, 1
  br i1 %592, label %593, label %586

593:                                              ; preds = %589
  %594 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %581 monotonic, align 8, !noalias !33049
  %595 = call i64 @llvm.ssub.sat.i64(i64 %594, i64 %581)
  %596 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33049
  br label %597

597:                                              ; preds = %600, %593
  %598 = phi i64 [ %596, %593 ], [ %603, %600 ]
  %599 = icmp slt i64 %595, %598
  br i1 %599, label %600, label %604

600:                                              ; preds = %597
  %601 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %598, i64 %595 monotonic monotonic, align 8, !noalias !33049
  %602 = extractvalue { i64, i1 } %601, 1
  %603 = extractvalue { i64, i1 } %601, 0
  br i1 %602, label %604, label %597

604:                                              ; preds = %600, %597
  %605 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33049
  br label %__rustc::__rust_dealloc (.exit240)

__rustc::__rust_dealloc (.exit240): ; preds = %586, %604
  %606 = icmp ne i64 %579, 0
  call void @llvm.assume(i1 %606), !noalias !33049
  call void @free(ptr noundef nonnull %577) #88, !noalias !33049
  br label %607

607:                                              ; preds = %__rustc::__rust_dealloc (.exit240), %569
  %608 = icmp eq i64 %572, %566
  br i1 %608, label %.loopexit318, label %569

.loopexit318:                                     ; preds = %607, %562
  %609 = icmp eq i64 %364, 0
  br i1 %609, label %612, label %610

610:                                              ; preds = %.loopexit318
  %611 = mul nuw i64 %364, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %363, i64 noundef %611, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33042
  br label %612

612:                                              ; preds = %610, %.loopexit318
  call void @llvm.lifetime.end.p0(ptr nonnull %117)
  call void @llvm.experimental.noalias.scope.decl(metadata !33052)
  call void @llvm.experimental.noalias.scope.decl(metadata !33055)
  %613 = icmp eq i64 %378, 0
  br i1 %613, label %.loopexit316, label %.preheader315

.preheader315:                                    ; preds = %612
  %614 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %615

615:                                              ; preds = %.preheader315, %653
  %616 = phi i64 [ %618, %653 ], [ 0, %.preheader315 ]
  %617 = getelementptr inbounds nuw [40 x i8], ptr %377, i64 %616
  %618 = add nuw nsw i64 %616, 1
  %619 = load i64, ptr %617, align 8, !range !1940, !alias.scope !33058, !noalias !33052, !noundef !1708
  %620 = icmp ugt i64 %619, 5
  br i1 %620, label %621, label %653

621:                                              ; preds = %615
  %622 = getelementptr i8, ptr %617, i64 8
  %623 = load ptr, ptr %622, align 8, !alias.scope !33055, !noalias !33052, !nonnull !1708, !noundef !1708
  %624 = shl i64 %619, 3
  %625 = add i64 %624, -8
  %626 = load i64, ptr %267, align 8, !noalias !33061, !noundef !1708
  %627 = call i64 @llvm.umin.i64(i64 %625, i64 9223372036854775807)
  %628 = call i64 @llvm.ssub.sat.i64(i64 %626, i64 %627)
  store i64 %628, ptr %267, align 8, !noalias !33061
  %629 = load i64, ptr %614, align 8, !noalias !33061, !noundef !1708
  %630 = icmp slt i64 %628, %629
  br i1 %630, label %631, label %.preheader2498

631:                                              ; preds = %621
  store i64 %628, ptr %614, align 8, !noalias !33061
  br label %.preheader2498

.preheader2498:                                   ; preds = %631, %621
  br label %632

632:                                              ; preds = %.preheader2498, %635
  %633 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33061
  %634 = icmp slt i64 %633, 0
  br i1 %634, label %635, label %__rustc::__rust_dealloc (.exit241)

635:                                              ; preds = %632
  %636 = add nsw i64 %633, 1
  %637 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %633, i64 %636 acq_rel acquire, align 8, !noalias !33061
  %638 = extractvalue { i64, i1 } %637, 1
  br i1 %638, label %639, label %632

639:                                              ; preds = %635
  %640 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %627 monotonic, align 8, !noalias !33061
  %641 = call i64 @llvm.ssub.sat.i64(i64 %640, i64 %627)
  %642 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33061
  br label %643

643:                                              ; preds = %646, %639
  %644 = phi i64 [ %642, %639 ], [ %649, %646 ]
  %645 = icmp slt i64 %641, %644
  br i1 %645, label %646, label %650

646:                                              ; preds = %643
  %647 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %644, i64 %641 monotonic monotonic, align 8, !noalias !33061
  %648 = extractvalue { i64, i1 } %647, 1
  %649 = extractvalue { i64, i1 } %647, 0
  br i1 %648, label %650, label %643

650:                                              ; preds = %646, %643
  %651 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33061
  br label %__rustc::__rust_dealloc (.exit241)

__rustc::__rust_dealloc (.exit241): ; preds = %632, %650
  %652 = icmp ne i64 %625, 0
  call void @llvm.assume(i1 %652), !noalias !33061
  call void @free(ptr noundef nonnull %623) #88, !noalias !33061
  br label %653

653:                                              ; preds = %__rustc::__rust_dealloc (.exit241), %615
  %654 = icmp eq i64 %618, %378
  br i1 %654, label %.loopexit316, label %615

.loopexit316:                                     ; preds = %653, %612
  %655 = load i64, ptr %118, align 8, !alias.scope !33052
  %656 = icmp eq i64 %655, 0
  br i1 %656, label %659, label %657

657:                                              ; preds = %.loopexit316
  %658 = mul nuw i64 %655, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %377, i64 noundef %658, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33052
  br label %659

659:                                              ; preds = %657, %.loopexit316
  call void @llvm.lifetime.end.p0(ptr nonnull %118)
  br label %3210

660:                                              ; preds = %500, %497
  call void @llvm.lifetime.end.p0(ptr nonnull %115)
  br label %396

661:                                              ; preds = %__rustc::__rust_dealloc (.exit239), %.loopexit314
  call void @llvm.lifetime.end.p0(ptr nonnull %117)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %143, ptr noundef nonnull align 8 dereferenceable(24) %118, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %118)
  br label %662

662:                                              ; preds = %3062, %661
  %663 = phi i8 [ 1, %3062 ], [ 0, %661 ]
  %664 = getelementptr inbounds nuw i8, ptr %5, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !33064)
  %665 = load ptr, ptr %664, align 8, !alias.scope !33064, !noalias !33067, !nonnull !1708, !noundef !1708
  %666 = getelementptr inbounds nuw i8, ptr %665, i64 40
  %667 = load atomic i32, ptr %666 acquire, align 4, !noalias !33069
  %668 = icmp eq i32 %667, 0
  br i1 %668, label %3065, label %3075

669:                                              ; preds = %504, %494, %391
  %670 = landingpad { ptr, i32 }
          cleanup
  store ptr %380, ptr %366, align 8
  store i64 %393, ptr %369, align 8
  br label %673

671:                                              ; preds = %554
  %672 = landingpad { ptr, i32 }
          cleanup
  br label %673

673:                                              ; preds = %671, %669
  %674 = phi { ptr, i32 } [ %670, %669 ], [ %672, %671 ]
  %675 = load i64, ptr %115, align 8, !range !1940, !alias.scope !24096, !noundef !1708
  %676 = icmp ugt i64 %675, 5
  br i1 %676, label %677, label %389

677:                                              ; preds = %673
  %678 = load ptr, ptr %371, align 8, !nonnull !1708, !noundef !1708
  br label %383

679:                                              ; preds = %3654, %3650, %3649, %3504, %3503, %3318, %3316, %2912, %714, %351, %235, %190
  %680 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86
  unreachable

681:                                              ; preds = %311
  %682 = getelementptr inbounds nuw i8, ptr %149, i64 184
  %683 = load i64, ptr %682, align 8, !noundef !1708
  %684 = call noundef range(i64 0, 230584300921369396) i64 @llvm.umin.i64(i64 %683, i64 range(i64 0, 230584300921369396) %219)
  %685 = getelementptr inbounds nuw i8, ptr %152, i64 8
  %686 = load ptr, ptr %685, align 8, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %142)
  %687 = getelementptr inbounds nuw i8, ptr %5, i64 888
  %688 = getelementptr inbounds nuw i8, ptr %5, i64 904
  %689 = load i64, ptr %688, align 8, !noundef !1708
  %690 = getelementptr inbounds nuw i8, ptr %5, i64 1040
  %691 = load i64, ptr %690, align 16, !noundef !1708
  %692 = icmp ult i64 %689, 115292150460684698
  call void @llvm.assume(i1 %692)
  %693 = add i64 %691, %689
  store i64 %693, ptr %142, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %141)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
  %694 = invoke fastcc noundef ptr @<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot(ptr noundef nonnull align 16 %5, i64 noundef %684)
          to label %695 unwind label %355

695:                                              ; preds = %681
  store ptr %694, ptr %141, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %139)
  call void @llvm.lifetime.start.p0(ptr nonnull %138)
  %696 = getelementptr inbounds nuw i8, ptr %5, i64 616
  %697 = load ptr, ptr %696, align 8, !noundef !1708
  %698 = icmp eq ptr %697, null
  br i1 %698, label %717, label %699

699:                                              ; preds = %695
  %700 = getelementptr inbounds nuw i8, ptr %697, i64 16
  %701 = load i64, ptr %700, align 8
  %702 = icmp ugt i64 %701, -3
  br i1 %702, label %703, label %717

703:                                              ; preds = %699
  %704 = getelementptr inbounds nuw i8, ptr %697, i64 40
  %705 = load i64, ptr %704, align 8
  %706 = icmp ult i64 %705, -2
  br label %717

707:                                              ; preds = %3318, %3316, %2878, %1132, %927, %923, %818, %715
  %708 = phi { ptr, i32 } [ %3319, %3318 ], [ %1133, %1132 ], [ %819, %818 ], [ %716, %715 ], [ %924, %927 ], [ %924, %923 ], [ %3317, %3316 ], [ %2879, %2878 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !33070)
  %709 = load ptr, ptr %141, align 8, !alias.scope !33070, !noundef !1708
  %710 = icmp eq ptr %709, null
  br i1 %710, label %351, label %711

711:                                              ; preds = %707
  %712 = atomicrmw sub ptr %709, i64 1 release, align 8, !noalias !33073
  %713 = icmp eq i64 %712, 1
  br i1 %713, label %714, label %351

714:                                              ; preds = %711
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %141) #87
          to label %351 unwind label %679, !inline_history !1744

715:                                              ; preds = %841, %744, %843, %829, %822, %737
  %716 = landingpad { ptr, i32 }
          cleanup
  br label %707

717:                                              ; preds = %703, %699, %695
  %718 = phi i1 [ false, %695 ], [ true, %699 ], [ %706, %703 ]
  %719 = getelementptr inbounds nuw i8, ptr %5, i64 1234
  %720 = load i8, ptr %719, align 2, !range !1746, !noundef !1708
  %721 = trunc nuw i8 %720 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %137)
  store ptr %5, ptr %137, align 8
  %722 = getelementptr inbounds nuw i8, ptr %137, i64 8
  store ptr %141, ptr %722, align 8
  %723 = getelementptr inbounds nuw i8, ptr %137, i64 16
  store ptr %149, ptr %723, align 8
  %724 = getelementptr inbounds nuw i8, ptr %137, i64 24
  store ptr %144, ptr %724, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %136)
  store ptr %686, ptr %136, align 8
  %725 = getelementptr inbounds nuw i8, ptr %136, i64 8
  store i64 %684, ptr %725, align 8
  %726 = getelementptr inbounds nuw i8, ptr %136, i64 16
  store ptr %146, ptr %726, align 8
  %727 = getelementptr inbounds nuw i8, ptr %136, i64 24
  store ptr %145, ptr %727, align 8
  %728 = getelementptr inbounds nuw i8, ptr %136, i64 32
  store ptr %147, ptr %728, align 8
  %729 = getelementptr inbounds nuw i8, ptr %136, i64 40
  store ptr %142, ptr %729, align 8
  br i1 %718, label %826, label %730

730:                                              ; preds = %717
  call void @llvm.lifetime.start.p0(ptr nonnull %17)
  call void @llvm.lifetime.start.p0(ptr nonnull %18)
  store ptr %686, ptr %18, align 8, !noalias !33076
  %731 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store i64 %684, ptr %731, align 8, !noalias !33076
  store ptr %149, ptr %17, align 8, !noalias !33076
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !33076
  store ptr %137, ptr %16, align 8, !noalias !33076
  %732 = getelementptr inbounds nuw i8, ptr %16, i64 8
  store ptr %18, ptr %732, align 8, !noalias !33076
  %733 = getelementptr inbounds nuw i8, ptr %16, i64 16
  store ptr %136, ptr %733, align 8, !noalias !33076
  %734 = getelementptr inbounds nuw i8, ptr %16, i64 24
  store ptr %17, ptr %734, align 8, !noalias !33076
  %735 = icmp samesign ult i64 %684, 1025
  %736 = or i1 %735, %721
  br i1 %736, label %737, label %738

737:                                              ; preds = %730
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#0}
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#0}(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %138, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) %16) #91
          to label %825 unwind label %715, !inline_history !33082

738:                                              ; preds = %730
  %739 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %740 = load ptr, ptr %739, align 8, !noundef !1708
  %741 = icmp eq ptr %740, null
  br i1 %741, label %744, label %742

742:                                              ; preds = %738
  %743 = getelementptr inbounds nuw i8, ptr %740, i64 272
  br label %746

744:                                              ; preds = %738
; invoke rayon_core::registry::global_registry
  %745 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %._crit_edge unwind label %715

._crit_edge:                                      ; preds = %744
  %.pre = load ptr, ptr %18, align 8, !noalias !33076
  %.pre1014 = load i64, ptr %731, align 8, !noalias !33076
  br label %746

746:                                              ; preds = %._crit_edge, %742
  %747 = phi i64 [ %684, %742 ], [ %.pre1014, %._crit_edge ]
  %748 = phi ptr [ %686, %742 ], [ %.pre, %._crit_edge ]
  %749 = phi ptr [ %743, %742 ], [ %745, %._crit_edge ]
  %750 = load ptr, ptr %749, align 8, !nonnull !1708, !noundef !1708
  %751 = getelementptr inbounds nuw i8, ptr %750, i64 520
  %752 = load i64, ptr %751, align 8, !noundef !1708
  %753 = icmp ult i64 %752, 192153584101141163
  call void @llvm.assume(i1 %753)
  %754 = call i64 @llvm.umax.i64(i64 %752, i64 1)
  %755 = shl nuw nsw i64 %754, 2
  %756 = udiv i64 %684, %755
  %757 = call noundef range(i64 16, 0) i64 @llvm.umax.i64(i64 %756, i64 16)
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !33076
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !33083
  store i64 0, ptr %14, align 8, !alias.scope !33087, !noalias !33083
  %758 = getelementptr inbounds nuw i8, ptr %14, i64 8
  store ptr inttoptr (i64 16 to ptr), ptr %758, align 8, !alias.scope !33087, !noalias !33083
  %759 = getelementptr inbounds nuw i8, ptr %14, i64 16
  store i64 0, ptr %759, align 8, !alias.scope !33087, !noalias !33083
  call void @llvm.experimental.noalias.scope.decl(metadata !33090)
  %760 = udiv i64 %747, %757
  %761 = urem i64 %747, %757
  %762 = icmp ne i64 %761, 0
  %763 = zext i1 %762 to i64
  %764 = add nuw nsw i64 %760, %763
  call void @llvm.experimental.noalias.scope.decl(metadata !33093), !noalias !33096
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !33097
  store i64 %764, ptr %13, align 8, !noalias !33099
  %765 = icmp eq i64 %764, 0
  br i1 %765, label %770, label %766, !prof !1974

766:                                              ; preds = %746
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %14, i64 noundef 0, i64 noundef %764, i64 noundef 16, i64 noundef 256)
          to label %767 unwind label %816, !inline_history !33101

767:                                              ; preds = %766
  %768 = load i64, ptr %759, align 8, !alias.scope !33102, !noalias !33105
  %769 = load i64, ptr %14, align 8, !range !1817, !alias.scope !33102, !noalias !33105
  br label %770

770:                                              ; preds = %767, %746
  %771 = phi i64 [ %769, %767 ], [ 0, %746 ]
  %772 = phi i64 [ %768, %767 ], [ 0, %746 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !33099
  %773 = icmp ult i64 %772, 36028797018963968
  call void @llvm.assume(i1 %773), !noalias !33106
  %774 = sub nsw i64 %771, %772
  %775 = icmp ult i64 %774, %764
  br i1 %775, label %776, label %778, !prof !1803

776:                                              ; preds = %770
; invoke core::panicking::panic
  invoke void @core::panicking::panic(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.a12f493ba210922c94e5446ac885c35e.965, i64 noundef 47, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.967) #92
          to label %777 unwind label %816, !inline_history !33101

777:                                              ; preds = %776
  unreachable

778:                                              ; preds = %770
  %779 = load ptr, ptr %758, align 8, !alias.scope !33102, !noalias !33105, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !33107
  store ptr %748, ptr %9, align 8, !noalias !33111
  %780 = getelementptr inbounds nuw i8, ptr %9, i64 8
  store i64 %747, ptr %780, align 8, !noalias !33111
  %781 = getelementptr inbounds nuw i8, ptr %9, i64 16
  store i64 %757, ptr %781, align 8, !noalias !33111
  %782 = getelementptr inbounds nuw i8, ptr %9, i64 24
  store ptr %137, ptr %782, align 8, !noalias !33111
  %783 = getelementptr inbounds nuw i8, ptr %9, i64 32
  store ptr %136, ptr %783, align 8, !noalias !33111
  %784 = getelementptr inbounds nuw i8, ptr %9, i64 40
  store ptr %17, ptr %784, align 8, !noalias !33111
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !33112
  %785 = getelementptr inbounds nuw i8, ptr %8, i64 16
  store i64 %757, ptr %785, align 8, !noalias !33112
  store ptr %748, ptr %8, align 8, !noalias !33112
  %786 = getelementptr inbounds nuw i8, ptr %8, i64 8
  store i64 %747, ptr %786, align 8, !noalias !33112
  %787 = load ptr, ptr %739, align 8, !noundef !1708
  %788 = icmp eq ptr %787, null
  br i1 %788, label %791, label %789

789:                                              ; preds = %778
  %790 = getelementptr inbounds nuw i8, ptr %787, i64 272
  br label %793

791:                                              ; preds = %778
; invoke rayon_core::registry::global_registry
  %792 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %793 unwind label %816

793:                                              ; preds = %789, %791
  %794 = phi ptr [ %790, %789 ], [ %792, %791 ]
  %795 = load ptr, ptr %794, align 8, !nonnull !1708, !noundef !1708
  %796 = getelementptr inbounds nuw i8, ptr %795, i64 520
  %797 = load i64, ptr %796, align 8, !noundef !1708
  %798 = icmp ult i64 %797, 192153584101141163
  call void @llvm.assume(i1 %798)
  %799 = getelementptr inbounds nuw [256 x i8], ptr %779, i64 %772
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !33123
  store ptr %782, ptr %7, align 8, !noalias !33128
  %800 = getelementptr inbounds nuw i8, ptr %7, i64 8
  store ptr %799, ptr %800, align 8, !noalias !33128
  %801 = getelementptr inbounds nuw i8, ptr %7, i64 16
  store i64 %764, ptr %801, align 8, !noalias !33128
; invoke rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#1}>>
  invoke fastcc void @rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#1}>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %12, i64 noundef %764, i1 noundef zeroext false, i64 noundef %797, i64 noundef 1, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %8, ptr noalias nofree noundef readonly align 8 captures(address) dereferenceable(24) %7)
          to label %802 unwind label %816, !inline_history !33129

802:                                              ; preds = %793
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !33123
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !33112
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !33107
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !33099
  %803 = getelementptr inbounds nuw i8, ptr %12, i64 16
  %804 = load i64, ptr %803, align 8, !noalias !33099, !noundef !1708
  store i64 %804, ptr %11, align 8, !noalias !33099
  %805 = icmp eq i64 %804, %764
  br i1 %805, label %822, label %806, !prof !1974

806:                                              ; preds = %802
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !33099
  store ptr %13, ptr %10, align 8, !noalias !33099
  %807 = getelementptr inbounds nuw i8, ptr %10, i64 8
  store ptr @<usize as core::fmt::Display>::fmt, ptr %807, align 8, !noalias !33099
  %808 = getelementptr inbounds nuw i8, ptr %10, i64 16
  store ptr %11, ptr %808, align 8, !noalias !33099
  %809 = getelementptr inbounds nuw i8, ptr %10, i64 24
  store ptr @<usize as core::fmt::Display>::fmt, ptr %809, align 8, !noalias !33099
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.a12f493ba210922c94e5446ac885c35e.550, ptr noundef nonnull %10, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.552) #90
          to label %813 unwind label %810, !noalias !33099, !inline_history !33130

810:                                              ; preds = %806
  %811 = landingpad { ptr, i32 }
          cleanup
  %812 = load ptr, ptr %12, align 8, !noalias !33099, !noundef !1708
; invoke core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>(ptr %812, i64 %804) #89
          to label %818 unwind label %814, !noalias !33099, !inline_history !33130

813:                                              ; preds = %806
  unreachable

814:                                              ; preds = %810
  %815 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33099, !inline_history !33130
  unreachable

816:                                              ; preds = %791, %793, %776, %766
  %817 = landingpad { ptr, i32 }
          cleanup
  br label %818

818:                                              ; preds = %816, %810
  %819 = phi { ptr, i32 } [ %817, %816 ], [ %811, %810 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %14) #89
          to label %707 unwind label %820, !noalias !33096, !inline_history !33131

820:                                              ; preds = %818
  %821 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33096, !inline_history !33131
  unreachable

822:                                              ; preds = %802
  %823 = add nuw nsw i64 %772, %764
  store i64 %823, ptr %759, align 8, !alias.scope !33132, !noalias !33105
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !33099
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !33099
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !33097
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %15, ptr noundef nonnull align 8 dereferenceable(24) %14, i64 24, i1 false), !noalias !33133
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !33083
; invoke purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
  invoke fastcc void @purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %138, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %15)
          to label %824 unwind label %715, !inline_history !33082

824:                                              ; preds = %822
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !33076
  br label %825

825:                                              ; preds = %824, %737
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !33076
  call void @llvm.lifetime.end.p0(ptr nonnull %17)
  call void @llvm.lifetime.end.p0(ptr nonnull %18)
  br label %1101

826:                                              ; preds = %717
  call void @llvm.lifetime.start.p0(ptr nonnull %33)
  store ptr %149, ptr %33, align 8, !noalias !33134
  %827 = icmp samesign ult i64 %684, 1025
  %828 = or i1 %827, %721
  br i1 %828, label %829, label %835

829:                                              ; preds = %826
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !33134
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %32, ptr noundef nonnull readonly align 8 dereferenceable(32) %137, i64 32, i1 false), !noalias !33140
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !33134
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %31, ptr noundef nonnull readonly align 8 dereferenceable(48) %136, i64 48, i1 false), !noalias !33141
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !33134
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !33134
  store ptr %686, ptr %23, align 8, !noalias !33142
  %830 = getelementptr inbounds nuw i8, ptr %23, i64 8
  store i64 %684, ptr %830, align 8, !noalias !33142
  store ptr %149, ptr %22, align 8, !noalias !33142
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !33142
  store ptr %32, ptr %21, align 8, !noalias !33142
  %831 = getelementptr inbounds nuw i8, ptr %21, i64 8
  store ptr %23, ptr %831, align 8, !noalias !33142
  %832 = getelementptr inbounds nuw i8, ptr %21, i64 16
  store ptr %31, ptr %832, align 8, !noalias !33142
  %833 = getelementptr inbounds nuw i8, ptr %21, i64 24
  store ptr %22, ptr %833, align 8, !noalias !33142
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#0}
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#0}(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %138, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) %21) #91
          to label %834 unwind label %715, !inline_history !33148

834:                                              ; preds = %829
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !33142
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !33134
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !33134
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !33134
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !33134
  br label %1100

835:                                              ; preds = %826
  %836 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %837 = load ptr, ptr %836, align 8, !noundef !1708
  %838 = icmp eq ptr %837, null
  br i1 %838, label %841, label %839

839:                                              ; preds = %835
  %840 = getelementptr inbounds nuw i8, ptr %837, i64 272
  br label %843

841:                                              ; preds = %835
; invoke rayon_core::registry::global_registry
  %842 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %843 unwind label %715

843:                                              ; preds = %839, %841
  %844 = phi ptr [ %840, %839 ], [ %842, %841 ]
  %845 = load ptr, ptr %844, align 8, !nonnull !1708, !noundef !1708
  %846 = getelementptr inbounds nuw i8, ptr %845, i64 520
  %847 = load i64, ptr %846, align 8, !noundef !1708
  %848 = icmp ult i64 %847, 192153584101141163
  call void @llvm.assume(i1 %848)
  %849 = call i64 @llvm.umax.i64(i64 %847, i64 1)
  %850 = shl nuw i64 %849, 6
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !33134
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !33134
  %851 = udiv i64 %684, %850
  %852 = call i64 @llvm.umax.i64(i64 %851, i64 64)
  store ptr %686, ptr %29, align 8, !noalias !33134
  %853 = getelementptr inbounds nuw i8, ptr %29, i64 8
  store i64 %684, ptr %853, align 8, !noalias !33134
  %854 = getelementptr inbounds nuw i8, ptr %29, i64 16
  store i64 %852, ptr %854, align 8, !noalias !33134
; invoke <alloc::vec::Vec<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>]> as alloc::vec::spec_from_iter::SpecFromIter<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>], core::slice::iter::Chunks<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>>::from_iter
  invoke fastcc void @<alloc::vec::Vec<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>]> as alloc::vec::spec_from_iter::SpecFromIter<&[purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>], core::slice::iter::Chunks<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>>::from_iter(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %30, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %29)
          to label %855 unwind label %715, !inline_history !33148

855:                                              ; preds = %843
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !33134
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !33134
  %856 = getelementptr inbounds nuw i8, ptr %30, i64 8
  %857 = getelementptr inbounds nuw i8, ptr %30, i64 16
  %858 = load i64, ptr %857, align 8, !noalias !33134, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33149)
  call void @llvm.experimental.noalias.scope.decl(metadata !33152)
  %859 = mul i64 %858, 272
  %860 = icmp ugt i64 %858, 33909456017848440
  br i1 %860, label %866, label %861, !prof !5895

861:                                              ; preds = %855
  %862 = icmp eq i64 %859, 0
  br i1 %862, label %869, label %863

863:                                              ; preds = %861
; call __rustc::__rust_alloc
  %864 = call noundef align 16 ptr @__rustc::__rust_alloc(i64 noundef %859, i64 noundef range(i64 1, 17) 16) #88, !noalias !33155, !inline_history !33148
  %865 = icmp eq ptr %864, null
  br i1 %865, label %866, label %869

866:                                              ; preds = %863, %855
  %867 = phi i64 [ 16, %863 ], [ 0, %855 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %867, i64 %859) #90
          to label %868 unwind label %930, !noalias !33158, !inline_history !33148

868:                                              ; preds = %866
  unreachable

869:                                              ; preds = %863, %861
  %870 = phi i64 [ 0, %861 ], [ %858, %863 ]
  %871 = phi ptr [ inttoptr (i64 16 to ptr), %861 ], [ %864, %863 ]
  %872 = icmp samesign ule i64 %858, %870
  call void @llvm.assume(i1 %872)
  %873 = icmp eq i64 %858, 0
  br i1 %873, label %.loopexit312, label %.preheader311.preheader

.preheader311.preheader:                          ; preds = %869
  %xtraiter = and i64 %858, 7
  %874 = icmp ult i64 %858, 8
  br i1 %874, label %.preheader311.epil.preheader, label %.preheader311.preheader.new

.preheader311.preheader.new:                      ; preds = %.preheader311.preheader
  %unroll_iter = and i64 %858, 36028797018963960
  br label %.preheader311

.preheader311:                                    ; preds = %.preheader311, %.preheader311.preheader.new
  %875 = phi i64 [ 0, %.preheader311.preheader.new ], [ %907, %.preheader311 ]
  %niter = phi i64 [ 0, %.preheader311.preheader.new ], [ %niter.next.7, %.preheader311 ]
  %876 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  store i32 0, ptr %876, align 16, !noalias !33159
  %877 = getelementptr inbounds nuw i8, ptr %876, i64 4
  store i8 0, ptr %877, align 4, !noalias !33159
  %878 = getelementptr inbounds nuw i8, ptr %876, i64 16
  store i64 2, ptr %878, align 16, !noalias !33159
  %879 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  %880 = getelementptr inbounds nuw i8, ptr %879, i64 272
  store i32 0, ptr %880, align 16, !noalias !33159
  %881 = getelementptr inbounds nuw i8, ptr %879, i64 276
  store i8 0, ptr %881, align 4, !noalias !33159
  %882 = getelementptr inbounds nuw i8, ptr %879, i64 288
  store i64 2, ptr %882, align 16, !noalias !33159
  %883 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  %884 = getelementptr inbounds nuw i8, ptr %883, i64 544
  store i32 0, ptr %884, align 16, !noalias !33159
  %885 = getelementptr inbounds nuw i8, ptr %883, i64 548
  store i8 0, ptr %885, align 4, !noalias !33159
  %886 = getelementptr inbounds nuw i8, ptr %883, i64 560
  store i64 2, ptr %886, align 16, !noalias !33159
  %887 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  %888 = getelementptr inbounds nuw i8, ptr %887, i64 816
  store i32 0, ptr %888, align 16, !noalias !33159
  %889 = getelementptr inbounds nuw i8, ptr %887, i64 820
  store i8 0, ptr %889, align 4, !noalias !33159
  %890 = getelementptr inbounds nuw i8, ptr %887, i64 832
  store i64 2, ptr %890, align 16, !noalias !33159
  %891 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  %892 = getelementptr inbounds nuw i8, ptr %891, i64 1088
  store i32 0, ptr %892, align 16, !noalias !33159
  %893 = getelementptr inbounds nuw i8, ptr %891, i64 1092
  store i8 0, ptr %893, align 4, !noalias !33159
  %894 = getelementptr inbounds nuw i8, ptr %891, i64 1104
  store i64 2, ptr %894, align 16, !noalias !33159
  %895 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  %896 = getelementptr inbounds nuw i8, ptr %895, i64 1360
  store i32 0, ptr %896, align 16, !noalias !33159
  %897 = getelementptr inbounds nuw i8, ptr %895, i64 1364
  store i8 0, ptr %897, align 4, !noalias !33159
  %898 = getelementptr inbounds nuw i8, ptr %895, i64 1376
  store i64 2, ptr %898, align 16, !noalias !33159
  %899 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  %900 = getelementptr inbounds nuw i8, ptr %899, i64 1632
  store i32 0, ptr %900, align 16, !noalias !33159
  %901 = getelementptr inbounds nuw i8, ptr %899, i64 1636
  store i8 0, ptr %901, align 4, !noalias !33159
  %902 = getelementptr inbounds nuw i8, ptr %899, i64 1648
  store i64 2, ptr %902, align 16, !noalias !33159
  %903 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %875
  %904 = getelementptr inbounds nuw i8, ptr %903, i64 1904
  store i32 0, ptr %904, align 16, !noalias !33159
  %905 = getelementptr inbounds nuw i8, ptr %903, i64 1908
  store i8 0, ptr %905, align 4, !noalias !33159
  %906 = getelementptr inbounds nuw i8, ptr %903, i64 1920
  store i64 2, ptr %906, align 16, !noalias !33159
  %907 = add nuw i64 %875, 8
  %niter.next.7 = add i64 %niter, 8
  %niter.ncmp.7 = icmp eq i64 %niter.next.7, %unroll_iter
  br i1 %niter.ncmp.7, label %.loopexit312.loopexit.unr-lcssa, label %.preheader311

.loopexit312.loopexit.unr-lcssa:                  ; preds = %.preheader311
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.loopexit312, label %.preheader311.epil.preheader

.preheader311.epil.preheader:                     ; preds = %.loopexit312.loopexit.unr-lcssa, %.preheader311.preheader
  %.epil.init = phi i64 [ 0, %.preheader311.preheader ], [ %907, %.loopexit312.loopexit.unr-lcssa ]
  %lcmp.mod2536 = icmp ne i64 %xtraiter, 0
  call void @llvm.assume(i1 %lcmp.mod2536)
  br label %.preheader311.epil

.preheader311.epil:                               ; preds = %.preheader311.epil, %.preheader311.epil.preheader
  %908 = phi i64 [ %912, %.preheader311.epil ], [ %.epil.init, %.preheader311.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %.preheader311.epil ], [ 0, %.preheader311.epil.preheader ]
  %909 = getelementptr inbounds nuw [272 x i8], ptr %871, i64 %908
  store i32 0, ptr %909, align 16, !noalias !33159
  %910 = getelementptr inbounds nuw i8, ptr %909, i64 4
  store i8 0, ptr %910, align 4, !noalias !33159
  %911 = getelementptr inbounds nuw i8, ptr %909, i64 16
  store i64 2, ptr %911, align 16, !noalias !33159
  %912 = add nuw i64 %908, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.loopexit312, label %.preheader311.epil, !llvm.loop !33176

.loopexit312:                                     ; preds = %.loopexit312.loopexit.unr-lcssa, %.preheader311.epil, %869
  store i64 %870, ptr %28, align 8, !alias.scope !33177, !noalias !33134
  %913 = getelementptr inbounds nuw i8, ptr %28, i64 8
  store ptr %871, ptr %913, align 8, !alias.scope !33177, !noalias !33134
  %914 = getelementptr inbounds nuw i8, ptr %28, i64 16
  store i64 %858, ptr %914, align 8, !alias.scope !33177, !noalias !33134
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !33134
  store i64 0, ptr %27, align 8, !noalias !33134
  %915 = load i64, ptr %857, align 8, !noalias !33134, !noundef !1708
  %916 = icmp ult i64 %915, 576460752303423488
  call void @llvm.assume(i1 %916)
  %917 = call i64 @llvm.umin.i64(i64 %849, i64 %915)
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !33134
  store ptr %27, ptr %26, align 8, !noalias !33134
  %918 = getelementptr inbounds nuw i8, ptr %26, i64 8
  store ptr %30, ptr %918, align 8, !noalias !33134
  %919 = getelementptr inbounds nuw i8, ptr %26, i64 16
  store ptr %137, ptr %919, align 8, !noalias !33134
  %920 = getelementptr inbounds nuw i8, ptr %26, i64 24
  store ptr %136, ptr %920, align 8, !noalias !33134
  %921 = getelementptr inbounds nuw i8, ptr %26, i64 32
  store ptr %33, ptr %921, align 8, !noalias !33134
  %922 = getelementptr inbounds nuw i8, ptr %26, i64 40
  store ptr %28, ptr %922, align 8, !noalias !33134
; invoke <rayon::range::Iter<usize> as rayon::iter::ParallelIterator>::drive_unindexed::<rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#1}>>
  invoke fastcc void @<rayon::range::Iter<usize> as rayon::iter::ParallelIterator>::drive_unindexed::<rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#1}>>(i64 noundef range(i64 0, 576460752303423488) %917, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %26)
          to label %932 unwind label %1096

923:                                              ; preds = %1096, %962, %960, %930
  %924 = phi { ptr, i32 } [ %1097, %1096 ], [ %961, %960 ], [ %931, %930 ], [ %1070, %962 ]
  %925 = load i64, ptr %30, align 8, !noalias !33134
  %926 = icmp eq i64 %925, 0
  br i1 %926, label %707, label %927

927:                                              ; preds = %923
  %928 = load ptr, ptr %856, align 8, !noalias !33134, !nonnull !1708, !noundef !1708
  %929 = shl nuw i64 %925, 4
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %928, i64 noundef %929, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33158, !inline_history !33148
  br label %707

930:                                              ; preds = %866
  %931 = landingpad { ptr, i32 }
          cleanup
  br label %923

932:                                              ; preds = %.loopexit312
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !33134
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !33134
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !33134
  %933 = load ptr, ptr %913, align 8, !noalias !33134, !nonnull !1708, !noundef !1708
  %934 = load i64, ptr %28, align 8, !range !1817, !noalias !33134, !noundef !1708
  %935 = load i64, ptr %914, align 8, !noalias !33134, !noundef !1708
  %936 = icmp ult i64 %935, 33909456017848441
  call void @llvm.assume(i1 %936)
  %937 = mul nuw i64 %935, 272
  %938 = getelementptr inbounds nuw i8, ptr %933, i64 %937
  %939 = getelementptr inbounds nuw i8, ptr %24, i64 8
  %940 = getelementptr inbounds nuw i8, ptr %24, i64 16
  %941 = getelementptr inbounds nuw i8, ptr %24, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !33178)
  call void @llvm.experimental.noalias.scope.decl(metadata !33181)
  call void @llvm.experimental.noalias.scope.decl(metadata !33183)
  call void @llvm.experimental.noalias.scope.decl(metadata !33186)
  %942 = mul i64 %934, 272
  %943 = icmp eq i64 %935, 0
  br i1 %943, label %.loopexit310, label %.preheader309.preheader

.preheader309.preheader:                          ; preds = %932
  %944 = add i64 %937, -272
  %945 = udiv i64 %944, 272
  %946 = add nuw nsw i64 %945, 1
  %xtraiter2537 = and i64 %946, 7
  %lcmp.mod2538.not = icmp eq i64 %xtraiter2537, 0
  br i1 %lcmp.mod2538.not, label %.preheader309.prol.loopexit, label %.preheader309.prol

.preheader309.prol:                               ; preds = %.preheader309.preheader, %957
  %947 = phi ptr [ %952, %957 ], [ %933, %.preheader309.preheader ]
  %948 = phi ptr [ %958, %957 ], [ %933, %.preheader309.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %957 ], [ 0, %.preheader309.preheader ]
  %949 = getelementptr inbounds nuw i8, ptr %947, i64 16
  %950 = load i64, ptr %949, align 16, !noalias !33188
  %951 = getelementptr inbounds nuw i8, ptr %947, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %951, i64 248, i1 false), !noalias !33188
  %952 = getelementptr inbounds nuw i8, ptr %947, i64 272
  %953 = icmp eq i64 %950, 2
  br i1 %953, label %957, label %954

954:                                              ; preds = %.preheader309.prol
  store i64 %950, ptr %948, align 16, !noalias !33195
  %955 = getelementptr inbounds nuw i8, ptr %948, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %955, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %956 = getelementptr inbounds nuw i8, ptr %948, i64 256
  br label %957

957:                                              ; preds = %954, %.preheader309.prol
  %958 = phi ptr [ %956, %954 ], [ %948, %.preheader309.prol ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %prol.iter.next = add i64 %prol.iter, 1
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter2537
  br i1 %prol.iter.cmp.not, label %.preheader309.prol.loopexit, label %.preheader309.prol, !llvm.loop !33198

.preheader309.prol.loopexit:                      ; preds = %957, %.preheader309.preheader
  %.lcssa2495.unr = phi ptr [ poison, %.preheader309.preheader ], [ %958, %957 ]
  %.unr2539 = phi ptr [ %933, %.preheader309.preheader ], [ %952, %957 ]
  %.unr2540 = phi ptr [ %933, %.preheader309.preheader ], [ %958, %957 ]
  %959 = icmp ult i64 %944, 1904
  br i1 %959, label %.loopexit310, label %.preheader309

960:                                              ; preds = %1088, %1083
  %961 = landingpad { ptr, i32 }
          cleanup
  br label %923

962:                                              ; preds = %.loopexit306
; invoke core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#2}>>
  invoke fastcc void @core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#2}>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %24) #89
          to label %923 unwind label %1081, !noalias !33199, !inline_history !33148

.preheader309:                                    ; preds = %.preheader309.prol.loopexit, %1029
  %963 = phi ptr [ %1024, %1029 ], [ %.unr2539, %.preheader309.prol.loopexit ]
  %964 = phi ptr [ %1030, %1029 ], [ %.unr2540, %.preheader309.prol.loopexit ]
  %965 = getelementptr inbounds nuw i8, ptr %963, i64 16
  %966 = load i64, ptr %965, align 16, !noalias !33188
  %967 = getelementptr inbounds nuw i8, ptr %963, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %967, i64 248, i1 false), !noalias !33188
  %968 = icmp eq i64 %966, 2
  br i1 %968, label %.preheader309.1, label %969

969:                                              ; preds = %.preheader309
  store i64 %966, ptr %964, align 16, !noalias !33195
  %970 = getelementptr inbounds nuw i8, ptr %964, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %970, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %971 = getelementptr inbounds nuw i8, ptr %964, i64 256
  br label %.preheader309.1

.preheader309.1:                                  ; preds = %969, %.preheader309
  %972 = phi ptr [ %971, %969 ], [ %964, %.preheader309 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %973 = getelementptr inbounds nuw i8, ptr %963, i64 288
  %974 = load i64, ptr %973, align 16, !noalias !33188
  %975 = getelementptr inbounds nuw i8, ptr %963, i64 296
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %975, i64 248, i1 false), !noalias !33188
  %976 = icmp eq i64 %974, 2
  br i1 %976, label %.preheader309.2, label %977

977:                                              ; preds = %.preheader309.1
  store i64 %974, ptr %972, align 16, !noalias !33195
  %978 = getelementptr inbounds nuw i8, ptr %972, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %978, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %979 = getelementptr inbounds nuw i8, ptr %972, i64 256
  br label %.preheader309.2

.preheader309.2:                                  ; preds = %977, %.preheader309.1
  %980 = phi ptr [ %979, %977 ], [ %972, %.preheader309.1 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %981 = getelementptr inbounds nuw i8, ptr %963, i64 560
  %982 = load i64, ptr %981, align 16, !noalias !33188
  %983 = getelementptr inbounds nuw i8, ptr %963, i64 568
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %983, i64 248, i1 false), !noalias !33188
  %984 = icmp eq i64 %982, 2
  br i1 %984, label %.preheader309.3, label %985

985:                                              ; preds = %.preheader309.2
  store i64 %982, ptr %980, align 16, !noalias !33195
  %986 = getelementptr inbounds nuw i8, ptr %980, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %986, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %987 = getelementptr inbounds nuw i8, ptr %980, i64 256
  br label %.preheader309.3

.preheader309.3:                                  ; preds = %985, %.preheader309.2
  %988 = phi ptr [ %987, %985 ], [ %980, %.preheader309.2 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %989 = getelementptr inbounds nuw i8, ptr %963, i64 832
  %990 = load i64, ptr %989, align 16, !noalias !33188
  %991 = getelementptr inbounds nuw i8, ptr %963, i64 840
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %991, i64 248, i1 false), !noalias !33188
  %992 = icmp eq i64 %990, 2
  br i1 %992, label %.preheader309.4, label %993

993:                                              ; preds = %.preheader309.3
  store i64 %990, ptr %988, align 16, !noalias !33195
  %994 = getelementptr inbounds nuw i8, ptr %988, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %994, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %995 = getelementptr inbounds nuw i8, ptr %988, i64 256
  br label %.preheader309.4

.preheader309.4:                                  ; preds = %993, %.preheader309.3
  %996 = phi ptr [ %995, %993 ], [ %988, %.preheader309.3 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %997 = getelementptr inbounds nuw i8, ptr %963, i64 1104
  %998 = load i64, ptr %997, align 16, !noalias !33188
  %999 = getelementptr inbounds nuw i8, ptr %963, i64 1112
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %999, i64 248, i1 false), !noalias !33188
  %1000 = icmp eq i64 %998, 2
  br i1 %1000, label %.preheader309.5, label %1001

1001:                                             ; preds = %.preheader309.4
  store i64 %998, ptr %996, align 16, !noalias !33195
  %1002 = getelementptr inbounds nuw i8, ptr %996, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %1002, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %1003 = getelementptr inbounds nuw i8, ptr %996, i64 256
  br label %.preheader309.5

.preheader309.5:                                  ; preds = %1001, %.preheader309.4
  %1004 = phi ptr [ %1003, %1001 ], [ %996, %.preheader309.4 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %1005 = getelementptr inbounds nuw i8, ptr %963, i64 1376
  %1006 = load i64, ptr %1005, align 16, !noalias !33188
  %1007 = getelementptr inbounds nuw i8, ptr %963, i64 1384
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %1007, i64 248, i1 false), !noalias !33188
  %1008 = icmp eq i64 %1006, 2
  br i1 %1008, label %.preheader309.6, label %1009

1009:                                             ; preds = %.preheader309.5
  store i64 %1006, ptr %1004, align 16, !noalias !33195
  %1010 = getelementptr inbounds nuw i8, ptr %1004, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %1010, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %1011 = getelementptr inbounds nuw i8, ptr %1004, i64 256
  br label %.preheader309.6

.preheader309.6:                                  ; preds = %1009, %.preheader309.5
  %1012 = phi ptr [ %1011, %1009 ], [ %1004, %.preheader309.5 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %1013 = getelementptr inbounds nuw i8, ptr %963, i64 1648
  %1014 = load i64, ptr %1013, align 16, !noalias !33188
  %1015 = getelementptr inbounds nuw i8, ptr %963, i64 1656
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %1015, i64 248, i1 false), !noalias !33188
  %1016 = icmp eq i64 %1014, 2
  br i1 %1016, label %.preheader309.7, label %1017

1017:                                             ; preds = %.preheader309.6
  store i64 %1014, ptr %1012, align 16, !noalias !33195
  %1018 = getelementptr inbounds nuw i8, ptr %1012, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %1018, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %1019 = getelementptr inbounds nuw i8, ptr %1012, i64 256
  br label %.preheader309.7

.preheader309.7:                                  ; preds = %1017, %.preheader309.6
  %1020 = phi ptr [ %1019, %1017 ], [ %1012, %.preheader309.6 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %1021 = getelementptr inbounds nuw i8, ptr %963, i64 1920
  %1022 = load i64, ptr %1021, align 16, !noalias !33188
  %1023 = getelementptr inbounds nuw i8, ptr %963, i64 1928
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %19, ptr noundef nonnull align 8 dereferenceable(248) %1023, i64 248, i1 false), !noalias !33188
  %1024 = getelementptr inbounds nuw i8, ptr %963, i64 2176
  %1025 = icmp eq i64 %1022, 2
  br i1 %1025, label %1029, label %1026

1026:                                             ; preds = %.preheader309.7
  store i64 %1022, ptr %1020, align 16, !noalias !33195
  %1027 = getelementptr inbounds nuw i8, ptr %1020, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(248) %1027, ptr noundef nonnull align 8 dereferenceable(248) %19, i64 248, i1 false), !noalias !33188
  %1028 = getelementptr inbounds nuw i8, ptr %1020, i64 256
  br label %1029

1029:                                             ; preds = %1026, %.preheader309.7
  %1030 = phi ptr [ %1028, %1026 ], [ %1020, %.preheader309.7 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  %1031 = icmp eq ptr %1024, %938
  br i1 %1031, label %.loopexit310, label %.preheader309

.loopexit310:                                     ; preds = %.preheader309.prol.loopexit, %1029, %932
  %1032 = phi ptr [ %933, %932 ], [ %938, %1029 ], [ %938, %.preheader309.prol.loopexit ]
  %1033 = phi ptr [ %933, %932 ], [ %.lcssa2495.unr, %.preheader309.prol.loopexit ], [ %1030, %1029 ]
  %1034 = ptrtoint ptr %1033 to i64
  %1035 = ptrtoint ptr %933 to i64
  %1036 = sub nuw i64 %1034, %1035
  %1037 = lshr exact i64 %1036, 8
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !33200
  store ptr %933, ptr %20, align 8, !noalias !33200
  %1038 = getelementptr inbounds nuw i8, ptr %20, i64 8
  store i64 %1037, ptr %1038, align 8, !noalias !33200
  %1039 = getelementptr inbounds nuw i8, ptr %20, i64 16
  store i64 %934, ptr %1039, align 8, !noalias !33200
  call void @llvm.experimental.noalias.scope.decl(metadata !33201)
  %1040 = ptrtoint ptr %938 to i64
  %1041 = ptrtoint ptr %1032 to i64
  %1042 = sub nuw i64 %1040, %1041
  %1043 = udiv exact i64 %1042, 272
  store i64 0, ptr %940, align 8, !alias.scope !33204, !noalias !33205
  store ptr inttoptr (i64 16 to ptr), ptr %24, align 8, !alias.scope !33204, !noalias !33205
  store ptr inttoptr (i64 16 to ptr), ptr %939, align 8, !alias.scope !33204, !noalias !33205
  store ptr inttoptr (i64 16 to ptr), ptr %941, align 8, !alias.scope !33204, !noalias !33205
  call void @llvm.experimental.noalias.scope.decl(metadata !33206)
  %1044 = icmp eq ptr %938, %1032
  br i1 %1044, label %.loopexit308, label %.preheader307

.preheader307:                                    ; preds = %.loopexit310, %1052
  %1045 = phi i64 [ %1047, %1052 ], [ 0, %.loopexit310 ]
  %1046 = getelementptr inbounds nuw [272 x i8], ptr %1032, i64 %1045
  %1047 = add nuw nsw i64 %1045, 1
  %1048 = getelementptr inbounds nuw i8, ptr %1046, i64 16
  %1049 = load i64, ptr %1048, align 16, !range !12503, !alias.scope !33209, !noalias !33216, !noundef !1708
  %1050 = icmp eq i64 %1049, 2
  br i1 %1050, label %1052, label %1051

1051:                                             ; preds = %.preheader307
; invoke core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef nonnull readonly align 16 dereferenceable(256) %1048)
          to label %1052 unwind label %1054, !noalias !33216, !inline_history !33148

1052:                                             ; preds = %1051, %.preheader307
  %1053 = icmp eq i64 %1047, %1043
  br i1 %1053, label %.loopexit308, label %.preheader307

1054:                                             ; preds = %1051
  %1055 = landingpad { ptr, i32 }
          cleanup
  %1056 = icmp eq i64 %1047, %1043
  br i1 %1056, label %.loopexit306, label %.preheader305

.preheader305:                                    ; preds = %1054, %1064
  %1057 = phi i64 [ %1059, %1064 ], [ %1047, %1054 ]
  %1058 = getelementptr inbounds nuw [272 x i8], ptr %1032, i64 %1057
  %1059 = add i64 %1057, 1
  %1060 = getelementptr inbounds nuw i8, ptr %1058, i64 16
  %1061 = load i64, ptr %1060, align 16, !range !12503, !alias.scope !33217, !noalias !33216, !noundef !1708
  %1062 = icmp eq i64 %1061, 2
  br i1 %1062, label %1064, label %1063

1063:                                             ; preds = %.preheader305
; invoke core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef nonnull readonly align 16 dereferenceable(256) %1060)
          to label %1064 unwind label %1066, !noalias !33216, !inline_history !33148

1064:                                             ; preds = %1063, %.preheader305
  %1065 = icmp eq i64 %1059, %1043
  br i1 %1065, label %.loopexit306, label %.preheader305

1066:                                             ; preds = %1063
  %1067 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33224, !inline_history !33148
  unreachable

1068:                                             ; preds = %1079
  %1069 = landingpad { ptr, i32 }
          cleanup
  br label %.loopexit306

.loopexit306:                                     ; preds = %1064, %1068, %1054
  %1070 = phi { ptr, i32 } [ %1069, %1068 ], [ %1055, %1054 ], [ %1055, %1064 ]
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %20) #89
          to label %962 unwind label %1081, !noalias !33225, !inline_history !33148

.loopexit308:                                     ; preds = %1052, %.loopexit310
  %1071 = and i64 %942, 240
  %1072 = icmp eq i64 %1071, 0
  br i1 %1072, label %1083, label %1073

1073:                                             ; preds = %.loopexit308
  %1074 = and i64 %942, -256
  %1075 = icmp eq i64 %1074, 0
  br i1 %1075, label %1076, label %1077

1076:                                             ; preds = %1073
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %933, i64 noundef %942, i64 noundef 16) #88, !noalias !33225, !inline_history !33148
  br label %1083

1077:                                             ; preds = %1073
; call <purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc
  %_0.i = call noalias noundef align 16 ptr @<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @qualification_454_native_cost::GLOBAL (.llvm.11174181910260379007), ptr noundef nonnull %933, i64 noundef 16, i64 noundef %942, i64 noundef %1074) #88, !noalias !33225
  %1078 = icmp eq ptr %_0.i, null
  br i1 %1078, label %1079, label %1083, !prof !4226

1079:                                             ; preds = %1077
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 16, i64 noundef %1074) #90
          to label %1080 unwind label %1068, !noalias !33225, !inline_history !33148

1080:                                             ; preds = %1079
  unreachable

1081:                                             ; preds = %.loopexit306, %962
  %1082 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33225, !inline_history !33148
  unreachable

1083:                                             ; preds = %1077, %1076, %.loopexit308
  %1084 = phi ptr [ %933, %.loopexit308 ], [ %_0.i, %1077 ], [ inttoptr (i64 16 to ptr), %1076 ]
  %1085 = lshr i64 %942, 8
  store i64 %1085, ptr %25, align 8, !alias.scope !33226, !noalias !33227
  %1086 = getelementptr inbounds nuw i8, ptr %25, i64 8
  store ptr %1084, ptr %1086, align 8, !alias.scope !33226, !noalias !33227
  %1087 = getelementptr inbounds nuw i8, ptr %25, i64 16
  store i64 %1037, ptr %1087, align 8, !alias.scope !33226, !noalias !33227
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !33200
; invoke core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#2}>>
  invoke fastcc void @core::ptr::drop_glue::<core::iter::adapters::filter_map::FilterMap<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>, purrdf_sparql_eval::parallel::par_blocks_try_map_init<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#2}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#3}, purrdf_sparql_eval::expr::eval_extend<purrdf_core::ir::dataset::RdfDataset>::{closure#4}>::{closure#2}>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %24)
          to label %1088 unwind label %960, !noalias !33158, !inline_history !33148

1088:                                             ; preds = %1083
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !33134
; invoke purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>
  invoke fastcc void @purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(256) %138, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %25)
          to label %1089 unwind label %960, !inline_history !33148

1089:                                             ; preds = %1088
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !33134
  call void @llvm.lifetime.end.p0(ptr nonnull %27), !noalias !33134
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !33134
  %1090 = load i64, ptr %30, align 8, !noalias !33134
  %1091 = icmp eq i64 %1090, 0
  br i1 %1091, label %1095, label %1092

1092:                                             ; preds = %1089
  %1093 = load ptr, ptr %856, align 8, !noalias !33134, !nonnull !1708, !noundef !1708
  %1094 = shl nuw i64 %1090, 4
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1093, i64 noundef %1094, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33158, !inline_history !33148
  br label %1095

1095:                                             ; preds = %1092, %1089
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !33134
  br label %1100

1096:                                             ; preds = %.loopexit312
  %1097 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint)), purrdf_sparql_eval::error::EvalError>>>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %28) #89
          to label %923 unwind label %1098, !noalias !33158, !inline_history !33148

1098:                                             ; preds = %1096
  %1099 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33158, !inline_history !33148
  unreachable

1100:                                             ; preds = %1095, %834
  call void @llvm.lifetime.end.p0(ptr nonnull %33)
  br label %1101

1101:                                             ; preds = %1100, %825
  call void @llvm.lifetime.end.p0(ptr nonnull %136)
  call void @llvm.lifetime.end.p0(ptr nonnull %137)
  %1102 = load i64, ptr %138, align 16, !range !2062, !noundef !1708
  %1103 = icmp eq i64 %1102, -1
  br i1 %1103, label %1104, label %1110

1104:                                             ; preds = %1101
  %1105 = getelementptr inbounds nuw i8, ptr %138, i64 16
  %1106 = getelementptr inbounds nuw i8, ptr %138, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %139, ptr noundef nonnull align 16 dereferenceable(64) %1106, i64 64, i1 false)
  %1107 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %1108 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %1109 = load <4 x i64>, ptr %1105, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %138)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %1107, ptr noundef nonnull align 16 dereferenceable(64) %139, i64 64, i1 false)
  store <4 x i64> %1109, ptr %1108, align 16
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %139)
  br label %3307

1110:                                             ; preds = %1101
  %1111 = getelementptr inbounds nuw i8, ptr %138, i64 8
  %1112 = getelementptr inbounds nuw i8, ptr %138, i64 24
  %1113 = load i64, ptr %1112, align 8
  %1114 = getelementptr inbounds nuw i8, ptr %138, i64 32
  %1115 = load i64, ptr %1114, align 16
  %1116 = getelementptr inbounds nuw i8, ptr %138, i64 40
  %1117 = load i64, ptr %1116, align 8
  %1118 = getelementptr inbounds nuw i8, ptr %138, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(208) %139, ptr noundef nonnull align 16 dereferenceable(208) %1118, i64 208, i1 false)
  %1119 = getelementptr inbounds nuw i8, ptr %132, i64 24
  %1120 = getelementptr inbounds nuw i8, ptr %140, i64 8
  %1121 = load <2 x i64>, ptr %1111, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %138)
  call void @llvm.lifetime.start.p0(ptr nonnull %132)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %1119, ptr noundef nonnull align 16 dereferenceable(208) %139, i64 208, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %140)
  store i64 %1102, ptr %140, align 8
  store <2 x i64> %1121, ptr %1120, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %139)
  call void @llvm.lifetime.start.p0(ptr nonnull %133)
  %1122 = add i64 %1113, -3
  %1123 = icmp ult i64 %1122, -2
  %1124 = select i1 %1123, i64 %1117, i64 %1113
  %1125 = add i64 %1124, -1
  %1126 = select i1 %1123, i64 %1113, i64 1
  %1127 = select i1 %1123, i64 1, i64 %1117
  store i64 %1126, ptr %132, align 8
  %1128 = getelementptr inbounds nuw i8, ptr %132, i64 8
  store i64 %1115, ptr %1128, align 8
  %1129 = getelementptr inbounds nuw i8, ptr %132, i64 16
  store i64 %1127, ptr %1129, align 8
  %1130 = getelementptr inbounds nuw i8, ptr %132, i64 232
  store i64 0, ptr %1130, align 8
  %1131 = getelementptr inbounds nuw i8, ptr %132, i64 240
  store i64 %1125, ptr %1131, align 8
; invoke <purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(240) %133, ptr noalias nofree noundef align 8 captures(address) dereferenceable(248) %132)
          to label %1134 unwind label %3318

1132:                                             ; preds = %2866
  %1133 = landingpad { ptr, i32 }
          cleanup
  br label %707

1134:                                             ; preds = %1110
  call void @llvm.lifetime.end.p0(ptr nonnull %132)
  call void @llvm.lifetime.start.p0(ptr nonnull %135)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %135, ptr noundef nonnull align 8 dereferenceable(32) %133, i64 32, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %134)
  %1135 = getelementptr inbounds nuw i8, ptr %133, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %134, ptr noundef nonnull align 8 dereferenceable(208) %1135, i64 208, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %133)
  %1136 = load ptr, ptr %696, align 8, !noundef !1708
  %1137 = icmp eq ptr %1136, null
  br i1 %1137, label %2625, label %1138

1138:                                             ; preds = %1134
  call void @llvm.lifetime.start.p0(ptr nonnull %126)
  call void @llvm.lifetime.start.p0(ptr nonnull %125)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %125, ptr noundef nonnull align 8 dereferenceable(24) %140, i64 24, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %124)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %124, ptr noundef nonnull align 8 dereferenceable(208) %134, i64 208, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !33228)
  call void @llvm.experimental.noalias.scope.decl(metadata !33231)
  call void @llvm.experimental.noalias.scope.decl(metadata !33233)
  call void @llvm.experimental.noalias.scope.decl(metadata !33235)
  %1139 = getelementptr inbounds nuw i8, ptr %149, i64 194
  %1140 = load i8, ptr %1139, align 2, !range !3634, !alias.scope !33228, !noalias !33237, !noundef !1708
  %1141 = icmp eq i8 %1140, 2
  br i1 %1141, label %1265, label %1142

1142:                                             ; preds = %1138
  call void @llvm.lifetime.start.p0(ptr nonnull %102)
  call void @llvm.lifetime.start.p0(ptr nonnull %101), !noalias !33239
  store i64 0, ptr %101, align 8, !noalias !33239
  %1143 = getelementptr inbounds nuw i8, ptr %101, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1143, align 8, !noalias !33239
  %1144 = getelementptr inbounds nuw i8, ptr %101, i64 16
  store i64 0, ptr %1144, align 8, !noalias !33239
  call void @llvm.experimental.noalias.scope.decl(metadata !33240)
  call void @llvm.experimental.noalias.scope.decl(metadata !33243)
  call void @llvm.lifetime.start.p0(ptr nonnull %95), !noalias !33245
  call void @llvm.lifetime.start.p0(ptr nonnull %94), !noalias !33245
  %1145 = getelementptr inbounds nuw i8, ptr %125, i64 16
  %1146 = load i64, ptr %1145, align 8, !alias.scope !33248, !noalias !33249, !noundef !1708
  %1147 = icmp ult i64 %1146, 230584300921369396
  call void @llvm.assume(i1 %1147)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %94, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %101, i64 noundef %1146)
          to label %1148 unwind label %1263, !noalias !33250

1148:                                             ; preds = %1142
  %1149 = load i64, ptr %94, align 16, !range !2530, !noalias !33245, !noundef !1708
  %1150 = icmp eq i64 %1149, -1
  %1151 = getelementptr inbounds nuw i8, ptr %94, i64 8
  %1152 = load i64, ptr %1151, align 8, !noalias !33245
  %1153 = getelementptr inbounds nuw i8, ptr %94, i64 16
  %1154 = load ptr, ptr %1153, align 16, !noalias !33245
  %1155 = getelementptr inbounds nuw i8, ptr %94, i64 24
  %1156 = load i64, ptr %1155, align 8, !noalias !33245
  br i1 %1150, label %1165, label %1157

1157:                                             ; preds = %1148
  %1158 = getelementptr inbounds nuw i8, ptr %94, i64 32
  %1159 = load i64, ptr %1158, align 16, !noalias !33251
  %1160 = getelementptr inbounds nuw i8, ptr %94, i64 40
  %1161 = load i64, ptr %1160, align 8, !noalias !33251
  %1162 = getelementptr inbounds nuw i8, ptr %94, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %102, ptr noundef nonnull align 16 dereferenceable(48) %1162, i64 48, i1 false), !noalias !33251
  call void @llvm.lifetime.end.p0(ptr nonnull %94), !noalias !33245
  call void @llvm.lifetime.end.p0(ptr nonnull %95), !noalias !33245
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %140)
          to label %2601 unwind label %1163

1163:                                             ; preds = %1157
  %1164 = landingpad { ptr, i32 }
          cleanup
  br label %2621

1165:                                             ; preds = %1148
  call void @llvm.lifetime.end.p0(ptr nonnull %94), !noalias !33245
  store i64 %1152, ptr %95, align 8, !noalias !33245
  %1166 = getelementptr inbounds nuw i8, ptr %95, i64 8
  store ptr %1154, ptr %1166, align 8, !noalias !33245
  %1167 = getelementptr inbounds nuw i8, ptr %95, i64 16
  store i64 %1156, ptr %1167, align 8, !noalias !33245
  %1168 = getelementptr inbounds nuw i8, ptr %125, i64 8
  %1169 = load ptr, ptr %1168, align 8, !alias.scope !33248, !noalias !33249, !nonnull !1708, !noundef !1708
  %1170 = load i64, ptr %125, align 8, !range !1817, !alias.scope !33248, !noalias !33249, !noundef !1708
  %1171 = mul nuw nsw i64 %1146, 40
  %1172 = getelementptr inbounds nuw i8, ptr %1169, i64 %1171
  call void @llvm.lifetime.start.p0(ptr nonnull %93), !noalias !33245
  store ptr %1169, ptr %93, align 8, !noalias !33245
  %1173 = getelementptr inbounds nuw i8, ptr %93, i64 8
  %1174 = getelementptr inbounds nuw i8, ptr %93, i64 16
  store i64 %1170, ptr %1174, align 8, !noalias !33245
  %1175 = getelementptr inbounds nuw i8, ptr %93, i64 24
  store ptr %1172, ptr %1175, align 8, !noalias !33245
  %1176 = icmp eq i64 %1146, 0
  br i1 %1176, label %.loopexit304, label %1177

1177:                                             ; preds = %1165
  %1178 = getelementptr inbounds nuw i8, ptr %91, i64 8
  %1179 = getelementptr inbounds nuw i8, ptr %92, i64 8
  %1180 = getelementptr inbounds nuw i8, ptr %5, i64 664
  %1181 = getelementptr inbounds nuw i8, ptr %91, i64 16
  %1182 = getelementptr inbounds nuw i8, ptr %92, i64 16
  %1183 = getelementptr inbounds nuw i8, ptr %92, i64 24
  %1184 = getelementptr inbounds nuw i8, ptr %92, i64 32
  %1185 = getelementptr inbounds nuw i8, ptr %92, i64 40
  br label %1190

1186:                                             ; preds = %1197
  %1187 = landingpad { ptr, i32 }
          cleanup
  store ptr %1194, ptr %1173, align 8, !noalias !33245
  br label %1188

1188:                                             ; preds = %1232, %1229, %1186
  %1189 = phi { ptr, i32 } [ %1187, %1186 ], [ %1230, %1232 ], [ %1230, %1229 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %93) #89
          to label %1202 unwind label %1261, !noalias !33252

1190:                                             ; preds = %1235, %1177
  %1191 = phi ptr [ %1154, %1177 ], [ %1236, %1235 ]
  %1192 = phi i64 [ %1156, %1177 ], [ %1241, %1235 ]
  %1193 = phi ptr [ %1169, %1177 ], [ %1194, %1235 ]
  %1194 = getelementptr inbounds nuw i8, ptr %1193, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %92), !noalias !33245
  call void @llvm.experimental.noalias.scope.decl(metadata !33253)
  call void @llvm.lifetime.start.p0(ptr nonnull %91), !noalias !33245
  store ptr %5, ptr %91, align 8, !noalias !33256
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1178, ptr noundef nonnull align 8 dereferenceable(40) %1193, i64 40, i1 false), !noalias !33252
  call void @llvm.experimental.noalias.scope.decl(metadata !33259)
  call void @llvm.experimental.noalias.scope.decl(metadata !33262)
  %1195 = load i64, ptr %1178, align 8, !alias.scope !33262, !noalias !33264, !noundef !1708
  %1196 = icmp eq i64 %1195, 0
  br i1 %1196, label %1197, label %1199

1197:                                             ; preds = %1190
  %1198 = load ptr, ptr %1180, align 8, !alias.scope !33266, !noalias !33267, !nonnull !1708, !align !1818, !noundef !1708
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %92, ptr noalias nofree noundef align 8 dereferenceable(184) %687, ptr noundef nonnull align 8 %1198, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %1181)
          to label %1209 unwind label %1186, !noalias !33252

1199:                                             ; preds = %1190
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1179, ptr noundef nonnull align 8 dereferenceable(40) %1193, i64 40, i1 false), !noalias !33252
  call void @llvm.lifetime.end.p0(ptr nonnull %91), !noalias !33245
  br label %1219

.loopexit304:                                     ; preds = %1235, %1165
  %1200 = phi i64 [ %1156, %1165 ], [ %1241, %1235 ]
  %1201 = phi ptr [ %1169, %1165 ], [ %1172, %1235 ]
  store ptr %1201, ptr %1173, align 8, !noalias !33245
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %93)
          to label %1206 unwind label %1204, !noalias !33252

1202:                                             ; preds = %1204, %1188
  %1203 = phi { ptr, i32 } [ %1205, %1204 ], [ %1189, %1188 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %95) #89, !noalias !33252
  br label %2621

1204:                                             ; preds = %1212, %.loopexit304
  %1205 = landingpad { ptr, i32 }
          cleanup
  br label %1202

1206:                                             ; preds = %.loopexit304
  call void @llvm.lifetime.end.p0(ptr nonnull %93), !noalias !33245
  %1207 = load i64, ptr %95, align 8, !noalias !33251
  %1208 = load ptr, ptr %1166, align 8, !noalias !33251
  call void @llvm.lifetime.end.p0(ptr nonnull %95), !noalias !33245
  call void @llvm.lifetime.end.p0(ptr nonnull %101), !noalias !33239
  br label %2611

1209:                                             ; preds = %1197
  %1210 = load i64, ptr %92, align 16, !noalias !33245
  call void @llvm.lifetime.end.p0(ptr nonnull %91), !noalias !33245
  %1211 = icmp eq i64 %1210, -1
  br i1 %1211, label %1219, label %1212

1212:                                             ; preds = %1209
  store ptr %1194, ptr %1173, align 8, !noalias !33245
  %1213 = load i64, ptr %1179, align 8, !noalias !33245
  %1214 = load ptr, ptr %1182, align 16, !noalias !33245
  %1215 = load i64, ptr %1183, align 8, !noalias !33245
  %1216 = load i64, ptr %1184, align 16, !noalias !33245
  %1217 = load i64, ptr %1185, align 8, !noalias !33245
  %1218 = getelementptr inbounds nuw i8, ptr %92, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %102, ptr noundef nonnull align 16 dereferenceable(48) %1218, i64 48, i1 false), !noalias !33251
  call void @llvm.lifetime.end.p0(ptr nonnull %92), !noalias !33245
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %93)
          to label %1243 unwind label %1204, !noalias !33252

1219:                                             ; preds = %1209, %1199
  %1220 = load i64, ptr %1179, align 8, !noalias !33245
  %1221 = load ptr, ptr %1182, align 16, !noalias !33245
  %1222 = load <2 x i64>, ptr %1183, align 8, !noalias !33245
  %1223 = load i64, ptr %1185, align 8, !noalias !33245
  call void @llvm.lifetime.end.p0(ptr nonnull %92), !noalias !33245
  call void @llvm.experimental.noalias.scope.decl(metadata !33268)
  %1224 = load i64, ptr %95, align 8, !range !1817, !alias.scope !33268, !noalias !33271, !noundef !1708
  %1225 = icmp eq i64 %1192, %1224
  br i1 %1225, label %1226, label %1235

1226:                                             ; preds = %1219
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %95)
          to label %1227 unwind label %1229, !noalias !33273

1227:                                             ; preds = %1226
  %1228 = load ptr, ptr %1166, align 8, !alias.scope !33268, !noalias !33271
  br label %1235

1229:                                             ; preds = %1226
  %1230 = landingpad { ptr, i32 }
          cleanup
  store ptr %1194, ptr %1173, align 8, !noalias !33245
  %1231 = icmp ugt i64 %1220, 5
  br i1 %1231, label %1232, label %1188

1232:                                             ; preds = %1229
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1221) ]
  %1233 = shl i64 %1220, 3
  %1234 = add i64 %1233, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1221, i64 noundef %1234, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33274
  br label %1188

1235:                                             ; preds = %1227, %1219
  %1236 = phi ptr [ %1228, %1227 ], [ %1191, %1219 ]
  %1237 = getelementptr inbounds nuw [40 x i8], ptr %1236, i64 %1192
  store i64 %1220, ptr %1237, align 8, !noalias !33277
  %1238 = getelementptr inbounds nuw i8, ptr %1237, i64 8
  store ptr %1221, ptr %1238, align 8, !noalias !33277
  %1239 = getelementptr inbounds nuw i8, ptr %1237, i64 16
  store <2 x i64> %1222, ptr %1239, align 8, !noalias !33252
  %1240 = getelementptr inbounds nuw i8, ptr %1237, i64 32
  store i64 %1223, ptr %1240, align 8, !noalias !33252
  %1241 = add i64 %1192, 1
  store i64 %1241, ptr %1167, align 8, !alias.scope !33268, !noalias !33271
  %1242 = icmp eq ptr %1194, %1172
  br i1 %1242, label %.loopexit304, label %1190

1243:                                             ; preds = %1212
  call void @llvm.lifetime.end.p0(ptr nonnull %93), !noalias !33245
  call void @llvm.experimental.noalias.scope.decl(metadata !33278)
  call void @llvm.experimental.noalias.scope.decl(metadata !33281)
  %1244 = icmp eq i64 %1192, 0
  br i1 %1244, label %.loopexit303, label %.preheader302

.preheader302:                                    ; preds = %1243, %1255
  %1245 = phi i64 [ %1247, %1255 ], [ 0, %1243 ]
  %1246 = getelementptr inbounds nuw [40 x i8], ptr %1191, i64 %1245
  %1247 = add nuw nsw i64 %1245, 1
  %1248 = load i64, ptr %1246, align 8, !range !1940, !alias.scope !33284, !noalias !33287, !noundef !1708
  %1249 = icmp ugt i64 %1248, 5
  br i1 %1249, label %1250, label %1255

1250:                                             ; preds = %.preheader302
  %1251 = getelementptr i8, ptr %1246, i64 8
  %1252 = load ptr, ptr %1251, align 8, !alias.scope !33281, !noalias !33287, !nonnull !1708, !noundef !1708
  %1253 = shl i64 %1248, 3
  %1254 = add i64 %1253, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1252, i64 noundef %1254, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33288
  br label %1255

1255:                                             ; preds = %1250, %.preheader302
  %1256 = icmp eq i64 %1247, %1192
  br i1 %1256, label %.loopexit303, label %.preheader302

.loopexit303:                                     ; preds = %1255, %1243
  %1257 = load i64, ptr %95, align 8, !alias.scope !33278, !noalias !33245
  %1258 = icmp eq i64 %1257, 0
  br i1 %1258, label %2600, label %1259

1259:                                             ; preds = %.loopexit303
  %1260 = mul nuw i64 %1257, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1191, i64 noundef %1260, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33287
  br label %2600

1261:                                             ; preds = %1263, %1188
  %1262 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33252
  unreachable

1263:                                             ; preds = %1142
  %1264 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %140) #89
          to label %2621 unwind label %1261

1265:                                             ; preds = %1138
  call void @llvm.lifetime.start.p0(ptr nonnull %100), !noalias !33239
  call void @llvm.lifetime.start.p0(ptr nonnull %99), !noalias !33239
  %1266 = load i64, ptr %124, align 8, !alias.scope !33235, !noalias !33291
  %1267 = getelementptr inbounds nuw i8, ptr %124, i64 8
  %1268 = load i64, ptr %1267, align 8, !alias.scope !33235, !noalias !33291
  %1269 = getelementptr inbounds nuw i8, ptr %124, i64 16
  %1270 = load i64, ptr %1269, align 8, !alias.scope !33235, !noalias !33291
  %1271 = getelementptr inbounds nuw i8, ptr %99, i64 24
  %1272 = getelementptr inbounds nuw i8, ptr %134, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %1271, ptr noundef nonnull align 8 dereferenceable(184) %1272, i64 184, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !33292)
  %1273 = icmp ugt i64 %1266, 2
  %1274 = select i1 %1273, i64 %1270, i64 %1266
  %1275 = add i64 %1274, -1
  %1276 = select i1 %1273, i64 %1266, i64 1
  %1277 = select i1 %1273, i64 1, i64 %1270
  store i64 %1276, ptr %99, align 8, !alias.scope !33295, !noalias !33239
  %1278 = getelementptr inbounds nuw i8, ptr %99, i64 8
  store i64 %1268, ptr %1278, align 8, !alias.scope !33295, !noalias !33239
  %1279 = getelementptr inbounds nuw i8, ptr %99, i64 16
  store i64 %1277, ptr %1279, align 8, !alias.scope !33295, !noalias !33239
  %1280 = getelementptr inbounds nuw i8, ptr %99, i64 208
  store i64 0, ptr %1280, align 8, !alias.scope !33297, !noalias !33298
  %1281 = getelementptr inbounds nuw i8, ptr %99, i64 216
  store i64 %1275, ptr %1281, align 8, !alias.scope !33297, !noalias !33298
; invoke <purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %100, ptr noalias nofree noundef align 8 captures(address) dereferenceable(224) %99)
          to label %1282 unwind label %2619, !noalias !33299

1282:                                             ; preds = %1265
  call void @llvm.lifetime.end.p0(ptr nonnull %99), !noalias !33239
  %1283 = getelementptr inbounds nuw i8, ptr %100, i64 8
  %1284 = load ptr, ptr %1283, align 8, !noalias !33239, !nonnull !1708, !noundef !1708
  %1285 = getelementptr inbounds nuw i8, ptr %100, i64 16
  %1286 = load i64, ptr %1285, align 8, !noalias !33239, !noundef !1708
  %1287 = mul nuw nsw i64 %1286, 200
  %1288 = getelementptr inbounds nuw i8, ptr %1284, i64 %1287
  %1289 = icmp eq i64 %1286, 0
  br i1 %1289, label %1358, label %.preheader301.preheader

.preheader301.preheader:                          ; preds = %1282
  %xtraiter2541 = and i64 %1286, 3
  %1290 = icmp ult i64 %1286, 4
  br i1 %1290, label %.preheader301.epil.preheader, label %.preheader301.preheader.new

.preheader301.preheader.new:                      ; preds = %.preheader301.preheader
  %unroll_iter2550 = and i64 %1286, -4
  br label %.preheader301

.preheader301:                                    ; preds = %1335, %.preheader301.preheader.new
  %1291 = phi i64 [ 0, %.preheader301.preheader.new ], [ %1338, %1335 ]
  %1292 = phi i64 [ 0, %.preheader301.preheader.new ], [ %1337, %1335 ]
  %niter2551 = phi i64 [ 0, %.preheader301.preheader.new ], [ %niter2551.next.3, %1335 ]
  %1293 = getelementptr inbounds nuw [200 x i8], ptr %1284, i64 %1291
  %1294 = getelementptr i8, ptr %1293, i64 168
  %1295 = load i64, ptr %1294, align 8, !noalias !33299, !noundef !1708
  %1296 = getelementptr i8, ptr %1293, i64 176
  %1297 = load i64, ptr %1296, align 8, !noalias !33299, !noundef !1708
  %1298 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1297, i64 %1295)
  %1299 = extractvalue { i64, i1 } %1298, 0
  %1300 = extractvalue { i64, i1 } %1298, 1
  br i1 %1300, label %1301, label %.preheader301.1, !prof !1803

1301:                                             ; preds = %.preheader301
  br label %.preheader301.1

.preheader301.1:                                  ; preds = %1301, %.preheader301
  %1302 = phi i64 [ -1, %1301 ], [ %1299, %.preheader301 ]
  %1303 = call noundef i64 @llvm.uadd.sat.i64(i64 %1292, i64 %1302)
  %1304 = getelementptr inbounds nuw [200 x i8], ptr %1284, i64 %1291
  %1305 = getelementptr i8, ptr %1304, i64 368
  %1306 = load i64, ptr %1305, align 8, !noalias !33299, !noundef !1708
  %1307 = getelementptr i8, ptr %1304, i64 376
  %1308 = load i64, ptr %1307, align 8, !noalias !33299, !noundef !1708
  %1309 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1308, i64 %1306)
  %1310 = extractvalue { i64, i1 } %1309, 0
  %1311 = extractvalue { i64, i1 } %1309, 1
  br i1 %1311, label %1312, label %.preheader301.2, !prof !1803

1312:                                             ; preds = %.preheader301.1
  br label %.preheader301.2

.preheader301.2:                                  ; preds = %1312, %.preheader301.1
  %1313 = phi i64 [ -1, %1312 ], [ %1310, %.preheader301.1 ]
  %1314 = call noundef i64 @llvm.uadd.sat.i64(i64 %1303, i64 %1313)
  %1315 = getelementptr inbounds nuw [200 x i8], ptr %1284, i64 %1291
  %1316 = getelementptr i8, ptr %1315, i64 568
  %1317 = load i64, ptr %1316, align 8, !noalias !33299, !noundef !1708
  %1318 = getelementptr i8, ptr %1315, i64 576
  %1319 = load i64, ptr %1318, align 8, !noalias !33299, !noundef !1708
  %1320 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1319, i64 %1317)
  %1321 = extractvalue { i64, i1 } %1320, 0
  %1322 = extractvalue { i64, i1 } %1320, 1
  br i1 %1322, label %1323, label %.preheader301.3, !prof !1803

1323:                                             ; preds = %.preheader301.2
  br label %.preheader301.3

.preheader301.3:                                  ; preds = %1323, %.preheader301.2
  %1324 = phi i64 [ -1, %1323 ], [ %1321, %.preheader301.2 ]
  %1325 = call noundef i64 @llvm.uadd.sat.i64(i64 %1314, i64 %1324)
  %1326 = getelementptr inbounds nuw [200 x i8], ptr %1284, i64 %1291
  %1327 = getelementptr i8, ptr %1326, i64 768
  %1328 = load i64, ptr %1327, align 8, !noalias !33299, !noundef !1708
  %1329 = getelementptr i8, ptr %1326, i64 776
  %1330 = load i64, ptr %1329, align 8, !noalias !33299, !noundef !1708
  %1331 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1330, i64 %1328)
  %1332 = extractvalue { i64, i1 } %1331, 0
  %1333 = extractvalue { i64, i1 } %1331, 1
  br i1 %1333, label %1334, label %1335, !prof !1803

1334:                                             ; preds = %.preheader301.3
  br label %1335

1335:                                             ; preds = %1334, %.preheader301.3
  %1336 = phi i64 [ -1, %1334 ], [ %1332, %.preheader301.3 ]
  %1337 = call noundef i64 @llvm.uadd.sat.i64(i64 %1325, i64 %1336)
  %1338 = add nuw i64 %1291, 4
  %niter2551.next.3 = add i64 %niter2551, 4
  %niter2551.ncmp.3 = icmp eq i64 %niter2551.next.3, %unroll_iter2550
  br i1 %niter2551.ncmp.3, label %.unr-lcssa, label %.preheader301

.unr-lcssa:                                       ; preds = %1335
  %lcmp.mod2547.not = icmp eq i64 %xtraiter2541, 0
  br i1 %lcmp.mod2547.not, label %.epilog-lcssa, label %.preheader301.epil.preheader

.preheader301.epil.preheader:                     ; preds = %.unr-lcssa, %.preheader301.preheader
  %.epil.init2544 = phi i64 [ 0, %.preheader301.preheader ], [ %1338, %.unr-lcssa ]
  %.epil.init2546 = phi i64 [ 0, %.preheader301.preheader ], [ %1337, %.unr-lcssa ]
  %lcmp.mod2549 = icmp ne i64 %xtraiter2541, 0
  call void @llvm.assume(i1 %lcmp.mod2549)
  br label %.preheader301.epil

.preheader301.epil:                               ; preds = %1350, %.preheader301.epil.preheader
  %1339 = phi i64 [ %1353, %1350 ], [ %.epil.init2544, %.preheader301.epil.preheader ]
  %1340 = phi i64 [ %1352, %1350 ], [ %.epil.init2546, %.preheader301.epil.preheader ]
  %epil.iter2542 = phi i64 [ %epil.iter2542.next, %1350 ], [ 0, %.preheader301.epil.preheader ]
  %1341 = getelementptr inbounds nuw [200 x i8], ptr %1284, i64 %1339
  %1342 = getelementptr i8, ptr %1341, i64 168
  %1343 = load i64, ptr %1342, align 8, !noalias !33299, !noundef !1708
  %1344 = getelementptr i8, ptr %1341, i64 176
  %1345 = load i64, ptr %1344, align 8, !noalias !33299, !noundef !1708
  %1346 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %1345, i64 %1343)
  %1347 = extractvalue { i64, i1 } %1346, 0
  %1348 = extractvalue { i64, i1 } %1346, 1
  br i1 %1348, label %1349, label %1350, !prof !1803

1349:                                             ; preds = %.preheader301.epil
  br label %1350

1350:                                             ; preds = %1349, %.preheader301.epil
  %1351 = phi i64 [ -1, %1349 ], [ %1347, %.preheader301.epil ]
  %1352 = call noundef i64 @llvm.uadd.sat.i64(i64 %1340, i64 %1351)
  %1353 = add nuw i64 %1339, 1
  %epil.iter2542.next = add i64 %epil.iter2542, 1
  %epil.iter2542.cmp.not = icmp eq i64 %epil.iter2542.next, %xtraiter2541
  br i1 %epil.iter2542.cmp.not, label %.epilog-lcssa, label %.preheader301.epil, !llvm.loop !33300

.epilog-lcssa:                                    ; preds = %1350, %.unr-lcssa
  %.lcssa2474 = phi i64 [ %1337, %.unr-lcssa ], [ %1352, %1350 ]
  %1354 = load ptr, ptr %696, align 8, !alias.scope !33231, !noalias !33299, !noundef !1708
  %1355 = icmp eq ptr %1354, null
  %1356 = icmp eq i64 %.lcssa2474, 0
  %1357 = or i1 %1356, %1355
  br i1 %1357, label %1358, label %1410

1358:                                             ; preds = %1414, %1410, %.epilog-lcssa, %1282
  %1359 = load i64, ptr %100, align 8, !range !1817, !noalias !33239, !noundef !1708
  %1360 = icmp ult i64 %1286, 46116860184273880
  call void @llvm.assume(i1 %1360)
  call void @llvm.lifetime.start.p0(ptr nonnull %90), !noalias !33301
  store i64 0, ptr %90, align 8, !noalias !33301
  %1361 = getelementptr inbounds nuw i8, ptr %90, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1361, align 8, !noalias !33301
  %1362 = getelementptr inbounds nuw i8, ptr %90, i64 16
  store i64 0, ptr %1362, align 8, !noalias !33301
  call void @llvm.lifetime.start.p0(ptr nonnull %89), !noalias !33301
  store ptr %1284, ptr %89, align 8, !noalias !33305
  %1363 = getelementptr inbounds nuw i8, ptr %89, i64 8
  %1364 = getelementptr inbounds nuw i8, ptr %89, i64 16
  store i64 %1359, ptr %1364, align 8, !noalias !33305
  %1365 = getelementptr inbounds nuw i8, ptr %89, i64 24
  store ptr %1288, ptr %1365, align 8, !noalias !33305
  br i1 %1289, label %.loopexit297, label %1366

1366:                                             ; preds = %1358
  %1367 = getelementptr inbounds nuw i8, ptr %88, i64 8
  %1368 = getelementptr inbounds nuw i8, ptr %88, i64 152
  br label %1373

1369:                                             ; preds = %1380, %1371
  %1370 = phi { ptr, i32 } [ %1372, %1371 ], [ %1390, %1380 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %90) #89
          to label %2623 unwind label %1408, !noalias !33306

1371:                                             ; preds = %1396
  %1372 = landingpad { ptr, i32 }
          cleanup
  br label %1369

1373:                                             ; preds = %1405, %1366
  %1374 = phi ptr [ inttoptr (i64 8 to ptr), %1366 ], [ %1401, %1405 ]
  %1375 = phi i64 [ 0, %1366 ], [ %1403, %1405 ]
  %1376 = phi ptr [ %1284, %1366 ], [ %1377, %1405 ]
  %1377 = getelementptr inbounds nuw i8, ptr %1376, i64 200
  %1378 = load i64, ptr %1376, align 8, !noalias !33307
  %1379 = icmp eq i64 %1378, -1
  br i1 %1379, label %.loopexit297, label %1381

1380:                                             ; preds = %1389
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %89)
          to label %1369 unwind label %1408, !noalias !33306

1381:                                             ; preds = %1373
  %1382 = getelementptr inbounds nuw i8, ptr %1376, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %88), !noalias !33301
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %1367, ptr noundef nonnull align 8 dereferenceable(152) %1382, i64 152, i1 false), !noalias !33306
  store i64 %1378, ptr %88, align 8, !noalias !33301
  %1383 = load i8, ptr %1368, align 8, !range !1746, !noalias !33301, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33313)
  %1384 = load i64, ptr %90, align 8, !range !1817, !alias.scope !33313, !noalias !33316, !noundef !1708
  %1385 = icmp eq i64 %1375, %1384
  br i1 %1385, label %1386, label %1400

1386:                                             ; preds = %1381
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %90)
          to label %1387 unwind label %1389, !noalias !33318

1387:                                             ; preds = %1386
  %1388 = load ptr, ptr %1361, align 8, !alias.scope !33313, !noalias !33316
  br label %1400

1389:                                             ; preds = %1386
  %1390 = landingpad { ptr, i32 }
          cleanup
  store ptr %1377, ptr %1363, align 8, !noalias !33301
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(160) %88) #89
          to label %1380 unwind label %1391, !noalias !33319

1391:                                             ; preds = %1389
  %1392 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33320
  unreachable

.loopexit297:                                     ; preds = %1405, %1373, %1358
  %1393 = phi i64 [ 0, %1358 ], [ %1403, %1405 ], [ %1375, %1373 ]
  %1394 = phi ptr [ inttoptr (i64 8 to ptr), %1358 ], [ %1401, %1405 ], [ %1374, %1373 ]
  %1395 = phi ptr [ %1284, %1358 ], [ %1288, %1405 ], [ %1377, %1373 ]
  store ptr %1395, ptr %1363, align 8, !noalias !33301
  br label %1396

1396:                                             ; preds = %1407, %.loopexit297
  %1397 = phi i64 [ %1403, %1407 ], [ %1393, %.loopexit297 ]
  %1398 = phi ptr [ %1401, %1407 ], [ %1394, %.loopexit297 ]
  %1399 = phi i8 [ 1, %1407 ], [ 0, %.loopexit297 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %89)
          to label %1417 unwind label %1371, !noalias !33306

1400:                                             ; preds = %1387, %1381
  %1401 = phi ptr [ %1388, %1387 ], [ %1374, %1381 ]
  %1402 = getelementptr inbounds nuw [160 x i8], ptr %1401, i64 %1375
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(160) %1402, ptr noundef nonnull readonly align 8 dereferenceable(160) %88, i64 160, i1 false), !noalias !33319
  %1403 = add nuw nsw i64 %1375, 1
  store i64 %1403, ptr %1362, align 8, !alias.scope !33313, !noalias !33316
  %1404 = trunc nuw i8 %1383 to i1
  br i1 %1404, label %1407, label %1405

1405:                                             ; preds = %1400
  call void @llvm.lifetime.end.p0(ptr nonnull %88), !noalias !33301
  %1406 = icmp eq ptr %1377, %1288
  br i1 %1406, label %.loopexit297, label %1373

1407:                                             ; preds = %1400
  store ptr %1377, ptr %1363, align 8, !noalias !33301
  call void @llvm.lifetime.end.p0(ptr nonnull %88), !noalias !33301
  br label %1396

1408:                                             ; preds = %1380, %1369
  %1409 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33306
  unreachable

1410:                                             ; preds = %.epilog-lcssa
  %1411 = getelementptr inbounds nuw i8, ptr %1354, i64 336
  %1412 = load ptr, ptr %1411, align 8, !noalias !33299, !noundef !1708
  %1413 = icmp eq ptr %1412, null
  br i1 %1413, label %1358, label %1414

1414:                                             ; preds = %1410
  %1415 = getelementptr inbounds nuw i8, ptr %1354, i64 352
  %1416 = atomicrmw add ptr %1415, i64 %.lcssa2474 monotonic, align 8, !noalias !33299
  br label %1358

1417:                                             ; preds = %1396
  call void @llvm.lifetime.end.p0(ptr nonnull %89), !noalias !33301
  %1418 = load i64, ptr %90, align 8, !noalias !33321
  call void @llvm.lifetime.end.p0(ptr nonnull %90), !noalias !33301
  %1419 = getelementptr inbounds nuw i8, ptr %149, i64 193
  %1420 = load i8, ptr %1419, align 1, !range !1746, !alias.scope !33228, !noalias !33237, !noundef !1708
  %1421 = trunc nuw i8 %1420 to i1
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1398) ]
  %1422 = icmp ne i64 %1397, 0
  br i1 %1422, label %iter.check, label %.loopexit296

iter.check:                                       ; preds = %1417
  %min.iters.check = icmp ult i64 %1397, 8
  br i1 %min.iters.check, label %.preheader295.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check2061 = icmp ult i64 %1397, 32
  br i1 %min.iters.check2061, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %1397, 24
  %n.vec = and i64 %1397, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1423, %vector.body ]
  %vec.phi2062 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1424, %vector.body ]
  %vec.phi2063 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1425, %vector.body ]
  %vec.phi2064 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1426, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %vec.ind
  %wide.gep2065 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %step.add
  %wide.gep2066 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %step.add.2
  %wide.gep2067 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %step.add.3
  %wide.gep2068 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep2069 = getelementptr i8, <8 x ptr> %wide.gep2065, i64 64
  %wide.gep2070 = getelementptr i8, <8 x ptr> %wide.gep2066, i64 64
  %wide.gep2071 = getelementptr i8, <8 x ptr> %wide.gep2067, i64 64
  %wide.masked.gather = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2068, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33322
  %wide.masked.gather2072 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2069, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33322
  %wide.masked.gather2073 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2070, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33322
  %wide.masked.gather2074 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2071, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33322
  %1423 = add <8 x i64> %wide.masked.gather, %vec.phi
  %1424 = add <8 x i64> %wide.masked.gather2072, %vec.phi2062
  %1425 = add <8 x i64> %wide.masked.gather2073, %vec.phi2063
  %1426 = add <8 x i64> %wide.masked.gather2074, %vec.phi2064
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %1427 = icmp eq i64 %index.next, %n.vec
  br i1 %1427, label %middle.block, label %vector.body, !llvm.loop !33325

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %1424, %1423
  %bin.rdx2075 = add <8 x i64> %1425, %bin.rdx
  %bin.rdx2076 = add <8 x i64> %1426, %bin.rdx2075
  %1428 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx2076)
  %cmp.n = icmp eq i64 %1397, %n.vec
  br i1 %cmp.n, label %.loopexit296, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader295.preheader, label %vec.epilog.ph, !prof !29315

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %1428, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec2078 = and i64 %1397, -8
  %1429 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index2079 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next2085, %vec.epilog.vector.body ]
  %vec.ind2080 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next2086, %vec.epilog.vector.body ]
  %vec.phi2081 = phi <8 x i64> [ %1429, %vec.epilog.ph ], [ %1430, %vec.epilog.vector.body ]
  %wide.gep2082 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %vec.ind2080
  %wide.gep2083 = getelementptr i8, <8 x ptr> %wide.gep2082, i64 64
  %wide.masked.gather2084 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2083, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33322
  %1430 = add <8 x i64> %wide.masked.gather2084, %vec.phi2081
  %index.next2085 = add nuw i64 %index2079, 8
  %vec.ind.next2086 = add nuw <8 x i64> %vec.ind2080, splat (i64 8)
  %1431 = icmp eq i64 %index.next2085, %n.vec2078
  br i1 %1431, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !33326

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %1432 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %1430)
  %cmp.n2087 = icmp eq i64 %1397, %n.vec2078
  br i1 %cmp.n2087, label %.loopexit296, label %.preheader295.preheader

.preheader295.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph2458 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec2078, %vec.epilog.middle.block ]
  %.ph2459 = phi i64 [ 0, %iter.check ], [ %1428, %vec.epilog.iter.check ], [ %1432, %vec.epilog.middle.block ]
  br label %.preheader295

.preheader295:                                    ; preds = %.preheader295.preheader, %.preheader295
  %1433 = phi i64 [ %1440, %.preheader295 ], [ %.ph2458, %.preheader295.preheader ]
  %1434 = phi i64 [ %1439, %.preheader295 ], [ %.ph2459, %.preheader295.preheader ]
  %1435 = getelementptr inbounds nuw [160 x i8], ptr %1398, i64 %1433
  %1436 = getelementptr i8, ptr %1435, i64 64
  %1437 = load i64, ptr %1436, align 8, !noalias !33322, !noundef !1708
  %1438 = icmp ult i64 %1437, 288230376151711744
  call void @llvm.assume(i1 %1438)
  %1439 = add i64 %1437, %1434
  %1440 = add nuw i64 %1433, 1
  %1441 = icmp eq i64 %1440, %1397
  br i1 %1441, label %.loopexit296, label %.preheader295, !llvm.loop !33327

.loopexit296:                                     ; preds = %.preheader295, %middle.block, %vec.epilog.middle.block, %1417
  %1442 = phi i64 [ 0, %1417 ], [ %1432, %vec.epilog.middle.block ], [ %1428, %middle.block ], [ %1439, %.preheader295 ]
  %1443 = trunc nuw i8 %1399 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %98)
  %1444 = getelementptr inbounds nuw i8, ptr %149, i64 195
  %1445 = load i8, ptr %1444, align 1, !range !10412, !alias.scope !33228, !noalias !33237, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %97), !noalias !33239
  store i64 %1418, ptr %97, align 8, !noalias !33239
  %1446 = getelementptr inbounds nuw i8, ptr %97, i64 8
  store ptr %1398, ptr %1446, align 8, !noalias !33239
  %1447 = getelementptr inbounds nuw i8, ptr %97, i64 16
  store i64 %1397, ptr %1447, align 8, !noalias !33239
  %1448 = getelementptr inbounds nuw i8, ptr %97, i64 24
  store i8 %1399, ptr %1448, align 8, !noalias !33239
  call void @llvm.experimental.noalias.scope.decl(metadata !33328)
  call void @llvm.experimental.noalias.scope.decl(metadata !33331)
  call void @llvm.lifetime.start.p0(ptr nonnull %87), !noalias !33333
  call void @llvm.lifetime.start.p0(ptr nonnull %86), !noalias !33333
  call void @llvm.lifetime.start.p0(ptr nonnull %85), !noalias !33333
  store i64 0, ptr %85, align 8, !noalias !33333
  %1449 = getelementptr inbounds nuw i8, ptr %85, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1449, align 8, !noalias !33333
  %1450 = getelementptr inbounds nuw i8, ptr %85, i64 16
  store i64 0, ptr %1450, align 8, !noalias !33333
  %1451 = getelementptr inbounds nuw i8, ptr %125, i64 16
  %1452 = load i64, ptr %1451, align 8, !alias.scope !33336, !noalias !33337, !noundef !1708
  %1453 = icmp ult i64 %1452, 230584300921369396
  call void @llvm.assume(i1 %1453)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %86, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %85, i64 noundef %1452)
          to label %1454 unwind label %2538, !noalias !33338

1454:                                             ; preds = %.loopexit296
  call void @llvm.lifetime.end.p0(ptr nonnull %85), !noalias !33333
  %1455 = load i64, ptr %86, align 16, !range !2530, !noalias !33333, !noundef !1708
  %1456 = icmp eq i64 %1455, -1
  %1457 = getelementptr inbounds nuw i8, ptr %86, i64 8
  %1458 = load i64, ptr %1457, align 8, !noalias !33333
  %1459 = getelementptr inbounds nuw i8, ptr %86, i64 16
  %1460 = load ptr, ptr %1459, align 16, !noalias !33333
  %1461 = getelementptr inbounds nuw i8, ptr %86, i64 24
  %1462 = load i64, ptr %1461, align 8, !noalias !33333
  br i1 %1456, label %1471, label %1463

1463:                                             ; preds = %1454
  %1464 = getelementptr inbounds nuw i8, ptr %86, i64 32
  %1465 = load i8, ptr %1464, align 16, !noalias !33339
  %1466 = getelementptr inbounds nuw i8, ptr %86, i64 33
  %1467 = load i56, ptr %1466, align 1, !noalias !33339
  %1468 = getelementptr inbounds nuw i8, ptr %86, i64 40
  %1469 = load i64, ptr %1468, align 8, !noalias !33339
  %1470 = getelementptr inbounds nuw i8, ptr %86, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(48) %98, ptr noundef nonnull align 16 dereferenceable(48) %1470, i64 48, i1 false), !noalias !33339
  call void @llvm.lifetime.end.p0(ptr nonnull %86), !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %87), !noalias !33333
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %140)
          to label %2508 unwind label %2506

1471:                                             ; preds = %1454
  call void @llvm.lifetime.end.p0(ptr nonnull %86), !noalias !33333
  store i64 %1458, ptr %87, align 8, !noalias !33333
  %1472 = getelementptr inbounds nuw i8, ptr %87, i64 8
  store ptr %1460, ptr %1472, align 8, !noalias !33333
  %1473 = getelementptr inbounds nuw i8, ptr %87, i64 16
  store i64 %1462, ptr %1473, align 8, !noalias !33333
  call void @llvm.lifetime.start.p0(ptr nonnull %84), !noalias !33333
  %1474 = getelementptr inbounds nuw i8, ptr %125, i64 8
  %1475 = load ptr, ptr %1474, align 8, !alias.scope !33336, !noalias !33337, !nonnull !1708, !noundef !1708
  %1476 = load i64, ptr %125, align 8, !range !1817, !alias.scope !33336, !noalias !33337, !noundef !1708
  %1477 = getelementptr inbounds nuw [40 x i8], ptr %1475, i64 %1452
  store ptr %1475, ptr %84, align 8, !noalias !33333
  %1478 = getelementptr inbounds nuw i8, ptr %84, i64 16
  store i64 %1476, ptr %1478, align 8, !noalias !33333
  %1479 = getelementptr inbounds nuw i8, ptr %84, i64 8
  store ptr %1475, ptr %1479, align 8, !noalias !33333
  %1480 = getelementptr inbounds nuw i8, ptr %84, i64 24
  store ptr %1477, ptr %1480, align 8, !noalias !33333
  %1481 = load ptr, ptr %696, align 8, !alias.scope !33340, !noalias !33341, !noundef !1708
  %1482 = icmp eq ptr %1481, null
  br i1 %1482, label %1486, label %1483

1483:                                             ; preds = %1471
  %1484 = atomicrmw add ptr %1481, i64 1 monotonic, align 8, !noalias !33341
  %1485 = icmp slt i64 %1484, 0
  br i1 %1485, label %1607, label %1588

1486:                                             ; preds = %1471
  call void @llvm.lifetime.start.p0(ptr nonnull %83), !noalias !33333
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %83, ptr noundef nonnull align 8 dereferenceable(32) %84, i64 32, i1 false), !noalias !33333
  %1487 = getelementptr inbounds nuw i8, ptr %83, i64 24
  %1488 = load ptr, ptr %1487, align 8, !alias.scope !33342, !noalias !33345, !nonnull !1708, !noundef !1708
  %1489 = getelementptr inbounds nuw i8, ptr %83, i64 8
  %1490 = load ptr, ptr %1489, align 8, !alias.scope !33342, !noalias !33345
  %1491 = icmp eq ptr %1490, %1488
  br i1 %1491, label %.loopexit270, label %1492

1492:                                             ; preds = %1486
  %1493 = getelementptr inbounds nuw i8, ptr %81, i64 8
  %1494 = getelementptr inbounds nuw i8, ptr %82, i64 8
  %1495 = getelementptr inbounds nuw i8, ptr %5, i64 664
  %1496 = getelementptr inbounds nuw i8, ptr %81, i64 16
  %1497 = getelementptr inbounds nuw i8, ptr %82, i64 16
  %1498 = getelementptr inbounds nuw i8, ptr %82, i64 24
  %1499 = getelementptr inbounds nuw i8, ptr %82, i64 32
  %1500 = getelementptr inbounds nuw i8, ptr %82, i64 33
  %1501 = getelementptr inbounds nuw i8, ptr %82, i64 40
  br label %1506

1502:                                             ; preds = %1513
  %1503 = landingpad { ptr, i32 }
          cleanup
  store ptr %1510, ptr %1489, align 8, !noalias !33333
  br label %1504

1504:                                             ; preds = %1562, %1559, %1502
  %1505 = phi { ptr, i32 } [ %1503, %1502 ], [ %1560, %1562 ], [ %1560, %1559 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %83) #89
          to label %2499 unwind label %1586, !noalias !33341

1506:                                             ; preds = %1565, %1492
  %1507 = phi ptr [ %1460, %1492 ], [ %1566, %1565 ]
  %1508 = phi i64 [ %1462, %1492 ], [ %1573, %1565 ]
  %1509 = phi ptr [ %1490, %1492 ], [ %1510, %1565 ]
  %1510 = getelementptr inbounds nuw i8, ptr %1509, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %81), !noalias !33333
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1493, ptr noundef nonnull align 8 dereferenceable(40) %1509, i64 40, i1 false), !noalias !33341
  call void @llvm.lifetime.start.p0(ptr nonnull %82), !noalias !33333
  store ptr %5, ptr %81, align 8, !noalias !33333
  call void @llvm.experimental.noalias.scope.decl(metadata !33347)
  call void @llvm.experimental.noalias.scope.decl(metadata !33350)
  %1511 = load i64, ptr %1493, align 8, !alias.scope !33350, !noalias !33352, !noundef !1708
  %1512 = icmp eq i64 %1511, 0
  br i1 %1512, label %1513, label %1515

1513:                                             ; preds = %1506
  %1514 = load ptr, ptr %1495, align 8, !alias.scope !33354, !noalias !33355, !nonnull !1708, !align !1818, !noundef !1708
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %82, ptr noalias nofree noundef align 8 dereferenceable(184) %687, ptr noundef nonnull align 8 %1514, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %1496)
          to label %1536 unwind label %1502, !noalias !33341

1515:                                             ; preds = %1506
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1494, ptr noundef nonnull align 8 dereferenceable(40) %1509, i64 40, i1 false), !noalias !33341
  call void @llvm.lifetime.end.p0(ptr nonnull %81), !noalias !33333
  br label %1547

.loopexit270:                                     ; preds = %1565, %1486
  %1516 = phi i64 [ %1462, %1486 ], [ %1573, %1565 ]
  %1517 = phi ptr [ %1490, %1486 ], [ %1510, %1565 ]
  store ptr %1517, ptr %1489, align 8, !noalias !33333
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %83)
          to label %1522 unwind label %1518, !noalias !33341

1518:                                             ; preds = %2434, %1744, %1539, %.loopexit270
  %1519 = phi i8 [ %2406, %2434 ], [ 0, %1744 ], [ 1, %1539 ], [ 1, %.loopexit270 ]
  %1520 = phi i8 [ 0, %2434 ], [ 0, %1744 ], [ 1, %1539 ], [ 1, %.loopexit270 ]
  %1521 = landingpad { ptr, i32 }
          cleanup
  br i1 %1482, label %1525, label %2495

1522:                                             ; preds = %.loopexit270
  call void @llvm.lifetime.end.p0(ptr nonnull %83), !noalias !33333
  %1523 = load i64, ptr %87, align 8, !noalias !33339
  %1524 = load ptr, ptr %1472, align 8, !noalias !33339
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %87), !noalias !33333
  br label %2508

1525:                                             ; preds = %2495, %1530, %1518
  %1526 = phi i8 [ %1531, %1530 ], [ %2498, %2495 ], [ %1519, %1518 ]
  %1527 = phi i8 [ %1532, %1530 ], [ %2497, %2495 ], [ %1520, %1518 ]
  %1528 = phi { ptr, i32 } [ %1533, %1530 ], [ %2496, %2495 ], [ %1521, %1518 ]
  %1529 = trunc nuw i8 %1526 to i1
  br i1 %1529, label %2499, label %2502

1530:                                             ; preds = %2438, %1748
  %1531 = phi i8 [ %1584, %2438 ], [ 0, %1748 ]
  %1532 = phi i8 [ %1585, %2438 ], [ 0, %1748 ]
  %1533 = landingpad { ptr, i32 }
          cleanup
  br label %1525

1534:                                             ; preds = %2486, %.loopexit269, %2436
  call void @llvm.lifetime.end.p0(ptr nonnull %87), !noalias !33333
  %1535 = trunc nuw i8 %1585 to i1
  br i1 %1535, label %2508, label %2542

1536:                                             ; preds = %1513
  %1537 = load i64, ptr %82, align 16, !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %81), !noalias !33333
  %1538 = icmp eq i64 %1537, -1
  br i1 %1538, label %1547, label %1539

1539:                                             ; preds = %1536
  store ptr %1510, ptr %1489, align 8, !noalias !33333
  %1540 = load i64, ptr %1494, align 8, !noalias !33333
  %1541 = load ptr, ptr %1497, align 16, !noalias !33333
  %1542 = load i64, ptr %1498, align 8, !noalias !33333
  %1543 = load i8, ptr %1499, align 16, !noalias !33333
  %1544 = load i56, ptr %1500, align 1, !noalias !33333
  %1545 = load i64, ptr %1501, align 8, !noalias !33333
  %1546 = getelementptr inbounds nuw i8, ptr %82, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(48) %98, ptr noundef nonnull align 16 dereferenceable(48) %1546, i64 48, i1 false), !noalias !33339
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !33333
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %83)
          to label %1575 unwind label %1518, !noalias !33341

1547:                                             ; preds = %1536, %1515
  %1548 = load i64, ptr %1494, align 8, !noalias !33333
  %1549 = load ptr, ptr %1497, align 16, !noalias !33333
  %1550 = load i64, ptr %1498, align 8, !noalias !33333
  %1551 = load i8, ptr %1499, align 16, !noalias !33333
  %1552 = load i56, ptr %1500, align 1, !noalias !33333
  %1553 = load i64, ptr %1501, align 8, !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !33333
  call void @llvm.experimental.noalias.scope.decl(metadata !33356)
  %1554 = load i64, ptr %87, align 8, !range !1817, !alias.scope !33356, !noalias !33359, !noundef !1708
  %1555 = icmp eq i64 %1508, %1554
  br i1 %1555, label %1556, label %1565

1556:                                             ; preds = %1547
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %87)
          to label %1557 unwind label %1559, !noalias !33361

1557:                                             ; preds = %1556
  %1558 = load ptr, ptr %1472, align 8, !alias.scope !33356, !noalias !33359
  br label %1565

1559:                                             ; preds = %1556
  %1560 = landingpad { ptr, i32 }
          cleanup
  store ptr %1510, ptr %1489, align 8, !noalias !33333
  %1561 = icmp ugt i64 %1548, 5
  br i1 %1561, label %1562, label %1504

1562:                                             ; preds = %1559
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1549) ]
  %1563 = shl i64 %1548, 3
  %1564 = add i64 %1563, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1549, i64 noundef %1564, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33362
  br label %1504

1565:                                             ; preds = %1557, %1547
  %1566 = phi ptr [ %1558, %1557 ], [ %1507, %1547 ]
  %1567 = getelementptr inbounds nuw [40 x i8], ptr %1566, i64 %1508
  store i64 %1548, ptr %1567, align 8, !noalias !33365
  %1568 = getelementptr inbounds nuw i8, ptr %1567, i64 8
  store ptr %1549, ptr %1568, align 8, !noalias !33365
  %1569 = getelementptr inbounds nuw i8, ptr %1567, i64 16
  store i64 %1550, ptr %1569, align 8, !noalias !33341
  %1570 = getelementptr inbounds nuw i8, ptr %1567, i64 24
  store i8 %1551, ptr %1570, align 8, !noalias !33341
  %1571 = getelementptr inbounds nuw i8, ptr %1567, i64 25
  store i56 %1552, ptr %1571, align 1, !noalias !33341
  %1572 = getelementptr inbounds nuw i8, ptr %1567, i64 32
  store i64 %1553, ptr %1572, align 8, !noalias !33341
  %1573 = add i64 %1508, 1
  store i64 %1573, ptr %1473, align 8, !alias.scope !33356, !noalias !33359
  %1574 = icmp eq ptr %1510, %1488
  br i1 %1574, label %.loopexit270, label %1506

1575:                                             ; preds = %1539
  call void @llvm.lifetime.end.p0(ptr nonnull %83), !noalias !33333
  br label %1576

1576:                                             ; preds = %2435, %1575
  %1577 = phi i56 [ %1544, %1575 ], [ %2399, %2435 ]
  %1578 = phi i64 [ %1545, %1575 ], [ %2400, %2435 ]
  %1579 = phi i8 [ %1543, %1575 ], [ %2401, %2435 ]
  %1580 = phi i64 [ %1542, %1575 ], [ %2402, %2435 ]
  %1581 = phi ptr [ %1541, %1575 ], [ %2403, %2435 ]
  %1582 = phi i64 [ %1540, %1575 ], [ %2404, %2435 ]
  %1583 = phi i64 [ %1537, %1575 ], [ %2405, %2435 ]
  %1584 = phi i8 [ 1, %1575 ], [ %2406, %2435 ]
  %1585 = phi i8 [ 1, %1575 ], [ 0, %2435 ]
  br i1 %1482, label %2436, label %2438

1586:                                             ; preds = %2540, %2538, %2495, %1804, %1707, %1688, %1504
  %1587 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33341
  unreachable

1588:                                             ; preds = %1483
  %1589 = load ptr, ptr %696, align 8, !alias.scope !33340, !noalias !33341, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %80), !noalias !33333
  store ptr %1589, ptr %80, align 8, !noalias !33333
  %1590 = getelementptr inbounds nuw i8, ptr %1589, i64 16
  %1591 = load i64, ptr %1590, align 8, !noalias !33341
  %1592 = icmp eq i64 %1591, -1
  %1593 = getelementptr inbounds nuw i8, ptr %1589, i64 40
  %1594 = load i64, ptr %1593, align 8, !noalias !33341
  %1595 = icmp ne i64 %1594, -1
  %1596 = and i1 %1422, %1595
  br i1 %1596, label %iter.check2126, label %1608

iter.check2126:                                   ; preds = %1588
  %min.iters.check2089 = icmp ult i64 %1397, 8
  br i1 %min.iters.check2089, label %.preheader294.preheader, label %vector.main.loop.iter.check2090

vector.main.loop.iter.check2090:                  ; preds = %iter.check2126
  %min.iters.check2091 = icmp ult i64 %1397, 32
  br i1 %min.iters.check2091, label %vec.epilog.ph2130, label %vector.ph2092

vector.ph2092:                                    ; preds = %vector.main.loop.iter.check2090
  %n.mod.vf2093 = and i64 %1397, 24
  %n.vec2094 = and i64 %1397, -32
  br label %vector.body2095

vector.body2095:                                  ; preds = %vector.body2095, %vector.ph2092
  %index2096 = phi i64 [ 0, %vector.ph2092 ], [ %index.next2117, %vector.body2095 ]
  %vec.ind2097 = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph2092 ], [ %vec.ind.next2118, %vector.body2095 ]
  %vec.phi2098 = phi <8 x i64> [ zeroinitializer, %vector.ph2092 ], [ %1597, %vector.body2095 ]
  %vec.phi2099 = phi <8 x i64> [ zeroinitializer, %vector.ph2092 ], [ %1598, %vector.body2095 ]
  %vec.phi2100 = phi <8 x i64> [ zeroinitializer, %vector.ph2092 ], [ %1599, %vector.body2095 ]
  %vec.phi2101 = phi <8 x i64> [ zeroinitializer, %vector.ph2092 ], [ %1600, %vector.body2095 ]
  %step.add2102 = add nuw <8 x i64> %vec.ind2097, splat (i64 8)
  %step.add.22103 = add nuw <8 x i64> %vec.ind2097, splat (i64 16)
  %step.add.32104 = add nuw <8 x i64> %vec.ind2097, splat (i64 24)
  %wide.gep2105 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %vec.ind2097
  %wide.gep2106 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %step.add2102
  %wide.gep2107 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %step.add.22103
  %wide.gep2108 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %step.add.32104
  %wide.gep2109 = getelementptr i8, <8 x ptr> %wide.gep2105, i64 16
  %wide.gep2110 = getelementptr i8, <8 x ptr> %wide.gep2106, i64 16
  %wide.gep2111 = getelementptr i8, <8 x ptr> %wide.gep2107, i64 16
  %wide.gep2112 = getelementptr i8, <8 x ptr> %wide.gep2108, i64 16
  %wide.masked.gather2113 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2109, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33341
  %wide.masked.gather2114 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2110, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33341
  %wide.masked.gather2115 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2111, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33341
  %wide.masked.gather2116 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2112, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33341
  %1597 = add <8 x i64> %wide.masked.gather2113, %vec.phi2098
  %1598 = add <8 x i64> %wide.masked.gather2114, %vec.phi2099
  %1599 = add <8 x i64> %wide.masked.gather2115, %vec.phi2100
  %1600 = add <8 x i64> %wide.masked.gather2116, %vec.phi2101
  %index.next2117 = add nuw i64 %index2096, 32
  %vec.ind.next2118 = add nuw <8 x i64> %vec.ind2097, splat (i64 32)
  %1601 = icmp eq i64 %index.next2117, %n.vec2094
  br i1 %1601, label %middle.block2119, label %vector.body2095, !llvm.loop !33366

middle.block2119:                                 ; preds = %vector.body2095
  %bin.rdx2120 = add <8 x i64> %1598, %1597
  %bin.rdx2121 = add <8 x i64> %1599, %bin.rdx2120
  %bin.rdx2122 = add <8 x i64> %1600, %bin.rdx2121
  %1602 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx2122)
  %cmp.n2123 = icmp eq i64 %1397, %n.vec2094
  br i1 %cmp.n2123, label %.loopexit2149, label %vec.epilog.iter.check2128

vec.epilog.iter.check2128:                        ; preds = %middle.block2119
  %min.epilog.iters.check2129 = icmp eq i64 %n.mod.vf2093, 0
  br i1 %min.epilog.iters.check2129, label %.preheader294.preheader, label %vec.epilog.ph2130, !prof !29315

vec.epilog.ph2130:                                ; preds = %vector.main.loop.iter.check2090, %vec.epilog.iter.check2128
  %vec.epilog.resume.val2124 = phi i64 [ %n.vec2094, %vec.epilog.iter.check2128 ], [ 0, %vector.main.loop.iter.check2090 ]
  %bc.merge.rdx2125 = phi i64 [ %1602, %vec.epilog.iter.check2128 ], [ 0, %vector.main.loop.iter.check2090 ]
  %n.vec2132 = and i64 %1397, -8
  %1603 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx2125, i64 0
  %broadcast.splatinsert2133 = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val2124, i64 0
  %broadcast.splat2134 = shufflevector <8 x i64> %broadcast.splatinsert2133, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction2135 = or disjoint <8 x i64> %broadcast.splat2134, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body2136

vec.epilog.vector.body2136:                       ; preds = %vec.epilog.vector.body2136, %vec.epilog.ph2130
  %index2137 = phi i64 [ %vec.epilog.resume.val2124, %vec.epilog.ph2130 ], [ %index.next2143, %vec.epilog.vector.body2136 ]
  %vec.ind2138 = phi <8 x i64> [ %induction2135, %vec.epilog.ph2130 ], [ %vec.ind.next2144, %vec.epilog.vector.body2136 ]
  %vec.phi2139 = phi <8 x i64> [ %1603, %vec.epilog.ph2130 ], [ %1604, %vec.epilog.vector.body2136 ]
  %wide.gep2140 = getelementptr inbounds nuw [160 x i8], ptr %1398, <8 x i64> %vec.ind2138
  %wide.gep2141 = getelementptr i8, <8 x ptr> %wide.gep2140, i64 16
  %wide.masked.gather2142 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2141, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !33341
  %1604 = add <8 x i64> %wide.masked.gather2142, %vec.phi2139
  %index.next2143 = add nuw i64 %index2137, 8
  %vec.ind.next2144 = add nuw <8 x i64> %vec.ind2138, splat (i64 8)
  %1605 = icmp eq i64 %index.next2143, %n.vec2132
  br i1 %1605, label %vec.epilog.middle.block2145, label %vec.epilog.vector.body2136, !llvm.loop !33367

vec.epilog.middle.block2145:                      ; preds = %vec.epilog.vector.body2136
  %1606 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %1604)
  %cmp.n2146 = icmp eq i64 %1397, %n.vec2132
  br i1 %cmp.n2146, label %.loopexit2149, label %.preheader294.preheader

.preheader294.preheader:                          ; preds = %iter.check2126, %vec.epilog.iter.check2128, %vec.epilog.middle.block2145
  %.ph2450 = phi i64 [ 0, %iter.check2126 ], [ %n.vec2094, %vec.epilog.iter.check2128 ], [ %n.vec2132, %vec.epilog.middle.block2145 ]
  %.ph2451 = phi i64 [ 0, %iter.check2126 ], [ %1602, %vec.epilog.iter.check2128 ], [ %1606, %vec.epilog.middle.block2145 ]
  br label %.preheader294

1607:                                             ; preds = %1483
  call void @llvm.trap()
  unreachable

1608:                                             ; preds = %1704, %1700, %1588
  %1609 = icmp ult i64 %1397, 57646075230342349
  call void @llvm.assume(i1 %1609)
  %1610 = mul nuw nsw i64 %1397, 160
  %1611 = getelementptr inbounds nuw i8, ptr %1398, i64 %1610
  call void @llvm.lifetime.start.p0(ptr nonnull %79), !noalias !33333
  store ptr %1398, ptr %79, align 8, !noalias !33333
  %1612 = getelementptr inbounds nuw i8, ptr %79, i64 8
  store ptr %1398, ptr %1612, align 8, !noalias !33333
  %1613 = getelementptr inbounds nuw i8, ptr %79, i64 16
  store i64 %1418, ptr %1613, align 8, !noalias !33333
  %1614 = getelementptr inbounds nuw i8, ptr %79, i64 24
  store ptr %1611, ptr %1614, align 8, !noalias !33333
  %1615 = icmp eq i64 %1397, 0
  br i1 %1615, label %.loopexit293, label %1616

1616:                                             ; preds = %1608
  %1617 = getelementptr inbounds nuw i8, ptr %78, i64 8
  %1618 = getelementptr inbounds nuw i8, ptr %78, i64 24
  %1619 = getelementptr inbounds nuw i8, ptr %78, i64 32
  %1620 = getelementptr inbounds nuw i8, ptr %78, i64 40
  %1621 = getelementptr inbounds nuw i8, ptr %78, i64 16
  %1622 = getelementptr inbounds nuw i8, ptr %77, i64 16
  %1623 = getelementptr inbounds nuw i8, ptr %77, i64 8
  %1624 = getelementptr inbounds nuw i8, ptr %77, i64 24
  %1625 = getelementptr inbounds nuw i8, ptr %78, i64 96
  %1626 = getelementptr inbounds nuw i8, ptr %78, i64 48
  %1627 = getelementptr inbounds nuw i8, ptr %78, i64 56
  %1628 = getelementptr inbounds nuw i8, ptr %78, i64 64
  %1629 = getelementptr inbounds nuw i8, ptr %1589, i64 80
  %1630 = getelementptr inbounds nuw i8, ptr %62, i64 1
  %1631 = getelementptr inbounds nuw i8, ptr %62, i64 8
  %1632 = getelementptr inbounds nuw i8, ptr %62, i64 16
  %1633 = getelementptr inbounds nuw i8, ptr %5, i64 632
  %1634 = getelementptr inbounds nuw i8, ptr %5, i64 1228
  %1635 = zext nneg i8 %1445 to i64
  %1636 = getelementptr inbounds nuw i8, ptr %1589, i64 296
  %1637 = getelementptr inbounds nuw i8, ptr %1589, i64 272
  %1638 = getelementptr inbounds nuw i8, ptr %5, i64 1048
  %1639 = getelementptr inbounds nuw i8, ptr %5, i64 1056
  %1640 = getelementptr inbounds nuw i8, ptr %1589, i64 104
  %1641 = getelementptr inbounds nuw i8, ptr %56, i64 1
  %1642 = getelementptr inbounds nuw i8, ptr %56, i64 8
  %1643 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %1644 = getelementptr inbounds nuw i8, ptr %61, i64 8
  %1645 = getelementptr inbounds nuw i8, ptr %50, i64 1
  %1646 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %1647 = getelementptr inbounds nuw i8, ptr %50, i64 16
  %1648 = getelementptr inbounds nuw i8, ptr %57, i64 8
  %1649 = getelementptr inbounds nuw i8, ptr %59, i64 1
  %1650 = getelementptr inbounds nuw i8, ptr %59, i64 8
  %1651 = getelementptr inbounds nuw i8, ptr %59, i64 16
  %1652 = getelementptr inbounds nuw i8, ptr %58, i64 8
  %1653 = getelementptr inbounds nuw i8, ptr %55, i64 8
  %1654 = getelementptr inbounds nuw i8, ptr %54, i64 8
  %1655 = getelementptr inbounds nuw i8, ptr %52, i64 1
  %1656 = getelementptr inbounds nuw i8, ptr %52, i64 8
  %1657 = getelementptr inbounds nuw i8, ptr %52, i64 16
  %1658 = getelementptr inbounds nuw i8, ptr %51, i64 8
  %1659 = getelementptr inbounds nuw i8, ptr %69, i64 8
  %1660 = getelementptr inbounds nuw i8, ptr %70, i64 8
  %1661 = getelementptr inbounds nuw i8, ptr %5, i64 664
  %1662 = getelementptr inbounds nuw i8, ptr %69, i64 16
  %1663 = getelementptr inbounds nuw i8, ptr %70, i64 16
  %1664 = getelementptr inbounds nuw i8, ptr %70, i64 24
  %1665 = getelementptr inbounds nuw i8, ptr %78, i64 88
  %1666 = getelementptr inbounds nuw i8, ptr %78, i64 120
  %1667 = getelementptr inbounds nuw i8, ptr %47, i64 1
  %1668 = getelementptr inbounds nuw i8, ptr %47, i64 8
  %1669 = getelementptr inbounds nuw i8, ptr %47, i64 16
  %1670 = getelementptr inbounds nuw i8, ptr %70, i64 32
  %1671 = getelementptr inbounds nuw i8, ptr %70, i64 33
  %1672 = getelementptr inbounds nuw i8, ptr %70, i64 40
  br label %1708

.preheader294:                                    ; preds = %.preheader294.preheader, %.preheader294
  %1673 = phi i64 [ %1680, %.preheader294 ], [ %.ph2450, %.preheader294.preheader ]
  %1674 = phi i64 [ %1679, %.preheader294 ], [ %.ph2451, %.preheader294.preheader ]
  %1675 = getelementptr inbounds nuw [160 x i8], ptr %1398, i64 %1673
  %1676 = getelementptr i8, ptr %1675, i64 16
  %1677 = load i64, ptr %1676, align 8, !noalias !33341, !noundef !1708
  %1678 = icmp ult i64 %1677, 104811045873349726
  call void @llvm.assume(i1 %1678)
  %1679 = add i64 %1677, %1674
  %1680 = add nuw i64 %1673, 1
  %1681 = icmp eq i64 %1680, %1397
  br i1 %1681, label %.loopexit2149, label %.preheader294, !llvm.loop !33368

1682:                                             ; preds = %1707, %1689
  %1683 = phi i8 [ %1690, %1689 ], [ %1807, %1707 ]
  %1684 = phi i8 [ %1691, %1689 ], [ 0, %1707 ]
  %1685 = phi { ptr, i32 } [ %1692, %1689 ], [ %1808, %1707 ]
  %1686 = atomicrmw sub ptr %1589, i64 1 release, align 8, !noalias !33369
  %1687 = icmp eq i64 %1686, 1
  br i1 %1687, label %1688, label %2495

1688:                                             ; preds = %1682
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %80) #87
          to label %2495 unwind label %1586, !noalias !33341

1689:                                             ; preds = %2430, %1745, %.loopexit293, %1704, %1699
  %1690 = phi i8 [ %2406, %2430 ], [ 1, %1745 ], [ 1, %.loopexit293 ], [ 1, %1704 ], [ 1, %1699 ]
  %1691 = phi i8 [ 0, %2430 ], [ 0, %1745 ], [ 0, %.loopexit293 ], [ 1, %1704 ], [ 1, %1699 ]
  %1692 = landingpad { ptr, i32 }
          cleanup
  br label %1682

.loopexit2149:                                    ; preds = %.preheader294, %vec.epilog.middle.block2145, %middle.block2119
  %.lcssa1994 = phi i64 [ %1606, %vec.epilog.middle.block2145 ], [ %1602, %middle.block2119 ], [ %1679, %.preheader294 ]
  %1693 = getelementptr inbounds nuw i8, ptr %5, i64 912
  %1694 = getelementptr inbounds nuw i8, ptr %5, i64 928
  %1695 = load i64, ptr %1694, align 16, !alias.scope !33374, !noalias !33341, !noundef !1708
  %1696 = load i64, ptr %1693, align 16, !range !1817, !alias.scope !33374, !noalias !33341, !noundef !1708
  %1697 = sub i64 %1696, %1695
  %1698 = icmp ugt i64 %.lcssa1994, %1697
  br i1 %1698, label %1699, label %1700, !prof !4226

1699:                                             ; preds = %.loopexit2149
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1693, i64 noundef %1695, i64 noundef %.lcssa1994, i64 noundef 8, i64 noundef 80)
          to label %1700 unwind label %1689, !noalias !33341

1700:                                             ; preds = %1699, %.loopexit2149
  %1701 = getelementptr inbounds nuw i8, ptr %5, i64 1016
  %1702 = load i64, ptr %1701, align 8, !alias.scope !33379, !noalias !33341, !noundef !1708
  %1703 = icmp ugt i64 %.lcssa1994, %1702
  br i1 %1703, label %1704, label %1608, !prof !4226

1704:                                             ; preds = %1700
  %1705 = getelementptr inbounds nuw i8, ptr %5, i64 1000
; invoke <hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>
  %1706 = invoke { i64, i64 } @<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %1705, i64 noundef %.lcssa1994, ptr noundef nonnull align 8 %1693, i1 noundef zeroext true) #87
          to label %1608 unwind label %1689, !noalias !33341

1707:                                             ; preds = %2494, %2491, %2488
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %79) #89
          to label %1682 unwind label %1586, !noalias !33341

1708:                                             ; preds = %1841, %1616
  %1709 = phi ptr [ %1475, %1616 ], [ %1800, %1841 ]
  %1710 = phi ptr [ %1398, %1616 ], [ %1711, %1841 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !33382)
  %1711 = getelementptr inbounds nuw i8, ptr %1710, i64 160
  store ptr %1711, ptr %1612, align 8, !alias.scope !33382, !noalias !33385
  %1712 = load i64, ptr %1710, align 8, !noalias !33387
  %1713 = icmp eq i64 %1712, -1
  br i1 %1713, label %.loopexit293, label %1714

1714:                                             ; preds = %1708
  %1715 = getelementptr inbounds nuw i8, ptr %1710, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %78), !noalias !33333
  store i64 %1712, ptr %78, align 8, !noalias !33333
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %1617, ptr noundef nonnull align 8 dereferenceable(152) %1715, i64 152, i1 false), !noalias !33341
  %1716 = load i64, ptr %1618, align 8, !noalias !33333
  %1717 = load ptr, ptr %1619, align 8, !noalias !33333
  %1718 = load i64, ptr %1620, align 8, !noalias !33333
  call void @llvm.lifetime.start.p0(ptr nonnull %77), !noalias !33333
  %1719 = load ptr, ptr %1617, align 8, !noalias !33333, !nonnull !1708, !noundef !1708
  %1720 = load i64, ptr %1621, align 8, !noalias !33333, !noundef !1708
  %1721 = icmp ult i64 %1720, 104811045873349726
  call void @llvm.assume(i1 %1721)
  %1722 = getelementptr inbounds nuw [88 x i8], ptr %1719, i64 %1720
  store ptr %1719, ptr %77, align 8, !noalias !33333
  store i64 %1712, ptr %1622, align 8, !noalias !33333
  store ptr %1719, ptr %1623, align 8, !noalias !33333
  store ptr %1722, ptr %1624, align 8, !noalias !33333
  %1723 = load ptr, ptr %1627, align 8, !noalias !33333, !nonnull !1708, !noundef !1708
  %1724 = load i64, ptr %1626, align 8, !range !1817, !noalias !33333, !noundef !1708
  %1725 = load i64, ptr %1628, align 8, !noalias !33333, !noundef !1708
  %1726 = icmp ult i64 %1725, 288230376151711744
  call void @llvm.assume(i1 %1726)
  %1727 = shl nuw nsw i64 %1725, 5
  %1728 = getelementptr inbounds nuw i8, ptr %1723, i64 %1727
  %1729 = icmp eq i64 %1725, 0
  br i1 %1729, label %.loopexit292, label %1730

1730:                                             ; preds = %1714
  %1731 = load i64, ptr %1625, align 8, !noalias !33333, !noundef !1708
  %1732 = icmp ult i64 %1718, 384307168202282326
  %1733 = ptrtoint ptr %1722 to i64
  br label %1783

.loopexit293:                                     ; preds = %1841, %1708, %1608
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %79)
          to label %1734 unwind label %1689, !noalias !33341

1734:                                             ; preds = %.loopexit293
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !33333
  %1735 = xor i1 %1421, true
  %1736 = or i1 %1443, %1735
  %1737 = select i1 %1736, i1 true, i1 %1592
  br i1 %1737, label %1738, label %1745

1738:                                             ; preds = %1747, %1734
  %1739 = load i64, ptr %87, align 8, !noalias !33339
  %1740 = load ptr, ptr %1472, align 8, !noalias !33339
  %1741 = load i64, ptr %1473, align 8, !noalias !33339
  %1742 = atomicrmw sub ptr %1589, i64 1 release, align 8, !noalias !33388
  %1743 = icmp eq i64 %1742, 1
  br i1 %1743, label %1744, label %1748

1744:                                             ; preds = %1738
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %80) #87
          to label %1748 unwind label %1518, !noalias !33341

1745:                                             ; preds = %1734
  call void @llvm.lifetime.start.p0(ptr nonnull %68), !noalias !33333
  call void @llvm.lifetime.start.p0(ptr nonnull %67), !noalias !33333
  store i64 1, ptr %67, align 8, !noalias !33333
  %1746 = getelementptr inbounds nuw i8, ptr %67, i64 8
  store i64 0, ptr %1746, align 8, !noalias !33333
; invoke <purrdf_sparql_eval::governor::GovernorState>::commit_reported_items
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(address) dereferenceable(40) %68, ptr noundef nonnull align 8 %1590, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %67, i64 noundef 1)
          to label %1747 unwind label %1689, !noalias !33341

1747:                                             ; preds = %1745
  call void @llvm.lifetime.end.p0(ptr nonnull %67), !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %68), !noalias !33333
  br label %1738

1748:                                             ; preds = %1744, %1738
  call void @llvm.lifetime.end.p0(ptr nonnull %80), !noalias !33333
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %84)
          to label %1749 unwind label %1530, !noalias !33341

1749:                                             ; preds = %1748
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %87), !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %97), !noalias !33239
  br label %2552

1750:                                             ; preds = %2049
  %1751 = landingpad { ptr, i32 }
          cleanup
  store ptr %2045, ptr %1623, align 8, !noalias !33333
  br label %1778

1752:                                             ; preds = %2027
  %1753 = landingpad { ptr, i32 }
          cleanup
  br label %1778

1754:                                             ; preds = %2115
  %1755 = landingpad { ptr, i32 }
          cleanup
  store ptr %2111, ptr %1623, align 8, !noalias !33333
  br label %1778

1756:                                             ; preds = %2009
  %1757 = landingpad { ptr, i32 }
          cleanup
  store ptr %2005, ptr %1623, align 8, !noalias !33333
  br label %1778

1758:                                             ; preds = %2357
  %1759 = landingpad { ptr, i32 }
          cleanup
  store ptr %2354, ptr %1479, align 8, !noalias !33333
  br label %1778

1760:                                             ; preds = %2337
  %1761 = landingpad { ptr, i32 }
          cleanup
  store ptr %2333, ptr %1623, align 8, !noalias !33333
  br label %1778

1762:                                             ; preds = %2271
  %1763 = landingpad { ptr, i32 }
          cleanup
  br label %1778

1764:                                             ; preds = %2244
  %1765 = landingpad { ptr, i32 }
          cleanup
  store ptr %2240, ptr %1623, align 8, !noalias !33333
  br label %1778

1766:                                             ; preds = %2186
  %1767 = landingpad { ptr, i32 }
          cleanup
  store ptr %2182, ptr %1623, align 8, !noalias !33333
  br label %1778

1768:                                             ; preds = %2147, %2131, %2099
  %1769 = landingpad { ptr, i32 }
          cleanup
  br label %1778

1770:                                             ; preds = %.preheader288
  %1771 = landingpad { ptr, i32 }
          cleanup
  br label %1778

1772:                                             ; preds = %1862
  %1773 = landingpad { ptr, i32 }
          cleanup
  br label %1778

1774:                                             ; preds = %2316, %2312, %2303
  %1775 = landingpad { ptr, i32 }
          cleanup
  br label %1778

1776:                                             ; preds = %1890, %1845
  %1777 = landingpad { ptr, i32 }
          cleanup
  br label %1778

1778:                                             ; preds = %2385, %2382, %1776, %1774, %1772, %1770, %1768, %1766, %1764, %1762, %1760, %1758, %1756, %1754, %1752, %1750
  %1779 = phi { ptr, i32 } [ %2383, %2382 ], [ %2383, %2385 ], [ %1751, %1750 ], [ %1753, %1752 ], [ %1755, %1754 ], [ %1757, %1756 ], [ %1759, %1758 ], [ %1761, %1760 ], [ %1763, %1762 ], [ %1765, %1764 ], [ %1767, %1766 ], [ %1769, %1768 ], [ %1771, %1770 ], [ %1773, %1772 ], [ %1775, %1774 ], [ %1777, %1776 ]
  %1780 = icmp eq i64 %1724, 0
  br i1 %1780, label %1804, label %1781

1781:                                             ; preds = %1778
  %1782 = shl nuw i64 %1724, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1723, i64 noundef %1782, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33393
  br label %1804

1783:                                             ; preds = %.loopexit274, %1730
  %1784 = phi ptr [ %1709, %1730 ], [ %2351, %.loopexit274 ]
  %1785 = phi ptr [ %1719, %1730 ], [ %2166, %.loopexit274 ]
  %1786 = phi i64 [ 0, %1730 ], [ %1888, %.loopexit274 ]
  %1787 = phi ptr [ %1723, %1730 ], [ %1790, %.loopexit274 ]
  %1788 = phi ptr [ %1719, %1730 ], [ %2168, %.loopexit274 ]
  %1789 = phi i64 [ %1731, %1730 ], [ %2167, %.loopexit274 ]
  %1790 = getelementptr inbounds nuw i8, ptr %1787, i64 32
  %1791 = load i64, ptr %1787, align 8, !noalias !33396
  %1792 = getelementptr inbounds nuw i8, ptr %1787, i64 8
  %1793 = load i64, ptr %1792, align 8, !noalias !33396
  %1794 = getelementptr inbounds nuw i8, ptr %1787, i64 16
  %1795 = load i64, ptr %1794, align 8, !noalias !33396
  %1796 = getelementptr inbounds nuw i8, ptr %1787, i64 24
  %1797 = load i64, ptr %1796, align 8, !noalias !33396
  %1798 = icmp eq i64 %1791, 0
  %1799 = select i1 %1798, i1 true, i1 %1592
  br i1 %1799, label %1843, label %1850

.loopexit292:                                     ; preds = %.loopexit274, %1714
  %1800 = phi ptr [ %1709, %1714 ], [ %2351, %.loopexit274 ]
  %1801 = icmp eq i64 %1724, 0
  br i1 %1801, label %1805, label %1802

1802:                                             ; preds = %.loopexit292
  %1803 = shl nuw i64 %1724, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1723, i64 noundef %1803, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33399
  br label %1805

1804:                                             ; preds = %1781, %1778
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %77) #89
          to label %1806 unwind label %1586, !noalias !33341

1805:                                             ; preds = %1802, %.loopexit292
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %77)
          to label %1816 unwind label %1812, !noalias !33341

1806:                                             ; preds = %1814, %1812, %1804
  %1807 = phi i8 [ 1, %1804 ], [ 1, %1812 ], [ %2406, %1814 ]
  %1808 = phi { ptr, i32 } [ %1779, %1804 ], [ %1813, %1812 ], [ %1815, %1814 ]
  %1809 = icmp eq i64 %1716, 0
  br i1 %1809, label %1820, label %1810

1810:                                             ; preds = %1806
  %1811 = mul nuw i64 %1716, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1717) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1717, i64 noundef %1811, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33341
  br label %1820

1812:                                             ; preds = %1805
  %1813 = landingpad { ptr, i32 }
          cleanup
  br label %1806

1814:                                             ; preds = %2411
  %1815 = landingpad { ptr, i32 }
          cleanup
  br label %1806

1816:                                             ; preds = %1805
  call void @llvm.lifetime.end.p0(ptr nonnull %77), !noalias !33333
  %1817 = icmp eq i64 %1716, 0
  br i1 %1817, label %1827, label %1818

1818:                                             ; preds = %1816
  %1819 = mul nuw i64 %1716, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1717) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1717, i64 noundef %1819, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33341
  br label %1827

1820:                                             ; preds = %1810, %1806
  call void @llvm.experimental.noalias.scope.decl(metadata !33402)
  %1821 = load ptr, ptr %1665, align 8, !alias.scope !33402, !noalias !33333, !noundef !1708
  %1822 = icmp eq ptr %1821, null
  br i1 %1822, label %2488, label %1823

1823:                                             ; preds = %1820
  %1824 = atomicrmw sub ptr %1821, i64 1 release, align 8, !noalias !33405
  %1825 = icmp eq i64 %1824, 1
  br i1 %1825, label %1826, label %2488

1826:                                             ; preds = %1823
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1665) #87, !noalias !33341
  br label %2488

1827:                                             ; preds = %1818, %1816
  call void @llvm.experimental.noalias.scope.decl(metadata !33410)
  %1828 = load ptr, ptr %1665, align 8, !alias.scope !33410, !noalias !33333, !noundef !1708
  %1829 = icmp eq ptr %1828, null
  br i1 %1829, label %1834, label %1830

1830:                                             ; preds = %1827
  %1831 = atomicrmw sub ptr %1828, i64 1 release, align 8, !noalias !33413
  %1832 = icmp eq i64 %1831, 1
  br i1 %1832, label %1833, label %1834

1833:                                             ; preds = %1830
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1665) #87, !noalias !33341
  br label %1834

1834:                                             ; preds = %1833, %1830, %1827
  call void @llvm.experimental.noalias.scope.decl(metadata !33418)
  %1835 = load ptr, ptr %1666, align 8, !alias.scope !33418, !noalias !33333, !noundef !1708
  %1836 = icmp eq ptr %1835, null
  br i1 %1836, label %1841, label %1837

1837:                                             ; preds = %1834
  %1838 = atomicrmw sub ptr %1835, i64 1 release, align 8, !noalias !33421
  %1839 = icmp eq i64 %1838, 1
  br i1 %1839, label %1840, label %1841

1840:                                             ; preds = %1837
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1666) #87, !noalias !33341
  br label %1841

1841:                                             ; preds = %1840, %1837, %1834
  call void @llvm.lifetime.end.p0(ptr nonnull %78), !noalias !33333
  %1842 = icmp eq ptr %1711, %1611
  br i1 %1842, label %.loopexit293, label %1708

1843:                                             ; preds = %1879, %1873, %1866, %1783
  call void @llvm.assume(i1 %1732)
  %1844 = icmp ugt i64 %1786, %1718
  br i1 %1844, label %1845, label %1885, !prof !1803

1845:                                             ; preds = %1843
  call void @llvm.lifetime.start.p0(ptr nonnull %66), !noalias !33333
  store i64 %1786, ptr %66, align 8, !noalias !33333
  call void @llvm.lifetime.start.p0(ptr nonnull %65), !noalias !33333
  store i64 %1718, ptr %65, align 8, !noalias !33333
  call void @llvm.lifetime.start.p0(ptr nonnull %64), !noalias !33333
  store ptr %66, ptr %64, align 8, !noalias !33333
  %1846 = getelementptr inbounds nuw i8, ptr %64, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %1846, align 8, !noalias !33333
  %1847 = getelementptr inbounds nuw i8, ptr %64, i64 16
  store ptr %65, ptr %1847, align 8, !noalias !33333
  %1848 = getelementptr inbounds nuw i8, ptr %64, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %1848, align 8, !noalias !33333
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.a12f493ba210922c94e5446ac885c35e.2054, ptr noundef nonnull %64, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.300) #92
          to label %1849 unwind label %1776, !noalias !33341

1849:                                             ; preds = %1845
  unreachable

1850:                                             ; preds = %1783
  call void @llvm.lifetime.start.p0(ptr nonnull %63), !noalias !33426
  %1851 = load atomic i64, ptr %1629 monotonic, align 8, !noalias !33433
  br label %1852

1852:                                             ; preds = %1852, %1850
  %1853 = phi i64 [ %1851, %1850 ], [ %1857, %1852 ]
  %1854 = call i64 @llvm.uadd.sat.i64(i64 %1853, i64 %1791)
  %1855 = cmpxchg weak ptr %1629, i64 %1853, i64 %1854 monotonic monotonic, align 8, !noalias !33433
  %1856 = extractvalue { i64, i1 } %1855, 1
  %1857 = extractvalue { i64, i1 } %1855, 0
  br i1 %1856, label %1858, label %1852

1858:                                             ; preds = %1852
  %1859 = call i64 @llvm.uadd.sat.i64(i64 %1857, i64 %1791)
  %1860 = load i64, ptr %1590, align 8, !noalias !33433
  %1861 = icmp ugt i64 %1859, %1860
  br i1 %1861, label %1862, label %1866

1862:                                             ; preds = %1858
  call void @llvm.lifetime.start.p0(ptr nonnull %62), !noalias !33436
  store i8 0, ptr %1630, align 1, !noalias !33436
  store i64 %1860, ptr %1631, align 8, !noalias !33436
  store i64 %1859, ptr %1632, align 8, !noalias !33436
  store i8 0, ptr %62, align 8, !noalias !33436
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %63, ptr noundef nonnull align 8 %1590, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %62)
          to label %1863 unwind label %1772, !noalias !33341

1863:                                             ; preds = %1862
  call void @llvm.lifetime.end.p0(ptr nonnull %62), !noalias !33436
  %1864 = load i8, ptr %63, align 8, !noalias !33426
  %1865 = icmp eq i8 %1864, -1
  br i1 %1865, label %1866, label %1869

1866:                                             ; preds = %1863, %1858
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !33426
  %1867 = load ptr, ptr %1633, align 8, !alias.scope !33340, !noalias !33341, !noundef !1708
  %1868 = icmp eq ptr %1867, null
  br i1 %1868, label %1843, label %1873

1869:                                             ; preds = %1863
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !33426
  %1870 = load i64, ptr %87, align 8, !noalias !33339
  %1871 = load ptr, ptr %1472, align 8, !noalias !33339
  %1872 = load i64, ptr %1473, align 8, !noalias !33339
  br label %2398

1873:                                             ; preds = %1866
  %1874 = load i32, ptr %1634, align 4, !alias.scope !33340, !noalias !33341, !noundef !1708
  %1875 = getelementptr i8, ptr %1867, i64 56
  %1876 = load i64, ptr %1875, align 8, !noalias !33341, !noundef !1708
  %1877 = zext i32 %1874 to i64
  %1878 = icmp ugt i64 %1876, %1877
  br i1 %1878, label %1879, label %1843

1879:                                             ; preds = %1873
  %1880 = getelementptr i8, ptr %1867, i64 48
  %1881 = load ptr, ptr %1880, align 8, !noalias !33341, !nonnull !1708, !noundef !1708
  %1882 = getelementptr inbounds nuw [136 x i8], ptr %1881, i64 %1877
  %1883 = getelementptr inbounds nuw [8 x i8], ptr %1882, i64 %1635
  %1884 = atomicrmw add ptr %1883, i64 %1791 monotonic, align 8, !noalias !33341
  br label %1843

1885:                                             ; preds = %1843
  %1886 = icmp ult i64 %1793, %1786
  %1887 = call i64 @llvm.umin.i64(i64 %1793, i64 range(i64 0, 384307168202282326) %1718)
  %1888 = select i1 %1886, i64 %1786, i64 %1887
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1717) ]
  %1889 = icmp samesign ult i64 %1888, %1786
  br i1 %1889, label %1890, label %1891, !prof !10448

1890:                                             ; preds = %1885
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %1786, i64 noundef %1888, i64 noundef %1718, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.301) #90
          to label %2410 unwind label %1776, !noalias !33341

1891:                                             ; preds = %1885
  %1892 = mul nuw nsw i64 %1786, 24
  %1893 = getelementptr inbounds nuw i8, ptr %1717, i64 %1892
  %1894 = mul nuw nsw i64 %1888, 24
  %1895 = getelementptr inbounds nuw i8, ptr %1717, i64 %1894
  %1896 = icmp eq i64 %1786, %1888
  br i1 %1896, label %.loopexit291, label %1897

1897:                                             ; preds = %1891
  %1898 = sub nuw nsw i64 %1894, %1892
  %1899 = udiv exact i64 %1898, 24
  br label %1900

1900:                                             ; preds = %1915, %1897
  %1901 = phi i64 [ 0, %1897 ], [ %1916, %1915 ]
  %1902 = phi i64 [ 0, %1897 ], [ %1917, %1915 ]
  %1903 = phi i64 [ 0, %1897 ], [ %1918, %1915 ]
  %1904 = phi i64 [ 0, %1897 ], [ %1919, %1915 ]
  %1905 = getelementptr inbounds nuw [24 x i8], ptr %1893, i64 %1904
  %1906 = load i8, ptr %1905, align 8, !range !10848, !noalias !33437, !noundef !1708
  %1907 = getelementptr i8, ptr %1905, i64 8
  %1908 = load i64, ptr %1907, align 8, !noalias !33437
  switch i8 %1906, label %.unreachabledefault [
    i8 0, label %1909
    i8 1, label %1911
    i8 2, label %1915
    i8 3, label %1913
  ]

.unreachabledefault:                              ; preds = %1900
  unreachable

default.unreachable1340:                          ; preds = %.preheader278
  unreachable

1909:                                             ; preds = %1900
  %1910 = call i64 @llvm.uadd.sat.i64(i64 %1903, i64 %1908)
  br label %1915

1911:                                             ; preds = %1900
  %1912 = call i64 @llvm.uadd.sat.i64(i64 %1902, i64 %1908)
  br label %1915

1913:                                             ; preds = %1900
  %1914 = call i64 @llvm.umax.i64(i64 %1901, i64 %1908)
  br label %1915

1915:                                             ; preds = %1913, %1911, %1909, %1900
  %1916 = phi i64 [ %1901, %1909 ], [ %1901, %1911 ], [ %1914, %1913 ], [ %1901, %1900 ]
  %1917 = phi i64 [ %1902, %1909 ], [ %1912, %1911 ], [ %1902, %1913 ], [ %1902, %1900 ]
  %1918 = phi i64 [ %1910, %1909 ], [ %1903, %1911 ], [ %1903, %1913 ], [ %1903, %1900 ]
  %1919 = add nuw i64 %1904, 1
  %1920 = icmp eq i64 %1919, %1899
  br i1 %1920, label %.loopexit291, label %1900

.loopexit291:                                     ; preds = %1915, %1891
  %1921 = phi i64 [ 0, %1891 ], [ %1918, %1915 ]
  %1922 = phi i64 [ 0, %1891 ], [ %1917, %1915 ]
  %1923 = phi i64 [ 0, %1891 ], [ %1916, %1915 ]
  br i1 %1595, label %1927, label %.loopexit289

.loopexit289:                                     ; preds = %1939, %1927, %.loopexit291
  %1924 = phi i64 [ 0, %.loopexit291 ], [ 0, %1927 ], [ %1941, %1939 ]
  %1925 = load atomic i32, ptr %1636 acquire, align 8, !noalias !33441
  %1926 = icmp eq i32 %1925, 0
  br i1 %1926, label %1943, label %1946

1927:                                             ; preds = %.loopexit291
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1788) ]
  %1928 = ptrtoint ptr %1788 to i64
  %1929 = call i64 @llvm.usub.sat.i64(i64 %1795, i64 %1789)
  %1930 = sub nuw i64 %1733, %1928
  %1931 = udiv exact i64 %1930, 88
  %1932 = call i64 @llvm.umin.i64(i64 %1929, i64 %1931)
  %1933 = icmp eq i64 %1932, 0
  br i1 %1933, label %.loopexit289, label %.preheader288

.preheader288:                                    ; preds = %1927, %1939
  %1934 = phi i64 [ %1941, %1939 ], [ 0, %1927 ]
  %1935 = phi i64 [ %1940, %1939 ], [ 0, %1927 ]
  %1936 = getelementptr inbounds nuw [88 x i8], ptr %1788, i64 %1935
  %1937 = getelementptr inbounds nuw i8, ptr %1936, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %1938 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %1937)
          to label %1939 unwind label %1770, !noalias !33341

1939:                                             ; preds = %.preheader288
  %1940 = add nuw nsw i64 %1935, 1
  %1941 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %1934, i64 %1938)
  %1942 = icmp eq i64 %1940, %1932
  br i1 %1942, label %.loopexit289, label %.preheader288

1943:                                             ; preds = %.loopexit289
  %1944 = load i8, ptr %1637, align 8, !noalias !33341
  %1945 = icmp eq i8 %1944, -1
  br i1 %1945, label %1946, label %1953

1946:                                             ; preds = %1943, %.loopexit289
  br i1 %1592, label %1947, label %1948

1947:                                             ; preds = %1948, %1946
  br i1 %1595, label %1956, label %1954

1948:                                             ; preds = %1946
  %1949 = load atomic i64, ptr %1629 monotonic, align 8, !noalias !33444
  %1950 = load i64, ptr %1590, align 8, !noalias !33444
  %1951 = call i64 @llvm.uadd.sat.i64(i64 %1949, i64 %1921)
  %1952 = icmp ugt i64 %1951, %1950
  br i1 %1952, label %1953, label %1947

1953:                                             ; preds = %1956, %1948, %1943
  br i1 %1896, label %.loopexit280, label %.preheader278

1954:                                             ; preds = %1956, %1947
  %1955 = or i1 %1592, %1896
  br i1 %1955, label %.loopexit287, label %.preheader286

1956:                                             ; preds = %1947
  %1957 = load i64, ptr %1638, align 8, !alias.scope !33340, !noalias !33341, !noundef !1708
  %1958 = load atomic i64, ptr %1639 monotonic, align 16, !alias.scope !33340, !noalias !33341
  %1959 = call noundef i64 @llvm.usub.sat.i64(i64 %1957, i64 %1958)
  %1960 = call i64 @llvm.uadd.sat.i64(i64 %1959, i64 %1924)
  %1961 = call i64 @llvm.uadd.sat.i64(i64 %1960, i64 %1922)
  %1962 = call i64 @llvm.uadd.sat.i64(i64 %1961, i64 %1923)
  %1963 = load atomic i64, ptr %1640 monotonic, align 8, !noalias !33447
  %1964 = load i64, ptr %1593, align 8, !noalias !33447
  %1965 = call i64 @llvm.uadd.sat.i64(i64 %1963, i64 %1962)
  %1966 = icmp ugt i64 %1965, %1964
  br i1 %1966, label %1953, label %1954

.preheader278:                                    ; preds = %1953, %2149
  %1967 = phi ptr [ %2150, %2149 ], [ %1785, %1953 ]
  %1968 = phi ptr [ %1972, %2149 ], [ %1893, %1953 ]
  %1969 = phi ptr [ %2153, %2149 ], [ %1788, %1953 ]
  %1970 = phi i64 [ %2152, %2149 ], [ %1789, %1953 ]
  %1971 = phi ptr [ %2151, %2149 ], [ %1785, %1953 ]
  %1972 = getelementptr inbounds nuw i8, ptr %1968, i64 24
  %1973 = load i8, ptr %1968, align 8, !range !10848, !noalias !33341, !noundef !1708
  switch i8 %1973, label %default.unreachable1340 [
    i8 0, label %1979
    i8 1, label %1986
    i8 2, label %1991
    i8 3, label %2014
  ]

.loopexit280:                                     ; preds = %2149, %1953
  %1974 = phi ptr [ %1785, %1953 ], [ %2150, %2149 ]
  %1975 = phi i64 [ %1789, %1953 ], [ %2152, %2149 ]
  %1976 = phi ptr [ %1788, %1953 ], [ %2153, %2149 ]
  %1977 = icmp ult i64 %1975, %1795
  %1978 = select i1 %1595, i1 %1977, i1 false
  br i1 %1978, label %2172, label %2165

1979:                                             ; preds = %.preheader278
  %1980 = getelementptr inbounds nuw i8, ptr %1968, i64 1
  %1981 = load i8, ptr %1980, align 1, !range !1905, !noalias !33341, !noundef !1708
  %1982 = getelementptr inbounds nuw i8, ptr %1968, i64 8
  %1983 = load i64, ptr %1982, align 8, !noalias !33341, !noundef !1708
  %1984 = getelementptr inbounds nuw i8, ptr %1968, i64 16
  %1985 = load i64, ptr %1984, align 8, !noalias !33341, !noundef !1708
  br i1 %1592, label %2149, label %2015

1986:                                             ; preds = %.preheader278
  %1987 = getelementptr inbounds nuw i8, ptr %1968, i64 8
  %1988 = load i64, ptr %1987, align 8, !noalias !33341, !noundef !1708
  %1989 = getelementptr inbounds nuw i8, ptr %1968, i64 16
  %1990 = load i64, ptr %1989, align 8, !noalias !33341, !noundef !1708
  br i1 %1595, label %2074, label %2149

1991:                                             ; preds = %.preheader278
  %1992 = getelementptr inbounds nuw i8, ptr %1968, i64 8
  %1993 = load i64, ptr %1992, align 8, !noalias !33341, !noundef !1708
  %1994 = icmp ult i64 %1970, %1993
  br i1 %1994, label %1995, label %2126

1995:                                             ; preds = %1991
  %1996 = icmp eq ptr %1969, %1722
  br i1 %1996, label %.loopexit273, label %1997

1997:                                             ; preds = %1995
  %1998 = add i64 %1993, -1
  br label %2002

1999:                                             ; preds = %2012
  %2000 = add i64 %2004, 1
  %2001 = icmp eq ptr %2005, %1722
  br i1 %2001, label %.loopexit273, label %2002

2002:                                             ; preds = %1999, %1997
  %2003 = phi ptr [ %2005, %1999 ], [ %1969, %1997 ]
  %2004 = phi i64 [ %2000, %1999 ], [ %1970, %1997 ]
  %2005 = getelementptr inbounds nuw i8, ptr %2003, i64 88
  %2006 = getelementptr inbounds nuw i8, ptr %2003, i64 8
  %2007 = load i64, ptr %2006, align 8, !noalias !33450
  %2008 = icmp eq i64 %2007, -1
  br i1 %2008, label %.loopexit273, label %2009

2009:                                             ; preds = %2002
  %2010 = getelementptr inbounds nuw i8, ptr %2003, i64 16
  %2011 = load i64, ptr %2003, align 8, !noalias !33450
  call void @llvm.lifetime.start.p0(ptr nonnull %61), !noalias !33453
  store i64 %2007, ptr %61, align 8, !noalias !33453
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1644, ptr noundef nonnull align 8 dereferenceable(72) %2010, i64 72, i1 false), !noalias !33341
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %687, i64 noundef %2011, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %61)
          to label %2012 unwind label %1756, !noalias !33341

2012:                                             ; preds = %2009
  call void @llvm.lifetime.end.p0(ptr nonnull %61), !noalias !33453
  %2013 = icmp eq i64 %2004, %1998
  br i1 %2013, label %.loopexit273, label %1999

2014:                                             ; preds = %.preheader278
  br i1 %1595, label %2135, label %2149

2015:                                             ; preds = %1979
  call void @llvm.lifetime.start.p0(ptr nonnull %60), !noalias !33456
  %2016 = load atomic i64, ptr %1629 monotonic, align 8, !noalias !33463
  br label %2017

2017:                                             ; preds = %2017, %2015
  %2018 = phi i64 [ %2016, %2015 ], [ %2022, %2017 ]
  %2019 = call i64 @llvm.uadd.sat.i64(i64 %2018, i64 %1983)
  %2020 = cmpxchg weak ptr %1629, i64 %2018, i64 %2019 monotonic monotonic, align 8, !noalias !33463
  %2021 = extractvalue { i64, i1 } %2020, 1
  %2022 = extractvalue { i64, i1 } %2020, 0
  br i1 %2021, label %2023, label %2017

2023:                                             ; preds = %2017
  %2024 = call i64 @llvm.uadd.sat.i64(i64 %2022, i64 %1983)
  %2025 = load i64, ptr %1590, align 8, !noalias !33463
  %2026 = icmp ugt i64 %2024, %2025
  br i1 %2026, label %2027, label %2036

2027:                                             ; preds = %2023
  call void @llvm.lifetime.start.p0(ptr nonnull %59), !noalias !33466
  store i8 0, ptr %1649, align 1, !noalias !33466
  store i64 %2025, ptr %1650, align 8, !noalias !33466
  store i64 %2024, ptr %1651, align 8, !noalias !33466
  store i8 0, ptr %59, align 8, !noalias !33466
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %60, ptr noundef nonnull align 8 %1590, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %59)
          to label %2028 unwind label %1752, !noalias !33341

2028:                                             ; preds = %2027
  call void @llvm.lifetime.end.p0(ptr nonnull %59), !noalias !33466
  %2029 = load i8, ptr %60, align 8, !noalias !33456
  %2030 = icmp eq i8 %2029, -1
  br i1 %2030, label %2036, label %2031

2031:                                             ; preds = %2028
  call void @llvm.lifetime.end.p0(ptr nonnull %60), !noalias !33456
  %2032 = icmp ne i64 %1985, 0
  %2033 = add i64 %1985, -1
  %2034 = icmp ult i64 %1970, %2033
  %2035 = select i1 %2032, i1 %2034, i1 false
  br i1 %2035, label %2038, label %.loopexit279

2036:                                             ; preds = %2028, %2023
  call void @llvm.lifetime.end.p0(ptr nonnull %60), !noalias !33456
  %2037 = icmp eq i8 %1981, -1
  br i1 %2037, label %2149, label %2057

2038:                                             ; preds = %2031
  %2039 = icmp eq ptr %1969, %1722
  br i1 %2039, label %.loopexit271, label %2040

2040:                                             ; preds = %2038
  %2041 = add i64 %1985, -2
  br label %2042

2042:                                             ; preds = %2052, %2040
  %2043 = phi ptr [ %2045, %2052 ], [ %1969, %2040 ]
  %2044 = phi i64 [ %2054, %2052 ], [ %1970, %2040 ]
  %2045 = getelementptr inbounds nuw i8, ptr %2043, i64 88
  %2046 = getelementptr inbounds nuw i8, ptr %2043, i64 8
  %2047 = load i64, ptr %2046, align 8, !noalias !33467
  %2048 = icmp eq i64 %2047, -1
  br i1 %2048, label %.loopexit271, label %2049

2049:                                             ; preds = %2042
  %2050 = getelementptr inbounds nuw i8, ptr %2043, i64 16
  %2051 = load i64, ptr %2043, align 8, !noalias !33467
  call void @llvm.lifetime.start.p0(ptr nonnull %58), !noalias !33470
  store i64 %2047, ptr %58, align 8, !noalias !33470
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1652, ptr noundef nonnull align 8 dereferenceable(72) %2050, i64 72, i1 false), !noalias !33341
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %687, i64 noundef %2051, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %58)
          to label %2052 unwind label %1750, !noalias !33341

2052:                                             ; preds = %2049
  call void @llvm.lifetime.end.p0(ptr nonnull %58), !noalias !33470
  %2053 = icmp eq i64 %2044, %2041
  %2054 = add nuw i64 %2044, 1
  %2055 = icmp eq ptr %2045, %1722
  %2056 = select i1 %2053, i1 true, i1 %2055
  br i1 %2056, label %.loopexit271, label %2042

2057:                                             ; preds = %2036
  %2058 = load ptr, ptr %1633, align 8, !alias.scope !33340, !noalias !33341, !noundef !1708
  %2059 = icmp eq ptr %2058, null
  br i1 %2059, label %2149, label %2060

2060:                                             ; preds = %2057
  %2061 = load i32, ptr %1634, align 4, !alias.scope !33340, !noalias !33341, !noundef !1708
  %2062 = getelementptr i8, ptr %2058, i64 56
  %2063 = load i64, ptr %2062, align 8, !noalias !33341, !noundef !1708
  %2064 = zext i32 %2061 to i64
  %2065 = icmp ugt i64 %2063, %2064
  br i1 %2065, label %2066, label %2149

2066:                                             ; preds = %2060
  %2067 = getelementptr i8, ptr %2058, i64 48
  %2068 = load ptr, ptr %2067, align 8, !noalias !33341, !nonnull !1708, !noundef !1708
  %2069 = zext nneg i8 %1981 to i64
  %2070 = getelementptr inbounds nuw [136 x i8], ptr %2068, i64 %2064
  %2071 = getelementptr inbounds nuw [8 x i8], ptr %2070, i64 %2069
  %2072 = atomicrmw add ptr %2071, i64 %1983 monotonic, align 8, !noalias !33341
  br label %2149

2073:                                             ; preds = %2078
  br i1 %2080, label %.loopexit279, label %2149

2074:                                             ; preds = %1986
  call void @llvm.lifetime.start.p0(ptr nonnull %73), !noalias !33333
  %2075 = load i64, ptr %1593, align 8, !noalias !33341
  %2076 = icmp eq i64 %2075, -1
  br i1 %2076, label %2077, label %2083

2077:                                             ; preds = %2094, %2074
  call void @llvm.lifetime.end.p0(ptr nonnull %73), !noalias !33333
  br label %2149

2078:                                             ; preds = %2100, %2098
  %2079 = load i8, ptr %73, align 8, !noalias !33333
  %2080 = icmp ne i8 %2079, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %73), !noalias !33333
  %2081 = icmp ne i64 %1990, 0
  %2082 = and i1 %2081, %2080
  br i1 %2082, label %2101, label %2073

2083:                                             ; preds = %2074
  %2084 = load atomic i32, ptr %1636 acquire, align 8, !noalias !33473
  %2085 = icmp eq i32 %2084, 0
  br i1 %2085, label %2098, label %2086

2086:                                             ; preds = %2083
  %2087 = load atomic i64, ptr %1640 monotonic, align 8, !noalias !33473
  br label %2088

2088:                                             ; preds = %2088, %2086
  %2089 = phi i64 [ %2087, %2086 ], [ %2093, %2088 ]
  %2090 = call i64 @llvm.uadd.sat.i64(i64 %2089, i64 %1988)
  %2091 = cmpxchg weak ptr %1640, i64 %2089, i64 %2090 monotonic monotonic, align 8, !noalias !33473
  %2092 = extractvalue { i64, i1 } %2091, 1
  %2093 = extractvalue { i64, i1 } %2091, 0
  br i1 %2092, label %2094, label %2088

2094:                                             ; preds = %2088
  %2095 = call i64 @llvm.uadd.sat.i64(i64 %2093, i64 %1988)
  %2096 = load i64, ptr %1593, align 8, !noalias !33473
  %2097 = icmp ugt i64 %2095, %2096
  br i1 %2097, label %2099, label %2077

2098:                                             ; preds = %2083
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %73, ptr noundef nonnull align 8 dereferenceable(24) %1637, i64 24, i1 false), !noalias !33341
  br label %2078

2099:                                             ; preds = %2094
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !33476
  store i8 3, ptr %1645, align 1, !noalias !33476
  store i64 %2096, ptr %1646, align 8, !noalias !33476
  store i64 %2095, ptr %1647, align 8, !noalias !33476
  store i8 0, ptr %50, align 8, !noalias !33476
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %73, ptr noundef nonnull align 8 %1590, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %50)
          to label %2100 unwind label %1768, !noalias !33341

2100:                                             ; preds = %2099
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !33476
  br label %2078

2101:                                             ; preds = %2078
  %2102 = add i64 %1990, -1
  %2103 = icmp ult i64 %1970, %2102
  br i1 %2103, label %2104, label %.loopexit279

2104:                                             ; preds = %2101
  %2105 = icmp eq ptr %1969, %1722
  br i1 %2105, label %.loopexit271, label %2106

2106:                                             ; preds = %2104
  %2107 = add i64 %1990, -2
  br label %2108

2108:                                             ; preds = %2118, %2106
  %2109 = phi ptr [ %2111, %2118 ], [ %1969, %2106 ]
  %2110 = phi i64 [ %2120, %2118 ], [ %1970, %2106 ]
  %2111 = getelementptr inbounds nuw i8, ptr %2109, i64 88
  %2112 = getelementptr inbounds nuw i8, ptr %2109, i64 8
  %2113 = load i64, ptr %2112, align 8, !noalias !33477
  %2114 = icmp eq i64 %2113, -1
  br i1 %2114, label %.loopexit271, label %2115

2115:                                             ; preds = %2108
  %2116 = getelementptr inbounds nuw i8, ptr %2109, i64 16
  %2117 = load i64, ptr %2109, align 8, !noalias !33477
  call void @llvm.lifetime.start.p0(ptr nonnull %57), !noalias !33480
  store i64 %2113, ptr %57, align 8, !noalias !33480
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1648, ptr noundef nonnull align 8 dereferenceable(72) %2116, i64 72, i1 false), !noalias !33341
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %687, i64 noundef %2117, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %57)
          to label %2118 unwind label %1754, !noalias !33341

2118:                                             ; preds = %2115
  call void @llvm.lifetime.end.p0(ptr nonnull %57), !noalias !33480
  %2119 = icmp eq i64 %2110, %2107
  %2120 = add nuw i64 %2110, 1
  %2121 = icmp eq ptr %2111, %1722
  %2122 = select i1 %2119, i1 true, i1 %2121
  br i1 %2122, label %.loopexit271, label %2108

.loopexit273:                                     ; preds = %2012, %2002, %1999, %1995
  %2123 = phi ptr [ %1971, %1995 ], [ %2005, %1999 ], [ %2005, %2002 ], [ %2005, %2012 ]
  %2124 = phi i64 [ %1970, %1995 ], [ %1993, %2012 ], [ %2004, %2002 ], [ %2000, %1999 ]
  %2125 = phi ptr [ %1969, %1995 ], [ %2005, %1999 ], [ %2005, %2002 ], [ %2005, %2012 ]
  store ptr %2123, ptr %1623, align 8, !noalias !33333
  br label %2126

2126:                                             ; preds = %.loopexit273, %1991
  %2127 = phi ptr [ %1967, %1991 ], [ %2123, %.loopexit273 ]
  %2128 = phi ptr [ %1971, %1991 ], [ %2123, %.loopexit273 ]
  %2129 = phi i64 [ %1970, %1991 ], [ %2124, %.loopexit273 ]
  %2130 = phi ptr [ %1969, %1991 ], [ %2125, %.loopexit273 ]
  br i1 %1595, label %2131, label %2149

2131:                                             ; preds = %2126
  call void @llvm.lifetime.start.p0(ptr nonnull %72), !noalias !33333
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %72, ptr noundef nonnull align 16 dereferenceable(1248) %5)
          to label %2132 unwind label %1768, !noalias !33341

2132:                                             ; preds = %2131
  %2133 = load i8, ptr %72, align 8, !range !1906, !noalias !33333, !noundef !1708
  %2134 = icmp eq i8 %2133, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %72), !noalias !33333
  br i1 %2134, label %2149, label %.loopexit279

2135:                                             ; preds = %2014
  %2136 = getelementptr inbounds nuw i8, ptr %1968, i64 8
  %2137 = load i64, ptr %2136, align 8, !noalias !33341, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %71), !noalias !33333
  %2138 = load atomic i32, ptr %1636 acquire, align 8, !noalias !33483
  %2139 = icmp eq i32 %2138, 0
  br i1 %2139, label %2140, label %2141

2140:                                             ; preds = %2135
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %71, ptr noundef nonnull align 8 dereferenceable(24) %1637, i64 24, i1 false), !noalias !33341
  br label %2155

2141:                                             ; preds = %2135
  %2142 = load atomic i64, ptr %1640 monotonic, align 8, !noalias !33483
  %2143 = call i64 @llvm.uadd.sat.i64(i64 %2142, i64 %2137)
  %2144 = load i64, ptr %1593, align 8, !noalias !33483
  %2145 = icmp ugt i64 %2143, %2144
  br i1 %2145, label %2147, label %2146

2146:                                             ; preds = %2141
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !33333
  br label %2149

2147:                                             ; preds = %2141
  call void @llvm.lifetime.start.p0(ptr nonnull %56), !noalias !33486
  store i8 3, ptr %1641, align 1, !noalias !33486
  store i64 %2144, ptr %1642, align 8, !noalias !33486
  store i64 %2143, ptr %1643, align 8, !noalias !33486
  store i8 0, ptr %56, align 8, !noalias !33486
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %71, ptr noundef nonnull align 8 %1590, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %56)
          to label %2148 unwind label %1768, !noalias !33341

2148:                                             ; preds = %2147
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !33486
  br label %2155

2149:                                             ; preds = %2155, %2146, %2132, %2126, %2077, %2073, %2066, %2060, %2057, %2036, %2014, %1986, %1979
  %2150 = phi ptr [ %1967, %2073 ], [ %2127, %2126 ], [ %1967, %2014 ], [ %1967, %1979 ], [ %1967, %1986 ], [ %1967, %2146 ], [ %2127, %2132 ], [ %1967, %2155 ], [ %1967, %2036 ], [ %1967, %2057 ], [ %1967, %2066 ], [ %1967, %2060 ], [ %1967, %2077 ]
  %2151 = phi ptr [ %1971, %2073 ], [ %2128, %2126 ], [ %1971, %2014 ], [ %1971, %1979 ], [ %1971, %1986 ], [ %1971, %2146 ], [ %2128, %2132 ], [ %1971, %2155 ], [ %1971, %2036 ], [ %1971, %2057 ], [ %1971, %2066 ], [ %1971, %2060 ], [ %1971, %2077 ]
  %2152 = phi i64 [ %1970, %2073 ], [ %2129, %2126 ], [ %1970, %2014 ], [ %1970, %1979 ], [ %1970, %1986 ], [ %1970, %2146 ], [ %2129, %2132 ], [ %1970, %2155 ], [ %1970, %2036 ], [ %1970, %2057 ], [ %1970, %2066 ], [ %1970, %2060 ], [ %1970, %2077 ]
  %2153 = phi ptr [ %1969, %2073 ], [ %2130, %2126 ], [ %1969, %2014 ], [ %1969, %1979 ], [ %1969, %1986 ], [ %1969, %2146 ], [ %2130, %2132 ], [ %1969, %2155 ], [ %1969, %2036 ], [ %1969, %2057 ], [ %1969, %2066 ], [ %1969, %2060 ], [ %1969, %2077 ]
  %2154 = icmp eq ptr %1972, %1895
  br i1 %2154, label %.loopexit280, label %.preheader278

2155:                                             ; preds = %2148, %2140
  %2156 = load i8, ptr %71, align 8, !noalias !33333
  %2157 = icmp eq i8 %2156, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !33333
  br i1 %2157, label %2149, label %.loopexit279

.loopexit271:                                     ; preds = %2118, %2108, %2052, %2042, %2104, %2038
  %2158 = phi ptr [ %1971, %2104 ], [ %1971, %2038 ], [ %2045, %2052 ], [ %2045, %2042 ], [ %2111, %2108 ], [ %2111, %2118 ]
  store ptr %2158, ptr %1623, align 8, !noalias !33333
  br label %.loopexit279

.loopexit279:                                     ; preds = %2155, %2132, %2073, %.loopexit271, %2101, %2031
  %2159 = load i64, ptr %87, align 8, !noalias !33339
  %2160 = load ptr, ptr %1472, align 8, !noalias !33339
  %2161 = load i64, ptr %1473, align 8, !noalias !33339
  br label %2398

.loopexit276:                                     ; preds = %2189, %2179, %2176, %2172
  %2162 = phi ptr [ %1974, %2172 ], [ %2182, %2176 ], [ %2182, %2179 ], [ %2182, %2189 ]
  %2163 = phi i64 [ %1975, %2172 ], [ %1795, %2189 ], [ %2181, %2179 ], [ %2177, %2176 ]
  %2164 = phi ptr [ %1976, %2172 ], [ %2182, %2176 ], [ %2182, %2179 ], [ %2182, %2189 ]
  store ptr %2162, ptr %1623, align 8, !noalias !33333
  br label %2165

2165:                                             ; preds = %2283, %2258, %.loopexit276, %.loopexit280
  %2166 = phi ptr [ %2253, %2258 ], [ %2284, %2283 ], [ %1974, %.loopexit280 ], [ %2162, %.loopexit276 ]
  %2167 = phi i64 [ %2254, %2258 ], [ %2285, %2283 ], [ %1975, %.loopexit280 ], [ %2163, %.loopexit276 ]
  %2168 = phi ptr [ %2255, %2258 ], [ %2286, %2283 ], [ %1976, %.loopexit280 ], [ %2164, %.loopexit276 ]
  %2169 = icmp eq i64 %1797, 0
  br i1 %2169, label %.loopexit274, label %2170

2170:                                             ; preds = %2165
  %2171 = load ptr, ptr %1480, align 8, !alias.scope !33487, !noalias !33490, !nonnull !1708, !noundef !1708
  br label %2346

2172:                                             ; preds = %.loopexit280
  %2173 = icmp eq ptr %1976, %1722
  br i1 %2173, label %.loopexit276, label %2174

2174:                                             ; preds = %2172
  %2175 = add i64 %1795, -1
  br label %2179

2176:                                             ; preds = %2189
  %2177 = add i64 %2181, 1
  %2178 = icmp eq ptr %2182, %1722
  br i1 %2178, label %.loopexit276, label %2179

2179:                                             ; preds = %2176, %2174
  %2180 = phi ptr [ %2182, %2176 ], [ %1976, %2174 ]
  %2181 = phi i64 [ %2177, %2176 ], [ %1975, %2174 ]
  %2182 = getelementptr inbounds nuw i8, ptr %2180, i64 88
  %2183 = getelementptr inbounds nuw i8, ptr %2180, i64 8
  %2184 = load i64, ptr %2183, align 8, !noalias !33492
  %2185 = icmp eq i64 %2184, -1
  br i1 %2185, label %.loopexit276, label %2186

2186:                                             ; preds = %2179
  %2187 = getelementptr inbounds nuw i8, ptr %2180, i64 16
  %2188 = load i64, ptr %2180, align 8, !noalias !33492
  call void @llvm.lifetime.start.p0(ptr nonnull %55), !noalias !33495
  store i64 %2184, ptr %55, align 8, !noalias !33495
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1653, ptr noundef nonnull align 8 dereferenceable(72) %2187, i64 72, i1 false), !noalias !33341
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %687, i64 noundef %2188, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %55)
          to label %2189 unwind label %1766, !noalias !33341

2189:                                             ; preds = %2186
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !33495
  %2190 = icmp eq i64 %2181, %2175
  br i1 %2190, label %.loopexit276, label %2176

.loopexit287:                                     ; preds = %2206, %1954
  %2191 = icmp eq i64 %1786, %1888
  br i1 %2191, label %.loopexit285, label %.lr.ph

2192:                                             ; preds = %.lr.ph
  %2193 = icmp eq ptr %1893, %2195
  br i1 %2193, label %.loopexit285, label %.lr.ph

.lr.ph:                                           ; preds = %.loopexit287, %2192
  %2194 = phi ptr [ %2195, %2192 ], [ %1895, %.loopexit287 ]
  %2195 = getelementptr inbounds i8, ptr %2194, i64 -24
  %2196 = load i8, ptr %2195, align 8, !range !10848, !noalias !33498, !noundef !1708
  %2197 = icmp eq i8 %2196, 2
  br i1 %2197, label %2226, label %2192

.preheader286:                                    ; preds = %1954, %2206
  %2198 = phi ptr [ %2199, %2206 ], [ %1893, %1954 ]
  %2199 = getelementptr inbounds nuw i8, ptr %2198, i64 24
  %2200 = load i8, ptr %2198, align 8, !range !10848, !noalias !33341, !noundef !1708
  %2201 = icmp eq i8 %2200, 0
  br i1 %2201, label %2202, label %2206

2202:                                             ; preds = %.preheader286
  %2203 = getelementptr inbounds nuw i8, ptr %2198, i64 1
  %2204 = load i8, ptr %2203, align 1, !range !1905, !noalias !33341, !noundef !1708
  %2205 = icmp eq i8 %2204, -1
  br i1 %2205, label %2206, label %2208

2206:                                             ; preds = %2217, %2211, %2208, %2202, %.preheader286
  %2207 = icmp eq ptr %2199, %1895
  br i1 %2207, label %.loopexit287, label %.preheader286

2208:                                             ; preds = %2202
  %2209 = load ptr, ptr %1633, align 8, !alias.scope !33340, !noalias !33341, !noundef !1708
  %2210 = icmp eq ptr %2209, null
  br i1 %2210, label %2206, label %2211

2211:                                             ; preds = %2208
  %2212 = load i32, ptr %1634, align 4, !alias.scope !33340, !noalias !33341, !noundef !1708
  %2213 = getelementptr i8, ptr %2209, i64 56
  %2214 = load i64, ptr %2213, align 8, !noalias !33341, !noundef !1708
  %2215 = zext i32 %2212 to i64
  %2216 = icmp ugt i64 %2214, %2215
  br i1 %2216, label %2217, label %2206

2217:                                             ; preds = %2211
  %2218 = getelementptr i8, ptr %2209, i64 48
  %2219 = load ptr, ptr %2218, align 8, !noalias !33341, !nonnull !1708, !noundef !1708
  %2220 = getelementptr inbounds nuw i8, ptr %2198, i64 8
  %2221 = load i64, ptr %2220, align 8, !noalias !33341, !noundef !1708
  %2222 = zext nneg i8 %2204 to i64
  %2223 = getelementptr inbounds nuw [136 x i8], ptr %2219, i64 %2215
  %2224 = getelementptr inbounds nuw [8 x i8], ptr %2223, i64 %2222
  %2225 = atomicrmw add ptr %2224, i64 %2221 monotonic, align 8, !noalias !33341
  br label %2206

2226:                                             ; preds = %.lr.ph
  %2227 = getelementptr i8, ptr %2194, i64 -16
  %2228 = load i64, ptr %2227, align 8, !noalias !33498
  %2229 = icmp ult i64 %1789, %2228
  br i1 %2229, label %2230, label %.loopexit285

2230:                                             ; preds = %2226
  %2231 = icmp eq ptr %1788, %1722
  br i1 %2231, label %.loopexit283, label %2232

2232:                                             ; preds = %2230
  %2233 = add i64 %2228, -1
  br label %2237

2234:                                             ; preds = %2247
  %2235 = add i64 %2239, 1
  %2236 = icmp eq ptr %2240, %1722
  br i1 %2236, label %.loopexit283, label %2237

2237:                                             ; preds = %2234, %2232
  %2238 = phi ptr [ %2240, %2234 ], [ %1788, %2232 ]
  %2239 = phi i64 [ %2235, %2234 ], [ %1789, %2232 ]
  %2240 = getelementptr inbounds nuw i8, ptr %2238, i64 88
  %2241 = getelementptr inbounds nuw i8, ptr %2238, i64 8
  %2242 = load i64, ptr %2241, align 8, !noalias !33501
  %2243 = icmp eq i64 %2242, -1
  br i1 %2243, label %.loopexit283, label %2244

2244:                                             ; preds = %2237
  %2245 = getelementptr inbounds nuw i8, ptr %2238, i64 16
  %2246 = load i64, ptr %2238, align 8, !noalias !33501
  call void @llvm.lifetime.start.p0(ptr nonnull %54), !noalias !33504
  store i64 %2242, ptr %54, align 8, !noalias !33504
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1654, ptr noundef nonnull align 8 dereferenceable(72) %2245, i64 72, i1 false), !noalias !33341
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %687, i64 noundef %2246, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %54)
          to label %2247 unwind label %1764, !noalias !33341

2247:                                             ; preds = %2244
  call void @llvm.lifetime.end.p0(ptr nonnull %54), !noalias !33504
  %2248 = icmp eq i64 %2239, %2233
  br i1 %2248, label %.loopexit283, label %2234

.loopexit283:                                     ; preds = %2247, %2237, %2234, %2230
  %2249 = phi ptr [ %1785, %2230 ], [ %2240, %2234 ], [ %2240, %2237 ], [ %2240, %2247 ]
  %2250 = phi i64 [ %1789, %2230 ], [ %2228, %2247 ], [ %2239, %2237 ], [ %2235, %2234 ]
  %2251 = phi ptr [ %1788, %2230 ], [ %2240, %2234 ], [ %2240, %2237 ], [ %2240, %2247 ]
  store ptr %2249, ptr %1623, align 8, !noalias !33333
  br label %.loopexit285

.loopexit285:                                     ; preds = %2192, %.loopexit287, %.loopexit283, %2226
  %2252 = phi i1 [ false, %.loopexit283 ], [ false, %2226 ], [ true, %.loopexit287 ], [ true, %2192 ]
  %2253 = phi ptr [ %2249, %.loopexit283 ], [ %1785, %2226 ], [ %1785, %.loopexit287 ], [ %1785, %2192 ]
  %2254 = phi i64 [ %2250, %.loopexit283 ], [ %1789, %2226 ], [ %1789, %.loopexit287 ], [ %1789, %2192 ]
  %2255 = phi ptr [ %2251, %.loopexit283 ], [ %1788, %2226 ], [ %1788, %.loopexit287 ], [ %1788, %2192 ]
  %2256 = icmp eq i64 %1921, 0
  %2257 = or i1 %1592, %2256
  br i1 %2257, label %2258, label %2259

2258:                                             ; preds = %2275, %.loopexit285
  br i1 %1595, label %2277, label %2165

2259:                                             ; preds = %.loopexit285
  call void @llvm.lifetime.start.p0(ptr nonnull %53), !noalias !33507
  %2260 = load atomic i64, ptr %1629 monotonic, align 8, !noalias !33514
  br label %2261

2261:                                             ; preds = %2261, %2259
  %2262 = phi i64 [ %2260, %2259 ], [ %2266, %2261 ]
  %2263 = call i64 @llvm.uadd.sat.i64(i64 %2262, i64 %1921)
  %2264 = cmpxchg weak ptr %1629, i64 %2262, i64 %2263 monotonic monotonic, align 8, !noalias !33514
  %2265 = extractvalue { i64, i1 } %2264, 1
  %2266 = extractvalue { i64, i1 } %2264, 0
  br i1 %2265, label %2267, label %2261

2267:                                             ; preds = %2261
  %2268 = call i64 @llvm.uadd.sat.i64(i64 %2266, i64 %1921)
  %2269 = load i64, ptr %1590, align 8, !noalias !33514
  %2270 = icmp ugt i64 %2268, %2269
  br i1 %2270, label %2271, label %2275

2271:                                             ; preds = %2267
  call void @llvm.lifetime.start.p0(ptr nonnull %52), !noalias !33517
  store i8 0, ptr %1655, align 1, !noalias !33517
  store i64 %2269, ptr %1656, align 8, !noalias !33517
  store i64 %2268, ptr %1657, align 8, !noalias !33517
  store i8 0, ptr %52, align 8, !noalias !33517
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %53, ptr noundef nonnull align 8 %1590, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %52)
          to label %2272 unwind label %1762, !noalias !33341

2272:                                             ; preds = %2271
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !33517
  %2273 = load i8, ptr %53, align 8, !noalias !33507
  %2274 = icmp eq i8 %2273, -1
  br i1 %2274, label %2275, label %2276

2275:                                             ; preds = %2272, %2267
  call void @llvm.lifetime.end.p0(ptr nonnull %53), !noalias !33507
  br label %2258

2276:                                             ; preds = %2272
  call void @llvm.lifetime.end.p0(ptr nonnull %53), !noalias !33507
  br i1 %1595, label %2320, label %2342

2277:                                             ; preds = %2258
  call void @llvm.lifetime.start.p0(ptr nonnull %76), !noalias !33333
  %2278 = load i64, ptr %1593, align 8, !noalias !33341
  %2279 = icmp eq i64 %2278, -1
  br i1 %2279, label %2305, label %2287

.loopexit281:                                     ; preds = %2340, %2330, %2327, %2323
  %2280 = phi ptr [ %2253, %2323 ], [ %2333, %2327 ], [ %2333, %2330 ], [ %2333, %2340 ]
  %2281 = phi i64 [ %2254, %2323 ], [ %1795, %2340 ], [ %2332, %2330 ], [ %2328, %2327 ]
  %2282 = phi ptr [ %2255, %2323 ], [ %2333, %2327 ], [ %2333, %2330 ], [ %2333, %2340 ]
  store ptr %2280, ptr %1623, align 8, !noalias !33333
  br label %2283

2283:                                             ; preds = %2320, %.loopexit281
  %2284 = phi ptr [ %2253, %2320 ], [ %2280, %.loopexit281 ]
  %2285 = phi i64 [ %2254, %2320 ], [ %2281, %.loopexit281 ]
  %2286 = phi ptr [ %2255, %2320 ], [ %2282, %.loopexit281 ]
  br i1 %2321, label %2342, label %2165

2287:                                             ; preds = %2277
  %2288 = load atomic i32, ptr %1636 acquire, align 8, !noalias !33518
  %2289 = icmp eq i32 %2288, 0
  br i1 %2289, label %2302, label %2290

2290:                                             ; preds = %2287
  %2291 = load atomic i64, ptr %1640 monotonic, align 8, !noalias !33518
  br label %2292

2292:                                             ; preds = %2292, %2290
  %2293 = phi i64 [ %2291, %2290 ], [ %2297, %2292 ]
  %2294 = call i64 @llvm.uadd.sat.i64(i64 %2293, i64 %1922)
  %2295 = cmpxchg weak ptr %1640, i64 %2293, i64 %2294 monotonic monotonic, align 8, !noalias !33518
  %2296 = extractvalue { i64, i1 } %2295, 1
  %2297 = extractvalue { i64, i1 } %2295, 0
  br i1 %2296, label %2298, label %2292

2298:                                             ; preds = %2292
  %2299 = call i64 @llvm.uadd.sat.i64(i64 %2297, i64 %1922)
  %2300 = load i64, ptr %1593, align 8, !noalias !33518
  %2301 = icmp ugt i64 %2299, %2300
  br i1 %2301, label %2303, label %2305

2302:                                             ; preds = %2287
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %76, ptr noundef nonnull align 8 dereferenceable(24) %1637, i64 24, i1 false), !noalias !33341
  br label %2306

2303:                                             ; preds = %2298
  call void @llvm.lifetime.start.p0(ptr nonnull %47), !noalias !33521
  store i8 3, ptr %1667, align 1, !noalias !33521
  store i64 %2300, ptr %1668, align 8, !noalias !33521
  store i64 %2299, ptr %1669, align 8, !noalias !33521
  store i8 0, ptr %47, align 8, !noalias !33521
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %76, ptr noundef nonnull align 8 %1590, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %47)
          to label %2304 unwind label %1774, !noalias !33299

2304:                                             ; preds = %2303
  call void @llvm.lifetime.end.p0(ptr nonnull %47), !noalias !33521
  br label %2306

2305:                                             ; preds = %2306, %2298, %2277
  call void @llvm.lifetime.end.p0(ptr nonnull %76), !noalias !33333
  br i1 %2252, label %2310, label %2312

2306:                                             ; preds = %2304, %2302
  %2307 = load i8, ptr %76, align 8, !noalias !33333
  %2308 = icmp eq i8 %2307, -1
  br i1 %2308, label %2305, label %2309

2309:                                             ; preds = %2306
  call void @llvm.lifetime.end.p0(ptr nonnull %76), !noalias !33333
  br label %2320

2310:                                             ; preds = %2313, %2305
  %2311 = icmp eq i64 %1923, 0
  br i1 %2311, label %2320, label %2316

2312:                                             ; preds = %2305
  call void @llvm.lifetime.start.p0(ptr nonnull %75), !noalias !33333
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %75, ptr noundef nonnull align 16 dereferenceable(1248) %5)
          to label %2313 unwind label %1774, !noalias !33341

2313:                                             ; preds = %2312
  %2314 = load i8, ptr %75, align 8, !range !1906, !noalias !33333, !noundef !1708
  %2315 = icmp eq i8 %2314, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %75), !noalias !33333
  br i1 %2315, label %2310, label %2320

2316:                                             ; preds = %2310
  call void @llvm.lifetime.start.p0(ptr nonnull %74), !noalias !33333
; invoke <purrdf_sparql_eval::governor::GovernorState>::admit_transient
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::admit_transient(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %74, ptr noundef nonnull align 8 %1590, i8 noundef 3, i64 noundef %1923)
          to label %2317 unwind label %1774, !noalias !33341

2317:                                             ; preds = %2316
  %2318 = load i8, ptr %74, align 8, !range !1906, !noalias !33333, !noundef !1708
  %2319 = icmp ne i8 %2318, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %74), !noalias !33333
  br label %2320

2320:                                             ; preds = %2317, %2313, %2310, %2309, %2276
  %2321 = phi i1 [ false, %2310 ], [ %2319, %2317 ], [ true, %2276 ], [ true, %2309 ], [ true, %2313 ]
  %2322 = icmp ult i64 %2254, %1795
  br i1 %2322, label %2323, label %2283

2323:                                             ; preds = %2320
  %2324 = icmp eq ptr %2255, %1722
  br i1 %2324, label %.loopexit281, label %2325

2325:                                             ; preds = %2323
  %2326 = add i64 %1795, -1
  br label %2330

2327:                                             ; preds = %2340
  %2328 = add i64 %2332, 1
  %2329 = icmp eq ptr %2333, %1722
  br i1 %2329, label %.loopexit281, label %2330

2330:                                             ; preds = %2327, %2325
  %2331 = phi ptr [ %2333, %2327 ], [ %2255, %2325 ]
  %2332 = phi i64 [ %2328, %2327 ], [ %2254, %2325 ]
  %2333 = getelementptr inbounds nuw i8, ptr %2331, i64 88
  %2334 = getelementptr inbounds nuw i8, ptr %2331, i64 8
  %2335 = load i64, ptr %2334, align 8, !noalias !33522
  %2336 = icmp eq i64 %2335, -1
  br i1 %2336, label %.loopexit281, label %2337

2337:                                             ; preds = %2330
  %2338 = getelementptr inbounds nuw i8, ptr %2331, i64 16
  %2339 = load i64, ptr %2331, align 8, !noalias !33522
  call void @llvm.lifetime.start.p0(ptr nonnull %51), !noalias !33525
  store i64 %2335, ptr %51, align 8, !noalias !33525
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1658, ptr noundef nonnull align 8 dereferenceable(72) %2338, i64 72, i1 false), !noalias !33341
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %687, i64 noundef %2339, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %51)
          to label %2340 unwind label %1760, !noalias !33341

2340:                                             ; preds = %2337
  call void @llvm.lifetime.end.p0(ptr nonnull %51), !noalias !33525
  %2341 = icmp eq i64 %2332, %2326
  br i1 %2341, label %.loopexit281, label %2327

2342:                                             ; preds = %2283, %2276
  %2343 = load i64, ptr %87, align 8, !noalias !33339
  %2344 = load ptr, ptr %1472, align 8, !noalias !33339
  %2345 = load i64, ptr %1473, align 8, !noalias !33339
  br label %2398

2346:                                             ; preds = %2388, %2170
  %2347 = phi i64 [ %1797, %2170 ], [ %2349, %2388 ]
  %2348 = phi ptr [ %1784, %2170 ], [ %2354, %2388 ]
  %2349 = add i64 %2347, -1
  %2350 = icmp eq ptr %2348, %2171
  br i1 %2350, label %.loopexit274, label %2353

.loopexit274:                                     ; preds = %2388, %2346, %2165
  %2351 = phi ptr [ %1784, %2165 ], [ %2354, %2388 ], [ %2348, %2346 ]
  store ptr %2351, ptr %1479, align 8, !noalias !33333
  %2352 = icmp eq ptr %1790, %1728
  br i1 %2352, label %.loopexit292, label %1783

2353:                                             ; preds = %2346
  %2354 = getelementptr inbounds nuw i8, ptr %2348, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %69), !noalias !33333
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1659, ptr noundef nonnull align 8 dereferenceable(40) %2348, i64 40, i1 false), !noalias !33341
  call void @llvm.lifetime.start.p0(ptr nonnull %70), !noalias !33333
  store ptr %5, ptr %69, align 8, !noalias !33333
  call void @llvm.experimental.noalias.scope.decl(metadata !33528)
  call void @llvm.experimental.noalias.scope.decl(metadata !33531)
  %2355 = load i64, ptr %1659, align 8, !alias.scope !33531, !noalias !33533, !noundef !1708
  %2356 = icmp eq i64 %2355, 0
  br i1 %2356, label %2357, label %2359

2357:                                             ; preds = %2353
  %2358 = load ptr, ptr %1661, align 8, !alias.scope !33535, !noalias !33536, !nonnull !1708, !align !1818, !noundef !1708
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %70, ptr noalias nofree noundef align 8 dereferenceable(184) %687, ptr noundef nonnull align 8 %2358, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %1662)
          to label %2360 unwind label %1758, !noalias !33341

2359:                                             ; preds = %2353
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1660, ptr noundef nonnull align 8 dereferenceable(40) %2348, i64 40, i1 false), !noalias !33341
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !33333
  br label %2371

2360:                                             ; preds = %2357
  %2361 = load i64, ptr %70, align 16, !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !33333
  %2362 = icmp eq i64 %2361, -1
  br i1 %2362, label %2371, label %2363

2363:                                             ; preds = %2360
  store ptr %2354, ptr %1479, align 8, !noalias !33333
  %2364 = load i64, ptr %1660, align 8, !noalias !33333
  %2365 = load ptr, ptr %1663, align 16, !noalias !33333
  %2366 = load i64, ptr %1664, align 8, !noalias !33333
  %2367 = load i8, ptr %1670, align 16, !noalias !33333
  %2368 = load i56, ptr %1671, align 1, !noalias !33333
  %2369 = load i64, ptr %1672, align 8, !noalias !33333
  %2370 = getelementptr inbounds nuw i8, ptr %70, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(48) %98, ptr noundef nonnull align 16 dereferenceable(48) %2370, i64 48, i1 false), !noalias !33339
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !33333
  br label %2398

2371:                                             ; preds = %2360, %2359
  %2372 = load i64, ptr %1660, align 8, !noalias !33333
  %2373 = load ptr, ptr %1663, align 16, !noalias !33333
  %2374 = load i64, ptr %1664, align 8, !noalias !33333
  %2375 = load i8, ptr %1670, align 16, !noalias !33333
  %2376 = load i56, ptr %1671, align 1, !noalias !33333
  %2377 = load i64, ptr %1672, align 8, !noalias !33333
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !33333
  call void @llvm.experimental.noalias.scope.decl(metadata !33537)
  %2378 = load i64, ptr %1473, align 8, !alias.scope !33537, !noalias !33540, !noundef !1708
  %2379 = load i64, ptr %87, align 8, !range !1817, !alias.scope !33537, !noalias !33540, !noundef !1708
  %2380 = icmp eq i64 %2378, %2379
  br i1 %2380, label %2381, label %2388

2381:                                             ; preds = %2371
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %87)
          to label %2388 unwind label %2382, !noalias !33542

2382:                                             ; preds = %2381
  %2383 = landingpad { ptr, i32 }
          cleanup
  store ptr %2354, ptr %1479, align 8, !noalias !33333
  %2384 = icmp ugt i64 %2372, 5
  br i1 %2384, label %2385, label %1778

2385:                                             ; preds = %2382
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2373) ]
  %2386 = shl i64 %2372, 3
  %2387 = add i64 %2386, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2373, i64 noundef %2387, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33543
  br label %1778

2388:                                             ; preds = %2381, %2371
  %2389 = load ptr, ptr %1472, align 8, !alias.scope !33537, !noalias !33540, !nonnull !1708, !noundef !1708
  %2390 = getelementptr inbounds nuw [40 x i8], ptr %2389, i64 %2378
  store i64 %2372, ptr %2390, align 8, !noalias !33546
  %2391 = getelementptr inbounds nuw i8, ptr %2390, i64 8
  store ptr %2373, ptr %2391, align 8, !noalias !33546
  %2392 = getelementptr inbounds nuw i8, ptr %2390, i64 16
  store i64 %2374, ptr %2392, align 8, !noalias !33341
  %2393 = getelementptr inbounds nuw i8, ptr %2390, i64 24
  store i8 %2375, ptr %2393, align 8, !noalias !33341
  %2394 = getelementptr inbounds nuw i8, ptr %2390, i64 25
  store i56 %2376, ptr %2394, align 1, !noalias !33341
  %2395 = getelementptr inbounds nuw i8, ptr %2390, i64 32
  store i64 %2377, ptr %2395, align 8, !noalias !33341
  %2396 = add i64 %2378, 1
  store i64 %2396, ptr %1473, align 8, !alias.scope !33537, !noalias !33540
  %2397 = icmp eq i64 %2349, 0
  br i1 %2397, label %.loopexit274, label %2346

2398:                                             ; preds = %2363, %2342, %.loopexit279, %1869
  %2399 = phi i56 [ %2368, %2363 ], [ undef, %.loopexit279 ], [ undef, %2342 ], [ undef, %1869 ]
  %2400 = phi i64 [ %2369, %2363 ], [ undef, %.loopexit279 ], [ undef, %2342 ], [ undef, %1869 ]
  %2401 = phi i8 [ %2367, %2363 ], [ 1, %.loopexit279 ], [ 1, %2342 ], [ 0, %1869 ]
  %2402 = phi i64 [ %2366, %2363 ], [ %2161, %.loopexit279 ], [ %2345, %2342 ], [ %1872, %1869 ]
  %2403 = phi ptr [ %2365, %2363 ], [ %2160, %.loopexit279 ], [ %2344, %2342 ], [ %1871, %1869 ]
  %2404 = phi i64 [ %2364, %2363 ], [ %2159, %.loopexit279 ], [ %2343, %2342 ], [ %1870, %1869 ]
  %2405 = phi i64 [ %2361, %2363 ], [ -1, %.loopexit279 ], [ -1, %2342 ], [ -1, %1869 ]
  %2406 = phi i8 [ 1, %2363 ], [ 0, %.loopexit279 ], [ 0, %2342 ], [ 0, %1869 ]
  %2407 = icmp eq i64 %1724, 0
  br i1 %2407, label %2411, label %2408

2408:                                             ; preds = %2398
  %2409 = shl nuw i64 %1724, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1723, i64 noundef %2409, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33547
  br label %2411

2410:                                             ; preds = %1890
  unreachable

2411:                                             ; preds = %2408, %2398
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %77)
          to label %2412 unwind label %1814, !noalias !33341

2412:                                             ; preds = %2411
  call void @llvm.lifetime.end.p0(ptr nonnull %77), !noalias !33333
  %2413 = icmp eq i64 %1716, 0
  br i1 %2413, label %2416, label %2414

2414:                                             ; preds = %2412
  %2415 = mul nuw i64 %1716, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1717) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1717, i64 noundef %2415, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33341
  br label %2416

2416:                                             ; preds = %2414, %2412
  call void @llvm.experimental.noalias.scope.decl(metadata !33550)
  %2417 = load ptr, ptr %1665, align 8, !alias.scope !33550, !noalias !33333, !noundef !1708
  %2418 = icmp eq ptr %2417, null
  br i1 %2418, label %2423, label %2419

2419:                                             ; preds = %2416
  %2420 = atomicrmw sub ptr %2417, i64 1 release, align 8, !noalias !33553
  %2421 = icmp eq i64 %2420, 1
  br i1 %2421, label %2422, label %2423

2422:                                             ; preds = %2419
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1665) #87, !noalias !33341
  br label %2423

2423:                                             ; preds = %2422, %2419, %2416
  call void @llvm.experimental.noalias.scope.decl(metadata !33558)
  %2424 = load ptr, ptr %1666, align 8, !alias.scope !33558, !noalias !33333, !noundef !1708
  %2425 = icmp eq ptr %2424, null
  br i1 %2425, label %2430, label %2426

2426:                                             ; preds = %2423
  %2427 = atomicrmw sub ptr %2424, i64 1 release, align 8, !noalias !33561
  %2428 = icmp eq i64 %2427, 1
  br i1 %2428, label %2429, label %2430

2429:                                             ; preds = %2426
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1666) #87, !noalias !33341
  br label %2430

2430:                                             ; preds = %2429, %2426, %2423
  call void @llvm.lifetime.end.p0(ptr nonnull %78), !noalias !33333
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %79)
          to label %2431 unwind label %1689, !noalias !33341

2431:                                             ; preds = %2430
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !33333
  %2432 = atomicrmw sub ptr %1589, i64 1 release, align 8, !noalias !33566
  %2433 = icmp eq i64 %2432, 1
  br i1 %2433, label %2434, label %2435

2434:                                             ; preds = %2431
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %80) #87
          to label %2435 unwind label %1518, !noalias !33341

2435:                                             ; preds = %2434, %2431
  call void @llvm.lifetime.end.p0(ptr nonnull %80), !noalias !33333
  br label %1576

2436:                                             ; preds = %2438, %1576
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !33333
  %2437 = trunc nuw i8 %1584 to i1
  br i1 %2437, label %2439, label %1534

2438:                                             ; preds = %1576
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %84)
          to label %2436 unwind label %1530, !noalias !33341

2439:                                             ; preds = %2436
  call void @llvm.experimental.noalias.scope.decl(metadata !33571)
  %2440 = load ptr, ptr %1472, align 8, !alias.scope !33571, !noalias !33333, !nonnull !1708, !noundef !1708
  %2441 = load i64, ptr %1473, align 8, !alias.scope !33571, !noalias !33333, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33574)
  %2442 = icmp eq i64 %2441, 0
  br i1 %2442, label %.loopexit269, label %.preheader268

.preheader268:                                    ; preds = %2439
  %2443 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %2444

2444:                                             ; preds = %.preheader268, %2482
  %2445 = phi i64 [ %2447, %2482 ], [ 0, %.preheader268 ]
  %2446 = getelementptr inbounds nuw [40 x i8], ptr %2440, i64 %2445
  %2447 = add nuw nsw i64 %2445, 1
  %2448 = load i64, ptr %2446, align 8, !range !1940, !alias.scope !33577, !noalias !33580, !noundef !1708
  %2449 = icmp ugt i64 %2448, 5
  br i1 %2449, label %2450, label %2482

2450:                                             ; preds = %2444
  %2451 = getelementptr i8, ptr %2446, i64 8
  %2452 = load ptr, ptr %2451, align 8, !alias.scope !33574, !noalias !33580, !nonnull !1708, !noundef !1708
  %2453 = shl i64 %2448, 3
  %2454 = add i64 %2453, -8
  %2455 = load i64, ptr %267, align 8, !noalias !33581, !noundef !1708
  %2456 = call i64 @llvm.umin.i64(i64 %2454, i64 9223372036854775807)
  %2457 = call i64 @llvm.ssub.sat.i64(i64 %2455, i64 %2456)
  store i64 %2457, ptr %267, align 8, !noalias !33581
  %2458 = load i64, ptr %2443, align 8, !noalias !33581, !noundef !1708
  %2459 = icmp slt i64 %2457, %2458
  br i1 %2459, label %2460, label %.preheader2193

2460:                                             ; preds = %2450
  store i64 %2457, ptr %2443, align 8, !noalias !33581
  br label %.preheader2193

.preheader2193:                                   ; preds = %2460, %2450
  br label %2461

2461:                                             ; preds = %.preheader2193, %2464
  %2462 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33581
  %2463 = icmp slt i64 %2462, 0
  br i1 %2463, label %2464, label %__rustc::__rust_dealloc (.exit246)

2464:                                             ; preds = %2461
  %2465 = add nsw i64 %2462, 1
  %2466 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2462, i64 %2465 acq_rel acquire, align 8, !noalias !33581
  %2467 = extractvalue { i64, i1 } %2466, 1
  br i1 %2467, label %2468, label %2461

2468:                                             ; preds = %2464
  %2469 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2456 monotonic, align 8, !noalias !33581
  %2470 = call i64 @llvm.ssub.sat.i64(i64 %2469, i64 %2456)
  %2471 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33581
  br label %2472

2472:                                             ; preds = %2475, %2468
  %2473 = phi i64 [ %2471, %2468 ], [ %2478, %2475 ]
  %2474 = icmp slt i64 %2470, %2473
  br i1 %2474, label %2475, label %2479

2475:                                             ; preds = %2472
  %2476 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2473, i64 %2470 monotonic monotonic, align 8, !noalias !33581
  %2477 = extractvalue { i64, i1 } %2476, 1
  %2478 = extractvalue { i64, i1 } %2476, 0
  br i1 %2477, label %2479, label %2472

2479:                                             ; preds = %2475, %2472
  %2480 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33581
  br label %__rustc::__rust_dealloc (.exit246)

__rustc::__rust_dealloc (.exit246): ; preds = %2461, %2479
  %2481 = icmp ne i64 %2454, 0
  call void @llvm.assume(i1 %2481), !noalias !33581
  call void @free(ptr noundef nonnull %2452) #88, !noalias !33581
  br label %2482

2482:                                             ; preds = %__rustc::__rust_dealloc (.exit246), %2444
  %2483 = icmp eq i64 %2447, %2441
  br i1 %2483, label %.loopexit269, label %2444

.loopexit269:                                     ; preds = %2482, %2439
  %2484 = load i64, ptr %87, align 8, !alias.scope !33571, !noalias !33333
  %2485 = icmp eq i64 %2484, 0
  br i1 %2485, label %1534, label %2486

2486:                                             ; preds = %.loopexit269
  %2487 = mul nuw i64 %2484, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2440, i64 noundef %2487, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33580
  br label %1534

2488:                                             ; preds = %1826, %1823, %1820
  call void @llvm.experimental.noalias.scope.decl(metadata !33584)
  %2489 = load ptr, ptr %1666, align 8, !alias.scope !33584, !noalias !33333, !noundef !1708
  %2490 = icmp eq ptr %2489, null
  br i1 %2490, label %1707, label %2491

2491:                                             ; preds = %2488
  %2492 = atomicrmw sub ptr %2489, i64 1 release, align 8, !noalias !33587
  %2493 = icmp eq i64 %2492, 1
  br i1 %2493, label %2494, label %1707

2494:                                             ; preds = %2491
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1666) #87, !noalias !33341
  br label %1707

2495:                                             ; preds = %1688, %1682, %1518
  %2496 = phi { ptr, i32 } [ %1521, %1518 ], [ %1685, %1682 ], [ %1685, %1688 ]
  %2497 = phi i8 [ %1520, %1518 ], [ %1684, %1682 ], [ %1684, %1688 ]
  %2498 = phi i8 [ %1519, %1518 ], [ %1683, %1682 ], [ %1683, %1688 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %84) #89
          to label %1525 unwind label %1586, !noalias !33341

2499:                                             ; preds = %1525, %1504
  %2500 = phi { ptr, i32 } [ %1528, %1525 ], [ %1505, %1504 ]
  %2501 = phi i8 [ %1527, %1525 ], [ 1, %1504 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %87) #89, !noalias !33341
  br label %2502

2502:                                             ; preds = %2499, %1525
  %2503 = phi i8 [ %1527, %1525 ], [ %2501, %2499 ]
  %2504 = phi { ptr, i32 } [ %1528, %1525 ], [ %2500, %2499 ]
  %2505 = trunc nuw i8 %2503 to i1
  br i1 %2505, label %2540, label %3316

2506:                                             ; preds = %1463
  %2507 = landingpad { ptr, i32 }
          cleanup
  br label %2540

2508:                                             ; preds = %1534, %1522, %1463
  %2509 = phi i56 [ undef, %1522 ], [ %1577, %1534 ], [ %1467, %1463 ]
  %2510 = phi i64 [ undef, %1522 ], [ %1578, %1534 ], [ %1469, %1463 ]
  %2511 = phi i8 [ 2, %1522 ], [ %1579, %1534 ], [ %1465, %1463 ]
  %2512 = phi i64 [ %1516, %1522 ], [ %1580, %1534 ], [ %1462, %1463 ]
  %2513 = phi ptr [ %1524, %1522 ], [ %1581, %1534 ], [ %1460, %1463 ]
  %2514 = phi i64 [ %1523, %1522 ], [ %1582, %1534 ], [ %1458, %1463 ]
  %2515 = phi i64 [ -1, %1522 ], [ %1583, %1534 ], [ %1455, %1463 ]
  %2516 = icmp eq i64 %1397, 0
  br i1 %2516, label %._crit_edge2057, label %.lr.ph2056

2517:                                             ; preds = %.lr.ph2056
  %2518 = icmp eq i64 %2521, %1397
  br i1 %2518, label %._crit_edge2057, label %.lr.ph2056

.lr.ph2056:                                       ; preds = %2508, %2517
  %2519 = phi i64 [ %2521, %2517 ], [ 0, %2508 ]
  %2520 = getelementptr inbounds nuw [160 x i8], ptr %1398, i64 %2519
  %2521 = add nuw nsw i64 %2519, 1
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %2520)
          to label %2517 unwind label %2525, !noalias !33592

2522:                                             ; preds = %.lr.ph2059
  %2523 = add i64 %2528, 1
  %2524 = icmp eq i64 %2523, %1397
  br i1 %2524, label %._crit_edge2060, label %.lr.ph2059

2525:                                             ; preds = %.lr.ph2056
  %2526 = landingpad { ptr, i32 }
          cleanup
  %2527 = icmp eq i64 %2521, %1397
  br i1 %2527, label %._crit_edge2060, label %.lr.ph2059

.lr.ph2059:                                       ; preds = %2525, %2522
  %2528 = phi i64 [ %2523, %2522 ], [ %2521, %2525 ]
  %2529 = getelementptr inbounds nuw [160 x i8], ptr %1398, i64 %2528
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %2529) #89
          to label %2522 unwind label %2530, !noalias !33592

2530:                                             ; preds = %.lr.ph2059
  %2531 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33595
  unreachable

._crit_edge2060:                                  ; preds = %2522, %2525
  %2532 = icmp eq i64 %1418, 0
  br i1 %2532, label %3316, label %2533

2533:                                             ; preds = %._crit_edge2060
  %2534 = mul nuw i64 %1418, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1398, i64 noundef %2534, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33592
  br label %3316

._crit_edge2057:                                  ; preds = %2517, %2508
  %2535 = icmp eq i64 %1418, 0
  br i1 %2535, label %2542, label %2536

2536:                                             ; preds = %._crit_edge2057
  %2537 = mul nuw i64 %1418, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1398, i64 noundef %2537, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33592
  br label %2542

2538:                                             ; preds = %.loopexit296
  %2539 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %140) #89
          to label %2540 unwind label %1586

2540:                                             ; preds = %2538, %2506, %2502
  %2541 = phi { ptr, i32 } [ %2507, %2506 ], [ %2504, %2502 ], [ %2539, %2538 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %97) #89
          to label %3316 unwind label %1586, !noalias !33598

2542:                                             ; preds = %2536, %._crit_edge2057, %1534
  %2543 = phi i56 [ %2509, %._crit_edge2057 ], [ %2509, %2536 ], [ %1577, %1534 ]
  %2544 = phi i64 [ %2510, %._crit_edge2057 ], [ %2510, %2536 ], [ %1578, %1534 ]
  %2545 = phi i8 [ %2511, %._crit_edge2057 ], [ %2511, %2536 ], [ %1579, %1534 ]
  %2546 = phi i64 [ %2512, %._crit_edge2057 ], [ %2512, %2536 ], [ %1580, %1534 ]
  %2547 = phi ptr [ %2513, %._crit_edge2057 ], [ %2513, %2536 ], [ %1581, %1534 ]
  %2548 = phi i64 [ %2514, %._crit_edge2057 ], [ %2514, %2536 ], [ %1582, %1534 ]
  %2549 = phi i64 [ %2515, %._crit_edge2057 ], [ %2515, %2536 ], [ %1583, %1534 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %97), !noalias !33239
  %2550 = icmp eq i64 %2549, -1
  br i1 %2550, label %2552, label %2551

2551:                                             ; preds = %2542
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %126, ptr noundef nonnull align 1 dereferenceable(48) %98, i64 48, i1 false), !noalias !33599
  call void @llvm.lifetime.end.p0(ptr nonnull %98)
  call void @llvm.lifetime.end.p0(ptr nonnull %100), !noalias !33239
  call void @llvm.lifetime.end.p0(ptr nonnull %124)
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  br label %2806

2552:                                             ; preds = %2542, %1749
  %2553 = phi i64 [ %1739, %1749 ], [ %2548, %2542 ]
  %2554 = phi ptr [ %1740, %1749 ], [ %2547, %2542 ]
  %2555 = phi i64 [ %1741, %1749 ], [ %2546, %2542 ]
  %2556 = phi i8 [ 2, %1749 ], [ %2545, %2542 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %98)
  call void @llvm.lifetime.start.p0(ptr nonnull %96), !noalias !33239
  store i64 %2553, ptr %96, align 8, !noalias !33239
  %2557 = getelementptr inbounds nuw i8, ptr %96, i64 8
  store ptr %2554, ptr %2557, align 8, !noalias !33239
  %2558 = getelementptr inbounds nuw i8, ptr %96, i64 16
  store i64 %2555, ptr %2558, align 8, !noalias !33239
  call void @llvm.lifetime.start.p0(ptr nonnull %49)
  %2559 = load ptr, ptr %696, align 8, !alias.scope !33231, !noalias !33299, !noundef !1708
  %2560 = icmp eq ptr %2559, null
  br i1 %2560, label %2571, label %2561

2561:                                             ; preds = %2552
  %2562 = getelementptr inbounds nuw i8, ptr %2559, i64 296
  %2563 = load atomic i32, ptr %2562 acquire, align 4, !noalias !33600
  %2564 = icmp eq i32 %2563, 0
  br i1 %2564, label %2565, label %2569

2565:                                             ; preds = %2561
  %2566 = getelementptr inbounds nuw i8, ptr %2559, i64 272
  %2567 = load i8, ptr %2566, align 8, !noalias !33299
  %2568 = getelementptr inbounds nuw i8, ptr %2559, i64 273
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %49, ptr noundef nonnull align 1 dereferenceable(23) %2568, i64 23, i1 false), !noalias !33299
  br label %2569

2569:                                             ; preds = %2565, %2561
  %2570 = phi i8 [ %2567, %2565 ], [ -1, %2561 ]
  switch i8 %2556, label %2575 [
    i8 2, label %2592
    i8 0, label %2574
  ]

2571:                                             ; preds = %2552
  %2572 = icmp eq i8 %2556, 2
  %2573 = and i1 %2572, %1443
  br label %2592

2574:                                             ; preds = %2589, %2577, %2569
  br label %2592

2575:                                             ; preds = %2569
  %2576 = icmp eq i8 %2570, -1
  br i1 %2576, label %2592, label %2577

2577:                                             ; preds = %2575
  %2578 = load i8, ptr %196, align 8, !range !3634, !alias.scope !33231, !noalias !33299, !noundef !1708
  %2579 = icmp eq i8 %2578, 2
  br i1 %2579, label %2580, label %2574

2580:                                             ; preds = %2577
  %2581 = getelementptr inbounds nuw i8, ptr %5, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !33603)
  %2582 = load ptr, ptr %2581, align 8, !alias.scope !33606, !noalias !33607, !nonnull !1708, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !33609
  store i8 %2570, ptr %48, align 8, !noalias !33613
  %2583 = getelementptr inbounds nuw i8, ptr %48, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %2583, ptr noundef nonnull align 1 dereferenceable(23) %49, i64 23, i1 false), !noalias !33239
  %2584 = getelementptr inbounds nuw i8, ptr %2582, i64 40
  %2585 = load atomic i32, ptr %2584 acquire, align 4, !noalias !33614
  %2586 = icmp eq i32 %2585, 0
  br i1 %2586, label %2589, label %2587, !prof !1974

2587:                                             ; preds = %2580
  %2588 = getelementptr inbounds nuw i8, ptr %2582, i64 16
; invoke <std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !>
  invoke fastcc void @<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.13412714042204560522)(ptr noundef nonnull align 8 %2588, ptr noundef nonnull align 8 %48)
          to label %2589 unwind label %2590, !noalias !33299

2589:                                             ; preds = %2587, %2580
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !33609
  br label %2574

2590:                                             ; preds = %2587
  %2591 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %96) #89, !noalias !33299
  br label %3316

2592:                                             ; preds = %2575, %2574, %2571, %2569
  %2593 = phi i8 [ -1, %2571 ], [ -1, %2575 ], [ %2570, %2574 ], [ %2570, %2569 ]
  %2594 = phi i1 [ %2573, %2571 ], [ false, %2575 ], [ false, %2574 ], [ %1443, %2569 ]
  %2595 = icmp eq i8 %2593, -1
  %2596 = select i1 %2594, i1 %2595, i1 false
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  call void @llvm.lifetime.end.p0(ptr nonnull %96), !noalias !33239
  %2597 = zext i1 %2596 to i8
  call void @llvm.lifetime.end.p0(ptr nonnull %100), !noalias !33239
  call void @llvm.lifetime.end.p0(ptr nonnull %124)
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  br label %2819

2598:                                             ; preds = %2623, %2621
  %2599 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33299
  unreachable

2600:                                             ; preds = %1259, %.loopexit303
  call void @llvm.lifetime.end.p0(ptr nonnull %95), !noalias !33245
  br label %2601

2601:                                             ; preds = %2600, %1157
  %2602 = phi i64 [ %1216, %2600 ], [ %1159, %1157 ]
  %2603 = phi i64 [ %1217, %2600 ], [ %1161, %1157 ]
  %2604 = phi i64 [ %1210, %2600 ], [ %1149, %1157 ]
  %2605 = phi i64 [ %1213, %2600 ], [ %1152, %1157 ]
  %2606 = phi ptr [ %1214, %2600 ], [ %1154, %1157 ]
  %2607 = phi i64 [ %1215, %2600 ], [ %1156, %1157 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %101), !noalias !33239
  %2608 = trunc i64 %2602 to i8
  %2609 = lshr i64 %2602, 8
  %2610 = trunc nuw i64 %2609 to i56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %126, ptr noundef nonnull align 8 dereferenceable(48) %102, i64 48, i1 false), !noalias !33599
  br label %2611

2611:                                             ; preds = %2601, %1206
  %2612 = phi i56 [ 0, %1206 ], [ %2610, %2601 ]
  %2613 = phi i8 [ 0, %1206 ], [ %2608, %2601 ]
  %2614 = phi i64 [ undef, %1206 ], [ %2603, %2601 ]
  %2615 = phi i64 [ %1200, %1206 ], [ %2607, %2601 ]
  %2616 = phi ptr [ %1208, %1206 ], [ %2606, %2601 ]
  %2617 = phi i64 [ %1207, %1206 ], [ %2605, %2601 ]
  %2618 = phi i64 [ -1, %1206 ], [ %2604, %2601 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %102)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(208) %124)
          to label %2804 unwind label %3314

2619:                                             ; preds = %1265
  %2620 = landingpad { ptr, i32 }
          cleanup
  br label %2623

2621:                                             ; preds = %1263, %1202, %1163
  %2622 = phi { ptr, i32 } [ %1164, %1163 ], [ %1264, %1263 ], [ %1203, %1202 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(208) %124) #89
          to label %3316 unwind label %2598, !noalias !33615

2623:                                             ; preds = %2619, %1369
  %2624 = phi { ptr, i32 } [ %2620, %2619 ], [ %1370, %1369 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %125) #89
          to label %3316 unwind label %2598, !noalias !33616

2625:                                             ; preds = %1134
  call void @llvm.lifetime.start.p0(ptr nonnull %130)
  call void @llvm.lifetime.start.p0(ptr nonnull %129)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %129, ptr noundef nonnull align 8 dereferenceable(24) %140, i64 24, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %128)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %128, ptr noundef nonnull align 8 dereferenceable(208) %134, i64 208, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %127)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %127, ptr noundef nonnull align 8 dereferenceable(24) %152, i64 24, i1 false)
  store i64 0, ptr %152, align 8
  store ptr inttoptr (i64 8 to ptr), ptr %685, align 8
  store i64 0, ptr %218, align 8
  %2626 = getelementptr inbounds nuw i8, ptr %149, i64 194
  %2627 = load i8, ptr %2626, align 2, !range !3634, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33617)
  call void @llvm.experimental.noalias.scope.decl(metadata !33620)
  %2628 = icmp eq i8 %2627, 0
  br i1 %2628, label %2630, label %2629, !prof !10563

2629:                                             ; preds = %2625
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.a12f493ba210922c94e5446ac885c35e.142, ptr noundef nonnull inttoptr (i64 83 to ptr), ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.144) #90
          to label %2796 unwind label %2799, !noalias !33622

2630:                                             ; preds = %2625
  call void @llvm.lifetime.start.p0(ptr nonnull %46)
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !33626
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %45, ptr noundef nonnull readonly align 8 dereferenceable(24) %127, i64 24, i1 false), !noalias !33627
  call void @llvm.experimental.noalias.scope.decl(metadata !33628)
  call void @llvm.experimental.noalias.scope.decl(metadata !33631)
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !33633
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !33633
  %2631 = getelementptr inbounds nuw i8, ptr %129, i64 16
  %2632 = load i64, ptr %2631, align 8, !alias.scope !33636, !noalias !33637, !noundef !1708
  %2633 = icmp ult i64 %2632, 230584300921369396
  call void @llvm.assume(i1 %2633)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %43, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %45, i64 noundef %2632)
          to label %2634 unwind label %2776, !noalias !33638

2634:                                             ; preds = %2630
  %2635 = load i64, ptr %43, align 16, !range !2530, !noalias !33633, !noundef !1708
  %2636 = icmp eq i64 %2635, -1
  %2637 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %2638 = load i64, ptr %2637, align 8, !noalias !33633
  %2639 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %2640 = load ptr, ptr %2639, align 16, !noalias !33633
  %2641 = getelementptr inbounds nuw i8, ptr %43, i64 24
  %2642 = load i64, ptr %2641, align 8, !noalias !33633
  br i1 %2636, label %2649, label %2643

2643:                                             ; preds = %2634
  %2644 = getelementptr inbounds nuw i8, ptr %43, i64 32
  %2645 = load i64, ptr %2644, align 16, !noalias !33639
  %2646 = getelementptr inbounds nuw i8, ptr %43, i64 40
  %2647 = load i64, ptr %2646, align 8, !noalias !33639
  %2648 = getelementptr inbounds nuw i8, ptr %43, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %46, ptr noundef nonnull align 16 dereferenceable(48) %2648, i64 48, i1 false), !noalias !33639
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !33633
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !33633
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %140)
          to label %2781 unwind label %2778

2649:                                             ; preds = %2634
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !33633
  store i64 %2638, ptr %44, align 8, !noalias !33633
  %2650 = getelementptr inbounds nuw i8, ptr %44, i64 8
  store ptr %2640, ptr %2650, align 8, !noalias !33633
  %2651 = getelementptr inbounds nuw i8, ptr %44, i64 16
  store i64 %2642, ptr %2651, align 8, !noalias !33633
  %2652 = getelementptr inbounds nuw i8, ptr %129, i64 8
  %2653 = load ptr, ptr %2652, align 8, !alias.scope !33636, !noalias !33637, !nonnull !1708, !noundef !1708
  %2654 = load i64, ptr %129, align 8, !range !1817, !alias.scope !33636, !noalias !33637, !noundef !1708
  %2655 = mul nuw nsw i64 %2632, 40
  %2656 = getelementptr inbounds nuw i8, ptr %2653, i64 %2655
  call void @llvm.lifetime.start.p0(ptr nonnull %42), !noalias !33633
  store ptr %2653, ptr %42, align 8, !noalias !33633
  %2657 = getelementptr inbounds nuw i8, ptr %42, i64 8
  %2658 = getelementptr inbounds nuw i8, ptr %42, i64 16
  store i64 %2654, ptr %2658, align 8, !noalias !33633
  %2659 = getelementptr inbounds nuw i8, ptr %42, i64 24
  store ptr %2656, ptr %2659, align 8, !noalias !33633
  %2660 = icmp eq i64 %2632, 0
  br i1 %2660, label %.loopexit267, label %2661

2661:                                             ; preds = %2649
  %2662 = getelementptr inbounds nuw i8, ptr %40, i64 8
  %2663 = getelementptr inbounds nuw i8, ptr %41, i64 8
  %2664 = getelementptr inbounds nuw i8, ptr %5, i64 664
  %2665 = getelementptr inbounds nuw i8, ptr %40, i64 16
  %2666 = getelementptr inbounds nuw i8, ptr %41, i64 16
  %2667 = getelementptr inbounds nuw i8, ptr %41, i64 24
  %2668 = getelementptr inbounds nuw i8, ptr %41, i64 32
  %2669 = getelementptr inbounds nuw i8, ptr %41, i64 40
  br label %2674

2670:                                             ; preds = %2681
  %2671 = landingpad { ptr, i32 }
          cleanup
  store ptr %2678, ptr %2657, align 8, !noalias !33633
  br label %2672

2672:                                             ; preds = %2716, %2713, %2670
  %2673 = phi { ptr, i32 } [ %2671, %2670 ], [ %2714, %2716 ], [ %2714, %2713 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42) #89
          to label %2686 unwind label %2774, !noalias !33640

2674:                                             ; preds = %2719, %2661
  %2675 = phi ptr [ %2640, %2661 ], [ %2720, %2719 ]
  %2676 = phi i64 [ %2642, %2661 ], [ %2725, %2719 ]
  %2677 = phi ptr [ %2653, %2661 ], [ %2678, %2719 ]
  %2678 = getelementptr inbounds nuw i8, ptr %2677, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !33633
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %2662, ptr noundef nonnull align 8 dereferenceable(40) %2677, i64 40, i1 false), !noalias !33640
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !33633
  store ptr %5, ptr %40, align 8, !noalias !33633
  call void @llvm.experimental.noalias.scope.decl(metadata !33641)
  call void @llvm.experimental.noalias.scope.decl(metadata !33644)
  %2679 = load i64, ptr %2662, align 8, !alias.scope !33644, !noalias !33646, !noundef !1708
  %2680 = icmp eq i64 %2679, 0
  br i1 %2680, label %2681, label %2683

2681:                                             ; preds = %2674
  %2682 = load ptr, ptr %2664, align 8, !alias.scope !33648, !noalias !33649, !nonnull !1708, !align !1818, !noundef !1708
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %41, ptr noalias nofree noundef align 8 dereferenceable(184) %687, ptr noundef nonnull align 8 %2682, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %2665)
          to label %2693 unwind label %2670, !noalias !33640

2683:                                             ; preds = %2674
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %2663, ptr noundef nonnull align 8 dereferenceable(40) %2677, i64 40, i1 false), !noalias !33640
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !33633
  br label %2703

.loopexit267:                                     ; preds = %2719, %2649
  %2684 = phi i64 [ %2642, %2649 ], [ %2725, %2719 ]
  %2685 = phi ptr [ %2653, %2649 ], [ %2656, %2719 ]
  store ptr %2685, ptr %2657, align 8, !noalias !33633
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42)
          to label %2690 unwind label %2688, !noalias !33640

2686:                                             ; preds = %2688, %2672
  %2687 = phi { ptr, i32 } [ %2689, %2688 ], [ %2673, %2672 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %44) #89, !noalias !33640
  br label %2797

2688:                                             ; preds = %2696, %.loopexit267
  %2689 = landingpad { ptr, i32 }
          cleanup
  br label %2686

2690:                                             ; preds = %.loopexit267
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !33633
  %2691 = load i64, ptr %44, align 8, !noalias !33639
  %2692 = load ptr, ptr %2650, align 8, !noalias !33639
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !33633
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !33626
  br label %2789

2693:                                             ; preds = %2681
  %2694 = load i64, ptr %41, align 16, !noalias !33633
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !33633
  %2695 = icmp eq i64 %2694, -1
  br i1 %2695, label %2703, label %2696

2696:                                             ; preds = %2693
  store ptr %2678, ptr %2657, align 8, !noalias !33633
  %2697 = load i64, ptr %2663, align 8, !noalias !33633
  %2698 = load ptr, ptr %2666, align 16, !noalias !33633
  %2699 = load i64, ptr %2667, align 8, !noalias !33633
  %2700 = load i64, ptr %2668, align 16, !noalias !33633
  %2701 = load i64, ptr %2669, align 8, !noalias !33633
  %2702 = getelementptr inbounds nuw i8, ptr %41, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %46, ptr noundef nonnull align 16 dereferenceable(48) %2702, i64 48, i1 false), !noalias !33639
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !33633
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42)
          to label %2727 unwind label %2688, !noalias !33640

2703:                                             ; preds = %2693, %2683
  %2704 = load i64, ptr %2663, align 8, !noalias !33633
  %2705 = load ptr, ptr %2666, align 16, !noalias !33633
  %2706 = load <2 x i64>, ptr %2667, align 8, !noalias !33633
  %2707 = load i64, ptr %2669, align 8, !noalias !33633
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !33633
  call void @llvm.experimental.noalias.scope.decl(metadata !33650)
  %2708 = load i64, ptr %44, align 8, !range !1817, !alias.scope !33650, !noalias !33653, !noundef !1708
  %2709 = icmp eq i64 %2676, %2708
  br i1 %2709, label %2710, label %2719

2710:                                             ; preds = %2703
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %44)
          to label %2711 unwind label %2713, !noalias !33655

2711:                                             ; preds = %2710
  %2712 = load ptr, ptr %2650, align 8, !alias.scope !33650, !noalias !33653
  br label %2719

2713:                                             ; preds = %2710
  %2714 = landingpad { ptr, i32 }
          cleanup
  store ptr %2678, ptr %2657, align 8, !noalias !33633
  %2715 = icmp ugt i64 %2704, 5
  br i1 %2715, label %2716, label %2672

2716:                                             ; preds = %2713
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2705) ]
  %2717 = shl i64 %2704, 3
  %2718 = add i64 %2717, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2705, i64 noundef %2718, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33656
  br label %2672

2719:                                             ; preds = %2711, %2703
  %2720 = phi ptr [ %2712, %2711 ], [ %2675, %2703 ]
  %2721 = getelementptr inbounds nuw [40 x i8], ptr %2720, i64 %2676
  store i64 %2704, ptr %2721, align 8, !noalias !33659
  %2722 = getelementptr inbounds nuw i8, ptr %2721, i64 8
  store ptr %2705, ptr %2722, align 8, !noalias !33659
  %2723 = getelementptr inbounds nuw i8, ptr %2721, i64 16
  store <2 x i64> %2706, ptr %2723, align 8, !noalias !33640
  %2724 = getelementptr inbounds nuw i8, ptr %2721, i64 32
  store i64 %2707, ptr %2724, align 8, !noalias !33640
  %2725 = add i64 %2676, 1
  store i64 %2725, ptr %2651, align 8, !alias.scope !33650, !noalias !33653
  %2726 = icmp eq ptr %2678, %2656
  br i1 %2726, label %.loopexit267, label %2674

2727:                                             ; preds = %2696
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !33633
  call void @llvm.experimental.noalias.scope.decl(metadata !33660)
  call void @llvm.experimental.noalias.scope.decl(metadata !33663)
  %2728 = icmp eq i64 %2676, 0
  br i1 %2728, label %.loopexit266, label %.preheader265

.preheader265:                                    ; preds = %2727
  %2729 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %2730

2730:                                             ; preds = %.preheader265, %2768
  %2731 = phi i64 [ %2733, %2768 ], [ 0, %.preheader265 ]
  %2732 = getelementptr inbounds nuw [40 x i8], ptr %2675, i64 %2731
  %2733 = add nuw nsw i64 %2731, 1
  %2734 = load i64, ptr %2732, align 8, !range !1940, !alias.scope !33666, !noalias !33669, !noundef !1708
  %2735 = icmp ugt i64 %2734, 5
  br i1 %2735, label %2736, label %2768

2736:                                             ; preds = %2730
  %2737 = getelementptr i8, ptr %2732, i64 8
  %2738 = load ptr, ptr %2737, align 8, !alias.scope !33663, !noalias !33669, !nonnull !1708, !noundef !1708
  %2739 = shl i64 %2734, 3
  %2740 = add i64 %2739, -8
  %2741 = load i64, ptr %267, align 8, !noalias !33670, !noundef !1708
  %2742 = call i64 @llvm.umin.i64(i64 %2740, i64 9223372036854775807)
  %2743 = call i64 @llvm.ssub.sat.i64(i64 %2741, i64 %2742)
  store i64 %2743, ptr %267, align 8, !noalias !33670
  %2744 = load i64, ptr %2729, align 8, !noalias !33670, !noundef !1708
  %2745 = icmp slt i64 %2743, %2744
  br i1 %2745, label %2746, label %.preheader2172

2746:                                             ; preds = %2736
  store i64 %2743, ptr %2729, align 8, !noalias !33670
  br label %.preheader2172

.preheader2172:                                   ; preds = %2746, %2736
  br label %2747

2747:                                             ; preds = %.preheader2172, %2750
  %2748 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33670
  %2749 = icmp slt i64 %2748, 0
  br i1 %2749, label %2750, label %__rustc::__rust_dealloc (.exit247)

2750:                                             ; preds = %2747
  %2751 = add nsw i64 %2748, 1
  %2752 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %2748, i64 %2751 acq_rel acquire, align 8, !noalias !33670
  %2753 = extractvalue { i64, i1 } %2752, 1
  br i1 %2753, label %2754, label %2747

2754:                                             ; preds = %2750
  %2755 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2742 monotonic, align 8, !noalias !33670
  %2756 = call i64 @llvm.ssub.sat.i64(i64 %2755, i64 %2742)
  %2757 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33670
  br label %2758

2758:                                             ; preds = %2761, %2754
  %2759 = phi i64 [ %2757, %2754 ], [ %2764, %2761 ]
  %2760 = icmp slt i64 %2756, %2759
  br i1 %2760, label %2761, label %2765

2761:                                             ; preds = %2758
  %2762 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2759, i64 %2756 monotonic monotonic, align 8, !noalias !33670
  %2763 = extractvalue { i64, i1 } %2762, 1
  %2764 = extractvalue { i64, i1 } %2762, 0
  br i1 %2763, label %2765, label %2758

2765:                                             ; preds = %2761, %2758
  %2766 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33670
  br label %__rustc::__rust_dealloc (.exit247)

__rustc::__rust_dealloc (.exit247): ; preds = %2747, %2765
  %2767 = icmp ne i64 %2740, 0
  call void @llvm.assume(i1 %2767), !noalias !33670
  call void @free(ptr noundef nonnull %2738) #88, !noalias !33670
  br label %2768

2768:                                             ; preds = %__rustc::__rust_dealloc (.exit247), %2730
  %2769 = icmp eq i64 %2733, %2676
  br i1 %2769, label %.loopexit266, label %2730

.loopexit266:                                     ; preds = %2768, %2727
  %2770 = load i64, ptr %44, align 8, !alias.scope !33660, !noalias !33633
  %2771 = icmp eq i64 %2770, 0
  br i1 %2771, label %2780, label %2772

2772:                                             ; preds = %.loopexit266
  %2773 = mul nuw i64 %2770, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2675, i64 noundef %2773, i64 noundef range(i64 1, -9223372036854775807) 8) #88, !noalias !33669
  br label %2780

2774:                                             ; preds = %2776, %2672
  %2775 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33640
  unreachable

2776:                                             ; preds = %2630
  %2777 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %140) #89
          to label %2797 unwind label %2774

2778:                                             ; preds = %2643
  %2779 = landingpad { ptr, i32 }
          cleanup
  br label %2797

2780:                                             ; preds = %2772, %.loopexit266
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !33633
  br label %2781

2781:                                             ; preds = %2780, %2643
  %2782 = phi i64 [ %2700, %2780 ], [ %2645, %2643 ]
  %2783 = phi i64 [ %2701, %2780 ], [ %2647, %2643 ]
  %2784 = phi i64 [ %2694, %2780 ], [ %2635, %2643 ]
  %2785 = phi i64 [ %2697, %2780 ], [ %2638, %2643 ]
  %2786 = phi ptr [ %2698, %2780 ], [ %2640, %2643 ]
  %2787 = phi i64 [ %2699, %2780 ], [ %2642, %2643 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !33626
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %130, ptr noundef nonnull align 8 dereferenceable(48) %46, i64 48, i1 false), !noalias !33673
  br label %2789

2788:                                             ; preds = %2797
  br i1 %2628, label %3316, label %2803

2789:                                             ; preds = %2781, %2690
  %2790 = phi i64 [ undef, %2690 ], [ %2783, %2781 ]
  %2791 = phi i64 [ 0, %2690 ], [ %2782, %2781 ]
  %2792 = phi i64 [ %2684, %2690 ], [ %2787, %2781 ]
  %2793 = phi ptr [ %2692, %2690 ], [ %2786, %2781 ]
  %2794 = phi i64 [ %2691, %2690 ], [ %2785, %2781 ]
  %2795 = phi i64 [ -1, %2690 ], [ %2784, %2781 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %46)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(208) %128)
          to label %2873 unwind label %3314

2796:                                             ; preds = %2629
  unreachable

2797:                                             ; preds = %2799, %2778, %2776, %2686
  %2798 = phi { ptr, i32 } [ %2779, %2778 ], [ %2800, %2799 ], [ %2687, %2686 ], [ %2777, %2776 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(208) %128) #89
          to label %2788 unwind label %2801, !noalias !33674

2799:                                             ; preds = %2629
  %2800 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %127) #89, !noalias !33675
  br label %2797

2801:                                             ; preds = %2803, %2797
  %2802 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33622
  unreachable

2803:                                             ; preds = %2788
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %129) #89
          to label %3316 unwind label %2801, !noalias !33676

2804:                                             ; preds = %2611
  call void @llvm.lifetime.end.p0(ptr nonnull %124)
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  %2805 = icmp eq i64 %2618, -1
  br i1 %2805, label %2819, label %2806

2806:                                             ; preds = %2804, %2551
  %2807 = phi i64 [ %2549, %2551 ], [ %2618, %2804 ]
  %2808 = phi i64 [ %2548, %2551 ], [ %2617, %2804 ]
  %2809 = phi ptr [ %2547, %2551 ], [ %2616, %2804 ]
  %2810 = phi i64 [ %2546, %2551 ], [ %2615, %2804 ]
  %2811 = phi i64 [ %2544, %2551 ], [ %2614, %2804 ]
  %2812 = phi i8 [ %2545, %2551 ], [ %2613, %2804 ]
  %2813 = phi i56 [ %2543, %2551 ], [ %2612, %2804 ]
  %2814 = zext i56 %2813 to i64
  %2815 = shl nuw i64 %2814, 8
  %2816 = zext i8 %2812 to i64
  %2817 = or disjoint i64 %2815, %2816
  %2818 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %2818, ptr noundef nonnull align 16 dereferenceable(48) %126, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %126)
  br label %2866

2819:                                             ; preds = %2804, %2592
  %2820 = phi i64 [ %2553, %2592 ], [ %2617, %2804 ]
  %2821 = phi ptr [ %2554, %2592 ], [ %2616, %2804 ]
  %2822 = phi i64 [ %2555, %2592 ], [ %2615, %2804 ]
  %2823 = phi i64 [ %1442, %2592 ], [ %2614, %2804 ]
  %2824 = phi i8 [ %2597, %2592 ], [ %2613, %2804 ]
  %2825 = zext i8 %2824 to i64
  call void @llvm.lifetime.end.p0(ptr nonnull %126)
  br label %2826

2826:                                             ; preds = %2877, %2819
  %2827 = phi i64 [ %2794, %2877 ], [ %2820, %2819 ]
  %2828 = phi ptr [ %2793, %2877 ], [ %2821, %2819 ]
  %2829 = phi i64 [ %2792, %2877 ], [ %2822, %2819 ]
  %2830 = phi i64 [ %2791, %2877 ], [ %2825, %2819 ]
  %2831 = phi i64 [ %2790, %2877 ], [ %2823, %2819 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %131)
  store i64 %2827, ptr %131, align 8
  %2832 = getelementptr inbounds nuw i8, ptr %131, i64 8
  store ptr %2828, ptr %2832, align 8
  %2833 = getelementptr inbounds nuw i8, ptr %131, i64 16
  store i64 %2829, ptr %2833, align 8
  %2834 = load i64, ptr %135, align 8
  %2835 = getelementptr inbounds nuw i8, ptr %135, i64 8
  %2836 = load i64, ptr %2835, align 8
  %2837 = getelementptr inbounds nuw i8, ptr %135, i64 16
  %2838 = load i64, ptr %2837, align 8
  %2839 = getelementptr inbounds nuw i8, ptr %135, i64 24
  %2840 = load i64, ptr %2839, align 8
  %2841 = icmp ugt i64 %2834, 2
  %2842 = select i1 %2841, i64 %2838, i64 %2834
  %2843 = add i64 %2842, -1
  %2844 = select i1 %2841, i64 %2834, i64 1
  %2845 = select i1 %2841, i64 1, i64 %2838
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !33677
  store i64 %2844, ptr %39, align 8, !noalias !33681
  %2846 = getelementptr inbounds nuw i8, ptr %39, i64 8
  store i64 %2836, ptr %2846, align 8, !noalias !33681
  %2847 = getelementptr inbounds nuw i8, ptr %39, i64 16
  store i64 %2845, ptr %2847, align 8, !noalias !33681
  %2848 = getelementptr inbounds nuw i8, ptr %39, i64 24
  store i64 %2840, ptr %2848, align 8, !noalias !33681
  %2849 = getelementptr inbounds nuw i8, ptr %39, i64 32
  store i64 0, ptr %2849, align 8, !noalias !33677
  %2850 = getelementptr inbounds nuw i8, ptr %39, i64 40
  store i64 %2843, ptr %2850, align 8, !noalias !33677
  %2851 = icmp eq i64 %2843, 0
  br i1 %2851, label %.loopexit264, label %2852

2852:                                             ; preds = %2826
  %2853 = inttoptr i64 %2836 to ptr
  %2854 = select i1 %2841, ptr %2853, ptr %2846
  %2855 = getelementptr inbounds nuw i8, ptr %5, i64 640
  br label %2858

2856:                                             ; preds = %2858
  %2857 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %39) #89
          to label %2878 unwind label %2864, !noalias !33682

2858:                                             ; preds = %2862, %2852
  %2859 = phi i64 [ 0, %2852 ], [ %2860, %2862 ]
  %2860 = add nuw i64 %2859, 1
  store i64 %2860, ptr %2849, align 8, !alias.scope !33683, !noalias !33686
  %2861 = getelementptr inbounds nuw [24 x i8], ptr %2854, i64 %2859
  call void @llvm.lifetime.start.p0(ptr nonnull %38), !noalias !33677
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %38, ptr noundef nonnull align 8 dereferenceable(24) %2861, i64 24, i1 false), !noalias !33682
; invoke <purrdf_sparql_eval::witness::RelationWitness>::merge
  invoke void @<purrdf_sparql_eval::witness::RelationWitness>::merge(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %2855, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %38)
          to label %2862 unwind label %2856, !noalias !33682

.loopexit264:                                     ; preds = %2862, %2826
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %39)
          to label %2882 unwind label %2880

2862:                                             ; preds = %2858
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !33677
  %2863 = icmp eq i64 %2860, %2843
  br i1 %2863, label %.loopexit264, label %2858

2864:                                             ; preds = %2856
  %2865 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !33682
  unreachable

2866:                                             ; preds = %2875, %2806
  %.sink1702 = phi i64 [ %2795, %2875 ], [ %2807, %2806 ]
  %.sink1700 = phi i64 [ %2794, %2875 ], [ %2808, %2806 ]
  %.sink1698 = phi ptr [ %2793, %2875 ], [ %2809, %2806 ]
  %.sink1696 = phi i64 [ %2792, %2875 ], [ %2810, %2806 ]
  %.sink1694 = phi i64 [ %2791, %2875 ], [ %2817, %2806 ]
  %.sink = phi i64 [ %2790, %2875 ], [ %2811, %2806 ]
  %2867 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %.sink1702, ptr %2867, align 16
  %2868 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %.sink1700, ptr %2868, align 8
  %2869 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store ptr %.sink1698, ptr %2869, align 16
  %2870 = getelementptr inbounds nuw i8, ptr %0, i64 40
  store i64 %.sink1696, ptr %2870, align 8
  %2871 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %.sink1694, ptr %2871, align 16
  %2872 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i64 %.sink, ptr %2872, align 8
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %134)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %135)
          to label %3048 unwind label %1132

2873:                                             ; preds = %2789
  call void @llvm.lifetime.end.p0(ptr nonnull %127)
  call void @llvm.lifetime.end.p0(ptr nonnull %128)
  call void @llvm.lifetime.end.p0(ptr nonnull %129)
  %2874 = icmp eq i64 %2795, -1
  br i1 %2874, label %2877, label %2875

2875:                                             ; preds = %2873
  %2876 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %2876, ptr noundef nonnull align 16 dereferenceable(48) %130, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %130)
  br label %2866

2877:                                             ; preds = %2873
  call void @llvm.lifetime.end.p0(ptr nonnull %130)
  br label %2826

2878:                                             ; preds = %2912, %2880, %2856
  %2879 = phi { ptr, i32 } [ %2857, %2856 ], [ %2881, %2880 ], [ %2913, %2912 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %131) #89
  br label %707

2880:                                             ; preds = %.loopexit263, %3046, %2897, %2894, %.loopexit264
  %2881 = landingpad { ptr, i32 }
          cleanup
  br label %2878

2882:                                             ; preds = %.loopexit264
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !33677
  %2883 = trunc i64 %2830 to i1
  br i1 %2883, label %2884, label %2887

2884:                                             ; preds = %2882
  %2885 = load i64, ptr %218, align 8, !noundef !1708
  %2886 = icmp ugt i64 %2831, %2885
  br i1 %2886, label %2897, label %2894, !prof !1803

2887:                                             ; preds = %3061, %2882
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %143, ptr noundef nonnull align 8 dereferenceable(24) %131, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %131)
  call void @llvm.lifetime.end.p0(ptr nonnull %134)
  call void @llvm.lifetime.end.p0(ptr nonnull %135)
  call void @llvm.lifetime.end.p0(ptr nonnull %140)
  call void @llvm.experimental.noalias.scope.decl(metadata !33688)
  %2888 = load ptr, ptr %141, align 8, !alias.scope !33688, !noundef !1708
  %2889 = icmp eq ptr %2888, null
  br i1 %2889, label %3062, label %2890

2890:                                             ; preds = %2887
  %2891 = atomicrmw sub ptr %2888, i64 1 release, align 8, !noalias !33691
  %2892 = icmp eq i64 %2891, 1
  br i1 %2892, label %2893, label %3062

2893:                                             ; preds = %2890
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %141) #87
          to label %3062 unwind label %355, !inline_history !1744

2894:                                             ; preds = %2884
  %2895 = load ptr, ptr %685, align 8, !nonnull !1708, !noundef !1708
  %2896 = sub nuw i64 %2885, %2831
  call void @llvm.lifetime.start.p0(ptr nonnull %123)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %123, ptr noundef nonnull align 16 %5, i1 noundef zeroext false, i64 noundef %2896)
          to label %2898 unwind label %2880

2897:                                             ; preds = %2884
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %2831, i64 noundef %2885, i64 noundef %2885, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.390) #90
          to label %555 unwind label %2880

2898:                                             ; preds = %2894
  %2899 = getelementptr inbounds nuw [40 x i8], ptr %2895, i64 %2885
  %2900 = icmp samesign eq i64 %2831, %2885
  br i1 %2900, label %.loopexit263, label %2901

2901:                                             ; preds = %2898
  %2902 = getelementptr inbounds nuw [40 x i8], ptr %2895, i64 %2831
  %2903 = getelementptr inbounds nuw i8, ptr %121, i64 16
  %2904 = getelementptr inbounds nuw i8, ptr %121, i64 8
  %2905 = getelementptr inbounds nuw i8, ptr %36, i64 8
  %2906 = getelementptr inbounds nuw i8, ptr %120, i64 8
  br label %2907

2907:                                             ; preds = %3039, %2901
  %2908 = phi ptr [ %2828, %2901 ], [ %3040, %3039 ]
  %2909 = phi i64 [ %2829, %2901 ], [ %3044, %3039 ]
  %2910 = phi ptr [ %2902, %2901 ], [ %2911, %3039 ]
  %2911 = getelementptr inbounds nuw i8, ptr %2910, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %122)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %122, ptr noalias nofree noundef align 8 dereferenceable(200) %123, ptr noundef nonnull align 16 %5)
          to label %2916 unwind label %2914

2912:                                             ; preds = %3057, %3053, %3035, %3032, %2927, %2923, %2914
  %2913 = phi { ptr, i32 } [ %2924, %2923 ], [ %3033, %3035 ], [ %2915, %2914 ], [ %2924, %2927 ], [ %3033, %3032 ], [ %3054, %3053 ], [ %3054, %3057 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %123)
          to label %2878 unwind label %679

2914:                                             ; preds = %2907
  %2915 = landingpad { ptr, i32 }
          cleanup
  br label %2912

2916:                                             ; preds = %2907
  %2917 = load i8, ptr %122, align 8, !range !1906, !noundef !1708
  %2918 = icmp eq i8 %2917, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %122)
  br i1 %2918, label %2919, label %.loopexit263

2919:                                             ; preds = %2916
  call void @llvm.lifetime.start.p0(ptr nonnull %121)
  %2920 = load i64, ptr %146, align 8, !noundef !1708
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !33694
  store i64 1, ptr %37, align 8, !noalias !33694
  %2921 = icmp ugt i64 %2920, 4
  br i1 %2921, label %2922, label %2932, !prof !1803

2922:                                             ; preds = %2919
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %37, i64 noundef 0, i64 noundef %2920, i1 noundef zeroext false) #87
          to label %2932 unwind label %2923, !noalias !33694

2923:                                             ; preds = %2922
  %2924 = landingpad { ptr, i32 }
          cleanup
  %2925 = load i64, ptr %37, align 8, !range !1940, !alias.scope !33697, !noalias !33694, !noundef !1708
  %2926 = icmp ugt i64 %2925, 5
  br i1 %2926, label %2927, label %2912

2927:                                             ; preds = %2923
  %2928 = getelementptr inbounds nuw i8, ptr %37, i64 8
  %2929 = load ptr, ptr %2928, align 8, !noalias !33694, !nonnull !1708, !noundef !1708
  %2930 = shl i64 %2925, 3
  %2931 = add i64 %2930, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2929, i64 noundef %2931, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33700
  br label %2912

2932:                                             ; preds = %2922, %2919
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %121, ptr noundef nonnull align 8 dereferenceable(40) %37, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !33694
  %2933 = load i64, ptr %2910, align 8, !range !1940, !noundef !1708
  %2934 = add i64 %2933, -1
  %2935 = icmp ugt i64 %2934, 4
  %2936 = getelementptr inbounds nuw i8, ptr %2910, i64 8
  br i1 %2935, label %2937, label %2942

2937:                                             ; preds = %2932
  %2938 = load ptr, ptr %2936, align 8, !nonnull !1708, !noundef !1708
  %2939 = getelementptr inbounds nuw i8, ptr %2910, i64 16
  %2940 = load i64, ptr %2939, align 8, !noundef !1708
  %2941 = add i64 %2940, -1
  br label %2942

2942:                                             ; preds = %2937, %2932
  %2943 = phi i64 [ %2941, %2937 ], [ %2934, %2932 ]
  %2944 = phi ptr [ %2938, %2937 ], [ %2936, %2932 ]
  %2945 = load i64, ptr %121, align 8, !range !1940, !alias.scope !33703, !noalias !33708, !noundef !1708
  %2946 = add i64 %2945, -1
  %2947 = icmp ugt i64 %2946, 4
  %2948 = load i64, ptr %2903, align 8, !alias.scope !33703, !noalias !33708
  %2949 = add i64 %2948, -1
  %2950 = select i1 %2947, i64 %2949, i64 %2946
  %2951 = call i64 @llvm.umax.i64(i64 %2946, i64 4)
  %2952 = sub i64 %2951, %2950
  %2953 = icmp ult i64 %2952, %2943
  br i1 %2953, label %2954, label %2957, !prof !1803

2954:                                             ; preds = %2942
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %121, i64 noundef %2950, i64 noundef range(i64 0, 1152921504606846976) %2943, i1 noundef zeroext true) #87
          to label %2955 unwind label %3049

2955:                                             ; preds = %2954
  %2956 = load i64, ptr %121, align 8, !range !1940, !alias.scope !33710, !noalias !33708
  br label %2957

2957:                                             ; preds = %2955, %2942
  %2958 = phi i64 [ %2945, %2942 ], [ %2956, %2955 ]
  %2959 = icmp ugt i64 %2958, 5
  %2960 = load ptr, ptr %2904, align 8, !alias.scope !33710, !noalias !33708, !nonnull !1708
  %2961 = select i1 %2959, ptr %2960, ptr %2904
  %2962 = select i1 %2959, ptr %2903, ptr %121
  %2963 = load i64, ptr %2962, align 8, !alias.scope !33710, !noalias !33708, !noundef !1708
  %2964 = getelementptr [8 x i8], ptr %2961, i64 %2963
  %2965 = getelementptr i8, ptr %2964, i64 -8
  %2966 = shl nuw nsw i64 %2943, 3
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 4 %2965, ptr nonnull readonly align 4 %2944, i64 %2966, i1 false)
  %2967 = add i64 %2963, %2943
  store i64 %2967, ptr %2962, align 8, !alias.scope !33710, !noalias !33708
  %2968 = load i64, ptr %146, align 8, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33711)
  %2969 = load i64, ptr %121, align 8, !range !1940, !alias.scope !33711, !noundef !1708
  %2970 = add i64 %2969, -1
  %2971 = icmp ugt i64 %2970, 4
  %2972 = load i64, ptr %2903, align 8, !alias.scope !33711
  %2973 = add i64 %2972, -1
  %2974 = select i1 %2971, i64 %2973, i64 %2970
  %2975 = icmp ugt i64 %2968, %2974
  br i1 %2975, label %2984, label %2976

2976:                                             ; preds = %2957
  %2977 = icmp ugt i64 %2969, 5
  %2978 = select i1 %2977, i64 %2972, i64 %2969
  %2979 = add i64 %2978, -1
  %2980 = icmp ult i64 %2968, %2979
  br i1 %2980, label %2981, label %2987

2981:                                             ; preds = %2976
  %2982 = select i1 %2977, ptr %2903, ptr %121
  %2983 = add nuw i64 %2968, 1
  store i64 %2983, ptr %2982, align 8, !alias.scope !33714
  br label %2987

2984:                                             ; preds = %2957
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !33711
  %2985 = sub nuw i64 %2968, %2974
  store i32 2, ptr %36, align 8, !noalias !33711
  store i64 %2985, ptr %2905, align 8, !noalias !33711
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %121, ptr noalias nofree noundef align 8 captures(address) dereferenceable(16) %36)
          to label %2986 unwind label %3049

2986:                                             ; preds = %2984
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !33711
  br label %2987

2987:                                             ; preds = %2986, %2981, %2976
  call void @llvm.lifetime.start.p0(ptr nonnull %120)
  %2988 = load i64, ptr %121, align 8, !range !1940, !noundef !1708
  %2989 = add i64 %2988, -1
  %2990 = icmp ugt i64 %2989, 4
  %2991 = load ptr, ptr %2904, align 8, !nonnull !1708
  %2992 = load i64, ptr %2903, align 8
  %2993 = add i64 %2992, -1
  %2994 = select i1 %2990, i64 %2993, i64 %2989
  %2995 = select i1 %2990, ptr %2991, ptr %2904
  %2996 = load ptr, ptr %145, align 8, !nonnull !1708, !noundef !1708
  %2997 = getelementptr inbounds nuw i8, ptr %2996, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %120, ptr noalias nofree noundef align 8 dereferenceable(216) %144, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %2995, i64 noundef %2994, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %2997, ptr noalias nofree noundef align 16 dereferenceable(1248) %5)
          to label %2998 unwind label %3049

2998:                                             ; preds = %2987
  %2999 = load i64, ptr %120, align 16, !range !2530, !noundef !1708
  %3000 = icmp eq i64 %2999, -1
  %3001 = load <2 x i32>, ptr %2906, align 8
  br i1 %3000, label %3013, label %3002

3002:                                             ; preds = %2998
  %3003 = getelementptr inbounds nuw i8, ptr %120, i64 16
  %3004 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %3004, ptr noundef nonnull align 16 dereferenceable(80) %3003, i64 80, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %120)
  %3005 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %2999, ptr %3005, align 16
  %3006 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store <2 x i32> %3001, ptr %3006, align 8
  store i64 1, ptr %0, align 16
  %3007 = load i64, ptr %121, align 8, !range !1940, !alias.scope !24096, !noundef !1708
  %3008 = icmp ugt i64 %3007, 5
  br i1 %3008, label %3009, label %3046

3009:                                             ; preds = %3002
  %3010 = load ptr, ptr %2904, align 8, !nonnull !1708, !noundef !1708
  %3011 = shl i64 %3007, 3
  %3012 = add i64 %3011, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3010, i64 noundef %3012, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33717
  br label %3046

3013:                                             ; preds = %2998
  call void @llvm.lifetime.end.p0(ptr nonnull %120)
  %3014 = load i64, ptr %121, align 8, !range !1940, !noundef !1708
  %3015 = icmp ugt i64 %3014, 5
  %3016 = load i64, ptr %2903, align 8
  %3017 = select i1 %3015, i64 %3016, i64 %3014
  %3018 = add i64 %3017, -1
  %3019 = load i64, ptr %147, align 8, !noundef !1708
  %3020 = icmp ult i64 %3019, %3018
  br i1 %3020, label %3021, label %3038

3021:                                             ; preds = %3013
  %3022 = load ptr, ptr %2904, align 8, !nonnull !1708
  %3023 = select i1 %3015, ptr %3022, ptr %2904
  %3024 = getelementptr inbounds nuw [8 x i8], ptr %3023, i64 %3019
  store <2 x i32> %3001, ptr %3024, align 4
  call void @llvm.lifetime.start.p0(ptr nonnull %119)
  %3025 = load i64, ptr %121, align 8
  %3026 = load ptr, ptr %2904, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %119, ptr noundef nonnull align 8 dereferenceable(24) %2903, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !33720)
  %3027 = load i64, ptr %131, align 8, !range !1817, !alias.scope !33720, !noalias !33723, !noundef !1708
  %3028 = icmp eq i64 %2909, %3027
  br i1 %3028, label %3029, label %3039

3029:                                             ; preds = %3021
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %131)
          to label %3030 unwind label %3032, !noalias !33723

3030:                                             ; preds = %3029
  %3031 = load ptr, ptr %2832, align 8, !alias.scope !33720, !noalias !33723
  br label %3039

3032:                                             ; preds = %3029
  %3033 = landingpad { ptr, i32 }
          cleanup
  %3034 = icmp ugt i64 %3025, 5
  br i1 %3034, label %3035, label %2912

3035:                                             ; preds = %3032
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %3026) ]
  %3036 = shl i64 %3025, 3
  %3037 = add i64 %3036, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3026, i64 noundef %3037, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33725
  br label %2912

3038:                                             ; preds = %3013
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %3019, i64 noundef %3018, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.a12f493ba210922c94e5446ac885c35e.389) #90
          to label %555 unwind label %3051

3039:                                             ; preds = %3030, %3021
  %3040 = phi ptr [ %3031, %3030 ], [ %2908, %3021 ]
  %3041 = getelementptr inbounds nuw [40 x i8], ptr %3040, i64 %2909
  store i64 %3025, ptr %3041, align 8, !noalias !33720
  %3042 = getelementptr inbounds nuw i8, ptr %3041, i64 8
  store ptr %3026, ptr %3042, align 8, !noalias !33720
  %3043 = getelementptr inbounds nuw i8, ptr %3041, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3043, ptr noundef nonnull align 8 dereferenceable(24) %119, i64 24, i1 false), !noalias !33720
  %3044 = add i64 %2909, 1
  store i64 %3044, ptr %2833, align 8, !alias.scope !33720, !noalias !33723
  call void @llvm.lifetime.end.p0(ptr nonnull %119)
  call void @llvm.lifetime.end.p0(ptr nonnull %121)
  %3045 = icmp eq ptr %2911, %2899
  br i1 %3045, label %.loopexit263, label %2907

3046:                                             ; preds = %3009, %3002
  call void @llvm.lifetime.end.p0(ptr nonnull %121)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %123)
          to label %3047 unwind label %2880

3047:                                             ; preds = %3046
  call void @llvm.lifetime.end.p0(ptr nonnull %123)
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %131)
  call void @llvm.lifetime.end.p0(ptr nonnull %131)
  call void @llvm.lifetime.end.p0(ptr nonnull %134)
  br label %3048

3048:                                             ; preds = %3047, %2866
  call void @llvm.lifetime.end.p0(ptr nonnull %135)
  call void @llvm.lifetime.end.p0(ptr nonnull %140)
  br label %3307

3049:                                             ; preds = %2987, %2984, %2954
  %3050 = landingpad { ptr, i32 }
          cleanup
  br label %3053

3051:                                             ; preds = %3038
  %3052 = landingpad { ptr, i32 }
          cleanup
  br label %3053

3053:                                             ; preds = %3051, %3049
  %3054 = phi { ptr, i32 } [ %3050, %3049 ], [ %3052, %3051 ]
  %3055 = load i64, ptr %121, align 8, !range !1940, !alias.scope !24096, !noundef !1708
  %3056 = icmp ugt i64 %3055, 5
  br i1 %3056, label %3057, label %2912

3057:                                             ; preds = %3053
  %3058 = load ptr, ptr %2904, align 8, !nonnull !1708, !noundef !1708
  %3059 = shl i64 %3055, 3
  %3060 = add i64 %3059, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3058, i64 noundef %3060, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !33728
  br label %2912

.loopexit263:                                     ; preds = %3039, %2916, %2898
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %123)
          to label %3061 unwind label %2880

3061:                                             ; preds = %.loopexit263
  call void @llvm.lifetime.end.p0(ptr nonnull %123)
  br label %2887

3062:                                             ; preds = %2893, %2890, %2887
  call void @llvm.lifetime.end.p0(ptr nonnull %141)
  call void @llvm.lifetime.end.p0(ptr nonnull %142)
  br label %662

3063:                                             ; preds = %3127
  %3064 = landingpad { ptr, i32 }
          cleanup
  br label %351

3065:                                             ; preds = %662
  %3066 = getelementptr inbounds nuw i8, ptr %665, i64 16
  %3067 = load i8, ptr %3066, align 8, !noalias !33064
  %3068 = icmp eq i8 %3067, -1
  br i1 %3068, label %3075, label %3069

3069:                                             ; preds = %3065
  %3070 = getelementptr inbounds nuw i8, ptr %665, i64 17
  call void @llvm.lifetime.start.p0(ptr nonnull %111)
  store i8 %3067, ptr %111, align 8
  %3071 = getelementptr inbounds nuw i8, ptr %111, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %3071, ptr noundef nonnull align 1 dereferenceable(23) %3070, i64 23, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %110)
  %3072 = load ptr, ptr %145, align 8, !nonnull !1708, !noundef !1708
  %3073 = atomicrmw add ptr %3072, i64 1 monotonic, align 8
  %3074 = icmp slt i64 %3073, 0
  br i1 %3074, label %3130, label %3128

3075:                                             ; preds = %3065, %662
  call void @llvm.lifetime.start.p0(ptr nonnull %109)
  call void @llvm.lifetime.start.p0(ptr nonnull %108)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %108, ptr noundef nonnull align 8 dereferenceable(104) %157, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %107)
  %3076 = load ptr, ptr %145, align 8, !nonnull !1708, !noundef !1708
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %107, ptr noundef nonnull align 8 dereferenceable(24) %143, i64 24, i1 false)
  %3077 = getelementptr inbounds nuw i8, ptr %107, i64 24
  store ptr %3076, ptr %3077, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !33731)
  call void @llvm.experimental.noalias.scope.decl(metadata !33734)
  call void @llvm.experimental.noalias.scope.decl(metadata !33736)
  %3078 = load i64, ptr %108, align 8, !range !2062, !alias.scope !33734, !noalias !33738, !noundef !1708
  %3079 = icmp eq i64 %3078, -1
  br i1 %3079, label %3081, label %3080

3080:                                             ; preds = %3075
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %109, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %107, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %157)
  br label %3083

3081:                                             ; preds = %3075
  %3082 = getelementptr inbounds nuw i8, ptr %109, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %3082, ptr noundef nonnull readonly align 8 dereferenceable(32) %107, i64 32, i1 false), !alias.scope !33738, !noalias !33734
  store i64 -1, ptr %109, align 8, !alias.scope !33731, !noalias !33739
  br label %3083

3083:                                             ; preds = %3081, %3080
  %3084 = getelementptr inbounds nuw i8, ptr %108, i64 72
  %3085 = load i64, ptr %3084, align 8, !range !1940, !alias.scope !33740, !noalias !33738, !noundef !1708
  %3086 = icmp ugt i64 %3085, 5
  br i1 %3086, label %3087, label %3120

3087:                                             ; preds = %3083
  %3088 = getelementptr inbounds nuw i8, ptr %108, i64 80
  %3089 = load ptr, ptr %3088, align 8, !alias.scope !33734, !noalias !33738, !nonnull !1708, !noundef !1708
  %3090 = mul i64 %3085, 3
  %3091 = add i64 %3090, -3
  %3092 = load i64, ptr %267, align 8, !noalias !33743, !noundef !1708
  %3093 = call i64 @llvm.umin.i64(i64 %3091, i64 9223372036854775807)
  %3094 = call i64 @llvm.ssub.sat.i64(i64 %3092, i64 %3093)
  store i64 %3094, ptr %267, align 8, !noalias !33743
  %3095 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3096 = load i64, ptr %3095, align 8, !noalias !33743, !noundef !1708
  %3097 = icmp slt i64 %3094, %3096
  br i1 %3097, label %3098, label %.preheader2158

3098:                                             ; preds = %3087
  store i64 %3094, ptr %3095, align 8, !noalias !33743
  br label %.preheader2158

.preheader2158:                                   ; preds = %3098, %3087
  br label %3099

3099:                                             ; preds = %.preheader2158, %3102
  %3100 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33743
  %3101 = icmp slt i64 %3100, 0
  br i1 %3101, label %3102, label %__rustc::__rust_dealloc (.exit248)

3102:                                             ; preds = %3099
  %3103 = add nsw i64 %3100, 1
  %3104 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3100, i64 %3103 acq_rel acquire, align 8, !noalias !33743
  %3105 = extractvalue { i64, i1 } %3104, 1
  br i1 %3105, label %3106, label %3099

3106:                                             ; preds = %3102
  %3107 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3093 monotonic, align 8, !noalias !33743
  %3108 = call i64 @llvm.ssub.sat.i64(i64 %3107, i64 %3093)
  %3109 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33743
  br label %3110

3110:                                             ; preds = %3113, %3106
  %3111 = phi i64 [ %3109, %3106 ], [ %3116, %3113 ]
  %3112 = icmp slt i64 %3108, %3111
  br i1 %3112, label %3113, label %3117

3113:                                             ; preds = %3110
  %3114 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3111, i64 %3108 monotonic monotonic, align 8, !noalias !33743
  %3115 = extractvalue { i64, i1 } %3114, 1
  %3116 = extractvalue { i64, i1 } %3114, 0
  br i1 %3115, label %3117, label %3110

3117:                                             ; preds = %3113, %3110
  %3118 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33743
  br label %__rustc::__rust_dealloc (.exit248)

__rustc::__rust_dealloc (.exit248): ; preds = %3099, %3117
  %3119 = icmp ne i64 %3091, 0
  call void @llvm.assume(i1 %3119), !noalias !33743
  call void @free(ptr noundef nonnull %3089) #88, !noalias !33743
  br label %3120

3120:                                             ; preds = %__rustc::__rust_dealloc (.exit248), %3083
  %3121 = getelementptr inbounds nuw i8, ptr %108, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !33746)
  %3122 = load ptr, ptr %3121, align 8, !alias.scope !33749, !noalias !33738, !noundef !1708
  %3123 = icmp eq ptr %3122, null
  br i1 %3123, label %3212, label %3124

3124:                                             ; preds = %3120
  %3125 = atomicrmw sub ptr %3122, i64 1 release, align 8, !noalias !33750
  %3126 = icmp eq i64 %3125, 1
  br i1 %3126, label %3127, label %3212

3127:                                             ; preds = %3124
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %3121) #87
          to label %3212 unwind label %3063

3128:                                             ; preds = %3069
  %3129 = load ptr, ptr %145, align 8, !nonnull !1708, !noundef !1708
; invoke <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %110, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %111, ptr noundef nonnull %3129)
          to label %3131 unwind label %3305

3130:                                             ; preds = %3069
  call void @llvm.trap()
  unreachable

3131:                                             ; preds = %3128
  %3132 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %3132, ptr noundef nonnull align 8 dereferenceable(96) %110, i64 96, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %110)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %111)
  call void @llvm.experimental.noalias.scope.decl(metadata !33755)
  %3133 = getelementptr inbounds nuw i8, ptr %143, i64 8
  %3134 = load ptr, ptr %3133, align 8, !alias.scope !33755, !nonnull !1708, !noundef !1708
  %3135 = getelementptr inbounds nuw i8, ptr %143, i64 16
  %3136 = load i64, ptr %3135, align 8, !alias.scope !33755, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33758)
  %3137 = icmp eq i64 %3136, 0
  br i1 %3137, label %.loopexit262, label %.preheader261

.preheader261:                                    ; preds = %3131
  %3138 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %3139

3139:                                             ; preds = %.preheader261, %3177
  %3140 = phi i64 [ %3142, %3177 ], [ 0, %.preheader261 ]
  %3141 = getelementptr inbounds nuw [40 x i8], ptr %3134, i64 %3140
  %3142 = add nuw nsw i64 %3140, 1
  %3143 = load i64, ptr %3141, align 8, !range !1940, !alias.scope !33761, !noalias !33755, !noundef !1708
  %3144 = icmp ugt i64 %3143, 5
  br i1 %3144, label %3145, label %3177

3145:                                             ; preds = %3139
  %3146 = getelementptr i8, ptr %3141, i64 8
  %3147 = load ptr, ptr %3146, align 8, !alias.scope !33758, !noalias !33755, !nonnull !1708, !noundef !1708
  %3148 = shl i64 %3143, 3
  %3149 = add i64 %3148, -8
  %3150 = load i64, ptr %267, align 8, !noalias !33764, !noundef !1708
  %3151 = call i64 @llvm.umin.i64(i64 %3149, i64 9223372036854775807)
  %3152 = call i64 @llvm.ssub.sat.i64(i64 %3150, i64 %3151)
  store i64 %3152, ptr %267, align 8, !noalias !33764
  %3153 = load i64, ptr %3138, align 8, !noalias !33764, !noundef !1708
  %3154 = icmp slt i64 %3152, %3153
  br i1 %3154, label %3155, label %.preheader2160

3155:                                             ; preds = %3145
  store i64 %3152, ptr %3138, align 8, !noalias !33764
  br label %.preheader2160

.preheader2160:                                   ; preds = %3155, %3145
  br label %3156

3156:                                             ; preds = %.preheader2160, %3159
  %3157 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33764
  %3158 = icmp slt i64 %3157, 0
  br i1 %3158, label %3159, label %__rustc::__rust_dealloc (.exit249)

3159:                                             ; preds = %3156
  %3160 = add nsw i64 %3157, 1
  %3161 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3157, i64 %3160 acq_rel acquire, align 8, !noalias !33764
  %3162 = extractvalue { i64, i1 } %3161, 1
  br i1 %3162, label %3163, label %3156

3163:                                             ; preds = %3159
  %3164 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3151 monotonic, align 8, !noalias !33764
  %3165 = call i64 @llvm.ssub.sat.i64(i64 %3164, i64 %3151)
  %3166 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33764
  br label %3167

3167:                                             ; preds = %3170, %3163
  %3168 = phi i64 [ %3166, %3163 ], [ %3173, %3170 ]
  %3169 = icmp slt i64 %3165, %3168
  br i1 %3169, label %3170, label %3174

3170:                                             ; preds = %3167
  %3171 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3168, i64 %3165 monotonic monotonic, align 8, !noalias !33764
  %3172 = extractvalue { i64, i1 } %3171, 1
  %3173 = extractvalue { i64, i1 } %3171, 0
  br i1 %3172, label %3174, label %3167

3174:                                             ; preds = %3170, %3167
  %3175 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33764
  br label %__rustc::__rust_dealloc (.exit249)

__rustc::__rust_dealloc (.exit249): ; preds = %3156, %3174
  %3176 = icmp ne i64 %3149, 0
  call void @llvm.assume(i1 %3176), !noalias !33764
  call void @free(ptr noundef nonnull %3147) #88, !noalias !33764
  br label %3177

3177:                                             ; preds = %__rustc::__rust_dealloc (.exit249), %3139
  %3178 = icmp eq i64 %3142, %3136
  br i1 %3178, label %.loopexit262, label %3139

.loopexit262:                                     ; preds = %3177, %3131
  %3179 = load i64, ptr %143, align 8, !alias.scope !33755
  %3180 = icmp eq i64 %3179, 0
  br i1 %3180, label %3210, label %3181

3181:                                             ; preds = %.loopexit262
  %3182 = mul nuw i64 %3179, 40
  %3183 = load i64, ptr %267, align 8, !noalias !33755, !noundef !1708
  %3184 = call i64 @llvm.umin.i64(i64 %3182, i64 9223372036854775807)
  %3185 = call i64 @llvm.ssub.sat.i64(i64 %3183, i64 %3184)
  store i64 %3185, ptr %267, align 8, !noalias !33755
  %3186 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3187 = load i64, ptr %3186, align 8, !noalias !33755, !noundef !1708
  %3188 = icmp slt i64 %3185, %3187
  br i1 %3188, label %3189, label %.preheader2159

3189:                                             ; preds = %3181
  store i64 %3185, ptr %3186, align 8, !noalias !33755
  br label %.preheader2159

.preheader2159:                                   ; preds = %3189, %3181
  br label %3190

3190:                                             ; preds = %.preheader2159, %3193
  %3191 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33755
  %3192 = icmp slt i64 %3191, 0
  br i1 %3192, label %3193, label %__rustc::__rust_dealloc (.exit250)

3193:                                             ; preds = %3190
  %3194 = add nsw i64 %3191, 1
  %3195 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3191, i64 %3194 acq_rel acquire, align 8, !noalias !33755
  %3196 = extractvalue { i64, i1 } %3195, 1
  br i1 %3196, label %3197, label %3190

3197:                                             ; preds = %3193
  %3198 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3184 monotonic, align 8, !noalias !33755
  %3199 = call i64 @llvm.ssub.sat.i64(i64 %3198, i64 %3184)
  %3200 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33755
  br label %3201

3201:                                             ; preds = %3204, %3197
  %3202 = phi i64 [ %3200, %3197 ], [ %3207, %3204 ]
  %3203 = icmp slt i64 %3199, %3202
  br i1 %3203, label %3204, label %3208

3204:                                             ; preds = %3201
  %3205 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3202, i64 %3199 monotonic monotonic, align 8, !noalias !33755
  %3206 = extractvalue { i64, i1 } %3205, 1
  %3207 = extractvalue { i64, i1 } %3205, 0
  br i1 %3206, label %3208, label %3201

3208:                                             ; preds = %3204, %3201
  %3209 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33755
  br label %__rustc::__rust_dealloc (.exit250)

__rustc::__rust_dealloc (.exit250): ; preds = %3190, %3208
  call void @free(ptr noundef nonnull %3134) #88, !noalias !33755
  br label %3210

3210:                                             ; preds = %3320, %__rustc::__rust_dealloc (.exit250), %.loopexit262, %659
  %3211 = phi i8 [ 1, %3320 ], [ 0, %659 ], [ %663, %.loopexit262 ], [ %663, %__rustc::__rust_dealloc (.exit250) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %143)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %144)
          to label %3321 unwind label %303

3212:                                             ; preds = %3127, %3124, %3120
  call void @llvm.lifetime.end.p0(ptr nonnull %107)
  call void @llvm.lifetime.end.p0(ptr nonnull %108)
  %3213 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %3213, ptr noundef nonnull align 8 dereferenceable(96) %109, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %109)
  call void @llvm.lifetime.end.p0(ptr nonnull %143)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %144)
          to label %3214 unwind label %306

3214:                                             ; preds = %3212
  call void @llvm.lifetime.end.p0(ptr nonnull %144)
  call void @llvm.lifetime.end.p0(ptr nonnull %145)
  call void @llvm.lifetime.end.p0(ptr nonnull %146)
  call void @llvm.lifetime.end.p0(ptr nonnull %147)
  call void @llvm.lifetime.end.p0(ptr nonnull %148)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %149)
          to label %3215 unwind label %191

3215:                                             ; preds = %3214
  call void @llvm.lifetime.end.p0(ptr nonnull %149)
  call void @llvm.experimental.noalias.scope.decl(metadata !33767)
  call void @llvm.experimental.noalias.scope.decl(metadata !33770)
  %3216 = load ptr, ptr %232, align 8, !alias.scope !33773, !nonnull !1708, !noundef !1708
  %3217 = atomicrmw sub ptr %3216, i64 1 release, align 8, !noalias !33773
  %3218 = icmp eq i64 %3217, 1
  br i1 %3218, label %3219, label %3223

3219:                                             ; preds = %3215
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %232) #87
          to label %3223 unwind label %3220

3220:                                             ; preds = %3219
  %3221 = landingpad { ptr, i32 }
          cleanup
  %3222 = trunc nuw i8 %663 to i1
  br i1 %3222, label %3304, label %3652

3223:                                             ; preds = %3219, %3215
  %3224 = trunc nuw i8 %663 to i1
  br i1 %3224, label %3226, label %3225

3225:                                             ; preds = %__rustc::__rust_dealloc (.exit252), %.loopexit260, %3223
  call void @llvm.lifetime.end.p0(ptr nonnull %152)
  br label %3303

3226:                                             ; preds = %3223
  call void @llvm.experimental.noalias.scope.decl(metadata !33774)
  %3227 = getelementptr inbounds nuw i8, ptr %152, i64 8
  %3228 = load ptr, ptr %3227, align 8, !alias.scope !33774, !nonnull !1708, !noundef !1708
  %3229 = load i64, ptr %218, align 8, !alias.scope !33774, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33777)
  %3230 = icmp eq i64 %3229, 0
  br i1 %3230, label %.loopexit260, label %.preheader259

.preheader259:                                    ; preds = %3226
  %3231 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %3232

3232:                                             ; preds = %.preheader259, %3270
  %3233 = phi i64 [ %3235, %3270 ], [ 0, %.preheader259 ]
  %3234 = getelementptr inbounds nuw [40 x i8], ptr %3228, i64 %3233
  %3235 = add nuw nsw i64 %3233, 1
  %3236 = load i64, ptr %3234, align 8, !range !1940, !alias.scope !33780, !noalias !33774, !noundef !1708
  %3237 = icmp ugt i64 %3236, 5
  br i1 %3237, label %3238, label %3270

3238:                                             ; preds = %3232
  %3239 = getelementptr i8, ptr %3234, i64 8
  %3240 = load ptr, ptr %3239, align 8, !alias.scope !33777, !noalias !33774, !nonnull !1708, !noundef !1708
  %3241 = shl i64 %3236, 3
  %3242 = add i64 %3241, -8
  %3243 = load i64, ptr %267, align 8, !noalias !33783, !noundef !1708
  %3244 = call i64 @llvm.umin.i64(i64 %3242, i64 9223372036854775807)
  %3245 = call i64 @llvm.ssub.sat.i64(i64 %3243, i64 %3244)
  store i64 %3245, ptr %267, align 8, !noalias !33783
  %3246 = load i64, ptr %3231, align 8, !noalias !33783, !noundef !1708
  %3247 = icmp slt i64 %3245, %3246
  br i1 %3247, label %3248, label %.preheader2157

3248:                                             ; preds = %3238
  store i64 %3245, ptr %3231, align 8, !noalias !33783
  br label %.preheader2157

.preheader2157:                                   ; preds = %3248, %3238
  br label %3249

3249:                                             ; preds = %.preheader2157, %3252
  %3250 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33783
  %3251 = icmp slt i64 %3250, 0
  br i1 %3251, label %3252, label %__rustc::__rust_dealloc (.exit251)

3252:                                             ; preds = %3249
  %3253 = add nsw i64 %3250, 1
  %3254 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3250, i64 %3253 acq_rel acquire, align 8, !noalias !33783
  %3255 = extractvalue { i64, i1 } %3254, 1
  br i1 %3255, label %3256, label %3249

3256:                                             ; preds = %3252
  %3257 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3244 monotonic, align 8, !noalias !33783
  %3258 = call i64 @llvm.ssub.sat.i64(i64 %3257, i64 %3244)
  %3259 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33783
  br label %3260

3260:                                             ; preds = %3263, %3256
  %3261 = phi i64 [ %3259, %3256 ], [ %3266, %3263 ]
  %3262 = icmp slt i64 %3258, %3261
  br i1 %3262, label %3263, label %3267

3263:                                             ; preds = %3260
  %3264 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3261, i64 %3258 monotonic monotonic, align 8, !noalias !33783
  %3265 = extractvalue { i64, i1 } %3264, 1
  %3266 = extractvalue { i64, i1 } %3264, 0
  br i1 %3265, label %3267, label %3260

3267:                                             ; preds = %3263, %3260
  %3268 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33783
  br label %__rustc::__rust_dealloc (.exit251)

__rustc::__rust_dealloc (.exit251): ; preds = %3249, %3267
  %3269 = icmp ne i64 %3242, 0
  call void @llvm.assume(i1 %3269), !noalias !33783
  call void @free(ptr noundef nonnull %3240) #88, !noalias !33783
  br label %3270

3270:                                             ; preds = %__rustc::__rust_dealloc (.exit251), %3232
  %3271 = icmp eq i64 %3235, %3229
  br i1 %3271, label %.loopexit260, label %3232

.loopexit260:                                     ; preds = %3270, %3226
  %3272 = load i64, ptr %152, align 8, !alias.scope !33774
  %3273 = icmp eq i64 %3272, 0
  br i1 %3273, label %3225, label %3274

3274:                                             ; preds = %.loopexit260
  %3275 = mul nuw i64 %3272, 40
  %3276 = load i64, ptr %267, align 8, !noalias !33774, !noundef !1708
  %3277 = call i64 @llvm.umin.i64(i64 %3275, i64 9223372036854775807)
  %3278 = call i64 @llvm.ssub.sat.i64(i64 %3276, i64 %3277)
  store i64 %3278, ptr %267, align 8, !noalias !33774
  %3279 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3280 = load i64, ptr %3279, align 8, !noalias !33774, !noundef !1708
  %3281 = icmp slt i64 %3278, %3280
  br i1 %3281, label %3282, label %.preheader2156

3282:                                             ; preds = %3274
  store i64 %3278, ptr %3279, align 8, !noalias !33774
  br label %.preheader2156

.preheader2156:                                   ; preds = %3282, %3274
  br label %3283

3283:                                             ; preds = %.preheader2156, %3286
  %3284 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33774
  %3285 = icmp slt i64 %3284, 0
  br i1 %3285, label %3286, label %__rustc::__rust_dealloc (.exit252)

3286:                                             ; preds = %3283
  %3287 = add nsw i64 %3284, 1
  %3288 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3284, i64 %3287 acq_rel acquire, align 8, !noalias !33774
  %3289 = extractvalue { i64, i1 } %3288, 1
  br i1 %3289, label %3290, label %3283

3290:                                             ; preds = %3286
  %3291 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3277 monotonic, align 8, !noalias !33774
  %3292 = call i64 @llvm.ssub.sat.i64(i64 %3291, i64 %3277)
  %3293 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33774
  br label %3294

3294:                                             ; preds = %3297, %3290
  %3295 = phi i64 [ %3293, %3290 ], [ %3300, %3297 ]
  %3296 = icmp slt i64 %3292, %3295
  br i1 %3296, label %3297, label %3301

3297:                                             ; preds = %3294
  %3298 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3295, i64 %3292 monotonic monotonic, align 8, !noalias !33774
  %3299 = extractvalue { i64, i1 } %3298, 1
  %3300 = extractvalue { i64, i1 } %3298, 0
  br i1 %3299, label %3301, label %3294

3301:                                             ; preds = %3297, %3294
  %3302 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33774
  br label %__rustc::__rust_dealloc (.exit252)

__rustc::__rust_dealloc (.exit252): ; preds = %3283, %3301
  call void @free(ptr noundef nonnull %3228) #88, !noalias !33774
  br label %3225

3303:                                             ; preds = %3647, %3495, %3492, %3488, %3225
  call void @llvm.lifetime.end.p0(ptr nonnull %157)
  ret void

3304:                                             ; preds = %3220
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %152) #89
  br label %3652

3305:                                             ; preds = %3128
  %3306 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %143) #89
  br label %351

3307:                                             ; preds = %3048, %1104
  call void @llvm.experimental.noalias.scope.decl(metadata !33786)
  %3308 = load ptr, ptr %141, align 8, !alias.scope !33786, !noundef !1708
  %3309 = icmp eq ptr %3308, null
  br i1 %3309, label %3320, label %3310

3310:                                             ; preds = %3307
  %3311 = atomicrmw sub ptr %3308, i64 1 release, align 8, !noalias !33789
  %3312 = icmp eq i64 %3311, 1
  br i1 %3312, label %3313, label %3320

3313:                                             ; preds = %3310
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %141) #87
          to label %3320 unwind label %355, !inline_history !1744

3314:                                             ; preds = %2789, %2611
  %3315 = landingpad { ptr, i32 }
          cleanup
  br label %3316

3316:                                             ; preds = %3314, %2803, %2788, %2623, %2621, %2590, %2540, %2533, %._crit_edge2060, %2502
  %3317 = phi { ptr, i32 } [ %3315, %3314 ], [ %2798, %2788 ], [ %2798, %2803 ], [ %2622, %2621 ], [ %2526, %2533 ], [ %2541, %2540 ], [ %2504, %2502 ], [ %2624, %2623 ], [ %2591, %2590 ], [ %2526, %._crit_edge2060 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %135) #89
          to label %707 unwind label %679

3318:                                             ; preds = %1110
  %3319 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %140) #89
          to label %707 unwind label %679

3320:                                             ; preds = %3313, %3310, %3307
  call void @llvm.lifetime.end.p0(ptr nonnull %141)
  call void @llvm.lifetime.end.p0(ptr nonnull %142)
  br label %3210

3321:                                             ; preds = %3210
  call void @llvm.lifetime.end.p0(ptr nonnull %144)
  call void @llvm.experimental.noalias.scope.decl(metadata !33792)
  call void @llvm.experimental.noalias.scope.decl(metadata !33795)
  %3322 = load ptr, ptr %145, align 8, !alias.scope !33798, !nonnull !1708, !noundef !1708
  %3323 = atomicrmw sub ptr %3322, i64 1 release, align 8, !noalias !33798
  %3324 = icmp eq i64 %3323, 1
  br i1 %3324, label %3325, label %3326

3325:                                             ; preds = %3321
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %145) #87
          to label %3326 unwind label %250

3326:                                             ; preds = %3325, %3321
  call void @llvm.lifetime.end.p0(ptr nonnull %145)
  call void @llvm.lifetime.end.p0(ptr nonnull %146)
  call void @llvm.lifetime.end.p0(ptr nonnull %147)
  call void @llvm.lifetime.end.p0(ptr nonnull %148)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %149)
          to label %3327 unwind label %191

3327:                                             ; preds = %3326
  call void @llvm.lifetime.end.p0(ptr nonnull %149)
  call void @llvm.experimental.noalias.scope.decl(metadata !33799)
  call void @llvm.experimental.noalias.scope.decl(metadata !33802)
  %3328 = load ptr, ptr %232, align 8, !alias.scope !33805, !nonnull !1708, !noundef !1708
  %3329 = atomicrmw sub ptr %3328, i64 1 release, align 8, !noalias !33805
  %3330 = icmp eq i64 %3329, 1
  br i1 %3330, label %3331, label %3335

3331:                                             ; preds = %3327
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %232) #87
          to label %3335 unwind label %3332

3332:                                             ; preds = %3331
  %3333 = landingpad { ptr, i32 }
          cleanup
  %3334 = trunc nuw i8 %3211 to i1
  br i1 %3334, label %3496, label %3654

3335:                                             ; preds = %3331, %3327
  %3336 = trunc nuw i8 %3211 to i1
  br i1 %3336, label %3338, label %3337

3337:                                             ; preds = %__rustc::__rust_dealloc (.exit254), %.loopexit, %3335
  call void @llvm.lifetime.end.p0(ptr nonnull %152)
  br label %3415

3338:                                             ; preds = %3335
  call void @llvm.experimental.noalias.scope.decl(metadata !33806)
  %3339 = getelementptr inbounds nuw i8, ptr %152, i64 8
  %3340 = load ptr, ptr %3339, align 8, !alias.scope !33806, !nonnull !1708, !noundef !1708
  %3341 = load i64, ptr %218, align 8, !alias.scope !33806, !noundef !1708
  call void @llvm.experimental.noalias.scope.decl(metadata !33809)
  %3342 = icmp eq i64 %3341, 0
  br i1 %3342, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %3338
  %3343 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %3344

3344:                                             ; preds = %.preheader, %3382
  %3345 = phi i64 [ %3347, %3382 ], [ 0, %.preheader ]
  %3346 = getelementptr inbounds nuw [40 x i8], ptr %3340, i64 %3345
  %3347 = add nuw nsw i64 %3345, 1
  %3348 = load i64, ptr %3346, align 8, !range !1940, !alias.scope !33812, !noalias !33806, !noundef !1708
  %3349 = icmp ugt i64 %3348, 5
  br i1 %3349, label %3350, label %3382

3350:                                             ; preds = %3344
  %3351 = getelementptr i8, ptr %3346, i64 8
  %3352 = load ptr, ptr %3351, align 8, !alias.scope !33809, !noalias !33806, !nonnull !1708, !noundef !1708
  %3353 = shl i64 %3348, 3
  %3354 = add i64 %3353, -8
  %3355 = load i64, ptr %267, align 8, !noalias !33815, !noundef !1708
  %3356 = call i64 @llvm.umin.i64(i64 %3354, i64 9223372036854775807)
  %3357 = call i64 @llvm.ssub.sat.i64(i64 %3355, i64 %3356)
  store i64 %3357, ptr %267, align 8, !noalias !33815
  %3358 = load i64, ptr %3343, align 8, !noalias !33815, !noundef !1708
  %3359 = icmp slt i64 %3357, %3358
  br i1 %3359, label %3360, label %.preheader2155

3360:                                             ; preds = %3350
  store i64 %3357, ptr %3343, align 8, !noalias !33815
  br label %.preheader2155

.preheader2155:                                   ; preds = %3360, %3350
  br label %3361

3361:                                             ; preds = %.preheader2155, %3364
  %3362 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33815
  %3363 = icmp slt i64 %3362, 0
  br i1 %3363, label %3364, label %__rustc::__rust_dealloc (.exit253)

3364:                                             ; preds = %3361
  %3365 = add nsw i64 %3362, 1
  %3366 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3362, i64 %3365 acq_rel acquire, align 8, !noalias !33815
  %3367 = extractvalue { i64, i1 } %3366, 1
  br i1 %3367, label %3368, label %3361

3368:                                             ; preds = %3364
  %3369 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3356 monotonic, align 8, !noalias !33815
  %3370 = call i64 @llvm.ssub.sat.i64(i64 %3369, i64 %3356)
  %3371 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33815
  br label %3372

3372:                                             ; preds = %3375, %3368
  %3373 = phi i64 [ %3371, %3368 ], [ %3378, %3375 ]
  %3374 = icmp slt i64 %3370, %3373
  br i1 %3374, label %3375, label %3379

3375:                                             ; preds = %3372
  %3376 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3373, i64 %3370 monotonic monotonic, align 8, !noalias !33815
  %3377 = extractvalue { i64, i1 } %3376, 1
  %3378 = extractvalue { i64, i1 } %3376, 0
  br i1 %3377, label %3379, label %3372

3379:                                             ; preds = %3375, %3372
  %3380 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33815
  br label %__rustc::__rust_dealloc (.exit253)

__rustc::__rust_dealloc (.exit253): ; preds = %3361, %3379
  %3381 = icmp ne i64 %3354, 0
  call void @llvm.assume(i1 %3381), !noalias !33815
  call void @free(ptr noundef nonnull %3352) #88, !noalias !33815
  br label %3382

3382:                                             ; preds = %__rustc::__rust_dealloc (.exit253), %3344
  %3383 = icmp eq i64 %3347, %3341
  br i1 %3383, label %.loopexit, label %3344

.loopexit:                                        ; preds = %3382, %3338
  %3384 = load i64, ptr %152, align 8, !alias.scope !33806
  %3385 = icmp eq i64 %3384, 0
  br i1 %3385, label %3337, label %3386

3386:                                             ; preds = %.loopexit
  %3387 = mul nuw i64 %3384, 40
  %3388 = load i64, ptr %267, align 8, !noalias !33806, !noundef !1708
  %3389 = call i64 @llvm.umin.i64(i64 %3387, i64 9223372036854775807)
  %3390 = call i64 @llvm.ssub.sat.i64(i64 %3388, i64 %3389)
  store i64 %3390, ptr %267, align 8, !noalias !33806
  %3391 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3392 = load i64, ptr %3391, align 8, !noalias !33806, !noundef !1708
  %3393 = icmp slt i64 %3390, %3392
  br i1 %3393, label %3394, label %.preheader2154

3394:                                             ; preds = %3386
  store i64 %3390, ptr %3391, align 8, !noalias !33806
  br label %.preheader2154

.preheader2154:                                   ; preds = %3394, %3386
  br label %3395

3395:                                             ; preds = %.preheader2154, %3398
  %3396 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33806
  %3397 = icmp slt i64 %3396, 0
  br i1 %3397, label %3398, label %__rustc::__rust_dealloc (.exit254)

3398:                                             ; preds = %3395
  %3399 = add nsw i64 %3396, 1
  %3400 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3396, i64 %3399 acq_rel acquire, align 8, !noalias !33806
  %3401 = extractvalue { i64, i1 } %3400, 1
  br i1 %3401, label %3402, label %3395

3402:                                             ; preds = %3398
  %3403 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3389 monotonic, align 8, !noalias !33806
  %3404 = call i64 @llvm.ssub.sat.i64(i64 %3403, i64 %3389)
  %3405 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33806
  br label %3406

3406:                                             ; preds = %3409, %3402
  %3407 = phi i64 [ %3405, %3402 ], [ %3412, %3409 ]
  %3408 = icmp slt i64 %3404, %3407
  br i1 %3408, label %3409, label %3413

3409:                                             ; preds = %3406
  %3410 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3407, i64 %3404 monotonic monotonic, align 8, !noalias !33806
  %3411 = extractvalue { i64, i1 } %3410, 1
  %3412 = extractvalue { i64, i1 } %3410, 0
  br i1 %3411, label %3413, label %3406

3413:                                             ; preds = %3409, %3406
  %3414 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33806
  br label %__rustc::__rust_dealloc (.exit254)

__rustc::__rust_dealloc (.exit254): ; preds = %3395, %3413
  call void @free(ptr noundef nonnull %3340) #88, !noalias !33806
  br label %3337

3415:                                             ; preds = %3337, %164
  call void @llvm.experimental.noalias.scope.decl(metadata !33818)
  %3416 = getelementptr inbounds nuw i8, ptr %157, i64 72
  %3417 = load i64, ptr %3416, align 8, !range !1940, !alias.scope !33821, !noundef !1708
  %3418 = icmp ugt i64 %3417, 5
  br i1 %3418, label %3419, label %3453

3419:                                             ; preds = %3415
  %3420 = getelementptr inbounds nuw i8, ptr %157, i64 80
  %3421 = load ptr, ptr %3420, align 8, !alias.scope !33818, !nonnull !1708, !noundef !1708
  %3422 = mul i64 %3417, 3
  %3423 = add i64 %3422, -3
  %3424 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3425 = load i64, ptr %3424, align 8, !noalias !33824, !noundef !1708
  %3426 = call i64 @llvm.umin.i64(i64 %3423, i64 9223372036854775807)
  %3427 = call i64 @llvm.ssub.sat.i64(i64 %3425, i64 %3426)
  store i64 %3427, ptr %3424, align 8, !noalias !33824
  %3428 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3429 = load i64, ptr %3428, align 8, !noalias !33824, !noundef !1708
  %3430 = icmp slt i64 %3427, %3429
  br i1 %3430, label %3431, label %.preheader2151

3431:                                             ; preds = %3419
  store i64 %3427, ptr %3428, align 8, !noalias !33824
  br label %.preheader2151

.preheader2151:                                   ; preds = %3431, %3419
  br label %3432

3432:                                             ; preds = %.preheader2151, %3435
  %3433 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33824
  %3434 = icmp slt i64 %3433, 0
  br i1 %3434, label %3435, label %__rustc::__rust_dealloc (.exit255)

3435:                                             ; preds = %3432
  %3436 = add nsw i64 %3433, 1
  %3437 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3433, i64 %3436 acq_rel acquire, align 8, !noalias !33824
  %3438 = extractvalue { i64, i1 } %3437, 1
  br i1 %3438, label %3439, label %3432

3439:                                             ; preds = %3435
  %3440 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3426 monotonic, align 8, !noalias !33824
  %3441 = call i64 @llvm.ssub.sat.i64(i64 %3440, i64 %3426)
  %3442 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33824
  br label %3443

3443:                                             ; preds = %3446, %3439
  %3444 = phi i64 [ %3442, %3439 ], [ %3449, %3446 ]
  %3445 = icmp slt i64 %3441, %3444
  br i1 %3445, label %3446, label %3450

3446:                                             ; preds = %3443
  %3447 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3444, i64 %3441 monotonic monotonic, align 8, !noalias !33824
  %3448 = extractvalue { i64, i1 } %3447, 1
  %3449 = extractvalue { i64, i1 } %3447, 0
  br i1 %3448, label %3450, label %3443

3450:                                             ; preds = %3446, %3443
  %3451 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33824
  br label %__rustc::__rust_dealloc (.exit255)

__rustc::__rust_dealloc (.exit255): ; preds = %3432, %3450
  %3452 = icmp ne i64 %3423, 0
  call void @llvm.assume(i1 %3452), !noalias !33824
  call void @free(ptr noundef nonnull %3421) #88, !noalias !33824
  br label %3453

3453:                                             ; preds = %__rustc::__rust_dealloc (.exit255), %3415
  %3454 = load i64, ptr %157, align 8, !range !2062, !alias.scope !33818, !noundef !1708
  %3455 = icmp sgt i64 %3454, 0
  br i1 %3455, label %3456, label %3488

3456:                                             ; preds = %3453
  %3457 = getelementptr inbounds nuw i8, ptr %157, i64 8
  %3458 = load ptr, ptr %3457, align 8, !alias.scope !33818, !nonnull !1708, !noundef !1708
  %3459 = mul nuw i64 %3454, 3
  %3460 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3461 = load i64, ptr %3460, align 8, !noalias !33818, !noundef !1708
  %3462 = call i64 @llvm.umin.i64(i64 %3459, i64 9223372036854775807)
  %3463 = call i64 @llvm.ssub.sat.i64(i64 %3461, i64 %3462)
  store i64 %3463, ptr %3460, align 8, !noalias !33818
  %3464 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3465 = load i64, ptr %3464, align 8, !noalias !33818, !noundef !1708
  %3466 = icmp slt i64 %3463, %3465
  br i1 %3466, label %3467, label %.preheader2150

3467:                                             ; preds = %3456
  store i64 %3463, ptr %3464, align 8, !noalias !33818
  br label %.preheader2150

.preheader2150:                                   ; preds = %3467, %3456
  br label %3468

3468:                                             ; preds = %.preheader2150, %3471
  %3469 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33818
  %3470 = icmp slt i64 %3469, 0
  br i1 %3470, label %3471, label %__rustc::__rust_dealloc (.exit256)

3471:                                             ; preds = %3468
  %3472 = add nsw i64 %3469, 1
  %3473 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3469, i64 %3472 acq_rel acquire, align 8, !noalias !33818
  %3474 = extractvalue { i64, i1 } %3473, 1
  br i1 %3474, label %3475, label %3468

3475:                                             ; preds = %3471
  %3476 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3462 monotonic, align 8, !noalias !33818
  %3477 = call i64 @llvm.ssub.sat.i64(i64 %3476, i64 %3462)
  %3478 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33818
  br label %3479

3479:                                             ; preds = %3482, %3475
  %3480 = phi i64 [ %3478, %3475 ], [ %3485, %3482 ]
  %3481 = icmp slt i64 %3477, %3480
  br i1 %3481, label %3482, label %3486

3482:                                             ; preds = %3479
  %3483 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3480, i64 %3477 monotonic monotonic, align 8, !noalias !33818
  %3484 = extractvalue { i64, i1 } %3483, 1
  %3485 = extractvalue { i64, i1 } %3483, 0
  br i1 %3484, label %3486, label %3479

3486:                                             ; preds = %3482, %3479
  %3487 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33818
  br label %__rustc::__rust_dealloc (.exit256)

__rustc::__rust_dealloc (.exit256): ; preds = %3468, %3486
  call void @free(ptr noundef nonnull %3458) #88, !noalias !33818
  br label %3488

3488:                                             ; preds = %__rustc::__rust_dealloc (.exit256), %3453
  %3489 = getelementptr inbounds nuw i8, ptr %157, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !33827)
  %3490 = load ptr, ptr %3489, align 8, !alias.scope !33830, !noundef !1708
  %3491 = icmp eq ptr %3490, null
  br i1 %3491, label %3303, label %3492

3492:                                             ; preds = %3488
  %3493 = atomicrmw sub ptr %3490, i64 1 release, align 8, !noalias !33831
  %3494 = icmp eq i64 %3493, 1
  br i1 %3494, label %3495, label %3303

3495:                                             ; preds = %3492
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %3489) #87
  br label %3303

3496:                                             ; preds = %3332
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %152) #89
  br label %3654

3497:                                             ; preds = %303, %302
  %3498 = phi { ptr, i32 } [ %305, %303 ], [ %354, %302 ]
  %3499 = phi i8 [ %304, %303 ], [ %352, %302 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !33836)
  call void @llvm.experimental.noalias.scope.decl(metadata !33839)
  %3500 = load ptr, ptr %145, align 8, !alias.scope !33842, !nonnull !1708, !noundef !1708
  %3501 = atomicrmw sub ptr %3500, i64 1 release, align 8, !noalias !33842
  %3502 = icmp eq i64 %3501, 1
  br i1 %3502, label %3503, label %235

3503:                                             ; preds = %3497
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %145) #87
          to label %235 unwind label %679

3504:                                             ; preds = %245
  %3505 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)(ptr noalias nofree noundef align 8 dereferenceable(56) %148) #89
          to label %235 unwind label %679

3506:                                             ; preds = %190, %182
  %3507 = trunc nuw i8 %183 to i1
  br i1 %3507, label %3508, label %158

3508:                                             ; preds = %3506
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %152) #89
  br label %158

3509:                                             ; preds = %178
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !33843
  store ptr %176, ptr %35, align 8, !noalias !33846
  %3510 = getelementptr inbounds nuw i8, ptr %176, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(56) %156, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %3510)
          to label %3516 unwind label %3511

3511:                                             ; preds = %3509
  %3512 = landingpad { ptr, i32 }
          cleanup
  %3513 = atomicrmw sub ptr %176, i64 1 release, align 8, !noalias !33849
  %3514 = icmp eq i64 %3513, 1
  br i1 %3514, label %3515, label %3654

3515:                                             ; preds = %3511
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %35) #87
          to label %3654 unwind label %3520, !noalias !33846

3516:                                             ; preds = %3509
  %3517 = atomicrmw sub ptr %176, i64 1 release, align 8, !noalias !33854
  %3518 = icmp eq i64 %3517, 1
  br i1 %3518, label %3519, label %3522

3519:                                             ; preds = %3516
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %35) #87
          to label %3522 unwind label %159

3520:                                             ; preds = %3515
  %3521 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #86, !noalias !33846
  unreachable

3522:                                             ; preds = %3519, %3516
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !33843
  br label %3539

3523:                                             ; preds = %174
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !33859
; invoke purrdf_sparql_eval::eval::syntactic_schema
  %3524 = invoke noundef nonnull ptr @purrdf_sparql_eval::eval::syntactic_schema(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2)
          to label %3525 unwind label %159

3525:                                             ; preds = %3523
  store ptr %3524, ptr %34, align 8, !noalias !33859
  %3526 = getelementptr inbounds nuw i8, ptr %3524, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(56) %156, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %3526)
          to label %3532 unwind label %3527

3527:                                             ; preds = %3525
  %3528 = landingpad { ptr, i32 }
          cleanup
  %3529 = atomicrmw sub ptr %3524, i64 1 release, align 8, !noalias !33862
  %3530 = icmp eq i64 %3529, 1
  br i1 %3530, label %3531, label %3654

3531:                                             ; preds = %3527
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %34) #87
          to label %3654 unwind label %3536, !noalias !33859

3532:                                             ; preds = %3525
  %3533 = atomicrmw sub ptr %3524, i64 1 release, align 8, !noalias !33867
  %3534 = icmp eq i64 %3533, 1
  br i1 %3534, label %3535, label %3538

3535:                                             ; preds = %3532
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %34) #87
          to label %3538 unwind label %159

3536:                                             ; preds = %3531
  %3537 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #86, !noalias !33859
  unreachable

3538:                                             ; preds = %3535, %3532
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !33859
  br label %3539

3539:                                             ; preds = %3538, %3522
  %3540 = load ptr, ptr %3, align 8, !nonnull !1708, !noundef !1708
  %3541 = atomicrmw add ptr %3540, i64 1 monotonic, align 8
  %3542 = icmp slt i64 %3541, 0
  br i1 %3542, label %3547, label %3543

3543:                                             ; preds = %3539
  %3544 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %3545 = load i64, ptr %3544, align 8, !noundef !1708
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %3546 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %156, ptr noundef nonnull %3540, i64 noundef %3545)
          to label %3548 unwind label %3650

3547:                                             ; preds = %3539
  tail call void @llvm.trap()
  unreachable

3548:                                             ; preds = %3543
  call void @llvm.lifetime.start.p0(ptr nonnull %155)
  call void @llvm.lifetime.start.p0(ptr nonnull %154)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %154, ptr noundef nonnull align 8 dereferenceable(104) %157, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %153)
  %3549 = getelementptr inbounds nuw i8, ptr %106, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %106)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %3549, ptr noundef nonnull align 8 dereferenceable(56) %156, i64 56, i1 false)
  store i64 1, ptr %106, align 8
  %3550 = getelementptr inbounds nuw i8, ptr %106, i64 8
  store i64 1, ptr %3550, align 8
  %3551 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #88, !noalias !33872
  %3552 = icmp eq ptr %3551, null
  br i1 %3552, label %__rustc::__rust_alloc (.exit257.thread), label %3553

3553:                                             ; preds = %3548
  %3554 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3555 = load i64, ptr %3554, align 8, !noalias !33872, !noundef !1708
  %3556 = call i64 @llvm.uadd.sat.i64(i64 %3555, i64 1)
  store i64 %3556, ptr %3554, align 8, !noalias !33872
  %3557 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3558 = load i64, ptr %3557, align 8, !noalias !33872, !noundef !1708
  %3559 = call i64 @llvm.uadd.sat.i64(i64 %3558, i64 72)
  store i64 %3559, ptr %3557, align 8, !noalias !33872
  %3560 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3561 = load i64, ptr %3560, align 8, !noalias !33872, !noundef !1708
  %3562 = call i64 @llvm.sadd.sat.i64(i64 %3561, i64 72)
  store i64 %3562, ptr %3560, align 8, !noalias !33872
  %3563 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3564 = load i64, ptr %3563, align 8, !noalias !33872, !noundef !1708
  %3565 = icmp sgt i64 %3562, %3564
  br i1 %3565, label %3566, label %.preheader2153

3566:                                             ; preds = %3553
  store i64 %3562, ptr %3563, align 8, !noalias !33872
  br label %.preheader2153

.preheader2153:                                   ; preds = %3566, %3553
  br label %3567

3567:                                             ; preds = %.preheader2153, %3570
  %3568 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33872
  %3569 = icmp slt i64 %3568, 0
  br i1 %3569, label %3570, label %__rustc::__rust_alloc (.exit257)

3570:                                             ; preds = %3567
  %3571 = add nsw i64 %3568, 1
  %3572 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3568, i64 %3571 acq_rel acquire, align 8, !noalias !33872
  %3573 = extractvalue { i64, i1 } %3572, 1
  br i1 %3573, label %3574, label %3567

3574:                                             ; preds = %3570
  %3575 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !33872
  %3576 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !33872
  %3577 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !33872
  %3578 = call i64 @llvm.sadd.sat.i64(i64 %3577, i64 72)
  %3579 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !33872
  br label %3580

3580:                                             ; preds = %3583, %3574
  %3581 = phi i64 [ %3579, %3574 ], [ %3586, %3583 ]
  %3582 = icmp sgt i64 %3578, %3581
  br i1 %3582, label %3583, label %3587

3583:                                             ; preds = %3580
  %3584 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %3581, i64 %3578 monotonic monotonic, align 8, !noalias !33872
  %3585 = extractvalue { i64, i1 } %3584, 1
  %3586 = extractvalue { i64, i1 } %3584, 0
  br i1 %3585, label %3587, label %3580

3587:                                             ; preds = %3583, %3580
  %3588 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33872
  br label %__rustc::__rust_alloc (.exit257)

__rustc::__rust_alloc (.exit257.thread): ; preds = %3548
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #90
          to label %3589 unwind label %3590

3589:                                             ; preds = %__rustc::__rust_alloc (.exit257.thread)
  unreachable

3590:                                             ; preds = %__rustc::__rust_alloc (.exit257.thread)
  %3591 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %3549)
          to label %3649 unwind label %3592

3592:                                             ; preds = %3590
  %3593 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86
  unreachable

__rustc::__rust_alloc (.exit257): ; preds = %3567, %3587
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %3551, ptr noundef nonnull align 8 dereferenceable(72) %106, i64 72, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %106)
  %3594 = getelementptr inbounds nuw i8, ptr %153, i64 24
  store ptr %3551, ptr %3594, align 8, !alias.scope !33875
  store i64 0, ptr %153, align 8, !alias.scope !33875
  %3595 = getelementptr inbounds nuw i8, ptr %153, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %3595, align 8, !alias.scope !33875
  %3596 = getelementptr inbounds nuw i8, ptr %153, i64 16
  store i64 0, ptr %3596, align 8, !alias.scope !33875
  call void @llvm.experimental.noalias.scope.decl(metadata !33878)
  call void @llvm.experimental.noalias.scope.decl(metadata !33881)
  call void @llvm.experimental.noalias.scope.decl(metadata !33883)
  %3597 = load i64, ptr %154, align 8, !range !2062, !alias.scope !33881, !noalias !33885, !noundef !1708
  %3598 = icmp eq i64 %3597, -1
  br i1 %3598, label %3600, label %3599

3599:                                             ; preds = %__rustc::__rust_alloc (.exit257)
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %155, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %153, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %157)
  br label %3602

3600:                                             ; preds = %__rustc::__rust_alloc (.exit257)
  %3601 = getelementptr inbounds nuw i8, ptr %155, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %3601, ptr noundef nonnull readonly align 8 dereferenceable(32) %153, i64 32, i1 false), !alias.scope !33885, !noalias !33881
  store i64 -1, ptr %155, align 8, !alias.scope !33878, !noalias !33886
  br label %3602

3602:                                             ; preds = %3600, %3599
  %3603 = getelementptr inbounds nuw i8, ptr %154, i64 72
  %3604 = load i64, ptr %3603, align 8, !range !1940, !alias.scope !33887, !noalias !33885, !noundef !1708
  %3605 = icmp ugt i64 %3604, 5
  br i1 %3605, label %3606, label %3639

3606:                                             ; preds = %3602
  %3607 = getelementptr inbounds nuw i8, ptr %154, i64 80
  %3608 = load ptr, ptr %3607, align 8, !alias.scope !33881, !noalias !33885, !nonnull !1708, !noundef !1708
  %3609 = mul i64 %3604, 3
  %3610 = add i64 %3609, -3
  %3611 = load i64, ptr %3560, align 8, !noalias !33890, !noundef !1708
  %3612 = call i64 @llvm.umin.i64(i64 %3610, i64 9223372036854775807)
  %3613 = call i64 @llvm.ssub.sat.i64(i64 %3611, i64 %3612)
  store i64 %3613, ptr %3560, align 8, !noalias !33890
  %3614 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %3615 = load i64, ptr %3614, align 8, !noalias !33890, !noundef !1708
  %3616 = icmp slt i64 %3613, %3615
  br i1 %3616, label %3617, label %.preheader2152

3617:                                             ; preds = %3606
  store i64 %3613, ptr %3614, align 8, !noalias !33890
  br label %.preheader2152

.preheader2152:                                   ; preds = %3617, %3606
  br label %3618

3618:                                             ; preds = %.preheader2152, %3621
  %3619 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !33890
  %3620 = icmp slt i64 %3619, 0
  br i1 %3620, label %3621, label %__rustc::__rust_dealloc (.exit258)

3621:                                             ; preds = %3618
  %3622 = add nsw i64 %3619, 1
  %3623 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %3619, i64 %3622 acq_rel acquire, align 8, !noalias !33890
  %3624 = extractvalue { i64, i1 } %3623, 1
  br i1 %3624, label %3625, label %3618

3625:                                             ; preds = %3621
  %3626 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3612 monotonic, align 8, !noalias !33890
  %3627 = call i64 @llvm.ssub.sat.i64(i64 %3626, i64 %3612)
  %3628 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !33890
  br label %3629

3629:                                             ; preds = %3632, %3625
  %3630 = phi i64 [ %3628, %3625 ], [ %3635, %3632 ]
  %3631 = icmp slt i64 %3627, %3630
  br i1 %3631, label %3632, label %3636

3632:                                             ; preds = %3629
  %3633 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3630, i64 %3627 monotonic monotonic, align 8, !noalias !33890
  %3634 = extractvalue { i64, i1 } %3633, 1
  %3635 = extractvalue { i64, i1 } %3633, 0
  br i1 %3634, label %3636, label %3629

3636:                                             ; preds = %3632, %3629
  %3637 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !33890
  br label %__rustc::__rust_dealloc (.exit258)

__rustc::__rust_dealloc (.exit258): ; preds = %3618, %3636
  %3638 = icmp ne i64 %3610, 0
  call void @llvm.assume(i1 %3638), !noalias !33890
  call void @free(ptr noundef nonnull %3608) #88, !noalias !33890
  br label %3639

3639:                                             ; preds = %__rustc::__rust_dealloc (.exit258), %3602
  %3640 = getelementptr inbounds nuw i8, ptr %154, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !33893)
  %3641 = load ptr, ptr %3640, align 8, !alias.scope !33896, !noalias !33885, !noundef !1708
  %3642 = icmp eq ptr %3641, null
  br i1 %3642, label %3647, label %3643

3643:                                             ; preds = %3639
  %3644 = atomicrmw sub ptr %3641, i64 1 release, align 8, !noalias !33897
  %3645 = icmp eq i64 %3644, 1
  br i1 %3645, label %3646, label %3647

3646:                                             ; preds = %3643
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %3640) #87
  br label %3647

3647:                                             ; preds = %3646, %3643, %3639
  call void @llvm.lifetime.end.p0(ptr nonnull %153)
  call void @llvm.lifetime.end.p0(ptr nonnull %154)
  %3648 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %3648, ptr noundef nonnull align 8 dereferenceable(96) %155, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %155)
  call void @llvm.lifetime.end.p0(ptr nonnull %156)
  br label %3303

3649:                                             ; preds = %3590
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %157) #89
          to label %3652 unwind label %679

3650:                                             ; preds = %3543
  %3651 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.13412714042204560522)(ptr noalias nofree noundef align 8 dereferenceable(56) %156) #89
          to label %3654 unwind label %679

3652:                                             ; preds = %3654, %3649, %3304, %3220, %158
  %3653 = phi { ptr, i32 } [ %3655, %3654 ], [ %185, %158 ], [ %3221, %3304 ], [ %3221, %3220 ], [ %3591, %3649 ]
  resume { ptr, i32 } %3653

3654:                                             ; preds = %3650, %3531, %3527, %3515, %3511, %3496, %3332, %159, %158
  %3655 = phi { ptr, i32 } [ %185, %158 ], [ %3651, %3650 ], [ %3333, %3332 ], [ %3333, %3496 ], [ %160, %159 ], [ %3512, %3511 ], [ %3512, %3515 ], [ %3528, %3531 ], [ %3528, %3527 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %157) #89
          to label %3652 unwind label %679
}

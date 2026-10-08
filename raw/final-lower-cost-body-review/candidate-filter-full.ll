define void @purrdf_sparql_eval::expr::eval_filter::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef align 16 dereferenceable(1248) %4) unnamed_addr #8 personality ptr @rust_eh_personality !guid !23098 {
  %6 = alloca [96 x i8], align 16
  %7 = alloca [96 x i8], align 16
  %8 = alloca [24 x i8], align 8
  %9 = alloca [23 x i8], align 1
  %10 = alloca [24 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [24 x i8], align 8
  %13 = alloca [24 x i8], align 8
  %14 = alloca [24 x i8], align 8
  %15 = alloca [24 x i8], align 8
  %16 = alloca [32 x i8], align 8
  %17 = alloca [8 x i8], align 8
  %18 = alloca [8 x i8], align 8
  %19 = alloca [96 x i8], align 16
  %20 = alloca [96 x i8], align 16
  %21 = alloca [40 x i8], align 8
  %22 = alloca [24 x i8], align 8
  %23 = alloca [48 x i8], align 8
  %24 = alloca [80 x i8], align 8
  %25 = alloca [16 x i8], align 8
  %26 = alloca [40 x i8], align 8
  %27 = alloca [16 x i8], align 8
  %28 = alloca [40 x i8], align 8
  %29 = alloca [24 x i8], align 8
  %30 = alloca [24 x i8], align 8
  %31 = alloca [24 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [24 x i8], align 8
  %34 = alloca [24 x i8], align 8
  %35 = alloca [8 x i8], align 8
  %36 = alloca [32 x i8], align 8
  %37 = alloca [160 x i8], align 8
  %38 = alloca [152 x i8], align 8
  %39 = alloca [32 x i8], align 8
  %40 = alloca [8 x i8], align 8
  %41 = alloca [32 x i8], align 8
  %42 = alloca [32 x i8], align 8
  %43 = alloca [24 x i8], align 8
  %44 = alloca [96 x i8], align 16
  %45 = alloca [24 x i8], align 8
  %46 = alloca [160 x i8], align 8
  %47 = alloca [32 x i8], align 8
  %48 = alloca [24 x i8], align 8
  %49 = alloca [24 x i8], align 8
  %50 = alloca [32 x i8], align 8
  %51 = alloca [48 x i8], align 16
  %52 = alloca [224 x i8], align 8
  %53 = alloca [24 x i8], align 8
  %54 = alloca [24 x i8], align 8
  %55 = alloca [32 x i8], align 8
  %56 = alloca [104 x i8], align 8
  %57 = alloca [96 x i8], align 8
  %58 = alloca [96 x i8], align 8
  %59 = alloca [24 x i8], align 8
  %60 = alloca [24 x i8], align 8
  %61 = alloca [80 x i8], align 16
  %62 = alloca [24 x i8], align 8
  %63 = alloca [40 x i8], align 8
  %64 = alloca [32 x i8], align 8
  %65 = alloca [32 x i8], align 8
  %66 = alloca [24 x i8], align 8
  %67 = alloca [16 x i8], align 8
  %68 = alloca [80 x i8], align 16
  %69 = alloca [24 x i8], align 8
  %70 = alloca [200 x i8], align 8
  %71 = alloca [24 x i8], align 8
  %72 = alloca [248 x i8], align 8
  %73 = alloca [240 x i8], align 8
  %74 = alloca [208 x i8], align 8
  %75 = alloca [32 x i8], align 8
  %76 = alloca [24 x i8], align 8
  %77 = alloca [32 x i8], align 8
  %78 = alloca [256 x i8], align 16
  %79 = alloca [208 x i8], align 16
  %80 = alloca [24 x i8], align 8
  %81 = alloca [8 x i8], align 8
  %82 = alloca [24 x i8], align 8
  %83 = alloca [216 x i8], align 8
  %84 = alloca [200 x i8], align 8
  %85 = alloca [8 x i8], align 8
  %86 = alloca [112 x i8], align 16
  %87 = alloca [104 x i8], align 8
  %88 = alloca [104 x i8], align 8
  %89 = alloca [32 x i8], align 8
  %90 = alloca [32 x i8], align 8
  %91 = alloca [104 x i8], align 8
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %91, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %89)
  call void @llvm.lifetime.start.p0(ptr nonnull %88)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %86, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %3)
          to label %92 unwind label %2541, !inline_history !13414

92:                                               ; preds = %5
  %93 = load i64, ptr %86, align 16, !range !1739, !noundef !1740
  %94 = trunc nuw i64 %93 to i1
  br i1 %94, label %95, label %178

95:                                               ; preds = %92
  %96 = getelementptr inbounds nuw i8, ptr %86, i64 16
  %97 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %97, ptr noundef nonnull align 16 dereferenceable(96) %96, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %88)
  call void @llvm.lifetime.end.p0(ptr nonnull %89)
  %98 = getelementptr inbounds nuw i8, ptr %91, i64 72
  %99 = load i64, ptr %98, align 8, !range !1778, !noundef !1740
  %100 = icmp ugt i64 %99, 5
  br i1 %100, label %101, label %135

101:                                              ; preds = %95
  %102 = getelementptr inbounds nuw i8, ptr %91, i64 80
  %103 = load ptr, ptr %102, align 8, !nonnull !1740, !noundef !1740
  %104 = mul i64 %99, 3
  %105 = add i64 %104, -3
  %106 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %107 = load i64, ptr %106, align 8, !noalias !23099, !noundef !1740
  %108 = tail call i64 @llvm.umin.i64(i64 %105, i64 9223372036854775807)
  %109 = tail call i64 @llvm.ssub.sat.i64(i64 %107, i64 %108)
  store i64 %109, ptr %106, align 8, !noalias !23099
  %110 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %111 = load i64, ptr %110, align 8, !noalias !23099, !noundef !1740
  %112 = icmp slt i64 %109, %111
  br i1 %112, label %113, label %.preheader1460

113:                                              ; preds = %101
  store i64 %109, ptr %110, align 8, !noalias !23099
  br label %.preheader1460

.preheader1460:                                   ; preds = %113, %101
  br label %114

114:                                              ; preds = %.preheader1460, %117
  %115 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23099
  %116 = icmp slt i64 %115, 0
  br i1 %116, label %117, label %__rustc::__rust_dealloc (.exit)

117:                                              ; preds = %114
  %118 = add nsw i64 %115, 1
  %119 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %115, i64 %118 acq_rel acquire, align 8, !noalias !23099
  %120 = extractvalue { i64, i1 } %119, 1
  br i1 %120, label %121, label %114

121:                                              ; preds = %117
  %122 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %108 monotonic, align 8, !noalias !23099
  %123 = tail call i64 @llvm.ssub.sat.i64(i64 %122, i64 %108)
  %124 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23099
  br label %125

125:                                              ; preds = %128, %121
  %126 = phi i64 [ %124, %121 ], [ %131, %128 ]
  %127 = icmp slt i64 %123, %126
  br i1 %127, label %128, label %132

128:                                              ; preds = %125
  %129 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %126, i64 %123 monotonic monotonic, align 8, !noalias !23099
  %130 = extractvalue { i64, i1 } %129, 1
  %131 = extractvalue { i64, i1 } %129, 0
  br i1 %130, label %132, label %125

132:                                              ; preds = %128, %125
  %133 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23099
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %114, %132
  %134 = icmp ne i64 %105, 0
  tail call void @llvm.assume(i1 %134), !noalias !23099
  tail call void @free(ptr noundef nonnull %103) #93, !noalias !23099
  br label %135

135:                                              ; preds = %__rustc::__rust_dealloc (.exit), %95
  %136 = load i64, ptr %91, align 8, !range !2059, !noundef !1740
  %137 = icmp sgt i64 %136, 0
  br i1 %137, label %138, label %170

138:                                              ; preds = %135
  %139 = getelementptr inbounds nuw i8, ptr %91, i64 8
  %140 = load ptr, ptr %139, align 8, !nonnull !1740, !noundef !1740
  %141 = mul nuw i64 %136, 3
  %142 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %143 = load i64, ptr %142, align 8, !noalias !23104, !noundef !1740
  %144 = tail call i64 @llvm.umin.i64(i64 %141, i64 9223372036854775807)
  %145 = tail call i64 @llvm.ssub.sat.i64(i64 %143, i64 %144)
  store i64 %145, ptr %142, align 8, !noalias !23104
  %146 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %147 = load i64, ptr %146, align 8, !noalias !23104, !noundef !1740
  %148 = icmp slt i64 %145, %147
  br i1 %148, label %149, label %.preheader1459

149:                                              ; preds = %138
  store i64 %145, ptr %146, align 8, !noalias !23104
  br label %.preheader1459

.preheader1459:                                   ; preds = %149, %138
  br label %150

150:                                              ; preds = %.preheader1459, %153
  %151 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23104
  %152 = icmp slt i64 %151, 0
  br i1 %152, label %153, label %__rustc::__rust_dealloc (.exit134)

153:                                              ; preds = %150
  %154 = add nsw i64 %151, 1
  %155 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %151, i64 %154 acq_rel acquire, align 8, !noalias !23104
  %156 = extractvalue { i64, i1 } %155, 1
  br i1 %156, label %157, label %150

157:                                              ; preds = %153
  %158 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %144 monotonic, align 8, !noalias !23104
  %159 = tail call i64 @llvm.ssub.sat.i64(i64 %158, i64 %144)
  %160 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23104
  br label %161

161:                                              ; preds = %164, %157
  %162 = phi i64 [ %160, %157 ], [ %167, %164 ]
  %163 = icmp slt i64 %159, %162
  br i1 %163, label %164, label %168

164:                                              ; preds = %161
  %165 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %162, i64 %159 monotonic monotonic, align 8, !noalias !23104
  %166 = extractvalue { i64, i1 } %165, 1
  %167 = extractvalue { i64, i1 } %165, 0
  br i1 %166, label %168, label %161

168:                                              ; preds = %164, %161
  %169 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23104
  br label %__rustc::__rust_dealloc (.exit134)

__rustc::__rust_dealloc (.exit134): ; preds = %150, %168
  tail call void @free(ptr noundef nonnull %140) #93, !noalias !23104
  br label %170

170:                                              ; preds = %__rustc::__rust_dealloc (.exit134), %135
  %171 = getelementptr inbounds nuw i8, ptr %91, i64 96
  %172 = load ptr, ptr %171, align 8, !noundef !1740
  %173 = icmp eq ptr %172, null
  br i1 %173, label %2536, label %174

174:                                              ; preds = %170
  %175 = atomicrmw sub ptr %172, i64 1 release, align 8, !noalias !23105
  %176 = icmp eq i64 %175, 1
  br i1 %176, label %177, label %2536

177:                                              ; preds = %174
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %171) #92
  br label %2536

178:                                              ; preds = %92
  %179 = getelementptr inbounds nuw i8, ptr %86, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %88, ptr noundef nonnull align 8 dereferenceable(96) %179, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %89, ptr noalias nofree noundef align 8 dereferenceable(104) %91, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %88)
          to label %180 unwind label %2541

180:                                              ; preds = %178
  %181 = load i64, ptr %89, align 8, !range !2059, !noundef !1740
  %182 = icmp eq i64 %181, -1
  br i1 %182, label %2501, label %183

183:                                              ; preds = %180
  call void @llvm.lifetime.start.p0(ptr nonnull %90)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %90, ptr noundef nonnull align 8 dereferenceable(32) %89, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %88)
  call void @llvm.lifetime.end.p0(ptr nonnull %89)
  call void @llvm.lifetime.start.p0(ptr nonnull %87)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %87, ptr noundef nonnull align 8 dereferenceable(104) %91, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23112)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23115)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !23117)
  call void @llvm.lifetime.start.p0(ptr nonnull %85), !noalias !23119
  %184 = getelementptr inbounds nuw i8, ptr %90, i64 24
  %185 = load ptr, ptr %184, align 8, !alias.scope !23115, !noalias !23123, !nonnull !1740, !noundef !1740
  %186 = atomicrmw add ptr %185, i64 1 monotonic, align 8, !noalias !23119
  %187 = icmp slt i64 %186, 0
  br i1 %187, label %190, label %188

188:                                              ; preds = %183
  store ptr %185, ptr %85, align 8, !noalias !23119
; invoke <purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop
  %189 = invoke fastcc noundef zeroext i1 @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop(ptr noundef nonnull align 16 dereferenceable(1248) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %197 unwind label %192, !noalias !23124

190:                                              ; preds = %183
  tail call void @llvm.trap()
  unreachable

191:                                              ; preds = %225
  br i1 %227, label %2489, label %2421

192:                                              ; preds = %2332, %218, %188
  %193 = phi i8 [ 1, %188 ], [ 1, %218 ], [ %2255, %2332 ]
  %194 = landingpad { ptr, i32 }
          cleanup
  br label %2489

195:                                              ; preds = %2258
  %196 = landingpad { ptr, i32 }
          cleanup
  br label %2421

197:                                              ; preds = %188
  %198 = getelementptr inbounds nuw i8, ptr %4, i64 472
  %199 = load i8, ptr %198, align 8, !range !3730
  %200 = icmp eq i8 %199, 2
  %201 = select i1 %189, i1 %200, i1 false
  br i1 %201, label %202, label %218

202:                                              ; preds = %197
  %203 = getelementptr inbounds nuw i8, ptr %4, i64 616
  %204 = load ptr, ptr %203, align 8, !noalias !23124, !noundef !1740
  %205 = icmp eq ptr %204, null
  br i1 %205, label %218, label %206

206:                                              ; preds = %202
  %207 = getelementptr inbounds nuw i8, ptr %204, i64 24
  %208 = load i64, ptr %207, align 8, !noalias !23125
  %209 = getelementptr inbounds nuw i8, ptr %204, i64 48
  %210 = icmp ult i64 %208, -2
  br i1 %210, label %218, label %211

211:                                              ; preds = %206
  %212 = getelementptr inbounds nuw i8, ptr %204, i64 32
  %213 = load i64, ptr %212, align 8, !noalias !23125
  %214 = icmp ult i64 %213, -2
  br i1 %214, label %218, label %215

215:                                              ; preds = %211
  %216 = load i64, ptr %209, align 8, !noalias !23125
  %217 = icmp ugt i64 %216, -3
  br label %218

218:                                              ; preds = %215, %211, %206, %202, %197
  %219 = phi i1 [ false, %197 ], [ false, %206 ], [ true, %202 ], [ false, %211 ], [ %217, %215 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %84), !noalias !23119
  %220 = getelementptr inbounds nuw i8, ptr %90, i64 16
  %221 = load i64, ptr %220, align 8, !alias.scope !23115, !noalias !23123, !noundef !1740
  %222 = icmp ult i64 %221, 230584300921369396
  tail call void @llvm.assume(i1 %222)
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::for_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(200) %84, ptr noundef nonnull align 16 dereferenceable(1248) %4, i1 noundef zeroext %219, i64 noundef %221)
          to label %223 unwind label %192

223:                                              ; preds = %218
; invoke purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>
  %224 = invoke fastcc noundef nonnull ptr @purrdf_sparql_eval::vm::program_at::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 16 dereferenceable(1248) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2)
          to label %233 unwind label %229, !noalias !23124

225:                                              ; preds = %562, %229
  %226 = phi i8 [ %230, %229 ], [ %563, %562 ]
  %227 = phi i1 [ %231, %229 ], [ %564, %562 ]
  %228 = phi { ptr, i32 } [ %232, %229 ], [ %565, %562 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %84)
          to label %191 unwind label %583

229:                                              ; preds = %2256, %2254, %233, %223
  %230 = phi i8 [ %2255, %2254 ], [ %572, %2256 ], [ 1, %233 ], [ 1, %223 ]
  %231 = phi i1 [ true, %2254 ], [ false, %2256 ], [ true, %233 ], [ true, %223 ]
  %232 = landingpad { ptr, i32 }
          cleanup
  br label %225

233:                                              ; preds = %223
  call void @llvm.lifetime.start.p0(ptr nonnull %83), !noalias !23119
  %234 = load ptr, ptr %85, align 8, !noalias !23119, !nonnull !1740, !noundef !1740
  %235 = getelementptr inbounds nuw i8, ptr %234, i64 16
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::link::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(216) %83, ptr noundef nonnull %224, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %235, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %236 unwind label %229, !noalias !23124

236:                                              ; preds = %233
  call void @llvm.lifetime.start.p0(ptr nonnull %82), !noalias !23119
  br i1 %219, label %585, label %237

237:                                              ; preds = %236
  call void @llvm.lifetime.start.p0(ptr nonnull %66), !noalias !23119
  store i64 0, ptr %66, align 8, !noalias !23119
  %238 = getelementptr inbounds nuw i8, ptr %66, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %238, align 8, !noalias !23119
  %239 = getelementptr inbounds nuw i8, ptr %66, i64 16
  store i64 0, ptr %239, align 8, !noalias !23119
  %240 = getelementptr inbounds nuw i8, ptr %90, i64 8
  %241 = load ptr, ptr %240, align 8, !alias.scope !23115, !noalias !23123, !nonnull !1740, !noundef !1740
  %242 = load i64, ptr %90, align 8, !range !1835, !alias.scope !23115, !noalias !23123, !noundef !1740
  %243 = mul nuw nsw i64 %221, 40
  %244 = getelementptr inbounds nuw i8, ptr %241, i64 %243
  call void @llvm.lifetime.start.p0(ptr nonnull %65), !noalias !23119
  store ptr %241, ptr %65, align 8, !noalias !23119
  %245 = getelementptr inbounds nuw i8, ptr %65, i64 8
  %246 = getelementptr inbounds nuw i8, ptr %65, i64 16
  store i64 %242, ptr %246, align 8, !noalias !23119
  %247 = getelementptr inbounds nuw i8, ptr %65, i64 24
  store ptr %244, ptr %247, align 8, !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %64)
  %248 = icmp eq i64 %221, 0
  br i1 %248, label %.loopexit195, label %249

249:                                              ; preds = %237
  %250 = getelementptr inbounds nuw i8, ptr %63, i64 8
  %251 = getelementptr inbounds nuw i8, ptr %63, i64 16
  %252 = getelementptr inbounds nuw i8, ptr %64, i64 8
  %253 = getelementptr inbounds nuw i8, ptr %20, i64 8
  %254 = getelementptr inbounds nuw i8, ptr %20, i64 12
  %255 = getelementptr inbounds nuw i8, ptr %19, i64 8
  %256 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %257 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %265

258:                                              ; preds = %581, %457
  %259 = phi ptr [ %582, %581 ], [ %362, %457 ]
  %260 = phi { ptr, i32 } [ %579, %581 ], [ %455, %457 ]
  %261 = shl i64 %271, 3
  %262 = add i64 %261, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %259, i64 noundef %262, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !1740
  br label %263

263:                                              ; preds = %578, %454, %258
  %264 = phi { ptr, i32 } [ %455, %454 ], [ %579, %578 ], [ %260, %258 ]
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %65) #90, !noalias !23124
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %66) #90, !noalias !23124
  br label %562

265:                                              ; preds = %458, %249
  %266 = phi ptr [ inttoptr (i64 8 to ptr), %249 ], [ %459, %458 ]
  %267 = phi i64 [ 0, %249 ], [ %460, %458 ]
  %268 = phi ptr [ inttoptr (i64 8 to ptr), %249 ], [ %461, %458 ]
  %269 = phi ptr [ %241, %249 ], [ %270, %458 ]
  %270 = getelementptr inbounds nuw i8, ptr %269, i64 40
  %271 = load i64, ptr %269, align 8, !noalias !23131
  %272 = getelementptr inbounds nuw i8, ptr %269, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %64, ptr noundef nonnull align 8 dereferenceable(32) %272, i64 32, i1 false), !noalias !23131
  %273 = icmp eq i64 %271, 0
  br i1 %273, label %.loopexit195, label %274

274:                                              ; preds = %265
  call void @llvm.lifetime.start.p0(ptr nonnull %63), !noalias !23119
  store i64 %271, ptr %63, align 8, !noalias !23119
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %250, ptr noundef nonnull align 8 dereferenceable(32) %64, i64 32, i1 false), !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %62), !noalias !23119
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %62, ptr noalias nofree noundef align 8 dereferenceable(200) %84, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %356 unwind label %578, !noalias !23124

.loopexit195:                                     ; preds = %458, %265, %237
  %275 = phi ptr [ %241, %237 ], [ %244, %458 ], [ %270, %265 ]
  store ptr %275, ptr %245, align 8
  br label %276

276:                                              ; preds = %569, %.loopexit195
  %277 = phi ptr [ %270, %569 ], [ %275, %.loopexit195 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %64)
  %278 = ptrtoint ptr %244 to i64
  %279 = ptrtoint ptr %277 to i64
  %280 = sub nuw i64 %278, %279
  %281 = udiv exact i64 %280, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23134), !noalias !23124
  %282 = icmp eq ptr %244, %277
  br i1 %282, label %.loopexit190, label %.preheader189

.preheader189:                                    ; preds = %276
  %283 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %284 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %285

285:                                              ; preds = %.preheader189, %323
  %286 = phi i64 [ %288, %323 ], [ 0, %.preheader189 ]
  %287 = getelementptr inbounds nuw [40 x i8], ptr %277, i64 %286
  %288 = add nuw nsw i64 %286, 1
  %289 = load i64, ptr %287, align 8, !range !1778, !alias.scope !23137, !noalias !23140, !noundef !1740
  %290 = icmp ugt i64 %289, 5
  br i1 %290, label %291, label %323

291:                                              ; preds = %285
  %292 = getelementptr i8, ptr %287, i64 8
  %293 = load ptr, ptr %292, align 8, !alias.scope !23134, !noalias !23140, !nonnull !1740, !noundef !1740
  %294 = shl i64 %289, 3
  %295 = add i64 %294, -8
  %296 = load i64, ptr %283, align 8, !noalias !23145, !noundef !1740
  %297 = call i64 @llvm.umin.i64(i64 %295, i64 9223372036854775807)
  %298 = call i64 @llvm.ssub.sat.i64(i64 %296, i64 %297)
  store i64 %298, ptr %283, align 8, !noalias !23145
  %299 = load i64, ptr %284, align 8, !noalias !23145, !noundef !1740
  %300 = icmp slt i64 %298, %299
  br i1 %300, label %301, label %.preheader1692

301:                                              ; preds = %291
  store i64 %298, ptr %284, align 8, !noalias !23145
  br label %.preheader1692

.preheader1692:                                   ; preds = %301, %291
  br label %302

302:                                              ; preds = %.preheader1692, %305
  %303 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23145
  %304 = icmp slt i64 %303, 0
  br i1 %304, label %305, label %__rustc::__rust_dealloc (.exit135)

305:                                              ; preds = %302
  %306 = add nsw i64 %303, 1
  %307 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %303, i64 %306 acq_rel acquire, align 8, !noalias !23145
  %308 = extractvalue { i64, i1 } %307, 1
  br i1 %308, label %309, label %302

309:                                              ; preds = %305
  %310 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %297 monotonic, align 8, !noalias !23145
  %311 = call i64 @llvm.ssub.sat.i64(i64 %310, i64 %297)
  %312 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23145
  br label %313

313:                                              ; preds = %316, %309
  %314 = phi i64 [ %312, %309 ], [ %319, %316 ]
  %315 = icmp slt i64 %311, %314
  br i1 %315, label %316, label %320

316:                                              ; preds = %313
  %317 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %314, i64 %311 monotonic monotonic, align 8, !noalias !23145
  %318 = extractvalue { i64, i1 } %317, 1
  %319 = extractvalue { i64, i1 } %317, 0
  br i1 %318, label %320, label %313

320:                                              ; preds = %316, %313
  %321 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23145
  br label %__rustc::__rust_dealloc (.exit135)

__rustc::__rust_dealloc (.exit135): ; preds = %302, %320
  %322 = icmp ne i64 %295, 0
  call void @llvm.assume(i1 %322), !noalias !23145
  call void @free(ptr noundef nonnull %293) #93, !noalias !23145
  br label %323

323:                                              ; preds = %__rustc::__rust_dealloc (.exit135), %285
  %324 = icmp eq i64 %288, %281
  br i1 %324, label %.loopexit190, label %285

.loopexit190:                                     ; preds = %323, %276
  %325 = icmp eq i64 %242, 0
  br i1 %325, label %570, label %326

326:                                              ; preds = %.loopexit190
  %327 = mul nuw i64 %242, 40
  %328 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %329 = load i64, ptr %328, align 8, !noalias !23140, !noundef !1740
  %330 = call i64 @llvm.umin.i64(i64 %327, i64 9223372036854775807)
  %331 = call i64 @llvm.ssub.sat.i64(i64 %329, i64 %330)
  store i64 %331, ptr %328, align 8, !noalias !23140
  %332 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %333 = load i64, ptr %332, align 8, !noalias !23140, !noundef !1740
  %334 = icmp slt i64 %331, %333
  br i1 %334, label %335, label %.preheader1691

335:                                              ; preds = %326
  store i64 %331, ptr %332, align 8, !noalias !23140
  br label %.preheader1691

.preheader1691:                                   ; preds = %335, %326
  br label %336

336:                                              ; preds = %.preheader1691, %339
  %337 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23140
  %338 = icmp slt i64 %337, 0
  br i1 %338, label %339, label %__rustc::__rust_dealloc (.exit136)

339:                                              ; preds = %336
  %340 = add nsw i64 %337, 1
  %341 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %337, i64 %340 acq_rel acquire, align 8, !noalias !23140
  %342 = extractvalue { i64, i1 } %341, 1
  br i1 %342, label %343, label %336

343:                                              ; preds = %339
  %344 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %330 monotonic, align 8, !noalias !23140
  %345 = call i64 @llvm.ssub.sat.i64(i64 %344, i64 %330)
  %346 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23140
  br label %347

347:                                              ; preds = %350, %343
  %348 = phi i64 [ %346, %343 ], [ %353, %350 ]
  %349 = icmp slt i64 %345, %348
  br i1 %349, label %350, label %354

350:                                              ; preds = %347
  %351 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %348, i64 %345 monotonic monotonic, align 8, !noalias !23140
  %352 = extractvalue { i64, i1 } %351, 1
  %353 = extractvalue { i64, i1 } %351, 0
  br i1 %352, label %354, label %347

354:                                              ; preds = %350, %347
  %355 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23140
  br label %__rustc::__rust_dealloc (.exit136)

__rustc::__rust_dealloc (.exit136): ; preds = %336, %354
  call void @free(ptr noundef nonnull %241) #93, !noalias !23140
  br label %570

356:                                              ; preds = %274
  %357 = load i8, ptr %62, align 8, !range !1743, !noalias !23119, !noundef !1740
  %358 = icmp eq i8 %357, -1
  br i1 %358, label %359, label %393

359:                                              ; preds = %356
  call void @llvm.lifetime.end.p0(ptr nonnull %62), !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %61)
  %360 = add i64 %271, -1
  %361 = icmp ugt i64 %360, 4
  %362 = load ptr, ptr %250, align 8, !noalias !23119
  %363 = load i64, ptr %251, align 8, !noalias !23119
  %364 = add i64 %363, -1
  %365 = select i1 %361, i64 %364, i64 %360
  %366 = select i1 %361, ptr %362, ptr %250
  %367 = load ptr, ptr %85, align 8, !noalias !23119, !nonnull !1740, !noundef !1740
  %368 = getelementptr inbounds nuw i8, ptr %367, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !23148
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %20, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %83, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %366, i64 noundef range(i64 0, 1152921504606846976) %365, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %368, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %369 unwind label %578, !inline_history !23155

369:                                              ; preds = %359
  %370 = load i64, ptr %20, align 16, !range !2527, !noalias !23148, !noundef !1740
  %371 = icmp eq i64 %370, -1
  %372 = load i32, ptr %253, align 8, !noalias !23148
  %373 = load i32, ptr %254, align 4, !noalias !23148
  br i1 %371, label %379, label %374

374:                                              ; preds = %369
  %375 = getelementptr inbounds nuw i8, ptr %20, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %61, ptr noundef nonnull align 16 dereferenceable(80) %375, i64 80, i1 false), !noalias !23156
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !23148
  %376 = trunc i32 %372 to i8
  %377 = lshr i32 %372, 8
  %378 = trunc nuw i32 %377 to i24
  br label %399

379:                                              ; preds = %369
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !23148
  %380 = icmp eq i32 %372, 2
  br i1 %380, label %381, label %382

381:                                              ; preds = %379
  call void @llvm.lifetime.end.p0(ptr nonnull %61)
  br label %416

382:                                              ; preds = %379
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !23148
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %19, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i32 noundef %372, i32 noundef %373)
          to label %383 unwind label %578, !inline_history !23155

383:                                              ; preds = %382
  %384 = load i64, ptr %19, align 16, !range !2527, !noalias !23148, !noundef !1740
  %385 = icmp eq i64 %384, -1
  %386 = load i8, ptr %255, align 8, !noalias !23148
  br i1 %385, label %413, label %387

387:                                              ; preds = %383
  %388 = getelementptr inbounds nuw i8, ptr %19, i64 9
  %389 = load i24, ptr %388, align 1, !noalias !23156
  %390 = getelementptr inbounds nuw i8, ptr %19, i64 12
  %391 = load i32, ptr %390, align 4, !noalias !23156
  %392 = getelementptr inbounds nuw i8, ptr %19, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %61, ptr noundef nonnull align 16 dereferenceable(80) %392, i64 80, i1 false), !noalias !23156
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !23148
  br label %399

393:                                              ; preds = %356
  store ptr %270, ptr %245, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %62), !noalias !23119
  %394 = icmp ugt i64 %271, 5
  br i1 %394, label %395, label %569

395:                                              ; preds = %393
  %396 = load ptr, ptr %250, align 8, !nonnull !1740, !noundef !1740
  %397 = shl i64 %271, 3
  %398 = add i64 %397, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %396, i64 noundef %398, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !23157
  br label %569

399:                                              ; preds = %387, %374
  %400 = phi i24 [ %378, %374 ], [ %389, %387 ]
  %401 = phi i8 [ %376, %374 ], [ %386, %387 ]
  %402 = phi i32 [ %373, %374 ], [ %391, %387 ]
  %403 = phi i64 [ %370, %374 ], [ %384, %387 ]
  %404 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %400, ptr %404, align 1, !noalias !23160
  %405 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %402, ptr %405, align 4, !noalias !23160
  %406 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %406, ptr noundef nonnull align 16 dereferenceable(80) %61, i64 80, i1 false), !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %61)
  %407 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %403, ptr %407, align 16, !alias.scope !23112, !noalias !23160
  %408 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %401, ptr %408, align 8, !alias.scope !23112, !noalias !23160
  store i64 1, ptr %0, align 16, !alias.scope !23112, !noalias !23160
  %409 = icmp ugt i64 %271, 5
  br i1 %409, label %410, label %469

410:                                              ; preds = %399
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %362) ]
  %411 = shl i64 %271, 3
  %412 = add i64 %411, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %362, i64 noundef %412, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !23161
  br label %469

413:                                              ; preds = %383
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !23148
  call void @llvm.lifetime.end.p0(ptr nonnull %61)
  %414 = and i8 %386, 1
  %415 = icmp eq i8 %414, 0
  br i1 %415, label %416, label %448

416:                                              ; preds = %413, %381
  %417 = icmp ugt i64 %271, 5
  br i1 %417, label %418, label %458

418:                                              ; preds = %416
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %362) ]
  %419 = shl i64 %271, 3
  %420 = add i64 %419, -8
  %421 = load i64, ptr %256, align 8, !noalias !23164, !noundef !1740
  %422 = call i64 @llvm.umin.i64(i64 %420, i64 9223372036854775807)
  %423 = call i64 @llvm.ssub.sat.i64(i64 %421, i64 %422)
  store i64 %423, ptr %256, align 8, !noalias !23164
  %424 = load i64, ptr %257, align 8, !noalias !23164, !noundef !1740
  %425 = icmp slt i64 %423, %424
  br i1 %425, label %426, label %.preheader1695

426:                                              ; preds = %418
  store i64 %423, ptr %257, align 8, !noalias !23164
  br label %.preheader1695

.preheader1695:                                   ; preds = %426, %418
  br label %427

427:                                              ; preds = %.preheader1695, %430
  %428 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23164
  %429 = icmp slt i64 %428, 0
  br i1 %429, label %430, label %__rustc::__rust_dealloc (.exit137)

430:                                              ; preds = %427
  %431 = add nsw i64 %428, 1
  %432 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %428, i64 %431 acq_rel acquire, align 8, !noalias !23164
  %433 = extractvalue { i64, i1 } %432, 1
  br i1 %433, label %434, label %427

434:                                              ; preds = %430
  %435 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %422 monotonic, align 8, !noalias !23164
  %436 = call i64 @llvm.ssub.sat.i64(i64 %435, i64 %422)
  %437 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23164
  br label %438

438:                                              ; preds = %441, %434
  %439 = phi i64 [ %437, %434 ], [ %444, %441 ]
  %440 = icmp slt i64 %436, %439
  br i1 %440, label %441, label %445

441:                                              ; preds = %438
  %442 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %439, i64 %436 monotonic monotonic, align 8, !noalias !23164
  %443 = extractvalue { i64, i1 } %442, 1
  %444 = extractvalue { i64, i1 } %442, 0
  br i1 %443, label %445, label %438

445:                                              ; preds = %441, %438
  %446 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23164
  br label %__rustc::__rust_dealloc (.exit137)

__rustc::__rust_dealloc (.exit137): ; preds = %427, %445
  %447 = icmp ne i64 %420, 0
  call void @llvm.assume(i1 %447), !noalias !23164
  call void @free(ptr noundef nonnull %362) #93, !noalias !23164
  br label %458

448:                                              ; preds = %413
  call void @llvm.lifetime.start.p0(ptr nonnull %60)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %60, ptr noundef nonnull align 8 dereferenceable(24) %252, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !23167)
  %449 = load i64, ptr %66, align 8, !range !1835, !alias.scope !23167, !noalias !23170, !noundef !1740
  %450 = icmp eq i64 %267, %449
  br i1 %450, label %451, label %463

451:                                              ; preds = %448
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %66)
          to label %452 unwind label %454, !noalias !23170

452:                                              ; preds = %451
  %453 = load ptr, ptr %238, align 8, !alias.scope !23167, !noalias !23170
  br label %463

454:                                              ; preds = %451
  %455 = landingpad { ptr, i32 }
          cleanup
  store ptr %270, ptr %245, align 8
  %456 = icmp ugt i64 %271, 5
  br i1 %456, label %457, label %263

457:                                              ; preds = %454
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %362) ]
  br label %258

458:                                              ; preds = %463, %__rustc::__rust_dealloc (.exit137), %416
  %459 = phi ptr [ %266, %__rustc::__rust_dealloc (.exit137) ], [ %266, %416 ], [ %464, %463 ]
  %460 = phi i64 [ %267, %__rustc::__rust_dealloc (.exit137) ], [ %267, %416 ], [ %468, %463 ]
  %461 = phi ptr [ %268, %__rustc::__rust_dealloc (.exit137) ], [ %268, %416 ], [ %464, %463 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %64)
  call void @llvm.lifetime.start.p0(ptr nonnull %64)
  %462 = icmp eq ptr %270, %244
  br i1 %462, label %.loopexit195, label %265

463:                                              ; preds = %452, %448
  %464 = phi ptr [ %453, %452 ], [ %266, %448 ]
  %465 = getelementptr inbounds nuw [40 x i8], ptr %464, i64 %267
  store i64 %271, ptr %465, align 8, !noalias !23172
  %466 = getelementptr inbounds nuw i8, ptr %465, i64 8
  store ptr %362, ptr %466, align 8, !noalias !23172
  %467 = getelementptr inbounds nuw i8, ptr %465, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %467, ptr noundef nonnull align 8 dereferenceable(24) %60, i64 24, i1 false), !noalias !23172
  %468 = add i64 %267, 1
  store i64 %468, ptr %239, align 8, !alias.scope !23167, !noalias !23170
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  br label %458

469:                                              ; preds = %410, %399
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %64)
  %470 = ptrtoint ptr %244 to i64
  %471 = ptrtoint ptr %270 to i64
  %472 = sub nuw i64 %470, %471
  %473 = udiv exact i64 %472, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !23173), !noalias !23124
  %474 = icmp eq ptr %244, %270
  br i1 %474, label %.loopexit194, label %.preheader193

.preheader193:                                    ; preds = %469, %512
  %475 = phi i64 [ %477, %512 ], [ 0, %469 ]
  %476 = getelementptr inbounds nuw [40 x i8], ptr %270, i64 %475
  %477 = add nuw nsw i64 %475, 1
  %478 = load i64, ptr %476, align 8, !range !1778, !alias.scope !23176, !noalias !23179, !noundef !1740
  %479 = icmp ugt i64 %478, 5
  br i1 %479, label %480, label %512

480:                                              ; preds = %.preheader193
  %481 = getelementptr i8, ptr %476, i64 8
  %482 = load ptr, ptr %481, align 8, !alias.scope !23173, !noalias !23179, !nonnull !1740, !noundef !1740
  %483 = shl i64 %478, 3
  %484 = add i64 %483, -8
  %485 = load i64, ptr %256, align 8, !noalias !23184, !noundef !1740
  %486 = call i64 @llvm.umin.i64(i64 %484, i64 9223372036854775807)
  %487 = call i64 @llvm.ssub.sat.i64(i64 %485, i64 %486)
  store i64 %487, ptr %256, align 8, !noalias !23184
  %488 = load i64, ptr %257, align 8, !noalias !23184, !noundef !1740
  %489 = icmp slt i64 %487, %488
  br i1 %489, label %490, label %.preheader1694

490:                                              ; preds = %480
  store i64 %487, ptr %257, align 8, !noalias !23184
  br label %.preheader1694

.preheader1694:                                   ; preds = %490, %480
  br label %491

491:                                              ; preds = %.preheader1694, %494
  %492 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23184
  %493 = icmp slt i64 %492, 0
  br i1 %493, label %494, label %__rustc::__rust_dealloc (.exit138)

494:                                              ; preds = %491
  %495 = add nsw i64 %492, 1
  %496 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %492, i64 %495 acq_rel acquire, align 8, !noalias !23184
  %497 = extractvalue { i64, i1 } %496, 1
  br i1 %497, label %498, label %491

498:                                              ; preds = %494
  %499 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %486 monotonic, align 8, !noalias !23184
  %500 = call i64 @llvm.ssub.sat.i64(i64 %499, i64 %486)
  %501 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23184
  br label %502

502:                                              ; preds = %505, %498
  %503 = phi i64 [ %501, %498 ], [ %508, %505 ]
  %504 = icmp slt i64 %500, %503
  br i1 %504, label %505, label %509

505:                                              ; preds = %502
  %506 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %503, i64 %500 monotonic monotonic, align 8, !noalias !23184
  %507 = extractvalue { i64, i1 } %506, 1
  %508 = extractvalue { i64, i1 } %506, 0
  br i1 %507, label %509, label %502

509:                                              ; preds = %505, %502
  %510 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23184
  br label %__rustc::__rust_dealloc (.exit138)

__rustc::__rust_dealloc (.exit138): ; preds = %491, %509
  %511 = icmp ne i64 %484, 0
  call void @llvm.assume(i1 %511), !noalias !23184
  call void @free(ptr noundef nonnull %482) #93, !noalias !23184
  br label %512

512:                                              ; preds = %__rustc::__rust_dealloc (.exit138), %.preheader193
  %513 = icmp eq i64 %477, %473
  br i1 %513, label %.loopexit194, label %.preheader193

.loopexit194:                                     ; preds = %512, %469
  %514 = icmp eq i64 %242, 0
  br i1 %514, label %517, label %515

515:                                              ; preds = %.loopexit194
  %516 = mul nuw i64 %242, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %241, i64 noundef %516, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !23179
  br label %517

517:                                              ; preds = %515, %.loopexit194
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23187)
  call void @llvm.experimental.noalias.scope.decl(metadata !23190), !noalias !23124
  %518 = icmp eq i64 %267, 0
  br i1 %518, label %.loopexit192, label %.preheader191

.preheader191:                                    ; preds = %517, %556
  %519 = phi i64 [ %521, %556 ], [ 0, %517 ]
  %520 = getelementptr inbounds nuw [40 x i8], ptr %268, i64 %519
  %521 = add nuw nsw i64 %519, 1
  %522 = load i64, ptr %520, align 8, !range !1778, !alias.scope !23193, !noalias !23196, !noundef !1740
  %523 = icmp ugt i64 %522, 5
  br i1 %523, label %524, label %556

524:                                              ; preds = %.preheader191
  %525 = getelementptr i8, ptr %520, i64 8
  %526 = load ptr, ptr %525, align 8, !alias.scope !23190, !noalias !23196, !nonnull !1740, !noundef !1740
  %527 = shl i64 %522, 3
  %528 = add i64 %527, -8
  %529 = load i64, ptr %256, align 8, !noalias !23197, !noundef !1740
  %530 = call i64 @llvm.umin.i64(i64 %528, i64 9223372036854775807)
  %531 = call i64 @llvm.ssub.sat.i64(i64 %529, i64 %530)
  store i64 %531, ptr %256, align 8, !noalias !23197
  %532 = load i64, ptr %257, align 8, !noalias !23197, !noundef !1740
  %533 = icmp slt i64 %531, %532
  br i1 %533, label %534, label %.preheader1693

534:                                              ; preds = %524
  store i64 %531, ptr %257, align 8, !noalias !23197
  br label %.preheader1693

.preheader1693:                                   ; preds = %534, %524
  br label %535

535:                                              ; preds = %.preheader1693, %538
  %536 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23197
  %537 = icmp slt i64 %536, 0
  br i1 %537, label %538, label %__rustc::__rust_dealloc (.exit139)

538:                                              ; preds = %535
  %539 = add nsw i64 %536, 1
  %540 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %536, i64 %539 acq_rel acquire, align 8, !noalias !23197
  %541 = extractvalue { i64, i1 } %540, 1
  br i1 %541, label %542, label %535

542:                                              ; preds = %538
  %543 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %530 monotonic, align 8, !noalias !23197
  %544 = call i64 @llvm.ssub.sat.i64(i64 %543, i64 %530)
  %545 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23197
  br label %546

546:                                              ; preds = %549, %542
  %547 = phi i64 [ %545, %542 ], [ %552, %549 ]
  %548 = icmp slt i64 %544, %547
  br i1 %548, label %549, label %553

549:                                              ; preds = %546
  %550 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %547, i64 %544 monotonic monotonic, align 8, !noalias !23197
  %551 = extractvalue { i64, i1 } %550, 1
  %552 = extractvalue { i64, i1 } %550, 0
  br i1 %551, label %553, label %546

553:                                              ; preds = %549, %546
  %554 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23197
  br label %__rustc::__rust_dealloc (.exit139)

__rustc::__rust_dealloc (.exit139): ; preds = %535, %553
  %555 = icmp ne i64 %528, 0
  call void @llvm.assume(i1 %555), !noalias !23197
  call void @free(ptr noundef nonnull %526) #93, !noalias !23197
  br label %556

556:                                              ; preds = %__rustc::__rust_dealloc (.exit139), %.preheader191
  %557 = icmp eq i64 %521, %267
  br i1 %557, label %.loopexit192, label %.preheader191

.loopexit192:                                     ; preds = %556, %517
  %558 = load i64, ptr %66, align 8, !alias.scope !23187, !noalias !23124
  %559 = icmp eq i64 %558, 0
  br i1 %559, label %568, label %560

560:                                              ; preds = %.loopexit192
  %561 = mul nuw i64 %558, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %268, i64 noundef %561, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !23196
  br label %568

562:                                              ; preds = %2318, %2104, %611, %608, %604, %566, %263
  %563 = phi i8 [ 1, %566 ], [ 0, %263 ], [ %572, %2318 ], [ %572, %2104 ], [ 1, %611 ], [ 1, %604 ], [ 1, %608 ]
  %564 = phi i1 [ true, %566 ], [ true, %263 ], [ true, %2318 ], [ false, %2104 ], [ true, %611 ], [ true, %604 ], [ true, %608 ]
  %565 = phi { ptr, i32 } [ %567, %566 ], [ %264, %263 ], [ %2319, %2318 ], [ %2105, %2104 ], [ %605, %611 ], [ %605, %604 ], [ %605, %608 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %83) #90
          to label %225 unwind label %583, !noalias !23124

566:                                              ; preds = %2326, %1812, %585
  %567 = landingpad { ptr, i32 }
          cleanup
  br label %562

568:                                              ; preds = %560, %.loopexit192
  call void @llvm.lifetime.end.p0(ptr nonnull %66), !noalias !23119
  br label %2254

569:                                              ; preds = %395, %393
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !23119
  br label %276

570:                                              ; preds = %__rustc::__rust_dealloc (.exit136), %.loopexit190
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !23119
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %82, ptr noundef nonnull align 8 dereferenceable(24) %66, i64 24, i1 false), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %66), !noalias !23119
  br label %571

571:                                              ; preds = %2103, %570
  %572 = phi i8 [ 1, %2103 ], [ 0, %570 ]
  %573 = getelementptr inbounds nuw i8, ptr %4, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !23200)
  %574 = load ptr, ptr %573, align 8, !alias.scope !23200, !noalias !23203, !nonnull !1740, !noundef !1740
  %575 = getelementptr inbounds nuw i8, ptr %574, i64 40
  %576 = load atomic i32, ptr %575 acquire, align 4, !noalias !23205
  %577 = icmp eq i32 %576, 0
  br i1 %577, label %2106, label %2116

578:                                              ; preds = %382, %359, %274
  %579 = landingpad { ptr, i32 }
          cleanup
  store ptr %270, ptr %245, align 8
  %580 = icmp ugt i64 %271, 5
  br i1 %580, label %581, label %263

581:                                              ; preds = %578
  %582 = load ptr, ptr %250, align 8, !nonnull !1740, !noundef !1740
  br label %258

583:                                              ; preds = %2496, %2495, %2427, %2327, %1877, %611, %562, %225
  %584 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !23112
  unreachable

585:                                              ; preds = %236
  %586 = getelementptr inbounds nuw i8, ptr %84, i64 184
  %587 = load i64, ptr %586, align 8, !noundef !1740
  %588 = tail call noundef range(i64 0, 230584300921369396) i64 @llvm.umin.i64(i64 %587, i64 range(i64 0, 230584300921369396) %221)
  %589 = getelementptr inbounds nuw i8, ptr %90, i64 8
  %590 = load ptr, ptr %589, align 8, !alias.scope !23115, !noalias !23123, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %81), !noalias !23119
; invoke <purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
  %591 = invoke fastcc noundef ptr @<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot(ptr noundef nonnull align 16 dereferenceable(1248) %4, i64 noundef %588)
          to label %592 unwind label %566, !noalias !23124

592:                                              ; preds = %585
  store ptr %591, ptr %81, align 8, !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %79)
  call void @llvm.lifetime.start.p0(ptr nonnull %78), !noalias !23119
  %593 = getelementptr inbounds nuw i8, ptr %4, i64 616
  %594 = load ptr, ptr %593, align 8, !noundef !1740
  %595 = icmp eq ptr %594, null
  br i1 %595, label %614, label %596

596:                                              ; preds = %592
  %597 = getelementptr inbounds nuw i8, ptr %594, i64 16
  %598 = load i64, ptr %597, align 8
  %599 = icmp ugt i64 %598, -3
  br i1 %599, label %600, label %614

600:                                              ; preds = %596
  %601 = getelementptr inbounds nuw i8, ptr %594, i64 40
  %602 = load i64, ptr %601, align 8
  %603 = icmp ult i64 %602, -2
  br label %614

604:                                              ; preds = %2329, %2327, %1798, %661, %612
  %605 = phi { ptr, i32 } [ %2330, %2329 ], [ %662, %661 ], [ %613, %612 ], [ %2328, %2327 ], [ %1799, %1798 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23206)
  %606 = load ptr, ptr %81, align 8, !alias.scope !23206, !noalias !23124, !noundef !1740
  %607 = icmp eq ptr %606, null
  br i1 %607, label %562, label %608

608:                                              ; preds = %604
  %609 = atomicrmw sub ptr %606, i64 1 release, align 8, !noalias !23209
  %610 = icmp eq i64 %609, 1
  br i1 %610, label %611, label %562

611:                                              ; preds = %608
  fence acquire, !noalias !23124
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %81) #92
          to label %562 unwind label %583, !inline_history !2025

612:                                              ; preds = %625, %624
  %613 = landingpad { ptr, i32 }
          cleanup
  br label %604

614:                                              ; preds = %600, %596, %592
  %615 = phi i1 [ false, %592 ], [ true, %596 ], [ %603, %600 ]
  %616 = getelementptr inbounds nuw i8, ptr %4, i64 1234
  %617 = load i8, ptr %616, align 2, !range !1747, !noundef !1740
  %618 = trunc nuw i8 %617 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %77), !noalias !23119
  store ptr %4, ptr %77, align 8, !noalias !23119
  %619 = getelementptr inbounds nuw i8, ptr %77, i64 8
  store ptr %81, ptr %619, align 8, !noalias !23119
  %620 = getelementptr inbounds nuw i8, ptr %77, i64 16
  store ptr %84, ptr %620, align 8, !noalias !23119
  %621 = getelementptr inbounds nuw i8, ptr %77, i64 24
  store ptr %83, ptr %621, align 8, !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %76), !noalias !23119
  store ptr %590, ptr %76, align 8, !noalias !23119
  %622 = getelementptr inbounds nuw i8, ptr %76, i64 8
  store i64 %588, ptr %622, align 8, !noalias !23119
  %623 = getelementptr inbounds nuw i8, ptr %76, i64 16
  store ptr %85, ptr %623, align 8, !noalias !23119
  br i1 %615, label %625, label %624

624:                                              ; preds = %614
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %78, i1 noundef zeroext %618, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %590, i64 noundef range(i64 0, 230584300921369396) %588, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %77, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %76, ptr noundef nonnull align 8 %84)
          to label %626 unwind label %612, !inline_history !23212

625:                                              ; preds = %614
; invoke purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>
  invoke fastcc void @purrdf_sparql_eval::parallel::par_blocks_try_map_init::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::eval::EvalCtx, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint), purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#0}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#1}, purrdf_sparql_eval::expr::eval_filter_sequence<purrdf_core::ir::dataset::RdfDataset>::{closure#2}>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(256) %78, i1 noundef zeroext %618, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %590, i64 noundef range(i64 0, 230584300921369396) %588, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %77, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(24) %76, ptr noundef nonnull align 8 %84)
          to label %626 unwind label %612, !inline_history !23212

626:                                              ; preds = %625, %624
  call void @llvm.lifetime.end.p0(ptr nonnull %76), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %77), !noalias !23119
  %627 = load i64, ptr %78, align 16, !range !2059, !noalias !23119, !noundef !1740
  %628 = icmp eq i64 %627, -1
  br i1 %628, label %629, label %635

629:                                              ; preds = %626
  %630 = getelementptr inbounds nuw i8, ptr %78, i64 16
  %631 = getelementptr inbounds nuw i8, ptr %78, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %79, ptr noundef nonnull align 16 dereferenceable(64) %631, i64 64, i1 false), !noalias !23119
  %632 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %633 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %634 = load <4 x i64>, ptr %630, align 16, !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %78), !noalias !23119
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %632, ptr noundef nonnull align 16 dereferenceable(64) %79, i64 64, i1 false), !noalias !23160
  store <4 x i64> %634, ptr %633, align 16, !alias.scope !23112, !noalias !23160
  store i64 1, ptr %0, align 16, !alias.scope !23112, !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %79)
  br label %2320

635:                                              ; preds = %626
  %636 = getelementptr inbounds nuw i8, ptr %78, i64 8
  %637 = load i64, ptr %636, align 8, !noalias !23119
  %638 = getelementptr inbounds nuw i8, ptr %78, i64 16
  %639 = load i64, ptr %638, align 16, !noalias !23119
  %640 = getelementptr inbounds nuw i8, ptr %78, i64 24
  %641 = load i64, ptr %640, align 8, !noalias !23119
  %642 = getelementptr inbounds nuw i8, ptr %78, i64 32
  %643 = load i64, ptr %642, align 16, !noalias !23119
  %644 = getelementptr inbounds nuw i8, ptr %78, i64 40
  %645 = load i64, ptr %644, align 8, !noalias !23119
  %646 = getelementptr inbounds nuw i8, ptr %78, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(208) %79, ptr noundef nonnull align 16 dereferenceable(208) %646, i64 208, i1 false), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %78), !noalias !23119
  %647 = getelementptr inbounds nuw i8, ptr %72, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %72), !noalias !23119
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %647, ptr noundef nonnull align 16 dereferenceable(208) %79, i64 208, i1 false), !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %80), !noalias !23119
  store i64 %627, ptr %80, align 8, !noalias !23119
  %648 = getelementptr inbounds nuw i8, ptr %80, i64 8
  store i64 %637, ptr %648, align 8, !noalias !23119
  %649 = getelementptr inbounds nuw i8, ptr %80, i64 16
  store i64 %639, ptr %649, align 8, !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %79)
  call void @llvm.lifetime.start.p0(ptr nonnull %73), !noalias !23119
  %650 = add i64 %641, -3
  %651 = icmp ult i64 %650, -2
  %652 = select i1 %651, i64 %645, i64 %641
  %653 = add i64 %652, -1
  %654 = select i1 %651, i64 %641, i64 1
  %655 = select i1 %651, i64 1, i64 %645
  store i64 %654, ptr %72, align 8, !noalias !23119
  %656 = getelementptr inbounds nuw i8, ptr %72, i64 8
  store i64 %643, ptr %656, align 8, !noalias !23119
  %657 = getelementptr inbounds nuw i8, ptr %72, i64 16
  store i64 %655, ptr %657, align 8, !noalias !23119
  %658 = getelementptr inbounds nuw i8, ptr %72, i64 232
  store i64 0, ptr %658, align 8, !noalias !23119
  %659 = getelementptr inbounds nuw i8, ptr %72, i64 240
  store i64 %653, ptr %659, align 8, !noalias !23119
  %660 = inttoptr i64 %637 to ptr
; invoke <purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::RowCheckpoint, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(240) %73, ptr noalias nofree noundef align 8 captures(address) dereferenceable(248) %72)
          to label %663 unwind label %2329, !noalias !23124

661:                                              ; preds = %1748
  %662 = landingpad { ptr, i32 }
          cleanup
  br label %604

663:                                              ; preds = %635
  call void @llvm.lifetime.end.p0(ptr nonnull %72), !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %75), !noalias !23119
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %75, ptr noundef nonnull align 8 dereferenceable(32) %73, i64 32, i1 false), !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %74), !noalias !23119
  %664 = getelementptr inbounds nuw i8, ptr %73, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(208) %74, ptr noundef nonnull align 8 dereferenceable(208) %664, i64 208, i1 false), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %73), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23213)
  call void @llvm.experimental.noalias.scope.decl(metadata !23216)
  %665 = getelementptr inbounds nuw i8, ptr %84, i64 194
  %666 = load i8, ptr %665, align 2, !range !3730, !alias.scope !23213, !noalias !23218, !noundef !1740
  %667 = icmp ne i8 %666, 2
  %668 = load ptr, ptr %593, align 8, !alias.scope !23222, !noalias !23223
  %669 = icmp eq ptr %668, null
  %670 = select i1 %667, i1 true, i1 %669
  br i1 %670, label %671, label %674

671:                                              ; preds = %663
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(208) %74)
          to label %1758 unwind label %672, !noalias !23124

672:                                              ; preds = %671
  %673 = landingpad { ptr, i32 }
          cleanup
  br label %2327

674:                                              ; preds = %663
  call void @llvm.lifetime.start.p0(ptr nonnull %54), !noalias !23224
  store i64 %627, ptr %54, align 8, !noalias !23225
  %675 = getelementptr inbounds nuw i8, ptr %54, i64 8
  store ptr %660, ptr %675, align 8, !noalias !23225
  %676 = getelementptr inbounds nuw i8, ptr %54, i64 16
  store i64 %639, ptr %676, align 8, !noalias !23225
  %677 = load i64, ptr %74, align 8, !noalias !23226
  %678 = getelementptr inbounds nuw i8, ptr %74, i64 8
  %679 = load i64, ptr %678, align 8, !noalias !23226
  %680 = getelementptr inbounds nuw i8, ptr %74, i64 16
  %681 = load i64, ptr %680, align 8, !noalias !23226
  %682 = getelementptr inbounds nuw i8, ptr %74, i64 24
  %683 = getelementptr inbounds nuw i8, ptr %52, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %52), !noalias !23227
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(184) %683, ptr noundef nonnull align 8 dereferenceable(184) %682, i64 184, i1 false), !noalias !23124
  call void @llvm.experimental.noalias.scope.decl(metadata !23234)
  call void @llvm.lifetime.start.p0(ptr nonnull %53), !noalias !23227
  call void @llvm.experimental.noalias.scope.decl(metadata !23235)
  %684 = icmp ugt i64 %677, 2
  %685 = select i1 %684, i64 %681, i64 %677
  %686 = add i64 %685, -1
  %687 = select i1 %684, i64 %677, i64 1
  %688 = select i1 %684, i64 1, i64 %681
  store i64 %687, ptr %52, align 8, !alias.scope !23238, !noalias !23240
  %689 = getelementptr inbounds nuw i8, ptr %52, i64 8
  store i64 %679, ptr %689, align 8, !alias.scope !23238, !noalias !23240
  %690 = getelementptr inbounds nuw i8, ptr %52, i64 16
  store i64 %688, ptr %690, align 8, !alias.scope !23238, !noalias !23240
  %691 = getelementptr inbounds nuw i8, ptr %52, i64 208
  store i64 0, ptr %691, align 8, !alias.scope !23241, !noalias !23242
  %692 = getelementptr inbounds nuw i8, ptr %52, i64 216
  store i64 %686, ptr %692, align 8, !alias.scope !23241, !noalias !23242
; invoke <purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]> as core::iter::traits::iterator::Iterator>::collect::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %53, ptr noalias nofree noundef align 8 captures(address) dereferenceable(224) %52)
          to label %695 unwind label %693, !noalias !23240

693:                                              ; preds = %674
  %694 = landingpad { ptr, i32 }
          cleanup
  br label %1746

695:                                              ; preds = %674
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !23227
  %696 = getelementptr inbounds nuw i8, ptr %53, i64 8
  %697 = load ptr, ptr %696, align 8, !noalias !23227, !nonnull !1740, !noundef !1740
  %698 = getelementptr inbounds nuw i8, ptr %53, i64 16
  %699 = load i64, ptr %698, align 8, !noalias !23227, !noundef !1740
  %700 = mul nuw nsw i64 %699, 200
  %701 = getelementptr inbounds nuw i8, ptr %697, i64 %700
  %702 = icmp eq i64 %699, 0
  br i1 %702, label %772, label %.preheader188.preheader

.preheader188.preheader:                          ; preds = %695
  %xtraiter = and i64 %699, 3
  %703 = icmp ult i64 %699, 4
  br i1 %703, label %.preheader188.epil.preheader, label %.preheader188.preheader.new

.preheader188.preheader.new:                      ; preds = %.preheader188.preheader
  %unroll_iter = and i64 %699, -4
  br label %.preheader188

.preheader188:                                    ; preds = %748, %.preheader188.preheader.new
  %704 = phi i64 [ 0, %.preheader188.preheader.new ], [ %751, %748 ]
  %705 = phi i64 [ 0, %.preheader188.preheader.new ], [ %750, %748 ]
  %niter = phi i64 [ 0, %.preheader188.preheader.new ], [ %niter.next.3, %748 ]
  %706 = getelementptr inbounds nuw [200 x i8], ptr %697, i64 %704
  %707 = getelementptr i8, ptr %706, i64 168
  %708 = load i64, ptr %707, align 8, !noalias !23240, !noundef !1740
  %709 = getelementptr i8, ptr %706, i64 176
  %710 = load i64, ptr %709, align 8, !noalias !23240, !noundef !1740
  %711 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %710, i64 %708)
  %712 = extractvalue { i64, i1 } %711, 0
  %713 = extractvalue { i64, i1 } %711, 1
  br i1 %713, label %714, label %.preheader188.1, !prof !1742

714:                                              ; preds = %.preheader188
  br label %.preheader188.1

.preheader188.1:                                  ; preds = %714, %.preheader188
  %715 = phi i64 [ -1, %714 ], [ %712, %.preheader188 ]
  %716 = call noundef i64 @llvm.uadd.sat.i64(i64 %705, i64 %715)
  %717 = getelementptr inbounds nuw [200 x i8], ptr %697, i64 %704
  %718 = getelementptr i8, ptr %717, i64 368
  %719 = load i64, ptr %718, align 8, !noalias !23240, !noundef !1740
  %720 = getelementptr i8, ptr %717, i64 376
  %721 = load i64, ptr %720, align 8, !noalias !23240, !noundef !1740
  %722 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %721, i64 %719)
  %723 = extractvalue { i64, i1 } %722, 0
  %724 = extractvalue { i64, i1 } %722, 1
  br i1 %724, label %725, label %.preheader188.2, !prof !1742

725:                                              ; preds = %.preheader188.1
  br label %.preheader188.2

.preheader188.2:                                  ; preds = %725, %.preheader188.1
  %726 = phi i64 [ -1, %725 ], [ %723, %.preheader188.1 ]
  %727 = call noundef i64 @llvm.uadd.sat.i64(i64 %716, i64 %726)
  %728 = getelementptr inbounds nuw [200 x i8], ptr %697, i64 %704
  %729 = getelementptr i8, ptr %728, i64 568
  %730 = load i64, ptr %729, align 8, !noalias !23240, !noundef !1740
  %731 = getelementptr i8, ptr %728, i64 576
  %732 = load i64, ptr %731, align 8, !noalias !23240, !noundef !1740
  %733 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %732, i64 %730)
  %734 = extractvalue { i64, i1 } %733, 0
  %735 = extractvalue { i64, i1 } %733, 1
  br i1 %735, label %736, label %.preheader188.3, !prof !1742

736:                                              ; preds = %.preheader188.2
  br label %.preheader188.3

.preheader188.3:                                  ; preds = %736, %.preheader188.2
  %737 = phi i64 [ -1, %736 ], [ %734, %.preheader188.2 ]
  %738 = call noundef i64 @llvm.uadd.sat.i64(i64 %727, i64 %737)
  %739 = getelementptr inbounds nuw [200 x i8], ptr %697, i64 %704
  %740 = getelementptr i8, ptr %739, i64 768
  %741 = load i64, ptr %740, align 8, !noalias !23240, !noundef !1740
  %742 = getelementptr i8, ptr %739, i64 776
  %743 = load i64, ptr %742, align 8, !noalias !23240, !noundef !1740
  %744 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %743, i64 %741)
  %745 = extractvalue { i64, i1 } %744, 0
  %746 = extractvalue { i64, i1 } %744, 1
  br i1 %746, label %747, label %748, !prof !1742

747:                                              ; preds = %.preheader188.3
  br label %748

748:                                              ; preds = %747, %.preheader188.3
  %749 = phi i64 [ -1, %747 ], [ %745, %.preheader188.3 ]
  %750 = call noundef i64 @llvm.uadd.sat.i64(i64 %738, i64 %749)
  %751 = add nuw i64 %704, 4
  %niter.next.3 = add i64 %niter, 4
  %niter.ncmp.3 = icmp eq i64 %niter.next.3, %unroll_iter
  br i1 %niter.ncmp.3, label %.unr-lcssa, label %.preheader188

.unr-lcssa:                                       ; preds = %748
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.epilog-lcssa, label %.preheader188.epil.preheader

.preheader188.epil.preheader:                     ; preds = %.unr-lcssa, %.preheader188.preheader
  %.epil.init = phi i64 [ 0, %.preheader188.preheader ], [ %751, %.unr-lcssa ]
  %.epil.init1738 = phi i64 [ 0, %.preheader188.preheader ], [ %750, %.unr-lcssa ]
  %lcmp.mod1740 = icmp ne i64 %xtraiter, 0
  call void @llvm.assume(i1 %lcmp.mod1740)
  br label %.preheader188.epil

.preheader188.epil:                               ; preds = %763, %.preheader188.epil.preheader
  %752 = phi i64 [ %766, %763 ], [ %.epil.init, %.preheader188.epil.preheader ]
  %753 = phi i64 [ %765, %763 ], [ %.epil.init1738, %.preheader188.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %763 ], [ 0, %.preheader188.epil.preheader ]
  %754 = getelementptr inbounds nuw [200 x i8], ptr %697, i64 %752
  %755 = getelementptr i8, ptr %754, i64 168
  %756 = load i64, ptr %755, align 8, !noalias !23240, !noundef !1740
  %757 = getelementptr i8, ptr %754, i64 176
  %758 = load i64, ptr %757, align 8, !noalias !23240, !noundef !1740
  %759 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %758, i64 %756)
  %760 = extractvalue { i64, i1 } %759, 0
  %761 = extractvalue { i64, i1 } %759, 1
  br i1 %761, label %762, label %763, !prof !1742

762:                                              ; preds = %.preheader188.epil
  br label %763

763:                                              ; preds = %762, %.preheader188.epil
  %764 = phi i64 [ -1, %762 ], [ %760, %.preheader188.epil ]
  %765 = call noundef i64 @llvm.uadd.sat.i64(i64 %753, i64 %764)
  %766 = add nuw i64 %752, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.epilog-lcssa, label %.preheader188.epil, !llvm.loop !23243

.epilog-lcssa:                                    ; preds = %763, %.unr-lcssa
  %.lcssa1690 = phi i64 [ %750, %.unr-lcssa ], [ %765, %763 ]
  %767 = icmp eq i64 %.lcssa1690, 0
  br i1 %767, label %772, label %836

768:                                              ; preds = %790, %770
  %769 = phi { ptr, i32 } [ %771, %770 ], [ %800, %790 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %48) #90
          to label %1746 unwind label %818, !noalias !23244

770:                                              ; preds = %806
  %771 = landingpad { ptr, i32 }
          cleanup
  br label %768

772:                                              ; preds = %840, %836, %.epilog-lcssa, %695
  %773 = load i64, ptr %53, align 8, !range !1835, !noalias !23227, !noundef !1740
  %774 = icmp ult i64 %699, 46116860184273880
  call void @llvm.assume(i1 %774)
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !23248
  store i64 0, ptr %48, align 8, !noalias !23248
  %775 = getelementptr inbounds nuw i8, ptr %48, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %775, align 8, !noalias !23248
  %776 = getelementptr inbounds nuw i8, ptr %48, i64 16
  store i64 0, ptr %776, align 8, !noalias !23248
  call void @llvm.lifetime.start.p0(ptr nonnull %47), !noalias !23248
  store ptr %697, ptr %47, align 8, !noalias !23248
  %777 = getelementptr inbounds nuw i8, ptr %47, i64 8
  %778 = getelementptr inbounds nuw i8, ptr %47, i64 16
  store i64 %773, ptr %778, align 8, !noalias !23248
  %779 = getelementptr inbounds nuw i8, ptr %47, i64 24
  store ptr %701, ptr %779, align 8, !noalias !23248
  br i1 %702, label %.loopexit184, label %780

780:                                              ; preds = %772
  %781 = getelementptr inbounds nuw i8, ptr %46, i64 8
  %782 = getelementptr inbounds nuw i8, ptr %46, i64 152
  br label %783

783:                                              ; preds = %815, %780
  %784 = phi ptr [ inttoptr (i64 8 to ptr), %780 ], [ %811, %815 ]
  %785 = phi i64 [ 0, %780 ], [ %813, %815 ]
  %786 = phi ptr [ %697, %780 ], [ %787, %815 ]
  %787 = getelementptr inbounds nuw i8, ptr %786, i64 200
  %788 = load i64, ptr %786, align 8, !noalias !23249
  %789 = icmp eq i64 %788, -1
  br i1 %789, label %.loopexit184, label %791

790:                                              ; preds = %799
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %47)
          to label %768 unwind label %818

791:                                              ; preds = %783
  %792 = getelementptr inbounds nuw i8, ptr %786, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %46), !noalias !23248
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %781, ptr noundef nonnull align 8 dereferenceable(152) %792, i64 152, i1 false), !noalias !23244
  store i64 %788, ptr %46, align 8, !noalias !23248
  %793 = load i8, ptr %782, align 8, !range !1747, !noalias !23248, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23255)
  %794 = load i64, ptr %48, align 8, !range !1835, !alias.scope !23255, !noalias !23258, !noundef !1740
  %795 = icmp eq i64 %785, %794
  br i1 %795, label %796, label %810

796:                                              ; preds = %791
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %48)
          to label %797 unwind label %799, !noalias !23258

797:                                              ; preds = %796
  %798 = load ptr, ptr %775, align 8, !alias.scope !23255, !noalias !23258
  br label %810

799:                                              ; preds = %796
  %800 = landingpad { ptr, i32 }
          cleanup
  store ptr %787, ptr %777, align 8
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(160) %46) #90
          to label %790 unwind label %801, !noalias !23260

801:                                              ; preds = %799
  %802 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !23261
  unreachable

.loopexit184:                                     ; preds = %815, %783, %772
  %803 = phi i64 [ 0, %772 ], [ %813, %815 ], [ %785, %783 ]
  %804 = phi ptr [ inttoptr (i64 8 to ptr), %772 ], [ %811, %815 ], [ %784, %783 ]
  %805 = phi ptr [ %697, %772 ], [ %701, %815 ], [ %787, %783 ]
  store ptr %805, ptr %777, align 8
  br label %806

806:                                              ; preds = %817, %.loopexit184
  %807 = phi i64 [ %813, %817 ], [ %803, %.loopexit184 ]
  %808 = phi ptr [ %811, %817 ], [ %804, %.loopexit184 ]
  %809 = phi i8 [ 1, %817 ], [ 0, %.loopexit184 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %47)
          to label %820 unwind label %770

810:                                              ; preds = %797, %791
  %811 = phi ptr [ %798, %797 ], [ %784, %791 ]
  %812 = getelementptr inbounds nuw [160 x i8], ptr %811, i64 %785
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(160) %812, ptr noundef nonnull readonly align 8 dereferenceable(160) %46, i64 160, i1 false), !noalias !23260
  %813 = add nuw nsw i64 %785, 1
  store i64 %813, ptr %776, align 8, !alias.scope !23255, !noalias !23258
  %814 = trunc nuw i8 %793 to i1
  br i1 %814, label %817, label %815

815:                                              ; preds = %810
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !23248
  %816 = icmp eq ptr %787, %701
  br i1 %816, label %.loopexit184, label %783

817:                                              ; preds = %810
  store ptr %787, ptr %777, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %46), !noalias !23248
  br label %806

818:                                              ; preds = %790, %768
  %819 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !23244
  unreachable

820:                                              ; preds = %806
  call void @llvm.lifetime.end.p0(ptr nonnull %47), !noalias !23248
  %821 = load i64, ptr %48, align 8, !noalias !23248
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !23248
  %822 = getelementptr inbounds nuw i8, ptr %84, i64 193
  %823 = load i8, ptr %822, align 1, !range !1747, !alias.scope !23234, !noalias !23262, !noundef !1740
  %824 = trunc nuw i8 %823 to i1
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %808) ]
  %825 = icmp eq i64 %807, 0
  br i1 %825, label %.loopexit183, label %iter.check

iter.check:                                       ; preds = %820
  %min.iters.check = icmp ult i64 %807, 8
  br i1 %min.iters.check, label %.preheader182.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check1317 = icmp ult i64 %807, 32
  br i1 %min.iters.check1317, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %807, 24
  %n.vec = and i64 %807, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %826, %vector.body ]
  %vec.phi1318 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %827, %vector.body ]
  %vec.phi1319 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %828, %vector.body ]
  %vec.phi1320 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %829, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %vec.ind
  %wide.gep1321 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %step.add
  %wide.gep1322 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %step.add.2
  %wide.gep1323 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %step.add.3
  %wide.gep1324 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep1325 = getelementptr i8, <8 x ptr> %wide.gep1321, i64 64
  %wide.gep1326 = getelementptr i8, <8 x ptr> %wide.gep1322, i64 64
  %wide.gep1327 = getelementptr i8, <8 x ptr> %wide.gep1323, i64 64
  %wide.masked.gather = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1324, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23263
  %wide.masked.gather1328 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1325, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23263
  %wide.masked.gather1329 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1326, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23263
  %wide.masked.gather1330 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1327, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23263
  %826 = add <8 x i64> %wide.masked.gather, %vec.phi
  %827 = add <8 x i64> %wide.masked.gather1328, %vec.phi1318
  %828 = add <8 x i64> %wide.masked.gather1329, %vec.phi1319
  %829 = add <8 x i64> %wide.masked.gather1330, %vec.phi1320
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %830 = icmp eq i64 %index.next, %n.vec
  br i1 %830, label %middle.block, label %vector.body, !llvm.loop !23266

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %827, %826
  %bin.rdx1331 = add <8 x i64> %828, %bin.rdx
  %bin.rdx1332 = add <8 x i64> %829, %bin.rdx1331
  %831 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1332)
  %cmp.n = icmp eq i64 %807, %n.vec
  br i1 %cmp.n, label %.loopexit183, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader182.preheader, label %vec.epilog.ph, !prof !11068

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %831, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec1334 = and i64 %807, -8
  %832 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index1335 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next1341, %vec.epilog.vector.body ]
  %vec.ind1336 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next1342, %vec.epilog.vector.body ]
  %vec.phi1337 = phi <8 x i64> [ %832, %vec.epilog.ph ], [ %833, %vec.epilog.vector.body ]
  %wide.gep1338 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %vec.ind1336
  %wide.gep1339 = getelementptr i8, <8 x ptr> %wide.gep1338, i64 64
  %wide.masked.gather1340 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1339, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23263
  %833 = add <8 x i64> %wide.masked.gather1340, %vec.phi1337
  %index.next1341 = add nuw i64 %index1335, 8
  %vec.ind.next1342 = add nuw <8 x i64> %vec.ind1336, splat (i64 8)
  %834 = icmp eq i64 %index.next1341, %n.vec1334
  br i1 %834, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !23267

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %835 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %833)
  %cmp.n1343 = icmp eq i64 %807, %n.vec1334
  br i1 %cmp.n1343, label %.loopexit183, label %.preheader182.preheader

.preheader182.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph1674 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec1334, %vec.epilog.middle.block ]
  %.ph1675 = phi i64 [ 0, %iter.check ], [ %831, %vec.epilog.iter.check ], [ %835, %vec.epilog.middle.block ]
  br label %.preheader182

836:                                              ; preds = %.epilog-lcssa
  %837 = getelementptr inbounds nuw i8, ptr %668, i64 336
  %838 = load ptr, ptr %837, align 8, !noalias !23240, !noundef !1740
  %839 = icmp eq ptr %838, null
  br i1 %839, label %772, label %840

840:                                              ; preds = %836
  %841 = getelementptr inbounds nuw i8, ptr %668, i64 352
  %842 = atomicrmw add ptr %841, i64 %.lcssa1690 monotonic, align 8, !noalias !23240
  br label %772

.preheader182:                                    ; preds = %.preheader182.preheader, %.preheader182
  %843 = phi i64 [ %850, %.preheader182 ], [ %.ph1674, %.preheader182.preheader ]
  %844 = phi i64 [ %849, %.preheader182 ], [ %.ph1675, %.preheader182.preheader ]
  %845 = getelementptr inbounds nuw [160 x i8], ptr %808, i64 %843
  %846 = getelementptr i8, ptr %845, i64 64
  %847 = load i64, ptr %846, align 8, !noalias !23263, !noundef !1740
  %848 = icmp ult i64 %847, 288230376151711744
  call void @llvm.assume(i1 %848), !noalias !23240
  %849 = add i64 %847, %844
  %850 = add nuw i64 %843, 1
  %851 = icmp eq i64 %850, %807
  br i1 %851, label %.loopexit183, label %.preheader182, !llvm.loop !23268

.loopexit183:                                     ; preds = %.preheader182, %middle.block, %vec.epilog.middle.block, %820
  %852 = phi i64 [ 0, %820 ], [ %835, %vec.epilog.middle.block ], [ %831, %middle.block ], [ %849, %.preheader182 ]
  %853 = trunc nuw i8 %809 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %51)
  %854 = getelementptr inbounds nuw i8, ptr %84, i64 195
  %855 = load i8, ptr %854, align 1, !range !6141, !alias.scope !23234, !noalias !23262, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !23227
  store i64 %821, ptr %50, align 8, !noalias !23227
  %856 = getelementptr inbounds nuw i8, ptr %50, i64 8
  store ptr %808, ptr %856, align 8, !noalias !23227
  %857 = getelementptr inbounds nuw i8, ptr %50, i64 16
  store i64 %807, ptr %857, align 8, !noalias !23227
  %858 = getelementptr inbounds nuw i8, ptr %50, i64 24
  store i8 %809, ptr %858, align 8, !noalias !23227
  call void @llvm.experimental.noalias.scope.decl(metadata !23269)
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !23272
  store i64 0, ptr %43, align 8, !noalias !23272
  %859 = getelementptr inbounds nuw i8, ptr %43, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %859, align 8, !noalias !23272
  %860 = getelementptr inbounds nuw i8, ptr %43, i64 16
  store i64 0, ptr %860, align 8, !noalias !23272
  %861 = icmp ult i64 %639, 230584300921369396
  call void @llvm.assume(i1 %861), !noalias !23276
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %44, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %43, i64 noundef %639)
          to label %862 unwind label %1687, !noalias !23277

862:                                              ; preds = %.loopexit183
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !23272
  %863 = load i64, ptr %44, align 16, !range !2527, !noalias !23272, !noundef !1740
  %864 = icmp eq i64 %863, -1
  %865 = getelementptr inbounds nuw i8, ptr %44, i64 8
  %866 = load i64, ptr %865, align 8, !noalias !23272
  %867 = getelementptr inbounds nuw i8, ptr %44, i64 16
  %868 = load ptr, ptr %867, align 16, !noalias !23272
  %869 = getelementptr inbounds nuw i8, ptr %44, i64 24
  %870 = load i64, ptr %869, align 8, !noalias !23272
  br i1 %864, label %928, label %871

871:                                              ; preds = %862
  %872 = getelementptr inbounds nuw i8, ptr %44, i64 32
  %873 = load i8, ptr %872, align 16, !noalias !23272
  %874 = getelementptr inbounds nuw i8, ptr %44, i64 33
  %875 = load i56, ptr %874, align 1, !noalias !23272
  %876 = getelementptr inbounds nuw i8, ptr %44, i64 40
  %877 = load i64, ptr %876, align 8, !noalias !23272
  %878 = getelementptr inbounds nuw i8, ptr %44, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %51, ptr noundef nonnull align 16 dereferenceable(48) %878, i64 48, i1 false), !noalias !23278
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !23272
  call void @llvm.experimental.noalias.scope.decl(metadata !23279), !noalias !23282
  %879 = icmp eq i64 %639, 0
  br i1 %879, label %.loopexit181, label %.preheader180

.preheader180:                                    ; preds = %871
  %880 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %881 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %882

882:                                              ; preds = %.preheader180, %920
  %883 = phi i64 [ %885, %920 ], [ 0, %.preheader180 ]
  %884 = getelementptr inbounds nuw [40 x i8], ptr %660, i64 %883
  %885 = add nuw nsw i64 %883, 1
  %886 = load i64, ptr %884, align 8, !range !1778, !alias.scope !23283, !noalias !23286, !noundef !1740
  %887 = icmp ugt i64 %886, 5
  br i1 %887, label %888, label %920

888:                                              ; preds = %882
  %889 = getelementptr i8, ptr %884, i64 8
  %890 = load ptr, ptr %889, align 8, !alias.scope !23279, !noalias !23286, !nonnull !1740, !noundef !1740
  %891 = shl i64 %886, 3
  %892 = add i64 %891, -8
  %893 = load i64, ptr %880, align 8, !noalias !23289, !noundef !1740
  %894 = call i64 @llvm.umin.i64(i64 %892, i64 9223372036854775807)
  %895 = call i64 @llvm.ssub.sat.i64(i64 %893, i64 %894)
  store i64 %895, ptr %880, align 8, !noalias !23289
  %896 = load i64, ptr %881, align 8, !noalias !23289, !noundef !1740
  %897 = icmp slt i64 %895, %896
  br i1 %897, label %898, label %.preheader1673

898:                                              ; preds = %888
  store i64 %895, ptr %881, align 8, !noalias !23289
  br label %.preheader1673

.preheader1673:                                   ; preds = %898, %888
  br label %899

899:                                              ; preds = %.preheader1673, %902
  %900 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23289
  %901 = icmp slt i64 %900, 0
  br i1 %901, label %902, label %__rustc::__rust_dealloc (.exit140)

902:                                              ; preds = %899
  %903 = add nsw i64 %900, 1
  %904 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %900, i64 %903 acq_rel acquire, align 8, !noalias !23289
  %905 = extractvalue { i64, i1 } %904, 1
  br i1 %905, label %906, label %899

906:                                              ; preds = %902
  %907 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %894 monotonic, align 8, !noalias !23289
  %908 = call i64 @llvm.ssub.sat.i64(i64 %907, i64 %894)
  %909 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23289
  br label %910

910:                                              ; preds = %913, %906
  %911 = phi i64 [ %909, %906 ], [ %916, %913 ]
  %912 = icmp slt i64 %908, %911
  br i1 %912, label %913, label %917

913:                                              ; preds = %910
  %914 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %911, i64 %908 monotonic monotonic, align 8, !noalias !23289
  %915 = extractvalue { i64, i1 } %914, 1
  %916 = extractvalue { i64, i1 } %914, 0
  br i1 %915, label %917, label %910

917:                                              ; preds = %913, %910
  %918 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23289
  br label %__rustc::__rust_dealloc (.exit140)

__rustc::__rust_dealloc (.exit140): ; preds = %899, %917
  %919 = icmp ne i64 %892, 0
  call void @llvm.assume(i1 %919), !noalias !23289
  call void @free(ptr noundef nonnull %890) #93, !noalias !23289
  br label %920

920:                                              ; preds = %__rustc::__rust_dealloc (.exit140), %882
  %921 = icmp eq i64 %885, %639
  br i1 %921, label %.loopexit181, label %882

.loopexit181:                                     ; preds = %920, %871
  %922 = icmp eq i64 %627, 0
  br i1 %922, label %925, label %923

923:                                              ; preds = %.loopexit181
  %924 = mul nuw i64 %627, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %660, i64 noundef %924, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !23286
  br label %925

925:                                              ; preds = %923, %.loopexit181
  %926 = zext i56 %875 to i64
  %927 = shl nuw i64 %926, 8
  br label %1629

928:                                              ; preds = %862
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !23272
  store i64 %866, ptr %45, align 8, !noalias !23272
  %929 = getelementptr inbounds nuw i8, ptr %45, i64 8
  store ptr %868, ptr %929, align 8, !noalias !23272
  %930 = getelementptr inbounds nuw i8, ptr %45, i64 16
  store i64 %870, ptr %930, align 8, !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %42), !noalias !23272
  %931 = icmp ne i64 %637, 0
  call void @llvm.assume(i1 %931)
  %932 = getelementptr inbounds nuw [40 x i8], ptr %660, i64 %639
  store ptr %660, ptr %42, align 8, !noalias !23272
  %933 = getelementptr inbounds nuw i8, ptr %42, i64 16
  store i64 %627, ptr %933, align 8, !noalias !23272
  %934 = getelementptr inbounds nuw i8, ptr %42, i64 8
  store ptr %660, ptr %934, align 8, !noalias !23272
  %935 = getelementptr inbounds nuw i8, ptr %42, i64 24
  store ptr %932, ptr %935, align 8, !noalias !23272
  %936 = load ptr, ptr %593, align 8, !alias.scope !23292, !noalias !23293, !noundef !1740
  %937 = icmp eq ptr %936, null
  br i1 %937, label %941, label %938

938:                                              ; preds = %928
  %939 = atomicrmw add ptr %936, i64 1 monotonic, align 8, !noalias !23294
  %940 = icmp slt i64 %939, 0
  br i1 %940, label %1008, label %1000

941:                                              ; preds = %928
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !23272
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %41, ptr noundef nonnull align 8 dereferenceable(32) %42, i64 32, i1 false), !noalias !23272
  %942 = getelementptr inbounds nuw i8, ptr %41, i64 24
  %943 = load ptr, ptr %942, align 8, !alias.scope !23295, !noalias !23298, !nonnull !1740, !noundef !1740
  %944 = getelementptr inbounds nuw i8, ptr %41, i64 8
  %945 = load ptr, ptr %944, align 8, !alias.scope !23295, !noalias !23298
  %946 = icmp eq ptr %945, %943
  br i1 %946, label %968, label %.preheader158

947:                                              ; preds = %983, %980
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %41) #90, !noalias !23294
  br label %1689

.preheader158:                                    ; preds = %941, %986
  %948 = phi ptr [ %987, %986 ], [ %868, %941 ]
  %949 = phi i64 [ %994, %986 ], [ %870, %941 ]
  %950 = phi ptr [ %951, %986 ], [ %945, %941 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23295)
  %951 = getelementptr inbounds nuw i8, ptr %950, i64 40
  %952 = load i64, ptr %950, align 8, !noalias !23300
  %953 = getelementptr inbounds nuw i8, ptr %950, i64 8
  %954 = load ptr, ptr %953, align 8, !noalias !23300
  %955 = getelementptr inbounds nuw i8, ptr %950, i64 16
  %956 = load i64, ptr %955, align 8, !noalias !23300
  %957 = getelementptr inbounds nuw i8, ptr %950, i64 24
  %958 = load i8, ptr %957, align 8, !noalias !23300
  %959 = getelementptr inbounds nuw i8, ptr %950, i64 25
  %960 = load i56, ptr %959, align 1, !noalias !23300
  %961 = getelementptr inbounds nuw i8, ptr %950, i64 32
  %962 = load i64, ptr %961, align 8, !noalias !23300
  %963 = icmp eq i64 %952, 0
  %964 = load i64, ptr %45, align 8, !noalias !23294
  br i1 %963, label %.loopexit159, label %975

.loopexit159:                                     ; preds = %.preheader158, %996
  %965 = phi i64 [ %997, %996 ], [ %964, %.preheader158 ]
  %966 = phi i64 [ %994, %996 ], [ %949, %.preheader158 ]
  %967 = load ptr, ptr %929, align 8, !noalias !23272
  br label %968

968:                                              ; preds = %.loopexit159, %941
  %969 = phi i64 [ %870, %941 ], [ %966, %.loopexit159 ]
  %970 = phi ptr [ %868, %941 ], [ %967, %.loopexit159 ]
  %971 = phi i64 [ %866, %941 ], [ %965, %.loopexit159 ]
  %972 = phi ptr [ %945, %941 ], [ %951, %.loopexit159 ]
  store ptr %972, ptr %944, align 8
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %41), !noalias !23294
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !23272
  br label %1629

973:                                              ; preds = %1618, %1125
  %974 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42) #90, !noalias !23294
  br label %2327

975:                                              ; preds = %.preheader158
  call void @llvm.experimental.noalias.scope.decl(metadata !23301)
  %976 = icmp eq i64 %949, %964
  br i1 %976, label %977, label %986

977:                                              ; preds = %975
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %45)
          to label %978 unwind label %980, !noalias !23304

978:                                              ; preds = %977
  %979 = load ptr, ptr %929, align 8, !alias.scope !23301, !noalias !23304
  br label %986

980:                                              ; preds = %977
  %981 = landingpad { ptr, i32 }
          cleanup
  store ptr %951, ptr %944, align 8
  %982 = icmp ugt i64 %952, 5
  br i1 %982, label %983, label %947

983:                                              ; preds = %980
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %954) ]
  %984 = shl i64 %952, 3
  %985 = add i64 %984, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %954, i64 noundef %985, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !23306
  br label %947

986:                                              ; preds = %978, %975
  %987 = phi ptr [ %979, %978 ], [ %948, %975 ]
  %988 = getelementptr inbounds nuw [40 x i8], ptr %987, i64 %949
  store i64 %952, ptr %988, align 8, !noalias !23309
  %989 = getelementptr inbounds nuw i8, ptr %988, i64 8
  store ptr %954, ptr %989, align 8, !noalias !23309
  %990 = getelementptr inbounds nuw i8, ptr %988, i64 16
  store i64 %956, ptr %990, align 8, !noalias !23309
  %991 = getelementptr inbounds nuw i8, ptr %988, i64 24
  store i8 %958, ptr %991, align 8, !noalias !23309
  %992 = getelementptr inbounds nuw i8, ptr %988, i64 25
  store i56 %960, ptr %992, align 1, !noalias !23309
  %993 = getelementptr inbounds nuw i8, ptr %988, i64 32
  store i64 %962, ptr %993, align 8, !noalias !23309
  %994 = add i64 %949, 1
  store i64 %994, ptr %930, align 8, !alias.scope !23301, !noalias !23304
  %995 = icmp eq ptr %951, %943
  br i1 %995, label %996, label %.preheader158

996:                                              ; preds = %986
  %997 = load i64, ptr %45, align 8, !noalias !23272
  br label %.loopexit159

998:                                              ; preds = %1692, %1172, %1089, %1082
  %999 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !23310
  unreachable

1000:                                             ; preds = %938
  %1001 = load ptr, ptr %593, align 8, !alias.scope !23292, !noalias !23293, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !23272
  store ptr %1001, ptr %40, align 8, !noalias !23272
  %1002 = getelementptr inbounds nuw i8, ptr %1001, i64 16
  %1003 = load i64, ptr %1002, align 8, !noalias !23294
  %1004 = icmp eq i64 %1003, -1
  %1005 = getelementptr inbounds nuw i8, ptr %1001, i64 40
  %1006 = load i64, ptr %1005, align 8, !noalias !23294
  %1007 = icmp eq i64 %1006, -1
  br i1 %1007, label %1009, label %1056

1008:                                             ; preds = %938
  call void @llvm.trap(), !noalias !23276
  unreachable

1009:                                             ; preds = %.loopexit179, %1000
  %1010 = icmp ult i64 %807, 57646075230342349
  call void @llvm.assume(i1 %1010), !noalias !23276
  %1011 = mul nuw nsw i64 %807, 160
  %1012 = getelementptr inbounds nuw i8, ptr %808, i64 %1011
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !23272
  store ptr %808, ptr %39, align 8, !noalias !23272
  %1013 = getelementptr inbounds nuw i8, ptr %39, i64 8
  store ptr %808, ptr %1013, align 8, !noalias !23272
  %1014 = getelementptr inbounds nuw i8, ptr %39, i64 16
  store i64 %821, ptr %1014, align 8, !noalias !23272
  %1015 = getelementptr inbounds nuw i8, ptr %39, i64 24
  store ptr %1012, ptr %1015, align 8, !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %38)
  br i1 %825, label %.loopexit177, label %1016

1016:                                             ; preds = %1009
  %1017 = getelementptr inbounds nuw i8, ptr %37, i64 8
  %1018 = getelementptr inbounds nuw i8, ptr %37, i64 24
  %1019 = getelementptr inbounds nuw i8, ptr %37, i64 32
  %1020 = getelementptr inbounds nuw i8, ptr %37, i64 40
  %1021 = getelementptr inbounds nuw i8, ptr %37, i64 16
  %1022 = getelementptr inbounds nuw i8, ptr %36, i64 16
  %1023 = getelementptr inbounds nuw i8, ptr %36, i64 8
  %1024 = getelementptr inbounds nuw i8, ptr %36, i64 24
  %1025 = getelementptr inbounds nuw i8, ptr %37, i64 96
  %1026 = getelementptr inbounds nuw i8, ptr %37, i64 48
  %1027 = getelementptr inbounds nuw i8, ptr %37, i64 56
  %1028 = getelementptr inbounds nuw i8, ptr %37, i64 64
  %1029 = getelementptr inbounds nuw i8, ptr %4, i64 632
  %1030 = getelementptr inbounds nuw i8, ptr %4, i64 1228
  %1031 = zext nneg i8 %855 to i64
  %1032 = getelementptr inbounds nuw i8, ptr %1001, i64 296
  %1033 = getelementptr inbounds nuw i8, ptr %1001, i64 272
  %1034 = getelementptr inbounds nuw i8, ptr %1001, i64 80
  %1035 = getelementptr inbounds nuw i8, ptr %4, i64 1048
  %1036 = getelementptr inbounds nuw i8, ptr %4, i64 1056
  %1037 = getelementptr inbounds nuw i8, ptr %1001, i64 104
  %1038 = getelementptr inbounds nuw i8, ptr %24, i64 8
  %1039 = getelementptr inbounds nuw i8, ptr %4, i64 888
  %1040 = getelementptr inbounds nuw i8, ptr %27, i64 8
  %1041 = getelementptr inbounds nuw i8, ptr %28, i64 16
  %1042 = getelementptr inbounds nuw i8, ptr %37, i64 88
  %1043 = getelementptr inbounds nuw i8, ptr %37, i64 120
  %1044 = getelementptr inbounds nuw i8, ptr %14, i64 1
  %1045 = getelementptr inbounds nuw i8, ptr %14, i64 8
  %1046 = getelementptr inbounds nuw i8, ptr %14, i64 16
  %1047 = getelementptr inbounds nuw i8, ptr %10, i64 1
  %1048 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %1049 = getelementptr inbounds nuw i8, ptr %10, i64 16
  %1050 = getelementptr inbounds nuw i8, ptr %11, i64 1
  %1051 = getelementptr inbounds nuw i8, ptr %11, i64 8
  %1052 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %1053 = getelementptr inbounds nuw i8, ptr %12, i64 1
  %1054 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %1055 = getelementptr inbounds nuw i8, ptr %12, i64 16
  br label %1090

1056:                                             ; preds = %1000
  br i1 %825, label %.loopexit179, label %iter.check1382

iter.check1382:                                   ; preds = %1056
  %min.iters.check1345 = icmp ult i64 %807, 8
  br i1 %min.iters.check1345, label %.preheader178.preheader, label %vector.main.loop.iter.check1346

vector.main.loop.iter.check1346:                  ; preds = %iter.check1382
  %min.iters.check1347 = icmp ult i64 %807, 32
  br i1 %min.iters.check1347, label %vec.epilog.ph1386, label %vector.ph1348

vector.ph1348:                                    ; preds = %vector.main.loop.iter.check1346
  %n.mod.vf1349 = and i64 %807, 24
  %n.vec1350 = and i64 %807, -32
  br label %vector.body1351

vector.body1351:                                  ; preds = %vector.body1351, %vector.ph1348
  %index1352 = phi i64 [ 0, %vector.ph1348 ], [ %index.next1373, %vector.body1351 ]
  %vec.ind1353 = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph1348 ], [ %vec.ind.next1374, %vector.body1351 ]
  %vec.phi1354 = phi <8 x i64> [ zeroinitializer, %vector.ph1348 ], [ %1057, %vector.body1351 ]
  %vec.phi1355 = phi <8 x i64> [ zeroinitializer, %vector.ph1348 ], [ %1058, %vector.body1351 ]
  %vec.phi1356 = phi <8 x i64> [ zeroinitializer, %vector.ph1348 ], [ %1059, %vector.body1351 ]
  %vec.phi1357 = phi <8 x i64> [ zeroinitializer, %vector.ph1348 ], [ %1060, %vector.body1351 ]
  %step.add1358 = add nuw <8 x i64> %vec.ind1353, splat (i64 8)
  %step.add.21359 = add nuw <8 x i64> %vec.ind1353, splat (i64 16)
  %step.add.31360 = add nuw <8 x i64> %vec.ind1353, splat (i64 24)
  %wide.gep1361 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %vec.ind1353
  %wide.gep1362 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %step.add1358
  %wide.gep1363 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %step.add.21359
  %wide.gep1364 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %step.add.31360
  %wide.gep1365 = getelementptr i8, <8 x ptr> %wide.gep1361, i64 16
  %wide.gep1366 = getelementptr i8, <8 x ptr> %wide.gep1362, i64 16
  %wide.gep1367 = getelementptr i8, <8 x ptr> %wide.gep1363, i64 16
  %wide.gep1368 = getelementptr i8, <8 x ptr> %wide.gep1364, i64 16
  %wide.masked.gather1369 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1365, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23294
  %wide.masked.gather1370 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1366, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23294
  %wide.masked.gather1371 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1367, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23294
  %wide.masked.gather1372 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1368, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23294
  %1057 = add <8 x i64> %wide.masked.gather1369, %vec.phi1354
  %1058 = add <8 x i64> %wide.masked.gather1370, %vec.phi1355
  %1059 = add <8 x i64> %wide.masked.gather1371, %vec.phi1356
  %1060 = add <8 x i64> %wide.masked.gather1372, %vec.phi1357
  %index.next1373 = add nuw i64 %index1352, 32
  %vec.ind.next1374 = add nuw <8 x i64> %vec.ind1353, splat (i64 32)
  %1061 = icmp eq i64 %index.next1373, %n.vec1350
  br i1 %1061, label %middle.block1375, label %vector.body1351, !llvm.loop !23311

middle.block1375:                                 ; preds = %vector.body1351
  %bin.rdx1376 = add <8 x i64> %1058, %1057
  %bin.rdx1377 = add <8 x i64> %1059, %bin.rdx1376
  %bin.rdx1378 = add <8 x i64> %1060, %bin.rdx1377
  %1062 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx1378)
  %cmp.n1379 = icmp eq i64 %807, %n.vec1350
  br i1 %cmp.n1379, label %.loopexit179, label %vec.epilog.iter.check1384

vec.epilog.iter.check1384:                        ; preds = %middle.block1375
  %min.epilog.iters.check1385 = icmp eq i64 %n.mod.vf1349, 0
  br i1 %min.epilog.iters.check1385, label %.preheader178.preheader, label %vec.epilog.ph1386, !prof !11068

vec.epilog.ph1386:                                ; preds = %vector.main.loop.iter.check1346, %vec.epilog.iter.check1384
  %vec.epilog.resume.val1380 = phi i64 [ %n.vec1350, %vec.epilog.iter.check1384 ], [ 0, %vector.main.loop.iter.check1346 ]
  %bc.merge.rdx1381 = phi i64 [ %1062, %vec.epilog.iter.check1384 ], [ 0, %vector.main.loop.iter.check1346 ]
  %n.vec1388 = and i64 %807, -8
  %1063 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx1381, i64 0
  %broadcast.splatinsert1389 = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val1380, i64 0
  %broadcast.splat1390 = shufflevector <8 x i64> %broadcast.splatinsert1389, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction1391 = or disjoint <8 x i64> %broadcast.splat1390, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body1392

vec.epilog.vector.body1392:                       ; preds = %vec.epilog.vector.body1392, %vec.epilog.ph1386
  %index1393 = phi i64 [ %vec.epilog.resume.val1380, %vec.epilog.ph1386 ], [ %index.next1399, %vec.epilog.vector.body1392 ]
  %vec.ind1394 = phi <8 x i64> [ %induction1391, %vec.epilog.ph1386 ], [ %vec.ind.next1400, %vec.epilog.vector.body1392 ]
  %vec.phi1395 = phi <8 x i64> [ %1063, %vec.epilog.ph1386 ], [ %1064, %vec.epilog.vector.body1392 ]
  %wide.gep1396 = getelementptr inbounds nuw [160 x i8], ptr %808, <8 x i64> %vec.ind1394
  %wide.gep1397 = getelementptr i8, <8 x ptr> %wide.gep1396, i64 16
  %wide.masked.gather1398 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep1397, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !23294
  %1064 = add <8 x i64> %wide.masked.gather1398, %vec.phi1395
  %index.next1399 = add nuw i64 %index1393, 8
  %vec.ind.next1400 = add nuw <8 x i64> %vec.ind1394, splat (i64 8)
  %1065 = icmp eq i64 %index.next1399, %n.vec1388
  br i1 %1065, label %vec.epilog.middle.block1401, label %vec.epilog.vector.body1392, !llvm.loop !23312

vec.epilog.middle.block1401:                      ; preds = %vec.epilog.vector.body1392
  %1066 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %1064)
  %cmp.n1402 = icmp eq i64 %807, %n.vec1388
  br i1 %cmp.n1402, label %.loopexit179, label %.preheader178.preheader

.preheader178.preheader:                          ; preds = %iter.check1382, %vec.epilog.iter.check1384, %vec.epilog.middle.block1401
  %.ph1665 = phi i64 [ 0, %iter.check1382 ], [ %n.vec1350, %vec.epilog.iter.check1384 ], [ %n.vec1388, %vec.epilog.middle.block1401 ]
  %.ph1666 = phi i64 [ 0, %iter.check1382 ], [ %1062, %vec.epilog.iter.check1384 ], [ %1066, %vec.epilog.middle.block1401 ]
  br label %.preheader178

.preheader178:                                    ; preds = %.preheader178.preheader, %.preheader178
  %1067 = phi i64 [ %1074, %.preheader178 ], [ %.ph1665, %.preheader178.preheader ]
  %1068 = phi i64 [ %1073, %.preheader178 ], [ %.ph1666, %.preheader178.preheader ]
  %1069 = getelementptr inbounds nuw [160 x i8], ptr %808, i64 %1067
  %1070 = getelementptr i8, ptr %1069, i64 16
  %1071 = load i64, ptr %1070, align 8, !noalias !23294, !noundef !1740
  %1072 = icmp ult i64 %1071, 104811045873349726
  call void @llvm.assume(i1 %1072), !noalias !23294
  %1073 = add i64 %1071, %1068
  %1074 = add nuw i64 %1067, 1
  %1075 = icmp eq i64 %1074, %807
  br i1 %1075, label %.loopexit179, label %.preheader178, !llvm.loop !23313

1076:                                             ; preds = %1089, %1083
  %1077 = phi i1 [ %1084, %1083 ], [ %1175, %1089 ]
  %1078 = phi i1 [ %1085, %1083 ], [ false, %1089 ]
  %1079 = phi { ptr, i32 } [ %1086, %1083 ], [ %1176, %1089 ]
  %1080 = atomicrmw sub ptr %1001, i64 1 release, align 8, !noalias !23314
  %1081 = icmp eq i64 %1080, 1
  br i1 %1081, label %1082, label %1626

1082:                                             ; preds = %1076
  fence acquire, !noalias !23294
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #92
          to label %1626 unwind label %998

1083:                                             ; preds = %1614, %1126, %.loopexit177, %.loopexit179
  %1084 = phi i1 [ false, %1614 ], [ true, %1126 ], [ true, %.loopexit177 ], [ true, %.loopexit179 ]
  %1085 = phi i1 [ false, %1614 ], [ false, %1126 ], [ false, %.loopexit177 ], [ true, %.loopexit179 ]
  %1086 = landingpad { ptr, i32 }
          cleanup
  br label %1076

.loopexit179:                                     ; preds = %.preheader178, %middle.block1375, %vec.epilog.middle.block1401, %1056
  %1087 = phi i64 [ 0, %1056 ], [ %1066, %vec.epilog.middle.block1401 ], [ %1062, %middle.block1375 ], [ %1073, %.preheader178 ]
  %1088 = getelementptr inbounds nuw i8, ptr %4, i64 888
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1088, i64 noundef %1087)
          to label %1009 unwind label %1083, !noalias !23294

1089:                                             ; preds = %1625, %1622, %1619
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39) #90
          to label %1076 unwind label %998, !noalias !23294

1090:                                             ; preds = %1209, %1016
  %1091 = phi ptr [ %808, %1016 ], [ %1093, %1209 ]
  %1092 = phi ptr [ %660, %1016 ], [ %1168, %1209 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23319)
  %1093 = getelementptr inbounds nuw i8, ptr %1091, i64 160
  store ptr %1093, ptr %1013, align 8, !alias.scope !23319, !noalias !23322
  %1094 = load i64, ptr %1091, align 8, !noalias !23324
  %1095 = getelementptr inbounds nuw i8, ptr %1091, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %38, ptr noundef nonnull align 8 dereferenceable(152) %1095, i64 152, i1 false), !noalias !23324
  %1096 = icmp eq i64 %1094, -1
  br i1 %1096, label %.loopexit177, label %1097

1097:                                             ; preds = %1090
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !23272
  store i64 %1094, ptr %37, align 8, !noalias !23272
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %1017, ptr noundef nonnull align 8 dereferenceable(152) %38, i64 152, i1 false), !noalias !23272
  %1098 = load i64, ptr %1018, align 8, !noalias !23272
  %1099 = load ptr, ptr %1019, align 8, !noalias !23272
  %1100 = load i64, ptr %1020, align 8, !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !23272
  %1101 = load ptr, ptr %1017, align 8, !noalias !23272, !nonnull !1740, !noundef !1740
  %1102 = load i64, ptr %1021, align 8, !noalias !23272, !noundef !1740
  %1103 = icmp ult i64 %1102, 104811045873349726
  call void @llvm.assume(i1 %1103), !noalias !23276
  %1104 = getelementptr inbounds nuw [88 x i8], ptr %1101, i64 %1102
  store ptr %1101, ptr %36, align 8, !noalias !23272
  store i64 %1094, ptr %1022, align 8, !noalias !23272
  store ptr %1101, ptr %1023, align 8, !noalias !23272
  store ptr %1104, ptr %1024, align 8, !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %35), !noalias !23272
  %1105 = load i64, ptr %1025, align 8, !noalias !23272, !noundef !1740
  store i64 %1105, ptr %35, align 8, !noalias !23272
  %1106 = load ptr, ptr %1027, align 8, !noalias !23272, !nonnull !1740, !noundef !1740
  %1107 = load i64, ptr %1026, align 8, !range !1835, !noalias !23272, !noundef !1740
  %1108 = load i64, ptr %1028, align 8, !noalias !23272, !noundef !1740
  %1109 = icmp ult i64 %1108, 288230376151711744
  call void @llvm.assume(i1 %1109), !noalias !23276
  %1110 = shl nuw nsw i64 %1108, 5
  %1111 = getelementptr inbounds nuw i8, ptr %1106, i64 %1110
  %1112 = icmp eq i64 %1108, 0
  br i1 %1112, label %.loopexit176, label %1113

1113:                                             ; preds = %1097
  %1114 = icmp ult i64 %1100, 384307168202282326
  br label %1154

.loopexit177:                                     ; preds = %1209, %1090, %1009
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39)
          to label %1115 unwind label %1083, !noalias !23294

1115:                                             ; preds = %.loopexit177
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !23272
  %1116 = xor i1 %824, true
  %1117 = or i1 %853, %1116
  %1118 = select i1 %1117, i1 true, i1 %1004
  br i1 %1118, label %1119, label %1126

1119:                                             ; preds = %1128, %1115
  %1120 = load i64, ptr %45, align 8, !noalias !23272
  %1121 = load ptr, ptr %929, align 8, !noalias !23272
  %1122 = load i64, ptr %930, align 8, !noalias !23272
  %1123 = atomicrmw sub ptr %1001, i64 1 release, align 8, !noalias !23325
  %1124 = icmp eq i64 %1123, 1
  br i1 %1124, label %1125, label %1694

1125:                                             ; preds = %1119
  fence acquire, !noalias !23294
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #92
          to label %1694 unwind label %973

1126:                                             ; preds = %1115
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !23272
  store i64 1, ptr %25, align 8, !noalias !23272
  %1127 = getelementptr inbounds nuw i8, ptr %25, i64 8
  store i64 0, ptr %1127, align 8, !noalias !23272
; invoke <purrdf_sparql_eval::governor::GovernorState>::commit_reported_items
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(address) dereferenceable(40) %26, ptr noundef nonnull align 8 %1002, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %25, i64 noundef 1)
          to label %1128 unwind label %1083, !noalias !23294

1128:                                             ; preds = %1126
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !23272
  br label %1119

1129:                                             ; preds = %1372
  %1130 = landingpad { ptr, i32 }
          cleanup
  store ptr %1368, ptr %1023, align 8
  br label %1149

1131:                                             ; preds = %1391
  %1132 = landingpad { ptr, i32 }
          cleanup
  br label %1136

1133:                                             ; preds = %1461, %1445, %1440
  %1134 = phi ptr [ %1443, %1445 ], [ %1343, %1461 ], [ %1343, %1440 ]
  %1135 = landingpad { ptr, i32 }
          cleanup
  br label %1136

1136:                                             ; preds = %1133, %1131
  %1137 = phi ptr [ %1343, %1131 ], [ %1134, %1133 ]
  %1138 = phi { ptr, i32 } [ %1132, %1131 ], [ %1135, %1133 ]
  store ptr %1137, ptr %1023, align 8
  br label %1149

1139:                                             ; preds = %.preheader171
  %1140 = landingpad { ptr, i32 }
          cleanup
  br label %1149

1141:                                             ; preds = %1230
  %1142 = landingpad { ptr, i32 }
          cleanup
  br label %1149

1143:                                             ; preds = %1543, %1539, %1535, %1527, %1518, %1511, %1475
  %1144 = landingpad { ptr, i32 }
          cleanup
  br label %1149

1145:                                             ; preds = %1213
  %1146 = landingpad { ptr, i32 }
          cleanup
  br label %1149

1147:                                             ; preds = %.invoke, %1255
  %1148 = landingpad { ptr, i32 }
          cleanup
  br label %1149

1149:                                             ; preds = %1574, %1571, %1147, %1145, %1143, %1141, %1139, %1136, %1129
  %1150 = phi { ptr, i32 } [ %1572, %1571 ], [ %1572, %1574 ], [ %1130, %1129 ], [ %1138, %1136 ], [ %1148, %1147 ], [ %1140, %1139 ], [ %1142, %1141 ], [ %1144, %1143 ], [ %1146, %1145 ]
  %1151 = icmp eq i64 %1107, 0
  br i1 %1151, label %1172, label %1152

1152:                                             ; preds = %1149
  %1153 = shl nuw i64 %1107, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1106, i64 noundef %1153, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !23330
  br label %1172

1154:                                             ; preds = %.loopexit163, %1113
  %1155 = phi i64 [ 0, %1113 ], [ %1253, %.loopexit163 ]
  %1156 = phi ptr [ %1106, %1113 ], [ %1158, %.loopexit163 ]
  %1157 = phi ptr [ %1092, %1113 ], [ %1550, %.loopexit163 ]
  %1158 = getelementptr inbounds nuw i8, ptr %1156, i64 32
  %1159 = load i64, ptr %1156, align 8, !noalias !23333
  %1160 = getelementptr inbounds nuw i8, ptr %1156, i64 8
  %1161 = load i64, ptr %1160, align 8, !noalias !23333
  %1162 = getelementptr inbounds nuw i8, ptr %1156, i64 16
  %1163 = load i64, ptr %1162, align 8, !noalias !23333
  %1164 = getelementptr inbounds nuw i8, ptr %1156, i64 24
  %1165 = load i64, ptr %1164, align 8, !noalias !23333
  %1166 = icmp eq i64 %1159, 0
  %1167 = select i1 %1166, i1 true, i1 %1004
  br i1 %1167, label %1211, label %1218

.loopexit176:                                     ; preds = %.loopexit163, %1097
  %1168 = phi ptr [ %1092, %1097 ], [ %1550, %.loopexit163 ]
  %1169 = icmp eq i64 %1107, 0
  br i1 %1169, label %1173, label %1170

1170:                                             ; preds = %.loopexit176
  %1171 = shl nuw i64 %1107, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1106, i64 noundef %1171, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !23336
  br label %1173

1172:                                             ; preds = %1152, %1149
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36) #90
          to label %1174 unwind label %998, !noalias !23294

1173:                                             ; preds = %1170, %.loopexit176
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !23272
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36)
          to label %1184 unwind label %1180, !noalias !23294

1174:                                             ; preds = %1182, %1180, %1172
  %1175 = phi i1 [ true, %1172 ], [ true, %1180 ], [ false, %1182 ]
  %1176 = phi { ptr, i32 } [ %1150, %1172 ], [ %1181, %1180 ], [ %1183, %1182 ]
  %1177 = icmp eq i64 %1098, 0
  br i1 %1177, label %1188, label %1178

1178:                                             ; preds = %1174
  %1179 = mul nuw i64 %1098, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1099) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1099, i64 noundef %1179, i64 noundef range(i64 1, -9223372036854775807) 8) #93
  br label %1188

1180:                                             ; preds = %1173
  %1181 = landingpad { ptr, i32 }
          cleanup
  br label %1174

1182:                                             ; preds = %1595
  %1183 = landingpad { ptr, i32 }
          cleanup
  br label %1174

1184:                                             ; preds = %1173
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23272
  %1185 = icmp eq i64 %1098, 0
  br i1 %1185, label %1195, label %1186

1186:                                             ; preds = %1184
  %1187 = mul nuw i64 %1098, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1099) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1099, i64 noundef %1187, i64 noundef range(i64 1, -9223372036854775807) 8) #93
  br label %1195

1188:                                             ; preds = %1178, %1174
  call void @llvm.experimental.noalias.scope.decl(metadata !23339)
  %1189 = load ptr, ptr %1042, align 8, !alias.scope !23339, !noalias !23294, !noundef !1740
  %1190 = icmp eq ptr %1189, null
  br i1 %1190, label %1619, label %1191

1191:                                             ; preds = %1188
  %1192 = atomicrmw sub ptr %1189, i64 1 release, align 8, !noalias !23342
  %1193 = icmp eq i64 %1192, 1
  br i1 %1193, label %1194, label %1619

1194:                                             ; preds = %1191
  fence acquire, !noalias !23294
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1042) #92, !noalias !23294
  br label %1619

1195:                                             ; preds = %1186, %1184
  call void @llvm.experimental.noalias.scope.decl(metadata !23347)
  %1196 = load ptr, ptr %1042, align 8, !alias.scope !23347, !noalias !23294, !noundef !1740
  %1197 = icmp eq ptr %1196, null
  br i1 %1197, label %1202, label %1198

1198:                                             ; preds = %1195
  %1199 = atomicrmw sub ptr %1196, i64 1 release, align 8, !noalias !23350
  %1200 = icmp eq i64 %1199, 1
  br i1 %1200, label %1201, label %1202

1201:                                             ; preds = %1198
  fence acquire, !noalias !23294
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1042) #92, !noalias !23294
  br label %1202

1202:                                             ; preds = %1201, %1198, %1195
  call void @llvm.experimental.noalias.scope.decl(metadata !23355)
  %1203 = load ptr, ptr %1043, align 8, !alias.scope !23355, !noalias !23294, !noundef !1740
  %1204 = icmp eq ptr %1203, null
  br i1 %1204, label %1209, label %1205

1205:                                             ; preds = %1202
  %1206 = atomicrmw sub ptr %1203, i64 1 release, align 8, !noalias !23358
  %1207 = icmp eq i64 %1206, 1
  br i1 %1207, label %1208, label %1209

1208:                                             ; preds = %1205
  fence acquire, !noalias !23294
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1043) #92, !noalias !23294
  br label %1209

1209:                                             ; preds = %1208, %1205, %1202
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
  call void @llvm.lifetime.start.p0(ptr nonnull %38)
  %1210 = icmp eq ptr %1093, %1012
  br i1 %1210, label %.loopexit177, label %1090

1211:                                             ; preds = %1244, %1238, %1234, %1154
  call void @llvm.assume(i1 %1114), !noalias !23276
  %1212 = icmp ugt i64 %1155, %1100
  br i1 %1212, label %1213, label %1250, !prof !1742

1213:                                             ; preds = %1211
  call void @llvm.lifetime.start.p0(ptr nonnull %18)
  store i64 %1155, ptr %18, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %17)
  store i64 %1100, ptr %17, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %16)
  store ptr %18, ptr %16, align 8
  %1214 = getelementptr inbounds nuw i8, ptr %16, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %1214, align 8
  %1215 = getelementptr inbounds nuw i8, ptr %16, i64 16
  store ptr %17, ptr %1215, align 8
  %1216 = getelementptr inbounds nuw i8, ptr %16, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %1216, align 8
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.e5162873a9a3251d11c4df37a70e4654.2158, ptr noundef nonnull %16, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.289) #89
          to label %1217 unwind label %1145

1217:                                             ; preds = %1213
  unreachable

1218:                                             ; preds = %1154
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !23363
  %1219 = load atomic i64, ptr %1034 monotonic, align 8, !noalias !23370
  br label %1220

1220:                                             ; preds = %1220, %1218
  %1221 = phi i64 [ %1219, %1218 ], [ %1225, %1220 ]
  %1222 = call i64 @llvm.uadd.sat.i64(i64 %1221, i64 %1159)
  %1223 = cmpxchg weak ptr %1034, i64 %1221, i64 %1222 monotonic monotonic, align 8, !noalias !23370
  %1224 = extractvalue { i64, i1 } %1223, 1
  %1225 = extractvalue { i64, i1 } %1223, 0
  br i1 %1224, label %1226, label %1220

1226:                                             ; preds = %1220
  %1227 = call i64 @llvm.uadd.sat.i64(i64 %1225, i64 %1159)
  %1228 = load i64, ptr %1002, align 8, !noalias !23370
  %1229 = icmp ugt i64 %1227, %1228
  br i1 %1229, label %1230, label %1234

1230:                                             ; preds = %1226
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !23370
  store i8 0, ptr %1044, align 1, !noalias !23370
  store i64 %1228, ptr %1045, align 8, !noalias !23370
  store i64 %1227, ptr %1046, align 8, !noalias !23370
  store i8 0, ptr %14, align 8, !noalias !23370
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %15, ptr noundef nonnull align 8 %1002, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %14)
          to label %1231 unwind label %1141

1231:                                             ; preds = %1230
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !23370
  %1232 = load i8, ptr %15, align 8, !noalias !23363
  %1233 = icmp eq i8 %1232, -1
  br i1 %1233, label %1234, label %1237

1234:                                             ; preds = %1231, %1226
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !23363
  %1235 = load ptr, ptr %1029, align 8, !alias.scope !23292, !noalias !23293, !noundef !1740
  %1236 = icmp eq ptr %1235, null
  br i1 %1236, label %1211, label %1238

1237:                                             ; preds = %1231
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !23363
  br label %.loopexit175

1238:                                             ; preds = %1234
  %1239 = load i32, ptr %1030, align 4, !alias.scope !23292, !noalias !23293, !noundef !1740
  %1240 = getelementptr i8, ptr %1235, i64 56
  %1241 = load i64, ptr %1240, align 8, !noundef !1740
  %1242 = zext i32 %1239 to i64
  %1243 = icmp ugt i64 %1241, %1242
  br i1 %1243, label %1244, label %1211

1244:                                             ; preds = %1238
  %1245 = getelementptr i8, ptr %1235, i64 48
  %1246 = load ptr, ptr %1245, align 8, !nonnull !1740, !noundef !1740
  %1247 = getelementptr inbounds nuw [136 x i8], ptr %1246, i64 %1242
  %1248 = getelementptr inbounds nuw [8 x i8], ptr %1247, i64 %1031
  %1249 = atomicrmw add ptr %1248, i64 %1159 monotonic, align 8
  br label %1211

1250:                                             ; preds = %1211
  %1251 = icmp ult i64 %1161, %1155
  %1252 = call i64 @llvm.umin.i64(i64 %1161, i64 range(i64 0, 384307168202282326) %1100)
  %1253 = select i1 %1251, i64 %1155, i64 %1252
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1099) ]
  %1254 = icmp samesign ult i64 %1253, %1155
  br i1 %1254, label %1255, label %1256, !prof !10950

1255:                                             ; preds = %1250
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %1155, i64 noundef %1253, i64 noundef %1100, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.290) #94
          to label %1594 unwind label %1147, !noalias !23294

1256:                                             ; preds = %1250
  %1257 = mul nuw nsw i64 %1155, 24
  %1258 = getelementptr inbounds nuw i8, ptr %1099, i64 %1257
  %1259 = mul nuw nsw i64 %1253, 24
  %1260 = getelementptr inbounds nuw i8, ptr %1099, i64 %1259
  %1261 = icmp samesign eq i64 %1155, %1253
  br i1 %1261, label %.loopexit174, label %1262

1262:                                             ; preds = %1256
  %1263 = sub nuw nsw i64 %1259, %1257
  %1264 = udiv exact i64 %1263, 24
  br label %1265

1265:                                             ; preds = %1280, %1262
  %1266 = phi i64 [ 0, %1262 ], [ %1281, %1280 ]
  %1267 = phi i64 [ 0, %1262 ], [ %1282, %1280 ]
  %1268 = phi i64 [ 0, %1262 ], [ %1283, %1280 ]
  %1269 = phi i64 [ 0, %1262 ], [ %1284, %1280 ]
  %1270 = getelementptr inbounds nuw [24 x i8], ptr %1258, i64 %1269
  %1271 = load i8, ptr %1270, align 8, !range !11176, !noalias !23373, !noundef !1740
  %1272 = getelementptr i8, ptr %1270, i64 8
  %1273 = load i64, ptr %1272, align 8, !noalias !23373
  switch i8 %1271, label %.unreachabledefault [
    i8 0, label %1274
    i8 1, label %1276
    i8 2, label %1280
    i8 3, label %1278
  ]

.unreachabledefault:                              ; preds = %1265
  unreachable

default.unreachable857:                           ; preds = %1340
  unreachable

1274:                                             ; preds = %1265
  %1275 = call i64 @llvm.uadd.sat.i64(i64 %1268, i64 %1273)
  br label %1280

1276:                                             ; preds = %1265
  %1277 = call i64 @llvm.uadd.sat.i64(i64 %1267, i64 %1273)
  br label %1280

1278:                                             ; preds = %1265
  %1279 = call i64 @llvm.umax.i64(i64 %1266, i64 %1273)
  br label %1280

1280:                                             ; preds = %1278, %1276, %1274, %1265
  %1281 = phi i64 [ %1266, %1274 ], [ %1266, %1276 ], [ %1279, %1278 ], [ %1266, %1265 ]
  %1282 = phi i64 [ %1267, %1274 ], [ %1277, %1276 ], [ %1267, %1278 ], [ %1267, %1265 ]
  %1283 = phi i64 [ %1275, %1274 ], [ %1268, %1276 ], [ %1268, %1278 ], [ %1268, %1265 ]
  %1284 = add nuw i64 %1269, 1
  %1285 = icmp eq i64 %1284, %1264
  br i1 %1285, label %.loopexit174, label %1265

.loopexit174:                                     ; preds = %1280, %1256
  %1286 = phi i64 [ 0, %1256 ], [ %1282, %1280 ]
  %1287 = phi i64 [ 0, %1256 ], [ %1283, %1280 ]
  %1288 = phi i64 [ 0, %1256 ], [ %1281, %1280 ]
  br i1 %1007, label %.loopexit172, label %1292

.loopexit172:                                     ; preds = %1308, %1292, %.loopexit174
  %1289 = phi i64 [ 0, %.loopexit174 ], [ 0, %1292 ], [ %1310, %1308 ]
  %1290 = load atomic i32, ptr %1032 acquire, align 8, !noalias !23377
  %1291 = icmp eq i32 %1290, 0
  br i1 %1291, label %1312, label %1315

1292:                                             ; preds = %.loopexit174
  %1293 = load i64, ptr %35, align 8, !noalias !23272, !noundef !1740
  %1294 = load ptr, ptr %1023, align 8, !nonnull !1740, !noundef !1740
  %1295 = load ptr, ptr %1024, align 8, !nonnull !1740, !noundef !1740
  %1296 = ptrtoint ptr %1295 to i64
  %1297 = ptrtoint ptr %1294 to i64
  %1298 = sub nuw i64 %1296, %1297
  %1299 = call i64 @llvm.usub.sat.i64(i64 %1163, i64 %1293)
  %1300 = udiv exact i64 %1298, 88
  %1301 = call i64 @llvm.umin.i64(i64 %1299, i64 %1300)
  %1302 = icmp eq i64 %1301, 0
  br i1 %1302, label %.loopexit172, label %.preheader171

.preheader171:                                    ; preds = %1292, %1308
  %1303 = phi i64 [ %1310, %1308 ], [ 0, %1292 ]
  %1304 = phi i64 [ %1309, %1308 ], [ 0, %1292 ]
  %1305 = getelementptr inbounds nuw [88 x i8], ptr %1294, i64 %1304
  %1306 = getelementptr inbounds nuw i8, ptr %1305, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %1307 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %1306)
          to label %1308 unwind label %1139

1308:                                             ; preds = %.preheader171
  %1309 = add nuw nsw i64 %1304, 1
  %1310 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %1303, i64 %1307)
  %1311 = icmp eq i64 %1309, %1301
  br i1 %1311, label %.loopexit172, label %.preheader171

1312:                                             ; preds = %.loopexit172
  %1313 = load i8, ptr %1033, align 8, !noalias !23294
  %1314 = icmp eq i8 %1313, -1
  br i1 %1314, label %1315, label %1322

1315:                                             ; preds = %1312, %.loopexit172
  br i1 %1004, label %1316, label %1317

1316:                                             ; preds = %1317, %1315
  br i1 %1007, label %1327, label %1329

1317:                                             ; preds = %1315
  %1318 = load atomic i64, ptr %1034 monotonic, align 8, !noalias !23380
  %1319 = load i64, ptr %1002, align 8, !noalias !23380
  %1320 = call i64 @llvm.uadd.sat.i64(i64 %1318, i64 %1287)
  %1321 = icmp ugt i64 %1320, %1319
  br i1 %1321, label %1322, label %1316

1322:                                             ; preds = %1329, %1317, %1312
  %1323 = load i64, ptr %35, align 8
  %1324 = load ptr, ptr %1023, align 8
  br i1 %1261, label %.loopexit170, label %1325

1325:                                             ; preds = %1322
  %1326 = load ptr, ptr %1024, align 8, !nonnull !1740
  br label %1340

1327:                                             ; preds = %1329, %1316
  %1328 = or i1 %1004, %1261
  br i1 %1328, label %.loopexit167, label %.preheader166

1329:                                             ; preds = %1316
  %1330 = load i64, ptr %1035, align 8, !noalias !23294, !noundef !1740
  %1331 = load atomic i64, ptr %1036 monotonic, align 16, !noalias !23294
  %1332 = call noundef i64 @llvm.usub.sat.i64(i64 %1330, i64 %1331)
  %1333 = call i64 @llvm.uadd.sat.i64(i64 %1332, i64 %1289)
  %1334 = call i64 @llvm.uadd.sat.i64(i64 %1333, i64 %1286)
  %1335 = call i64 @llvm.uadd.sat.i64(i64 %1334, i64 %1288)
  %1336 = load atomic i64, ptr %1037 monotonic, align 8, !noalias !23383
  %1337 = load i64, ptr %1005, align 8, !noalias !23383
  %1338 = call i64 @llvm.uadd.sat.i64(i64 %1336, i64 %1335)
  %1339 = icmp ugt i64 %1338, %1337
  br i1 %1339, label %1322, label %1327

1340:                                             ; preds = %1463, %1325
  %1341 = phi ptr [ %1258, %1325 ], [ %1344, %1463 ]
  %1342 = phi i64 [ %1323, %1325 ], [ %1465, %1463 ]
  %1343 = phi ptr [ %1324, %1325 ], [ %1464, %1463 ]
  %1344 = getelementptr inbounds nuw i8, ptr %1341, i64 24
  %1345 = load i8, ptr %1341, align 8, !range !11176, !noalias !23294, !noundef !1740
  switch i8 %1345, label %default.unreachable857 [
    i8 0, label %1348
    i8 1, label %1355
    i8 2, label %1360
    i8 3, label %1378
  ]

.loopexit170:                                     ; preds = %1463, %1322
  %1346 = phi ptr [ %1324, %1322 ], [ %1464, %1463 ]
  %1347 = phi i64 [ %1323, %1322 ], [ %1465, %1463 ]
  store i64 %1347, ptr %35, align 8
  store ptr %1346, ptr %1023, align 8
  br i1 %1007, label %1471, label %1475

1348:                                             ; preds = %1340
  %1349 = getelementptr inbounds nuw i8, ptr %1341, i64 1
  %1350 = load i8, ptr %1349, align 1, !range !1741, !noalias !23294, !noundef !1740
  %1351 = getelementptr inbounds nuw i8, ptr %1341, i64 8
  %1352 = load i64, ptr %1351, align 8, !noalias !23294, !noundef !1740
  %1353 = getelementptr inbounds nuw i8, ptr %1341, i64 16
  %1354 = load i64, ptr %1353, align 8, !noalias !23294, !noundef !1740
  br i1 %1004, label %1463, label %1379

1355:                                             ; preds = %1340
  %1356 = getelementptr inbounds nuw i8, ptr %1341, i64 8
  %1357 = load i64, ptr %1356, align 8, !noalias !23294, !noundef !1740
  %1358 = getelementptr inbounds nuw i8, ptr %1341, i64 16
  %1359 = load i64, ptr %1358, align 8, !noalias !23294, !noundef !1740
  br i1 %1007, label %1463, label %1416

1360:                                             ; preds = %1340
  %1361 = getelementptr inbounds nuw i8, ptr %1341, i64 8
  %1362 = load i64, ptr %1361, align 8, !noalias !23294, !noundef !1740
  %1363 = icmp ult i64 %1342, %1362
  br i1 %1363, label %.preheader160, label %.loopexit161

.preheader160:                                    ; preds = %1360, %1375
  %1364 = phi i64 [ %1376, %1375 ], [ %1342, %1360 ]
  %1365 = phi ptr [ %1368, %1375 ], [ %1343, %1360 ]
  %1366 = icmp eq ptr %1365, %1326
  br i1 %1366, label %.loopexit161, label %1367

1367:                                             ; preds = %.preheader160
  %1368 = getelementptr inbounds nuw i8, ptr %1365, i64 88
  %1369 = getelementptr inbounds nuw i8, ptr %1365, i64 8
  %1370 = load i64, ptr %1369, align 8, !noalias !23386
  %1371 = icmp eq i64 %1370, -1
  br i1 %1371, label %.loopexit161, label %1372

1372:                                             ; preds = %1367
  %1373 = getelementptr inbounds nuw i8, ptr %1365, i64 16
  %1374 = load i64, ptr %1365, align 8, !noalias !23386
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !23389
  store i64 %1370, ptr %24, align 8, !noalias !23389
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1038, ptr noundef nonnull align 8 dereferenceable(72) %1373, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1039, i64 noundef %1374, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %24)
          to label %1375 unwind label %1129

1375:                                             ; preds = %1372
  %1376 = add nuw i64 %1364, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !23389
  %1377 = icmp eq i64 %1376, %1362
  br i1 %1377, label %.loopexit161, label %.preheader160

1378:                                             ; preds = %1340
  br i1 %1007, label %1463, label %1449

1379:                                             ; preds = %1348
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !23392
  %1380 = load atomic i64, ptr %1034 monotonic, align 8, !noalias !23399
  br label %1381

1381:                                             ; preds = %1381, %1379
  %1382 = phi i64 [ %1380, %1379 ], [ %1386, %1381 ]
  %1383 = call i64 @llvm.uadd.sat.i64(i64 %1382, i64 %1352)
  %1384 = cmpxchg weak ptr %1034, i64 %1382, i64 %1383 monotonic monotonic, align 8, !noalias !23399
  %1385 = extractvalue { i64, i1 } %1384, 1
  %1386 = extractvalue { i64, i1 } %1384, 0
  br i1 %1385, label %1387, label %1381

1387:                                             ; preds = %1381
  %1388 = call i64 @llvm.uadd.sat.i64(i64 %1386, i64 %1352)
  %1389 = load i64, ptr %1002, align 8, !noalias !23399
  %1390 = icmp ugt i64 %1388, %1389
  br i1 %1390, label %1391, label %1397

1391:                                             ; preds = %1387
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !23399
  store i8 0, ptr %1053, align 1, !noalias !23399
  store i64 %1389, ptr %1054, align 8, !noalias !23399
  store i64 %1388, ptr %1055, align 8, !noalias !23399
  store i8 0, ptr %12, align 8, !noalias !23399
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %13, ptr noundef nonnull align 8 %1002, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %12)
          to label %1392 unwind label %1131

1392:                                             ; preds = %1391
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !23399
  %1393 = load i8, ptr %13, align 8, !noalias !23392
  %1394 = icmp eq i8 %1393, -1
  br i1 %1394, label %1397, label %1395

1395:                                             ; preds = %1392
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !23392
  %1396 = icmp eq i64 %1354, 0
  br i1 %1396, label %.loopexit168, label %.invoke

1397:                                             ; preds = %1392, %1387
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !23392
  %1398 = icmp eq i8 %1350, -1
  br i1 %1398, label %1463, label %1399

1399:                                             ; preds = %1397
  %1400 = load ptr, ptr %1029, align 8, !alias.scope !23292, !noalias !23293, !noundef !1740
  %1401 = icmp eq ptr %1400, null
  br i1 %1401, label %1463, label %1402

1402:                                             ; preds = %1399
  %1403 = load i32, ptr %1030, align 4, !alias.scope !23292, !noalias !23293, !noundef !1740
  %1404 = getelementptr i8, ptr %1400, i64 56
  %1405 = load i64, ptr %1404, align 8, !noundef !1740
  %1406 = zext i32 %1403 to i64
  %1407 = icmp ugt i64 %1405, %1406
  br i1 %1407, label %1408, label %1463

1408:                                             ; preds = %1402
  %1409 = getelementptr i8, ptr %1400, i64 48
  %1410 = load ptr, ptr %1409, align 8, !nonnull !1740, !noundef !1740
  %1411 = zext nneg i8 %1350 to i64
  %1412 = getelementptr inbounds nuw [136 x i8], ptr %1410, i64 %1406
  %1413 = getelementptr inbounds nuw [8 x i8], ptr %1412, i64 %1411
  %1414 = atomicrmw add ptr %1413, i64 %1352 monotonic, align 8
  br label %1463

1415:                                             ; preds = %1419
  br i1 %1420, label %.loopexit168, label %1463

1416:                                             ; preds = %1355
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !23272
  %1417 = load i64, ptr %1005, align 8, !noalias !23294
  %1418 = icmp eq i64 %1417, -1
  br i1 %1418, label %1423, label %1424

1419:                                             ; preds = %1441, %1439
  %.pr = load i8, ptr %31, align 8, !noalias !23272
  %1420 = icmp ne i8 %.pr, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !23272
  %1421 = icmp ne i64 %1359, 0
  %1422 = and i1 %1421, %1420
  br i1 %1422, label %.invoke, label %1415

1423:                                             ; preds = %1416
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !23272
  br label %1463

1424:                                             ; preds = %1416
  %1425 = load atomic i32, ptr %1032 acquire, align 8, !noalias !23402
  %1426 = icmp eq i32 %1425, 0
  br i1 %1426, label %1439, label %1427

1427:                                             ; preds = %1424
  %1428 = load atomic i64, ptr %1037 monotonic, align 8, !noalias !23402
  br label %1429

1429:                                             ; preds = %1429, %1427
  %1430 = phi i64 [ %1428, %1427 ], [ %1434, %1429 ]
  %1431 = call i64 @llvm.uadd.sat.i64(i64 %1430, i64 %1357)
  %1432 = cmpxchg weak ptr %1037, i64 %1430, i64 %1431 monotonic monotonic, align 8, !noalias !23402
  %1433 = extractvalue { i64, i1 } %1432, 1
  %1434 = extractvalue { i64, i1 } %1432, 0
  br i1 %1433, label %1435, label %1429

1435:                                             ; preds = %1429
  %1436 = call i64 @llvm.uadd.sat.i64(i64 %1434, i64 %1357)
  %1437 = load i64, ptr %1005, align 8, !noalias !23402
  %1438 = icmp ugt i64 %1436, %1437
  br i1 %1438, label %1440, label %.thread150

1439:                                             ; preds = %1424
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %31, ptr noundef nonnull align 8 dereferenceable(24) %1033, i64 24, i1 false)
  br label %1419

.thread150:                                       ; preds = %1435
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !23272
  br label %1463

1440:                                             ; preds = %1435
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !23402
  store i8 3, ptr %1050, align 1, !noalias !23402
  store i64 %1437, ptr %1051, align 8, !noalias !23402
  store i64 %1436, ptr %1052, align 8, !noalias !23402
  store i8 0, ptr %11, align 8, !noalias !23402
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %31, ptr noundef nonnull align 8 %1002, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %11)
          to label %1441 unwind label %1133

1441:                                             ; preds = %1440
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !23402
  br label %1419

.invoke:                                          ; preds = %1419, %1395
  %.sink = phi i64 [ %1354, %1395 ], [ %1359, %1419 ]
  store i64 %1342, ptr %35, align 8
  store ptr %1343, ptr %1023, align 8
  %1442 = add i64 %.sink, -1
; invoke purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}(ptr nonnull %35, ptr nonnull %36, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i64 noundef %1442)
          to label %.loopexit175 unwind label %1147

.loopexit161:                                     ; preds = %1375, %1367, %.preheader160, %1360
  %1443 = phi ptr [ %1343, %1360 ], [ %1365, %.preheader160 ], [ %1368, %1367 ], [ %1368, %1375 ]
  %1444 = phi i64 [ %1342, %1360 ], [ %1364, %.preheader160 ], [ %1364, %1367 ], [ %1362, %1375 ]
  br i1 %1007, label %1463, label %1445

1445:                                             ; preds = %.loopexit161
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !23272
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %30, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %1446 unwind label %1133, !noalias !23294

1446:                                             ; preds = %1445
  %1447 = load i8, ptr %30, align 8, !range !1743, !noalias !23272, !noundef !1740
  %1448 = icmp eq i8 %1447, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !23272
  br i1 %1448, label %1463, label %.loopexit168

1449:                                             ; preds = %1378
  %1450 = getelementptr inbounds nuw i8, ptr %1341, i64 8
  %1451 = load i64, ptr %1450, align 8, !noalias !23294, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !23272
  %1452 = load atomic i32, ptr %1032 acquire, align 8, !noalias !23405
  %1453 = icmp eq i32 %1452, 0
  br i1 %1453, label %1454, label %1455

1454:                                             ; preds = %1449
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %29, ptr noundef nonnull align 8 dereferenceable(24) %1033, i64 24, i1 false), !noalias !23294
  br label %1467

1455:                                             ; preds = %1449
  %1456 = load atomic i64, ptr %1037 monotonic, align 8, !noalias !23405
  %1457 = call i64 @llvm.uadd.sat.i64(i64 %1456, i64 %1451)
  %1458 = load i64, ptr %1005, align 8, !noalias !23405
  %1459 = icmp ugt i64 %1457, %1458
  br i1 %1459, label %1461, label %1460

1460:                                             ; preds = %1455
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !23272
  br label %1463

1461:                                             ; preds = %1455
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !23405
  store i8 3, ptr %1047, align 1, !noalias !23405
  store i64 %1458, ptr %1048, align 8, !noalias !23405
  store i64 %1457, ptr %1049, align 8, !noalias !23405
  store i8 0, ptr %10, align 8, !noalias !23405
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %29, ptr noundef nonnull align 8 %1002, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %10)
          to label %1462 unwind label %1133

1462:                                             ; preds = %1461
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !23405
  br label %1467

1463:                                             ; preds = %.thread150, %1467, %1460, %1446, %.loopexit161, %1423, %1415, %1408, %1402, %1399, %1397, %1378, %1355, %1348
  %1464 = phi ptr [ %1343, %1399 ], [ %1343, %1402 ], [ %1343, %1408 ], [ %1343, %1355 ], [ %1343, %1348 ], [ %1443, %.loopexit161 ], [ %1343, %1415 ], [ %1343, %1378 ], [ %1343, %1467 ], [ %1443, %1446 ], [ %1343, %1423 ], [ %1343, %1460 ], [ %1343, %1397 ], [ %1343, %.thread150 ]
  %1465 = phi i64 [ %1342, %1399 ], [ %1342, %1402 ], [ %1342, %1408 ], [ %1342, %1355 ], [ %1342, %1348 ], [ %1444, %.loopexit161 ], [ %1342, %1415 ], [ %1342, %1378 ], [ %1342, %1467 ], [ %1444, %1446 ], [ %1342, %1423 ], [ %1342, %1460 ], [ %1342, %1397 ], [ %1342, %.thread150 ]
  %1466 = icmp eq ptr %1344, %1260
  br i1 %1466, label %.loopexit170, label %1340

1467:                                             ; preds = %1462, %1454
  %1468 = load i8, ptr %29, align 8, !noalias !23272
  %1469 = icmp eq i8 %1468, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !23272
  br i1 %1469, label %1463, label %.loopexit168

.loopexit168:                                     ; preds = %1467, %1446, %1415, %1395
  %1470 = phi ptr [ %1343, %1395 ], [ %1343, %1467 ], [ %1343, %1415 ], [ %1443, %1446 ]
  store ptr %1470, ptr %1023, align 8
  br label %.loopexit175

1471:                                             ; preds = %1526, %1517, %1475, %.loopexit170
  %1472 = icmp eq i64 %1165, 0
  br i1 %1472, label %.loopexit163, label %1473

1473:                                             ; preds = %1471
  %1474 = load ptr, ptr %935, align 8, !alias.scope !23408, !noalias !23411, !nonnull !1740, !noundef !1740
  br label %1545

1475:                                             ; preds = %.loopexit170
; invoke purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}(ptr nonnull %35, ptr nonnull %36, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i64 noundef %1163)
          to label %1471 unwind label %1143

.loopexit167:                                     ; preds = %1491, %1327
  %1476 = icmp eq i64 %1155, %1253
  br i1 %1476, label %.loopexit165, label %.lr.ph

1477:                                             ; preds = %.lr.ph
  %1478 = icmp eq ptr %1258, %1480
  br i1 %1478, label %.loopexit165, label %.lr.ph

.lr.ph:                                           ; preds = %.loopexit167, %1477
  %1479 = phi ptr [ %1480, %1477 ], [ %1260, %.loopexit167 ]
  %1480 = getelementptr inbounds i8, ptr %1479, i64 -24
  %1481 = load i8, ptr %1480, align 8, !range !11176, !noalias !23413, !noundef !1740
  %1482 = icmp eq i8 %1481, 2
  br i1 %1482, label %1511, label %1477

.preheader166:                                    ; preds = %1327, %1491
  %1483 = phi ptr [ %1484, %1491 ], [ %1258, %1327 ]
  %1484 = getelementptr inbounds nuw i8, ptr %1483, i64 24
  %1485 = load i8, ptr %1483, align 8, !range !11176, !noalias !23294, !noundef !1740
  %1486 = icmp eq i8 %1485, 0
  br i1 %1486, label %1487, label %1491

1487:                                             ; preds = %.preheader166
  %1488 = getelementptr inbounds nuw i8, ptr %1483, i64 1
  %1489 = load i8, ptr %1488, align 1, !range !1741, !noalias !23294, !noundef !1740
  %1490 = icmp eq i8 %1489, -1
  br i1 %1490, label %1491, label %1493

1491:                                             ; preds = %1502, %1496, %1493, %1487, %.preheader166
  %1492 = icmp eq ptr %1484, %1260
  br i1 %1492, label %.loopexit167, label %.preheader166

1493:                                             ; preds = %1487
  %1494 = load ptr, ptr %1029, align 8, !alias.scope !23292, !noalias !23293, !noundef !1740
  %1495 = icmp eq ptr %1494, null
  br i1 %1495, label %1491, label %1496

1496:                                             ; preds = %1493
  %1497 = load i32, ptr %1030, align 4, !alias.scope !23292, !noalias !23293, !noundef !1740
  %1498 = getelementptr i8, ptr %1494, i64 56
  %1499 = load i64, ptr %1498, align 8, !noundef !1740
  %1500 = zext i32 %1497 to i64
  %1501 = icmp ugt i64 %1499, %1500
  br i1 %1501, label %1502, label %1491

1502:                                             ; preds = %1496
  %1503 = getelementptr i8, ptr %1494, i64 48
  %1504 = load ptr, ptr %1503, align 8, !nonnull !1740, !noundef !1740
  %1505 = getelementptr inbounds nuw i8, ptr %1483, i64 8
  %1506 = load i64, ptr %1505, align 8, !noalias !23294, !noundef !1740
  %1507 = zext nneg i8 %1489 to i64
  %1508 = getelementptr inbounds nuw [136 x i8], ptr %1504, i64 %1500
  %1509 = getelementptr inbounds nuw [8 x i8], ptr %1508, i64 %1507
  %1510 = atomicrmw add ptr %1509, i64 %1506 monotonic, align 8
  br label %1491

1511:                                             ; preds = %.lr.ph
  %1512 = getelementptr i8, ptr %1479, i64 -16
  %1513 = load i64, ptr %1512, align 8, !noalias !23413
; invoke purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}(ptr nonnull %35, ptr nonnull %36, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i64 noundef %1513)
          to label %.loopexit165 unwind label %1143

.loopexit165:                                     ; preds = %1477, %.loopexit167, %1511
  %1514 = phi i1 [ false, %1511 ], [ true, %.loopexit167 ], [ true, %1477 ]
  %1515 = icmp eq i64 %1287, 0
  %1516 = or i1 %1004, %1515
  br i1 %1516, label %1517, label %1518

1517:                                             ; preds = %1519, %.loopexit165
  br i1 %1007, label %1471, label %1523

1518:                                             ; preds = %.loopexit165
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !23272
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !23272
  store i64 %1287, ptr %27, align 8, !noalias !23272
  store i64 0, ptr %1040, align 8, !noalias !23272
; invoke <purrdf_sparql_eval::governor::GovernorState>::commit_reported_items
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(address) dereferenceable(40) %28, ptr noundef nonnull align 8 %1002, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %27, i64 noundef 1)
          to label %1519 unwind label %1143, !noalias !23294

1519:                                             ; preds = %1518
  %1520 = load i8, ptr %1041, align 8, !range !1743, !noalias !23272, !noundef !1740
  %1521 = icmp eq i8 %1520, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %27), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %28), !noalias !23272
  br i1 %1521, label %1517, label %1522

1522:                                             ; preds = %1519
  br i1 %1007, label %.loopexit175, label %1543

1523:                                             ; preds = %1517
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !23272
  %1524 = load i64, ptr %1005, align 8, !noalias !23294
  %1525 = icmp eq i64 %1524, -1
  br i1 %1525, label %1528, label %1527

1526:                                             ; preds = %1543
  br i1 %1544, label %.loopexit175, label %1471

1527:                                             ; preds = %1523
; invoke <purrdf_sparql_eval::governor::GovernorState>::charge_work
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %34, ptr noundef nonnull align 8 %1002, i8 noundef 3, i64 noundef %1286, i1 noundef zeroext false)
          to label %1529 unwind label %1143

1528:                                             ; preds = %1529, %1523
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !23272
  br i1 %1514, label %1533, label %1535

1529:                                             ; preds = %1527
  %1530 = load i8, ptr %34, align 8, !range !1743, !noalias !23272, !noundef !1740
  %1531 = icmp eq i8 %1530, -1
  br i1 %1531, label %1528, label %1532

1532:                                             ; preds = %1529
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !23272
  br label %1543

1533:                                             ; preds = %1536, %1528
  %1534 = icmp eq i64 %1288, 0
  br i1 %1534, label %1543, label %1539

1535:                                             ; preds = %1528
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !23272
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %33, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %1536 unwind label %1143, !noalias !23294

1536:                                             ; preds = %1535
  %1537 = load i8, ptr %33, align 8, !range !1743, !noalias !23272, !noundef !1740
  %1538 = icmp eq i8 %1537, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !23272
  br i1 %1538, label %1533, label %1543

1539:                                             ; preds = %1533
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !23272
; invoke <purrdf_sparql_eval::governor::GovernorState>::admit_transient
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::admit_transient(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %32, ptr noundef nonnull align 8 %1002, i8 noundef 3, i64 noundef %1288)
          to label %1540 unwind label %1143, !noalias !23294

1540:                                             ; preds = %1539
  %1541 = load i8, ptr %32, align 8, !range !1743, !noalias !23272, !noundef !1740
  %1542 = icmp ne i8 %1541, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !23272
  br label %1543

1543:                                             ; preds = %1540, %1536, %1533, %1532, %1522
  %1544 = phi i1 [ false, %1533 ], [ %1542, %1540 ], [ true, %1522 ], [ true, %1532 ], [ true, %1536 ]
; invoke purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}
  invoke fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#5}(ptr nonnull %35, ptr nonnull %36, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i64 noundef %1163)
          to label %1526 unwind label %1143

1545:                                             ; preds = %1577, %1473
  %1546 = phi i64 [ %1165, %1473 ], [ %1548, %1577 ]
  %1547 = phi ptr [ %1157, %1473 ], [ %1553, %1577 ]
  %1548 = add i64 %1546, -1
  call void @llvm.experimental.noalias.scope.decl(metadata !23408)
  %1549 = icmp eq ptr %1547, %1474
  br i1 %1549, label %.loopexit163, label %1552

.loopexit163:                                     ; preds = %1577, %1552, %1545, %1471
  %1550 = phi ptr [ %1157, %1471 ], [ %1547, %1545 ], [ %1553, %1552 ], [ %1553, %1577 ]
  store ptr %1550, ptr %934, align 8
  %1551 = icmp eq ptr %1158, %1111
  br i1 %1551, label %.loopexit176, label %1154

1552:                                             ; preds = %1545
  %1553 = getelementptr inbounds nuw i8, ptr %1547, i64 40
  %1554 = load i64, ptr %1547, align 8, !noalias !23416
  %1555 = getelementptr inbounds nuw i8, ptr %1547, i64 8
  %1556 = load ptr, ptr %1555, align 8, !noalias !23416
  %1557 = getelementptr inbounds nuw i8, ptr %1547, i64 16
  %1558 = load i64, ptr %1557, align 8, !noalias !23416
  %1559 = getelementptr inbounds nuw i8, ptr %1547, i64 24
  %1560 = load i8, ptr %1559, align 8, !noalias !23416
  %1561 = getelementptr inbounds nuw i8, ptr %1547, i64 25
  %1562 = load i56, ptr %1561, align 1, !noalias !23416
  %1563 = getelementptr inbounds nuw i8, ptr %1547, i64 32
  %1564 = load i64, ptr %1563, align 8, !noalias !23416
  %1565 = icmp eq i64 %1554, 0
  br i1 %1565, label %.loopexit163, label %1566

1566:                                             ; preds = %1552
  call void @llvm.experimental.noalias.scope.decl(metadata !23417)
  %1567 = load i64, ptr %930, align 8, !alias.scope !23417, !noalias !23420, !noundef !1740
  %1568 = load i64, ptr %45, align 8, !range !1835, !alias.scope !23417, !noalias !23420, !noundef !1740
  %1569 = icmp eq i64 %1567, %1568
  br i1 %1569, label %1570, label %1577

1570:                                             ; preds = %1566
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %45)
          to label %1577 unwind label %1571, !noalias !23420

1571:                                             ; preds = %1570
  %1572 = landingpad { ptr, i32 }
          cleanup
  store ptr %1553, ptr %934, align 8
  %1573 = icmp ugt i64 %1554, 5
  br i1 %1573, label %1574, label %1149

1574:                                             ; preds = %1571
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1556) ]
  %1575 = shl i64 %1554, 3
  %1576 = add i64 %1575, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1556, i64 noundef %1576, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !23422
  br label %1149

1577:                                             ; preds = %1570, %1566
  %1578 = load ptr, ptr %929, align 8, !alias.scope !23417, !noalias !23420, !nonnull !1740, !noundef !1740
  %1579 = getelementptr inbounds nuw [40 x i8], ptr %1578, i64 %1567
  store i64 %1554, ptr %1579, align 8, !noalias !23425
  %1580 = getelementptr inbounds nuw i8, ptr %1579, i64 8
  store ptr %1556, ptr %1580, align 8, !noalias !23425
  %1581 = getelementptr inbounds nuw i8, ptr %1579, i64 16
  store i64 %1558, ptr %1581, align 8, !noalias !23425
  %1582 = getelementptr inbounds nuw i8, ptr %1579, i64 24
  store i8 %1560, ptr %1582, align 8, !noalias !23425
  %1583 = getelementptr inbounds nuw i8, ptr %1579, i64 25
  store i56 %1562, ptr %1583, align 1, !noalias !23425
  %1584 = getelementptr inbounds nuw i8, ptr %1579, i64 32
  store i64 %1564, ptr %1584, align 8, !noalias !23425
  %1585 = add i64 %1567, 1
  store i64 %1585, ptr %930, align 8, !alias.scope !23417, !noalias !23420
  %1586 = icmp eq i64 %1548, 0
  br i1 %1586, label %.loopexit163, label %1545

.loopexit175:                                     ; preds = %1526, %1522, %.invoke, %.loopexit168, %1237
  %1587 = phi i8 [ 1, %.invoke ], [ 0, %1237 ], [ 1, %.loopexit168 ], [ 1, %1522 ], [ 1, %1526 ]
  %1588 = load ptr, ptr %929, align 8, !noalias !23272
  %1589 = load i64, ptr %930, align 8, !noalias !23272
  %1590 = load i64, ptr %45, align 8, !noalias !23272
  %1591 = icmp eq i64 %1107, 0
  br i1 %1591, label %1595, label %1592

1592:                                             ; preds = %.loopexit175
  %1593 = shl nuw i64 %1107, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1106, i64 noundef %1593, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !23426
  br label %1595

1594:                                             ; preds = %1255
  unreachable

1595:                                             ; preds = %1592, %.loopexit175
  call void @llvm.lifetime.end.p0(ptr nonnull %35), !noalias !23272
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %36)
          to label %1596 unwind label %1182, !noalias !23294

1596:                                             ; preds = %1595
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !23272
  %1597 = icmp eq i64 %1098, 0
  br i1 %1597, label %1600, label %1598

1598:                                             ; preds = %1596
  %1599 = mul nuw i64 %1098, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1099) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1099, i64 noundef %1599, i64 noundef range(i64 1, -9223372036854775807) 8) #93
  br label %1600

1600:                                             ; preds = %1598, %1596
  call void @llvm.experimental.noalias.scope.decl(metadata !23429)
  %1601 = load ptr, ptr %1042, align 8, !alias.scope !23429, !noalias !23294, !noundef !1740
  %1602 = icmp eq ptr %1601, null
  br i1 %1602, label %1607, label %1603

1603:                                             ; preds = %1600
  %1604 = atomicrmw sub ptr %1601, i64 1 release, align 8, !noalias !23432
  %1605 = icmp eq i64 %1604, 1
  br i1 %1605, label %1606, label %1607

1606:                                             ; preds = %1603
  fence acquire, !noalias !23294
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1042) #92, !noalias !23294
  br label %1607

1607:                                             ; preds = %1606, %1603, %1600
  call void @llvm.experimental.noalias.scope.decl(metadata !23437)
  %1608 = load ptr, ptr %1043, align 8, !alias.scope !23437, !noalias !23294, !noundef !1740
  %1609 = icmp eq ptr %1608, null
  br i1 %1609, label %1614, label %1610

1610:                                             ; preds = %1607
  %1611 = atomicrmw sub ptr %1608, i64 1 release, align 8, !noalias !23440
  %1612 = icmp eq i64 %1611, 1
  br i1 %1612, label %1613, label %1614

1613:                                             ; preds = %1610
  fence acquire, !noalias !23294
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1043) #92, !noalias !23294
  br label %1614

1614:                                             ; preds = %1613, %1610, %1607
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %39)
          to label %1615 unwind label %1083, !noalias !23294

1615:                                             ; preds = %1614
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !23272
  %1616 = atomicrmw sub ptr %1001, i64 1 release, align 8, !noalias !23445
  %1617 = icmp eq i64 %1616, 1
  br i1 %1617, label %1618, label %1694

1618:                                             ; preds = %1615
  fence acquire, !noalias !23294
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #92
          to label %1694 unwind label %973

1619:                                             ; preds = %1194, %1191, %1188
  call void @llvm.experimental.noalias.scope.decl(metadata !23450)
  %1620 = load ptr, ptr %1043, align 8, !alias.scope !23450, !noalias !23294, !noundef !1740
  %1621 = icmp eq ptr %1620, null
  br i1 %1621, label %1089, label %1622

1622:                                             ; preds = %1619
  %1623 = atomicrmw sub ptr %1620, i64 1 release, align 8, !noalias !23453
  %1624 = icmp eq i64 %1623, 1
  br i1 %1624, label %1625, label %1089

1625:                                             ; preds = %1622
  fence acquire, !noalias !23294
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1043) #92, !noalias !23294
  br label %1089

1626:                                             ; preds = %1082, %1076
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42) #90, !noalias !23294
  br i1 %1077, label %1627, label %1628

1627:                                             ; preds = %1626
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %45) #90, !noalias !23294
  br i1 %1078, label %1692, label %2327

1628:                                             ; preds = %1626
  br i1 %1078, label %1692, label %2327

1629:                                             ; preds = %968, %925
  %1630 = phi i64 [ %971, %968 ], [ %866, %925 ]
  %1631 = phi ptr [ %970, %968 ], [ %868, %925 ]
  %1632 = phi i64 [ %969, %968 ], [ %870, %925 ]
  %1633 = phi i64 [ undef, %968 ], [ %877, %925 ]
  %1634 = phi i64 [ 0, %968 ], [ %927, %925 ]
  %1635 = phi i8 [ 2, %968 ], [ %873, %925 ]
  %1636 = phi i64 [ -1, %968 ], [ %863, %925 ]
  %1637 = icmp eq i64 %807, 0
  br i1 %1637, label %._crit_edge, label %.lr.ph1313

1638:                                             ; preds = %.lr.ph1313
  %1639 = icmp eq i64 %1642, %807
  br i1 %1639, label %._crit_edge, label %.lr.ph1313

.lr.ph1313:                                       ; preds = %1629, %1638
  %1640 = phi i64 [ %1642, %1638 ], [ 0, %1629 ]
  %1641 = getelementptr inbounds nuw [160 x i8], ptr %808, i64 %1640
  %1642 = add nuw nsw i64 %1640, 1
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %1641)
          to label %1638 unwind label %1646, !noalias !23458

1643:                                             ; preds = %.lr.ph1315
  %1644 = add i64 %1649, 1
  %1645 = icmp eq i64 %1644, %807
  br i1 %1645, label %._crit_edge1316, label %.lr.ph1315

1646:                                             ; preds = %.lr.ph1313
  %1647 = landingpad { ptr, i32 }
          cleanup
  %1648 = icmp eq i64 %1642, %807
  br i1 %1648, label %._crit_edge1316, label %.lr.ph1315

.lr.ph1315:                                       ; preds = %1646, %1643
  %1649 = phi i64 [ %1644, %1643 ], [ %1642, %1646 ]
  %1650 = getelementptr inbounds nuw [160 x i8], ptr %808, i64 %1649
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %1650) #90
          to label %1643 unwind label %1651, !noalias !23458

1651:                                             ; preds = %.lr.ph1315
  %1652 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !23461
  unreachable

._crit_edge1316:                                  ; preds = %1643, %1646
  %1653 = icmp eq i64 %821, 0
  br i1 %1653, label %2327, label %1654

1654:                                             ; preds = %._crit_edge1316
  %1655 = mul nuw i64 %821, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %808, i64 noundef %1655, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !23458
  br label %2327

._crit_edge:                                      ; preds = %1638, %1629
  %1656 = icmp eq i64 %821, 0
  br i1 %1656, label %1699, label %1657

1657:                                             ; preds = %._crit_edge
  %1658 = mul nuw i64 %821, 160
  %1659 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1660 = load i64, ptr %1659, align 8, !noalias !23458, !noundef !1740
  %1661 = call i64 @llvm.umin.i64(i64 %1658, i64 9223372036854775807)
  %1662 = call i64 @llvm.ssub.sat.i64(i64 %1660, i64 %1661)
  store i64 %1662, ptr %1659, align 8, !noalias !23458
  %1663 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1664 = load i64, ptr %1663, align 8, !noalias !23458, !noundef !1740
  %1665 = icmp slt i64 %1662, %1664
  br i1 %1665, label %1666, label %.preheader1491

1666:                                             ; preds = %1657
  store i64 %1662, ptr %1663, align 8, !noalias !23458
  br label %.preheader1491

.preheader1491:                                   ; preds = %1666, %1657
  br label %1667

1667:                                             ; preds = %.preheader1491, %1670
  %1668 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23458
  %1669 = icmp slt i64 %1668, 0
  br i1 %1669, label %1670, label %__rustc::__rust_dealloc (.exit141)

1670:                                             ; preds = %1667
  %1671 = add nsw i64 %1668, 1
  %1672 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1668, i64 %1671 acq_rel acquire, align 8, !noalias !23458
  %1673 = extractvalue { i64, i1 } %1672, 1
  br i1 %1673, label %1674, label %1667

1674:                                             ; preds = %1670
  %1675 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1661 monotonic, align 8, !noalias !23458
  %1676 = call i64 @llvm.ssub.sat.i64(i64 %1675, i64 %1661)
  %1677 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23458
  br label %1678

1678:                                             ; preds = %1681, %1674
  %1679 = phi i64 [ %1677, %1674 ], [ %1684, %1681 ]
  %1680 = icmp slt i64 %1676, %1679
  br i1 %1680, label %1681, label %1685

1681:                                             ; preds = %1678
  %1682 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1679, i64 %1676 monotonic monotonic, align 8, !noalias !23458
  %1683 = extractvalue { i64, i1 } %1682, 1
  %1684 = extractvalue { i64, i1 } %1682, 0
  br i1 %1683, label %1685, label %1678

1685:                                             ; preds = %1681, %1678
  %1686 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23458
  br label %__rustc::__rust_dealloc (.exit141)

__rustc::__rust_dealloc (.exit141): ; preds = %1667, %1685
  call void @free(ptr noundef nonnull %808) #93, !noalias !23458
  br label %1699

1687:                                             ; preds = %.loopexit183
  %1688 = landingpad { ptr, i32 }
          cleanup
  br label %1689

1689:                                             ; preds = %1687, %947
  %1690 = phi ptr [ %45, %947 ], [ %54, %1687 ]
  %1691 = phi { ptr, i32 } [ %981, %947 ], [ %1688, %1687 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %1690) #90, !noalias !23282
  br label %1692

1692:                                             ; preds = %1689, %1628, %1627
  %1693 = phi { ptr, i32 } [ %1079, %1627 ], [ %1079, %1628 ], [ %1691, %1689 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %50) #90
          to label %2327 unwind label %998, !noalias !23310

1694:                                             ; preds = %1618, %1615, %1125, %1119
  %1695 = phi i64 [ %1590, %1615 ], [ %1590, %1618 ], [ %1120, %1119 ], [ %1120, %1125 ]
  %1696 = phi ptr [ %1588, %1615 ], [ %1588, %1618 ], [ %1121, %1119 ], [ %1121, %1125 ]
  %1697 = phi i64 [ %1589, %1615 ], [ %1589, %1618 ], [ %1122, %1119 ], [ %1122, %1125 ]
  %1698 = phi i8 [ %1587, %1615 ], [ %1587, %1618 ], [ 2, %1119 ], [ 2, %1125 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !23272
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %42), !noalias !23294
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !23272
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !23227
  br label %1701

1699:                                             ; preds = %__rustc::__rust_dealloc (.exit141), %._crit_edge
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !23227
  %1700 = icmp eq i64 %1636, -1
  br i1 %1700, label %1701, label %1748

1701:                                             ; preds = %1699, %1694
  %1702 = phi i8 [ %1698, %1694 ], [ %1635, %1699 ]
  %1703 = phi i64 [ %1697, %1694 ], [ %1632, %1699 ]
  %1704 = phi ptr [ %1696, %1694 ], [ %1631, %1699 ]
  %1705 = phi i64 [ %1695, %1694 ], [ %1630, %1699 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %51)
  call void @llvm.lifetime.start.p0(ptr nonnull %49), !noalias !23227
  store i64 %1705, ptr %49, align 8, !noalias !23227
  %1706 = getelementptr inbounds nuw i8, ptr %49, i64 8
  store ptr %1704, ptr %1706, align 8, !noalias !23227
  %1707 = getelementptr inbounds nuw i8, ptr %49, i64 16
  store i64 %1703, ptr %1707, align 8, !noalias !23227
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  %1708 = load ptr, ptr %593, align 8, !noalias !23464, !noundef !1740
  %1709 = icmp eq ptr %1708, null
  br i1 %1709, label %1720, label %1710

1710:                                             ; preds = %1701
  %1711 = getelementptr inbounds nuw i8, ptr %1708, i64 296
  %1712 = load atomic i32, ptr %1711 acquire, align 4, !noalias !23465
  %1713 = icmp eq i32 %1712, 0
  br i1 %1713, label %1714, label %1718

1714:                                             ; preds = %1710
  %1715 = getelementptr inbounds nuw i8, ptr %1708, i64 272
  %1716 = load i8, ptr %1715, align 8, !noalias !23464
  %1717 = getelementptr inbounds nuw i8, ptr %1708, i64 273
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %9, ptr noundef nonnull align 1 dereferenceable(23) %1717, i64 23, i1 false), !noalias !23464
  br label %1718

1718:                                             ; preds = %1714, %1710
  %1719 = phi i8 [ %1716, %1714 ], [ -1, %1710 ]
  switch i8 %1702, label %1724 [
    i8 2, label %1741
    i8 0, label %1723
  ]

1720:                                             ; preds = %1701
  %1721 = icmp eq i8 %1702, 2
  %1722 = and i1 %1721, %853
  br label %1741

1723:                                             ; preds = %1738, %1726, %1718
  br label %1741

1724:                                             ; preds = %1718
  %1725 = icmp eq i8 %1719, -1
  br i1 %1725, label %1741, label %1726

1726:                                             ; preds = %1724
  %1727 = load i8, ptr %198, align 8, !range !3730, !noalias !23464, !noundef !1740
  %1728 = icmp eq i8 %1727, 2
  br i1 %1728, label %1729, label %1723

1729:                                             ; preds = %1726
  %1730 = getelementptr inbounds nuw i8, ptr %4, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !23468), !noalias !23464
  %1731 = load ptr, ptr %1730, align 8, !alias.scope !23468, !noalias !23471, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !23473
  store i8 %1719, ptr %8, align 8, !noalias !23477
  %1732 = getelementptr inbounds nuw i8, ptr %8, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %1732, ptr noundef nonnull align 1 dereferenceable(23) %9, i64 23, i1 false), !noalias !23464
  %1733 = getelementptr inbounds nuw i8, ptr %1731, i64 40
  %1734 = load atomic i32, ptr %1733 acquire, align 4, !noalias !23473
  %1735 = icmp eq i32 %1734, 0
  br i1 %1735, label %1738, label %1736, !prof !1953

1736:                                             ; preds = %1729
  %1737 = getelementptr inbounds nuw i8, ptr %1731, i64 16
; invoke <std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !>
  invoke fastcc void @<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.11631829254914579133)(ptr noundef nonnull align 8 %1737, ptr noundef nonnull align 8 %8)
          to label %1738 unwind label %1739

1738:                                             ; preds = %1736, %1729
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !23473
  br label %1723

1739:                                             ; preds = %1736
  %1740 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %49) #90, !noalias !23464
  br label %2327

1741:                                             ; preds = %1724, %1723, %1720, %1718
  %1742 = phi i8 [ -1, %1720 ], [ -1, %1724 ], [ %1719, %1723 ], [ %1719, %1718 ]
  %1743 = phi i1 [ %1722, %1720 ], [ false, %1724 ], [ false, %1723 ], [ %853, %1718 ]
  %1744 = icmp eq i8 %1742, -1
  %1745 = select i1 %1743, i1 %1744, i1 false
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !23227
  call void @llvm.lifetime.end.p0(ptr nonnull %53), !noalias !23227
  call void @llvm.lifetime.end.p0(ptr nonnull %54), !noalias !23224
  br label %1758

1746:                                             ; preds = %768, %693
  %1747 = phi { ptr, i32 } [ %769, %768 ], [ %694, %693 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %54) #90, !noalias !23478
  br label %2327

1748:                                             ; preds = %1699
  %1749 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %1749, ptr noundef nonnull align 16 dereferenceable(48) %51, i64 48, i1 false), !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %51)
  call void @llvm.lifetime.end.p0(ptr nonnull %53), !noalias !23227
  call void @llvm.lifetime.end.p0(ptr nonnull %54), !noalias !23224
  %1750 = zext i8 %1635 to i64
  %1751 = or disjoint i64 %1634, %1750
  %1752 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %1636, ptr %1752, align 16, !alias.scope !23112, !noalias !23160
  %1753 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %1630, ptr %1753, align 8, !noalias !23160
  %1754 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store ptr %1631, ptr %1754, align 16, !noalias !23160
  %1755 = getelementptr inbounds nuw i8, ptr %0, i64 40
  store i64 %1632, ptr %1755, align 8, !noalias !23160
  %1756 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %1751, ptr %1756, align 16, !alias.scope !23112, !noalias !23160
  %1757 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i64 %1633, ptr %1757, align 8, !alias.scope !23112, !noalias !23160
  store i64 1, ptr %0, align 16, !alias.scope !23112, !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %74), !noalias !23119
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %75)
          to label %2100 unwind label %661, !noalias !23124

1758:                                             ; preds = %1741, %671
  %1759 = phi i64 [ %852, %1741 ], [ undef, %671 ]
  %1760 = phi i64 [ %1703, %1741 ], [ %639, %671 ]
  %1761 = phi ptr [ %1704, %1741 ], [ %660, %671 ]
  %1762 = phi i64 [ %1705, %1741 ], [ %627, %671 ]
  %1763 = phi i1 [ %1745, %1741 ], [ false, %671 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %71), !noalias !23119
  store i64 %1762, ptr %71, align 8, !noalias !23119
  %1764 = getelementptr inbounds nuw i8, ptr %71, i64 8
  store ptr %1761, ptr %1764, align 8, !noalias !23119
  %1765 = getelementptr inbounds nuw i8, ptr %71, i64 16
  store i64 %1760, ptr %1765, align 8, !noalias !23119
  %1766 = load i64, ptr %75, align 8, !noalias !23119
  %1767 = getelementptr inbounds nuw i8, ptr %75, i64 8
  %1768 = load i64, ptr %1767, align 8, !noalias !23119
  %1769 = getelementptr inbounds nuw i8, ptr %75, i64 16
  %1770 = load i64, ptr %1769, align 8, !noalias !23119
  %1771 = getelementptr inbounds nuw i8, ptr %75, i64 24
  %1772 = load i64, ptr %1771, align 8, !noalias !23119
  %1773 = icmp ugt i64 %1766, 2
  %1774 = select i1 %1773, i64 %1770, i64 %1766
  %1775 = add i64 %1774, -1
  %1776 = select i1 %1773, i64 %1766, i64 1
  %1777 = select i1 %1773, i64 1, i64 %1770
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !23479
  store i64 %1776, ptr %23, align 8, !noalias !23483
  %1778 = getelementptr inbounds nuw i8, ptr %23, i64 8
  store i64 %1768, ptr %1778, align 8, !noalias !23483
  %1779 = getelementptr inbounds nuw i8, ptr %23, i64 16
  store i64 %1777, ptr %1779, align 8, !noalias !23483
  %1780 = getelementptr inbounds nuw i8, ptr %23, i64 24
  store i64 %1772, ptr %1780, align 8, !noalias !23483
  %1781 = getelementptr inbounds nuw i8, ptr %23, i64 32
  store i64 0, ptr %1781, align 8, !noalias !23479
  %1782 = getelementptr inbounds nuw i8, ptr %23, i64 40
  store i64 %1775, ptr %1782, align 8, !noalias !23479
  %1783 = icmp eq i64 %1775, 0
  br i1 %1783, label %.loopexit157, label %1784

1784:                                             ; preds = %1758
  %1785 = inttoptr i64 %1768 to ptr
  %1786 = select i1 %1773, ptr %1785, ptr %1778
  %1787 = getelementptr inbounds nuw i8, ptr %4, i64 640
  br label %1790

1788:                                             ; preds = %1790
  %1789 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %23) #90
          to label %1798 unwind label %1796, !noalias !23484

1790:                                             ; preds = %1794, %1784
  %1791 = phi i64 [ 0, %1784 ], [ %1792, %1794 ]
  %1792 = add nuw i64 %1791, 1
  store i64 %1792, ptr %1781, align 8, !alias.scope !23485, !noalias !23488
  %1793 = getelementptr inbounds nuw [24 x i8], ptr %1786, i64 %1791
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !23479
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %22, ptr noundef nonnull align 8 dereferenceable(24) %1793, i64 24, i1 false), !noalias !23484
; invoke <purrdf_sparql_eval::witness::RelationWitness>::merge
  invoke void @<purrdf_sparql_eval::witness::RelationWitness>::merge(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %1787, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %22)
          to label %1794 unwind label %1788, !noalias !23484

.loopexit157:                                     ; preds = %1794, %1758
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(48) %23)
          to label %1802 unwind label %1800

1794:                                             ; preds = %1790
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !23479
  %1795 = icmp eq i64 %1792, %1775
  br i1 %1795, label %.loopexit157, label %1790

1796:                                             ; preds = %1788
  %1797 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !23484
  unreachable

1798:                                             ; preds = %1877, %1800, %1788
  %1799 = phi { ptr, i32 } [ %1789, %1788 ], [ %1801, %1800 ], [ %1878, %1877 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %71) #90, !noalias !23124
  br label %604

1800:                                             ; preds = %.loopexit156, %1921, %1854, %.loopexit157
  %1801 = landingpad { ptr, i32 }
          cleanup
  br label %1798

1802:                                             ; preds = %.loopexit157
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !23479
  br i1 %1763, label %1803, label %1806

1803:                                             ; preds = %1802
  %1804 = load i64, ptr %220, align 8, !alias.scope !23115, !noalias !23123, !noundef !1740
  %1805 = icmp ugt i64 %1759, %1804
  br i1 %1805, label %1854, label %1813, !prof !1742

1806:                                             ; preds = %2101, %1802
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %82, ptr noundef nonnull align 8 dereferenceable(24) %71, i64 24, i1 false), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %74), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %75), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %80), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23490)
  %1807 = load ptr, ptr %81, align 8, !alias.scope !23490, !noalias !23124, !noundef !1740
  %1808 = icmp eq ptr %1807, null
  br i1 %1808, label %2103, label %1809

1809:                                             ; preds = %1806
  %1810 = atomicrmw sub ptr %1807, i64 1 release, align 8, !noalias !23493
  %1811 = icmp eq i64 %1810, 1
  br i1 %1811, label %1812, label %2103

1812:                                             ; preds = %1809
  fence acquire, !noalias !23124
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %81) #92
          to label %2103 unwind label %566, !inline_history !2025

1813:                                             ; preds = %1803
  %1814 = load ptr, ptr %589, align 8, !alias.scope !23115, !noalias !23123, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %70), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23496)
  %1815 = load ptr, ptr %593, align 8, !noalias !23499, !noundef !1740
  %1816 = icmp eq ptr %1815, null
  br i1 %1816, label %1840, label %1817

1817:                                             ; preds = %1813
  %1818 = getelementptr inbounds nuw i8, ptr %1815, i64 16
  %1819 = load i64, ptr %1818, align 8, !noalias !23124
  %1820 = icmp ne i64 %1819, -1
  %1821 = getelementptr inbounds nuw i8, ptr %1815, i64 40
  %1822 = load i64, ptr %1821, align 8, !noalias !23124
  %1823 = icmp ne i64 %1822, -1
  %1824 = getelementptr inbounds nuw i8, ptr %1815, i64 336
  %1825 = load ptr, ptr %1824, align 8, !noalias !23124, !noundef !1740
  %1826 = icmp ne ptr %1825, null
  %1827 = select i1 %1826, i1 true, i1 %1820
  %1828 = select i1 %1827, i1 true, i1 %1823
  %1829 = getelementptr inbounds nuw i8, ptr %70, i64 192
  %1830 = getelementptr inbounds nuw i8, ptr %70, i64 160
  %1831 = getelementptr inbounds nuw i8, ptr %70, i64 184
  %1832 = getelementptr inbounds nuw i8, ptr %70, i64 8
  %1833 = getelementptr inbounds nuw i8, ptr %70, i64 16
  %1834 = getelementptr inbounds nuw i8, ptr %70, i64 32
  %1835 = getelementptr inbounds nuw i8, ptr %70, i64 40
  %1836 = getelementptr inbounds nuw i8, ptr %70, i64 56
  %1837 = getelementptr inbounds nuw i8, ptr %70, i64 64
  %1838 = getelementptr inbounds nuw i8, ptr %70, i64 72
  %1839 = getelementptr inbounds nuw i8, ptr %70, i64 88
  br i1 %1828, label %1853, label %1852

1840:                                             ; preds = %1813
  %1841 = getelementptr inbounds nuw i8, ptr %70, i64 192
  %1842 = getelementptr inbounds nuw i8, ptr %70, i64 160
  %1843 = getelementptr inbounds nuw i8, ptr %70, i64 184
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1842, i8 0, i64 24, i1 false), !alias.scope !23496, !noalias !23124
  store i64 -1, ptr %1843, align 8, !alias.scope !23496, !noalias !23124
  store <4 x i8> <i8 0, i8 0, i8 0, i8 4>, ptr %1841, align 8, !alias.scope !23496, !noalias !23124
  store i64 0, ptr %70, align 8, !alias.scope !23496, !noalias !23124
  %1844 = getelementptr inbounds nuw i8, ptr %70, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1844, align 8, !alias.scope !23496, !noalias !23124
  %1845 = getelementptr inbounds nuw i8, ptr %70, i64 16
  %1846 = getelementptr inbounds nuw i8, ptr %70, i64 32
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1845, i8 0, i64 16, i1 false), !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1846, align 8, !alias.scope !23496, !noalias !23124
  %1847 = getelementptr inbounds nuw i8, ptr %70, i64 40
  %1848 = getelementptr inbounds nuw i8, ptr %70, i64 56
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1847, i8 0, i64 16, i1 false), !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1848, align 8, !alias.scope !23496, !noalias !23124
  %1849 = getelementptr inbounds nuw i8, ptr %70, i64 64
  store i64 0, ptr %1849, align 8, !alias.scope !23496, !noalias !23124
  %1850 = getelementptr inbounds nuw i8, ptr %70, i64 72
  %1851 = getelementptr inbounds nuw i8, ptr %70, i64 88
  br label %1855

1852:                                             ; preds = %1817
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1830, i8 0, i64 24, i1 false), !alias.scope !23496, !noalias !23124
  store i64 -1, ptr %1831, align 8, !alias.scope !23496, !noalias !23124
  store <4 x i8> <i8 0, i8 0, i8 0, i8 4>, ptr %1829, align 8, !alias.scope !23496, !noalias !23124
  store i64 0, ptr %70, align 8, !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1832, align 8, !alias.scope !23496, !noalias !23124
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1833, i8 0, i64 16, i1 false), !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1834, align 8, !alias.scope !23496, !noalias !23124
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1835, i8 0, i64 16, i1 false), !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1836, align 8, !alias.scope !23496, !noalias !23124
  store i64 0, ptr %1837, align 8, !alias.scope !23496, !noalias !23124
  br label %1855

1853:                                             ; preds = %1817
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1830, i8 0, i64 24, i1 false), !alias.scope !23496, !noalias !23124
  store i64 -1, ptr %1831, align 8, !alias.scope !23496, !noalias !23124
  store <4 x i8> <i8 0, i8 0, i8 1, i8 4>, ptr %1829, align 8, !alias.scope !23496, !noalias !23124
  store i64 0, ptr %70, align 8, !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1832, align 8, !alias.scope !23496, !noalias !23124
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1833, i8 0, i64 16, i1 false), !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1834, align 8, !alias.scope !23496, !noalias !23124
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1835, i8 0, i64 16, i1 false), !alias.scope !23496, !noalias !23124
  store ptr inttoptr (i64 8 to ptr), ptr %1836, align 8, !alias.scope !23496, !noalias !23124
  store i64 0, ptr %1837, align 8, !alias.scope !23496, !noalias !23124
  br label %1855

1854:                                             ; preds = %1803
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %1759, i64 noundef %1804, i64 noundef %1804, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.405) #94
          to label %2102 unwind label %1800, !noalias !23124

1855:                                             ; preds = %1853, %1852, %1840
  %1856 = phi ptr [ %1838, %1853 ], [ %1838, %1852 ], [ %1850, %1840 ]
  %1857 = phi ptr [ %1839, %1853 ], [ %1839, %1852 ], [ %1851, %1840 ]
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1856, i8 -1, i64 16, i1 false), !alias.scope !23496, !noalias !23124
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(65) %1857, i8 0, i64 65, i1 false), !alias.scope !23496, !noalias !23124
  %1858 = getelementptr inbounds nuw [40 x i8], ptr %1814, i64 %1804
  %1859 = icmp samesign eq i64 %1759, %1804
  br i1 %1859, label %.loopexit156, label %1860

1860:                                             ; preds = %1855
  %1861 = getelementptr inbounds nuw [40 x i8], ptr %1814, i64 %1759
  %1862 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %1863 = getelementptr inbounds nuw i8, ptr %21, i64 16
  %1864 = getelementptr inbounds nuw i8, ptr %21, i64 24
  %1865 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %1866 = getelementptr inbounds nuw i8, ptr %7, i64 12
  %1867 = getelementptr inbounds nuw i8, ptr %6, i64 8
  %wide.gep1453 = getelementptr inbounds nuw [8 x i8], ptr %1862, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.gep1454 = getelementptr inbounds nuw i8, <4 x ptr> %wide.gep1453, i64 4
  br label %1868

1868:                                             ; preds = %2073, %1860
  %1869 = phi ptr [ %1761, %1860 ], [ %2074, %2073 ]
  %1870 = phi i64 [ %1760, %1860 ], [ %2075, %2073 ]
  %1871 = phi ptr [ %1861, %1860 ], [ %1872, %2073 ]
  %1872 = getelementptr inbounds nuw i8, ptr %1871, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %69), !noalias !23119
; invoke <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::pass::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %69, ptr noalias nofree noundef align 8 dereferenceable(200) %70, ptr noundef nonnull align 16 dereferenceable(1248) %4)
          to label %1879 unwind label %1873, !noalias !23124

1873:                                             ; preds = %1910, %1892, %1868
  %1874 = landingpad { ptr, i32 }
          cleanup
  br label %1877

1875:                                             ; preds = %1964
  %1876 = landingpad { ptr, i32 }
          cleanup
  br label %1877

1877:                                             ; preds = %2089, %2086, %1875, %1873
  %1878 = phi { ptr, i32 } [ %2087, %2086 ], [ %2087, %2089 ], [ %1874, %1873 ], [ %1876, %1875 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %70)
          to label %1798 unwind label %583

1879:                                             ; preds = %1868
  %1880 = load i8, ptr %69, align 8, !range !1743, !noalias !23119, !noundef !1740
  %1881 = icmp eq i8 %1880, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !23119
  br i1 %1881, label %1882, label %.loopexit156

1882:                                             ; preds = %1879
  call void @llvm.lifetime.start.p0(ptr nonnull %68)
  %1883 = load i64, ptr %1871, align 8, !range !1778, !noalias !23124, !noundef !1740
  %1884 = add i64 %1883, -1
  %1885 = icmp ugt i64 %1884, 4
  %1886 = getelementptr inbounds nuw i8, ptr %1871, i64 8
  br i1 %1885, label %1887, label %1892

1887:                                             ; preds = %1882
  %1888 = load ptr, ptr %1886, align 8, !noalias !23124, !nonnull !1740, !noundef !1740
  %1889 = getelementptr inbounds nuw i8, ptr %1871, i64 16
  %1890 = load i64, ptr %1889, align 8, !noalias !23124, !noundef !1740
  %1891 = add i64 %1890, -1
  br label %1892

1892:                                             ; preds = %1887, %1882
  %1893 = phi i64 [ %1891, %1887 ], [ %1884, %1882 ]
  %1894 = phi ptr [ %1888, %1887 ], [ %1886, %1882 ]
  %1895 = load ptr, ptr %85, align 8, !noalias !23119, !nonnull !1740, !noundef !1740
  %1896 = getelementptr inbounds nuw i8, ptr %1895, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !23500
; invoke <purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>::term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %7, ptr noalias nofree noundef nonnull align 8 dereferenceable(216) %83, ptr noalias nofree noundef nonnull readonly align 4 captures(address, read_provenance) %1894, i64 noundef range(i64 0, 1152921504606846976) %1893, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(56) %1896, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4)
          to label %1897 unwind label %1873, !inline_history !23155

1897:                                             ; preds = %1892
  %1898 = load i64, ptr %7, align 16, !range !2527, !noalias !23500, !noundef !1740
  %1899 = icmp eq i64 %1898, -1
  %1900 = load i32, ptr %1865, align 8, !noalias !23500
  %1901 = load i32, ptr %1866, align 4, !noalias !23500
  br i1 %1899, label %1907, label %1902

1902:                                             ; preds = %1897
  %1903 = getelementptr inbounds nuw i8, ptr %7, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %68, ptr noundef nonnull align 16 dereferenceable(80) %1903, i64 80, i1 false), !noalias !23507
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !23500
  %1904 = trunc i32 %1900 to i8
  %1905 = lshr i32 %1900, 8
  %1906 = trunc nuw i32 %1905 to i24
  br label %1921

1907:                                             ; preds = %1897
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !23500
  %1908 = icmp eq i32 %1900, 2
  br i1 %1908, label %1909, label %1910

1909:                                             ; preds = %1907
  call void @llvm.lifetime.end.p0(ptr nonnull %68)
  br label %2073

1910:                                             ; preds = %1907
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !23500
; invoke purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::expr::ebv_term::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %6, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, i32 noundef %1900, i32 noundef %1901)
          to label %1911 unwind label %1873, !inline_history !23155

1911:                                             ; preds = %1910
  %1912 = load i64, ptr %6, align 16, !range !2527, !noalias !23500, !noundef !1740
  %1913 = icmp eq i64 %1912, -1
  %1914 = load i8, ptr %1867, align 8, !noalias !23500
  br i1 %1913, label %1931, label %1915

1915:                                             ; preds = %1911
  %1916 = getelementptr inbounds nuw i8, ptr %6, i64 9
  %1917 = load i24, ptr %1916, align 1, !noalias !23507
  %1918 = getelementptr inbounds nuw i8, ptr %6, i64 12
  %1919 = load i32, ptr %1918, align 4, !noalias !23507
  %1920 = getelementptr inbounds nuw i8, ptr %6, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %68, ptr noundef nonnull align 16 dereferenceable(80) %1920, i64 80, i1 false), !noalias !23507
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !23500
  br label %1921

1921:                                             ; preds = %1915, %1902
  %1922 = phi i24 [ %1906, %1902 ], [ %1917, %1915 ]
  %1923 = phi i8 [ %1904, %1902 ], [ %1914, %1915 ]
  %1924 = phi i32 [ %1901, %1902 ], [ %1919, %1915 ]
  %1925 = phi i64 [ %1898, %1902 ], [ %1912, %1915 ]
  %1926 = getelementptr inbounds nuw i8, ptr %0, i64 25
  store i24 %1922, ptr %1926, align 1, !noalias !23160
  %1927 = getelementptr inbounds nuw i8, ptr %0, i64 28
  store i32 %1924, ptr %1927, align 4, !noalias !23160
  %1928 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %1928, ptr noundef nonnull align 16 dereferenceable(80) %68, i64 80, i1 false), !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %68)
  %1929 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %1925, ptr %1929, align 16, !alias.scope !23112, !noalias !23160
  %1930 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %1923, ptr %1930, align 8, !alias.scope !23112, !noalias !23160
  store i64 1, ptr %0, align 16, !alias.scope !23112, !noalias !23160
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %70)
          to label %2099 unwind label %1800

1931:                                             ; preds = %1911
  call void @llvm.lifetime.end.p0(ptr nonnull %6), !noalias !23500
  call void @llvm.lifetime.end.p0(ptr nonnull %68)
  %1932 = and i8 %1914, 1
  %1933 = icmp eq i8 %1932, 0
  br i1 %1933, label %2073, label %1934

1934:                                             ; preds = %1931
  call void @llvm.lifetime.start.p0(ptr nonnull %67)
  call void @llvm.experimental.noalias.scope.decl(metadata !23508)
  %1935 = load i64, ptr %1871, align 8, !range !1778, !alias.scope !23508, !noalias !23511, !noundef !1740
  %1936 = add i64 %1935, -1
  %1937 = icmp ugt i64 %1936, 4
  %1938 = getelementptr inbounds nuw i8, ptr %1871, i64 16
  %1939 = load i64, ptr %1938, align 8, !alias.scope !23508, !noalias !23511
  %1940 = add i64 %1939, -1
  %1941 = select i1 %1937, i64 %1940, i64 %1936
  %1942 = icmp ugt i64 %1941, 4
  br i1 %1942, label %1952, label %1943

1943:                                             ; preds = %1934
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !23513
  %1944 = icmp ugt i64 %1935, 5
  %1945 = load ptr, ptr %1886, align 8, !alias.scope !23508, !noalias !23511, !nonnull !1740
  %1946 = select i1 %1944, ptr %1945, ptr %1886
  %1947 = icmp eq i64 %1941, 0
  br i1 %1947, label %1960, label %vector.body1446

vector.body1446:                                  ; preds = %1943
  %trip.count.minus.1 = add nsw i64 %1941, -1
  %broadcast.splatinsert1444 = insertelement <4 x i64> poison, i64 %trip.count.minus.1, i64 0
  %broadcast.splat1445 = shufflevector <4 x i64> %broadcast.splatinsert1444, <4 x i64> poison, <4 x i32> zeroinitializer
  %1948 = icmp uge <4 x i64> %broadcast.splat1445, <i64 0, i64 1, i64 2, i64 3>
  %wide.gep1449 = getelementptr inbounds nuw [8 x i8], ptr %1946, <4 x i64> <i64 0, i64 1, i64 2, i64 3>
  %wide.masked.gather1450 = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep1449, <4 x i1> %1948, <4 x i32> poison), !noalias !23511
  %wide.gep1451 = getelementptr i8, <4 x ptr> %wide.gep1449, i64 4
  %wide.masked.gather1452 = call <4 x i32> @llvm.masked.gather.v4i32.v4p0(<4 x ptr> align 4 %wide.gep1451, <4 x i1> %1948, <4 x i32> poison), !noalias !23511
  %1949 = icmp eq <4 x i32> %wide.masked.gather1450, splat (i32 2)
  %1950 = select <4 x i1> %1949, <4 x i32> undef, <4 x i32> %wide.masked.gather1452
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %wide.masked.gather1450, <4 x ptr> align 4 %wide.gep1453, <4 x i1> %1948), !noalias !23513
  call void @llvm.masked.scatter.v4i32.v4p0(<4 x i32> %1950, <4 x ptr> align 4 %wide.gep1454, <4 x i1> %1948), !noalias !23513
  %1951 = add nuw nsw i64 %1941, 1
  br label %1960

1952:                                             ; preds = %1934
  %1953 = shl i64 %1941, 3
  %1954 = icmp ugt i64 %1941, 2305843009213693951
  %1955 = icmp ugt i64 %1953, 9223372036854775804
  %1956 = or i1 %1954, %1955
  br i1 %1956, label %1964, label %1957, !prof !6191

1957:                                             ; preds = %1952
; call __rustc::__rust_alloc
  %1958 = call noundef align 4 ptr @__rustc::__rust_alloc(i64 noundef %1953, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !23514
  %1959 = icmp eq ptr %1958, null
  br i1 %1959, label %1964, label %iter.check1423

1960:                                             ; preds = %vector.body1446, %1943
  %1961 = phi i64 [ 1, %1943 ], [ %1951, %vector.body1446 ]
  store i64 %1961, ptr %21, align 8, !noalias !23513
  %1962 = load ptr, ptr %1862, align 8, !noalias !23517
  %1963 = load i64, ptr %1863, align 8, !noalias !23517
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %67, ptr noundef nonnull align 8 dereferenceable(16) %1864, i64 16, i1 false), !noalias !23517
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !23513
  br label %2077

1964:                                             ; preds = %1957, %1952
  %1965 = phi i64 [ 4, %1957 ], [ 0, %1952 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %1965, i64 %1953) #94
          to label %1966 unwind label %1875

1966:                                             ; preds = %1964
  unreachable

iter.check1423:                                   ; preds = %1957
  %1967 = load ptr, ptr %1886, align 8, !alias.scope !23508, !noalias !23511, !nonnull !1740
  %1968 = select i1 %1937, ptr %1967, ptr %1886
  %min.iters.check1406 = icmp ult i64 %1941, 8
  br i1 %min.iters.check1406, label %vec.epilog.scalar.ph1424.preheader, label %vector.memcheck

vector.memcheck:                                  ; preds = %iter.check1423
  %scevgep = getelementptr i8, ptr %1958, i64 %1953
  %scevgep1405 = getelementptr i8, ptr %1968, i64 %1953
  %bound0 = icmp ult ptr %1958, %scevgep1405
  %bound1 = icmp ult ptr %1968, %scevgep
  %found.conflict = and i1 %bound0, %bound1
  br i1 %found.conflict, label %vec.epilog.scalar.ph1424.preheader, label %vector.main.loop.iter.check1407

vector.main.loop.iter.check1407:                  ; preds = %vector.memcheck
  %min.iters.check1408 = icmp ult i64 %1941, 32
  br i1 %min.iters.check1408, label %vec.epilog.ph1427, label %vector.ph1409

vector.ph1409:                                    ; preds = %vector.main.loop.iter.check1407
  %n.mod.vf1410 = and i64 %1941, 24
  %n.vec1411 = and i64 %1941, 2305843009213693920
  br label %vector.body1412

vector.body1412:                                  ; preds = %vector.body1412, %vector.ph1409
  %index1413 = phi i64 [ 0, %vector.ph1409 ], [ %index.next1419, %vector.body1412 ]
  %1969 = or disjoint i64 %index1413, 16
  %1970 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %index1413
  %1971 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %1969
  %wide.vec = load <32 x i32>, ptr %1970, align 4, !alias.scope !23518, !noalias !23521
  %strided.vec = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec1414 = shufflevector <32 x i32> %wide.vec, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %wide.vec1415 = load <32 x i32>, ptr %1971, align 4, !alias.scope !23518, !noalias !23521
  %strided.vec1416 = shufflevector <32 x i32> %wide.vec1415, <32 x i32> poison, <16 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14, i32 16, i32 18, i32 20, i32 22, i32 24, i32 26, i32 28, i32 30>
  %strided.vec1417 = shufflevector <32 x i32> %wide.vec1415, <32 x i32> poison, <16 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15, i32 17, i32 19, i32 21, i32 23, i32 25, i32 27, i32 29, i32 31>
  %1972 = icmp eq <16 x i32> %strided.vec, splat (i32 2)
  %1973 = icmp eq <16 x i32> %strided.vec1416, splat (i32 2)
  %1974 = select <16 x i1> %1972, <16 x i32> undef, <16 x i32> %strided.vec1414
  %1975 = select <16 x i1> %1973, <16 x i32> undef, <16 x i32> %strided.vec1417
  %1976 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %index1413
  %1977 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %1969
  %interleaved.vec = shufflevector <16 x i32> %strided.vec, <16 x i32> %1974, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec, ptr %1976, align 4, !alias.scope !23534, !noalias !23536
  %interleaved.vec1418 = shufflevector <16 x i32> %strided.vec1416, <16 x i32> %1975, <32 x i32> <i32 0, i32 16, i32 1, i32 17, i32 2, i32 18, i32 3, i32 19, i32 4, i32 20, i32 5, i32 21, i32 6, i32 22, i32 7, i32 23, i32 8, i32 24, i32 9, i32 25, i32 10, i32 26, i32 11, i32 27, i32 12, i32 28, i32 13, i32 29, i32 14, i32 30, i32 15, i32 31>
  store <32 x i32> %interleaved.vec1418, ptr %1977, align 4, !alias.scope !23534, !noalias !23536
  %index.next1419 = add nuw i64 %index1413, 32
  %1978 = icmp eq i64 %index.next1419, %n.vec1411
  br i1 %1978, label %middle.block1420, label %vector.body1412, !llvm.loop !23543

middle.block1420:                                 ; preds = %vector.body1412
  %ind.escape = add nsw i64 %n.vec1411, -1
  %cmp.n1421 = icmp eq i64 %1941, %n.vec1411
  br i1 %cmp.n1421, label %.loopexit1458, label %vec.epilog.iter.check1425

vec.epilog.iter.check1425:                        ; preds = %middle.block1420
  %min.epilog.iters.check1426 = icmp eq i64 %n.mod.vf1410, 0
  br i1 %min.epilog.iters.check1426, label %vec.epilog.scalar.ph1424.preheader, label %vec.epilog.ph1427, !prof !11068

vec.epilog.ph1427:                                ; preds = %vector.main.loop.iter.check1407, %vec.epilog.iter.check1425
  %vec.epilog.resume.val1422 = phi i64 [ %n.vec1411, %vec.epilog.iter.check1425 ], [ 0, %vector.main.loop.iter.check1407 ]
  %n.vec1429 = and i64 %1941, 2305843009213693944
  br label %vec.epilog.vector.body1430

vec.epilog.vector.body1430:                       ; preds = %vec.epilog.vector.body1430, %vec.epilog.ph1427
  %index1431 = phi i64 [ %vec.epilog.resume.val1422, %vec.epilog.ph1427 ], [ %index.next1436, %vec.epilog.vector.body1430 ]
  %1979 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %index1431
  %wide.vec1432 = load <16 x i32>, ptr %1979, align 4, !alias.scope !23518, !noalias !23521
  %strided.vec1433 = shufflevector <16 x i32> %wide.vec1432, <16 x i32> poison, <8 x i32> <i32 0, i32 2, i32 4, i32 6, i32 8, i32 10, i32 12, i32 14>
  %strided.vec1434 = shufflevector <16 x i32> %wide.vec1432, <16 x i32> poison, <8 x i32> <i32 1, i32 3, i32 5, i32 7, i32 9, i32 11, i32 13, i32 15>
  %1980 = icmp eq <8 x i32> %strided.vec1433, splat (i32 2)
  %1981 = select <8 x i1> %1980, <8 x i32> undef, <8 x i32> %strided.vec1434
  %1982 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %index1431
  %interleaved.vec1435 = shufflevector <8 x i32> %strided.vec1433, <8 x i32> %1981, <16 x i32> <i32 0, i32 8, i32 1, i32 9, i32 2, i32 10, i32 3, i32 11, i32 4, i32 12, i32 5, i32 13, i32 6, i32 14, i32 7, i32 15>
  store <16 x i32> %interleaved.vec1435, ptr %1982, align 4, !alias.scope !23534, !noalias !23536
  %index.next1436 = add nuw i64 %index1431, 8
  %1983 = icmp eq i64 %index.next1436, %n.vec1429
  br i1 %1983, label %vec.epilog.middle.block1437, label %vec.epilog.vector.body1430, !llvm.loop !23544

vec.epilog.middle.block1437:                      ; preds = %vec.epilog.vector.body1430
  %ind.escape1438 = add nsw i64 %n.vec1429, -1
  %cmp.n1439 = icmp eq i64 %1941, %n.vec1429
  br i1 %cmp.n1439, label %.loopexit1458, label %vec.epilog.scalar.ph1424.preheader

vec.epilog.scalar.ph1424.preheader:               ; preds = %vector.memcheck, %iter.check1423, %vec.epilog.iter.check1425, %vec.epilog.middle.block1437
  %.ph = phi i64 [ 0, %iter.check1423 ], [ 0, %vector.memcheck ], [ %n.vec1411, %vec.epilog.iter.check1425 ], [ %n.vec1429, %vec.epilog.middle.block1437 ]
  %xtraiter1741 = and i64 %1941, 7
  %lcmp.mod1742.not = icmp eq i64 %xtraiter1741, 0
  br i1 %lcmp.mod1742.not, label %vec.epilog.scalar.ph1424.prol.loopexit, label %vec.epilog.scalar.ph1424.prol

vec.epilog.scalar.ph1424.prol:                    ; preds = %vec.epilog.scalar.ph1424.preheader, %vec.epilog.scalar.ph1424.prol
  %1984 = phi i64 [ %1993, %vec.epilog.scalar.ph1424.prol ], [ %.ph, %vec.epilog.scalar.ph1424.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %vec.epilog.scalar.ph1424.prol ], [ 0, %vec.epilog.scalar.ph1424.preheader ]
  %1985 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %1984
  %1986 = load i32, ptr %1985, align 4, !range !1785, !noalias !23521, !noundef !1740
  %1987 = getelementptr i8, ptr %1985, i64 4
  %1988 = load i32, ptr %1987, align 4, !noalias !23521
  %1989 = icmp eq i32 %1986, 2
  %1990 = select i1 %1989, i32 undef, i32 %1988
  %1991 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %1984
  store i32 %1986, ptr %1991, align 4, !noalias !23536
  %1992 = getelementptr inbounds nuw i8, ptr %1991, i64 4
  store i32 %1990, ptr %1992, align 4, !noalias !23536
  %1993 = add nuw nsw i64 %1984, 1
  %prol.iter.next = add i64 %prol.iter, 1
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter1741
  br i1 %prol.iter.cmp.not, label %vec.epilog.scalar.ph1424.prol.loopexit, label %vec.epilog.scalar.ph1424.prol, !llvm.loop !23545

vec.epilog.scalar.ph1424.prol.loopexit:           ; preds = %vec.epilog.scalar.ph1424.prol, %vec.epilog.scalar.ph1424.preheader
  %.lcssa1469.unr = phi i64 [ poison, %vec.epilog.scalar.ph1424.preheader ], [ %1984, %vec.epilog.scalar.ph1424.prol ]
  %.unr1743 = phi i64 [ %.ph, %vec.epilog.scalar.ph1424.preheader ], [ %1993, %vec.epilog.scalar.ph1424.prol ]
  %1994 = sub nsw i64 %.ph, %1941
  %1995 = icmp ugt i64 %1994, -8
  br i1 %1995, label %.loopexit1458, label %vec.epilog.scalar.ph1424

vec.epilog.scalar.ph1424:                         ; preds = %vec.epilog.scalar.ph1424.prol.loopexit, %vec.epilog.scalar.ph1424
  %1996 = phi i64 [ %2068, %vec.epilog.scalar.ph1424 ], [ %.unr1743, %vec.epilog.scalar.ph1424.prol.loopexit ]
  %1997 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %1996
  %1998 = load i32, ptr %1997, align 4, !range !1785, !noalias !23521, !noundef !1740
  %1999 = getelementptr i8, ptr %1997, i64 4
  %2000 = load i32, ptr %1999, align 4, !noalias !23521
  %2001 = icmp eq i32 %1998, 2
  %2002 = select i1 %2001, i32 undef, i32 %2000
  %2003 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %1996
  store i32 %1998, ptr %2003, align 4, !noalias !23536
  %2004 = getelementptr inbounds nuw i8, ptr %2003, i64 4
  store i32 %2002, ptr %2004, align 4, !noalias !23536
  %2005 = add nuw nsw i64 %1996, 1
  %2006 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %2005
  %2007 = load i32, ptr %2006, align 4, !range !1785, !noalias !23521, !noundef !1740
  %2008 = getelementptr i8, ptr %2006, i64 4
  %2009 = load i32, ptr %2008, align 4, !noalias !23521
  %2010 = icmp eq i32 %2007, 2
  %2011 = select i1 %2010, i32 undef, i32 %2009
  %2012 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %2005
  store i32 %2007, ptr %2012, align 4, !noalias !23536
  %2013 = getelementptr inbounds nuw i8, ptr %2012, i64 4
  store i32 %2011, ptr %2013, align 4, !noalias !23536
  %2014 = add nuw nsw i64 %1996, 2
  %2015 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %2014
  %2016 = load i32, ptr %2015, align 4, !range !1785, !noalias !23521, !noundef !1740
  %2017 = getelementptr i8, ptr %2015, i64 4
  %2018 = load i32, ptr %2017, align 4, !noalias !23521
  %2019 = icmp eq i32 %2016, 2
  %2020 = select i1 %2019, i32 undef, i32 %2018
  %2021 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %2014
  store i32 %2016, ptr %2021, align 4, !noalias !23536
  %2022 = getelementptr inbounds nuw i8, ptr %2021, i64 4
  store i32 %2020, ptr %2022, align 4, !noalias !23536
  %2023 = add nuw nsw i64 %1996, 3
  %2024 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %2023
  %2025 = load i32, ptr %2024, align 4, !range !1785, !noalias !23521, !noundef !1740
  %2026 = getelementptr i8, ptr %2024, i64 4
  %2027 = load i32, ptr %2026, align 4, !noalias !23521
  %2028 = icmp eq i32 %2025, 2
  %2029 = select i1 %2028, i32 undef, i32 %2027
  %2030 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %2023
  store i32 %2025, ptr %2030, align 4, !noalias !23536
  %2031 = getelementptr inbounds nuw i8, ptr %2030, i64 4
  store i32 %2029, ptr %2031, align 4, !noalias !23536
  %2032 = add nuw nsw i64 %1996, 4
  %2033 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %2032
  %2034 = load i32, ptr %2033, align 4, !range !1785, !noalias !23521, !noundef !1740
  %2035 = getelementptr i8, ptr %2033, i64 4
  %2036 = load i32, ptr %2035, align 4, !noalias !23521
  %2037 = icmp eq i32 %2034, 2
  %2038 = select i1 %2037, i32 undef, i32 %2036
  %2039 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %2032
  store i32 %2034, ptr %2039, align 4, !noalias !23536
  %2040 = getelementptr inbounds nuw i8, ptr %2039, i64 4
  store i32 %2038, ptr %2040, align 4, !noalias !23536
  %2041 = add nuw nsw i64 %1996, 5
  %2042 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %2041
  %2043 = load i32, ptr %2042, align 4, !range !1785, !noalias !23521, !noundef !1740
  %2044 = getelementptr i8, ptr %2042, i64 4
  %2045 = load i32, ptr %2044, align 4, !noalias !23521
  %2046 = icmp eq i32 %2043, 2
  %2047 = select i1 %2046, i32 undef, i32 %2045
  %2048 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %2041
  store i32 %2043, ptr %2048, align 4, !noalias !23536
  %2049 = getelementptr inbounds nuw i8, ptr %2048, i64 4
  store i32 %2047, ptr %2049, align 4, !noalias !23536
  %2050 = add nuw nsw i64 %1996, 6
  %2051 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %2050
  %2052 = load i32, ptr %2051, align 4, !range !1785, !noalias !23521, !noundef !1740
  %2053 = getelementptr i8, ptr %2051, i64 4
  %2054 = load i32, ptr %2053, align 4, !noalias !23521
  %2055 = icmp eq i32 %2052, 2
  %2056 = select i1 %2055, i32 undef, i32 %2054
  %2057 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %2050
  store i32 %2052, ptr %2057, align 4, !noalias !23536
  %2058 = getelementptr inbounds nuw i8, ptr %2057, i64 4
  store i32 %2056, ptr %2058, align 4, !noalias !23536
  %2059 = add nuw nsw i64 %1996, 7
  %2060 = getelementptr inbounds nuw [8 x i8], ptr %1968, i64 %2059
  %2061 = load i32, ptr %2060, align 4, !range !1785, !noalias !23521, !noundef !1740
  %2062 = getelementptr i8, ptr %2060, i64 4
  %2063 = load i32, ptr %2062, align 4, !noalias !23521
  %2064 = icmp eq i32 %2061, 2
  %2065 = select i1 %2064, i32 undef, i32 %2063
  %2066 = getelementptr inbounds nuw [8 x i8], ptr %1958, i64 %2059
  store i32 %2061, ptr %2066, align 4, !noalias !23536
  %2067 = getelementptr inbounds nuw i8, ptr %2066, i64 4
  store i32 %2065, ptr %2067, align 4, !noalias !23536
  %2068 = add nuw nsw i64 %1996, 8
  %2069 = icmp eq i64 %2068, %1941
  br i1 %2069, label %.loopexit1458, label %vec.epilog.scalar.ph1424, !llvm.loop !23546

.loopexit1458:                                    ; preds = %vec.epilog.scalar.ph1424.prol.loopexit, %vec.epilog.scalar.ph1424, %vec.epilog.middle.block1437, %middle.block1420
  %.lcssa = phi i64 [ %ind.escape1438, %vec.epilog.middle.block1437 ], [ %ind.escape, %middle.block1420 ], [ %.lcssa1469.unr, %vec.epilog.scalar.ph1424.prol.loopexit ], [ %2059, %vec.epilog.scalar.ph1424 ]
  %2070 = icmp samesign ult i64 %1941, 1152921504606846976
  call void @llvm.assume(i1 %2070), !noalias !23124
  %2071 = add nuw nsw i64 %.lcssa, 2
  %2072 = add nuw nsw i64 %1941, 1
  br label %2077

2073:                                             ; preds = %2092, %1931, %1909
  %2074 = phi ptr [ %2093, %2092 ], [ %1869, %1931 ], [ %1869, %1909 ]
  %2075 = phi i64 [ %2098, %2092 ], [ %1870, %1931 ], [ %1870, %1909 ]
  %2076 = icmp eq ptr %1872, %1858
  br i1 %2076, label %.loopexit156, label %1868

2077:                                             ; preds = %.loopexit1458, %1960
  %2078 = phi ptr [ %1958, %.loopexit1458 ], [ %1962, %1960 ]
  %2079 = phi i64 [ %2072, %.loopexit1458 ], [ %1961, %1960 ]
  %2080 = phi i64 [ %2071, %.loopexit1458 ], [ %1963, %1960 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23547)
  %2081 = load i64, ptr %71, align 8, !range !1835, !alias.scope !23547, !noalias !23550, !noundef !1740
  %2082 = icmp eq i64 %1870, %2081
  br i1 %2082, label %2083, label %2092

2083:                                             ; preds = %2077
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %71)
          to label %2084 unwind label %2086, !noalias !23550

2084:                                             ; preds = %2083
  %2085 = load ptr, ptr %1764, align 8, !alias.scope !23547, !noalias !23550
  br label %2092

2086:                                             ; preds = %2083
  %2087 = landingpad { ptr, i32 }
          cleanup
  %2088 = icmp samesign ugt i64 %2079, 5
  br i1 %2088, label %2089, label %1877

2089:                                             ; preds = %2086
  %2090 = shl nuw i64 %2079, 3
  %2091 = add i64 %2090, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2078, i64 noundef %2091, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !23552
  br label %1877

2092:                                             ; preds = %2084, %2077
  %2093 = phi ptr [ %2085, %2084 ], [ %1869, %2077 ]
  %2094 = getelementptr inbounds nuw [40 x i8], ptr %2093, i64 %1870
  store i64 %2079, ptr %2094, align 8, !noalias !23555
  %2095 = getelementptr inbounds nuw i8, ptr %2094, i64 8
  store ptr %2078, ptr %2095, align 8, !noalias !23555
  %2096 = getelementptr inbounds nuw i8, ptr %2094, i64 16
  store i64 %2080, ptr %2096, align 8, !noalias !23555
  %2097 = getelementptr inbounds nuw i8, ptr %2094, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %2097, ptr noundef nonnull align 8 dereferenceable(16) %67, i64 16, i1 false), !noalias !23555
  %2098 = add i64 %1870, 1
  store i64 %2098, ptr %1765, align 8, !alias.scope !23547, !noalias !23550
  call void @llvm.lifetime.end.p0(ptr nonnull %67)
  br label %2073

2099:                                             ; preds = %1921
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !23119
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %71), !noalias !23124
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %74), !noalias !23119
  br label %2100

2100:                                             ; preds = %2099, %1748
  call void @llvm.lifetime.end.p0(ptr nonnull %75), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %80), !noalias !23119
  br label %2320

.loopexit156:                                     ; preds = %2073, %1879, %1855
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %70)
          to label %2101 unwind label %1800

2101:                                             ; preds = %.loopexit156
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !23119
  br label %1806

2102:                                             ; preds = %1854
  unreachable

2103:                                             ; preds = %1812, %1809, %1806
  call void @llvm.lifetime.end.p0(ptr nonnull %81), !noalias !23119
  br label %571

2104:                                             ; preds = %2169
  %2105 = landingpad { ptr, i32 }
          cleanup
  br label %562

2106:                                             ; preds = %571
  %2107 = getelementptr inbounds nuw i8, ptr %574, i64 16
  %2108 = load i8, ptr %2107, align 8, !noalias !23556
  %2109 = icmp eq i8 %2108, -1
  br i1 %2109, label %2116, label %2110

2110:                                             ; preds = %2106
  %2111 = getelementptr inbounds nuw i8, ptr %574, i64 17
  call void @llvm.lifetime.start.p0(ptr nonnull %59), !noalias !23119
  store i8 %2108, ptr %59, align 8, !noalias !23119
  %2112 = getelementptr inbounds nuw i8, ptr %59, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %2112, ptr noundef nonnull align 1 dereferenceable(23) %2111, i64 23, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %58), !noalias !23119
  %2113 = load ptr, ptr %85, align 8, !noalias !23119, !nonnull !1740, !noundef !1740
  %2114 = atomicrmw add ptr %2113, i64 1 monotonic, align 8, !noalias !23124
  %2115 = icmp slt i64 %2114, 0
  br i1 %2115, label %2172, label %2170

2116:                                             ; preds = %2106, %571
  call void @llvm.lifetime.start.p0(ptr nonnull %57), !noalias !23119
  call void @llvm.lifetime.start.p0(ptr nonnull %56), !noalias !23119
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %56, ptr noundef nonnull align 8 dereferenceable(104) %87, i64 104, i1 false), !noalias !23557
  call void @llvm.lifetime.start.p0(ptr nonnull %55), !noalias !23119
  %2117 = load ptr, ptr %85, align 8, !noalias !23119, !nonnull !1740, !noundef !1740
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %55, ptr noundef nonnull align 8 dereferenceable(24) %82, i64 24, i1 false), !noalias !23119
  %2118 = getelementptr inbounds nuw i8, ptr %55, i64 24
  store ptr %2117, ptr %2118, align 8, !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23558)
  call void @llvm.experimental.noalias.scope.decl(metadata !23561)
  call void @llvm.experimental.noalias.scope.decl(metadata !23563)
  %2119 = load i64, ptr %56, align 8, !range !2059, !alias.scope !23561, !noalias !23565, !noundef !1740
  %2120 = icmp eq i64 %2119, -1
  br i1 %2120, label %2122, label %2121

2121:                                             ; preds = %2116
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %57, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %55, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %87), !noalias !23566
  br label %2124

2122:                                             ; preds = %2116
  %2123 = getelementptr inbounds nuw i8, ptr %57, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %2123, ptr noundef nonnull readonly align 8 dereferenceable(32) %55, i64 32, i1 false), !alias.scope !23567, !noalias !23568
  store i64 -1, ptr %57, align 8, !alias.scope !23558, !noalias !23569
  br label %2124

2124:                                             ; preds = %2122, %2121
  %2125 = getelementptr inbounds nuw i8, ptr %56, i64 72
  %2126 = load i64, ptr %2125, align 8, !range !1778, !alias.scope !23570, !noalias !23565, !noundef !1740
  %2127 = icmp ugt i64 %2126, 5
  br i1 %2127, label %2128, label %2162

2128:                                             ; preds = %2124
  %2129 = getelementptr inbounds nuw i8, ptr %56, i64 80
  %2130 = load ptr, ptr %2129, align 8, !alias.scope !23561, !noalias !23565, !nonnull !1740, !noundef !1740
  %2131 = mul i64 %2126, 3
  %2132 = add i64 %2131, -3
  %2133 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2134 = load i64, ptr %2133, align 8, !noalias !23573, !noundef !1740
  %2135 = call i64 @llvm.umin.i64(i64 %2132, i64 9223372036854775807)
  %2136 = call i64 @llvm.ssub.sat.i64(i64 %2134, i64 %2135)
  store i64 %2136, ptr %2133, align 8, !noalias !23573
  %2137 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2138 = load i64, ptr %2137, align 8, !noalias !23573, !noundef !1740
  %2139 = icmp slt i64 %2136, %2138
  br i1 %2139, label %2140, label %.preheader1466

2140:                                             ; preds = %2128
  store i64 %2136, ptr %2137, align 8, !noalias !23573
  br label %.preheader1466

.preheader1466:                                   ; preds = %2140, %2128
  br label %2141

2141:                                             ; preds = %.preheader1466, %2144
  %2142 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23573
  %2143 = icmp slt i64 %2142, 0
  br i1 %2143, label %2144, label %__rustc::__rust_dealloc (.exit142)

2144:                                             ; preds = %2141
  %2145 = add nsw i64 %2142, 1
  %2146 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2142, i64 %2145 acq_rel acquire, align 8, !noalias !23573
  %2147 = extractvalue { i64, i1 } %2146, 1
  br i1 %2147, label %2148, label %2141

2148:                                             ; preds = %2144
  %2149 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2135 monotonic, align 8, !noalias !23573
  %2150 = call i64 @llvm.ssub.sat.i64(i64 %2149, i64 %2135)
  %2151 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23573
  br label %2152

2152:                                             ; preds = %2155, %2148
  %2153 = phi i64 [ %2151, %2148 ], [ %2158, %2155 ]
  %2154 = icmp slt i64 %2150, %2153
  br i1 %2154, label %2155, label %2159

2155:                                             ; preds = %2152
  %2156 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2153, i64 %2150 monotonic monotonic, align 8, !noalias !23573
  %2157 = extractvalue { i64, i1 } %2156, 1
  %2158 = extractvalue { i64, i1 } %2156, 0
  br i1 %2157, label %2159, label %2152

2159:                                             ; preds = %2155, %2152
  %2160 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23573
  br label %__rustc::__rust_dealloc (.exit142)

__rustc::__rust_dealloc (.exit142): ; preds = %2141, %2159
  %2161 = icmp ne i64 %2132, 0
  call void @llvm.assume(i1 %2161), !noalias !23573
  call void @free(ptr noundef nonnull %2130) #93, !noalias !23573
  br label %2162

2162:                                             ; preds = %__rustc::__rust_dealloc (.exit142), %2124
  %2163 = getelementptr inbounds nuw i8, ptr %56, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23576), !noalias !23124
  %2164 = load ptr, ptr %2163, align 8, !alias.scope !23579, !noalias !23565, !noundef !1740
  %2165 = icmp eq ptr %2164, null
  br i1 %2165, label %2256, label %2166

2166:                                             ; preds = %2162
  %2167 = atomicrmw sub ptr %2164, i64 1 release, align 8, !noalias !23580
  %2168 = icmp eq i64 %2167, 1
  br i1 %2168, label %2169, label %2256

2169:                                             ; preds = %2166
  fence acquire, !noalias !23124
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2163) #92
          to label %2256 unwind label %2104

2170:                                             ; preds = %2110
  %2171 = load ptr, ptr %85, align 8, !noalias !23119, !nonnull !1740, !noundef !1740
; invoke <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %58, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %59, ptr noundef nonnull %2171)
          to label %2173 unwind label %2318, !noalias !23124

2172:                                             ; preds = %2110
  call void @llvm.trap()
  unreachable

2173:                                             ; preds = %2170
  %2174 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %2174, ptr noundef nonnull align 8 dereferenceable(96) %58, i64 96, i1 false), !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %58), !noalias !23119
  store i64 0, ptr %0, align 16, !alias.scope !23112, !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %59), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23585)
  %2175 = getelementptr inbounds nuw i8, ptr %82, i64 8
  %2176 = load ptr, ptr %2175, align 8, !alias.scope !23585, !noalias !23124, !nonnull !1740, !noundef !1740
  %2177 = getelementptr inbounds nuw i8, ptr %82, i64 16
  %2178 = load i64, ptr %2177, align 8, !alias.scope !23585, !noalias !23124, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23588), !noalias !23124
  %2179 = icmp eq i64 %2178, 0
  br i1 %2179, label %.loopexit154, label %.preheader153

.preheader153:                                    ; preds = %2173
  %2180 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2181 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %2182

2182:                                             ; preds = %.preheader153, %2220
  %2183 = phi i64 [ %2185, %2220 ], [ 0, %.preheader153 ]
  %2184 = getelementptr inbounds nuw [40 x i8], ptr %2176, i64 %2183
  %2185 = add nuw nsw i64 %2183, 1
  %2186 = load i64, ptr %2184, align 8, !range !1778, !alias.scope !23591, !noalias !23594, !noundef !1740
  %2187 = icmp ugt i64 %2186, 5
  br i1 %2187, label %2188, label %2220

2188:                                             ; preds = %2182
  %2189 = getelementptr i8, ptr %2184, i64 8
  %2190 = load ptr, ptr %2189, align 8, !alias.scope !23588, !noalias !23594, !nonnull !1740, !noundef !1740
  %2191 = shl i64 %2186, 3
  %2192 = add i64 %2191, -8
  %2193 = load i64, ptr %2180, align 8, !noalias !23595, !noundef !1740
  %2194 = call i64 @llvm.umin.i64(i64 %2192, i64 9223372036854775807)
  %2195 = call i64 @llvm.ssub.sat.i64(i64 %2193, i64 %2194)
  store i64 %2195, ptr %2180, align 8, !noalias !23595
  %2196 = load i64, ptr %2181, align 8, !noalias !23595, !noundef !1740
  %2197 = icmp slt i64 %2195, %2196
  br i1 %2197, label %2198, label %.preheader1468

2198:                                             ; preds = %2188
  store i64 %2195, ptr %2181, align 8, !noalias !23595
  br label %.preheader1468

.preheader1468:                                   ; preds = %2198, %2188
  br label %2199

2199:                                             ; preds = %.preheader1468, %2202
  %2200 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23595
  %2201 = icmp slt i64 %2200, 0
  br i1 %2201, label %2202, label %__rustc::__rust_dealloc (.exit143)

2202:                                             ; preds = %2199
  %2203 = add nsw i64 %2200, 1
  %2204 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2200, i64 %2203 acq_rel acquire, align 8, !noalias !23595
  %2205 = extractvalue { i64, i1 } %2204, 1
  br i1 %2205, label %2206, label %2199

2206:                                             ; preds = %2202
  %2207 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2194 monotonic, align 8, !noalias !23595
  %2208 = call i64 @llvm.ssub.sat.i64(i64 %2207, i64 %2194)
  %2209 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23595
  br label %2210

2210:                                             ; preds = %2213, %2206
  %2211 = phi i64 [ %2209, %2206 ], [ %2216, %2213 ]
  %2212 = icmp slt i64 %2208, %2211
  br i1 %2212, label %2213, label %2217

2213:                                             ; preds = %2210
  %2214 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2211, i64 %2208 monotonic monotonic, align 8, !noalias !23595
  %2215 = extractvalue { i64, i1 } %2214, 1
  %2216 = extractvalue { i64, i1 } %2214, 0
  br i1 %2215, label %2217, label %2210

2217:                                             ; preds = %2213, %2210
  %2218 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23595
  br label %__rustc::__rust_dealloc (.exit143)

__rustc::__rust_dealloc (.exit143): ; preds = %2199, %2217
  %2219 = icmp ne i64 %2192, 0
  call void @llvm.assume(i1 %2219), !noalias !23595
  call void @free(ptr noundef nonnull %2190) #93, !noalias !23595
  br label %2220

2220:                                             ; preds = %__rustc::__rust_dealloc (.exit143), %2182
  %2221 = icmp eq i64 %2185, %2178
  br i1 %2221, label %.loopexit154, label %2182

.loopexit154:                                     ; preds = %2220, %2173
  %2222 = load i64, ptr %82, align 8, !alias.scope !23585, !noalias !23124
  %2223 = icmp eq i64 %2222, 0
  br i1 %2223, label %2254, label %2224

2224:                                             ; preds = %.loopexit154
  %2225 = mul nuw i64 %2222, 40
  %2226 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2227 = load i64, ptr %2226, align 8, !noalias !23594, !noundef !1740
  %2228 = call i64 @llvm.umin.i64(i64 %2225, i64 9223372036854775807)
  %2229 = call i64 @llvm.ssub.sat.i64(i64 %2227, i64 %2228)
  store i64 %2229, ptr %2226, align 8, !noalias !23594
  %2230 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2231 = load i64, ptr %2230, align 8, !noalias !23594, !noundef !1740
  %2232 = icmp slt i64 %2229, %2231
  br i1 %2232, label %2233, label %.preheader1467

2233:                                             ; preds = %2224
  store i64 %2229, ptr %2230, align 8, !noalias !23594
  br label %.preheader1467

.preheader1467:                                   ; preds = %2233, %2224
  br label %2234

2234:                                             ; preds = %.preheader1467, %2237
  %2235 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23594
  %2236 = icmp slt i64 %2235, 0
  br i1 %2236, label %2237, label %__rustc::__rust_dealloc (.exit144)

2237:                                             ; preds = %2234
  %2238 = add nsw i64 %2235, 1
  %2239 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2235, i64 %2238 acq_rel acquire, align 8, !noalias !23594
  %2240 = extractvalue { i64, i1 } %2239, 1
  br i1 %2240, label %2241, label %2234

2241:                                             ; preds = %2237
  %2242 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2228 monotonic, align 8, !noalias !23594
  %2243 = call i64 @llvm.ssub.sat.i64(i64 %2242, i64 %2228)
  %2244 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23594
  br label %2245

2245:                                             ; preds = %2248, %2241
  %2246 = phi i64 [ %2244, %2241 ], [ %2251, %2248 ]
  %2247 = icmp slt i64 %2243, %2246
  br i1 %2247, label %2248, label %2252

2248:                                             ; preds = %2245
  %2249 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2246, i64 %2243 monotonic monotonic, align 8, !noalias !23594
  %2250 = extractvalue { i64, i1 } %2249, 1
  %2251 = extractvalue { i64, i1 } %2249, 0
  br i1 %2250, label %2252, label %2245

2252:                                             ; preds = %2248, %2245
  %2253 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23594
  br label %__rustc::__rust_dealloc (.exit144)

__rustc::__rust_dealloc (.exit144): ; preds = %2234, %2252
  call void @free(ptr noundef nonnull %2176) #93, !noalias !23594
  br label %2254

2254:                                             ; preds = %2331, %__rustc::__rust_dealloc (.exit144), %.loopexit154, %568
  %2255 = phi i8 [ 1, %2331 ], [ 0, %568 ], [ %572, %.loopexit154 ], [ %572, %__rustc::__rust_dealloc (.exit144) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !23119
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %83)
          to label %2332 unwind label %229, !noalias !23124

2256:                                             ; preds = %2169, %2166, %2162
  call void @llvm.lifetime.end.p0(ptr nonnull %55), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !23119
  %2257 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %2257, ptr noundef nonnull align 8 dereferenceable(96) %57, i64 96, i1 false), !noalias !23160
  store i64 0, ptr %0, align 16, !alias.scope !23112, !noalias !23160
  call void @llvm.lifetime.end.p0(ptr nonnull %57), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !23119
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(216) %83)
          to label %2258 unwind label %229, !noalias !23124

2258:                                             ; preds = %2256
  call void @llvm.lifetime.end.p0(ptr nonnull %83), !noalias !23119
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %84)
          to label %2259 unwind label %195

2259:                                             ; preds = %2258
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !23119
  call void @llvm.lifetime.end.p0(ptr nonnull %85), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23598)
  call void @llvm.experimental.noalias.scope.decl(metadata !23601), !noalias !23604
  %2260 = load ptr, ptr %184, align 8, !alias.scope !23605, !noalias !23604, !nonnull !1740, !noundef !1740
  %2261 = atomicrmw sub ptr %2260, i64 1 release, align 8, !noalias !23606
  %2262 = icmp eq i64 %2261, 1
  br i1 %2262, label %2263, label %2267

2263:                                             ; preds = %2259
  fence acquire, !noalias !23604
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %184) #92
          to label %2267 unwind label %2264

2264:                                             ; preds = %2263
  %2265 = landingpad { ptr, i32 }
          cleanup
  %2266 = trunc nuw i8 %572 to i1
  br i1 %2266, label %2537, label %2539

2267:                                             ; preds = %2263, %2259
  %2268 = trunc nuw i8 %572 to i1
  br i1 %2268, label %2269, label %2535

2269:                                             ; preds = %2267
  call void @llvm.experimental.noalias.scope.decl(metadata !23607)
  %2270 = getelementptr inbounds nuw i8, ptr %90, i64 8
  %2271 = load ptr, ptr %2270, align 8, !alias.scope !23607, !nonnull !1740, !noundef !1740
  %2272 = load i64, ptr %220, align 8, !alias.scope !23607, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23610)
  %2273 = icmp eq i64 %2272, 0
  br i1 %2273, label %.loopexit152, label %.preheader151

.preheader151:                                    ; preds = %2269
  %2274 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2275 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %2276

2276:                                             ; preds = %.preheader151, %2314
  %2277 = phi i64 [ %2279, %2314 ], [ 0, %.preheader151 ]
  %2278 = getelementptr inbounds nuw [40 x i8], ptr %2271, i64 %2277
  %2279 = add nuw nsw i64 %2277, 1
  %2280 = load i64, ptr %2278, align 8, !range !1778, !alias.scope !23613, !noalias !23607, !noundef !1740
  %2281 = icmp ugt i64 %2280, 5
  br i1 %2281, label %2282, label %2314

2282:                                             ; preds = %2276
  %2283 = getelementptr i8, ptr %2278, i64 8
  %2284 = load ptr, ptr %2283, align 8, !alias.scope !23610, !noalias !23607, !nonnull !1740, !noundef !1740
  %2285 = shl i64 %2280, 3
  %2286 = add i64 %2285, -8
  %2287 = load i64, ptr %2274, align 8, !noalias !23616, !noundef !1740
  %2288 = call i64 @llvm.umin.i64(i64 %2286, i64 9223372036854775807)
  %2289 = call i64 @llvm.ssub.sat.i64(i64 %2287, i64 %2288)
  store i64 %2289, ptr %2274, align 8, !noalias !23616
  %2290 = load i64, ptr %2275, align 8, !noalias !23616, !noundef !1740
  %2291 = icmp slt i64 %2289, %2290
  br i1 %2291, label %2292, label %.preheader1465

2292:                                             ; preds = %2282
  store i64 %2289, ptr %2275, align 8, !noalias !23616
  br label %.preheader1465

.preheader1465:                                   ; preds = %2292, %2282
  br label %2293

2293:                                             ; preds = %.preheader1465, %2296
  %2294 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23616
  %2295 = icmp slt i64 %2294, 0
  br i1 %2295, label %2296, label %__rustc::__rust_dealloc (.exit145)

2296:                                             ; preds = %2293
  %2297 = add nsw i64 %2294, 1
  %2298 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2294, i64 %2297 acq_rel acquire, align 8, !noalias !23616
  %2299 = extractvalue { i64, i1 } %2298, 1
  br i1 %2299, label %2300, label %2293

2300:                                             ; preds = %2296
  %2301 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2288 monotonic, align 8, !noalias !23616
  %2302 = call i64 @llvm.ssub.sat.i64(i64 %2301, i64 %2288)
  %2303 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23616
  br label %2304

2304:                                             ; preds = %2307, %2300
  %2305 = phi i64 [ %2303, %2300 ], [ %2310, %2307 ]
  %2306 = icmp slt i64 %2302, %2305
  br i1 %2306, label %2307, label %2311

2307:                                             ; preds = %2304
  %2308 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2305, i64 %2302 monotonic monotonic, align 8, !noalias !23616
  %2309 = extractvalue { i64, i1 } %2308, 1
  %2310 = extractvalue { i64, i1 } %2308, 0
  br i1 %2309, label %2311, label %2304

2311:                                             ; preds = %2307, %2304
  %2312 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23616
  br label %__rustc::__rust_dealloc (.exit145)

__rustc::__rust_dealloc (.exit145): ; preds = %2293, %2311
  %2313 = icmp ne i64 %2286, 0
  call void @llvm.assume(i1 %2313), !noalias !23616
  call void @free(ptr noundef nonnull %2284) #93, !noalias !23616
  br label %2314

2314:                                             ; preds = %__rustc::__rust_dealloc (.exit145), %2276
  %2315 = icmp eq i64 %2279, %2272
  br i1 %2315, label %.loopexit152, label %2276

.loopexit152:                                     ; preds = %2314, %2269
  %2316 = load i64, ptr %90, align 8, !alias.scope !23607
  %2317 = icmp eq i64 %2316, 0
  br i1 %2317, label %2535, label %2503

2318:                                             ; preds = %2170
  %2319 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %82) #90, !noalias !23124
  br label %562

2320:                                             ; preds = %2100, %629
  call void @llvm.experimental.noalias.scope.decl(metadata !23619)
  %2321 = load ptr, ptr %81, align 8, !alias.scope !23619, !noalias !23124, !noundef !1740
  %2322 = icmp eq ptr %2321, null
  br i1 %2322, label %2331, label %2323

2323:                                             ; preds = %2320
  %2324 = atomicrmw sub ptr %2321, i64 1 release, align 8, !noalias !23622
  %2325 = icmp eq i64 %2324, 1
  br i1 %2325, label %2326, label %2331

2326:                                             ; preds = %2323
  fence acquire, !noalias !23124
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %81) #92
          to label %2331 unwind label %566, !inline_history !2025

2327:                                             ; preds = %1746, %1739, %1692, %1654, %._crit_edge1316, %1628, %1627, %973, %672
  %2328 = phi { ptr, i32 } [ %673, %672 ], [ %1693, %1692 ], [ %1747, %1746 ], [ %1079, %1627 ], [ %1740, %1739 ], [ %1079, %1628 ], [ %974, %973 ], [ %1647, %._crit_edge1316 ], [ %1647, %1654 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %75) #90
          to label %604 unwind label %583, !noalias !23124

2329:                                             ; preds = %635
  %2330 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %80) #90, !noalias !23124
  br label %604

2331:                                             ; preds = %2326, %2323, %2320
  call void @llvm.lifetime.end.p0(ptr nonnull %81), !noalias !23119
  br label %2254

2332:                                             ; preds = %2254
  call void @llvm.lifetime.end.p0(ptr nonnull %83), !noalias !23119
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(200) %84)
          to label %2333 unwind label %192

2333:                                             ; preds = %2332
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23625)
  call void @llvm.experimental.noalias.scope.decl(metadata !23628), !noalias !23124
  %2334 = load ptr, ptr %85, align 8, !alias.scope !23631, !noalias !23124, !nonnull !1740, !noundef !1740
  %2335 = atomicrmw sub ptr %2334, i64 1 release, align 8, !noalias !23632
  %2336 = icmp eq i64 %2335, 1
  br i1 %2336, label %2337, label %2340

2337:                                             ; preds = %2333
  fence acquire, !noalias !23124
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %85) #92
          to label %2340 unwind label %2338

2338:                                             ; preds = %2337
  %2339 = landingpad { ptr, i32 }
          cleanup
  br label %2496

2340:                                             ; preds = %2337, %2333
  call void @llvm.lifetime.end.p0(ptr nonnull %85), !noalias !23119
  call void @llvm.experimental.noalias.scope.decl(metadata !23633)
  %2341 = getelementptr inbounds nuw i8, ptr %87, i64 72
  %2342 = load i64, ptr %2341, align 8, !range !1778, !alias.scope !23636, !noalias !23566, !noundef !1740
  %2343 = icmp ugt i64 %2342, 5
  br i1 %2343, label %2344, label %2378

2344:                                             ; preds = %2340
  %2345 = getelementptr inbounds nuw i8, ptr %87, i64 80
  %2346 = load ptr, ptr %2345, align 8, !alias.scope !23633, !noalias !23566, !nonnull !1740, !noundef !1740
  %2347 = mul i64 %2342, 3
  %2348 = add i64 %2347, -3
  %2349 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2350 = load i64, ptr %2349, align 8, !noalias !23639, !noundef !1740
  %2351 = call i64 @llvm.umin.i64(i64 %2348, i64 9223372036854775807)
  %2352 = call i64 @llvm.ssub.sat.i64(i64 %2350, i64 %2351)
  store i64 %2352, ptr %2349, align 8, !noalias !23639
  %2353 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2354 = load i64, ptr %2353, align 8, !noalias !23639, !noundef !1740
  %2355 = icmp slt i64 %2352, %2354
  br i1 %2355, label %2356, label %.preheader1464

2356:                                             ; preds = %2344
  store i64 %2352, ptr %2353, align 8, !noalias !23639
  br label %.preheader1464

.preheader1464:                                   ; preds = %2356, %2344
  br label %2357

2357:                                             ; preds = %.preheader1464, %2360
  %2358 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23639
  %2359 = icmp slt i64 %2358, 0
  br i1 %2359, label %2360, label %__rustc::__rust_dealloc (.exit146)

2360:                                             ; preds = %2357
  %2361 = add nsw i64 %2358, 1
  %2362 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2358, i64 %2361 acq_rel acquire, align 8, !noalias !23639
  %2363 = extractvalue { i64, i1 } %2362, 1
  br i1 %2363, label %2364, label %2357

2364:                                             ; preds = %2360
  %2365 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2351 monotonic, align 8, !noalias !23639
  %2366 = call i64 @llvm.ssub.sat.i64(i64 %2365, i64 %2351)
  %2367 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23639
  br label %2368

2368:                                             ; preds = %2371, %2364
  %2369 = phi i64 [ %2367, %2364 ], [ %2374, %2371 ]
  %2370 = icmp slt i64 %2366, %2369
  br i1 %2370, label %2371, label %2375

2371:                                             ; preds = %2368
  %2372 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2369, i64 %2366 monotonic monotonic, align 8, !noalias !23639
  %2373 = extractvalue { i64, i1 } %2372, 1
  %2374 = extractvalue { i64, i1 } %2372, 0
  br i1 %2373, label %2375, label %2368

2375:                                             ; preds = %2371, %2368
  %2376 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23639
  br label %__rustc::__rust_dealloc (.exit146)

__rustc::__rust_dealloc (.exit146): ; preds = %2357, %2375
  %2377 = icmp ne i64 %2348, 0
  call void @llvm.assume(i1 %2377), !noalias !23639
  call void @free(ptr noundef nonnull %2346) #93, !noalias !23639
  br label %2378

2378:                                             ; preds = %__rustc::__rust_dealloc (.exit146), %2340
  %2379 = load i64, ptr %87, align 8, !range !2059, !alias.scope !23633, !noalias !23566, !noundef !1740
  %2380 = icmp sgt i64 %2379, 0
  br i1 %2380, label %2381, label %2413

2381:                                             ; preds = %2378
  %2382 = getelementptr inbounds nuw i8, ptr %87, i64 8
  %2383 = load ptr, ptr %2382, align 8, !alias.scope !23633, !noalias !23566, !nonnull !1740, !noundef !1740
  %2384 = mul nuw i64 %2379, 3
  %2385 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2386 = load i64, ptr %2385, align 8, !noalias !23642, !noundef !1740
  %2387 = call i64 @llvm.umin.i64(i64 %2384, i64 9223372036854775807)
  %2388 = call i64 @llvm.ssub.sat.i64(i64 %2386, i64 %2387)
  store i64 %2388, ptr %2385, align 8, !noalias !23642
  %2389 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2390 = load i64, ptr %2389, align 8, !noalias !23642, !noundef !1740
  %2391 = icmp slt i64 %2388, %2390
  br i1 %2391, label %2392, label %.preheader1463

2392:                                             ; preds = %2381
  store i64 %2388, ptr %2389, align 8, !noalias !23642
  br label %.preheader1463

.preheader1463:                                   ; preds = %2392, %2381
  br label %2393

2393:                                             ; preds = %.preheader1463, %2396
  %2394 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23642
  %2395 = icmp slt i64 %2394, 0
  br i1 %2395, label %2396, label %__rustc::__rust_dealloc (.exit147)

2396:                                             ; preds = %2393
  %2397 = add nsw i64 %2394, 1
  %2398 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2394, i64 %2397 acq_rel acquire, align 8, !noalias !23642
  %2399 = extractvalue { i64, i1 } %2398, 1
  br i1 %2399, label %2400, label %2393

2400:                                             ; preds = %2396
  %2401 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2387 monotonic, align 8, !noalias !23642
  %2402 = call i64 @llvm.ssub.sat.i64(i64 %2401, i64 %2387)
  %2403 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23642
  br label %2404

2404:                                             ; preds = %2407, %2400
  %2405 = phi i64 [ %2403, %2400 ], [ %2410, %2407 ]
  %2406 = icmp slt i64 %2402, %2405
  br i1 %2406, label %2407, label %2411

2407:                                             ; preds = %2404
  %2408 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2405, i64 %2402 monotonic monotonic, align 8, !noalias !23642
  %2409 = extractvalue { i64, i1 } %2408, 1
  %2410 = extractvalue { i64, i1 } %2408, 0
  br i1 %2409, label %2411, label %2404

2411:                                             ; preds = %2407, %2404
  %2412 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23642
  br label %__rustc::__rust_dealloc (.exit147)

__rustc::__rust_dealloc (.exit147): ; preds = %2393, %2411
  call void @free(ptr noundef nonnull %2383) #93, !noalias !23642
  br label %2413

2413:                                             ; preds = %__rustc::__rust_dealloc (.exit147), %2378
  %2414 = getelementptr inbounds nuw i8, ptr %87, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !23643), !noalias !23566
  %2415 = load ptr, ptr %2414, align 8, !alias.scope !23646, !noalias !23566, !noundef !1740
  %2416 = icmp eq ptr %2415, null
  br i1 %2416, label %2430, label %2417

2417:                                             ; preds = %2413
  %2418 = atomicrmw sub ptr %2415, i64 1 release, align 8, !noalias !23647
  %2419 = icmp eq i64 %2418, 1
  br i1 %2419, label %2420, label %2430

2420:                                             ; preds = %2417
  fence acquire, !noalias !23566
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2414) #92
          to label %2430 unwind label %2428

2421:                                             ; preds = %2496, %2428, %195, %191
  %2422 = phi i8 [ %2255, %2428 ], [ %2497, %2496 ], [ %226, %191 ], [ %572, %195 ]
  %2423 = phi { ptr, i32 } [ %2429, %2428 ], [ %2498, %2496 ], [ %228, %191 ], [ %196, %195 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23652)
  call void @llvm.experimental.noalias.scope.decl(metadata !23655), !noalias !23112
  %2424 = load ptr, ptr %184, align 8, !alias.scope !23658, !noalias !23112, !nonnull !1740, !noundef !1740
  %2425 = atomicrmw sub ptr %2424, i64 1 release, align 8, !noalias !23659
  %2426 = icmp eq i64 %2425, 1
  br i1 %2426, label %2427, label %2499

2427:                                             ; preds = %2421
  fence acquire, !noalias !23112
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %184) #92
          to label %2499 unwind label %583

2428:                                             ; preds = %2420
  %2429 = landingpad { ptr, i32 }
          cleanup
  br label %2421

2430:                                             ; preds = %2420, %2417, %2413
  call void @llvm.experimental.noalias.scope.decl(metadata !23660)
  call void @llvm.experimental.noalias.scope.decl(metadata !23663), !noalias !23112
  %2431 = load ptr, ptr %184, align 8, !alias.scope !23666, !noalias !23112, !nonnull !1740, !noundef !1740
  %2432 = atomicrmw sub ptr %2431, i64 1 release, align 8, !noalias !23667
  %2433 = icmp eq i64 %2432, 1
  br i1 %2433, label %2434, label %2438

2434:                                             ; preds = %2430
  fence acquire, !noalias !23112
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %184) #92
          to label %2438 unwind label %2435

2435:                                             ; preds = %2434
  %2436 = landingpad { ptr, i32 }
          cleanup
  %2437 = trunc nuw i8 %2255 to i1
  br i1 %2437, label %2537, label %2539

2438:                                             ; preds = %2434, %2430
  %2439 = trunc nuw i8 %2255 to i1
  br i1 %2439, label %2440, label %2535

2440:                                             ; preds = %2438
  call void @llvm.experimental.noalias.scope.decl(metadata !23668)
  %2441 = getelementptr inbounds nuw i8, ptr %90, i64 8
  %2442 = load ptr, ptr %2441, align 8, !alias.scope !23668, !nonnull !1740, !noundef !1740
  %2443 = load i64, ptr %220, align 8, !alias.scope !23668, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !23671)
  %2444 = icmp eq i64 %2443, 0
  br i1 %2444, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %2440
  %2445 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2446 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %2447

2447:                                             ; preds = %.preheader, %2485
  %2448 = phi i64 [ %2450, %2485 ], [ 0, %.preheader ]
  %2449 = getelementptr inbounds nuw [40 x i8], ptr %2442, i64 %2448
  %2450 = add nuw nsw i64 %2448, 1
  %2451 = load i64, ptr %2449, align 8, !range !1778, !alias.scope !23674, !noalias !23668, !noundef !1740
  %2452 = icmp ugt i64 %2451, 5
  br i1 %2452, label %2453, label %2485

2453:                                             ; preds = %2447
  %2454 = getelementptr i8, ptr %2449, i64 8
  %2455 = load ptr, ptr %2454, align 8, !alias.scope !23671, !noalias !23668, !nonnull !1740, !noundef !1740
  %2456 = shl i64 %2451, 3
  %2457 = add i64 %2456, -8
  %2458 = load i64, ptr %2445, align 8, !noalias !23677, !noundef !1740
  %2459 = call i64 @llvm.umin.i64(i64 %2457, i64 9223372036854775807)
  %2460 = call i64 @llvm.ssub.sat.i64(i64 %2458, i64 %2459)
  store i64 %2460, ptr %2445, align 8, !noalias !23677
  %2461 = load i64, ptr %2446, align 8, !noalias !23677, !noundef !1740
  %2462 = icmp slt i64 %2460, %2461
  br i1 %2462, label %2463, label %.preheader1462

2463:                                             ; preds = %2453
  store i64 %2460, ptr %2446, align 8, !noalias !23677
  br label %.preheader1462

.preheader1462:                                   ; preds = %2463, %2453
  br label %2464

2464:                                             ; preds = %.preheader1462, %2467
  %2465 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !23677
  %2466 = icmp slt i64 %2465, 0
  br i1 %2466, label %2467, label %__rustc::__rust_dealloc (.exit148)

2467:                                             ; preds = %2464
  %2468 = add nsw i64 %2465, 1
  %2469 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2465, i64 %2468 acq_rel acquire, align 8, !noalias !23677
  %2470 = extractvalue { i64, i1 } %2469, 1
  br i1 %2470, label %2471, label %2464

2471:                                             ; preds = %2467
  %2472 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2459 monotonic, align 8, !noalias !23677
  %2473 = call i64 @llvm.ssub.sat.i64(i64 %2472, i64 %2459)
  %2474 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !23677
  br label %2475

2475:                                             ; preds = %2478, %2471
  %2476 = phi i64 [ %2474, %2471 ], [ %2481, %2478 ]
  %2477 = icmp slt i64 %2473, %2476
  br i1 %2477, label %2478, label %2482

2478:                                             ; preds = %2475
  %2479 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2476, i64 %2473 monotonic monotonic, align 8, !noalias !23677
  %2480 = extractvalue { i64, i1 } %2479, 1
  %2481 = extractvalue { i64, i1 } %2479, 0
  br i1 %2480, label %2482, label %2475

2482:                                             ; preds = %2478, %2475
  %2483 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !23677
  br label %__rustc::__rust_dealloc (.exit148)

__rustc::__rust_dealloc (.exit148): ; preds = %2464, %2482
  %2484 = icmp ne i64 %2457, 0
  call void @llvm.assume(i1 %2484), !noalias !23677
  call void @free(ptr noundef nonnull %2455) #93, !noalias !23677
  br label %2485

2485:                                             ; preds = %__rustc::__rust_dealloc (.exit148), %2447
  %2486 = icmp eq i64 %2450, %2443
  br i1 %2486, label %.loopexit, label %2447

.loopexit:                                        ; preds = %2485, %2440
  %2487 = load i64, ptr %90, align 8, !alias.scope !23668
  %2488 = icmp eq i64 %2487, 0
  br i1 %2488, label %2535, label %2503

2489:                                             ; preds = %192, %191
  %2490 = phi { ptr, i32 } [ %194, %192 ], [ %228, %191 ]
  %2491 = phi i8 [ %193, %192 ], [ %226, %191 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !23680)
  call void @llvm.experimental.noalias.scope.decl(metadata !23683), !noalias !23124
  %2492 = load ptr, ptr %85, align 8, !alias.scope !23686, !noalias !23124, !nonnull !1740, !noundef !1740
  %2493 = atomicrmw sub ptr %2492, i64 1 release, align 8, !noalias !23687
  %2494 = icmp eq i64 %2493, 1
  br i1 %2494, label %2495, label %2496

2495:                                             ; preds = %2489
  fence acquire, !noalias !23124
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %85) #92
          to label %2496 unwind label %583

2496:                                             ; preds = %2495, %2489, %2338
  %2497 = phi i8 [ %2255, %2338 ], [ %2491, %2495 ], [ %2491, %2489 ]
  %2498 = phi { ptr, i32 } [ %2339, %2338 ], [ %2490, %2495 ], [ %2490, %2489 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %87) #90
          to label %2421 unwind label %583, !noalias !23566

2499:                                             ; preds = %2427, %2421
  %2500 = trunc nuw i8 %2422 to i1
  br i1 %2500, label %2537, label %2539

2501:                                             ; preds = %180
  call void @llvm.lifetime.end.p0(ptr nonnull %88)
  call void @llvm.lifetime.end.p0(ptr nonnull %89)
  %2502 = getelementptr inbounds nuw i8, ptr %0, i64 8
; call <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  call fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %2502, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %91)
  store i64 0, ptr %0, align 16
  br label %2536

2503:                                             ; preds = %.loopexit, %.loopexit152
  %2504 = phi i64 [ %2316, %.loopexit152 ], [ %2487, %.loopexit ]
  %2505 = phi ptr [ %2271, %.loopexit152 ], [ %2442, %.loopexit ]
  %2506 = mul nuw i64 %2504, 40
  %2507 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2508 = load i64, ptr %2507, align 8, !noalias !1740, !noundef !1740
  %2509 = call i64 @llvm.umin.i64(i64 %2506, i64 9223372036854775807)
  %2510 = call i64 @llvm.ssub.sat.i64(i64 %2508, i64 %2509)
  store i64 %2510, ptr %2507, align 8, !noalias !1740
  %2511 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %2512 = load i64, ptr %2511, align 8, !noalias !1740, !noundef !1740
  %2513 = icmp slt i64 %2510, %2512
  br i1 %2513, label %2514, label %.preheader1461

2514:                                             ; preds = %2503
  store i64 %2510, ptr %2511, align 8, !noalias !1740
  br label %.preheader1461

.preheader1461:                                   ; preds = %2514, %2503
  br label %2515

2515:                                             ; preds = %.preheader1461, %2518
  %2516 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !1740
  %2517 = icmp slt i64 %2516, 0
  br i1 %2517, label %2518, label %__rustc::__rust_dealloc (.exit149)

2518:                                             ; preds = %2515
  %2519 = add nsw i64 %2516, 1
  %2520 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %2516, i64 %2519 acq_rel acquire, align 8, !noalias !1740
  %2521 = extractvalue { i64, i1 } %2520, 1
  br i1 %2521, label %2522, label %2515

2522:                                             ; preds = %2518
  %2523 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %2509 monotonic, align 8, !noalias !1740
  %2524 = call i64 @llvm.ssub.sat.i64(i64 %2523, i64 %2509)
  %2525 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !1740
  br label %2526

2526:                                             ; preds = %2529, %2522
  %2527 = phi i64 [ %2525, %2522 ], [ %2532, %2529 ]
  %2528 = icmp slt i64 %2524, %2527
  br i1 %2528, label %2529, label %2533

2529:                                             ; preds = %2526
  %2530 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %2527, i64 %2524 monotonic monotonic, align 8, !noalias !1740
  %2531 = extractvalue { i64, i1 } %2530, 1
  %2532 = extractvalue { i64, i1 } %2530, 0
  br i1 %2531, label %2533, label %2526

2533:                                             ; preds = %2529, %2526
  %2534 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !1740
  br label %__rustc::__rust_dealloc (.exit149)

__rustc::__rust_dealloc (.exit149): ; preds = %2515, %2533
  call void @free(ptr noundef nonnull %2505) #93, !noalias !1740
  br label %2535

2535:                                             ; preds = %__rustc::__rust_dealloc (.exit149), %.loopexit, %2438, %.loopexit152, %2267
  call void @llvm.lifetime.end.p0(ptr nonnull %87)
  call void @llvm.lifetime.end.p0(ptr nonnull %90)
  br label %2536

2536:                                             ; preds = %2535, %2501, %177, %174, %170
  ret void

2537:                                             ; preds = %2499, %2435, %2264
  %2538 = phi { ptr, i32 } [ %2265, %2264 ], [ %2436, %2435 ], [ %2423, %2499 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %90) #90, !noalias !23112
  br label %2539

2539:                                             ; preds = %2541, %2537, %2499, %2435, %2264
  %2540 = phi { ptr, i32 } [ %2265, %2264 ], [ %2542, %2541 ], [ %2423, %2499 ], [ %2436, %2435 ], [ %2538, %2537 ]
  resume { ptr, i32 } %2540

2541:                                             ; preds = %178, %5
  %2542 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %91) #90
          to label %2539 unwind label %2543

2543:                                             ; preds = %2541
  %2544 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #91
  unreachable
}
